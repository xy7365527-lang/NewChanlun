//! #1394 局部测试原型：只限制 capture 的两次 requested Layout 分配。
//! 不覆盖 Verifier、NodeRead clone、overlay、SQLite、分配器元数据或 RSS。
//! 共享计数器 Rc 是固定测试仪表，不属于 capture payload；没有生产默认额度。

use super::{CodecError, NodeDigest};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::cell::Cell;
use std::ptr::NonNull;
use std::rc::Rc;

pub const LIMIT: &str = "prototype_capture_owned_payload_limit";
pub const INCOMPLETE: &str = "prototype_audit_allocation_incomplete";

#[derive(Clone)]
pub struct OwnedBudget(Rc<Meter>);
struct Meter {
    limit: usize,
    live: Cell<usize>,
    peak: Cell<usize>,
    attempts: Cell<u64>,
    allocated: Cell<u64>,
    released: Cell<u64>,
    rejected: Cell<u64>,
    copied_rows: Cell<u64>,
    copied_bytes: Cell<u64>,
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
fn refused() -> CodecError {
    CodecError::new(LIMIT, "capture requested Layout 超过显式测试额度")
}
impl OwnedBudget {
    pub fn new(limit: usize) -> Self {
        Self(Rc::new(Meter {
            limit,
            live: Cell::new(0),
            peak: Cell::new(0),
            attempts: Cell::new(0),
            allocated: Cell::new(0),
            released: Cell::new(0),
            rejected: Cell::new(0),
            copied_rows: Cell::new(0),
            copied_bytes: Cell::new(0),
        }))
    }
    pub fn snapshot(&self) -> Snapshot {
        let m = &self.0;
        Snapshot {
            limit_bytes: m.limit,
            live_requested_layout_bytes: m.live.get(),
            peak_requested_layout_bytes: m.peak.get(),
            allocation_attempts: m.attempts.get(),
            allocated_layouts: m.allocated.get(),
            released_layouts: m.released.get(),
            rejected_layouts: m.rejected.get(),
            copied_rows: m.copied_rows.get(),
            copied_body_bytes: m.copied_bytes.get(),
        }
    }
    pub fn preflight(&self, bytes: usize) -> Result<(), CodecError> {
        if self
            .0
            .live
            .get()
            .checked_add(bytes)
            .is_none_or(|n| n > self.0.limit)
        {
            self.0.rejected.set(self.0.rejected.get() + 1);
            return Err(refused());
        }
        Ok(())
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
        budget.preflight(layout.size())?;
        let m = &budget.0;
        if layout.size() == 0 {
            return Ok(Self {
                pointer: NonNull::new(layout.align() as *mut u8).unwrap(),
                layout,
                budget: budget.clone(),
            });
        }
        m.attempts.set(m.attempts.get() + 1);
        m.live.set(m.live.get() + layout.size());
        // SAFETY: nonzero valid Layout; zeroed bytes initialize every Record field.
        let pointer = unsafe { alloc_zeroed(layout) };
        let Some(pointer) = NonNull::new(pointer) else {
            m.live.set(m.live.get() - layout.size());
            return Err(CodecError::new(LIMIT, "capture 分配器拒绝已预留 Layout"));
        };
        m.allocated.set(m.allocated.get() + 1);
        m.peak.set(m.peak.get().max(m.live.get()));
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
            let m = &self.budget.0;
            m.live.set(m.live.get() - self.layout.size());
            m.released.set(m.released.get() + 1);
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
        let m = &self.body.budget.0;
        m.copied_rows.set(m.copied_rows.get() + 1);
        m.copied_bytes.set(m.copied_bytes.get() + body.len() as u64);
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
