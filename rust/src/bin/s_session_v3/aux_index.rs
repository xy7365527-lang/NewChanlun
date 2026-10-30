//! #1394 C §5.4：见证、关系、首次观察的独立内容索引。
//! plan 先产生含本批占位的值根；manifest 求出 batch3 后 stage 同事务暂写出生记录。
//! 观察 detail 与来源数组保留为边；仅单条原始事件沿既有投影转换。索引不替代来源业务审计。
use super::business_json::{self as bj, RawEventsView};
use super::generation::{Generated, SequenceMemo, WitnessRecord};
use super::layout::Layout;
use crate::v3::audit_store::{CodecRead, CodecStore, NativeTree, RefValue};
use crate::{content_store, meta_get_opt};
use aj::JsonView;
use bj::TextValue;
use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
use newchan_rust::s_content::ContextError as PublishError;
#[cfg_attr(
    not(test),
    allow(unused_imports, reason = "#1488 再合 #1608：仅测试使用")
)]
use newchan_rust::s_content::{
    self as sc, path::PathStep, Built, ContentRef, Kind, NodeDigest, NodeStore,
};
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::{params, Transaction};
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use sc::{CodecVec, ValueView as VV};
#[cfg(test)]
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
) -> Result<RefValue, PublishError> {
    s.lookup_field_ref(r.view(), k).map_err(publish_err)
}
fn digest(r: &(impl AuxRef + ?Sized)) -> Result<NodeDigest, PublishError> {
    r.view().digest().map_err(publish_err)
}
fn scalar(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<bj::JsonOwner, PublishError> {
    let leaf = field(s, r, k)?;
    if !matches!(
        leaf.kind(),
        Kind::Null | Kind::Bool | Kind::Number | Kind::String
    ) {
        return Err(publish_err(format!("{k}不是标量")));
    }
    render(s, &leaf)
}
fn text(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<TextValue, PublishError> {
    let value = scalar(s, r, k)?;
    let value = value
        .view()
        .as_str()
        .ok_or_else(|| publish_err(format!("{k}不是文本")))?;
    s.native_text(value).map_err(publish_err)
}
fn integer(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<i64, PublishError> {
    scalar(s, r, k)?
        .view()
        .as_i64()
        .ok_or_else(|| publish_err(format!("{k}不是i64")))
}
fn render(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<bj::JsonOwner, PublishError> {
    bj::render_json(s, r.view()).map_err(Into::into)
}
#[cfg(test)]
fn render_plain(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<Value, String> {
    let value = render(s, r).map_err(newchan_rust::s_content::ContextError::into_message)?;
    serde_json::to_value(value.view()).map_err(err)
}
#[cfg(test)]
fn write_json(s: &mut (impl CodecStore + ?Sized), v: &Value) -> Result<Built, String> {
    s.write_json_view(v.into()).map_err(err)
}
fn write_fields(
    s: &mut (impl CodecStore + ?Sized),
    fields: &[(&str, JsonView<'_>)],
) -> Result<Built, PublishError> {
    s.write_json_view(JsonView::object_fields(fields).map_err(publish_err)?)
        .map_err(publish_err)
}
fn edge(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<VV<'static>, PublishError> {
    Ok(VV::Edge(digest(&field(s, r, k)?)?))
}
fn array_elements(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<CodecVec<RefValue>, PublishError> {
    if r.view().kind() != Kind::Array {
        return Err(publish_err("预期数组"));
    }
    let mut out = bj::new_work(s.allocation_budget().as_ref());
    for i in 0..r.view().extent().ok_or_else(|| publish_err("数组缺长度"))? {
        s.native_codec_room(&mut out, 1).map_err(publish_err)?;
        out.try_push(
            s.lookup_ref(r.view(), &[PathStep::Index(i)])
                .map_err(publish_err)?,
        )
        .map_err(publish_err)?;
    }
    Ok(out)
}
fn root(s: &mut (impl CodecRead + ?Sized), b: &[u8]) -> Result<RefValue, PublishError> {
    let d: NodeDigest = b.try_into().map_err(|_| publish_err("根不是32字节"))?;
    let r = s
        .content_ref_view(sc::FixedRoot::new(&d).as_str())
        .map_err(publish_err)?;
    s.verify_many_ref_fresh([r.view()], sc::Mode::Public)
        .map_err(publish_err)?;
    Ok(r)
}
fn built_ref(s: &(impl CodecRead + ?Sized), b: &Built) -> Result<RefValue, PublishError> {
    s.native_built_ref(b).map_err(publish_err)
}
fn exact(r: &(impl AuxRef + ?Sized), n: u64) -> Result<(), PublishError> {
    if r.view().kind() != Kind::Object || r.view().extent() != Some(n) {
        Err(publish_err(format!("预期{n}字段对象")))
    } else {
        Ok(())
    }
}
fn fmt_text(
    s: &(impl CodecRead + ?Sized),
    args: std::fmt::Arguments<'_>,
    budget: Option<&OwnedBudget>,
) -> Result<TextValue, PublishError> {
    use std::fmt::Write;
    struct Out<'a> {
        work: Option<&'a sc::capture_allocation::scratch::WorkScope>,
        bytes: CodecVec<u8>,
        error: Option<sc::CodecError>,
    }
    impl std::fmt::Write for Out<'_> {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            super::audit_store::NativeAdmission(self.work)
                .native_codec_room(&mut self.bytes, value.len())
                .map_err(|error| {
                    self.error = Some(error);
                    std::fmt::Error
                })?;
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
        work: s.codec_work(),
        bytes: bj::new_work(budget),
        error: None,
    };
    let result = out.write_fmt(args);
    if let Some(error) = out.error {
        return Err(publish_err(error));
    }
    result.map_err(|error| publish_err(error.to_string()))?;
    let text = std::str::from_utf8(&out.bytes).map_err(publish_err)?;
    if budget.is_none() {
        s.native_text(text).map_err(publish_err)
    } else {
        TextValue::from_str(text, budget).map_err(publish_err)
    }
}
/// 见证/关系/观察的公开主序键：出生代（表列 `published_generation`，载入为 `first`）经
/// [`Layout::rank`] 取代际前缀，再接 layout/1 原键；快照排序与历史 cut 重建只经这三处取键。
fn witness_rank(layout: Layout, row: &Witness) -> (i64, (&TextValue, i64)) {
    layout.rank(row.first, (&row.object_id, row.slot))
}
fn relation_rank(layout: Layout, row: &Relation) -> (i64, (&TextValue, &TextValue, &TextValue)) {
    layout.rank(row.first, (&row.subject, &row.relation_type, &row.object))
}
fn obs_rank(layout: Layout, row: &Observation) -> (i64, (&TextValue, Option<i64>)) {
    layout.rank(row.first, (&row.kind, row.window_start))
}
fn stable_sort<T>(
    rows: &mut CodecVec<T>,
    compare: impl Fn(&T, &T) -> std::cmp::Ordering,
    budget: Option<&OwnedBudget>,
    admission: &(impl CodecRead + ?Sized),
) -> Result<(), PublishError> {
    let mut order = bj::new_work(budget);
    let mut target = bj::new_work(budget);
    admission
        .native_codec_room(&mut order, rows.len())
        .map_err(publish_err)?;
    admission
        .native_codec_room(&mut target, rows.len())
        .map_err(publish_err)?;
    for i in 0..rows.len() {
        order.try_push(i).map_err(publish_err)?;
        target.try_push(0usize).map_err(publish_err)?;
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
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn try_clone(&self) -> Result<Self, String> {
        self.try_clone_admitted(&super::audit_store::NativeAdmission(None))
            .map_err(newchan_rust::s_content::ContextError::into_message)
    }
    fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self {
            id: self.id.try_clone_admitted(store.codec_work())?,
            object_id: self.object_id.try_clone_admitted(store.codec_work())?,
            slot: self.slot,
            merged_index: self.merged_index,
            generated: store
                .native_ref_clone(&self.generated)
                .map_err(publish_err)?,
            wire: store.native_ref_clone(&self.wire).map_err(publish_err)?,
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
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：native 非测试、s_session 非测试、s_session 测试 构建不触达"
        )
    )]
    fn try_clone(&self) -> Result<Self, String> {
        self.try_clone_admitted(&super::audit_store::NativeAdmission(None))
            .map_err(newchan_rust::s_content::ContextError::into_message)
    }
    fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self {
            id: self.id.try_clone_admitted(store.codec_work())?,
            subject: self.subject.try_clone_admitted(store.codec_work())?,
            relation_type: self.relation_type.try_clone_admitted(store.codec_work())?,
            object: self.object.try_clone_admitted(store.codec_work())?,
            generated: store
                .native_ref_clone(&self.generated)
                .map_err(publish_err)?,
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
    #[allow(dead_code, reason = "#1488 再合 #1608：8组合实测无调用方，保留不删")]
    fn try_clone(&self) -> Result<Self, String> {
        self.try_clone_admitted(&super::audit_store::NativeAdmission(None))
            .map_err(newchan_rust::s_content::ContextError::into_message)
    }
    fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self {
            id: self.id.try_clone_admitted(store.codec_work())?,
            kind: self.kind.try_clone_admitted(store.codec_work())?,
            window_start: self.window_start,
            generated: store
                .native_ref_clone(&self.generated)
                .map_err(publish_err)?,
            wire: store.native_ref_clone(&self.wire).map_err(publish_err)?,
            batch: self
                .batch
                .as_ref()
                .map(|v| v.try_clone_admitted(store.codec_work()))
                .transpose()?,
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
    #[cfg(test)]
    pub(super) fn try_clone(&self) -> Result<Self, String> {
        self.clone_in(self.budget.clone())
    }
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn clone_in(&self, budget: Option<OwnedBudget>) -> Result<Self, String> {
        self.clone_in_admitted(budget, &super::audit_store::NativeAdmission(None))
            .map_err(newchan_rust::s_content::ContextError::into_message)
    }
    fn clone_in_admitted(
        &self,
        budget: Option<OwnedBudget>,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<Self, PublishError> {
        let retained = self.witnesses.capacity() != 0
            || self.relations.capacity() != 0
            || self.observations.capacity() != 0;
        if retained && !same_budget(self.budget.as_ref(), budget.as_ref()) {
            return Err(publish_err(
                "prototype_audit_allocation_incomplete：索引账本改变，须重新load",
            ));
        }
        let mut out = Self::new(budget);
        for row in &self.witnesses {
            store
                .native_codec_room(&mut out.witnesses, 1)
                .map_err(publish_err)?;
            out.witnesses
                .try_push(row.try_clone_admitted(store)?)
                .map_err(publish_err)?;
        }
        for row in &self.relations {
            store
                .native_codec_room(&mut out.relations, 1)
                .map_err(publish_err)?;
            out.relations
                .try_push(row.try_clone_admitted(store)?)
                .map_err(publish_err)?;
        }
        for row in &self.observations {
            store
                .native_codec_room(&mut out.observations, 1)
                .map_err(publish_err)?;
            out.observations
                .try_push(row.try_clone_admitted(store)?)
                .map_err(publish_err)?;
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
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
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
    fn check(&self, budget: Option<&OwnedBudget>) -> Result<(), PublishError> {
        if self.map.is_some() && !same_budget(self.budget.as_ref(), budget) {
            return Err(publish_err(
                "prototype_audit_allocation_incomplete：缓存账本改变，须discard",
            ));
        }
        Ok(())
    }
    fn get(&self, key: &K, budget: Option<&OwnedBudget>) -> Result<Option<&V>, PublishError> {
        self.check(budget)?;
        Ok(self.map.as_ref().and_then(|m| m.get(key)))
    }
    fn insert(
        &mut self,
        key: K,
        value: V,
        budget: Option<&OwnedBudget>,
    ) -> Result<(), PublishError> {
        self.check(budget)?;
        let map = self.map.get_or_insert_with(|| work_map(budget));
        self.budget = budget.cloned();
        map.insert(key, value).map_err(publish_err)?;
        Ok(())
    }
    fn insert_admitted(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        key: K,
        value: V,
        budget: Option<&OwnedBudget>,
    ) -> Result<(), PublishError> {
        self.check(budget)?;
        let map = self.map.get_or_insert_with(|| work_map(budget));
        s.native_map_room(map).map_err(publish_err)?;
        self.budget = budget.cloned();
        map.insert(key, value).map_err(publish_err)?;
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
    #[cfg(all(test, feature = "s_incremental"))]
    pub(super) fn visit_runtime_roots(
        &self,
        f: &mut dyn FnMut(&'static str, NodeDigest),
    ) -> Result<(), String> {
        if let Some(map) = &self.witness_wires.map {
            for ((root, _), value) in map.iter() {
                f("aux.witness_wires.key", *root);
                f("aux.witness_wires.value", value.digest().map_err(err)?);
            }
        }
        if let Some(map) = &self.bar_wires.map {
            for (root, value) in map.iter() {
                f("aux.bar_wires.key", *root);
                f("aux.bar_wires.value", value.digest().map_err(err)?);
            }
        }
        if let Some(map) = &self.observation_ids.map {
            for (root, _) in map.iter() {
                f("aux.observation_ids.key", *root);
            }
        }
        if let Some(map) = &self.raw_wires.map {
            for (_, value) in map.iter() {
                f("aux.raw_wires.value", *value);
            }
        }
        for (name, memo) in [
            "aux.seq0", "aux.seq1", "aux.seq2", "aux.seq3", "aux.seq4", "aux.seq5", "aux.seq6",
            "aux.seq7",
        ]
        .into_iter()
        .zip(&self.sequences)
        {
            memo.visit_runtime_roots(name, f);
        }
        Ok(())
    }

    pub(super) fn discard(&mut self) {
        *self = Self::default();
    }

    fn witness(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        w: &WitnessRecord,
        merged_index: i64,
        g: i64,
    ) -> Result<Witness, PublishError> {
        let generated = built_ref(s, &w.built)?;
        s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
            .map_err(publish_err)?;
        exact(&generated, 9)?;
        let object_id = text(s, &generated, "object_id")?;
        let slot = integer(s, &generated, "slot")?;
        let id = text(s, &generated, "witness_id")?;
        if object_id.as_str() != w.object_id.as_str()
            || slot != w.slot as i64
            || id.as_str() != w.witness_id.as_str()
            || !(0..3).contains(&slot)
        {
            return Err(publish_err("见证生成记录与元数据不符"));
        }
        let wire = match self.witness_wires.get(
            &(w.built.digest, merged_index),
            s.allocation_budget().as_ref(),
        )? {
            Some(r) => s.native_ref_clone(r).map_err(publish_err)?,
            None => {
                verify_witness_id(s, &generated, &id, &object_id, slot, merged_index)?;
                let r = self.witness_wire(s, &generated)?;
                self.witness_wires.insert_admitted(
                    s,
                    (w.built.digest, merged_index),
                    s.native_ref_clone(&r).map_err(publish_err)?,
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
    ) -> Result<RefValue, PublishError> {
        let bars = field(s, r, "raw_bars")?;
        let owned_budget = s.allocation_budget();
        let mut values = bj::new_work(owned_budget.as_ref());
        for bar in array_elements(s, &bars)? {
            let d = digest(&bar)?;
            let projected =
                if let Some(cached) = self.bar_wires.get(&d, s.allocation_budget().as_ref())? {
                    s.native_ref_clone(cached).map_err(publish_err)?
                } else {
                    let original = render(s, &bar)?;
                    let built = project_bar(s, original.view())?;
                    let r = built_ref(s, &built)?;
                    self.bar_wires.insert_admitted(
                        s,
                        d,
                        s.native_ref_clone(&r).map_err(publish_err)?,
                        owned_budget.as_ref(),
                    )?;
                    r
                };
            s.native_codec_room(&mut values, 1).map_err(publish_err)?;
            values
                .try_push(VV::Edge(digest(&projected)?))
                .map_err(publish_err)?;
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
        let b = s
            .write_value_view(VV::Object(&fields))
            .map_err(publish_err)?;
        Ok(built_ref(s, &b)?)
    }
    fn observation(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        b: &Built,
        g: i64,
    ) -> Result<Observation, PublishError> {
        let generated = built_ref(s, b)?;
        s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
            .map_err(publish_err)?;
        exact(&generated, 6)?;
        let kind = text(s, &generated, "kind")?;
        let window_start = window(s, &generated, "window_start")?;
        let id = if let Some(id) = self
            .observation_ids
            .get(&b.digest, s.allocation_budget().as_ref())?
        {
            id.try_clone_admitted(s.codec_work())?
        } else {
            let id = observation_id(s, &generated)?;
            self.observation_ids.insert_admitted(
                s,
                b.digest,
                id.try_clone_admitted(s.codec_work())?,
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
        layout: Layout,
        old: &Index,
        generated: &Generated<'_>,
        replaces: impl Into<RawEventsView<'a>>,
        raw: impl Into<RawEventsView<'a>>,
        previous_frontier: i64,
        frontier: i64,
        g: i64,
    ) -> Result<Plan, PublishError> {
        let raw = raw.into();
        let replaces = replaces.into();
        if g < 1
            || previous_frontier < -1
            || frontier <= previous_frontier
            || frontier as usize >= raw.len()
        {
            return Err(publish_err("代号/原始前沿非法"));
        }
        let owned_budget = s.allocation_budget();
        let mut snapshot = old.clone_in_admitted(owned_budget.clone(), s)?;
        let mut births = Index::new(owned_budget.clone());
        let mut ws = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.witnesses.iter().enumerate() {
            s.native_map_room(&mut ws).map_err(publish_err)?;
            ws.insert(row.id.try_clone_admitted(s.codec_work())?, i)
                .map_err(publish_err)?;
        }
        // 未变行跳过（#1488 S3）：生成态逐代给全量见证/关系/观察，但旧快照已含同内容
        // 行时，冷口重算只得到同一行并在下方比对后丢弃。按生成记录摘要直接跳过，
        // 读量只随本代新内容；换内容仍走完整路径并照旧报冲突。
        let unchanged = incremental_plan();
        let mut known_witnesses = work_map(owned_budget.as_ref());
        if unchanged {
            for row in snapshot.witnesses.iter() {
                s.native_map_room(&mut known_witnesses)
                    .map_err(publish_err)?;
                known_witnesses
                    .insert(digest(&row.generated)?, ())
                    .map_err(publish_err)?;
            }
        }
        for w in &generated.witnesses {
            if known_witnesses.get(&w.built.digest).is_some() {
                continue;
            }
            let object = generated
                .objects
                .iter()
                .find(|o| o.object_id() == w.object_id.as_str())
                .ok_or_else(|| publish_err("见证对象不在本代生成态"))?;
            let generated_object = built_ref(s, object.generated())?;
            let group = s
                .lookup_ref(
                    generated_object.view(),
                    &[
                        f("input_refs_json"),
                        PathStep::JsonText,
                        PathStep::Index(w.slot as u64),
                    ],
                )
                .map_err(publish_err)?;
            // 身份配方用合并序号 mi；公开 merged_source_index 是原始坐标，二者不可互换。
            let merged_index = integer(s, &group, "merged_index")?;
            let row = self.witness(s, w, merged_index, g)?;
            if let Some(i) = ws.get(&row.id) {
                let prior = &snapshot.witnesses[*i];
                if prior.generated != row.generated
                    || prior.wire != row.wire
                    || prior.merged_index != row.merged_index
                {
                    return Err(publish_err("同见证身份换内容"));
                }
            } else {
                s.native_map_room(&mut ws).map_err(publish_err)?;
                ws.insert(
                    row.id.try_clone_admitted(s.codec_work())?,
                    snapshot.witnesses.len(),
                )
                .map_err(publish_err)?;
                s.native_codec_room(&mut snapshot.witnesses, 1)
                    .map_err(publish_err)?;
                snapshot
                    .witnesses
                    .try_push(row.try_clone_admitted(s)?)
                    .map_err(publish_err)?;
                s.native_codec_room(&mut births.witnesses, 1)
                    .map_err(publish_err)?;
                births.witnesses.try_push(row).map_err(publish_err)?;
            }
        }
        let mut rs = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.relations.iter().enumerate() {
            s.native_map_room(&mut rs).map_err(publish_err)?;
            rs.insert(row.id.try_clone_admitted(s.codec_work())?, i)
                .map_err(publish_err)?;
        }
        let mut known_relations = work_map(owned_budget.as_ref());
        if unchanged {
            for row in snapshot.relations.iter() {
                s.native_map_room(&mut known_relations)
                    .map_err(publish_err)?;
                known_relations
                    .insert(digest(&row.generated)?, ())
                    .map_err(publish_err)?;
            }
        }
        let mut candidates = bj::new_work(owned_budget.as_ref());
        for b in &generated.relations {
            if known_relations.get(&b.digest).is_some() {
                continue;
            }
            s.native_codec_room(&mut candidates, 1)
                .map_err(publish_err)?;
            candidates.try_push(built_ref(s, b)?).map_err(publish_err)?;
        }
        for r in replaces {
            s.native_codec_room(&mut candidates, 1)
                .map_err(publish_err)?;
            candidates
                .try_push(replacement_relation(s, r)?)
                .map_err(publish_err)?;
        }
        // 旧前缀的 supersedes 关系已由前代出生入旧快照；只扫本代新增原始事件（Δ_R）。
        let supersedes_from = if unchanged {
            (previous_frontier + 1) as usize
        } else {
            0
        };
        for n in supersedes_from..=frontier as usize {
            let event = raw.get(n).ok_or_else(|| publish_err("原始前缀越界"))?;
            if let Some(r) = supersedes_relation(s, event)? {
                s.native_codec_room(&mut candidates, 1)
                    .map_err(publish_err)?;
                candidates.try_push(r).map_err(publish_err)?;
            }
        }
        for r in candidates {
            let row = relation(s, r, g)?;
            if let Some(i) = rs.get(&row.id) {
                if snapshot.relations[*i].generated != row.generated {
                    return Err(publish_err("同关系身份换内容"));
                }
            } else {
                s.native_map_room(&mut rs).map_err(publish_err)?;
                rs.insert(
                    row.id.try_clone_admitted(s.codec_work())?,
                    snapshot.relations.len(),
                )
                .map_err(publish_err)?;
                s.native_codec_room(&mut snapshot.relations, 1)
                    .map_err(publish_err)?;
                snapshot
                    .relations
                    .try_push(row.try_clone_admitted(s)?)
                    .map_err(publish_err)?;
                s.native_codec_room(&mut births.relations, 1)
                    .map_err(publish_err)?;
                births.relations.try_push(row).map_err(publish_err)?;
            }
        }
        let mut obs = work_map(owned_budget.as_ref());
        for (i, row) in snapshot.observations.iter().enumerate() {
            s.native_map_room(&mut obs).map_err(publish_err)?;
            obs.insert(row.id.try_clone_admitted(s.codec_work())?, i)
                .map_err(publish_err)?;
        }
        let mut known_observations = work_map(owned_budget.as_ref());
        if unchanged {
            for row in snapshot.observations.iter() {
                s.native_map_room(&mut known_observations)
                    .map_err(publish_err)?;
                known_observations
                    .insert(digest(&row.generated)?, ())
                    .map_err(publish_err)?;
            }
        }
        for b in &generated.observations {
            if known_observations.get(&b.digest).is_some() {
                continue;
            }
            let row = self.observation(s, b, g)?;
            if let Some(i) = obs.get(&row.id) {
                // 旧非 tb02a 配方不把 detail 纳入身份：重现保留首版，绝不覆盖；
                // 对哈希身份字段逐值核对，tb02a 全内容身份还须整根相同。
                let prior = &snapshot.observations[*i];
                if observation_identity_value(s, &prior.generated)?
                    != observation_identity_value(s, &row.generated)?
                {
                    return Err(publish_err("同观察身份的身份字段冲突"));
                }
            } else {
                s.native_map_room(&mut obs).map_err(publish_err)?;
                obs.insert(
                    row.id.try_clone_admitted(s.codec_work())?,
                    snapshot.observations.len(),
                )
                .map_err(publish_err)?;
                s.native_codec_room(&mut snapshot.observations, 1)
                    .map_err(publish_err)?;
                snapshot
                    .observations
                    .try_push(row.try_clone_admitted(s)?)
                    .map_err(publish_err)?;
                s.native_codec_room(&mut births.observations, 1)
                    .map_err(publish_err)?;
                births.observations.try_push(row).map_err(publish_err)?;
            }
        }
        stable_sort(
            &mut snapshot.witnesses,
            |a, b| witness_rank(layout, a).cmp(&witness_rank(layout, b)),
            owned_budget.as_ref(),
            s,
        )?;
        stable_sort(
            &mut snapshot.relations,
            |a, b| relation_rank(layout, a).cmp(&relation_rank(layout, b)),
            owned_budget.as_ref(),
            s,
        )?;
        stable_sort(
            &mut snapshot.observations,
            |a, b| obs_rank(layout, a).cmp(&obs_rank(layout, b)),
            owned_budget.as_ref(),
            s,
        )?;
        let mut raw_nodes = bj::new_work(owned_budget.as_ref());
        for (n, event) in raw.prefix(frontier as usize + 1)?.iter().enumerate() {
            if bj::field(event, "seq").as_i64() != Some(n as i64) {
                return Err(publish_err("原始序号与输入前缀不符"));
            }
            let key: NodeDigest = bj::sha_json(event, raw.budget())?;
            let wire = if let Some(d) = self.raw_wires.get(&key, s.allocation_budget().as_ref())? {
                *d
            } else {
                let d = s
                    .write_json_view(bj::RawWire::new(event)?.view())
                    .map_err(publish_err)?
                    .digest;
                self.raw_wires
                    .insert(key, d, s.allocation_budget().as_ref())?;
                d
            };
            raw_nodes.try_push(wire).map_err(publish_err)?;
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

#[cfg(test)]
thread_local! { static FULL_PLAN_FOR_TEST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// 测试离线对照臂：置位后 plan 走旧完整重算（逐行回读、全原始前缀扫 supersedes）。
#[cfg(test)]
pub(super) fn set_full_plan_for_test(full: bool) {
    FULL_PLAN_FOR_TEST.with(|f| f.set(full));
}
fn incremental_plan() -> bool {
    #[cfg(test)]
    {
        !FULL_PLAN_FOR_TEST.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        true
    }
}

fn project_bar(
    s: &mut (impl CodecStore + ?Sized),
    bar: JsonView<'_>,
) -> Result<Built, PublishError> {
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
        s.native_codec_room(&mut fields, 1).map_err(publish_err)?;
        fields.try_push((key, value)).map_err(publish_err)?;
    }
    let extra = if ohlc {
        &["schema_revision", "open", "high", "low", "close"][..]
    } else {
        &[]
    };
    for key in ["seq"].into_iter().chain(extra.iter().copied()) {
        if object.get(key).is_none() {
            s.native_codec_room(&mut fields, 1).map_err(publish_err)?;
            fields
                .try_push((
                    key,
                    if key == "seq" {
                        JsonView::String(seq.text())
                    } else {
                        JsonView::Null
                    },
                ))
                .map_err(publish_err)?;
        }
    }
    write_fields(s, &fields)
}
fn replacement_relation(
    s: &mut (impl CodecStore + ?Sized),
    replacement: JsonView<'_>,
) -> Result<RefValue, PublishError> {
    let subject = bj::field(replacement, "new_object_id")
        .as_str()
        .ok_or_else(|| publish_err("replaces缺new_object_id"))?;
    let object = bj::field(replacement, "old_object_id")
        .as_str()
        .ok_or_else(|| publish_err("replaces缺old_object_id"))?;
    let fields = [
        ("subject", JsonView::String(subject)),
        ("relation_type", JsonView::String("replaces")),
        ("object", JsonView::String(object)),
    ];
    let b = write_fields(s, &fields)?;
    Ok(built_ref(s, &b)?)
}
fn supersedes_relation(
    s: &mut (impl CodecStore + ?Sized),
    event: JsonView<'_>,
) -> Result<Option<RefValue>, PublishError> {
    let Some(sup) = bj::field(event, "supersedes_revision").as_i64() else {
        return Ok(None);
    };
    let identity = bj::field(event, "identity_key")
        .as_str()
        .ok_or_else(|| publish_err("源身份缺失"))?;
    let rev = bj::field(event, "revision")
        .as_i64()
        .ok_or_else(|| publish_err("源版本缺失"))?;
    let budget = s.allocation_budget();
    let subject = fmt_text(s, format_args!("{identity}@{rev}"), budget.as_ref())?;
    let object = fmt_text(s, format_args!("{identity}@{sup}"), budget.as_ref())?;
    let fields = [
        ("subject", JsonView::String(&subject)),
        ("relation_type", JsonView::String("supersedes")),
        ("object", JsonView::String(&object)),
    ];
    let b = write_fields(s, &fields)?;
    Ok(Some(built_ref(s, &b)?))
}
fn public_sets(
    s: &mut (impl CodecStore + ?Sized),
    i: &Index,
    raw: &[NodeDigest],
    memo: &mut [SequenceMemo],
) -> Result<PublicSets, PublishError> {
    let budget = s.allocation_budget();
    let mut w = bj::new_work(budget.as_ref());
    let mut r = bj::new_work(budget.as_ref());
    let mut o = bj::new_work(budget.as_ref());
    for row in &i.witnesses {
        w.try_push(digest(&row.wire)?).map_err(publish_err)?;
    }
    for row in &i.relations {
        r.try_push(digest(&row.generated)?).map_err(publish_err)?;
    }
    for row in &i.observations {
        o.try_push(digest(&row.wire)?).map_err(publish_err)?;
    }
    Ok(PublicSets {
        witnesses: memo[0].sync(s.generic_store(), &w, "见证快照", budget.as_ref())?,
        relations: memo[1].sync(s.generic_store(), &r, "关系快照", budget.as_ref())?,
        observations: memo[2].sync(s.generic_store(), &o, "观察快照", budget.as_ref())?,
        raw_history: memo[3].sync(s.generic_store(), raw, "原始历史", budget.as_ref())?,
    })
}
fn window(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
    k: &str,
) -> Result<Option<i64>, PublishError> {
    let v = scalar(s, r, k)?;
    if v.view().is_null() {
        Ok(None)
    } else {
        v.view()
            .as_i64()
            .map(Some)
            .ok_or_else(|| publish_err("窗口不是i64/null"))
    }
}
fn observation_wire(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
    id: &str,
    batch: Option<&str>,
) -> Result<RefValue, PublishError> {
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
    let b = s
        .write_template_view(VV::Object(&fields))
        .map_err(publish_err)?;
    Ok(built_ref(s, &b)?)
}
fn relation(
    s: &mut (impl CodecStore + ?Sized),
    generated: RefValue,
    g: i64,
) -> Result<Relation, PublishError> {
    audit_count(2);
    s.verify_many_ref_fresh([generated.view()], sc::Mode::Public)
        .map_err(publish_err)?;
    exact(&generated, 3)?;
    let subject = text(s, &generated, "subject")?;
    let relation_type = text(s, &generated, "relation_type")?;
    let object = text(s, &generated, "object")?;
    let fields = [
        ("s", JsonView::String(&subject)),
        ("t", JsonView::String(&relation_type)),
        ("o", JsonView::String(&object)),
    ];
    let hash = bj::sha_json_admitted(s, JsonView::object_fields(&fields).map_err(publish_err)?)?;
    let hash = sc::FixedRoot::new(&hash);
    let id = fmt_text(
        s,
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
        #[cfg(test)]
        super::state::trace_local_sha256(0, b.len());
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn stream_hash(
    s: &mut (impl CodecRead + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<TextValue, PublishError> {
    let mut out = HashWriter(Sha256::new());
    s.render_ref(r.view(), &mut out).map_err(publish_err)?;
    #[cfg(test)]
    super::state::trace_local_sha256(1, 0);
    let hash = sc::FixedRoot::new(&out.0.finalize().into());
    s.native_text(&hash.as_str()[4..]).map_err(publish_err)
}
fn verify_witness_id(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
    id: &str,
    object: &str,
    slot: i64,
    mi: i64,
) -> Result<(), PublishError> {
    if mi < 0 {
        return Err(publish_err("见证合并序号非法"));
    }
    let mi = serde_json::Number::from(mi);
    let slot = serde_json::Number::from(slot);
    let fields = [
        ("mi", VV::Json(JsonView::Number(&mi))),
        ("object_id", VV::String(object)),
        ("slot", VV::Json(JsonView::Number(&slot))),
        ("raw_bars", edge(s, r, "raw_bars")?),
    ];
    let b = s
        .write_value_view(VV::Object(&fields))
        .map_err(publish_err)?;
    let recipe = built_ref(s, &b)?;
    let hash = stream_hash(s, &recipe)?;
    let expected = fmt_text(
        s,
        format_args!("wit-{}", &hash[..16]),
        s.allocation_budget().as_ref(),
    )?;
    if id != expected.as_str() {
        return Err(publish_err("见证身份配方不符"));
    }
    Ok(())
}
fn observation_identity_value(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<RefValue, PublishError> {
    let detail = field(s, r, "detail")?;
    if detail.kind() != Kind::Object {
        return Err(publish_err("观察detail不是对象"));
    }
    let tb = match s
        .lookup_optional_field_ref(detail.view(), "tb02a")
        .map_err(publish_err)?
    {
        Some(value) => {
            value.kind() == Kind::Bool && render(s, &value)?.view().as_bool() == Some(true)
        }
        None => false,
    };
    if tb {
        return Ok(built_ref(
            s,
            &Built {
                digest: digest(r)?,
                kind: r.view().kind(),
                extent: r.view().extent(),
            },
        )?);
    }
    let fields = [
        ("kind", edge(s, r, "kind")?),
        ("reason", edge(s, r, "reason")?),
        ("window_start", edge(s, r, "window_start")?),
        ("window_mid", edge(s, r, "window_mid")?),
        ("window_end", edge(s, r, "window_end")?),
    ];
    let b = s
        .write_value_view(VV::Object(&fields))
        .map_err(publish_err)?;
    Ok(built_ref(s, &b)?)
}
fn observation_id(
    s: &mut (impl CodecStore + ?Sized),
    r: &(impl AuxRef + ?Sized),
) -> Result<TextValue, PublishError> {
    let recipe = observation_identity_value(s, r)?;
    let hash = stream_hash(s, &recipe)?;
    let full = recipe.view().root() == r.view().root()
        && recipe.view().kind() == r.view().kind()
        && recipe.view().extent() == r.view().extent();
    fmt_text(
        s,
        format_args!("obs-{}", if full { hash.as_str() } else { &hash[..16] }),
        s.allocation_budget().as_ref(),
    )
}

fn state_members(
    s: &mut (impl CodecRead + ?Sized),
    state: &(impl AuxRef + ?Sized),
    name: &str,
) -> Result<CodecVec<RefValue>, PublishError> {
    let members = field(s, state, name)?;
    array_elements(s, &members).map_err(Into::into)
}

#[cfg(test)]
thread_local! {static AUX_AUDIT_STATS:std::cell::Cell<[u64;6]>=const{std::cell::Cell::new([0;6])};}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
)]
pub(super) fn reset_aux_audit_stats() {
    AUX_AUDIT_STATS.with(|s| s.set([0; 6]));
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
)]
pub(super) fn aux_audit_stats() -> [u64; 6] {
    AUX_AUDIT_STATS.with(std::cell::Cell::get)
}
fn audit_count(slot: usize) {
    #[cfg(test)]
    AUX_AUDIT_STATS.with(|s| {
        let mut v = s.get();
        v[slot] += 1;
        s.set(v);
    });
    #[cfg(not(test))]
    let _ = slot;
}
#[derive(Debug, PartialEq, Eq, Hash)]
struct RelationRefKey {
    format: NativeValue<String>,
    root: NativeValue<String>,
    kind: Kind,
    extent: Option<u64>,
}
impl RelationRefKey {
    fn new(r: &RefValue, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self {
            format: store
                .native_string(r.view().format())
                .map_err(publish_err)?,
            root: store.native_string(r.view().root()).map_err(publish_err)?,
            kind: r.kind(),
            extent: r.extent(),
        })
    }
}
struct RelationCore {
    id: TextValue,
    subject: TextValue,
    relation_type: TextValue,
    object: TextValue,
    generated: RefValue,
}
impl RelationCore {
    fn at(&self, g: i64, admission: &(impl CodecRead + ?Sized)) -> Result<Relation, PublishError> {
        Ok(Relation {
            id: self.id.try_clone_admitted(admission.codec_work())?,
            subject: self.subject.try_clone_admitted(admission.codec_work())?,
            relation_type: self
                .relation_type
                .try_clone_admitted(admission.codec_work())?,
            object: self.object.try_clone_admitted(admission.codec_work())?,
            generated: admission
                .native_ref_clone(&self.generated)
                .map_err(publish_err)?,
            first: g,
        })
    }
}
fn native_tables<T>(
    rows: &[T],
    id: fn(&T) -> &str,
    first: fn(&T) -> i64,
    store: &(impl CodecRead + ?Sized),
) -> Result<
    (
        bj::BusinessMap<NativeValue<String>, usize>,
        NativeTree<i64, CodecVec<usize>>,
        Option<i64>,
    ),
    String,
> {
    let mut map = bj::new_map(None);
    let mut born = NativeTree::<i64, CodecVec<usize>>::default();
    let mut duplicate = None;
    for (ordinal, row) in rows.iter().enumerate() {
        audit_count(0);
        audit_count(1);
        let name = id(row);
        let g = first(row);
        if !born.contains_key(&g) {
            born.insert_admitted(g, bj::new_work(None), store)
                .map_err(err)?;
        }
        let bucket = born.get_mut(&g).expect("已插入出生桶");
        store.native_codec_room(bucket, 1).map_err(err)?;
        bucket.try_push(ordinal).map_err(err)?;
        if let Some(previous) = map.get(name).copied() {
            let from = g.max(first(&rows[previous]));
            duplicate = Some(duplicate.map_or(from, |old: i64| old.min(from)));
            if g < first(&rows[previous]) {
                map.insert(store.native_string(name).map_err(err)?, ordinal)
                    .map_err(err)?;
            }
        } else {
            store.native_map_room(&mut map).map_err(err)?;
            map.insert(store.native_string(name).map_err(err)?, ordinal)
                .map_err(err)?;
        }
    }
    Ok((map, born, duplicate))
}
/// 完整身份表按原 SQL 行序保存稳定 ordinal；首次观察桶不借用索引行或身份文本。
pub(super) struct NativeAuxFacts {
    witnesses: bj::BusinessMap<NativeValue<String>, usize>,
    relations: bj::BusinessMap<NativeValue<String>, usize>,
    observations: bj::BusinessMap<NativeValue<String>, usize>,
    born: [NativeTree<i64, CodecVec<usize>>; 3],
    duplicates: [Option<i64>; 3],
    raw_scanned: usize,
    raw_revisions: CodecVec<usize>,
}
/// 本事务 facade 绑定原件及索引；纯 codec memo 随 facade 终结，不移交为当前性资格。
pub(super) struct NativeAuxAudit<'index, 'capture, 'raw> {
    capture: &'capture sc::AuditCapture,
    index: &'index Index,
    raw: RawEventsView<'raw>,
    facts: NativeAuxFacts,
    #[cfg(feature = "s_incremental")]
    retained_raw_capacity: usize,
    relation_memo: bj::BusinessMap<RelationRefKey, RelationCore>,
    raw_memo: bj::BusinessMap<usize, Option<RefValue>>,
}
impl std::ops::Deref for NativeAuxAudit<'_, '_, '_> {
    type Target = NativeAuxFacts;
    fn deref(&self) -> &Self::Target {
        &self.facts
    }
}
impl std::ops::DerefMut for NativeAuxAudit<'_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.facts
    }
}
fn same_birth_order<'a>(
    generated: &[TextValue],
    actual: impl ExactSizeIterator<Item = &'a str>,
) -> bool {
    generated.len() == actual.len() && generated.iter().zip(actual).all(|(a, b)| a.as_str() == b)
}
impl<'index, 'capture, 'raw> NativeAuxAudit<'index, 'capture, 'raw> {
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
    )]
    pub(super) fn from_loaded(
        s: &super::history::HashReader<'capture>,
        index: &'index Index,
        raw: RawEventsView<'raw>,
    ) -> Option<Self> {
        Self::from_loaded_in(s, index, raw, &super::audit_store::NativeAdmission(None))
            .expect("无scope的Native辅助表构造")
    }
    pub(super) fn from_loaded_admitted(
        s: &super::history::HashReader<'capture>,
        index: &'index Index,
        raw: RawEventsView<'raw>,
    ) -> Result<Option<Self>, String> {
        Self::from_loaded_in(s, index, raw, s)
    }
    fn from_loaded_in(
        s: &super::history::HashReader<'capture>,
        index: &'index Index,
        raw: RawEventsView<'raw>,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<Option<Self>, String> {
        if index.budget.is_some() || raw.budget().is_some() {
            return Ok(None);
        }
        let Some(capture) = s.native_projection_capture() else {
            return Ok(None);
        };
        let (w, bw, dw) =
            native_tables(&index.witnesses, |r| r.id.as_str(), |r| r.first, admission)?;
        let (r, br, dr) =
            native_tables(&index.relations, |r| r.id.as_str(), |r| r.first, admission)?;
        let (o, bo, do_) = native_tables(
            &index.observations,
            |r| r.id.as_str(),
            |r| r.first,
            admission,
        )?;
        Ok(Some(Self {
            capture,
            index,
            raw,
            facts: NativeAuxFacts {
                witnesses: w,
                relations: r,
                observations: o,
                born: [bw, br, bo],
                duplicates: [dw, dr, do_],
                raw_scanned: 0,
                raw_revisions: bj::new_work(None),
            },
            #[cfg(feature = "s_incremental")]
            retained_raw_capacity: 0,
            relation_memo: bj::new_map(None),
            raw_memo: bj::new_map(None),
        }))
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn restore(
        s: &super::history::HashReader<'capture>,
        index: &'index Index,
        raw: RawEventsView<'raw>,
        facts: NativeAuxFacts,
        _bound: &super::audit_owner::BoundOriginals,
    ) -> Result<Self, String> {
        if facts.raw_scanned > raw.len() {
            return Err(err("aux raw 前沿超过绑定原件"));
        }
        Ok(Self {
            capture: s
                .native_projection_capture()
                .ok_or_else(|| err("aux 缺 Native owner"))?,
            index,
            raw,
            retained_raw_capacity: facts.raw_revisions.capacity(),
            facts,
            relation_memo: bj::new_map(None),
            raw_memo: bj::new_map(None),
        })
    }
    pub(super) fn audit(
        &mut self,
        s: &mut super::history::HashReader<'_>,
        state: &(impl AuxRef + ?Sized),
        replaces: JsonView<'_>,
        prefix: usize,
        g: i64,
    ) -> Result<(), PublishError> {
        let result = (|| {
            if !s
                .native_projection_capture()
                .is_some_and(|c| std::ptr::eq(c, self.capture))
            {
                return Err(publish_err("Native aux owner capture/权限不符"));
            }
            let raw = self.raw.prefix(prefix)?;
            let index = self.index;
            verify_generated_members_in_view(s, index, state, replaces, raw, g, Some(self))
        })();
        if result.is_err() {
            self.relation_memo.retain(|_, _| false);
            self.raw_memo.retain(|_, _| false);
            self.raw_scanned = 0;
            self.raw_revisions.clear();
        }
        result
    }
    fn duplicate(&self, role: usize, g: i64) -> bool {
        self.duplicates[role].is_some_and(|first| first <= g)
    }
    pub(super) fn into_facts(self) -> NativeAuxFacts {
        self.facts
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn into_facts_retained(
        self,
        delta: &mut super::audit_retention::RetainedDelta,
    ) -> NativeAuxFacts {
        delta.vec_capacity::<usize>(
            self.retained_raw_capacity,
            self.facts.raw_revisions.capacity(),
        );
        self.facts
    }

    fn birth_names(&self, role: usize, g: i64) -> impl ExactSizeIterator<Item = &str> {
        self.born[role]
            .get(&g)
            .map(|v| &v[..])
            .unwrap_or(&[])
            .iter()
            .map(move |ordinal| match role {
                0 => self.index.witnesses[*ordinal].id.as_str(),
                1 => self.index.relations[*ordinal].id.as_str(),
                2 => self.index.observations[*ordinal].id.as_str(),
                _ => unreachable!("固定辅助族"),
            })
    }
    fn relation(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        candidate: RefValue,
        g: i64,
    ) -> Result<Relation, PublishError> {
        if !self
            .capture
            .contains(&candidate.digest().map_err(publish_err)?)
        {
            return relation(s, candidate, g);
        }
        s.native_verify_original(self.capture, candidate.view(), sc::Mode::Public)
            .map_err(publish_err)?;
        let key = RelationRefKey::new(&candidate, s)?;
        if let Some(core) = self.relation_memo.get(&key) {
            return core.at(g, s);
        }
        let parsed = relation(s, candidate, g)?;
        let core = RelationCore {
            id: parsed.id,
            subject: parsed.subject,
            relation_type: parsed.relation_type,
            object: parsed.object,
            generated: parsed.generated,
        };
        let result = core.at(g, s)?;
        s.native_map_room(&mut self.relation_memo)
            .map_err(publish_err)?;
        self.relation_memo.insert(key, core).map_err(publish_err)?;
        Ok(result)
    }
    fn raw_relation(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        ordinal: usize,
    ) -> Result<Option<RefValue>, PublishError> {
        if let Some(value) = self.raw_memo.get(&ordinal) {
            if let Some(r) = value {
                s.native_verify_original(self.capture, r.view(), sc::Mode::Public)
                    .map_err(publish_err)?;
            }
            return value
                .as_ref()
                .map(|r| s.native_ref_clone(r).map_err(publish_err))
                .transpose();
        }
        let event = self
            .raw
            .get(ordinal)
            .ok_or_else(|| publish_err("raw ordinal越界"))?;
        let value = supersedes_relation(s, event)?;
        let eligible = match &value {
            None => true,
            Some(r) => {
                if self.capture.contains(&r.digest().map_err(publish_err)?) {
                    s.native_verify_original(self.capture, r.view(), sc::Mode::Public)
                        .map_err(publish_err)?;
                    true
                } else {
                    false
                }
            }
        };
        if eligible {
            s.native_map_room(&mut self.raw_memo).map_err(publish_err)?;
            self.raw_memo
                .insert(
                    ordinal,
                    value
                        .as_ref()
                        .map(|r| s.native_ref_clone(r).map_err(publish_err))
                        .transpose()?,
                )
                .map_err(publish_err)?;
        }
        Ok(value)
    }
    fn advance_raw(
        &mut self,
        prefix: usize,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<usize, PublishError> {
        self.raw.prefix(prefix)?;
        // 原件只实际读一遍；历史回退取 ordinal 前缀，不把未来修订写回旧 cut。
        for ordinal in self.raw_scanned..prefix {
            audit_count(5);
            let event = self
                .raw
                .get(ordinal)
                .ok_or_else(|| publish_err("raw ordinal越界"))?;
            if !bj::field(event, "supersedes_revision").is_null() {
                admission
                    .native_codec_room(&mut self.facts.raw_revisions, 1)
                    .map_err(publish_err)?;
                self.raw_revisions.try_push(ordinal).map_err(publish_err)?;
            }
        }
        self.raw_scanned = self.raw_scanned.max(prefix);
        Ok(self
            .raw_revisions
            .partition_point(|ordinal| *ordinal < prefix))
    }
}

/// 生成态与见证/关系/观察独立索引的来源绑定核验。
///
/// 这里只消费调用方已核过的原始前缀；不重跑结构分类，也不展开观察 detail 或 raw_bars
/// 的整段 JSON。生成态当前成员必须能在独立索引找到并逐根相等；本代首次成员则另核
/// 独立索引的首次插入顺序，避免只凭索引自洽放过出生漏项、幽灵行或错绑原件。
#[cfg(test)]
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
    verify_generated_members_in_view(
        s,
        index,
        state,
        JsonView::BorrowedArray(&replaces),
        raw,
        g,
        None,
    )
    .map_err(newchan_rust::s_content::ContextError::into_message)
}
pub(super) fn verify_generated_members_with_changes<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &Index,
    state: &(impl AuxRef + ?Sized),
    replaces: JsonView<'_>,
    raw: impl Into<RawEventsView<'a>>,
    g: i64,
) -> Result<(), PublishError> {
    verify_generated_members_in_view(s, index, state, replaces, raw.into(), g, None)
}
fn verify_generated_members_in_view(
    s: &mut (impl CodecStore + ?Sized),
    index: &Index,
    state: &(impl AuxRef + ?Sized),
    replaces: JsonView<'_>,
    raw: RawEventsView<'_>,
    g: i64,
    mut native: Option<&mut NativeAuxAudit<'_, '_, '_>>,
) -> Result<(), PublishError> {
    let owned_budget = s.allocation_budget();
    if g < 1 {
        return Err(publish_err("生成代非法"));
    }

    let mut witness_by_id = work_map::<&str, &Witness>(owned_budget.as_ref());
    if let Some(owner) = native.as_ref() {
        if owner.duplicate(0, g) {
            return Err(publish_err("见证独立索引重复身份"));
        }
    } else {
        for row in &index.witnesses {
            audit_count(0);
            if row.first <= g {
                s.native_map_room(&mut witness_by_id).map_err(publish_err)?;
                if witness_by_id
                    .insert(row.id.as_str(), row)
                    .map_err(publish_err)?
                    .is_some()
                {
                    return Err(publish_err("见证独立索引重复身份"));
                }
            }
        }
    }
    let generated_witnesses = state_members(s, state, "witnesses")?;
    let mut seen = work_map(owned_budget.as_ref());
    let mut generated_births = bj::new_work(owned_budget.as_ref());
    for generated in generated_witnesses {
        audit_count(3);
        let id = text(s, &generated, "witness_id")?;
        let generated_digest = digest(&generated)?;
        s.native_map_room(&mut seen).map_err(publish_err)?;
        if seen
            .insert(s.native_text(id.as_str())?, generated_digest)
            .map_err(publish_err)?
            .is_some()
        {
            return Err(publish_err("生成见证重复身份"));
        }
        let row = if let Some(owner) = native.as_ref() {
            owner
                .witnesses
                .get(id.as_str())
                .map(|ordinal| &owner.index.witnesses[*ordinal])
                .filter(|r| r.first <= g)
        } else {
            witness_by_id.get(&id.as_str()).copied()
        }
        .ok_or_else(|| publish_err("生成见证不在独立索引内"))?;
        if row.generated != generated {
            return Err(publish_err("生成见证原件与独立索引不一致"));
        }
        if row.first == g {
            s.native_codec_room(&mut generated_births, 1)
                .map_err(publish_err)?;
            generated_births.try_push(id).map_err(publish_err)?;
        }
    }
    let mut indexed_births = bj::new_work(owned_budget.as_ref());
    if native.is_none() {
        for row in index.witnesses.iter().filter(|row| {
            audit_count(1);
            row.first == g
        }) {
            s.native_codec_room(&mut indexed_births, 1)
                .map_err(publish_err)?;
            indexed_births
                .try_push(row.id.as_str())
                .map_err(publish_err)?;
        }
    }
    audit_count(4);
    let births_match = if let Some(owner) = native.as_ref() {
        same_birth_order(&generated_births, owner.birth_names(0, g))
    } else {
        same_birth_order(&generated_births, indexed_births.iter().copied())
    };
    if !births_match {
        return Err(publish_err("见证出生集合或顺序与独立索引不同"));
    }

    let mut relation_by_id = work_map::<&str, &Relation>(owned_budget.as_ref());
    if let Some(owner) = native.as_ref() {
        if owner.duplicate(1, g) {
            return Err(publish_err("关系独立索引重复身份"));
        }
    } else {
        for row in &index.relations {
            audit_count(0);
            if row.first <= g {
                s.native_map_room(&mut relation_by_id)
                    .map_err(publish_err)?;
                if relation_by_id
                    .insert(row.id.as_str(), row)
                    .map_err(publish_err)?
                    .is_some()
                {
                    return Err(publish_err("关系独立索引重复身份"));
                }
            }
        }
    }
    let mut relation_candidates = state_members(s, state, "relations")?;
    for replacement in replaces
        .as_array()
        .ok_or_else(|| publish_err("replaces不是数组"))?
        .iter()
    {
        s.native_codec_room(&mut relation_candidates, 1)
            .map_err(publish_err)?;
        relation_candidates
            .try_push(replacement_relation(s, replacement)?)
            .map_err(publish_err)?;
    }
    if let Some(owner) = native.as_deref_mut() {
        let count = owner.advance_raw(raw.len(), s)?;
        for i in 0..count {
            let ordinal = owner.raw_revisions[i];
            if let Some(r) = owner.raw_relation(s, ordinal)? {
                s.native_codec_room(&mut relation_candidates, 1)
                    .map_err(publish_err)?;
                relation_candidates.try_push(r).map_err(publish_err)?;
            }
        }
    } else {
        for event in raw {
            audit_count(5);
            if let Some(r) = supersedes_relation(s, event)? {
                s.native_codec_room(&mut relation_candidates, 1)
                    .map_err(publish_err)?;
                relation_candidates.try_push(r).map_err(publish_err)?;
            }
        }
    }
    let mut relation_seen = work_map(owned_budget.as_ref());
    let mut relation_births = bj::new_work(owned_budget.as_ref());
    for candidate in relation_candidates {
        audit_count(3);
        let row = if let Some(owner) = native.as_deref_mut() {
            owner.relation(s, candidate, g)?
        } else {
            relation(s, candidate, g)?
        };
        let row_digest = digest(&row.generated)?;
        s.native_map_room(&mut relation_seen).map_err(publish_err)?;
        if let Some(previous) = relation_seen
            .insert(s.native_text(row.id.as_str())?, row_digest)
            .map_err(publish_err)?
        {
            if previous != row_digest {
                return Err(publish_err("同关系身份对应多个生成原件"));
            }
            continue;
        }
        let stored = if let Some(owner) = native.as_ref() {
            owner
                .relations
                .get(row.id.as_str())
                .map(|ordinal| &owner.index.relations[*ordinal])
                .filter(|r| r.first <= g)
        } else {
            relation_by_id.get(&row.id.as_str()).copied()
        }
        .ok_or_else(|| publish_err("生成关系不在独立索引内"))?;
        if stored.generated != row.generated {
            return Err(publish_err("生成关系原件与独立索引不一致"));
        }
        if stored.first == g {
            s.native_codec_room(&mut relation_births, 1)
                .map_err(publish_err)?;
            relation_births.try_push(row.id).map_err(publish_err)?;
        }
    }
    let mut indexed_relation_births = bj::new_work(owned_budget.as_ref());
    if native.is_none() {
        for row in index.relations.iter().filter(|row| {
            audit_count(1);
            row.first == g
        }) {
            s.native_codec_room(&mut indexed_relation_births, 1)
                .map_err(publish_err)?;
            indexed_relation_births
                .try_push(row.id.as_str())
                .map_err(publish_err)?;
        }
    }
    audit_count(4);
    let births_match = if let Some(owner) = native.as_ref() {
        same_birth_order(&relation_births, owner.birth_names(1, g))
    } else {
        same_birth_order(&relation_births, indexed_relation_births.iter().copied())
    };
    if !births_match {
        return Err(publish_err("关系出生集合或顺序与独立索引不同"));
    }

    let mut observation_by_id = work_map::<&str, &Observation>(owned_budget.as_ref());
    if let Some(owner) = native.as_ref() {
        if owner.duplicate(2, g) {
            return Err(publish_err("观察独立索引重复身份"));
        }
    } else {
        for row in &index.observations {
            audit_count(0);
            if row.first <= g {
                s.native_map_room(&mut observation_by_id)
                    .map_err(publish_err)?;
                if observation_by_id
                    .insert(row.id.as_str(), row)
                    .map_err(publish_err)?
                    .is_some()
                {
                    return Err(publish_err("观察独立索引重复身份"));
                }
            }
        }
    }
    let generated_observations = state_members(s, state, "observations")?;
    let mut observation_seen = work_map(owned_budget.as_ref());
    let mut observation_births = bj::new_work(owned_budget.as_ref());
    for generated in generated_observations {
        audit_count(3);
        let id = observation_id(s, &generated)?;
        s.native_map_room(&mut observation_seen)
            .map_err(publish_err)?;
        if observation_seen
            .insert(s.native_text(id.as_str())?, ())
            .map_err(publish_err)?
            .is_some()
        {
            return Err(publish_err("生成观察重复身份"));
        }
        let stored = if let Some(owner) = native.as_ref() {
            owner
                .observations
                .get(id.as_str())
                .map(|ordinal| &owner.index.observations[*ordinal])
                .filter(|r| r.first <= g)
        } else {
            observation_by_id.get(&id.as_str()).copied()
        }
        .ok_or_else(|| publish_err("生成观察不在独立索引内"))?;
        if observation_identity_value(s, &stored.generated)?
            != observation_identity_value(s, &generated)?
        {
            return Err(publish_err("生成观察身份字段与独立首版不一致"));
        }
        if stored.first == g {
            if stored.generated != generated {
                return Err(publish_err("观察出生原件与独立首版不一致"));
            }
            s.native_codec_room(&mut observation_births, 1)
                .map_err(publish_err)?;
            observation_births.try_push(id).map_err(publish_err)?;
        }
    }
    let mut indexed_observation_births = bj::new_work(owned_budget.as_ref());
    if native.is_none() {
        for row in index.observations.iter().filter(|row| {
            audit_count(1);
            row.first == g
        }) {
            s.native_codec_room(&mut indexed_observation_births, 1)
                .map_err(publish_err)?;
            indexed_observation_births
                .try_push(row.id.as_str())
                .map_err(publish_err)?;
        }
    }
    audit_count(4);
    let births_match = if let Some(owner) = native.as_ref() {
        same_birth_order(&observation_births, owner.birth_names(2, g))
    } else {
        same_birth_order(
            &observation_births,
            indexed_observation_births.iter().copied(),
        )
    };
    if !births_match {
        return Err(publish_err("观察出生集合或顺序与独立索引不同"));
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
        let entry = audit_persisted_witness(&mut row, s, latest, owned_budget.as_ref(), &mut rt)
            .map_err(newchan_rust::s_content::ContextError::into_message)?;
        s.native_codec_room(&mut out.witnesses, 1).map_err(err)?;
        out.witnesses.try_push(entry).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT relation_id,subject,relation_type,object,generated_root,published_generation FROM s_content_relations ORDER BY rowid").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let entry = audit_persisted_relation(&mut row, s, latest, owned_budget.as_ref())
            .map_err(newchan_rust::s_content::ContextError::into_message)?;
        s.native_codec_room(&mut out.relations, 1).map_err(err)?;
        out.relations.try_push(entry).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT observation_id,kind,window_start,batch_id,generated_root,birth_wire_root,published_generation FROM s_content_observations ORDER BY rowid").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let entry = audit_persisted_observation(&mut row, s, latest, owned_budget.as_ref())
            .map_err(newchan_rust::s_content::ContextError::into_message)?;
        s.native_codec_room(&mut out.observations, 1).map_err(err)?;
        out.observations.try_push(entry).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    Ok(out)
}
#[cfg(feature = "s_incremental")]
pub(super) fn update_from_owner(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'_>,
    latest: i64,
    index: &mut Index,
    facts: &mut NativeAuxFacts,
    rowids: &[Vec<i64>; 3],
    _bound: &super::audit_owner::BoundOriginals,
    retained: &mut super::audit_retention::RetainedDelta,
) -> Result<(), PublishError> {
    let mut rt = Runtime::default();
    for role in 0..3 {
        for rowid in &rowids[role] {
            let sql=match role {
                0=>c"SELECT witness_id,object_id,slot,generated_root,wire_root,published_generation,merged_index FROM s_content_witnesses WHERE rowid=?1",
                1=>c"SELECT relation_id,subject,relation_type,object,generated_root,published_generation FROM s_content_relations WHERE rowid=?1",
                _=>c"SELECT observation_id,kind,window_start,batch_id,generated_root,birth_wire_root,published_generation FROM s_content_observations WHERE rowid=?1",
            };
            let mut stmt = CheckedStatement::prepare(tx, sql).map_err(publish_err)?;
            stmt.bind_integer(1, *rowid).map_err(publish_err)?;
            let mut row = stmt
                .next()
                .map_err(publish_err)?
                .ok_or_else(|| publish_err("缺 aux 实际行"))?;
            let (id, ordinal, first) = match role {
                0 => {
                    let e = audit_persisted_witness(&mut row, s, latest, None, &mut rt)?;
                    let out = (
                        s.native_string(e.id.as_str()).map_err(publish_err)?,
                        index.witnesses.len(),
                        e.first,
                    );
                    retained.add(&e);
                    let capacity = index.witnesses.capacity();
                    s.native_codec_room(&mut index.witnesses, 1)
                        .map_err(publish_err)?;
                    index.witnesses.try_push(e).map_err(publish_err)?;
                    retained.vec_capacity::<Witness>(capacity, index.witnesses.capacity());
                    out
                }
                1 => {
                    let e = audit_persisted_relation(&mut row, s, latest, None)?;
                    let out = (
                        s.native_string(e.id.as_str()).map_err(publish_err)?,
                        index.relations.len(),
                        e.first,
                    );
                    retained.add(&e);
                    let capacity = index.relations.capacity();
                    s.native_codec_room(&mut index.relations, 1)
                        .map_err(publish_err)?;
                    index.relations.try_push(e).map_err(publish_err)?;
                    retained.vec_capacity::<Relation>(capacity, index.relations.capacity());
                    out
                }
                _ => {
                    let e = audit_persisted_observation(&mut row, s, latest, None)?;
                    let out = (
                        s.native_string(e.id.as_str()).map_err(publish_err)?,
                        index.observations.len(),
                        e.first,
                    );
                    retained.add(&e);
                    let capacity = index.observations.capacity();
                    s.native_codec_room(&mut index.observations, 1)
                        .map_err(publish_err)?;
                    index.observations.try_push(e).map_err(publish_err)?;
                    retained.vec_capacity::<Observation>(capacity, index.observations.capacity());
                    out
                }
            };
            drop(row);
            stmt.finalize().map_err(publish_err)?;
            let map = match role {
                0 => &mut facts.witnesses,
                1 => &mut facts.relations,
                _ => &mut facts.observations,
            };
            let capacity = map.capacity();
            retained.add(&id);
            s.native_map_room(map).map_err(publish_err)?;
            if map.insert(id, ordinal).map_err(publish_err)?.is_some() {
                return Err(publish_err("辅助索引重复身份"));
            }
            retained.hash_capacity::<NativeValue<String>, usize>(capacity, map.capacity());
            let tree_bytes = facts.born[role].allocation_bytes();
            if !facts.born[role].contains_key(&first) {
                facts.born[role]
                    .insert_admitted(first, bj::new_work(None), s)
                    .map_err(publish_err)?;
            }
            let bucket = facts.born[role].get_mut(&first).expect("已插入出生桶");
            let capacity = bucket.capacity();
            s.native_codec_room(bucket, 1).map_err(publish_err)?;
            bucket.try_push(ordinal).map_err(publish_err)?;
            retained.vec_capacity::<usize>(capacity, bucket.capacity());
            retained.release_bytes(tree_bytes);
            retained.add_bytes(facts.born[role].allocation_bytes());
        }
    }
    Ok(())
}

/// cold 与增量回读共用同一原行谓词，不接受插入参数冒充持久原件。
fn audit_persisted_witness(
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    s: &mut (impl CodecStore + ?Sized),
    latest: i64,
    allocation: Option<&OwnedBudget>,
    rt: &mut Runtime,
) -> Result<Witness, PublishError> {
    let id =
        TextValue::from_sql_admitted(row, 0, allocation, s.codec_work()).map_err(publish_err)?;
    let object_id =
        TextValue::from_sql_admitted(row, 1, allocation, s.codec_work()).map_err(publish_err)?;
    let slot: i64 = row.integer(2).map_err(publish_err)?;
    let merged_index: i64 = row.integer(6).map_err(publish_err)?;
    let generated = root(s, row.blob(3).map_err(publish_err)?)?;
    let wire = root(s, row.blob(4).map_err(publish_err)?)?;
    let first: i64 = row.integer(5).map_err(publish_err)?;
    check_generation(first, latest)?;
    exact(&generated, 9)?;
    if text(s, &generated, "witness_id")? != id
        || text(s, &generated, "object_id")? != object_id
        || integer(s, &generated, "slot")? != slot
        || !(0..3).contains(&slot)
    {
        return Err(publish_err("见证索引与原件不符"));
    }
    verify_witness_id(s, &generated, &id, &object_id, slot, merged_index)?;
    if rt.witness_wire(s, &generated)? != wire {
        return Err(publish_err("见证wire与生成原件不符"));
    }
    Ok(Witness {
        id,
        object_id,
        slot,
        merged_index,
        generated,
        wire,
        first,
    })
}

/// cold 与增量回读共用同一原行谓词，不接受插入参数冒充持久原件。
fn audit_persisted_relation(
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    s: &mut (impl CodecStore + ?Sized),
    latest: i64,
    _allocation: Option<&OwnedBudget>,
) -> Result<Relation, PublishError> {
    let first: i64 = row.integer(5).map_err(publish_err)?;
    check_generation(first, latest)?;
    let generated = root(s, row.blob(4).map_err(publish_err)?)?;
    let r = relation(s, generated, first)?;
    if r.id != row.text(0).map_err(publish_err)?
        || r.subject != row.text(1).map_err(publish_err)?
        || r.relation_type != row.text(2).map_err(publish_err)?
        || r.object != row.text(3).map_err(publish_err)?
    {
        return Err(publish_err("关系索引与原件不符"));
    }
    Ok(r)
}

/// cold 与增量回读共用同一原行谓词，不接受插入参数冒充持久原件。
fn audit_persisted_observation(
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    s: &mut (impl CodecStore + ?Sized),
    latest: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<Observation, PublishError> {
    let id =
        TextValue::from_sql_admitted(row, 0, allocation, s.codec_work()).map_err(publish_err)?;
    let kind =
        TextValue::from_sql_admitted(row, 1, allocation, s.codec_work()).map_err(publish_err)?;
    let window_start: Option<i64> = row.optional_integer(2).map_err(publish_err)?;
    let batch =
        TextValue::from_sql_admitted(row, 3, allocation, s.codec_work()).map_err(publish_err)?;
    check_batch(&batch)?;
    let generated = root(s, row.blob(4).map_err(publish_err)?)?;
    let wire = root(s, row.blob(5).map_err(publish_err)?)?;
    let first: i64 = row.integer(6).map_err(publish_err)?;
    check_generation(first, latest)?;
    exact(&generated, 6)?;
    if text(s, &generated, "kind")? != kind
        || window(s, &generated, "window_start")? != window_start
        || observation_id(s, &generated)? != id
        || observation_wire(s, &generated, &id, Some(&batch))? != wire
    {
        return Err(publish_err("观察索引与出生原件不符"));
    }
    Ok(Observation {
        id,
        kind,
        window_start,
        batch: Some(batch),
        generated,
        wire,
        first,
    })
}

fn check_generation(first: i64, latest: i64) -> Result<(), String> {
    if first < 1 || first > latest {
        Err(err("索引出生代号越界"))
    } else {
        Ok(())
    }
}

/// 历史 cut 的独立成员方程：Snapshot 全历史按库布局的公开主序（[`Layout::rank`]）；
/// Delta 仅本代首次、按插入序。load 按 rowid 返回，保留首次插入顺序。
#[cfg(test)]
pub(super) fn cut_members(
    index: &Index,
    layout: Layout,
    generation: i64,
    delta: bool,
) -> Result<[CodecVec<NodeDigest>; 3], String> {
    cut_members_metered(index, layout, generation, delta, None)
}
pub(super) fn cut_members_metered(
    index: &Index,
    layout: Layout,
    generation: i64,
    delta: bool,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<[CodecVec<NodeDigest>; 3], String> {
    cut_members_projected(index, layout, generation, delta, budget, None)
        .map(|v| v.0)
        .map_err(newchan_rust::s_content::ContextError::into_message)
}
pub(super) fn native_cut_members(
    index: &Index,
    s: &mut super::history::HashReader<'_>,
    layout: Layout,
    generation: i64,
    delta: bool,
    batch: &str,
) -> Result<([CodecVec<NodeDigest>; 3], sc::Built), PublishError> {
    if index.budget.is_some() || s.native_projection_capture().is_none() {
        return Err(publish_err("Native观察双投影资格不符"));
    }
    let (members, template) =
        cut_members_projected(index, layout, generation, delta, None, Some((s, batch)))?;
    Ok((
        members,
        template.ok_or_else(|| publish_err("缺Native观察模板"))?,
    ))
}
fn cut_members_projected(
    index: &Index,
    layout: Layout,
    generation: i64,
    delta: bool,
    budget: Option<&dyn super::audit_budget::Budget>,
    mut native: Option<(&mut super::history::HashReader<'_>, &str)>,
) -> Result<([CodecVec<NodeDigest>; 3], Option<sc::Built>), PublishError> {
    let admission = super::audit_store::NativeAdmission(
        native.as_ref().and_then(|(s, _)| s.native_work_scope()),
    );
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
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut w, 1).map_err(publish_err)?;
        }
        w.try_push(row).map_err(publish_err)?;
    }
    for row in index.relations.iter().filter(|row| member(row.first)) {
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut r, 1).map_err(publish_err)?;
        }
        r.try_push(row).map_err(publish_err)?;
    }
    for row in index.observations.iter().filter(|row| member(row.first)) {
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut o, 1).map_err(publish_err)?;
        }
        o.try_push(row).map_err(publish_err)?;
    }
    if !delta {
        stable_sort(
            &mut w,
            |a, b| witness_rank(layout, a).cmp(&witness_rank(layout, b)),
            owned,
            &admission,
        )?;
        stable_sort(
            &mut r,
            |a, b| relation_rank(layout, a).cmp(&relation_rank(layout, b)),
            owned,
            &admission,
        )?;
        stable_sort(
            &mut o,
            |a, b| obs_rank(layout, a).cmp(&obs_rank(layout, b)),
            owned,
            &admission,
        )?;
    }
    let mut witnesses = bj::new_work(owned);
    let mut relations = bj::new_work(owned);
    let mut observations = bj::new_work(owned);
    let mut template_observations = native.as_ref().map(|_| bj::new_work(None));
    for row in w {
        if let Some(b) = budget {
            b.record().map_err(publish_err)?;
        }
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut witnesses, 1)
                .map_err(publish_err)?;
        }
        witnesses
            .try_push(digest(&row.wire)?)
            .map_err(publish_err)?;
    }
    for row in r {
        if let Some(b) = budget {
            b.record().map_err(publish_err)?;
        }
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut relations, 1)
                .map_err(publish_err)?;
        }
        relations
            .try_push(digest(&row.generated)?)
            .map_err(publish_err)?;
    }
    for row in o {
        if let Some(b) = budget {
            b.record().map_err(publish_err)?;
        }
        if let Some((s, batch)) = native.as_mut() {
            let saved = row
                .batch
                .as_ref()
                .ok_or_else(|| publish_err("历史观察缺实际batch"))?;
            if (row.first == generation) != (saved.as_str() == *batch) {
                return Err(publish_err("观察出生/旧代batch不符"));
            }
            let capture = s
                .native_projection_capture()
                .ok_or_else(|| publish_err("观察capture丢失"))?;
            s.native_verify_original(capture, row.wire.view(), sc::Mode::Public)
                .map_err(publish_err)?;
            let template = if row.first == generation {
                const NAMES: &[&str] = &[
                    "observation_id",
                    "batch_id",
                    "kind",
                    "reason",
                    "detail",
                    "window_start",
                    "window_mid",
                    "window_end",
                ];
                let fields = s
                    .lookup_fields_ref(row.wire.view(), NAMES)
                    .map_err(publish_err)?;
                let mut values = bj::new_work(None);
                s.native_codec_room(&mut values, NAMES.len())
                    .map_err(publish_err)?;
                for name in NAMES {
                    let r = fields
                        .get_admitted(*s, name)
                        .map_err(publish_err)?
                        .ok_or_else(|| publish_err("观察wire缺字段"))?;
                    values
                        .try_push((
                            *name,
                            if *name == "batch_id" {
                                VV::BatchSelf
                            } else {
                                VV::Edge(r.digest().map_err(publish_err)?)
                            },
                        ))
                        .map_err(publish_err)?;
                }
                s.write_template_view(VV::Object(&values))
                    .map_err(publish_err)?
                    .digest
            } else {
                digest(&row.wire)?
            };
            s.native_codec_room(template_observations.as_mut().unwrap(), 1)
                .map_err(publish_err)?;
            template_observations
                .as_mut()
                .unwrap()
                .try_push(template)
                .map_err(publish_err)?;
        }
        if let Some((s, _)) = native.as_ref() {
            s.native_codec_room(&mut observations, 1)
                .map_err(publish_err)?;
        }
        observations
            .try_push(digest(&row.wire)?)
            .map_err(publish_err)?;
    }
    let template = if let Some(nodes) = template_observations {
        let mut values = bj::new_work(None);
        native
            .as_ref()
            .unwrap()
            .0
            .native_codec_room(&mut values, nodes.len())
            .map_err(publish_err)?;
        for node in &nodes {
            values.try_push(VV::Edge(*node)).map_err(publish_err)?;
        }
        Some(
            native
                .as_mut()
                .unwrap()
                .0
                .write_template_view(VV::Array(&values))
                .map_err(publish_err)?,
        )
    } else {
        None
    };
    Ok(([witnesses, relations, observations], template))
}
fn check_batch(batch: &str) -> Result<(), PublishError> {
    if batch.strip_prefix("batch3-").is_none_or(|h| {
        h.len() != 64
            || !h
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        Err(publish_err("非法batch3身份"))
    } else {
        Ok(())
    }
}

/// manifest 已得到真实身份之后调用；任何后续错误由调用方回滚整个事务。无 INSERT OR IGNORE。
#[cfg_attr(
    all(not(test), feature = "s_incremental"),
    allow(dead_code, reason = "#1488：8组合探针仅 native 非测试未调用")
)]
pub(super) fn stage(
    tx: &Transaction<'_>,
    s: &mut (impl CodecStore + ?Sized),
    p: &Plan,
    batch: &str,
) -> Result<(), String> {
    stage_in_material(
        tx,
        s,
        p,
        batch,
        #[cfg(feature = "s_incremental")]
        None,
    )
    .map_err(newchan_rust::s_content::ContextError::into_message)
}
pub(super) fn stage_in_material(
    tx: &Transaction<'_>,
    s: &mut (impl CodecStore + ?Sized),
    p: &Plan,
    batch: &str,
    #[cfg(feature = "s_incremental")] material: Option<
        &super::audit_owner::MaterialAccess<'_, '_, '_>,
    >,
) -> Result<(), PublishError> {
    check_batch(batch)?;
    for w in &p.births.witnesses {
        if w.first != p.generation {
            return Err(publish_err("见证出生代错绑"));
        }
        #[cfg(feature = "s_incremental")]
        let construction_stage = super::audit_owner::begin_construction_stage(
            tx,
            super::audit_owner::StageKind::Witness,
        )?;
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
        .map_err(publish_err)?;
        #[cfg(feature = "s_incremental")]
        construction_stage.finish(material)?;
    }
    for r in &p.births.relations {
        if r.first != p.generation {
            return Err(publish_err("关系出生代错绑"));
        }
        #[cfg(feature = "s_incremental")]
        let construction_stage = super::audit_owner::begin_construction_stage(
            tx,
            super::audit_owner::StageKind::Relation,
        )?;
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
        .map_err(publish_err)?;
        #[cfg(feature = "s_incremental")]
        construction_stage.finish(material)?;
    }
    for o in &p.births.observations {
        if o.first != p.generation || o.batch.is_some() {
            return Err(publish_err("观察出生代/模板错绑"));
        }
        #[cfg(feature = "s_incremental")]
        let construction_stage = super::audit_owner::begin_construction_stage(
            tx,
            super::audit_owner::StageKind::Observation,
        )?;
        let wire = s
            .replace_ref(o.wire.view(), &[f("batch_id")], batch)
            .map_err(publish_err)?;
        s.verify_many_ref_fresh([wire.view()], sc::Mode::Public)
            .map_err(publish_err)?;
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
        .map_err(publish_err)?;
        #[cfg(feature = "s_incremental")]
        construction_stage.finish(material)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_aux_tables_retain_owners_across_scope_and_release_on_drop() {
        let rows = [("alpha", 3), ("beta", 1), ("alpha", 2)];
        let budget = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let (ids, born, duplicate) = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let store = super::super::audit_store::NativeAdmission(Some(scope));
                    native_tables(&rows, |r| r.0, |r| r.1, &store)
                })
                .unwrap();
            assert_eq!(ids.get("alpha"), Some(&2));
            assert_eq!(ids.get("beta"), Some(&1));
            assert_eq!(duplicate, Some(3));
            assert_eq!(&born.get(&2).unwrap()[..], &[2]);
            assert!(budget.live() > baseline);
            drop((ids, born));
            assert_eq!(budget.live(), baseline);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_aux_failed_table_construction_releases_partial_owners() {
        let rows = [("alpha", 1), ("beta", 2)];
        let limit = 1 << 20;
        let budget = super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
        let baseline = budget.live();
        // 只给工作域元数据留位：开域本身须计父账，父满时连 codec_work 都开不成，到不了被测表构造。
        let work_metadata =
            newchan_rust::s_content::capture_allocation::scratch::CodecWork::metadata_bytes();
        let occupied = budget.reserve(limit - baseline - work_metadata).unwrap();
        let before = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let store = super::super::audit_store::NativeAdmission(Some(scope));
            native_tables(&rows, |r| r.0, |r| r.1, &store)
        });
        assert!(result.is_err());
        assert_eq!(budget.live(), before);
        drop(occupied);
        assert_eq!(budget.live(), baseline);
    }

    fn clone_ref(value: &RefValue) -> RefValue {
        super::super::audit_store::NativeAdmission(None)
            .native_ref_clone(value)
            .unwrap()
    }
    #[cfg_attr(
        all(test, not(feature = "s_incremental")),
        allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
    )]
    fn clone_refs(values: &[RefValue]) -> Vec<RefValue> {
        values.iter().map(clone_ref).collect()
    }

    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_set, open_db, read_effective_events, read_raw_events, tb02a,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use rusqlite::Connection;

    #[cfg(feature = "s_incremental")]
    #[test]
    fn r12_cold_object_aux_shared_scope_matches_complete_plain_outputs() {
        use super::super::{audit_owner::native::Budget, history::HashReader, object_index};
        let mut fixture = super::super::prepared::tests::Fixture::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (event, token) = fixture.accept_begin(n, row, revision);
            fixture.publish(&event, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let raw = read_raw_events(&tx).unwrap();
        let raw = RawEventsView::plain(&raw);
        let batch: String = tx
            .query_row(
                "SELECT batch_id FROM s_content_publications WHERE generation=4",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        let mut plain = HashReader::captured_for_audit(&capture, None).unwrap();
        plain.enable_full_arrays_for_test();
        let mut expected_objects = object_index::load_for_history(&tx, &mut plain, 4).unwrap();
        let expected_entries = format!("{:?}", expected_objects.entries());
        let expected_dual = expected_objects
            .native_dual_at_cut(&mut plain, 4, &batch)
            .unwrap();
        let expected_aux = load(&tx, &mut plain, 4).unwrap();
        let expected_rows = format!(
            "{:?}",
            (
                &expected_aux.witnesses,
                &expected_aux.relations,
                &expected_aux.observations
            )
        );
        let layout = expected_objects.layout();
        let expected_sets = [false, true].map(|delta| {
            native_cut_members(&expected_aux, &mut plain, layout, 4, delta, &batch).unwrap()
        });
        let expected_facts = NativeAuxAudit::from_loaded(&plain, &expected_aux, raw)
            .unwrap()
            .into_facts();
        let budget = Budget::new(64 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let before = budget.live();
                // 与 state 冷入口一致：在当前父 scope 内独立捕获并启用 admitted 帧复用。
                // 上面的普通 oracle 仍保留自己的无复用 capture。
                let (scoped_capture, node_count) =
                    content_store::capture_for_request_prototype_admitted(&tx, Some(scope))
                        .map_err(err)?;
                let scoped_capture = scoped_capture
                    .enable_native_frame_reuse_admitted(Some(scope))
                    .map_err(err)?;
                let mut reader = HashReader::captured_for_audit(&scoped_capture, None)
                    .unwrap()
                    .with_codec_work(scope, &budget);
                reader.enable_full_arrays_for_test();
                let mut objects = object_index::load_for_history(&tx, &mut reader, 4)?;
                assert_eq!(format!("{:?}", objects.entries()), expected_entries);
                let sweep = objects
                    .native_sweep_admitted(&reader)
                    .map_err(newchan_rust::s_content::ContextError::into_message)?
                    .unwrap();
                assert_eq!(
                    objects
                        .native_dual_at_cut(&mut reader, 4, &batch)
                        .map_err(newchan_rust::s_content::ContextError::into_message)?,
                    expected_dual
                );
                drop(sweep);
                let index = load(&tx, &mut reader, 4)?;
                assert_eq!(
                    format!(
                        "{:?}",
                        (&index.witnesses, &index.relations, &index.observations)
                    ),
                    expected_rows
                );
                let aux = NativeAuxAudit::from_loaded_admitted(&reader, &index, raw)?.unwrap();
                for (actual, expected) in [
                    (&aux.witnesses, &expected_facts.witnesses),
                    (&aux.relations, &expected_facts.relations),
                    (&aux.observations, &expected_facts.observations),
                ] {
                    fn entries(
                        map: &bj::BusinessMap<NativeValue<String>, usize>,
                    ) -> std::collections::BTreeMap<&str, usize> {
                        map.iter()
                            .map(|(key, value)| (key.as_str(), *value))
                            .collect()
                    }
                    assert_eq!(entries(actual), entries(expected));
                }
                for (actual, expected) in aux.born.iter().zip(&expected_facts.born) {
                    assert_eq!(actual.len(), expected.len());
                    assert_eq!(
                        actual
                            .iter()
                            .map(|(generation, rows)| (*generation, &rows[..]))
                            .collect::<Vec<_>>(),
                        expected
                            .iter()
                            .map(|(generation, rows)| (*generation, &rows[..]))
                            .collect::<Vec<_>>()
                    );
                }
                assert_eq!(aux.duplicates, expected_facts.duplicates);
                assert_eq!(aux.raw_scanned, expected_facts.raw_scanned);
                assert_eq!(aux.raw_revisions, expected_facts.raw_revisions);
                for (delta, expected) in [false, true].into_iter().zip(&expected_sets) {
                    let (members, template) =
                        native_cut_members(&index, &mut reader, Layout::Sorted, 4, delta, &batch)
                            .map_err(newchan_rust::s_content::ContextError::into_message)?;
                    for (actual, expected) in members.iter().zip(&expected.0) {
                        assert_eq!(&actual[..], &expected[..]);
                    }
                    assert_eq!(template, expected.1);
                }
                assert!(budget.live() > before);
                assert!(scope.snapshot().attempts > 0);
                eprintln!(
                    "R12_INIT complete nodes={} live={} peak={} scope={:?}",
                    node_count,
                    budget.live(),
                    budget.peak(),
                    scope.snapshot()
                );
                Ok::<_, String>(())
            })
            .unwrap();
    }

    /// #1488 0077 P50（mut-0057 M2a 盲区）：Legacy→Native 转换期的索引许可须按「槽数×(size_of+1)」
    /// 全额计入本请求 scope（转换期峰值越过转换前 live + 许可）；漏计控制字节（M2a）或不经 scope
    /// 预留都变红。槽数按转换前 Legacy 节点数
    /// 独立推得（next_pow2(n)×4+16），不读被测实现的中间值。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p50_legacy_to_native_index_permit_counts_control_byte_per_slot() {
        use super::super::audit_owner::native::Budget;
        let mut fixture = super::super::prepared::tests::Fixture::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
        ] {
            let (event, token) = fixture.accept_begin(n, row, revision);
            fixture.publish(&event, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let stored: i64 = tx
            .query_row("SELECT COUNT(*) FROM s_content_nodes", [], |row| row.get(0))
            .unwrap();
        let stored = usize::try_from(stored).unwrap();
        let budget = Budget::new(64 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let (legacy, node_count) =
                    content_store::capture_for_request_prototype_admitted(&tx, Some(scope))
                        .map_err(err)?;
                assert!(node_count > 0 && node_count <= stored, "{node_count}/{stored}");
                assert!(legacy.native_index_layout_for_testonly().is_none(), "转换前须为 Legacy");
                let (before, peak_before) = (budget.live(), budget.peak());
                let native = legacy
                    .enable_native_frame_reuse_admitted(Some(scope))
                    .map_err(err)?;
                let (index_bytes, slot_now, _) = native
                    .native_index_layout_for_testonly()
                    .expect("转换后须为 Native");
                let slots = node_count.next_power_of_two() * 4 + 16;
                assert_eq!(
                    index_bytes,
                    slots * slot_now,
                    "索引许可须为槽数×(size_of+1)：slots={slots} slot={slot_now}"
                );
                // 转换期瞬时共存：索引许可先于 Legacy 原件释放计入，峰值须越过「转换前 live + 许可」。
                // （净 live 增量因 Legacy 释放小于许可，不作判据。）
                assert!(
                    peak_before < before + index_bytes && budget.peak() >= before + index_bytes,
                    "索引许可须在转换期计入本 scope：live {before}，峰值 {peak_before} → {}，许可 {index_bytes}",
                    budget.peak()
                );
                eprintln!(
                    "P50_INDEX nodes={node_count} stored={stored} slots={slots} slot={slot_now} index={index_bytes} live={before}->{} peak={peak_before}->{}",
                    budget.live(),
                    budget.peak()
                );
                Ok::<_, String>(())
            })
            .unwrap();
    }

    // 此夹具只验证真实生成器→辅助索引→SQLite，不声称完整 S3 控制/对象发布已接线。
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_aux_owner_prefix_duplicates_reference_and_overlay_matrix() {
        fn state(s: &mut impl CodecStore, relations: &[RefValue]) -> RefValue {
            let values = relations
                .iter()
                .map(|r| VV::Edge(r.digest().unwrap()))
                .collect::<Vec<_>>();
            let rel = s.write_value_view(VV::Array(&values)).unwrap();
            let empty = s.write_value_view(VV::Array(&[])).unwrap();
            let fields = [
                ("witnesses", VV::Edge(empty.digest)),
                ("relations", VV::Edge(rel.digest)),
                ("observations", VV::Edge(empty.digest)),
            ];
            RefValue::from(
                s.write_value_view(VV::Object(&fields))
                    .unwrap()
                    .content_ref(),
            )
        }
        let raw = vec![
            json!({"supersedes_revision":null}),
            json!({"identity_key":"raw","revision":2,"supersedes_revision":1}),
            json!({"identity_key":"raw","revision":3,"supersedes_revision":2}),
            json!({"revision":4,"supersedes_revision":3}),
        ];
        let mut nodes = sc::MemoryNodes::new();
        let r1 = supersedes_relation(&mut nodes, (&raw[1]).into())
            .unwrap()
            .unwrap();
        let r2 = supersedes_relation(&mut nodes, (&raw[2]).into())
            .unwrap()
            .unwrap();
        let empty = state(&mut nodes, &[]);
        let repeated = state(&mut nodes, &[clone_ref(&r1), clone_ref(&r1)]);
        let reordered = state(&mut nodes, &[clone_ref(&r2), clone_ref(&r1)]);
        let mut index = Index::default();
        index
            .relations
            .try_push(relation(&mut nodes, clone_ref(&r1), 2).unwrap())
            .unwrap();
        index
            .relations
            .try_push(relation(&mut nodes, clone_ref(&r2), 2).unwrap())
            .unwrap();
        let capture = sc::AuditCapture::new(nodes);
        let mut reader =
            super::super::history::HashReader::captured_for_audit(&capture, None).unwrap();
        reader.enable_full_arrays_for_test();
        let mut owner = NativeAuxAudit::from_loaded(&reader, &index, (&raw).into()).unwrap();
        owner
            .audit(
                &mut reader,
                &empty,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                1,
                1,
            )
            .unwrap();
        assert!(
            owner
                .audit(
                    &mut reader,
                    &repeated,
                    JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                    0,
                    1
                )
                .is_err(),
            "future births must not enter lookup"
        );
        // Both rows born at g2: raw prefix ordinal gives r1,r2 in the real insertion order.
        owner
            .audit(
                &mut reader,
                &empty,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                3,
                2,
            )
            .unwrap();
        reader.clear_overlay();
        owner
            .audit(
                &mut reader,
                &empty,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                1,
                1,
            )
            .unwrap();
        assert!(owner
            .audit(
                &mut reader,
                &reordered,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                3,
                2
            )
            .unwrap_err()
            .to_string()
            .contains("出生集合或顺序"));
        assert!(owner
            .audit(
                &mut reader,
                &empty,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                4,
                2
            )
            .unwrap_err()
            .to_string()
            .contains("源身份缺失"));
        assert!(owner.raw_memo.is_empty() && owner.relation_memo.is_empty());
        let other = sc::AuditCapture::new(sc::MemoryNodes::new());
        let mut foreign =
            super::super::history::HashReader::captured_for_audit(&other, None).unwrap();
        foreign.enable_full_arrays_for_test();
        assert!(owner
            .audit(
                &mut foreign,
                &empty,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                1,
                1
            )
            .is_err());
        // Ref declarations are not encoded by Edge; pass the malformed STATE declaration
        // directly into the actual owner entry instead of panicking in the builder.
        let mut malformed_state = clone_ref(&empty);
        if let RefValue::Plain(r) = &malformed_state {
            assert!(!r.is_native_admitted());
            let mut value = r.as_ref().clone();
            value.format = "bad-format".into();
            malformed_state = value.into();
        }
        assert!(owner
            .audit(
                &mut reader,
                &malformed_state,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                1,
                1
            )
            .is_err());
        let mut malformed_relation = clone_ref(&r1);
        if let RefValue::Plain(r) = &malformed_relation {
            assert!(!r.is_native_admitted());
            let mut value = r.as_ref().clone();
            value.extent = Some(4);
            malformed_relation = value.into();
        }
        assert!(owner.relation(&mut reader, malformed_relation, 2).is_err());
        drop(owner);
        let mut changed = index.try_clone().unwrap();
        changed.relations[0].first = 1;
        changed.relations[0].generated = clone_ref(&r2);
        let mut owner =
            NativeAuxAudit::from_loaded(&reader, &changed, RawEventsView::plain(&[])).unwrap();
        assert!(owner
            .audit(
                &mut reader,
                &repeated,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                0,
                1
            )
            .unwrap_err()
            .to_string()
            .contains("原件与独立索引不一致"));
        drop(owner);
        let missing = Index::default();
        let mut owner =
            NativeAuxAudit::from_loaded(&reader, &missing, RawEventsView::plain(&[])).unwrap();
        assert!(owner
            .audit(
                &mut reader,
                &repeated,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                0,
                1
            )
            .is_err());
        drop(owner);
        let mut duplicated = index.try_clone().unwrap();
        duplicated.relations[0].first = 1;
        let mut future = duplicated.relations[0].try_clone().unwrap();
        future.first = 3;
        let rows = std::mem::take(&mut duplicated.relations);
        duplicated.relations.try_push(future).unwrap();
        for row in rows {
            duplicated.relations.try_push(row).unwrap();
        }

        let mut owner =
            NativeAuxAudit::from_loaded(&reader, &duplicated, RawEventsView::plain(&[])).unwrap();
        owner
            .audit(
                &mut reader,
                &repeated,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                0,
                1,
            )
            .unwrap();
        assert!(owner
            .audit(
                &mut reader,
                &repeated,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                0,
                3
            )
            .unwrap_err()
            .to_string()
            .contains("独立索引重复"));
        drop(owner);
        let mut owner =
            NativeAuxAudit::from_loaded(&reader, &index, RawEventsView::plain(&[])).unwrap();
        let overlay = supersedes_relation(
            &mut reader,
            (&json!({"identity_key":"overlay","revision":9,"supersedes_revision":8})).into(),
        )
        .unwrap()
        .unwrap();
        let overlay_state = state(&mut reader, &[overlay]);
        assert!(owner
            .audit(
                &mut reader,
                &overlay_state,
                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                0,
                2
            )
            .is_err());
        assert!(owner.raw_memo.is_empty() && owner.relation_memo.is_empty());
        reader.clear_overlay();
        drop(owner);
        let mut overlay_index = Index::default();
        let original_overlay = supersedes_relation(
            &mut reader,
            (&json!({"identity_key":"overlay","revision":9,"supersedes_revision":8})).into(),
        )
        .unwrap()
        .unwrap();
        overlay_index
            .relations
            .try_push(relation(&mut reader, clone_ref(&original_overlay), 1).unwrap())
            .unwrap();
        let mut overlay_owner =
            NativeAuxAudit::from_loaded(&reader, &overlay_index, RawEventsView::plain(&[]))
                .unwrap();
        for _ in 0..2 {
            let current = supersedes_relation(
                &mut reader,
                (&json!({"identity_key":"overlay","revision":9,"supersedes_revision":8})).into(),
            )
            .unwrap()
            .unwrap();
            let current_state = state(&mut reader, &[current]);
            overlay_owner
                .audit(
                    &mut reader,
                    &current_state,
                    JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                    0,
                    1,
                )
                .unwrap();
            assert!(overlay_owner.relation_memo.is_empty());
            reader.clear_overlay();
        }
    }

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
                            witness_id: sc::capture_allocation::native_owned::NativeValue::<String>::from_unscoped(id),
                            object_id: sc::capture_allocation::native_owned::NativeValue::<String>::from_unscoped(object.to_owned()),
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
            let r = built_ref(&store, &b).unwrap();
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
        let expected_members = cut_members(&expected, Layout::Sorted, 1, false).unwrap();
        let mut normal_runtime = Runtime::default();
        let b = |r: &RefValue| Built {
            digest: r.digest().unwrap(),
            kind: r.kind(),
            extent: r.extent(),
        };
        let w = &expected.witnesses[0];
        let metadata = WitnessRecord {
            witness_id: sc::capture_allocation::native_owned::NativeValue::<String>::from_unscoped(
                w.id.as_str().to_owned(),
            ),
            object_id: sc::capture_allocation::native_owned::NativeValue::<String>::from_unscoped(
                w.object_id.as_str().to_owned(),
            ),
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
                .try_push(relation(&mut normal, clone_ref(&r.generated), 1).unwrap())
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
                        .try_push(relation(store, clone_ref(&r.generated), 1).unwrap())
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
                let delta_sets =
                    public_sets(store, &constructed, &[], &mut runtime.sequences[4..]).unwrap();
                assert_eq!(delta_sets.witnesses, normal_sets.witnesses);
                assert_eq!(delta_sets.relations, normal_sets.relations);
                assert_eq!(delta_sets.observations, normal_sets.observations);
                assert!(
                    runtime.sequences.iter().all(|memo| memo
                        .allocation_budget()
                        .is_some_and(|b| b.same_ledger(&budget))),
                    "真实snapshot/delta八份持久keys均须留在原账，包括空raw"
                );
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
                let members = cut_members(&actual, Layout::Sorted, 1, false).unwrap();
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
    fn owned_aux_public_sets_sequence_keys_growth_one_byte_short_preserves_roots_and_discard_releases(
    ) {
        let budget = OwnedBudget::new(1_000_000);
        let mut nodes = sc::MemoryNodes::new();
        let first = sc::write_value(&mut nodes, &sc::Value::Bool(true))
            .unwrap()
            .digest;
        let second = sc::write_value(&mut nodes, &sc::Value::Bool(false))
            .unwrap()
            .digest;
        let mut slab = sc::capture_allocation::CaptureSlab::new(2, 0, &budget).unwrap();
        for (digest, tag, body) in nodes.iter() {
            slab.push(*digest, tag, body).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        super::super::audit_store::with_owned_store_probe(&capture, &budget, 64, 4096, |store| {
            let index = Index::default();
            let mut runtime = Runtime::default();
            let before = public_sets(store, &index, &[first], &mut runtime.sequences[..4]).unwrap();
            assert!(runtime.sequences[..4].iter().all(|memo| memo
                .allocation_budget()
                .is_some_and(|b| b.same_ledger(&budget))));
            let live = budget.snapshot().live_requested_layout_bytes;
            let fill = sc::capture_allocation::CaptureSlab::new(
                0,
                budget.snapshot().limit_bytes - live - 64 + 1,
                &budget,
            )
            .unwrap();
            let error = public_sets(store, &index, &[first, second], &mut runtime.sequences[..4])
                .err()
                .unwrap();
            assert!(
                error.to_string().contains(sc::capture_allocation::LIMIT),
                "{error}"
            );
            let probe = budget.allocation_request_probe();
            assert_eq!(probe.refused_bytes, 64);
            assert_eq!(
                probe.live_before_refusal + 64,
                budget.snapshot().limit_bytes + 1
            );
            drop(fill);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, live);
            let after = public_sets(store, &index, &[first], &mut runtime.sequences[..4]).unwrap();
            assert_eq!(after.witnesses, before.witnesses);
            assert_eq!(after.relations, before.relations);
            assert_eq!(after.observations, before.observations);
            assert_eq!(after.raw_history, before.raw_history);
            let before_discard = budget.snapshot().live_requested_layout_bytes;
            runtime.discard();
            assert_eq!(
                before_discard - budget.snapshot().live_requested_layout_bytes,
                32
            );
            println!("real Owned public_sets keys request=64 refused one-byte-short; old roots preserved; discard releases32");
        });
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
                .plan(
                    &mut s,
                    Layout::Sorted,
                    &old,
                    &generated,
                    &[],
                    &raw,
                    previous,
                    frontier,
                    g,
                )
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
                    .find(|r| r.id.as_str() == w.witness_id.as_str())
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
                &*generated.generated_state,
                &[],
                &raw[..=frontier as usize],
                g,
            )
            .unwrap();
            #[cfg(feature = "s_incremental")]
            {
                let capture = content_store::capture_for_prototype(&tx).unwrap();
                let mut reader =
                    super::super::history::HashReader::captured_for_audit(&capture, None).unwrap();
                reader.enable_full_arrays_for_test();
                reset_aux_audit_stats();
                let mut owner =
                    NativeAuxAudit::from_loaded(&reader, &stored, (&raw).into()).unwrap();
                owner
                    .audit(
                        &mut reader,
                        &*generated.generated_state,
                        JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                        frontier as usize + 1,
                        g,
                    )
                    .unwrap();
                let first = aux_audit_stats();
                let n = (stored.witnesses.len()
                    + stored.relations.len()
                    + stored.observations.len()) as u64;
                assert_eq!(first[0], n);
                assert_eq!(first[1], n);
                reader.clear_overlay();
                owner
                    .audit(
                        &mut reader,
                        &*generated.generated_state,
                        JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                        frontier as usize + 1,
                        g,
                    )
                    .unwrap();
                let repeated = aux_audit_stats();
                assert_eq!(repeated[0], n);
                assert_eq!(repeated[1], n);
                assert_eq!(
                    repeated[2], first[2],
                    "actual eligible relation decode must not repeat"
                );
                assert_eq!(
                    repeated[4], 6,
                    "all three birth comparisons execute each cut"
                );
                assert!(repeated[3] > first[3]);
                drop(owner);
                if g == 1 {
                    fn replacement_state(
                        s: &mut super::super::history::HashReader<'_>,
                        original: &ContentRef,
                        name: &str,
                        items: &[RefValue],
                    ) -> RefValue {
                        let mut roots = Vec::new();
                        for field_name in ["witnesses", "relations", "observations"] {
                            let r = if field_name == name {
                                let v = items
                                    .iter()
                                    .map(|r| VV::Edge(r.digest().unwrap()))
                                    .collect::<Vec<_>>();
                                RefValue::from(
                                    s.write_value_view(VV::Array(&v)).unwrap().content_ref(),
                                )
                            } else {
                                field(s, original, field_name).unwrap()
                            };
                            roots.push((field_name, VV::Edge(r.digest().unwrap())));
                        }
                        RefValue::from(
                            s.write_value_view(VV::Object(&roots))
                                .unwrap()
                                .content_ref(),
                        )
                    }
                    for name in ["witnesses", "observations"] {
                        let original =
                            state_members(&mut reader, &*generated.generated_state, name)
                                .unwrap()
                                .into_iter()
                                .collect::<Vec<_>>();
                        assert!(!original.is_empty(), "real fixture {name}");
                        for (label, items) in [
                            ("duplicate", {
                                let mut v = clone_refs(&original);
                                v.push(clone_ref(&v[0]));
                                v
                            }),
                            ("missing", clone_refs(&original[1..])),
                        ] {
                            let badstate = replacement_state(
                                &mut reader,
                                &generated.generated_state,
                                name,
                                &items,
                            );
                            let old = verify_generated_members_with_changes(
                                &mut reader,
                                &stored,
                                &badstate,
                                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                                &raw,
                                g,
                            )
                            .is_err();
                            let mut owner =
                                NativeAuxAudit::from_loaded(&reader, &stored, (&raw).into())
                                    .unwrap();
                            let new = owner
                                .audit(
                                    &mut reader,
                                    &badstate,
                                    JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                                    raw.len(),
                                    g,
                                )
                                .is_err();
                            assert!(
                                old && new,
                                "{name} {label} must reject in actual new owner entry"
                            );
                        }
                    }
                    if stored.witnesses.len() > 1 {
                        let items =
                            state_members(&mut reader, &*generated.generated_state, "witnesses")
                                .unwrap()
                                .into_iter()
                                .collect::<Vec<_>>();
                        let mut reverse = clone_refs(&items);
                        reverse.swap(0, 1);
                        let badstate = replacement_state(
                            &mut reader,
                            &generated.generated_state,
                            "witnesses",
                            &reverse,
                        );
                        let mut owner =
                            NativeAuxAudit::from_loaded(&reader, &stored, (&raw).into()).unwrap();
                        assert!(owner
                            .audit(
                                &mut reader,
                                &badstate,
                                JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                                raw.len(),
                                g
                            )
                            .unwrap_err()
                            .to_string()
                            .contains("出生集合或顺序"));
                    }
                    let bad_replaces = json!([{}]);
                    let mut owner =
                        NativeAuxAudit::from_loaded(&reader, &stored, (&raw).into()).unwrap();
                    assert!(owner
                        .audit(
                            &mut reader,
                            &*generated.generated_state,
                            (&bad_replaces).into(),
                            raw.len(),
                            g
                        )
                        .is_err());
                }
                if g == 1 {
                    let mut bad = stored.try_clone().unwrap();
                    let removed = bad.witnesses.pop().unwrap();
                    let mut owner =
                        NativeAuxAudit::from_loaded(&reader, &bad, (&raw).into()).unwrap();
                    assert!(owner
                        .audit(
                            &mut reader,
                            &*generated.generated_state,
                            JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                            raw.len(),
                            g
                        )
                        .is_err());
                    drop(owner);
                    bad.witnesses.try_push(removed).unwrap();
                    bad.witnesses
                        .try_push(bad.witnesses[0].try_clone().unwrap())
                        .unwrap();
                    let mut owner =
                        NativeAuxAudit::from_loaded(&reader, &bad, (&raw).into()).unwrap();
                    assert!(owner
                        .audit(
                            &mut reader,
                            &*generated.generated_state,
                            JsonView::BorrowedArray(&RawEventsView::plain(&[])),
                            raw.len(),
                            g
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("独立索引重复"));
                }
            }
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
                    &*generated.generated_state,
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
                    &*generated.generated_state,
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
                    .map(|(_, row)| clone_ref(&row.generated))
                    .unwrap();
                let mut wrong = stored.try_clone().unwrap();
                wrong.witnesses[first].generated = other;
                assert!(verify_generated_members(
                    &mut s,
                    &wrong,
                    &*generated.generated_state,
                    &[],
                    &raw[..=frontier as usize],
                    g,
                )
                .is_err());
            }
            for (is_delta, expected) in [(false, &p.snapshot), (true, &p.births)] {
                let members = cut_members(&stored, Layout::Sorted, g, is_delta).unwrap();
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
            // #1491 layout 锁：Delta 两布局同为插入序；bounded Snapshot ＝ layout/1 公开序按出生代
            // 稳定重排（代际单调不减，同代内沿原键）。
            let sorted_delta = cut_members(&stored, Layout::Sorted, g, true).unwrap();
            let bounded_delta = cut_members(&stored, Layout::Bounded1, g, true).unwrap();
            for (sorted, bounded) in sorted_delta.iter().zip(&bounded_delta) {
                assert_eq!(&sorted[..], &bounded[..]);
            }
            let bounded = cut_members(&stored, Layout::Bounded1, g, false).unwrap();
            let mut witnesses: Vec<_> = p.snapshot.witnesses.iter().collect();
            witnesses.sort_by_key(|r| r.first);
            let witnesses: Vec<_> = witnesses.iter().map(|r| digest(&r.wire).unwrap()).collect();
            assert_eq!(&bounded[0][..], &witnesses[..]);
            let mut relations: Vec<_> = p.snapshot.relations.iter().collect();
            relations.sort_by_key(|r| r.first);
            let relations: Vec<_> = relations
                .iter()
                .map(|r| digest(&r.generated).unwrap())
                .collect();
            assert_eq!(&bounded[1][..], &relations[..]);
            let mut observations: Vec<_> = p.snapshot.observations.iter().collect();
            observations.sort_by_key(|r| r.first);
            let observations: Vec<_> = observations
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
                .collect();
            assert_eq!(&bounded[2][..], &observations[..]);
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

    /// #1488 S3：plan 的暖口（跨代 Runtime 记忆）与冷口（每代新 Runtime）逐位相同，
    /// 并打出每代暖口 SQL 读量，供 D6「读量 ∝ 本代重建后缀与 Δ_R」核对。
    #[test]
    fn aux_plan_warm_matches_cold_and_reads_track_delta() {
        fn rows_of(index: &Index) -> Value {
            json!({
                "witnesses": index.witnesses.iter().map(|r| json!([r.id.as_str(), r.object_id.as_str(), r.slot, r.merged_index, r.generated.digest().unwrap().to_vec(), r.wire.digest().unwrap().to_vec(), r.first])).collect::<Vec<_>>(),
                "relations": index.relations.iter().map(|r| json!([r.id.as_str(), r.subject.as_str(), r.relation_type.as_str(), r.object.as_str(), r.generated.digest().unwrap().to_vec(), r.first])).collect::<Vec<_>>(),
                "observations": index.observations.iter().map(|r| json!([r.id.as_str(), r.kind.as_str(), r.window_start, r.generated.digest().unwrap().to_vec(), r.wire.digest().unwrap().to_vec(), r.batch.as_ref().map(|b| b.as_str().to_owned()), r.first])).collect::<Vec<_>>(),
            })
        }
        fn sets_of(p: &PublicSets) -> [NodeDigest; 4] {
            [
                p.witnesses.digest,
                p.relations.digest,
                p.observations.digest,
                p.raw_history.digest,
            ]
        }
        fn bar(i: usize) -> (String, i64, i64, i64, i64, i64) {
            let (high, low) = if i % 2 == 0 {
                (20 + (i % 7) as i64, 12 + (i % 5) as i64)
            } else {
                (14 + (i % 3) as i64, 5 + (i % 4) as i64)
            };
            (
                format!("x{i}"),
                (i * 10) as i64,
                low + 1,
                high,
                low,
                low + 1,
            )
        }
        let files = TestFiles::new();
        init_test_db(&files);
        let mut conn = database();
        let mut runtime = Runtime::default();
        let mut previous = -1;
        let mut next = 0usize;
        for g in 1..=10i64 {
            let grow = if g == 1 { 8 } else { 3 };
            let mut rows = (next..next + grow).map(bar).collect::<Vec<_>>();
            next += grow;
            let mut input = {
                let borrowed = rows
                    .iter()
                    .map(|(id, c, o, h, l, cl)| (id.as_str(), *c, *o, *h, *l, *cl))
                    .collect::<Vec<_>>();
                located_ohlc_input(&borrowed)
            };
            if g == 5 || g == 8 {
                let at = if g == 5 { 3 } else { 10 };
                let (id, c, o, h, l, cl) = bar(at);
                rows = vec![(id, c, o, h + 2, l, cl)];
                let borrowed = rows
                    .iter()
                    .map(|(id, c, o, h, l, cl)| (id.as_str(), *c, *o, *h, *l, *cl))
                    .collect::<Vec<_>>();
                let mut revision = located_ohlc_input(&borrowed);
                revision["events"][0]["revision"] = json!("2");
                accept_ohlc(&files, &revision);
            }
            accept_ohlc(&files, &input.take());
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
                format!("aux-s3/{}/{g}", files.db().display()),
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
            let sql = content_store::sql_reads_on_thread();
            let frames = sc::frame_calls_on_thread();
            let warm = runtime
                .plan(
                    &mut s,
                    Layout::Sorted,
                    &old,
                    &generated,
                    &[],
                    &raw,
                    previous,
                    frontier,
                    g,
                )
                .unwrap();
            let warm_sql = content_store::sql_reads_on_thread() - sql;
            let warm_frames = sc::frame_calls_on_thread() - frames;
            let sql = content_store::sql_reads_on_thread();
            set_full_plan_for_test(true);
            let cold = Runtime::default().plan(
                &mut s,
                Layout::Sorted,
                &old,
                &generated,
                &[],
                &raw,
                previous,
                frontier,
                g,
            );
            set_full_plan_for_test(false);
            let cold = cold.unwrap();
            let cold_sql = content_store::sql_reads_on_thread() - sql;
            assert_eq!(
                rows_of(&warm.snapshot),
                rows_of(&cold.snapshot),
                "g={g} snapshot"
            );
            assert_eq!(rows_of(&warm.births), rows_of(&cold.births), "g={g} births");
            assert_eq!(
                sets_of(&warm.snapshot_values),
                sets_of(&cold.snapshot_values),
                "g={g}"
            );
            assert_eq!(
                sets_of(&warm.delta_values),
                sets_of(&cold.delta_values),
                "g={g}"
            );
            let state = warm.snapshot.witnesses.len()
                + warm.snapshot.relations.len()
                + warm.snapshot.observations.len();
            let births = warm.births.witnesses.len()
                + warm.births.relations.len()
                + warm.births.observations.len();
            eprintln!(
                "S3_AUX_PLAN g={g} raw={} delta_raw={} state_rows={state} birth_rows={births} generated_witnesses={} warm_sql={warm_sql} warm_frames={warm_frames} cold_sql={cold_sql}",
                raw.len(),
                frontier - previous,
                generated.witnesses.len(),
            );
            // 读量锁（D6）：暖口 SQL 读 ≤ 常数 ×（本代出生行 + Δ_R），常数与 g 无关。
            // 实测每行约 1.0–1.3k（g=1..10）；旧完整重算到 g=4 即超 2k/行并随状态线性增长。
            let delta = births as u64 + (frontier - previous) as u64;
            assert!(
                warm_sql <= 1600 * delta,
                "g={g} 暖口读量 {warm_sql} 超出 1600×(出生行{births}+Δ_R{})",
                frontier - previous
            );
            let batch = format!("batch3-{}", (g % 10).to_string().repeat(64));
            stage(&tx, &mut s, &warm, &batch).unwrap();
            drop(s);
            tx.commit().unwrap();
            previous = frontier;
        }
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

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, Retained};

    retained_struct!(Witness {
        id,
        object_id,
        slot,
        merged_index,
        generated,
        wire,
        first
    });
    retained_struct!(Relation {
        id,
        subject,
        relation_type,
        object,
        generated,
        first
    });
    retained_struct!(Observation {
        id,
        kind,
        window_start,
        generated,
        wire,
        batch,
        first
    });
    retained_struct!(NativeAuxFacts {
        witnesses,
        relations,
        observations,
        born,
        duplicates,
        raw_scanned,
        raw_revisions
    });
    impl Retained for Index {
        fn heap_bytes(&self) -> usize {
            let Self {
                witnesses,
                relations,
                observations,
                budget,
            } = self;
            if budget.is_some() {
                return usize::MAX;
            }
            crate::v3::audit_retention::sum([
                witnesses.heap_bytes(),
                relations.heap_bytes(),
                observations.heap_bytes(),
            ])
        }
    }
}

#[cfg(test)]
impl Index {
    pub(super) fn overlay_ab_budget_absent(&self) -> bool {
        self.budget.is_none()
    }
}

fn publish_err(error: impl Into<PublishError>) -> PublishError {
    error.into().context("AuxIndexInvalid：")
}
