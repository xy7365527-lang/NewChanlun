//! #634 拆分后三域测试共用的夹具（原为单文件 `cand_event.rs` 内 `mod tests` 的私有夹具，
//! 逐字移入，无口径改动）。
//!
//! 只在 `#[cfg(test)]` 下编译；夹具本身不带 `#[test]`，不进测试名清单。

use super::super::super::types::{Center, Direction, Segment, Side, Tick};
use super::super::decompose::decompose;
use super::key::{
    CandidateKey, CandidateKind, CandidateState, ParentFingerprint, StructuralPredicates,
    CANDIDATE_RULE_VERSION,
};
use super::observe::{structural_observations_for_level, CandidateObservation};

pub(super) fn center(zd: Tick, zg: Tick, end_index: usize) -> Center {
    Center {
        zd,
        zg,
        dd: zd - 10,
        gg: zg + 10,
        start_index: end_index.saturating_sub(2),
        end_index,
    }
}

pub(super) fn segment(
    direction: Direction,
    start: usize,
    end: usize,
    start_price: Tick,
    end_price: Tick,
) -> Segment {
    Segment {
        direction,
        start_index: start,
        end_index: end,
        start_price,
        end_price,
    }
}

pub(super) fn observation(interval: (usize, usize), state: CandidateState) -> CandidateObservation {
    let key = CandidateKey {
        rule_version: CANDIDATE_RULE_VERSION,
        level: 0,
        kind: CandidateKind::Trend,
        side: Side::Long,
        previous_center_start: Some(10),
        parent: ParentFingerprint {
            center_start: 20,
            zd: 100,
            zg: 110,
        },
        seg_a: (11, 19),
        c_start: 30,
    };
    CandidateObservation {
        key,
        kind: CandidateKind::Trend,
        center_ids: Some((10, 20)),
        candidate_group_id: 1,
        pair_id: 2,
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: (11, 19),
        third_class_proof: None,
        interval,
        state,
        // 与生产口径一致：未决观察不带首证钟。
        first_provable_at: (state != CandidateState::Unresolved).then_some(35),
        confirmed_at: None,
    }
}

/// #551 状态机全谱共用夹具（几何取自 `signal::first_class_point_deferred_to_envelope_breaking_leg_03720`）。
///
/// A 腿 b_lo=90；C 腿1 @11 破核心（95 < zd=100）但未破包络极值（95 ≥ 90）；C 内反向段 @13
/// 端点 98 < zd 未回核心 ⟹ 与腿1 同一 episode（λ_C=9）；C 腿2 @15 端点 85 < 90 破极值。
/// 故腿1 与腿2 **共享同一 CandidateKey**（同 seg_a、同 λ_C、同中枢对），只在谓词与右端不同。
pub(super) fn deferred_envelope_break_scan() -> (
    [Center; 2],
    Vec<Segment>,
    Vec<Option<Direction>>,
    Vec<usize>,
) {
    let centers = [center(300, 400, 2), center(100, 200, 8)];
    let segments = vec![
        segment(Direction::Down, 3, 5, 350, 90),
        segment(Direction::Up, 5, 7, 90, 95),
        segment(Direction::Down, 9, 11, 150, 95),
        segment(Direction::Up, 11, 13, 95, 98),
        segment(Direction::Down, 13, 15, 98, 85),
    ];
    let anchors = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    (centers, segments, anchors, (0..16).collect())
}

pub(super) fn scan_observations(
    centers: &[Center],
    segments: &[Segment],
    anchors: &[Option<Direction>],
    close_src: &[usize],
) -> Vec<CandidateObservation> {
    structural_observations_for_level(
        0,
        centers,
        &decompose(centers),
        segments,
        anchors,
        close_src,
    )
}

/// growth_revision 的**生产路径**夹具：在 `deferred_envelope_break_scan` 之后再接同 episode
/// 的一对段（反向段端点 88 < zd=100 ⟹ 未回中枢核心 ⟹ 不开新 episode，λ_C 仍为 9），
/// 使同一 key 在两次扫描下都判 `Provisional` 而 I(C) 右端由 15 生长到 19。
pub(super) fn extended_episode_scan() -> (
    [Center; 2],
    Vec<Segment>,
    Vec<Option<Direction>>,
    Vec<usize>,
) {
    let (centers, mut segments, _, _) = deferred_envelope_break_scan();
    segments.push(segment(Direction::Up, 15, 17, 85, 88));
    segments.push(segment(Direction::Down, 17, 19, 88, 80));
    let anchors = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    (centers, segments, anchors, (0..20).collect())
}
