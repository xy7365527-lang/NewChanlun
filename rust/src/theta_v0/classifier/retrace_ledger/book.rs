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
use super::audit::{RetraceAuditEvent, RetraceAuditRecord, RetraceRejectionCode};
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

/// 警报计数（裁定六：四类拒收/警报另记 audit 流，**不进真相恢复路径**——它们不改状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetraceAlarms {
    /// 终态后同身份迟到输入静默吸收次数（显著非零 ⟹ 查 provider）。
    pub late_absorbed: u64,
    /// 注册期拒收次数（残废四桶 + 单活跃候选纪律 + 两拍证据一致性守卫，票 #622 补）。
    pub registration_rejected: u64,
    /// 门卫钟倒退拒收次数（内核注记现算）。
    pub retrograde_rejected: u64,
    /// 死中枢新注册拒收次数（裁定四「给死人挂号」）。
    ///
    /// **S2 落地拒收**（票 #622，修复影子评审 #621 MEDIUM-4）：判据统一为
    /// [`RetraceLedger::death_certificate`]（「该锚下是否存在任何 Confirmed」），在
    /// [`RetraceLedger::observe`] 的注册前置检查中直接 fail-loud 拒收——本计数与实际拒收次数
    /// 恒等（同一次自增即同一次拒收，S1 时代「两查法不等价」的漏计已随统一查法消失）。
    pub dead_center_registrations: u64,
    /// 引擎改口处死次数（裁定一：窗口变 / 中枢从 provider 消失 ⟹ `NotConstituted{CenterRebased}`，
    /// 票 #622）。**不进四类 audit 流**——这是真实终态事件，走 [`super::log`] 的修订日志本体，
    /// 本计数只是工程侧的快速可观测面。
    pub center_rebased: u64,
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
///
/// **两拍口径**（影子评审 #621 MEDIUM-2 如实声明）：`frame`/`identity` 取自**注册拍**（frame 是
/// 身份一部分），`side`/`leave_end`/`retest_end` 取自**落锤拍**（终态证据载荷）——同一证据包
/// 内部横跨两拍。真实引擎两拍同值（leave 笔已走完才有 departure，既成事实）；provider 两拍
/// 改口的一致性守卫（裁定二 fail-loud 同族）上浮 S2 待裁，S1 无守卫如实声明。
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
/// 折叠投影，由 [`RetraceLedger::assert_invariants`] 逐轮钉死（`fold(journal) == book`，
/// 拆分口径见 `log` 模块头）。
#[derive(Debug, Clone, PartialEq)]
pub struct RetraceLedger {
    pub(super) book: LedgerBook<RetracePolicy>,
    pub(super) journal: Vec<RetraceRecord>,
    /// per-中枢锚的当下候选（路由投影，裁定二）。
    pub(super) active_by_anchor: BTreeMap<CenterAnchor, RetraceKey>,
    pub(super) late_absorbed: u64,
    pub(super) registration_rejected: u64,
    pub(super) dead_center_registrations: u64,
    pub(super) center_rebased: u64,
    /// 四类警报的 append-only 记录（裁定六：审计归审计，`fold` 不吃本字段）。
    pub(super) audit_log: Vec<RetraceAuditRecord>,
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
            center_rebased: 0,
            audit_log: Vec::new(),
            provenance,
        }
    }

    /// 追加一条 audit 事件（四类警报统一入口，序号即产生序）。
    fn push_audit(&mut self, event: RetraceAuditEvent) {
        let sequence = self.audit_log.len() as u64;
        self.audit_log.push(RetraceAuditRecord { sequence, event });
    }

    // ── 观察进 ──

    /// 唯一推进入口：残废四桶拒收 → 注册 / 准入 → 判决。
    pub fn observe(&mut self, input: &RetraceInput) -> Result<RetraceStep, RetraceRejection> {
        let admitted = admit_input(input).map_err(|rejection| {
            self.registration_rejected += 1;
            self.push_audit(RetraceAuditEvent::ResidualRejected {
                as_of: input.as_of,
                code: adapter_rejection_code(&rejection),
            });
            rejection
        })?;
        if self.book.contains(&admitted.observation.key) {
            self.advance_existing(&admitted)
        } else {
            self.register_new(&admitted)
        }
    }

    /// 首次注册：死人挂号前置拒收 → 单活跃候选纪律（含引擎改口处死）→ 建项 → 钉快照 →
    /// Restart 谱系 → 判决。
    fn register_new(
        &mut self,
        admitted: &AdmittedObservation,
    ) -> Result<RetraceStep, RetraceRejection> {
        let observation = admitted.observation;
        let key = observation.key;
        let mut delta = LedgerDelta::new();
        self.guard_single_active(key, observation.as_of, &mut delta)?;
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

    /// 裁定四：死人挂号前置拒收（自查 [`Self::death_certificate`]，不依赖外部）→
    /// 裁定二/裁定一：同一中枢同时刻至多一个活跃候选——同 departure 而窗口变 ⟹ 引擎改口处死
    /// （旧候选进 [`kill_as_rebased`](Self::kill_as_rebased)，本次注册继续）；不同 departure 而
    /// 旧候选未判完 ⟹ provider 有病，报错拒收。
    fn guard_single_active(
        &mut self,
        incoming: RetraceKey,
        as_of: usize,
        delta: &mut LedgerDelta<RetracePolicy>,
    ) -> Result<(), RetraceRejection> {
        if let Some(certificate) = self.death_certificate(incoming.anchor()) {
            self.dead_center_registrations += 1;
            self.push_audit(RetraceAuditEvent::DeadCenterRejected {
                anchor: incoming.anchor(),
                attempted: incoming,
                death_issued_as_of: certificate.issued_as_of,
                as_of,
            });
            return Err(RetraceRejection::DeadCenterReentry {
                anchor: incoming.anchor(),
                attempted: incoming,
                death_certificate: certificate,
            });
        }
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
        if incoming.departure_move_index == active.departure_move_index {
            // 同一 departure、不同窗口（键不同故走到本分支）⟹ 引擎改口（裁定一：活着期间每次
            // 观察比对窗口，偏离 ⟹ 处死记档）。**不算倒退**（裁定五），无论 as_of 相对旧候选如何。
            self.kill_as_rebased(delta, active, Some(incoming.frame), as_of);
            return Ok(());
        }
        self.registration_rejected += 1;
        self.push_audit(RetraceAuditEvent::ResidualRejected {
            as_of,
            code: RetraceRejectionCode::ActiveCandidateNotSettled,
        });
        Err(RetraceRejection::ActiveCandidateNotSettled {
            active,
            incoming_departure_move_index: incoming.departure_move_index,
        })
    }

    /// 引擎改口处死：活跃候选 Provisional → Invalidated{CenterRebased}，证据带新旧窗口对照
    /// （沿用注册快照的侧/离开边，无买卖判定语义——处死不是判案）。
    fn kill_as_rebased(
        &mut self,
        delta: &mut LedgerDelta<RetracePolicy>,
        key: RetraceKey,
        observed_window: Option<CenterFrame>,
        as_of: usize,
    ) {
        let evidence = self.book.get(&key).and_then(|entry| entry.registration());
        let reason = NotConstitutedReason::CenterRebased {
            registered_window: key.frame,
            observed_window,
        };
        let settlement = LedgerSettlement {
            state: LedgerState::Invalidated,
            kind: RetraceRevisionKind::NotConstituted { reason },
            reason: Some(reason),
            evidence,
        };
        let entry = self.book.get_mut(&key).expect("活跃候选必在账");
        if as_of > entry.last_as_of {
            entry.last_as_of = as_of;
        }
        let revision = entry.settle(settlement, as_of);
        self.record(delta, revision);
        self.center_rebased += 1;
    }

    /// per-中枢锚窗口核对（裁定一「活着期间每次观察比对当前中枢窗口与注册快照」的显式通道）：
    /// 供引擎在**无新 departure** 时汇报当前窗口——`current_window = None` 即「中枢从 provider
    /// 消失」。无活跃候选 / 候选已终态 ⟹ 零动作；窗口同注册快照 ⟹ **同窗口重确认零动作**
    /// （裁定一，连门卫钟都不动——真是零动作，不是零输出）；偏离 ⟹ 处死记档进警报桶。
    pub fn reconcile_window(
        &mut self,
        anchor: CenterAnchor,
        current_window: Option<CenterFrame>,
        as_of: usize,
    ) -> Option<RetraceStep> {
        let active = self.active_by_anchor.get(&anchor).copied()?;
        let entry = self.book.get(&active).expect("路由投影只指向已建仓身份");
        if entry.state.is_terminal() || current_window == Some(active.frame) {
            return None;
        }
        let mut delta = LedgerDelta::new();
        self.kill_as_rebased(&mut delta, active, current_window, as_of);
        Some(self.step(active, delta, None))
    }

    /// Restart = 新档注册 + 谱系载荷记前任（裁定三）：anchor 上曾有过任何前任（无论因判败重回
    /// 还是引擎改口而不再活跃）都记录谱系——死人挂号的前任不会走到这里（`guard_single_active`
    /// 已在死亡证明命中时前置拒收，本函数运行时该锚绝无 Confirmed 前任）。
    fn link_restart(
        &mut self,
        delta: &mut LedgerDelta<RetracePolicy>,
        key: RetraceKey,
        as_of: usize,
    ) {
        let Some(previous) = self.active_by_anchor.insert(key.anchor(), key) else {
            return;
        };
        let restarted = self.push(key, RetraceRevisionKind::Restarted { previous }, as_of, None);
        self.record(delta, restarted);
    }

    /// 已建仓身份的推进：两拍证据一致性守卫（零改写）→ 倒退拒绝 / 终态吸收（+ 警报）/ 判决。
    fn advance_existing(
        &mut self,
        admitted: &AdmittedObservation,
    ) -> Result<RetraceStep, RetraceRejection> {
        let key = admitted.observation.key;
        if admitted.observation.outcome.is_some() {
            self.guard_terminal_evidence_consistency(key, admitted)?;
        }
        let mut delta = LedgerDelta::new();
        let admission = self.book.admit(&key, admitted.observation.as_of);
        match admission {
            LedgerAdmission::RetrogradeRejected => {
                let entry = self.book.get(&key).expect("准入要求条目已建仓");
                self.push_audit(RetraceAuditEvent::RetrogradeRejected {
                    key,
                    last_as_of: entry.last_as_of,
                    rejected_as_of: admitted.observation.as_of,
                });
            }
            LedgerAdmission::TerminalAbsorbed => {
                self.late_absorbed += 1;
                let entry = self.book.get(&key).expect("准入要求条目已建仓");
                self.push_audit(RetraceAuditEvent::LateAbsorbed {
                    key,
                    terminal_as_of: entry.terminal_as_of.expect("终态吸收必有落锤钟"),
                    late_as_of: admitted.observation.as_of,
                });
            }
            LedgerAdmission::Accepted => self.judge(&mut delta, admitted),
        }
        Ok(self.step(key, delta, Some(admission)))
    }

    /// 两拍证据一致性守卫（裁定二 fail-loud 同族；影子评审 #621 MEDIUM-2 补，票 #622）：
    /// 落锤拍的侧 / 离开边须与注册拍一致，否则 provider 两拍改口 ⟹ 报错拒收（零改写——检查发生
    /// 在 `admit` 之前）。前缀态（尚无注册快照）无从比对，放行给后续路径自然处理。
    fn guard_terminal_evidence_consistency(
        &mut self,
        key: RetraceKey,
        admitted: &AdmittedObservation,
    ) -> Result<(), RetraceRejection> {
        let Some(registered) = self.book.get(&key).and_then(|entry| entry.registration()) else {
            return Ok(());
        };
        if registered.side == admitted.evidence.side && registered.leave_end == admitted.evidence.leave_end {
            return Ok(());
        }
        self.registration_rejected += 1;
        self.push_audit(RetraceAuditEvent::ResidualRejected {
            as_of: admitted.observation.as_of,
            code: RetraceRejectionCode::TerminalEvidenceContradictsRegistration,
        });
        Err(RetraceRejection::TerminalEvidenceContradictsRegistration {
            key,
            registered_side: registered.side,
            registered_leave_end: registered.leave_end,
            incoming_side: admitted.evidence.side,
            incoming_leave_end: admitted.evidence.leave_end,
        })
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
            center_rebased: self.center_rebased,
        }
    }

    /// Audit 流只读面（裁定六：四类警报的 append-only 记录，不进真相恢复路径）。
    pub fn audit_log(&self) -> &[RetraceAuditRecord] {
        &self.audit_log
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

    /// 内核骨架 + 域不变量 + **日志唯一真相**（`fold(journal) == book`，拆分口径见 `log` 模块头）。
    pub fn assert_invariants(&self) {
        self.book.assert_core_invariants();
        for entry in self.book.values() {
            Self::assert_entry_invariants(entry);
        }
        self.assert_anchor_invariants();
        self.assert_dead_center_invariant();
        self.assert_at_most_one_confirmed_per_anchor();
        self.assert_log_is_truth();
    }

    /// 死人挂号拒收落地后的不可达状态（票 #622）：一旦某锚有死亡证明，该锚不可能再有未决候选
    /// ——任何试图挂号的新 departure 都在 `guard_single_active` 前置拒收，从未建仓。
    fn assert_dead_center_invariant(&self) {
        for entry in self.book.values() {
            if entry.state != LedgerState::Provisional {
                continue;
            }
            assert!(
                self.death_certificate(entry.key.anchor()).is_none(),
                "死人挂号拒收后不可达：{:?} 的锚已有死亡证明，仍有未决候选",
                entry.key
            );
        }
    }

    /// 死人挂号拒收的推论（订正影子评审 #621 LOW-2）：同一中枢锚至多一个 `Confirmed`——
    /// 拒收落地后再无法产出第二个，死亡证明查询的键序取首个不再有歧义。
    fn assert_at_most_one_confirmed_per_anchor(&self) {
        let mut seen: BTreeMap<CenterAnchor, RetraceKey> = BTreeMap::new();
        for entry in self.book.in_state(LedgerState::Confirmed) {
            let anchor = entry.key.anchor();
            if let Some(existing) = seen.insert(anchor, entry.key) {
                panic!("同一中枢锚至多一个 Confirmed：{anchor:?} 同时有 {existing:?} 与 {:?}", entry.key);
            }
        }
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
        // 判败侧镜像（影子评审 #623 S3 MEDIUM-2 订正）：Confirmed 一侧有上面这条不变量兜底
        // 「判胜必带回抽位置」，`Invalidated{RetestReentered}` 一侧此前零断言——`portal.rs` 的
        // `short_retrace_record_of` 对同一前提用 `expect`，活路上不可达（适配器挡死），但重放
        // 路径不挡（`log.rs` 声明的在案敞口：JSONL 落盘后被外部改写）。补齐后，篡改在这里
        // fail-loud，而不是留到 `short_retrace_records()` 这个只读派生函数才 panic。
        if entry.not_constituted_reason() == Some(NotConstitutedReason::RetestReentered) {
            let evidence = entry
                .terminal_evidence()
                .expect("Invalidated{RetestReentered} 必有落锤证据");
            assert!(evidence.retest_end.is_some(), "判败必带回抽位置：{key:?}");
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

/// [`RetraceRejection`] → [`RetraceRejectionCode`]（audit 流轻量标签，`observe` 的
/// `admit_input` 错误映射点专用；`ActiveCandidateNotSettled`/`DeadCenterReentry`/
/// `TerminalEvidenceContradictsRegistration` 由本模块直接产出、直接自选对应码，不经此函数）。
fn adapter_rejection_code(rejection: &RetraceRejection) -> RetraceRejectionCode {
    match rejection {
        RetraceRejection::MalformedFrame { .. } => RetraceRejectionCode::MalformedFrame,
        RetraceRejection::MissingRetestPosition { .. } => RetraceRejectionCode::MissingRetestPosition,
        RetraceRejection::LeaveNotOutsideCenter { .. } => RetraceRejectionCode::LeaveNotOutsideCenter,
        RetraceRejection::RetestSameDirection { .. } => RetraceRejectionCode::RetestSameDirection,
        RetraceRejection::NotAdjacent { .. } => RetraceRejectionCode::NotAdjacent,
        RetraceRejection::ActiveCandidateNotSettled { .. } => {
            RetraceRejectionCode::ActiveCandidateNotSettled
        }
        RetraceRejection::DeadCenterReentry { .. } => {
            unreachable!("死人挂号走独立 DeadCenterRejected 事件，admit_input 不产出本变体")
        }
        RetraceRejection::TerminalEvidenceContradictsRegistration { .. } => {
            unreachable!("两拍守卫走 guard_terminal_evidence_consistency 自选码，admit_input 不产出本变体")
        }
    }
}
