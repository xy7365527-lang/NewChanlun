//! #1394 封闭审计读取边界。动态来源没有捕获能力；唯有具体 HashReader 自己选择
//! 与实际读取绑定的不可变 owner。两种路径都用同一业务函数，不传成功 token。
use super::history::HashReader;
use newchan_rust::s_content::{
    self as sc, CodecError, ContentRef, NodeDigest, NodeRead, NodeSink, NodeSource, NodeStore,
};

use sc::path::PathStep;
use std::io::Write;

/// 仅bin内的静态codec门；具体捕获owner不可擦除成外来来源。
pub(crate) trait CodecRead: NodeSource {
    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, CodecError> {
        let refs = sc::path::lookup_array_items_refview(self.generic_source(), r)?;
        let mut out = sc::CodecVec::from_plain(Vec::new());
        for r in refs {
            out.try_push(RefValue::Plain(r))?;
        }
        Ok(out)
    }
    fn verify_many_ref<'a>(
        &mut self,
        refs: impl IntoIterator<Item = sc::RefView<'a>>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        sc::audit_verify_many_refview(self.generic_source(), refs, mode)
    }
    fn verify_many_ref_fresh<'a>(
        &mut self,
        refs: impl IntoIterator<Item = sc::RefView<'a>>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        self.verify_many_ref(refs, mode)
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        None
    }
    fn content_ref_view(&mut self, root: &str) -> Result<RefValue, CodecError> {
        self.content_ref_of(root).map(RefValue::Plain)
    }
    fn lookup_ref(&mut self, r: sc::RefView<'_>, p: &[PathStep]) -> Result<RefValue, CodecError> {
        sc::path::lookup_path_refview(self.generic_source(), r, p).map(RefValue::Plain)
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_refview(self.generic_source(), r, names, false)
            .map(RefFields::Plain)
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_refview(self.generic_source(), r, names, true).map(RefFields::Plain)
    }
    fn verify_ref(&mut self, r: sc::RefView<'_>, mode: sc::Mode) -> Result<(), CodecError> {
        sc::verify_refview(self.generic_source(), r, mode)
    }
    fn render_ref(&mut self, r: sc::RefView<'_>, out: &mut dyn Write) -> Result<(), CodecError> {
        sc::render_refview(self.generic_source(), r, out)
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource;
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        sc::CodecVec::from_plain(Vec::new())
    }
    fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, CodecError> {
        sc::content_ref_of(self.generic_source(), root)
    }
    fn lookup_path(&mut self, r: &ContentRef, p: &[PathStep]) -> Result<ContentRef, CodecError> {
        sc::path::lookup_path(self.generic_source(), r, p)
    }
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, CodecError> {
        sc::path::lookup_array_items(self.generic_source(), r).map(sc::CodecVec::from_plain)
    }
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        sc::path::lookup_object_fields(self.generic_source(), r, n).map(sc::CodecFields::from_plain)
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        sc::path::lookup_object_fields_subset(self.generic_source(), r, n)
            .map(sc::CodecFields::from_plain)
    }
    fn render_canonical(&mut self, r: &ContentRef, w: &mut dyn Write) -> Result<(), CodecError> {
        sc::render_canonical(self.generic_source(), r, w)
    }
    fn verify_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::verify(self.generic_source(), r).map(|_| ()),
            sc::Mode::Template => sc::verify_template(self.generic_source(), r).map(|_| ()),
        }
    }
    fn verify_audit_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::audit_verify_public(self.generic_source(), r),
            sc::Mode::Template => sc::audit_verify(self.generic_source(), r, mode),
        }
    }
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        sc::audit_verify_many(self.generic_source(), r, mode)
    }
    fn verify_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::verify_many(self.generic_source(), r).map(|_| ()),
            sc::Mode::Template => sc::verify_template_many(self.generic_source(), r).map(|_| ()),
        }
    }
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        self.verify_fresh(r, mode)
    }
    fn verify_many_cached(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        self.verify_many_fresh(r, mode)
    }
}
pub(crate) trait CodecStore: CodecRead + NodeSink {
    fn replace_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        batch: &str,
    ) -> Result<RefValue, CodecError> {
        sc::path::replace_batch_placeholder_refview(self.generic_store(), r, p, batch)
            .map(RefValue::Plain)
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        sc::write_value_view(self.generic_store(), v)
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        sc::write_template_view(self.generic_store(), v)
    }
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<sc::Built, CodecError> {
        sc::write_json_view(self.generic_store(), v)
    }
    fn generic_store(&mut self) -> &mut dyn NodeStore;
    fn write_value(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_value(self.generic_store(), v)
    }
    fn write_template(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_template(self.generic_store(), v)
    }
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        batch: &str,
    ) -> Result<ContentRef, CodecError> {
        sc::path::replace_batch_placeholder(self.generic_store(), r, p, batch)
    }
}
impl CodecRead for dyn NodeSource + '_ {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecRead for dyn NodeStore + '_ {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for dyn NodeStore + '_ {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for sc::MemoryNodes {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for sc::MemoryNodes {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for crate::content_store::TxStore<'_, '_> {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for crate::content_store::TxStore<'_, '_> {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for crate::content_store::ReadStore<'_> {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}

enum Backend<'a, 'source> {
    Generic(&'a mut dyn NodeStore),
    Audit(&'a mut HashReader<'source>),
}
/// 字段及Backend私有；不能把任意dyn store和另一份capture拼成一条准入路径。
pub(crate) struct StoreView<'a, 'source> {
    backend: Backend<'a, 'source>,
}
impl<'a, 'source> StoreView<'a, 'source> {
    pub(crate) fn generic(s: &'a mut dyn NodeStore) -> Self {
        Self {
            backend: Backend::Generic(s),
        }
    }
    pub(crate) fn for_audit(s: &'a mut HashReader<'source>) -> Self {
        Self {
            backend: Backend::Audit(s),
        }
    }
    fn store(&mut self) -> &mut dyn NodeStore {
        match &mut self.backend {
            Backend::Generic(s) => &mut **s,
            Backend::Audit(s) => &mut **s,
        }
    }
    fn source(&self) -> &dyn NodeSource {
        match &self.backend {
            Backend::Generic(s) => &**s,
            Backend::Audit(s) => &**s,
        }
    }
    pub(crate) fn owned_codec_active(&self) -> bool {
        match &self.backend {
            Backend::Generic(_) => false,
            Backend::Audit(s) => s.owned_codec_active(),
        }
    }
    pub(crate) fn swap_codec_full_array_role(&mut self, enabled: bool) -> bool {
        match &mut self.backend {
            Backend::Generic(s) => s.audit_full_array_scan(),
            Backend::Audit(s) => s.swap_codec_full_array_role(enabled),
        }
    }
    pub(crate) fn verify_public(&mut self, r: &ContentRef) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => sc::verify(&mut **s, r).map(|_| ()),
            Backend::Audit(s) => s.verify_public(r),
        }
    }
    pub(crate) fn verify_many(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => sc::audit_verify_many(&mut **s, r, mode),
            Backend::Audit(s) => s.verify_many(r, mode),
        }
    }
}
impl CodecRead for StoreView<'_, '_> {
    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_array_ref(r),
            Backend::Audit(s) => s.lookup_array_ref(r),
        }
    }
    fn verify_many_ref<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_ref(r, m),
            Backend::Audit(s) => s.verify_many_ref(r, m),
        }
    }
    fn verify_many_ref_fresh<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_ref_fresh(r, m),
            Backend::Audit(s) => s.verify_many_ref_fresh(r, m),
        }
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        match &self.backend {
            Backend::Generic(s) => s.allocation_budget(),
            Backend::Audit(s) => s.allocation_budget(),
        }
    }
    fn content_ref_view(&mut self, r: &str) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.content_ref_view(r),
            Backend::Audit(s) => s.content_ref_view(r),
        }
    }
    fn lookup_ref(&mut self, r: sc::RefView<'_>, p: &[PathStep]) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_ref(r, p),
            Backend::Audit(s) => s.lookup_ref(r, p),
        }
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_fields_ref(r, n),
            Backend::Audit(s) => s.lookup_fields_ref(r, n),
        }
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_subset_ref(r, n),
            Backend::Audit(s) => s.lookup_subset_ref(r, n),
        }
    }
    fn render_ref(&mut self, r: sc::RefView<'_>, o: &mut dyn Write) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.render_ref(r, o),
            Backend::Audit(s) => s.render_ref(r, o),
        }
    }
    fn verify_ref(&mut self, r: sc::RefView<'_>, m: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_ref(r, m),
            Backend::Audit(s) => s.verify_ref(r, m),
        }
    }
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        match &mut self.backend {
            Backend::Generic(s) => s.new_ref_result(),
            Backend::Audit(s) => s.new_ref_result(),
        }
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
    fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.content_ref_of(root),
            Backend::Audit(s) => s.content_ref_of(root),
        }
    }
    fn lookup_path(&mut self, r: &ContentRef, p: &[PathStep]) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_path(r, p),
            Backend::Audit(s) => s.lookup_path(r, p),
        }
    }
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_array_items(r),
            Backend::Audit(s) => s.lookup_array_items(r),
        }
    }
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_object_fields(r, n),
            Backend::Audit(s) => s.lookup_object_fields(r, n),
        }
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_object_fields_subset(r, n),
            Backend::Audit(s) => s.lookup_object_fields_subset(r, n),
        }
    }
    fn render_canonical(&mut self, r: &ContentRef, w: &mut dyn Write) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.render_canonical(r, w),
            Backend::Audit(s) => s.render_canonical(r, w),
        }
    }
    fn verify_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_fresh(r, mode),
            Backend::Audit(s) => s.verify_fresh(r, mode),
        }
    }
    fn verify_audit_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_audit_fresh(r, mode),
            Backend::Audit(s) => s.verify_audit_fresh(r, mode),
        }
    }
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_audit_many_fresh(r, mode),
            Backend::Audit(s) => s.verify_audit_many_fresh(r, mode),
        }
    }
    fn verify_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_fresh(r, mode),
            Backend::Audit(s) => s.verify_many_fresh(r, mode),
        }
    }
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_cached(r, mode),
            Backend::Audit(s) => s.verify_cached(r, mode),
        }
    }
    fn verify_many_cached(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_cached(r, mode),
            Backend::Audit(s) => s.verify_many_cached(r, mode),
        }
    }
}
impl CodecStore for StoreView<'_, '_> {
    fn replace_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        batch: &str,
    ) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.replace_ref(r, p, batch),
            Backend::Audit(s) => s.replace_ref(r, p, batch),
        }
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_value_view(v),
            Backend::Audit(s) => s.write_value_view(v),
        }
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_template_view(v),
            Backend::Audit(s) => s.write_template_view(v),
        }
    }
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_json_view(v),
            Backend::Audit(s) => s.write_json_view(v),
        }
    }
    fn write_value(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_value(v),
            Backend::Audit(s) => s.write_value(v),
        }
    }
    fn write_template(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_template(v),
            Backend::Audit(s) => s.write_template(v),
        }
    }
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        batch: &str,
    ) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.replace_batch_placeholder(r, p, batch),
            Backend::Audit(s) => s.replace_batch_placeholder(r, p, batch),
        }
    }
}
impl NodeSource for StoreView<'_, '_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.store().read(d)
    }
    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, CodecError> {
        self.store().checked_read(d)
    }
    fn before_path_index(&mut self, p: &[sc::path::PathStep], i: u64) -> Result<(), CodecError> {
        self.store().before_path_index(p, i)
    }
    fn audit_full_array_scan(&self) -> bool {
        self.source().audit_full_array_scan()
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.source().max_render_bytes()
    }
}
impl NodeSink for StoreView<'_, '_> {
    fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), CodecError> {
        self.store().put(d, t, b)
    }
}

