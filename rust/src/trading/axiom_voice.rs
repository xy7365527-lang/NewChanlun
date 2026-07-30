//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! axiom_voice — 公理演绎统一 voice FSM（fusion_va；2026-06-12 编排者方法论定型轮）。
//!
//! 方法论：FSM 不是设计出来的，是五条公理各自自我否定后的扬弃之总和
//! （推导记录 `analysis/unified_voice_axiom_derivation.md`；推导工位 A1/A4/A5
//! + 对抗审查三 lens + 否证边界对照）。回测角色 = 证伪检验，非发现检验。
//!
//! ## 五公理 → FSM 要素（每要素承载 ≥2 矛盾，检验3）
//!
//! | 公理 | 否定运动产物 | FSM 要素 | 承载矛盾 |
//! |------|-------------|---------|---------|
//! | 1 走势完全分类 | 分类=（级别,走势）对属性→逐层相位读数；kind 留作 26:80 豁免域定理映射（535 终局，对抗 lens 裁决） | Φ(k) 中枢账本相位 + exempt_d(k) 祖先 settled-tail-kind | regime 特化消除+层间指令冲突+塔顶候选性 |
//! | 2 买卖点结构性存在 | 确认滞后 vs 操作即时→区间套三段式 | nest 双侧（武装→次级别证据→破极值否定） | 滞后+假信号+委托定位 |
//! | 3 递归自相似 | 递归终止=成本门自我界定 | 每层完全相同的转移链；Q(k) 活跃门 | 级别特化消除+a0 边界+递归终止 |
//! | 4 级别=操作量 | 嵌套 slice+44课铰链身份事后授予 | PairedOut 短差载具（身份悬置：接回=短差/升级=翻转） | 级别错配+守恒 vs 合法性+恒仓 M=N |
//! | 5 成本门 | 摩擦随载体涌现→参数/端口分离，k_cost≡1 钉死 | Q(k)=defined(θ_mean)∧θ_mean≥C_rt（C=端口） | 递归终止+级别去特化+震荡参与资格 |
//!
//! ## 状态判断链（编排者：非门合取，有优先级的状态判断链）
//!
//! 每层每 bar，优先级从高到低（义务 ≻ 出口 ≻ 开启）：
//! P1 PairedOut 出口集：满仓义务（exempt_up ∨ Φ=Move↑ ⇒ 强制接回，049:52）≻
//!    锚死亡向下 ⇒ 升级翻转（049:52"不能回补了"+044:44 铰链）≻
//!    本层 t1 卖 ⇒ 升级翻转（044:44 父点先到）≻
//!    锚 ZD 触线 ⇒ 几何接回（049:64"在下方如数接回"——位置触发，
//!    否证边界特别裁决：事件等待=三连败共同死因，配对闭环=唯一存活形态）≻
//!    中枢上移 ⇒ 接回（049:54 新中枢；sc 在册 8/10）≻ 买点事件辅助接回
//! P2 Long 卖链：exempt_up ⇒ 冻结（026:80，含 t1——fusion_td 粒度错配否证边界）→
//!    Φ=Move↑∧¬t1 ⇒ 冻结（049:52）→ Φ=Osc ⇒ 短差轨道（Q∧c≥ZG∧卖词汇 ⇒
//!    PairedOut 配对快照；否则拒）→ Φ=Move↓ ∨ (Move↑∧t1) ⇒ 翻转轨道
//!    （卖出→Coin，d 翻 −1，物理零暴露=载体部署层）
//! P3 Coin 买链（P2 的极性镜像，逐条对偶）：exempt_dn ⇒ 停回补[镜像推导] →
//!    Φ=Move↓∧¬t1买 ⇒ 冻结（满空义务镜像）→ Φ=Osc∧(¬Q∨¬(c≤ZD)) ⇒ 拒 →
//!    else ⇒ 买回（翻多，M=N 经 enter_or_defer 配额）
//!
//! ## 否证边界对照（推导触碰处的裁决，全部在册）
//!
//! - 49:54 出口三连败 ⇒ Osc 短差改配对闭环（本实装），非全词汇重开；
//! - P6 单边换窗口 ⇒ 26:80 豁免保留 kind 行读数（settled-tail），Φ 相位
//!   只承载 49:52 自层义务——两窗口两条款（535 条款—例外并立）；
//! - fusion_td 3/3 否证 ⇒ exempt 窗口内 t1 同冻结（仅自层 Φ=Move↑ 放行 t1）；
//! - cycle38 否证 ⇒ 接回判据本级别先行（锚 ZD 触线），次级别仅作 nest 时点精化；
//! - 84课 fatal+ES/QQQ 反手全负 ⇒ 翻转落点恒零暴露（Coin），真实空头=部署层；
//! - 共享仓位爆仓否证 ⇒ 配额制独立 slice（θ 权重=嵌套的会计形式）。
//!
//! ## 诚实声明（090号）
//!
//! - d=−1 书的镜像短差（持币中 ZD 买入→ZG 再卖）需要空头单位，Gated 零暴露
//!   退化形态下不可表示——非对称由载体层吸收（空头单相单律非对称定理在册）。
//! - 挣筹码相（cost_basis≤0 守恒律切换）不入本 FSM：L3 有效域近空在册
//!   （全库 0-1 笔），fusion 权益传导基座下无 cost_basis 概念（v2 设计矛盾14）。
//! - C 端口回测注入值 = 2×SUB_FRICTION_RT（往返摩擦；k_cost≡1 概念重释，
//!   数值与在册成本门等价）；impact(s) 分量按对抗 lens 裁决排除（非端口）。

