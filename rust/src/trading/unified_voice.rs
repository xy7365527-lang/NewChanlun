//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! unified_voice — 统一递归 voice FSM（fusion_v；2026-06-12 任务）。
//!
//! 谱系位置：`analysis/unified_recursive_voice_fsm_design.md` v2 §10 Phase 1
//! 的接线实装。零概念开关——唯一参数 = a0（磁带粒度，引擎层）+
//! floor_ladder（BSP 承载下限，结构常数非策略参数）。在册 Fusion 路径
//! 零接触（新入口；GH2 先例）。
//!
//! ## 五轴自动化（flag → 走势结构读数）
//!
//! | 在册 flag 轴 | fusion_v 替代读数 | 原文锚 |
//! |-------------|------------------|--------|
//! | trend_hold / trend_scope | freeze_d 递归传导：freeze_up(k) = self_up(k) ∨ anc_up(k)（275号局部依赖展开式；26:80 豁免下沉） | 049:52 / 026:80 |
//! | short_mask / short_ghost | R14 字面翻空全层（卖点确认 ⇒ voice LONG→SHORT 真实开空，概念链第19环字面实现："走势终完美对涨跌都成立 ⇒ 永远有方向"） | 038:36 镜像 [镜像推导] |
//! | nest_forward | 恒开（candidate 武装+次级别证据触发，双侧——翻空时点 = 区间套定位，不等 confirmed） | 027课 / 038:258 |
//! | osc / h1_freeze | 统一 osc 三门恒开（相位/振幅/g2 强弱 + 49:68 candidate 冻结——Δf 6/8 非负在册） | 035:30 / 093:26 / 049:68 |
//! | r2_gate | 双侧位置门恒开，域 = Φ(k)==Osc（削减@c≥ZG / 回补@c≤ZD） | 049:52 / 049:64 |
//! | （无） | R18-20 T2W 第二翻转窗口（confirmed type1 被门拒 ⇒ 锁存；type2 同侧 ∧ 门放行 ⇒ 重试；越极值 ⇒ 清） | 053:28 / 086:80 |
//!
//! R4（49:54 双侧出口）已删除——概念链审计（`analysis/
//! unified_voice_from_concept_chain.md`）判定其为链外机制（从走势终完美
//! 的概念运动推不出来），且 L3 6/8 为负（BTC −963pp）。删除后 Φ 直读
//! freeze，无出口分支。
//!
//! ## Φ 三值化（公共读数——矛盾2 的承载）
//!
//! ```text
//! phi(k) = MoveUp   if freeze_up(k)
//!          MoveDown if freeze_dn(k)   [镜像推导]
//!          Osc      otherwise
//! ```
//!
//! 全部行为从 Φ 读：停削（MoveUp）、停回补（MoveDown）、osc 路由①门
//! （≠Osc skip）、osc 满仓义务（==MoveUp）、44课铰链抑制（==MoveUp）、
//! r2 双侧门域（==Osc）。MoveUp 优先于 MoveDown（多头书的满仓义务优先，
//! 049:52；自层与祖先层读数冲突时保护多头侧）。
//!
//! ## 翻转 = 会计断面（cascade settle 四步序）
//!
//! 卖点确认（confirmed 掩码 ∨ nest 正向触发 ∨ T2W 第二窗口）且门放行 ⇒
//! ②平旧（削减 trade 行，实际变现价）→ ③开新（同 bar 同价开空，M = N
//! 同股数定理：units = 平多股数，26:34 单位数量纲；1x 虚拟逐仓 margin =
//! units×c = 平多所得 ⇒ 翻转 bar pool 净流转 0，断面无渗漏）。①子树
//! 结算由 osc step_exit 铰链承载（osc 在外时主层卖点先到 = hinge
//! escalate，044:44 在册；翻转分支前置守卫保证翻转 bar 本层无在外子
//! voice）；④子腿重开：空头书的子 voice 短差与 fusion_btr 在册同款
//! 不承载（osc try_open 的 Long 要求自然排除）。买点确认 ⇒ 镜像断面
//! （平空 → enter_or_defer 翻多重配额）。翻空仓位 = 该级别配额全量
//! （概念链第15环：级别 = 操作量——M = N 即该级别在册全量）。
//!
//! ## 诚实声明（090号）
//!
//! - 杠杆三元组（L_max = 1/(D_struct+mm)）**不在本实装**——设计 v2
//!   Phase 1 范围外（逐 bar D_struct 影子价格是独立工程），列开放轴。
//! - 空头载体 = 1x 虚拟逐仓（fusion_btr 在册会计，强平价 2×entry）；
//!   认沽期权等真实载体选择是部署层动作（026:36），不入 FSM。
//! - R18 的"candidate 永不 confirmed（小转大 orphan）"武装支未实装——
//!   第一形态仅门拒支；orphan 支列开放轴（M13-e 判据先裁决可达性）。

