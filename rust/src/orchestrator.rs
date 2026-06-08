//! orchestrator.rs — 递归编排器（组合层）逐位等价移植自
//! src/newchan/orchestrator/recursive.py 的 `RecursiveOrchestrator`
//! + src/newchan/core/recursion/ 的五层有状态引擎 + recursive_stack.py。
//!
//! ## 移植边界（严格声明）
//!
//! 本模块复刻**逐 bar 结构化状态**的演化：strokes → segments → zhongshus →
//! moves → buysellpoints → 递归级别（zhongshus/moves）。输出逐位等价于 Python
//! `RecursiveOrchestrator.process_bar(bar)` 返回的 `RecursiveOrchestratorSnapshot`
//! 中的**结构化列表**字段。
//!
//! **DomainEvent 流不在范围内**（与 bi_engine.rs 同款声明）：events 是 diff 的纯
//! 副产物，不参与状态递归——下游引擎只读 `snapshot.moves` 不读 `.events`（已核查
//! recursive_stack.py / recursive_level_engine.py / *_engine.py）。故"逐位等价"
//! 收缩到结构化列表，与既有 parity 测试（比对列表非事件）一致。
//!
//! ## 短路缓存（与 Python 各引擎同构）
//!
//! 每层持 `prev_*` 状态 + `last_*_key` 短路键：输入指纹未变 ⟹ 返回缓存（状态不变）。
//! 短路是**纯性能**——全量重算确定且产出相同状态，短路只是跳过重算。键逐字段复刻
//! Python 各引擎（key 初值用 None，首 bar 必重算，产出与 Python 初值 `()` 短路
//! 等同的空状态，逐位等价）。
//!
//! ## 线段层：全量重算（resume≡full）
//!
//! Python `SegmentEngine` 用 checkpoint/resume 增量（O(tail)）；其 docstring 证明
//! resume 数学等价于全量。Rust `segments_from_strokes_v1` 只移植批量路径，故本层
//! 笔尾变化时**全量重算**——对 Python（用 resume）逐位等价（两者产出同一线段列表）。
//! resume 是纯性能优化，非语义；不移植不构成正确性缺口（认识论等级：L0）。

use crate::bi_engine::BiEngine;
use crate::buysellpoint::{buysellpoints_from_level, BuySellPoint};
use crate::divergence::{
    divergences_from_moves_v1, MacdCtx, MoveView, SegView, ZsView,
};
use crate::level::{
    moves_from_level_zhongshus, zhongshu_from_components, CompView, LevelZhongshu,
};
use crate::macd::OnlineMacdState;
use crate::moves::{moves_from_zhongshus, Move};
use crate::ph::{attach_persistence, should_stop_recursion, ZsPriceView};
use crate::segment::{segments_from_strokes_v1, Segment};
use crate::stroke::{Direction, Stroke};
use crate::zhongshu::{zhongshu_from_segments, BreakDir, Zhongshu};

// ════════════════════════════════════════════════════════════
// 短路键（逐字段复刻 Python 各引擎的 last_*_key）
// ════════════════════════════════════════════════════════════

/// 线段层笔尾键。移植自 `segment_engine._stroke_tail_key`。
#[derive(Debug, Clone, PartialEq)]
enum SegTailKey {
    Empty,
    One {
        n: usize,
        a: (usize, usize, f64, f64),
    },
    Two {
        n: usize,
        a: (usize, usize, f64, f64),
        b: (usize, usize, f64, f64),
    },
}

fn seg_tail_key(strokes: &[Stroke]) -> SegTailKey {
    let n = strokes.len();
    if n == 0 {
        return SegTailKey::Empty;
    }
    if n >= 2 {
        let s1 = &strokes[n - 2];
        let s2 = &strokes[n - 1];
        SegTailKey::Two {
            n,
            a: (s1.i0, s1.i1, s1.p0, s1.p1),
            b: (s2.i0, s2.i1, s2.p0, s2.p1),
        }
    } else {
        let s1 = &strokes[n - 1];
        SegTailKey::One {
            n,
            a: (s1.i0, s1.i1, s1.p0, s1.p1),
        }
    }
}

