//! positional_fusion — hold26 × CounterSeg 合流（B+C 合体，2026-06-12 任务）。
//!
//! 谱系位置：534号（嵌套递归会计语义概念分离）的实装——矛盾消解点 =
//! 49课行52 同段二相：趋势相（中枢向上移动满仓 = 独立暴露）/震荡相
//! （围绕中枢差价上减下增 = 降成本短差）。两相 regime 正交，无全域赢家。
//! 原文架构判决：`analysis/bc_architecture_research.md`——B 管层间
//! （量按级别、k 级信号最多清 k 级净配额），C 管层内（同股数借出还回、
//! 目的=降成本），26课恒仓段（026:34）是合体的原文原型。
//!
//! ## 三机制（每机制课号锚定）
//!
//! 1. **基座 = hold26 恒仓极性**（026:34"仓位是一直不变的……卖点的时候变少，
//!    买点的时候又回复原来的数量，但绝对不加仓"）——per-layer slice 默认持有，
//!    本层任意 confirmed 卖点削减、任意买点回复（positional.rs 在册，
//!    BTC +1247% L2）。
//! 2. **趋势相停削**（049:52"中枢向上移动时，就应该满仓，这才是最正确的
//!    仓位"）——层 k 趋势相判据 = 尾 move kind==Trend ∧ dir==Up（17课趋势
//!    定义"≥2 同向中枢"的引擎直读：trend_flips × dir_flips 磁带行；引擎
//!    映射近似声明：49课"中枢向上移动"的对象是层 k 的走势，本实现以该层
//!    尾 move kind 与方向行的合取读出）。趋势相 ⇒ 本层卖点不削减（差价
//!    不能做）；震荡相 ⇒ hold26 在册削减。熊市（dir≠Up）行为与 hold26
//!    逐字相同——P2 熊市 α 的承载面零接触。
//! 3. **层内 C 短差（CounterSeg 词汇）**（053:34"在第二、三买卖点之间，
//!    都是中枢震荡……如果参与其中的买卖，用的都是低级别的买卖点" +
//!    049:64"在中枢上方全部抛出筹码，在下方如数接回"）——震荡相中
//!    k−1 级卖证据（confirmed Sell1 ∨ 盘背卖；`SubMode::CounterSeg`
//!    开腿谓词的 Short 侧逐字镜像）全抛本层 slice，k−1 级买证据
//!    （confirmed Buy1/Buy3 ∨ 盘背买；counterseg_close_trigger 镜像）
//!    如数接回。35课成本门（θ_q(k−1) 因果滚动中位数 ≥ k×friction）守卫
//!    级别可操作性；笔=a0（77/78课）⇒ k−1 < FIRST_BSP_LADDER 不开。
//!
//! ## 44课铰链（044:44，两线均未实现的新轴）
//!
//! > "就必须先出一部分，然后在出现上一段所说的情况时在出清。当然，如果
//! > 没有出现上一段所说的情况，就可以回补，权当弄了一个短差。"
//!
//! 层内 C 卖出**不预声明身份**——同一笔卖出的两种事后路径：
//! - k−1 买回证据先到 ⇒ 回补如数接回（事后归类为**短差**，C 路径）；
//! - 本层 k 级卖点先到（恶化升级条件的引擎映射：该买点起始的走势被本级别
//!   卖点宣告结束）⇒ **升级为减仓**（出清，B 路径）——trade 以实际变现价
//!   （短差卖出 bar 的 close）记账，exit_reason = "hinge_escalate"。
//! - 趋势相开始 ⇒ 强制回补（049:52 满仓义务——对象域消失）。
//!
//! ## 会计不变量
//!
//! 短差卖出 = 股数→现金的等价转换（卖出 bar close 单一成交价，现金入
//! pool）；接回 = 现金→同股数。trade 行的 exit_bar/exit_price 恒为实际
//! 变现点 ⇒ 逐 trade 行重建 NAV 与 final_nav 逐位一致（回测脚本 assert）。
//! 回补义务因 pool 不足推迟时逐 bar 重试（D7 推迟同构），且回补在阶段 A
//! 处理（先于一切新入场）——义务优先于配额（osc 僵尸腿判决：闭腿是义务）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, PositionalResult,
    EQUITY_SAMPLE_BARS,
};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::divergence::DivKind;
use crate::stroke::Direction;

