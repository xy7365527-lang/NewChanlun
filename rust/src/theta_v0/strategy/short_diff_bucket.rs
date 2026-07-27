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
//! ## 范围边界
//!
//! 本模块只做「桶记账 + TW 桥 + 两条断言」——本仓成本基本体（[`CoreCostBasisSnapshot`]）
//! 只读传导，唯二合法写口在别处（#197 G3 既有机制，不在本票范围）；每仓 campaign 粒度属
//! #294（T4）；`CenterOscillationAction` 的**数量**（units）来源（真实 sizing 口径）未在
//! #292/#293 任一契约内定义——本模块把 units 作调用方显式入参（不臆造默认值），生产侧
//! 真正实例化 TW 账本 + 喂入真实 units 属 #294（每仓 campaign 粒度落地时）的自然接线点。

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
/// 报告层累计读数 + 挂起在途 units（恒仓断言状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortDiffBucket {
    /// 报告层读数：短差往返累计净现金（= Σ d_cash，缠师口径「现金积累=成本」的载体）。
    pub realized_cash: i64,
    /// 挂起在途 units（本轮 `Reduce` 卖出、尚未被 `Replenish` 买回的部分）——恒仓断言状态。
    open_units: i64,
}

impl ShortDiffBucket {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前挂起在途 units（诊断/测试用只读访问）。
    pub fn open_units(&self) -> i64 {
        self.open_units
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
    /// `short_diff`），返回新 [`TwState`]（不可变模式）。
    pub fn apply(&self, tw: &TwState) -> TwState {
        let tw1 = tw_step(tw, self.short_diff);
        match self.realize {
            Some(ev) => tw_step(&tw1, ev),
            None => tw1,
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffAccount {
    pub bucket: ShortDiffBucket,
    pub cost_basis: CoreCostBasisSnapshot,
}

impl ShortDiffAccount {
    pub fn new(cost_basis: CoreCostBasisSnapshot) -> Self {
        Self { bucket: ShortDiffBucket::new(), cost_basis }
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
    /// `avg_cost`：持仓均价，caller 显式入参——同 `units`/`price` 一样不臆造默认值（本模块
    /// 「范围边界」既有原则），`Reduce`/`Replenish` 均生效（P1 续修前 `Replenish` 分支对该
    /// 入参不生效，是 #293 复审浮出的病灶）。真实生产来源 = 持仓账本接口（本仓成本基/units
    /// 均价读数，见 [`CoreCostBasisSnapshot`]）；本模块只读消费，不推导/不重算。生产侧尚未
    /// 实例化真实账本接线——这与 units 的真实 sizing 口径是**同一个接口缺口**（#294 前置票
    /// 落地时一并接入）。
    ///
    /// 恒仓断言（严格，违规即显式失败）：`units<=0` 或回补超过挂起在途量均返回 typed
    /// `Err`，不静默钳制/吸收；失败时本账本状态**不变**（桶/在途量均未落笔）。
    pub fn record_action(
        &mut self,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
        avg_cost: i64,
    ) -> Result<ShortDiffEvents, ShortDiffViolation> {
        if units <= 0 {
            return Err(ShortDiffViolation::NonPositiveUnits(units));
        }
        let events = match action {
            CenterOscillationAction::Reduce => {
                self.bucket.open_units += units;
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
    /// 账本——TW 桥的端到端接线点（成本基划转 + 已实现盈亏两笔均入账）。
    pub fn record_and_apply(
        &mut self,
        tw: &TwState,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
        avg_cost: i64,
    ) -> Result<TwState, ShortDiffViolation> {
        let events = self.record_action(action, units, price, avg_cost)?;
        Ok(events.apply(tw))
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

    // ── 单源记账（禁双写，新口径：桶净现金增量 = 本次产出全部 TwEvent 金额之和） ──

    /// P1 修复核心用例：`Reduce` 拆两笔——`ShortDiff(units·avg_cost)`（成本基划转）+
    /// `Realize(units·(price−avg_cost))`（已实现短差盈亏）；两笔金额之和等于桶净现金增量
    /// （新禁双写口径，替代旧「单一 d_cash 同时喂桶与 TwEvent」表述）。
    #[test]
    fn reduce_splits_into_short_diff_and_realize_bucket_delta_equals_sum() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let events = acct.record_action(CenterOscillationAction::Reduce, 10, 12, 9).unwrap();
        assert_eq!(events.short_diff, TwEvent::ShortDiff(90), "成本基划转=units·avg_cost=10·9=90");
        assert_eq!(
            events.realize,
            Some(TwEvent::Realize(30)),
            "已实现短差盈亏=units·(price−avg_cost)=10·(12−9)=30"
        );
        assert_eq!(acct.bucket.realized_cash, 120, "桶净现金读数=units·price=10·12=120（报告层口径不变）");
        assert_eq!(events.total_d_cash(), 120, "两笔金额之和=桶净现金增量（新禁双写口径）");

        let tw0 = TwState::initial();
        let tw1 = events.apply(&tw0);
        let tw1_manual = tw_step(&tw_step(&tw0, events.short_diff), events.realize.unwrap());
        assert_eq!(tw1, tw1_manual, "ShortDiffEvents::apply 与手工两步 tw_step 逐字节相同");
    }

    /// P1 续修核心用例（issue #293 复审浮出）：`Replenish` 对偶拆两笔——
    /// `ShortDiff(-units·avg_cost)`（按原均价划回成本基，非成交价）+
    /// `Realize(units·(avg_cost−price))`（买回侧已实现盈亏）。覆盖买价<均价（买便宜=正
    /// Realize）与买价>均价（买贵=负 Realize）两种情形。
    #[test]
    fn replenish_splits_into_short_diff_and_realize_dual_to_reduce() {
        // 买价(7) < 均价(9) ⟹ 买便宜为正=赚。
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12, 9).unwrap();
        let cheap = acct.record_action(CenterOscillationAction::Replenish, 10, 7, 9).unwrap();
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
        let mut acct2 = ShortDiffAccount::new(snapshot(0, 0));
        acct2.record_action(CenterOscillationAction::Reduce, 10, 12, 9).unwrap();
        let expensive = acct2.record_action(CenterOscillationAction::Replenish, 10, 11, 9).unwrap();
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
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let events = acct.record_action(CenterOscillationAction::Reduce, 10, 12, 9).unwrap();
        match events.realize {
            Some(TwEvent::Realize(d)) => {
                assert_eq!(d, 30, "短差盈亏必须作为独立 Realize 落账（非 0/None——0/None 是静默吸收的症状）")
            }
            other => panic!("Reduce 必须产出 Realize 事件，实际={other:?}"),
        }
        let tw0 = TwState::initial();
        let tw1 = events.apply(&tw0);
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
        let mut acct = ShortDiffAccount::new(snapshot(300, 3_000)); // 均价10，300股
        acct.record_action(CenterOscillationAction::Reduce, 10, 12, 10).unwrap(); // 卖10@12
        let events = acct.record_action(CenterOscillationAction::Replenish, 10, 9, 10).unwrap(); // 买回10@9
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
        let tw0 = TwState::initial();
        let tw1 = events.apply(&tw0);
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
        let mut acct = ShortDiffAccount::new(snapshot(1000, 50_000));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12, 10).unwrap(); // 卖10@12，均价10
        acct.record_action(CenterOscillationAction::Replenish, 10, 9, 10).unwrap(); // 买回10@9，均价10
        assert_eq!(acct.bucket.open_units(), 0, "同股数进出 ⟹ 挂起在途量归零");
        assert!(acct.assert_conserved().is_ok(), "往返闭合 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash, 10 * 12 - 10 * 9, "桶累计=往返净现金=30（高抛低吸获利）");
    }

    /// 多轮往返（高抛→回补→再高抛→再回补）逐轮都守恒，累计现金逐轮叠加。
    #[test]
    fn multi_round_trips_each_conserve_independently() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 5, 20, 17).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 5, 18, 17).unwrap();
        assert!(acct.assert_conserved().is_ok());
        acct.record_action(CenterOscillationAction::Reduce, 7, 22, 19).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 7, 19, 19).unwrap();
        assert!(acct.assert_conserved().is_ok());
        assert_eq!(acct.bucket.realized_cash, (5 * 20 - 5 * 18) + (7 * 22 - 7 * 19));
    }

    // ── 恒仓断言：违规显式失败（用户裁定：严格） ────────────────────────────

    /// units<=0 当场拒绝，不静默吸收为 0/1。
    #[test]
    fn nonpositive_units_is_explicit_violation() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, 0, 10, 8),
            Err(ShortDiffViolation::NonPositiveUnits(0))
        );
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, -3, 10, 8),
            Err(ShortDiffViolation::NonPositiveUnits(-3))
        );
        assert_eq!(acct.bucket.realized_cash, 0, "拒绝的调用不落笔");
    }

    /// 超额回补（买回多于挂起在途量）当场拒绝——不静默钳制到「只买回挂起的那部分」。
    #[test]
    fn over_replenish_is_explicit_violation_and_state_unchanged() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 5, 10, 8).unwrap(); // 卖5，挂起=5
        let before = acct;
        let result = acct.record_action(CenterOscillationAction::Replenish, 8, 9, 8); // 试图买8
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
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 10, 20, 17).unwrap(); // 卖10
        let partial = acct.record_action(CenterOscillationAction::Replenish, 1, 18, 17); // 只买1
        assert!(partial.is_ok(), "部分回补（1<10）当场合法——往返可以分批收口");
        assert_eq!(acct.bucket.open_units(), 9, "N:1（10:1）⟹ 挂起在途量=9，未闭合");
        assert_eq!(
            acct.assert_conserved(),
            Err(ShortDiffViolation::UnclosedRoundTrip { open_units: 9 }),
            "边界核验：N:1 不守恒 ⟹ 显式失败，不静默视为已收口"
        );
    }

    // ── 本仓成本基不动断言 ───────────────────────────────────────────────

    /// 往返后成本基逐位不变（PartialEq 全字段比对）；报告层成本读数（均价口径）与桶
    /// 累计（短差净现金）分列在同一记账体的两个不相干字段，互不覆写。
    #[test]
    fn cost_basis_untouched_across_round_trips_bucket_listed_separately() {
        let original = snapshot(1_000, 50_000);
        let mut acct = ShortDiffAccount::new(original);
        for _ in 0..3 {
            acct.record_action(CenterOscillationAction::Reduce, 10, 15, 13).unwrap();
            acct.record_action(CenterOscillationAction::Replenish, 10, 11, 13).unwrap();
        }
        assert_eq!(acct.cost_basis, original, "本仓成本基（均价口径）往返后逐位不变");
        assert_eq!(acct.bucket.realized_cash, 3 * (10 * 15 - 10 * 11), "桶累计另列，不并入成本基");
        assert!(acct.assert_conserved().is_ok());
    }

    /// 违规调用（超额回补被拒）也不动成本基（成本基字段完全独立于短差记账结果）。
    #[test]
    fn cost_basis_untouched_even_when_short_diff_call_is_rejected() {
        let original = snapshot(200, 8_000);
        let mut acct = ShortDiffAccount::new(original);
        acct.record_action(CenterOscillationAction::Reduce, 5, 10, 8).unwrap();
        let _ = acct.record_action(CenterOscillationAction::Replenish, 99, 9, 8); // 拒绝
        assert_eq!(acct.cost_basis, original);
    }

    // ── TW 桥：守恒口径改判（P1 修复，issue #293） ──────────────────────────

    /// ★口径改判核心用例：往返**不再断言 TW 不变**——`ShortDiff` 恒守恒（成本基划转），
    /// `Realize` 恒漂移（已实现盈亏入账），二者语义不同不该合并断言；旧版把 Reduce 全额
    /// 款塞单一 `ShortDiff` 制造的表面守恒正是本 bug 的症状。新断言：**TW 漂移 = Σ Realize**
    /// （对齐 `ledger.rs::tw_step_realize_drift_equals_dpi` 既有定理，本测试是桥接层核验）。
    #[test]
    fn tw_drift_equals_sum_of_realize_through_short_diff_round_trip() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let tw0 = TwState { free: 1_000, holding: 500, ..TwState::initial() };
        let tw0_total = tw0.tw();

        // Reduce(20@8，均价6) ⟹ ShortDiff(20·6=120)（守恒）+ Realize(20·(8−6)=40)（漂移+40）。
        let events1 = acct.record_action(CenterOscillationAction::Reduce, 20, 8, 6).unwrap();
        let tw1 = events1.apply(&tw0);
        assert_eq!(tw1.free, tw0.free + 120 + 40, "free 吸收成本基划转(120)+已实现盈亏(40)");
        assert_eq!(tw1.holding, tw0.holding - 120, "holding 只减成本基份额(120)，非全额卖出款(160)");
        assert_eq!(tw1.tw() - tw0_total, 40, "TW 漂移=本轮 Realize=40（非 0——旧口径的症状）");

        // Replenish(20@6，均价6) ⟹ ShortDiff(-20·6=-120)（守恒）+ Realize(20·(6−6)=0)——买回价
        // 恰等于均价，本轮无盈亏，但仍是显式 Realize(0) 事件（非 None，对偶拆分恒产出两笔）。
        let events2 = acct.record_action(CenterOscillationAction::Replenish, 20, 6, 6).unwrap();
        assert_eq!(events2.realize, Some(TwEvent::Realize(0)), "买价=均价 ⟹ Realize(0)，非 None");
        let tw2 = events2.apply(&tw1);
        assert_eq!(tw2.tw(), tw1.tw(), "本轮 Realize=0 ⟹ TW 不再漂移（ShortDiff 恒守恒）");

        assert_eq!(tw2.tw() - tw0_total, 40, "全程 TW 总漂移=Σ Realize=40（唯一一次非零 Realize）");
        assert_eq!(acct.bucket.realized_cash, 160 - 120, "桶净现金读数=往返净现金=40（报告层口径不变）");
    }

    /// ★全往返对账核心用例（issue #293 复审要求，P1 续修）：卖侧
    /// `Realize(units·(price_sell−avg_cost))` + 买侧 `Realize(units·(avg_cost−price_buy))`
    /// = ΣRealize = **真实现金差价**（化简后与 `avg_cost` 无关，恰等于
    /// `units·(price_sell−price_buy)`）；`ShortDiff` 两笔相消 ⟹ `holding` 净 0，往返后回到
    /// 原值（本仓成本基份额行为化不动，修 7 裁定）；TW 全程漂移 = ΣRealize。
    #[test]
    fn full_round_trip_holding_returns_to_original_and_tw_drift_equals_sum_of_realize() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let tw0 = TwState { free: 1_000, holding: 500, ..TwState::initial() };

        // 卖20@8，均价6 ⟹ ShortDiff(120) + Realize(20·(8−6)=40)。
        let sell = acct.record_action(CenterOscillationAction::Reduce, 20, 8, 6).unwrap();
        let tw1 = sell.apply(&tw0);
        assert_eq!(tw1.holding, tw0.holding - 120, "holding 减成本基份额=units·avg_cost=120");

        // 买回20@5（买便宜），均价6（同一持仓，成本基不变）⟹ ShortDiff(-120) + Realize(20·(6−5)=20)。
        let buy = acct.record_action(CenterOscillationAction::Replenish, 20, 5, 6).unwrap();
        let tw2 = buy.apply(&tw1);
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
        assert_eq!(acct.bucket.realized_cash, 20 * 8 - 20 * 5, "桶净现金读数=往返净现金=60（报告层口径不变）");
        assert!(acct.assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
    }

    // ── #292 → #293 端到端桥接：真实 `CenterOscillationAction` 落成 TW 事件 ──

    fn cid(start_index: usize, zd: i64, zg: i64) -> CenterId {
        CenterId::of(&Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index: start_index + 50 })
    }

    /// ★端到端契约衔接证据：#292 `CenterOscillationBook::on_trigger` 产出的真实
    /// `CenterOscillationAction`（非合成/占位值）直接喂入 `ShortDiffAccount::record_and_apply`
    /// ⟹ 正确落成 `ShortDiffEvents`（Reduce/Replenish 均两笔，对偶拆分）并推进 TW 账本；往返
    /// 闭合后恒仓断言过、成本基不动；TW 全程总漂移=ΣRealize=真实现金差价（P1 续修口径，非旧
    /// 「回补侧无 Realize」）。
    #[test]
    fn center_oscillation_action_from_book_bridges_into_tw_ledger() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let mut acct = ShortDiffAccount::new(snapshot(500, 20_000));
        let tw0 = TwState::initial();
        let avg_cost = 20_000 / 500; // 本仓成本基（均价口径，见 snapshot），= 40。

        // 上沿高抛：次级别卖点，价格=中枢上沿 zg（真实 #292 触发构造，非合成值）。
        let reduce_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let reduce_action = book.on_trigger(reduce_trigger).expect("上沿触碰产出 Reduce");
        assert_eq!(reduce_action, CenterOscillationAction::Reduce);
        let tw1 = acct.record_and_apply(&tw0, reduce_action, 15, id.zg, avg_cost).unwrap();
        assert_eq!(
            tw1.tw() - tw0.tw(),
            15 * (id.zg - avg_cost),
            "TW 漂移=本轮已实现盈亏（Realize），非 0——P1 修复口径"
        );
        assert_eq!(acct.bucket.open_units(), 15);

        // 下沿回补：次级别买点，价格=中枢下沿 zd；avg_cost 仍是同一持仓的原均价（不因回补而变）。
        let cover_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        let cover_action = book.on_trigger(cover_trigger).expect("挂起中 ⟹ 下沿触碰产出 Replenish");
        assert_eq!(cover_action, CenterOscillationAction::Replenish);
        let tw2 = acct.record_and_apply(&tw1, cover_action, 15, id.zd, avg_cost).unwrap();
        assert_eq!(
            tw2.tw() - tw1.tw(),
            15 * (avg_cost - id.zd),
            "回补侧 TW 漂移=本轮 Realize=units·(avg_cost−price)（P1 续修：非旧版恒 0）"
        );

        assert!(acct.assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash, 15 * id.zg - 15 * id.zd, "桶累计=高抛低吸净现金（报告层口径不变）");
        assert_eq!(
            tw2.tw() - tw0.tw(),
            15 * (id.zg - id.zd),
            "全程 TW 总漂移=ΣRealize=真实现金差价=units·(price_sell−price_buy)，与 avg_cost 无关"
        );
        assert_eq!(acct.cost_basis, snapshot(500, 20_000), "本仓成本基全程不动");
        assert!(!book.is_suspended(id), "回补出口=挂起清空（#292 既有语义不受影响）");
    }
}
