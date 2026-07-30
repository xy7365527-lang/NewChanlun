//! recursive_nested_fugue — 递归嵌套多重赋格（"平多≠开空"推到极限；mode = "rnf"）。
//!
//! 设计源头：`docs/concept_movement_chain.md`（23 环）+ `docs/nested_fugue_
//! accounting.md`（§1-§8 严格会计）。编排者 2026-06-13："从递归嵌套多重赋格
//! 这一个概念重写交易层——吃到每一笔。不是 patch，不是补缺口。"
//!
//! ## 与 v4（nested_fugue）/ URS（unified_recursive）的唯一构成性差异：清仓
//!
//! 三个系统会计原语逐字相同（§1-§8 = nested_fugue pub(super) 复用，bit-exact）。
//! 唯一差异是 **C 清仓块**——"平多"在不同级别归属上的不同形态：
//!
//! - **v4**：平多 = `confirmed Sell1@top ∧ located@top`（top = θ 振幅棘轮层）。
//! - **URS**：平多 = `sell1[E*] ∧ located[E*]`（E* = 根 voice 涌现归属级别）。
//! - **RNF（本系统）**：**根永不平多**（除 EOD 变现）。"平多 = 最高涌现级别走势
//!   真正完美"被推到极限——在任何有限回测窗口内**近乎不发生**（会计 §6"十年
//!   1-2 次"）。绝大多数"卖出"是**降成本（对子级别 = 开空）**，不是平多。全部
//!   regime 适应来自子 voice 开空的**存活/死亡**——净多头暴露 = 根 long units =
//!   N − Σ(在场子空 units)，**随走势结构呼吸**，无任何离散清仓决策。
//!
//! ### 为什么"根永不平多"是"平多≠开空"的严格形式
//!
//! 编排者核心强调：平多 ≠ 开空。
//! - **平多** = 该级别 voice 走势**真正完美**（最高涌现级别 type1），关闭头寸。
//!   这是十年 1-2 次的事件——回测窗口内当作"永不"。
//! - **开空** = 子级别检测到反向走势，创建独立逐仓空头 voice（降成本，待回补）。
//!   这是每个次级别卖点都发生的高频事件。
//!
//! v4/URS 把"根层 type1 背驰"当平多触发器（清仓 N→0）。但根层 type1 ≠ 最高
//! 涌现级别走势完美——它只是**根自己级别**的次级别走势完美 = 应当**开空**
//! （降成本），不是平多。v4/URS 的踏空（BTC/ES/BRN < BH）正源于此误判：强牛中
//! 每个 type1 后创新高，把"开空时机"当"平多时机"⇒ 清仓踏空。RNF 修正：根层及
//! 以下的一切卖点 = 开空（spawn 子空），根的 N 股**只在 EOD 变现** = 持有到底。
//!
//! ### 净暴露呼吸 = 连续 regime 适应（替代离散清仓二难）
//!
//! v4/URS 的离散清仓是 regime 函数（强牛要不清、崩盘要清，同一 type1 信号
//! 决策时不可区分——539 号开放轴）。RNF 用**子空存活/死亡**替代离散决策：
//!   · 强牛：子空 spawn 后价格创新高 ⇒ 027:25 否定线触发 ⇒ 子空死（付轧空税
//!     m(c−P)/c 缩水）⇒ 净暴露回 N ⇒ 骑住涨势（≈BH，无清仓踏空）。
//!   · 崩盘：子空 spawn 后价格续跌 ⇒ 否定线不破 ⇒ 子空存活 ⇒ 净暴露持续低
//!     ⇒ 被保护（无需离散清仓）。
//! regime 判别**涌现自子空命运**，不是预先决策。轧空税（强牛中死掉的子空）
//! vs 保护（崩盘中存活的子空）的净额是否使 8/8≥BH 成立 = 纯经验 L3 问题。
//!
//! ## 五条全局不变量（会计 §8，每 bar 强制检查——违反即 Err，矛盾显形非吞错）
//!
//! 1. **Σunits = N**（恒仓）：链上在手单位和 = N_base（重定基后）。
//! 2. **NAV 同价守恒**（单次记账 + 视图一致）：bar 内所有操作在价格 c 下 NAV
//!    中性——nav_pre(c) == nav_post(c)。一笔物理交易只记一次，子/父双视图导出
//!    自同一物理量。
//! 3. **零强平**（027:25 否定线先于保证金线）：子空 1x 逐仓强平计数应恒 0。
//! 4. **NAV ≥ 0**（清偿性）：任意 bar NAV 非负。
//! 5. **子 P&L ≡ 父降成本**（结构性，pop_tail 同一数字传导）：回补时子视图
//!    P&L = 父 cost_pool 递减额——同一现金流，不双写。
//!
//! ## 概念链 → 代码映射（23 环全覆盖）：同 unified_recursive.rs 头注（环 1-23）。
//! 唯一区别：环 23"出场=翻转=新建仓"在 RNF 中退化为"建仓一次→持有到底"
//! （根永不平多 ⇒ 无翻转循环；翻转通过子空的方向交替体现，环 22）。
//!
//! ## 每 bar 优先序（同 bar 单事件；§1-§8 会计 = v4 复用，bit-exact）
//!
//! A. 强平兜底 → B. 否定扫描（子空死=轧空税）→ D. 尾回补（子级别走势完美=
//! 降成本兑现）→ E. spawn 降成本（开空）→ F. 根入场（链空，仅一次）。
//! **无 C 清仓块**——这是与 v4/URS 的唯一差异。
//!
//! ## 零 flag 声明
//!
//! 唯一参数 = a0 粒度（K线周期，数据决定）+ floor_ladder（结构常量 = BSP
//! 承载层下界，所有标的同）。无 ClearanceMode、无 regime 门、无标的白名单。

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

