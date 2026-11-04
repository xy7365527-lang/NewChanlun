//! #1394 单次封闭普通 audit 的当前 SQL raw 原件；不保存判据结果。
//! 构造能力只由 state 发出，类型没有 tx getter/NodeSink/Clone 或外部材料入口。
use super::{content_store, state::OrdinaryAuditSeal};
use newchan_rust::s_content::{
    self as sc, job::CheckedFrame, CodecError, NodeDigest, NodeRead, NodeSource,
};

use sc::raw_materials::RawMaterials;
pub(super) use sc::raw_materials::Stats;
#[cfg(test)]
pub(super) use sc::raw_materials::CAP;
pub(super) struct OrdinaryAuditMaterials {
    raw: RawMaterials,
}
pub(super) struct AuditMaterialSource<'p, 'tx> {
    reader: content_store::ReadStore<'tx>,
    pool: &'p mut OrdinaryAuditMaterials,
}
// 保守计入 Option owner、两个wrapper、借用Option以及stats导出值；无另一个heap owner。
pub(super) const METADATA_BYTES: usize = std::mem::size_of::<Option<OrdinaryAuditMaterials>>()
    + 2 * std::mem::size_of::<Option<AuditMaterialSource<'static, 'static>>>()
    + 2 * std::mem::size_of::<Option<content_store::ReadStore<'static>>>()
    + std::mem::size_of::<Option<&'static mut OrdinaryAuditMaterials>>()
    + std::mem::size_of::<Stats>()
    + std::mem::size_of::<OrdinaryAuditSeal>()
    + std::mem::size_of::<Option<std::cell::RefCell<OrdinaryAuditMaterials>>>()
    + std::mem::size_of::<Option<std::cell::RefMut<'static, OrdinaryAuditMaterials>>>();
const _: () = assert!(METADATA_BYTES <= 4096);

impl OrdinaryAuditMaterials {
    pub(super) fn new(_seal: OrdinaryAuditSeal) -> Self {
        Self {
            raw: RawMaterials::production(METADATA_BYTES),
        }
    }
    pub(super) fn for_readonly_history(tx: &rusqlite::Transaction<'_>) -> Result<Self, CodecError> {
        if !tx
            .is_readonly(rusqlite::MAIN_DB)
            .map_err(|e| CodecError::new(sc::code::SOURCE, e.to_string()))?
        {
            return Err(CodecError::new(sc::code::SOURCE, "历史原件池需要只读事务"));
        }
        Ok(Self {
            raw: RawMaterials::production(METADATA_BYTES),
        })
    }
    #[cfg(test)]
    pub(super) fn stats(&self) -> Stats {
        self.raw.stats()
    }
    #[cfg(test)]
    fn allocate(cap: usize, entries: usize, body: usize) -> Self {
        Self {
            raw: RawMaterials::with_limits(cap, entries, body, METADATA_BYTES),
        }
    }
    pub(super) fn source<'p, 'tx>(
        &'p mut self,
        reader: content_store::ReadStore<'tx>,
    ) -> AuditMaterialSource<'p, 'tx> {
        AuditMaterialSource { reader, pool: self }
    }
}
impl NodeSource for AuditMaterialSource<'_, '_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        Ok(match self.checked_read(digest)? {
            None => NodeRead::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_parts();
                NodeRead::Found { tag, body }
            }
        })
    }
    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.pool
            .raw
            .read_material(digest, || self.reader.checked_read(digest))
    }

    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, CodecError> {
        self.checked_read(digest)
            .map(|f| f.map(|f| sc::job::ScopedFrame::new(f, None)))
    }
}
#[cfg(test)]
thread_local! {
    pub(super) static LAST_REPORT: std::cell::Cell<Option<Stats>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
impl Drop for OrdinaryAuditMaterials {
    fn drop(&mut self) {
        LAST_REPORT.with(|r| r.set(Some(self.stats())));
    }
}
#[cfg(test)]
#[path = "ordinary_materials_tests.rs"]
mod tests;

pub(super) struct SqliteAuditOwner<'tx> {
    base: sc::sqlite_audit::SqliteAuditBase<'tx>,
}
impl<'tx> SqliteAuditOwner<'tx> {
    pub(super) fn new(tx: &'tx rusqlite::Transaction<'_>, _seal: OrdinaryAuditSeal) -> Self {
        Self {
            base: sc::sqlite_audit::SqliteAuditBase::new(tx, METADATA_BYTES),
        }
    }
    pub(super) fn base(&self) -> &sc::sqlite_audit::SqliteAuditBase<'tx> {
        &self.base
    }
}
#[cfg(test)]
thread_local! {
    pub(super) static LAST_PROOFS: std::cell::Cell<Option<sc::sqlite_audit::ProofStats>> =
        const { std::cell::Cell::new(None) };
}
#[cfg(test)]
impl Drop for SqliteAuditOwner<'_> {
    fn drop(&mut self) {
        LAST_REPORT.with(|r| r.set(Some(self.base.material_stats())));
        LAST_PROOFS.with(|r| r.set(Some(self.base.proof_stats())));
    }
}
