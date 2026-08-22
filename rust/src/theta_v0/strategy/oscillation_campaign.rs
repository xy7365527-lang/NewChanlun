//! 每重 campaign（SPEC #287 T4，issue #294；★#880 改挂「重」，SPEC #847 S2，ADR 0013 裁定二）：
//! 把 #293/#354 的 `ShortDiffAccount` + TW 桥升格为**每重各一 campaign** 的完整生命周期——开仓生（`notional_in`=本重成本基）→ 短差冲减成本
//! （sizing=开局冻结快照 1/3，#348 用户裁定阶段一「固定 1/3」+ #380 项二订正基准=cost_basis
//! 快照而非当时持仓，可复检）→ 到 0 转移（`RecoverCapital`，复用
//! `closed_loop::transition::stage_progression` 单一来源，#124 裁定4）→ 全平死（`ClearCampaign`，
//! 挂起随死）。
//!
//! ## 分工边界（两台阶段机分立，SPEC #287 决议）
//!
//! - **结构阶段机**（[`center_oscillation_trade::CenterOscillationBook`]，#274/#292）：管「许不许
//!   做短差」——中枢生死驱动挂起开合，不读本模块任何状态。
//! - **经济阶段机**（本模块 [`OscillationCampaign`]）：管「本金收到哪」——消费短差盈亏桶
//!   （[`short_diff_bucket::ShortDiffAccount`]）产出的 TW 事件，推进 `TStage`。两者互不越权：
//!   本模块不读中枢生死，`CenterOscillationBook` 不读 `TwState`。短差盈亏桶是唯一桥。
//!
//! ## 粒度：每重各一（★#880 改挂，SPEC #847 S2，ADR 0013 裁定二）
//!
//! **三阶段实例单位 = 「重」**（ADR 0013 裁定二）：一重 ＝ 一份专属筹码 ＝ 一个成本 ＝
//! 一个三阶段状态，挂在**操作**级别上——**键 = (标的, 操作级别)**（[`ChongKey`]，成本状态
//! 不进键）；一重内部塔的各个结构级别都来做短差，全部汇进这一重的那个成本（级别只决定
//! 量）⟹ 同一重跨结构级别**共享同一 campaign 实例**。⚠️ 这不是回退到 per-TInstance——
//! `docs/three_phase_unified_design.md` 反对的「每个塔层一个三阶段」仍然有效；改的是挂到
//! 「重」上，重 ≠ 塔层。重内单向（ADR 0014 裁定一）：一份筹码一个持仓方向，侧（side）是
//! campaign 开局时的属性、**不进键**。
//!
//! （#880 前旧粒度：「仓」=本级别的核心持仓，按 `(结构级别, 侧)` 分键——与
//! [`center_oscillation_trade::CenterOscillationBook`] 同粒度。该口径即 SPEC #847 S2 点名的
//! 「`CampaignBook` 按 (级别, 方向) 分」。）生命周期：
//! - **开仓生**：本仓成本基快照从「空仓」（`units()==0`）变为「有持仓」（`units()>0`）——
//!   campaign 开局注资 `notional_in = holding = cost_basis()`（缠师口径「成本入账」）。
//! - **全平死**：本仓成本基快照回到「空仓」——`TwEvent::ClearCampaign`（挂起随死：此时短差
//!   盈亏桶若仍有挂起在途量，不强求先收口，按未闭合减出核销并分列呈报）。
//!
//! ## 到 0 转移（issue #294 验收③，#124 裁定4：单一来源）
//!
//! `RecoverCapital`/`EnterEarning` 的派发**不重新实现判据**——委托
//! [`closed_loop::transition::stage_progression`]（生产 I_Θ 组合层与本模块共用同一算子，不得
//! 镜像重写）。κ=0 基线（[`RiskPolicy::baseline`]，M7 冻结口径，PDF §10 canonical 最小规范）。
//!
//! ★★到 0 判据 = **挂起空 ∧ free≥本金**（2026-07-27 用户裁定，ADR 补充十）：`free≥本金` 由上述
//! 算子给（TW 层，不知桶挂起），「挂起空」由本模块在**派发前置**处加（见
//! [`OscillationCampaign::apply_action`]）——算子签名/语义不动，前置只决定「这一刻派不派」。
//!
//! ## P2-D（issue #294 范围⑤）：TW/R 双账入口对齐
//!
//! [`short_diff_bucket::ShortDiffAccount::record_and_apply_dual`]（本票新增，[`short_diff_bucket`]
//! 同文件追加方法——不改动 #293/#354 既有 `record_and_apply` 签名/行为，纯新增）把同一笔
//! `TwEvent::Realize(d)` 镜像进 `LedgerEvent::Realize(d)`，对齐
//! `closed_loop::transition::transition_adapter` 的双账本写回节奏（同一次成交的利润分量，TW 账本
//! `free` 与 R=Π-A-W 账本 `pi` 各自独立入账，两账本不同构 #90/674号，不混称）。短差场景无
//! `Allocate`（不改变持仓资本化额——本仓成本基本体不动，修7）——故只镜像 `Realize` 分量。
//!
//! ## High-2（issue #294 范围⑥）：快照/账本接口形状裁定
//!
//! 单一快照类型 [`short_diff_bucket::CoreCostBasisSnapshot`] 同时驱动两处消费：
//! [`CampaignBook::sync_position`]（开仓/全平生死判据）与
//! [`OscillationCampaign::reduce_units`]（sizing 现算派生）——不存在第二种快照形状。本裁定钉死
//! 「一个快照接口，两个消费点，零派生副本」，防止未来某一消费点悄悄另立一套字段子集导致两处
//! 读到不一致的持仓视图（同构 #354 P1-A `avg_cost` 结构性消解的精神：单一只读源，禁两份重算）。

use super::super::classifier::center_lifecycle::{CenterId, CenterLifecycleEvent};
use super::super::closed_loop::state::RiskMode;
use super::super::closed_loop::transition::{cash_sound_gate, stage_progression, TransitionError};
use super::super::types::Side;
use super::center_oscillation_trade::{
    CenterOscillationAction, SuspensionEventOutcome, SuspensionTerminationSource,
    TerminationSettlement, TriggerError,
};
use super::chong::ChongKey;
use super::ledger::{tw_step, LedgerComp, RiskPolicy, TwEvent, TwState};
use super::short_diff_bucket::{CoreCostBasisSnapshot, ShortDiffAccount, ShortDiffViolation};
use super::voice::VoiceSide;
use std::collections::BTreeMap;

// ── #1191（D01）：记账族 + 簿记迁出（纯移动零行为，消费面零改）──────────────────────
// 战役记账族（CampaignOutcome/SuspensionBatch/CoverAssignment/SuspensionAttribution/
// UnclosedReduction/ReplenishPlan/DeathWriteOff）→ `campaign_ledger`；
// CampaignBook → `campaign_book`。重导出保 `oscillation_campaign::X` 原路径逐字不变
// （fill/runner/center_oscillation_trade/short_diff_bucket 四消费方不改调用）。
// OscillationCampaign 与 CampaignWiringWitness（C3 #749 裁定面）留本文件。
pub use super::campaign_book::CampaignBook;
use super::campaign_ledger::ReplenishPlan;
pub use super::campaign_ledger::{
    CampaignOutcome, CoverAssignment, DeathWriteOff, SuspensionAttribution, SuspensionBatch,
    UnclosedReduction,
};

/// 本模块 typed 拒绝（无静默兜底，与 [`ShortDiffViolation`]/[`TransitionError`] 同规格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignViolation {
    /// 短差盈亏桶层拒绝（[`ShortDiffAccount::record_and_apply_dual`] 透传）。
    ShortDiff(ShortDiffViolation),
    /// 阶段推进事件（`RecoverCapital`/`EnterEarning`）被 closed_loop 通道拒绝——生产路径
    /// （`stage_progression` 只在 barrier 过关/free 足额时派事件）恒不触发，此分支覆盖外部
    /// 注入/边界态防御性核验。
    StageTransition(TransitionError),
    /// 本重尚无该侧开局 campaign（[`CampaignBook::sync_position`] 未见过该侧 units>0 的
    /// 快照，或存续 campaign 在另一侧）——对空仓侧重调用动作是接线错误，不静默创建幽灵
    /// campaign。★#880：键改 (标的, 操作级别) 后，「该侧无 campaign」含「重有 campaign 但
    /// 在另一侧」（重内单向，ADR 0014 裁定一）。
    NoActiveCampaign,
    /// ★#880（SPEC #847 S2，ADR 0014 裁定一「重内单向·不穿零」）：存续 campaign 的持仓侧
    /// 与 incoming 快照侧不一致且双方 units>0——穿零未经过空仓 bar。生产不可达（重内单向由
    /// 合成层 [`super::coverage::enforce_chong_unidirectional`] 保证，本模块只承载结果），
    /// 触达即 enforcement 被改坏的真警报——当场拒绝，不静默翻侧、不静默死亡重开。
    SideConflict {
        /// 存续 campaign 的持仓侧。
        campaign_side: VoiceSide,
        /// incoming 快照/动作的侧。
        incoming: VoiceSide,
    },
    /// sizing 现算派生（当时持仓 1/3，整数除法）落到 0 或以下——当场拒绝，不静默钳制为 1
    /// （持仓过小时「1/3」本就无意义，照实拒绝优于编造最小单位）。
    SizingRoundsToZero { held: i64 },
    /// ★#380 项四（ADR 补充七「挂起归属可观测」，#367 项三 A 裁）：来源中枢后续买点触发
    /// `Replenish`，但**货已满**（挂起在途量=0，无货可回补）——照实记「触发但货满」，不与
    /// [`Self::SizingRoundsToZero`]（sizing 取整到 0，真正的持仓过小场景）混计：前者是「结构
    /// 说回补、仓位本就满」的预期经济场景，后者是记账层的量纲异常。该信号对主策略的加仓语义
    /// 不受影响（本模块只观测，不干预）。
    ReplenishWhileFull,
    /// ★#383（ADR 补充八题二）：**阶段三唯一硬门**——等金额回补腿落账后桶现金为负。
    ///
    /// 「阶段一看住股，阶段三看住钱」：阶段三 sizing=`floor(桶现金÷回补价)`，取整已保证
    /// `回补金额 ≤ 可用现金` ⟹ 桶现金恒不为负；真为负即 sizing/记账被改坏，当场拒绝、整笔不落账
    /// （生产路径恒不触发，同 [`CampaignViolation::StageTransition`] 的防御性核验定性）。
    ///
    /// **有效域**（★2026-07-27 用户裁定订正，ADR 补充十——原「门按阶段分域」表述作废）：
    /// - 阶段一/二（`earning_active()==false`）：#380 项一口径不变，`free<0` 的真实亏损往返
    ///   **放行入账**（旧口径整笔回滚会让亏钱的往返在账本里根本不存在）；
    /// - 阶段三（`earning_active()==true`）：本门生效。**到 0 判据前置**（挂起空 ∧ free≥本金，
    ///   见 [`OscillationCampaign::apply_action`]）后，`EnterEarning` 只可能在桶挂起**为空**
    ///   的那一刻派出 ⟹ 进阶段三时无任何跨阶段挂起批次，本门约束的对象只剩**等金额腿**，
    ///   而等金额腿 `bought = floor(池÷价)` 的取整已保证 `bought·价 ≤ 池 ≤ 桶现金` ⟹
    ///   **结构性不可达**。ADR 补充八题二的原论证句「唯一硬门，floor 取整保证」因此成立，
    ///   #380「亏损如实入账」与本门不再有交集（前者管阶段一/二，后者管的那腿花的是池里的钱）。
    ///
    /// **唯一残余可达窗口（如实标注，非本裁定授权的代价）**：`EnterEarning` 派发的那根 bar 上
    /// 模式尚未换尺（题三「次 bar 换模式」，机制保留），该 bar 上若再落一笔 `Reduce`，其批次
    /// 按阶段一等量口径锁定，可跨到次 bar 后在阶段三收口——此腿的现金由桶总现金承担、floor
    /// 管不到，亏损收口时仍会撞本门（用例
    /// `stage_three_hard_gate_rejects_negative_bucket_cash_without_mutating_state` 即走这条）。
    /// 故非 0 读数的排查顺序：先核是不是这条事件 bar 窗口，排除后即 sizing/记账被改坏的真警报。
    EarningCashUnsound { free: i64 },
    /// ★#383：阶段三等金额回补触发，但**现金池买不起一股**（`pool < price`，取整到 0）且无
    /// 阶段一锁定批次可等量收口——预期经济场景（回补价远高于卖出价 / 零头尚未累计够），
    /// 照实分流计数，不与 [`Self::SizingRoundsToZero`]（持仓过小的量纲异常）或
    /// [`Self::ReplenishWhileFull`]（货本就满）混计。挂起继续挂着，不静默补齐。
    EarningSizingRoundsToZero { pool: i64, price: i64 },
    /// ★★#414 项三（ADR 补充十一，2026-07-27 用户裁定）：`Replenish` 触发时桶里**有**挂起在途
    /// 量，但**没有一批来自本次触发的那个来源中枢** ⟹ 拒绝，不拿这次回补去抹平别的中枢的
    /// 挂起（旧口径的「余按时间序」外溢，wf8 证据 `cover_by_side ("long","other_center"):3`）。
    ///
    /// 与 [`Self::ReplenishWhileFull`]（桶总量=0，货本就满）**分列**：那是「没货可补」，这是
    /// 「有货但不是你的货」。两者都是预期经济场景（非警报），但归因完全不同，混桶后就读不出
    /// 中枢绑定实际拦了多少。`open_units_total` = 当时桶里其他中枢的挂起总量（诊断载荷）。
    ReplenishForeignCenter { open_units_total: i64 },
}

