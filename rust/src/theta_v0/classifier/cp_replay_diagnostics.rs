//! P52 全量增量重放专用的 frontier 只读计数器。
//!
//! 默认关闭；只有诊断 bin 显式 [`enable`] 后，分类器在既有 pop/recompose 与 dirty 依赖门处
//! 累加旁路计数。计数不参与任何分类、交易、订单或风控分支。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpFrontierCounters {
    pub pending_fallbacks: u64,
    pub tail_reinherits: u64,
    pub certificate_clear_recomputes: u64,
}

thread_local! {
    static COUNTERS: RefCell<Vec<CpFrontierCounters>> = const { RefCell::new(Vec::new()) };
}

pub fn enable() {
    COUNTERS.with(|counters| counters.borrow_mut().clear());
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
    COUNTERS.with(|counters| counters.borrow_mut().clear());
}

pub fn snapshot() -> Vec<CpFrontierCounters> {
    if !ENABLED.load(Ordering::Relaxed) {
        return Vec::new();
    }
    COUNTERS.with(|counters| counters.borrow().clone())
}

fn with_level(level: usize, f: impl FnOnce(&mut CpFrontierCounters)) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    COUNTERS.with(|counters| {
        let mut counters = counters.borrow_mut();
        if counters.len() <= level {
            counters.resize(level + 1, CpFrontierCounters::default());
        }
        f(&mut counters[level]);
    });
}

pub(crate) fn record_dirty_invalidation(
    level: usize,
    pending_fallbacks: u64,
    certificate_clear_recomputes: u64,
) {
    with_level(level, |counter| {
        counter.pending_fallbacks += pending_fallbacks;
        counter.certificate_clear_recomputes += certificate_clear_recomputes;
    });
}

pub(crate) fn record_tail_reinherit(level: usize) {
    with_level(level, |counter| counter.tail_reinherits += 1);
}
