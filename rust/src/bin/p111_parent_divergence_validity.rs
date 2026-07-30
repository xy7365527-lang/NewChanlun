//! task #111 父级背驰有效性判别探针（只读归因探针，不进证书门，不为增产放宽任何判据）。
//!
//! 依据 `chanlun/review-results/doc-divergence-validity-test-20260717.md` §2/§5 的修正判别式：
//!   - 阈值按父级类型分裂：Trend 父级 → 最后中枢（B）DD（029:30）；Pan 父级 → 中枢（A）ZD（020:60）。
//!     Short 侧镜像：Trend → GG、Pan → ZG。
//!   - 核心判据是**时序**：自 parent.end 起「先触阈值、后破极值」⟹ 定理后果兑现 ⟹ (b) 拒发正确；
//!     「先破极值、未触阈值」⟹ 父级背驰不成立 ⟹ (a*)（应重锚到仍在延伸的真背驰段）。
//!     日历窗口 (parent.end, child.end] 只是时序判据的近似（本探针以全数据扫描为准，
//!     同时输出窗口内事实供对照）。
//!   - (a*) 内部按 §5.2 细分：父事件 divergence_confirmed=false ⟹ (a3)（父级层面本无背驰力度结构）；
//!     否则在 child.end 处重测父级 c（延伸至 child.end）对 seg_a（教义 b 段）的 MACD 面积背驰
//!     （061:26 口径）：仍背驰 ⟹ (a1)（背驰段仍在延伸，链应以修正父区间重发）；转强 ⟹ (a2)。
//!
//! 总体 = 全量重放**全部**「被拒链边」的父级事件（不止 p102 四条边）：
//!   ① 6,482 条结构链包含门（is_sub@B）击杀边的同侧父候选池（p109 管线逐字复刻，
//!      锚：inclusion=30 / direction=11 / 事件总数 3,683 / 证书 66 双向 diff=0）；
//!   ② A 口径证书全部相邻 rung 边中 is_sub@B 判负边（含 p102 五条边的硬锚：
//!      父级 turn 704358 / 1574631 / 1832757 / 3750585 / 689893）。
//!   对照组：B 口径唯一多级证书（dump:806，基例 147126）的父级——活体证书的父级不应被判 (a*)。
//!
//! 工件缺口（doc §5.3）：NestCandidateEvent 无中枢字段（level_view.rs:475-490），
//!   ZD/DD 由探针在同一 run 内按生产同一路径几何复原（Trend→seeds[block_end_center]，
//!   Pan→nearest_confirmed_center_idx 等价复刻 signal.rs:208），不进任何生产真值。
//!
//! 用法：
//! `cargo run --release --bin p111_parent_divergence_validity -- <btc_1m_full.json> [p92_ckpt_dump.txt]`
//! 冒烟：`P111_MAX_BARS=250000 ...`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::{compute_macd, segment_macd_area};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CompletionEvidence,
    CompletionStatus, CoordinateWindow, LevelViewMaterial, LevelViewQuery, LowerLeg,
    NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_typed_certificates, is_sub, NestInterval, NestIntervalCaliber, TypedNestCertificate,
};
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

// ───────────────────────── 基础数据结构 ─────────────────────────

/// 事件键（level, is_short, is_pan, seg_a, interval_b, turn_source）——可排序去重。
type EventKey = (u32, bool, bool, (usize, usize), (usize, usize), usize);

fn event_key(e: &NestCandidateEvent) -> EventKey {
    (
        e.level,
        matches!(e.side, Side::Short),
        matches!(e.kind, NestDivergenceKind::Consolidation),
        e.seg_a,
        e.interval_b,
        e.turn_source,
    )
}

/// 严格 C2 pair（同 run 两相邻 Completed Move，[leave.start, retest.end]）——p109 逐字口径。
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // leave_dir/leave_via_divergence 留作归因排查字段
struct PairRec {
    start: usize,
    end: usize,
    leave_kind: MoveKind,
    leave_dir: Option<Direction>,
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
    runs: usize,
    pairs: Vec<PairRec>,
    events: Vec<NestCandidateEvent>,
}

/// 包含门击杀边的完整记录：一条链在 kill_level 处，frontier 中全部子事件 × 同侧父池全部判负。
#[derive(Debug, Clone)]
struct InclusionRow {
    chain_id: usize,
    kill_level: usize,
    base_turn: usize,
    child: EventKey,
    parents: Vec<EventKey>,
}

