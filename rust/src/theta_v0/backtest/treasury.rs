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
/// 仍直接调本函数的位点（研究 bin/诊断跑批）一律是"未标定常率"语义，不消费 datum 档
/// （见 `ExecConfig::fee_schedule` 文档的消费面登记）。★#374 起 `runner::RunResult::fee_rate`
/// 改经 [`scalar_cost_rate`]（同值，但标定档下 fail-loud），不再直接调本函数。
pub fn fee_rate(exec: &ExecConfig) -> f64 {
    (exec.commission_bps + exec.slippage_bps + exec.tax_bps) / 10_000.0
}

/// **单标量成本口径的取值方式**（#374 MED-A）——供"把成本当作一个常数费率"的下游用：
/// `runner::RunResult::fee_rate`（→ `metrics::random_entry_controls` 的随机对照成本、
/// `l3_delta_r_alpha::rebuild_cost_series` 的鞅守卫成本剥离）。
///
/// **有效域严格小于定义域**（231号）：单标量费率只在**存在一个与 (qty, px, side) 无关的常数
/// 等效费率**的档上良定义。★#423 起该判定**按档位形态分叉**（#374 防线的精细化，不是拆除）：
///
/// - **未标定档**（`fee_schedule = None`）：常率 [`fee_rate`]，良定义（逐位不变）。
/// - **按金额标定档**（[`FeeUnit::Notional`](super::super::venue_fee::FeeUnit::Notional)，
///   maker/taker **对称**）：良定义。`effective_rate` 的 `Notional` 分支直接给 `bps/1e4`，
///   函数体不读 `qty`/`px`（`VenueFeeSchedule::effective_rate` 的 `Notional` 分支，
///   venue_fee.rs:190-200）⟹ 规模维消失；`maker_bps == taker_bps`
///   **逐位相等** ⟹ 角色维消失（★#423 收尾轮 D：角色维的消失单由这条对称性推出；同处的
///   [`PRODUCTION_LIQUIDITY_ROLE`](super::super::venue_fee::PRODUCTION_LIQUIDITY_ROLE)
///   取值在对称守卫下两支逐位同值 ⟹ **不参与判别**，其作用是角色来源单一化的纵深防御，
///   逐条论证见 `constant_effective_rate` 文档）（★#423 第二阶段 C 线起生产报价
///   [`FeeQuoter::production_rate_or_fallback`](super::super::venue_fee::FeeQuoter::production_rate_or_fallback)
///   亦不接收角色参数，与本判定同取该常量 ⟹ 两侧**同源**，不只是同值）。
///   反事实臂（随机对照在不同 px 上重执行）用**同一个**常数，不引入口径
///   不对称；逐 bar 成本分布在该档本就是常数×名义额，标量不丢分布信息。
/// - **按股标定档**（[`FeeUnit::PerShare`](super::super::venue_fee::FeeUnit::PerShare)）：
///   **无良定义**（#374 原论证，收窄到本档）。per-share 费率是 (qty, px, side) 的**非线性**
///   函数（最低佣金托底、1% 名义额上限、仅卖出监管费）⟹ 不存在一个使消费面成立的常数：
///   - 随机对照在**反事实**入场点上重执行（qty 同、px 不同）⟹ 逐笔正确费率与 Θ 实际路径的
///     费率不同；任何取自实际路径的标量（含 Σfee/Σnotional 实际有效费率）对反事实臂仍是错的，
///     把它写进"随机对照含同等成本"的断言只是把不对称藏进一个更贵的常数（090 声明膨胀）；
///   - 成本剥离要的是**逐 bar** 成本分布，标量只能保总额、保不住分布。
/// - **按金额但 maker≠taker**：**落回无良定义**。角色维不消失 ⟹ 常数不由 datum 内容唯一确定，
///   选一侧充数就是把不对称藏进常数（同上）。
///
/// 无良定义的两档**不做近似**，仍是 fail-loud 锁（panic），与同类"口径双源"问题
/// （`tax_bps` 与 datum 重复计，见 [`fee_quoter`]）**同级处置**（090 一致性）。
/// 两档共同的解锁路径 = 给这两个消费面接 (qty, px, side) 缝、改用 [`fee_quoter`] 逐笔解析（另票）
/// ——按股档解锁的是规模/方向维，按金额非对称档解锁的是角色维，缝是同一条（★#423 收尾轮 A）。
pub fn scalar_cost_rate(exec: &ExecConfig) -> f64 {
    // ★#423 收尾轮 A：本函数**看得到 `exec`** ⟹ 在共用正文后追加「本档实际落哪一支」的标签。
    //   共用正文（下面的 const）由只拿到 `Option<f64>`、看不到档的消费点使用，必须两成因并列；
    //   落支标签只有此处可解析。两段拼成同一条消息 ⟹ 单一来源不破（不存在第二份正文）。
    match scalar_cost_rate_opt(exec) {
        Some(rate) => rate,
        None => panic!(
            "{}{}",
            SCALAR_COST_RATE_UNDEFINED,
            scalar_cost_rate_undefined_cause(exec)
        ),
    }
}

