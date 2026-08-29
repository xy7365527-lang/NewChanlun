//! #1308 第二批：回放 driver 核心（从 `theta_replay` bin 抽出的可复用半边）。
//!
//! `theta_replay`（#1305 ①件）把「Dataset → ThetaPiStream::push_bar 循环 + 排序 dump +
//! env 钉死 + 同输入双跑逐位自检」全部写在 bin 里，`theta_accept`（#1308 验收报告器）要复用
//! 同一份回放物证，不能「另写一份同义代码」（AGENTS.md 总缝规则）——故抽出本模块：
//! bin 只做 CLI 解析与打印，回放与双跑自检的语义在本处单源。
//!
//! ## 确定性自证（同 #1305 原文）
//!
//! 1. **dump 输出按键排序**：终态逐声部账本 `OverlayState::active_voices()` 迭代的是内部
//!    `HashMap`（跨进程 `RandomState` 序不稳定，shadow.rs:235 教训），`closed_voices()` 的
//!    Vec 序也随 books 的 HashMap 键序漂移——dump 前按 `(level, ordinal[, entry_bar,
//!    exit_bar])` 排序，逐字节稳定。
//! 2. **env 钉死档案**：回放 env 固定集 = [`env_registry`] 全键（运行时枚举，自跟踪注册表
//!    漂移）。每键当前值（未设 ⟹ null）逐条进 dump 的 `env_archive` 行，行为门置位计数随行。
//!
//! ## 同输入双跑逐位自检（验收判据 1）
//!
//! 同一 `Dataset` 喂两个独立 `ThetaPiStream` 各跑一遍，dump 字节串逐位比较；不一致则记录首个
//! 分歧偏移（[`ReplayOutcome::identical`] = false）。浮点域逐位稳定的前提：决策核在整数 tick
//! 域（价格），p_star/账本 f64 由固定顺序的 IEEE-754 运算产出；dump 层只求和排序后序列，
//! 不读 `OverlayState::total_voice_pnl()`（HashMap 序 f64 求和，跨进程最后一位不稳）。

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::super::classifier::recursive_tower::ElementId;
use super::super::config::ThetaConfig;
use super::super::env_registry::{self, GateKind};
use super::super::strategy::coverage::Vertical;
use super::super::strategy::overlay_state::{ClosedVoice, VoiceBook};
use super::super::strategy::voice::VoiceSide;
use super::super::stream::ThetaPiStream;
use super::super::types::StrictAction;
use super::data::{bars_per_year, Dataset};
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

/// 单品种一次回放验收的完整结果（#1308 验收报告器的回放侧物证）。
#[derive(Debug)]
pub struct ReplayOutcome {
    pub symbol: String,
    pub window: Option<(String, String)>,
    pub n_bars: usize,
    pub n_tradable: usize,
    pub untradable_ratio: f64,
    pub initial_nav: f64,
    pub years: f64,
    /// run-1 的确定性 dump（header + env_archive + body）。
    pub dump: String,
    /// 同输入双跑逐位一致 ⟺ true。
    pub identical: bool,
    /// 首个分歧字节偏移（`identical` 时恒 None）。
    pub first_divergence_offset: Option<usize>,
    pub elapsed: Duration,
}

/// initial_nav 口径（与 `theta_replay` #1305 原文一致）：首价 × 1e4 倍容量，下限 1e6——
/// 与品种价量级匹配，否则 sizing qty 取整为 0。
pub fn initial_nav_for(dataset: &Dataset, config: &ThetaConfig) -> f64 {
    let first_px = dataset.bars[0].close as f64 * config.tick.tick_size;
    (first_px.abs() * 10_000.0).max(1.0e6)
}

/// 单品种同输入双跑逐位自检（确定性自证）。空数据集调用方自行判 inconclusive。
pub fn run_replay_double(
    dataset: &Dataset,
    config: &ThetaConfig,
    window: Option<(&str, &str)>,
) -> ReplayOutcome {
    let initial_nav = initial_nav_for(dataset, config);
    let years = (dataset.bars.len() as f64 / bars_per_year(dataset.bar_seconds)).max(1e-9);

    let header = header_line(dataset, config, window, initial_nav, years);
    let env_archive = env_archive_line();

    let started = Instant::now();
    let body1 = replay(dataset, config, initial_nav);
    let body2 = replay(dataset, config, initial_nav);
    let elapsed = started.elapsed();

    let dump1 = format!("{header}{env_archive}{body1}");
    let dump2 = format!("{header}{env_archive}{body2}");
    let identical = dump1 == dump2;
    let first_divergence_offset = if identical {
        None
    } else {
        dump1
            .bytes()
            .zip(dump2.bytes())
            .position(|(a, b)| a != b)
            .or(Some(dump1.len().min(dump2.len())))
    };

    ReplayOutcome {
        symbol: dataset.symbol.clone(),
        window: window.map(|(s, e)| (s.to_string(), e.to_string())),
        n_bars: dataset.bars.len(),
        n_tradable: dataset.bars.iter().filter(|b| !b.untradable).count(),
        untradable_ratio: dataset.untradable_ratio(),
        initial_nav,
        years,
        dump: dump1,
        identical,
        first_divergence_offset,
        elapsed,
    }
}