impl From<ShortDiffViolation> for CampaignViolation {
    fn from(v: ShortDiffViolation) -> Self {
        CampaignViolation::ShortDiff(v)
    }
}

/// campaign 生命周期事件（[`CampaignBook::sync_position`] 产出，观测/witness 用）。
///
/// ★#880：`level` 字段改 `key`——三阶段实例单位 = 重，键 = (标的, 操作级别)（ADR 0013
/// 裁定二）。`ChongKey` 含 `String` ⟹ 本类型不再 `Copy`（仅 `Clone`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CampaignLifecycleEvent {
    /// 开仓生：本重成本基快照从空仓变为有持仓。`side`=本 campaign 的持仓侧（重内单向，
    /// 侧是开局属性、不进键）。
    Opened {
        key: ChongKey,
        side: VoiceSide,
        notional_in: i64,
    },
    /// 全平死：本重成本基快照回到空仓。`side` 取自将死 campaign 自身（空仓快照不携侧）。
    ///
    /// ★#441（ADR 补充十二）：`settlement`=死亡时未收口挂起的**核销呈报**
    /// （[`DeathWriteOff`]；`None`=死亡时无挂起，不编造零读数记录）。旧单一股数标量字段已退役：
    /// 它与 `settlement.units_gap` 同源同量，却没有位置呈报现金，现统一改为未闭合减出核销。
    Died {
        key: ChongKey,
        side: VoiceSide,
        settlement: Option<DeathWriteOff>,
    },
}

/// 单仓 campaign（[`TwState`] + [`LedgerComp`] 双账本并置 + [`ShortDiffAccount`] 短差桥）。
///
/// 不可变模式（coding-style）：[`apply_action`](Self::apply_action) 返回新实例，不就地修改；
/// 生死进出由 [`CampaignBook`] 持有/移除整份实例承载（campaign 边界=实例边界）。
/// ★#380：`short_diff`（含开局冻结快照）仍是 sizing 基准；新增 `current_units`（当时真实持仓，
/// 逐 bar 由 [`CampaignBook::sync_position`] 刷新，项二防线读当前的唯一来源）、`opened_bar`
/// （开局 bar，项三「开局以来 bar 数」的基准）、`suspension`（挂起归属账，项四）。
/// `suspension` 含 `Vec` ⟹ 本类型不再 `Copy`（仅 `Clone`），生死进出语义不变。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscillationCampaign {
    tw: TwState,
    ledger: LedgerComp,
    short_diff: ShortDiffAccount,
    /// 当时真实持仓（分侧现值，生产侧=`balance_side(Core{level}, Long)`）——**只**供防线校验，
    /// 不参与 sizing（sizing 恒读开局冻结快照，ADR 补充七 #367 项一 C 裁）。
    pub(super) current_units: i64,
    /// campaign 开局 bar（项三 witness 的「开局以来 bar 数」基准）。
    opened_bar: usize,
    suspension: SuspensionAttribution,
    /// ★#381：本 campaign 的持仓侧——空头侧短差为多头镜像（减=回补空头、补=加回空头），
    /// 镜像的记账实现在 [`ShortDiffAccount`]（同侧构造，一本账不跨侧）。
    pub(super) side: VoiceSide,
    /// ★#383：本 campaign 的 **bar 时钟**——由 [`CampaignBook::sync_position`] 逐 bar 刷新
    /// （生产侧 `drive_campaign_wiring` 对每个重键每 bar 必调一次，且在消费本 bar 动作
    /// **之前**）。`apply_action` 不另收 bar 入参，一切「当前 bar」读数取本字段——单一时钟源，
    /// 避免同一 bar 内两处各传一次 bar 而不一致。
    pub(super) last_sync_bar: usize,
    /// ★#383（ADR 补充八题三）：派发 `EnterEarning` 的那根 bar（`None`=尚未进阶段三）。
    /// **次 bar 换模式**：`last_sync_bar > earning_event_bar` 才生效（事件 bar 收盘后才确认，
    /// 同 bar 换会让同一根 bar 用两本尺）——判据见 [`Self::earning_active`]。
    earning_event_bar: Option<usize>,
    /// ★#383（补充八题一「零头留桶累计」）：阶段三**等金额回补的现金池**——阶段三锁定的
    /// `Reduce` 把该笔桶实收现金记入本池，等金额回补按 `floor(池÷回补价)` 取股数并从池中扣
    /// `股数·回补价`，取整零头**留在池里累计进下次**。
    ///
    /// 与 [`short_diff_bucket::ShortDiffBucket::realized_cash`] 的关系同 `suspension` 与
    /// `open_units`：桶记标量总现金（记账口径），本池只是其中「阶段三待回补」那一份的**算量
    /// 基准**（sizing 口径），不另立第二套记账。
    earning_pool: i64,
}

impl OscillationCampaign {
    /// 开仓生（[`CampaignBook::sync_position`] 唯一构造点）：`notional_in = holding =
    /// snapshot.cost_basis()`（缠师口径「成本入账」——本仓已投入的成本即本 campaign 退本金目标）。
    /// `free=0`（尚未任何短差冲减，现金积累从 0 起步，「现金积累=成本」口径的起点）。
    pub(super) fn open(snapshot: CoreCostBasisSnapshot, bar: usize, side: VoiceSide) -> Self {
        Self {
            tw: TwState {
                holding: snapshot.cost_basis(),
                notional_in: snapshot.cost_basis(),
                ..TwState::initial()
            },
            ledger: LedgerComp::initial(snapshot.cost_basis()),
            short_diff: ShortDiffAccount::new_side(snapshot, side),
            current_units: snapshot.units(),
            opened_bar: bar,
            suspension: SuspensionAttribution::default(),
            side,
            last_sync_bar: bar,
            earning_event_bar: None,
            earning_pool: 0,
        }
    }

    /// ★#383（补充八题三）：阶段三等金额 sizing 是否**已生效**——`EnterEarning` 的**次 bar**
    /// 起为真（同 bar 不换：事件 bar 收盘后才确认，与「取数时点=前 bar 收盘」既有口径一致）。
    pub fn earning_active(&self) -> bool {
        self.earning_event_bar
            .is_some_and(|b| self.last_sync_bar > b)
    }

    /// 派发 `EnterEarning` 的那根 bar（`None`=尚未进阶段三；观测/见证读）。
    pub fn earning_event_bar(&self) -> Option<usize> {
        self.earning_event_bar
    }

    /// 阶段三等金额回补现金池（含累计零头；观测读数，见字段文档）。
    pub fn earning_pool(&self) -> i64 {
        self.earning_pool
    }

    /// 本 campaign 的持仓侧（#381 只读观测）。
    pub fn side(&self) -> VoiceSide {
        self.side
    }

    /// campaign 开局 bar（项三 witness 读）。
    pub fn opened_bar(&self) -> usize {
        self.opened_bar
    }

    /// 开局以来 bar 数（`bar` 早于开局时诚实返回 0，不构造负数）。
    pub fn bars_since_open(&self, bar: usize) -> usize {
        bar.saturating_sub(self.opened_bar)
    }

    /// 当时真实持仓（项二防线读的现值，逐 bar 由 `sync_position` 刷新）。
    pub fn current_units(&self) -> i64 {
        self.current_units
    }

    /// 挂起归属账（项四只读观测）。
    pub fn suspension(&self) -> &SuspensionAttribution {
        &self.suspension
    }

    pub fn tw(&self) -> TwState {
        self.tw
    }

    pub fn ledger(&self) -> LedgerComp {
        self.ledger
    }

    pub fn short_diff(&self) -> ShortDiffAccount {
        self.short_diff
    }

    /// sizing = 开局冻结快照（cost_basis().units()）÷ 3，整数除法。
    /// 基准是 **cost_basis 快照**不是当时真实持仓——#380 项二裁定两个基准分开传
    /// （sizing=开局冻结快照，防线=当时真实持仓）；#348 的「固定 1/3」落在快照侧。
    /// ★可复检——原型口径零发明，如有原文/数据依据可升级。
    fn reduce_units(&self) -> i64 {
        self.short_diff.cost_basis().units() / 3
    }

    /// ★#383（ADR 补充八题一/题三）：一次 `Replenish` 的**分模式 sizing 方案**。
    ///
    /// 本方法取代 #348 的 `replenish_units`（原口径「全额买回当前挂起在途量」= 本方法在
    /// 「无阶段三批次」时的取值，回归锁见测试 `replenish_plan_in_stage_one_is_full_open_units`）。
    ///
    /// 挂起批次按**卖出时锁定**的模式分两族（题三「中途不变规矩」）：
    /// - **阶段一锁定族**（`legacy_open`）：等量买回，逐字节沿用 #348 口径（恒仓约束硬——
    ///   同股数进出，每次触发即整轮收口）；
    /// - **阶段三锁定族**（`earning_open`）：等金额买回——`bought = floor(现金池 ÷ 回补价)`
    ///   （题一：抛出所得现金全额回补、现金不透支、零头留桶累计）。
    ///   `bought > earning_open` ⟹ 多出来的就是**净增股数**（挣股数，[`Self::extra_units`]）；
    ///   `bought < earning_open`（回补价高于卖出价，同一笔钱买不回同样多的货）⟹ 只收口买得起
    ///   的那部分，其余照实继续挂着，不静默补齐。
    ///
    /// 现金池为 0 或该族无挂起（`earning_open == 0`）⟹ 等金额腿整体不成立（`bought = 0`），
    /// 避免在「货已满」时凭空拿池里的零头买股（`ReplenishWhileFull` 语义不被绕过）。
    ///
    /// ★★#414 项三：算量只看**本次回补触发的那个来源中枢**的挂起批次（`source_center`）——
    /// 与 [`SuspensionAttribution::cover`] 的同中枢绑定同一口径，两处必须同源，否则算出的
    /// `close_units` 会超过实际可冲抵量而外溢/落空。
    ///
    /// **已知口径边界（如实标注）**：本方法按模式**分族算量**，而挂起冲抵
    /// （[`SuspensionAttribution::cover`]）按到达序**不看模式**——当 `bought < earning_open`
    /// （买不回同样多）导致只能部分收口时，实际被冲抵的批次可能落在另一族（#414 后两者已同属
    /// 一个中枢，外溢面收窄到该中枢内部）。总量与现金池扣减恒正确（池是标量，非按批次挂账），
    /// 下一次回补按彼时**实际剩余**的批次重算，故不会漂移；受影响的只是「哪一批先收口」这一
    /// 归属观测。现金池不随批次核销（#366 `write_off_unclosed`）归还——核销掉的减出腿其现金
    /// 已实收进桶，池里的那份留着给后续等金额回补用（不冲销，同 [`UnclosedReduction`] 精神）。
    ///
    /// ★现金池的**级别粒度**未变（campaign 粒度=级别，#294 决议 2）：池是本 campaign 的标量，
    /// 不按中枢分账——阶段三某中枢的等金额回补可能花的是另一中枢减出攒下的钱。这是既有粒度的
    /// 直接后果、非本票新引入，如实登记；本票只约束**货**（挂起在途量）的中枢绑定。
    fn replenish_plan(&self, price: i64, source_center: CenterId) -> ReplenishPlan {
        let (legacy_open, earning_open) = self.suspension.open_units_by_mode(source_center);
        let bought = if earning_open > 0 && price > 0 {
            (self.earning_pool / price).max(0)
        } else {
            0
        };
        let earning_close = bought.min(earning_open);
        ReplenishPlan {
            close_units: legacy_open + earning_close,
            extra_units: bought - earning_close,
            pool_spent: bought * price,
            earning_bought: bought,
        }
    }

