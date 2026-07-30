//! task #112：754 对趋势基例谓词分歧逐案机械对审探针（只读，不改任何生产源码）。
//!
//! 依据 `chanlun/review-results/doc-trend-divergence-predicate-20260717.md` 的全合取谓词
//! （比较对象唯一=c vs b；完整 MACD 口径=回拉0轴 ∧（面积∨黄白线∨高度）；BSP 谓词缺
//! 037:20 破极值项+非教义 anchor 门；D2 谓词缺 037:18 三买项 + 025:761 回拉0轴项），
//! 对 p109 报告的 754 条「趋势基例 D2 背驰确认 ∧ 同级同 source 无 confirm_side BSP」链
//! 逐案机械判定归属：
//!   (i)  BSP 谓词欠配错杀 —— 全合取成立而 BSP 无点（anchor 门 / 取段差异 / τ 门差异）；
//!   (ii) D2 谓词欠配错杀 —— 全合取不成立（缺三买或缺回拉0轴）而 D2 确认（D2 宽）；
//!   (iii) 真不背驰 —— D2 自己检查的项目（趋势块/破极值/面积）机械复算不成立
//!         （两谓词正确施加都判负；若计数 >0 则是 provider 装配工件证据）。
//!
//! 管线复刻 p109（V3 CarriedOnly + #105 修复，终态快照，batch 单遍）：
//!   严格 C2 pair → 全深度链枚举 → 身份门 → 背驰确认门 → 终端门（首个空候选门=击杀门）。
//! 硬锚（与 p109 逐字一致才可信）：candidates=3683 divergence_confirmed=1235
//!   trend_confirmed=429；身份门击杀 3790、背驰确认门 1020、终端门 1631、
//!   终端门趋势基例 754、过终端门 50。
//!
//! 机械判定五项合取（每案）：
//!   T1 趋势块 ≥2 中枢（027:14/037:16，构造性成立，复算验证）；
//!   T2 c 包络破 b 包络极值（037:20，同 D2 Extreme 门原语 range_envelope）；
//!   T3 c 内含对最后中枢 B 的三买（037:18，judge_third 几何判据：同向离开破核心 +
//!      紧邻反向回试端点不重回 [ZD,ZG]，无 anchor 门——教义谓词无该工程门）；
//!   T4 黄白线回拉 0 轴（025:761/024:24，B 中枢 span 内 DIF 穿越/触及 0 轴为主口径，
//!      另报 DIF∧DEA 双穿、min max(|dif|,|dea|) ≤10%/25% 全段极值 三个敏感性口径）；
//!   T5 MACD 面积 c<b（§3 合法 proxy，segments_diverge 原语复算）。
//! (i) 类子诊断复刻 BSP 门链（judge_segment 第一类路径，输入 = 生产同源 units/anchors/
//! centers）：S0 无 unit 端点落在 turn（取段边界差异）→ S1 τ 门闭 → S2 anchor 资格门 →
//! S3 未破 ZG/ZD → S4 A 段不可配 → S5 BSP 取段面积 C≥A → S6 全通（复现缝隙，须为 0）。
//!
//! 用法：
//! `cargo run --release --features backtest_bin --bin p112_trend_predicate_caseaudit -- \
//!    <btc_1m_full.json>`
//! 冒烟：`P112_MAX_BARS=250000 ...`（硬锚仅全量有效）。

use newchan_rust::theta_v0::classifier::bsp::BspPoint;
use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose::{center_own_dir_at, center_trend_gate};
use newchan_rust::theta_v0::classifier::divergence::{
    compute_macd, departure_move_c_start, locate_departure_move_a, segment_macd_area,
    segments_diverge,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CompletionEvidence,
    CompletionStatus, CoordinateWindow, LevelViewMaterial, LevelViewQuery,
    NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{is_sub, NestInterval};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    map_src_to_close_idx, project_to_units, LeveledMove,
};
use newchan_rust::theta_v0::classifier::Classification;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{
    quantize, Bar, BspBits, Center, Direction, MoveKind, Segment, Side, Tick, Timestamp,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// 严格 C2 pair（同 p109：同 run 内两个不同、正向相邻、均 Completed 的 Move）。
#[derive(Debug, Clone, Copy)]
struct PairRec {
    start: usize,
    end: usize,
    leave_kind: MoveKind,
    leave_via_divergence: bool,
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
    moves: usize,
    completed: usize,
    pairs: Vec<PairRec>,
    events: Vec<NestCandidateEvent>,
}

/// D2 pair 的教义合取审计上下文（per L1 run 收集）。
#[derive(Debug, Clone, Copy)]
struct PairAuditCtx {
    direction: Direction,
    move_start: usize,
    n_block_centers: usize,
    prev: Center,
    last: Center,
}

/// 逐案审计输出。
#[derive(Debug)]
struct CaseAudit {
    turn: usize,
    side: Side,
    direction: Direction,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
    chains: usize,
    t1: bool,
    t2: bool,
    t3: bool,
    t3_hit: Option<(usize, usize)>,
    t3_ext: bool,
    t3_ext_hit: Option<(usize, usize)>,
    win_legs: usize,
    t4_cross_dif: bool,
    t4_cross_both: bool,
    t4_r10: bool,
    t4_r25: bool,
    dif_min: f64,
    dea_min: f64,
    band_max: f64,
    t5: bool,
    area_a: f64,
    area_c: f64,
    bsp_gate: &'static str,
    bsp_book: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    BspFalseKill, // (i) BSP 谓词欠配错杀
    D2Overwide,   // (ii) D2 谓词欠配错杀
    TrueNegative, // (iii) 真不背驰（D2 自检项复算不成立）
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Verdict::BspFalseKill => "i_bsp_false_kill",
            Verdict::D2Overwide => "ii_d2_overwide",
            Verdict::TrueNegative => "iii_true_negative",
        }
    }
}