/// 中枢层线段键。移植自 `zhongshu_engine.process_segment_snapshot`。
#[derive(Debug, Clone, PartialEq)]
enum ZsSegKey {
    Empty,
    One { s0: usize, s1: usize },
    Many { n: usize, s0: usize, s1: usize, dir: Direction },
}

fn zs_seg_key(segs: &[Segment]) -> ZsSegKey {
    let n = segs.len();
    if n >= 2 {
        let s = &segs[n - 2];
        ZsSegKey::Many { n, s0: s.s0, s1: s.s1, dir: s.direction }
    } else if n == 1 {
        ZsSegKey::One { s0: segs[0].s0, s1: segs[0].s1 }
    } else {
        ZsSegKey::Empty
    }
}

/// 走势层中枢键。移植自 `move_engine.process_zhongshu_snapshot`（O(1) settled 推导）。
#[derive(Debug, Clone, PartialEq)]
enum MoveZsKey {
    /// n_settled >= 1：4 元组 (n_zs, n_settled, last_settled.seg_end, num_seg)。
    A { n_zs: usize, n_settled: usize, seg_end: usize, num_seg: i64 },
    /// n_zs==0 → (0,0,ns)；n_settled==0 → (n_zs,0,ns)。两者均为 (n_zs,0,ns) 形。
    B { n_zs: usize, num_seg: i64 },
}

fn move_zs_key(zss: &[Zhongshu], num_seg: i64) -> MoveZsKey {
    let n_zs = zss.len();
    if n_zs == 0 {
        return MoveZsKey::B { n_zs: 0, num_seg };
    }
    let last = &zss[n_zs - 1];
    let (n_settled, last_settled): (usize, Option<&Zhongshu>) = if last.settled {
        (n_zs, Some(last))
    } else if n_zs >= 2 {
        (n_zs - 1, Some(&zss[n_zs - 2]))
    } else {
        (n_zs - 1, None)
    };
    if n_settled >= 1 {
        MoveZsKey::A {
            n_zs,
            n_settled,
            seg_end: last_settled.unwrap().seg_end,
            num_seg,
        }
    } else {
        MoveZsKey::B { n_zs, num_seg }
    }
}

/// 买卖点层输入键。移植自 `buysellpoint_engine.process_snapshots`。
#[derive(Debug, Clone, PartialEq)]
enum BspKey {
    Small { n_seg: usize, n_zs: usize, n_mv: usize },
    Big { n_seg: usize, s0: usize, s1: usize, n_zs: usize, n_mv: usize },
}

fn bsp_key(segs: &[Segment], n_zs: usize, n_mv: usize) -> BspKey {
    let n_seg = segs.len();
    if n_seg >= 2 {
        let s = &segs[n_seg - 2];
        BspKey::Big { n_seg, s0: s.s0, s1: s.s1, n_zs, n_mv }
    } else {
        BspKey::Small { n_seg, n_zs, n_mv }
    }
}

/// 走势末尾键（递归层 + 栈顶共用）。移植自 `recursive_level_engine` /
/// `recursive_stack` 的 move_key / in_key：(n, seg_start, seg_end, settled) 或 (0,)。
#[derive(Debug, Clone, PartialEq)]
enum MoveTailKey {
    Empty,
    Filled { n: usize, ss: i64, se: i64, settled: bool },
}

fn move_tail_key(moves: &[Move]) -> MoveTailKey {
    let n = moves.len();
    if n >= 1 {
        let m = &moves[n - 1];
        MoveTailKey::Filled { n, ss: m.seg_start, se: m.seg_end, settled: m.settled }
    } else {
        MoveTailKey::Empty
    }
}

// ════════════════════════════════════════════════════════════
// 视图转换（Rust 结构 → divergence/bsp 入参 View）
// ════════════════════════════════════════════════════════════

fn seg_views(segs: &[Segment]) -> Vec<SegView> {
    segs.iter()
        .map(|s| SegView {
            direction: s.direction,
            high: s.high,
            low: s.low,
            i0: s.i0,
            i1: s.i1,
        })
        .collect()
}

fn zs_views(zss: &[Zhongshu]) -> Vec<ZsView> {
    zss.iter()
        .map(|z| ZsView {
            zd: z.zd,
            zg: z.zg,
            seg_start: z.seg_start,
            seg_end: z.seg_end,
            settled: z.settled,
        })
        .collect()
}

