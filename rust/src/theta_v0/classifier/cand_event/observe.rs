//! #634 拆自单文件 `classifier/cand_event.rs` 的**观察产出**域。
//!
//! 单次塔扫描 → [`CandidateObservation`] 的投影侧：结构域（趋势背驰段候选，经
//! [`super::super::signal::first_structural_gates`] 结构门）与 Pan 域（既有 `judge_pan_div`
//! 证书投影）。本文件不持状态、不判修订——观察进事件簿的协议在 [`super::book`]，
//! 身份键与谓词类型在 [`super::key`]。

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use super::super::super::types::{Center, Direction, Segment};
use super::super::decompose::center_trend_gate;
use super::super::divergence::{
    departure_move_c_start, locate_departure_move_a, move_range_envelope,
};
use super::super::signal;
use super::key::{
    CandidateKey, CandidateKind, ObservedState, ParentFingerprint, StructuralPredicates,
    CANDIDATE_RULE_VERSION, FNV_OFFSET_BASIS, FNV_PRIME,
};

const PAIR_ID_SEED: u64 = 0x84222325cbf29ce4;

/// 单次塔扫描给事件机的业务投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateObservation {
    pub key: CandidateKey,
    pub kind: CandidateKind,
    pub center_ids: Option<(usize, usize)>,
    /// SPEC E2E-D2 占位：当前是 key 的 FNV 种子哈希，无独立业务信息。
    pub candidate_group_id: u64,
    /// SPEC E2E-D2 占位：当前是 key 的另一种子哈希，无独立业务信息。
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    /// SPEC E2E-D3 占位：当前恒为 `key.seg_a` 的副本，无独立业务信息。
    pub extreme_proof: (usize, usize),
    pub third_class_proof: Option<usize>,
    pub interval: (usize, usize),
    pub state: ObservedState,
    /// 本次观察若使结构宽候选完全成立，则为该次的结构位；未决观察为 `None`。
    /// 事件簿只在首证钟尚空时采纳它（一次写入不后移）。
    pub first_provable_at: Option<usize>,
    pub confirmed_at: Option<usize>,
}

/// 同一次分类迭代内的结构宽候选投影。
///
/// D1-D3 的唯一判定点是 [`signal::first_structural_gates`]：调用方传入本级循环现成的中枢、
/// 块序列所决定的趋势方向、A 段、λ_C 与坐标序列。本函数只把门投影为事件观察；
/// `BspPoint.bits`（力度确认）与 `force` 均不读取。
///
/// ★同 episode 归约（#551）：同一 key 可由**同一 episode 内的多条同向腿**各产一次门结果
/// （λ_C 与 seg_a 相同、只右端不同）。它们是同一候选在**同一时刻**的多份证据，不是时间序列——
/// 必须先归约成每 key 一条观察再入事件簿，否则同 as_of 重跑时靠前的短区间会被误判为区间回缩，
/// 破坏幂等（US8）。归约口径见 [`merged`]。
pub(crate) fn structural_observations_for_level(
    level: u32,
    centers: &[Center],
    blocks: &[super::super::decompose::MoveBlock],
    segments: &[Segment],
    anchor_dirs: &[Option<Direction>],
    close_src: &[usize],
) -> Vec<CandidateObservation> {
    let center_gate = center_trend_gate(centers.len(), blocks);
    let context = StructuralScan {
        level,
        centers,
        segments,
        anchor_dirs,
        close_src,
    };
    let mut by_key: BTreeMap<CandidateKey, CandidateObservation> = BTreeMap::new();
    for (i, segment) in segments.iter().enumerate() {
        let Some(leg) = structural_observation(&context, &center_gate, i, segment) else {
            continue;
        };
        match by_key.entry(leg.key) {
            Entry::Vacant(slot) => {
                slot.insert(leg);
            }
            Entry::Occupied(mut slot) => {
                let reduced = merged(slot.get(), &leg);
                slot.insert(reduced);
            }
        }
    }
    let mut observations: Vec<_> = by_key.into_values().collect();
    observations.sort_by_key(|observation| (observation.interval, observation.key));
    observations
}

