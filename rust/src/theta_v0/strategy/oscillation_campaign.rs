//! 每仓 campaign（SPEC #287 T4，issue #294）：把 #293/#354 的 `ShortDiffAccount` + TW 桥升格为
//! **每仓各一 campaign** 的完整生命周期——开仓生（`notional_in`=本仓成本基）→ 短差冲减成本
//! （sizing=当时持仓 1/3，#348 裁定，可复检）→ 到 0 转移（`RecoverCapital`，复用
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
//! ## 粒度：每仓各一（SPEC #287 决议2，issue #294 验收①②）
//!
//! 「仓」=本级别的核心持仓（与 [`center_oscillation_trade::CenterOscillationBook`] 同粒度，
//! 每级别一台）——不按中枢身份分（同一级别可能历经多个中枢的生灭，仓位延续），亦不按窗口
//! 聚合（窗口级汇总只是派生读数，见 [`CampaignBook`] 文档）。生命周期：
//! - **开仓生**：本仓成本基快照从「空仓」（`units()==0`）变为「有持仓」（`units()>0`）——
//!   campaign 开局注资 `notional_in = holding = cost_basis()`（缠师口径「成本入账」）。
//! - **全平死**：本仓成本基快照回到「空仓」——`TwEvent::ClearCampaign`（挂起随死：此时短差
//!   盈亏桶若仍有挂起在途量，不强求先收口，承诺随仓位一起作废——持仓已不存在，往返无从谈起）。
//!
//! ## 到 0 转移（issue #294 验收③，#124 裁定4：单一来源）
//!
//! `RecoverCapital`/`EnterEarning` 的派发**不重新实现判据**——委托
//! [`closed_loop::transition::stage_progression`]（生产 I_Θ 组合层与本模块共用同一算子，不得
//! 镜像重写）。κ=0 基线（[`RiskPolicy::baseline`]，M7 冻结口径，PDF §10 canonical 最小规范）。
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

use super::super::closed_loop::state::RiskMode;
use super::super::closed_loop::transition::{cash_sound_gate, stage_progression, TransitionError};
use super::center_oscillation_trade::{CenterOscillationAction, SuspensionTerminationSource, TriggerError};
use super::ledger::{tw_step, LedgerComp, RiskPolicy, TwEvent, TwState};
use super::short_diff_bucket::{CoreCostBasisSnapshot, ShortDiffAccount, ShortDiffViolation};
use std::collections::BTreeMap;

/// 本模块 typed 拒绝（无静默兜底，与 [`ShortDiffViolation`]/[`TransitionError`] 同规格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignViolation {
    /// 短差盈亏桶层拒绝（[`ShortDiffAccount::record_and_apply_dual`] 透传）。
    ShortDiff(ShortDiffViolation),
    /// 阶段推进事件（`RecoverCapital`/`EnterEarning`）被 closed_loop 通道拒绝——生产路径
    /// （`stage_progression` 只在 barrier 过关/free 足额时派事件）恒不触发，此分支覆盖外部
    /// 注入/边界态防御性核验。
    StageTransition(TransitionError),
    /// 本级尚无开局 campaign（[`CampaignBook::sync_position`] 未见过 units>0 的快照）——
    /// 对空仓级别调用动作是接线错误，不静默创建幽灵 campaign。
    NoActiveCampaign,
    /// sizing 现算派生（当时持仓 1/3，整数除法）落到 0 或以下——当场拒绝，不静默钳制为 1
    /// （持仓过小时「1/3」本就无意义，照实拒绝优于编造最小单位）。
    SizingRoundsToZero { held: i64 },
}

impl From<ShortDiffViolation> for CampaignViolation {
    fn from(v: ShortDiffViolation) -> Self {
        CampaignViolation::ShortDiff(v)
    }
}

/// 一次动作应用的产出：本次真实成交 units + 应用后 TW 态 + 若本次触发阶段推进则携带该事件
/// （`RecoverCapital`=到 0 转移证据，`EnterEarning`=进增股数证据；`None`=本次未推进阶段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignOutcome {
    pub units: i64,
    pub tw: TwState,
    pub ledger: LedgerComp,
    pub stage_event: Option<TwEvent>,
}

/// campaign 生命周期事件（[`CampaignBook::sync_position`] 产出，观测/witness 用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignLifecycleEvent {
    /// 开仓生：本仓成本基快照从空仓变为有持仓。
    Opened { level: u32, notional_in: i64 },
    /// 全平死：本仓成本基快照回到空仓——`suspended_units_forfeited`=死亡时短差盈亏桶尚挂起
    /// 的在途量（挂起随死，非 0 时是「未及收口即随仓终结」的照实记录，非违规）。
    Died { level: u32, suspended_units_forfeited: i64 },
}

