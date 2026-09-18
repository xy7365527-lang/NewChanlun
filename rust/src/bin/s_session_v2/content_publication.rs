//! #1394 C §5.4：从当前事务的独立出生索引许可本批占位，再定点解析。
//!
//! 调用顺序：核旧域完整原件 → 构造模板/manifest → 在同一 Commit 事务暂写独立索引
//! （真实 batch3 已知）→ 本模块 → 持久实际根、Delta、clock 和前沿 → commit。
//! 本模块不提交事务；任何错误必须使调用者回滚。返回值只证明本批占位和发布元数据，
//! 不能替代对象身份、来源、全部历史、Delta 方程或资源预算的完整业务核验。
//! S3 的 Delta 按已批准 §5.4 固定为真实变化；不调用旧域 `delta_storage::read_mode`，
//! 不从旧私有格式表推断协议。未接线的 S3 writer 必须生成此合同，不能复用 legacy-full 集合。
#![allow(dead_code)]

use super::{canonical_json, content_manifest, content_store, meta_get_opt, sha256_hex, v2};
use content_manifest::{ManifestMetadata, ManifestRoots, UnpublishedManifest};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, replace_batch_placeholder, PathStep},
    ContentRef, Kind, NodeDigest, NodeSink, NodeStore, Value as V,
};
use newchan_rust::session_protocol::{parse_canonical_i64, strict_json, validate_envelope};
use rusqlite::{params, Transaction};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct ResolvedTemplates {
    pub snapshot: ContentRef,
    pub delta: ContentRef,
}

pub(super) fn resolve_current_births(
    tx: &Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &ManifestRoots,
    manifest: &UnpublishedManifest,
) -> Result<ResolvedTemplates, String> {
    verify_current_begin(tx, begin_token, metadata)?;
    let mut store = content_store::TxStore::new(tx).map_err(err)?;
    let computed =
        content_manifest::assemble_unpublished(&mut store, metadata, roots).map_err(err)?;
    if computed.batch_id != manifest.batch_id
        || computed.value.content_ref() != manifest.value.content_ref()
    {
        return Err("IdentityConflict：manifest 与当前三个根/上下文不符".into());
    }
    let previous_frontier =
        parse_canonical_i64(&meta(tx, "last_advance_frontier")?, "last_advance_frontier")?;
    resolve_indexed_templates(
        tx,
        &mut store,
        metadata,
        roots,
        &manifest.batch_id,
        previous_frontier,
    )
}

/// 发布和历史回读共用同一占位许可规则。历史调用方提供只读节点源及内存写层；
/// 独立索引按目标代投影，未来出生/撤回不得改变旧 cut。此口不授予新的 Commit 权限。
pub(super) fn resolve_indexed_templates(
    tx: &Transaction<'_>,
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &ManifestRoots,
    batch_id: &str,
    previous_frontier: i64,
) -> Result<ResolvedTemplates, String> {
    let objects = object_births(tx, metadata)?;
    let observations = observation_births(tx, metadata)?;
    let mut snapshot = roots.publication_template.clone();
    let mut delta = roots.delta_template.clone();
    match_field(
        store,
        &snapshot,
        "history_mode",
        text("RecomputedWithRevision"),
    )?;
    match_field(store, &snapshot, "as_of_generation", V::Null)?;
    let from = previous_frontier
        .checked_add(1)
        .ok_or_else(|| err("seq_range 起点溢出"))?;
    if previous_frontier < -1 || from > metadata.input_frontier {
        return Err(err("seq_range 无实际新增输入"));
    }
    for root in [&snapshot, &delta] {
        match_field(
            store,
            root,
            "seq_range",
            V::Object(vec![
                ("from".into(), text(from)),
                ("to".into(), text(metadata.input_frontier)),
            ]),
        )?;
    }
    resolve_objects(
        store,
        &mut snapshot,
        "objects",
        &objects,
        metadata,
        batch_id,
        ObjectFamily::Active,
    )?;
    resolve_objects(
        store,
        &mut snapshot,
        "withdrawn_objects",
        &objects,
        metadata,
        batch_id,
        ObjectFamily::Withdrawn,
    )?;
    resolve_objects(
        store,
        &mut delta,
        "upserts",
        &objects,
        metadata,
        batch_id,
        ObjectFamily::Born,
    )?;
    resolve_observations(
        store,
        &mut snapshot,
        &observations,
        metadata,
        batch_id,
        false,
    )?;
    resolve_observations(store, &mut delta, &observations, metadata, batch_id, true)?;
    for root in [&mut snapshot, &mut delta] {
        for path in [
            vec![field("index_frontier")],
            vec![field("catalog_evidence"), field("batch_id")],
        ] {
            *root = replace_batch_placeholder(store, root, &path, batch_id).map_err(err)?;
        }
        // 未列入 schema 的任意嵌套占位到这里仍存在，Public 核验必须拒绝。
        sc::verify(store, root).map_err(err)?;
    }
    Ok(ResolvedTemplates { snapshot, delta })
}

