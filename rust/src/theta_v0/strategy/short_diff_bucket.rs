//! 短差盈亏桶 + TW 桥（SPEC #287 T3，issue #293）。
//!
//! ADR 0001 修正案一/补充二/补充三裁定落地：短差往返
//! （[`center_oscillation_trade::CenterOscillationAction`] 的 Reduce/Replenish，#292 T2）
//! **不改**本仓成本基（均价口径全程不动；唯二合法出口=全平清零/加仓重算，属
//! [`super::account::ParallelAccountLedger::fill`] 的既有职责，本模块不触碰）。短差往返
//! 盈亏单独记该级账内「短差盈亏」标签桶——桶是**报告层读数 + TW 输入的单一数据源**：
//! 同一笔调用产出的全部 TwEvent 金额之和累计进桶（报告层），构造出的 [`TwEvent`] 序列同时
//! 喂入 TW（TW 输入），禁止两处各自重算（禁双写，口径见下节 P1 修复）。
//!
//! ## P1 修复（issue #293，影子评审判不过的守恒隐性破坏）
//!
//! 旧版把 `Reduce` 的全额卖出款（`units·price`）整笔塞进单一 `TwEvent::ShortDiff`（本应只是
//! 成本基划转 free⇄holding）——短差盈亏（`price−avg_cost`）既不进利润账也不使 TW 漂移，被
//! `holding` 静默吸收：守恒测试绿但盈亏不可见，「现金积累=成本」模型账面破产。
//!
//! 修复：[`ShortDiffAccount::record_action`] 新增 `avg_cost`（持仓均价）显式入参，`Reduce`
//! 拆两笔 [`ShortDiffEvents`]：
//! - `ShortDiff(units·avg_cost)`——成本基划转（free⇄holding，恒不漂移 TW）；
//! - `Realize(units·(price−avg_cost))`——已实现短差盈亏入账（唯一 TW 漂移构造子，语义见
//!   [`TwEvent::Realize`]）。
//!
//! ## P1 修复续（买回侧对偶拆分，issue #293 复审浮出）
//!
//! 首版只拆了卖侧（`Reduce`），`Replenish` 仍整笔塞单一 `ShortDiff(-units·price)`——同一病灶
//! 换了个方向复发：均价 10、卖 10 股 @12、买回 10 股 @9，旧 `Replenish` 记
//! `ShortDiff(-90)`，成本基（`holding` 摘要）划回 90 而非原均价份额 100，买便宜省下的 10
//! 被 `holding` 静默吸收（本仓成本基被悄悄下修 3000→2990，违反修 7 本仓成本基不动裁定）。
//!
//! 对偶修复：`Replenish` 同样拆两笔：
//! - `ShortDiff(-units·avg_cost)`——按**原均价**划回成本基（free→holding，恒不漂移 TW，逐位
//!   镜像 `Reduce` 那笔的量纲，往返后 `ShortDiff` 两笔互相抵消，`holding` 净 0）；
//! - `Realize(units·(avg_cost−price))`——买回侧已实现盈亏（买便宜为正=赚，买贵为负=亏，照实
//!   入账，唯一 TW 漂移构造子）。
//!
//! 全往返对账（卖 + 买回同一均价 `avg_cost`）：
//! `Σ Realize = units·(price_sell−avg_cost) + units·(avg_cost−price_buy)
//! = units·(price_sell−price_buy)` —— 与 `avg_cost` 无关，恰等于真实现金差价（高抛低吸的
//! 净现金流），TW 全程漂移 = 该值；`ShortDiff` 两笔相消 ⟹ `holding` 回到往返前原值（本仓
//! 成本基份额不动，修 7 裁定的行为化断言，见测试
//! `full_round_trip_holding_returns_to_original_and_tw_drift_equals_sum_of_realize`）。
//!
//! [`ShortDiffBucket::realized_cash`] 保留为报告层读数——语义不变（Σ 净现金：`Reduce` 记
//! `+units·price`，`Replenish` 记 `-units·price`），但不再是单一 TwEvent 的同源值：新禁双写
//! 口径是**桶净现金增量 = 本次调用产出全部 TwEvent 金额之和**（`Reduce`/`Replenish` 均是
//! `ShortDiff`+`Realize` 两份相加＝`±units·price`，符号随方向）。
//!
//! 守恒断言口径同步改判：往返**不再断言 TW 不变**（旧版全额款塞单一 `ShortDiff` 制造的表面
//! 守恒正是本 bug 的症状）——`ShortDiff` 恒守恒、`Realize` 恒漂移，二者语义不同不该合并断言，
//! 新断言是 **TW 漂移 = Σ Realize**（`ledger.rs::tw_step_realize_drift_equals_dpi` 既有定理
//! 在本模块的桥接核验，见测试 `tw_drift_equals_sum_of_realize_through_short_diff_round_trip`）。
//! 恒仓断言（[`ShortDiffAccount::assert_conserved`]，Σ|units| 守恒）语义不受影响。
//!
//! ## 恒仓断言（用户裁定：严格，违规显式失败）
//!
//! 短差往返须「同股数进出」（Σ|units| 守恒）：`Reduce` 卖出的 units 记入挂起在途量，
//! `Replenish` 买回不得超过在途量——买回多于卖出（超额回补）由 [`ShortDiffAccount::record_action`]
//! 当场拒绝（[`ShortDiffViolation::OverReplenish`]）；「N:1」型不守恒（卖 N 只补 1，往返
//! 未闭合）不是当场非法（部分回补是合法中间态），由边界核验
//! [`ShortDiffAccount::assert_conserved`] 在往返理应收口处（campaign 清算/中枢终结等）
//! 显式核验——挂起在途量非零即 `Err`，不静默视为已收口。两条路径都是 typed `Result`
//! （无静默钳制/吸收/继续记账），是「违规显式失败」的落地。
//!
//! ## P2-E 合并（issue #354，吸纳 #352/#353，旧 TW 桥退役——谱系注记）
//!
//! 终审（#293 复审）指出两处独立漏洞：**P1-A**（`Reduce`/`Replenish` 的 `avg_cost` 由调用方各传
//! 一次，两次不一致即静默错账）与 **P1-B**（[`ShortDiffEvents::apply`] 旧版直调 raw
//! [`tw_step`]，绕过 [`super::super::closed_loop::transition::transition_adapter`] 出口唯一的
//! CashUnsound 关卡——负 free 可静默落盘）。P2-E 终审裁定：`closed_loop/transition.rs` 已有
//! 同构的「ShortDiff(成本基)+Realize(成交额−成本基) 拆分 + OQ-9 门 + CashUnsound 门」实现——
//! 一套实现、一处真相，本模块**合并**接入而非另起一套等效修法：
//!
//! - **P1-A 结构性消解**：`avg_cost` 不再是 [`ShortDiffAccount::record_action`] 的调用方入参，
//!   改为每次调用从 `self.cost_basis`（只读、无 setter）现算派生（同构
//!   `closed_loop::transition::schedule_adapter` 的 `avg_cost = holding/positions` 现算，见该
//!   文件 `filled_delta<0` 分支）——两腿不一致在类型层不可构造（不是运行时检查后的承诺）。
//!   **附加守卫**：`ShortDiffBucket` 记录挂起批次开仓时派生的 `open_avg_cost`；若外部把
//!   `bucket`（`pub` 字段，`Copy`）整体移植到另一个 `cost_basis` 不同的 `ShortDiffAccount`
//!   （唯一仍可能引入不一致的路径——本模块内部调用永不触发），批次内再次派生的 `avg_cost`
//!   与批次记录值不一致即 typed `Err`（[`ShortDiffViolation::AvgCostMismatch`]）。
//! - **P1-B 旧桥退役**：[`ShortDiffEvents::apply`] 不再直调 raw `tw_step` 后即返回——改为
//!   OQ-9 legal 检查（[`TwEvent::is_legal_from`]，`ShortDiff`/`Realize` 恒真，防御性核验证明
//!   本模块与 closed_loop 走同一张合法性表）+ 应用后经
//!   [`super::super::closed_loop::transition::cash_sound_gate`]（issue #354 从
//!   `transition_adapter` 出口检查原地抽出的同一 chokepoint，逐字节同判据，非另写一份）——
//!   负 free 由此显式失败（[`ShortDiffViolation::ChannelRejected`]），P1-B 关卡白捡。
//!   [`ShortDiffAccount::record_and_apply`] 在通道拒绝时回滚账本状态（同 `NonPositiveUnits`/
//!   `OverReplenish` 既有的「违规显式失败，状态不变」纪律）。
//!
//! ## 范围边界
//!
//! 本模块只做「桶记账 + TW 桥 + 三条断言」——本仓成本基本体（[`CoreCostBasisSnapshot`]）
//! 只读传导，唯二合法写口在别处（#197 G3 既有机制，不在本票范围）；每仓 campaign 粒度属
//! #294（T4）；`CenterOscillationAction` 的**数量**（units）来源（真实 sizing 口径）未在
//! #292/#293 任一契约内定义——本模块把 units 作调用方显式入参（不臆造默认值），生产侧
//! 真正实例化 TW 账本 + 喂入真实 units 属 #294（每仓 campaign 粒度落地时）的自然接线点。
//!
//! ## 评审尾巴四条小修（issue #354，承接 #353 一票）
//!
//! P2-E 合并落地后复审浮出的四处收尾：
//!
//! - **High-1**：[`ShortDiffAccount::record_action`] 开头新增 typed 拒绝——`units` 超过本仓
//!   实际持有份数（`self.cost_basis.units()`）即 `Err`（[`ShortDiffViolation::UnitsExceedCostBasis`]），
//!   与 [`ShortDiffViolation::OverReplenish`] 同规格但是独立的一条防线：`OverReplenish` 卡
//!   「买回超过本轮挂起在途量」（短差账本内部状态），本变体卡「记账超过本仓真实持仓」
//!   （外部成本基状态）。
//! - **High-3**：`ShortDiffAccount` 的 `bucket` 字段改真私有 + 只读访问器
//!   [`ShortDiffAccount::bucket`]（与 `cost_basis` 同规格）——旧版"P2-C 票面范围只列
//!   cost_basis/realized_cash 两项、bucket 保持 pub"的处置名实不符，`pub` 字段的整体赋值本身
//!   就是一种 setter。
//! - **High-4**：[`ShortDiffAccount::record_action`] 降为私有，外部唯一出口收窄到
//!   [`ShortDiffAccount::record_and_apply`]（grep 确认原本即无外部直接调用，本条是把既有事实
//!   钉成类型层保证）。
//! - **#353 附带**：E2E 桥接测试（`center_oscillation_action_from_book_bridges_into_tw_ledger`）
//!   起手态补齐 `tw0.holding` 非负的显式断言，把 P1-B 起手态修正隐含的前提钉成可见断言。

