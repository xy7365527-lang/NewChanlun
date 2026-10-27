//! #1768 TB-07-E：B/E 持久事件的 `domain-event/1` 外壳投影与 snapshot/delta 读口（ADR 0051 裁定一/四）。
//!
//! 只读投影：不新增表、不新增 wire op、不改既有字段名，也不改 `fact_key` 构成（book.rs:92-99）。
//! 外壳从 commits 根链、图像 history、`book.phase.events` 与永久回执逐条导出；
//! 状态载体是根链加 transition，外壳本身从不内联前后完整图像。
use super::book::hash;
use super::store::{
    append, audit, check_epoch, cut, err, immediate, latest, put, root_with_image_hash, row, Config,
};
use crate::session_protocol::{nonnegative, positive, required, strict_json};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// 外壳合同名；改形状即新版本（ADR 0041:17）。
pub(super) const CONTRACT: &str = "domain-event/1";
/// ADR 0051 裁定四的三个可调字段：事前单选，运行中只走一个值，改值即新版本。
pub(super) const STATE_CARRIER: &str = "root_chain+transition";
pub(super) const CONTENT_HASH_ALGO: &str = "jcs-sha256";
pub(super) const DELTA_RETENTION: &str = "retain_all";

/// 合同记录：外壳名与三个可调字段的冻结值。
pub(super) fn contract_record() -> Value {
    json!({"contract":CONTRACT,"state_carrier":STATE_CARRIER,"content_hash_algo":CONTENT_HASH_ALGO,"delta_retention":DELTA_RETENTION})
}

/// 裁定一外壳的全部必填键；可空值只有 structure_cut / structure_generation / model_ref / transition。
pub(super) const FIELDS: [&str; 17] = [
    "contract",
    "domain_id",
    "event_id",
    "writer_epoch",
    "cut",
    "structure_cut",
    "structure_generation",
    "prev_root",
    "root",
    "identity",
    "source",
    "policy_refs",
    "model_ref",
    "transition",
    "cause",
    "kind",
    "payload",
];
const SOURCE_FIELDS: [&str; 5] = [
    "source_namespace",
    "source_epoch",
    "external_fact_id",
    "source_cursor",
    "received_time",
];

fn exact(v: &Value, keys: &[&str], what: &str) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("InvalidDomain：{what}须对象"))?;
    if let Some(k) = keys.iter().find(|k| !o.contains_key(**k)) {
        return Err(format!("InvalidDomain：{what}缺必填字段{k}"));
    }
    if o.len() != keys.len() {
        return Err(format!("InvalidDomain：{what}含未登记字段"));
    }
    Ok(())
}
fn text_or_null(v: &Value, k: &str, what: &str) -> Result<(), String> {
    if v[k].is_null() || v[k].as_str().is_some_and(|s| !s.is_empty()) {
        return Ok(());
    }
    Err(format!("InvalidDomain：{what}.{k}须非空文本或null"))
}

/// 外壳形状与内部一致性：event_id、cut、root 必须指向同一次提交；
/// 写者纪元与来源纪元分列，身份只取 fact_key 与 command_id 之一。
pub(super) fn validate(v: &Value) -> Result<(), String> {
    exact(v, &FIELDS, "外壳")?;
    if v["contract"] != CONTRACT {
        return Err("SchemaUnsupported：外壳合同不是domain-event/1".into());
    }
    let domain = required(v, "domain_id")?;
    let id = &v["event_id"];
    exact(id, &["domain_id", "commit_seq", "ordinal"], "event_id")?;
    positive(required(id, "commit_seq")?, "event_id.commit_seq")?;
    nonnegative(required(id, "ordinal")?, "event_id.ordinal")?;
    positive(required(v, "writer_epoch")?, "writer_epoch")?;
    let c = &v["cut"];
    exact(c, &["domain_id", "commit_seq", "root_hash"], "cut")?;
    let root = required(v, "root")?;
    required(v, "prev_root")?;
    if id["domain_id"] != domain
        || c["domain_id"] != domain
        || c["commit_seq"] != id["commit_seq"]
        || c["root_hash"] != root
    {
        return Err("IdentityConflict：event_id/cut/root 不属于同一提交".into());
    }
    if !v["structure_cut"].is_null() && !v["structure_cut"].is_object() {
        return Err("InvalidDomain：structure_cut须对象或null".into());
    }
    if !v["structure_generation"].is_null() {
        nonnegative(required(v, "structure_generation")?, "structure_generation")?;
    }
    let identity = &v["identity"];
    let key = if identity.get("fact_key").is_some() {
        "fact_key"
    } else {
        "command_id"
    };
    exact(identity, &["chong_id", "voice", key], "identity")?;
    required(identity, key)?;
    if !identity["chong_id"].is_null() && !identity["chong_id"].is_array() {
        return Err("InvalidDomain：identity.chong_id须(S,ℓ)或null".into());
    }
    let source = &v["source"];
    exact(source, &SOURCE_FIELDS, "source")?;
    required(source, "source_namespace")?;
    required(source, "source_epoch")?;
    for k in ["external_fact_id", "source_cursor", "received_time"] {
        text_or_null(source, k, "source")?;
    }
    for p in v["policy_refs"]
        .as_array()
        .ok_or("InvalidDomain：policy_refs须数组")?
    {
        exact(
            p,
            &["policy_id", "version", "profile_id", "content_hash"],
            "policy_ref",
        )?;
        for k in ["policy_id", "version", "content_hash"] {
            required(p, k)?;
        }
        text_or_null(p, "profile_id", "policy_ref")?;
    }
    if !v["model_ref"].is_null() {
        exact(
            &v["model_ref"],
            &["model_id", "revision", "content_hash"],
            "model_ref",
        )?;
        for k in ["model_id", "revision", "content_hash"] {
            required(&v["model_ref"], k)?;
        }
    }
    if !v["transition"].is_null() {
        exact(&v["transition"], &["field", "from", "to"], "transition")?;
        required(&v["transition"], "field")?;
    }
    for c in v["cause"].as_array().ok_or("InvalidDomain：cause须数组")? {
        let k = if c.get("event_id").is_some() {
            "event_id"
        } else {
            "fact_key"
        };
        exact(c, &[k], "cause")?;
    }
    required(v, "kind")?;
    if v["payload"].is_null() {
        return Err("InvalidDomain：外壳缺payload".into());
    }
    Ok(())
}

