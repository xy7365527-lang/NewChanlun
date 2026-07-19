//! task #109：#83 结构嵌套链逐门对账探针（只读，不改任何生产源码）。
//!
//! 以 #83 口径（严格 C2 pair 区间 [leave.start, retest.end]、相邻级闭包含 is_sub、
//! 全深度完整链）在**现行生产语义**（V3 CarriedOnly + #105 Trend C 终段修复）下重测
//! 结构嵌套链，并对每条链逐门对账死于哪扇门：
//!   身份门（provider 无对应事件）→ 背驰确认门（基例 divergence_confirmed）
//!   → 终端门（terminal_bits_new confirm_side）→ 方向/包含门（逐边 side ∧ is_sub@B）
//!   → 时钟门（全通但 CKPT/CERT dump 无因果见证）。
//! 归因语义 = 生产装配 ∃ 语义的首个空候选门（与 assemble_typed_certificate 的 DFS
//! 完备性一致：门按管线序处理，候选集首次清空的门即击杀门）。
//!
//! 交叉校验（硬锚）：候选事件总数须复现 P92_YIELD candidates=3,683；
//! 终端快照证书集须与 /tmp/p92_ckpt_dump.txt 的 66 行 CERT 双向 diff=0。
//!
//! 用法：
//! `cargo run --release --bin p109_chain157_gate_attrition -- <btc_1m_full.json> [p92_ckpt_dump.txt]`
//! 冒烟：`P109_MAX_BARS=250000 ...`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows,
    project_extended_windows_carried_only, provide_nest_candidate_events, C2LevelViewConfig,
    C2VersionTuple, CompletionEvidence, CompletionStatus, CoordinateWindow, LevelViewMaterial,
    LevelViewQuery, NestCandidateEvent, ProjectionMaterial, ProviderVersion,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_typed_certificates, is_sub, NestInterval, NestIntervalCaliber, TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::classifier::Classification;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{
    quantize, Bar, BspBits, Direction, MoveKind, Side, Timestamp,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// 严格 C2 pair（同 run 内两个不同、正向相邻、均 Completed 的 Move）。
#[derive(Debug, Clone, Copy)]
struct PairRec {
    start: usize,
    end: usize,
    leave_kind: MoveKind,
    leave_dir: Option<Direction>,
    retest_kind: MoveKind,
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
    tower_windows: usize,
    runs: usize,
    invalid_windows: usize,
    moves: usize,
    completed: usize,
    pairs: Vec<PairRec>,
    events: Vec<NestCandidateEvent>,
}

/// 击杀人（门）分类。Direction 归并到身份门（链在该级找不到同向可用身份）；
/// Clock 只裁决全通链；Alive 表示全门通过且 dump 有见证。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillGate {
    Identity,
    Divergence,
    Terminal,
    Direction,
    Inclusion,
    Clock,
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
            KillGate::Clock => "clock",
            KillGate::Alive => "alive",
        }
    }
}

#[derive(Debug, Clone)]
struct ChainOutcome {
    id: usize,
    pairs: Vec<PairRec>,
    events_per_level: Vec<usize>,
    kill_gate: KillGate,
    kill_level: usize,
    kill_detail: String,
    /// 最佳路径实际通过的最深级别（1..=n；0 = 基例门未过）。
    max_depth: usize,
    base_side: Option<Side>,
    /// 生产 DFS 序首条最深可行路径的事件身份（低→高，level 起始于 1）。
    canonical: Vec<(u32, usize, (usize, usize))>,
    feasible_full_paths: usize,
    /// 最长全门前缀深度 m（exec=1 的 (1,m) 证书在逻辑上存在）。
    prefix_cert_depth: usize,
    prefix_cert_in_dump: Option<bool>,
    base_in_66: Option<bool>,
    clock_witness: Option<String>,
    /// 基例门逐类计数：(背驰确认 Trend 基例数, 背驰确认 Pan 基例数)。
    div_base_trend: usize,
    div_base_pan: usize,
}

#[derive(Debug, Default, Clone, Copy)]
struct EdgeDiag {
    status: Option<&'static str>,
    pool_side: usize,
    left_gap: i64,
    right_gap: i64,
    shape: &'static str,
    parent_turn: usize,
    parent_b: (usize, usize),
}

fn typed_b(event: &NestCandidateEvent) -> NestInterval {
    NestInterval {
        start_time: event.interval_b.0 as u64,
        end_time: event.interval_b.1 as u64,
        idx: event.turn_source as u64,
    }
}

fn typed_a(event: &NestCandidateEvent) -> NestInterval {
    NestInterval {
        start_time: event.interval_a.0 as u64,
        end_time: event.interval_a.1 as u64,
        idx: event.turn_source as u64,
    }
}

fn identity_of(event: &NestCandidateEvent) -> (u32, usize, (usize, usize)) {
    (event.level, event.turn_source, event.interval_b)
}