use super::super::closed_loop::transition::{cash_sound_gate, TransitionError};
use super::center_oscillation_trade::CenterOscillationAction;
use super::ledger::{tw_step, TwEvent, TwState};

/// 短差记账的 typed 拒绝（无静默兜底，恒仓断言违规的唯一载体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortDiffViolation {
    /// units 非正（构造非法：卖出/买回数量必须 >0）。
    NonPositiveUnits(i64),
    /// `Replenish` 试图买回超过当前挂起未回补的 units——当场非法，不静默钳制到全部买回。
    OverReplenish { open_units: i64, attempted: i64 },
    /// 边界核验（[`ShortDiffAccount::assert_conserved`]）：往返未闭合，挂起在途量非零
    /// （含「N:1」型不守恒——卖出量与买回量不相等）。
    UnclosedRoundTrip { open_units: i64 },
    /// P2-E 附加守卫（issue #354，承接 #352 P1-A）：当前从 `self.cost_basis` 现算派生的
    /// `avg_cost`（`derived`）与挂起批次开仓时记录的 `avg_cost`（`recorded`）不一致——本模块
    /// 内部调用永不触发（`cost_basis` 全程只读不变），唯一触发路径是把 `bucket` 整体移植到
    /// 另一个 `cost_basis` 不同的账本（High-3 后 `bucket` 已真私有，此路径现只能发生在本模块
    /// 内部——同文件测试对私有字段可见，见 [`ShortDiffAccount`] 文档「High-3 补齐」节）。
    AvgCostMismatch { recorded: i64, derived: i64 },
    /// P2-E 通道切换（issue #354，承接 #353 P1-B）：TW 事件经
    /// [`super::super::closed_loop::transition::cash_sound_gate`] 或 OQ-9 legal 检查被拒绝——
    /// 直接携带 [`TransitionError`]（同一 chokepoint 的同一错误类型，证明本模块的 TW 输出确实
    /// 流经该通道，而非另起一套等效实现）。
    ChannelRejected(TransitionError),
    /// #354 评审尾巴 High-1：`units` 超过本仓实际持有份数（`self.cost_basis.units()`）——
    /// 当场非法，不静默钳制到「只记账实际可用的那部分」。与 [`Self::OverReplenish`] 同规格
    /// 但是独立的一条防线：`OverReplenish` 卡「买回超过本轮挂起在途量」（短差账本内部状态），
    /// 本变体卡「记账超过本仓真实持仓」（外部成本基状态）——两条边界互不覆盖，缺一漏一。
    UnitsExceedCostBasis { held: i64, attempted: i64 },
}

/// 本仓成本基快照（均价口径，[`ShortDiffAccount`] 只读传导，绝不写）。
///
/// 唯二合法写口在别处（[`super::account::ParallelAccountLedger::fill`] 的加仓重算 /
/// 全平清零两分支，#197 G3 既有机制）——本类型不提供任何 mutate 方法，「不动」是
/// **类型层保证**：字段私有（[`Self::new`] 构造、[`Self::units`]/[`Self::cost_basis`] 只读
/// 访问，无 setter），外部无法绕过构造函数直接改字段——不是运行时检查后的承诺（复审指出旧版
/// 字段 `pub` 时此说法名实不符：`pub` 字段的直接赋值本身就是一种 setter；本版把字段收为
/// 私有，令注释与类型定义重新一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoreCostBasisSnapshot {
    units: i64,
    cost_basis: i64,
}

impl CoreCostBasisSnapshot {
    pub fn new(units: i64, cost_basis: i64) -> Self {
        Self { units, cost_basis }
    }