/// A 证书宇宙中 is_sub@B 判负的 rung 边。
#[derive(Debug, Clone)]
struct CertEdgeRow {
    exec: usize,
    top: usize,
    side: Side,
    base_turn: usize,
    child: EventKey,
    parent: EventKey,
    p102_anchor: bool,
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

fn typed_b(e: &NestCandidateEvent) -> NestInterval {
    NestInterval {
        start_time: e.interval_b.0 as u64,
        end_time: e.interval_b.1 as u64,
        idx: e.turn_source as u64,
    }
}

fn terminal_bits_new(
    classification: &Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    classification
        .levels
        .get(event.level as usize)?
        .bsp
        .iter()
        .find(|point| {
            point.source_index == event.turn_source && point.bits.confirm_side(event.side)
        })
        .map(|point| point.bits)
}

/// level_view.rs:431 私有 `leg_as_segment` 的探针内等价复刻（字段全 pub，逐字段一致）。
fn leg_as_segment_local(value: &LowerLeg) -> Segment {
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

/// signal.rs:208 `nearest_confirmed_center_idx`（pub(crate)，bin 不可达）的等价复刻。
fn nearest_center_local(centers: &[Center], seg_start: usize) -> Option<Center> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(centers[hi - 1])
    }
}

// ───────────────────────── 分级测量（p109 V3 管线 + run 内中枢复原） ─────────────────────────

fn measure_level(
    level: usize,
    windows: &[LeveledMove],
    lower_legs: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    centers_map: &mut BTreeMap<EventKey, Center>,
    center_miss: &mut usize,
) -> Result<LevelData, String> {
    let mut data = LevelData::default();
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).is_ok();
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L{level} run 投影失败: {error:?}"))?;
                let centers: Vec<Center> =
                    projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = decompose::decompose(&centers);
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
                for pair in view.moves.windows(2) {
                    let (
                        CompletionStatus::Completed { evidence: ev0, .. },
                        CompletionStatus::Completed { .. },
                    ) = (&pair[0].completion, &pair[1].completion)
                    else {
                        continue;
                    };
                    data.pairs.push(PairRec {
                        start: pair[0].start_index,
                        end: pair[1].end_index,
                        leave_kind: pair[0].kind,
                        leave_dir: pair[0].direction,
                        leave_via_divergence: matches!(
                            ev0,
                            CompletionEvidence::TerminalDivergence { .. }
                        ),
                    });
                }
                let run_events = provide_nest_candidate_events(
                    level as u32,
                    &projection,
                    &blocks,
                    lower_legs,
                    &view,
                    hist,
                    dif,
                    close_src,
                );
                // run 内中枢复原（doc §5.3 工件缺口的探针侧补位，不改生产）：
                //   Trend → 生产同一 pair 的 block_end_center 中枢（= 教义 B，最后中枢）；
                //   Pan   → c 段 start 前最近已确认中枢（= 教义 A，生产 signal.rs:208 同式）。
                let segments: Vec<Segment> = lower_legs.iter().map(leg_as_segment_local).collect();
                for e in &run_events {
                    let recovered = match e.kind {
                        NestDivergenceKind::Trend => {
                            let dir = match e.side {
                                Side::Long => Direction::Down,
                                Side::Short => Direction::Up,
                            };
                            let mut hits = view.pairs.iter().filter(|p| {
                                p.seg_a == e.seg_a
                                    && p.seg_c == e.interval_b
                                    && p.id.direction == dir
                                    && p.id.block_end_center < projection.seeds.len()
                            });
                            let first = hits.next();
                            if hits.next().is_some() {
                                // 同一 (seg_a,seg_c,dir) 对应多个 block_end_center：歧义，标记。
                                println!(
                                    "P111_CENTER_AMBIGUOUS level={} turn={} seg_a={:?} seg_c={:?}",
                                    e.level, e.turn_source, e.seg_a, e.interval_b
                                );
                            }
                            first.map(|p| projection.seeds[p.id.block_end_center].center)
                        }
                        NestDivergenceKind::Consolidation => segments
                            .iter()
                            .find(|s| s.end_index == e.interval_b.1)
                            .and_then(|s| nearest_center_local(&centers, s.start_index)),
                    };
                    match recovered {
                        Some(c) => {
                            centers_map.entry(event_key(e)).or_insert(c);
                        }
                        None => *center_miss += 1,
                    }
                }
                data.events.extend(run_events);
                run_start = None;
            }
            _ => {}
        }
    }
    // 与 p92 collect_snapshot_candidates / p102 / p109 同 dedup 键序。
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