fn ids_string(path: &[(u32, usize, (usize, usize))]) -> String {
    path.iter()
        .rev()
        .map(|(level, turn, b)| format!("{level}:{turn}:{}-{}", b.0, b.1))
        .collect::<Vec<_>>()
        .join("|")
}

fn terminal_bits_new(classification: &Classification, event: &NestCandidateEvent) -> Option<BspBits> {
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

fn v2_tuple() -> C2VersionTuple {
    C2VersionTuple {
        direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
        divergence_pair_provider_version: Some(ProviderVersion::MOVE_BLOCK_AC_V1),
        projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V2),
    }
}

/// 单级测量：runs 切分（V3=carried_only / V2=历史基准）→ 投影 → decompose → C2 视图
/// → 严格 pair 与（仅 V3）provider 候选事件。与 p92/p102 终态快照同管线。
fn measure_level(
    level: usize,
    windows: &[LeveledMove],
    lower_legs: &[newchan_rust::theta_v0::classifier::level_view::LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    v3: bool,
    collect_events: bool,
) -> Result<LevelData, String> {
    let mut data = LevelData {
        tower_windows: windows.len(),
        ..LevelData::default()
    };
    let project = |slice: &[LeveledMove]| {
        if v3 {
            project_extended_windows_carried_only(slice)
        } else {
            project_extended_windows(slice)
        }
    };
    let version = if v3 {
        C2VersionTuple::auto_pairing()
    } else {
        v2_tuple()
    };
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && match project(std::slice::from_ref(&windows[index])) {
                Ok(_) => true,
                Err(_) => {
                    data.invalid_windows += 1;
                    false
                }
            };
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project(&windows[start..index])
                    .map_err(|error| format!("L{level} run 投影失败: {error:?}"))?;
                let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = decompose::decompose(&centers);
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
                data.runs += 1;
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
                        leave_dir: pair[0].direction,
                        retest_kind: pair[1].kind,
                        leave_via_divergence: matches!(ev0, CompletionEvidence::TerminalDivergence { .. }),
                    });
                }
                if collect_events {
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
                }
                run_start = None;
            }
            _ => {}
        }
    }
    if collect_events {
        // 与 p92 collect_snapshot_candidates / p102 同 dedup 键序。
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
    }
    Ok(data)
}

/// p83 chain_stats 语义复刻：返回（全深度完整链数，最大连续深度，该深度链数）。
fn chain_dp_count(levels: &[LevelData]) -> (u128, usize, u128) {
    if levels.is_empty() {
        return (0, 0, 0);
    }
    let mut previous: Vec<(usize, u128)> = levels[0].pairs.iter().map(|_| (1usize, 1u128)).collect();
    let mut global_depth = usize::from(!previous.is_empty());
    let mut global_count = previous.len() as u128;
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
        let level_depth = current.iter().map(|v| v.0).max().unwrap_or(0);
        let level_count: u128 = current
            .iter()
            .filter(|v| v.0 == level_depth)
            .map(|v| v.1)
            .sum();
        if level_depth > global_depth {
            global_depth = level_depth;
            global_count = level_count;
        } else if level_depth == global_depth && level_depth > 0 {
            global_count += level_count;
        }
        previous = current;
    }
    let complete = previous
        .iter()
        .filter(|v| v.0 == levels.len())
        .map(|v| v.1)
        .sum();
    (complete, global_depth, global_count)
}