pub(super) fn event_id(domain: &str, seq: i64, ordinal: usize) -> Value {
    json!({"domain_id":domain,"commit_seq":seq.to_string(),"ordinal":ordinal.to_string()})
}
/// 去重键（裁定二）：只取身份，不取内容。
pub(super) fn identity_key(v: &Value) -> Option<&str> {
    v["identity"]["fact_key"]
        .as_str()
        .or_else(|| v["identity"]["command_id"].as_str())
}

/// 裁定二的判定结果：同 ID 同内容为 no-op，同 ID 异内容为 IdentityConflict。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Admission {
    New,
    Noop,
    Conflict,
}
/// 只看每次提交的首条事件（ordinal 0）；派生阶段事件沿用同一身份，不另作去重键。
pub(super) fn admit(prior: &[Value], candidate: &Value) -> Admission {
    let Some(key) = identity_key(candidate) else {
        return Admission::Conflict;
    };
    match prior
        .iter()
        .find(|e| e["event_id"]["ordinal"] == "0" && identity_key(e) == Some(key))
    {
        None => Admission::New,
        Some(e)
            if e["kind"] == candidate["kind"]
                && hash(&e["payload"]) == hash(&candidate["payload"]) =>
        {
            Admission::Noop
        }
        Some(_) => Admission::Conflict,
    }
}

fn chong_id(config: &Config) -> Result<Value, String> {
    Ok(match config.domain.strip_prefix("B:") {
        Some(id) => {
            let r = config.route(id)?;
            json!([r["symbol"], r["operation_level"]])
        }
        None => Value::Null,
    })
}
fn commit(c: &Connection, seq: i64) -> Result<(String, String, String, Value), String> {
    let r: Option<(String, String, String, String)> = c
        .query_row(
            "SELECT prev_root,root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(err)?;
    let (prev, root, ns, image) = r.ok_or("MissingDependency：指定历史cut不存在")?;
    Ok((prev, root, ns, strict_json(image.as_bytes())?))
}
/// 提交时刻的写者纪元：取 `at_ns` 不晚于提交时刻的最大纪元。
/// 恢复时刻与提交时刻相同则无法判定提交落在哪一代，如实报 Incomplete，不择一（#1768）。
fn writer_epoch_at(epochs: &[(i64, i64)], ns: &str) -> Result<i64, String> {
    let n = nonnegative(ns, "commit_ns")?;
    let mut found = None;
    for &(epoch, at) in epochs {
        if at < n || (epoch == 1 && at == n) {
            found = Some(epoch);
        } else if at == n {
            return Err(
                "Incomplete：恢复时刻与提交时刻相同，writer_epoch 归属不可判定（#1768）".into(),
            );
        }
    }
    found.ok_or_else(|| "StorageUnavailable：提交早于创世纪元".into())
}
fn epochs(c: &Connection) -> Result<Vec<(i64, i64)>, String> {
    let mut s = c
        .prepare("SELECT epoch,at_ns FROM epochs ORDER BY epoch")
        .map_err(err)?;
    let rows = s
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(err)?;
    let mut out = Vec::new();
    for r in rows {
        let (e, at) = r.map_err(err)?;
        out.push((e, nonnegative(&at, "epoch at_ns")?));
    }
    Ok(out)
}
/// 每个业务提交恰有一条永久回执；按回执 cut 的 commit_seq 归位。
fn receipts(c: &Connection) -> Result<BTreeMap<String, (String, Value)>, String> {
    let mut s = c
        .prepare("SELECT k,json FROM receipts ORDER BY k")
        .map_err(err)?;
    let rows = s
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(err)?;
    let mut out = BTreeMap::new();
    for r in rows {
        let (k, raw) = r.map_err(err)?;
        let v = strict_json(raw.as_bytes())?;
        let seq = required(&v["cut"], "commit_seq")?.to_owned();
        if out.insert(seq, (k, v)).is_some() {
            return Err("StorageUnavailable：同一提交持有多条永久回执".into());
        }
    }
    Ok(out)
}
fn policy_ref(policy: &Value, content_hash: &Value) -> Value {
    json!({"policy_id":policy["fill_allocation_policy_id"],"version":policy["policy_revision"],"profile_id":null,"content_hash":content_hash})
}
/// 外效分配政策的既有引用原样展开（ADR 0051 裁定一对应表）；不签任何值。
fn manifest_policy_refs(config: &Config) -> Vec<Value> {
    let Some(policy) = config.manifest.get("fill_allocation_policy") else {
        return Vec::new();
    };
    config.manifest["fill_manifests"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| policy_ref(policy, &m["manifest_hash"]))
                .collect()
        })
        .unwrap_or_default()
}

