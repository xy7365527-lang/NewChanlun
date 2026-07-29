//! 三态状态机 + 成立档门户 + 警报计数（票 #621 S1 第二、四段）。
//!
//! # 全部经 T1 内核表达（AC「无自造账本机制」）
//!
//! | 责任点 | 内核原语 |
//! |---|---|
//! | per-key 注册表 / 只读枚举 | [`LedgerBook`] `entries`/`values`/`in_state` |
//! | 首次观察建项 | [`LedgerBook::open_on_observation`] |
//! | append-only 追加 | [`LedgerEntryCore::push_revision`]（全模块唯一追加点） |
//! | 倒退拒绝 + 终态吸收 | [`LedgerBook::admit`] → [`LedgerAdmission`] |
//! | 终态落账（禁复活） | [`LedgerEntryCore::settle`] + [`LedgerSettlement`] |
//! | 钟首写不改 | [`super::first_write_clock`]（出生钟建仓写、落锤钟 `write_settlement` 写） |
//! | 增量返回 | [`LedgerDelta`] |
//! | 不变量骨架 | [`LedgerBook::assert_core_invariants`] |
//!
//! 本模块只添加**域判据**：单活跃候选纪律、判胜/判败映射、Restart 谱系、成立档投影。

use std::collections::BTreeMap;

use super::super::ledger_kernel::{
    LedgerAdmission, LedgerBook, LedgerDelta, LedgerEntryCore, LedgerSettlement, LedgerState,
};
use super::adapter::{admit_input, AdmittedObservation, RetraceInput, RetraceRejection};
use super::log::{RetraceProvenance, RetraceRecord};
use super::{
    CenterAnchor, CenterFrame, NotConstitutedReason, RetraceEntry, RetraceEvidence, RetraceKey,
    RetraceOutcome, RetracePoint, RetracePolicy, RetraceRevision, RetraceRevisionKind, RetraceSide,
    RetraceState,
};

// ═══════════════════════════════════════════════════════════════════════════
// 推进结果 / 警报
// ═══════════════════════════════════════════════════════════════════════════

/// 一次推进的结果（内核增量返回 + 准入裁决）。
#[derive(Debug, Clone)]
pub struct RetraceStep {
    pub key: RetraceKey,
    pub state: RetraceState,
    /// 本轮新增修订（顺序 = 产生序）。
    pub delta: LedgerDelta<RetracePolicy>,
    /// 已建仓身份的准入裁决；本轮为**首次注册**时为 `None`。
    pub admission: Option<LedgerAdmission>,
}

impl RetraceStep {
    /// 本轮是否为终态后的迟到静默吸收（裁定三）。
    pub fn absorbed_late(&self) -> bool {
        self.admission == Some(LedgerAdmission::TerminalAbsorbed)
    }

    /// 本轮是否被门卫钟倒退拒收（裁定五）。
    pub fn retrograde_rejected(&self) -> bool {
        self.admission == Some(LedgerAdmission::RetrogradeRejected)
    }
}

/// 警报计数（裁定六：拒收/警报另记 audit 流，**不进真相恢复路径**——它们不改状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetraceAlarms {
    /// 终态后同身份迟到输入静默吸收次数（显著非零 ⟹ 查 provider）。
    pub late_absorbed: u64,
    /// 注册期拒收次数（残废四桶 + 单活跃候选纪律）。
    pub registration_rejected: u64,
    /// 门卫钟倒退拒收次数（内核注记现算）。
    pub retrograde_rejected: u64,
    /// 死中枢新注册次数（裁定四「给死人挂号」）。
    ///
    /// **S1 只计数不拒收**：检测与 fail-loud 拒收归 S2（#622）。计数即 S2 的接手点——
    /// 改判后本计数应恒等于 S2 的拒收数。
    pub dead_center_registrations: u64,
}

// ═══════════════════════════════════════════════════════════════════════════
// 成立档门户（裁定八；S1 只做这一档）
// ═══════════════════════════════════════════════════════════════════════════

/// 中枢死亡证明（裁定一：Success 事件即死亡证明，发中枢账登记 Broken）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterDeathCertificate {
    /// 中枢身份 = 快照转正后的四条边（临时右边成为永恒右边）。
    pub center: CenterFrame,
    /// **开具时间 = 落锤知情时**；与「中枢右边缘 = 快照右边」是两个时间，不混（裁定一）。
    pub issued_as_of: usize,
}

