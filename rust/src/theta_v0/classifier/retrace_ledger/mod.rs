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
//! | 四 | Success 后同中枢永禁新轮（死人挂号拒收） | 票 #622 落地：[`book::RetraceLedger::observe`] 前置查 [`book::RetraceLedger::death_certificate`]，命中即 [`RetraceRejection::DeadCenterReentry`] |
//! | 五 | 三钟（出生/落锤/门卫）+ 每条修订带知情时 + 位置进证据载荷 | [`RetraceEntry`] 三钟字段 + [`RetraceEvidence`] |
//! | 六 | 唯一真相 = append-only 修订日志；状态 = 日志折叠；快照仅派生缓存带溯源；拒收/警报另记 audit 流 | [`log`] 模块（M5=A 裁定）+ [`audit`] 模块（票 #622：四类警报 append-only JSONL） |
//! | 七 | 缺席 ≠ 消失；永不超时处死 | 观察 `outcome = None` 即维持 `Provisional`，无任何超时路径 |
//! | 八 | 三档消费门户 | **S1 只做成立档**（[`ThirdPointPack`]）；备战/短差档归 S3 |
//!
//! # 消费方登记（票 #624 S4 ③；#575 验收「无消费方不接生产」）
//!
//! 三档门户各自的消费方、消费面、**接线状态**逐条登记如下。「未接线」= 本账已产出该消费面、
//! 但仓内**尚无**生产路径读它——这是显式登记的在案状态，不是遗漏；本账不因此自行接线
//! （接线是各消费方自己的票）。
//!
//! | # | 消费面（本账产出） | 消费方 | 接线状态 | 接线票 / 缺口 |
//! |---:|---|---|---|---|
//! | 1 | [`CenterDeathCertificate`]（[`book::RetraceLedger::death_certificate`] + [`ThirdPointPack::death_certificate`]） | 中枢生命周期账（`crate::trading::center_book::CenterBook`） | **已接线（#637）：消费入口已立** | `CenterBook::consume_death_certificate` 消费本证明、对判同锚登记 Broken（票 #637 修复轮，2026-07-29 编排者裁定 3A：票面「生产调用点 ≥1」按字面结，`CenterDeathCertificate` 出现于生产代码即达标）；驱动入口的上游生产链——`RetraceLedger` 本身接生产驱动 + 交易层消费 `ThirdPointPack`——归 #575 后续票 |
//! | 2 | [`ThirdPointPack`]（[`book::RetraceLedger::established`] / [`book::RetraceLedger::established_pack`]） | 交易层三类点成立登记账（`crate::trading::third_point_book::ThirdPointBook`——票 #638） | **已接线（#638）：消费入口已立；驱动链归 #575 后续票** | ①消费入口 = `ThirdPointBook::register`（签名 bound `S: TradableSignal + Into<ThirdPointPack>` **即禁区闸门**，见 #2' 行）+ `sync_from_ledger`（读 [`book::RetraceLedger::established`]）/ `register_from_ledger`（读 [`book::RetraceLedger::established_pack`]）；登记 = 照实全字段在册 + 幂等 + 同身份改口 fail-loud，**零投影零判据**；②迟到处置 = **照实登记、不产独立信号**（#587 字面「不追……登记观测，不作独立信号消费」，ADR-0001 补充十六 `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:273`）——交易层**不新造迟到判据**（2026-07-29 编排者裁定一），量化判据留白归 **#680**（GitHub 依赖边 `blocked_by #638`——#638 是缘起 / 登记面提供者，**#680 自身声明的前置条件是 #575 驱动票落地**（`RetraceLedger` 接生产驱动、有真实 pack 数据流），不是 #638；修复轮订正：勿把 `blocked_by` 读成「#638 是 #680 的前置条件」；理由：交易层 bar 坐标与本账知情时是两套引擎坐标，仓内无对齐依据，同 #637 裁定 2A 跨量纲禁比）；**本账侧零参与**（不进口外部状态，裁定八总禁区）；③驱动链——`RetraceLedger` 接生产驱动 + trading ladder ↔ 本账 `RetraceProvenance.level` 坐标对齐——归 #575 后续票（2026-07-29 编排者裁定二，同 #637 裁定 3A / #639 裁定 B 口径）；票面「生产调用点 ≥1（grep 可证）」达标口径 = 类型 / `established_pack` 出现于生产代码即达标，非「已被生产调用方驱动」；④[`ThirdPointPack`] 自带 `side`，`CenterDeathCertificate` 现（票 #664）亦带同源 `side`——两者不再有方向落差；若后续票把 `pack.death_certificate` 转投 #1 行的 `CenterBook::consume_death_certificate`，该证明自带的 `side` 在**先杀（首次登记 Broken）**时足够驱动 `dead_down`/`frozen`/`CenterEvent::Terminated`（票 #664 闭合；#664 修复轮改口——先杀落位、后不覆写，锚已被另一通道先杀时本证明的方向不覆写既有登记；影子评审 MEDIUM-1 如实登记：此"不覆写"是 cert 侧对自身的纪律，`ingest` 侧的 `frozen` 置位受 Python parity 约束在死亡守卫外，跨通道时仍可能覆写 cert 已落位的 `frozen`，细节见 `CenterBook::consume_death_certificate` doc），本消费面**不需要**另行搬运方向；⑤本登记账**不发**中枢生死语义——它是同一教义事件（18 课定理三）的**第三条观测通道**（前两条 = `CenterBook::ingest` 的 confirmed Type3 kill 分支与 #1 行的证明消费面），三通道互不代劳 |
//! | 2' | [`StandbyWatch`]（[`book::RetraceLedger::standby`]）——同一消费方的**备战**面 | 交易层（盯次级别回切入点，024:36） | **未接线** | 禁区：不许被消费成买入信号，类型面隔离已由 [`portal::TradableSignal`] 编译期把关 |
//! | 3 | [`ShortRetraceRecord`] / [`ShortRetracePortal`] / [`FailureDisposalNotice`]（亚型签 [`PanDivSubtype`]） | 盘背短差通道（`signal::drain_pan_div_short_retrace_observations`，pan_div_diag/signal 一线——票 #639） | **已接线（#639）：消费入口已立，管线未起** | ①消费入口 = 本票新立 `signal::drain_pan_div_short_retrace_observations`，物理位置与 #606 S1 一类点分级 sidecar 同构，但**非管线同构**——本票未起 collector/summary/runner 骨架（理由：`RetraceLedger` 尚无驱动源，无驱动源支撑的骨架即死代码；collector 形态归 #575 驱动票按当时真实驱动需要决定，不由本票预先猜形）；②与仓内既有 pan_div 生产主链**零连接**——该主链——`signal::locate_pan_div_structure` → `PanDivCert` → `strategy::oscillation::PanDivTrigger`（#292 后已降格为可选辅助，`backtest::fill::step_center_oscillation` 不再消费它，见 `fill.rs:761`；中枢震荡交易语义改由次级别买卖点驱动，产出 `strategy::center_oscillation_trade::CenterOscillationTrigger`，024:36/46 上沿减/下沿补语义落在该链末端 `short_diff_bucket`/`oscillation_campaign`）——自身盘背产出，本票交付物一行未动，两条产线并行、互不知情；③驱动链——`RetraceLedger` 本身接生产驱动（喂真实 replay 判败事件）——归 #575 后续票（2026-07-29 编排者裁定 B，同 #637 裁定 3A 口径）；票面「生产调用点 ≥1」的达标口径 = 类型 `ShortRetraceRecord`/`ShortRetracePortal` 出现于生产代码即达标（裁定 B / #637 裁定 3A 同口径），非「本函数已被生产调用方驱动」；`drain` 的 `consume` 与 [`portal::ShortRetracePortal::disposal_notices`] 共享同一 `consumed` 判重集——通道侧目前无「已入场者通知」消费方（grep 实证，当前无实害），但接线后若通知面与本通道挂在**同一个** [`portal::ShortRetracePortal`] 实例上，通道每 drain 一条、通知面就少一条（通道先跑则通知面恒空）；[`FailureDisposalNotice`]/`disposal_notices` **不是**「已备好待接」（原措辞已失准），#575 后续票接通知面时须给两面各自独立门户实例、或给 `disposal_notices` 另设判重 |
//!
//! 生产接线点计数（登记时点 2026-07-29，#638 接线轮更新）：**3/3（入口计数口径）**。本模块在
//! `lib` 内被 [`super`](super) 注册；#1 消费面经 `CenterBook::consume_death_certificate` 接线、
//! #2 消费面经 `trading::third_point_book::ThirdPointBook`（`register` / `sync_from_ledger` /
//! `register_from_ledger`）接线、#3 消费面经 `signal::drain_pan_div_short_retrace_observations`
//! 接线（三面**均为消费入口已立、非驱动链全通**——`RetraceLedger` 自身仍无生产实例，见 #575
//! 后续票）；**#2'（[`StandbyWatch`] 备战档）仍是「未接线」且按裁定八禁区不进本轮计数**——它
//! 只有测试群消费，除此之外零生产调用点，与登记表一致。
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
//! - **S2**（#622，已落地）：引擎改口处死（[`NotConstitutedReason::CenterRebased`]）+ 死人挂号
//!   拒收（[`RetraceRejection::DeadCenterReentry`]）+ 两拍证据一致性守卫
//!   （[`RetraceRejection::TerminalEvidenceContradictsRegistration`]）+ 四类警报 audit 流
//!   （[`audit`] 模块）；
//! - **S3**（#623）：备战档 / 短差档门户；
//! - **S4**（#624，已落地）：旧模块 `first_retrace_replay` 的 fixtures 行为等价面对拍
//!   （[`tests::replay_parity`]）+ 事件日志 golden 锚（[`tests::golden_log`]）+ 消费方登记
//!   （见下 §消费方登记）+ 旧模块删除（编排者 2026-07-29 裁定 A「5 删」）——其
//!   [`StrictCompletedPair`] / [`RetraceOutcome`] 两枚域词汇迁入本模块，一个 bit 不改。