struct Context {
    chong_id: Value,
    epochs: Vec<(i64, i64)>,
    receipts: BTreeMap<String, (String, Value)>,
}
impl Context {
    fn load(c: &Connection, config: &Config) -> Result<Self, String> {
        Ok(Self {
            chong_id: chong_id(config)?,
            epochs: epochs(c)?,
            receipts: receipts(c)?,
        })
    }
}

/// 单次提交的全部外壳：ordinal 0 是该提交追加的 history 条目，
/// ordinal 1.. 是同一提交追加的 `book.phase.events`（tb03c_book.rs:585-596）。
pub(super) fn events_at(c: &Connection, config: &Config, seq: i64) -> Result<Vec<Value>, String> {
    events_in(c, config, &Context::load(c, config)?, seq)
}
/// 全部业务提交的外壳，按 (commit_seq, ordinal) 升序。
pub(super) fn events(c: &Connection, config: &Config) -> Result<Vec<Value>, String> {
    let context = Context::load(c, config)?;
    let mut out = Vec::new();
    for seq in 1..=latest(c)?.0 {
        out.extend(events_in(c, config, &context, seq)?);
    }
    Ok(out)
}

fn events_in(
    c: &Connection,
    config: &Config,
    context: &Context,
    seq: i64,
) -> Result<Vec<Value>, String> {
    if seq < 1 {
        return Err("InvalidDomain：创世提交不承载域事件".into());
    }
    let (prev_root, root, ns, image) = commit(c, seq)?;
    let (_, previous_root, _, previous) = commit(c, seq - 1)?;
    if prev_root != previous_root {
        return Err("StorageUnavailable：域损坏：提交链不连续".into());
    }
    let history = image["history"]
        .as_array()
        .ok_or("StorageUnavailable：域损坏：history形状")?;
    let before = previous["history"]
        .as_array()
        .ok_or("StorageUnavailable：域损坏：history形状")?;
    if history.len() != before.len() + 1 || history[..before.len()] != before[..] {
        return Err("StorageUnavailable：域损坏：业务提交未恰追加一条history".into());
    }
    let event = history.last().unwrap();
    if event["kind"] == WITHDRAW_OP {
        return withdrawal_event(c, config, context, seq, &prev_root, &root, &ns, event);
    }
    let (receipt_key, receipt) = context
        .receipts
        .get(&seq.to_string())
        .ok_or("StorageUnavailable：域损坏：业务提交缺永久回执")?;
    let writer_epoch = writer_epoch_at(&context.epochs, &ns)?.to_string();
    let fact = match receipt["fact_key"].as_str() {
        Some(k) => Some((
            k.to_owned(),
            super::store::row(c, "facts", k)?.ok_or("StorageUnavailable：域损坏：回执缺原件")?,
        )),
        None => None,
    };
    let voice = if event["kind"] == "BindVoice" {
        event["record"]["identity"].clone()
    } else {
        Value::Null
    };
    let identity = match &fact {
        Some((k, _)) => json!({"chong_id":context.chong_id,"voice":voice,"fact_key":k}),
        None => json!({"chong_id":context.chong_id,"voice":voice,"command_id":receipt_key}),
    };
    let source = match &fact {
        Some((_, f)) => {
            let s = &f["event"]["source"];
            json!({"source_namespace":s["source_namespace"],"source_epoch":s["source_epoch"],"external_fact_id":s["external_fact_id"],"source_cursor":s["source_cursor"],"received_time":s["received_time"]})
        }
        None => {
            let o = if event.get("origin_message").is_some() {
                &event["origin_message"]
            } else {
                &event["request"]
            };
            json!({"source_namespace":o["source_namespace"],"source_epoch":o["source_epoch"],"external_fact_id":null,"source_cursor":null,"received_time":null})
        }
    };
    let policy_refs = match (&fact, config.manifest.get("fill_allocation_policy")) {
        (Some((_, f)), Some(policy)) if super::tb03c_allocation::is_projected(&f["event"]) => {
            vec![policy_ref(
                policy,
                &super::tb03c_allocation::report(&f["event"])["manifest_hash"],
            )]
        }
        _ => Vec::new(),
    };
    let cause = if event["kind"] == "ResolveEvidence" {
        event["covered"]
            .as_array()
            .map(|a| a.iter().map(|k| json!({"fact_key":k})).collect())
            .unwrap_or_default()
    } else {
        match &fact {
            // B 的应用以 E 已提交原事实为前件。
            Some((k, _)) if config.domain != "E" => vec![json!({"fact_key":k})],
            _ => Vec::new(),
        }
    };
    let transition = if event["kind"] == "BindVoice" {
        let (from, to) = (
            &previous["book"]["voice"]["status"],
            &image["book"]["voice"]["status"],
        );
        (from != to).then(|| json!({"field":"voice","from":from,"to":to}))
    } else {
        let (from, to) = (
            &previous["book"]["phase"]["latch"],
            &image["book"]["phase"]["latch"],
        );
        (from != to).then(|| json!({"field":"latch","from":from,"to":to}))
    };
    let envelope = |ordinal: usize,
                    kind: &Value,
                    payload: &Value,
                    transition: Option<Value>,
                    cause: Vec<Value>| {
        json!({"contract":CONTRACT,"domain_id":config.domain,"event_id":event_id(&config.domain,seq,ordinal),
            "writer_epoch":writer_epoch,"cut":cut(config,seq,&root),"structure_cut":null,"structure_generation":null,
            "prev_root":prev_root,"root":root,"identity":identity,"source":source,"policy_refs":policy_refs,
            "model_ref":null,"transition":transition,"cause":cause,"kind":kind,"payload":payload})
    };
    let first = envelope(0, &event["kind"], event, transition, cause);
    let mut out = vec![first];
    let phase = |image: &Value| {
        image["book"]["phase"]["events"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let (old, new) = (phase(&previous), phase(&image));
    if new.len() < old.len() || new[..old.len()] != old[..] {
        return Err("StorageUnavailable：域损坏：阶段事件史被改写".into());
    }
    for (i, e) in new[old.len()..].iter().enumerate() {
        if e["commit_seq"] != seq.to_string() {
            return Err("StorageUnavailable：域损坏：阶段事件不属于该提交".into());
        }
        let transition = (e["kind"] == "growth_eligible")
            .then(|| json!({"field":"growth_eligible","from":e["from"],"to":e["to"]}));
        out.push(envelope(
            i + 1,
            &e["kind"],
            e,
            transition,
            vec![json!({"event_id":event_id(&config.domain,seq,0)})],
        ));
    }
    for v in &out {
        validate(v)?;
    }
    Ok(out)
}

/// 撤回外壳（裁定三）：身份沿用原事件，不重铸 ID；前件指向被撤回事件。
/// 持久撤回由 [`withdraw`] 追加一次提交；外壳由 `events_in` 从该提交的 history 条目经本函数导出。
pub(super) fn withdrawn(
    original: &Value,
    at: &Value,
    withdrawn_generation: &str,
    withdrawal_reason: &str,
    superseded_by: Value,
) -> Result<Value, String> {
    validate(original)?;
    exact(
        at,
        &["cut", "prev_root", "writer_epoch", "ordinal"],
        "撤回位置",
    )?;
    let c = &at["cut"];
    let seq = positive(required(c, "commit_seq")?, "撤回commit_seq")?;
    let ordinal = nonnegative(required(at, "ordinal")?, "撤回ordinal")? as usize;
    let v = json!({"contract":CONTRACT,"domain_id":original["domain_id"],"event_id":event_id(required(original,"domain_id")?,seq,ordinal),
        "writer_epoch":at["writer_epoch"],"cut":c,"structure_cut":original["structure_cut"],"structure_generation":original["structure_generation"],
        "prev_root":at["prev_root"],"root":c["root_hash"],"identity":original["identity"],"source":original["source"],
        "policy_refs":original["policy_refs"],"model_ref":original["model_ref"],
        "transition":{"field":"withdrawn","from":null,"to":withdrawn_generation},
        "cause":[{"event_id":original["event_id"]}],"kind":"withdrawn",
        "payload":{"withdrawn_generation":withdrawn_generation,"withdrawal_reason":withdrawal_reason,"superseded_by":superseded_by}});
    validate(&v)?;
    Ok(v)
}
/// 撤回只追加：旧事件逐字保留，新事件不得复用已有 event_id，前件须在日志内。
pub(super) fn append_withdrawal(log: &[Value], w: &Value) -> Result<Vec<Value>, String> {
    validate(w)?;
    if w["kind"] != "withdrawn" {
        return Err("InvalidDomain：非撤回事件".into());
    }
    if log.iter().any(|e| e["event_id"] == w["event_id"]) {
        return Err("IdentityConflict：撤回复用已有event_id".into());
    }
    let target = &w["cause"][0]["event_id"];
    let original = log
        .iter()
        .find(|e| e["event_id"] == *target)
        .ok_or("MissingDependency：撤回前件不在日志内")?;
    if original["identity"] != w["identity"] {
        return Err("IdentityConflict：撤回改写了原事件身份".into());
    }
    let mut out = log.to_vec();
    out.push(w.clone());
    Ok(out)
}

/// 持久撤回的 wire op 与永久回执类型（ADR 0051 裁定三/五）：撤回是一次追加提交，
/// history 追加一条撤回条目，旧提交、旧事件、旧物化行一律不动；权限照例须 manifest 显式授予。
pub(super) const WITHDRAW_OP: &str = "WithdrawDomainEvent";
pub(super) const WITHDRAW_RECEIPT: &str = "DomainEventWithdrawn";
const WITHDRAW_FIELDS: [&str; 5] = [
    "op",
    "target",
    "withdrawn_generation",
    "withdrawal_reason",
    "superseded_by",
];
const WITHDRAW_ENTRY_FIELDS: [&str; 8] = [
    "kind",
    "target",
    "withdrawn_generation",
    "withdrawal_reason",
    "superseded_by",
    "origin_message",
    "commit_seq",
    "commit_ns",
];

/// 撤回的去重键是被撤回事件的身份（裁定二），不是内容：同一事件只能被撤回一次。
pub(super) fn withdrawal_receipt_key(target: &Value) -> String {
    format!("domain-event-withdrawn:{}", hash(target))
}
pub(super) fn withdrawal_receipt(config: &Config, entry: &Value, cut: Value) -> Value {
    json!({"kind":WITHDRAW_RECEIPT,"domain_id":config.domain,"target":entry["target"],
        "withdrawn_generation":entry["withdrawn_generation"],"withdrawal_reason":entry["withdrawal_reason"],
        "superseded_by":entry["superseded_by"],"commit_ns":entry["commit_ns"],"cut":cut})
}
/// 请求、history 条目与回执三处共有的撤回内容。
fn withdrawal_content(v: &Value) -> Value {
    json!([
        v["target"],
        v["withdrawn_generation"],
        v["withdrawal_reason"],
        v["superseded_by"]
    ])
}
fn event_ref(config: &Config, v: &Value, what: &str) -> Result<(i64, usize), String> {
    exact(v, &["domain_id", "commit_seq", "ordinal"], what)?;
    if v["domain_id"] != config.domain {
        return Err(format!("IdentityConflict：{what}跨域"));
    }
    let seq = positive(required(v, "commit_seq")?, what)?;
    let ordinal = nonnegative(required(v, "ordinal")?, what)? as usize;
    if *v != event_id(&config.domain, seq, ordinal) {
        return Err(format!("InvalidDomain：{what}非规范event_id"));
    }
    Ok((seq, ordinal))
}
fn withdrawal_request(config: &Config, p: &Value) -> Result<(), String> {
    exact(p, &WITHDRAW_FIELDS, "撤回请求")?;
    event_ref(config, &p["target"], "撤回target")?;
    required(p, "withdrawn_generation")?;
    required(p, "withdrawal_reason")?;
    if !p["superseded_by"].is_null() {
        event_ref(config, &p["superseded_by"], "superseded_by")?;
    }
    Ok(())
}
/// 在 `seq` 之前的图像里定位一条已存在的域事件，返回其所在提交的 history 条目。
/// ordinal 0 是该提交的 history 条目；ordinal k≥1 是该提交追加的第 k 条阶段事件。
fn existing_event<'a>(
    config: &Config,
    previous: &'a Value,
    v: &Value,
    seq: i64,
    what: &str,
) -> Result<&'a Value, String> {
    let (t_seq, ordinal) = event_ref(config, v, what)?;
    if t_seq >= seq {
        return Err(format!("MissingDependency：{what}不在本提交之前"));
    }
    let history = previous["history"]
        .as_array()
        .ok_or("StorageUnavailable：域损坏：history形状")?;
    let entry = history
        .get((t_seq - 1) as usize)
        .ok_or_else(|| format!("MissingDependency：{what}不存在"))?;
    if ordinal > 0 {
        let n = previous["book"]["phase"]["events"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|e| e["commit_seq"] == t_seq.to_string())
                    .count()
            })
            .unwrap_or(0);
        if ordinal > n {
            return Err(format!("MissingDependency：{what}不存在"));
        }
    }
    Ok(entry)
}
/// 撤回提交的唯一判据（#804）：写路径与审计重放共用。返回追加撤回条目后的完整图像。
pub(super) fn withdrawal_image(
    config: &Config,
    previous: &Value,
    entry: &Value,
) -> Result<Value, String> {
    exact(entry, &WITHDRAW_ENTRY_FIELDS, "撤回条目")?;
    if entry["kind"] != WITHDRAW_OP {
        return Err("InvalidDomain：非撤回条目".into());
    }
    let seq = positive(required(entry, "commit_seq")?, "撤回commit_seq")?;
    nonnegative(required(entry, "commit_ns")?, "撤回commit_ns")?;
    let history = previous["history"]
        .as_array()
        .ok_or("StorageUnavailable：域损坏：history形状")?;
    if history.len() as i64 != seq - 1 {
        return Err("InvalidDomain：撤回条目不是紧接链尾的提交".into());
    }
    let target = existing_event(config, previous, &entry["target"], seq, "撤回target")?;
    let (t_seq, t_ordinal) = event_ref(config, &entry["target"], "撤回target")?;
    if target["kind"] == WITHDRAW_OP && t_ordinal == 0 {
        return Err(format!("InvalidDomain：撤回事件不可再撤回（seq {t_seq}）"));
    }
    if history
        .iter()
        .any(|h| h["kind"] == WITHDRAW_OP && h["target"] == entry["target"])
    {
        return Err("IdentityConflict：该事件已被撤回".into());
    }
    if !entry["superseded_by"].is_null() {
        existing_event(
            config,
            previous,
            &entry["superseded_by"],
            seq,
            "superseded_by",
        )?;
        if entry["superseded_by"] == entry["target"] {
            return Err("InvalidDomain：superseded_by不得指向被撤回事件本身".into());
        }
    }
    let mut next = previous.clone();
    next["history"].as_array_mut().unwrap().push(entry.clone());
    Ok(next)
}

