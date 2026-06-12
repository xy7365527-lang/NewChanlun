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
//! | （无——v1 缺失） | R4 双侧出口：freeze 内新中枢形成 ⇒ 短差词汇局部重开 | 049:54 后半（双侧） |
//! | short_mask / short_ghost | R14 默认落点 Gated 全层（方向承诺+零暴露；M7-a 普适形态。真实空头=载体×部署层白名单，不入 FSM） | 038:36 镜像 [镜像推导] |
//! | nest_forward | 恒开（candidate 武装+次级别证据触发，双侧） | 027课 / 038:258 |
//! | osc / h1_freeze | 统一 osc 三门恒开（相位/振幅/g2 强弱 + 49:68 candidate 冻结——Δf 6/8 非负在册） | 035:30 / 093:26 / 049:68 |
//! | r2_gate | 双侧位置门恒开，域 = Φ(k)==Osc（削减@c≥ZG / 回补@c≤ZD） | 049:52 / 049:64 |
//! | （无） | R18-20 T2W 第二翻转窗口（confirmed type1 被门拒 ⇒ 锁存；type2 同侧 ∧ 门放行 ⇒ 重试；越极值 ⇒ 清） | 053:28 / 086:80 |
//!
//! ## Φ 三值化（公共读数——矛盾2 的承载）
//!
//! ```text
//! phi(k) = MoveUp   if freeze_up(k) ∧ ¬r4_up_exit(k)
//!          MoveDown if freeze_dn(k) ∧ ¬r4_dn_exit(k)   [镜像推导]
//!          Osc      otherwise
//! ```
//!
//! 全部行为从 Φ 读：停削（MoveUp）、停回补（MoveDown）、osc 路由①门
//! （≠Osc skip）、osc 满仓义务（==MoveUp）、44课铰链抑制（==MoveUp）、
//! r2 双侧门域（==Osc）。MoveUp 优先于 MoveDown（多头书的满仓义务优先，
//! 049:52；自层与祖先层读数冲突时保护多头侧）。
//!
//! ## R4 出口的锚语义
//!
//! freeze 上升沿冻结当时存活中枢 seg_start 为锚（无存活中枢 ⇒ i64::MIN）；
//! 出口开 iff 存活中枢 seg_start > 锚（新中枢已形成）∧ 无 candidate 离开
//! 窗口（049:68 当下读法——离开中 = 中枢移动 proper，不做短差）。中枢
//! 移动段（旧中枢死、新中枢未成）alive=None ⇒ 出口关，逐字对应"中枢
//! 向上移动时就应该满仓"；新中枢成 ⇒ 出口开，逐字对应"新中枢形成后
//! 围绕新中枢的短差恢复"。freeze 下降沿清锚。
//!
//! ## 翻转 = 会计断面（cascade settle 的 fusion_v 退化形态）
//!
//! Gated 落点无新书 ⇒ 四步序退化为 ②平旧（削减 trade 行，实际变现价）
//! → 落点 Gated；①子树结算由 osc step_exit 铰链承载（osc 在外时主层
//! 卖点先到 = hinge escalate，044:44 在册）；③开新/④子腿重开在 Gated
//! 形态无对象。M=N 的回复侧 = enter_or_defer 重配额（hold26 在册同款）。
//!
//! ## 诚实声明（090号）
//!
//! - 杠杆三元组（L_max = 1/(D_struct+mm)）**不在本实装**——设计 v2
//!   Phase 1 范围外（逐 bar D_struct 影子价格是独立工程），列开放轴。
//! - 真实空头/认沽载体**不在本实装**——载体选择是部署层动作（026:36），
//!   FSM 统一落点 Gated（任务条款："期权是载体选择不是 FSM 逻辑"）。
//! - R18 的"candidate 永不 confirmed（小转大 orphan）"武装支未实装——
//!   第一形态仅门拒支；orphan 支列开放轴（M13-e 判据先裁决可达性）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, PositionalResult,
    EQUITY_SAMPLE_BARS,
};
use super::positional_fusion::{PhaseView, SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::unified_osc::{OscLayer, OscOut};
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
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
        evs.iter().any(|e| e.class.side() == side)
            || devs.iter().any(|d| d.side() == side)
    } else {
        let want = match side {
            Side::Sell => Direction::Down,
            Side::Buy => Direction::Up,
        };
        flip_edge == Some(want)
    }
}

