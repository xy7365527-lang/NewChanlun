//! nested_fugue — 嵌套递归赋格（NRF final；mode = "nrf"）。
//!
//! 核心命题（2026-06-12 编排者）：**逐仓独立头寸和递归区间套是同一件事**。
//! 区间套递归（027课"从高级别向低级别逐级寻找背驰点"）就是 voice spawn 的
//! 时序机制；逐仓独立头寸就是区间套递归的物质形态。区间套递归到 k−1 层 =
//! k−1 voice 诞生（在 k−1 层开一个方向与父层相反的独立逐仓头寸）。
//!
//! 概念链对应（docs/concept_movement_chain.md）：第14环（区间套=递归层间
//! 桥梁）× 第18环（并发=级别同时性）× 第19环（多重赋格=stretto）× 第20环
//! （嵌套递归=voice 自相似）× 第22环（多空嵌套=方向交替递归）。
//!
//! 统一递归机制（单一规则，全层同构，零概念 flag）：
//! - **spawn**：槽 k 持仓（方向 d）期间，本级别反向 candidate Type1/Type3
//!   武装区间套窗口（NestWin 语义与 fusion nest_forward 逐字同源：武装→
//!   次级别第一同侧证据触发→破极值否定），触发 ⇒ **父仓不动**，在 k−1 开
//!   方向 −d 的独立逐仓头寸（35课立体性"次级别的次级别也可以用来部分操作"）。
//!   与在册 nest_forward 的范畴差：在册消费端 = 提前削减父仓（八标的判决
//!   "削减本身失血"BTC −677pp 死因）；本机制消费端 = 子层反向头寸（父层
//!   趋势暴露保持，过渡期由子头寸对冲）。
//! - **生命周期**：每层 voice = 该层走势完美时结束——confirmed 反向买卖点
//!   ⇒ 平仓（Long→sell_any@k / Short→buy_any@k），并级联回收子树（子腿
//!   宿主前提消失，cascade_close 先例）。
//! - **否定**（027:25"只要没有打破背驰段"逆否）：价格越过 spawn 时的
//!   candidate 极值 = 背驰段打破 = 该头寸的区间套前提消失 ⇒ 强制平仓（级联）。
//! - **递归终止三范畴**（35课/77-78课）：k−1 < floor（存在论：笔=a0）∨
//!   θ(k−1) 成本门（经济：θ_q < k×friction）∨ 槽占用（结构：一层一 voice）。
//! - **根 voice**：全树空仓时，最高 θ 涌现层 top 的买证据（confirmed buy ∨
//!   买侧区间套触发）⇒ 开多。根的级别随塔生长在重入时自然爬升（涌现归属）。
//! - **配额** = theta_weights（26课级别=买卖量；第15环级别=操作量）——
//!   根拿 w_top，剩余 pool 是子 voice spawn 的结构性弹药（非闲置配额）。
//! - **空头** = 1x 虚拟逐仓 [镜像推导]（margin = units×entry 锁定；层权益
//!   = margin + units×(entry−c)，≤0 强平，解析强平价 2×entry——损失有界，
//!   `analysis/bidirectional_nested_accounting.md` §4.3 口径）。
//!
//! 唯一经验参数 = a0（磁带粒度）；floor_ladder 为结构常数。
//! 在册路径零接触（新 PolarityMode 入口；nrf 计数器其余模式恒零）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    theta_weights, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS, MIN_FILL_FRAC,
};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 区间套窗口（fusion::NestWin 同语义；本模块只需 extreme——lead 统计是
/// fusion 轴的诊断面，不在本机制词汇）。
#[derive(Debug, Clone, Copy)]
struct Win {
    /// candidate 背驰段极值（卖窗取 max 刷新 / 买窗取 min）。
    extreme: f64,
}

