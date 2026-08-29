//! #1305 ①件（#1304 第一批）：生产 π 回路历史回放 driver bin。
//!
//! ## 它做什么
//!
//! 数据路径：`analysis/data_cache/*.json` → [`load_by_symbol`] → [`Dataset`] →
//! [`ThetaPiStream::push_bar`]（#951 seam）逐 bar 循环——生产 runner 链的历史回放入口。
//! 决策核与批量 fill loop（`run_theta_v0_pi`）**共核**（同一 `pi_theta_step_traced_with_risk_seeds`
//! + `OverlayState::step` + `LevelLedgerMirror::step`），本 driver 是流式单 bar 那半边。
//!
//! ## 确定性自证（#1304 scan §1 的两处回放消除项）
//!
//! 1. **dump 输出按键排序**：终态逐声部账本 `OverlayState::active_voices()` 迭代的是内部
//!    `HashMap`（跨进程 `RandomState` 序不稳定，shadow.rs:235 教训），`closed_voices()` 的
//!    Vec 序也随 books 的 HashMap 键序漂移——本 driver dump 前按 `(level, ordinal[, entry_bar,
//!    exit_bar])` 排序，逐字节稳定。
//! 2. **env 钉死档案**：回放 env 固定集 = [`env_registry`] 全键（运行时枚举，自跟踪注册表
//!    漂移）。每键当前值（未设 ⟹ null）逐条进 dump 的 `env_archive` 行，行为门置位计数随行。
//!
//! ## 同输入双跑逐位自检（验收判据 1）
//!
//! 同一 `Dataset` 喂两个独立 `ThetaPiStream` 各跑一遍，dump 字节串逐位比较；不一致则打印首个
//! 分歧偏移并 exit FAILURE。浮点域逐位稳定的前提：决策核在整数 tick 域（价格），p_star/账本
//! f64 由固定顺序的 IEEE-754 运算产出；本 driver 的 dump 层只求和排序后序列，不读
//! `OverlayState::total_voice_pnl()`（该法是 HashMap 序 f64 求和，跨进程最后一位不稳）。
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

use std::collections::BTreeMap;
use std::time::Instant;

use newchan_rust::theta_v0::backtest::data::{bars_per_year, load_by_symbol, Dataset};
use newchan_rust::theta_v0::classifier::recursive_tower::ElementId;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::env_registry::{self, GateKind};
use newchan_rust::theta_v0::strategy::coverage::Vertical;
use newchan_rust::theta_v0::strategy::overlay_state::{ClosedVoice, VoiceBook};
use newchan_rust::theta_v0::strategy::voice::VoiceSide;
use newchan_rust::theta_v0::stream::ThetaPiStream;
use newchan_rust::theta_v0::types::StrictAction;
use serde_json::{json, Value};

fn action_str(a: StrictAction) -> &'static str {
    match a {
        StrictAction::Buy => "buy",
        StrictAction::Sell => "sell",
        StrictAction::Add => "add",
        StrictAction::Reduce => "reduce",
        StrictAction::Hold => "hold",
        StrictAction::Close => "close",
        StrictAction::Wait => "wait",
    }
}

fn side_str(s: VoiceSide) -> &'static str {
    match s {
        VoiceSide::Long => "long",
        VoiceSide::Short => "short",
        VoiceSide::Flat => "flat",
    }
}

fn vertical_str(v: Vertical) -> &'static str {
    match v {
        Vertical::Ambient => "ambient",
        Vertical::FollowParent => "follow_parent",
        Vertical::ReverseOpen => "reverse_open",
    }
}

/// 声部身份稳定排序键（`ElementId` 无 `Ord`，dump 层按 `(level, ordinal)` 排）。
fn id_key(id: &ElementId) -> (u32, u64) {
    (id.level, id.ordinal)
}

/// header 行（单次回放不变量；两跑共用同一份前缀，比较时不参与重跑差异）。
fn header_line(
    dataset: &Dataset,
    config: &ThetaConfig,
    window: Option<(&str, &str)>,
    initial_nav: f64,
    years: f64,
) -> String {
    let n_tradable = dataset.bars.iter().filter(|b| !b.untradable).count();
    let line = json!({
        "kind": "header",
        "symbol": dataset.symbol,
        "window": window,
        "n_bars": dataset.bars.len(),
        "n_tradable": n_tradable,
        "untradable_ratio": dataset.untradable_ratio(),
        "bar_seconds": dataset.bar_seconds,
        "tick_size": config.tick.tick_size,
        "initial_nav": initial_nav,
        "years": years,
        "env_registry_keys": env_registry::REGISTRY.len(),
    });
    serde_json::to_string(&line).expect("header 行序列化失败") + "\n"
}

