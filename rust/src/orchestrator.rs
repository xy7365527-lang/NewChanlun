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
//! ⚠口径变更（#246 裁定，supersede Lead #84 点3；#277 裁路①、#288 落码 2026-07-26）：
//! 段层相切边界（三笔重叠含端点 `<=`、缺口谓词严格 `>`）与 Python 参考同批切换，
//! 逐位等价在**新口径**下继续成立，相切边界不再 bit-exact 对齐旧口径历史基线
//! （实测段端点零变化，仅 `break_evidence.gap_type` 标签级翻转，下游中枢/走势/
//! BSP 链不读 gap_type）。
//! 裁定书：`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。
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
//! ## 线段层：增量续算（resume≡full，已移植）
//!
//! 笔尾变化时**增量续算**而非全量重算（消除主链 O(strokes²) 瓶颈——每次笔尾变化原
//! O(strokes)，× O(B) 次变化 = O(B·strokes)；BTC 4.6M bars 在 3M+ 后显著变慢的根因）。
//! `SegCheckpoint` 维护不可变稳定前缀 + `segments_from_strokes_v1_into` 原地续算尾部，
//! 每次摊还 O(tail)。稳定性判据移植自 Python `SegmentEngine._update_checkpoint` 的修复版
//! （gap_type 无关：`trigger_k+1+MARGIN ≤ n_strokes`），保证 resume≡full——见 `SegCheckpoint`
//! 文档。bit-exact 已验证（OKLO 447K 逐 bar + DX/GC/CL 400K day-batch 差分，认识论等级：
//! 实现正确性差分测试，真实数据）。

use crate::bi_engine::BiEngine;
use crate::buysellpoint::{buysellpoints_from_level, BuySellPoint};
use crate::divergence::{
    divergences_from_moves_v1, Divergence, MacdCtx, MoveView, SegView, ZsView,
};
use crate::segment_layers::{
    IncrementalSegBsp, IncrementalSegDivergences, IncrementalSegZhongshu,
};
use crate::level::{
    moves_from_level_zhongshus, zhongshu_from_components, CompView, LevelZhongshu,
};
use crate::macd::OnlineMacdState;
use crate::moves::{moves_from_zhongshus, Move};
use crate::ph::{attach_persistence, should_stop_recursion, ZsPriceView};
use crate::segment::{
    segments_from_strokes_v1, segments_from_strokes_v1_into, Segment, MAX_SECOND_SEQ_SCAN,
};
use crate::stroke::{Direction, Stroke};
use crate::zhongshu::{BreakDir, Zhongshu};

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

// ════════════════════════════════════════════════════════════
// 线段层增量检查点（resume≡full，替代每次笔尾变化的 O(strokes²) 全量重算）
// ════════════════════════════════════════════════════════════

/// 线段层增量检查点。
///
/// ## 稳定性判据（移植自 Python `SegmentEngine._update_checkpoint` 的修复版）
///
/// 一段**稳定** ⟺ `confirmed ∧ break_evidence≠None ∧ trigger_k+1+MARGIN ≤ n_strokes`。
/// 判据**与 gap_type 无关**——这是修复 resume≠full 发散（谱系：segment resume 回归）的
/// 关键。段内任一候选缺口分型的第二特征序列扫描窗口（≤MARGIN=`MAX_SECOND_SEQ_SCAN`）
/// 一旦完全展开（`trigger_k+1+MARGIN ≤ n_strokes`），后续新增笔不再改变该段断点。
/// 单调不变量：`trigger_k` 固定、`n_strokes` 单调增 ⟹ 一旦稳定永远稳定。
///
/// ## 复用策略（设计 B：O(tail)/笔尾变化，无 O(stable_count) 前缀拷贝）
///
/// 保留前 `reuse_count = stable_count - 1` 段为**不可变前缀**——orchestrator 原地
/// `truncate` 到此边界后复用（零 clone），从第 `reuse_count` 段（= 最后一个稳定段）的
/// 起始笔 `s0` 用 fresh `FeatureSeqState` 重算：该段被逐位重新发射，其后接尾部。
///
/// 偏移 `-1`（复用 `stable_count-1` 而非 `stable_count`）使 `finalize_last_segment` /
/// `ensure_last_unconfirmed` 可能修改的 `segments.last()` **永远落在重算区**，绝不触碰
/// 复用前缀——绕开 Python 用整段 clean clone 规避的边界污染问题，零拷贝达成同一不变量。
///
/// ## 续扫单调（避免检查点更新自身退化为 O(N²)）
///
/// `stable_count` 随笔单调增；以 `stable_key`（边界段身份）做 O(1) 校验后从上次
/// `stable_count` 续扫，故每次更新摊还 O(新增稳定段数)，全程 O(n_seg)。
struct SegCheckpoint {
    valid: bool,
    /// 不可变复用前缀长度 R = stable_count - 1。
    reuse_count: usize,
    /// resume 起始笔下标 = prev_segments[reuse_count].s0。
    seg_start: usize,
    /// resume 段方向 = prev_segments[reuse_count].direction。
    seg_dir: Direction,
    /// strokes[seg_start] 身份指纹 (i0, i1, p0_bits, p1_bits)——失配回退全量。
    stroke_key: (usize, usize, u64, u64),
    /// 续扫单调状态：上次稳定段数（O(1) 续扫起点）。
    stable_count: usize,
    /// segments[stable_count-1] 身份键 (s0, s1, i0, i1)，O(1) 校验续扫边界。
    stable_key: (usize, usize, usize, usize),
}