/// 单仓 campaign（[`TwState`] + [`LedgerComp`] 双账本并置 + [`ShortDiffAccount`] 短差桥）。
///
/// 不可变模式（coding-style）：[`apply_action`](Self::apply_action) 返回新实例，不就地修改；
/// 生死进出由 [`CampaignBook`] 持有/移除整份实例承载（campaign 边界=实例边界）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OscillationCampaign {
    tw: TwState,
    ledger: LedgerComp,
    short_diff: ShortDiffAccount,
}

impl OscillationCampaign {
    /// 开仓生（[`CampaignBook::sync_position`] 唯一构造点）：`notional_in = holding =
    /// snapshot.cost_basis()`（缠师口径「成本入账」——本仓已投入的成本即本 campaign 退本金目标）。
    /// `free=0`（尚未任何短差冲减，现金积累从 0 起步，「现金积累=成本」口径的起点）。
    fn open(snapshot: CoreCostBasisSnapshot) -> Self {
        Self {
            tw: TwState {
                holding: snapshot.cost_basis(),
                notional_in: snapshot.cost_basis(),
                ..TwState::initial()
            },
            ledger: LedgerComp::initial(snapshot.cost_basis()),
            short_diff: ShortDiffAccount::new(snapshot),
        }
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

    /// sizing=当时持仓 1/3（issue #348 用户裁定「阶段一降成本 sizing=固定 1/3」，整数除法，
    /// ★可复检——原型口径零发明，如有原文/数据依据可升级）。
    fn reduce_units(&self) -> i64 {
        self.short_diff.cost_basis().units() / 3
    }

    /// 阶段一恒仓约束硬（#348 决议「分阶段分域」）：`Replenish` 全额买回当前挂起在途量
    /// （同股数进出——不留部分敞口，简化为「每次触发即整轮收口」）。
    fn replenish_units(&self) -> i64 {
        self.short_diff.bucket().open_units()
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
    ) -> Result<(Self, CampaignOutcome), CampaignViolation> {
        let units = match action {
            CenterOscillationAction::Reduce => self.reduce_units(),
            CenterOscillationAction::Replenish => self.replenish_units(),
        };
        if units <= 0 {
            return Err(CampaignViolation::SizingRoundsToZero {
                held: self.short_diff.cost_basis().units(),
            });
        }
        let mut short_diff = self.short_diff;
        let (tw_after_action, ledger_after_action) =
            short_diff.record_and_apply_dual(&self.tw, &self.ledger, action, units, price)?;

        // ★到 0 转移（issue #294 验收③）：委托单一来源 stage_progression（#124 裁定4）——
        // 不镜像重写 barrier 判据。κ=0 基线（M7 冻结口径）。
        let policy = RiskPolicy::baseline();
        let (tw_final, stage_event) = match stage_progression(&policy, &tw_after_action, risk_mode) {
            Some(ev) => {
                if !ev.is_legal_from(&tw_after_action) {
                    return Err(CampaignViolation::StageTransition(TransitionError::Oq9Illegal {
                        event: ev,
                        stage: tw_after_action.stage,
                    }));
                }
                let advanced = tw_step(&tw_after_action, ev);
                let advanced =
                    cash_sound_gate(advanced).map_err(CampaignViolation::StageTransition)?;
                (advanced, Some(ev))
            }
            None => (tw_after_action, None),
        };

        let next = Self { tw: tw_final, ledger: ledger_after_action, short_diff };
        let outcome = CampaignOutcome { units, tw: tw_final, ledger: ledger_after_action, stage_event };
        Ok((next, outcome))
    }

    /// 全平死：`TwEvent::ClearCampaign`（挂起随死——不核验 `assert_conserved`，全平即终结，
    /// 未收口的挂起在途量随之作废，见 [`CampaignLifecycleEvent::Died`]）。
    ///
    /// `ClearCampaign` 在本模块恒合法：本模块只用 `ShortDiff`/`Realize`（[`TwEvent::is_legal_from`]
    /// 对 `ClearCampaign` 的唯一门槛 `open_legacy_legs==0` 恒满足——本模块从不构造
    /// `OpenShareLeg`/`CloseShareLeg`）。
    fn close(self) -> (TwState, i64) {
        debug_assert!(
            TwEvent::ClearCampaign.is_legal_from(&self.tw),
            "本模块从不用 legacy 腿构造子，ClearCampaign 前置 open_legacy_legs==0 恒满足"
        );
        (tw_step(&self.tw, TwEvent::ClearCampaign), self.short_diff.bucket().open_units())
    }
}

/// 每仓 campaign 账簿（issue #294 验收①②）：BTreeMap 按 level 键——与
/// [`center_oscillation_trade::CenterOscillationBook`]（每级别一台）同粒度、同确定性迭代序理由
/// （#292 H1 先例：`BTreeMap` 非 `HashMap`，避免哈希序跨进程/跨版本不确定）。
///
/// 窗口级汇总（如「本窗口总共几个 campaign 到过 II」）**只是派生读数**——从 `campaigns` 现算
/// （遍历/过滤/计数），本结构不额外维护累计计数器（避免双份状态漂移，同 `state.rs` phase 派生
/// 写回的精神）。
#[derive(Debug, Default)]
pub struct CampaignBook {
    campaigns: BTreeMap<u32, OscillationCampaign>,
}

impl CampaignBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn campaign(&self, level: u32) -> Option<&OscillationCampaign> {
        self.campaigns.get(&level)
    }

