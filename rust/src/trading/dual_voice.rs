//! dual_voice — 双书独立逐仓 voice + 区间套递归到 a0（fusion_vd/vn/vdn；
//! 2026-06-12 任务）。
//!
//! 基座 = fusion_v（`unified_voice.rs` 同构复制，零接触纪律——在册路径
//! 不动）。两个正交轴，2×2 消融矩阵：
//!
//! ## 轴1 dual_book：每级别 voice 独立逐仓头寸（非净额合并）
//!
//! 在册 fusion_v 每层单一 `LayerState`（Long XOR Short），翻转断面 M=N
//! （开空量 = 平多股数，margin = 平多所得）——同级别多空互斥即净额合并
//! 的存在方式。dual_book=true 拆为两本独立书：
//!
//! - **多头书**（`LayerState`，在册结构）：买点入场（026:34）、卖点出清、
//!   MoveUp 停削（049:52）、osc 短差——多头循环逐字不变；
//! - **空头书**（`ShortState`，本地新增）：卖点开空（θ 配额独立逐仓，
//!   margin = 配额金额，1x 虚拟逐仓强平价 2×entry——fusion_btr 在册会计）、
//!   买点平空、MoveDown 停回补 [镜像推导]、MoveUp 强制平空。
//!
//! 同一卖点确认（门放行）同 bar 驱动两本书：多头出清 ∧ 空头开仓——
//! 两个独立会计动作，非单一翻转断面。**同级别多空可共存**（如：MoveDown
//! 中持空 + 买点驱动多头入场而空头停回补拦截 ⇒ {+Q, −Q'} 两腿并立，
//! 各自成本坐标保留）——这是 `bidirectional_nested_accounting.md` v2
//! §4.1"m>N 翻转位不可表示"的承载位补全：极性翻转不再经过测度零断面，
//! 而是两本书的独立开闭。物理执行 = 各层各书净额之和（账本层不折叠）。
//!
//! M=N 同股数定理（26:34）在 dual_book 下让位于配额对称性：空头开仓量 =
//! 该级别 θ 配额全量（与多头入场同口径——概念链第15环"级别 = 操作量"
//! 对两个方向对称成立）。同级别双书共存 = 双份配额占用（逐仓的本质：
//! 每腿独立保证金，互不净额抵消）。
//!
//! 门的双书分派（049:52 双相位）：
//! - Φ==MoveUp：多头停削 ∧ 空头**不开仓**（满仓义务窗口，空头前提不成立）
//!   ∧ 在册空头强制平空——三个分支同一读数；
//! - Φ==Osc：r2 双侧位置门（削减/开空 @ c≥ZG；回补/平空 @ c≤ZD）；
//! - Φ==MoveDown：削减/开空放行（逃命/顺势）∧ 停回补。
//! T2W 第二窗口为市场性质锁存，双书共享：卖侧触发驱动两本书后统一清。
//!
//! ## 轴2 nest_deep：区间套从特定级别递归到 a0（非一层截断）
//!
//! 在册 nest_forward 是一层截断（k 武装 → k−1 任意同侧证据触发），
//! `interval_nesting_forward_positioning.md` §4.2 显式登记"多层递归到
//! 最低级别"为开放轴。nest_deep=true 实装 0027:11 字面程序：
//!
//! > 某大级别的转折点，先找到其背驰段，然后在次级别图里，找出相应背驰段
//! > 在次级别里的背驰段，**将该过程反复进行下去，直到最低级别**。
//!
//! fire(k, side) ⟺ win[k] 活动 ∧ ∀j∈[FIRST_BSP_LADDER, k)：win[j] 同侧
//! 活动（每个次级别都在背驰段内 = 区间嵌套贯通）∧ bi 层同侧方向翻转沿
//! （磁带最低结构词汇；bi 翻转定义在 a0 K 线序列上——"直到最低级别"的
//! 严格可达形式）。退化一致性：k==FIRST_BSP_LADDER 时中间层集合空，判据
//! ≡ 在册一层截断的 bi 分支——递归是在册机制的严格扩张非替换。
//! 否定词汇不变（027:25 逆否：各层窗口各自破极值作废——链上任意层破 ⇒
//! 贯通失败，自然承载"区间套破裂"）。
//!
//! ## 守卫（O0≡P5 先例）
//!
//! dual_book=false ∧ nest_deep=false ⟹ 与 fusion_v 逐位等价（单测守卫
//! `guard_baseline_bit_exact`）。在册 fusion_v 入口零接触。
//!
//! ## 诚实声明（090号）
//!
//! - a0（bar 层，ladder 0）无结构事件行——递归下界 = bi 层翻转沿（其
//!   定义域是 a0 序列）。"递归到 a0 的 K 线本身"无磁带词汇，不声明。
//! - 空头载体 = 1x 虚拟逐仓（在册会计）；真实载体（认沽等）是部署层动作。
//! - MoveUp 强制平空后多头书 Flat 时同 bar enter_or_defer（049:52 满仓
//!   义务与在册翻多对齐——义务一致性，最小差分）。
//! - 同级别双书共存时无强制对冲平仓——共存的盈亏由各书出口规则各自裁决，
//!   不引入"净额归零"特例（净额思维正是本轴否定的对象）。

use super::center_book::CenterBook;
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS,
    MIN_FILL_FRAC,
};
use super::positional_fusion::PhaseView;
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use super::unified_osc::{OscLayer, OscOut};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 区间套定位窗口（unified_voice::NestWin 同构；本地定义零接触）。
#[derive(Debug, Clone, Copy)]
struct NestWin {
    extreme: f64,
    cs: Option<i64>,
}

