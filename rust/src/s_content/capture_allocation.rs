//! #1394 局部原型：同OwnedBudget计量capture/固定overlay、codec节点body与选定全部工作集真实Layout。
//! 未覆盖业务JSON/其它业务容器、ContentRef字符串/外部输出、SQLite、外层错误/诊断、allocator元数据与物理RSS。
//! 普通NodeSource及擦除后的公开Vec契约保持；固定Arc/Mutex仪表不算payload，无生产默认额度/完整准入。

use super::{CodecError, NodeDigest};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, MutexGuard};

pub const LIMIT: &str = "prototype_capture_owned_payload_limit";
pub const INCOMPLETE: &str = "prototype_audit_allocation_incomplete";

#[derive(Clone)]
pub struct OwnedBudget(Arc<Mutex<Meter>>);
struct Meter {
    limit: usize,
    live: usize,
    peak: usize,
    attempts: u64,
    allocated: u64,
    released: u64,
    rejected: u64,
    copied_rows: u64,
    copied_bytes: u64,
    request_size: usize,
    request_align: usize,
    request_kind: &'static str,
    refused_size: usize,
    refused_align: usize,
    refused_live: usize,
    refused_kind: &'static str,
    broken: bool,
    sqlite_live: usize,
    sqlite_payload_live: usize,
    sqlite_peak: usize,
    sqlite_payload_peak: usize,
    sqlite_allocated: u64,
    sqlite_released: u64,
    sqlite_rejected: u64,
    test_ceiling: Option<usize>,
    reject_sqlite_for_testonly: bool,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Snapshot {
    pub limit_bytes: usize,
    pub live_requested_layout_bytes: usize,
    pub peak_requested_layout_bytes: usize,
    pub allocation_attempts: u64,
    pub allocated_layouts: u64,
    pub released_layouts: u64,
    pub rejected_layouts: u64,
    pub copied_rows: u64,
    pub copied_body_bytes: u64,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct LedgerHealth {
    pub invariant_broken: bool,
    pub sqlite_live_layout_bytes: usize,
    pub sqlite_live_payload_bytes: usize,
    pub sqlite_peak_layout_bytes: usize,
    pub sqlite_peak_payload_bytes: usize,
    pub sqlite_allocated_layouts: u64,
    pub sqlite_released_layouts: u64,
    pub sqlite_rejected_layouts: u64,
}
fn refused() -> CodecError {
    CodecError::static_detail(LIMIT, "capture requested Layout 超过显式测试额度")
}
impl OwnedBudget {
    pub fn new(limit: usize) -> Self {
        let budget = Self(Arc::new(Mutex::new(Meter {
            limit,
            live: 0,
            peak: 0,
            attempts: 0,
            allocated: 0,
            released: 0,
            rejected: 0,
            copied_rows: 0,
            copied_bytes: 0,
            request_size: 0,
            request_align: 1,
            request_kind: "none",
            refused_size: 0,
            refused_align: 1,
            refused_live: 0,
            refused_kind: "none",
            broken: false,
            sqlite_live: 0,
            sqlite_payload_live: 0,
            sqlite_peak: 0,
            sqlite_payload_peak: 0,
            sqlite_allocated: 0,
            sqlite_released: 0,
            sqlite_rejected: 0,
            test_ceiling: None,
            reject_sqlite_for_testonly: false,
        })));
        // #1394: initialize the fixed accounting instrument before any metered operation
        // or SQLite callback. macOS may allocate its first pthread mutex backing here.
        drop(Self::lock(&budget.0));
        budget
    }
    // Recovery never clears poisoned accounting. All later reservations refuse.
    fn lock(core: &Mutex<Meter>) -> MutexGuard<'_, Meter> {
        match core.lock() {
            Ok(m) => m,
            Err(e) => {
                let mut m = e.into_inner();
                m.broken = true;
                m
            }
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let m = Self::lock(&self.0);
        Snapshot {
            limit_bytes: m.limit,
            live_requested_layout_bytes: m.live,
            peak_requested_layout_bytes: m.peak,
            allocation_attempts: m.attempts,
            allocated_layouts: m.allocated,
            released_layouts: m.released,
            rejected_layouts: m.rejected,
            copied_rows: m.copied_rows,
            copied_body_bytes: m.copied_bytes,
        }
    }
    /// #1394：只比较同一实际计量核心；数值相同的两个预算不是同一账本。
    #[doc(hidden)]
    pub fn same_ledger(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    #[doc(hidden)]
    pub fn ledger_health(&self) -> LedgerHealth {
        let m = Self::lock(&self.0);
        LedgerHealth {
            invariant_broken: m.broken,
            sqlite_live_layout_bytes: m.sqlite_live,
            sqlite_live_payload_bytes: m.sqlite_payload_live,
            sqlite_peak_layout_bytes: m.sqlite_peak,
            sqlite_peak_payload_bytes: m.sqlite_payload_peak,
            sqlite_allocated_layouts: m.sqlite_allocated,
            sqlite_released_layouts: m.sqlite_released,
            sqlite_rejected_layouts: m.sqlite_rejected,
        }
    }
    #[doc(hidden)]
    pub fn allocation_request_probe(&self) -> RequestProbe {
        let m = Self::lock(&self.0);
        RequestProbe {
            last_request_bytes: m.request_size,
            last_request_align: m.request_align,
            last_request_kind: m.request_kind,
            refused_bytes: m.refused_size,
            refused_align: m.refused_align,
            live_before_refusal: m.refused_live,
            refused_kind: m.refused_kind,
        }
    }
    // Aggregate capacity check is only an early diagnostic; it grants no allocation permit.
    pub fn preflight(&self, bytes: usize) -> Result<(), CodecError> {
        let mut m = Self::lock(&self.0);
        if m.broken
            || m.live
                .checked_add(bytes)
                .is_none_or(|n| n > m.limit.min(m.test_ceiling.unwrap_or(m.limit)))
        {
            Self::reject(&mut m, bytes, 1, "aggregate preflight", false);
            return Err(refused());
        }
        Ok(())
    }
    fn bump(counter: &mut u64) -> bool {
        match counter.checked_add(1) {
            Some(n) => {
                *counter = n;
                true
            }
            None => false,
        }
    }
    fn reject(m: &mut Meter, bytes: usize, align: usize, kind: &'static str, sqlite: bool) {
        if !Self::bump(&mut m.rejected) {
            m.broken = true;
        }
        if sqlite && !Self::bump(&mut m.sqlite_rejected) {
            m.broken = true;
        }
        m.refused_size = bytes;
        m.refused_align = align;
        m.refused_live = m.live;
        m.refused_kind = kind;
    }
    fn reserve_on(
        core: &Mutex<Meter>,
        layout: Layout,
        kind: &'static str,
        sqlite_payload: Option<usize>,
    ) -> bool {
        let mut m = Self::lock(core);
        m.request_size = layout.size();
        m.request_align = layout.align();
        m.request_kind = kind;
        let next = m.live.checked_add(layout.size());
        let sql_next = m.sqlite_live.checked_add(if sqlite_payload.is_some() {
            layout.size()
        } else {
            0
        });
        let payload_next = m
            .sqlite_payload_live
            .checked_add(sqlite_payload.unwrap_or(0));
        if m.broken
            || (sqlite_payload.is_some() && m.reject_sqlite_for_testonly)
            || next.is_none_or(|n| n > m.limit.min(m.test_ceiling.unwrap_or(m.limit)))
            || sql_next.is_none()
            || payload_next.is_none()
        {
            Self::reject(
                &mut m,
                layout.size(),
                layout.align(),
                kind,
                sqlite_payload.is_some(),
            );
            return false;
        }
        if layout.size() == 0 {
            return true;
        }
        if !Self::bump(&mut m.attempts) {
            m.broken = true;
            return false;
        }
        // checked above; unwrap_or does not panic even after an invariant defect.
        m.live = next.unwrap_or(m.live);
        m.peak = m.peak.max(m.live);
        if sqlite_payload.is_some() {
            m.sqlite_live = sql_next.unwrap_or(m.sqlite_live);
            m.sqlite_payload_live = payload_next.unwrap_or(m.sqlite_payload_live);
            m.sqlite_peak = m.sqlite_peak.max(m.sqlite_live);
            m.sqlite_payload_peak = m.sqlite_payload_peak.max(m.sqlite_payload_live);
        }
        true
    }
    pub(crate) fn reserve_layout(
        &self,
        layout: Layout,
        kind: &'static str,
    ) -> Result<(), CodecError> {
        if Self::reserve_on(&self.0, layout, kind, None) {
            Ok(())
        } else {
            Err(refused())
        }
    }
    fn commit_on(core: &Mutex<Meter>, sqlite: bool) {
        let mut m = Self::lock(core);
        if !Self::bump(&mut m.allocated) {
            m.broken = true;
        }
        if sqlite && !Self::bump(&mut m.sqlite_allocated) {
            m.broken = true;
        }
    }
    pub(crate) fn commit_layout(&self, layout: Layout) {
        if layout.size() != 0 {
            Self::commit_on(&self.0, false);
        }
    }
    fn release_on(
        core: &Mutex<Meter>,
        layout: Layout,
        sqlite_payload: Option<usize>,
        committed: bool,
    ) {
        if layout.size() == 0 {
            return;
        }
        let mut m = Self::lock(core);
        // A broken subtraction preserves the conservative live count rather than zeroing it.
        let next = m.live.checked_sub(layout.size());
        let sql_next = m.sqlite_live.checked_sub(if sqlite_payload.is_some() {
            layout.size()
        } else {
            0
        });
        let payload_next = m
            .sqlite_payload_live
            .checked_sub(sqlite_payload.unwrap_or(0));
        if next.is_none() || sql_next.is_none() || payload_next.is_none() {
            m.broken = true;
            return;
        }
        m.live = next.unwrap_or(m.live);
        m.sqlite_live = sql_next.unwrap_or(m.sqlite_live);
        m.sqlite_payload_live = payload_next.unwrap_or(m.sqlite_payload_live);
        if committed {
            if !Self::bump(&mut m.released) {
                m.broken = true;
            }
            if sqlite_payload.is_some() && !Self::bump(&mut m.sqlite_released) {
                m.broken = true;
            }
        }
    }
    pub(crate) fn rollback_layout(&self, layout: Layout) {
        Self::release_on(&self.0, layout, None, false);
    }
    pub(crate) fn release_layout(&self, layout: Layout) {
        Self::release_on(&self.0, layout, None, true);
    }
    pub(crate) fn note_copy(&self, bytes: usize) {
        let mut m = Self::lock(&self.0);
        if !Self::bump(&mut m.copied_rows) {
            m.broken = true;
        }
        match m.copied_bytes.checked_add(bytes as u64) {
            Some(n) => m.copied_bytes = n,
            None => m.broken = true,
        }
    }
}

struct OwnedLayout {
    pointer: NonNull<u8>,
    layout: Layout,
    budget: OwnedBudget,
}
impl OwnedLayout {
    fn new(layout: Layout, budget: &OwnedBudget) -> Result<Self, CodecError> {
        // Each actual allocation has its own BEFORE gate, even after aggregate preflight.
        budget.reserve_layout(layout, "capture/body/overlay Layout")?;
        if layout.size() == 0 {
            return Ok(Self {
                pointer: NonNull::new(layout.align() as *mut u8).unwrap(),
                layout,
                budget: budget.clone(),
            });
        }
        // SAFETY: nonzero valid Layout; the allocation is initialized to zero bytes.
        let pointer = unsafe { alloc_zeroed(layout) };
        let Some(pointer) = NonNull::new(pointer) else {
            budget.rollback_layout(layout);
            return Err(CodecError::static_detail(
                LIMIT,
                "capture 分配器拒绝已预留 Layout",
            ));
        };
        budget.commit_layout(layout);
        Ok(Self {
            pointer,
            layout,
            budget: budget.clone(),
        })
    }
}
impl Drop for OwnedLayout {
    fn drop(&mut self) {
        if self.layout.size() != 0 {
            // SAFETY: sole owner, same nonzero Layout used for allocation.
            unsafe { dealloc(self.pointer.as_ptr(), self.layout) };
            self.budget.release_layout(self.layout);
        }
    }
}

#[derive(Clone, Copy)]
struct IndexLayout {
    layout: Layout,
    offsets_at: usize,
    tags_at: usize,
}
impl IndexLayout {
    fn new(count: usize) -> Result<Self, CodecError> {
        let digests = Layout::array::<NodeDigest>(count).map_err(|_| refused())?;
        let offsets = Layout::array::<usize>(count).map_err(|_| refused())?;
        let tags = Layout::array::<u8>(count).map_err(|_| refused())?;
        // extend checks column arithmetic and alignment; final padding is charged too.
        let (layout, offsets_at) = digests.extend(offsets).map_err(|_| refused())?;
        let (layout, tags_at) = layout.extend(tags).map_err(|_| refused())?;
        Ok(Self {
            layout: layout.pad_to_align(),
            offsets_at,
            tags_at,
        })
    }
}

struct CaptureIndex {
    block: OwnedLayout,
    columns: IndexLayout,
    count: usize,
}
impl CaptureIndex {
    fn new(count: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        let columns = IndexLayout::new(count)?;
        Ok(Self {
            block: OwnedLayout::new(columns.layout, budget)?,
            columns,
            count,
        })
    }
    fn digests(&self, used: usize) -> &[NodeDigest] {
        assert!(used <= self.count);
        // SAFETY: the checked Layout holds count aligned digests at offset zero. All
        // digest bytes are initialized by alloc_zeroed; the aligned zero-size pointer
        // also supports an empty slice. Only the published prefix is exposed.
        unsafe { std::slice::from_raw_parts(self.block.pointer.as_ptr().cast(), used) }
    }
    fn offset(&self, row: usize) -> usize {
        assert!(row < self.count);
        // SAFETY: extend places count usizes at this aligned, checked column offset;
        // row is in bounds and alloc_zeroed initializes every usize before any read.
        unsafe {
            self.block
                .pointer
                .as_ptr()
                .add(self.columns.offsets_at)
                .cast::<usize>()
                .add(row)
                .read()
        }
    }
    fn tag(&self, row: usize) -> u8 {
        assert!(row < self.count);
        // SAFETY: checked column Layout holds count initialized u8s; row is in bounds.
        unsafe {
            self.block
                .pointer
                .as_ptr()
                .add(self.columns.tags_at)
                .add(row)
                .read()
        }
    }
    fn write(&mut self, row: usize, digest: NodeDigest, offset: usize, tag: u8) {
        assert!(row < self.count);
        // SAFETY: row is within all three checked, aligned columns. This private block
        // is exclusively borrowed; writes initialize the row before used is advanced.
        unsafe {
            self.block
                .pointer
                .as_ptr()
                .cast::<NodeDigest>()
                .add(row)
                .write(digest);
            self.block
                .pointer
                .as_ptr()
                .add(self.columns.offsets_at)
                .cast::<usize>()
                .add(row)
                .write(offset);
            self.block
                .pointer
                .as_ptr()
                .add(self.columns.tags_at)
                .add(row)
                .write(tag);
        }
    }
}

/// Fixed index and body arena. No BTree/HashMap allocation or capacity rounding.
pub struct CaptureSlab {
    index: CaptureIndex,
    body: OwnedLayout,
    count: usize,
    used: usize,
    body_used: usize,
}
impl CaptureSlab {
    pub fn requested_bytes(count: usize, bodies: usize) -> Result<usize, CodecError> {
        let index = IndexLayout::new(count)?;
        Layout::array::<u8>(bodies).map_err(|_| refused())?;
        index.layout.size().checked_add(bodies).ok_or_else(refused)
    }
    pub fn new(count: usize, bodies: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        budget.preflight(Self::requested_bytes(count, bodies)?)?;
        Ok(Self {
            index: CaptureIndex::new(count, budget)?,
            body: OwnedLayout::new(Layout::array::<u8>(bodies).map_err(|_| refused())?, budget)?,
            count,
            used: 0,
            body_used: 0,
        })
    }
    pub fn push(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        let end = self.body_used.checked_add(body.len()).ok_or_else(refused)?;
        if self.used == self.count || end > self.body.layout.size() {
            return Err(refused());
        }
        // Caller provides digest order. This also rejects duplicates without allocating a set.
        if self
            .index
            .digests(self.used)
            .last()
            .is_some_and(|last| *last >= digest)
        {
            return Err(CodecError::new(
                super::code::SOURCE,
                "capture 摘要顺序或唯一性变化",
            ));
        }
        // SAFETY: bounds checked above; caller bytes do not alias these privately owned arrays.
        unsafe {
            std::ptr::copy_nonoverlapping(
                body.as_ptr(),
                self.body.pointer.as_ptr().add(self.body_used),
                body.len(),
            );
        }
        self.index.write(self.used, digest, self.body_used, tag);
        self.used += 1;
        self.body_used = end;
        self.body.budget.note_copy(body.len());
        Ok(())
    }
    pub fn finish(self) -> Result<Self, CodecError> {
        if self.used != self.count || self.body_used != self.body.layout.size() {
            return Err(CodecError::new(
                super::code::SOURCE,
                "capture 两遍读取的节点数或字节数变化",
            ));
        }
        Ok(self)
    }
    pub fn original(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        let i = self.index.digests(self.used).binary_search(digest).ok()?;
        Some((self.index.tag(i), self.body_at(i)))
    }
    fn body_at(&self, row: usize) -> &[u8] {
        assert!(row < self.used);
        let start = self.index.offset(row);
        let end = if row + 1 < self.used {
            self.index.offset(row + 1)
        } else {
            self.body_used
        };
        // SAFETY: only successful push publishes rows, with nondecreasing offsets
        // bounded by body_used <= arena size. The last published row ends at body_used,
        // including unfinished slabs and empty bodies. Shared self prevents mutation.
        unsafe { std::slice::from_raw_parts(self.body.pointer.as_ptr().add(start), end - start) }
    }
    pub fn try_clone(&self) -> Result<Self, CodecError> {
        let mut clone = Self::new(self.count, self.body_used, &self.body.budget)?;
        for (row, digest) in self.index.digests(self.used).iter().enumerate() {
            clone.push(*digest, self.index.tag(row), self.body_at(row))?;
        }
        clone.finish()
    }
    pub fn len(&self) -> usize {
        self.used
    }
    pub fn body_bytes(&self) -> usize {
        self.body_used
    }
    pub(crate) fn retained_layout_bytes(&self) -> usize {
        self.index.block.layout.size() + self.body.layout.size()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.body.budget.snapshot()
    }
    /// This prototype MUST NOT be treated as an admission ticket for complete audit heap.
    pub fn require_complete_audit_coverage(&self) -> Result<(), CodecError> {
        Err(CodecError::new(
            INCOMPLETE,
            "Verifier/memo/steps/aggs、NodeRead clone、overlay 未纳入写前界",
        ))
    }
}

#[doc(hidden)]
pub mod overlay;

// #1394 有界键比较局部原型。此类型不提供 Vec 逃逸或隐式 clone。
pub(crate) mod body;
impl CaptureSlab {
    pub(super) fn copy_body(&self, bytes: &[u8]) -> Result<body::MeteredBody, CodecError> {
        body::MeteredBody::copy_from(bytes, &self.body.budget)
    }
}

impl CaptureSlab {
    pub(super) fn owned_budget(&self) -> OwnedBudget {
        self.body.budget.clone()
    }
}

pub(crate) mod work;

/// #1394 caller-owned dispatch allocations; no global allocator installation.
#[doc(hidden)]
pub mod dispatch;

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct RequestProbe {
    pub last_request_bytes: usize,
    pub last_request_align: usize,
    pub last_request_kind: &'static str,
    pub refused_bytes: usize,
    pub refused_align: usize,
    pub live_before_refusal: usize,
    pub refused_kind: &'static str,
}

#[cfg(feature = "s_allocation_sqlite_testonly")]
#[doc(hidden)]
pub mod sqlite;

#[doc(hidden)]
pub mod audit_json;

#[cfg(test)]
mod capture_index_tests {
    use super::*;

    fn expected_layout(count: usize) -> Layout {
        let digests = Layout::array::<NodeDigest>(count).unwrap();
        let offsets = Layout::array::<usize>(count).unwrap();
        let tags = Layout::array::<u8>(count).unwrap();
        digests
            .extend(offsets)
            .unwrap()
            .0
            .extend(tags)
            .unwrap()
            .0
            .pad_to_align()
    }

    #[test]
    fn actual_index_layout_includes_columns_alignment_and_padding() {
        for count in [0, 1, 2, 7, 8, 9, 104_465] {
            let expected = expected_layout(count);
            let columns = IndexLayout::new(count).unwrap();
            assert_eq!(columns.layout, expected);
            assert_eq!(columns.offsets_at % std::mem::align_of::<usize>(), 0);
            assert_eq!(columns.tags_at, count * (32 + std::mem::size_of::<usize>()));
            assert_eq!(
                CaptureSlab::requested_bytes(count, 3).unwrap(),
                expected.size() + 3
            );
        }
        let budget = OwnedBudget::new(CaptureSlab::requested_bytes(3, 2).unwrap());
        let slab = CaptureSlab::new(3, 2, &budget).unwrap();
        assert_eq!(slab.index.block.layout, expected_layout(3));
        assert_eq!(
            slab.retained_layout_bytes(),
            budget.snapshot().live_requested_layout_bytes
        );
        assert_eq!(budget.snapshot().allocated_layouts, 2);
        drop(slab);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn observed_count_layout_saving_is_only_a_local_test_case() {
        // #1394 observed fixture dimensions, not a service quota or whole-audit admission.
        let count = 104_465;
        let index = IndexLayout::new(count).unwrap().layout;
        assert_eq!(index.size(), 4_283_072);
        assert_eq!(index.size() - count * 41, 7);
        assert_eq!(5_850_040 - index.size(), 1_566_968);
        assert_eq!(
            CaptureSlab::requested_bytes(count, 6_463_541).unwrap(),
            10_746_613
        );
    }

    #[test]
    fn zero_layout_and_overflow_do_not_allocate() {
        let budget = OwnedBudget::new(0);
        let empty = CaptureSlab::new(0, 0, &budget).unwrap().finish().unwrap();
        assert_eq!(
            empty.index.block.layout.align(),
            std::mem::align_of::<usize>()
        );
        assert_eq!(empty.original(&[0; 32]), None);
        let clone = empty.try_clone().unwrap();
        assert_eq!(clone.len(), 0);
        assert_eq!(budget.snapshot().allocation_attempts, 0);
        assert_eq!(budget.snapshot().allocated_layouts, 0);
        assert!(CaptureSlab::new(usize::MAX, 0, &budget).is_err());
        assert!(CaptureSlab::requested_bytes(0, usize::MAX).is_err());
        let count = isize::MAX as usize / std::mem::size_of::<NodeDigest>();
        assert!(Layout::array::<NodeDigest>(count).is_ok());
        assert!(IndexLayout::new(count).is_err());
        assert_eq!(budget.snapshot().allocation_attempts, 0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn published_prefix_and_consecutive_empty_bodies_remain_readable() {
        let bytes = CaptureSlab::requested_bytes(5, 3).unwrap();
        let budget = OwnedBudget::new(2 * bytes);
        let mut slab = CaptureSlab::new(5, 3, &budget).unwrap();
        assert_eq!(slab.original(&[0; 32]), None);
        slab.push([1; 32], 11, b"").unwrap();
        slab.push([2; 32], 12, b"").unwrap();
        slab.push([3; 32], 13, b"abc").unwrap();
        assert_eq!(slab.original(&[1; 32]), Some((11, &b""[..])));
        assert_eq!(slab.original(&[2; 32]), Some((12, &b""[..])));
        assert_eq!(slab.original(&[3; 32]), Some((13, &b"abc"[..])));
        assert_eq!(slab.original(&[0; 32]), None);
        assert_eq!(slab.original(&[4; 32]), None);
        let before = budget.snapshot();
        assert_eq!(
            slab.try_clone().err().unwrap().code(),
            super::super::code::SOURCE
        );
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        assert_eq!(budget.snapshot().peak_requested_layout_bytes, 2 * bytes);
        assert_eq!(
            budget.snapshot().allocated_layouts - before.allocated_layouts,
            2
        );
        assert_eq!(
            budget.snapshot().released_layouts - before.released_layouts,
            2
        );
        assert_eq!(slab.len(), 3);
        slab.push([4; 32], 14, b"").unwrap();
        slab.push([5; 32], 15, b"").unwrap();
        let slab = slab.finish().unwrap();
        assert_eq!(slab.original(&[3; 32]), Some((13, &b"abc"[..])));
        assert_eq!(slab.original(&[4; 32]), Some((14, &b""[..])));
        assert_eq!(slab.original(&[5; 32]), Some((15, &b""[..])));
        drop(slab);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn rejected_push_preserves_rows_body_and_error_precedence() {
        let budget = OwnedBudget::new(CaptureSlab::requested_bytes(2, 2).unwrap());
        let mut slab = CaptureSlab::new(2, 2, &budget).unwrap();
        slab.push([2; 32], 7, b"a").unwrap();
        let before = budget.snapshot();
        for digest in [[1; 32], [2; 32]] {
            assert_eq!(
                slab.push(digest, 8, b"b").unwrap_err().code(),
                super::super::code::SOURCE
            );
            assert_eq!((slab.used, slab.body_used), (1, 1));
            assert_eq!(slab.original(&[2; 32]), Some((7, &b"a"[..])));
        }
        // Arena/count refusal precedes a malformed digest, as in the original slab.
        assert_eq!(slab.push([1; 32], 8, b"bc").unwrap_err().code(), LIMIT);
        assert_eq!((slab.used, slab.body_used), (1, 1));
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        assert_eq!(
            budget.snapshot().copied_body_bytes,
            before.copied_body_bytes
        );
        slab.push([3; 32], 9, b"b").unwrap();
        assert_eq!(slab.push([1; 32], 8, b"").unwrap_err().code(), LIMIT);
        assert_eq!((slab.used, slab.body_used), (2, 2));
        assert_eq!(slab.original(&[3; 32]), Some((9, &b"b"[..])));
        drop(slab.finish().unwrap());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn finish_requires_exact_count_and_body_bytes() {
        for (count, bodies, push) in [(2, 1, true), (1, 2, true), (0, 1, false)] {
            let budget = OwnedBudget::new(CaptureSlab::requested_bytes(count, bodies).unwrap());
            let mut slab = CaptureSlab::new(count, bodies, &budget).unwrap();
            if push {
                slab.push([1; 32], 0, b"a").unwrap();
            }
            assert_eq!(
                slab.finish().err().unwrap().code(),
                super::super::code::SOURCE
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }

    #[test]
    fn exact_clone_keeps_both_owners_charged_and_refusal_keeps_original() {
        let bytes = CaptureSlab::requested_bytes(3, 2).unwrap();
        for cap in [2 * bytes - 1, 2 * bytes] {
            let budget = OwnedBudget::new(cap);
            let mut slab = CaptureSlab::new(3, 2, &budget).unwrap();
            slab.push([1; 32], 1, b"").unwrap();
            slab.push([2; 32], 2, b"ab").unwrap();
            slab.push([3; 32], 3, b"").unwrap();
            let slab = slab.finish().unwrap();
            let before = budget.snapshot();
            if cap < 2 * bytes {
                assert_eq!(slab.try_clone().err().unwrap().code(), LIMIT);
                assert_eq!(
                    budget.snapshot().allocated_layouts,
                    before.allocated_layouts
                );
                assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
            } else {
                let clone = slab.try_clone().unwrap();
                assert_eq!(clone.retained_layout_bytes(), bytes);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 2 * bytes);
                assert_eq!(budget.snapshot().peak_requested_layout_bytes, 2 * bytes);
                for digest in [[1; 32], [2; 32], [3; 32]] {
                    assert_eq!(slab.original(&digest), clone.original(&digest));
                }
                drop(clone);
            }
            assert_eq!(slab.original(&[2; 32]), Some((2, &b"ab"[..])));
            assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
            drop(slab);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }

    #[test]
    fn later_real_allocation_gate_failure_releases_partial_slab() {
        let bytes = CaptureSlab::requested_bytes(1, 1).unwrap();
        let budget = OwnedBudget::new(bytes);
        // Existing counter-overflow gate deterministically refuses the second actual
        // allocation after aggregate preflight and successful index allocation.
        OwnedBudget::lock(&budget.0).attempts = u64::MAX - 1;
        assert_eq!(CaptureSlab::new(1, 1, &budget).err().unwrap().code(), LIMIT);
        let snapshot = budget.snapshot();
        assert_eq!(snapshot.allocated_layouts, 1);
        assert_eq!(snapshot.released_layouts, 1);
        assert_eq!(
            snapshot.peak_requested_layout_bytes,
            expected_layout(1).size()
        );
        assert_eq!(snapshot.live_requested_layout_bytes, 0);
        assert!(budget.ledger_health().invariant_broken);
    }

    #[test]
    fn clone_later_gate_failure_releases_only_new_index() {
        let bytes = CaptureSlab::requested_bytes(1, 1).unwrap();
        let budget = OwnedBudget::new(2 * bytes);
        let mut slab = CaptureSlab::new(1, 1, &budget).unwrap();
        slab.push([1; 32], 7, b"a").unwrap();
        let slab = slab.finish().unwrap();
        let before = budget.snapshot();
        OwnedBudget::lock(&budget.0).attempts = u64::MAX - 1;
        assert_eq!(slab.try_clone().err().unwrap().code(), LIMIT);
        let after = budget.snapshot();
        assert_eq!(after.allocated_layouts - before.allocated_layouts, 1);
        assert_eq!(after.released_layouts - before.released_layouts, 1);
        assert_eq!(after.live_requested_layout_bytes, bytes);
        assert_eq!(
            after.peak_requested_layout_bytes,
            bytes + expected_layout(1).size()
        );
        assert_eq!(after.copied_rows, before.copied_rows);
        assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
        assert_eq!(slab.original(&[1; 32]), Some((7, &b"a"[..])));
        drop(slab);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(test)]
mod shared_core_tests {
    use super::*;
    #[test]
    fn shared_reservation_is_atomic_and_poison_is_conservative() {
        let budget = OwnedBudget::new(100);
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let b = budget.clone();
            let gate = barrier.clone();
            handles.push(std::thread::spawn(move || {
                let block = OwnedLayout::new(Layout::from_size_align(64, 8).unwrap(), &b).ok();
                gate.wait();
                gate.wait();
                drop(block);
            }));
        }
        barrier.wait();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 64);
        assert_eq!(budget.snapshot().allocated_layouts, 1);
        assert_eq!(budget.snapshot().rejected_layouts, 7);
        barrier.wait();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let block = OwnedLayout::new(Layout::from_size_align(64, 8).unwrap(), &budget).unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _locked = budget.0.lock().unwrap();
            panic!("test poison");
        }));
        assert!(budget.ledger_health().invariant_broken);
        assert!(OwnedLayout::new(Layout::from_size_align(1, 1).unwrap(), &budget).is_err());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 64);
        drop(block);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(budget.ledger_health().invariant_broken);
    }
}