    /// 当前存活 campaign 数（诊断/派生读数——非累计维护，见结构体文档）。
    pub fn active_count(&self) -> usize {
        self.campaigns.len()
    }

    /// 同步本级持仓快照 ⟹ 生死判据（High-2 裁定：本方法与 [`OscillationCampaign::reduce_units`]
    /// 共用同一 [`CoreCostBasisSnapshot`] 接口形状，零派生副本）：
    /// - 空仓→有持仓：开仓生（[`CampaignLifecycleEvent::Opened`]）。
    /// - 有持仓→空仓：全平死（[`CampaignLifecycleEvent::Died`]，挂起随死）。
    /// - 其余（仍空/仍持仓）：no-op（`None`）——本模块不处理加仓重算，唯二合法写口在别处
    ///   （#197 G3 既有机制，同 [`short_diff_bucket`] 既有范围边界声明）。
    pub fn sync_position(
        &mut self,
        level: u32,
        snapshot: CoreCostBasisSnapshot,
    ) -> Option<CampaignLifecycleEvent> {
        let currently_open = self.campaigns.contains_key(&level);
        match (currently_open, snapshot.units() > 0) {
            (false, true) => {
                self.campaigns.insert(level, OscillationCampaign::open(snapshot));
                Some(CampaignLifecycleEvent::Opened { level, notional_in: snapshot.cost_basis() })
            }
            (true, false) => {
                let campaign = self.campaigns.remove(&level).expect("contains_key 刚核验为 true");
                let (_final_tw, suspended_units_forfeited) = campaign.close();
                Some(CampaignLifecycleEvent::Died { level, suspended_units_forfeited })
            }
            _ => None,
        }
    }

