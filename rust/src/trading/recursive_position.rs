//! RecursivePosition — 递归建仓：量 = 确认深度的递增函数（2026-06-11 任务）。
//!
//! 存在论（concurrent_fugue_deep_think Part II 的仓位投影）：级别不是事前
//! 知道的，是随结构涌现被追溯确认的。仓位管理反映这个过程——confirmed
//! 买点出现时你还不知道它最终是什么级别的转折，先按**该事件所在级别**的
//! 量入场；结构发展、更高级别确认到来，再按更高级别的量追加。每次新的
//! 级别确认触发一次递归追加。卖侧对称：低级别卖点小量减仓，高级别确认
//! （趋势结束）大量减仓，最终各级别独立流干。
//!
//! ## 形式规则
//! - 每级别 k 一个 slice（Part II §11.1 token 池；38课"分区卷钱"）：
//!   `quota_k = base_frac × weight(k)`，weight 随级别递增（26课"级别的
//!   意义基本只和买卖量有关"——日线量 ≫ 1分钟量）。
//! - confirmed 买点事件@k → slice k **填至** quota_k（饱和：受剩余现金
//!   约束，已满 = no-op）。confirmed 卖点事件@k → slice k **清至** 0
//!   （空 slice = no-op）。六类买卖点全消费（26课"任何的买点都是买点，
//!   唯一需要控制的只是量"）。
//! - 总量守恒：现金不可为负（饱和算术）⇒ Σ 投入 ≤ 初始资金，免证。
//! - 无控制状态：无 FSM、无相位、无配对槽。唯一状态 = per-slice 持仓
//!   （资源状态，Part II §11 范畴二分）。填/清是幂等操作 ⇒ candidate→
//!   confirmed 重发、seg_idx 修订重发自然无害（无需事件去重集）。
//! - master/voice 不再是两种角色：最高级别 slice 与最低级别 slice 是
//!   同一条规则在不同 k 上的实例，区别只剩 quota（40课"每一层次的操作
//!   都是独立又在一个整体的操作中"）。
//! - 各级别卖点独立清仓 = deep_think §3.3 选项(ii)（27课区间套定理：
//!   大级别转折结构上必然伴随各低级别背驰段→各级别卖点各自到来），
//!   无 master 强制清仓通道。
//!
//! ## 与 OrganicLedger 的关系（范畴区分，不共享账本）
//! OrganicLedger 是"满仓入场 + 短差腿降 cost_basis"模型——cost_basis 是
//! 全仓位的共享标量。RecursivePosition 是"从零建仓"模型：不存在全局
//! cost_basis（满仓模型的概念在此无定义域），成本即各 slice 的持仓均价，
//! 权益 = cash + Σ shares_k × price 直接可读。两账本不可混表。
//!
//! ## 已知近似（诚实标注）
//! - confirmed-only 消费：candidate 不消费（修订语义契约未设计——
//!   Part II §17 开放问题1）。等待型延迟风险已由四案否证先例标注；
//!   首版取保守侧，延迟代价在回测中可观测。
//! - 同 bar 多事件按磁带序（ladder 升序、层内事件序）确定性执行
//!   （Part II §17 开放问题4 的一个显式选择）。
//! - 买点事件重发若发生在该 slice 被卖点清空之后，会重新填仓——陈旧
//!   买点被当作新信号。fills 日志可审计。

use super::tape::SignalTape;
use super::types::{BspEvent, INITIAL_CAPITAL, FIRST_BSP_LADDER, MAX_LADDER};
use crate::buysellpoint::Side;

/// 级别权重函数。Exp2 = 2^(k−floor)（级别时间尺度几何递增的镜像，主变体）；
/// Linear = k−floor+1（消融轴：权重递增的陡峭度因果隔离）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightFn {
    Exp2,
    Linear,
}

impl WeightFn {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "exp2" => Some(WeightFn::Exp2),
            "linear" => Some(WeightFn::Linear),
            _ => None,
        }
    }

    /// weight(k)，k ≥ floor。
    pub fn weight(self, ladder: usize, floor: usize) -> f64 {
        let d = (ladder - floor) as u32;
        match self {
            WeightFn::Exp2 => (1u64 << d) as f64,
            WeightFn::Linear => (d + 1) as f64,
        }
    }
}

