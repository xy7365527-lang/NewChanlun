//! nested_fugue — 嵌套递归赋格 v5（清仓参照系修复；mode = "nrf"）。
//!
//! 设计规格：`docs/nested_fugue_accounting.md`（2026-06-12 编排者）。
//! 核心修正（对 v2/v3 的否定）：**卖点不清仓——绝大多数卖点只是降成本
//! （释放 m 给子 voice），清仓只在最高涌现级别走势完美时发生（十年 1-2
//! 次）。这是"平多≠开空"的终极形式。** v2 的 C 规则（根完美→全链清算）
//! 摧毁了嵌套递归；v3 的根翻转（字面翻空）已被八标的 L3 否证（P1 2/8，
//! BTC 强平归零，见 `analysis/nrf_v3_root_flip_recursive_nest.md`）。
//!
//! ## v5 修复（清仓参照系；`analysis/cl_clearance_diagnosis.md`）
//!
//! v4 在深崩域（CL +2.7% vs v2 +221.5%）坍塌的双机制根因被 L2 定位：
//! **修复A（清仓参照系解耦棘轮）**：v4 的清仓参照层 `top` =「历史累积
//! ≥10 中枢的最高阶梯」是单调攀高的**棘轮**，叠加「confirmed Sell1@top ∧
//! located 同层同 bar」合取，把 CL（5.5M bar 高波动）的清仓频率压死在
//! 3 次（与单边强牛 OKLO 相同）——波动率信号被棘轮稀疏性淹没。v5 改为
//! `top* = max{ k ≥ floor : located_sell[k] 活跃 }`（当前正在走势完美的
//! 最高级别），清仓 ⟺ confirmed Sell1@top*（located 由 top* 定义蕴含）。
//! 「走势终完美对**当前**走势成立，不是历史棘轮」⇒ 清仓频率成为波动率的
//! **涌现函数**（高波动 ⇒ 高级别下跌走势频繁完美 ⇒ located 频繁在高层
//! 活跃 ⇒ 清仓多；单边强牛 ⇒ 高级别完美罕见 ⇒ 清仓少），零 per-asset
//! 参数。入场层 `top`（棘轮 θ 涌现层）与清仓 `top*` 分离（诊断下游推论 3）。
//! **修复B（清仓后 root 真正重置）**：清仓后 located + nest 区间套窗口全清
//! ——「旧走势完美 = 结束，新走势独立 = 从 a0 重新涌现」。chain 已由
//! unwind_to(0) 归零（root 树重置），v5 补全区间套状态归零，使新树不继承
//! 旧走势的 candidate 窗口。修复A 让清仓频繁化 ⇒ F 频繁在棘轮 top 重入
//! ⇒ 恢复高级别重涌现（v2 的 alpha 源：清仓→root→爬到 recL3）。**不重置
//! depth_ref**——级别涌现是市场性质（因果），重置会使 root 落到 floor 层
//! （最低层最快累积 10 中枢）⇒ 递归深度归零，与 B 的目标（recL3 重涌现）
//! 自相矛盾。
//!
//! ## 会计规则（规格 §1-§8 的实装映射）
//!
//! 1. **N 恒仓**（§1/§6）：建仓 N 股后根 voice 永不清零；重定基仅两途：
//!    earning 增仓（N′=N+Δ）与亏损回补缩水（N′=N−δ，镜像）。
//! 2. **卖出原子**（§2）：voice@k 的卖词汇 ⇒ 释放 m 给子 voice@k−1
//!    （SHORT，capital = m×c）。m = 在手单位 × θ_{k−1}/θ_total——53课
//!    配额留白的在册裁决形态（hold26 θ 配额表逐字同源）；35课成本门
//!    决定**是否**可释放（θ_sub ≥ k×friction），零摩擦口径下振幅/摩擦
//!    比与 m 无关，量的留白由 θ 配额承接。
//! 3. **回补原子**（§3）：子 voice@k−1 的 confirmed 反向点 = 走势完美 ⇒
//!    平子 + 父回满（N−m → N）+ 父 cost_pool 减少（≡ 子 P&L，同一数字
//!    存一次）。亏损回补按 capital 可买量缩水（δ 传播 + N 重定基）。
//! 4. **词汇分工**（§3"或区间套定位" vs §5 的唯一自洽读法）：
//!    confirmed@own-level = 走势完美 = 平子回补；nest 定位@own-level =
//!    candidate 在场 = spawn 子声部（§5 的 k−2 买点即 nest_buy[k−1] 的
//!    递归证据）。confirmed 同侧清窗保证两词汇不同 bar 碰撞。
//! 5. **递归嵌套**（§5）：空头子在手 m 释放 m2 给多头孙（物理 = capital
//!    买入 m2 股）；孙平仓所得回流子 capital。任意深度同律。
//! 6. **清仓**（§6/§9；v5 修复A/B）：仅 confirmed sell@top*（top* = 当前
//!    located 活跃最高级别，**非** v4 棘轮累积层）触发全链解栈回现金 +
//!    区间套窗口归零（新树从 a0 重涌现）；root 自身层 < top* 的 confirmed
//!    卖 = 降成本 spawn。频率随波动率涌现（深崩域多清、单边强牛少清）。
//!    confirmed 卖（走势完美）= 清仓——§9"其他卖点全部走E"自动成立。
//! 7. **earning**（§7）：cost_pool ≤ 0 后回补纯利润在买点买入 Δ =
//!    excess/c，N 重定基。空头侧"挣负股数"L0 构造性不可表示（在册结算
//!    优先于规格 §7 对称声明）——命中计数观测，现金沉淀 capital 不增仓。
//! 8. **全局不变量**（§8）：每 bar 检验 Σ(链上在手单位) = N_base——违反
//!    即 Err（fail-fast，矛盾显形非吞错）。物理账存一次（无双写）；
//!    子 P&L ≡ 父降成本由同一数字传导保证；零强平由 027:25 否定线先于
//!    保证金线（A 规则兜底计数）。
//!
//! ## 区间套递归（v3 第14环，保留）
//!
//! candidate@k 触发证据从 k−1 起逐层下探直到 a0（bi 层方向翻转沿），
//! 任何一层同侧直接证据即触发。
//!
//! ## 每 bar 优先序（同 bar 单事件）
//!
//! A. 强平兜底（尾空头 capital + u×(basis−c) ≤ 0 ⇒ 1x 逐仓解析强平）
//! B. 否定扫描（根→尾第一个破 027:25 极值线 ⇒ 该层及以深解栈）
//! C. 清仓（v5：confirmed Sell1@top*（top* = 当前 located 活跃最高级别）
//!    ⇒ 全链解栈 + 区间套状态归零——频率随波动率涌现）
//! D. 尾回补（非根尾的 confirmed 反向点@own-level ⇒ 平子+父回满+earning）
//! E. spawn（尾的 nest 定位反向点@own-level ∨ 根尾非 top 的 confirmed 卖
//!    ⇒ 释放 θ 配额 m 给子；终止 = floor ∨ 35课成本门）
//! F. 根入场（链空 ⇒ 最高 θ 涌现层买证据满仓开多）
//!
//! 空头会计 [镜像推导]（38:36 镜像止于判断-动作序列）。在册路径零接触
//! （独立 PolarityMode 入口；nrf 计数器其余模式恒零）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{theta_weights, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 区间套窗口（candidate 武装/刷新极值、confirmed 同侧清窗、破极值否定）。
#[derive(Debug, Clone, Copy)]
struct Win {
    extreme: f64,
}