/// env 钉死档案行：env_registry 全键（运行时枚举）→ 当前值（未设 ⟹ null），键名字典序。
fn env_archive_line() -> String {
    let mut env: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut behavior_gates_set = 0usize;
    for m in env_registry::all() {
        let value = std::env::var(m.key).ok();
        if m.kind == GateKind::Behavior && value.is_some() {
            behavior_gates_set += 1;
        }
        env.insert(m.key.to_string(), value);
    }
    let line = json!({
        "kind": "env_archive",
        "behavior_gates_set": behavior_gates_set,
        "env": env,
    });
    serde_json::to_string(&line).expect("env_archive 行序列化失败") + "\n"
}

fn voice_book_json(v: &VoiceBook) -> Value {
    json!({
        "id": (v.id.level, v.id.ordinal),
        "side": side_str(v.side),
        "q": v.q,
        "role_v": vertical_str(v.role_v),
        "parent_id": v.parent_id.map(|p| (p.level, p.ordinal)),
        "entry_bar": v.entry_bar,
        "entry_px": v.entry_px,
        "pnl_v": v.pnl_v,
    })
}

fn closed_voice_json(c: &ClosedVoice) -> Value {
    json!({
        "id": (c.id.level, c.id.ordinal),
        "side": side_str(c.side),
        "role_v": vertical_str(c.role_v),
        "parent_id": c.parent_id.map(|p| (p.level, p.ordinal)),
        "entry_bar": c.entry_bar,
        "exit_bar": c.exit_bar,
        "entry_px": c.entry_px,
        "exit_px": c.exit_px,
        "pnl_v": c.pnl_v,
    })
}