/// 枚举全部全深度链（L1 起逐级向上，闭包含边）。cap 防爆。
fn enumerate_chains(levels: &[LevelData], cap: usize) -> (Vec<Vec<(usize, usize)>>, bool) {
    let n = levels.len();
    let mut out: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut truncated = false;
    // parents[k][i] = level k 第 i 个 pair 在 level k+1 的父 pair 下标表（k: 0-based）。
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
    // chain 元素 = (level_offset, pair_index)；显式栈 DFS，path.truncate 回溯。
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


/// 逐门对账：门按生产装配管线序处理，候选集首次清空的那扇门即击杀门（∃ 语义）。
#[allow(clippy::too_many_arguments)]
fn reconcile_chain(
    id: usize,
    chain: &[(usize, usize)],
    levels: &[LevelData],
    match_maps: &[BTreeMap<(usize, usize), Vec<usize>>],
    classification: &Classification,
    edge_diags: &mut Vec<(usize, usize, EdgeDiag)>,
    invariant_violations: &mut usize,
) -> ChainOutcome {
    let n = chain.len();
    let pairs: Vec<PairRec> = chain
        .iter()
        .map(|(k, pi)| levels[*k].pairs[*pi])
        .collect();
    let events_per_level: Vec<usize> = (0..n)
        .map(|k| {
            let (lk, pi) = chain[k];
            let pair = &levels[lk].pairs[pi];
            match_maps[lk]
                .get(&(pair.start, pair.end))
                .map_or(0, Vec::len)
        })
        .collect();
    let events_of = |k: usize| -> &Vec<NestCandidateEvent> { &levels[chain[k].0].events };
    let matched = |k: usize| -> Vec<usize> {
        let (lk, pi) = chain[k];
        let pair = &levels[lk].pairs[pi];
        match_maps[lk]
            .get(&(pair.start, pair.end))
            .cloned()
            .unwrap_or_default()
    };

    let mut outcome = ChainOutcome {
        id,
        pairs,
        events_per_level,
        kill_gate: KillGate::Alive,
        kill_level: 0,
        kill_detail: String::new(),
        max_depth: 0,
        base_side: None,
        canonical: Vec::new(),
        feasible_full_paths: 0,
        prefix_cert_depth: 0,
        prefix_cert_in_dump: None,
        base_in_66: None,
        clock_witness: None,
        div_base_trend: 0,
        div_base_pan: 0,
    };

    // 门 1：身份门@L1 —— 基例级别 pair 在 provider 宇宙中无事件。
    let base_candidates = matched(0);
    if base_candidates.is_empty() {
        outcome.kill_gate = KillGate::Identity;
        outcome.kill_level = 1;
        let p = &outcome.pairs[0];
        outcome.kill_detail = format!(
            "L1 pair=({}, {}) leave={:?}/{:?} via_div={} retest={:?}：provider 无 interval_a 匹配事件",
            p.start, p.end, p.leave_kind, p.leave_dir, p.leave_via_divergence, p.retest_kind
        );
        return outcome;
    }
    // 门 2：背驰确认门@L1 —— 基例 divergence_confirmed。
    let div_bases: Vec<usize> = base_candidates
        .iter()
        .copied()
        .filter(|&i| events_of(0)[i].divergence_confirmed)
        .collect();
    outcome.div_base_trend = div_bases
        .iter()
        .filter(|&&i| {
            events_of(0)[i].kind
                == newchan_rust::theta_v0::classifier::level_view::NestDivergenceKind::Trend
        })
        .count();
    outcome.div_base_pan = div_bases.len() - outcome.div_base_trend;
    if div_bases.is_empty() {
        outcome.kill_gate = KillGate::Divergence;
        outcome.kill_level = 1;
        outcome.max_depth = 1;
        let kinds: Vec<String> = base_candidates
            .iter()
            .map(|&i| {
                let e = &events_of(0)[i];
                format!(
                    "{:?}/{:?} turn={} seg_a={:?} interval_b={:?}",
                    e.kind, e.side, e.turn_source, e.seg_a, e.interval_b
                )
            })
            .collect();
        outcome.kill_detail = format!(
            "L1 {} 个基例事件全部 divergence_confirmed=false：[{}]",
            base_candidates.len(),
            kinds.join("; ")
        );
        outcome.canonical = base_candidates
            .iter()
            .map(|&i| identity_of(&events_of(0)[i]))
            .take(1)
            .collect();
        return outcome;
    }
    // 门 3：终端门@L1 —— 基例 BSP confirm_side。
    let term_bases: Vec<usize> = div_bases
        .iter()
        .copied()
        .filter(|&i| terminal_bits_new(classification, &events_of(0)[i]).is_some())
        .collect();
    if term_bases.is_empty() {
        outcome.kill_gate = KillGate::Terminal;
        outcome.kill_level = 1;
        outcome.max_depth = 1;
        let kinds: Vec<String> = div_bases
            .iter()
            .map(|&i| {
                let e = &events_of(0)[i];
                let any_bsp = classification
                    .levels
                    .get(e.level as usize)
                    .map(|lvl| {
                        lvl.bsp
                            .iter()
                            .any(|point| point.source_index == e.turn_source)
                    })
                    .unwrap_or(false);
                format!(
                    "{:?}/{:?} turn={} interval_b={:?} any_bsp_at_turn={}",
                    e.kind, e.side, e.turn_source, e.interval_b, any_bsp
                )
            })
            .collect();
        outcome.kill_detail = format!(
            "L1 {} 个背驰确认基例全部无 confirm_side BSP：[{}]",
            div_bases.len(),
            kinds.join("; ")
        );
        outcome.canonical = div_bases
            .iter()
            .map(|&i| identity_of(&events_of(0)[i]))
            .take(1)
            .collect();
        return outcome;
    }
    outcome.max_depth = 1;
    outcome.prefix_cert_depth = 1;
    // 门 4：逐级边（方向一致 ∧ 闭包含@B；身份缺口先记）。frontier = 可行事件路径。
    // 候选排序复刻生产 extend_typed_upward：(interval_b 降序 sel_key, turn_source, index)。
    let mut frontier: Vec<Vec<usize>> = term_bases.iter().map(|&i| vec![i]).collect();
    let mut killed: Option<(KillGate, usize, String)> = None;
    for k in 1..n {
        let level = k + 1;
        let level_matches = matched(k);
        let mut next: Vec<Vec<usize>> = Vec::new();
        let mut any_identity = false;
        let mut any_side = false;
        let mut first_side_pool: Vec<usize> = Vec::new();
        let mut first_path: Option<&Vec<usize>> = None;
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
            if first_path.is_none() {
                first_path = Some(path);
                first_side_pool = pool.clone();
            }
            // 生产 DFS 候选序：typed_interval(B).sel_key() 升序（end, start, turn），再 index。
            pool.sort_by_key(|&i| {
                let e = &events_of(k)[i];
                (e.interval_b.1, e.interval_b.0, e.turn_source, i)
            });
            let child_iv = typed_b(&events_of(k - 1)[*path.last().expect("nonempty path")]);
            for i in pool {
                let parent_iv = typed_b(&events_of(k)[i]);
                if is_sub(&child_iv, &parent_iv) {
                    // A 口径不变量：匹配事件 interval_a == pair 区间，链边构造性成立。
                    let child_a = typed_a(&events_of(k - 1)[*path.last().expect("nonempty path")]);
                    let parent_a = typed_a(&events_of(k)[i]);
                    if !is_sub(&child_a, &parent_a) {
                        *invariant_violations += 1;
                    }
                    let mut extended = path.clone();
                    extended.push(i);
                    next.push(extended);
                }
            }
        }
        if next.is_empty() {
            let (gate, detail) = if !any_identity {
                let p = &outcome.pairs[k];
                (
                    KillGate::Identity,
                    format!(
                        "L{level} pair=({}, {}) leave={:?}/{:?} via_div={} retest={:?}：provider 无 interval_a 匹配事件",
                        p.start, p.end, p.leave_kind, p.leave_dir, p.leave_via_divergence, p.retest_kind
                    ),
                )
            } else if !any_side {
                (
                    KillGate::Direction,
                    format!(
                        "L{level} pair 有 {} 个事件但与基例侧不一致",
                        level_matches.len()
                    ),
                )
            } else {
                let diag = edge_gap_diag(
                    &events_of(k - 1)[*first_path.expect("side path").last().expect("nonempty")],
                    &first_side_pool,
                    events_of(k),
                );
                edge_diags.push((id, level, diag));
                (
                    KillGate::Inclusion,
                    format!(
                        "L{level} 同侧候选池={} 全部 is_sub@B 判负：最近候选 turn={} parent_b={:?} left_gap={} right_gap={} shape={}",
                        diag.pool_side, diag.parent_turn, diag.parent_b, diag.left_gap, diag.right_gap, diag.shape
                    ),
                )
            };
            killed = Some((gate, level, detail));
            break;
        }
        frontier = next;
        outcome.max_depth = level;
        outcome.prefix_cert_depth = level;
        if frontier.len() > 50_000 {
            frontier.truncate(50_000);
        }
    }
    // 规范路径 = 字典序（基例序 × rung 候选序）最小可行路径 = 生产 DFS 首成功路径。
    if let Some(best) = frontier.first() {
        outcome.base_side = Some(events_of(0)[best[0]].side);
        outcome.canonical = best
            .iter()
            .enumerate()
            .map(|(k, &i)| identity_of(&events_of(k)[i]))
            .collect();
        outcome.feasible_full_paths = frontier.len();
    }
    if let Some((gate, level, detail)) = killed {
        outcome.kill_gate = gate;
        outcome.kill_level = level;
        outcome.kill_detail = detail;
    }
    outcome
}