impl SegCheckpoint {
    fn new() -> Self {
        SegCheckpoint {
            valid: false,
            reuse_count: 0,
            seg_start: 0,
            seg_dir: Direction::Up,
            stroke_key: (0, 0, 0, 0),
            stable_count: 0,
            stable_key: (0, 0, 0, 0),
        }
    }

    fn reset(&mut self) {
        *self = SegCheckpoint::new();
    }

    /// 永久固定段前缀长度。`segments[0, stable_count)` 跨调用逐位不变
    /// （reuse_count=stable_count-1 不可变 + 第 stable_count-1 段重算恒同）——
    /// 线段级中枢/背驰增量器的 append-only 永久边界来源。单调非减。
    fn stable_count(&self) -> usize {
        self.stable_count
    }

    /// 检查复用是否有效，有效则返回 (reuse_count, seg_start, seg_dir)。
    /// 移植自 Python `_try_resume`（用 to_bits 精确身份比对替代 round(p,8) 防御性比对——
    /// 更严格：仅在 strokes[seg_start] 逐位相同才复用，稳定段不变量保证不误退）。
    fn try_resume(&self, strokes: &[Stroke]) -> Option<(usize, usize, Direction)> {
        if !self.valid {
            return None;
        }
        if self.seg_start >= strokes.len() {
            return None;
        }
        let sk = &strokes[self.seg_start];
        let key = (sk.i0, sk.i1, sk.p0.to_bits(), sk.p1.to_bits());
        if key != self.stroke_key {
            return None;
        }
        Some((self.reuse_count, self.seg_start, self.seg_dir))
    }