/// 成立档证据包（裁定八②：交易层「三类点成立」+ 全套证据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThirdPointPack {
    pub identity: RetraceKey,
    pub side: RetraceSide,
    /// 四条边快照（转正）。
    pub frame: CenterFrame,
    /// 「离开的边」= leave 笔终点。
    pub leave_end: RetracePoint,
    /// 三类点位置 = 回抽笔终点。
    pub retest_end: RetracePoint,
    /// 出生知情时。
    pub registered_as_of: usize,
    /// 落锤知情时。
    pub confirmed_as_of: usize,
    /// 死亡证明（不问迟到——054:60 延迟合法、#583 在案，照开）。
    pub death_certificate: CenterDeathCertificate,
}

// ═══════════════════════════════════════════════════════════════════════════
// 账本
// ═══════════════════════════════════════════════════════════════════════════

/// 买卖点身份账本。
///
/// **真相是 `journal`**（append-only 修订记录，裁定六）；`book` 与 `active_by_anchor` 是它的
/// 折叠投影，由 [`RetraceLedger::assert_invariants`] 逐轮钉死（`fold(journal) == book`）。
#[derive(Debug, Clone, PartialEq)]
pub struct RetraceLedger {
    pub(super) book: LedgerBook<RetracePolicy>,
    pub(super) journal: Vec<RetraceRecord>,
    /// per-中枢锚的当下候选（路由投影，裁定二）。
    pub(super) active_by_anchor: BTreeMap<CenterAnchor, RetraceKey>,
    pub(super) late_absorbed: u64,
    pub(super) registration_rejected: u64,
    pub(super) dead_center_registrations: u64,
    pub(super) provenance: RetraceProvenance,
}

impl RetraceLedger {
    pub fn new(provenance: RetraceProvenance) -> Self {
        Self {
            book: LedgerBook::new(),
            journal: Vec::new(),
            active_by_anchor: BTreeMap::new(),
            late_absorbed: 0,
            registration_rejected: 0,
            dead_center_registrations: 0,
            provenance,
        }
    }

    // ── 观察进 ──

    /// 唯一推进入口：残废四桶拒收 → 注册 / 准入 → 判决。
    pub fn observe(&mut self, input: &RetraceInput) -> Result<RetraceStep, RetraceRejection> {
        let admitted = admit_input(input).map_err(|rejection| {
            self.registration_rejected += 1;
            rejection
        })?;
        if self.book.contains(&admitted.observation.key) {
            Ok(self.advance_existing(&admitted))
        } else {
            self.register_new(&admitted)
        }
    }

    /// 首次注册：单活跃候选纪律 → 建项 → 钉快照 → Restart 谱系 → 判决。
    fn register_new(
        &mut self,
        admitted: &AdmittedObservation,
    ) -> Result<RetraceStep, RetraceRejection> {
        let observation = admitted.observation;
        let key = observation.key;
        self.guard_single_active(key)?;
        let mut delta = LedgerDelta::new();
        let opened = self
            .book
            .open_on_observation(&observation, observation.as_of)
            .expect("未建仓的键必然建仓成功");
        self.record(&mut delta, opened);
        let pinned = self.push(
            key,
            RetraceRevisionKind::SnapshotPinned,
            observation.as_of,
            Some(admitted.evidence),
        );
        self.record(&mut delta, pinned);
        self.link_restart(&mut delta, key, observation.as_of);
        self.judge(&mut delta, admitted);
        Ok(self.step(key, delta, None))
    }

    /// 裁定二：同一中枢同时刻至多一个活跃候选；未判完而新 departure 来 ⟹ 报错拒收。
    fn guard_single_active(&mut self, incoming: RetraceKey) -> Result<(), RetraceRejection> {
        let Some(active) = self.active_by_anchor.get(&incoming.anchor()).copied() else {
            return Ok(());
        };
        let state = self
            .book
            .get(&active)
            .expect("路由投影只指向已建仓身份")
            .state;
        if state.is_terminal() {
            return Ok(());
        }
        self.registration_rejected += 1;
        Err(RetraceRejection::ActiveCandidateNotSettled {
            active,
            incoming_departure_move_index: incoming.departure_move_index,
        })
    }