use serde::{Deserialize, Serialize};

use super::super::types::{Direction, Tick};
use super::ledger_kernel::{
    first_write_clock, LedgerEntryCore, LedgerPolicy, LedgerRevision, LedgerState,
};

pub mod adapter;
pub mod audit;
pub mod book;
pub mod log;
pub mod portal;

#[cfg(test)]
mod tests;

pub use adapter::{admit_input, AdmittedObservation, RetraceInput, RetraceRejection};
pub use audit::{
    JsonlRetraceAuditStore, RetraceAuditError, RetraceAuditEvent, RetraceAuditRecord,
    RetraceRejectionCode, AUDIT_SCHEMA_VERSION,
};
pub use book::{CenterDeathCertificate, RetraceAlarms, RetraceLedger, RetraceStep, ThirdPointPack};
pub use log::{
    JsonlRetraceLogStore, RestoreRoute, RetraceLogError, RetraceProvenance, RetraceRecord,
    RetraceSnapshot, SnapshotRejection, LOG_SCHEMA_VERSION,
};
pub use portal::{
    FailureDisposalNotice, PanDivSubtype, ShortRetracePortal, ShortRetraceRecord,
    ShortRetraceRejection, StandbyWatch, TradableSignal,
};

// ═══════════════════════════════════════════════════════════════════════════
// 域词汇（票 #624：自旧模块 `first_retrace_replay` 迁入）
// ═══════════════════════════════════════════════════════════════════════════

