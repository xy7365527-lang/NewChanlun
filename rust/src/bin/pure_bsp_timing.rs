//! # `pure_bsp_timing` —— 纯买卖点离散择时隔离实验（goal #5 最根本问题）
//!
//! ## 问题（命题 A：π_Θ ⊋ 纯择时）
//! 生产引擎 π_Θ 是「结构覆盖净额投影」（每 bar 对 AncOK 活动集所有元素生成腿 + net_target
//! 净额化）。acceptance[2] 测它 = b1 否证（净敞口动但无 alpha）。但**纯买卖点离散择时从未被
//! 单独测试**。本 bin 隔离测它——回答缠论最本真的操作（买卖点择时）到底有没有 edge。
//!
//! ## 操作语义（与 π_Θ 严格不同）
//! - π_Θ 覆盖（已测 b1）：每 bar Σ_e Leg(e) over 整个 AncOK 活动集，净额化。
//! - **纯择时（本 bin）**：离散事件状态机——仓位∈{+1,0,-1}，买卖点事件驱动切换：
//!   第一类买点 buy1 触发开多 q=+1 → 持有 → 第一类卖点 sell1 触发平多 q=0（再反手开空可选）。
//!   不做 leg_target / net_target 覆盖，不每 bar 重算所有元素。
//!
//! ## 坐标系纪律（644 meta-rule：探针必走生产路径，无影子分叉）
//! - **买卖点序列**：走生产 `IncrementalClassifier::classify_at(i)`（因果塔，≤i 数据，无 look-ahead）+
//!   与 runner `newly_confirmed_step` **逐字相同**的 seen-set append-only diff（同一确认时点语义）。
//! - **NAV 口径**：复用 `metrics::compute`（Sharpe/strat_return）+ `metrics::significance`
//!   （theta_return_same_caliber/随机对照），与工位 L 测 π_Θ **完全同口径** ⟹ Sharpe 可比。
//!
//! ## 认识论等级
//! 真实数据（CL/BTC）回测 = **L2**（可否证）。判定：
//! - 纯择时 Sharpe>0 而 π_Θ≤0 ⟹ 坐实「π_Θ 覆盖语义抹平了买卖点择时 alpha」。
//! - 纯择时也≤0 ⟹ 缠论买卖点信号本身在 1min/CL/BTC 无预测力（更根本否证）。
//!
//! ## no-patch / 有效域诚实
//! 不改 π_Θ 生产逻辑（coverage.rs/runner.rs net_target 不动）。本 bin 是独立实验，复用
//! 生产 classifier 输出 + 生产 metrics 口径。仓位状态机是**新增**的离散择时操作语义，非 π_Θ 子集。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin pure_bsp_timing -- <SYMBOL> [START END]
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::backtest::metrics::{self, TradeRecord};
use newchan_rust::theta_v0::config::ThetaConfig;
use std::collections::HashSet;

