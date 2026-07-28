//! #550 塔内原生背驰段候选事件。
//!
//! 本模块只产出、存储候选生命史，不接 BSP/admission/订单消费点。候选事实在
//! `classify_impl` 的逐级循环内，由与 BSP 共用的 `signal::judge_first_cached` 结构判定产出；
//! 力度只影响 BSP bit，不进入候选身份或事件字段。

use std::collections::{BTreeMap, BTreeSet};

use super::super::types::{Center, Direction, Segment, Tick};
use super::decompose::{center_trend_gate, decompose};
use super::divergence::{
    departure_move_c_start, locate_departure_move_a, move_range_envelope, DivergenceGauge,
};
use super::signal;

/// 背驰段候选类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind {
    Trend,
    Pan,
}

/// Bₚ 的稳定结构指纹。中枢延伸的右端不参与身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParentFingerprint {
    pub center_start: usize,
    pub zd: i64,
    pub zg: i64,
}

/// 候选稳定身份。C 右端与 as_of 均不在键中。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateKey {
    pub level: u32,
    pub side_tag: u8,
    pub previous_center_start: usize,
    pub parent: ParentFingerprint,
    pub seg_a: (usize, usize),
    pub c_start: usize,
}

/// E2E-D5 候选状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateState {
    Provisional,
    Unresolved,
    Confirmed,
    Invalidated,
}

impl CandidateState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Confirmed | Self::Invalidated)
    }
}

/// 结构谓词证据；字段值来自同一次塔扫描，不在本模块重判。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralPredicates {
    pub direction: bool,
    pub comparable: bool,
    pub extreme: bool,
}

/// 一等候选事件的一次不可变 revision。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvent {
    pub key: CandidateKey,
    pub kind: CandidateKind,
    pub event_level: u32,
    pub center_ids: (usize, usize),
    pub candidate_group_id: u64,
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    pub extreme_proof: (usize, usize),
    pub third_class_proof: Option<usize>,
    /// C 段闭区间，source_index 坐标域。
    pub interval: (usize, usize),
    pub state: CandidateState,
    pub observed_at: usize,
    pub first_provable_at: usize,
    pub confirmed_at: Option<usize>,
    pub invalidated_at: Option<usize>,
    pub revision: u32,
    pub supersedes_revision: Option<u32>,
    pub revision_at: usize,
}

/// 每级一条 append-only 流。
pub type CandidateStreams = Vec<Vec<CandidateEvent>>;

/// 闭区间 C⊆C；端点相等（相切）算包含。
pub fn interval_is_sub(child: (usize, usize), parent: (usize, usize)) -> bool {
    child.0 <= child.1 && parent.0 <= parent.1 && parent.0 <= child.0 && child.1 <= parent.1
}

/// 跨级候选 C⊆C。坐标均为 source_index，无级别换算。
pub fn candidate_is_sub(child: &CandidateEvent, parent: &CandidateEvent) -> bool {
    child.event_level < parent.event_level && interval_is_sub(child.interval, parent.interval)
}

/// 单次塔扫描给事件机的业务投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateObservation {
    pub key: CandidateKey,
    pub kind: CandidateKind,
    pub center_ids: (usize, usize),
    pub candidate_group_id: u64,
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    pub extreme_proof: (usize, usize),
    pub third_class_proof: Option<usize>,
    pub interval: (usize, usize),
    pub state: CandidateState,
    pub first_provable_at: usize,
}

/// 每级 append-only 修订簿。终态不复活；同投影重跑零 Delta。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CandidateEventBook {
    streams: CandidateStreams,
    latest: BTreeMap<CandidateKey, (usize, usize)>,
}

impl CandidateEventBook {
    pub fn streams(&self) -> &CandidateStreams {
        &self.streams
    }

    pub fn advance(
        &mut self,
        observations: &[CandidateObservation],
        as_of: usize,
    ) -> Vec<CandidateEvent> {
        let mut delta = Vec::new();
        let mut seen = BTreeSet::new();
        for observation in observations {
            seen.insert(observation.key);
            let prior = self
                .latest
                .get(&observation.key)
                .copied()
                .and_then(|(level, index)| {
                    self.streams
                        .get(level)
                        .and_then(|stream| stream.get(index))
                        .cloned()
                });
            if prior
                .as_ref()
                .is_some_and(|event| observation.interval.1 <= event.interval.1)
            {
                continue;
            }
            if prior
                .as_ref()
                .is_some_and(|event| event.state.is_terminal())
            {
                continue;
            }
            let next = make_revision(prior.as_ref(), observation);
            if prior
                .as_ref()
                .is_some_and(|event| same_projection(event, &next))
            {
                continue;
            }
            self.append(next.clone());
            delta.push(next);
        }

        let active: Vec<CandidateKey> = self.latest.keys().copied().collect();
        for key in active {
            if seen.contains(&key) {
                continue;
            }
            let (level, index) = self.latest[&key];
            let prior = self.streams[level][index].clone();
            if prior.state.is_terminal() {
                continue;
            }
            let mut invalidated = prior.clone();
            invalidated.state = CandidateState::Invalidated;
            invalidated.invalidated_at = Some(as_of);
            invalidated.revision += 1;
            invalidated.supersedes_revision = Some(prior.revision);
            invalidated.revision_at = as_of;
            self.append(invalidated.clone());
            delta.push(invalidated);
        }
        delta
    }