    /// 从新线段列表更新检查点。续扫单调 ⟹ 每次摊还 O(新增稳定段数)。
    /// 移植自 Python `_update_checkpoint`，但去掉其"早退避免 segments[:k] 切片"分支——
    /// 设计 B 无前缀切片/拷贝，更新全程 O(1)+O(growth)，去早退反而消除 stroke_key 滞留
    /// 边界（价格漂移但段下标不变时仍刷新身份键，resume 可恢复）。
    fn update(&mut self, segments: &[Segment], strokes: &[Stroke]) {
        let n_strokes = strokes.len();
        let n_seg = segments.len();

        // 续扫起点：O(1) 校验缓存边界仍有效。
        let mut start = self.stable_count;
        if start > n_seg {
            start = 0;
        } else if start > 0 {
            let bnd = &segments[start - 1];
            if (bnd.s0, bnd.s1, bnd.i0, bnd.i1) != self.stable_key {
                start = 0;
            }
        }

        let mut stable_count = start;
        while stable_count < n_seg {
            let seg = &segments[stable_count];
            if !seg.confirmed {
                break;
            }
            let trigger_k = match seg.break_evidence {
                Some(be) => be.trigger_stroke_k,
                None => break,
            };
            if trigger_k + 1 + MAX_SECOND_SEQ_SCAN > n_strokes {
                break;
            }
            stable_count += 1;
        }

        if stable_count == 0 {
            self.valid = false;
            self.stable_count = 0;
            return;
        }

        self.stable_count = stable_count;
        let bnd = &segments[stable_count - 1];
        self.stable_key = (bnd.s0, bnd.s1, bnd.i0, bnd.i1);

        // 设计 B：复用 stable_count-1 段，从最后稳定段的 s0 重算。
        let reuse = stable_count - 1;
        let rseg = &segments[reuse];
        let ss = rseg.s0;
        if ss < n_strokes {
            let sk = &strokes[ss];
            self.reuse_count = reuse;
            self.seg_start = ss;
            self.seg_dir = rseg.direction;
            self.stroke_key = (sk.i0, sk.i1, sk.p0.to_bits(), sk.p1.to_bits());
            self.valid = true;
        } else {
            self.valid = false;
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

/// 走势相等判据（diff 公共前缀）。移植自 `move_state._move_equal`：
/// kind/direction/seg_start/zs_end/settled 全等。
fn move_equal(a: &Move, b: &Move) -> bool {
    a.kind == b.kind
        && a.direction == b.direction
        && a.seg_start == b.seg_start
        && a.zs_end == b.zs_end
        && a.settled == b.settled
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
            // 次级别走势完成判定 = 线段已被破坏（confirmed，第65课）。require_settled 时读。
            settled: s.confirmed,
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
/// level_id/zhongshus/moves 字段 + 递归层买卖点（Piece 2，2026-06-13）。
#[derive(Debug, Clone)]
pub struct LevelSnapshot {
    pub level_id: i64,
    pub zhongshus: Vec<LevelZhongshu>,
    pub moves: Vec<Move>,
    /// 本递归级别（level≥2）的买卖点。次级别走势 = 下级 Move（严格走势类型，非线段）。
    /// 编排者 2026-06-13 区间套架构「Level 1+ confirmed」侧——`level_buysellpoints` 产出，
    /// 提升自 `c_segment_verify::level_bsps`（per_level_bsp.py 复刻）为生产路径。
    pub buysellpoints: Vec<BuySellPoint>,
}

/// 递归级别（level≥2）买卖点：次级别走势 = 下级 Move（i0=first_seg_s0/i1=last_seg_s1，
/// component-index 跨度作 duration，521号拓扑代理力度），中枢 = LevelZhongshu
/// （seg_start=comp_start/seg_end=comp_end/break_seg=break_comp），df_macd=None。
/// 生产路径，逐字复刻 `c_segment_verify::level_bsps`（该处保留为差分对照）。
/// SegView.settled = Move.settled（下级走势是否完成）；递归层不开 require_settled
/// （strictness 来自级别本身 = 严格走势类型，编排者区间套架构 Level 1+ confirmed 侧）。
fn level_buysellpoints(
    prev_moves: &[Move],
    zss: &[LevelZhongshu],
    moves: &[Move],
    level_id: i64,
) -> Vec<BuySellPoint> {
    let segs: Vec<SegView> = prev_moves
        .iter()
        .map(|m| SegView {
            direction: m.direction,
            high: m.high,
            low: m.low,
            i0: m.first_seg_s0,
            i1: m.last_seg_s1,
            settled: m.settled,
        })
        .collect();
    let zsv: Vec<ZsView> = zss
        .iter()
        .map(|z| ZsView {
            zd: z.zd,
            zg: z.zg,
            seg_start: z.comp_start,
            seg_end: z.comp_end,
            settled: z.settled,
        })
        .collect();
    let zs_break: Vec<(bool, BreakDir, i64)> = zss
        .iter()
        .map(|z| (z.settled, z.break_direction, z.break_comp))
        .collect();
    let mvv: Vec<MoveView> = moves
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
        .collect();
    let divs = divergences_from_moves_v1(&segs, &zsv, &mvv, level_id, None);
    buysellpoints_from_level(&segs, &zsv, &zs_break, &mvv, &divs, level_id, false)
}

/// 级别递归引擎。移植自 `recursive_level_engine.RecursiveLevelEngine`。
struct LevelEngine {
    level_id: i64,
    prev_zhongshus: Vec<LevelZhongshu>,
    prev_moves: Vec<Move>,
    /// 本级别买卖点缓存（短路键未变时复用，Piece 2）。
    prev_buysellpoints: Vec<BuySellPoint>,
    last_move_key: Option<MoveTailKey>,
}

impl LevelEngine {
    fn new(level_id: i64) -> Self {
        LevelEngine {
            level_id,
            prev_zhongshus: Vec::new(),
            prev_moves: Vec::new(),
            prev_buysellpoints: Vec::new(),
            last_move_key: None,
        }
    }

    /// 消费下级 moves，产生本级 zhongshus + moves + buysellpoints。移植自 `process_move_snapshot`。
    /// `in_moves` = 下级 moves（= 本级次级别走势，BSP 检测的「segment」载体，Piece 2）。
    fn process(&mut self, in_moves: &[Move]) -> LevelSnapshot {
        let key = move_tail_key(in_moves);
        if self.last_move_key.as_ref() == Some(&key) {
            return LevelSnapshot {
                level_id: self.level_id,
                zhongshus: self.prev_zhongshus.clone(),
                moves: self.prev_moves.clone(),
                buysellpoints: self.prev_buysellpoints.clone(),
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
        // 修复A：num_components = completed 组件数，末组 seg_end 扩展与 level-1 对齐。
        let curr_moves = moves_from_level_zhongshus(&curr_zhongshus, Some(comps.len()));
        let pv = level_zs_price_views(&curr_zhongshus);
        let curr_moves = attach_persistence(&curr_moves, &pv);

        // Piece 2：本级别买卖点（次级别走势 = in_moves = 下级走势类型）。
        let curr_bsps = level_buysellpoints(in_moves, &curr_zhongshus, &curr_moves, self.level_id);

        self.prev_zhongshus = curr_zhongshus.clone();
        self.prev_moves = curr_moves.clone();
        self.prev_buysellpoints = curr_bsps.clone();

        LevelSnapshot {
            level_id: self.level_id,
            zhongshus: curr_zhongshus,
            moves: curr_moves,
            buysellpoints: curr_bsps,
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
    /// 返回 `recomputed`：`false` ⟺ 输入 key 未变（缓存 `self.cached` 逐位不变，**不克隆**——
    /// 调用层经 `cached()` 按引用读取，消除每-bar O(递归结构) 全量克隆的 B-scaling）。
    /// `true` ⟺ 重算并就地更新 `self.cached`；调用层据此门控 recursive_epoch。
    fn process(&mut self, l1_moves: &[Move]) -> bool {
        let in_key = move_tail_key(l1_moves);
        if self.last_input_key.as_ref() == Some(&in_key) {
            // 状态不变 → 缓存逐位不变，无需重算/克隆（调用层按引用读 cached）。
            return false;
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

        self.cached = snapshots; // 就地更新（move，非 clone）
        true
    }

    /// 最近一次递归快照（按引用，零克隆）。供 orchestrator accessor 透传。
    fn cached(&self) -> &[LevelSnapshot] {
        &self.cached
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
    /// 是否计算走势级买卖点 + 递归层。E 版只消费 `current_moves()`（moves 在 bsp 之前算），
    /// 不读 bsp/recursive → 关闭后 process_bar 跳过这两层的 O(n_seg²) 全量重算（纯省，
    /// 不改 moves 输出 ⟹ bit-exact）。I 版需要 → 默认开启。
    enable_bsp: bool,
    macd: Option<OnlineMacdState>,

    // 线段层
    prev_segments: Vec<Segment>,
    last_seg_key: Option<SegTailKey>,
    seg_cp: SegCheckpoint,
    // 中枢层（增量：消除 zhongshu_from_segments 每变化 O(n_seg) 过滤+扫描的 O(n_seg²)）
    inc_seg_zs: IncrementalSegZhongshu,
    last_zs_key: Option<ZsSegKey>,
    // 走势层（全量重算 inc 中枢输出——中枢/走势结构稀疏，per-call O(n_zs) 廉价）
    prev_moves: Vec<Move>,
    last_move_key: Option<MoveZsKey>,
    // 买卖点层
    last_bsp_key: Option<BspKey>,
    /// 增量路径开关 = enable_bsp ∧ ¬enable_macd（背驰纯结构性，per-move 有界回溯）。
    /// enable_macd 时回退全量 `compute_bsps`（macd 背驰依赖原始 bar，不在增量器有效域）。
    use_inc_bsp: bool,
    /// 次级别走势 settle 合取门（编排者 2026-06-13）：level-1 BSP confirmed 合取 anchor
    /// 段 `Segment.confirmed`（走势已完成）。默认 false（在册口径，逐位等价 Python）。
    require_settled_subseg: bool,
    /// 增量背驰（per-move 窗口重算，消除 divergences_from_moves_v1 每变化 O(n_seg) 段遍历）。
    inc_seg_div: IncrementalSegDivergences,
    /// 增量买卖点（IncrementalSegBsp：无状态 binary-search 窗口，对易变 div/move/中枢尾鲁棒）。
    inc_bsp: IncrementalSegBsp,
    /// macd 回退路径的全量买卖点结果（use_inc_bsp=false 时用）。
    prev_bsps: Vec<BuySellPoint>,
    // ── view 镜像（增量维护喂 inc_bsp：截断到稳定边界 + 重推有界尾部，消除每变化 O(n_seg) 构造）──
    seg_view_mirror: Vec<SegView>,
    seg_view_synced: usize,
    zs_view_mirror: Vec<ZsView>,
    zs_break_mirror: Vec<(bool, BreakDir, i64)>,
    zs_view_synced: usize,
    move_view_mirror: Vec<MoveView>,

    // ── 走势 settle delta（消除 E 引擎每-epoch 全量 current_moves() marshal + Python diff_moves）──
    /// 上次 diff 的走势列表（move_settle_delta 的对照基准）。
    move_diff_prev: Vec<Move>,
    /// 本次 delta 的新结算走势（diff_moves 的 MoveSettleV1 等价集，缓存供 accessor 返回）。
    settle_out: Vec<Move>,
    /// 上次 settle delta 见到的 move_epoch（O(1) 门控：未变 ⟹ 无新结算）。
    last_settle_epoch: u64,

    // 递归栈（cached 按引用透传 accessor，无 per-bar self.recursive 克隆字段）
    stack: RecursiveStack,

    // ── 内容变化纪元（epoch）：消除调用层每-bar 全量 marshal 的 B-scaling O(N²) ──
    // 各 epoch 仅在对应层**重算**时自增（重算 ⟺ 结构 key 变化）。调用层缓存上次见到的
    // epoch，epoch 未变 ⟹ prev_* 对象逐位不变 ⟹ 跳过 marshal/scan（O(1) 门控）。
    // 单调不变量：epoch 自增 ⊇ 内容变化（重算可能产出相同内容 → 调用层做无害冗余扫描）。
    move_epoch: u64,
    bsp_epoch: u64,
    recursive_epoch: u64,
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
        enable_bsp: bool,
        require_settled_subseg: bool,
    ) -> Self {
        RecursiveOrchestrator {
            bi: BiEngine::new(stroke_mode, min_strict_sep, reset_dir_on_fractal, new_raw_gap_min),
            level_id: 1,
            enable_macd: enable_macd_divergence,
            enable_bsp,
            macd: if enable_macd_divergence {
                Some(OnlineMacdState::new(12, 26, 9))
            } else {
                None
            },
            prev_segments: Vec::new(),
            last_seg_key: None,
            seg_cp: SegCheckpoint::new(),
            inc_seg_zs: IncrementalSegZhongshu::new(),
            last_zs_key: None,
            prev_moves: Vec::new(),
            last_move_key: None,
            last_bsp_key: None,
            use_inc_bsp: enable_bsp && !enable_macd_divergence,
            require_settled_subseg,
            inc_seg_div: IncrementalSegDivergences::new(),
            inc_bsp: IncrementalSegBsp::new(1, require_settled_subseg),
            prev_bsps: Vec::new(),
            seg_view_mirror: Vec::new(),
            seg_view_synced: 0,
            zs_view_mirror: Vec::new(),
            zs_break_mirror: Vec::new(),
            zs_view_synced: 0,
            move_view_mirror: Vec::new(),
            move_diff_prev: Vec::new(),
            settle_out: Vec::new(),
            last_settle_epoch: 0,
            stack: RecursiveStack::new(max_levels),
            move_epoch: 0,
            bsp_epoch: 0,
            recursive_epoch: 0,
        }
    }

    pub fn reset(&mut self) {
        self.bi.reset();
        if let Some(m) = self.macd.as_mut() {
            m.reset();
        }
        self.prev_segments.clear();
        self.last_seg_key = None;
        self.seg_cp.reset();
        self.inc_seg_zs.reset();
        self.last_zs_key = None;
        self.prev_moves.clear();
        self.last_move_key = None;
        self.last_bsp_key = None;
        self.inc_seg_div.reset();
        self.inc_bsp.reset();
        self.prev_bsps.clear();
        self.seg_view_mirror.clear();
        self.seg_view_synced = 0;
        self.zs_view_mirror.clear();
        self.zs_break_mirror.clear();
        self.zs_view_synced = 0;
        self.move_view_mirror.clear();
        self.move_diff_prev.clear();
        self.settle_out.clear();
        self.last_settle_epoch = 0;
        self.stack.reset();
        self.move_epoch = 0;
        self.bsp_epoch = 0;
        self.recursive_epoch = 0;
    }

    /// 逐 bar 驱动全链。移植自 `RecursiveOrchestrator.process_bar`。
    ///
    /// ⚠口径变更（#246 裁定，supersede Lead #84 点3；#277 裁路①、#288 落码 2026-07-26）：
    /// 本函数线段层经 `segments_from_strokes_v1[_into]` 走 segment.rs 新口径
    /// （相切=重合），与 Python 参考同批切换；逐位等价在新口径下继续成立
    /// （实测段端点零变化，仅 `break_evidence.gap_type` 标签级翻转）。
    /// 裁定书：`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。
    pub fn process_bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        // ── Level=1 五层管线 ──
        self.bi.process_bar(o, h, l, c);

        // MACD 增量（enable 时每 bar 无条件 update，⇔ Python `_compute_macd`：
        // EMA 因果递推每 bar 推进，与 bsp 是否短路无关）。
        if let Some(m) = self.macd.as_mut() {
            m.update(c);
        }

        // 线段层（笔尾变化 → 增量续算；resume≡full，复用不可变前缀避免 O(strokes²)）
        let seg_key = seg_tail_key(self.bi.current_strokes());
        if self.last_seg_key.as_ref() != Some(&seg_key) {
            self.last_seg_key = Some(seg_key);
            let strokes = self.bi.current_strokes();
            // 字段级不相交借用：seg_cp / bi(strokes) / prev_segments 为 self 的不同字段。
            let new_segs = match self.seg_cp.try_resume(strokes) {
                Some((reuse_count, ss, sd)) => {
                    // 原地复用前缀：take 出存储（O(1)）、truncate 到复用边界（O(tail)）、
                    // 从 ss 续算追加。不可变前缀 [0, reuse_count) 不被触碰。
                    let mut work = std::mem::take(&mut self.prev_segments);
                    work.truncate(reuse_count);
                    segments_from_strokes_v1_into(&mut work, strokes, 3, true, ss, sd);
                    work
                }
                None => segments_from_strokes_v1(strokes, 3, true),
            };
            self.seg_cp.update(&new_segs, strokes);
            self.prev_segments = new_segs;
        }
        let n_seg = self.prev_segments.len();

        let stable_count = self.seg_cp.stable_count();

        // 中枢层（增量：稳定段前缀的中枢缓存，易变段尾每次续扫——摊还 O(易变尾)）。
        let zs_key = zs_seg_key(&self.prev_segments);
        if self.last_zs_key.as_ref() != Some(&zs_key) {
            self.last_zs_key = Some(zs_key);
            self.inc_seg_zs.update(&self.prev_segments, stable_count);
        }

        // 走势层（全量重算 inc 中枢输出）。中枢/走势稀疏 → per-call O(n_settled_zs) 廉价
        // （OKLO 447K：n_settled_zs≈580，move 层全程 <10ms，非主导）。
        //
        // 注：曾尝试「settled 结构不变 ⟹ 仅更新末走势 seg_end」的 O(1) 廉价路径，但被真实数据
        // 差分（bar 185656）证伪——settled 中枢的 `last_seg_s1`（锚点）可因 volatile 段修订而变，
        // 而 `move_zs_key` 仅含 component 索引 `seg_end`、不含锚点，settled_eq 误判 ⟹ 留下陈旧
        // `last_seg_s1`。要正确检测须比对整个 pending group 的锚点 = 等价于全量重算。故保持全量
        // （走势稀疏使其非瓶颈；真正 O(1) 增量需 running-aggregate IncrementalSegMoves，benefit≈0）。
        let mv_key = move_zs_key(self.inc_seg_zs.zhongshus(), n_seg as i64);
        if self.last_move_key.as_ref() != Some(&mv_key) {
            self.last_move_key = Some(mv_key);
            let mut moves = moves_from_zhongshus(self.inc_seg_zs.zhongshus(), Some(n_seg));
            let pv = zs_price_views(self.inc_seg_zs.zhongshus());
            moves = attach_persistence(&moves, &pv);
            self.prev_moves = moves;
            self.move_epoch += 1;
        }

        // 买卖点层 + 递归层（仅 enable_bsp 时；E 版关闭以跳过它不消费的 O(n_seg²) 重算）。
        if self.enable_bsp {
            let n_zs = self.inc_seg_zs.zhongshus().len();
            let bk = bsp_key(&self.prev_segments, n_zs, self.prev_moves.len());
            if self.last_bsp_key.as_ref() != Some(&bk) {
                self.last_bsp_key = Some(bk);
                if self.use_inc_bsp {
                    // 增量路径（纯结构性背驰）：view 镜像增量维护 + 背驰/买卖点增量器。
                    self.update_bsps_incremental(n_seg, stable_count);
                } else {
                    // macd 回退路径：全量 compute_bsps（macd 背驰不在增量器有效域）。
                    self.prev_bsps = self.compute_bsps();
                }
                self.bsp_epoch += 1;
            }

            // ── 递归层（level >= 2）：cached 就地更新，无 per-bar 克隆 ──
            let rec_changed = self.stack.process(&self.prev_moves);
            if rec_changed {
                self.recursive_epoch += 1;
            }
        }
    }

    /// 增量买卖点路径（use_inc_bsp）：增量维护 view 镜像 → 增量背驰 → 增量买卖点。
    ///
    /// view 镜像（seg/zs）截断到稳定边界 + 重推有界尾部（摊还 O(n)），消除全量 compute_bsps
    /// 每变化 O(n_seg) 的视图构造 + divergence 全 move 段遍历 + MoveLookup 全段构建。
    /// 逐位等价于 `compute_bsps`（无 macd 分支）——由真实数据端到端差分守卫。
    fn update_bsps_incremental(&mut self, n_seg: usize, stable_count: usize) {
        // ── seg view 镜像（截断到上次稳定边界 + 重推 [synced..n_seg)）──
        let sc = stable_count.min(n_seg);
        if self.seg_view_synced > sc {
            // 防御：稳定边界回退（不应发生）→ 全量重建。
            self.seg_view_mirror.clear();
            self.seg_view_synced = 0;
        }
        self.seg_view_mirror.truncate(self.seg_view_synced);
        for s in &self.prev_segments[self.seg_view_synced..] {
            self.seg_view_mirror.push(SegView {
                direction: s.direction,
                high: s.high,
                low: s.low,
                i0: s.i0,
                i1: s.i1,
                settled: s.confirmed,
            });
        }
        self.seg_view_synced = sc;

        // ── zs view / break 镜像（截断到 inc 中枢稳定前缀 + 重推尾部）──
        let zss = self.inc_seg_zs.zhongshus();
        let zs_stable = self.inc_seg_zs.stable_count().min(zss.len());
        if self.zs_view_synced > zs_stable {
            self.zs_view_mirror.clear();
            self.zs_break_mirror.clear();
            self.zs_view_synced = 0;
        }
        self.zs_view_mirror.truncate(self.zs_view_synced);
        self.zs_break_mirror.truncate(self.zs_view_synced);
        for z in &zss[self.zs_view_synced..] {
            self.zs_view_mirror.push(ZsView {
                zd: z.zd,
                zg: z.zg,
                seg_start: z.seg_start,
                seg_end: z.seg_end,
                settled: z.settled,
            });
            self.zs_break_mirror
                .push((z.settled, z.break_direction, z.break_seg));
        }
        self.zs_view_synced = zs_stable;

        // ── move view 镜像（稀疏，全量重建 O(n_moves)）──
        self.move_view_mirror.clear();
        for m in &self.prev_moves {
            self.move_view_mirror.push(MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            });
        }

        // ── 增量背驰（per-move 窗口重算）+ 增量买卖点（IncrementalBsp 段窗口重算）──
        self.inc_seg_div.update(
            &self.move_view_mirror,
            &self.seg_view_mirror,
            &self.zs_view_mirror,
            self.level_id,
            n_seg,
        );
        // volatile_floor：第一个未 commit move 的 seg_start（moves 空 → 0）。
        // B2 后易变 div 的 seg_c_end 可远落于 stable_anchor 之前，anchor 须以此为上限。
        let volatile_floor = self
            .move_view_mirror
            .get(self.inc_seg_div.processed_moves())
            .map(|m| m.seg_start)
            .unwrap_or(if self.move_view_mirror.is_empty() {
                0
            } else {
                i64::MAX
            });
        self.inc_bsp.update(
            &self.seg_view_mirror,
            &self.zs_view_mirror,
            &self.zs_break_mirror,
            &self.move_view_mirror,
            self.inc_seg_div.current(),
            self.inc_seg_div.stable_len(),
            volatile_floor,
        );
    }

    /// 计算背驰 + 买卖点（含 MACD 路径）。macd 回退路径用（use_inc_bsp=false）。
    fn compute_bsps(&self) -> Vec<BuySellPoint> {
        let segs = seg_views(&self.prev_segments);
        let zss = zs_views(self.inc_seg_zs.zhongshus());
        let zsb = zs_break(self.inc_seg_zs.zhongshus());
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
        buysellpoints_from_level(
            &segs,
            &zss,
            &zsb,
            &mvs,
            &divs,
            self.level_id,
            self.require_settled_subseg,
        )
    }

    /// 走势 settle delta —— 返回自上次以来**新结算**的走势（diff_moves 的 MoveSettleV1 等价集）。
    ///
    /// 逐位等价于 Python `diff_moves(prev, curr)` 中 `MoveSettleV1` 事件对应的 settled move：
    /// 公共前缀（_move_equal: kind/direction/seg_start/zs_end/settled）之后，curr 后缀里
    ///   - 同身份（seg_start 相同）且 prev 未结算→curr 结算 → settle
    ///   - 全新（无同身份 prev）且 curr 结算 → settle
    /// 消除 E 引擎每-epoch 全量 `current_moves()` marshal + Python 端 `diff_moves`（O(B·n_moves)
    /// → 端到端 O(n_seg·新结算)）。`move_epoch` 未变 ⟹ 走势列表逐位不变 ⟹ 无新结算（O(1)）。
    pub fn move_settle_delta(&mut self) -> &[Move] {
        self.settle_out.clear();
        if self.move_epoch == self.last_settle_epoch {
            return &self.settle_out; // 走势未重算 ⟹ 列表不变 ⟹ 无新事件
        }
        self.last_settle_epoch = self.move_epoch;

        let prev = &self.move_diff_prev;
        let curr = &self.prev_moves;
        // 公共前缀（_move_equal）
        let mut common = 0usize;
        let upper = prev.len().min(curr.len());
        while common < upper && move_equal(&prev[common], &curr[common]) {
            common += 1;
        }
        // curr 后缀：同身份升级(F→T) / 全新且 settled → settle
        for i in common..curr.len() {
            let m = &curr[i];
            if i < prev.len() && prev[i].seg_start == m.seg_start {
                // 同身份：仅 prev 未结算→curr 结算 发 settle
                if !prev[i].settled && m.settled {
                    self.settle_out.push(*m);
                }
            } else if m.settled {
                // 全新且结算
                self.settle_out.push(*m);
            }
        }
        self.move_diff_prev = curr.clone();
        &self.settle_out
    }

    // ── accessors（供 PyO3 层读取结构化快照）──
    pub fn strokes(&self) -> &[Stroke] {
        self.bi.current_strokes()
    }
    pub fn segments(&self) -> &[Segment] {
        &self.prev_segments
    }
    /// merged bar → raw bar 范围映射（`merged_to_raw[m] = (raw_start, raw_end)`）。
    ///
    /// 线段端点 `i0/i1` 是 K 线包含处理后的 **merged bar 坐标**；回测要把买卖点对齐到
    /// raw OHLC（逐 bar MtM）须经此映射换算（记忆 `current_strokes_i1_merged_coord`）。
    /// 透传 `bi.merged_to_raw()`——本就是 MACD 背驰路径（raw 索引对齐）所需的同一映射。
    pub fn merged_to_raw(&self) -> &[(usize, usize)] {
        self.bi.merged_to_raw()
    }
    pub fn zhongshus(&self) -> &[Zhongshu] {
        self.inc_seg_zs.zhongshus()
    }
    /// 走势级背驰（inc_seg_div 增量缓存直读；逐位等价于
    /// `divergences_from_moves_v1(.., None)`）。只读 surfacing：背驰本就是
    /// 买卖点层的中间产物（update_bsps_incremental 步骤），此前算完即弃。
    /// 有效域 = `!enable_macd`（macd 回退路径走全量 compute_bsps，
    /// inc_seg_div 不更新——调用方经 `macd_divergence_enabled` 判定）。
    pub fn trend_divergences(&self) -> &[Divergence] {
        self.inc_seg_div.current()
    }
    pub fn macd_divergence_enabled(&self) -> bool {
        self.enable_macd
    }
    pub fn moves(&self) -> &[Move] {
        &self.prev_moves
    }
    pub fn buysellpoints(&self) -> &[BuySellPoint] {
        // 增量路径按引用透传 inc_bsp（零克隆）；macd 回退路径读全量结果。
        if self.use_inc_bsp {
            self.inc_bsp.current()
        } else {
            &self.prev_bsps
        }
    }
    pub fn recursive(&self) -> &[LevelSnapshot] {
        self.stack.cached()
    }

    // ── epoch accessors（内容变化纪元；调用层 O(1) 门控）──
    pub fn move_epoch(&self) -> u64 {
        self.move_epoch
    }
    pub fn bsp_epoch(&self) -> u64 {
        self.bsp_epoch
    }
    pub fn recursive_epoch(&self) -> u64 {
        self.recursive_epoch
    }
}

// ════════════════════════════════════════════════════════════
// 线段层增量检查点：resume≡full 差分测试（in-repo 守护 bit-exact 契约）
// ════════════════════════════════════════════════════════════
//
// 镜像 zhongshu::incremental_tests / moves::incremental_moves_tests 的逐前缀差分模式：
// 笔列表逐步增长（追加维度——正是 `40 vs 42 段` 回归暴露的维度），每个前缀上用
// SegCheckpoint 驱动的 resume 路径（try_resume→truncate→segments_from_strokes_v1_into→
// update，逐字段复刻 process_bar 的线段层）与独立全量 segments_from_strokes_v1 比对。
// 真实数据（OKLO 447K 逐 bar + CL 790K）的尾笔变异维度由 Python 差分脚本
// analysis/_verify_segment_incremental.py 覆盖；此处守护纯追加维度的 resume 机制。
#[cfg(test)]
mod seg_checkpoint_tests {
    use super::*;

    /// 从严格 zigzag 极值价构造交替方向的笔（bi 不变量：方向逐笔交替）。
    /// stroke k 连接 prices[k]→prices[k+1]，high/low=两端 max/min，i0=k、i1=k+1。
    fn mk_strokes(prices: &[f64]) -> Vec<Stroke> {
        let mut out = Vec::new();
        for k in 0..prices.len().saturating_sub(1) {
            let p0 = prices[k];
            let p1 = prices[k + 1];
            let direction = if p1 > p0 { Direction::Up } else { Direction::Down };
            out.push(Stroke {
                i0: k,
                i1: k + 1,
                direction,
                high: p0.max(p1),
                low: p0.min(p1),
                p0,
                p1,
                confirmed: true,
            });
        }
        out
    }

    /// 逐前缀断言：每个前缀长度 m 上，checkpoint resume 路径 == 全量重算。
    /// 跨前缀复用同一 SegCheckpoint/prev_segments（模拟 process_bar 的跨 bar 持久状态）。
    fn assert_resume_matches_full(strokes: &[Stroke]) {
        let mut seg_cp = SegCheckpoint::new();
        let mut prev_segments: Vec<Segment> = Vec::new();

        for m in 1..=strokes.len() {
            let s = &strokes[..m];
            // ── 复刻 process_bar 线段层（笔尾变化时执行）──
            let new_segs = match seg_cp.try_resume(s) {
                Some((reuse_count, ss, sd)) => {
                    let mut work = std::mem::take(&mut prev_segments);
                    work.truncate(reuse_count);
                    segments_from_strokes_v1_into(&mut work, s, 3, true, ss, sd);
                    work
                }
                None => segments_from_strokes_v1(s, 3, true),
            };
            seg_cp.update(&new_segs, s);
            prev_segments = new_segs;

            let full = segments_from_strokes_v1(s, 3, true);
            assert_eq!(
                prev_segments, full,
                "resume≠full 在前缀长度 {m}（resume {} 段 vs full {} 段）",
                prev_segments.len(),
                full.len()
            );
        }
    }

    #[test]
    fn resume_matches_full_simple_zigzag() {
        // 等幅 zigzag：交替方向，相邻笔区间重叠 → 反复形成中枢/线段。
        let prices: Vec<f64> = (0..40)
            .map(|k| if k % 2 == 0 { 100.0 } else { 110.0 })
            .collect();
        assert_resume_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn resume_matches_full_trending_breaks() {
        // 渐进上移 zigzag：制造向上突破（线段断裂）+ 缺口，覆盖 settled 发射路径。
        let mut prices = Vec::new();
        let mut base = 100.0;
        for k in 0..60 {
            if k % 2 == 0 {
                prices.push(base);
            } else {
                prices.push(base + 12.0);
                base += 4.0; // 中心上移 → 后续低点抬升 → 突破前段
            }
        }
        assert_resume_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn resume_matches_full_lcg_stress() {
        // 确定性 LCG zigzag：随机游走中心 + 随机半幅，覆盖 overlap/break/extend/缺口/
        // 第二特征序列各路径。400 笔触发多次稳定前缀推进，压测续扫单调 + truncate 复用。
        let mut state: u64 = 0xD1B54A32D192ED03;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64 // [0,1)
        };
        let mut prices = Vec::new();
        let mut center = 100.0_f64;
        for k in 0..400 {
            center += (next() - 0.5) * 6.0; // 随机游走中心
            let half = 1.0 + next() * 7.0; // 随机半幅
            // 偶数 idx 取低点、奇数取高点 → 严格 zigzag
            if k % 2 == 0 {
                prices.push(center - half);
            } else {
                prices.push(center + half);
            }
        }
        assert_resume_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn resume_matches_full_under_three_strokes() {
        // <3 笔 → 恒空；resume 与 full 均空。
        let prices = vec![100.0, 110.0];
        assert_resume_matches_full(&mk_strokes(&prices));
    }
}