/// 单次回放：Dataset → push_bar 循环 + 终态账本 → JSONL 行（不含 header/env 前缀）。
fn replay(dataset: &Dataset, config: &ThetaConfig, initial_nav: f64) -> String {
    let mut out = String::new();
    let mut stream = ThetaPiStream::new(config.clone());

    // ── 决策核逐 bar 循环（#951 seam：push_bar(bar, p_t, nav) -> p_star）。 ──
    for (i, bar) in dataset.bars.iter().enumerate() {
        let p_t = stream.target_net_units(); // self-state：上一 bar 的 p_star（全成交模拟）
        let p_star = stream.push_bar(*bar, p_t, initial_nav);
        let order = stream.last_order();
        let line = json!({
            "kind": "bar",
            "i": i,
            "p_star": p_star,
            "action": action_str(order.action),
            "qty": order.qty,
            "exec_index": order.exec_index,
        });
        out.push_str(&serde_json::to_string(&line).expect("bar 行序列化失败"));
        out.push('\n');
    }

    // ── 收尾：窗口终点强平 + 终态账本导出（D1 per-leg 出口面）。 ──
    stream.finish();
    let overlay = stream.overlay();
    let level_ledger = stream.level_ledger();

    // ★dump 键排序（shadow.rs:235 教训）：active/closed 源自 HashMap 迭代，跨进程序不稳定。
    let mut active: Vec<&VoiceBook> = overlay.active_voices().collect();
    active.sort_by_key(|v| id_key(&v.id));
    let active_json: Vec<Value> = active.iter().map(|v| voice_book_json(v)).collect();

    let mut closed: Vec<&ClosedVoice> = overlay.closed_voices().iter().collect();
    closed.sort_by_key(|c| (id_key(&c.id), c.entry_bar, c.exit_bar));
    let closed_json: Vec<Value> = closed.iter().map(|c| closed_voice_json(c)).collect();

    // 分账本侧求和用**排序后**序列（f64 求和序敏感；OverlayState::total_voice_pnl 是
    // HashMap 序求和，跨进程最后一位不稳 ⟹ 本 dump 不读它，自算排序和并加 `_sorted` 标）。
    let account_price_pnl = overlay.account_price_pnl();
    let total_voice_pnl_sorted: f64 =
        active.iter().map(|v| v.pnl_v).sum::<f64>() + closed.iter().map(|c| c.pnl_v).sum::<f64>();
    let reconcile_residual_sorted = (account_price_pnl - total_voice_pnl_sorted).abs();

    let level_nets: Vec<(u32, i64)> = level_ledger.nets().iter().map(|(l, n)| (*l, *n)).collect();
    let w = level_ledger.lee_net_witness();

    let line = json!({
        "kind": "result",
        "bar_count": stream.bar_count(),
        "p_star": stream.target_net_units(),
        "n_orders": stream.n_orders(),
        "account_price_pnl": account_price_pnl,
        "total_voice_pnl_sorted": total_voice_pnl_sorted,
        "reconcile_residual_sorted": reconcile_residual_sorted,
        "active_voices": active_json,
        "closed_voices": closed_json,
        "level_nets": level_nets,
        "lee_net_witness": {
            "n_observations": w.n_observations,
            "max_abs_residual": w.max_abs_residual,
            "max_abs_net": w.max_abs_net,
            "identity_witnessed": w.identity_witnessed(),
        },
    });
    out.push_str(&serde_json::to_string(&line).expect("result 行序列化失败"));
    out.push('\n');
    out
}

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

    // initial_nav：与品种价量级匹配（首价 × 容量倍数），否则 sizing qty 取整为 0。
    let first_px = dataset.bars[0].close as f64 * config.tick.tick_size;
    let initial_nav = (first_px.abs() * 10_000.0).max(1.0e6);
    let years = (dataset.bars.len() as f64 / bars_per_year(dataset.bar_seconds)).max(1e-9);

    let header = header_line(&dataset, &config, window, initial_nav, years);
    let env_archive = env_archive_line();

    // ── 同输入双跑逐位自检（确定性自证）。 ──
    let started = Instant::now();
    let body1 = replay(&dataset, &config, initial_nav);
    let body2 = replay(&dataset, &config, initial_nav);
    let elapsed = started.elapsed();

    let dump1 = format!("{header}{env_archive}{body1}");
    let dump2 = format!("{header}{env_archive}{body2}");
    let identical = dump1 == dump2;

    println!("=== θ 回放 driver（#1305 ①件，#1304 第一批）===");
    println!("品种            : {}", symbol);
    match window {
        Some((s, e)) => println!("日期窗          : {s} .. {e}"),
        None => println!("日期窗          : 全量"),
    }
    println!("bar 数          : {}", dataset.bars.len());
    println!(
        "不可交易占比    : {:.2}%",
        dataset.untradable_ratio() * 100.0
    );
    println!("初始 NAV        : {:.2}", initial_nav);
    println!("env 注册表键数  : {}", env_registry::REGISTRY.len());
    println!("dump 字节数     : {}", dump1.len());
    println!(
        "双跑耗时        : {:.2}s（{:.0} bar/s，双跑共 {} bar）",
        elapsed.as_secs_f64(),
        (dataset.bars.len() as f64 * 2.0) / elapsed.as_secs_f64().max(1e-9),
        dataset.bars.len() * 2,
    );
    if identical {
        println!("同输入双跑自检  : PASS（逐位一致）");
    } else {
        let offset = dump1
            .bytes()
            .zip(dump2.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(dump1.len().min(dump2.len()));
        eprintln!("同输入双跑自检  : FAIL（首个分歧偏移 = {}）", offset);
        let lo = offset.saturating_sub(64);
        let hi = (offset + 64).min(dump1.len().min(dump2.len()));
        eprintln!("run-1 片段: {}", &dump1[lo..hi]);
        eprintln!("run-2 片段: {}", &dump2[lo..hi]);
        return std::process::ExitCode::FAILURE;
    }

    if let Some(path) = out_path {
        if let Err(e) = std::fs::write(path, &dump1) {
            eprintln!("写 dump 文件 {path} 失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
        println!("dump 已写入     : {path}");
    }

    if dataset.untradable_ratio() > 0.20 {
        eprintln!(
            "警告: 不可交易占比 {:.1}% > 20%，协议 §5.5 判该窗 inconclusive。",
            dataset.untradable_ratio() * 100.0
        );
    }

    std::process::ExitCode::SUCCESS
}