    /// 单次动作应用：sizing 现算派生 → 短差盈亏桶记账 + TW/R 双账入口（P2-D）→ 阶段推进
    /// （barrier-gated，单一来源 [`stage_progression`]）。
    ///
    /// 不可变：返回 `(新 campaign 实例, 产出)`，调用方（[`CampaignBook`]）负责替换/持有新实例。
    pub fn apply_action(
        &self,
        action: CenterOscillationAction,
        price: i64,
        risk_mode: RiskMode,
        source_center: CenterId,
    ) -> Result<(Self, CampaignOutcome), CampaignViolation> {
        // ★#383（补充八题一）：高抛侧维持持仓 1/3（与阶段一同尺，阶段变化只改回补侧）；
        // 回补侧按挂起批次**卖出时锁定**的模式分流（[`Self::replenish_plan`]）。
        let (units, plan) = match action {
            CenterOscillationAction::Reduce => (self.reduce_units(), ReplenishPlan::inert()),
            CenterOscillationAction::Replenish => {
                let plan = self.replenish_plan(price, source_center);
                (plan.close_units, plan)
            }
        };
        // ★#380 项四：「触发但货满」——买点触发回补但挂起在途量=0（仓位本就满），照实分流为
        // 独立 typed 拒绝，不混入 sizing 取整异常。
        if matches!(action, CenterOscillationAction::Replenish) {
            if self.short_diff.bucket().open_units() == 0 {
                return Err(CampaignViolation::ReplenishWhileFull);
            }
            // ★★#414 项三：桶里有挂起、但**不是这个来源中枢**的 ⟹ 禁静默冲抵，当场拒绝。
            // 顺序在「货满」之后：货本就满（桶总量=0）是既有归因，不被本条改写读数。
            if self.suspension.open_units_of(source_center) == 0 {
                return Err(CampaignViolation::ReplenishForeignCenter {
                    open_units_total: self.suspension.open_units(),
                });
            }
            // ★#383：有挂起、但等金额腿的现金池买不起一股且无等量族可收口——照实分流。
            if units == 0 && plan.extra_units == 0 {
                return Err(CampaignViolation::EarningSizingRoundsToZero {
                    pool: self.earning_pool,
                    price,
                });
            }
        }
        // 高抛侧 1/3 取整到 0 的量纲异常（回补侧的三种零量场景已在上方各自分流，不复用本桶）。
        if matches!(action, CenterOscillationAction::Reduce) && units <= 0 {
            return Err(CampaignViolation::SizingRoundsToZero {
                held: self.short_diff.cost_basis().units(),
            });
        }
        let mut short_diff = self.short_diff;
        // ★#366 订正（评审 §2.4）：本笔**桶实收现金**取记账前后桶 `realized_cash` 之差——
        // 唯一算料来自桶自身，本模块不重算第二套公式（多头 `units·price` / 空头
        // `units·(2·avg_cost−price)` 的两侧差异因此自动正确，见 `SuspensionBatch::cash_booked`）。
        let cash_before = short_diff.bucket().realized_cash();
        // ★#380 项二：sizing 基准=开局冻结快照（上方 `reduce_units`），防线基准=当时真实持仓
        // （`self.current_units`，逐 bar 由 `sync_position` 刷新）——两个基准分开传。
        let (tw_after_action, ledger_after_action) = short_diff.record_and_apply_dual(
            &self.tw,
            &self.ledger,
            action,
            units,
            plan.extra_units,
            price,
            self.current_units,
        )?;
        // ★#383（补充八题二）：阶段三唯一硬门——等金额腿落账后桶现金不得为负。放在记账**之后**
        // 读最终态、拒绝时整笔不落账（`self` 不可变，`short_diff` 是本地副本，返回 Err 即丢弃）。
        // 有效域见 [`CampaignViolation::EarningCashUnsound`]（★2026-07-27 订正：原「按阶段分域」
        // 表述作废）——到 0 判据前置（挂起空 ∧ free≥本金）后，进阶段三时挂起为空 ⟹ 本门约束的
        // 对象只剩等金额腿，floor 取整保证其结构性不可达；阶段一/二 仍走 #380 项一的亏损放行。
        if self.earning_active() && tw_after_action.free < 0 {
            return Err(CampaignViolation::EarningCashUnsound {
                free: tw_after_action.free,
            });
        }
        let cash_delta = short_diff.bucket().realized_cash() - cash_before;
        // ★#380 项四：记账放行后同步挂起归属账（Reduce 入队带中枢标签；Replenish 同中枢优先
        // 冲抵、余按时间序）——与短差桶标量逐笔同步，不另立第二套记账。
        // ★#383：Reduce 把**卖出时**的 sizing 模式锁进批次；阶段三锁定的减出把桶实收现金记入
        // 等金额现金池，回补时按 `floor(池÷价)` 取量并扣 `股数·价`（零头留池累计）。
        let mut suspension = self.suspension.clone();
        let mut earning_pool = self.earning_pool;
        let cover = match action {
            CenterOscillationAction::Reduce => {
                let earning = self.earning_active();
                suspension.push(source_center, units, cash_delta, earning);
                if earning {
                    earning_pool += cash_delta;
                }
                Vec::new()
            }
            CenterOscillationAction::Replenish => {
                earning_pool -= plan.pool_spent;
                suspension.cover(source_center, units)
            }
        };

        // ★到 0 转移（issue #294 验收③）：委托单一来源 stage_progression（#124 裁定4）——
        // 不镜像重写 barrier 判据。κ=0 基线（M7 冻结口径）。
        //
        // ★★#383 订正（2026-07-27 用户裁定，ADR 补充十）：**到 0 判据 = 挂起空 ∧ free≥本金**。
        // `stage_progression` 只看 TW 标量（`free≥本金`），不知桶挂起——它是 TW 层通用算子，
        // 本模块不改其签名/语义（#124 裁定4「单一来源，不镜像重写」不动），改为在 campaign 层
        // 做**派发前置过滤**：本次动作落账后桶仍有挂起在途量 ⟹ 本次**不派发**阶段推进事件
        // （视同未推进，哪怕 free 已够本金）。理由：挂起在途=有货卖出去还没买回来，此刻的 free
        // 里掺着「还欠一笔回补」的钱，拿它去退本金/进增股数等于用未收口的往返充当利润。
        // 该前置**不静默**：被拦下的那次落 [`CampaignOutcome::stage_progress_suspended`]，
        // 生产侧计入 [`CampaignWiringWitness::profit_ready_but_suspended`]。
        let policy = RiskPolicy::baseline();
        let suspended_open = suspension.open_units();
        let progression = stage_progression(&policy, &tw_after_action, risk_mode);
        // 「free 够本金但挂起非空」——前置拦下的次数（观测读数，不参与任何裁决）。
        let stage_progress_suspended = suspended_open > 0 && progression.is_some();
        let (tw_final, stage_event) = match progression.filter(|_| suspended_open == 0) {
            Some(ev) => {
                if !ev.is_legal_from(&tw_after_action) {
                    return Err(CampaignViolation::StageTransition(
                        TransitionError::Oq9Illegal {
                            event: ev,
                            stage: tw_after_action.stage,
                        },
                    ));
                }
                let advanced = tw_step(&tw_after_action, ev);
                let advanced =
                    cash_sound_gate(advanced).map_err(CampaignViolation::StageTransition)?;
                (advanced, Some(ev))
            }
            None => (tw_after_action, None),
        };

        let next = Self {
            tw: tw_final,
            ledger: ledger_after_action,
            short_diff,
            current_units: self.current_units,
            opened_bar: self.opened_bar,
            suspension,
            side: self.side,
            last_sync_bar: self.last_sync_bar,
            // ★#383（题三）：`EnterEarning` 派发即记事件 bar；模式**次 bar** 才生效
            // （[`Self::earning_active`]）——同 bar 不换尺。
            earning_event_bar: match stage_event {
                Some(TwEvent::EnterEarning) => Some(self.last_sync_bar),
                _ => self.earning_event_bar,
            },
            earning_pool,
        };
        let outcome = CampaignOutcome {
            units: units + plan.extra_units,
            tw: tw_final,
            ledger: ledger_after_action,
            stage_event,
            cover,
            // ★#380 项一：本次落账后桶现金为负 ⟹ 亏损往返如实入账（旧口径此笔整笔回滚）。
            loss_accounted: tw_after_action.free < 0,
            earning_replenish: plan.earning_bought > 0,
            earning_units_gained: plan.extra_units,
            stage_progress_suspended,
        };
        Ok((next, outcome))
    }

    /// ★#366/#472/#489：**未闭合减出核销**——终局不回补时，把该来源中枢的挂起批次从在途量
    /// 核销，产出货缺口/桶实收现金两笔分开的呈报（[`UnclosedReduction`]）。来源包括教义三类点
    /// 不回补路径，以及 `RebaseVanished` 工程失踪路径。不构造任何回补动作、不产任何
    /// `TwEvent`（不冲销，见 [`ShortDiffAccount::write_off_unclosed`]）。
    ///
    /// 该中枢无挂起批次 ⟹ `Ok(None)`（无事可核销，非错误——高抛后已自然收口，或该侧本就
    /// 没做过短差）。归属账与桶标量恒等（两者逐笔同步推进），故桶层核销恒不越界；越界即
    /// 记账错误，typed 上报不静默。
    pub(super) fn write_off_unclosed(
        &self,
        level: u32,
        center: CenterId,
    ) -> Result<(Self, Option<UnclosedReduction>), CampaignViolation> {
        let mut suspension = self.suspension.clone();
        let (units_gap, cash_booked) = suspension.write_off(center);
        if units_gap <= 0 {
            return Ok((self.clone(), None));
        }
        let mut short_diff = self.short_diff;
        short_diff.write_off_unclosed(units_gap)?;
        let next = Self {
            short_diff,
            suspension,
            ..self.clone()
        };
        Ok((
            next,
            Some(UnclosedReduction {
                level,
                side: self.side,
                center,
                units_gap,
                cash_booked,
            }),
        ))
    }

    /// 全平死：`TwEvent::ClearCampaign`（不核验 `assert_conserved`，全平即终结）。
    ///
    /// ★★#441（ADR 补充十二，2026-07-27 用户裁定）：**死亡吞挂起 = 未闭合减出核销**——旧口径
    /// 只把在途量当标量读出来叫「作废」（无核销、无现金那一笔、无 witness）；现改为走 #366 的
    /// 同一条核销路径：归属账 [`SuspensionAttribution::write_off_all`] 清账 + 短差桶
    /// [`ShortDiffAccount::write_off_unclosed`] 记账，产出 [`DeathWriteOff`]（货缺口/桶实收现金
    /// 分列，不冲销）。**不产任何 `TwEvent`**（不冲销，同 #366）——`ClearCampaign` 是既有的终结
    /// 事件，与核销无关。
    ///
    /// **留痕在哪**（#441 复审订正）：死亡路径上桶的 `written_off_units` **不可观测**——`close`
    /// 返回后本实例即被丢弃（`campaigns.remove` 已取走所有权），全仓该读数的消费者只有测试与
    /// 活体 campaign（#366 路径）。所以这里写桶的实际作用只剩「给下面那条 `debug_assert` 一个
    /// 检查点」（桶拒绝 ⟹ 记账错误当场炸），**不是**留痕。死亡核销的留痕是产出的
    /// [`DeathWriteOff`] → `CampaignLifecycleEvent::Died` → witness 三桶，别去桶里查。
    ///
    /// **死亡优先于延续**（#414/ADR 补充十一）：campaign 没了，挂起不可能延续到原中枢的三类
    /// 买卖点——出口在此就地闭合。行为化锚见测试
    /// `death_takes_precedence_over_continuation_third_class_settlement_finds_nothing`。
    ///
    /// `ClearCampaign` 在本模块恒合法：本模块只用 `ShortDiff`/`Realize`（[`TwEvent::is_legal_from`]
    /// 对 `ClearCampaign` 的唯一门槛 `open_legacy_legs==0` 恒满足——本模块从不构造
    /// `OpenShareLeg`/`CloseShareLeg`）。
    ///
    /// 返回值第三项 = 逐（中枢）核销明细（★#719 L19②，观测门 dump 用；无核销时为空 Vec）。
    pub(super) fn close(mut self) -> (TwState, Option<DeathWriteOff>, Vec<(CenterId, i64, i64)>) {
        debug_assert!(
            TwEvent::ClearCampaign.is_legal_from(&self.tw),
            "本模块从不用 legacy 腿构造子，ClearCampaign 前置 open_legacy_legs==0 恒满足"
        );
        let (units_gap, cash_booked, breakdown) = self.suspension.write_off_all();
        let settlement = if units_gap > 0 {
            debug_assert_eq!(
                units_gap,
                self.short_diff.bucket().open_units(),
                "归属账与桶标量恒等（逐笔同步推进）——不等即记账错误"
            );
            let bucket_rejected = self.short_diff.write_off_unclosed(units_gap).is_err();
            if bucket_rejected {
                // 结构性不可达：`units_gap` 取自与桶恒等的归属账且 >0，桶核销的两条拒绝分支
                // （非正 / 超出在途量）均不可命中。此处不静默吞真实错误——`debug_assert` 在
                // debug 侧即炸；release 侧经 [`DeathWriteOff::bucket_rejected`] 落 witness
                // `other_violation_*` 警报桶（★#719 L19①：#441 关票移交「桶拒绝落
                // other_violation_by_kind（对齐既有先例）」；生死签名仍不因观测路径改 Result）。
                debug_assert!(false, "桶拒绝了取自归属账的在途量核销（记账错误）");
            }
            Some(DeathWriteOff {
                units_gap,
                cash_booked,
                bucket_rejected,
                centers: breakdown.len(),
            })
        } else {
            None
        };
        (
            tw_step(&self.tw, TwEvent::ClearCampaign),
            settlement,
            breakdown,
        )
    }
}