/// 持久撤回（裁定三）：经既有写纪律（纪元、审计、消息绑定、回执）追加一次提交。
/// 同一 target 同内容重投为 no-op 返回原回执；同 target 异内容为 IdentityConflict。
pub(super) fn withdraw(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
) -> Result<Value, String> {
    let p = &e["payload"];
    withdrawal_request(config, p)?;
    let key = withdrawal_receipt_key(&p["target"]);
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    if let Some(r) = super::service::previous_message(&tx, e)? {
        return Ok(r);
    }
    if let Some(r) = row(&tx, "receipts", &key)? {
        if r["kind"] != WITHDRAW_RECEIPT || withdrawal_content(&r) != withdrawal_content(p) {
            return Err("IdentityConflict：该事件已以不同内容撤回".into());
        }
        super::service::bind_message(&tx, e, &key)?;
        tx.commit().map_err(err)?;
        return Ok(r);
    }
    let (last, previous_root, last_ns, image) = latest(&tx)?;
    let seq = last
        .checked_add(1)
        .ok_or("InvalidDomain：commit seq 溢出")?;
    let recovered_ns: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last_ns, "last commit")?
        .max(nonnegative(&recovered_ns, "epoch at_ns")?)
        .to_string();
    let (_, _, ns) = super::service::clocks(config, e, &floor, epoch)?;
    let entry = json!({"kind":WITHDRAW_OP,"target":p["target"],"withdrawn_generation":p["withdrawn_generation"],
        "withdrawal_reason":p["withdrawal_reason"],"superseded_by":p["superseded_by"],
        "origin_message":{"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]},
        "commit_seq":seq.to_string(),"commit_ns":ns});
    let next = withdrawal_image(config, &image, &entry)?;
    let cut = append(&tx, config, seq, &previous_root, &ns, &next)?;
    let receipt = withdrawal_receipt(config, &entry, cut);
    put(&tx, "receipts", &key, &receipt)?;
    super::service::bind_message(&tx, e, &key)?;
    tx.commit().map_err(err)?;
    Ok(receipt)
}