// ───────────────────────── 链枚举与逐门对账（p109 语义复刻） ─────────────────────────

fn chain_dp_count(levels: &[LevelData]) -> u128 {
    if levels.is_empty() {
        return 0;
    }
    let mut previous: Vec<(usize, u128)> =
        levels[0].pairs.iter().map(|_| (1usize, 1u128)).collect();
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

/// p109 reconcile 的精简复刻：门序一致（身份→背驰→终端→方向/包含），
/// 包含门击杀时记录**全部** frontier 子事件 × 同侧父池（不止 p109 的 first_path 最近父）。
fn reconcile_chain(
    id: usize,
    chain: &[(usize, usize)],
    levels: &[LevelData],
    match_maps: &[BTreeMap<(usize, usize), Vec<usize>>],
    classification: &Classification,
    inclusion_rows: &mut Vec<InclusionRow>,
) -> (KillGate, usize) {
    let n = chain.len();
    let events_of = |k: usize| -> &Vec<NestCandidateEvent> { &levels[chain[k].0].events };
    let matched = |k: usize| -> Vec<usize> {
        let (lk, pi) = chain[k];
        let pair = &levels[lk].pairs[pi];
        match_maps[lk]
            .get(&(pair.start, pair.end))
            .cloned()
            .unwrap_or_default()
    };
    let base_candidates = matched(0);
    if base_candidates.is_empty() {
        return (KillGate::Identity, 1);
    }
    let div_bases: Vec<usize> = base_candidates
        .iter()
        .copied()
        .filter(|&i| events_of(0)[i].divergence_confirmed)
        .collect();
    if div_bases.is_empty() {
        return (KillGate::Divergence, 1);
    }
    let term_bases: Vec<usize> = div_bases
        .iter()
        .copied()
        .filter(|&i| terminal_bits_new(classification, &events_of(0)[i]).is_some())
        .collect();
    if term_bases.is_empty() {
        return (KillGate::Terminal, 1);
    }
    let mut frontier: Vec<Vec<usize>> = term_bases.iter().map(|&i| vec![i]).collect();
    for k in 1..n {
        let level = k + 1;
        let level_matches = matched(k);
        let mut next: Vec<Vec<usize>> = Vec::new();
        let mut any_identity = false;
        let mut any_side = false;
        let mut fail_pairs: Vec<(usize, Vec<usize>)> = Vec::new(); // (child event idx, side pool)
        for path in &frontier {
            let side = events_of(0)[path[0]].side;
            if level_matches.is_empty() {
                continue;
            }
            any_identity = true;
            let mut pool: Vec<usize> = level_matches
                .iter()
                .copied()
                .filter(|&i| events_of(k)[i].side == side)
                .collect();
            if pool.is_empty() {
                continue;
            }
            any_side = true;
            // 生产 DFS 候选序（p109:609-612 同式）。
            pool.sort_by_key(|&i| {
                let e = &events_of(k)[i];
                (e.interval_b.1, e.interval_b.0, e.turn_source, i)
            });
            let child_idx = *path.last().expect("nonempty path");
            let child_iv = typed_b(&events_of(k - 1)[child_idx]);
            let mut child_pass = false;
            for &i in &pool {
                if is_sub(&child_iv, &typed_b(&events_of(k)[i])) {
                    child_pass = true;
                    let mut extended = path.clone();
                    extended.push(i);
                    next.push(extended);
                }
            }
            if !child_pass {
                fail_pairs.push((child_idx, pool.clone()));
            }
        }
        if next.is_empty() {
            if !any_identity {
                return (KillGate::Identity, level);
            }
            if !any_side {
                return (KillGate::Direction, level);
            }
            for (child_idx, pool) in fail_pairs {
                inclusion_rows.push(InclusionRow {
                    chain_id: id,
                    kill_level: level,
                    base_turn: events_of(0)[*term_bases.first().expect("nonempty term_bases")]
                        .turn_source,
                    child: event_key(&events_of(k - 1)[child_idx]),
                    parents: pool.iter().map(|&i| event_key(&events_of(k)[i])).collect(),
                });
            }
            return (KillGate::Inclusion, level);
        }
        frontier = next;
        if frontier.len() > 50_000 {
            frontier.truncate(50_000);
        }
    }
    (KillGate::Alive, n)
}

// ───────────────────────── dump 解析（只读，p109 同款） ─────────────────────────

#[derive(Debug, Default)]
struct DumpBook {
    cert_lines: BTreeSet<String>,
}

fn parse_dump(path: &str) -> Result<DumpBook, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 dump 失败: {e}"))?;
    let mut book = DumpBook::default();
    for line in text.lines() {
        if !line.starts_with("CERT ") {
            continue;
        }
        let mut caliber = "";
        let mut exec = "";
        let mut top = "";
        let mut side = "";
        let mut ids = "";
        for field in line.split_whitespace() {
            if let Some(v) = field.strip_prefix("caliber=") {
                caliber = v;
            } else if let Some(v) = field.strip_prefix("exec=") {
                exec = v;
            } else if let Some(v) = field.strip_prefix("top=") {
                top = v;
            } else if let Some(v) = field.strip_prefix("side=") {
                side = v;
            } else if let Some(v) = field.strip_prefix("ids=") {
                ids = v;
            }
        }
        if !ids.is_empty() {
            book.cert_lines.insert(format!(
                "caliber={caliber} exec={exec} top={top} side={side} ids={ids}"
            ));
        }
    }
    Ok(book)
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
    let ids = certificate
        .identities()
        .iter()
        .map(|id| (id.level, id.turn_source, id.interval_b))
        .collect::<Vec<_>>();
    format!(
        "{exec}:{top}:{:?}:{:?}:{:?}:{:?}:{:?}:{ids:?}",
        certificate.caliber(),
        cert.side(),
        cert.base_interval(),
        rungs,
        certificate.kinds()
    )
}

