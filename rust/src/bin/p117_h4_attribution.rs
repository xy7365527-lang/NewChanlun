//! task #117（主线阶段1关② T1 验收·H4 红灯归因）：37 案逐案机制归因探针（只读，不改任何生产源码/账本）。
//!
//! 用途：H4 探针 `/tmp/p117_recheck.out` rescued=0/37 的逐案根因裁决。四假设并列
//! （T1 实装有 bug / 探针重建有误 / 设计预测错误 / fiat 反读成立），本探针只产证据不预设结论。
//!
//! ══════════════ 090 声明（声明必须与实际能力一致）══════════════
//! - **只读诊断探针**：stdout 唯一输出；不改账本/塔/判据/事件结构；主仓零写入；零 git mutation；
//!   `rust/Cargo.toml` 零改动（bin 走 autobins 自动发现，同 p112/p116/p117 先例）。
//! - **数据唯一来源**：与 p117 探针同一终态（塔增量重放 ≡ batch bit-equal，p116 P116_BIT_EXACT 锁定；
//!   数据加载/塔重放/事件收集/pair 上下文骨架逐字复制 p117_s1a_level_recheck.rs）。
//! - **复刻清单与自校验**（凡 pub(crate)/private 不可直达者逐字复刻，复刻件均以生产实测自校验）：
//!   1. `trend_confirm_time`（level_view.rs:535-626 私有）→ `trend_confirm_time_diag` 逐字复刻 +
//!      失败原因记录。自校验 ATTR_REPLICA_TSTAR：30 个 confirmed 事件复刻 t* 必须逐位等于
//!      `event.turn_source`（不等即复刻失真，全部相关结论作废）。
//!   2. `trend_third_class_in_c`（signal.rs:497-521 pub(crate)）→ 逐字复刻（窗口/命中条件不动）。
//!   3. `move_range_envelope`（divergence.rs:864-876 pub(crate)）→ 逐字复刻（span 过滤 + fold）。
//!   4. `judge_first_cached` L0 判据链（signal.rs:285-398）→ `judge_replica` 诊断复刻（anchor=自锚
//!      L0 语义、first_match 三元组语义、gate/broke/037:20/λ_C/面积 逐环判负原因）。自校验
//!      ATTR_REPLICA_JUDGE：6 个 hit 点复刻必须 = Confirm；书中零 bit struct_break 点复刻必须 =
//!      AreaFail；出现 fork 即打印逐案告警（预期 0）。
//!   5. 中枢延伸吸收（recursive_tower.rs:734-744）→ `seal_end` 复刻扫描。自校验 ATTR_SEAL_CHECK：
//!      复刻 seal_end 必须等于 `levels[0].centers` 实测 `end_index`（37/37 预期，不等逐案告警）。
//! - **owner 语义**：hit/账本点的「判决中枢」= 生产 `nearest_confirmed_center_idx`
//!   （signal.rs:211-218 终态数组 partition_point 语义）复刻——extract 主循环（signal.rs:1076-1089）
//!   以终态 centers 数组按 `end_index <= seg.start` 取归属中枢，本探针同一表达式，禁第二口径。
//!
//! ══════════════ 输出格式（stdout）══════════════
//!   逐案：ATTR_CASE / ATTR_B（seed vs detect 中枢对拍）/ ATTR_GATES（b/prev 门值与块位置）/
//!   ATTR_SEAL（延伸吸收对拍）/ ATTR_TURN（c 破核腿判决复刻 + 该书点实况）/
//!   ATTR_PT（窗口邻域书点逐点 dump：bits/struct_break_dir/confirm/owner）/
//!   ATTR_LEFT / ATTR_RIGHT（全书中窗口两侧最近 confirm_side 点）/
//!   ATTR_BPOINT（c_start 起 owner=B 的首个 confirm 点与首个候选点）/
//!   ATTR_HIT（6 命中案 hit 点身份复刻）/ ATTR_R1（7 事件缺失案 R1 合取分解）。
//!   汇总：ATTR_REPLICA_TSTAR / ATTR_REPLICA_JUDGE / ATTR_SEAL_CHECK /
//!   ATTR_Q1..Q4_SUMMARY / ATTR_FORK（任何复刻失真告警，预期 0 行）。
//!
//! 用法：`cargo run --release --bin p117_h4_attribution -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::bsp::BspPoint;
use newchan_rust::theta_v0::classifier::decompose::{self, center_trend_gate};
use newchan_rust::theta_v0::classifier::divergence::{
    departure_move_c_start, dif_crosses_zero, locate_departure_move_a, same_color_area,
    same_dir_hist_peak, segment_dif_peak, AbcDivergence,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, LowerLeg, NestCandidateEvent, NestDivergenceKind,
    ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    event_bsp_book_level, terminal_bits_at_event, TerminalMatch,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{map_src_to_close_idx, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{
    quantize, Bar, Center, Direction, Segment, Side, Tick, Timestamp,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

// ══════════════ 37 案硬锚坐标（逐字复制 p117_s1a_level_recheck.rs，同源附录 B）══════════════
#[derive(Debug, Clone, Copy)]
struct CaseCoord {
    turn: usize,
    side: Side,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
}

#[rustfmt::skip]
const CASES: [CaseCoord; 37] = [
    CaseCoord { turn: 867223,  side: Side::Short, seg_a: (866696, 867022),   seg_c: (867071, 867223) },   // #1
    CaseCoord { turn: 977720,  side: Side::Long,  seg_a: (976723, 977458),   seg_c: (977629, 977720) },   // #2
    CaseCoord { turn: 1154662, side: Side::Long,  seg_a: (1154302, 1154427), seg_c: (1154603, 1154662) }, // #3
    CaseCoord { turn: 1444502, side: Side::Long,  seg_a: (1443997, 1444195), seg_c: (1444331, 1444502) }, // #4
    CaseCoord { turn: 1511155, side: Side::Short, seg_a: (1510289, 1510925), seg_c: (1511033, 1511155) }, // #5
    CaseCoord { turn: 1545297, side: Side::Long,  seg_a: (1544839, 1545074), seg_c: (1545202, 1545297) }, // #6
    CaseCoord { turn: 1684581, side: Side::Short, seg_a: (1683941, 1684307), seg_c: (1684405, 1684581) }, // #7
    CaseCoord { turn: 1812973, side: Side::Short, seg_a: (1812461, 1812918), seg_c: (1812944, 1812973) }, // #8
    CaseCoord { turn: 1890249, side: Side::Short, seg_a: (1889712, 1889979), seg_c: (1890175, 1890249) }, // #9
    CaseCoord { turn: 1963252, side: Side::Long,  seg_a: (1962195, 1962964), seg_c: (1963034, 1963252) }, // #10
    CaseCoord { turn: 2035662, side: Side::Short, seg_a: (2035153, 2035494), seg_c: (2035524, 2035662) }, // #11 延迟
    CaseCoord { turn: 2043126, side: Side::Short, seg_a: (2042665, 2042926), seg_c: (2042972, 2043126) }, // #12
    CaseCoord { turn: 2090467, side: Side::Short, seg_a: (2089931, 2090155), seg_c: (2090302, 2090467) }, // #13
    CaseCoord { turn: 2136917, side: Side::Short, seg_a: (2136303, 2136796), seg_c: (2136843, 2136917) }, // #14
    CaseCoord { turn: 2178474, side: Side::Short, seg_a: (2178045, 2178296), seg_c: (2178454, 2178474) }, // #15
    CaseCoord { turn: 2195437, side: Side::Short, seg_a: (2194211, 2195150), seg_c: (2195268, 2195437) }, // #16
    CaseCoord { turn: 2241463, side: Side::Long,  seg_a: (2240644, 2241205), seg_c: (2241312, 2241463) }, // #17
    CaseCoord { turn: 2669193, side: Side::Long,  seg_a: (2668016, 2668543), seg_c: (2668906, 2669193) }, // #18 延迟
    CaseCoord { turn: 3595896, side: Side::Long,  seg_a: (3595279, 3595683), seg_c: (3595811, 3595896) }, // #19
    CaseCoord { turn: 3602474, side: Side::Long,  seg_a: (3602103, 3602278), seg_c: (3602333, 3602474) }, // #20
    CaseCoord { turn: 3818615, side: Side::Long,  seg_a: (3818234, 3818361), seg_c: (3818541, 3818615) }, // #21
    CaseCoord { turn: 3828263, side: Side::Long,  seg_a: (3827769, 3828015), seg_c: (3828115, 3828263) }, // #22 延迟
    CaseCoord { turn: 3916879, side: Side::Long,  seg_a: (3916169, 3916573), seg_c: (3916719, 3916879) }, // #23
    CaseCoord { turn: 3971708, side: Side::Short, seg_a: (3970897, 3971250), seg_c: (3971407, 3971708) }, // #24 延迟
    CaseCoord { turn: 3989658, side: Side::Short, seg_a: (3988504, 3989337), seg_c: (3989432, 3989658) }, // #25
    CaseCoord { turn: 4015179, side: Side::Short, seg_a: (4014529, 4014952), seg_c: (4015041, 4015179) }, // #26
    CaseCoord { turn: 4018257, side: Side::Long,  seg_a: (4017832, 4018070), seg_c: (4018100, 4018257) }, // #27
    CaseCoord { turn: 4032359, side: Side::Short, seg_a: (4031579, 4032102), seg_c: (4032284, 4032359) }, // #28
    CaseCoord { turn: 4044258, side: Side::Short, seg_a: (4043558, 4044121), seg_c: (4044196, 4044258) }, // #29
    CaseCoord { turn: 4108519, side: Side::Short, seg_a: (4107943, 4108212), seg_c: (4108342, 4108519) }, // #30
    CaseCoord { turn: 4172278, side: Side::Short, seg_a: (4171561, 4171885), seg_c: (4172028, 4172278) }, // #31
    CaseCoord { turn: 4186102, side: Side::Short, seg_a: (4185578, 4185847), seg_c: (4185938, 4186102) }, // #32 延迟
    CaseCoord { turn: 4189745, side: Side::Short, seg_a: (4189197, 4189527), seg_c: (4189641, 4189745) }, // #33
    CaseCoord { turn: 4200339, side: Side::Long,  seg_a: (4199942, 4200135), seg_c: (4200242, 4200339) }, // #34
    CaseCoord { turn: 4298765, side: Side::Short, seg_a: (4298004, 4298330), seg_c: (4298554, 4298765) }, // #35
    CaseCoord { turn: 4321214, side: Side::Short, seg_a: (4320459, 4321032), seg_c: (4321166, 4321214) }, // #36
    CaseCoord { turn: 4324005, side: Side::Long,  seg_a: (4323446, 4323756), seg_c: (4323850, 4324005) }, // #37
];

fn side_of_dir(direction: Direction) -> Side {
    match direction {
        Direction::Up => Side::Short,
        Direction::Down => Side::Long,
    }
}

fn dir_name(d: Option<Direction>) -> String {
    match d {
        Some(Direction::Up) => "Up".to_string(),
        Some(Direction::Down) => "Down".to_string(),
        None => "None".to_string(),
    }
}

fn bits_str(p: &BspPoint) -> String {
    let b = &p.bits;
    format!(
        "{}{}{}{}{}{}",
        u8::from(b.buy1),
        u8::from(b.buy2),
        u8::from(b.buy3),
        u8::from(b.sell1),
        u8::from(b.sell2),
        u8::from(b.sell3)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EventKey {
    level: u32,
    short: bool,
    kind: NestDivergenceKind,
    seg_a: (usize, usize),
    interval_b: (usize, usize),
    interval_a: (usize, usize),
    turn_source: usize,
}

impl From<&NestCandidateEvent> for EventKey {
    fn from(event: &NestCandidateEvent) -> Self {
        Self {
            level: event.level,
            short: event.side == Side::Short,
            kind: event.kind,
            seg_a: event.seg_a,
            interval_b: event.interval_b,
            interval_a: event.interval_a,
            turn_source: event.turn_source,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ProviderAudit {
    snapshots: usize,
    views: usize,
    projection_too_short: usize,
    projection_invalid_seed: usize,
    projection_missing_carried_center: usize,
    projection_other: usize,
}

impl ProviderAudit {
    fn observe_projection(&mut self, error: ProjectionError) {
        match error {
            ProjectionError::TooShort { .. } => self.projection_too_short += 1,
            ProjectionError::InvalidSeed { .. } => self.projection_invalid_seed += 1,
            ProjectionError::MissingCarriedCenter { .. } => {
                self.projection_missing_carried_center += 1
            }
            ProjectionError::InvalidLowerLeg { .. } => self.projection_other += 1,
        }
    }
}

struct TerminalState {
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

/// 增量塔重放（逐字复制 p117/p116 run_terminal_pass）。
fn run_terminal_pass(bars: &[Bar], config: &ThetaConfig) -> Result<TerminalState, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut terminal = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "ATTR_PROGRESS bar={index}/{} elapsed={:.1}s",
                bars.len(),
                started.elapsed().as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((classification, tower));
        }
    }
    let (classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        classification,
        tower,
        cache,
    })
}

/// 终态快照候选收集（逐字复制 p117/p116 collect_snapshot_candidates）。
fn collect_snapshot_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    audit: &mut ProviderAudit,
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    audit.snapshots += 1;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = if index < windows.len() {
                match project_extended_windows_carried_only(std::slice::from_ref(&windows[index])) {
                    Ok(_) => true,
                    Err(error) => {
                        audit.observe_projection(error);
                        false
                    }
                }
            } else {
                false
            };
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    let projection = project_extended_windows_carried_only(&windows[start..index])
                        .map_err(|error| format!("L{level} run projection 失败: {error:?}"))?;
                    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                    let blocks = decompose::decompose(&centers);
                    let start_source = projection.seeds.first().expect("nonempty run").start_index;
                    let end_source = projection.seeds.last().expect("nonempty run").end_index;
                    let query = LevelViewQuery {
                        level: level as u32,
                        coordinate_window: CoordinateWindow {
                            start: start_source,
                            end: end_source,
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
                            lower_legs: &lower,
                            hist,
                            dif,
                            close_src,
                        },
                    )
                    .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
                    audit.views += 1;
                    by_level[level].extend(provide_nest_candidate_events(
                        level as u32,
                        &projection,
                        &blocks,
                        &lower,
                        &view,
                        hist,
                        dif,
                        close_src,
                    ));
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    for events in &mut by_level {
        events.sort_by_key(|event| EventKey::from(&*event));
        events.dedup_by(|left, right| EventKey::from(&*left) == EventKey::from(&*right));
    }
    Ok(by_level)
}

/// level_view.rs `leg_as_segment` 同口径复刻（p117 同款）。
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

/// signal.rs:211 `nearest_confirmed_center_idx` 同口径复刻（终态数组 partition_point 语义）。
fn nearest_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(hi - 1)
    }
}

/// D2 pair 的 L0 门链诊断上下文（逐字复制 p117 PairCtx）。
#[derive(Debug, Clone, Copy)]
struct PairCtx {
    direction: Direction,
    prev: Center,
    last: Center,
    seg_a: (usize, usize),
    c_start: usize,
}

/// L1 run 逐对收集 D2 pair 上下文（逐字复制 p117 collect_l1_pair_contexts）。
fn collect_l1_pair_contexts(
    tower: &[Rc<Vec<LeveledMove>>],
    lower: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<(BTreeMap<usize, PairCtx>, usize), String> {
    let mut out: BTreeMap<usize, PairCtx> = BTreeMap::new();
    let mut collisions = 0usize;
    if tower.len() < 2 {
        return Ok((out, collisions));
    }
    let windows = &tower[1];
    let segments: Vec<Segment> = lower.iter().map(leg_as_segment).collect();
    let version = C2VersionTuple::auto_pairing();
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).is_ok();
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L1 run 投影失败: {error:?}"))?;
                let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = decompose::decompose(&centers);
                let query = LevelViewQuery {
                    level: 1,
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
                        lower_legs: lower,
                        hist,
                        dif,
                        close_src,
                    },
                )
                .map_err(|error| format!("L1 C2 assemble 失败: {error:?}"))?;
                for pair in &view.pairs {
                    let (Some(last_seed), Some(prev_seed)) = (
                        projection.seeds.get(pair.id.block_end_center),
                        pair.id
                            .block_end_center
                            .checked_sub(1)
                            .and_then(|p| projection.seeds.get(p)),
                    ) else {
                        continue;
                    };
                    let Some(c_terminal) = segments.iter().find(|segment| {
                        segment.direction == pair.id.direction
                            && segment.start_index >= last_seed.center.end_index
                            && segment.end_index <= as_of
                    }) else {
                        continue;
                    };
                    let ctx = PairCtx {
                        direction: pair.id.direction,
                        prev: prev_seed.center,
                        last: last_seed.center,
                        seg_a: pair.seg_a,
                        c_start: pair.seg_c.0,
                    };
                    if out.insert(c_terminal.end_index, ctx).is_some() {
                        collisions += 1;
                    }
                }
                run_start = None;
            }
            _ => {}
        }
    }
    Ok((out, collisions))
}

