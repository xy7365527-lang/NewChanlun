//! #1394 C §5.2–5.4：绑定同 cut 来源原件、typed/CC006 身份和见证。
//! 只读事实及其引用，不重新运行包含、分型或新笔判定。原始账必须先经 read_raw_events 核验。
use super::audit_store::RefValue;
use super::audit_store::{CodecRead, CodecStore};
use super::business_json::BusinessMap;
use super::business_json::{self as bj, RawEventsView};
use crate::{
    canonical_json,
    content_producer::{
        self, Cc004Producer, ShapeIdentityView, CC004_STEP_KIND, CC005_GROUP_KIND,
        CC006_SHAPE_KIND, CC007_FRACTAL_KIND, CC054_KNOWLEDGE_KIND,
    },
    read_effective_events, stored_source_coord, tb02a, tb02a_facts, tb02b,
};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    Built, ContentRef, Kind, NodeDigest, NodeSink, NodeSource, NodeStore, Value as V,
};
use serde_json::{json, Value};
#[path = "source_prefix.rs"]
mod source_prefix;
use source_prefix::{NativeSourceMemo, ParseDomain, SourceCoords};
#[cfg(test)]
pub(super) fn reset_native_source_prefix_stats() {
    source_prefix::reset_stats();
}
#[cfg(test)]
pub(super) fn native_source_prefix_stats() -> Option<[u64; 4]> {
    source_prefix::stats()
}
fn err(e: impl std::fmt::Display) -> String {
    format!("SourceAuditInvalid：{e}")
}
fn f(k: &str) -> PathStep {
    PathStep::Field(k.into())
}
// Only immutable Native originals may enter these maps; no cut/business verdict is retained.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NativeFieldRefKey {
    format: String,
    root: String,
    kind: Kind,
    extent: Option<u64>,
    digest: NodeDigest,
}
impl NativeFieldRefKey {
    fn new(r: sc::RefView<'_>) -> Result<Self, sc::CodecError> {
        Ok(Self {
            format: r.format().into(),
            root: r.root().into(),
            kind: r.kind(),
            extent: r.extent(),
            digest: r.digest()?,
        })
    }
}
#[derive(Default)]
struct NativeScalarFields {
    values: std::collections::HashMap<NativeFieldRefKey, Value>,
    decodes: u64,
    hits: u64,
}
struct NativeFieldMemo<'capture> {
    capture: &'capture sc::AuditCapture,
    fields: std::collections::HashMap<
        (NativeFieldRefKey, Vec<String>),
        std::collections::BTreeMap<String, ContentRef>,
    >,
    scalars: NativeScalarFields,
    field_decodes: u64,
    field_hits: u64,
}
impl<'capture> NativeFieldMemo<'capture> {
    fn new(capture: &'capture sc::AuditCapture) -> Self {
        Self {
            capture,
            fields: Default::default(),
            scalars: Default::default(),
            field_decodes: 0,
            field_hits: 0,
        }
    }
    fn qualified(&self, r: sc::RefView<'_>) -> Result<bool, sc::CodecError> {
        // Root presence is insufficient; original public validation includes format/root/header
        // and the complete immutable closure. Overlay-only records never obtain a handle.
        if let sc::RefView::Normal(original) = r {
            if self.capture.contains(&original.digest()?) {
                self.capture.verify(original, sc::Mode::Public)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn clear(&mut self) {
        self.fields.clear();
        self.scalars.values.clear();
    }
}
// Local adapter only; ordinary/Owned use the original scalar decoder.
trait SourceFieldStore: CodecStore {
    fn decoded_scalar(&mut self, r: sc::RefView<'_>) -> Result<bj::JsonOwner, String> {
        scalar_value(self, r)
    }
}
impl SourceFieldStore for dyn NodeStore + '_ {}
impl SourceFieldStore for sc::MemoryNodes {}
// Preserve the exact original layout: Object / Owned work collections must not grow.
struct SourceFields<'names>(super::audit_store::RefFields<'names>);
impl<'names> SourceFields<'names> {
    fn read_subset(
        s: &mut (impl CodecStore + ?Sized),
        r: sc::RefView<'_>,
        names: &[&'names str],
    ) -> Result<Self, String> {
        s.lookup_subset_ref(r, names).map(Self).map_err(err)
    }
    fn get(&self, k: &str) -> Result<RefValue, String> {
        self.0.get(k).ok_or_else(|| err(format!("原件缺字段{k}")))
    }
    fn scalar(
        &self,
        s: &mut (impl SourceFieldStore + ?Sized),
        k: &str,
    ) -> Result<bj::JsonOwner, String> {
        s.decoded_scalar(self.get(k)?.view())
    }
    fn text(
        &self,
        s: &mut (impl SourceFieldStore + ?Sized),
        k: &str,
    ) -> Result<bj::TextValue, String> {
        let value = self.scalar(s, k)?;
        let view = value.view();
        let text = view.as_str().ok_or_else(|| err(format!("{k}不是文本")))?;
        bj::TextValue::from_str(text, s.allocation_budget().as_ref())
    }
    fn integer(&self, s: &mut (impl SourceFieldStore + ?Sized), k: &str) -> Result<i64, String> {
        self.scalar(s, k)?
            .view()
            .as_i64()
            .ok_or_else(|| err(format!("{k}不是i64")))
    }
}

fn field<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<RefValue, String> {
    field_ref(s, r.into(), k)
}
fn scalar<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<bj::JsonOwner, String> {
    let r = field(s, r, k)?;
    scalar_value(s, r.view())
}
fn scalar_value<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
) -> Result<bj::JsonOwner, String> {
    let r = r.into();
    if !matches!(
        r.kind(),
        Kind::Number | Kind::String | Kind::Bool | Kind::Null
    ) {
        return Err(err("预期标量"));
    }
    bj::render_json(s, r)
}
/// 仅当前 part/payload 的固定小字段；Some 由 ArrayAuditSource 关闭批读。
fn local_fields<'a, 'names>(
    s: &mut (impl CodecStore + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    names: &[&'names str],
) -> Result<Option<SourceFields<'names>>, String> {
    if s.audit_full_array_scan() {
        SourceFields::read_subset(s, r.into(), names)
            .map(Some)
            .map_err(err)
    } else {
        Ok(None)
    }
}
fn text<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<bj::TextValue, String> {
    let r = field(s, r, k)?;
    text_value(s, r.view(), k)
}
fn text_value<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<bj::TextValue, String> {
    let value = scalar_value(s, r)?;
    let v = value.view();
    let text = v.as_str().ok_or_else(|| err(format!("{k}不是文本")))?;
    bj::TextValue::from_str(text, s.allocation_budget().as_ref())
}
fn number<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<i64, String> {
    scalar(s, r, k)?
        .view()
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是整数")))
}
fn decimal(v: &str) -> Result<i64, String> {
    newchan_rust::session_protocol::parse_canonical_i64(v, "source coordinate").map_err(err)
}
fn digest<'a>(r: impl Into<sc::RefView<'a>>) -> Result<NodeDigest, String> {
    r.into().digest().map_err(err)
}
fn built<'a>(r: impl Into<sc::RefView<'a>>) -> Result<Built, String> {
    let r = r.into();
    Ok(Built {
        digest: r.digest().map_err(err)?,
        kind: r.kind(),
        extent: r.extent(),
    })
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(n) => V::Number(sc::NumberToken::parse(&n.to_string()).map_err(err)?),
        Value::Array(a) => V::Array(a.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(o) => V::Object(
            o.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
fn hash(v: &Value) -> Result<NodeDigest, String> {
    Ok(sc::write_json_view(&mut HashOnly, v.into())
        .map_err(err)?
        .digest)
}
fn parsed_json(s: &(impl CodecRead + ?Sized), bytes: &[u8]) -> Result<bj::JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = s.allocation_budget() {
        return sc::capture_allocation::audit_json::OwnedJson::parse(bytes, &b)
            .map(bj::JsonOwner::Owned)
            .map_err(err);
    }
    newchan_rust::session_protocol::strict_json(bytes).map(bj::JsonOwner::Plain)
}
fn canonical_matches(
    s: &(impl CodecRead + ?Sized),
    v: sc::capture_allocation::audit_json::JsonView<'_>,
    bytes: &[u8],
) -> Result<bool, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = s.allocation_budget() {
        return sc::capture_allocation::audit_json::canonical_bytes(v, &b)
            .map(|out| &*out == bytes)
            .map_err(err);
    }
    match v {
        sc::capture_allocation::audit_json::JsonView::Plain(v) => {
            Ok(canonical_json(v).as_bytes() == bytes)
        }
        _ => Err(err("owned JSON组合缺实际账本")),
    }
}
fn exact<'a>(r: impl Into<sc::RefView<'a>>, kind: Kind, n: u64) -> Result<(), String> {
    let r = r.into();
    if r.kind() != kind || r.extent() != Some(n) {
        Err(err(format!("字段/成员形状不符：预期{kind:?}/{n}")))
    } else {
        Ok(())
    }
}
pub(super) fn elements<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
) -> Result<sc::CodecVec<RefValue>, String> {
    elements_ref(s, r.into())
}
/// 活期仅为本次完整来源审计；预算/期限 Some 强制原路径。
struct ArrayAuditSource<'a, 'borrow, 'source, 'capture> {
    source: &'a mut super::audit_store::StoreView<'borrow, 'source>,
    full_arrays: bool,
    deadline: Option<std::time::Instant>,
    fields: Option<&'a mut NativeFieldMemo<'capture>>,
}
impl NodeSource for ArrayAuditSource<'_, '_, '_, '_> {
    fn audit_full_array_scan(&self) -> bool {
        self.full_arrays && self.deadline.is_none()
    }
    fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        self.source.read(d)
    }
    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        self.source.checked_read(d)
    }
    fn before_path_index(&mut self, p: &[PathStep], i: u64) -> Result<(), sc::CodecError> {
        self.source.before_path_index(p, i)
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.source.max_render_bytes()
    }
}
impl NodeSink for ArrayAuditSource<'_, '_, '_, '_> {
    fn put(&mut self, d: NodeDigest, t: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.source.put(d, t, body)
    }
}

impl<'a, 'borrow, 'source, 'capture> ArrayAuditSource<'a, 'borrow, 'source, 'capture> {
    fn owned_role<T>(
        &mut self,
        operation: impl FnOnce(
            &mut super::audit_store::StoreView<'borrow, 'source>,
        ) -> Result<T, sc::CodecError>,
    ) -> Result<T, sc::CodecError> {
        let role = self.full_arrays && self.deadline.is_none();
        let previous = self.source.swap_codec_full_array_role(role);
        let result = operation(self.source);
        self.source.swap_codec_full_array_role(previous);
        result
    }
}
impl SourceFieldStore for ArrayAuditSource<'_, '_, '_, '_> {
    fn decoded_scalar(&mut self, r: sc::RefView<'_>) -> Result<bj::JsonOwner, String> {
        let eligible = self.full_arrays
            && !self.source.owned_codec_active()
            && self.deadline.is_none()
            && self.source.allocation_budget().is_none()
            && self.max_render_bytes().is_none()
            && self
                .fields
                .as_ref()
                .map(|m| m.qualified(r))
                .transpose()
                .map_err(err)?
                .unwrap_or(false);
        let key = if eligible {
            Some(NativeFieldRefKey::new(r).map_err(err)?)
        } else {
            None
        };
        if let Some(key) = &key {
            if let Some(memo) = self.fields.as_mut() {
                if let Some(value) = memo.scalars.values.get(key).cloned() {
                    memo.scalars.hits += 1;
                    return Ok(bj::JsonOwner::Plain(value));
                }
            }
        }
        let value = scalar_value(self, r)?;
        if let Some(key) = key {
            if let (Some(memo), bj::JsonOwner::Plain(parsed)) = (self.fields.as_mut(), &value) {
                memo.scalars.decodes += 1;
                memo.scalars.values.insert(key, parsed.clone());
            }
        }
        Ok(value)
    }
}
impl CodecRead for ArrayAuditSource<'_, '_, '_, '_> {
    fn content_ref_view(&mut self, r: &str) -> Result<RefValue, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.content_ref_view(r))
        } else {
            sc::content_ref_of(self, r).map(RefValue::Plain)
        }
    }
    fn lookup_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
    ) -> Result<RefValue, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_ref(r, p))
        } else {
            sc::path::lookup_path_refview(self, r, p).map(RefValue::Plain)
        }
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<super::audit_store::RefFields<'a>, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_fields_ref(r, n))
        } else {
            sc::path::lookup_fields_refview(self, r, n, false)
                .map(super::audit_store::RefFields::Plain)
        }
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<super::audit_store::RefFields<'a>, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_subset_ref(r, n))
        } else {
            let eligible = self.full_arrays
                && self.deadline.is_none()
                && self.source.allocation_budget().is_none()
                && self.max_render_bytes().is_none()
                && self
                    .fields
                    .as_ref()
                    .map(|m| m.qualified(r))
                    .transpose()?
                    .unwrap_or(false);
            let key = if eligible {
                Some((
                    NativeFieldRefKey::new(r)?,
                    n.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>(),
                ))
            } else {
                None
            };
            if let Some(key) = &key {
                if let Some(memo) = self.fields.as_mut() {
                    if let Some(fields) = memo.fields.get(key) {
                        let fields = fields.clone();
                        memo.field_hits += 1;
                        return Ok(super::audit_store::RefFields::Plain(
                            sc::CodecFields::from_plain(fields),
                        ));
                    }
                }
            }
            let fields = sc::path::lookup_fields_refview(self, r, n, true)?;
            if let Some(key) = key {
                // Keep the exact requested name sequence (including duplicates) in the key.
                let originals = n
                    .iter()
                    .filter_map(|name| fields.get(name).cloned().map(|r| ((*name).to_owned(), r)))
                    .collect();
                if let Some(memo) = self.fields.as_mut() {
                    memo.field_decodes += 1;
                    memo.fields.insert(key, originals);
                }
            }
            Ok(super::audit_store::RefFields::Plain(fields))
        }
    }
    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_array_ref(r))
        } else {
            let plain = sc::path::lookup_array_items_refview(self, r)?;
            let mut out = bj::new_work(None);
            for r in plain {
                out.try_push(RefValue::Plain(r))?;
            }
            Ok(out)
        }
    }
    fn render_ref(
        &mut self,
        r: sc::RefView<'_>,
        o: &mut dyn std::io::Write,
    ) -> Result<(), sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.render_ref(r, o))
        } else {
            sc::render_refview(self, r, o)
        }
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        self.source.allocation_budget()
    }
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        if self.source.owned_codec_active() {
            self.source.new_ref_result()
        } else {
            sc::CodecVec::from_plain(Vec::new())
        }
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
    fn content_ref_of(&mut self, r: &str) -> Result<ContentRef, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.content_ref_of(r))
        } else {
            sc::content_ref_of(self, r)
        }
    }
    fn lookup_path(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
    ) -> Result<ContentRef, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_path(r, p))
        } else {
            lookup_path(self, r, p)
        }
    }
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_array_items(r))
        } else {
            sc::path::lookup_array_items(self, r).map(sc::CodecVec::from_plain)
        }
    }
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_object_fields(r, n))
        } else {
            sc::path::lookup_object_fields(self, r, n).map(sc::CodecFields::from_plain)
        }
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.lookup_object_fields_subset(r, n))
        } else {
            sc::path::lookup_object_fields_subset(self, r, n).map(sc::CodecFields::from_plain)
        }
    }
    fn render_canonical(
        &mut self,
        r: &ContentRef,
        w: &mut dyn std::io::Write,
    ) -> Result<(), sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.render_canonical(r, w))
        } else {
            sc::render_canonical(self, r, w)
        }
    }
}
impl CodecStore for ArrayAuditSource<'_, '_, '_, '_> {
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<Built, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.write_json_view(v))
        } else {
            sc::write_json_view(self, v)
        }
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<Built, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.write_value_view(v))
        } else {
            sc::write_value_view(self, v)
        }
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<Built, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.write_template_view(v))
        } else {
            sc::write_template_view(self, v)
        }
    }
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
    fn write_value(&mut self, v: &V) -> Result<Built, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.write_value(v))
        } else {
            sc::write_value(self, v)
        }
    }
    fn write_template(&mut self, v: &V) -> Result<Built, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.write_template(v))
        } else {
            sc::write_template(self, v)
        }
    }
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        b: &str,
    ) -> Result<ContentRef, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.replace_batch_placeholder(r, p, b))
        } else {
            sc::path::replace_batch_placeholder(self, r, p, b)
        }
    }
}