/// 撤回提交的外壳：身份、来源、政策沿用被撤回事件（经 [`withdrawn`]），位置取本提交。
#[allow(clippy::too_many_arguments)]
fn withdrawal_event(
    c: &Connection,
    config: &Config,
    context: &Context,
    seq: i64,
    prev_root: &str,
    root: &str,
    ns: &str,
    entry: &Value,
) -> Result<Vec<Value>, String> {
    let (t_seq, t_ordinal) = event_ref(config, &entry["target"], "撤回target")?;
    let original = events_in(c, config, context, t_seq)?
        .into_iter()
        .nth(t_ordinal)
        .ok_or("StorageUnavailable：域损坏：撤回target不存在")?;
    let at = json!({"cut":cut(config,seq,root),"prev_root":prev_root,
        "writer_epoch":writer_epoch_at(&context.epochs, ns)?.to_string(),"ordinal":"0"});
    Ok(vec![withdrawn(
        &original,
        &at,
        required(entry, "withdrawn_generation")?,
        required(entry, "withdrawal_reason")?,
        entry["superseded_by"].clone(),
    )?])
}
/// 根绑定图像里某提交 ordinal 0 应有的 (kind, payload)：撤回条目投影为 withdrawn。
fn first_expected(entry: &Value) -> (Value, Value) {
    if entry["kind"] == WITHDRAW_OP {
        (
            json!("withdrawn"),
            json!({"withdrawn_generation":entry["withdrawn_generation"],"withdrawal_reason":entry["withdrawal_reason"],"superseded_by":entry["superseded_by"]}),
        )
    } else {
        (entry["kind"].clone(), entry.clone())
    }
}

