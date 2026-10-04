//! #1394 C：对象独立出生/撤回索引与历史投影。变化判据复用 object_diff::plan。
//! 保存内部记录、生成态、不可变内容和出生 wire 四根；所有来源/载荷保持内容边。
//! 本层证明索引与这些原件互相绑定；原件是否由真实源按正确业务规则产生仍归来源审计。
use super::audit_store::RefValue;
// Re-export only the opaque type; its fields and constructor stay history-private.
use super::business_json::{self as bj, TextValue};
pub(crate) use super::history::NativeCutProjection;
use super::object_records::{BatchOwner, BatchView, LifeOwner, LifeView};
use super::{
    generation::ProducedObject,
    object_records::{self, BatchRef, BorrowedObject, PublicationLife},
};
use crate::v3::audit_store::{CodecRead, CodecStore};
use crate::{canonical_json, content_store, object_diff};
use newchan_rust::s_content::{
    self as sc, path::PathStep, Built, ContentRef, Kind, NodeDigest, NodeSource, NodeStore,
    Value as V,
};
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::{params, Transaction};
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};

pub(super) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS s_content_objects (
 object_id TEXT PRIMARY KEY, object_revision INTEGER NOT NULL, kind TEXT NOT NULL,
 fact_key TEXT, window_start INTEGER, window_mid INTEGER, window_end INTEGER,
 generated_root BLOB NOT NULL CHECK(typeof(generated_root)='blob' AND length(generated_root)=32),
 internal_root BLOB NOT NULL CHECK(typeof(internal_root)='blob' AND length(internal_root)=32),
 immutable_root BLOB NOT NULL CHECK(typeof(immutable_root)='blob' AND length(immutable_root)=32),
 birth_wire_root BLOB NOT NULL CHECK(typeof(birth_wire_root)='blob' AND length(birth_wire_root)=32),
 batch_id TEXT NOT NULL, first_known_generation INTEGER NOT NULL CHECK(first_known_generation>=1),
 first_known_cut TEXT NOT NULL, published_generation INTEGER NOT NULL,
 withdrawn_generation INTEGER, withdrawal_reason TEXT, superseded_by TEXT);";
pub(crate) fn init_schema(tx: &Transaction<'_>) -> Result<(), String> {
    content_store::ensure_format(tx).map_err(err)?;
    tx.execute_batch(SCHEMA).map_err(err)
}
fn err(e: impl std::fmt::Display) -> String {
    format!("ObjectIndexInvalid：{e}")
}
fn f(k: &str) -> PathStep {
    PathStep::Field(k.into())
}
fn field(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    s.lookup_path(r, &[f(k)]).map_err(err)
}
fn digest(r: &ContentRef) -> Result<NodeDigest, String> {
    r.digest().map_err(err)
}
fn built(r: ContentRef) -> Result<Built, String> {
    Ok(Built {
        digest: digest(&r)?,
        kind: r.kind,
        extent: r.extent,
    })
}
fn scalar(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<Value, String> {
    let r = field(s, r, k)?;
    scalar_value(s, &r)
}
fn scalar_value(s: &mut (impl CodecRead + ?Sized), r: &ContentRef) -> Result<Value, String> {
    if !matches!(r.kind, Kind::Number | Kind::String | Kind::Null) {
        return Err(err("预期有界标量"));
    }
    let mut out = Vec::new();
    s.render_canonical(r, &mut out).map_err(err)?;
    newchan_rust::session_protocol::strict_json(&out)
}

/// 同一原件绑定调用中的固定小字段；大载荷只保存引用，不持久缓存或信任摘要替代闭包。
pub(super) struct Fields(sc::CodecFields);
impl Fields {
    fn read(
        s: &mut (impl CodecStore + ?Sized),
        root: &ContentRef,
        names: &[&str],
    ) -> Result<Self, String> {
        s.lookup_object_fields(root, names).map(Self).map_err(err)
    }
    pub(super) fn read_subset(
        s: &mut (impl CodecStore + ?Sized),
        root: &ContentRef,
        names: &[&str],
    ) -> Result<Self, String> {
        s.lookup_object_fields_subset(root, names)
            .map(Self)
            .map_err(err)
    }
    pub(super) fn get(&self, name: &str) -> Result<&ContentRef, String> {
        self.0
            .get(name)
            .ok_or_else(|| err(format!("原件缺字段{name}")))
    }
    pub(super) fn text(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        name: &str,
    ) -> Result<String, String> {
        scalar_value(s, self.get(name)?)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| err(format!("{name}不是文本")))
    }
    pub(super) fn integer(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        name: &str,
    ) -> Result<i64, String> {
        scalar_value(s, self.get(name)?)?
            .as_i64()
            .ok_or_else(|| err(format!("{name}不是i64")))
    }
    fn equal(&self, key: &str, other: &Self, other_key: &str) -> Result<(), String> {
        if self.get(key)? != other.get(other_key)? {
            return Err(err(format!("{key}/{other_key}内容边不同")));
        }
        Ok(())
    }
    fn slot(&self, s: &mut (impl CodecStore + ?Sized), kind: &str) -> Result<Slot, String> {
        if kind == SHAPE {
            Ok(Slot::Shape([
                self.integer(s, "window_start")?,
                self.integer(s, "window_mid")?,
                self.integer(s, "window_end")?,
            ]))
        } else {
            Ok(Slot::Typed {
                kind: kind.into(),
                fact_key: self.text(s, "fact_key")?.into(),
            })
        }
    }
    fn immutable(&self, s: &mut (impl CodecStore + ?Sized), shape: bool) -> Result<Built, String> {
        let names = if shape {
            SHAPE_IMMUTABLE
        } else {
            TYPED_IMMUTABLE
        };
        let mut fields = Vec::new();
        for name in names {
            let child = self.get(name)?;
            if child.kind == Kind::Null {
                return Err(err("不可变字段不得为空"));
            }
            fields.push(((*name).into(), V::Edge(digest(child)?)));
        }
        s.write_value(&V::Object(fields)).map_err(err)
    }
    fn bind_json_text(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        json: &str,
        value: &str,
    ) -> Result<(), String> {
        let parsed = s
            .lookup_path(self.get(json)?, &[PathStep::JsonText])
            .map_err(err)?;
        if &parsed != self.get(value)? {
            return Err(err("内部JSON视图与值边不同"));
        }
        Ok(())
    }
}
fn text(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<String, String> {
    scalar(s, r, k)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| err(format!("{k}不是文本")))
}
fn integer(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<i64, String> {
    scalar(s, r, k)?
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是i64")))
}
/// 本口只取真实节点头；load返回前统一验证这些根的全部闭包。
fn root_header(s: &mut (impl CodecRead + ?Sized), b: Vec<u8>) -> Result<Built, String> {
    let d: NodeDigest = b.try_into().map_err(|_| err("根不是32字节"))?;
    let r = s.content_ref_of(&sc::root_string(&d)).map_err(err)?;
    built(r)
}
const SHAPE: &str = "CC-006.local_shape";
const TYPED_IMMUTABLE: &[&str] = &[
    "object_revision",
    "kind",
    "fact_key",
    "payload_json",
    "input_refs_json",
    "source_coords_json",
];
const SHAPE_IMMUTABLE: &[&str] = &[
    "object_revision",
    "kind",
    "branch",
    "dir_ab",
    "dir_bc",
    "window_start",
    "window_mid",
    "window_end",
    "comparisons_json",
    "input_refs_json",
    "source_coords_json",
];

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    Shape([i64; 3]),
    Typed {
        kind: TextValue,
        fact_key: TextValue,
    },
}
impl Slot {
    fn key(&self) -> String {
        match self {
            Self::Shape(w) => canonical_json(&json!([0, w[0], w[1], w[2]])),
            Self::Typed { kind, fact_key } => canonical_json(&json!([1, kind, fact_key])),
        }
    }
    fn is_shape(&self) -> bool {
        matches!(self, Self::Shape(_))
    }
}
#[derive(Debug)]
pub(super) struct Entry {
    pub id: TextValue,
    pub revision: i64,
    pub kind: TextValue,
    pub slot: Slot,
    pub generated: Built,
    pub internal: Built,
    pub immutable: Built,
    pub birth_wire: RefValue,
    pub life: LifeOwner,
}
pub(super) struct Plan {
    pub generation: i64,
    pub index: Vec<Entry>,
    pub births: Vec<Entry>,
    pub withdrawals: Vec<Value>,
    pub replaces: Vec<Value>,
    pub active: Built,
    pub withdrawn: Built,
    pub upserts: Built,
}
fn slot(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, kind: &str) -> Result<Slot, String> {
    if kind == SHAPE {
        Ok(Slot::Shape([
            integer(s, r, "window_start")?,
            integer(s, r, "window_mid")?,
            integer(s, r, "window_end")?,
        ]))
    } else {
        Ok(Slot::Typed {
            kind: kind.into(),
            fact_key: text(s, r, "fact_key")?.into(),
        })
    }
}
fn immutable(
    s: &mut (impl CodecStore + ?Sized),
    r: &ContentRef,
    is_shape: bool,
) -> Result<Built, String> {
    Fields::read(
        s,
        r,
        if is_shape {
            &object_records::SHAPE_SEALED_FIELDS
        } else {
            &object_records::TYPED_SEALED_FIELDS
        },
    )?
    .immutable(s, is_shape)
}
fn raw_withdrawal(e: &Entry, reason: &str, sup: Option<&str>) -> Value {
    match &e.slot {
        Slot::Shape(w) => {
            json!({"object_id":e.id,"window_start":w[0],"window_mid":w[1],"window_end":w[2],"reason":reason,"superseded_by":sup})
        }
        Slot::Typed { fact_key, .. } => {
            json!({"object_id":e.id,"fact_key":fact_key,"reason":reason,"superseded_by":sup})
        }
    }
}
fn ordering(a: &Entry, b: &Entry, dead: bool) -> std::cmp::Ordering {
    match (&a.slot, &b.slot) {
        (Slot::Shape(wa), Slot::Shape(wb)) => {
            if dead {
                a.life
                    .first_known_generation
                    .cmp(&b.life.first_known_generation)
            } else {
                wa[0].cmp(&wb[0])
            }
        }
        (Slot::Shape(_), _) => std::cmp::Ordering::Less,
        (_, Slot::Shape(_)) => std::cmp::Ordering::Greater,
        (
            Slot::Typed {
                kind: ka,
                fact_key: fa,
            },
            Slot::Typed {
                kind: kb,
                fact_key: fb,
            },
        ) => (ka, fa, &a.id, a.revision).cmp(&(kb, fb, &b.id, b.revision)),
    }
}
fn candidate(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, String> {
    let generated = o.generated().clone();
    s.verify_audit_fresh(&generated.content_ref(), sc::Mode::Public)
        .map_err(err)?;
    candidate_verified(s, o, g)
}
// #1394：仅本模块完整Public核验成功的控制流调用，不导出skip开关或成功token。
fn candidate_verified(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, String> {
    let generated = o.generated().clone();
    if integer(s, &generated.content_ref(), "first_known_generation")? != g
        || integer(s, &generated.content_ref(), "published_generation")? != g
        || text(s, &generated.content_ref(), "first_known_cut")? != format!("cut-{g}")
    {
        return Err(err("候选对象不属于本次生成代"));
    }
    let kind = text(s, &generated.content_ref(), "kind")?;
    let revision = integer(s, &generated.content_ref(), "object_revision")?;
    if text(s, &generated.content_ref(), "object_id")? != o.object_id() || kind != o.kind() {
        return Err(err("生成对象元数据不同"));
    }
    let slot = slot(s, &generated.content_ref(), &kind)?;
    let internal = match o.borrowed() {
        BorrowedObject::Typed(v) => v.record.clone(),
        BorrowedObject::Shape(v) => v.record.clone(),
        BorrowedObject::Fractal(v) => v.record.clone(),
    };
    let life = PublicationLife {
        first_known_generation: g,
        first_known_cut: format!("cut-{g}"),
        published_generation: g,
        withdrawn_generation: None,
        withdrawal_reason: None,
        superseded_by: None,
        batch: BatchRef::CurrentBatch,
    };
    let birth_wire = o
        .wire_template(s.generic_store(), &life, g)
        .map_err(err)?
        .content_ref();
    let immutable = immutable(s, &generated.content_ref(), slot.is_shape())?;
    Ok(Entry {
        id: o.object_id().into(),
        revision,
        kind: kind.into(),
        slot,
        generated,
        internal,
        immutable,
        birth_wire: birth_wire.into(),
        life: LifeOwner::from_normal(life),
    })
}

/// #1394：仅Native联合Public核验成功后，一次绑定当前generated字段。
/// 首读kind/字段结构的错误可提前拒绝；默认/Owned仍用candidate_verified。
#[cfg(feature = "s_capture_native_testonly")]
fn candidate_fields(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, String> {
    let generated = o.generated().clone();
    let root = generated.content_ref();
    let selected_kind = text(s, &root, "kind")?;
    let fields = Fields::read(
        s,
        &root,
        if selected_kind == SHAPE {
            &object_records::SHAPE_SEALED_FIELDS
        } else {
            &object_records::TYPED_SEALED_FIELDS
        },
    )?;
    if fields.integer(s, "first_known_generation")? != g
        || fields.integer(s, "published_generation")? != g
        || fields.text(s, "first_known_cut")? != format!("cut-{g}")
    {
        return Err(err("候选对象不属于本次生成代"));
    }
    let kind = fields.text(s, "kind")?;
    let revision = fields.integer(s, "object_revision")?;
    if fields.text(s, "object_id")? != o.object_id() || kind != selected_kind || kind != o.kind() {
        return Err(err("生成对象元数据不同"));
    }
    let slot = fields.slot(s, &kind)?;
    let internal = match o.borrowed() {
        BorrowedObject::Typed(v) => v.record.clone(),
        BorrowedObject::Shape(v) => v.record.clone(),
        BorrowedObject::Fractal(v) => v.record.clone(),
    };
    let life = PublicationLife {
        first_known_generation: g,
        first_known_cut: format!("cut-{g}"),
        published_generation: g,
        withdrawn_generation: None,
        withdrawal_reason: None,
        superseded_by: None,
        batch: BatchRef::CurrentBatch,
    };
    // internal与generated是不同原件，仍由原wire_template读取并核内部JSON/value边。
    let birth_wire = o
        .wire_template(s.generic_store(), &life, g)
        .map_err(err)?
        .content_ref();
    let immutable = fields.immutable(s, slot.is_shape())?;
    Ok(Entry {
        id: o.object_id().into(),
        revision,
        kind: kind.into(),
        slot,
        generated,
        internal,
        immutable,
        birth_wire: birth_wire.into(),
        life: LifeOwner::from_normal(life),
    })
}
/// 返回本代模板：新生维持08，已有记录只借出生字段并改写小生命周期元数据。
fn wire_at(s: &mut (impl CodecStore + ?Sized), e: &Entry, g: i64) -> Result<RefValue, String> {
    #[cfg(test)]
    WIRE_PROJECTIONS.with(|calls| calls.set(calls.get() + 1));
    let life = effective_life(e, g)?;
    let saved = e.life.view().batch;
    let built =
        object_records::reproject_stored_wire_view(s, e.birth_wire.view(), life, g).map_err(err)?;
    let mut wire = if s.allocation_budget().is_some() {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            RefValue::Owned(built.owned_ref())
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            return Err(err("owned引用执行未启用"));
        }
    } else {
        RefValue::Plain(built.content_ref())
    };
    if e.life.first_known_generation == g {
        if let BatchView::Existing(batch) = saved {
            wire = s
                .replace_ref(wire.view(), &[f("batch_id")], batch)
                .map_err(err)?;
        }
    }
    Ok(wire)
}

/// cut 上下文仍逐次建立；出生 CurrentBatch 是中间态，最终 wire 恢复固定首版批号。
fn effective_life(e: &Entry, g: i64) -> Result<LifeView<'_>, String> {
    if e.life.first_known_generation > g {
        return Err(err("对象尚未出生"));
    }
    let mut life = e.life.view();
    if life.withdrawn_generation.is_some_and(|d| d > g) {
        life.withdrawn_generation = None;
        life.withdrawal_reason = None;
        life.superseded_by = None;
    }
    if life.first_known_generation == g {
        life.batch = BatchView::CurrentBatch;
    }
    Ok(life)
}

/// 只能由本次 load 成功构造；私有索引只读，memo 随本轮历史审计销毁，不进入返回状态。
pub(super) struct HistoryObjects<'capture> {
    entries: sc::CodecVec<Entry>,
    native: Option<NativeProjection<'capture>>,
}
struct NativeProjection<'capture> {
    capture: &'capture sc::AuditCapture,
    // 序号由私有、不可变 entries 给定；同一对象只有 active / withdrawn 两个最终 wire。
    wires: Vec<[Option<RefValue>; 2]>,
}
impl NativeProjection<'_> {
    fn wire(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        e: &Entry,
        ordinal: usize,
        g: i64,
    ) -> Result<RefValue, String> {
        let life = effective_life(e, g)?;
        let phase = usize::from(life.withdrawn_generation.is_some());
        if let Some(wire) = &self.wires[ordinal][phase] {
            // 命中也必须按本 cut 跑唯一生命周期判据；不能拿旧代成功代替当前准入。
            object_records::validate_life_view(life, g, e.id.as_str()).map_err(err)?;
            self.capture
                .verify_refview(wire.view(), sc::Mode::Public)
                .map_err(err)?;
            return Ok(wire.clone());
        }
        let wire = wire_at(s, e, g)?;
        // clear_overlay 每 cut 清除派生 body。只允许当前 capture 自己证明的完整 Public
        // 闭包进入 memo，不保留临时 overlay 引用，也不靠 root 存在替代闭包检查。
        self.capture
            .verify_refview(wire.view(), sc::Mode::Public)
            .map_err(err)?;
        self.wires[ordinal][phase] = Some(wire.clone());
        Ok(wire)
    }
}
impl HistoryObjects<'_> {
    pub(super) fn native_sweep(&self) -> Option<HistorySweep> {
        self.native
            .as_ref()
            .map(|_| HistorySweep::new(self.entries()))
    }
    pub(super) fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub(super) fn into_entries(self) -> sc::CodecVec<Entry> {
        self.entries
    }
    pub(super) fn at_cut(
        &mut self,
        s: &mut super::history::HashReader<'_>,
        g: i64,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<(Built, Built, Built), String> {
        if let Some(native) = &self.native {
            if budget.is_some()
                || !s
                    .native_projection_capture()
                    .is_some_and(|current| std::ptr::eq(current, native.capture))
            {
                return Err(err("Native投影不属于本次捕获审计上下文"));
            }
        }
        at_cut_projected(s, &self.entries, g, budget, self.native.as_mut())
    }
}

