//! #1406 C4b：迟到费用与具名冲销。只从原 fill/原 E 分配读取金额依据。
//! 跨多笔原 fill 的总量 bust 按 ADR 0032 裁定六（#1683）：逐重单位数 = A(F) − A(F−b)，
//! 每单位原价取其累计位置所在的存续区间，不引入 FIFO、平均价或另设拆分。
use super::book::{array, integer};
use crate::session_protocol::{nonnegative, positive, required};
use serde_json::Value;

pub(super) fn is_correction(event: &Value) -> bool {
    matches!(
        event["netted_fill"]["report_kind"].as_str(),
        Some("fee_only" | "bust" | "reverse_component")
    )
}

pub(super) fn decode(
    event: &Value,
    pointer: &str,
    check: &dyn Fn(&str, &str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    let f = &event["netted_fill"];
    let mut fields = vec![
        "manifest_hash",
        "symbol",
        "report_kind",
        "fill_ref",
        "income_changes",
        "source_ref",
    ];
    match required(f, "report_kind")? {
        "fee_only" => {
            fields.extend(["amount", "fee_type", "cause"]);
            positive(required(f, "amount")?, "fee amount")?;
            required(f, "fee_type")?;
            if f["cause"] != "fill" {
                return Err("Unsupported：净额独立费用本片只支持 fill 因果".into());
            }
        }
        "bust" => {
            fields.extend(["from", "to", "reported_quantity", "component"]);
            let from = positive(required(f, "from")?, "bust.from")?;
            let to = nonnegative(required(f, "to")?, "bust.to")?;
            if to >= from
                || positive(required(f, "reported_quantity")?, "bust.quantity")? != from - to
            {
                return Err("IdentityConflict：bust 数量与逆区间不符".into());
            }
            if f["component"] != "acquisition_cash" {
                return Err("Unsupported：数量 bust 只反冲具名采购现金，不隐含退费".into());
            }
        }
        "reverse_component" => {
            fields.push("component");
            if f["component"] != "commission" {
                return Err(
                    "Unsupported：本片原子分量逆规则只支持原 fill commission 全额反冲".into(),
                );
            }
        }
        _ => return Err("SchemaUnsupported：未知更正类型".into()),
    }
    super::voice::exact(f, &fields)?;
    let reference = &f["fill_ref"];
    super::voice::exact(
        reference,
        &[
            "source_namespace",
            "source_epoch",
            "external_fact_id",
            "allocation_id",
            "manifest_hash",
            "fill_allocation_policy_id",
            "policy_revision",
        ],
    )?;
    for key in [
        "source_namespace",
        "source_epoch",
        "external_fact_id",
        "allocation_id",
        "manifest_hash",
        "fill_allocation_policy_id",
        "policy_revision",
    ] {
        required(reference, key)?;
    }
    super::tb03c_e::check_depends_on(event, pointer, check)?;
    let dependency = event
        .get("depends_on")
        .ok_or("MissingDependency：费用/更正须声明具名原 fill")?;
    if ["source_namespace", "source_epoch", "external_fact_id"]
        .iter()
        .any(|k| reference[k] != dependency[k])
        || reference["manifest_hash"] != f["manifest_hash"]
    {
        return Err("IdentityConflict：费用/更正具名原 fill 与依赖/清单不符".into());
    }
    Ok(())
}

pub(super) fn predecessor<'a>(
    event: &Value,
    records: &'a [Value],
) -> Result<Option<&'a Value>, String> {
    if !is_correction(event) {
        return Ok(None);
    }
    let key = super::tb03c_e::dependency_key(event)?.ok_or("MissingDependency：费用/更正缺依赖")?;
    let record = records
        .iter()
        .find(|r| r["fact_key"] == key)
        .ok_or("AwaitingDependency：具名原 fill 尚未持久接纳/应用")?;
    let original = &record["event"];
    let original_fill = &original["netted_fill"];
    let reference = &event["netted_fill"]["fill_ref"];
    if !matches!(
        original_fill["report_kind"].as_str(),
        Some("incremental" | "cumulative")
    ) || integer(original_fill, "from")? >= integer(original_fill, "to")?
    {
        return Err("IdentityConflict：具名前件不是实际非零净额 fill".into());
    }
    if ["source_namespace", "source_epoch", "external_fact_id"]
        .iter()
        .any(|k| reference[k] != original["source"][k])
        || reference["allocation_id"] != original["allocation_id"]
        || reference["manifest_hash"] != original_fill["manifest_hash"]
        || event["netted_fill"]["symbol"] != original_fill["symbol"]
    {
        return Err("IdentityConflict：具名原 fill 身份/清单/标的不符".into());
    }
    Ok(Some(record))
}

