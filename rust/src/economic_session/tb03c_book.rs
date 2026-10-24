//! #1406 TB-03-C：`tb03c-book/1` 书账规则（根裁定 OQ-4/5/6/7）。
//!
//! 只对 manifest 冻结了 `book_revision = tb03c-book/1` 的新域生效；旧 `/1`、`/2` 域逐字走
//! `book.rs` 旧规则（#799 无回落）。口径：
//! - 成本基 K（多头）= Σacquisition_cash + Σcommission + Σfee_only − Σdisposal_cash − Σ分红净额，
//!   有符号 i64、不 clamp（ADR 0024 裁定四；ADR 0031 裁定三/五）。
//! - 本金 P = Σ principal_contribution 批，只增不减；P_recovered 只由 recover_capital 增加且不超过 P
//!   （ADR 0031 裁定一）；A、W 不动 P。
//! - 唯一扩量资格谓词 [`growth_eligible`]；`TStage` latch 只记阶段事件史、不作准入输入（ADR 0031 裁定三）。
//!   单向不变式：多头资格为真 ⇒ latch = EarningShares（根裁定 OQ-2）；空头无赚股第三相（SPEC C05.5），
//!   latch 止于 CapitalRecovered。
//! - cost_remaining：多头 K；空头 ΣM_b − 费后累计已实现利润（ADR 0036 裁定五）。
//! - TestOnly 原件首次资格为真记 SimulatedGateReached；PostGatePolicy 全槽未签，
//!   growth_enabled 为 AwaitingPolicy{post_gate_params}（ADR 0036 裁定七/八）。
//! - 估值观察只记 E，B 拒收（C05.4）；拆股按整数比重基、不整除即 Unsupported（ADR 0031 裁定五）。
//! - `tb03c-book/2`（#1406 C8）= 本规则 + manifest 冻结资金合同：开局批按 ADR 0040 裁定四第 4/5 步核，
//!   追加批与 q_rebase 为 AwaitingPolicy{funding_revision | q_rebase}；见 `tb03c_funding`。
use super::book::{add, array, decode, integer, push, reserve, sub, target, Rules};
use crate::session_protocol::{nonnegative, positive, required};
use crate::theta_v0::strategy::ledger::{ledger_step_components, LedgerEvent};
use serde_json::{json, Value};

/// 旧白名单（acquisition_cash / commission）之外，本版新增的成本分量类型。
pub(super) const COMPONENT_KINDS: [&str; 7] = [
    "disposal_cash",
    "fee_only",
    "dividend_cash",
    "withholding_tax",
    "principal_contribution",
    "recover_capital",
    "growth_capitalization",
];
const TOTALS: [&str; 6] = [
    "acquisition_cash",
    "disposal_cash",
    "commission",
    "fee_only",
    "dividend_cash",
    "withholding_tax",
];
/// 零数量变化合法的分量（ADR 0031 裁定二：费用独立件；本金、退本金、分红为非成交类原件）。
const DU0_KINDS: [&str; 6] = [
    "fee_only",
    "principal_contribution",
    "recover_capital",
    "dividend_cash",
    "withholding_tax",
    "growth_capitalization",
];
/// 只能出现在零数量变化原件里的分量。
const DU0_ONLY: [&str; 5] = [
    "fee_only",
    "recover_capital",
    "dividend_cash",
    "withholding_tax",
    "growth_capitalization",
];
const OPENING_KINDS: [&str; 3] = ["acquisition_cash", "commission", "principal_contribution"];
/// 根裁定 OQ-5：最小 TestOnly 空头书账只记费用、保证金批与具名实现利润。
const SHORT_KINDS: [&str; 3] = ["commission", "fee_only", "principal_contribution"];
const LATCH: [&str; 3] = ["CostReduction", "CapitalRecovered", "EarningShares"];

type CheckRef<'a> = &'a dyn Fn(&str, &str, &Value) -> Result<(), String>;

fn kind(c: &Value) -> &str {
    c["kind"].as_str().unwrap_or("")
}

