//! 三态/原因码/力度三值化/修订/entry（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 三态 / 原因码 / 力度三值化（卡 §2.3/§4 + #78 修复 1）
// ═══════════════════════════════════════════════════════════════════════════

/// 三态（E2E-D5 最小子集；`Unresolved` 出切片——模块头 090 登记 2）。
///
/// **本类型即内核三态**（票 #573 T1 重基）：[`LedgerState`] 承载「未决 / 成立 / 失效」这条
/// **账本纪律**——内核承载终态吸收（`admit`）与禁复活（`settle` 守卫，票 #573 T1 修复轮新增）；
/// 终态钟只写一次由「`settle` 守卫（禁二次落账）+ 域侧 `write_settlement` 写入点」联合保证。
/// 三个变体逐位同名同序。域语义
/// ——何时进入哪一态、该态在缠论下的名分——仍在本模块，逐条登记如下：
///
/// - `Provisional` = 活假设（061:26「都可以先假设是进入背驰段」——默认态，非例外态）;
/// - `Confirmed` = first_provable 已写 ∧ c 结构完成 ∧ 完成时复核仍弱（024:24）;
/// - `Invalidated` = 力度反超（061:26，可由 Provisional 直接到达，无需结构先完成）、
///   从未构成（061:28）或身份消失（E2E §1:81）；终态留档，禁删除模拟失效。
///
/// `is_terminal` 由内核提供（同一条纪律，无域内容），语义一个 bit 不变。
pub type NestEventState = LedgerState;

/// 活假设账本的泛化面实例（票 #573 T1）：把 11 个对象无关责任点接到内核
/// [`LedgerBook`]，域侧只留自己的判据（桥/同锚/中枢升级、三态业务条件、原因码含义、
/// 域钟语义、provider/feed、力度判定、完成信号与消失扫描）。
///
/// 四组类型参数的域取值：`Key` = [`LifecycleKey`]（六元组）、`Observation` =
/// [`LifecycleObservation`]（事件 / pan 活窗双通道）、`RevisionKind` =
/// [`LifecycleRevisionKind`]、ReasonPayload = [`InvalidatedReason`] + [`ForceEvidence`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestPolicy;

impl LedgerPolicy for NestPolicy {
    type Key = LifecycleKey;
    type Observation = LifecycleObservation;
    type RevisionKind = LifecycleRevisionKind;
    type Reason = InvalidatedReason;
    type Evidence = ForceEvidence;
    type Entry = NestLifecycleEntry;

    fn observation_key(observation: &Self::Observation) -> Self::Key {
        observation.key()
    }

    fn opened_revision_kind() -> Self::RevisionKind {
        LifecycleRevisionKind::Observed
    }
}

/// 身份消失的两类成因（票 #559 编排者裁定 2026-07-28；挂 #523 遗留问题 2 名下）。
///
/// 裁定原文：「凡消失的身份必须带可审计原因码，且原因码拆两类落**账本字段**——
/// (a) 假设被推翻（真终局）；(b) 观测接缝伪影（provider 换轨丢下）。混记会污染
/// 寿命/反超率统计，禁。」
///
/// **判据（`advance` 第 8 步现算，唯一分类点）**：消失身份的**锚**
/// = `(level, side, kind, seg_a, b_center_start)`（身份键去掉 `seg_c_full`）。
/// 本 prefix 的观察集合里若仍有同锚身份（必然是不同 `seg_c_full.0`——同左端会被
/// [`bridge_identity`] 吸收成 `Supersedes` 而不进消失扫描），则该锚的假设并未死，
/// 只是 provider 把 C 换到了下一段 ⟹ (b)；否则该锚本 prefix 完全不再产窗 ⟹ (a)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanishCause {
    /// (a) 假设被推翻（真终局）：本 prefix 该锚不再产出任何窗，结构前提被后续演化否证。
    /// 教义依据：`parser/tail.rs`「未完成走势只能唯一分类为当下状态，不能强行分类为
    /// 最终结果」⟹ 基于当下状态建立的活假设必然可被后续结构演化取消。
    HypothesisRefuted,
    /// (b) 观测接缝伪影：同 prefix 同锚仍有窗/事件产出，只是 C 段左端换了
    /// （`successor_c_start`）。假设本身没死，是 provider 换轨把旧 key 丢下——与 #523
    /// 钉的根因同类（从「首见即完成」变成「窗口错配」）。
    ///
    /// `successor_c_start` = 本 prefix 同锚新身份的 `seg_c_full.0`；锚其余五元与消失身份
    /// 逐位相同，故该值唯一定位接手身份（右端随 bar 延展，不进身份）。同 prefix 存在多个
    /// 同锚新身份时取 `BTreeSet` 序首个（确定性，无平局歧义）。
    ObservationSeam { successor_c_start: usize },
}

