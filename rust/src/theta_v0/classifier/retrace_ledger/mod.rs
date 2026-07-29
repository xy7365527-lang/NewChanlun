//! 买卖点身份账本 S1：核心生命周期最小环（票 #621；#465 裁定 A 之 T3 首环）。
//!
//! # 一句话
//!
//! 观察进（适配器残废四桶注册期拒收）→ 账本活（三态 + 三钟，全部经 T1 内核
//! [`super::ledger_kernel`] 表达）→ 日志真（append-only 修订留档 = 唯一真相，外化 JSONL）→
//! 成功可消费（成立档证据包 + 中枢死亡证明）。
//!
//! # 语义规约
//!
//! `chanlun/review-results/issue574-semantic-contract-20260728.md`（#574 契约）八条裁定的落位：
//!
//! | 裁定 | 内容 | 落位 |
//! |---:|---|---|
//! | 一 | 身份 =（中枢四条边快照, departure）；活着期间窗口不动；Success 快照转正 | [`RetraceKey`] / [`CenterFrame`] / [`CenterDeathCertificate`] |
//! | 二 | 同一中枢同时刻至多一个活跃候选；未判完新 departure 报错拒收 | [`book::RetraceLedger::observe`] + [`RetraceRejection::ActiveCandidateNotSettled`] |
//! | 三 | 三态 + 判败四行语义 + 终态吸收 + 迟到静默吸收 + 警报 + 残废注册期拒收 + Restart 新档记前任 | [`RetraceState`] / [`NotConstitutedReason`] / [`RetraceAlarms`] / [`RetraceRevisionKind::Restarted`] |
//! | 四 | Success 后同中枢永禁新轮（死人挂号拒收） | **S1 只计数不拒收**，见 [`RetraceAlarms::dead_center_registrations`] 与 §「留给 S2 的钩子」 |
//! | 五 | 三钟（出生/落锤/门卫）+ 每条修订带知情时 + 位置进证据载荷 | [`RetraceEntry`] 三钟字段 + [`RetraceEvidence`] |
//! | 六 | 唯一真相 = append-only 修订日志；状态 = 日志折叠；快照仅派生缓存带溯源 | [`log`] 模块（M5=A 裁定：留档外化 JSONL + 重放折叠恢复） |
//! | 七 | 缺席 ≠ 消失；永不超时处死 | 观察 `outcome = None` 即维持 `Provisional`，无任何超时路径 |
//! | 八 | 三档消费门户 | **S1 只做成立档**（[`ThirdPointPack`]）；备战/短差档归 S3 |
//!
//! # 「状态 = 日志折叠」的结构性兑现
//!
//! [`RetraceEntry`] **只保留三钟 + 三态 + 计数 + 留档**六项内核必需字段；注册快照、侧、位置、
//! 判败原因码、终态证据、Restart 前任身份**一律不设字段**，全部由留档现算（`registration()` /
//! `side()` / `not_constituted_reason()` / `terminal_evidence()` / `restarted_from()`）。
//! 日志与状态因此**结构上不可能漂移**——投影没有第二份存放处（裁定六「日志+状态双真相必漂移」）。
//!
//! # 本票不做（范围外，勿在此模块寻找）
//!
//! - **S2**（#622）：引擎改口检测（[`NotConstitutedReason::CenterRebased`] 只留词汇位，S1 零产出路径）
//!   与死人挂号拒收；
//! - **S3**（#623）：备战档 / 短差档门户；
//! - **S4**：与 [`super::first_retrace_replay`] 的对拍与旧模块处置——本模块**只借用**其
//!   [`StrictCompletedPair`] / [`RetraceOutcome`] 域词汇，一个字节不改它。
//!
//! # 留给 S2 的钩子
//!
//! [`book::RetraceLedger::death_certificate`]（按中枢锚查死亡证明）+
//! [`RetraceAlarms::dead_center_registrations`]（S1 计数、S2 改判拒收）。

use serde::{Deserialize, Serialize};

