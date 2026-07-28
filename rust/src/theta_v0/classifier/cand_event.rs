//! #550 塔内原生背驰段候选事件。
//!
//! 本模块只产出、存储候选生命史，不接 BSP/admission/订单消费点。候选事实在
//! `classify_impl` 的逐级循环内，由与 BSP 共用的 `signal::judge_first_cached` 结构判定产出；
//! 力度只影响 BSP bit，不进入候选身份或事件字段。

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::super::types::{Center, Direction, Segment, Side, Tick};
use super::decompose::center_trend_gate;
use super::divergence::{
    departure_move_c_start, locate_departure_move_a, move_range_envelope, DivergenceGauge,
};
use super::signal;

pub const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;
const PAIR_ID_SEED: u64 = 0x84222325cbf29ce4;

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
    pub kind: CandidateKind,
    pub side: Side,
    /// Trend 域的前中枢；Pan 域没有前中枢，必须为 None。
    pub previous_center_start: Option<usize>,
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
///
/// 当前 #550 生产点只在单源结构 judge 通过后构造事件，因此三项恒为 true；需要表达部分前提
/// 成立的中间形态归 #551，不在本票伪造半成品状态。
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
    /// Trend 域为（前中枢，父中枢）；Pan 域没有前中枢，必须为 None。
    pub center_ids: Option<(usize, usize)>,
    /// SPEC E2E-D2 字段；当前为 CandidateKey 的 FNV 种子哈希，与 key 双射，无独立信息。
    pub candidate_group_id: u64,
    /// SPEC E2E-D2 字段；当前为 CandidateKey 的另一种子哈希，与 key 双射，无独立信息。
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    /// SPEC E2E-D3 形状位；当前恒为 `key.seg_a` 的副本，无独立业务信息。
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

/// 每级一条 append-only 流；双层 Rc 让事件入口只克隆引用，不深拷贝全簿。
pub type CandidateStreams = Rc<Vec<Rc<Vec<CandidateEvent>>>>;

/// 闭区间 C⊆C；端点相等（相切）算包含。
pub fn interval_is_sub(child: (usize, usize), parent: (usize, usize)) -> bool {
    child.0 <= child.1 && parent.0 <= parent.1 && parent.0 <= child.0 && child.1 <= parent.1
}

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
    pub state: CandidateState,
    pub first_provable_at: usize,
    pub confirmed_at: Option<usize>,
}

/// 每级 append-only 修订簿。终态不复活；同投影重跑零 Delta。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEventBook {
    streams: CandidateStreams,
    latest: BTreeMap<CandidateKey, (usize, usize)>,
}

impl Default for CandidateEventBook {
    fn default() -> Self {
        Self {
            streams: Rc::new(Vec::new()),
            latest: BTreeMap::new(),
        }
    }
}

impl CandidateEventBook {
    pub fn streams(&self) -> CandidateStreams {
        Rc::clone(&self.streams)
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
            if let Some(event) = self.advance_observation(observation, as_of) {
                delta.push(event);
            }
        }
        delta.extend(self.invalidate_unseen(&seen, as_of));
        delta
    }

    fn invalidate_unseen(
        &mut self,
        seen: &BTreeSet<CandidateKey>,
        as_of: usize,
    ) -> Vec<CandidateEvent> {
        let mut delta = Vec::new();
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
            let invalidated = invalidate(&prior, as_of);
            self.append(invalidated.clone());
            delta.push(invalidated);
        }
        delta
    }

    fn advance_observation(
        &mut self,
        observation: &CandidateObservation,
        as_of: usize,
    ) -> Option<CandidateEvent> {
        let prior = self.latest_event(observation.key);
        if prior
            .as_ref()
            .is_some_and(|event| event.state.is_terminal())
        {
            return None;
        }
        if prior
            .as_ref()
            .is_some_and(|event| observation.interval.1 < event.interval.1)
        {
            let invalidated = invalidate(prior.as_ref().unwrap(), as_of);
            self.append(invalidated.clone());
            return Some(invalidated);
        }
        let next = make_revision(prior.as_ref(), observation, as_of);
        if prior
            .as_ref()
            .is_some_and(|event| same_projection(event, &next))
        {
            return None;
        }
        self.append(next.clone());
        Some(next)
    }

    fn latest_event(&self, key: CandidateKey) -> Option<CandidateEvent> {
        self.latest.get(&key).copied().and_then(|(level, index)| {
            self.streams
                .get(level)
                .and_then(|stream| stream.get(index))
                .cloned()
        })
    }

    fn append(&mut self, event: CandidateEvent) {
        let level = event.event_level as usize;
        let streams = Rc::make_mut(&mut self.streams);
        if streams.len() <= level {
            streams.resize_with(level + 1, || Rc::new(Vec::new()));
        }
        let stream = Rc::make_mut(&mut streams[level]);
        let index = stream.len();
        self.latest.insert(event.key, (level, index));
        stream.push(event);
    }
}