/// 失效原因码（入 revision 载荷，诊断可查账；裁定 #64 §2(b)）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidatedReason {
    /// 被反超：**曾构成、后被否证**（061:26「一旦力度大于前者，那么就可以断定背驰段不
    /// 成立」）——要求曾写入 first_provable，是「先成立、再被推翻」。
    ForceOvertake,
    /// 身份消失（上一 prefix 有、本 prefix 不再产出该 key；E2E §1:81）。成因两分见
    /// [`VanishCause`]（票 #559 裁定：混记禁）。
    IdentityVanished { cause: VanishCause },
    /// 从未构成：**根本未构成**（061:28「因为背驰如果没有创新高，是不存在的」）——结构
    /// 完成时该活假设从未写入 first_provable，与 ForceOvertake 的「曾构成」严格互补
    /// （两码的 first_provable_at 一有一无，assert_invariants 逐条钉死）。
    NeverConstituted,
}

/// 力度不可验原因码（#78 修复 1：「不可验」≠「不再弱」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailReason {
    /// 缺 hist/dif 力度序列。
    MissingForceSeries,
    /// `map_src_to_close_idx` 源坐标 → close 下标映射失败。
    CoordinateMapFailed,
}

/// 力度核查三值化（#78 修复 1 全量）：**仅 `Verified(false)` 判 Invalidated**；
/// `Unavailable` 只记审计注记（不判 Invalidated、不写 first_provable）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceCheck {
    /// 力度谓词在当 prefix 窗口上现算结果（true = 仍弱 / false = 不再弱）。
    Verified(bool),
    /// 力度不可验（原因码入注记）。
    Unavailable(UnavailReason),
}

/// 力度证据载荷（裁定 #64 §4「反超证据（力度对比量）入 revision 载荷」；票 #425 起
/// `NeverConstituted` 同样现算入载荷——两条力度类否证均可查账）。
///
/// 逐通道现算复用 `same_color_area`/`segment_dif_peak`/`same_dir_hist_peak`
/// （与 `segments_diverge_or` 同一组原语，禁第二查法）；仅审计载荷，不进真值路径。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceEvidence {
    pub area_a: f64,
    pub area_c: f64,
    pub dif_peak_a: f64,
    pub dif_peak_c: f64,
    pub hist_peak_a: f64,
    pub hist_peak_c: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// 修订 / entry（E2E §1:83 修订协议最小形态）
// ═══════════════════════════════════════════════════════════════════════════