/// 35课成本门参数——逐字复用 `OrganicConfig` 默认（sub_cost_k=2.0,
/// sub_friction_rt=0.001；231号零新参数纪律；单测
/// `cost_gate_constants_match_organic_defaults` 守护漂移）。
const SUB_COST_K: f64 = 2.0;
const SUB_FRICTION_RT: f64 = 0.001;

/// 层内 C 短差的在外态（44课铰链：身份未决——回补=短差 / k 级卖点=减仓）。
#[derive(Debug, Clone, Copy)]
struct SubOut {
    /// 卖出的股数（= 卖出 bar 本层全部股数，049:64"全部抛出筹码"）。
    shares: f64,
    sell_bar: i64,
    sell_price: f64,
    /// 回补触发已发生但 pool 不足 ⇒ 逐 bar 重试（D7 推迟同构）。
    restore_due: bool,
}

/// 层内短差开（先卖）谓词——`SubMode::CounterSeg` 开腿三岔镜像的 Short 侧
/// 逐字复刻（confirmed Sell1 ∨ 盘背卖；type2/3 不在开腿词汇——震荡型）。
fn sub_open_trigger(evs: &[BspEvent], devs: &[DivEvent]) -> bool {
    evs.iter().any(|e| e.confirmed && e.class == BspClass::Sell1)
        || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up)
}

/// 层内短差回补（后买）谓词——`counterseg_close_trigger` 的 Short 侧逐字
/// 镜像（confirmed Buy1 ∨ confirmed Buy3 ∨ 盘背买；Buy3 = 中枢向上离开 =
/// 向下反向段终结的更强结构陈述）。
fn sub_close_trigger(evs: &[BspEvent], devs: &[DivEvent]) -> bool {
    evs.iter()
        .any(|e| e.confirmed && (e.class == BspClass::Buy1 || e.class == BspClass::Buy3))
        || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Down)
}

/// NAV：pool + 各层在市股数（短差在外的层其价值已是 pool 现金）。
fn nav_fusion(
    layers: &[LayerState; MAX_LADDER],
    subs: &[Option<SubOut>; MAX_LADDER],
    cash: f64,
    c: f64,
    floor: usize,
) -> f64 {
    let mut v = cash;
    for k in floor..MAX_LADDER {
        if subs[k].is_some() {
            continue;
        }
        if let LayerState::Long { shares, .. } = layers[k] {
            v += shares * c;
        }
    }
    v
}

/// 回补尝试（如数接回）。成功 ⇒ 推入在外窗口 trade 行（"sub_diff"）、
/// 本层 Long 周期重锚到当下（entry_bar/entry_price = 接回点）、清 sub 态；
/// pool 不足 ⇒ 置 restore_due 逐 bar 重试并计数。
#[allow(clippy::too_many_arguments)]
fn try_restore(
    k: usize,
    c: f64,
    bar: i64,
    pool: &mut f64,
    layers: &mut [LayerState; MAX_LADDER],
    subs: &mut [Option<SubOut>; MAX_LADDER],
    res: &mut PositionalResult,
) -> bool {
    let sub = subs[k].expect("调用前提：subs[k] 为 Some");
    let cost = sub.shares * c;
    if cost > *pool {
        subs[k] = Some(SubOut { restore_due: true, ..sub });
        res.n_sub_restore_defer_bars += 1;
        return false;
    }
    let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
        layers[k]
    else {
        unreachable!("sub 在外 ⇒ 本层恒 Long（sub 生命周期内层不变迁）")
    };
    debug_assert!((shares - sub.shares).abs() < 1e-12, "全抛/如数接回 ⇒ 股数恒等");
    *pool -= cost;
    res.sub_net_cash_by_ladder[k] += sub.shares * sub.sell_price - cost;
    res.trades.push(LayerTrade {
        ladder: k as u8,
        entry_bar,
        entry_price,
        exit_bar: sub.sell_bar,
        exit_price: sub.sell_price,
        shares: sub.shares,
        weight_at_entry: weight,
        deferred_bars,
        partial,
        exit_reason: "sub_diff",
    });
    layers[k] = LayerState::Long {
        entry_bar: bar,
        entry_price: c,
        shares,
        weight,
        deferred_bars: 0,
        partial,
    };
    subs[k] = None;
    true
}

