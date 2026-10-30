//! #1394 C：对象独立出生/撤回索引与历史投影。变化判据复用 object_diff::plan。
//! 保存内部记录、生成态、不可变内容和出生 wire 四根；所有来源/载荷保持内容边。
//! 本层证明索引与这些原件互相绑定；原件是否由真实源按正确业务规则产生仍归来源审计。
use newchan_rust::s_content::ContextError as PublishError;
use super::audit_store::RefValue;
// Re-export only the opaque type; its fields and constructor stay history-private.
use super::audit_store::NativeTree;
use super::business_json::{self as bj, TextValue};
pub(crate) use super::history::NativeCutProjection;
use super::layout::Layout;
use super::native_shared::NativeArc;
use super::object_records::{BatchOwner, BatchView, LifeOwner, LifeView};
#[cfg_attr(
    not(test),
    allow(unused_imports, reason = "#1488 再合 #1608：仅测试使用")
)]
use super::{
    generation::ProducedObject,
    object_records::{self, BatchRef, BorrowedObject, PublicationLife},
};
use crate::v3::audit_store::{CodecRead, CodecStore};
use crate::{canonical_json, content_store, object_diff};
#[cfg_attr(
    not(test),
    allow(unused_imports, reason = "#1488 再合 #1608：仅测试使用")
)]
use newchan_rust::s_content::{
    self as sc, path::PathStep, Built, ContentRef, Kind, NodeDigest, NodeSource, NodeStore,
    Value as V,
};
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::{params, Transaction};
use sc::capture_allocation::native_owned::NativeValue;
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use serde_json::{json, Value};
#[allow(unused_imports, reason = "#1488：8组合探针 BTreeSet 无调用方")]
use std::collections::BTreeSet;
#[cfg_attr(
    not(all(test, feature = "s_incremental")),
    allow(unused_imports, reason = "#1488：HashSet 仅 native 测试使用")
)]
use std::collections::HashSet;

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
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
fn field(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<ContentRef, PublishError> {
    s.lookup_path(r, &[f(k)]).map_err(publish_err)
}
fn digest(r: &ContentRef) -> Result<NodeDigest, PublishError> {
    r.digest().map_err(publish_err)
}
#[allow(
    dead_code,
    reason = "#1608：私有辅助函数，当前无调用方；先量再动，保留不删"
)]
fn built(r: ContentRef) -> Result<Built, String> {
    Ok(Built {
        digest: digest(&r)?,
        kind: r.kind,
        extent: r.extent,
    })
}
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
fn scalar(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<Value, PublishError> {
    let r = field(s, r, k)?;
    scalar_value(s, &r)
}
fn scalar_value(s: &mut (impl CodecRead + ?Sized), r: &ContentRef) -> Result<Value, PublishError> {
    if !matches!(r.kind, Kind::Number | Kind::String | Kind::Null) {
        return Err(publish_err("预期有界标量"));
    }
    let mut out = Vec::new();
    s.render_canonical(r, &mut out).map_err(publish_err)?;
    newchan_rust::session_protocol::strict_json(&out).map_err(Into::into)
}

/// 同一原件绑定调用中的固定小字段；大载荷只保存引用，不持久缓存或信任摘要替代闭包。
pub(super) struct Fields(sc::CodecFields);
impl Fields {
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn read(
        s: &mut (impl CodecStore + ?Sized),
        root: &ContentRef,
        names: &[&str],
    ) -> Result<Self, PublishError> {
        s.lookup_object_fields(root, names).map(Self).map_err(publish_err)
    }
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub(super) fn read_subset(
        s: &mut (impl CodecStore + ?Sized),
        root: &ContentRef,
        names: &[&str],
    ) -> Result<Self, String> {
        s.lookup_object_fields_subset(root, names)
            .map(Self)
            .map_err(err)
    }
    pub(super) fn get(&self, name: &str) -> Result<&ContentRef, PublishError> {
        self.0
            .get(name)
            .ok_or_else(|| publish_err(format!("原件缺字段{name}")))
    }
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    #[cfg_attr(
        feature = "s_incremental",
        allow(dead_code, reason = "#1488：8组合探针 native 路径无调用，保留接口")
    )]
    pub(super) fn text(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        name: &str,
    ) -> Result<String, PublishError> {
        scalar_value(s, self.get(name)?)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| publish_err(format!("{name}不是文本")))
    }
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    #[cfg_attr(
        feature = "s_incremental",
        allow(dead_code, reason = "#1488：8组合探针 native 路径无调用，保留接口")
    )]
    pub(super) fn integer(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        name: &str,
    ) -> Result<i64, PublishError> {
        scalar_value(s, self.get(name)?)?
            .as_i64()
            .ok_or_else(|| publish_err(format!("{name}不是i64")))
    }
    #[allow(
        dead_code,
        reason = "#1608：私有辅助函数，当前无调用方；先量再动，保留不删"
    )]
    fn equal(&self, key: &str, other: &Self, other_key: &str) -> Result<(), String> {
        if self.get(key)? != other.get(other_key)? {
            return Err(err(format!("{key}/{other_key}内容边不同")));
        }
        Ok(())
    }
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    #[cfg_attr(
        feature = "s_incremental",
        allow(dead_code, reason = "#1488：8组合探针 native 路径无调用，保留接口")
    )]
    fn slot(&self, s: &mut (impl CodecStore + ?Sized), kind: &str) -> Result<Slot, PublishError> {
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
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn immutable(&self, s: &mut (impl CodecStore + ?Sized), shape: bool) -> Result<Built, PublishError> {
        let names = if shape {
            SHAPE_IMMUTABLE
        } else {
            TYPED_IMMUTABLE
        };
        let mut fields = Vec::new();
        for name in names {
            let child = self.get(name)?;
            if child.kind == Kind::Null {
                return Err(publish_err("不可变字段不得为空"));
            }
            fields.push(((*name).into(), V::Edge(digest(child)?)));
        }
        s.write_value(&V::Object(fields)).map_err(publish_err)
    }
    #[allow(
        dead_code,
        reason = "#1608：私有辅助函数，当前无调用方；先量再动，保留不删"
    )]
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
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
fn text(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<String, PublishError> {
    scalar(s, r, k)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| publish_err(format!("{k}不是文本")))
}
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
fn integer(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, k: &str) -> Result<i64, PublishError> {
    scalar(s, r, k)?
        .as_i64()
        .ok_or_else(|| publish_err(format!("{k}不是i64")))
}
/// 本口只取真实节点头；load返回前统一验证这些根的全部闭包。
#[allow(
    dead_code,
    reason = "#1608：私有辅助函数，当前无调用方；先量再动，保留不删"
)]
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
    #[allow(
        dead_code,
        reason = "#1608：私有辅助函数，当前无调用方；先量再动，保留不删"
    )]
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
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub index: sc::CodecVec<Entry>,
    pub births: sc::CodecVec<Entry>,
    pub withdrawals: bj::JsonOwner,
    pub replaces: bj::JsonOwner,
    pub active: Built,
    pub withdrawn: Built,
    pub upserts: Built,
}
#[cfg(test)]
fn slot(s: &mut (impl CodecRead + ?Sized), r: &ContentRef, kind: &str) -> Result<Slot, PublishError> {
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
#[cfg(test)]
fn immutable(
    s: &mut (impl CodecStore + ?Sized),
    r: &ContentRef,
    is_shape: bool,
) -> Result<Built, PublishError> {
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
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
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
fn push_raw_withdrawal(
    rows: &mut bj::ScopedJsonArray<'_>,
    e: &Entry,
    reason: &str,
    sup: Option<&str>,
) -> Result<(), PublishError> {
    use aj::JsonView as J;
    let common = [
        ("object_id", J::String(e.id.as_str())),
        ("reason", J::String(reason)),
        ("superseded_by", sup.map(J::String).unwrap_or(J::Null)),
    ];
    match &e.slot {
        Slot::Shape(window) => {
            let numbers = window.map(Value::from);
            let fields = [
                common[0],
                common[1],
                common[2],
                ("window_start", J::Plain(&numbers[0])),
                ("window_mid", J::Plain(&numbers[1])),
                ("window_end", J::Plain(&numbers[2])),
            ];
            rows.push(J::object_fields(&fields).map_err(publish_err)?)
        }
        Slot::Typed { fact_key, .. } => {
            let fields = [
                common[0],
                common[1],
                common[2],
                ("fact_key", J::String(fact_key.as_str())),
            ];
            rows.push(J::object_fields(&fields).map_err(publish_err)?)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum OrderKey<T> {
    Shape(i64),
    Typed(T, T, T, i64),
}
fn order_key(e: &Entry, dead: bool) -> OrderKey<&str> {
    match &e.slot {
        Slot::Shape(w) => OrderKey::Shape(if dead {
            e.life.first_known_generation
        } else {
            w[0]
        }),
        Slot::Typed { kind, fact_key } => {
            OrderKey::Typed(kind.as_str(), fact_key.as_str(), e.id.as_str(), e.revision)
        }
    }
}
impl OrderKey<&str> {
    fn owned_admitted(
        self,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<OrderKey<NativeValue<String>>, PublishError> {
        Ok(match self {
            Self::Shape(n) => OrderKey::Shape(n),
            Self::Typed(k, f, i, r) => OrderKey::Typed(
                store.native_string(k).map_err(publish_err)?,
                store.native_string(f).map_err(publish_err)?,
                store.native_string(i).map_err(publish_err)?,
                r,
            ),
        })
    }
}
impl OrderKey<NativeValue<String>> {
    fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        match self {
            Self::Shape(n) => Ok(Self::Shape(*n)),
            Self::Typed(k, f, id, revision) => {
                OrderKey::Typed(k.as_str(), f.as_str(), id.as_str(), *revision)
                    .owned_admitted(store)
            }
        }
    }
}
fn ordering(a: &Entry, b: &Entry, dead: bool) -> std::cmp::Ordering {
    order_key(a, dead).cmp(&order_key(b, dead))
}
/// #1491（ADR 0037 §一）：快照 `objects` / `withdrawn_objects` 的公开主序键，经
/// [`Layout::rank`] 取（代际前缀，[`order_key`]）。活动成员取首获知代，撤回成员取撤回代；
/// layout/1 前缀恒 0，与 [`ordering`] 逐位同序。diff 与 changes 不是公开数组，仍沿
/// [`ordering`] 的 layout/1 原键（#1491 第 1 轮作者评论），两者判的不是同一个序。
/// 撤回成员缺撤回代即报错（fail-closed），不以默认代吸收（#1491 独审 LOW-1）。
fn public_rank(layout: Layout, e: &Entry, dead: bool) -> Result<(i64, OrderKey<&str>), PublishError> {
    let event = if dead {
        e.life
            .withdrawn_generation
            .ok_or_else(|| publish_err(format!("撤回成员 {} 缺撤回代", e.id.as_str())))?
    } else {
        e.life.first_known_generation
    };
    Ok(layout.rank(event, order_key(e, dead)))
}
/// 排序前一次性校验：比较器不能传播错误，故先经本函数把缺撤回代的成员拒在排序之外。
fn check_public_ranks<'e>(
    layout: Layout,
    members: impl IntoIterator<Item = &'e Entry>,
    dead: bool,
) -> Result<(), PublishError> {
    for e in members {
        public_rank(layout, e, dead)?;
    }
    Ok(())
}
/// 仅在 [`check_public_ranks`] 通过后用于排序；两侧均为 `Ok`，比较即公开主序。
fn public_ordering(layout: Layout, a: &Entry, b: &Entry, dead: bool) -> std::cmp::Ordering {
    public_rank(layout, a, dead).map_err(|error| error.to_string()).cmp(&public_rank(layout, b, dead).map_err(|error| error.to_string()))
}

fn candidate(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, PublishError> {
    let root = s.native_built_ref(o.generated()).map_err(publish_err)?;
    match &root {
        RefValue::Plain(reference) => s.verify_audit_fresh(reference, sc::Mode::Public),
        RefValue::Owned(_) => s.verify_many_ref_fresh([root.view()], sc::Mode::Public),
    }
    .map_err(publish_err)?;
    #[cfg(test)]
    if LEGACY_CANDIDATE_FIELDS.with(|v| v.get()) {
        return candidate_verified_legacy_fields(s, o, g);
    }
    candidate_fixed_fields(s, o, root.view(), g)
}
// #1394：仅本模块完整Public核验成功的控制流调用，不导出skip开关或成功token。
#[cfg(test)]
fn candidate_verified_legacy_fields(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, PublishError> {
    let generated = o.generated().clone();
    if integer(s, &generated.content_ref(), "first_known_generation")? != g
        || integer(s, &generated.content_ref(), "published_generation")? != g
        || text(s, &generated.content_ref(), "first_known_cut")? != format!("cut-{g}")
    {
        return Err(publish_err("候选对象不属于本次生成代"));
    }
    let kind = text(s, &generated.content_ref(), "kind")?;
    let revision = integer(s, &generated.content_ref(), "object_revision")?;
    if text(s, &generated.content_ref(), "object_id")? != o.object_id() || kind != o.kind() {
        return Err(publish_err("生成对象元数据不同"));
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
        .map_err(publish_err)?
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

/// #1488：完整 fresh Public 核验之后，一次绑定generated metadata；wire后exact重新读取。
/// 结果引用只活到本 candidate 返回；internal 始终由 wire_template 独立读取。
struct CandidateFixedFields(sc::path::FixedFields<'static>);
impl CandidateFixedFields {
    fn read_metadata(
        s: &mut (impl CodecStore + ?Sized),
        root: sc::RefView<'_>,
        shape: bool,
    ) -> Result<Self, PublishError> {
        const TYPED: &[&str] = &[
            "first_known_generation",
            "published_generation",
            "first_known_cut",
            "kind",
            "object_revision",
            "object_id",
            "fact_key",
        ];
        const SHAPE: &[&str] = &[
            "first_known_generation",
            "published_generation",
            "first_known_cut",
            "kind",
            "object_revision",
            "object_id",
            "window_start",
            "window_mid",
            "window_end",
        ];
        sc::path::lookup_object_metadata_fixed(
            s.generic_source(),
            root,
            if shape { SHAPE } else { TYPED },
        )
        .map(Self)
        .map_err(publish_err)
    }
    fn read_exact(
        s: &mut (impl CodecStore + ?Sized),
        root: sc::RefView<'_>,
        shape: bool,
    ) -> Result<Self, PublishError> {
        sc::path::lookup_object_fields_fixed(
            s.generic_source(),
            root,
            if shape {
                &object_records::SHAPE_SEALED_FIELDS
            } else {
                &object_records::TYPED_SEALED_FIELDS
            },
        )
        .map(Self)
        .map_err(publish_err)
    }
    fn get(&self, name: &str) -> Result<sc::RefView<'_>, PublishError> {
        self.0
            .get_required(name)
            .map(sc::RefOwner::view)
            .map_err(publish_err)
    }
    fn scalar(
        &self,
        s: &mut (impl CodecStore + ?Sized),
        name: &str,
    ) -> Result<bj::JsonOwner, PublishError> {
        let r = self.get(name)?;
        if !matches!(r.kind(), Kind::Number | Kind::String | Kind::Null) {
            return Err(publish_err("预期有界标量"));
        }
        bj::render_json(s, r).map_err(Into::into)
    }
    fn text(&self, s: &mut (impl CodecStore + ?Sized), name: &str) -> Result<TextValue, PublishError> {
        let value = self.scalar(s, name)?;
        let view = value.view();
        let text = view
            .as_str()
            .ok_or_else(|| publish_err(format!("{name}不是文本")))?;
        s.native_text(text)
    }
    fn integer(&self, s: &mut (impl CodecStore + ?Sized), name: &str) -> Result<i64, PublishError> {
        self.scalar(s, name)?
            .view()
            .as_i64()
            .ok_or_else(|| publish_err(format!("{name}不是i64")))
    }
    fn slot(&self, s: &mut (impl CodecStore + ?Sized), kind: &str) -> Result<Slot, PublishError> {
        if kind == SHAPE {
            Ok(Slot::Shape([
                self.integer(s, "window_start")?,
                self.integer(s, "window_mid")?,
                self.integer(s, "window_end")?,
            ]))
        } else {
            Ok(Slot::Typed {
                kind: s.native_text(kind)?,
                fact_key: self.text(s, "fact_key")?,
            })
        }
    }
    fn immutable(&self, s: &mut (impl CodecStore + ?Sized), shape: bool) -> Result<Built, PublishError> {
        let names = if shape {
            SHAPE_IMMUTABLE
        } else {
            TYPED_IMMUTABLE
        };
        let mut fields = [("", sc::ValueView::BatchSelf); 11];
        for (i, name) in names.iter().enumerate() {
            let child = self.get(name)?;
            if child.kind() == Kind::Null {
                return Err(publish_err("不可变字段不得为空"));
            }
            fields[i] = (*name, sc::ValueView::Edge(child.digest().map_err(publish_err)?));
        }
        s.write_value_view(sc::ValueView::Object(&fields[..names.len()]))
            .map_err(publish_err)
    }
}
fn candidate_fixed_fields(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    root: sc::RefView<'_>,
    g: i64,
) -> Result<Entry, PublishError> {
    // 只收metadata；缺字段按下列原消费顺序报错，exact仍留在wire之后。
    let fields = CandidateFixedFields::read_metadata(s, root, o.kind() == SHAPE)?;
    if fields.integer(s, "first_known_generation")? != g
        || fields.integer(s, "published_generation")? != g
        || fields
            .text(s, "first_known_cut")?
            .as_str()
            .strip_prefix("cut-")
            != Some(super::business_json::Decimal::new(g).text())
    {
        return Err(publish_err("候选对象不属于本次生成代"));
    }
    let kind = fields.text(s, "kind")?;
    let revision = fields.integer(s, "object_revision")?;
    if fields.text(s, "object_id")?.as_str() != o.object_id() || kind.as_str() != o.kind() {
        return Err(publish_err("生成对象元数据不同"));
    }
    let slot = fields.slot(s, kind.as_str())?;
    let internal = match o.borrowed() {
        BorrowedObject::Typed(v) => v.record.clone(),
        BorrowedObject::Shape(v) => v.record.clone(),
        BorrowedObject::Fractal(v) => v.record.clone(),
    };
    let life = LifeOwner {
        first_known_generation: g,
        first_known_cut: fields.text(s, "first_known_cut")?,
        published_generation: g,
        withdrawn_generation: None,
        withdrawal_reason: None,
        superseded_by: None,
        batch: BatchOwner::CurrentBatch,
    };
    drop(fields); // 不把绑定结果带过任何wire写入。
    let birth_wire = o.wire_template_view(s, life.view(), g).map_err(publish_err)?;
    let birth_wire = s.native_built_ref(&birth_wire).map_err(publish_err)?;
    // 原件必须在wire写后重新exact读取，触发器改坏generated仍按旧顺序拒绝。
    let fields = CandidateFixedFields::read_exact(s, root, slot.is_shape())?;
    let immutable = fields.immutable(s, slot.is_shape())?;
    Ok(Entry {
        id: s.native_text(o.object_id())?,
        revision,
        kind,
        slot,
        generated: o.generated().clone(),
        internal,
        immutable,
        birth_wire,
        life,
    })
}

#[cfg(test)]
thread_local! { static LAST_CANDIDATE_FIELD_READS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) }; }
#[cfg(test)]
pub(super) fn candidate_field_reads_for_test() -> (u64, u64) {
    LAST_CANDIDATE_FIELD_READS.with(|v| v.get())
}
#[cfg(test)]
thread_local! { static LEGACY_CANDIDATE_FIELDS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// 只用于独立旧字段oracle；生产不含选择开关，也不会失败转旧实现。
#[cfg(test)]
pub(super) fn with_legacy_candidate_fields_for_test<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            LEGACY_CANDIDATE_FIELDS.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(LEGACY_CANDIDATE_FIELDS.with(|v| v.replace(true)));
    f()
}

/// #1394：仅Native联合Public核验成功后，一次绑定当前generated字段。
/// 首读kind/字段结构的错误可提前拒绝；本片保持Native实现与调用点不变。
#[cfg(feature = "s_incremental")]
fn candidate_fields(
    s: &mut (impl CodecStore + ?Sized),
    o: &ProducedObject,
    g: i64,
) -> Result<Entry, PublishError> {
    let generated = o.generated().clone();
    let root = s.native_built_ref(&generated).map_err(publish_err)?;
    let selected = s.lookup_field_ref(root.view(), "kind").map_err(publish_err)?;
    let selected_kind = bj::read_text(s, selected.view())?;
    let fields = OwnerFields::read(
        s,
        root.view(),
        if selected_kind == SHAPE {
            &object_records::SHAPE_SEALED_FIELDS
        } else {
            &object_records::TYPED_SEALED_FIELDS
        },
    )?;
    if fields.integer(s, "first_known_generation")? != g
        || fields.integer(s, "published_generation")? != g
        || fields.text(s, "first_known_cut")?.strip_prefix("cut-")
            != Some(super::business_json::Decimal::new(g).text())
    {
        return Err(publish_err("候选对象不属于本次生成代"));
    }
    let kind = fields.text(s, "kind")?;
    let revision = fields.integer(s, "object_revision")?;
    if fields.text(s, "object_id")? != o.object_id() || kind != selected_kind || kind != o.kind() {
        return Err(publish_err("生成对象元数据不同"));
    }
    let slot = fields.slot(s, &kind)?;
    let internal = match o.borrowed() {
        BorrowedObject::Typed(v) => v.record.clone(),
        BorrowedObject::Shape(v) => v.record.clone(),
        BorrowedObject::Fractal(v) => v.record.clone(),
    };
    let life = LifeOwner {
        first_known_generation: g,
        first_known_cut: fields.text(s, "first_known_cut")?,
        published_generation: g,
        withdrawn_generation: None,
        withdrawal_reason: None,
        superseded_by: None,
        batch: BatchOwner::CurrentBatch,
    };
    // internal与generated是不同原件，仍由原wire_template读取并核内部JSON/value边。
    let birth_wire = o.wire_template_view(s, life.view(), g).map_err(publish_err)?;
    let birth_wire = s.native_built_ref(&birth_wire).map_err(publish_err)?;
    let immutable = fields.immutable(s, slot.is_shape())?;
    Ok(Entry {
        id: s.native_text(o.object_id())?,
        revision,
        kind,
        slot,
        generated,
        internal,
        immutable,
        birth_wire,
        life,
    })
}
// #1488 PathStep 的公开接口需要 String；许可先于真实分配，局部路径先于许可析构。
fn replace_batch_field(
    s: &mut (impl CodecStore + ?Sized),
    wire: sc::RefView<'_>,
    batch: &str,
) -> Result<RefValue, PublishError> {
    let _path_permit = s
        .codec_work()
        .map(|work| work.reserve_native("batch_id".len()))
        .transpose()
        .map_err(publish_err)?;
    let path = [f("batch_id")];
    s.replace_ref(wire, &path, batch).map_err(publish_err)
}
/// 返回本代模板：新生维持08，已有记录只借出生字段并改写小生命周期元数据。
fn wire_at(s: &mut (impl CodecStore + ?Sized), e: &Entry, g: i64) -> Result<RefValue, PublishError> {
    #[cfg(test)]
    WIRE_PROJECTIONS.with(|calls| calls.set(calls.get() + 1));
    let life = effective_life(e, g)?;
    let saved = e.life.view().batch;
    let built =
        object_records::reproject_stored_wire_view(s, e.birth_wire.view(), life, g).map_err(publish_err)?;
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if s.allocation_budget().is_some() {
        return Err(publish_err("owned引用执行未启用"));
    }
    let mut wire = built_ref(s, &built)?;
    if e.life.first_known_generation == g {
        if let BatchView::Existing(batch) = saved {
            wire = replace_batch_field(s, wire.view(), batch)?;
        }
    }
    Ok(wire)
}

/// cut 上下文仍逐次建立；出生 CurrentBatch 是中间态，最终 wire 恢复固定首版批号。
fn effective_life(e: &Entry, g: i64) -> Result<LifeView<'_>, PublishError> {
    if e.life.first_known_generation > g {
        return Err(publish_err("对象尚未出生"));
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
    /// 库内 meta `rule_revision` 的布局（[`Layout::of_library`]）；投影与审计次序同源。
    layout: Layout,
}
struct NativeProjection<'capture> {
    capture: &'capture sc::AuditCapture,
    // 序号由私有、不可变 entries 给定；同一对象只有 active / withdrawn 两个最终 wire。
    wires: sc::CodecVec<[Option<RefValue>; 2]>,
    // 只读事件表与 before-g 成员审计共享；through-g 游标与集合归本 owner。
    sweep: Option<ProjectionSweep>,
    #[cfg(feature = "s_incremental")]
    retained: super::audit_retention::RetainedDelta,
}
impl NativeProjection<'_> {
    fn remember_wire(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        ordinal: usize,
        phase: usize,
        wire: &RefValue,
    ) -> Result<(), PublishError> {
        let saved = s.native_ref_clone(wire).map_err(publish_err)?;
        #[cfg(feature = "s_incremental")]
        {
            if let Some(old) = &self.wires[ordinal][phase] {
                self.retained.release(old);
            }
            self.retained.add(&saved);
        }
        self.wires[ordinal][phase] = Some(saved);
        Ok(())
    }

    fn wire(
        &mut self,
        s: &mut (impl CodecStore + ?Sized),
        e: &Entry,
        ordinal: usize,
        g: i64,
    ) -> Result<RefValue, PublishError> {
        let life = effective_life(e, g)?;
        let phase = usize::from(life.withdrawn_generation.is_some());
        if let Some(wire) = &self.wires[ordinal][phase] {
            // 命中也必须按本 cut 跑唯一生命周期判据；不能拿旧代成功代替当前准入。
            object_records::validate_life_view(life, g, e.id.as_str()).map_err(publish_err)?;
            s.native_verify_original(self.capture, wire.view(), sc::Mode::Public)
                .map_err(publish_err)?;
            return s.native_ref_clone(wire).map_err(publish_err);
        }
        let wire = wire_at(s, e, g)?;
        // clear_overlay 每 cut 清除派生 body。只允许当前 capture 自己证明的完整 Public
        // 闭包进入 memo，不保留临时 overlay 引用，也不靠 root 存在替代闭包检查。
        s.native_verify_original(self.capture, wire.view(), sc::Mode::Public)
            .map_err(publish_err)?;
        self.remember_wire(s, ordinal, phase, &wire)?;
        Ok(wire)
    }
}
impl HistoryObjects<'_> {
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
    )]
    pub(super) fn native_sweep(&mut self) -> Option<HistorySweep> {
        self.native_sweep_admitted(&super::audit_store::NativeAdmission(None))
            .expect("无scope的Native事件构造")
    }
    pub(super) fn native_sweep_admitted(
        &mut self,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<Option<HistorySweep>, String> {
        let Some(native) = self.native.as_mut() else {
            return Ok(None);
        };
        if native.sweep.is_none() {
            native.sweep = Some(ProjectionSweep::new_admitted(&self.entries, store)?);
        }
        let projection = native.sweep.as_ref().unwrap();
        Ok(Some(HistorySweep::from_events(NativeArc::clone(
            &projection.events,
        ))))
    }
    pub(super) fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub(super) fn layout(&self) -> Layout {
        self.layout
    }
    #[cfg_attr(
        feature = "s_incremental",
        allow(dead_code, reason = "#1608：s_incremental 构建不触达此项")
    )]
    pub(super) fn into_entries(self) -> sc::CodecVec<Entry> {
        self.entries
    }
    /// 归属随连接移动；事件Arc只借给在途facade，返回后保持唯一可增长的事件所有者。
    #[cfg(feature = "s_incremental")]
    pub(super) fn into_entries_preserving(
        mut self,
        owner: &mut Option<HistorySweep>,
    ) -> sc::CodecVec<Entry> {
        if let (Some(native), Some(owner)) = (self.native.take(), owner.as_mut()) {
            owner.retained.merge(native.retained);
            if let Some(sweep) = native.sweep {
                owner.retained.merge(sweep.retained);
                owner.projection = Some(OwnerProjection {
                    wires: native.wires,
                    active: sweep.active,
                    withdrawn: sweep.withdrawn,
                    previous_cut: sweep.previous_cut,
                    #[cfg(test)]
                    visits: sweep.visits,
                });
            }
        }
        self.entries
    }

    pub(super) fn at_cut_bound_births(
        &mut self,
        tx: &Transaction<'_>,
        s: &mut super::history::HashReader<'_>,
        g: i64,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<(Built, Built, Built), PublishError> {
        if !tx.is_readonly(rusqlite::MAIN_DB).map_err(publish_err)?
            || !s.ordinary_bound_births_qualified()
            || self.native.is_some()
        {
            return Err(publish_err("出生根复用不属于普通只读完整索引上下文"));
        }
        at_cut_projected_bound(s, self.layout, &self.entries, g, budget, None, true)
    }

    pub(super) fn at_cut(
        &mut self,
        s: &mut super::history::HashReader<'_>,
        g: i64,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<(Built, Built, Built), PublishError> {
        if let Some(native) = &self.native {
            if budget.is_some()
                || !s
                    .native_projection_capture()
                    .is_some_and(|current| std::ptr::eq(current, native.capture))
            {
                return Err(publish_err("Native投影不属于本次捕获审计上下文"));
            }
        }
        at_cut_projected(
            s,
            self.layout,
            &self.entries,
            g,
            budget,
            self.native.as_mut(),
        )
    }
}

impl HistoryObjects<'_> {
    /// Only this complete-load Native owner can produce both projections. No arbitrary arrays.
    pub(super) fn native_dual_at_cut(
        &mut self,
        s: &mut super::history::HashReader<'_>,
        g: i64,
        batch: &str,
    ) -> Result<[(Built, Built); 3], PublishError> {
        let native = self
            .native
            .as_mut()
            .ok_or_else(|| publish_err("缺Native完整索引资格"))?;
        if !s
            .native_projection_capture()
            .is_some_and(|c| std::ptr::eq(c, native.capture))
        {
            return Err(publish_err("双投影capture不符"));
        }
        if native.sweep.is_none() {
            native.sweep = Some(ProjectionSweep::new_admitted(&self.entries, s)?);
        }
        let sweep = native.sweep.as_mut().unwrap();
        sweep.seek_admitted(g, self.layout, &self.entries, s)?;
        // #1394：先为真实 Vec 容量取得许可，避免 collect 的最小扩容超出元素数上界。
        let mut active = sc::CodecVec::default();
        s.native_codec_room(&mut active, sweep.active.len())
            .map_err(publish_err)?;
        for (_, i) in sweep.active.keys() {
            active.try_push((*i, &self.entries[*i])).map_err(publish_err)?;
        }
        let mut dead = sc::CodecVec::default();
        s.native_codec_room(&mut dead, sweep.withdrawn.len())
            .map_err(publish_err)?;
        for (_, i) in sweep.withdrawn.keys() {
            dead.try_push((*i, &self.entries[*i])).map_err(publish_err)?;
        }
        let born = sweep
            .events
            .births
            .get(&g)
            .map(|bucket| &bucket[..])
            .unwrap_or(&[]);
        let mut births = sc::CodecVec::default();
        s.native_codec_room(&mut births, born.len()).map_err(publish_err)?;
        for i in born {
            births.try_push((*i, &self.entries[*i])).map_err(publish_err)?;
        }
        #[cfg(test)]
        {
            sweep.visits.output_entries += active.len() + dead.len() + births.len();
        }
        // 仅当前cut的新生对象在active/dead与upserts重复；旧对象复用既有两phase wire。
        // 派生Template不进入持久Public槽，pair表只保存本代出生项。
        let mut pairs = NativeTree::<usize, (RefValue, RefValue)>::default();
        let mut collect = |items: sc::CodecVec<(usize, &Entry)>| -> Result<(Built, Built), PublishError> {
            let mut template = sc::CodecVec::default();
            let mut public = sc::CodecVec::default();
            for (ordinal, e) in items {
                if e.life.published_generation > g {
                    return Err(publish_err("对象出生后发布代号被改写"));
                }
                let saved = match e.life.view().batch {
                    BatchView::Existing(v) => v,
                    BatchView::CurrentBatch => return Err(publish_err("历史索引未绑定实际出生batch")),
                };
                if (e.life.first_known_generation == g) != (saved == batch) {
                    return Err(publish_err("出生/旧代batch与当前cut不符"));
                }
                let pair = if e.life.first_known_generation == g {
                    if let Some(pair) = pairs.get(&ordinal) {
                        (
                            s.native_ref_clone(&pair.0).map_err(publish_err)?,
                            s.native_ref_clone(&pair.1).map_err(publish_err)?,
                        )
                    } else {
                        let life = effective_life(e, g)?;
                        object_records::validate_life_view(life, g, e.id.as_str()).map_err(publish_err)?;
                        let phase = usize::from(life.withdrawn_generation.is_some());
                        #[cfg(test)]
                        WIRE_PROJECTIONS.with(|v| v.set(v.get() + 1));
                        let b = object_records::reproject_stored_wire_view(
                            s,
                            e.birth_wire.view(),
                            life,
                            g,
                        )
                        .map_err(publish_err)?;
                        let template = built_ref(s, &b)?;
                        let public = replace_batch_field(s, template.view(), saved)?;
                        s.native_verify_original(native.capture, public.view(), sc::Mode::Public)
                            .map_err(publish_err)?;
                        native.remember_wire(s, ordinal, phase, &public)?;
                        let pair = (template, public);
                        pairs
                            .insert_admitted(
                                ordinal,
                                (
                                    s.native_ref_clone(&pair.0).map_err(publish_err)?,
                                    s.native_ref_clone(&pair.1).map_err(publish_err)?,
                                ),
                                s,
                            )
                            .map_err(publish_err)?;
                        pair
                    }
                } else {
                    // 保留旧双投影的逐对象生命周期检查；memo仍按本cut再核。
                    let life = effective_life(e, g)?;
                    object_records::validate_life_view(life, g, e.id.as_str()).map_err(publish_err)?;
                    let public = native.wire(s, e, ordinal, g)?;
                    (s.native_ref_clone(&public).map_err(publish_err)?, public)
                };
                let (a, b) = pair;
                s.native_codec_room(&mut template, 1).map_err(publish_err)?;
                s.native_codec_room(&mut public, 1).map_err(publish_err)?;
                template.try_push(a).map_err(publish_err)?;
                public.try_push(b).map_err(publish_err)?;
            }
            Ok((array(s, &template)?, array(s, &public)?))
        };
        Ok([collect(active)?, collect(dead)?, collect(births)?])
    }
}

#[cfg(test)]
impl HistoryObjects<'_> {
    // #1394冻结旧扫描算法，只作合法生命周期对照，不接生产失败分支。
    /// Only this complete-load Native owner can produce both projections. No arbitrary arrays.
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
    )]
    fn native_dual_at_cut_scan_for_test(
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
        let layout = self.layout;
        check_public_ranks(layout, dead.iter().map(|m| m.1), true)?;
        active
            .sort_unstable_by(|a, b| public_ordering(layout, a.1, b.1, false).then(a.0.cmp(&b.0)));
        dead.sort_unstable_by(|a, b| public_ordering(layout, a.1, b.1, true).then(a.0.cmp(&b.0)));
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
                        let template = s.native_built_ref(&b).map_err(err)?;
                        let public = s
                            .replace_ref(template.view(), &[f("batch_id")], saved)
                            .map_err(err)?;
                        s.native_verify_original(native.capture, public.view(), sc::Mode::Public)
                            .map_err(err)?;
                        native.wires[ordinal][phase] =
                            Some(s.native_ref_clone(&public).map_err(err)?);
                        (template, public)
                    } else {
                        let public = native.wire(s, e, ordinal, g)?;
                        (s.native_ref_clone(&public).map_err(err)?, public)
                    };
                    pairs.insert(ordinal, pair);
                }
                let (a, b) = pairs.get(&ordinal).unwrap();
                template.push(s.native_ref_clone(a).map_err(err)?);
                public.push(s.native_ref_clone(b).map_err(err)?);
            }
            Ok((array(s, &template)?, array(s, &public)?))
        };
        Ok([collect(active)?, collect(dead)?, collect(births)?])
    }
}

