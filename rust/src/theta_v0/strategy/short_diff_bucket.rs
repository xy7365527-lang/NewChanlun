//! 短差盈亏桶 + TW 桥（SPEC #287 T3，issue #293）。
//!
//! ADR 0001 修正案一/补充二/补充三裁定落地：短差往返
//! （[`center_oscillation_trade::CenterOscillationAction`] 的 Reduce/Replenish，#292 T2）
//! **不改**本仓成本基（均价口径全程不动；唯二合法出口=全平清零/加仓重算，属
//! [`super::account::ParallelAccountLedger::fill`] 的既有职责，本模块不触碰）。短差往返
//! 盈亏单独记该级账内「短差盈亏」标签桶——桶是**报告层读数 + TW 输入的单一数据源**：
//! 同一笔 `d_cash` 既累计进桶（报告层），也构造 [`TwEvent::ShortDiff`]（TW 输入），
//! 禁止两处各自重算（禁双写）。
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
/// **类型层保证**（无 setter 可调用），不是运行时检查后的承诺。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoreCostBasisSnapshot {
    pub units: i64,
    pub cost_basis: i64,
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

    /// 单源记账：把一次 [`CenterOscillationAction`]（#292 T2 产出）落成 `d_cash`——同一个
    /// 值既累计进桶（报告层 [`ShortDiffBucket::realized_cash`]）也构造
    /// [`TwEvent::ShortDiff`]（TW 输入），禁止两处各自重算（禁双写）。
    ///
    /// - `Reduce`（高抛卖出）⟹ `d_cash = units·price > 0`（TW 侧 holding→free）。
    /// - `Replenish`（回补买入）⟹ `d_cash = -units·price < 0`（TW 侧 free→holding）。
    ///
    /// 恒仓断言（严格，违规即显式失败）：`units<=0` 或回补超过挂起在途量均返回 typed
    /// `Err`，不静默钳制/吸收；失败时本账本状态**不变**（桶/在途量均未落笔）。
    pub fn record_action(
        &mut self,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
    ) -> Result<TwEvent, ShortDiffViolation> {
        if units <= 0 {
            return Err(ShortDiffViolation::NonPositiveUnits(units));
        }
        let d_cash = match action {
            CenterOscillationAction::Reduce => {
                self.bucket.open_units += units;
                units * price
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
                -(units * price)
            }
        };
        self.bucket.realized_cash += d_cash;
        Ok(TwEvent::ShortDiff(d_cash))
    }

    /// [`record_action`](Self::record_action) + 立即经 [`tw_step`] 落到 TW 账本
    /// ——TW 桥的端到端接线点（成本基划转入账）。
    pub fn record_and_apply(
        &mut self,
        tw: &TwState,
        action: CenterOscillationAction,
        units: i64,
        price: i64,
    ) -> Result<TwState, ShortDiffViolation> {
        let event = self.record_action(action, units, price)?;
        Ok(tw_step(tw, event))
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
        CoreCostBasisSnapshot { units, cost_basis }
    }

    // ── 单源记账（禁双写） ────────────────────────────────────────────────

    /// ★禁双写：`record_action` 返回的 `TwEvent` 与桶累计的 `d_cash` 是同一个值——
    /// 手工重算 `tw_step` 必须与 `record_and_apply` 的产出逐字节相同。
    #[test]
    fn bucket_and_tw_event_share_single_d_cash_source() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let event = acct.record_action(CenterOscillationAction::Reduce, 10, 5).unwrap();
        assert_eq!(event, TwEvent::ShortDiff(50), "Reduce(10@5) ⟹ d_cash=+50（卖出）");
        assert_eq!(acct.bucket.realized_cash, 50, "桶累计=同一 d_cash");

        let tw0 = TwState::initial();
        let mut acct2 = ShortDiffAccount::new(snapshot(0, 0));
        let tw1 = acct2.record_and_apply(&tw0, CenterOscillationAction::Reduce, 10, 5).unwrap();
        let tw1_manual = tw_step(&tw0, TwEvent::ShortDiff(50));
        assert_eq!(tw1, tw1_manual, "record_and_apply 与手工 tw_step(同一 d_cash) 逐字节相同");
        assert_eq!(acct2.bucket.realized_cash, 50, "同一记账体：桶累计与 TW 输入同源一致");
    }

    /// `Replenish` 的 `d_cash` 符号相反（买入，free→holding）。
    #[test]
    fn replenish_produces_negative_d_cash() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap();
        let event = acct.record_action(CenterOscillationAction::Replenish, 10, 9).unwrap();
        assert_eq!(event, TwEvent::ShortDiff(-90), "Replenish(10@9) ⟹ d_cash=-90（买回）");
    }

    // ── 恒仓断言：守恒过 ──────────────────────────────────────────────────

    /// 完整往返（同股数进出）⟹ 挂起在途量归零，恒仓断言过；桶累计=往返净现金（缠师
    /// 「现金积累=成本」口径）。
    #[test]
    fn full_round_trip_conserves_units_and_realizes_cash() {
        let mut acct = ShortDiffAccount::new(snapshot(1000, 50_000));
        acct.record_action(CenterOscillationAction::Reduce, 10, 12).unwrap(); // 卖10@12
        acct.record_action(CenterOscillationAction::Replenish, 10, 9).unwrap(); // 买10@9
        assert_eq!(acct.bucket.open_units(), 0, "同股数进出 ⟹ 挂起在途量归零");
        assert!(acct.assert_conserved().is_ok(), "往返闭合 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash, 10 * 12 - 10 * 9, "桶累计=往返净现金=30（高抛低吸获利）");
    }

    /// 多轮往返（高抛→回补→再高抛→再回补）逐轮都守恒，累计现金逐轮叠加。
    #[test]
    fn multi_round_trips_each_conserve_independently() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        acct.record_action(CenterOscillationAction::Reduce, 5, 20).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 5, 18).unwrap();
        assert!(acct.assert_conserved().is_ok());
        acct.record_action(CenterOscillationAction::Reduce, 7, 22).unwrap();
        acct.record_action(CenterOscillationAction::Replenish, 7, 19).unwrap();
        assert!(acct.assert_conserved().is_ok());
        assert_eq!(acct.bucket.realized_cash, (5 * 20 - 5 * 18) + (7 * 22 - 7 * 19));
    }

    // ── 恒仓断言：违规显式失败（用户裁定：严格） ────────────────────────────

    /// units<=0 当场拒绝，不静默吸收为 0/1。
    #[test]
    fn nonpositive_units_is_explicit_violation() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, 0, 10),
            Err(ShortDiffViolation::NonPositiveUnits(0))
        );
        assert_eq!(
            acct.record_action(CenterOscillationAction::Reduce, -3, 10),
            Err(ShortDiffViolation::NonPositiveUnits(-3))
        );
        assert_eq!(acct.bucket.realized_cash, 0, "拒绝的调用不落笔");
    }

    /// 超额回补（买回多于挂起在途量）当场拒绝——不静默钳制到「只买回挂起的那部分」。
    #[test]
    fn over_replenish_is_explicit_violation_and_state_unchanged() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
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
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
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
        assert_eq!(acct.cost_basis, original, "本仓成本基（均价口径）往返后逐位不变");
        assert_eq!(acct.bucket.realized_cash, 3 * (10 * 15 - 10 * 11), "桶累计另列，不并入成本基");
        assert!(acct.assert_conserved().is_ok());
    }

    /// 违规调用（超额回补被拒）也不动成本基（成本基字段完全独立于短差记账结果）。
    #[test]
    fn cost_basis_untouched_even_when_short_diff_call_is_rejected() {
        let original = snapshot(200, 8_000);
        let mut acct = ShortDiffAccount::new(original);
        acct.record_action(CenterOscillationAction::Reduce, 5, 10).unwrap();
        let _ = acct.record_action(CenterOscillationAction::Replenish, 99, 9); // 拒绝
        assert_eq!(acct.cost_basis, original);
    }

    // ── TW 桥：TwEvent::ShortDiff 守恒对齐（回归 ledger.rs 既有守恒定理） ────

    /// 短差往返经 [`record_and_apply`] 全程保 `TW=free+holding+withdrawn` 守恒
    /// （对齐 `ledger.rs::tw_step_preserves_tw`——`ShortDiff` 是守恒构造子之一，本测试
    /// 从 T3 桥接层再次核验，非重复 ledger.rs 单测，是桥接正确性的独立证据）。
    #[test]
    fn tw_conservation_preserved_through_short_diff_round_trip() {
        let mut acct = ShortDiffAccount::new(snapshot(0, 0));
        let tw0 = TwState { free: 1_000, holding: 500, ..TwState::initial() };
        let tw0_total = tw0.tw();
        let tw1 = acct.record_and_apply(&tw0, CenterOscillationAction::Reduce, 20, 8).unwrap();
        assert_eq!(tw1.tw(), tw0_total, "Reduce 后 TW 守恒");
        assert_eq!(tw1.free, tw0.free + 160, "d_cash=160 计入 free（holding→free）");
        assert_eq!(tw1.holding, tw0.holding - 160);
        let tw2 = acct.record_and_apply(&tw1, CenterOscillationAction::Replenish, 20, 6).unwrap();
        assert_eq!(tw2.tw(), tw0_total, "Replenish 后 TW 仍守恒");
        assert_eq!(tw2.free, tw1.free - 120, "d_cash=-120 计入 free（free→holding）");
        assert_eq!(acct.bucket.realized_cash, 160 - 120, "桶累计=往返净现金=40");
    }

    // ── #292 → #293 端到端桥接：真实 `CenterOscillationAction` 落成 TW 事件 ──

    fn cid(start_index: usize, zd: i64, zg: i64) -> CenterId {
        CenterId::of(&Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index: start_index + 50 })
    }

    /// ★端到端契约衔接证据：#292 `CenterOscillationBook::on_trigger` 产出的真实
    /// `CenterOscillationAction`（非合成/占位值）直接喂入 `ShortDiffAccount::record_and_apply`
    /// ⟹ 正确落成 `TwEvent::ShortDiff` 并推进 TW 账本；往返闭合后恒仓断言过、成本基不动。
    #[test]
    fn center_oscillation_action_from_book_bridges_into_tw_ledger() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let mut acct = ShortDiffAccount::new(snapshot(500, 20_000));
        let tw0 = TwState::initial();

        // 上沿高抛：次级别卖点，价格=中枢上沿 zg（真实 #292 触发构造，非合成值）。
        let reduce_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let reduce_action = book.on_trigger(reduce_trigger).expect("上沿触碰产出 Reduce");
        assert_eq!(reduce_action, CenterOscillationAction::Reduce);
        let tw1 = acct.record_and_apply(&tw0, reduce_action, 15, id.zg).unwrap();
        assert_eq!(tw1.tw(), tw0.tw(), "TW 守恒");
        assert_eq!(acct.bucket.open_units(), 15);

        // 下沿回补：次级别买点，价格=中枢下沿 zd。
        let cover_trigger =
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        let cover_action = book.on_trigger(cover_trigger).expect("挂起中 ⟹ 下沿触碰产出 Replenish");
        assert_eq!(cover_action, CenterOscillationAction::Replenish);
        let tw2 = acct.record_and_apply(&tw1, cover_action, 15, id.zd).unwrap();
        assert_eq!(tw2.tw(), tw0.tw(), "回补后 TW 仍守恒");

        assert!(acct.assert_conserved().is_ok(), "同股数进出 ⟹ 恒仓断言过");
        assert_eq!(acct.bucket.realized_cash, 15 * id.zg - 15 * id.zd, "桶累计=高抛低吸净现金");
        assert_eq!(acct.cost_basis, snapshot(500, 20_000), "本仓成本基全程不动");
        assert!(!book.is_suspended(id), "回补出口=挂起清空（#292 既有语义不受影响）");
    }
}