fn zs_break(zss: &[Zhongshu]) -> Vec<(bool, BreakDir, i64)> {
    zss.iter()
        .map(|z| (z.settled, z.break_direction, z.break_seg))
        .collect()
}

fn move_views(moves: &[Move]) -> Vec<MoveView> {
    moves
        .iter()
        .map(|m| MoveView {
            kind: m.kind,
            direction: m.direction,
            seg_start: m.seg_start,
            seg_end: m.seg_end,
            zs_start: m.zs_start,
            zs_end: m.zs_end,
            zs_count: m.zs_count,
            settled: m.settled,
        })
        .collect()
}

/// 中枢/泛化中枢 → PH 价格视图（dd/gg）。
fn zs_price_views(zss: &[Zhongshu]) -> Vec<ZsPriceView> {
    zss.iter().map(|z| ZsPriceView { dd: z.dd, gg: z.gg }).collect()
}

fn level_zs_price_views(zss: &[LevelZhongshu]) -> Vec<ZsPriceView> {
    zss.iter().map(|z| ZsPriceView { dd: z.dd, gg: z.gg }).collect()
}

// ════════════════════════════════════════════════════════════
// 递归级别引擎（level >= 2）
// ════════════════════════════════════════════════════════════

/// 一个递归级别快照（结构化部分）。对应 Python `RecursiveLevelSnapshot` 的
/// level_id/zhongshus/moves 字段。
#[derive(Debug, Clone)]
pub struct LevelSnapshot {
    pub level_id: i64,
    pub zhongshus: Vec<LevelZhongshu>,
    pub moves: Vec<Move>,
}

/// 级别递归引擎。移植自 `recursive_level_engine.RecursiveLevelEngine`。
struct LevelEngine {
    level_id: i64,
    prev_zhongshus: Vec<LevelZhongshu>,
    prev_moves: Vec<Move>,
    last_move_key: Option<MoveTailKey>,
}

impl LevelEngine {
    fn new(level_id: i64) -> Self {
        LevelEngine {
            level_id,
            prev_zhongshus: Vec::new(),
            prev_moves: Vec::new(),
            last_move_key: None,
        }
    }

    /// 消费下级 moves，产生本级 zhongshus + moves。移植自 `process_move_snapshot`。
    fn process(&mut self, in_moves: &[Move]) -> LevelSnapshot {
        let key = move_tail_key(in_moves);
        if self.last_move_key.as_ref() == Some(&key) {
            return LevelSnapshot {
                level_id: self.level_id,
                zhongshus: self.prev_zhongshus.clone(),
                moves: self.prev_moves.clone(),
            };
        }
        self.last_move_key = Some(key);

        // settled 走势 → MoveAsComponent（high=zg_max||high，low=zd_min||low，
        // component_idx = settled 列表中的位置）。
        let comps: Vec<CompView> = in_moves
            .iter()
            .filter(|m| m.settled)
            .enumerate()
            .map(|(i, m)| CompView {
                high: if m.zg_max != 0.0 { m.zg_max } else { m.high },
                low: if m.zd_min != 0.0 { m.zd_min } else { m.low },
                component_idx: i,
            })
            .collect();

        let curr_zhongshus = zhongshu_from_components(&comps, self.level_id);
        let curr_moves = moves_from_level_zhongshus(&curr_zhongshus);
        let pv = level_zs_price_views(&curr_zhongshus);
        let curr_moves = attach_persistence(&curr_moves, &pv);

        self.prev_zhongshus = curr_zhongshus.clone();
        self.prev_moves = curr_moves.clone();

        LevelSnapshot {
            level_id: self.level_id,
            zhongshus: curr_zhongshus,
            moves: curr_moves,
        }
    }
}

/// 递归栈调度器。移植自 `recursive_stack.RecursiveStack`。
struct RecursiveStack {
    max_levels: i64,
    /// engines[i] 对应 level_id = i + 2（懒创建，按序 push）。
    engines: Vec<LevelEngine>,
    last_input_key: Option<MoveTailKey>,
    cached: Vec<LevelSnapshot>,
}