    /// 单次动作应用（透传 [`OscillationCampaign::apply_action`]）——本级须已开局
    /// （[`CampaignViolation::NoActiveCampaign`]：空仓级别调用是接线错误，不静默创建幽灵 campaign）。
    pub fn apply_action(
        &mut self,
        level: u32,
        action: CenterOscillationAction,
        price: i64,
        risk_mode: RiskMode,
    ) -> Result<CampaignOutcome, CampaignViolation> {
        let campaign = self.campaigns.get(&level).ok_or(CampaignViolation::NoActiveCampaign)?;
        let (next, outcome) = campaign.apply_action(action, price, risk_mode)?;
        self.campaigns.insert(level, next);
        Ok(outcome)
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
    pub dropped_other_trigger: usize,
    /// 减/补动作按 (级别, 动作) 分桶计数——issue #357 验收「归属正确」的产物级见证。
    pub action_by_level: BTreeMap<(u32, &'static str), usize>,
    /// 挂起终结来源分桶（挂起归宿：`BrokenByThirdClassBuy`/`.._Sell`/`Reset`/`Superseded`/
    /// `RebaseVanished` 五源，见 [`super::center_oscillation_trade::SuspensionTerminationSource`]）。
    pub suspension_by_source: BTreeMap<&'static str, usize>,
    /// campaign 开仓生事件计数（[`CampaignLifecycleEvent::Opened`]）。
    pub lifecycle_opened: usize,
    /// campaign 全平死事件计数（[`CampaignLifecycleEvent::Died`]）。
    pub lifecycle_died: usize,
    /// `apply_action` 遇 [`CampaignViolation::NoActiveCampaign`] 的计数——★非接线错误：
    /// `CenterOscillationBook`（#292 T2）的减/补决策独立于本级 Core 持仓状态（「无门」设计，
    /// 见该模块文档），故结构信号在本级空仓时触发是**真实经济场景**（结构说做、但当前无仓可
    /// 操作），非 bug。真实生产数据（wf8 实测）此项非零属预期读数，报告层照实呈现即可，
    /// 不得断言恒 0（`other_violation_count` 才是真正的接线错误警报）。
    ///
    /// ★issue #357 关票条件 C：本仓取数改分侧口径（只读多头侧余额，方案2）后，「本级持有
    /// 空头仓位」不再落本桶——那属于「本接线未支持空头 campaign」，单独计入
    /// [`Self::unsupported_short_position_count`]，本桶专指「本级多头侧真空仓」这一预期场景。
    pub no_active_campaign_count: usize,
    /// `apply_action` 遇 [`CampaignViolation::NoActiveCampaign`]、且**本级空头侧持仓非零**的
    /// 计数（issue #357 关票条件 C，方案2）——★本接线只支持多头侧 campaign（`drive_campaign_wiring`
    /// 只读 `balance_side(Core{level}, Long)`），本级若持有空头仓位（FollowParent×Short 顺父
    /// 级联核心仓），多头侧读数恒为空仓，结构信号触发时必落 `NoActiveCampaign`——但这不是
    /// 「预期经济场景」（真空仓、结构说做当前无仓可操作），而是「未支持」（真有仓、只是本接线
    /// 认不出这一侧）。与 [`Self::no_active_campaign_count`] 分列，不得混桶——前者是接线覆盖
    /// 缺口的产物级见证，需与真正的空仓场景分开呈现，供后续票（空头 campaign 支持）依据。
    pub unsupported_short_position_count: usize,
    /// `apply_action` 遇 [`CampaignViolation::ShortDiff`]`(`[`ShortDiffViolation::ChannelRejected`]`(`
    /// [`TransitionError::CashUnsound`]`{ holding<0, .. }`」的计数——★同样非接线错误，而是
    /// **预算耗尽的真实经济场景**：`OscillationCampaign::reduce_units`/`replenish_units`
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
    /// （[`Self::resource_exhausted_free_negative_count`]，回补价高于卖出价的亏损往返）。
    /// 拆桶后归因才可从读数上直接判定，不再靠推断。
    pub resource_exhausted_holding_negative_count: usize,
    /// `apply_action` 遇 `ChannelRejected(CashUnsound{ free<0, .. })` 的计数（issue #357 关票
    /// 条件 B 新增子桶）——★**亏损往返**：`Reduce` 使 `free += units·p_sell`，`Replenish` 使
    /// `free -= units·p_buy`，`p_buy > p_sell`（买回价高于卖出价，高抛低吸反着做）即 `free`
    /// 转负、被 `cash_sound_gate` 拒绝。非 0 时意味着「亏钱的短差往返在本账本里根本记不进去」
    /// （`record_and_apply_dual` 整笔回滚）——若本读数非零，报告层须撤销「预期经济场景」定性，
    /// 如实呈现为待另立票的设计缺口（见 [`super`] 模块级 issue #357 报告，非本字段文档断言）。
    pub resource_exhausted_free_negative_count: usize,
    /// `apply_action` 遇其余 [`CampaignViolation`]（`AvgCostMismatch`/`NonPositiveUnits`/
    /// `OverReplenish`/`UnclosedRoundTrip`/`UnitsExceedCostBasis`/`StageTransition`/
    /// `SizingRoundsToZero`）的计数——诊断用，生产路径正常接线下恒 0（非 0 = 真正的接线/
    /// 记账逻辑错误，与 `no_active_campaign_count`/`resource_exhausted_count` 的「预期读数」
    /// 性质不同，不得混桶）。
    pub other_violation_count: usize,
    /// `other_violation_count` 的细分诊断分桶（变体名标签，如 `sizing_rounds_to_zero`/
    /// `short_diff_units_exceed_cost_basis` 等）——定位非 0 读数的具体成因用。
    pub other_violation_by_kind: BTreeMap<&'static str, usize>,
}

impl CampaignWiringWitness {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一次触发构造尝试的结果（`Ok`/`Err` 分布，丢弃率分母+分子）。
    pub fn record_trigger_result<T>(&mut self, result: &Result<T, TriggerError>) {
        self.trigger_attempts += 1;
        match result {
            Ok(_) => {}
            Err(TriggerError::CenterNotAlive) => self.dropped_center_not_alive += 1,
            Err(TriggerError::FlatSignal) | Err(TriggerError::PriceOutsideZone) => {
                self.dropped_other_trigger += 1
            }
        }
    }

    /// 记一次挂起终结来源（挂起归宿分桶）。
    pub fn record_suspension_source(&mut self, source: SuspensionTerminationSource) {
        let label = match source {
            SuspensionTerminationSource::BrokenByThirdClassBuy => "broken_by_third_class_buy",
            SuspensionTerminationSource::BrokenByThirdClassSell => "broken_by_third_class_sell",
            SuspensionTerminationSource::Reset => "reset",
            SuspensionTerminationSource::Superseded => "superseded",
            SuspensionTerminationSource::RebaseVanished => "rebase_vanished",
        };
        *self.suspension_by_source.entry(label).or_insert(0) += 1;
    }

    /// 记一次动作按 (级别, 动作) 分桶（归属正确性见证）。
    pub fn record_action(&mut self, level: u32, action: CenterOscillationAction) {
        let label = match action {
            CenterOscillationAction::Reduce => "reduce",
            CenterOscillationAction::Replenish => "replenish",
        };
        *self.action_by_level.entry((level, label)).or_insert(0) += 1;
    }

    /// 记一次 campaign 生死事件。
    pub fn record_lifecycle(&mut self, event: CampaignLifecycleEvent) {
        match event {
            CampaignLifecycleEvent::Opened { .. } => self.lifecycle_opened += 1,
            CampaignLifecycleEvent::Died { .. } => self.lifecycle_died += 1,
        }
    }

    /// 记一次 `apply_action` 拒绝，遇 `NoActiveCampaign` 时额外核验本级空头侧持仓
    /// （issue #357 关票条件 C，方案2）：空头侧非零 ⟹ 落
    /// [`Self::unsupported_short_position_count`]（未支持，非预期）；空头侧为零 ⟹ 落
    /// [`Self::no_active_campaign_count`]（真空仓，预期场景）。`is_short_side_held` 由调用方
    /// （`drive_campaign_wiring`）在拒绝发生的同一 level 上现读 `balance_side(Core{level},
    /// Short)` 传入——本方法不持账本，保持纯判据（同 [`super::account::reason_of_reverse_close`]
    /// 的 `core_residual` 传入模式）。
    ///
    /// 其余变体按既定分桶（`ShortDiff(ChannelRejected(CashUnsound))` 拆 holding/free 两子桶，
    /// issue #357 关票条件 B；其余是真正的接线/记账错误警报，见字段文档，不得混桶）。
    pub fn record_violation(&mut self, violation: CampaignViolation, is_short_side_held: bool) {
        match violation {
            CampaignViolation::NoActiveCampaign if is_short_side_held => {
                self.unsupported_short_position_count += 1;
            }
            CampaignViolation::NoActiveCampaign => {
                self.no_active_campaign_count += 1;
            }
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { holding, .. },
            )) if holding < 0 => {
                self.resource_exhausted_holding_negative_count += 1;
            }
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { free, .. },
            )) if free < 0 => {
                self.resource_exhausted_free_negative_count += 1;
            }
            other => {
                self.other_violation_count += 1;
                let label = match other {
                    CampaignViolation::NoActiveCampaign => "no_active_campaign_unreachable",
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
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnclosedRoundTrip { .. }) => {
                        "short_diff_unclosed_round_trip"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::AvgCostMismatch { .. }) => {
                        "short_diff_avg_cost_mismatch"
                    }
                    CampaignViolation::ShortDiff(ShortDiffViolation::UnitsExceedCostBasis { .. }) => {
                        "short_diff_units_exceed_cost_basis"
                    }
                    CampaignViolation::StageTransition(_) => "stage_transition",
                };
                *self.other_violation_by_kind.entry(label).or_insert(0) += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ledger::TStage;
    use super::super::short_diff_bucket::CoreCostBasisSnapshot;