/// 单次成交记录（事件级，非 trade 级——递归建仓没有 entry→exit 大 trade 概念）。
#[derive(Debug, Clone)]
pub struct FillRec {
    pub bar: i64,
    pub ladder: u8,
    /// 0 = buy（填），1 = sell（清）。
    pub side: u8,
    pub shares: f64,
    pub price: f64,
    pub cash_after: f64,
    /// 成交后总权益（cash + Σ shares×price）。
    pub equity_after: f64,
}

#[derive(Debug, Default)]
pub struct RecursiveResult {
    pub fills: Vec<FillRec>,
    /// (最终权益 − 初始资金) / 初始资金 × 100。
    pub final_return_pct: f64,
    /// 权益曲线峰值最大回撤（%，正数）。
    pub max_dd_pct: f64,
    /// 时间均值 invested/equity（暴露率）。
    pub avg_invested_frac: f64,
    /// 期末 invested/equity。
    pub final_invested_frac: f64,
    /// Σ|成交额| / 初始资金（换手率）。
    pub turnover: f64,
    // ── per-ladder 可观测面 ──
    pub buy_fills: [u64; MAX_LADDER],
    /// slice 已满（quota 达成）的买点 no-op——"二买对未卖出者无事可做"。
    pub buy_noops: [u64; MAX_LADDER],
    /// 现金不足、目标量被截断的买入（截断仍计 buy_fills）。
    pub buy_starved: [u64; MAX_LADDER],
    pub sell_clears: [u64; MAX_LADDER],
    /// 空 slice 卖点 no-op。
    pub sell_noops: [u64; MAX_LADDER],
}

/// 持仓状态：纯资源状态（Part II §11），无控制状态。
struct Slices {
    shares: [f64; MAX_LADDER],
    cash: f64,
}

impl Slices {
    fn invested(&self, price: f64) -> f64 {
        self.shares.iter().map(|s| s * price).sum()
    }

    fn equity(&self, price: f64) -> f64 {
        self.cash + self.invested(price)
    }
}