/// 修订种类（业务载荷投影变化才产一条，E2E §1:83）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifecycleRevisionKind {
    /// ∅ → Provisional：首次观察到比较对。
    Observed,
    /// 白名单工程桥身份迁移（收束/回扩/活窗延展）：钟不动、链留痕、无 Invalidated。
    Supersedes { from: LifecycleKey },
    /// 中枢升级认领（票 #603 档 1）：同一候选（同 `seg_a` 同 C 左端）的 B 参照被一个**更晚
    /// 确认、因而更近**的中枢替换（判据 [`bridge_by_center_upgrade`]）。两种形态，由前身
    /// 死活决定，**都不改前身条目的任何 bit**：
    ///
    /// - 前身仍 `Provisional` ⟹ **迁移**：与 `Supersedes` 同款（remove + 继承五钟 + 链留痕），
    ///   前身条目被迁移走，无 `Invalidated`、无 `IdentityVanished`；
    /// - 前身已终态 ⟹ **认领留痕**：新身份独立建仓（`Observed` 后紧跟本修订），前身条目
    ///   原样留档（终态吸收/禁复活/终态钟只写一次三条不变量一个 bit 不动，E2E §1:83）。
    ///   本修订是两条历史之间**唯一**的可审计关联——统计口径按**本修订自带的 `from`** 反查
    ///   即可把「被认领的前身终局」从反超/寿命分母中剔除（#599 §5-1「口径正确性修复」）。
    ///   **禁经 `superseded_from` 中转**（票 #619 H1）：那是「当下来源指针」，每次桥迁移
    ///   整体覆盖；修订链 append-only、永不覆盖，才是认领事件的权威载体。
    CenterUpgraded { from: LifecycleKey },
    /// D1–D4 首次同真（仅 `Verified(true)` 分支写入——#78 核验 T14 锚定）。
    FirstProvable,
    /// c 结构完成信号首次到达。仅力度可验且此前未由第二终局路径吸收时产生；同一 prefix
    /// 随后结算为 Confirmed 或 Invalidated(NeverConstituted)。曾可证身份若先在活窗观察
    /// 被力度反超，合法链为 Provisional→Invalidated(ForceOvertake)，不伪造本修订。
    /// Unavailable prefix 只写 ForceUnavailable + `CompletionForceUnavailableAudit`，
    /// 数据补齐并重发信号后再留痕。
    StructureCompleted,
    /// 力度不可验审计注记（不判 Invalidated、不写 first_provable）。
    ForceUnavailable { reason: UnavailReason },
    /// Provisional → Confirmed（完成时复核仍弱，024:24）。
    Confirmed,
    /// Provisional → Invalidated；原因逐一为 ForceOvertake、NeverConstituted 或
    /// IdentityVanished（逐码入载荷，可区分——T6 / T17）。
    Invalidated { reason: InvalidatedReason },
}

/// 迁移来源码（[`NestLifecycleBook::migrate_entry`] 的唯一入参形态，票 #619 L12）。
///
/// 只说「哪一种迁移」，**不带 `from`**——`from` 由迁移点自己填（它就是被 `remove` 的那个键），
/// 调用方无从填错。两处逐句同构的迁移块收敛到单一写入点后，「写 `superseded_from`」与「写哪
/// 种来源修订」的一致性由类型保证，而非靠两处代码各自遵守纪律（Fowler: Duplicated Code）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MigrationKind {
    /// 白名单工程桥（除 C 右端外全等，判据 [`bridge_identity`]）。
    Bridge,
    /// 中枢升级认领（严格同锚 + B 单调前进，判据 [`bridge_by_center_upgrade`]，票 #603 档 1）。
    CenterUpgrade,
}

/// 一条修订（entry 留档 + advance 增量返回双通道；禁删除模拟失效，E2E §1:81）。
///
/// **本类型即内核修订**（票 #573 T1 重基）：[`LedgerRevision`] 四字段与旧域结构逐位同构
/// ——`key`（修订发生时 entry 的身份键，Supersedes 后为新键）、`kind`、`as_of`（产生该修订
/// 的 prefix，一切钟 ≤ 此 as_of，E2E-S5 判据）、`evidence`（力度证据：ForceOvertake /
/// NeverConstituted 且力度序列齐备时现算，IdentityVanished 恒 None）。append-only 纪律与
/// 「计数 == 留档长度」由内核唯一追加点 [`LedgerEntryCore::push_revision`] 结构性保证。
pub type LifecycleRevision = LedgerRevision<NestPolicy>;

