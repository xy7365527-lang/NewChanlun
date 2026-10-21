//! #1406 TB-03-C C3：E 侧 Pending 条目、ResolveEvidence 与 B 应用前的显式等待（只对 tb03c-book/1 域）。
//!
//! - Capture（INTERFACE :35；C06.4）：Recorded 与 Pending 同事务；原件声明的前件尚未 Recorded ⟹
//!   `AwaitingDependency`；目标重不在 manifest ⟹ 照记、Pending 范围封为 `account:*`，不路由任何 B。
//! - ResolveEvidence（INTERFACE :36；C06.5）：全有或全无；只移除证据覆盖的 token；判据复用 #1394
//!   `finance::covered_pending`（同一判断不另写一份，#799），另核来源前沿与前件；不读任何 B（C06.3）。
//! - B 应用（根裁定 OQ-3）：E 侧 `AwaitingDependency`/`AwaitingPolicy` 挡 B（typed 等待、不建收据）；
//!   普通风险 Pending 不挡 B。
//! 旧域（无 book_revision）Pending 文案与操作集合逐字不变。
use super::book::{fact_key, hash, target, Rules};
use super::store::Config;
use crate::session_protocol::{nonnegative, required};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) const RESOLVE_RECEIPT_PREFIX: &str = "resolve:";

pub(super) fn receipt_key(id: &str) -> String {
    format!("{RESOLVE_RECEIPT_PREFIX}{}", hash(&json!(id)))
}

pub(super) fn dependency_key(event: &Value) -> Result<Option<String>, String> {
    let Some(d) = event.get("depends_on") else {
        return Ok(None);
    };
    Ok(Some(hash(&json!([
        required(d, "source_namespace")?,
        required(d, "source_epoch")?,
        required(d, "external_fact_id")?
    ]))))
}

/// #1406 H1：费用必须绑定原 fill 的来源身份和原 allocation，不能让任意 Applied 原件充当前件。
/// 只读 E 已接纳或 B 已应用的原件；字段来自带来源索引的费用分量，不采请求中的派生归属。
pub(super) fn check_fill_predecessor(event: &Value, records: &[Value]) -> Result<(), String> {
    if super::tb03c_allocation::is_projected(event) {
        super::tb03c_corrections::predecessor(event, records)?;
        return Ok(()); // 原成交与更正均由同一具名前件核；累计分配在 Resolve 核。
    }
    for component in super::book::array(event, "known_cost_components")? {
        if component["kind"] != "fee_only" || component["cause"] != "fill" {
            continue;
        }
        let reference = component
            .get("fill_ref")
            .ok_or("MissingDependency：fill 费用缺具名绑定")?;
        let key = dependency_key(event)?.ok_or("MissingDependency：fill 费用缺前件")?;
        let predecessor = &records
            .iter()
            .find(|r| r["fact_key"] == key)
            .ok_or("AwaitingDependency：具名 fill 尚未持久接纳/应用")?["event"];
        if predecessor.get("observed_existing_book").is_some()
            || predecessor.get("corporate_action").is_some()
            || predecessor.get("valuation_observation").is_some()
            || super::book::integer(&predecessor["deltas"], "units")? == 0
        {
            return Err("IdentityConflict：费用前件不是实际数量变动 fill".into());
        }
        if ["source_namespace", "source_epoch", "external_fact_id"]
            .iter()
            .any(|k| reference[k] != predecessor["source"][k])
            || reference["allocation_id"] != predecessor["allocation_id"]
            || reference["family_id"] != predecessor["family_id"]
            || reference["chong_id"] != target(predecessor)?
        {
            return Err("IdentityConflict：费用具名 fill 与原持久分配不符".into());
        }
    }
    Ok(())
}

