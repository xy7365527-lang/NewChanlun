//! #1394 单次封闭普通 audit 的当前 SQL raw 原件；不保存判据结果。
//! 构造能力只由 state 发出，类型没有 tx getter/NodeSink/Clone 或外部材料入口。
use super::{content_store, state::OrdinaryAuditSeal};
use newchan_rust::s_content::{
    self as sc, job::CheckedFrame, CodecError, NodeDigest, NodeRead, NodeSource,
};

pub(super) const CAP: usize = 32 * 1024 * 1024;
const META_RESERVE: usize = 4096;
const ENTRY_BYTES: usize = 42;
const ENTRIES: usize = 65536;
const BUCKETS: usize = 131072;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Stats {
    pub logical_reads: u64,
    pub sql_material_reads: u64,
    pub material_hits: u64,
    pub save_rejected_bytes: u64,
    pub save_rejected_entries: u64,
    pub collision_probes: u64,
    pub max_probes: u64,
    pub saved_entries: usize,
    pub saved_body_bytes: usize,
    pub copied_for_save: u64,
    pub copied_for_return: u64,
    pub slab_len: usize,
    pub slab_capacity: usize,
    pub retained_capacity_highwater: usize,
    pub disabled: Option<&'static str>,
}
#[derive(Clone, Copy)]
struct Layout {
    entries: usize,
    body_start: usize,
    len: usize,
}
impl Layout {
    fn checked(entries: usize, buckets: usize, body_bytes: usize) -> Option<Self> {
        if entries == 0 || buckets == 0 || entries > buckets / 2 || entries % entries.min(4) != 0 {
            return None;
        }
        let body_start = entries
            .checked_mul(ENTRY_BYTES)?
            .checked_add(buckets.checked_mul(4)?)?;
        let len = body_start.checked_add(body_bytes)?;
        if len > u32::MAX as usize || entries >= u32::MAX as usize {
            return None;
        }
        Some(Self {
            entries,
            body_start,
            len,
        })
    }
}
/// 本 owner 不携带事务或已核结论；只保存本次 audit 从真实 SQL 取得的原件。
pub(super) struct OrdinaryAuditMaterials {
    slab: Vec<u8>,
    layout: Layout,
    stats: Stats,
}
/// ReadStore 只在封闭 audit 的两个既有构造点移入，不接任意 NodeSource。
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
    + std::mem::size_of::<OrdinaryAuditSeal>();
const _: () = assert!(METADATA_BYTES <= META_RESERVE);

