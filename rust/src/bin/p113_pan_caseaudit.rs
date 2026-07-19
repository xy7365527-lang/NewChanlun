//! task #113：盘背逐案判别探针（只读，不改任何生产源码，不改 Cargo.toml——bin 自动发现）。
//!
//! 依据 `chanlun/review-results/doc-pan-divergence-doctrine-20260717.md` §6 的两套可机械化
//! 判别式，对 p109（`p109-chain157-gate-attrition-20260717.md`）两笔『待裁定』做逐案实证：
//!
//! **Part A（背驰确认门 1,020 条，pan MACD 面积失败）**——§6.1 判别式。生产谓词 =
//! `segments_diverge`（混合柱 Σ|hist| 严格 curr<prev，`divergence.rs:243-245,266-274`）。
//! 逐案计算五个原文信号：
//!   1. 同色柱面积之和 C<A（教义面积口径：060:44 / 057:40 / 024:40『全部同色柱子面积之和』）；
//!   2. 黄白线不创新极值（026:521；027:32 加 B 域回抽0轴前提作严口径单列；056:60 ≥1分钟并看）；
//!   3. 柱高不突破（025:16 / 025:38『柱子伸长的高度』）；
//!   4. 价格几何（027:30-31 幅度/速度先例）。
//!   任一成立 ⟹ 原文下有盘背信号，判负 = 谓词过严（聋度）；全部不成立 ⟹ 市场事实。
//!
//! **Part B（身份门 Cons-leave 1,238 条，pan 定位错位）**——§6.2 判别式。复刻生产
//! `locate_pan_div_structure`（`signal.rs:572-629`）为失败原子细分版，对每对
//! （盘整块中枢 × 触发段）记录击杀要件：①C 破核心 ②A 锚（前次同向破核心段）
//!   ③回中枢段 ④Extreme 新极值（`signal.rs:632-653`）。
//!   对 locate 失败或 Extreme 判负的尝试，候选 A 扩至『中枢前最近同向段』
//!   （049:36-38『最标准』≠唯一；080:364；061:28 中枢两头比较）重跑定位+Extreme：
//!   - 扩 A 后定位 ∧ Extreme 过 ⟹ A 锚定窄化（聋度候选）；
//!   - C 未破核心 ⟹ 中枢震荡域（024:36① / 037:150-152；不计入盘背链候选，单独计数）；
//!   - Extreme 判负维持（044:234），按 038:192 标注『不创新高/新低型力度衰竭』。
//!
//! **硬锚（探针 = 生产路径，任一失败即结论无效）**：
//!   1. provider 事件总数复现 P109_ANCHOR_EVENTS（candidates=3683 confirmed=1235
//!      trend_confirmed=429，`p109-chain157-gate-attrition-20260717.md:67`）；
//!   2. instrument 复刻扫描的『locate 成功 ∧ Extreme 过』事件集 == 生产
//!      `provide_nest_candidate_events` 的 Cons 事件集（(level,turn,seg_a,seg_c,side) 双向 diff=0）；
//!   3. 逐链门级 attrition 复现 P109_GATE（identity=3790 / divergence=1020）与
//!      P109_GATE_SUB identity leave_consolidation=1238；
//!   4. 面积对账：!confirmed 事件的混合柱面积 C<A 计数 == 0（与 confirmed=false 逐位一致）。
//!
//! 用法：
//! `cargo run --release --features backtest_bin --bin p113_pan_caseaudit -- <btc_1m_full.json>`
//! 冒烟：`P113_MAX_BARS=250000 ...`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose::{center_block_kind, MoveBlock};
use newchan_rust::theta_v0::classifier::divergence::{
    compute_macd, departure_move_c_start, segment_dif_peak, segment_macd_area,
    segment_price_amplitude, segment_price_speed, self_anchors, MacdSeries,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CompletionEvidence,
    CompletionStatus, CoordinateWindow, LevelViewMaterial, LevelViewQuery, LowerLeg,
    NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{is_sub, NestInterval};
use newchan_rust::theta_v0::classifier::recursive_tower::{map_src_to_close_idx, LeveledMove};
use newchan_rust::theta_v0::classifier::Classification;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{
    quantize, Bar, BspBits, Center, Direction, MoveKind, Segment, Side, Tick, Timestamp,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// §0. 复刻生产私有件（只读镜像；硬锚 2 锁定与生产逐位一致）
// ═══════════════════════════════════════════════════════════════════════════

/// 镜像 `signal.rs:208-215 nearest_confirmed_center_idx`（pub(crate) 不可达，逐行同构）。
fn nearest_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(hi - 1)
    }
}

/// 镜像 `level_view.rs:431-443 leg_as_segment`（私有 fn 不可达，逐行同构）。
fn leg_to_segment(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
}

/// 镜像 `level_view.rs:492-504 range_envelope` / `signal.rs:632-653` 的包络原语。
fn span_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    segments
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc: Option<(Tick, Tick)>, segment| {
            let lo = segment.start_price.min(segment.end_price);
            let hi = segment.start_price.max(segment.end_price);
            Some(match acc {
                None => (lo, hi),
                Some((old_lo, old_hi)) => (old_lo.min(lo), old_hi.max(hi)),
            })
        })
}

/// Extreme 比较（`pan_div_structure_extreme` 的 match 分量）：C 包络越 A 包络。
fn extreme_beyond(segments: &[Segment], seg_a: (usize, usize), seg_c: (usize, usize), side: Side) -> bool {
    let (Some(a), Some(c)) = (span_envelope(segments, seg_a), span_envelope(segments, seg_c)) else {
        return false;
    };
    match side {
        Side::Long => c.0 < a.0,
        Side::Short => c.1 > a.1,
    }
}

/// locate 失败原子（§6.2 几何审计的分类基元）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LocateFail {
    /// ① seg 端点未破中枢核心（Down 端点 ≥ zd / Up 端点 ≤ zg）——中枢震荡域候选。
    NoCoreBreak,
    /// ③前半：窗口内无回中枢段（只有一次离开，无 A/C 对）。
    NoReenter,
    /// ② A 锚缺失：前一次同向且自身破核心的离开段不存在。
    NoAAnchor,
    /// ③后半：A、C 之间无回中枢段（同一次离开）。
    NoReenterBetween,
    /// 其他（lambda_c/lambda_a/rho_a 无法定位；生产亦 None，单列不报教义分类）。
    OtherFail,
}

/// 复刻 `signal.rs:560-566 PanDivStructure` 的判定相关字段。
#[derive(Debug, Clone, Copy)]
struct PanStructure {
    source_index: usize,
    side: Side,
    dir: Direction,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
}