/// NAV：pool + 各层在市股数（Gated 层贡献 0——现金已在 pool；osc 在外腿
/// 所得已在 pool ⇒ 跳过股数计值。fusion_v 无 Short/SubOut）。
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
        if let LayerState::Long { shares, .. } = layers[k] {
            v += shares * c;
        }
    }
    v
}

/// 主入口（`PolarityMode::UnifiedVoice` 经 `run_positional` 分派至此）。
/// 零 flag——磁带能力是统一配置的前提（全词汇要求，缺行即拒绝）。
/// 三 bool 仅为预注册消融臂（U3c/M7-a 失败语义的定位工具；部署形态
/// fusion_v 恒全 true）：r4_exit = R4 双侧出口；gated_landing = R14
/// 削减落点 Gated（false ⇒ Flat 对照）；anc_freeze = freeze 递归传导
/// （false ⇒ 仅自层对照）。
pub(crate) fn run_unified_voice(
    tape: &SignalTape,
    floor_ladder: usize,
    r4_exit_on: bool,
    gated_landing: bool,
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

    // R4 出口锚（freeze 上升沿冻结；None = 窗口未武装）。
    let mut r4_up_anchor: [Option<i64>; MAX_LADDER] = [None; MAX_LADDER];
    let mut r4_dn_anchor: [Option<i64>; MAX_LADDER] = [None; MAX_LADDER];
    let mut freeze_up_prev: [bool; MAX_LADDER] = [false; MAX_LADDER];
    let mut freeze_dn_prev: [bool; MAX_LADDER] = [false; MAX_LADDER];

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
        //    窗口的价格否定（049:68 当下读法——R4 出口与 osc h1 共用）──
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
                    let win = if sellside { &mut nest_sell[k] } else { &mut nest_buy[k] };
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
                if nest_sub_evidence(
                    sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub],
                ) {
                    nf_sell[k] = true;
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if let Some(cs) = w.cs {
                        nest_fired.insert((k, true, cs), i as i64);
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if nest_sub_evidence(
                    sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub],
                ) {
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
            if anc_freeze { (k..MAX_LADDER).any(self_up) } else { self_up(k) }
        };
        let freeze_dn = |k: usize| {
            if anc_freeze { (k..MAX_LADDER).any(self_dn) } else { self_dn(k) }
        };

        // ── R4 出口锚维护（freeze 沿检测；上升沿冻结锚，下降沿清）──
        for k in floor_ladder..MAX_LADDER {
            let fu = freeze_up(k);
            if fu && !freeze_up_prev[k] {
                r4_up_anchor[k] =
                    Some(book.alive(k).map_or(i64::MIN, |lc| lc.seg_start));
            } else if !fu {
                r4_up_anchor[k] = None;
            }
            freeze_up_prev[k] = fu;
            let fd = freeze_dn(k);
            if fd && !freeze_dn_prev[k] {
                r4_dn_anchor[k] =
                    Some(book.alive(k).map_or(i64::MIN, |lc| lc.seg_start));
            } else if !fd {
                r4_dn_anchor[k] = None;
            }
            freeze_dn_prev[k] = fd;
        }
        let r4_up_exit = |k: usize| {
            r4_exit_on
                && r4_up_anchor[k].is_some_and(|a| {
                    book.alive(k).is_some_and(|lc| lc.seg_start > a)
                        && !book.has_pending_departure(k)
                })
        };
        let r4_dn_exit = |k: usize| {
            r4_exit_on
                && r4_dn_anchor[k].is_some_and(|a| {
                    book.alive(k).is_some_and(|lc| lc.seg_start > a)
                        && !book.has_pending_departure(k)
                })
        };

        // ── Φ 三值化（公共读数；MoveUp 优先 = 多头书满仓义务优先）──
        let phi = |k: usize| -> PhaseView {
            if freeze_up(k) && !r4_up_exit(k) {
                PhaseView::MoveUp
            } else if freeze_dn(k) && !r4_dn_exit(k) {
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
            if freeze_up(k) && r4_up_exit(k) {
                res.r4_up_exit_bars_by_ladder[k] += 1;
            }
            if freeze_dn(k) && r4_dn_exit(k) {
                res.r4_dn_exit_bars_by_ladder[k] += 1;
            }
        }

        // 双侧位置门（049:52"在中枢上方仓位减少" / 049:64"在下方如数接回"）。
        // 域 = Φ(k)==Osc 精确（MoveDown 削减放行 = 逃命语义；MoveUp 回补
        // 走强制回复分支）。NaN 比较恒 false ⇒ 拦截（保守方向在册同构）。
        let r2_trim_blocked = |k: usize| {
            phi(k) == PhaseView::Osc
                && book.alive(k).is_some_and(|lc| !(c >= lc.zg))
        };
        let r2_restore_blocked = |k: usize| {
            phi(k) == PhaseView::Osc
                && book.alive(k).is_some_and(|lc| !(c <= lc.zd))
        };

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
            evrows[k].iter().any(|e| e.confirmed && e.class == BspClass::Sell2)
        };
        let conf_buy2 = |k: usize| {
            evrows[k].iter().any(|e| e.confirmed && e.class == BspClass::Buy2)
        };

        // ── 阶段 A：层级出场（停削/位置门/削减→Gated）/ osc 在外腿出口 ──
        for k in floor_ladder..MAX_LADDER {
            let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
                layers[k]
            else {
                continue;
            };
            res.held_bars_by_ladder[k] += 1;
            if osc_layer.outs[k].is_some() {
                // osc 在外腿出口集（满仓义务/铰链/回补）——Φ 读数透传，
                // nest 触发与 confirmed 同词汇地位双侧透传。
                osc_layer.step_exit(
                    k, c, i as i64, sig, nf_sell[k], nf_buy[k], &phi, &book,
                    &mut layers, &mut pool, &mut res,
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
                // 削减（hold26 在册）→ R14 默认落点 Gated（方向承诺 d=−1、
                // 市场暴露 0；三组活跃义务在阶段 C 的 Gated 分支消费）。
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
                if r4_up_exit(k) {
                    res.n_r4_up_trims_by_ladder[k] += 1;
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
                if gated_landing {
                    layers[k] = LayerState::Gated { entry_bar: i as i64, weight };
                    res.n_gate_enters_by_ladder[k] += 1;
                } else {
                    // 消融对照（fusion_v_flat）：落点 Flat——Gated 三组
                    // 义务门（MoveDown 停回补/R2 c≤ZD/MoveUp 强制）摘除。
                    layers[k] = LayerState::Flat;
                }
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
                    k, c, i as i64, sig, true, true, &phi, &book, &depth_ref,
                    &layers, &mut pool, &mut res,
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
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                LayerState::Pending { confirm_bar } => {
                    if sig.sell_any.get(k) || nf_sell[k] {
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
                    unreachable!("fusion_v 无 ARMED 相位——confirmed 事件直接消费")
                }
                LayerState::Long { .. } => {}
                LayerState::Short { .. } => {
                    unreachable!("fusion_v 无线性空头书——R14 默认落点 Gated")
                }
                // ── Gated（d=−1 逻辑态的零暴露执行形态）：三组活跃义务
                //    （R15/R16 + MoveDown 停回复 + 空侧 R2 门）──
                LayerState::Gated { weight: _, .. } => {
                    res.gate_held_bars_by_ladder[k] += 1;
                    let t2w_bfire = t2w_buy[k].is_some() && conf_buy2(k);
                    if phi(k) == PhaseView::MoveUp {
                        // R16：Φ 翻回 MoveUp ⇒ 强制回复（049:52 满仓义务）。
                        res.n_gate_moveup_restores_by_ladder[k] += 1;
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
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
                                t2w_buy[k] =
                                    Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else if r2_restore_blocked(k) {
                            // R15 位置门：c ≤ ZD 才回补（049:64 镜像）。
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                            if let Some(px) = conf_buy1_px(k) {
                                if t2w_buy[k].is_none() {
                                    res.n_t2w_arms_by_ladder[k] += 1;
                                }
                                t2w_buy[k] =
                                    Some(t2w_buy[k].map_or(px, |x| x.min(px)));
                            }
                        } else {
                            res.n_gate_restores_by_ladder[k] += 1;
                            if !sig.buy_any.get(k) && !nf_buy[k] {
                                res.n_t2w_fires_by_ladder[k] += 1;
                            }
                            if r4_dn_exit(k) {
                                res.n_r4_dn_restores_by_ladder[k] += 1;
                            }
                            layers[k] = enter_or_defer(
                                k, i as i64, i as i64, c, bar_nav, &thetas,
                                theta_total, &mut pool, &mut res,
                            );
                            t2w_buy[k] = None;
                        }
                    }
                }
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层收口（Gated 无遗留账务——现金已在削减 bar 入 pool；
    //    osc 在外腿悬置收口，所得已在 pool）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        if let LayerState::Gated { .. } = layers[k] {
            layers[k] = LayerState::Flat;
            continue;
        }
        let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
            layers[k]
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
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect()
    }

    /// 部署形态（全 true）。
    fn full_v() -> PolarityMode {
        PolarityMode::UnifiedVoice { r4_exit: true, gated_landing: true, anc_freeze: true }
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
            PolarityMode::parse("fusion_v_nor4"),
            Some(PolarityMode::UnifiedVoice {
                r4_exit: false,
                gated_landing: true,
                anc_freeze: true
            })
        );
        assert_eq!(
            PolarityMode::parse("fusion_v_flat"),
            Some(PolarityMode::UnifiedVoice {
                r4_exit: true,
                gated_landing: false,
                anc_freeze: true
            })
        );
        assert_eq!(
            PolarityMode::parse("fusion_v_self"),
            Some(PolarityMode::UnifiedVoice {
                r4_exit: true,
                gated_landing: true,
                anc_freeze: false
            })
        );
    }

    #[test]
    fn capability_guards() {
        // 无 trend/dir 行 ⇒ 拒绝（freeze 读数无数据基础）。
        let t = SignalTape { bars: warmup(2), ..Default::default() };
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
    fn trim_lands_in_gated_and_restores_on_buy() {
        // R14 默认落点：削减后 Gated（非 Flat）；买点回复（R15 门放行）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(100.0), 2)); // 削减（Osc 相、c=100≥ZG=51）→ Gated
        bars.push(buypt(bar(40.0), 2)); // 买点 @40 ≤ ZD=50 → 回复
        bars.push(bar(40.0));
        let r = run_v(bars, vec![], vec![]);
        assert_eq!(r.n_gate_enters_by_ladder[2], 1, "削减落点 Gated");
        assert_eq!(r.n_gate_restores_by_ladder[2], 1, "买点回复");
        assert_eq!(r.n_entries_by_ladder[2], 2);
    }

    #[test]
    fn gated_restore_blocked_above_zd() {
        // R15 空侧位置门：Osc 相中 c > ZD ⇒ 回补拦截（049:64 镜像）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 2)); // → Gated
        bars.push(buypt(bar(80.0), 2)); // 80 > ZD=50 ⇒ 拦截
        bars.push(bar(80.0));
        let r = run_v(bars, vec![], vec![]);
        assert_eq!(r.n_short_r2_blocks_by_ladder[2], 1, "ZD 上方回补拦截");
        assert_eq!(r.n_gate_restores_by_ladder[2], 0);
    }

    #[test]
    fn freeze_up_suppresses_trim_ancestor_component() {
        // freeze_d 递归传导：祖先层（3）Trend∧Up ⇒ 层2 停削（26:80 下沉）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 祖先 freeze ⇒ 停削
        bars.push(bar(110.0));
        let w = SUB_COST_MIN_OBS as i64;
        // 行注入在 warmup 后（锚 cs 在 warmup 期间递增——freeze 上升沿若在
        // warmup 内，新锚会立即满足 R4 出口判据，污染停削断言）。
        let r = run_v(
            bars,
            vec![(w + 1, 3, Direction::Up)],
            vec![(w + 1, 3, true)],
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "停削");
        assert_eq!(r.n_anc_exempt_blocks_by_ladder[2], 1, "祖先分量归因");
        assert_eq!(
            r.trades.iter().filter(|t| t.ladder == 2 && t.exit_reason != "eod").count(),
            0
        );
    }

    #[test]
    fn movedown_blocks_restore_until_flip() {
        // MoveDown 停回补（49:52 镜像）：freeze_dn 中买点拦截；翻 Osc 后回复。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 入场
        bars.push(sellpt(bar(90.0), 2)); // MoveDown 中卖点削减放行（逃命）→ Gated
        bars.push(buypt(bar(40.0), 2)); // freeze_dn ⇒ 停回补
        bars.push(buypt(bar(40.0), 2)); // 翻 Osc（行注入）⇒ 回复 @40≤ZD
        bars.push(bar(40.0));
        let w = SUB_COST_MIN_OBS as i64;
        // freeze_dn 上升沿在 warmup 后（锚污染规避同 freeze_up 测试）。
        let r = run_v(
            bars,
            vec![(w, 2, Direction::Down)],
            vec![(w, 2, true), (w + 3, 2, false)],
        );
        assert_eq!(r.n_short_trend_holds_by_ladder[2], 1, "MoveDown 停回补");
        assert_eq!(r.n_gate_restores_by_ladder[2], 1, "翻 Osc 后回复");
        // MoveDown 中削减不被停削/位置门拦（多头逃命语义）。
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "sellpt")
            .collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_price, 90.0);
    }

    #[test]
    fn r4_exit_reopens_trim_on_new_center() {
        // R4 多侧出口：freeze_up 进入时锚=中枢 cs=10..；新中枢（cs 更大）
        // 形成 ⇒ 出口开 ⇒ 削减词汇重开（049:54 后半）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2)); // 入场（Osc 相——行在 w+1 注入）
        bars.push(sellpt(bar(100.0), 2)); // freeze_up ⇒ 停削
        // 新中枢出现（cs=900 > 锚）⇒ R4 出口开。
        bars.push(with_anchor(bar(100.0), 2, 900, 50.0, 51.0));
        bars.push(sellpt(bar(100.0), 2)); // 出口窗口内削减放行
        bars.push(bar(100.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_v(
            bars,
            vec![(w + 1, 2, Direction::Up)],
            vec![(w + 1, 2, true)],
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "出口前停削");
        assert_eq!(r.n_r4_up_trims_by_ladder[2], 1, "出口窗口内削减");
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "sellpt")
            .collect();
        assert_eq!(t2.len(), 1, "R4 出口削减成交");
        assert!(r.r4_up_exit_bars_by_ladder[2] >= 1);
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
        bars.push(sellpt(with_ev(bar(100.0), 2, conf(BspClass::Sell1, 120.0)), 2));
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
        bars.push(sellpt(with_ev(bar(100.0), 2, conf(BspClass::Sell1, 120.0)), 2));
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
    fn nav_conservation_with_gated() {
        // Gated 期 NAV = pool（现金已变现）；逐 trade 重建与 final_nav 一致。
        let mut bars = warmup(2);
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 削减 @110 → Gated
        bars.push(bar(120.0)); // Gated 期价格上涨不影响 NAV
        bars.push(bar(120.0));
        let r = run_v(bars, vec![], vec![]);
        assert!((r.final_nav - 110_000.0).abs() < 1e-6, "Gated 持币 NAV 守恒");
    }
}