use super::center_book::CenterBook;
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS,
};
use super::positional_fusion::PhaseView;
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use super::unified_osc::{OscLayer, OscOut};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 区间套正向定位窗口（positional_fusion::NestWin 同构；恒开故本地定义，
/// 避免提升在册私有类型的可见性——零接触纪律）。
#[derive(Debug, Clone, Copy)]
struct NestWin {
    extreme: f64,
    cs: Option<i64>,
}

/// 次级别证据（positional_fusion::nest_sub_evidence 逐字同构）。
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

/// NAV：pool + 各层在市权益（osc 在外腿所得已在 pool ⇒ 跳过股数计值；
/// 空头层权益 = margin + units×(B_s − c)，虚拟逐仓口径——会计文档 §4.3
/// NAV 恒等式；浮亏可为负，强平在阶段 C 收口。fusion_v 无 SubOut）。
fn nav_v(
    layers: &[LayerState; MAX_LADDER],
    oscs: &[Option<OscOut>; MAX_LADDER],
    cash: f64,
    c: f64,
    floor: usize,
) -> f64 {
    let mut v = cash;
    for k in floor..MAX_LADDER {
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

/// 主入口（`PolarityMode::UnifiedVoice` 经 `run_positional` 分派至此）。
/// 零 flag——磁带能力是统一配置的前提（全词汇要求，缺行即拒绝）。
/// anc_freeze 是唯一预注册消融臂（freeze 递归传导，false ⇒ 仅自层
/// 对照；部署形态 fusion_v 恒 true）。R4/Gated 两轴已删除——前者链外
/// 机制（L3 6/8 负），后者被字面翻空替代，无"翻空还是 Gated"的选择。
pub(crate) fn run_unified_voice(
    tape: &SignalTape,
    floor_ladder: usize,
    anc_freeze: bool,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "fusion_v 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("fusion_v 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !tape.has_div_events() {
        return Err(
            "fusion_v 要求背驰磁带——nest 次级别证据 + osc 词汇的结构分量，\
             缺行即词汇残缺（不静默降级）"
                .to_string(),
        );
    }
    if !(tape.has_trend_rows() && tape.has_dir_rows()) {
        return Err(
            "fusion_v 要求磁带 trend_flips + dir_flips 行——freeze_d 递归传导\
             读数 = kind==Trend ∧ dir（17课趋势定义），无行即读数无数据基础"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);
    let mut osc_layer = OscLayer::new();

    // 行滚动状态（runner.rs 同构：稀疏翻转行 → 逐 bar 视图）。
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];

    // 区间套正向定位窗口（恒开）。
    let mut nest_sell: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_fired: std::collections::HashMap<(usize, bool, i64), i64> =
        std::collections::HashMap::new();

    // T2W 锁存（extreme = 被拒 type1 的事件价；R20 越极值清）。
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

        // ── 市场性质：中枢账本 + 振幅参照 + osc ③门参照 + candidate 离开
        //    窗口的价格否定（049:68 当下读法——osc h1 消费）──
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

        // ── 区间套正向定位（positional_fusion 逐字同构；恒开）──
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
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if nest_sub_evidence(sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_sell[k] = true;
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if let Some(cs) = w.cs {
                        nest_fired.insert((k, true, cs), i as i64);
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if nest_sub_evidence(sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_buy[k] = true;
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if let Some(cs) = w.cs {
                        nest_fired.insert((k, false, cs), i as i64);
                    }
                }
            }
        }

        // ── freeze_d 递归传导读数（275号局部依赖的展开式：
        //    freeze_d(k) = (trend ∧ dir==d 有利)(k) ∨ freeze_d(k+1)，
        //    归纳展开 ≡ self ∨ ∃祖先）──
        let self_up = |k: usize| trend_state[k] && dir_state[k] == Some(Direction::Up);
        let self_dn = |k: usize| trend_state[k] && dir_state[k] == Some(Direction::Down);
        let freeze_up = |k: usize| {
            if anc_freeze {
                (k..MAX_LADDER).any(self_up)
            } else {
                self_up(k)
            }
        };
        let freeze_dn = |k: usize| {
            if anc_freeze {
                (k..MAX_LADDER).any(self_dn)
            } else {
                self_dn(k)
            }
        };

        // ── Φ 三值化（公共读数；MoveUp 优先 = 多头书满仓义务优先）──
        let phi = |k: usize| -> PhaseView {
            if freeze_up(k) {
                PhaseView::MoveUp
            } else if freeze_dn(k) {
                PhaseView::MoveDown
            } else {
                PhaseView::Osc
            }
        };
        // 驻留观测（市场性质逐 bar）。
        for k in floor_ladder..MAX_LADDER {
            match phi(k) {
                PhaseView::MoveUp => res.freeze_up_bars_by_ladder[k] += 1,
                PhaseView::MoveDown => res.freeze_dn_bars_by_ladder[k] += 1,
                PhaseView::Osc => {}
            }
        }

        // 双侧位置门（049:52"在中枢上方仓位减少" / 049:64"在下方如数接回"）。
        // 域 = Φ(k)==Osc 精确（MoveDown 削减放行 = 逃命语义；MoveUp 回补
        // 走强制回复分支）。NaN 比较恒 false ⇒ 拦截（保守方向在册同构）。
        let r2_trim_blocked =
            |k: usize| phi(k) == PhaseView::Osc && book.alive(k).is_some_and(|lc| !(c >= lc.zg));
        let r2_restore_blocked =
            |k: usize| phi(k) == PhaseView::Osc && book.alive(k).is_some_and(|lc| !(c <= lc.zd));

        // T2W 否定（R20：close 越过锁存 extreme ⇒ 假 type1，清锁存）。
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
        // confirmed type1/type2 本 bar 提取（T2W 武装/触发词汇）。
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

        // ── 阶段 A：层级出场（停削/位置门/削减→Gated）/ osc 在外腿出口 ──
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
                // osc 在外腿出口集（满仓义务/铰链/回补）——Φ 读数透传，
                // nest 触发与 confirmed 同词汇地位双侧透传。
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
            // T2W 触发（R19）：武装 ∧ confirmed Sell2 ∧ 门放行（门在词汇
            // 之后一视同仁——tp/r2 拦截下方分支统一处理）。
            let t2w_fire = t2w_sell[k].is_some() && conf_sell2(k);
            let sell_trig = sig.sell_any.get(k) || nf_sell[k] || t2w_fire;
            if !sell_trig {
                continue;
            }
            let tp = phi(k) == PhaseView::MoveUp;
            if tp {
                // 049:52 趋势相停削（freeze_up 有效窗口）。
                res.n_trend_holds_by_ladder[k] += 1;
                if !self_up(k) {
                    // 祖先分量归因（26:80 豁免下沉；U4 可观测量）。
                    res.n_anc_exempt_blocks_by_ladder[k] += 1;
                }
                // R18 武装：confirmed type1 动作被 freeze 拒 ⇒ T2W 锁存。
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
                // 翻转断面（②平旧→③开新；概念链第19环字面实现：卖点
                // 确认 ⇒ voice LONG→SHORT）。M = N 同股数定理：units =
                // 平多股数（26:34 单位数量纲）；1x 虚拟逐仓 margin =
                // units×c = 平多所得 ⇒ 翻转 bar pool 净流转 0（断面无
                // 渗漏）。同 bar 同价、会计分两行 trade（段归属核算硬
                // 要求，不可合并）。翻空时点 = 触发词汇本身（nest 正向
                // 触发先于 confirmed 到达 ⇒ 区间套定位的时间坐标）。
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

        // ── 阶段 B'：osc 开腿（相位递归路由三门：①Φ==Osc ②振幅 H4
        //    ③g2 强弱 + 49:68 candidate 冻结；恒开。custody：Gated/在外
        //    层由 try_open 的 Long 要求自然排除）──
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

        // ── 阶段 C：入场/回复（ladder 降序——大级别优先拿配额）──
        let bar_nav = nav_v(&layers, &osc_layer.outs, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            match layers[k] {
                LayerState::Flat => {
                    // R0：树根入场（confirmed 买点 ∨ nest 正向触发）。
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
                    unreachable!("fusion_v 无 ARMED 相位——confirmed 事件直接消费")
                }
                LayerState::Long { .. } => {}
                // ── 空头书出口集（翻转断面的对偶面）。优先序：逐仓强平
                //    （物理事件）→ MoveUp 强制平空翻多（49:52 满仓义务
                //    镜像：空头前提消失）→ 买点回补翻多（MoveDown 停
                //    回补 / 空侧 R2 位置门 + T2W 第二窗口）──
                LayerState::Short {
                    entry_bar,
                    entry_price,
                    units,
                    weight,
                    margin,
                } => {
                    res.short_held_bars_by_ladder[k] += 1;
                    let equity_k = margin + units * (entry_price - c);
                    let cover = |exit_price: f64,
                                 exit_reason: &'static str,
                                 pool: &mut f64,
                                 res: &mut PositionalResult| {
                        // 1x 逐仓：损失上界 = margin（强平价记账 ⇒ 现金
                        // 流出恰为全部 margin，逐 trade 重建零渗漏）。
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
                        // 虚拟逐仓强平：1x 解析强平价 = 2×entry_price
                        // （margin = units×entry_price 的 equity=0 解）。
                        cover(2.0 * entry_price, "short_liquidated", &mut pool, &mut res);
                        res.n_short_liquidations_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                        t2w_buy[k] = None;
                    } else if phi(k) == PhaseView::MoveUp {
                        // R16：Φ 翻回 MoveUp ⇒ 强制平空翻多（049:52）。
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
                            // 49:52 镜像：中枢向下移动 ⇒ 停回补 [镜像推导]。
                            res.n_short_trend_holds_by_ladder[k] += 1;
                            if let Some(px) = conf_buy1_px(k) {
                                if t2w_buy[k].is_none() {
                                    res.n_t2w_arms_by_ladder[k] += 1;
                                }
                                t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else if r2_restore_blocked(k) {
                            // R15 位置门：c ≤ ZD 才回补（049:64 镜像）。
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                            if let Some(px) = conf_buy1_px(k) {
                                if t2w_buy[k].is_none() {
                                    res.n_t2w_arms_by_ladder[k] += 1;
                                }
                                t2w_buy[k] = Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else {
                            // 镜像断面：平空 → enter_or_defer 翻多重配额。
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
                    unreachable!("fusion_v 无 Gated——字面翻空替代（概念链第19环）")
                }
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层收口（空头层按市价平空，末 bar 跳穿逐仓界则按强平价
    //    记账——与阶段 C 强平同一会计口径；osc 在外腿悬置收口，所得已
    //    在 pool）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
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

    /// 高 ZG 锚（cs 固定）：c≥ZG 位置门可控。zd=50/zg=51 ⇒ c=100 恒过门。
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

    /// 部署形态。
    fn full_v() -> PolarityMode {
        PolarityMode::UnifiedVoice { anc_freeze: true }
    }

    fn run_v(
        bars: Vec<BarSig>,
        dir_flips: Vec<(i64, u8, Direction)>,
        trend_flips: Vec<(i64, u8, bool)>,
    ) -> PositionalResult {
        let t = SignalTape {
            bars,
            dir_flips: Some(dir_flips),
            trend_flips: Some(trend_flips),
            ..Default::default()
        };
        run_positional(&t, 2, full_v()).unwrap()
    }

    #[test]
    fn parse_fusion_v() {
        assert_eq!(PolarityMode::parse("fusion_v"), Some(full_v()));
        assert_eq!(
            PolarityMode::parse("fusion_v_self"),
            Some(PolarityMode::UnifiedVoice { anc_freeze: false })
        );
        // 已删除的消融臂（R4 链外机制 / Gated 落点）不再可解析。
        assert_eq!(PolarityMode::parse("fusion_v_nor4"), None);
        assert_eq!(PolarityMode::parse("fusion_v_flat"), None);
    }

    #[test]
    fn capability_guards() {
        // 无 trend/dir 行 ⇒ 拒绝（freeze 读数无数据基础）。
        let t = SignalTape {
            bars: warmup(2),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, full_v()).is_err());
        // 无 div 行 ⇒ 拒绝（nest/osc 词汇残缺）。
        let no_div: Vec<BarSig> = warmup(2)
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t2 = SignalTape {
            bars: no_div,
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, full_v()).is_err());
    }

    #[test]
    fn trim_flips_short_and_covers_on_buy() {
        // R14 字面翻空：卖点确认 ⇒ LONG→SHORT 真实开空（概念链第19环）；
        // 买点确认 ⇒ 平空翻多（镜像断面）。空头腿在下跌段挣钱。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(100.0), 2)); // 削减（Osc 相、c=100≥ZG=51）→ 开空 @100
        bars.push(buypt(bar(40.0), 2)); // 买点 @40 ≤ ZD=50 → 平空翻多
        bars.push(bar(40.0));
        let r = run_v(bars, vec![], vec![]);
        assert_eq!(r.n_flip_shorts_by_ladder[2], 1, "削减翻空");
        assert_eq!(r.n_short_covers_by_ladder[2], 1, "买点平空翻多");
        assert_eq!(r.n_entries_by_ladder[2], 2);
        // 空头腿净现金 = units×(100−40) > 0（下跌段直接收割）。
        assert!(r.short_net_cash_by_ladder[2] > 0.0);
        let sh: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.polarity == Polarity::Short)
            .collect();
        assert_eq!(sh.len(), 1);
        assert_eq!(sh[0].exit_reason, "cover_buypt");
    }

    #[test]
    fn short_cover_blocked_above_zd() {
        // R15 空侧位置门：Osc 相中 c > ZD ⇒ 回补拦截（049:64 镜像）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 2)); // → Short @100
        bars.push(buypt(bar(80.0), 2)); // 80 > ZD=50 ⇒ 拦截（继续持空）
        bars.push(bar(80.0));
        let r = run_v(bars, vec![], vec![]);
        assert_eq!(r.n_short_r2_blocks_by_ladder[2], 1, "ZD 上方回补拦截");
        assert_eq!(r.n_short_covers_by_ladder[2], 0);
    }

    #[test]
    fn short_liquidates_at_2x_entry() {
        // 1x 虚拟逐仓强平：c ≥ 2×entry ⇒ 层权益 ≤ 0，按强平价 2×entry
        // 记账（损失上界 = margin，确认滞后的结构税有界化）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 2)); // → Short @100，强平价 200
        bars.push(bar(230.0)); // 跳穿强平界
        bars.push(bar(230.0));
        let r = run_v(bars, vec![], vec![]);
        assert_eq!(r.n_short_liquidations_by_ladder[2], 1);
        let sh: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.polarity == Polarity::Short)
            .collect();
        assert_eq!(sh.len(), 1);
        assert_eq!(sh[0].exit_price, 200.0, "强平价 = 2×entry");
        // 全 margin 蒸发：NAV = 0（损失有界 = margin）。
        assert!(r.final_nav.abs() < 1e-6);
    }

    #[test]
    fn freeze_up_suppresses_trim_ancestor_component() {
        // freeze_d 递归传导：祖先层（3）Trend∧Up ⇒ 层2 停削（26:80 下沉）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 祖先 freeze ⇒ 停削
        bars.push(bar(110.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_v(
            bars,
            vec![(w + 1, 3, Direction::Up)],
            vec![(w + 1, 3, true)],
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "停削");
        assert_eq!(r.n_anc_exempt_blocks_by_ladder[2], 1, "祖先分量归因");
        assert_eq!(
            r.trades
                .iter()
                .filter(|t| t.ladder == 2 && t.exit_reason != "eod")
                .count(),
            0
        );
    }

    #[test]
    fn movedown_blocks_cover_until_flip() {
        // MoveDown 停回补（49:52 镜像）：freeze_dn 中买点拦截（满空仓
        // 义务）；翻 Osc 后平空翻多。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 入场
        bars.push(sellpt(bar(90.0), 2)); // MoveDown 中卖点削减放行（逃命）→ Short @90
        bars.push(buypt(bar(40.0), 2)); // freeze_dn ⇒ 停回补（持空吃下跌）
        bars.push(buypt(bar(40.0), 2)); // 翻 Osc（行注入）⇒ 平空翻多 @40≤ZD
        bars.push(bar(40.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_v(
            bars,
            vec![(w, 2, Direction::Down)],
            vec![(w, 2, true), (w + 3, 2, false)],
        );
        assert_eq!(r.n_short_trend_holds_by_ladder[2], 1, "MoveDown 停回补");
        assert_eq!(r.n_short_covers_by_ladder[2], 1, "翻 Osc 后平空翻多");
        // MoveDown 中削减不被停削/位置门拦（多头逃命语义）→ 翻空。
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "sellpt")
            .collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_price, 90.0);
        // 空头腿 @90→@40：净现金为正（MoveDown 持空 = 下跌段收割）。
        assert!(r.short_net_cash_by_ladder[2] > 0.0);
    }

    #[test]
    fn t2w_arms_on_rejected_type1_fires_on_type2() {
        // R18-R20：freeze 拦截 confirmed Sell1 ⇒ T2W 武装；freeze 退出后
        // confirmed Sell2 ⇒ 第二窗口削减（exit_reason="t2w_sell"）。
        let conf = |class: BspClass, px: f64| BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: None,
            zd: None,
            zg: None,
            price: px,
        };
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        // freeze_up 中 confirmed Sell1（不置 sell_any 掩码——T2W 锁存武装
        // 需要 sell_trig，此处用事件+掩码同 bar）。
        bars.push(sellpt(
            with_ev(bar(100.0), 2, conf(BspClass::Sell1, 120.0)),
            2,
        ));
        // freeze 退出（行注入）后 confirmed Sell2 到达（不置掩码）⇒ t2w 触发。
        bars.push(with_ev(bar(100.0), 2, conf(BspClass::Sell2, 100.0)));
        bars.push(bar(100.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_v(
            bars,
            vec![(w + 1, 2, Direction::Up)],
            vec![(w + 1, 2, true), (w + 2, 2, false)],
        );
        assert_eq!(r.n_t2w_arms_by_ladder[2], 1, "type1 被拒 ⇒ 武装");
        assert_eq!(r.n_t2w_fires_by_ladder[2], 1, "type2 第二窗口触发");
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "t2w_sell")
            .collect();
        assert_eq!(t2.len(), 1);
    }

    #[test]
    fn t2w_negated_on_extreme_break() {
        // R20：close 越过锁存 extreme ⇒ 清锁存，后续 type2 不触发。
        let conf = |class: BspClass, px: f64| BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: None,
            zd: None,
            zg: None,
            price: px,
        };
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(
            with_ev(bar(100.0), 2, conf(BspClass::Sell1, 120.0)),
            2,
        ));
        bars.push(bar(130.0)); // 越过 extreme=120 ⇒ 清
        bars.push(with_ev(bar(100.0), 2, conf(BspClass::Sell2, 100.0)));
        bars.push(bar(100.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_v(
            bars,
            vec![(w + 1, 2, Direction::Up)],
            vec![(w + 1, 2, true), (w + 2, 2, false)],
        );
        assert_eq!(r.n_t2w_negates_by_ladder[2], 1);
        assert_eq!(r.n_t2w_fires_by_ladder[2], 0, "清锁存后不触发");
    }

    #[test]
    fn nav_accounting_through_flip() {
        // 翻转断面会计：翻转 bar pool 净流转 0（M=N，margin = 平多所得）；
        // 持空期价格上涨 = 空头浮亏，eod 平空兑现。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 1000 股 @100
        bars.push(sellpt(bar(110.0), 2)); // 平多 @110（+10000）→ 开空 @110
        bars.push(bar(120.0)); // 空头浮亏 −10000
        bars.push(bar(120.0));
        let r = run_v(bars, vec![], vec![]);
        // 110000（平多后）− 10000（空头腿 110→120）= 100000。
        assert!((r.final_nav - 100_000.0).abs() < 1e-6, "翻转断面 NAV 守恒");
        assert!((r.short_net_cash_by_ladder[2] + 10_000.0).abs() < 1e-6);
    }
}