    pub fn units(&self) -> i64 {
        self.units
    }

    pub fn cost_basis(&self) -> i64 {
        self.cost_basis
    }
}

/// 短差盈亏桶（单级账一份，与 [`center_oscillation_trade::CenterOscillationBook`] 同粒度）：
/// 报告层累计读数 + 挂起在途 units（恒仓断言状态）+ 挂起批次均价（P2-E 附加守卫状态）。
///
/// ★P2-C 处置（issue #354，承接 #352 附带项）：`realized_cash` 改真私有（只读访问
/// [`Self::realized_cash`]），与姊妹字段 `open_units` 同规格——旧版 `pub` 字段的整体赋值本身
/// 就是一种 setter，与「报告层只读读数」的语义名实不符（同类修复见 [`CoreCostBasisSnapshot`]
/// 模块文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortDiffBucket {
    /// 报告层读数：短差往返累计净现金（= Σ d_cash，缠师口径「现金积累=成本」的载体）。
    realized_cash: i64,
    /// 挂起在途 units（本轮 `Reduce` 卖出、尚未被 `Replenish` 买回的部分）——恒仓断言状态。
    open_units: i64,
    /// 挂起批次开仓时派生的 `avg_cost`（`None`=当前无挂起批次）——P2-E 附加守卫状态，见
    /// [`ShortDiffViolation::AvgCostMismatch`]。
    open_avg_cost: Option<i64>,
}

impl ShortDiffBucket {
    pub fn new() -> Self {
        Self::default()
    }

    /// 报告层读数：短差往返累计净现金（只读访问，P2-C 处置）。
    pub fn realized_cash(&self) -> i64 {
        self.realized_cash
    }

    /// 当前挂起在途 units（诊断/测试用只读访问）。
    pub fn open_units(&self) -> i64 {
        self.open_units
    }

    /// 当前挂起批次的开仓均价（诊断/测试用只读访问，`None`=无挂起批次）。
    pub fn open_avg_cost(&self) -> Option<i64> {
        self.open_avg_cost
    }

    /// 恒仓断言（边界核验）：往返必须闭合（挂起在途量归零）才算「同股数进出」守恒。
    pub fn assert_conserved(&self) -> Result<(), ShortDiffViolation> {
        if self.open_units != 0 {
            return Err(ShortDiffViolation::UnclosedRoundTrip { open_units: self.open_units });
        }
        Ok(())
    }
}

/// `record_action` 单次产出的 TW 事件序列（P1 修复 #293：`Reduce`/`Replenish` 对偶拆分，均两笔）。
///
/// - `short_diff`：成本基划转（[`TwEvent::ShortDiff`]，恒不漂移 TW，`ledger.rs::tw_step_preserves_tw`
///   覆盖的守恒构造子之一；`Reduce` 为 `+units·avg_cost`，`Replenish` 为 `-units·avg_cost`，
///   往返后两笔相消）。
/// - `realize`：已实现盈亏（[`TwEvent::Realize`]），`Reduce`/`Replenish` 均产出（`Some`）——
///   `Reduce` 记 `units·(price−avg_cost)`，`Replenish` 记 `units·(avg_cost−price)`，二者按
///   原均价对偶，全往返之和恰等于真实现金差价（`units·(price_sell−price_buy)`，与 `avg_cost`
///   无关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffEvents {
    pub short_diff: TwEvent,
    pub realize: Option<TwEvent>,
}

impl ShortDiffEvents {
    /// 按序应用到 TW 账本（先成本基划转，后已实现盈亏入账；`realize=None` 时只应用
    /// `short_diff`），经 closed_loop 唯一通道放行后返回新 [`TwState`]（不可变模式）。
    ///
    /// ★P2-E 旧桥退役（issue #354，承接 #353 P1-B）：旧版直调 raw [`tw_step`] 后即返回，
    /// 绕过 [`super::super::closed_loop::transition::transition_adapter`] 出口唯一的
    /// CashUnsound 关卡（负 free 可静默落盘）。新版逐笔先过 OQ-9 legal 检查
    /// （[`TwEvent::is_legal_from`]——`ShortDiff`/`Realize` 恒真，此处是防御性核验，证明本模块
    /// 与 closed_loop 走同一张合法性表，而非未检查即放行），两笔都应用完后再经
    /// [`super::super::closed_loop::transition::cash_sound_gate`]（issue #354 从
    /// `transition_adapter` 出口检查原地抽出的**同一** chokepoint，逐字节同判据）核验最终态
    /// TW 三量非负——与 `transition_adapter` 的检查时序一致（逐事件 OQ-9 前置检查 + 末尾单次
    /// 现金-sound 检查，非逐事件都做现金-sound 检查）。
    pub fn apply(&self, tw: &TwState) -> Result<TwState, ShortDiffViolation> {
        if !self.short_diff.is_legal_from(tw) {
            return Err(ShortDiffViolation::ChannelRejected(TransitionError::Oq9Illegal {
                event: self.short_diff,
                stage: tw.stage,
            }));
        }
        let tw_after_short_diff = tw_step(tw, self.short_diff);
        let tw_after_realize = match self.realize {
            Some(ev) => {
                if !ev.is_legal_from(&tw_after_short_diff) {
                    return Err(ShortDiffViolation::ChannelRejected(TransitionError::Oq9Illegal {
                        event: ev,
                        stage: tw_after_short_diff.stage,
                    }));
                }
                tw_step(&tw_after_short_diff, ev)
            }
            None => tw_after_short_diff,
        };
        cash_sound_gate(tw_after_realize).map_err(ShortDiffViolation::ChannelRejected)
    }

    /// 本次产出全部 TwEvent 金额之和（新禁双写口径核验用：须等于桶净现金增量，见
    /// [`ShortDiffAccount::record_action`]）。
    fn total_d_cash(&self) -> i64 {
        let TwEvent::ShortDiff(short_diff_amount) = self.short_diff else {
            unreachable!("ShortDiffEvents.short_diff 恒为 TwEvent::ShortDiff（构造闸唯一入口）")
        };
        let realize_amount = match self.realize {
            Some(TwEvent::Realize(d)) => d,
            Some(_) => unreachable!("ShortDiffEvents.realize 恒为 TwEvent::Realize 或 None"),
            None => 0,
        };
        short_diff_amount + realize_amount
    }
}

/// 短差记账体：桶 + 本仓成本基只读传导（#293 T3 对 #292 `CenterOscillationAction` 的
/// 完整消费点）。
///
/// ★P2-C 处置（issue #354，承接 #352 附带项）：`cost_basis` 改真私有（只读访问
/// [`Self::cost_basis`]）——旧版 `pub` 字段允许整体替换（=setter），与「本仓成本基本模块只读
/// 传导、不写」的叙事名实不符。
///
/// ★High-3 补齐（#354 评审尾巴）：`bucket` 原判"保持 `pub`"（P2-C 票面范围只列
/// `cost_basis`/`realized_cash` 两项）名实亦不符——`pub` 字段的整体赋值本身就是一种 setter，
/// 与「桶是报告层只读读数 + 挂起状态」的叙事同样不一致。现改真私有（只读访问
/// [`Self::bucket`]），与 `cost_basis` 同规格。外部整体移植（唯一仍可能引入 `avg_cost`
/// 两腿不一致的路径，见 [`ShortDiffViolation::AvgCostMismatch`] 附加守卫）现只能发生在本模块
/// 内部（同文件测试，私有字段对子模块可见）——附加守卫仍保留，作为该内部路径的防御性核验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffAccount {
    bucket: ShortDiffBucket,
    cost_basis: CoreCostBasisSnapshot,
}