impl RecursiveStack {
    fn new(max_levels: i64) -> Self {
        RecursiveStack {
            max_levels,
            engines: Vec::new(),
            last_input_key: None,
            cached: Vec::new(),
        }
    }

    /// 从 level-1 moves 递归向上。移植自 `process_level1_move_snapshot`。
    fn process(&mut self, l1_moves: &[Move]) -> Vec<LevelSnapshot> {
        let in_key = move_tail_key(l1_moves);
        if self.last_input_key.as_ref() == Some(&in_key) {
            // 状态不变 → 复用缓存（Python 仅刷新 bar_idx/ts，结构不变）。
            return self.cached.clone();
        }
        self.last_input_key = Some(in_key);

        let mut snapshots: Vec<LevelSnapshot> = Vec::new();
        let mut current_moves: Vec<Move> = l1_moves.to_vec();
        let mut prev_moves: Vec<Move> = l1_moves.to_vec();
        let mut current_level: i64 = 1;

        while current_level < self.max_levels {
            let next_level = current_level + 1;
            let idx = (next_level - 2) as usize;
            if idx == self.engines.len() {
                self.engines.push(LevelEngine::new(next_level));
            }
            let snap = self.engines[idx].process(&current_moves);
            let snap_moves = snap.moves.clone();
            snapshots.push(snap);

            if snap_moves.len() < 3 {
                break;
            }
            if should_stop_recursion(&snap_moves, &prev_moves) {
                break;
            }

            current_moves = snap_moves.clone();
            prev_moves = snap_moves;
            current_level = next_level;
        }

        self.cached = snapshots.clone();
        snapshots
    }

    fn reset(&mut self) {
        self.engines.clear();
        self.last_input_key = None;
        self.cached.clear();
    }
}

// ════════════════════════════════════════════════════════════
// 主编排器
// ════════════════════════════════════════════════════════════

/// 递归编排器。移植自 `RecursiveOrchestrator`。逐 bar 驱动全链。
pub struct RecursiveOrchestrator {
    bi: BiEngine,
    level_id: i64,
    enable_macd: bool,
    macd: Option<OnlineMacdState>,

    // 线段层
    prev_segments: Vec<Segment>,
    last_seg_key: Option<SegTailKey>,
    // 中枢层
    prev_zhongshus: Vec<Zhongshu>,
    last_zs_key: Option<ZsSegKey>,
    // 走势层
    prev_moves: Vec<Move>,
    last_move_key: Option<MoveZsKey>,
    // 买卖点层
    prev_bsps: Vec<BuySellPoint>,
    last_bsp_key: Option<BspKey>,

    // 递归栈
    stack: RecursiveStack,
    // 最近一次递归快照（供 accessor 读取）
    recursive: Vec<LevelSnapshot>,
}

