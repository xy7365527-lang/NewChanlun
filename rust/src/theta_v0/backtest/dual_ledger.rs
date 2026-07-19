//! 双账本（hedge-mode 分腿头寸簿，关⑤方案 B）——M14 `P^sep=∏_{v}(R≥0 e⁺_v ⊕ R≥0 e⁻_v)`
//! 的账户层兑现。
//!
//! 施工图：`chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md` §4。
//!
//! ## 核心语义（与现行净额账本的关系）
//!
//! - **分腿不先净额**（M14/M30）：多腿 `q_long≥0` 与空腿 `q_short≥0` 是两个独立坐标，
//!   开空**不触多腿**（M13 父仓保持——净额账户「先平后开」（runner.rs:3110-3112）使多空双腿
//!   物理上不可共存，bsp_consumption_redesign §9.1 裁定的「净额翻转是空头不赚根因」即此）。
//! - **realized 只出平仓腿**（A' 结算源保留，runner.rs:3178-3184 口径按腿延伸）：开仓腿不产
//!   PnL；平仓腿产 `(平仓净价 − 腿成本基) × 平量`（多/空镜像对称，与 `apply_fill` 逐字同构）。
//! - **bit-exact 嵌入恒等**（§4.4）：现行净语义是双账本子集——对任一固定订单流存在腿标记
//!   方案（[`compatible_leg_orders`]）使两账本 (cash, equity, realized, fee, trade_pnls) 逐笔
//!   相同（单测 D8 逐字节对拍锁）。真正分叉只出现在嵌套声部启用后（新订单形态「持多腿时
//!   开空腿」在旧账本不存在对应物）——那是本关的**新能力**，不是旧数字漂移。
//!
//! ## 诚实有效域（施工图 §4.1/§7）
//!
//! - `q_short` 是**真空头**（期货/永续域）；现货标的的「机动份额暂驻」读法须部署层 gate=0
//!   （引擎不区分标的，标的无特化纪律）。
//! - 做空保证金/借券成本未建模（沿用 runner.rs:3119-3121 声明）；双腿共存时 \|net\| 基
//!   accrual 是近似口径（per-leg 精化列遗留 L6，涉成本数字变更须独立裁定）。
//! - 估值取净投影 `equity = cash + (q⁺−q⁻)·px`（M30：两腿浮盈亏在净值上抵消；分账本腿级
//!   收益 G^sep>0（M24）≠ 净额账户收益命题）。
//!
//! 认识论 L0（账本结构恒等）/L1（管线串通）——不声称 L2 alpha。

use super::super::strategy::voice::VoiceSide;
use super::super::strategy::LegOrder;
use super::super::types::{Order, StrictAction};

/// 双账本（hedge-mode 分腿头寸簿，M14 `P^sep` 的账户层坐标）。
///
/// 字段（M14 `P^sep=∏(R≥0 e⁺ ⊕ R≥0 e⁻)`）：
/// - `cash`：现金（净额账本同一坐标，两账本现金流公式同形）。
/// - `q_long`：多腿手数（≥0，e⁺ 坐标）。
/// - `q_short`：空腿手数（≥0，e⁻ 坐标）。
/// - `cost_long`：多腿每单位含费成本基（买入均价含买入费，同 `apply_fill` 多侧口径）。
/// - `cost_short`：空腿每单位含费成本基（卖出均价扣卖出费净收，同 `apply_fill` 空侧口径）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DualLedger {
    pub cash: f64,
    pub q_long: f64,
    pub q_short: f64,
    pub cost_long: f64,
    pub cost_short: f64,
}

impl DualLedger {
    /// 以初始现金建账（双腿空仓，成本基 0）。
    pub fn new(cash: f64) -> Self {
        DualLedger { cash, q_long: 0.0, q_short: 0.0, cost_long: 0.0, cost_short: 0.0 }
    }

    /// 净额投影 `Net(p)=Σ(q⁺−q⁻)`（M14 另行定义的净额映射；正=净多，负=净空）。
    pub fn net_units(&self) -> f64 {
        self.q_long - self.q_short
    }

    /// 毛敞口 `G=q⁺+q⁻`（strict §11 G_t；双开时 N=0 但 G=2k——净约束不能替代毛约束）。
    pub fn gross_units(&self) -> f64 {
        self.q_long + self.q_short
    }