/// 镜像 `signal.rs:572-629 locate_pan_div_structure`：判定逻辑逐行同构，
/// 唯一差异 = 失败时返回原子分类（生产返回 None 不细分）。
fn locate_pan(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
) -> Result<PanStructure, LocateFail> {
    // 要件①：破中枢核心（signal.rs:578-583）。
    let side = match seg.direction {
        Direction::Down if seg.end_price < c.zd => Side::Long,
        Direction::Up if seg.end_price > c.zg => Side::Short,
        _ => return Err(LocateFail::NoCoreBreak),
    };
    let dir = seg.direction;
    let lo = segments.partition_point(|s| s.start_index < c.end_index);
    let hi = segments.partition_point(|s| s.start_index <= seg.start_index);
    let win = &segments[lo..hi];
    let reenters = |s: &Segment| {
        s.direction != dir
            && match dir {
                Direction::Down => s.end_price >= c.zd,
                Direction::Up => s.end_price <= c.zg,
            }
    };
    // 要件③前半（signal.rs:595）。
    if !win
        .iter()
        .rev()
        .filter(|s| s.end_index <= seg.start_index)
        .any(|s| reenters(s))
    {
        return Err(LocateFail::NoReenter);
    }
    let lambda_c = match departure_move_c_start(segments, anchors_self, c, dir, seg.start_index) {
        Some(v) => v,
        None => return Err(LocateFail::OtherFail),
    };
    // 要件②：A = 前一次同向且自身破核心的离开段（signal.rs:597-604）。
    let Some(a_anchor) = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.end_index <= lambda_c)
        .find(|s| match dir {
            Direction::Down => s.end_price < c.zd,
            Direction::Up => s.end_price > c.zg,
        })
    else {
        return Err(LocateFail::NoAAnchor);
    };
    // 要件③后半：A、C 间存在回中枢段（signal.rs:605-610）。
    if !win
        .iter()
        .any(|s| reenters(s) && s.start_index >= a_anchor.end_index && s.end_index <= lambda_c)
    {
        return Err(LocateFail::NoReenterBetween);
    }
    let lambda_a = match departure_move_c_start(segments, anchors_self, c, dir, a_anchor.start_index)
    {
        Some(v) => v,
        None => return Err(LocateFail::OtherFail),
    };
    let episode_end = win
        .iter()
        .find(|s| reenters(s) && s.start_index >= a_anchor.end_index)
        .map_or(lambda_c, |r| r.start_index);
    let Some(rho_a) = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.start_index >= lambda_a && s.end_index <= episode_end)
        .map(|s| s.end_index)
        .next()
    else {
        return Err(LocateFail::OtherFail);
    };
    Ok(PanStructure {
        source_index: seg.end_index,
        side,
        dir,
        seg_a: (lambda_a, rho_a),
        seg_c: (lambda_c, seg.end_index),
    })
}

/// 『中枢前最近同向段』查询结构（§6.2-1 候选 A 扩展的 O(log n) 支撑）。
/// 对全部段按 end_index 升序建前缀最近同向段索引：Up/Down 各一条链。
struct PrevSameDir {
    order: Vec<usize>,
    ends: Vec<usize>,
    prev: [Vec<Option<usize>>; 2], // [Down, Up]：prev[d][k] = order[..=k] 中最近方向 d 段的 order 位置
}

impl PrevSameDir {
    fn build(segs: &[Segment]) -> Self {
        let mut order: Vec<usize> = (0..segs.len()).collect();
        order.sort_by_key(|&i| (segs[i].end_index, i));
        let ends: Vec<usize> = order.iter().map(|&i| segs[i].end_index).collect();
        let mut prev = [vec![None; order.len()], vec![None; order.len()]];
        let mut last: [Option<usize>; 2] = [None, None];
        for (k, &i) in order.iter().enumerate() {
            let d = match segs[i].direction {
                Direction::Down => 0,
                Direction::Up => 1,
            };
            last[d] = Some(k);
            prev[0][k] = last[0];
            prev[1][k] = last[1];
        }
        Self { order, ends, prev }
    }