    fn append(&mut self, event: CandidateEvent) {
        let level = event.event_level as usize;
        if self.streams.len() <= level {
            self.streams.resize_with(level + 1, Vec::new);
        }
        let index = self.streams[level].len();
        self.latest.insert(event.key, (level, index));
        self.streams[level].push(event);
    }
}

fn make_revision(
    prior: Option<&CandidateEvent>,
    observation: &CandidateObservation,
) -> CandidateEvent {
    let revision = prior.map_or(0, |event| event.revision + 1);
    let observed_at = prior.map_or(observation.first_provable_at, |event| event.observed_at);
    let first_provable_at = prior.map_or(observation.first_provable_at, |event| {
        event.first_provable_at
    });
    CandidateEvent {
        key: observation.key,
        kind: observation.kind,
        event_level: observation.key.level,
        center_ids: observation.center_ids,
        candidate_group_id: observation.candidate_group_id,
        pair_id: observation.pair_id,
        structural_predicates: observation.structural_predicates,
        extreme_proof: observation.extreme_proof,
        third_class_proof: observation.third_class_proof,
        interval: observation.interval,
        state: observation.state,
        observed_at,
        first_provable_at,
        confirmed_at: if observation.state == CandidateState::Confirmed {
            prior
                .and_then(|event| event.confirmed_at)
                .or(Some(observation.first_provable_at))
        } else {
            None
        },
        invalidated_at: None,
        revision,
        supersedes_revision: prior.map(|event| event.revision),
        revision_at: observation.interval.1,
    }
}

fn same_projection(a: &CandidateEvent, b: &CandidateEvent) -> bool {
    a.kind == b.kind
        && a.center_ids == b.center_ids
        && a.candidate_group_id == b.candidate_group_id
        && a.pair_id == b.pair_id
        && a.structural_predicates == b.structural_predicates
        && a.extreme_proof == b.extreme_proof
        && a.third_class_proof == b.third_class_proof
        && a.interval == b.interval
        && a.state == b.state
}

/// 同一次分类迭代内的结构宽候选投影。
///
/// D1-D3 的唯一判定点是 [`signal::judge_first_cached`]：调用方传入本级循环现成的中枢、
/// 块序列所决定的趋势方向、A 段、λ_C 与坐标序列。本函数只把 `Some` 投影为事件观察；
/// `BspPoint.bits`（力度确认）与 `force` 均不读取。
#[allow(clippy::too_many_arguments)]
pub(crate) fn structural_observations_for_level(
    level: u32,
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
) -> Vec<CandidateObservation> {
    let blocks = decompose(centers);
    let center_gate = center_trend_gate(centers.len(), &blocks);
    let mut observations = Vec::new();

    for (i, seg) in segments.iter().enumerate() {
        let Some(c_idx) = signal::nearest_confirmed_center_idx(centers, seg.start_index) else {
            continue;
        };
        let Some(direction) = center_gate.get(c_idx).copied().flatten() else {
            continue;
        };
        if c_idx == 0 {
            continue;
        }
        let previous = &centers[c_idx - 1];
        let parent_center = &centers[c_idx];
        let a = locate_departure_move_a(segments, anchor_dirs, previous, parent_center, direction)
            .and_then(|span| move_range_envelope(segments, span).map(|envelope| (span, envelope)));
        let lambda_c = departure_move_c_start(
            segments,
            anchor_dirs,
            parent_center,
            direction,
            seg.start_index,
        );
        let Some(point) = signal::judge_first_cached(
            parent_center,
            direction,
            seg,
            anchor_dirs[i],
            hist,
            dif,
            closes_tick,
            close_src,
            a,
            lambda_c,
            gauge,
        ) else {
            continue;
        };
        let seg_a = a.expect("judge Some => A 段映射成立").0;
        let c_start = lambda_c.expect("judge Some => lambda_C 成立");
        let side_tag = match point.struct_break_dir.expect("结构候选必有方向") {
            super::super::types::Side::Long => 0,
            super::super::types::Side::Short => 1,
        };
        let parent = ParentFingerprint {
            center_start: parent_center.start_index,
            zd: parent_center.zd,
            zg: parent_center.zg,
        };
        let key = CandidateKey {
            level,
            side_tag,
            previous_center_start: previous.start_index,
            parent,
            seg_a,
            c_start,
        };
        observations.push(CandidateObservation {
            key,
            kind: CandidateKind::Trend,
            center_ids: (previous.start_index, parent.center_start),
            candidate_group_id: stable_id(&key, 0xcbf29ce484222325),
            pair_id: stable_id(&key, 0x84222325cbf29ce4),
            structural_predicates: StructuralPredicates {
                direction: true,
                comparable: true,
                extreme: true,
            },
            extreme_proof: seg_a,
            third_class_proof: None,
            interval: (c_start, seg.end_index),
            state: CandidateState::Provisional,
            first_provable_at: seg.end_index,
        });
    }
    observations.sort_by_key(|observation| (observation.interval, observation.key));
    observations
}