// 单独cargo rustc -- --cfg issue1394_closed_owner_refusal实际编译本段，预期拒绝。
// 普通构建不包含本编译反例；它不用stub类型冒充真实边界。
#[allow(unexpected_cfgs)]
#[cfg(issue1394_closed_owner_refusal)]
mod compile_refusal {
    use super::*;
    fn dynamic_cannot_select_captured_branch(
        source: &mut dyn NodeStore,
        capture: &sc::AuditCapture,
    ) {
        let _ = StoreView::for_audit(source);
        let _ = HashReader::captured(capture, source);
    }
}

/// 内部引用所有者；Owned只有固定inline字段，不补造ContentRef/String。
#[derive(Clone)]
pub(crate) enum RefValue {
    Plain(ContentRef),
    Owned(sc::RefOwner),
}
impl RefValue {
    pub fn view(&self) -> sc::RefView<'_> {
        match self {
            Self::Plain(r) => r.into(),
            Self::Owned(r) => r.view(),
        }
    }
    pub fn kind(&self) -> sc::Kind {
        self.view().kind()
    }
    pub fn extent(&self) -> Option<u64> {
        self.view().extent()
    }
    pub fn root(&self) -> &str {
        match self {
            Self::Plain(r) => &r.root,
            Self::Owned(r) => r.root(),
        }
    }
    pub fn digest(&self) -> Result<NodeDigest, CodecError> {
        self.view().digest()
    }
}
impl From<ContentRef> for RefValue {
    fn from(r: ContentRef) -> Self {
        Self::Plain(r)
    }
}
impl From<sc::RefOwner> for RefValue {
    fn from(r: sc::RefOwner) -> Self {
        Self::Owned(r)
    }
}
impl PartialEq for RefValue {
    fn eq(&self, o: &Self) -> bool {
        let a = self.view();
        let b = o.view();
        a.format() == b.format()
            && a.root() == b.root()
            && a.kind() == b.kind()
            && a.extent() == b.extent()
    }
}
impl Eq for RefValue {}
impl std::fmt::Debug for RefValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.view();
        f.debug_struct("ContentRef")
            .field("format", &v.format())
            .field("root", &v.root())
            .field("kind", &v.kind())
            .field("extent", &v.extent())
            .finish()
    }
}
impl serde::Serialize for RefValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let v = self.view();
        let mut st = s.serialize_struct("ContentRef", 4)?;
        st.serialize_field("format", v.format())?;
        st.serialize_field("root", v.root())?;
        st.serialize_field("kind", v.kind().name())?;
        match v.extent() {
            Some(mut n) => {
                let mut digits = [0u8; 20];
                let mut start = digits.len();
                loop {
                    start -= 1;
                    digits[start] = b'0' + (n % 10) as u8;
                    n /= 10;
                    if n == 0 {
                        break;
                    }
                }
                let extent =
                    std::str::from_utf8(&digits[start..]).map_err(serde::ser::Error::custom)?;
                st.serialize_field("extent", extent)?;
            }
            None => st.serialize_field("extent", &None::<&str>)?,
        }
        st.end()
    }
}
#[cfg(test)]
mod ref_value_tests {
    use super::*;