/// [`scalar_cost_rate`] 的 fail-loud 消息**共用正文**（单一来源）——两条落回分支共用，
/// 消费点 `expect` 同串，防第二查法。
///
/// ★#423 收尾轮 A 改文（旧文见 git 历史）。旧文两处误用：
/// 1. 开头写「venue **标定档下**…无良定义」——分叉后「经 venue datum 标定」**不再蕴含**无良定义
///    （按金额且 maker/taker 逐位对称的 datum 档就是标定档且有良定义），该蕴含是分叉前的旧口径；
/// 2. 只写了按股档的成因（「逐笔费率随 qty/px/side 变」），而落回本支的**另一条**分支
///    （按金额非对称档）成因不同——费率与 qty/px 无关，是**角色维**不消失。
///
/// 本文两成因并列点明；「本档实际落哪一支」由 [`scalar_cost_rate`] 追加的落支标签给出
/// （`[A]` / `[B]`，见该函数）。同型措辞纪律见 `wverify_run::SCALAR_UNDEFINED_UNAVAILABLE`
/// 的「不得含标定档」断言——两串同源同口径。
pub const SCALAR_COST_RATE_UNDEFINED: &str =
    "取单标量成本费率（随机对照的「同等成本」/ 鞅守卫的成本剥离）：本档**不存在**与 \
     (qty, px, side) 无关的常数等效费率 ⟹ 无良定义。落回本支的档有两条，成因**不同**：\
     [A] 按股档（per-share）——逐笔费率是 (qty, px, side) 的非线性函数（最低佣金托底 / \
     1% 名义额上限 / 仅卖出监管费）⟹ 规模维与方向维都不消失；\
     [B] 按金额档（per-notional）但 maker≠taker——费率与 qty/px 无关，但角色维不消失 ⟹ \
     常数不由 datum 内容唯一确定，选一侧充数就是把不对称藏进常数。\
     判定**不**由「档是否经 venue datum 标定」决定：按金额且 maker/taker 逐位对称的 datum \
     档有良定义，不落本支。解锁路径 = 给这两个消费面接 (qty, px, side) 缝改用 fee_quoter \
     逐笔解析（另票）；否则策略臂按 datum 扣费、对照臂按未标定常率 ⟹ 口径不对称，读数无意义。";

/// 本 `exec` 的档**实际落哪一条**无良定义分支（★#423 收尾轮 A）——[`scalar_cost_rate`] 的
/// panic 消息后缀。良定义档不调本函数（`scalar_cost_rate_opt` 给 `Some`）。
///
/// 分支判定不在此处重做：本函数只把 [`super::super::venue_fee::VenueFeeSchedule::unit`]
/// 的形态翻译成人读标签，良定义判定本体仍是
/// [`constant_effective_rate`](super::super::venue_fee::VenueFeeSchedule::constant_effective_rate)。
fn scalar_cost_rate_undefined_cause(exec: &ExecConfig) -> &'static str {
    use super::super::venue_fee::FeeUnit;
    match exec.fee_schedule.as_ref().map(|s| &s.unit) {
        Some(FeeUnit::PerShare(_)) => {
            "｜本档落支 = [A]：计费单位为按股（per-share），逐笔费率随 qty/px/side 变。"
        }
        Some(FeeUnit::Notional { .. }) => {
            "｜本档落支 = [B]：计费单位为按金额（per-notional）但 maker_bps≠taker_bps（或非有限），\
             角色维未消失。"
        }
        // 未标定档恒 `Some(fee_rate)` ⟹ 走不到本函数；照实标注而不假装不可达（090）。
        None => "｜本档落支 = 未标定档（`fee_schedule = None`）——该档恒有良定义，到此即实现自相矛盾。",
    }
}