/// 买卖点身份判别（与 runner.rs `bsp_bits_disc` 同口径——6 类 bit 打包，2买/3买 V 型共存视为不同身份）。
fn bsp_bits_disc(b: &newchan_rust::theta_v0::types::BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        eprintln!(
            "用法: {} <SYMBOL> [START_DATE END_DATE]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO",
            args.first().map(String::as_str).unwrap_or("pure_bsp_timing")
        );
        return std::process::ExitCode::from(2);
    }
    let config = ThetaConfig::default();
    let full = match load_by_symbol(&args[1], &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let dataset = if args.len() == 4 {
        full.slice_date_window(&args[2], &args[3])
    } else {
        full
    };
    if dataset.bars.is_empty() {
        eprintln!("空数据集——无法回测（inconclusive）");
        return std::process::ExitCode::FAILURE;
    }

    let bars = &dataset.bars;
    let n = bars.len();
    let prices: Vec<f64> = bars
        .iter()
        .map(|b| b.close as f64 * config.tick.tick_size)
        .collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let years = (n as f64 / (365.25 * 24.0 * 60.0)).max(1e-9);

    // ── 纯择时离散状态机：仓位∈{+1,0,-1}，买卖点确认驱动切换。 ──
    // 走生产因果塔 + 与 runner 同的 seen-set diff（644：无影子分叉）。
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    let mut position: i8 = 0; // +1 多 / 0 空仓 / -1 空
    let mut entry_bar: usize = 0;
    let qty = 1.0; // 离散单位仓位（定值，纯择时不做 sizing）
    let mut trades: Vec<TradeRecord> = Vec::new();

    // 平/开一笔（close 成交口径，与 metrics::significance trade_abs_pnl 一致）。
    let mut close_trade = |trades: &mut Vec<TradeRecord>, entry: usize, exit: usize, long: bool| {
        if exit > entry {
            trades.push(TradeRecord {
                entry_bar: entry,
                exit_bar: exit,
                hold_bars: exit - entry,
                qty,
                long,
                forced_close: false,
            });
        }
    };

    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        // 本 bar 新确认的买卖点（append-only diff，与 runner newly_confirmed_step 同语义）。
        // ★任意买点 bit（buy1∨buy2∨buy3）→ 开多；任意卖点 bit（sell1∨sell2∨sell3）→ 平多/开空。
        // 不限第一类：纯择时测的是「缠论买卖点信号整体」的择时 edge（buy1 在 1min 短窗稀疏，
        // 限 1 类 ⟹ 零交易 ⟹ inconclusive 而非否证；no-patch：用 classifier 实际产出的全部买卖点）。
        let mut new_buy = false;
        let mut new_sell = false;
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in &ls.bsp {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))) {
                    let b = &p.bits;
                    if b.buy1 || b.buy2 || b.buy3 {
                        new_buy = true;
                    }
                    if b.sell1 || b.sell2 || b.sell3 {
                        new_sell = true;
                    }
                }
            }
        }
        let new_buy1 = new_buy;
        let new_sell1 = new_sell;

        // 状态机迁移（事件驱动，buy1 开多 / sell1 平多并反手开空）。
        // ponytail: 只用第一类买卖点（最本真的趋势底/顶背驰择时）。2/3 类可扩展，YAGNI 先测 1 类。
        match position {
            0 => {
                if new_buy1 {
                    position = 1;
                    entry_bar = i;
                } else if new_sell1 {
                    position = -1;
                    entry_bar = i;
                }
            }
            1 => {
                if new_sell1 {
                    close_trade(&mut trades, entry_bar, i, true); // 平多
                    position = -1; // 反手开空
                    entry_bar = i;
                }
            }
            -1 => {
                if new_buy1 {
                    close_trade(&mut trades, entry_bar, i, false); // 平空
                    position = 1; // 反手开多
                    entry_bar = i;
                }
            }
            _ => unreachable!(),
        }
    }
    // 窗口终点：未平仓位强平（含浮盈口径，与工位 L forced_close 语义一致）。
    if position != 0 && n - 1 > entry_bar {
        trades.push(TradeRecord {
            entry_bar,
            exit_bar: n - 1,
            hold_bars: n - 1 - entry_bar,
            qty,
            long: position == 1,
            forced_close: true,
        });
    }

    // ── 逐 bar 持仓 MtM 权益曲线（同工位 L NAV 口径：close-to-close，含成本于成交 bar）。 ──
    // 仓位序列从 trades 重建（每笔 [entry,exit) 持有 long/short）。equity 归一化初始=1。
    let mut pos_per_bar: Vec<i8> = vec![0; n];
    for t in &trades {
        let sign: i8 = if t.long { 1 } else { -1 };
        for b in t.entry_bar..t.exit_bar {
            pos_per_bar[b] = sign;
        }
    }
    // 名义基准 = Σ 各笔入场名义额（与 significance nav_base 同口径，方向无关 |n_v|）。
    let nav_base: f64 = trades
        .iter()
        .map(|t| t.qty * prices[t.entry_bar.min(n - 1)] * (1.0 + fee_rate))
        .sum::<f64>()
        .max(1e-9);
    // per-bar 绝对盈亏 = 持仓 × 价格变化（close-to-close）；成交 bar 扣双边费。
    let mut equity_curve: Vec<f64> = Vec::with_capacity(n);
    let mut daily_returns: Vec<f64> = Vec::with_capacity(n);
    let mut cum_abs = 0.0;
    for i in 0..n {
        if i > 0 {
            let dp = prices[i] - prices[i - 1];
            cum_abs += pos_per_bar[i - 1] as f64 * qty * dp;
        }
        // 成交 bar 费用（开/平仓各扣一次单边名义费）。
        let traded_here = trades
            .iter()
            .filter(|t| t.entry_bar == i || t.exit_bar == i)
            .count() as f64;
        cum_abs -= traded_here * qty * prices[i] * fee_rate;
        let eq = 1.0 + cum_abs / nav_base;
        let ret = if i == 0 {
            0.0
        } else {
            let prev = *equity_curve.last().unwrap();
            if prev.abs() > 1e-12 {
                eq / prev - 1.0
            } else {
                0.0
            }
        };
        equity_curve.push(eq);
        daily_returns.push(ret);
    }

    // 逐笔无复利 pnl（significance bootstrap 输入；同口径）。
    let trade_pnls: Vec<f64> = trades
        .iter()
        .map(|t| {
            let e = prices[t.entry_bar.min(n - 1)];
            let x = prices[t.exit_bar.min(n - 1)];
            // trade_abs_pnl 私有——用同公式（方向感知）÷ nav_base。
            let raw = if t.long {
                t.qty * (x * (1.0 - fee_rate) - e * (1.0 + fee_rate))
            } else {
                t.qty * (e * (1.0 - fee_rate) - x * (1.0 + fee_rate))
            };
            raw / nav_base
        })
        .collect();

    let bh_return = if n >= 2 && prices[0].abs() > 1e-12 {
        prices[n - 1] / prices[0] - 1.0
    } else {
        0.0
    };
    let m = metrics::compute(&equity_curve, &daily_returns, &trade_pnls, years, bh_return);
    let sig = metrics::significance(
        &trade_pnls,
        &daily_returns,
        &trades,
        &prices,
        fee_rate,
        m.strat_return,
    );

    println!("=== 纯买卖点择时回测（pure_bsp_timing，离散状态机，非 π_Θ 覆盖）===");
    println!("品种            : {}", dataset.symbol);
    println!("bar 数          : {n}");
    println!("年化基数(years) : {years:.4}");
    println!("--- 交易（任意买点 bit→开多/任意卖点 bit→平多反手，仓位∈{{+1,0,-1}}）---");
    println!("成交交易笔数    : {}", trades.len());
    let n_long = trades.iter().filter(|t| t.long).count();
    println!("  多头/空头     : {n_long} / {}", trades.len() - n_long);
    println!("--- 指标（同工位 L NAV 口径，可比 π_Θ）---");
    println!("★Sharpe         : {:.4}", m.sharpe);
    println!("strat_return    : {:.4}", m.strat_return);
    println!("buy&hold_return : {:.4}", m.bh_return);
    println!("Sortino         : {:.4}", m.sortino);
    println!("MaxDrawdown     : {:.4}", m.max_drawdown);
    println!("win_rate        : {:.4}", m.win_rate);
    println!("profit_factor   : {:.4}", m.profit_factor);
    println!("--- 同口径随机对照（§4，seed 冻结）---");
    println!("theta_same_caliber : {:.6}", sig.theta_return_same_caliber);
    println!("shift  mean/p   : {:.6} / {:.4}", sig.shift_mean_return, sig.shift_pvalue);
    println!("indep  mean/p   : {:.6} / {:.4}", sig.indep_mean_return, sig.indep_pvalue);
    println!("beats_random    : {}", sig.theta_beats_random);
    println!("controls_degen  : {}", sig.controls_degenerate);
    println!("--- 认识论 L2（真实数据，可否证）---");
    if trades.is_empty() {
        println!("等级: L1（无第一类买卖点确认 ⟹ 无交易，inconclusive）");
    } else {
        println!("等级: L2——纯择时 Sharpe={:.4} 可与 π_Θ 覆盖 Sharpe 对照", m.sharpe);
    }
    std::process::ExitCode::SUCCESS
}
