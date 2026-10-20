//! #1406 C4a/C4b / ADR 0032：冻结投影清单 → E 持久分配证明 → B 逐重消费。
//! 原件、派生分配和提交证明各自保存；金额与数量只调用 C1 同一配额函数。
use super::book::{array, hash, integer};
use super::store::Config;
use crate::session_protocol::{nonnegative, positive, required};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn is_projected(event: &Value) -> bool {
    event.get("netted_fill").is_some() || event.get("per_leg_report").is_some()
}
pub(super) fn report(event: &Value) -> &Value {
    event.get("netted_fill").unwrap_or(&event["per_leg_report"])
}
pub(super) fn target_domain(event: &Value) -> Result<String, String> {
    if is_projected(event) {
        Ok(format!(
            "E:{}:{}",
            if event.get("per_leg_report").is_some() {
                "per_leg"
            } else {
                "netted"
            },
            required(report(event), "manifest_hash")?
        ))
    } else {
        Ok(format!("B:{}", super::book::target(event)?))
    }
}
pub(super) fn application_id(record: &Value) -> Result<&str, String> {
    required(
        record
            .get("resolved_allocation")
            .unwrap_or(&record["event"]),
        "allocation_id",
    )
}
pub(super) fn application_domain(record: &Value) -> Result<String, String> {
    match record.get("resolved_allocation") {
        Some(a) => Ok(format!("B:{}", required(a, "chong_id")?)),
        None => Ok(required(record, "target_domain")?.to_owned()),
    }
}

pub(super) fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("fill_manifests").is_none() && config.get("fill_allocation_policy").is_none() {
        return Ok(());
    }
    if super::book::rules(config)? != super::book::Rules::Tb03cV1 {
        return Err("SchemaUnsupported：净额分配须 tb03c-book/1".into());
    }
    if let Some(policy) = config.get("fill_allocation_policy") {
        super::voice::exact(policy, &["fill_allocation_policy_id", "policy_revision"])?;
        if policy["fill_allocation_policy_id"] != super::allocation::FILL_ALLOCATION_POLICY_ID
            && policy["fill_allocation_policy_id"] != super::tb03c_per_leg::POLICY
        {
            return Err("SchemaUnsupported：净额政策仅 /2；per_leg 仅来源归属 /1".into());
        }
        required(policy, "policy_revision")?;
    }
    let mut hashes = BTreeSet::new();
    for manifest in array(config, "fill_manifests")? {
        super::voice::exact(
            manifest,
            &[
                "projection_kind",
                "symbol",
                "participants",
                "tie_break_order",
                "manifest_hash",
            ],
        )?;
        let expected_policy = match required(manifest, "projection_kind")? {
            "netted" => super::allocation::FILL_ALLOCATION_POLICY_ID,
            "per_leg" => super::tb03c_per_leg::POLICY,
            _ => return Err("SchemaUnsupported：未知冻结投影类型".into()),
        };
        if let Some(p) = config.get("fill_allocation_policy") {
            if p["fill_allocation_policy_id"] != expected_policy {
                return Err("IdentityConflict：冻结投影类型与外效政策不符".into());
            }
        }
        let mut body = manifest.clone();
        body.as_object_mut().unwrap().remove("manifest_hash");
        if manifest["manifest_hash"] != hash(&body)
            || !hashes.insert(required(manifest, "manifest_hash")?)
        {
            return Err("IdentityConflict：投影清单 hash 不符或重复".into());
        }
        let symbol = required(manifest, "symbol")?;
        let mut ids = BTreeSet::new();
        for participant in array(manifest, "participants")? {
            super::voice::exact(
                participant,
                &[
                    "chong_id",
                    "command_id",
                    "family_id",
                    "revision",
                    "requested_q",
                ],
            )?;
            let id = required(participant, "chong_id")?;
            required(participant, "command_id")?;
            required(participant, "revision")?;
            if !ids.insert(id) {
                return Err("IdentityConflict：投影参与重重复".into());
            }
            required(participant, "family_id")?;
            positive(required(participant, "requested_q")?, "requested_q")?;
            let route = array(config, "chongs")?
                .iter()
                .find(|r| r["chong_id"] == id)
                .ok_or("MissingDependency：冻结参与重无路由")?;
            if route["symbol"] != symbol {
                return Err("IdentityConflict：净额参与重必须同一冻结标的".into());
            }
        }
        let expected: Vec<Value> = ids.iter().map(|id| json!(id)).collect();
        if expected.is_empty() || array(manifest, "tie_break_order")? != &expected {
            return Err("IdentityConflict：tie_break_order 必须是冻结参与重字典序".into());
        }
    }
    Ok(())
}
fn manifest<'a>(config: &'a Config, event: &Value) -> Result<&'a Value, String> {
    let key = required(report(event), "manifest_hash")?;
    array(&config.manifest, "fill_manifests")?
        .iter()
        .find(|m| m["manifest_hash"] == key)
        .ok_or_else(|| "MissingDependency：原件投影清单未冻结".into())
}
pub(super) fn has_policy(config: &Config) -> bool {
    config.manifest.get("fill_allocation_policy").is_some()
}
pub(super) fn check_capture(config: &Config, event: &Value) -> Result<(), String> {
    if is_projected(event) {
        let m = manifest(config, event)?;
        let projection = if event.get("per_leg_report").is_some() {
            "per_leg"
        } else {
            "netted"
        };
        if m["projection_kind"] != projection {
            return Err("IdentityConflict：原件类型与冻结投影不符，不得回落".into());
        }
        if report(event)["symbol"] != m["symbol"] {
            return Err("IdentityConflict：净额原件标的与冻结投影不符".into());
        }
    }
    Ok(())
}