/// 由累积观察 `acc` 与同 key 的后一条 episode 腿 `leg` 归约出**新的**累积观察
/// （腿按段序到达 ⟹ `leg.interval.1` 单调不减）。两个入参均只读，不被修改。
///
/// 承载体（除下述字段外的全部字段）取右端更新的那一条：`leg.interval.1 > acc.interval.1`
/// 时为 `leg`，否则为 `acc`。在此之上逐字段归约：
///
/// - `interval`：右端取最新腿的端点——I(C) = [λ_C, 当前 episode 最新同向段端点]（Q5 区间口径）。
/// - `extreme`：取**析取**（两条的析取，独立于承载体的选择）。037:20 判的是 I(C) 包络破 b 包络，
///   而「I(C) 包络首次破 ⟺ 某同向段端点首次越界」（`signal` 函数头等价性注记）⟹ episode 内
///   任一腿破极值即整个 I(C) 已破，该事实不因后续腿端点回撤而消失。
/// - `comparable`：取最新腿的值——A/C 可比较性由当前 I(C) 区间的坐标可映射性决定，
///   故随承载体走。
/// - `state`：由归约后的谓词组重新派生（[`StructuralPredicates::resolved_state`] 是单一来源）。
/// - `first_provable_at`：取**首个**使全谓词成立的腿的结构位，不被后续腿改写（首证钟语义）；
///   `acc` 的首证钟优先于 `leg` 的，两者皆空且归约后成立时落在承载体的区间右端。
fn merged(acc: &CandidateObservation, leg: &CandidateObservation) -> CandidateObservation {
    let extreme = acc.structural_predicates.extreme || leg.structural_predicates.extreme;
    let carrier = if leg.interval.1 > acc.interval.1 {
        leg
    } else {
        acc
    };
    let structural_predicates = StructuralPredicates {
        extreme,
        ..carrier.structural_predicates
    };
    let state = structural_predicates.resolved_state();
    let first_provable_at = match state {
        ObservedState::Provisional => acc
            .first_provable_at
            .or(leg.first_provable_at)
            .or(Some(carrier.interval.1)),
        _ => acc.first_provable_at.or(leg.first_provable_at),
    };
    CandidateObservation {
        structural_predicates,
        state,
        first_provable_at,
        ..carrier.clone()
    }
}

struct StructuralScan<'a> {
    level: u32,
    centers: &'a [Center],
    segments: &'a [Segment],
    anchor_dirs: &'a [Option<Direction>],
    close_src: &'a [usize],
}

fn structural_observation(
    scan: &StructuralScan<'_>,
    center_gate: &[Option<Direction>],
    i: usize,
    segment: &Segment,
) -> Option<CandidateObservation> {
    let c_idx = signal::nearest_confirmed_center_idx(scan.centers, segment.start_index)?;
    let direction = center_gate.get(c_idx).copied().flatten()?;
    let previous = scan.centers.get(c_idx.checked_sub(1)?)?;
    let parent = &scan.centers[c_idx];
    let a = locate_departure_move_a(scan.segments, scan.anchor_dirs, previous, parent, direction)
        .and_then(|span| move_range_envelope(scan.segments, span).map(|range| (span, range)));
    let c_start = departure_move_c_start(
        scan.segments,
        scan.anchor_dirs,
        parent,
        direction,
        segment.start_index,
    );
    let gates = signal::first_structural_gates(
        parent,
        direction,
        segment,
        scan.anchor_dirs[i],
        scan.close_src,
        a,
        c_start,
    )?;
    Some(trend_observation(
        scan.level, previous, parent, segment, &gates,
    ))
}