/// Capture 与审计重放共用的 Pending 条目（`previous` = 本次提交前的图像）。
pub(super) fn pending_entry(
    config: &Config,
    previous: &Value,
    record: &Value,
    seq: i64,
) -> Result<Value, String> {
    let key = &record["fact_key"];
    if config.book_rules()? == Rules::Legacy {
        return Ok(
            json!({"fact_key":key,"token":record["token"],"target_domain":record["target_domain"],"status":"Pending","reason":"No Resolve/Reduce evidence in TB-03-A","first_known_ns":record["first_known_ns"],"recorded_seq":seq.to_string()}),
        );
    }
    let event = &record["event"];
    let recorded = previous["recorded"]
        .as_array()
        .ok_or("StorageUnavailable：recorded 形状")?;
    super::tb03c_transfer::check_capture(config, event)?;
    super::tb03c_transfer::check_claim(recorded, record)?;
    let transfer_wait = super::tb03c_transfer::waiting_on(recorded, event)?;
    let netted = super::tb03c_allocation::is_projected(event);
    // #1406 C8：`/2` 的签署缺口（q_rebase、追加投入）先于其余等待；`/1` 返回 None、原文案不变。
    let funding_wait = super::tb03c_funding::pending_reason(&config.manifest, event)?;
    let reason = if let Some(wait) = funding_wait {
        wait
    } else if event.get("q_rebase").is_some() {
        json!({"kind":"AwaitingPolicy","slot":"capacity"})
    } else if let Some(on) = transfer_wait {
        json!({"kind":"AwaitingDependency","on":on})
    } else if netted && !super::tb03c_allocation::has_policy(config) {
        json!({"kind":"AwaitingPolicy","slot":"fill_allocation_policy"})
    } else {
        match dependency_key(event)? {
            Some(dep) if !recorded.iter().any(|f| f["fact_key"] == dep.as_str()) => {
                json!({"kind":"AwaitingDependency","on":dep})
            }
            _ => json!({"kind":"Unresolved"}),
        }
    };
    let scope = if netted || config.route(target(event)?).is_ok() {
        record["target_domain"].clone()
    } else {
        json!("account:*")
    };
    Ok(
        json!({"fact_key":key,"token":record["token"],"target_domain":record["target_domain"],"status":"Pending","reason":reason,"scope":scope,"first_known_ns":record["first_known_ns"],"recorded_seq":seq.to_string()}),
    )
}

/// 证据请求的业务内容（去掉 op），回执与重放都按它比较同内容。
pub(super) fn evidence_body(payload: &Value) -> Value {
    let mut body = payload.clone();
    body.as_object_mut().map(|m| m.remove("op"));
    body
}