fn invalidate(prior: &CandidateEvent, as_of: usize) -> CandidateEvent {
    let mut invalidated = prior.clone();
    invalidated.state = CandidateState::Invalidated;
    invalidated.invalidated_at = Some(as_of);
    invalidated.revision += 1;
    invalidated.supersedes_revision = Some(prior.revision);
    invalidated.revision_at = as_of;
    invalidated
}

fn make_revision(
    prior: Option<&CandidateEvent>,
    observation: &CandidateObservation,
    as_of: usize,
) -> CandidateEvent {
    let revision = prior.map_or(0, |event| event.revision + 1);
    let observed_at = prior.map_or(as_of, |event| event.observed_at);
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
                .or_else(|| {
                    (observation.kind != CandidateKind::Pan)
                        .then_some(observation.confirmed_at)
                        .flatten()
                })
                .or(Some(as_of))
        } else {
            None
        },
        invalidated_at: None,
        revision,
        supersedes_revision: prior.map(|event| event.revision),
        revision_at: as_of,
    }
}

#[derive(PartialEq)]
struct CandidateProjection {
    kind: CandidateKind,
    center_ids: Option<(usize, usize)>,
    candidate_group_id: u64,
    pair_id: u64,
    structural_predicates: StructuralPredicates,
    extreme_proof: (usize, usize),
    third_class_proof: Option<usize>,
    interval: (usize, usize),
    state: CandidateState,
}

impl From<&CandidateEvent> for CandidateProjection {
    fn from(event: &CandidateEvent) -> Self {
        Self {
            kind: event.kind,
            center_ids: event.center_ids,
            candidate_group_id: event.candidate_group_id,
            pair_id: event.pair_id,
            structural_predicates: event.structural_predicates,
            extreme_proof: event.extreme_proof,
            third_class_proof: event.third_class_proof,
            interval: event.interval,
            state: event.state,
        }
    }
}

fn same_projection(a: &CandidateEvent, b: &CandidateEvent) -> bool {
    CandidateProjection::from(a) == CandidateProjection::from(b)
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
    blocks: &[super::decompose::MoveBlock],
    segments: &[Segment],
    anchor_dirs: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
) -> Vec<CandidateObservation> {
    let center_gate = center_trend_gate(centers.len(), blocks);
    let context = StructuralScan {
        level,
        centers,
        segments,
        anchor_dirs,
        hist,
        dif,
        closes_tick,
        close_src,
        gauge,
    };
    let mut observations: Vec<_> = segments
        .iter()
        .enumerate()
        .filter_map(|(i, segment)| structural_observation(&context, &center_gate, i, segment))
        .collect();
    observations.sort_by_key(|observation| (observation.interval, observation.key));
    observations
}

struct StructuralScan<'a> {
    level: u32,
    centers: &'a [Center],
    segments: &'a [Segment],
    anchor_dirs: &'a [Option<Direction>],
    hist: &'a [f64],
    dif: &'a [f64],
    closes_tick: &'a [Tick],
    close_src: &'a [usize],
    gauge: DivergenceGauge,
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
    let point = signal::judge_first_cached(
        parent,
        direction,
        segment,
        scan.anchor_dirs[i],
        scan.hist,
        scan.dif,
        scan.closes_tick,
        scan.close_src,
        a,
        c_start,
        scan.gauge,
    )?;
    Some(trend_observation(
        scan.level,
        previous,
        parent,
        segment,
        a?,
        c_start?,
        point.struct_break_dir.expect("结构候选必有方向"),
    ))
}