impl RecursiveOrchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_levels: i64,
        stroke_mode: &str,
        min_strict_sep: i64,
        reset_dir_on_fractal: bool,
        new_raw_gap_min: i64,
        enable_macd_divergence: bool,
    ) -> Self {
        RecursiveOrchestrator {
            bi: BiEngine::new(stroke_mode, min_strict_sep, reset_dir_on_fractal, new_raw_gap_min),
            level_id: 1,
            enable_macd: enable_macd_divergence,
            macd: if enable_macd_divergence {
                Some(OnlineMacdState::new(12, 26, 9))
            } else {
                None
            },
            prev_segments: Vec::new(),
            last_seg_key: None,
            prev_zhongshus: Vec::new(),
            last_zs_key: None,
            prev_moves: Vec::new(),
            last_move_key: None,
            prev_bsps: Vec::new(),
            last_bsp_key: None,
            stack: RecursiveStack::new(max_levels),
            recursive: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.bi.reset();
        if let Some(m) = self.macd.as_mut() {
            m.reset();
        }
        self.prev_segments.clear();
        self.last_seg_key = None;
        self.prev_zhongshus.clear();
        self.last_zs_key = None;
        self.prev_moves.clear();
        self.last_move_key = None;
        self.prev_bsps.clear();
        self.last_bsp_key = None;
        self.stack.reset();
        self.recursive.clear();
    }

    /// 逐 bar 驱动全链。移植自 `RecursiveOrchestrator.process_bar`。
    pub fn process_bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        // ── Level=1 五层管线 ──
        self.bi.process_bar(o, h, l, c);

        // MACD 增量（enable 时每 bar 无条件 update，⇔ Python `_compute_macd`：
        // EMA 因果递推每 bar 推进，与 bsp 是否短路无关）。
        if let Some(m) = self.macd.as_mut() {
            m.update(c);
        }

        // 线段层（笔尾变化 → 全量重算；resume≡full）
        let seg_key = seg_tail_key(self.bi.current_strokes());
        if self.last_seg_key.as_ref() != Some(&seg_key) {
            self.last_seg_key = Some(seg_key);
            self.prev_segments = segments_from_strokes_v1(self.bi.current_strokes(), 3, true);
        }
        let n_seg = self.prev_segments.len();

        // 中枢层
        let zs_key = zs_seg_key(&self.prev_segments);
        if self.last_zs_key.as_ref() != Some(&zs_key) {
            self.last_zs_key = Some(zs_key);
            // Segment → (s0,s1,high,low,confirmed,kind==settled) 元组（复用既有过滤）
            let seg_tuples: Vec<(usize, usize, f64, f64, bool, bool)> = self
                .prev_segments
                .iter()
                .map(|s| {
                    (s.s0, s.s1, s.high, s.low, s.confirmed, s.kind.as_str() == "settled")
                })
                .collect();
            self.prev_zhongshus = zhongshu_from_segments(&seg_tuples);
        }

        // 走势层（num_segments = 线段总数）
        let mv_key = move_zs_key(&self.prev_zhongshus, n_seg as i64);
        if self.last_move_key.as_ref() != Some(&mv_key) {
            self.last_move_key = Some(mv_key);
            let mut moves = moves_from_zhongshus(&self.prev_zhongshus, Some(n_seg));
            let pv = zs_price_views(&self.prev_zhongshus);
            moves = attach_persistence(&moves, &pv);
            self.prev_moves = moves;
        }

        // 买卖点层
        let bk = bsp_key(&self.prev_segments, self.prev_zhongshus.len(), self.prev_moves.len());
        if self.last_bsp_key.as_ref() != Some(&bk) {
            self.last_bsp_key = Some(bk);
            self.prev_bsps = self.compute_bsps();
        }

        // ── 递归层（level >= 2）──
        self.recursive = self.stack.process(&self.prev_moves);
    }

    /// 计算背驰 + 买卖点（含 MACD 路径）。
    fn compute_bsps(&self) -> Vec<BuySellPoint> {
        let segs = seg_views(&self.prev_segments);
        let zss = zs_views(&self.prev_zhongshus);
        let zsb = zs_break(&self.prev_zhongshus);
        let mvs = move_views(&self.prev_moves);

        // MACD 上下文：enable 时取 macd 状态 series + bi.merged_to_raw（⇔ Python
        // df_macd is not None and merged_to_raw is not None）。
        let macd_series = if self.enable_macd {
            self.macd.as_ref().map(|m| m.series())
        } else {
            None
        };
        let m2r = self.bi.merged_to_raw();
        let macd_ctx = macd_series.as_ref().map(|s| MacdCtx {
            macd: s.macd.as_slice(),
            hist: s.hist.as_slice(),
            merged_to_raw: m2r,
        });

        let divs =
            divergences_from_moves_v1(&segs, &zss, &mvs, self.level_id, macd_ctx.as_ref());
        buysellpoints_from_level(&segs, &zss, &zsb, &mvs, &divs, self.level_id)
    }

    // ── accessors（供 PyO3 层读取结构化快照）──
    pub fn strokes(&self) -> &[Stroke] {
        self.bi.current_strokes()
    }
    pub fn segments(&self) -> &[Segment] {
        &self.prev_segments
    }
    pub fn zhongshus(&self) -> &[Zhongshu] {
        &self.prev_zhongshus
    }
    pub fn moves(&self) -> &[Move] {
        &self.prev_moves
    }
    pub fn buysellpoints(&self) -> &[BuySellPoint] {
        &self.prev_bsps
    }
    pub fn recursive(&self) -> &[LevelSnapshot] {
        &self.recursive
    }
}