    /// 估值（净投影，M30 有效域声明）：`equity = cash + Net·px`。
    pub fn equity(&self, px: f64) -> f64 {
        self.cash + self.net_units() * px
    }
}

/// 单次腿订单的真实成交分解（[`apply_fill_dual`] 返回，A' 结算源按腿延伸）。
///
/// `closed_long+closed_short+opened_long+opened_short = executed_qty`；未成交余量只进
/// `rejected_qty`（close_only 语义：平仓超腿持仓的余量被拒，**不借机开反向**）。
/// `realized` 只在平仓腿产（TW Realize 唯一合法资金源的结算边界不变）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillOutcomeDual {
    pub requested_qty: f64,
    pub executed_qty: f64,
    pub closed_long: f64,
    pub closed_short: f64,
    pub opened_long: f64,
    pub opened_short: f64,
    pub rejected_qty: f64,
    /// 本次 fill 的费后已实现 PnL（仅平仓腿分量产生；无平仓 = 0.0）。
    pub realized: f64,
    /// 本次成交实付费用 = 成交名义 × fee_rate（独立测得，RDecomposition 守恒断言口径）。
    pub fee: f64,
}

impl FillOutcomeDual {
    pub fn noop() -> Self {
        FillOutcomeDual {
            requested_qty: 0.0,
            executed_qty: 0.0,
            closed_long: 0.0,
            closed_short: 0.0,
            opened_long: 0.0,
            opened_short: 0.0,
            rejected_qty: 0.0,
            realized: 0.0,
            fee: 0.0,
        }
    }

    /// 平仓总量（两腿和；runner `apply_voice_fill` 双账版消费）。
    pub fn closed_qty(&self) -> f64 {
        self.closed_long + self.closed_short
    }

    /// 开仓总量（两腿和）。
    pub fn opened_qty(&self) -> f64 {
        self.opened_long + self.opened_short
    }
}