/// [`scalar_cost_rate`] 的**类型承载版**（#388 T2）：良定义档 `Some(常数)`；无良定义档 `None`。
///
/// 与 [`scalar_cost_rate`] 的关系 = 同一条有效域判定的两种承载方式，**防线等价不减弱**：
/// - 值层：两者对良定义档给同一个 f64（`scalar_cost_rate` 即本函数 + `expect`）；
/// - 无良定义档：本函数给 `None` —— `None` 不能被当费率参与任何算术，消费面要么显式
///   `expect(SCALAR_COST_RATE_UNDEFINED)`（保持原 fail-loud 语义，只是位点从**构造期**
///   移到**消费期**），要么按 #385 Implementation Decisions 的裁定把该读数**标注不可用**。
///
/// 为什么要移位点（#388）：`RunResult` 的构造期 panic 使**整条跑批**在标定档下不可运行，
/// 而跑批的绝大多数读数（execR/费用科目/TW 三态/门控统计）与单标量费率**无关**。构造期
/// 挡住 = 用一个消费面的有效域收窄，锁死与它无关的所有读数。移到消费期后：无关读数照常
/// 产出，相关读数（随机对照系 LCB/beats_random、l3 鞅守卫成本剥离）按裁定标"不可用"。
/// **不是**给这些消费面喂近似值——那条路 #374 已明文否决。
///
/// ## 三分叉（★#423，判定依据见 [`scalar_cost_rate`] 文档）
///
/// | 档 | 取值 |
/// |---|---|
/// | `None`（未标定） | `Some(fee_rate(exec))`，**逐位不变** |
/// | `Some`，per-notional 且 maker/taker 逐位对称 | `Some(档 bps/1e4 + slippage_bps/1e4 + tax_bps/1e4)` |
/// | `Some`，per-share **或** per-notional 非对称 | `None`，**逐位不变**（fail-loud 保持） |
///
/// 良定义判定本体 = `VenueFeeSchedule::constant_effective_rate`（结构检查：档内容的单位形态
/// 与 maker/taker 对称性 + 角色取自编译期常量），本函数只做**口径合成**，不重复判定逻辑。
///
/// **为什么式子里有一个恒为 0 的 `tax_bps` 项**：标量必须与标定档的**实际成交费率**同口径，
/// 而后者 = datum 费率 + `slippage_bps/1e4`（`FeeQuoter` 的 `uncalibrated_addon`，
/// venue_fee.rs:285-289）。`tax_bps` 在标定档下由 [`fee_quoter`] 的 assert 强制为 0
/// （treasury.rs:128，禁与 datum 税费科目双计）⟹ 该项实际恒为 0。仍写进式内的理由有二：
/// (1) 口径完备——本式与 [`fee_rate`] 的三项合成**同形**，读者不必回查哪一项被省略；
/// (2) 若将来落一份含税辖区的 datum 并放宽那条 assert，本式无须再改。**不是**声明本函数
/// 支持非零税辖区：非零 `tax_bps` + 标定档在成交点即 panic，走不到消费本标量的地方。
pub fn scalar_cost_rate_opt(exec: &ExecConfig) -> Option<f64> {
    match exec.fee_schedule.as_ref() {
        None => Some(fee_rate(exec)),
        Some(sched) => sched.constant_effective_rate().map(|datum_rate| {
            datum_rate + exec.slippage_bps / 10_000.0 + exec.tax_bps / 10_000.0
        }),
    }
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

    /// ★#374 MED-A：未标定档下单标量成本口径 = [`fee_rate`] 逐位（现状不动）。
    /// **认识论 L0**（纯定义/算术，零数据依赖）。
    #[test]
    fn scalar_cost_rate_uncalibrated_is_bit_exact_fee_rate() {
        let exec = ExecConfig::default();
        assert_eq!(scalar_cost_rate(&exec), fee_rate(&exec));
    }

    /// ★#388 T2：`scalar_cost_rate_opt` 未标定档 = [`scalar_cost_rate`] 逐位同值（`Some`）。
    /// **认识论 L0**（纯定义/契约，零数据依赖）。
    #[test]
    fn scalar_cost_rate_opt_uncalibrated_is_some_bit_exact() {
        let exec = ExecConfig::default();
        assert_eq!(scalar_cost_rate_opt(&exec), Some(fee_rate(&exec)));
    }

    /// 合成 per-notional 档（`maker_bps`/`taker_bps` 由用例指定，用于对称性分叉的两侧）。
    fn synth_notional(maker_bps: f64, taker_bps: f64) -> super::super::super::venue_fee::VenueFeeSchedule {
        use super::super::super::venue_fee::{FeeUnit, VenueFeeSchedule};
        VenueFeeSchedule {
            venue: "SYNTH".into(),
            symbol: "X".into(),
            tier: "T".into(),
            unit: FeeUnit::Notional { maker_bps, taker_bps },
            datum_sha256: "0".repeat(64),
        }
    }

    /// 合成 per-share 档（取值形态照 IBKR datum，数值不对账——本组用例只测**档形态**分叉）。
    fn synth_per_share() -> super::super::super::venue_fee::VenueFeeSchedule {
        use super::super::super::venue_fee::{FeeUnit, PerShareFees, VenueFeeSchedule};
        VenueFeeSchedule {
            venue: "SYNTH_PS".into(),
            symbol: "X".into(),
            tier: "T".into(),
            unit: FeeUnit::PerShare(PerShareFees {
                commission_per_share_usd: 0.0035,
                min_commission_usd: 0.35,
                max_commission_frac_of_notional: 0.01,
                clearing_per_share_usd: 0.0002,
                cat_per_share_usd: 0.000_003,
                sell_sec_fee_frac: 0.000_020_6,
                sell_taf_per_share_usd: 0.000_195,
                sell_taf_cap_usd: 9.79,
                passthru_exchange_frac_of_commission: 0.000_175,
                passthru_finra_frac_of_commission: 0.000_565,
            }),
            datum_sha256: "0".repeat(64),
        }
    }

    /// 挂一份 venue 费率档的 `ExecConfig`（其余字段取默认 ⟹ `slippage_bps=2`、`tax_bps=0`）。
    fn exec_with(sched: super::super::super::venue_fee::VenueFeeSchedule) -> ExecConfig {
        ExecConfig { fee_schedule: Some(sched), ..ExecConfig::default() }
    }

    /// ★#423：**按金额对称档 ⟹ `Some(常数)`**，且与 [`fee_quoter`] 在多组 (qty, px, side) 上
    /// 逐位一致——"恒定"的物证（口头声明不算，见 231号有效域规则）。
    /// **认识论 L0**（合成档 + 纯算术契约，零市场信息增量）。
    #[test]
    fn scalar_cost_rate_opt_notional_symmetric_is_constant() {
        use super::super::super::strategy::exec::FillSide;
        let exec = exec_with(synth_notional(10.0, 10.0));
        let scalar = scalar_cost_rate_opt(&exec).expect("对称 per-notional 档有良定义");
        assert_eq!(
            scalar,
            10.0 / 10_000.0 + exec.slippage_bps / 10_000.0 + exec.tax_bps / 10_000.0,
            "标量 = 档 bps + slippage + tax（tax 在标定档恒 0，形态上仍在式内）"
        );
        let q = fee_quoter(&exec);
        for qty in [1e-8, 1.0, 12_345.678, 1e9] {
            for px in [1e-6, 20.0, 50_000.0, 1e7] {
                for side in [FillSide::Buy, FillSide::Sell] {
                    assert_eq!(
                        q.production_rate_or_fallback(qty, px, side),
                        scalar,
                        "标量须与生产报价的逐笔费率逐位相同（qty={qty}, px={px}）"
                    );
                }
            }
        }
    }

    /// ★#423：**maker≠taker ⟹ `None`**（角色维不消失 ⟹ 常数不由 datum 内容唯一确定）。
    /// **认识论 L0**（合成档，纯结构检查）。
    #[test]
    fn scalar_cost_rate_opt_notional_asymmetric_is_none() {
        let exec = exec_with(synth_notional(5.0, 10.0));
        assert_eq!(scalar_cost_rate_opt(&exec), None);
    }

    /// ★#423 / ★#374 MED-A：**按股档 ⟹ `None`**（逐笔费率随 qty/px/side 变，逐位不变的旧行为）。
    /// **认识论 L0**（合成档，纯结构检查）。
    #[test]
    fn scalar_cost_rate_opt_per_share_is_none() {
        let exec = exec_with(synth_per_share());
        assert_eq!(scalar_cost_rate_opt(&exec), None);
    }

    /// ★#423：**真实 datum 两档的分叉**——Binance 现货 VIP0（per-notional，maker=taker=10bp）
    /// ⟹ `Some(1e-3 + 2e-4)`；IBKR Pro 美股（per-share）⟹ `None`。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2）。订正理由：`datum` 是真实 venue 官方费率表
    /// 快照不等于本测是 L2。按有效域规则，L2 = 「真实数据**假设检验**，可能产生否定性结果」；
    /// 本测做的是「读一份静态 JSON，断言其档位形态（Notional/PerShare）与分叉算术」——被测对象
    /// 是**装载与分叉管线**，输入不含任何可被市场否证的假设（费率表是契约值，不是估计量），
    /// 断言不可能"失败于市场"、只可能失败于代码 ⟹ 信息增量为零，等级为 L1。
    /// 同文件同型先例 = `wverify_run::fee_datum_spec_resolves_repo_datum`（读真实 datum 的装载
    /// 算术，标 L1）。
    /// **本测能否证的假设：无**（照实登记——若将来把它接到"该费率下策略是否仍有 alpha"才升 L2）。
    #[test]
    fn scalar_cost_rate_opt_real_datum_forks_by_unit() {
        use super::super::super::venue_fee::{datum_dir, load_datum};
        // ★#423 收尾轮 C：两档各构一个**不可变** `ExecConfig`（原实现 `let mut exec` + 字段赋值
        //   复用同一实例跑两档，违 coding-style「ALWAYS create new objects, NEVER mutate」；
        //   同文件已有 `exec_with(sched)` 的正确写法，照它统一）。
        let binance = load_datum(&datum_dir().join("venue_fee_binance_spot_20260726.json"))
            .expect("Binance datum 可加载");
        let exec_notional = exec_with(binance.resolve("BTC", "VIP0").expect("BTC/VIP0 在册"));
        assert_eq!(
            scalar_cost_rate_opt(&exec_notional),
            Some(1e-3 + 2e-4),
            "VIP0 对称 10bp/side + 未被 datum 覆盖的 slippage 2bp（报告 §3.3）"
        );

        let ibkr = load_datum(&datum_dir().join("venue_fee_ibkr_pro_20260726.json"))
            .expect("IBKR datum 可加载");
        let exec_per_share = exec_with(
            ibkr.resolve("OKLO", "PRO_TIERED_LE_300K_SHARES")
                .expect("OKLO 在册"),
        );
        assert_eq!(scalar_cost_rate_opt(&exec_per_share), None, "per-share 档无良定义");
    }

    /// ★#374 MED-A / ★#423：**无良定义档下 fail-loud**——不静默产一个对随机对照/成本剥离
    /// 无效的常数。与 `tax_bps` 双计（[`fee_quoter`] 的 assert）同级处置（090 一致性）。
    ///
    /// ★#423 改动理由：本测原用**对称合成 per-notional** 档，分叉后该档有良定义（返回 `Some`）
    /// ⟹ 不再 panic。故把锁 panic 的档换成 per-share（"无良定义"的正例），fail-loud 测试保留不删。
    /// **认识论 L0**（纯定义/契约，零数据依赖——故用**合成**档，不读磁盘 datum：被测的是
    /// "无良定义即挡"这条契约，与具体费率表无关，挂钩真实文件只会引入轮换连坐）。
    ///
    /// ★#423 收尾轮 A：`expected` 从共用的「无良定义」收窄到**按股档专有**的落支标签。
    /// 旧 `expected` 对两条落回分支都匹配 ⟹ 对"消息把成因说错/说串"这类误用**不可判别**
    /// （非对称档 panic 也含「无良定义」）。现在按股档只能被 `[A]` 标签匹配。
    #[test]
    #[should_panic(expected = "本档落支 = [A]：计费单位为按股（per-share）")]
    fn scalar_cost_rate_per_share_panics() {
        let exec = exec_with(synth_per_share());
        let _ = scalar_cost_rate(&exec);
    }

    /// ★#423：非对称 per-notional 档同样 fail-loud（落回分支与 per-share 同一条防线，
    /// 但**成因不同**——角色维不消失，与 qty/px 无关）。
    /// **认识论 L0**（合成档，纯结构检查）。
    ///
    /// ★#423 收尾轮 A：`expected` 收窄到**非对称档专有**的落支标签（见上一测的判别力说明）。
    #[test]
    #[should_panic(expected = "本档落支 = [B]：计费单位为按金额（per-notional）但 maker_bps≠taker_bps")]
    fn scalar_cost_rate_asymmetric_notional_panics() {
        let exec = exec_with(synth_notional(5.0, 10.0));
        let _ = scalar_cost_rate(&exec);
    }

    /// ★#423 收尾轮 A：fail-loud 消息**共用正文**的口径断言——两条成因各自点明，且**不把
    /// 「经 venue datum 标定」当成无良定义的充分条件**（按金额对称档同为 datum 档但有良定义）。
    ///
    /// 为什么要这条：共用正文由**只拿到 `Option<f64>`、看不到档**的消费点 `expect`
    /// （`runner.rs` / `l3_pi_falsify.rs` / `l3_delta_r_alpha.rs`）使用 ⟹ 它必须自含两成因；
    /// 而落支标签只在 `exec` 可见的 [`scalar_cost_rate`] 里追加。两者是**同一条消息的两段**，
    /// 不是两个常量（单一来源不破）。**认识论 L0**（措辞契约，零数据依赖）。
    #[test]
    fn scalar_cost_rate_undefined_message_names_both_causes() {
        let m = SCALAR_COST_RATE_UNDEFINED;
        assert!(m.contains("无良定义"), "共用正文须自解释判定结论：{m}");
        assert!(
            m.contains("[A]") && m.contains("per-share") && m.contains("非线性"),
            "成因 A（按股档逐笔非线性）须点明：{m}"
        );
        assert!(
            m.contains("[B]") && m.contains("maker≠taker") && m.contains("角色维不消失"),
            "成因 B（按金额非对称档角色维不消失）须点明：{m}"
        );
        assert!(
            !m.contains("标定档"),
            "不得把适用范围说成「标定档」——按金额对称档是 datum 标定档且有良定义，\
             该蕴含是 ★#423 分叉前的旧口径（与 `wverify_run::SCALAR_UNDEFINED_UNAVAILABLE` \
             的同型断言同源）：{m}"
        );
    }

    /// ★#360 单源门面：`fee_schedule = None` ⟹ quoter 对**任意** (qty, px, side)
    /// 逐位返回 [`fee_rate`] 的常率（None 不破的接口层物证）。
    ///
    /// ★#423 第二阶段 C 线：原测还枚举 `role` 维。收口后 quoter 的 `pub` 报价方法不接收角色
    /// （角色单一来源 = `venue_fee::PRODUCTION_LIQUIDITY_ROLE`）⟹ 该维在本层不可表达，
    /// 枚举随之删除——**不是**覆盖被削弱，是被枚举的参数不存在了。
    #[test]
    fn fee_quoter_none_is_bit_exact_fallback() {
        use super::super::super::strategy::exec::FillSide;
        let exec = ExecConfig::default();
        let q = fee_quoter(&exec);
        assert!(!q.is_calibrated());
        assert_eq!(q.fallback_rate(), fee_rate(&exec));
        for qty in [1e-8, 1.0, 1e6] {
            for px in [1e-6, 20.0, 120_000.0] {
                for side in [FillSide::Buy, FillSide::Sell] {
                    assert_eq!(q.production_rate_or_fallback(qty, px, side), 3e-4);
                }
            }
        }
    }

    /// ★#360：`Some(datum)` ⟹ quoter 走 venue 档（Binance 现货 VIP0 = 10bp/side，报告 §2.1）。
    #[test]
    fn fee_quoter_some_resolves_venue_datum() {
        use super::super::super::strategy::exec::FillSide;
        use super::super::super::venue_fee::{datum_dir, load_datum};
        let book = load_datum(&datum_dir().join("venue_fee_binance_spot_20260726.json"))
            .expect("Binance datum 可加载");
        let mut exec = ExecConfig::default();
        exec.fee_schedule = Some(book.resolve("BTC", "VIP0").expect("BTC/VIP0 在册"));
        let q = fee_quoter(&exec);
        assert!(q.is_calibrated());
        assert_eq!(q.datum_sha256(), Some(book.datum_sha256.as_str()));
        assert_eq!(
            q.production_rate_or_fallback(1.0, 50_000.0, FillSide::Buy),
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