use super::super::types::{Direction, Tick};
use super::first_retrace_replay::{RetraceOutcome, StrictCompletedPair};
use super::ledger_kernel::{
    first_write_clock, LedgerEntryCore, LedgerPolicy, LedgerRevision, LedgerState,
};

pub mod adapter;
pub mod book;
pub mod log;

#[cfg(test)]
mod tests;

pub use adapter::{admit_input, AdmittedObservation, RetraceInput, RetraceRejection};
pub use book::{
    CenterDeathCertificate, RetraceAlarms, RetraceLedger, RetraceStep, ThirdPointPack,
};
pub use log::{
    JsonlRetraceLogStore, RetraceLogError, RetraceProvenance, RetraceRecord, RetraceSnapshot,
    RestoreRoute, SnapshotRejection, LOG_SCHEMA_VERSION,
};

// ═══════════════════════════════════════════════════════════════════════════
// 身份（裁定一）
// ═══════════════════════════════════════════════════════════════════════════

/// 中枢四条边快照（裁定一「临时右边」）：候选判案专用的框，价格只对该框 ZG/ZD。
///
/// 四条边 = 上下两条价格沿（ZG/ZD）+ 起止两条时间边（补充十三）。快照在**注册拍**拍摄，
/// 活着期间一个 bit 不动（行情物理保证：候选活着 ⟹ 价格在中枢外 ⟹ 中枢无延伸）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CenterFrame {
    /// 核心区间下沿 ZD（闭区间）。
    pub zd: Tick,
    /// 核心区间上沿 ZG（闭区间）。
    pub zg: Tick,
    /// 起时间边（中枢诞生即定）。
    pub start_index: usize,
    /// 止时间边 = 注册拍摄的「临时右边」：判胜转正为永恒右边、重回作废。
    pub end_index: usize,
}

impl CenterFrame {
    /// per-中枢路由锚（见 [`CenterAnchor`]）。
    pub fn anchor(self) -> CenterAnchor {
        CenterAnchor(self.start_index)
    }
}

/// 中枢路由锚 = 起时间边。
///
/// 裁定二（同一中枢同时刻至多一个活跃候选）与裁定三（Restart 新档记前任）在结构上都要求一个
/// **跨代际稳定的中枢标识**：临时右边逐代变（延伸右扩），不能做锚；起时间边在中枢诞生那一刻
/// 定死，延伸只右扩、扩张只动 ZG/ZD，起点均不动，故取起时间边。
///
/// **边界条件（本锚失效的唯一情形）**：上游引擎重基/重切致中枢起点被改写。那正是裁定一
/// 「引擎改口 → 处死记档进警报桶」要处理的情形，归 S2 `CenterRebased` 路径，S1 不承诺。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CenterAnchor(pub usize);

/// 候选侧：三类买点（向上离开）/ 三类卖点（向下离开）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetraceSide {
    /// 向上离开中枢后回抽不回 ⟹ 第三类买点候选。
    Buy,
    /// 向下离开中枢后回抽不回 ⟹ 第三类卖点候选。
    Sell,
}

impl RetraceSide {
    /// 离开方向 → 候选侧（唯一映射点）。
    pub fn from_departure(direction: Direction) -> Self {
        match direction {
            Direction::Up => RetraceSide::Buy,
            Direction::Down => RetraceSide::Sell,
        }
    }
}

/// 身份 = **（中枢四条边快照, departure move 索引）**（裁定一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RetraceKey {
    /// 注册拍的四条边快照（临时右边）。
    pub frame: CenterFrame,
    /// 离开中枢的那根 CompletedMove 索引。
    pub departure_move_index: usize,
}

impl RetraceKey {
    /// per-中枢路由锚（裁定二：不同中枢各有活跃候选，天然并存）。
    pub fn anchor(self) -> CenterAnchor {
        CenterAnchor(self.frame.start_index)
    }

