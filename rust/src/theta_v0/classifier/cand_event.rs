//! #550 塔内原生背驰段候选事件。
//!
//! 本模块只产出、存储候选生命史，不接 BSP/admission/订单消费点。候选事实在
//! `classify_impl` 的逐级循环内，由与 BSP 共用的 `signal::first_structural_gates` 结构门产出；
//! 力度只影响 BSP bit，不进入候选身份、状态或任一事件字段（#551 起本模块在类型层够不到 MACD 序列）。

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::super::types::{Center, Direction, Segment, Side};
use super::decompose::center_trend_gate;
use super::divergence::{departure_move_c_start, locate_departure_move_a, move_range_envelope};
use super::signal;

pub const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;
const PAIR_ID_SEED: u64 = 0x84222325cbf29ce4;

/// 候选判定规则版本（E2E-O「上游身份或规则版本变 ⟹ 新 key，禁复活旧 key」的版本分量）。
///
/// 语义：本常量随**候选判定规则**（结构门口径、状态映射、身份字段构成）的任何实质变更递增。
/// 版本进 [`CandidateKey`] ⟹ 规则变更后旧 key 在新扫描中不再被观察到 ⟹ 经既有消失路径判
/// `Invalidated`（终态），新规则的候选以新 key 从 ∅ 开始——旧 key 不被复活、旧生命史不被改写。
///
/// 值 1 = #550 落地的结构宽候选口径 + #551 的四态全谱映射。
pub const CANDIDATE_RULE_VERSION: u32 = 1;

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
    /// 判定规则版本（见 [`CANDIDATE_RULE_VERSION`]）：规则变更 ⟹ 全体 key 变 ⟹ 旧 key 走消失
    /// 路径判终态，新 key 从 ∅ 起——E2E-O「规则版本变 ⟹ 新 key 禁复活」的机器载体。
    pub rule_version: u32,
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

/// 结构谓词证据；字段值来自同一次塔扫描的 [`signal::FirstStructuralGates`]，不在本模块重判。
///
/// #551 起三项与结构门逐条同源，**不再恒真**：
/// - `direction`：破最后中枢 ∧ 破向 = 趋势向。恒 `true` 是**结构必然**而非占位——该门不成立时
///   结构门函数返回 `None`，候选身份根本不存在（无 `seg_a`/`c_start` 来源），故凡入簿事件此项必真。
/// - `comparable`：I(A)/I(C) 均可映射到 closes 下标（MACD 面积坐标系上 A/C 可比较）。
/// - `extreme`：037:20，c 端点严格破 b = I(A) 包络极值。
///
/// `comparable ∧ extreme` ⟺ 结构宽候选完全成立 ⟺ [`CandidateState::Provisional`]；任一不成立
/// ⟹ [`CandidateState::Unresolved`]（结构未决，非失效——同一 key 的 C 段后续可延伸至成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralPredicates {
    pub direction: bool,
    pub comparable: bool,
    pub extreme: bool,
}

impl StructuralPredicates {
    /// 三谓词全成立 ⟹ 结构宽候选成立（`Provisional`）；否则未决（`Unresolved`）。
    pub fn all_hold(self) -> bool {
        self.direction && self.comparable && self.extreme
    }