fn array(s: &mut (impl CodecStore + ?Sized), refs: &[RefValue]) -> Result<Built, PublishError> {
    let allocation = s.allocation_budget();
    let mut views = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut views, refs.len()).map_err(publish_err)?;
    for r in refs {
        views
            .try_push(sc::ValueView::Edge(r.digest().map_err(publish_err)?))
            .map_err(publish_err)?;
    }
    s.write_template_view(sc::ValueView::Array(&views))
        .map_err(publish_err)
}
/// 完整历史索引按目标代投影。Delta 出生序保留行插入序；活动/撤回按库布局的公开主序
/// （[`public_ordering`]）。
pub(super) fn at_cut(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    index: &[Entry],
    g: i64,
) -> Result<(Built, Built, Built), PublishError> {
    at_cut_metered(s, layout, index, g, None)
}
pub(super) fn at_cut_metered(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(Built, Built, Built), PublishError> {
    at_cut_projected(s, layout, index, g, budget, None)
}
fn at_cut_projected(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
    native: Option<&mut NativeProjection<'_>>,
) -> Result<(Built, Built, Built), PublishError> {
    at_cut_projected_bound(s, layout, index, g, budget, native, false)
}
fn at_cut_projected_bound(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
    mut native: Option<&mut NativeProjection<'_>>,
    bound_births: bool,
) -> Result<(Built, Built, Built), PublishError> {
    let allocation = s.allocation_budget();
    let mut active = bj::new_work(allocation.as_ref());
    let mut dead = bj::new_work(allocation.as_ref());
    // 三份工作集按旧求值顺序建立；尚未读取wire，budget gate仍在逐对象投影之前。
    for (ordinal, e) in index.iter().enumerate().filter(|(_, e)| {
        e.life.first_known_generation <= g && e.life.withdrawn_generation.is_none_or(|d| d > g)
    }) {
        s.native_codec_room(&mut active, 1).map_err(publish_err)?;
        active.try_push((ordinal, e)).map_err(publish_err)?;
    }
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.withdrawn_generation.is_some_and(|d| d <= g))
    {
        s.native_codec_room(&mut dead, 1).map_err(publish_err)?;
        dead.try_push((ordinal, e)).map_err(publish_err)?;
    }
    check_public_ranks(layout, dead.iter().map(|m| m.1), true)?;
    active.sort_unstable_by(|a, b| public_ordering(layout, a.1, b.1, false).then(a.0.cmp(&b.0)));
    dead.sort_unstable_by(|a, b| public_ordering(layout, a.1, b.1, true).then(a.0.cmp(&b.0)));
    let mut births = bj::new_work(allocation.as_ref());
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.first_known_generation == g)
    {
        s.native_codec_room(&mut births, 1).map_err(publish_err)?;
        births.try_push((ordinal, e)).map_err(publish_err)?;
    }
    let mut collect = |items: sc::CodecVec<(usize, &Entry)>| -> Result<Built, PublishError> {
        let mut refs = bj::new_work(allocation.as_ref());
        s.native_codec_room(&mut refs, items.len()).map_err(publish_err)?;
        for (ordinal, e) in items {
            if let Some(b) = budget {
                b.record().map_err(publish_err)?;
            }
            let wire = match native.as_deref_mut() {
                Some(native) => native.wire(s, e, ordinal, g)?,
                None if bound_births => {
                    let life = effective_life(e, g)?;
                    object_records::validate_life_view(life, g, e.id.as_str()).map_err(publish_err)?;
                    if life.withdrawn_generation.is_none()
                        && matches!(e.life.view().batch, BatchView::Existing(_))
                    {
                        s.native_ref_clone(&e.birth_wire).map_err(publish_err)?
                    } else {
                        wire_at(s, e, g)?
                    }
                }
                None => wire_at(s, e, g)?,
            };
            refs.try_push(wire).map_err(publish_err)?;
        }
        array(s, &refs)
    };
    Ok((collect(active)?, collect(dead)?, collect(births)?))
}

/// 测试入口：布局取当前写者规则串，与 fixture 库 init 写入的 meta 同源。
#[cfg(test)]
pub(super) fn plan(
    s: &mut (impl CodecStore + ?Sized),
    objects: &[ProducedObject],
    old: &[Entry],
    g: i64,
) -> Result<Plan, PublishError> {
    let layout = Layout::of_rule(crate::V3_RULE_REVISION).map_err(publish_err)?;
    plan_for_layout(s, layout, objects, old, g)
}
/// `layout` 由调用方取自库内 meta `rule_revision`（[`Layout::of_rule`]），只定公开数组次序。
pub(super) fn plan_for_layout(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    objects: &[ProducedObject],
    old: &[Entry],
    g: i64,
) -> Result<Plan, PublishError> {
    if g < 1 {
        return Err(publish_err("非法代号"));
    }
    #[cfg(test)]
    let observed_before = (
        content_store::sql_reads_on_thread(),
        sc::frame_calls_on_thread(),
    );
    let allocation = s.allocation_budget();
    let mut new = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut new, objects.len()).map_err(publish_err)?;
    for object in objects {
        new.try_push(candidate(s, object, g)?).map_err(publish_err)?;
    }
    #[cfg(test)]
    LAST_CANDIDATE_FIELD_READS.with(|v| {
        v.set((
            content_store::sql_reads_on_thread() - observed_before.0,
            sc::frame_calls_on_thread() - observed_before.1,
        ))
    });
    plan_from_candidates(s, layout, new, old, g)
}