    /// 身份 → 严格相邻 pair：`retest = leave + 1` 由适配器强制（补充十五紧邻语义）
    /// ⟹ pair 是身份的**纯函数**，无需另存，重放亦可现算。
    pub fn pair(self) -> StrictCompletedPair {
        StrictCompletedPair {
            leave_move_index: self.departure_move_index,
            retest_move_index: self.departure_move_index + 1,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 位置与证据（裁定五：位置进证据载荷，不是钟）
// ═══════════════════════════════════════════════════════════════════════════

/// 一个笔终点（位置证据的原子）。
///
/// **知情时 ≠ 位置**（裁定五）：`index` 是行情上那根 bar 的源坐标，账本何时知道由修订自带的
/// `as_of` 说；回测不用未来信息靠后者保证，画图定位用前者。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RetracePoint {
    /// 源坐标（笔终点所在 bar 下标）。
    pub index: usize,
    /// 该终点价。
    pub price: Tick,
}

/// 修订证据载荷：判案锚的框 + 侧 + 两处位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RetraceEvidence {
    /// 判案锚的四条边快照（注册拍；判胜时即转正后的中枢永恒边界）。
    pub frame: CenterFrame,
    pub side: RetraceSide,
    /// 「离开的边」= leave 笔终点（裁定五）。
    pub leave_end: RetracePoint,
    /// 三类点位置 = 回抽笔终点；未决（回抽尚未走完）时**诚实 `None`**。
    pub retest_end: Option<RetracePoint>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 三态与词汇（裁定三）
// ═══════════════════════════════════════════════════════════════════════════

/// 三态：`Provisional`（未决）/ `Confirmed`（成立）/ `Invalidated`（从未成立族）。
///
/// 内核三态一个 bit 不变；负向终态的**名分**由 [`NotConstitutedReason`] 承担
/// ——契约措辞 `NotConstituted{reason}` 即「内核 `Invalidated` + 本原因码」。
pub type RetraceState = LedgerState;

/// 从未成立族的原因码（裁定三）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotConstitutedReason {
    /// 判败：回抽重回快照框内。四行语义（裁定三）——
    /// ① 候选**从未成立**（第一次回抽资格一次性，已消费，020:62）；
    /// ② **中枢未破坏**（018:64 定理三逆否、101:76）；
    /// ③ **不派生任何中枢生命周期事件**（补充十四：延伸无事件）；
    /// ④ 判败事件是**盘背观测源**（027:16），归观测/短差通道，非终结非信号。
    RetestReentered,
    /// 引擎改口致身份灭失（重基/重切致窗口变或中枢消失，对标 `RebaseVanished`）。
    ///
    /// **S1 零产出路径**——本票只落词汇位，检测与处死归 S2（#622）。
    CenterRebased,
}

/// 修订词汇（append-only 留档的字母表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetraceRevisionKind {
    /// 建项（内核 [`LedgerPolicy::opened_revision_kind`]，无载荷——内核建项口固定不带证据）。
    Registered,
    /// 注册拍快照：把四条边 + 侧 + 位置钉进留档。
    ///
    /// 与 [`Self::Registered`] 是活路上的**原子对**（同一 `observe` 内先后追加）；重放时二者
    /// 各自独立生效，前缀恰好截在两者之间时该条目 `registration()` 为 `None`——截到哪是哪，
    /// 是合法的前缀态（裁定七「prefix 重放一致性白捡」）。
    SnapshotPinned,
    /// Restart 新档的谱系载荷：记前任身份（裁定三；**不走桥迁移**，本账快照不换键）。
    Restarted { previous: RetraceKey },
    /// 判胜落锤：Provisional → Confirmed。
    Confirmed,
    /// 判败落锤：Provisional → Invalidated{reason}。
    NotConstituted { reason: NotConstitutedReason },
}

/// 一条修订（内核类型实例）：`key` + `kind` + **知情时 `as_of`** + 证据载荷。
pub type RetraceRevision = LedgerRevision<RetracePolicy>;

// ═══════════════════════════════════════════════════════════════════════════
// 泛化面实例（AC「账本状态机经 T1 内核表达」的落点）
// ═══════════════════════════════════════════════════════════════════════════

