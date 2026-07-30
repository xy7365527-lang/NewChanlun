//! # `theta_backtest` —— S_Θ 回测生产入口 CLI（goal acceptance[5] 最小闭环）
//!
//! 数据 → S_Θ（canonical π_Θ 七链引擎）→ 回测指标的**可运行生产入口**。证明 CLI 能驱动
//! S_Θ 产出（formalization-validity-domain：**L1** 管线就绪 + 真实数据加载；**L2** 当且仅当
//! 引擎产非空订单流——由 `RunResult.is_l2` 如实标注，CLI **不**伪造"已验证 Θ 有效"）。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin theta_backtest -- <SYMBOL> [START END]
//!   <SYMBOL>      品种代码（BTC/ES/CL/GC/BRN/DX/QQQ/OKLO，见 data::SYMBOLS）
//!   [START END]   可选 ISO 日期窗闭区间（如 2023-01-01 2025-06-30）；省略 = 全量
//! 例：cargo run --release --features backtest_bin --bin theta_backtest -- BTC 2024-01-01 2024-06-30
//! ```
//!
//! ## 回测/实盘统一 + Nautilus 接入（设计 §4.1 / §5，本轮 ceiling）
//!
//! 本 CLI 是**最小生产路径**：用 in-crate 引擎（`run_theta_v0_pi` = π_Θ 七链 + 内部 fill 模拟）
//! 直接驱动 S_Θ。订单经 `theta_v0::nautilus::order_adapter` 可转为 `OrderIntent`（数据流贯通已由
//! nautilus 模块 self-check 证），但**本轮不接 nautilus_trader crate**——
//!
//! ponytail: 不加 nautilus_trader 依赖。理由（设计 §5.4 + §1.4 边界）：nautilus crate 重、
//! bi-weekly breaking change、Rust IB adapter 仅在 develop 分支（无 tagged release 背书）。
//! 最小生产路径（证 "CLI 驱动 S_Θ 产回测结果"）**不需要** nautilus 引擎——in-crate fill 已闭环。
//! 升级路径（goal "回测实盘同引擎" 的完整兑现）：新增 workspace member `rust/nautilus_bridge/`
//! 依赖 nautilus-{trading,model,backtest}，把本 CLI 的 `run_theta_v0_pi` 替换为
//! `BacktestNode.add_strategy(ThetaStrategy)`（回测）/ `LiveNode + IB`（实盘），同一 `ThetaStrategy`
//! 零代码切换（设计 §4.1）。`ThetaStrategy` 适配器骨架已在 `theta_v0::nautilus::strategy` 就位。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::runner::run_theta_v0_pi;
use newchan_rust::theta_v0::config::ThetaConfig;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // ── 输入验证（系统边界，coding-style 不可省）。 ──
    if args.len() != 2 && args.len() != 4 {
        eprintln!(
            "用法: {} <SYMBOL> [START_DATE END_DATE]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO\n  日期窗(可选): ISO 闭区间，如 2023-01-01 2025-06-30",
            args.first().map(String::as_str).unwrap_or("theta_backtest")
        );
        return std::process::ExitCode::from(2);
    }
    let symbol = &args[1];
    let window = if args.len() == 4 {
        Some((args[2].as_str(), args[3].as_str()))
    } else {
        None
    };

    // ── #344 LOW-3：非 BTC 品种在 ⑤ 段（in-crate 回测）起跑前 fail-fast。 ──
    // 复用 ⑥ 段同一门控（`require_btc_symbol`，#343），单一事实源——避免非 BTC 品种
    // 白跑一遍全量 in-crate 回测才在 ⑥ 段真实 Nautilus 引擎处才发现不支持。⑥ 段入口
    // （`run_theta_backtest` 内部）的门控调用保留，两处调用同一函数不构成重复门控。
    //
    // 冒烟验证：`cargo run --release --features backtest_bin --bin theta_backtest -- ES`
    // 应立即 fail-fast 打印门控错误并退出，不打印任何 "=== S_Θ 回测结果 ===" 输出。
    if let Err(e) = newchan_rust::theta_v0::nautilus::backtest_engine::require_btc_symbol(symbol) {
        eprintln!("{e}");
        return std::process::ExitCode::FAILURE;
    }

    let config = ThetaConfig::default();

    // ── 数据加载（data → S_Θ 边界）。未知品种/读取失败 fail-loud。 ──
    let full = match load_by_symbol(symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let dataset = match window {
        Some((start, end)) => {
            if start > end {
                eprintln!("日期窗非法: START `{start}` > END `{end}`");
                return std::process::ExitCode::from(2);
            }
            full.slice_date_window(start, end)
        }
        None => full,
    };

    // 空窗/空数据：合法的"该窗无数据"，如实报告（非 panic）。
    if dataset.bars.is_empty() {
        eprintln!(
            "品种 {} 在指定窗内无 bar（空数据集）——无法回测，判 inconclusive。",
            symbol
        );
        return std::process::ExitCode::FAILURE;
    }

    // years：bar 数 / 一年 bar 数（CAGR 年化基数，runner §3.1）。粒度从 dataset 携带（barspec-impl
    // A 点）——1min ⟹ bars_per_year(60) bit-exact = 旧硬编码 365.25*24*60；1s ⟹ ×60。
    // ponytail: 用 bar 计数近似而非历法换算（data.rs 已声明无 chrono 依赖，时间戳只用于排序）。
    let years = (dataset.bars.len() as f64
        / newchan_rust::theta_v0::backtest::data::bars_per_year(dataset.bar_seconds))
    .max(1e-9);

    // initial_nav：与品种价量级匹配（首价 × 容量倍数），否则 sizing qty 取整为 0（runner:111）。
    let first_px = dataset.bars[0].close as f64 * config.tick.tick_size;
    let initial_nav = (first_px.abs() * 10_000.0).max(1.0e6);

    // ── S_Θ 回测（π_Θ 七链生产引擎——runner.rs:1442 标注为接 Nautilus 实装的链）。 ──
    let r = run_theta_v0_pi(&dataset, &config, years, initial_nav);

    println!("=== S_Θ 回测结果（theta_v0::run_theta_v0_pi）===");
    println!("品种            : {}", r.symbol);
    if let Some((s, e)) = window {
        println!("日期窗          : {s} .. {e}");
    } else {
        println!("日期窗          : 全量");
    }
    println!("bar 数          : {}", r.n_bars);
    println!("不可交易占比    : {:.2}%", r.untradable_ratio * 100.0);
    println!("初始 NAV        : {:.2}", initial_nav);
    println!("年化基数(years) : {:.4}", years);
    println!("--- 订单/交易 ---");
    println!("订单数          : {}", r.n_orders);
    println!("成交交易笔数    : {}", r.metrics.n_trades);
    if let Some(s) = &r.strict_nest_sidecar {
        println!("--- 严格区间套 sidecar（THETA_STRICT_NEST_SIDECAR）---");
        println!("sidecar 帧数     : {}", s.frames);
        println!("base cand_delta  : {}", s.base_count);
        println!("terminal 查无    : {}", s.terminal_missing);
        println!("证书总数         : {}", s.cert_total);
        println!("每级证书数       : {:?}", s.cert_per_top);
    }
    println!("--- 指标（含浮盈口径，runner §3）---");
    println!("strat_return    : {:.4}", r.metrics.strat_return);
    println!("buy&hold_return : {:.4}", r.metrics.bh_return);
    println!("CAGR            : {:.4}", r.metrics.cagr);
    println!("Sharpe          : {:.4}", r.metrics.sharpe);
    println!("MaxDrawdown     : {:.4}", r.metrics.max_drawdown);
    println!("win_rate        : {:.4}", r.metrics.win_rate);
    println!("--- 认识论等级（formalization-validity-domain 231号）---");
    if r.is_l2 {
        println!("等级: L2（真实数据 + 非空订单流）——结果可被否证。");
        println!("★L2 ≠ 证明 Θ 盈利：单标的单窗，否证鲁棒性需 L3（多标的多窗）。");
    } else {
        println!("等级: L1（管线串通，订单流为空）——无交易，未达 L2。");
        println!("★n_orders=0：数据无缠论结构(中枢+离开+回试) ⟹ 无 BSP ⟹ 无订单（诚实退化）。");
    }

    // 不可交易占比 >20% 协议判 inconclusive（§5.5）——警告但不改 exit code（结果已如实打印）。
    if r.untradable_ratio > 0.20 {
        eprintln!(
            "警告: 不可交易占比 {:.1}% > 20%，协议 §5.5 判该窗 inconclusive。",
            r.untradable_ratio * 100.0
        );
    }

    // ── ⑥ 真实 Nautilus BacktestEngine 路径（goal acceptance[5] L2：真引擎产非空订单流）。 ──
    // ★生产 Param：entry_delay_bars=0（#7：流式逐 bar 退出在末根触发，delay 归 venue 撮合）。
    let mut nautilus_config = ThetaConfig::default();
    nautilus_config.exec.entry_delay_bars = 0;
    println!("--- ⑥ 真实 Nautilus BacktestEngine（生产引擎贯通验证）---");
    match newchan_rust::theta_v0::nautilus::backtest_engine::run_theta_backtest(
        &dataset,
        &nautilus_config,
    ) {
        Ok(bt) => {
            println!("引擎迭代次数    : {}", bt.iterations);
            println!("总订单数        : {}", bt.total_orders);
            println!("总持仓数        : {}", bt.total_positions);
            if bt.total_orders > 0 {
                println!("等级: L2（真实 Nautilus 引擎 + 非空订单流）——生产引擎贯通验证通过。");
            } else {
                println!(
                    "等级: L1（真实引擎跑通，订单流为空）——数据无缠论结构 ⟹ 无 BSP ⟹ 无订单。"
                );
            }
        }
        Err(e) => {
            eprintln!("Nautilus BacktestEngine 运行失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    }

    std::process::ExitCode::SUCCESS
}
