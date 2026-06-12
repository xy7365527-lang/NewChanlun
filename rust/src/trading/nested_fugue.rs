//! nested_fugue — 嵌套递归赋格 v2（NRF final；mode = "nrf"）。
//!
//! 核心命题（2026-06-12 编排者，三段补充裁决后的完整形式）：
//! **逐仓独立头寸和递归区间套是同一件事，且同一笔物理交易在递归的不同级别
//! 有不同的会计身份。**
//!
//! 1. 父 voice 在卖点卖出部分多头（降成本短差卖出）——这同一笔卖出在子
//!    级别 = 开空头寸。物理一笔交易，会计双层记账。
//! 2. 父 voice 回补（短差闭合）——同一笔买入在子级别 = 平空 + 开多
//!    （翻转）。子级别空头 P&L ≡ 父级别降成本金额（同一数字两个身份，
//!    存一次、两个视图读——不双写避免 NAV 重复计入）。
//! 3. 资金守恒：父卖出 N 股释放的现金 = 子开空 N 股的资本。零配额机制、
//!    零额外保证金（53课留白 = 不需要）；总资金 = 最高级别的初始投入，
//!    在递归各层间流转但不增不减。
//! 4. 区间套 = voice spawn 的时序机制：父层反向 candidate 武装窗口 ×
//!    次级别第一证据触发 ⇒ spawn（027课程序定理）。父级别的回补时点
//!    必然是子级别走势完美（区间套对齐保证）。
//! 5. 平多 ≠ 开空：高级别 voice 不平仓（走势没完美），次级别 voice 开空
//!    ——两个独立级别的独立会计事件。高级别走势真正完美（confirmed）时，
//!    高级别 voice 平仓**且立即翻转**（v3，见下）。
//! 6. 每层做完全相同的事（正则化/递归自相似，第20环）。
//! 7. 零概念 flag——唯一经验参数 = a0；floor_ladder 为结构常数。
//!
//! ## v3 增补（2026-06-12 编排者任务：补完两个概念链缺口）
//!
//! **根翻转（第23环字面：出场=翻转=新建仓）**——走势完美 = 旧势耗尽 +
//! 新势开始，新势方向与旧势相反，voice 跟随新势。根 confirmed 反向点 ⇒
//! 全链清算 ⇒ 立即按新方向在当前最高 θ 涌现层满仓重建根（重建即 F 式
//! 入场——保留 v2 的根爬升机制，alpha 集中在根爬升长持腿）。根空头 =
//! **真实 1x 逐仓空头**（保证金 = 锁定现金 = units×basis，损失有界于
//! 保证金——A 强平在空根相位是物理事件；NAV 参与下跌方向）。根空头与
//! 子 voice 机制完全一样（正则化）：可 spawn 子多 voice（物理 = 回补
//! 平空，同一笔交易双层身份的镜像），子多再 spawn 孙空（物理 = 重开
//! 空头）。链的物理暴露由"尾方向 × 根方向"决定：同向 ⇒ 满暴露（根向
//! ±1），异向 ⇒ flat（0）——长根链 {+1,0} 与空根链 {−1,0} 完全镜像。
//!
//! **递归区间套（第14环完整实现）**——candidate@k 的触发证据从 k−1 起
//! 逐层下探直到 a0（bi 层方向翻转沿）：任何一层出现同侧直接证据即触发
//! （终止条件仅 a0 ∨ 找到证据——任务裁决）。最低层证据最先出现 ⇒ 触发
//! 时点尽可能早，覆盖率 =（窗口触发 vs 打破竞速）的胜率随词汇密度上升。
//!
//! ## 统一 FSM（链式塔）
//!
//! 活跃 voice 构成自顶向下的连续链 [top..j]（spawn 逐级 k→k−1）。物理
//! 状态唯一由链尾方向 × 根方向决定（v3）：同向 ⇔ 满暴露（长根 = 持股
//! units；空根 = 真实 1x 空头 units）；异向 ⇔ 持现金 locked（长根 =
//! 卖出所得 = 子空资本；空根 = 回补残值 = 子多资本）。上层 voice 全部
//! 处于"短差在外"等待——其状态由子方向编码（子反向 ⇔ 父短差在外；
//! 子翻回 ⇔ 父短差闭合），不需要独立 sub_out 标记。
//!
//! 每 bar 优先序（同 bar 单事件）：
//! A. 强平兜底（尾 Short 权益 ≤ 0 ⇔ c ≥ 2×basis，1x 逐仓解析强平）
//! B. 否定扫描（根→尾第一个出生相 voice 破 027:25 极值线 ⇒ 该层及子孙
//!    结算，物理恢复其父方向；根破线 ⇒ 全平）
//! C. 根走势完美（confirmed 反向点@root ⇒ 全链级联平仓 + 根按新势方向
//!    立即满仓重建——v3 根翻转，第23环字面）
//! D. 尾翻转（尾 confirmed 反向 ⇒ 物理反向交易，尾平旧开新；同一笔交易
//!    在父层 = 短差腿闭/开，38课两相循环）
//! E. spawn（尾反向 candidate 窗 × 次级别证据 ⇒ 物理交易 + push 子 voice；
//!    终止 = floor（77-78课笔=a0）∨ 35课成本门）
//! F. 根入场（链空 ⇒ 最高 θ 涌现层买证据满仓开多）
//!
//! 中间层不消费 confirmed 词汇（53课显微镜原则：委托给子孙后该层走势不在
//! 观察中）；其退出途径 = 否定线（仅出生相）∨ 级联。
//!
//! 空头会计 [镜像推导]（38:36 镜像止于判断-动作序列；84:316 唯一直接判决
//! 为负）。在册路径零接触（独立 PolarityMode 入口；nrf 计数器其余模式恒零）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// 区间套窗口（fusion::NestWin 同语义——candidate 武装/刷新极值、confirmed
/// 同侧清窗、破极值否定）。
#[derive(Debug, Clone, Copy)]
struct Win {
    extreme: f64,
}

