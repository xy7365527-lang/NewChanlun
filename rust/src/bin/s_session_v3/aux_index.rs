//! #1394 C §5.4：见证、关系、首次观察的独立内容索引。
//! plan 先产生含本批占位的值根；manifest 求出 batch3 后 stage 同事务暂写出生记录。
//! 观察 detail 与来源数组保留为边；仅单条原始事件沿既有投影转换。索引不替代来源业务审计。
use super::business_json::{self as bj, RawEventsView};
use super::generation::{Generated, SequenceMemo, WitnessRecord};
use crate::v3::audit_store::{CodecRead, CodecStore, RefValue};
use crate::{content_store, meta_get_opt};
use aj::JsonView;
use bj::TextValue;
use newchan_rust::s_content::{
    self as sc, path::PathStep, Built, ContentRef, Kind, NodeDigest, NodeStore,
};
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::{params, Transaction};
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use sc::{CodecVec, ValueView as VV};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
pub(crate) trait AuxRef {
    fn view(&self) -> sc::RefView<'_>;
}
impl AuxRef for ContentRef {
    fn view(&self) -> sc::RefView<'_> {
        self.into()
    }
}
impl AuxRef for RefValue {
    fn view(&self) -> sc::RefView<'_> {
        self.view()
    }
}
fn field(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_ref(r.view(), &[f(k)]).map_err(err)
}
fn digest(r: &(impl AuxRef + ?Sized)) -> Result<NodeDigest, String> {
    r.view().digest().map_err(err)
}
fn scalar(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<bj::JsonOwner, String> {
    let leaf = field(s, r, k)?;
    if !matches!(
        leaf.kind(),
        Kind::Null | Kind::Bool | Kind::Number | Kind::String
    ) {
        return Err(err(format!("{k}不是标量")));
    }
    render(s, &leaf)
}
fn text(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<TextValue, String> {
    let value = scalar(s, r, k)?;
    let value = value
        .view()
        .as_str()
        .ok_or_else(|| err(format!("{k}不是文本")))?;
    TextValue::from_str(value, s.allocation_budget().as_ref()).map_err(err)
}
fn integer(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<i64, String> {
    scalar(s, r, k)?
        .view()
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是i64")))
}
fn render(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<bj::JsonOwner, String> {
    bj::render_json(s, r.view())
}
#[cfg(test)]
fn render_plain(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<Value, String> {
    let value = render(s, r)?;
    serde_json::to_value(value.view()).map_err(err)
}
#[cfg(test)]
fn write_json(s: &mut (impl CodecStore + ?Sized), v: &Value) -> Result<Built, String> {
    s.write_json_view(v.into()).map_err(err)
}
fn write_fields(
    s: &mut (impl CodecStore + ?Sized),
    fields: &[(&str, JsonView<'_>)],
) -> Result<Built, String> {
    s.write_json_view(JsonView::object_fields(fields).map_err(err)?)
        .map_err(err)
}
fn edge(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<VV<'static>, String> {
    Ok(VV::Edge(digest(&field(s, r, k)?)?))
}
fn array_elements(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<CodecVec<RefValue>, String> {
    if r.view().kind() != Kind::Array {
        return Err(err("预期数组"));
    }
    let mut out = bj::new_work(s.allocation_budget().as_ref());
    for i in 0..r.view().extent().ok_or_else(|| err("数组缺长度"))? {
        out.try_push(s.lookup_ref(r.view(), &[PathStep::Index(i)]).map_err(err)?)
            .map_err(err)?;
    }
    Ok(out)
}
fn root(s: &mut (impl CodecRead + ?Sized), b: &[u8]) -> Result<RefValue, String> {
    let d: NodeDigest = b.try_into().map_err(|_| err("根不是32字节"))?;
    let r = s
        .content_ref_view(sc::FixedRoot::new(&d).as_str())
        .map_err(err)?;
    s.verify_many_ref_fresh([r.view()], sc::Mode::Public)
        .map_err(err)?;
    Ok(r)
}
fn built_ref(s: &(impl CodecRead + ?Sized), b: &Built) -> RefValue {
    #[cfg(feature = "s_allocation_json_testonly")]
    if s.allocation_budget().is_some() {
        return RefValue::Owned(b.owned_ref());
    }
    RefValue::Plain(b.content_ref())
}
fn exact(r: &(impl AuxRef + ?Sized), n: u64) -> Result<(), String> {
    if r.view().kind() != Kind::Object || r.view().extent() != Some(n) {
        Err(err(format!("预期{n}字段对象")))
    } else {
        Ok(())
    }
}
fn fmt_text(
    args: std::fmt::Arguments<'_>,
    budget: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    use std::fmt::Write;
    struct Out {
        bytes: CodecVec<u8>,
        error: Option<sc::CodecError>,
    }
    impl std::fmt::Write for Out {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            #[cfg(feature = "s_allocation_json_testonly")]
            {
                aj::extend_bytes(&mut self.bytes, value.as_bytes()).map_err(|error| {
                    self.error = Some(error);
                    std::fmt::Error
                })
            }
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            {
                for &byte in value.as_bytes() {
                    self.bytes.try_push(byte).map_err(|error| {
                        self.error = Some(error);
                        std::fmt::Error
                    })?;
                }
                Ok(())
            }
        }
    }
    let mut out = Out {
        bytes: bj::new_work(budget),
        error: None,
    };
    let result = out.write_fmt(args);
    if let Some(error) = out.error {
        return Err(err(error));
    }
    result.map_err(err)?;
    TextValue::from_str(std::str::from_utf8(&out.bytes).map_err(err)?, budget).map_err(err)
}
fn stable_sort<T>(
    rows: &mut CodecVec<T>,
    compare: impl Fn(&T, &T) -> std::cmp::Ordering,
    budget: Option<&OwnedBudget>,
) -> Result<(), String> {
    let mut order = bj::new_work(budget);
    let mut target = bj::new_work(budget);
    for i in 0..rows.len() {
        order.try_push(i).map_err(err)?;
        target.try_push(0usize).map_err(err)?;
    }
    order.sort_unstable_by(|a, b| compare(&rows[*a], &rows[*b]).then(a.cmp(b)));
    for (new, old) in order.iter().enumerate() {
        target[*old] = new;
    }
    for i in 0..rows.len() {
        while target[i] != i {
            let j = target[i];
            rows.swap(i, j);
            target.swap(i, j);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct Witness {
    pub id: TextValue,
    pub object_id: TextValue,
    pub slot: i64,
    pub merged_index: i64,
    pub generated: RefValue,
    pub wire: RefValue,
    pub first: i64,
}
impl Witness {
    fn try_clone(&self) -> Result<Self, String> {
        Ok(Self {
            id: self.id.try_clone()?,
            object_id: self.object_id.try_clone()?,
            slot: self.slot,
            merged_index: self.merged_index,
            generated: self.generated.clone(),
            wire: self.wire.clone(),
            first: self.first,
        })
    }
}
#[derive(Debug)]
pub(super) struct Relation {
    pub id: TextValue,
    pub subject: TextValue,
    pub relation_type: TextValue,
    pub object: TextValue,
    pub generated: RefValue,
    pub first: i64,
}
impl Relation {
    fn try_clone(&self) -> Result<Self, String> {
        Ok(Self {
            id: self.id.try_clone()?,
            subject: self.subject.try_clone()?,
            relation_type: self.relation_type.try_clone()?,
            object: self.object.try_clone()?,
            generated: self.generated.clone(),
            first: self.first,
        })
    }
}
#[derive(Debug)]
pub(super) struct Observation {
    pub id: TextValue,
    pub kind: TextValue,
    pub window_start: Option<i64>,
    pub generated: RefValue,
    pub wire: RefValue,
    pub batch: Option<TextValue>,
    pub first: i64,
}
impl Observation {
    fn try_clone(&self) -> Result<Self, String> {
        Ok(Self {
            id: self.id.try_clone()?,
            kind: self.kind.try_clone()?,
            window_start: self.window_start,
            generated: self.generated.clone(),
            wire: self.wire.clone(),
            batch: self.batch.as_ref().map(TextValue::try_clone).transpose()?,
            first: self.first,
        })
    }
}
#[derive(Default)]
pub(super) struct Index {
    pub witnesses: CodecVec<Witness>,
    pub relations: CodecVec<Relation>,
    pub observations: CodecVec<Observation>,
    budget: Option<OwnedBudget>,
}
impl Index {
    fn new(budget: Option<OwnedBudget>) -> Self {
        Self {
            witnesses: bj::new_work(budget.as_ref()),
            relations: bj::new_work(budget.as_ref()),
            observations: bj::new_work(budget.as_ref()),
            budget,
        }
    }
    pub(super) fn try_clone(&self) -> Result<Self, String> {
        self.clone_in(self.budget.clone())
    }
    fn clone_in(&self, budget: Option<OwnedBudget>) -> Result<Self, String> {
        let retained = self.witnesses.capacity() != 0
            || self.relations.capacity() != 0
            || self.observations.capacity() != 0;
        if retained && !same_budget(self.budget.as_ref(), budget.as_ref()) {
            return Err(err(
                "prototype_audit_allocation_incomplete：索引账本改变，须重新load",
            ));
        }
        let mut out = Self::new(budget);
        for row in &self.witnesses {
            out.witnesses.try_push(row.try_clone()?).map_err(err)?;
        }
        for row in &self.relations {
            out.relations.try_push(row.try_clone()?).map_err(err)?;
        }
        for row in &self.observations {
            out.observations.try_push(row.try_clone()?).map_err(err)?;
        }
        Ok(out)
    }
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Index")
            .field("witnesses", &self.witnesses)
            .field("relations", &self.relations)
            .field("observations", &self.observations)
            .finish()
    }
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

fn same_budget(a: Option<&OwnedBudget>, b: Option<&OwnedBudget>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_ledger(b),
        _ => false,
    }
}
struct Cache<K, V> {
    map: Option<aj::BusinessMap<K, V>>,
    budget: Option<OwnedBudget>,
}
impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self {
            map: None,
            budget: None,
        }
    }
}
fn work_map<K: Eq + std::hash::Hash, V>(budget: Option<&OwnedBudget>) -> aj::BusinessMap<K, V> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return aj::BusinessMap::owned(b);
    }
    let _ = budget;
    aj::BusinessMap::plain()
}
impl<K: Eq + std::hash::Hash, V> Cache<K, V> {
    fn check(&self, budget: Option<&OwnedBudget>) -> Result<(), String> {
        if self.map.is_some() && !same_budget(self.budget.as_ref(), budget) {
            return Err(err(
                "prototype_audit_allocation_incomplete：缓存账本改变，须discard",
            ));
        }
        Ok(())
    }
    fn get(&self, key: &K, budget: Option<&OwnedBudget>) -> Result<Option<&V>, String> {
        self.check(budget)?;
        Ok(self.map.as_ref().and_then(|m| m.get(key)))
    }
    fn insert(&mut self, key: K, value: V, budget: Option<&OwnedBudget>) -> Result<(), String> {
        self.check(budget)?;
        let map = self.map.get_or_insert_with(|| work_map(budget));
        self.budget = budget.cloned();
        map.insert(key, value).map_err(err)?;
        Ok(())
    }
}
/// 缓存只按真实内容根命中；实际owner/容器由store选同账，回滚/换库仍必须discard。
#[derive(Default)]
pub(super) struct Runtime {
    witness_wires: Cache<(NodeDigest, i64), RefValue>,
    bar_wires: Cache<NodeDigest, RefValue>,
    observation_ids: Cache<NodeDigest, TextValue>,
    raw_wires: Cache<NodeDigest, NodeDigest>,
    sequences: [SequenceMemo; 8],
}
impl Runtime {
    pub(super) fn discard(&mut self) {
        *self = Self::default();
    }

    fn witness(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        w: &WitnessRecord,
        merged_index: i64,
        g: i64,
    ) -> Result<Witness, String> {
        let generated = built_ref(s, &w.built);
        s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
            .map_err(err)?;
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
        let wire = match self.witness_wires.get(
            &(w.built.digest, merged_index),
            s.allocation_budget().as_ref(),
        )? {
            Some(r) => r.clone(),
            None => {
                verify_witness_id(s, &generated, &id, &object_id, slot, merged_index)?;
                let r = self.witness_wire(s, &generated)?;
                self.witness_wires.insert(
                    (w.built.digest, merged_index),
                    r.clone(),
                    s.allocation_budget().as_ref(),
                )?;
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
        s: &mut (impl CodecStore + ?Sized),
        r: &(impl AuxRef + ?Sized),
    ) -> Result<RefValue, String> {
        let bars = field(s, r, "raw_bars")?;
        let owned_budget = s.allocation_budget();
        let mut values = bj::new_work(owned_budget.as_ref());
        for bar in array_elements(s, &bars)? {
            let d = digest(&bar)?;
            let projected =
                if let Some(cached) = self.bar_wires.get(&d, s.allocation_budget().as_ref())? {
                    cached.clone()
                } else {
                    let original = render(s, &bar)?;
                    let built = project_bar(s, original.view())?;
                    let r = built_ref(s, &built);
                    self.bar_wires.insert(d, r.clone(), owned_budget.as_ref())?;
                    r
                };
            values
                .try_push(VV::Edge(digest(&projected)?))
                .map_err(err)?;
        }
        let witness_id = edge(s, r, "witness_id")?;
        let object_id = edge(s, r, "object_id")?;
        let high = edge(s, r, "merged_high")?;
        let low = edge(s, r, "merged_low")?;
        let open = edge(s, r, "merged_open")?;
        let close = edge(s, r, "merged_close")?;
        let slot = bj::Decimal::new(integer(s, r, "slot")?);
        let source = bj::Decimal::new(integer(s, r, "merged_source_index")?);
        let fields = [
            ("witness_id", witness_id),
            ("object_id", object_id),
            ("merged_high", high),
            ("merged_low", low),
            ("merged_open", open),
            ("merged_close", close),
            ("slot", VV::String(slot.text())),
            ("merged_source_index", VV::String(source.text())),
            ("raw_bars", VV::Array(&values)),
        ];
        let b = s.write_value_view(VV::Object(&fields)).map_err(err)?;
        Ok(built_ref(s, &b))
    }
    fn observation(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        b: &Built,
        g: i64,
    ) -> Result<Observation, String> {
        let generated = built_ref(s, b);
        s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
            .map_err(err)?;
        exact(&generated, 6)?;
        let kind = text(s, &generated, "kind")?;
        let window_start = window(s, &generated, "window_start")?;
        let id = if let Some(id) = self
            .observation_ids
            .get(&b.digest, s.allocation_budget().as_ref())?
        {
            id.try_clone()?
        } else {
            let id = observation_id(s, &generated)?;
            self.observation_ids.insert(
                b.digest,
                id.try_clone()?,
                s.allocation_budget().as_ref(),
            )?;
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

    pub(super) fn plan<'a>(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        old: &Index,
        generated: &Generated<'_>,
        replaces: impl Into<RawEventsView<'a>>,
        raw: impl Into<RawEventsView<'a>>,
        previous_frontier: i64,
        frontier: i64,
        g: i64,
    ) -> Result<Plan, String> {
        let raw = raw.into();
        let replaces = replaces.into();
        if g < 1
            || previous_frontier < -1
            || frontier <= previous_frontier
            || frontier as usize >= raw.len()
        {
            return Err(err("代号/原始前沿非法"));
        }
        let owned_budget = s.allocation_budget();
        let mut snapshot = old.clone_in(owned_budget.clone())?;
        let mut births = Index::new(owned_budget.clone());
        let mut ws = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.witnesses.iter().enumerate() {
            ws.insert(row.id.try_clone()?, i).map_err(err)?;
        }
        for w in &generated.witnesses {
            let object = generated
                .objects
                .iter()
                .find(|o| o.object_id() == w.object_id)
                .ok_or_else(|| err("见证对象不在本代生成态"))?;
            let generated_object = built_ref(s, object.generated());
            let group = s
                .lookup_ref(
                    generated_object.view(),
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
                ws.insert(row.id.try_clone()?, snapshot.witnesses.len())
                    .map_err(err)?;
                snapshot.witnesses.try_push(row.try_clone()?).map_err(err)?;
                births.witnesses.try_push(row).map_err(err)?;
            }
        }
        let mut rs = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.relations.iter().enumerate() {
            rs.insert(row.id.try_clone()?, i).map_err(err)?;
        }
        let mut candidates = bj::new_work(owned_budget.as_ref());
        for b in &generated.relations {
            candidates.try_push(built_ref(s, b)).map_err(err)?;
        }
        for r in replaces {
            candidates
                .try_push(replacement_relation(s, r)?)
                .map_err(err)?;
        }
        for event in raw.prefix(frontier as usize + 1)? {
            if let Some(r) = supersedes_relation(s, event)? {
                candidates.try_push(r).map_err(err)?;
            }
        }
        for r in candidates {
            let row = relation(s, r, g)?;
            if let Some(i) = rs.get(&row.id) {
                if snapshot.relations[*i].generated != row.generated {
                    return Err(err("同关系身份换内容"));
                }
            } else {
                rs.insert(row.id.try_clone()?, snapshot.relations.len())
                    .map_err(err)?;
                snapshot.relations.try_push(row.try_clone()?).map_err(err)?;
                births.relations.try_push(row).map_err(err)?;
            }
        }
        let mut obs = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.observations.iter().enumerate() {
            obs.insert(row.id.try_clone()?, i).map_err(err)?;
        }
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
                obs.insert(row.id.try_clone()?, snapshot.observations.len())
                    .map_err(err)?;
                snapshot
                    .observations
                    .try_push(row.try_clone()?)
                    .map_err(err)?;
                births.observations.try_push(row).map_err(err)?;
            }
        }
        stable_sort(
            &mut snapshot.witnesses,
            |a, b| (&a.object_id, a.slot).cmp(&(&b.object_id, b.slot)),
            owned_budget.as_ref(),
        )?;
        stable_sort(
            &mut snapshot.relations,
            |a, b| {
                (&a.subject, &a.relation_type, &a.object).cmp(&(
                    &b.subject,
                    &b.relation_type,
                    &b.object,
                ))
            },
            owned_budget.as_ref(),
        )?;
        stable_sort(
            &mut snapshot.observations,
            |a, b| (&a.kind, a.window_start).cmp(&(&b.kind, b.window_start)),
            owned_budget.as_ref(),
        )?;
        let mut raw_nodes = bj::new_work(owned_budget.as_ref());
        for (n, event) in raw.prefix(frontier as usize + 1)?.iter().enumerate() {
            if bj::field(event, "seq").as_i64() != Some(n as i64) {
                return Err(err("原始序号与输入前缀不符"));
            }
            let key: NodeDigest = bj::sha_json(event, raw.budget())?;
            let wire = if let Some(d) = self.raw_wires.get(&key, s.allocation_budget().as_ref())? {
                *d
            } else {
                let d = s
                    .write_json_view(bj::RawWire::new(event)?.view())
                    .map_err(err)?
                    .digest;
                self.raw_wires
                    .insert(key, d, s.allocation_budget().as_ref())?;
                d
            };
            raw_nodes.try_push(wire).map_err(err)?;
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

fn project_bar(s: &mut (impl CodecStore + ?Sized), bar: JsonView<'_>) -> Result<Built, String> {
    let object = bar.as_object().ok_or("raw_bars 成员必须是对象")?;
    let number = |v: Option<JsonView<'_>>| {
        v.and_then(JsonView::as_i64)
            .map(bj::Decimal::new)
            .ok_or_else(|| "持久整数必须是 i64，不能是浮点、布尔、空值或文本".to_string())
    };
    let seq = number(object.get("seq"))?;
    let revision = object
        .get("revision")
        .map(|v| number(Some(v)))
        .transpose()?;
    let supersedes = match object.get("supersedes_revision") {
        Some(v) if v.as_i64().is_some() => Some(bj::Decimal::new(v.as_i64().unwrap())),
        Some(v) if !v.is_null() => {
            return Err("raw_bars.supersedes_revision 必须是 i64 或 null".into())
        }
        _ => None,
    };
    let ohlc =
        object.get("schema_revision").and_then(JsonView::as_str) == Some(crate::tb02a::RAW_SCHEMA);
    let mut fields = bj::new_work(s.allocation_budget().as_ref());
    for (key, value) in object.iter() {
        if ohlc && key == "price" {
            continue;
        }
        let value = match key {
            "seq" => JsonView::String(seq.text()),
            "revision" => JsonView::String(revision.as_ref().unwrap().text()),
            "supersedes_revision" => supersedes
                .as_ref()
                .map(|v| JsonView::String(v.text()))
                .unwrap_or(JsonView::Null),
            _ => value,
        };
        fields.try_push((key, value)).map_err(err)?;
    }
    let extra = if ohlc {
        &["schema_revision", "open", "high", "low", "close"][..]
    } else {
        &[]
    };
    for key in ["seq"].into_iter().chain(extra.iter().copied()) {
        if object.get(key).is_none() {
            fields
                .try_push((
                    key,
                    if key == "seq" {
                        JsonView::String(seq.text())
                    } else {
                        JsonView::Null
                    },
                ))
                .map_err(err)?;
        }
    }
    write_fields(s, &fields)
}
fn replacement_relation(
    s: &mut (impl CodecStore + ?Sized),
    replacement: JsonView<'_>,
) -> Result<RefValue, String> {
    let subject = bj::field(replacement, "new_object_id")
        .as_str()
        .ok_or_else(|| err("replaces缺new_object_id"))?;
    let object = bj::field(replacement, "old_object_id")
        .as_str()
        .ok_or_else(|| err("replaces缺old_object_id"))?;
    let fields = [
        ("subject", JsonView::String(subject)),
        ("relation_type", JsonView::String("replaces")),
        ("object", JsonView::String(object)),
    ];
    let b = write_fields(s, &fields)?;
    Ok(built_ref(s, &b))
}
fn supersedes_relation(
    s: &mut (impl CodecStore + ?Sized),
    event: JsonView<'_>,
) -> Result<Option<RefValue>, String> {
    let Some(sup) = bj::field(event, "supersedes_revision").as_i64() else {
        return Ok(None);
    };
    let identity = bj::field(event, "identity_key")
        .as_str()
        .ok_or_else(|| err("源身份缺失"))?;
    let rev = bj::field(event, "revision")
        .as_i64()
        .ok_or_else(|| err("源版本缺失"))?;
    let budget = s.allocation_budget();
    let subject = fmt_text(format_args!("{identity}@{rev}"), budget.as_ref())?;
    let object = fmt_text(format_args!("{identity}@{sup}"), budget.as_ref())?;
    let fields = [
        ("subject", JsonView::String(&subject)),
        ("relation_type", JsonView::String("supersedes")),
        ("object", JsonView::String(&object)),
    ];
    let b = write_fields(s, &fields)?;
    Ok(Some(built_ref(s, &b)))
}
fn public_sets(
    s: &mut (impl CodecStore + ?Sized),
    i: &Index,
    raw: &[NodeDigest],
    memo: &mut [SequenceMemo],
) -> Result<PublicSets, String> {
    let budget = s.allocation_budget();
    let mut w = bj::new_work(budget.as_ref());
    let mut r = bj::new_work(budget.as_ref());
    let mut o = bj::new_work(budget.as_ref());
    for row in &i.witnesses {
        w.try_push(digest(&row.wire)?).map_err(err)?;
    }
    for row in &i.relations {
        r.try_push(digest(&row.generated)?).map_err(err)?;
    }
    for row in &i.observations {
        o.try_push(digest(&row.wire)?).map_err(err)?;
    }
    Ok(PublicSets {
        witnesses: memo[0].sync(s.generic_store(), &w, "见证快照")?,
        relations: memo[1].sync(s.generic_store(), &r, "关系快照")?,
        observations: memo[2].sync(s.generic_store(), &o, "观察快照")?,
        raw_history: memo[3].sync(s.generic_store(), raw, "原始历史")?,
    })
}
fn window(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<Option<i64>, String> {
    let v = scalar(s, r, k)?;
    if v.view().is_null() {
        Ok(None)
    } else {
        v.view()
            .as_i64()
            .map(Some)
            .ok_or_else(|| err("窗口不是i64/null"))
    }
}
fn observation_wire(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
    id: &str,
    batch: Option<&str>,
) -> Result<RefValue, String> {
    let kind = edge(s, r, "kind")?;
    let reason = edge(s, r, "reason")?;
    let detail = edge(s, r, "detail")?;
    let start = window(s, r, "window_start")?.map(bj::Decimal::new);
    let mid = window(s, r, "window_mid")?.map(bj::Decimal::new);
    let end = window(s, r, "window_end")?.map(bj::Decimal::new);
    fn number(v: &Option<bj::Decimal>) -> VV<'_> {
        v.as_ref()
            .map(|v| VV::String(v.text()))
            .unwrap_or(VV::Json(JsonView::Null))
    }
    let fields = [
        ("observation_id", VV::String(id)),
        ("batch_id", batch.map(VV::String).unwrap_or(VV::BatchSelf)),
        ("kind", kind),
        ("reason", reason),
        ("detail", detail),
        ("window_start", number(&start)),
        ("window_mid", number(&mid)),
        ("window_end", number(&end)),
    ];
    let b = s.write_template_view(VV::Object(&fields)).map_err(err)?;
    Ok(built_ref(s, &b))
}
fn relation(
    s: &mut (impl CodecStore + ?Sized),
    generated: RefValue,
    g: i64,
) -> Result<Relation, String> {
    s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
        .map_err(err)?;
    exact(&generated, 3)?;
    let subject = text(s, &generated, "subject")?;
    let relation_type = text(s, &generated, "relation_type")?;
    let object = text(s, &generated, "object")?;
    let fields = [
        ("s", JsonView::String(&subject)),
        ("t", JsonView::String(&relation_type)),
        ("o", JsonView::String(&object)),
    ];
    let hash = bj::sha_json(
        JsonView::object_fields(&fields).map_err(err)?,
        s.allocation_budget().as_ref(),
    )?;
    let hash = sc::FixedRoot::new(&hash);
    let id = fmt_text(
        format_args!("rel-{}", &hash.as_str()[4..20]),
        s.allocation_budget().as_ref(),
    )?;
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
fn stream_hash(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<TextValue, String> {
    let mut out = HashWriter(Sha256::new());
    s.render_ref(r.view(), &mut out).map_err(err)?;
    let hash = sc::FixedRoot::new(&out.0.finalize().into());
    TextValue::from_str(&hash.as_str()[4..], s.allocation_budget().as_ref()).map_err(err)
}
fn verify_witness_id(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
    id: &str,
    object: &str,
    slot: i64,
    mi: i64,
) -> Result<(), String> {
    if mi < 0 {
        return Err(err("见证合并序号非法"));
    }
    let mi = serde_json::Number::from(mi);
    let slot = serde_json::Number::from(slot);
    let fields = [
        ("mi", VV::Json(JsonView::Number(&mi))),
        ("object_id", VV::String(object)),
        ("slot", VV::Json(JsonView::Number(&slot))),
        ("raw_bars", edge(s, r, "raw_bars")?),
    ];
    let b = s.write_value_view(VV::Object(&fields)).map_err(err)?;
    let recipe = built_ref(s, &b);
    let hash = stream_hash(s, &recipe)?;
    let expected = fmt_text(
        format_args!("wit-{}", &hash[..16]),
        s.allocation_budget().as_ref(),
    )?;
    if id != expected.as_str() {
        return Err(err("见证身份配方不符"));
    }
    Ok(())
}
fn observation_identity_value(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<RefValue, String> {
    let detail = field(s, r, "detail")?;
    if detail.kind() != Kind::Object {
        return Err(err("观察detail不是对象"));
    }
    let tb = match s.lookup_ref(detail.view(), &[f("tb02a")]) {
        Ok(value) => {
            value.kind() == Kind::Bool && render(s, &value)?.view().as_bool() == Some(true)
        }
        Err(e) if e.code() == sc::path::path_code::PATH_FIELD_MISSING => false,
        Err(e) => return Err(err(e)),
    };
    if tb {
        return Ok(built_ref(
            s,
            &Built {
                digest: digest(r)?,
                kind: r.view().kind(),
                extent: r.view().extent(),
            },
        ));
    }
    let fields = [
        ("kind", edge(s, r, "kind")?),
        ("reason", edge(s, r, "reason")?),
        ("window_start", edge(s, r, "window_start")?),
        ("window_mid", edge(s, r, "window_mid")?),
        ("window_end", edge(s, r, "window_end")?),
    ];
    let b = s.write_value_view(VV::Object(&fields)).map_err(err)?;
    Ok(built_ref(s, &b))
}
fn observation_id(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<TextValue, String> {
    let recipe = observation_identity_value(s, r)?;
    let hash = stream_hash(s, &recipe)?;
    let full = recipe.view().root() == r.view().root()
        && recipe.view().kind() == r.view().kind()
        && recipe.view().extent() == r.view().extent();
    fmt_text(
        format_args!("obs-{}", if full { hash.as_str() } else { &hash[..16] }),
        s.allocation_budget().as_ref(),
    )
}

fn state_members(
    s: &mut (impl CodecRead + ?Sized),
    state: &(impl AuxRef + ?Sized),
    name: &str,
) -> Result<CodecVec<RefValue>, String> {
    let members = field(s, state, name)?;
    array_elements(s, &members)
}

/// 生成态与见证/关系/观察独立索引的来源绑定核验。
///
/// 这里只消费调用方已核过的原始前缀；不重跑结构分类，也不展开观察 detail 或 raw_bars
/// 的整段 JSON。生成态当前成员必须能在独立索引找到并逐根相等；本代首次成员则另核
/// 独立索引的首次插入顺序，避免只凭索引自洽放过出生漏项、幽灵行或错绑原件。
pub(super) fn verify_generated_members<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &Index,
    state: &(impl AuxRef + ?Sized),
    replaces: impl Into<RawEventsView<'a>>,
    raw: impl Into<RawEventsView<'a>>,
    g: i64,
) -> Result<(), String> {
    let raw = raw.into();
    let replaces = replaces.into();
    let owned_budget = s.allocation_budget();
    if g < 1 {
        return Err(err("生成代非法"));
    }

    let mut witness_by_id = work_map::<&str, &Witness>(owned_budget.as_ref());
    for row in &index.witnesses {
        if row.first <= g {
            if witness_by_id
                .insert(row.id.as_str(), row)
                .map_err(err)?
                .is_some()
            {
                return Err(err("见证独立索引重复身份"));
            }
        }
    }
    let generated_witnesses = state_members(s, state, "witnesses")?;
    let mut seen = work_map(owned_budget.as_ref());
    let mut generated_births = bj::new_work(owned_budget.as_ref());
    for generated in generated_witnesses {
        let id = text(s, &generated, "witness_id")?;
        let generated_digest = digest(&generated)?;
        if seen
            .insert(id.try_clone()?, generated_digest)
            .map_err(err)?
            .is_some()
        {
            return Err(err("生成见证重复身份"));
        }
        let row = witness_by_id
            .get(&id.as_str())
            .ok_or_else(|| err("生成见证不在独立索引内"))?;
        if row.generated != generated {
            return Err(err("生成见证原件与独立索引不一致"));
        }
        if row.first == g {
            generated_births.try_push(id).map_err(err)?;
        }
    }
    let mut indexed_births = bj::new_work(owned_budget.as_ref());
    for row in index.witnesses.iter().filter(|row| row.first == g) {
        indexed_births.try_push(row.id.as_str()).map_err(err)?;
    }
    if generated_births.len() != indexed_births.len()
        || generated_births
            .iter()
            .zip(&indexed_births)
            .any(|(a, b)| a.as_str() != *b)
    {
        return Err(err("见证出生集合或顺序与独立索引不同"));
    }

    let mut relation_by_id = work_map::<&str, &Relation>(owned_budget.as_ref());
    for row in &index.relations {
        if row.first <= g {
            if relation_by_id
                .insert(row.id.as_str(), row)
                .map_err(err)?
                .is_some()
            {
                return Err(err("关系独立索引重复身份"));
            }
        }
    }
    let mut relation_candidates = state_members(s, state, "relations")?;
    for replacement in replaces {
        relation_candidates
            .try_push(replacement_relation(s, replacement)?)
            .map_err(err)?;
    }
    for event in raw {
        if let Some(r) = supersedes_relation(s, event)? {
            relation_candidates.try_push(r).map_err(err)?;
        }
    }
    let mut relation_seen = work_map(owned_budget.as_ref());
    let mut relation_births = bj::new_work(owned_budget.as_ref());
    for candidate in relation_candidates {
        let row = relation(s, candidate, g)?;
        let row_digest = digest(&row.generated)?;
        if let Some(previous) = relation_seen
            .insert(row.id.try_clone()?, row_digest)
            .map_err(err)?
        {
            if previous != row_digest {
                return Err(err("同关系身份对应多个生成原件"));
            }
            continue;
        }
        let stored = relation_by_id
            .get(&row.id.as_str())
            .ok_or_else(|| err("生成关系不在独立索引内"))?;
        if stored.generated != row.generated {
            return Err(err("生成关系原件与独立索引不一致"));
        }
        if stored.first == g {
            relation_births.try_push(row.id).map_err(err)?;
        }
    }
    let mut indexed_relation_births = bj::new_work(owned_budget.as_ref());
    for row in index.relations.iter().filter(|row| row.first == g) {
        indexed_relation_births
            .try_push(row.id.as_str())
            .map_err(err)?;
    }
    if relation_births.len() != indexed_relation_births.len()
        || relation_births
            .iter()
            .zip(&indexed_relation_births)
            .any(|(a, b)| a.as_str() != *b)
    {
        return Err(err("关系出生集合或顺序与独立索引不同"));
    }

    let mut observation_by_id = work_map::<&str, &Observation>(owned_budget.as_ref());
    for row in &index.observations {
        if row.first <= g {
            if observation_by_id
                .insert(row.id.as_str(), row)
                .map_err(err)?
                .is_some()
            {
                return Err(err("观察独立索引重复身份"));
            }
        }
    }
    let generated_observations = state_members(s, state, "observations")?;
    let mut observation_seen = work_map(owned_budget.as_ref());
    let mut observation_births = bj::new_work(owned_budget.as_ref());
    for generated in generated_observations {
        let id = observation_id(s, &generated)?;
        if observation_seen
            .insert(id.try_clone()?, ())
            .map_err(err)?
            .is_some()
        {
            return Err(err("生成观察重复身份"));
        }
        let stored = observation_by_id
            .get(&id.as_str())
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
            observation_births.try_push(id).map_err(err)?;
        }
    }
    let mut indexed_observation_births = bj::new_work(owned_budget.as_ref());
    for row in index.observations.iter().filter(|row| row.first == g) {
        indexed_observation_births
            .try_push(row.id.as_str())
            .map_err(err)?;
    }
    if observation_births.len() != indexed_observation_births.len()
        || observation_births
            .iter()
            .zip(&indexed_observation_births)
            .any(|(a, b)| a.as_str() != *b)
    {
        return Err(err("观察出生集合或顺序与独立索引不同"));
    }
    Ok(())
}

/// 读取独立记录并验证闭包、身份、公开投影和小元数据。store 可为只读源+内存重算层。
/// 完整生成来源/跨代成员审计仍由上层完成，索引自洽不能证明来源真实。
pub(super) fn load(
    tx: &Transaction<'_>,
    s: &mut (impl CodecStore + ?Sized),
    latest: i64,
) -> Result<Index, String> {
    let owned_budget = s.allocation_budget();
    let mut out = Index::new(owned_budget.clone());
    let mut rt = Runtime::default();
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT witness_id,object_id,slot,generated_root,wire_root,published_generation,merged_index FROM s_content_witnesses ORDER BY rowid").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let id = TextValue::from_sql(&mut row, 0, owned_budget.as_ref()).map_err(err)?;
        let object_id = TextValue::from_sql(&mut row, 1, owned_budget.as_ref()).map_err(err)?;
        let slot: i64 = row.integer(2).map_err(err)?;
        let merged_index: i64 = row.integer(6).map_err(err)?;
        let generated = root(s, row.blob(3).map_err(err)?)?;
        let wire = root(s, row.blob(4).map_err(err)?)?;
        let first: i64 = row.integer(5).map_err(err)?;
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
        out.witnesses
            .try_push(Witness {
                id,
                object_id,
                slot,
                merged_index,
                generated,
                wire,
                first,
            })
            .map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT relation_id,subject,relation_type,object,generated_root,published_generation FROM s_content_relations ORDER BY rowid").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let first: i64 = row.integer(5).map_err(err)?;
        check_generation(first, latest)?;
        let generated = root(s, row.blob(4).map_err(err)?)?;
        let r = relation(s, generated, first)?;
        if r.id != row.text(0).map_err(err)?
            || r.subject != row.text(1).map_err(err)?
            || r.relation_type != row.text(2).map_err(err)?
            || r.object != row.text(3).map_err(err)?
        {
            return Err(err("关系索引与原件不符"));
        }
        out.relations.try_push(r).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT observation_id,kind,window_start,batch_id,generated_root,birth_wire_root,published_generation FROM s_content_observations ORDER BY rowid").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let id = TextValue::from_sql(&mut row, 0, owned_budget.as_ref()).map_err(err)?;
        let kind = TextValue::from_sql(&mut row, 1, owned_budget.as_ref()).map_err(err)?;
        let window_start: Option<i64> = row.optional_integer(2).map_err(err)?;
        let batch = TextValue::from_sql(&mut row, 3, owned_budget.as_ref()).map_err(err)?;
        check_batch(&batch)?;
        let generated = root(s, row.blob(4).map_err(err)?)?;
        let wire = root(s, row.blob(5).map_err(err)?)?;
        let first: i64 = row.integer(6).map_err(err)?;
        check_generation(first, latest)?;
        exact(&generated, 6)?;
        if text(s, &generated, "kind")? != kind
            || window(s, &generated, "window_start")? != window_start
            || observation_id(s, &generated)? != id
            || observation_wire(s, &generated, &id, Some(&batch))? != wire
        {
            return Err(err("观察索引与出生原件不符"));
        }
        out.observations
            .try_push(Observation {
                id,
                kind,
                window_start,
                batch: Some(batch),
                generated,
                wire,
                first,
            })
            .map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
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
) -> Result<[CodecVec<NodeDigest>; 3], String> {
    cut_members_metered(index, generation, delta, None)
}
pub(super) fn cut_members_metered(
    index: &Index,
    generation: i64,
    delta: bool,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<[CodecVec<NodeDigest>; 3], String> {
    let member = |first| {
        if delta {
            first == generation
        } else {
            first <= generation
        }
    };
    let owned = index.budget.as_ref();
    let mut w = bj::new_work(owned);
    let mut r = bj::new_work(owned);
    let mut o = bj::new_work(owned);
    for row in index.witnesses.iter().filter(|row| member(row.first)) {
        w.try_push(row).map_err(err)?;
    }
    for row in index.relations.iter().filter(|row| member(row.first)) {
        r.try_push(row).map_err(err)?;
    }
    for row in index.observations.iter().filter(|row| member(row.first)) {
        o.try_push(row).map_err(err)?;
    }
    if !delta {
        stable_sort(
            &mut w,
            |a, b| (&a.object_id, a.slot).cmp(&(&b.object_id, b.slot)),
            owned,
        )?;
        stable_sort(
            &mut r,
            |a, b| {
                (&a.subject, &a.relation_type, &a.object).cmp(&(
                    &b.subject,
                    &b.relation_type,
                    &b.object,
                ))
            },
            owned,
        )?;
        stable_sort(
            &mut o,
            |a, b| (&a.kind, a.window_start).cmp(&(&b.kind, b.window_start)),
            owned,
        )?;
    }
    let mut witnesses = bj::new_work(owned);
    let mut relations = bj::new_work(owned);
    let mut observations = bj::new_work(owned);
    for row in w {
        if let Some(b) = budget {
            b.record().map_err(err)?;
        }
        witnesses.try_push(digest(&row.wire)?).map_err(err)?;
    }
    for row in r {
        if let Some(b) = budget {
            b.record().map_err(err)?;
        }
        relations.try_push(digest(&row.generated)?).map_err(err)?;
    }
    for row in o {
        if let Some(b) = budget {
            b.record().map_err(err)?;
        }
        observations.try_push(digest(&row.wire)?).map_err(err)?;
    }
    Ok([witnesses, relations, observations])
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
    s: &mut (impl CodecStore + ?Sized),
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
                w.id.as_str(),
                w.object_id.as_str(),
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
                r.id.as_str(),
                r.subject.as_str(),
                r.relation_type.as_str(),
                r.object.as_str(),
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
        let wire = s
            .replace_ref(o.wire.view(), &[f("batch_id")], batch)
            .map_err(err)?;
        s.verify_many_ref_fresh([wire.view()], sc::Mode::Public)
            .map_err(err)?;
        tx.execute(
            "INSERT INTO s_content_observations VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                o.id.as_str(),
                o.kind.as_str(),
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
        json!({"witnesses":render_plain(s,&p.witnesses.content_ref()).unwrap(),"relations":render_plain(s,&p.relations.content_ref()).unwrap(),"raw_history":render_plain(s,&p.raw_history.content_ref()).unwrap()})
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn owned_aux_fixture() -> Connection {
        let input = json!({"object_id":"obj3-fixture","merged_index":7,"bars":[{"seq":0,"revision":1,"supersedes_revision":null,"source_coord":"0","schema_revision":crate::tb02a::RAW_SCHEMA,"price":"12","open":"10","high":"14","low":"8","close":"12","extension":{"preserved":true}}],
            "relations":[{"subject":"obj-z","relation_type":"replaces","object":"obj-a"},{"subject":"raw-a@2","relation_type":"supersedes","object":"raw-a@1"},{"subject":"obj-z","relation_type":"member_of","object":"raw-a@2"}],
            "observations":[{"kind":"fixture","window_start":null,"window_mid":3,"window_end":5,"reason":"first","detail":{"extension":[1,2]}},{"kind":"fixture","window_start":null,"window_mid":3,"window_end":5,"reason":"second","detail":{"extension":[3,4]}},{"kind":"tb02a-fixture","window_start":1,"window_mid":3,"window_end":5,"reason":"typed","detail":{"tb02a":true,"extension":{"retained":true}}}]});
        let bytes = serde_json::to_vec(&input).unwrap();
        std::fs::write(
            "/private/tmp/issue1394-aux-owned-input-20261003.json",
            &bytes,
        )
        .unwrap();
        println!("aux input SHA256 {}", crate::sha256_hex(&bytes));
        let input: Value = serde_json::from_slice(&bytes).unwrap();
        let mut conn = database();
        let tx = conn.transaction().unwrap();
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let mut runtime = Runtime::default();
        let mut index = Index::default();
        let object = input["object_id"].as_str().unwrap();
        let mi = input["merged_index"].as_i64().unwrap();
        let recipe = json!({"mi":mi,"object_id":object,"slot":0,"raw_bars":input["bars"]});
        let id = format!(
            "wit-{}",
            &crate::sha256_hex(crate::canonical_json(&recipe).as_bytes())[..16]
        );
        let witness = json!({"witness_id":id,"object_id":object,"slot":0,"merged_source_index":0,"merged_high":"14","merged_low":"8","merged_open":"10","merged_close":"12","raw_bars":input["bars"]});
        let b = write_json(&mut store, &witness).unwrap();
        index
            .witnesses
            .try_push(
                runtime
                    .witness(
                        &mut store,
                        &WitnessRecord {
                            witness_id: id,
                            object_id: object.to_owned(),
                            slot: 0,
                            built: b,
                        },
                        mi,
                        1,
                    )
                    .unwrap(),
            )
            .unwrap();
        for input in input["relations"].as_array().unwrap() {
            let b = write_json(&mut store, input).unwrap();
            let r = built_ref(&store, &b);
            index
                .relations
                .try_push(relation(&mut store, r, 1).unwrap())
                .unwrap();
        }
        for input in input["observations"].as_array().unwrap() {
            let b = write_json(&mut store, input).unwrap();
            index
                .observations
                .try_push(runtime.observation(&mut store, &b, 1).unwrap())
                .unwrap();
        }
        let snapshot_values =
            public_sets(&mut store, &index, &[], &mut runtime.sequences[..4]).unwrap();
        let delta_values =
            public_sets(&mut store, &index, &[], &mut runtime.sequences[4..]).unwrap();
        let p = Plan {
            generation: 1,
            snapshot: index.try_clone().unwrap(),
            births: index,
            snapshot_values,
            delta_values,
        };
        stage(&tx, &mut store, &p, &format!("batch3-{}", "a".repeat(64))).unwrap();
        tx.commit().unwrap();
        conn
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn owned_aux_capture(tx: &Transaction<'_>, budget: &OwnedBudget) -> sc::AuditCapture {
        let (count, bytes): (i64, i64) = tx
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(length(body)),0) FROM s_content_nodes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut slab = sc::capture_allocation::CaptureSlab::new(
            count.try_into().unwrap(),
            bytes.try_into().unwrap(),
            budget,
        )
        .unwrap();
        let mut stmt = tx
            .prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let digest: NodeDigest = row
                .get_ref(0)
                .unwrap()
                .as_blob()
                .unwrap()
                .try_into()
                .unwrap();
            let tag: u8 = row.get(1).unwrap();
            slab.push(digest, tag, row.get_ref(2).unwrap().as_blob().unwrap())
                .unwrap();
        }
        sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap())
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn aux_values(index: &Index) -> Value {
        json!({"witnesses":index.witnesses.iter().map(|r|json!({"id":r.id,"object_id":r.object_id,"slot":r.slot,"merged_index":r.merged_index,"generated":r.generated,"wire":r.wire,"first":r.first})).collect::<Vec<_>>(),
            "relations":index.relations.iter().map(|r|json!({"id":r.id,"subject":r.subject,"relation_type":r.relation_type,"object":r.object,"generated":r.generated,"first":r.first})).collect::<Vec<_>>(),
            "observations":index.observations.iter().map(|r|json!({"id":r.id,"kind":r.kind,"window_start":r.window_start,"generated":r.generated,"wire":r.wire,"batch":r.batch,"first":r.first})).collect::<Vec<_>>()})
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_aux_actual_sql_load_matches_normal_types_order_refs_and_projection() {
        let mut conn = owned_aux_fixture();
        let tx = conn.transaction().unwrap();
        let mut normal = content_store::TxStore::new(&tx).unwrap();
        let expected = load(&tx, &mut normal, 1).unwrap();
        let expected_values = aux_values(&expected);
        let expected_members = cut_members(&expected, 1, false).unwrap();
        let mut normal_runtime = Runtime::default();
        let b = |r: &RefValue| Built {
            digest: r.digest().unwrap(),
            kind: r.kind(),
            extent: r.extent(),
        };
        let w = &expected.witnesses[0];
        let metadata = WitnessRecord {
            witness_id: w.id.as_str().to_owned(),
            object_id: w.object_id.as_str().to_owned(),
            slot: w.slot as usize,
            built: b(&w.generated),
        };
        let mut constructed = Index::default();
        constructed
            .witnesses
            .try_push(
                normal_runtime
                    .witness(&mut normal, &metadata, w.merged_index, 1)
                    .unwrap(),
            )
            .unwrap();
        for r in &expected.relations {
            constructed
                .relations
                .try_push(relation(&mut normal, r.generated.clone(), 1).unwrap())
                .unwrap();
        }
        for r in &expected.observations {
            constructed
                .observations
                .try_push(
                    normal_runtime
                        .observation(&mut normal, &b(&r.generated), 1)
                        .unwrap(),
                )
                .unwrap();
        }
        let constructed_values = aux_values(&constructed);
        let normal_sets = public_sets(
            &mut normal,
            &constructed,
            &[],
            &mut normal_runtime.sequences[..4],
        )
        .unwrap();
        let budget = OwnedBudget::new(8 * 1024 * 1024);
        let capture = owned_aux_capture(&tx, &budget);
        let actual = super::super::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            4096,
            512 * 1024,
            |store| {
                assert!(store.allocation_budget().is_some());
                let actual = load(&tx, store, 1).unwrap();
                assert_eq!(aux_values(&actual), expected_values);
                let mut runtime = Runtime::default();
                let mut constructed = Index::new(store.allocation_budget());
                constructed
                    .witnesses
                    .try_push(
                        runtime
                            .witness(store, &metadata, w.merged_index, 1)
                            .unwrap(),
                    )
                    .unwrap();
                for r in &actual.relations {
                    constructed
                        .relations
                        .try_push(relation(store, r.generated.clone(), 1).unwrap())
                        .unwrap();
                }
                for r in &actual.observations {
                    constructed
                        .observations
                        .try_push(runtime.observation(store, &b(&r.generated), 1).unwrap())
                        .unwrap();
                }
                assert_eq!(aux_values(&constructed), constructed_values);
                let actual_sets =
                    public_sets(store, &constructed, &[], &mut runtime.sequences[..4]).unwrap();
                assert_eq!(actual_sets.witnesses, normal_sets.witnesses);
                assert_eq!(actual_sets.relations, normal_sets.relations);
                assert_eq!(actual_sets.observations, normal_sets.observations);
                let foreign = OwnedBudget::new(budget.snapshot().limit_bytes);
                let equal_numbers = OwnedBudget::new(budget.snapshot().limit_bytes);
                assert_eq!(
                    foreign.snapshot().live_requested_layout_bytes,
                    equal_numbers.snapshot().live_requested_layout_bytes
                );
                assert_eq!(
                    foreign.snapshot().allocated_layouts,
                    equal_numbers.snapshot().allocated_layouts
                );
                assert!(!foreign.same_ledger(&equal_numbers));
                assert!(budget.same_ledger(&budget.clone()));
                assert!(actual.clone_in(Some(foreign.clone())).is_err());
                assert!(runtime
                    .witness_wires
                    .get(&(metadata.built.digest, w.merged_index), Some(&foreign))
                    .is_err());
                assert_eq!(foreign.snapshot().live_requested_layout_bytes, 0);
                drop(constructed);
                drop(runtime);
                tx.execute("UPDATE s_content_witnesses SET slot=1", [])
                    .unwrap();
                let owned_error = load(&tx, store, 1).err().unwrap();
                let normal_error = load(&tx, &mut normal, 1).err().unwrap();
                assert_eq!(
                    owned_error, normal_error,
                    "同一错误SQL元数据的拒绝判据/错误不漂移"
                );
                tx.execute("UPDATE s_content_witnesses SET slot=0", [])
                    .unwrap();
                let members = cut_members(&actual, 1, false).unwrap();
                for (a, b) in members.iter().zip(&expected_members) {
                    assert_eq!(&a[..], &b[..]);
                }
                for w in &actual.witnesses {
                    assert!(matches!(w.id, TextValue::Owned(_)));
                    assert!(matches!(w.object_id, TextValue::Owned(_)));
                    assert!(matches!(w.generated, RefValue::Owned(_)));
                    assert!(matches!(w.wire, RefValue::Owned(_)));
                }
                for r in &actual.relations {
                    for t in [&r.id, &r.subject, &r.relation_type, &r.object] {
                        assert!(matches!(t, TextValue::Owned(_)));
                    }
                    assert!(matches!(r.generated, RefValue::Owned(_)));
                }
                for o in &actual.observations {
                    assert!(matches!(o.id, TextValue::Owned(_)));
                    assert!(matches!(o.kind, TextValue::Owned(_)));
                    assert!(matches!(o.batch, Some(TextValue::Owned(_))));
                    assert!(matches!(o.generated, RefValue::Owned(_)));
                    assert!(matches!(o.wire, RefValue::Owned(_)));
                }
                let wire = render_plain(store, &actual.witnesses[0].wire).unwrap();
                assert_eq!(wire["raw_bars"][0]["extension"], json!({"preserved":true}));
                assert!(wire["raw_bars"][0].get("price").is_none());
                assert_eq!(wire["raw_bars"][0]["seq"], "0");
                actual
            },
        );
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        drop(actual);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_aux_existing_rows_clone_and_growth_one_byte_short_refuse_before_change() {
        let mut conn = owned_aux_fixture();
        let tx = conn.transaction().unwrap();
        let before_nodes = count(&tx, "s_content_nodes");
        let budget = OwnedBudget::new(8 * 1024 * 1024);
        let capture = owned_aux_capture(&tx, &budget);
        let actual = super::super::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            4096,
            512 * 1024,
            |store| load(&tx, store, 1).unwrap(),
        );
        let before = aux_values(&actual);
        let live = budget.snapshot().live_requested_layout_bytes;
        let request = actual.witnesses[0].id.as_str().len();
        let fill = sc::capture_allocation::CaptureSlab::new(
            0,
            budget.snapshot().limit_bytes - live - request + 1,
            &budget,
        )
        .unwrap();
        let error = actual
            .try_clone()
            .err()
            .expect("真实旧行仍活着时，clone差一字节须拒绝");
        assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, request);
        assert_eq!(
            probe.live_before_refusal + request,
            budget.snapshot().limit_bytes + 1
        );
        assert_eq!(aux_values(&actual), before);
        assert_eq!(count(&tx, "s_content_nodes"), before_nodes);
        drop(fill);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, live);
        let mut probe_rows = bj::new_work(Some(&budget));
        probe_rows
            .try_push(actual.witnesses[0].try_clone().unwrap())
            .unwrap();
        let growth_request = budget.allocation_request_probe().last_request_bytes;
        drop(probe_rows);
        let candidate = actual.witnesses[0].try_clone().unwrap();
        let mut rows: CodecVec<Witness> = bj::new_work(Some(&budget));
        let with_candidate = budget.snapshot().live_requested_layout_bytes;
        let fill = sc::capture_allocation::CaptureSlab::new(
            0,
            budget.snapshot().limit_bytes - with_candidate - growth_request + 1,
            &budget,
        )
        .unwrap();
        let error = rows.try_push(candidate).unwrap_err();
        assert_eq!(error.code(), sc::capture_allocation::LIMIT);
        assert!(rows.is_empty());
        assert_eq!(rows.capacity(), 0);
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, growth_request);
        assert_eq!(
            probe.live_before_refusal + growth_request,
            budget.snapshot().limit_bytes + 1
        );
        assert_eq!(aux_values(&actual), before);
        assert_eq!(count(&tx, "s_content_nodes"), before_nodes);
        drop(fill);
        drop(rows);
        let clone = actual.try_clone().unwrap();
        assert_eq!(aux_values(&clone), before);
        drop(clone);
        drop(actual);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
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
                all_events: (&raw).into(),
                effective: (&effective).into(),
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
            let delta_raw =
                render_plain(&mut s, &p.delta_values.raw_history.content_ref()).unwrap();
            assert_eq!(
                delta_raw.as_array().unwrap().len(),
                (frontier - previous) as usize
            );
            for w in &generated.witnesses {
                let expected = crate::project_witness_wire(
                    &render_plain(&mut s, &w.built.content_ref()).unwrap(),
                )
                .unwrap();
                let actual = p
                    .snapshot
                    .witnesses
                    .iter()
                    .find(|r| r.id == w.witness_id)
                    .unwrap();
                assert_eq!(render_plain(&mut s, &actual.wire).unwrap(), expected);
            }
            for id in &prior_ids {
                assert!(!p.births.observations.iter().any(|r| &r.id == id));
            }
            if g == 1 {
                first_witness = p.snapshot.witnesses[0].id.as_str().to_owned();
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
                let mut missing = stored.try_clone().unwrap();
                let rows = std::mem::take(&mut missing.witnesses);
                for (i, row) in rows.into_iter().enumerate() {
                    if i != first {
                        missing.witnesses.try_push(row).unwrap();
                    }
                }
                assert!(verify_generated_members(
                    &mut s,
                    &missing,
                    &generated.generated_state,
                    &[],
                    &raw[..=frontier as usize],
                    g,
                )
                .is_err());

                let mut ghost = stored.try_clone().unwrap();
                ghost
                    .witnesses
                    .try_push(Witness {
                        id: "wit-ghost".into(),
                        ..stored.witnesses[first].try_clone().unwrap()
                    })
                    .unwrap();
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
                let mut wrong = stored.try_clone().unwrap();
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
                    &members[0][..],
                    expected
                        .witnesses
                        .iter()
                        .map(|r| digest(&r.wire).unwrap())
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    &members[1][..],
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
                            s.replace_ref(r.wire.view(), &[f("batch_id")], &batch)
                                .unwrap()
                                .digest()
                                .unwrap()
                        } else {
                            digest(&r.wire).unwrap()
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(&members[2][..], &obs[..]);
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
                let mut original = render_plain(&mut s, &o.generated).unwrap();
                original["observation_id"] = json!(o.id);
                original["batch_id"] = json!(o.batch.as_ref().unwrap());
                assert_eq!(
                    render_plain(&mut s, &o.wire).unwrap(),
                    crate::project_observation_wire(&original).unwrap()
                );
            }
            prior_ids = stored
                .observations
                .iter()
                .map(|r| r.id.as_str().to_owned())
                .collect();
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
                field(&mut s, &ob.wire, "batch_id").unwrap().kind(),
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
                observations: CodecVec::from_plain(vec![row]),
                ..Index::default()
            };
            let a = public_sets(&mut s, &index, &[], &mut rt.sequences[..4]).unwrap();
            let b = public_sets(&mut s, &index, &[], &mut rt.sequences[4..]).unwrap();
            let p = Plan {
                generation: 1,
                snapshot: index.try_clone().unwrap(),
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
