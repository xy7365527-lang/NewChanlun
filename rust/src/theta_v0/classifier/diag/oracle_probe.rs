//! ★A3 证书 oracle 探针（仅 test 构建）：记录证书热路径分支命中，供 always-run oracle 断言
//! 「fixture 确实触发了 had_emitted_window pop（T==1/T>1）与两处早停缓存血缘失效」——防止
//! 「always-run 但覆盖为零」的陷阱（codex 审计第6条根修）。release/非 test 构建完全不编译。
//!
//! #748（C4）纯移动自 `classifier/mod.rs`（原内嵌 `pub mod oracle_probe`）。

use std::cell::RefCell;

#[derive(Default, Clone, Debug)]
pub struct Probe {
    /// had_emitted_window pop 后重扫仅复现被 pop 窗口（`tail_upper.len() == 1`）——did_extend 证伪正向锁。
    pub t_eq1: u64,
    /// had_emitted_window pop 后重扫产出 >1 窗口（frontier 值改写，bar-1464 型）。
    pub t_gt1: u64,
    /// `units.len() < min_parts` 早停 break 触发次数（truncate(level_idx) 语句执行——含 no-op）。
    pub minparts_break: u64,
    /// `units.is_empty()` 早停 break 触发次数（truncate(level_idx+1) 语句执行——含 no-op）。
    pub empty_break: u64,
    /// `units.len() < min_parts` 早停且实际 truncate 掉已建级（§2.6 血缘失效路径1，depth 下降）。
    pub reentry_minparts: u64,
    /// `units.is_empty()` 早停且实际 truncate 掉已建级（§2.6 血缘失效路径2，depth 下降）。
    pub reentry_empty: u64,
    /// on2w2 诊断：forest_dirty 由各 E-site 触发的分解计数（E1 L0 塔 / E2 did_extend / E3 cascade）。
    pub fd_l0: u64,
    pub fd_extend: u64,
    pub fd_cascade: u64,
    pub fd_any: u64,
    pub fd_calls: u64,
    /// on2w2-cascade 放行条件3 falsification 探针（env-gated，THETA_CASCADE_EPROBE=1）：cascade
    /// 命中事件计数（frontier_mutated 触发）。
    pub cascade_events: u64,
    /// cascade 事件中 `e` 坍缩到 units 起点（保留前缀 P==0，即无可保留前缀）的次数。
    /// 若 cascade_e0 / cascade_events ≈ 1.0 ⟹ e 常态坍缩 ⟹ 设计前提证伪 ⟹ NO-SHIP。
    pub cascade_e0: u64,
    /// cascade 事件的保留前缀比例 P/len 累加（× 1e6 定点）——均值 = sum / cascade_events / 1e6。
    /// 高比例（接近 1）⟹ e 局部化，增量收益大；接近 0 ⟹ 收益消失。
    pub cascade_keep_ppm_sum: u64,
}

thread_local! {
    static PROBE: RefCell<Probe> = RefCell::new(Probe::default());
}

pub fn reset() {
    PROBE.with(|p| *p.borrow_mut() = Probe::default());
}
pub fn snapshot() -> Probe {
    PROBE.with(|p| p.borrow().clone())
}
/// `t` = `tail_upper.len()`（本 bar 本级 compose 产出窗口数），had_emitted_window 时调用。
pub fn on_pop_rescan(t: usize) {
    PROBE.with(|p| {
        let mut p = p.borrow_mut();
        if t == 1 {
            p.t_eq1 += 1;
        } else if t > 1 {
            p.t_gt1 += 1;
        }
    });
}
/// on2w2：forest_dirty 各 E-site 分解计数。
pub fn on_forest_dirty(l0: bool, extend: bool, cascade: bool) {
    PROBE.with(|p| {
        let mut p = p.borrow_mut();
        p.fd_calls += 1;
        if l0 {
            p.fd_l0 += 1;
        }
        if extend {
            p.fd_extend += 1;
        }
        if cascade {
            p.fd_cascade += 1;
        }
        if l0 || extend || cascade {
            p.fd_any += 1;
        }
    });
}
/// on2w2-cascade 放行条件3 探针：一次 cascade 事件的保留前缀比例 P/len（`keep_frac` ∈ [0,1]）。
/// P=0（e 坍缩到起点，无可保留）时 `keep_frac==0.0`。仅在 THETA_CASCADE_EPROBE=1 时被调用方触发。
pub fn on_cascade_event(keep_frac: f64) {
    PROBE.with(|p| {
        let mut p = p.borrow_mut();
        p.cascade_events += 1;
        if keep_frac <= 0.0 {
            p.cascade_e0 += 1;
        }
        p.cascade_keep_ppm_sum += (keep_frac.clamp(0.0, 1.0) * 1e6) as u64;
    });
}
/// min_parts 早停：`removed` = 本次 truncate 是否实际删掉已建级（depth 下降）。
pub fn on_minparts_break(removed: bool) {
    PROBE.with(|p| {
        let mut p = p.borrow_mut();
        p.minparts_break += 1;
        if removed {
            p.reentry_minparts += 1;
        }
    });
}
/// units.is_empty 早停：`removed` = 本次 truncate 是否实际删掉已建级（depth 下降）。
pub fn on_empty_break(removed: bool) {
    PROBE.with(|p| {
        let mut p = p.borrow_mut();
        p.empty_break += 1;
        if removed {
            p.reentry_empty += 1;
        }
    });
}