/// **分腿成交**（关⑤ §4.3，不先净额）：[`LegOrder`]（腿标记 + 开/平）× 绝对手数。
///
/// 规则表（施工图 §4.3，PnL/现金流/成本基公式与 `apply_fill`（runner.rs:3167）逐字同构）：
///
/// | 订单 | 腿变化 | 现金 | 成本基 | realized |
/// |---|---|---|---|---|
/// | open Long qty | q_long += qty | cash −= qty·px·(1+fee)（不足 ⟹ rejected） | 加权平均 | 0 |
/// | open Short qty | q_short += qty（**q_long 不动**，M13 父仓保持） | cash += qty·px·(1−fee) | 加权平均 | 0 |
/// | close Long qty | q_long −= min(qty,q_long)（余量 rejected，不借机开反向） | cash += 平量·px·(1−fee) | 全平 ⟹ 0 | (px(1−fee)−cost_long)·平量 |
/// | close Short qty | q_short −= min(qty,q_short) | cash −= 平量·px·(1+fee) | 全平 ⟹ 0 | (cost_short−px(1+fee))·平量 |
///
/// 费用口径：fee = 成交名义 × fee_rate 逐段累计（同 runner.rs:3209/:3252）。
pub fn apply_fill_dual(
    lo: &LegOrder,
    px: f64,
    fee_rate: f64,
    ledger: &mut DualLedger,
    trade_pnls: &mut Vec<f64>,
) -> FillOutcomeDual {
    let qty = lo.order.qty as f64;
    let mut out = FillOutcomeDual { requested_qty: qty.max(0.0), ..FillOutcomeDual::noop() };
    if qty <= 0.0 || px <= 0.0 {
        out.rejected_qty = qty.max(0.0);
        return out;
    }
    match (lo.leg, lo.close) {
        // ── 开多腿（现金买入；不足拒绝，同现行 need_cash，runner.rs:3239-3251）。 ──
        (VoiceSide::Long, false) => {
            let cost = qty * px * (1.0 + fee_rate);
            if ledger.cash < cost {
                out.rejected_qty = qty;
                return out;
            }
            out.fee += qty * px * fee_rate;
            let unit_cost = px * (1.0 + fee_rate); // 含买入费成本基（同 :3255 delta=+1）
            let new_q = ledger.q_long + qty;
            // 加权平均含费成本基（同 :3260-3261 式按腿拆分）。
            ledger.cost_long = (ledger.cost_long * ledger.q_long + unit_cost * qty) / new_q;
            ledger.q_long = new_q;
            ledger.cash += -qty * px * (1.0 + fee_rate); // 同 :3237 delta=+1
            out.opened_long = qty;
            out.executed_qty = qty;
        }
        // ── 开空腿（卖出净收，无预付——保证金未建模声明沿用 runner.rs:3119-3121）。
        //    **q_long 不动**（M13 父仓保持，不先净额——本账本与净额路径的本质分叉点）。 ──
        (VoiceSide::Short, false) => {
            out.fee += qty * px * fee_rate;
            let unit_cost = px * (1.0 - fee_rate); // 卖出净收成本基（同 :3255 delta=−1）
            let new_q = ledger.q_short + qty;
            ledger.cost_short = (ledger.cost_short * ledger.q_short + unit_cost * qty) / new_q;
            ledger.q_short = new_q;
            ledger.cash += qty * px * (1.0 - fee_rate); // 同 :3237 delta=−1
            out.opened_short = qty;
            out.executed_qty = qty;
        }
        // ── 平多腿（clamp 到腿持仓；余量 rejected——close_only 不借机开反向，:3135-3136）。 ──
        (VoiceSide::Long, true) => {
            let close_qty = qty.min(ledger.q_long);
            if close_qty <= 0.0 {
                out.rejected_qty = qty;
                return out;
            }
            ledger.cash += close_qty * px * (1.0 - fee_rate); // 同 :3208 delta=−1
            out.fee += close_qty * px * fee_rate;
            // 平多 PnL = proceeds(扣卖出费) − 成本基(含买入费)（:3211 多侧式）。
            let pnl = (px * (1.0 - fee_rate) - ledger.cost_long) * close_qty;
            ledger.q_long -= close_qty;
            if ledger.q_long == 0.0 {
                ledger.cost_long = 0.0; // 全平 ⟹ 成本基归零（同 :3226-3228）
            }
            trade_pnls.push(pnl);
            out.realized = pnl; // A'：平仓结算事实（TW Realize 唯一合法资金源）
            out.closed_long = close_qty;
            out.executed_qty = close_qty;
            out.rejected_qty = qty - close_qty;
        }
        // ── 平空腿（镜像对称）。 ──
        (VoiceSide::Short, true) => {
            let close_qty = qty.min(ledger.q_short);
            if close_qty <= 0.0 {
                out.rejected_qty = qty;
                return out;
            }
            ledger.cash += -close_qty * px * (1.0 + fee_rate); // 同 :3208 delta=+1
            out.fee += close_qty * px * fee_rate;
            // 平空 PnL = 开空成本基(扣卖出费净收) − 平空支出(含买入费)（:3212 空侧式）。
            let pnl = (ledger.cost_short - px * (1.0 + fee_rate)) * close_qty;
            ledger.q_short -= close_qty;
            if ledger.q_short == 0.0 {
                ledger.cost_short = 0.0;
            }
            trade_pnls.push(pnl);
            out.realized = pnl;
            out.closed_short = close_qty;
            out.executed_qty = close_qty;
            out.rejected_qty = qty - close_qty;
        }
        // Flat 腿非法（调用方保证非 Flat；显式拒绝不静默）。
        (VoiceSide::Flat, _) => {
            out.rejected_qty = qty;
        }
    }
    out
}

