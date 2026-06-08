//! PH 计算层 — 逐位等价移植自 src/newchan/ph_layer.py
//!
//! 为递归级别的 Move 附着 persistence 值，并给出递归终止判据。
//!
//! ## 逐位等价要点（bit-exact 基础）
//! 1. **无 filtration、无 union-find**：`compute_move_persistence` 取 1D sublevel
//!    filtration 的最大（最久存活）H0 特征，它恒等于全局价格极差
//!    `max(prices) - min(prices)`（全局连通分量从全局 min 跨到 finite_cap=max）。
//!    Python 早已用此闭式（ph_layer.py docstring + 闭式实现），认识论等级 L0。
//!    本移植直接复刻闭式：O(k) 极值扫描，无对象分配。**不引入区间树**——闭式
//!    已是最优，区间树只对"完整 barcode"（次级 H0 prominence）有意义，而本路径
//!    只取最大 H0 bar，区间树是杀鸡用牛刀（伪优化）。
//! 2. **唯一算术是减法 `hi - lo`**：其余全是比较 + max/min 选择，bit-exact 稳健。
//! 3. **max/min 约简顺序与 Python 一致**：按列表索引顺序 fold，f64 比较无 NaN
//!    争议（persistence 由价格极差产生，恒有限）。

use crate::moves::Move;

/// 中枢价格视图 — `compute_move_persistence` 只需 dd/gg 两个字段。
///
/// 对应 `ph_layer._HasZgZd` Protocol 中本闭式实际访问的子集（dd=固定区间含离开段
/// 下界，gg=上界）。其余 zg/zd 字段闭式路径不用。
#[derive(Debug, Clone, Copy)]
pub struct ZsPriceView {
    pub dd: f64,
    pub gg: f64,
}

/// 计算单个 Move 的 H0 persistence（闭式 max-min）。
///
/// 移植自 `ph_layer.compute_move_persistence`。
///
/// 价格序列 = Move 内所有 center 的 (dd, gg) 交替；persistence = 极差。
/// center 数不足（slice 为空或仅 1 个）退化为 `high - low`。
///
/// 认识论等级：L0（代数恒等式，零信息增量）。
pub fn compute_move_persistence(m: &Move, zhongshus: &[ZsPriceView]) -> f64 {
    let n = zhongshus.len();
    // Python: end = min(zs_end, len-1); if start > end or start >= len → high-low。
    // 空列表时 len-1 在 Python 为 -1，且 start(usize>=0) >= len(0) 恒真 → high-low。
    // Rust usize 无法表达 -1，故空列表特判（与 Python 分支等价）。
    if n == 0 {
        return m.high - m.low;
    }
    let end = m.zs_end.min(n - 1);
    if m.zs_start > end || m.zs_start >= n {
        return m.high - m.low;
    }
    // len(prices) = 2 * (end - start + 1)；end == start ⟺ 仅 1 个 center → 退化。
    if end == m.zs_start {
        return m.high - m.low;
    }

    // 无分配 max/min：等价于 max(_center_prices(slice)) - min(...)。热路径。
    let mut hi = f64::NEG_INFINITY;
    let mut lo = f64::INFINITY;
    for z in &zhongshus[m.zs_start..=end] {
        if z.dd > hi {
            hi = z.dd;
        }
        if z.gg > hi {
            hi = z.gg;
        }
        if z.dd < lo {
            lo = z.dd;
        }
        if z.gg < lo {
            lo = z.gg;
        }
    }
    hi - lo
}

/// 为 Move 列表计算并附着 persistence 值，返回新列表（immutable，复刻 Python 语义）。
///
/// 移植自 `ph_layer.attach_persistence`。Move 是 Copy，逐字段复制后仅改 persistence，
/// 与 Python "直接 Move(...) 构造（非 dataclasses.replace）" 逐位等价。
pub fn attach_persistence(moves: &[Move], zhongshus: &[ZsPriceView]) -> Vec<Move> {
    moves
        .iter()
        .map(|m| {
            let mut nm = *m;
            nm.persistence = compute_move_persistence(m, zhongshus);
            nm
        })
        .collect()
}

