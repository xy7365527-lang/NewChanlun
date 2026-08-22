//! NestLifecycleBook 注册表（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。
//!
//! ★声明锚（#454 验收）：三态不变量断言集（#299 P0a）随
//! [`NestLifecycleBook::assert_invariants`] 迁至本文件——该方法是断言集的唯一公开入口，
//! 账本级骨架 `assert_core_invariants` 仍留在 `ledger_kernel`。

use super::entry::MigrationKind;
use super::key::same_anchor;
use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// NestLifecycleBook（sidecar 注册表）
// ═══════════════════════════════════════════════════════════════════════════

/// sidecar 注册表（范式复用：strategy/persistent.rs Pi 注册表与 `PersistentElement`、
/// recursive_tower.rs `CpScanOwnership` 生命周期挂 book 内对象 + 显式推进函数、
/// p92 YieldBook first-write-wins `entry().or_insert()`）。
///
/// 事件流零改动（T5 bit-exact 护栏）；状态只活在本 entry。**消费边界**（裁定 #64 §2(a)）：
/// 本 book 是观察记录——不进 `d_parent_interval_snapshot/terminal` 输入、不改
/// `divergence_confirmed` 布尔口径、N^δ 装配仍只消费已闭合完整 c_p。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestLifecycleBook {
    /// 账本内核（票 #573 T1）：per-key 注册表 + 倒退拒绝注记面。11 个对象无关责任点
    /// （建项 / 追加 / 计数一致 / 倒退拒绝 / 终态吸收 / 钟首写 / 增量 / 迁移 / 枚举 /
    /// 不变量骨架）全在此，本模块只加自己的判据与域结算。
    ///
    /// 倒退拒绝注记随内核走：同一倒退喂入重复执行重复记录（重复违规事实本身；与
    /// ForceUnavailable 同 as_of 幂等不对称——原 #78 评审 Low 登记同判，不阻塞）。
    ledger: LedgerBook<NestPolicy>,
    /// 数据源真实首完成信号（按桥身份唯一；与终态独立，禁用终态冒充「已经完成」）。
    completion_signals: Vec<CompletionSignal>,
    /// 完成信号与力度不可验同时发生的独立审计面（#428；按桥身份唯一）。
    completion_force_unavailable_audits: Vec<CompletionForceUnavailableAudit>,
}