/// 生产接线观测统计（issue #357 验收③：enabled=true wf8 真实验收——减补动作出现且归属正确、
/// `CenterNotAlive` 丢弃率、挂起归宿、campaign 生死事件均须产物级可见）。
///
/// 门关（`center_oscillation.enabled=false`）⟹ 本结构全程不构造/不喂入，恒 `default()`
/// （诚实空，同 [`super::center_oscillation_trade::CenterOscillationActionRecord`] 先例）——
/// 累计只是**观测读数**，不参与任何裁决/门控，接入/移除不改变生产行为。
#[derive(Debug, Default, Clone)]
pub struct CampaignWiringWitness {
    /// 次级别买卖点触发候选构造尝试总数（`Ok`+`Err` 之和，丢弃率分母）。
    pub trigger_attempts: usize,
    /// `TriggerError::CenterNotAlive` 丢弃数（A 裁定：本级中枢不在场，丢弃率分子）。
    pub dropped_center_not_alive: usize,
    /// 其余触发构造拒绝（`FlatSignal`/`PriceOutsideZone`）丢弃数——与 `CenterNotAlive` 分列，
    /// 不混入同一桶（丢弃成因不同，混计会掩盖「中枢不在场」这一验收关注点）。
    /// ★#366：`PriceOutsideZone` 的**判据**已换（中轴二分 → 离开中枢 ZG/ZD），桶名与归属
    /// 不变——本桶读数在 #366 前后不可跨版本比较（SPEC #386 §6「PriceOutsideZone 桶读数变化
    /// 写入报告」的对照对象）。
    pub dropped_other_trigger: usize,
    /// ★#366：回补侧「中枢不下移」前置过滤（89 课）拒绝数——价格已向下离开中枢、次级别底
    /// 背驰已在，但本级中枢链已下移 ⟹ 不回补。独立分桶（不并入 `dropped_other_trigger`）：
    /// 这是本票新增判据的唯一产物级读数，混入既有桶就无法判定新过滤实际拦了多少。
    pub dropped_center_moved_down: usize,
    /// 减/补动作按 (级别, 持仓侧, 动作) 分桶计数——issue #357 验收「归属正确」的产物级见证。
    /// ★#381：键增持仓侧维（`"long"`/`"short"`）——同一次触发对两侧产出镜像动作，级别+动作
    /// 已不足以定位记账对象。
    pub action_by_level: BTreeMap<(u32, &'static str, &'static str), usize>,
    /// 挂起终结来源分桶（挂起归宿：`BrokenByThirdClassBuy`/`.._Sell`/`RebaseVanished`
    /// 三源，见 [`super::center_oscillation_trade::SuspensionTerminationSource`]）。
    /// ★#381 关票修复：键增持仓侧维 `(侧, 来源)`——`on_lifecycle_event`/`on_chain_rebase` 现在
    /// 对同一事件**逐侧**各产一条 `SuspensionOutcome`，不分侧则多空并存时计数翻倍且无法归属
    /// （破本 ADR 补充九自立的「两侧不得相加」）。
    /// ★★#414 桶退役：`("侧","superseded")` 子桶随 `SuspensionTerminationSource::Superseded`
    /// 变体删除而**退役**——被取代不再是挂起的归宿（ADR 补充十一：中枢终结唯一 = 三类买卖点）。
    /// 其读数（wf8 旧口径 long=67/short=49，占挂起终结 81%）改道进 [`Self::suspension_continued_count`]
    /// 与后续真实清算桶。**跨版本不可比**：本桶在 #414 前后不是同一个量。
    /// ★#442 探针：键增 level 维 `(级别, 侧, 来源)`（纯观测，不改决策）——裁决「容读窗口太窄
    /// vs 高级别不产三类点」需要清算按级别分布，只分侧读不出来。跨版本不可比同上。
    pub suspension_by_source: BTreeMap<(u32, &'static str, &'static str), usize>,
    /// ★#489：Reset 广播到场时主格仍有活中枢的漏发警报，按 `(级别, 一类点侧)` 分桶。
    /// 逐条的 CenterId 与 bar 仍由 `CenterLifecycleEvent::Reset` 及 opsem 事件行承载；本桶只做
    /// wf8 可汇总计数。`died_center=None` 的场空 Reset 是合法广播，不计警报。
    pub reset_alive_center_leak_by_level_side: BTreeMap<(u32, &'static str), usize>,
    /// ★#489 修复车：Reset 事件收到任何非空生命周期消费输出的次数。按事件计数；终结、
    /// 核销、回补或延续任一路非空都击中，作为 Reset 广播意外产生清算副作用的活回归门。
    reset_nonempty_lifecycle_outcome_count: usize,
    /// ★★#414（ADR 补充十一）：挂起**延续**计数（分侧）——`Superseded` 命中挂起、挂起不终结
    /// 也不清算的次数。正面读数，非警报；它与真实清算桶（`suspension_by_source` 的两个
    /// `broken_by_third_class_*` + `settlement_by_side`）的差额即「延续了但没等到三类点」的
    /// 残量，见 [`super::center_oscillation_trade::CenterOscillationBook::on_lifecycle_event`]
    /// 的有效域标注（容读窗口只保链尾前一格）。
    /// ★#442 探针：键增 level 维 `(级别, 侧)`（纯观测）——延续集中在哪个级别是本探针的裁决量。
    pub suspension_continued_count: BTreeMap<(u32, &'static str), usize>,
    /// ★#414/#472：终结的**清算终局**分流（键
    /// `(级别, 侧, "cover_and_close"|"write_off_unclosed")`）。与
    /// [`Self::suspension_by_source`] 是同一批终结的两个正交切面（来源 vs 终局），两桶的分侧
    /// 总数恒相等（同一 `SuspensionOutcome` 各记一次），可互为对账。
    /// ★#442 探针：键增 level 维 `(级别, 侧, 终局)`（纯观测）——与 `suspension_by_source` 同步
    /// 加维，两桶逐 (级别, 侧) 总数仍恒相等，对账关系不因加维破坏。
    pub settlement_by_side: BTreeMap<(u32, &'static str, &'static str), usize>,
    /// ★#487：精确挂起绑定授权的 historical-bound 投递**送达**数（按级别，含 `Alive`/
    /// `Tolerated`/`HistoricalBound` 三分支——只要 `broken_total` 因这次投递而增量即计一次）。
    /// 这是显式路由分支的审计读数；不参与任何清算判据。
    ///
    /// ★#689 教训（两轮审计曾被误导）：本桶原文档曾写成「目标已滑出 alive/prev_slot 的
    /// historical-bound 路由命中数」——被自家反证臂实测证伪（main HEAD 该桶 `={}`，反证臂
    /// `={1: 6}`，这 6 次恰恰**没有**滑出主格/容读格，仍计入本桶）。真实语义是「投递送达」，
    /// 不区分具体落在哪个分支。
    ///
    /// 与 [`Self::historical_bound_attempted_by_level`] / [`Self::historical_bound_failed_by_level`]
    /// 并读，但**三桶不构成恒等式**：`attempted` 落在调用 `push_point_historical_bound` 之前，
    /// 该次调用若解到 `KillResolution::ArenaEmpty` 则返回 `Ok(PointOutcome::Silent)`
    /// （`center_lifecycle.rs:687`）——既不是送达（本桶不动）也不是 `MisKill`（failed 桶不动）。
    /// 同 bar 内前一个 bound 点先杀掉主格、后续 bound 点落空即走这条路径（#487/A3 多 Owner
    /// 同事件的常规形态）。故只保证 `attempted >= 本桶 + failed`，wf8 未触发不等于该路径不存在。
    pub historical_bound_by_level: BTreeMap<u32, usize>,
    /// ★#689：historical-bound 投递**尝试**总数（按级别，成功+失败之和）——修那个「只认成功
    /// 分支」的失明：不管这次尝试最终落 [`Self::historical_bound_by_level`] 还是
    /// [`Self::historical_bound_failed_by_level`]，尝试本身先在这里落一次。
    pub historical_bound_attempted_by_level: BTreeMap<u32, usize>,
    /// ★#689：historical-bound 投递尝试后以 `MisKill` 收场的次数（按级别）——「通道被调用但
    /// 失败」的显式可见读数，专治 [`Self::historical_bound_by_level`] 文档所述的失明教训。
    pub historical_bound_failed_by_level: BTreeMap<u32, usize>,
    /// ★#689：`CenterEventMachine::historical_bound_kills()` 逐级别快照（run 结束时一次性
    /// 采集，非逐事件累加）——「#487 特批分支命中数」在报告行的生产可查读数。此前该数字只
    /// 活在 `cl_machines` 内部，报告行完全没接，出现「查不出特批分支到底有没有命中」的失明
    /// （评审 #689 面2 指出的反向新增）。
    pub historical_bound_kills_by_level: BTreeMap<u32, usize>,
    /// ★#689：`CenterEventMachine::historical_bound_reroutes()` 逐级别快照，采集方式同
    /// [`Self::historical_bound_kills_by_level`]——改道成功命中数的生产可查读数。
    pub historical_bound_reroutes_by_level: BTreeMap<u32, usize>,
    /// ★#487/A3：同一 `(source_index, 买卖侧)` 紧邻证同时绑定多个冻结 Owner 时，
    /// 每多出一个 Owner 记一次（按级别）。这是合法多投递的观测读数，不参与路由或清算判据。
    pub historical_multi_owner_same_event: BTreeMap<u32, usize>,
    /// ★#487：生命周期路由拒绝错杀的 `CenterMisKill` 次数（按级别）；验收要求全级别为 0。
    pub center_mis_kill_by_level: BTreeMap<u32, usize>,
    /// ★#414 项三：**异中枢回补被拒**计数（分侧）——桶里有挂起、但**不是**本次回补触发的那个
    /// 来源中枢的（[`CampaignViolation::ReplenishForeignCenter`]）。旧口径此路径静默走「余按
    /// 时间序」冲抵别的中枢的挂起（wf8 产物级证据 `cover_by_side ("long","other_center"):3`），
    /// 本票禁止之：挂起绑定来源中枢，只能被同中枢回补或原中枢三类点清算。预期经济场景，非警报。
    pub replenish_foreign_center_count: BTreeMap<&'static str, usize>,
    /// campaign 开仓生事件计数（[`CampaignLifecycleEvent::Opened`]）。
    /// ★#381 关票修复：改分侧分桶（键=`"long"`/`"short"`）——事件本身已带 `side`，标量相加是
    /// 跨侧求和，既破「两侧不得相加」，也使多头侧生死零漂移不可逐条核验。
    pub lifecycle_opened: BTreeMap<&'static str, usize>,
    /// campaign 全平死事件计数（[`CampaignLifecycleEvent::Died`]）。★#381：同上，改分侧分桶。
    pub lifecycle_died: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇 [`CampaignViolation::NoActiveCampaign`] 的计数——★非接线错误：
    /// `CenterOscillationBook`（#292 T2）的减/补决策独立于本级 Core 持仓状态（「无门」设计，
    /// 见该模块文档），故结构信号在本级空仓时触发是**真实经济场景**（结构说做、但当前无仓可
    /// 操作），非 bug。真实生产数据（wf8 实测）此项非零属预期读数，报告层照实呈现即可，
    /// 不得断言恒 0（`other_violation_count` 才是真正的接线错误警报）。
    ///
    /// ★#381 桶退役注记：`unsupported_short_position_count`（#357 关票条件 C 落的
    /// 「本级持有空头仓位、但本接线只支持多头 campaign」缺口桶）随空头 campaign 实装**退役**
    /// ——空头侧持仓现在正常开局 campaign 并正常记账（★#880：按重分账、侧为开局属性、
    /// 不进键），不再存在「有仓但认不出」这一形态；空头侧的 `NoActiveCampaign` 与多头侧同义
    /// （真空仓，预期经济场景），归入
    /// [`Self::no_active_campaign_count`]。★#381：改**分侧**分桶（键=`"long"`/`"short"`）——
    /// 两侧同义但归属不同，混计后多空并存时无法判读是哪一侧空仓。
    pub no_active_campaign_count: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇 [`CampaignViolation::ShortDiff`]`(`[`ShortDiffViolation::ChannelRejected`]`(`
    /// [`TransitionError::CashUnsound`]`{ holding<0, .. }`」的计数——★同样非接线错误，而是
    /// **预算耗尽的真实经济场景**：`OscillationCampaign::reduce_units`/`replenish_plan`
    /// （#294/#348 既有落地代码，本票不改动）按**campaign 开局时冻结的总量**算 sizing（非
    /// 「当前剩余可减仓量」），而 `CampaignBook` 按**级别**（非按中枢）聚合——同级多个中枢各自
    /// 独立触发 `Reduce` 会共享同一份 `holding` 预算；当连续同向触发次数超过冻结总量所能支撑
    /// 的轮次（如原始持仓 3 等分后再遇第 4 次 `Reduce`），`cash_sound_gate` 显式拒绝（`holding`
    /// 将变负）——这正是「违规显式失败」设计纪律的产物级见证（非静默钳制/吸收），资源约束下的
    /// 诚实拒绝，非本票引入的接线缺陷。
    ///
    /// ★issue #357 关票条件 B（拆因）：`resource_exhausted_count`（单桶）曾把
    /// [`TransitionError`] 内层错误下划线丢弃——本路径 OQ-9 恒真（`ShortDiff`/`Realize` 的
    /// `is_legal_from` 恒真），故必为 `CashUnsound`；但 `CashUnsound` 有两条含义完全不同的
    /// 成因：`holding<0`（本字段，连续 Reduce 打穿冻结预算）与 `free<0`
    /// （回补价高于卖出价的亏损往返——★#380 项一后该族改为放行入账，原
    /// `resource_exhausted_free_negative_count` 桶退役，接替它的正面读数见
    /// [`Self::loss_round_trip_accounted_count`]）。
    /// 拆桶后归因才可从读数上直接判定，不再靠推断。★#381：改分侧分桶（同上）。
    pub resource_exhausted_holding_negative_count: BTreeMap<&'static str, usize>,
    /// ★#380 项一（ADR 补充七，#367 项四 A 裁）：**亏损往返如实入账**的计数——`Reduce` 使
    /// `free += units·p_sell`、`Replenish` 使 `free -= units·p_buy`，`p_buy > p_sell`（高抛低吸
    /// 反着做）即桶现金转负。旧口径（#357）此路径被 `cash_sound_gate` 拒绝、整笔回滚，落
    /// `resource_exhausted_free_negative_count` 桶——**该桶本票退役**（新通道
    /// [`super::short_diff_bucket`] 的 `short_diff_cash_gate` 下已不可达；若仍到达即为记账错误，
    /// 归入 `other_violation_by_kind` 的 `cash_unsound_free_negative_unreachable`）。本桶接替
    /// 观测同一现象的**正面**读数：这些往返现在如实进账，`realized_cash` 收负。
    ///
    /// ★#381：改**分侧**分桶——两侧不得相加。严格语义仍是「落账后桶现金为负的动作次数」，
    /// 但空头侧的**单腿**桶现金不等于单笔成交现金（`Realize` 镜像，见
    /// [`super::short_diff_bucket::ShortDiffBucket::realized_cash`] 的口径注记），故两侧读数
    /// 同名不同义，报告层须分列呈现、不得相加。
    pub loss_round_trip_accounted_count: BTreeMap<&'static str, usize>,
    /// ★#380 项四：`Replenish` 触发但**货已满**（挂起在途量=0）的计数——
    /// [`CampaignViolation::ReplenishWhileFull`]，预期经济场景（结构说回补、仓位本就满），
    /// 不与 `other_violation_count`（记账错误警报）混计。★#381：改分侧分桶。
    pub replenish_triggered_but_full_count: BTreeMap<&'static str, usize>,
    /// ★#380 项二：防线（读**当时真实持仓**）拒绝的超卖计数——
    /// [`ShortDiffViolation::UnitsExceedCostBasis`]。按冻结快照算出的 sizing 超过当时真实持仓
    /// （主仓其间被减仓）⟹ 「不卖没有的货」显式拒绝。这是**预期读数**（基准与现值分离的必然
    /// 副产物），非接线错误，故与 `other_violation_count` 分列。★#381：改分侧分桶。
    pub defense_units_exceed_current_holding_count: BTreeMap<&'static str, usize>,
    /// ★#380 项三：阶段推进事件（`RecoverCapital`/`EnterEarning`）计数（见
    /// [`Self::stage_events`] 的逐条明细）。
    pub stage_recover_capital_count: usize,
    pub stage_enter_earning_count: usize,
    /// ★★#383 订正（2026-07-27 用户裁定，ADR 补充十）：**「到 0 掺水被拦」计数**（分侧）——
    /// 本次动作落账后 `free` 已够本金（`stage_progression` 会派 `RecoverCapital`/`EnterEarning`），
    /// 但桶挂起在途量非空 ⟹ 阶段推进被 campaign 层前置过滤拦下（见
    /// [`OscillationCampaign::apply_action`] 的前置注释）。
    ///
    /// **不是警报**：这是判据的正面读数（有货没买回来时不认「到 0」），预期非零。它与
    /// [`Self::stage_recover_capital_count`]/[`Self::stage_enter_earning_count`] 构成同一判据的
    /// 两侧读数：拦下 N 次、放行 M 次。恒 0 反而说明该判据在本窗从未 binding（此时前置过滤
    /// 与旧口径等价），#384 终验须分列呈现、不得与 `other_violation_*` 警报桶混计。
    pub profit_ready_but_suspended: BTreeMap<&'static str, usize>,
    /// ★#380 项三：阶段推进事件逐条明细（campaign 标识=级别 + 开局以来 bar 数 + 金额）——
    /// `fill.rs` 旧版 `Ok(_outcome) => {}` 把 `stage_event` 整个丢弃，#368 切换开关因此无物可读。
    pub stage_events: Vec<StageEventRecord>,
    /// ★#383（#383 报告层三项之二）：**等金额回补次数**（分侧）——回补动作中含阶段三锁定批次
    /// 的次数。与 `action_by_level` 的 `replenish` 桶是包含关系（后者含两种口径），分列呈报。
    pub earning_replenish_count: BTreeMap<&'static str, usize>,
    /// ★#383（报告层三项之一，补充八题二「股数不设门 ⟹ 改作观测读数」）：阶段三**累计净增
    /// 股数**（分侧）——每次等金额回补里超出挂起在途量、用桶现金新买的那部分之和。两侧不得相加。
    pub earning_units_gained: BTreeMap<&'static str, i64>,
    /// ★#383（报告层三项之三）：**切换时点读数**——键 `(级别, 侧)`，值 = 等金额 sizing
    /// **生效**的 bar（= `EnterEarning` 事件 bar + 1，题三「次 bar 换模式」）。
    pub earning_mode_switch_bar: BTreeMap<(u32, &'static str), usize>,
    /// ★#383：阶段三硬门（桶现金永不为负）被触发的计数（分侧）——★2026-07-27 订正（ADR
    /// 补充十）：到 0 判据前置（挂起空 ∧ free≥本金）后，阶段三无跨阶段挂起批次 ⟹ 本门约束的
    /// 对象只剩等金额腿，`floor` 取整保证其不透支 ⟹ **恒 0，非 0 即警报**（补充八题二原论证句
    /// 恢复成立）。唯一须先排除的非警报路径 = `EnterEarning` 事件 bar 上（尺未换）落下的阶段一
    /// 锁定批次跨到次 bar 后亏损收口，见 [`CampaignViolation::EarningCashUnsound`] 的残余窗口注。
    pub earning_cash_unsound_count: BTreeMap<&'static str, usize>,
    /// ★#383：等金额回补触发但现金池买不起一股的计数（分侧）——预期经济场景，独立分桶。
    pub earning_sizing_rounds_to_zero_count: BTreeMap<&'static str, usize>,
    /// ★#380 项四：回补冲抵按「同中枢/异中枢」分桶（同中枢优先的产物级见证）。
    /// ★#381：键增持仓侧维——`(侧, "same_center"|"other_center")`，多空并存时冲抵归属可判读
    /// （SPEC #386 §2「witness 呈现归属与冲抵顺序」对两侧对称适用）。
    pub cover_by_side: BTreeMap<(&'static str, &'static str), usize>,
    /// ★#366/#472：**未闭合减出**实际核销的分侧计数。
    pub unclosed_write_off_count: BTreeMap<&'static str, usize>,
    /// ★#366：核销的**货缺口**累计（分侧，单位=股数）——与下方桶实收现金**分列呈报，不相减**
    /// （相减＝冲销＝装没发生，见 [`UnclosedReduction`]）。
    pub unclosed_write_off_units_gap: BTreeMap<&'static str, i64>,
    /// ★#366：核销的**桶实收现金**累计（分侧，单位=现金）——见上，不与货缺口冲销。
    ///
    /// ★口径订正（2026-07-27，评审 §2.4）：本桶旧名 `unclosed_write_off_cash_surplus`、旧口径
    /// `Σ units·price`，空头侧值与符号双错（空头「减」是买回＝现金支出，非「卖出成交额」）。
    /// 现口径 = 各减出腿记进桶 `realized_cash` 的实收增量之和，两侧同名不同义的分列声明见
    /// [`UnclosedReduction`]。**跨版本不可比**：本桶在 #366 订正前后不是同一个量。
    pub unclosed_write_off_cash_booked: BTreeMap<&'static str, i64>,
    /// ★★#441（ADR 补充十二，2026-07-27 用户裁定）：**死亡吞挂起**核销的分侧计数——campaign
    /// 全平死亡时名下尚有未收口挂起的次数（[`CampaignLifecycleEvent::Died`] 带
    /// [`DeathWriteOff`] 的那些）。
    ///
    /// 与上方结构终结核销桶（`unclosed_write_off_*`，#366/#472）**分列、不混计**：两者虽同口径
    /// （未闭合减出核销），但归宿成因不同——上方由中枢三类点或止血来源逐中枢请求核销，本桶由
    /// **campaign** 死（主仓全平）把名下所有中枢挂起一并灭失。混桶就读不出「盲区有多大」。
    ///
    /// 与 [`Self::lifecycle_died`] 的关系：本桶 ≤ 死亡总数，差额=死时挂起为空的那些（干净死）
    /// ——「无可核销」不另立桶（可由两桶相减读出），不编造零记录。
    ///
    /// **分桶键只分侧**（与 #366 同族三桶、[`Self::lifecycle_died`] 同规格）：级别维是 #442
    /// 探针的裁决对象，其加维范围由该票统一定；本票不单独扩键，避免同族桶规格分叉。
    pub death_write_off_count: BTreeMap<&'static str, usize>,
    /// ★#441：死亡吞挂起的**货缺口**累计（分侧，单位=股数）——与下方桶实收现金**分列呈报，
    /// 不相减**（相减＝冲销＝装没发生，见 [`UnclosedReduction`] 的同一论证）。
    pub death_write_off_units_gap: BTreeMap<&'static str, i64>,
    /// ★#441：死亡吞挂起的**桶实收现金**累计（分侧，单位=现金）——口径同
    /// [`Self::unclosed_write_off_cash_booked`]（各减出腿记进 `realized_cash` 的增量之和），
    /// 两侧同名不同义（空头侧「减」=买回），报告层分列呈现、不得相加。
    pub death_write_off_cash_booked: BTreeMap<&'static str, i64>,
    /// ★#366/#472：未闭合减出核销请求到达但**无可核销**的分侧计数（该侧空仓无 campaign，
    /// 或该中枢本就没有挂起批次）——照实计数，不与真实核销读数混计。
    pub unclosed_write_off_nothing_to_settle: BTreeMap<&'static str, usize>,
    /// `apply_action` 遇其余 [`CampaignViolation`]（`AvgCostMismatch`/`NonPositiveUnits`/
    /// `OverReplenish`/`UnclosedRoundTrip`/`UnitsExceedCostBasis`/`StageTransition`/
    /// `SizingRoundsToZero`）的计数——诊断用，生产路径正常接线下恒 0（非 0 = 真正的接线/
    /// 记账逻辑错误，与 `no_active_campaign_count`/`resource_exhausted_count` 的「预期读数」
    /// 性质不同，不得混桶）。
    pub other_violation_count: usize,
    /// `other_violation_count` 的细分诊断分桶（变体名标签，如 `sizing_rounds_to_zero`/
    /// `short_diff_units_exceed_cost_basis` 等）——定位非 0 读数的具体成因用。
    /// ★#381：键增持仓侧维 `(侧, 变体名)`。
    pub other_violation_by_kind: BTreeMap<(&'static str, &'static str), usize>,
}

/// ★#381：持仓侧的稳定标签（witness 分桶键/明细用，`&'static str` 与既有动作标签同规格）。
fn side_label(side: VoiceSide) -> &'static str {
    match side {
        VoiceSide::Long => "long",
        VoiceSide::Short => "short",
        VoiceSide::Flat => "flat",
    }
}

/// ★#380 项三：一条阶段推进事件的见证记录（campaign 标识=级别；`bars_since_open`=开局以来
/// bar 数；`amount`=事件金额，`RecoverCapital(w)`=退本金额；`EnterEarning` 无载荷 ⟹ 记 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageEventRecord {
    pub level: u32,
    /// ★#381：本事件所属 campaign 的持仓侧。
    pub side: &'static str,
    pub bar: usize,
    pub bars_since_open: usize,
    pub kind: &'static str,
    pub amount: i64,
}

impl CampaignWiringWitness {
    pub fn new() -> Self {
        Self::default()
    }