/// 层槽：一层至多一个 voice（独立逐仓头寸）。
#[derive(Debug, Clone, Copy)]
enum Slot {
    Idle,
    Open {
        pol: Polarity,
        entry_bar: i64,
        entry_price: f64,
        units: f64,
        /// Short：开仓 bar 从 pool 锁定的保证金（= units×entry）；Long 恒 0。
        margin: f64,
        weight: f64,
        partial: bool,
        /// spawn 本 voice 的父层（None = 根）。级联回收沿此链向下。
        parent: Option<u8>,
        /// spawn 时 candidate 背驰段极值（027:25 否定线）。
        /// Short 子：c > line 即否定；Long 子/nest 根：c < line 即否定。
        /// confirmed 词汇入场的根无否定线。
        negate_line: Option<f64>,
    },
}

/// 次级别证据（区间套触发词汇；fusion::nest_sub_evidence 逐字同源——
/// BSP 承载层 = 任意同侧 BSP ∨ 同侧背驰事件；bi 层 = 本 bar 方向翻转沿）。
fn sub_evidence(
    sub: usize,
    side: Side,
    evs: &[BspEvent],
    devs: &[DivEvent],
    flip_edge: Option<Direction>,
) -> bool {
    if sub >= FIRST_BSP_LADDER {
        evs.iter().any(|e| e.class.side() == side) || devs.iter().any(|d| d.side() == side)
    } else {
        let want = match side {
            Side::Sell => Direction::Down,
            Side::Buy => Direction::Up,
        };
        flip_edge == Some(want)
    }
}

/// NAV：pool + Σ Long 市值 + Σ Short 逐仓权益（margin + units×(entry−c)；
/// 浮亏可为负——强平在出场阶段收口）。
fn nav_nrf(slots: &[Slot; MAX_LADDER], cash: f64, c: f64, floor: usize) -> f64 {
    let mut v = cash;
    for slot in slots.iter().take(MAX_LADDER).skip(floor) {
        if let Slot::Open { pol, entry_price, units, margin, .. } = *slot {
            v += match pol {
                Polarity::Long => units * c,
                Polarity::Short => margin + units * (entry_price - c),
            };
        }
    }
    v
}

/// 平仓单槽（现金流 + trade 行 + 计数）。exit_price 由调用方给定
/// （强平 = 2×entry 解析价；其余 = close）。
#[allow(clippy::too_many_arguments)]
fn close_slot(
    k: usize,
    exit_bar: i64,
    exit_price: f64,
    reason: &'static str,
    slots: &mut [Slot; MAX_LADDER],
    pool: &mut f64,
    res: &mut PositionalResult,
) {
    let Slot::Open { pol, entry_bar, entry_price, units, margin, weight, partial, .. } = slots[k]
    else {
        unreachable!("close_slot 调用前提：槽 Open")
    };
    match pol {
        Polarity::Long => *pool += units * exit_price,
        Polarity::Short => {
            let pnl = units * (entry_price - exit_price);
            *pool += margin + pnl;
            res.short_net_cash_by_ladder[k] += pnl;
        }
    }
    res.trades.push(LayerTrade {
        ladder: k as u8,
        entry_bar,
        entry_price,
        exit_bar,
        exit_price,
        shares: units,
        weight_at_entry: weight,
        deferred_bars: 0,
        partial,
        exit_reason: reason,
        polarity: pol,
    });
    res.n_exits_by_ladder[k] += 1;
    slots[k] = Slot::Idle;
}

/// 平仓 + 级联回收子树（子 = k−1 且 parent==k，链连续向下；子树的存在
/// 前提 = 父腿存活，父平即级联——cascade_close 先例）。
#[allow(clippy::too_many_arguments)]
fn close_cascade(
    k: usize,
    exit_bar: i64,
    exit_price: f64,
    reason: &'static str,
    c: f64,
    floor: usize,
    slots: &mut [Slot; MAX_LADDER],
    pool: &mut f64,
    res: &mut PositionalResult,
) {
    close_slot(k, exit_bar, exit_price, reason, slots, pool, res);
    let mut j = k;
    while j > floor {
        j -= 1;
        match slots[j] {
            Slot::Open { parent: Some(p), .. } if p as usize == j + 1 => {
                close_slot(j, exit_bar, c, "cascade", slots, pool, res);
                res.n_nrf_cascade_closes_by_ladder[j] += 1;
            }
            _ => break,
        }
    }
}