fn source_allocation(
    s: &(impl CodecRead + ?Sized),
) -> Result<Option<sc::capture_allocation::OwnedBudget>, String> {
    let allocation = s.allocation_budget();
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err(err("owned来源集合入口未启用"));
    }
    Ok(allocation)
}
fn coords(s: &mut (impl CodecRead + ?Sized), r: &RefValue) -> Result<sc::CodecVec<i64>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    for item in elements(s, r)? {
        if item.kind() != Kind::String {
            return Err(err("源坐标必须为文本"));
        }
        let value = bj::read_text(s, item.view())?;
        let n = decimal(value.as_str())?;
        if out.last().is_some_and(|prev| *prev >= n) {
            return Err(err("源坐标不是严格升序"));
        }
        out.try_push(n).map_err(err)?;
    }
    Ok(out)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Source {
    wire: NodeDigest,
    numeric: NodeDigest,
    raw: NodeDigest,
}
struct Object {
    id: bj::TextValue,
    kind: bj::TextValue,
    fields: SourceFields<'static>,
    refs: RefValue,
    sources: SourceCoords,
}
struct Group {
    payload: RefValue,
    members: SourceCoords,
    sources: SourceCoords,
    anchor: i64,
}
struct Shape {
    groups: [i64; 3],
}
#[derive(Debug)]
pub(super) struct Counts {
    pub objects: usize,
    pub groups: usize,
    pub windows: usize,
    pub effective: usize,
    pub insufficient: usize,
    pub outside_domain: usize,
}
fn references(
    s: &mut (impl CodecRead + ?Sized),
    r: &RefValue,
    raw: &BusinessMap<i64, Source>,
    numeric: bool,
    checked: &mut BusinessMap<(NodeDigest, bool), i64>,
) -> Result<sc::CodecVec<i64>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    for item in elements(s, r)? {
        // elements每次仍核实际成员头；仅本次已全核state、固定raw读集内复用
        // 首次完整八字段原件匹配。未知引用仍走原坐标读取及独立原件对拍。
        let key = (digest(&item)?, numeric);
        let coord = if let Some(coord) = checked.get(&key) {
            *coord
        } else {
            let coord = decimal(&text(s, &item, "source_coord")?)?;
            let expected = raw
                .get(&coord)
                .ok_or_else(|| err("引用不是本cut生效来源"))?;
            if key.0
                != if numeric {
                    expected.numeric
                } else {
                    expected.wire
                }
            {
                return Err(err("源引用不是本cut的真实完整八字段原件"));
            }
            checked.insert(key, coord).map_err(err)?;
            coord
        };
        if out.last().is_some_and(|prev| *prev >= coord) {
            return Err(err("源引用不是严格升序"));
        }
        out.try_push(coord).map_err(err)?;
    }
    Ok(out)
}
fn source_coords(
    s: &mut (impl CodecRead + ?Sized),
    r: &RefValue,
    memo: Option<&mut NativeSourceMemo<'_>>,
    domain: ParseDomain,
) -> Result<SourceCoords, String> {
    match memo {
        Some(memo) => memo.coords(s, r, domain),
        None => coords(s, r).map(SourceCoords::Original),
    }
}
fn source_references(
    s: &mut (impl CodecRead + ?Sized),
    r: &RefValue,
    raw: &BusinessMap<i64, Source>,
    numeric: bool,
    checked: &mut BusinessMap<(NodeDigest, bool), i64>,
    memo: Option<&mut NativeSourceMemo<'_>>,
    prefix: usize,
    domain: ParseDomain,
) -> Result<SourceCoords, String> {
    match memo {
        Some(memo) => memo.references(s, r, raw, numeric, checked, prefix, domain),
        None => references(s, r, raw, numeric, checked).map(SourceCoords::Original),
    }
}
fn copy_source_coords(
    s: &(impl CodecRead + ?Sized),
    values: &SourceCoords,
) -> Result<SourceCoords, String> {
    match values {
        SourceCoords::Shared(v) => Ok(SourceCoords::Shared(v.clone())),
        SourceCoords::Original(v) => copy_coords(s, v).map(SourceCoords::Original),
    }
}
/// 仅复制i64；从实际读取源取allocator，不开放普通深Clone或空树后补账。
fn copy_coords(s: &(impl CodecRead + ?Sized), values: &[i64]) -> Result<sc::CodecVec<i64>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    out.try_reserve_exact(values.len()).map_err(err)?;
    for &value in values {
        out.try_push(value).map_err(err)?;
    }
    Ok(out)
}
/// 两个严格升序坐标集的线性并集；旧集合在新集合成功完成前始终活着。
fn union_coords(
    s: &(impl CodecRead + ?Sized),
    left: &[i64],
    right: &[i64],
) -> Result<sc::CodecVec<i64>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    let (mut i, mut j) = (0, 0);
    while i < left.len() || j < right.len() {
        let value = if j == right.len() || (i < left.len() && left[i] < right[j]) {
            let v = left[i];
            i += 1;
            v
        } else if i == left.len() || right[j] < left[i] {
            let v = right[j];
            j += 1;
            v
        } else {
            let v = left[i];
            i += 1;
            j += 1;
            v
        };
        out.try_push(value).map_err(err)?;
    }
    Ok(out)
}
fn difference_coords(
    s: &(impl CodecRead + ?Sized),
    all: &[i64],
    members: &[i64],
) -> Result<sc::CodecVec<i64>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    let mut j = 0;
    for &value in all {
        while j < members.len() && members[j] < value {
            j += 1;
        }
        if members.get(j) != Some(&value) {
            out.try_push(value).map_err(err)?;
        }
    }
    Ok(out)
}
fn sorted_numeric_entries<'a, V>(
    s: &(impl CodecRead + ?Sized),
    map: &'a BusinessMap<i64, V>,
) -> Result<sc::CodecVec<(i64, &'a V)>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    for (key, value) in map.iter() {
        out.try_push((*key, value)).map_err(err)?;
    }
    out.sort_unstable_by_key(|(key, _)| *key);
    Ok(out)
}
const SHAPE_SOURCE_FIELDS: [&str; 12] = [
    "object_id",
    "kind",
    "object_revision",
    "input_refs_json",
    "source_coords_json",
    "branch",
    "dir_ab",
    "dir_bc",
    "comparisons_json",
    "window_start",
    "window_mid",
    "window_end",
];
const TYPED_SOURCE_FIELDS: [&str; 7] = [
    "object_id",
    "kind",
    "object_revision",
    "input_refs_json",
    "source_coords_json",
    "payload_json",
    "fact_key",
];
// #1394：只在本次 fresh Native audit 内分享纯身份；不带出 raw/cut 校验结果。
#[derive(Debug, PartialEq, Eq, Hash)]
struct NativeWitnessIdentityKey {
    capture: usize,
    object_id: String,
    slot: usize,
    mi: usize,
    digest: NodeDigest,
    kind: Kind,
    extent: Option<u64>,
}
struct NativeWitnessIdentityMemo<'capture> {
    capture: &'capture sc::AuditCapture,
    entries: std::collections::HashMap<NativeWitnessIdentityKey, String>,
    #[cfg(test)]
    stats: NativeWitnessIdentityStats,
}
#[cfg(test)]
#[derive(Default, Clone, Copy, serde::Serialize)]
pub(super) struct NativeWitnessIdentityStats {
    hits: u64,
    misses: u64,
    render_calls: u64,
}
#[cfg(test)]
thread_local! {
    static NATIVE_WITNESS_IDENTITY_STATS: std::cell::Cell<Option<NativeWitnessIdentityStats>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
pub(super) fn reset_native_witness_identity_stats() {
    NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(None));
}
#[cfg(test)]
pub(super) fn native_witness_identity_stats() -> Option<NativeWitnessIdentityStats> {
    NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.get())
}
impl<'capture> NativeWitnessIdentityMemo<'capture> {
    #[cfg(feature = "s_capture_native_testonly")]
    fn new(capture: &'capture sc::AuditCapture) -> Result<Self, String> {
        if capture.is_allocation_prototype() {
            return Err(err("Native身份memo不接收Owned capture"));
        }
        #[cfg(test)]
        NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(Some(NativeWitnessIdentityStats::default())));
        Ok(Self {
            capture,
            entries: Default::default(),
            #[cfg(test)]
            stats: Default::default(),
        })
    }
    fn key(
        &self,
        object_id: &str,
        slot: usize,
        mi: usize,
        bars: &Built,
    ) -> NativeWitnessIdentityKey {
        NativeWitnessIdentityKey {
            capture: std::ptr::from_ref(self.capture) as usize,
            object_id: object_id.to_owned(),
            slot,
            mi,
            digest: bars.digest,
            kind: bars.kind,
            extent: bars.extent,
        }
    }
    #[cfg(test)]
    fn record_render_attempt(&mut self, object_id: &str) {
        self.stats.misses += 1;
        // 原函数拒绝转义对象文本时尚未调用 renderer。
        self.stats.render_calls += u64::from(!object_id.contains(['"', '\\']));
        NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(Some(self.stats)));
    }
    // 只能由下面同 Capture 构造的实际读取上下文在完整本 cut 核验后调用。
    fn check_identity(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        stored: &str,
        object_id: &str,
        slot: usize,
        mi: usize,
        bars: &Built,
    ) -> Result<(), String> {
        if !self.capture.contains(&bars.digest) {
            // 派生 overlay 根不取得缓存资格，明确沿原函数计算。
            #[cfg(test)]
            self.record_render_attempt(object_id);
            let want = super::generation::witness_identity(s, object_id, slot, mi, bars)?;
            return if stored == want {
                Ok(())
            } else {
                Err(err("见证ID不符"))
            };
        }
        // 根在 Capture 不等于完整闭包在 Capture；用不可变原件验证全部声明与闭包。
        self.capture
            .verify(&bars.content_ref(), sc::Mode::Public)
            .map_err(err)?;
        let key = self.key(object_id, slot, mi, bars);
        if let Some(want) = self.entries.get(&key) {
            #[cfg(test)]
            {
                self.stats.hits += 1;
                NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(Some(self.stats)));
            }
            return if stored == want {
                Ok(())
            } else {
                Err(err("见证ID不符"))
            };
        }
        #[cfg(test)]
        self.record_render_attempt(object_id);
        let want = super::generation::witness_identity(s, object_id, slot, mi, bars)?;
        if stored != want {
            return Err(err("见证ID不符"));
        }
        // 首次纯函数成功和本次 stored ID 比较都通过之后，才保留输入声明及结果文本。
        self.entries.insert(key, want);
        Ok(())
    }
}
// #1394：账本只借同一完整 immutable raw；请求只能指定 prefix，不能替换原件。
// 先选本 cut 最终身份赢家，再核全部变化坐标，最后才按 coord hash；跳跃中的
// 暂态赢家、暂态坐标碰撞和未来事件均不取得 Source 构造资格。
#[derive(Clone, Copy)]
struct NativeRawWinner {
    revision: i64,
    ordinal: usize,
    coord: Option<i64>,
}
#[cfg(test)]
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct NativeRawLedgerStats {
    pub raw_visits: u64,
    pub coord_decodes: u64,
    pub source_hashes: u64,
    pub map_updates: u64,
    pub rewinds: u64,
}
struct NativeRawLedger<'raw> {
    raw: RawEventsView<'raw>,
    prefix: usize,
    winners: std::collections::HashMap<&'raw str, NativeRawWinner>,
    sources: BusinessMap<i64, Source>,
    originals: std::collections::HashMap<usize, (i64, Source)>,
    #[cfg(test)]
    stats: NativeRawLedgerStats,
}
impl<'raw> NativeRawLedger<'raw> {
    fn new(raw: RawEventsView<'raw>) -> Self {
        Self {
            raw,
            prefix: 0,
            winners: Default::default(),
            sources: bj::new_map(None),
            originals: Default::default(),
            #[cfg(test)]
            stats: Default::default(),
        }
    }
    fn reset_progress(&mut self) {
        self.prefix = 0;
        self.winners.clear();
        self.sources = bj::new_map(None);
    }
    fn clear(&mut self) {
        self.reset_progress();
        self.originals.clear();
    }
    fn advance(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        prefix: usize,
    ) -> Result<&BusinessMap<i64, Source>, String> {
        if let Err(error) = self.advance_inner(s, prefix) {
            self.clear();
            return Err(error);
        }
        Ok(&self.sources)
    }
    fn advance_inner(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        prefix: usize,
    ) -> Result<(), String> {
        self.raw.prefix(prefix)?;
        if prefix < self.prefix {
            self.reset_progress();
            #[cfg(test)]
            {
                self.stats.rewinds += 1;
            }
        }
        let mut changed = std::collections::BTreeMap::new();
        for ordinal in self.prefix..prefix {
            #[cfg(test)]
            {
                self.stats.raw_visits += 1;
            }
            let event = self.raw.get(ordinal).expect("checked raw prefix");
            let identity = bj::field(event, "identity_key").as_str().unwrap_or("");
            let revision = bj::field(event, "revision").as_i64().unwrap_or(0);
            let previous = self.winners.get(identity).copied();
            if previous.is_some_and(|old| {
                !bj::effective_precedence(revision, ordinal, old.revision, old.ordinal).is_lt()
            }) {
                continue;
            }
            changed
                .entry(identity)
                .or_insert(previous.and_then(|old| old.coord));
            self.winners.insert(
                identity,
                NativeRawWinner {
                    revision,
                    ordinal,
                    coord: None,
                },
            );
        }
        // 与 effective 相同：先按身份顺序验证坐标，再计算任何来源摘要。
        let mut additions = Vec::with_capacity(changed.len());
        for &identity in changed.keys() {
            let winner = self.winners.get(identity).expect("changed winner");
            let coord = if let Some((coord, _)) = self.originals.get(&winner.ordinal) {
                *coord
            } else {
                #[cfg(test)]
                {
                    self.stats.coord_decodes += 1;
                }
                bj::coord(self.raw.get(winner.ordinal).expect("winner original"))?
            };
            additions.push((coord, identity, winner.ordinal));
        }
        // 先移除全部旧赢家再核最终坐标；允许坐标交换，不拒绝跳过前缀的暂态冲突。
        for coord in changed.values().flatten() {
            self.sources.remove(coord);
        }
        additions.sort_unstable_by_key(|(coord, _, _)| *coord);
        if additions.windows(2).any(|v| v[0].0 == v[1].0)
            || additions
                .iter()
                .any(|(coord, _, _)| self.sources.contains_key(coord))
        {
            return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".into());
        }
        for (coord, identity, ordinal) in additions {
            let source = if let Some((_, source)) = self.originals.get(&ordinal) {
                *source
            } else {
                let event = self.raw.get(ordinal).expect("winner original");
                let wire = bj::RawReference::new(event, false)?;
                let numeric = bj::RawReference::new(event, true)?;
                // hash_json_in_store 用 Discard 丢弃节点，不写 reader overlay。
                // 后续只比较这三个摘要，从不以它们读取节点。
                let source = Source {
                    wire: super::history::hash_json_in_store(s, wire.view(), None, None)?,
                    numeric: super::history::hash_json_in_store(s, numeric.view(), None, None)?,
                    raw: super::history::hash_json_in_store(s, event, None, None)?,
                };
                #[cfg(test)]
                {
                    self.stats.source_hashes += 3;
                }
                self.originals.insert(ordinal, (coord, source));
                source
            };
            self.sources.insert(coord, source).map_err(err)?;
            self.winners
                .get_mut(identity)
                .expect("changed winner")
                .coord = Some(coord);
            #[cfg(test)]
            {
                self.stats.map_updates += 1;
            }
        }
        self.prefix = prefix;
        Ok(())
    }
}
/// 私有 Native 当前审计上下文：源及身份 memo 都由同一不可变 Capture 构造。
/// audit 不接外来 store、资格 bool 或跨请求数据；reader/overlay 随本次 audit 释放。
pub(super) struct NativeSourceAudit<'capture> {
    reader: super::history::HashReader<'capture>,
    memo: NativeWitnessIdentityMemo<'capture>,
    source_memo: NativeSourceMemo<'capture>,
    ledger: NativeRawLedger<'capture>,
    fields: NativeFieldMemo<'capture>,
    raw: RawEventsView<'capture>,
}
impl<'capture> NativeSourceAudit<'capture> {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn new(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
    ) -> Result<Self, String> {
        if raw.budget().is_some() {
            return Err(err("Native来源memo不接收Owned原始账"));
        }
        Ok(Self {
            source_memo: NativeSourceMemo::new(capture, raw),
            ledger: NativeRawLedger::new(raw),
            fields: NativeFieldMemo::new(capture),
            raw,
            reader: super::history::HashReader::captured(capture, None),
            memo: NativeWitnessIdentityMemo::new(capture)?,
        })
    }
    #[cfg(test)]
    pub(super) fn raw_ledger_stats(&self) -> NativeRawLedgerStats {
        self.ledger.stats
    }
    pub(super) fn audit<'a>(
        &mut self,
        state: impl Into<sc::RefView<'a>>,
        prefix_len: usize,
        profile: &str,
        rule: &str,
    ) -> Result<Counts, String> {
        let result = (|| {
            audit_generated_in_store(
                &mut super::audit_store::StoreView::for_audit(&mut self.reader),
                state.into(),
                self.raw.prefix(prefix_len)?,
                profile,
                rule,
                None,
                None,
                Some(&mut self.memo),
                Some(&mut self.source_memo),
                Some(&mut self.fields),
                Some(&mut self.ledger),
            )
        })();
        if result.is_err() {
            self.fields.clear();
            self.ledger.clear();
            self.source_memo.clear();
            self.memo.entries.clear();
        }
        result
    }
}
/// raw_prefix 是同一次读集、截至本代 input_frontier 的独立原始账，不是manifest自己提供的值。
pub(super) fn audit_generated<'a>(
    s: &mut dyn NodeStore,
    state: impl Into<sc::RefView<'a>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    profile: &str,
    rule: &str,
) -> Result<Counts, String> {
    audit_generated_metered(s, state, raw_prefix, profile, rule, None, None)
}
pub(super) fn audit_generated_until<'a>(
    s: &mut dyn NodeStore,
    state: impl Into<sc::RefView<'a>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
) -> Result<Counts, String> {
    audit_generated_metered(s, state, raw_prefix, profile, rule, deadline, None)
}
pub(super) fn audit_generated_metered<'a>(
    s: &mut dyn NodeStore,
    state: impl Into<sc::RefView<'a>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    audit_generated_in_store(
        &mut super::audit_store::StoreView::generic(s),
        state.into(),
        raw_prefix.into(),
        profile,
        rule,
        deadline,
        budget,
        None,
        None,
        None,
        None,
    )
}
pub(super) fn audit_generated_for_audit<'a>(
    s: &mut super::history::HashReader<'_>,
    state: impl Into<sc::RefView<'a>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    audit_generated_in_store(
        &mut super::audit_store::StoreView::for_audit(s),
        state.into(),
        raw_prefix.into(),
        profile,
        rule,
        deadline,
        budget,
        None,
        None,
        None,
        None,
    )
}
fn audit_generated_in_store(
    s: &mut super::audit_store::StoreView<'_, '_>,
    state: sc::RefView<'_>,
    raw_prefix: RawEventsView<'_>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    mut native_identity: Option<&mut NativeWitnessIdentityMemo<'_>>,
    mut native_sources: Option<&mut NativeSourceMemo<'_>>,
    native_fields: Option<&mut NativeFieldMemo<'_>>,
    native_ledger: Option<&mut NativeRawLedger<'_>>,
) -> Result<Counts, String> {
    super::history::check_deadline(deadline)?;
    #[cfg(test)]
    super::state::trace_event("source_closure", None);
    s.verify_ref(state, sc::Mode::Public).map_err(err)?;
    let mut array_source = ArrayAuditSource {
        source: s,
        full_arrays: budget.is_none(),
        deadline,
        fields: native_fields,
    };
    let s = &mut array_source;
    #[cfg(test)]
    super::state::trace_event("source_raw", None);
    if profile != tb02a::PROFILE {
        return Err(err(
            "当前来源审计仅实现OHLC档；其他档仍待接线，拒绝按此口径放行",
        ));
    }
    let producer = Cc004Producer::new(profile, rule);
    let allocation;
    let ordinary_effective;
    let mut ordinary_raw;
    let raw = if let Some(ledger) = native_ledger {
        allocation = source_allocation(s)?;
        ledger.advance(s, raw_prefix.len())?
    } else {
        ordinary_effective = bj::effective(raw_prefix, deadline)?;
        allocation = source_allocation(s)?;
        ordinary_raw = bj::new_map(allocation.as_ref());
        for event in ordinary_effective.iter() {
            super::history::check_deadline(deadline)?;
            let coord = bj::coord(event)?;
            let wire = bj::RawReference::new(event, false)?;
            let numeric = bj::RawReference::new(event, true)?;
            ordinary_raw
                .insert(
                    coord,
                    Source {
                        wire: super::history::hash_json_in_store(s, wire.view(), deadline, budget)?,
                        numeric: super::history::hash_json_in_store(
                            s,
                            numeric.view(),
                            deadline,
                            budget,
                        )?,
                        raw: super::history::hash_json_in_store(s, event, deadline, budget)?,
                    },
                )
                .map_err(err)?;
        }
        &ordinary_raw
    };
    let object_array = s.lookup_ref(state, &[f("objects")]).map_err(err)?;
    #[cfg(test)]
    super::state::trace_event("source_objects", None);
    // 活期只覆盖当前一次audit_generated；不在代际、事务或源修订之间带出。
    let mut checked_refs = bj::new_map(allocation.as_ref());
    let mut objects = bj::new_work(allocation.as_ref());
    let mut ids = bj::new_map(allocation.as_ref());
    let mut groups = bj::new_map(allocation.as_ref());
    for record in elements_ref(s, object_array.view())? {
        super::history::check_deadline(deadline)?;
        let kind = if s.fields.is_some() {
            SourceFields::read_subset(s, record.view(), &["kind"])?.text(s, "kind")?
        } else {
            text(s, &record, "kind")?
        };
        let names: &[&'static str] = if kind == CC006_SHAPE_KIND {
            &SHAPE_SOURCE_FIELDS
        } else {
            &TYPED_SOURCE_FIELDS
        };
        let fields = SourceFields::read_subset(s, record.view(), names).map_err(err)?;
        let id = fields.text(s, "object_id").map_err(err)?;
        if ids.insert(id.try_clone()?, ()).map_err(err)?.is_some()
            || fields.integer(s, "object_revision").map_err(err)? != 1
        {
            return Err(err("对象身份重复或修订不符"));
        }
        let refs = s
            .lookup_ref(
                fields.get("input_refs_json").map_err(err)?.view(),
                &[PathStep::JsonText],
            )
            .map_err(err)?;
        let source_ref = s
            .lookup_ref(
                fields.get("source_coords_json").map_err(err)?.view(),
                &[PathStep::JsonText],
            )
            .map_err(err)?;
        let sources = source_coords(
            s,
            &source_ref,
            native_sources.as_deref_mut(),
            ParseDomain::JsonInner,
        )?;
        if kind == CC006_SHAPE_KIND {
            exact(&record, Kind::Object, 15)?;
        } else {
            exact(&record, Kind::Object, 10)?;
            if ![
                CC004_STEP_KIND,
                CC005_GROUP_KIND,
                CC007_FRACTAL_KIND,
                CC054_KNOWLEDGE_KIND,
            ]
            .contains(&kind.as_str())
                && !tb02b::KINDS.contains(&kind.as_str())
            {
                return Err(err("未声明typed事实种类"));
            }
            let payload = s
                .lookup_ref(
                    fields.get("payload_json").map_err(err)?.view(),
                    &[PathStep::JsonText],
                )
                .map_err(err)?;
            if payload.kind() != Kind::Object {
                return Err(err("typed载荷不是对象"));
            }
            if source_references(
                s,
                &refs,
                &raw,
                false,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )? != sources
            {
                return Err(err("typed引用与source_coords不同"));
            }
            let key = fields.text(s, "fact_key").map_err(err)?;
            let parsed_owner = parsed_json(s, key.as_bytes())?;
            let parsed = parsed_owner.view();
            if !canonical_matches(s, parsed, key.as_bytes())?
                || parsed.as_array().is_none_or(|a| a.len() != 2)
                || parsed
                    .index(0)
                    .unwrap_or(sc::capture_allocation::audit_json::JsonView::Null)
                    .as_str()
                    != Some(kind.as_str())
            {
                return Err(err("typed槽位种类/规范文本不符"));
            }
            let expected = producer
                .typed_object_id(&kind, &key, &built(payload.view())?, &built(refs.view())?)
                .map_err(err)?;
            if id != expected {
                return Err(err("typed obj3身份与来源/载荷原件不符"));
            }
            if tb02b::KINDS.contains(&kind.as_str()) {
                for (k, want) in [
                    ("schema_revision", "s-new-bi/1"),
                    (
                        "semantic_version",
                        newchan_rust::theta_v0::parser::stroke::RULE,
                    ),
                    ("policy_version", "standard_raw_gap_3"),
                    ("view_role", "main"),
                ] {
                    if text(s, &payload, k)? != want {
                        return Err(err("新笔载荷语义头不同"));
                    }
                }
                if digest(&field(s, &payload, "slot")?)?
                    != super::history::hash_json_in_store(
                        s,
                        parsed
                            .index(1)
                            .unwrap_or(sc::capture_allocation::audit_json::JsonView::Null),
                        deadline,
                        budget,
                    )?
                {
                    return Err(err("新笔payload槽位与fact_key不同"));
                }
            }
            if kind == CC005_GROUP_KIND {
                let index = decimal(&text(s, &payload, "group_index")?)?;
                let anchor = decimal(&text(s, &payload, "group_anchor")?)?;
                let members_root = field(s, &payload, "members")?;
                let members = source_coords(
                    s,
                    &members_root,
                    native_sources.as_deref_mut(),
                    ParseDomain::JsonInner,
                )?;
                if index < 0
                    || members.is_empty()
                    || !members.contains(&anchor)
                    || members.iter().any(|c| !raw.contains_key(c))
                {
                    return Err(err("组成员/锚/序号不符"));
                }
                let mut union = copy_coords(s, &members)?;
                for key in ["direction_evidence", "confirmation_evidence"] {
                    super::history::check_deadline(deadline)?;
                    let evidence = field(s, &payload, key)?;
                    if evidence.kind() != Kind::Null {
                        let list = field(s, &evidence, "source_coords")?;
                        let evidence_coords = source_coords(
                            s,
                            &list,
                            native_sources.as_deref_mut(),
                            ParseDomain::JsonInner,
                        )?;
                        union = union_coords(s, &union, &evidence_coords)?;
                    }
                }
                if &*union != &*sources {
                    return Err(err("组来源不等于成员及方向/封口依据的并集"));
                }
                if groups
                    .insert(
                        index,
                        Group {
                            payload: payload.clone(),
                            members,
                            sources: copy_source_coords(s, &sources)?,
                            anchor,
                        },
                    )
                    .map_err(err)?
                    .is_some()
                {
                    return Err(err("同cut组合并序号重复"));
                }
            }
        }
        objects
            .try_push(Object {
                id,
                kind,
                fields,
                refs,
                sources,
            })
            .map_err(err)?;
    }
    let mut partition = bj::new_map(allocation.as_ref());
    let ordered_groups = sorted_numeric_entries(s, &groups)?;
    for (expected, (index, group)) in ordered_groups.iter().enumerate() {
        super::history::check_deadline(deadline)?;
        if *index != expected as i64 {
            return Err(err("组合并序号不连续"));
        }
        for c in &group.members {
            super::history::check_deadline(deadline)?;
            if partition.insert(*c, ()).map_err(err)?.is_some() {
                return Err(err("同cut组成员重叠"));
            }
        }
    }
    drop(ordered_groups);
    if partition.len() != raw.len() || partition.iter().any(|(coord, _)| !raw.contains_key(coord)) {
        return Err(err("同cut组未完整覆盖生效原始账"));
    }
    let mut shapes = bj::new_map(allocation.as_ref());
    #[cfg(test)]
    super::state::trace_event("source_shapes", None);
    for object in objects.iter().filter(|o| o.kind == CC006_SHAPE_KIND) {
        super::history::check_deadline(deadline)?;
        exact(&object.refs, Kind::Array, 3)?;
        let mut indices = [0; 3];
        let mut highs: [bj::TextValue; 3] =
            std::array::from_fn(|_| bj::TextValue::Plain(String::new()));
        let mut lows: [bj::TextValue; 3] =
            std::array::from_fn(|_| bj::TextValue::Plain(String::new()));
        let mut windows =
            std::array::from_fn::<_, 3, _>(|_| crate::content_manifest::Decimal::new(0));
        let mut source_union = bj::new_work(allocation.as_ref());
        for (slot, part) in elements(s, &object.refs)?.into_iter().enumerate() {
            super::history::check_deadline(deadline)?;
            exact(&part, Kind::Object, 3)?;
            let part_fields =
                local_fields(s, &part, &["merged_index", "raw_refs", "dependency_refs"])?;
            let index = if let Some(fields) = &part_fields {
                fields
                    .scalar(s, "merged_index")?
                    .view()
                    .as_i64()
                    .ok_or_else(|| err("merged_index不是整数"))?
            } else {
                number(s, &part, "merged_index")?
            };
            indices[slot] = index;
            if slot > 0 && index != indices[slot - 1] + 1 {
                return Err(err("形态来源组不是连续三组"));
            }
            let group = groups
                .get(&index)
                .ok_or_else(|| err("形态引用没有同cut组事实"))?;
            let raw_refs = if let Some(fields) = &part_fields {
                fields.get("raw_refs").map_err(err)?
            } else {
                field(s, &part, "raw_refs")?
            };
            let members = source_references(
                s,
                &raw_refs,
                &raw,
                true,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )?;
            if members != group.members {
                return Err(err("形态raw_refs不是同cut组成员"));
            }
            let deps = if let Some(fields) = &part_fields {
                fields.get("dependency_refs").map_err(err)?
            } else {
                field(s, &part, "dependency_refs")?
            };
            let dependency = source_references(
                s,
                &deps,
                &raw,
                true,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                ParseDomain::JsonInner,
            )?;
            let expected = difference_coords(s, &group.sources, &members)?;
            if &*dependency != &*expected {
                return Err(err("形态dependency_refs不是组事实来源差集"));
            }
            source_union = union_coords(s, &source_union, &members)?;
            source_union = union_coords(s, &source_union, &dependency)?;
            windows[slot] = crate::content_manifest::Decimal::new(
                object
                    .fields
                    .integer(s, ["window_start", "window_mid", "window_end"][slot])
                    .map_err(err)?,
            );
            if windows[slot].as_str()
                != crate::content_manifest::Decimal::new(group.anchor).as_str()
            {
                return Err(err("形态窗口不等于同cut组锚"));
            }
            if let Some(fields) = local_fields(s, &group.payload, &["high", "low"])? {
                highs[slot] = fields.text(s, "high")?;
                lows[slot] = fields.text(s, "low")?;
            } else {
                highs[slot] = text(s, &group.payload, "high")?;
                lows[slot] = text(s, &group.payload, "low")?;
            }
        }
        if &*source_union != &*object.sources {
            return Err(err("形态source_coords不是完整来源并集"));
        }
        let branch = object.fields.text(s, "branch").map_err(err)?;
        let dir_ab = object.fields.text(s, "dir_ab").map_err(err)?;
        let dir_bc = object.fields.text(s, "dir_bc").map_err(err)?;
        let comparison_text = object.fields.text(s, "comparisons_json").map_err(err)?;
        let comparisons_owner = parsed_json(s, comparison_text.as_bytes())?;
        let comparisons = comparisons_owner.view();
        if !canonical_matches(s, comparisons, comparison_text.as_bytes())?
            || comparisons.as_array().is_none_or(|v| v.len() != 4)
        {
            return Err(err("形态比较不是规范四项"));
        }
        let comparisons = Built {
            digest: super::history::hash_json_in_store(s, comparisons, deadline, budget)?,
            kind: Kind::Array,
            extent: Some(4),
        };
        let input_refs = built(object.refs.view())?;
        let identity = producer
            .shape_identity_borrowed(ShapeIdentityView {
                branch: &branch,
                dir_ab: &dir_ab,
                dir_bc: &dir_bc,
                window: windows.each_ref().map(|v| v.as_str()),
                highs: highs.each_ref().map(|v| v.as_str()),
                lows: lows.each_ref().map(|v| v.as_str()),
                comparisons: &comparisons,
                input_refs: &input_refs,
            })
            .map_err(err)?;
        if content_producer::object_id_of(&identity) != object.id {
            return Err(err("CC006 obj3与同cut组事实身份不符"));
        }
        shapes
            .insert(object.id.try_clone()?, Shape { groups: indices })
            .map_err(err)?;
    }
    let witness_array = field_ref(s, state, "witnesses")?;
    #[cfg(test)]
    super::state::trace_event("source_witnesses", None);
    let mut seen = bj::new_map(allocation.as_ref());
    for witness in elements(s, &witness_array)? {
        super::history::check_deadline(deadline)?;
        exact(&witness, Kind::Object, 9)?;
        let fields = SourceFields::read_subset(
            s,
            witness.view(),
            &[
                "witness_id",
                "object_id",
                "slot",
                "merged_source_index",
                "merged_high",
                "merged_low",
                "merged_open",
                "merged_close",
                "raw_bars",
            ],
        )
        .map_err(err)?;
        let id = fields.text(s, "witness_id").map_err(err)?;
        let object_id = fields.text(s, "object_id").map_err(err)?;
        let slot = fields.integer(s, "slot").map_err(err)?;
        if !(0..3).contains(&slot)
            || seen
                .insert((object_id.try_clone()?, slot), ())
                .map_err(err)?
                .is_some()
        {
            return Err(err("见证槽位重复/越界"));
        }
        let shape = shapes
            .get(object_id.as_str())
            .ok_or_else(|| err("见证没有同cut形态对象"))?;
        let mi = shape.groups[slot as usize];
        let group = groups.get(&mi).expect("shape was bound to existing groups");
        if fields.integer(s, "merged_source_index").map_err(err)? != group.anchor {
            return Err(err("见证锚与同cut组不同"));
        }
        let payload_fields = local_fields(s, &group.payload, &["high", "low", "open", "close"])?;
        for (merged, field_name) in [
            ("merged_high", "high"),
            ("merged_low", "low"),
            ("merged_open", "open"),
            ("merged_close", "close"),
        ] {
            let actual = fields.get(merged).map_err(err)?;
            let expected = if let Some(fields) = &payload_fields {
                fields.get(field_name).map_err(err)?
            } else {
                field(s, &group.payload, field_name)?
            };
            if actual != expected {
                return Err(err("见证合并值不是同cut组事实"));
            }
        }
        let bars = fields.get("raw_bars").map_err(err)?;
        let items = elements(s, &bars)?;
        if items.len() != group.members.len() {
            return Err(err("见证raw_bars成员数不同"));
        }
        for (bar, coord) in items.iter().zip(&group.members) {
            super::history::check_deadline(deadline)?;
            if digest(bar)? != raw.get(coord).expect("group members were bound to raw").raw {
                return Err(err("见证raw_bars不是同cut真实原始成员"));
            }
        }
        let bars = built(bars.view())?;
        match native_identity.as_deref_mut() {
            Some(memo) => {
                memo.check_identity(s, &id, &object_id, slot as usize, mi as usize, &bars)?
            }
            None => {
                let want = super::generation::witness_identity(
                    s,
                    &object_id,
                    slot as usize,
                    mi as usize,
                    &bars,
                )?;
                if id != want {
                    return Err(err("见证ID不符"));
                }
            }
        }
    }
    if seen.len() != shapes.len() * 3 {
        return Err(err("形态缺完整三槽见证"));
    }
    let observations = field_ref(s, state, "observations")?;
    #[cfg(test)]
    super::state::trace_event("source_observations", None);
    let mut insufficient = 0;
    let mut outside_domain = 0;
    for item in elements(s, &observations)? {
        super::history::check_deadline(deadline)?;
        match text(s, &item, "kind")?.as_str() {
            "insufficient_knowledge" => insufficient += 1,
            "domain_not_satisfied" => outside_domain += 1,
            _ => {}
        }
    }
    Ok(Counts {
        objects: objects.len(),
        groups: groups.len(),
        windows: groups.len().saturating_sub(2),
        effective: raw.len(),
        insufficient,
        outside_domain,
    })
}

