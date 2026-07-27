//! Treasury 薄适配层（S1，task #9）——μ 摆动信号 → `TwEvent::Realize` 的费后结算换算。
//!
//! ## 设计边界（treasury-execution-plan-20260707.md S1 / treasury-prereg-v1-draft-20260707.md）
//!
//! - **不是新账本**。TW 账本单一生产真值源 = [`TwState`]（#124 裁定4，runner.rs:860）。
//!   本模块只做一件事：把 μ 层转发来的已兑现摆动 `(units, H, L)` 换算成费后已实现
//!   PnL，经 [`TwEvent::Realize`]（唯一 TW 漂移构造子，codex GAP3 裁定 A'）入账。
//! - **w≡1 首阶段**：API 不暴露权重参数（死参数是坏品味）。w 非平凡升级点预注册为
//!   P3 里程碑（prereg 草案 §1）——届时在 μ 层调制 `units`，本层签名不变。
//! - **成本口径**：`fee_rate = (commission + slippage + tax) bps / 1e4`，与
//!   [`econ_positive`](super::econ_positive)（econ_positive.rs:233）及
//!   `mu_estimator::marginal_return` 同口径，单边计费、双腿各扣一次。
//! - **舍入策略**：入账额 `floor`（f64 → i64 现金单位）——盈利低估、亏损高估，
//!   会计保守方向。调用方负责让价格与 TW 账本共享同一现金单位。
//!
//! ## 认识论等级
//!
//! **L1**（适配算术正确性，零市场信息增量）。判据 `(H−L)/(H+L) > fee_rate` 与
//! R3（gn-feasibility-btc-20260707.md §1）逐字对齐；本模块不主张任何 alpha——
//! 充分性检验属 S3（walk-forward OOS）。

use super::super::config::ExecConfig;
use super::super::strategy::ledger::{tw_step, TwEvent, TwState};

/// **未标定 fallback 单边费率**（分数）。`ExecConfig::default()` 下恰为 `3e-4`（1+2+0 bps）。
///
/// ★#360 后语义收窄：本函数给的是 `fee_schedule = None` 档的常率（口径标签
/// [`RATE_UNCALIBRATED_LABEL`](super::super::strategy::risk::RATE_UNCALIBRATED_LABEL)）。
/// **生产成交点不再直接调它**——改调 [`fee_quoter`]（None 档解析结果与本函数逐位相同）。
/// 仍直接调本函数的位点（`runner::RunResult::fee_rate` 随机对照成本口径、研究 bin/诊断跑批）
/// 一律是"未标定常率"语义，不消费 datum 档（见 `ExecConfig::fee_schedule` 文档的消费面登记）。
pub fn fee_rate(exec: &ExecConfig) -> f64 {
    (exec.commission_bps + exec.slippage_bps + exec.tax_bps) / 10_000.0
}

/// **成交费率单源门面**（#360，报告 §3.1 item 3）：把 `ExecConfig` 的 None/Some 分叉收口成
/// 一个解析器，生产成交回路只跟它打交道。
///
/// `None` ⟹ 恒返回 [`fee_rate`] 的常率（**逐位现状**）；`Some` ⟹ 按 datum 逐笔解析。
pub fn fee_quoter(exec: &ExecConfig) -> super::super::venue_fee::FeeQuoter<'_> {
    // ★标定档下 `tax_bps` 必须为 0（fail-loud，禁静默双计）：两个在册 venue 的交易税费科目
    //   已由 datum 逐项承载（IBKR = SEC Section 31 + FINRA TAF/CAT；Binance 现货无此科目），
    //   再叠一个笼统 `tax_bps` 会重复计。非零税辖区（如 A 股印花税）没有在册 venue 档。
    assert!(
        exec.fee_schedule.is_none() || exec.tax_bps == 0.0,
        "venue 标定档与非零 tax_bps({}) 并用：交易税费科目已在 datum 内逐项承载，重复计禁止\
         （要么清零 tax_bps，要么为该辖区落一份含税科目的 datum）",
        exec.tax_bps
    );
    super::super::venue_fee::FeeQuoter::new(
        fee_rate(exec),
        // datum 不覆盖的摩擦科目：滑点（价差/冲击性质，报告 §3.3 明文保留未标定）。
        exec.slippage_bps / 10_000.0,
        exec.fee_schedule.as_ref(),
    )
}