fn trend_observation(
    level: u32,
    previous: &Center,
    parent_center: &Center,
    segment: &Segment,
    gates: &signal::FirstStructuralGates,
) -> CandidateObservation {
    let key = CandidateKey {
        rule_version: CANDIDATE_RULE_VERSION,
        level,
        kind: CandidateKind::Trend,
        side: gates.side,
        previous_center_start: Some(previous.start_index),
        parent: ParentFingerprint {
            center_start: parent_center.start_index,
            zd: parent_center.zd,
            zg: parent_center.zg,
        },
        seg_a: gates.seg_a,
        c_start: gates.lambda_c,
    };
    let structural_predicates = StructuralPredicates {
        // 门存在 ⟹ 破中枢 ∧ 破向 = 趋势向已成立（不成立时 `first_structural_gates` 返回 None）。
        direction: true,
        comparable: gates.comparable(),
        extreme: gates.extreme,
    };
    let state = structural_predicates.resolved_state();
    CandidateObservation {
        key,
        kind: CandidateKind::Trend,
        center_ids: Some((previous.start_index, parent_center.start_index)),
        candidate_group_id: stable_id(&key, FNV_OFFSET_BASIS),
        pair_id: stable_id(&key, PAIR_ID_SEED),
        structural_predicates,
        extreme_proof: gates.seg_a,
        third_class_proof: None,
        interval: (gates.lambda_c, segment.end_index),
        state,
        // 首证钟只在本次观察使结构宽候选完全成立时才有值——未决观察不提前落钟。
        first_provable_at: (state == ObservedState::Provisional).then_some(segment.end_index),
        confirmed_at: None,
    }
}

/// 将既有 `judge_pan_div` 唯一构造的证书投影为 Pan 域确认候选，不重判结构或力度。
pub(crate) fn pan_observations_for_level(
    level: u32,
    certs: &[signal::PanDivCert],
) -> Vec<CandidateObservation> {
    certs
        .iter()
        .map(|cert| {
            let parent = ParentFingerprint {
                center_start: cert.center.start_index,
                zd: cert.center.zd,
                zg: cert.center.zg,
            };
            let key = CandidateKey {
                rule_version: CANDIDATE_RULE_VERSION,
                level,
                kind: CandidateKind::Pan,
                side: cert.side,
                previous_center_start: None,
                parent,
                seg_a: cert.seg_a,
                c_start: cert.seg_c.0,
            };
            CandidateObservation {
                key,
                kind: CandidateKind::Pan,
                center_ids: None,
                candidate_group_id: stable_id(&key, FNV_OFFSET_BASIS),
                pair_id: stable_id(&key, PAIR_ID_SEED),
                structural_predicates: StructuralPredicates {
                    direction: true,
                    comparable: true,
                    extreme: true,
                },
                extreme_proof: cert.seg_a,
                third_class_proof: None,
                interval: cert.seg_c,
                state: ObservedState::Confirmed,
                first_provable_at: Some(cert.source_index),
                confirmed_at: None,
            }
        })
        .collect()
}

/// 单级双域候选的统一接线点，供全量与增量分类循环共同调用。
///
/// ★#551：入参不含 `hist`/`dif`/`closes_tick`/`gauge`——候选产出在**类型层**够不到 MACD 力度
/// 序列（力度只经既有 BSP 路径影响 `BspPoint.bits`，不进候选身份、状态与任一事件字段）。
/// `close_src` 仍需要，仅用于 `comparable` 谓词的坐标可映射性判定（不读价格/面积）。
pub(crate) fn observations_for_level(
    level: u32,
    centers: &[Center],
    blocks: &[super::super::decompose::MoveBlock],
    segments: &[Segment],
    anchor_dirs: &[Option<Direction>],
    close_src: &[usize],
    pan_div: &[signal::PanDivCert],
) -> Vec<CandidateObservation> {
    let mut observations =
        structural_observations_for_level(level, centers, blocks, segments, anchor_dirs, close_src);
    observations.extend(pan_observations_for_level(level, pan_div));
    observations.sort_by_key(|observation| (observation.interval, observation.key));
    observations
}

