//! #1394 C §5.4：见证、关系、首次观察的独立内容索引。
//! plan 先产生含本批占位的值根；manifest 求出 batch3 后 stage 同事务暂写出生记录。
//! 观察 detail 与来源数组保留为边；仅单条原始事件沿既有投影转换。索引不替代来源业务审计。
use super::generation::{Generated, SequenceMemo, WitnessRecord};
use crate::{canonical_json, content_store, meta_get_opt, sha256_hex};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, replace_batch_placeholder, PathStep},
    Built, ContentRef, Kind, NodeDigest, NodeSource, NodeStore, Value as V,
};
use rusqlite::{params, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS s_content_witnesses (
 witness_id TEXT PRIMARY KEY, object_id TEXT NOT NULL, slot INTEGER NOT NULL, merged_index INTEGER NOT NULL CHECK(merged_index>=0),
 generated_root BLOB NOT NULL CHECK(typeof(generated_root)='blob' AND length(generated_root)=32),
 wire_root BLOB NOT NULL CHECK(typeof(wire_root)='blob' AND length(wire_root)=32),
 published_generation INTEGER NOT NULL CHECK(published_generation>=1));
CREATE TABLE IF NOT EXISTS s_content_relations (
 relation_id TEXT PRIMARY KEY, subject TEXT NOT NULL, relation_type TEXT NOT NULL, object TEXT NOT NULL,
 generated_root BLOB NOT NULL CHECK(typeof(generated_root)='blob' AND length(generated_root)=32),
 published_generation INTEGER NOT NULL CHECK(published_generation>=1));
CREATE TABLE IF NOT EXISTS s_content_observations (
 observation_id TEXT PRIMARY KEY, kind TEXT NOT NULL, window_start INTEGER, batch_id TEXT NOT NULL,
 generated_root BLOB NOT NULL CHECK(typeof(generated_root)='blob' AND length(generated_root)=32),
 birth_wire_root BLOB NOT NULL CHECK(typeof(birth_wire_root)='blob' AND length(birth_wire_root)=32),
 published_generation INTEGER NOT NULL CHECK(published_generation>=1));";

pub(crate) fn init_schema(tx: &Transaction<'_>) -> Result<(), String> {
    content_store::ensure_format(tx).map_err(err)?;
    if meta_get_opt(tx, "protocol_revision")?.as_deref() != Some(super::PROTOCOL_REVISION) {
        return Err(err("根协议不是S3"));
    }
    tx.execute_batch(SCHEMA).map_err(err)
}
fn err(e: impl std::fmt::Display) -> String {
    format!("AuxIndexInvalid：{e}")
}
fn f(k: &str) -> PathStep {
    PathStep::Field(k.into())
}
fn field(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[f(k)]).map_err(err)
}
fn digest(r: &ContentRef) -> Result<NodeDigest, String> {
    r.digest().map_err(err)
}
fn scalar(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<Value, String> {
    let leaf = field(s, r, k)?;
    if !matches!(
        leaf.kind,
        Kind::Null | Kind::Bool | Kind::Number | Kind::String
    ) {
        return Err(err(format!("{k}不是标量")));
    }
    render(s, &leaf)
}
fn text(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<String, String> {
    scalar(s, r, k)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| err(format!("{k}不是文本")))
}
fn integer(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<i64, String> {
    scalar(s, r, k)?
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是i64")))
}
fn render(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Value, String> {
    let mut b = Vec::new();
    sc::render_canonical(s, r, &mut b).map_err(err)?;
    newchan_rust::session_protocol::strict_json(&b)
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(x) => V::Number(sc::NumberToken::parse(&x.to_string()).map_err(err)?),
        Value::Array(xs) => V::Array(xs.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(xs) => V::Object(
            xs.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
fn write_json(s: &mut dyn NodeStore, v: &Value) -> Result<Built, String> {
    sc::write_value(s, &cv(v)?).map_err(err)
}
fn edge(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<V, String> {
    Ok(V::Edge(digest(&field(s, r, k)?)?))
}
fn array_elements(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Vec<ContentRef>, String> {
    if r.kind != Kind::Array {
        return Err(err("预期数组"));
    }
    (0..r.extent.ok_or_else(|| err("数组缺长度"))?)
        .map(|i| lookup_path(s, r, &[PathStep::Index(i)]).map_err(err))
        .collect()
}
fn root(s: &mut dyn NodeSource, b: Vec<u8>) -> Result<ContentRef, String> {
    let d: NodeDigest = b.try_into().map_err(|_| err("根不是32字节"))?;
    let r = sc::content_ref_of(s, &sc::root_string(&d)).map_err(err)?;
    sc::verify(s, &r).map_err(err)?;
    Ok(r)
}
fn exact(r: &ContentRef, n: u64) -> Result<(), String> {
    if r.kind != Kind::Object || r.extent != Some(n) {
        Err(err(format!("预期{n}字段对象")))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(super) struct Witness {
    pub id: String,
    pub object_id: String,
    pub slot: i64,
    pub merged_index: i64,
    pub generated: ContentRef,
    pub wire: ContentRef,
    pub first: i64,
}
#[derive(Clone, Debug)]
pub(super) struct Relation {
    pub id: String,
    pub subject: String,
    pub relation_type: String,
    pub object: String,
    pub generated: ContentRef,
    pub first: i64,
}
#[derive(Clone, Debug)]
pub(super) struct Observation {
    pub id: String,
    pub kind: String,
    pub window_start: Option<i64>,
    pub generated: ContentRef,
    pub wire: ContentRef,
    pub batch: Option<String>,
    pub first: i64,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Index {
    pub witnesses: Vec<Witness>,
    pub relations: Vec<Relation>,
    pub observations: Vec<Observation>,
}
pub(super) struct PublicSets {
    pub witnesses: Built,
    pub relations: Built,
    pub observations: Built,
    pub raw_history: Built,
}
pub(super) struct Plan {
    pub generation: i64,
    pub snapshot: Index,
    pub births: Index,
    pub snapshot_values: PublicSets,
    pub delta_values: PublicSets,
}

/// 缓存只按真实内容根命中；回滚/换库必须 discard，不能靠缓存绕过旧历史审计。
#[derive(Default)]
pub(super) struct Runtime {
    witness_wires: BTreeMap<(NodeDigest, i64), ContentRef>,
    bar_wires: BTreeMap<NodeDigest, ContentRef>,
    observation_ids: BTreeMap<NodeDigest, String>,
    raw_wires: BTreeMap<NodeDigest, NodeDigest>,
    sequences: [SequenceMemo; 8],
}
impl Runtime {
    pub(super) fn discard(&mut self) {
        *self = Self::default();
    }

    fn witness(
        &mut self,
        s: &mut dyn NodeStore,
        w: &WitnessRecord,
        merged_index: i64,
        g: i64,
    ) -> Result<Witness, String> {
        let generated = w.built.content_ref();
        sc::verify(s, &generated).map_err(err)?;
        exact(&generated, 9)?;
        let object_id = text(s, &generated, "object_id")?;
        let slot = integer(s, &generated, "slot")?;
        let id = text(s, &generated, "witness_id")?;
        if object_id != w.object_id
            || slot != w.slot as i64
            || id != w.witness_id
            || !(0..3).contains(&slot)
        {
            return Err(err("见证生成记录与元数据不符"));
        }
        let wire = match self.witness_wires.get(&(w.built.digest, merged_index)) {
            Some(r) => r.clone(),
            None => {
                verify_witness_id(s, &generated, &id, &object_id, slot, merged_index)?;
                let r = self.witness_wire(s, &generated)?;
                self.witness_wires
                    .insert((w.built.digest, merged_index), r.clone());
                r
            }
        };
        Ok(Witness {
            id,
            object_id,
            slot,
            merged_index,
            generated,
            wire,
            first: g,
        })
    }
    fn witness_wire(
        &mut self,
        s: &mut dyn NodeStore,
        r: &ContentRef,
    ) -> Result<ContentRef, String> {
        let bars = field(s, r, "raw_bars")?;
        let mut values = Vec::new();
        for bar in array_elements(s, &bars)? {
            let d = digest(&bar)?;
            let projected = if let Some(cached) = self.bar_wires.get(&d) {
                cached.clone()
            } else {
                // 一次只物化一条来源原件，整条 raw_bars 始终由内容边组成。
                let original = render(s, &bar)?;
                let one = crate::project_raw_bars(&json!([original]))?;
                let built = write_json(s, &one[0])?.content_ref();
                self.bar_wires.insert(d, built.clone());
                built
            };
            values.push(V::Edge(digest(&projected)?));
        }
        let mut fields = Vec::new();
        for key in [
            "witness_id",
            "object_id",
            "merged_high",
            "merged_low",
            "merged_open",
            "merged_close",
        ] {
            fields.push((key.into(), edge(s, r, key)?));
        }
        for key in ["slot", "merged_source_index"] {
            fields.push((key.into(), V::String(integer(s, r, key)?.to_string())));
        }
        fields.push(("raw_bars".into(), V::Array(values)));
        Ok(sc::write_value(s, &V::Object(fields))
            .map_err(err)?
            .content_ref())
    }
    fn observation(
        &mut self,
        s: &mut dyn NodeStore,
        b: &Built,
        g: i64,
    ) -> Result<Observation, String> {
        let generated = b.content_ref();
        sc::verify(s, &generated).map_err(err)?;
        exact(&generated, 6)?;
        let kind = text(s, &generated, "kind")?;
        let window_start = window(s, &generated, "window_start")?;
        let id = if let Some(id) = self.observation_ids.get(&b.digest) {
            id.clone()
        } else {
            let id = observation_id(s, &generated)?;
            self.observation_ids.insert(b.digest, id.clone());
            id
        };
        let wire = observation_wire(s, &generated, &id, None)?;
        Ok(Observation {
            id,
            kind,
            window_start,
            generated,
            wire,
            batch: None,
            first: g,
        })
    }

    pub(super) fn plan(
        &mut self,
        s: &mut dyn NodeStore,
        old: &Index,
        generated: &Generated<'_>,
        replaces: &[Value],
        raw: &[Value],
        previous_frontier: i64,
        frontier: i64,
        g: i64,
    ) -> Result<Plan, String> {
        if g < 1
            || previous_frontier < -1
            || frontier <= previous_frontier
            || frontier as usize >= raw.len()
        {
            return Err(err("代号/原始前沿非法"));
        }
        let mut snapshot = old.clone();
        let mut births = Index::default();
        let mut ws: BTreeMap<String, usize> = snapshot
            .witnesses
            .iter()
            .enumerate()
            .map(|(i, w)| (w.id.clone(), i))
            .collect();
        for w in &generated.witnesses {
            let object = generated
                .objects
                .iter()
                .find(|o| o.object_id() == w.object_id)
                .ok_or_else(|| err("见证对象不在本代生成态"))?;
            let group = lookup_path(
                s,
                &object.generated().content_ref(),
                &[
                    f("input_refs_json"),
                    PathStep::JsonText,
                    PathStep::Index(w.slot as u64),
                ],
            )
            .map_err(err)?;
            // 身份配方用合并序号 mi；公开 merged_source_index 是原始坐标，二者不可互换。
            let merged_index = integer(s, &group, "merged_index")?;
            let row = self.witness(s, w, merged_index, g)?;
            if let Some(i) = ws.get(&row.id) {
                let prior = &snapshot.witnesses[*i];
                if prior.generated != row.generated
                    || prior.wire != row.wire
                    || prior.merged_index != row.merged_index
                {
                    return Err(err("同见证身份换内容"));
                }
            } else {
                ws.insert(row.id.clone(), snapshot.witnesses.len());
                snapshot.witnesses.push(row.clone());
                births.witnesses.push(row);
            }
        }
        let mut rs: BTreeMap<String, usize> = snapshot
            .relations
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        let mut candidates = generated
            .relations
            .iter()
            .map(Built::content_ref)
            .collect::<Vec<_>>();
        for r in replaces {
            let subject = r["new_object_id"]
                .as_str()
                .ok_or_else(|| err("replaces缺new_object_id"))?;
            let object = r["old_object_id"]
                .as_str()
                .ok_or_else(|| err("replaces缺old_object_id"))?;
            candidates.push(
                write_json(
                    s,
                    &json!({"subject":subject,"relation_type":"replaces","object":object}),
                )?
                .content_ref(),
            );
        }
        for ev in &raw[..=frontier as usize] {
            if let Some(sup) = ev["supersedes_revision"].as_i64() {
                let identity = ev["identity_key"]
                    .as_str()
                    .ok_or_else(|| err("源身份缺失"))?;
                let rev = ev["revision"].as_i64().ok_or_else(|| err("源版本缺失"))?;
                candidates.push(write_json(s,&json!({"subject":format!("{identity}@{rev}"),"relation_type":"supersedes","object":format!("{identity}@{sup}")}))?.content_ref());
            }
        }
        for r in candidates {
            let row = relation(s, r, g)?;
            if let Some(i) = rs.get(&row.id) {
                if snapshot.relations[*i].generated != row.generated {
                    return Err(err("同关系身份换内容"));
                }
            } else {
                rs.insert(row.id.clone(), snapshot.relations.len());
                snapshot.relations.push(row.clone());
                births.relations.push(row);
            }
        }
        let mut obs: BTreeMap<String, usize> = snapshot
            .observations
            .iter()
            .enumerate()
            .map(|(i, o)| (o.id.clone(), i))
            .collect();
        for b in &generated.observations {
            let row = self.observation(s, b, g)?;
            if let Some(i) = obs.get(&row.id) {
                // 旧非 tb02a 配方不把 detail 纳入身份：重现保留首版，绝不覆盖；
                // 对哈希身份字段逐值核对，tb02a 全内容身份还须整根相同。
                let prior = &snapshot.observations[*i];
                if observation_identity_value(s, &prior.generated)?
                    != observation_identity_value(s, &row.generated)?
                {
                    return Err(err("同观察身份的身份字段冲突"));
                }
            } else {
                obs.insert(row.id.clone(), snapshot.observations.len());
                snapshot.observations.push(row.clone());
                births.observations.push(row);
            }
        }
        snapshot
            .witnesses
            .sort_by(|a, b| (&a.object_id, a.slot).cmp(&(&b.object_id, b.slot)));
        snapshot.relations.sort_by(|a, b| {
            (&a.subject, &a.relation_type, &a.object).cmp(&(
                &b.subject,
                &b.relation_type,
                &b.object,
            ))
        });
        // 稳定排序保持 SQL 相同键下的首插入顺序。
        snapshot
            .observations
            .sort_by(|a, b| (&a.kind, a.window_start).cmp(&(&b.kind, b.window_start)));
        let mut raw_nodes = Vec::new();
        for (n, event) in raw[..=frontier as usize].iter().enumerate() {
            if event["seq"].as_i64() != Some(n as i64) {
                return Err(err("原始序号与输入前缀不符"));
            }
            let key: NodeDigest = Sha256::digest(canonical_json(event).as_bytes()).into();
            let wire = if let Some(d) = self.raw_wires.get(&key) {
                *d
            } else {
                let d = write_json(s, &crate::project_raw_event_wire(event)?)?.digest;
                self.raw_wires.insert(key, d);
                d
            };
            raw_nodes.push(wire);
        }
        let snapshot_values = public_sets(s, &snapshot, &raw_nodes, &mut self.sequences[..4])?;
        let delta_values = public_sets(
            s,
            &births,
            &raw_nodes[(previous_frontier + 1) as usize..],
            &mut self.sequences[4..],
        )?;
        Ok(Plan {
            generation: g,
            snapshot,
            births,
            snapshot_values,
            delta_values,
        })
    }
}

fn public_sets(
    s: &mut dyn NodeStore,
    i: &Index,
    raw: &[NodeDigest],
    memo: &mut [SequenceMemo],
) -> Result<PublicSets, String> {
    let w = i
        .witnesses
        .iter()
        .map(|r| digest(&r.wire))
        .collect::<Result<Vec<_>, _>>()?;
    let r = i
        .relations
        .iter()
        .map(|r| digest(&r.generated))
        .collect::<Result<Vec<_>, _>>()?;
    let o = i
        .observations
        .iter()
        .map(|r| digest(&r.wire))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PublicSets {
        witnesses: memo[0].sync(s, &w, "见证快照")?,
        relations: memo[1].sync(s, &r, "关系快照")?,
        observations: memo[2].sync(s, &o, "观察快照")?,
        raw_history: memo[3].sync(s, raw, "原始历史")?,
    })
}
fn window(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<Option<i64>, String> {
    let v = scalar(s, r, k)?;
    if v.is_null() {
        Ok(None)
    } else {
        v.as_i64().map(Some).ok_or_else(|| err("窗口不是i64/null"))
    }
}
fn observation_wire(
    s: &mut dyn NodeStore,
    r: &ContentRef,
    id: &str,
    batch: Option<&str>,
) -> Result<ContentRef, String> {
    let mut fields = vec![
        ("observation_id".into(), V::String(id.into())),
        (
            "batch_id".into(),
            batch.map(|b| V::String(b.into())).unwrap_or(V::BatchSelf),
        ),
    ];
    for k in ["kind", "reason", "detail"] {
        fields.push((k.into(), edge(s, r, k)?));
    }
    for k in ["window_start", "window_mid", "window_end"] {
        fields.push((
            k.into(),
            window(s, r, k)?
                .map(|n| V::String(n.to_string()))
                .unwrap_or(V::Null),
        ));
    }
    Ok(sc::write_template(s, &V::Object(fields))
        .map_err(err)?
        .content_ref())
}
fn relation(s: &mut dyn NodeStore, generated: ContentRef, g: i64) -> Result<Relation, String> {
    sc::verify(s, &generated).map_err(err)?;
    exact(&generated, 3)?;
    let subject = text(s, &generated, "subject")?;
    let relation_type = text(s, &generated, "relation_type")?;
    let object = text(s, &generated, "object")?;
    let id = format!(
        "rel-{}",
        &sha256_hex(canonical_json(&json!({"s":subject,"t":relation_type,"o":object})).as_bytes())
            [..16]
    );
    Ok(Relation {
        id,
        subject,
        relation_type,
        object,
        generated,
        first: g,
    })
}
struct HashWriter(Sha256);
impl std::io::Write for HashWriter {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn stream_hash(s: &mut dyn NodeSource, r: &ContentRef) -> Result<String, String> {
    let mut out = HashWriter(Sha256::new());
    sc::render_canonical(s, r, &mut out).map_err(err)?;
    Ok(format!("{:x}", out.0.finalize()))
}
fn verify_witness_id(
    s: &mut dyn NodeStore,
    r: &ContentRef,
    id: &str,
    object: &str,
    slot: i64,
    mi: i64,
) -> Result<(), String> {
    if mi < 0 {
        return Err(err("见证合并序号非法"));
    }
    let bars = edge(s, r, "raw_bars")?;
    let recipe = V::Object(vec![
        ("mi".into(), cv(&json!(mi))?),
        ("object_id".into(), V::String(object.into())),
        ("slot".into(), cv(&json!(slot))?),
        ("raw_bars".into(), bars),
    ]);
    let recipe = sc::write_value(s, &recipe).map_err(err)?.content_ref();
    if id != format!("wit-{}", &stream_hash(s, &recipe)?[..16]) {
        return Err(err("见证身份配方不符"));
    }
    Ok(())
}
fn observation_identity_value(s: &mut dyn NodeStore, r: &ContentRef) -> Result<ContentRef, String> {
    let detail = field(s, r, "detail")?;
    if detail.kind != Kind::Object {
        return Err(err("观察detail不是对象"));
    }
    let tb = match lookup_path(s, &detail, &[f("tb02a")]) {
        Ok(value) => value.kind == Kind::Bool && render(s, &value)? == json!(true),
        Err(e) if e.code() == sc::path::path_code::PATH_FIELD_MISSING => false,
        Err(e) => return Err(err(e)),
    };
    if tb {
        return Ok(r.clone());
    }
    let mut fields = Vec::new();
    for k in ["kind", "reason", "window_start", "window_mid", "window_end"] {
        fields.push((k.into(), edge(s, r, k)?));
    }
    Ok(sc::write_value(s, &V::Object(fields))
        .map_err(err)?
        .content_ref())
}
fn observation_id(s: &mut dyn NodeStore, r: &ContentRef) -> Result<String, String> {
    let recipe = observation_identity_value(s, r)?;
    let hash = stream_hash(s, &recipe)?;
    Ok(format!(
        "obs-{}",
        if recipe == *r {
            hash.as_str()
        } else {
            &hash[..16]
        }
    ))
}

fn state_members(
    s: &mut dyn NodeSource,
    state: &ContentRef,
    name: &str,
) -> Result<Vec<ContentRef>, String> {
    let members = field(s, state, name)?;
    array_elements(s, &members)
}

/// 生成态与见证/关系/观察独立索引的来源绑定核验。
///
/// 这里只消费调用方已核过的原始前缀；不重跑结构分类，也不展开观察 detail 或 raw_bars
/// 的整段 JSON。生成态当前成员必须能在独立索引找到并逐根相等；本代首次成员则另核
/// 独立索引的首次插入顺序，避免只凭索引自洽放过出生漏项、幽灵行或错绑原件。
pub(super) fn verify_generated_members(
    s: &mut dyn NodeStore,
    index: &Index,
    state: &ContentRef,
    replaces: &[Value],
    raw: &[Value],
    g: i64,
) -> Result<(), String> {
    if g < 1 {
        return Err(err("生成代非法"));
    }

    let mut witness_by_id = BTreeMap::<String, &Witness>::new();
    for row in &index.witnesses {
        if row.first <= g {
            if witness_by_id.insert(row.id.clone(), row).is_some() {
                return Err(err("见证独立索引重复身份"));
            }
        }
    }
    let generated_witnesses = state_members(s, state, "witnesses")?;
    let mut seen = BTreeMap::<String, NodeDigest>::new();
    let mut generated_births = Vec::new();
    for generated in generated_witnesses {
        let id = text(s, &generated, "witness_id")?;
        let generated_digest = digest(&generated)?;
        if seen.insert(id.clone(), generated_digest).is_some() {
            return Err(err("生成见证重复身份"));
        }
        let row = witness_by_id
            .get(&id)
            .ok_or_else(|| err("生成见证不在独立索引内"))?;
        if row.generated != generated {
            return Err(err("生成见证原件与独立索引不一致"));
        }
        if row.first == g {
            generated_births.push(id);
        }
    }
    let indexed_births = index
        .witnesses
        .iter()
        .filter(|row| row.first == g)
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    if generated_births != indexed_births {
        return Err(err("见证出生集合或顺序与独立索引不同"));
    }

    let mut relation_by_id = BTreeMap::<String, &Relation>::new();
    for row in &index.relations {
        if row.first <= g {
            if relation_by_id.insert(row.id.clone(), row).is_some() {
                return Err(err("关系独立索引重复身份"));
            }
        }
    }
    let mut relation_candidates = state_members(s, state, "relations")?;
    for replacement in replaces {
        let subject = replacement["new_object_id"]
            .as_str()
            .ok_or_else(|| err("replaces缺new_object_id"))?;
        let object = replacement["old_object_id"]
            .as_str()
            .ok_or_else(|| err("replaces缺old_object_id"))?;
        relation_candidates.push(
            write_json(
                s,
                &json!({"subject":subject,"relation_type":"replaces","object":object}),
            )?
            .content_ref(),
        );
    }
    for event in raw {
        let Some(supersedes) = event["supersedes_revision"].as_i64() else {
            continue;
        };
        let identity = event["identity_key"]
            .as_str()
            .ok_or_else(|| err("源身份缺失"))?;
        let revision = event["revision"]
            .as_i64()
            .ok_or_else(|| err("源版本缺失"))?;
        relation_candidates.push(
            write_json(
                s,
                &json!({
                    "subject": format!("{identity}@{revision}"),
                    "relation_type": "supersedes",
                    "object": format!("{identity}@{supersedes}"),
                }),
            )?
            .content_ref(),
        );
    }
    let mut relation_seen = BTreeMap::<String, NodeDigest>::new();
    let mut relation_births = Vec::new();
    for candidate in relation_candidates {
        let row = relation(s, candidate, g)?;
        let row_digest = digest(&row.generated)?;
        if let Some(previous) = relation_seen.insert(row.id.clone(), row_digest) {
            if previous != row_digest {
                return Err(err("同关系身份对应多个生成原件"));
            }
            continue;
        }
        let stored = relation_by_id
            .get(&row.id)
            .ok_or_else(|| err("生成关系不在独立索引内"))?;
        if stored.generated != row.generated {
            return Err(err("生成关系原件与独立索引不一致"));
        }
        if stored.first == g {
            relation_births.push(row.id);
        }
    }
    let indexed_relation_births = index
        .relations
        .iter()
        .filter(|row| row.first == g)
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    if relation_births != indexed_relation_births {
        return Err(err("关系出生集合或顺序与独立索引不同"));
    }

    let mut observation_by_id = BTreeMap::<String, &Observation>::new();
    for row in &index.observations {
        if row.first <= g {
            if observation_by_id.insert(row.id.clone(), row).is_some() {
                return Err(err("观察独立索引重复身份"));
            }
        }
    }
    let generated_observations = state_members(s, state, "observations")?;
    let mut observation_seen = BTreeSet::new();
    let mut observation_births = Vec::new();
    for generated in generated_observations {
        let id = observation_id(s, &generated)?;
        if !observation_seen.insert(id.clone()) {
            return Err(err("生成观察重复身份"));
        }
        let stored = observation_by_id
            .get(&id)
            .ok_or_else(|| err("生成观察不在独立索引内"))?;
        if observation_identity_value(s, &stored.generated)?
            != observation_identity_value(s, &generated)?
        {
            return Err(err("生成观察身份字段与独立首版不一致"));
        }
        if stored.first == g {
            if stored.generated != generated {
                return Err(err("观察出生原件与独立首版不一致"));
            }
            observation_births.push(id);
        }
    }
    let indexed_observation_births = index
        .observations
        .iter()
        .filter(|row| row.first == g)
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    if observation_births != indexed_observation_births {
        return Err(err("观察出生集合或顺序与独立索引不同"));
    }
    Ok(())
}

/// 读取独立记录并验证闭包、身份、公开投影和小元数据。store 可为只读源+内存重算层。
/// 完整生成来源/跨代成员审计仍由上层完成，索引自洽不能证明来源真实。
pub(super) fn load(
    tx: &Transaction<'_>,
    s: &mut dyn NodeStore,
    latest: i64,
) -> Result<Index, String> {
    let mut out = Index::default();
    let mut rt = Runtime::default();
    let mut stmt=tx.prepare("SELECT witness_id,object_id,slot,generated_root,wire_root,published_generation,merged_index FROM s_content_witnesses ORDER BY rowid").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        let id: String = row.get(0).map_err(err)?;
        let object_id: String = row.get(1).map_err(err)?;
        let slot: i64 = row.get(2).map_err(err)?;
        let merged_index: i64 = row.get(6).map_err(err)?;
        let generated = root(s, row.get(3).map_err(err)?)?;
        let wire = root(s, row.get(4).map_err(err)?)?;
        let first: i64 = row.get(5).map_err(err)?;
        check_generation(first, latest)?;
        exact(&generated, 9)?;
        if text(s, &generated, "witness_id")? != id
            || text(s, &generated, "object_id")? != object_id
            || integer(s, &generated, "slot")? != slot
            || !(0..3).contains(&slot)
        {
            return Err(err("见证索引与原件不符"));
        }
        verify_witness_id(s, &generated, &id, &object_id, slot, merged_index)?;
        if rt.witness_wire(s, &generated)? != wire {
            return Err(err("见证wire与生成原件不符"));
        }
        out.witnesses.push(Witness {
            id,
            object_id,
            slot,
            merged_index,
            generated,
            wire,
            first,
        });
    }
    let mut stmt=tx.prepare("SELECT relation_id,subject,relation_type,object,generated_root,published_generation FROM s_content_relations ORDER BY rowid").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        let first: i64 = row.get(5).map_err(err)?;
        check_generation(first, latest)?;
        let generated = root(s, row.get(4).map_err(err)?)?;
        let r = relation(s, generated, first)?;
        if r.id != row.get::<_, String>(0).map_err(err)?
            || r.subject != row.get::<_, String>(1).map_err(err)?
            || r.relation_type != row.get::<_, String>(2).map_err(err)?
            || r.object != row.get::<_, String>(3).map_err(err)?
        {
            return Err(err("关系索引与原件不符"));
        }
        out.relations.push(r);
    }
    let mut stmt=tx.prepare("SELECT observation_id,kind,window_start,batch_id,generated_root,birth_wire_root,published_generation FROM s_content_observations ORDER BY rowid").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        let id: String = row.get(0).map_err(err)?;
        let kind: String = row.get(1).map_err(err)?;
        let window_start: Option<i64> = row.get(2).map_err(err)?;
        let batch: String = row.get(3).map_err(err)?;
        check_batch(&batch)?;
        let generated = root(s, row.get(4).map_err(err)?)?;
        let wire = root(s, row.get(5).map_err(err)?)?;
        let first: i64 = row.get(6).map_err(err)?;
        check_generation(first, latest)?;
        exact(&generated, 6)?;
        if text(s, &generated, "kind")? != kind
            || window(s, &generated, "window_start")? != window_start
            || observation_id(s, &generated)? != id
            || observation_wire(s, &generated, &id, Some(&batch))? != wire
        {
            return Err(err("观察索引与出生原件不符"));
        }
        out.observations.push(Observation {
            id,
            kind,
            window_start,
            batch: Some(batch),
            generated,
            wire,
            first,
        });
    }
    Ok(out)
}
fn check_generation(first: i64, latest: i64) -> Result<(), String> {
    if first < 1 || first > latest {
        Err(err("索引出生代号越界"))
    } else {
        Ok(())
    }
}

/// 历史 cut 的独立成员方程：Snapshot 全历史按旧公开顺序；Delta 仅本代首次、按插入序。
/// load 按 rowid 返回，保留首次插入顺序。
pub(super) fn cut_members(
    index: &Index,
    generation: i64,
    delta: bool,
) -> Result<[Vec<NodeDigest>; 3], String> {
    let member = |first| {
        if delta {
            first == generation
        } else {
            first <= generation
        }
    };
    let mut w = index
        .witnesses
        .iter()
        .filter(|r| member(r.first))
        .collect::<Vec<_>>();
    let mut r = index
        .relations
        .iter()
        .filter(|r| member(r.first))
        .collect::<Vec<_>>();
    let mut o = index
        .observations
        .iter()
        .filter(|r| member(r.first))
        .collect::<Vec<_>>();
    if !delta {
        w.sort_by(|a, b| (&a.object_id, a.slot).cmp(&(&b.object_id, b.slot)));
        r.sort_by(|a, b| {
            (&a.subject, &a.relation_type, &a.object).cmp(&(
                &b.subject,
                &b.relation_type,
                &b.object,
            ))
        });
        o.sort_by(|a, b| (&a.kind, a.window_start).cmp(&(&b.kind, b.window_start)));
    }
    Ok([
        w.iter()
            .map(|v| digest(&v.wire))
            .collect::<Result<_, _>>()?,
        r.iter()
            .map(|v| digest(&v.generated))
            .collect::<Result<_, _>>()?,
        o.iter()
            .map(|v| digest(&v.wire))
            .collect::<Result<_, _>>()?,
    ])
}
fn check_batch(batch: &str) -> Result<(), String> {
    if batch.strip_prefix("batch3-").is_none_or(|h| {
        h.len() != 64
            || !h
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        Err(err("非法batch3身份"))
    } else {
        Ok(())
    }
}

/// manifest 已得到真实身份之后调用；任何后续错误由调用方回滚整个事务。无 INSERT OR IGNORE。
pub(super) fn stage(
    tx: &Transaction<'_>,
    s: &mut dyn NodeStore,
    p: &Plan,
    batch: &str,
) -> Result<(), String> {
    check_batch(batch)?;
    for w in &p.births.witnesses {
        if w.first != p.generation {
            return Err(err("见证出生代错绑"));
        }
        tx.execute(
            "INSERT INTO s_content_witnesses(witness_id,object_id,slot,generated_root,wire_root,published_generation,merged_index) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                w.id,
                w.object_id,
                w.slot,
                digest(&w.generated)?.as_slice(),
                digest(&w.wire)?.as_slice(),
                w.first,
                w.merged_index
            ],
        )
        .map_err(err)?;
    }
    for r in &p.births.relations {
        if r.first != p.generation {
            return Err(err("关系出生代错绑"));
        }
        tx.execute(
            "INSERT INTO s_content_relations VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                r.id,
                r.subject,
                r.relation_type,
                r.object,
                digest(&r.generated)?.as_slice(),
                r.first
            ],
        )
        .map_err(err)?;
    }
    for o in &p.births.observations {
        if o.first != p.generation || o.batch.is_some() {
            return Err(err("观察出生代/模板错绑"));
        }
        let wire = replace_batch_placeholder(s, &o.wire, &[f("batch_id")], batch).map_err(err)?;
        sc::verify(s, &wire).map_err(err)?;
        tx.execute(
            "INSERT INTO s_content_observations VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                o.id,
                o.kind,
                o.window_start,
                batch,
                digest(&o.generated)?.as_slice(),
                digest(&wire)?.as_slice(),
                o.first
            ],
        )
        .map_err(err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_set, open_db, read_effective_events, read_raw_events, tb02a,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use rusqlite::Connection;

    // 此夹具只验证真实生成器→辅助索引→SQLite，不声称完整 S3 控制/对象发布已接线。
    fn database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::SCHEMA).unwrap();
        conn.execute_batch(crate::v2::SCHEMA_V2).unwrap();
        meta_set(&conn, "protocol_revision", "s-session/3").unwrap();
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1,'s-session/3','1','test','{}',1,'0')",
            [],
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        content_store::init_schema(&tx).unwrap();
        init_schema(&tx).unwrap();
        tx.commit().unwrap();
        conn
    }
    fn count(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .unwrap()
    }
    fn stored_wires(p: &PublicSets, s: &mut dyn NodeStore) -> Value {
        json!({"witnesses":render(s,&p.witnesses.content_ref()).unwrap(),"relations":render(s,&p.relations.content_ref()).unwrap(),"raw_history":render(s,&p.raw_history.content_ref()).unwrap()})
    }
    #[test]
    fn real_generations_preserve_first_records_and_emit_only_new_auxiliary_members() {
        let files = TestFiles::new();
        init_test_db(&files);
        let rows = tb02b_oracle_rows();
        let mut conn = database();
        let mut runtime = Runtime::default();
        let mut previous = -1;
        let mut prior_ids = Vec::new();
        let mut first_witness = String::new();
        for (g, range) in [(1, 0..8), (2, 8..14), (3, 0..0)] {
            if g == 3 {
                let mut revision = located_ohlc_input(&[("b2", 20, 11, 13, 9, 11)]);
                revision["events"][0]["revision"] = json!("2");
                accept_ohlc(&files, &revision);
            } else {
                accept_ohlc(&files, &located_ohlc_input(&rows[range]));
            }
            let source = open_db(&files.db()).unwrap();
            let raw = read_raw_events(&source).unwrap();
            let effective = read_effective_events(&raw).unwrap();
            let profile_hash = meta_get_opt(&source, "profile_hash").unwrap().unwrap();
            let rule = meta_get_opt(&source, "rule_revision").unwrap().unwrap();
            let catalog = meta_get_opt(&source, "catalog_revision").unwrap().unwrap();
            let scope =
                serde_json::from_str(&meta_get_opt(&source, "scope").unwrap().unwrap()).unwrap();
            let cut = format!("cut-{g}");
            let frontier = raw.len() as i64 - 1;
            let tx = conn.transaction().unwrap();
            let mut s = content_store::TxStore::new(&tx).unwrap();
            let old = load(&tx, &mut s, g - 1).unwrap();
            let mut generator = GenerationRuntime::new(
                format!("aux-test/{}/{g}", files.db().display()),
                tb02a::PROFILE,
                &rule,
            );
            let input = GenerationInput {
                all_events: &raw,
                effective: &effective,
                generation: g,
                structure_cut: &cut,
                input_frontier: frontier,
                profile_id: tb02a::PROFILE,
                profile_hash: &profile_hash,
                rule_revision: &rule,
                catalog_revision: &catalog,
                scope: &scope,
                semantic_commit_ns: None,
                prior: PriorState {
                    bi_records: vec![],
                    version_record: None,
                },
            };
            let generated = generator.generate(&mut s, &input).unwrap();
            assert!(!generated.witnesses.is_empty());
            let p = runtime
                .plan(&mut s, &old, &generated, &[], &raw, previous, frontier, g)
                .unwrap();
            let public = stored_wires(&p.snapshot_values, &mut s);
            assert_eq!(
                public["raw_history"],
                json!(raw
                    .iter()
                    .map(crate::project_raw_event_wire)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap())
            );
            let delta_raw = render(&mut s, &p.delta_values.raw_history.content_ref()).unwrap();
            assert_eq!(
                delta_raw.as_array().unwrap().len(),
                (frontier - previous) as usize
            );
            for w in &generated.witnesses {
                let expected =
                    crate::project_witness_wire(&render(&mut s, &w.built.content_ref()).unwrap())
                        .unwrap();
                let actual = p
                    .snapshot
                    .witnesses
                    .iter()
                    .find(|r| r.id == w.witness_id)
                    .unwrap();
                assert_eq!(render(&mut s, &actual.wire).unwrap(), expected);
            }
            for id in &prior_ids {
                assert!(!p.births.observations.iter().any(|r| &r.id == id));
            }
            if g == 1 {
                first_witness = p.snapshot.witnesses[0].id.clone();
            } else {
                assert_eq!(
                    p.snapshot
                        .witnesses
                        .iter()
                        .find(|w| w.id == first_witness)
                        .unwrap()
                        .first,
                    1
                );
            }
            let batch = format!("batch3-{}", g.to_string().repeat(64));
            stage(&tx, &mut s, &p, &batch).unwrap();
            let stored = load(&tx, &mut s, g).unwrap();
            verify_generated_members(
                &mut s,
                &stored,
                &generated.generated_state,
                &[],
                &raw[..=frontier as usize],
                g,
            )
            .unwrap();
            if g == 1 {
                let first = stored
                    .witnesses
                    .iter()
                    .position(|row| row.first == g)
                    .unwrap();
                let mut missing = stored.clone();
                missing.witnesses.remove(first);
                assert!(verify_generated_members(
                    &mut s,
                    &missing,
                    &generated.generated_state,
                    &[],
                    &raw[..=frontier as usize],
                    g,
                )
                .is_err());

                let mut ghost = stored.clone();
                ghost.witnesses.push(Witness {
                    id: "wit-ghost".into(),
                    ..stored.witnesses[first].clone()
                });
                assert!(verify_generated_members(
                    &mut s,
                    &ghost,
                    &generated.generated_state,
                    &[],
                    &raw[..=frontier as usize],
                    g,
                )
                .is_err());

                let other = stored
                    .witnesses
                    .iter()
                    .enumerate()
                    .find(|(i, row)| *i != first && row.first == g)
                    .map(|(_, row)| row.generated.clone())
                    .unwrap();
                let mut wrong = stored.clone();
                wrong.witnesses[first].generated = other;
                assert!(verify_generated_members(
                    &mut s,
                    &wrong,
                    &generated.generated_state,
                    &[],
                    &raw[..=frontier as usize],
                    g,
                )
                .is_err());
            }
            for (is_delta, expected) in [(false, &p.snapshot), (true, &p.births)] {
                let members = cut_members(&stored, g, is_delta).unwrap();
                assert_eq!(
                    members[0],
                    expected
                        .witnesses
                        .iter()
                        .map(|r| digest(&r.wire).unwrap())
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    members[1],
                    expected
                        .relations
                        .iter()
                        .map(|r| digest(&r.generated).unwrap())
                        .collect::<Vec<_>>()
                );
                let obs = expected
                    .observations
                    .iter()
                    .map(|r| {
                        if r.first == g {
                            replace_batch_placeholder(&mut s, &r.wire, &[f("batch_id")], &batch)
                                .unwrap()
                                .digest()
                                .unwrap()
                        } else {
                            digest(&r.wire).unwrap()
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(members[2], obs);
            }
            if g == 3 {
                let edge = p
                    .births
                    .relations
                    .iter()
                    .find(|r| r.relation_type == "supersedes")
                    .expect("真实原始修订应有首次替代边");
                assert_eq!(
                    edge.subject,
                    format!(
                        "{}@2",
                        raw.last().unwrap()["identity_key"].as_str().unwrap()
                    )
                );
                assert_eq!(
                    edge.object,
                    format!(
                        "{}@1",
                        raw.last().unwrap()["identity_key"].as_str().unwrap()
                    )
                );
            }
            assert_eq!(stored.witnesses.len(), p.snapshot.witnesses.len());
            assert_eq!(stored.relations.len(), p.snapshot.relations.len());
            assert_eq!(stored.observations.len(), p.snapshot.observations.len());
            for o in &stored.observations {
                let mut original = render(&mut s, &o.generated).unwrap();
                original["observation_id"] = json!(o.id);
                original["batch_id"] = json!(o.batch.as_ref().unwrap());
                assert_eq!(
                    render(&mut s, &o.wire).unwrap(),
                    crate::project_observation_wire(&original).unwrap()
                );
            }
            prior_ids = stored.observations.iter().map(|r| r.id.clone()).collect();
            tx.commit().unwrap();
            previous = frontier;
        }
        for old in ["witnesses", "relations", "observations"] {
            assert_eq!(count(&conn, old), 0);
        }
        assert!(count(&conn, "s_content_witnesses") > 0);
    }

    #[test]
    fn observation_detail_remains_a_shared_edge_and_identity_matches_both_old_recipes() {
        let mut conn = database();
        let tx = conn.transaction().unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        let mut rt = Runtime::default();
        for detail in [
            json!({"large":(0..1000).collect::<Vec<_>>()}),
            json!({"tb02a":true,"large":(0..1000).collect::<Vec<_>>()}),
        ] {
            let original = json!({"kind":"insufficient_knowledge","window_start":null,"window_mid":3,"window_end":5,"reason":"fixture","detail":detail});
            let built = write_json(&mut s, &original).unwrap();
            let ob = rt.observation(&mut s, &built, 1).unwrap();
            assert_eq!(ob.id, crate::observation_id(&original));
            assert_eq!(
                field(&mut s, &built.content_ref(), "detail").unwrap(),
                field(&mut s, &ob.wire, "detail").unwrap()
            );
            assert_eq!(
                field(&mut s, &ob.wire, "batch_id").unwrap().kind,
                Kind::TemplateBatch
            );
        }
    }

    #[test]
    fn staged_observation_rolls_back_with_its_content_and_rejects_repeated_insert() {
        let mut conn = database();
        let mut rt = Runtime::default();
        {
            let tx = conn.transaction().unwrap();
            let mut s = content_store::TxStore::new(&tx).unwrap();
            let original = json!({"kind":"insufficient_knowledge","window_start":null,"window_mid":null,"window_end":null,"reason":"fixture","detail":{"large":[1,2,3]}});
            let built = write_json(&mut s, &original).unwrap();
            let row = rt.observation(&mut s, &built, 1).unwrap();
            let index = Index {
                observations: vec![row],
                ..Index::default()
            };
            let a = public_sets(&mut s, &index, &[], &mut rt.sequences[..4]).unwrap();
            let b = public_sets(&mut s, &index, &[], &mut rt.sequences[4..]).unwrap();
            let p = Plan {
                generation: 1,
                snapshot: index.clone(),
                births: index,
                snapshot_values: a,
                delta_values: b,
            };
            let batch = format!("batch3-{}", "1".repeat(64));
            stage(&tx, &mut s, &p, &batch).unwrap();
            assert!(stage(&tx, &mut s, &p, &batch).is_err());
            assert_eq!(load(&tx, &mut s, 1).unwrap().observations.len(), 1);
            tx.rollback().unwrap();
        }
        rt.discard();
        assert_eq!(count(&conn, "s_content_observations"), 0);
        assert_eq!(count(&conn, "s_content_nodes"), 0);
    }

    #[test]
    fn persisted_index_metadata_and_wire_cannot_be_rebound() {
        let mut f = super::super::commit::tests::fixture();
        let tx = f.conn.transaction().unwrap();
        let (m, roots, manifest) = super::super::commit::tests::prepare(&tx, &f.envelope);
        super::super::commit::publish_prepared(tx, &f.token, &m, &roots, &manifest).unwrap();
        let tx = f.conn.transaction().unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        assert_eq!(load(&tx, &mut s, 1).unwrap().observations.len(), 1);
        tx.execute("UPDATE s_content_observations SET kind='wrong'", [])
            .unwrap();
        assert!(load(&tx, &mut s, 1)
            .unwrap_err()
            .contains("观察索引与出生原件不符"));
        tx.rollback().unwrap();
        let tx = f.conn.transaction().unwrap();
        tx.execute("DELETE FROM s_content_nodes WHERE digest=(SELECT generated_root FROM s_content_observations LIMIT 1)",[]).unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        assert!(load(&tx, &mut s, 1).is_err());
        tx.rollback().unwrap();
        let tx = f.conn.transaction().unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        tx.execute(
            "UPDATE s_content_observations SET birth_wire_root=generated_root",
            [],
        )
        .unwrap();
        assert!(load(&tx, &mut s, 1)
            .unwrap_err()
            .contains("观察索引与出生原件不符"));
    }
}