    /// ★#380 项三：记一次阶段推进事件（`RecoverCapital`/`EnterEarning`）——其余 `TwEvent`
    /// 不是阶段推进事件，不记（`stage_progression` 只派这两种，此处是防御性 match）。
    pub fn record_stage_event(
        &mut self,
        level: u32,
        side: VoiceSide,
        bar: usize,
        bars_since_open: usize,
        event: TwEvent,
    ) {
        let (kind, amount) = match event {
            TwEvent::RecoverCapital(w) => {
                self.stage_recover_capital_count += 1;
                ("recover_capital", w)
            }
            // `EnterEarning` 是无载荷变体（相变本身即事件），金额记 0——不编造数值。
            TwEvent::EnterEarning => {
                self.stage_enter_earning_count += 1;
                // ★#383 报告层「切换时点读数」：等金额 sizing 的**生效** bar = 事件 bar + 1
                // （题三「次 bar 换模式」），与 [`OscillationCampaign::earning_active`] 同判据。
                self.earning_mode_switch_bar
                    .insert((level, side_label(side)), bar + 1);
                ("enter_earning", 0)
            }
            _ => return,
        };
        self.stage_events.push(StageEventRecord {
            level,
            side: side_label(side),
            bar,
            bars_since_open,
            kind,
            amount,
        });
    }