/// **兼容腿标记**（§4.4-3 嵌入恒等的构造性内容）：把现行净语义订单（[`StrictAction`] +
/// 当前有符号 `units`）翻译为等效 [`LegOrder`] 序列——「先平反向腿，余量开新腿」与
/// `apply_fill` 两段（runner.rs:3202/:3232）**同序同式**：
///
/// - `Sell qty while units=+N` ⟺ `close Long min(qty,N)` + `open Short (qty−min(qty,N))`；
/// - `Buy qty while units=−N` ⟺ `close Short min(qty,N)` + `open Long 余量`；
/// - `Close/Reduce` ⟺ 平当前腿（超腿余量由 [`apply_fill_dual`] clamp+reject，同 close_only）。
///
/// 此翻译下不变量 `q_long−q_short == units` 逐笔保持，且段1 平仓腿 realized == 现行段1 公式
/// （:3210-3216 逐字），段2 开仓腿不产 realized（:3181 同）——D8 逐字节对拍的对象。
pub fn compatible_leg_orders(o: &Order, units: f64) -> Vec<LegOrder> {
    let qty = o.qty.max(0) as f64;
    let mut out = Vec::new();
    if qty <= 0.0 {
        return out;
    }
    let mk = |action: StrictAction, q: f64, leg: VoiceSide, close: bool| LegOrder {
        order: Order { action, qty: q as i64, exec_index: o.exec_index },
        leg,
        close,
    };
    match o.action {
        StrictAction::Buy | StrictAction::Add => {
            if units < 0.0 {
                let c = qty.min(-units);
                out.push(mk(StrictAction::Close, c, VoiceSide::Short, true));
                if qty - c > 0.0 {
                    out.push(mk(o.action, qty - c, VoiceSide::Long, false));
                }
            } else {
                out.push(mk(o.action, qty, VoiceSide::Long, false));
            }
        }
        StrictAction::Sell => {
            if units > 0.0 {
                let c = qty.min(units);
                out.push(mk(StrictAction::Close, c, VoiceSide::Long, true));
                if qty - c > 0.0 {
                    out.push(mk(o.action, qty - c, VoiceSide::Short, false));
                }
            } else {
                out.push(mk(o.action, qty, VoiceSide::Short, false));
            }
        }
        StrictAction::Reduce | StrictAction::Close => {
            if units > 0.0 {
                out.push(mk(o.action, qty, VoiceSide::Long, true));
            } else if units < 0.0 {
                out.push(mk(o.action, qty, VoiceSide::Short, true));
            }
            // 空仓无仓可平（apply_order 的 noop 分支对应——不产腿订单）。
        }
        StrictAction::Hold | StrictAction::Wait => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::runner;

    const FEE: f64 = 0.0003;

    fn lo(leg: VoiceSide, close: bool, qty: i64) -> LegOrder {
        let action = if close {
            StrictAction::Close
        } else {
            match leg {
                VoiceSide::Long => StrictAction::Buy,
                VoiceSide::Short => StrictAction::Sell,
                VoiceSide::Flat => StrictAction::Wait,
            }
        };
        LegOrder { order: Order { action, qty, exec_index: 0 }, leg, close }
    }

    fn ledger_with(cash: f64) -> DualLedger {
        DualLedger::new(cash)
    }

    /// D1（不先净额锁 / M13 父仓保持）：q_long=10 时 open Short 6 ⟹ q_long 不变、q_short=6、
    /// cash 增量 = 6·px·(1−fee)、equity = cash + (10−6)·px。
    #[test]
    fn dual_open_short_keeps_long_leg() {
        let mut l = ledger_with(1_000_000.0);
        let mut pnls = Vec::new();
        let px = 100.0;
        apply_fill_dual(&lo(VoiceSide::Long, false, 10), px, FEE, &mut l, &mut pnls);
        let cash_after_long = l.cash;
        let f = apply_fill_dual(&lo(VoiceSide::Short, false, 6), px, FEE, &mut l, &mut pnls);
        assert_eq!(f.opened_short, 6.0);
        assert_eq!(l.q_long, 10.0, "开空不触多腿（M13 父仓保持，不先净额）");
        assert_eq!(l.q_short, 6.0);
        // cash 增量 = 空腿卖出净收 6·px·(1−fee)。
        let expect = cash_after_long + 6.0 * px * (1.0 - FEE);
        assert_eq!(l.cash, expect);
        // equity 取净投影：cash + (10−6)·px。
        assert_eq!(l.equity(px), l.cash + (10.0 - 6.0) * px);
        assert!(pnls.is_empty(), "开仓腿不产 realized（A'）");
        assert_eq!(f.realized, 0.0);
    }

    /// D2（M14 坐标非零 / Net=0）：q_long=q_short=N ⟹ net==0、gross==2N、
    /// cash == nav − N·px(1+fee) + N·px(1−fee)。
    #[test]
    fn dual_coexist_net_zero_gross_nonzero() {
        let nav = 1_000_000.0;
        let mut l = ledger_with(nav);
        let mut pnls = Vec::new();
        let px = 100.0;
        let n = 10.0;
        apply_fill_dual(&lo(VoiceSide::Long, false, n as i64), px, FEE, &mut l, &mut pnls);
        apply_fill_dual(&lo(VoiceSide::Short, false, n as i64), px, FEE, &mut l, &mut pnls);
        assert_eq!(l.net_units(), 0.0, "双开共存净额为零");
        assert_eq!(l.gross_units(), 2.0 * n, "毛敞口=两腿和（净约束平凡满足但毛敞口真实化）");
        let expect = nav + (-n * px * (1.0 + FEE)) + (n * px * (1.0 - FEE));
        assert_eq!(l.cash, expect);
        // 净投影估值：equity == cash（net=0），两腿浮盈亏在净值上抵消（M30）。
        assert_eq!(l.equity(px), l.cash);
    }

    /// D3：两段加权成本基后 close Long ⟹ realized == (px·(1−fee) − cost_long)·平量
    /// （与 apply_fill 多侧公式逐字同构）；未全平 ⟹ 成本基不变。
    #[test]
    fn dual_close_long_realizes_per_leg() {
        let mut l = ledger_with(1_000_000.0);
        let mut pnls = Vec::new();
        apply_fill_dual(&lo(VoiceSide::Long, false, 4), 100.0, FEE, &mut l, &mut pnls);
        apply_fill_dual(&lo(VoiceSide::Long, false, 4), 110.0, FEE, &mut l, &mut pnls);
        // 加权成本基 = (100(1+f)·4 + 110(1+f)·4)/8 = 105(1+f)。
        let cost = (100.0 * (1.0 + FEE) * 4.0 + 110.0 * (1.0 + FEE) * 4.0) / 8.0;
        assert_eq!(l.cost_long, cost);
        let px = 120.0;
        let f = apply_fill_dual(&lo(VoiceSide::Long, true, 6), px, FEE, &mut l, &mut pnls);
        let expect_pnl = (px * (1.0 - FEE) - cost) * 6.0;
        assert_eq!(f.realized, expect_pnl);
        assert_eq!(pnls.len(), 1);
        assert_eq!(pnls[0], expect_pnl);
        assert_eq!(l.q_long, 2.0);
        assert_eq!(l.cost_long, cost, "未全平 ⟹ 剩余腿成本基不变（同价同费基）");
        assert_eq!(f.rejected_qty, 0.0);
    }

    /// D4：镜像空侧 realized == (cost_short − px·(1+fee))·平量。
    #[test]
    fn dual_close_short_realizes_per_leg() {
        let mut l = ledger_with(1_000_000.0);
        let mut pnls = Vec::new();
        apply_fill_dual(&lo(VoiceSide::Short, false, 4), 100.0, FEE, &mut l, &mut pnls);
        apply_fill_dual(&lo(VoiceSide::Short, false, 4), 90.0, FEE, &mut l, &mut pnls);
        let cost = (100.0 * (1.0 - FEE) * 4.0 + 90.0 * (1.0 - FEE) * 4.0) / 8.0;
        assert_eq!(l.cost_short, cost);
        let px = 80.0;
        let f = apply_fill_dual(&lo(VoiceSide::Short, true, 6), px, FEE, &mut l, &mut pnls);
        let expect_pnl = (cost - px * (1.0 + FEE)) * 6.0;
        assert_eq!(f.realized, expect_pnl);
        assert_eq!(pnls[0], expect_pnl);
        assert_eq!(l.q_short, 2.0);
        assert_eq!(l.cost_short, cost);
    }

    /// D5（close_only 保留）：close qty > 腿持仓 ⟹ 平到 0、余量 rejected、不开反向。
    #[test]
    fn dual_close_clamp_no_reverse() {
        let mut l = ledger_with(1_000_000.0);
        let mut pnls = Vec::new();
        apply_fill_dual(&lo(VoiceSide::Long, false, 5), 100.0, FEE, &mut l, &mut pnls);
        let f = apply_fill_dual(&lo(VoiceSide::Long, true, 10), 110.0, FEE, &mut l, &mut pnls);
        assert_eq!(l.q_long, 0.0, "平到 0");
        assert_eq!(f.executed_qty, 5.0);
        assert_eq!(f.rejected_qty, 5.0, "超腿余量被拒（close_only）");
        assert_eq!(l.q_short, 0.0, "不借机开反向仓");
        assert_eq!(l.cost_long, 0.0, "全平 ⟹ 成本基归零");
        // 空腿侧同样 clamp：空腿上空平 ⟹ 全拒。
        let f2 = apply_fill_dual(&lo(VoiceSide::Short, true, 3), 110.0, FEE, &mut l, &mut pnls);
        assert_eq!(f2.executed_qty, 0.0);
        assert_eq!(f2.rejected_qty, 3.0);
    }

    /// D6：现金不足 ⟹ open Long rejected（need_cash 同现行 runner.rs:3239-3251）。
    #[test]
    fn dual_cash_constraint_long_open() {
        let mut l = ledger_with(100.0);
        let mut pnls = Vec::new();
        let f = apply_fill_dual(&lo(VoiceSide::Long, false, 10), 100.0, FEE, &mut l, &mut pnls);
        assert_eq!(f.opened_long, 0.0);
        assert_eq!(f.rejected_qty, 10.0);
        assert_eq!(l.q_long, 0.0);
        assert_eq!(l.cash, 100.0, "拒绝 ⟹ 现金不动");
        assert_eq!(f.fee, 0.0, "拒绝段不计费（同 :3241 提前返回口径）");
        // 开空无预付（保证金未建模声明）：现金不足仍成交。
        let f2 = apply_fill_dual(&lo(VoiceSide::Short, false, 10), 100.0, FEE, &mut l, &mut pnls);
        assert_eq!(f2.opened_short, 10.0);
    }

    /// D7（PDF §11 线性对账）：双腿共存下 equity == cash + net·px——
    /// Q⁺ΔP − Q⁻ΔP = (Q⁺−Q⁻)ΔP（overlay_state.rs:22 已锚的恒等式）。
    #[test]
    fn dual_equity_linearity_coexisting() {
        let mut l = ledger_with(1_000_000.0);
        let mut pnls = Vec::new();
        apply_fill_dual(&lo(VoiceSide::Long, false, 10), 100.0, FEE, &mut l, &mut pnls);
        apply_fill_dual(&lo(VoiceSide::Short, false, 6), 105.0, FEE, &mut l, &mut pnls);
        for px in [95.0, 100.0, 110.0] {
            assert_eq!(l.equity(px), l.cash + l.net_units() * px);
        }
        // 线性：Δequity = net·Δpx（毛敞口不进净值）。
        let d = l.equity(110.0) - l.equity(100.0);
        assert_eq!(d, l.net_units() * 10.0);
    }

    /// D8（**核心回归锁**，§4.4 嵌入恒等逐字节锁）：脚本化净语义订单流
    /// （Buy/Add/Sell/Reduce/Close 序列，含翻转/减仓/clamp）经 [`compatible_leg_orders`]
    /// 重放 vs 现行 `apply_order`/`apply_fill`——cash/equity/realized/fee/trade_pnls/成本基
    /// **逐字节相等**（四恒等逐项断言）。
    #[test]
    fn dual_bitexact_embedding_vs_apply_fill() {
        // 净语义订单流（确定性脚本，非随机——v3 禁概率推断）。
        let script: Vec<(StrictAction, i64, f64)> = vec![
            (StrictAction::Buy, 10, 100.0),   // 开多 10
            (StrictAction::Add, 6, 105.0),    // 加多 6 ⟹ 16
            (StrictAction::Sell, 20, 112.0),  // 平多 16 + 开空 4（翻转）
            (StrictAction::Sell, 6, 108.0),   // 加空 6 ⟹ −10
            (StrictAction::Buy, 15, 95.0),    // 平空 10 + 开多 5（翻转）
            (StrictAction::Reduce, 3, 98.0),  // 减多 3 ⟹ +2
            (StrictAction::Close, 5, 101.0),  // 平多 2（clamp）+ 余量 3 拒
            (StrictAction::Sell, 4, 101.0),   // 开空 4
            (StrictAction::Close, 4, 90.0),   // 平空 4
            (StrictAction::Buy, 2, 90.0),     // 开多 2（收尾持仓）
        ];
        // 净账本（现行路径）。
        let mut cash = 1_000_000.0;
        let mut units = 0.0;
        let mut entry_cost = 0.0;
        let mut net_pnls: Vec<f64> = Vec::new();
        let mut net_realized = 0.0;
        let mut net_fee = 0.0;
        // 双账本（兼容腿标记重放）。
        let mut dual = DualLedger::new(1_000_000.0);
        let mut dual_pnls: Vec<f64> = Vec::new();
        let mut dual_realized = 0.0;
        let mut dual_fee = 0.0;

        for (action, qty, px) in script {
            let o = Order { action, qty, exec_index: 0 };
            // 先取兼容腿标记（读取本订单成交**前**的有符号 units，与 apply_order 内部同前态），
            // 再两边各自成交（净侧 apply_order 先平后开两段；双侧按腿序列逐腿成交，同序）。
            let leg_orders = compatible_leg_orders(&o, units);
            let nf = runner::apply_order(
                &o, px, FEE, &mut cash, &mut units, &mut entry_cost, &mut net_pnls,
            );
            net_realized += nf.realized;
            net_fee += nf.fee;
            let mut df_exec = 0.0;
            let mut df_rejected = 0.0;
            for leg_o in &leg_orders {
                let df = apply_fill_dual(leg_o, px, FEE, &mut dual, &mut dual_pnls);
                dual_realized += df.realized;
                dual_fee += df.fee;
                df_exec += df.executed_qty;
                df_rejected += df.rejected_qty;
            }
            // 恒等 1（现金流逐笔恒等）：cash 逐字节相同。
            assert_eq!(dual.cash.to_bits(), cash.to_bits(), "cash 恒等 @({action:?},{qty},{px})");
            // 恒等 2（估值恒等）：q⁺−q⁻ == units 逐笔保持 ⟹ equity 逐字节相同。
            assert_eq!(dual.net_units().to_bits(), units.to_bits(), "Net==units @({action:?},{qty},{px})");
            assert_eq!(
                dual.equity(px).to_bits(),
                (cash + units * px).to_bits(),
                "equity 恒等 @({action:?},{qty},{px})"
            );
            // 恒等 3（realized 恒等）：本笔 realized（平仓腿和）逐字节相同。
            assert_eq!(dual_realized.to_bits(), net_realized.to_bits(), "realized 累计恒等");
            // 恒等 4（费用恒等）：分腿累计 == 两段合计。
            assert_eq!(dual_fee.to_bits(), net_fee.to_bits(), "fee 累计恒等");
            // 成本基恒等：在仓方向的成本基逐字节相同（空仓则归零相同）。
            let dual_cost = if units > 0.0 {
                dual.cost_long
            } else if units < 0.0 {
                dual.cost_short
            } else {
                0.0
            };
            assert_eq!(dual_cost.to_bits(), entry_cost.to_bits(), "成本基恒等 @({action:?},{qty},{px})");
            // 成交/拒绝量恒等（clamp+reject 口径一致）。
            assert_eq!(df_exec, nf.executed_qty, "executed 恒等");
            assert_eq!(df_rejected, nf.rejected_qty, "rejected 恒等");
            // trade_pnls 逐笔逐字节相同。
            assert_eq!(dual_pnls.len(), net_pnls.len(), "trade_pnls 笔数恒等");
            for (a, b) in dual_pnls.iter().zip(net_pnls.iter()) {
                assert_eq!(a.to_bits(), b.to_bits(), "trade_pnls 逐笔恒等");
            }
            // 兼容标记下每步至多一腿非零（无真双开——净语义子集性质）。
            assert!(
                dual.q_long == 0.0 || dual.q_short == 0.0,
                "兼容腿标记 ⟹ 两腿不同时非零（净语义子集）"
            );
        }
        // 收尾：双账本含现行净语义为子集——全序列累计无漂移。
        assert_eq!(dual_pnls.len(), net_pnls.len());
        assert_eq!(dual_realized.to_bits(), net_realized.to_bits());
    }
}
