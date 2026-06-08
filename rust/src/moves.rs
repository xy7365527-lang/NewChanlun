//! 走势类型实例 v1 — 逐位等价移植自 src/newchan/a_move_v1.py
//!
//! 从已闭合中枢列表贪心分组构造 Move（盘整=1中枢 / 趋势=2+同向中枢）。
//! 模块名 `moves`（`move` 是 Rust 关键字）。
//!
//! ## 逐位等价要点
//! 1. 无浮点算术，只比较 + max/min 选择（high/low/zg_max/zd_min）。bit-exact 稳健。
//! 2. `persistence` 字段恒 0.0（PH 层后填，move 层不算），复刻 Python 默认值。
//! 3. `seg_end` 可能 = next_seg_start-1 / num_segments-1，用 i64 复刻 Python int。

use crate::stroke::Direction;
use crate::zhongshu::{BreakDir, Zhongshu};

/// 走势类型。对应 Python Literal["consolidation", "trend"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Consolidation,
    Trend,
}

impl MoveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MoveKind::Consolidation => "consolidation",
            MoveKind::Trend => "trend",
        }
    }
}

/// 一个走势类型实例。对应 Python `Move` frozen dataclass。
#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub kind: MoveKind,
    pub direction: Direction,
    pub seg_start: i64,
    pub seg_end: i64,
    pub zs_start: usize,
    pub zs_end: usize,
    pub zs_count: usize,
    pub settled: bool,
    pub high: f64,
    pub low: f64,
    pub first_seg_s0: usize,
    pub last_seg_s1: usize,
    pub zg_max: f64,
    pub zd_min: f64,
    pub persistence: f64,
}

/// 分组方向状态。对应 Python current_dir: "" / "up" / "down"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupDir {
    None,
    Up,
    Down,
}

/// 后枢 ZD 严格高于 前枢 ZG → 上涨延续。移植自 `_is_ascending`。
#[inline]
fn is_ascending(c1: &Zhongshu, c2: &Zhongshu) -> bool {
    c2.zd > c1.zg
}

/// 后枢 ZG 严格低于 前枢 ZD → 下跌延续。移植自 `_is_descending`。
#[inline]
fn is_descending(c1: &Zhongshu, c2: &Zhongshu) -> bool {
    c2.zg < c1.zd
}

/// 贪心分组：同向中枢归入同一 group。移植自 `_greedy_group`。
fn greedy_group(settled_zs: &[Zhongshu]) -> Vec<(Vec<usize>, GroupDir)> {
    let mut groups: Vec<(Vec<usize>, GroupDir)> = Vec::new();
    let mut current_offsets: Vec<usize> = vec![0];
    let mut current_dir = GroupDir::None;

    for k in 1..settled_zs.len() {
        let prev_zs = &settled_zs[k - 1];
        let curr_zs = &settled_zs[k];

        if is_ascending(prev_zs, curr_zs)
            && (current_dir == GroupDir::None || current_dir == GroupDir::Up)
        {
            current_offsets.push(k);
            current_dir = GroupDir::Up;
        } else if is_descending(prev_zs, curr_zs)
            && (current_dir == GroupDir::None || current_dir == GroupDir::Down)
        {
            current_offsets.push(k);
            current_dir = GroupDir::Down;
        } else {
            groups.push((current_offsets, current_dir));
            current_offsets = vec![k];
            current_dir = GroupDir::None;
        }
    }

    groups.push((current_offsets, current_dir));
    groups
}

/// 将一个 group 转换为 Move。移植自 `_group_to_move`。
fn group_to_move(
    offsets: &[usize],
    direction: GroupDir,
    settled_zs: &[Zhongshu],
    settled_indices: &[usize],
    next_seg_start: Option<i64>,
    num_segments: Option<usize>,
) -> Move {
    let first_zs = &settled_zs[offsets[0]];
    let last_zs = &settled_zs[offsets[offsets.len() - 1]];
    let zs_count = offsets.len();

    let mut base_seg_end: i64 = last_zs.seg_end as i64;
    if let Some(nss) = next_seg_start {
        base_seg_end = nss - 1;
    } else if let Some(ns) = num_segments {
        if ns > 0 {
            base_seg_end = ns as i64 - 1;
        }
    }

    let (kind, move_dir) = if zs_count >= 2 {
        // trend：方向 = group direction（必为 Up/Down）
        let d = match direction {
            GroupDir::Up => Direction::Up,
            GroupDir::Down => Direction::Down,
            GroupDir::None => Direction::Up, // 不可达（trend 必有方向）
        };
        (MoveKind::Trend, d)
    } else {
        // consolidation：方向 = first_zs.break_direction（settled 中枢必为 up/down）
        let d = match first_zs.break_direction {
            BreakDir::Up => Direction::Up,
            BreakDir::Down => Direction::Down,
            BreakDir::None => Direction::Up, // 不可达（settled 中枢有突破方向）
        };
        (MoveKind::Consolidation, d)
    };

    // group_centers = [settled_zs[o] for o in offsets]，max/min 顺序约简
    let mut high = settled_zs[offsets[0]].gg;
    let mut low = settled_zs[offsets[0]].dd;
    let mut zg_max = settled_zs[offsets[0]].zg;
    let mut zd_min = settled_zs[offsets[0]].zd;
    for &o in &offsets[1..] {
        let z = &settled_zs[o];
        high = high.max(z.gg);
        low = low.min(z.dd);
        zg_max = zg_max.max(z.zg);
        zd_min = zd_min.min(z.zd);
    }

    Move {
        kind,
        direction: move_dir,
        seg_start: first_zs.seg_start as i64,
        seg_end: base_seg_end,
        zs_start: settled_indices[offsets[0]],
        zs_end: settled_indices[offsets[offsets.len() - 1]],
        zs_count,
        settled: true,
        high,
        low,
        first_seg_s0: first_zs.first_seg_s0,
        last_seg_s1: last_zs.last_seg_s1,
        zg_max,
        zd_min,
        persistence: 0.0,
    }
}

/// 从中枢列表构造 Move（贪心分组，只处理 settled 中枢）。移植自 `moves_from_zhongshus`。
pub fn moves_from_zhongshus(zhongshus: &[Zhongshu], num_segments: Option<usize>) -> Vec<Move> {
    // _filter_settled
    let mut settled_indices: Vec<usize> = Vec::new();
    let mut settled_zs: Vec<Zhongshu> = Vec::new();
    for (i, zs) in zhongshus.iter().enumerate() {
        if zs.settled {
            settled_indices.push(i);
            settled_zs.push(*zs);
        }
    }
    if settled_zs.is_empty() {
        return Vec::new();
    }

    let groups = greedy_group(&settled_zs);

    let mut result: Vec<Move> = Vec::new();
    let n_groups = groups.len();
    for (g_idx, (offsets, direction)) in groups.iter().enumerate() {
        let next_seg_start: Option<i64> = if g_idx < n_groups - 1 {
            let next_first_offset = groups[g_idx + 1].0[0];
            Some(settled_zs[next_first_offset].seg_start as i64)
        } else {
            None
        };

        result.push(group_to_move(
            offsets,
            *direction,
            &settled_zs,
            &settled_indices,
            next_seg_start,
            num_segments,
        ));
    }

    if let Some(last) = result.last_mut() {
        last.settled = false;
    }

    result
}
