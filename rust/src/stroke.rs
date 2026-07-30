//! GUARD-ROLE: toplevel-loose-files——名分：现役（详见 `lib.rs` 头部 GUARD-ROLE 块，#764 C7-E5 核定；零删除/零移入 legacy/）。
//!
//! 笔构造（Stroke Construction）— 逐位等价移植自 src/newchan/a_stroke.py
//!
//! 从分型序列构造笔：分型去重（择优）、顶底交替、gap 检查、方向验证。

use crate::fractal::{Fractal, FractalKind};

/// 笔方向。对应 Python Literal["up", "down"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }
}

/// 一笔。对应 Python `Stroke` frozen dataclass。
#[derive(Debug, Clone, Copy)]
pub struct Stroke {
    /// 起点分型中心 idx（df_merged iloc 位置）。
    pub i0: usize,
    /// 终点分型中心 idx。
    pub i1: usize,
    pub direction: Direction,
    /// i0..=i1 区间内 high 的最大值。
    pub high: f64,
    /// i0..=i1 区间内 low 的最小值。
    pub low: f64,
    /// 起点分型极值价。
    pub p0: f64,
    /// 终点分型极值价。
    pub p1: f64,
    pub confirmed: bool,
}

/// numpy `ndarray.max()` 的逐位等价：对 vals[i0..=i1] 做**顺序**约简。
///
/// numpy 1D `.max()` 是 `np.maximum.reduce`——从首元素起逐个 `maximum`，
/// 无 pairwise 重排（pairwise 仅用于 sum/mean）。故初值取首元素、从左到右
/// 单调扫描即逐位等价。f64::max 对正常价格（无 NaN）等价于 numpy maximum。
#[inline]
fn slice_max(vals: &[f64], i0: usize, i1: usize) -> f64 {
    let mut m = vals[i0];
    let mut k = i0 + 1;
    while k <= i1 {
        m = m.max(vals[k]);
        k += 1;
    }
    m
}

/// numpy `ndarray.min()` 的逐位等价。见 [`slice_max`]。
#[inline]
fn slice_min(vals: &[f64], i0: usize, i1: usize) -> f64 {
    let mut m = vals[i0];
    let mut k = i0 + 1;
    while k <= i1 {
        m = m.min(vals[k]);
        k += 1;
    }
    m
}

/// 同类分型中 cand 是否比 start 更极端。移植自 `_is_more_extreme`。
#[inline]
pub fn is_more_extreme(cand: &Fractal, start: &Fractal) -> bool {
    (cand.kind == FractalKind::Top && cand.price > start.price)
        || (cand.kind == FractalKind::Bottom && cand.price < start.price)
}

/// §4.3 gap 检查：宽笔/严笔/新笔三种模式。移植自 `_check_gap`。
///
/// `merged_to_raw` 在 new 模式下必需。`merged_gap`/`raw_gap` 用 i64 计算以
/// 复刻 Python int 的有符号语义（避免 usize underflow）。
#[inline]
pub fn check_gap(
    start: &Fractal,
    cand: &Fractal,
    use_new_bi: bool,
    min_gap: i64,
    merged_to_raw: &[(usize, usize)],
    new_raw_gap_min: i64,
) -> bool {
    let merged_gap: i64 = cand.idx as i64 - start.idx as i64;
    if use_new_bi {
        let raw_gap: i64 = merged_to_raw[cand.idx].0 as i64 - merged_to_raw[start.idx].1 as i64 - 1;
        return merged_gap >= 2 && raw_gap >= new_raw_gap_min;
    }
    merged_gap >= min_gap
}

/// §4.4 方向与有效性。移植自 `_validate_direction`。
///
/// 返回 (direction, valid)。Python 签名为 `... | None` 但两分支均返回 tuple，
/// 实际永不为 None；此处直接返回元组以消除不可达分支。
#[inline]
pub fn validate_direction(start: &Fractal, cand: &Fractal) -> (Direction, bool) {
    if start.kind == FractalKind::Bottom && cand.kind == FractalKind::Top {
        (Direction::Up, cand.price > start.price)
    } else {
        (Direction::Down, cand.price < start.price)
    }
}

/// 从一对异类分型构造一笔。移植自 `_build_stroke`（confirmed=True）。
#[inline]
pub fn build_stroke(
    start: &Fractal,
    cand: &Fractal,
    direction: Direction,
    highs: &[f64],
    lows: &[f64],
) -> Stroke {
    // **S6（T7 笔=两个相反分型连接，最小方向单元）运行时证明**：每笔起止分型方向相反
    // （一顶一底）∧ 方向与端点一致（底→顶=Up、顶→底=Down）。同类分型间不构成笔（无方向，
    // T7：相邻两相反分型之间的连接 = 一段单向运动）。violation = panic。
    assert!(
        start.kind != cand.kind,
        "S6(T7) 违反@i0 {} i1 {}：笔的起止分型同类（{:?}→{:?}）——笔必连接相反分型（一顶一底）",
        start.idx,
        cand.idx,
        start.kind,
        cand.kind
    );
    debug_assert!(
        match direction {
            Direction::Up => start.kind == FractalKind::Bottom && cand.kind == FractalKind::Top,
            Direction::Down => start.kind == FractalKind::Top && cand.kind == FractalKind::Bottom,
        },
        "S6(T7) 方向与端点不一致：{direction:?} 但 {:?}→{:?}",
        start.kind,
        cand.kind
    );
    let i0 = start.idx;
    let i1 = cand.idx;
    Stroke {
        i0,
        i1,
        direction,
        high: slice_max(highs, i0, i1),
        low: slice_min(lows, i0, i1),
        p0: start.price,
        p1: cand.price,
        confirmed: true,
    }
}

/// 锁定态：延伸上一笔至更极端的同类分型（原地替换列表尾元素）。
/// 移植自 `_extend_prev_stroke`。
#[inline]
pub fn extend_prev_stroke(strokes: &mut [Stroke], cand: &Fractal, highs: &[f64], lows: &[f64]) {
    let last_idx = strokes.len() - 1;
    let prev = strokes[last_idx];
    let new_i1 = cand.idx;
    let seg_high = slice_max(highs, prev.i0, new_i1);
    let seg_low = slice_min(lows, prev.i0, new_i1);
    strokes[last_idx] = Stroke {
        i0: prev.i0,
        i1: new_i1,
        direction: prev.direction,
        high: seg_high,
        low: seg_low,
        p0: prev.p0,
        p1: cand.price,
        confirmed: prev.confirmed,
    };
}