// ───────────────────────── 父级有效性判别（doc §2/§5 实装） ─────────────────────────

#[derive(Debug, Clone)]
struct ParentVerdict {
    key: EventKey,
    center: Option<Center>,
    threshold_name: &'static str,
    threshold: Tick,
    c_extreme: Tick,
    t_touch: i64,
    t_break: i64,
    verdict: &'static str,
    divergence_confirmed: bool,
    edges_using: usize,
    control: bool,
}

/// 时序判别：自 parent.end（= interval_b.1）下一 bar 起全数据扫描。
/// Long（底背驰）：触阈值 = high ≥ threshold；破极值 = low < c_low（interval_b 内 min low）。
/// Short（顶背驰）镜像：触 = low ≤ threshold；破 = high > c_high。
fn test_parent(
    e: &NestCandidateEvent,
    center: Option<Center>,
    bars: &[Bar],
    control: bool,
) -> ParentVerdict {
    let (s, end) = e.interval_b;
    let long = matches!(e.side, Side::Long);
    let c_extreme: Tick = if long {
        bars[s..=end]
            .iter()
            .map(|b| b.low)
            .min()
            .unwrap_or(Tick::MAX)
    } else {
        bars[s..=end]
            .iter()
            .map(|b| b.high)
            .max()
            .unwrap_or(Tick::MIN)
    };
    let (threshold_name, threshold) = match (center, &e.kind) {
        (Some(c), NestDivergenceKind::Trend) => {
            if long {
                ("DD(B)", c.dd)
            } else {
                ("GG(B)", c.gg)
            }
        }
        (Some(c), NestDivergenceKind::Consolidation) => {
            if long {
                ("ZD(A)", c.zd)
            } else {
                ("ZG(A)", c.zg)
            }
        }
        (None, _) => ("NA", 0),
    };
    let mut t_touch: i64 = -1;
    let mut t_break: i64 = -1;
    if center.is_some() {
        for i in (end + 1)..bars.len() {
            let b = &bars[i];
            if t_touch < 0 {
                let touched = if long {
                    b.high >= threshold
                } else {
                    b.low <= threshold
                };
                if touched {
                    t_touch = i as i64;
                }
            }
            if t_break < 0 {
                let broke = if long {
                    b.low < c_extreme
                } else {
                    b.high > c_extreme
                };
                if broke {
                    t_break = i as i64;
                }
            }
            if t_touch >= 0 && t_break >= 0 {
                break;
            }
        }
    }
    let verdict = if center.is_none() {
        "NO_CENTER"
    } else if end + 1 >= bars.len() {
        "NO_DATA_AFTER"
    } else {
        match (t_touch >= 0, t_break >= 0) {
            (true, true) => {
                if t_touch < t_break {
                    "TOUCH_BEFORE_BREAK" // (b) 拒发正确
                } else if t_break < t_touch {
                    "BREAK_BEFORE_TOUCH" // (a*) 父级背驰不成立
                } else {
                    "SAME_BAR" // 1m bar 内时序不可知，单列
                }
            }
            (true, false) => "TOUCH_NO_BREAK", // (b)：定理后果已兑现，极值未再破
            (false, true) => "BREAK_NO_TOUCH", // (a*)：破极值且至数据末未触阈值
            (false, false) => "NEITHER",       // 中枢下方横盘，单列观察
        }
    };
    ParentVerdict {
        key: event_key(e),
        center,
        threshold_name,
        threshold,
        c_extreme,
        t_touch,
        t_break,
        verdict,
        divergence_confirmed: e.divergence_confirmed,
        edges_using: 0,
        control,
    }
}