/// 五条全局不变量检查（§8）。`nav_pre` = bar 起始（操作前）在价格 c 下的 NAV。
/// 违反硬会计不变量（1/2/4）即 Err（fail-fast）；强平（3）由调用方计数后另报。
fn check_invariants(
    chain: &[Voice],
    free: f64,
    n_base: f64,
    c: f64,
    nav_pre: f64,
    bar: i64,
) -> Result<(), String> {
    // §8.1 Σunits = N。
    let sum_units: f64 = chain.iter().map(|v| v.units).sum();
    if (sum_units - n_base).abs() > 1e-6 * n_base.max(1.0) {
        return Err(format!(
            "不变量1违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}（恒仓 §8.1）"
        ));
    }
    // §8.2 NAV 同价守恒（bar 内操作 NAV 中性——单次记账 + 视图一致）。
    let nav_post = nav(chain, free, c);
    let tol = 1e-6 * nav_pre.abs().max(1.0);
    if (nav_post - nav_pre).abs() > tol {
        return Err(format!(
            "不变量2违反@bar {bar}：NAV 同价不守恒 pre={nav_pre} post={nav_post}（§8.2 单次记账）"
        ));
    }
    // §8.4 NAV ≥ 0（清偿性；零强平 §8.5 保证下不应触及）。
    if nav_post < -tol {
        return Err(format!(
            "不变量4违反@bar {bar}：NAV={nav_post} < 0（清偿性 §8.4）"
        ));
    }
    Ok(())
}