pub(super) struct Effects {
    pub units: Vec<i64>,
    pub cash: Vec<i64>,
    pub fees: Vec<i64>,
    pub from: i64,
    pub to: i64,
    pub component: &'static str,
    pub reverse: bool,
    pub original_fact_key: Value,
}

struct LiveRange {
    start: i64,
    end: i64,
    price: i64,
    fact_key: Value,
}

/// 把存续区间栈从顶 `from` 截到 `to`，返回被撤回的段（自底向上）。#1683 / ADR 0032 裁定六：
/// 无指向的总量 b 可跨多笔原 fill；每单位原价按其累计位置所在的存续区间取回，不另设拆分。
/// 具名原 fill 只须是被撤回段的来源之一。
fn rewind(
    ranges: &mut Vec<LiveRange>,
    from: i64,
    to: i64,
    named: &Value,
) -> Result<Vec<LiveRange>, String> {
    if ranges.last().map(|r| r.end) != Some(from) || to >= from {
        return Err("MissingDependency：bust 不在当前存续区间顶端".into());
    }
    let mut removed = Vec::new();
    while let Some(top) = ranges.last_mut() {
        if top.end <= to {
            break;
        }
        let start = top.start.max(to);
        removed.push(LiveRange {
            start,
            end: top.end,
            price: top.price,
            fact_key: top.fact_key.clone(),
        });
        top.end = start;
        if top.start == top.end {
            ranges.pop();
        }
    }
    if removed.last().map(|r| r.start) != Some(to) {
        return Err("InvalidDomain：bust 超出全部存续数量区间".into());
    }
    if !removed.iter().any(|r| &r.fact_key == named) {
        return Err("IdentityConflict：bust 具名原 fill 不是当前存续逆区间的来源".into());
    }
    removed.reverse();
    Ok(removed)
}

/// 从 append-only 原回报与更正恢复当前存续的累计区间；金额仍取具名原 fill。
/// 这是按实际已发生的撤回重放来源，不是为跨 fill 现金挑选 FIFO/LIFO 政策。
fn surviving_ranges(image: &Value, manifest_hash: &Value) -> Result<Vec<LiveRange>, String> {
    let mut ranges: Vec<LiveRange> = Vec::new();
    let mut frontier = 0;
    for row in array(image, "fill_allocations")?
        .iter()
        .filter(|r| &r["manifest_hash"] == manifest_hash)
    {
        let fact = array(image, "recorded")?
            .iter()
            .find(|r| r["fact_key"] == row["fact_key"])
            .ok_or("StorageUnavailable：存续区间缺原始持久回报")?;
        let event = &fact["event"];
        let fill = &event["netted_fill"];
        let from = integer(row, "from")?;
        let to = integer(row, "to")?;
        if from != frontier {
            return Err("StorageUnavailable：存续区间前沿不连续".into());
        }
        match required(fill, "report_kind")? {
            "incremental" | "cumulative" => {
                if from != integer(fill, "from")? || to != integer(fill, "to")? || to <= from {
                    return Err("StorageUnavailable：存续区间与原 fill 不符".into());
                }
                ranges.push(LiveRange {
                    start: from,
                    end: to,
                    price: integer(fill, "unit_price")?,
                    fact_key: fact["fact_key"].clone(),
                });
            }
            "bust" => {
                let original = predecessor(event, array(image, "recorded")?)?
                    .ok_or("StorageUnavailable：持久 bust 无具名原 fill")?;
                if from != integer(fill, "from")?
                    || to != integer(fill, "to")?
                    || to >= from
                    || row["original_fact_key"] != original["fact_key"]
                {
                    return Err("StorageUnavailable：存续区间与原 bust 因果不符".into());
                }
                rewind(&mut ranges, from, to, &original["fact_key"]).map_err(|e| {
                    format!("StorageUnavailable：持久 bust 不能按存续区间栈重放（{e}）")
                })?;
            }
            "fee_only" | "reverse_component" => {
                if to != from {
                    return Err("StorageUnavailable：零数量分量改变了存续前沿".into());
                }
            }
            _ => return Err("StorageUnavailable：存续来源含未知回报类型".into()),
        }
        frontier = to;
    }
    Ok(ranges)
}

