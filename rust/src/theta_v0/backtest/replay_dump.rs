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
        // ★#1318 补锁（运行时断言，非注释）：Schedule_Θ 单出口契约——回放每 bar 的分派/
        // allocator 产出必须与 |round(p_star − p_t)| 逐位一致（与 stream 集成测试同款锁，
        // 但此处跑在回放 driver 本体路径上，release 同样执行）。
        assert_eq!(
            order.qty,
            (p_star - p_t).abs().round() as i64,
            "#1318 回放锁：bar {i} last_order.qty 与 |round(p_star − p_t)| 不一致（Schedule_Θ 单出口破）"
        );
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

    // ★#1318 补锁（运行时断言）：决策核（分类塔 → 三类买卖点判据 → π step）必须逐 bar 驱动
    // 每一根输入 bar——bar_count 落后 ⟹ 回放静默漏 bar（生产链未逐 bar 执行）。
    assert_eq!(
        stream.bar_count(),
        dataset.bars.len(),
        "#1318 回放锁：决策核驱动 bar 数 {} != 数据集 bar 数 {}（分类塔/买卖点判据未逐 bar 执行）",
        stream.bar_count(),
        dataset.bars.len()
    );

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

    // ★#1318 补锁（运行时断言）：两处账本写入的守恒锚——①级别账本 LEE-Net 恒等（整数手数
    // 求和，残差必须恒 0，非 eps 容差）；②逐声部账本对账（account_price_pnl ≈ Σ_v pnl_v，
    // 排序和与账户侧一致，f64 求和序误差按相对容差判）。任一破 ⟹ 账本写入不守恒，回放结果
    // 不可信。
    assert_eq!(
        w.max_abs_residual, 0,
        "#1318 回放锁：LEE-Net 恒等残差 {} != 0（级别账本写入与 overlay 净敞口漂移）",
        w.max_abs_residual
    );
    // f64 对账残差取**相对容差**（1e-6 × 最大侧量级，下限 1.0）：account_price_pnl 与分账本
    // pnl_v 的求和序不同（账户按 bar 时序、分账本按声部序再排序求和），误差随量级线性放大，
    // 绝对容差会在 12 个月大名义回放上误报（与 stream 集成测试的绝对 1e-6 口径不同——那是
    // 80 根小名义合成 bar）。
    let reconcile_scale = account_price_pnl
        .abs()
        .max(total_voice_pnl_sorted.abs())
        .max(1.0);
    assert!(
        reconcile_residual_sorted < 1e-6 * reconcile_scale,
        "#1318 回放锁：账户/分账本对账残差 {reconcile_residual_sorted} >= 1e-6 × scale {reconcile_scale}（逐声部账本写入不守恒）"
    );

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
    /// run-2 的确定性 dump——仅 `identical` = false 时保留（`Some`）；成功路径置 `None`，
    /// 以免批量验收（`theta_accept` 不读 dump 内容）为每品种多留一份全量 dump。
    pub dump2: Option<String>,
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
    // run-2 dump 只在分歧时保留（分歧点双片段打印的物证）；成功路径不保留，以免
    // theta_accept 批量验收为每品种多留一份全量 dump。
    let dump2 = if identical { None } else { Some(dump2) };

    ReplayOutcome {
        symbol: dataset.symbol.clone(),
        window: window.map(|(s, e)| (s.to_string(), e.to_string())),
        n_bars: dataset.bars.len(),
        n_tradable: dataset.bars.iter().filter(|b| !b.untradable).count(),
        untradable_ratio: dataset.untradable_ratio(),
        initial_nav,
        years,
        dump: dump1,
        dump2,
        identical,
        first_divergence_offset,
        elapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::types::Bar;
    use super::*;

    /// 确定性合成锯齿行情（全可交易，无 untradable bar）——与 `theta_pi_diff` 测试同款
    /// 4 升/4 降步进形态：足以形成分型/笔/段/中枢，让回放路径跑在真结构上；决策层可能
    /// 恒空（不产买卖点信号），但回放锁（逐 bar 驱动 + Schedule_Θ 单出口 + 两账本守恒）
    /// 在空账与真结构上都必须成立。
    fn synthetic_bars() -> Vec<Bar> {
        (0..240)
            .map(|i| {
                let up = ((i / 4) % 2) == 0;
                let base = 10_000_000_000i64;
                let step = 250_000_000i64 * ((i % 4) as i64);
                let close_tick = if up {
                    base + step
                } else {
                    base + 1_000_000_000 - step
                };
                Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: close_tick,
                    high: close_tick,
                    low: close_tick,
                    close: close_tick,
                    volume: 1.0,
                    untradable: false,
                }
            })
            .collect()
    }

    /// #1318：回放路径本体必须触发逐 bar 决策核 + Schedule_Θ 单出口 + 两账本守恒的运行时
    /// 断言——本测试直接跑 `run_replay_double`（即 theta_replay 的同一份回放核心），断言
    /// 在合成数据上逐位双跑一致（= 四道回放锁全部通过，任一锁破会在此先 panic）。
    #[test]
    fn replay_double_exercises_runtime_locks_on_synthetic_data() {
        let bars = synthetic_bars();
        let dates: Vec<String> = bars
            .iter()
            .enumerate()
            .map(|(i, _)| format!("2024-01-01T{:02}:{:02}:00+00:00", (i / 60) % 24, i % 60))
            .collect();
        let dataset = Dataset {
            symbol: "SYNTH".to_string(),
            bars,
            dates,
            bar_seconds: 60,
        };
        let config = ThetaConfig::default();
        let outcome = run_replay_double(&dataset, &config, None);
        assert!(outcome.identical, "同输入双跑必须逐位一致");
        assert_eq!(outcome.n_bars, dataset.bars.len());
        assert_eq!(outcome.n_bars, 240);
    }
}