fn trend_observation(
    level: u32,
    previous: &Center,
    parent_center: &Center,
    segment: &Segment,
    a: ((usize, usize), (Tick, Tick)),
    c_start: usize,
    side: Side,
) -> CandidateObservation {
    let key = CandidateKey {
        level,
        kind: CandidateKind::Trend,
        side,
        previous_center_start: Some(previous.start_index),
        parent: ParentFingerprint {
            center_start: parent_center.start_index,
            zd: parent_center.zd,
            zg: parent_center.zg,
        },
        seg_a: a.0,
        c_start,
    };
    CandidateObservation {
        key,
        kind: CandidateKind::Trend,
        center_ids: Some((previous.start_index, parent_center.start_index)),
        candidate_group_id: stable_id(&key, FNV_OFFSET_BASIS),
        pair_id: stable_id(&key, PAIR_ID_SEED),
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: a.0,
        third_class_proof: None,
        interval: (c_start, segment.end_index),
        state: CandidateState::Provisional,
        first_provable_at: segment.end_index,
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
                state: CandidateState::Confirmed,
                first_provable_at: cert.source_index,
                confirmed_at: None,
            }
        })
        .collect()
}

/// 单级双域候选的统一接线点，供全量与增量分类循环共同调用。
#[allow(clippy::too_many_arguments)]
pub(crate) fn observations_for_level(
    level: u32,
    centers: &[Center],
    blocks: &[super::decompose::MoveBlock],
    segments: &[Segment],
    anchor_dirs: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
    pan_div: &[signal::PanDivCert],
) -> Vec<CandidateObservation> {
    let mut observations = structural_observations_for_level(
        level,
        centers,
        blocks,
        segments,
        anchor_dirs,
        hist,
        dif,
        closes_tick,
        close_src,
        gauge,
    );
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
    use super::*;
    use crate::theta_v0::classifier::decompose::decompose;

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
            first_provable_at: 35,
            confirmed_at: None,
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
        assert_eq!(book.advance(std::slice::from_ref(&first), 37).len(), 1);
        assert!(book.advance(std::slice::from_ref(&first), 37).is_empty());

        let grown = observation((30, 40), CandidateState::Confirmed);
        assert_eq!(book.advance(std::slice::from_ref(&grown), 40).len(), 1);
        let stream = &book.streams()[0];
        assert_eq!(stream.len(), 2);
        assert_eq!(stream[0].key, stream[1].key);
        assert_eq!(stream[1].revision, 1);
        assert_eq!(stream[1].supersedes_revision, Some(0));
        assert_eq!(stream[1].observed_at, 37);
        assert_eq!(stream[1].first_provable_at, 35);
        assert_eq!(stream[1].confirmed_at, Some(40));
        assert_eq!(stream[1].revision_at, 40);
    }

    #[test]
    fn confirmed_clock_uses_first_observation_as_of_not_geometry_clock() {
        let mut book = CandidateEventBook::default();
        let mut confirmed = observation((30, 35), CandidateState::Confirmed);
        confirmed.kind = CandidateKind::Pan;
        confirmed.key.kind = CandidateKind::Pan;
        confirmed.confirmed_at = Some(35);
        let event = book.advance(&[confirmed], 42).remove(0);
        assert_eq!(event.first_provable_at, 35);
        assert_eq!(event.observed_at, 42);
        assert_eq!(event.confirmed_at, Some(42));
    }

    #[test]
    fn same_right_endpoint_projection_change_appends_revision() {
        let mut book = CandidateEventBook::default();
        let first = observation((30, 35), CandidateState::Provisional);
        book.advance(std::slice::from_ref(&first), 35);
        let mut changed = first;
        changed.center_ids = Some((10, 21));
        let delta = book.advance(&[changed], 36);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].revision, 1);
        assert_eq!(delta[0].center_ids, Some((10, 21)));
    }

    #[test]
    fn interval_shrink_appends_invalidation() {
        let mut book = CandidateEventBook::default();
        let first = observation((30, 40), CandidateState::Provisional);
        book.advance(std::slice::from_ref(&first), 40);
        let shrunk = observation((30, 35), CandidateState::Provisional);
        let delta = book.advance(&[shrunk], 41);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].state, CandidateState::Invalidated);
        assert_eq!(delta[0].invalidated_at, Some(41));
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
            &decompose(&centers),
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
        assert_eq!(observations[0].key.side, Side::Long);
        assert_eq!(observations[0].first_provable_at, 11);
        assert_eq!(observations[0].state, CandidateState::Provisional);
    }
}
