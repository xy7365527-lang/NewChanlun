//! 阶段计时插桩（profile-only，#106 真热点定位）。
//!
//! thread_local 累加器，env `THETA_PROFILE_STAGES=1` 时启用。只测时间不改逻辑（bit-exact 安全）。
//!
//! #748（C4）纯移动自 `classifier/mod.rs`（原内嵌 `pub mod stage_profile`）。

use std::cell::RefCell;
use std::time::{Duration, Instant};

thread_local! {
    static ACC: RefCell<Vec<(&'static str, Duration)>> = const { RefCell::new(Vec::new()) };
    // 跨度累加器（(label, sum, max, count)）——区分 H-detect（跨度随 n 增长）vs H-detect-bounded
    // （跨度 O(1)）。env-gated，未启用时 record_span 直通。
    static SPANS: RefCell<Vec<(&'static str, u64, u64, u64)>> = const { RefCell::new(Vec::new()) };
    static ENABLED: bool = std::env::var(crate::theta_v0::env_registry::THETA_PROFILE_STAGES).is_ok();
}

pub fn enabled() -> bool {
    ENABLED.with(|e| *e)
}

/// 记录一次跨度样本（如 05 续扫的 `units.len() - start_i`）。env 未启用时零开销直通。
pub fn record_span(label: &'static str, v: u64) {
    if !enabled() {
        return;
    }
    SPANS.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(slot) = s.iter_mut().find(|(l, ..)| *l == label) {
            slot.1 += v;
            slot.2 = slot.2.max(v);
            slot.3 += 1;
        } else {
            s.push((label, v, v, 1));
        }
    });
}

pub struct Guard {
    label: &'static str,
    start: Instant,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let d = self.start.elapsed();
        let label = self.label;
        ACC.with(|a| {
            let mut a = a.borrow_mut();
            if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
                slot.1 += d;
            } else {
                a.push((label, d));
            }
        });
    }
}

pub fn stage(label: &'static str) -> Option<Guard> {
    if enabled() {
        Some(Guard {
            label,
            start: Instant::now(),
        })
    } else {
        None
    }
}

/// 计时一个表达式（闭包包裹；env 未启用时零开销直通）。
pub fn time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
    if !enabled() {
        return f();
    }
    let start = Instant::now();
    let r = f();
    let d = start.elapsed();
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
            slot.1 += d;
        } else {
            a.push((label, d));
        }
    });
    r
}

/// 清空累加器（多窗口 profile 隔离，避免跨 n 累计污染）。env 未启用时直通。
pub fn reset() {
    if !enabled() {
        return;
    }
    ACC.with(|a| a.borrow_mut().clear());
    SPANS.with(|s| s.borrow_mut().clear());
}

pub fn dump() {
    if !enabled() {
        return;
    }
    ACC.with(|a| {
        let a = a.borrow();
        eprintln!("=== THETA STAGE PROFILE ===");
        let mut rows: Vec<_> = a.iter().collect();
        rows.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        for (label, d) in rows {
            eprintln!("  {:<36} {:>10.3} ms", label, d.as_secs_f64() * 1000.0);
        }
        eprintln!("===========================");
    });
    SPANS.with(|s| {
        let s = s.borrow();
        if s.is_empty() {
            return;
        }
        eprintln!("=== THETA STAGE SPANS ===");
        for (label, sum, max, count) in s.iter() {
            let avg = if *count > 0 {
                *sum as f64 / *count as f64
            } else {
                0.0
            };
            eprintln!(
                "  {:<24} sum={:>14} max={:>10} count={:>10} avg={:>12.2}",
                label, sum, max, count, avg
            );
        }
        eprintln!("=========================");
    });
}
