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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum GroupDir {
    #[default]
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

// ════════════════════════════════════════════════════════════
// 增量走势引擎 — 消除 `moves_from_zhongshus` 逐笔全量 O(Z) 的 O(S²) 累积
// ════════════════════════════════════════════════════════════

/// 增量走势构造器 —— bit-exact 等价于逐前缀 `moves_from_zhongshus`，摊还 O(1)/笔。
///
/// ## 不变量（bit-exact 契约，由 differential test 守卫）
/// 任意时刻 `current() == moves_from_zhongshus(&zhongshus_so_far, Some(num_segments))`。
///
/// ## 增量原理
/// settled 中枢恒为 `zhongshus[..stable_count]` 的 append-only 前缀（IncrementalBiZhongshu
/// 把每个 settled 中枢并入 stable_count——settled 中枢 extent 永久固定）。greedy 分组左→右
/// 单调：一个 group 被「方向不延续的后继中枢」封闭后永久固定，其 `group_to_move` 的
/// `next_seg_start` = 后继 group 起点（已固定）→ closed move 永久固定。唯一易变的是最后一个
/// **pending group**（其 move 的 `seg_end = num_segments-1` 随笔数变 + settled 标记翻转）。
/// 故缓存 closed moves（永久）+ greedy 续算状态，每次只重算 pending move。
#[derive(Debug, Clone, Default)]
pub struct IncrementalMoves {
    /// settled 中枢副本（append-only），greedy 续算需相邻比较。
    settled_zs: Vec<Zhongshu>,
    /// 对应原中枢列表的绝对索引（group_to_move 的 zs_start/zs_end 来源）。
    settled_indices: Vec<usize>,
    /// 已并入 settled_zs 的中枢前缀长度（下次从此续扫提取 settled）。
    settled_scan: usize,
    /// greedy 已处理的 settled_zs 索引（下一个待比较位置）。
    greedy_k: usize,
    greedy_started: bool,
    /// 当前 pending group 的 settled_zs 偏移 + 方向。
    cur_offsets: Vec<usize>,
    cur_dir: GroupDir,
    /// closed group 的 move（永久固定）。
    closed_moves: Vec<Move>,
    /// 上次并入 full_moves 的 closed move 数（增量拼接锚）。
    prev_closed_len: usize,
    /// 全量 moves = closed_moves + pending move（增量维护，避免每次 O(Z) 重建）。
    full_moves: Vec<Move>,
}

impl IncrementalMoves {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加/更新中枢列表（append-only settled 前缀）+ 当前段总数，增量推进走势。
    ///
    /// `zhongshus`：IncrementalBiZhongshu.current()（[settled*, 可选末尾 unsettled]）。
    /// `num_segments`：当前 confirmed 段（笔）总数（仅末 move 的 seg_end 用）。
    pub fn update(&mut self, zhongshus: &[Zhongshu], num_segments: usize) {
        // 1. 提取新 settled 中枢（settled 恒为前缀；末尾 unsettled 不并入，待其 settle）。
        let mut i = self.settled_scan;
        while i < zhongshus.len() && zhongshus[i].settled {
            self.settled_zs.push(zhongshus[i]);
            self.settled_indices.push(i);
            i += 1;
        }
        self.settled_scan = i;

        // 2. greedy 续算（逐位复刻 greedy_group，从 greedy_k 续跑）。
        if !self.greedy_started && !self.settled_zs.is_empty() {
            self.cur_offsets = vec![0];
            self.cur_dir = GroupDir::None;
            self.greedy_k = 1;
            self.greedy_started = true;
        }
        while self.greedy_k < self.settled_zs.len() {
            let k = self.greedy_k;
            let prev_zs = &self.settled_zs[k - 1];
            let curr_zs = &self.settled_zs[k];
            if is_ascending(prev_zs, curr_zs)
                && (self.cur_dir == GroupDir::None || self.cur_dir == GroupDir::Up)
            {
                self.cur_offsets.push(k);
                self.cur_dir = GroupDir::Up;
            } else if is_descending(prev_zs, curr_zs)
                && (self.cur_dir == GroupDir::None || self.cur_dir == GroupDir::Down)
            {
                self.cur_offsets.push(k);
                self.cur_dir = GroupDir::Down;
            } else {
                // 封闭当前 group：next_seg_start = 后继 group 起点中枢的 seg_start（固定）。
                let next_ss = self.settled_zs[k].seg_start as i64;
                let mv = group_to_move(
                    &self.cur_offsets,
                    self.cur_dir,
                    &self.settled_zs,
                    &self.settled_indices,
                    Some(next_ss),
                    None,
                );
                self.closed_moves.push(mv);
                self.cur_offsets = vec![k];
                self.cur_dir = GroupDir::None;
            }
            self.greedy_k += 1;
        }

        // 3. 增量拼接 full_moves = closed_moves + pending move（避免每次 O(Z) 全 clone）。
        //    truncate 移除上次 pending；extend 新封闭 closed；push 当前 pending。
        self.full_moves.truncate(self.prev_closed_len);
        self.full_moves
            .extend_from_slice(&self.closed_moves[self.prev_closed_len..]);
        self.prev_closed_len = self.closed_moves.len();

        if !self.cur_offsets.is_empty() {
            let pending = group_to_move(
                &self.cur_offsets,
                self.cur_dir,
                &self.settled_zs,
                &self.settled_indices,
                None,
                Some(num_segments),
            );
            self.full_moves.push(pending);
        }
        // 最后一个 move 置未结算（复刻 moves_from_zhongshus 末行）。
        if let Some(last) = self.full_moves.last_mut() {
            last.settled = false;
        }
    }

    /// 当前全量 moves（逐位等价于 `moves_from_zhongshus(&zhongshus, Some(num_segments))`）。
    pub fn current(&self) -> &[Move] {
        &self.full_moves
    }
}

#[cfg(test)]
mod incremental_moves_tests {
    use super::*;
    use crate::zhongshu::{BreakDir, IncrementalBiZhongshu, Zhongshu};

    /// 用 IncrementalBiZhongshu 从合成 Component 生成中枢序列，逐前缀断言增量 moves == 全量。
    fn assert_inc_moves_matches_full(comps: &[(f64, f64, usize)]) {
        let mut bz = IncrementalBiZhongshu::new();
        let mut im = IncrementalMoves::new();
        for (k, &(high, low, idx)) in comps.iter().enumerate() {
            bz.push_confirmed(high, low, idx, idx);
            let zss = bz.current();
            let num_segments = k + 1; // confirmed 笔数
            im.update(zss, num_segments);
            let full = moves_from_zhongshus(zss, Some(num_segments));
            let inc = im.current();
            assert_eq!(
                inc.len(),
                full.len(),
                "前缀 {} 处 move 数发散 inc={} full={}",
                k + 1,
                inc.len(),
                full.len()
            );
            for (a, b) in inc.iter().zip(full.iter()) {
                assert_eq!(a.kind, b.kind, "前缀{} kind", k + 1);
                assert_eq!(a.direction, b.direction, "前缀{} dir", k + 1);
                assert_eq!(a.seg_start, b.seg_start, "前缀{} seg_start", k + 1);
                assert_eq!(a.seg_end, b.seg_end, "前缀{} seg_end", k + 1);
                assert_eq!(a.zs_start, b.zs_start, "前缀{} zs_start", k + 1);
                assert_eq!(a.zs_end, b.zs_end, "前缀{} zs_end", k + 1);
                assert_eq!(a.zs_count, b.zs_count, "前缀{} zs_count", k + 1);
                assert_eq!(a.settled, b.settled, "前缀{} settled", k + 1);
                assert_eq!(a.high.to_bits(), b.high.to_bits(), "前缀{} high", k + 1);
                assert_eq!(a.low.to_bits(), b.low.to_bits(), "前缀{} low", k + 1);
                assert_eq!(
                    a.zg_max.to_bits(),
                    b.zg_max.to_bits(),
                    "前缀{} zg_max",
                    k + 1
                );
                assert_eq!(
                    a.zd_min.to_bits(),
                    b.zd_min.to_bits(),
                    "前缀{} zd_min",
                    k + 1
                );
                assert_eq!(a.first_seg_s0, b.first_seg_s0, "前缀{} first_seg_s0", k + 1);
                assert_eq!(a.last_seg_s1, b.last_seg_s1, "前缀{} last_seg_s1", k + 1);
            }
        }
        let _ = BreakDir::None;
        let _: Option<Zhongshu> = None;
    }

    #[test]
    fn inc_moves_simple_trend() {
        // 上升趋势：多个同向中枢
        let comps = vec![
            (110.0, 90.0, 0),
            (105.0, 95.0, 1),
            (108.0, 92.0, 2),
            (130.0, 120.0, 3),
            (135.0, 122.0, 4),
            (133.0, 121.0, 5),
            (160.0, 150.0, 6),
            (165.0, 152.0, 7),
            (163.0, 151.0, 8),
            (190.0, 180.0, 9),
        ];
        assert_inc_moves_matches_full(&comps);
    }

    #[test]
    fn inc_moves_lcg_stress() {
        let mut state: u64 = 0x2545F4914F6CDD1D;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64
        };
        let mut comps: Vec<(f64, f64, usize)> = Vec::new();
        let mut center = 100.0_f64;
        for k in 0..500 {
            center += (next() - 0.5) * 10.0;
            let half = 1.0 + next() * 5.0;
            comps.push((center + half, center - half, k));
        }
        assert_inc_moves_matches_full(&comps);
    }

    #[test]
    fn inc_moves_empty_and_consolidation() {
        // 反复盘整（无趋势延续）→ 多个单中枢 consolidation move
        let comps = vec![
            (110.0, 90.0, 0),
            (105.0, 95.0, 1),
            (108.0, 92.0, 2),
            (130.0, 120.0, 3), // 突破上 → 中枢1 settled
            (88.0, 80.0, 4),   // 反向
            (95.0, 85.0, 5),
            (92.0, 82.0, 6),
            (70.0, 60.0, 7), // 突破下 → 中枢2 settled
        ];
        assert_inc_moves_matches_full(&comps);
    }
}