/// 本版对单条原件的附加形状校验（与重的当前状态无关）；由 `book::decode` 在 v1 下调用。
pub(super) fn decode_extensions(
    event: &Value,
    pointer: &str,
    check_ref: CheckRef,
) -> Result<(), String> {
    let sp = event
        .get("stable_point")
        .filter(|v| v.is_object())
        .ok_or("MissingDependency：tb03c-book/1 原件须声明稳定点（根裁定 OQ-6）")?;
    if sp["profile"] != "TestOnly" {
        return Err("SchemaUnsupported：稳定点只接受 TestOnly 声明（真实 S 绑定归 TB-04）".into());
    }
    nonnegative(required(sp, "seq")?, "stable_point.seq")?;
    check_ref(
        required(sp, "source_ref")?,
        &format!("{pointer}/stable_point"),
        sp,
    )?;
    super::tb03c_e::check_depends_on(event, pointer, check_ref)?;
    super::tb03c_transfer::decode(event, pointer, check_ref)?;
    let comps = array(event, "known_cost_components")?;
    if let Some(rebase) = event.get("q_rebase") {
        super::voice::exact(rebase, &["new_q", "policy_ref", "source_ref"])?;
        positive(required(rebase, "new_q")?, "q_rebase.new_q")?;
        required(rebase, "policy_ref")?;
        check_ref(
            required(rebase, "source_ref")?,
            &format!("{pointer}/q_rebase"),
            rebase,
        )?;
        if event.get("observed_existing_book").is_some()
            || event.get("corporate_action").is_some()
            || event.get("valuation_observation").is_some()
            || !comps.is_empty()
            || ["units", "Pi", "A", "W"]
                .iter()
                .any(|k| event["deltas"][k] != "0")
        {
            return Err(
                "SchemaUnsupported：q_rebase 只承载独立数量重基请求，不夹带金额/成交".into(),
            );
        }
    }
    if event.get("observed_existing_book").is_some() {
        match event["book_identity"]["side"].as_str() {
            Some("long") | Some("short") => (),
            _ => return Err("SchemaUnsupported：开局须声明 side（long/short）".into()),
        }
        if event.get("corporate_action").is_some() || event.get("valuation_observation").is_some() {
            return Err("SchemaUnsupported：开局原件不带公司行动或估值观察".into());
        }
        if comps.iter().any(|c| !OPENING_KINDS.contains(&kind(c))) {
            return Err("SchemaUnsupported：开局只收采购现金、佣金与本金批".into());
        }
    }
    for c in comps {
        match kind(c) {
            "fee_only" => {
                required(c, "fee_type")?;
                if !["fill", "command", "period"].contains(&required(c, "cause")?) {
                    return Err(
                        "InvalidDomain：费用因果只有 fill/command/period（ADR 0032 裁定五）".into(),
                    );
                }
                if c["cause"] == "fill" {
                    // #1406 H1：fee_only 是独立费用件；同件成交佣金使用 commission。
                    if integer(&event["deltas"], "units")? != 0 {
                        return Err("SchemaUnsupported：fee_only 不得混在数量变动原件".into());
                    }
                    let fill = c
                        .get("fill_ref")
                        .filter(|v| v.is_object())
                        .ok_or("MissingDependency：fill 费用缺具名原分配绑定")?;
                    super::voice::exact(
                        fill,
                        &[
                            "source_namespace",
                            "source_epoch",
                            "external_fact_id",
                            "allocation_id",
                            "family_id",
                            "chong_id",
                        ],
                    )?;
                    for field in [
                        "source_namespace",
                        "source_epoch",
                        "external_fact_id",
                        "allocation_id",
                        "family_id",
                        "chong_id",
                    ] {
                        required(fill, field)?;
                    }
                    let dependency = event
                        .get("depends_on")
                        .filter(|v| v.is_object())
                        .ok_or("MissingDependency：fill 费用必须声明具名 fill 前件")?;
                    if ["source_namespace", "source_epoch", "external_fact_id"]
                        .iter()
                        .any(|key| fill[key] != dependency[key])
                        || fill["family_id"] != event["family_id"]
                        || fill["chong_id"] != target(event)?
                    {
                        return Err("IdentityConflict：费用前件与具名 fill/原 family/重不符".into());
                    }
                }
            }
            "dividend_cash" => {
                let gross = nonnegative(required(c, "gross_amount")?, "gross_amount")?;
                let withheld = nonnegative(required(c, "withheld_amount")?, "withheld_amount")?;
                if sub(gross, withheld)? != integer(c, "canonical_amount")? {
                    return Err("IdentityConflict：分红 gross − withheld ≠ net".into());
                }
            }
            "withholding_tax" => {
                if c["netted_in_dividend"] != true {
                    return Err("SchemaUnsupported：代扣税须声明已含于同件分红净额".into());
                }
                let amount = c["canonical_amount"].clone();
                if !comps
                    .iter()
                    .any(|d| kind(d) == "dividend_cash" && d["withheld_amount"] == amount)
                {
                    return Err("IdentityConflict：代扣税与同件分红扣税额不符".into());
                }
            }
            "growth_capitalization" => {
                if comps.len() != 1
                    || event["deltas"]["units"] != "0"
                    || event["deltas"]["Pi"] != "0"
                    || event["deltas"]["W"] != "0"
                    || integer(c, "canonical_amount")? != integer(&event["deltas"], "A")?
                {
                    return Err(
                        "IdentityConflict：增长资本化必须是独立已完成 A 分量，金额与原见证相同"
                            .into(),
                    );
                }
                required(c, "authorization_ref")?;
                if !array(event, "component_witnesses")?
                    .iter()
                    .any(|w| w["kind"] == "completed_profit_capitalization")
                {
                    return Err("MissingDependency：增长资本化缺已完成原件见证".into());
                }
                // completed_profit_capitalization 的来源和金额由 book::decode 的共同见证核验证。
            }
            "principal_contribution" => {
                required(c, "batch_id")?;
                required(c, "authorization_ref")?;
            }
            _ => (),
        }
    }
    if let Some(ca) = event.get("corporate_action") {
        if required(ca, "kind")? != "split" {
            return Err("SchemaUnsupported：本片公司行动只有拆股".into());
        }
        positive(required(ca, "num")?, "split.num")?;
        positive(required(ca, "den")?, "split.den")?;
        check_ref(
            required(ca, "source_ref")?,
            &format!("{pointer}/corporate_action"),
            ca,
        )?;
        if !comps.is_empty() {
            return Err("SchemaUnsupported：拆股原件不带现金分量（零股现金处置未支持）".into());
        }
    }
    if let Some(v) = event.get("valuation_observation") {
        nonnegative(
            required(v, "reference_price_before")?,
            "reference_price_before",
        )?;
        nonnegative(
            required(v, "reference_price_after")?,
            "reference_price_after",
        )?;
        check_ref(
            required(v, "source_ref")?,
            &format!("{pointer}/valuation_observation"),
            v,
        )?;
        if !comps.is_empty() {
            return Err("SchemaUnsupported：估值观察不带现金分量".into());
        }
    }
    Ok(())
}