/// 包含门击杀的最近父候选几何（p102 同款 left/right gap 与形态分类）。
fn edge_gap_diag(
    child: &NestCandidateEvent,
    side_pool: &[usize],
    parent_events: &[NestCandidateEvent],
) -> EdgeDiag {
    let child_b = typed_b(child);
    let mut best: Option<(i64, EdgeDiag)> = None;
    for &i in side_pool {
        let parent = &parent_events[i];
        let parent_b = typed_b(parent);
        let left_gap = child_b.start_time as i64 - parent_b.start_time as i64;
        let right_gap = parent_b.end_time as i64 - child_b.end_time as i64;
        let violation = (if left_gap < 0 { -left_gap } else { 0 })
            + (if right_gap < 0 { -right_gap } else { 0 });
        let shape = if child_b.start_time > parent_b.end_time {
            "child_right_of_parent"
        } else if child_b.end_time < parent_b.start_time {
            "child_left_of_parent"
        } else {
            "overlap_not_contained"
        };
        let diag = EdgeDiag {
            status: Some("b_fail"),
            pool_side: side_pool.len(),
            left_gap,
            right_gap,
            shape,
            parent_turn: parent.turn_source,
            parent_b: parent.interval_b,
        };
        let replace = match &best {
            None => true,
            Some((v, _)) => violation < *v,
        };
        if replace {
            best = Some((violation, diag));
        }
    }
    best.map(|(_, d)| d).unwrap_or(EdgeDiag {
        status: Some("b_fail"),
        pool_side: 0,
        ..EdgeDiag::default()
    })
}