impl NestLifecycleBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.ledger.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ledger.is_empty()
    }

    /// entry 读面（TDD 接缝外部行为断言面）。
    pub fn get(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        self.ledger.get(key)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&LifecycleKey, &NestLifecycleEntry)> {
        self.ledger.entries()
    }

    /// 倒退拒绝注记门户（#78 修复 2 审计面）。
    pub fn retrograde_rejections(&self) -> &[RetrogradeRejection] {
        self.ledger.retrograde_rejections()
    }

    /// 数据源真实首完成信号门户（按桥身份唯一；独立于状态终局）。
    pub fn completion_signals(&self) -> &[CompletionSignal] {
        &self.completion_signals
    }

    /// 完成信号已到但力度不可验的审计门户（只读，不参与状态判定）。
    pub fn completion_force_unavailable_audits(&self) -> &[CompletionForceUnavailableAudit] {
        &self.completion_force_unavailable_audits
    }

    /// 汇总终局分布与寿命；闪现只计终局钟与 `observed_at` 同刻的身份。
    pub fn settlement_stats(&self) -> LifecycleSettlementStats {
        let mut stats = LifecycleSettlementStats {
            entry_count: self.ledger.len(),
            ..LifecycleSettlementStats::default()
        };
        let mut nonflash_lifetimes = Vec::new();
        let mut force_overtake_lifetimes = Vec::new();
        // 票 #603 档 1：被 `CenterUpgraded` 认领的前身键集合。来源取 **append-only 修订链里
        // 每一条 `CenterUpgraded` 自带的 `from`**（票 #619 H1 订正）——不经 `superseded_from`
        // 中转：后者是「当下来源指针」，认领方只要多活一个 bar 被桥迁移（dump 实证桥迁移逐
        // bar 发生），该指针即被改写为「上一 bar 的自己」，认领关联静默丢失 ⟹ 本口径少计。
        // 取全部 `from`（不止首条）：同一 entry 可先留痕认领终态前身、再逐 bar 迁移，迁移写
        // 入的 `from` 是自己的旧键——旧键已被 `remove`、不在 `entries` 内，`claimed.contains`
        // 恒不命中，不污染口径。账本自足，不另存状态。
        let claimed: std::collections::BTreeSet<LifecycleKey> = self
            .ledger
            .values()
            .flat_map(|entry| entry.revisions.iter())
            .filter_map(|revision| match revision.kind {
                LifecycleRevisionKind::CenterUpgraded { from } => Some(from),
                _ => None,
            })
            .collect();
        for entry in self.ledger.values() {
            stats.first_provable_count += usize::from(entry.first_provable_at.is_some());
            let (terminal_at, force_overtake) = match entry.state {
                NestEventState::Provisional => {
                    stats.provisional_count += 1;
                    (None, false)
                }
                NestEventState::Confirmed => {
                    stats.confirmed_count += 1;
                    (entry.confirmed_at, false)
                }
                NestEventState::Invalidated => {
                    match entry.invalidated_reason.expect("Invalidated 必有原因码") {
                        InvalidatedReason::ForceOvertake => {
                            stats.force_overtake_count += 1;
                            stats.force_overtake_claimed_count +=
                                usize::from(claimed.contains(&entry.key));
                            (entry.invalidated_at, true)
                        }
                        InvalidatedReason::NeverConstituted => {
                            stats.never_constituted_count += 1;
                            (entry.invalidated_at, false)
                        }
                        InvalidatedReason::IdentityVanished { cause } => {
                            match cause {
                                VanishCause::HypothesisRefuted => {
                                    stats.identity_vanished_refuted_count += 1
                                }
                                VanishCause::ObservationSeam { .. } => {
                                    stats.identity_vanished_seam_count += 1
                                }
                            }
                            (entry.invalidated_at, false)
                        }
                    }
                }
            };
            let Some(terminal_at) = terminal_at else {
                continue;
            };
            let lifetime = terminal_at
                .checked_sub(entry.observed_at)
                .expect("终局钟不得早于 observed_at");
            if lifetime == 0 {
                stats.flash_terminal_count += 1;
            } else {
                nonflash_lifetimes.push(lifetime);
            }
            if force_overtake {
                force_overtake_lifetimes.push(lifetime);
            }
        }
        stats.nonflash_lifetime = LifetimeDistribution::from_values(nonflash_lifetimes);
        stats.force_overtake_lifetime = LifetimeDistribution::from_values(force_overtake_lifetimes);
        stats
    }

    /// 消费侧 Closed-only（裁定 #64 §2(a)「消费不放开」）：只放 Confirmed。
    /// Provisional/Invalidated 永不经本门户离开 book——Invalidated 可查账
    /// （entries/revisions 全程留档），不开放消费（模块头 090 登记 4）。
    pub fn consumable_closed(&self) -> Vec<&NestLifecycleEntry> {
        self.ledger.in_state(NestEventState::Confirmed)
    }

    /// 谱系构建节点读面（裁定 #64 §2(a)「构建放开」）。
    ///
    /// ★红线：本读面仅供谱系**构建**侧；**禁止**出现在任何
    /// `d_parent_interval_snapshot`/装配候选/证书真值路径（nest.rs:802-810/:987-989
    /// 装配输入不变；白名单工程桥不得进证书真值路径——裁定 §2(c) 裁定义务）。
    pub fn lineage_nodes(&self) -> Vec<&NestLifecycleEntry> {
        self.ledger.values().collect()
    }

    /// 推进一 prefix（卡 §2.3 转移表 + #78 修复全量）。返回本 prefix 新产出修订
    /// （终态吸收/幂等/拒绝 = 空）。步骤：
    /// 1. 建仓 / 白名单桥迁移（Supersedes）/ 终态吸收（含桥匹配到终态）；桥匹配分支自带
    ///    倒退守卫（被拒 key 尚未建仓，T18 锁定）；
    /// 2. 倒退守卫（per-identity last_as_of，显式拒绝 + 注记；直接匹配分支，T15 锁定）；
    /// 3. 终态吸收（禁复活）；
    /// 4. 力度求值（事件通道恒 Verified；活窗三值化）；
    /// 5. first_provable 首次写入（仅 Verified(true)）；
    /// 6. 反超判负（仅 Verified(false) ⟹ Invalidated(ForceOvertake)）/
    ///    Unavailable 审计注记（不判 Invalidated、不进确认）；
    /// 7. 完成时复核（本 prefix 现算 force，禁「曾经弱过」冒充，E2E §4.1:151）：曾可证 ∧
    ///    仍弱 ⟹ Confirmed；从未可证 ⟹ Invalidated(NeverConstituted)（061:28，ADR-0003）；
    /// 8. 身份消失扫描（倒退 prefix 不制造 IdentityVanished）。
    pub fn advance(
        &mut self,
        observations: &[LifecycleObservation],
        as_of: usize,
        material: &ForceMaterial,
    ) -> Vec<LifecycleRevision> {
        let mut delta = LedgerDelta::new();
        let mut seen = BTreeSet::new();
        for obs in observations {
            let key = obs.key();
            seen.insert(key);
            // 第 1 步：建仓 / 白名单桥迁移 / 终态吸收（含桥匹配到终态）。
            if !self.ledger.contains(&key) && !self.open_or_migrate(key, obs, as_of, &mut delta) {
                continue;
            }
            // 第 2/3 步：倒退守卫（per-identity last_as_of）+ 终态吸收（禁复活）。
            match self.ledger.admit(&key, as_of) {
                LedgerAdmission::RetrogradeRejected => continue,
                LedgerAdmission::TerminalAbsorbed => continue,
                LedgerAdmission::Accepted => {}
            }
            let entry = self.ledger.get_mut(&key).expect("entry 已建仓");
            // 第 4/5 步：力度求值 + first_provable 首次写入（仅 Verified(true)）。
            let force = obs.force(material);
            if force == ForceCheck::Verified(true)
                && first_write_clock(&mut entry.first_provable_at, as_of)
            {
                let revision =
                    entry.push_revision(LifecycleRevisionKind::FirstProvable, as_of, None);
                delta.record(revision);
            }
            // 第 6 步：反超判负 / Unavailable 审计注记。
            if !self.apply_force(obs, key, force, as_of, material, &mut delta) {
                continue;
            }
            // 第 7 步：完成时复核。
            if obs.structure_completed() {
                self.complete_structure(obs, key, force, as_of, material, &mut delta);
            }
        }
        // 第 8 步：身份消失扫描（倒退 prefix 不制造 IdentityVanished）。
        self.scan_vanished(&seen, as_of, &mut delta);
        // 库内 debug 构建自动核验；release 不作“自动核验”声明。生产 p123 接线在每个
        // trigger 喂入后显式调用本公开入口。
        #[cfg(debug_assertions)]
        self.assert_invariants();
        delta.into_vec()
    }

    /// `advance` 第 1 步：建仓 / 白名单桥迁移 / 终态吸收（含桥匹配到终态）。返回 `false`
    /// 表示本轮观察已被吸收或拒绝（调用方 `continue`），`true` 表示可继续推进。
    fn open_or_migrate(
        &mut self,
        key: LifecycleKey,
        obs: &LifecycleObservation,
        as_of: usize,
        delta: &mut LedgerDelta<NestPolicy>,
    ) -> bool {
        match self.bridge_match(&key) {
            Some(old_key) => {
                let old = self.ledger.get(&old_key).expect("桥匹配键在册");
                let (old_last_as_of, old_terminal) = (old.last_as_of, old.state.is_terminal());
                // 同一身份的倒退喂入：显式拒绝（不迁移、不建仓、零 revision）。
                if as_of < old_last_as_of {
                    self.ledger.reject_retrograde(key, old_last_as_of, as_of);
                    return false;
                }
                // 终态吸收含桥匹配：终态不迁移、不建仓、零输出（禁复活，E2E §1:83）。
                if old_terminal {
                    return false;
                }
                // Supersedes 迁移（仅 Provisional 可达——终态已在上一步吸收）：
                // 钟不动、链留痕（单一写入点 [`Self::migrate_entry`]）。
                let revision = self.migrate_entry(old_key, key, MigrationKind::Bridge, as_of);
                delta.record(revision);
            }
            None => {
                // 档 1（票 #603 / #599 裁定）：桥未命中 ⟹ 试中枢升级认领（严格同锚，
                // 判据 [`bridge_by_center_upgrade`]）。**前身条目一律不改任何 bit**，
                // 两形态由前身死活决定：
                //  - 前身仍 Provisional 且非倒退 ⟹ **迁移**（继承五钟，与 Supersedes 同款）；
                //  - 前身已终态 / 倒退喂入 ⟹ **认领留痕**（新身份独立建仓 + 关联修订，
                //    前身原样留档——终态吸收/禁复活/终态钟只写一次三条不变量不动）。
                // 优选由 `center_upgrade_match` 一并给出（票 #619 H2：留痕形态把终态
                // 前身留在册 ⟹ 同链可**匹配**多只，必须择优而非按字节序盲取）。
                let claim = self.center_upgrade_match(&key, as_of);
                match claim {
                    (Some(old_key), true) => {
                        // 迁移（钟不动、链留痕、无 Invalidated；单一写入点）。
                        let revision =
                            self.migrate_entry(old_key, key, MigrationKind::CenterUpgrade, as_of);
                        delta.record(revision);
                    }
                    (claimed_from, _) => {
                        // 新身份建仓（内核首次观察建项：Provisional + 观察钟/门卫钟同取
                        // as_of + 一条 `Observed` 建项修订；observed_at 建仓写一次、无任何
                        // 改写点。倒退 as_of 的新身份无基线可违照建——#78 评审信息项，
                        // observed_at ≤ last_as_of 不受损）。`claimed_from` 有值 ⟹ 紧跟
                        // 一条认领留痕修订。
                        let revision = self
                            .ledger
                            .open_on_observation(obs, as_of)
                            .expect("建仓分支的身份此前不在册");
                        delta.record(revision);
                        if let Some(old_key) = claimed_from {
                            let entry = self.ledger.get_mut(&key).expect("本轮刚建仓的身份在册");
                            entry.set_migrated_from(old_key);
                            let claim_revision = entry.push_revision(
                                LifecycleRevisionKind::CenterUpgraded { from: old_key },
                                as_of,
                                None,
                            );
                            delta.record(claim_revision);
                        }
                    }
                }
            }
        }
        true
    }

    /// `advance` 第 6 步：反超判负 / Unavailable 审计注记。返回 `false` 表示本轮观察已经
    /// 反超或被 Unavailable 注记接住（调用方 `continue`），`true` 表示可进入第 7 步。
    ///
    /// 反超只要求曾可证；结构完成不是前置条件，因此这里合法产生
    /// Provisional→ForceOvertake 第二终局路径（Q3）。
    fn apply_force(
        &mut self,
        obs: &LifecycleObservation,
        key: LifecycleKey,
        force: ForceCheck,
        as_of: usize,
        material: &ForceMaterial,
        delta: &mut LedgerDelta<NestPolicy>,
    ) -> bool {
        let entry = self.ledger.get_mut(&key).expect("entry 已建仓");
        match force {
            ForceCheck::Verified(false) => {
                if entry.first_provable_at.is_some() {
                    // 反超（061:26）：曾可证 ∧ 当前不再弱 ⟹ Invalidated(ForceOvertake)。
                    let evidence = obs.force_evidence(material);
                    let revision =
                        entry.invalidate(InvalidatedReason::ForceOvertake, as_of, evidence);
                    delta.record(revision);
                    return false;
                }
            }
            ForceCheck::Unavailable(reason) => {
                // 「不可验」≠「不再弱」：只记审计注记（按桥身份唯一），不判
                // Invalidated、不写 first_provable、不进完成确认（entry 保持原态）。
                // audit 按完成信号桥身份唯一；provider 跨 trigger 重发不重复抬高发生率分子。
                if obs.structure_completed()
                    && !self
                        .completion_force_unavailable_audits
                        .iter()
                        .any(|audit| audit.key == key || bridge_identity(&audit.key, &key))
                {
                    self.completion_force_unavailable_audits
                        .push(CompletionForceUnavailableAudit { key, as_of, reason });
                }
                if entry.force_unavailable_at != Some(as_of) {
                    entry.force_unavailable_at = Some(as_of);
                    let revision = entry.push_revision(
                        LifecycleRevisionKind::ForceUnavailable { reason },
                        as_of,
                        None,
                    );
                    delta.record(revision);
                }
                return false;
            }
            ForceCheck::Verified(true) => {}
        }
        true
    }

    /// `advance` 第 7 步：完成时复核（024:24——同一谓词在完成窗上重算为真才结算；禁
    /// 「曾经弱过」冒充，E2E §4.1:151；复核用本 prefix 现算 force，不沿用旧值）。
    fn complete_structure(
        &mut self,
        obs: &LifecycleObservation,
        key: LifecycleKey,
        force: ForceCheck,
        as_of: usize,
        material: &ForceMaterial,
        delta: &mut LedgerDelta<NestPolicy>,
    ) {
        let entry = self.ledger.get_mut(&key).expect("entry 已建仓");
        if first_write_clock(&mut entry.structure_end_at, as_of) {
            let revision =
                entry.push_revision(LifecycleRevisionKind::StructureCompleted, as_of, None);
            delta.record(revision);
        }
        if entry.first_provable_at.is_some() && force == ForceCheck::Verified(true) {
            let revision = entry.confirm(as_of);
            delta.record(revision);
        } else if entry.first_provable_at.is_none() {
            // 从未构成（061:28「因为背驰如果没有创新高，是不存在的」，ADR-0003）：
            // 结构完成时从未写入 first_provable ⟹ 该假设根本未构成，转终态挂
            // NeverConstituted，不再以活假设身份挂账。**与 ForceOvertake 严格互补**
            // ——后者要求「曾可证」（061:26 曾构成后被否证），此处恒无（T17 分辨）。
            // 力度证据同 ForceOvertake 现算入载荷（090 登记 4 可查账）。
            // 到达本臂时 force 必为 Verified(false)：Verified(true) 已在第 5 步写入
            // first_provable（走上一臂），Unavailable 已在第 6 步 continue。
            // **残留登记**：完成 prefix 上力度不可验时
            // 第 6 步先 continue ⟹ 本臂不到达、该身份滞留活假设（#78「不可验 ≠ 不再
            // 弱」优先——宁可推迟结算，不从缺失数据造否证；T19 锁定）。数据补齐后的
            // 完成信号照常结算；始终不补则永久滞留。
            let evidence = obs.force_evidence(material);
            let revision = entry.invalidate(InvalidatedReason::NeverConstituted, as_of, evidence);
            delta.record(revision);
        }
        // 完成观察已反超者已被第 6 步接住（Invalidated(ForceOvertake) 而非
        // Confirmed，confirmed_at 保持 None，T7(b) 锁定）；若是此前活窗先反超，
        // 则合法绕过 StructureCompleted——两条否证路径不互相冒充。
    }

    /// `advance` 第 8 步：身份消失扫描（上一 prefix 有、本 prefix 不再产出该 key ⟹
    /// Invalidated；倒退 prefix 不制造 IdentityVanished——last_as_of > as_of 的身份跳过）。
    fn scan_vanished(
        &mut self,
        seen: &BTreeSet<LifecycleKey>,
        as_of: usize,
        delta: &mut LedgerDelta<NestPolicy>,
    ) {
        let vanished: Vec<LifecycleKey> = self
            .ledger
            .entries()
            .filter(|(key, entry)| {
                entry.state == NestEventState::Provisional
                    && entry.last_as_of <= as_of
                    && !seen.contains(key)
            })
            .map(|(key, _)| *key)
            .collect();
        for key in vanished {
            // 成因两分（票 #559 裁定，唯一分类点）：本 prefix 观察集合里若仍有同锚身份
            // （必然 c 左端不同——同左端已被桥吸收成 Supersedes，不进本扫描），则该锚的
            // 假设未死，是 provider 换轨丢下旧 key ⟹ 观测接缝伪影；否则该锚本 prefix
            // 完全不再产窗 ⟹ 假设被推翻。判据只读本 prefix 的 `seen`，不做历史重建。
            let cause = seen
                .iter()
                .find(|other| **other != key && same_anchor(other, &key))
                .map(|successor| VanishCause::ObservationSeam {
                    successor_c_start: successor.seg_c_full.0,
                })
                .unwrap_or(VanishCause::HypothesisRefuted);
            let entry = self.ledger.get_mut(&key).expect("扫描键存在");
            // 证据传 None：IdentityVanished 无力度语义（invariant 逐条钉死）。
            let revision =
                entry.invalidate(InvalidatedReason::IdentityVanished { cause }, as_of, None);
            delta.record(revision);
        }
    }

    /// 钟不变量显式化（review judgement call 4：公开可调用，不只靠 debug_assert）。
    ///
    /// 全列：revision 计数 == 留档长度；observed_at ≤ last_as_of；钟序
    /// observed ≤ first_provable ≤ structure_end ≤ confirmed；一切钟 ≤ last_as_of；
    /// 反超 invalidated ≥ first_provable（允许 structure_end 为空；身份消失路径独立）；终态互洽
    /// （state ⟺ 终态钟、终态互斥）；IdentityVanished 恒无力度证据；迁移链两端满足桥身份；
    /// **被反超与从未构成在 first_provable 上严格互补**（前者恒有、后者恒无），从未构成的
    /// invalidated_at == structure_end_at（票 #425）；首完成信号按桥身份唯一且不早于观察钟。
    pub fn assert_invariants(&self) {
        // 账本级骨架（票 #573 T1，对象无关）：注册表键一致、计数 == 留档、出生钟 ≤ 门卫钟、
        // 终态 ⟺ 终态钟、留档时序非降、首条修订为建项词汇、来源链不自环。域断言在其之上
        // 逐条叠加（两层互不替代——下面全部既有域断言语义一条不改）。
        self.ledger.assert_core_invariants();
        for entry in self.ledger.values() {
            let key = entry.key;
            assert_eq!(
                entry.revision as usize,
                entry.revisions.len(),
                "revision 计数 == 留档长度：{key:?}"
            );
            assert!(
                entry.observed_at <= entry.last_as_of,
                "observed_at ≤ last_as_of：{key:?}"
            );
            if let Some(first) = entry.first_provable_at {
                assert!(
                    entry.observed_at <= first && first <= entry.last_as_of,
                    "observed ≤ first_provable ≤ last_as_of：{key:?}"
                );
            }
            if let Some(structure_end) = entry.structure_end_at {
                assert!(
                    entry.observed_at <= structure_end && structure_end <= entry.last_as_of,
                    "observed ≤ structure_end ≤ last_as_of：{key:?}"
                );
                if let Some(first) = entry.first_provable_at {
                    assert!(
                        first <= structure_end,
                        "first_provable ≤ structure_end：{key:?}"
                    );
                }
            }
            // 票 #559 新不变量（全态覆盖）：`vanish_cause` 有值 ⟺ 原因码是 IdentityVanished。
            // 「凡消失必带可审计两类原因码」的账本级钉死点；其它终局/活假设恒无成因字段。
            assert_eq!(
                entry.vanish_cause.is_some(),
                matches!(
                    entry.invalidated_reason,
                    Some(InvalidatedReason::IdentityVanished { .. })
                ),
                "vanish_cause 有值 ⟺ IdentityVanished：{key:?}"
            );
            match entry.state {
                NestEventState::Provisional => {
                    assert!(
                        entry.confirmed_at.is_none() && entry.invalidated_at.is_none(),
                        "Provisional 无终态钟：{key:?}"
                    );
                }
                NestEventState::Confirmed => {
                    let confirmed = entry.confirmed_at.expect("Confirmed 必有 confirmed_at");
                    let first = entry.first_provable_at.expect("Confirmed 必先可证");
                    let structure_end = entry.structure_end_at.expect("Confirmed 必先结构完成");
                    assert!(
                        first <= structure_end
                            && structure_end <= confirmed
                            && confirmed <= entry.last_as_of,
                        "钟序 first ≤ structure_end ≤ confirmed ≤ last_as_of：{key:?}"
                    );
                    assert!(entry.invalidated_at.is_none(), "终态互斥：{key:?}");
                }
                NestEventState::Invalidated => {
                    let invalidated = entry
                        .invalidated_at
                        .expect("Invalidated 必有 invalidated_at");
                    assert!(
                        entry.observed_at <= invalidated,
                        "observed ≤ invalidated：{key:?}"
                    );
                    assert!(entry.confirmed_at.is_none(), "终态互斥：{key:?}");
                    let reason = entry.invalidated_reason.expect("Invalidated 必有原因码");
                    match reason {
                        InvalidatedReason::ForceOvertake => {
                            let first = entry
                                .first_provable_at
                                .expect("反超定义要求曾可证（卡 §4.1）");
                            assert!(
                                first <= invalidated,
                                "反超 invalidated ≥ first_provable：{key:?}"
                            );
                            // 反超发生于该身份被投喂的 prefix（last_as_of 已先推进到当 prefix）。
                            assert!(
                                invalidated <= entry.last_as_of,
                                "反超 invalidated ≤ last_as_of：{key:?}"
                            );
                        }
                        InvalidatedReason::NeverConstituted => {
                            // 从未构成（061:28）与被反超（061:26）严格互补：前者恒无
                            // first_provable，后者恒有（上一臂）——两码不可混。
                            assert!(
                                entry.first_provable_at.is_none(),
                                "从未构成 ⟹ 首次可证时点恒空（否则应走 ForceOvertake）：{key:?}"
                            );
                            // 结算点 = 结构完成那一刻（advance 第 7 步同 prefix 内结算，
                            // 终态吸收禁后续改写 ⟹ 两钟恒等）。
                            assert_eq!(
                                entry.structure_end_at,
                                Some(invalidated),
                                "从未构成 ⟹ invalidated_at == structure_end_at：{key:?}"
                            );
                            assert!(
                                invalidated <= entry.last_as_of,
                                "从未构成 invalidated ≤ last_as_of：{key:?}"
                            );
                        }
                        InvalidatedReason::IdentityVanished { cause } => {
                            // 身份消失路径 invalidated_at 独立于 first_provable_at（卡 §2.3）；
                            // 失效在「本 prefix 不再产出」时结算 ⟹ invalidated ≥ last_as_of。
                            assert!(
                                entry.last_as_of <= invalidated,
                                "身份消失 invalidated ≥ last_as_of：{key:?}"
                            );
                            assert!(
                                entry.force_evidence.is_none(),
                                "IdentityVanished 恒无力度证据：{key:?}"
                            );
                            // 票 #559 新不变量：凡消失必带可审计成因，且账本字段与原因码
                            // 载荷逐位一致（单一写入点 `invalidate` 的结构性保证）。
                            assert_eq!(
                                entry.vanish_cause,
                                Some(cause),
                                "身份消失成因入账本字段且与原因码载荷一致：{key:?}"
                            );
                            if let VanishCause::ObservationSeam { successor_c_start } = cause {
                                assert_ne!(
                                    successor_c_start, key.seg_c_full.0,
                                    "接缝成因的接手身份 c 左端必异于消失身份（同左端应被桥吸收）：{key:?}"
                                );
                            }
                        }
                    }
                }
            }
            if let Some(from) = entry.superseded_from {
                assert!(from != key, "身份来源链不自环：{key:?}");
                // 两码互斥且穷尽（票 #603 档 1）：桥迁移（`Supersedes`，B 相等）或中枢升级认领
                // （`CenterUpgraded`，B 严格更晚）——`bridge_identity` 与
                // `bridge_by_center_upgrade` 在 `b_center_start` 上互斥（相等 vs 严格小于）。
                assert!(
                    bridge_identity(&from, &key) || bridge_by_center_upgrade(&from, &key),
                    "身份来源链两端满足桥身份或中枢升级认领：{key:?}"
                );
            }
        }
        for (index, signal) in self.completion_signals.iter().enumerate() {
            assert!(
                !self.completion_signals[..index].iter().any(|prior| {
                    prior.key == signal.key || bridge_identity(&prior.key, &signal.key)
                }),
                "首完成信号按桥身份唯一：{:?}",
                signal.key
            );
            if let Some(entry) = self.bridge_entry(&signal.key) {
                assert!(
                    entry.observed_at <= signal.as_of,
                    "完成信号不得早于观察钟：{:?}",
                    signal.key
                );
            }
            // 完成钟两分单调（票 #527，#559 C3 订正）：物理完成 ≤ 账本收到。二者恒等不是
            // 要求，恒序才是——违序 ⟹ provider 回填/前视，停线。
            assert!(
                signal.completed_at <= signal.as_of,
                "完成钟两分单调 completed ≤ 账本：{:?}",
                signal.key
            );
        }
    }

    /// 该身份（含桥同身份）是否已进终态——喂数出口「完成即停延展」的判据。
    ///
    /// 终态吸收（advance 第 1/3 步）使照喂与不喂的 book 逐位相同（零 revision、
    /// `last_as_of` 不动、身份消失扫描只看 Provisional）⟹ 跳过是纯成本优化，非行为改动
    /// （F5 等价性测试锚定）。
    pub(super) fn terminal_bridge_hit(&self, key: &LifecycleKey) -> bool {
        let entry = self.bridge_entry(key);
        entry.is_some_and(|entry| entry.state.is_terminal())
    }

    /// 同一完成信号是否已在此前 trigger 留档；终态本身不得冒充完成信号。
    fn completion_signal_seen(&self, key: &LifecycleKey) -> bool {
        self.completion_signals
            .iter()
            .any(|signal| signal.key == *key || bridge_identity(&signal.key, key))
    }

    /// 留档真实首完成；倒退信号不入分母，随后由 `advance` 的既有守卫显式拒绝。
    pub(super) fn register_completion_signal(
        &mut self,
        key: LifecycleKey,
        as_of: usize,
        completed_lower_id: ElementId,
        completed_at: usize,
    ) -> bool {
        if self.completion_signal_seen(&key)
            || self
                .bridge_entry(&key)
                .is_some_and(|entry| as_of < entry.last_as_of)
        {
            return false;
        }
        self.completion_signals.push(CompletionSignal {
            key,
            as_of,
            completed_lower_id,
            completed_at,
        });
        true
    }

    pub(super) fn bridge_entry(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        match self.ledger.get(key) {
            Some(entry) => Some(entry),
            None => self.bridge_match(key).and_then(|old| self.ledger.get(&old)),
        }
    }

    /// 白名单桥匹配：除 seg_c 右端外全等的既有键（同身份不同右端的键在 book 内至多一只，
    /// 迁移即替换 ⟹ 匹配唯一）。
    fn bridge_match(&self, key: &LifecycleKey) -> Option<LifecycleKey> {
        self.ledger
            .keys()
            .find(|old| **old != *key && bridge_identity(old, key))
            .copied()
    }

    /// 身份迁移：把 `old_key` 的整条 entry 原样搬到 `new_key` 下，写当下来源指针 + 追加一条
    /// 来源修订。**全 book 仅有的两个迁移点收敛于此**（票 #619 L12），搬运本身由内核
    /// [`LedgerBook::migrate`] 独占执行（票 #573 T1：旧键退表 → 改键 → 来源链留痕 →
    /// 追加迁移修订 → 新键入表）。
    ///
    /// 「钟一个 bit 不动」由实现形态保证——entry 整体搬走，五钟不经任何赋值。本方法只负责
    /// **域侧的来源码 → 修订词汇映射**（见 [`MigrationKind`]）：`from` 恒取 `old_key`
    /// （被退表的那个键），调用方不参与构造 ⟹「写来源指针」与「写哪种来源修订」不可能不一致。
    fn migrate_entry(
        &mut self,
        old_key: LifecycleKey,
        new_key: LifecycleKey,
        migration: MigrationKind,
        as_of: usize,
    ) -> LifecycleRevision {
        let kind = match migration {
            MigrationKind::Bridge => LifecycleRevisionKind::Supersedes { from: old_key },
            MigrationKind::CenterUpgrade => LifecycleRevisionKind::CenterUpgraded { from: old_key },
        };
        self.ledger.migrate(old_key, new_key, kind, as_of)
    }

    /// 中枢升级认领匹配（票 #603 档 1）：同锚（`seg_a` + C 左端全等）且 B 严格更晚的既有键，
    /// 连同「该前身此刻可否迁移」一并给出（`Provisional` ∧ 非倒退喂入）。
    ///
    /// **只服务 `advance` 第 1 步的建仓分支**——`bridge_entry`/`terminal_bridge_hit`/
    /// `completion_signal_seen` 一律仍走 [`bridge_identity`]（严格更强），本方法不参与那三处
    /// 判定：认领是「两个身份之间的关联」，不是「它们是同一个身份」，把它塞进桥语义会让终态
    /// 前身把新身份一并吸收（禁复活的适用面被误扩），那正是本设计要避免的。
    ///
    /// ## 多候选语义（票 #619 H2 订正）
    ///
    /// **可匹配者可以有多只**：认领留痕形态**恰恰把终态前身留在册**（那是它的设计要点），
    /// 于是同一条链上可同时存在「已终态的 P1（B 较早）」与「留痕后新建、仍 `Provisional` 的
    /// P2（B 居中）」；第三次 B 升级到达时两只都满足 [`bridge_by_center_upgrade`]。
    /// 旧口径「同链至多一只**存活**」把「存活」当成了「匹配」，论证不成立——已订正。
    ///
    /// **优选规则**（确定性，无平局歧义）：
    /// 1. 首选 `BTreeMap` 序首个**可迁移**者（`Provisional` ∧ `as_of ≥ last_as_of`）；
    /// 2. 无可迁移者时退回 `BTreeMap` 序首个匹配者，走认领留痕。
    ///
    /// 为什么不能按 [`LifecycleKey`] 序盲取：`sort_tuple`（见 key.rs:34）在 `b_center_start` **之前**
    /// 先比 `seg_c_full`，而 C 右端随 `as_of` 漂移、**与前身死活完全无关** ⟹ 盲取会以「谁的 C
    /// 右端更小」决定要不要迁移。若盲选到终态前身，本该被迁移的那只**存活**前身不被迁移 ⟹
    /// 五钟不继承、该 entry 沦为孤儿 ⟹ 后续走 `IdentityVanished`（凭空多一条身份消失）。
    fn center_upgrade_match(
        &self,
        key: &LifecycleKey,
        as_of: usize,
    ) -> (Option<LifecycleKey>, bool) {
        let mut fallback = None;
        for (old_key, old) in self.ledger.entries() {
            if !bridge_by_center_upgrade(old_key, key) {
                continue;
            }
            if old.state == NestEventState::Provisional && as_of >= old.last_as_of {
                return (Some(*old_key), true);
            }
            fallback.get_or_insert(*old_key);
        }
        (fallback, false)
    }
}