    /// Restart = 新档注册 + 谱系载荷记前任（裁定三）；前任已判胜时另记死人挂号计数（裁定四）。
    fn link_restart(
        &mut self,
        delta: &mut LedgerDelta<RetracePolicy>,
        key: RetraceKey,
        as_of: usize,
    ) {
        let Some(previous) = self.active_by_anchor.insert(key.anchor(), key) else {
            return;
        };
        if self.book.get(&previous).map(|entry| entry.state) == Some(LedgerState::Confirmed) {
            self.dead_center_registrations += 1;
        }
        let restarted = self.push(key, RetraceRevisionKind::Restarted { previous }, as_of, None);
        self.record(delta, restarted);
    }

    /// 已建仓身份的推进：倒退拒绝 / 终态吸收（+ 警报计数）/ 判决。
    fn advance_existing(&mut self, admitted: &AdmittedObservation) -> RetraceStep {
        let key = admitted.observation.key;
        let mut delta = LedgerDelta::new();
        let admission = self.book.admit(&key, admitted.observation.as_of);
        match admission {
            LedgerAdmission::RetrogradeRejected => {}
            LedgerAdmission::TerminalAbsorbed => self.late_absorbed += 1,
            LedgerAdmission::Accepted => self.judge(&mut delta, admitted),
        }
        self.step(key, delta, Some(admission))
    }

    /// 判决：`None` 结局 ⟹ 维持 `Provisional`（裁定七，永不超时处死）。
    fn judge(&mut self, delta: &mut LedgerDelta<RetracePolicy>, admitted: &AdmittedObservation) {
        let Some(outcome) = admitted.observation.outcome else {
            return;
        };
        let evidence = Some(admitted.evidence);
        let settlement = match outcome {
            RetraceOutcome::Success => LedgerSettlement {
                state: LedgerState::Confirmed,
                kind: RetraceRevisionKind::Confirmed,
                reason: None,
                evidence,
            },
            RetraceOutcome::RetestReenters => LedgerSettlement {
                state: LedgerState::Invalidated,
                kind: RetraceRevisionKind::NotConstituted {
                    reason: NotConstitutedReason::RetestReentered,
                },
                reason: Some(NotConstitutedReason::RetestReentered),
                evidence,
            },
        };
        let revision = self
            .book
            .get_mut(&admitted.observation.key)
            .expect("判决要求条目已建仓")
            .settle(settlement, admitted.observation.as_of);
        self.record(delta, revision);
    }

    // ── 写入原语（内核唯一追加点的域侧包装） ──

    fn push(
        &mut self,
        key: RetraceKey,
        kind: RetraceRevisionKind,
        as_of: usize,
        evidence: Option<RetraceEvidence>,
    ) -> RetraceRevision {
        self.book
            .get_mut(&key)
            .expect("追加修订要求条目已建仓")
            .push_revision(kind, as_of, evidence)
    }

    /// 一条修订同时进**日志（真相）**与增量返回（消费）。
    pub(super) fn record(
        &mut self,
        delta: &mut LedgerDelta<RetracePolicy>,
        revision: RetraceRevision,
    ) {
        let sequence = self.journal.len() as u64;
        self.journal.push(RetraceRecord { sequence, revision });
        delta.record(revision);
    }

    fn step(
        &self,
        key: RetraceKey,
        delta: LedgerDelta<RetracePolicy>,
        admission: Option<LedgerAdmission>,
    ) -> RetraceStep {
        RetraceStep {
            key,
            state: self.book.get(&key).expect("推进后条目必在账").state,
            delta,
            admission,
        }
    }

    // ── 只读面 ──

    pub fn provenance(&self) -> &RetraceProvenance {
        &self.provenance
    }

    pub fn journal(&self) -> &[RetraceRecord] {
        &self.journal
    }

    pub fn entry(&self, key: &RetraceKey) -> Option<&RetraceEntry> {
        self.book.get(key)
    }

    pub fn entries(&self) -> impl Iterator<Item = &RetraceEntry> {
        self.book.values()
    }

    pub fn len(&self) -> usize {
        self.book.len()
    }

    pub fn is_empty(&self) -> bool {
        self.book.is_empty()
    }

    /// per-中枢锚的当下候选身份。
    pub fn active_candidate(&self, anchor: CenterAnchor) -> Option<RetraceKey> {
        self.active_by_anchor.get(&anchor).copied()
    }