    fn snapshot(units: i64, cost_basis: i64) -> CoreCostBasisSnapshot {
        CoreCostBasisSnapshot::new(units, cost_basis)
    }

    // ── 生命周期：开仓生 / 全平死 ────────────────────────────────────────

    #[test]
    fn sync_position_opens_campaign_on_transition_from_flat_to_held() {
        let mut book = CampaignBook::new();
        let outcome = book.sync_position(0, snapshot(300, 3_000));
        assert_eq!(outcome, Some(CampaignLifecycleEvent::Opened { level: 0, notional_in: 3_000 }));
        let campaign = book.campaign(0).expect("开仓后应存在 campaign");
        assert_eq!(campaign.tw().notional_in, 3_000, "notional_in=本仓成本基（缠师口径成本入账）");
        assert_eq!(campaign.tw().holding, 3_000, "holding=本仓成本基（与 CoreCostBasisSnapshot 同步）");
        assert_eq!(campaign.tw().stage, TStage::CostReduction, "开局即降成本阶段");
    }

    #[test]
    fn sync_position_is_noop_while_still_flat_or_still_held() {
        let mut book = CampaignBook::new();
        assert_eq!(book.sync_position(0, snapshot(0, 0)), None, "仍空仓 ⟹ no-op");
        book.sync_position(0, snapshot(300, 3_000));
        assert_eq!(book.sync_position(0, snapshot(300, 3_000)), None, "仍持仓 ⟹ no-op（加仓重算不在本模块范围）");
        assert_eq!(book.active_count(), 1);
    }