impl OrdinaryAuditMaterials {
    pub(super) fn new(_seal: OrdinaryAuditSeal) -> Self {
        let slab_len = CAP - META_RESERVE;
        let layout = Layout::checked(
            ENTRIES,
            BUCKETS,
            slab_len - ENTRIES * ENTRY_BYTES - BUCKETS * 4,
        )
        .unwrap();
        Self::allocate(CAP, layout)
    }
    fn allocate(cap: usize, layout: Layout) -> Self {
        let mut this = Self {
            slab: Vec::new(),
            layout,
            stats: Stats {
                retained_capacity_highwater: METADATA_BYTES,
                ..Stats::default()
            },
        };
        if layout
            .len
            .checked_add(METADATA_BYTES)
            .is_none_or(|n| n > cap)
        {
            this.stats.disabled = Some("disabled_cap");
            return this;
        }
        #[cfg(test)]
        if FAIL_ALLOCATION.with(std::cell::Cell::get) {
            this.stats.disabled = Some("disabled_allocation");
            return this;
        }
        if this.slab.try_reserve_exact(layout.len).is_err() {
            this.stats.disabled = Some("disabled_allocation");
            return this;
        }
        if this
            .slab
            .capacity()
            .checked_add(METADATA_BYTES)
            .is_none_or(|n| n > cap)
        {
            this.slab = Vec::new();
            this.stats.disabled = Some("disabled_capacity");
            return this;
        }
        this.slab.resize(layout.len, 0);
        this.stats.slab_len = this.slab.len();
        this.stats.slab_capacity = this.slab.capacity();
        this.stats.retained_capacity_highwater = METADATA_BYTES + this.slab.capacity();
        this
    }
    pub(super) fn source<'p, 'tx>(
        &'p mut self,
        reader: content_store::ReadStore<'tx>,
    ) -> AuditMaterialSource<'p, 'tx> {
        AuditMaterialSource { reader, pool: self }
    }
    fn u32_at(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.slab[at..at + 4].try_into().unwrap())
    }
    fn put_u32(&mut self, at: usize, n: u32) {
        self.slab[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn slot(&mut self, digest: &NodeDigest) -> Result<usize, usize> {
        let ways = self.layout.entries.min(4);
        let sets = self.layout.entries / ways;
        let set = u64::from_le_bytes(digest[..8].try_into().unwrap()) as usize % sets;
        let first = set * ways;
        let mut empty = None;
        for way in 0..ways {
            let at = (first + way) * ENTRY_BYTES;
            self.stats.collision_probes = self.stats.collision_probes.saturating_add(1);
            self.stats.max_probes = self.stats.max_probes.max((way + 1) as u64);
            if self.slab[at + 41] == 0 {
                empty.get_or_insert(at);
            } else if &self.slab[at..at + 32] == digest {
                return Ok(at);
            }
        }
        let next = self.u32_at(self.layout.entries * ENTRY_BYTES + set * 4) as usize;
        Err(empty.unwrap_or((first + next % ways) * ENTRY_BYTES))
    }
    fn hit(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        if self.slab.is_empty() {
            return Ok(None);
        }
        let Ok(at) = self.slot(digest) else {
            return Ok(None);
        };
        let offset = self.u32_at(at + 33) as usize;
        let len = self.u32_at(at + 37) as usize;
        let body = self.slab[offset..offset + len].to_vec();
        self.stats.material_hits = self.stats.material_hits.saturating_add(1);
        self.stats.copied_for_return = self.stats.copied_for_return.saturating_add(len as u64);
        // 原件命中不意味着F/role/closure成功：每次重新执行唯一F。
        CheckedFrame::checked(digest, self.slab[at + 32], body).map(Some)
    }
    fn save(&mut self, digest: &NodeDigest, frame: &CheckedFrame) {
        if self.slab.is_empty() {
            return;
        }
        let at = match self.slot(digest) {
            Ok(_) => return,
            Err(at) => at,
        };
        let body_bytes = (self.slab.len() - self.layout.body_start) / self.layout.entries;
        if frame.body().len() > body_bytes {
            self.stats.save_rejected_bytes = self.stats.save_rejected_bytes.saturating_add(1);
            return;
        }
        let offset = self.layout.body_start + (at / ENTRY_BYTES) * body_bytes;
        let end = offset + frame.body().len();
        if self.slab[at + 41] == 1 {
            self.stats.saved_body_bytes -= self.u32_at(at + 37) as usize;
        } else {
            self.stats.saved_entries += 1;
        }
        self.slab[offset..end].copy_from_slice(frame.body());
        self.slab[at..at + 32].copy_from_slice(digest);
        self.slab[at + 32] = frame.tag();
        self.put_u32(at + 33, offset as u32);
        self.put_u32(at + 37, frame.body().len() as u32);
        self.slab[at + 41] = 1;
        let ways = self.layout.entries.min(4);
        let counter = self.layout.entries * ENTRY_BYTES + (at / ENTRY_BYTES / ways) * 4;
        self.put_u32(counter, ((at / ENTRY_BYTES + 1) % ways) as u32);
        self.stats.saved_body_bytes += frame.body().len();
        self.stats.copied_for_save = self
            .stats
            .copied_for_save
            .saturating_add(frame.body().len() as u64);
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
        self.pool.stats.logical_reads = self.pool.stats.logical_reads.saturating_add(1);
        if let Some(frame) = self.pool.hit(digest)? {
            return Ok(Some(frame));
        }
        self.pool.stats.sql_material_reads = self.pool.stats.sql_material_reads.saturating_add(1);
        let frame = self.reader.checked_read(digest)?;
        if let Some(frame) = frame.as_ref() {
            self.pool.save(digest, frame);
        }
        Ok(frame)
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
    static FAIL_ALLOCATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(test)]
impl Drop for OrdinaryAuditMaterials {
    fn drop(&mut self) {
        LAST_REPORT.with(|r| r.set(Some(self.stats)));
    }
}
#[cfg(test)]
#[path = "ordinary_materials_tests.rs"]
mod tests;