/// CKPT/CERT dump 解析（只读）：证书身份向量集合 + 基例身份集合。
#[derive(Debug, Default)]
struct DumpBook {
    cert_lines: BTreeSet<String>,
    cert_ids_all: BTreeSet<String>,
    ckpt_ids_all: BTreeSet<String>,
    base_ids_b_exec1: BTreeSet<String>,
    base_ids_any_exec1: BTreeSet<String>,
}

fn parse_dump(path: &str) -> Result<DumpBook, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 dump 失败: {e}"))?;
    let mut book = DumpBook::default();
    for line in text.lines() {
        let is_cert = line.starts_with("CERT ");
        let is_ckpt = line.starts_with("CKPT ");
        if !is_cert && !is_ckpt {
            continue;
        }
        let mut caliber = "";
        let mut exec = "";
        let mut top = "";
        let mut side = "";
        let mut ids = "";
        for field in line.split_whitespace() {
            if let Some(value) = field.strip_prefix("caliber=") {
                caliber = value;
            } else if let Some(value) = field.strip_prefix("exec=") {
                exec = value;
            } else if let Some(value) = field.strip_prefix("top=") {
                top = value;
            } else if let Some(value) = field.strip_prefix("side=") {
                side = value;
            } else if let Some(value) = field.strip_prefix("ids=") {
                ids = value;
            }
        }
        if ids.is_empty() {
            continue;
        }
        if is_cert {
            book.cert_lines.insert(format!(
                "caliber={caliber} exec={exec} top={top} side={side} ids={ids}"
            ));
            book.cert_ids_all.insert(ids.to_string());
            if exec == "1" {
                if let Some(base) = ids.rsplit('|').next() {
                    book.base_ids_any_exec1.insert(base.to_string());
                    if caliber == "B" {
                        book.base_ids_b_exec1.insert(base.to_string());
                    }
                }
            }
        } else {
            book.ckpt_ids_all.insert(ids.to_string());
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

/// 终态快照双口径证书复现（与 p92 observe_snapshot / p102 同管线同 dedup 键）。
fn reproduce_certs(
    events: &[Vec<NestCandidateEvent>],
    classification: &Classification,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for caliber in [NestIntervalCaliber::A, NestIntervalCaliber::B] {
        let mut seen = BTreeSet::new();
        for exec in 1..events.len() {
            for top in exec..events.len() {
                for cert in assemble_typed_certificates(events, exec, top, caliber, |e| {
                    terminal_bits_new(classification, e)
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
                    out.insert(format!(
                        "caliber={caliber:?} exec={exec} top={top} side={:?} ids={ids}",
                        cert.certificate().side()
                    ));
                }
            }
        }
    }
    out
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or(
        "用法: p109_chain157_gate_attrition <btc_1m_full.json> [p92_ckpt_dump.txt]",
    )?;
    let dump_path = args.next();
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P109_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    let bars = &loaded.bars[..max_bars];
    let as_of = bars.len() - 1;
    println!(
        "P109_INPUT bars={} replay_bars={} as_of={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        as_of,
        loaded.first_date,
        loaded.last_date
    );
    let layer = parse_layer(bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let hist = compute_macd(&closes, &config.macd).hist;
    let dif = compute_macd(&closes, &config.macd).dif;
    println!(
        "P109_INPUT merged={} tower_levels={} classification_levels={}",
        layer.merged_bars.len(),
        tower.len(),
        classification.levels.len()
    );

    // ── V3（现行生产）分级测量：严格 pair + provider 事件 ──
    let mut levels_v3: Vec<LevelData> = Vec::new();
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
            true,
            true,
        )?;
        let trend = data
            .events
            .iter()
            .filter(|e| e.kind == newchan_rust::theta_v0::classifier::level_view::NestDivergenceKind::Trend)
            .count();
        let confirmed = data.events.iter().filter(|e| e.divergence_confirmed).count();
        println!(
            "P109_LEVEL_V3 L{level} tower_windows={} runs={} invalid={} moves={} completed={} strict_pairs={} events={} trend_events={} divergence_confirmed={}",
            data.tower_windows,
            data.runs,
            data.invalid_windows,
            data.moves,
            data.completed,
            data.pairs.len(),
            data.events.len(),
            trend,
            confirmed
        );
        levels_v3.push(data);
    }
    // ── V2（#83 历史基准锚，只测 pair/链数）──
    let mut levels_v2: Vec<LevelData> = Vec::new();
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
            false,
            false,
        )?;
        println!(
            "P109_LEVEL_V2 L{level} runs={} invalid={} moves={} completed={} strict_pairs={}",
            data.runs,
            data.invalid_windows,
            data.moves,
            data.completed,
            data.pairs.len()
        );
        levels_v2.push(data);
    }

    // ── 硬锚 ①：provider 事件总数复现 P92_YIELD ──
    let total_events: usize = levels_v3.iter().map(|l| l.events.len()).sum();
    let total_confirmed: usize = levels_v3
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.divergence_confirmed)
        .count();
    let trend_events: usize = levels_v3
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| e.kind == newchan_rust::theta_v0::classifier::level_view::NestDivergenceKind::Trend)
        .count();
    let trend_confirmed: usize = levels_v3
        .iter()
        .flat_map(|l| l.events.iter())
        .filter(|e| {
            e.divergence_confirmed
                && e.kind == newchan_rust::theta_v0::classifier::level_view::NestDivergenceKind::Trend
        })
        .count();
    println!(
        "P109_ANCHOR_EVENTS candidates={} trend={} pan={} divergence_confirmed={} trend_confirmed={} pan_confirmed={} expected_candidates=3683 expected_confirmed=1235 expected_trend_confirmed=429",
        total_events,
        trend_events,
        total_events - trend_events,
        total_confirmed,
        trend_confirmed,
        total_confirmed - trend_confirmed
    );

    // ── 硬锚 ②：终态快照证书集复现 dump 66 行 CERT ──
    // assemble_typed_certificates 的 events 按真实级别索引（0 级空槽）。
    let mut events_by_level: Vec<Vec<NestCandidateEvent>> = vec![Vec::new()];
    events_by_level.extend(levels_v3.iter().map(|l| l.events.clone()));
    let certs = reproduce_certs(&events_by_level, &classification);
    let dump = match &dump_path {
        Some(p) => Some(parse_dump(p)?),
        None => None,
    };
    if let Some(book) = &dump {
        let missing: Vec<_> = book.cert_lines.difference(&certs).collect();
        let extra: Vec<_> = certs.difference(&book.cert_lines).collect();
        println!(
            "P109_ANCHOR_CERT reproduced={} dump={} missing_in_repro={} extra_in_repro={}",
            certs.len(),
            book.cert_lines.len(),
            missing.len(),
            extra.len()
        );
        for line in missing.iter().take(10) {
            println!("P109_ANCHOR_CERT_MISSING {line}");
        }
        for line in extra.iter().take(10) {
            println!("P109_ANCHOR_CERT_EXTRA {line}");
        }
    } else {
        println!("P109_ANCHOR_CERT reproduced={} dump=NA", certs.len());
    }

    // ── 链计数（V2/V3 双口径）与全深度链枚举（V3）──
    let (complete_v2, depth_v2, depth_count_v2) = chain_dp_count(&levels_v2);
    let (complete_v3, depth_v3, depth_count_v3) = chain_dp_count(&levels_v3);
    println!(
        "P109_NEST_V2 complete_chains={} max_depth={} max_depth_count={} note=V2 投影+#105修复后代码（非 #83 原值 157）",
        complete_v2, depth_v2, depth_count_v2
    );
    println!(
        "P109_NEST_V3 complete_chains={} max_depth={} max_depth_count={}",
        complete_v3, depth_v3, depth_count_v3
    );
    let (chains, truncated) = enumerate_chains(&levels_v3, 500_000);
    println!(
        "P109_ENUM chains_enumerated={} truncated={} dp_complete={}",
        chains.len(),
        truncated,
        complete_v3
    );

    // ── 每级 interval_a → 事件下标 匹配表 ──
    let match_maps: Vec<BTreeMap<(usize, usize), Vec<usize>>> = levels_v3
        .iter()
        .map(|level| {
            let mut map: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
            for (index, event) in level.events.iter().enumerate() {
                map.entry(event.interval_a).or_default().push(index);
            }
            map
        })
        .collect();
    // 匹配体检：严格 pair 的 interval_a 精确命中率（身份门可信性自检）。
    for (offset, level) in levels_v3.iter().enumerate() {
        let mut hit = 0usize;
        let mut near = 0usize;
        for pair in &level.pairs {
            if match_maps[offset].contains_key(&(pair.start, pair.end)) {
                hit += 1;
            } else {
                let anchored = level.events.iter().any(|e| {
                    e.interval_a.0 == pair.start || e.interval_a.1 == pair.end
                });
                near += usize::from(anchored);
            }
        }
        println!(
            "P109_MATCH L{} pairs={} exact_event_hit={} miss={} miss_with_endpoint_anchor={}",
            offset + 1,
            level.pairs.len(),
            hit,
            level.pairs.len() - hit,
            near
        );
    }

    // ── 逐链对账 ──
    let mut outcomes: Vec<ChainOutcome> = Vec::with_capacity(chains.len());
    let mut edge_diags: Vec<(usize, usize, EdgeDiag)> = Vec::new();
    let mut invariant_violations = 0usize;
    for (id, chain) in chains.iter().enumerate() {
        outcomes.push(reconcile_chain(
            id,
            chain,
            &levels_v3,
            &match_maps,
            &classification,
            &mut edge_diags,
            &mut invariant_violations,
        ));
    }
    println!(
        "P109_INVARIANT a_edge_violations={}（期望 0：匹配事件 interval_a==pair 区间时 A 边构造性成立）",
        invariant_violations
    );

    // ── 时钟门裁决与前缀证书对账（dump 存在时）──
    if let Some(book) = &dump {
        for outcome in &mut outcomes {
            if outcome.canonical.is_empty() {
                continue;
            }
            let base = outcome.canonical.first().expect("nonempty canonical");
            let base_id = format!("{}:{}:{}-{}", base.0, base.1, base.2 .0, base.2 .1);
            outcome.base_in_66 = Some(book.base_ids_any_exec1.contains(&base_id));
            if outcome.prefix_cert_depth >= 1 && outcome.max_depth >= outcome.prefix_cert_depth {
                let prefix: Vec<(u32, usize, (usize, usize))> = outcome
                    .canonical
                    .iter()
                    .take(outcome.prefix_cert_depth)
                    .copied()
                    .collect();
                if prefix.len() == outcome.prefix_cert_depth {
                    let ids = ids_string(&prefix);
                    outcome.prefix_cert_in_dump = Some(
                        book.cert_ids_all.contains(&ids) || book.ckpt_ids_all.contains(&ids),
                    );
                }
            }
            if outcome.kill_gate == KillGate::Alive {
                let ids = ids_string(&outcome.canonical);
                let in_cert = book.cert_ids_all.contains(&ids);
                let in_ckpt = book.ckpt_ids_all.contains(&ids);
                let base_seen = book.base_ids_any_exec1.contains(&base_id);
                outcome.clock_witness = Some(match (in_cert, in_ckpt, base_seen) {
                    (true, _, _) => format!("witnessed cert_in_dump ids={ids}"),
                    (false, true, _) => format!("witnessed ckpt_only ids={ids}"),
                    (false, false, true) => {
                        format!("ghost_or_migrated base_seen_but_chain_absent ids={ids}")
                    }
                    (false, false, false) => format!("never_witnessed ids={ids}"),
                });
                if !in_cert && !in_ckpt {
                    outcome.kill_gate = KillGate::Clock;
                }
            }
        }
    }

    // ── 门级 attrition 汇总 ──
    let total = outcomes.len();
    let mut gate_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut gate_level_counts: BTreeMap<(&'static str, usize), usize> = BTreeMap::new();
    for outcome in &outcomes {
        *gate_counts.entry(outcome.kill_gate.label()).or_default() += 1;
        *gate_level_counts
            .entry((outcome.kill_gate.label(), outcome.kill_level))
            .or_default() += 1;
    }
    for (gate, count) in &gate_counts {
        let pct = if total == 0 {
            0.0
        } else {
            *count as f64 * 100.0 / total as f64
        };
        println!("P109_GATE gate={gate} count={count} pct={pct:.2} total={total}");
    }
    for ((gate, level), count) in &gate_level_counts {
        println!("P109_GATE_LEVEL gate={gate} kill_level={level} count={count}");
    }
    // 身份门细分：被击杀链的 leave kind / via_divergence 分布。
    let mut id_trend = 0usize;
    let mut id_cons = 0usize;
    let mut id_trend_via_div = 0usize;
    let mut div_pan = 0usize;
    let mut div_trend = 0usize;
    let mut term_any_bsp = 0usize;
    let mut term_no_bsp = 0usize;
    for outcome in &outcomes {
        match outcome.kill_gate {
            KillGate::Identity => {
                let p = &outcome.pairs[outcome.kill_level - 1];
                match p.leave_kind {
                    MoveKind::Trend => {
                        id_trend += 1;
                        id_trend_via_div += usize::from(p.leave_via_divergence);
                    }
                    MoveKind::Consolidation => id_cons += 1,
                }
            }
            KillGate::Divergence => {
                let p = &outcome.pairs[0];
                match p.leave_kind {
                    MoveKind::Trend => div_trend += 1,
                    MoveKind::Consolidation => div_pan += 1,
                }
            }
            KillGate::Terminal => {
                if outcome.kill_detail.contains("any_bsp_at_turn=true") {
                    term_any_bsp += 1;
                } else {
                    term_no_bsp += 1;
                }
            }
            _ => {}
        }
    }
    println!(
        "P109_GATE_SUB identity leave_trend={} （其中经背驰完成={}） leave_consolidation={}",
        id_trend, id_trend_via_div, id_cons
    );
    println!(
        "P109_GATE_SUB divergence base_trend={} base_pan={}（Trend 基例构造性不可能死于此门）",
        div_trend, div_pan
    );
    let term_trend = outcomes
        .iter()
        .filter(|o| o.kill_gate == KillGate::Terminal && o.div_base_trend > 0)
        .count();
    let term_all_pan = outcomes
        .iter()
        .filter(|o| o.kill_gate == KillGate::Terminal && o.div_base_trend == 0)
        .count();
    println!(
        "P109_GATE_SUB terminal any_bsp_at_turn={} no_bsp_at_turn={} has_trend_divergent_base={} all_pan_bases={}",
        term_any_bsp, term_no_bsp, term_trend, term_all_pan
    );
    // 包含门边几何汇总。
    let mut shape_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (_, _, diag) in &edge_diags {
        *shape_counts.entry(diag.shape).or_default() += 1;
    }
    for (shape, count) in &shape_counts {
        println!("P109_EDGE_SHAPE shape={shape} count={count}");
    }
    for (id, level, diag) in &edge_diags {
        println!(
            "P109_EDGE_GAP chain={id} edge_to_L{level} pool_side={} parent_turn={} parent_b=({}, {}) left_gap={} right_gap={} shape={}",
            diag.pool_side, diag.parent_turn, diag.parent_b.0, diag.parent_b.1, diag.left_gap, diag.right_gap, diag.shape
        );
    }

    // ── 每门代表样本（各取前 5 条链）──
    for gate in [
        KillGate::Identity,
        KillGate::Divergence,
        KillGate::Terminal,
        KillGate::Direction,
        KillGate::Inclusion,
        KillGate::Clock,
        KillGate::Alive,
    ] {
        let mut shown = 0usize;
        for outcome in &outcomes {
            if outcome.kill_gate != gate {
                continue;
            }
            if shown >= 5 {
                break;
            }
            shown += 1;
            let pairs: Vec<String> = outcome
                .pairs
                .iter()
                .enumerate()
                .map(|(k, p)| format!("L{}=({},{})", k + 1, p.start, p.end))
                .collect();
            println!(
                "P109_SAMPLE gate={} chain={} kill_level={} pairs=[{}] detail={}",
                gate.label(),
                outcome.id,
                outcome.kill_level,
                pairs.join(" "),
                outcome.kill_detail
            );
        }
    }

    // ── 交叉对账：基例入 66 证书身份、前缀证书 dump 在场 ──
    let passing_terminal = outcomes
        .iter()
        .filter(|o| o.prefix_cert_depth >= 1)
        .count();
    let base_seen = outcomes
        .iter()
        .filter(|o| o.base_in_66 == Some(true))
        .count();
    let prefix_seen = outcomes
        .iter()
        .filter(|o| o.prefix_cert_in_dump == Some(true))
        .count();
    let prefix_missed = outcomes
        .iter()
        .filter(|o| o.prefix_cert_in_dump == Some(false))
        .count();
    println!(
        "P109_CROSS chains={} pass_terminal_gate={} base_in_66={} prefix_cert_in_dump={} prefix_cert_absent={}",
        total, passing_terminal, base_seen, prefix_seen, prefix_missed
    );
    // 前缀证书深度谱（exec=1 的 (1,m) 逻辑证书可达深度）。
    let mut prefix_spectrum: BTreeMap<usize, usize> = BTreeMap::new();
    for outcome in &outcomes {
        *prefix_spectrum.entry(outcome.prefix_cert_depth).or_default() += 1;
    }
    for (depth, count) in &prefix_spectrum {
        println!("P109_PREFIX_DEPTH depth={depth} chains={count}");
    }

    // ── 全量逐链表 ──
    for outcome in &outcomes {
        let pairs: Vec<String> = outcome
            .pairs
            .iter()
            .enumerate()
            .map(|(k, p)| format!("L{}={}-{}", k + 1, p.start, p.end))
            .collect();
        let evs: Vec<String> = outcome
            .events_per_level
            .iter()
            .map(|v| v.to_string())
            .collect();
        let ids = if outcome.canonical.is_empty() {
            "-".to_string()
        } else {
            ids_string(&outcome.canonical)
        };
        println!(
            "P109_CHAIN id={} kill={}@{} max_depth={} prefix_depth={} side={:?} paths={} events=[{}] pairs=[{}] ids={} witness={}",
            outcome.id,
            outcome.kill_gate.label(),
            outcome.kill_level,
            outcome.max_depth,
            outcome.prefix_cert_depth,
            outcome.base_side,
            outcome.feasible_full_paths,
            evs.join(","),
            pairs.join(" "),
            ids,
            outcome.clock_witness.clone().unwrap_or_else(|| "NA".to_string())
        );
        println!("P109_CHAIN_DETAIL id={} detail={}", outcome.id, outcome.kill_detail);
    }
    println!("P109_DONE chains={total}");
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