impl HistoryObjects<'_> {
    /// Only this complete-load Native owner can produce both projections. No arbitrary arrays.
    pub(super) fn native_dual_at_cut(
        &mut self,
        s: &mut super::history::HashReader<'_>,
        g: i64,
        batch: &str,
    ) -> Result<[(Built, Built); 3], String> {
        let native = self
            .native
            .as_mut()
            .ok_or_else(|| err("缺Native完整索引资格"))?;
        if !s
            .native_projection_capture()
            .is_some_and(|c| std::ptr::eq(c, native.capture))
        {
            return Err(err("双投影capture不符"));
        }
        let mut active = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.life.first_known_generation <= g
                    && e.life.withdrawn_generation.is_none_or(|d| d > g)
            })
            .collect::<Vec<_>>();
        let mut dead = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.life.withdrawn_generation.is_some_and(|d| d <= g))
            .collect::<Vec<_>>();
        active.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
        dead.sort_unstable_by(|a, b| ordering(a.1, b.1, true).then(a.0.cmp(&b.0)));
        let births = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.life.first_known_generation == g)
            .collect::<Vec<_>>();
        // Local to this cut: derived Template references never enter persistent Public slots.
        let mut pairs = std::collections::BTreeMap::<usize, (RefValue, RefValue)>::new();
        let mut collect = |items: Vec<(usize, &Entry)>| -> Result<(Built, Built), String> {
            let mut template = Vec::new();
            let mut public = Vec::new();
            for (ordinal, e) in items {
                if e.life.published_generation > g {
                    return Err(err("对象出生后发布代号被改写"));
                }
                let saved = match e.life.view().batch {
                    BatchView::Existing(v) => v,
                    BatchView::CurrentBatch => return Err(err("历史索引未绑定实际出生batch")),
                };
                if (e.life.first_known_generation == g) != (saved == batch) {
                    return Err(err("出生/旧代batch与当前cut不符"));
                }
                if !pairs.contains_key(&ordinal) {
                    let life = effective_life(e, g)?;
                    object_records::validate_life_view(life, g, e.id.as_str()).map_err(err)?;
                    let phase = usize::from(life.withdrawn_generation.is_some());
                    let pair = if e.life.first_known_generation == g {
                        #[cfg(test)]
                        WIRE_PROJECTIONS.with(|v| v.set(v.get() + 1));
                        let b = object_records::reproject_stored_wire_view(
                            s,
                            e.birth_wire.view(),
                            life,
                            g,
                        )
                        .map_err(err)?;
                        let template = RefValue::Plain(b.content_ref());
                        let public = s
                            .replace_ref(template.view(), &[f("batch_id")], saved)
                            .map_err(err)?;
                        native
                            .capture
                            .verify_refview(public.view(), sc::Mode::Public)
                            .map_err(err)?;
                        native.wires[ordinal][phase] = Some(public.clone());
                        (template, public)
                    } else {
                        let public = native.wire(s, e, ordinal, g)?;
                        (public.clone(), public)
                    };
                    pairs.insert(ordinal, pair);
                }
                let (a, b) = pairs.get(&ordinal).unwrap();
                template.push(a.clone());
                public.push(b.clone());
            }
            Ok((array(s, &template)?, array(s, &public)?))
        };
        Ok([collect(active)?, collect(dead)?, collect(births)?])
    }
}

fn array(s: &mut (impl CodecStore + ?Sized), refs: &[RefValue]) -> Result<Built, String> {
    let allocation = s.allocation_budget();
    let mut views = bj::new_work(allocation.as_ref());
    for r in refs {
        views
            .try_push(sc::ValueView::Edge(r.digest().map_err(err)?))
            .map_err(err)?;
    }
    s.write_template_view(sc::ValueView::Array(&views))
        .map_err(err)
}
/// 完整历史索引按目标代投影。Delta 出生序保留行插入序；活动/撤回各遵旧公开主序。
pub(super) fn at_cut(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    g: i64,
) -> Result<(Built, Built, Built), String> {
    at_cut_metered(s, index, g, None)
}
pub(super) fn at_cut_metered(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(Built, Built, Built), String> {
    at_cut_projected(s, index, g, budget, None)
}
fn at_cut_projected(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
    mut native: Option<&mut NativeProjection<'_>>,
) -> Result<(Built, Built, Built), String> {
    let allocation = s.allocation_budget();
    let mut active = bj::new_work(allocation.as_ref());
    let mut dead = bj::new_work(allocation.as_ref());
    // 三份工作集按旧求值顺序建立；尚未读取wire，budget gate仍在逐对象投影之前。
    for (ordinal, e) in index.iter().enumerate().filter(|(_, e)| {
        e.life.first_known_generation <= g && e.life.withdrawn_generation.is_none_or(|d| d > g)
    }) {
        active.try_push((ordinal, e)).map_err(err)?;
    }
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.withdrawn_generation.is_some_and(|d| d <= g))
    {
        dead.try_push((ordinal, e)).map_err(err)?;
    }
    active.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
    dead.sort_unstable_by(|a, b| ordering(a.1, b.1, true).then(a.0.cmp(&b.0)));
    let mut births = bj::new_work(allocation.as_ref());
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.first_known_generation == g)
    {
        births.try_push((ordinal, e)).map_err(err)?;
    }
    let mut collect = |items: sc::CodecVec<(usize, &Entry)>| -> Result<Built, String> {
        let mut refs = bj::new_work(allocation.as_ref());
        for (ordinal, e) in items {
            if let Some(b) = budget {
                b.record().map_err(err)?;
            }
            let wire = match native.as_deref_mut() {
                Some(native) => native.wire(s, e, ordinal, g)?,
                None => wire_at(s, e, g)?,
            };
            refs.try_push(wire).map_err(err)?;
        }
        array(s, &refs)
    };
    Ok((collect(active)?, collect(dead)?, collect(births)?))
}

pub(super) fn plan(
    s: &mut (impl CodecStore + ?Sized),
    objects: &[ProducedObject],
    old: &[Entry],
    g: i64,
) -> Result<Plan, String> {
    if g < 1 {
        return Err(err("非法代号"));
    }
    let new = objects
        .iter()
        .map(|o| candidate(s, o, g))
        .collect::<Result<Vec<_>, _>>()?;
    plan_from_candidates(s, new, old, g)
}

/// #1394：只由Runtime显式CapturedNative模式选择，预算存在时拒绝。
/// 联合核验同事务生成完成的Public根，先于任何candidate wire/immutable写入。
/// 多处坏状态或wire故障的首报次序可改变，不扩至默认/Owned/Job路径。
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn plan_native_union_for_testonly(
    s: &mut (impl CodecStore + ?Sized),
    objects: &[ProducedObject],
    old: &[Entry],
    g: i64,
) -> Result<Plan, String> {
    if g < 1 {
        return Err(err("非法代号"));
    }
    if s.allocation_budget().is_some() {
        return Err(err("Native候选联合核验拒绝allocation预算"));
    }
    s.verify_audit_many_fresh(
        objects.iter().map(|o| o.generated().content_ref()),
        sc::Mode::Public,
    )
    .map_err(err)?;
    let new = objects
        .iter()
        .map(|o| candidate_fields(s, o, g))
        .collect::<Result<Vec<_>, _>>()?;
    plan_from_candidates(s, new, old, g)
}
fn plan_from_candidates(
    s: &mut (impl CodecStore + ?Sized),
    new: Vec<Entry>,
    old: &[Entry],
    g: i64,
) -> Result<Plan, String> {
    let mut ids = BTreeSet::new();
    for e in &new {
        if !ids.insert(e.id.try_clone()?) {
            return Err(err("生成集重复对象身份"));
        }
    }
    let mut order = (0..old.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| ordering(&old[*a], &old[*b], false));
    let allocation = s.allocation_budget();
    let mut old_keys = bj::new_work(allocation.as_ref());
    for i in &order {
        let e = &old[*i];
        old_keys
            .try_push(object_diff::Existing {
                key: object_diff::Key {
                    object_id: e.id.as_str(),
                    slot: &e.slot,
                    immutable: object_diff::Ready(e.immutable.digest),
                },
                active: e.life.withdrawn_generation.is_none(),
            })
            .map_err(err)?;
    }
    let mut keys = bj::new_work(allocation.as_ref());
    for e in &new {
        keys.try_push(object_diff::Key {
            object_id: e.id.as_str(),
            slot: &e.slot,
            immutable: object_diff::Ready(e.immutable.digest),
        })
        .map_err(err)?;
    }
    let changes = object_diff::plan_budgeted(&keys, &old_keys, allocation.as_ref())?;
    let mut index = Vec::new();
    for e in old {
        index.push(e.try_clone()?);
    }
    let mut withdrawals = Vec::new();
    let mut replaces = Vec::new();
    for w in changes.withdrawals {
        let e = &mut index[order[w.old_index]];
        let (reason, sup) = match w.removal {
            object_diff::Removal::Removed => (
                if e.slot.is_shape() {
                    "window_removed"
                } else {
                    "fact_removed"
                },
                None,
            ),
            object_diff::Removal::Superseded => (
                "superseded_by_revision",
                Some(
                    new[w.new_index.ok_or_else(|| err("替代缺新对象"))?]
                        .id
                        .try_clone()?,
                ),
            ),
        };
        withdrawals.push(raw_withdrawal(e, reason, sup.as_deref()));
        e.life.withdrawn_generation = Some(g);
        e.life.withdrawal_reason = Some(reason.into());
        e.life.superseded_by = sup;
    }
    for r in changes.replaces {
        replaces.push(
            json!({"new_object_id":new[r.new_index].id,"old_object_id":old[order[r.old_index]].id}),
        );
    }
    let births = changes
        .births
        .into_iter()
        .map(|i| new[i].try_clone())
        .collect::<Result<Vec<_>, String>>()?;
    for e in &births {
        index.push(e.try_clone()?);
    }
    let (active, withdrawn, upserts) = at_cut(s, &index, g)?;
    Ok(Plan {
        generation: g,
        index,
        births,
        withdrawals,
        replaces,
        active,
        withdrawn,
        upserts,
    })
}

/// 对持久三种记录逐字段对拍，不展开 payload/input_refs/source_coords 大载荷。
fn bind_originals(
    s: &mut (impl CodecStore + ?Sized),
    e: &Entry,
    leaf_bindings: &mut Option<ShapeLeafBindings<'_>>,
) -> Result<(), String> {
    let shape = e.slot.is_shape();
    let generated_ref = built_ref(s, &e.generated);
    let internal_ref = built_ref(s, &e.internal);
    let r = OwnerFields::read(
        s,
        generated_ref.view(),
        if shape {
            &object_records::SHAPE_SEALED_FIELDS
        } else {
            &object_records::TYPED_SEALED_FIELDS
        },
    )?;
    let i = OwnerFields::read(
        s,
        internal_ref.view(),
        if shape {
            &object_records::SHAPE_RECORD_FIELDS
        } else {
            &object_records::TYPED_RECORD_FIELDS
        },
    )?;
    let w = OwnerFields::read(
        s,
        e.birth_wire.view(),
        if shape {
            &object_records::SHAPE_WIRE_FIELDS
        } else {
            &object_records::TYPED_WIRE_FIELDS
        },
    )?;
    for key in [
        "object_id",
        "object_revision",
        "kind",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ] {
        r.equal(key, &i, key)?;
    }
    for key in if shape {
        &SHAPE_IMMUTABLE[1..]
    } else {
        &TYPED_IMMUTABLE[1..]
    } {
        if *key == "comparisons_json" {
            continue;
        }
        r.equal(key, &i, key)?;
    }
    if r.text(s, "object_id")? != e.id
        || r.integer(s, "object_revision")? != e.revision
        || r.text(s, "kind")? != e.kind
        || r.slot(s, &e.kind)? != e.slot
    {
        return Err(err("对象索引与生成原件不符"));
    }
    for key in ["first_known_generation", "published_generation"] {
        if r.integer(s, key)? != e.life.first_known_generation {
            return Err(err("出生生成代号不符"));
        }
    }
    if r.text(s, "first_known_cut")? != e.life.first_known_cut
        || e.life.published_generation != e.life.first_known_generation
    {
        return Err(err("出生cut/发布代不符"));
    }
    for (json, value) in [
        ("input_refs_json", "input_refs"),
        ("source_coords_json", "source_coords"),
    ] {
        i.bind_json_text(s, json, value)?;
    }
    i.equal("source_coords", &w, "source_coords")?;
    if shape {
        for key in ["branch", "dir_ab", "dir_bc"] {
            i.equal(key, &w, key)?;
        }
        for key in ["window_start", "window_mid", "window_end"] {
            if w.text(s, key)? != i.integer(s, key)?.to_string() {
                return Err(err("窗口公开投影不符"));
            }
        }
        let comparison = i.get("comparisons")?;
        let mut out = Vec::new();
        s.render_ref(comparison.view(), &mut out).map_err(err)?;
        if r.text(s, "comparisons_json")?.as_bytes() != out {
            return Err(err("comparisons文本与内部值不符"));
        }
        i.equal("comparisons", &w, "comparisons")?;
        let generated_refs = i.get("input_refs")?;
        let wire_refs = w.get("input_refs")?;
        bind_shape_refs_view(s, generated_refs.view(), wire_refs.view(), leaf_bindings)?;
    } else {
        i.bind_json_text(s, "payload_json", "payload")?;
        for key in ["fact_key", "payload", "input_refs"] {
            i.equal(key, &w, key)?;
        }
    }
    if w.text(s, "object_id")? != e.id
        || w.text(s, "kind")? != e.kind
        || w.text(s, "object_revision")? != e.revision.to_string()
    {
        return Err(err("出生公开身份不符"));
    }
    if r.immutable(s, shape)? != e.immutable {
        return Err(err("不可变内容根不符"));
    }
    if wire_at(s, e, e.life.first_known_generation)? != e.birth_wire {
        return Err(err("出生公开元数据与独立记录不同"));
    }
    Ok(())
}
fn bind_shape_refs(
    s: &mut (impl CodecStore + ?Sized),
    generated: &ContentRef,
    wire: &ContentRef,
) -> Result<(), String> {
    bind_shape_refs_view(s, generated.into(), wire.into(), &mut None)
}