fn rule_revision(c: &Connection, config: &Config) -> Result<Value, String> {
    let book: Option<String> = c
        .query_row("SELECT v FROM meta WHERE k='book_revision'", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(err)?;
    Ok(json!({"schema_revision":config.revision()?,"book_revision":book}))
}
fn snapshot_value(
    config: &Config,
    writer_epoch: &str,
    rules: &Value,
    cut: Value,
    ns: &str,
    image: Value,
) -> Result<Value, String> {
    Ok(
        json!({"kind":"DomainSnapshot","contract":contract_record(),"domain_id":config.domain,"writer_epoch":writer_epoch,
        "frontier":{"commit_seq":cut["commit_seq"],"commit_ns":ns},"cut":cut,"structure_cut":null,"rule_revision":rules,
        "scope":{"domain_id":config.domain,"chong_id":chong_id(config)?},"policy_refs":manifest_policy_refs(config),
        "model_ref":null,"image_hash":hash(&image),"image":image}),
    )
}

/// snapshot 读口（裁定四.1）：先核写者纪元，再按 service 既有 cut/AsKnown 选择（二者互斥）。
/// 与 ReadView 同一读纪律：同一读事务内先全域审计（逐提交重算
/// root(seq, prev_root, commit_ns, image) 并重放），任一不合即 StorageUnavailable，不返回图像。
pub(super) fn snapshot(
    c: &Connection,
    config: &Config,
    writer_epoch: i64,
    query: &Value,
) -> Result<Value, String> {
    let tx = c.unchecked_transaction().map_err(err)?;
    check_epoch(&tx, writer_epoch)?;
    audit(&tx, config)?;
    let (seq, root, ns, image) = super::service::select_cut(&tx, config, query)?;
    snapshot_value(
        config,
        &writer_epoch.to_string(),
        &rule_revision(c, config)?,
        cut(config, seq, &root),
        &ns,
        image,
    )
}

fn gap(config: &Config, base: &Value, rebuild: &Value, reason: &str) -> Value {
    json!({"kind":"Gap","contract":CONTRACT,"domain_id":config.domain,"base_cut":base,"rebuild_cut":rebuild,"reason":reason})
}
/// 保留窗下界：`retain_all` 即全史可读；别的保留值是新版本，不在运行中切换。
fn retention_floor() -> i64 {
    match DELTA_RETENTION {
        "retain_all" => 0,
        _ => unreachable!("delta_retention 只登记 retain_all"),
    }
}
/// delta 读口（裁定四.2/.4）：旧 writer_epoch 报 StaleWriter；落后保留窗显式 Gap 并给重建 cut。
pub(super) fn delta(
    c: &Connection,
    config: &Config,
    writer_epoch: i64,
    base_cut: &Value,
) -> Result<Value, String> {
    delta_within(c, config, writer_epoch, base_cut, retention_floor())
}
pub(super) fn delta_within(
    c: &Connection,
    config: &Config,
    writer_epoch: i64,
    base_cut: &Value,
    floor: i64,
) -> Result<Value, String> {
    let tx = c.unchecked_transaction().map_err(err)?;
    let c: &Connection = &tx;
    check_epoch(c, writer_epoch)?;
    audit(c, config)?;
    let (last, last_root, last_ns, image) = latest(c)?;
    let rebuild = cut(config, last, &last_root);
    if base_cut["domain_id"] != config.domain {
        return Err("IdentityConflict：跨域cut".into());
    }
    if nonnegative(required(base_cut, "commit_seq")?, "commit_seq")? < floor {
        return Ok(gap(config, base_cut, &rebuild, "消费者游标落后于保留窗"));
    }
    let (base, _, _, _) = super::service::select_cut(c, config, &json!({ "cut": base_cut }))?;
    let context = Context::load(c, config)?;
    let mut links = Vec::new();
    let mut events = Vec::new();
    for seq in base + 1..=last {
        let (prev_root, root, ns, image) = commit(c, seq)?;
        links.push(json!({"cut":cut(config,seq,&root),"prev_root":prev_root,"commit_ns":ns,"image_hash":hash(&image)}));
        events.extend(events_in(c, config, &context, seq)?);
    }
    Ok(
        json!({"kind":"DomainDelta","contract":contract_record(),"domain_id":config.domain,"writer_epoch":writer_epoch.to_string(),
        "base_cut":base_cut,"next_cut":rebuild,"seq_range":{"from":(base+1).to_string(),"to":last.to_string()},
        "batch_id":hash(&json!([base_cut,rebuild])),"links":links,"events":events,"commit_ns":last_ns,
        "image_hash":hash(&image),"image":image}),
    )
}

/// 续接（裁定四.3）：snapshot(base) 加 delta 得 next 快照；链、根、纪元、批次任一不合即 Gap。
pub(super) fn resume(config: &Config, base: &Value, delta: &Value) -> Value {
    if delta["kind"] == "Gap" {
        return delta.clone();
    }
    resume_checked(config, base, delta)
        .unwrap_or_else(|reason| gap(config, &base["cut"], &delta["next_cut"], &reason))
}
fn resume_checked(config: &Config, base: &Value, delta: &Value) -> Result<Value, String> {
    if base["kind"] != "DomainSnapshot" || delta["kind"] != "DomainDelta" {
        return Err("续接只接受 DomainSnapshot + DomainDelta".into());
    }
    if base["contract"] != contract_record() || delta["contract"] != contract_record() {
        return Err("合同记录不一致".into());
    }
    if base["domain_id"] != config.domain || delta["domain_id"] != config.domain {
        return Err("跨域续接".into());
    }
    if base["writer_epoch"] != delta["writer_epoch"] {
        return Err("writer_epoch 不一致".into());
    }
    if base["cut"] != delta["base_cut"] || base["image_hash"] != hash(&base["image"]) {
        return Err("delta 基点不是该 snapshot".into());
    }
    let base_seq = nonnegative(required(&base["cut"], "commit_seq")?, "commit_seq")?;
    let mut seq = base_seq;
    let mut prev = required(&base["cut"], "root_hash")?.to_owned();
    let mut ns = required(&base["frontier"], "commit_ns")?.to_owned();
    let mut image_hash = base["image_hash"].clone();
    let mut links_by_seq = Vec::new();
    for link in delta["links"].as_array().ok_or("links形状")? {
        exact(
            link,
            &["cut", "prev_root", "commit_ns", "image_hash"],
            "link",
        )?;
        seq += 1;
        let link_ns = required(link, "commit_ns")?;
        let link_hash = required(link, "image_hash")?;
        let root = root_with_image_hash(config, seq, &prev, link_ns, link_hash);
        if link["prev_root"] != prev
            || link["cut"] != cut(config, seq, &root)
            || nonnegative(link_ns, "commit_ns")? < nonnegative(&ns, "commit_ns")?
        {
            return Err(format!("提交链在 seq {seq} 断开"));
        }
        links_by_seq.push((seq, link.clone(), prev.clone()));
        prev = root;
        ns = link_ns.to_owned();
        image_hash = json!(link_hash);
    }
    let next = cut(config, seq, &prev);
    if delta["next_cut"] != next
        || delta["seq_range"] != json!({"from":(base_seq+1).to_string(),"to":seq.to_string()})
        || delta["batch_id"] != hash(&json!([base["cut"], next]))
        || delta["commit_ns"] != ns
    {
        return Err("delta 终点/区间/批次 id 与链不一致".into());
    }
    if delta["image_hash"] != image_hash || hash(&delta["image"]) != image_hash {
        return Err("delta 图像不等于链终点根所承诺的图像".into());
    }
    check_events(config, delta, &links_by_seq, seq)?;
    let mut out = base.clone();
    out["cut"] = next.clone();
    out["frontier"] = json!({"commit_seq":next["commit_seq"],"commit_ns":ns});
    out["image_hash"] = image_hash;
    out["image"] = delta["image"].clone();
    Ok(out)
}

/// 事件集合逐位核对（裁定四.3）：每个链接提交应有的事件由链尾根绑定图像推出——
/// ordinal 0 是该提交的 history 条目，ordinal 1.. 是该提交追加的阶段事件。
/// delta.events 必须恰好是这组事件，按 (commit_seq, ordinal) 升序、无缺项、无重复、无多余，
/// 且每条的 kind/payload 与图像一致、cut/prev_root/root 与对应链接一致。
fn check_events(
    config: &Config,
    delta: &Value,
    links: &[(i64, Value, String)],
    last: i64,
) -> Result<(), String> {
    let history = delta["image"]["history"]
        .as_array()
        .ok_or("delta 图像 history 形状")?;
    let phase = delta["image"]["book"]["phase"]["events"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if history.len() as i64 != last {
        return Err("delta 图像 history 长度与链终点不一致".into());
    }
    let mut expected = Vec::new();
    for (seq, link, prev_root) in links {
        let entry = &history[(*seq - 1) as usize];
        let (kind, payload) = first_expected(entry);
        expected.push((
            event_id(&config.domain, *seq, 0),
            kind,
            payload,
            link,
            prev_root,
        ));
        let own = phase.iter().filter(|e| e["commit_seq"] == seq.to_string());
        for (i, e) in own.enumerate() {
            expected.push((
                event_id(&config.domain, *seq, i + 1),
                e["kind"].clone(),
                e.clone(),
                link,
                prev_root,
            ));
        }
    }
    let events = delta["events"].as_array().ok_or("events形状")?;
    if events.len() != expected.len() {
        return Err(format!(
            "delta 事件数 {} 不等于链上应有的 {}",
            events.len(),
            expected.len()
        ));
    }
    for (e, (id, kind, payload, link, prev_root)) in events.iter().zip(&expected) {
        validate(e)?;
        if e["event_id"] != *id
            || e["kind"] != *kind
            || e["payload"] != *payload
            || e["cut"] != link["cut"]
            || e["prev_root"] != **prev_root
            || e["root"] != link["cut"]["root_hash"]
        {
            return Err(format!("delta 事件 {id} 与链上应有事件不一致"));
        }
    }
    Ok(())
}

/// 逐字段比对：返回取值不同（含一侧缺失）的顶层字段名。
pub(super) fn fieldwise_diff(a: &Value, b: &Value) -> Vec<String> {
    let (Some(x), Some(y)) = (a.as_object(), b.as_object()) else {
        return vec!["<非对象>".into()];
    };
    let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| x.get(*k) != y.get(*k))
        .cloned()
        .collect()
}