fn verdict_of(a: &CaseAudit, t4: bool) -> Verdict {
    if !(a.t1 && a.t2 && a.t5) {
        Verdict::TrueNegative
    } else if a.t3 && t4 {
        Verdict::BspFalseKill
    } else {
        Verdict::D2Overwide
    }
}


/// level_view.rs:492 `range_envelope` 同口径复刻（段须整支落入 span）。
fn range_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    segments
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc, segment| {
            let lo = segment.start_price.min(segment.end_price);
            let hi = segment.start_price.max(segment.end_price);
            Some(match acc {
                None => (lo, hi),
                Some((old_lo, old_hi)) => (old_lo.min(lo), old_hi.max(hi)),
            })
        })
}

/// signal.rs:208 `nearest_confirmed_center_idx` 同口径复刻（前缀末下标）。
fn nearest_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(hi - 1)
    }
}

/// mod.rs:223 `unit_to_segment` 同口径复刻（向上单元 start=lo/end=hi，向下镜像）。
fn unit_to_segment_pub(u: &newchan_rust::theta_v0::classifier::center::UnitRange) -> Segment {
    let (start_price, end_price) = match u.direction {
        Direction::Up => (u.lo, u.hi),
        Direction::Down => (u.hi, u.lo),
    };
    Segment {
        direction: u.direction,
        start_index: u.start_index,
        end_index: u.end_index,
        start_price,
        end_price,
    }
}