/// 已验证观察（内核建项入口 = [`LedgerPolicy::Observation`] 的域取值）。
///
/// 契约「观察携带（严格相邻 pair, outcome, 当前中枢窗口, as_of）」逐项落位：pair 由
/// [`RetraceKey::pair`] 现算（严格相邻是适配器的准入条件，故 pair ≡ 身份的纯函数）；
/// 当前中枢窗口即 `key.frame`；`outcome` 与 `as_of` 各占一字段。
///
/// **证据不在此**：注册拍证据随 `SnapshotPinned` 修订进留档，不在建项口重复搬运
/// ——重放时建项记录只带身份与知情时，证据由紧随的那条记录带回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceObservation {
    /// 身份（注册拍四条边快照 + departure）。
    pub key: RetraceKey,
    /// 结局；**`None` = 未决**（回抽那笔尚未判完）——裁定七：沉默是正常 pending，
    /// 永不设超时处死。
    pub outcome: Option<RetraceOutcome>,
    /// 知情时。
    pub as_of: usize,
}

impl RetraceObservation {
    /// 严格相邻 pair（现算，见 [`RetraceKey::pair`]）。
    pub fn pair(&self) -> StrictCompletedPair {
        self.key.pair()
    }
}

/// 买卖点身份账本的泛化面实例（T1 内核六关联类型的域取值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetracePolicy;

impl LedgerPolicy for RetracePolicy {
    type Key = RetraceKey;
    type Observation = RetraceObservation;
    type RevisionKind = RetraceRevisionKind;
    type Reason = NotConstitutedReason;
    type Evidence = RetraceEvidence;
    type Entry = RetraceEntry;

    fn observation_key(observation: &Self::Observation) -> Self::Key {
        observation.key
    }

