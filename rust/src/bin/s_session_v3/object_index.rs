//! #1394 C：对象独立出生/撤回索引与历史投影。变化判据复用 object_diff::plan。
//! 保存内部记录、生成态、不可变内容和出生 wire 四根；所有来源/载荷保持内容边。
//! 本层证明索引与这些原件互相绑定；原件是否由真实源按正确业务规则产生仍归来源审计。
use super::audit_store::RefValue;
use super::business_json::{self as bj, TextValue};
use super::object_records::{BatchOwner, BatchView, LifeOwner};
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
use std::collections::BTreeSet;

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
/// 返回本代模板：新生维持08，已有记录只借出生字段并改写小生命周期元数据。
fn wire_at(s: &mut (impl CodecStore + ?Sized), e: &Entry, g: i64) -> Result<RefValue, String> {
    if e.life.first_known_generation > g {
        return Err(err("对象尚未出生"));
    }
    let mut life = e.life.view();
    if life.withdrawn_generation.is_some_and(|d| d > g) {
        life.withdrawn_generation = None;
        life.withdrawal_reason = None;
        life.superseded_by = None;
    }
    let saved = life.batch;
    if life.first_known_generation == g {
        life.batch = BatchView::CurrentBatch;
    }
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
        for (_, e) in items {
            if let Some(b) = budget {
                b.record().map_err(err)?;
            }
            refs.try_push(wire_at(s, e, g)?).map_err(err)?;
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
fn bind_originals(s: &mut (impl CodecStore + ?Sized), e: &Entry) -> Result<(), String> {
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
        bind_shape_refs_view(s, generated_refs.view(), wire_refs.view())?;
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
    bind_shape_refs_view(s, generated.into(), wire.into())
}
fn bind_shape_refs_view(
    s: &mut (impl CodecStore + ?Sized),
    generated: sc::RefView<'_>,
    wire: sc::RefView<'_>,
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
                let a_fields = OwnerFields::read(s, ar.view(), &names)?;
                let b_fields = OwnerFields::read(s, br.view(), &names)?;
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
                    if b_fields.text(s, field_name)? != a_fields.integer(s, field_name)?.to_string()
                    {
                        return Err(err("来源数字公开投影不符"));
                    }
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
    load_in_store(tx, &mut super::audit_store::StoreView::generic(s), latest)
}
pub(super) fn load_for_audit(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'_>,
    latest: i64,
) -> Result<sc::CodecVec<Entry>, String> {
    load_in_store(tx, &mut super::audit_store::StoreView::for_audit(s), latest)
}
fn load_in_store(
    tx: &Transaction<'_>,
    s: &mut super::audit_store::StoreView<'_, '_>,
    latest: i64,
) -> Result<sc::CodecVec<Entry>, String> {
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
        bind_originals(s, e)?;
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
/// 按该代真实候选序复用同一diff判据，再对拍独立生命周期；不假定候选槽位唯一。
pub(super) fn verify_generated_members<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
) -> Result<(), String> {
    let state = state.into();
    let allocation = s.allocation_budget();
    let mut by_id = bj::new_map(allocation.as_ref());
    for e in index.iter().filter(|e| e.life.first_known_generation <= g) {
        by_id.insert(e.id.as_str(), e).map_err(err)?;
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
        let e = by_id
            .get(id.as_str())
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
    let changes = object_diff::plan_budgeted(&new, &old, allocation.as_ref())?;
    if !changes
        .births
        .iter()
        .map(|i| new[*i].object_id.as_str())
        .eq(index
            .iter()
            .filter(|e| e.life.first_known_generation == g)
            .map(|e| e.id.as_str()))
    {
        return Err(err("真实出生顺序与独立索引不同"));
    }
    let (expected_withdrawals, expected_replaces) =
        changes_at_budgeted(index, g, allocation.as_ref())?;
    let mut withdrawals = change_array(allocation.as_ref())?;
    for w in changes.withdrawals {
        let e = before[w.old_index].1;
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
            before[r.old_index].1.id.as_str(),
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
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_get_opt, meta_set, open_db, read_effective_events, read_raw_events, tb02a,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use rusqlite::Connection;
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
            let p = plan(&mut s, &generated.objects, &old, g).unwrap();
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
            if let Some(position) = duplicate_position {
                generated.objects.remove(position);
            }
            // 恢复真实单次输入的原生成集，独立再次验证同一出生/撤回结果；生产断言仍是三代真输入。
            verify_generated_members(&mut s, &stored, &generated.generated_state, g).unwrap();
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