    #[test]
    fn ref_value_transport_matches_public_ref_for_all_kinds_and_extents() {
        for kind in sc::Kind::ALL {
            for extent in [None, Some(0), Some(u64::MAX)] {
                let built = sc::Built {
                    digest: [0xad; 32],
                    kind,
                    extent,
                };
                let public_ref = built.content_ref();
                let expected = public_ref.to_transport_json();
                assert_eq!(expected.as_object().unwrap().len(), 4);
                assert_eq!(
                    serde_json::to_value(RefValue::Plain(public_ref)).unwrap(),
                    expected,
                    "normal kind={} extent={extent:?}",
                    kind.name(),
                );
                #[cfg(feature = "s_allocation_json_testonly")]
                assert_eq!(
                    serde_json::to_value(RefValue::Owned(built.owned_ref())).unwrap(),
                    expected,
                    "owned kind={} extent={extent:?}",
                    kind.name(),
                );
            }
        }
    }
}
pub(crate) enum RefFields<'a> {
    Plain(sc::CodecFields),
    Owned(sc::OwnedRefFields<'a>),
}
impl RefFields<'_> {
    pub fn get(&self, key: &str) -> Option<RefValue> {
        match self {
            Self::Plain(f) => f.get(key).cloned().map(RefValue::Plain),
            Self::Owned(f) => f.get(key).copied().map(RefValue::Owned),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Plain(f) => f.len(),
            Self::Owned(f) => f.len(),
        }
    }
}

/// 仅测试：真实capture/overlay/config进入实际HashReader backend，不制作owned成功token。
#[cfg(all(test, feature = "s_allocation_json_testonly"))]
pub(crate) fn with_owned_store_probe<T>(
    capture: &sc::AuditCapture,
    budget: &sc::capture_allocation::OwnedBudget,
    slots: usize,
    bytes: usize,
    action: impl FnOnce(&mut StoreView<'_, '_>) -> T,
) -> T {
    super::history::with_overlay_prototype(
        super::history::OverlayPrototype {
            slots,
            bytes,
            budget: budget.clone(),
            peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
        },
        || {
            let mut reader = super::history::HashReader::captured_for_audit(capture, None)
                .expect("test actual closed capture/overlay initialization");
            action(&mut StoreView::for_audit(&mut reader))
        },
    )
}

impl<'a> From<&'a RefValue> for sc::RefView<'a>{fn from(r:&'a RefValue)->Self{r.view()}}