/// #1394：只由Runtime显式CapturedNative模式选择，预算存在时拒绝。
/// 联合核验同事务生成完成的Public根，先于任何candidate wire/immutable写入。
/// 多处坏状态或wire故障的首报次序可改变，不扩至默认/Owned/Job路径。
#[cfg(feature = "s_incremental")]
pub(super) fn plan_native_union(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    objects: &[ProducedObject],
    old: &[Entry],
    g: i64,
) -> Result<Plan, PublishError> {
    if g < 1 {
        return Err(publish_err("非法代号"));
    }
    if s.allocation_budget().is_some() {
        return Err(publish_err("Native候选联合核验拒绝allocation预算"));
    }
    s.verify_audit_many_fresh(
        objects.iter().map(|o| o.generated().content_ref()),
        sc::Mode::Public,
    )
    .map_err(publish_err)?;
    let mut new = bj::new_work(None);
    s.native_codec_room(&mut new, objects.len()).map_err(publish_err)?;
    for object in objects {
        new.try_push(candidate_fields(s, object, g)?).map_err(publish_err)?;
    }
    plan_from_candidates(s, layout, new, old, g)
}
fn plan_from_candidates(
    s: &mut (impl CodecStore + ?Sized),
    layout: Layout,
    new: sc::CodecVec<Entry>,
    old: &[Entry],
    g: i64,
) -> Result<Plan, PublishError> {
    let mut ids = NativeTree::default();
    for e in &new {
        if ids
            .insert_admitted(e.id.try_clone_admitted(s.codec_work())?, (), s)
            .map_err(publish_err)?
            .is_some()
        {
            return Err(publish_err("生成集重复对象身份"));
        }
    }
    let allocation = s.allocation_budget();
    let mut order = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut order, old.len()).map_err(publish_err)?;
    for ordinal in 0..old.len() {
        order.try_push(ordinal).map_err(publish_err)?;
    }
    // 相同排序键以旧ordinal打破平局，与原stable排序逐位同序而无需额外scratch。
    order.sort_unstable_by(|a, b| ordering(&old[*a], &old[*b], false).then(a.cmp(b)));
    let mut old_keys = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut old_keys, order.len())
        .map_err(publish_err)?;
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
            .map_err(publish_err)?;
    }
    let mut keys = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut keys, new.len()).map_err(publish_err)?;
    for e in &new {
        keys.try_push(object_diff::Key {
            object_id: e.id.as_str(),
            slot: &e.slot,
            immutable: object_diff::Ready(e.immutable.digest),
        })
        .map_err(publish_err)?;
    }
    let changes =
        object_diff::plan_admitted(&keys, &old_keys, allocation.as_ref(), s.codec_work())?;
    let mut index = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut index, old.len()).map_err(publish_err)?;
    for e in old {
        index.try_push(e.try_clone_admitted(s)?).map_err(publish_err)?;
    }
    let mut withdrawals = change_array(s, allocation.as_ref())?;
    let mut replaces = change_array(s, allocation.as_ref())?;
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
                    new[w.new_index.ok_or_else(|| publish_err("替代缺新对象"))?]
                        .id
                        .try_clone_admitted(s.codec_work())?,
                ),
            ),
        };
        push_raw_withdrawal(&mut withdrawals, e, reason, sup.as_deref())?;
        e.life.withdrawn_generation = Some(g);
        e.life.withdrawal_reason = Some(TextValue::from_str_admitted(
            reason,
            allocation.as_ref(),
            s.codec_work(),
        )?);
        e.life.superseded_by = sup;
    }
    for r in changes.replaces {
        let fields = [
            (
                "new_object_id",
                aj::JsonView::String(new[r.new_index].id.as_str()),
            ),
            (
                "old_object_id",
                aj::JsonView::String(old[order[r.old_index]].id.as_str()),
            ),
        ];
        replaces.push(aj::JsonView::object_fields(&fields).map_err(publish_err)?)?;
    }
    let mut births = bj::new_work(allocation.as_ref());
    s.native_codec_room(&mut births, changes.births.len())
        .map_err(publish_err)?;
    s.native_codec_room(&mut index, changes.births.len())
        .map_err(publish_err)?;
    for ordinal in changes.births {
        births
            .try_push(new[ordinal].try_clone_admitted(s)?)
            .map_err(publish_err)?;
    }
    for e in &births {
        index.try_push(e.try_clone_admitted(s)?).map_err(publish_err)?;
    }
    let withdrawals = withdrawals.finish();
    let replaces = replaces.finish();
    let (active, withdrawn, upserts) = at_cut(s, layout, &index, g)?;
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
    let generated_ref = built_ref(s, &e.generated)?;
    let internal_ref = built_ref(s, &e.internal)?;
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
        r.equal(s, key, &i, key)?;
    }
    for key in if shape {
        &SHAPE_IMMUTABLE[1..]
    } else {
        &TYPED_IMMUTABLE[1..]
    } {
        if *key == "comparisons_json" {
            continue;
        }
        r.equal(s, key, &i, key)?;
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
    i.equal(s, "source_coords", &w, "source_coords")?;
    if shape {
        for key in ["branch", "dir_ab", "dir_bc"] {
            i.equal(s, key, &w, key)?;
        }
        for key in ["window_start", "window_mid", "window_end"] {
            if w.text(s, key)? != bj::Decimal::new(i.integer(s, key)?).text() {
                return Err(err("窗口公开投影不符"));
            }
        }
        let comparison = i.get(s, "comparisons")?;
        if let Some(work) = s.codec_work() {
            struct Out {
                bytes: sc::capture_allocation::scratch::ScratchBytes,
                error: Option<sc::CodecError>,
            }
            impl std::io::Write for Out {
                fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                    self.bytes.try_extend_bytes(bytes).map_err(|e| {
                        self.error = Some(e);
                        std::io::Error::from(std::io::ErrorKind::Other)
                    })?;
                    Ok(bytes.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let mut out = Out {
                bytes: work.bytes(),
                error: None,
            };
            let result = s.render_ref(comparison.view(), &mut out);
            if let Some(e) = out.error {
                return Err(err(e));
            }
            result.map_err(err)?;
            if r.text(s, "comparisons_json")?.as_bytes() != &*out.bytes {
                return Err(err("comparisons文本与内部值不符"));
            }
        } else {
            let mut out = Vec::new();
            s.render_ref(comparison.view(), &mut out).map_err(err)?;
            if r.text(s, "comparisons_json")?.as_bytes() != out {
                return Err(err("comparisons文本与内部值不符"));
            }
        }
        i.equal(s, "comparisons", &w, "comparisons")?;
        let generated_refs = i.get(s, "input_refs")?;
        let wire_refs = w.get(s, "input_refs")?;
        bind_shape_refs_view(s, generated_refs.view(), wire_refs.view(), leaf_bindings)?;
    } else {
        i.bind_json_text(s, "payload_json", "payload")?;
        for key in ["fact_key", "payload", "input_refs"] {
            i.equal(s, key, &w, key)?;
        }
    }
    if w.text(s, "object_id")? != e.id
        || w.text(s, "kind")? != e.kind
        || w.text(s, "object_revision")? != bj::Decimal::new(e.revision).text()
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
#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
fn bind_shape_refs(
    s: &mut (impl CodecStore + ?Sized),
    generated: &ContentRef,
    wire: &ContentRef,
) -> Result<(), String> {
    bind_shape_refs_view(s, generated.into(), wire.into(), &mut None)
}

type ShapeLeafDeclaration = (NativeValue<String>, NativeValue<String>, Kind, Option<u64>);
/// #1394：只活在一次完整 Native load 中，保存来源叶八字段关系的成功结果。
/// capture 从本次实际 HashReader 的正资格入口取得，不接受调用方预填 memo。
struct ShapeLeafBindings<'capture> {
    capture: &'capture sc::AuditCapture,
    successful: bj::BusinessMap<[ShapeLeafDeclaration; 2], ()>,
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
            s.native_verify_original(self.capture, ar, sc::Mode::Public)
                .map_err(err)?;
            s.native_verify_original(self.capture, br, sc::Mode::Public)
                .map_err(err)?;
            let declaration = |r: sc::RefView<'_>| -> Result<ShapeLeafDeclaration, String> {
                Ok((
                    s.native_string(r.format()).map_err(err)?,
                    s.native_string(r.root()).map_err(err)?,
                    r.kind(),
                    r.extent(),
                ))
            };
            let key = [declaration(ar)?, declaration(br)?];
            if !self.successful.contains_key(&key) {
                bind_shape_leaf(s, ar, br)?;
                s.native_map_room(&mut self.successful).map_err(err)?;
                self.successful.insert(key, ()).map_err(err)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.successful.retain(|_, _| false);
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
        a_fields.equal(s, field_name, &b_fields, field_name)?;
    }
    for field_name in ["revision", "seq"] {
        if b_fields.text(s, field_name)?
            != bj::Decimal::new(a_fields.integer(s, field_name)?).text()
        {
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
                != bj::Decimal::new(integer_ref(s, a.view(), "merged_index")?).text()
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
                    (
                        s.native_ref_clone(&a[k as usize]).map_err(err)?,
                        s.native_ref_clone(&b[k as usize]).map_err(err)?,
                    )
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
#[cfg(test)]
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
        None,
    )
}
#[cfg_attr(
    not(all(test, feature = "s_incremental")),
    allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
)]
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
        None,
    )
}
pub(super) fn load_for_history<'capture>(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'capture>,
    latest: i64,
) -> Result<HistoryObjects<'capture>, String> {
    // 不接受外部旧 index/memo。先全核本事务原件，成功之后才能开当前索引的两个槽。
    let layout = Layout::of_library(tx).map_err(err)?;
    let capture = s.native_projection_capture();
    let mut events = None;
    let entries = load_in_store(
        tx,
        &mut super::audit_store::StoreView::for_audit(s),
        latest,
        capture,
        capture.map(|_| &mut events),
    )?;
    let native = if let Some(capture) = capture {
        let mut wires = sc::CodecVec::default();
        s.native_codec_room(&mut wires, entries.len())
            .map_err(err)?;
        for _ in 0..entries.len() {
            wires.try_push([None, None]).map_err(err)?;
        }
        Some(NativeProjection {
            capture,
            wires,
            sweep: events.map(ProjectionSweep::from_events),
            #[cfg(feature = "s_incremental")]
            retained: Default::default(),
        })
    } else {
        None
    };
    Ok(HistoryObjects {
        entries,
        native,
        layout,
    })
}
/// 原始 SQL 行解码的唯一谓词；回读新增或死亡更新行仍须调用此处及原件绑定核。
fn decode_persisted_entry(
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    s: &mut (impl CodecStore + ?Sized),
    latest: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<Entry, PublishError> {
    let kind = TextValue::from_sql_admitted(row, 2, allocation, s.codec_work()).map_err(publish_err)?;
    let fact_key =
        TextValue::optional_sql_admitted(row, 3, allocation, s.codec_work()).map_err(publish_err)?;
    let coords = [
        row.optional_integer(4).map_err(publish_err)?,
        row.optional_integer(5).map_err(publish_err)?,
        row.optional_integer(6).map_err(publish_err)?,
    ];
    let slot = if kind == SHAPE {
        if fact_key.is_some() {
            return Err(publish_err("形态不得带typed槽位"));
        }
        Slot::Shape([
            coords[0].ok_or_else(|| publish_err("缺窗口"))?,
            coords[1].ok_or_else(|| publish_err("缺窗口"))?,
            coords[2].ok_or_else(|| publish_err("缺窗口"))?,
        ])
    } else {
        if coords != [None, None, None] {
            return Err(publish_err("typed不得带形态窗口"));
        }
        Slot::Typed {
            kind: kind.try_clone_admitted(s.codec_work())?,
            fact_key: fact_key.ok_or_else(|| publish_err("缺typed槽位"))?,
        }
    };
    let e = Entry {
        id: TextValue::from_sql_admitted(row, 0, allocation, s.codec_work()).map_err(publish_err)?,
        revision: row.integer(1).map_err(publish_err)?,
        kind,
        slot,
        generated: root_header_bytes(s, row.blob(7).map_err(publish_err)?)?,
        internal: root_header_bytes(s, row.blob(8).map_err(publish_err)?)?,
        immutable: root_header_bytes(s, row.blob(9).map_err(publish_err)?)?,
        birth_wire: root_ref_bytes(s, row.blob(10).map_err(publish_err)?)?,
        life: LifeOwner {
            batch: BatchOwner::Existing(
                TextValue::from_sql_admitted(row, 11, allocation, s.codec_work()).map_err(publish_err)?,
            ),
            first_known_generation: row.integer(12).map_err(publish_err)?,
            first_known_cut: TextValue::from_sql_admitted(row, 13, allocation, s.codec_work())
                .map_err(publish_err)?,
            published_generation: row.integer(14).map_err(publish_err)?,
            withdrawn_generation: row.optional_integer(15).map_err(publish_err)?,
            withdrawal_reason: TextValue::optional_sql_admitted(
                row,
                16,
                allocation,
                s.codec_work(),
            )
            .map_err(publish_err)?,
            superseded_by: TextValue::optional_sql_admitted(row, 17, allocation, s.codec_work())
                .map_err(publish_err)?,
        },
    };
    if e.life.first_known_generation > latest
        || e.life.withdrawn_generation.is_some_and(|d| d > latest)
    {
        return Err(publish_err("出生代超过已发布代"));
    }
    Ok(e)
}

fn load_in_store(
    tx: &Transaction<'_>,
    s: &mut super::audit_store::StoreView<'_, '_>,
    latest: i64,
    capture: Option<&sc::AuditCapture>,
    events: Option<&mut Option<NativeArc<HistoryEvents>>>,
) -> Result<sc::CodecVec<Entry>, String> {
    let mut leaf_bindings = capture.map(|capture| ShapeLeafBindings {
        capture,
        successful: bj::new_map(None),
    });
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT object_id,object_revision,kind,fact_key,window_start,window_mid,window_end,generated_root,internal_root,immutable_root,birth_wire_root,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects ORDER BY rowid").map_err(err)?;
    let allocation = s.allocation_budget();
    let mut out = bj::new_work(allocation.as_ref());
    while let Some(mut row) = stmt.next().map_err(err)? {
        let e = decode_persisted_entry(&mut row, s, latest, allocation.as_ref())?;
        s.native_codec_room(&mut out, 1).map_err(err)?;
        out.try_push(e).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    // 含已撤回身份的完整表；只克隆身份键，不复制正文。Owned 路径沿既有分配器。
    let mut by_id = bj::new_map(allocation.as_ref());
    for (ordinal, e) in out.iter().enumerate() {
        #[cfg(test)]
        LOAD_ID_VISITS.with(|n| n.set(n.get() + 1));
        s.native_map_room(&mut by_id).map_err(err)?;
        if by_id
            .insert(e.id.try_clone_admitted(s.codec_work())?, ordinal)
            .map_err(err)?
            .is_some()
        {
            return Err(err("独立对象索引重复身份"));
        }
    }
    #[cfg(test)]
    LOAD_ID_CAPACITY.with(|n| n.set(by_id.capacity()));
    // 一次调用复用codec既有的上下文记忆；每根声明仍核，所有来源/生命周期绑定仍逐行执行。
    #[cfg(test)]
    super::state::trace_event("object_index_closures", None);
    let allocation = s.allocation_budget();
    let mut refs = bj::new_work(allocation.as_ref());
    for e in &out {
        for b in [&e.generated, &e.internal, &e.immutable] {
            s.native_codec_room(&mut refs, 1).map_err(err)?;
            refs.try_push(built_ref(s, b)?).map_err(err)?;
        }
        s.native_codec_room(&mut refs, 1).map_err(err)?;
        refs.try_push(s.native_ref_clone(&e.birth_wire).map_err(err)?)
            .map_err(err)?;
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
        validate_withdrawal_target(e, &out, &by_id)?;
    }
    #[cfg(test)]
    super::state::trace_event("object_index_done", None);
    if let Some(events) = events {
        // 仅 Native 无分配预算入口保有 std 有序事件表；失败不转普通路径。
        let value = HistoryEvents::from_ids_admitted(&out, by_id, s)?;
        *events = Some(value.into_arc_admitted(s)?);
    }

    Ok(out)
}
fn validate_withdrawal_target(
    e: &Entry,
    index: &[Entry],
    by_id: &bj::BusinessMap<TextValue, usize>,
) -> Result<(), String> {
    if let Some(death) = e.life.withdrawn_generation {
        match (
            e.life.withdrawal_reason.as_deref(),
            e.life.superseded_by.as_deref(),
        ) {
            (Some("superseded_by_revision"), Some(id)) => {
                #[cfg(test)]
                LOAD_TARGET_LOOKUPS.with(|n| n.set(n.get() + 1));
                let target = &index[*by_id.get(id).ok_or_else(|| err("替代目标不存在"))?];
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
    Ok(())
}

#[cfg(feature = "s_incremental")]
pub(super) fn update_from_owner(
    tx: &Transaction<'_>,
    s: &mut super::history::HashReader<'_>,
    latest: i64,
    index: &mut sc::CodecVec<Entry>,
    sweep: &mut HistorySweep,
    rowids: &[i64],
    _bound: &super::audit_owner::BoundOriginals,
    retained: &mut super::audit_retention::RetainedDelta,
) -> Result<(), PublishError> {
    let mut changed = sc::CodecVec::default();
    s.native_codec_room(&mut changed, rowids.len())
        .map_err(publish_err)?;
    let mut leaf_bindings = s
        .native_projection_capture()
        .map(|capture| ShapeLeafBindings {
            capture,
            successful: bj::new_map(None),
        });
    let mut store = super::audit_store::StoreView::for_audit(s);
    let events =
        NativeArc::get_mut(&mut sweep.events).ok_or_else(|| publish_err("对象事件仍被旧 facade 借用"))?;
    for rowid in rowids {
        let mut q = CheckedStatement::prepare(tx,c"SELECT object_id,object_revision,kind,fact_key,window_start,window_mid,window_end,generated_root,internal_root,immutable_root,birth_wire_root,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects WHERE rowid=?1").map_err(publish_err)?;
        q.bind_integer(1, *rowid).map_err(publish_err)?;
        let mut row = q
            .next()
            .map_err(publish_err)?
            .ok_or_else(|| publish_err("缺对象实际效果行"))?;
        let e = decode_persisted_entry(&mut row, &mut store, latest, None)?;
        drop(row);
        q.finalize().map_err(publish_err)?;
        let refs = [
            built_ref(&store, &e.generated)?,
            built_ref(&store, &e.internal)?,
            built_ref(&store, &e.immutable)?,
            store.native_ref_clone(&e.birth_wire).map_err(publish_err)?,
        ];
        store
            .verify_many_ref(refs.iter().map(RefValue::view), sc::Mode::Public)
            .map_err(publish_err)?;
        bind_originals(&mut store, &e, &mut leaf_bindings)?;
        wire_at(&mut store, &e, latest)?;
        let ordinal = match events.by_id.get(e.id.as_str()) {
            Some(i) => {
                let i = *i;
                retained.release(&index[i]);
                retained.add(&e);
                index[i] = e;
                i
            }
            None => {
                let i = index.len();
                let id = store.native_text(e.id.as_str())?;
                retained.add(&id);
                let capacity = events.by_id.capacity();
                store.native_map_room(&mut events.by_id).map_err(publish_err)?;
                events.by_id.insert(id, i).map_err(publish_err)?;
                retained.hash_capacity::<TextValue, usize>(capacity, events.by_id.capacity());
                let rank = order_key(&e, false).owned_admitted(&store)?;
                retained.add(&rank);
                let capacity = events.ranks.capacity();
                store.native_codec_room(&mut events.ranks, 1).map_err(publish_err)?;
                events.ranks.try_push(rank).map_err(publish_err)?;
                retained.vec_capacity::<NativeRank>(capacity, events.ranks.capacity());
                let tree_before = events.births.allocation_bytes();
                let bucket =
                    event_bucket(&mut events.births, e.life.first_known_generation, &store)?;
                let capacity = bucket.capacity();
                store.native_codec_room(bucket, 1).map_err(publish_err)?;
                bucket.try_push(i).map_err(publish_err)?;
                retained.vec_capacity::<usize>(capacity, bucket.capacity());
                retained.add_bytes(events.births.allocation_bytes().saturating_sub(tree_before));
                retained.add(&e);
                let capacity = index.capacity();
                store.native_codec_room(index, 1).map_err(publish_err)?;
                index.try_push(e).map_err(publish_err)?;
                retained.vec_capacity::<Entry>(capacity, index.capacity());
                i
            }
        };
        if let Some(death) = index[ordinal].life.withdrawn_generation {
            let tree_before = events.deaths.allocation_bytes();
            let ids = event_bucket(&mut events.deaths, death, &store)?;
            let capacity = ids.capacity();
            let at = ids.partition_point(|other| {
                (&events.ranks[*other], *other) < (&events.ranks[ordinal], ordinal)
            });
            if ids.get(at) != Some(&ordinal) {
                store.native_codec_room(ids, 1).map_err(publish_err)?;
                ids.try_push(ordinal).map_err(publish_err)?;
                ids[at..].rotate_right(1);
            }
            retained.vec_capacity::<usize>(capacity, ids.capacity());
            retained.add_bytes(events.deaths.allocation_bytes().saturating_sub(tree_before));
        }
        changed.try_push(ordinal).map_err(publish_err)?;
    }
    for ordinal in changed {
        validate_withdrawal_target(&index[ordinal], index, &events.by_id)?;
    }
    Ok(())
}
#[cfg(feature = "s_incremental")]
pub(super) fn restore_for_owner<'a>(
    entries: sc::CodecVec<Entry>,
    layout: Layout,
    s: &super::history::HashReader<'a>,
    sweep: &mut HistorySweep,
    _bound: &super::audit_owner::BoundOriginals,
    retained: &mut super::audit_retention::RetainedDelta,
) -> Result<HistoryObjects<'a>, super::audit_owner::native::Failure<PublishError>> {
    let capture = s
        .native_projection_capture()
        .ok_or_else(|| publish_err("对象恢复缺 Native owner"))?;
    let saved = sweep.projection.take();
    let (mut wires, projection) = match saved {
        Some(saved) => (
            saved.wires,
            ProjectionSweep {
                events: sweep.events.clone(),
                active: saved.active,
                withdrawn: saved.withdrawn,
                previous_cut: saved.previous_cut,
                retained: Default::default(),
                #[cfg(test)]
                visits: saved.visits,
            },
        ),
        None => (
            sc::CodecVec::default(),
            ProjectionSweep::from_events(sweep.events.clone()),
        ),
    };
    // 仅新增身份取得两个空memo槽；旧wire、游标及活动/撤回集合原样移动。
    let capacity = wires.capacity();
    let additional = entries
        .len()
        .checked_sub(wires.len())
        .ok_or_else(|| publish_err("owner wire 槽数超过对象数"))?;
    s.native_codec_room(&mut wires, additional).map_err(publish_err)?;
    for _ in 0..additional {
        wires.try_push([None, None]).map_err(publish_err)?;
    }
    retained.vec_capacity::<[Option<RefValue>; 2]>(capacity, wires.capacity());
    let native = Some(NativeProjection {
        capture,
        wires,
        sweep: Some(projection),
        retained: Default::default(),
    });
    Ok(HistoryObjects {
        entries,
        native,
        layout,
    })
}

/// 内部 prior 只来自上代活动CC010/CC056；调用者保持本Vec活到generate结束。
pub(super) fn prior_records(index: &[Entry]) -> Result<(Vec<Built>, Option<Built>), PublishError> {
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
                return Err(publish_err("活动版本对象不唯一"));
            }
        }
    }
    Ok((bi, version))
}
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
    ).map_err(Into::into)
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
    for e in &p.births {
        if e.life.first_known_generation != p.generation || e.life.batch != BatchOwner::CurrentBatch
        {
            return Err(publish_err("新生上下文不符"));
        }
        #[cfg(feature = "s_incremental")]
        let construction_stage = super::audit_owner::begin_construction_stage(
            tx,
            super::audit_owner::StageKind::ObjectBirth,
        )?;
        let wire = s
            .replace_ref(e.birth_wire.view(), &[f("batch_id")], batch)
            .map_err(publish_err)?;
        s.verify_many_ref_fresh([wire.view()], sc::Mode::Public)
            .map_err(publish_err)?;
        let (fact, w) = match &e.slot {
            Slot::Shape(w) => (None, Some(*w)),
            Slot::Typed { fact_key, .. } => (Some(fact_key.as_str()), None),
        };
        tx.execute("INSERT INTO s_content_objects VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?13,NULL,NULL,NULL)",params![e.id,e.revision,e.kind,fact,w.map(|v|v[0]),w.map(|v|v[1]),w.map(|v|v[2]),e.generated.digest.as_slice(),e.internal.digest.as_slice(),e.immutable.digest.as_slice(),wire.digest().map_err(publish_err)?.as_slice(),batch,p.generation,format!("cut-{}",p.generation)]).map_err(publish_err)?;
        #[cfg(feature = "s_incremental")]
        construction_stage.finish(material)?;
    }
    for w in bj::RawEventsView::from(&p.withdrawals) {
        #[cfg(feature = "s_incremental")]
        let construction_stage = super::audit_owner::begin_construction_stage(
            tx,
            super::audit_owner::StageKind::ObjectWithdrawal,
        )?;
        let changed=tx.execute("UPDATE s_content_objects SET withdrawn_generation=?1,withdrawal_reason=?2,superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",params![p.generation,bj::field(w, "reason").as_str(),bj::field(w, "superseded_by").as_str(),bj::field(w, "object_id").as_str()]).map_err(publish_err)?;
        if changed != 1 {
            return Err(publish_err("撤回身份缺失或已撤回"));
        }
        #[cfg(feature = "s_incremental")]
        construction_stage.finish(material)?;
    }
    Ok(())
}