/// 生命周期 entry：身份 + 三态 + 五钟 + 留档（卡 §2.3 + #78 修复 1/2 字段）。
///
/// 五钟 `observed_at / first_provable_at / structure_end_at / confirmed_at /
/// invalidated_at`（E2E §4.1:144）：首次写入不后移、终态钟只写一次、无删除 API。
#[derive(Debug, Clone, PartialEq)]
pub struct NestLifecycleEntry {
    pub key: LifecycleKey,
    pub state: NestEventState,
    /// 修订计数（== `revisions.len()`；业务载荷投影变化才 +1）。
    pub revision: u32,
    /// 首见 prefix；建仓写一次、无任何改写点。
    pub observed_at: usize,
    /// D1–D4 首次同真 prefix；仅 `Verified(true)` 写入、只写一次。
    pub first_provable_at: Option<usize>,
    /// c 结构完成 prefix（首次到达写一次）。
    pub structure_end_at: Option<usize>,
    /// 首次进 Confirmed。
    pub confirmed_at: Option<usize>,
    /// 首次进 Invalidated。
    pub invalidated_at: Option<usize>,
    /// 失效原因码（ForceOvertake / NeverConstituted / IdentityVanished 三者可区分）。
    pub invalidated_reason: Option<InvalidatedReason>,
    /// 身份消失成因（票 #559 裁定：两类落账本字段）。**恒与 `invalidated_reason` 同真**
    /// ——`IdentityVanished{cause}` ⟺ `Some(cause)`，其余原因码恒 `None`
    /// （`assert_invariants` 逐条钉死；唯一写入点 `invalidate`）。
    pub vanish_cause: Option<VanishCause>,
    /// 力度证据终态留档（ForceOvertake 与 NeverConstituted 现算，材料缺则诚实 None；
    /// IdentityVanished 恒 None——无力度语义）。
    pub force_evidence: Option<ForceEvidence>,
    /// 最近一次力度不可验注记 prefix（同 as_of 幂等去重基准；只作去重，不阻塞恢复推进）。
    pub force_unavailable_at: Option<usize>,
    /// 身份来源链的**当下来源指针**（两码：白名单桥迁移 `Supersedes` / 中枢升级认领
    /// `CenterUpgraded`，两种迁移都不改写任何钟）。
    ///
    /// **有效域（票 #619 H1 订正）**：本字段是单个 `Option`、**每次迁移整体覆盖**——它只
    /// 回答「这只 entry 的当下身份是从哪个键搬过来/认领来的」，不承载历史。留痕认领产出的
    /// 新身份若在后续 bar 被桥迁移，本字段即被改写为「上一 bar 的自己」，与终态前身的关联
    /// 从本字段消失。故**认领关联的权威来源是 append-only 的 `CenterUpgraded` 修订**
    /// （`revisions` 内自带 `from`，见 [`NestLifecycleBook::settlement_stats`] 的 claimed
    /// 集合），本字段不得用于任何跨 bar 的口径统计。
    pub superseded_from: Option<LifecycleKey>,
    /// 该身份已见最大 as_of（#78 修复 2 倒退守卫基准）。
    pub last_as_of: usize,
    /// 修订留档（append-only）。
    pub revisions: Vec<LifecycleRevision>,
}

/// 条目通用面（票 #573 T1）：域字段布局一个 bit 不动，只经访问器把内核所需的读写口
/// 暴露出去。追加（`push_revision`）与终态落账（`settle`）两条写入路径由内核默认实现
/// 独占 ⟹ 凡经这两条路径写入的修订与计数不会不一致；不变量断言在 `assert_invariants`
/// 调用点逮住绕过路径造成的不一致。
impl LedgerEntryCore<NestPolicy> for NestLifecycleEntry {
    /// 建空白条目：三态默认 Provisional、观察钟与门卫钟同取 `as_of`、五钟其余为空、
    /// 计数 0、留档空、无来源链。建项修订由 [`LedgerBook::open_on_observation`] 追加。
    fn open(key: LifecycleKey, as_of: usize) -> Self {
        Self {
            key,
            state: NestEventState::Provisional,
            revision: 0,
            observed_at: as_of,
            first_provable_at: None,
            structure_end_at: None,
            confirmed_at: None,
            invalidated_at: None,
            invalidated_reason: None,
            vanish_cause: None,
            force_evidence: None,
            force_unavailable_at: None,
            superseded_from: None,
            last_as_of: as_of,
            revisions: Vec::new(),
        }
    }