/// 数量变化与分量类别的配对（ADR 0031 裁定二）。
pub(super) fn check_quantity_class(event: &Value, du: i64) -> Result<(), String> {
    let comps = array(event, "known_cost_components")?;
    let special = event.get("corporate_action").is_some()
        || event.get("valuation_observation").is_some()
        || event.get("q_rebase").is_some()
        || super::tb03c_transfer::is_transfer(event);
    if du == 0 {
        if !special && (comps.is_empty() || comps.iter().any(|c| !DU0_KINDS.contains(&kind(c)))) {
            return Err(
                "SchemaUnsupported：零数量变化只许费用独立件、本金、退本金、分红或公司行动原件"
                    .into(),
            );
        }
    } else if special && !super::tb03c_transfer::is_transfer(event) {
        return Err("SchemaUnsupported：公司行动与估值原件的数量变化须为零".into());
    } else if comps.iter().any(|c| DU0_ONLY.contains(&kind(c))) {
        return Err("SchemaUnsupported：费用独立件、退本金与分红只能是零数量变化原件".into());
    }
    Ok(())
}

/// 唯一扩量资格谓词（ADR 0031 裁定一/三）。无本金批 ⟹ NotEvaluable（不把缺本金当 0）。
pub(crate) fn growth_eligible(
    long: bool,
    k: i64,
    principal: Option<i64>,
    p_recovered: i64,
    realized_after_fees: i64,
) -> &'static str {
    let Some(p) = principal else {
        return "NotEvaluable";
    };
    let eligible = if long {
        k <= 0 && p_recovered >= p
    } else {
        realized_after_fees >= p
    };
    if eligible {
        "Eligible"
    } else {
        "NotEligible"
    }
}

struct Numbers {
    long: bool,
    k: i64,
    principal: Option<i64>,
    p_recovered: i64,
    gross: i64,
    fees: i64,
}
impl Numbers {
    fn of(book: &Value) -> Result<Self, String> {
        let t = &book["cost_totals"];
        let n = |k: &str| integer(t, k);
        let k = sub(
            add(
                add(
                    add(n("acquisition_cash")?, n("commission")?)?,
                    n("fee_only")?,
                )?,
                0,
            )?,
            add(n("disposal_cash")?, n("dividend_cash")?)?,
        )?;
        let transfer = |field: &str| -> Result<i64, String> {
            if book.get(field).is_none() {
                Ok(0)
            } else {
                integer(book, field)
            }
        };
        let k = sub(
            add(k, transfer("transfer_cost_in")?)?,
            transfer("transfer_cost_out")?,
        )?;
        let principal = if book["principal"]["batches"]
            .as_array()
            .ok_or("StorageUnavailable：本金批次形状")?
            .is_empty()
        {
            None
        } else {
            Some(
                book["principal"]["batches"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .try_fold(0i64, |a, b| add(a, integer(b, "amount")?))?,
            )
        };
        Ok(Self {
            long: book["side"] == "long",
            k,
            principal,
            p_recovered: integer(&book["principal"], "P_recovered")?,
            gross: integer(&book["realized"], "gross")?,
            fees: add(n("commission")?, n("fee_only")?)?,
        })
    }
    fn after_fees(&self) -> Result<i64, String> {
        sub(self.gross, self.fees)
    }
    fn status(&self) -> Result<&'static str, String> {
        Ok(growth_eligible(
            self.long,
            self.k,
            self.principal,
            self.p_recovered,
            self.after_fees()?,
        ))
    }
    fn cost_remaining(&self) -> Result<Option<i64>, String> {
        if self.long {
            return Ok(Some(self.k));
        }
        self.principal
            .map(|p| sub(p, self.after_fees()?))
            .transpose()
    }
}