fn stable_id(key: &CandidateKey, seed: u64) -> u64 {
    let mut hash = seed;
    for value in [
        key.level as u64,
        key.side_tag as u64,
        key.previous_center_start as u64,
        key.parent.center_start as u64,
        key.parent.zd as u64,
        key.parent.zg as u64,
        key.seg_a.0 as u64,
        key.seg_a.1 as u64,
        key.c_start as u64,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(zd: Tick, zg: Tick, end_index: usize) -> Center {
        Center {
            zd,
            zg,
            dd: zd - 10,
            gg: zg + 10,
            start_index: end_index.saturating_sub(2),
            end_index,
        }
    }

    fn segment(
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

    fn observation(interval: (usize, usize), state: CandidateState) -> CandidateObservation {
        let key = CandidateKey {
            level: 0,
            side_tag: 0,
            previous_center_start: 10,
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
            center_ids: (10, 20),
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
            first_provable_at: 35,
        }
    }

    #[test]
    fn closed_interval_sub_truth_table_includes_touching_endpoints() {
        assert!(interval_is_sub((10, 20), (10, 20)));
        assert!(interval_is_sub((10, 15), (10, 20)));
        assert!(interval_is_sub((15, 20), (10, 20)));
        assert!(interval_is_sub((11, 19), (10, 20)));
        assert!(!interval_is_sub((9, 20), (10, 20)));
        assert!(!interval_is_sub((10, 21), (10, 20)));
        assert!(!interval_is_sub((20, 10), (10, 20)));
    }

    #[test]
    fn append_only_revisions_keep_identity_and_clocks_and_same_as_of_is_idempotent() {
        let mut book = CandidateEventBook::default();
        let first = observation((30, 35), CandidateState::Provisional);
        assert_eq!(book.advance(std::slice::from_ref(&first), 35).len(), 1);
        assert!(book.advance(std::slice::from_ref(&first), 35).is_empty());

        let grown = observation((30, 40), CandidateState::Confirmed);
        assert_eq!(book.advance(std::slice::from_ref(&grown), 40).len(), 1);
        let stream = &book.streams()[0];
        assert_eq!(stream.len(), 2);
        assert_eq!(stream[0].key, stream[1].key);
        assert_eq!(stream[1].revision, 1);
        assert_eq!(stream[1].supersedes_revision, Some(0));
        assert_eq!(stream[1].observed_at, 35);
        assert_eq!(stream[1].first_provable_at, 35);
        assert_eq!(stream[1].confirmed_at, Some(35));
    }

    #[test]
    fn disappearance_appends_invalidation_and_terminal_never_revives() {
        let mut book = CandidateEventBook::default();
        let candidate = observation((30, 35), CandidateState::Unresolved);
        book.advance(std::slice::from_ref(&candidate), 35);
        let invalidated = book.advance(&[], 36);
        assert_eq!(invalidated[0].state, CandidateState::Invalidated);
        assert_eq!(invalidated[0].invalidated_at, Some(36));
        assert!(book.advance(&[candidate], 37).is_empty());
        assert_eq!(book.streams()[0].len(), 2);
    }

    #[test]
    fn candidate_event_stream_fnv1a_golden() {
        let mut book = CandidateEventBook::default();
        book.advance(&[observation((30, 35), CandidateState::Provisional)], 35);
        book.advance(&[observation((30, 40), CandidateState::Confirmed)], 40);
        let text = format!("{:?}", book.streams());
        let digest = text.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(0x100000001b3)
        });
        assert_eq!(
            digest, 6427703010027133843,
            "事件流输出漂移须诚实更新 golden"
        );
    }

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
        let flat_strength = vec![0.0; close_src.len()];
        let closes_tick = vec![100; close_src.len()];
        let observations = structural_observations_for_level(
            0,
            &centers,
            &segments,
            &anchors,
            &flat_strength,
            &flat_strength,
            &closes_tick,
            &close_src,
            DivergenceGauge::MacdArea,
        );
        assert_eq!(
            observations.len(),
            1,
            "结构门成立即产候选，力度 C<A=false 不得拦截"
        );
        assert_eq!(observations[0].key.side_tag, 0);
        assert_eq!(observations[0].first_provable_at, 11);
        assert_eq!(observations[0].state, CandidateState::Provisional);
    }
}
