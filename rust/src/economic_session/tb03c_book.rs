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
use super::book::{add, array, decode, integer, push, reserve, sub, target, Rules};
use crate::session_protocol::{nonnegative, positive, required};
use crate::theta_v0::strategy::ledger::{ledger_step_components, LedgerEvent};
use serde_json::{json, Value};

/// 旧白名单（acquisition_cash / commission）之外，本版新增的成本分量类型。
pub(super) const COMPONENT_KINDS: [&str; 6] = [
    "disposal_cash",
    "fee_only",
    "dividend_cash",
    "withholding_tax",
    "principal_contribution",
    "recover_capital",
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
const DU0_KINDS: [&str; 5] = [
    "fee_only",
    "principal_contribution",
    "recover_capital",
    "dividend_cash",
    "withholding_tax",
];
/// 只能出现在零数量变化原件里的分量。
const DU0_ONLY: [&str; 4] = [
    "fee_only",
    "recover_capital",
    "dividend_cash",
    "withholding_tax",
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
    let comps = array(event, "known_cost_components")?;
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
    let special =
        event.get("corporate_action").is_some() || event.get("valuation_observation").is_some();
    if du == 0 {
        if !special && (comps.is_empty() || comps.iter().any(|c| !DU0_KINDS.contains(&kind(c)))) {
            return Err(
                "SchemaUnsupported：零数量变化只许费用独立件、本金、退本金、分红或公司行动原件"
                    .into(),
            );
        }
    } else if special {
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

fn open_book(doc: &Value, event: &Value, record: &Value) -> Result<Value, String> {
    let o = &event["observed_existing_book"];
    let id = &event["book_identity"];
    let units = positive(required(o, "units")?, "opening_units")?;
    Ok(json!({
        "book_revision": super::book::TB03C_BOOK_V1,
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
    }))
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
    for c in comps {
        let k = kind(c);
        let amount = nonnegative(required(c, "canonical_amount")?, "canonical_amount")?;
        match k {
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
    let (doc, event) = decode(
        required(record, "raw_utf8")?,
        required(record, "json_pointer")?,
        Rules::Tb03cV1,
    )?;
    if event.get("valuation_observation").is_some() {
        return Err("SchemaUnsupported：估值观察只记 E，不进 B 成本账（C05.4）".into());
    }
    let mut next = image.clone();
    let opening = event.get("observed_existing_book").is_some();
    let old = &image["book"];
    if opening && !old.is_null() {
        return Err("IdentityConflict：持久重已非空，不可更换开局".into());
    }
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
            open_book(&doc, &event, record)?
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
