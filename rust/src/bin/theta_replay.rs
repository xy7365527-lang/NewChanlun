//! #1305 ①件（#1304 第一批）：生产 π 回路历史回放 driver bin。
//!
//! 数据路径：`analysis/data_cache/*.json` → [`load_by_symbol`] → [`Dataset`] →
//! [`ThetaPiStream::push_bar`]（#951 seam）逐 bar 循环——生产 runner 链的历史回放入口。
//! 决策核与批量 fill loop（`run_theta_v0_pi`）**共核**（同一 `pi_theta_step_traced_with_risk_seeds`
//! + `OverlayState::step` + `LevelLedgerMirror::step`），本 driver 是流式单 bar 那半边。
//!
//! #1308 第二批：回放核心（排序 dump + env 钉死 + 同输入双跑自检）已抽到
//! [`newchan_rust::theta_v0::backtest::replay_dump`] 单源——本 bin 只做 CLI 解析与打印，
//! `theta_accept`（一键验收报告器）复用同一份 [`run_replay_double`] 物证。
//!
//! ## p_t / nav 喂法（state 裁定，stream.rs 模块头「无状态持仓口径」）
//!
//! `p_t` = 上一 bar 的 `p_star`（self-state 全成交无滑点模拟，同 `ffi::push_bar_selfstate` 研究
//! 对拍口径）；`nav` = 固定 `initial_nav`（首价量级匹配，同 theta_backtest）。生产实盘由 Python
//! 侧显式传真实净持仓；本 driver 是**决策核**历史回放，不接 fill 模拟（fill 差异归 #1304 ②件
//! 对拍 harness 的「共核不共外围」预期差清单）。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin theta_replay -- <SYMBOL> [START END] [--out PATH]
//!   <SYMBOL>       品种代码（BTC/ES/CL/GC/BRN/DX/QQQ/OKLO，见 data::SYMBOLS）
//!   [START END]    可选 ISO 日期窗闭区间（如 2024-01-01 2024-12-31）；省略 = 全量
//!   [--out PATH]   可选：把 run-1 的确定性 dump（JSONL）写到此路径
//! ```
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! **L1**（管线正确性 + 确定性）：本 driver 验证「历史 feed → 决策核」串通与确定性，不验证
//! Θ 在市场有效。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::replay_dump::run_replay_double;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::env_registry;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let mut symbol: Option<&str> = None;
    let mut window: Option<(&str, &str)> = None;
    let mut out_path: Option<&str> = None;

    let mut rest = args.iter().skip(1).peekable();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--out" => match rest.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("--out 需要路径参数");
                    return std::process::ExitCode::from(2);
                }
            },
            _ if symbol.is_none() => symbol = Some(a),
            start => match rest.next() {
                Some(end) => window = Some((start, end)),
                None => {
                    eprintln!("日期窗需要 START END 两个参数");
                    return std::process::ExitCode::from(2);
                }
            },
        }
    }

    let symbol = match symbol {
        Some(s) => s,
        None => {
            eprintln!(
                "用法: {} <SYMBOL> [START_DATE END_DATE] [--out PATH]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO\n  日期窗(可选): ISO 闭区间，如 2024-01-01 2024-12-31",
                args.first().map(String::as_str).unwrap_or("theta_replay")
            );
            return std::process::ExitCode::from(2);
        }
    };

    let config = ThetaConfig::default();

    // ── 数据加载（data → 决策核边界）。未知品种/读取失败 fail-loud。 ──
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
            "品种 {} 在指定窗内无 bar（空数据集）——无法回放，判 inconclusive。",
            symbol
        );
        return std::process::ExitCode::FAILURE;
    }

    // ── 同输入双跑逐位自检（确定性自证，#1308 起在 replay_dump 单源）。 ──
    let outcome = run_replay_double(&dataset, &config, window);

    println!("=== θ 回放 driver（#1305 ①件，#1304 第一批）===");
    println!("品种            : {}", symbol);
    match window {
        Some((s, e)) => println!("日期窗          : {s} .. {e}"),
        None => println!("日期窗          : 全量"),
    }
    println!("bar 数          : {}", outcome.n_bars);
    println!("不可交易占比    : {:.2}%", outcome.untradable_ratio * 100.0);
    println!("初始 NAV        : {:.2}", outcome.initial_nav);
    println!("env 注册表键数  : {}", env_registry::REGISTRY.len());
    println!("dump 字节数     : {}", outcome.dump.len());
    println!(
        "双跑耗时        : {:.2}s（{:.0} bar/s，双跑共 {} bar）",
        outcome.elapsed.as_secs_f64(),
        (outcome.n_bars as f64 * 2.0) / outcome.elapsed.as_secs_f64().max(1e-9),
        outcome.n_bars * 2,
    );
    if outcome.identical {
        println!("同输入双跑自检  : PASS（逐位一致）");
    } else {
        let offset = outcome.first_divergence_offset.unwrap_or(0);
        eprintln!("同输入双跑自检  : FAIL（首个分歧偏移 = {}）", offset);
        let lo = offset.saturating_sub(64);
        // 分歧点双片段（run-1/run-2）——两跑字节不等的现场，缺一不可诊断。
        let hi = match &outcome.dump2 {
            Some(d2) => (offset + 64).min(outcome.dump.len().min(d2.len())),
            None => (offset + 64).min(outcome.dump.len()),
        };
        eprintln!("run-1 片段: {}", &outcome.dump[lo..hi]);
        if let Some(d2) = &outcome.dump2 {
            eprintln!("run-2 片段: {}", &d2[lo..hi]);
        }
        return std::process::ExitCode::FAILURE;
    }

    if let Some(path) = out_path {
        if let Err(e) = std::fs::write(path, &outcome.dump) {
            eprintln!("写 dump 文件 {path} 失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
        println!("dump 已写入     : {path}");
    }

    if outcome.untradable_ratio > 0.20 {
        eprintln!(
            "警告: 不可交易占比 {:.1}% > 20%，协议 §5.5 判该窗 inconclusive。",
            outcome.untradable_ratio * 100.0
        );
    }

    std::process::ExitCode::SUCCESS
}