fn err(e: impl std::fmt::Display) -> String {
    format!("PublicationInvalid：{e}")
}
fn field(s: &str) -> PathStep {
    PathStep::Field(s.into())
}
fn meta(tx: &Transaction<'_>, key: &str) -> Result<String, String> {
    meta_get_opt(tx, key)?.ok_or_else(|| err(format!("缺 meta.{key}")))
}
struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
fn hash(v: V) -> Result<NodeDigest, String> {
    sc::write_value(&mut HashOnly, &v)
        .map(|v| v.digest)
        .map_err(err)
}
fn text(s: impl ToString) -> V {
    V::String(s.to_string())
}
fn match_field(
    store: &mut dyn NodeStore,
    root: &ContentRef,
    key: &str,
    v: V,
) -> Result<(), String> {
    let actual = lookup_path(store, root, &[field(key)])
        .map_err(err)?
        .digest()
        .map_err(err)?;
    if actual != hash(v)? {
        return Err(err(format!("{key} 与独立记录不同")));
    }
    Ok(())
}
fn json_value(v: &serde_json::Value) -> Result<V, String> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => V::Null,
        J::Bool(x) => V::Bool(*x),
        J::String(x) => V::String(x.clone()),
        J::Number(x) => V::Number(sc::NumberToken::parse(&x.to_string()).map_err(err)?),
        J::Array(xs) => V::Array(xs.iter().map(json_value).collect::<Result<_, _>>()?),
        J::Object(xs) => V::Object(
            xs.iter()
                .map(|(k, v)| Ok((k.clone(), json_value(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}

/// 冻结标量和当前 Begin 必须来自同一事务；不调用只认识 v2/旧 batch 的根审计器。
/// 旧可达历史与全部来源审计是调用者在本事务写入前的独立义务，本函数不为它们背书。
pub(super) fn verify_current_begin(
    tx: &Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
) -> Result<(), String> {
    for (key, expected) in [
        ("protocol_revision", "s-session/3"),
        ("session_id", m.session_id.as_str()),
        ("writer_epoch", m.writer_epoch.as_str()),
        ("structure_cut", m.base_cut.as_str()),
        ("profile_id", m.profile_id.as_str()),
        ("profile_hash", m.profile_hash.as_str()),
        ("catalog_revision", m.catalog_revision.as_str()),
        ("rule_revision", m.rule_revision.as_str()),
        ("begin_token", token),
    ] {
        if meta(tx, key)? != expected {
            return Err(err(format!("当前 {key} 不符")));
        }
    }
    if m.generation.checked_sub(1)
        != Some(parse_canonical_i64(&meta(tx, "generation")?, "generation")?)
        || m.structure_cut != format!("cut-{}", m.generation)
        || meta(tx, "advance_state")?
            != format!("begun:{token}:{}:{}", m.generation, m.input_frontier)
    {
        return Err("StaleWriter：当前 Begin 所有权/代号/cut 不符".into());
    }
    if hash(json_value(&strict_json(meta(tx, "scope")?.as_bytes())?)?)?
        != m.scope.digest().map_err(err)?
    {
        return Err(err("scope 与冻结域不同"));
    }
    let frontier: i64 = tx
        .query_row("SELECT COALESCE(MAX(seq),-1) FROM raw_events", [], |r| {
            r.get(0)
        })
        .map_err(err)?;
    if frontier != m.input_frontier {
        return Err("StaleWriter：输入前沿已变化".into());
    }
    let (protocol,inc,clock_hash,clock_raw,next_ordinal,phase_frontier):(String,String,String,String,i64,String)=tx.query_row(
        "SELECT protocol_revision,session_generation,clock_plan_hash,clock_plan_json,next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1",[],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(err)?;
    let clock = v2::ClockPlan::parse(strict_json(clock_raw.as_bytes())?)?;
    if protocol != "s-session/3"
        || inc != m.session_generation
        || clock_hash != m.clock_plan_hash
        || clock.hash != clock_hash
        || canonical_json(&clock.value) != clock_raw
    {
        return Err(err("协议/会话代号/clock 冻结原件不同"));
    }
    let s = &m.source_commit;
    let (ph,raw,accepted,status,first,cid,attempts,receipt):(String,String,i64,String,Option<i64>,String,i64,String)=tx.query_row(
        "SELECT payload_hash,canonical_envelope,accepted_seq,status,first_published_generation,clock_event_id,attempt_count,receipt_id FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",
        params![s.source_namespace,s.source_epoch,s.message_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).map_err(err)?;
    let accepted_receipt: String = tx
        .query_row(
            "SELECT receipt_id FROM raw_events WHERE seq=?1",
            params![accepted],
            |r| r.get(0),
        )
        .map_err(err)?;
    if receipt != accepted_receipt {
        return Err(err("message 的接纳收据与实际源行不同"));
    }
    let envelope = validate_envelope(strict_json(raw.as_bytes())?, "s-session/3")?;
    if canonical_json(&envelope) != raw
        || ph != s.payload_hash
        || cid != s.clock_event_id
        || accepted > frontier
        || accepted < 0
        || status != "AcceptedPending"
        || first.is_some()
        || attempts.checked_sub(1) != Some(s.attempt_index)
    {
        return Err(err("source_commit 不是当前已接纳未发布的 attempt"));
    }
    for (key, expected) in [
        ("session_id", &m.session_id),
        ("session_generation", &m.session_generation),
        ("source_namespace", &s.source_namespace),
        ("source_epoch", &s.source_epoch),
        ("message_id", &s.message_id),
        ("payload_hash", &s.payload_hash),
        ("producer_id", &s.producer_id),
        ("producer_epoch", &s.producer_epoch),
    ] {
        if envelope[key].as_str() != Some(expected.as_str()) {
            return Err(err(format!("封存 envelope.{key} 不符")));
        }
    }
    if envelope["payload"]["clock_event_id"].as_str() != Some(&s.clock_event_id) {
        return Err(err("envelope clock_event_id 不符"));
    }
    let (begin_ns,message_key,ordinal,generation):(String,String,i64,i64)=tx.query_row(
        "SELECT semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events WHERE clock_event_id=?1 AND phase='begin' AND attempt_index=?2",
        params![cid,s.attempt_index],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(err)?;
    let expected_key = canonical_json(&serde_json::json!([
        s.source_namespace,
        s.source_epoch,
        s.message_id
    ]));
    let expected_token = format!(
        "begin-{}",
        sha256_hex(
            canonical_json(&serde_json::json!([
                m.session_id,
                m.session_generation,
                m.writer_epoch,
                ordinal.to_string(),
                m.base_cut,
                m.generation.to_string(),
                m.input_frontier.to_string()
            ]))
            .as_bytes()
        )
    );
    let commit_ns = clock.phase(&cid, "commit", s.attempt_index)?;
    if ordinal < 1
        || ordinal.checked_add(1) != Some(next_ordinal)
        || generation != m.generation
        || message_key != expected_key
        || begin_ns != clock.phase(&cid, "begin", s.attempt_index)?
        || commit_ns != m.semantic_commit_ns
        || token != expected_token
        || phase_frontier != begin_ns
        || parse_canonical_i64(&phase_frontier, "clock frontier")?
            > parse_canonical_i64(&commit_ns, "commit clock")?
    {
        return Err(err("Begin/clock/attempt 与本次发布不同"));
    }
    Ok(())
}

struct ObjectBirth {
    revision: i64,
    kind: String,
    batch: String,
    first: i64,
    cut: String,
    published: i64,
    withdrawn: Option<i64>,
    reason: Option<String>,
    superseded: Option<String>,
}
fn object_births(
    tx: &Transaction<'_>,
    m: &ManifestMetadata,
) -> Result<BTreeMap<NodeDigest, ObjectBirth>, String> {
    let mut out = BTreeMap::new();
    let latest = super::meta_i64(tx, "generation")?.max(m.generation);
    {
        let sql="SELECT object_id,object_revision,kind,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects";
        let mut stmt = tx.prepare(&sql).map_err(err)?;
        let mut rows = stmt.query([]).map_err(err)?;
        while let Some(r) = rows.next().map_err(err)? {
            let id: String = r.get(0).map_err(err)?;
            let mut b = ObjectBirth {
                revision: r.get(1).map_err(err)?,
                kind: r.get(2).map_err(err)?,
                batch: r.get(3).map_err(err)?,
                first: r.get(4).map_err(err)?,
                cut: r.get(5).map_err(err)?,
                published: r.get(6).map_err(err)?,
                withdrawn: r.get(7).map_err(err)?,
                reason: r.get(8).map_err(err)?,
                superseded: r.get(9).map_err(err)?,
            };
            if b.first < 1
                || b.first > latest
                || b.published < b.first
                || b.published > latest
                || b.cut != format!("cut-{}", b.first)
                || b.withdrawn.is_some_and(|g| g <= b.first || g > latest)
            {
                return Err(err("独立对象出生/生命周期代号非法"));
            }
            if b.first > m.generation {
                continue;
            }
            if b.published > m.generation {
                return Err(err("对象出生后发布代号被改写"));
            }
            if b.withdrawn.is_some_and(|g| g > m.generation) {
                b.withdrawn = None;
                b.reason = None;
                b.superseded = None;
            }
            if out.insert(hash(text(id))?, b).is_some() {
                return Err(err("对象身份同时存在两个独立表"));
            }
        }
    }
    Ok(out)
}
struct ObservationBirth {
    kind: String,
    batch: String,
    first: i64,
}
fn observation_births(
    tx: &Transaction<'_>,
    m: &ManifestMetadata,
) -> Result<BTreeMap<NodeDigest, ObservationBirth>, String> {
    let mut out = BTreeMap::new();
    let latest = super::meta_i64(tx, "generation")?.max(m.generation);
    let mut stmt = tx
        .prepare(
            "SELECT observation_id,kind,batch_id,published_generation FROM s_content_observations",
        )
        .map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(r) = rows.next().map_err(err)? {
        let id: String = r.get(0).map_err(err)?;
        let b = ObservationBirth {
            kind: r.get(1).map_err(err)?,
            batch: r.get(2).map_err(err)?,
            first: r.get(3).map_err(err)?,
        };
        if b.first < 1 || b.first > latest {
            return Err(err("独立观察首次发布代号非法"));
        }
        if b.first > m.generation {
            continue;
        }
        out.insert(hash(text(id))?, b);
    }
    Ok(out)
}
#[derive(Clone, Copy)]
enum ObjectFamily {
    Active,
    Withdrawn,
    Born,
}
fn member(b: &ObjectBirth, m: &ManifestMetadata, f: ObjectFamily) -> bool {
    match f {
        ObjectFamily::Active => b.withdrawn.is_none(),
        ObjectFamily::Withdrawn => b.withdrawn.is_some(),
        ObjectFamily::Born => b.first == m.generation,
    }
}
fn array_len(store: &mut dyn NodeStore, root: &ContentRef, name: &str) -> Result<u64, String> {
    let r = lookup_path(store, root, &[field(name)]).map_err(err)?;
    if r.kind != Kind::Array {
        return Err(err(format!("{name} 不是数组")));
    }
    r.extent.ok_or_else(|| err("缺数组长度"))
}
fn resolve_record_batch(
    store: &mut dyn NodeStore,
    root: &mut ContentRef,
    path: &[PathStep],
    record: &ContentRef,
    first: i64,
    stored_batch: &str,
    m: &ManifestMetadata,
    batch: &str,
) -> Result<(), String> {
    let value = lookup_path(store, record, &[field("batch_id")]).map_err(err)?;
    if first == m.generation {
        if stored_batch != batch || value.kind != Kind::TemplateBatch {
            return Err(err("本批出生索引/占位不符"));
        }
        let mut p = path.to_vec();
        p.push(field("batch_id"));
        *root = replace_batch_placeholder(store, root, &p, batch).map_err(err)?;
    } else {
        if stored_batch == batch {
            return Err(err("旧出生记录不得换成本批身份"));
        }
        match_field(store, record, "batch_id", text(stored_batch))?;
    }
    Ok(())
}
fn resolve_objects(
    store: &mut dyn NodeStore,
    root: &mut ContentRef,
    name: &str,
    births: &BTreeMap<NodeDigest, ObjectBirth>,
    m: &ManifestMetadata,
    batch: &str,
    family: ObjectFamily,
) -> Result<(), String> {
    let expected: BTreeSet<_> = births
        .iter()
        .filter(|(_, b)| member(b, m, family))
        .map(|(id, _)| *id)
        .collect();
    let n = array_len(store, root, name)?;
    if n != expected.len() as u64 {
        return Err(err(format!("{name} 与独立索引成员数不符")));
    }
    let mut seen = BTreeSet::new();
    for i in 0..n {
        let path = [field(name), PathStep::Index(i)];
        let record = lookup_path(store, root, &path).map_err(err)?;
        let id = lookup_path(store, &record, &[field("object_id")])
            .map_err(err)?
            .digest()
            .map_err(err)?;
        if !expected.contains(&id) || !seen.insert(id) {
            return Err(err(format!("{name} 存在越界/重复身份")));
        }
        let b = &births[&id];
        for (k, v) in [
            ("kind", text(&b.kind)),
            ("object_revision", text(b.revision)),
            ("first_known_generation", text(b.first)),
            ("first_known_cut", text(&b.cut)),
            ("published_generation", text(b.published)),
            (
                "withdrawn_generation",
                b.withdrawn.map(text).unwrap_or(V::Null),
            ),
            (
                "withdrawal_reason",
                b.reason.as_ref().map(text).unwrap_or(V::Null),
            ),
            (
                "superseded_by",
                b.superseded.as_ref().map(text).unwrap_or(V::Null),
            ),
            (
                "lifecycle",
                text(if b.withdrawn.is_some() {
                    "withdrawn"
                } else {
                    "active"
                }),
            ),
        ] {
            match_field(store, &record, k, v)?;
        }
        resolve_record_batch(store, root, &path, &record, b.first, &b.batch, m, batch)?;
    }
    Ok(())
}
fn resolve_observations(
    store: &mut dyn NodeStore,
    root: &mut ContentRef,
    births: &BTreeMap<NodeDigest, ObservationBirth>,
    m: &ManifestMetadata,
    batch: &str,
    delta: bool,
) -> Result<(), String> {
    let expected: BTreeSet<_> = births
        .iter()
        .filter(|(_, b)| !delta || b.first == m.generation)
        .map(|(id, _)| *id)
        .collect();
    let n = array_len(store, root, "observations")?;
    if n != expected.len() as u64 {
        return Err(err("观察与独立索引成员数不符"));
    }
    let mut seen = BTreeSet::new();
    for i in 0..n {
        let path = [field("observations"), PathStep::Index(i)];
        let record = lookup_path(store, root, &path).map_err(err)?;
        let id = lookup_path(store, &record, &[field("observation_id")])
            .map_err(err)?
            .digest()
            .map_err(err)?;
        if !expected.contains(&id) || !seen.insert(id) {
            return Err(err("观察有越界/重复身份"));
        }
        let b = &births[&id];
        match_field(store, &record, "kind", text(&b.kind))?;
        resolve_record_batch(store, root, &path, &record, b.first, &b.batch, m, batch)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{canonical_json, json, meta_set, sha256_hex, tb02a, SCHEMA};
    use super::*;
    use content_manifest::SourceCommit;
    use rusqlite::{params, Connection};
    use sc::{NumberToken, Value as V};
    use serde_json::Value as J;

    fn cv(v: &J) -> V {
        match v {
            J::Null => V::Null,
            J::Bool(x) => V::Bool(*x),
            J::Number(x) => V::Number(NumberToken::parse(&x.to_string()).unwrap()),
            J::String(x) => V::String(x.clone()),
            J::Array(xs) => V::Array(xs.iter().map(cv).collect()),
            J::Object(xs) => V::Object(xs.iter().map(|(k, v)| (k.clone(), cv(v))).collect()),
        }
    }
    fn object(id: &str, generation: &str, batch: V) -> V {
        V::Object(vec![
            ("object_id".into(), V::String(id.into())),
            ("object_revision".into(), V::String("1".into())),
            ("kind".into(), V::String("CC-010.bi".into())),
            (
                "first_known_generation".into(),
                V::String(generation.into()),
            ),
            (
                "first_known_cut".into(),
                V::String(format!("cut-{generation}")),
            ),
            ("published_generation".into(), V::String(generation.into())),
            ("withdrawn_generation".into(), V::Null),
            ("withdrawal_reason".into(), V::Null),
            ("superseded_by".into(), V::Null),
            ("lifecycle".into(), V::String("active".into())),
            ("batch_id".into(), batch),
            (
                "payload".into(),
                cv(&json!({"batch_id":"原文里的同名字段"})),
            ),
        ])
    }
    fn field_set(v: &mut V, key: &str, value: V) {
        let V::Object(fields) = v else {
            panic!("fixture object")
        };
        fields.iter_mut().find(|(k, _)| k == key).unwrap().1 = value;
    }
    #[derive(Clone, Copy)]
    enum FixtureChange {
        None,
        PayloadPlaceholder,
        OldPlaceholder,
        CurrentBatchString,
        Duplicate,
        Observations,
        OldObservationPlaceholder,
        LegacyFullDelta,
    }

    fn fixtures(tx: &Transaction<'_>) -> (ManifestMetadata, ManifestRoots, UnpublishedManifest) {
        fixtures_with(tx, FixtureChange::None)
    }

    fn fixtures_with(
        tx: &Transaction<'_>,
        change: FixtureChange,
    ) -> (ManifestMetadata, ManifestRoots, UnpublishedManifest) {
        tx.execute_batch(SCHEMA).unwrap();
        tx.execute_batch(v2::SCHEMA_V2).unwrap();
        tx.execute_batch(tb02a::SCHEMA).unwrap();
        let clock=v2::ClockPlan::parse(json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"birth-test","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{"event-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"24"}]}}})).unwrap();
        let payload = json!({"op":"ingest","clock_event_id":"event-2"});
        let ph = sha256_hex(canonical_json(&payload).as_bytes());
        let envelope = json!({"schema_revision":"s-session/3","session_id":"birth-test","session_generation":"1","source_namespace":"feed","source_epoch":"epoch","message_id":"message-2","producer_id":"producer","producer_epoch":"2","payload_hash":ph,"causal_refs":[],"payload":payload});
        for (k, v) in [
            ("protocol_revision", "s-session/3"),
            ("session_id", "birth-test"),
            ("writer_epoch", "2"),
            ("generation", "1"),
            ("structure_cut", "cut-1"),
            ("last_advance_frontier", "0"),
            ("begin_token", "token-2"),
            ("advance_state", "begun:token-2:2:1"),
            ("profile_id", tb02a::PROFILE),
            ("rule_revision", "fixture-rule"),
            ("catalog_revision", "fixture-catalog"),
        ] {
            meta_set(tx, k, v).unwrap();
        }
        meta_set(tx, "profile_hash", &"c".repeat(64)).unwrap();
        let scope = json!({"structure":"CompleteCut","economic":"not_started"});
        meta_set(tx, "scope", &canonical_json(&scope)).unwrap();
        tx.execute(
            "INSERT INTO s_protocol_meta VALUES(1,'s-session/3','1',?1,?2,2,'21')",
            params![clock.hash, canonical_json(&clock.value)],
        )
        .unwrap();
        tx.execute("INSERT INTO s_input_messages VALUES('feed','epoch','message-2',?1,?2,'receipt-2',1,'AcceptedPending',NULL,'event-2',1)",params![ph,canonical_json(&envelope)]).unwrap();
        tx.execute(
            "INSERT INTO s_clock_events VALUES('event-2','begin',0,'21',?1,1,2)",
            params![canonical_json(&json!(["feed", "epoch", "message-2"]))],
        )
        .unwrap();
        tx.execute("INSERT INTO raw_events VALUES('raw-2',1,'1','hash','receipt-2',1,'feed','epoch','fixture','event-2','now','original','1','1','1','1',NULL)",[]).unwrap();
        content_store::init_schema(tx).unwrap();
        crate::v3::aux_index::init_schema(tx).unwrap();
        crate::v3::object_index::init_schema(tx).unwrap();
        let mut store = content_store::TxStore::new(tx).unwrap();
        let m = ManifestMetadata {
            session_id: "birth-test".into(),
            session_generation: "1".into(),
            writer_epoch: "2".into(),
            generation: 2,
            base_cut: "cut-1".into(),
            structure_cut: "cut-2".into(),
            input_frontier: 1,
            source_commit: SourceCommit {
                source_namespace: "feed".into(),
                source_epoch: "epoch".into(),
                message_id: "message-2".into(),
                payload_hash: ph,
                producer_id: "producer".into(),
                producer_epoch: "2".into(),
                clock_event_id: "event-2".into(),
                attempt_index: 0,
            },
            clock_plan_hash: clock.hash,
            semantic_commit_ns: "24".into(),
            profile_id: tb02a::PROFILE.into(),
            profile_hash: "c".repeat(64),
            rule_revision: "fixture-rule".into(),
            catalog_revision: "fixture-catalog".into(),
            scope: sc::write_value(&mut store, &cv(&scope))
                .unwrap()
                .content_ref(),
        };
        // 这些夹具只测独立出生许可；不声称两个 fixture ID 通过对象身份核验。
        let mut old = object(
            "obj3-old",
            "1",
            V::String(format!("batch3-{}", "d".repeat(64))),
        );
        let mut new = object("obj3-new", "2", V::BatchSelf);
        if matches!(change, FixtureChange::PayloadPlaceholder) {
            field_set(
                &mut new,
                "payload",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
        }
        if matches!(change, FixtureChange::OldPlaceholder) {
            field_set(&mut old, "batch_id", V::BatchSelf);
        }
        if matches!(change, FixtureChange::CurrentBatchString) {
            field_set(
                &mut new,
                "batch_id",
                V::String(format!("batch3-{}", "d".repeat(64))),
            );
        }
        let generated = cv(
            &json!({"generation":2,"structure_cut":"cut-2","input_frontier":1,"catalog_revision":"fixture-catalog","profile_id":tb02a::PROFILE,"profile_hash":"c".repeat(64),"rule_revision":"fixture-rule","raw_events":[],"effective_events":[],"objects":[],"witnesses":[],"relations":[],"observations":[],"scope":scope}),
        );
        let mut publication = cv(
            &json!({"session_id":"birth-test","generation":"2","structure_cut":"cut-2","catalog_revision":"fixture-catalog","scope":scope,"index_frontier":null,"profile_id":tb02a::PROFILE,"profile_hash":"c".repeat(64),"history_mode":"RecomputedWithRevision","as_of_generation":null,"objects":[],"withdrawn_objects":[],"witnesses":[],"relations":[],"observations":[],"raw_history":[],"input_frontier":"1","seq_range":{"from":"1","to":"1"},"catalog_run_status":"not_run","catalog_evidence":{"batch_id":null}}),
        );
        let old_for_delta = old.clone();
        let objects = if matches!(change, FixtureChange::Duplicate) {
            vec![old.clone(), old]
        } else {
            vec![old, new.clone()]
        };
        field_set(&mut publication, "objects", V::Array(objects));
        let mut delta = cv(
            &json!({"generation":"2","session_id":"birth-test","catalog_revision":"fixture-catalog","base_cut":"cut-1","next_cut":"cut-2","seq_range":{"from":"1","to":"1"},"index_frontier":null,"input_frontier":"1","catalog_run_status":"not_run","catalog_evidence":{"batch_id":null},"upserts":[],"withdrawals":[],"replaces":[],"witnesses":[],"relations":[],"observations":[],"raw_history_added":[]}),
        );
        field_set(
            &mut delta,
            "upserts",
            V::Array(if matches!(change, FixtureChange::LegacyFullDelta) {
                vec![old_for_delta, new]
            } else {
                vec![new]
            }),
        );
        let has_observations = matches!(
            change,
            FixtureChange::Observations | FixtureChange::OldObservationPlaceholder
        );
        if has_observations {
            let ob = |id: &str, batch: V| {
                V::Object(vec![
                    ("observation_id".into(), text(id)),
                    ("kind".into(), text("insufficient_knowledge")),
                    ("batch_id".into(), batch),
                    ("detail".into(), cv(&json!({"batch_id":"原样保留"}))),
                ])
            };
            let old_batch = if matches!(change, FixtureChange::OldObservationPlaceholder) {
                V::BatchSelf
            } else {
                text(format!("batch3-{}", "e".repeat(64)))
            };
            let new_ob = ob("ob-new", V::BatchSelf);
            field_set(
                &mut publication,
                "observations",
                V::Array(vec![ob("ob-old", old_batch), new_ob.clone()]),
            );
            field_set(&mut delta, "observations", V::Array(vec![new_ob]));
        }
        for root in [&mut publication, &mut delta] {
            field_set(root, "index_frontier", V::BatchSelf);
            field_set(
                root,
                "catalog_evidence",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
        }
        let roots = ManifestRoots {
            generated_state: sc::write_value(&mut store, &generated)
                .unwrap()
                .content_ref(),
            publication_template: sc::write_template(&mut store, &publication)
                .unwrap()
                .content_ref(),
            delta_template: sc::write_template(&mut store, &delta)
                .unwrap()
                .content_ref(),
        };
        let token = format!(
            "begin-{}",
            sha256_hex(
                canonical_json(&json!(["birth-test", "1", "2", "1", "cut-1", "2", "1"])).as_bytes()
            )
        );
        meta_set(tx, "begin_token", &token).unwrap();
        meta_set(tx, "advance_state", &format!("begun:{token}:2:1")).unwrap();
        let manifest = content_manifest::assemble_unpublished(&mut store, &m, &roots).unwrap();
        for (id, g, batch) in [
            ("obj3-old", 1, format!("batch3-{}", "d".repeat(64))),
            ("obj3-new", 2, manifest.batch_id.clone()),
        ] {
            let node = sc::write_value(&mut store, &V::Null).unwrap();
            tx.execute("INSERT INTO s_content_objects VALUES(?1,1,'CC-010.bi','key',NULL,NULL,NULL,?5,?5,?5,?5,?2,?3,?4,?3,NULL,NULL,NULL)",params![id,batch,g,format!("cut-{g}"),node.digest.as_slice()]).unwrap();
        }
        if has_observations {
            for (id, g, batch) in [
                ("ob-old", 1, format!("batch3-{}", "e".repeat(64))),
                ("ob-new", 2, manifest.batch_id.clone()),
            ] {
                // 本夹具只核出生许可，值根的原件语义在辅助索引测试单独核。
                let root = sc::write_value(&mut store, &V::Null).unwrap();
                tx.execute("INSERT INTO s_content_observations VALUES(?1,'insufficient_knowledge',NULL,?2,?3,?3,?4)",params![id,batch,root.digest.as_slice(),g]).unwrap();
            }
        }
        (m, roots, manifest)
    }
    fn rendered(store: &mut dyn sc::NodeSource, r: &ContentRef) -> J {
        let mut out = Vec::new();
        sc::render_canonical(store, r, &mut out).unwrap();
        serde_json::from_slice(&out).unwrap()
    }
    #[test]
    fn historical_template_resolution_ignores_future_births_and_withdrawals() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let before =
            resolve_indexed_templates(&tx, &mut store, &m, &roots, &manifest.batch_id, 0).unwrap();
        meta_set(&tx, "generation", "3").unwrap();
        tx.execute("UPDATE s_content_objects SET withdrawn_generation=3,withdrawal_reason='superseded',superseded_by='future-object' WHERE object_id='obj3-old'",[]).unwrap();
        let future = sc::write_value(&mut store, &V::Null).unwrap();
        tx.execute("INSERT INTO s_content_objects VALUES('future-object',1,'CC-010.bi','future-key',NULL,NULL,NULL,?1,?1,?1,?1,'future-batch',3,'cut-3',3,NULL,NULL,NULL)",params![future.digest.as_slice()]).unwrap();
        let node = sc::write_value(&mut store, &V::Null).unwrap();
        tx.execute("INSERT INTO s_content_observations VALUES('future-observation','insufficient_knowledge',NULL,'future-batch',?1,?1,3)",params![node.digest.as_slice()]).unwrap();
        let after =
            resolve_indexed_templates(&tx, &mut store, &m, &roots, &manifest.batch_id, 0).unwrap();
        assert_eq!(before.snapshot, after.snapshot);
        assert_eq!(before.delta, after.delta);
    }
    #[test]
    fn content_publication_resolves_new_birth_and_preserves_old_batch_and_payload() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let token = meta(&tx, "begin_token").unwrap();
        let resolved = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap();
        let mut store = content_store::ReadStore::new(&tx).unwrap();
        let snapshot = rendered(&mut store, &resolved.snapshot);
        let delta = rendered(&mut store, &resolved.delta);
        assert_eq!(
            snapshot["objects"][0]["batch_id"],
            format!("batch3-{}", "d".repeat(64))
        );
        assert_eq!(snapshot["objects"][1]["batch_id"], manifest.batch_id);
        assert_eq!(
            snapshot["objects"][1]["payload"]["batch_id"],
            "原文里的同名字段"
        );
        assert_eq!(delta["upserts"][0], snapshot["objects"][1]);
        assert_eq!(snapshot["index_frontier"], manifest.batch_id);
        assert_eq!(delta["catalog_evidence"]["batch_id"], manifest.batch_id);
        assert_eq!(
            super::super::meta_get_opt(&tx, "generation")
                .unwrap()
                .as_deref(),
            Some("1")
        );
    }

    #[test]
    fn content_publication_refuses_missing_or_changed_independent_authority() {
        for sql in [
            "DELETE FROM s_content_objects WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET first_known_generation=1 WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET first_known_cut='cut-1' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET batch_id='other' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET kind='CC-056.version' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET published_generation=2 WHERE object_id='obj3-old'",
            "UPDATE meta SET value='3' WHERE key='writer_epoch'",
            "UPDATE meta SET value='other-token' WHERE key='begin_token'",
            "UPDATE s_protocol_meta SET next_attempt_ordinal=3",
            "UPDATE s_protocol_meta SET logical_phase_frontier='20'",
            "DELETE FROM s_clock_events WHERE phase='begin'",
            "UPDATE s_input_messages SET status='Committed',first_published_generation=2",
            "UPDATE s_input_messages SET attempt_count=2",
            "UPDATE s_input_messages SET receipt_id='other-receipt'",
            "UPDATE meta SET value='1' WHERE key='last_advance_frontier'",
            "UPDATE raw_events SET seq=2",
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures(&tx);
            let token = meta(&tx, "begin_token").unwrap();
            tx.execute(sql, []).unwrap();
            assert!(
                resolve_current_births(&tx, &token, &m, &roots, &manifest).is_err(),
                "必须拒绝：{sql}"
            );
        }
    }

    #[test]
    fn content_publication_refuses_self_asserted_or_out_of_schema_placeholders() {
        for change in [
            FixtureChange::PayloadPlaceholder,
            FixtureChange::OldPlaceholder,
            FixtureChange::CurrentBatchString,
            FixtureChange::Duplicate,
            FixtureChange::OldObservationPlaceholder,
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures_with(&tx, change);
            let token = meta(&tx, "begin_token").unwrap();
            let count = |tx: &Transaction<'_>| -> i64 {
                tx.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
                    .unwrap()
            };
            let before = count(&tx);
            tx.execute_batch("SAVEPOINT rejected_publication").unwrap();
            let error = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap_err();
            if matches!(change, FixtureChange::PayloadPlaceholder) {
                assert!(error.contains(sc::code::TEMPLATE_PLACEHOLDER), "{error}");
            }
            // 操作不自行提交；调用者回滚使包括部分祖先重写在内的所有节点回到原状。
            tx.execute_batch("ROLLBACK TO rejected_publication; RELEASE rejected_publication")
                .unwrap();
            assert_eq!(count(&tx), before);
            assert_eq!(meta(&tx, "generation").unwrap(), "1");
        }
    }

    #[test]
    fn content_publication_observation_birth_is_independent_and_old_batch_stays() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::Observations);
        let token = meta(&tx, "begin_token").unwrap();
        let resolved = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap();
        let mut store = content_store::ReadStore::new(&tx).unwrap();
        let snapshot = rendered(&mut store, &resolved.snapshot);
        let delta = rendered(&mut store, &resolved.delta);
        assert_eq!(
            snapshot["observations"][0]["batch_id"],
            format!("batch3-{}", "e".repeat(64))
        );
        assert_eq!(snapshot["observations"][1]["batch_id"], manifest.batch_id);
        assert_eq!(delta["observations"], json!([snapshot["observations"][1]]));
        assert_eq!(
            snapshot["observations"][1]["detail"]["batch_id"],
            "原样保留"
        );
        tx.execute(
            "UPDATE s_content_observations SET published_generation=1 WHERE observation_id='ob-new'",
            [],
        )
        .unwrap();
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest).is_err());
    }

    #[test]
    fn content_publication_rejects_unrelated_manifest_and_source_producer() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (mut m, roots, mut manifest) = fixtures(&tx);
        let token = meta(&tx, "begin_token").unwrap();
        let actual = manifest.batch_id.clone();
        manifest.batch_id = format!("batch3-{}", "f".repeat(64));
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("IdentityConflict"));
        manifest.batch_id = actual;
        m.source_commit.producer_id = "wrong-producer".into();
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("producer_id"));
    }

    #[test]
    fn content_publication_s3_rejects_legacy_full_delta_without_private_mode_fallback() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::LegacyFullDelta);
        let token = meta(&tx, "begin_token").unwrap();
        let count: i64 = tx
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name='s_storage_format'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("upserts 与独立索引成员数不符"));
    }
}