/// level_view.rs:431 `leg_as_segment` 同口径复刻（LowerLeg → Segment）。
fn leg_as_segment_pub(value: &newchan_rust::theta_v0::classifier::level_view::LowerLeg) -> Segment {
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

/// T3（037:18）：c 内含对最后中枢 B 的第三类买卖点。
/// 判据 = judge_third 几何（signal.rs:399-447）去 anchor 门：c 窗口
/// [max(c_start,last.end_index), seg_c.1] 内存在紧邻段对 (leave, retest)，leave 同趋势
/// 方向且端点破核心（Up: >zg / Down: <zd），retest 反向且端点不重回核心（严格不等式）。
/// 返回首个命中 (leave.end_index, retest.end_index)。
/// 构造事实：D2 seg_c = [episode 首段起点, 首个同向段终点]（#105 正向 find）⟹ 窗口恒为
/// 单条 L0 线段 ⟹ 本严格口径构造性恒假（这正是「D2 缺三买项」的机械显形）。
fn third_class_in_c(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    c_start: usize,
    c_end: usize,
) -> Option<(usize, usize)> {
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let hi = segments.partition_point(|s| s.end_index <= c_end);
    if hi <= lo + 1 {
        return None;
    }
    let win = &segments[lo..hi];
    for pair in win.windows(2) {
        let (leave, retest) = (&pair[0], &pair[1]);
        if leave.direction != direction || retest.direction == direction {
            continue;
        }
        let hit = match direction {
            Direction::Up => leave.end_price > last.zg && retest.end_price > last.zg,
            Direction::Down => leave.end_price < last.zd && retest.end_price < last.zd,
        };
        if hit {
            return Some((leave.end_index, retest.end_index));
        }
    }
    None
}

/// T3_ext（037:18 的全离开段口径，第二诊断轴）：从 c_start 起向未来扫全部后续段，
/// 找首个「同向离开破核心 + 紧邻反向回试不重回核心」紧邻段对——回试段允许落在
/// turn（seg_c.1）之后（即测「若 c 取整个离开走势而非首个离开腿，三买结构是否存在」）。
/// 用于把 (ii) 类分解为：(ii-a) 回拉重回 B=c 根本未确立（中枢震荡，应归盘整背驰对象）；
/// (ii-b) 三买存在于全离开段=失败仅因 #105 c 取段截断（修法后可重审 (i)/(iii)）。
fn third_class_extended(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    c_start: usize,
) -> Option<(usize, usize)> {
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let win = &segments[lo..];
    for pair in win.windows(2) {
        let (leave, retest) = (&pair[0], &pair[1]);
        if leave.direction != direction || retest.direction == direction {
            continue;
        }
        let hit = match direction {
            Direction::Up => leave.end_price > last.zg && retest.end_price > last.zg,
            Direction::Down => leave.end_price < last.zd && retest.end_price < last.zd,
        };
        if hit {
            return Some((leave.end_index, retest.end_index));
        }
    }
    None
}

/// T4（025:761/024:24）黄白线回拉 0 轴：B 中枢 span（close 下标）内
/// cross_dif = DIF 变号或触 0；cross_both = DIF∧DEA 均变号或触 0；
/// ratio10/25 = B span 内 min max(|dif|,|dea|) ≤ 10%/25% × 全段 [seg_a.0, seg_c.1] 极值。
/// 返回 (cross_dif, cross_both, r10, r25, dif_min, dea_min, band_max)；span 不可映射全 false。
#[allow(clippy::too_many_arguments)]
fn pullback_to_zero(
    dif: &[f64],
    dea: &[f64],
    close_src: &[usize],
    b_span: (usize, usize),
    outer_span: (usize, usize),
) -> (bool, bool, bool, bool, f64, f64, f64) {
    let Some((b_lo, b_hi)) = map_src_to_close_idx(close_src, b_span.0, b_span.1) else {
        return (false, false, false, false, f64::NAN, f64::NAN, f64::NAN);
    };
    let Some((o_lo, o_hi)) = map_src_to_close_idx(close_src, outer_span.0, outer_span.1) else {
        return (false, false, false, false, f64::NAN, f64::NAN, f64::NAN);
    };
    let band_max = (o_lo..=o_hi)
        .map(|t| dif[t].abs().max(dea[t].abs()))
        .fold(0.0f64, f64::max);
    let mut dif_min = f64::INFINITY;
    let mut dea_min = f64::INFINITY;
    let mut band_min = f64::INFINITY;
    let mut cross_dif = false;
    let mut cross_dea = false;
    for t in b_lo..=b_hi {
        dif_min = dif_min.min(dif[t].abs());
        dea_min = dea_min.min(dea[t].abs());
        band_min = band_min.min(dif[t].abs().max(dea[t].abs()));
        if dif[t] == 0.0 || (t > b_lo && dif[t] * dif[t - 1] < 0.0) {
            cross_dif = true;
        }
        if dea[t] == 0.0 || (t > b_lo && dea[t] * dea[t - 1] < 0.0) {
            cross_dea = true;
        }
    }
    (
        cross_dif,
        cross_dif && cross_dea,
        band_min <= 0.10 * band_max,
        band_min <= 0.25 * band_max,
        dif_min,
        dea_min,
        band_max,
    )
}

/// BSP 门链子诊断（复刻 judge_segment 第一类路径，signal.rs:1150-1168 +
/// judge_first_cached:276-368；输入 = 生产同源级别-1 units/anchors/centers）。
/// 返回门链首杀位置标签；S6 = 门链全通（复现缝隙，须为 0）。
#[allow(clippy::too_many_arguments)]
fn bsp_gate_diag(
    segs1: &[Segment],
    anchors1: &[Option<Direction>],
    centers1: &[Center],
    gate1: &[Option<Direction>],
    hist: &[f64],
    close_src: &[usize],
    trend_dir: Direction,
    turn: usize,
) -> &'static str {
    let Some(u) = segs1.iter().position(|s| s.end_index == turn) else {
        return "S0_no_unit_at_turn";
    };
    let seg = &segs1[u];
    let Some(c_idx) = nearest_center_idx(centers1, seg.start_index) else {
        return "S0b_no_center";
    };
    // 生产 first_match_idx：中枢 end_index 严格递增 ⟹ pos==c_idx（resume 路径同此点等价）。
    let pos = c_idx;
    match gate1.get(pos).copied().flatten() {
        None => return "S1a_tau_none",
        Some(d) if d != trend_dir => return "S1b_tau_opposite",
        _ => {}
    }
    if anchors1.get(u).copied().flatten() != Some(trend_dir) {
        return "S2_anchor_gate";
    }
    let c = &centers1[pos];
    let broke = match trend_dir {
        Direction::Down => seg.end_price < c.zd,
        Direction::Up => seg.end_price > c.zg,
    };
    if !broke {
        return "S3_no_break_zgzd";
    }
    let Some(prev) = pos.checked_sub(1).map(|p| &centers1[p]) else {
        return "S1a_tau_none";
    };
    if locate_departure_move_a(segs1, anchors1, prev, c, trend_dir).is_none() {
        return "S4_a_unpairable";
    }
    let Some(lambda_c) = departure_move_c_start(segs1, anchors1, c, trend_dir, seg.start_index)
    else {
        return "S4b_no_lambda_c";
    };
    let a_seg = locate_departure_move_a(segs1, anchors1, prev, c, trend_dir);
    let (Some((a0, a1)), Some((c0, c1))) = (
        a_seg.and_then(|(s, e)| map_src_to_close_idx(close_src, s, e)),
        map_src_to_close_idx(close_src, lambda_c, seg.end_index),
    ) else {
        return "S4c_unmap";
    };
    if !segments_diverge(hist, (a0, a1), (c0, c1)) {
        return "S5_area_flip";
    }
    "S6_bsp_should_fire"
}

