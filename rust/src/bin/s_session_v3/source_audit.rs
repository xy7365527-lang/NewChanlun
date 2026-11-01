//! #1394 C §5.2–5.4：绑定同 cut 来源原件、typed/CC006 身份和见证。
//! 只读事实及其引用，不重新运行包含、分型或新笔判定。原始账必须先经 read_raw_events 核验。
use super::audit_store::RefValue;
use super::audit_store::{CodecRead, CodecStore};
use super::business_json::BusinessMap;
use super::business_json::{self as bj, RawEventsView};
#[cfg(test)]
use super::read_effective_events;
use crate::{
    canonical_json,
    content_producer::{
        self, Cc004Producer, ShapeIdentityView, CC004_STEP_KIND, CC005_GROUP_KIND,
        CC006_SHAPE_KIND, CC007_DEVELOPMENT_BATCH_KIND, CC007_FRACTAL_KIND, CC054_KNOWLEDGE_KIND,
    },
    tb02a, tb02b,
};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    Built, ContentRef, Kind, NodeDigest, NodeSink, NodeSource, NodeStore, Value as V,
};
use sc::capture_allocation::native_owned::{NativeRefFields, NativeValue};
#[cfg_attr(
    not(test),
    allow(unused_imports, reason = "#1488 再合 #1608：仅测试使用")
)]
use serde_json::{json, Value};
#[cfg(test)]
#[path = "source_audit_oracle.rs"]
mod frozen_oracle;
#[cfg(feature = "s_incremental")]
#[path = "source_contributions.rs"]
mod source_contributions;
#[cfg(test)]
#[path = "source_delta.rs"]
mod source_delta;
#[cfg(all(test, feature = "s_incremental"))]
pub(crate) use source_contributions::SourceWork;
#[cfg(feature = "s_incremental")]
pub(crate) use source_contributions::{audit_native_bound, NativeSourceState};
#[path = "ordinary_source.rs"]
mod ordinary_source;
pub(super) use ordinary_source::OrdinaryProofs;
#[cfg(all(test, feature = "s_incremental"))]
pub(super) use ordinary_source::{
    reset_stats as reset_ordinary_proof_stats, stats as ordinary_proof_stats,
};
#[path = "source_pending.rs"]
mod source_pending;
#[path = "source_prefix.rs"]
mod source_prefix;
#[cfg(feature = "s_incremental")]
#[path = "source_receipt.rs"]
pub(super) mod source_receipt;
use source_prefix::{NativeSourceMemo, ParseDomain, SourceCoords};
#[cfg(test)]
pub(super) fn reset_native_source_prefix_stats() {
    source_prefix::reset_stats();
}
#[cfg(test)]
pub(super) fn native_source_prefix_stats() -> Option<[u64; 4]> {
    source_prefix::stats()
}
#[cfg(test)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceCallStats {
    pub(crate) full_source_calls: usize,
    pub(crate) full_pending_calls: usize,
    pub(crate) fixture_bind_calls: usize,
    pub(crate) full_members_enumerations: usize,
}
#[cfg(test)]
thread_local! { static SOURCE_CALL_STATS: std::cell::Cell<SourceCallStats> = const { std::cell::Cell::new(SourceCallStats {full_source_calls:0,full_pending_calls:0,fixture_bind_calls:0,full_members_enumerations:0}) }; }
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
pub(crate) fn reset_source_call_stats() {
    SOURCE_CALL_STATS.with(|v| v.set(SourceCallStats::default()));
}
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
pub(crate) fn source_call_stats() -> SourceCallStats {
    SOURCE_CALL_STATS.with(std::cell::Cell::get)
}
#[cfg(test)]
fn source_call_note(update: impl FnOnce(&mut SourceCallStats)) {
    SOURCE_CALL_STATS.with(|v| {
        let mut n = v.get();
        update(&mut n);
        v.set(n);
    });
}
fn err(e: impl std::fmt::Display) -> String {
    format!("SourceAuditInvalid：{e}")
}
#[cfg(test)]
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
)]
fn f(k: &str) -> PathStep {
    PathStep::Field(k.into())
}
// Only immutable Native originals may enter these maps; no cut/business verdict is retained.
#[derive(Debug, PartialEq, Eq, Hash)]
struct NativeFieldRefKey {
    format: NativeValue<String>,
    root: NativeValue<String>,
    kind: Kind,
    extent: Option<u64>,
    digest: NodeDigest,
}
impl NativeFieldRefKey {
    fn new(s: &(impl CodecRead + ?Sized), r: sc::RefView<'_>) -> Result<Self, sc::CodecError> {
        Ok(Self {
            format: s.native_string(r.format())?,
            root: s.native_string(r.root())?,
            kind: r.kind(),
            extent: r.extent(),
            digest: r.digest()?,
        })
    }
}
struct NativeScalarFields {
    values: BusinessMap<NativeFieldRefKey, bj::PlainJson>,
    decodes: u64,
    hits: u64,
}
impl Default for NativeScalarFields {
    fn default() -> Self {
        Self {
            values: bj::new_map(None),
            decodes: 0,
            hits: 0,
        }
    }
}
// The requested sequence is an identity component, including duplicates and order.
struct NativeFieldNames(sc::CodecVec<NativeValue<String>>);
impl PartialEq for NativeFieldNames {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}
impl Eq for NativeFieldNames {}
impl std::hash::Hash for NativeFieldNames {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0[..], state);
    }
}
struct NativeFieldMemo<'capture> {
    work: Option<&'capture sc::capture_allocation::scratch::WorkScope>,
    capture: &'capture sc::AuditCapture,
    records: Option<super::history::NativeRecordFields<'capture>>,
    fields: BusinessMap<(NativeFieldRefKey, NativeFieldNames), NativeRefFields>,
    scalars: NativeScalarFields,
    field_decodes: u64,
    field_hits: u64,
}
impl<'capture> NativeFieldMemo<'capture> {
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    fn new(capture: &'capture sc::AuditCapture) -> Self {
        Self {
            capture,
            work: None,
            records: None,
            fields: bj::new_map(None),
            scalars: Default::default(),
            field_decodes: 0,
            field_hits: 0,
        }
    }
    fn qualified(
        &self,
        r: sc::RefView<'_>,
        measure: Option<&super::audit_budget::NativeMeasure<'_>>,
    ) -> Result<bool, sc::CodecError> {
        // Root presence is insufficient; original public validation includes format/root/header
        // and the complete immutable closure. Overlay-only records never obtain a handle.
        if let sc::RefView::Normal(original) = r {
            if self.capture.contains(&original.digest()?) {
                super::audit_store::NativeAdmission(self.work).native_verify_original_measured(
                    self.capture,
                    original.into(),
                    sc::Mode::Public,
                    measure,
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn clear(&mut self) {
        if let Some(records) = &mut self.records {
            records.clear();
        }
        self.fields.retain(|_, _| false);
        self.scalars.values.retain(|_, _| false);
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
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：native 非测试、s_session 非测试、s_session 测试 构建不触达"
        )
    )]
    fn get(&self, k: &str) -> Result<sc::RefView<'_>, String> {
        self.0.get(k).ok_or_else(|| err(format!("原件缺字段{k}")))
    }
    fn get_admitted(&self, s: &(impl CodecRead + ?Sized), k: &str) -> Result<RefValue, String> {
        self.0
            .get_admitted(s, k)
            .map_err(err)?
            .ok_or_else(|| err(format!("原件缺字段{k}")))
    }
    fn scalar(
        &self,
        s: &mut (impl SourceFieldStore + ?Sized),
        k: &str,
    ) -> Result<bj::JsonOwner, String> {
        let r = self.get_admitted(s, k)?;
        s.decoded_scalar(r.view())
    }
    fn text(
        &self,
        s: &mut (impl SourceFieldStore + ?Sized),
        k: &str,
    ) -> Result<bj::TextValue, String> {
        let value = self.scalar(s, k)?;
        let view = value.view();
        let text = view.as_str().ok_or_else(|| err(format!("{k}不是文本")))?;
        s.native_text(text)
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
    s.native_text(text)
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
#[cfg(test)]
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
#[cfg_attr(
    not(all(test, feature = "s_incremental")),
    allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
)]
struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
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
    s.parse_rendered_plain(bytes).map(bj::JsonOwner::Plain)
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
            if s.codec_work().is_none() {
                return Ok(canonical_json(v).as_bytes() == bytes);
            }
            // serde_json::Map 采用排序 BTreeMap；逐段比较同一规范编码，不分配 parts/join。
            struct Compare<'a> {
                remaining: &'a [u8],
                same: bool,
            }
            impl std::io::Write for Compare<'_> {
                fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                    if self.remaining.starts_with(bytes) {
                        self.remaining = &self.remaining[bytes.len()..];
                    } else {
                        self.same = false;
                    }
                    Ok(bytes.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let mut out = Compare {
                remaining: bytes,
                same: true,
            };
            serde_json::to_writer(&mut out, v).map_err(err)?;
            Ok(out.same && out.remaining.is_empty())
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
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        self.source.checked_read_scoped(d)
    }

    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source.codec_work()
    }
    fn audit_full_array_scan(&self) -> bool {
        self.full_arrays && (self.source.native_measure().is_some() || self.deadline.is_none())
    }
    fn preserve_sequence_read_trace(&self) -> bool {
        !self.audit_full_array_scan()
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
    fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source.codec_work()
    }
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
            && (self.source.native_measure().is_some() || self.deadline.is_none())
            && self.source.allocation_budget().is_none()
            && self.max_render_bytes().is_none()
            && self
                .fields
                .as_ref()
                .map(|m| m.qualified(r, self.source.native_measure().as_ref()))
                .transpose()
                .map_err(err)?
                .unwrap_or(false);
        let key = if eligible {
            Some(NativeFieldRefKey::new(self.source, r).map_err(err)?)
        } else {
            None
        };
        if let Some(key) = &key {
            if let Some(memo) = self.fields.as_mut() {
                if let Some(value) = memo.scalars.values.get(key) {
                    let value = self.source.native_json_clone(value).map_err(err)?;
                    memo.scalars.hits += 1;
                    return Ok(bj::JsonOwner::Plain(value));
                }
            }
        }
        let value = scalar_value(self, r)?;
        if let Some(key) = key {
            if let (Some(memo), bj::JsonOwner::Plain(parsed)) = (self.fields.as_mut(), &value) {
                memo.scalars.decodes += 1;
                self.source
                    .native_map_room(&mut memo.scalars.values)
                    .map_err(err)?;
                memo.scalars
                    .values
                    .insert(key, self.source.native_json_clone(parsed).map_err(err)?)
                    .map_err(err)?;
            }
        }
        Ok(value)
    }
}
impl CodecRead for ArrayAuditSource<'_, '_, '_, '_> {
    fn native_measure(&self) -> Option<super::audit_budget::NativeMeasure<'_>> {
        self.source.native_measure()
    }
    fn parse_rendered_plain(&self, bytes: &[u8]) -> Result<bj::PlainJson, String> {
        self.source.parse_rendered_plain(bytes)
    }
    fn content_ref_view(&mut self, r: &str) -> Result<RefValue, sc::CodecError> {
        if self.source.owned_codec_active() {
            self.owned_role(|s| s.content_ref_view(r))
        } else {
            sc::path::content_ref_of_native(self, r).map(RefValue::Plain)
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
            sc::path::lookup_path_native(self, r, p).map(RefValue::Plain)
        }
    }
    fn lookup_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<RefValue, sc::CodecError> {
        if self.source.owned_codec_active() {
            // 同 lookup_ref 的实际 full-array 角色；不得继承先造 String 的默认实现。
            self.owned_role(|s| s.lookup_field_ref(r, key))
        } else {
            sc::path::lookup_path_view_native(self, r, &super::audit_store::BorrowedFieldPath(key))
                .map(RefValue::Plain)
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
            sc::path::lookup_fields_native(self, r, n, false)
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
                && (self.source.native_measure().is_some() || self.deadline.is_none())
                && self.source.allocation_budget().is_none()
                && self.max_render_bytes().is_none()
                && self
                    .fields
                    .as_ref()
                    .map(|m| m.qualified(r, self.source.native_measure().as_ref()))
                    .transpose()?
                    .unwrap_or(false);
            if eligible {
                if let Some(records) = self.fields.as_ref().and_then(|memo| memo.records.as_ref()) {
                    if let Some(fields) = records.subset(r, n)? {
                        return Ok(super::audit_store::RefFields::Plain(fields));
                    }
                }
            }
            let key = if eligible {
                let reference = NativeFieldRefKey::new(self, r)?;
                let mut names = bj::new_work(None);
                self.native_codec_room(&mut names, n.len())?;
                for name in n {
                    names.try_push(self.native_string(name)?)?;
                }
                Some((reference, NativeFieldNames(names)))
            } else {
                None
            };
            if let Some(key) = &key {
                if let Some(memo) = self.fields.as_mut() {
                    if let Some(fields) = memo.fields.get(key) {
                        let fields = NativeRefFields::try_from_views(
                            fields.iter().map(|(name, r)| (name, r.into())),
                            self.source.codec_work(),
                        )?;
                        memo.field_hits += 1;
                        return Ok(super::audit_store::RefFields::Plain(fields));
                    }
                }
            }
            let fields = sc::path::lookup_fields_native(self, r, n, true)?;
            if let Some(key) = key {
                // Copy canonical unique fields; the separate key retains the original name sequence.
                let originals = NativeRefFields::try_from_views(
                    fields.iter().map(|(name, r)| (name, r.into())),
                    self.source.codec_work(),
                )?;
                if let Some(memo) = self.fields.as_mut() {
                    memo.field_decodes += 1;
                    self.source.native_map_room(&mut memo.fields)?;
                    memo.fields.insert(key, originals)?;
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
            let plain = sc::path::lookup_array_items_native(self, r)?;
            let mut out = bj::new_work(None);
            self.native_codec_room(&mut out, plain.len())?;
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
        s.native_codec_room(&mut out, 1).map_err(err)?;
        out.try_push(n).map_err(err)?;
    }
    Ok(out)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Source {
    wire: NodeDigest,
    numeric: NodeDigest,
    raw: NodeDigest,
    bar: NodeDigest,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Counts {
    pub objects: usize,
    pub groups: usize,
    pub windows: usize,
    pub effective: usize,
    pub insufficient: usize,
    pub outside_domain: usize,
}
// 来源判据依赖精确查找而不依赖哈希布局。Native 的持久树不会在扩容时重排全历史。
trait RawLookup {
    fn get(&self, coord: &i64) -> Option<&Source>;
    fn contains_key(&self, coord: &i64) -> bool {
        self.get(coord).is_some()
    }
    fn full_source_map(&self) -> Option<&BusinessMap<i64, Source>> {
        None
    }
}
impl RawLookup for BusinessMap<i64, Source> {
    fn get(&self, coord: &i64) -> Option<&Source> {
        BusinessMap::get(self, coord)
    }
    fn full_source_map(&self) -> Option<&BusinessMap<i64, Source>> {
        Some(self)
    }
}
impl<T: RawLookup + ?Sized> RawLookup for &T {
    fn get(&self, coord: &i64) -> Option<&Source> {
        (**self).get(coord)
    }
    fn full_source_map(&self) -> Option<&BusinessMap<i64, Source>> {
        (**self).full_source_map()
    }
}
trait GroupLookup {
    fn get(&self, index: &i64) -> Option<&Group>;
    fn len(&self) -> usize;
}
impl GroupLookup for BusinessMap<i64, Group> {
    fn get(&self, index: &i64) -> Option<&Group> {
        BusinessMap::get(self, index)
    }
    fn len(&self) -> usize {
        BusinessMap::len(self)
    }
}
trait ShapeLookup {
    fn get(&self, id: &str) -> Option<&Shape>;
}
impl ShapeLookup for BusinessMap<bj::TextValue, Shape> {
    fn get(&self, id: &str) -> Option<&Shape> {
        BusinessMap::get(self, id)
    }
}
fn references(
    s: &mut (impl CodecRead + ?Sized),
    r: &RefValue,
    raw: &(impl RawLookup + ?Sized),
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
            s.native_map_room(checked).map_err(err)?;
            checked.insert(key, coord).map_err(err)?;
            coord
        };
        if out.last().is_some_and(|prev| *prev >= coord) {
            return Err(err("源引用不是严格升序"));
        }
        s.native_codec_room(&mut out, 1).map_err(err)?;
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
    raw: &(impl RawLookup + ?Sized),
    numeric: bool,
    checked: &mut BusinessMap<(NodeDigest, bool), i64>,
    memo: Option<&mut NativeSourceMemo<'_>>,
    prefix: usize,
    domain: ParseDomain,
) -> Result<SourceCoords, String> {
    match memo {
        Some(memo) => memo.references(
            s,
            r,
            raw.full_source_map()
                .ok_or_else(|| err("完整来源memo不接增量树账"))?,
            numeric,
            checked,
            prefix,
            domain,
        ),
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
    s.native_codec_room(&mut out, values.len()).map_err(err)?;
    out.try_reserve_exact(values.len()).map_err(err)?;
    for &value in values {
        s.native_codec_room(&mut out, 1).map_err(err)?;
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
        s.native_codec_room(&mut out, 1).map_err(err)?;
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
            s.native_codec_room(&mut out, 1).map_err(err)?;
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
        s.native_codec_room(&mut out, 1).map_err(err)?;
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
    object_id: NativeValue<String>,
    slot: usize,
    mi: usize,
    digest: NodeDigest,
    kind: Kind,
    extent: Option<u64>,
}
struct NativeWitnessIdentityMemo<'capture> {
    capture: &'capture sc::AuditCapture,
    entries: BusinessMap<NativeWitnessIdentityKey, NativeValue<String>>,
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
    #[cfg(feature = "s_incremental")]
    fn new(capture: &'capture sc::AuditCapture) -> Result<Self, String> {
        if capture.is_allocation_prototype() {
            return Err(err("Native身份memo不接收Owned capture"));
        }
        #[cfg(test)]
        NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(Some(NativeWitnessIdentityStats::default())));
        Ok(Self {
            capture,
            entries: bj::new_map(None),
            #[cfg(test)]
            stats: Default::default(),
        })
    }
    fn key(
        &self,
        s: &(impl CodecRead + ?Sized),
        object_id: &str,
        slot: usize,
        mi: usize,
        bars: &Built,
    ) -> Result<NativeWitnessIdentityKey, sc::CodecError> {
        Ok(NativeWitnessIdentityKey {
            capture: std::ptr::from_ref(self.capture) as usize,
            object_id: s.native_string(object_id)?,
            slot,
            mi,
            digest: bars.digest,
            kind: bars.kind,
            extent: bars.extent,
        })
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
            let want = super::generation::witness_identity_native(s, object_id, slot, mi, bars)?;
            return if stored == want.as_str() {
                Ok(())
            } else {
                Err(err("见证ID不符"))
            };
        }
        // 根在 Capture 不等于完整闭包在 Capture；用不可变原件验证全部声明与闭包。
        let bars_ref = s.native_built_ref(bars).map_err(err)?;
        s.native_verify_original(self.capture, bars_ref.view(), sc::Mode::Public)
            .map_err(err)?;
        let key = self.key(s, object_id, slot, mi, bars).map_err(err)?;
        if let Some(want) = self.entries.get(&key) {
            #[cfg(test)]
            {
                self.stats.hits += 1;
                NATIVE_WITNESS_IDENTITY_STATS.with(|s| s.set(Some(self.stats)));
            }
            return if stored == want.as_str() {
                Ok(())
            } else {
                Err(err("见证ID不符"))
            };
        }
        #[cfg(test)]
        self.record_render_attempt(object_id);
        let want = super::generation::witness_identity_native(s, object_id, slot, mi, bars)?;
        if stored != want.as_str() {
            return Err(err("见证ID不符"));
        }
        // 首次纯函数成功和本次 stored ID 比较都通过之后，才保留输入声明及结果文本。
        s.native_map_room(&mut self.entries).map_err(err)?;
        self.entries.insert(key, want).map_err(err)?;
        Ok(())
    }
}
// #1394：账本只借同一完整 immutable raw；请求只能指定 prefix，不能替换原件。
// 先选本 cut 最终身份赢家，再核全部变化坐标，最后才按 coord hash；跳跃中的
// 暂态赢家、暂态坐标碰撞和未来事件均不取得 Source 构造资格。
#[cfg(test)]
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct NativeRawLedgerStats {
    pub raw_visits: u64,
    pub coord_decodes: u64,
    pub source_hashes: u64,
    pub map_updates: u64,
    pub rewinds: u64,
}
/// 单代来源核完成后的纯所有权账；保留已访问 seq 的四类摘要、身份赢家和有序坐标。
/// 不包含 capture/reader/RawEventsView，不能单独授予新事务的来源资格。
pub(super) struct NativeRawFacts {
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    winners: bj::RawPrefixFacts,
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    sources: BusinessMap<i64, Source>,
    originals: BusinessMap<usize, (i64, Source)>,
}
impl NativeRawFacts {
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
    )]
    pub(super) fn prefix_len(&self) -> usize {
        self.winners.prefix_len()
    }
    #[cfg_attr(
        not(all(test, feature = "s_incremental")),
        allow(dead_code, reason = "#1608：仅 s_incremental 测试使用")
    )]
    pub(super) fn hashed_rows(&self) -> usize {
        self.originals.len()
    }
}
struct NativeRawLedger<'raw> {
    raw: RawEventsView<'raw>,
    winners: bj::RawPrefixIndex<'raw>,
    sources: BusinessMap<i64, Source>,
    originals: BusinessMap<usize, (i64, Source)>,
    #[cfg(feature = "s_incremental")]
    retained: super::audit_retention::RetainedDelta,
    #[cfg(test)]
    stats: NativeRawLedgerStats,
}
impl<'raw> NativeRawLedger<'raw> {
    fn into_facts(self) -> NativeRawFacts {
        NativeRawFacts {
            winners: self.winners.into_facts(),
            sources: self.sources,
            originals: self.originals,
        }
    }