/// 两种存储共用字段投影；Owned数组直接复制借用字段，不经过普通JSON所有者。
fn change_array<'work>(
    s: &'work (impl CodecRead + ?Sized),
    allocation: Option<&OwnedBudget>,
) -> Result<bj::ScopedJsonArray<'work>, PublishError> {
    match (s.codec_work(), allocation) {
        (Some(_), Some(_)) => Err(publish_err("Native变化JSON不能同时接Owned父账")),
        (Some(work), None) => bj::ScopedJsonArray::native(work),
        (None, Some(allocation)) => bj::ScopedJsonArray::owned(allocation),
        (None, None) => Ok(bj::ScopedJsonArray::unscoped()),
    }
}
fn push_change(
    _s: &(impl CodecRead + ?Sized),
    array: &mut bj::ScopedJsonArray<'_>,
    view: aj::JsonView<'_>,
) -> Result<(), PublishError> {
    array.push(view)
}
fn push_withdrawal(
    s: &(impl CodecRead + ?Sized),
    array: &mut bj::ScopedJsonArray<'_>,
    e: &Entry,
    reason: &str,
    superseded_by: Option<&str>,
) -> Result<(), PublishError> {
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
            push_change(s, array, J::object_fields(&fields).map_err(publish_err)?)
        }
        Slot::Typed { fact_key, .. } => {
            let fields = [
                ("object_id", J::String(e.id.as_str())),
                ("fact_key", J::String(fact_key.as_str())),
                ("reason", J::String(reason)),
                ("superseded_by", sup),
            ];
            push_change(s, array, J::object_fields(&fields).map_err(publish_err)?)
        }
    }
}
fn push_replace(
    s: &(impl CodecRead + ?Sized),
    array: &mut bj::ScopedJsonArray<'_>,
    old_id: &str,
    new_id: &str,
) -> Result<(), PublishError> {
    let fields = [
        ("new_object_id", aj::JsonView::String(new_id)),
        ("old_object_id", aj::JsonView::String(old_id)),
    ];
    push_change(s, array, aj::JsonView::object_fields(&fields).map_err(publish_err)?)
}
/// 独立生命周期导出的本代变化集合，顺序沿上一代活动主序。
/// 旧接口仅消费同一核心的普通数组，Owned所有者不能擦除为普通Vec。
#[cfg(test)]
pub(super) fn changes_at(index: &[Entry], g: i64) -> Result<(Vec<Value>, Vec<Value>), String> {
    let (withdrawals, replaces) = changes_at_budgeted(index, g, None)?;
    let plain = |owner| match owner {
        bj::JsonOwner::Plain(rows) if !rows.is_native_admitted() => rows
            .as_array()
            .map(|rows| rows.to_vec())
            .ok_or_else(|| err("变化JSON不是普通数组")),
        _ => Err(err("变化JSON不是普通数组")),
    };
    Ok((plain(withdrawals)?, plain(replaces)?))
}
/// dead排序工作集和真实返回JSON所有者持续使用调用方实际OwnedBudget。
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
pub(super) fn changes_at_budgeted(
    index: &[Entry],
    g: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<(bj::JsonOwner, bj::JsonOwner), String> {
    changes_at_admitted(
        &super::audit_store::NativeAdmission(None),
        index,
        g,
        allocation,
    )
}
/// 当前候选域入口；旧接口只用于无 scope 的调用。
pub(super) fn changes_at_admitted(
    s: &(impl CodecRead + ?Sized),
    index: &[Entry],
    g: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<(bj::JsonOwner, bj::JsonOwner), PublishError> {
    let mut dead = bj::new_work(allocation);
    for (ordinal, e) in index
        .iter()
        .enumerate()
        .filter(|(_, e)| e.life.withdrawn_generation == Some(g))
    {
        s.native_codec_room(&mut dead, 1).map_err(publish_err)?;
        dead.try_push((ordinal, e)).map_err(publish_err)?;
    }
    // 同一主序加原始序号，保留stable排序的平局行为；unstable排序不另分配std工作集。
    dead.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
    let mut withdrawals = change_array(s, allocation)?;
    let mut replaces = change_array(s, allocation)?;
    for (_, e) in dead {
        let reason = e
            .life
            .withdrawal_reason
            .as_deref()
            .ok_or_else(|| publish_err("撤回缺原因"))?;
        push_withdrawal(
            s,
            &mut withdrawals,
            e,
            reason,
            e.life.superseded_by.as_deref(),
        )?;
        if let Some(id) = &e.life.superseded_by {
            push_replace(s, &mut replaces, e.id.as_str(), id.as_str())?;
        }
    }
    Ok((withdrawals.finish(), replaces.finish()))
}
/// 当前已核Capture的只读事件表；两游标共享，普通/Owned入口不创建它。
struct HistoryEvents {
    by_id: bj::BusinessMap<TextValue, usize>,
    births: NativeTree<i64, sc::CodecVec<usize>>,
    deaths: NativeTree<i64, sc::CodecVec<usize>>,
    /// layout/1 活动原键：before-g 成员/diff 游标与死亡桶（changes）次序只用它。
    ranks: sc::CodecVec<NativeRank>,
    #[cfg(test)]
    initial_entries: usize,
    #[cfg(test)]
    sort_passes: usize,
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
)]
pub(super) fn take_load_lookup_counts() -> (usize, usize) {
    (
        LOAD_ID_VISITS.with(|n| n.replace(0)),
        LOAD_TARGET_LOOKUPS.with(|n| n.replace(0)),
    )
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
)]
pub(super) fn load_id_capacity() -> usize {
    LOAD_ID_CAPACITY.with(|n| n.get())
}
impl HistoryEvents {
    fn new_admitted(index: &[Entry], store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        let mut by_id = bj::new_map(None);
        for (i, e) in index.iter().enumerate() {
            store.native_map_room(&mut by_id).map_err(publish_err)?;
            assert!(by_id
                .insert(e.id.try_clone_admitted(store.codec_work())?, i)
                .map_err(publish_err)?
                .is_none());
        }
        Self::from_ids_admitted(index, by_id, store)
    }
    fn into_arc_admitted(
        self,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<NativeArc<Self>, PublishError> {
        // 动态字段已分别持许可；这里只签共享块及lease元数据，clone不重复签费。
        NativeArc::try_new_shallow(self, store.codec_work()).map_err(publish_err)
    }
    fn from_ids_admitted(
        index: &[Entry],
        by_id: bj::BusinessMap<TextValue, usize>,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<Self, PublishError> {
        let mut ranks = sc::CodecVec::default();
        store
            .native_codec_room(&mut ranks, index.len())
            .map_err(publish_err)?;
        for e in index {
            ranks
                .try_push(order_key(e, false).owned_admitted(store)?)
                .map_err(publish_err)?;
        }
        let mut out = Self {
            by_id,
            births: Default::default(),
            deaths: Default::default(),
            ranks,
            #[cfg(test)]
            initial_entries: 0,
            #[cfg(test)]
            sort_passes: 0,
        };
        let mut order = sc::CodecVec::default();
        store
            .native_codec_room(&mut order, index.len())
            .map_err(publish_err)?;
        for i in 0..index.len() {
            order.try_push(i).map_err(publish_err)?;
        }
        #[cfg(test)]
        {
            out.sort_passes += 1;
        }
        order.sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], false).then(a.cmp(b)));
        for (i, e) in index.iter().enumerate() {
            #[cfg(test)]
            {
                out.initial_entries += 1;
            }
            let g = e.life.first_known_generation;
            let bucket = event_bucket(&mut out.births, g, store)?;
            store.native_codec_room(bucket, 1).map_err(publish_err)?;
            bucket.try_push(i).map_err(publish_err)?;
        }
        // 本代changes沿旧活动序，不能换成累计withdrawn的主序。
        for i in &order {
            if let Some(g) = index[*i].life.withdrawn_generation {
                let bucket = event_bucket(&mut out.deaths, g, store)?;
                store.native_codec_room(bucket, 1).map_err(publish_err)?;
                bucket.try_push(*i).map_err(publish_err)?;
            }
        }
        #[cfg(test)]
        {
            out.sort_passes += 1;
        }
        order.sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], true).then(a.cmp(b)));
        Ok(out)
    }
}
fn event_bucket<'a>(
    table: &'a mut NativeTree<i64, sc::CodecVec<usize>>,
    g: i64,
    store: &(impl CodecRead + ?Sized),
) -> Result<&'a mut sc::CodecVec<usize>, PublishError> {
    if !table.contains_key(&g) {
        table
            .insert_admitted(g, sc::CodecVec::default(), store)
            .map_err(publish_err)?;
    }
    Ok(table.get_mut(&g).expect("刚建立的事件桶"))
}
type NativeRank = OrderKey<NativeValue<String>>;
/// #1491：RankSet 键＝（代际前缀，layout/1 原键）。投影游标的活动/撤回集经 [`native_rank`]
/// 取库布局的公开主序；before-g 成员/diff 游标经 [`diff_rank`] 沿 layout/1 原键（前缀恒 0），
/// 与写者 diff 及 changes 同序。
type PublicRank = (i64, NativeRank);
type RankSet = NativeTree<(PublicRank, usize), ()>;
fn native_rank(
    layout: Layout,
    e: &Entry,
    dead: bool,
    store: &(impl CodecRead + ?Sized),
) -> Result<PublicRank, String> {
    let (epoch, key) = public_rank(layout, e, dead)?;
    Ok((epoch, key.owned_admitted(store)?))
}
fn diff_rank(rank: &NativeRank, store: &(impl CodecRead + ?Sized)) -> Result<PublicRank, PublishError> {
    Ok((0, rank.try_clone_admitted(store)?))
}
fn insert_rank(
    set: &mut RankSet,
    rank: PublicRank,
    ordinal: usize,
    store: &(impl CodecRead + ?Sized),
    #[cfg(feature = "s_incremental")] retained: &mut super::audit_retention::RetainedDelta,
) -> Result<(), PublishError> {
    let key = (rank, ordinal);
    #[cfg(feature = "s_incremental")]
    let (before, fresh) = (set.allocation_bytes(), !set.contains_key(&key));
    #[cfg(feature = "s_incremental")]
    let bytes = {
        use super::audit_retention::Retained;
        key.heap_bytes()
    };
    set.insert_admitted(key, (), store).map_err(publish_err)?;
    #[cfg(feature = "s_incremental")]
    {
        if fresh {
            retained.add_bytes(bytes);
        }
        retained.add_bytes(set.allocation_bytes().saturating_sub(before));
    }
    Ok(())
}
fn remove_rank(
    set: &mut RankSet,
    rank: PublicRank,
    ordinal: usize,
    #[cfg(feature = "s_incremental")] retained: &mut super::audit_retention::RetainedDelta,
) -> Result<(), PublishError> {
    let key = (rank, ordinal);
    #[cfg(feature = "s_incremental")]
    let before = set.allocation_bytes();
    // #1491 独审 LOW-1：出集键由当场生命期取得；键缺失说明入集与出集取键不一致（如撤回代被
    // 改写），留下陈旧成员会静默改变公开数组，故 fail-closed。
    let Some((old, ())) = set.remove_entry(&key) else {
        return Err(publish_err(format!(
            "RankSet出集键不存在（序号 {ordinal}）：入集与出集取键不一致"
        )));
    };
    #[cfg(feature = "s_incremental")]
    retained.release(&old);
    drop(old);
    if set.is_empty() {
        set.clear();
    }
    #[cfg(feature = "s_incremental")]
    retained.release_bytes(before.saturating_sub(set.allocation_bytes()));
    Ok(())
}
/// before-g游标保留原成员/变化审计入口和边界；不消费through-g状态。
pub(super) struct HistorySweep {
    events: NativeArc<HistoryEvents>,
    active: RankSet,
    previous: i64,
    #[cfg(feature = "s_incremental")]
    projection: Option<OwnerProjection>,
    #[cfg(feature = "s_incremental")]
    retained: super::audit_retention::RetainedDelta,
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
    // #1488 history::audit_one_published_cut 的失败直接传播到按值拥有全部游标的
    // read_indexed_history_inner / append_owner_cut；失败游标必须随候选销毁，不得重试。
    // 此函数不承诺可在预算拒绝后继续使用同一 &mut self。
    pub(super) fn advance_admitted(
        &mut self,
        g: i64,
        store: &(impl CodecRead + ?Sized),
    ) -> Result<(), PublishError> {
        if g <= self.previous {
            return Err(publish_err("生命周期 sweep 代号不递增"));
        }
        for (_, ids) in self.events.births.range(self.previous..g) {
            for i in ids {
                insert_rank(
                    &mut self.active,
                    diff_rank(&self.events.ranks[*i], store)?,
                    *i,
                    store,
                    #[cfg(feature = "s_incremental")]
                    &mut self.retained,
                )?;
                #[cfg(test)]
                {
                    self.visits.births += 1;
                }
            }
        }
        for (_, ids) in self.events.deaths.range(self.previous..g) {
            for i in ids {
                remove_rank(
                    &mut self.active,
                    diff_rank(&self.events.ranks[*i], store)?,
                    *i,
                    #[cfg(feature = "s_incremental")]
                    &mut self.retained,
                )?;
                #[cfg(test)]
                {
                    self.visits.deaths += 1;
                }
            }
        }
        self.previous = g;
        Ok(())
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn take_retained_delta(&mut self) -> super::audit_retention::RetainedDelta {
        std::mem::take(&mut self.retained)
    }

    #[cfg(test)]
    fn new(index: &[Entry]) -> Self {
        Self::new_admitted(index, &super::audit_store::NativeAdmission(None))
            .expect("无scope的Native投影构造")
    }
    #[cfg(test)]
    fn new_admitted(index: &[Entry], store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self::from_events(
            HistoryEvents::new_admitted(index, store)?.into_arc_admitted(store)?,
        ))
    }
    fn from_events(events: NativeArc<HistoryEvents>) -> Self {
        Self {
            #[cfg(test)]
            visits: SweepVisits {
                initial_entries: events.initial_entries,
                ..Default::default()
            },
            events,
            active: Default::default(),
            previous: 0,
            #[cfg(feature = "s_incremental")]
            projection: None,
            #[cfg(feature = "s_incremental")]
            retained: Default::default(),
        }
    }
    #[cfg(all(test, feature = "s_incremental"))]
    pub(super) fn assert_owner_projection_oracle(&self, latest: i64) {
        if let Some(p) = &self.projection {
            assert_eq!(p.previous_cut, (latest > 0).then_some(latest));
            assert_eq!(p.wires.len(), self.events.ranks.len());
            assert_eq!(
                p.visits.births,
                self.events
                    .births
                    .range(..=latest)
                    .map(|(_, v)| v.len())
                    .sum::<usize>(),
                "owner不得重放旧birth"
            );
            assert_eq!(
                p.visits.deaths,
                self.events
                    .deaths
                    .range(..=latest)
                    .map(|(_, v)| v.len())
                    .sum::<usize>(),
                "owner不得重放旧death"
            );
        } else {
            assert_eq!(latest, 0, "已发布owner必须保留投影游标");
        }
    }
    fn active_ordinals(&self) -> impl Iterator<Item = usize> + '_ {
        self.active.keys().map(|(_, i)| {
            #[cfg(test)]
            self.visits.active.set(self.visits.active.get() + 1);
            *i
        })
    }
    #[cfg(test)]
    fn advance(&mut self, g: i64) -> Result<(), String> {
        self.advance_admitted(g, &super::audit_store::NativeAdmission(None))
    }
    #[cfg(test)]
    pub(super) fn changes(
        &self,
        index: &[Entry],
        g: i64,
    ) -> Result<(bj::JsonOwner, bj::JsonOwner), String> {
        self.changes_admitted(&super::audit_store::NativeAdmission(None), index, g)
    }
    pub(super) fn changes_admitted(
        &self,
        s: &(impl CodecRead + ?Sized),
        index: &[Entry],
        g: i64,
    ) -> Result<(bj::JsonOwner, bj::JsonOwner), PublishError> {
        let mut withdrawals = change_array(s, None)?;
        let mut replaces = change_array(s, None)?;
        for i in self.events.deaths.get(&g).into_iter().flatten() {
            let e = &index[*i];
            let reason = e
                .life
                .withdrawal_reason
                .as_deref()
                .ok_or_else(|| publish_err("撤回缺原因"))?;
            push_withdrawal(
                s,
                &mut withdrawals,
                e,
                reason,
                e.life.superseded_by.as_deref(),
            )?;
            if let Some(id) = &e.life.superseded_by {
                push_replace(s, &mut replaces, e.id.as_str(), id.as_str())?;
            }
        }
        Ok((withdrawals.finish(), replaces.finish()))
    }
}
#[cfg(feature = "s_incremental")]
struct OwnerProjection {
    wires: sc::CodecVec<[Option<RefValue>; 2]>,
    active: RankSet,
    withdrawn: RankSet,
    previous_cut: Option<i64>,
    #[cfg(test)]
    visits: ProjectionVisits,
}
/// through-g双投影游标；任意合法cut可重复、跳跃、回退，不改准入判据。
struct ProjectionSweep {
    events: NativeArc<HistoryEvents>,
    active: RankSet,
    withdrawn: RankSet,
    previous_cut: Option<i64>,
    #[cfg(feature = "s_incremental")]
    retained: super::audit_retention::RetainedDelta,
    #[cfg(test)]
    visits: ProjectionVisits,
}
#[cfg(test)]
#[derive(Default)]
struct ProjectionVisits {
    births: usize,
    deaths: usize,
    output_entries: usize,
}
impl ProjectionSweep {
    // #1488 唯一生产调用经 native_dual_at_cut → history 私有单代核；
    // 任一步失败由 cold/append 按值候选边界销毁本游标，只有完整成功才能重新封装。
    /// 活动/撤回集按库布局的公开主序键（[`native_rank`]）入出；键由 `index` 当场取，
    /// 撤回键随死亡代而定，不预存会因 owner 增量改写撤回代而陈旧的副本。
    fn seek_admitted(
        &mut self,
        g: i64,
        layout: Layout,
        index: &[Entry],
        store: &(impl CodecRead + ?Sized),
    ) -> Result<(), PublishError> {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        if self.previous_cut == Some(g) {
            return Ok(());
        }
        if self.previous_cut.is_none_or(|previous| previous < g) {
            let lower = self.previous_cut.map_or(Unbounded, Excluded);
            for (_, ids) in self.events.births.range((lower, Included(g))) {
                for i in ids {
                    insert_rank(
                        &mut self.active,
                        native_rank(layout, &index[*i], false, store)?,
                        *i,
                        store,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    #[cfg(test)]
                    {
                        self.visits.births += 1;
                    }
                }
            }
            for (_, ids) in self.events.deaths.range((lower, Included(g))) {
                for i in ids {
                    remove_rank(
                        &mut self.active,
                        native_rank(layout, &index[*i], false, store)?,
                        *i,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    insert_rank(
                        &mut self.withdrawn,
                        native_rank(layout, &index[*i], true, store)?,
                        *i,
                        store,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    #[cfg(test)]
                    {
                        self.visits.deaths += 1;
                    }
                }
            }
        } else {
            let previous = self.previous_cut.unwrap();
            // 反向仍先撤death再撤birth，保持原cut成员/排序语义。
            for (_, ids) in self.events.deaths.range((Excluded(g), Included(previous))) {
                for i in ids {
                    remove_rank(
                        &mut self.withdrawn,
                        native_rank(layout, &index[*i], true, store)?,
                        *i,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    insert_rank(
                        &mut self.active,
                        native_rank(layout, &index[*i], false, store)?,
                        *i,
                        store,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    #[cfg(test)]
                    {
                        self.visits.deaths += 1;
                    }
                }
            }
            for (_, ids) in self.events.births.range((Excluded(g), Included(previous))) {
                for i in ids {
                    remove_rank(
                        &mut self.active,
                        native_rank(layout, &index[*i], false, store)?,
                        *i,
                        #[cfg(feature = "s_incremental")]
                        &mut self.retained,
                    )?;
                    #[cfg(test)]
                    {
                        self.visits.births += 1;
                    }
                }
            }
        }
        self.previous_cut = Some(g);
        Ok(())
    }
    #[cfg(test)]
    #[cfg_attr(
        all(test, not(feature = "s_incremental")),
        allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
    )]
    fn new(index: &[Entry]) -> Self {
        Self::new_admitted(index, &super::audit_store::NativeAdmission(None))
            .expect("无scope的Native投影构造")
    }
    fn new_admitted(index: &[Entry], store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(Self::from_events(
            HistoryEvents::new_admitted(index, store)?.into_arc_admitted(store)?,
        ))
    }
    fn from_events(events: NativeArc<HistoryEvents>) -> Self {
        Self {
            events,
            active: Default::default(),
            withdrawn: Default::default(),
            previous_cut: None,
            #[cfg(feature = "s_incremental")]
            retained: Default::default(),
            #[cfg(test)]
            visits: ProjectionVisits::default(),
        }
    }
    #[cfg(test)]
    #[cfg_attr(
        all(test, not(feature = "s_incremental")),
        allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
    )]
    fn seek(&mut self, g: i64, layout: Layout, index: &[Entry]) {
        self.seek_admitted(g, layout, index, &super::audit_store::NativeAdmission(None))
            .expect("无scope测试游标");
    }
}
/// 按该代真实候选序复用同一diff判据，再对拍独立生命周期；不假定候选槽位唯一。
#[cfg(test)]
pub(super) fn verify_generated_members<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
) -> Result<(), String> {
    verify_generated_members_swept(s, index, state, g, None)
}
#[cfg(test)]
pub(super) fn verify_generated_members_swept<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
    sweep: Option<&HistorySweep>,
) -> Result<(), String> {
    verify_generated_members_swept_with_records(s, index, state, g, sweep, None)
}
pub(super) fn verify_generated_members_swept_with_records<'a>(
    s: &mut (impl CodecStore + ?Sized),
    index: &[Entry],
    state: impl Into<sc::RefView<'a>>,
    g: i64,
    sweep: Option<&HistorySweep>,
    mut records: Option<&mut super::history::NativeRecordFields<'_>>,
) -> Result<(), PublishError> {
    let state = state.into();
    let allocation = s.allocation_budget();
    let cut_number = bj::Decimal::new(g);
    let mut by_id = bj::new_map(allocation.as_ref());
    if sweep.is_none() {
        for e in index.iter().filter(|e| e.life.first_known_generation <= g) {
            s.native_map_room(&mut by_id).map_err(publish_err)?;
            by_id.insert(e.id.as_str(), e).map_err(publish_err)?;
        }
    }
    let objects = field_ref(s, state, "objects")?;
    if objects.kind() != Kind::Array {
        return Err(publish_err("生成对象集不是数组"));
    }
    let mut seen = bj::new_map(allocation.as_ref());
    let mut new = bj::new_work(allocation.as_ref());
    let mut cursor = s.array_cursor(objects.view()).map_err(publish_err)?;
    let items = if cursor.is_none() && s.audit_full_array_scan() {
        Some(s.lookup_array_ref(objects.view()).map_err(publish_err)?)
    } else {
        None
    };
    for n in 0..objects.extent().ok_or_else(|| publish_err("生成集缺长度"))? {
        let object = if let Some(cursor) = &mut cursor {
            s.next_array_member(cursor).map_err(publish_err)?
        } else if let Some(items) = &items {
            s.native_ref_clone(&items[n as usize]).map_err(publish_err)?
        } else {
            s.lookup_ref(objects.view(), &[PathStep::Index(n)])
                .map_err(publish_err)?
        };
        let id = text_ref(s, object.view(), "object_id")?;
        s.native_map_room(&mut seen).map_err(publish_err)?;
        if seen
            .insert(s.native_text(id.as_str())?, ())
            .map_err(publish_err)?
            .is_some()
        {
            return Err(publish_err("生成集重复对象身份"));
        }
        let e = match sweep {
            Some(sweep) => sweep
                .events
                .by_id
                .get(id.as_str())
                .map(|i| &index[*i])
                .filter(|e| e.life.first_known_generation <= g),
            None => by_id.get(id.as_str()).copied(),
        }
        .ok_or_else(|| publish_err("生成对象不在独立身份史内"))?;
        let names: &[&str] = if e.slot.is_shape() {
            &object_records::SHAPE_SEALED_FIELDS
        } else {
            &object_records::TYPED_SEALED_FIELDS
        };
        let fields = OwnerFields::read(s, object.view(), names)?;
        if object.kind() != Kind::Object
            || object.extent() != Some(if e.slot.is_shape() { 15 } else { 10 })
            || fields.integer(s, "first_known_generation")? != g
            || fields.integer(s, "published_generation")? != g
            || fields.text(s, "first_known_cut")?.strip_prefix("cut-") != Some(cut_number.text())
        {
            return Err(publish_err("生成对象字段/当代元数据不符"));
        }
        if fields.immutable(s, e.slot.is_shape())? != e.immutable {
            return Err(publish_err("生成集同身份内容与独立原件不同"));
        }
        if e.life.first_known_generation == g && object != built_ref(s, &e.generated)? {
            return Err(publish_err("出生原件不等于真实首次生成记录"));
        }
        // #1394：全部原record检查通过之后，只交接同capture的exact字段材料。
        if let Some(records) = records.as_deref_mut() {
            records
                .seed_measured(object.view(), names, &fields.0, s.native_measure().as_ref())
                .map_err(publish_err)?;
        }
        s.native_codec_room(&mut new, 1).map_err(publish_err)?;
        new.try_push(object_diff::Key {
            object_id: id,
            slot: &e.slot,
            immutable: object_diff::Ready(e.immutable.digest),
        })
        .map_err(publish_err)?;
    }
    let mut before = bj::new_work(allocation.as_ref());
    for (ordinal, e) in index
        .iter()
        .take(if sweep.is_none() { index.len() } else { 0 })
        .enumerate()
        .filter(|(_, e)| e.life.first_known_generation < g)
    {
        s.native_codec_room(&mut before, 1).map_err(publish_err)?;
        before.try_push((ordinal, e)).map_err(publish_err)?;
    }
    before.sort_unstable_by(|a, b| ordering(a.1, b.1, false).then(a.0.cmp(&b.0)));
    let mut old = bj::new_work(allocation.as_ref());
    for (_, e) in &before {
        s.native_codec_room(&mut old, 1).map_err(publish_err)?;
        old.try_push(object_diff::Existing {
            key: object_diff::Key {
                object_id: e.id.as_str(),
                slot: &e.slot,
                immutable: object_diff::Ready(e.immutable.digest),
            },
            active: e.life.withdrawn_generation.is_none_or(|d| d >= g),
        })
        .map_err(publish_err)?;
    }
    let changes = if let Some(sweep) = sweep {
        object_diff::plan_indexed_admitted(
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
                    .events
                    .by_id
                    .get(id)
                    .copied()
                    .filter(|i| index[*i].life.first_known_generation < g)
            },
            |id| {
                sweep.events.by_id.get(id).copied().filter(|i| {
                    index[*i].life.first_known_generation < g
                        && index[*i].life.withdrawn_generation.is_none_or(|d| d >= g)
                })
            },
            None,
            s.codec_work(),
        )?
    } else {
        object_diff::plan_admitted(&new, &old, allocation.as_ref(), s.codec_work())?
    };
    let births_match = match sweep {
        Some(sweep) => changes
            .births
            .iter()
            .map(|i| new[*i].object_id.as_str())
            .eq(sweep
                .events
                .births
                .get(&g)
                .into_iter()
                .flatten()
                .map(|i| index[*i].id.as_str())),
        None => changes
            .births
            .iter()
            .map(|i| new[*i].object_id.as_str())
            .eq(index
                .iter()
                .filter(|e| e.life.first_known_generation == g)
                .map(|e| e.id.as_str())),
    };
    if !births_match {
        return Err(publish_err("真实出生顺序与独立索引不同"));
    }
    let (expected_withdrawals, expected_replaces) = match sweep {
        Some(sweep) => sweep.changes_admitted(s, index, g)?,
        None => changes_at_admitted(s, index, g, allocation.as_ref())?,
    };
    let mut withdrawals = change_array(s, allocation.as_ref())?;
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
                    new[w.new_index.ok_or_else(|| publish_err("替代缺候选"))?]
                        .object_id
                        .as_str(),
                ),
            ),
        };
        push_withdrawal(s, &mut withdrawals, e, reason, sup)?;
    }
    let mut replaces = change_array(s, allocation.as_ref())?;
    for r in changes.replaces {
        push_replace(
            s,
            &mut replaces,
            if sweep.is_some() {
                index[r.old_index].id.as_str()
            } else {
                before[r.old_index].1.id.as_str()
            },
            new[r.new_index].object_id.as_str(),
        )?;
    }
    let withdrawals = withdrawals.finish();
    let replaces = replaces.finish();
    if withdrawals.view() != expected_withdrawals.view()
        || replaces.view() != expected_replaces.view()
    {
        return Err(publish_err("真实撤回/替代与独立索引不同"));
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static LOAD_ID_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static LOAD_TARGET_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static LOAD_ID_CAPACITY: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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

    struct FixedSql<'a, 'c> {
        inner: content_store::TxStore<'a, 'c>,
        budget: Option<OwnedBudget>,
    }
    impl NodeSource for FixedSql<'_, '_> {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.inner.read(d)
        }
    }
    impl NodeSink for FixedSql<'_, '_> {
        fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.inner.put(d, t, b)
        }
    }
    impl CodecRead for FixedSql<'_, '_> {
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            self
        }
        fn allocation_budget(&self) -> Option<OwnedBudget> {
            self.budget.clone()
        }
    }
    impl CodecStore for FixedSql<'_, '_> {
        fn generic_store(&mut self) -> &mut dyn NodeStore {
            self
        }
    }
    fn fixed_candidate_late_trigger(budget: Option<OwnedBudget>) {
        let (nodes, objects) = native_candidate_fixture();
        let o = objects.iter().find(|o| o.kind() != SHAPE).unwrap();
        let mut errors = Vec::new();
        for oracle in [true, false] {
            let mut conn = database();
            let tx = conn.transaction().unwrap();
            let mut store = FixedSql {
                inner: content_store::TxStore::new(&tx).unwrap(),
                budget: budget.clone(),
            };
            for (d, t, b) in nodes.iter().filter(|(_, t, _)| *t != sc::tag::BATCH_SELF) {
                store.put(*d, t, b).unwrap();
            }
            let before = candidate_sql_material_for_test(&tx);
            let generated_hex = o
                .generated()
                .digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            tx.execute_batch(&format!("CREATE TEMP TRIGGER fixed_fields_after_wire AFTER INSERT ON s_content_nodes WHEN NEW.tag=8 BEGIN UPDATE s_content_nodes SET body=x'00' WHERE digest=x'{generated_hex}'; END; SAVEPOINT fixed_fields_try;")).unwrap();
            let result = if oracle {
                with_legacy_candidate_fields_for_test(|| candidate(&mut store, o, 1))
            } else {
                candidate(&mut store, o, 1)
            };
            let error = result.unwrap_err();
            assert!(error.to_string().contains(sc::code::NODE_DIGEST), "{error}");
            assert_ne!(
                candidate_sql_material_for_test(&tx),
                before,
                "必须真实wire写后触发，不是preflight拒绝"
            );
            tx.execute_batch("ROLLBACK TO fixed_fields_try; RELEASE fixed_fields_try; DROP TRIGGER fixed_fields_after_wire;").unwrap();
            assert_eq!(candidate_sql_material_for_test(&tx), before);
            errors.push(error);
            candidate(&mut store, o, 1).unwrap();
            drop(store);
            tx.rollback().unwrap();
        }
        assert_eq!(
            errors[0], errors[1],
            "wire后生成原件损坏仍命中相同晚exact错误"
        );
        if let Some(b) = budget {
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn fixed_candidate_plain_wire_trigger_corrupts_generated_then_late_exact_refuses() {
        fixed_candidate_late_trigger(None);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_wire_trigger_corrupts_generated_then_late_exact_refuses() {
        fixed_candidate_late_trigger(Some(OwnedBudget::new(50 * 1024 * 1024)));
    }

    fn fixed_candidate_second_object_failure(budget: Option<OwnedBudget>) {
        let (mut nodes, mut objects) = native_candidate_fixture();
        assert!(objects.len() > 1);
        let original = objects[1].generated().clone();
        let changed = mutate_fixed_generated(
            &mut nodes,
            &original,
            objects[1].kind() == SHAPE,
            "generation",
        );
        objects[1].set_generated_for_test(changed);
        let mut conn = database();
        let tx = conn.transaction().unwrap();
        let mut store = FixedSql {
            inner: content_store::TxStore::new(&tx).unwrap(),
            budget: budget.clone(),
        };
        for (d, t, b) in nodes.iter() {
            store.put(*d, t, b).unwrap();
        }
        let before = candidate_sql_material_for_test(&tx);
        tx.execute_batch("SAVEPOINT fixed_second").unwrap();
        let error = plan(&mut store, &objects, &[], 1).err().unwrap();
        assert!(error.to_string().contains("候选对象不属于本次生成代"), "{error}");
        assert_ne!(
            candidate_sql_material_for_test(&tx),
            before,
            "第一对象必须已写真实wire"
        );
        tx.execute_batch("ROLLBACK TO fixed_second; RELEASE fixed_second;")
            .unwrap();
        assert_eq!(candidate_sql_material_for_test(&tx), before);
        assert_eq!(
            tx.query_row("SELECT COUNT(*) FROM s_content_objects", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        objects[1].set_generated_for_test(original);
        let actual = plan(&mut store, &objects, &[], 1).unwrap();
        let mut cold = CandidateProbe {
            nodes,
            budget: budget.clone(),
            ..Default::default()
        };
        let expected =
            with_legacy_candidate_fields_for_test(|| plan(&mut cold, &objects, &[], 1)).unwrap();
        assert_eq!(
            candidate_plan_signature(&actual),
            candidate_plan_signature(&expected)
        );
        drop(actual);
        drop(expected);
        drop(store);
        drop(cold);
        tx.rollback().unwrap();
        if let Some(b) = budget {
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn fixed_candidate_plain_second_object_failure_rolls_back_and_retries_full_plan() {
        fixed_candidate_second_object_failure(None);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_second_object_failure_rolls_back_and_retries_full_plan() {
        fixed_candidate_second_object_failure(Some(OwnedBudget::new(50 * 1024 * 1024)));
    }

    fn candidate_field_nodes(nodes: &sc::MemoryNodes) -> Vec<(NodeDigest, u8, Vec<u8>)> {
        let mut rows = nodes
            .iter()
            .map(|(d, t, b)| (*d, t, b.to_vec()))
            .collect::<Vec<_>>();
        rows.sort();
        rows
    }
    fn fixed_candidate_same_inputs(budget: Option<OwnedBudget>) {
        let (nodes, objects) = native_candidate_fixture();
        let mut seen = [false; 2];
        for o in &objects {
            let i = usize::from(o.kind() == SHAPE);
            if seen[i] {
                continue;
            }
            seen[i] = true;
            let mut old = CandidateProbe {
                nodes: nodes.clone(),
                generated_root: Some(o.generated().digest),
                budget: budget.clone(),
                ..Default::default()
            };
            let mut new = CandidateProbe {
                nodes: nodes.clone(),
                generated_root: Some(o.generated().digest),
                budget: budget.clone(),
                ..Default::default()
            };
            let old_f = sc::frame_calls_on_thread();
            let expected =
                with_legacy_candidate_fields_for_test(|| candidate(&mut old, o, 1)).unwrap();
            let old_f = sc::frame_calls_on_thread() - old_f;
            let new_f = sc::frame_calls_on_thread();
            let actual = candidate(&mut new, o, 1).unwrap();
            let new_f = sc::frame_calls_on_thread() - new_f;
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
            assert_eq!(
                candidate_field_nodes(&new.nodes),
                candidate_field_nodes(&old.nodes)
            );
            assert_eq!(
                old.generated_path_lookups,
                if o.kind() == SHAPE { 9 } else { 7 }
            );
            assert_eq!(
                new.generated_path_lookups, 0,
                "改为metadata扫描和晚exact，不再独立逐字段下降"
            );
            assert_eq!(old.singles, 1);
            assert_eq!(new.singles, 1, "each candidate仍完整fresh Public核验");
            assert!(
                new.reads < old.reads,
                "{} physical NodeSource {} vs {}",
                o.kind(),
                new.reads,
                old.reads
            );
            assert!(new_f < old_f, "{} F {new_f} vs {old_f}", o.kind());
            eprintln!(
                "fixed_fields kind={} old_NodeSource={} new_NodeSource={} old_F={old_f} new_F={new_f}",
                o.kind(),
                old.reads,
                new.reads
            );
        }
        assert_eq!(seen, [true, true], "必须真实非空typed与shape");
        if let Some(b) = budget {
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn fixed_candidate_plain_same_entry_full_nodes_and_actual_reads_as_legacy() {
        fixed_candidate_same_inputs(None);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_same_entry_full_nodes_actual_reads_and_release() {
        fixed_candidate_same_inputs(Some(OwnedBudget::new(50 * 1024 * 1024)));
    }

    fn mutate_fixed_generated(
        nodes: &mut sc::MemoryNodes,
        original: &Built,
        shape: bool,
        fault: &str,
    ) -> Built {
        let names = if shape {
            &object_records::SHAPE_SEALED_FIELDS[..]
        } else {
            &object_records::TYPED_SEALED_FIELDS[..]
        };
        let fields = sc::path::lookup_object_fields(nodes, &original.content_ref(), names).unwrap();
        let mut edited = Vec::new();
        for (name, child) in fields {
            if fault == "missing" && name == "object_id" {
                continue;
            }
            let changed = match (fault, name.as_str()) {
                ("kind", "kind") => Some(V::String("foreign-kind".into())),
                ("kind-type", "kind") => Some(V::Number(sc::NumberToken::parse("7").unwrap())),
                ("id", "object_id") => Some(V::String("foreign-id".into())),
                ("generation", "first_known_generation")
                | ("multi-first-error", "first_known_generation") => {
                    Some(V::Number(sc::NumberToken::parse("99").unwrap()))
                }
                ("published", "published_generation") => {
                    Some(V::Number(sc::NumberToken::parse("99").unwrap()))
                }
                ("cut", "first_known_cut") => Some(V::String("cut-99".into())),
                ("revision-type", "object_revision") => Some(V::String("bad".into())),
                ("slot-type", "fact_key") => Some(V::Number(sc::NumberToken::parse("17").unwrap())),
                ("slot-type", "window_start") => Some(V::String("bad".into())),
                ("immutable-null", "payload_json") | ("immutable-null", "comparisons_json") => {
                    Some(V::Null)
                }
                _ => None,
            };
            edited.push((
                name,
                changed.unwrap_or_else(|| V::Edge(child.digest().unwrap())),
            ));
        }
        if matches!(fault, "extra" | "multi-first-error") {
            edited.push(("extra_field".into(), V::Bool(true)));
        }
        let root = sc::write_value(nodes, &V::Object(edited)).unwrap();
        sc::audit_verify_public(nodes, &root.content_ref()).unwrap();
        root
    }
    fn fixed_candidate_bad_fields(budget: Option<OwnedBudget>) {
        let faults = [
            "missing",
            "extra",
            "kind",
            "kind-type",
            "id",
            "generation",
            "published",
            "cut",
            "revision-type",
            "slot-type",
            "immutable-null",
        ];
        for shape in [false, true] {
            for fault in faults {
                let (mut nodes, mut objects) = native_candidate_fixture();
                let o = objects
                    .iter_mut()
                    .find(|o| (o.kind() == SHAPE) == shape)
                    .unwrap();
                let good = o.generated().clone();
                let changed = mutate_fixed_generated(&mut nodes, &good, shape, fault);
                o.set_generated_for_test(changed);
                let mut old = CandidateProbe {
                    nodes: nodes.clone(),
                    budget: budget.clone(),
                    ..Default::default()
                };
                let mut new = CandidateProbe {
                    nodes,
                    budget: budget.clone(),
                    ..Default::default()
                };
                let old_error = with_legacy_candidate_fields_for_test(|| candidate(&mut old, o, 1))
                    .unwrap_err();
                let new_error = candidate(&mut new, o, 1).unwrap_err();
                assert_eq!(old_error, new_error, "single fault shape={shape} {fault}");
                assert_eq!(new.singles, 1);
                if fault == "immutable-null" {
                    assert!(new.puts > 0, "Null仍在wire之后拒绝");
                }
                o.set_generated_for_test(good);
                candidate(&mut new, o, 1).unwrap();
                assert_eq!(new.singles, 2, "错误后重试仍重新完整核验");
            }
        }
        if let Some(b) = budget {
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn fixed_candidate_plain_reidentified_metadata_slot_schema_and_late_null_refuse() {
        fixed_candidate_bad_fields(None);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_reidentified_metadata_slot_schema_and_late_null_refuse() {
        fixed_candidate_bad_fields(Some(OwnedBudget::new(50 * 1024 * 1024)));
    }
    #[test]
    fn fixed_candidate_multiple_fault_first_error_matches_legacy_before_wire() {
        let (mut nodes, mut objects) = native_candidate_fixture();
        let o = objects.iter_mut().find(|o| o.kind() != SHAPE).unwrap();
        let changed = mutate_fixed_generated(&mut nodes, o.generated(), false, "multi-first-error");
        o.set_generated_for_test(changed);
        let mut old = CandidateProbe {
            nodes: nodes.clone(),
            ..Default::default()
        };
        let mut new = CandidateProbe {
            nodes,
            ..Default::default()
        };
        let old_error =
            with_legacy_candidate_fields_for_test(|| candidate(&mut old, o, 1)).unwrap_err();
        let new_error = candidate(&mut new, o, 1).unwrap_err();
        assert!(old_error.to_string().contains("候选对象不属于本次生成代"));
        assert_eq!(new_error, old_error);
        assert_eq!((old.puts, new.puts), (0, 0));
        eprintln!(
            "fixed_fields multi_fault old={old_error:?} new={new_error:?}; original_first_error_preserved=true"
        );
    }

    fn fixed_candidate_corrupt_content(budget: Option<OwnedBudget>) {
        let (nodes, objects) = native_candidate_fixture();
        let o = objects.iter().find(|o| o.kind() != SHAPE).unwrap();
        let mut normal = nodes.clone();
        let deep = field(&mut normal, &o.generated().content_ref(), "object_id")
            .unwrap()
            .digest()
            .unwrap();
        let internal = match o.borrowed() {
            BorrowedObject::Typed(t) => t.record.digest,
            BorrowedObject::Fractal(t) => t.record.digest,
            _ => unreachable!(),
        };
        for (target, corrupt) in [
            (deep, false),
            (deep, true),
            (o.generated().digest, true),
            (internal, true),
        ] {
            let mut broken = sc::MemoryNodes::new();
            for (d, t, body) in nodes.iter() {
                if *d != target {
                    broken.insert_unchecked(*d, t, body.to_vec());
                } else if corrupt {
                    broken.insert_unchecked(*d, t, vec![0]);
                }
            }
            let mut store = CandidateProbe {
                nodes: broken,
                budget: budget.clone(),
                ..Default::default()
            };
            let error = candidate(&mut store, o, 1).unwrap_err();
            assert!(
                error.to_string().contains(if corrupt {
                    sc::code::NODE_DIGEST
                } else {
                    sc::code::NODE_MISSING
                }),
                "{error}"
            );
            assert_eq!(
                store.puts, 0,
                "完整generated或独立internal坏件必须在wire写入前拒绝"
            );
        }
        if let Some(b) = budget {
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn fixed_candidate_plain_fresh_generated_deep_leaf_and_independent_internal_refuse() {
        fixed_candidate_corrupt_content(None);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_fresh_generated_deep_leaf_and_independent_internal_refuse() {
        fixed_candidate_corrupt_content(Some(OwnedBudget::new(50 * 1024 * 1024)));
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_retained_text_and_clone_charge_original_not_foreign_ledger() {
        let (nodes, objects) = native_candidate_fixture();
        let o = objects.iter().find(|o| o.kind() != SHAPE).unwrap();
        let original = OwnedBudget::new(50 * 1024 * 1024);
        let foreign = OwnedBudget::new(50 * 1024 * 1024);
        assert!(!original.same_ledger(&foreign));
        let mut store = CandidateProbe {
            nodes,
            budget: Some(original.clone()),
            ..Default::default()
        };
        let entry = candidate(&mut store, o, 1).unwrap();
        assert!(matches!(&entry.kind, TextValue::Owned(_)));
        if let Slot::Typed { kind, fact_key } = &entry.slot {
            assert!(matches!(kind, TextValue::Owned(_)));
            assert!(matches!(fact_key, TextValue::Owned(_)));
        } else {
            panic!("typed见证不可为空");
        }
        let retained = original.snapshot();
        assert!(retained.live_requested_layout_bytes > 0);
        assert!(retained.allocation_attempts > 0);
        let copy = entry.try_clone().unwrap();
        assert!(
            original.snapshot().live_requested_layout_bytes > retained.live_requested_layout_bytes
        );
        assert_eq!(foreign.snapshot().allocation_attempts, 0);
        drop(copy);
        assert_eq!(
            original.snapshot().live_requested_layout_bytes,
            retained.live_requested_layout_bytes
        );
        drop(entry);
        assert_eq!(original.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(
            original.snapshot().allocated_layouts,
            original.snapshot().released_layouts
        );
        eprintln!(
            "fixed_fields original retained={retained:?} after_drop={:?} foreign={:?}",
            original.snapshot(),
            foreign.snapshot()
        );
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_candidate_owned_scalar_refusal_releases_all_local_owners_without_fallback() {
        let (nodes, objects) = native_candidate_fixture();
        let o = objects.iter().find(|o| o.kind() != SHAPE).unwrap();
        for limit in [0, 1, 32, 128, 512] {
            let budget = OwnedBudget::new(limit);
            let mut store = CandidateProbe {
                nodes: nodes.clone(),
                budget: Some(budget.clone()),
                ..Default::default()
            };
            let result = candidate(&mut store, o, 1);
            if limit == 0 {
                assert!(
                    result.is_err(),
                    "零额度必须在真实Owned标量分配处拒绝，不得Plain回退"
                );
            }
            if let Err(error) = &result {
                assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
            }
            drop(result);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }
    #[cfg(all(feature = "s_allocation_json_testonly", feature = "s_incremental"))]
    #[test]
    fn cold_object_by_id_owned_load_matches_normal_and_releases() {
        let mut fixture = super::super::prepared::tests::Fixture::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (event, token) = fixture.accept_begin(n, row, revision);
            fixture.publish(&event, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let mut plain = content_store::ReadStore::new(&tx).unwrap();
        let mut reader = super::super::history::HashReader::new(&mut plain);
        let expected = load_for_audit(&tx, &mut reader, 3).unwrap();
        let budget = OwnedBudget::new(64 * 1024 * 1024);
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(
            content_store::capture_slab_for_prototype(
                &tx,
                &budget,
                &content_store::CaptureCancellation::never(),
            )
            .unwrap(),
        );
        super::super::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            16384,
            4 * 1024 * 1024,
            |store| {
                let actual = load_in_store(&tx, store, 3, None, None).unwrap();
                assert!(actual.allocation_budget().unwrap().same_ledger(&budget));
                assert_eq!(actual.len(), expected.len());
                for (a, b) in actual.iter().zip(expected.iter()) {
                    assert_eq!(a.id, b.id);
                    assert_eq!(a.revision, b.revision);
                    assert_eq!(a.kind, b.kind);
                    assert_eq!(a.slot, b.slot);
                    assert_eq!(a.generated, b.generated);
                    assert_eq!(a.internal, b.internal);
                    assert_eq!(a.immutable, b.immutable);
                    assert_eq!(a.birth_wire.view().root(), b.birth_wire.view().root());
                    assert_eq!(a.life.first_known_generation, b.life.first_known_generation);
                    assert_eq!(a.life.withdrawn_generation, b.life.withdrawn_generation);
                    assert_eq!(a.life.superseded_by, b.life.superseded_by);
                }
                assert!(budget.snapshot().live_requested_layout_bytes > 0);
                Ok::<_, String>(())
            },
        )
        .unwrap();
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        eprintln!(
            "Owned对象完整load：{}身份，索引/键沿原budget，释放后live=0",
            expected.len()
        );
    }
    thread_local! {
        pub(super) static SHAPE_LEAF_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    #[cfg(feature = "s_incremental")]
    fn leaf_checks<T>(action: impl FnOnce() -> T) -> (T, usize) {
        SHAPE_LEAF_CHECKS.with(|checks| checks.set(0));
        let result = action();
        (result, SHAPE_LEAF_CHECKS.with(std::cell::Cell::get))
    }
    #[cfg(feature = "s_incremental")]
    fn shape_leaf_fixture(tx: &Transaction<'_>) -> (sc::MemoryNodes, sc::CodecVec<Entry>) {
        let (mut nodes, objects) = native_candidate_fixture();
        let p = plan(&mut nodes, &objects, &[], 1).unwrap();
        stage(tx, &mut nodes, &p, &format!("batch3-{}", "1".repeat(64))).unwrap();
        let entries = load(tx, &mut nodes, 1).unwrap();
        (nodes, entries)
    }
    #[cfg(feature = "s_incremental")]
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
    #[cfg(feature = "s_incremental")]
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
        println!(
            "Native shape leaf real load: checks ordinary={ordinary_checks} native={native_checks}; distinct birth entries share source leaves"
        );
    }
    #[cfg(feature = "s_incremental")]
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
    #[cfg(feature = "s_incremental")]
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
            successful: bj::new_map(None),
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
            successful: bj::new_map(None),
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
    fn candidate_plan_signature(p: &Plan) -> Value {
        json!({"generation":p.generation,"index":format!("{:?}",p.index),"births":format!("{:?}",p.births),
            "withdrawals":p.withdrawals,"replaces":p.replaces,
            "active":RefValue::from(p.active.content_ref()),"withdrawn":RefValue::from(p.withdrawn.content_ref()),"upserts":RefValue::from(p.upserts.content_ref())})
    }
    #[cfg(feature = "s_incremental")]
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
        let mut entries = bj::new_work(None);
        s.native_codec_room(&mut entries, objects.len()).unwrap();
        for object in objects {
            entries
                .try_push(candidate_verified_legacy_fields(s, object, g).unwrap())
                .unwrap();
        }
        plan_from_candidates(s, Layout::Sorted, entries, old, g).unwrap()
    }
    fn candidate_sql_material_for_test(tx: &Transaction<'_>) -> Vec<(Vec<u8>, i64, Vec<u8>)> {
        tx.prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
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
        (nodes, generated.objects.into_iter().collect())
    }
    #[derive(Default)]
    struct CandidateProbe {
        nodes: sc::MemoryNodes,
        generated_root: Option<NodeDigest>,
        generated_path_lookups: usize,
        puts: usize,
        reads: usize,
        singles: usize,
        unions: usize,
        budget: Option<OwnedBudget>,
        second_declaration: Option<&'static str>,
        second_reference: Option<ContentRef>,
        future_put_restore: Option<(NodeDigest, u8, Vec<u8>)>,
    }
    impl NodeSource for CandidateProbe {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.reads += 1;
            self.nodes.read(d)
        }
    }
    impl NodeSink for CandidateProbe {
        fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.puts += 1;
            if let Some((digest, tag, body)) = self.future_put_restore.take() {
                self.nodes.put(digest, tag, &body)?;
            }
            self.nodes.put(d, t, b)
        }
    }
    impl CodecRead for CandidateProbe {
        fn verify_many_ref_fresh<'a>(
            &mut self,
            roots: impl IntoIterator<Item = sc::RefView<'a>>,
            mode: sc::Mode,
        ) -> Result<(), sc::CodecError> {
            // 单候选 fresh 核验的新借视图入口；真实核验全部原件，不触联合核验注入器。
            self.singles += 1;
            sc::audit_verify_many_refview(self, roots, mode)
        }

        fn lookup_path(
            &mut self,
            r: &ContentRef,
            path: &[PathStep],
        ) -> Result<ContentRef, sc::CodecError> {
            if r.digest().ok() == self.generated_root {
                self.generated_path_lookups += 1;
            }
            sc::path::lookup_path(self, r, path)
        }
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
    impl CodecStore for CandidateProbe {
        fn generic_store(&mut self) -> &mut dyn NodeStore {
            self
        }
    }

    // 组件级lookup注入：真实合法F的另一份generated视图；不声称完整S坏输入端到端。
    #[cfg(feature = "s_incremental")]
    struct CandidateFieldsProbe {
        inner: CandidateProbe,
        generated_root: NodeDigest,
        replacement: Option<ContentRef>,
        field_reads: usize,
        read_digests: Vec<NodeDigest>,
        put_digests: Vec<NodeDigest>,
    }
    #[cfg(feature = "s_incremental")]
    impl CandidateFieldsProbe {
        fn current(&self, r: &ContentRef) -> ContentRef {
            if r.digest().unwrap() == self.generated_root {
                self.replacement.as_ref().unwrap_or(r).clone()
            } else {
                r.clone()
            }
        }
    }
    #[cfg(feature = "s_incremental")]
    impl NodeSource for CandidateFieldsProbe {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.read_digests.push(*d);
            self.inner.read(d)
        }
    }
    #[cfg(feature = "s_incremental")]
    impl NodeSink for CandidateFieldsProbe {
        fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.put_digests.push(d);
            self.inner.put(d, t, b)
        }
    }
    #[cfg(feature = "s_incremental")]
    impl CodecRead for CandidateFieldsProbe {
        fn lookup_field_ref(
            &mut self,
            r: sc::RefView<'_>,
            key: &str,
        ) -> Result<RefValue, sc::CodecError> {
            let replacement = self.replacement.clone();
            let current = if r.digest()? == self.generated_root {
                replacement.as_ref().map(|v| v.into()).unwrap_or(r)
            } else {
                r
            };
            sc::path::lookup_path_native(self, current, &[f(key)]).map(RefValue::Plain)
        }
        fn lookup_fields_ref<'a>(
            &mut self,
            r: sc::RefView<'_>,
            names: &[&'a str],
        ) -> Result<super::super::audit_store::RefFields<'a>, sc::CodecError> {
            self.field_reads += 1;
            let replacement = self.replacement.clone();
            let current = if r.digest()? == self.generated_root {
                replacement.as_ref().map(|v| v.into()).unwrap_or(r)
            } else {
                r
            };
            sc::path::lookup_fields_native(self, current, names, false)
                .map(super::super::audit_store::RefFields::Plain)
        }

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
    #[cfg(feature = "s_incremental")]
    impl CodecStore for CandidateFieldsProbe {
        fn generic_store(&mut self) -> &mut dyn NodeStore {
            self
        }
    }
    #[cfg(feature = "s_incremental")]
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
    #[cfg(feature = "s_incremental")]
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
            let old = candidate_verified_legacy_fields(&mut baseline, o, 1).unwrap();
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
                assert!(error.to_string().contains("不可变字段不得为空"), "{error}");
                assert!(store.inner.puts > 0, "Null仍在wire写入之后拒绝");
            } else {
                assert_eq!(
                    store.inner.puts, 0,
                    "{fault}: metadata/slot错误须在wire前拒绝：{error}"
                );
            }
        }
    }
    #[cfg(feature = "s_incremental")]
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
        let error = plan_native_union(&mut store, Layout::Sorted, &objects, &[], 1)
            .err()
            .unwrap();
        assert!(error.to_string().contains("candidate_fields_wire_SQL_fault"), "{error}");
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
    #[cfg(feature = "s_incremental")]
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
            let error = plan_native_union(&mut store, Layout::Sorted, objects, &[], 1)
                .err()
                .unwrap();
            assert!(error.to_string().contains(code), "{fault}: {error}");
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
        let actual = plan_native_union(&mut native, Layout::Sorted, objects, &[], 1).unwrap();
        assert_eq!(
            candidate_plan_signature(&actual),
            candidate_plan_signature(&expected)
        );
        assert!(native.reads < ordinary.reads, "真实source读取须减少");
        println!(
            "candidate two-root real source reads ordinary={} union={}; candidate puts ordinary={} union={}",
            ordinary.reads, native.reads, ordinary.puts, native.puts
        );
        let mut mixed = CandidateProbe {
            nodes,
            budget: Some(OwnedBudget::new(1_000_000)),
            ..Default::default()
        };
        assert!(
            plan_native_union(&mut mixed, Layout::Sorted, objects, &[], 1)
                .err()
                .unwrap()
                .to_string().contains("拒绝allocation预算")
        );
        assert_eq!(
            (mixed.reads, mixed.puts, mixed.singles, mixed.unions),
            (0, 0, 0, 0)
        );
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_candidate_union_real_generations_match_plan_and_reduce_sql_reads() {
        real_generations(false, true, true);
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
                .events
                .births
                .range(sweep.previous..g)
                .map(|(_, v)| v.len())
                .sum::<usize>();
            death_event_visits += sweep
                .events
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
        meta_set(&c, "rule_revision", crate::V3_RULE_REVISION).unwrap();
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
    #[cfg(feature = "s_incremental")]
    fn projection_reachable_material(
        source: &mut dyn NodeSource,
        root: &Built,
        mode: sc::Mode,
    ) -> std::collections::BTreeMap<NodeDigest, sc::NodeRead> {
        struct ReadMaterials<'a> {
            source: &'a mut dyn NodeSource,
            nodes: std::collections::BTreeMap<NodeDigest, sc::NodeRead>,
        }
        impl NodeSource for ReadMaterials<'_> {
            fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                let node = self.source.read(digest)?;
                self.nodes.insert(*digest, node.clone());
                Ok(node)
            }
        }
        let mut read = ReadMaterials {
            source,
            nodes: Default::default(),
        };
        sc::verify_refview(&mut read, (&root.content_ref()).into(), mode).unwrap();
        read.nodes
    }
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

    #[cfg(feature = "s_incremental")]
    fn plan_input_fixture(shape_replacement: bool) -> (sc::MemoryNodes, Vec<Entry>, Entry) {
        let (nodes, entries) = projection_fixture();
        let mut old = entries[..2]
            .iter()
            .map(|e| e.try_clone().unwrap())
            .collect::<Vec<_>>();
        for e in &mut old {
            e.life.withdrawn_generation = None;
            e.life.withdrawal_reason = None;
            e.life.superseded_by = None;
        }
        let fresh = entries[if shape_replacement { 5 } else { 6 }]
            .try_clone()
            .unwrap();
        (nodes, old, fresh)
    }

    #[cfg(feature = "s_incremental")]
    fn admitted_plan_candidate(
        store: &(impl CodecRead + ?Sized),
        fresh: &Entry,
    ) -> Result<sc::CodecVec<Entry>, String> {
        let mut new = sc::CodecVec::default();
        store.native_codec_room(&mut new, 1).map_err(err)?;
        new.try_push(fresh.try_clone_admitted(store)?)
            .map_err(err)?;
        Ok(new)
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_plan_owners_outlive_scope_and_match_independent_change_rows() {
        for shape_replacement in [false, true] {
            let (mut expected_nodes, old, fresh) = plan_input_fixture(shape_replacement);
            let old_snapshot = format!("{old:?}");
            let mut expected_index = old
                .iter()
                .map(|e| e.try_clone().unwrap())
                .collect::<Vec<_>>();
            let mut shape_withdrawals = Vec::new();
            let mut typed_withdrawals = Vec::new();
            let mut expected_replaces = Vec::new();
            // 旧公开顺序为 shape 在前、typed 在后。这里直接按场景构建oracle，
            // 不经新plan或其push_raw_withdrawal辅助函数；index仍按old原序，
            // 撤回行按 kind 分桶后 shape 桶在前拼接（夹具每桶至多一行）。
            for e in &mut expected_index {
                let superseded = e.slot.is_shape() == shape_replacement;
                let reason = if superseded {
                    "superseded_by_revision"
                } else if e.slot.is_shape() {
                    "window_removed"
                } else {
                    "fact_removed"
                };
                let sup = superseded.then_some(fresh.id.as_str());
                if e.slot.is_shape() {
                    &mut shape_withdrawals
                } else {
                    &mut typed_withdrawals
                }
                .push(raw_withdrawal(e, reason, sup));
                if superseded {
                    expected_replaces.push(json!({
                        "old_object_id":e.id,
                        "new_object_id":fresh.id
                    }));
                }
                e.life.withdrawn_generation = Some(3);
                e.life.withdrawal_reason = Some(reason.into());
                e.life.superseded_by = sup.map(Into::into);
            }
            let expected_withdrawals = [shape_withdrawals, typed_withdrawals].concat();
            expected_index.push(fresh.try_clone().unwrap());
            let expected_roots =
                at_cut(&mut expected_nodes, Layout::Sorted, &expected_index, 3).unwrap();
            let (nodes, _, _) = plan_input_fixture(shape_replacement);
            let capture = sc::AuditCapture::new(nodes);
            let budget =
                super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
            let baseline = budget.live();
            let plan = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let mut reader =
                        super::super::history::HashReader::captured_for_audit(&capture, None)
                            .map_err(err)?
                            .with_codec_work(scope, &budget);
                    let new = admitted_plan_candidate(&reader, &fresh)?;
                    plan_from_candidates(&mut reader, Layout::Sorted, new, &old, 3)
                })
                .unwrap();
            assert_eq!(
                format!("{old:?}"),
                old_snapshot,
                "候选规划不得改写原历史输入"
            );
            assert_eq!(
                plan.withdrawals.view(),
                aj::JsonView::Plain(&Value::Array(expected_withdrawals))
            );
            assert_eq!(
                plan.replaces.view(),
                aj::JsonView::Plain(&Value::Array(expected_replaces))
            );
            assert_eq!(plan.active.digest, expected_roots.0.digest);
            assert_eq!(plan.withdrawn.digest, expected_roots.1.digest);
            assert_eq!(plan.upserts.digest, expected_roots.2.digest);
            assert!(matches!(&plan.withdrawals, bj::JsonOwner::Plain(v) if v.is_native_admitted()));
            assert!(matches!(&plan.replaces, bj::JsonOwner::Plain(v) if v.is_native_admitted()));
            let retained = budget.live();
            assert!(retained > baseline, "scope结束后各返回owner仍持费");
            drop(plan.index);
            assert!(budget.live() < retained, "index实体与容量许可随index释放");
            let retained = budget.live();
            drop(plan.births);
            assert!(budget.live() < retained, "births实体与容量许可独立释放");
            let retained = budget.live();
            drop(plan.withdrawals);
            assert!(budget.live() < retained, "撤回JSON owner独立释放");
            drop(plan.replaces);
            assert_eq!(budget.live(), baseline, "末个返回owner销毁后回到原基线");
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_plan_late_missing_wire_releases_partial_plan_without_mutating_input() {
        let (_, old, fresh) = plan_input_fixture(true);
        let old_snapshot = format!("{old:?}");
        let fresh_snapshot = format!("{fresh:?}");
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut reader = super::super::history::HashReader::captured_for_audit(&capture, None)
                .map_err(err)?
                .with_codec_work(scope, &budget);
            let new = admitted_plan_candidate(&reader, &fresh)?;
            // metadata、撤回JSON、births/index已产生后，真实wire读取才发现原库缺节点。
            plan_from_candidates(&mut reader, Layout::Sorted, new, &old, 3)
        });
        let message = match result.err().expect("缺原wire不得产生Plan") {
            sc::capture_allocation::scratch::WorkError::Operation(message) => message,
            other => panic!("缺原wire必须返回业务读取错误：{other:?}"),
        };
        assert!(message.to_string().contains(sc::code::NODE_MISSING), "{message}");
        assert_eq!(format!("{old:?}"), old_snapshot);
        assert_eq!(format!("{fresh:?}"), fresh_snapshot);
        assert_eq!(budget.live(), baseline, "失败局部的所有许可必须回收");
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_plan_capacity_refusal_preserves_input_and_budget() {
        let (_, old, _) = plan_input_fixture(false);
        let old_snapshot = format!("{old:?}");
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut reader = super::super::history::HashReader::captured_for_audit(&capture, None)
                .map_err(err)?
                .with_codec_work(scope, &budget);
            let _held = budget.reserve(1024 * 1024 - budget.live() - 1).unwrap();
            plan_from_candidates(
                &mut reader,
                Layout::Sorted,
                sc::CodecVec::default(),
                &old,
                3,
            )
        });
        assert!(result.is_err(), "一字节预算不得准入历史ordinal工作集");
        assert_eq!(format!("{old:?}"), old_snapshot);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_changes_use_current_scope_and_return_owned_json_permits() {
        let (_, entries) = projection_fixture();
        let expected = changes_at(&entries, 3).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let (withdrawals, replaces) = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let store = super::super::audit_store::NativeAdmission(Some(scope));
                changes_at_admitted(&store, &entries, 3, None)
            })
            .unwrap();
        assert!(matches!(&withdrawals, bj::JsonOwner::Plain(value) if value.is_native_admitted()));
        assert!(matches!(&replaces, bj::JsonOwner::Plain(value) if value.is_native_admitted()));
        assert_eq!(
            withdrawals.view(),
            aj::JsonView::Plain(&Value::Array(expected.0))
        );
        assert_eq!(
            replaces.view(),
            aj::JsonView::Plain(&Value::Array(expected.1))
        );
        assert!(budget.live() > baseline);
        drop((withdrawals, replaces));
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_event_owners_share_once_and_release_after_last_scope_owner() {
        let (_, entries) = projection_fixture();
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let (events, alias) = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let store = super::super::audit_store::NativeAdmission(Some(scope));
                let events =
                    HistoryEvents::new_admitted(&entries, &store)?.into_arc_admitted(&store)?;
                let before_clone = budget.live();
                let alias = events.clone();
                assert_eq!(
                    budget.live(),
                    before_clone,
                    "共享事件clone不得重收载荷和控制块"
                );
                assert!(NativeArc::ptr_eq(&events, &alias));
                Ok::<_, String>((events, alias))
            })
            .unwrap();
        assert!(
            budget.live() > baseline,
            "离域共享对象仍持有动态字段及共享块许可"
        );
        let retained = budget.live();
        drop(events);
        assert_eq!(budget.live(), retained, "另一强owner仍持有事件载荷");
        assert_eq!(alias.ranks.len(), entries.len());
        drop(alias);
        assert_eq!(budget.live(), baseline, "末个owner销毁才归还全部原许可");
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_event_cursors_admitted_cuts_match_scan_and_drop_to_baseline() {
        use super::super::audit_retention::Retained;
        let (_, entries) = projection_fixture();
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let sweep = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let store = super::super::audit_store::NativeAdmission(Some(scope));
                    let mut sweep = ProjectionSweep::new_admitted(&entries, &store)?;
                    for g in [0, 1, 3, 3, 2, 0, 3] {
                        let before = sweep.active.heap_bytes() + sweep.withdrawn.heap_bytes();
                        sweep.seek_admitted(g, Layout::Sorted, &entries, &store)?;
                        let delta = std::mem::take(&mut sweep.retained);
                        assert_eq!(
                            delta.apply(before).unwrap(),
                            sweep.active.heap_bytes() + sweep.withdrawn.heap_bytes()
                        );
                        for (dead, actual) in [(false, &sweep.active), (true, &sweep.withdrawn)] {
                            let mut expected = entries
                                .iter()
                                .enumerate()
                                .filter(|(_, e)| {
                                    if dead {
                                        e.life.withdrawn_generation.is_some_and(|d| d <= g)
                                    } else {
                                        e.life.first_known_generation <= g
                                            && e.life.withdrawn_generation.is_none_or(|d| d > g)
                                    }
                                })
                                .map(|(i, _)| i)
                                .collect::<Vec<_>>();
                            expected.sort_unstable_by(|a, b| {
                                ordering(&entries[*a], &entries[*b], dead).then(a.cmp(b))
                            });
                            assert_eq!(
                                actual.keys().map(|(_, i)| *i).collect::<Vec<_>>(),
                                expected,
                                "cut={g}, dead={dead}"
                            );
                        }
                    }
                    Ok::<_, String>(sweep)
                })
                .unwrap();
            assert!(budget.live() > baseline);
            drop(sweep);
            assert_eq!(
                budget.live(),
                baseline,
                "游标/共享事件/临时排序载荷不可累计retain"
            );
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_event_failed_consuming_candidate_drops_cursor_and_all_permits() {
        let (_, entries) = projection_fixture();
        let limit = 8 * 1024 * 1024;
        let budget = super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
        let baseline = budget.live();
        let error = budget.codec_work().unwrap().run(|scope| {
            let store = super::super::audit_store::NativeAdmission(Some(scope));
            let mut candidate = ProjectionSweep::new_admitted(&entries, &store)?;
            candidate.seek_admitted(1, Layout::Sorted, &entries, &store)?;
            let occupied = budget.reserve(limit - budget.live()).unwrap();
            // 与 history 消费边界相同：候选按值进入，Err 不返回部分游标。
            let consume = |mut candidate: ProjectionSweep| -> Result<ProjectionSweep, String> {
                candidate.seek_admitted(3, Layout::Sorted, &entries, &store)?;
                Ok(candidate)
            };
            let result = consume(candidate);
            assert!(result.is_err());
            drop(occupied);
            result.map(|_| ())
        });
        assert!(error.is_err());
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_event_failed_wire_replacement_preserves_saved_owner() {
        let (_, entries) = projection_fixture();
        let wire = &entries[0].birth_wire;
        let capture = sc::AuditCapture::new(sc::MemoryNodes::default());
        let limit = 1 << 20;
        let budget = super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
        let baseline = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let store = super::super::audit_store::NativeAdmission(Some(scope));
            let mut memo = projection_memo(&capture, 0);
            store.native_codec_room(&mut memo.wires, 1).map_err(err)?;
            memo.wires.try_push([None, None]).map_err(err)?;
            memo.remember_wire(&store, 0, 0, wire)?;
            let before = budget.live();
            let occupied = budget.reserve(limit - before).unwrap();
            let error = memo.remember_wire(&store, 0, 0, wire).unwrap_err();
            let actual = memo.wires[0][0].as_ref().unwrap().view();
            let expected = wire.view();
            assert_eq!(
                (
                    actual.format(),
                    actual.root(),
                    actual.kind(),
                    actual.extent()
                ),
                (
                    expected.format(),
                    expected.root(),
                    expected.kind(),
                    expected.extent()
                )
            );
            drop(occupied);
            assert_eq!(budget.live(), before, "失败复制必须销毁临时permit");
            Err::<(), String>(error.to_string())
        });
        assert!(result.is_err());
        assert_eq!(budget.live(), baseline);
    }

    /// #1491：撤回代错开的投影夹具（shape-first 延至第 4 代撤回），使 bounded 撤回尾与
    /// layout/1 内容序可区分；其余身份与生命期同 [`projection_fixture`]。
    fn staggered_withdrawal_fixture() -> (sc::MemoryNodes, Vec<Entry>) {
        let (mut nodes, mut entries) = projection_fixture();
        assert_eq!(entries[1].id.as_str(), "shape-first");
        entries[1].life.withdrawn_generation = Some(4);
        wire_at(&mut nodes, &entries[1], 4).unwrap();
        (nodes, entries)
    }

    fn cut_arrays(
        nodes: &mut sc::MemoryNodes,
        entries: &[Entry],
        g: i64,
        active: &[usize],
        withdrawn: &[usize],
    ) -> (Built, Built, Built) {
        let mut project = |ordinals: Vec<usize>| {
            let refs = ordinals
                .into_iter()
                .map(|i| wire_at(&mut *nodes, &entries[i], g).unwrap())
                .collect::<Vec<_>>();
            array(&mut *nodes, &refs).unwrap()
        };
        let births = (0..entries.len())
            .filter(|i| entries[*i].life.first_known_generation == g)
            .collect();
        (
            project(active.to_vec()),
            project(withdrawn.to_vec()),
            project(births),
        )
    }

    #[test]
    fn layout_object_index_sorted_public_rank_equals_order_key() {
        let (_, entries) = staggered_withdrawal_fixture();
        for a in &entries {
            for b in &entries {
                for dead in [false, true] {
                    let withdrawn = |e: &Entry| e.life.withdrawn_generation.is_some();
                    if dead && !(withdrawn(a) && withdrawn(b)) {
                        continue;
                    }
                    assert_eq!(
                        public_ordering(Layout::Sorted, a, b, dead),
                        ordering(a, b, dead),
                        "{} {} dead={dead}",
                        a.id.as_str(),
                        b.id.as_str()
                    );
                }
            }
        }
    }

    #[test]
    fn layout_object_index_bounded_revision_and_withdrawal_fall_to_tail() {
        let (mut nodes, entries) = staggered_withdrawal_fixture();
        // 0 typed-z(1→3) 1 shape-first(1→4) 2 shape-second(1) 3 typed-b(1) 4 future-born(2)
        // 5 shape-replacement(3) 6 typed-a(3)：5、6 是 1、0 的修订。
        for (layout, g, active, withdrawn) in [
            (Layout::Sorted, 3, &[1, 2, 5, 4, 6, 3][..], &[0][..]),
            (Layout::Bounded1, 3, &[1, 2, 3, 4, 5, 6][..], &[0][..]),
            (Layout::Sorted, 4, &[2, 5, 4, 6, 3][..], &[1, 0][..]),
            (Layout::Bounded1, 4, &[2, 3, 4, 5, 6][..], &[0, 1][..]),
        ] {
            let expected = cut_arrays(&mut nodes, &entries, g, active, withdrawn);
            let actual = at_cut(&mut nodes, layout, &entries, g).unwrap();
            assert_eq!(actual, expected, "{layout:?} cut={g}");
        }
    }

    #[test]
    fn layout_object_index_incremental_sweep_equals_full_rebuild_under_both_layouts() {
        let (mut nodes, entries) = staggered_withdrawal_fixture();
        for layout in [Layout::Sorted, Layout::Bounded1] {
            let mut sweep = ProjectionSweep::new(&entries);
            for g in [1, 2, 3, 4, 2, 1, 4, 3] {
                let full = at_cut(&mut nodes, layout, &entries, g).unwrap();
                sweep.seek(g, layout, &entries);
                let active = sweep.active.keys().map(|(_, i)| *i).collect::<Vec<_>>();
                let withdrawn = sweep.withdrawn.keys().map(|(_, i)| *i).collect::<Vec<_>>();
                let incremental = cut_arrays(&mut nodes, &entries, g, &active, &withdrawn);
                assert_eq!(incremental, full, "{layout:?} cut={g}");
            }
        }
    }

    /// #1491 独审 LOW-1 负控：撤回代在入集后被改写（4→3），bounded 撤回键随之改变而不在集内；
    /// 旧实现静默放行并留下陈旧成员，现须报错。layout/1 的 Shape 撤回键取首获知代、不随撤回代
    /// 变化，同一改写下照常出集（正控）。
    #[test]
    fn layout_object_index_remove_rank_missing_key_fails_closed() {
        let store = super::super::audit_store::NativeAdmission(None);
        for layout in [Layout::Sorted, Layout::Bounded1] {
            let (_, entries) = staggered_withdrawal_fixture();
            let (_, mut rewritten) = staggered_withdrawal_fixture();
            assert_eq!(rewritten[1].life.withdrawn_generation, Some(4));
            rewritten[1].life.withdrawn_generation = Some(3);
            let mut sweep = ProjectionSweep::new(&entries);
            sweep.seek(4, layout, &entries);
            let result = sweep.seek_admitted(1, layout, &rewritten, &store);
            match layout {
                Layout::Sorted => result.unwrap(),
                Layout::Bounded1 => {
                    let error = result.unwrap_err();
                    assert!(error.contains("RankSet出集键不存在"), "{error}");
                }
            }
        }
    }

    /// #1491 独审 LOW-1 负控：撤回成员缺撤回代时公开主序键报错，不以 `i64::MAX` 吸收；
    /// 撤回事件已入游标、生命期却丢了撤回代时，入撤回集即拒。
    #[test]
    fn layout_object_index_withdrawn_without_generation_fails_closed() {
        let (_, mut entries) = staggered_withdrawal_fixture();
        assert_eq!(entries[2].life.withdrawn_generation, None);
        for layout in [Layout::Sorted, Layout::Bounded1] {
            let error = public_rank(layout, &entries[2], true).unwrap_err();
            assert!(error.to_string().contains("缺撤回代"), "{error}");
            assert!(public_rank(layout, &entries[2], false).is_ok());
        }
        let store = super::super::audit_store::NativeAdmission(None);
        let mut sweep = ProjectionSweep::new(&entries);
        entries[1].life.withdrawn_generation = None;
        let error = sweep
            .seek_admitted(4, Layout::Bounded1, &entries, &store)
            .unwrap_err();
        assert!(error.contains("缺撤回代"), "{error}");
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_relation_index_sort_ties_keep_complete_projection_roots() {
        let (mut nodes, entries) = projection_fixture();
        assert_eq!(
            ordering(&entries[1], &entries[2], false),
            std::cmp::Ordering::Equal
        );
        let mut sweep = ProjectionSweep::new(&entries);
        for g in [1, 2, 3, 4, 2, 1, 4] {
            let expected = at_cut(&mut nodes, Layout::Sorted, &entries, g).unwrap();
            sweep.seek(g, Layout::Sorted, &entries);
            let mut project = |ordinals: Vec<usize>| {
                let refs = ordinals
                    .into_iter()
                    .map(|i| wire_at(&mut nodes, &entries[i], g).unwrap())
                    .collect::<Vec<_>>();
                array(&mut nodes, &refs).unwrap()
            };
            let active = project(sweep.active.keys().map(|(_, i)| *i).collect());
            let withdrawn = project(sweep.withdrawn.keys().map(|(_, i)| *i).collect());
            let births = project(
                sweep
                    .events
                    .births
                    .get(&g)
                    .into_iter()
                    .flatten()
                    .copied()
                    .collect(),
            );
            assert_eq!(expected, (active, withdrawn, births), "cut={g}");
        }
        assert_eq!(sweep.events.sort_passes, 2);
        assert_eq!(sweep.events.initial_entries, entries.len());
    }
    #[cfg(feature = "s_incremental")]
    fn projection_memo(capture: &sc::AuditCapture, count: usize) -> NativeProjection<'_> {
        NativeProjection {
            capture,
            wires: sc::CodecVec::from_plain((0..count).map(|_| [None, None]).collect()),
            sweep: None,
            retained: Default::default(),
        }
    }
    #[cfg(feature = "s_incremental")]
    #[derive(Default)]
    struct RecordMeter {
        records: std::cell::Cell<usize>,
        stop: bool,
    }
    #[cfg(feature = "s_incremental")]
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
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_cut_projection_endpoints_sort_ties_records_and_overlay_lifetime() {
        let (mut nodes, entries) = projection_fixture();
        let capture = sc::AuditCapture::new(nodes.clone());
        let mut store = super::super::history::HashReader::captured(&capture, None);
        let mut memo = projection_memo(&capture, entries.len());
        let old_meter = RecordMeter::default();
        let memo_meter = RecordMeter::default();
        for g in 1..=6 {
            let oracle =
                at_cut_metered(&mut nodes, Layout::Sorted, &entries, g, Some(&old_meter)).unwrap();
            // 测共用 collect 的记录门；生产 Native 构造门仍只允许 budget=None。
            let actual = at_cut_projected(
                &mut store,
                Layout::Sorted,
                &entries,
                g,
                Some(&memo_meter),
                Some(&mut memo),
            )
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
        let stopped = at_cut_projected(
            &mut store,
            Layout::Sorted,
            &entries,
            6,
            Some(&stop),
            Some(&mut memo),
        );
        assert!(stopped.unwrap_err().to_string().contains("record_stop"));
        assert_eq!(stop.records.get(), 1);
        assert_eq!(
            WIRE_PROJECTIONS.with(|calls| calls.get()),
            0,
            "记录门必须在命中前拒绝"
        );
    }
    #[test]
    fn history_public_coverage_ordinary_bound_births_sorted_insert_withdraw_revision_matches_full_projection(
    ) {
        let (mut nodes, entries) = projection_fixture();
        for g in [1, 2, 3, 4, 2] {
            let expected = at_cut_metered(&mut nodes, Layout::Sorted, &entries, g, None).unwrap();
            WIRE_PROJECTIONS.with(|v| v.set(0));
            let actual =
                at_cut_projected_bound(&mut nodes, Layout::Sorted, &entries, g, None, None, true)
                    .unwrap();
            assert_eq!(actual, expected, "cut={g}");
            let deaths = entries
                .iter()
                .filter(|e| e.life.withdrawn_generation.is_some_and(|d| d <= g))
                .count();
            assert_eq!(WIRE_PROJECTIONS.with(|v| v.get()), deaths);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn history_public_coverage_ordinary_bound_births_preserves_record_budget_before_reuse() {
        let (mut nodes, entries) = projection_fixture();
        let old_meter = RecordMeter::default();
        let new_meter = RecordMeter::default();
        let old =
            at_cut_metered(&mut nodes, Layout::Sorted, &entries, 3, Some(&old_meter)).unwrap();
        let new = at_cut_projected_bound(
            &mut nodes,
            Layout::Sorted,
            &entries,
            3,
            Some(&new_meter),
            None,
            true,
        )
        .unwrap();
        assert_eq!(new, old);
        assert_eq!(new_meter.records.get(), old_meter.records.get());
        let stop = RecordMeter {
            records: std::cell::Cell::new(0),
            stop: true,
        };
        assert!(at_cut_projected_bound(
            &mut nodes,
            Layout::Sorted,
            &entries,
            3,
            Some(&stop),
            None,
            true,
        )
        .unwrap_err()
        .to_string().contains("record_stop"));
        assert_eq!(stop.records.get(), 1);
    }

    #[test]
    fn history_public_coverage_ordinary_bound_births_keeps_life_first_error() {
        let (mut nodes, mut entries) = projection_fixture();
        entries[0].life.published_generation = 10;
        let old = at_cut_metered(&mut nodes, Layout::Sorted, &entries, 1, None).unwrap_err();
        let new = at_cut_projected_bound(&mut nodes, Layout::Sorted, &entries, 1, None, None, true)
            .unwrap_err();
        assert_eq!(new, old);
    }

    #[cfg(feature = "s_incremental")]
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
            .to_string().contains("尚未出生"));
        for g in [1, 2] {
            let original = entries[0].life.published_generation;
            entries[0].life.published_generation = 10;
            assert!(memo
                .wire(&mut store, &entries[0], 0, g)
                .unwrap_err()
                .to_string().contains("晚于本次生成代"));
            entries[0].life.published_generation = original;
        }
        memo.wire(&mut store, &entries[0], 0, 3).unwrap();
        entries[0].life.withdrawal_reason = None;
        for g in [3, 4, 6] {
            assert!(memo
                .wire(&mut store, &entries[0], 0, g)
                .unwrap_err()
                .to_string().contains("撤回必须携带"));
        }
        // 第一次 exact 字段检查失败，不能留下成功槽；同一错误重试也必须拒绝。
        let empty = sc::write_value(&mut store, &V::Object(Vec::new())).unwrap();
        entries[1].birth_wire = empty.content_ref().into();
        for _ in 0..2 {
            assert!(memo.wire(&mut store, &entries[1], 1, 1).is_err());
            assert!(memo.wires[1].iter().all(Option::is_none));
        }
    }
    #[cfg(feature = "s_incremental")]
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
                    error.to_string().contains(if corrupt {
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
    fn history_public_coverage_bound_births_entry_routes() {
        let mut fixture = crate::v3::prepared::tests::Fixture::new();
        let mut readonly = Connection::open_with_flags(
            fixture.conn.path().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let tx = readonly.transaction().unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut reader = crate::v3::history::HashReader::new(&mut source);
        let mut loaded = load_for_history(&tx, &mut reader, 0).unwrap();
        loaded
            .at_cut_bound_births(&tx, &mut reader, 0, None)
            .unwrap();
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        let mut captured =
            crate::v3::history::HashReader::captured_for_audit(&capture, None).unwrap();
        assert!(loaded
            .at_cut_bound_births(&tx, &mut captured, 0, None)
            .is_err());
        #[cfg(feature = "s_incremental")]
        {
            captured.enable_full_arrays_for_test();
            let mut native = load_for_history(&tx, &mut captured, 0).unwrap();
            assert!(native.native.is_some());
            assert!(native
                .at_cut_bound_births(&tx, &mut reader, 0, None)
                .is_err());
        }
        let writable = fixture.conn.transaction().unwrap();
        assert!(loaded
            .at_cut_bound_births(&writable, &mut reader, 0, None)
            .is_err());
    }
    #[test]
    fn history_public_coverage_complete_load_real_generations() {
        real_generations(false, false, true);
    }
    #[test]
    fn three_real_generations_match_shared_diff_and_preserve_historical_cut_and_prior() {
        real_generations(false, false, true);
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_cut_projection_real_generations_reuse_exact_wire_after_overlay_clear() {
        real_generations(true, false, true);
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_history_projection_sweep_positive_real_generations_complete_roots_and_material() {
        // 唯一false入口：真实输入正常追加/修订；不进入旧混合候选或负例分支。
        real_generations(true, false, false);
    }
    #[cfg(feature = "s_incremental")]
    fn projection_sweep_positive_event_counts(index: &[Entry]) {
        use crate::v3::audit_retention::Retained;
        // 调用方传入经过完整load的真实索引，绝不使用旧抽象death==birth夹具。
        for e in index {
            assert!(e
                .life
                .withdrawn_generation
                .is_none_or(|death| e.life.first_known_generation < death));
        }
        let mut before_g = HistorySweep::new(index);
        for g in 1..=4 {
            let before = before_g.heap_bytes();
            before_g.advance(g).unwrap();
            assert_eq!(
                before_g.take_retained_delta().apply(before).unwrap(),
                before_g.heap_bytes(),
                "before-g 独立克隆及删除计费"
            );
        }
        let mut projection = ProjectionSweep::new(index);
        let advance_retained = |projection: &mut ProjectionSweep, g| {
            let before = projection.active.heap_bytes() + projection.withdrawn.heap_bytes();
            projection.seek(g, Layout::Sorted, index);
            let delta = std::mem::take(&mut projection.retained);
            assert_eq!(
                delta.apply(before).unwrap(),
                projection.active.heap_bytes() + projection.withdrawn.heap_bytes(),
                "through-g 活动/撤回独立克隆与真实删除计费，cut={g}"
            );
        };
        let check = |projection: &ProjectionSweep, g: i64| {
            let mut active = index
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.life.first_known_generation <= g
                        && e.life.withdrawn_generation.is_none_or(|d| d > g)
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let mut withdrawn = index
                .iter()
                .enumerate()
                .filter(|(_, e)| e.life.withdrawn_generation.is_some_and(|d| d <= g))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            active.sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], false).then(a.cmp(b)));
            withdrawn
                .sort_unstable_by(|a, b| ordering(&index[*a], &index[*b], true).then(a.cmp(b)));
            assert_eq!(
                projection
                    .active
                    .keys()
                    .map(|(_, i)| *i)
                    .collect::<Vec<_>>(),
                active
            );
            assert_eq!(
                projection
                    .withdrawn
                    .keys()
                    .map(|(_, i)| *i)
                    .collect::<Vec<_>>(),
                withdrawn
            );
        };
        for g in 1..=512 {
            advance_retained(&mut projection, g);
            check(&projection, g);
        }
        assert_eq!(projection.events.initial_entries, index.len());
        assert_eq!(projection.events.sort_passes, 2);
        assert_eq!(
            projection.visits.births,
            index.len(),
            "512 cuts出生事件各过一次"
        );
        assert_eq!(
            projection.visits.deaths,
            index
                .iter()
                .filter(|e| e.life.withdrawn_generation.is_some())
                .count(),
            "512 cuts撤回事件各过一次"
        );
        let visited = (projection.visits.births, projection.visits.deaths);
        advance_retained(&mut projection, 512);
        assert_eq!(
            (projection.visits.births, projection.visits.deaths),
            visited,
            "重复cut不推进事件"
        );
        for g in [1, 3, 3, 2, 8, i64::MAX, 1] {
            advance_retained(&mut projection, g);
            check(&projection, g);
            assert_eq!(
                projection.events.sort_passes, 2,
                "跳跃或回退只过所跨事件，不重排序"
            );
        }
    }
    fn real_generations(
        native_projection: bool,
        native_union: bool,
        include_legacy_negative_cases: bool,
    ) {
        #[cfg(not(feature = "s_incremental"))]
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
            if include_legacy_negative_cases && g == 3 {
                // 仅旧测试：混合真实producer候选不属于本次正常来源路径。
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
                generated.objects.try_push(old_candidate).unwrap();
                generated.objects[position..].rotate_right(1);
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
            #[cfg(feature = "s_incremental")]
            if include_legacy_negative_cases && native_union && g == 1 {
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
                let error = plan_native_union(&mut s, Layout::Sorted, &generated.objects, &old, g)
                    .err()
                    .unwrap();
                assert!(
                    error.to_string().contains(sc::code::NODE_DIGEST),
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
            #[cfg(feature = "s_incremental")]
            let p = if native_union {
                let reads_before = content_store::sql_reads_on_thread();
                let old_union =
                    candidate_union_old_fields_for_test(&mut s, &generated.objects, &old, g);
                let old_union_reads = content_store::sql_reads_on_thread() - reads_before;
                let old_union_material = candidate_sql_material_for_test(&tx);
                let reads_before = content_store::sql_reads_on_thread();
                let union =
                    plan_native_union(&mut s, Layout::Sorted, &generated.objects, &old, g).unwrap();
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
                assert!(
                    union_reads < old_union_reads,
                    "generated一次字段绑定须减少实际SQL读取：gen{g} old_union={old_union_reads} fields={union_reads}"
                );
                println!(
                    "Native candidate fields gen{g}: actual_SQL_reads old_union={old_union_reads} fields={union_reads}; all_plan_roots_metadata_and_node_bytes_equal=1"
                );
                assert!(
                    union_reads < ordinary_reads,
                    "实际SQL读数须减少：gen{g} ordinary={ordinary_reads} union={union_reads}"
                );
                println!(
                    "Native candidate union gen{g}: actual_SQL_reads ordinary={ordinary_reads} union={union_reads}; plan roots/metadata identical"
                );
                union
            } else {
                p
            };
            #[cfg(not(feature = "s_incremental"))]
            let _ = (native_union, ordinary_reads);
            assert_eq!(
                p.withdrawals.view(),
                aj::JsonView::BorrowedArray(&bj::ValuesArray(&oracle.1))
            );
            assert_eq!(
                p.replaces.view(),
                aj::JsonView::BorrowedArray(&bj::ValuesArray(&oracle.2))
            );
            assert_eq!(
                p.births.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
                oracle
                    .0
                    .iter()
                    .map(|v| v["object_id"].as_str().unwrap())
                    .collect::<Vec<_>>()
            );
            assert!(!p.births.is_empty());
            total_deaths += p.withdrawals.array_len();
            let batch = format!("batch3-{}", g.to_string().repeat(64));
            stage(&tx, &mut s, &p, &batch).unwrap();
            let stored = load(&tx, &mut s, g).unwrap();
            let old_arrays = at_cut_metered(&mut s, Layout::Sorted, &stored, g, None).unwrap();
            let bound_arrays =
                at_cut_projected_bound(&mut s, Layout::Sorted, &stored, g, None, None, true)
                    .unwrap();
            assert_eq!(bound_arrays, old_arrays, "complete load cut={g}");
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
            let (actual_active, actual_dead, actual_new) =
                at_cut(&mut s, Layout::Sorted, &stored, g).unwrap();
            assert_eq!(actual_active.extent, Some(generated.objects.len() as u64));
            assert_eq!(actual_new.extent, Some(p.births.len() as u64));
            for r in &stored {
                let actual = wire_at(&mut s, r, g).unwrap();
                let expected = crate::project_object_wire(&legacy_value(&mut s, r)).unwrap();
                assert_eq!(value_ref(&mut s, actual.view()), expected);
            }
            let historical = at_cut(&mut s, Layout::Sorted, &stored, 1).unwrap();
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
            assert_eq!(
                aj::JsonView::BorrowedArray(&bj::ValuesArray(&expected_changes.0)),
                p.withdrawals.view()
            );
            assert_eq!(
                aj::JsonView::BorrowedArray(&bj::ValuesArray(&expected_changes.1)),
                p.replaces.view()
            );
            if include_legacy_negative_cases && g == 2 {
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
                    .to_string().contains("IdentityConflict"));
            }
            let _ = actual_dead;
            #[cfg(feature = "s_incremental")]
            if native_projection && g == 3 {
                let capture = content_store::capture_for_prototype(&tx).unwrap();
                let mut captured = super::super::history::HashReader::captured(&capture, None);
                captured.enable_full_arrays_for_test();
                let mut current = load_for_history(&tx, &mut captured, g).unwrap();
                projection_sweep_positive_event_counts(current.entries());
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

                // 同一完整load/capture，合法修订和撤回后覆盖单调、重复、跳跃及回退cut。
                let mut scan = load_for_history(&tx, &mut captured, g).unwrap();
                let before = current.native_sweep().unwrap();
                assert!(
                    NativeArc::ptr_eq(
                        &before.events,
                        &current
                            .native
                            .as_ref()
                            .unwrap()
                            .sweep
                            .as_ref()
                            .unwrap()
                            .events,
                    ),
                    "before-g和through-g必须共享一份事件表"
                );
                let mut output_entries = 0;
                for cut in [1, 2, 3, 3, 6, 1, 3, 2, i64::MAX, 1] {
                    let cut_batch = format!("batch3-{}", cut.to_string().repeat(64));
                    let old = scan
                        .native_dual_at_cut_scan_for_test(&mut captured, cut, &cut_batch)
                        .unwrap();
                    let new = current
                        .native_dual_at_cut(&mut captured, cut, &cut_batch)
                        .unwrap();
                    for (want, got) in old.iter().zip(&new) {
                        assert_eq!(want, got, "cut-{cut}完整Template/Public根和头相同");
                        for (a, b, mode) in [
                            (&want.0, &got.0, sc::Mode::Template),
                            (&want.1, &got.1, sc::Mode::Public),
                        ] {
                            let expected = projection_reachable_material(&mut captured, a, mode);
                            let actual = projection_reachable_material(&mut captured, b, mode);
                            assert_eq!(expected, actual, "cut-{cut}全部可达节点tag/body相同");
                        }
                        assert_eq!(
                            value(&mut captured, &want.1.content_ref()),
                            value(&mut captured, &got.1.content_ref())
                        );
                        output_entries += got.1.extent.unwrap() as usize;
                    }
                    let sweep = current.native.as_ref().unwrap().sweep.as_ref().unwrap();
                    assert_eq!(sweep.events.sort_passes, 2, "每cut不得重排序");
                    assert_eq!(
                        sweep.events.initial_entries,
                        stored.len(),
                        "索引只初始化一次"
                    );
                    captured.clear_overlay();
                }
                let sweep = current.native.as_ref().unwrap().sweep.as_ref().unwrap();
                // 先前g=3和g=1两次投影也计入输出量；不裁剪完整数组。
                let earlier_outputs = dual
                    .iter()
                    .chain(dual_old.iter())
                    .map(|p| p.1.extent.unwrap() as usize)
                    .sum::<usize>();
                assert_eq!(
                    sweep.visits.output_entries,
                    output_entries + earlier_outputs
                );

                assert!(current.native.is_some());
                if include_legacy_negative_cases {
                    let other_capture = content_store::capture_for_prototype(&tx).unwrap();
                    let mut other_store =
                        super::super::history::HashReader::captured(&other_capture, None);
                    other_store.enable_full_arrays_for_test();
                    assert!(current
                        .at_cut(&mut other_store, 1, None)
                        .unwrap_err()
                        .to_string().contains("不属于本次捕获"));
                    let records = RecordMeter::default();
                    assert!(current
                        .at_cut(&mut captured, 1, Some(&records))
                        .unwrap_err()
                        .to_string().contains("不属于本次捕获"));
                }
                let mut ordinary_store =
                    super::super::history::HashReader::captured(&capture, None);
                let ordinary = load_for_history(&tx, &mut ordinary_store, g).unwrap();
                assert!(
                    ordinary.native.is_none(),
                    "普通 captured reader 不能取得 memo"
                );
                WIRE_PROJECTIONS.with(|calls| calls.set(0));
                for cut in 1..=6 {
                    let oracle = at_cut(&mut s, Layout::Sorted, &stored, cut).unwrap();
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
        if include_legacy_negative_cases {
            let tx = conn.transaction().unwrap();
            tx.execute("UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT MIN(rowid) FROM s_content_objects)",[]).unwrap();
            let mut s = content_store::TxStore::new(&tx).unwrap();
            assert!(load(&tx, &mut s, 3)
                .unwrap_err()
                .contains("不可变内容根不符"));
            drop(s);
            tx.rollback().unwrap();
            let tx = conn.transaction().unwrap();
            let mut s = content_store::TxStore::new(&tx).unwrap();
            let stored = load(&tx, &mut s, 3).unwrap();
            let mut wire = value_ref(&mut s, stored[0].birth_wire.view());
            wire["batch_id"] = json!(format!("batch3-{}", "f".repeat(64)));
            let bad = sc::write_json_view(&mut s, aj::JsonView::Plain(&wire)).unwrap();
            tx.execute(
                "UPDATE s_content_objects SET birth_wire_root=?1 WHERE rowid=(SELECT MIN(rowid) FROM s_content_objects)",
                params![bad.digest.as_slice()],
            ).unwrap();
            assert!(load(&tx, &mut s, 3)
                .unwrap_err()
                .contains("出生公开元数据与独立记录不同"));
        }
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
    fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        Ok(match self {
            Self::Shape(v) => Self::Shape(*v),
            Self::Typed { kind, fact_key } => Self::Typed {
                kind: kind.try_clone_admitted(store.codec_work())?,
                fact_key: fact_key.try_clone_admitted(store.codec_work())?,
            },
        })
    }
}
impl Entry {
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub fn try_clone(&self) -> Result<Self, String> {
        self.try_clone_admitted(&super::audit_store::NativeAdmission(None)).map_err(Into::into)
    }
    pub fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, PublishError> {
        let text = |v: &TextValue| v.try_clone_admitted(store.codec_work());
        Ok(Self {
            id: text(&self.id)?,
            revision: self.revision,
            kind: text(&self.kind)?,
            slot: self.slot.try_clone_admitted(store)?,
            generated: self.generated.clone(),
            internal: self.internal.clone(),
            immutable: self.immutable.clone(),
            birth_wire: store.native_ref_clone(&self.birth_wire).map_err(publish_err)?,
            life: LifeOwner {
                first_known_generation: self.life.first_known_generation,
                first_known_cut: text(&self.life.first_known_cut)?,
                published_generation: self.life.published_generation,
                withdrawn_generation: self.life.withdrawn_generation,
                withdrawal_reason: self.life.withdrawal_reason.as_ref().map(text).transpose()?,
                superseded_by: self.life.superseded_by.as_ref().map(text).transpose()?,
                batch: match &self.life.batch {
                    BatchOwner::CurrentBatch => BatchOwner::CurrentBatch,
                    BatchOwner::Existing(v) => BatchOwner::Existing(text(v)?),
                },
            },
        })
    }
}
fn root_ref_bytes(s: &mut (impl CodecRead + ?Sized), bytes: &[u8]) -> Result<RefValue, PublishError> {
    let digest: NodeDigest = bytes.try_into().map_err(|_| publish_err("根不是32字节"))?;
    s.content_ref_view(sc::FixedRoot::new(&digest).as_str())
        .map_err(publish_err)
}
fn root_header_bytes(s: &mut (impl CodecRead + ?Sized), bytes: &[u8]) -> Result<Built, PublishError> {
    let r = root_ref_bytes(s, bytes)?;
    Ok(Built {
        digest: r.digest().map_err(publish_err)?,
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
    ) -> Result<Self, PublishError> {
        s.lookup_fields_ref(r, names).map(Self).map_err(publish_err)
    }
    fn get(&self, s: &(impl CodecRead + ?Sized), k: &str) -> Result<RefValue, PublishError> {
        self.0
            .get_admitted(s, k)
            .map_err(publish_err)?
            .ok_or_else(|| publish_err(format!("原件缺字段{k}")))
    }
    fn text(&self, s: &mut (impl CodecRead + ?Sized), k: &str) -> Result<TextValue, PublishError> {
        let r = self.get(s, k)?;
        bj::read_text(s, r.view()).map_err(publish_err)
    }
    fn integer(&self, s: &mut (impl CodecRead + ?Sized), k: &str) -> Result<i64, PublishError> {
        let r = self.get(s, k)?;
        bj::render_json(s, r.view())?
            .view()
            .as_i64()
            .ok_or_else(|| publish_err(format!("{k}不是i64")))
    }
    fn equal(
        &self,
        s: &(impl CodecRead + ?Sized),
        k: &str,
        o: &Self,
        j: &str,
    ) -> Result<(), String> {
        if self.get(s, k)? != o.get(s, j)? {
            Err(err(format!("{k}/{j}内容边不同")))
        } else {
            Ok(())
        }
    }
    fn slot(&self, s: &mut (impl CodecStore + ?Sized), kind: &str) -> Result<Slot, PublishError> {
        if kind == SHAPE {
            Ok(Slot::Shape([
                self.integer(s, "window_start")?,
                self.integer(s, "window_mid")?,
                self.integer(s, "window_end")?,
            ]))
        } else {
            Ok(Slot::Typed {
                kind: s.native_text(kind)?,
                fact_key: self.text(s, "fact_key")?,
            })
        }
    }
    fn immutable(&self, s: &mut (impl CodecStore + ?Sized), shape: bool) -> Result<Built, PublishError> {
        let names = if shape {
            SHAPE_IMMUTABLE
        } else {
            TYPED_IMMUTABLE
        };
        let mut fields = [("", sc::ValueView::BatchSelf); 12];
        for (i, name) in names.iter().enumerate() {
            let child = self.get(s, name)?;
            if child.kind() == Kind::Null {
                return Err(publish_err("不可变字段不得为空"));
            }
            fields[i] = (*name, sc::ValueView::Edge(child.digest().map_err(publish_err)?));
        }
        s.write_value_view(sc::ValueView::Object(&fields[..names.len()]))
            .map_err(publish_err)
    }
    fn bind_json_text(
        &self,
        s: &mut (impl CodecRead + ?Sized),
        json: &str,
        value: &str,
    ) -> Result<(), String> {
        let input = self.get(s, json)?;
        let parsed = s
            .lookup_ref(input.view(), &[PathStep::JsonText])
            .map_err(err)?;
        if parsed != self.get(s, value)? {
            return Err(err("内部JSON视图与值边不同"));
        }
        Ok(())
    }
}
fn built_ref(s: &(impl CodecRead + ?Sized), b: &Built) -> Result<RefValue, PublishError> {
    s.native_built_ref(b).map_err(publish_err)
}

fn field_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_field_ref(r, k).map_err(err)
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

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, Retained};

    retained_struct!(Entry {
        id,
        revision,
        kind,
        slot,
        generated,
        internal,
        immutable,
        birth_wire,
        life
    });
    impl Retained for Slot {
        fn heap_bytes(&self) -> usize {
            match self {
                Self::Shape(_) => 0,
                Self::Typed { kind, fact_key } => {
                    kind.heap_bytes().saturating_add(fact_key.heap_bytes())
                }
            }
        }
    }
    impl Retained for NativeRank {
        fn heap_bytes(&self) -> usize {
            match self {
                Self::Shape(_) => 0,
                Self::Typed(k, f, i, _) => crate::v3::audit_retention::sum([
                    k.heap_bytes(),
                    f.heap_bytes(),
                    i.heap_bytes(),
                ]),
            }
        }
    }
    impl Retained for NativeArc<HistoryEvents> {
        fn heap_bytes(&self) -> usize {
            self.allocation_bytes()
                .saturating_add(self.as_ref().heap_bytes())
        }
    }
    impl Retained for HistoryEvents {
        fn heap_bytes(&self) -> usize {
            let Self {
                by_id,
                births,
                deaths,
                ranks,
                #[cfg(test)]
                    initial_entries: _,
                #[cfg(test)]
                    sort_passes: _,
            } = self;
            crate::v3::audit_retention::sum([
                by_id.heap_bytes(),
                births.heap_bytes(),
                deaths.heap_bytes(),
                ranks.heap_bytes(),
            ])
        }
    }
    impl Retained for HistorySweep {
        fn heap_bytes(&self) -> usize {
            let Self {
                events,
                active,
                previous: _,
                projection,
                retained: _,
                #[cfg(test)]
                    visits: _,
            } = self;
            crate::v3::audit_retention::sum([
                events.heap_bytes(),
                active.heap_bytes(),
                projection.heap_bytes(),
            ])
        }
    }
    impl Retained for OwnerProjection {
        fn heap_bytes(&self) -> usize {
            let Self {
                wires,
                active,
                withdrawn,
                previous_cut: _,
                #[cfg(test)]
                    visits: _,
            } = self;
            crate::v3::audit_retention::sum([
                wires.heap_bytes(),
                active.heap_bytes(),
                withdrawn.heap_bytes(),
            ])
        }
    }
}

fn publish_err(error: impl Into<PublishError>) -> PublishError {
    error.into().context("ObjectIndexInvalid：")
}