/// 单级测量（V3 CarriedOnly，同 p109 measure_level；L1 附带收集 D2 pair 审计上下文）。
fn measure_level(
    level: usize,
    windows: &[LeveledMove],
    lower_legs: &[newchan_rust::theta_v0::classifier::level_view::LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    pair_audit: &mut BTreeMap<((usize, usize), (usize, usize)), PairAuditCtx>,
    pair_audit_collisions: &mut usize,
) -> Result<LevelData, String> {
    let mut data = LevelData::default();
    let version = C2VersionTuple::auto_pairing();
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).is_ok();
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L{level} run 投影失败: {error:?}"))?;
                let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = newchan_rust::theta_v0::classifier::decompose::decompose(&centers);
                let query = LevelViewQuery {
                    level: level as u32,
                    coordinate_window: CoordinateWindow {
                        start: projection.seeds.first().expect("nonempty run").start_index,
                        end: projection.seeds.last().expect("nonempty run").end_index,
                    },
                    as_of,
                    version,
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
                data.moves += view.moves.len();
                data.completed += view
                    .moves
                    .iter()
                    .filter(|m| matches!(m.completion, CompletionStatus::Completed { .. }))
                    .count();
                for pair in view.moves.windows(2) {
                    let (CompletionStatus::Completed { evidence: ev0, .. }, CompletionStatus::Completed { .. }) =
                        (&pair[0].completion, &pair[1].completion)
                    else {
                        continue;
                    };
                    data.pairs.push(PairRec {
                        start: pair[0].start_index,
                        end: pair[1].end_index,
                        leave_kind: pair[0].kind,
                        leave_via_divergence: matches!(ev0, CompletionEvidence::TerminalDivergence { .. }),
                    });
                }
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
                if level == 1 {
                    collect_pair_audit(&projection, &view, pair_audit, pair_audit_collisions);
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

/// L1：把每个 D2 pair 的教义合取审计上下文（prev/last 中枢、块中枢数）按
/// (seg_a, seg_c) 键收集（事件经 seg_a/interval_b 与之同源连接）。
fn collect_pair_audit(
    projection: &newchan_rust::theta_v0::classifier::level_view::ExactThreeProjection,
    view: &newchan_rust::theta_v0::classifier::level_view::LevelAsOfView,
    out: &mut BTreeMap<((usize, usize), (usize, usize)), PairAuditCtx>,
    collisions: &mut usize,
) {
    for pair in &view.pairs {
        let (Some(last), Some(prev)) = (
            projection.seeds.get(pair.id.block_end_center),
            pair.id
                .block_end_center
                .checked_sub(1)
                .and_then(|p| projection.seeds.get(p)),
        ) else {
            continue;
        };
        let ctx = PairAuditCtx {
            direction: pair.id.direction,
            move_start: pair.move_start,
            n_block_centers: pair.id.block_end_center - pair.id.block_start_center + 1,
            prev: prev.center,
            last: last.center,
        };
        if out.insert((pair.seg_a, pair.seg_c), ctx).is_some() {
            *collisions += 1;
        }
    }
}

fn terminal_bits_new(classification: &Classification, event: &NestCandidateEvent) -> Option<BspBits> {
    // issue #747 C1 单源：改调 classifier::bsp::bind_turn（生产绑定规则原型 nest.rs:681 语义，
    // bin 侧诊断复制点收敛，见 issue747-impl 报告）。
    newchan_rust::theta_v0::classifier::bsp::bind_turn(
        &classification.levels.get(event.level as usize)?.bsp,
        event.turn_source,
        event.side,
    )
        .map(|point| point.bits)
}


/// 枚举全部全深度链（同 p109 enumerate_chains：L1 起逐级向上，闭包含边）。
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

#[allow(clippy::too_many_arguments)]
fn audit_case(
    key: ((usize, usize), (usize, usize)),
    ctx: &PairAuditCtx,
    side: Side,
    chains: usize,
    segs_l0: &[Segment],
    dif: &[f64],
    dea: &[f64],
    hist: &[f64],
    close_src: &[usize],
    segs1: &[Segment],
    anchors1: &[Option<Direction>],
    centers1: &[Center],
    gate1: &[Option<Direction>],
    bsp_book: &[BspPoint],
) -> CaseAudit {
    let (seg_a, seg_c) = key;
    let direction = ctx.direction;
    let t1 = ctx.n_block_centers >= 2;
    // T2（037:20）：c 包络破 b 包络极值（同 D2 Extreme 门原语）。
    let t2 = match (range_envelope(segs_l0, seg_a), range_envelope(segs_l0, seg_c)) {
        (Some(a), Some(c)) => match direction {
            Direction::Down => c.0 < a.0,
            Direction::Up => c.1 > a.1,
        },
        _ => false,
    };
    // T3（037:18）：c 内含对 B 的三买（严格口径 = D2 seg_c 窗口内，构造性恒假）。
    let t3_hit = third_class_in_c(segs_l0, &ctx.last, direction, seg_c.0, seg_c.1);
    let t3 = t3_hit.is_some();
    // T3_ext（037:18 全离开段口径，第二诊断轴）：回试允许落在 turn 之后。
    let t3_ext_hit = third_class_extended(segs_l0, &ctx.last, direction, seg_c.0);
    let t3_ext = t3_ext_hit.is_some();
    // c 窗口段数（构造事实显形：D2 seg_c 恒为首个离开腿 ⟹ 恒 1）。
    let win_legs = {
        let lo = segs_l0.partition_point(|s| s.start_index < seg_c.0.max(ctx.last.end_index));
        let hi = segs_l0.partition_point(|s| s.end_index <= seg_c.1);
        hi.saturating_sub(lo)
    };
    // T4（025:761）：黄白线回拉 0 轴（B 中枢 span）。
    let (t4_cross_dif, t4_cross_both, t4_r10, t4_r25, dif_min, dea_min, band_max) =
        pullback_to_zero(dif, dea, close_src, (ctx.last.start_index, ctx.last.end_index), (seg_a.0, seg_c.1));
    // T5（§3 MACD 面积 c<b，segments_diverge 原语复算）。
    let (area_a, area_c, t5) = match (
        map_src_to_close_idx(close_src, seg_a.0, seg_a.1),
        map_src_to_close_idx(close_src, seg_c.0, seg_c.1),
    ) {
        (Some(a), Some(c)) => (
            segment_macd_area(hist, a.0, a.1),
            segment_macd_area(hist, c.0, c.1),
            segments_diverge(hist, a, c),
        ),
        _ => (f64::NAN, f64::NAN, false),
    };
    // BSP 门链子诊断。
    let bsp_gate = bsp_gate_diag(
        segs1, anchors1, centers1, gate1, hist, close_src, direction, seg_c.1,
    );
    // BSP 账本实况（turn 上的点与 bit）。
    // issue #747 C1 单源：改调 classifier::bsp::bsp_all_at（诊断枚举族内独立成员——
    // 见 issue747-impl 报告）。
    let at_turn: Vec<&BspPoint> =
        newchan_rust::theta_v0::classifier::bsp::bsp_all_at(&bsp_book, seg_c.1).collect();
    let bsp_book = if at_turn.is_empty() {
        "none".to_string()
    } else {
        let confirm = at_turn.iter().any(|p| p.bits.confirm_side(side));
        let any_bits = at_turn.iter().any(|p| {
            p.bits.buy1 || p.bits.buy2 || p.bits.buy3 || p.bits.sell1 || p.bits.sell2 || p.bits.sell3
        });
        let third_same_side = at_turn.iter().any(|p| match side {
            Side::Long => p.bits.buy3,
            Side::Short => p.bits.sell3,
        });
        format!(
            "points={} confirm={} any_bits={} third_same_side={}",
            at_turn.len(), confirm, any_bits, third_same_side
        )
    };
    CaseAudit {
        turn: seg_c.1,
        side,
        direction,
        seg_a,
        seg_c,
        chains,
        t1,
        t2,
        t3,
        t3_hit,
        t3_ext,
        t3_ext_hit,
        win_legs,
        t4_cross_dif,
        t4_cross_both,
        t4_r10,
        t4_r25,
        dif_min,
        dea_min,
        band_max,
        t5,
        area_a,
        area_c,
        bsp_gate,
        bsp_book,
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p112_trend_predicate_caseaudit <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P112_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    let bars = &loaded.bars[..max_bars];
    let as_of = bars.len() - 1;
    println!(
        "P112_INPUT bars={} replay_bars={} as_of={} first_date={} last_date={}",
        loaded.bars.len(), max_bars, as_of, loaded.first_date, loaded.last_date
    );
    let layer = parse_layer(bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let series = compute_macd(&closes, &config.macd);
    let (dif, dea, hist) = (&series.dif, &series.dea, &series.hist);
    println!(
        "P112_INPUT merged={} tower_levels={} classification_levels={}",
        layer.merged_bars.len(),
        tower.len(),
        classification.levels.len()
    );

    // ── V3 分级测量（同 p109）＋ L1 D2 pair 审计上下文 ──
    let mut levels: Vec<LevelData> = Vec::new();
    let mut pair_audit: BTreeMap<((usize, usize), (usize, usize)), PairAuditCtx> = BTreeMap::new();
    let mut pair_audit_collisions = 0usize;
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let data = measure_level(
            level,
            &tower[level],
            &lower,
            as_of,
            hist,
            dif,
            &close_src,
            &mut pair_audit,
            &mut pair_audit_collisions,
        )?;
        println!(
            "P112_LEVEL L{level} moves={} completed={} strict_pairs={} events={}",
            data.moves,
            data.completed,
            data.pairs.len(),
            data.events.len()
        );
        levels.push(data);
    }
    println!(
        "P112_PAIR_AUDIT pairs={} collisions={}（collisions 须为 0：(seg_a,seg_c) 键唯一）",
        pair_audit.len(),
        pair_audit_collisions
    );

    // ── 硬锚 ①：provider 事件总数（复现 P109_ANCHOR_EVENTS）──
    let total_events: usize = levels.iter().map(|l| l.events.len()).sum();
    let total_confirmed: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed)
        .count();
    let trend_confirmed: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed && e.kind == NestDivergenceKind::Trend)
        .count();
    println!(
        "P112_ANCHOR_EVENTS candidates={} divergence_confirmed={} trend_confirmed={} expected=3683/1235/429",
        total_events, total_confirmed, trend_confirmed
    );

    // ── 每级 interval_a → 事件下标匹配表（同 p109）──
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

    // ── 链枚举 ──
    let (chains, truncated) = enumerate_chains(&levels, 500_000);
    println!(
        "P112_ENUM chains_enumerated={} truncated={} expected=6482",
        chains.len(),
        truncated
    );

    // ── 门对账（身份 → 背驰确认 → 终端；与 p109 管线序一致，终端门后不再深入）──
    let mut n_identity = 0usize;
    let mut n_divergence = 0usize;
    let mut n_terminal = 0usize;
    let mut n_pass_terminal = 0usize;
    // 键：(turn, seg_a, interval_b) → 触链数与事件快照。
    let mut case_set: BTreeMap<(usize, (usize, usize), (usize, usize)), (usize, Side)> =
        BTreeMap::new();
    let mut chain_trend_events: BTreeMap<usize, Vec<(usize, (usize, usize), (usize, usize))>> =
        BTreeMap::new();
    for (id, chain) in chains.iter().enumerate() {
        let (lk, pi) = chain[0];
        let pair = &levels[lk].pairs[pi];
        let Some(base_candidates) = match_maps[lk].get(&(pair.start, pair.end)) else {
            n_identity += 1;
            continue;
        };
        let events = &levels[lk].events;
        let div_bases: Vec<usize> = base_candidates
            .iter()
            .copied()
            .filter(|&i| events[i].divergence_confirmed)
            .collect();
        if div_bases.is_empty() {
            n_divergence += 1;
            continue;
        }
        let term_bases: Vec<usize> = div_bases
            .iter()
            .copied()
            .filter(|&i| terminal_bits_new(&classification, &events[i]).is_some())
            .collect();
        if !term_bases.is_empty() {
            n_pass_terminal += 1;
            continue;
        }
        n_terminal += 1;
        let trend_keys: Vec<(usize, (usize, usize), (usize, usize))> = div_bases
            .iter()
            .filter(|&&i| events[i].kind == NestDivergenceKind::Trend)
            .map(|&i| {
                let e = &events[i];
                (e.turn_source, e.seg_a, e.interval_b)
            })
            .collect();
        if !trend_keys.is_empty() {
            for &key in &trend_keys {
                let e_side = div_bases
                    .iter()
                    .map(|&i| &events[i])
                    .find(|e| (e.turn_source, e.seg_a, e.interval_b) == key)
                    .expect("trend base event")
                    .side;
                case_set.entry(key).or_insert((0, e_side)).0 += 1;
            }
            chain_trend_events.insert(id, trend_keys);
        }
    }
    println!(
        "P112_ANCHOR_GATES identity@L1={} divergence={} terminal={} pass_terminal={} expected=3781@L1（另 9 条 identity@L2 在 pass_terminal=50 内，p109 口径 3790=3781+9）/1020/1631/50",
        n_identity, n_divergence, n_terminal, n_pass_terminal
    );
    println!(
        "P112_ANCHOR_CASES terminal_with_trend_base={} distinct_trend_base_events={} expected_chains=754",
        chain_trend_events.len(),
        case_set.len()
    );

    // ── 审计材料：L0 段（D2 侧）与级别-1 BSP 输入（生产同源 units/anchors/centers）──
    // D2 侧 anchors 全 Some（level_view.rs:542 同口径），故三买/包络复算直接用段方向。
    let legs0 = lower_legs_from(&tower[0]).map_err(|error| format!("L0 legs 失败: {error:?}"))?;
    let segs_l0: Vec<Segment> = legs0.iter().map(leg_as_segment_pub).collect();
    if tower.len() < 2 || classification.levels.len() < 2 {
        return Err("塔/分类级别不足（须 ≥2）".to_string());
    }
    let units1 = project_to_units(&tower[1], &classification.levels[0].moves);
    let anchors1: Vec<Option<Direction>> = (0..units1.len())
        .map(|i| center_own_dir_at(&classification.levels[0].moves, i))
        .collect();
    let segs1: Vec<Segment> = units1.iter().map(unit_to_segment_pub).collect();
    let centers1: &[Center] = &classification.levels[1].centers;
    let gate1 = center_trend_gate(centers1.len(), &classification.levels[1].moves);
    let bsp_book: &[BspPoint] = &classification.levels[1].bsp;

    // ── 逐案审计 ──
    let mut audits: Vec<CaseAudit> = Vec::new();
    let mut miss_ctx = 0usize;
    for (&(_turn, seg_a, seg_c), &(n_chains, side)) in &case_set {
        let Some(ctx) = pair_audit.get(&(seg_a, seg_c)) else {
            miss_ctx += 1;
            println!(
                "P112_CASE_MISS seg_a={seg_a:?} seg_c={seg_c:?} —— D2 pair 上下文缺失（复现缝隙，须为 0）"
            );
            continue;
        };
        audits.push(audit_case(
            (seg_a, seg_c),
            ctx,
            side,
            n_chains,
            &segs_l0,
            dif,
            dea,
            hist,
            &close_src,
            &segs1,
            &anchors1,
            centers1,
            &gate1,
            bsp_book,
        ));
    }
    println!("P112_AUDIT cases={} miss_ctx={}（miss_ctx 须为 0）", audits.len(), miss_ctx);

    // ── 调试（P112_DEBUG=N）：前 N 案 dump c 窗口段序列与三买逐项几何 ──
    if let Ok(n) = std::env::var("P112_DEBUG").map(|v| v.parse::<usize>().unwrap_or(0)) {
        for (&(_turn, seg_a, seg_c), _) in case_set.iter().take(n) {
            let ctx = pair_audit.get(&(seg_a, seg_c)).expect("ctx");
            let (lo, hi) = (
                segs_l0.partition_point(|s| s.start_index < seg_c.0.max(ctx.last.end_index)),
                segs_l0.partition_point(|s| s.end_index <= seg_c.1),
            );
            println!(
                "P112_DEBUG seg_c={seg_c:?} last_zd={} last_zg={} last_span=({}, {}) win_legs={}",
                ctx.last.zd, ctx.last.zg, ctx.last.start_index, ctx.last.end_index, hi - lo
            );
            for s in &segs_l0[lo..hi] {
                println!(
                    "P112_DEBUG_LEG ({}, {}) {:?} p=({}, {})",
                    s.start_index, s.end_index, s.direction, s.start_price, s.end_price
                );
            }
        }
    }

    // ── 判定（主口径 T4 = DIF 穿越/触及 0 轴；另报三个敏感性口径）──
    let calibers: [(&str, fn(&CaseAudit) -> bool); 4] = [
        ("cross_dif", |a: &CaseAudit| a.t4_cross_dif),
        ("cross_both", |a: &CaseAudit| a.t4_cross_both),
        ("ratio10", |a: &CaseAudit| a.t4_r10),
        ("ratio25", |a: &CaseAudit| a.t4_r25),
    ];
    for (name, f) in &calibers {
        let mut c_i = 0usize;
        let mut c_ii = 0usize;
        let mut c_iii = 0usize;
        for a in &audits {
            match verdict_of(a, f(a)) {
                Verdict::BspFalseKill => c_i += 1,
                Verdict::D2Overwide => c_ii += 1,
                Verdict::TrueNegative => c_iii += 1,
            }
        }
        println!(
            "P112_VERDICT caliber={name} i_bsp_false_kill={c_i} ii_d2_overwide={c_ii} iii_true_negative={c_iii} total={}",
            audits.len()
        );
    }
    // 主口径逐案判定表（event 级）。
    let primary: Vec<Verdict> = audits.iter().map(|a| verdict_of(a, a.t4_cross_dif)).collect();

    // ── 链级归属（优先序 i > ii > iii；链可能有多个趋势基例事件）──
    let key_to_verdict: BTreeMap<((usize, usize), (usize, usize)), Verdict> = audits
        .iter()
        .zip(&primary)
        .map(|(a, v)| ((a.seg_a, a.seg_c), *v))
        .collect();
    let mut chain_counts = [0usize; 4]; // i / ii / iii / mixed_unresolved
    let mut chain_multi = 0usize;
    for keys in chain_trend_events.values() {
        if keys.len() > 1 {
            chain_multi += 1;
        }
        let mut best: Option<Verdict> = None;
        for &(_, seg_a, seg_c) in keys {
            let v = key_to_verdict.get(&(seg_a, seg_c));
            best = match (best, v) {
                (None, x) => x.copied(),
                (Some(Verdict::BspFalseKill), _) => Some(Verdict::BspFalseKill),
                (Some(Verdict::D2Overwide), Some(Verdict::BspFalseKill)) => {
                    Some(Verdict::BspFalseKill)
                }
                (Some(Verdict::D2Overwide), _) => Some(Verdict::D2Overwide),
                (Some(Verdict::TrueNegative), x) => x.copied().or(Some(Verdict::TrueNegative)),
            };
        }
        match best {
            Some(Verdict::BspFalseKill) => chain_counts[0] += 1,
            Some(Verdict::D2Overwide) => chain_counts[1] += 1,
            Some(Verdict::TrueNegative) => chain_counts[2] += 1,
            None => chain_counts[3] += 1,
        }
    }
    println!(
        "P112_VERDICT_CHAIN caliber=cross_dif i={} ii={} iii={} unresolved={} total={} multi_base_chains={}",
        chain_counts[0],
        chain_counts[1],
        chain_counts[2],
        chain_counts[3],
        chain_trend_events.len(),
        chain_multi
    );

    // ── (ii) 类失败项分解（主口径）──
    let mut ii_t3_only = 0usize;
    let mut ii_t4_only = 0usize;
    let mut ii_both = 0usize;
    for (a, v) in audits.iter().zip(&primary) {
        if *v != Verdict::D2Overwide {
            continue;
        }
        match (a.t3, a.t4_cross_dif) {
            (false, true) => ii_t3_only += 1,
            (true, false) => ii_t4_only += 1,
            (false, false) => ii_both += 1,
            (true, true) => {}
        }
    }
    println!(
        "P112_D2_FAIL_ITEMS t3_only={} t4_only={} t3_and_t4={}（ii 类合计 {}）",
        ii_t3_only,
        ii_t4_only,
        ii_both,
        ii_t3_only + ii_t4_only + ii_both
    );

    // ── (i) 类 BSP 门链子型 ──
    let mut gate_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut gate_counts_i: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (a, v) in audits.iter().zip(&primary) {
        *gate_counts.entry(a.bsp_gate).or_default() += 1;
        if *v == Verdict::BspFalseKill {
            *gate_counts_i.entry(a.bsp_gate).or_default() += 1;
        }
    }
    for (gate, count) in &gate_counts {
        let in_i = gate_counts_i.get(gate).copied().unwrap_or(0);
        println!("P112_BSP_GATE {gate} all_cases={count} in_verdict_i={in_i}");
    }

    // ── T3/T4 通过谱与 BSP 账本实况 ──
    let t3_pass = audits.iter().filter(|a| a.t3).count();
    let t4_cd = audits.iter().filter(|a| a.t4_cross_dif).count();
    let t4_cb = audits.iter().filter(|a| a.t4_cross_both).count();
    let t4_r10 = audits.iter().filter(|a| a.t4_r10).count();
    let t4_r25 = audits.iter().filter(|a| a.t4_r25).count();
    let t1_pass = audits.iter().filter(|a| a.t1).count();
    let t2_pass = audits.iter().filter(|a| a.t2).count();
    let t5_pass = audits.iter().filter(|a| a.t5).count();
    println!(
        "P112_ITEMS_PASS t1={} t2={} t3={} t4_cross_dif={} t4_cross_both={} t4_r10={} t4_r25={} t5={} total={}",
        t1_pass, t2_pass, t3_pass, t4_cd, t4_cb, t4_r10, t4_r25, t5_pass, audits.len()
    );
    let mut book_none = 0usize;
    let mut book_present = 0usize;
    for a in &audits {
        if a.bsp_book == "none" {
            book_none += 1;
        } else {
            book_present += 1;
        }
    }
    println!("P112_BSP_BOOK none={book_none} points_present={book_present}");

    // ── T3_ext（全离开段口径）与 c 取段构造事实 ──
    let t3_ext_pass = audits.iter().filter(|a| a.t3_ext).count();
    let win1 = audits.iter().filter(|a| a.win_legs == 1).count();
    let win2p = audits.iter().filter(|a| a.win_legs >= 2).count();
    let win0 = audits.iter().filter(|a| a.win_legs == 0).count();
    println!(
        "P112_T3_EXT t3_strict={} t3_ext={} win_legs==1={} win_legs>=2={} win_legs==0={} total={}",
        t3_pass,
        t3_ext_pass,
        win1,
        win2p,
        win0,
        audits.len()
    );
    // 修法参考池：T1∧T2∧T5∧T4(cross_dif)∧T3_ext 全成立 = 若 c 取全离开段则教义全合取成立
    // （D2 截断是唯一结构失败）⟹ 这些案的 BSP 沉默才构成「BSP 欠配错杀」候选。
    let mut pool = 0usize;
    let mut pool_gate: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut pool_t3ext_fail = 0usize;
    for a in &audits {
        let base = a.t1 && a.t2 && a.t5 && a.t4_cross_dif;
        if base && a.t3_ext {
            pool += 1;
            *pool_gate.entry(a.bsp_gate).or_default() += 1;
        } else if base {
            pool_t3ext_fail += 1;
        }
    }
    println!(
        "P112_POOL_REFORMED_C doctrine_positive_under_full_c={} t3ext_fail_c_never_established={}（口径：T1∧T2∧T5∧T4_cross_dif 预筛）",
        pool, pool_t3ext_fail
    );
    for (gate, count) in &pool_gate {
        println!("P112_POOL_REFORMED_C_GATE {gate} count={count}");
    }

    // ── 逐案明细（坐标 + 各项真值 + 判定 + 门链）──
    for (a, v) in audits.iter().zip(&primary) {
        println!(
            "P112_CASE turn={} side={:?} dir={:?} seg_a=({}, {}) seg_c=({}, {}) chains={} t1={} t2={} t3={} t3_hit={:?} t3_ext={} t3_ext_hit={:?} win_legs={} t4_cd={} t4_cb={} t4_r10={} t4_r25={} dif_min={:.6} dea_min={:.6} band_max={:.6} t5={} area_a={:.4} area_c={:.4} verdict={} bsp_gate={} bsp_book={}",
            a.turn,
            a.side,
            a.direction,
            a.seg_a.0,
            a.seg_a.1,
            a.seg_c.0,
            a.seg_c.1,
            a.chains,
            a.t1 as u8,
            a.t2 as u8,
            a.t3 as u8,
            a.t3_hit,
            a.t3_ext as u8,
            a.t3_ext_hit,
            a.win_legs,
            a.t4_cross_dif as u8,
            a.t4_cross_both as u8,
            a.t4_r10 as u8,
            a.t4_r25 as u8,
            a.dif_min,
            a.dea_min,
            a.band_max,
            a.t5 as u8,
            a.area_a,
            a.area_c,
            v.label(),
            a.bsp_gate,
            a.bsp_book
        );
    }
    // ── 代表样本（每类前 5 案）──
    for target in [
        Verdict::BspFalseKill,
        Verdict::D2Overwide,
        Verdict::TrueNegative,
    ] {
        let mut shown = 0usize;
        for (a, v) in audits.iter().zip(&primary) {
            if *v != target || shown >= 5 {
                continue;
            }
            shown += 1;
            println!(
                "P112_SAMPLE verdict={} turn={} side={:?} seg_a={:?} seg_c={:?} chains={} t3={} t4_cd={} area_a={:.4} area_c={:.4} bsp_gate={}",
                v.label(),
                a.turn,
                a.side,
                a.seg_a,
                a.seg_c,
                a.chains,
                a.t3 as u8,
                a.t4_cross_dif as u8,
                a.area_a,
                a.area_c,
                a.bsp_gate
            );
        }
    }
    println!("P112_DONE cases={}", audits.len());
    Ok(())
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
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
    let first_date = raw.dates.first().cloned().unwrap_or_default();
    let last_date = raw.dates.last().cloned().unwrap_or_default();
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
    Ok(LoadedBars {
        bars,
        first_date,
        last_date,
    })
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
