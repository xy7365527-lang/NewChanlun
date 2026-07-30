//! #634 拆自单文件 `classifier/cand_event.rs` 的**事件簿**域。
//!
//! 每级 append-only 修订簿 [`CandidateEventBook`]（终态不复活、同投影重跑零 Delta）与生命史
//! 覆盖探针 [`event_probe`]（#551 防真空绿）。观察怎么来的在 [`super::observe`]，
//! 身份键与谓词类型在 [`super::key`]。

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::key::{CandidateEvent, CandidateKey, CandidateKind, CandidateState, CandidateStreams};
use super::observe::CandidateObservation;

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
            reject_unobservable_invalidation(observation);
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
    ///
    /// ★`next.state == Invalidated` 在本函数是**不可达**的，两处 `unreachable!` 是该事实的机器载体
    /// （不是「应该不会发生」的注释声明）。推导链：
    /// 1. 本函数的唯一调用点是 [`super::CandidateEventBook::advance_observation`] 的落簿处，
    ///    其 `next` 由 [`super::make_revision`] 产出 ⟹ `next.state == observation.state`（逐字复制）。
    /// 2. 观察进事件簿的唯一入口 [`super::CandidateEventBook::advance`] 已由
    ///    [`super::reject_unobservable_invalidation`] 把 `observation.state == Invalidated` 全部挡下
    ///    ⟹ 走到本函数的 `next.state` 值域 = {`Provisional`, `Unresolved`, `Confirmed`}。
    /// 3. 两条真实失效路径（缺席 / 回缩）调 [`super::invalidate`] 后走 [`on_invalidate`]，不经本函数。
    ///
    /// 故 `∅ → Invalidated` 与「活候选 → Invalidated」两条 E2E-D5 禁止边在此当场失败，
    /// 而不是记零或静默滑过——静默滑过会让「该边从未发生」与「该边发生过但没人看见」不可区分。
    pub fn on_append(prior: Option<&CandidateEvent>, next: &CandidateEvent) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            match prior {
                None => match next.state {
                    CandidateState::Provisional => p.birth_provisional += 1,
                    CandidateState::Unresolved => p.birth_unresolved += 1,
                    CandidateState::Confirmed => p.birth_confirmed += 1,
                    CandidateState::Invalidated => unreachable!(
                        "E2E-D5 禁止边 ∅→Invalidated 被走到：候选不得一出生即失效终态\
                         （失效只能由 invalidate() 对既有 revision 判出，见 on_append 头部推导链）"
                    ),
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
                            CandidateState::Invalidated => unreachable!(
                                "E2E-D5 禁止边「活候选→(被观察为)Invalidated」被走到：\
                                 失效不经 on_append（走 invalidate() + on_invalidate），\
                                 见 on_append 头部推导链"
                            ),
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

/// E2E-D5 禁止边 `∅ → Invalidated`（以及「活候选 →（被观察为）Invalidated」）的机器锁，
/// 位于事件簿的**唯一观察入口** [`CandidateEventBook::advance`]。
///
/// 推导链（为什么 `Invalidated` 不是可观察状态）：
/// 1. `Invalidated` 的唯一构造点是 [`invalidate`]——它写 `invalidated_at = Some(as_of)`、
///    递增 `revision`、置 `supersedes_revision`。两条失效路径（缺席
///    [`CandidateEventBook::invalidate_unseen`]、回缩 [`CandidateEventBook::advance_observation`]）
///    都走它，且都要求存在 `prior`：失效是**事件簿对既有 revision 的判决**，不是外部观察的输入。
/// 2. 观察侧走的是 [`make_revision`]，它硬编码 `invalidated_at: None`。因此一条
///    `state = Invalidated` 的观察若被放行，落簿的是「终态但无失效钟」的畸形事件；
///    又因 [`CandidateState::is_terminal`] 为真，该 key 的后续全部修订被永久挡回（禁复活），
///    畸形态就此钉死且不可修复。
/// 3. 故 `∅ → Invalidated`（候选一出生即终态）在 E2E-D5 状态机里是禁止边，
///    「活候选被观察为 Invalidated」同理——两者的前提都是 `observation.state == Invalidated`。
///
/// 本函数是这条禁止边的**机器**载体：一旦被走到就当场失败，而不是静默滑过。
/// 生产恒不触发（结构域观察的状态由 [`super::key::StructuralPredicates::resolved_state`] 派生，
/// 值域 = {`Provisional`, `Unresolved`}；Pan 域恒 `Confirmed`），
/// 但 [`CandidateObservation`] 是 `pub` 且字段全开 ⟹ 该边在类型层可表达，必须在运行期拒绝。
fn reject_unobservable_invalidation(observation: &CandidateObservation) {
    assert!(
        observation.state != CandidateState::Invalidated,
        "E2E-D5 禁止边：`Invalidated` 是事件簿自产终态（唯一构造点 invalidate()），不是可观察状态；\
         key={:?} 的观察携带 Invalidated ⟹ 拒绝入簿",
        observation.key
    );
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

fn same_projection(a: &CandidateEvent, b: &CandidateEvent) -> bool {
    a.projection() == b.projection()
}

#[cfg(test)]
mod tests {
    use super::super::super::signal;
    use super::super::fixtures::{
        center, deferred_envelope_break_scan, extended_episode_scan, observation, scan_observations,
    };
    use super::super::observe::pan_observations_for_level;
    use super::*;
    use crate::theta_v0::types::Side;

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
    #[should_panic(expected = "E2E-D5 禁止边")]
    fn observed_invalidation_at_birth_is_rejected_by_the_book_entry_lock() {
        // ★#551 E2E-D5 禁止边 ∅→Invalidated 的机器锁见证：候选不得一出生即失效终态。
        // 反事实（本锁之前）：该观察静默入簿，落成「state=Invalidated 但 invalidated_at=None」的
        // 畸形终态事件，并因 is_terminal() 永久挡回该 key 的后续修订 —— 无人看得见。
        let mut book = CandidateEventBook::default();
        book.advance(&[observation((30, 35), CandidateState::Invalidated)], 35);
    }

    #[test]
    #[should_panic(expected = "E2E-D5 禁止边")]
    fn observed_invalidation_on_live_candidate_is_rejected_by_the_same_lock() {
        // ★同一条锁的第二个入口：活候选被**观察为** Invalidated（on_append 的 :Some(prior) 侧禁止边）。
        // 合法失效走 invalidate()（缺席/回缩两路径），不经观察 —— 见
        // disappearance_appends_invalidation_and_terminal_never_revives / interval_shrink_appends_invalidation。
        let mut book = CandidateEventBook::default();
        let live = observation((30, 35), CandidateState::Provisional);
        book.advance(std::slice::from_ref(&live), 35);
        book.advance(&[observation((30, 40), CandidateState::Invalidated)], 40);
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

    #[test]
    fn growth_revision_counts_same_state_right_end_extension() {
        // ★#551 growth_revision 非零锁（此前全仓只有 assert_eq!(.., 0)，方向与「已覆盖」相反）。
        // 走生产扫描：同一 key 连续两次观察状态不变（Provisional→Provisional）而 I(C) 右端生长
        // ⟹ 落在 on_append 的「状态不变 ∧ 右端增长」分支，不是状态转移分支、也不是载荷修订分支。
        event_probe::reset();
        let (centers, segments, anchors, close_src) = extended_episode_scan();
        let early = scan_observations(&centers, &segments[..5], &anchors[..5], &close_src);
        let late = scan_observations(&centers, &segments, &anchors, &close_src);
        assert_eq!(early.len(), 1);
        assert_eq!(late.len(), 1);
        assert_eq!(early[0].key, late[0].key, "右端不入键 ⟹ 同一候选身份");
        assert_eq!(early[0].state, CandidateState::Provisional);
        assert_eq!(late[0].state, CandidateState::Provisional);
        assert_eq!(early[0].interval, (9, 15));
        assert_eq!(late[0].interval, (9, 19), "同 episode 续腿 ⟹ I(C) 右端生长");

        let mut book = CandidateEventBook::default();
        assert_eq!(book.advance(&early, 15).len(), 1);
        assert_eq!(book.advance(&late, 19).len(), 1);
        let probe = event_probe::snapshot();
        assert!(probe.growth_revision > 0, "生长修订分支必须真被触发");
        assert_eq!(probe.growth_revision, 1);
        assert_eq!(probe.birth_provisional, 1, "首条 revision 是出生，不是生长");
        assert_eq!(probe.payload_revision, 0, "右端生长不得记入载荷修订");
        assert_eq!(probe.to_provisional, 0, "状态不变 ⟹ 不计状态转移");
    }

    #[test]
    fn invalidated_shrink_counts_the_stale_geometry_path() {
        // ★#551 invalidated_shrink 非零锁（此前全仓只有 assert_eq!(.., 0)）。
        // 触发条件是事件簿契约层的「同 key 新观察右端 < 在案右端」（陈旧几何 ⟹ 判 Invalidated），
        // 与缺席失效是两条独立路径，必须分别可见：本测同时钉死 absent 计数为零，防两路混记。
        event_probe::reset();
        let mut book = CandidateEventBook::default();
        let first = observation((30, 40), CandidateState::Provisional);
        assert_eq!(book.advance(std::slice::from_ref(&first), 40).len(), 1);
        let shrunk = observation((30, 35), CandidateState::Provisional);
        let delta = book.advance(&[shrunk], 41);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].state, CandidateState::Invalidated);
        assert_eq!(delta[0].invalidated_at, Some(41));
        let probe = event_probe::snapshot();
        assert!(probe.invalidated_shrink > 0, "回缩失效分支必须真被触发");
        assert_eq!(probe.invalidated_shrink, 1);
        assert_eq!(probe.invalidated_absent, 0, "回缩失效不得记到缺席路径");
    }

    #[test]
    fn birth_confirmed_counts_pan_domain_first_revision() {
        // ★#551 birth_confirmed 非零锁（此前全仓零读取零断言）。走生产投影 `pan_observations_for_level`：
        // Pan 域证书恒投影为 Confirmed 观察 ⟹ 一入簿即 ∅→Confirmed（出生边，非状态转移边）。
        event_probe::reset();
        let cert = signal::PanDivCert {
            source_index: 15,
            side: Side::Long,
            center: center(100, 200, 8),
            seg_a: (3, 5),
            seg_c: (9, 15),
        };
        let observations = pan_observations_for_level(0, std::slice::from_ref(&cert));
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].state, CandidateState::Confirmed);

        let mut book = CandidateEventBook::default();
        let delta = book.advance(&observations, 42);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].revision, 0, "首条 revision ⟹ 出生边");
        assert_eq!(delta[0].supersedes_revision, None);
        assert_eq!(delta[0].state, CandidateState::Confirmed);
        assert_eq!(delta[0].confirmed_at, Some(42));
        let probe = event_probe::snapshot();
        assert!(
            probe.birth_confirmed > 0,
            "∅→Confirmed 出生分支必须真被触发"
        );
        assert_eq!(probe.birth_confirmed, 1);
        assert_eq!(probe.birth_provisional, 0);
        assert_eq!(probe.birth_unresolved, 0);
        assert_eq!(
            probe.to_confirmed, 0,
            "∅→Confirmed 是出生边，不计入状态转移"
        );
    }
}