pub(super) fn decode(
    event: &Value,
    pointer: &str,
    check_ref: &dyn Fn(&str, &str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    if event.get("per_leg_report").is_some() {
        return super::tb03c_per_leg::decode(event, pointer, check_ref);
    }
    let correction = super::tb03c_corrections::is_correction(event);
    let mut event_fields = vec![
        "event",
        "allocation_id",
        "source",
        "stable_point",
        "netted_fill",
    ];
    if correction {
        event_fields.push("depends_on");
    }
    super::voice::exact(event, &event_fields)?;
    let fill = &event["netted_fill"];
    if correction {
        super::tb03c_corrections::decode(event, pointer, check_ref)?;
    } else {
        super::voice::exact(
            fill,
            &[
                "manifest_hash",
                "symbol",
                "report_kind",
                "reported_quantity",
                "from",
                "to",
                "unit_price",
                "commission",
                "cash_kind",
                "income_changes",
                "source_ref",
            ],
        )?;
        required(fill, "manifest_hash")?;
        required(fill, "symbol")?;
        let from = nonnegative(required(fill, "from")?, "from")?;
        let to = positive(required(fill, "to")?, "to")?;
        if from >= to {
            return Err("InvalidDomain：C4a 只接受正向非零成交区间".into());
        }
        let reported = positive(required(fill, "reported_quantity")?, "reported_quantity")?;
        match required(fill, "report_kind")? {
            "incremental" if reported == to - from => (),
            "cumulative" if reported == to => (),
            _ => return Err("IdentityConflict：逐笔/累计回报与累计区间不符".into()),
        }
        nonnegative(required(fill, "unit_price")?, "unit_price")?;
        nonnegative(required(fill, "commission")?, "commission")?;
        if fill["cash_kind"] != "acquisition_cash" {
            return Err("SchemaUnsupported：C4a 净额原件仅支持具名多头购入现金".into());
        }
    }
    super::voice::exact(&fill["income_changes"], &["Pi", "A", "W"])?;
    for k in ["Pi", "A", "W"] {
        if integer(&fill["income_changes"], k)? != 0 {
            return Err("SchemaUnsupported：净额成交不得隐含收益账变化".into());
        }
    }
    check_ref(
        required(fill, "source_ref")?,
        &format!("{pointer}/netted_fill"),
        fill,
    )?;
    let sp = &event["stable_point"];
    super::voice::exact(sp, &["profile", "seq", "source_ref"])?;
    if sp["profile"] != "TestOnly" {
        return Err("SchemaUnsupported：稳定点仅 TestOnly".into());
    }
    nonnegative(required(sp, "seq")?, "stable_point.seq")?;
    check_ref(
        required(sp, "source_ref")?,
        &format!("{pointer}/stable_point"),
        sp,
    )
}

pub(super) fn fees(total: i64, delta: &[i64], ranks: &[usize]) -> Result<Vec<i64>, String> {
    let mut result = vec![0; delta.len()];
    if total == 0 {
        return Ok(result);
    }
    let active: Vec<usize> = (0..delta.len()).filter(|&i| delta[i] > 0).collect();
    let weights = active
        .iter()
        .map(|&i| {
            delta[i]
                .checked_mul(total)
                .ok_or_else(|| "InvalidDomain：费用比例缩放溢出".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let compact_ranks = active
        .iter()
        .map(|&i| active.iter().filter(|&&j| ranks[j] < ranks[i]).count())
        .collect::<Vec<_>>();
    let apportioned = super::allocation::apportion(total, &weights, &compact_ranks)?;
    for (&i, amount) in active.iter().zip(apportioned.alloc) {
        result[i] = amount;
    }
    Ok(result)
}

pub(super) fn resolve(
    config: &Config,
    next: &mut Value,
    fact: &Value,
    evidence_id: &str,
) -> Result<(), String> {
    let event = &fact["event"];
    if !is_projected(event) {
        return Ok(());
    }
    if !has_policy(config) {
        return Err("AwaitingPolicy：外效分配政策未冻结".into());
    }
    check_capture(config, event)?;
    let m = manifest(config, event)?;
    if m["projection_kind"] == "per_leg" {
        return super::tb03c_per_leg::resolve(config, next, fact, m, evidence_id);
    }
    let fill = &event["netted_fill"];
    let correction = super::tb03c_corrections::is_correction(event);
    let prior = next.get("fill_allocations").and_then(Value::as_array);
    let last = prior.and_then(|rows| {
        rows.iter()
            .rev()
            .find(|r| r["manifest_hash"] == m["manifest_hash"])
    });
    let frontier = last.map(|r| integer(r, "to")).transpose()?.unwrap_or(0);
    let from = if correction {
        frontier
    } else {
        integer(fill, "from")?
    };
    let to = if correction {
        frontier
    } else {
        integer(fill, "to")?
    };
    if from != frontier {
        return Err("MissingDependency：净额累计前沿缺口、倒退或重复冲突".into());
    }
    let participants = array(m, "participants")?;
    let weights = participants
        .iter()
        .map(|p| integer(p, "requested_q"))
        .collect::<Result<Vec<_>, _>>()?;
    let order = array(m, "tie_break_order")?;
    let ranks = participants
        .iter()
        .map(|p| {
            order
                .iter()
                .position(|id| id == &p["chong_id"])
                .ok_or_else(|| "IdentityConflict：参与重不在并列顺序".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let effects = if correction {
        Some(super::tb03c_corrections::resolve(
            event, next, frontier, &weights, &ranks,
        )?)
    } else {
        None
    };
    let delta = match &effects {
        Some(e) => e.units.clone(),
        None => super::allocation::increments(from, to, &weights, &ranks)?,
    };
    let fee = match &effects {
        Some(e) => e.fees.clone(),
        None => fees(integer(fill, "commission")?, &delta, &ranks)?,
    };
    let mut allocations = Vec::new();
    for (i, p) in participants.iter().enumerate() {
        let cash = match &effects {
            Some(e) => e.cash[i],
            None => delta[i]
                .checked_mul(integer(fill, "unit_price")?)
                .ok_or("InvalidDomain：成交现金溢出")?,
        };
        let id = format!(
            "netted:{}",
            hash(&json!([
                fact["fact_key"],
                event["allocation_id"],
                m["manifest_hash"],
                p
            ]))
        );
        allocations.push(json!({"allocation_id":id,"chong_id":p["chong_id"],"command_id":p["command_id"],"family_id":p["family_id"],"revision":p["revision"],"units":delta[i].to_string(),"acquisition_cash":cash.to_string(),"commission":fee[i].to_string()}));
    }
    if let Some(e) = &effects {
        for a in &mut allocations {
            a["component"] = json!(e.component);
            a["reversal"] = json!(e.reverse);
            a["original_fact_key"] = e.original_fact_key.clone();
            if e.component == "fee_only" {
                let amount = a.as_object_mut().unwrap().remove("commission").unwrap();
                a["fee_only"] = amount;
            }
        }
    }
    if next.get("fill_allocations").is_none() {
        next["fill_allocations"] = json!([]);
    }
    next["fill_allocations"].as_array_mut().unwrap().push(json!({"fact_key":fact["fact_key"],"fact_hash":fact["fact_hash"],"manifest_hash":m["manifest_hash"],"symbol":m["symbol"],"fill_allocation_policy_id":config.manifest["fill_allocation_policy"]["fill_allocation_policy_id"],"policy_revision":config.manifest["fill_allocation_policy"]["policy_revision"],"from":fill["from"],"to":fill["to"],"evidence_id":evidence_id,"allocations":allocations}));
    if let Some(e) = effects {
        let row = next["fill_allocations"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        row["from"] = json!(e.from.to_string());
        row["to"] = json!(e.to.to_string());
        row["report_kind"] = fill["report_kind"].clone();
        row["component"] = json!(e.component);
        row["original_fact_key"] = e.original_fact_key;
    }
    Ok(())
}

/// 固定 E 已审计快照内取实际 Resolve 提交；B 保存两份证明，恢复无需当前 E。
pub(super) fn attach_resolution(
    c: &rusqlite::Connection,
    config: &Config,
    record: &mut Value,
) -> Result<(), String> {
    if !is_projected(&record["event"]) {
        return Ok(());
    }
    let image = super::store::latest(c)?.3;
    let row = image["fill_allocations"]
        .as_array()
        .ok_or("AwaitingDependency：净额事实尚无 E 持久分配")?
        .iter()
        .find(|r| r["fact_key"] == record["fact_key"])
        .ok_or("AwaitingDependency：净额事实尚无 E 持久分配")?;
    let id = config
        .domain
        .strip_prefix("B:")
        .ok_or("InvalidDomain：分配消费须为 B")?;
    let allocation = array(row, "allocations")?
        .iter()
        .find(|a| a["chong_id"] == id)
        .ok_or("IdentityConflict：本重不在冻结分配清单")?
        .clone();
    let event = super::tb03c_e::event_for(&image, required(row, "evidence_id")?)?;
    let seq = positive(required(event, "commit_seq")?, "Resolve seq")?;
    let (previous_root, ns, raw): (String, String, String) = c
        .query_row(
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(super::store::err)?;
    let (previous_previous_root, previous_ns, previous_raw): (String, String, String) = c
        .query_row(
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [seq - 1],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(super::store::err)?;
    let origin = &event["origin_message"];
    let key = hash(&json!([
        origin["source_namespace"],
        origin["source_epoch"],
        origin["message_id"]
    ]));
    let request: String = c
        .query_row("SELECT request_json FROM messages WHERE k=?", [key], |r| {
            r.get(0)
        })
        .map_err(super::store::err)?;
    let receipt = super::store::row(
        c,
        "receipts",
        &super::tb03c_e::receipt_key(required(row, "evidence_id")?),
    )?
    .ok_or("StorageUnavailable：Resolve 缺回执")?;
    record["resolved_allocation"] = allocation;
    record["e_resolution_proof"] = json!({"commit_seq":seq.to_string(),"previous_root":previous_root,"commit_ns":ns,"image":crate::session_protocol::strict_json(raw.as_bytes())?,"previous_previous_root":previous_previous_root,"previous_commit_ns":previous_ns,"previous_image":crate::session_protocol::strict_json(previous_raw.as_bytes())?,"request":crate::session_protocol::strict_json(request.as_bytes())?,"receipt":receipt});
    Ok(())
}

pub(super) fn verify_resolution(record: &Value, config: &Config) -> Result<(), String> {
    if !is_projected(&record["event"]) {
        if record.get("resolved_allocation").is_some() || record.get("e_resolution_proof").is_some()
        {
            return Err("StorageUnavailable：单重原件不得夹带净额派生证明".into());
        }
        return Ok(());
    }
    let proof = &record["e_resolution_proof"];
    let seq = positive(required(proof, "commit_seq")?, "Resolve seq")?;
    if seq <= positive(required(record, "recorded_seq")?, "recorded_seq")? {
        return Err("StorageUnavailable：Resolve 必须晚于 Capture".into());
    }
    let ns = required(proof, "commit_ns")?;
    let mut ec = config.clone();
    ec.domain = "E".into();
    let previous = &proof["previous_image"];
    let previous_root = super::store::root(
        &ec,
        seq - 1,
        required(proof, "previous_previous_root")?,
        required(proof, "previous_commit_ns")?,
        previous,
    );
    if proof["previous_root"] != previous_root {
        return Err("StorageUnavailable：Resolve 前图像证明不符".into());
    }
    let request =
        crate::session_protocol::validate_envelope(proof["request"].clone(), ec.revision()?)?;
    ec.authorized(&request)?;
    if request["payload"]["op"] != "ResolveEvidence"
        || request["payload"]["frontier"]["source_namespace"] != request["source_namespace"]
        || request["payload"]["frontier"]["source_epoch"] != request["source_epoch"]
    {
        return Err("StorageUnavailable：Resolve 原消息授权/操作不符".into());
    }
    let (mut next, covered) = super::tb03c_e::resolve_image(&ec, previous, &request["payload"])?;
    let history = super::tb03c_e::resolve_history(&request, &covered, seq, ns);
    next["history"]
        .as_array_mut()
        .unwrap()
        .push(history.clone());
    let cut = super::store::cut(
        &ec,
        seq,
        &super::store::root(&ec, seq, &previous_root, ns, &next),
    );
    if proof["image"] != next
        || proof["receipt"] != super::tb03c_e::resolve_receipt(&ec, &history, cut)
    {
        return Err("StorageUnavailable：Resolve 证明不能从原件与冻结政策重放".into());
    }
    let row = array(&next, "fill_allocations")?
        .iter()
        .find(|r| r["fact_key"] == record["fact_key"])
        .ok_or("StorageUnavailable：Resolve 证明缺原事实分配")?;
    let allocation = &record["resolved_allocation"];
    if row["fact_hash"] != record["fact_hash"]
        || !covered.iter().any(|k| record["fact_key"] == k.as_str())
        || !array(row, "allocations")?.contains(allocation)
        || application_domain(record)? != config.domain
    {
        return Err("StorageUnavailable：逐重分配不属于该 Resolve/原件/B".into());
    }
    let capture = record["e_commit_proof"]["image"]["recorded"]
        .as_array()
        .and_then(|a| a.last())
        .ok_or("StorageUnavailable：Capture 证明缺原件")?;
    if !array(previous, "recorded")?.contains(capture) {
        return Err("StorageUnavailable：Resolve 前图像不含对应 Capture 原件".into());
    }
    Ok(())
}