    fn key(&self) -> LifecycleKey {
        self.key
    }

    fn set_key(&mut self, key: LifecycleKey) {
        self.key = key;
    }

    fn state(&self) -> NestEventState {
        self.state
    }

    fn set_state(&mut self, state: NestEventState) {
        self.state = state;
    }

    fn revision_count(&self) -> u32 {
        self.revision
    }

    fn set_revision_count(&mut self, count: u32) {
        self.revision = count;
    }

    fn revisions(&self) -> &[LifecycleRevision] {
        &self.revisions
    }

    fn revisions_mut(&mut self) -> &mut Vec<LifecycleRevision> {
        &mut self.revisions
    }

    /// 出生钟 = 首见 prefix（建仓写一次、无任何改写点）。
    fn opened_at(&self) -> usize {
        self.observed_at
    }

    fn last_as_of(&self) -> usize {
        self.last_as_of
    }

    fn set_last_as_of(&mut self, as_of: usize) {
        self.last_as_of = as_of;
    }

    /// 落锤钟合并读数：域按终态分列两只字段（终态互斥 ⟹ 至多一只有值）。
    fn settled_at(&self) -> Option<usize> {
        self.confirmed_at.or(self.invalidated_at)
    }

    fn migrated_from(&self) -> Option<LifecycleKey> {
        self.superseded_from
    }

    fn set_migrated_from(&mut self, from: LifecycleKey) {
        self.superseded_from = Some(from);
    }

    /// 终态载荷落账（内核 [`LedgerEntryCore::settle`] 的域侧写入部）：按终态写对应的落锤钟
    /// 与载荷。`vanish_cause` 由 `reason` 现场投影 ⟹ 两个字段结构上不可能不一致；
    /// Confirmed 臂不碰任何 Invalidated 侧字段（正向终态无原因码、无力度证据）。
    fn write_settlement(
        &mut self,
        state: NestEventState,
        reason: Option<InvalidatedReason>,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) {
        match state {
            NestEventState::Confirmed => self.confirmed_at = Some(as_of),
            NestEventState::Invalidated => {
                self.invalidated_at = Some(as_of);
                self.invalidated_reason = reason;
                self.vanish_cause = match reason.expect("Invalidated 必有原因码") {
                    InvalidatedReason::IdentityVanished { cause } => Some(cause),
                    InvalidatedReason::ForceOvertake | InvalidatedReason::NeverConstituted => None,
                };
                self.force_evidence = evidence;
            }
            NestEventState::Provisional => unreachable!("非终态不入终态落账"),
        }
    }
}

impl NestLifecycleEntry {
    /// 转入终态 `Invalidated` 并留档（三个原因码的**唯一**写入点——原因码与力度证据同写，
    /// 结构上兑现模块头 090 登记 4「原因码与力度证据入载荷」；无删除路径，禁删除模拟失效）。
    ///
    /// `evidence`：ForceOvertake / NeverConstituted 传本 prefix 现算证据（材料缺则诚实
    /// None）；IdentityVanished 传 None（无力度语义）。转终态与追加修订的次序纪律由内核
    /// [`LedgerEntryCore::settle`] 承担（全内核唯一转终态点），本方法只装域载荷。
    pub(super) fn invalidate(
        &mut self,
        reason: InvalidatedReason,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) -> LifecycleRevision {
        self.settle(
            LedgerSettlement {
                state: NestEventState::Invalidated,
                kind: LifecycleRevisionKind::Invalidated { reason },
                reason: Some(reason),
                evidence,
            },
            as_of,
        )
    }

    /// 转入终态 `Confirmed` 并留档（完成时复核仍弱，024:24；正向终态无原因码、无力度证据）。
    pub(super) fn confirm(&mut self, as_of: usize) -> LifecycleRevision {
        self.settle(
            LedgerSettlement {
                state: NestEventState::Confirmed,
                kind: LifecycleRevisionKind::Confirmed,
                reason: None,
                evidence: None,
            },
            as_of,
        )
    }
}
