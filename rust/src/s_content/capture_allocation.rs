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
        Self(Arc::new(Mutex::new(Meter {
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
        })))
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
        // SAFETY: nonzero valid Layout; zeroed bytes initialize every Record field.
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
#[repr(C)]
struct Record {
    digest: NodeDigest,
    offset: usize,
    len: usize,
    tag: u8,
}

/// Fixed index and body arena. No BTree/HashMap allocation or capacity rounding.
pub struct CaptureSlab {
    index: OwnedLayout,
    body: OwnedLayout,
    count: usize,
    used: usize,
    body_used: usize,
}
impl CaptureSlab {
    pub fn requested_bytes(count: usize, bodies: usize) -> Result<usize, CodecError> {
        let index = Layout::array::<Record>(count).map_err(|_| refused())?;
        Layout::array::<u8>(bodies).map_err(|_| refused())?;
        index.size().checked_add(bodies).ok_or_else(refused)
    }
    pub fn new(count: usize, bodies: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        budget.preflight(Self::requested_bytes(count, bodies)?)?;
        Ok(Self {
            index: OwnedLayout::new(
                Layout::array::<Record>(count).map_err(|_| refused())?,
                budget,
            )?,
            body: OwnedLayout::new(Layout::array::<u8>(bodies).map_err(|_| refused())?, budget)?,
            count,
            used: 0,
            body_used: 0,
        })
    }
    fn records(&self) -> &[Record] {
        // SAFETY: index is aligned and zero-initialized for count Records; only used is exposed.
        unsafe { std::slice::from_raw_parts(self.index.pointer.as_ptr().cast(), self.used) }
    }
    pub fn push(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        let end = self.body_used.checked_add(body.len()).ok_or_else(refused)?;
        if self.used == self.count || end > self.body.layout.size() {
            return Err(refused());
        }
        // Caller provides digest order. This also rejects duplicates without allocating a set.
        if self
            .records()
            .last()
            .is_some_and(|last| last.digest >= digest)
        {
            return Err(CodecError::new(
                super::code::SOURCE,
                "capture 摘要顺序或唯一性变化",
            ));
        }
        let record = Record {
            digest,
            tag,
            offset: self.body_used,
            len: body.len(),
        };
        // SAFETY: bounds checked above; caller bytes do not alias these privately owned arrays.
        unsafe {
            std::ptr::copy_nonoverlapping(
                body.as_ptr(),
                self.body.pointer.as_ptr().add(self.body_used),
                body.len(),
            );
            self.index
                .pointer
                .as_ptr()
                .cast::<Record>()
                .add(self.used)
                .write(record);
        }
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
        let i = self
            .records()
            .binary_search_by_key(digest, |r| r.digest)
            .ok()?;
        let r = self.records()[i];
        // SAFETY: only push creates records, after validating arena bounds; no mutation via self.
        let bytes =
            unsafe { std::slice::from_raw_parts(self.body.pointer.as_ptr().add(r.offset), r.len) };
        Some((r.tag, bytes))
    }
    pub fn try_clone(&self) -> Result<Self, CodecError> {
        let mut clone = Self::new(self.count, self.body_used, &self.body.budget)?;
        for r in self.records() {
            clone.push(r.digest, r.tag, self.original(&r.digest).unwrap().1)?;
        }
        clone.finish()
    }
    pub fn len(&self) -> usize {
        self.used
    }
    pub fn body_bytes(&self) -> usize {
        self.body_used
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

#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub mod audit_json;

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