/// 链上一个 voice（一个级别的会计视图）。
#[derive(Debug, Clone, Copy)]
struct Voice {
    ladder: usize,
    dir: Polarity,
    /// 在手单位（多 = 持股；空 = 未回补敞口）。Σ 链上在手 = N_base。
    units: f64,
    /// 相位开仓均价（视图 P&L 锚；earning 增仓时加权更新）。
    basis: f64,
    /// 成本池（§7 earning 判据：回补利润递减，≤0 后纯利润）。
    cost_pool: f64,
    /// 空头 voice 在手现金（= 父层卖出所得 = 回补弹药；多头恒 0）。
    capital: f64,
    /// 相位开仓 bar。
    entry_bar: i64,
    /// 出生相否定线（spawn 时 candidate 极值，027:25）；confirmed 出生无。
    negate_line: Option<f64>,
}

/// 物理 NAV（单一真值）：自由现金 + 多头在手×价 + 空头在手现金。
/// 空头视图的未实现 P&L 在回补时以缩水/剩余现金形式物化（存一次）。
fn nav(chain: &[Voice], free: f64, c: f64) -> f64 {
    let mut v = free;
    for x in chain {
        match x.dir {
            Polarity::Long => v += x.units * c,
            Polarity::Short => v += x.capital,
        }
    }
    v
}