/// 严格相邻的已完成 Move pair：`leave` = 离开中枢那根，`retest` = 回抽那根。
///
/// **出处与迁入理由**：原 `classifier::first_retrace_replay`（D7 只读复核原语，`e942e1d3c1` 入库）。
/// 编排者 2026-07-29 裁定 A（5 删）删除该模块时，本类型与 [`RetraceOutcome`] 是账本**仍在用**的
/// 两枚域词汇，故迁入本模块；其余七项（D1 seed → 唯一 CompletedMove 映射、四类 fail-closed 错误码、
/// 瞬态 replay 自动机及其两个错误码）随模块一并删除。逐条去向见
/// `chanlun/review-results/issue624-fixture-replay-diff-20260729.md` §四。
///
/// **判据分工**：「严格相邻」（`retest == leave + 1`，补充十五紧邻语义）由
/// [`adapter::admit_input`] 在注册期强制；`leave` / `retest` 各自是否唯一属于一根 Completed Move
/// 是**上游 provider 的职责**——本账入口只接已配对 pair，不重做投影层的映射证明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictCompletedPair {
    /// 离开中枢那根 CompletedMove 的索引。
    pub leave_move_index: usize,
    /// 回抽那根 CompletedMove 的索引（严格 = `leave_move_index + 1`）。
    pub retest_move_index: usize,
}

/// 回抽结局（同上，自旧模块迁入；一个 bit 不改）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetraceOutcome {
    /// 回抽重回中枢框内 ⟹ 判败（[`NotConstitutedReason::RetestReentered`]）。
    RetestReenters,
    /// 回抽不回 ⟹ 判胜（三类点成立，快照转正 + 中枢死亡证明）。
    Success,
}

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
/// **边界条件（本锚失效的唯一情形）**：上游引擎重基/重切致中枢起点被改写——旧锚下的候选从
/// 观察者视角**等价于中枢消失**（新起点下是全新的锚，旧锚再也收不到任何观察）。那正是裁定一
/// 「引擎改口 → 处死记档进警报桶」要处理的情形（票 #622 落地：`observed_window = None` 经
/// [`book::RetraceLedger::reconcile_window`] 显式核对通道处死；同锚内仅右边变的常见改口则由
/// [`book::RetraceLedger::observe`] 在注册碰撞时直接感知，无需外部显式核对）。
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
    /// 引擎改口致身份灭失（重基/重切致窗口变或中枢消失，对标 `RebaseVanished` 哲学；票 #622 落地）。
    ///
    /// 身份灭失、非破坏（裁定一）；不与 [`Self::RetestReentered`] 混行——不是竞选失败，是判案锚
    /// 本身没了。证据载荷带**新旧窗口对照**：
    CenterRebased {
        /// 注册拍钉下的旧窗口（该身份的 `key.frame`）。
        registered_window: CenterFrame,
        /// 引擎当拍报的新窗口；`None` = 中枢已从 provider 消失（两种偏离同一处死路径）。
        observed_window: Option<CenterFrame>,
    },
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
        self.revisions
            .iter()
            .find_map(|revision| match revision.kind {
                RetraceRevisionKind::Restarted { previous } => Some(previous),
                _ => None,
            })
    }

    /// 判败原因码（现算：`NotConstituted` 修订载荷）。
    pub fn not_constituted_reason(&self) -> Option<NotConstitutedReason> {
        self.revisions
            .iter()
            .find_map(|revision| match revision.kind {
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