type ShapeLeafDeclaration = (String, String, Kind, Option<u64>);
/// #1394：只活在一次完整 Native load 中，保存来源叶八字段关系的成功结果。
/// capture 从本次实际 HashReader 的正资格入口取得，不接受调用方预填 memo。
struct ShapeLeafBindings<'capture> {
    capture: &'capture sc::AuditCapture,
    successful: HashSet<[ShapeLeafDeclaration; 2]>,
}
impl ShapeLeafBindings<'_> {
    fn bind(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        ar: sc::RefView<'_>,
        br: sc::RefView<'_>,
    ) -> Result<(), String> {
        let result = (|| {
            // 每次在查 memo 前判资格；overlay-only 仍跑原叶判据，不获缓存资格。
            let ad = ar.digest().map_err(err)?;
            let bd = br.digest().map_err(err)?;
            if !self.capture.contains(&ad) || !self.capture.contains(&bd) {
                return bind_shape_leaf(s, ar, br);
            }
            // 两端自有时必须核完整声明及 capture 自己的 Public 闭包。
            // 自有根缺深叶/坏原件是实际错误，overlay 不得替当前 capture 补证。
            self.capture
                .verify_refview(ar, sc::Mode::Public)
                .map_err(err)?;
            self.capture
                .verify_refview(br, sc::Mode::Public)
                .map_err(err)?;
            let declaration = |r: sc::RefView<'_>| {
                (
                    r.format().to_owned(),
                    r.root().to_owned(),
                    r.kind(),
                    r.extent(),
                )
            };
            let key = [declaration(ar), declaration(br)];
            if !self.successful.contains(&key) {
                bind_shape_leaf(s, ar, br)?;
                self.successful.insert(key);
            }
            Ok(())
        })();
        if result.is_err() {
            self.successful.clear();
        }
        result
    }
}
fn bind_shape_leaf(
    s: &mut (impl CodecStore + ?Sized),
    ar: sc::RefView<'_>,
    br: sc::RefView<'_>,
) -> Result<(), String> {
    #[cfg(test)]
    tests::SHAPE_LEAF_CHECKS.with(|checks| checks.set(checks.get() + 1));
    if ar.extent() != Some(8) || br.extent() != Some(8) {
        return Err(err("来源字段集不符"));
    }
    let names = [
        "identity_key",
        "payload_hash",
        "receipt_id",
        "event_id",
        "input_revision",
        "source_coord",
        "revision",
        "seq",
    ];
    let a_fields = OwnerFields::read(s, ar, &names)?;
    let b_fields = OwnerFields::read(s, br, &names)?;
    for field_name in [
        "identity_key",
        "payload_hash",
        "receipt_id",
        "event_id",
        "input_revision",
        "source_coord",
    ] {
        a_fields.equal(field_name, &b_fields, field_name)?;
    }
    for field_name in ["revision", "seq"] {
        if b_fields.text(s, field_name)? != a_fields.integer(s, field_name)?.to_string() {
            return Err(err("来源数字公开投影不符"));
        }
    }
    Ok(())
}
fn bind_shape_refs_view(
    s: &mut (impl CodecStore + ?Sized),
    generated: sc::RefView<'_>,
    wire: sc::RefView<'_>,
    leaf_bindings: &mut Option<ShapeLeafBindings<'_>>,
) -> Result<(), String> {
    if generated.kind() != Kind::Array
        || generated.extent() != Some(3)
        || wire.kind() != Kind::Array
        || wire.extent() != Some(3)
    {
        return Err(err("形态来源不是三组"));
    }
    for n in 0..3 {
        let a = s
            .lookup_ref(generated, &[PathStep::Index(n)])
            .map_err(err)?;
        let b = s.lookup_ref(wire, &[PathStep::Index(n)]).map_err(err)?;
        if a.extent() != Some(3)
            || b.extent() != Some(3)
            || text_ref(s, b.view(), "merged_index")?
                != integer_ref(s, a.view(), "merged_index")?.to_string()
        {
            return Err(err("合并序号公开投影不符"));
        }
        for key in ["raw_refs", "dependency_refs"] {
            let aa = field_ref(s, a.view(), key)?;
            let bb = field_ref(s, b.view(), key)?;
            if aa.kind() != Kind::Array || bb.kind() != Kind::Array || aa.extent() != bb.extent() {
                return Err(err("来源数组长度不符"));
            }
            let items = if s.audit_full_array_scan() {
                Some((
                    s.lookup_array_ref(aa.view()).map_err(err)?,
                    s.lookup_array_ref(bb.view()).map_err(err)?,
                ))
            } else {
                None
            };
            for k in 0..aa.extent().ok_or_else(|| err("来源缺长度"))? {
                let (ar, br) = if let Some((a, b)) = &items {
                    (a[k as usize].clone(), b[k as usize].clone())
                } else {
                    // Some 仍逐位置交错原件读取与字段/业务检查，预算与期限门顺序不变。
                    (
                        s.lookup_ref(aa.view(), &[PathStep::Index(k)])
                            .map_err(err)?,
                        s.lookup_ref(bb.view(), &[PathStep::Index(k)])
                            .map_err(err)?,
                    )
                };
                if let Some(bindings) = leaf_bindings {
                    bindings.bind(s, ar.view(), br.view())?;
                } else {
                    bind_shape_leaf(s, ar.view(), br.view())?;
                }
            }
        }
    }
    Ok(())
}
/// 顺序保留真实出生插入序；公开视图和 diff 在各自边界按原合同排序。
pub(super) fn load(
    tx: &Transaction<'_>,
    s: &mut dyn NodeStore,
    latest: i64,
) -> Result<sc::CodecVec<Entry>, String> {
    load_in_store(
        tx,
        &mut super::audit_store::StoreView::generic(s),
        latest,
        None,
    )
}
pub(super) fn load_for_audit(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'_>,
    latest: i64,
) -> Result<sc::CodecVec<Entry>, String> {
    let capture = s.native_projection_capture();
    load_in_store(
        tx,
        &mut super::audit_store::StoreView::for_audit(s),
        latest,
        capture,
    )
}
pub(super) fn load_for_history<'capture>(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'capture>,
    latest: i64,
) -> Result<HistoryObjects<'capture>, String> {
    // 不接受外部旧 index/memo。先全核本事务原件，成功之后才能开当前索引的两个槽。
    let entries = load_for_audit(tx, s, latest)?;
    let native = s
        .native_projection_capture()
        .map(|capture| NativeProjection {
            capture,
            wires: vec![[None, None]; entries.len()],
        });
    Ok(HistoryObjects { entries, native })
}
fn load_in_store(
    tx: &Transaction<'_>,
    s: &mut super::audit_store::StoreView<'_, '_>,
    latest: i64,
    capture: Option<&sc::AuditCapture>,
) -> Result<sc::CodecVec<Entry>, String> {
    let mut leaf_bindings = capture.map(|capture| ShapeLeafBindings {
        capture,
        successful: HashSet::new(),
    });
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT object_id,object_revision,kind,fact_key,window_start,window_mid,window_end,generated_root,internal_root,immutable_root,birth_wire_root,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects ORDER BY rowid").map_err(err)?;
    let allocation = s.allocation_budget();
    let mut out = bj::new_work(allocation.as_ref());
    while let Some(mut row) = stmt.next().map_err(err)? {
        let kind = TextValue::from_sql(&mut row, 2, allocation.as_ref()).map_err(err)?;
        let fact_key = TextValue::optional_sql(&mut row, 3, allocation.as_ref()).map_err(err)?;
        let coords = [
            row.optional_integer(4).map_err(err)?,
            row.optional_integer(5).map_err(err)?,
            row.optional_integer(6).map_err(err)?,
        ];
        let slot = if kind == SHAPE {
            if fact_key.is_some() {
                return Err(err("形态不得带typed槽位"));
            }
            Slot::Shape([
                coords[0].ok_or_else(|| err("缺窗口"))?,
                coords[1].ok_or_else(|| err("缺窗口"))?,
                coords[2].ok_or_else(|| err("缺窗口"))?,
            ])
        } else {
            if coords != [None, None, None] {
                return Err(err("typed不得带形态窗口"));
            }
            Slot::Typed {
                kind: kind.try_clone()?,
                fact_key: fact_key.ok_or_else(|| err("缺typed槽位"))?,
            }
        };
        let e = Entry {
            id: TextValue::from_sql(&mut row, 0, allocation.as_ref()).map_err(err)?,
            revision: row.integer(1).map_err(err)?,
            kind,
            slot,
            generated: root_header_bytes(s, row.blob(7).map_err(err)?)?,
            internal: root_header_bytes(s, row.blob(8).map_err(err)?)?,
            immutable: root_header_bytes(s, row.blob(9).map_err(err)?)?,
            birth_wire: root_ref_bytes(s, row.blob(10).map_err(err)?)?,
            life: LifeOwner {
                batch: BatchOwner::Existing(
                    TextValue::from_sql(&mut row, 11, allocation.as_ref()).map_err(err)?,
                ),
                first_known_generation: row.integer(12).map_err(err)?,
                first_known_cut: TextValue::from_sql(&mut row, 13, allocation.as_ref())
                    .map_err(err)?,
                published_generation: row.integer(14).map_err(err)?,
                withdrawn_generation: row.optional_integer(15).map_err(err)?,
                withdrawal_reason: TextValue::optional_sql(&mut row, 16, allocation.as_ref())
                    .map_err(err)?,
                superseded_by: TextValue::optional_sql(&mut row, 17, allocation.as_ref())
                    .map_err(err)?,
            },
        };
        if e.life.first_known_generation > latest
            || e.life.withdrawn_generation.is_some_and(|d| d > latest)
        {
            return Err(err("出生代超过已发布代"));
        }
        out.try_push(e).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    // 一次调用复用codec既有的上下文记忆；每根声明仍核，所有来源/生命周期绑定仍逐行执行。
    #[cfg(test)]
    super::state::trace_event("object_index_closures", None);
    let allocation = s.allocation_budget();
    let mut refs = bj::new_work(allocation.as_ref());
    for e in &out {
        for b in [&e.generated, &e.internal, &e.immutable] {
            refs.try_push(built_ref(s, b)).map_err(err)?;
        }
        refs.try_push(e.birth_wire.clone()).map_err(err)?;
    }
    s.verify_many_ref(refs.iter().map(RefValue::view), sc::Mode::Public)
        .map_err(err)?;

    for e in &out {
        #[cfg(test)]
        super::state::trace_event(
            "object_index_bind_originals",
            Some(e.life.first_known_generation),
        );
        bind_originals(s, e, &mut leaf_bindings)?;
        #[cfg(test)]
        super::state::trace_event("object_index_wire_at", Some(e.life.first_known_generation));
        wire_at(s, e, latest)?;
        if let Some(death) = e.life.withdrawn_generation {
            match (
                e.life.withdrawal_reason.as_deref(),
                e.life.superseded_by.as_deref(),
            ) {
                (Some("superseded_by_revision"), Some(id)) => {
                    let target = out
                        .iter()
                        .find(|v| v.id == id)
                        .ok_or_else(|| err("替代目标不存在"))?;
                    if target.slot != e.slot
                        || target.life.first_known_generation > death
                        || target.life.withdrawn_generation.is_some_and(|d| d <= death)
                    {
                        return Err(err("替代目标槽位/出生代不同"));
                    }
                }
                (Some(reason), None)
                    if reason
                        == if e.slot.is_shape() {
                            "window_removed"
                        } else {
                            "fact_removed"
                        } => {}
                _ => return Err(err("撤回原因与替代身份不符")),
            }
        }
    }
    #[cfg(test)]
    super::state::trace_event("object_index_done", None);
    Ok(out)
}
/// 内部 prior 只来自上代活动CC010/CC056；调用者保持本Vec活到generate结束。
pub(super) fn prior_records(index: &[Entry]) -> Result<(Vec<Built>, Option<Built>), String> {
    let mut bi = Vec::new();
    let mut version = None;
    for e in index
        .iter()
        .filter(|e| e.life.withdrawn_generation.is_none())
    {
        if e.kind == "CC-010.bi" {
            bi.push(e.internal.clone());
        }
        if e.kind == "CC-056.version" {
            if version.replace(e.internal.clone()).is_some() {
                return Err(err("活动版本对象不唯一"));
            }
        }
    }
    Ok((bi, version))
}
pub(super) fn stage(
    tx: &Transaction<'_>,
    s: &mut (impl CodecStore + ?Sized),
    p: &Plan,
    batch: &str,
) -> Result<(), String> {
    for e in &p.births {
        if e.life.first_known_generation != p.generation || e.life.batch != BatchOwner::CurrentBatch
        {
            return Err(err("新生上下文不符"));
        }
        let wire = s
            .replace_ref(e.birth_wire.view(), &[f("batch_id")], batch)
            .map_err(err)?;
        s.verify_many_ref_fresh([wire.view()], sc::Mode::Public)
            .map_err(err)?;
        let (fact, w) = match &e.slot {
            Slot::Shape(w) => (None, Some(*w)),
            Slot::Typed { fact_key, .. } => (Some(fact_key.as_str()), None),
        };
        tx.execute("INSERT INTO s_content_objects VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?13,NULL,NULL,NULL)",params![e.id,e.revision,e.kind,fact,w.map(|v|v[0]),w.map(|v|v[1]),w.map(|v|v[2]),e.generated.digest.as_slice(),e.internal.digest.as_slice(),e.immutable.digest.as_slice(),wire.digest().map_err(err)?.as_slice(),batch,p.generation,format!("cut-{}",p.generation)]).map_err(err)?;
    }
    for w in &p.withdrawals {
        let changed=tx.execute("UPDATE s_content_objects SET withdrawn_generation=?1,withdrawal_reason=?2,superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",params![p.generation,w["reason"].as_str(),w["superseded_by"].as_str(),w["object_id"].as_str()]).map_err(err)?;
        if changed != 1 {
            return Err(err("撤回身份缺失或已撤回"));
        }
    }
    Ok(())
}

/// 两种存储共用字段投影；Owned数组直接复制借用字段，不经过普通JSON所有者。
fn change_array(allocation: Option<&OwnedBudget>) -> Result<bj::JsonOwner, String> {
    if let Some(allocation) = allocation {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            return aj::OwnedJson::array(allocation)
                .map(bj::JsonOwner::Owned)
                .map_err(err);
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = allocation;
            return Err(err("owned变化JSON入口未启用"));
        }
    }
    Ok(bj::JsonOwner::Plain(Value::Array(Vec::new())))
}
fn push_change(array: &mut bj::JsonOwner, view: aj::JsonView<'_>) -> Result<(), String> {
    match array {
        bj::JsonOwner::Plain(Value::Array(rows)) => {
            rows.push(serde_json::to_value(view).map_err(err)?);
            Ok(())
        }
        bj::JsonOwner::Plain(_) => Err(err("变化JSON不是数组")),
        #[cfg(feature = "s_allocation_json_testonly")]
        bj::JsonOwner::Owned(rows) => rows.push(view).map_err(err),
    }
}
fn push_withdrawal(
    array: &mut bj::JsonOwner,
    e: &Entry,
    reason: &str,
    superseded_by: Option<&str>,
) -> Result<(), String> {
    use aj::JsonView as J;
    let sup = superseded_by.map(J::String).unwrap_or(J::Null);
    match &e.slot {
        Slot::Shape(w) => {
            let numbers = w.map(serde_json::Number::from);
            let fields = [
                ("object_id", J::String(e.id.as_str())),
                ("window_start", J::Number(&numbers[0])),
                ("window_mid", J::Number(&numbers[1])),
                ("window_end", J::Number(&numbers[2])),
                ("reason", J::String(reason)),
                ("superseded_by", sup),
            ];
            push_change(array, J::object_fields(&fields).map_err(err)?)
        }
        Slot::Typed { fact_key, .. } => {
            let fields = [
                ("object_id", J::String(e.id.as_str())),
                ("fact_key", J::String(fact_key.as_str())),
                ("reason", J::String(reason)),
                ("superseded_by", sup),
            ];
            push_change(array, J::object_fields(&fields).map_err(err)?)
        }
    }
}
fn push_replace(array: &mut bj::JsonOwner, old_id: &str, new_id: &str) -> Result<(), String> {
    let fields = [
        ("new_object_id", aj::JsonView::String(new_id)),
        ("old_object_id", aj::JsonView::String(old_id)),
    ];
    push_change(array, aj::JsonView::object_fields(&fields).map_err(err)?)
}
/// 独立生命周期导出的本代变化集合，顺序沿上一代活动主序。
/// 旧接口仅消费同一核心的普通数组，Owned所有者不能擦除为普通Vec。
pub(super) fn changes_at(index: &[Entry], g: i64) -> Result<(Vec<Value>, Vec<Value>), String> {
    let (withdrawals, replaces) = changes_at_budgeted(index, g, None)?;
    let plain = |owner| match owner {
        bj::JsonOwner::Plain(Value::Array(rows)) => Ok(rows),
        _ => Err(err("变化JSON不是普通数组")),
    };
    Ok((plain(withdrawals)?, plain(replaces)?))
}
/// dead排序工作集和真实返回JSON所有者持续使用调用方实际OwnedBudget。
pub(super) fn changes_at_budgeted(
    index: &[Entry],
    g: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<(bj::JsonOwner, bj::JsonOwner), String> {
    let mut dead = bj::new_work(allocation);
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.withdrawn_generation == Some(g))
    {
        dead.try_push((ordinal, e)).map_err(err)?;
    }
    // 同一主序加原始序号，保留stable排序的平局行为；unstable排序不另分配std工作集。
    dead.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
    let mut withdrawals = change_array(allocation)?;
    let mut replaces = change_array(allocation)?;
    for (_, e) in dead {
        let reason = e
            .life
            .withdrawal_reason
            .as_deref()
            .ok_or_else(|| err("撤回缺原因"))?;
        push_withdrawal(&mut withdrawals, e, reason, e.life.superseded_by.as_deref())?;
        if let Some(id) = &e.life.superseded_by {
            push_replace(&mut replaces, e.id.as_str(), id.as_str())?;
        }
    }
    Ok((withdrawals, replaces))
}
/// 当前已核 Capture 内的纯派生生命周期事件；不跨 Capture 保存。
pub(super) struct HistorySweep {
    by_id: std::collections::HashMap<String, usize>,
    births: std::collections::BTreeMap<i64, Vec<usize>>,
    deaths: std::collections::BTreeMap<i64, Vec<usize>>,
    ranks: Vec<usize>,
    active: BTreeSet<(usize, usize)>,
    previous: i64,
    #[cfg(test)]
    visits: SweepVisits,
}
#[cfg(test)]
#[derive(Default)]
struct SweepVisits {
    initial_entries: usize,
    births: usize,
    deaths: usize,
    active: std::cell::Cell<usize>,
}
impl HistorySweep {
    fn new(index: &[Entry]) -> Self {
        let mut order: Vec<_> = (0..index.len()).collect();
        order.sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], false).then(a.cmp(b)));
        let mut ranks = vec![0; index.len()];
        for (rank, i) in order.iter().enumerate() {
            ranks[*i] = rank;
        }
        let mut out = Self {
            by_id: Default::default(),
            births: Default::default(),
            deaths: Default::default(),
            ranks,
            active: Default::default(),
            previous: 0,
            #[cfg(test)]
            visits: SweepVisits::default(),
        };
        for (i, e) in index.iter().enumerate() {
            #[cfg(test)]
            {
                out.visits.initial_entries += 1;
            }
            out.by_id.insert(e.id.as_str().to_owned(), i);
            out.births
                .entry(e.life.first_known_generation)
                .or_default()
                .push(i);
        }
        for i in order {
            if let Some(g) = index[i].life.withdrawn_generation {
                out.deaths.entry(g).or_default().push(i);
            }
        }
        out
    }
    fn active_ordinals(&self) -> impl Iterator<Item = usize> + '_ {
        self.active.iter().map(|(_, i)| {
            #[cfg(test)]
            self.visits.active.set(self.visits.active.get() + 1);
            *i
        })
    }
    pub(super) fn advance(&mut self, g: i64) -> Result<(), String> {
        if g <= self.previous {
            return Err(err("生命周期 sweep 代号不递增"));
        }
        for (_, ids) in self.births.range(self.previous..g) {
            for i in ids {
                self.active.insert((self.ranks[*i], *i));
                #[cfg(test)]
                {
                    self.visits.births += 1;
                }
            }
        }
        for (_, ids) in self.deaths.range(self.previous..g) {
            for i in ids {
                self.active.remove(&(self.ranks[*i], *i));
                #[cfg(test)]
                {
                    self.visits.deaths += 1;
                }
            }
        }
        self.previous = g;
        Ok(())
    }
    pub(super) fn changes(
        &self,
        index: &[Entry],
        g: i64,
    ) -> Result<(bj::JsonOwner, bj::JsonOwner), String> {
        let mut withdrawals = change_array(None)?;
        let mut replaces = change_array(None)?;
        for i in self.deaths.get(&g).into_iter().flatten() {
            let e = &index[*i];
            let reason = e
                .life
                .withdrawal_reason
                .as_deref()
                .ok_or_else(|| err("撤回缺原因"))?;
            push_withdrawal(&mut withdrawals, e, reason, e.life.superseded_by.as_deref())?;
            if let Some(id) = &e.life.superseded_by {
                push_replace(&mut replaces, e.id.as_str(), id.as_str())?;
            }
        }
        Ok((withdrawals, replaces))
    }
}
/// 按该代真实候选序复用同一diff判据，再对拍独立生命周期；不假定候选槽位唯一。
pub(super) fn verify_generated_members<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
) -> Result<(), String> {
    verify_generated_members_swept(s, index, state, g, None)
}
pub(super) fn verify_generated_members_swept<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
    sweep: Option<&HistorySweep>,
) -> Result<(), String> {
    let state = state.into();
    let allocation = s.allocation_budget();
    let mut by_id = bj::new_map(allocation.as_ref());
    if sweep.is_none() {
        for e in index.iter().filter(|e| e.life.first_known_generation <= g) {
            by_id.insert(e.id.as_str(), e).map_err(err)?;
        }
    }
    let objects = field_ref(s, state, "objects")?;
    if objects.kind() != Kind::Array {
        return Err(err("生成对象集不是数组"));
    }
    let mut seen = bj::new_map(allocation.as_ref());
    let mut new = bj::new_work(allocation.as_ref());
    let items = if s.audit_full_array_scan() {
        Some(s.lookup_array_ref(objects.view()).map_err(err)?)
    } else {
        None
    };
    for n in 0..objects.extent().ok_or_else(|| err("生成集缺长度"))? {
        let object = if let Some(items) = &items {
            items[n as usize].clone()
        } else {
            s.lookup_ref(objects.view(), &[PathStep::Index(n)])
                .map_err(err)?
        };
        let id = text_ref(s, object.view(), "object_id")?;
        if seen.insert(id.try_clone()?, ()).map_err(err)?.is_some() {
            return Err(err("生成集重复对象身份"));
        }
        let e = match sweep {
            Some(sweep) => sweep
                .by_id
                .get(id.as_str())
                .map(|i| &index[*i])
                .filter(|e| e.life.first_known_generation <= g),
            None => by_id.get(id.as_str()).copied(),
        }
        .ok_or_else(|| err("生成对象不在独立身份史内"))?;
        let fields = OwnerFields::read(
            s,
            object.view(),
            if e.slot.is_shape() {
                &object_records::SHAPE_SEALED_FIELDS
            } else {
                &object_records::TYPED_SEALED_FIELDS
            },
        )?;
        if object.kind() != Kind::Object
            || object.extent() != Some(if e.slot.is_shape() { 15 } else { 10 })
            || fields.integer(s, "first_known_generation")? != g
            || fields.integer(s, "published_generation")? != g
            || fields.text(s, "first_known_cut")? != format!("cut-{g}")
        {
            return Err(err("生成对象字段/当代元数据不符"));
        }
        if fields.immutable(s, e.slot.is_shape())? != e.immutable {
            return Err(err("生成集同身份内容与独立原件不同"));
        }
        if e.life.first_known_generation == g && object != built_ref(s, &e.generated) {
            return Err(err("出生原件不等于真实首次生成记录"));
        }
        new.try_push(object_diff::Key {
            object_id: id,
            slot: &e.slot,
            immutable: object_diff::Ready(e.immutable.digest),
        })
        .map_err(err)?;
    }
    let mut before = bj::new_work(allocation.as_ref());
    for (ordinal, e) in index
        .iter()
        .take(if sweep.is_none() { index.len() } else { 0 })
        .enumerate()
        .filter(|(_, e)| e.life.first_known_generation < g)
    {
        before.try_push((ordinal, e)).map_err(err)?;
    }
    before.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
    let mut old = bj::new_work(allocation.as_ref());
    for (_, e) in &before {
        old.try_push(object_diff::Existing {
            key: object_diff::Key {
                object_id: e.id.as_str(),
                slot: &e.slot,
                immutable: object_diff::Ready(e.immutable.digest),
            },
            active: e.life.withdrawn_generation.is_none_or(|d| d >= g),
        })
        .map_err(err)?;
    }
    let changes = if let Some(sweep) = sweep {
        object_diff::plan_indexed_budgeted(
            &new,
            |i| object_diff::Existing {
                key: object_diff::Key {
                    object_id: index[i].id.as_str(),
                    slot: &index[i].slot,
                    immutable: object_diff::Ready(index[i].immutable.digest),
                },
                active: false,
            },
            sweep.active_ordinals(),
            |id| {
                sweep
                    .by_id
                    .get(id)
                    .copied()
                    .filter(|i| index[*i].life.first_known_generation < g)
            },
            |id| {
                sweep.by_id.get(id).copied().filter(|i| {
                    index[*i].life.first_known_generation < g
                        && index[*i].life.withdrawn_generation.is_none_or(|d| d >= g)
                })
            },
            None,
        )?
    } else {
        object_diff::plan_budgeted(&new, &old, allocation.as_ref())?
    };
    if !changes
        .births
        .iter()
        .map(|i| new[*i].object_id.as_str())
        .eq(match sweep {
            Some(sweep) => Box::new(
                sweep
                    .births
                    .get(&g)
                    .into_iter()
                    .flatten()
                    .map(|i| index[*i].id.as_str()),
            ) as Box<dyn Iterator<Item = &str>>,
            None => Box::new(
                index
                    .iter()
                    .filter(|e| e.life.first_known_generation == g)
                    .map(|e| e.id.as_str()),
            ),
        })
    {
        return Err(err("真实出生顺序与独立索引不同"));
    }
    let (expected_withdrawals, expected_replaces) = match sweep {
        Some(sweep) => sweep.changes(index, g)?,
        None => changes_at_budgeted(index, g, allocation.as_ref())?,
    };
    let mut withdrawals = change_array(allocation.as_ref())?;
    for w in changes.withdrawals {
        let e = if sweep.is_some() {
            &index[w.old_index]
        } else {
            before[w.old_index].1
        };
        let (reason, sup) = match w.removal {
            object_diff::Removal::Removed => (
                if e.slot.is_shape() {
                    "window_removed"
                } else {
                    "fact_removed"
                },
                None,
            ),
            object_diff::Removal::Superseded => (
                "superseded_by_revision",
                Some(
                    new[w.new_index.ok_or_else(|| err("替代缺候选"))?]
                        .object_id
                        .as_str(),
                ),
            ),
        };
        push_withdrawal(&mut withdrawals, e, reason, sup)?;
    }
    let mut replaces = change_array(allocation.as_ref())?;
    for r in changes.replaces {
        push_replace(
            &mut replaces,
            if sweep.is_some() {
                index[r.old_index].id.as_str()
            } else {
                before[r.old_index].1.id.as_str()
            },
            new[r.new_index].object_id.as_str(),
        )?;
    }
    if withdrawals.view() != expected_withdrawals.view()
        || replaces.view() != expected_replaces.view()
    {
        return Err(err("真实撤回/替代与独立索引不同"));
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static WIRE_PROJECTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_get_opt, meta_set, open_db, read_effective_events, read_raw_events, tb02a,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use newchan_rust::s_content::NodeSink;
    use rusqlite::Connection;
    thread_local! {
        pub(super) static SHAPE_LEAF_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn leaf_checks<T>(action: impl FnOnce() -> T) -> (T, usize) {
        SHAPE_LEAF_CHECKS.with(|checks| checks.set(0));
        let result = action();
        (result, SHAPE_LEAF_CHECKS.with(std::cell::Cell::get))
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn shape_leaf_fixture(tx: &Transaction<'_>) -> (sc::MemoryNodes, sc::CodecVec<Entry>) {
        let (mut nodes, objects) = native_candidate_fixture();
        let p = plan(&mut nodes, &objects, &[], 1).unwrap();
        stage(tx, &mut nodes, &p, &format!("batch3-{}", "1".repeat(64))).unwrap();
        let entries = load(tx, &mut nodes, 1).unwrap();
        (nodes, entries)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn shape_leaf_pairs(
        nodes: &mut sc::MemoryNodes,
        entries: &[Entry],
    ) -> Vec<(String, usize, &'static str, u64, ContentRef, ContentRef)> {
        let mut pairs = Vec::new();
        for e in entries.iter().filter(|e| e.slot.is_shape()) {
            let a = field(nodes, &e.internal.content_ref(), "input_refs").unwrap();
            let b = field_ref(nodes, e.birth_wire.view(), "input_refs").unwrap();
            for n in 0..3 {
                for key in ["raw_refs", "dependency_refs"] {
                    let path = [PathStep::Index(n), f(key)];
                    let aa = nodes.lookup_ref((&a).into(), &path).unwrap();
                    let bb = nodes.lookup_ref(b.view(), &path).unwrap();
                    for k in 0..aa.extent().unwrap() {
                        let ar = nodes.lookup_ref(aa.view(), &[PathStep::Index(k)]).unwrap();
                        let br = nodes.lookup_ref(bb.view(), &[PathStep::Index(k)]).unwrap();
                        let plain = |r: &RefValue| ContentRef {
                            format: r.view().format().to_owned(),
                            root: r.view().root().to_owned(),
                            kind: r.kind(),
                            extent: r.extent(),
                        };
                        pairs.push((
                            e.id.as_str().to_owned(),
                            n as usize,
                            key,
                            k,
                            plain(&ar),
                            plain(&br),
                        ));
                    }
                }
            }
        }
        pairs
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_shape_leaf_real_load_reuses_across_distinct_births_and_excludes_gates() {
        let mut c = database();
        let tx = c.transaction().unwrap();
        let (mut nodes, entries) = shape_leaf_fixture(&tx);
        let pairs = shape_leaf_pairs(&mut nodes, &entries);
        let unique = pairs
            .iter()
            .map(|(_, _, _, _, a, b)| (a.root.clone(), b.root.clone()))
            .collect::<HashSet<_>>();
        assert!(!unique.is_empty());
        assert!(
            pairs.iter().any(|(id, _, _, _, a, b)| pairs
                .iter()
                .any(|(other, _, _, _, aa, bb)| id != other && a == aa && b == bb)),
            "必须来自不同真实出生 Entry 的共享非空来源叶"
        );
        let meter = RecordMeter::default();
        let capture = sc::AuditCapture::new(nodes.clone());
        let ordinary = || super::super::history::HashReader::captured(&capture, None);
        let mut plain = ordinary();
        let (expected, ordinary_checks) =
            leaf_checks(|| load_for_audit(&tx, &mut plain, 1).unwrap());
        assert_eq!(ordinary_checks, pairs.len());
        let mut native = ordinary();
        native.enable_full_arrays_for_test();
        let (actual, native_checks) =
            leaf_checks(|| load_for_history(&tx, &mut native, 1).unwrap());
        assert_eq!(format!("{:?}", actual.entries()), format!("{expected:?}"));
        assert_eq!(native_checks, unique.len());
        assert!(native_checks < ordinary_checks);
        native.clear_overlay();
        let (_, rebuilt_checks) = leaf_checks(|| load_for_audit(&tx, &mut native, 1).unwrap());
        assert_eq!(
            rebuilt_checks,
            unique.len(),
            "每次 load 重新建缓存，clear 后不继承旧结果"
        );
        let other_capture = sc::AuditCapture::new(nodes.clone());
        let mut other = super::super::history::HashReader::captured(&other_capture, None);
        other.enable_full_arrays_for_test();
        let (_, other_checks) = leaf_checks(|| load_for_audit(&tx, &mut other, 1).unwrap());
        assert_eq!(other_checks, unique.len(), "异 capture 独立重新核验");
        let mut bounded = ordinary().with_audit_gate(Some(&meter), None);
        bounded.enable_full_arrays_for_test();
        let (_, checks) = leaf_checks(|| load_for_audit(&tx, &mut bounded, 1).unwrap());
        assert_eq!(checks, pairs.len());
        let mut deadline = ordinary().with_native_publication_deadline_for_test(
            std::time::Instant::now() + std::time::Duration::from_secs(60),
        );
        deadline.enable_full_arrays_for_test();
        let (_, checks) = leaf_checks(|| load_for_audit(&tx, &mut deadline, 1).unwrap());
        assert_eq!(checks, pairs.len());
        let budget = OwnedBudget::new(100_000_000);
        let mut slab = sc::capture_allocation::CaptureSlab::new(
            nodes.iter().count(),
            nodes.iter().map(|(_, _, b)| b.len()).sum(),
            &budget,
        )
        .unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let mut owned_reader = super::super::history::HashReader::captured(&owned, None);
        owned_reader.enable_full_arrays_for_test();
        let (_, checks) = leaf_checks(|| load_for_audit(&tx, &mut owned_reader, 1).unwrap());
        assert_eq!(checks, pairs.len());
        println!("Native shape leaf real load: checks ordinary={ordinary_checks} native={native_checks}; distinct birth entries share source leaves");
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_shape_leaf_real_load_rejects_changed_projection_and_shared_entry_damage() {
        let mut c = database();
        let tx = c.transaction().unwrap();
        let (mut nodes, entries) = shape_leaf_fixture(&tx);
        let pairs = shape_leaf_pairs(&mut nodes, &entries);
        let (id, n, key, k, _, _) = pairs
            .iter()
            .rev()
            .find(|(id, _, _, _, a, b)| {
                pairs
                    .iter()
                    .any(|(other, _, _, _, aa, bb)| id != other && a == aa && b == bb)
            })
            .unwrap();
        let e = entries.iter().find(|e| e.id.as_str() == id).unwrap();
        for field_name in [
            "identity_key",
            "payload_hash",
            "receipt_id",
            "event_id",
            "input_revision",
            "source_coord",
            "revision",
            "seq",
        ] {
            let mut damaged = nodes.clone();
            let mut wire = value_ref(&mut damaged, e.birth_wire.view());
            wire["input_refs"][*n][*key][*k as usize][field_name] =
                json!("invalid-public-projection");
            let bad_wire = sc::write_json_view(&mut damaged, aj::JsonView::Plain(&wire)).unwrap();
            tx.execute_batch("SAVEPOINT shape_leaf_fault").unwrap();
            tx.execute(
                "UPDATE s_content_objects SET birth_wire_root=?1 WHERE object_id=?2",
                params![bad_wire.digest.as_slice(), id],
            )
            .unwrap();
            let capture = sc::AuditCapture::new(damaged);
            let mut native = super::super::history::HashReader::captured(&capture, None);
            native.enable_full_arrays_for_test();
            let error = load_for_audit(&tx, &mut native, 1).unwrap_err();
            assert!(
                error.contains(if matches!(field_name, "revision" | "seq") {
                    "来源数字公开投影不符"
                } else {
                    "内容边不同"
                }),
                "{field_name}: {error}"
            );
            tx.execute_batch("ROLLBACK TO shape_leaf_fault; RELEASE shape_leaf_fault")
                .unwrap();
            native.clear_overlay();
            let good_capture = sc::AuditCapture::new(nodes.clone());
            let mut good = super::super::history::HashReader::captured(&good_capture, None);
            good.enable_full_arrays_for_test();
            let (_, checks) = leaf_checks(|| load_for_audit(&tx, &mut good, 1).unwrap());
            assert!(checks > 0, "错误退出后，新load重新核判据");
        }
        for sql in [
            "UPDATE s_content_objects SET object_revision=object_revision+1 WHERE object_id=?1",
            "UPDATE s_content_objects SET window_end=window_end+1 WHERE object_id=?1",
            "UPDATE s_content_objects SET first_known_cut='bad-cut' WHERE object_id=?1",
            "UPDATE s_content_objects SET published_generation=2 WHERE object_id=?1",
            "UPDATE s_content_objects SET immutable_root=generated_root WHERE object_id=?1",
        ] {
            tx.execute_batch("SAVEPOINT shape_entry_fault").unwrap();
            tx.execute(sql, params![id]).unwrap();
            let capture = sc::AuditCapture::new(nodes.clone());
            let mut native = super::super::history::HashReader::captured(&capture, None);
            native.enable_full_arrays_for_test();
            assert!(
                load_for_audit(&tx, &mut native, 1).is_err(),
                "共享叶不得放行坏Entry: {sql}"
            );
            tx.execute_batch("ROLLBACK TO shape_entry_fault; RELEASE shape_entry_fault")
                .unwrap();
        }
        let mut damaged = nodes.clone();
        let (_, tag, _) = nodes
            .iter()
            .find(|(d, _, _)| **d == e.generated.digest)
            .unwrap();
        damaged.insert_unchecked(e.generated.digest, tag, vec![0]);
        let capture = sc::AuditCapture::new(damaged);
        let mut native = super::super::history::HashReader::captured(&capture, None);
        native.enable_full_arrays_for_test();
        assert!(load_for_history(&tx, &mut native, 1)
            .err()
            .unwrap()
            .contains(sc::code::NODE_DIGEST));
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_shape_leaf_complete_declarations_error_clear_and_capture_owned_closure() {
        let mut c = database();
        let tx = c.transaction().unwrap();
        let (mut nodes, entries) = shape_leaf_fixture(&tx);
        let pairs = shape_leaf_pairs(&mut nodes, &entries);
        let (_, _, _, _, ar, br) = &pairs[0];
        let capture = sc::AuditCapture::new(nodes.clone());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        reader.enable_full_arrays_for_test();
        let mut bindings = ShapeLeafBindings {
            capture: &capture,
            successful: HashSet::new(),
        };
        bindings.bind(&mut reader, ar.into(), br.into()).unwrap();
        assert_eq!(bindings.successful.len(), 1);
        for field in ["format", "root", "kind", "extent"] {
            let mut bad = ar.clone();
            match field {
                "format" => bad.format = "bad-format".into(),
                "root" => bad.root = "bad-root".into(),
                "kind" => bad.kind = Kind::Array,
                "extent" => bad.extent = Some(7),
                _ => unreachable!(),
            }
            assert!(bindings
                .bind(&mut reader, (&bad).into(), br.into())
                .is_err());
            assert!(bindings.successful.is_empty(), "{field}错误清空已成功关系");
            let (_, checks) =
                leaf_checks(|| bindings.bind(&mut reader, ar.into(), br.into()).unwrap());
            assert_eq!(checks, 1, "错误不入缓存，合法关系须重建");
        }
        // overlay-only端点仍按原判据合法绑定，每次clear→重建后也不能进入memo。
        let leaf = ar.digest().unwrap();
        let (_, tag, body) = nodes.iter().find(|(d, _, _)| **d == leaf).unwrap();
        let mut overlay_only = sc::MemoryNodes::new();
        for (d, t, b) in nodes.iter().filter(|(d, _, _)| **d != leaf) {
            overlay_only.insert_unchecked(*d, t, b.to_vec());
        }
        let overlay_capture = sc::AuditCapture::new(overlay_only);
        let mut native = super::super::history::HashReader::captured(&overlay_capture, None);
        native.enable_full_arrays_for_test();
        let mut overlay_bindings = ShapeLeafBindings {
            capture: &overlay_capture,
            successful: HashSet::new(),
        };
        for _ in 0..2 {
            native.put(leaf, tag, body).unwrap();
            for _ in 0..2 {
                let (_, checks) = leaf_checks(|| {
                    overlay_bindings
                        .bind(&mut native, ar.into(), br.into())
                        .unwrap()
                });
                assert_eq!(checks, 1);
                assert!(overlay_bindings.successful.is_empty());
            }
            // 完整load的既有capture闭包门仍拒绝缺失原库节点，不由overlay补证。
            let error = load_for_audit(&tx, &mut native, 1).unwrap_err();
            assert!(error.contains(sc::code::NODE_MISSING), "{error}");
            native.clear_overlay();
            assert!(load_for_audit(&tx, &mut native, 1).is_err());
        }
        // 自有root缺深叶或原件损坏，overlay补叶仍不能修补capture自身的Public闭包。
        let child = field(&mut nodes, ar, "identity_key")
            .unwrap()
            .digest()
            .unwrap();
        for corrupt in [false, true] {
            let mut damaged = sc::MemoryNodes::new();
            for (d, t, b) in nodes.iter() {
                if *d == if corrupt { leaf } else { child } {
                    if corrupt {
                        damaged.insert_unchecked(*d, t, vec![0]);
                    }
                } else {
                    damaged.insert_unchecked(*d, t, b.to_vec());
                }
            }
            let damaged_capture = sc::AuditCapture::new(damaged);
            let mut native = super::super::history::HashReader::captured(&damaged_capture, None);
            native.enable_full_arrays_for_test();
            if !corrupt {
                let (_, tag, body) = nodes.iter().find(|(d, _, _)| **d == child).unwrap();
                native.put(child, tag, body).unwrap();
            }
            let error = load_for_audit(&tx, &mut native, 1).unwrap_err();
            assert!(
                error.contains(if corrupt {
                    sc::code::NODE_DIGEST
                } else {
                    sc::code::NODE_MISSING
                }),
                "{error}"
            );
            native.clear_overlay();
            assert!(load_for_audit(&tx, &mut native, 1).is_err());
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn candidate_plan_signature(p: &Plan) -> Value {
        json!({"generation":p.generation,"index":format!("{:?}",p.index),"births":format!("{:?}",p.births),
            "withdrawals":p.withdrawals,"replaces":p.replaces,
            "active":RefValue::from(p.active.content_ref()),"withdrawn":RefValue::from(p.withdrawn.content_ref()),"upserts":RefValue::from(p.upserts.content_ref())})
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn candidate_union_old_fields_for_test(
        s: &mut (impl CodecStore + ?Sized),
        objects: &[ProducedObject],
        old: &[Entry],
        g: i64,
    ) -> Plan {
        s.verify_audit_many_fresh(
            objects.iter().map(|o| o.generated().content_ref()),
            sc::Mode::Public,
        )
        .unwrap();
        let entries = objects
            .iter()
            .map(|o| candidate_verified(s, o, g))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        plan_from_candidates(s, entries, old, g).unwrap()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn candidate_sql_material_for_test(tx: &Transaction<'_>) -> Vec<(Vec<u8>, i64, Vec<u8>)> {
        tx.prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn native_candidate_fixture() -> (sc::MemoryNodes, Vec<ProducedObject>) {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()[..8]));
        let source = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&source).unwrap();
        let effective = read_effective_events(&raw).unwrap();
        let get = |k| meta_get_opt(&source, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        let rule = get("rule_revision");
        let catalog = get("catalog_revision");
        let scope = serde_json::from_str(&get("scope")).unwrap();
        let input = GenerationInput {
            all_events: (&raw).into(),
            effective: (&effective).into(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: raw.len() as i64 - 1,
            profile_id: tb02a::PROFILE,
            profile_hash: &profile_hash,
            rule_revision: &rule,
            catalog_revision: &catalog,
            scope: &scope,
            semantic_commit_ns: None,
            prior: PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        };
        let mut nodes = sc::MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            format!("candidate-union/{}", files.db().display()),
            tb02a::PROFILE,
            &rule,
        );
        let generated = runtime.generate(&mut nodes, &input).unwrap();
        (nodes, generated.objects)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[derive(Default)]
    struct CandidateProbe {
        nodes: sc::MemoryNodes,
        puts: usize,
        reads: usize,
        singles: usize,
        unions: usize,
        budget: Option<OwnedBudget>,
        second_declaration: Option<&'static str>,
        second_reference: Option<ContentRef>,
        future_put_restore: Option<(NodeDigest, u8, Vec<u8>)>,
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl NodeSource for CandidateProbe {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.reads += 1;
            self.nodes.read(d)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl NodeSink for CandidateProbe {
        fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.puts += 1;
            if let Some((digest, tag, body)) = self.future_put_restore.take() {
                self.nodes.put(digest, tag, &body)?;
            }
            self.nodes.put(d, t, b)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl CodecRead for CandidateProbe {
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            self
        }
        fn allocation_budget(&self) -> Option<OwnedBudget> {
            self.budget.clone()
        }
        fn verify_audit_fresh(
            &mut self,
            r: &ContentRef,
            mode: sc::Mode,
        ) -> Result<(), sc::CodecError> {
            self.singles += 1;
            sc::audit_verify(self, r, mode)
        }
        fn verify_audit_many_fresh(
            &mut self,
            roots: impl IntoIterator<Item = ContentRef>,
            mode: sc::Mode,
        ) -> Result<(), sc::CodecError> {
            self.unions += 1;
            let declaration = self.second_declaration;
            let replacement = self.second_reference.clone();
            sc::audit_verify_many(
                self,
                roots.into_iter().enumerate().map(move |(i, mut r)| {
                    if i == 1 {
                        if let Some(replacement) = &replacement {
                            r = replacement.clone();
                        }
                        match declaration {
                            Some("kind") => r.kind = Kind::Bool,
                            Some("extent") => r.extent = Some(r.extent.unwrap() + 1),
                            Some("format") => r.format = "bad-format".into(),
                            _ => {}
                        }
                    }
                    r
                }),
                mode,
            )
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl CodecStore for CandidateProbe {
        fn generic_store(&mut self) -> &mut dyn NodeStore {
            self
        }
    }

    // 组件级lookup注入：真实合法F的另一份generated视图；不声称完整S坏输入端到端。
    #[cfg(feature = "s_capture_native_testonly")]
    struct CandidateFieldsProbe {
        inner: CandidateProbe,
        generated_root: NodeDigest,
        replacement: Option<ContentRef>,
        field_reads: usize,
        read_digests: Vec<NodeDigest>,
        put_digests: Vec<NodeDigest>,
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl CandidateFieldsProbe {
        fn current(&self, r: &ContentRef) -> ContentRef {
            if r.digest().unwrap() == self.generated_root {
                self.replacement.as_ref().unwrap_or(r).clone()
            } else {
                r.clone()
            }
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl NodeSource for CandidateFieldsProbe {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.read_digests.push(*d);
            self.inner.read(d)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl NodeSink for CandidateFieldsProbe {
        fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.put_digests.push(d);
            self.inner.put(d, t, b)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl CodecRead for CandidateFieldsProbe {
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            self
        }
        fn lookup_path(
            &mut self,
            r: &ContentRef,
            p: &[PathStep],
        ) -> Result<ContentRef, sc::CodecError> {
            let current = self.current(r);
            sc::path::lookup_path(self, &current, p)
        }
        fn lookup_object_fields(
            &mut self,
            r: &ContentRef,
            names: &[&str],
        ) -> Result<sc::CodecFields, sc::CodecError> {
            self.field_reads += 1;
            let current = self.current(r);
            sc::path::lookup_object_fields(self, &current, names).map(sc::CodecFields::from_plain)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl CodecStore for CandidateFieldsProbe {
        fn generic_store(&mut self) -> &mut dyn NodeStore {
            self
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn candidate_fields_probe_for_test(
        nodes: sc::MemoryNodes,
        root: NodeDigest,
        replacement: Option<ContentRef>,
    ) -> CandidateFieldsProbe {
        CandidateFieldsProbe {
            inner: CandidateProbe {
                nodes,
                ..Default::default()
            },
            generated_root: root,
            replacement,
            field_reads: 0,
            read_digests: Vec::new(),
            put_digests: Vec::new(),
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_candidate_fields_component_exact_metadata_slot_and_late_null_gates() {
        let (nodes, objects) = native_candidate_fixture();
        let typed = objects
            .iter()
            .find(|o| matches!(o.borrowed(), BorrowedObject::Typed(_)))
            .unwrap();
        let shape = objects
            .iter()
            .find(|o| matches!(o.borrowed(), BorrowedObject::Shape(_)))
            .unwrap();
        for o in [typed, shape] {
            let mut baseline = CandidateProbe {
                nodes: nodes.clone(),
                ..Default::default()
            };
            baseline
                .verify_audit_fresh(&o.generated().content_ref(), sc::Mode::Public)
                .unwrap();
            let old = candidate_verified(&mut baseline, o, 1).unwrap();
            let mut current =
                candidate_fields_probe_for_test(nodes.clone(), o.generated().digest, None);
            current
                .verify_audit_fresh(&o.generated().content_ref(), sc::Mode::Public)
                .unwrap();
            let new = candidate_fields(&mut current, o, 1).unwrap();
            assert_eq!(format!("{new:?}"), format!("{old:?}"));
            assert_eq!(current.field_reads, 1, "exact generated一次绑定");
            assert!(
                current.read_digests.contains(&new.internal.digest),
                "wire必须实际读不同internal根"
            );
            assert_ne!(new.generated.digest, new.internal.digest);
            let wire = current
                .put_digests
                .iter()
                .position(|d| *d == new.birth_wire.digest().unwrap())
                .unwrap();
            let immutable = current
                .put_digests
                .iter()
                .position(|d| *d == new.immutable.digest)
                .unwrap();
            assert!(wire < immutable, "正常wire→immutable顺序不变");
        }
        for fault in [
            "missing",
            "extra",
            "kind",
            "kind-type",
            "id",
            "generation-type",
            "revision-type",
            "slot-type",
            "immutable-null",
        ] {
            let mut changed = nodes.clone();
            let mut original = value(&mut changed, &typed.generated().content_ref());
            match fault {
                "missing" => {
                    original.as_object_mut().unwrap().remove("object_id");
                }
                "extra" => original["extra_field"] = json!(true),
                "kind" => original["kind"] = json!("foreign-kind"),
                "kind-type" => original["kind"] = json!(7),
                "id" => original["object_id"] = json!("foreign-id"),
                "generation-type" => original["first_known_generation"] = json!("not-integer"),
                "revision-type" => original["object_revision"] = json!("not-integer"),
                "slot-type" => original["fact_key"] = json!(17),
                "immutable-null" => original["payload_json"] = Value::Null,
                _ => unreachable!(),
            }
            // 所有边按原件ContentRef保存；只替换目标小字段，不把JsonText边改写为普通String。
            let fields = sc::path::lookup_object_fields(
                &mut changed,
                &typed.generated().content_ref(),
                &object_records::TYPED_SEALED_FIELDS,
            )
            .unwrap();
            let mut edited = Vec::new();
            for (name, child) in fields {
                if fault == "missing" && name == "object_id" {
                    continue;
                }
                let replace = match fault {
                    "kind" | "kind-type" => name == "kind",
                    "id" => name == "object_id",
                    "generation-type" => name == "first_known_generation",
                    "revision-type" => name == "object_revision",
                    "slot-type" => name == "fact_key",
                    "immutable-null" => name == "payload_json",
                    _ => false,
                };
                let root = if replace {
                    sc::write_json_view(&mut changed, aj::JsonView::Plain(&original[&name]))
                        .unwrap()
                        .digest
                } else {
                    child.digest().unwrap()
                };
                edited.push((name, V::Edge(root)));
            }
            if fault == "extra" {
                edited.push(("extra_field".into(), V::Bool(true)));
            }
            let replacement = sc::write_value(&mut changed, &V::Object(edited))
                .unwrap()
                .content_ref();
            sc::audit_verify_public(&mut changed, &replacement).unwrap();
            let mut store = candidate_fields_probe_for_test(
                changed,
                typed.generated().digest,
                Some(replacement),
            );
            let error = candidate_fields(&mut store, typed, 1).err().unwrap();
            if fault == "immutable-null" {
                assert!(error.contains("不可变字段不得为空"), "{error}");
                assert!(store.inner.puts > 0, "Null仍在wire写入之后拒绝");
            } else {
                assert_eq!(
                    store.inner.puts, 0,
                    "{fault}: metadata/slot错误须在wire前拒绝：{error}"
                );
            }
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_candidate_fields_internal_record_json_edge_gate_and_real_wire_sql_failure() {
        let (mut nodes, objects) = native_candidate_fixture();
        let typed = objects
            .iter()
            .find_map(|o| match o.borrowed() {
                BorrowedObject::Typed(v) => Some(v.clone()),
                _ => None,
            })
            .unwrap();
        let alternate = sc::write_value(
            &mut nodes,
            &V::JsonText(Box::new(sc::JsonValue::Object(vec![(
                "changed".into(),
                sc::JsonValue::Bool(true),
            )]))),
        )
        .unwrap();
        let mut fields = sc::path::lookup_object_fields(
            &mut nodes,
            &typed.record.content_ref(),
            &object_records::TYPED_RECORD_FIELDS,
        )
        .unwrap();
        fields.insert("payload_json".into(), alternate.content_ref());
        let changed_record = sc::write_value(
            &mut nodes,
            &V::Object(
                fields
                    .into_iter()
                    .map(|(name, r)| (name, V::Edge(r.digest().unwrap())))
                    .collect(),
            ),
        )
        .unwrap();
        sc::audit_verify_public(&mut nodes, &changed_record.content_ref()).unwrap();
        let mut changed = typed.clone();
        changed.record = changed_record;
        let mut probe = CandidateProbe {
            nodes: nodes.clone(),
            ..Default::default()
        };
        let life = PublicationLife {
            first_known_generation: 1,
            first_known_cut: "cut-1".into(),
            published_generation: 1,
            withdrawn_generation: None,
            withdrawal_reason: None,
            superseded_by: None,
            batch: BatchRef::CurrentBatch,
        };
        let error =
            object_records::wire_template(&mut probe, BorrowedObject::Typed(&changed), &life, 1)
                .err()
                .unwrap()
                .to_string();
        assert!(
            error.contains("payload_json") && error.contains("边与构造结果"),
            "{error}"
        );
        assert_eq!(
            probe.puts, 0,
            "原internal record核对不能由generated视图替代"
        );

        let mut conn = database();
        let tx = conn.transaction().unwrap();
        let mut store = content_store::TxStore::new(&tx).unwrap();
        // Public generated不依赖08；省去可能的未引用模板原件，实际wire首写08触发SQL失败。
        for (d, t, b) in nodes.iter() {
            if t != sc::tag::BATCH_SELF {
                store.put(*d, t, b).unwrap();
            }
        }
        let before = candidate_sql_material_for_test(&tx);
        tx.execute_batch("CREATE TEMP TRIGGER candidate_fields_wire_sql_fault BEFORE INSERT ON s_content_nodes WHEN NEW.tag=8 BEGIN SELECT RAISE(ABORT,'candidate_fields_wire_SQL_fault'); END;").unwrap();
        let error = plan_native_union_for_testonly(&mut store, &objects, &[], 1)
            .err()
            .unwrap();
        assert!(error.contains("candidate_fields_wire_SQL_fault"), "{error}");
        assert_eq!(
            candidate_sql_material_for_test(&tx),
            before,
            "真实wire SQL失败不吞错或预写immutable"
        );
        tx.rollback().unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_candidate_union_second_root_errors_have_no_put_or_future_wire_repair_and_budget_refuses(
    ) {
        let (nodes, objects) = native_candidate_fixture();
        let objects = &objects[..2];
        let first = objects[0].generated().content_ref();
        let second = objects[1].generated().content_ref();
        assert_ne!(first.root, second.root);
        let leaf = sc::frame(sc::tag::BYTES_CHUNK, objects[1].object_id().as_bytes());
        assert!(nodes.iter().any(|(d, _, _)| *d == leaf));
        for (fault, code) in [
            ("kind", sc::code::KIND),
            ("extent", sc::code::EXTENT),
            ("format", sc::code::FORMAT),
            ("seen-root-kind", sc::code::KIND),
            ("missing-child", sc::code::NODE_MISSING),
            ("damaged-root", sc::code::NODE_DIGEST),
            ("public-role", sc::code::TEMPLATE_PLACEHOLDER),
        ] {
            let mut store = CandidateProbe {
                nodes: nodes.clone(),
                ..Default::default()
            };
            match fault {
                "kind" | "extent" | "format" => store.second_declaration = Some(fault),
                "seen-root-kind" => {
                    store.second_reference = Some(first.clone());
                    store.second_declaration = Some("kind");
                }
                "missing-child" => {
                    let mut missing = sc::MemoryNodes::new();
                    for (d, t, b) in nodes.iter() {
                        if *d != leaf {
                            missing.insert_unchecked(*d, t, b.to_vec());
                        }
                    }
                    store.nodes = missing;
                    store.future_put_restore = Some((
                        leaf,
                        sc::tag::BYTES_CHUNK,
                        objects[1].object_id().as_bytes().to_vec(),
                    ));
                }
                "damaged-root" => {
                    store
                        .nodes
                        .insert_unchecked(second.digest().unwrap(), sc::tag::OBJECT, vec![0])
                }
                "public-role" => {
                    store.second_reference = Some(
                        sc::write_template(&mut store.nodes, &V::BatchSelf)
                            .unwrap()
                            .content_ref(),
                    )
                }
                _ => unreachable!(),
            }
            sc::audit_verify_public(&mut store.nodes, &first).unwrap();
            let error = plan_native_union_for_testonly(&mut store, objects, &[], 1)
                .err()
                .unwrap();
            assert!(error.contains(code), "{fault}: {error}");
            assert_eq!(store.puts, 0, "{fault}: 联合失败不得触及candidate put");
            assert_eq!(store.unions, 1);
            assert_eq!(store.singles, 0);
            if fault == "missing-child" {
                assert!(
                    store.future_put_restore.is_some(),
                    "后来wire补子不得变成前置核验依据"
                );
            }
        }
        let mut ordinary = CandidateProbe {
            nodes: nodes.clone(),
            ..Default::default()
        };
        let expected = plan(&mut ordinary, objects, &[], 1).unwrap();
        assert_eq!(ordinary.singles, objects.len());
        assert_eq!(ordinary.unions, 0, "None预算本身不授予Native联合路径");
        let mut native = CandidateProbe {
            nodes: nodes.clone(),
            ..Default::default()
        };
        let actual = plan_native_union_for_testonly(&mut native, objects, &[], 1).unwrap();
        assert_eq!(
            candidate_plan_signature(&actual),
            candidate_plan_signature(&expected)
        );
        assert!(native.reads < ordinary.reads, "真实source读取须减少");
        println!("candidate two-root real source reads ordinary={} union={}; candidate puts ordinary={} union={}",ordinary.reads,native.reads,ordinary.puts,native.puts);
        let mut mixed = CandidateProbe {
            nodes,
            budget: Some(OwnedBudget::new(1_000_000)),
            ..Default::default()
        };
        assert!(plan_native_union_for_testonly(&mut mixed, objects, &[], 1)
            .err()
            .unwrap()
            .contains("拒绝allocation预算"));
        assert_eq!(
            (mixed.reads, mixed.puts, mixed.singles, mixed.unions),
            (0, 0, 0, 0)
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_candidate_union_real_generations_match_plan_and_reduce_sql_reads() {
        real_generations(false, true);
    }
    #[test]
    fn native_history_sweep_lifecycle_events_match_old_scans_and_bound_visits() {
        fn entry(i: usize, first: i64, death: Option<i64>) -> Entry {
            let built = Built {
                digest: [i as u8; 32],
                kind: Kind::Null,
                extent: None,
            };
            Entry {
                id: format!("id-{i}").into(),
                revision: 1,
                kind: SHAPE.into(),
                // 同主序、重复槽位仍按ordinal保留所有身份。
                slot: Slot::Shape([0, 1, 2]),
                generated: built.clone(),
                internal: built.clone(),
                immutable: built.clone(),
                birth_wire: built.content_ref().into(),
                life: LifeOwner::from_normal(PublicationLife {
                    first_known_generation: first,
                    first_known_cut: format!("cut-{first}"),
                    published_generation: first,
                    withdrawn_generation: death,
                    withdrawal_reason: death.map(|_| "window_removed".to_owned()),
                    superseded_by: None,
                    batch: BatchRef::CurrentBatch,
                }),
            }
        }
        let mut index = (0..100).map(|i| entry(i, 1, Some(2))).collect::<Vec<_>>();
        index.extend([
            entry(100, 1, None),
            entry(101, 3, Some(3)),
            entry(102, 5, None),
        ]);
        let mut sweep = HistorySweep::new(&index);
        let mut active_visits = 0;
        let mut birth_event_visits = 0;
        let mut death_event_visits = 0;
        for g in 1..=8 {
            birth_event_visits += sweep
                .births
                .range(sweep.previous..g)
                .map(|(_, v)| v.len())
                .sum::<usize>();
            death_event_visits += sweep
                .deaths
                .range(sweep.previous..g)
                .map(|(_, v)| v.len())
                .sum::<usize>();
            sweep.advance(g).unwrap();
            let mut expected = index
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.life.first_known_generation < g
                        && e.life.withdrawn_generation.is_none_or(|d| d >= g)
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            expected
                .sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], false).then(a.cmp(b)));
            let actual = sweep.active_ordinals().collect::<Vec<_>>();
            active_visits += actual.len();
            assert_eq!(actual, expected, "cut-{g}");
            let old = changes_at_budgeted(&index, g, None).unwrap();
            let new = sweep.changes(&index, g).unwrap();
            assert_eq!(old.0.view(), new.0.view());
            assert_eq!(old.1.view(), new.1.view());
            if g == 2 {
                assert_eq!(actual.len(), 101, "death==g仍参与本代");
            }
            if g == 3 {
                assert_eq!(actual, [100], "birth==g不在before");
            }
            if g == 4 {
                assert_eq!(actual, [100], "同代出生撤回不应残留");
            }
        }
        assert_eq!(birth_event_visits, index.len());
        assert_eq!(death_event_visits, 101);
        assert_eq!(active_visits, 110);
        assert_eq!(sweep.visits.initial_entries, index.len());
        assert_eq!(sweep.visits.births, birth_event_visits);
        assert_eq!(sweep.visits.deaths, death_event_visits);
        assert_eq!(sweep.visits.active.get(), active_visits);
        // 老五条全历史过滤每cut各看103项；sweep初始化一次，事件各过一次。
        assert_eq!(5 * index.len() * 8, 4120);
        assert!(index.len() + birth_event_visits + death_event_visits + active_visits < 4120);
        assert!(sweep.advance(8).is_err());
    }

    fn database() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        c.execute_batch(crate::SCHEMA).unwrap();
        c.execute_batch(crate::v2::SCHEMA_V2).unwrap();
        meta_set(&c, "protocol_revision", "s-session/3").unwrap();
        c.execute(
            "INSERT INTO s_protocol_meta VALUES(1,'s-session/3','1','test','{}',1,'0')",
            [],
        )
        .unwrap();
        let tx = c.transaction().unwrap();
        content_store::init_schema(&tx).unwrap();
        init_schema(&tx).unwrap();
        tx.commit().unwrap();
        c
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_changes_keep_complete_shape_stable_order_and_live_return_owner() {
        fn entry(id: &str, slot: Slot, sup: Option<&str>) -> Entry {
            let built = Built {
                digest: [0; 32],
                kind: Kind::Null,
                extent: None,
            };
            Entry {
                id: id.into(),
                revision: 1,
                kind: if slot.is_shape() {
                    SHAPE.into()
                } else {
                    "typed".into()
                },
                slot,
                generated: built.clone(),
                internal: built.clone(),
                immutable: built.clone(),
                birth_wire: built.content_ref().into(),
                life: LifeOwner::from_normal(PublicationLife {
                    first_known_generation: 1,
                    first_known_cut: "cut-1".into(),
                    published_generation: 1,
                    withdrawn_generation: Some(2),
                    withdrawal_reason: Some("superseded_by_revision".into()),
                    superseded_by: sup.map(str::to_owned),
                    batch: BatchRef::CurrentBatch,
                }),
            }
        }
        // 同起点shape保留插入序，typed按kind/fact/id/revision；未来撤回被排除。
        let mut index = vec![
            entry(
                "typed😀",
                Slot::Typed {
                    kind: "typed".into(),
                    fact_key: "facté".into(),
                },
                Some("new😀"),
            ),
            entry("shape-first", Slot::Shape([7, 8, 9]), None),
            entry("shape-second", Slot::Shape([7, 9, 10]), Some("new-shape")),
            entry("future", Slot::Shape([0, 1, 2]), None),
        ];
        index[3].life.withdrawn_generation = Some(3);
        let budget = OwnedBudget::new(1_000_000);
        let plain = changes_at(&index, 2).unwrap();
        {
            let owned = changes_at_budgeted(&index, 2, Some(&budget)).unwrap();
            assert_eq!(
                owned.0.view(),
                aj::JsonView::Plain(&Value::Array(plain.0.clone()))
            );
            assert_eq!(
                owned.1.view(),
                aj::JsonView::Plain(&Value::Array(plain.1.clone()))
            );
            let rows = owned.0.view().as_array().unwrap();
            assert_eq!(
                rows.get(0).unwrap().get("object_id").unwrap().as_str(),
                Some("shape-first")
            );
            assert_eq!(
                rows.get(1).unwrap().get("object_id").unwrap().as_str(),
                Some("shape-second")
            );
            assert_eq!(
                rows.get(2).unwrap().get("object_id").unwrap().as_str(),
                Some("typed😀")
            );
            assert_eq!(
                rows.get(0).unwrap(),
                aj::JsonView::Plain(&raw_withdrawal(&index[1], "superseded_by_revision", None))
            );
            assert_eq!(
                rows.get(0).unwrap().get("superseded_by"),
                Some(aj::JsonView::Null)
            );
            assert!(rows.get(0).unwrap().get("fact_key").is_none());
            assert_eq!(
                rows.get(2).unwrap().get("fact_key").unwrap().as_str(),
                Some("facté")
            );
            assert!(rows.get(2).unwrap().get("window_start").is_none());
            assert_eq!(owned.1.view().as_array().unwrap().len(), 2);
            assert!(budget.snapshot().live_requested_layout_bytes > 0);
            assert!(owned.0.view().allocation_budget().unwrap().is_some());
            assert!(owned.1.view().allocation_budget().unwrap().is_some());
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        // 既覆盖工作集拒绝，也让数组头已获准后在真实JSON字段分配时失败并释放。
        let mut saw_json_refusal = false;
        for limit in [1, 64, 128, 256, 512, 1024, 2048] {
            let tiny = OwnedBudget::new(limit);
            let result = changes_at_budgeted(&index, 2, Some(&tiny));
            match result {
                Err(error) => {
                    assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
                    saw_json_refusal |= tiny.snapshot().allocation_attempts > 3;
                }
                Ok(owners) => drop(owners),
            }
            assert_eq!(tiny.snapshot().live_requested_layout_bytes, 0);
        }
        assert!(saw_json_refusal, "必须实测变化JSON所有者中途预算拒绝");
        let mut invalid = index;
        invalid[1].life.withdrawal_reason = None;
        let old_error = changes_at(&invalid, 2).unwrap_err();
        let owned_error = changes_at_budgeted(&invalid, 2, Some(&budget))
            .err()
            .unwrap();
        assert_eq!(old_error, owned_error);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    fn value(s: &mut dyn NodeSource, r: &ContentRef) -> Value {
        value_ref(s, r.into())
    }
    fn value_ref(s: &mut dyn NodeSource, r: sc::RefView<'_>) -> Value {
        let mut b = Vec::new();
        sc::render_refview(s, r, &mut b).unwrap();
        serde_json::from_slice(&b).unwrap()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn projection_fixture() -> (sc::MemoryNodes, Vec<Entry>) {
        // 仅投影/排序夹具；真实 producer 与成功 load 的绑定由 real_generations 另核。
        let mut nodes = sc::MemoryNodes::new();
        let mut entries = Vec::new();
        for (id, shape, first, death, sup) in [
            ("typed-z", false, 1, Some(3), Some("typed-a")),
            ("shape-first", true, 1, Some(3), Some("shape-replacement")),
            ("shape-second", true, 1, None, None),
            ("typed-b", false, 1, None, None),
            ("future-born", false, 2, None, None),
            ("shape-replacement", true, 3, None, None),
            ("typed-a", false, 3, None, None),
        ] {
            let batch = format!("batch3-{}", "a".repeat(64));
            let mut wire = json!({
                "object_id":id,"object_revision":"1","kind":if shape {SHAPE} else {"typed"},
                "batch_id":batch,"input_refs":[],"source_coords":[],
                "first_known_generation":first.to_string(),"first_known_cut":format!("cut-{first}"),
                "published_generation":first.to_string(),"withdrawn_generation":null,
                "withdrawal_reason":null,"superseded_by":null,"lifecycle":"active"
            });
            let slot = if shape {
                let window = if id == "shape-second" {
                    [7, 9, 10]
                } else {
                    [7, 8, 9]
                };
                wire["branch"] = json!("test");
                wire["dir_ab"] = json!("up");
                wire["dir_bc"] = json!("down");
                for (name, n) in ["window_start", "window_mid", "window_end"]
                    .into_iter()
                    .zip(window)
                {
                    wire[name] = json!(n.to_string());
                }
                wire["comparisons"] = json!([]);
                Slot::Shape(window)
            } else {
                wire["fact_key"] = json!("fact");
                wire["payload"] = json!({"body":"x".repeat(5000)});
                Slot::Typed {
                    kind: "typed".into(),
                    fact_key: "fact".into(),
                }
            };
            let birth = sc::write_json_view(&mut nodes, aj::JsonView::Plain(&wire)).unwrap();
            let e = Entry {
                id: id.into(),
                revision: 1,
                kind: if shape { SHAPE.into() } else { "typed".into() },
                slot,
                generated: birth.clone(),
                internal: birth.clone(),
                immutable: birth.clone(),
                birth_wire: birth.content_ref().into(),
                life: LifeOwner::from_normal(PublicationLife {
                    first_known_generation: first,
                    first_known_cut: format!("cut-{first}"),
                    published_generation: first,
                    withdrawn_generation: death,
                    withdrawal_reason: death.map(|_| "superseded_by_revision".into()),
                    superseded_by: sup.map(str::to_owned),
                    batch: BatchRef::Existing(batch),
                }),
            };
            wire_at(&mut nodes, &e, first).unwrap();
            if let Some(death) = death {
                wire_at(&mut nodes, &e, death).unwrap();
            }
            entries.push(e);
        }
        (nodes, entries)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn projection_memo(capture: &sc::AuditCapture, count: usize) -> NativeProjection<'_> {
        NativeProjection {
            capture,
            wires: vec![[None, None]; count],
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[derive(Default)]
    struct RecordMeter {
        records: std::cell::Cell<usize>,
        stop: bool,
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl super::super::audit_budget::Budget for RecordMeter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn accept_prepared_frame(&self, _: &sc::job::CheckedFrame) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn record(&self) -> Result<(), sc::CodecError> {
            self.records.set(self.records.get() + 1);
            if self.stop {
                Err(sc::CodecError::new("record_stop", "记录门拒绝"))
            } else {
                Ok(())
            }
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_endpoints_sort_ties_records_and_overlay_lifetime() {
        let (mut nodes, entries) = projection_fixture();
        let capture = sc::AuditCapture::new(nodes.clone());
        let mut store = super::super::history::HashReader::captured(&capture, None);
        let mut memo = projection_memo(&capture, entries.len());
        let old_meter = RecordMeter::default();
        let memo_meter = RecordMeter::default();
        for g in 1..=6 {
            let oracle = at_cut_metered(&mut nodes, &entries, g, Some(&old_meter)).unwrap();
            // 测共用 collect 的记录门；生产 Native 构造门仍只允许 budget=None。
            let actual =
                at_cut_projected(&mut store, &entries, g, Some(&memo_meter), Some(&mut memo))
                    .unwrap();
            assert_eq!(oracle, actual, "cut-{g} 三根/头必须一致");
            for (old, new) in [
                (&oracle.0, &actual.0),
                (&oracle.1, &actual.1),
                (&oracle.2, &actual.2),
            ] {
                assert_eq!(
                    value(&mut nodes, &old.content_ref()),
                    value(&mut store, &new.content_ref())
                );
            }
            assert_eq!(old_meter.records.get(), memo_meter.records.get());
            let ids = |v: Value| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|row| row["object_id"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            };
            if g == 1 {
                assert_eq!(
                    ids(value(&mut store, &actual.0.content_ref())),
                    ["shape-first", "shape-second", "typed-b", "typed-z"]
                );
                assert_eq!(
                    ids(value(&mut store, &actual.2.content_ref())),
                    ["typed-z", "shape-first", "shape-second", "typed-b"]
                );
            }
            if g == 3 {
                assert_eq!(
                    ids(value(&mut store, &actual.1.content_ref())),
                    ["shape-first", "typed-z"]
                );
                assert_eq!(
                    ids(value(&mut store, &actual.2.content_ref())),
                    ["shape-replacement", "typed-a"]
                );
            }
            store.clear_overlay();
            // 已填 memo 的每条 wire 闭包只读当前 capture，清除所有派生 body 后仍可渲染。
            for wire in memo.wires.iter().flatten().flatten() {
                capture
                    .verify_refview(wire.view(), sc::Mode::Public)
                    .unwrap();
                value_ref(&mut store, wire.view());
            }
        }
        let stop = RecordMeter {
            records: std::cell::Cell::new(0),
            stop: true,
        };
        WIRE_PROJECTIONS.with(|calls| calls.set(0));
        assert!(
            at_cut_projected(&mut store, &entries, 6, Some(&stop), Some(&mut memo))
                .unwrap_err()
                .contains("record_stop")
        );
        assert_eq!(stop.records.get(), 1);
        assert_eq!(
            WIRE_PROJECTIONS.with(|calls| calls.get()),
            0,
            "记录门必须在命中前拒绝"
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_life_checked_on_hits_and_first_failure_not_cached() {
        let (nodes, mut entries) = projection_fixture();
        let capture = sc::AuditCapture::new(nodes);
        let mut store = super::super::history::HashReader::captured(&capture, None);
        let mut memo = projection_memo(&capture, entries.len());
        memo.wire(&mut store, &entries[0], 0, 1).unwrap();
        assert!(memo
            .wire(&mut store, &entries[0], 0, 0)
            .unwrap_err()
            .contains("尚未出生"));
        for g in [1, 2] {
            let original = entries[0].life.published_generation;
            entries[0].life.published_generation = 10;
            assert!(memo
                .wire(&mut store, &entries[0], 0, g)
                .unwrap_err()
                .contains("晚于本次生成代"));
            entries[0].life.published_generation = original;
        }
        memo.wire(&mut store, &entries[0], 0, 3).unwrap();
        entries[0].life.withdrawal_reason = None;
        for g in [3, 4, 6] {
            assert!(memo
                .wire(&mut store, &entries[0], 0, g)
                .unwrap_err()
                .contains("撤回必须携带"));
        }
        // 第一次 exact 字段检查失败，不能留下成功槽；同一错误重试也必须拒绝。
        let empty = sc::write_value(&mut store, &V::Object(Vec::new())).unwrap();
        entries[1].birth_wire = empty.content_ref().into();
        for _ in 0..2 {
            assert!(memo.wire(&mut store, &entries[1], 1, 1).is_err());
            assert!(memo.wires[1].iter().all(Option::is_none));
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_missing_corrupt_or_overlay_only_closure_never_cached() {
        let (nodes, entries) = projection_fixture();
        let (leaf, tag, body) = nodes
            .iter()
            .find(|(_, tag, body)| *tag == sc::tag::BYTES_CHUNK && body.len() == 4096)
            .unwrap();
        let leaf = *leaf;
        for corrupt in [false, true] {
            let mut damaged = sc::MemoryNodes::new();
            for (d, t, b) in nodes.iter() {
                if *d == leaf {
                    if corrupt {
                        damaged.insert_unchecked(*d, t, vec![0]);
                    }
                } else {
                    damaged.insert_unchecked(*d, t, b.to_vec());
                }
            }
            let capture = sc::AuditCapture::new(damaged);
            let mut store = super::super::history::HashReader::captured(&capture, None);
            // 缺叶可临时补进 overlay 让旧投影成功，但它不能授予跨 cut 存活资格。
            if !corrupt {
                store.put(leaf, tag, body).unwrap();
            }
            let mut memo = projection_memo(&capture, entries.len());
            for _ in 0..2 {
                let error = memo.wire(&mut store, &entries[0], 0, 1).unwrap_err();
                assert!(
                    error.contains(if corrupt {
                        sc::code::NODE_DIGEST
                    } else {
                        sc::code::NODE_MISSING
                    }),
                    "{error}"
                );
                assert!(memo.wires[0].iter().all(Option::is_none));
            }
            store.clear_overlay();
            assert!(memo.wire(&mut store, &entries[0], 0, 2).is_err());
        }
    }
    fn legacy_value(s: &mut dyn NodeSource, e: &Entry) -> Value {
        let mut v = value(s, &e.generated.content_ref());
        v["batch_id"] = match &e.life.batch {
            BatchOwner::Existing(b) => json!(b),
            BatchOwner::CurrentBatch => Value::Null,
        };
        v["withdrawn_generation"] = json!(e.life.withdrawn_generation);
        v["withdrawal_reason"] = json!(e.life.withdrawal_reason);
        v["superseded_by"] = json!(e.life.superseded_by);
        v
    }
    #[test]
    fn three_real_generations_match_shared_diff_and_preserve_historical_cut_and_prior() {
        real_generations(false, false);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_real_generations_reuse_exact_wire_after_overlay_clear() {
        real_generations(true, false);
    }
    fn real_generations(native_projection: bool, native_union: bool) {
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let _ = native_projection;
        let files = TestFiles::new();
        init_test_db(&files);
        let rows = tb02b_oracle_rows();
        let mut conn = database();
        let mut first_cut = None;
        let mut total_deaths = 0;
        let mut previous_raw = Vec::new();
        for (g, range) in [(1, 0..8), (2, 8..14), (3, 0..0)] {
            if g == 3 {
                let mut rev = located_ohlc_input(&[("b2", 20, 11, 13, 9, 11)]);
                rev["events"][0]["revision"] = json!("2");
                accept_ohlc(&files, &rev);
            } else {
                accept_ohlc(&files, &located_ohlc_input(&rows[range]));
            }
            let source = open_db(&files.db()).unwrap();
            let raw = read_raw_events(&source).unwrap();
            let effective = read_effective_events(&raw).unwrap();
            let get = |k| meta_get_opt(&source, k).unwrap().unwrap();
            let profile_hash = get("profile_hash");
            let rule = get("rule_revision");
            let catalog = get("catalog_revision");
            let scope = serde_json::from_str(&get("scope")).unwrap();
            let cut = format!("cut-{g}");
            let tx = conn.transaction().unwrap();
            let mut s = content_store::TxStore::new(&tx).unwrap();
            let old = load(&tx, &mut s, g - 1).unwrap();
            let (bi, version) = prior_records(&old).unwrap();
            let prior = PriorState {
                bi_records: bi.iter().collect(),
                version_record: version.as_ref(),
            };
            let mut runtime = GenerationRuntime::new(
                format!("object-index/{}/{g}", files.db().display()),
                tb02a::PROFILE,
                &rule,
            );
            let input = GenerationInput {
                all_events: (&raw).into(),
                effective: (&effective).into(),
                generation: g,
                structure_cut: &cut,
                input_frontier: raw.len() as i64 - 1,
                profile_id: tb02a::PROFILE,
                profile_hash: &profile_hash,
                rule_revision: &rule,
                catalog_revision: &catalog,
                scope: &scope,
                semantic_commit_ns: None,
                prior,
            };
            let mut generated = runtime.generate(&mut s, &input).unwrap();
            let mut duplicate_position = None;
            if g == 3 {
                // 同一生成代的两个真实producer候选，具有同槽不同身份：前位仍是旧活动身份。
                // 本例专测adapter的旧末位语义，不把混合候选当作真实来源业务验收。
                let earlier_effective = read_effective_events(&previous_raw).unwrap();
                let mut earlier_runtime = GenerationRuntime::new(
                    format!("object-index/earlier/{}/3", files.db().display()),
                    tb02a::PROFILE,
                    &rule,
                );
                let earlier_input = GenerationInput {
                    all_events: (&previous_raw).into(),
                    effective: (&earlier_effective).into(),
                    generation: g,
                    structure_cut: &cut,
                    input_frontier: previous_raw.len() as i64 - 1,
                    profile_id: tb02a::PROFILE,
                    profile_hash: &profile_hash,
                    rule_revision: &rule,
                    catalog_revision: &catalog,
                    scope: &scope,
                    semantic_commit_ns: None,
                    prior: PriorState {
                        bi_records: bi.iter().collect(),
                        version_record: version.as_ref(),
                    },
                };
                let earlier = earlier_runtime.generate(&mut s, &earlier_input).unwrap();
                let old_candidate = earlier
                    .objects
                    .into_iter()
                    .find(|o| {
                        o.fact_key().is_some()
                            && old.iter().any(|e| {
                                e.id == o.object_id() && e.life.withdrawn_generation.is_none()
                            })
                            && generated.objects.iter().any(|n| {
                                n.object_id() != o.object_id()
                                    && n.kind() == o.kind()
                                    && n.fact_key() == o.fact_key()
                            })
                    })
                    .expect("修订应有同槽新旧身份");
                let position = generated
                    .objects
                    .iter()
                    .position(|o| {
                        o.kind() == old_candidate.kind() && o.fact_key() == old_candidate.fact_key()
                    })
                    .unwrap();
                generated.objects.insert(position, old_candidate);
                duplicate_position = Some(position);
            }
            let mut ordered = old
                .iter()
                .filter(|e| e.life.withdrawn_generation.is_none())
                .collect::<Vec<_>>();
            ordered.sort_by(|a, b| ordering(a, b, false));
            let active = ordered.iter().map(|e| legacy_value(&mut s, e)).collect();
            let identities = old
                .iter()
                .map(|e| (e.id.as_str().to_owned(), legacy_value(&mut s, e)))
                .collect();
            let new = generated
                .objects
                .iter()
                .map(|o| value(&mut s, &o.generated().content_ref()))
                .collect::<Vec<_>>();
            let oracle = crate::compute_diff(
                &new,
                &crate::StoredObjects { active, identities },
                crate::delta_storage::Mode::ChangesOnly,
            )
            .unwrap();
            #[cfg(feature = "s_capture_native_testonly")]
            if native_union && g == 1 {
                let root = generated.objects[1].generated().digest;
                let before_nodes: i64 = tx
                    .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
                    .unwrap();
                tx.execute_batch("SAVEPOINT native_union_bad_root;")
                    .unwrap();
                tx.execute(
                    "UPDATE s_content_nodes SET body=x'00' WHERE digest=?1",
                    params![&root[..]],
                )
                .unwrap();
                tx.execute_batch("CREATE TEMP TRIGGER candidate_union_insert_fault BEFORE INSERT ON s_content_nodes BEGIN SELECT RAISE(ABORT,'unexpected_candidate_put_before_union_reject'); END;").unwrap();
                let error = plan_native_union_for_testonly(&mut s, &generated.objects, &old, g)
                    .err()
                    .unwrap();
                assert!(
                    error.contains(sc::code::NODE_DIGEST),
                    "先拒坏完整Public root，未触candidate INSERT：{error}"
                );
                assert_eq!(
                    tx.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    before_nodes
                );
                tx.execute_batch(
                    "ROLLBACK TO native_union_bad_root; RELEASE native_union_bad_root;",
                )
                .unwrap();
            }
            let reads_before = content_store::sql_reads_on_thread();
            let p = plan(&mut s, &generated.objects, &old, g).unwrap();
            let ordinary_reads = content_store::sql_reads_on_thread() - reads_before;
            #[cfg(feature = "s_capture_native_testonly")]
            let p = if native_union {
                let reads_before = content_store::sql_reads_on_thread();
                let old_union =
                    candidate_union_old_fields_for_test(&mut s, &generated.objects, &old, g);
                let old_union_reads = content_store::sql_reads_on_thread() - reads_before;
                let old_union_material = candidate_sql_material_for_test(&tx);
                let reads_before = content_store::sql_reads_on_thread();
                let union =
                    plan_native_union_for_testonly(&mut s, &generated.objects, &old, g).unwrap();
                let union_reads = content_store::sql_reads_on_thread() - reads_before;
                assert_eq!(
                    candidate_plan_signature(&union),
                    candidate_plan_signature(&p)
                );
                assert_eq!(
                    candidate_plan_signature(&union),
                    candidate_plan_signature(&old_union)
                );
                assert_eq!(
                    candidate_sql_material_for_test(&tx),
                    old_union_material,
                    "旧union/新fields必须生成逐字相同SQL节点原件集合"
                );
                assert!(union_reads < old_union_reads, "generated一次字段绑定须减少实际SQL读取：gen{g} old_union={old_union_reads} fields={union_reads}");
                println!("Native candidate fields gen{g}: actual_SQL_reads old_union={old_union_reads} fields={union_reads}; all_plan_roots_metadata_and_node_bytes_equal=1");
                assert!(
                    union_reads < ordinary_reads,
                    "实际SQL读数须减少：gen{g} ordinary={ordinary_reads} union={union_reads}"
                );
                println!("Native candidate union gen{g}: actual_SQL_reads ordinary={ordinary_reads} union={union_reads}; plan roots/metadata identical");
                union
            } else {
                p
            };
            #[cfg(not(feature = "s_capture_native_testonly"))]
            let _ = (native_union, ordinary_reads);
            assert_eq!(p.withdrawals, oracle.1);
            assert_eq!(p.replaces, oracle.2);
            assert_eq!(
                p.births.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
                oracle
                    .0
                    .iter()
                    .map(|v| v["object_id"].as_str().unwrap())
                    .collect::<Vec<_>>()
            );
            assert!(!p.births.is_empty());
            total_deaths += p.withdrawals.len();
            let batch = format!("batch3-{}", g.to_string().repeat(64));
            stage(&tx, &mut s, &p, &batch).unwrap();
            let stored = load(&tx, &mut s, g).unwrap();
            let member_state = sc::write_value(
                &mut s,
                &V::Object(vec![(
                    "objects".into(),
                    V::Array(
                        generated
                            .objects
                            .iter()
                            .map(|o| V::Edge(o.generated().digest))
                            .collect(),
                    ),
                )]),
            )
            .unwrap();
            verify_generated_members(&mut s, &stored, &member_state.content_ref(), g).unwrap();
            let mut sweep = HistorySweep::new(&stored);
            sweep.advance(g).unwrap();
            verify_generated_members_swept(
                &mut s,
                &stored,
                &member_state.content_ref(),
                g,
                Some(&sweep),
            )
            .unwrap();
            if let Some(position) = duplicate_position {
                generated.objects.remove(position);
            }
            // 恢复真实单次输入的原生成集，独立再次验证同一出生/撤回结果；生产断言仍是三代真输入。
            verify_generated_members(&mut s, &stored, &generated.generated_state, g).unwrap();
            verify_generated_members_swept(
                &mut s,
                &stored,
                &generated.generated_state,
                g,
                Some(&sweep),
            )
            .unwrap();
            let (actual_active, actual_dead, actual_new) = at_cut(&mut s, &stored, g).unwrap();
            assert_eq!(actual_active.extent, Some(generated.objects.len() as u64));
            assert_eq!(actual_new.extent, Some(p.births.len() as u64));
            for r in &stored {
                let actual = wire_at(&mut s, r, g).unwrap();
                let expected = crate::project_object_wire(&legacy_value(&mut s, r)).unwrap();
                assert_eq!(value_ref(&mut s, actual.view()), expected);
            }
            let historical = at_cut(&mut s, &stored, 1).unwrap();
            let roots = (
                historical.0.digest,
                historical.1.digest,
                historical.2.digest,
            );
            if let Some(first) = first_cut {
                assert_eq!(roots, first, "未来出生/撤回不得改旧cut");
            } else {
                first_cut = Some(roots);
            }
            let expected_changes = changes_at(&stored, g).unwrap();
            assert_eq!(expected_changes.0, p.withdrawals);
            assert_eq!(expected_changes.1, p.replaces);
            if g == 2 {
                let common = old
                    .iter()
                    .position(|e| {
                        generated
                            .objects
                            .iter()
                            .any(|o| o.object_id() == e.id.as_str())
                    })
                    .expect("应有沿用身份");
                let allocation = s.allocation_budget();
                let mut wrong = bj::new_work(allocation.as_ref());
                for e in old.iter() {
                    wrong.try_push(e.try_clone().unwrap()).unwrap();
                }
                wrong[common].immutable =
                    sc::write_value(&mut s, &V::String("corrupt".into())).unwrap();
                assert!(plan(&mut s, &generated.objects, &wrong, g)
                    .err()
                    .unwrap()
                    .contains("IdentityConflict"));
            }
            let _ = actual_dead;
            #[cfg(feature = "s_capture_native_testonly")]
            if native_projection && g == 3 {
                let capture = content_store::capture_for_prototype(&tx).unwrap();
                let mut captured = super::super::history::HashReader::captured(&capture, None);
                captured.enable_full_arrays_for_test();
                let mut current = load_for_history(&tx, &mut captured, g).unwrap();
                let dual = current
                    .native_dual_at_cut(&mut captured, g, &batch)
                    .unwrap();
                assert_eq!(
                    [dual[0].1.digest, dual[1].1.digest, dual[2].1.digest],
                    [actual_active.digest, actual_dead.digest, actual_new.digest]
                );
                let old_batch = format!("batch3-{}", "1".repeat(64));
                let dual_old = current
                    .native_dual_at_cut(&mut captured, 1, &old_batch)
                    .unwrap();
                assert_eq!(
                    [
                        dual_old[0].1.digest,
                        dual_old[1].1.digest,
                        dual_old[2].1.digest
                    ],
                    [
                        historical.0.digest,
                        historical.1.digest,
                        historical.2.digest
                    ]
                );

                assert!(current.native.is_some());
                let other_capture = content_store::capture_for_prototype(&tx).unwrap();
                let mut other_store =
                    super::super::history::HashReader::captured(&other_capture, None);
                other_store.enable_full_arrays_for_test();
                assert!(current
                    .at_cut(&mut other_store, 1, None)
                    .unwrap_err()
                    .contains("不属于本次捕获"));
                let records = RecordMeter::default();
                assert!(current
                    .at_cut(&mut captured, 1, Some(&records))
                    .unwrap_err()
                    .contains("不属于本次捕获"));
                let mut ordinary_store =
                    super::super::history::HashReader::captured(&capture, None);
                let ordinary = load_for_history(&tx, &mut ordinary_store, g).unwrap();
                assert!(
                    ordinary.native.is_none(),
                    "普通 captured reader 不能取得 memo"
                );
                WIRE_PROJECTIONS.with(|calls| calls.set(0));
                for cut in 1..=6 {
                    let oracle = at_cut(&mut s, &stored, cut).unwrap();
                    let before = WIRE_PROJECTIONS.with(|calls| calls.get());
                    let actual = current.at_cut(&mut captured, cut, None).unwrap();
                    let projection_count = WIRE_PROJECTIONS.with(|calls| calls.get() - before);
                    for (want, got) in [
                        (&oracle.0, &actual.0),
                        (&oracle.1, &actual.1),
                        (&oracle.2, &actual.2),
                    ] {
                        assert_eq!(want, got, "cut-{cut} 三根必须逐位一致");
                        assert_eq!(
                            value(&mut s, &want.content_ref()),
                            value(&mut captured, &got.content_ref())
                        );
                    }
                    if cut > 3 {
                        assert_eq!(projection_count, 0, "相同撤回/活动态不应在后续 cut 重编码");
                    }
                    captured.clear_overlay();
                }
            }
            tx.commit().unwrap();
            previous_raw = raw;
        }
        assert!(total_deaths > 0);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM objects", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        let tx = conn.transaction().unwrap();
        tx.execute("UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT MIN(rowid) FROM s_content_objects)",[]).unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        assert!(load(&tx, &mut s, 3)
            .unwrap_err()
            .contains("不可变内容根不符"));
    }
    #[test]
    fn empty_plan_rolls_back_nodes_and_rejects_missing_independent_table() {
        let mut c = database();
        let tx = c.transaction().unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        let p = plan(&mut s, &[], &[], 1).unwrap();
        assert_eq!(p.active.extent, Some(0));
        assert_eq!(p.withdrawn.extent, Some(0));
        assert_eq!(p.upserts.extent, Some(0));
        tx.rollback().unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM s_content_nodes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        c.execute("DROP TABLE s_content_objects", []).unwrap();
        let tx = c.transaction().unwrap();
        let mut s = content_store::TxStore::new(&tx).unwrap();
        assert!(load(&tx, &mut s, 0).is_err());
    }
}

impl Slot {
    fn try_clone(&self) -> Result<Self, String> {
        Ok(match self {
            Self::Shape(v) => Self::Shape(*v),
            Self::Typed { kind, fact_key } => Self::Typed {
                kind: kind.try_clone()?,
                fact_key: fact_key.try_clone()?,
            },
        })
    }
}
impl Entry {
    pub fn try_clone(&self) -> Result<Self, String> {
        Ok(Self {
            id: self.id.try_clone()?,
            revision: self.revision,
            kind: self.kind.try_clone()?,
            slot: self.slot.try_clone()?,
            generated: self.generated.clone(),
            internal: self.internal.clone(),
            immutable: self.immutable.clone(),
            birth_wire: self.birth_wire.clone(),
            life: self.life.try_clone()?,
        })
    }
}
fn root_ref_bytes(s: &mut (impl CodecRead + ?Sized), bytes: &[u8]) -> Result<RefValue, String> {
    let digest: NodeDigest = bytes.try_into().map_err(|_| err("根不是32字节"))?;
    s.content_ref_view(sc::FixedRoot::new(&digest).as_str())
        .map_err(err)
}
fn root_header_bytes(s: &mut (impl CodecRead + ?Sized), bytes: &[u8]) -> Result<Built, String> {
    let r = root_ref_bytes(s, bytes)?;
    Ok(Built {
        digest: r.digest().map_err(err)?,
        kind: r.kind(),
        extent: r.extent(),
    })
}

/// 同一字段读取核的owned借用结果；引用的root/format不补造String。
struct OwnerFields<'a>(super::audit_store::RefFields<'a>);
impl<'a> OwnerFields<'a> {
    fn read(
        s: &mut (impl CodecStore + ?Sized),
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<Self, String> {
        s.lookup_fields_ref(r, names).map(Self).map_err(err)
    }
    fn get(&self, k: &str) -> Result<RefValue, String> {
        self.0.get(k).ok_or_else(|| err(format!("原件缺字段{k}")))
    }
    fn text(&self, s: &mut (impl CodecRead + ?Sized), k: &str) -> Result<TextValue, String> {
        let r = self.get(k)?;
        bj::read_text(s, r.view()).map_err(err)
    }
    fn integer(&self, s: &mut (impl CodecRead + ?Sized), k: &str) -> Result<i64, String> {
        let r = self.get(k)?;
        bj::render_json(s, r.view())?
            .view()
            .as_i64()
            .ok_or_else(|| err(format!("{k}不是i64")))
    }
    fn equal(&self, k: &str, o: &Self, j: &str) -> Result<(), String> {
        if self.get(k)? != o.get(j)? {
            Err(err(format!("{k}/{j}内容边不同")))
        } else {
            Ok(())
        }
    }
    fn slot(&self, s: &mut (impl CodecStore + ?Sized), kind: &str) -> Result<Slot, String> {
        if kind == SHAPE {
            Ok(Slot::Shape([
                self.integer(s, "window_start")?,
                self.integer(s, "window_mid")?,
                self.integer(s, "window_end")?,
            ]))
        } else {
            Ok(Slot::Typed {
                kind: TextValue::from_str(kind, s.allocation_budget().as_ref())?,
                fact_key: self.text(s, "fact_key")?,
            })
        }
    }
    fn immutable(&self, s: &mut (impl CodecStore + ?Sized), shape: bool) -> Result<Built, String> {
        let names = if shape {
            SHAPE_IMMUTABLE
        } else {
            TYPED_IMMUTABLE
        };
        let mut fields = [("", sc::ValueView::BatchSelf); 12];
        for (i, name) in names.iter().enumerate() {
            let child = self.get(name)?;
            if child.kind() == Kind::Null {
                return Err(err("不可变字段不得为空"));
            }
            fields[i] = (*name, sc::ValueView::Edge(child.digest().map_err(err)?));
        }
        s.write_value_view(sc::ValueView::Object(&fields[..names.len()]))
            .map_err(err)
    }
    fn bind_json_text(
        &self,
        s: &mut (impl CodecRead + ?Sized),
        json: &str,
        value: &str,
    ) -> Result<(), String> {
        let input = self.get(json)?;
        let parsed = s
            .lookup_ref(input.view(), &[PathStep::JsonText])
            .map_err(err)?;
        if parsed != self.get(value)? {
            return Err(err("内部JSON视图与值边不同"));
        }
        Ok(())
    }
}
fn built_ref(s: &(impl CodecRead + ?Sized), b: &Built) -> RefValue {
    #[cfg(feature = "s_allocation_json_testonly")]
    if s.allocation_budget().is_some() {
        return RefValue::Owned(b.owned_ref());
    }
    RefValue::Plain(b.content_ref())
}

fn field_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_ref(r, &[f(k)]).map_err(err)
}
fn text_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<TextValue, String> {
    let v = field_ref(s, r, k)?;
    bj::read_text(s, v.view()).map_err(err)
}
fn integer_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<i64, String> {
    let v = field_ref(s, r, k)?;
    bj::render_json(s, v.view())?
        .view()
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是i64")))
}