/// 主入口（`PolarityMode::Fusion` 经 `run_positional` 分派至此）。
pub(crate) fn run_fusion(
    tape: &SignalTape,
    floor_ladder: usize,
    trend_hold: bool,
    counter_sub: bool,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "fusion 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("fusion 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(trend_hold || counter_sub) {
        return Err(
            "Fusion{trend_hold:false, counter_sub:false} 是 Hold26 的冗余表示\
             ——声明=能力，显式拒绝（Recursive base_frac=1 先例）"
                .to_string(),
        );
    }
    if trend_hold && !(tape.has_trend_rows() && tape.has_dir_rows()) {
        return Err(
            "trend_hold（49课:52 二相停削）要求磁带 trend_flips + dir_flips 行\
             ——趋势相判据 = kind==Trend ∧ dir==Up（17课趋势定义），无行即\
             判据无数据基础（不提供方向行单独代理降级：方向 ≠ 趋势）"
                .to_string(),
        );
    }
    if counter_sub && !tape.has_div_events() {
        return Err(
            "counter_sub（层内 C 短差）要求背驰磁带（div_events 全空）——\
             盘背是开/回补词汇的结构分量（CounterSeg 三岔镜像），缺行即词汇\
             残缺（不静默降级为纯事件词汇）"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut subs: [Option<SubOut>; MAX_LADDER] = [None; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // D3/趋势滚动状态（runner.rs 同构：稀疏翻转行 → 逐 bar 视图）。
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;

        while flip_ptr < flips.len() && flips[flip_ptr].0 == i as i64 {
            let (_, lad, dir) = flips[flip_ptr];
            dir_state[lad as usize] = Some(dir);
            flip_ptr += 1;
        }
        while tflip_ptr < tflips.len() && tflips[tflip_ptr].0 == i as i64 {
            let (_, lad, is_trend) = tflips[tflip_ptr];
            trend_state[lad as usize] = is_trend;
            tflip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照（与 run_positional 逐字同构）。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] =
            sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // 层 k 趋势相（49课:52"中枢向上移动" = kind==Trend ∧ dir==Up）。
        let in_trend =
            |k: usize| trend_hold && trend_state[k] && dir_state[k] == Some(Direction::Up);

        // ── 阶段 A：层级出场 / 44课铰链 / 回补（资金释放先于一切入场；
        //    回补义务在本阶段处理 ⇒ 对 pool 的优先权高于阶段 C 新入场）──
        for k in floor_ladder..MAX_LADDER {
            let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
                layers[k]
            else {
                continue;
            };
            res.held_bars_by_ladder[k] += 1;
            let tp = in_trend(k);
            if let Some(sub) = subs[k] {
                if tp {
                    // 049:52 满仓义务：趋势相开始 ⇒ 强制回补（对象域消失）。
                    if try_restore(k, c, i as i64, &mut pool, &mut layers, &mut subs, &mut res)
                    {
                        res.n_sub_phase_closes_by_ladder[k] += 1;
                    }
                } else if sig.sell_any.get(k) {
                    // 044:44 铰链恶化升级：本层卖点先到 ⇒ 短差卖出升级为
                    // 减仓（出清）。现金已在卖出 bar 入 pool——只改记账身份。
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: sub.sell_bar,
                        exit_price: sub.sell_price,
                        shares: sub.shares,
                        weight_at_entry: weight,
                        deferred_bars,
                        partial,
                        exit_reason: "hinge_escalate",
                    });
                    res.n_sub_escalates_by_ladder[k] += 1;
                    res.n_exits_by_ladder[k] += 1;
                    layers[k] = LayerState::Flat;
                    subs[k] = None;
                } else {
                    let sc = k >= 1 && sub_close_trigger(&evrows[k - 1], &devrows[k - 1]);
                    let kb = sig.buy_any.get(k);
                    if sub.restore_due || sc || kb {
                        let ok = try_restore(
                            k, c, i as i64, &mut pool, &mut layers, &mut subs, &mut res,
                        );
                        if ok {
                            if sc || sub.restore_due {
                                res.n_sub_restores_by_ladder[k] += 1;
                            } else {
                                res.n_sub_kbuy_restores_by_ladder[k] += 1;
                            }
                        }
                    }
                }
            } else if sig.sell_any.get(k) {
                if tp {
                    // 049:52 趋势相停削（"那种中枢完成后的向上移动时的差价
                    // 是不能做的，中枢向上移动时，就应该满仓"）。
                    res.n_trend_holds_by_ladder[k] += 1;
                } else {
                    // hold26 在册削减（震荡相：上减——26课/49课二相的震荡侧）。
                    pool += shares * c;
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: i as i64,
                        exit_price: c,
                        shares,
                        weight_at_entry: weight,
                        deferred_bars,
                        partial,
                        exit_reason: "sellpt",
                    });
                    res.n_exits_by_ladder[k] += 1;
                    layers[k] = LayerState::Flat;
                }
            }
        }

        // ── 阶段 B：层内短差开（先卖；53课次级别词汇 + 49课:64 全抛）。
        //    股数→现金等价转换 ⇒ NAV 不变，阶段 C 快照不受先后影响 ──
        if counter_sub {
            for k in (floor_ladder.max(FIRST_BSP_LADDER + 1))..MAX_LADDER {
                if subs[k].is_some() || in_trend(k) {
                    continue;
                }
                let LayerState::Long { shares, .. } = layers[k] else { continue };
                let sk = k - 1; // 笔=a0（77/78课）：循环下界已保证 sk ≥ FIRST_BSP_LADDER
                if !sub_open_trigger(&evrows[sk], &devrows[sk]) {
                    continue;
                }
                // 35课成本门：次级别典型中枢振幅 ≥ k×friction 才可操作；
                // 参照不可定义 ⇒ 保守拒绝（不静默放行）。
                match depth_ref.theta(sk, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                    None => {
                        res.n_sub_noref_rejects_by_ladder[k] += 1;
                        continue;
                    }
                    Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                        res.n_sub_cost_rejects_by_ladder[k] += 1;
                        continue;
                    }
                    Some(_) => {}
                }
                pool += shares * c;
                subs[k] = Some(SubOut {
                    shares,
                    sell_bar: i as i64,
                    sell_price: c,
                    restore_due: false,
                });
                res.n_sub_opens_by_ladder[k] += 1;
            }
        }

        // ── 阶段 C：入场/回复（hold26 同构：confirmed 买点直接消费，无
        //    ARMED；ladder 降序——大级别优先拿配额）──
        let bar_nav = nav_fusion(&layers, &subs, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            match layers[k] {
                LayerState::Flat => {
                    if sig.buy_any.get(k) {
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                LayerState::Pending { confirm_bar } => {
                    if sig.sell_any.get(k) {
                        res.n_pending_cancels_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    } else {
                        layers[k] = enter_or_defer(
                            k, confirm_bar, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                LayerState::Armed { .. } => {
                    unreachable!("Fusion 无 ARMED 相位——confirmed 事件直接消费")
                }
                LayerState::Long { .. } => {}
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层收口。短差在外的层其股数已变现（现金在 pool）——trade 行
    //    以实际变现点记账（身份悬置为 eod，铰链未及裁决）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
            layers[k]
        else {
            continue;
        };
        let (exit_bar, exit_price, sh) = match subs[k] {
            Some(sub) => (sub.sell_bar, sub.sell_price, sub.shares),
            None => {
                pool += shares * last_close;
                (n as i64 - 1, last_close, shares)
            }
        };
        res.trades.push(LayerTrade {
            ladder: k as u8,
            entry_bar,
            entry_price,
            exit_bar,
            exit_price,
            shares: sh,
            weight_at_entry: weight,
            deferred_bars,
            partial,
            exit_reason: "eod",
        });
        res.n_exits_by_ladder[k] += 1;
        layers[k] = LayerState::Flat;
        subs[k] = None;
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

    fn anchor_ev(cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: false,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    fn ev(class: BspClass) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: None,
            zd: None,
            zg: None,
            price: 0.0,
        }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_anchor(b: BarSig, lad: usize, cs: i64, zd: f64, zg: f64) -> BarSig {
        with_ev(b, lad, anchor_ev(cs, zd, zg))
    }

    fn div_ev(dir: Direction) -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: dir,
            seg_idx: 0,
            force_a: 0.0,
            force_c: 0.0,
            price: 0.0,
        }
    }

    fn with_div(mut b: BarSig, lad: usize, d: DivEvent) -> BarSig {
        let rows = b
            .div_events
            .get_or_insert_with(|| Box::new(<[Vec<DivEvent>; MAX_LADDER]>::default()));
        rows[lad].push(d);
        b
    }

    /// 空 div 行占位（counter_sub capability guard 的最小满足）。
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

    /// 头部参照：θ₃=1%、θ₄=3%（c=100 口径）⇒ w₄=0.75；θ₃ 过 35课成本门
    /// （1% ≥ 2×0.1%）。首 bar 带空 div 行满足 counter_sub guard。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
            let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    fn run(bars: Vec<BarSig>, mode: &str) -> PositionalResult {
        run_with_rows(bars, mode, None, None)
    }

    fn run_with_rows(
        bars: Vec<BarSig>,
        mode: &str,
        dir_flips: Option<Vec<(i64, u8, Direction)>>,
        trend_flips: Option<Vec<(i64, u8, bool)>>,
    ) -> PositionalResult {
        let t = SignalTape { bars, dir_flips, trend_flips, ..Default::default() };
        run_positional(&t, 2, PolarityMode::parse(mode).unwrap()).unwrap()
    }

    #[test]
    fn cost_gate_constants_match_organic_defaults() {
        let d = crate::trading::config::OrganicConfig::default();
        assert_eq!(SUB_COST_K, d.sub_cost_k, "成本门倍数与 OrganicConfig 默认漂移");
        assert_eq!(SUB_FRICTION_RT, d.sub_friction_rt, "摩擦参数与 OrganicConfig 默认漂移");
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(
            PolarityMode::parse("fusion"),
            Some(PolarityMode::Fusion { trend_hold: true, counter_sub: true })
        );
        assert_eq!(
            PolarityMode::parse("fusion_t"),
            Some(PolarityMode::Fusion { trend_hold: true, counter_sub: false })
        );
        assert_eq!(
            PolarityMode::parse("fusion_s"),
            Some(PolarityMode::Fusion { trend_hold: false, counter_sub: true })
        );
        let bars = warmup34();
        // Fusion{false,false} = Hold26 冗余表示 ⇒ 拒绝
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(
            &t,
            2,
            PolarityMode::Fusion { trend_hold: false, counter_sub: false }
        )
        .is_err());
        // trend_hold 无 trend/dir 行 ⇒ 拒绝
        let t2 = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion { trend_hold: true, counter_sub: false }
        )
        .is_err());
        // counter_sub 无 div 磁带 ⇒ 拒绝（warmup34 不带 div 行时）
        let no_div: Vec<BarSig> = bars
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t3 = SignalTape { bars: no_div, ..Default::default() };
        assert!(run_positional(
            &t3,
            2,
            PolarityMode::Fusion { trend_hold: false, counter_sub: true }
        )
        .is_err());
    }

    #[test]
    fn trend_phase_suppresses_trim_oscillation_resumes() {
        // θ₂ 参照 → 层2 全仓入场 → 趋势相卖点停削 → 震荡相卖点削减。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(110.0), 2)); // 趋势相（行在下方注入）→ 停削
        bars.push(sellpt(bar(108.0), 2)); // 趋势翻落 → 削减 @108
        bars.push(bar(108.0));
        let entry_bar = SUB_COST_MIN_OBS as i64;
        let r = run_with_rows(
            bars,
            "fusion_t",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(0, 2, true), (entry_bar + 2, 2, false)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "趋势相卖点停削");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 108.0, "震荡相恢复削减");
        assert!((r.final_nav - 108_000.0).abs() < 1e-6);
    }

    #[test]
    fn bear_direction_keeps_hold26_trim() {
        // 趋势相判据要求 dir==Up：kind==Trend ∧ dir==Down（下行趋势）不停削
        // ——P2 熊市 α 承载面零接触。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(90.0), 2)); // 下行趋势中卖点 → 正常削减
        bars.push(bar(80.0));
        let r = run_with_rows(
            bars,
            "fusion_t",
            Some(vec![(0, 2, Direction::Down)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 0);
        assert_eq!(r.trades[0].exit_reason, "sellpt");
        assert_eq!(r.trades[0].exit_price, 90.0);
    }

    #[test]
    fn sub_cycle_sell_high_buy_back_same_shares() {
        // 层4 @100 入场（w=0.75，750股）→ Sell1@3 @110 全抛 → Buy1@3 @105
        // 如数接回（净 +3750）→ eod @105。NAV = 100000 + 750×(110−105) +
        // 750×(105−100) = 107500。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(with_ev(bar(105.0), 3, ev(BspClass::Buy1)));
        bars.push(bar(105.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 1);
        assert!((r.sub_net_cash_by_ladder[4] - 3750.0).abs() < 1e-6);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 2);
        assert_eq!(t4[0].exit_reason, "sub_diff");
        assert_eq!(t4[0].exit_price, 110.0);
        assert_eq!(t4[1].exit_reason, "eod");
        assert_eq!(t4[1].entry_price, 105.0, "接回点重锚");
        assert!((t4[0].shares - t4[1].shares).abs() < 1e-12, "如数接回股数守恒");
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
    }

    #[test]
    fn sub_opens_on_consolidation_div_and_closes_on_buy3() {
        // 盘背卖（div Consolidation×Up @3）开 → confirmed Buy3@3 回补。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_div(bar(110.0), 3, div_ev(Direction::Up)));
        bars.push(with_ev(bar(104.0), 3, ev(BspClass::Buy3)));
        bars.push(bar(104.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[4], 1, "盘背卖词汇分量");
        assert_eq!(r.n_sub_restores_by_ladder[4], 1, "Buy3 回补词汇分量");
        assert!((r.sub_net_cash_by_ladder[4] - 750.0 * 6.0).abs() < 1e-6);
    }

    #[test]
    fn hinge_escalates_to_layer_exit_on_k_level_sellpt() {
        // 044:44 铰链：短差在外 + 本层卖点 ⇒ 升级为减仓。变现价 = 短差卖出
        // 价 110（非升级 bar 的 120）——身份事后授予，记账以实际现金流。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(sellpt(bar(120.0), 4));
        bars.push(bar(130.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_escalates_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 0);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 1);
        assert_eq!(t4[0].exit_reason, "hinge_escalate");
        assert_eq!(t4[0].exit_price, 110.0);
        // 750×(110−100) = +7500
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
    }

    #[test]
    fn trend_phase_forces_buy_back() {
        // 短差在外 + 趋势相开始 ⇒ 强制回补（049:52 满仓义务）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(106.0)); // 趋势相起点（行注入）→ 强制回补 @106
        bars.push(bar(115.0));
        let entry_bar = SUB_COST_MIN_OBS as i64;
        let r = run_with_rows(
            bars,
            "fusion",
            Some(vec![(entry_bar + 2, 4, Direction::Up)]),
            Some(vec![(entry_bar + 2, 4, true)]),
        );
        assert_eq!(r.n_sub_phase_closes_by_ladder[4], 1);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4[0].exit_reason, "sub_diff");
        assert_eq!(t4[1].entry_price, 106.0);
        assert_eq!(t4[1].exit_reason, "eod");
        // +750×(110−106) 短差 + 750×(115−100−4) 持有
        assert!((r.sub_net_cash_by_ladder[4] - 3000.0).abs() < 1e-6);
    }

    #[test]
    fn k_level_buypt_closes_sub() {
        // 26课"任何的买点都是买点"：本层 k 级买点也回补在外短差。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(buypt(bar(103.0), 4));
        bars.push(bar(103.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_kbuy_restores_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 0);
    }

    #[test]
    fn trend_phase_blocks_sub_open() {
        // 趋势相中 k−1 卖证据不开短差（049:52 差价不能做）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(112.0));
        let r = run_with_rows(
            bars,
            "fusion",
            Some(vec![(0, 4, Direction::Up)]),
            Some(vec![(0, 4, true)]),
        );
        assert_eq!(r.n_sub_opens_by_ladder[4], 0);
    }

    #[test]
    fn cost_gate_noref_and_thin_amp_reject() {
        // 组1：θ₃ 无参照（仅 θ₄ warm-up）→ noref 保守拒。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(110.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_noref_rejects_by_ladder[4], 1);
        assert_eq!(r.n_sub_opens_by_ladder[4], 0);

        // 组2：θ₃ = 0.01% < 2×0.1% → 成本拒。
        let mut bars2: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 50.01);
                let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars2.push(buypt(bar(100.0), 4));
        bars2.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars2.push(bar(110.0));
        let r2 = run(bars2, "fusion_s");
        assert_eq!(r2.n_sub_cost_rejects_by_ladder[4], 1);
        assert_eq!(r2.n_sub_opens_by_ladder[4], 0);
    }

    #[test]
    fn bi_floor_blocks_segment_layer_sub() {
        // 层2 的 k−1 = bi（a0，77/78课）⇒ 结构上不开短差。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(110.0), 1, ev(BspClass::Sell1))); // 笔级"事件"（合成）
        bars.push(bar(110.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[2], 0);
        assert_eq!(r.n_sub_noref_rejects_by_ladder[2], 0, "未进入成本门——存在论拒绝在先");
    }

    #[test]
    fn restore_deferred_when_pool_drained_then_retries() {
        // 层4 全仓（仅 θ₄）→ 短差在外 → 层2 入场抽走部分 pool → 回补触发时
        // 价格上行 pool 不足 ⇒ 推迟；价格回落 ⇒ 补完（义务恒在）。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
                let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 4)); // w₄=0.75 → 750 股，pool=25000
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1))); // pool=107500
        bars.push(buypt(bar(100.0), 2)); // 层2 Flat→入场？θ₂ 无参照 → noref skip
        // 用层3 抽 pool：θ₃=1% 有参照，w₃=0.25 → 抽 ~26875
        bars.push(buypt(bar(100.0), 3));
        bars.push(with_ev(bar(120.0), 3, ev(BspClass::Buy1))); // cost=90000 > pool ⇒ 推迟
        bars.push(bar(95.0)); // cost=71250 ≤ pool ⇒ 补完
        bars.push(bar(95.0));
        let r = run(bars, "fusion_s");
        assert!(r.n_sub_restore_defer_bars >= 1, "资金不足推迟计数");
        assert_eq!(r.n_sub_restores_by_ladder[4], 1, "义务最终补完");
    }

    #[test]
    fn nav_reconstruction_invariant_round_trip() {
        // 价格不变的完整短差往返 ⇒ NAV 守恒（会计自检）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(100.0), 3, ev(BspClass::Sell1)));
        bars.push(with_ev(bar(100.0), 3, ev(BspClass::Buy1)));
        bars.push(sellpt(bar(100.0), 4));
        bars.push(bar(100.0));
        let r = run(bars, "fusion_s");
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9);
        // trade 行现金流重建 = final_nav（脚本 assert 的 Rust 侧前哨）
        let mut pool = INITIAL_CAPITAL;
        for t in &r.trades {
            pool += t.shares * (t.exit_price - t.entry_price);
        }
        assert!((pool - r.final_nav).abs() < 1e-9);
    }
}
