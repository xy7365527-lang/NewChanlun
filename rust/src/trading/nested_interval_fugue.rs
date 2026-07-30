//! nested_interval_fugue — 区间套递归赋格（mode = "nif{N}"，N = min_trade_ladder）。
//!
//! 设计源头：`analysis/why_not_profitable.md`（alpha 瓶颈 L3 诊断）+
//! `docs/concept_movement_chain.md`（第14/15/16/17环）+ `docs/nested_fugue_
//! accounting.md`（§1-§8 严格会计，逐字复用 `nested_fugue` 原语）。
//!
//! 任务（2026-06-14 编排者）："改操作语义层——BSP 的消费规则。" 不改信号层
//! （`buysellpoint.rs` 不动）、不改会计层（守恒律 + 双层记账逐字复用 URS/v4），
//! 只改 **BSP → 仓位的映射规则**。本引擎是 `unified_recursive`（URS，当前最优
//! 基座）的**构成性推广**：加一个 `min_trade_ladder` 交易 floor，把"所有级别的
//! BSP 被平等消费"改为"按级别分层消费"。
//!
//! ## 与 URS 的唯一构成性差异：min_trade_ladder（交易 floor）
//!
//! URS 的递归向下递归到 `floor_ladder = FIRST_BSP_LADDER = 2`（segment），即
//! 每个级别的反向信号都 spawn 降成本子 voice，**包括 segment 级**。诊断
//! （`why_not_profitable.md` §3/§6）：segment 级（占 99% 笔数）close 可实现幅度
//! 0.02–0.19% **全部低于摩擦地板（0.2%）**——方向 70% 正确也救不了，幅度才是
//! alpha。alpha 密度集中在递归高级别（recL2 +2~25%、recL3 +12~75%/swing）。
//!
//! 本引擎引入 `min_trade_ladder`：**势在 < min_trade_ladder 的级别不存在**
//! （第16环"成本门=递归终止"的**结构形态**——不是 θ<friction 的连续阈值，是
//! 一个离散级别 floor：低于它的幅度系统性低于摩擦，势消失）。三层改动：
//!
//! ### 改动1（segment 级别不独立开仓 = 成本门递归终止；第16环）
//! 只有 `ladder ≥ min_trade_ladder` 的 BSP 才触发独立开仓（F 根入场 / E spawn /
//! C 翻转）。低于 min_trade_ladder 的 BSP **仍被引擎检测、仍武装区间套窗口、仍
//! 参与递归定位（located 链）**——信号层零改动，定位词汇完整——但**交易层不
//! 消费它们开仓**。代码形式：F 的 `top` 搜索 `[min_trade_ladder, MAX)`；E 的
//! spawn floor 从 `floor_ladder` 抬到 `min_trade_ladder`（`tail.ladder ≤
//! min_trade_ladder ⇒ floor_stop`）；C 翻转的次级别落点 `ladder−1 ≥
//! min_trade_ladder`。
//!
//! ### 改动2（仓位集中在高级别，区间套定位；第14/15环）
//! F 在 **最高 θ 涌现层 top**（≥ min_trade_ladder）满仓开多——仓位压倒性偏置
//! 高级别（"级别=操作量"，第15环：高级别势大→大仓位）。入场点由区间套向下
//! 定位（`buy_any[top] ∨ nf_buy[top]`，nf 经 `rec_sub_evidence` 下探至 a0 精确
//! 定位）。E 释放的降成本子 voice 量 = `θ_sub/θ_total` 配额（53课留白；θ_total
//! 只跨可交易层 `[min_trade_ladder, MAX)`——untradeable 层的 θ 不进分母）。
//!
//! ### 改动3（出场对齐同级别反向 BSP；第14环区间套 + 第17环降成本）
//! 根 voice 持有到**涌现归属级别 E\*（≥ 根入场级别）的反向 BSP** 才清仓/翻转
//! （C）——不被低级别反向信号提前平仓。低级别（< E\*）的反向 BSP 触发的是
//! **降成本子 voice spawn（E）**，不是父 voice 平仓。子 voice 持有到**自身级别**
//! 的反向 BSP 才回补（D，隔离平仓返父）。这正是 URS 的 C/D/E 结构——本引擎
//! 逐字保留，只在 spawn 落点上叠加 min_trade_ladder floor。
//!
//! ## 退化定理（验证锚）
//!
//! `min_trade_ladder = floor_ladder = FIRST_BSP_LADDER` ⇒ 三处 gate 全部退化为
//! URS 的 `floor_ladder` 判据（`tail.ladder == floor` / `top ∈ [floor,MAX)` /
//! `ladder != floor`），本引擎 **bit-exact 退化为 URS**。单测 `nif_floor_equals_urs`
//! 逐字段对账验证。⇒ 改动是 URS 之上的**严格叠加**，min > floor 是新实验点。
//!
//! ## 零额外概念 flag
//!
//! 唯一新参数 = `min_trade_ladder`（交易 floor，结构常量；与 a0 粒度、floor_ladder
//! 同为 deployment 常量，非 per-bar 状态、非 regime 门）。会计 §1-§8、区间套
//! 定位、E\* 涌现层判据全部逐字复用 URS。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::nested_fugue::{nav, pop_tail, rec_sub_evidence, unwind_to, Voice, Win};
use super::positional::{theta_weights, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 根 voice 的涌现归属级别 E\*（会计 §1"根 = 最高涌现级别"；URS
/// `root_emergent_ladder` 逐字复用——改动3 的"同级别或更高级别"判据本体）。
///
/// 从根入场级别 `root_ladder` 向上爬，父级别 lad+1 需同时满足
/// ① `dir_state[lad+1] == Up`（方向向上）② `anchor_state[lad+1] ≥
/// root_entry_bar`（该段在持仓期内由走势自身生长出来）。两者合取缺一不可
/// （仅① = 全局塔高 Climb 缺陷）。
fn root_emergent_ladder(
    root_ladder: usize,
    root_entry_bar: i64,
    dir_state: &[Option<Direction>; MAX_LADDER],
    anchor_state: &[i64; MAX_LADDER],
    max_l: usize,
) -> usize {
    let mut lad = root_ladder;
    while lad + 1 < max_l
        && dir_state[lad + 1] == Some(Direction::Up)
        && anchor_state[lad + 1] >= root_entry_bar
    {
        lad += 1;
    }
    lad
}

/// 递归确认（URS `recursive_confirmed` 逐字复用）：a0(bi) 方向翻向目标侧 ∧
/// `[FIRST_BSP_LADDER, estar]` 每层 located 完整。
///
/// **注意**：定位链下界恒为 `FIRST_BSP_LADDER`（非 min_trade_ladder）——改动1
/// 明确"低于 min_trade_ladder 的 BSP 仍参与递归定位"。located 是**确认证据**
/// （区间套词汇），不是**开仓动作**；只有开仓动作受 min_trade_ladder 约束。
fn recursive_confirmed(
    estar: usize,
    want: Direction,
    located: &[Option<f64>; MAX_LADDER],
    dir_state: &[Option<Direction>; MAX_LADDER],
) -> bool {
    if dir_state[FIRST_BSP_LADDER - 1] != Some(want) {
        return false;
    }
    (FIRST_BSP_LADDER..=estar).all(|k| located[k].is_some())
}

/// 主入口（`PolarityMode::NestedInterval { min_trade_ladder }` 经 `run_positional`
/// 分派至此）。`floor_ladder` = 结构 BSP 承载层下界（=FIRST_BSP_LADDER）；
/// `min_trade_ladder` = 交易 floor（≥ floor_ladder）。
pub(crate) fn run_nested_interval_fugue(
    tape: &SignalTape,
    floor_ladder: usize,
    min_trade_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "nested_interval_fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !(floor_ladder..MAX_LADDER).contains(&min_trade_ladder) {
        return Err(format!(
            "nested_interval_fugue 要求 min_trade_ladder ∈ [floor_ladder={floor_ladder}, \
             {MAX_LADDER})（交易 floor ≥ 结构 floor，且 < 顶层——否则无可交易级别）；\
             min_trade_ladder={min_trade_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("nested_interval_fugue 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "nested_interval_fugue 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理）；E* 涌现层读 \
             dir_state + anchor_state，缺 dir_flips 行即判据残缺"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut chain: Vec<Voice> = Vec::with_capacity(MAX_LADDER);
    let mut free = INITIAL_CAPITAL;
    let mut n_base = 0.0f64;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut located_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let mut anchor_state: [i64; MAX_LADDER] = [-1; MAX_LADDER];

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
            dir_state[lad as usize] = Some(dir);
            anchor_state[lad as usize] = bar;
            flip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照（与持仓无关，事件 bar 驱动；信号层口径）。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 区间套窗口维护（第14环，改动3 的定位机制；改动1：定位在**所有**
        //    BSP 承载层运作——含 < min_trade_ladder，定位词汇完整）：① 打破否定
        //    → ② candidate 武装 / confirmed 清窗 → ③ 递归证据触发 ──
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
                    let win = if sellside {
                        &mut nest_sell[k]
                    } else {
                        &mut nest_buy[k]
                    };
                    if e.confirmed {
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
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, &flip_edge) {
                    nf_sell[k] = Some(w.extreme);
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, &flip_edge) {
                    nf_buy[k] = Some(w.extreme);
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }

        // 区间套定位记忆（§6.2"背驰已被区间套递归确认"）。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if let Some(ext) = nf_sell[k] {
                located_sell[k] = Some(ext);
            }
            if located_sell[k].is_some_and(|ext| c > ext) {
                located_sell[k] = None;
            }
        }

        // ── 改动1+2：最高 θ 涌现层（F 入场层）搜索下界 = min_trade_ladder
        //    （segment 及以下不作独立入场层；势不存在于交易 floor 之下）──
        let top = (min_trade_ladder..MAX_LADDER).rev().find(|&k| {
            depth_ref
                .theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS)
                .is_some()
        });
        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        // ── A. 强平兜底（尾空头 1x 逐仓解析强平）──
        let mut acted = false;
        if let Some(tail) = chain.last().copied() {
            if tail.dir == Polarity::Short && tail.capital + tail.units * (tail.basis - c) <= 0.0 {
                pop_tail(
                    bar,
                    2.0 * tail.basis,
                    c,
                    "liq",
                    false,
                    &mut chain,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                res.n_short_liquidations_by_ladder[tail.ladder] += 1;
                acted = true;
            }
        }

        // ── B. 否定扫描（根→尾第一个破 027:25 极值线 ⇒ 该层及以深解栈）──
        if !acted {
            let broke = chain.iter().position(|v| {
                v.negate_line.is_some_and(|line| match v.dir {
                    Polarity::Short => c > line,
                    Polarity::Long => c < line,
                })
            });
            if let Some(g) = broke {
                let lad = chain[g].ladder;
                unwind_to(
                    g,
                    bar,
                    c,
                    c,
                    "negate",
                    &mut chain,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted = true;
            }
        }

        // ── C. 清仓/翻转（改动3：根持有到 E\* 涌现层反向 BSP 才平——不被低级别
        //    反向信号提前平仓）。根 E\* 出现卖点（sell_any）∧ located[E\*] ∧
        //    递归链完整 ⇒ 翻转（= 降成本 m=N 特例，子空携 027:25 否定线）。
        //    改动1：翻转的次级别落点 `ladder−1 ≥ min_trade_ladder`；落点不可
        //    交易 ⇒ 退化为清仓到现金（不在 < min_trade_ladder 开空）──
        if !acted && chain.first().is_some_and(|r| r.units > 0.0) {
            let root = chain[0];
            let estar = root_emergent_ladder(
                root.ladder,
                root.entry_bar,
                &dir_state,
                &anchor_state,
                max_l,
            );
            if sig.sell_any.get(estar)
                && recursive_confirmed(estar, Direction::Down, &located_sell, &dir_state)
            {
                let flip_line = located_sell[estar];
                // 改动1：翻转落点 sub = ladder−1 须 ≥ min_trade_ladder（不在交易
                // floor 之下开空）。退化锚：min=floor 时 `ladder > min` ⟺
                // `ladder != floor`（ladder ≥ floor 恒成立）= URS。
                let can_flip = chain.len() == 1
                    && chain[0].ladder > min_trade_ladder
                    && chain[0].units > 0.0
                    && chain[0].units.is_finite();
                if can_flip {
                    let m = chain[0].units; // 根独存 ⇒ = N
                    let sub = chain[0].ladder - 1;
                    let cash_released = m * c; // step1：父卖出全部 N 股
                    chain[0].units -= m; // 父 husk（保留 cost_pool/basis）
                    chain.push(Voice {
                        ladder: sub,
                        dir: Polarity::Short,
                        units: m,
                        basis: c,
                        cost_pool: cash_released,
                        capital: cash_released, // step2：子用释放现金反向开空 N
                        entry_bar: bar,
                        negate_line: flip_line,
                    });
                    res.n_nrf_root_flips_by_ladder[sub] += 1;
                    res.n_entries_by_ladder[sub] += 1;
                    located_sell = [None; MAX_LADDER];
                    acted = true;
                } else if chain[0].ladder <= min_trade_ladder {
                    // 翻转落点不可交易（< min_trade_ladder）⇒ 清仓到现金。
                    let _ = flip_line;
                    unwind_to(
                        0,
                        bar,
                        c,
                        c,
                        "sellpt",
                        &mut chain,
                        &mut free,
                        &mut n_base,
                        &mut res,
                    );
                    located_sell = [None; MAX_LADDER];
                    acted = true;
                }
                // else：len>1（有降成本子）⇒ C no-op，子先经 D 独立回补，根稍后
                // 在 len==1 时翻（逐仓独立，不 collapse）。
            }
        }

        // ── D. 尾回补（改动3：子 voice 持有到**自身级别** confirmed 反向点 =
        //    子级别走势完美 ⇒ 隔离平仓返父 + cost_pool 传导 + earning 时机）──
        if !acted && chain.len() > 1 {
            let tail = *chain.last().expect("len>1");
            let perfected = match tail.dir {
                Polarity::Short => sig.buy_any.get(tail.ladder),
                Polarity::Long => sig.sell_any.get(tail.ladder),
            };
            if perfected {
                pop_tail(
                    bar,
                    c,
                    c,
                    "recover",
                    true,
                    &mut chain,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                acted = true;
            }
        }

        // ── E. spawn 降成本（改动1+2+3）：尾的 nest 定位反向点 ∨ 根尾 confirmed
        //    卖（C 未消费的一切卖点，§9"其他卖点全部走 E"）⇒ 释放 θ 配额 m 给子
        //    voice。**改动1：递归终止 floor = min_trade_ladder**（`tail.ladder ≤
        //    min_trade_ladder ⇒ floor_stop`——势在交易 floor 之下不存在，不 spawn
        //    segment 降成本散单）。**改动2：θ_total 只跨 [min_trade_ladder, MAX)**
        //    （untradeable 层 θ 不进配额分母）──
        if !acted {
            if let Some(tail) = chain.last().copied() {
                let (nest_fired, confirmed_sell_root) = match tail.dir {
                    Polarity::Long => (
                        nf_sell[tail.ladder],
                        chain.len() == 1 && sig.sell_any.get(tail.ladder),
                    ),
                    Polarity::Short => (nf_buy[tail.ladder], false),
                };
                if nest_fired.is_some() || confirmed_sell_root {
                    // 改动1：递归终止 floor 抬到 min_trade_ladder（退化锚：min=floor
                    // 时 `tail.ladder ≤ floor` ⟺ `tail.ladder == floor` = URS）。
                    if tail.ladder <= min_trade_ladder {
                        res.n_nrf_floor_stops_by_ladder[tail.ladder] += 1;
                    } else {
                        let sub = tail.ladder - 1;
                        match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                            None => res.n_nrf_noref_rejects_by_ladder[sub] += 1,
                            Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                                res.n_nrf_cost_rejects_by_ladder[sub] += 1;
                            }
                            Some(_) => {
                                // 改动2：配额分母 = 可交易层 θ 之和。
                                let (thetas, theta_total) =
                                    theta_weights(&depth_ref, min_trade_ladder);
                                let w = thetas[sub].map(|t| t / theta_total);
                                let m_quota = w.map_or(0.0, |w| tail.units * w);
                                let m = match tail.dir {
                                    Polarity::Long => m_quota,
                                    Polarity::Short => m_quota.min(tail.capital / c),
                                };
                                if m > 0.0 && m.is_finite() {
                                    let j = chain.len() - 1;
                                    let child_dir = match tail.dir {
                                        Polarity::Long => Polarity::Short,
                                        Polarity::Short => Polarity::Long,
                                    };
                                    let child = match child_dir {
                                        Polarity::Short => Voice {
                                            ladder: sub,
                                            dir: Polarity::Short,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: m * c,
                                            entry_bar: bar,
                                            negate_line: nest_fired,
                                        },
                                        Polarity::Long => Voice {
                                            ladder: sub,
                                            dir: Polarity::Long,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: 0.0,
                                            entry_bar: bar,
                                            negate_line: nest_fired,
                                        },
                                    };
                                    chain[j].units -= m;
                                    if child_dir == Polarity::Long {
                                        chain[j].capital -= m * c;
                                    }
                                    chain.push(child);
                                    res.n_nrf_spawns_by_ladder[sub] += 1;
                                    res.n_entries_by_ladder[sub] += 1;
                                    acted = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── F. 根入场（改动2：链空 ⇒ 最高 θ 涌现层 top（≥ min_trade_ladder）满仓
        //    开多——仓位压倒性偏置高级别，入场点由区间套定位 buy_any ∨ nf_buy）──
        if !acted && chain.is_empty() {
            if let Some(top) = top {
                let nf = nf_buy[top];
                if sig.buy_any.get(top) || nf.is_some() {
                    let units = free / c;
                    if units > 0.0 && units.is_finite() {
                        let line = if sig.buy_any.get(top) { None } else { nf };
                        chain.push(Voice {
                            ladder: top,
                            dir: Polarity::Long,
                            units,
                            basis: c,
                            cost_pool: free,
                            capital: 0.0,
                            entry_bar: bar,
                            negate_line: line,
                        });
                        n_base = units;
                        free = 0.0;
                        res.n_nrf_root_entries_by_ladder[top] += 1;
                        res.n_entries_by_ladder[top] += 1;
                    }
                }
            }
        }

        // ── 守恒律逐 bar 强制（任务裁决：violation = panic，非 Err——守恒违反是
        //    会计 bug，必须立即 abort，不静默吞错）。§8.1 股数守恒：Σ 链上在手
        //    单位 = N_base（pop_tail/unwind_to 会计原语逐字复用 nested_fugue，
        //    §11 审计 D1/D6 CONFORMS ⇒ NAV 不变性由构造保证）──
        let sum_units: f64 = chain.iter().map(|v| v.units).sum();
        assert!(
            (sum_units - n_base).abs() <= 1e-6 * n_base.max(1.0),
            "守恒律 §8.1 违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}"
        );

        // 观测：链深度直方图 + 物理暴露 + 各层视图持有 bar 计数。
        res.nrf_depth_bars[chain.len().min(MAX_LADDER - 1)] += 1;
        let long_units: f64 = chain
            .iter()
            .filter(|v| v.dir == Polarity::Long)
            .map(|v| v.units)
            .sum();
        let short_units: f64 = chain
            .iter()
            .filter(|v| v.dir == Polarity::Short)
            .map(|v| v.units)
            .sum();
        if long_units > 0.0 {
            res.nrf_phys_long_bars += 1;
        }
        if short_units > 0.0 {
            res.nrf_phys_short_bars += 1;
        }
        for v in &chain {
            res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, nav(&chain, free, c)));
        }
    }

    // eod：全链解栈。
    if !chain.is_empty() {
        let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
        let last_bar = (n as i64) - 1;
        unwind_to(
            0,
            last_bar,
            c_last,
            c_last,
            "eod",
            &mut chain,
            &mut free,
            &mut n_base,
            &mut res,
        );
    }
    res.final_nav = free;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    fn nif(min_trade_ladder: usize) -> PolarityMode {
        PolarityMode::NestedInterval { min_trade_ladder }
    }

    fn bar(close: f64) -> BarSig {
        BarSig {
            close,
            max_ladder: 5,
            ..Default::default()
        }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() {
            (Some(50.0), Some(60.0))
        } else {
            (None, None)
        };
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs,
            zd,
            zg,
            price,
        }
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

    fn sellanypt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热（层 2/3/4 各 SUB_COST_MIN_OBS 个锚——含 segment 层 2，使
    /// min_trade_ladder 的差异可观测）。
    fn warmup234() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(
                bar(100.0),
                2,
                ev_full(BspClass::Sell1, false, 0.0, Some(1 + j)),
            );
            b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
            b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            rows[4][0].zd = Some(50.0);
            rows[4][0].zg = Some(53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    fn run(
        bars: Vec<BarSig>,
        dir_flips: Vec<(i64, u8, Direction)>,
        min: usize,
    ) -> PositionalResult {
        let t = SignalTape {
            bars,
            dir_flips: Some(dir_flips),
            ..Default::default()
        };
        run_positional(&t, 2, nif(min)).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("nif3"), Some(nif(3)));
        assert_eq!(PolarityMode::parse("nif4"), Some(nif(4)));
        // floor=2 时 min=2 合法（= URS 退化）。
        assert_eq!(PolarityMode::parse("nif2"), Some(nif(2)));
        // min < floor ⇒ Err（floor 默认 2，min=1 不在 [2,MAX)）。
        let t = SignalTape {
            bars: vec![with_empty_div(with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            ))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(
            run_positional(&t, 2, nif(1)).is_err(),
            "min<floor 拒绝（parse 已挡 nif1）"
        );
        // 缺背驰磁带 ⇒ Err。
        let t2 = SignalTape {
            bars: vec![with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            )],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, nif(3))
            .unwrap_err()
            .contains("背驰磁带"));
    }

    #[test]
    fn parse_rejects_untradeable_digits() {
        assert_eq!(
            PolarityMode::parse("nif1"),
            None,
            "min<FIRST_BSP_LADDER 非法"
        );
        assert_eq!(PolarityMode::parse("nif0"), None);
        assert_eq!(PolarityMode::parse("nif"), None, "缺级别数字非法");
        assert_eq!(PolarityMode::parse("nifx"), None);
    }

    /// 退化定理：min_trade_ladder = floor_ladder = FIRST_BSP_LADDER(=2) ⇒ nif 与
    /// URS 逐字段 bit-exact（同一磁带）。改动是 URS 之上的严格叠加。
    #[test]
    fn nif_floor_equals_urs() {
        // 构造非平凡磁带：根入场 + 降成本 spawn + 回补（覆盖 E/D 路径）。
        let mk = || {
            let mut bars = warmup234();
            bars.push(buypt(bar(100.0), 4));
            bars.push(with_ev(
                bar(105.0),
                4,
                ev_full(BspClass::Sell1, false, 110.0, None),
            ));
            bars.push(with_ev(
                bar(104.0),
                3,
                ev_full(BspClass::Sell1, true, 0.0, None),
            ));
            bars.push(buypt(bar(96.0), 3));
            bars.push(with_ev(
                bar(98.0),
                2,
                ev_full(BspClass::Sell1, false, 99.0, None),
            ));
            bars.push(bar(97.0));
            bars
        };
        let flips = vec![(20i64, 1u8, Direction::Down)];
        let t_nif = SignalTape {
            bars: mk(),
            dir_flips: Some(flips.clone()),
            ..Default::default()
        };
        let t_urs = SignalTape {
            bars: mk(),
            dir_flips: Some(flips),
            ..Default::default()
        };
        let r_nif = run_positional(&t_nif, 2, nif(2)).unwrap();
        let r_urs = run_positional(&t_urs, 2, PolarityMode::UnifiedRecursive).unwrap();
        assert_eq!(
            r_nif.trades.len(),
            r_urs.trades.len(),
            "trade 笔数 bit-exact"
        );
        for (a, b) in r_nif.trades.iter().zip(r_urs.trades.iter()) {
            assert_eq!(a.ladder, b.ladder);
            assert_eq!(a.entry_bar, b.entry_bar);
            assert!((a.entry_price - b.entry_price).abs() < 1e-12);
            assert_eq!(a.exit_bar, b.exit_bar);
            assert!((a.exit_price - b.exit_price).abs() < 1e-12);
            assert!((a.shares - b.shares).abs() < 1e-12);
            assert_eq!(a.exit_reason, b.exit_reason);
            assert_eq!(a.polarity, b.polarity);
        }
        assert!(
            (r_nif.final_nav - r_urs.final_nav).abs() < 1e-9,
            "final_nav bit-exact"
        );
        assert_eq!(r_nif.n_nrf_spawns_by_ladder, r_urs.n_nrf_spawns_by_ladder);
        assert_eq!(
            r_nif.n_nrf_floor_stops_by_ladder,
            r_urs.n_nrf_floor_stops_by_ladder
        );
    }

    /// 改动1：root@4 confirmed 卖在 min=4 时**不** spawn 子@3（sub=3 < min=4 ⇒
    /// floor_stop）；min=3 时 spawn 子@3（sub=3 ≥ min=3）。同一磁带，min 不同
    /// 行为分离——交易 floor 真实生效。
    #[test]
    fn min_trade_ladder_gates_spawn() {
        let mk = || {
            let mut bars = warmup234();
            bars.push(buypt(bar(100.0), 4));
            bars.push(with_ev(
                bar(105.0),
                4,
                ev_full(BspClass::Sell1, false, 110.0, None),
            ));
            bars.push(with_ev(
                bar(104.0),
                3,
                ev_full(BspClass::Sell1, true, 0.0, None),
            ));
            bars.push(bar(104.0));
            bars
        };
        let r3 = {
            let t = SignalTape {
                bars: mk(),
                dir_flips: Some(vec![]),
                ..Default::default()
            };
            run_positional(&t, 2, nif(3)).unwrap()
        };
        let r4 = {
            let t = SignalTape {
                bars: mk(),
                dir_flips: Some(vec![]),
                ..Default::default()
            };
            run_positional(&t, 2, nif(4)).unwrap()
        };
        assert_eq!(
            r3.n_nrf_spawns_by_ladder[3], 1,
            "min=3：子@3 诞生（sub=3≥min）"
        );
        assert_eq!(
            r4.n_nrf_spawns_by_ladder[3], 0,
            "min=4：子@3 被 floor_stop（sub=3<min）"
        );
        assert_eq!(
            r4.n_nrf_floor_stops_by_ladder[4], 1,
            "min=4：根@4 卖点触 floor_stop"
        );
        // 改动1 不变量：min=4 时根全程满仓持有到 eod（无降成本散单）。
        let eod4: Vec<_> = r4
            .trades
            .iter()
            .filter(|t| t.exit_reason == "eod")
            .collect();
        assert_eq!(eod4.len(), 1, "min=4：单根满仓到 eod");
        assert_eq!(eod4[0].ladder, 4);
    }

    /// 改动1（入场层 floor）：最高 θ 涌现层 = 3，但 min=4 ⇒ top 搜索 [4,MAX) 无
    /// 参照 ⇒ 根不入场（势不存在于可交易级别）。min=3 ⇒ 入场@3。
    #[test]
    fn min_trade_ladder_gates_root_entry() {
        // 只预热层 2/3（最高 θ 层 = 3）。
        let mut warm = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(
                bar(100.0),
                2,
                ev_full(BspClass::Sell1, false, 0.0, Some(1 + j)),
            );
            b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            warm.push(if j == 0 { with_empty_div(b) } else { b });
        }
        let mk = || {
            let mut bars = warm.clone();
            bars.push(buypt(bar(100.0), 3));
            bars.push(bar(101.0));
            bars
        };
        let r3 = {
            let t = SignalTape {
                bars: mk(),
                dir_flips: Some(vec![]),
                ..Default::default()
            };
            run_positional(&t, 2, nif(3)).unwrap()
        };
        let r4 = {
            let t = SignalTape {
                bars: mk(),
                dir_flips: Some(vec![]),
                ..Default::default()
            };
            run_positional(&t, 2, nif(4)).unwrap()
        };
        assert_eq!(r3.n_nrf_root_entries_by_ladder[3], 1, "min=3：根入场@3");
        assert_eq!(
            r4.n_nrf_root_entries_by_ladder.iter().sum::<u64>(),
            0,
            "min=4：无可交易入场层"
        );
        assert!(
            (r4.final_nav - INITIAL_CAPITAL).abs() < 1e-9,
            "min=4：全程现金"
        );
    }

    /// 改动3：根@4 持有，根层(4) 自身 confirmed 卖（E\*=4，无更高 Up 段）走 E
    /// 降成本（min=3 ⇒ spawn@3），**不**清仓——低级别反向信号不提前平根仓。
    #[test]
    fn exit_aligned_root_level_not_premature() {
        let mut bars = warmup234();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellanypt(bar(105.0), 4)); // 根层 confirmed 卖 ⇒ E 降成本
        bars.push(bar(105.0));
        let r = run(bars, vec![], 3);
        assert_eq!(
            r.n_nrf_spawns_by_ladder[3], 1,
            "根层卖点走 E 降成本（非清仓）"
        );
        assert!(
            r.trades.iter().all(|t| t.exit_reason != "sellpt"),
            "无 sellpt——根仓未被低级别信号提前平"
        );
    }

    /// 守恒律：递归流转每 bar Σunits=N_base（引擎内 assert 守卫；未 panic 即通过）
    /// + 现金流闭合 final_nav 重建。
    #[test]
    fn conservation_through_recursion() {
        let mut bars = warmup234();
        bars.push(buypt(bar(100.0), 4)); // 根 1000@100
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // 子空 250@104
        bars.push(buypt(bar(95.0), 3)); // 子走势完美 ⇒ 回补@95
        bars.push(bar(95.0));
        let r = run(bars, vec![], 3);
        // 根回满 1000@95 + 子利润 250×(104−95)=2250。
        let expect = 1000.0 * 95.0 + 250.0 * 9.0;
        assert!(
            (r.final_nav - expect).abs() < 1e-6,
            "final={} expect={expect}",
            r.final_nav
        );
    }
}