/// 次级别证据（区间套触发词汇）。
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

/// 递归区间套证据（第14环完整形式）：从 j = k−1 起逐层下探直到 a0
/// （j = FIRST_BSP_LADDER−1 = bi 层，词汇 = 方向翻转沿）。任何一层出现
/// 同侧直接证据即触发——终止条件仅 a0 ∨ 找到证据（任务裁决"递归到底"；
/// 武装窗口链门控读法在真实数据上域空已被否证）。返回证据层。
fn rec_sub_evidence(
    start: usize,
    side: Side,
    evrows: &[Vec<BspEvent>; MAX_LADDER],
    devrows: &[Vec<DivEvent>; MAX_LADDER],
    flip_edge: &[Option<Direction>; MAX_LADDER],
) -> Option<usize> {
    let lo = FIRST_BSP_LADDER - 1; // a0 下界（77-78课笔）
    (lo..=start)
        .rev()
        .find(|&j| sub_evidence(j, side, &evrows[j], &devrows[j], flip_edge[j]))
}

/// 结算一个 voice 的当前相位（trade 行 + 计数）。返回该相位视图 P&L。
fn settle_phase(
    v: &Voice,
    exit_bar: i64,
    exit_price: f64,
    reason: &'static str,
    res: &mut PositionalResult,
) -> f64 {
    let pnl = match v.dir {
        Polarity::Long => v.units * (exit_price - v.basis),
        Polarity::Short => v.units * (v.basis - exit_price),
    };
    if v.dir == Polarity::Short {
        res.short_net_cash_by_ladder[v.ladder] += pnl;
    }
    res.trades.push(LayerTrade {
        ladder: v.ladder as u8,
        entry_bar: v.entry_bar,
        entry_price: v.basis,
        exit_bar,
        exit_price,
        shares: v.units,
        weight_at_entry: 1.0,
        deferred_bars: 0,
        partial: false,
        exit_reason: reason,
        polarity: v.dir,
    });
    res.n_exits_by_ladder[v.ladder] += 1;
    pnl
}

/// 弹出链尾并物理解栈一层（trade 行价 = px；物理成交价 = 市价 c）。
///
/// 空头尾：capital 弹药买回 u′ = min(u, K/c)，缩水 δ = u−u′ 传播 + N
/// 重定基（亏损的物理形式，单位回不来）；剩余现金 = 降成本金额 ≡ 子
/// P&L（同一数字：父 cost_pool 递减，现金沉淀 free 或 earning 增仓）。
/// 多头尾（孙）：卖出所得回流父（空头）capital，利润递减父 cost_pool
/// （空头降成本 = 均价抬高的会计形式）。
/// 根弹出：在手变现回 free，N_base 归零。
///
/// `at_point`：本次解栈是否发生在 confirmed 买卖点（earning 增仓的
/// 时机约束，§7"必须在买点"）；cascade/negate/liq 路径为 false。
#[allow(clippy::too_many_arguments)]
fn pop_tail(
    bar: i64,
    px: f64,
    c: f64,
    reason: &'static str,
    at_point: bool,
    chain: &mut Vec<Voice>,
    free: &mut f64,
    n_base: &mut f64,
    res: &mut PositionalResult,
) {
    let v = *chain.last().expect("pop_tail 前提：链非空");
    settle_phase(&v, bar, px, reason, res);
    chain.pop();
    match chain.last_mut() {
        None => {
            debug_assert_eq!(v.dir, Polarity::Long, "根恒多头（v4 无根翻转）");
            *free += v.units * c + v.capital;
            *n_base = 0.0;
        }
        Some(parent) => match v.dir {
            Polarity::Short => {
                let u_back = v.units.min(v.capital / c);
                let leftover = v.capital - u_back * c;
                let shortfall = v.units - u_back;
                parent.units += u_back;
                *n_base -= shortfall;
                res.nrf_shrink_units += shortfall;
                let reduce = leftover.min(parent.cost_pool.max(0.0));
                parent.cost_pool -= reduce;
                let excess = leftover - reduce;
                if excess > 0.0 {
                    if at_point && parent.dir == Polarity::Long {
                        // §7 earning：纯利润在买点买入 Δ，N 重定基；
                        // basis 加权更新（视图锚与物理一致）。
                        let dq = excess / c;
                        parent.basis = (parent.basis * parent.units + excess)
                            / (parent.units + dq);
                        parent.units += dq;
                        *n_base += dq;
                        res.n_nrf_earning_adds_by_ladder[parent.ladder] += 1;
                        res.nrf_earning_units += dq;
                        *free += reduce;
                    } else {
                        if parent.dir == Polarity::Short {
                            res.nrf_short_earning_hits += 1;
                        }
                        *free += leftover;
                    }
                } else {
                    *free += leftover;
                }
            }
            Polarity::Long => {
                // 孙卖出：现金回流父（空头）capital；利润递减父成本池。
                // 空头 earning（挣负股数）L0 不可构造——现金留在 capital。
                let proceeds = v.units * c;
                parent.capital += proceeds;
                parent.units += v.units;
                let profit = v.units * (c - v.basis);
                let reduce = profit.max(0.0).min(parent.cost_pool.max(0.0));
                parent.cost_pool -= reduce;
                if profit > parent.cost_pool.max(0.0) + reduce && at_point {
                    res.nrf_short_earning_hits += 1;
                }
            }
        },
    }
}

