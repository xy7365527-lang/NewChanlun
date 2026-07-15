//! task #84：对 #83 全量终点快照中的 D1 InvalidSeed 与
//! Pending(MissingDivergencePair) 做 fail-closed 缺口分型。
//!
//! 只读消费生产塔、投影、分解与 level-view seam；不修改生产对象、不执行语义裁决。
//! 用法：`cargo run --release --bin p84_failclosed_audit -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::center::{
    center_from_segments, center_from_window, dir_alternates, UnitRange,
};
use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::divergence::{
    compute_macd, departure_move_c_start, locate_departure_move_a, self_anchors,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows, C2LevelViewConfig,
    C2VersionTuple, CompletionStatus, CoordinateWindow, ExactThreeProjection, LevelViewMaterial,
    LevelViewQuery, LowerLeg, PendingReason, ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{ElementId, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Direction, Segment, Timestamp};
use serde::Deserialize;
use std::fmt;
use std::path::Path;

const EXPECTED_D1: [usize; 5] = [165, 43, 7, 0, 0];
const EXPECTED_MISSING_PAIR: [usize; 5] = [0, 17, 2, 1, 1];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AAtom {
    A1,
    A2,
    A3,
}

impl AAtom {
    fn index(self) -> usize {
        match self {
            Self::A1 => 0,
            Self::A2 => 1,
            Self::A3 => 2,
        }
    }
}

impl fmt::Display for AAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BAtom {
    B1,
    B2,
    B3,
    B4,
    Success,
}

impl BAtom {
    fn index(self) -> usize {
        match self {
            Self::B1 => 0,
            Self::B2 => 1,
            Self::B3 => 2,
            Self::B4 => 3,
            Self::Success => 4,
        }
    }
}

impl fmt::Display for BAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
struct D1Detail {
    level: usize,
    id: ElementId,
    sub_count: usize,
    atom: AAtom,
    a1_sub: Option<usize>,
    zd: Option<i64>,
    zg: Option<i64>,
    first_three: String,
    other_valid_starts: Vec<usize>,
}

#[derive(Debug)]
struct BDetail {
    level: usize,
    run_index: usize,
    block_start: usize,
    block_end: usize,
    direction: Direction,
    atom: BAtom,
    evidence: &'static str,
    move_start: usize,
    move_end: usize,
    prev_center_end: Option<usize>,
    last_center_end: Option<usize>,
    a_window_segments: usize,
    a_window_same_dir_anchors: usize,
    a_reentry_boundary: Option<usize>,
    a_same_dir_after_boundary: usize,
    post_last_segments: usize,
    post_last_same_dir: usize,
    c_terminal: Option<(usize, usize)>,
    c_start: Option<usize>,
}

#[derive(Debug, Default)]
struct LevelAudit {
    tower_windows: usize,
    too_short: usize,
    d1_atoms: [usize; 3],
    d1_other_valid: [usize; 3],
    trend_blocks: usize,
    b_atoms_all_trends: [usize; 5],
    missing_atoms: [usize; 4],
}

impl LevelAudit {
    fn d1_total(&self) -> usize {
        self.d1_atoms.iter().sum()
    }

    fn missing_total(&self) -> usize {
        self.missing_atoms.iter().sum()
    }
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p84_failclosed_audit <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let as_of = loaded.bars.len() - 1;
    let layer = parse_layer(&loaded.bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes: Vec<_> = layer
        .merged_bars
        .iter()
        .map(|bar| bar.close as f64)
        .collect();
    let close_src: Vec<_> = layer
        .merged_bars
        .iter()
        .map(|bar| bar.source_index)
        .collect();
    let hist = compute_macd(&closes, &config.macd).hist;

    if tower.len() != EXPECTED_D1.len() + 1 {
        return Err(format!(
            "守恒失败：tower_levels={}，预期含 L0 的 {}",
            tower.len(),
            EXPECTED_D1.len() + 1
        ));
    }

    let mut levels = Vec::with_capacity(EXPECTED_D1.len());
    let mut d1_details = Vec::new();
    let mut b_details = Vec::new();
    for level in 1..tower.len() {
        levels.push(audit_level(
            level,
            &tower[level],
            &tower[level - 1],
            as_of,
            &hist,
            &close_src,
            &mut d1_details,
            &mut b_details,
        )?);
    }

    validate_conservation(&levels, &d1_details, &b_details)?;

    println!(
        "P84_CONSERVATION status=PASS bars={} merged={} as_of={} first_date={} last_date={} tower_levels={} classification_levels={} d1_invalid_seed={} d1_too_short={} missing_divergence_pair={}",
        loaded.bars.len(),
        layer.merged_bars.len(),
        as_of,
        loaded.first_date,
        loaded.last_date,
        tower.len(),
        classification.levels.len(),
        d1_details.len(),
        levels.iter().map(|s| s.too_short).sum::<usize>(),
        b_details.len(),
    );
    println!(
        "P84_ATOM_ORDER D1=A1_as_unit_none>A2_L1_dir_alternates>A3_zd_gt_zg D2=B1_guard>B2_no_A>B3_no_C_terminal>B4_no_C_start"
    );

    for (offset, stats) in levels.iter().enumerate() {
        let level = offset + 1;
        println!(
            "P84_LEVEL L{level} tower_windows={} d1_too_short={} d1_invalid_seed={} A1={} A2={} A3={} other_valid_total={} A1_other={} A2_other={} A3_other={} trend_blocks={} B1_all={} B2_all={} B3_all={} B4_all={} B_success={} missing_pair={} missing_B1={} missing_B2={} missing_B3={} missing_B4={}",
            stats.tower_windows,
            stats.too_short,
            stats.d1_total(),
            stats.d1_atoms[0],
            stats.d1_atoms[1],
            stats.d1_atoms[2],
            stats.d1_other_valid.iter().sum::<usize>(),
            stats.d1_other_valid[0],
            stats.d1_other_valid[1],
            stats.d1_other_valid[2],
            stats.trend_blocks,
            stats.b_atoms_all_trends[0],
            stats.b_atoms_all_trends[1],
            stats.b_atoms_all_trends[2],
            stats.b_atoms_all_trends[3],
            stats.b_atoms_all_trends[4],
            stats.missing_total(),
            stats.missing_atoms[0],
            stats.missing_atoms[1],
            stats.missing_atoms[2],
            stats.missing_atoms[3],
        );
    }

    for detail in &d1_details {
        let starts = if detail.other_valid_starts.is_empty() {
            "-".to_string()
        } else {
            detail
                .other_valid_starts
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "P84_D1 level=L{} id={}:{} sub_moves={} atom={} a1_sub={} zd={} zg={} first_three={} other_valid_exists={} other_valid_count={} other_valid_starts={}",
            detail.level,
            detail.id.level,
            detail.id.ordinal,
            detail.sub_count,
            detail.atom,
            detail.a1_sub.map_or_else(|| "-".to_string(), |v| (v + 1).to_string()),
            detail.zd.map_or_else(|| "-".to_string(), |v| v.to_string()),
            detail.zg.map_or_else(|| "-".to_string(), |v| v.to_string()),
            detail.first_three,
            !detail.other_valid_starts.is_empty(),
            detail.other_valid_starts.len(),
            starts,
        );
    }

    for detail in &b_details {
        println!(
            "P84_B level=L{} run={} block={}:{} direction={:?} atom={} evidence={} move={}:{} prev_center_end={} last_center_end={} as_of={} a_window_segments={} a_window_same_dir_anchors={} a_reentry_boundary={} a_same_dir_after_boundary={} post_last_segments={} post_last_same_dir={} c_terminal={} c_start={}",
            detail.level,
            detail.run_index,
            detail.block_start,
            detail.block_end,
            detail.direction,
            detail.atom,
            detail.evidence,
            detail.move_start,
            detail.move_end,
            opt_usize(detail.prev_center_end),
            opt_usize(detail.last_center_end),
            as_of,
            detail.a_window_segments,
            detail.a_window_same_dir_anchors,
            opt_usize(detail.a_reentry_boundary),
            detail.a_same_dir_after_boundary,
            detail.post_last_segments,
            detail.post_last_same_dir,
            detail.c_terminal.map_or_else(|| "-".to_string(), |(s, e)| format!("{s}:{e}")),
            opt_usize(detail.c_start),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn audit_level(
    level: usize,
    windows: &[LeveledMove],
    lower: &[LeveledMove],
    as_of: usize,
    hist: &[f64],
    close_src: &[usize],
    d1_details: &mut Vec<D1Detail>,
    b_details: &mut Vec<BDetail>,
) -> Result<LevelAudit, String> {
    let lower_legs =
        lower_legs_from(lower).map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
    let mut stats = LevelAudit {
        tower_windows: windows.len(),
        ..LevelAudit::default()
    };

    let mut run_start = None;
    let mut run_index = 0;
    for index in 0..=windows.len() {
        let valid = if index < windows.len() {
            match project_extended_windows(std::slice::from_ref(&windows[index])) {
                Ok(_) => true,
                Err(ProjectionError::TooShort { .. }) => {
                    stats.too_short += 1;
                    false
                }
                Err(ProjectionError::InvalidSeed { .. }) => {
                    let detail = classify_d1(level, &windows[index])?;
                    stats.d1_atoms[detail.atom.index()] += 1;
                    if !detail.other_valid_starts.is_empty() {
                        stats.d1_other_valid[detail.atom.index()] += 1;
                    }
                    d1_details.push(detail);
                    false
                }
                Err(ProjectionError::InvalidLowerLeg { .. }) => {
                    return Err(format!("L{level} 单窗口意外返回 InvalidLowerLeg"));
                }
                Err(ProjectionError::MissingCarriedCenter { .. }) => {
                    return Err(format!(
                        "L{level} 单窗口意外返回 MissingCarriedCenter（#90：塔应逐窗携带核）"
                    ));
                }
            }
        } else {
            false
        };
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                audit_run(
                    level,
                    run_index,
                    &windows[start..index],
                    &lower_legs,
                    as_of,
                    hist,
                    close_src,
                    &mut stats,
                    b_details,
                )?;
                run_index += 1;
                run_start = None;
            }
            _ => {}
        }
    }
    Ok(stats)
}

fn classify_d1(level: usize, window: &LeveledMove) -> Result<D1Detail, String> {
    if window.sub_moves.len() < 3 {
        return Err(format!("L{level} {:?} InvalidSeed 却少于三段", window.id));
    }
    let units: Vec<_> = window.sub_moves.iter().take(3).map(as_unit).collect();
    let a1_sub = units.iter().position(Option::is_none);
    let mut zd = None;
    let mut zg = None;
    let atom = if a1_sub.is_some() {
        AAtom::A1
    } else {
        let a = units[0].expect("checked Some");
        let b = units[1].expect("checked Some");
        let c = units[2].expect("checked Some");
        if level == 1 && !dir_alternates(&a, &b, &c) {
            AAtom::A2
        } else {
            let computed_zd = a.lo.max(b.lo).max(c.lo);
            let computed_zg = a.hi.min(b.hi).min(c.hi);
            zd = Some(computed_zd);
            zg = Some(computed_zg);
            if computed_zd <= computed_zg {
                return Err(format!(
                    "L{level} {:?} InvalidSeed 无法归入 A1/A2/A3: zd={computed_zd} zg={computed_zg}",
                    window.id
                ));
            }
            AAtom::A3
        }
    };

    let all_valid_starts: Vec<_> = if window.sub_moves.len() > 3 {
        (0..=window.sub_moves.len() - 3)
            .filter(|start| valid_triplet(level, &window.sub_moves[*start..*start + 3]))
            .collect()
    } else {
        Vec::new()
    };
    if all_valid_starts.first() == Some(&0) {
        return Err(format!(
            "L{level} {:?} 生产首三段 InvalidSeed，但同级滑窗复核 start=0 合法",
            window.id
        ));
    }
    let other_valid_starts = all_valid_starts
        .into_iter()
        .filter(|start| *start > 0)
        .collect();
    let first_three = window
        .sub_moves
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, value)| format_sub(i + 1, value))
        .collect::<Vec<_>>()
        .join("|");
    Ok(D1Detail {
        level,
        id: window.id,
        sub_count: window.sub_moves.len(),
        atom,
        a1_sub,
        zd,
        zg,
        first_three,
        other_valid_starts,
    })
}

fn valid_triplet(level: usize, values: &[LeveledMove]) -> bool {
    let [Some(a), Some(b), Some(c)] = [
        as_unit(&values[0]),
        as_unit(&values[1]),
        as_unit(&values[2]),
    ] else {
        return false;
    };
    if level == 1 {
        center_from_segments(&a, &b, &c).is_some()
    } else {
        center_from_window(&a, &b, &c).is_some()
    }
}

fn as_unit(value: &LeveledMove) -> Option<UnitRange> {
    Some(UnitRange {
        start_index: value.start_index,
        end_index: value.end_index,
        direction: first_leaf_direction(value)?,
        lo: value.rmove.lo(),
        hi: value.rmove.hi(),
    })
}

fn first_leaf_direction(value: &LeveledMove) -> Option<Direction> {
    let mut node = &value.rmove;
    loop {
        match node {
            RMove::Segment { direction, .. } => return Some(*direction),
            RMove::Compose { subs, .. } => node = subs.first()?,
        }
    }
}

fn format_sub(index: usize, value: &LeveledMove) -> String {
    let direction = first_leaf_direction(value)
        .map(|v| format!("{v:?}"))
        .unwrap_or_else(|| "None".to_string());
    format!(
        "s{index}[{}:{}:{direction}:{}:{}]",
        value.start_index,
        value.end_index,
        value.rmove.lo(),
        value.rmove.hi()
    )
}

#[allow(clippy::too_many_arguments)]
fn audit_run(
    level: usize,
    run_index: usize,
    windows: &[LeveledMove],
    lower_legs: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    close_src: &[usize],
    stats: &mut LevelAudit,
    b_details: &mut Vec<BDetail>,
) -> Result<(), String> {
    let projection = project_extended_windows(windows)
        .map_err(|error| format!("L{level} run={run_index} D1 投影失败: {error:?}"))?;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let start = projection
        .seeds
        .first()
        .ok_or_else(|| format!("L{level} run={run_index} 空投影"))?
        .start_index;
    let end = projection.seeds.last().expect("nonempty").end_index;
    let query = LevelViewQuery {
        level: level as u32,
        coordinate_window: CoordinateWindow { start, end },
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
            close_src,
        },
    )
    .map_err(|error| format!("L{level} run={run_index} assemble 失败: {error:?}"))?;
    if view.moves.len() != blocks.len() {
        return Err(format!("L{level} run={run_index} move/block 数不等"));
    }

    let segments: Vec<_> = lower_legs.iter().map(leg_as_segment).collect();
    let anchors = self_anchors(&segments);
    for (block, assembled) in blocks.iter().zip(&view.moves) {
        if block.dir.is_none() {
            continue;
        }
        let direction = block.dir.expect("trend direction");
        stats.trend_blocks += 1;
        let diagnosis = diagnose_provider(
            block.start_center,
            block.end_center,
            direction,
            &projection,
            &segments,
            &anchors,
            as_of,
        );
        stats.b_atoms_all_trends[diagnosis.atom.index()] += 1;
        let missing = matches!(
            assembled.completion,
            CompletionStatus::Pending {
                reason: PendingReason::MissingDivergencePair,
                ..
            }
        );
        if missing {
            if diagnosis.atom == BAtom::Success {
                return Err(format!(
                    "L{level} run={run_index} MissingPair 却诊断为 Success"
                ));
            }
            stats.missing_atoms[diagnosis.atom.index()] += 1;
            b_details.push(BDetail {
                level,
                run_index,
                block_start: block.start_center,
                block_end: block.end_center,
                direction,
                atom: diagnosis.atom,
                evidence: diagnosis.evidence,
                move_start: assembled.start_index,
                move_end: assembled.end_index,
                prev_center_end: diagnosis.prev_center_end,
                last_center_end: diagnosis.last_center_end,
                a_window_segments: diagnosis.a_window_segments,
                a_window_same_dir_anchors: diagnosis.a_window_same_dir_anchors,
                a_reentry_boundary: diagnosis.a_reentry_boundary,
                a_same_dir_after_boundary: diagnosis.a_same_dir_after_boundary,
                post_last_segments: diagnosis.post_last_segments,
                post_last_same_dir: diagnosis.post_last_same_dir,
                c_terminal: diagnosis.c_terminal,
                c_start: diagnosis.c_start,
            });
        } else if diagnosis.atom != BAtom::Success {
            return Err(format!(
                "L{level} run={run_index} 非 MissingPair 却诊断为 {}",
                diagnosis.atom
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ProviderDiagnosis {
    atom: BAtom,
    evidence: &'static str,
    prev_center_end: Option<usize>,
    last_center_end: Option<usize>,
    a_window_segments: usize,
    a_window_same_dir_anchors: usize,
    a_reentry_boundary: Option<usize>,
    a_same_dir_after_boundary: usize,
    post_last_segments: usize,
    post_last_same_dir: usize,
    c_terminal: Option<(usize, usize)>,
    c_start: Option<usize>,
}

fn diagnose_provider(
    block_start: usize,
    block_end: usize,
    direction: Direction,
    projection: &ExactThreeProjection,
    segments: &[Segment],
    anchors: &[Option<Direction>],
    as_of: usize,
) -> ProviderDiagnosis {
    let mut out = ProviderDiagnosis {
        atom: BAtom::B1,
        evidence: "其他",
        prev_center_end: None,
        last_center_end: None,
        a_window_segments: 0,
        a_window_same_dir_anchors: 0,
        a_reentry_boundary: None,
        a_same_dir_after_boundary: 0,
        post_last_segments: 0,
        post_last_same_dir: 0,
        c_terminal: None,
        c_start: None,
    };
    if block_end <= block_start || block_end >= projection.seeds.len() {
        return out;
    }
    let prev = projection.seeds[block_end - 1].center;
    let last = projection.seeds[block_end].center;
    out.prev_center_end = Some(prev.end_index);
    out.last_center_end = Some(last.end_index);
    let a_lo = segments.partition_point(|s| s.start_index < prev.end_index);
    let a_hi = segments.partition_point(|s| s.start_index < last.end_index);
    out.a_window_segments = a_hi - a_lo;
    out.a_window_same_dir_anchors = anchors[a_lo..a_hi]
        .iter()
        .filter(|value| **value == Some(direction))
        .count();
    let reenters = |s: &Segment| {
        s.direction != direction
            && match direction {
                Direction::Down => s.end_price >= prev.zd,
                Direction::Up => s.end_price <= prev.zg,
            }
    };
    let boundary = segments[a_lo..a_hi]
        .iter()
        .rev()
        .find(|s| reenters(s))
        .map(|s| s.end_index);
    out.a_reentry_boundary = boundary;
    let boundary_value = boundary.unwrap_or(0);
    out.a_same_dir_after_boundary = segments[a_lo..a_hi]
        .iter()
        .zip(&anchors[a_lo..a_hi])
        .filter(|(s, anchor)| **anchor == Some(direction) && s.start_index >= boundary_value)
        .count();
    if locate_departure_move_a(segments, anchors, &prev, &last, direction).is_none() {
        out.atom = BAtom::B2;
        out.evidence = "真无A离开episode";
        return out;
    }
    let post: Vec<_> = segments
        .iter()
        .filter(|s| s.start_index >= last.end_index && s.end_index <= as_of)
        .collect();
    out.post_last_segments = post.len();
    out.post_last_same_dir = post.iter().filter(|s| s.direction == direction).count();
    let Some(c_terminal) = segments.iter().rev().find(|segment| {
        segment.direction == direction
            && segment.start_index >= last.end_index
            && segment.end_index <= as_of
    }) else {
        out.atom = BAtom::B3;
        out.evidence = "史末截断";
        return out;
    };
    out.c_terminal = Some((c_terminal.start_index, c_terminal.end_index));
    let Some(c_start) =
        departure_move_c_start(segments, anchors, &last, direction, c_terminal.start_index)
    else {
        out.atom = BAtom::B4;
        out.evidence = "其他";
        return out;
    };
    out.c_start = Some(c_start);
    out.atom = BAtom::Success;
    out.evidence = "provider成功";
    out
}

fn leg_as_segment(value: &LowerLeg) -> Segment {
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

fn validate_conservation(
    levels: &[LevelAudit],
    d1_details: &[D1Detail],
    b_details: &[BDetail],
) -> Result<(), String> {
    let actual_d1: Vec<_> = levels.iter().map(LevelAudit::d1_total).collect();
    let actual_missing: Vec<_> = levels.iter().map(LevelAudit::missing_total).collect();
    let too_short: usize = levels.iter().map(|s| s.too_short).sum();
    if actual_d1 != EXPECTED_D1 || d1_details.len() != 215 || too_short != 0 {
        return Err(format!(
            "#83 D1 守恒失败：actual={actual_d1:?} total={} too_short={too_short}; expected={EXPECTED_D1:?} total=215 too_short=0",
            d1_details.len()
        ));
    }
    if actual_missing != EXPECTED_MISSING_PAIR || b_details.len() != 21 {
        return Err(format!(
            "#83 MissingDivergencePair 守恒失败：actual={actual_missing:?} total={}; expected={EXPECTED_MISSING_PAIR:?} total=21",
            b_details.len()
        ));
    }
    Ok(())
}

fn opt_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
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