    /// 最近方向 dir 且 end_index <= before_end 的段（segs 下标）。
    fn find(&self, dir: Direction, before_end: usize) -> Option<usize> {
        let k = self.ends.partition_point(|&e| e <= before_end);
        if k == 0 {
            return None;
        }
        let d = match dir {
            Direction::Down => 0,
            Direction::Up => 1,
        };
        self.prev[d][k - 1].map(|pos| self.order[pos])
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §1. 审计数据结构
// ═══════════════════════════════════════════════════════════════════════════

/// 一次 pan 定位尝试（中枢 × 触发段）的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttemptOutcome {
    /// locate 成功 ∧ Extreme 过（应逐位等于 provider Cons 事件）。
    Success,
    /// locate 成功但 Extreme 判负（要件④，044:234 维持判负域）。
    ExtremeFail,
    /// ① C 未破核心（中枢震荡域，024:36①/037:150-152）。
    NoCoreBreak,
    /// ③前半 无回中枢段。
    NoReenter,
    /// ② A 锚缺失。
    NoAAnchor,
    /// ③后半 A、C 间无回中枢段。
    NoReenterBetween,
    /// 其他定位失败。
    OtherFail,
}

/// §6.2-1 扩 A 重试结果（中枢前最近同向段为锚）。
#[derive(Debug, Clone, Copy)]
struct WideA {
    a_prime: (usize, usize),
    seg_c: (usize, usize),
    extreme_ok: bool,
}

#[derive(Debug, Clone, Copy)]
struct PanAttempt {
    center_idx: usize,
    block_span: (usize, usize),
    seg: (usize, usize),
    dir: Direction,
    outcome: AttemptOutcome,
    /// locate 成功时（Success/ExtremeFail）的结构坐标 (seg_a, seg_c)。
    structure: Option<((usize, usize), (usize, usize))>,
    wide_a: Option<WideA>,
}

/// 严格 C2 pair（同 p109 PairRec + leave 块 source span）。
#[derive(Debug, Clone, Copy)]
struct PairRec {
    start: usize,
    end: usize,
    leave_kind: MoveKind,
    leave_dir: Option<Direction>,
    leave_via_divergence: bool,
    retest_kind: MoveKind,
    leave_span: (usize, usize),
}

impl PairRec {
    fn iv(&self) -> NestInterval {
        NestInterval {
            start_time: self.start as u64,
            end_time: self.end as u64,
            idx: 0,
        }
    }
}

#[derive(Debug, Default)]
struct LevelData {
    tower_windows: usize,
    runs: usize,
    invalid_windows: usize,
    moves: usize,
    completed: usize,
    pairs: Vec<PairRec>,
    events: Vec<NestCandidateEvent>,
    pan_attempts: Vec<PanAttempt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillGate {
    Identity,
    Divergence,
    Terminal,
    Direction,
    Inclusion,
    Alive,
}

impl KillGate {
    fn label(self) -> &'static str {
        match self {
            KillGate::Identity => "identity",
            KillGate::Divergence => "divergence",
            KillGate::Terminal => "terminal",
            KillGate::Direction => "direction",
            KillGate::Inclusion => "inclusion",
            KillGate::Alive => "alive",
        }
    }
}

#[derive(Debug)]
struct Outcome {
    id: usize,
    gate: KillGate,
    kill_level: usize,
    pairs: Vec<PairRec>,
    /// kill=Divergence 时的 L1 基例事件下标（levels[0].events）。
    base_events: Vec<usize>,
}

// ═══════════════════════════════════════════════════════════════════════════
// §2. 单级测量（fork p109 measure_level：V3 生产口径 + pan instrument 扫描）
// ═══════════════════════════════════════════════════════════════════════════

fn measure_level(
    level: usize,
    windows: &[LeveledMove],
    lower_legs: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<LevelData, String> {
    let mut data = LevelData {
        tower_windows: windows.len(),
        ..LevelData::default()
    };
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && match project_extended_windows_carried_only(std::slice::from_ref(&windows[index])) {
                Ok(_) => true,
                Err(_) => {
                    data.invalid_windows += 1;
                    false
                }
            };
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L{level} run 投影失败: {error:?}"))?;
                let centers: Vec<Center> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks: Vec<MoveBlock> =
                    newchan_rust::theta_v0::classifier::decompose::decompose(&centers);
                let query = LevelViewQuery {
                    level: level as u32,
                    coordinate_window: CoordinateWindow {
                        start: projection.seeds.first().expect("nonempty run").start_index,
                        end: projection.seeds.last().expect("nonempty run").end_index,
                    },
                    as_of,
                    version: C2VersionTuple::auto_pairing(),
                };
                let view = assemble_level_view(
                    C2LevelViewConfig { enabled: true },
                    query,
                    LevelViewMaterial {
                        projection: ProjectionMaterial::ExactThree(&projection),
                        move_blocks: &blocks,
                        lower_legs,
                        hist,
                        dif,
                        close_src,
                    },
                )
                .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
                data.runs += 1;
                data.moves += view.moves.len();
                data.completed += view
                    .moves
                    .iter()
                    .filter(|m| matches!(m.completion, CompletionStatus::Completed { .. }))
                    .count();
                for (wi, pair) in view.moves.windows(2).enumerate() {
                    let (
                        CompletionStatus::Completed { evidence: ev0, .. },
                        CompletionStatus::Completed { .. },
                    ) = (&pair[0].completion, &pair[1].completion)
                    else {
                        continue;
                    };
                    // moves 与 blocks 一一对应（level_view.rs:837-898 无跳过）：
                    // leave 块 = blocks[wi]，span 取 seeds 端点（structural_block_span 同口径）。
                    let lb = &blocks[wi];
                    let leave_span = (
                        projection.seeds[lb.start_center].start_index,
                        projection.seeds[lb.end_center].end_index,
                    );
                    data.pairs.push(PairRec {
                        start: pair[0].start_index,
                        end: pair[1].end_index,
                        leave_kind: pair[0].kind,
                        leave_dir: pair[0].direction,
                        retest_kind: pair[1].kind,
                        leave_via_divergence: matches!(ev0, CompletionEvidence::TerminalDivergence { .. }),
                        leave_span,
                    });
                }
                // provider 事件（生产原函数，不改）。
                data.events.extend(provide_nest_candidate_events(
                    level as u32,
                    &projection,
                    &blocks,
                    lower_legs,
                    &view,
                    hist,
                    dif,
                    close_src,
                ));
                // ── instrument pan 扫描（镜像 level_view.rs:596-656 的 Cons 部分 + 失败原子）──
                let segments: Vec<Segment> = lower_legs.iter().map(leg_to_segment).collect();
                let anchors = self_anchors(&segments);
                let kinds = center_block_kind(centers.len(), &blocks);
                let prev_same = PrevSameDir::build(&segments);
                for seg in segments.iter().filter(|s| s.end_index <= as_of) {
                    let Some(ci) = nearest_center_idx(&centers, seg.start_index) else {
                        continue;
                    };
                    if kinds.get(ci) != Some(&Some(MoveKind::Consolidation)) {
                        continue;
                    }
                    // ownership 反查块（center_block_kind_at 同规则：start<ci<=end；ci=0 归首块）。
                    let bi = if ci == 0 {
                        0
                    } else {
                        blocks.partition_point(|b| b.end_center < ci)
                    };
                    let Some(block) = blocks.get(bi).filter(|b| ci == 0 || b.start_center < ci)
                    else {
                        continue;
                    };
                    let block_span = (
                        projection.seeds[block.start_center].start_index,
                        projection.seeds[block.end_center].end_index,
                    );
                    let c = &centers[ci];
                    let dir = seg.direction;
                    let side = match dir {
                        Direction::Down => Side::Long,
                        Direction::Up => Side::Short,
                    };
                    let attempt = match locate_pan(c, seg, &segments, &anchors) {
                        Ok(st) => {
                            if extreme_beyond(&segments, st.seg_a, st.seg_c, st.side) {
                                PanAttempt {
                                    center_idx: ci,
                                    block_span,
                                    seg: (seg.start_index, seg.end_index),
                                    dir,
                                    outcome: AttemptOutcome::Success,
                                    structure: Some((st.seg_a, st.seg_c)),
                                    wide_a: None,
                                }
                            } else {
                                // Extreme 判负：扩 A 重试（A' 极值可能浅于 A，049:36-38）。
                                let wide_a = prev_same.find(dir, c.start_index).map(|ai| {
                                    let ap = (
                                        segments[ai].start_index,
                                        segments[ai].end_index,
                                    );
                                    WideA {
                                        a_prime: ap,
                                        seg_c: st.seg_c,
                                        extreme_ok: extreme_beyond(&segments, ap, st.seg_c, side),
                                    }
                                });
                                PanAttempt {
                                    center_idx: ci,
                                    block_span,
                                    seg: (seg.start_index, seg.end_index),
                                    dir,
                                    outcome: AttemptOutcome::ExtremeFail,
                                    structure: Some((st.seg_a, st.seg_c)),
                                    wide_a,
                                }
                            }
                        }
                        Err(LocateFail::NoCoreBreak) => PanAttempt {
                            center_idx: ci,
                            block_span,
                            seg: (seg.start_index, seg.end_index),
                            dir,
                            outcome: AttemptOutcome::NoCoreBreak,
                            structure: None,
                            wide_a: None,
                        },
                        Err(f @ (LocateFail::NoReenter
                        | LocateFail::NoAAnchor
                        | LocateFail::NoReenterBetween)) => {
                            // §6.2-1 扩 A：C 仍取当前离开 episode（λ_C 可独立定位），
                            // A' = 中枢前最近同向段（061:28 中枢两头比较形态 A'→c→C）。
                            let outcome = match f {
                                LocateFail::NoReenter => AttemptOutcome::NoReenter,
                                LocateFail::NoAAnchor => AttemptOutcome::NoAAnchor,
                                _ => AttemptOutcome::NoReenterBetween,
                            };
                            let wide_a = (|| {
                                let lambda_c = departure_move_c_start(
                                    &segments,
                                    &anchors,
                                    c,
                                    dir,
                                    seg.start_index,
                                )?;
                                let seg_c = (lambda_c, seg.end_index);
                                let ai = prev_same.find(dir, c.start_index)?;
                                let ap = (segments[ai].start_index, segments[ai].end_index);
                                Some(WideA {
                                    a_prime: ap,
                                    seg_c,
                                    extreme_ok: extreme_beyond(&segments, ap, seg_c, side),
                                })
                            })();
                            PanAttempt {
                                center_idx: ci,
                                block_span,
                                seg: (seg.start_index, seg.end_index),
                                dir,
                                outcome,
                                structure: None,
                                wide_a,
                            }
                        }
                        Err(_) => PanAttempt {
                            center_idx: ci,
                            block_span,
                            seg: (seg.start_index, seg.end_index),
                            dir,
                            outcome: AttemptOutcome::OtherFail,
                            structure: None,
                            wide_a: None,
                        },
                    };
                    data.pan_attempts.push(attempt);
                }
                run_start = None;
            }
            _ => {}
        }
    }
    // 与 p109 同 dedup 键序。
    data.events.sort_by_key(|event| {
        (
            event.level,
            matches!(event.side, Side::Short),
            event.kind,
            event.seg_a,
            event.interval_b,
            event.interval_a,
            event.turn_source,
        )
    });
    data.events.dedup_by(|left, right| {
        (
            left.level,
            left.side,
            left.kind,
            left.seg_a,
            left.interval_b,
            left.interval_a,
            left.turn_source,
        ) == (
            right.level,
            right.side,
            right.kind,
            right.seg_a,
            right.interval_b,
            right.interval_a,
            right.turn_source,
        )
    });
    Ok(data)
}