/// 判断递归是否应当终止。
///
/// 移植自 `ph_layer.should_stop_recursion`。
///
/// 条件：当前层所有 (settled ∧ persistence>0) Move 的 max persistence
///       < 前一层所有 (settled ∧ persistence>0) Move 的 min persistence。
/// 任一层无合格 Move → False（复刻 Python `if not curr or not prev: return False`）。
///
/// 解释：当前层最大结构比前一层最小结构还小 → 尺度分离完成（L0/L1/L2 多级别 settle
/// 的层间终止判据，见 recursive_stack.py）。
pub fn should_stop_recursion(curr_moves: &[Move], prev_moves: &[Move]) -> bool {
    // max over (settled ∧ persistence>0)，按索引顺序约简（与 Python max 一致）。
    let curr_max = curr_moves
        .iter()
        .filter(|m| m.settled && m.persistence > 0.0)
        .map(|m| m.persistence)
        .fold(None, |acc: Option<f64>, p| {
            Some(match acc {
                Some(a) => a.max(p),
                None => p,
            })
        });
    let prev_min = prev_moves
        .iter()
        .filter(|m| m.settled && m.persistence > 0.0)
        .map(|m| m.persistence)
        .fold(None, |acc: Option<f64>, p| {
            Some(match acc {
                Some(a) => a.min(p),
                None => p,
            })
        });

    match (curr_max, prev_min) {
        (Some(cmax), Some(pmin)) => cmax < pmin,
        _ => false, // 任一层无合格 Move
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::{Move, MoveKind};
    use crate::stroke::Direction;

    /// 构造一个最小 Move，仅填本层关心的字段。
    fn mk_move(zs_start: usize, zs_end: usize, high: f64, low: f64, settled: bool) -> Move {
        Move {
            kind: MoveKind::Trend,
            direction: Direction::Up,
            seg_start: 0,
            seg_end: 0,
            zs_start,
            zs_end,
            // zs_count 不参与 PH 计算；saturating 避免 start>end 用例下溢。
            zs_count: zs_end.saturating_sub(zs_start) + 1,
            settled,
            high,
            low,
            first_seg_s0: 0,
            last_seg_s1: 0,
            zg_max: 0.0,
            zd_min: 0.0,
            persistence: 0.0,
        }
    }

    fn zs(dd: f64, gg: f64) -> ZsPriceView {
        ZsPriceView { dd, gg }
    }

    #[test]
    fn persistence_empty_zhongshus_returns_high_low() {
        let m = mk_move(0, 0, 110.0, 90.0, true);
        assert_eq!(compute_move_persistence(&m, &[]), 20.0);
    }

    #[test]
    fn persistence_single_center_degenerates_to_high_low() {
        // end == start → high - low
        let m = mk_move(0, 0, 105.0, 95.0, true);
        let zss = [zs(96.0, 104.0)];
        assert_eq!(compute_move_persistence(&m, &zss), 10.0);
    }

    #[test]
    fn persistence_multi_center_is_price_range() {
        // 两个 center：dd/gg 极差 = max(101,108,90,99) - min(...) = 108 - 90 = 18
        let m = mk_move(0, 1, 200.0, 0.0, true);
        let zss = [zs(101.0, 108.0), zs(90.0, 99.0)];
        assert_eq!(compute_move_persistence(&m, &zss), 18.0);
    }

    #[test]
    fn persistence_zs_end_clamped_to_len() {
        // zs_end 超界 → 被 clamp 到 len-1
        let m = mk_move(0, 99, 200.0, 0.0, true);
        let zss = [zs(101.0, 108.0), zs(90.0, 99.0)];
        assert_eq!(compute_move_persistence(&m, &zss), 18.0);
    }

    #[test]
    fn persistence_start_beyond_end_returns_high_low() {
        // zs_start > clamped end → high - low
        let m = mk_move(5, 1, 130.0, 100.0, true);
        let zss = [zs(101.0, 108.0), zs(90.0, 99.0)];
        assert_eq!(compute_move_persistence(&m, &zss), 30.0);
    }

    #[test]
    fn attach_sets_persistence_field() {
        let moves = [mk_move(0, 1, 200.0, 0.0, true)];
        let zss = [zs(101.0, 108.0), zs(90.0, 99.0)];
        let out = attach_persistence(&moves, &zss);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].persistence, 18.0);
        // 其余字段不变
        assert_eq!(out[0].high, 200.0);
        assert_eq!(out[0].zs_start, 0);
    }

    #[test]
    fn stop_recursion_true_when_curr_max_below_prev_min() {
        let mut a = mk_move(0, 0, 0.0, 0.0, true);
        a.persistence = 5.0;
        let mut b = mk_move(0, 0, 0.0, 0.0, true);
        b.persistence = 3.0;
        let curr = [a, b]; // max = 5
        let mut c = mk_move(0, 0, 0.0, 0.0, true);
        c.persistence = 10.0;
        let mut d = mk_move(0, 0, 0.0, 0.0, true);
        d.persistence = 8.0;
        let prev = [c, d]; // min = 8
        assert!(should_stop_recursion(&curr, &prev)); // 5 < 8
    }

    #[test]
    fn stop_recursion_false_when_overlap() {
        let mut a = mk_move(0, 0, 0.0, 0.0, true);
        a.persistence = 9.0;
        let curr = [a]; // max = 9
        let mut c = mk_move(0, 0, 0.0, 0.0, true);
        c.persistence = 8.0;
        let prev = [c]; // min = 8
        assert!(!should_stop_recursion(&curr, &prev)); // 9 < 8 false
    }

    #[test]
    fn stop_recursion_false_when_no_qualifying_moves() {
        // 全部 unsettled → curr_settled 空 → False
        let a = mk_move(0, 0, 0.0, 0.0, false);
        let mut c = mk_move(0, 0, 0.0, 0.0, true);
        c.persistence = 8.0;
        assert!(!should_stop_recursion(&[a], &[c]));
    }

    #[test]
    fn stop_recursion_filters_zero_persistence() {
        // persistence == 0 不计入（Python: persistence > 0）
        let mut a = mk_move(0, 0, 0.0, 0.0, true);
        a.persistence = 0.0; // 被过滤
        let mut b = mk_move(0, 0, 0.0, 0.0, true);
        b.persistence = 2.0;
        let curr = [a, b]; // 仅 b 合格，max = 2
        let mut c = mk_move(0, 0, 0.0, 0.0, true);
        c.persistence = 5.0;
        let prev = [c]; // min = 5
        assert!(should_stop_recursion(&curr, &prev)); // 2 < 5
    }
}