/// μ 层转发的一笔已兑现摆动：在 `high` 卖出 `units`、在 `low` 买回。
///
/// S1 骨架里 μ 层不做预测——`MuSwing` 由上游（S3 回测器/测试）构造，本层只结算。
/// 价格单位 = TW 账本现金单位（调用方保证）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MuSwing {
    /// 摆动仓量（>0）。w≡1：无权重调制。
    pub units: f64,
    /// 卖出价 H。
    pub high: f64,
    /// 买回价 L。
    pub low: f64,
}

impl MuSwing {
    /// 费后已实现 PnL（未舍入）：`units·(H−L) − fee·units·(H+L)`。
    ///
    /// 双腿成本 `Cs+Cb = fee·units·H + fee·units·L`——与 R3 §1 的化简同一式。
    pub fn net_realized(&self, fee_rate: f64) -> f64 {
        self.units * (self.high - self.low) - fee_rate * self.units * (self.high + self.low)
    }

    /// R3 判据：`(H−L)/(H+L) > fee_rate` ⟺ `net_realized > 0`（units>0 时恒等价）。
    pub fn clears_cost(&self, fee_rate: f64) -> bool {
        (self.high - self.low) / (self.high + self.low) > fee_rate
    }
}

/// 结算一笔摆动入 TW 账本。返回（新账本态，入账额 d_pi）。
///
/// 漂移不变量：`tw(后) − tw(前) == d_pi`（由 ledger 的
/// `tw_step_realize_drift_equals_dpi` 承保，此处经适配层再断言一次）。
pub fn settle(s: &TwState, swing: &MuSwing, fee_rate: f64) -> (TwState, i64) {
    debug_assert!(swing.units > 0.0, "MuSwing.units 必须为正（空摆动不应构造）");
    debug_assert!(
        swing.high >= swing.low,
        "MuSwing 约定 high >= low（方向由上游归一）"
    );
    let d_pi = swing.net_realized(fee_rate).floor() as i64;
    let next = tw_step(s, TwEvent::Realize(d_pi));
    debug_assert_eq!(next.tw() - s.tw(), d_pi, "Realize 漂移必须恰等于 d_pi");
    (next, d_pi)
}