/// 从链位置 g 起解栈到尾（逐层解栈：每层载体回流其直接父层——"附庸的
/// 附庸不是我的附庸"的会计形式）。g 层 trade 行用 g_price/reason，
/// 以深子孙用市价 c、reason="cascade"。
#[allow(clippy::too_many_arguments)]
fn unwind_to(
    g: usize,
    bar: i64,
    g_price: f64,
    c: f64,
    reason: &'static str,
    chain: &mut Vec<Voice>,
    free: &mut f64,
    n_base: &mut f64,
    res: &mut PositionalResult,
) {
    debug_assert!(g < chain.len(), "unwind_to 前提：g 在链上");
    while chain.len() > g {
        let j = chain.len() - 1;
        let lad = chain[j].ladder;
        let (px, why) = if j == g { (g_price, reason) } else { (c, "cascade") };
        if j > g {
            res.n_nrf_cascade_closes_by_ladder[lad] += 1;
        }
        pop_tail(bar, px, c, why, false, chain, free, n_base, res);
    }
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
    let mut chain: Vec<Voice> = Vec::with_capacity(MAX_LADDER);
    let mut free = INITIAL_CAPITAL;
    let mut n_base = 0.0f64;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // 区间套窗口（双侧，市场性质——candidate 即武装，消费时才查链状态）。
    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    // 卖侧区间套定位记忆（§6.2 清仓合取条件的载体）。
    let mut located_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];

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

        // ── 区间套窗口维护：① 打破否定 → ② candidate 武装/confirmed
        //    清窗 → ③ 递归证据触发（nf_* 携带触发时极值）──
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
                        *win = None; // confirmed 同侧让位（本 bar 走 confirmed 路径）
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

        // 区间套定位记忆（§6.2"背驰已被区间套递归确认"）：nf 触发时记录
        // 极值，价格破极值则定位失效（027:25 同律）；C 消费后清空。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if let Some(ext) = nf_sell[k] {
                located_sell[k] = Some(ext);
            }
            if located_sell[k].is_some_and(|ext| c > ext) {
                located_sell[k] = None;
            }
        }

        // 当前最高 θ 涌现层（v5：仅 F 入场层；C 清仓改用 located 活跃
        // 最高层 top*——修复A 把清仓参照系从棘轮累积层解耦）。
        let top = (floor_ladder..MAX_LADDER)
            .rev()
            .find(|&k| depth_ref.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS).is_some());

        // ── A. 强平兜底（尾空头 capital + u×(basis − c) ≤ 0；1x 逐仓
        //    解析强平——trade 行记 2×basis，物理按 capital 可买量缩水）──
        let mut acted = false;
        if let Some(tail) = chain.last().copied() {
            if tail.dir == Polarity::Short
                && tail.capital + tail.units * (tail.basis - c) <= 0.0
            {
                pop_tail(
                    bar, 2.0 * tail.basis, c, "liq", false, &mut chain, &mut free,
                    &mut n_base, &mut res,
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
                unwind_to(g, bar, c, c, "negate", &mut chain, &mut free, &mut n_base, &mut res);
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted = true;
            }
        }

        // ── C. 清仓（v5 修复A：参照系 = 当前正在走势完美的最高级别
        //    top* = max{ k ≥ floor : located_sell[k] 活跃 }，**非** v4 的
        //    棘轮累积层 top。清仓 ⟺ confirmed **背驰** Sell1@top*（located
        //    已由 top* 定义蕴含——「背驰已被区间套递归确认」）⇒ 全链级联
        //    解栈回现金。频率随结构涌现自然分化（高波动标的高级别下跌走势
        //    频繁完美 ⇒ 清仓多；单边强牛高级别完美罕见 ⇒ 清仓少）。其余
        //    一切卖点走 E 降成本（§9）。
        //    修复B：清仓后 located + nest 区间套窗口全清——旧走势完美即
        //    结束，新树从 a0 重涌现（root 真正重置：chain 已空 + 区间套
        //    状态归零）；depth_ref 不重置（级别涌现是市场因果性质）──
        if !acted && !chain.is_empty() {
            let top_clear = (floor_ladder..MAX_LADDER)
                .rev()
                .find(|&k| located_sell[k].is_some());
            if let Some(tc) = top_clear {
                if sig.sell1.get(tc) {
                    unwind_to(0, bar, c, c, "sellpt", &mut chain, &mut free, &mut n_base, &mut res);
                    located_sell = [None; MAX_LADDER];
                    nest_sell = [None; MAX_LADDER];
                    nest_buy = [None; MAX_LADDER];
                    acted = true;
                }
            }
        }

        // ── D. 尾回补（非根尾的 confirmed 反向点@own-level = 子级别走势
        //    完美 ⇒ 平子 + 父回满 + cost_pool 传导 + earning 时机）──
        if !acted && chain.len() > 1 {
            let tail = *chain.last().expect("len>1");
            let perfected = match tail.dir {
                Polarity::Short => sig.buy_any.get(tail.ladder),
                Polarity::Long => sig.sell_any.get(tail.ladder),
            };
            if perfected {
                pop_tail(bar, c, c, "recover", true, &mut chain, &mut free, &mut n_base, &mut res);
                acted = true;
            }
        }

        // ── E. spawn（降成本释放）：尾的 nest 定位反向点@own-level ∨
        //    根尾（层 < top）的 confirmed 卖 ⇒ 释放 θ 配额 m 给子 voice。
        //    终止 = floor（77-78课笔=a0）∨ 35课成本门 ──
        if !acted {
            if let Some(tail) = chain.last().copied() {
                // 根尾的 confirmed 卖：C 未消费（非背驰 ∨ 未定位）的一切
                // 卖点 = 降成本触发（§9"其他卖点全部走E"）。非根尾的
                // confirmed 反向点已在 D 消费（走势完美 = 回补）。
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
                                // m = 在手 × θ_sub/θ_total（53课配额留白的
                                // hold26 在册裁决形态）；空头父释放受 capital
                                // 可买量约束（资金守恒）。
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
                                        // 父多卖 m：所得 = 子空 capital。
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
                                        // 父空回补 m（capital 买入）：股 = 子多载体。
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

        // ── F. 根入场（链空）：最高 θ 涌现层买证据 ⇒ 满仓开多 ──
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

        // ── 全局不变量（§8.1）：Σ 链上在手单位 = N_base ──
        let sum_units: f64 = chain.iter().map(|v| v.units).sum();
        if (sum_units - n_base).abs() > 1e-6 * n_base.max(1.0) {
            return Err(format!(
                "不变量违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}（守恒律 §8.1）"
            ));
        }

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
        unwind_to(0, last_bar, c_last, c_last, "eod", &mut chain, &mut free, &mut n_base, &mut res);
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

    /// confirmed Sell1（背驰卖点——C 清仓词汇；同时置 sell_any）。
    fn sell1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热：层 3/4 各喂 SUB_COST_MIN_OBS 个锚（θ₃=1%、θ₄=3%，
    /// c=100 口径——两层均过 35课成本门；θ_total=4% ⇒ 根@4 释放配额
    /// m = N×0.01/0.04 = N/4）。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
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

    /// 层 2 预热补充（θ₂=0.5%；含层 2 时 θ_total=4.5%）。
    fn warmup2(bars: &mut Vec<BarSig>) {
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 2, ev_full(BspClass::Sell1, false, 0.0, Some(500 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            bars.push(b);
        }
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
        let t = SignalTape {
            bars: vec![with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, PolarityMode::NestedRecursive)
            .unwrap_err()
            .contains("背驰磁带"));
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
    fn root_enters_full_position_at_top() {
        // 根满仓（总资金 = 最高级别初始投入，零配额）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(bar(101.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "根开在最高 θ 涌现层");
        let t = &r.trades[0];
        assert_eq!((t.ladder, t.polarity), (4, Polarity::Long));
        assert!((t.shares - INITIAL_CAPITAL / 100.0).abs() < 1e-9, "满仓：units = 全部资金/价");
        assert!((r.final_nav - 101_000.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_releases_theta_quota_not_all() {
        // §2 卖出原子：nest 定位卖@4 ⇒ 释放 m = N×θ₃/θ_total = 250 股给
        // 子空，根保留 750 股（不清仓原则）。Σ 在手 = N 守恒。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根 1000 股 @100
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 证据@3
        bars.push(bar(104.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "子空@3 诞生");
        // eod 解栈：子空 104→104 平（pnl 0，cascade），根 1000 股@104 变现。
        let short = r.trades.iter().find(|t| t.polarity == Polarity::Short).unwrap();
        assert!((short.shares - 250.0).abs() < 1e-9, "θ 配额 m = 1000×1/4");
        // 解栈先平子（回流 250 股）再结算根——根行显示回满后的 1000 股；
        // spawn 后链上在手 = 750 + 250 由 §8.1 不变量逐 bar 保证。
        let root = r.trades.iter().find(|t| t.polarity == Polarity::Long).unwrap();
        assert!((root.shares - 1000.0).abs() < 1e-9, "根行 = 回补后满仓");
        // NAV：750×104 + 26_000(子 capital) = 104_000。
        assert!((r.final_nav - 104_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn recovery_refills_parent_and_reduces_cost_pool() {
        // §3 回补原子：confirmed 买@3 ⇒ 平子 + 父回满 N + 降成本现金沉淀。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 1000 股 @100，cost_pool=100_000
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 子空 250@104
        bars.push(buypt(bar(96.0), 3)); // 走势完美 ⇒ 回补@96
        bars.push(bar(96.0));
        let r = run(bars);
        let rec = r.trades.iter().find(|t| t.exit_reason == "recover").unwrap();
        assert_eq!((rec.ladder, rec.polarity), (3, Polarity::Short));
        // 子 P&L = 250×(104−96) = 2000 ≡ 父降成本（现金沉淀 free）。
        let pnl = rec.shares * (rec.entry_price - rec.exit_price);
        assert!((pnl - 2000.0).abs() < 1e-6);
        // 根回满 1000 股；eod NAV = 2000 + 1000×96 = 98_000。
        let root = r.trades.iter().find(|t| t.exit_reason == "eod").unwrap();
        assert!((root.shares - 1000.0).abs() < 1e-9, "父回满 N");
        assert!((r.final_nav - 98_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn negation_kills_child_with_shrink_rebase() {
        // 027:25 否定：破极值 ⇒ 子死，capital 追价买回缩水 δ 传播 + N
        // 重定基（亏损的物理形式——§8.1 守恒在重定基下保持）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 1000 股
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 子空 250@104，线 110
        bars.push(bar(111.0)); // 破 110 ⇒ 否定
        bars.push(bar(111.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_negate_closes_by_ladder[3], 1);
        // capital 26_000 在 111 买回 234.2 股，缩水 δ = 250 − 26000/111。
        let expect_back = 26_000.0 / 111.0;
        assert!((r.nrf_shrink_units - (250.0 - expect_back)).abs() < 1e-9);
        let root = r.trades.iter().find(|t| t.exit_reason == "eod").unwrap();
        assert!((root.shares - (750.0 + expect_back)).abs() < 1e-9, "根 = 750 + 缩水回补");
        // NAV = (750 + 234.23)×111 = 109_250（守恒：83250 + 26000）。
        assert!((r.final_nav - (750.0 * 111.0 + 26_000.0)).abs() < 1e-6);
    }

    #[test]
    fn liquidation_at_top_clear_located_level() {
        // v5 修复A：清仓参照系 = 当前 located 活跃最高级别 top*（=4）。
        // confirmed Sell1@4（背驰词汇）∧ located_sell[4] 在场（nf_sell[4]
        // 已触发未被破极值）⇒ top*=4 ∧ sell1[4] ⇒ 清仓（全链解栈）；
        // 子空随级联结算。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // nf@4 ⇒ located
        bars.push(sell1pt(bar(103.0), 4)); // top* 层 confirmed 背驰
        bars.push(bar(103.0));
        let r = run(bars);
        let root = r.trades.iter().find(|t| t.ladder == 4).unwrap();
        assert_eq!(root.exit_reason, "sellpt");
        assert_eq!(r.n_nrf_cascade_closes_by_ladder[3], 1, "子随级联结算");
        assert!(r.nrf_depth_bars[0] > 0, "清仓后回 Idle（真正的空仓 gap）");
        // NAV = 子回补@103（250 股，capital 26000 剩 250×1 = 250 利润）
        //     + 根 750+250 = 1000 股 ×103 = 103_000 + 250。
        assert!((r.final_nav - 103_250.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn liquidation_fires_below_ratchet_top_at_located_level() {
        // v5 修复A 的核心解耦：棘轮 top=4（warmup34 喂满层 3/4），但
        // located 只到层 3（证据@2 触发 nf_sell[3]）。v4 会检查 sell1@top=4
        // 而 located_sell[4]=None ⇒ 永不清仓；v5 检查 top*=3（最高 located）
        // ∧ sell1[3] ⇒ 清仓。这是「清仓频率随结构涌现」的最小复现：
        // 当前完美级别（3）低于历史棘轮（4）时，v5 仍按当前级别清仓。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根@4（棘轮 top=4）
        bars.push(with_ev(bar(105.0), 3, ev_full(BspClass::Sell1, false, 110.0, None))); // 武装 nest_sell[3]
        bars.push(with_ev(bar(104.0), 2, ev_full(BspClass::Sell1, true, 0.0, None))); // 证据@2 ⇒ nf_sell[3] ⇒ located[3]
        bars.push(sell1pt(bar(103.0), 3)); // 层 3 confirmed 背驰（< 棘轮 top=4）
        bars.push(bar(103.0));
        let r = run(bars);
        assert!(
            r.trades.iter().any(|t| t.exit_reason == "sellpt"),
            "v5：top*=3（located 最高层）∧ sell1[3] ⇒ 清仓（v4 因 located[4]=None 不清）"
        );
        assert!(r.nrf_depth_bars[0] > 0, "清仓后回 Idle");
    }

    #[test]
    fn non_divergence_sells_never_liquidate() {
        // §6/§9：清仓词汇 = Sell1（背驰）∧ located 合取——非 top 层
        // confirmed 卖、top 层无定位记忆的 confirmed 卖（sell_any）都
        // 不清仓；后者走 E 降成本 spawn（confirmed 出生无否定线）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(105.0), 3)); // 非 top 层 confirmed 卖：无动作
        bars.push(sellpt(bar(105.0), 4)); // top 层 confirmed 卖（非背驰/未定位）⇒ spawn
        bars.push(bar(105.0));
        let r = run(bars);
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "永不清仓");
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "confirmed 卖走 E 降成本");
        let root = r.trades.iter().find(|t| t.exit_reason == "eod").unwrap();
        assert_eq!(root.ladder, 4);
        assert!((r.final_nav - 105_000.0).abs() < 1e-6);
    }

    #[test]
    fn grandchild_long_from_short_child_capital() {
        // §5 递归嵌套：子空@3 在手 250，nest 定位买@3（candidate@3 ×
        // 证据@2）⇒ 释放 m2 给孙多@2（物理 = capital 买入），三层守恒。
        let mut bars = warmup34();
        warmup2(&mut bars); // θ₂=0.5% ⇒ θ_total=4.5%
        bars.push(buypt(bar(100.0), 4)); // 1000 股
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 子空@3
        bars.push(with_ev(bar(98.0), 3, ev_full(BspClass::Buy1, false, 97.0, None))); // 买窗@3
        bars.push(with_ev(bar(99.0), 2, ev_full(BspClass::Buy1, true, 0.0, None))); // 证据@2 ⇒ 孙多@2
        bars.push(bar(99.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[2], 1, "孙多@2 诞生（深度 3 链）");
        assert!(r.nrf_depth_bars[3] > 0, "三层并存");
        let gc = r.trades.iter().find(|t| t.ladder == 2).unwrap();
        assert_eq!(gc.polarity, Polarity::Long);
        // m（@4 spawn）= 1000×1/4.5 = 222.2…；m2 = 222.2×0.5/4.5 = 24.69…
        let m = 1000.0 * 1.0 / 4.5;
        let m2 = m * 0.5 / 4.5;
        assert!((gc.shares - m2).abs() < 1e-9, "孙配额 m2={m2} got={}", gc.shares);
    }

    #[test]
    fn earning_adds_units_at_buy_point_after_pool_zero() {
        // §7 earning：成本池 ≤ 0 后回补纯利润在买点买入 Δ，N 重定基。
        // 构造小成本池：入场后先验证常规回补不增仓（pool 远大于利润），
        // 再以多轮高利润回补磨穿池（θ₃ 配额 250 股 × 8 元/轮 = 2000/轮，
        // 池 100_000 需 50 轮——直接验证 50+1 轮后增仓发生）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // pool = 100_000
        for _ in 0..51 {
            // 每轮：nest 卖@4（候选 110）→ 证据@3 spawn 子空@104 → confirmed
            // 买@3 回补@96：利润 250×8 = 2000。
            let mut b = with_ev(bar(104.0), 4, ev_full(BspClass::Sell1, false, 110.0, None));
            b = with_ev(b, 3, ev_full(BspClass::Sell1, true, 0.0, None));
            bars.push(b);
            bars.push(buypt(bar(96.0), 3));
        }
        bars.push(bar(96.0));
        let r = run(bars);
        assert!(
            r.nrf_earning_units > 0.0,
            "池磨穿后 earning 增仓应发生：units={}",
            r.nrf_earning_units
        );
        assert!(r.n_nrf_earning_adds_by_ladder[4] > 0, "增仓发生在根层（买点时机）");
        // 守恒：引擎内部 §8.1 检查未 Err 即重定基一致。
        assert!(r.final_nav > 100_000.0, "50 轮短差利润沉淀 NAV 上升");
    }

    #[test]
    fn fund_conservation_through_recursion() {
        // 资金守恒：递归三层流转不增不减——NAV 重建 = 现金流闭合。
        let mut bars = warmup34();
        warmup2(&mut bars);
        bars.push(buypt(bar(100.0), 4)); // 1000 股 @100
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 子空 m@104
        bars.push(with_ev(bar(98.0), 3, ev_full(BspClass::Buy1, false, 97.0, None)));
        bars.push(with_ev(bar(97.0), 2, ev_full(BspClass::Buy1, true, 0.0, None))); // 孙多 m2@97
        bars.push(sellpt(bar(99.0), 2)); // 孙走势完美 ⇒ 平孙回流子 capital
        bars.push(buypt(bar(95.0), 3)); // 子走势完美 ⇒ 回补，父回满
        bars.push(bar(95.0));
        let r = run(bars);
        let m = 1000.0 * 1.0 / 4.5;
        let m2 = m * 0.5 / 4.5;
        // 现金流闭合：根 1000@100；卖 m@104（+104m）；买 m2@97（−97m2）；
        // 卖 m2@99（+99m2）；买回 m@95（−95m）。
        // eod NAV = (1000−m+m)×95… 根回满 1000 股×95 + 利润现金。
        let profit = m * (104.0 - 95.0) + m2 * (99.0 - 97.0);
        let expect = 1000.0 * 95.0 + profit;
        assert!((r.final_nav - expect).abs() < 1e-6, "final={} expect={expect}", r.final_nav);
        let rec_rows: Vec<_> =
            r.trades.iter().filter(|t| t.exit_reason == "recover").collect();
        assert_eq!(rec_rows.len(), 2, "孙、子各一次走势完美回补");
    }
}
