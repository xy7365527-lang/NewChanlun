//! #1394 C：对象独立出生/撤回索引与历史投影。变化判据复用 object_diff::plan。
//! 保存内部记录、生成态、不可变内容和出生 wire 四根；所有来源/载荷保持内容边。
//! 本层证明索引与这些原件互相绑定；原件是否由真实源按正确业务规则产生仍归来源审计。
use super::{
    generation::ProducedObject,
    object_records::{self, BatchRef, BorrowedObject, PublicationLife},
};
use crate::{canonical_json, content_store, object_diff};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, replace_batch_placeholder, PathStep},
    Built, ContentRef, Kind, NodeDigest, NodeSource, NodeStore, Value as V,
};
use rusqlite::{params, Transaction};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

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
fn field(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[f(k)]).map_err(err)
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
fn scalar(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<Value, String> {
    let r = field(s, r, k)?;
    if !matches!(r.kind, Kind::Number | Kind::String | Kind::Null) {
        return Err(err("预期有界标量"));
    }
    let mut out = Vec::new();
    sc::render_canonical(s, &r, &mut out).map_err(err)?;
    newchan_rust::session_protocol::strict_json(&out)
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
fn root(s: &mut dyn NodeSource, b: Vec<u8>) -> Result<Built, String> {
    let d: NodeDigest = b.try_into().map_err(|_| err("根不是32字节"))?;
    let r = sc::content_ref_of(s, &sc::root_string(&d)).map_err(err)?;
    sc::verify(s, &r).map_err(err)?;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    Shape([i64; 3]),
    Typed { kind: String, fact_key: String },
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
#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub id: String,
    pub revision: i64,
    pub kind: String,
    pub slot: Slot,
    pub generated: Built,
    pub internal: Built,
    pub immutable: Built,
    pub birth_wire: ContentRef,
    pub life: PublicationLife,
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
fn slot(s: &mut dyn NodeSource, r: &ContentRef, kind: &str) -> Result<Slot, String> {
    if kind == SHAPE {
        Ok(Slot::Shape([
            integer(s, r, "window_start")?,
            integer(s, r, "window_mid")?,
            integer(s, r, "window_end")?,
        ]))
    } else {
        Ok(Slot::Typed {
            kind: kind.into(),
            fact_key: text(s, r, "fact_key")?,
        })
    }
}
fn immutable(s: &mut dyn NodeStore, r: &ContentRef, is_shape: bool) -> Result<Built, String> {
    let names = if is_shape {
        SHAPE_IMMUTABLE
    } else {
        TYPED_IMMUTABLE
    };
    let mut fields = Vec::new();
    for name in names {
        let child = field(s, r, name)?;
        if child.kind == Kind::Null {
            return Err(err("不可变字段不得为空"));
        }
        fields.push(((*name).into(), V::Edge(digest(&child)?)));
    }
    sc::write_value(s, &V::Object(fields)).map_err(err)
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
fn candidate(s: &mut dyn NodeStore, o: &ProducedObject, g: i64) -> Result<Entry, String> {
    let generated = o.generated().clone();
    sc::verify(s, &generated.content_ref()).map_err(err)?;
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
    let birth_wire = o.wire_template(s, &life, g).map_err(err)?.content_ref();
    let immutable = immutable(s, &generated.content_ref(), slot.is_shape())?;
    Ok(Entry {
        id: o.object_id().into(),
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
/// 返回本代模板：新生维持08，已有记录只借出生字段并改写小生命周期元数据。
fn wire_at(s: &mut dyn NodeStore, e: &Entry, g: i64) -> Result<ContentRef, String> {
    if e.life.first_known_generation > g {
        return Err(err("对象尚未出生"));
    }
    let mut life = e.life.clone();
    if life.withdrawn_generation.is_some_and(|v| v > g) {
        life.withdrawn_generation = None;
        life.withdrawal_reason = None;
        life.superseded_by = None;
    }
    let saved = life.batch.clone();
    if life.first_known_generation == g {
        life.batch = BatchRef::CurrentBatch;
    }
    let mut wire = object_records::reproject_stored_wire(s, &e.birth_wire, &life, g)
        .map_err(err)?
        .content_ref();
    if e.life.first_known_generation == g {
        if let BatchRef::Existing(batch) = saved {
            wire = replace_batch_placeholder(s, &wire, &[f("batch_id")], &batch).map_err(err)?;
        }
    }
    Ok(wire)
}
fn array(s: &mut dyn NodeStore, refs: &[ContentRef]) -> Result<Built, String> {
    let xs = refs
        .iter()
        .map(|r| Ok(V::Edge(digest(r)?)))
        .collect::<Result<_, String>>()?;
    sc::write_template(s, &V::Array(xs)).map_err(err)
}
/// 完整历史索引按目标代投影。Delta 出生序保留行插入序；活动/撤回各遵旧公开主序。
pub(super) fn at_cut(
    s: &mut dyn NodeStore,
    index: &[Entry],
    g: i64,
) -> Result<(Built, Built, Built), String> {
    at_cut_metered(s, index, g, None)
}
pub(super) fn at_cut_metered(
    s: &mut dyn NodeStore,
    index: &[Entry],
    g: i64,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(Built, Built, Built), String> {
    let mut active = index
        .iter()
        .filter(|e| {
            e.life.first_known_generation <= g && e.life.withdrawn_generation.is_none_or(|d| d > g)
        })
        .collect::<Vec<_>>();
    let mut dead = index
        .iter()
        .filter(|e| e.life.withdrawn_generation.is_some_and(|d| d <= g))
        .collect::<Vec<_>>();
    active.sort_by(|a, b| ordering(a, b, false));
    dead.sort_by(|a, b| ordering(a, b, true));
    let births = index
        .iter()
        .filter(|e| e.life.first_known_generation == g)
        .collect::<Vec<_>>();
    let mut collect = |items: Vec<&Entry>| -> Result<Built, String> {
        let refs = items
            .into_iter()
            .map(|e| {
                if let Some(b) = budget {
                    b.record().map_err(err)?;
                }
                wire_at(s, e, g)
            })
            .collect::<Result<Vec<_>, _>>()?;
        array(s, &refs)
    };
    Ok((collect(active)?, collect(dead)?, collect(births)?))
}
pub(super) fn plan(
    s: &mut dyn NodeStore,
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
        if !ids.insert(e.id.clone()) {
            return Err(err("生成集重复对象身份"));
        }
    }
    let mut order = (0..old.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| ordering(&old[*a], &old[*b], false));
    let old_keys = order
        .iter()
        .map(|i| {
            let e = &old[*i];
            object_diff::Existing {
                key: object_diff::Key {
                    object_id: e.id.clone(),
                    slot: e.slot.key(),
                    immutable: object_diff::Ready(e.immutable.digest),
                },
                active: e.life.withdrawn_generation.is_none(),
            }
        })
        .collect::<Vec<_>>();
    let keys = new
        .iter()
        .map(|e| object_diff::Key {
            object_id: e.id.clone(),
            slot: e.slot.key(),
            immutable: object_diff::Ready(e.immutable.digest),
        })
        .collect::<Vec<_>>();
    let changes = object_diff::plan(&keys, &old_keys)?;
    let mut index = old.to_vec();
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
                        .clone(),
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
        .map(|i| new[i].clone())
        .collect::<Vec<_>>();
    index.extend(births.iter().cloned());
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

fn equal_field(
    s: &mut dyn NodeStore,
    a: &ContentRef,
    ka: &str,
    b: &ContentRef,
    kb: &str,
) -> Result<(), String> {
    if field(s, a, ka)? != field(s, b, kb)? {
        return Err(err(format!("{ka}/{kb}内容边不同")));
    }
    Ok(())
}
fn bind_json_text(
    s: &mut dyn NodeStore,
    internal: &ContentRef,
    json: &str,
    value: &str,
) -> Result<(), String> {
    let parsed = lookup_path(s, internal, &[f(json), PathStep::JsonText]).map_err(err)?;
    if parsed != field(s, internal, value)? {
        return Err(err("内部JSON视图与值边不同"));
    }
    Ok(())
}
/// 对持久三种记录逐字段对拍，不展开 payload/input_refs/source_coords 大载荷。
fn bind_originals(s: &mut dyn NodeStore, e: &Entry) -> Result<(), String> {
    let r = e.generated.content_ref();
    let i = e.internal.content_ref();
    let w = &e.birth_wire;
    let shape = e.slot.is_shape();
    if r.kind != Kind::Object
        || r.extent != Some(if shape { 15 } else { 10 })
        || i.kind != Kind::Object
        || i.extent != Some(if shape { 17 } else { 13 })
    {
        return Err(err("内部/生成态字段集不符"));
    }
    for key in [
        "object_id",
        "object_revision",
        "kind",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ] {
        equal_field(s, &r, key, &i, key)?;
    }
    for key in if shape {
        &SHAPE_IMMUTABLE[1..]
    } else {
        &TYPED_IMMUTABLE[1..]
    } {
        if *key == "comparisons_json" {
            continue;
        }
        equal_field(s, &r, key, &i, key)?;
    }
    if text(s, &r, "object_id")? != e.id
        || integer(s, &r, "object_revision")? != e.revision
        || text(s, &r, "kind")? != e.kind
        || slot(s, &r, &e.kind)? != e.slot
    {
        return Err(err("对象索引与生成原件不符"));
    }
    for key in ["first_known_generation", "published_generation"] {
        if integer(s, &r, key)? != e.life.first_known_generation {
            return Err(err("出生生成代号不符"));
        }
    }
    if text(s, &r, "first_known_cut")? != e.life.first_known_cut
        || e.life.published_generation != e.life.first_known_generation
    {
        return Err(err("出生cut/发布代不符"));
    }
    for (json, value) in [
        ("input_refs_json", "input_refs"),
        ("source_coords_json", "source_coords"),
    ] {
        bind_json_text(s, &i, json, value)?;
    }
    equal_field(s, &i, "source_coords", w, "source_coords")?;
    if shape {
        for key in ["branch", "dir_ab", "dir_bc"] {
            equal_field(s, &i, key, w, key)?;
        }
        for key in ["window_start", "window_mid", "window_end"] {
            if text(s, w, key)? != integer(s, &i, key)?.to_string() {
                return Err(err("窗口公开投影不符"));
            }
        }
        let comparison = field(s, &i, "comparisons")?;
        let mut out = Vec::new();
        sc::render_canonical(s, &comparison, &mut out).map_err(err)?;
        if text(s, &r, "comparisons_json")?.as_bytes() != out {
            return Err(err("comparisons文本与内部值不符"));
        }
        equal_field(s, &i, "comparisons", w, "comparisons")?;
        let generated_refs = field(s, &i, "input_refs")?;
        let wire_refs = field(s, w, "input_refs")?;
        bind_shape_refs(s, &generated_refs, &wire_refs)?;
    } else {
        bind_json_text(s, &i, "payload_json", "payload")?;
        for key in ["fact_key", "payload", "input_refs"] {
            equal_field(s, &i, key, w, key)?;
        }
    }
    if text(s, w, "object_id")? != e.id
        || text(s, w, "kind")? != e.kind
        || text(s, w, "object_revision")? != e.revision.to_string()
    {
        return Err(err("出生公开身份不符"));
    }
    if immutable(s, &r, shape)? != e.immutable {
        return Err(err("不可变内容根不符"));
    }
    if wire_at(s, e, e.life.first_known_generation)? != e.birth_wire {
        return Err(err("出生公开元数据与独立记录不同"));
    }
    Ok(())
}
fn bind_shape_refs(
    s: &mut dyn NodeStore,
    generated: &ContentRef,
    wire: &ContentRef,
) -> Result<(), String> {
    if generated.kind != Kind::Array
        || generated.extent != Some(3)
        || wire.kind != Kind::Array
        || wire.extent != Some(3)
    {
        return Err(err("形态来源不是三组"));
    }
    for n in 0..3 {
        let a = lookup_path(s, generated, &[PathStep::Index(n)]).map_err(err)?;
        let b = lookup_path(s, wire, &[PathStep::Index(n)]).map_err(err)?;
        if a.extent != Some(3)
            || b.extent != Some(3)
            || text(s, &b, "merged_index")? != integer(s, &a, "merged_index")?.to_string()
        {
            return Err(err("合并序号公开投影不符"));
        }
        for key in ["raw_refs", "dependency_refs"] {
            let aa = field(s, &a, key)?;
            let bb = field(s, &b, key)?;
            if aa.kind != Kind::Array || bb.kind != Kind::Array || aa.extent != bb.extent {
                return Err(err("来源数组长度不符"));
            }
            for k in 0..aa.extent.ok_or_else(|| err("来源缺长度"))? {
                let ar = lookup_path(s, &aa, &[PathStep::Index(k)]).map_err(err)?;
                let br = lookup_path(s, &bb, &[PathStep::Index(k)]).map_err(err)?;
                if ar.extent != Some(8) || br.extent != Some(8) {
                    return Err(err("来源字段集不符"));
                }
                for field_name in [
                    "identity_key",
                    "payload_hash",
                    "receipt_id",
                    "event_id",
                    "input_revision",
                    "source_coord",
                ] {
                    equal_field(s, &ar, field_name, &br, field_name)?;
                }
                for field_name in ["revision", "seq"] {
                    if text(s, &br, field_name)? != integer(s, &ar, field_name)?.to_string() {
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
) -> Result<Vec<Entry>, String> {
    let mut stmt=tx.prepare("SELECT object_id,object_revision,kind,fact_key,window_start,window_mid,window_end,generated_root,internal_root,immutable_root,birth_wire_root,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects ORDER BY rowid").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(err)? {
        let kind: String = row.get(2).map_err(err)?;
        let fact_key: Option<String> = row.get(3).map_err(err)?;
        let coords = [
            row.get::<_, Option<i64>>(4).map_err(err)?,
            row.get(5).map_err(err)?,
            row.get(6).map_err(err)?,
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
                kind: kind.clone(),
                fact_key: fact_key.ok_or_else(|| err("缺typed槽位"))?,
            }
        };
        let e = Entry {
            id: row.get(0).map_err(err)?,
            revision: row.get(1).map_err(err)?,
            kind,
            slot,
            generated: root(s, row.get(7).map_err(err)?)?,
            internal: root(s, row.get(8).map_err(err)?)?,
            immutable: root(s, row.get(9).map_err(err)?)?,
            birth_wire: root(s, row.get(10).map_err(err)?)?.content_ref(),
            life: PublicationLife {
                batch: BatchRef::Existing(row.get(11).map_err(err)?),
                first_known_generation: row.get(12).map_err(err)?,
                first_known_cut: row.get(13).map_err(err)?,
                published_generation: row.get(14).map_err(err)?,
                withdrawn_generation: row.get(15).map_err(err)?,
                withdrawal_reason: row.get(16).map_err(err)?,
                superseded_by: row.get(17).map_err(err)?,
            },
        };
        if e.life.first_known_generation > latest
            || e.life.withdrawn_generation.is_some_and(|d| d > latest)
        {
            return Err(err("出生代超过已发布代"));
        }
        bind_originals(s, &e)?;
        wire_at(s, &e, latest)?;
        out.push(e);
    }
    for e in &out {
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
    s: &mut dyn NodeStore,
    p: &Plan,
    batch: &str,
) -> Result<(), String> {
    for e in &p.births {
        if e.life.first_known_generation != p.generation || e.life.batch != BatchRef::CurrentBatch {
            return Err(err("新生上下文不符"));
        }
        let wire =
            replace_batch_placeholder(s, &e.birth_wire, &[f("batch_id")], batch).map_err(err)?;
        sc::verify(s, &wire).map_err(err)?;
        let (fact, w) = match &e.slot {
            Slot::Shape(w) => (None, Some(*w)),
            Slot::Typed { fact_key, .. } => (Some(fact_key.as_str()), None),
        };
        tx.execute("INSERT INTO s_content_objects VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?13,NULL,NULL,NULL)",params![e.id,e.revision,e.kind,fact,w.map(|v|v[0]),w.map(|v|v[1]),w.map(|v|v[2]),e.generated.digest.as_slice(),e.internal.digest.as_slice(),e.immutable.digest.as_slice(),digest(&wire)?.as_slice(),batch,p.generation,format!("cut-{}",p.generation)]).map_err(err)?;
    }
    for w in &p.withdrawals {
        let changed=tx.execute("UPDATE s_content_objects SET withdrawn_generation=?1,withdrawal_reason=?2,superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",params![p.generation,w["reason"].as_str(),w["superseded_by"].as_str(),w["object_id"].as_str()]).map_err(err)?;
        if changed != 1 {
            return Err(err("撤回身份缺失或已撤回"));
        }
    }
    Ok(())
}

/// 独立生命周期导出的本代变化集合，顺序沿上一代活动主序。
pub(super) fn changes_at(index: &[Entry], g: i64) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut dead = index
        .iter()
        .filter(|e| e.life.withdrawn_generation == Some(g))
        .collect::<Vec<_>>();
    dead.sort_by(|a, b| ordering(a, b, false));
    let mut withdrawals = Vec::new();
    let mut replaces = Vec::new();
    for e in dead {
        let reason = e
            .life
            .withdrawal_reason
            .as_deref()
            .ok_or_else(|| err("撤回缺原因"))?;
        withdrawals.push(raw_withdrawal(e, reason, e.life.superseded_by.as_deref()));
        if let Some(id) = &e.life.superseded_by {
            replaces.push(json!({"new_object_id":id,"old_object_id":e.id}));
        }
    }
    Ok((withdrawals, replaces))
}
/// 按该代真实候选序复用同一diff判据，再对拍独立生命周期；不假定候选槽位唯一。
pub(super) fn verify_generated_members(
    s: &mut dyn NodeStore,
    index: &[Entry],
    state: &ContentRef,
    g: i64,
) -> Result<(), String> {
    let by_id = index
        .iter()
        .filter(|e| e.life.first_known_generation <= g)
        .map(|e| (e.id.as_str(), e))
        .collect::<BTreeMap<_, _>>();
    let objects = field(s, state, "objects")?;
    if objects.kind != Kind::Array {
        return Err(err("生成对象集不是数组"));
    }
    let mut seen = BTreeSet::new();
    let mut new = Vec::new();
    for n in 0..objects.extent.ok_or_else(|| err("生成集缺长度"))? {
        let object = lookup_path(s, &objects, &[PathStep::Index(n)]).map_err(err)?;
        let id = text(s, &object, "object_id")?;
        if !seen.insert(id.clone()) {
            return Err(err("生成集重复对象身份"));
        }
        let e = by_id
            .get(id.as_str())
            .ok_or_else(|| err("生成对象不在独立身份史内"))?;
        if object.kind != Kind::Object
            || object.extent != Some(if e.slot.is_shape() { 15 } else { 10 })
            || integer(s, &object, "first_known_generation")? != g
            || integer(s, &object, "published_generation")? != g
            || text(s, &object, "first_known_cut")? != format!("cut-{g}")
        {
            return Err(err("生成对象字段/当代元数据不符"));
        }
        if immutable(s, &object, e.slot.is_shape())? != e.immutable {
            return Err(err("生成集同身份内容与独立原件不同"));
        }
        if e.life.first_known_generation == g && object != e.generated.content_ref() {
            return Err(err("出生原件不等于真实首次生成记录"));
        }
        new.push(object_diff::Key {
            object_id: id,
            slot: e.slot.key(),
            immutable: object_diff::Ready(e.immutable.digest),
        });
    }
    let mut before = index
        .iter()
        .filter(|e| e.life.first_known_generation < g)
        .collect::<Vec<_>>();
    before.sort_by(|a, b| ordering(a, b, false));
    let old = before
        .iter()
        .map(|e| object_diff::Existing {
            key: object_diff::Key {
                object_id: e.id.clone(),
                slot: e.slot.key(),
                immutable: object_diff::Ready(e.immutable.digest),
            },
            active: e.life.withdrawn_generation.is_none_or(|d| d >= g),
        })
        .collect::<Vec<_>>();
    let changes = object_diff::plan(&new, &old)?;
    let expected_births = index
        .iter()
        .filter(|e| e.life.first_known_generation == g)
        .map(|e| e.id.as_str())
        .collect::<Vec<_>>();
    if changes
        .births
        .iter()
        .map(|i| new[*i].object_id.as_str())
        .collect::<Vec<_>>()
        != expected_births
    {
        return Err(err("真实出生顺序与独立索引不同"));
    }
    let (expected_withdrawals, expected_replaces) = changes_at(index, g)?;
    let mut withdrawals = Vec::new();
    for w in changes.withdrawals {
        let e = before[w.old_index];
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
        withdrawals.push(raw_withdrawal(e, reason, sup));
    }
    let replaces=changes.replaces.iter().map(|r|json!({"new_object_id":new[r.new_index].object_id,"old_object_id":before[r.old_index].id})).collect::<Vec<_>>();
    if withdrawals != expected_withdrawals || replaces != expected_replaces {
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
    fn value(s: &mut dyn NodeSource, r: &ContentRef) -> Value {
        let mut b = Vec::new();
        sc::render_canonical(s, r, &mut b).unwrap();
        serde_json::from_slice(&b).unwrap()
    }
    fn legacy_value(s: &mut dyn NodeSource, e: &Entry) -> Value {
        let mut v = value(s, &e.generated.content_ref());
        v["batch_id"] = match &e.life.batch {
            BatchRef::Existing(b) => json!(b),
            BatchRef::CurrentBatch => Value::Null,
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
                all_events: &raw,
                effective: &effective,
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
                    all_events: &previous_raw,
                    effective: &earlier_effective,
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
                .map(|e| (e.id.clone(), legacy_value(&mut s, e)))
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
                assert_eq!(value(&mut s, &actual), expected);
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
                    .position(|e| generated.objects.iter().any(|o| o.object_id() == e.id))
                    .expect("应有沿用身份");
                let mut wrong = old.clone();
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