    #[cfg(feature = "s_incremental")]
    fn into_facts_retained(
        self,
        delta: &mut super::audit_retention::RetainedDelta,
    ) -> NativeRawFacts {
        delta.merge(self.retained);
        NativeRawFacts {
            // 这是独立归属的第二份赢家账，不按共享 raw 减费。
            winners: self.winners.into_facts_retained(delta),
            sources: self.sources,
            originals: self.originals,
        }
    }

    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    fn new(raw: RawEventsView<'raw>) -> Self {
        Self {
            raw,
            winners: bj::RawPrefixIndex::new(raw),
            sources: bj::new_map(None),
            originals: bj::new_map(None),
            #[cfg(feature = "s_incremental")]
            retained: Default::default(),
            #[cfg(test)]
            stats: Default::default(),
        }
    }
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：仅 s_incremental 下使用")
    )]
    fn restore(
        raw: RawEventsView<'raw>,
        facts: NativeRawFacts,
        bound: &super::audit_owner::BoundOriginals,
    ) -> Result<Self, String> {
        Ok(Self {
            raw,
            winners: bj::RawPrefixIndex::restore(raw, facts.winners, bound)?,
            sources: facts.sources,
            originals: facts.originals,
            #[cfg(feature = "s_incremental")]
            retained: Default::default(),
            #[cfg(test)]
            stats: Default::default(),
        })
    }
    fn reset_progress(&mut self) {
        self.winners.clear();
        self.sources = bj::new_map(None);
    }
    fn clear(&mut self) {
        self.reset_progress();
        self.originals.retain(|_, _| false);
    }
    #[cfg(test)]
    fn advance(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        prefix: usize,
    ) -> Result<&BusinessMap<i64, Source>, String> {
        self.advance_measured(s, prefix, None, None)
    }
    fn advance_measured(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        prefix: usize,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<&BusinessMap<i64, Source>, String> {
        if let Err(error) = self.advance_inner(s, prefix, deadline, budget) {
            self.clear();
            return Err(error);
        }
        Ok(&self.sources)
    }
    fn advance_inner(
        &mut self,
        s: &mut (impl CodecRead + ?Sized),
        prefix: usize,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<(), String> {
        let changes = self.winners.advance_measured(prefix, s, deadline, budget)?;
        if changes.rewound {
            #[cfg(feature = "s_incremental")]
            self.retained
                .hash_capacity::<i64, Source>(self.sources.capacity(), 0);
            self.sources = bj::new_map(None);
        }
        // 修订先 remove 后重插同一坐标；跨完整变更量容量，不能把中间墓碑当释放。
        #[cfg(feature = "s_incremental")]
        let source_capacity = self.sources.capacity();
        #[cfg(test)]
        {
            self.stats.raw_visits += changes.raw_visits as u64;
            self.stats.coord_decodes += changes.coord_decodes as u64;
            self.stats.rewinds += u64::from(changes.rewound);
        }
        for coord in changes.removed {
            self.sources.remove(&coord);
        }
        for (coord, ordinal) in changes.added {
            let source = if let Some((_, source)) = self.originals.get(&ordinal) {
                *source
            } else {
                super::history::check_deadline(deadline)?;
                let event = self.raw.get(ordinal).expect("winner original");
                if let Some(budget) = budget {
                    budget.record().map_err(err)?;
                }
                let wire = bj::RawReference::new(event, false)?;
                let numeric = bj::RawReference::new(event, true)?;
                // hash_json_in_store 用 Discard 丢弃节点，不写 reader overlay。
                // 后续只比较摘要（含 r3 的五字段 bar 投影），从不以它们读取节点。
                let source = Source {
                    wire: super::history::hash_json_in_store(s, wire.view(), deadline, budget)?,
                    numeric: super::history::hash_json_in_store(
                        s,
                        numeric.view(),
                        deadline,
                        budget,
                    )?,
                    raw: super::history::hash_json_in_store(s, event, deadline, budget)?,
                    bar: source_pending::raw_bar(s, event, deadline, budget)?,
                };
                #[cfg(test)]
                {
                    self.stats.source_hashes += 4;
                }
                #[cfg(test)]
                {
                    super::state::trace_raw_hashes(true, 4);
                    super::state::trace_event("historical_raw_source_hashes", None);
                }
                #[cfg(feature = "s_incremental")]
                let capacity = self.originals.capacity();
                s.native_map_room(&mut self.originals).map_err(err)?;
                self.originals
                    .insert(ordinal, (coord, source))
                    .map_err(err)?;
                #[cfg(feature = "s_incremental")]
                self.retained
                    .hash_capacity::<usize, (i64, Source)>(capacity, self.originals.capacity());
                source
            };
            if !self.sources.contains_key(&coord) {
                s.native_map_room(&mut self.sources).map_err(err)?;
            }
            self.sources.insert(coord, source).map_err(err)?;
            #[cfg(test)]
            {
                self.stats.map_updates += 1;
            }
        }
        #[cfg(feature = "s_incremental")]
        self.retained
            .hash_capacity::<i64, Source>(source_capacity, self.sources.capacity());
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
    #[cfg(feature = "s_incremental")]
    pub(super) fn with_codec_work(
        mut self,
        scope: &'capture sc::capture_allocation::scratch::WorkScope,
        native: &'capture super::audit_owner::native::Budget,
    ) -> Self {
        self.reader = self.reader.with_codec_work(scope, native);
        self.fields.work = Some(scope);
        self
    }

    pub(super) fn into_raw_facts(self) -> NativeRawFacts {
        self.ledger.into_facts()
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn into_raw_facts_retained(
        self,
        delta: &mut super::audit_retention::RetainedDelta,
    ) -> NativeRawFacts {
        self.ledger.into_facts_retained(delta)
    }

    #[cfg(feature = "s_incremental")]
    pub(super) fn new(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
    ) -> Result<Self, String> {
        Self::new_with_records(capture, raw, None)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn new_with_records(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
        records: Option<super::history::NativeRecordFields<'capture>>,
    ) -> Result<Self, String> {
        Self::new_with_records_admitted(capture, raw, records, None)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn new_with_records_admitted(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
        records: Option<super::history::NativeRecordFields<'capture>>,
        work: Option<&'capture sc::capture_allocation::scratch::WorkScope>,
    ) -> Result<Self, String> {
        Self::new_with_records_measured(
            capture,
            raw,
            records,
            work,
            None,
            None,
            super::history::SourcePolicy::Ordinary,
        )
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn new_with_records_measured(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
        records: Option<super::history::NativeRecordFields<'capture>>,
        work: Option<&'capture sc::capture_allocation::scratch::WorkScope>,
        budget: Option<&'capture dyn super::audit_budget::Budget>,
        deadline: Option<std::time::Instant>,
        policy: super::history::SourcePolicy,
    ) -> Result<Self, String> {
        if raw.budget().is_some() {
            return Err(err("Native来源memo不接收Owned原始账"));
        }
        if records
            .as_ref()
            .is_some_and(|records| !records.belongs_to(capture))
        {
            return Err(err("Native字段交接不属于本次capture"));
        }
        let mut fields = NativeFieldMemo::new(capture);
        fields.records = records;
        fields.work = work;
        Ok(Self {
            source_memo: NativeSourceMemo::new(capture, raw),
            ledger: NativeRawLedger::new(raw),
            fields,
            raw,
            reader: super::history::HashReader::captured(capture, None)
                .with_source_policy(policy)
                .with_audit_gate(budget, deadline)
                .with_native_scope(work)
                .map_err(err)?,
            memo: NativeWitnessIdentityMemo::new(capture)?,
        })
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn restore(
        capture: &'capture sc::AuditCapture,
        raw: RawEventsView<'capture>,
        facts: NativeRawFacts,
        bound: &super::audit_owner::BoundOriginals,
    ) -> Result<Self, String> {
        let mut result = Self::new(capture, raw)?;
        result.ledger = NativeRawLedger::restore(raw, facts, bound)?;
        Ok(result)
    }
    #[cfg(test)]
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1608：测试辅助，仅 s_incremental 测试使用")
    )]
    pub(super) fn raw_ledger_stats(&self) -> NativeRawLedgerStats {
        self.ledger.stats
    }
    /// #1481：复用来源counts不等于已推进独立raw事实账。
    pub(super) fn advance_raw_facts(&mut self, prefix_len: usize) -> Result<(), String> {
        let measure = self.reader.native_measure();
        self.ledger.advance_measured(
            &mut super::audit_store::StoreView::for_audit(&mut self.reader),
            prefix_len,
            measure.as_ref().and_then(|m| m.deadline),
            measure.as_ref().and_then(|m| m.budget),
        )?;
        Ok(())
    }
    pub(super) fn audit<'a>(
        &mut self,
        state: impl Into<sc::RefView<'a>>,
        prefix_len: usize,
        profile: &str,
        rule: &str,
    ) -> Result<Counts, String> {
        let measure = self.reader.native_measure();
        let result = (|| {
            audit_generated_in_store(
                &mut super::audit_store::StoreView::for_audit(&mut self.reader),
                state.into(),
                self.raw.prefix(prefix_len)?,
                profile,
                rule,
                measure.as_ref().and_then(|m| m.deadline),
                measure.as_ref().and_then(|m| m.budget),
                Some(&mut self.memo),
                Some(&mut self.source_memo),
                Some(&mut self.fields),
                Some(&mut self.ledger),
                None,
                #[cfg(feature = "s_incremental")]
                None,
            )
        })();
        if result.is_err() {
            self.fields.clear();
            self.ledger.clear();
            self.source_memo.clear();
            self.memo.entries.retain(|_, _| false);
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
#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
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
        None,
        #[cfg(feature = "s_incremental")]
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
    #[cfg(test)]
    if !super::history::ordinary_proofs_enabled() {
        let state = state.into();
        // 冻结核只调用通用 verify_ref；对照臂仍须消费已核历史的 source Public 根资格。
        s.verify_history_public_root(state, sc::Mode::Public)
            .map_err(err)?;
        return audit_generated_frozen_for_test(
            s, state, raw_prefix, profile, rule, deadline, budget,
        );
    }
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
        None,
        #[cfg(feature = "s_incremental")]
        None,
    )
}

pub(super) fn audit_generated_with_proofs(
    s: &mut super::history::HashReader<'_>,
    state: sc::RefView<'_>,
    raw: RawEventsView<'_>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    proofs: &mut OrdinaryProofs,
) -> Result<Counts, String> {
    proofs.begin(profile, rule);
    audit_generated_in_store(
        &mut super::audit_store::StoreView::for_audit(s),
        state,
        raw,
        profile,
        rule,
        deadline,
        budget,
        None,
        None,
        None,
        None,
        Some(proofs),
        #[cfg(feature = "s_incremental")]
        None,
    )
}

// #1488 SFT-M1：故障对拍必须调用冻结旧核，不能借用重构后的同一实现。
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
pub(super) fn audit_generated_frozen_for_test<'a>(
    s: &mut super::history::HashReader<'_>,
    state: impl Into<sc::RefView<'a>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    frozen_oracle::audit_generated_in_store(
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

/// 单对象同判据核：身份唯一性仍在载荷读取之前；调用方提供当前 cut 的全局索引。
// 同一局部判据只把“当前位置之前是否出现”交给容器；不改变读取/拒绝次序。
trait ObjectSlots {
    fn reserve_group(&mut self, _s: &(impl CodecRead + ?Sized)) -> Result<(), String> {
        Ok(())
    }
    fn insert_id(&mut self, s: &(impl CodecRead + ?Sized), id: &str) -> Result<bool, String>;
    /// `id` 是该 CC005 对象的 object_id；36M cover 以它而非 group_index 作组身份。
    fn insert_group(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        index: i64,
        group: Group,
    ) -> Result<bool, String>;
}
struct FullObjectSlots<'a> {
    ids: &'a mut BusinessMap<bj::TextValue, ()>,
    groups: &'a mut BusinessMap<i64, Group>,
    owners: Option<&'a mut BusinessMap<i64, bj::TextValue>>,
}
impl ObjectSlots for FullObjectSlots<'_> {
    fn reserve_group(&mut self, s: &(impl CodecRead + ?Sized)) -> Result<(), String> {
        s.native_map_room(self.groups).map_err(err)
    }
    fn insert_id(&mut self, s: &(impl CodecRead + ?Sized), id: &str) -> Result<bool, String> {
        s.native_map_room(self.ids).map_err(err)?;
        Ok(self
            .ids
            .insert(s.native_text(id)?, ())
            .map_err(err)?
            .is_some())
    }
    fn insert_group(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        index: i64,
        group: Group,
    ) -> Result<bool, String> {
        if let Some(owners) = self.owners.as_deref_mut() {
            s.native_map_room(owners).map_err(err)?;
            owners.insert(index, s.native_text(id)?).map_err(err)?;
        }
        Ok(self.groups.insert(index, group).map_err(err)?.is_some())
    }
}
/// 生产路径自 36M 起直接调用 `audit_object_record_with_slots`，本包装只剩测试调用者。
#[cfg(test)]
fn audit_object_record(
    s: &mut (impl SourceFieldStore + ?Sized),
    record: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    producer: &Cc004Producer,
    ids: &mut BusinessMap<bj::TextValue, ()>,
    groups: &mut BusinessMap<i64, Group>,
    checked_refs: &mut BusinessMap<(NodeDigest, bool), i64>,
    native_sources: Option<&mut NativeSourceMemo<'_>>,
    raw_prefix_len: usize,
    memo_fields: bool,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Object, String> {
    audit_object_record_with_slots(
        s,
        record,
        raw,
        producer,
        &mut FullObjectSlots {
            ids,
            groups,
            owners: None,
        },
        checked_refs,
        native_sources,
        raw_prefix_len,
        memo_fields,
        deadline,
        budget,
    )
}

fn audit_object_record_with_slots(
    s: &mut (impl SourceFieldStore + ?Sized),
    record: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    producer: &Cc004Producer,
    slots: &mut impl ObjectSlots,
    checked_refs: &mut BusinessMap<(NodeDigest, bool), i64>,
    mut native_sources: Option<&mut NativeSourceMemo<'_>>,
    raw_prefix_len: usize,
    memo_fields: bool,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Object, String> {
    let kind = if memo_fields {
        SourceFields::read_subset(s, record.view(), &["kind"])?.text(s, "kind")?
    } else {
        text(s, record, "kind")?
    };
    let names: &[&'static str] = if kind == CC006_SHAPE_KIND {
        &SHAPE_SOURCE_FIELDS
    } else {
        &TYPED_SOURCE_FIELDS
    };
    let fields = SourceFields::read_subset(s, record.view(), names).map_err(err)?;
    let id = fields.text(s, "object_id").map_err(err)?;
    if slots.insert_id(s, id.as_str())? || fields.integer(s, "object_revision").map_err(err)? != 1 {
        return Err(err("对象身份重复或修订不符"));
    }
    let refs = s
        .lookup_ref(
            fields
                .get_admitted(s, "input_refs_json")
                .map_err(err)?
                .view(),
            &[PathStep::JsonText],
        )
        .map_err(err)?;
    let source_ref = s
        .lookup_ref(
            fields
                .get_admitted(s, "source_coords_json")
                .map_err(err)?
                .view(),
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
        exact(record, Kind::Object, 15)?;
    } else {
        exact(record, Kind::Object, 10)?;
        if ![
            CC004_STEP_KIND,
            CC005_GROUP_KIND,
            CC007_FRACTAL_KIND,
            CC007_DEVELOPMENT_BATCH_KIND,
            CC054_KNOWLEDGE_KIND,
        ]
        .contains(&kind.as_str())
            && !tb02b::KINDS.contains(&kind.as_str())
        {
            return Err(err("未声明typed事实种类"));
        }
        let payload = s
            .lookup_ref(
                fields.get_admitted(s, "payload_json").map_err(err)?.view(),
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
            checked_refs,
            native_sources.as_deref_mut(),
            raw_prefix_len,
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
        let identity = producer
            .typed_identity_scoped(
                &kind,
                &key,
                &built(payload.view())?,
                &built(refs.view())?,
                s.codec_work(),
            )
            .map_err(err)?;
        let expected = s
            .native_format(format_args!(
                "{}",
                content_producer::object_id_display(&identity)
            ))
            .map_err(err)?;
        if id.as_str() != expected.as_str() {
            return Err(err("typed obj3身份与来源/载荷原件不符"));
        }
        if tb02b::KINDS.contains(&kind.as_str()) {
            let contract = tb02b::contract_for_rule(producer.rule_revision())
                .ok_or_else(|| err("新笔来源审计规则修订未声明"))?;
            for (k, want) in [
                ("schema_revision", contract.schema),
                ("semantic_version", contract.semantic),
                ("policy_version", contract.policy),
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
        if kind == CC054_KNOWLEDGE_KIND {
            let contract = tb02b::contract_for_rule(producer.rule_revision())
                .ok_or_else(|| err("知识域来源审计规则修订未声明"))?;
            if text(s, &payload, "domain")? != contract.knowledge_domain {
                return Err(err("TB-02-A 知识域与cut规则版本不符"));
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
            slots.reserve_group(s)?;
            if slots.insert_group(
                s,
                id.as_str(),
                index,
                Group {
                    payload: s.native_ref_clone(&payload).map_err(err)?,
                    members,
                    sources: copy_source_coords(s, &sources)?,
                    anchor,
                },
            )? {
                return Err(err("同cut组合并序号重复"));
            }
        }
    }
    Ok(Object {
        id,
        kind,
        fields,
        refs,
        sources,
    })
}

/// 单形态核，依赖三组当前成员/依据/锚及原始来源。
fn audit_shape_record(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
    raw: &(impl RawLookup + ?Sized),
    groups: &(impl GroupLookup + ?Sized),
    producer: &Cc004Producer,
    checked_refs: &mut BusinessMap<(NodeDigest, bool), i64>,
    mut native_sources: Option<&mut NativeSourceMemo<'_>>,
    raw_prefix_len: usize,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Shape, String> {
    let allocation = source_allocation(s)?;
    exact(&object.refs, Kind::Array, 3)?;
    let mut indices = [0; 3];
    let mut highs: [bj::TextValue; 3] = std::array::from_fn(|_| bj::TextValue::from(String::new()));
    let mut lows: [bj::TextValue; 3] = std::array::from_fn(|_| bj::TextValue::from(String::new()));
    let mut windows = std::array::from_fn::<_, 3, _>(|_| crate::content_manifest::Decimal::new(0));
    let mut source_union = bj::new_work(allocation.as_ref());
    for (slot, part) in elements(s, &object.refs)?.into_iter().enumerate() {
        super::history::check_deadline(deadline)?;
        exact(&part, Kind::Object, 3)?;
        let part_fields = local_fields(s, &part, &["merged_index", "raw_refs", "dependency_refs"])?;
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
            fields.get_admitted(s, "raw_refs").map_err(err)?
        } else {
            field(s, &part, "raw_refs")?
        };
        let members = source_references(
            s,
            &raw_refs,
            &raw,
            true,
            checked_refs,
            native_sources.as_deref_mut(),
            raw_prefix_len,
            ParseDomain::JsonInner,
        )?;
        if members != group.members {
            return Err(err("形态raw_refs不是同cut组成员"));
        }
        let deps = if let Some(fields) = &part_fields {
            fields.get_admitted(s, "dependency_refs").map_err(err)?
        } else {
            field(s, &part, "dependency_refs")?
        };
        let dependency = source_references(
            s,
            &deps,
            &raw,
            true,
            checked_refs,
            native_sources.as_deref_mut(),
            raw_prefix_len,
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
        if windows[slot].as_str() != crate::content_manifest::Decimal::new(group.anchor).as_str() {
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
        .shape_identity_scoped(
            ShapeIdentityView {
                branch: &branch,
                dir_ab: &dir_ab,
                dir_bc: &dir_bc,
                window: windows.each_ref().map(|v| v.as_str()),
                highs: highs.each_ref().map(|v| v.as_str()),
                lows: lows.each_ref().map(|v| v.as_str()),
                comparisons: &comparisons,
                input_refs: &input_refs,
            },
            s.codec_work(),
        )
        .map_err(err)?;
    let expected = s
        .native_format(format_args!(
            "{}",
            content_producer::object_id_display(&identity)
        ))
        .map_err(err)?;
    if expected.as_str() != object.id.as_str() {
        return Err(err("CC006 obj3与同cut组事实身份不符"));
    }
    Ok(Shape { groups: indices })
}

/// 单见证核，槽位重复在形态/组原件读取之前拒绝；三槽全体性仍由贡献层核验。
trait WitnessSlots {
    fn reserve(&mut self, s: &(impl CodecRead + ?Sized)) -> Result<(), String>;
    fn insert_slot(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        slot: i64,
    ) -> Result<bool, String>;
}
struct FullWitnessSlots<'a>(&'a mut BusinessMap<(bj::TextValue, i64), ()>);
impl WitnessSlots for FullWitnessSlots<'_> {
    fn reserve(&mut self, s: &(impl CodecRead + ?Sized)) -> Result<(), String> {
        s.native_map_room(self.0).map_err(err)
    }
    fn insert_slot(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        slot: i64,
    ) -> Result<bool, String> {
        Ok(self
            .0
            .insert((s.native_text(id)?, slot), ())
            .map_err(err)?
            .is_some())
    }
}
fn audit_witness_record(
    s: &mut (impl SourceFieldStore + ?Sized),
    witness: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    groups: &(impl GroupLookup + ?Sized),
    shapes: &(impl ShapeLookup + ?Sized),
    seen: &mut BusinessMap<(bj::TextValue, i64), ()>,
    native_identity: Option<&mut NativeWitnessIdentityMemo<'_>>,
    deadline: Option<std::time::Instant>,
) -> Result<(bj::TextValue, i64), String> {
    audit_witness_record_with_slots(
        s,
        witness,
        raw,
        groups,
        shapes,
        &mut FullWitnessSlots(seen),
        native_identity,
        deadline,
    )
}

fn audit_witness_record_with_slots(
    s: &mut (impl SourceFieldStore + ?Sized),
    witness: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    groups: &(impl GroupLookup + ?Sized),
    shapes: &(impl ShapeLookup + ?Sized),
    slots: &mut impl WitnessSlots,
    mut native_identity: Option<&mut NativeWitnessIdentityMemo<'_>>,
    deadline: Option<std::time::Instant>,
) -> Result<(bj::TextValue, i64), String> {
    exact(witness, Kind::Object, 9)?;
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
    slots.reserve(s)?;
    if !(0..3).contains(&slot) || slots.insert_slot(s, object_id.as_str(), slot)? {
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
        let actual = fields.get_admitted(s, merged).map_err(err)?;
        let expected = if let Some(fields) = &payload_fields {
            fields.get_admitted(s, field_name).map_err(err)?
        } else {
            field(s, &group.payload, field_name)?
        };
        if actual != expected {
            return Err(err("见证合并值不是同cut组事实"));
        }
    }
    let bars = fields.get_admitted(s, "raw_bars").map_err(err)?;
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
        Some(memo) => memo.check_identity(s, &id, &object_id, slot as usize, mi as usize, &bars)?,
        None => {
            let want = super::generation::witness_identity_native(
                s,
                &object_id,
                slot as usize,
                mi as usize,
                &bars,
            )?;
            if id.as_str() != want.as_str() {
                return Err(err("见证ID不符"));
            }
        }
    }
    Ok((object_id, slot))
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
    mut ordinary: Option<&mut OrdinaryProofs>,
    #[cfg(feature = "s_incremental")] mut exports: Option<
        &mut source_receipt::source_facts::SourceExports,
    >,
) -> Result<Counts, String> {
    #[cfg(test)]
    source_call_note(|n| {
        n.full_source_calls += 1;
    });
    super::history::check_deadline(deadline)?;
    #[cfg(test)]
    super::state::trace_event("source_closure", None);
    s.verify_history_public_root(state, sc::Mode::Public)
        .map_err(err)?;
    let full_arrays = s.native_measure().is_some() || budget.is_none();
    let mut array_source = ArrayAuditSource {
        source: s,
        full_arrays,
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
    let producer = Cc004Producer::for_source_audit(profile, rule, s.codec_work()).map_err(err)?;
    let allocation;
    let ordinary_effective;
    let mut ordinary_raw;
    // 36M cover 只能从完整原件的赢家 seq 导出；Native 账本路径不签发导出。
    #[cfg(feature = "s_incremental")]
    let mut winner_seqs = None;
    let raw = if let Some(ledger) = native_ledger {
        #[cfg(feature = "s_incremental")]
        if exports.is_some() {
            return Err(err("cover 导出需要完整原件赢家 seq，不接 Native 账本"));
        }
        allocation = source_allocation(s)?;
        ledger.advance_measured(s, raw_prefix.len(), deadline, budget)?
    } else {
        ordinary_effective = bj::effective(raw_prefix, deadline)?;
        allocation = source_allocation(s)?;
        ordinary_raw = bj::new_map(allocation.as_ref());
        #[cfg(feature = "s_incremental")]
        let mut seqs = bj::new_map(allocation.as_ref());
        for event in ordinary_effective.iter() {
            super::history::check_deadline(deadline)?;
            let coord = bj::coord(event)?;
            #[cfg(feature = "s_incremental")]
            if exports.is_some() {
                let seq = bj::field(event, "seq")
                    .as_i64()
                    .ok_or_else(|| err("生效原件seq非i64"))?;
                s.native_map_room(&mut seqs).map_err(err)?;
                seqs.insert(coord, seq).map_err(err)?;
            }
            let wire = bj::RawReference::new(event, false)?;
            let numeric = bj::RawReference::new(event, true)?;
            s.native_map_room(&mut ordinary_raw).map_err(err)?;
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
                        bar: source_pending::raw_bar(s, event, deadline, budget)?,
                    },
                )
                .map_err(err)?;
        }
        #[cfg(feature = "s_incremental")]
        {
            winner_seqs = Some(seqs);
        }
        &ordinary_raw
    };
    let object_array = field_ref(s, state, "objects")?;
    #[cfg(test)]
    super::state::trace_event("source_objects", None);
    // 活期只覆盖当前一次audit_generated；不在代际、事务或源修订之间带出。
    let mut checked_refs = bj::new_map(allocation.as_ref());
    let mut objects = bj::new_work(allocation.as_ref());
    let mut ids = bj::new_map(allocation.as_ref());
    let mut groups = bj::new_map(allocation.as_ref());
    let mut owners = bj::new_map(allocation.as_ref());
    #[cfg(test)]
    source_call_note(|n| n.full_members_enumerations += 1);
    if object_array.kind() != Kind::Array {
        return Err(err("预期数组"));
    }
    let mut cursor = if s.audit_full_array_scan() {
        s.array_cursor(object_array.view()).map_err(err)?
    } else {
        None
    };
    let owned_items = if cursor.is_none() {
        Some(elements_ref(s, object_array.view())?)
    } else {
        None
    };
    for i in 0..object_array.extent().ok_or_else(|| err("数组缺长度"))? {
        let record = if let Some(cursor) = &mut cursor {
            s.next_array_member(cursor).map_err(err)?
        } else {
            s.native_ref_clone(&owned_items.as_ref().unwrap()[i as usize])
                .map_err(err)?
        };
        super::history::check_deadline(deadline)?;
        let memo_fields = s.fields.is_some();
        let object = if let Some(proofs) = ordinary.as_deref_mut() {
            proofs.object(
                s,
                &record,
                raw,
                &producer,
                &mut FullObjectSlots {
                    ids: &mut ids,
                    groups: &mut groups,
                    owners: Some(&mut owners),
                },
                &mut checked_refs,
                raw_prefix.len(),
                deadline,
                budget,
            )?
        } else {
            audit_object_record_with_slots(
                s,
                &record,
                raw,
                &producer,
                &mut FullObjectSlots {
                    ids: &mut ids,
                    groups: &mut groups,
                    owners: Some(&mut owners),
                },
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                memo_fields,
                deadline,
                budget,
            )?
        };
        s.native_codec_room(&mut objects, 1).map_err(err)?;
        objects.try_push(object).map_err(err)?;
    }
    // #1394 r3：先绑定已发布未定事实，再核组成员与 pending 的不交完备并集。
    let pending = source_pending::audit_with_proofs(
        s,
        objects.iter(),
        raw,
        &groups,
        rule,
        deadline,
        ordinary.as_deref_mut(),
    )?;
    let mut partition = bj::new_map(allocation.as_ref());
    let ordered_groups = sorted_numeric_entries(s, &groups)?;
    #[cfg(test)]
    source_call_note(|n| n.full_members_enumerations += 1);
    for (expected, (index, group)) in ordered_groups.iter().enumerate() {
        super::history::check_deadline(deadline)?;
        if *index != expected as i64 {
            return Err(err("组合并序号不连续"));
        }
        for c in &group.members {
            super::history::check_deadline(deadline)?;
            s.native_map_room(&mut partition).map_err(err)?;
            if partition.insert(*c, ()).map_err(err)?.is_some() {
                return Err(err("同cut组成员重叠"));
            }
        }
    }
    drop(ordered_groups);
    for coord in pending {
        super::history::check_deadline(deadline)?;
        s.native_map_room(&mut partition).map_err(err)?;
        if partition.insert(coord, ()).map_err(err)?.is_some() {
            return Err(err("同cut组成员与初始未定来源重叠"));
        }
    }
    if partition.len() != raw.len() || partition.iter().any(|(coord, _)| !raw.contains_key(coord)) {
        return Err(err("同cut组未完整覆盖生效原始账"));
    }
    #[cfg(feature = "s_incremental")]
    if let Some(exported) = exports.as_deref_mut() {
        // 分区核通过后才导出整份 cover：组成员挂 object_id，已证 pending 无组身份。
        let seqs = winner_seqs
            .as_ref()
            .ok_or_else(|| err("cover 导出缺赢家 seq 对照"))?;
        let mut owner_of = bj::new_map(allocation.as_ref());
        for (index, group) in groups.iter() {
            let owner = owners.get(index).ok_or_else(|| err("组序号缺 object_id"))?;
            for coord in group.members.iter() {
                super::history::check_deadline(deadline)?;
                s.native_map_room(&mut owner_of).map_err(err)?;
                owner_of.insert(*coord, owner.as_str()).map_err(err)?;
            }
        }
        for (coord, _) in sorted_numeric_entries(s, raw)?.iter() {
            super::history::check_deadline(deadline)?;
            let seq = *seqs.get(coord).ok_or_else(|| err("生效坐标缺赢家 seq"))?;
            exported.push_cover(s, *coord, seq, owner_of.get(coord).copied())?;
        }
        exported.effective = raw.len();
    }
    #[cfg(not(feature = "s_incremental"))]
    let _ = &owners;
    let mut shapes = bj::new_map(allocation.as_ref());
    #[cfg(test)]
    super::state::trace_event("source_shapes", None);
    #[cfg(test)]
    source_call_note(|n| n.full_members_enumerations += 1);
    for object in objects.iter().filter(|o| o.kind == CC006_SHAPE_KIND) {
        super::history::check_deadline(deadline)?;
        let shape = if let Some(proofs) = ordinary.as_deref_mut() {
            proofs.shape(
                s,
                object,
                raw,
                &groups,
                &producer,
                &mut checked_refs,
                raw_prefix.len(),
                deadline,
                budget,
            )?
        } else {
            audit_shape_record(
                s,
                object,
                raw,
                &groups,
                &producer,
                &mut checked_refs,
                native_sources.as_deref_mut(),
                raw_prefix.len(),
                deadline,
                budget,
            )?
        };
        s.native_map_room(&mut shapes).map_err(err)?;
        shapes
            .insert(s.native_text(object.id.as_str())?, shape)
            .map_err(err)?;
    }
    let witness_array = field_ref(s, state, "witnesses")?;
    #[cfg(test)]
    super::state::trace_event("source_witnesses", None);
    let mut seen = bj::new_map(allocation.as_ref());
    #[cfg(test)]
    source_call_note(|n| n.full_members_enumerations += 1);
    for witness in elements(s, &witness_array)? {
        super::history::check_deadline(deadline)?;
        if let Some(proofs) = ordinary.as_deref_mut() {
            proofs.witness(
                s,
                &witness,
                raw,
                &groups,
                &shapes,
                &mut FullWitnessSlots(&mut seen),
                deadline,
            )?;
        } else {
            audit_witness_record(
                s,
                &witness,
                raw,
                &groups,
                &shapes,
                &mut seen,
                native_identity.as_deref_mut(),
                deadline,
            )?;
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
    #[cfg(test)]
    source_call_note(|n| n.full_members_enumerations += 1);
    for item in elements(s, &observations)? {
        super::history::check_deadline(deadline)?;
        #[cfg(feature = "s_incremental")]
        if let Some(exported) = exports.as_deref_mut() {
            let sources = source_receipt::source_facts::observation_sources(s, &item, |index| {
                groups.get(&index).map(|g| &g.sources[..])
            })?;
            s.native_codec_room(&mut exported.observation_sources, 1)
                .map_err(err)?;
            exported
                .observation_sources
                .try_push((digest(&item)?, sources))
                .map_err(err)?;
        }
        let kind = if let Some(proofs) = ordinary.as_deref_mut() {
            proofs.observation(s, &item)?
        } else {
            text(s, &item, "kind")?
        };
        match kind.as_str() {
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
    pub(super) const STATE: &[&str] = &[
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
    pub(super) fn fixture() -> (MemoryNodes, ContentRef, Vec<Value>, String) {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        let c = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&c).unwrap();
        let effective = read_effective_events(&raw).unwrap();
        let get = |k| meta_get_opt(&c, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        // #1394 整合 W-2：本夹具借 v2 的 `init_test_db` 建库取原始账，但跑的是 v3 生成；规则修订
        // 取 v3 写者自己的（v2 写者合并后是 `…+segment-second/1`，它进对象身份配方，冻结轨迹随之变）。
        let rule = crate::V3_RULE_REVISION.to_string();
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

    pub(super) fn tb1405_pending_fixture(
        rows: &[(&str, i64, i64, i64, i64, i64)],
    ) -> (MemoryNodes, ContentRef, Vec<Value>, String) {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(rows));
        let c = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&c).unwrap();
        let effective = read_effective_events(&raw).unwrap();
        let get = |k| meta_get_opt(&c, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        let rule = crate::V3_RULE_REVISION.to_string();
        let catalog = get("catalog_revision");
        let scope = serde_json::from_str(&get("scope")).unwrap();
        let mut store = MemoryNodes::default();
        let mut runtime =
            GenerationRuntime::new("tb1405-pending".to_string(), tb02a::PROFILE, &rule);
        let generated = runtime
            .generate(
                &mut store,
                &GenerationInput {
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
                },
            )
            .unwrap();
        let state = generated.generated_state.clone();
        (store, state, raw, rule)
    }

    const TB1405_EQUAL_STEP_ROWS: &[(&str, i64, i64, i64, i64, i64)] = &[
        ("e0", 0, 0, 1, 0, 1),
        ("e1", 1, 3, 10, 3, 10),
        ("e2", 2, 5, 10, 5, 10),
        ("e3", 3, 20, 30, 20, 30),
    ];

    fn tb1405_none_some_read_only(
        s: &mut MemoryNodes,
        state: &ContentRef,
        raw: &[Value],
        rule: &str,
    ) -> Counts {
        let before: Vec<_> = s.iter().map(|(d, t, b)| (*d, t, b.to_vec())).collect();
        let none = audit_generated(s, state, raw, tb02a::PROFILE, rule).unwrap();
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
        .unwrap();
        drop(reader);
        assert_eq!(none, some);
        let after: Vec<_> = s.iter().map(|(d, t, b)| (*d, t, b.to_vec())).collect();
        assert_eq!(before, after, "来源审计不得改写任何原件节点");
        none
    }

    #[test]
    fn tb1405_equal_step_evidence_audits_in_none_and_some() {
        let (mut s, state, raw, rule) = tb1405_pending_fixture(TB1405_EQUAL_STEP_ROWS);
        let counts = tb1405_none_some_read_only(&mut s, &state, &raw, &rule);
        assert_eq!((counts.groups, counts.windows, counts.effective), (3, 1, 4));
        let mut bytes = Vec::new();
        sc::render_canonical(&mut s, &state, &mut bytes).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let objects = value["objects"].as_array().unwrap();
        let key = canonical_json(&json!([CC004_STEP_KIND, "2"]));
        let step = objects.iter().find(|o| o["fact_key"] == key).unwrap();
        let payload: Value = serde_json::from_str(step["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(payload["action"], "merge");
        assert!(!payload["direction_evidence"].is_null());
        assert_eq!(
            payload["waiting_reasons"],
            json!(["equal_extreme_identity"])
        );
        let knowledge = objects
            .iter()
            .find(|o| o["kind"] == CC054_KNOWLEDGE_KIND)
            .unwrap();
        let payload: Value =
            serde_json::from_str(knowledge["payload_json"].as_str().unwrap()).unwrap();
        let request = payload["requests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["request_id"] == key)
            .unwrap();
        assert_eq!(request["reasons"], json!([]));
        assert_eq!(
            request["axes"],
            json!({"input_quality":"SUFFICIENT", "applicability":"IN_DOMAIN", "computation":"COMPLETE", "validity":"CURRENT"})
        );
    }

    #[test]
    fn tb1405_initial_unsettled_and_seed_fallback_still_audit() {
        let initial = [
            ("i0", 0, 0, 10, 0, 10),
            ("i1", 1, 1, 9, 1, 9),
            ("i2", 2, 20, 30, 20, 30),
        ];
        for rows in [&initial[..], &initial[..1]] {
            let (mut s, state, raw, rule) = tb1405_pending_fixture(rows);
            let counts = tb1405_none_some_read_only(&mut s, &state, &raw, &rule);
            assert_eq!(counts.effective, rows.len());
            assert_eq!(counts.groups, 1);
            assert_eq!(counts.windows, 0);
        }
    }

    #[cfg(feature = "s_incremental")]
    fn ledger_event(identity: &str, coord: &str, revision: i64, seq: i64) -> Value {
        json!({"identity_key": identity, "source_coord": coord, "revision": revision,
            "seq": seq, "payload_hash": "same-price", "receipt_id": format!("r-{seq}"),
            "event_id": format!("e-{seq}"), "input_revision": "1"})
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn original_sources(
        raw: &[Value],
        n: usize,
    ) -> Result<BusinessMap<i64, Source>, String> {
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
                        bar: source_pending::raw_bar(&MemoryNodes::default(), event, None, None)?,
                    },
                )
                .map_err(err)?;
        }
        Ok(sources)
    }
    #[cfg(feature = "s_incremental")]
    fn source_entries(map: &BusinessMap<i64, Source>) -> std::collections::BTreeMap<i64, Source> {
        map.iter()
            .map(|(coord, source)| (*coord, *source))
            .collect()
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_raw_ledger_measured_visits_hits_rewind_and_failure() {
        use super::super::audit_budget::AuditJob;
        use sc::job::{Existing, Job, JobLimits};
        use std::time::Instant;
        let make_job = |records: u64, nodes: u64| {
            let limits = JobLimits::from_canonical_json(&json!({
                "max_total_wire_bytes": "1048576",
                "max_total_node_bytes": "1048576",
                "max_total_nodes": nodes.to_string(),
                "max_total_records": records.to_string(),
                "max_total_requests": "20", "max_elapsed_ms": "10000",
                "max_staged_bytes": "1048576", "max_store_bytes": "33554432"
            }))
            .unwrap();
            Job::new(
                limits,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap()
        };
        let mut fixture = super::super::prepared::tests::Fixture::new();
        fixture.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        fixture.accept_begin(2, ("b", 1, 8, 16, 6, 12), 1);
        let raw = read_raw_events(&fixture.conn).unwrap();
        let capture = sc::AuditCapture::new(MemoryNodes::default());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let mut ledger = NativeRawLedger::new((&raw).into());
        let mut job = make_job(100, 10000);
        {
            let audit = AuditJob::new(&mut job);
            let got = ledger
                .advance_measured(&mut reader, 2, None, Some(&audit))
                .unwrap();
            assert_eq!(
                source_entries(got),
                source_entries(&original_sources(&raw, 2).unwrap())
            );
        }
        assert_eq!(job.snapshot().records, 6, "每行新增、坐标和来源各访问一次");
        assert!(job.snapshot().nodes > 0, "四投影进入同一 Job 节点核");
        let before = job.snapshot();
        {
            let audit = AuditJob::new(&mut job);
            ledger
                .advance_measured(&mut reader, 2, None, Some(&audit))
                .unwrap();
        }
        assert_eq!(job.snapshot().records, before.records);
        assert_eq!(job.snapshot().nodes, before.nodes);
        {
            let audit = AuditJob::new(&mut job);
            ledger
                .advance_measured(&mut reader, 1, None, Some(&audit))
                .unwrap();
            ledger
                .advance_measured(&mut reader, 2, None, Some(&audit))
                .unwrap();
        }
        assert_eq!(
            job.snapshot().records,
            8,
            "回退后独立扫描收费，坐标及来源命中不虚收"
        );
        for records in [1, 3, 4] {
            let mut limited = make_job(records, 10000);
            let mut ledger = NativeRawLedger::new((&raw).into());
            let audit = AuditJob::new(&mut limited);
            assert!(ledger
                .advance_measured(&mut reader, 2, None, Some(&audit))
                .is_err());
            assert!(ledger.sources.is_empty());
            assert!(ledger.originals.is_empty());
            assert!(ledger.winners.is_empty());
            let got = ledger.advance(&mut reader, 2).unwrap();
            assert_eq!(
                source_entries(got),
                source_entries(&original_sources(&raw, 2).unwrap())
            );
        }
        let mut limited = make_job(100, 1);
        let mut ledger = NativeRawLedger::new((&raw).into());
        let audit = AuditJob::new(&mut limited);
        assert!(ledger
            .advance_measured(&mut reader, 2, None, Some(&audit))
            .is_err());
        assert!(ledger.sources.is_empty());
        assert!(ledger.originals.is_empty());
        assert!(ledger.winners.is_empty());
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_raw_index_normal_revisions_unordered_duplicate_prices_and_replay_view() {
        use crate::v3::audit_retention::{Retained, RetainedDelta};
        let mut fixture = super::super::prepared::tests::Fixture::new();
        // 正常 Accept/Begin，实际读回独立 raw；本测试只核来源索引，不运行生成 runtime。
        for (n, row, revision) in [
            (1, ("a", 20, 6, 12, 3, 7), 1),
            (2, ("b", 0, 6, 12, 3, 7), 1),
            (3, ("c", 10, 8, 16, 6, 12), 1),
            (4, ("a", 20, 8, 17, 6, 12), 2),
        ] {
            fixture.accept_begin(n, row, revision);
        }
        let raw = read_raw_events(&fixture.conn).unwrap();
        let capture = sc::AuditCapture::new(MemoryNodes::default());
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let mut ledger = NativeRawLedger::new((&raw).into());
        let mut old_hashes = 0;
        for n in 0..=raw.len() {
            let want = original_sources(&raw, n).unwrap();
            old_hashes += 4 * want.len();
            assert_eq!(
                source_entries(ledger.advance(&mut reader, n).unwrap()),
                source_entries(&want)
            );
        }
        assert_eq!(ledger.stats.raw_visits, 4);
        assert_eq!(ledger.stats.source_hashes, 16);
        assert_eq!(old_hashes, 36);
        for n in [1, 4, 2, 4, 4] {
            assert_eq!(
                source_entries(ledger.advance(&mut reader, n).unwrap()),
                source_entries(&original_sources(&raw, n).unwrap())
            );
        }
        assert_eq!(ledger.stats.coord_decodes, 4);
        assert_eq!(ledger.stats.source_hashes, 16);
        eprintln!("有序来源原件 raw=4 cuts=4 source_hashes={old_hashes}->16；回退/跳跃不重Hash");
        eprintln!(
            "raw索引容量 [赢家map,坐标map,BTree条目]={:?}；未含BTree布局及allocator开销",
            ledger.winners.retained_slots()
        );

        let mut retained = RetainedDelta::default();
        let facts = ledger.into_facts_retained(&mut retained);
        assert_eq!(
            retained.apply(0).unwrap(),
            facts.heap_bytes(),
            "独立来源账的赢家 key、坐标、source/original 桶按真实容量计费"
        );

        // 相同修订的重复借用视图保首次；未向数据库插入重复行或修改原件。
        let repeated = [raw[0].clone(), raw[1].clone(), raw[0].clone()];
        let view = RawEventsView::plain(&repeated);
        let mut index = bj::RawPrefixIndex::new(view);
        index.advance(3).unwrap();
        assert_eq!(index.ordered().collect::<Vec<_>>(), [(0, 1), (20, 0)]);
        assert!(bj::effective(view, None)
            .unwrap()
            .iter()
            .eq(index.ordered().map(|(_, i)| view.get(i).unwrap())));
        let mut retained = RetainedDelta::default();
        let facts = index.into_facts_retained(&mut retained);
        assert_eq!(
            retained.apply(0).unwrap(),
            facts.heap_bytes(),
            "重复借用视图不增加赢家 key"
        );
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_raw_facts_keep_original_bucket_permits_across_scope_and_move() {
        let raw = vec![
            ledger_event("a", "7", 1, 1),
            ledger_event("b", "9", 1, 2),
            ledger_event("a", "7", 2, 3),
        ];
        let expected = original_sources(&raw, raw.len()).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(8 << 20, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let facts = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let mut admission = super::super::audit_store::NativeAdmission(Some(scope));
                    let mut ledger = NativeRawLedger::new((&raw).into());
                    for n in 1..=raw.len() {
                        ledger.advance(&mut admission, n).unwrap();
                    }
                    let hashed = ledger.stats.source_hashes;
                    ledger.advance(&mut admission, 1).unwrap();
                    ledger.advance(&mut admission, raw.len()).unwrap();
                    assert_eq!(ledger.stats.source_hashes, hashed, "回退重走仍复用同一原件");
                    assert_eq!(source_entries(&ledger.sources), source_entries(&expected));
                    assert_eq!(ledger.originals.len(), 3);
                    let live = budget.live();
                    let facts = ledger.into_facts();
                    assert_eq!(budget.live(), live, "移动不退款、不重签");
                    Ok::<_, sc::CodecError>(facts)
                })
                .unwrap();
            assert!(budget.live() > baseline, "离开scope仍由事实账持有原桶许可");
            assert_eq!(facts.hashed_rows(), 3);
            assert_eq!(source_entries(&facts.sources), source_entries(&expected));
            drop(facts);
            assert_eq!(budget.live(), baseline, "所有真实事实拥有者销毁后归基线");
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn source_identity_producer_outlives_scope_without_freeing_initial_strings() {
        let mut nodes = MemoryNodes::default();
        let payload = sc::write_value(&mut nodes, &V::Null).unwrap();
        let refs = sc::write_value(&mut nodes, &V::Array(vec![])).unwrap();
        let ordinary = Cc004Producer::new(tb02a::PROFILE, crate::V3_RULE_REVISION);
        let expected = ordinary
            .typed_identity_scoped(CC004_STEP_KIND, "fact", &payload, &refs, None)
            .unwrap();
        let budget = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let producer = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let producer = Cc004Producer::for_source_audit(
                        tb02a::PROFILE,
                        crate::V3_RULE_REVISION,
                        Some(scope),
                    )?;
                    assert_eq!(
                        producer
                            .typed_identity_scoped(
                                CC004_STEP_KIND,
                                "fact",
                                &payload,
                                &refs,
                                Some(scope),
                            )
                            .unwrap(),
                        expected
                    );
                    Ok::<_, sc::CodecError>(producer)
                })
                .unwrap();
            assert!(budget.live() > baseline);
            drop(producer);
            assert_eq!(budget.live(), baseline);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_source_normal_fixture_oracle() {
        let (mut nodes, state, raw, rule) = fixture();
        let expected = audit_generated(&mut nodes, &state, &raw, tb02a::PROFILE, &rule).unwrap();
        assert_eq!(expected.effective, 14);
        assert!(expected.objects > expected.groups && expected.groups > 3);
        let capture = sc::AuditCapture::new(nodes);
        let mut native = NativeSourceAudit::new(&capture, (&raw).into()).unwrap();
        let actual = native
            .audit(&state, raw.len(), tb02a::PROFILE, &rule)
            .unwrap();
        assert_eq!(format!("{expected:?}"), format!("{actual:?}"));
        eprintln!("既有正常 OHLC oracle：14条原件，完整 Counts={actual:?}");
    }
    #[cfg(feature = "s_incremental")]
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
        assert_eq!(ledger.stats.source_hashes, 20);
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
            ledger.stats.source_hashes, 20,
            "successful originals hash once, including rewind"
        );
        assert_eq!(ledger.stats.coord_decodes, 5);
        assert_eq!(ledger.stats.rewinds, 2);
        let fresh = NativeRawLedger::new((&raw).into());
        assert!(fresh.sources.is_empty() && fresh.originals.is_empty() && fresh.winners.is_empty());
    }
    #[cfg(feature = "s_incremental")]
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
        assert_eq!(ledger.stats.source_hashes, 16);
        assert_eq!(
            ledger.advance(&mut reader, 7).err().unwrap(),
            original_sources(&raw, 7).err().unwrap()
        );
        assert!(
            ledger.sources.is_empty() && ledger.originals.is_empty() && ledger.winners.is_empty()
        );
        ledger.advance(&mut reader, 6).unwrap();
        assert_eq!(
            ledger.stats.source_hashes, 24,
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
    #[cfg(feature = "s_incremental")]
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
        assert_eq!(ledger.stats.source_hashes, 8);
        assert_eq!(
            ledger.advance(&mut reader, 2).err().unwrap(),
            original_sources(&raw, 2).err().unwrap()
        );
        assert!(ledger.originals.is_empty());
    }
    #[cfg(feature = "s_incremental")]
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
        assert_eq!(stats.source_hashes, raw.len() as u64 * 4);
        assert_eq!(stats.coord_decodes, raw.len() as u64);
        assert_eq!(stats.rewinds, 2);
        assert!(stats.raw_visits < (raw.len() * (raw.len() + 1) / 2) as u64);
        assert_eq!(native.ledger.originals.len(), raw.len());
        assert!(native.audit(&roots[0], 1, "wrong-profile", &rule).is_err());
        assert!(native.ledger.sources.is_empty() && native.ledger.originals.is_empty());
        native.audit(&roots[0], 1, tb02a::PROFILE, &rule).unwrap();
        assert_eq!(
            native.raw_ledger_stats().source_hashes,
            stats.source_hashes + 4
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
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_history_record_handoff_eliminates_both_source_field_lookups() {
        let (nodes, state, raw, rule) = fixture();
        let capture = sc::AuditCapture::new(nodes);
        let mut reader = super::super::history::HashReader::captured(&capture, None);
        let object_array = reader.lookup_ref((&state).into(), &[f("objects")]).unwrap();
        let objects = reader.lookup_array_ref(object_array.view()).unwrap();
        let mut records = super::super::history::NativeRecordFields::new(&capture).unwrap();
        super::super::history::reset_native_record_field_stats();
        for object in &objects {
            let names = if object.extent() == Some(15) {
                &super::super::object_records::SHAPE_SEALED_FIELDS[..]
            } else {
                &super::super::object_records::TYPED_SEALED_FIELDS[..]
            };
            let fields = reader.lookup_fields_ref(object.view(), names).unwrap();
            records.seed(object.view(), names, &fields).unwrap();
        }
        assert!(!objects.is_empty());
        let count = objects.len() as u64;
        let ordinary =
            audit_generated_for_audit(&mut reader, &state, &raw, tb02a::PROFILE, &rule, None, None)
                .unwrap();
        let mut native =
            NativeSourceAudit::new_with_records(&capture, (&raw).into(), Some(records)).unwrap();
        let actual = native
            .audit(&state, raw.len(), tb02a::PROFILE, &rule)
            .unwrap();
        assert_eq!(format!("{ordinary:?}"), format!("{actual:?}"));
        let stats = super::super::history::native_record_field_stats();
        assert_eq!(stats.full_record_seeds, count);
        assert_eq!(
            stats.source_subset_hits,
            2 * count,
            "每个真实对象kind及core各一次字段定位均消除"
        );
        // prefix 错误仍现场拒绝，失败清除已交接材料。
        assert!(native
            .audit(&state, raw.len() + 1, tb02a::PROFILE, &rule)
            .is_err());
        let before = super::super::history::native_record_field_stats().source_subset_hits;
        native
            .audit(&state, raw.len(), tb02a::PROFILE, &rule)
            .unwrap();
        assert_eq!(
            super::super::history::native_record_field_stats().source_subset_hits,
            before
        );
        let other = sc::AuditCapture::new(MemoryNodes::default());
        let wrong = super::super::history::NativeRecordFields::new(&other).unwrap();
        assert!(NativeSourceAudit::new_with_records(&capture, (&raw).into(), Some(wrong)).is_err());
    }
    #[cfg(feature = "s_incremental")]
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
    #[cfg(feature = "s_incremental")]
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
            let scalar_root = built(fields.get("a").unwrap()).unwrap().content_ref();
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
                    built(ordinary.get("a").unwrap()).unwrap().digest,
                    built(native.get("a").unwrap()).unwrap().digest
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
            .qualified((&root).into(), None)
            .unwrap());
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_field_memo_scoped_hit_copies_clear_and_drop_keep_original_permits() {
        let mut nodes = MemoryNodes::default();
        let root = sc::write_value(
            &mut nodes,
            &cv(&json!({"a":"nonempty original", "b":17})).unwrap(),
        )
        .unwrap()
        .content_ref();
        let expected_a = field(&mut nodes, &root, "a").unwrap();
        let capture = sc::AuditCapture::new(nodes);
        let budget = super::super::audit_owner::native::Budget::new(8 << 20, 0).unwrap();
        let baseline = budget.live();
        for clear_first in [false, true] {
            // 把原 map 移出，而非复制缓存或保留 WorkScope；临时 reader 与 scope
            // 析构后测父账。若仍有 legacy retain 残费，最终回基线断言必须失败。
            let (cached_fields, first, second) = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let mut memo = NativeFieldMemo::new(&capture);
                    memo.work = Some(scope);
                    let mut reader = super::super::history::HashReader::captured(&capture, None)
                        .with_codec_work(scope, &budget);
                    let (first, second) = {
                        let mut store =
                            super::super::audit_store::StoreView::for_audit(&mut reader);
                        let mut source = ArrayAuditSource {
                            source: &mut store,
                            full_arrays: true,
                            deadline: None,
                            fields: Some(&mut memo),
                        };
                        drop(
                            SourceFields::read_subset(&mut source, (&root).into(), &["a", "b"])
                                .unwrap(),
                        );
                        let first =
                            SourceFields::read_subset(&mut source, (&root).into(), &["a", "b"])
                                .unwrap();
                        let second =
                            SourceFields::read_subset(&mut source, (&root).into(), &["a", "b"])
                                .unwrap();
                        (first, second)
                    };
                    assert_eq!(memo.field_decodes, 1);
                    assert_eq!(memo.field_hits, 2);
                    assert_eq!(memo.fields.len(), 1);
                    assert_ne!(
                        first.get("a").unwrap().root().as_ptr(),
                        second.get("a").unwrap().root().as_ptr(),
                        "命中副本各自拥有非空 ref 文本，不能共享或偷走 memo 的原件"
                    );
                    Ok::<_, sc::CodecError>((memo.fields, first, second))
                })
                .unwrap();
            let mut memo = NativeFieldMemo::new(&capture);
            memo.fields = cached_fields;
            let capacity = memo.fields.capacity();
            assert!(capacity > 0);
            let all_live = budget.live();
            assert!(all_live > baseline);
            if clear_first {
                memo.clear();
                assert!(budget.live() < all_live, "clear 销毁实际键和值后退对应许可");
            }
            assert_eq!(first.get("a").unwrap().root(), expected_a.view().root());
            assert_eq!(second.get("a").unwrap().root(), expected_a.view().root());
            let before_first_drop = budget.live();
            drop(first);
            let after_first_drop = budget.live();
            assert!(after_first_drop < before_first_drop, "第一个副本独立退费");
            assert_eq!(second.get("a").unwrap().root(), expected_a.view().root());
            let before_second_drop = budget.live();
            drop(second);
            let after_second_drop = budget.live();
            assert_eq!(
                before_first_drop - after_first_drop,
                before_second_drop - after_second_drop,
                "相同字段的两次 hit 各自持有完整、等额且独立的副本许可"
            );
            if !clear_first {
                memo.clear();
                assert!(budget.live() < after_second_drop);
            }
            assert!(memo.fields.is_empty());
            assert_eq!(memo.fields.capacity(), capacity, "clear 保留真实桶容量");
            let buckets_live = budget.live();
            assert!(buckets_live > baseline, "空缓存的活桶仍收费");
            memo.clear();
            assert_eq!(budget.live(), buckets_live, "重复 clear 不重复退款");
            drop(memo);
            assert_eq!(budget.live(), baseline, "最终桶拥有者销毁才回基线");
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_witness_memo_scoped_hits_clear_and_drop_keep_original_permits() {
        let mut nodes = MemoryNodes::default();
        let bars = sc::write_value(
            &mut nodes,
            &cv(&json!([{ "source_coord":"7", "open":"10", "high":"12",
                "low":"9", "close":"11", "identity_key":"bar-7" }]))
            .unwrap(),
        )
        .unwrap();
        let capture = sc::AuditCapture::new(nodes);
        let mut ordinary = super::super::history::HashReader::captured(&capture, None);
        let expected =
            super::super::generation::witness_identity(&mut ordinary, "obj3-scoped", 2, 7, &bars)
                .unwrap();
        let budget = super::super::audit_owner::native::Budget::new(8 << 20, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..2 {
            let mut memo = NativeWitnessIdentityMemo::new(&capture).unwrap();
            let check = |memo: &mut NativeWitnessIdentityMemo<'_>, stored: &str| {
                budget.codec_work().unwrap().run(|scope| {
                    let mut reader = super::super::history::HashReader::captured(&capture, None)
                        .with_codec_work(scope, &budget);
                    Ok::<_, sc::CodecError>(memo.check_identity(
                        &mut reader,
                        stored,
                        "obj3-scoped",
                        2,
                        7,
                        &bars,
                    ))
                })
            };
            assert!(check(&mut memo, "wrong-id").unwrap().is_err());
            assert!(memo.entries.is_empty());
            assert_eq!(budget.live(), baseline, "失败首次计算不能留下键或文本费用");
            check(&mut memo, &expected).unwrap().unwrap();
            let cached_live = budget.live();
            assert!(cached_live > baseline, "离开 scope，非空身份缓存仍持原许可");
            assert_eq!(memo.entries.len(), 1);
            let rendered = memo.stats.render_calls;
            for stored in [expected.as_str(), "wrong-id", expected.as_str()] {
                assert_eq!(
                    check(&mut memo, stored).unwrap().is_ok(),
                    stored == expected
                );
                assert_eq!(
                    budget.live(),
                    cached_live,
                    "命中临时 key 和验证工作均归还，不重复持费"
                );
                assert_eq!(memo.entries.len(), 1);
            }
            assert_eq!(memo.stats.hits, 3);
            assert_eq!(memo.stats.render_calls, rendered, "命中不重算见证身份");
            let capacity = memo.entries.capacity();
            memo.entries.retain(|_, _| false);
            assert!(memo.entries.is_empty());
            assert_eq!(memo.entries.capacity(), capacity);
            let buckets_live = budget.live();
            assert!(buckets_live > baseline && buckets_live < cached_live);
            memo.entries.retain(|_, _| false);
            assert_eq!(budget.live(), buckets_live, "空桶不能再次退款");
            drop(memo);
            assert_eq!(budget.live(), baseline, "键、身份文本及桶全部销毁后回基线");
        }
    }

    #[cfg(feature = "s_incremental")]
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
        let key = memo.key(&reader, "obj3-first", 0, 1, &bars).unwrap();
        let mut changed = bars.clone();
        changed.kind = Kind::Object;
        assert_ne!(
            key,
            memo.key(&reader, "obj3-first", 0, 1, &changed).unwrap()
        );
        changed = bars.clone();
        changed.extent = Some(999);
        assert_ne!(
            key,
            memo.key(&reader, "obj3-first", 0, 1, &changed).unwrap()
        );
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
        assert_ne!(key, fresh.key(&reader, "obj3-first", 0, 1, &bars).unwrap());
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_source_measured_preserves_fields_witness_and_failure_cleanup() {
        use super::super::audit_budget::AuditJob;
        use sc::job::{Existing, Job, JobLimits};
        let make_job = |records: u64| {
            Job::new(
                JobLimits::from_canonical_json(&json!({
                    "max_total_wire_bytes":"1048576", "max_total_node_bytes":"1048576",
                    "max_total_nodes":"100000", "max_total_records":records.to_string(),
                    "max_total_requests":"20", "max_elapsed_ms":"10000",
                    "max_staged_bytes":"1048576", "max_store_bytes":"33554432"
                }))
                .unwrap(),
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(std::time::Instant::now),
            )
            .unwrap()
        };
        let (nodes, state, raw, rule) = fixture();
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse()
            .unwrap();
        let expected = NativeSourceAudit::new(&capture, (&raw).into())
            .unwrap()
            .audit(&state, raw.len(), tb02a::PROFILE, &rule)
            .unwrap();
        let mut job = make_job(100000);
        {
            let audit = AuditJob::new(&mut job);
            let mut native = NativeSourceAudit::new_with_records_measured(
                &capture,
                (&raw).into(),
                None,
                None,
                Some(&audit),
                Some(std::time::Instant::now() + std::time::Duration::from_secs(10)),
                super::super::history::SourcePolicy::NativeMeasured,
            )
            .unwrap();
            let got = native
                .audit(&state, raw.len(), tb02a::PROFILE, &rule)
                .unwrap();
            assert_eq!(format!("{got:?}"), format!("{expected:?}"));
            let decoded = native.fields.field_decodes;
            assert!(decoded > 0);
            native
                .audit(&state, raw.len(), tb02a::PROFILE, &rule)
                .unwrap();
            assert_eq!(native.fields.field_decodes, decoded);
            assert!(native.fields.field_hits > 0);
            assert!(native.memo.stats.hits > 0);
        }
        assert!(job.snapshot().records >= (raw.len() * 3) as u64);
        assert!(job.snapshot().nodes > 0);
        let mut limited = make_job(1);
        let audit = AuditJob::new(&mut limited);
        let mut native = NativeSourceAudit::new_with_records_measured(
            &capture,
            (&raw).into(),
            None,
            None,
            Some(&audit),
            None,
            super::super::history::SourcePolicy::NativeMeasured,
        )
        .unwrap();
        assert!(native
            .audit(&state, raw.len(), tb02a::PROFILE, &rule)
            .is_err());
        assert!(native.fields.fields.is_empty() && native.fields.scalars.values.is_empty());
        assert!(native.ledger.sources.is_empty() && native.ledger.originals.is_empty());
        assert!(native.memo.entries.is_empty());
    }

    #[cfg(feature = "s_incremental")]
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
        // #1394（与根裁定 B / 240b061e66 同因）：FIN 04e8a7f48b 的 CC004 incoming 与原始 OHLC
        // 五字段绑定排在见证检查之前，只改 high 现由它拒绝；见证 raw_bars 绑定的完整原始字节改由
        // 只改 raw_text（不在 RawReference 八字段、也不在 CC004 OHLC 投影里）在 Native 臂锁住。
        let mut revised = raw.clone();
        revised[0]["high"] = json!("999999");
        let mut revised_context = NativeSourceAudit::new(&capture, (&revised).into()).unwrap();
        let error = run(&mut revised_context, &state, &revised).unwrap_err();
        assert_eq!(
            error,
            "SourceAuditInvalid：CC004 incoming/槽位/OHLC不是同cut原始来源"
        );
        let mut revised = raw.clone();
        let text = revised[0]["raw_text"]
            .as_str()
            .expect("原始账带 raw_text")
            .to_string();
        revised[0]["raw_text"] = json!(format!("{text} 篡改"));
        let mut revised_context = NativeSourceAudit::new(&capture, (&revised).into()).unwrap();
        let error = run(&mut revised_context, &state, &revised).unwrap_err();
        assert_eq!(
            error,
            "SourceAuditInvalid：见证raw_bars不是同cut真实原始成员"
        );
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
    #[cfg(feature = "s_incremental")]
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
        let expected = super::super::generation::witness_identity_native(
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
    pub(super) fn replace_field<'a>(
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
        elements[index] = RefValue::from(record);
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
                    fields.get_admitted(s, name).map_err(err)?
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
                    bj::JsonOwner::Plain(v) => v.as_ref().clone(),
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
        // #1394 r3：相同原 fixture 新增 CC004/CC054 绑定及 raw bar 投影费用。
        // dc425df7 旧轨迹为 151351 reads / 170797 F；本次未缩减原访问，
        // 新轨迹增加 13016 reads / 14276 F，固定以下实测计数和完整有序摘要。
        // #1394 合入 #1405 后重冻：对象/组/窗口计数不变，读取与 F 各多 120，为合并后实测值；
        // 规则修订升 new-bi/2，发布内容按 #1343 同价裁定改变，摘要随之改变。三点对比：#1405 适配合并
        // `271b8e8cc6` 之后的 `813fc75fa5`、`199caedfd0`、`dccb69793c` 读数与摘要逐值相同，故 +120 来自该
        // 合并本身（同价语义与规则修订），不来自后两处修复；未逐节点分解。
        // 旧摘要（new-bi/1）：read 8418acae…、budget f194dbc3…。
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
        assert_eq!(roots.len(), 151351 + 13016 + 120, "r3完整Some读取轨迹");
        assert_eq!(frames, 170797 + 14276 + 120, "r3完整Some F次数");
        assert_eq!(
            read_hash,
            "ffa6288e00eb063d0d10c234ae2f013fff0f15ce5e94e9777c5d0bf33359f2e9"
        );
        assert_eq!(
            budget_hash,
            "06e35514afeb5411501d062780ddd409a1796c54a38f3eac7912596d8eeda26e"
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
        let mut original_reader = super::super::history::HashReader::metered(s, None, None, None);
        let original = frozen_oracle::audit_generated_in_store(
            &mut super::super::audit_store::StoreView::for_audit(&mut original_reader),
            state.into(),
            raw.into(),
            tb02a::PROFILE,
            rule,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        drop(original_reader);
        assert_eq!(none, original, "#1488 旧独立 oracle 与局部核首错一致");
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
        parts[0] = RefValue::from(replace_field(
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
                RefValue::from(
                    sc::path::lookup_path_refview(&mut s, p.view(), &[PathStep::JsonText]).unwrap(),
                ) == RefValue::from(payload.clone())
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
    fn reidentified_legacy_bi_header_and_knowledge_domain_are_rejected_in_v2() {
        for (kind, key, replacement, reason) in [
            (
                "CC-008.endpoint",
                "schema_revision",
                "s-new-bi/1",
                "新笔载荷语义头不同",
            ),
            (
                CC054_KNOWLEDGE_KIND,
                "domain",
                "established_direction_without_extreme_identity_competition",
                "TB-02-A 知识域与cut规则版本不符",
            ),
        ] {
            let (mut s, state, raw, rule) = fixture();
            let objects_root = field(&mut s, &state, "objects").unwrap();
            let objects = elements(&mut s, &objects_root).unwrap();
            let index = objects
                .iter()
                .position(|object| text(&mut s, object, "kind").unwrap() == kind)
                .unwrap();
            let payload = field(&mut s, &objects[index], "payload_json").unwrap();
            let payload =
                sc::path::lookup_path_refview(&mut s, payload.view(), &[PathStep::JsonText])
                    .unwrap();
            let mut bytes = Vec::new();
            sc::render_canonical(&mut s, &payload, &mut bytes).unwrap();
            let mut value: Value = newchan_rust::session_protocol::strict_json(&bytes).unwrap();
            value[key] = json!(replacement);
            let changed_payload = sc::write_value(&mut s, &cv(&value).unwrap())
                .unwrap()
                .content_ref();
            let refs = field(&mut s, &objects[index], "input_refs_json").unwrap();
            let refs =
                sc::path::lookup_path_refview(&mut s, refs.view(), &[PathStep::JsonText]).unwrap();
            let fact_key = text(&mut s, &objects[index], "fact_key").unwrap();
            let id = Cc004Producer::new(tb02a::PROFILE, &rule)
                .typed_object_id(
                    kind,
                    &fact_key,
                    &built(&changed_payload).unwrap(),
                    &built(&refs).unwrap(),
                )
                .unwrap();
            let payload_json = json_text_edge(&mut s, &changed_payload);
            let changed = replace_field(
                &mut s,
                &objects[index],
                TYPED,
                "payload_json",
                V::Edge(digest(&payload_json).unwrap()),
            );
            let changed = replace_field(&mut s, &changed, TYPED, "object_id", V::String(id));
            let forged = replace_member(&mut s, &state, "objects", index, changed);
            assert_rejected_in_both_modes(&mut s, &forged, &raw, &rule, reason);
        }
    }

    #[test]
    fn tb1405_reidentified_equal_step_request_regressions_are_rejected() {
        for (axis, wrong) in [
            ("reasons", "equal_extreme_identity"),
            ("input_quality", "INSUFFICIENT"),
            ("applicability", "DOMAIN_PROOF_MISSING"),
            ("computation", "INCOMPLETE"),
            ("validity", "STALE"),
        ] {
            let (mut s, state, raw, rule) = tb1405_pending_fixture(TB1405_EQUAL_STEP_ROWS);
            let objects_root = field(&mut s, &state, "objects").unwrap();
            let objects = elements(&mut s, &objects_root).unwrap();
            let index = objects
                .iter()
                .position(|object| text(&mut s, object, "kind").unwrap() == CC054_KNOWLEDGE_KIND)
                .unwrap();
            let payload = field(&mut s, &objects[index], "payload_json").unwrap();
            let payload =
                sc::path::lookup_path_refview(&mut s, payload.view(), &[PathStep::JsonText])
                    .unwrap();
            let mut bytes = Vec::new();
            sc::render_canonical(&mut s, &payload, &mut bytes).unwrap();
            let mut value: Value = serde_json::from_slice(&bytes).unwrap();
            let key = canonical_json(&json!([CC004_STEP_KIND, "2"]));
            let request = value["requests"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|request| request["request_id"] == key)
                .unwrap();
            let expected_error = if axis == "reasons" {
                request["reasons"] = json!([wrong]);
                "CC004知识请求未绑定版本化等待reasons"
            } else {
                request["axes"][axis] = json!(wrong);
                "CC004知识请求四轴不同"
            };
            let changed_payload = sc::write_value(&mut s, &cv(&value).unwrap())
                .unwrap()
                .content_ref();
            let refs = field(&mut s, &objects[index], "input_refs_json").unwrap();
            let refs =
                sc::path::lookup_path_refview(&mut s, refs.view(), &[PathStep::JsonText]).unwrap();
            let fact_key = text(&mut s, &objects[index], "fact_key").unwrap();
            let id = Cc004Producer::new(tb02a::PROFILE, &rule)
                .typed_object_id(
                    CC054_KNOWLEDGE_KIND,
                    &fact_key,
                    &built(&changed_payload).unwrap(),
                    &built(&refs).unwrap(),
                )
                .unwrap();
            let payload_json = json_text_edge(&mut s, &changed_payload);
            let changed = replace_field(
                &mut s,
                &objects[index],
                TYPED,
                "payload_json",
                V::Edge(digest(&payload_json).unwrap()),
            );
            let changed = replace_field(&mut s, &changed, TYPED, "object_id", V::String(id));
            let forged = replace_member(&mut s, &state, "objects", index, changed);
            // Merkle与六字段身份均合法，必须由版本化请求/四轴语义拒绝。
            assert_rejected_in_both_modes(&mut s, &forged, &raw, &rule, expected_error);
        }
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
    /// #1394 整合（根裁定 B）：原名 witness_original_bytes_are_bound_even_if_reference_metadata_is_unchanged。
    /// FIN 04e8a7f48b 新增的 CC004 incoming 与原始 OHLC 绑定（source_pending.rs）先于见证检查执行，
    /// 只改 raw[0].high（引用元数据不变）现在由它拒绝、走不到见证检查；按实际走到的检查改名并精确匹配
    /// 其文案。见证绑定由下一用例单独锁住。
    #[test]
    fn raw_ohlc_tamper_with_unchanged_reference_is_rejected_by_cc004_incoming_binding() {
        let (mut s, state, mut raw, rule) = fixture();
        raw[0]["high"] = json!("999999");
        let error = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert_eq!(
            error,
            "SourceAuditInvalid：CC004 incoming/槽位/OHLC不是同cut原始来源"
        );
    }
    /// #1394 整合（根裁定 B）：见证 raw_bars 绑定完整原始事件字节。只改 raw[0] 的 raw_text——它不在
    /// 引用元数据（RawReference 八字段）里，也不在 CC004 比对的 OHLC 五字段投影里，所以引用绑定与 CC004
    /// 绑定都成立，只有见证 raw_bars 与同 cut 原始成员的完整字节不符。端到端走生产 audit_generated。
    #[test]
    fn witness_original_bytes_are_bound_even_if_reference_metadata_and_ohlc_are_unchanged() {
        let (mut s, state, mut raw, rule) = fixture();
        let text = raw[0]["raw_text"]
            .as_str()
            .expect("原始账带 raw_text")
            .to_string();
        raw[0]["raw_text"] = json!(format!("{text} 篡改"));
        let error = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert_eq!(
            error,
            "SourceAuditInvalid：见证raw_bars不是同cut真实原始成员"
        );
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

    #[test]
    fn source_field_uses_borrowed_entry_without_generic_retry() {
        let mut probe = super::super::state::borrowed_field_probe::Probe::new();
        let root = probe.reference.clone();
        assert_eq!(
            field_ref(&mut probe, (&root).into(), "borrowed😀")
                .unwrap()
                .digest()
                .unwrap(),
            root.digest().unwrap()
        );
        assert_eq!(probe.fields, 1);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_source_single_field_borrowed_navigation_roles_failure_and_layout_refusal() {
        let mut nodes = sc::MemoryNodes::new();
        let root = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("a".into(), V::String("1".into())),
                ("b".into(), V::String("2".into())),
                ("c".into(), V::String("3".into())),
            ]),
        )
        .unwrap()
        .content_ref();
        let ordinary = sc::AuditCapture::new(nodes.clone());
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let capture_live = budget.snapshot().live_requested_layout_bytes;
        let operation = |reader: &mut HashReader<'_>, full_arrays, deadline| {
            let frames = sc::frame_calls_on_thread();
            let mut view = StoreView::for_audit(reader);
            let mut source = ArrayAuditSource {
                source: &mut view,
                full_arrays,
                deadline,
                fields: None,
            };
            let got = field_ref(&mut source, (&root).into(), "b").unwrap();
            let failure = field_ref(&mut source, (&root).into(), "missing").unwrap_err();
            let shape = field_ref(&mut source, got.view(), "b").unwrap_err();
            drop(source);
            drop(view);
            assert!(
                !reader.audit_full_array_scan(),
                "成功与两类失败均恢复inner角色"
            );
            (
                got.digest().unwrap(),
                failure,
                shape,
                sc::frame_calls_on_thread() - frames,
            )
        };
        for (full_arrays, deadline) in [
            (true, None),
            (false, None),
            (
                true,
                Some(std::time::Instant::now() + std::time::Duration::from_secs(30)),
            ),
        ] {
            let expected = operation(
                &mut HashReader::captured(&ordinary, None),
                full_arrays,
                deadline,
            );
            let actual = with_overlay_prototype(
                OverlayPrototype {
                    slots: 32768,
                    bytes: 1_000_000,
                    budget: budget.clone(),
                    peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
                },
                || {
                    operation(
                        &mut HashReader::captured_for_audit(&owned, None).unwrap(),
                        full_arrays,
                        deadline,
                    )
                },
            );
            assert_eq!(actual, expected);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, capture_live);
        }
        with_overlay_prototype(
            OverlayPrototype {
                slots: 32768,
                bytes: 1_000_000,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&owned, None).unwrap();
                // 通用读取故障探针：实际Owned捕获不得经擦除后的动态source完成导航。
                assert_eq!(
                    reader.read(&root.digest().unwrap()).unwrap_err().code(),
                    sc::capture_allocation::INCOMPLETE
                );
                let mut view = StoreView::for_audit(&mut reader);
                let mut source = ArrayAuditSource {
                    source: &mut view,
                    full_arrays: true,
                    deadline: None,
                    fields: None,
                };
                field_ref(&mut source, (&root).into(), "b").unwrap();
                let live = budget.snapshot().live_requested_layout_bytes;
                let discovery =
                    CaptureSlab::new(0, budget.snapshot().limit_bytes - live - 1, &budget).unwrap();
                let failure = field_ref(&mut source, (&root).into(), "b").unwrap_err();
                assert!(failure.contains(sc::capture_allocation::LIMIT), "{failure}");
                let layout = budget.allocation_request_probe();
                assert!(layout.refused_kind.ends_with("::path::Ancestor"));
                drop(discovery);
                let ballast = CaptureSlab::new(
                    0,
                    budget.snapshot().limit_bytes - live - layout.refused_bytes + 1,
                    &budget,
                )
                .unwrap();
                let before = budget.snapshot();
                let failure = field_ref(&mut source, (&root).into(), "b").unwrap_err();
                assert!(failure.contains(sc::capture_allocation::LIMIT), "{failure}");
                let refusal = budget.allocation_request_probe();
                assert_eq!(refusal.refused_bytes, layout.refused_bytes);
                assert_eq!(refusal.refused_align, layout.refused_align);
                assert_eq!(
                    refusal.live_before_refusal + refusal.refused_bytes,
                    before.limit_bytes + 1
                );
                assert_eq!(
                    budget.snapshot().allocation_attempts,
                    before.allocation_attempts
                );
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    before.live_requested_layout_bytes
                );
                drop(ballast);
                assert!(matches!(
                    field_ref(&mut source, (&root).into(), "b").unwrap(),
                    RefValue::Owned(_)
                ));
                drop(source);
                drop(view);
                assert!(!reader.audit_full_array_scan());
            },
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, capture_live);
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            budget.snapshot().released_layouts
        );
    }
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
        coords(&mut source, &RefValue::from(root.clone()))
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
    s.lookup_field_ref(r, k).map_err(err)
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
        s.native_codec_room(&mut out, 1).map_err(err)?;
        out.try_push(v).map_err(err)?;
    }
    Ok(out)
}

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::retained_struct;

    retained_struct!(Source {
        wire,
        numeric,
        raw,
        bar
    });
    retained_struct!(NativeRawFacts {
        winners,
        sources,
        originals
    });
}

#[cfg(all(test, feature = "s_incremental"))]
impl NativeRawFacts {
    pub(super) fn h1_canonical(&self) -> Value {
        fn source(s: &Source) -> Value {
            let Source {
                wire,
                numeric,
                raw,
                bar,
            } = s;
            json!({"wire":wire,"numeric":numeric,"raw":raw,"bar":bar})
        }
        let Self {
            winners,
            sources,
            originals,
        } = self;
        let mut sources = sources
            .iter()
            .map(|(k, v)| json!([k, source(v)]))
            .collect::<Vec<_>>();
        sources.sort_by_key(crate::canonical_json);
        let mut originals = originals
            .iter()
            .map(|(k, (seq, s))| json!([k, seq, source(s)]))
            .collect::<Vec<_>>();
        originals.sort_by_key(crate::canonical_json);
        json!({"winners":winners.h1_canonical(),"sources":sources,"originals":originals})
    }
}