/// 主入口（`PolarityMode::RecursiveNested` 经 `run_positional` 分派至此）。零 flag。
pub(crate) fn run_recursive_nested_fugue(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "recursive_nested_fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})（BSP \
             承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("recursive_nested_fugue 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "recursive_nested_fugue 要求背驰磁带 + dir_flips 行——区间套次级别证据 \
             词汇 = BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理），缺行即词汇残缺"
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

    // 区间套窗口（双侧，市场性质——candidate 即武装，消费时才查链状态）。
    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        let bar = i as i64;

        // 操作前 NAV（不变量2 同价守恒基准——价格 c 下，操作前后应相等）。
        let nav_pre = nav(&chain, free, c);

        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        while flip_ptr < flips.len() && flips[flip_ptr].0 == bar {
            let (_, lad, dir) = flips[flip_ptr];
            flip_edge[lad as usize] = Some(dir);
            flip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 区间套窗口维护（第14环）：① 打破否定 → ② candidate 武装 /
        //    confirmed 清窗 → ③ 递归证据触发（nf_* 携带触发时极值）──
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
            // 第14环：递归下探至 a0。证据层 < k−1 ⇒ 深触发。
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

        // 当前最高 θ 涌现层（F 入场层）。
        let top = (floor_ladder..MAX_LADDER).rev().find(|&k| {
            depth_ref
                .theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS)
                .is_some()
        });

        // ── 根涌现归属上移（环13级别=递归层次 + 环23"建仓 at 最高涌现级别"）──
        //    根持 N 长 = 大势多头。大势级别随更高 θ 涌现层出现而上升 ⇒ 根 ladder
        //    跟随最高涌现层 top **上移**（仅 re-label，无物理交易，NAV/units 中性
        //    ⇒ §8.1/§8.2 不变量保持）。这是"不清仓即迁移"——区别 v4/URS 的清仓-
        //    再入场迁移（那是 regime 函数不可约的来源）。
        //
        //    没有上移则根冻结在入场层（早期历史 top=floor ⇒ 根困于 a0 底，区间套
        //    递归无处下探 ⇒ 退化为 buy-and-hold，已实测 spawns≡0）。上移让根骑大势
        //    至最高涌现层、向下 spawn 降成本子 voice 贯通到 a0 = 递归嵌套赋格本体。
        if let Some(t) = top {
            if let Some(root) = chain.first_mut() {
                if root.dir == Polarity::Long && root.ladder < t {
                    root.ladder = t;
                }
            }
        }

        // ── A. 强平兜底（尾空头 1x 逐仓解析强平；§8.5 零强平应使其恒 0）──
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

        // ── B. 否定扫描（根→尾第一个破 027:25 极值线 ⇒ 该层及以深解栈）。
        //    子空死 = 走势创新高否定降成本前提 = 轧空税缩水（pop_tail 物化）──
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

        // ── （无 C 清仓块）── 根永不平多。根层及以下的一切卖点 = 开空（走 E
        //    降成本 spawn）。这是 RNF 与 v4/URS 的唯一构成性差异。──

        // ── D. 尾回补（非根尾的 confirmed 反向点@own-level = 子级别走势完美
        //    ⇒ 平子 + 父回满 + cost_pool 传导 + earning；降成本兑现）──
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

        // ── E. spawn（降成本 = 开空）：尾的 nest 定位反向点@own-level ∨ 根尾
        //    的 confirmed 卖（一切未被 D 消费的卖点，§9"其他卖点全部走E"——
        //    RNF 中"其他"= 除 EOD 外的全部）⇒ 释放 θ 配额 m 给子 voice。
        //    终止 = floor ∨ 35课成本门 ──
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
                    if tail.ladder == floor_ladder {
                        res.n_nrf_floor_stops_by_ladder[tail.ladder] += 1;
                    } else {
                        let sub = tail.ladder - 1;
                        match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                            None => res.n_nrf_noref_rejects_by_ladder[sub] += 1,
                            Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                                res.n_nrf_cost_rejects_by_ladder[sub] += 1;
                            }
                            Some(_) => {
                                let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
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

        // ── F. 根入场（链空）：最高 θ 涌现层买证据 ⇒ 满仓开多。RNF 中根永不
        //    平多 ⇒ 链仅在 EOD 后清空，故 F 实际只触发一次（建仓）──
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

        // ── 五条全局不变量（§8，每 bar 强制检查）──
        check_invariants(&chain, free, n_base, c, nav_pre, bar)?;

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

    // eod：全链解栈（根唯一的"平多"——变现，非走势完美触发）。
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

    const RNF: PolarityMode = PolarityMode::RecursiveNested;

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

    fn sell1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热（层 3/4 各 SUB_COST_MIN_OBS 个锚；同 nested_fugue）。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)),
            );
            b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
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
            ..Default::default()
        };
        run_positional(&t, 2, RNF).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("rnf"), Some(RNF));
        // 缺背驰磁带 ⇒ Err。
        let t = SignalTape {
            bars: vec![with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            )],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, RNF).unwrap_err().contains("背驰磁带"));
    }

    #[test]
    fn root_enters_full_position_at_top() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(bar(101.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "根开在最高 θ 涌现层");
        assert!((r.final_nav - 101_000.0).abs() < 1e-6);
    }

    #[test]
    fn root_never_clears_on_type1() {
        // RNF 核心：根层 type1 背驰 ∧ located 都**不**清仓——走 E 降成本 spawn。
        // 这是与 v4/URS 的唯一差异（v4/URS 此处清仓）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根@4 入场
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // located[4]
        bars.push(sell1pt(bar(103.0), 4)); // 根层 type1——RNF 不清仓，spawn 子空
        bars.push(bar(103.0));
        let r = run(bars);
        assert!(
            r.trades.iter().all(|t| t.exit_reason != "sellpt"),
            "RNF 根永不平多——type1 走降成本，无 sellpt 出场"
        );
        assert_eq!(
            r.n_nrf_spawns_by_ladder[3], 1,
            "根层卖点 = 开空（spawn 子空@3）"
        );
        let root = r.trades.iter().find(|t| t.ladder == 4).unwrap();
        assert_eq!(root.exit_reason, "eod", "根持有到 EOD（唯一变现）");
    }

    #[test]
    fn spawn_releases_theta_quota() {
        // §2 卖出原子 = 开空：根@4 confirmed 卖 ⇒ 释放 m=N×θ₃/θ_total 给子空@3。
        let mut bars = warmup34();
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
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "子空@3 诞生");
        let short = r
            .trades
            .iter()
            .find(|t| t.polarity == Polarity::Short)
            .unwrap();
        assert!((short.shares - 250.0).abs() < 1e-9, "θ 配额 m = 1000×1/4");
        assert!(
            (r.final_nav - 104_000.0).abs() < 1e-6,
            "final={}",
            r.final_nav
        );
    }

    #[test]
    fn recovery_refills_parent_reduces_cost() {
        // §3 回补原子 = 降成本兑现：confirmed 买@3 ⇒ 平子 + 父回满 + 降成本。
        let mut bars = warmup34();
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
        bars.push(buypt(bar(96.0), 3)); // 子级别走势完美 ⇒ 回补
        bars.push(bar(96.0));
        let r = run(bars);
        let rec = r
            .trades
            .iter()
            .find(|t| t.exit_reason == "recover")
            .unwrap();
        assert_eq!((rec.ladder, rec.polarity), (3, Polarity::Short));
        let pnl = rec.shares * (rec.entry_price - rec.exit_price);
        assert!(
            (pnl - 2000.0).abs() < 1e-6,
            "子 P&L = 250×8 = 2000（≡ 父降成本）"
        );
        assert!(
            (r.final_nav - 98_000.0).abs() < 1e-6,
            "final={}",
            r.final_nav
        );
    }

    #[test]
    fn negation_pays_squeeze_tax() {
        // 027:25 否定 = 轧空税：子空@104 线 110，破 110 ⇒ 子死，N 缩水
        // δ = 250 − 26000/111（强牛中降成本失败的物理代价）。
        let mut bars = warmup34();
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
        bars.push(bar(111.0)); // 破 110 ⇒ 否定
        bars.push(bar(111.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_negate_closes_by_ladder[3], 1);
        let expect_back = 26_000.0 / 111.0;
        assert!(
            (r.nrf_shrink_units - (250.0 - expect_back)).abs() < 1e-9,
            "轧空税缩水"
        );
        // NAV = (750 + 234.23)×111 = 750×111 + 26000（守恒）。
        assert!((r.final_nav - (750.0 * 111.0 + 26_000.0)).abs() < 1e-6);
    }

    #[test]
    fn invariants_hold_through_recursion() {
        // 五不变量每 bar 成立（引擎内部 check_invariants 未 Err 即通过）+
        // 守恒：递归流转不增不减。
        let mut bars = warmup34();
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
        bars.push(buypt(bar(95.0), 3));
        bars.push(bar(95.0));
        let r = run(bars);
        let expect = 1000.0 * 95.0 + 250.0 * 9.0;
        assert!(
            (r.final_nav - expect).abs() < 1e-6,
            "final={} expect={expect}",
            r.final_nav
        );
        // 零强平（§8.5）。
        assert_eq!(r.n_short_liquidations_by_ladder.iter().sum::<u64>(), 0);
    }
}