/// 空头书状态（dual_book 轴专属；1x 虚拟逐仓——fusion_btr 在册会计同构，
/// 与多头书 `LayerState` 并立非互斥）。
#[derive(Debug, Clone, Copy, PartialEq)]
enum ShortState {
    Flat,
    Short {
        entry_bar: i64,
        entry_price: f64,
        units: f64,
        weight: f64,
        margin: f64,
    },
}

/// 次级别证据（一层截断词汇；unified_voice::nest_sub_evidence 逐字同构）。
fn nest_sub_evidence(
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

/// NAV：pool + 多头书在市权益 + 在册 Short 层权益 + 空头书权益
/// （虚拟逐仓口径 margin + units×(B_s − c)；osc 在外腿所得已在 pool）。
fn nav_dv(
    layers: &[LayerState; MAX_LADDER],
    shorts: &[ShortState; MAX_LADDER],
    oscs: &[Option<OscOut>; MAX_LADDER],
    cash: f64,
    c: f64,
    floor: usize,
) -> f64 {
    let mut v = cash;
    for k in floor..MAX_LADDER {
        if let ShortState::Short {
            entry_price,
            units,
            margin,
            ..
        } = shorts[k]
        {
            v += margin + units * (entry_price - c);
        }
        if oscs[k].is_some() {
            continue;
        }
        match layers[k] {
            LayerState::Long { shares, .. } => v += shares * c,
            LayerState::Short {
                entry_price,
                units,
                margin,
                ..
            } => {
                v += margin + units * (entry_price - c);
            }
            _ => {}
        }
    }
    v
}

/// 主入口（`PolarityMode::DualVoice` 经 `run_positional` 分派至此）。
/// 2×2 消融矩阵：fusion_vd = (true,false) / fusion_vn = (false,true) /
/// fusion_vdn = (true,true)；(false,false) = bit-exact 守卫臂（不暴露
/// parse，仅单测）。
pub(crate) fn run_dual_voice(
    tape: &SignalTape,
    floor_ladder: usize,
    dual_book: bool,
    nest_deep: bool,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "dual_voice 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("dual_voice 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !tape.has_div_events() {
        return Err(
            "dual_voice 要求背驰磁带——nest 次级别证据 + osc 词汇的结构分量，\
             缺行即词汇残缺（不静默降级）"
                .to_string(),
        );
    }
    if !(tape.has_trend_rows() && tape.has_dir_rows()) {
        return Err(
            "dual_voice 要求磁带 trend_flips + dir_flips 行——freeze_d 递归\
             传导读数无行即无数据基础"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut shorts: [ShortState; MAX_LADDER] = [ShortState::Flat; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);
    let mut osc_layer = OscLayer::new();

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];

    let mut nest_sell: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_fired: std::collections::HashMap<(usize, bool, i64), i64> =
        std::collections::HashMap::new();

    let mut t2w_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
    let mut t2w_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];

        while flip_ptr < flips.len() && flips[flip_ptr].0 == i as i64 {
            let (_, lad, dir) = flips[flip_ptr];
            dir_state[lad as usize] = Some(dir);
            flip_edge[lad as usize] = Some(dir);
            flip_ptr += 1;
        }
        while tflip_ptr < tflips.len() && tflips[tflip_ptr].0 == i as i64 {
            let (_, lad, is_trend) = tflips[tflip_ptr];
            trend_state[lad as usize] = is_trend;
            tflip_ptr += 1;
        }

        // ── 市场性质：中枢账本 + 振幅参照 + osc ③门参照 + candidate 否定 ──
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);
        osc_layer.observe_refs(&book, &dir_state);
        for lad in FIRST_BSP_LADDER..MAX_LADDER {
            book.negate_pending_departure(lad, c);
        }

        // ── 区间套定位窗口（武装/否定/让位在册同构；触发判据按 nest_deep
        //    二分：一层截断 vs 递归链贯通到 a0 端 bi 翻转沿）──
        let mut nf_sell = [false; MAX_LADDER];
        let mut nf_buy = [false; MAX_LADDER];
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
                    let win = if sellside {
                        &mut nest_sell[k]
                    } else {
                        &mut nest_buy[k]
                    };
                    if e.confirmed {
                        *win = None;
                        if let Some(cs) = e.cs {
                            if let Some(fb) = nest_fired.remove(&(k, sellside, cs)) {
                                res.nest_lead_bars_sum += (i as i64 - fb) as u64;
                                res.nest_lead_n += 1;
                            }
                        }
                    } else {
                        let ext = win.map_or(e.price, |w| {
                            if sellside {
                                w.extreme.max(e.price)
                            } else {
                                w.extreme.min(e.price)
                            }
                        });
                        let cs = e.cs.or(win.and_then(|w| w.cs));
                        *win = Some(NestWin { extreme: ext, cs });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                }
            }
        }
        // 触发判定（武装/否定全层完成后统一判定——区间套贯通是同时性
        // 状态非顺序消费：bi 翻转沿到达时所有贯通链同 bar 同时触发，
        // 027课区间套定理的多级别同时显现；低层 fire 清窗不得击穿高层
        // 链检查，故先对快照判定再清窗）。
        // nest_deep：0027:11"反复进行下去直到最低级别"——全部中间层
        // [FIRST_BSP_LADDER, k) 窗口同侧活动（区间嵌套贯通）∧ bi 层同侧
        // 翻转沿（a0 端最低结构词汇）。k==FIRST_BSP_LADDER 时与一层截断
        // 逐位等值（退化一致性）。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            let sub = k - 1;
            if nest_sell[k].is_some() {
                nf_sell[k] = if nest_deep {
                    (FIRST_BSP_LADDER..k).all(|j| nest_sell[j].is_some())
                        && flip_edge[1] == Some(Direction::Down)
                } else {
                    nest_sub_evidence(sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub])
                };
            }
            if nest_buy[k].is_some() {
                nf_buy[k] = if nest_deep {
                    (FIRST_BSP_LADDER..k).all(|j| nest_buy[j].is_some())
                        && flip_edge[1] == Some(Direction::Up)
                } else {
                    nest_sub_evidence(sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub])
                };
            }
        }
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if nf_sell[k] {
                if let Some(w) = nest_sell[k].take() {
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if let Some(cs) = w.cs {
                        nest_fired.insert((k, true, cs), i as i64);
                    }
                }
            }
            if nf_buy[k] {
                if let Some(w) = nest_buy[k].take() {
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if let Some(cs) = w.cs {
                        nest_fired.insert((k, false, cs), i as i64);
                    }
                }
            }
        }

        // ── freeze_d 递归传导读数（fusion_v 部署形态 anc_freeze 恒 true）──
        let self_up = |k: usize| trend_state[k] && dir_state[k] == Some(Direction::Up);
        let self_dn = |k: usize| trend_state[k] && dir_state[k] == Some(Direction::Down);
        let freeze_up = |k: usize| (k..MAX_LADDER).any(self_up);
        let freeze_dn = |k: usize| (k..MAX_LADDER).any(self_dn);

        let phi = |k: usize| -> PhaseView {
            if freeze_up(k) {
                PhaseView::MoveUp
            } else if freeze_dn(k) {
                PhaseView::MoveDown
            } else {
                PhaseView::Osc
            }
        };
        for k in floor_ladder..MAX_LADDER {
            match phi(k) {
                PhaseView::MoveUp => res.freeze_up_bars_by_ladder[k] += 1,
                PhaseView::MoveDown => res.freeze_dn_bars_by_ladder[k] += 1,
                PhaseView::Osc => {}
            }
        }

        let r2_trim_blocked =
            |k: usize| phi(k) == PhaseView::Osc && book.alive(k).is_some_and(|lc| !(c >= lc.zg));
        let r2_restore_blocked =
            |k: usize| phi(k) == PhaseView::Osc && book.alive(k).is_some_and(|lc| !(c <= lc.zd));

        // T2W 否定（R20）。
        for k in floor_ladder..MAX_LADDER {
            if t2w_sell[k].is_some_and(|x| c > x) {
                t2w_sell[k] = None;
                res.n_t2w_negates_by_ladder[k] += 1;
            }
            if t2w_buy[k].is_some_and(|x| c < x) {
                t2w_buy[k] = None;
                res.n_t2w_negates_by_ladder[k] += 1;
            }
        }
        let conf_sell1_px = |k: usize| -> Option<f64> {
            evrows[k]
                .iter()
                .filter(|e| e.confirmed && e.class == BspClass::Sell1)
                .map(|e| e.price)
                .fold(None, |m: Option<f64>, p| Some(m.map_or(p, |x| x.max(p))))
        };
        let conf_buy1_px = |k: usize| -> Option<f64> {
            evrows[k]
                .iter()
                .filter(|e| e.confirmed && e.class == BspClass::Buy1)
                .map(|e| e.price)
                .fold(None, |m: Option<f64>, p| Some(m.map_or(p, |x| x.min(p))))
        };
        let conf_sell2 = |k: usize| {
            evrows[k]
                .iter()
                .any(|e| e.confirmed && e.class == BspClass::Sell2)
        };
        let conf_buy2 = |k: usize| {
            evrows[k]
                .iter()
                .any(|e| e.confirmed && e.class == BspClass::Buy2)
        };

        // 卖侧 T2W 消费标记（dual_book：A/A2 双书共享锁存，动作发生后统一
        // 清——单书路径在册逐分支清，不经过此数组）。
        let mut sell_consumed = [false; MAX_LADDER];

        // ── 阶段 A：多头书出场（停削/位置门）∨ osc 在外腿出口 ──
        for k in floor_ladder..MAX_LADDER {
            let LayerState::Long {
                entry_bar,
                entry_price,
                shares,
                weight,
                deferred_bars,
                partial,
            } = layers[k]
            else {
                continue;
            };
            res.held_bars_by_ladder[k] += 1;
            if osc_layer.outs[k].is_some() {
                osc_layer.step_exit(
                    k,
                    c,
                    i as i64,
                    sig,
                    nf_sell[k],
                    nf_buy[k],
                    &phi,
                    &book,
                    &mut layers,
                    &mut pool,
                    &mut res,
                );
                continue;
            }
            let t2w_fire = t2w_sell[k].is_some() && conf_sell2(k);
            let sell_trig = sig.sell_any.get(k) || nf_sell[k] || t2w_fire;
            if !sell_trig {
                continue;
            }
            let tp = phi(k) == PhaseView::MoveUp;
            if tp {
                res.n_trend_holds_by_ladder[k] += 1;
                if !self_up(k) {
                    res.n_anc_exempt_blocks_by_ladder[k] += 1;
                }
                if let Some(px) = conf_sell1_px(k) {
                    if t2w_sell[k].is_none() {
                        res.n_t2w_arms_by_ladder[k] += 1;
                    }
                    t2w_sell[k] = Some(t2w_sell[k].map_or(px, |x| x.max(px)));
                }
            } else if r2_trim_blocked(k) {
                res.n_r2_pos_blocks_by_ladder[k] += 1;
                if let Some(px) = conf_sell1_px(k) {
                    if t2w_sell[k].is_none() {
                        res.n_t2w_arms_by_ladder[k] += 1;
                    }
                    t2w_sell[k] = Some(t2w_sell[k].map_or(px, |x| x.max(px)));
                }
            } else {
                // 卖点确认且门放行——双书二分：
                // dual_book=false：在册翻转断面（②平旧→③开新，M=N）；
                // dual_book=true：多头书纯出清（开空由阶段 A2 空头书独立
                // 承载——同一卖点的两本书消费）。
                pool += shares * c;
                let reason = if sig.sell_any.get(k) {
                    "sellpt"
                } else if nf_sell[k] {
                    "nest_sell"
                } else {
                    "t2w_sell"
                };
                if reason == "t2w_sell" {
                    res.n_t2w_fires_by_ladder[k] += 1;
                }
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
                    exit_reason: reason,
                    polarity: Polarity::Long,
                });
                res.n_exits_by_ladder[k] += 1;
                if dual_book {
                    layers[k] = LayerState::Flat;
                    sell_consumed[k] = true;
                } else {
                    let margin = shares * c;
                    pool -= margin;
                    layers[k] = LayerState::Short {
                        entry_bar: i as i64,
                        entry_price: c,
                        units: shares,
                        weight,
                        margin,
                    };
                    res.n_flip_shorts_by_ladder[k] += 1;
                    t2w_sell[k] = None;
                }
            }
        }

        // ── 阶段 A2（dual_book 专属）：空头书开仓——卖点确认 ∧ 门放行 ⇒
        //    按 θ 配额独立逐仓开空（与多头书状态正交：多头 Flat 的层也可
        //    开空；多头被停削持有的层不开空——MoveUp 门同读数拦截）──
        if dual_book {
            let a2_nav = nav_dv(&layers, &shorts, &osc_layer.outs, pool, c, floor_ladder);
            let (a2_thetas, a2_total) = theta_weights(&depth_ref, floor_ladder);
            for k in floor_ladder..MAX_LADDER {
                if shorts[k] != ShortState::Flat {
                    continue;
                }
                let t2w_fire = t2w_sell[k].is_some() && conf_sell2(k);
                let sell_trig = sig.sell_any.get(k) || nf_sell[k] || t2w_fire;
                if !sell_trig {
                    continue;
                }
                if phi(k) == PhaseView::MoveUp {
                    // 满仓义务窗口：空头前提不成立（049:52）；T2W 武装与
                    // 多头停削分支同款（卖点动作被门拒的第二窗口）。
                    res.n_dual_short_open_blocks_by_ladder[k] += 1;
                    if let Some(px) = conf_sell1_px(k) {
                        if t2w_sell[k].is_none() {
                            res.n_t2w_arms_by_ladder[k] += 1;
                        }
                        t2w_sell[k] = Some(t2w_sell[k].map_or(px, |x| x.max(px)));
                    }
                    continue;
                }
                if r2_trim_blocked(k) {
                    res.n_dual_short_open_blocks_by_ladder[k] += 1;
                    if let Some(px) = conf_sell1_px(k) {
                        if t2w_sell[k].is_none() {
                            res.n_t2w_arms_by_ladder[k] += 1;
                        }
                        t2w_sell[k] = Some(t2w_sell[k].map_or(px, |x| x.max(px)));
                    }
                    continue;
                }
                // θ 配额逐仓开空：margin = 配额金额（多头 enter_or_defer
                // 同口径），units = margin/c（1x）；pool 部分充足按可用量
                // 开（partial 同构），< MIN_FILL_FRAC×目标 ⇒ 防尘埃跳过。
                let Some(t) = a2_thetas[k] else {
                    res.n_noref_skips_by_ladder[k] += 1;
                    continue;
                };
                let weight = t / a2_total;
                let target = a2_nav * weight;
                let margin = target.min(pool);
                if !(margin > 0.0) || margin < MIN_FILL_FRAC * target {
                    continue;
                }
                let units = margin / c;
                pool -= margin;
                shorts[k] = ShortState::Short {
                    entry_bar: i as i64,
                    entry_price: c,
                    units,
                    weight,
                    margin,
                };
                res.n_dual_short_opens_by_ladder[k] += 1;
                if t2w_fire {
                    res.n_t2w_fires_by_ladder[k] += 1;
                }
                sell_consumed[k] = true;
            }
            // 卖侧 T2W 统一清（双书任一动作发生 = 卖点已消费）。
            for k in floor_ladder..MAX_LADDER {
                if sell_consumed[k] {
                    t2w_sell[k] = None;
                }
            }
        }

        // ── 阶段 B'：osc 开腿（多头书短差机制，在册三门恒开）──
        if sig.sell_any.0 != 0 {
            for k in floor_ladder..MAX_LADDER {
                if osc_layer.outs[k].is_some() {
                    continue;
                }
                osc_layer.try_open(
                    k, c, i as i64, sig, true, true, &phi, &book, &depth_ref, &layers, &mut pool,
                    &mut res,
                );
            }
        }

        // ── 阶段 C：空头书出口（回款先行）→ 多头书入场/在册 Short 出口
        //    （ladder 降序——大级别优先拿配额）──
        let bar_nav = nav_dv(&layers, &shorts, &osc_layer.outs, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            // 空头书出口集（dual_book 专属；优先序：逐仓强平 → MoveUp
            // 强制平空 → 买点回补 ∧ 门）。平空不自动翻多——多头书由下方
            // 独立按买点词汇驱动（MoveUp 强制平空除外：满仓义务对齐在册，
            // 多头书 Flat 时同 bar enter_or_defer）。
            if let ShortState::Short {
                entry_bar,
                entry_price,
                units,
                weight,
                margin,
            } = shorts[k]
            {
                res.short_held_bars_by_ladder[k] += 1;
                let equity_k = margin + units * (entry_price - c);
                let cover = |exit_price: f64,
                             exit_reason: &'static str,
                             pool: &mut f64,
                             res: &mut PositionalResult| {
                    *pool += margin + units * (entry_price - exit_price);
                    res.short_net_cash_by_ladder[k] += units * (entry_price - exit_price);
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: i as i64,
                        exit_price,
                        shares: units,
                        weight_at_entry: weight,
                        deferred_bars: 0,
                        partial: false,
                        exit_reason,
                        polarity: Polarity::Short,
                    });
                    res.n_exits_by_ladder[k] += 1;
                };
                let t2w_bfire = t2w_buy[k].is_some() && conf_buy2(k);
                if equity_k <= 0.0 {
                    cover(2.0 * entry_price, "short_liquidated", &mut pool, &mut res);
                    res.n_short_liquidations_by_ladder[k] += 1;
                    shorts[k] = ShortState::Flat;
                    t2w_buy[k] = None;
                } else if phi(k) == PhaseView::MoveUp {
                    cover(c, "cover_moveup", &mut pool, &mut res);
                    res.n_short_moveup_covers_by_ladder[k] += 1;
                    shorts[k] = ShortState::Flat;
                    t2w_buy[k] = None;
                    // 满仓义务（049:52）：多头书 Flat ⇒ 同 bar 回复多头
                    // （与在册翻多对齐——义务一致性声明见模块头）。
                    if layers[k] == LayerState::Flat {
                        layers[k] = enter_or_defer(
                            k,
                            i as i64,
                            i as i64,
                            c,
                            bar_nav,
                            &thetas,
                            theta_total,
                            &mut pool,
                            &mut res,
                        );
                    }
                } else if sig.buy_any.get(k) || nf_buy[k] || t2w_bfire {
                    if phi(k) == PhaseView::MoveDown {
                        res.n_short_trend_holds_by_ladder[k] += 1;
                        if let Some(px) = conf_buy1_px(k) {
                            if t2w_buy[k].is_none() {
                                res.n_t2w_arms_by_ladder[k] += 1;
                            }
                            t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                        }
                    } else if r2_restore_blocked(k) {
                        res.n_short_r2_blocks_by_ladder[k] += 1;
                        if let Some(px) = conf_buy1_px(k) {
                            if t2w_buy[k].is_none() {
                                res.n_t2w_arms_by_ladder[k] += 1;
                            }
                            t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                        }
                    } else {
                        let reason = if sig.buy_any.get(k) {
                            "cover_buypt"
                        } else if nf_buy[k] {
                            "cover_nest"
                        } else {
                            "cover_t2w"
                        };
                        if reason == "cover_t2w" {
                            res.n_t2w_fires_by_ladder[k] += 1;
                        }
                        cover(c, reason, &mut pool, &mut res);
                        res.n_short_covers_by_ladder[k] += 1;
                        shorts[k] = ShortState::Flat;
                        t2w_buy[k] = None;
                    }
                }
            }
            // 多头书入场 / 在册单书 Short 出口。
            match layers[k] {
                LayerState::Flat => {
                    if sig.buy_any.get(k) || nf_buy[k] {
                        layers[k] = enter_or_defer(
                            k,
                            i as i64,
                            i as i64,
                            c,
                            bar_nav,
                            &thetas,
                            theta_total,
                            &mut pool,
                            &mut res,
                        );
                    }
                }
                LayerState::Pending { confirm_bar } => {
                    if sig.sell_any.get(k) || nf_sell[k] {
                        res.n_pending_cancels_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    } else {
                        layers[k] = enter_or_defer(
                            k,
                            confirm_bar,
                            i as i64,
                            c,
                            bar_nav,
                            &thetas,
                            theta_total,
                            &mut pool,
                            &mut res,
                        );
                    }
                }
                LayerState::Armed { .. } => {
                    unreachable!("dual_voice 无 ARMED 相位——confirmed 事件直接消费")
                }
                LayerState::Long { .. } => {}
                // 在册单书翻转架构的 Short 层（dual_book=false 专属；
                // unified_voice 逐字同构）。
                LayerState::Short {
                    entry_bar,
                    entry_price,
                    units,
                    weight,
                    margin,
                } => {
                    debug_assert!(!dual_book, "dual_book 下多头书永不为 Short");
                    res.short_held_bars_by_ladder[k] += 1;
                    let equity_k = margin + units * (entry_price - c);
                    let cover = |exit_price: f64,
                                 exit_reason: &'static str,
                                 pool: &mut f64,
                                 res: &mut PositionalResult| {
                        *pool += margin + units * (entry_price - exit_price);
                        res.short_net_cash_by_ladder[k] += units * (entry_price - exit_price);
                        res.trades.push(LayerTrade {
                            ladder: k as u8,
                            entry_bar,
                            entry_price,
                            exit_bar: i as i64,
                            exit_price,
                            shares: units,
                            weight_at_entry: weight,
                            deferred_bars: 0,
                            partial: false,
                            exit_reason,
                            polarity: Polarity::Short,
                        });
                        res.n_exits_by_ladder[k] += 1;
                    };
                    let t2w_bfire = t2w_buy[k].is_some() && conf_buy2(k);
                    if equity_k <= 0.0 {
                        cover(2.0 * entry_price, "short_liquidated", &mut pool, &mut res);
                        res.n_short_liquidations_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                        t2w_buy[k] = None;
                    } else if phi(k) == PhaseView::MoveUp {
                        cover(c, "cover_moveup", &mut pool, &mut res);
                        res.n_short_moveup_covers_by_ladder[k] += 1;
                        layers[k] = enter_or_defer(
                            k,
                            i as i64,
                            i as i64,
                            c,
                            bar_nav,
                            &thetas,
                            theta_total,
                            &mut pool,
                            &mut res,
                        );
                        t2w_buy[k] = None;
                    } else if sig.buy_any.get(k) || nf_buy[k] || t2w_bfire {
                        if phi(k) == PhaseView::MoveDown {
                            res.n_short_trend_holds_by_ladder[k] += 1;
                            if let Some(px) = conf_buy1_px(k) {
                                if t2w_buy[k].is_none() {
                                    res.n_t2w_arms_by_ladder[k] += 1;
                                }
                                t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else if r2_restore_blocked(k) {
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                            if let Some(px) = conf_buy1_px(k) {
                                if t2w_buy[k].is_none() {
                                    res.n_t2w_arms_by_ladder[k] += 1;
                                }
                                t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else {
                            let reason = if sig.buy_any.get(k) {
                                "cover_buypt"
                            } else if nf_buy[k] {
                                "cover_nest"
                            } else {
                                "cover_t2w"
                            };
                            if reason == "cover_t2w" {
                                res.n_t2w_fires_by_ladder[k] += 1;
                            }
                            cover(c, reason, &mut pool, &mut res);
                            res.n_short_covers_by_ladder[k] += 1;
                            layers[k] = enter_or_defer(
                                k,
                                i as i64,
                                i as i64,
                                c,
                                bar_nav,
                                &thetas,
                                theta_total,
                                &mut pool,
                                &mut res,
                            );
                            t2w_buy[k] = None;
                        }
                    }
                }
                LayerState::Gated { .. } => {
                    unreachable!("dual_voice 无 Gated——双书/字面翻空替代")
                }
            }
        }

        // 同级别双书共存观测（bar 尾状态：多头书 Long ∧ 空头书 Short——
        // 非净额合并的核心新现象，m>N 承载位可观测面）。
        if dual_book {
            for k in floor_ladder..MAX_LADDER {
                if matches!(layers[k], LayerState::Long { .. })
                    && matches!(shorts[k], ShortState::Short { .. })
                {
                    res.dual_both_held_bars_by_ladder[k] += 1;
                }
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全书收口（空头书/在册 Short 层按市价平空，跳穿逐仓界按强平
    //    价记账；多头书在册收口）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        if let ShortState::Short {
            entry_bar,
            entry_price,
            units,
            weight,
            margin,
        } = shorts[k]
        {
            let equity_k = margin + units * (entry_price - last_close);
            let (exit_price, exit_reason) = if equity_k <= 0.0 {
                (2.0 * entry_price, "short_liquidated")
            } else {
                (last_close, "eod")
            };
            pool += margin + units * (entry_price - exit_price);
            res.short_net_cash_by_ladder[k] += units * (entry_price - exit_price);
            if exit_reason == "short_liquidated" {
                res.n_short_liquidations_by_ladder[k] += 1;
            }
            res.trades.push(LayerTrade {
                ladder: k as u8,
                entry_bar,
                entry_price,
                exit_bar: n as i64 - 1,
                exit_price,
                shares: units,
                weight_at_entry: weight,
                deferred_bars: 0,
                partial: false,
                exit_reason,
                polarity: Polarity::Short,
            });
            res.n_exits_by_ladder[k] += 1;
            shorts[k] = ShortState::Flat;
        }
        if let LayerState::Short {
            entry_bar,
            entry_price,
            units,
            weight,
            margin,
        } = layers[k]
        {
            let equity_k = margin + units * (entry_price - last_close);
            let (exit_price, exit_reason) = if equity_k <= 0.0 {
                (2.0 * entry_price, "short_liquidated")
            } else {
                (last_close, "eod")
            };
            pool += margin + units * (entry_price - exit_price);
            res.short_net_cash_by_ladder[k] += units * (entry_price - exit_price);
            if exit_reason == "short_liquidated" {
                res.n_short_liquidations_by_ladder[k] += 1;
            }
            res.trades.push(LayerTrade {
                ladder: k as u8,
                entry_bar,
                entry_price,
                exit_bar: n as i64 - 1,
                exit_price,
                shares: units,
                weight_at_entry: weight,
                deferred_bars: 0,
                partial: false,
                exit_reason,
                polarity: Polarity::Short,
            });
            res.n_exits_by_ladder[k] += 1;
            layers[k] = LayerState::Flat;
            continue;
        }
        let LayerState::Long {
            entry_bar,
            entry_price,
            shares,
            weight,
            deferred_bars,
            partial,
        } = layers[k]
        else {
            continue;
        };
        let (exit_bar, exit_price, sh) = match osc_layer.outs[k] {
            Some(osc) => (osc.sell_bar, osc.sell_price, osc.shares),
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
            polarity: Polarity::Long,
        });
        res.n_exits_by_ladder[k] += 1;
        layers[k] = LayerState::Flat;
        osc_layer.outs[k] = None;
    }
    res.final_nav = pool;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::config::SUB_COST_MIN_OBS;
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    fn bar(close: f64) -> BarSig {
        BarSig {
            close,
            max_ladder: 5,
            ..Default::default()
        }
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

    fn warmup(lad: usize) -> Vec<BarSig> {
        (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), lad, 10 + j, 50.0, 51.0);
                if j == 0 {
                    with_empty_div(b)
                } else {
                    b
                }
            })
            .collect()
    }

    fn run_mode(
        bars: Vec<BarSig>,
        dir_flips: Vec<(i64, u8, Direction)>,
        trend_flips: Vec<(i64, u8, bool)>,
        mode: PolarityMode,
    ) -> PositionalResult {
        let t = SignalTape {
            bars,
            dir_flips: Some(dir_flips),
            trend_flips: Some(trend_flips),
            ..Default::default()
        };
        run_positional(&t, 2, mode).unwrap()
    }

    #[test]
    fn parse_dual_voice() {
        assert_eq!(
            PolarityMode::parse("fusion_vd"),
            Some(PolarityMode::DualVoice {
                dual_book: true,
                nest_deep: false
            })
        );
        assert_eq!(
            PolarityMode::parse("fusion_vn"),
            Some(PolarityMode::DualVoice {
                dual_book: false,
                nest_deep: true
            })
        );
        assert_eq!(
            PolarityMode::parse("fusion_vdn"),
            Some(PolarityMode::DualVoice {
                dual_book: true,
                nest_deep: true
            })
        );
    }

    /// 守卫（O0≡P5 先例）：(false,false) 与 fusion_v 逐位等价——翻转/
    /// 强平/T2W/freeze 多场景磁带逐一对比 final_nav + trades 形状。
    #[test]
    fn guard_baseline_bit_exact() {
        let scenarios: Vec<(Vec<BarSig>, Vec<(i64, u8, Direction)>, Vec<(i64, u8, bool)>)> = {
            let w = SUB_COST_MIN_OBS as i64;
            vec![
                // 翻空 + 平空翻多
                (
                    {
                        let mut bars = warmup(2);
                        bars.push(buypt(bar(100.0), 2));
                        bars.push(sellpt(bar(100.0), 2));
                        bars.push(buypt(bar(40.0), 2));
                        bars.push(bar(40.0));
                        bars
                    },
                    vec![],
                    vec![],
                ),
                // 强平
                (
                    {
                        let mut bars = warmup(2);
                        bars.push(buypt(bar(100.0), 2));
                        bars.push(sellpt(bar(100.0), 2));
                        bars.push(bar(230.0));
                        bars.push(bar(230.0));
                        bars
                    },
                    vec![],
                    vec![],
                ),
                // freeze 停削 + 翻 Osc 后翻空
                (
                    {
                        let mut bars = warmup(2);
                        bars.push(buypt(bar(100.0), 2));
                        bars.push(sellpt(bar(110.0), 2));
                        bars.push(sellpt(bar(110.0), 2));
                        bars.push(bar(105.0));
                        bars
                    },
                    vec![(w + 1, 3, Direction::Up)],
                    vec![(w + 1, 3, true), (w + 2, 3, false)],
                ),
                // MoveDown 停回补
                (
                    {
                        let mut bars = warmup(2);
                        bars.push(buypt(bar(100.0), 2));
                        bars.push(sellpt(bar(90.0), 2));
                        bars.push(buypt(bar(40.0), 2));
                        bars.push(buypt(bar(40.0), 2));
                        bars.push(bar(40.0));
                        bars
                    },
                    vec![(w, 2, Direction::Down)],
                    vec![(w, 2, true), (w + 3, 2, false)],
                ),
            ]
        };
        for (idx, (bars, df, tf)) in scenarios.into_iter().enumerate() {
            let a = run_mode(
                bars.clone(),
                df.clone(),
                tf.clone(),
                PolarityMode::UnifiedVoice { anc_freeze: true },
            );
            let b = run_mode(
                bars,
                df,
                tf,
                PolarityMode::DualVoice {
                    dual_book: false,
                    nest_deep: false,
                },
            );
            assert!(
                (a.final_nav - b.final_nav).abs() < 1e-9,
                "场景{idx}: nav 漂移 {} vs {}",
                a.final_nav,
                b.final_nav
            );
            assert_eq!(a.trades.len(), b.trades.len(), "场景{idx}: trades 数漂移");
            for (ta, tb) in a.trades.iter().zip(b.trades.iter()) {
                assert_eq!(
                    (
                        ta.ladder,
                        ta.entry_bar,
                        ta.exit_bar,
                        ta.exit_reason,
                        ta.polarity
                    ),
                    (
                        tb.ladder,
                        tb.entry_bar,
                        tb.exit_bar,
                        tb.exit_reason,
                        tb.polarity
                    ),
                    "场景{idx}: trade 行漂移"
                );
            }
        }
    }

    #[test]
    fn dual_book_sell_opens_short_without_flip() {
        // 卖点确认（Osc 高位）⇒ 多头书出清 ∧ 空头书独立开空（θ 配额，
        // 非 M=N 翻转）；买点 @≤ZD ⇒ 平空 ∧ 多头书独立入场。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 2));
        bars.push(buypt(bar(40.0), 2));
        bars.push(bar(40.0));
        let r = run_mode(
            bars,
            vec![],
            vec![],
            PolarityMode::DualVoice {
                dual_book: true,
                nest_deep: false,
            },
        );
        assert_eq!(r.n_flip_shorts_by_ladder[2], 0, "双书无翻转断面");
        assert_eq!(r.n_dual_short_opens_by_ladder[2], 1, "空头书独立开仓");
        assert_eq!(r.n_short_covers_by_ladder[2], 1, "买点平空");
        assert_eq!(r.n_entries_by_ladder[2], 2, "多头书两次入场");
        assert!(r.short_net_cash_by_ladder[2] > 0.0, "下跌段空头书收割");
    }

    /// 双层 warmup（层2+层3 各得 θ ⇒ weight 0.5/0.5——单层全配额会使
    /// 空头 margin 占满 pool、多头入场推迟，共存不可达）。层3 锚用
    /// Buy1 candidate（同样形成中枢供 θ，但不武装卖窗——Sell1 锚会经
    /// nest fire 在 warmup 期间触发层3 空头书开仓，吃光配额）。
    fn warmup2() -> Vec<BarSig> {
        (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let mut e3 = anchor_ev(1000 + j, 50.0, 51.0);
                e3.class = BspClass::Buy1;
                let b = with_ev(with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0), 3, e3);
                if j == 0 {
                    with_empty_div(b)
                } else {
                    b
                }
            })
            .collect()
    }

    #[test]
    fn dual_book_coexists_long_and_short() {
        // MoveDown 中：先开空（卖点削减放行），后买点 ⇒ 空头停回补拦截
        // ∧ 多头书独立入场（partial）⇒ 同级别多空共存（非净额合并的
        // 核心现象）。
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 多头入场
        bars.push(sellpt(bar(90.0), 2)); // MoveDown：出清 + 开空
        bars.push(buypt(bar(80.0), 2)); // MoveDown：空头停回补 + 多头入场 ⇒ 共存
        bars.push(bar(80.0));
        let r = run_mode(
            bars,
            vec![(w, 2, Direction::Down)],
            vec![(w, 2, true)],
            PolarityMode::DualVoice {
                dual_book: true,
                nest_deep: false,
            },
        );
        assert_eq!(r.n_dual_short_opens_by_ladder[2], 1);
        assert_eq!(r.n_short_trend_holds_by_ladder[2], 1, "停回补拦截");
        assert!(r.dual_both_held_bars_by_ladder[2] > 0, "同级别多空共存");
    }

    #[test]
    fn dual_book_moveup_blocks_short_open() {
        // freeze_up：多头停削 ∧ 空头书不开仓（满仓义务窗口同读数）。
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2));
        bars.push(bar(110.0));
        let r = run_mode(
            bars,
            vec![(w + 1, 3, Direction::Up)],
            vec![(w + 1, 3, true)],
            PolarityMode::DualVoice {
                dual_book: true,
                nest_deep: false,
            },
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "多头停削");
        assert_eq!(r.n_dual_short_opens_by_ladder[2], 0, "空头不开仓");
        assert_eq!(r.n_dual_short_open_blocks_by_ladder[2], 1, "开空门拦截");
    }

    #[test]
    fn nest_deep_requires_full_chain() {
        // k=3 窗口活动但层 2 无同侧窗口 ⇒ 链不贯通不触发；层 2 窗口武装
        // 后 bi 翻转沿到达 ⇒ 链贯通触发（0027:11 递归程序）。
        let w = SUB_COST_MIN_OBS as i64;
        let cand_sell = |lad: usize, px: f64, cs: i64| {
            let mut e = anchor_ev(cs, 50.0, 51.0);
            e.price = px;
            let mut b = bar(100.0);
            let rows = b
                .bsp_events
                .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
            rows[lad].push(e);
            b
        };
        // 场景 A：仅 k=3 武装 + bi 翻转 ⇒ 不触发（层 2 缺位）。
        let mut bars_a = warmup(2);
        bars_a.push(buypt(bar(100.0), 3));
        bars_a.push(cand_sell(3, 120.0, 99));
        bars_a.push(bar(100.0)); // bi 翻转 bar（行注入）
        bars_a.push(bar(100.0));
        let r_a = run_mode(
            bars_a,
            vec![(w + 2, 1, Direction::Down)],
            vec![],
            PolarityMode::DualVoice {
                dual_book: false,
                nest_deep: true,
            },
        );
        assert_eq!(r_a.n_nest_fire_sell_by_ladder[3], 0, "链不贯通不触发");
        // 场景 B：k=3 与层 2 同侧武装 + bi 翻转 ⇒ 触发。
        let mut bars_b = warmup(2);
        bars_b.push(buypt(bar(100.0), 3));
        bars_b.push(cand_sell(3, 120.0, 99));
        bars_b.push(cand_sell(2, 118.0, 98));
        bars_b.push(bar(100.0)); // bi 翻转 bar
        bars_b.push(bar(100.0));
        let r_b = run_mode(
            bars_b,
            vec![(w + 3, 1, Direction::Down)],
            vec![],
            PolarityMode::DualVoice {
                dual_book: false,
                nest_deep: true,
            },
        );
        assert_eq!(r_b.n_nest_fire_sell_by_ladder[3], 1, "链贯通触发");
    }

    #[test]
    fn nest_deep_floor_degenerates_to_shallow() {
        // k==FIRST_BSP_LADDER：递归判据 ≡ 一层截断 bi 分支（退化一致性）。
        let w = SUB_COST_MIN_OBS as i64;
        let make = |deep: bool| {
            let mut bars = warmup(2);
            bars.push(buypt(bar(100.0), 2));
            let mut e = anchor_ev(99, 50.0, 51.0);
            e.price = 120.0;
            bars.push(with_ev(bar(100.0), 2, e));
            bars.push(bar(100.0)); // bi 翻转 bar
            bars.push(bar(100.0));
            run_mode(
                bars,
                vec![(w + 2, 1, Direction::Down)],
                vec![],
                PolarityMode::DualVoice {
                    dual_book: false,
                    nest_deep: deep,
                },
            )
        };
        let shallow = make(false);
        let deep = make(true);
        assert_eq!(
            shallow.n_nest_fire_sell_by_ladder[2], deep.n_nest_fire_sell_by_ladder[2],
            "floor 层退化一致"
        );
        assert!((shallow.final_nav - deep.final_nav).abs() < 1e-9);
    }
}