/// 链上一个 voice（一个级别的会计视图）。物理载体由链尾方向决定，
/// 各层只记自己相位的开仓价与累计已实现 P&L。
#[derive(Debug, Clone, Copy)]
struct Voice {
    ladder: usize,
    dir: Polarity,
    /// 当前相位开仓价（翻转时重置）。
    basis: f64,
    /// 当前相位开仓 bar。
    entry_bar: i64,
    /// 出生相否定线（spawn 时 candidate 极值，027:25）；翻转后退役 None。
    negate_line: Option<f64>,
    /// 本相位股数（partial 回补缩水后全链共享缩水，相位开启时快照）。
    units: f64,
}

/// 物理账（单一真值；各层 voice 是它的级别分解视图）。三种载体：
/// 持股（暴露 +1）、锁定现金（暴露 0——已卖出/已回补弹药）、真实空头
/// （暴露 −1，空根链相位；1x 逐仓，保证金 = locked = units×basis，
/// 损失有界于保证金）。暴露 = f(尾方向, 根方向)：同向满暴露、异向 flat。
struct Phys {
    /// 当前物理持股（长根链满暴露载体）。
    shares: f64,
    /// 真实空头单位数（空根链满暴露载体；v3 根翻空 [镜像推导]）。
    short_units: f64,
    /// 真实空头开仓价。
    short_basis: f64,
    /// 链锁定现金（flat 相位 = 弹药；空头相位 = 1x 逐仓保证金）。
    locked: f64,
    /// 自由现金（实现利润沉淀 + 入场剩余；根入场时全额投入）。
    free: f64,
}

impl Phys {
    fn nav(&self, c: f64) -> f64 {
        self.free + self.locked + self.shares * c
            + self.short_units * (self.short_basis - c)
    }
}