    /// 由谓词组合派生的活假设状态（Trend 域状态映射单一来源）。
    pub fn resolved_state(self) -> CandidateState {
        if self.all_hold() {
            CandidateState::Provisional
        } else {
            CandidateState::Unresolved
        }
    }
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
    /// 候选首次入簿时的 `as_of`（一次写入不后移）。
    pub observed_at: usize,
    /// 首证钟：结构宽候选**首次完全成立**时的结构位（一次写入不后移）。
    ///
    /// `None` = 尚未可证（该 key 迄今只观察到 [`CandidateState::Unresolved`]）。写入时机严格是
    /// 「首次全谓词成立的那次观察」的结构位——不提前（未决期不写）、不回填（写入后即钉死，
    /// 后续退回 `Unresolved` 也不抹去）。#535 红线②的机器载体。
    pub first_provable_at: Option<usize>,
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
    /// 本次观察若使结构宽候选完全成立，则为该次的结构位；未决观察为 `None`。
    /// 事件簿只在首证钟尚空时采纳它（一次写入不后移）。
    pub first_provable_at: Option<usize>,
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
            #[cfg(test)]
            event_probe::on_invalidate(&prior, event_probe::InvalidationCause::Absent);
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
            #[cfg(test)]
            event_probe::on_terminal_block();
            return None;
        }
        if prior
            .as_ref()
            .is_some_and(|event| observation.interval.1 < event.interval.1)
        {
            let stale = prior.as_ref().expect("回缩分支蕴含 prior 存在");
            let invalidated = invalidate(stale, as_of);
            #[cfg(test)]
            event_probe::on_invalidate(stale, event_probe::InvalidationCause::Shrink);
            self.append(invalidated.clone());
            return Some(invalidated);
        }
        let next = make_revision(prior.as_ref(), observation, as_of);
        if prior
            .as_ref()
            .is_some_and(|event| same_projection(event, &next))
        {
            #[cfg(test)]
            event_probe::on_idempotent_skip();
            return None;
        }
        #[cfg(test)]
        event_probe::on_append(prior.as_ref(), &next);
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

/// E2E-O 生命史覆盖探针（#551 防真空绿）。
///
/// 与 `classifier::oracle_probe` 同形状：模块常开、调用点 `#[cfg(test)]` ⟹ 生产零开销。
/// 计数按**事件簿实际走过的分支**归类，测试据此断言「某条转移/修订/钟路径真被触发」，
/// 而不是仅断言最终态——后者在候选恒不产出时也会绿。
pub mod event_probe {
    use super::{CandidateEvent, CandidateState};
    use std::cell::RefCell;