    /// ★★#383 订正（ADR 补充十）：记一次「free 够本金、但挂起非空 ⟹ 阶段推进被前置拦下」
    /// （见 [`Self::profit_ready_but_suspended`]；正面读数，非警报）。
    pub fn record_profit_ready_but_suspended(&mut self, side: VoiceSide) {
        *self
            .profit_ready_but_suspended
            .entry(side_label(side))
            .or_insert(0) += 1;
    }

    /// ★#380 项一：记一次「亏损往返如实入账」（本次动作落账后桶现金为负）。
    pub fn record_loss_accounted(&mut self, side: VoiceSide) {
        *self
            .loss_round_trip_accounted_count
            .entry(side_label(side))
            .or_insert(0) += 1;
    }

    /// ★#383：记一次**等金额回补**（次数 + 本次净增股数；`gained` 可为 0——回补价高于卖出价
    /// 时同一笔钱买不回同样多的货，照实记 0 不编造）。
    pub fn record_earning_replenish(&mut self, side: VoiceSide, gained: i64) {
        let sl = side_label(side);
        *self.earning_replenish_count.entry(sl).or_insert(0) += 1;
        *self.earning_units_gained.entry(sl).or_insert(0) += gained;
    }

    /// ★#380 项四：记一次回补的逐批冲抵明细（同中枢/异中枢分桶，见证「同中枢优先」）。
    pub fn record_cover(&mut self, side: VoiceSide, cover: &[CoverAssignment]) {
        for c in cover {
            let bucket = if c.same_center {
                "same_center"
            } else {
                "other_center"
            };
            *self
                .cover_by_side
                .entry((side_label(side), bucket))
                .or_insert(0) += 1;
        }
    }

    /// 记一次触发构造尝试的结果（`Ok`/`Err` 分布，丢弃率分母+分子）。
    pub fn record_trigger_result<T>(&mut self, result: &Result<T, TriggerError>) {
        self.trigger_attempts += 1;
        match result {
            Ok(_) => {}
            Err(TriggerError::CenterNotAlive) => self.dropped_center_not_alive += 1,
            // ★#366：新判据的专桶，不并入 `dropped_other_trigger`。
            Err(TriggerError::CenterMovedDown) => self.dropped_center_moved_down += 1,
            Err(TriggerError::FlatSignal) | Err(TriggerError::PriceOutsideZone) => {
                self.dropped_other_trigger += 1
            }
        }
    }

    /// ★#366：记一次「未闭合减出」核销（货缺口与桶实收现金**分列**累计，不相减）。
    pub fn record_write_off(&mut self, reduction: &UnclosedReduction) {
        let sl = side_label(reduction.side);
        *self.unclosed_write_off_count.entry(sl).or_insert(0) += 1;
        *self.unclosed_write_off_units_gap.entry(sl).or_insert(0) += reduction.units_gap;
        *self.unclosed_write_off_cash_booked.entry(sl).or_insert(0) += reduction.cash_booked;
    }

    /// ★#366/#472：记一次「核销请求到达但无可核销」（该侧空仓，或该中枢无挂起批次）。
    pub fn record_write_off_nothing_to_settle(&mut self, side: VoiceSide) {
        *self
            .unclosed_write_off_nothing_to_settle
            .entry(side_label(side))
            .or_insert(0) += 1;
    }

    /// 记一次挂起终结来源（挂起归宿分桶）。★#381 关票修复：按 (持仓侧, 来源) 分桶——
    /// `side` 即该终结所在 `SuspensionOutcome` 的持仓侧，调用点已在手。
    /// ★#442：加 `level`（调用点的 `lvl`，同样已在手），纯观测。
    pub fn record_suspension_source(
        &mut self,
        level: u32,
        side: VoiceSide,
        source: SuspensionTerminationSource,
    ) {
        let label = match source {
            SuspensionTerminationSource::BrokenByThirdClassBuy => "broken_by_third_class_buy",
            SuspensionTerminationSource::BrokenByThirdClassSell => "broken_by_third_class_sell",
            SuspensionTerminationSource::RebaseVanished => "rebase_vanished",
            SuspensionTerminationSource::ConstructionRemoved => "construction_removed",
        };
        *self
            .suspension_by_source
            .entry((level, side_label(side), label))
            .or_insert(0) += 1;
    }

    /// ★#489：把 Reset 事件里的活中枢只读见证记入漏发警报桶；不参与任何终结或清算判据。
    pub fn record_reset_alive_center_leak(&mut self, event: &CenterLifecycleEvent) {
        let CenterLifecycleEvent::Reset {
            level,
            died_center: Some(_),
            trigger_side,
            ..
        } = event
        else {
            return;
        };
        let side = match trigger_side {
            Side::Long => "long",
            Side::Short => "short",
        };
        *self
            .reset_alive_center_leak_by_level_side
            .entry((*level, side))
            .or_insert(0) += 1;
    }

    /// ★#489 修复车：Reset 的实际生命周期消费输出只要有一路非空，就按事件记一次。
    pub fn record_reset_termination_outcome(
        &mut self,
        event: &CenterLifecycleEvent,
        outcome: &SuspensionEventOutcome,
    ) {
        if matches!(event, CenterLifecycleEvent::Reset { .. })
            && (!outcome.terminations.is_empty() || !outcome.continuations.is_empty())
        {
            self.reset_nonempty_lifecycle_outcome_count += 1;
        }
    }

    /// ★#489 wf8 验收读数：Reset 实际产生任何终结/核销/回补/延续输出的事件数。
    pub fn reset_settlement_count(&self) -> usize {
        self.reset_nonempty_lifecycle_outcome_count
    }

    /// ★#489 wf8 验收读数：两类 Broken 清算按级别汇总；工程重基失踪不混入。
    pub fn broken_by_level(&self) -> BTreeMap<u32, usize> {
        let mut totals = BTreeMap::new();
        for ((level, _, source), count) in &self.suspension_by_source {
            if matches!(
                *source,
                "broken_by_third_class_buy" | "broken_by_third_class_sell"
            ) {
                *totals.entry(*level).or_insert(0) += *count;
            }
        }
        totals
    }

    /// ★#487/#489 题三挂钩：跨侧终结 = 多头被三卖 + 空头被三买；不得跨侧求和猜测。
    pub fn cross_side_termination_count(&self) -> usize {
        self.suspension_by_source
            .iter()
            .filter(|((_, side, source), _)| {
                (*side == "long" && *source == "broken_by_third_class_sell")
                    || (*side == "short" && *source == "broken_by_third_class_buy")
            })
            .map(|(_, count)| *count)
            .sum()
    }

    /// ★★#414（ADR 补充十一）：记一次挂起**延续**（`Superseded` 命中挂起 ⟹ 不终结、不清算）。
    /// 与 [`Self::suspension_by_source`] 分列——延续不是归宿，混进归宿桶等于把「什么都没发生」
    /// 记成一次终结。
    /// ★#442：加 `level`（纯观测）——延续的级别分布是探针的裁决量。
    pub fn record_suspension_continued(&mut self, level: u32, side: VoiceSide) {
        *self
            .suspension_continued_count
            .entry((level, side_label(side)))
            .or_insert(0) += 1;
    }

    /// ★#414/#472：记一次终结的**清算终局**分流（闭合/核销二分）。与
    /// [`Self::suspension_by_source`]（按**来源**分桶）是同一批终结的两个正交切面：
    /// 来源答「谁杀的」，终局答「账怎么了结」。
    /// ★#442：加 `level`（纯观测）——与来源桶同步加维，保持两切面逐 (级别, 侧) 可对账。
    pub fn record_settlement(
        &mut self,
        level: u32,
        side: VoiceSide,
        settlement: TerminationSettlement,
    ) {
        let label = match settlement {
            TerminationSettlement::CoverAndClose => "cover_and_close",
            TerminationSettlement::WriteOffUnclosed => "write_off_unclosed",
        };
        *self
            .settlement_by_side
            .entry((level, side_label(side), label))
            .or_insert(0) += 1;
    }

    /// ★#487：记一次已滑出容读窗的精确 historical-bound 命中（成功分支）。
    pub fn record_historical_bound(&mut self, level: u32) {
        *self.historical_bound_by_level.entry(level).or_insert(0) += 1;
    }

    /// ★#689：记一次 historical-bound 投递尝试（成功/失败之前，调用点先落一次）。
    pub fn record_historical_bound_attempted(&mut self, level: u32) {
        *self
            .historical_bound_attempted_by_level
            .entry(level)
            .or_insert(0) += 1;
    }

    /// ★#689：记一次 historical-bound 投递尝试以 `MisKill` 收场（失败分支，专治「只认成功
    /// 分支」的失明——见 [`Self::historical_bound_by_level`] 文档的两轮审计教训）。
    pub fn record_historical_bound_failed(&mut self, level: u32) {
        *self
            .historical_bound_failed_by_level
            .entry(level)
            .or_insert(0) += 1;
    }

    /// ★#487/A3：记一次同事件新增的合法冻结 Owner 绑定。
    pub fn record_historical_multi_owner_same_event(&mut self, level: u32) {
        *self
            .historical_multi_owner_same_event
            .entry(level)
            .or_insert(0) += 1;
    }

    /// ★#487：记一次生命周期路由的错杀请求。
    pub fn record_center_mis_kill(&mut self, level: u32) {
        *self.center_mis_kill_by_level.entry(level).or_insert(0) += 1;
    }

    /// 记一次动作按 (级别, 动作) 分桶（归属正确性见证）。
    pub fn record_action(&mut self, level: u32, side: VoiceSide, action: CenterOscillationAction) {
        let label = match action {
            CenterOscillationAction::Reduce => "reduce",
            CenterOscillationAction::Replenish => "replenish",
        };
        *self
            .action_by_level
            .entry((level, side_label(side), label))
            .or_insert(0) += 1;
    }

    /// 记一次 campaign 生死事件。★#381 关票修复：按事件自带的 `side` 分侧累计（两侧不得相加）。
    pub fn record_lifecycle(&mut self, event: CampaignLifecycleEvent) {
        match event {
            CampaignLifecycleEvent::Opened { side, .. } => {
                *self.lifecycle_opened.entry(side_label(side)).or_insert(0) += 1;
            }
            // ★#441（ADR 补充十二）：死亡吞挂起的核销读数在此落桶——生死计数不变（每次死亡
            // 恒记一次），核销三桶只在带挂起的那些死亡上累计（干净死不编造零记录）。
            CampaignLifecycleEvent::Died {
                side, settlement, ..
            } => {
                let sl = side_label(side);
                *self.lifecycle_died.entry(sl).or_insert(0) += 1;
                if let Some(w) = settlement {
                    *self.death_write_off_count.entry(sl).or_insert(0) += 1;
                    *self.death_write_off_units_gap.entry(sl).or_insert(0) += w.units_gap;
                    *self.death_write_off_cash_booked.entry(sl).or_insert(0) += w.cash_booked;
                    // ★#719（L19①）：桶拒绝（结构性不可达，true=记账错误）落警报桶——对齐
                    // `cash_unsound_free_negative_unreachable` 先例（计数+分桶双落）。
                    if w.bucket_rejected {
                        self.other_violation_count += 1;
                        *self
                            .other_violation_by_kind
                            .entry((sl, "death_write_off_bucket_rejected_unreachable"))
                            .or_insert(0) += 1;
                    }
                }
            }
        }
    }