/// 纯函数：校验证据并返回移除被覆盖 Pending 后的图像与被覆盖的 fact_key（按证据顺序）。
pub(super) fn resolve_image(
    config: &Config,
    previous: &Value,
    payload: &Value,
) -> Result<(Value, Vec<String>), String> {
    super::voice::exact(
        payload,
        &[
            "op",
            "evidence_id",
            "covered",
            "frontier",
            "dependency_version",
        ],
    )?;
    required(payload, "evidence_id")?;
    required(payload, "dependency_version")?;
    let frontier = &payload["frontier"];
    super::voice::exact(frontier, &["source_namespace", "source_epoch", "cursor"])?;
    let cursor = nonnegative(required(frontier, "cursor")?, "frontier.cursor")?;
    let covered = payload["covered"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("InvalidDomain：证据须覆盖至少一个 Pending")?;
    let mut keys = BTreeSet::new();
    let mut order = Vec::new();
    for c in covered {
        super::voice::exact(c, &["fact_key", "token", "fact_hash"])?;
        let k = required(c, "fact_key")?;
        if !keys.insert(k.to_owned()) {
            return Err("IdentityConflict：证据重复覆盖同一事实".into());
        }
        order.push(k.to_owned());
    }
    let pending = previous["pending"]
        .as_array()
        .ok_or("StorageUnavailable：pending 形状")?;
    let recorded = previous["recorded"]
        .as_array()
        .ok_or("StorageUnavailable：recorded 形状")?;
    for k in &order {
        let fact = recorded
            .iter()
            .find(|f| f["fact_key"] == k.as_str())
            .ok_or("MissingDependency：Resolve 缺持久原事实")?;
        // #1406 H2：风险 Pending 可被 Reserve 按完整责任覆盖清除；净额待办独立由
        // 已 Recorded 且未分配定义，不能因清风险永久丢失。非净额仍须原 Pending。
        let reconstructed;
        let p = if super::tb03c_allocation::is_projected(&fact["event"]) {
            if previous
                .get("fill_allocations")
                .map(|v| v.as_array().ok_or("StorageUnavailable：分配记录形状"))
                .transpose()?
                .is_some_and(|rows| rows.iter().any(|r| r["fact_key"] == k.as_str()))
            {
                return Err("IdentityConflict：净额事实已经持久分配".into());
            }
            reconstructed = pending_entry(
                config,
                previous,
                fact,
                crate::session_protocol::positive(required(fact, "recorded_seq")?, "recorded_seq")?,
            )?;
            &reconstructed
        } else {
            pending
                .iter()
                .find(|p| p["fact_key"] == k.as_str())
                .ok_or("MissingDependency：证据覆盖的非净额事实无现存 Pending")?
        };
        if p["reason"]["kind"] == "AwaitingPolicy" && p["reason"]["slot"] == "capacity" {
            return Err("AwaitingPolicy：capacity；Resolve 证据不能替代用户签署的容量".into());
        }
        // #1406 C8（`/2`）：重基与追加投入只能由新签署版本解除，Resolve 证据不能替代。
        if p["reason"]["kind"] == "AwaitingPolicy"
            && (p["reason"]["slot"] == "q_rebase" || p["reason"]["slot"] == "funding_revision")
        {
            return Err(format!(
                "AwaitingPolicy：{}；Resolve 证据不能替代用户签署的新版本",
                p["reason"]["slot"].as_str().unwrap_or("")
            ));
        }
        super::finance::covered_pending(previous, covered, p)?;
        // C06.2/C06.3：FIN 域只认 E 已持久的完整责任界。请求自报的摘要只能绑定事实，不能替代保责。
        if let Some(finance) = previous.get("finance") {
            let responsibilities = finance["responsibilities"]
                .as_array()
                .ok_or("StorageUnavailable：完整责任集合缺失")?;
            let retained: Vec<Value> = responsibilities
                .iter()
                .filter_map(|r| r["source"]["covered_facts"].as_array())
                .flat_map(|a| a.iter().cloned())
                .collect();
            super::finance::covered_pending(previous, &retained, p)?;
        }
        let fact = recorded
            .iter()
            .find(|f| f["fact_key"] == k.as_str())
            .ok_or("StorageUnavailable：Pending 缺原事实")?;
        let s = &fact["event"]["source"];
        if frontier["source_namespace"] != s["source_namespace"]
            || frontier["source_epoch"] != s["source_epoch"]
            || cursor < nonnegative(required(s, "source_cursor")?, "source_cursor")?
        {
            return Err("MissingDependency：证据来源前沿未到达该事实".into());
        }
    }
    let mut next = previous.clone();
    for k in &order {
        let fact = recorded
            .iter()
            .find(|f| f["fact_key"] == k.as_str())
            .ok_or("StorageUnavailable：Pending 缺原件")?;
        super::tb03c_allocation::resolve(
            config,
            &mut next,
            fact,
            required(payload, "evidence_id")?,
        )?;
    }
    next["pending"] = json!(pending
        .iter()
        .filter(|p| !p["fact_key"].as_str().is_some_and(|k| keys.contains(k)))
        .cloned()
        .collect::<Vec<_>>());
    Ok((next, order))
}

pub(super) fn resolve_history(request: &Value, covered: &[String], seq: i64, ns: &str) -> Value {
    let payload = &request["payload"];
    json!({"kind":"ResolveEvidence","evidence_id":payload["evidence_id"],"evidence":evidence_body(payload),"covered":covered,"commit_seq":seq.to_string(),"commit_ns":ns,
        "origin_message":{"source_namespace":request["source_namespace"],"source_epoch":request["source_epoch"],"message_id":request["message_id"]},"origin_request_hash":hash(request)})
}

pub(super) fn resolve_receipt(config: &Config, history: &Value, cut: Value) -> Value {
    json!({"kind":"Resolved","domain_id":config.domain,"evidence_id":history["evidence_id"],"evidence_hash":hash(&history["evidence"]),"covered":history["covered"],"commit_ns":history["commit_ns"],"cut":cut})
}

pub(super) fn event_for<'a>(image: &'a Value, id: &str) -> Result<&'a Value, String> {
    image["history"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|e| e["kind"] == "ResolveEvidence" && e["evidence_id"] == id)
        })
        .ok_or_else(|| "StorageUnavailable：Resolve 原提交事件缺失".into())
}

