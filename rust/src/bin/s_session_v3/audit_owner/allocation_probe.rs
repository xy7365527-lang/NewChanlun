//! 正常验收的 Rust 分配实测，只编入 Native 测试；不改变额度或分配成败。
//! 按线程记账：每个线程只累计自己发起的 alloc/dealloc，Window 只读开窗线程的本账，
//! 并发测试线程的分配不会混入读数。被测路径须在开窗线程内同步完成；别的线程替本线程
//! 释放的块记在释放方，本线程的 live 因而可能为负（读数按 i64 报告，不截断）。
//! 记录请求 layout 的实时共存值，不声称 allocator 元数据、realloc 内部暂存、SQLite C malloc、
//! 线程栈或 RSS 已包括在内。
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy)]
struct Ledger {
    live: i64,
    peak: i64,
    allocated: u64,
    released: u64,
}
std::thread_local! {
    // const 初始化且无析构：访问不分配、不注册 TLS 析构器，可在全局分配器内安全读写。
    static LEDGER: Cell<Ledger> = const {
        Cell::new(Ledger { live: 0, peak: 0, allocated: 0, released: 0 })
    };
}
struct ObservedSystem;
#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;
fn add(bytes: usize) {
    let _ = LEDGER.try_with(|ledger| {
        let mut l = ledger.get();
        l.live = l.live.wrapping_add(bytes as i64);
        l.allocated = l.allocated.wrapping_add(bytes as u64);
        l.peak = l.peak.max(l.live);
        ledger.set(l);
    });
}
fn sub(bytes: usize) {
    let _ = LEDGER.try_with(|ledger| {
        let mut l = ledger.get();
        l.live = l.live.wrapping_sub(bytes as i64);
        l.released = l.released.wrapping_add(bytes as u64);
        ledger.set(l);
    });
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
fn ledger() -> Ledger {
    LEDGER.with(Cell::get)
}
/// 开窗线程的本账窗口；不得跨线程移交（`!Send`），读数不含其他线程的分配。
pub(crate) struct Window {
    live: i64,
    allocated: u64,
    released: u64,
    _thread_bound: std::marker::PhantomData<*const ()>,
}
impl Window {
    pub(crate) fn start() -> Self {
        let l = LEDGER.with(|ledger| {
            let mut l = ledger.get();
            l.peak = l.live;
            ledger.set(l);
            l
        });
        Self {
            live: l.live,
            allocated: l.allocated,
            released: l.released,
            _thread_bound: std::marker::PhantomData,
        }
    }
    pub(crate) fn finish(self) -> serde_json::Value {
        let l = ledger();
        let allocated = l.allocated.wrapping_sub(self.allocated);
        let released = l.released.wrapping_sub(self.released);
        serde_json::json!({"scope":"开窗线程本账的Rust请求layout（按线程记账，不含并发测试线程）；含旧候选/新effects/派生状态/业务缓存实际共存，不含C malloc与allocator内部",
            "live_before":self.live,"live_after":l.live,"coexistence_peak":l.peak,
            "allocated_bytes":allocated,"released_bytes":released})
    }
}
