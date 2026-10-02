//! #1394 封闭审计读取边界。动态来源没有捕获能力；唯有具体 HashReader 自己选择
//! 与实际读取绑定的不可变 owner。两种路径都用同一业务函数，不传成功 token。
use super::history::HashReader;
use newchan_rust::s_content::{
    self as sc, CodecError, ContentRef, NodeDigest, NodeRead, NodeSink, NodeSource, NodeStore,
};

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