// ══════════════ 复刻件（pub(crate)/private 不可直达；逐字 + 自校验）══════════════

/// divergence.rs:864-876 `move_range_envelope` 逐字复刻（pub(crate) 不可达）。
fn move_range_envelope_replica(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
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

/// signal.rs:497-521 `trend_third_class_in_c` 逐字复刻（pub(crate) 不可达）。
fn trend_third_class_in_c_replica(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    c_start: usize,
    as_of: usize,
) -> Option<(usize, usize)> {
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let hi = segments.partition_point(|s| s.end_index <= as_of);
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

/// R1 全合取确认的诊断版（level_view.rs:535-626 `trend_confirm_time` 逐字复刻 + 失败原因记录）。
/// 自校验：30 个 confirmed 事件复刻 t* 必须 == event.turn_source。
#[derive(Debug, Clone, Copy)]
struct R1Diag {
    t4_ok: bool,
    t3: Option<(usize, usize)>,
    t_star: Option<usize>,
    t5_fail_at: Option<usize>,
    extreme_at_t5_fail: bool,
    env_a: Option<(Tick, Tick)>,
    env_c_final: Option<(Tick, Tick)>,
    reason: &'static str,
}

#[allow(clippy::too_many_arguments)]
fn trend_confirm_time_diag(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    side: Side,
    seg_a: (usize, usize),
    c_start: usize,
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> R1Diag {
    // T4（025:761）：B 中枢 span 内 DIF 变号或触 0。
    let Some((b_lo, b_hi)) = map_src_to_close_idx(close_src, last.start_index, last.end_index)
    else {
        return R1Diag {
            t4_ok: false,
            t3: None,
            t_star: None,
            t5_fail_at: None,
            extreme_at_t5_fail: false,
            env_a: None,
            env_c_final: None,
            reason: "map_fail_b_span",
        };
    };
    if !dif_crosses_zero(dif, b_lo, b_hi) {
        return R1Diag {
            t4_ok: false,
            t3: None,
            t_star: None,
            t5_fail_at: None,
            extreme_at_t5_fail: false,
            env_a: None,
            env_c_final: None,
            reason: "T4_dif_no_cross",
        };
    }
    let Some(a_idx) = map_src_to_close_idx(close_src, seg_a.0, seg_a.1) else {
        return R1Diag {
            t4_ok: true,
            t3: None,
            t_star: None,
            t5_fail_at: None,
            extreme_at_t5_fail: false,
            env_a: None,
            env_c_final: None,
            reason: "map_fail_seg_a",
        };
    };
    let Some(a_env) = move_range_envelope_replica(segments, seg_a) else {
        return R1Diag {
            t4_ok: true,
            t3: None,
            t_star: None,
            t5_fail_at: None,
            extreme_at_t5_fail: false,
            env_a: None,
            env_c_final: None,
            reason: "env_a_none",
        };
    };
    let area_a = same_color_area(hist, a_idx.0, a_idx.1, side);
    let dif_peak_a = segment_dif_peak(dif, a_idx.0, a_idx.1, direction);
    let hist_peak_a = same_dir_hist_peak(hist, a_idx.0, a_idx.1, side);
    let Some((_leave_end, t3)) =
        trend_third_class_in_c_replica(segments, last, direction, c_start, as_of)
    else {
        return R1Diag {
            t4_ok: true,
            t3: None,
            t_star: None,
            t5_fail_at: None,
            extreme_at_t5_fail: false,
            env_a: Some(a_env),
            env_c_final: None,
            reason: "T3_no_third_class",
        };
    };
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let hi = segments.partition_point(|s| s.end_index <= as_of);
    let mut env: Option<(Tick, Tick)> = None;
    let mut acc_hi: Option<usize> = None;
    let mut area_c = 0.0f64;
    let (mut dif_max, mut dif_min) = (f64::NEG_INFINITY, f64::INFINITY);
    let mut hist_max = f64::NEG_INFINITY;
    let mut hist_min = f64::INFINITY;
    for leg in &segments[lo..hi] {
        let leg_lo = leg.start_price.min(leg.end_price);
        let leg_hi = leg.start_price.max(leg.end_price);
        env = Some(match env {
            None => (leg_lo, leg_hi),
            Some((old_lo, old_hi)) => (old_lo.min(leg_lo), old_hi.max(leg_hi)),
        });
        if let Some((c_lo, cur_hi)) = map_src_to_close_idx(close_src, c_start, leg.end_index) {
            let from = acc_hi.map_or(c_lo, |prev| prev + 1);
            if from <= cur_hi {
                for t in from..=cur_hi {
                    match side {
                        Side::Long if hist[t] < 0.0 => area_c += hist[t].abs(),
                        Side::Short if hist[t] > 0.0 => area_c += hist[t],
                        _ => {}
                    }
                    hist_max = hist_max.max(hist[t]);
                    hist_min = hist_min.min(hist[t]);
                    dif_max = dif_max.max(dif[t]);
                    dif_min = dif_min.min(dif[t]);
                }
                acc_hi = Some(cur_hi);
            }
        }
        if leg.end_index < t3 {
            continue;
        }
        let Some(_acc) = acc_hi else {
            return R1Diag {
                t4_ok: true,
                t3: Some((_leave_end, t3)),
                t_star: None,
                t5_fail_at: None,
                extreme_at_t5_fail: false,
                env_a: Some(a_env),
                env_c_final: env,
                reason: "map_fail_force_window",
            };
        };
        let dif_peak_c = match direction {
            Direction::Up => dif_max.max(0.0),
            Direction::Down => dif_min.min(0.0).abs(),
        };
        let hist_peak_c = match side {
            Side::Long => hist_min.min(0.0).abs(),
            Side::Short => hist_max.max(0.0),
        };
        let force_ok = area_c < area_a || dif_peak_c < dif_peak_a || hist_peak_c < hist_peak_a;
        if !force_ok {
            let (env_lo, env_hi) = env.expect("扫描窗口非空");
            let extreme = match side {
                Side::Long => env_lo < a_env.0,
                Side::Short => env_hi > a_env.1,
            };
            return R1Diag {
                t4_ok: true,
                t3: Some((_leave_end, t3)),
                t_star: None,
                t5_fail_at: Some(leg.end_index),
                extreme_at_t5_fail: extreme,
                env_a: Some(a_env),
                env_c_final: env,
                reason: "T5_turned_false",
            };
        }
        let (env_lo, env_hi) = env.expect("扫描窗口非空（t3 已命中）");
        let extreme = match side {
            Side::Long => env_lo < a_env.0,
            Side::Short => env_hi > a_env.1,
        };
        if extreme {
            return R1Diag {
                t4_ok: true,
                t3: Some((_leave_end, t3)),
                t_star: Some(leg.end_index),
                t5_fail_at: None,
                extreme_at_t5_fail: true,
                env_a: Some(a_env),
                env_c_final: env,
                reason: "confirmed",
            };
        }
    }
    R1Diag {
        t4_ok: true,
        t3: Some((_leave_end, t3)),
        t_star: None,
        t5_fail_at: None,
        extreme_at_t5_fail: false,
        env_a: Some(a_env),
        env_c_final: env,
        reason: "T2_never_extreme",
    }
}

/// judge_first_cached L0 判据链诊断复刻（signal.rs:285-398；anchor=自锚 L0 语义）。
/// 自校验：hit 点（confirm 置位）复刻须 = Confirm；零 bit struct_break 点复刻须 = AreaFail。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JudgeOut {
    Confirm,
    AreaFail,
    GateClosed,
    NoBroke,
    ANone,
    NoExtreme03720,
    LambdaNone,
    MapFail,
    NoOwner,
}

impl JudgeOut {
    fn name(self) -> &'static str {
        match self {
            JudgeOut::Confirm => "Confirm",
            JudgeOut::AreaFail => "AreaFail(zero-bit)",
            JudgeOut::GateClosed => "GateClosed",
            JudgeOut::NoBroke => "NoBroke",
            JudgeOut::ANone => "ANone",
            JudgeOut::NoExtreme03720 => "NoExtreme03720",
            JudgeOut::LambdaNone => "LambdaNone",
            JudgeOut::MapFail => "MapFail",
            JudgeOut::NoOwner => "NoOwner",
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn judge_replica(
    leg: &Segment,
    owner_idx: usize,
    centers0: &[Center],
    gate0: &[Option<Direction>],
    first_match: &std::collections::HashMap<(usize, Tick, Tick), usize>,
    segs: &[Segment],
    anchors: &[Option<Direction>],
    hist: &[f64],
    close_src: &[usize],
    dir: Direction,
) -> (JudgeOut, Option<(usize, usize)>) {
    let owner = &centers0[owner_idx];
    // 生产 first_match 语义：pos = 三元组首匹配下标（signal.rs:1049-1056,1083-1089）。
    let pos = first_match
        .get(&(owner.end_index, owner.zd, owner.zg))
        .copied()
        .unwrap_or(owner_idx);
    let gate_dir = gate0.get(pos).copied().flatten();
    if gate_dir != Some(dir) {
        return (JudgeOut::GateClosed, None);
    }
    let broke = match dir {
        Direction::Down => leg.end_price < owner.zd,
        Direction::Up => owner.zg < leg.end_price,
    };
    if !broke {
        return (JudgeOut::NoBroke, None);
    }
    let Some(prev_center) = pos.checked_sub(1).and_then(|p| centers0.get(p)) else {
        return (JudgeOut::ANone, None);
    };
    let a_span = locate_departure_move_a(segs, anchors, prev_center, owner, dir);
    let Some(a) = a_span else {
        return (JudgeOut::ANone, None);
    };
    let extreme_037_20 = match move_range_envelope_replica(segs, a) {
        Some((b_lo, b_hi)) => match dir {
            Direction::Down => leg.end_price < b_lo,
            Direction::Up => leg.end_price > b_hi,
        },
        None => false,
    };
    if !extreme_037_20 {
        return (JudgeOut::NoExtreme03720, a_span);
    }
    let Some(lambda_c) = departure_move_c_start(segs, anchors, owner, dir, leg.start_index) else {
        return (JudgeOut::LambdaNone, a_span);
    };
    let (Some(c_idx), Some(a_idx)) = (
        map_src_to_close_idx(close_src, lambda_c, leg.end_index),
        map_src_to_close_idx(close_src, a.0, a.1),
    ) else {
        return (JudgeOut::MapFail, a_span);
    };
    let abc = AbcDivergence {
        seg_a: a,
        seg_c: (lambda_c, leg.end_index),
        is_trend: true,
    };
    if abc.diverges(hist, a_idx, c_idx) {
        (JudgeOut::Confirm, a_span)
    } else {
        (JudgeOut::AreaFail, a_span)
    }
}

/// #218 面 B 研究 bin 锚供给说明（诚实，090）：owner 判同已换两族锚（一/三类核心区间
/// 带判同经账本 `centers` 全功能；二类一类点身份锚判同需 T1 oracle + 事件锚账本）。
/// 本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解 = 诚实判负（与
/// 生产 gate 全接线读数有别，面 B 注册项；一/三类判同不受影响）。
fn bin_anchor_ctx() -> newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx<'static> {
    fn never(_: usize) -> Option<(newchan_rust::theta_v0::types::Tick, usize)> {
        None
    }
    newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx {
        anchor_at: &never,
        event_anchor: (None, None),
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p117_h4_attribution <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("ATTR_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    let as_of = max_bars - 1;
    println!(
        "ATTR_INPUT bars={} replay_bars={} as_of={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        as_of,
        loaded.first_date,
        loaded.last_date
    );

    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let mut audit = ProviderAudit::default();
    let events_by_level =
        collect_snapshot_candidates(&terminal.tower, as_of, hist, dif, close_src, &mut audit)?;
    println!(
        "ATTR_PROVIDER snapshots={} views={} too_short={} invalid_seed={} missing_carried_center={} other={}",
        audit.snapshots,
        audit.views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other
    );

    let classification = &terminal.classification;
    if terminal.tower.len() < 2 || classification.levels.len() < 2 {
        return Err("塔/分类级别不足（须 ≥2）".to_string());
    }
    let legs0 =
        lower_legs_from(&terminal.tower[0]).map_err(|error| format!("L0 legs 失败: {error:?}"))?;
    let segs_l0: Vec<Segment> = legs0.iter().map(leg_as_segment).collect();
    let anchors0: Vec<Option<Direction>> = segs_l0.iter().map(|s| Some(s.direction)).collect();
    let centers0 = &classification.levels[0].centers;
    let blocks0 = decompose::decompose(centers0);
    let gate0 = center_trend_gate(centers0.len(), &blocks0);
    // 生产 first_match_idx 同语义（signal.rs:1049-1056）。
    let mut first_match: std::collections::HashMap<(usize, Tick, Tick), usize> =
        std::collections::HashMap::new();
    for (idx, c) in centers0.iter().enumerate() {
        first_match.entry((c.end_index, c.zd, c.zg)).or_insert(idx);
    }
    // leg 终点 → leg（owner 回读用；重复终点保留首个，诊断足够）。
    let mut leg_by_end: BTreeMap<usize, &Segment> = BTreeMap::new();
    for seg in &segs_l0 {
        leg_by_end.entry(seg.end_index).or_insert(seg);
    }
    let (pair_map, pair_collisions) =
        collect_l1_pair_contexts(&terminal.tower, &legs0, as_of, hist, dif, close_src)?;
    println!(
        "ATTR_PAIR_AUDIT pairs={} turn_collisions={}",
        pair_map.len(),
        pair_collisions
    );
    let l1_events: &[NestCandidateEvent] = events_by_level.get(1).map_or(&[], Vec::as_slice);
    let bsp0 = &classification.levels[0].bsp;

    // ── 自校验计数 ──
    let mut tstar_match = 0usize;
    let mut tstar_total = 0usize;
    let mut judge_hit_ok = 0usize;
    let mut judge_hit_total = 0usize;
    let mut judge_zero_ok = 0usize;
    let mut judge_zero_total = 0usize;
    let mut seal_match = 0usize;
    let mut seal_total = 0usize;
    let mut fork_lines = 0usize;

    // ── 汇总计数 ──
    let mut q4_extended = 0usize;
    let mut q4_exact = 0usize;
    let mut q4_recut = 0usize;
    let mut q2_prev_frontier = 0usize;
    let mut q2_b_genuine = 0usize;
    let mut q2_other = 0usize;
    // Q1 分类：R=破B confirm 在 t* 右；Z=破B 仅零 bit 候选；N=破B 无任何候选；B=窗内 owner=B 命中；P=窗内 owner=prev 命中
    let mut q1_cat_r = 0usize;
    let mut q1_cat_z = 0usize;
    let mut q1_cat_n = 0usize;
    let mut q1_cat_b = 0usize;
    let mut q1_cat_p = 0usize;

    for (case_idx, case) in CASES.iter().enumerate() {
        let case_no = case_idx + 1;
        let pair = pair_map.get(&case.turn).copied();
        let dir = pair.map(|ctx| ctx.direction);
        // 事件配对（与 p117 探针同表达式）。
        let matches: Vec<&NestCandidateEvent> = l1_events
            .iter()
            .filter(|event| {
                event.kind == NestDivergenceKind::Trend
                    && event.side == case.side
                    && event.seg_a == case.seg_a
                    && event.interval_b.0 == case.seg_c.0
                    && event.divergence_confirmed
            })
            .collect();
        let event = matches.first().copied().copied();
        let unconfirmed = l1_events.iter().any(|e| {
            e.kind == NestDivergenceKind::Trend
                && e.side == case.side
                && e.seg_a == case.seg_a
                && e.interval_b.0 == case.seg_c.0
                && !e.divergence_confirmed
        });
        let status = if event.is_some() {
            "confirmed"
        } else if unconfirmed {
            "unconfirmed"
        } else {
            "absent"
        };
        let t_star = event.map(|e| e.turn_source);
        let ref_right = t_star.unwrap_or(case.turn);
        println!(
            "ATTR_CASE case={} side={:?} dir={} turn={} c_start={} tstar={} status={}",
            case_no,
            case.side,
            dir.map_or("NA".to_string(), |d| format!("{d:?}")),
            case.turn,
            case.seg_c.0,
            t_star.map_or("NA".to_string(), |t| t.to_string()),
            status
        );
        // 生产 hit（单一来源）。
        if let Some(e) = event {
            let hit = terminal_bits_at_event(
                classification,
                &e,
                TerminalMatch::CWindow,
                &bin_anchor_ctx(),
            )
            .is_some();
            let book_level = event_bsp_book_level(e.level);
            println!(
                "ATTR_PROD_HIT case={} book_level={:?} hit={}",
                case_no, book_level, hit
            );
        }
        let Some(ctx) = pair else {
            println!("ATTR_FORK case={} pair_missing", case_no);
            fork_lines += 1;
            continue;
        };
        // ── ATTR_B：seed B vs detect B（by start_index）──
        let b_detect = centers0
            .iter()
            .find(|c| c.start_index == ctx.last.start_index);
        let (b_idx, b_verdict) = match b_detect {
            Some(d) => {
                let idx = centers0
                    .iter()
                    .position(|c| c.start_index == ctx.last.start_index)
                    .expect("found above");
                let zg_eq = d.zg == ctx.last.zg;
                let zd_eq = d.zd == ctx.last.zd;
                let exact = *d == ctx.last;
                let verdict = if exact {
                    q4_exact += 1;
                    "EXACT"
                } else if zg_eq && zd_eq {
                    q4_extended += 1;
                    "EXTENDED"
                } else {
                    q4_recut += 1;
                    "RECUT_CORE_DIFF"
                };
                println!(
                    "ATTR_B case={} seed=({},{},zd={},zg={},dd={},gg={}) detect=({},{},zd={},zg={},dd={},gg={}) zg_eq={} zd_eq={} verdict={}",
                    case_no,
                    ctx.last.start_index, ctx.last.end_index, ctx.last.zd, ctx.last.zg, ctx.last.dd, ctx.last.gg,
                    d.start_index, d.end_index, d.zd, d.zg, d.dd, d.gg,
                    zg_eq, zd_eq, verdict
                );
                (Some(idx), verdict)
            }
            None => {
                q4_recut += 1;
                println!(
                    "ATTR_B case={} seed=({},{},zd={},zg={}) detect=MISSING_BY_START verdict=RECUT_OR_MISSING",
                    case_no, ctx.last.start_index, ctx.last.end_index, ctx.last.zd, ctx.last.zg
                );
                (None, "MISSING_BY_START")
            }
        };
        // ── ATTR_GATES：b/prev 门值与块位置 ──
        let prev_detect_idx = centers0
            .iter()
            .position(|c| c.start_index == ctx.prev.start_index);
        if let Some(bi) = b_idx {
            let block_b = blocks0
                .iter()
                .find(|b| {
                    b.kind == newchan_rust::theta_v0::types::MoveKind::Trend
                        && b.dir == Some(ctx.direction)
                        && b.start_center <= bi
                        && bi <= b.end_center
                })
                .copied();
            let prev_is_block_start = match (block_b, prev_detect_idx) {
                (Some(bl), Some(pi)) => bl.start_center == pi,
                _ => false,
            };
            println!(
                "ATTR_GATES case={} b_idx={} gate_b={} prev_idx={:?} gate_prev={} block={:?} prev_is_block_start={}",
                case_no,
                bi,
                dir_name(gate0.get(bi).copied().flatten()),
                prev_detect_idx,
                prev_detect_idx.map_or("NA".to_string(), |pi| dir_name(gate0.get(pi).copied().flatten())),
                block_b.map(|b| (b.start_center, b.end_center)),
                prev_is_block_start
            );
        }
        // ── ATTR_SEAL：延伸吸收复刻 vs detect 实测端 ──
        if let Some(d) = b_detect {
            let start_pos = segs_l0.partition_point(|s| s.end_index <= ctx.last.end_index);
            let mut seal_end = ctx.last.end_index;
            for s in &segs_l0[start_pos..] {
                let lo = s.start_price.min(s.end_price);
                let hi = s.start_price.max(s.end_price);
                if lo <= d.zg && hi >= d.zd {
                    seal_end = s.end_index;
                } else {
                    break;
                }
            }
            seal_total += 1;
            let matched = seal_end == d.end_index;
            seal_match += usize::from(matched);
            if !matched {
                println!(
                    "ATTR_FORK case={} seal_mismatch computed={} detect={}",
                    case_no, seal_end, d.end_index
                );
                fork_lines += 1;
            }
            println!(
                "ATTR_SEAL case={} computed={} detect={} match={} absorbed_past_turn={} absorbed_past_cstart={}",
                case_no, seal_end, d.end_index, matched,
                d.end_index >= case.turn, d.end_index > case.seg_c.0
            );
        }
        // ── ATTR_TURN：c 破核腿（#105 turn 端）判决复刻 + 书点实况 ──
        let owner_of = |seg_start: usize| -> Option<(usize, String)> {
            nearest_center_idx(centers0, seg_start).map(|oi| {
                let tag = if centers0[oi].start_index == ctx.last.start_index {
                    "B".to_string()
                } else if centers0[oi].start_index == ctx.prev.start_index {
                    "PREV".to_string()
                } else {
                    format!(
                        "OTHER({},{})",
                        centers0[oi].start_index, centers0[oi].end_index
                    )
                };
                (oi, tag)
            })
        };
        if let Some(leg) = segs_l0.iter().find(|s| s.end_index == case.turn) {
            let (jout, a_span) = match nearest_center_idx(centers0, leg.start_index) {
                Some(oi) => judge_replica(
                    leg,
                    oi,
                    centers0,
                    &gate0,
                    &first_match,
                    &segs_l0,
                    &anchors0,
                    hist,
                    close_src,
                    ctx.direction,
                ),
                None => (JudgeOut::NoOwner, None),
            };
            let owner_tag = owner_of(leg.start_index)
                .map(|(_, t)| t)
                .unwrap_or_else(|| "NONE".to_string());
            let point = bsp0.iter().find(|p| p.source_index == case.turn);
            let point_str = match point {
                Some(p) => format!(
                    "present(bits={},sbd={:?},conf={})",
                    bits_str(p),
                    p.struct_break_dir,
                    p.bits.confirm_side(case.side)
                ),
                None => "absent".to_string(),
            };
            println!(
                "ATTR_TURN case={} leg=({},{}) owner={} replica={} a_span={:?} point={}",
                case_no,
                leg.start_index,
                leg.end_index,
                owner_tag,
                jout.name(),
                a_span,
                point_str
            );
        } else {
            println!("ATTR_TURN case={} leg_at_turn=MISSING", case_no);
        }
        // ── ATTR_PT：窗口邻域书点逐点 dump ──
        let dump_lo = case.seg_c.0.saturating_sub(1500);
        let dump_hi = ref_right + 3000;
        for p in bsp0
            .iter()
            .filter(|p| p.source_index >= dump_lo && p.source_index <= dump_hi)
        {
            let (owner_str, jout_str) = match leg_by_end.get(&p.source_index) {
                Some(leg) => {
                    let o = owner_of(leg.start_index)
                        .map(|(_, t)| t)
                        .unwrap_or_else(|| "NONE".to_string());
                    let jo = if !p.bits.confirm_side(case.side) && p.struct_break_dir.is_some() {
                        // 零 bit struct_break 候选：复刻须 = AreaFail（自校验）。
                        judge_zero_total += 1;
                        let (jout, _) = match nearest_center_idx(centers0, leg.start_index) {
                            Some(oi) => judge_replica(
                                leg,
                                oi,
                                centers0,
                                &gate0,
                                &first_match,
                                &segs_l0,
                                &anchors0,
                                hist,
                                close_src,
                                ctx.direction,
                            ),
                            None => (JudgeOut::NoOwner, None),
                        };
                        if jout == JudgeOut::AreaFail {
                            judge_zero_ok += 1;
                        } else {
                            println!(
                                "ATTR_FORK case={} zerobit_replica={} idx={}",
                                case_no,
                                jout.name(),
                                p.source_index
                            );
                            fork_lines += 1;
                        }
                        jout.name().to_string()
                    } else {
                        "-".to_string()
                    };
                    (o, jo)
                }
                None => ("NO_LEG".to_string(), "-".to_string()),
            };
            println!(
                "ATTR_PT case={} idx={} bits={} sbd={:?} conf={} owner={} zerobit_replica={}",
                case_no,
                p.source_index,
                bits_str(p),
                p.struct_break_dir,
                p.bits.confirm_side(case.side),
                owner_str,
                jout_str
            );
        }
        // ── ATTR_LEFT / ATTR_RIGHT：全书窗口两侧最近 confirm_side 点 ──
        let left = bsp0
            .iter()
            .filter(|p| p.source_index < case.seg_c.0 && p.bits.confirm_side(case.side))
            .max_by_key(|p| p.source_index);
        match left {
            Some(p) => {
                let tag = leg_by_end
                    .get(&p.source_index)
                    .and_then(|leg| owner_of(leg.start_index).map(|(_, t)| t))
                    .unwrap_or_else(|| "NO_LEG".to_string());
                println!(
                    "ATTR_LEFT case={} idx={} dist={} bits={} owner={}",
                    case_no,
                    p.source_index,
                    case.seg_c.0 - p.source_index,
                    bits_str(p),
                    tag
                );
            }
            None => println!("ATTR_LEFT case={} none", case_no),
        }
        let right = bsp0
            .iter()
            .filter(|p| p.source_index > ref_right && p.bits.confirm_side(case.side))
            .min_by_key(|p| p.source_index);
        match right {
            Some(p) => {
                let tag = leg_by_end
                    .get(&p.source_index)
                    .and_then(|leg| owner_of(leg.start_index).map(|(_, t)| t))
                    .unwrap_or_else(|| "NO_LEG".to_string());
                println!(
                    "ATTR_RIGHT case={} idx={} dist={} bits={} owner={} ref={}",
                    case_no,
                    p.source_index,
                    p.source_index - ref_right,
                    bits_str(p),
                    tag,
                    ref_right
                );
            }
            None => println!("ATTR_RIGHT case={} none ref={}", case_no, ref_right),
        }
        // ── ATTR_BPOINT：c_start 起 owner=B 的首个 confirm 点 / 首个候选点（全书）──
        let owner_is_b = |p: &BspPoint| -> bool {
            leg_by_end
                .get(&p.source_index)
                .and_then(|leg| nearest_center_idx(centers0, leg.start_index))
                .is_some_and(|oi| centers0[oi].start_index == ctx.last.start_index)
        };
        let b_confirm = bsp0
            .iter()
            .filter(|p| {
                p.source_index >= case.seg_c.0 && p.bits.confirm_side(case.side) && owner_is_b(p)
            })
            .min_by_key(|p| p.source_index);
        let b_candidate = bsp0
            .iter()
            .filter(|p| {
                p.source_index >= case.seg_c.0
                    && p.struct_break_dir == Some(case.side)
                    && owner_is_b(p)
            })
            .min_by_key(|p| p.source_index);
        let rel = |idx: usize| -> String {
            match t_star {
                Some(t) if idx <= t => "inside".to_string(),
                Some(t) => format!("right+{}", idx - t),
                None => format!("turn+{}", idx.saturating_sub(case.turn)),
            }
        };
        println!(
            "ATTR_BPOINT case={} confirm={} candidate={}",
            case_no,
            b_confirm.map_or("none".to_string(), |p| format!(
                "(idx={},bits={},rel={})",
                p.source_index,
                bits_str(p),
                rel(p.source_index)
            )),
            b_candidate.map_or("none".to_string(), |p| format!(
                "(idx={},bits={},rel={})",
                p.source_index,
                bits_str(p),
                rel(p.source_index)
            ))
        );
        // Q1 分类（仅 event present 案参与 rescued 相关统计；对全部案输出分类供诊断）。
        match t_star {
            Some(_) => {
                // 窗内命中情形
                let in_window_confirm_prev = bsp0.iter().any(|p| {
                    p.source_index >= case.seg_c.0
                        && Some(p.source_index) <= t_star
                        && p.bits.confirm_side(case.side)
                        && !owner_is_b(p)
                });
                let in_window_confirm_b = bsp0.iter().any(|p| {
                    p.source_index >= case.seg_c.0
                        && Some(p.source_index) <= t_star
                        && p.bits.confirm_side(case.side)
                        && owner_is_b(p)
                });
                if in_window_confirm_b {
                    q1_cat_b += 1;
                } else if in_window_confirm_prev {
                    q1_cat_p += 1;
                } else if b_confirm.is_some() {
                    q1_cat_r += 1;
                } else if b_candidate.is_some() {
                    q1_cat_z += 1;
                } else {
                    q1_cat_n += 1;
                }
            }
            None => {}
        }
        // ── ATTR_HIT：hit 点身份复刻（生产单一来源 hit + 回读扫描坐标）──
        if let Some(e) = event {
            let book = &classification.levels[event_bsp_book_level(e.level).expect("level≥1")].bsp;
            let hit_idx = book
                .iter()
                .filter(|p| {
                    e.interval_b.0 <= p.source_index
                        && p.source_index <= e.turn_source
                        && p.bits.confirm_side(e.side)
                })
                .min_by_key(|p| p.source_index)
                .map(|p| p.source_index);
            if let Some(idx) = hit_idx {
                judge_hit_total += 1;
                let (owner_str, jout) = match leg_by_end.get(&idx) {
                    Some(leg) => {
                        let o = owner_of(leg.start_index)
                            .map(|(_, t)| t)
                            .unwrap_or_else(|| "NONE".to_string());
                        let (jout, _) = match nearest_center_idx(centers0, leg.start_index) {
                            Some(oi) => judge_replica(
                                leg,
                                oi,
                                centers0,
                                &gate0,
                                &first_match,
                                &segs_l0,
                                &anchors0,
                                hist,
                                close_src,
                                ctx.direction,
                            ),
                            None => (JudgeOut::NoOwner, None),
                        };
                        if jout == JudgeOut::Confirm {
                            judge_hit_ok += 1;
                        } else {
                            println!(
                                "ATTR_FORK case={} hit_replica={} idx={}",
                                case_no,
                                jout.name(),
                                idx
                            );
                            fork_lines += 1;
                        }
                        (o, jout)
                    }
                    None => ("NO_LEG".to_string(), JudgeOut::NoOwner),
                };
                let p = bsp0
                    .iter()
                    .find(|p| p.source_index == idx)
                    .expect("hit in book");
                println!(
                    "ATTR_HIT case={} idx={} bits={} owner={} replica={} seed_b=({},{}) seed_prev=({},{})",
                    case_no,
                    idx,
                    bits_str(p),
                    owner_str,
                    jout.name(),
                    ctx.last.start_index,
                    ctx.last.end_index,
                    ctx.prev.start_index,
                    ctx.prev.end_index
                );
                match owner_str.as_str() {
                    "B" => q2_b_genuine += 1,
                    "PREV" => q2_prev_frontier += 1,
                    _ => q2_other += 1,
                }
            }
            // ── R1 复刻自校验（confirmed 事件）──
            tstar_total += 1;
            let diag = trend_confirm_time_diag(
                &segs_l0,
                &ctx.last,
                ctx.direction,
                e.side,
                ctx.seg_a,
                ctx.c_start,
                as_of,
                hist,
                dif,
                close_src,
            );
            if diag.t_star == Some(e.turn_source) {
                tstar_match += 1;
            } else {
                println!(
                    "ATTR_FORK case={} tstar_replica={:?} event={}",
                    case_no, diag.t_star, e.turn_source
                );
                fork_lines += 1;
            }
        } else if unconfirmed {
            // ── ATTR_R1：7 事件缺失案 R1 合取分解 ──
            let diag = trend_confirm_time_diag(
                &segs_l0,
                &ctx.last,
                ctx.direction,
                case.side,
                ctx.seg_a,
                ctx.c_start,
                as_of,
                hist,
                dif,
                close_src,
            );
            if diag.t_star.is_some() {
                println!(
                    "ATTR_FORK case={} unconfirmed_but_replica_confirmed t={:?}",
                    case_no, diag.t_star
                );
                fork_lines += 1;
            }
            println!(
                "ATTR_R1 case={} t4={} t3={:?} t5_fail_at={:?} extreme_at_t5_fail={} env_a={:?} env_c_final={:?} reason={}",
                case_no,
                diag.t4_ok,
                diag.t3,
                diag.t5_fail_at,
                diag.extreme_at_t5_fail,
                diag.env_a,
                diag.env_c_final,
                diag.reason
            );
        }
    }

    // ── 汇总 ──
    println!(
        "ATTR_REPLICA_TSTAR match={}/{}（30 confirmed 事件复刻 t* 对拍）",
        tstar_match, tstar_total
    );
    println!(
        "ATTR_REPLICA_JUDGE hit_confirm={}/{} zerobit_areafail={}/{}",
        judge_hit_ok, judge_hit_total, judge_zero_ok, judge_zero_total
    );
    println!("ATTR_SEAL_CHECK match={}/{}", seal_match, seal_total);
    println!(
        "ATTR_Q4_SUMMARY extended={} exact={} recut_or_missing={}（b_center_missing 性质）",
        q4_extended, q4_exact, q4_recut
    );
    println!(
        "ATTR_Q2_SUMMARY hit_prev_frontier={} hit_b_genuine={} hit_other={}（6 命中案身份）",
        q2_prev_frontier, q2_b_genuine, q2_other
    );
    println!(
        "ATTR_Q1_SUMMARY window_hit_b={} window_hit_prev={} bpoint_right_of_tstar={} bpoint_zerobit_only={} bpoint_none={}（30 event-present 案）",
        q1_cat_b, q1_cat_p, q1_cat_r, q1_cat_z, q1_cat_n
    );
    println!("ATTR_FORK_LINES count={}", fork_lines);
    Ok(())
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

/// 数据加载（逐字复制 p117/p116 load_bars / BarsJson / date_to_timestamp）。
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
    Ok(LoadedBars {
        bars,
        first_date: raw.dates.first().cloned().unwrap_or_default(),
        last_date: raw.dates.last().cloned().unwrap_or_default(),
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