pub(super) fn resolve(
    event: &Value,
    image: &Value,
    frontier: i64,
    weights: &[i64],
    ranks: &[usize],
) -> Result<Effects, String> {
    let f = &event["netted_fill"];
    let original =
        predecessor(event, array(image, "recorded")?)?.ok_or("IdentityConflict：非更正原件")?;
    let rows =
        array(image, "fill_allocations").map_err(|_| "AwaitingDependency：原 fill 尚无持久分配")?;
    let original_row = rows
        .iter()
        .find(|r| r["fact_key"] == original["fact_key"])
        .ok_or("AwaitingDependency：具名原 fill 尚未 Resolve")?;
    let reference = &f["fill_ref"];
    if [
        "manifest_hash",
        "fill_allocation_policy_id",
        "policy_revision",
    ]
    .iter()
    .any(|k| reference[k] != original_row[k])
    {
        return Err("IdentityConflict：更正政策三元组不是原 fill 的冻结政策".into());
    }
    let allocations = array(original_row, "allocations")?;
    if allocations.len() != weights.len() {
        return Err("StorageUnavailable：原 fill 分配长度不符".into());
    }
    let mut result = Effects {
        units: vec![0; weights.len()],
        cash: vec![0; weights.len()],
        fees: vec![0; weights.len()],
        from: frontier,
        to: frontier,
        component: "fee_only",
        reverse: false,
        original_fact_key: original["fact_key"].clone(),
    };
    match required(f, "report_kind")? {
        "fee_only" => {
            let original_units = allocations
                .iter()
                .map(|a| integer(a, "units"))
                .collect::<Result<Vec<_>, _>>()?;
            result.fees =
                super::tb03c_allocation::fees(integer(f, "amount")?, &original_units, ranks)?;
        }
        "bust" => {
            let from = integer(f, "from")?;
            let to = integer(f, "to")?;
            if from != frontier {
                return Err("MissingDependency：bust 不在当前累计前沿".into());
            }
            // #1406 独审 M2：末条数量记录不等于存续归属。重放原件与逆事件的区间栈，
            // 新 fill 全部撤回后，旧 fill 的残余区间重新成为尾段，仍按旧原价冲销。
            // #1683：b 跨多笔原 fill 时逐段取 A(段尾) − A(段首)，各段之和即 A(F) − A(F−b)。
            let mut surviving = surviving_ranges(image, &original_row["manifest_hash"])?;
            for segment in rewind(&mut surviving, from, to, &original["fact_key"])? {
                let inverse =
                    super::allocation::increments(segment.start, segment.end, weights, ranks)?;
                for i in 0..weights.len() {
                    result.units[i] -= inverse[i];
                    result.cash[i] = inverse[i]
                        .checked_mul(segment.price)
                        .and_then(|c| result.cash[i].checked_add(c))
                        .ok_or("InvalidDomain：逆现金溢出")?;
                }
            }
            if result.units.iter().sum::<i64>() != to - from {
                return Err("InvalidDomain：bust 逐重反冲合计不等于 b".into());
            }
            result.from = from;
            result.to = to;
            result.component = "acquisition_cash";
            result.reverse = true;
        }
        "reverse_component" => {
            // 原子佣金分量只能完整反冲一次；换事实身份不能重复退款。
            if rows.iter().any(|r| {
                r["original_fact_key"] == original["fact_key"]
                    && r["report_kind"] == "reverse_component"
                    && r["component"] == "commission"
            }) {
                return Err("IdentityConflict：原 commission 分量累计反冲将超过原值".into());
            }
            result.fees = allocations
                .iter()
                .map(|a| integer(a, "commission"))
                .collect::<Result<Vec<_>, _>>()?;
            if result.fees.iter().all(|f| *f == 0) {
                return Err("InvalidDomain：原 fill 无非零 commission 可反冲".into());
            }
            result.component = "commission";
            result.reverse = true;
        }
        _ => return Err("SchemaUnsupported：未知更正".into()),
    }
    Ok(result)
}
