use super::{job::CheckedFrame, CodecError, NodeDigest};
pub const CAP: usize = 32 * 1024 * 1024;
const META_RESERVE: usize = 4096;
const ENTRY_BYTES: usize = 42;
const ENTRIES: usize = 65536;
const BUCKETS: usize = 131072;

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
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
pub struct RawMaterials {
    slab: Vec<u8>,
    layout: Layout,
    stats: Stats,
}
impl RawMaterials {
    pub fn production(metadata: usize) -> Self {
        let slab_len = CAP - META_RESERVE;
        let layout = Layout::checked(
            ENTRIES,
            BUCKETS,
            slab_len - ENTRIES * ENTRY_BYTES - BUCKETS * 4,
        )
        .unwrap();
        Self::allocate_layout(CAP, layout, metadata)
    }
    pub fn with_limits(cap: usize, entries: usize, body: usize, metadata: usize) -> Self {
        Self::allocate_layout(
            cap,
            Layout::checked(entries, entries * 2, body).unwrap(),
            metadata,
        )
    }
    pub fn stats(&self) -> Stats {
        self.stats
    }
    pub fn read_material(
        &mut self,
        digest: &NodeDigest,
        read: impl FnOnce() -> Result<Option<CheckedFrame>, CodecError>,
    ) -> Result<Option<CheckedFrame>, CodecError> {
        self.stats.logical_reads = self.stats.logical_reads.saturating_add(1);
        if let Some(frame) = self.hit(digest)? {
            return Ok(Some(frame));
        }
        self.stats.sql_material_reads = self.stats.sql_material_reads.saturating_add(1);
        let frame = read()?;
        if let Some(frame) = frame.as_ref() {
            self.save(digest, frame);
        }
        Ok(frame)
    }
    fn allocate_layout(cap: usize, layout: Layout, metadata: usize) -> Self {
        let mut this = Self {
            slab: Vec::new(),
            layout,
            stats: Stats {
                retained_capacity_highwater: metadata,
                ..Stats::default()
            },
        };
        if layout.len.checked_add(metadata).is_none_or(|n| n > cap) {
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
            .checked_add(metadata)
            .is_none_or(|n| n > cap)
        {
            this.slab = Vec::new();
            this.stats.disabled = Some("disabled_capacity");
            return this;
        }
        this.slab.resize(layout.len, 0);
        this.stats.slab_len = this.slab.len();
        this.stats.slab_capacity = this.slab.capacity();
        this.stats.retained_capacity_highwater = metadata + this.slab.capacity();
        this
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
#[cfg(test)]
thread_local! { static FAIL_ALLOCATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s_content::{self as sc, tag};
    const METADATA_BYTES: usize = 4096;
    fn pool(entries: usize, body: usize) -> RawMaterials {
        RawMaterials::with_limits(CAP, entries, body, METADATA_BYTES)
    }
    fn read(p: &mut RawMaterials, bytes: &[u8]) -> Result<Option<CheckedFrame>, CodecError> {
        let d = sc::frame(tag::BYTES_CHUNK, bytes);
        p.read_material(&d, || {
            CheckedFrame::checked(&d, tag::BYTES_CHUNK, bytes.to_vec()).map(Some)
        })
    }
    #[test]
    fn cap_zero_small_entries_and_body_do_not_change_values() {
        for (cap, entries, body) in [(0, 1, 8), (CAP, 1, 8), (CAP, 8, 2)] {
            let mut p = RawMaterials::with_limits(cap, entries, body, METADATA_BYTES);
            for b in [b"one", b"two", b"one", b"two"] {
                assert!(read(&mut p, b).unwrap().is_some());
            }
            if cap == 0 {
                assert_eq!(p.stats.sql_material_reads, 4);
                assert_eq!(p.stats.disabled, Some("disabled_cap"));
            } else if entries == 1 {
                assert_eq!(p.stats.saved_entries, 1);
                assert_eq!(p.stats.save_rejected_entries, 0);
                assert_eq!(p.stats.sql_material_reads, 4);
            } else {
                assert_eq!(p.stats.saved_entries, 0);
                assert_eq!(p.stats.save_rejected_bytes, 4);
            }
        }
    }
    #[test]
    fn full_digest_collisions_finite_probe_and_layout_overflow() {
        assert!(Layout::checked(usize::MAX, usize::MAX, usize::MAX).is_none());
        assert!(Layout::checked(2, 4, usize::MAX).is_none());
        assert!(Layout::checked(3, 4, 0).is_none());
        let mut p = pool(8, 256);
        let mut keys = Vec::new();
        for n in 0u32..10000 {
            let body = n.to_le_bytes().to_vec();
            let d = sc::frame(sc::tag::BYTES_CHUNK, &body);
            if u64::from_le_bytes(d[..8].try_into().unwrap()) as usize % 16 == 0 {
                keys.push((d, body));
                if keys.len() == 8 {
                    break;
                }
            }
        }
        assert_eq!(keys.len(), 8);
        for (d, b) in &keys {
            p.save(
                d,
                &CheckedFrame::checked(d, sc::tag::BYTES_CHUNK, b.clone()).unwrap(),
            );
        }
        for (d, _) in &keys[..4] {
            assert!(p.hit(d).unwrap().is_none());
        }
        for (d, b) in &keys[4..] {
            assert_eq!(p.hit(d).unwrap().unwrap().body(), b);
        }
        assert_eq!(p.stats.saved_entries, 4);
        assert_eq!(p.stats.saved_body_bytes, 16);
        assert_eq!(p.stats.max_probes, 4);
        let absent = [0xfe; 32];
        assert!(p.slot(&absent).is_err());
        assert_eq!(p.stats.max_probes, 4);
    }

    #[test]
    fn hit_rechecks_f_without_sql_fallback() {
        let mut p = pool(4, 64);
        read(&mut p, b"abc").unwrap();
        p.slab[p.layout.body_start] ^= 1;
        let before = p.stats.sql_material_reads;
        assert_eq!(
            read(&mut p, b"abc").unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        assert_eq!(p.stats.sql_material_reads, before);
        let mut next = pool(4, 64);
        read(&mut next, b"abc").unwrap();
        assert_eq!(next.stats.sql_material_reads, 1);
    }
    #[test]
    fn exact_production_capacity_accounting() {
        let p = RawMaterials::production(METADATA_BYTES);
        assert!(p.stats.disabled.is_none());
        assert_eq!(p.slab.len(), 33550336);
        assert_eq!(p.layout.body_start, 3276800);
        assert_eq!(p.slab.len() - p.layout.body_start, 30273536);
        assert!(p.stats.retained_capacity_highwater <= CAP);
    }
    #[test]
    fn optional_allocation_failure_is_cold_disabled_not_validation_retry() {
        FAIL_ALLOCATION.with(|v| v.set(true));
        let mut p = pool(4, 64);
        FAIL_ALLOCATION.with(|v| v.set(false));
        assert_eq!(p.stats.disabled, Some("disabled_allocation"));
        assert_eq!(p.slab.capacity(), 0);
        for _ in 0..2 {
            assert!(read(&mut p, b"abc").unwrap().is_some());
        }
        assert_eq!(p.stats.sql_material_reads, 2);
        assert_eq!(p.stats.material_hits, 0);
    }
}