    pub fn alarms(&self) -> RetraceAlarms {
        RetraceAlarms {
            late_absorbed: self.late_absorbed,
            registration_rejected: self.registration_rejected,
            retrograde_rejected: self.book.retrograde_rejections().len() as u64,
            dead_center_registrations: self.dead_center_registrations,
        }
    }

    // ── 成立档门户（裁定八②） ──

    /// 成立档：全部 `Confirmed` 身份的证据包（按身份键序，确定性）。
    pub fn established(&self) -> Vec<ThirdPointPack> {
        self.book
            .in_state(LedgerState::Confirmed)
            .into_iter()
            .map(Self::pack_of)
            .collect()
    }

    /// 单个身份的成立档证据包；非 `Confirmed` ⟹ `None`。
    pub fn established_pack(&self, key: &RetraceKey) -> Option<ThirdPointPack> {
        self.book
            .get(key)
            .filter(|entry| entry.state == LedgerState::Confirmed)
            .map(Self::pack_of)
    }

    /// 死亡证明查询（**留给 S2 的钩子**：死人挂号拒收据此自查，裁定四）。
    pub fn death_certificate(&self, anchor: CenterAnchor) -> Option<CenterDeathCertificate> {
        self.book
            .in_state(LedgerState::Confirmed)
            .into_iter()
            .find(|entry| entry.key.anchor() == anchor)
            .map(|entry| Self::certificate_of(entry))
    }

    fn certificate_of(entry: &RetraceEntry) -> CenterDeathCertificate {
        CenterDeathCertificate {
            center: entry.key.frame,
            issued_as_of: entry.terminal_as_of.expect("Confirmed 必有落锤钟"),
        }
    }

    fn pack_of(entry: &RetraceEntry) -> ThirdPointPack {
        let evidence = entry.terminal_evidence().expect("Confirmed 必有落锤证据");
        ThirdPointPack {
            identity: entry.key,
            side: evidence.side,
            frame: entry.key.frame,
            leave_end: evidence.leave_end,
            retest_end: evidence.retest_end.expect("判胜必带回抽位置（适配器 missing 桶保证）"),
            registered_as_of: entry.registered_as_of,
            confirmed_as_of: entry.terminal_as_of.expect("Confirmed 必有落锤钟"),
            death_certificate: Self::certificate_of(entry),
        }
    }

    // ── 不变量 ──

    /// 内核骨架 + 域不变量 + **日志唯一真相**（`fold(journal) == book`）。
    pub fn assert_invariants(&self) {
        self.book.assert_core_invariants();
        for entry in self.book.values() {
            Self::assert_entry_invariants(entry);
        }
        self.assert_anchor_invariants();
        self.assert_log_is_truth();
    }

    fn assert_entry_invariants(entry: &RetraceEntry) {
        let key = entry.key;
        assert_eq!(
            entry.state == LedgerState::Invalidated,
            entry.not_constituted_reason().is_some(),
            "Invalidated ⟺ 有原因码：{key:?}"
        );
        assert!(
            entry.state != LedgerState::Confirmed || entry.not_constituted_reason().is_none(),
            "正向终态无原因码：{key:?}"
        );
        assert!(
            entry.registration().is_some() || entry.revisions.len() == 1,
            "无注册快照者只可能是「建项与钉快照之间」的前缀态：{key:?}"
        );
        assert!(
            entry.restarted_from() != Some(key),
            "Restart 谱系不自环：{key:?}"
        );
        if entry.state == LedgerState::Confirmed {
            let evidence = entry.terminal_evidence().expect("Confirmed 必有落锤证据");
            assert!(evidence.retest_end.is_some(), "判胜必带回抽位置：{key:?}");
        }
    }

    /// 裁定二：每个中枢锚至多一个未决候选，且路由投影恰指向该锚最后注册的身份。
    fn assert_anchor_invariants(&self) {
        for (anchor, key) in &self.active_by_anchor {
            let entry = self.book.get(key).expect("路由投影只指向已建仓身份");
            assert_eq!(*anchor, entry.key.anchor(), "路由锚与身份锚一致：{key:?}");
        }
        let mut provisional: BTreeMap<CenterAnchor, usize> = BTreeMap::new();
        for entry in self.book.values() {
            if entry.state == LedgerState::Provisional {
                *provisional.entry(entry.key.anchor()).or_default() += 1;
            }
        }
        for (anchor, count) in provisional {
            assert_eq!(count, 1, "同一中枢至多一个未决候选：{anchor:?}");
        }
    }
}