fn fmt_key(k: &EventKey) -> String {
    format!(
        "L{}:{}:{}-{}:{}{}",
        k.0,
        k.5,
        k.4 .0,
        k.4 .1,
        if k.1 { "S" } else { "L" },
        if k.2 { "P" } else { "T" }
    )
}

fn find_event<'a>(
    events_by_level: &'a [Vec<NestCandidateEvent>],
    level: usize,
    turn_source: usize,
    interval_b: (usize, usize),
) -> Option<&'a NestCandidateEvent> {
    events_by_level
        .get(level)?
        .iter()
        .find(|e| e.turn_source == turn_source && e.interval_b == interval_b)
}

// ───────────────────────── main ─────────────────────────

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p111_parent_divergence_validity <btc_1m_full.json> [p92_ckpt_dump.txt]")?;
    let dump_path = args.next();
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P111_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    let bars = &loaded.bars[..max_bars];
    let as_of = bars.len() - 1;
    println!(
        "P111_INPUT bars={} replay_bars={} as_of={} last_date={}",
        loaded.bars.len(),
        max_bars,
        as_of,
        loaded.last_date
    );
    let layer = parse_layer(bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let hist = compute_macd(&closes, &config.macd).hist;
    let dif = compute_macd(&closes, &config.macd).dif;
    println!(
        "P111_INPUT merged={} tower_levels={} classification_levels={}",
        layer.merged_bars.len(),
        tower.len(),
        classification.levels.len()
    );

    // ── 分级测量（V3 生产口径）＋ run 内中枢复原 ──
    let mut levels: Vec<LevelData> = Vec::new();
    let mut centers_map: BTreeMap<EventKey, Center> = BTreeMap::new();
    let mut center_miss = 0usize;
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let data = measure_level(
            level,
            &tower[level],
            &lower,
            as_of,
            &hist,
            &dif,
            &close_src,
            &mut centers_map,
            &mut center_miss,
        )?;
        println!(
            "P111_LEVEL L{} runs={} strict_pairs={} events={}",
            level,
            data.runs,
            data.pairs.len(),
            data.events.len()
        );
        levels.push(data);
    }
    println!(
        "P111_CENTER_RECOVERY recovered={} miss={}",
        centers_map.len(),
        center_miss
    );

    // ── 硬锚①：事件总数复现 P92_YIELD / P109_ANCHOR_EVENTS ──
    let total_events: usize = levels.iter().map(|l| l.events.len()).sum();
    let total_confirmed: usize = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed)
        .count();
    let trend_confirmed = levels
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed && e.kind == NestDivergenceKind::Trend)
        .count();
    println!(
        "P111_ANCHOR_EVENTS candidates={} divergence_confirmed={} trend_confirmed={} expected=3683/1235/429",
        total_events, total_confirmed, trend_confirmed
    );

    // ── 证书复现（双口径对象 + dump 66 行双向 diff 锚）──
    let mut events_by_level: Vec<Vec<NestCandidateEvent>> = vec![Vec::new()];
    events_by_level.extend(levels.iter().map(|l| l.events.clone()));
    let mut certs_a: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    let mut certs_b: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    let mut cert_strings = BTreeSet::new();
    for caliber in [NestIntervalCaliber::A, NestIntervalCaliber::B] {
        let mut seen = BTreeSet::new();
        for exec in 1..events_by_level.len() {
            for top in exec..events_by_level.len() {
                for cert in assemble_typed_certificates(&events_by_level, exec, top, caliber, |e| {
                    terminal_bits_new(&classification, e)
                }) {
                    if !seen.insert(certificate_key(exec, top, &cert)) {
                        continue;
                    }
                    let ids = cert
                        .identities()
                        .iter()
                        .map(|id| {
                            format!(
                                "{}:{}:{}-{}",
                                id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    cert_strings.insert(format!(
                        "caliber={caliber:?} exec={exec} top={top} side={:?} ids={ids}",
                        cert.certificate().side()
                    ));
                    match caliber {
                        NestIntervalCaliber::A => certs_a.push((exec, top, cert)),
                        NestIntervalCaliber::B => certs_b.push((exec, top, cert)),
                    }
                }
            }
        }
    }
    let dump = match &dump_path {
        Some(p) => Some(parse_dump(p)?),
        None => None,
    };
    if let Some(book) = &dump {
        let missing = book.cert_lines.difference(&cert_strings).count();
        let extra = cert_strings.difference(&book.cert_lines).count();
        println!(
            "P111_ANCHOR_CERT reproduced={} dump={} missing_in_repro={} extra_in_repro={}",
            cert_strings.len(),
            book.cert_lines.len(),
            missing,
            extra
        );
    } else {
        println!("P111_ANCHOR_CERT reproduced={} dump=NA", cert_strings.len());
    }

    // ── 结构链枚举 + 逐门对账（锚 p109：inclusion=30）──
    let dp_complete = chain_dp_count(&levels);
    let (chains, truncated) = enumerate_chains(&levels, 500_000);
    println!(
        "P111_ANCHOR_ENUM chains={} truncated={} dp_complete={} expected=6482",
        chains.len(),
        truncated,
        dp_complete
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
    let mut inclusion_rows: Vec<InclusionRow> = Vec::new();
    let mut gate_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (id, chain) in chains.iter().enumerate() {
        let (gate, _kill_level) = reconcile_chain(
            id,
            chain,
            &levels,
            &match_maps,
            &classification,
            &mut inclusion_rows,
        );
        *gate_counts.entry(gate.label()).or_default() += 1;
    }
    for (gate, count) in &gate_counts {
        println!(
            "P111_ANCHOR_GATE gate={} count={} (p109 期望 identity=3790 terminal=1631 divergence=1020 inclusion=30 direction=11 alive=0)",
            gate, count
        );
    }
    println!(
        "P111_INCLUSION_ROWS rows={}（frontier 子×父池全展开，可多于 30）",
        inclusion_rows.len()
    );

    // ── A 证书宇宙 is_sub@B 判负边（含 p102 五边锚）──
    // p102 §3 五条边：(base_turn, parent_turn)；前四条 L2→L1，末条 L3→L2。
    let p102_anchors: [(usize, usize); 5] = [
        (706241, 704358),
        (1588321, 1574631),
        (1834972, 1832757),
        (3745754, 3750585),
        (706241, 689893),
    ];
    let mut cert_edges: Vec<CertEdgeRow> = Vec::new();
    for (exec, top, cert) in &certs_a {
        let ids = cert.identities();
        let base_turn = ids.last().map(|i| i.turn_source).unwrap_or(0);
        for k in 0..ids.len().saturating_sub(1) {
            let parent_id = ids[k];
            let child_id = ids[k + 1];
            let Some(parent) = find_event(
                &events_by_level,
                parent_id.level as usize,
                parent_id.turn_source,
                parent_id.interval_b,
            ) else {
                println!("P111_WARN cert 父事件未找到 {parent_id:?}");
                continue;
            };
            let Some(child) = find_event(
                &events_by_level,
                child_id.level as usize,
                child_id.turn_source,
                child_id.interval_b,
            ) else {
                println!("P111_WARN cert 子事件未找到 {child_id:?}");
                continue;
            };
            if is_sub(&typed_b(child), &typed_b(parent)) {
                continue; // B 口径也过，非被拒边
            }
            let anchor = p102_anchors
                .iter()
                .any(|&(b, p)| b == base_turn && p == parent.turn_source);
            cert_edges.push(CertEdgeRow {
                exec: *exec,
                top: *top,
                side: cert.certificate().side(),
                base_turn,
                child: event_key(child),
                parent: event_key(parent),
                p102_anchor: anchor,
            });
        }
    }
    let anchor_hit = cert_edges.iter().filter(|r| r.p102_anchor).count();
    println!(
        "P111_CERT_EDGES_A a_certs={} b_fail_edges={} p102_anchor_hit={}/5",
        certs_a.len(),
        cert_edges.len(),
        anchor_hit
    );

    // ── 对照组：B 证书 rung 父级（活体多级证书的父级，期望非 (a*)）──
    let mut control_parents: Vec<EventKey> = Vec::new();
    for (_exec, top, cert) in &certs_b {
        if *top <= 1 {
            continue;
        }
        let ids = cert.identities();
        for k in 0..ids.len().saturating_sub(1) {
            let parent_id = ids[k];
            if let Some(parent) = find_event(
                &events_by_level,
                parent_id.level as usize,
                parent_id.turn_source,
                parent_id.interval_b,
            ) {
                control_parents.push(event_key(parent));
            }
        }
    }

    // ── 汇总全部待测父级（去重）──
    let mut parent_set: BTreeMap<EventKey, bool> = BTreeMap::new(); // key → control?
    for row in &inclusion_rows {
        for p in &row.parents {
            parent_set.entry(*p).or_insert(false);
        }
    }
    for row in &cert_edges {
        parent_set.entry(row.parent).or_insert(false);
    }
    for p in &control_parents {
        parent_set.entry(*p).or_insert(true);
    }

    // ── 逐父级跑有效性判别 ──
    let mut verdicts: BTreeMap<EventKey, ParentVerdict> = BTreeMap::new();
    for (key, control) in &parent_set {
        let (level, _, _, _, interval_b, turn_source) = *key;
        let Some(e) = find_event(&events_by_level, level as usize, turn_source, interval_b) else {
            println!("P111_WARN 父事件回查失败 {}", fmt_key(key));
            continue;
        };
        let center = centers_map.get(key).copied();
        let mut v = test_parent(e, center, bars, *control);
        v.edges_using = inclusion_rows
            .iter()
            .filter(|r| r.parents.contains(key))
            .count()
            + cert_edges.iter().filter(|r| &r.parent == key).count();
        verdicts.insert(*key, v);
    }

    // ── 逐边细分（(a*) 的 a1/a2/a3 与窗口事实）──
    // 对每条被拒边 × 其父：输出 shape、窗口内触/破事实、(a*) 时按 doc §5.2 重测力度。
    let mut edge_rows_out: Vec<String> = Vec::new();
    let mut revive_chain: BTreeMap<usize, (usize, EventKey, EventKey)> = BTreeMap::new(); // chain → (kill_level, child, parent)
    let mut revive_cert = 0usize;
    let mut sub_counts: BTreeMap<&'static str, usize> = BTreeMap::new();

    let emit_edge = |src: &str,
                     id_label: String,
                     base_turn: usize,
                     kill_level: usize,
                     child_k: &EventKey,
                     parent_k: &EventKey,
                     verdicts: &BTreeMap<EventKey, ParentVerdict>,
                     events_by_level: &[Vec<NestCandidateEvent>],
                     edge_rows_out: &mut Vec<String>,
                     sub_counts: &mut BTreeMap<&'static str, usize>|
     -> bool {
        let Some(v) = verdicts.get(parent_k) else {
            edge_rows_out.push(format!(
                "P111_EDGE {src} {id_label} parent={} NO_VERDICT",
                fmt_key(parent_k)
            ));
            return false;
        };
        let child = find_event(events_by_level, child_k.0 as usize, child_k.5, child_k.4);
        let parent = find_event(events_by_level, parent_k.0 as usize, parent_k.5, parent_k.4);
        let (Some(child), Some(parent)) = (child, parent) else {
            edge_rows_out.push(format!("P111_EDGE {src} {id_label} LOOKUP_FAIL"));
            return false;
        };
        let child_b = child.interval_b;
        let parent_b = parent.interval_b;
        let left_gap = child_b.0 as i64 - parent_b.0 as i64;
        let right_gap = parent_b.1 as i64 - child_b.1 as i64;
        let shape = if child_b.0 > parent_b.1 {
            "child_right_of_parent"
        } else if child_b.1 < parent_b.0 {
            "child_left_of_parent"
        } else {
            "overlap_not_contained"
        };
        let child_end = child_b.1 as i64;
        let touch_in_window = v.t_touch >= 0 && v.t_touch <= child_end;
        let break_in_window = v.t_break >= 0 && v.t_break <= child_end;
        // (a*) 细分（doc §5.2）：仅 BREAK_* 且 shape=right 时重锚才有意义。
        let mut sub = "-";
        let mut area_a = f64::NAN;
        let mut area_cx = f64::NAN;
        let mut revivable = false;
        if matches!(v.verdict, "BREAK_BEFORE_TOUCH" | "BREAK_NO_TOUCH") {
            if !v.divergence_confirmed {
                sub = "a3"; // 父级层面本无背驰力度结构（级别误判型）
            } else if shape == "child_right_of_parent" {
                let a_map = map_src_to_close_idx(&close_src, parent.seg_a.0, parent.seg_a.1);
                let cx_map = map_src_to_close_idx(&close_src, parent_b.0, child_b.1);
                if let (Some(a), Some(cx)) = (a_map, cx_map) {
                    area_a = segment_macd_area(&hist, a.0, a.1);
                    area_cx = segment_macd_area(&hist, cx.0, cx.1);
                    if area_cx < area_a {
                        sub = "a1"; // 背驰段仍在延伸 → 可重锚
                                    // 重锚父区间 (seg_c.0, child.end) 的闭包含检查。
                        revivable = left_gap >= 0;
                    } else {
                        sub = "a2"; // 力度转强 → 父证无效
                    }
                } else {
                    sub = "a_unmap";
                }
            } else {
                sub = "a_shape_not_right";
            }
        }
        *sub_counts.entry(sub).or_insert(0) += 1;
        edge_rows_out.push(format!(
                "P111_EDGE src={} {} base_turn={} edge_to_L{} child={} parent={} shape={} left_gap={} right_gap={} verdict={} sub={} t_touch={} t_break={} child_end={} touch_in_window={} break_in_window={} area_a={:.4} area_c_ext={:.4} revivable={}",
                src, id_label, base_turn, kill_level, fmt_key(child_k), fmt_key(parent_k),
                shape, left_gap, right_gap, v.verdict, sub, v.t_touch, v.t_break, child_end,
                touch_in_window, break_in_window, area_a, area_cx, revivable
            ));
        revivable
    };

    for row in &inclusion_rows {
        for parent_k in &row.parents {
            let ok = emit_edge(
                "CHAIN",
                format!("chain={}", row.chain_id),
                row.base_turn,
                row.kill_level,
                &row.child,
                parent_k,
                &verdicts,
                &events_by_level,
                &mut edge_rows_out,
                &mut sub_counts,
            );
            if ok {
                revive_chain
                    .entry(row.chain_id)
                    .or_insert((row.kill_level, row.child, *parent_k));
            }
        }
    }
    for row in &cert_edges {
        let ok = emit_edge(
            "CERTA",
            format!(
                "exec={} top={} side={:?} p102_anchor={}",
                row.exec, row.top, row.side, row.p102_anchor
            ),
            row.base_turn,
            row.parent.0 as usize,
            &row.child,
            &row.parent,
            &verdicts,
            &events_by_level,
            &mut edge_rows_out,
            &mut sub_counts,
        );
        revive_cert += usize::from(ok);
    }

    // ── 输出 ──
    println!("P111_NOTE 阈值口径: Trend父→DD(B)/Short镜像GG(B); Pan父→ZD(A)/Short镜像ZG(A); 时序扫描区间=(parent.end, data_end]");
    let mut verdict_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for v in verdicts.values() {
        *verdict_counts.entry(v.verdict).or_insert(0) += 1;
        let c = v.center;
        println!(
            "P111_PARENT {} kind={} div_confirmed={} center={} threshold={}@{} c_extreme={} t_touch={} t_break={} verdict={} edges_using={} control={}",
            fmt_key(&v.key),
            if v.key.2 { "Pan" } else { "Trend" },
            v.divergence_confirmed,
            c.map_or("NA".to_string(), |c| format!(
                "[zd={},zg={},dd={},gg={},span=({}, {})]",
                c.zd, c.zg, c.dd, c.gg, c.start_index, c.end_index
            )),
            v.threshold_name,
            v.threshold,
            v.c_extreme,
            v.t_touch,
            v.t_break,
            v.verdict,
            v.edges_using,
            v.control
        );
    }
    for line in &edge_rows_out {
        println!("{line}");
    }
    for (verdict, count) in &verdict_counts {
        println!("P111_SUM_VERDICT verdict={verdict} parents={count}");
    }
    for (sub, count) in &sub_counts {
        println!("P111_SUM_SUB sub={sub} edges={count}");
    }
    let mut revive_levels: BTreeSet<usize> = BTreeSet::new();
    for (_chain, (level, _, _)) in &revive_chain {
        revive_levels.insert(*level);
    }
    println!(
        "P111_REVIVE chains_inclusion_killed={} chains_revivable_a1={} revive_levels={:?} cert_edges_revivable_a1={}",
        gate_counts.get("inclusion").copied().unwrap_or(0),
        revive_chain.len(),
        revive_levels,
        revive_cert
    );
    for (chain, (level, child, parent)) in &revive_chain {
        println!(
            "P111_REVIVE_ROW chain={} kill_level=L{} child={} parent={}",
            chain,
            level,
            fmt_key(child),
            fmt_key(parent)
        );
    }
    Ok(())
}

// ───────────────────────── bar 加载（p102/p109 同款） ─────────────────────────

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
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