/// 对已存书账重算资格（#804：同一函数、同一输入；只供同输入同输出锁）。
#[cfg(test)]
pub(crate) fn growth_eligible_of(book: &Value) -> Result<Value, String> {
    Ok(json!(Numbers::of(book)?.status()?))
}

fn open_book(
    doc: &Value,
    event: &Value,
    record: &Value,
    v2: bool,
    funding: Option<Value>,
) -> Result<Value, String> {
    let transfer = super::tb03c_transfer::is_in(event);
    let transferred_income = json!({"Pi":"0","A":"0","W":"0"});
    let o = if transfer {
        &transferred_income
    } else {
        &event["observed_existing_book"]
    };
    let id = if transfer {
        &event["transfer_opening"]
    } else {
        &event["book_identity"]
    };
    let units = if transfer {
        nonnegative(
            required(&event["transfer"]["claim"]["payload"], "units")?,
            "transfer units",
        )?
    } else {
        positive(required(o, "units")?, "opening_units")?
    };
    // 书账版本取自域规则（评审 M4）：`/2` 域里转移开出的重同样盖 `/2`。
    let revision = if v2 {
        super::book::TB03C_BOOK_V2
    } else {
        super::book::TB03C_BOOK_V1
    };
    let mut book = json!({
        "book_revision": revision,
        "chong_id": target(event)?, "symbol": id["symbol"], "operation_level": id["op_level"],
        "fixed_q": id["fixed_q"], "side": id["side"], "units": doc["units"], "family_id": event["family_id"],
        "opening_fact_ref": {"fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"e_cut":record["cut"],"json_pointer":record["json_pointer"],"raw_sha256":record["raw_sha256"]},
        "opening_quantity": units.to_string(), "actual_quantity": units.to_string(),
        "income_book": {"Pi":o["Pi"],"A":o["A"],"W":o["W"],"R":reserve(integer(o,"Pi")?,integer(o,"A")?,integer(o,"W")?)?.to_string()},
        "known_cost_components": [],
        "cost_totals": TOTALS.iter().map(|k| (k.to_string(), json!("0"))).collect::<serde_json::Map<_, _>>(),
        "realized": {"gross":"0"},
        "principal": {"batches":[],"P_recovered":"0"},
        "phase": {"latch":"CostReduction","events":[],"gate":null},
        "unknown": {"complete_historical_fees":{"status":"Unknown","reason":"only named known components available"},"target":{"status":"Unknown","reason":"not supplied"}},
        "voice": {"status":"MissingDependency","reason":"confirmed gamma binding absent","identity":null},
        "admission": "NotEvaluated"
    });
    // #1406 C8：`/2` 开局把核过的资金出处记入书账；`/1` 书账形状逐字不变。
    if let Some(f) = funding {
        book["funding"] = f;
    }
    Ok(book)
}

fn split(book: &mut Value, ca: &Value) -> Result<(), String> {
    let num = positive(required(ca, "num")?, "split.num")?;
    let den = positive(required(ca, "den")?, "split.den")?;
    let mut rebased = [0i64; 2];
    for (slot, field) in rebased.iter_mut().zip(["actual_quantity", "fixed_q"]) {
        let scaled = integer(book, field)?
            .checked_mul(num)
            .ok_or("InvalidDomain：拆股重基溢出")?;
        if scaled % den != 0 {
            return Err("Unsupported：拆股比不整除且原件未带零股处置（ADR 0031 裁定五）".into());
        }
        *slot = scaled / den;
    }
    book["actual_quantity"] = json!(rebased[0].to_string());
    book["fixed_q"] = json!(rebased[1].to_string());
    Ok(())
}

fn income(book: &mut Value, d: &Value) -> Result<(), String> {
    let l = &book["income_book"];
    let mut t = (
        integer(l, "Pi")?,
        integer(l, "A")?,
        integer(l, "W")?,
        integer(l, "R")?,
    );
    if reserve(t.0, t.1, t.2)? != t.3 {
        return Err("StorageUnavailable：域损坏：R恒等失败".into());
    }
    for e in [
        LedgerEvent::Realize(integer(d, "Pi")?),
        LedgerEvent::Allocate(integer(d, "A")?),
        LedgerEvent::Withdraw(integer(d, "W")?),
    ] {
        match e {
            LedgerEvent::Realize(x) => {
                add(t.0, x)?;
                add(t.3, x)?;
            }
            LedgerEvent::Allocate(x) => {
                add(t.1, x)?;
                sub(t.3, x)?;
            }
            LedgerEvent::Withdraw(x) => {
                add(t.2, x)?;
                sub(t.3, x)?;
            }
            LedgerEvent::Noop => (),
        }
        t = ledger_step_components(t, e);
        if reserve(t.0, t.1, t.2)? != t.3 {
            return Err("StorageUnavailable：域损坏：更新后R恒等失败".into());
        }
    }
    book["income_book"] =
        json!({"Pi":t.0.to_string(),"A":t.1.to_string(),"W":t.2.to_string(),"R":t.3.to_string()});
    Ok(())
}

fn components(book: &mut Value, comps: &[Value], record: &Value) -> Result<(), String> {
    component_effects(book, comps, record, false)
}

// 反冲方向仅由已验证的 E 派生分配提供，原 raw 分量不能自行启用。
fn component_effects(
    book: &mut Value,
    comps: &[Value],
    record: &Value,
    reversal: bool,
) -> Result<(), String> {
    for c in comps {
        let k = kind(c);
        let amount = nonnegative(required(c, "canonical_amount")?, "canonical_amount")?;
        let amount = if reversal { -amount } else { amount };
        match k {
            // #1406 C5：A 只由 income() 入账一次；资本化不改变本金或成本 K。
            "growth_capitalization" => (),
            "principal_contribution" => {
                let batch = required(c, "batch_id")?;
                let batches = book["principal"]["batches"].as_array_mut().unwrap();
                if batches.iter().any(|b| b["batch_id"] == batch) {
                    return Err("IdentityConflict：本金批次身份重复".into());
                }
                batches.push(json!({"batch_id":batch,"amount":amount.to_string(),"authorization_ref":c["authorization_ref"],"fact_key":record["fact_key"]}));
            }
            "recover_capital" => {
                let p = Numbers::of(book)?
                    .principal
                    .ok_or("MissingDependency：退本金须先有已授权本金批（ADR 0031 裁定一）")?;
                let next = add(integer(&book["principal"], "P_recovered")?, amount)?;
                if next > p {
                    return Err("InvalidDomain：退本金超过已登记本金；超出部分属利润提取 W".into());
                }
                book["principal"]["P_recovered"] = json!(next.to_string());
            }
            _ => {
                let total = add(integer(&book["cost_totals"], k)?, amount)?;
                book["cost_totals"][k] = json!(total.to_string());
            }
        }
        let mut sourced = c.clone();
        sourced["fact_key"] = record["fact_key"].clone();
        sourced["raw_sha256"] = record["raw_sha256"].clone();
        book["known_cost_components"]
            .as_array_mut()
            .unwrap()
            .push(sourced);
    }
    Ok(())
}

/// 由已更新的原始量重新导出全部派生栏，并按变化追加阶段事件史。
fn derive(
    book: &mut Value,
    before: Option<(Value, Option<i64>)>,
    seq: i64,
    ns: &str,
    event: &Value,
    record: &Value,
) -> Result<(), String> {
    let n = Numbers::of(book)?;
    let status = n.status()?;
    let remaining = n.cost_remaining()?;
    let sp = json!({"profile":event["stable_point"]["profile"],"seq":event["stable_point"]["seq"]});
    let mut events = book["phase"]["events"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let stamp = |mut e: Value| {
        e["stable_point"] = sp["seq"].clone();
        e["commit_seq"] = json!(seq.to_string());
        e["fact_key"] = record["fact_key"].clone();
        e["cost_remaining"] = remaining.map_or(Value::Null, |v| json!(v.to_string()));
        e
    };
    if let Some((prev_status, prev_remaining)) = before {
        if prev_status != status {
            events.push(stamp(
                json!({"kind":"growth_eligible","from":prev_status,"to":status}),
            ));
        }
        if let (Some(a), Some(b)) = (prev_remaining, remaining) {
            if (a > 0) != (b > 0) {
                let kind = if b > 0 {
                    "cost_remaining_positive"
                } else {
                    "cost_remaining_nonpositive"
                };
                events.push(stamp(json!({ "kind": kind })));
            }
        }
    }
    let prev_rank = LATCH
        .iter()
        .position(|x| book["phase"]["latch"] == *x)
        .ok_or("StorageUnavailable：latch 形状")?;
    let mut rank = 0;
    if n.long && n.principal.is_some_and(|p| n.p_recovered >= p) {
        rank = 1;
    }
    if status == "Eligible" {
        rank = if n.long { 2 } else { 1 };
    }
    let mut gate = book["phase"]["gate"].clone();
    if status == "Eligible" && gate.is_null() {
        gate = stamp(
            json!({"kind":"SimulatedGateReached","profile":"TestOnly","commit_ns":ns,
            "P":n.principal.map(|p| p.to_string()),"P_recovered":n.p_recovered.to_string()}),
        );
    }
    let enabled = match status {
        "Eligible" => json!({"status":"AwaitingPolicy","slot":"post_gate_params"}),
        "NotEligible" => json!({"status":"NotEligible"}),
        _ => json!({"status":"NotEvaluable","reason":"principal batches absent"}),
    };
    book["phase"] = json!({"growth_eligible":{"status":status},"latch":LATCH[prev_rank.max(rank)],
        "events":events,"gate":gate,"growth_enabled":enabled});
    book["realized"] = json!({"gross":n.gross.to_string(),"fees":n.fees.to_string(),"after_fees":n.after_fees()?.to_string()});
    book["principal"]["status"] = json!(if n.principal.is_some() {
        "Known"
    } else {
        "NotEvaluable"
    });
    match n.principal {
        Some(p) => book["principal"]["P"] = json!(p.to_string()),
        None => book["principal"]["reason"] = json!("no authorized principal batch"),
    }
    book["cost_remaining"] = match remaining {
        Some(v) => json!({"status":"Known","value":v.to_string()}),
        None => json!({"status":"NotEvaluable","reason":"principal batches absent"}),
    };
    let q = integer(book, "actual_quantity")?;
    if n.long {
        book["cost_basis"] = json!({"status":"Known","K":n.k.to_string()});
        book["unit_cost"] = if q == 0 {
            json!({"status":"NotComputable","reason":"quantity_zero","K":n.k.to_string(),"q":"0"})
        } else {
            json!({"status":"Known","K":n.k.to_string(),"q":q.to_string()})
        };
    } else {
        book["cost_basis"] =
            json!({"status":"NotApplicable","reason":"short_side_has_no_cost_basis"});
        book["unit_cost"] =
            json!({"status":"NotComputable","reason":"short_side_has_no_unit_cost"});
    }
    book["stable_point"] = sp;
    Ok(())
}

/// tb03c-book/1 的 B 应用：同一原件、同一图像 ⟹ 同一结果；失败不改任何状态。
pub(super) fn apply(
    image: &Value,
    record: &Value,
    seq: i64,
    ns: &str,
) -> Result<(Value, bool), String> {
    apply_with(image, record, seq, ns, None)
}

/// tb03c-book/2 的 B 应用：`/1` 全部规则 + manifest 冻结资金合同（ADR 0040 裁定四第 4/5 步）。
pub(super) fn apply_v2(
    image: &Value,
    record: &Value,
    seq: i64,
    ns: &str,
    funding: &super::tb03c_funding::Contract,
) -> Result<(Value, bool), String> {
    apply_with(image, record, seq, ns, Some(funding))
}

fn apply_with(
    image: &Value,
    record: &Value,
    seq: i64,
    ns: &str,
    funding: Option<&super::tb03c_funding::Contract>,
) -> Result<(Value, bool), String> {
    let (doc, event) = decode(
        required(record, "raw_utf8")?,
        required(record, "json_pointer")?,
        if funding.is_some() {
            Rules::Tb03cV2
        } else {
            Rules::Tb03cV1
        },
    )?;
    if super::tb03c_allocation::is_projected(&event) {
        return apply_projected(image, record, &doc, &event, seq, ns);
    }
    if event.get("valuation_observation").is_some() {
        return Err("SchemaUnsupported：估值观察只记 E，不进 B 成本账（C05.4）".into());
    }
    if event.get("q_rebase").is_some() {
        if let Some(f) = funding {
            // ADR 0040 裁定四末段：Q 变大不自动改现役 Q，输出 AwaitingPolicy{q_rebase, (S,ℓ)}。
            let c = f.chong(target(&event)?)?;
            return Err(format!(
                "AwaitingPolicy：q_rebase (S,ℓ)=({},{})；现役 Q 不自动改写，须签新版本",
                c.symbol(),
                c.level()
            ));
        }
        return Err("AwaitingPolicy：capacity；C(ℓ) 未签署，fixed_q 不变".into());
    }
    // #1406 C4 / ADR 0032 五：E 已解析并不代表本 B 已应用前件。费用/更正沿因果原件等待，
    // 不把声明了依赖的缺开局永久记为 Unmatched，也不以迟到件触发新 Voice。
    if let Some(dependency) = super::tb03c_e::dependency_key(&event)? {
        if !array(image, "applied")?
            .iter()
            .any(|a| a["fact_key"] == dependency && a["status"] == "Applied")
        {
            return Err("AwaitingDependency：本重尚未持久应用声明前件".into());
        }
    }
    super::tb03c_e::check_fill_predecessor(&event, array(image, "recorded")?)?;
    let mut next = image.clone();
    let opening =
        event.get("observed_existing_book").is_some() || super::tb03c_transfer::is_in(&event);
    let old = &image["book"];
    if super::tb03c_transfer::is_in(&event) && record.get("b_source_debit_proof").is_none() {
        return Err("AwaitingDependency：转移目标缺源 B 已持久扣账证明".into());
    }
    if event["transfer"]["role"] == "out" && old.is_null() {
        return Err("AwaitingDependency：源 B 尚无开局和具名增长来源，不建收据".into());
    }
    if opening && !old.is_null() {
        return Err("IdentityConflict：持久重已非空，不可更换开局".into());
    }
    // #1406 C8：`/2` 开局批按 ADR 0040 裁定四第 4 步核；追加批须先签新版本。转移目标的 P0 来自转移
    // （ADR 0034 裁定六），不属本 profile 的 funding-record 开局批。
    let funded = match funding {
        Some(f) if super::tb03c_funding::is_appended_contribution(&event) => {
            let c = f.chong(target(&event)?)?;
            return Err(format!(
                "AwaitingPolicy：funding_revision (S,ℓ)=({},{})；追加投入须签新版 funding-record",
                c.symbol(),
                c.level()
            ));
        }
        Some(f) if event.get("observed_existing_book").is_some() => {
            Some(f.check_opening(&event, record)?)
        }
        _ => None,
    };
    // 根裁定 OQ-3：Unmatched 原因闭集 = {family 不符, per_leg 缺归属, 未声明依赖的缺开局}。
    let unmatched = if opening {
        None
    } else if old.is_null() {
        Some((
            "opening_missing_undeclared",
            "MissingDependency: opening absent and no declared dependency",
        ))
    } else if old["family_id"] != event["family_id"] {
        Some((
            "family_mismatch",
            "IdentityConflict: fact family differs from the book opening family",
        ))
    } else {
        None
    };
    if let Some((code, reason)) = unmatched {
        push(
            &mut next,
            "unmatched",
            json!({"fact_key":record["fact_key"],"allocation_id":event["allocation_id"],"reason_code":code,"reason":reason,"commit_seq":seq.to_string(),"commit_ns":ns}),
        );
    } else {
        let mut book = if opening {
            open_book(&doc, &event, record, funding.is_some(), funded.clone())?
        } else {
            old.clone()
        };
        if book["units"] != doc["units"] {
            return Err("InvalidDomain：重的量纲不可变".into());
        }
        let comps = array(&event, "known_cost_components")?;
        if book["side"] == "short" && comps.iter().any(|c| !SHORT_KINDS.contains(&kind(c))) {
            return Err(
                "SchemaUnsupported：最小空头书账只收佣金、费用独立件与保证金批（根裁定 OQ-5）"
                    .into(),
            );
        }
        if (comps.iter().any(|c| kind(c) == "growth_capitalization")
            || event.get("completed_growth").is_some())
            && Numbers::of(&book)?.status()? != "Eligible"
        {
            return Err("Rejected：增长资本化前件 growth_eligible 不成立".into());
        }
        let before = if opening {
            None
        } else {
            let n = Numbers::of(&book)?;
            Some((json!(n.status()?), n.cost_remaining()?))
        };
        if !opening {
            if let Some(ca) = event.get("corporate_action") {
                split(&mut book, ca)?;
            }
            let d = &event["deltas"];
            let q = add(integer(&book, "actual_quantity")?, integer(d, "units")?)?;
            if q < 0 {
                return Err("InvalidDomain：重内不穿零：持量/欠券量不可为负".into());
            }
            book["actual_quantity"] = json!(q.to_string());
            income(&mut book, d)?;
            if let Some(p) = event.get("declared_realized_profit") {
                let gross = add(integer(&book["realized"], "gross")?, integer(p, "gross")?)?;
                book["realized"]["gross"] = json!(gross.to_string());
            }
        }
        super::tb03c_transfer::source_effect(image, &event, &mut book)?;
        super::tb03c_transfer::target_effect(&event, record, &mut book)?;
        components(&mut book, comps, record)?;
        if opening && book["side"] == "short" && Numbers::of(&book)?.principal.is_none() {
            return Err("MissingDependency：空头开局须带保证金批 M0（ADR 0031 裁定一）".into());
        }
        derive(&mut book, before, seq, ns, &event, record)?;
        next["book"] = book;
    }
    push(&mut next, "recorded", record.clone());
    let a = json!({"fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"allocation_id":event["allocation_id"],"target_domain":record["target_domain"],"status":if unmatched.is_some(){"Unmatched"}else{"Applied"},"commit_seq":seq.to_string(),"commit_ns":ns});
    push(&mut next, "applied", a.clone());
    push(
        &mut next,
        "history",
        json!({"kind":if unmatched.is_some(){"Unmatched"}else{"ApplyAllocation"},"application":a,"source":event["source"],"event":event,"commit_seq":seq.to_string(),"commit_ns":ns}),
    );
    Ok((next, unmatched.is_some()))
}

/// 净额分配是 E 证明的派生量，不重新包装成带原始见证的逐重成交。
fn apply_projected(
    image: &Value,
    record: &Value,
    doc: &Value,
    event: &Value,
    seq: i64,
    ns: &str,
) -> Result<(Value, bool), String> {
    let allocation = record
        .get("resolved_allocation")
        .ok_or("AwaitingDependency：缺 E 逐重分配证明")?;
    let id = super::tb03c_allocation::application_id(record)?;
    let domain = super::tb03c_allocation::application_domain(record)?;
    let old = &image["book"];
    let per_leg = event.get("per_leg_report").is_some();
    // 同一冻结投影的前序报告须先在本 B 应用，包括零份额报告。
    if let Some(prior) = record["e_resolution_proof"]["image"]["fill_allocations"]
        .as_array()
        .filter(|_| !per_leg)
    {
        for row in prior
            .iter()
            .take_while(|r| r["fact_key"] != record["fact_key"])
            .filter(|r| r["manifest_hash"] == event["netted_fill"]["manifest_hash"])
        {
            if !array(image, "applied")?
                .iter()
                .any(|a| a["fact_key"] == row["fact_key"] && a["status"] == "Applied")
            {
                return Err("AwaitingDependency：本重尚未应用同一冻结投影前序分配".into());
            }
        }
    }
    super::tb03c_corrections::predecessor(event, array(image, "recorded")?)?;
    let mut next = image.clone();
    let unmatched = if per_leg && allocation.get("unmatched_reason").is_some() {
        Some("per_leg_attribution_missing")
    } else if old.is_null() {
        Some("opening_missing_undeclared")
    } else if old["family_id"] != allocation["family_id"] {
        Some("family_mismatch")
    } else {
        None
    };
    if let Some(code) = unmatched {
        push(
            &mut next,
            "unmatched",
            json!({"fact_key":record["fact_key"],"allocation_id":id,"reason_code":code,"reason":code,"commit_seq":seq.to_string(),"commit_ns":ns}),
        );
    } else {
        let report = if per_leg {
            &event["per_leg_report"]
        } else {
            &event["netted_fill"]
        };
        if old["side"] != "long"
            || old["symbol"] != report["symbol"]
            || old["chong_id"] != allocation["chong_id"]
            || old["units"] != doc["units"]
        {
            return Err("IdentityConflict：净额购入分配与重的方向/身份/量纲不符".into());
        }
        let mut book = old.clone();
        let n = Numbers::of(old)?;
        let before = Some((json!(n.status()?), n.cost_remaining()?));
        let units = integer(allocation, "units")?;
        let quantity = add(integer(old, "actual_quantity")?, units)?;
        if quantity < 0 {
            return Err("InvalidDomain：冲销后数量不得为负".into());
        }
        book["actual_quantity"] = json!(quantity.to_string());
        // netted 显式原件已证明 Pi/A/W 均为零；per_leg 形状不声明收益账变化。
        if !per_leg {
            income(&mut book, &event["netted_fill"]["income_changes"])?;
        }
        let mut comps = Vec::new();
        let correction = super::tb03c_corrections::is_correction(event);
        if correction {
            let kind = required(allocation, "component")?;
            let amount = &allocation[kind];
            comps.push(json!({"kind":kind,"canonical_amount":amount,"derivation":{"kind":"E_resolved_netted_correction","allocation_id":id,"manifest_hash":event["netted_fill"]["manifest_hash"],"original_fact_key":allocation["original_fact_key"],"reversal":allocation["reversal"],"resolve_cut":record["e_resolution_proof"]["receipt"]["cut"]}}));
        } else if units > 0 {
            for kind in ["acquisition_cash", "commission"] {
                let derivation = if per_leg {
                    "E_resolved_per_leg_allocation"
                } else {
                    "E_resolved_netted_allocation"
                };
                comps.push(json!({"kind":kind,"canonical_amount":allocation[kind],"derivation":{"kind":derivation,"allocation_id":id,"manifest_hash":report["manifest_hash"],"resolve_cut":record["e_resolution_proof"]["receipt"]["cut"]}}));
            }
        } else if integer(allocation, "acquisition_cash")? != 0
            || integer(allocation, "commission")? != 0
        {
            return Err("IdentityConflict：零份额不得携带分配现金/费用".into());
        }
        component_effects(
            &mut book,
            &comps,
            record,
            correction && allocation["reversal"] == true,
        )?;
        derive(&mut book, before, seq, ns, event, record)?;
        next["book"] = book;
    }
    push(&mut next, "recorded", record.clone());
    let mut a = json!({"fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"allocation_id":id,"target_domain":domain,"status":if unmatched.is_some(){"Unmatched"}else{"Applied"},"commit_seq":seq.to_string(),"commit_ns":ns});
    if per_leg || super::tb03c_corrections::is_correction(event) {
        let row = array(&record["e_resolution_proof"]["image"], "fill_allocations")?
            .iter()
            .find(|r| r["fact_key"] == record["fact_key"])
            .ok_or("StorageUnavailable：缺政策证明")?;
        for key in [
            "manifest_hash",
            "fill_allocation_policy_id",
            "policy_revision",
        ] {
            a[key] = row[key].clone();
        }
        if let Some(reason) = unmatched {
            let item = next["unmatched"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap();
            item["reason_code"] = json!(reason);
            for key in [
                "manifest_hash",
                "fill_allocation_policy_id",
                "policy_revision",
            ] {
                item[key] = row[key].clone();
            }
        }
    }
    push(&mut next, "applied", a.clone());
    push(
        &mut next,
        "history",
        json!({"kind":if unmatched.is_some(){"Unmatched"}else{"ApplyAllocation"},"application":a,"source":event["source"],"event":event,"resolved_allocation":allocation,"commit_seq":seq.to_string(),"commit_ns":ns}),
    );
    Ok((next, unmatched.is_some()))
}