    /// 失效来源：观察缺席（候选身份消失） vs 区间回缩（陈旧几何）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InvalidationCause {
        Absent,
        Shrink,
    }

    #[derive(Default, Clone, Debug, PartialEq, Eq)]
    pub struct EventProbe {
        /// ∅ → 首个 revision，按落地状态分解。
        pub birth_provisional: u64,
        pub birth_unresolved: u64,
        pub birth_confirmed: u64,
        /// 已有 revision 且状态**改变**的转移，按目标状态分解。
        pub to_provisional: u64,
        pub to_unresolved: u64,
        pub to_confirmed: u64,
        /// 状态不变的修订：区间右端增长（活段生长） vs 右端不动的其他投影变化。
        pub growth_revision: u64,
        pub payload_revision: u64,
        /// 失效两路径。
        pub invalidated_absent: u64,
        pub invalidated_shrink: u64,
        /// 投影相同 ⟹ 零输出（幂等）。
        pub idempotent_skip: u64,
        /// 终态挡回的观察（禁复活）。
        pub terminal_block: u64,
        /// 首证钟由空首次写入。
        pub first_provable_written: u64,
        /// 首证钟已在案 ⟹ 沿用旧值（不后移）。
        pub first_provable_pinned: u64,
        /// `observed_at` 沿用旧值（不后移）。
        pub observed_pinned: u64,
        /// 确认钟已在案 ⟹ 沿用旧值（不后移）。
        pub confirmed_pinned: u64,
    }

    thread_local! {
        static PROBE: RefCell<EventProbe> = RefCell::new(EventProbe::default());
    }

    pub fn reset() {
        PROBE.with(|p| *p.borrow_mut() = EventProbe::default());
    }

    pub fn snapshot() -> EventProbe {
        PROBE.with(|p| p.borrow().clone())
    }

    /// 一次 revision 落簿：分解状态转移、修订类别与三只钟的写入/钉死。
    pub fn on_append(prior: Option<&CandidateEvent>, next: &CandidateEvent) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            match prior {
                None => match next.state {
                    CandidateState::Provisional => p.birth_provisional += 1,
                    CandidateState::Unresolved => p.birth_unresolved += 1,
                    CandidateState::Confirmed => p.birth_confirmed += 1,
                    CandidateState::Invalidated => {}
                },
                Some(prior) => {
                    if prior.state == next.state {
                        if next.interval.1 > prior.interval.1 {
                            p.growth_revision += 1;
                        } else {
                            p.payload_revision += 1;
                        }
                    } else {
                        match next.state {
                            CandidateState::Provisional => p.to_provisional += 1,
                            CandidateState::Unresolved => p.to_unresolved += 1,
                            CandidateState::Confirmed => p.to_confirmed += 1,
                            CandidateState::Invalidated => {}
                        }
                    }
                    p.observed_pinned += 1;
                    if prior.first_provable_at.is_some() {
                        p.first_provable_pinned += 1;
                    }
                    if prior.confirmed_at.is_some() && next.confirmed_at == prior.confirmed_at {
                        p.confirmed_pinned += 1;
                    }
                }
            }
            if prior.and_then(|event| event.first_provable_at).is_none()
                && next.first_provable_at.is_some()
            {
                p.first_provable_written += 1;
            }
        });
    }

    pub fn on_invalidate(prior: &CandidateEvent, cause: InvalidationCause) {
        let _ = prior;
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            match cause {
                InvalidationCause::Absent => p.invalidated_absent += 1,
                InvalidationCause::Shrink => p.invalidated_shrink += 1,
            }
        });
    }

    pub fn on_idempotent_skip() {
        PROBE.with(|p| p.borrow_mut().idempotent_skip += 1);
    }

    pub fn on_terminal_block() {
        PROBE.with(|p| p.borrow_mut().terminal_block += 1);
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
    // observed_at / first_provable_at 均为「一次写入不后移」：已在案的值优先，空缺才采纳本次观察。
    let observed_at = prior.map_or(as_of, |event| event.observed_at);
    let first_provable_at = prior
        .and_then(|event| event.first_provable_at)
        .or(observation.first_provable_at);
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
    blocks: &[super::decompose::MoveBlock],
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
        CandidateState::Provisional => acc
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
        first_provable_at: (state == CandidateState::Provisional).then_some(segment.end_index),
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
                state: CandidateState::Confirmed,
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
    blocks: &[super::decompose::MoveBlock],
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
    use super::super::super::types::Tick;
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
        assert_eq!(stream[1].first_provable_at, Some(35));
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
        assert_eq!(event.first_provable_at, Some(35));
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
        assert_eq!(observations[0].state, CandidateState::Provisional);
        assert!(observations[0].structural_predicates.all_hold());
    }

    /// #551 状态机全谱共用夹具（几何取自 `signal::first_class_point_deferred_to_envelope_breaking_leg_03720`）。
    ///
    /// A 腿 b_lo=90；C 腿1 @11 破核心（95 < zd=100）但未破包络极值（95 ≥ 90）；C 内反向段 @13
    /// 端点 98 < zd 未回核心 ⟹ 与腿1 同一 episode（λ_C=9）；C 腿2 @15 端点 85 < 90 破极值。
    /// 故腿1 与腿2 **共享同一 CandidateKey**（同 seg_a、同 λ_C、同中枢对），只在谓词与右端不同。
    fn deferred_envelope_break_scan() -> (
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

    fn scan_observations(
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
        assert_eq!(observation.state, CandidateState::Unresolved);
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
        assert_eq!(observations[0].state, CandidateState::Provisional);
        assert_eq!(
            observations[0].first_provable_at,
            Some(15),
            "首证钟 = 首个使全谓词成立的腿的结构位"
        );
    }

    #[test]
    fn same_key_transitions_unresolved_to_provisional_and_pins_first_provable_clock() {
        // ★#551 US「Unresolved→Provisional + 生长修订 + 首证钟一次写入」三合一，走**跨 as_of**
        // 的真实增量形态：episode 先只到腿1（未破极值 ⟹ 未决），后延伸到腿2（破极值 ⟹ 可证）。
        event_probe::reset();
        let (centers, segments, anchors, close_src) = deferred_envelope_break_scan();
        let early = scan_observations(&centers, &segments[..3], &anchors[..3], &close_src);
        let late = scan_observations(&centers, &segments, &anchors, &close_src);
        assert_eq!(early.len(), 1);
        assert_eq!(late.len(), 1);
        assert_eq!(early[0].key, late[0].key, "右端不入键 ⟹ 同一候选身份");
        assert_eq!(early[0].state, CandidateState::Unresolved);
        assert_eq!(late[0].state, CandidateState::Provisional);

        let mut book = CandidateEventBook::default();
        assert_eq!(book.advance(&early, 11).len(), 1);
        assert_eq!(book.advance(&late, 15).len(), 1);
        let stream = &book.streams()[0];
        assert_eq!(stream.len(), 2);
        assert_eq!(stream[0].state, CandidateState::Unresolved);
        assert_eq!(stream[0].revision, 0);
        assert_eq!(stream[0].first_provable_at, None);
        assert_eq!(stream[1].state, CandidateState::Provisional);
        assert_eq!(stream[1].revision, 1);
        assert_eq!(stream[1].supersedes_revision, Some(0));
        assert_eq!(stream[1].interval.1, 15, "C 段右端生长");
        assert_eq!(
            stream[1].first_provable_at,
            Some(15),
            "首证钟落在首次全谓词成立的结构位，不回填到未决期"
        );
        assert_eq!(stream[1].observed_at, 11, "入簿钟 = 首次入簿 as_of，不后移");

        // 覆盖非真空：本电池真触发 ∅→Unresolved 与 Unresolved→Provisional 两条路径。
        let probe = event_probe::snapshot();
        assert_eq!(probe.birth_unresolved, 1);
        assert_eq!(probe.to_provisional, 1);
        assert_eq!(probe.first_provable_written, 1);
        assert_eq!(probe.observed_pinned, 1);
        assert_eq!(probe.birth_provisional, 0);
        assert_eq!(probe.growth_revision, 0, "跨状态转移不计入同状态生长修订");

        // 幂等（US8）：同一批观察同 as_of 重跑 ⟹ Delta=∅。
        event_probe::reset();
        assert!(book.advance(&late, 15).is_empty());
        let idle = event_probe::snapshot();
        assert_eq!(idle.idempotent_skip, 1);
        assert_eq!(
            idle.invalidated_shrink, 0,
            "归约后不再把同扫描短腿误判为回缩"
        );
    }

    #[test]
    fn first_provable_clock_survives_fallback_to_unresolved() {
        // ★首证钟「一次写入不后移」的反向边：已可证的候选后续观察退回 Unresolved 时，钟不被抹去、
        // 不被改写（#535 红线②的对偶——既禁未来回填，也禁既有首证被后续未决擦除）。
        let mut book = CandidateEventBook::default();
        let provable = observation((30, 35), CandidateState::Provisional);
        book.advance(std::slice::from_ref(&provable), 35);
        let mut regressed = observation((30, 40), CandidateState::Unresolved);
        regressed.key = provable.key;
        let delta = book.advance(&[regressed], 40);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].state, CandidateState::Unresolved);
        assert_eq!(delta[0].first_provable_at, Some(35), "首证钟钉死");
    }

    #[test]
    fn rule_version_bump_mints_new_key_and_never_revives_terminal_one() {
        // ★#551 US「上游身份或规则版本变 ⟹ 新 key 禁复活」：把规则版本 +1 视为新规则的同一结构，
        // 旧 key 因不再被观察走消失路径判终态，新 key 从 ∅ 起，旧 key 事后再现也被终态挡回。
        event_probe::reset();
        let mut book = CandidateEventBook::default();
        let old = observation((30, 35), CandidateState::Provisional);
        book.advance(std::slice::from_ref(&old), 35);

        let mut renewed = old.clone();
        renewed.key.rule_version += 1;
        let delta = book.advance(std::slice::from_ref(&renewed), 36);
        assert_eq!(delta.len(), 2, "新 key 出生 + 旧 key 失效");
        assert!(delta
            .iter()
            .any(|event| event.key == renewed.key && event.revision == 0));
        let retired = delta
            .iter()
            .find(|event| event.key == old.key)
            .expect("旧 key 必判 Invalidated");
        assert_eq!(retired.state, CandidateState::Invalidated);

        let revived = book.advance(std::slice::from_ref(&old), 37);
        assert!(
            revived.iter().all(|event| event.key != old.key),
            "旧规则版本的 key 已终态 ⟹ 禁复活"
        );
        let probe = event_probe::snapshot();
        // 两次缺席失效：换版那轮旧 key 缺席、末轮新 key 缺席；一次终态挡回：末轮旧 key 试图复活。
        assert_eq!(probe.invalidated_absent, 2);
        assert_eq!(probe.terminal_block, 1);
    }
}