/// 结算一串摆动（S3 回测主循环的消费入口）。空流 ⇒ 账本严格不变（守恒）。
pub fn settle_all<'a, I>(s: &TwState, swings: I, fee_rate: f64) -> (TwState, Vec<i64>)
where
    I: IntoIterator<Item = &'a MuSwing>,
{
    let mut state = s.clone();
    let mut realized = Vec::new();
    for sw in swings {
        let (next, d) = settle(&state, sw, fee_rate);
        state = next;
        realized.push(d);
    }
    (state, realized)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 冻结口径：默认费率恰为 3e-4（prereg 草案 §2 / config.rs:228-230）。
    #[test]
    fn default_fee_rate_is_3e4() {
        let exec = ExecConfig::default();
        assert_eq!(fee_rate(&exec), 3e-4);
    }

    /// ★#360 单源门面：`fee_schedule = None` ⟹ quoter 对**任意** (qty, px, side, role)
    /// 逐位返回 [`fee_rate`] 的常率（None 不破的接口层物证）。
    #[test]
    fn fee_quoter_none_is_bit_exact_fallback() {
        use super::super::super::strategy::exec::FillSide;
        use super::super::super::venue_fee::LiquidityRole;
        let exec = ExecConfig::default();
        let q = fee_quoter(&exec);
        assert!(!q.is_calibrated());
        assert_eq!(q.fallback_rate(), fee_rate(&exec));
        for qty in [1e-8, 1.0, 1e6] {
            for px in [1e-6, 20.0, 120_000.0] {
                for side in [FillSide::Buy, FillSide::Sell] {
                    for role in [LiquidityRole::Maker, LiquidityRole::Taker] {
                        assert_eq!(q.rate_for(qty, px, side, role), 3e-4);
                    }
                }
            }
        }
    }

    /// ★#360：`Some(datum)` ⟹ quoter 走 venue 档（Binance 现货 VIP0 = 10bp/side，报告 §2.1）。
    #[test]
    fn fee_quoter_some_resolves_venue_datum() {
        use super::super::super::strategy::exec::FillSide;
        use super::super::super::venue_fee::{datum_dir, load_datum, LiquidityRole};
        let book = load_datum(&datum_dir().join("venue_fee_binance_spot_20260726.json"))
            .expect("Binance datum 可加载");
        let mut exec = ExecConfig::default();
        exec.fee_schedule = Some(book.resolve("BTC", "VIP0").expect("BTC/VIP0 在册"));
        let q = fee_quoter(&exec);
        assert!(q.is_calibrated());
        assert_eq!(q.datum_sha256(), Some(book.datum_sha256.as_str()));
        assert_eq!(
            q.rate_for(1.0, 50_000.0, FillSide::Buy, LiquidityRole::Taker),
            1e-3 + 2e-4,
            "VIP0 taker 10bp/side + 未被 datum 覆盖的 slippage 2bp（报告 §3.3）"
        );
        // fallback 仍可读（noop 位点占位），但已不是本档的成交费率。
        assert_eq!(q.fallback_rate(), 3e-4);
    }

    /// 守恒（S1 验收项）：空信号流下 treasury 恒等于初始值，严格相等。
    #[test]
    fn empty_signal_stream_preserves_tw_exactly() {
        let s0 = TwState::initial();
        let (s1, realized) = settle_all(&s0, [].iter(), 3e-4);
        assert!(realized.is_empty());
        assert_eq!(s1.tw(), s0.tw());
        assert_eq!(s1, s0);
    }

    /// 判据一致性：clears_cost ⟺ net_realized > 0，阈值两侧各取一点。
    #[test]
    fn clears_cost_iff_net_positive() {
        let fee = 3e-4;
        // 阈值点：(H−L)/(H+L) = fee ⟹ H/L = (1+fee)/(1−fee)。L=10000 基准。
        let l = 10_000.0;
        let h_at = l * (1.0 + fee) / (1.0 - fee);
        for (h, expect) in [(h_at * 1.001, true), (h_at * 0.999, false)] {
            let sw = MuSwing { units: 1e6, high: h, low: l };
            assert_eq!(sw.clears_cost(fee), expect, "h={h}");
            assert_eq!(sw.net_realized(fee) > 0.0, expect, "h={h}");
        }
    }

    /// 漂移不变量经适配层成立，且亏损摆动照实入负账（无语义回补）。
    #[test]
    fn settle_drift_equals_dpi_both_signs() {
        let fee = 3e-4;
        let s0 = TwState::initial();
        let win = MuSwing { units: 100.0, high: 10_100.0, low: 10_000.0 };
        let loss = MuSwing { units: 100.0, high: 10_001.0, low: 10_000.0 };
        let (s1, d1) = settle(&s0, &win, fee);
        assert!(d1 > 0);
        assert_eq!(s1.tw() - s0.tw(), d1);
        let (s2, d2) = settle(&s1, &loss, fee);
        assert!(d2 < 0, "幅比 5e-5 < fee ⟹ 必须负入账，d2={d2}");
        assert_eq!(s2.tw() - s1.tw(), d2);
    }

    /// 舍入保守方向：正 PnL 向下取整，负 PnL 向更负取整。
    #[test]
    fn floor_rounding_is_conservative() {
        let sw = MuSwing { units: 1.0, high: 101.7, low: 100.0 };
        let net = sw.net_realized(0.0); // 1.7
        let s0 = TwState::initial();
        let (_, d) = settle(&s0, &sw, 0.0);
        assert_eq!(d, 1);
        assert!((d as f64) <= net);
        let neg = MuSwing { units: 1.0, high: 100.1, low: 100.0 };
        let (_, dn) = settle(&s0, &neg, 3e-4); // net ≈ 0.1 − 0.06003 > 0 → floor 0
        assert_eq!(dn, 0, "微利被保守舍成 0，绝不虚增");
    }
}