/// 次级别证据（区间套触发词汇；fusion::nest_sub_evidence 逐字同源）。
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

/// 递归区间套证据（v3 第14环完整形式）：从 j = k−1 起逐层下探直到 a0
/// （j = FIRST_BSP_LADDER−1 = bi 层，词汇 = 方向翻转沿）。任何一层出现
/// 同侧直接证据即触发——终止条件仅 a0 ∨ 找到证据（任务裁决"递归到底"；
/// 武装窗口链门控读法在真实数据上域空：k−1 candidate 在 k candidate
/// 出现前几乎总被自身 confirmed 清窗，深触发恒零）。返回证据层（取最
/// 高有证据层；None = 全链词汇穷尽）。
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

/// 结算一个 voice 的当前相位（trade 行 + 计数）。返回该相位实现 P&L。
/// 现金流不在此处理（物理账由调用方按链状态统一变更）。
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

/// 物理对齐到 (新尾方向 × 根方向) 决定的暴露。同向 ⇒ 满暴露（长根：
/// 买入持股；空根：开 1x 逐仓空头——保证金 = min(want, locked/c)×c），
/// 异向 ⇒ flat（卖出全部 / 回补全部，所得锁定为弹药）。幂等。返回新
/// 相位载体单位数（满暴露 = 实际成交量，资金守恒按可用缩量——缩水即
/// 亏损的物理形式；flat = want，视图单位）。
///
/// M = N（026:34"绝对不加仓"）：满暴露恰 want 单位，剩余现金 = 实现
/// 利润沉淀为自由现金（降成本的物理形式——不复投单位数）。
fn phys_align(phys: &mut Phys, tail_dir: Polarity, root_dir: Polarity, want: f64, c: f64) -> f64 {
    if tail_dir != root_dir {
        // flat：卖出全部持股 ∨ 回补全部空头。空头残值有界于 0（1x 逐仓
        // 隔离保证：超出保证金的滑出由强平边界承接，A 规则先于此处触发）。
        if phys.shares > 0.0 {
            phys.locked += phys.shares * c;
            phys.shares = 0.0;
        }
        if phys.short_units > 0.0 {
            phys.locked =
                (phys.locked + phys.short_units * (phys.short_basis - c)).max(0.0);
            phys.short_units = 0.0;
            phys.short_basis = 0.0;
        }
        return want;
    }
    match root_dir {
        Polarity::Long => {
            if phys.shares > 0.0 {
                return phys.shares; // 已对齐（幂等）
            }
            debug_assert!(phys.short_units == 0.0, "长根链无真实空头载体");
            let m = want.min(phys.locked / c);
            phys.free += phys.locked - m * c;
            phys.locked = 0.0;
            phys.shares = m;
            m
        }
        Polarity::Short => {
            if phys.short_units > 0.0 {
                return phys.short_units; // 已对齐（幂等）
            }
            debug_assert!(phys.shares == 0.0, "空根链无持股载体");
            let m = want.min(phys.locked / c);
            phys.free += phys.locked - m * c;
            phys.locked = m * c;
            phys.short_units = m;
            phys.short_basis = c;
            m
        }
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
    let mut phys = Phys {
        shares: 0.0,
        short_units: 0.0,
        short_basis: 0.0,
        locked: 0.0,
        free: INITIAL_CAPITAL,
    };
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

        // ── 区间套窗口维护（v1 同序：① 打破否定 → ② candidate 武装/
        //    confirmed 清窗 → ③ 次级别证据触发；nf_* 携带触发时极值）──
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
            // v3：单层检查 → 递归下探至 a0（第14环）。证据层 < k−1 ⇒
            // 深触发（v2 在该 bar 不触发的增量时点）。
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

        // ── A. 强平兜底（尾 Short 权益 = locked + units×(basis − c)
        //    = units×(2·basis − c) ≤ 0 ⇔ c ≥ 2×basis；1x 逐仓解析）──
        let mut acted = false;
        if let Some(tail) = chain.last().copied() {
            if tail.dir == Polarity::Short && phys.locked + tail.units * (tail.basis - c) <= 0.0 {
                // 强平价 2×basis 记账（残值恰零：locked 全耗、股数归零）。
                let g = chain.len() - 1;
                liquidate_from(
                    g, bar, 2.0 * tail.basis, c, &mut chain, &mut phys, &mut res, "liq",
                );
                res.n_short_liquidations_by_ladder[tail.ladder] += 1;
                acted = true;
            }
        }

        // ── B. 否定扫描（根→尾；第一个出生相 voice 破 027:25 极值线 ⇒
        //    该层及子孙结算，物理恢复其父方向；根破线 ⇒ 全平）──
        if !acted {
            let broke = chain.iter().position(|v| {
                v.negate_line.is_some_and(|line| match v.dir {
                    Polarity::Short => c > line,
                    Polarity::Long => c < line,
                })
            });
            if let Some(g) = broke {
                let lad = chain[g].ladder;
                liquidate_from(g, bar, c, c, &mut chain, &mut phys, &mut res, "negate");
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted = true;
            }
        }

        // ── C. 根走势完美（confirmed 反向点@root ⇒ 全链级联平仓 + 根
        //    翻转重建——v3 第23环字面：出场=翻转=新建仓。走势完美 =
        //    旧势耗尽 + 新势开始，新势方向与旧势相反；重建 = F 式满仓
        //    入场（在当前最高 θ 涌现层——保留根爬升机制），出生相否定
        //    线不存在（confirmed 入场，与 D 翻转同律）──
        if !acted {
            if let Some(root) = chain.first().copied() {
                let perfected = match root.dir {
                    Polarity::Long => sig.sell_any.get(root.ladder),
                    Polarity::Short => sig.buy_any.get(root.ladder),
                };
                if perfected {
                    let reason = match root.dir {
                        Polarity::Long => "sellpt",
                        Polarity::Short => "buypt",
                    };
                    liquidate_from(0, bar, c, c, &mut chain, &mut phys, &mut res, reason);
                    // 翻转重建：全部资金按新势方向开仓。根空头物理形态 =
                    // 现金锁定等待回补（locked = units×basis，零杠杆）。
                    let new_dir = match root.dir {
                        Polarity::Long => Polarity::Short,
                        Polarity::Short => Polarity::Long,
                    };
                    let top = (floor_ladder..MAX_LADDER)
                        .rev()
                        .find(|&k| {
                            depth_ref.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS).is_some()
                        })
                        .unwrap_or(root.ladder);
                    let want = phys.free / c;
                    if want > 0.0 && want.is_finite() {
                        phys.locked = phys.free;
                        phys.free = 0.0;
                        let units = phys_align(&mut phys, new_dir, new_dir, want, c);
                        chain.push(Voice {
                            ladder: top,
                            dir: new_dir,
                            basis: c,
                            entry_bar: bar,
                            negate_line: None,
                            units,
                        });
                        res.n_nrf_root_flips_by_ladder[top] += 1;
                        res.n_entries_by_ladder[top] += 1;
                    }
                    acted = true;
                }
            }
        }

        // ── D. 尾翻转（尾 confirmed 反向 ⇒ 物理反向交易 + 尾平旧开新；
        //    同一笔交易在父层 = 短差腿闭/开——双层记账的物理一笔）。
        //    根是尾（链长 1）时不翻转——根的反向词汇已在 C 消费（平仓）──
        if !acted && chain.len() > 1 {
            let tail = *chain.last().expect("len>1");
            let perfected = match tail.dir {
                Polarity::Long => sig.sell_any.get(tail.ladder),
                Polarity::Short => sig.buy_any.get(tail.ladder),
            };
            if perfected {
                let j = chain.len() - 1;
                let pnl = settle_phase(&chain[j], bar, c, "flip", &mut res);
                let _ = pnl; // 子层 pnl ≡ 父层降成本（同一数字，视图导出不双写）
                let new_dir = match tail.dir {
                    Polarity::Long => Polarity::Short,
                    Polarity::Short => Polarity::Long,
                };
                let root_dir = chain[0].dir;
                let new_units = phys_align(&mut phys, new_dir, root_dir, tail.units, c);
                chain[j] = Voice {
                    ladder: tail.ladder,
                    dir: new_dir,
                    basis: c,
                    entry_bar: bar,
                    negate_line: None, // 翻转后出生相否定线退役
                    units: new_units,
                };
                res.n_nrf_flips_by_ladder[tail.ladder] += 1;
                res.n_entries_by_ladder[tail.ladder] += 1;
                acted = true;
            }
        }

        // ── E. spawn（区间套递归 = voice 诞生）：尾反向 candidate 窗 ×
        //    次级别证据 ⇒ 物理交易 + push 子 voice。父卖出释放的现金 =
        //    子空头资本（资金守恒，零配额）。终止 = floor ∨ 35课成本门 ──
        if !acted {
            if let Some(tail) = chain.last().copied() {
                let (fired, child_dir) = match tail.dir {
                    Polarity::Long => (nf_sell[tail.ladder], Polarity::Short),
                    Polarity::Short => (nf_buy[tail.ladder], Polarity::Long),
                };
                if let Some(extreme) = fired {
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
                                // 物理 = 同一笔交易双层身份：长根链子空 =
                                // 卖出全部（所得锁定 = 子空资本）/子多 =
                                // 买回；空根链镜像（子多 = 回补平空/孙空
                                // = 重开空头）。
                                let root_dir = chain[0].dir;
                                let units =
                                    phys_align(&mut phys, child_dir, root_dir, tail.units, c);
                                chain.push(Voice {
                                    ladder: sub,
                                    dir: child_dir,
                                    basis: c,
                                    entry_bar: bar,
                                    negate_line: Some(extreme),
                                    units,
                                });
                                res.n_nrf_spawns_by_ladder[sub] += 1;
                                res.n_entries_by_ladder[sub] += 1;
                                acted = true;
                            }
                        }
                    }
                }
            }
        }

        // ── F. 根入场（链空）：最高 θ 涌现层 top 的买证据 ⇒ 满仓开多
        //    （总资金 = 最高级别的初始投入，消息3）──
        if !acted && chain.is_empty() {
            let top = (floor_ladder..MAX_LADDER).rev().find(|&k| {
                depth_ref.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS).is_some()
            });
            if let Some(top) = top {
                let nf = nf_buy[top];
                if sig.buy_any.get(top) || nf.is_some() {
                    let units = phys.free / c;
                    if units > 0.0 && units.is_finite() {
                        phys.free = 0.0;
                        phys.shares = units;
                        let line = if sig.buy_any.get(top) { None } else { nf };
                        chain.push(Voice {
                            ladder: top,
                            dir: Polarity::Long,
                            basis: c,
                            entry_bar: bar,
                            negate_line: line,
                            units,
                        });
                        res.n_nrf_root_entries_by_ladder[top] += 1;
                        res.n_entries_by_ladder[top] += 1;
                    }
                }
            }
        }

        // 观测：链深度直方图 + 物理暴露（双向）+ 各层视图持有 bar 计数。
        res.nrf_depth_bars[chain.len().min(MAX_LADDER - 1)] += 1;
        if phys.shares > 0.0 {
            res.nrf_phys_long_bars += 1;
        }
        if phys.short_units > 0.0 {
            res.nrf_phys_short_bars += 1;
        }
        for v in &chain {
            res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, phys.nav(c)));
        }
    }

    // eod：全链结算。
    if !chain.is_empty() {
        let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
        let last_bar = (n as i64) - 1;
        liquidate_from(0, last_bar, c_last, c_last, &mut chain, &mut phys, &mut res, "eod");
    }
    res.final_nav = phys.nav(tape.bars.last().map_or(0.0, |b| b.close));
    Ok(res)
}