// ═══════════════════════════════════════════════════════════════════════════
// §3. 链枚举与逐门对账（复刻 p109 管线序，精简字段）
// ═══════════════════════════════════════════════════════════════════════════

fn chain_dp_count(levels: &[LevelData]) -> u128 {
    if levels.is_empty() {
        return 0;
    }
    let mut previous: Vec<(usize, u128)> = levels[0].pairs.iter().map(|_| (1usize, 1u128)).collect();
    for level_index in 1..levels.len() {
        let lower_pairs = &levels[level_index - 1].pairs;
        let mut current = Vec::with_capacity(levels[level_index].pairs.len());
        for parent in &levels[level_index].pairs {
            let parent_iv = parent.iv();
            let best = lower_pairs
                .iter()
                .enumerate()
                .filter(|(_, child)| is_sub(&child.iv(), &parent_iv))
                .map(|(ci, _)| previous[ci].0)
                .max()
                .unwrap_or(0);
            let (depth, count) = if best == 0 {
                (1, 1)
            } else {
                let count = lower_pairs
                    .iter()
                    .enumerate()
                    .filter(|(ci, child)| {
                        previous[*ci].0 == best && is_sub(&child.iv(), &parent_iv)
                    })
                    .map(|(ci, _)| previous[ci].1)
                    .sum();
                (best + 1, count)
            };
            current.push((depth, count));
        }
        previous = current;
    }
    previous
        .iter()
        .filter(|v| v.0 == levels.len())
        .map(|v| v.1)
        .sum()
}

fn enumerate_chains(levels: &[LevelData], cap: usize) -> (Vec<Vec<(usize, usize)>>, bool) {
    let n = levels.len();
    let mut out: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut truncated = false;
    let mut parents: Vec<Vec<Vec<usize>>> = Vec::with_capacity(n - 1);
    for k in 0..n - 1 {
        let mut edges = Vec::with_capacity(levels[k].pairs.len());
        for child in &levels[k].pairs {
            let mut ps = Vec::new();
            for (pi, parent) in levels[k + 1].pairs.iter().enumerate() {
                if is_sub(&child.iv(), &parent.iv()) {
                    ps.push(pi);
                }
            }
            edges.push(ps);
        }
        parents.push(edges);
    }
    for first in 0..levels[0].pairs.len() {
        let mut stack: Vec<(usize, usize)> = vec![(0usize, first)];
        let mut path: Vec<(usize, usize)> = Vec::new();
        while let Some((k, pi)) = stack.pop() {
            path.truncate(k);
            path.push((k, pi));
            if k + 1 == n {
                out.push(path.clone());
                if out.len() >= cap {
                    truncated = true;
                    break;
                }
                continue;
            }
            for &pj in parents[k][pi].iter().rev() {
                stack.push((k + 1, pj));
            }
        }
        if truncated {
            break;
        }
    }
    (out, truncated)
}

fn terminal_bits_new(classification: &Classification, event: &NestCandidateEvent) -> Option<BspBits> {
    classification
        .levels
        .get(event.level as usize)?
        .bsp
        .iter()
        .find(|point| point.source_index == event.turn_source && point.bits.confirm_side(event.side))
        .map(|point| point.bits)
}

/// 逐门对账：管线序 = identity → divergence → terminal → direction/inclusion（同 p109）。
fn reconcile(
    id: usize,
    chain: &[(usize, usize)],
    levels: &[LevelData],
    match_maps: &[BTreeMap<(usize, usize), Vec<usize>>],
    classification: &Classification,
) -> Outcome {
    let n = chain.len();
    let pairs: Vec<PairRec> = chain.iter().map(|(k, pi)| levels[*k].pairs[*pi]).collect();
    let matched = |k: usize| -> Vec<usize> {
        let (lk, pi) = chain[k];
        let pair = &levels[lk].pairs[pi];
        match_maps[lk]
            .get(&(pair.start, pair.end))
            .cloned()
            .unwrap_or_default()
    };
    let events_of = |k: usize| -> &Vec<NestCandidateEvent> { &levels[chain[k].0].events };

    // 门 1：身份门@L1。
    let base_candidates = matched(0);
    if base_candidates.is_empty() {
        return Outcome {
            id,
            gate: KillGate::Identity,
            kill_level: 1,
            pairs,
            base_events: Vec::new(),
        };
    }
    // 门 2：背驰确认门@L1。
    let div_bases: Vec<usize> = base_candidates
        .iter()
        .copied()
        .filter(|&i| events_of(0)[i].divergence_confirmed)
        .collect();
    if div_bases.is_empty() {
        return Outcome {
            id,
            gate: KillGate::Divergence,
            kill_level: 1,
            pairs,
            base_events: base_candidates,
        };
    }
    // 门 3：终端门@L1。
    let term_bases: Vec<usize> = div_bases
        .iter()
        .copied()
        .filter(|&i| terminal_bits_new(classification, &events_of(0)[i]).is_some())
        .collect();
    if term_bases.is_empty() {
        return Outcome {
            id,
            gate: KillGate::Terminal,
            kill_level: 1,
            pairs,
            base_events: Vec::new(),
        };
    }
    // 门 4：逐级边（方向 ∧ 包含）。
    let mut frontier: Vec<Vec<usize>> = term_bases.iter().map(|&i| vec![i]).collect();
    for k in 1..n {
        let level_matches = matched(k);
        let mut next: Vec<Vec<usize>> = Vec::new();
        let mut any_identity = false;
        let mut any_side = false;
        for path in &frontier {
            let side = events_of(0)[path[0]].side;
            if level_matches.is_empty() {
                continue;
            }
            any_identity = true;
            let pool: Vec<usize> = level_matches
                .iter()
                .copied()
                .filter(|&i| events_of(k)[i].side == side)
                .collect();
            if pool.is_empty() {
                continue;
            }
            any_side = true;
            let child_iv = typed_b(&events_of(k - 1)[*path.last().expect("nonempty path")]);
            for i in pool {
                if is_sub(&child_iv, &typed_b(&events_of(k)[i])) {
                    let mut extended = path.clone();
                    extended.push(i);
                    next.push(extended);
                }
            }
        }
        if next.is_empty() {
            let gate = if !any_identity {
                KillGate::Identity
            } else if !any_side {
                KillGate::Direction
            } else {
                KillGate::Inclusion
            };
            return Outcome {
                id,
                gate,
                kill_level: k + 1,
                pairs,
                base_events: Vec::new(),
            };
        }
        frontier = next;
        if frontier.len() > 50_000 {
            frontier.truncate(50_000);
        }
    }
    Outcome {
        id,
        gate: KillGate::Alive,
        kill_level: n,
        pairs,
        base_events: Vec::new(),
    }
}