    /// 记一次 `apply_action` 拒绝。★#381：`is_short_side_held` 入参随
    /// `unsupported_short_position_count` 桶一同退役——空头侧已有自己的 campaign
    /// （★#880：按重分账、侧为开局属性、不进键），`NoActiveCampaign` 在两侧同义（该侧真空仓，
    /// 结构信号独立于持仓的预期场景）。
    ///
    /// 其余变体按既定分桶（`ShortDiff(ChannelRejected(CashUnsound))` 拆 holding/free 两子桶，
    /// issue #357 关票条件 B；其余是真正的接线/记账错误警报，见字段文档，不得混桶）。
    pub fn record_violation(&mut self, side: VoiceSide, violation: CampaignViolation) {
        let sl = side_label(side);
        match violation {
            CampaignViolation::NoActiveCampaign => {
                *self.no_active_campaign_count.entry(sl).or_insert(0) += 1;
            }
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { holding, .. },
            )) if holding < 0 => {
                *self
                    .resource_exhausted_holding_negative_count
                    .entry(sl)
                    .or_insert(0) += 1;
            }
            // ★#380 项四：「触发但货满」——预期经济场景，独立分桶。
            CampaignViolation::ReplenishWhileFull => {
                *self
                    .replenish_triggered_but_full_count
                    .entry(sl)
                    .or_insert(0) += 1;
            }
            // ★#414 项三：「有货但不是这个中枢的」——预期经济场景，与「货满」分列。
            CampaignViolation::ReplenishForeignCenter { .. } => {
                *self.replenish_foreign_center_count.entry(sl).or_insert(0) += 1;
            }
            // ★#380 项二：防线（读当时真实持仓）拒绝的超卖——预期读数，独立分桶。
            CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis { .. }) => {
                *self
                    .defense_units_exceed_current_holding_count
                    .entry(sl)
                    .or_insert(0) += 1;
            }
            // ★#383：阶段三硬门（桶现金永不为负）——生产恒 0，非 0 即警报，但归属明确故独立
            // 分桶（不埋进 `other_violation_by_kind` 的混合桶里）。
            CampaignViolation::EarningCashUnsound { .. } => {
                *self.earning_cash_unsound_count.entry(sl).or_insert(0) += 1;
            }
            // ★#383：等金额腿买不起一股——预期经济场景，独立分桶。
            CampaignViolation::EarningSizingRoundsToZero { .. } => {
                *self
                    .earning_sizing_rounds_to_zero_count
                    .entry(sl)
                    .or_insert(0) += 1;
            }
            other => {
                self.other_violation_count += 1;
                let label = match other {
                    CampaignViolation::NoActiveCampaign => "no_active_campaign_unreachable",
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::CashUnsound { free, .. },
                    )) if free < 0 => {
                        // ★#380 项一：free<0 现由 `short_diff_cash_gate` 放行入账（亏损往返），
                        // 本路径**已不可达**——旧 `resource_exhausted_free_negative_count` 桶随之
                        // 退役；若仍到达即为通道被改坏的真正警报，落其余违规桶而非资源桶。
                        "cash_unsound_free_negative_unreachable"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::CashUnsound { .. },
                    )) => {
                        // holding<0/free<0 已在上方分支处理；三量皆非负时 cash_sound_gate 本不该
                        // 拒绝——若触达此分支属真正异常，照实归类而非静默吞入某个资源桶。
                        "resource_exhausted_cash_sound_neither_negative_unreachable"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                        TransitionError::Oq9Illegal { .. },
                    )) => {
                        // OQ-9 在本路径恒真（ShortDiff/Realize 的 is_legal_from 恒真）——若触达
                        // 说明合法性表被破坏，是真正的接线/记账错误，不得归入资源耗尽桶。
                        "resource_exhausted_oq9_illegal_unexpected"
                    }
                    CampaignViolation::SizingRoundsToZero { .. } => "sizing_rounds_to_zero",
                    CampaignViolation::ShortDiff(ShortDiffViolation::NonPositiveUnits(_)) => {
                        "short_diff_non_positive_units"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::OverReplenish { .. }) => {
                        "short_diff_over_replenish"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnclosedRoundTrip {
                        ..
                    }) => "short_diff_unclosed_round_trip",
                    CampaignViolation::ShortDiff(ShortDiffViolation::AvgCostMismatch {
                        ..
                    }) => "short_diff_avg_cost_mismatch",
                    // 下两条已在上方分支各自分桶（#380 项二/项四），此处只为穷尽性。
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis {
                        ..
                    }) => "short_diff_units_exceed_cost_basis_unreachable",
                    CampaignViolation::ReplenishWhileFull => "replenish_while_full_unreachable",
                    // ★#414：已在上方分支分桶，此处只为穷尽性。
                    CampaignViolation::ReplenishForeignCenter { .. } => {
                        "replenish_foreign_center_unreachable"
                    }
                    // 下两条已在上方分支各自分桶（#383），此处只为穷尽性。
                    CampaignViolation::EarningCashUnsound { .. } => {
                        "earning_cash_unsound_unreachable"
                    }
                    CampaignViolation::EarningSizingRoundsToZero { .. } => {
                        "earning_sizing_rounds_to_zero_unreachable"
                    }
                    // ★#366：核销越界＝归属账与桶标量失同步（两者逐笔同步推进，生产恒不
                    // 触达）——真正的记账错误警报，不得归入任何「预期读数」桶。
                    CampaignViolation::ShortDiff(ShortDiffViolation::WriteOffExceedsOpen {
                        ..
                    }) => "short_diff_write_off_exceeds_open",
                    CampaignViolation::StageTransition(_) => "stage_transition",
                    // ★#880：重内单向穿零警报（结构性不可达，见 [`CampaignViolation::SideConflict`]）。
                    CampaignViolation::SideConflict { .. } => "chong_side_conflict",
                };
                *self.other_violation_by_kind.entry((sl, label)).or_insert(0) += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★#880：测试重键——`(标的="T", 操作级别)`。键 = (标的, 操作级别)（ADR 0013 裁定二），
    /// 成本状态不进键。
    fn k(op_level: u8) -> ChongKey {
        ChongKey {
            symbol: "T".to_string(),
            op_level,
        }
    }

    /// 来源中枢身份（#380 项四）：`start_index` 区分不同中枢实例，其余字段固定。
    fn cid(start_index: usize) -> CenterId {
        CenterId::of(&crate::theta_v0::types::Center {
            zd: 100,
            zg: 200,
            dd: 98,
            gg: 202,
            start_index,
            end_index: start_index + 50,
        })
    }

    // ── CampaignWiringWitness（issue #357 验收③：产物级见证读数） ──────────

    #[test]
    fn witness_tallies_trigger_drop_rate_by_error_kind() {
        use super::super::center_oscillation_trade::TriggerError;
        let mut w = CampaignWiringWitness::new();
        w.record_trigger_result::<()>(&Ok(()));
        w.record_trigger_result::<()>(&Err(TriggerError::CenterNotAlive));
        w.record_trigger_result::<()>(&Err(TriggerError::CenterNotAlive));
        w.record_trigger_result::<()>(&Err(TriggerError::FlatSignal));
        w.record_trigger_result::<()>(&Err(TriggerError::PriceOutsideZone));
        assert_eq!(w.trigger_attempts, 5, "丢弃率分母=全部尝试次数");
        assert_eq!(
            w.dropped_center_not_alive, 2,
            "CenterNotAlive 单独分桶（丢弃率分子）"
        );
        assert_eq!(
            w.dropped_other_trigger, 2,
            "FlatSignal/PriceOutsideZone 合桶（非 CenterNotAlive 关注点）"
        );
    }

    #[test]
    fn reset_nonempty_termination_outcome_trips_live_regression_gate() {
        use super::super::center_oscillation_trade::SuspensionOutcome;

        let reset = CenterLifecycleEvent::Reset {
            level: 0,
            died_center: None,
            died_chain_index: None,
            trigger_source_index: 500,
            trigger_side: Side::Long,
        };
        let ev_out = SuspensionEventOutcome {
            terminations: vec![SuspensionOutcome {
                center: cid(0),
                source: SuspensionTerminationSource::RebaseVanished,
                cover_action: None,
                side: VoiceSide::Long,
                settlement: TerminationSettlement::WriteOffUnclosed,
            }],
            continuations: Vec::new(),
        };
        let mut w = CampaignWiringWitness::new();

        w.record_reset_termination_outcome(&reset, &ev_out);

        assert_eq!(
            w.reset_settlement_count(),
            1,
            "合成非空终结输出必须击中活门，不能退化成类型上永远为 0 的死读数"
        );
    }

    #[test]
    fn witness_tallies_action_by_level_and_suspension_source_and_lifecycle() {
        let mut w = CampaignWiringWitness::new();
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(1, VoiceSide::Long, CenterOscillationAction::Replenish);
        assert_eq!(
            w.action_by_level.get(&(0, "long", "reduce")),
            Some(&2),
            "归属正确：level 0 两次 Reduce 分桶计数"
        );
        assert_eq!(
            w.action_by_level.get(&(1, "long", "replenish")),
            Some(&1),
            "level 1 Replenish 独立分桶，不与 level 0 混计"
        );

        // ★#414/#489：`superseded` 与 `reset` 桶均退役；只用三类点与 RebaseVanished 锁分桶。
        // ★#442：键增 level 维——同侧同来源跨级别独立成桶（探针的裁决量）。
        w.record_suspension_source(
            0,
            VoiceSide::Long,
            SuspensionTerminationSource::BrokenByThirdClassBuy,
        );
        w.record_suspension_source(
            0,
            VoiceSide::Long,
            SuspensionTerminationSource::BrokenByThirdClassSell,
        );
        w.record_suspension_source(
            0,
            VoiceSide::Long,
            SuspensionTerminationSource::BrokenByThirdClassSell,
        );
        w.record_suspension_source(
            0,
            VoiceSide::Short,
            SuspensionTerminationSource::BrokenByThirdClassBuy,
        );
        w.record_suspension_source(
            2,
            VoiceSide::Long,
            SuspensionTerminationSource::RebaseVanished,
        );
        assert_eq!(
            w.suspension_by_source
                .get(&(0, "long", "broken_by_third_class_buy")),
            Some(&1)
        );
        assert_eq!(
            w.suspension_by_source
                .get(&(0, "long", "broken_by_third_class_sell")),
            Some(&2),
            "挂起归宿分桶按来源独立累计"
        );
        // ★#381 关票修复：同一来源两侧独立分桶——多空并存时不得相加成 3（同一事件逐侧各产一条
        // `SuspensionOutcome`，混计即计数翻倍且无法归属）。
        assert_eq!(
            w.suspension_by_source
                .get(&(0, "short", "broken_by_third_class_buy")),
            Some(&1),
            "空头侧同来源独立成桶"
        );
        assert_eq!(
            w.suspension_by_source
                .get(&(0, "short", "broken_by_third_class_sell")),
            None,
            "空头侧未发生的来源不出现（不被多头侧读数污染）"
        );
        assert_eq!(
            w.suspension_by_source.get(&(2, "long", "rebase_vanished")),
            Some(&1),
            "高级别工程失踪来源独立成桶"
        );
        assert_eq!(w.reset_settlement_count(), 0, "类型层已不能登记 Reset 清算");
        assert_eq!(
            w.broken_by_level(),
            BTreeMap::from([(0, 4)]),
            "Broken 清算按级别汇总，不混入 RebaseVanished"
        );
        assert_eq!(
            w.cross_side_termination_count(),
            3,
            "跨侧终结 = 多头被三卖 2 + 空头被三买 1"
        );

        // ★#442：延续/清算终局两桶同样按 (级别, 侧) 分桶，跨级别不混计。
        w.record_suspension_continued(0, VoiceSide::Long);
        w.record_suspension_continued(2, VoiceSide::Long);
        w.record_suspension_continued(2, VoiceSide::Long);
        assert_eq!(w.suspension_continued_count.get(&(0, "long")), Some(&1));
        assert_eq!(
            w.suspension_continued_count.get(&(2, "long")),
            Some(&2),
            "延续按级别独立累计"
        );
        w.record_settlement(0, VoiceSide::Long, TerminationSettlement::CoverAndClose);
        w.record_settlement(2, VoiceSide::Long, TerminationSettlement::WriteOffUnclosed);
        assert_eq!(
            w.settlement_by_side.get(&(0, "long", "cover_and_close")),
            Some(&1)
        );
        assert_eq!(
            w.settlement_by_side.get(&(2, "long", "write_off_unclosed")),
            Some(&1),
            "清算终局按级别独立分桶"
        );

        w.record_lifecycle(CampaignLifecycleEvent::Opened {
            key: k(0),
            side: VoiceSide::Long,
            notional_in: 3_000,
        });
        w.record_lifecycle(CampaignLifecycleEvent::Died {
            key: k(0),
            side: VoiceSide::Long,
            settlement: None,
        });
        w.record_lifecycle(CampaignLifecycleEvent::Opened {
            key: k(0),
            side: VoiceSide::Short,
            notional_in: 1_000,
        });
        // ★#381 关票修复：生死两轴分侧——事件已带 `side`，标量累加即跨侧求和。
        assert_eq!(w.lifecycle_opened.get("long"), Some(&1));
        assert_eq!(
            w.lifecycle_opened.get("short"),
            Some(&1),
            "空头侧开局独立计数，不与多头侧相加"
        );
        assert_eq!(w.lifecycle_died.get("long"), Some(&1));
        assert_eq!(
            w.lifecycle_died.get("short"),
            None,
            "空头侧未死 ⟹ 该侧桶不出现（非 0 值混入）"
        );

        assert!(w.no_active_campaign_count.is_empty());
        assert_eq!(
            w.other_violation_count, 0,
            "接线正常路径下其余通道拒绝计数恒 0"
        );
        w.record_violation(VoiceSide::Long, CampaignViolation::NoActiveCampaign);
        assert_eq!(
            w.no_active_campaign_count.get("long"),
            Some(&1),
            "NoActiveCampaign ⟹ 该侧真空仓的预期经济场景，按侧分桶"
        );
        assert_eq!(w.other_violation_count, 0, "不误落入其余违规桶");
        w.record_violation(
            VoiceSide::Long,
            CampaignViolation::SizingRoundsToZero { held: 2 },
        );
        assert_eq!(
            w.other_violation_count, 1,
            "SizingRoundsToZero 是真正的记账异常，落其余桶"
        );
    }

    #[test]
    fn witness_tallies_reset_alive_center_leak_by_level_and_side() {
        let center = crate::theta_v0::types::Center {
            zd: 100,
            zg: 200,
            dd: 98,
            gg: 202,
            start_index: 5,
            end_index: 55,
        };
        let mut w = CampaignWiringWitness::new();
        for event in [
            CenterLifecycleEvent::Reset {
                level: 0,
                died_center: Some(center),
                died_chain_index: Some(0),
                trigger_source_index: 10,
                trigger_side: Side::Long,
            },
            CenterLifecycleEvent::Reset {
                level: 2,
                died_center: Some(center),
                died_chain_index: Some(4),
                trigger_source_index: 20,
                trigger_side: Side::Short,
            },
            CenterLifecycleEvent::Reset {
                level: 0,
                died_center: None,
                died_chain_index: None,
                trigger_source_index: 30,
                trigger_side: Side::Long,
            },
        ] {
            w.record_reset_alive_center_leak(&event);
        }
        assert_eq!(
            w.reset_alive_center_leak_by_level_side,
            BTreeMap::from([((0, "long"), 1), ((2, "short"), 1)]),
            "只统计 Reset 到场时仍有活中枢的级别/侧；场空广播不报警"
        );
    }

    /// ★#381：`unsupported_short_position_count` 桶退役的回归锁——空头侧持仓现有自己的
    /// campaign（★#880：按重分账、侧为开局属性、不进键），`NoActiveCampaign` 在两侧同义
    /// （该侧真空仓的预期场景），一律落 `no_active_campaign_count`，不再有第二个「有仓但认不出」
    /// 的分流桶。
    /// （原 #357 关票条件 C 用例 `witness_splits_no_active_campaign_by_short_side_holding`
    /// 随桶一同退役——它断言的分流行为已不存在。）
    #[test]
    fn no_active_campaign_is_single_bucket_after_short_campaign_support() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, CampaignViolation::NoActiveCampaign);
        w.record_violation(VoiceSide::Short, CampaignViolation::NoActiveCampaign);
        assert_eq!(
            w.no_active_campaign_count.get("long"),
            Some(&1),
            "两侧同义、但按侧分列"
        );
        assert_eq!(
            w.no_active_campaign_count.get("short"),
            Some(&1),
            "空头侧不再落已退役的未支持桶"
        );
        assert_eq!(w.other_violation_count, 0, "非记账错误，不落警报桶");
    }

    /// ★issue #357 关票条件 B：`ChannelRejected(CashUnsound)` 拆子桶——不再丢弃
    /// `TransitionError` 内层错误，归因可直接从读数判定。★#380 项一续：`free<0` 子桶退役
    /// （该路径改为放行入账，见本用例尾部）。
    #[test]
    fn witness_buckets_holding_negative_and_flags_retired_free_negative_path() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(
            VoiceSide::Long,
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound {
                    free: 10,
                    holding: -5,
                    withdrawn: 0,
                },
            )),
        );
        assert_eq!(
            w.resource_exhausted_holding_negative_count.get("long"),
            Some(&1),
            "holding<0 ⟹ 预算耗尽子桶（按侧）"
        );
        assert_eq!(
            w.other_violation_count, 0,
            "holding<0 落资源耗尽子桶，不误落其余违规桶"
        );

        // ★#380 项一：free<0 路径经 `short_diff_cash_gate` 放行入账后**已不可达**——旧
        // `resource_exhausted_free_negative_count` 桶随之退役；若仍到达即为通道被改坏的真正
        // 警报，落其余违规桶（而非静默吞入某个资源桶）。
        w.record_violation(
            VoiceSide::Long,
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound {
                    free: -3,
                    holding: 100,
                    withdrawn: 0,
                },
            )),
        );
        assert_eq!(
            w.other_violation_count, 1,
            "free<0 拒绝现已不可达 ⟹ 到达即警报"
        );
        assert_eq!(
            w.other_violation_by_kind
                .get(&("long", "cash_unsound_free_negative_unreachable")),
            Some(&1),
            "退役桶的残余路径按不可达标签归类，不复用旧资源桶名"
        );
    }

    /// witness：死亡吞挂起的计数+单位量分侧落桶，与 #366/#472 结构终结核销桶**分列**
    /// （两条清算路径的读数不得混计——归宿不同：一条是结构终结，一条是 campaign 死）。
    #[test]
    fn witness_records_death_write_off_count_and_units_separately_from_third_class_bucket() {
        let mut w = CampaignWiringWitness::new();
        w.record_lifecycle(CampaignLifecycleEvent::Died {
            key: k(0),
            side: VoiceSide::Long,
            settlement: Some(DeathWriteOff {
                units_gap: 200,
                cash_booked: 2_400,
                bucket_rejected: false,
                centers: 2,
            }),
        });
        w.record_lifecycle(CampaignLifecycleEvent::Died {
            key: k(1),
            side: VoiceSide::Long,
            settlement: None,
        });
        assert_eq!(
            w.lifecycle_died.get("long"),
            Some(&2),
            "两次死亡（生死桶不受本票影响）"
        );
        assert_eq!(
            w.death_write_off_count.get("long"),
            Some(&1),
            "只有带挂起的那次记核销"
        );
        assert_eq!(w.death_write_off_units_gap.get("long"), Some(&200));
        assert_eq!(w.death_write_off_cash_booked.get("long"), Some(&2_400));
        assert!(
            w.unclosed_write_off_count.is_empty(),
            "死亡核销不落结构终结核销桶（两路径分列）"
        );
    }

    /// ★#719（L19①）：桶拒绝（`bucket_rejected=true`，结构性不可达 ⟹ true 即记账错误）
    /// 落 `other_violation_count` + `other_violation_by_kind` 警报桶——对齐
    /// `cash_unsound_free_negative_unreachable` 先例（计数+分桶双落）；正常核销不受影响。
    #[test]
    fn death_write_off_bucket_rejected_lands_in_other_violation_alert_bucket() {
        let mut w = CampaignWiringWitness::new();
        w.record_lifecycle(CampaignLifecycleEvent::Died {
            key: k(0),
            side: VoiceSide::Long,
            settlement: Some(DeathWriteOff {
                units_gap: 100,
                cash_booked: 1_200,
                bucket_rejected: true,
                centers: 1,
            }),
        });
        assert_eq!(w.other_violation_count, 1, "桶拒绝 = 记账错误警报计数");
        assert_eq!(
            w.other_violation_by_kind
                .get(&("long", "death_write_off_bucket_rejected_unreachable")),
            Some(&1),
            "桶拒绝落 other_violation_by_kind 专桶"
        );
        // 核销三桶仍照记（读数不吞——警报与读数分列）。
        assert_eq!(w.death_write_off_count.get("long"), Some(&1));
        assert_eq!(w.death_write_off_units_gap.get("long"), Some(&100));
    }

    /// ★#381：witness 的动作分桶与阶段事件明细均带持仓侧——两侧同级同动作不再混桶。
    #[test]
    fn witness_action_and_stage_records_carry_side() {
        let mut w = CampaignWiringWitness::new();
        w.record_action(0, VoiceSide::Long, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Short, CenterOscillationAction::Reduce);
        w.record_action(0, VoiceSide::Short, CenterOscillationAction::Reduce);
        assert_eq!(w.action_by_level.get(&(0, "long", "reduce")), Some(&1));
        assert_eq!(
            w.action_by_level.get(&(0, "short", "reduce")),
            Some(&2),
            "同级同动作按侧分列"
        );

        w.record_stage_event(0, VoiceSide::Short, 12, 7, TwEvent::RecoverCapital(1_000));
        assert_eq!(
            w.stage_events,
            vec![StageEventRecord {
                level: 0,
                side: "short",
                bar: 12,
                bars_since_open: 7,
                kind: "recover_capital",
                amount: 1_000,
            }],
            "阶段事件明细带侧"
        );
    }

    /// ★#381（评审补齐）：#380 四项观测面对两侧**分侧可归属**——多空并存时同一批读数不再混计。
    /// SPEC #386 §2「witness 呈现归属与冲抵顺序」与 §3「§2 全部口径对两侧对称适用」的产物级锁。
    #[test]
    fn witness_380_observations_are_attributable_per_side() {
        let mut w = CampaignWiringWitness::new();
        // 项一（亏损入账）/ 项二（防线超卖）/ 项四（货满 + 冲抵归属）各喂两侧。
        w.record_loss_accounted(VoiceSide::Long);
        w.record_loss_accounted(VoiceSide::Short);
        w.record_loss_accounted(VoiceSide::Short);
        assert_eq!(w.loss_round_trip_accounted_count.get("long"), Some(&1));
        assert_eq!(
            w.loss_round_trip_accounted_count.get("short"),
            Some(&2),
            "两侧不相加（同名不同义）"
        );

        w.record_violation(
            VoiceSide::Short,
            CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis {
                held: 5,
                attempted: 9,
            }),
        );
        assert_eq!(
            w.defense_units_exceed_current_holding_count.get("short"),
            Some(&1)
        );
        assert_eq!(
            w.defense_units_exceed_current_holding_count.get("long"),
            None,
            "多头侧不被误增"
        );

        w.record_violation(VoiceSide::Short, CampaignViolation::ReplenishWhileFull);
        assert_eq!(w.replenish_triggered_but_full_count.get("short"), Some(&1));

        let cover = vec![
            CoverAssignment {
                center: cid(1),
                units: 10,
                seq: 0,
                same_center: true,
            },
            CoverAssignment {
                center: cid(2),
                units: 10,
                seq: 1,
                same_center: false,
            },
        ];
        w.record_cover(VoiceSide::Short, &cover);
        assert_eq!(
            w.cover_by_side.get(&("short", "same_center")),
            Some(&1),
            "冲抵归属带侧"
        );
        assert_eq!(w.cover_by_side.get(&("short", "other_center")), Some(&1));
        assert_eq!(
            w.cover_by_side.get(&("long", "same_center")),
            None,
            "多头侧冲抵桶不被空头污染"
        );

        assert_eq!(w.other_violation_count, 0, "以上全为预期读数，非警报");
    }

    /// ★witness 报告层三项：等金额回补次数 / 累计净增股数 / 切换时点（生效 bar = 事件 bar+1）。
    #[test]
    fn witness_reports_earning_counts_gained_units_and_switch_bar() {
        let mut w = CampaignWiringWitness::new();
        w.record_stage_event(0, VoiceSide::Long, 9, 9, TwEvent::EnterEarning);
        assert_eq!(w.stage_enter_earning_count, 1);
        assert_eq!(
            w.earning_mode_switch_bar.get(&(0, "long")),
            Some(&10),
            "★切换时点读数=事件 bar+1（次 bar 换模式）"
        );

        w.record_earning_replenish(VoiceSide::Long, 50);
        w.record_earning_replenish(VoiceSide::Long, 71);
        w.record_earning_replenish(VoiceSide::Short, 12);
        assert_eq!(
            w.earning_replenish_count.get("long"),
            Some(&2),
            "等金额回补次数（分侧）"
        );
        assert_eq!(
            w.earning_units_gained.get("long"),
            Some(&121),
            "累计净增股数=50+71"
        );
        assert_eq!(
            w.earning_units_gained.get("short"),
            Some(&12),
            "两侧分列，不得相加"
        );
    }

    /// ★#689：attempted/success/failed 三桶各自独立分级别计数，互不覆盖——修
    /// `historical_bound_by_level`「只认成功分支」的失明（两轮审计曾因此误判「通道零命中」）。
    #[test]
    fn historical_bound_attempted_success_failed_buckets_are_independent_by_level() {
        let mut w = CampaignWiringWitness::new();
        w.record_historical_bound_attempted(0);
        w.record_historical_bound_attempted(0);
        w.record_historical_bound_attempted(1);
        w.record_historical_bound(0);
        w.record_historical_bound_failed(0);
        w.record_historical_bound_failed(1);

        assert_eq!(w.historical_bound_attempted_by_level.get(&0), Some(&2));
        assert_eq!(w.historical_bound_attempted_by_level.get(&1), Some(&1));
        assert_eq!(
            w.historical_bound_by_level.get(&0),
            Some(&1),
            "级别 0 的一次尝试成功、一次失败——成功桶只认那一次"
        );
        assert_eq!(
            w.historical_bound_by_level.get(&1),
            None,
            "级别 1 唯一一次尝试即失败，成功桶空"
        );
        assert_eq!(w.historical_bound_failed_by_level.get(&0), Some(&1));
        assert_eq!(w.historical_bound_failed_by_level.get(&1), Some(&1));
        // 级别 0：attempted(2) = success(1) + failed(1)。级别 1：attempted(1) = success(0) + failed(1)。
    }
}
