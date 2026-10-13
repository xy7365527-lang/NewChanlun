//! 正常验收的进程 Rust 分配实测，只编入 Native 测试；不改变额度或分配成败。
//! 单测试进程使用。记录请求 layout 的实时共存值，不声称 allocator 元数据、
//! realloc 内部暂存、SQLite C malloc、线程栈或 RSS 已包括在内。
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static RELEASED: AtomicUsize = AtomicUsize::new(0);
struct ObservedSystem;
#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;
fn add(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
    ALLOCATED.fetch_add(bytes, Relaxed);
    PEAK.fetch_max(live, Relaxed);
}
fn sub(bytes: usize) {
    LIVE.fetch_sub(bytes, Relaxed);
    RELEASED.fetch_add(bytes, Relaxed);
}
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        sub(layout.size());
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, layout, size) };
        if !next.is_null() {
            // 原分配已被 System 终结；只记可观察的前后布局，不把两者伪称同时存活。
            sub(layout.size());
            add(size);
        }
        next
    }
}
pub(crate) struct Window {
    live: usize,
    allocated: usize,
    released: usize,
}
impl Window {
    pub(crate) fn start() -> Self {
        let live = LIVE.load(Relaxed);
        PEAK.store(live, Relaxed);
        Self {
            live,
            allocated: ALLOCATED.load(Relaxed),
            released: RELEASED.load(Relaxed),
        }
    }
    pub(crate) fn finish(self) -> serde_json::Value {
        let live = LIVE.load(Relaxed);
        let peak = PEAK.load(Relaxed);
        let allocated = ALLOCATED.load(Relaxed) - self.allocated;
        let released = RELEASED.load(Relaxed) - self.released;
        serde_json::json!({"scope":"进程Rust请求layout；单测试线程；含旧候选/新effects/派生状态/业务缓存实际共存，不含C malloc与allocator内部",
            "live_before":self.live,"live_after":live,"coexistence_peak":peak,
            "allocated_bytes":allocated,"released_bytes":released})
    }
}