#[cfg(test)]
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_get_opt, open_db, read_raw_events,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use sc::MemoryNodes;
    const STATE: &[&str] = &[
        "generation",
        "structure_cut",
        "input_frontier",
        "catalog_revision",
        "profile_id",
        "profile_hash",
        "rule_revision",
        "raw_events",
        "effective_events",
        "objects",
        "witnesses",
        "relations",
        "observations",
        "scope",
    ];
    const TYPED: &[&str] = &[
        "object_id",
        "object_revision",
        "kind",
        "fact_key",
        "payload_json",
        "input_refs_json",
        "source_coords_json",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ];
    const SHAPE_FIELDS: &[&str] = &[
        "object_id",
        "object_revision",
        "kind",
        "branch",
        "window_start",
        "window_mid",
        "window_end",
        "dir_ab",
        "dir_bc",
        "comparisons_json",
        "input_refs_json",
        "source_coords_json",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ];
    const WITNESS: &[&str] = &[
        "witness_id",
        "object_id",
        "slot",
        "merged_source_index",
        "merged_high",
        "merged_low",
        "merged_open",
        "merged_close",
        "raw_bars",
    ];
    fn fixture() -> (MemoryNodes, ContentRef, Vec<Value>, String) {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        let c = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&c).unwrap();
        let effective = read_effective_events(&raw).unwrap();
        let get = |k| meta_get_opt(&c, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        let rule = get("rule_revision");
        let catalog = get("catalog_revision");
        let scope = serde_json::from_str(&get("scope")).unwrap();
        let mut store = MemoryNodes::default();
        let mut runtime = GenerationRuntime::new(
            format!("source-audit/{}", files.db().display()),
            tb02a::PROFILE,
            &rule,
        );
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
                bi_records: vec![],
                version_record: None,
            },
        };
        let generated = runtime.generate(&mut store, &input).unwrap();
        let root = generated.generated_state.clone();
        (store, root, raw, rule)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn ledger_event(identity: &str, coord: &str, revision: i64, seq: i64) -> Value {
        json!({"identity_key": identity, "source_coord": coord, "revision": revision,
            "seq": seq, "payload_hash": "same-price", "receipt_id": format!("r-{seq}"),
            "event_id": format!("e-{seq}"), "input_revision": "1"})
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn original_sources(raw: &[Value], n: usize) -> Result<BusinessMap<i64, Source>, String> {
        let effective = bj::effective(RawEventsView::plain(&raw[..n]), None)?;
        let mut sources = bj::new_map(None);
        for event in effective.iter() {
            let wire = bj::RawReference::new(event, false)?;
            let numeric = bj::RawReference::new(event, true)?;
            sources
                .insert(
                    bj::coord(event)?,
                    Source {
                        wire: sc::write_json_view(&mut HashOnly, wire.view())
                            .map_err(err)?
                            .digest,
                        numeric: sc::write_json_view(&mut HashOnly, numeric.view())
                            .map_err(err)?
                            .digest,
                        raw: sc::write_json_view(&mut HashOnly, event)
                            .map_err(err)?
                            .digest,
                    },
                )
                .map_err(err)?;
        }
        Ok(sources)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn source_entries(map: &BusinessMap<i64, Source>) -> std::collections::BTreeMap<i64, Source> {
        map.iter()
            .map(|(coord, source)| (*coord, *source))
            .collect()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_raw_ledger_incremental_winners_rewind_and_overlay_clear() {
        let raw = vec![
            ledger_event("a", "900", 3, 40),
            ledger_event("b", "7", 1, 20),
            ledger_event("a", "900", 3, 1),
            ledger_event("a", "900", 2, 80),
            ledger_event("b", "7", 5, 99),
            ledger_event("a", "900", 4, 2),
            ledger_event("c", "88", 1, 3),
        ];
        let capture = sc::AuditCapture::new(MemoryNodes::default());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let mut ledger = NativeRawLedger::new((&raw).into());
        for n in 0..=raw.len() {
            let want = original_sources(&raw, n).unwrap();
            let got = ledger.advance(&mut reader, n).unwrap();
            assert_eq!(source_entries(got), source_entries(&want), "prefix={n}");
            reader.clear_overlay();
        }
        assert_eq!(ledger.stats.raw_visits, 7);
        assert_eq!(ledger.stats.coord_decodes, 5);
        assert_eq!(ledger.stats.source_hashes, 15);
        assert_eq!(ledger.stats.map_updates, 5);
        for n in [0, 3, 6, 1, 7, 7] {
            assert_eq!(
                source_entries(ledger.advance(&mut reader, n).unwrap()),
                source_entries(&original_sources(&raw, n).unwrap()),
                "rewind prefix={n}"
            );
            reader.clear_overlay();
        }
        assert_eq!(
            ledger.stats.source_hashes, 15,
            "successful originals hash once, including rewind"
        );
        assert_eq!(ledger.stats.coord_decodes, 5);
        assert_eq!(ledger.stats.rewinds, 2);
        let fresh = NativeRawLedger::new((&raw).into());
        assert!(fresh.sources.is_empty() && fresh.originals.is_empty() && fresh.winners.is_empty());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_raw_ledger_jump_skips_invalid_transients_and_checks_final_collisions_first() {
        let mut bad = ledger_event("a", "-1", 2, 2);
        bad["seq"] = Value::Null;
        let raw = vec![
            ledger_event("a", "5", 1, 0),
            ledger_event("b", "9", 1, 1),
            bad,
            ledger_event("a", "7", 3, 3),
            ledger_event("b", "7", 2, 4),
            ledger_event("b", "5", 3, 5),
            ledger_event("future", "01", 1, 6),
        ];
        let capture = sc::AuditCapture::new(MemoryNodes::default());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let mut ledger = NativeRawLedger::new((&raw).into());
        for n in [2, 6] {
            assert_eq!(
                source_entries(ledger.advance(&mut reader, n).unwrap()),
                source_entries(&original_sources(&raw, n).unwrap())
            );
        }
        assert_eq!(ledger.stats.raw_visits, 6);
        assert_eq!(
            ledger.stats.coord_decodes, 4,
            "transient/future coords never decoded"
        );
        assert_eq!(ledger.stats.source_hashes, 12);
        assert_eq!(
            ledger.advance(&mut reader, 7).err().unwrap(),
            original_sources(&raw, 7).err().unwrap()
        );
        assert!(
            ledger.sources.is_empty() && ledger.originals.is_empty() && ledger.winners.is_empty()
        );
        ledger.advance(&mut reader, 6).unwrap();
        assert_eq!(
            ledger.stats.source_hashes, 18,
            "failure removed prior successful originals"
        );
        assert_eq!(
            ledger.advance(&mut reader, 5).err().unwrap(),
            original_sources(&raw, 5).err().unwrap()
        );
        assert!(ledger.originals.is_empty());
        let mut collision = vec![ledger_event("a", "7", 1, 0), ledger_event("b", "7", 1, 1)];
        collision[0]["seq"] = Value::Null;
        let mut ledger = NativeRawLedger::new((&collision).into());
        assert_eq!(
            ledger.advance(&mut reader, 2).err().unwrap(),
            original_sources(&collision, 2).err().unwrap()
        );
        assert_eq!(
            ledger.stats.source_hashes, 0,
            "final collision before RawReference or hash"
        );
        assert!(ledger.advance(&mut reader, 3).is_err());
        assert!(ledger.sources.is_empty() && ledger.originals.is_empty());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_raw_ledger_invalid_source_only_checked_for_final_winner() {
        let mut bad = ledger_event("a", "5", 2, 1);
        bad["seq"] = Value::Null;
        let raw = vec![
            ledger_event("a", "5", 1, 0),
            bad,
            ledger_event("a", "5", 3, 2),
        ];
        let capture = sc::AuditCapture::new(MemoryNodes::default());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let mut ledger = NativeRawLedger::new((&raw).into());
        ledger.advance(&mut reader, 1).unwrap();
        ledger.advance(&mut reader, 3).unwrap();
        assert_eq!(ledger.stats.source_hashes, 6);
        assert_eq!(
            ledger.advance(&mut reader, 2).err().unwrap(),
            original_sources(&raw, 2).err().unwrap()
        );
        assert!(ledger.originals.is_empty());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_raw_ledger_real_multicut_matches_full_source_audit() {
        // 复用已通过的不同 OHLC 夹具；每个前缀有实际来源变化及合法组/形状依赖。
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        let c = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&c).unwrap();
        let get = |k| meta_get_opt(&c, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        let rule = get("rule_revision");
        let catalog = get("catalog_revision");
        let scope = serde_json::from_str(&get("scope")).unwrap();
        let mut nodes = MemoryNodes::default();
        let mut roots = Vec::new();
        for n in 1..=raw.len() {
            let mut runtime =
                GenerationRuntime::new(format!("source-ledger-{n}"), tb02a::PROFILE, &rule);
            let effective = read_effective_events(&raw[..n]).unwrap();
            let cut = format!("cut-{n}");
            roots.push(
                runtime
                    .generate(
                        &mut nodes,
                        &GenerationInput {
                            all_events: (&raw[..n]).into(),
                            effective: (&effective).into(),
                            generation: n as i64,
                            structure_cut: &cut,
                            input_frontier: n as i64 - 1,
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
                        },
                    )
                    .unwrap()
                    .generated_state,
            );
        }
        let capture = sc::AuditCapture::new(nodes);
        let mut native = NativeSourceAudit::new(&capture, (&raw).into()).unwrap();
        for n in (1..=raw.len()).chain([3, raw.len(), 3, raw.len()]) {
            let root = &roots[n - 1];
            let mut ordinary = super::super::history::HashReader::captured(&capture, None);
            let want = audit_generated_for_audit(
                &mut ordinary,
                root,
                &raw[..n],
                tb02a::PROFILE,
                &rule,
                None,
                None,
            )
            .unwrap();
            let got = native.audit(root, n, tb02a::PROFILE, &rule).unwrap();
            assert_eq!(
                format!("{got:?}"),
                format!("{want:?}"),
                "real cut={n} root={}",
                root.root
            );
            native.reader.clear_overlay();
        }
        let stats = native.raw_ledger_stats();
        assert_eq!(stats.source_hashes, raw.len() as u64 * 3);
        assert_eq!(stats.coord_decodes, raw.len() as u64);
        assert_eq!(stats.rewinds, 2);
        assert!(stats.raw_visits < (raw.len() * (raw.len() + 1) / 2) as u64);
        assert_eq!(native.ledger.originals.len(), raw.len());
        assert!(native.audit(&roots[0], 1, "wrong-profile", &rule).is_err());
        assert!(native.ledger.sources.is_empty() && native.ledger.originals.is_empty());
        native.audit(&roots[0], 1, tb02a::PROFILE, &rule).unwrap();
        assert_eq!(
            native.raw_ledger_stats().source_hashes,
            stats.source_hashes + 3
        );
    }
    #[test]
    fn source_fields_preserves_owned_collection_element_layout() {
        assert_eq!(
            std::mem::size_of::<SourceFields<'static>>(),
            std::mem::size_of::<super::super::audit_store::RefFields<'static>>()
        );
        assert_eq!(
            std::mem::align_of::<SourceFields<'static>>(),
            std::mem::align_of::<super::super::audit_store::RefFields<'static>>()
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_fields_multi_cut_reuses_only_decoding_and_error_clears() {
        let (mut nodes, first, raw, rule) = fixture();
        let second = replace_field(
            &mut nodes,
            &first,
            STATE,
            "generation",
            V::Number(sc::NumberToken::parse("2").unwrap()),
        );
        let second = replace_field(
            &mut nodes,
            &second,
            STATE,
            "structure_cut",
            V::String("cut-2".into()),
        );
        let capture = sc::AuditCapture::new(nodes);
        let mut native = NativeSourceAudit::new(&capture, (&raw).into()).unwrap();
        let mut decodes = None;
        for state in [&first, &second, &first] {
            let mut ordinary = super::super::history::HashReader::captured(&capture, None);
            let want = audit_generated_for_audit(
                &mut ordinary,
                state,
                &raw,
                tb02a::PROFILE,
                &rule,
                None,
                None,
            )
            .unwrap();
            let got = native
                .audit(state, raw.len(), tb02a::PROFILE, &rule)
                .unwrap();
            assert_eq!(format!("{want:?}"), format!("{got:?}"));
            let now = (native.fields.field_decodes, native.fields.scalars.decodes);
            if let Some(first) = decodes {
                assert_eq!(now, first);
            } else {
                decodes = Some(now);
            }
        }
        assert!(native.fields.field_hits > 0 && native.fields.scalars.hits > 0);
        assert!(native
            .audit(&first, raw.len() + 1, tb02a::PROFILE, &rule)
            .is_err());
        assert!(native.fields.fields.is_empty() && native.fields.scalars.values.is_empty());
        let before = native.fields.scalars.decodes;
        native
            .audit(&first, raw.len(), tb02a::PROFILE, &rule)
            .unwrap();
        assert!(native.fields.scalars.decodes > before);
        let fresh = NativeSourceAudit::new(&capture, (&raw).into()).unwrap();
        assert!(fresh.fields.fields.is_empty() && fresh.fields.scalars.values.is_empty());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_fields_exact_headers_names_and_overlay_isolation() {
        let mut nodes = MemoryNodes::default();
        let root = sc::write_value(&mut nodes, &cv(&json!({"a":"original","b":7})).unwrap())
            .unwrap()
            .content_ref();
        let capture = sc::AuditCapture::new(nodes);
        let mut memo = NativeFieldMemo::new(&capture);
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        {
            let mut store = super::super::audit_store::StoreView::for_audit(&mut reader);
            let mut source = ArrayAuditSource {
                source: &mut store,
                full_arrays: true,
                deadline: None,
                fields: Some(&mut memo),
            };
            let fields = SourceFields::read_subset(&mut source, (&root).into(), &["a"]).unwrap();
            assert_eq!(fields.text(&mut source, "a").unwrap(), "original");
            assert_eq!(fields.text(&mut source, "a").unwrap(), "original");
            assert!(fields.get("b").is_err());
            let scalar_root = built(fields.get("a").unwrap().view())
                .unwrap()
                .content_ref();
            let mut forged_scalar = scalar_root.clone();
            forged_scalar.format = "bad-scalar-format".into();
            assert!(source.decoded_scalar((&forged_scalar).into()).is_err());
            forged_scalar = scalar_root.clone();
            forged_scalar.kind = Kind::Bool;
            assert!(source.decoded_scalar((&forged_scalar).into()).is_err());
            let b = SourceFields::read_subset(&mut source, (&root).into(), &["b"]).unwrap();
            assert_eq!(b.integer(&mut source, "b").unwrap(), 7);
            assert!(b.get("a").is_err());
            for names in [&["a", "b"][..], &["b", "a"][..]] {
                SourceFields::read_subset(&mut source, (&root).into(), names).unwrap();
            }
            let expected_decodes = source.fields.as_ref().unwrap().field_decodes;
            SourceFields::read_subset(&mut source, (&root).into(), &["b", "a"]).unwrap();
            assert_eq!(
                source.fields.as_ref().unwrap().field_decodes,
                expected_decodes
            );
            // Duplicate and reordered selections keep the original decoder's outcome;
            // ordered names, including duplicate occurrences, never share a cache key.
            let memo_handle = source.fields.take();
            let ordinary_duplicate =
                SourceFields::read_subset(&mut source, (&root).into(), &["a", "a"]);
            source.fields = memo_handle;
            let native_duplicate =
                SourceFields::read_subset(&mut source, (&root).into(), &["a", "a"]);
            assert_eq!(ordinary_duplicate.is_ok(), native_duplicate.is_ok());
            if let (Ok(ordinary), Ok(native)) = (ordinary_duplicate, native_duplicate) {
                assert_eq!(
                    built(ordinary.get("a").unwrap().view()).unwrap().digest,
                    built(native.get("a").unwrap().view()).unwrap().digest
                );
            }
            let mut bad = root.clone();
            bad.format = "invalid-format".into();
            assert!(SourceFields::read_subset(&mut source, (&bad).into(), &["a"]).is_err());
            bad = root.clone();
            bad.extent = Some(root.extent.unwrap() + 1);
            assert!(SourceFields::read_subset(&mut source, (&bad).into(), &["a"]).is_err());
            bad = root.clone();
            bad.kind = Kind::Array;
            assert!(SourceFields::read_subset(&mut source, (&bad).into(), &["a"]).is_err());
            // Full root text stays part of identity; a malformed spelling cannot reuse hot fields.
            bad = root.clone();
            bad.root.push('0');
            assert!(SourceFields::read_subset(&mut source, (&bad).into(), &["a"]).is_err());
            let field_count = source.fields.as_ref().unwrap().fields.len();
            let scalar_count = source.fields.as_ref().unwrap().scalars.values.len();
            let overlay = source
                .write_json_view((&json!({"a":"overlay"})).into())
                .unwrap()
                .content_ref();
            let fields = SourceFields::read_subset(&mut source, (&overlay).into(), &["a"]).unwrap();
            assert_eq!(fields.text(&mut source, "a").unwrap(), "overlay");
            assert_eq!(source.fields.as_ref().unwrap().fields.len(), field_count);
            assert_eq!(
                source.fields.as_ref().unwrap().scalars.values.len(),
                scalar_count
            );
        }
        assert!(memo.field_hits > 0 && memo.scalars.hits > 0);
        let fresh = NativeFieldMemo::new(&capture);
        assert!(fresh.fields.is_empty() && fresh.scalars.values.is_empty());
        let other = sc::AuditCapture::new(MemoryNodes::default());
        assert!(!NativeFieldMemo::new(&other)
            .qualified((&root).into())
            .unwrap());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_witness_identity_exact_inputs_and_success_only() {
        let mut nodes = MemoryNodes::default();
        let bars = sc::write_value(&mut nodes, &V::Array(vec![V::String("bar".into())])).unwrap();
        let other =
            sc::write_value(&mut nodes, &V::Array(vec![V::String("changed".into())])).unwrap();
        let capture = sc::AuditCapture::new(nodes);
        let mut memo = NativeWitnessIdentityMemo::new(&capture).unwrap();
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let original = |id: &str, slot, mi, bars: &Built| {
            let mut reader = super::super::history::HashReader::captured(&capture, None);
            super::super::generation::witness_identity(&mut reader, id, slot, mi, bars)
        };
        let expected = original("obj3-first", 0, 1, &bars).unwrap();
        assert!(memo
            .check_identity(&mut reader, "wrong", "obj3-first", 0, 1, &bars)
            .is_err());
        assert!(memo.entries.is_empty());
        memo.check_identity(&mut reader, &expected, "obj3-first", 0, 1, &bars)
            .unwrap();
        let rendered = memo.stats.render_calls;
        memo.check_identity(&mut reader, &expected, "obj3-first", 0, 1, &bars)
            .unwrap();
        assert_eq!(memo.stats.render_calls, rendered);
        assert_eq!(memo.stats.hits, 1);
        assert!(memo
            .check_identity(&mut reader, "wrong", "obj3-first", 0, 1, &bars)
            .is_err());
        for (id, slot, mi, input) in [
            ("obj3-second", 0, 1, &bars),
            ("obj3-first", 1, 1, &bars),
            ("obj3-first", 0, 2, &bars),
            ("obj3-first", 0, 1, &other),
        ] {
            let expected = original(id, slot, mi, input).unwrap();
            let before = memo.stats.render_calls;
            memo.check_identity(&mut reader, &expected, id, slot, mi, input)
                .unwrap();
            assert_eq!(memo.stats.render_calls, before + 1);
        }
        let key = memo.key("obj3-first", 0, 1, &bars);
        let mut changed = bars.clone();
        changed.kind = Kind::Object;
        assert_ne!(key, memo.key("obj3-first", 0, 1, &changed));
        changed = bars.clone();
        changed.extent = Some(999);
        assert_ne!(key, memo.key("obj3-first", 0, 1, &changed));
        let count = memo.entries.len();
        assert!(memo
            .check_identity(&mut reader, "unused", "bad\"id", 0, 1, &bars)
            .is_err());
        assert_eq!(memo.entries.len(), count);
        assert!(memo
            .check_identity(&mut reader, &expected, "obj3-first", 0, 1, &changed)
            .is_err());
        assert_eq!(memo.entries.len(), count);
        let another = sc::AuditCapture::new(MemoryNodes::default());
        let fresh = NativeWitnessIdentityMemo::new(&another).unwrap();
        assert!(fresh.entries.is_empty());
        assert_ne!(key, fresh.key("obj3-first", 0, 1, &bars));
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_witness_identity_keeps_current_cut_faults() {
        let (mut nodes, state, raw, rule) = fixture();
        let witnesses = field(&mut nodes, &state, "witnesses").unwrap();
        let witnesses = elements(&mut nodes, &witnesses).unwrap();
        let first = &witnesses[0];
        let wrong_id = replace_field(
            &mut nodes,
            first,
            WITNESS,
            "witness_id",
            V::String("wrong".into()),
        );
        let wrong_id = replace_member(&mut nodes, &state, "witnesses", 0, wrong_id);
        let changed_bars = sc::write_value(&mut nodes, &V::Array(vec![])).unwrap();
        let wrong_bars = replace_field(
            &mut nodes,
            first,
            WITNESS,
            "raw_bars",
            V::Edge(changed_bars.digest),
        );
        let wrong_bars = replace_member(&mut nodes, &state, "witnesses", 0, wrong_bars);
        let duplicate = replace_member(
            &mut nodes,
            &state,
            "witnesses",
            1,
            built(first.view()).unwrap().content_ref(),
        );
        let missing = replace_field(
            &mut nodes,
            &state,
            STATE,
            "witnesses",
            V::Array(
                witnesses
                    .iter()
                    .skip(1)
                    .map(|r| V::Edge(digest(r).unwrap()))
                    .collect(),
            ),
        );
        let capture = sc::AuditCapture::new(nodes);
        let mut context = NativeSourceAudit::new(&capture, (&raw).into()).unwrap();
        let run = |context: &mut NativeSourceAudit<'_>, state: &ContentRef, raw: &[Value]| {
            context.audit(state, raw.len(), tb02a::PROFILE, &rule)
        };
        let mut original_reader = super::super::history::HashReader::captured(&capture, None);
        let original = audit_generated_for_audit(
            &mut original_reader,
            &state,
            &raw,
            tb02a::PROFILE,
            &rule,
            None,
            None,
        )
        .unwrap();
        let first = run(&mut context, &state, &raw).unwrap();
        let field_decodes = context.fields.field_decodes;
        let scalar_decodes = context.fields.scalars.decodes;
        assert!(field_decodes > 0 && scalar_decodes > 0);
        assert_eq!(format!("{original:?}"), format!("{first:?}"));
        let renders = context.memo.stats.render_calls;
        let source_stats = context.source_memo.stats;
        run(&mut context, &state, &raw).unwrap();
        assert_eq!(context.memo.stats.render_calls, renders);
        assert_eq!(context.fields.field_decodes, field_decodes);
        assert_eq!(context.fields.scalars.decodes, scalar_decodes);
        assert!(context.fields.field_hits > 0 && context.fields.scalars.hits > 0);
        assert!(context.memo.stats.hits > 0);
        assert!(context.source_memo.stats[0] > source_stats[0]);
        assert!(context.source_memo.stats[2] > source_stats[2]);
        assert_eq!(context.source_memo.stats[1], source_stats[1]);
        assert_eq!(context.source_memo.stats[3], source_stats[3]);
        assert!(context
            .audit(&state, raw.len() + 1, tb02a::PROFILE, &rule)
            .is_err());
        assert!(context.fields.fields.is_empty() && context.fields.scalars.values.is_empty());
        // Future malformed/colliding raw cannot invalidate an earlier legal cut at init.
        for coord in ["-1", "01", raw[0]["source_coord"].as_str().unwrap()] {
            let mut future_raw = raw.clone();
            let mut future = raw[0].clone();
            future["identity_key"] = json!("future-different-identity");
            future["source_coord"] = json!(coord);
            future_raw.push(future);
            let mut future_context =
                NativeSourceAudit::new(&capture, (&future_raw).into()).unwrap();
            let previous = future_context
                .audit(&state, raw.len(), tb02a::PROFILE, &rule)
                .unwrap();
            assert_eq!(format!("{previous:?}"), format!("{original:?}"));
            assert!(future_context
                .audit(&state, future_raw.len(), tb02a::PROFILE, &rule)
                .is_err());
        }
        for (state, message) in [
            (&wrong_id, "见证ID不符"),
            (&wrong_bars, "raw_bars成员数不同"),
            (&duplicate, "见证槽位重复"),
            (&missing, "缺完整三槽"),
        ] {
            assert!(run(&mut context, state, &raw)
                .unwrap_err()
                .contains(message));
        }
        let mut revised = raw.clone();
        revised[0]["high"] = json!("999999");
        let mut revised_context = NativeSourceAudit::new(&capture, (&revised).into()).unwrap();
        assert!(run(&mut revised_context, &state, &revised)
            .unwrap_err()
            .contains("raw_bars不是同cut真实原始成员"));
        let mut revised = raw.clone();
        let revision = revised[0]["revision"].as_i64().unwrap();
        revised[0]["revision"] = json!(revision.checked_add(1).unwrap());
        let mut revised_context = NativeSourceAudit::new(&capture, (&revised).into()).unwrap();
        assert!(run(&mut revised_context, &state, &revised)
            .unwrap_err()
            .contains("源引用不是本cut的真实完整八字段原件"));
        let empty = sc::AuditCapture::new(MemoryNodes::default());
        let mut wrong_source = NativeSourceAudit::new(&empty, (&raw).into()).unwrap();
        assert!(run(&mut wrong_source, &state, &raw).is_err());
        assert!(wrong_source.memo.entries.is_empty());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_witness_identity_requires_original_complete_closure() {
        let mut nodes = MemoryNodes::default();
        let bars =
            sc::write_value(&mut nodes, &V::Array(vec![V::String("original".into())])).unwrap();
        let mut incomplete = MemoryNodes::default();
        let (tag, body) = nodes.get(&bars.digest).unwrap();
        incomplete.insert_unchecked(bars.digest, tag, body.to_vec());
        let incomplete = sc::AuditCapture::new(incomplete);
        let mut context = NativeSourceAudit::new(&incomplete, RawEventsView::plain(&[])).unwrap();
        assert!(context
            .memo
            .check_identity(&mut context.reader, "unused", "obj3-first", 0, 1, &bars)
            .is_err());
        assert!(context.memo.entries.is_empty());
        let capture = sc::AuditCapture::new(nodes);
        let mut context = NativeSourceAudit::new(&capture, RawEventsView::plain(&[])).unwrap();
        let overlay = sc::write_value(
            &mut context.reader,
            &V::Array(vec![V::String("overlay".into())]),
        )
        .unwrap();
        assert!(!capture.contains(&overlay.digest));
        let expected = super::super::generation::witness_identity(
            &mut context.reader,
            "obj3-overlay",
            0,
            1,
            &overlay,
        )
        .unwrap();
        for _ in 0..2 {
            context
                .memo
                .check_identity(
                    &mut context.reader,
                    &expected,
                    "obj3-overlay",
                    0,
                    1,
                    &overlay,
                )
                .unwrap();
        }
        assert!(context.memo.entries.is_empty());
        assert_eq!(context.memo.stats.hits, 0);
        assert_eq!(context.memo.stats.render_calls, 2);
        let budget = sc::capture_allocation::OwnedBudget::new(1024);
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(
            sc::capture_allocation::CaptureSlab::new(0, 0, &budget)
                .unwrap()
                .finish()
                .unwrap(),
        );
        assert!(NativeSourceAudit::new(&owned, RawEventsView::plain(&[])).is_err());
    }
    fn replace_field<'a>(
        s: &mut dyn NodeStore,
        r: impl Into<sc::RefView<'a>>,
        names: &[&str],
        key: &str,
        replacement: V,
    ) -> ContentRef {
        let r = r.into();
        let fields = names
            .iter()
            .map(|name| {
                (
                    (*name).into(),
                    if *name == key {
                        replacement.clone()
                    } else {
                        V::Edge(field(s, r, name).unwrap().digest().unwrap())
                    },
                )
            })
            .collect();
        sc::write_value(s, &V::Object(fields))
            .unwrap()
            .content_ref()
    }
    fn replace_member(
        s: &mut dyn NodeStore,
        state: &ContentRef,
        collection: &str,
        index: usize,
        record: ContentRef,
    ) -> ContentRef {
        let values = field(s, state, collection).unwrap();
        let mut elements = elements(s, &values).unwrap();
        elements[index] = RefValue::Plain(record);
        replace_field(
            s,
            state,
            STATE,
            collection,
            V::Array(
                elements
                    .into_iter()
                    .map(|r| V::Edge(r.digest().unwrap()))
                    .collect(),
            ),
        )
    }
    struct Reads<'a> {
        source: &'a mut dyn NodeSource,
        digests: Vec<NodeDigest>,
    }
    impl NodeSource for Reads<'_> {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.digests.push(*d);
            self.source.read(d)
        }
        fn checked_read(
            &mut self,
            d: &NodeDigest,
        ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
            self.digests.push(*d);
            self.source.checked_read(d)
        }
    }
    #[derive(Default)]
    struct Meter {
        events: std::cell::RefCell<Vec<String>>,
        stop_at: Option<usize>,
    }
    impl super::super::audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            self.events.borrow_mut().push("precheck".into());
            if self.stop_at == Some(self.events.borrow().len()) {
                Err(sc::CodecError::new("field_stop", "当前预算门拒绝"))
            } else {
                Ok(())
            }
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> {
            self.precheck()?;
            self.events.borrow_mut().push("prepare".into());
            Ok(())
        }
        fn accept_prepared_frame(
            &self,
            frame: &sc::job::CheckedFrame,
        ) -> Result<(), sc::CodecError> {
            self.events
                .borrow_mut()
                .push(sc::root_string(frame.digest()));
            Ok(())
        }
        fn node(&self, d: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            self.prepare_node()?;
            self.accept_prepared_frame(&sc::job::CheckedFrame::checked(d, tag, body.to_vec())?)
        }
        fn record(&self) -> Result<(), sc::CodecError> {
            self.precheck()?;
            self.events.borrow_mut().push("record".into());
            Ok(())
        }
    }
    // 与生产顺序一致：part 先整数，再两内容边；shape payload 先 high 文本再 low；
    // witness payload 只逐项 Ref 对拍，不额外 render。
    fn read_local(
        s: &mut dyn NodeStore,
        root: &ContentRef,
        names: &[&str],
        legacy: bool,
    ) -> Result<Vec<(ContentRef, Option<Value>)>, String> {
        let fields = if legacy {
            None
        } else {
            local_fields(s, root, names)?
        };
        names
            .iter()
            .map(|name| {
                let r = if let Some(fields) = &fields {
                    fields.get(name).map_err(err)?.clone()
                } else {
                    field(s, root, name)?
                };
                let value = if *name == "merged_index" || names.len() == 2 {
                    let value = scalar_value(s, &r)?;
                    if *name == "merged_index" {
                        value
                            .view()
                            .as_i64()
                            .ok_or_else(|| err("merged_index不是整数"))?;
                    } else {
                        value
                            .view()
                            .as_str()
                            .ok_or_else(|| err(format!("{name}不是文本")))?;
                    }
                    Some(value)
                } else {
                    None
                };
                let reference = ContentRef {
                    format: r.view().format().to_owned(),
                    root: r.root().to_owned(),
                    kind: r.kind(),
                    extent: r.extent(),
                };
                let value = value.map(|v| match v {
                    bj::JsonOwner::Plain(v) => v,
                    #[cfg(feature = "s_allocation_json_testonly")]
                    bj::JsonOwner::Owned(v) => serde_json::to_value(v.view())
                        .expect("test-only logical oracle serialization"),
                });
                Ok((reference, value))
            })
            .collect()
    }
    fn local_roots(s: &mut MemoryNodes, state: &ContentRef) -> (ContentRef, ContentRef) {
        let objects = field(s, state, "objects").unwrap();
        let objects = elements(s, &objects).unwrap();
        let shape = objects
            .iter()
            .find(|r| text(s, *r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let refs = field(s, shape, "input_refs_json").unwrap();
        let refs = sc::path::lookup_path_refview(s, refs.view(), &[PathStep::JsonText]).unwrap();
        let part = elements(s, &refs).unwrap().into_iter().next().unwrap();
        let index = number(s, &part, "merged_index").unwrap();
        let payload = objects
            .iter()
            .find_map(|r| {
                if text(s, r, "kind").unwrap() == CC005_GROUP_KIND {
                    let payload = field(s, r, "payload_json").unwrap();
                    let payload =
                        sc::path::lookup_path_refview(s, payload.view(), &[PathStep::JsonText])
                            .unwrap();
                    if text(s, &payload, "group_index").unwrap() == index.to_string() {
                        return Some(payload);
                    }
                }
                None
            })
            .unwrap();
        (
            ContentRef {
                format: part.view().format().to_owned(),
                root: part.root().to_owned(),
                kind: part.kind(),
                extent: part.extent(),
            },
            payload,
        )
    }
    #[test]
    fn local_none_subsets_keep_ordered_refs_types_values_and_reduce_real_reads() {
        let (mut nodes, state, _, _) = fixture();
        let (part, payload) = local_roots(&mut nodes, &state);
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let mut sink = crate::content_store::TxStore::new(&tx).unwrap();
        for (d, tag, body) in nodes.iter() {
            sink.put(*d, tag, body).unwrap();
        }
        for (root, names) in [
            (&part, &["merged_index", "raw_refs", "dependency_refs"][..]),
            (&payload, &["high", "low"][..]),
            (&payload, &["high", "low", "open", "close"][..]),
        ] {
            let run = |legacy| {
                let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
                let mut reads = Reads {
                    source: &mut source,
                    digests: vec![],
                };
                let sql = crate::content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let mut reader = super::super::history::HashReader::new(&mut reads);
                let mut typed_view = super::super::audit_store::StoreView::for_audit(&mut reader);
                let mut s = ArrayAuditSource {
                    source: &mut typed_view,
                    full_arrays: true,
                    deadline: None,
                    fields: None,
                };
                let result = read_local(&mut s, root, names, legacy).unwrap();
                drop(reader);
                (
                    result,
                    crate::content_store::sql_reads_on_thread() - sql,
                    sc::frame_calls_on_thread() - frames,
                    reads.digests,
                )
            };
            let old = run(true);
            let actual = run(false);
            assert_eq!(
                actual.0, old.0,
                "顺序、ContentRef、kind、extent及render值一致"
            );
            let root_digest = digest(root).unwrap();
            let roots = |ds: &[NodeDigest]| ds.iter().filter(|d| **d == root_digest).count();
            assert_eq!(roots(&old.3), names.len(), "旧路径每字段重复根寻路");
            eprintln!(
                "local {names:?}: SQL {} -> {}, F {} -> {}, root {} -> {}",
                old.1,
                actual.1,
                old.2,
                actual.2,
                roots(&old.3),
                roots(&actual.3)
            );
            assert_eq!(roots(&actual.3), 1, "None 同一局部根只批读一次");
            assert!(actual.1 < old.1, "必须减少真实SQL读取");
            assert_eq!(actual.2, actual.1, "每个实际节点读仍核本次F");
        }
    }
    #[test]
    fn local_some_keeps_sql_f_budget_error_order_and_deadline_before_sql() {
        let (mut nodes, state, _, _) = fixture();
        let (part, payload) = local_roots(&mut nodes, &state);
        let wrong_index = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("merged_index".into(), V::String("not-an-integer".into())),
                ("raw_refs".into(), V::Null),
                ("dependency_refs".into(), V::Null),
            ]),
        )
        .unwrap()
        .content_ref();
        let wrong_high_missing_low =
            sc::write_value(&mut nodes, &V::Object(vec![("high".into(), V::Bool(true))]))
                .unwrap()
                .content_ref();
        sc::verify(&mut nodes, &wrong_index).unwrap();
        sc::verify(&mut nodes, &wrong_high_missing_low).unwrap();
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let mut sink = crate::content_store::TxStore::new(&tx).unwrap();
        for (d, tag, body) in nodes.iter() {
            sink.put(*d, tag, body).unwrap();
        }
        for (root, names) in [
            (&part, &["merged_index", "raw_refs", "dependency_refs"][..]),
            (&payload, &["high", "low"][..]),
            (&payload, &["high", "low", "open", "close"][..]),
            (
                &wrong_index,
                &["merged_index", "raw_refs", "dependency_refs"][..],
            ),
            (&wrong_high_missing_low, &["high", "low"][..]),
        ] {
            let run = |legacy, stop_at, deadline| {
                let meter = Meter {
                    stop_at,
                    ..Meter::default()
                };
                let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
                source.enable_audit_same_material();
                let sql = crate::content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let mut reader = super::super::history::HashReader::metered(
                    &mut source,
                    deadline,
                    None,
                    Some(&meter),
                );
                let mut typed_view = super::super::audit_store::StoreView::for_audit(&mut reader);
                let mut s = ArrayAuditSource {
                    source: &mut typed_view,
                    full_arrays: false,
                    deadline,
                    fields: None,
                };
                let result = read_local(&mut s, root, names, legacy);
                drop(reader);
                (
                    result,
                    crate::content_store::sql_reads_on_thread() - sql,
                    sc::frame_calls_on_thread() - frames,
                    meter.events.into_inner(),
                )
            };
            let old = run(true, None, None);
            assert_eq!(run(false, None, None), old);
            if root == &wrong_index {
                assert_eq!(old.0, Err(err("merged_index不是整数")));
            } else if root == &wrong_high_missing_low {
                assert_eq!(
                    old.0,
                    Err(err("high不是文本")),
                    "Some须先拒绝high类型，不能提前读取缺失的low"
                );
            }
            assert_eq!(old.2, old.1 * 2, "Some 保持旧SQL源F和HashReader F");
            // 遍历旧成功轨迹的每一道预算前门，包含跨字段/render的即时读取交错。
            for (at, event) in old.3.iter().enumerate() {
                if event == "precheck" {
                    assert_eq!(
                        run(false, Some(at + 1), None),
                        run(true, Some(at + 1), None),
                        "Some stop_at={} 必须保持Ref/错误、SQL、F和预算门顺序",
                        at + 1
                    );
                }
            }
            let deadline = Some(std::time::Instant::now());
            let expired = run(false, None, deadline);
            assert_eq!(expired, run(true, None, deadline));
            assert!(expired.0.unwrap_err().contains("verify_deadline"));
            assert_eq!((expired.1, expired.2), (0, 0));
            assert!(expired.3.is_empty(), "期限门先于预算门与SQL");
        }
    }
    #[test]
    fn local_subset_budget_and_deadline_opt_in_keep_legacy_sql_f_and_gate_order() {
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let names = (0..15).map(|i| format!("field-{i:02}")).collect::<Vec<_>>();
        let keys = names[..10].iter().map(String::as_str).collect::<Vec<_>>();
        let root = sc::write_value(
            &mut crate::content_store::TxStore::new(&tx).unwrap(),
            &V::Object(names.iter().map(|k| (k.clone(), V::Null)).collect()),
        )
        .unwrap()
        .content_ref();
        let run = |legacy, budget_some, deadline, stop_at| {
            let meter = Meter {
                stop_at,
                ..Meter::default()
            };
            let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
            source.enable_audit_same_material();
            let mut reads = Reads {
                source: &mut source,
                digests: vec![],
            };
            let sql = crate::content_store::sql_reads_on_thread();
            let frames = sc::frame_calls_on_thread();
            let mut reader = super::super::history::HashReader::metered(
                &mut reads,
                deadline,
                None,
                if budget_some { Some(&meter) } else { None },
            );
            reader.enable_full_arrays_for_test();
            assert!(!reader.audit_full_array_scan());
            let mut typed_view = super::super::audit_store::StoreView::for_audit(&mut reader);
            let mut s = ArrayAuditSource {
                source: &mut typed_view,
                full_arrays: !budget_some,
                deadline,
                fields: None,
            };
            assert!(!s.audit_full_array_scan());
            let fields = if legacy {
                sc::path::lookup_object_fields_subset(&mut s, &root, &keys).map_err(err)
            } else {
                // 上层保留逐字段业务顺序；同时锁低层 Subset 不受本次优化影响。
                assert!(local_fields(&mut s, &root, &keys).unwrap().is_none());
                sc::path::lookup_object_fields_subset(&mut s, &root, &keys).map_err(err)
            };
            drop(reader);
            (
                fields,
                crate::content_store::sql_reads_on_thread() - sql,
                sc::frame_calls_on_thread() - frames,
                reads.digests,
                meter.events.into_inner(),
            )
        };
        for (budget, deadline) in [
            (true, None),
            (
                false,
                Some(std::time::Instant::now() + std::time::Duration::from_secs(3600)),
            ),
            (
                true,
                Some(std::time::Instant::now() + std::time::Duration::from_secs(3600)),
            ),
        ] {
            let old = run(true, budget, deadline, None);
            assert_eq!(run(false, budget, deadline, None), old);
            assert!(old.0.is_ok(), "Some 原 Subset 结果：{:?}", old.0);
            eprintln!(
                "SUBSET_GATE budget={budget} deadline={} SQL={} F={} reads={}",
                deadline.is_some(),
                old.1,
                old.2,
                old.3.len()
            );
            assert_eq!(old.1, 199, "Some 的真实 SQL 读取保持原 15取10");
            // 期限 Some/预算 None 的旧源仍逐次 SQL 读，但同素材仅核 91 次纯 F。
            assert_eq!(old.2, if budget { 398 } else { 91 });
            if budget {
                for (at, event) in old.4.iter().enumerate() {
                    if event == "precheck" {
                        assert_eq!(
                            run(false, budget, deadline, Some(at + 1)),
                            run(true, budget, deadline, Some(at + 1))
                        );
                    }
                }
            }
        }
        let expired = Some(std::time::Instant::now());
        for budget in [false, true] {
            let old = run(true, budget, expired, None);
            assert_eq!(run(false, budget, expired, None), old);
            assert!(old.0.unwrap_err().contains("verify_deadline"));
            assert_eq!((old.1, old.2), (0, 0));
            assert!(old.3.is_empty() && old.4.is_empty());
        }
    }
    #[test]
    fn full_some_audit_keeps_frozen_read_and_budget_trace() {
        let (mut nodes, state, raw, rule) = fixture();
        let mut reads = Reads {
            source: &mut nodes,
            digests: vec![],
        };
        let meter = Meter::default();
        let frames = sc::frame_calls_on_thread();
        let mut reader =
            super::super::history::HashReader::metered(&mut reads, None, None, Some(&meter));
        let counts = audit_generated_metered(
            &mut reader,
            &state,
            &raw,
            tb02a::PROFILE,
            &rule,
            None,
            Some(&meter),
        )
        .unwrap();
        drop(reader);
        let frames = sc::frame_calls_on_thread() - frames;
        let roots = reads
            .digests
            .iter()
            .map(sc::root_string)
            .collect::<Vec<_>>();
        let read_hash = crate::sha256_hex(canonical_json(&json!(roots)).as_bytes());
        let budget_hash =
            crate::sha256_hex(canonical_json(&json!(meter.events.into_inner())).as_bytes());
        eprintln!("Some full: counts={counts:?}, reads={}, F={frames}, read_hash={read_hash}, budget_hash={budget_hash}", roots.len());
        // dc425df7 固定fixture的旧路径实测；不是由候选路径重算期望。
        assert_eq!(
            (
                counts.objects,
                counts.groups,
                counts.windows,
                counts.effective,
                counts.insufficient,
                counts.outside_domain
            ),
            (76, 13, 11, 14, 0, 0)
        );
        assert_eq!(roots.len(), 151351, "固定基线的完整Some读取轨迹");
        assert_eq!(frames, 170797, "固定基线的完整Some F次数");
        assert_eq!(
            read_hash,
            "668e022e318e445f163d23036d33cccefb5e735a82ae97ce6db8b453b936d69a"
        );
        assert_eq!(
            budget_hash,
            "dc5c1b67c63046ecb4edf84c12e84c784793112fff94bc4a449cef1c8befd1b2"
        );
    }
    fn json_text_edge(s: &mut dyn NodeStore, value: &ContentRef) -> ContentRef {
        let body = digest(value).unwrap();
        let d = sc::frame(sc::tag::JSON_TEXT, &body);
        s.put(d, sc::tag::JSON_TEXT, &body).unwrap();
        ContentRef::new(Kind::JsonText, &d, None)
    }
    fn assert_rejected_in_both_modes(
        s: &mut MemoryNodes,
        state: &ContentRef,
        raw: &[Value],
        rule: &str,
        reason: &str,
    ) {
        sc::verify(s, state).unwrap();
        let none = audit_generated(s, state, raw, tb02a::PROFILE, rule).unwrap_err();
        let meter = Meter::default();
        let mut reader = super::super::history::HashReader::metered(s, None, None, Some(&meter));
        let some = audit_generated_metered(
            &mut reader,
            state,
            raw,
            tb02a::PROFILE,
            rule,
            None,
            Some(&meter),
        )
        .unwrap_err();
        assert!(none.contains(reason), "{none}");
        assert_eq!(none, some, "合法Merkle畸形在None/Some均须由原业务判据拒绝");
    }
    #[test]
    fn valid_merkle_shape_dependency_forgery_is_rejected_after_none_subset() {
        let (mut s, state, raw, rule) = fixture();
        let objects = field(&mut s, &state, "objects").unwrap();
        let objects = elements(&mut s, &objects).unwrap();
        let shape = objects
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let refs = field(&mut s, &objects[shape], "input_refs_json").unwrap();
        let refs =
            sc::path::lookup_path_refview(&mut s, refs.view(), &[PathStep::JsonText]).unwrap();
        let mut parts = elements(&mut s, &refs).unwrap();
        let members = field(&mut s, &parts[0], "raw_refs").unwrap();
        // 真成员不是来源差集；不引入坏节点/缺节点，必须由差集判据拒绝。
        parts[0] = RefValue::Plain(replace_field(
            &mut s,
            &parts[0],
            &["merged_index", "raw_refs", "dependency_refs"],
            "dependency_refs",
            V::Edge(digest(&members).unwrap()),
        ));
        let refs = sc::write_value(
            &mut s,
            &V::Array(parts.iter().map(|r| V::Edge(digest(r).unwrap())).collect()),
        )
        .unwrap()
        .content_ref();
        let refs = json_text_edge(&mut s, &refs);
        let changed = replace_field(
            &mut s,
            &objects[shape],
            SHAPE_FIELDS,
            "input_refs_json",
            V::Edge(digest(&refs).unwrap()),
        );
        let forged = replace_member(&mut s, &state, "objects", shape, changed);
        assert_rejected_in_both_modes(
            &mut s,
            &forged,
            &raw,
            &rule,
            "dependency_refs不是组事实来源差集",
        );
    }
    #[test]
    fn valid_merkle_reidentified_group_high_still_breaks_shape_identity() {
        let (mut s, state, raw, rule) = fixture();
        let (_, payload) = local_roots(&mut s, &state);
        let objects = field(&mut s, &state, "objects").unwrap();
        let objects = elements(&mut s, &objects).unwrap();
        let group = objects
            .iter()
            .position(|r| {
                if text(&mut s, r, "kind").unwrap() != CC005_GROUP_KIND {
                    return false;
                }
                let p = field(&mut s, r, "payload_json").unwrap();
                RefValue::Plain(
                    sc::path::lookup_path_refview(&mut s, p.view(), &[PathStep::JsonText]).unwrap(),
                ) == RefValue::Plain(payload.clone())
            })
            .unwrap();
        let mut bytes = Vec::new();
        sc::render_canonical(&mut s, &payload, &mut bytes).unwrap();
        let mut value: Value = newchan_rust::session_protocol::strict_json(&bytes).unwrap();
        value["high"] = json!("999999");
        let changed_payload = sc::write_value(&mut s, &cv(&value).unwrap())
            .unwrap()
            .content_ref();
        let refs = field(&mut s, &objects[group], "input_refs_json").unwrap();
        let refs =
            sc::path::lookup_path_refview(&mut s, refs.view(), &[PathStep::JsonText]).unwrap();
        let key = text(&mut s, &objects[group], "fact_key").unwrap();
        let id = Cc004Producer::new(tb02a::PROFILE, &rule)
            .typed_object_id(
                CC005_GROUP_KIND,
                &key,
                &built(&changed_payload).unwrap(),
                &built(&refs).unwrap(),
            )
            .unwrap();
        let payload_json = json_text_edge(&mut s, &changed_payload);
        let changed = replace_field(
            &mut s,
            &objects[group],
            TYPED,
            "payload_json",
            V::Edge(digest(&payload_json).unwrap()),
        );
        let changed = replace_field(&mut s, &changed, TYPED, "object_id", V::String(id));
        let forged = replace_member(&mut s, &state, "objects", group, changed);
        // 连typed身份都重算成合法，仍须在shape high/low配方拒绝，而不是提前撞typed ID。
        assert_rejected_in_both_modes(
            &mut s,
            &forged,
            &raw,
            &rule,
            "CC006 obj3与同cut组事实身份不符",
        );
    }
    #[test]
    fn real_ohlc_objects_and_witnesses_bind_to_sources_without_reclassification() {
        let (mut s, state, raw, rule) = fixture();
        let result = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap();
        assert!(result.objects > result.groups);
        assert!(result.groups > 3);
        assert_eq!(result.effective, 14);
        assert_eq!(result.windows, result.groups - 2);
        let error =
            audit_generated(&mut s, &state, &raw, tb02a::PROFILE, "other-rule").unwrap_err();
        assert!(error.contains("typed obj3身份"), "{error}");
    }
    #[test]
    fn valid_merkle_graph_with_changed_typed_id_or_shape_recipe_is_rejected() {
        let (mut s, state, raw, rule) = fixture();
        let root = field(&mut s, &state, "objects").unwrap();
        let all = elements(&mut s, &root).unwrap();
        let typed = all
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() != CC006_SHAPE_KIND)
            .unwrap();
        let changed = replace_field(
            &mut s,
            &all[typed],
            TYPED,
            "object_id",
            V::String(format!("obj3-{}", "a".repeat(64))),
        );
        let forged = replace_member(&mut s, &state, "objects", typed, changed);
        sc::verify(&mut s, &forged).unwrap();
        assert!(
            audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule)
                .unwrap_err()
                .contains("typed obj3身份")
        );
        let shape = all
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let branch = text(&mut s, &all[shape], "branch").unwrap();
        let replacement = if branch == "TOP" { "BOTTOM" } else { "TOP" };
        let changed = replace_field(
            &mut s,
            &all[shape],
            SHAPE_FIELDS,
            "branch",
            V::String(replacement.into()),
        );
        let forged = replace_member(&mut s, &state, "objects", shape, changed);
        sc::verify(&mut s, &forged).unwrap();
        assert!(
            audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule)
                .unwrap_err()
                .contains("CC006 obj3")
        );
    }
    #[test]
    fn witness_merged_claim_is_checked_against_group_fact_not_itself() {
        let (mut s, state, raw, rule) = fixture();
        let root = field(&mut s, &state, "witnesses").unwrap();
        let first = elements(&mut s, &root).unwrap().into_iter().next().unwrap();
        // 原 witness ID 配方不含 merged_high；只改此值仍有合法 witness ID 和 Merkle 闭包。
        let changed = replace_field(
            &mut s,
            &first,
            WITNESS,
            "merged_high",
            V::String("999999".into()),
        );
        let forged = replace_member(&mut s, &state, "witnesses", 0, changed);
        sc::verify(&mut s, &forged).unwrap();
        let error = audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert!(error.contains("合并值不是同cut组事实"), "{error}");
    }
    #[test]
    fn witness_original_bytes_are_bound_even_if_reference_metadata_is_unchanged() {
        let (mut s, state, mut raw, rule) = fixture();
        raw[0]["high"] = json!("999999");
        let error = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert!(error.contains("raw_bars不是同cut真实原始成员"), "{error}");
    }
}

#[cfg(test)]
mod closed_array_role_probe {
    use super::super::{
        audit_store::StoreView,
        history::{with_overlay_prototype, HashReader, OverlayPrototype},
    };
    use super::*;
    use sc::capture_allocation::{CaptureSlab, OwnedBudget};
    #[cfg(feature = "s_allocation_json_testonly")]
    fn owned_coordinate_source() -> (sc::AuditCapture, OwnedBudget, ContentRef, ContentRef) {
        let mut nodes = sc::MemoryNodes::new();
        let valid = sc::write_value(
            &mut nodes,
            &V::Array(
                [0, 2, 3, 7, 11, 15, 17, 20, 21, 25]
                    .into_iter()
                    .map(|n| V::String(n.to_string()))
                    .collect(),
            ),
        )
        .unwrap()
        .content_ref();
        let invalid = sc::write_value(
            &mut nodes,
            &V::Array(vec![
                V::String("0".into()),
                V::String("2".into()),
                V::String("2".into()),
            ]),
        )
        .unwrap()
        .content_ref();
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, body)| body.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + 1_000_000);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (digest, tag, body) in nodes.iter() {
            slab.push(*digest, tag, body).unwrap();
        }
        (
            sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            valid,
            invalid,
        )
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn role_coordinates(
        reader: &mut HashReader<'_>,
        root: &ContentRef,
    ) -> Result<sc::CodecVec<i64>, String> {
        let mut view = StoreView::for_audit(reader);
        let mut source = ArrayAuditSource {
            source: &mut view,
            full_arrays: true,
            deadline: None,
            fields: None,
        };
        coords(&mut source, &RefValue::Plain(root.clone()))
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_source_collections_clone_union_difference_and_numeric_order_release() {
        let (capture, budget, valid, _) = owned_coordinate_source();
        let base = budget.snapshot().live_requested_layout_bytes;
        with_overlay_prototype(
            OverlayPrototype {
                slots: 4,
                bytes: 32,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let values = role_coordinates(&mut reader, &valid).unwrap();
                assert!(values.allocation_budget().is_some());
                assert_eq!(&*values, &[0, 2, 3, 7, 11, 15, 17, 20, 21, 25]);
                assert!(!reader.audit_full_array_scan());
                let live = budget.snapshot().live_requested_layout_bytes;
                let cloned = copy_coords(&reader, &values).unwrap();
                assert_eq!(cloned, values);
                assert!(cloned.allocation_budget().is_some());
                assert!(budget.snapshot().live_requested_layout_bytes > live);
                let union = union_coords(&reader, &values, &[1, 2, 10, 25, 26]).unwrap();
                assert_eq!(&*union, &[0, 1, 2, 3, 7, 10, 11, 15, 17, 20, 21, 25, 26]);
                let difference = difference_coords(&reader, &union, &values).unwrap();
                assert_eq!(&*difference, &[1, 10, 26]);
                let allocation = reader.allocation_budget();
                let mut groups = bj::new_map(allocation.as_ref());
                for n in [7, 1, 11, 0, 3] {
                    groups
                        .insert(n, copy_coords(&reader, &[n]).unwrap())
                        .unwrap();
                }
                let ordered = sorted_numeric_entries(&reader, &groups).unwrap();
                assert_eq!(
                    ordered.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
                    [0, 1, 3, 7, 11]
                );
                for (n, values) in &ordered {
                    assert_eq!(&***values, &[*n]);
                }
            },
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_source_collections_clone_and_map_growth_one_byte_short_keep_values() {
        let (capture, budget, valid, _) = owned_coordinate_source();
        let base = budget.snapshot().live_requested_layout_bytes;
        with_overlay_prototype(
            OverlayPrototype {
                slots: 4,
                bytes: 32,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let values = role_coordinates(&mut reader, &valid).unwrap();
                let request = values.len() * std::mem::size_of::<i64>();
                let live = budget.snapshot().live_requested_layout_bytes;
                let fill = CaptureSlab::new(
                    0,
                    budget.snapshot().limit_bytes - live - request + 1,
                    &budget,
                )
                .unwrap();
                let error = copy_coords(&reader, &values).unwrap_err();
                assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
                assert_eq!(&*values, &[0, 2, 3, 7, 11, 15, 17, 20, 21, 25]);
                drop(fill);
                assert_eq!(copy_coords(&reader, &values).unwrap(), values);
                let allocation = reader.allocation_budget();
                let mut cache = bj::new_map(allocation.as_ref());
                cache.insert(([0u8; 32], false), 0i64).unwrap();
                let count = cache.capacity();
                for n in 1..count {
                    cache.insert(([n as u8; 32], false), n as i64).unwrap();
                }
                let mut probe = bj::new_map(allocation.as_ref());
                for n in 0..count {
                    probe.insert(([n as u8; 32], false), n as i64).unwrap();
                }
                probe.insert(([255u8; 32], true), 999).unwrap();
                let request = budget.allocation_request_probe().last_request_bytes;
                drop(probe);
                let live = budget.snapshot().live_requested_layout_bytes;
                let fill = CaptureSlab::new(
                    0,
                    budget.snapshot().limit_bytes - live - request + 1,
                    &budget,
                )
                .unwrap();
                let error = cache.insert(([255u8; 32], true), 999).unwrap_err();
                assert_eq!(error.code(), sc::capture_allocation::LIMIT);
                assert_eq!(cache.len(), count);
                assert!(!cache.contains_key(&([255u8; 32], true)));
                for n in 0..count {
                    assert_eq!(cache.get(&([n as u8; 32], false)), Some(&(n as i64)));
                }
                drop(fill);
            },
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_source_collections_false_reader_role_failure_restores_and_drops_partial_coords() {
        let (capture, budget, valid, invalid) = owned_coordinate_source();
        let base = budget.snapshot().live_requested_layout_bytes;
        with_overlay_prototype(
            OverlayPrototype {
                slots: 4,
                bytes: 32,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                assert!(!reader.audit_full_array_scan());
                let error = role_coordinates(&mut reader, &invalid).unwrap_err();
                assert!(error.contains("源坐标不是严格升序"), "{error}");
                assert!(!reader.audit_full_array_scan());
                let values = role_coordinates(&mut reader, &valid).unwrap();
                assert_eq!(&*values, &[0, 2, 3, 7, 11, 15, 17, 20, 21, 25]);
                assert!(!reader.audit_full_array_scan());
            },
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn closed_owned_second_hashreader_false_preserves_array_wrapper_true_role_and_restore() {
        let mut nodes = sc::MemoryNodes::new();
        let built = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("a".into(), V::String("1".into())),
                ("b".into(), V::String("2".into())),
                ("c".into(), V::String("3".into())),
            ]),
        )
        .unwrap();
        let legacy = sc::AuditCapture::new(nodes.clone());
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + 65536);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let operation = |reader: &mut HashReader<'_>| {
            assert!(
                !reader.audit_full_array_scan(),
                "真实state第二reader初值必须false"
            );
            let f = sc::frame_calls_on_thread();
            let reads = match reader.source_capture_for_role_probe() {
                Some(c) => c.retention()["capture_reads"].as_u64().unwrap(),
                None => 0,
            };
            let mut typed = StoreView::for_audit(reader);
            let mut source = ArrayAuditSource {
                source: &mut typed,
                full_arrays: true,
                deadline: None,
                fields: None,
            };
            assert!(source.audit_full_array_scan());
            let got = source.lookup_object_fields_subset(&built.content_ref(), &["b"]);
            let fail = source.lookup_object_fields_subset(&built.content_ref(), &["missing"]);
            assert!(fail.is_err());
            drop(source);
            drop(typed);
            assert!(
                !reader.audit_full_array_scan(),
                "成功/Err均恢复实际inner角色"
            );
            let now = reader.source_capture_for_role_probe().unwrap().retention()["capture_reads"]
                .as_u64()
                .unwrap();
            (got, fail, sc::frame_calls_on_thread() - f, now - reads)
        };
        let expected = operation(&mut HashReader::captured(&legacy, None));
        let actual = with_overlay_prototype(
            OverlayPrototype {
                slots: 4,
                bytes: 32,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || operation(&mut HashReader::captured_for_audit(&owned, None).unwrap()),
        );
        assert_eq!(actual, expected);
        // #1394：成功返回的 Owned 字段仍持账，释放返回所有者后才只剩原件 slab。
        assert!(budget.snapshot().live_requested_layout_bytes > base);
        drop(actual.0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        println!(
            "CLOSED_ARRAY_ROLE_RECEIPT f={} reads={} meter={}",
            actual.2,
            actual.3,
            serde_json::to_string(&budget.snapshot()).unwrap()
        );
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

fn field_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_ref(r, &[f(k)]).map_err(err)
}
fn elements_ref(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<sc::CodecVec<RefValue>, String> {
    if r.kind() != Kind::Array {
        return Err(err("预期数组"));
    }
    if s.audit_full_array_scan() {
        return s.lookup_array_ref(r).map_err(err);
    }
    let b = s.allocation_budget();
    let mut out = bj::new_work(b.as_ref());
    for i in 0..r.extent().ok_or_else(|| err("数组缺长度"))? {
        let v = s.lookup_ref(r, &[PathStep::Index(i)]).map_err(err)?;
        out.try_push(v).map_err(err)?;
    }
    Ok(out)
}