    fn opened_revision_kind() -> Self::RevisionKind {
        RetraceRevisionKind::Registered
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 条目：三钟 + 三态 + 留档，其余全部现算
// ═══════════════════════════════════════════════════════════════════════════

/// 账本条目。
///
/// **字段表恰好等于「内核必需 + 三钟」**：任何可由留档现算的投影都不设字段（模块头
/// §「状态 = 日志折叠」）。
///
/// 不派生 `Eq`：留档元素 [`RetraceRevision`] 是内核类型，内核只给 `PartialEq`。
#[derive(Debug, Clone, PartialEq)]
pub struct RetraceEntry {
    pub key: RetraceKey,
    pub state: RetraceState,
    /// 修订计数（== `revisions.len()`，内核唯一追加点结构性保证）。
    pub revision: u32,
    /// **出生钟**（裁定五 `registered_as_of`）：注册知情时，建仓写一次、无改写点。
    pub registered_as_of: usize,
    /// **落锤钟**（裁定五 `terminal_as_of`）：判决知情时，首写永不改；判胜时即中枢死亡证明
    /// 的开具时间。
    pub terminal_as_of: Option<usize>,
    /// **门卫钟**（裁定五 `last_as_of`）：随推进前移，专查时间倒退。
    pub last_as_of: usize,
    /// 修订留档（append-only）——本条目一切非钟投影的唯一来源。
    pub revisions: Vec<RetraceRevision>,
}

impl RetraceEntry {
    /// 注册拍快照（现算：`SnapshotPinned` 修订的证据）。
    ///
    /// `None` 仅出现在「前缀恰好截在建项与钉快照之间」的合法前缀态。
    pub fn registration(&self) -> Option<RetraceEvidence> {
        self.revisions
            .iter()
            .find(|revision| revision.kind == RetraceRevisionKind::SnapshotPinned)
            .and_then(|revision| revision.evidence)
    }

    /// 候选侧（现算自注册快照）。
    pub fn side(&self) -> Option<RetraceSide> {
        self.registration().map(|evidence| evidence.side)
    }

    /// Restart 前任身份（现算：`Restarted` 修订载荷）。
    pub fn restarted_from(&self) -> Option<RetraceKey> {
        self.revisions.iter().find_map(|revision| match revision.kind {
            RetraceRevisionKind::Restarted { previous } => Some(previous),
            _ => None,
        })
    }

    /// 判败原因码（现算：`NotConstituted` 修订载荷）。
    pub fn not_constituted_reason(&self) -> Option<NotConstitutedReason> {
        self.revisions.iter().find_map(|revision| match revision.kind {
            RetraceRevisionKind::NotConstituted { reason } => Some(reason),
            _ => None,
        })
    }

    /// 落锤证据（现算：终态修订的证据载荷）。
    pub fn terminal_evidence(&self) -> Option<RetraceEvidence> {
        self.revisions
            .iter()
            .find(|revision| Self::is_terminal_kind(revision.kind))
            .and_then(|revision| revision.evidence)
    }

    /// 该身份**留档所支持的门卫钟下界** = 最后一条修订的知情时。
    ///
    /// 门卫钟本体是**输入流守卫**：不产修订的未决观察也推它前移（裁定五「随推进前移」），
    /// 因此它不是身份事实、不进日志。跨进程恢复后门卫钟退回本下界——这是日志能支持的最强
    /// 断言，不是漂移。详见 [`log`] 模块 §「门卫钟的可恢复性」。
    pub fn log_supported_gate(&self) -> usize {
        self.revisions
            .last()
            .map_or(self.registered_as_of, |revision| revision.as_of)
    }

    fn is_terminal_kind(kind: RetraceRevisionKind) -> bool {
        matches!(
            kind,
            RetraceRevisionKind::Confirmed | RetraceRevisionKind::NotConstituted { .. }
        )
    }
}

impl LedgerEntryCore<RetracePolicy> for RetraceEntry {
    /// 建空白条目：`Provisional`、出生钟与门卫钟同取 `as_of`、落锤钟空、计数 0、留档空。
    fn open(key: RetraceKey, as_of: usize) -> Self {
        Self {
            key,
            state: RetraceState::Provisional,
            revision: 0,
            registered_as_of: as_of,
            terminal_as_of: None,
            last_as_of: as_of,
            revisions: Vec::new(),
        }
    }

    fn key(&self) -> RetraceKey {
        self.key
    }

    fn set_key(&mut self, _key: RetraceKey) {
        unreachable!("本账身份不换键（#574 裁定三：Restart 是新档，不走桥迁移）")
    }

    fn state(&self) -> RetraceState {
        self.state
    }

    fn set_state(&mut self, state: RetraceState) {
        self.state = state;
    }

    fn revision_count(&self) -> u32 {
        self.revision
    }

    fn set_revision_count(&mut self, count: u32) {
        self.revision = count;
    }

    fn revisions(&self) -> &[RetraceRevision] {
        &self.revisions
    }

    fn revisions_mut(&mut self) -> &mut Vec<RetraceRevision> {
        &mut self.revisions
    }

    fn opened_at(&self) -> usize {
        self.registered_as_of
    }

    fn last_as_of(&self) -> usize {
        self.last_as_of
    }

    fn set_last_as_of(&mut self, as_of: usize) {
        self.last_as_of = as_of;
    }

    fn settled_at(&self) -> Option<usize> {
        self.terminal_as_of
    }

    /// Restart 前任指针现算自留档（[`RetraceEntry::restarted_from`]）——内核在其上跑
    /// 「来源链不自环」不变量。
    fn migrated_from(&self) -> Option<RetraceKey> {
        self.restarted_from()
    }

    fn set_migrated_from(&mut self, _from: RetraceKey) {
        unreachable!("本账不用内核 migrate（#574 契约「第二消费方不使用迁移留史」）")
    }

    /// 终态载荷落账的域侧写入部：**只写落锤钟**。
    ///
    /// 原因码与证据不另设字段——它们随该次落锤修订进 append-only 留档，由
    /// [`RetraceEntry::not_constituted_reason`] / [`RetraceEntry::terminal_evidence`] 现算
    /// （裁定六：投影不单独持久化）。
    fn write_settlement(
        &mut self,
        state: RetraceState,
        _reason: Option<NotConstitutedReason>,
        as_of: usize,
        _evidence: Option<RetraceEvidence>,
    ) {
        assert!(state.is_terminal(), "非终态不入终态落账：{state:?}");
        assert!(
            first_write_clock(&mut self.terminal_as_of, as_of),
            "落锤钟首写不改（#574 裁定五）：{:?}",
            self.key
        );
    }
}