/// 主入口。base_frac = 最低承载层（floor）的 quota 占初始资金比例；
/// quota_k = base_frac × weight(k)。归一化责任在调用方（变体表给定
/// base_frac，使预期涌现塔的 Σ quota ≈ 1；超出部分由现金饱和截断，
/// 不足部分闲置——两侧都可观测）。
///
/// sell_t1_only（首验否证后的探索消融轴，post-hoc 标注）：卖侧只消费
/// confirmed Sell1（type1 卖 = 背驰趋势终结，确认撤销的范畴匹配形式）；
/// Sell2/Sell3 是结构内确认/中枢离开点，在 38课语境中是短差点而非离场点
/// ——全清语义把每个卖点都当趋势终结，暴露被持续打断（首验死因候选1）。
pub fn run_recursive(
    tape: &SignalTape,
    floor_ladder: usize,
    base_frac: f64,
    weight: WeightFn,
    sell_t1_only: bool,
    with_fills: bool,
) -> Result<RecursiveResult, String> {
    if floor_ladder < FIRST_BSP_LADDER {
        return Err(format!(
            "递归建仓要求 floor_ladder ≥ {FIRST_BSP_LADDER}（BSP 承载层）；\
             floor_ladder={floor_ladder}"
        ));
    }
    if !(base_frac > 0.0 && base_frac.is_finite() && base_frac <= 1.0) {
        return Err(format!("base_frac 须 ∈ (0, 1]：{base_frac}"));
    }
    if !tape.has_bsp_events() {
        return Err("递归建仓要求事件磁带（bsp_events 全空）".to_string());
    }

    let mut st = Slices { shares: [0.0; MAX_LADDER], cash: INITIAL_CAPITAL };
    let mut res = RecursiveResult::default();
    let mut peak = INITIAL_CAPITAL;
    let mut invested_frac_sum = 0.0;
    let n = tape.bars.len();

    for (i, sig) in tape.bars.iter().enumerate() {
        let c = sig.close;
        if let Some(rows) = sig.bsp_events.as_deref() {
            for (lad, evs) in rows.iter().enumerate().skip(floor_ladder) {
                for e in evs {
                    if !e.confirmed {
                        continue;
                    }
                    if sell_t1_only
                        && e.class.side() == Side::Sell
                        && e.class != super::types::BspClass::Sell1
                    {
                        continue;
                    }
                    apply_event(&mut st, &mut res, e, lad, floor_ladder, base_frac, weight, c, i as i64, with_fills);
                }
            }
        }
        let eq = st.equity(c);
        if eq > peak {
            peak = eq;
        }
        let dd = (peak - eq) / peak * 100.0;
        if dd > res.max_dd_pct {
            res.max_dd_pct = dd;
        }
        invested_frac_sum += if eq > 0.0 { st.invested(c) / eq } else { 0.0 };
    }

    let last_close = tape.bars[n - 1].close;
    let final_eq = st.equity(last_close);
    res.final_return_pct = (final_eq - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100.0;
    res.avg_invested_frac = invested_frac_sum / n as f64;
    res.final_invested_frac =
        if final_eq > 0.0 { st.invested(last_close) / final_eq } else { 0.0 };
    Ok(res)
}

/// 单事件消费：买 = 填至 quota（现金饱和），卖 = 清至 0。幂等。
#[allow(clippy::too_many_arguments)]
fn apply_event(
    st: &mut Slices,
    res: &mut RecursiveResult,
    e: &BspEvent,
    lad: usize,
    floor: usize,
    base_frac: f64,
    weight: WeightFn,
    c: f64,
    bar: i64,
    with_fills: bool,
) {
    const EPS: f64 = 1e-9;
    match e.class.side() {
        Side::Buy => {
            let target = base_frac * weight.weight(lad, floor) * INITIAL_CAPITAL;
            let need = target - st.shares[lad] * c;
            if need <= EPS {
                res.buy_noops[lad] += 1;
                return;
            }
            let spend = need.min(st.cash);
            if spend <= EPS {
                res.buy_starved[lad] += 1;
                return;
            }
            if spend < need - EPS {
                res.buy_starved[lad] += 1;
            }
            st.cash -= spend;
            st.shares[lad] += spend / c;
            res.buy_fills[lad] += 1;
            res.turnover += spend / INITIAL_CAPITAL;
            if with_fills {
                res.fills.push(FillRec {
                    bar,
                    ladder: lad as u8,
                    side: 0,
                    shares: spend / c,
                    price: c,
                    cash_after: st.cash,
                    equity_after: st.equity(c),
                });
            }
        }
        Side::Sell => {
            let sh = st.shares[lad];
            if sh <= EPS {
                res.sell_noops[lad] += 1;
                return;
            }
            st.shares[lad] = 0.0;
            st.cash += sh * c;
            res.sell_clears[lad] += 1;
            res.turnover += sh * c / INITIAL_CAPITAL;
            if with_fills {
                res.fills.push(FillRec {
                    bar,
                    ladder: lad as u8,
                    side: 1,
                    shares: sh,
                    price: c,
                    cash_after: st.cash,
                    equity_after: st.equity(c),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tape::BarSig;
    use super::super::types::BspClass;

    fn ev(class: BspClass, confirmed: bool) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs: None,
            zd: None,
            zg: None,
            price: 0.0,
        }
    }

    fn bar(close: f64, events: Vec<(usize, BspEvent)>) -> BarSig {
        let mut b = BarSig { close, ..Default::default() };
        if !events.is_empty() {
            let mut rows: Box<[Vec<BspEvent>; MAX_LADDER]> = Box::default();
            for (lad, e) in events {
                rows[lad].push(e);
            }
            b.bsp_events = Some(rows);
        }
        b
    }

    fn tape(bars: Vec<BarSig>) -> SignalTape {
        SignalTape { bars, ..Default::default() }
    }

    #[test]
    fn buy_fills_quota_and_is_idempotent() {
        // base=0.1, exp2：quota(2)=0.1, quota(3)=0.2。价格恒定 ⇒ 权益守恒。
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, true))]),
            bar(100.0, vec![(2, ev(BspClass::Buy2, true))]), // 已满 → no-op
            bar(100.0, vec![(3, ev(BspClass::Buy1, true))]),
        ]);
        let r = run_recursive(&t, 2, 0.1, WeightFn::Exp2, false, true).unwrap();
        assert_eq!(r.buy_fills[2], 1);
        assert_eq!(r.buy_noops[2], 1);
        assert_eq!(r.buy_fills[3], 1);
        assert_eq!(r.fills.len(), 2);
        // slice2 = 10k → 100 股；slice3 = 20k → 200 股
        assert!((r.fills[0].shares - 100.0).abs() < 1e-9);
        assert!((r.fills[1].shares - 200.0).abs() < 1e-9);
        assert!((r.final_return_pct - 0.0).abs() < 1e-9); // 价格不动，权益守恒
        assert!((r.final_invested_frac - 0.3).abs() < 1e-9);
    }

    #[test]
    fn sell_clears_slice_and_is_idempotent() {
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, true))]),
            bar(110.0, vec![(2, ev(BspClass::Sell1, true))]),
            bar(110.0, vec![(2, ev(BspClass::Sell2, true))]), // 空 → no-op
        ]);
        let r = run_recursive(&t, 2, 0.1, WeightFn::Exp2, false, false).unwrap();
        assert_eq!(r.sell_clears[2], 1);
        assert_eq!(r.sell_noops[2], 1);
        // 100 股 @100 买入、@110 清仓 → +1000 = +1% of 100k
        assert!((r.final_return_pct - 1.0).abs() < 1e-9);
        assert!((r.final_invested_frac - 0.0).abs() < 1e-9);
    }

    #[test]
    fn cash_saturation_truncates_high_level_fill() {
        // base=0.5, exp2：quota(2)=0.5, quota(3)=1.0 → 第二笔只剩 0.5 现金
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, true))]),
            bar(100.0, vec![(3, ev(BspClass::Buy1, true))]),
        ]);
        let r = run_recursive(&t, 2, 0.5, WeightFn::Exp2, false, false).unwrap();
        assert_eq!(r.buy_fills[3], 1);
        assert_eq!(r.buy_starved[3], 1); // 截断计数
        assert!((r.final_invested_frac - 1.0).abs() < 1e-9); // 满仓
        assert!((r.final_return_pct - 0.0).abs() < 1e-9); // 守恒
    }

    #[test]
    fn candidate_events_are_not_consumed() {
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, false))]),
            bar(100.0, vec![]),
        ]);
        let r = run_recursive(&t, 2, 0.1, WeightFn::Exp2, false, false).unwrap();
        assert_eq!(r.buy_fills[2], 0);
        assert!((r.final_invested_frac - 0.0).abs() < 1e-9);
    }

    #[test]
    fn sell_then_rebuy_reopens_slice() {
        // 38课循环的自然形式：卖点清 → 买点回补（无 FSM）
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, true))]),
            bar(110.0, vec![(2, ev(BspClass::Sell1, true))]),
            bar(105.0, vec![(2, ev(BspClass::Buy2, true))]),
        ]);
        let r = run_recursive(&t, 2, 0.1, WeightFn::Exp2, false, false).unwrap();
        assert_eq!(r.buy_fills[2], 2);
        assert_eq!(r.sell_clears[2], 1);
        // 第二次回补按 quota 重填（10k @105）
        assert!((r.final_invested_frac - 0.1 / (1.0 + 0.01)).abs() < 1e-3);
    }

    #[test]
    fn sell_t1_only_ignores_sell23() {
        let t = tape(vec![
            bar(100.0, vec![(2, ev(BspClass::Buy1, true))]),
            bar(110.0, vec![(2, ev(BspClass::Sell3, true))]), // 忽略
            bar(120.0, vec![(2, ev(BspClass::Sell2, true))]), // 忽略
            bar(130.0, vec![(2, ev(BspClass::Sell1, true))]), // 清
        ]);
        let r = run_recursive(&t, 2, 0.1, WeightFn::Exp2, true, false).unwrap();
        assert_eq!(r.sell_clears[2], 1);
        assert_eq!(r.sell_noops[2], 0); // 被忽略的卖点不入 noop 计数
        // 100 股 @100 → @130 清 → +3000 = +3%
        assert!((r.final_return_pct - 3.0).abs() < 1e-9);
    }

    #[test]
    fn linear_weight_and_floor_guard() {
        assert_eq!(WeightFn::parse("linear"), Some(WeightFn::Linear));
        assert_eq!(WeightFn::Linear.weight(4, 2), 3.0);
        assert_eq!(WeightFn::Exp2.weight(4, 2), 4.0);
        let t = tape(vec![bar(100.0, vec![(2, ev(BspClass::Buy1, true))])]);
        assert!(run_recursive(&t, 1, 0.1, WeightFn::Exp2, false, false).is_err());
        assert!(run_recursive(&t, 2, 0.0, WeightFn::Exp2, false, false).is_err());
        assert!(run_recursive(&t, 2, 1.5, WeightFn::Exp2, false, false).is_err());
    }
}