    #[test]
    fn sync_position_kills_campaign_on_transition_to_flat() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000));
        let outcome = book.sync_position(0, snapshot(0, 0));
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died { level: 0, suspended_units_forfeited: 0 }),
            "无挂起在途量的全平死亡"
        );
        assert!(book.campaign(0).is_none(), "全平后 campaign 实例被移除");
        assert_eq!(book.active_count(), 0);
    }

    /// ★挂起随死（issue #294 验收②「全平=campaign 终结含挂起随死」）：全平死亡时若短差盈亏桶
    /// 仍有挂起在途量（未及收口的半轮往返），死亡照实记录该量，不强求先 assert_conserved。
    #[test]
    fn sync_position_death_forfeits_open_suspended_units_without_requiring_conservation() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000)); // avg_cost=10
        book.apply_action(0, CenterOscillationAction::Reduce, 12, RiskMode::Normal).unwrap(); // 卖 100（1/3）
        assert_eq!(book.campaign(0).unwrap().short_diff().bucket().open_units(), 100, "挂起在途量=100（半轮往返）");
        let outcome = book.sync_position(0, snapshot(0, 0)); // 仓位全平（未先回补）
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died { level: 0, suspended_units_forfeited: 100 }),
            "挂起随死：全平死亡照实记录未收口的挂起量，非违规"
        );
        assert!(book.campaign(0).is_none());
    }

    #[test]
    fn each_level_has_independent_campaign_no_cross_book_offset() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000));
        book.sync_position(1, snapshot(500, 20_000));
        assert_eq!(book.active_count(), 2);
        book.sync_position(0, snapshot(0, 0));
        assert_eq!(book.active_count(), 1, "0 级死亡不影响 1 级 campaign（禁跨仓冲减，每级一本账强制）");
        assert!(book.campaign(1).is_some());
    }

    // ── sizing=当时持仓 1/3（issue #348） ───────────────────────────────

    #[test]
    fn reduce_sizes_to_one_third_of_currently_held_units() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000)); // avg_cost=10
        let outcome = book.apply_action(0, CenterOscillationAction::Reduce, 12, RiskMode::Normal).unwrap();
        assert_eq!(outcome.units, 100, "sizing=当时持仓(300)/3=100（#348 裁定，整数除法）");
    }

    #[test]
    fn replenish_sizes_to_full_open_units_hard_conservation_in_cost_reduction() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000)); // avg_cost=10
        book.apply_action(0, CenterOscillationAction::Reduce, 12, RiskMode::Normal).unwrap(); // 卖 100
        let outcome = book.apply_action(0, CenterOscillationAction::Replenish, 9, RiskMode::Normal).unwrap();
        assert_eq!(outcome.units, 100, "阶段一恒仓约束硬：Replenish 全额买回挂起在途量（同股数进出）");
        assert!(
            book.campaign(0).unwrap().short_diff().assert_conserved().is_ok(),
            "整轮收口 ⟹ 恒仓断言过"
        );
    }

    #[test]
    fn sizing_rounding_to_zero_is_explicit_violation_not_silent_minimum() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(2, 20)); // 持仓仅 2 ⟹ 2/3=0
        assert_eq!(
            book.apply_action(0, CenterOscillationAction::Reduce, 15, RiskMode::Normal),
            Err(CampaignViolation::SizingRoundsToZero { held: 2 }),
            "1/3 取整到 0 时显式拒绝，不静默钳制为 1"
        );
    }

    #[test]
    fn action_on_level_without_open_campaign_is_explicit_wiring_error() {
        let mut book = CampaignBook::new();
        assert_eq!(
            book.apply_action(0, CenterOscillationAction::Reduce, 10, RiskMode::Normal),
            Err(CampaignViolation::NoActiveCampaign)
        );
    }

    // ── 本仓成本基不动（修7）+ P2-D 双账入口对齐 ──────────────────────────

    #[test]
    fn cost_basis_untouched_and_ledger_pi_mirrors_tw_realize() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000)); // avg_cost=10
        let start_tw = book.campaign(0).unwrap().tw(); // 开局 tw()=holding(3000)+free(0)+withdrawn(0)=3000
        let original_cost_basis = book.campaign(0).unwrap().short_diff().cost_basis();

        let reduce = book.apply_action(0, CenterOscillationAction::Reduce, 12, RiskMode::Normal).unwrap();
        assert_eq!(reduce.ledger.pi, 100 * (12 - 10), "R 账本 Π 镜像本轮 Realize=units·(price−avg_cost)=200");
        assert!(reduce.ledger.inv_holds(), "R 账本恒等 R=Π-A-W 全程成立");

        let replenish = book.apply_action(0, CenterOscillationAction::Replenish, 9, RiskMode::Normal).unwrap();
        assert_eq!(
            replenish.ledger.pi,
            100 * (12 - 10) + 100 * (10 - 9),
            "R 账本 Π 累计两笔 Realize（P2-D：TW/R 双账入口对齐，同一 Realize 分量镜像）"
        );
        assert_eq!(
            replenish.tw.tw() - start_tw.tw(),
            replenish.ledger.pi,
            "campaign 开局以来 TW 净漂移=ΣRealize=R 账本 Π（两账本各自入账同一已实现值，非同构但同源）"
        );

        assert_eq!(
            book.campaign(0).unwrap().short_diff().cost_basis(),
            original_cost_basis,
            "本仓成本基（均价口径）全程不动（修7）"
        );
    }

    // ── 首见证：到 0 转移（issue #294 验收④，阶段机第一次真正到达 II） ────

    /// ★首见证核心用例：构造能到 0 的场景（多轮高抛低吸，模拟震荡+趋势窗内反复高抛低吸积累
    /// 现金）——断言 `RecoverCapital` 派发（阶段机到达 `CapitalRecovered`=II）、恒仓守恒逐笔、
    /// 本仓成本基不动、campaign 全平即死亡。
    ///
    /// 场景：本仓成本基 3_000（300 股，avg_cost=10）。每轮高抛低吸赚取「卖价−买价」的净现金，
    /// 需要 free 累计到 ≥ notional_in(3_000) 才能触发足额退本金（`stage_progression` 的
    /// cash-tight 门：`free≥recover_target`，见 `closed_loop::transition` 文档）。每轮 sizing=
    /// 100（300/3），净赚 4/股（卖12买8）⟹ 每轮 400，需 8 轮（3200≥3000）达门槛。
    #[test]
    fn first_witness_campaign_reaches_recover_capital_stage_ii() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000));
        assert_eq!(book.campaign(0).unwrap().tw().stage, TStage::CostReduction, "起点：阶段机在 I（降成本）");

        let mut recovered_at_round = None;
        for round in 1..=8 {
            let sell = book.apply_action(0, CenterOscillationAction::Reduce, 12, RiskMode::Normal).unwrap();
            assert_eq!(sell.units, 100, "每轮 sizing 恒=当时持仓(300)/3=100（本仓成本基不动，见下）");
            // 每轮卖出后立即核验恒仓状态：本轮挂起=100（尚未回补）。
            assert_eq!(book.campaign(0).unwrap().short_diff().bucket().open_units(), 100);

            let cover = book.apply_action(0, CenterOscillationAction::Replenish, 8, RiskMode::Normal).unwrap();
            assert_eq!(cover.units, 100, "整轮收口：买回等量 100（阶段一恒仓约束硬）");
            assert!(
                book.campaign(0).unwrap().short_diff().assert_conserved().is_ok(),
                "第 {round} 轮往返闭合 ⟹ 恒仓断言逐笔过"
            );

            if let Some(TwEvent::RecoverCapital(_)) = cover.stage_event {
                recovered_at_round = Some(round);
                break;
            }
        }

        let round = recovered_at_round.expect("★首见证：8 轮内必须派发 RecoverCapital（阶段机到达 II）");
        assert_eq!(round, 8, "净赚 4/股 x100 units x8 轮=3200≥notional_in(3000) ⟹ 第 8 轮足额退本金");

        let campaign = book.campaign(0).expect("退本金不终结 campaign（仓位仍在，只是阶段推进）");
        assert_eq!(campaign.tw().stage, TStage::CapitalRecovered, "★阶段机第一次真正到达 II");
        assert_eq!(campaign.tw().withdrawn, 3_000, "退本金额=notional_in（足额一次性退回）");
        assert_eq!(campaign.tw().l_wc(), 0, "在险本金归零（缠师「成本为 0」的现金口径落地）");
        assert_eq!(
            campaign.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基（均价口径）全程不动——「成本为 0」是现金口径,非均价口径（修7）"
        );

        // 全平即 campaign 死亡（联动 #274 出口规则：仓位归零时终结，不因阶段推进而提前终结）。
        let death = book.sync_position(0, snapshot(0, 0));
        assert_eq!(death, Some(CampaignLifecycleEvent::Died { level: 0, suspended_units_forfeited: 0 }));
        assert!(book.campaign(0).is_none(), "全平即 campaign 死亡");
    }

    /// 趋势窗对照：卖出与买回同价（11，无价差可赚）——每轮 `Reduce` 的 `Realize=units·(price−
    /// avg_cost)=100·(11−10)=100` 与 `Replenish` 的 `Realize=units·(avg_cost−price)=100·(10−11)
    /// =−100` 恰好相消，本轮净 Realize=0——现金积累不到 notional_in ⟹ 阶段机停留在 I（不到 0，
    /// 诚实退化，非 bug——见 `closed_loop::transition`「free 不足 ⟹ 等待」注记）。
    #[test]
    fn trend_window_without_enough_realized_profit_stays_at_stage_one() {
        let mut book = CampaignBook::new();
        book.sync_position(0, snapshot(300, 3_000)); // avg_cost=10
        for _ in 0..8 {
            book.apply_action(0, CenterOscillationAction::Reduce, 11, RiskMode::Normal).unwrap();
            let cover = book.apply_action(0, CenterOscillationAction::Replenish, 11, RiskMode::Normal).unwrap();
            assert_eq!(cover.stage_event, None, "同价往返净 Realize=0 ⟹ free 不积累 ⟹ 不推进");
        }
        assert_eq!(book.campaign(0).unwrap().tw().stage, TStage::CostReduction, "趋势单边同价窗：诚实停留在 I，非硬凑到 0");
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
        assert_eq!(w.dropped_center_not_alive, 2, "CenterNotAlive 单独分桶（丢弃率分子）");
        assert_eq!(w.dropped_other_trigger, 2, "FlatSignal/PriceOutsideZone 合桶（非 CenterNotAlive 关注点）");
    }

    #[test]
    fn witness_tallies_action_by_level_and_suspension_source_and_lifecycle() {
        let mut w = CampaignWiringWitness::new();
        w.record_action(0, CenterOscillationAction::Reduce);
        w.record_action(0, CenterOscillationAction::Reduce);
        w.record_action(1, CenterOscillationAction::Replenish);
        assert_eq!(w.action_by_level.get(&(0, "reduce")), Some(&2), "归属正确：level 0 两次 Reduce 分桶计数");
        assert_eq!(w.action_by_level.get(&(1, "replenish")), Some(&1), "level 1 Replenish 独立分桶，不与 level 0 混计");

        w.record_suspension_source(SuspensionTerminationSource::BrokenByThirdClassBuy);
        w.record_suspension_source(SuspensionTerminationSource::Superseded);
        w.record_suspension_source(SuspensionTerminationSource::Superseded);
        assert_eq!(w.suspension_by_source.get("broken_by_third_class_buy"), Some(&1));
        assert_eq!(w.suspension_by_source.get("superseded"), Some(&2), "挂起归宿分桶按来源独立累计");

        w.record_lifecycle(CampaignLifecycleEvent::Opened { level: 0, notional_in: 3_000 });
        w.record_lifecycle(CampaignLifecycleEvent::Died { level: 0, suspended_units_forfeited: 0 });
        assert_eq!(w.lifecycle_opened, 1);
        assert_eq!(w.lifecycle_died, 1);

        assert_eq!(w.no_active_campaign_count, 0);
        assert_eq!(w.other_violation_count, 0, "接线正常路径下其余通道拒绝计数恒 0");
        w.record_violation(CampaignViolation::NoActiveCampaign, false);
        assert_eq!(w.no_active_campaign_count, 1, "NoActiveCampaign 且本级空头侧无仓 ⟹ 预期经济场景，单独分桶");
        assert_eq!(w.unsupported_short_position_count, 0, "空头侧无仓 ⟹ 不落未支持桶");
        assert_eq!(w.other_violation_count, 0, "不误落入其余违规桶");
        w.record_violation(CampaignViolation::SizingRoundsToZero { held: 2 }, false);
        assert_eq!(w.other_violation_count, 1, "SizingRoundsToZero 是真正的记账异常，落其余桶");
    }

    /// ★issue #357 关票条件 C：`NoActiveCampaign` 且本级空头侧持仓非零 ⟹ 落
    /// `unsupported_short_position_count`（未支持），不与真空仓的 `no_active_campaign_count`
    /// 混桶——两者叙事不同（前者「有仓但认不出」，后者「真空仓」）。
    #[test]
    fn witness_splits_no_active_campaign_by_short_side_holding() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(CampaignViolation::NoActiveCampaign, true);
        assert_eq!(w.unsupported_short_position_count, 1, "空头侧持仓非零 ⟹ 落未支持桶");
        assert_eq!(w.no_active_campaign_count, 0, "不误落入真空仓桶");
        w.record_violation(CampaignViolation::NoActiveCampaign, false);
        assert_eq!(w.no_active_campaign_count, 1, "空头侧无仓 ⟹ 落真空仓桶");
        assert_eq!(w.unsupported_short_position_count, 1, "不回填/不误增未支持桶");
    }

    /// ★issue #357 关票条件 B：`ChannelRejected(CashUnsound)` 按 `holding<0`/`free<0` 拆两子桶
    /// ——不再丢弃 `TransitionError` 内层错误，归因可直接从读数判定。
    #[test]
    fn witness_splits_resource_exhausted_by_holding_vs_free_negative() {
        let mut w = CampaignWiringWitness::new();
        w.record_violation(
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { free: 10, holding: -5, withdrawn: 0 },
            )),
            false,
        );
        assert_eq!(w.resource_exhausted_holding_negative_count, 1, "holding<0 ⟹ 预算耗尽子桶");
        assert_eq!(w.resource_exhausted_free_negative_count, 0);

        w.record_violation(
            CampaignViolation::ShortDiff(ShortDiffViolation::ChannelRejected(
                TransitionError::CashUnsound { free: -3, holding: 100, withdrawn: 0 },
            )),
            false,
        );
        assert_eq!(w.resource_exhausted_free_negative_count, 1, "free<0 ⟹ 亏损往返子桶");
        assert_eq!(w.resource_exhausted_holding_negative_count, 1, "另一子桶不被误增");
        assert_eq!(w.other_violation_count, 0, "两条 CashUnsound 均落资源耗尽子桶，不误落其余违规桶");
    }
}
