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
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
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