/// 从链位置 g 起结算到尾（g 层 trade 行用 g_exit_price——强平时为解析价
/// 2×basis；g 以深子孙用市价 c，reason="cascade"），物理恢复 g 的父方向：
/// g=0 ⇒ 全平回现金（Idle）；g>0 ⇒ 父 Long 则买回（locked 弹药，缩量即
/// 亏损的物理形式）/父 Short 则卖出（所得重新锁定）。
#[allow(clippy::too_many_arguments)]
fn liquidate_from(
    g: usize,
    bar: i64,
    g_exit_price: f64,
    c: f64,
    chain: &mut Vec<Voice>,
    phys: &mut Phys,
    res: &mut PositionalResult,
    reason: &'static str,
) {
    debug_assert!(g < chain.len(), "liquidate_from 前提：g 在链上");
    // 结算 trade 行（先深后浅；g 层用调用方价格与归因，子孙 cascade）。
    for j in (g..chain.len()).rev() {
        let (px, why) = if j == g { (g_exit_price, reason) } else { (c, "cascade") };
        settle_phase(&chain[j], bar, px, why, res);
        if j > g {
            res.n_nrf_cascade_closes_by_ladder[chain[j].ladder] += 1;
        }
    }
    chain.truncate(g);
    // 物理恢复到新尾（= g 的父）的暴露；链空 ⇒ 全部变现金（持股变现 +
    // 空头回补，残值有界于 0——1x 逐仓隔离保证）。
    match chain.last().copied() {
        None => {
            if phys.shares > 0.0 {
                phys.free += phys.shares * c;
                phys.shares = 0.0;
            }
            if phys.short_units > 0.0 {
                phys.locked =
                    (phys.locked + phys.short_units * (phys.short_basis - c)).max(0.0);
                phys.short_units = 0.0;
                phys.short_basis = 0.0;
            }
            phys.free += phys.locked;
            phys.locked = 0.0;
        }
        Some(parent) => {
            let root_dir = chain[0].dir;
            let m = phys_align(phys, parent.dir, root_dir, parent.units, c);
            let last = chain.len() - 1;
            chain[last].units = m; // partial 缩水传播（单位回不来 = 亏损）
        }
    }
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
    /// c=100 口径——两层均过 35课成本门）。
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
        // eod NAV = 1000 股 × 101 = 101_000。
        assert!((r.final_nav - 101_000.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_is_single_physical_trade_dual_ledger() {
        // spawn = 物理一笔卖出，双层记账：父层短差在外（不出 trade 行），
        // 子层开空（资本 = 卖出所得）。NAV 在 spawn bar 前后连续。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根满仓@4：1000 股
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(bar(104.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "子空头@3 诞生");
        // spawn 不结算父层相位——此刻 trade 行为空（双层身份存一次）。
        // eod 全链结算：根 Long@100 + 子 Short@104，物理现金 = 1000×104。
        // eod 时子先结（104→104 平，pnl 0），根结算用市价 104（物理上股
        // 已卖出锁定 104000——根的"持股视图"以回补价结算）。
        let short = r.trades.iter().find(|t| t.polarity == Polarity::Short).unwrap();
        assert_eq!((short.ladder, short.exit_reason), (3, "cascade"));
        assert!((short.entry_price - 104.0).abs() < 1e-12);
        // NAV 守恒：卖出价 104 × 1000 股 = 104_000。
        assert!((r.final_nav - 104_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn child_perfection_flips_and_parent_cost_basis_drops() {
        // 子级别走势完美（confirmed buy@3）⇒ 同一笔买入：子平空+开多
        //（flip），父回补完成（降成本 = 子空头 pnl，同一数字）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根 1000 股 @100
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // spawn 空@104
        bars.push(buypt(bar(95.0), 3)); // 子走势完美 ⇒ 翻多@95
        bars.push(bar(95.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_flips_by_ladder[3], 1, "子翻转一次");
        let flip = r.trades.iter().find(|t| t.exit_reason == "flip").unwrap();
        assert_eq!((flip.ladder, flip.polarity), (3, Polarity::Short));
        // 子空头 pnl = (104−95)×1000 = +9000 = 父降成本金额（同一数字）。
        let pnl = flip.shares * (flip.entry_price - flip.exit_price);
        assert!((pnl - 9000.0).abs() < 1e-6);
        // M=N：买回恰 1000 股，利润 9000 沉淀自由现金。
        // eod NAV = 9000 + 1000×95 = 104_000（与卖出时点价值守恒）。
        assert!((r.final_nav - 104_000.0).abs() < 1e-6, "final={}", r.final_nav);
        // 翻多后的子多头与根多头两个视图并存，eod 各出一条 Long 行。
        let longs: Vec<_> = r.trades.iter().filter(|t| t.polarity == Polarity::Long).collect();
        assert_eq!(longs.len(), 2, "根视图 + 子翻多视图");
    }

    #[test]
    fn negation_kills_child_and_restores_parent_partial() {
        // 子空头期间价格破 spawn 极值 ⇒ 子死亡，父回补（追价缩水 partial
        // = 亏损的物理形式：股数回不来）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 1000 股 @100
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // 空@104，线 110
        bars.push(bar(111.0)); // 破 110 ⇒ 否定
        bars.push(bar(111.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_negate_closes_by_ladder[3], 1);
        let neg = r.trades.iter().find(|t| t.exit_reason == "negate").unwrap();
        assert_eq!(neg.polarity, Polarity::Short);
        // 资本 104_000 在 111 只能买回 936.9 股——父 units 缩水。
        let root = r.trades.iter().find(|t| t.exit_reason == "eod").unwrap();
        assert!((root.shares - 104_000.0 / 111.0).abs() < 1e-6, "父收回缩水股数");
        // NAV 守恒：104_000（现金价值在 111 价位全部转回股票）。
        assert!((r.final_nav - 104_000.0).abs() < 1e-6);
    }

    #[test]
    fn root_perfection_cascades_whole_chain_and_flips() {
        // 根 confirmed 卖 ⇒ 全链级联平仓 + 根立即翻空（v3 第23环：
        // 出场=翻转=新建仓，不再回 Idle 等待）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(sellpt(bar(103.0), 4)); // 根走势完美
        bars.push(bar(103.0));
        let r = run(bars);
        let root = r
            .trades
            .iter()
            .find(|t| t.ladder == 4 && t.polarity == Polarity::Long)
            .unwrap();
        assert_eq!(root.exit_reason, "sellpt");
        assert_eq!(r.n_nrf_cascade_closes_by_ladder[3], 1, "子随级联结算");
        assert_eq!(r.n_nrf_root_flips_by_ladder[4], 1, "根翻空重建");
        let s = r
            .trades
            .iter()
            .find(|t| t.polarity == Polarity::Short && t.exit_reason == "eod")
            .unwrap();
        assert_eq!(s.ladder, 4, "翻空根 = 最高 θ 涌现层");
        assert!((s.entry_price - 103.0).abs() < 1e-12);
        // NAV 守恒：104 价位卖出锁定的 104_000（空头相位 NAV 持平）。
        assert!((r.final_nav - 104_000.0).abs() < 1e-6);
    }

    #[test]
    fn root_flip_full_cycle_real_short_pnl() {
        // 第23环全循环：根多 → confirmed 卖翻空（真实 1x 逐仓空头）→
        // confirmed 买翻多。空头 P&L 真实计入 NAV（下跌方向暴露 −1）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 1000 股 @100
        bars.push(sellpt(bar(110.0), 4)); // 翻空：保证金 110_000，units=1000
        bars.push(bar(100.0));
        bars.push(buypt(bar(88.0), 4)); // 翻多：回补 +22_000 → 132_000/88 = 1500 股
        bars.push(bar(88.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_root_flips_by_ladder[4], 2, "多→空→多两次翻转");
        let short = r.trades.iter().find(|t| t.polarity == Polarity::Short).unwrap();
        assert_eq!(short.exit_reason, "buypt");
        // 空头 pnl = 1000×(110−88) = +22_000 物理计入：回补后总现金
        // 132_000，翻多买入 1500 股（BH 同期 1000 股缩值到 88_000）。
        let last_long =
            r.trades.iter().filter(|t| t.polarity == Polarity::Long).last().unwrap();
        assert!((last_long.shares - 132_000.0 / 88.0).abs() < 1e-9, "回补股数 1500");
        assert!((r.final_nav - 132_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn recursive_nest_fires_through_armed_candidate_chain() {
        // 第14环：candidate@4 武装时 k−1 当 bar 无直接证据，但 3 层窗口
        // 仍武装（更早 bar 的 candidate）⇒ 下探到 2 层直接证据 ⇒ 深触发
        // spawn（v2 单层检查在此 bar 不触发——时间错位的证据链）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根 1000 股
        bars.push(with_ev(bar(106.0), 3, ev_full(BspClass::Sell1, false, 108.0, None)));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 2, ev_full(BspClass::Sell1, false, 0.0, None)));
        bars.push(bar(104.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_deep_fires_by_ladder[4], 1, "经 3 层 candidate 链下探到 2 层证据");
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "深触发 spawn 子空@3");
    }

    #[test]
    fn root_short_spawns_child_long_voice() {
        // 正则化：翻空后的根也是完整 voice——反向（买侧）candidate 武装
        // × 次级别买证据 ⇒ spawn 子多 voice（与多头根 spawn 子空同律）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(110.0), 4)); // 根翻空 @110
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Buy1, false, 103.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Buy1, true, 0.0, None)));
        bars.push(bar(104.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "根空头 spawn 子多@3");
        let child = r.trades.iter().find(|t| t.ladder == 3).unwrap();
        assert_eq!(child.polarity, Polarity::Long);
        // 子多 = 物理回补平空（同一笔交易双层身份的镜像；M=N units 不变）。
        assert!((child.shares - 1000.0).abs() < 1e-9);
        // 回补实现空头 pnl = 1000×(110−104) = +6_000 → NAV 116_000。
        assert!((r.final_nav - 116_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn fund_conservation_through_recursion() {
        // 资金守恒：递归各层流转不增不减——NAV 重建 = trade 现金流闭合。
        // 孙 spawn 需要 θ₂ 参照：warmup 补层 2 锚（θ₂=0.5% 过成本门）。
        let mut bars = warmup34();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 2, ev_full(BspClass::Sell1, false, 0.0, Some(500 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            bars.push(b);
        }
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(buypt(bar(95.0), 3)); // 子翻多（+9000 利润沉淀）
        bars.push(with_ev(bar(98.0), 3, ev_full(BspClass::Sell1, false, 99.0, None)));
        bars.push(with_ev(bar(97.0), 2, ev_full(BspClass::Sell1, true, 0.0, None))); // spawn 孙空@2
        bars.push(bar(96.0));
        let r = run(bars);
        assert_eq!(r.n_nrf_spawns_by_ladder[2], 1, "孙空诞生（深度 3 链）");
        assert!(r.nrf_depth_bars[3] > 0, "三层并存（L4多+L3多+L2空 视图）");
        // 物理唯一真值核对：1000 股 @100 入场 → 104 卖 → 95 买回（+9000）
        // → 97 卖（孙空）。eod 链上三层全结算，NAV = 9000 + 97_000 = 106_000。
        assert!((r.final_nav - 106_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }
}