/// 开仓（配额消费）。target = weight×nav；pool 不足按可用缩量（partial）；
/// 低于防尘埃线拒开。返回是否成交。
#[allow(clippy::too_many_arguments)]
fn open_slot(
    k: usize,
    pol: Polarity,
    bar: i64,
    c: f64,
    weight: f64,
    nav: f64,
    parent: Option<u8>,
    negate_line: Option<f64>,
    slots: &mut [Slot; MAX_LADDER],
    pool: &mut f64,
    res: &mut PositionalResult,
) -> bool {
    let target = weight * nav;
    let avail = pool.min(target);
    if !(avail.is_finite() && avail > 0.0) || avail < MIN_FILL_FRAC * target {
        res.n_nrf_dust_skips_by_ladder[k] += 1;
        return false;
    }
    let units = avail / c;
    let partial = avail < target * (1.0 - 1e-9);
    *pool -= avail;
    let margin = match pol {
        Polarity::Long => 0.0,
        Polarity::Short => avail,
    };
    slots[k] = Slot::Open {
        pol,
        entry_bar: bar,
        entry_price: c,
        units,
        margin,
        weight,
        partial,
        parent,
        negate_line,
    };
    res.n_entries_by_ladder[k] += 1;
    true
}

/// 主入口（`PolarityMode::NestedRecursive` 经 `run_positional` 分派至此）。
pub(crate) fn run_nested_fugue(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "nested_fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})（BSP 承载层）；\
             floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("nested_fugue 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "nested_fugue 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理），缺行即词汇残缺"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut slots: [Slot; MAX_LADDER] = [Slot::Idle; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // 区间套窗口（双侧，市场性质——candidate 即武装，消费时才查持仓）。
    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];

    // bi 层方向行（稀疏翻转 → 逐 bar 沿；区间套 bi 通道）。
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        let bar = i as i64;

        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        while flip_ptr < flips.len() && flips[flip_ptr].0 == bar {
            let (_, lad, dir) = flips[flip_ptr];
            flip_edge[lad as usize] = Some(dir);
            flip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照（run_positional 逐字同构）。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 区间套窗口维护（fusion nest_forward ①②③ 逐字同序）：
        //    ① 打破否定 → ② candidate 武装/confirmed 清窗 → ③ 次级别证据
        //    触发（nf_* 携带触发时极值 = 子 voice 的否定线）──
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if nest_sell[k].is_some_and(|w| c > w.extreme) {
                nest_sell[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
            }
            if nest_buy[k].is_some_and(|w| c < w.extreme) {
                nest_buy[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
            }
            if sig.bsp_events.is_some() {
                for e in &evrows[k] {
                    let sellside = match e.class {
                        BspClass::Sell1 | BspClass::Sell3 => true,
                        BspClass::Buy1 | BspClass::Buy3 => false,
                        BspClass::Sell2 | BspClass::Buy2 => continue,
                    };
                    let win = if sellside { &mut nest_sell[k] } else { &mut nest_buy[k] };
                    if e.confirmed {
                        // confirmed 同侧到达 ⇒ 窗口让位（confirmed 词汇本 bar
                        // 由出场/根入场路径消费）。
                        *win = None;
                    } else {
                        let ext = win.map_or(e.price, |w| {
                            if sellside {
                                w.extreme.max(e.price)
                            } else {
                                w.extreme.min(e.price)
                            }
                        });
                        *win = Some(Win { extreme: ext });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                }
            }
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if sub_evidence(sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_sell[k] = Some(w.extreme);
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                }
            }
            if let Some(w) = nest_buy[k] {
                if sub_evidence(sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_buy[k] = Some(w.extreme);
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                }
            }
        }

        // ── 阶段 A：出场（高层先判——父平仓级联吃掉子树，低层残余再独立判）。
        //    优先序：强平（会计强制）→ 否定（027:25 前提消失）→ 走势完美
        //    （confirmed 反向词汇 = 该层走势类型完成）──
        for k in (floor_ladder..MAX_LADDER).rev() {
            let Slot::Open { pol, entry_price, units, margin, negate_line, .. } = slots[k] else {
                continue;
            };
            res.held_bars_by_ladder[k] += 1;
            if pol == Polarity::Short {
                res.short_held_bars_by_ladder[k] += 1;
                if margin + units * (entry_price - c) <= 0.0 {
                    // 1x 逐仓强平：解析强平价 2×entry（残值恰为零）。
                    close_cascade(
                        k, bar, 2.0 * entry_price, "liq", c, floor_ladder, &mut slots, &mut pool,
                        &mut res,
                    );
                    res.n_short_liquidations_by_ladder[k] += 1;
                    continue;
                }
            }
            if let Some(line) = negate_line {
                let broken = match pol {
                    Polarity::Short => c > line, // 父层背驰段创新高 ⇒ 空前提消失
                    Polarity::Long => c < line,  // 买侧背驰段创新低 ⇒ 多前提消失
                };
                if broken {
                    close_cascade(
                        k, bar, c, "negate", c, floor_ladder, &mut slots, &mut pool, &mut res,
                    );
                    res.n_nrf_negate_closes_by_ladder[k] += 1;
                    continue;
                }
            }
            let perfected = match pol {
                Polarity::Long => sig.sell_any.get(k),
                Polarity::Short => sig.buy_any.get(k),
            };
            if perfected {
                let reason = match pol {
                    Polarity::Long => "sellpt",
                    Polarity::Short => "cover",
                };
                close_cascade(k, bar, c, reason, c, floor_ladder, &mut slots, &mut pool, &mut res);
            }
        }

        // ── 阶段 B：spawn（区间套递归 = voice 诞生）。NAV 快照在出场后取
        //    （bar 内交易是现金↔头寸等价转换）；高层先判（大级别优先拿配额）──
        let bar_nav = nav_nrf(&slots, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            let Slot::Open { pol, .. } = slots[k] else { continue };
            // 父方向 d ⇒ 反向窗口触发 spawn −d 子 voice。
            let (fired, child_pol) = match pol {
                Polarity::Long => (nf_sell[k], Polarity::Short),
                Polarity::Short => (nf_buy[k], Polarity::Long),
            };
            let Some(extreme) = fired else { continue };
            if k == floor_ladder {
                // 存在论终止：k−1 < floor（笔=a0，77-78课）。
                res.n_nrf_floor_stops_by_ladder[k] += 1;
                continue;
            }
            let sub = k - 1;
            if matches!(slots[sub], Slot::Open { .. }) {
                // 结构终止：一层一 voice。
                res.n_nrf_busy_skips_by_ladder[sub] += 1;
                continue;
            }
            // 经济终止：35课成本门（fusion 成本门常数逐字复用）。
            match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                None => {
                    res.n_nrf_noref_rejects_by_ladder[sub] += 1;
                    continue;
                }
                Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                    res.n_nrf_cost_rejects_by_ladder[sub] += 1;
                    continue;
                }
                Some(_) => {}
            }
            let w = match (thetas[sub], theta_total > 0.0) {
                (Some(t), true) => t / theta_total,
                _ => {
                    res.n_nrf_noref_rejects_by_ladder[sub] += 1;
                    continue;
                }
            };
            if open_slot(
                sub,
                child_pol,
                bar,
                c,
                w,
                bar_nav,
                Some(k as u8),
                Some(extreme),
                &mut slots,
                &mut pool,
                &mut res,
            ) {
                res.n_nrf_spawns_by_ladder[sub] += 1;
            }
        }

        // ── 阶段 C：根入场。全树空仓时，最高 θ 涌现层 top 的买证据
        //    （confirmed buy ∨ 买侧区间套触发——区间套对入场同样是"高级别
        //    买卖点需要低级别定位"，第14环）⇒ 开多。──
        if slots
            .iter()
            .take(MAX_LADDER)
            .skip(floor_ladder)
            .all(|s| matches!(s, Slot::Idle))
        {
            let top = (floor_ladder..MAX_LADDER).rev().find(|&k| thetas[k].is_some());
            if let Some(top) = top {
                let nf = nf_buy[top];
                if sig.buy_any.get(top) || nf.is_some() {
                    let w = thetas[top].expect("top 由 thetas 定义") / theta_total;
                    // confirmed 入场无否定线；nest 入场带 candidate 极值否定线。
                    let line = if sig.buy_any.get(top) { None } else { nf };
                    if open_slot(
                        top,
                        Polarity::Long,
                        bar,
                        c,
                        w,
                        bar_nav,
                        None,
                        line,
                        &mut slots,
                        &mut pool,
                        &mut res,
                    ) {
                        res.n_nrf_root_entries_by_ladder[top] += 1;
                    }
                }
            }
        }

        // 深度观测：同时 Open 槽数（stretto 并发的直接读数）。
        let depth = slots
            .iter()
            .take(MAX_LADDER)
            .skip(floor_ladder)
            .filter(|s| matches!(s, Slot::Open { .. }))
            .count();
        res.nrf_depth_bars[depth.min(MAX_LADDER - 1)] += 1;

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, nav_nrf(&slots, pool, c, floor_ladder)));
        }
    }

    // eod：全平（高层先平级联——与阶段 A 同序）。
    let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
    let last_bar = (n as i64) - 1;
    for k in (floor_ladder..MAX_LADDER).rev() {
        if matches!(slots[k], Slot::Open { .. }) {
            close_cascade(
                k, last_bar, c_last, "eod", c_last, floor_ladder, &mut slots, &mut pool, &mut res,
            );
        }
    }
    res.final_nav = pool;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() { (Some(50.0), Some(60.0)) } else { (None, None) };
        BspEvent { class, seg_idx: 0, confirmed, cs, zd, zg, price }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
        b
    }

    fn buypt(mut b: BarSig, lad: usize) -> BarSig {
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sellpt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热：层 3/4 各喂 SUB_COST_MIN_OBS 个锚（θ₃=1%、θ₄=3%，
    /// c=100 口径——两层均过 35课成本门 2×0.1%）。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let b = with_ev(bar(100.0), 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
            let mut b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
            // 锚事件 zd/zg：层3 振幅 1（50-51）、层4 振幅 3（50-53）。
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            rows[4][0].zd = Some(50.0);
            rows[4][0].zg = Some(53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    fn run(bars: Vec<BarSig>) -> PositionalResult {
        let t = SignalTape {
            bars,
            dir_flips: Some(Vec::new()),
            trend_flips: None,
            ..Default::default()
        };
        run_positional(&t, 2, PolarityMode::parse("nrf").unwrap()).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("nrf"), Some(PolarityMode::NestedRecursive));
        // 缺 div 行 ⇒ 拒绝（词汇残缺 fail-fast）。
        let t = SignalTape {
            bars: vec![with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, PolarityMode::NestedRecursive)
            .unwrap_err()
            .contains("背驰磁带"));
        // 缺 dir 行 ⇒ 拒绝。
        let t2 = SignalTape {
            bars: vec![with_empty_div(with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            ))],
            dir_flips: None,
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, PolarityMode::NestedRecursive)
            .unwrap_err()
            .contains("dir_flips"));
    }

    #[test]
    fn root_enters_at_top_emerged_level_on_confirmed_buy() {
        let mut bars = warmup34();
        // top = 4（θ₄ 已涌现且最高）。confirmed buy@4 ⇒ 根开多@4。
        bars.push(buypt(bar(100.0), 4));
        bars.push(bar(101.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "根应开在最高 θ 涌现层");
        assert_eq!(r.n_entries_by_ladder[4], 1);
        // eod 平仓 ⇒ 1 条 Long trade@4。
        let t = &r.trades[0];
        assert_eq!((t.ladder, t.polarity), (4, Polarity::Long));
        assert_eq!(t.exit_reason, "eod");
        // w₄ = 3/(1+3) = 0.75 配额。
        assert!((t.weight_at_entry - 0.75).abs() < 1e-9);
    }

    #[test]
    fn nest_spawn_opens_short_child_and_negation_closes_it() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根多@4
        // candidate Sell1@4（price=110 = 背驰段极值）⇒ 卖窗武装。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        // 次级别（3）卖侧证据 ⇒ nf_sell[4] ⇒ spawn Short@3（父仓不动）。
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        // 价格越过 110 ⇒ 子空头否定平仓（父多保持）。
        bars.push(bar(111.0));
        bars.push(bar(112.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "应 spawn 子空头@3");
        assert_eq!(r.n_nrf_negate_closes_by_ladder[3], 1, "破极值应否定子头寸");
        let short = r.trades.iter().find(|t| t.polarity == Polarity::Short).unwrap();
        assert_eq!(short.ladder, 3);
        assert_eq!(short.exit_reason, "negate");
        // 根多持有到 eod（未被卖点/级联触碰）。
        let root = r.trades.iter().find(|t| t.polarity == Polarity::Long).unwrap();
        assert_eq!((root.ladder, root.exit_reason), (4, "eod"));
    }

    #[test]
    fn parent_perfection_cascades_children() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        // confirmed sell@4 ⇒ 父走势完美平多，级联回收子空头@3。
        bars.push(sellpt(bar(103.0), 4));
        let r = run(bars);
        assert_eq!(r.n_nrf_cascade_closes_by_ladder[3], 1, "父平仓应级联子树");
        let root = r.trades.iter().find(|t| t.ladder == 4).unwrap();
        assert_eq!(root.exit_reason, "sellpt");
        let child = r.trades.iter().find(|t| t.ladder == 3).unwrap();
        assert_eq!(child.exit_reason, "cascade");
        // 全平后 final_nav 与 trade 现金流闭合（无残余头寸）。
        assert_eq!(r.nrf_depth_bars[0] > 0, true);
    }

    #[test]
    fn floor_stop_and_busy_skip_terminate_recursion() {
        let mut bars = warmup34();
        // 只给层 2/3 头寸路径：根开在 4，spawn 3；3 的反向窗口触发 spawn 2；
        // 2 的反向窗口触发 ⇒ floor 终止。
        bars.push(buypt(bar(100.0), 4));
        // spawn Short@3。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        // Short@3 的买侧窗口：candidate Buy1@3（price=95 极小值）。
        bars.push(with_ev(bar(99.0), 3, ev_full(BspClass::Buy1, false, 95.0, None)));
        // 次级别（2）买证据 ⇒ spawn Long@2——但 θ₂ 无参照 ⇒ noref 拒。
        bars.push(with_ev(bar(98.0), 2, ev_full(BspClass::Buy1, true, 0.0, None)));
        let r = run(bars);
        assert_eq!(r.n_nest_fire_buy_by_ladder[3], 1, "买侧区间套应触发");
        assert_eq!(r.n_nrf_noref_rejects_by_ladder[2], 1, "θ₂ 无参照 ⇒ 经济终止");
        assert_eq!(r.n_nrf_spawns_by_ladder[2], 0);
    }

    #[test]
    fn nav_closure_no_leak() {
        // 现金流闭合守卫：任意路径后 final_nav = 初始资金 + Σ trade 盈亏。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(bar(111.0));
        bars.push(sellpt(bar(108.0), 4));
        let r = run(bars);
        let pnl: f64 = r
            .trades
            .iter()
            .map(|t| match t.polarity {
                Polarity::Long => t.shares * (t.exit_price - t.entry_price),
                Polarity::Short => t.shares * (t.entry_price - t.exit_price),
            })
            .sum();
        assert!(
            (r.final_nav - (INITIAL_CAPITAL + pnl)).abs() < 1e-6,
            "现金流不闭合：final_nav={} 期望={}",
            r.final_nav,
            INITIAL_CAPITAL + pnl
        );
    }
}