fn typed_b(event: &NestCandidateEvent) -> NestInterval {
    NestInterval {
        start_time: event.interval_b.0 as u64,
        end_time: event.interval_b.1 as u64,
        idx: event.turn_source as u64,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4. Part A：§6.1 逐案信号（同色面积 / 黄白线 / 柱高 / 价格几何）
// ═══════════════════════════════════════════════════════════════════════════

/// 同色柱面积（教义口径 060:44/057:40/024:40）：Long=向下离开 → 绿柱（hist<0）绝对值和；
/// Short=向上离开 → 红柱（hist>0）和。
fn same_color_area(hist: &[f64], lo: usize, hi: usize, side: Side) -> f64 {
    if lo > hi || hi >= hist.len() {
        return 0.0;
    }
    let mut area = 0.0;
    for &h in &hist[lo..=hi] {
        match side {
            Side::Long if h < 0.0 => area += h.abs(),
            Side::Short if h > 0.0 => area += h,
            _ => {}
        }
    }
    area
}

/// 同向柱峰（025:16/025:38『柱子伸长的高度』）：Long → 段内最深绿柱 |min|；Short → 最高红柱 max。
fn same_dir_hist_peak(hist: &[f64], lo: usize, hi: usize, side: Side) -> f64 {
    if lo > hi || hi >= hist.len() {
        return 0.0;
    }
    match side {
        Side::Long => hist[lo..=hi]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .min(0.0)
            .abs(),
        Side::Short => hist[lo..=hi]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            .max(0.0),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CaseSignals {
    area_abs_a: f64,
    area_abs_c: f64,
    area_same_a: f64,
    area_same_c: f64,
    dif_peak_a: f64,
    dif_peak_c: f64,
    /// B 域回抽量（Long=max(dif[B])，Short=-min(dif[B])；B 域空=None 以 NaN 标记不参与）。
    b_pullback: Option<f64>,
    hist_peak_a: f64,
    hist_peak_c: f64,
    amp_a: i64,
    amp_c: i64,
    speed_a: f64,
    speed_c: f64,
}

impl CaseSignals {
    fn s_area_same(&self) -> bool {
        self.area_same_c < self.area_same_a
    }
    fn s_dif(&self) -> bool {
        self.dif_peak_c < self.dif_peak_a
    }
    /// 027:32 严口径：B 域回抽0轴 ∧ 黄白线不创新极值。
    fn s_dif_pb(&self) -> bool {
        matches!(self.b_pullback, Some(pb) if pb >= 0.0) && self.s_dif()
    }
    fn s_hist(&self) -> bool {
        self.hist_peak_c < self.hist_peak_a
    }
    fn s_amp(&self) -> bool {
        self.amp_c < self.amp_a
    }
    fn s_speed(&self) -> bool {
        self.speed_c < self.speed_a
    }
    /// §6.1 主判定：任一原文信号成立（面积口径复核 + 2-4 项）。
    fn deaf_or(&self) -> bool {
        self.s_area_same() || self.s_dif() || self.s_hist() || self.s_amp()
    }
    /// 027:32 原文两项：『回抽0轴的黄白线再次下跌不创新低』∨『柱子面积明显小』。
    fn doc_02732(&self) -> bool {
        self.s_dif_pb() || self.s_area_same()
    }
}

fn case_signals(
    event: &NestCandidateEvent,
    macd: &MacdSeries,
    closes_tick: &[Tick],
    close_src: &[usize],
) -> Option<CaseSignals> {
    signals_for(
        event.side,
        event.seg_a,
        event.interval_b,
        macd,
        closes_tick,
        close_src,
    )
}

/// §6.1 信号核心（side + A/C 段坐标；扩 A 结构复用同一判别式）。
fn signals_for(
    side: Side,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
    macd: &MacdSeries,
    closes_tick: &[Tick],
    close_src: &[usize],
) -> Option<CaseSignals> {
    let a_idx = map_src_to_close_idx(close_src, seg_a.0, seg_a.1)?;
    let c_idx = map_src_to_close_idx(close_src, seg_c.0, seg_c.1)?;
    let dir = match side {
        Side::Long => Direction::Down,
        Side::Short => Direction::Up,
    };
    // B 域 = A 终到 C 起之间（close 下标开区间）。
    let b_pullback = if a_idx.1 < c_idx.0 && a_idx.1 + 1 <= c_idx.0 - 1 {
        let (b_lo, b_hi) = (a_idx.1 + 1, c_idx.0 - 1);
        match side {
            Side::Long => Some(
                macd.dif[b_lo..=b_hi]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max),
            ),
            Side::Short => Some(
                -macd.dif[b_lo..=b_hi]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min),
            ),
        }
    } else {
        None
    };
    Some(CaseSignals {
        area_abs_a: segment_macd_area(&macd.hist, a_idx.0, a_idx.1),
        area_abs_c: segment_macd_area(&macd.hist, c_idx.0, c_idx.1),
        area_same_a: same_color_area(&macd.hist, a_idx.0, a_idx.1, side),
        area_same_c: same_color_area(&macd.hist, c_idx.0, c_idx.1, side),
        dif_peak_a: segment_dif_peak(&macd.dif, a_idx.0, a_idx.1, dir),
        dif_peak_c: segment_dif_peak(&macd.dif, c_idx.0, c_idx.1, dir),
        b_pullback,
        hist_peak_a: same_dir_hist_peak(&macd.hist, a_idx.0, a_idx.1, side),
        hist_peak_c: same_dir_hist_peak(&macd.hist, c_idx.0, c_idx.1, side),
        amp_a: segment_price_amplitude(closes_tick, a_idx.0, a_idx.1),
        amp_c: segment_price_amplitude(closes_tick, c_idx.0, c_idx.1),
        speed_a: segment_price_speed(closes_tick, a_idx.0, a_idx.1),
        speed_c: segment_price_speed(closes_tick, c_idx.0, c_idx.1),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// §5. main
// ═══════════════════════════════════════════════════════════════════════════

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p113_pan_caseaudit <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P113_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    let bars = &loaded.bars[..max_bars];
    let as_of = bars.len() - 1;
    let layer = parse_layer(bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes_f64: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let closes_tick: Vec<Tick> = layer.merged_bars.iter().map(|b| b.close).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let macd = compute_macd(&closes_f64, &config.macd);
    println!(
        "P113_INPUT bars={} replay_bars={} as_of={} merged={} tower_levels={} classification_levels={}",
        loaded.bars.len(),
        max_bars,
        as_of,
        layer.merged_bars.len(),
        tower.len(),
        classification.levels.len()
    );

    // ── V3 生产口径分级测量（pairs + events + pan instrument）──
    let mut levels: Vec<LevelData> = Vec::new();
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let data = measure_level(level, &tower[level], &lower, as_of, &macd.hist, &macd.dif, &close_src)?;
        let cons_events = data
            .events
            .iter()
            .filter(|e| e.kind == NestDivergenceKind::Consolidation)
            .count();
        let cons_unconfirmed = data
            .events
            .iter()
            .filter(|e| e.kind == NestDivergenceKind::Consolidation && !e.divergence_confirmed)
            .count();
        println!(
            "P113_LEVEL L{} runs={} invalid={} moves={} completed={} strict_pairs={} events={} cons_events={} cons_unconfirmed={} pan_attempts={}",
            level, data.runs, data.invalid_windows, data.moves, data.completed,
            data.pairs.len(), data.events.len(), cons_events, cons_unconfirmed, data.pan_attempts.len()
        );
        levels.push(data);
    }

    // ── 硬锚 1：provider 事件总数复现 P109_ANCHOR_EVENTS ──
    let total_events: usize = levels.iter().map(|l| l.events.len()).sum();
    let total_confirmed: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed)
        .count();
    let trend_events: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.kind == NestDivergenceKind::Trend)
        .count();
    let trend_confirmed: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed && e.kind == NestDivergenceKind::Trend)
        .count();
    println!(
        "P113_ANCHOR_EVENTS candidates={} trend={} pan={} divergence_confirmed={} trend_confirmed={} pan_confirmed={} expected_candidates=3683 expected_confirmed=1235 expected_trend_confirmed=429",
        total_events,
        trend_events,
        total_events - trend_events,
        total_confirmed,
        trend_confirmed,
        total_confirmed - trend_confirmed
    );

    // ── 硬锚 2：instrument Success 集 == provider Cons 事件集（双向 diff=0）──
    let side_u8 = |s: Side| match s {
        Side::Long => 0u8,
        Side::Short => 1u8,
    };
    let provider_cons: BTreeSet<(u32, usize, (usize, usize), (usize, usize), u8)> = levels
        .iter()
        .flat_map(|l| {
            l.events
                .iter()
                .filter(|e| e.kind == NestDivergenceKind::Consolidation)
                .map(|e| (e.level, e.turn_source, e.seg_a, e.interval_b, side_u8(e.side)))
        })
        .collect();
    let instrument_success: BTreeSet<(u32, usize, (usize, usize), (usize, usize), u8)> = (1..tower
        .len())
        .zip(levels.iter())
        .flat_map(|(level, l)| {
            l.pan_attempts
                .iter()
                .filter(|a| a.outcome == AttemptOutcome::Success && a.structure.is_some())
                .map(move |a| {
                    let (seg_a, seg_c) = a.structure.expect("filtered is_some");
                    (
                        level as u32,
                        a.seg.1,
                        seg_a,
                        seg_c,
                        side_u8(match a.dir {
                            Direction::Down => Side::Long,
                            Direction::Up => Side::Short,
                        }),
                    )
                })
        })
        .collect();
    let prov_missing: Vec<_> = provider_cons.difference(&instrument_success).collect();
    let inst_extra: Vec<_> = instrument_success.difference(&provider_cons).collect();
    println!(
        "P113_ANCHOR_PROVISION instrument_success={} provider_cons={} missing_in_instrument={} extra_in_instrument={} expected_diff=0",
        instrument_success.len(),
        provider_cons.len(),
        prov_missing.len(),
        inst_extra.len()
    );
    for key in prov_missing.iter().take(5) {
        println!("P113_ANCHOR_PROVISION_MISSING {key:?}");
    }
    for key in inst_extra.iter().take(5) {
        println!("P113_ANCHOR_PROVISION_EXTRA {key:?}");
    }
    let mut atom_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for l in &levels {
        for a in &l.pan_attempts {
            let label = match a.outcome {
                AttemptOutcome::Success => "success",
                AttemptOutcome::ExtremeFail => "extreme_fail",
                AttemptOutcome::NoCoreBreak => "no_core_break",
                AttemptOutcome::NoReenter => "no_reenter",
                AttemptOutcome::NoAAnchor => "no_a_anchor",
                AttemptOutcome::NoReenterBetween => "no_reenter_between",
                AttemptOutcome::OtherFail => "other_fail",
            };
            *atom_counts.entry(label).or_default() += 1;
        }
    }
    for (label, count) in &atom_counts {
        println!("P113_ATTEMPT_ATOM atom={label} count={count}");
    }

    // ── 链枚举与逐门对账 ──
    let complete = chain_dp_count(&levels);
    let (chains, truncated) = enumerate_chains(&levels, 500_000);
    println!(
        "P113_ENUM chains_enumerated={} truncated={} dp_complete={}",
        chains.len(),
        truncated,
        complete
    );
    let match_maps: Vec<BTreeMap<(usize, usize), Vec<usize>>> = levels
        .iter()
        .map(|level| {
            let mut map: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
            for (index, event) in level.events.iter().enumerate() {
                map.entry(event.interval_a).or_default().push(index);
            }
            map
        })
        .collect();
    let outcomes: Vec<Outcome> = chains
        .iter()
        .enumerate()
        .map(|(id, chain)| reconcile(id, chain, &levels, &match_maps, &classification))
        .collect();
    let mut gate_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for outcome in &outcomes {
        *gate_counts.entry(outcome.gate.label()).or_default() += 1;
    }
    let identity_count = *gate_counts.get("identity").unwrap_or(&0);
    let divergence_count = *gate_counts.get("divergence").unwrap_or(&0);
    let cons_leave_chains = outcomes
        .iter()
        .filter(|o| o.gate == KillGate::Identity && o.pairs[o.kill_level - 1].leave_kind == MoveKind::Consolidation)
        .count();
    println!(
        "P113_ANCHOR_GATES identity={} divergence={} terminal={} direction={} inclusion={} alive={} cons_leave={} expected_identity=3790 expected_divergence=1020 expected_cons_leave=1238",
        identity_count,
        divergence_count,
        gate_counts.get("terminal").unwrap_or(&0),
        gate_counts.get("direction").unwrap_or(&0),
        gate_counts.get("inclusion").unwrap_or(&0),
        gate_counts.get("alive").unwrap_or(&0),
        cons_leave_chains
    );

    // ═══════════════════════════════════════════════════════════════════
    // Part A：pan MACD 面积失败逐案判（§6.1）
    // ═══════════════════════════════════════════════════════════════════
    // 母集 A1：全宇宙各级 Cons !confirmed 事件；母集 A2：kill=Divergence 链的 L1 基例并集。
    let mut universe: Vec<(usize, usize)> = Vec::new(); // (level_idx 0-based, event_idx)
    for (li, l) in levels.iter().enumerate() {
        for (ei, e) in l.events.iter().enumerate() {
            if e.kind == NestDivergenceKind::Consolidation && !e.divergence_confirmed {
                universe.push((li, ei));
            }
        }
    }
    let mut base_set: BTreeSet<usize> = BTreeSet::new(); // L1 = levels[0] 事件下标
    for o in &outcomes {
        if o.gate == KillGate::Divergence {
            for &ei in &o.base_events {
                base_set.insert(ei);
            }
        }
    }
    // 信号缓存：(li, ei) → Option<CaseSignals>（None = 坐标无法映射 close）。
    let mut sig_cache: BTreeMap<(usize, usize), Option<CaseSignals>> = BTreeMap::new();
    let mut sig_of = |li: usize, ei: usize| -> Option<CaseSignals> {
        if let Some(v) = sig_cache.get(&(li, ei)) {
            return *v;
        }
        let v = case_signals(&levels[li].events[ei], &macd, &closes_tick, &close_src);
        sig_cache.insert((li, ei), v);
        v
    };

    // 硬锚 4：!confirmed 事件混合面积 C<A 计数 == 0。
    let mut area_abs_viol = 0usize;
    let report_group = |name: &str,
                        group: &[(usize, usize)],
                        sig_of: &mut dyn FnMut(usize, usize) -> Option<CaseSignals>| {
        let mut n = 0usize;
        let mut unmappable = 0usize;
        let mut c_area_same = 0usize;
        let mut c_dif = 0usize;
        let mut c_dif_pb = 0usize;
        let mut c_b_nonempty = 0usize;
        let mut c_hist = 0usize;
        let mut c_amp = 0usize;
        let mut c_speed = 0usize;
        let mut c_deaf = 0usize;
        let mut c_doc = 0usize;
        let mut pb_values: Vec<f64> = Vec::new();
        for &(li, ei) in group {
            let Some(sig) = sig_of(li, ei) else {
                unmappable += 1;
                continue;
            };
            n += 1;
            c_area_same += usize::from(sig.s_area_same());
            c_dif += usize::from(sig.s_dif());
            c_dif_pb += usize::from(sig.s_dif_pb());
            c_b_nonempty += usize::from(sig.b_pullback.is_some());
            c_hist += usize::from(sig.s_hist());
            c_amp += usize::from(sig.s_amp());
            c_speed += usize::from(sig.s_speed());
            c_deaf += usize::from(sig.deaf_or());
            c_doc += usize::from(sig.doc_02732());
            if let Some(pb) = sig.b_pullback {
                pb_values.push(pb);
            }
        }
        println!(
            "P113A_SIGNAL group={} n={} unmappable={} area_same={} dif={} dif_pb02732={} b_domain_nonempty={} hist_peak={} price_amp={} price_speed={}",
            name, n, unmappable, c_area_same, c_dif, c_dif_pb, c_b_nonempty, c_hist, c_amp, c_speed
        );
        println!(
            "P113A_VERDICT group={} deaf_or={} market_fact={} doc02732={}",
            name,
            c_deaf,
            n - c_deaf,
            c_doc
        );
        if !pb_values.is_empty() {
            pb_values.sort_by(|x, y| x.partial_cmp(y).expect("finite"));
            let q = |p: f64| pb_values[((pb_values.len() - 1) as f64 * p) as usize];
            println!(
                "P113A_B_PULLBACK group={} n={} p50={:.4} p90={:.4} p99={:.4} max={:.4} （Long=max(dif[B])，Short=-min(dif[B])；>=0 为触及0轴）",
                name, pb_values.len(), q(0.5), q(0.9), q(0.99), *pb_values.last().expect("nonempty")
            );
        }
    };
    let base_group: Vec<(usize, usize)> = base_set.iter().map(|&ei| (0usize, ei)).collect();
    report_group("universe", &universe, &mut sig_of);
    report_group("chain_base", &base_group, &mut sig_of);
    // 链级判定：基例集任一事件 deaf_or → 该链教义下不该死于此门。
    let mut deaf_chains = 0usize;
    let mut fact_chains = 0usize;
    let mut unmappable_chains = 0usize;
    for o in &outcomes {
        if o.gate != KillGate::Divergence {
            continue;
        }
        let mut any = false;
        let mut any_mappable = false;
        for &ei in &o.base_events {
            match sig_of(0, ei) {
                Some(sig) => {
                    any_mappable = true;
                    if sig.deaf_or() {
                        any = true;
                    }
                }
                None => {}
            }
        }
        if any {
            deaf_chains += 1;
        } else if any_mappable {
            fact_chains += 1;
        } else {
            unmappable_chains += 1;
        }
    }
    println!(
        "P113A_CHAIN_VERDICT divergence_chains={} deaf_chains={} market_fact_chains={} unmappable_chains={}",
        divergence_count, deaf_chains, fact_chains, unmappable_chains
    );
    // 代表样本：chain_base 中 deaf 与 market_fact 各 5 个。
    for (label, want_deaf) in [("DEAF", true), ("FACT", false)] {
        let mut shown = 0usize;
        for &(li, ei) in &base_group {
            let Some(sig) = sig_of(li, ei) else { continue };
            if sig.deaf_or() != want_deaf {
                continue;
            }
            let e = &levels[li].events[ei];
            println!(
                "P113A_SAMPLE kind={} level={} side={:?} turn={} seg_a={:?} seg_c={:?} area_abs(a={:.2},c={:.2}) area_same(a={:.2},c={:.2}) dif(a={:.4},c={:.4}) b_pullback={} hist_peak(a={:.4},c={:.4}) amp(a={},c={}) speed(a={:.2},c={:.2})",
                label, e.level, e.side, e.turn_source, e.seg_a, e.interval_b,
                sig.area_abs_a, sig.area_abs_c, sig.area_same_a, sig.area_same_c,
                sig.dif_peak_a, sig.dif_peak_c,
                sig.b_pullback.map(|v| format!("{v:.4}")).unwrap_or_else(|| "NA".to_string()),
                sig.hist_peak_a, sig.hist_peak_c, sig.amp_a, sig.amp_c, sig.speed_a, sig.speed_c
            );
            shown += 1;
            if shown >= 5 {
                break;
            }
        }
    }
    // 面积对账锚：chain_base 内混合面积 C<A 应 0。
    for &ei in &base_set {
        if let Some(sig) = sig_of(0, ei) {
            if sig.area_abs_c < sig.area_abs_a {
                area_abs_viol += 1;
            }
        }
    }
    println!(
        "P113A_ANCHOR_AREA chain_base_abs_c_lt_a={} expected=0（与 divergence_confirmed=false 逐位一致）",
        area_abs_viol
    );

    // ═══════════════════════════════════════════════════════════════════
    // Part B：Cons-leave 1,238 几何审计（§6.2）
    // ═══════════════════════════════════════════════════════════════════
    // 收集 kill=Identity && leave==Consolidation 的链 → (kill_level, leave pair)。
    struct LeaveCase {
        chain_id: usize,
        level: usize,
        pair: PairRec,
    }
    let mut cases: Vec<LeaveCase> = Vec::new();
    for o in &outcomes {
        if o.gate != KillGate::Identity {
            continue;
        }
        let p = o.pairs[o.kill_level - 1];
        if p.leave_kind != MoveKind::Consolidation {
            continue;
        }
        cases.push(LeaveCase {
            chain_id: o.id,
            level: o.kill_level,
            pair: p,
        });
    }
    // 按 (level, leave_span) 聚合块（同块可属多 pair/链）。
    let mut groups: BTreeMap<(usize, (usize, usize)), Vec<usize>> = BTreeMap::new();
    let mut pair_set: BTreeSet<(usize, (usize, usize))> = BTreeSet::new();
    for (idx, case) in cases.iter().enumerate() {
        groups
            .entry((case.level, case.pair.leave_span))
            .or_default()
            .push(idx);
        pair_set.insert((case.level, (case.pair.start, case.pair.end)));
    }
    println!(
        "P113B_UNIVERSE chains={} unique_pairs={} unique_blocks={} expected_chains=1238",
        cases.len(),
        pair_set.len(),
        groups.len()
    );
    // 块级分类。
    #[derive(Default)]
    struct BlockTally {
        chains: usize,
        attempts: usize,
        success: usize,
        extreme_fail: usize,
        no_core_break: usize,
        no_reenter: usize,
        no_a_anchor: usize,
        no_reenter_between: usize,
        other_fail: usize,
        wide_rescued: usize,      // 扩 A 后 extreme 过（结构层聋度候选）
        wide_rescued_force: usize, // 扩 A 救回且 §6.1 力度信号成立（结构+力度双救回）
        wide_fail_extreme: usize, // 扩 A 定位成但 extreme 负
        wide_no_prime: usize,     // 中枢前无同向段（扩 A 无候选）
    }
    let mut cat_counts: BTreeMap<&'static str, usize> = BTreeMap::new(); // 块级
    let mut cat_chains: BTreeMap<&'static str, usize> = BTreeMap::new(); // 链级（按 leave 块归类）
    let mut cat_level_chains: BTreeMap<(&'static str, usize), usize> = BTreeMap::new(); // (类别, 级别) 链数
    let mut samples: BTreeMap<(&'static str, usize), Vec<String>> = BTreeMap::new(); // (类别, 级别) → 样本
    let mut tally_all = BlockTally::default();
    // 扩 A 救回的唯一结构事件集（(level, turn, a_prime, seg_c, side_u8)）。
    let mut rescued_events: BTreeSet<(usize, usize, (usize, usize), (usize, usize), u8)> =
        BTreeSet::new();
    let mut rescued_force_events = 0usize;
    for ((level, leave_span), case_idxs) in &groups {
        let li = level - 1;
        let attempts: Vec<&PanAttempt> = levels[li]
            .pan_attempts
            .iter()
            .filter(|a| a.block_span == *leave_span)
            .collect();
        let mut t = BlockTally {
            chains: case_idxs.len(),
            attempts: attempts.len(),
            ..BlockTally::default()
        };
        for a in &attempts {
            match a.outcome {
                AttemptOutcome::Success => t.success += 1,
                AttemptOutcome::ExtremeFail => t.extreme_fail += 1,
                AttemptOutcome::NoCoreBreak => t.no_core_break += 1,
                AttemptOutcome::NoReenter => t.no_reenter += 1,
                AttemptOutcome::NoAAnchor => t.no_a_anchor += 1,
                AttemptOutcome::NoReenterBetween => t.no_reenter_between += 1,
                AttemptOutcome::OtherFail => t.other_fail += 1,
            }
            if let Some(w) = &a.wide_a {
                if w.extreme_ok {
                    t.wide_rescued += 1;
                    let side = match a.dir {
                        Direction::Down => Side::Long,
                        Direction::Up => Side::Short,
                    };
                    let su8 = match side {
                        Side::Long => 0u8,
                        Side::Short => 1u8,
                    };
                    let is_new = rescued_events.insert((*level, a.seg.1, w.a_prime, w.seg_c, su8));
                    let force_ok = signals_for(
                        side,
                        w.a_prime,
                        w.seg_c,
                        &macd,
                        &closes_tick,
                        &close_src,
                    )
                    .map(|s| s.deaf_or())
                    .unwrap_or(false);
                    if force_ok {
                        t.wide_rescued_force += 1;
                        if is_new {
                            rescued_force_events += 1;
                        }
                    }
                } else {
                    t.wide_fail_extreme += 1;
                }
            } else if matches!(
                a.outcome,
                AttemptOutcome::NoReenter | AttemptOutcome::NoAAnchor | AttemptOutcome::NoReenterBetween | AttemptOutcome::ExtremeFail
            ) {
                t.wide_no_prime += 1;
            }
        }
        tally_all.chains += t.chains;
        tally_all.attempts += t.attempts;
        tally_all.success += t.success;
        tally_all.extreme_fail += t.extreme_fail;
        tally_all.no_core_break += t.no_core_break;
        tally_all.no_reenter += t.no_reenter;
        tally_all.no_a_anchor += t.no_a_anchor;
        tally_all.no_reenter_between += t.no_reenter_between;
        tally_all.other_fail += t.other_fail;
        tally_all.wide_rescued += t.wide_rescued;
        tally_all.wide_rescued_force += t.wide_rescued_force;
        tally_all.wide_fail_extreme += t.wide_fail_extreme;
        tally_all.wide_no_prime += t.wide_no_prime;
        // 分类优先级：锚违例 > A 锚窄化双救回 > A 锚窄化仅结构 > Extreme 判负域 > 中枢震荡域 > 无对照 > 混合。
        let cat: &'static str = if t.success > 0 {
            "anchor_violation"
        } else if t.wide_rescued_force > 0 {
            "rescued_with_force"
        } else if t.wide_rescued > 0 {
            "rescued_structure_only"
        } else if t.extreme_fail > 0 {
            "extreme_fail_domain"
        } else if t.attempts > 0 && t.no_core_break == t.attempts {
            "no_core_break_only"
        } else if t.wide_no_prime > 0 && t.wide_fail_extreme == 0 {
            "no_comparable_segment"
        } else if t.wide_fail_extreme > 0 {
            "wide_extreme_fail"
        } else {
            "mixed_other"
        };
        *cat_counts.entry(cat).or_default() += 1;
        *cat_chains.entry(cat).or_default() += t.chains;
        *cat_level_chains.entry((cat, *level)).or_default() += t.chains;
        let bucket = samples.entry((cat, *level)).or_default();
        if bucket.len() < 2 {
            let first = &attempts.first();
            bucket.push(format!(
                "L{} block=({}, {}) chains={} attempts={} atoms[succ={} xfail={} nocore={} noreenter={} noanchor={} nobetween={} other={}] wide[rescued={} rescued_force={} xfail={} noprime={}] first_attempt={:?}",
                level, leave_span.0, leave_span.1, t.chains, t.attempts,
                t.success, t.extreme_fail, t.no_core_break, t.no_reenter, t.no_a_anchor,
                t.no_reenter_between, t.other_fail, t.wide_rescued, t.wide_rescued_force,
                t.wide_fail_extreme, t.wide_no_prime, first.map(|a| (a.seg, a.outcome))
            ));
        }
    }
    println!(
        "P113B_ATTEMPTS total_blocks={} total_chains={} attempts={} success={} extreme_fail={} no_core_break={} no_reenter={} no_a_anchor={} no_reenter_between={} other_fail={} wide_rescued={} wide_rescued_force={} wide_fail_extreme={} wide_no_prime={}",
        groups.len(), tally_all.chains, tally_all.attempts, tally_all.success, tally_all.extreme_fail,
        tally_all.no_core_break, tally_all.no_reenter, tally_all.no_a_anchor,
        tally_all.no_reenter_between, tally_all.other_fail, tally_all.wide_rescued,
        tally_all.wide_rescued_force, tally_all.wide_fail_extreme, tally_all.wide_no_prime
    );
    println!(
        "P113B_RESCUE unique_rescued_events={} rescued_with_force_events={} rescued_structure_only_events={} （扩A救回结构去重事件；力度=§6.1同色面积∨黄白线∨柱高∨幅度）",
        rescued_events.len(),
        rescued_force_events,
        rescued_events.len() - rescued_force_events
    );
    for (cat, count) in &cat_counts {
        println!(
            "P113B_CATEGORY cat={} blocks={} chains={}",
            cat,
            count,
            cat_chains.get(cat).unwrap_or(&0)
        );
    }
    for ((cat, level), chains) in &cat_level_chains {
        println!("P113B_CAT_LEVEL cat={} level=L{} chains={}", cat, level, chains);
    }
    for ((cat, _level), bucket) in &samples {
        for line in bucket {
            println!("P113B_SAMPLE cat={} {}", cat, line);
        }
    }
    println!("P113_DONE");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// §6. 数据加载（复刻 p109，只读）
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
}

fn load_bars(path: &Path, tick_size: f64) -> Result<LoadedBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    let bars = (0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect();
    Ok(LoadedBars { bars })
}

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let mut digits = String::with_capacity(14);
    for ch in date.chars().take_while(|value| *value != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse().unwrap_or(0)
}