impl ShortDiffAccount {
    pub fn new(cost_basis: CoreCostBasisSnapshot) -> Self {
        Self { bucket: ShortDiffBucket::new(), cost_basis }
    }

    /// 短差盈亏桶（只读访问，High-3 处置）：报告层累计读数 + 挂起在途状态。
    pub fn bucket(&self) -> ShortDiffBucket {
        self.bucket
    }

    /// 本仓成本基快照（只读访问，P2-C 处置）。
    pub fn cost_basis(&self) -> CoreCostBasisSnapshot {
        self.cost_basis
    }

    /// 单源记账：把一次 [`CenterOscillationAction`]（#292 T2 产出）落成 [`ShortDiffEvents`]
    /// ——本次调用产出的全部 TwEvent 金额之和累计进桶（报告层 [`ShortDiffBucket::realized_cash`]），
    /// 禁止两处各自重算（禁双写，新口径见模块文档「P1 修复」节）。
    ///
    /// - `Reduce`（高抛卖出）拆两笔：`ShortDiff(units·avg_cost)`（成本基划转，holding→free）
    ///   + `Realize(units·(price−avg_cost))`（已实现短差盈亏，TW 漂移构造子，可正可负）。
    /// - `Replenish`（回补买入）对偶拆两笔：`ShortDiff(-units·avg_cost)`（成本基划转，
    ///   free→holding，按**原均价**划回而非成交价，往返后与 `Reduce` 那笔相消）+
    ///   `Realize(units·(avg_cost−price))`（买回侧已实现盈亏，买便宜为正=赚，买贵为负=亏，
    ///   TW 漂移构造子，可正可负）。
    ///
    /// `avg_cost`：持仓均价，**不再是调用方入参**（P2-E 结构性消解 P1-A，issue #354 承接
    /// #352）——每次调用从 `self.cost_basis`（只读、无 setter，全程不变）现算派生：
    /// `avg_cost = cost_basis.cost_basis() / cost_basis.units()`（`units()==0` 时取 0），
    /// 同构 `closed_loop::transition::schedule_adapter` 减/平仓分支的 `holding/positions`
    /// 现算（该文件 `filled_delta<0` 分支）。旧版调用方各传一次、两次不一致即静默错账的洞
    /// 在类型层不可再构造（`Reduce`/`Replenish` 读同一个只读字段）。附加守卫见下方
    /// [`ShortDiffViolation::AvgCostMismatch`]。真实生产来源 = 持仓账本接口（[`CoreCostBasisSnapshot`]）；
    /// 本模块只读消费，不推导/不重算——真实实例化属 #294。
    ///
    /// 恒仓断言（严格，违规即显式失败）：`units<=0`、`units` 超过本仓实际持有份数
    /// （[`ShortDiffViolation::UnitsExceedCostBasis`]，#354 评审尾巴 High-1）、回补超过挂起
    /// 在途量、或挂起批次均价不一致均返回 typed `Err`，不静默钳制/吸收；失败时本账本状态
    /// **不变**（桶/在途量均未落笔）。
    ///
    /// ★现在是私有出口（#354 评审尾巴 High-4）：外部唯一入口是
    /// [`record_and_apply`](Self::record_and_apply)——记账与 TW 应用必须原子发生，不留下
    /// 「记账已发生但 TW 未接受」的中间态（见该方法「原子回滚」节）；本方法只在
    /// `record_and_apply` 内部调用（及本模块测试直接调用以核验拆分细节）。
    fn record_action(
        &mut self,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
    ) -> Result<ShortDiffEvents, ShortDiffViolation> {
        if units <= 0 {
            return Err(ShortDiffViolation::NonPositiveUnits(units));
        }
        if units > self.cost_basis.units() {
            return Err(ShortDiffViolation::UnitsExceedCostBasis {
                held: self.cost_basis.units(),
                attempted: units,
            });
        }
        let avg_cost = if self.cost_basis.units() > 0 {
            self.cost_basis.cost_basis() / self.cost_basis.units()
        } else {
            0
        };
        if let Some(recorded) = self.bucket.open_avg_cost {
            if recorded != avg_cost {
                return Err(ShortDiffViolation::AvgCostMismatch { recorded, derived: avg_cost });
            }
        }
        let events = match action {
            CenterOscillationAction::Reduce => {
                self.bucket.open_units += units;
                self.bucket.open_avg_cost = Some(avg_cost);
                ShortDiffEvents {
                    short_diff: TwEvent::ShortDiff(units * avg_cost),
                    realize: Some(TwEvent::Realize(units * (price - avg_cost))),
                }
            }
            CenterOscillationAction::Replenish => {
                let remaining = self.bucket.open_units - units;
                if remaining < 0 {
                    return Err(ShortDiffViolation::OverReplenish {
                        open_units: self.bucket.open_units,
                        attempted: units,
                    });
                }
                self.bucket.open_units = remaining;
                self.bucket.open_avg_cost = if remaining == 0 { None } else { Some(avg_cost) };
                ShortDiffEvents {
                    short_diff: TwEvent::ShortDiff(-(units * avg_cost)),
                    realize: Some(TwEvent::Realize(units * (avg_cost - price))),
                }
            }
        };
        self.bucket.realized_cash += events.total_d_cash();
        Ok(events)
    }

    /// [`record_action`](Self::record_action) + 立即经 [`ShortDiffEvents::apply`] 落到 TW
    /// 账本——TW 桥的端到端接线点（成本基划转 + 已实现盈亏两笔均入账，经 closed_loop 通道
    /// 放行）。
    ///
    /// ★原子回滚（issue #354，同 `NonPositiveUnits`/`OverReplenish` 既有的「违规显式失败，
    /// 状态不变」纪律）：`record_action` 先落笔桶状态（open_units/open_avg_cost/realized_cash），
    /// 若随后 `apply` 被 closed_loop 通道拒绝（[`ShortDiffViolation::ChannelRejected`]），本账本
    /// 回滚到调用前状态——不留下「记账已发生但 TW 未接受」的不一致态。
    pub fn record_and_apply(
        &mut self,
        tw: &TwState,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
    ) -> Result<TwState, ShortDiffViolation> {
        let before = *self;
        let events = self.record_action(action, units, price)?;
        match events.apply(tw) {
            Ok(next_tw) => Ok(next_tw),
            Err(violation) => {
                *self = before;
                Err(violation)
            }
        }
    }