fn stable_id(key: &CandidateKey, seed: u64) -> u64 {
    let mut hash = seed;
    for value in [
        key.level as u64,
        key.kind as u64,
        key.side as u64,
        key.previous_center_start.is_some() as u64,
        key.previous_center_start.unwrap_or_default() as u64,
        key.parent.center_start as u64,
        key.parent.zd as u64,
        key.parent.zg as u64,
        key.seg_a.0 as u64,
        key.seg_a.1 as u64,
        key.c_start as u64,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{
        center, deferred_envelope_break_scan, scan_observations, segment,
    };
    use super::*;
    use crate::theta_v0::classifier::decompose::decompose;
    use crate::theta_v0::types::Side;

    #[test]
    fn structural_wide_candidate_does_not_require_divergence_strength() {
        let centers = [center(300, 400, 2), center(100, 200, 8)];
        let segments = [
            segment(Direction::Down, 3, 5, 350, 250),
            segment(Direction::Up, 5, 7, 250, 280),
            segment(Direction::Down, 9, 11, 150, 80),
        ];
        let anchors: Vec<Option<Direction>> = segments
            .iter()
            .map(|segment| Some(segment.direction))
            .collect();
        let close_src: Vec<usize> = (0..12).collect();
        let observations = structural_observations_for_level(
            0,
            &centers,
            &decompose(&centers),
            &segments,
            &anchors,
            &close_src,
        );
        assert_eq!(
            observations.len(),
            1,
            "结构门成立即产候选，力度 C<A=false 不得拦截"
        );
        assert_eq!(observations[0].key.side, Side::Long);
        assert_eq!(observations[0].first_provable_at, Some(11));
        assert_eq!(observations[0].state, ObservedState::Provisional);
        assert!(observations[0].structural_predicates.all_hold());
    }

    #[test]
    fn breaking_core_without_envelope_extreme_is_unresolved_not_absent() {
        // ★#551 US「∅→Unresolved」生产可达锁：破中枢但未破 b 包络极值 ⟹ 候选身份成立（进候选域）
        // 而结构未决——不是缺席、也不是 Provisional。反事实（#550 口径）：judge 返回 None ⟹ 零观察。
        let (centers, mut segments, mut anchors, close_src) = deferred_envelope_break_scan();
        segments.truncate(3); // 只保留 A / B / C 腿1（未破极值）
        anchors.truncate(3);
        let observations = scan_observations(&centers, &segments, &anchors, &close_src);
        assert_eq!(
            observations.len(),
            1,
            "破核心段必产候选观察（结构宽候选域）"
        );
        let observation = &observations[0];
        assert_eq!(observation.state, ObservedState::Unresolved);
        assert!(observation.structural_predicates.direction, "破中枢门成立");
        assert!(
            observation.structural_predicates.comparable,
            "A/C 区间可映射"
        );
        assert!(
            !observation.structural_predicates.extreme,
            "037:20 未破包络极值"
        );
        assert_eq!(
            observation.first_provable_at, None,
            "未决观察不落首证钟（禁提前写钟）"
        );
        assert_eq!(observation.interval, (9, 11));
    }

    #[test]
    fn same_episode_legs_reduce_to_one_observation_per_key() {
        // ★#551 同 episode 归约锁：腿1（未破极值，右端 11）与腿2（破极值，右端 15）共享 key ⟹
        // 一次扫描只出一条观察。I(C) 右端取最新腿；extreme 取析取（I(C) 包络已破的事实不回退）。
        let (centers, segments, anchors, close_src) = deferred_envelope_break_scan();
        let observations = scan_observations(&centers, &segments, &anchors, &close_src);
        assert_eq!(observations.len(), 1, "同 key 同扫描归约为一条观察");
        assert_eq!(observations[0].interval, (9, 15), "I(C) 右端 = 最新腿端点");
        assert!(
            observations[0].structural_predicates.extreme,
            "腿2 破极值 ⟹ I(C) 已破"
        );
        assert_eq!(observations[0].state, ObservedState::Provisional);
        assert_eq!(
            observations[0].first_provable_at,
            Some(15),
            "首证钟 = 首个使全谓词成立的腿的结构位"
        );
    }
}