use super::center_book::CenterBook;
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS,
};
use super::positional_fusion::{PhaseView, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// C 端口（回测部署层注入值）：往返摩擦 = 2×单边。k_cost≡1（035:30 存在论
/// 下界=长期期望非负的唯一零自由度形式；"足够小"的余量归部署层，035:32）。
const C_ROUND_TRIP: f64 = 2.0 * SUB_FRICTION_RT;

/// 区间套窗口（公理2 三段式；src_t1 标记武装源是否 type1——Move 相位中
/// 仅背驰类出口放行，049:54"直到移动出现背驰"）。
#[derive(Debug, Clone, Copy)]
struct NestWin {
    extreme: f64,
    src_t1: bool,
}

/// 配对闭环短差在外态（公理4 身份悬置载具；否证边界特别裁决的唯一存活
/// 接回形态——锚快照冻结，出口以几何触线为主）。
#[derive(Debug, Clone, Copy)]
struct PairedOut {
    shares: f64,
    sell_bar: i64,
    sell_price: f64,
    /// 配对锚（卖出 bar 的存活中枢快照——比较基准不可重读，公理2 配对快照）。
    anchor_cs: i64,
    anchor_zd: f64,
    anchor_zg: f64,
    restore_due: bool,
}

/// 次级别证据（公理2 区间套触发；bi 层用方向翻转沿——dir 行是 bi 层唯一
/// 结构通道，SC 先例）。
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

/// NAV：pool + 各层在市股数（Coin/PairedOut 的价值是现金已在 pool）。
fn nav_va(
    layers: &[LayerState; MAX_LADDER],
    pairs: &[Option<PairedOut>; MAX_LADDER],
    cash: f64,
    c: f64,
    floor: usize,
) -> f64 {
    let mut v = cash;
    for k in floor..MAX_LADDER {
        if pairs[k].is_some() {
            continue;
        }
        if let LayerState::Long { shares, .. } = layers[k] {
            v += shares * c;
        }
    }
    v
}

/// 主入口（`PolarityMode::AxiomVoice` 经 `run_positional` 分派至此）。
/// 零 flag——唯一参数 = a0（磁带粒度）。
pub(crate) fn run_axiom_voice(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "fusion_va 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("fusion_va 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !tape.has_div_events() {
        return Err(
            "fusion_va 要求背驰磁带——nest 次级别证据词汇的结构分量，缺行即\
             词汇残缺（不静默降级）"
                .to_string(),
        );
    }
    if !(tape.has_trend_rows() && tape.has_dir_rows()) {
        return Err(
            "fusion_va 要求磁带 trend_flips + dir_flips 行——26:80 豁免域 = \
             settled-tail-kind 读数（535 终局定理映射），无行即豁免条款无\
             数据基础"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut pairs: [Option<PairedOut>; MAX_LADDER] = [None; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // 行滚动状态（settled-tail-kind 读数源；26:80 豁免域专用——Φ 不消费）。
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];

    // 区间套窗口（公理2；恒开）。
    let mut nest_sell: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];

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

        // 市场性质：中枢账本 + 振幅参照 + candidate 离开窗口价格否定
        // （049:68 当下读法——塔顶候选性的逐层形式，对抗 lens RISK 修正：
        // 候选分级对每一层成立非仅塔顶）。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);
        for lad in FIRST_BSP_LADDER..MAX_LADDER {
            book.negate_pending_departure(lad, c);
        }

        // ── 区间套三段式（公理2；①破极值否定 ②candidate 武装 ③次级别
        //    证据触发）。confirmed 到达 ⇒ 窗口让位掩码路径 ──
        let mut nf_sell = [false; MAX_LADDER];
        let mut nf_buy = [false; MAX_LADDER];
        let mut nf_sell_t1 = [false; MAX_LADDER];
        let mut nf_buy_t1 = [false; MAX_LADDER];
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
                    let (sellside, is_t1) = match e.class {
                        BspClass::Sell1 => (true, true),
                        BspClass::Sell3 => (true, false),
                        BspClass::Buy1 => (false, true),
                        BspClass::Buy3 => (false, false),
                        BspClass::Sell2 | BspClass::Buy2 => continue,
                    };
                    let win = if sellside {
                        &mut nest_sell[k]
                    } else {
                        &mut nest_buy[k]
                    };
                    if e.confirmed {
                        *win = None;
                    } else {
                        let (ext, t1) = win.map_or((e.price, is_t1), |w| {
                            (
                                if sellside {
                                    w.extreme.max(e.price)
                                } else {
                                    w.extreme.min(e.price)
                                },
                                w.src_t1 || is_t1,
                            )
                        });
                        *win = Some(NestWin {
                            extreme: ext,
                            src_t1: t1,
                        });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                }
            }
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if nest_sub_evidence(sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_sell[k] = true;
                    nf_sell_t1[k] = w.src_t1;
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                }
            }
            if let Some(w) = nest_buy[k] {
                if nest_sub_evidence(sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub]) {
                    nf_buy[k] = true;
                    nf_buy_t1[k] = w.src_t1;
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                }
            }
        }

        // ── 读数层（公理1+5；全部从走势结构当下状态直读，零存储）──
        // Φ(k)：中枢账本相位。candidate 离开窗口（pending_departure，价格
        // 回中枢即否定）⊂ Move——候选分级逐层成立；中枢存活=Osc（震荡相）；
        // 中枢死亡至新中枢形成=Move(死亡方向)（049:52/54/60/62：相位分界
        // =第三类点，Move 终点=新中枢形成）。
        let phi = |k: usize| -> PhaseView {
            match book.pending_departure_side(k) {
                Some(Side::Buy) => PhaseView::MoveUp,
                Some(Side::Sell) => PhaseView::MoveDown,
                None => {
                    if book.alive(k).is_some() {
                        PhaseView::Osc
                    } else if let Some(lc) = book.last(k) {
                        if book.is_dead_down(k, lc.seg_start) {
                            PhaseView::MoveDown
                        } else {
                            PhaseView::MoveUp
                        }
                    } else {
                        PhaseView::Osc
                    }
                }
            }
        };
        // 26:80 豁免域（settled-tail-kind，535 定理映射；对抗 lens FAIL 裁决
        // ——kind 行是已完成走势的回溯计数，当下可判，不可锚到 Φ 相位
        // （P6 单边换窗口否证边界））。严格祖先 ∃j>k。
        let exempt_up = |k: usize| {
            ((k + 1)..MAX_LADDER).any(|j| trend_state[j] && dir_state[j] == Some(Direction::Up))
        };
        let exempt_dn = |k: usize| {
            ((k + 1)..MAX_LADDER).any(|j| trend_state[j] && dir_state[j] == Some(Direction::Down))
        };
        // 成本门 Q(k)（公理5）：defined(θ_mean) ∧ θ_mean ≥ C_rt，k_cost≡1。
        // 未定义短路拒绝（资格是需证据的存在谓词，053:26）。
        let q_gate = |k: usize| depth_ref.theta_mean(k).is_some_and(|m| m >= C_ROUND_TRIP);

        // 相位驻留观测。
        for k in floor_ladder..MAX_LADDER {
            match phi(k) {
                PhaseView::MoveUp => res.freeze_up_bars_by_ladder[k] += 1,
                PhaseView::MoveDown => res.freeze_dn_bars_by_ladder[k] += 1,
                PhaseView::Osc => {}
            }
            if exempt_up(k) {
                res.anc_up_bars_by_ladder[k] += 1;
            }
        }

        // ── P1：PairedOut 出口集（义务优先——资金守恒+满仓义务）──
        for k in floor_ladder..MAX_LADDER {
            let Some(po) = pairs[k] else { continue };
            let LayerState::Long {
                entry_bar,
                entry_price,
                shares,
                weight,
                deferred_bars,
                partial,
            } = layers[k]
            else {
                unreachable!("PairedOut 在外 ⇒ 本层恒 Long（载具生命周期内层不变迁）")
            };
            debug_assert!(
                (shares - po.shares).abs() < 1e-12,
                "全抛/如数接回 ⇒ 股数恒等"
            );
            // 接回闭包（M=N：同股数，049:64"如数接回"）。
            let mut restore = |reason: &'static str,
                               pool: &mut f64,
                               layers: &mut [LayerState; MAX_LADDER],
                               pairs: &mut [Option<PairedOut>; MAX_LADDER],
                               res: &mut PositionalResult|
             -> bool {
                let cost = po.shares * c;
                if cost > *pool {
                    pairs[k] = Some(PairedOut {
                        restore_due: true,
                        ..po
                    });
                    res.n_sub_restore_defer_bars += 1;
                    return false;
                }
                *pool -= cost;
                res.sub_net_cash_by_ladder[k] += po.shares * po.sell_price - cost;
                res.trades.push(LayerTrade {
                    ladder: k as u8,
                    entry_bar,
                    entry_price,
                    exit_bar: po.sell_bar,
                    exit_price: po.sell_price,
                    shares: po.shares,
                    weight_at_entry: weight,
                    deferred_bars,
                    partial,
                    exit_reason: reason,
                    polarity: Polarity::Long,
                });
                layers[k] = LayerState::Long {
                    entry_bar: i as i64,
                    entry_price: c,
                    shares,
                    weight,
                    deferred_bars: 0,
                    partial,
                };
                pairs[k] = None;
                true
            };
            // 升级翻转闭包（044:44 铰链：身份事后授予为出场第一腿；层转
            // Coin——d 翻 −1，零暴露）。
            let escalate = |reason: &'static str,
                            pool: &mut f64,
                            layers: &mut [LayerState; MAX_LADDER],
                            pairs: &mut [Option<PairedOut>; MAX_LADDER],
                            res: &mut PositionalResult| {
                let _ = pool;
                res.trades.push(LayerTrade {
                    ladder: k as u8,
                    entry_bar,
                    entry_price,
                    exit_bar: po.sell_bar,
                    exit_price: po.sell_price,
                    shares: po.shares,
                    weight_at_entry: weight,
                    deferred_bars,
                    partial,
                    exit_reason: reason,
                    polarity: Polarity::Long,
                });
                res.n_exits_by_ladder[k] += 1;
                res.n_sub_escalates_by_ladder[k] += 1;
                layers[k] = LayerState::Gated {
                    entry_bar: i as i64,
                    weight,
                };
                res.n_gate_enters_by_ladder[k] += 1;
                pairs[k] = None;
            };
            let t1_sell = sig.sell1.get(k) || nf_sell_t1[k];
            if exempt_up(k) || phi(k) == PhaseView::MoveUp {
                // 049:52 满仓义务：移动相/豁免域开始 ⇒ 强制接回。
                if restore("sub_diff", &mut pool, &mut layers, &mut pairs, &mut res) {
                    res.n_sub_phase_closes_by_ladder[k] += 1;
                }
            } else if book.is_dead_down(k, po.anchor_cs) {
                // 049:52"一旦出现第三类卖点，就不能回补了" ⇒ 升级翻转。
                escalate(
                    "hinge_escalate",
                    &mut pool,
                    &mut layers,
                    &mut pairs,
                    &mut res,
                );
            } else if t1_sell {
                // 044:44：本层走势级出口先到 ⇒ 升级翻转。
                escalate(
                    "hinge_escalate",
                    &mut pool,
                    &mut layers,
                    &mut pairs,
                    &mut res,
                );
            } else if c <= po.anchor_zd {
                // 049:64"在下方如数接回"——几何触线（特别裁决：唯一存活
                // 接回形态；事件等待=三连败死因）。
                if restore("sub_diff", &mut pool, &mut layers, &mut pairs, &mut res) {
                    res.n_osc_zd_restores_by_ladder[k] += 1;
                }
            } else if book
                .alive(k)
                .is_some_and(|lc| lc.seg_start != po.anchor_cs && lc.zd > po.anchor_zg)
            {
                // 中枢上移（新中枢 ZD > 锚 ZG）⇒ 接回（sc 在册 8/10 转正）。
                if restore("sub_diff", &mut pool, &mut layers, &mut pairs, &mut res) {
                    res.n_osc_shift_restores_by_ladder[k] += 1;
                }
            } else if sig.buy_any.get(k) || nf_buy[k] || po.restore_due {
                // 买点事件辅助出口 / 资金不足重试。
                if restore("sub_diff", &mut pool, &mut layers, &mut pairs, &mut res) {
                    res.n_sub_restores_by_ladder[k] += 1;
                }
            }
        }

        // ── P2：Long 卖链（状态判断链——优先级从上到下，首条命中即定）──
        for k in floor_ladder..MAX_LADDER {
            if pairs[k].is_some() {
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
            res.held_bars_by_ladder[k] += 1;
            let sell_trig = sig.sell_any.get(k) || nf_sell[k];
            if !sell_trig {
                continue;
            }
            let t1_sell = sig.sell1.get(k) || nf_sell_t1[k];
            if exempt_up(k) {
                // 026:80 豁免：祖先单边上扬冻结全部卖词汇（含 t1——
                // fusion_td 粒度错配否证边界）。
                res.n_trend_holds_by_ladder[k] += 1;
                res.n_anc_exempt_blocks_by_ladder[k] += 1;
                continue;
            }
            let ph = phi(k);
            if ph == PhaseView::MoveUp && !t1_sell {
                // 049:52 自层移动相义务（049:54"直到移动出现背驰"⇒ t1 放行）。
                res.n_trend_holds_by_ladder[k] += 1;
                continue;
            }
            if ph == PhaseView::Osc {
                // 震荡相 ⇒ 短差轨道（配对闭环开启；门链=资格∧位置）。
                if !q_gate(k) {
                    res.n_sub_cost_rejects_by_ladder[k] += 1;
                    continue;
                }
                let Some(lc) = book.alive(k) else {
                    res.n_sub_noref_rejects_by_ladder[k] += 1;
                    continue;
                };
                if !(c >= lc.zg) {
                    // 049:52"在中枢上方仓位减少"（NaN 保守拒，在册纪律）。
                    res.n_r2_pos_blocks_by_ladder[k] += 1;
                    continue;
                }
                pool += shares * c;
                pairs[k] = Some(PairedOut {
                    shares,
                    sell_bar: i as i64,
                    sell_price: c,
                    anchor_cs: lc.seg_start,
                    anchor_zd: lc.zd,
                    anchor_zg: lc.zg,
                    restore_due: false,
                });
                res.n_sub_opens_by_ladder[k] += 1;
                continue;
            }
            // 翻转轨道：Move↓（逃命）∨ Move↑∧t1（背驰出口）⇒ 卖出 → Coin。
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
                exit_reason: if !sig.sell_any.get(k) {
                    "nest_sell"
                } else {
                    "sellpt"
                },
                polarity: Polarity::Long,
            });
            res.n_exits_by_ladder[k] += 1;
            layers[k] = LayerState::Gated {
                entry_bar: i as i64,
                weight,
            };
            res.n_gate_enters_by_ladder[k] += 1;
        }

        // ── P3：Coin 买链（P2 的极性镜像；Flat=创世前的 Coin 同形——
        //    递归自相似零初始特例）──
        let bar_nav = nav_va(&layers, &pairs, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            match layers[k] {
                LayerState::Flat | LayerState::Gated { .. } => {
                    let was_gated = matches!(layers[k], LayerState::Gated { .. });
                    if was_gated {
                        res.gate_held_bars_by_ladder[k] += 1;
                    }
                    let buy_trig = sig.buy_any.get(k) || nf_buy[k];
                    if !buy_trig {
                        continue;
                    }
                    let t1_buy = sig.buy1.get(k) || nf_buy_t1[k];
                    if exempt_dn(k) {
                        // 026:80 镜像：祖先单边下跌冻结买词汇 [镜像推导]。
                        res.n_short_trend_holds_by_ladder[k] += 1;
                        continue;
                    }
                    let ph = phi(k);
                    if ph == PhaseView::MoveDown && !t1_buy {
                        // 满空义务镜像：向下移动相非背驰不接刀 [镜像推导]。
                        res.n_short_trend_holds_by_ladder[k] += 1;
                        continue;
                    }
                    if ph == PhaseView::Osc {
                        if !q_gate(k) {
                            res.n_sub_cost_rejects_by_ladder[k] += 1;
                            continue;
                        }
                        if !book.alive(k).is_some_and(|lc| c <= lc.zd) {
                            // 049:64"在下方"位置镜像（NaN 保守拒）。
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                            continue;
                        }
                    }
                    // 买回（翻多）：Move↑ 买点（049:60 三买后持股）∨
                    // Move↓ t1（背驰接回）∨ Osc 下方位置。
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
                    if was_gated
                        && matches!(
                            layers[k],
                            LayerState::Long { .. } | LayerState::Pending { .. }
                        )
                    {
                        res.n_gate_restores_by_ladder[k] += 1;
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
                LayerState::Long { .. } => {}
                LayerState::Armed { .. } => {
                    unreachable!("fusion_va 无 ARMED 相位——confirmed/nest 词汇直接消费")
                }
                LayerState::Short { .. } => {
                    unreachable!("fusion_va 翻转落点恒零暴露（Coin）——真实空头=部署层")
                }
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层收口（PairedOut 悬置——trade 行以实际变现点记账；
    //    Coin 无遗留账务）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        if let LayerState::Gated { .. } = layers[k] {
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
        let (exit_bar, exit_price, sh) = match pairs[k] {
            Some(po) => (po.sell_bar, po.sell_price, po.shares),
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
        pairs[k] = None;
    }
    res.final_nav = pool;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::config::SUB_COST_MIN_OBS;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    fn bar(close: f64) -> BarSig {
        BarSig {
            close,
            max_ladder: 5,
            ..Default::default()
        }
    }

    /// candidate 锚事件（中枢快照源）。
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

    fn sell_t1(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b
    }

    /// 同一中枢（cs 固定）反复观测 ⇒ θ 参照（zd=50/zg=51，振幅 1%≥C_rt）。
    /// 锚 cs 固定 ⇒ Φ 恒 Osc（中枢存活）且不污染相位。
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

    fn run_va(
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
        run_positional(&t, 2, PolarityMode::AxiomVoice).unwrap()
    }

    #[test]
    fn parse_fusion_va() {
        assert_eq!(
            PolarityMode::parse("fusion_va"),
            Some(PolarityMode::AxiomVoice)
        );
    }

    #[test]
    fn osc_trim_opens_paired_and_zd_touch_restores() {
        // Osc 相高位卖出 ⇒ PairedOut；锚 ZD 触线 ⇒ 几何接回（049:64）——
        // 不等买点事件（特别裁决：事件等待=三连败死因）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2)); // Osc 下方入场 @40 ≤ ZD=50
        bars.push(sellpt(bar(100.0), 2)); // Osc 高位 c=100≥ZG=51 ⇒ PairedOut
        bars.push(bar(45.0)); // 触线 c=45 ≤ ZD=50 ⇒ 几何接回（无买点事件！）
        bars.push(bar(45.0));
        let r = run_va(bars, vec![], vec![]);
        assert_eq!(r.n_sub_opens_by_ladder[2], 1, "配对短差开");
        assert_eq!(r.n_osc_zd_restores_by_ladder[2], 1, "ZD 触线几何接回");
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "sub_diff")
            .collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_price, 100.0, "短差卖出价=实际变现点");
    }

    #[test]
    fn osc_trim_blocked_below_zg_and_without_theta() {
        // 位置门：c < ZG 不卖（049:52"在中枢上方"）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2));
        bars.push(sellpt(bar(50.5), 2)); // 50.5 < ZG=51 ⇒ 拒
        bars.push(bar(50.5));
        let r = run_va(bars, vec![], vec![]);
        assert_eq!(r.n_r2_pos_blocks_by_ladder[2], 1, "中枢上方位置门拒");
        assert_eq!(r.n_sub_opens_by_ladder[2], 0);
    }

    #[test]
    fn exempt_up_freezes_all_sell_vocab_including_t1() {
        // 026:80 豁免域（settled-tail-kind）：祖先 Trend∧Up 冻结全部卖词汇
        // 含 t1（fusion_td 粒度错配否证边界）。
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2));
        bars.push(sell_t1(bar(100.0), 2)); // t1 卖也被豁免域冻结
        bars.push(bar(100.0));
        let w = SUB_COST_MIN_OBS as i64;
        let r = run_va(bars, vec![(w, 3, Direction::Up)], vec![(w, 3, true)]);
        assert_eq!(r.n_trend_holds_by_ladder[2], 1);
        assert_eq!(r.n_anc_exempt_blocks_by_ladder[2], 1, "豁免域归因");
        assert_eq!(r.n_sub_opens_by_ladder[2], 0);
        assert_eq!(
            r.trades
                .iter()
                .filter(|t| t.ladder == 2 && t.exit_reason != "eod")
                .count(),
            0
        );
    }

    #[test]
    fn move_up_phase_freezes_non_t1_but_lets_divergence_exit() {
        // 自层 Φ=Move↑（confirmed Buy3 终结中枢→pending/dead）：非背驰卖
        // 冻结（049:52）；t1 背驰放行（049:54"直到移动出现背驰"）→ 翻转
        // 落点 Coin。
        let conf_b3 = |cs: i64| BspEvent {
            class: BspClass::Buy3,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price: 100.0,
        };
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2)); // 入场
                                        // confirmed Buy3 终结锚中枢（向上离开确认）⇒ Φ=Move↑。
        bars.push(with_ev(
            bar(100.0),
            2,
            conf_b3(10 + SUB_COST_MIN_OBS as i64 - 1),
        ));
        bars.push(sellpt(bar(110.0), 2)); // 非 t1 卖 ⇒ 冻结
        bars.push(sell_t1(bar(108.0), 2)); // t1 背驰 ⇒ 放行翻转
        bars.push(bar(108.0));
        let r = run_va(bars, vec![], vec![]);
        assert!(r.n_trend_holds_by_ladder[2] >= 1, "移动相非背驰冻结");
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "sellpt")
            .collect();
        assert_eq!(t2.len(), 1, "t1 背驰出口成交");
        assert_eq!(t2[0].exit_price, 108.0);
        assert_eq!(r.n_gate_enters_by_ladder[2], 1, "翻转落点 Coin");
    }

    #[test]
    fn coin_restore_blocked_in_movedown_until_t1() {
        // Coin 买链镜像：Φ=Move↓（confirmed Sell3 终结）非背驰买冻结；
        // t1 买放行（背驰接回）。
        let conf_s3 = |cs: i64| BspEvent {
            class: BspClass::Sell3,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price: 40.0,
        };
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2)); // 入场
        bars.push(sellpt(
            with_ev(bar(45.0), 2, conf_s3(10 + SUB_COST_MIN_OBS as i64 - 1)),
            2,
        ));
        // ↑ Sell3 终结中枢 ⇒ Φ=Move↓；同 bar 卖词汇 ⇒ 翻转轨道（逃命）→ Coin
        bars.push(buypt(bar(30.0), 2)); // Move↓ 非 t1 买 ⇒ 冻结（不接刀）
        let mut b = buypt(bar(28.0), 2);
        b.buy1 = LadderMask(1 << 2); // t1 买 ⇒ 放行（背驰接回）
        bars.push(b);
        bars.push(bar(28.0));
        let r = run_va(bars, vec![], vec![]);
        assert_eq!(r.n_gate_enters_by_ladder[2], 1, "逃命翻转 → Coin");
        assert!(r.n_short_trend_holds_by_ladder[2] >= 1, "Move↓ 停接刀 [镜]");
        assert_eq!(r.n_gate_restores_by_ladder[2], 1, "t1 背驰接回翻多");
    }

    #[test]
    fn hinge_escalates_on_dead_down_anchor() {
        // PairedOut 期间锚中枢被三卖终结 ⇒ 升级翻转（049:52"不能回补了"）。
        let conf_s3 = |cs: i64| BspEvent {
            class: BspClass::Sell3,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price: 60.0,
        };
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2));
        bars.push(sellpt(bar(100.0), 2)); // PairedOut（锚 cs=10+w−1）
        bars.push(with_ev(bar(60.0), 2, conf_s3(10 + w - 1))); // 三卖终结锚
        bars.push(bar(60.0));
        let r = run_va(bars, vec![], vec![]);
        assert_eq!(r.n_sub_escalates_by_ladder[2], 1, "三卖 ⇒ 升级翻转");
        let t2: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 2 && t.exit_reason == "hinge_escalate")
            .collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_price, 100.0, "升级以实际变现价记账");
    }

    #[test]
    fn cost_gate_silences_layer_without_reference() {
        // 公理5：θ 未定义 ⇒ Q=false ⇒ Osc 词汇沉默（资格是存在谓词）。
        let mut b0 = with_empty_div(bar(100.0));
        b0.bsp_events.get_or_insert_with(Box::default); // 空事件行（磁带能力满足）
        let mut bars = vec![b0];
        bars.push(buypt(bar(100.0), 2)); // 无任何中枢参照 ⇒ Φ=Osc ∧ Q 未定义
        bars.push(bar(100.0));
        let r = run_va(bars, vec![], vec![]);
        assert_eq!(r.n_entries_by_ladder[2], 0, "无参照层不入场");
        assert!(r.n_sub_cost_rejects_by_ladder[2] >= 1, "Q 未定义短路拒绝");
    }

    #[test]
    fn nav_conservation_paired_roundtrip() {
        // 配对闭环往返的 NAV 守恒：卖 @100 接回 @45 ⇒ 净现金 +55/股。
        let mut bars = warmup(2);
        bars.push(buypt(bar(40.0), 2)); // 2500 股 @40（全配额 100k）
        bars.push(sellpt(bar(100.0), 2)); // 卖出 ⇒ pool=250k
        bars.push(bar(45.0)); // 接回 2500 股 @45 ⇒ pool=137.5k + 股
        bars.push(bar(45.0));
        let r = run_va(bars, vec![], vec![]);
        // eod: 137500 + 2500×45 = 250000
        assert!((r.final_nav - 250_000.0).abs() < 1e-6, "配对闭环 NAV 守恒");
    }
}