    /// 恒仓断言（边界核验），委托 [`ShortDiffBucket::assert_conserved`]。
    pub fn assert_conserved(&self) -> Result<(), ShortDiffViolation> {
        self.bucket.assert_conserved()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::classifier::center_lifecycle::CenterId;
    use crate::theta_v0::strategy::center_oscillation_trade::{
        CenterOscillationBook, CenterOscillationTrigger,
    };
    use crate::theta_v0::strategy::voice::VoiceSide;
    use crate::theta_v0::types::Center;

    fn snapshot(units: i64, cost_basis: i64) -> CoreCostBasisSnapshot {
        CoreCostBasisSnapshot::new(units, cost_basis)
    }

    /// 派生出恰为 `avg_cost` 的成本基快照——P2-E 后 `avg_cost` 不再是调用方入参，测试改用此
    /// 帮助函数精确控制现算派生结果。
    ///
    /// ★High-1 后 units 改用大基数（10_000，非原 1）：`avg_cost = cost_basis/units` 整除保证
    /// 派生值不变，但本模块新增「`units` 不得超过本仓实际持有份数」检查（见
    /// [`ShortDiffViolation::UnitsExceedCostBasis`]）后，`units=1` 会让本文件几乎所有既有用例
    /// （Reduce/Replenish 实际调用的 units 普遍 >1）当场被拒——大基数只为让「本仓总持仓」远超
    /// 各用例实际卖出/买回的量，不影响 `avg_cost` 派生语义。
    fn avg_cost_snapshot(avg_cost: i64) -> CoreCostBasisSnapshot {
        snapshot(10_000, avg_cost * 10_000)
    }

    // ── 单源记账（禁双写，新口径：桶净现金增量 = 本次产出全部 TwEvent 金额之和） ──

    /// P1 修复核心用例：`Reduce` 拆两笔——`ShortDiff(units·avg_cost)`（成本基划转）+
    /// `Realize(units·(price−avg_cost))`（已实现短差盈亏）；两笔金额之和等于桶净现金增量
    /// （新禁双写口径，替代旧「单一 d_cash 同时喂桶与 TwEvent」表述）。
    #[test]
    fn reduce_splits_into_short_diff_and_realize_bucket_delta_equals_sum() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(9));
        let events = acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        assert_eq!(events.short_diff, TwEvent::ShortDiff(90), "成本基划转=units·avg_cost=10·9=90");
        assert_eq!(
            events.realize,
            Some(TwEvent::Realize(30)),
            "已实现短差盈亏=units·(price−avg_cost)=10·(12−9)=30"
        );
        assert_eq!(acct.bucket.realized_cash(), 120, "桶净现金读数=units·price=10·12=120（报告层口径不变）");
        assert_eq!(events.total_d_cash(), 120, "两笔金额之和=桶净现金增量（新禁双写口径）");