pub(super) fn replay(
    c: &rusqlite::Connection,
    config: &Config,
    previous: &Value,
    event: &Value,
    seq: i64,
    ns: &str,
    budget: &mut super::store::ReadBudget<'_>,
) -> Result<Value, String> {
    use crate::session_protocol::{strict_json, validate_envelope};
    if config.domain != "E" || !config.book_rules()?.is_tb03c() {
        return Err("StorageUnavailable：Resolve 不属于本域/书账版本".into());
    }
    let origin = &event["origin_message"];
    let key = hash(&json!([
        required(origin, "source_namespace")?,
        required(origin, "source_epoch")?,
        required(origin, "message_id")?
    ]));
    let (raw, bound): (String, String) = budget.one(
        c,
        "SELECT request_json,receipt_key FROM messages WHERE k=?",
        [key],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let request = validate_envelope(strict_json(raw.as_bytes())?, config.revision()?)?;
    config.authorized(&request)?;
    let payload = &request["payload"];
    if payload["op"] != "ResolveEvidence"
        || bound != receipt_key(required(payload, "evidence_id")?)
        || event["origin_request_hash"] != hash(&request)
    {
        return Err("StorageUnavailable：Resolve 原消息绑定改变".into());
    }
    let (mut next, covered) = resolve_image(config, previous, payload)?;
    let expected = resolve_history(&request, &covered, seq, ns);
    if event != &expected {
        return Err("StorageUnavailable：Resolve 事件与原消息/提交不符".into());
    }
    next["history"].as_array_mut().unwrap().push(expected);
    Ok(next)
}

pub(super) fn audit_receipt(
    c: &rusqlite::Connection,
    config: &Config,
    image: &Value,
    receipt: &Value,
    budget: &mut super::store::ReadBudget<'_>,
) -> Result<(), String> {
    use crate::session_protocol::{positive, strict_json};
    let event = event_for(image, required(receipt, "evidence_id")?)?;
    let seq = positive(required(event, "commit_seq")?, "Resolve commit seq")?;
    let (root, ns, raw): (String, String, String) = budget.one(
        c,
        "SELECT root,commit_ns,image FROM commits WHERE seq=?",
        [seq],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let actual = strict_json(raw.as_bytes())?;
    if actual["history"].as_array().and_then(|a| a.last()) != Some(event)
        || event["commit_ns"] != ns
        || *receipt != resolve_receipt(config, event, super::store::cut(config, seq, &root))
    {
        return Err("StorageUnavailable：Resolve 回执不等于实际提交".into());
    }
    Ok(())
}

/// B 应用前在 E 的同一只读快照里读该事实的 Pending：显式等待挡 B。
pub(super) fn check_wait(e_tx: &rusqlite::Connection, key: &str) -> Result<(), String> {
    if let Some(p) = super::store::row(e_tx, "pending", key)? {
        if p["reason"]["kind"] == "AwaitingPolicy" && p["reason"]["slot"] == "capacity" {
            return Err("AwaitingPolicy：capacity；B 不应用、不建收据，fixed_q 不变".into());
        }
        match p["reason"]["kind"].as_str() {
            Some(kind @ ("AwaitingDependency" | "AwaitingPolicy")) => {
                return Err(format!(
                    "{kind}：E 侧显式等待，B 不应用、不建收据（根裁定 OQ-3）"
                ));
            }
            _ => (),
        }
    }
    Ok(())
}

/// 原件解码时对 `depends_on` 的形状与来源引用校验。
pub(super) fn check_depends_on(
    event: &Value,
    pointer: &str,
    check_ref: &dyn Fn(&str, &str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(d) = event.get("depends_on") {
        super::voice::exact(
            d,
            &[
                "source_namespace",
                "source_epoch",
                "external_fact_id",
                "source_ref",
            ],
        )?;
        if dependency_key(event)? == Some(fact_key(event)?) {
            return Err("InvalidDomain：原件不能依赖自身".into());
        }
        check_ref(
            required(d, "source_ref")?,
            &format!("{pointer}/depends_on"),
            d,
        )?;
    }
    Ok(())
}