        // ★P2-E 通道切换：apply 经 closed_loop cash_sound_gate，需 holding 足额覆盖成本基划转。
        let tw0 = TwState { holding: 90, ..TwState::initial() };
        let tw1 = events.apply(&tw0).unwrap();
        let tw1_manual = tw_step(&tw_step(&tw0, events.short_diff), events.realize.unwrap());
        assert_eq!(tw1, tw1_manual, "通道放行路径 = raw 两步 tw_step 逐字节相同（通道不改变合法转移的数值）");
    }

    /// P1 续修核心用例（issue #293 复审浮出）：`Replenish` 对偶拆两笔——
    /// `ShortDiff(-units·avg_cost)`（按原均价划回成本基，非成交价）+
    /// `Realize(units·(avg_cost−price))`（买回侧已实现盈亏）。覆盖买价<均价（买便宜=正
    /// Realize）与买价>均价（买贵=负 Realize）两种情形。
    #[test]
    fn replenish_splits_into_short_diff_and_realize_dual_to_reduce() {
        // 买价(7) < 均价(9) ⟹ 买便宜为正=赚。
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(9));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        let cheap = acct.record_action(CenterOscillationAction::Replenish, 10, 7).unwrap();
        assert_eq!(
            cheap.short_diff,
            TwEvent::ShortDiff(-90),
            "成本基划回=-units·avg_cost=-10·9=-90（按原均价，非成交价 7）"
        );
        assert_eq!(
            cheap.realize,
            Some(TwEvent::Realize(20)),
            "已实现盈亏=units·(avg_cost−price)=10·(9−7)=20（买便宜=赚）"
        );

        // 买价(11) > 均价(9) ⟹ 买贵为负=亏。
        let mut acct2 = ShortDiffAccount::new(avg_cost_snapshot(9));
        acct2.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        let expensive = acct2.record_action(CenterOscillationAction::Replenish, 10, 11).unwrap();
        assert_eq!(expensive.short_diff, TwEvent::ShortDiff(-90), "成本基划回同样=-90，与成交价无关");
        assert_eq!(
            expensive.realize,
            Some(TwEvent::Realize(-20)),
            "已实现盈亏=10·(9−11)=-20（买贵=亏，照实入账负数）"
        );
    }

    /// ★P1 回归锁（issue #293）：修复前，`Reduce` 把全额卖出款塞进单一 `ShortDiff`，短差
    /// 盈亏（`price−avg_cost`）不产出任何独立可观测事件——被 `holding` 静默吸收，TW 因此
    /// 不漂移。本测试锁死「盈亏必须产出可观测的 `Realize` 事件且使 TW 漂移」，防止未来
    /// 重新塌缩回旧的单笔全额行为。
    #[test]
    fn p1_reduce_produces_visible_realize_event_not_silently_absorbed() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(9));
        let events = acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        match events.realize {
            Some(TwEvent::Realize(d)) => {
                assert_eq!(d, 30, "短差盈亏必须作为独立 Realize 落账（非 0/None——0/None 是静默吸收的症状）")
            }
            other => panic!("Reduce 必须产出 Realize 事件，实际={other:?}"),
        }
        let tw0 = TwState { holding: 90, ..TwState::initial() };
        let tw1 = events.apply(&tw0).unwrap();
        assert_eq!(
            tw1.tw() - tw0.tw(),
            30,
            "TW 漂移必须恰等于已实现盈亏（旧版单笔 ShortDiff 会让漂移=0，盈亏被 holding 静默吸收）"
        );
    }

    /// ★P1 续修回归锁（issue #293，复审浮出的买回侧同病）：修复前，`Replenish` 把全额买回款
    /// 塞进单一 `ShortDiff(-units·price)`——成本基按**成交价**而非**原均价**划回，买回侧盈亏
    /// （`avg_cost−price`）不产出任何独立可观测事件，被 `holding` 静默吸收，本仓成本基因此被
    /// 悄悄下修（复审实证：均价 10、卖 10@12、买回 10@9，旧版成本基 3000→2990）。本测试锁死
    /// 「买回侧盈亏必须产出可观测的 `Realize` 事件且使 TW 漂移」，防止未来重新塌缩回旧的单笔
    /// 按成交价划回行为。
    #[test]
    fn p1_replenish_produces_visible_realize_event_not_silently_absorbed() {
        let mut acct = ShortDiffAccount::new(snapshot(300, 3_000)); // 均价10=3000/300，300股
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap(); // 卖10@12
        let events = acct.record_action(CenterOscillationAction::Replenish, 10, 9).unwrap(); // 买回10@9
        assert_eq!(
            events.short_diff,
            TwEvent::ShortDiff(-100),
            "成本基按原均价划回=-units·avg_cost=-10·10=-100（非旧版-units·price=-90）"
        );
        match events.realize {
            Some(TwEvent::Realize(d)) => assert_eq!(
                d, 10,
                "买回侧盈亏必须作为独立 Realize 落账=units·(avg_cost−price)=10·(10−9)=10（非 0/None——静默吸收的症状）"
            ),
            other => panic!("Replenish 必须产出 Realize 事件，实际={other:?}"),
        }
        // 单独核验回补侧事件的 TW 效应：free 需覆盖 ShortDiff(-100) 的买回现金支出。
        let tw0 = TwState { free: 100, ..TwState::initial() };
        let tw1 = events.apply(&tw0).unwrap();
        assert_eq!(
            tw1.tw() - tw0.tw(),
            10,
            "TW 漂移必须恰等于买回侧已实现盈亏（旧版单笔 ShortDiff(-90) 会让漂移=0，10 的差价被 \
             holding 静默吸收，本仓成本基随之被悄悄下修）"
        );
    }

    // ── 恒仓断言：守恒过 ──────────────────────────────────────────────────

    /// 完整往返（同股数进出）⟹ 挂起在途量归零，恒仓断言过；桶累计=往返净现金（缠师
    /// 「现金积累=成本」口径，不受 `avg_cost` 拆分影响——报告层公式不变）。
    #[test]
    fn full_round_trip_conserves_units_and_realizes_cash() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(10));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap(); // 卖10@12，均价10
        acct.record_action(CenterOscillationAction::Replenish, 10, 9).unwrap(); // 买回10@9，均价10
        assert_eq!(acct.bucket.open_units(), 0, "同股数进出 ⟹ 挂起在途量归零");
        assert!(acct.assert_conserved().is_ok(), "往返闭合 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash(), 10 * 12 - 10 * 9, "桶累计=往返净现金=30（高抛低吸获利）");
    }

    /// 多轮往返（高抛→回补→再高抛→再回补）逐轮都守恒，累计现金逐轮叠加。
    ///
    /// ★P2-E 结构变化：两轮共用**同一** `avg_cost`（17）——`avg_cost` 现从 `self.cost_basis`
    /// 现算派生，同一账本实例全程只读不变的 `cost_basis` 决定两轮必然同值（旧版两轮各传
    /// 17/19 两个不同 `avg_cost` 的写法在新 API 下已不可构造，正是 P1-A 结构性消解的效果）。
    #[test]
    fn multi_round_trips_each_conserve_independently() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(17));
        acct.record_action(CenterOscillationAction::Reduce, 5, 20).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 5, 18).unwrap();
        assert!(acct.assert_conserved().is_ok());
        acct.record_action(CenterOscillationAction::Reduce, 7, 22).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 7, 19).unwrap();
        assert!(acct.assert_conserved().is_ok());
        assert_eq!(acct.bucket.realized_cash(), (5 * 20 - 5 * 18) + (7 * 22 - 7 * 19));
    }

    // ── 恒仓断言：违规显式失败（用户裁定：严格） ────────────────────────────

    /// units<=0 当场拒绝，不静默吸收为 0/1。
    #[test]
    fn nonpositive_units_is_explicit_violation() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(8));
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, 0, 10),
            Err(ShortDiffViolation::NonPositiveUnits(0))
        );
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, -3, 10),
            Err(ShortDiffViolation::NonPositiveUnits(-3))
        );
        assert_eq!(acct.bucket.realized_cash(), 0, "拒绝的调用不落笔");
    }

    /// ★High-1（issue #354 评审尾巴）：`units` 超过本仓实际持有份数（`self.cost_basis.units()`）
    /// 当场拒绝——不静默钳制到「只记账实际可用的那部分」。与 `OverReplenish` 同规格但是独立的
    /// 一条防线：`OverReplenish` 卡「买回超过挂起在途量」（短差账本内部状态，见
    /// [`over_replenish_is_explicit_violation_and_state_unchanged`]），本用例卡「记账超过本仓
    /// 真实持仓」（外部成本基状态）——两条边界互不覆盖，均须各自拒绝。
    #[test]
    fn units_exceed_cost_basis_is_explicit_violation() {
        let mut acct = ShortDiffAccount::new(snapshot(5, 40)); // 本仓仅持 5 股（均价 8）
        let result = acct.record_action(CenterOscillationAction::Reduce, 10, 12); // 试图卖 10 股
        assert_eq!(
            result,
            Err(ShortDiffViolation::UnitsExceedCostBasis { held: 5, attempted: 10 }),
            "卖出(10)>本仓实际持有(5) ⟹ 显式拒绝"
        );
        assert_eq!(acct.bucket.realized_cash(), 0, "拒绝的调用不落笔");
        assert_eq!(acct.bucket.open_units(), 0, "拒绝的调用不改在途量");
    }

    /// 超额回补（买回多于挂起在途量）当场拒绝——不静默钳制到「只买回挂起的那部分」。
    #[test]
    fn over_replenish_is_explicit_violation_and_state_unchanged() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(8));
        acct.record_action(CenterOscillationAction::Reduce, 5, 10).unwrap(); // 卖5，挂起=5
        let before = acct;
        let result = acct.record_action(CenterOscillationAction::Replenish, 8, 9); // 试图买8
        assert_eq!(
            result,
            Err(ShortDiffViolation::OverReplenish { open_units: 5, attempted: 8 }),
            "买回(8)>挂起(5) ⟹ 显式拒绝"
        );
        assert_eq!(acct, before, "拒绝的调用不改任何状态（桶/在途量均不落笔）");
    }

    /// ★用户裁定用例：「N:1 不守恒即违规」——卖出 N（10）只买回 1，往返未闭合。
    /// 这不是当场非法（部分回补是合法中间态，允许继续往返），但边界核验
    /// `assert_conserved` 必须显式失败，不得静默当作已收口。
    #[test]
    fn n_to_one_mismatch_is_not_immediately_illegal_but_fails_boundary_assertion() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(17));
        acct.record_action(CenterOscillationAction::Reduce, 10, 20).unwrap(); // 卖10
        let partial = acct.record_action(CenterOscillationAction::Replenish, 1, 18); // 只买1
        assert!(partial.is_ok(), "部分回补（1<10）当场合法——往返可以分批收口");
        assert_eq!(acct.bucket.open_units(), 9, "N:1（10:1）⟹ 挂起在途量=9，未闭合");
        assert_eq!(
            acct.assert_conserved(),
            Err(ShortDiffViolation::UnclosedRoundTrip { open_units: 9 }),
            "边界核验：N:1 不守恒 ⟹ 显式失败，不静默视为已收口"
        );
    }

    // ── P2-E 附加守卫：挂起批次均价一致性（issue #354，承接 #352） ──────────

    /// ★P1-A 附加守卫核心用例：`bucket`（High-3 后真私有字段，同文件测试仍可见、`Copy`）被
    /// 整体移植到另一个 `cost_basis` 不同的账本——本模块内部调用永不触发此路径（`cost_basis`
    /// 全程只读不变），但这是 `avg_cost` 结构性派生之外**唯一**仍可能引入两腿不一致的路径
    /// （High-3 后已收窄到本模块内部），附加守卫在此显式拒绝，不静默按新 `cost_basis` 重算
    /// 历史批次。
    #[test]
    fn avg_cost_mismatch_after_bucket_transplant_is_explicit_violation() {
        let mut opened = ShortDiffAccount::new(avg_cost_snapshot(10));
        opened.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap(); // 挂起批次记 avg_cost=10
        let mut mismatched = ShortDiffAccount::new(avg_cost_snapshot(20)); // 不同 cost_basis
        mismatched.bucket = opened.bucket; // 唯一仍可行的不一致引入路径（私有字段，同文件测试可见）
        let result = mismatched.record_action(CenterOscillationAction::Replenish, 10, 9);
        assert_eq!(
            result,
            Err(ShortDiffViolation::AvgCostMismatch { recorded: 10, derived: 20 }),
            "挂起批次记录的 avg_cost(10) 与本账本现算派生的 avg_cost(20) 不一致 ⟹ 显式拒绝"
        );
    }

    // ── High-3（issue #354 评审尾巴）：bucket 只读访问器 ──────────────────

    /// ★High-3：`bucket` 改真私有 + 只读访问器 `bucket()`（与 `cost_basis()` 同规格）——
    /// 访问器读到的桶状态须与记账后直接字段读一致，证明访问器不是摆设、不丢失/篡改读数。
    #[test]
    fn bucket_accessor_reads_current_state() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(9));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        assert_eq!(acct.bucket().open_units(), 10, "访问器读到的挂起在途量与直接字段读一致");
        assert_eq!(acct.bucket().realized_cash(), 120, "访问器读到的累计现金与直接字段读一致");
        assert_eq!(acct.bucket(), acct.bucket, "访问器返回值与私有字段逐位相等（Copy 只读，非另存一份）");
    }

    // ── 本仓成本基不动断言 ───────────────────────────────────────────────

    /// 往返后成本基逐位不变（PartialEq 全字段比对）；报告层成本读数（均价口径）与桶
    /// 累计（短差净现金）分列在同一记账体的两个不相干字段，互不覆写。
    #[test]
    fn cost_basis_untouched_across_round_trips_bucket_listed_separately() {
        let original = snapshot(1_000, 50_000);
        let mut acct = ShortDiffAccount::new(original);
        for _ in 0..3 {
            acct.record_action(CenterOscillationAction::Reduce, 10, 15).unwrap();
            acct.record_action(CenterOscillationAction::Replenish, 10, 11).unwrap();
        }
        assert_eq!(acct.cost_basis(), original, "本仓成本基（均价口径）往返后逐位不变");
        assert_eq!(acct.bucket.realized_cash(), 3 * (10 * 15 - 10 * 11), "桶累计另列，不并入成本基");
        assert!(acct.assert_conserved().is_ok());
    }

    /// 违规调用（超额回补被拒）也不动成本基（成本基字段完全独立于短差记账结果）。
    #[test]
    fn cost_basis_untouched_even_when_short_diff_call_is_rejected() {
        let original = snapshot(200, 8_000);
        let mut acct = ShortDiffAccount::new(original);
        acct.record_action(CenterOscillationAction::Reduce, 5, 10).unwrap();
        let _ = acct.record_action(CenterOscillationAction::Replenish, 99, 9); // 拒绝
        assert_eq!(acct.cost_basis(), original);
    }

    // ── TW 桥：守恒口径改判（P1 修复，issue #293） ──────────────────────────

    /// ★口径改判核心用例：往返**不再断言 TW 不变**——`ShortDiff` 恒守恒（成本基划转），
    /// `Realize` 恒漂移（已实现盈亏入账），二者语义不同不该合并断言；旧版把 Reduce 全额
    /// 款塞单一 `ShortDiff` 制造的表面守恒正是本 bug 的症状。新断言：**TW 漂移 = Σ Realize**
    /// （对齐 `ledger.rs::tw_step_realize_drift_equals_dpi` 既有定理，本测试是桥接层核验）。
    #[test]
    fn tw_drift_equals_sum_of_realize_through_short_diff_round_trip() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(6));
        let tw0 = TwState { free: 1_000, holding: 500, ..TwState::initial() };
        let tw0_total = tw0.tw();

        // Reduce(20@8，均价6) ⟹ ShortDiff(20·6=120)（守恒）+ Realize(20·(8−6)=40)（漂移+40）。
        let events1 = acct.record_action(CenterOscillationAction::Reduce, 20, 8).unwrap();
        let tw1 = events1.apply(&tw0).unwrap();
        assert_eq!(tw1.free, tw0.free + 120 + 40, "free 吸收成本基划转(120)+已实现盈亏(40)");
        assert_eq!(tw1.holding, tw0.holding - 120, "holding 只减成本基份额(120)，非全额卖出款(160)");
        assert_eq!(tw1.tw() - tw0_total, 40, "TW 漂移=本轮 Realize=40（非 0——旧口径的症状）");

        // Replenish(20@6，均价6) ⟹ ShortDiff(-20·6=-120)（守恒）+ Realize(20·(6−6)=0)——买回价
        // 恰等于均价，本轮无盈亏，但仍是显式 Realize(0) 事件（非 None，对偶拆分恒产出两笔）。
        let events2 = acct.record_action(CenterOscillationAction::Replenish, 20, 6).unwrap();
        assert_eq!(events2.realize, Some(TwEvent::Realize(0)), "买价=均价 ⟹ Realize(0)，非 None");
        let tw2 = events2.apply(&tw1).unwrap();
        assert_eq!(tw2.tw(), tw1.tw(), "本轮 Realize=0 ⟹ TW 不再漂移（ShortDiff 恒守恒）");

        assert_eq!(tw2.tw() - tw0_total, 40, "全程 TW 总漂移=Σ Realize=40（唯一一次非零 Realize）");
        assert_eq!(acct.bucket.realized_cash(), 160 - 120, "桶净现金读数=往返净现金=40（报告层口径不变）");
    }

    /// ★全往返对账核心用例（issue #293 复审要求，P1 续修）：卖侧
    /// `Realize(units·(price_sell−avg_cost))` + 买侧 `Realize(units·(avg_cost−price_buy))`
    /// = ΣRealize = **真实现金差价**（化简后与 `avg_cost` 无关，恰等于
    /// `units·(price_sell−price_buy)`）；`ShortDiff` 两笔相消 ⟹ `holding` 净 0，往返后回到
    /// 原值（本仓成本基份额行为化不动，修 7 裁定）；TW 全程漂移 = ΣRealize。
    #[test]
    fn full_round_trip_holding_returns_to_original_and_tw_drift_equals_sum_of_realize() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(6));
        let tw0 = TwState { free: 1_000, holding: 500, ..TwState::initial() };

        // 卖20@8，均价6 ⟹ ShortDiff(120) + Realize(20·(8−6)=40)。
        let sell = acct.record_action(CenterOscillationAction::Reduce, 20, 8).unwrap();
        let tw1 = sell.apply(&tw0).unwrap();
        assert_eq!(tw1.holding, tw0.holding - 120, "holding 减成本基份额=units·avg_cost=120");

        // 买回20@5（买便宜），均价6（同一持仓，成本基不变）⟹ ShortDiff(-120) + Realize(20·(6−5)=20)。
        let buy = acct.record_action(CenterOscillationAction::Replenish, 20, 5).unwrap();
        let tw2 = buy.apply(&tw1).unwrap();
        assert_eq!(tw2.holding, tw0.holding, "ShortDiff 两笔相消 ⟹ holding 回到往返前原值（净 0）");

        let sum_realize = match (sell.realize, buy.realize) {
            (Some(TwEvent::Realize(a)), Some(TwEvent::Realize(b))) => a + b,
            other => panic!("卖/买两侧均须产出 Realize，实际={other:?}"),
        };
        assert_eq!(
            sum_realize, 60,
            "ΣRealize=40+20=60=units·(price_sell−price_buy)=20·(8−5)=60，与 avg_cost 无关"
        );
        assert_eq!(tw2.tw() - tw0.tw(), sum_realize, "TW 全程漂移=ΣRealize=真实现金差价");
        assert_eq!(acct.bucket.realized_cash(), 20 * 8 - 20 * 5, "桶净现金读数=往返净现金=60（报告层口径不变）");
        assert!(acct.assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
    }

    // ── P2-E 通道切换：CashUnsound gate（issue #354，承接 #353 P1-B） ────────

    /// ★P1-B 验收核心用例（issue #353 具名场景实证复现）：free=0 卖 10@12（无盈亏，
    /// avg_cost=12）后急拉回补 10@50（买贵 38/股）——旧版 `ShortDiffEvents::apply` 直调 raw
    /// `tw_step` 会让 free 静默落到 **-380**（issue #353 实证数字）且无 Err/无 panic。新版
    /// 经 [`super::super::closed_loop::transition::cash_sound_gate`] 通道，显式返回
    /// `ChannelRejected(TransitionError::CashUnsound { free: -380, .. })`——回补调用本身
    /// 经 `record_and_apply` 原子回滚，不留下「记账已发生但 TW 未接受」的不一致态。
    #[test]
    fn p1b_extreme_replenish_negative_free_is_explicit_channel_rejection() {
        let mut acct = ShortDiffAccount::new(avg_cost_snapshot(12));
        let tw0 = TwState { holding: 120, ..TwState::initial() };
        let tw1 = acct.record_and_apply(&tw0, CenterOscillationAction::Reduce, 10, 12).unwrap();
        assert_eq!(tw1.free, 120, "卖出按均价成交，无盈亏（Realize=0）");
        assert_eq!(tw1.holding, 0, "holding 全额划出（成本基=120，全部回流 free）");

        let before = acct;
        let result = acct.record_and_apply(&tw1, CenterOscillationAction::Replenish, 10, 50);
        assert_eq!(
            result,
            Err(ShortDiffViolation::ChannelRejected(TransitionError::CashUnsound {
                free: -380,
                holding: 120,
                withdrawn: 0,
            })),
            "急拉回补致 free=-380（issue #353 实证数字）⟹ 经通道显式拒绝，非静默落盘"
        );
        assert_eq!(acct, before, "通道拒绝 ⟹ record_and_apply 原子回滚，账本状态不变（同 OverReplenish 纪律）");
    }

    // ── #292 → #293 端到端桥接：真实 `CenterOscillationAction` 落成 TW 事件 ──

    fn cid(start_index: usize, zd: i64, zg: i64) -> CenterId {
        CenterId::of(&Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index: start_index + 50 })
    }

    /// ★端到端契约衔接证据：#292 `CenterOscillationBook::on_trigger` 产出的真实
    /// `CenterOscillationAction`（非合成/占位值）直接喂入 `ShortDiffAccount::record_and_apply`
    /// ⟹ 正确落成 `ShortDiffEvents`（Reduce/Replenish 均两笔，对偶拆分）经 closed_loop 通道
    /// 放行、推进 TW 账本；往返闭合后恒仓断言过、成本基不动；TW 全程总漂移=ΣRealize=真实
    /// 现金差价（P1 续修口径，非旧「回补侧无 Realize」）。
    ///
    /// ★P1-B 起手态修正（issue #354 承接 #353「E2E 起手态 holding 断言补齐」）：`tw0.holding`
    /// 须覆盖本仓成本基（=20_000=500 股·均价 40），与 `snapshot(500, 20_000)` 一致——旧版
    /// `tw0=TwState::initial()`（holding=0）能通过纯因为旧桥未经 CashUnsound 检查；新通道下
    /// 若仍用 holding=0，Reduce 的 `ShortDiff(15·40=600)` 会让 holding 变负，被通道正确拦截
    /// （这正是 P1-B 要防的类型，此处起手态改为真实反映持仓成本基，而非制造该失败）。附带项
    /// （issue #353）：起手态显式断言 `tw0.holding` 非负（真实持仓不可能为负），把这条隐含前提
    /// 钉成可见断言，防止未来编辑把起手态改回不真实的负值/0 而不被察觉。
    #[test]
    fn center_oscillation_action_from_book_bridges_into_tw_ledger() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let mut acct = ShortDiffAccount::new(snapshot(500, 20_000));
        let tw0 = TwState { holding: 20_000, ..TwState::initial() };
        assert!(tw0.holding >= 0, "起手态 holding 不得为负（真实持仓不可能为负）");
        let avg_cost = 20_000 / 500; // 本仓成本基（均价口径，见 snapshot），= 40，供断言消息复算用。

        // 上沿高抛：次级别卖点，价格=中枢上沿 zg（真实 #292 触发构造，非合成值）。
        let reduce_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let reduce_action = book.on_trigger(reduce_trigger).expect("上沿触碰产出 Reduce");
        assert_eq!(reduce_action, CenterOscillationAction::Reduce);
        let tw1 = acct.record_and_apply(&tw0, reduce_action, 15, id.zg).unwrap();
        assert_eq!(
            tw1.tw() - tw0.tw(),
            15 * (id.zg - avg_cost),
            "TW 漂移=本轮已实现盈亏（Realize），非 0——P1 修复口径"
        );
        assert_eq!(acct.bucket.open_units(), 15);

        // 下沿回补：次级别买点，价格=中枢下沿 zd；avg_cost 仍是同一持仓的原均价（现算派生，
        // 不因回补而变——本账本 cost_basis 全程不变）。
        let cover_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        let cover_action = book.on_trigger(cover_trigger).expect("挂起中 ⟹ 下沿触碰产出 Replenish");
        assert_eq!(cover_action, CenterOscillationAction::Replenish);
        let tw2 = acct.record_and_apply(&tw1, cover_action, 15, id.zd).unwrap();
        assert_eq!(
            tw2.tw() - tw1.tw(),
            15 * (avg_cost - id.zd),
            "回补侧 TW 漂移=本轮 Realize=units·(avg_cost−price)（P1 续修：非旧版恒 0）"
        );

        assert!(acct.assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash(), 15 * id.zg - 15 * id.zd, "桶累计=高抛低吸净现金（报告层口径不变）");
        assert_eq!(
            tw2.tw() - tw0.tw(),
            15 * (id.zg - id.zd),
            "全程 TW 总漂移=ΣRealize=真实现金差价=units·(price_sell−price_buy)，与 avg_cost 无关"
        );
        assert_eq!(acct.cost_basis(), snapshot(500, 20_000), "本仓成本基全程不动");
        assert!(!book.is_suspended(id), "回补出口=挂起清空（#292 既有语义不受影响）");
    }
}
