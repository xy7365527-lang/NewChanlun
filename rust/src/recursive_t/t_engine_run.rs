//! **T 操作层引擎全量回测 runner**：8 标的 × 3 模式（Structural/AND/OR）。
//!
//! 与 `backtest_run.rs` 的范畴差：`backtest_run` 跑 `backtest.rs` 的**简化** `apply_bsp`
//! （只做多/减仓不回补/向下空仓，单级别）；本 runner 跑 `stream.rs → t_engine.rs` 的
//! **操作层自我复制完整引擎**（每级别独立运转同一套逻辑：该级别卖点→平多+次级别做空 /
//! 该级别买点→平空+次级别做多，多空双向，无 root，275号局部依赖）。
//!
//! `#[cfg(test)]` + `#[ignore]`（重型 + 依赖 `analysis/data_cache/*.json`）。跑法：
//! ```text
//! cargo test --release recursive_t::t_engine_run -- --ignored --nocapture
//! BT_SYMBOLS=OKLO,CL cargo test --release recursive_t::t_engine_run -- --ignored --nocapture
//! ```
//!
//! 管线：OHLC(json) → `TFugueStreamCore.push_bar`（内部 orchestrator→重跑 T→操作层引擎，
//! 全 Rust 内聚，真流式因果）→ `finish()` → FugueResult。三模式唯一变量 = [`PerfectionMode`]
//! （步骤c 背驰判定），操作层逐字相同 ⟹ 受控实验。
//!
//! ## 认识论等级：**L3**（真实数据，可否证）。strat 偏乐观（确认滞后口径，见 stream.rs 模块头）。

use super::backtest::BacktestMode;
use super::backtest_run::{load_clean_ohlc, SYMBOLS};
use super::stream::TFugueStreamCore;
use super::t_engine::BASE_LADDER;
use crate::fugue_v3::layer::FugueResult;
use crate::fugue_v3::INITIAL_CAPITAL;
use crate::trading::types::MAX_LADDER;
use std::path::PathBuf;

/// 从 equity 采样曲线算最大回撤（%，正数）。
fn max_drawdown(res: &FugueResult) -> f64 {
    let mut peak = f64::MIN;
    let mut dd = 0.0f64;
    for &(_, v) in &res.equity {
        if v > peak {
            peak = v;
        }
        if peak > 0.0 {
            dd = dd.max((peak - v) / peak);
        }
    }
    dd * 100.0
}

/// 单标的汇总行（矩阵打印用）。
struct Row {
    sym: String,
    n_bars: usize,
    bh_pct: f64,
    /// [Structural, And, Or] 的 strat_pct。
    strat: [f64; 3],
    trades: [usize; 3],
    /// 物理多头/空头在场 bar 数（暴露画像）。
    long_bars: [u64; 3],
    short_bars: [u64; 3],
    max_dd: [f64; 3],
    /// **每级别（ladder）独立已实现盈亏**（多尺度滤波器版核心产出：每个 LevelEngine 独立 Z₂
    /// 翻转的累计 cash PnL = `mobile_realized_pnl_by_ladder[ladder]`），三模式各一份。
    pnl_by_ladder: [[f64; MAX_LADDER]; 3],
}

/// f64 → JSON 数字（非有限值 → null）。
fn jf(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.6}")
    } else {
        "null".to_string()
    }
}

/// **逐笔导出**（`BT_DUMP_TRADES=1` 门控；默认汇总契约不变）：序列化 trades(trade11) +
/// equity + 按级别计数器，供 Python 诊断重建净敞口时间线（每 trade row = 一段 chunk 的
/// [entry_bar, exit_bar) × shares × polarity，按极性求和即得任意 bar 的净 long/short units）。
///
/// schema 与 `t_fugue_*.json` 的 11 字段元组一致：
/// `[ladder, entry_bar, entry_price, exit_bar, exit_price, shares, weight_at_entry,
///   deferred_bars, partial, exit_reason, polarity]`。
fn dump_trades_json(res: &FugueResult, sym: &str, mode: &str, n_bars: usize) -> String {
    use crate::trading::types::Polarity;
    let mut s = String::with_capacity(res.trades.len() * 64 + 1024);
    s.push_str(&format!(
        "{{\"symbol\":\"{sym}\",\"mode\":\"{mode}\",\"engine\":\"operation_self_replication\",\
\"n_bars\":{n_bars},\"final_nav\":{},\"n_trades\":{},\
\"trade_schema\":[\"ladder\",\"entry_bar\",\"entry_price\",\"exit_bar\",\"exit_price\",\
\"shares\",\"weight_at_entry\",\"deferred_bars\",\"partial\",\"exit_reason\",\"polarity\",\
\"origin\"],\
\"trades\":[",
        jf(res.final_nav),
        res.trades.len(),
    ));
    for (i, t) in res.trades.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let pol = match t.polarity {
            Polarity::Long => "long",
            Polarity::Short => "short",
        };
        // 第 12 字段：腿出生途径（与 trades 平行；T 翻转引擎填满，其他引擎空 ⟹ fallback "?"）。
        let origin = res.trade_origins.get(i).copied().unwrap_or("?");
        s.push_str(&format!(
            "[{},{},{},{},{},{},{},{},{},\"{}\",\"{}\",\"{}\"]",
            t.ladder,
            t.entry_bar,
            jf(t.entry_price),
            t.exit_bar,
            jf(t.exit_price),
            jf(t.shares),
            jf(t.weight_at_entry),
            t.deferred_bars,
            if t.partial { "true" } else { "false" },
            t.exit_reason,
            pol,
            origin,
        ));
    }
    s.push_str("],\"equity\":[");
    for (i, &(b, v)) in res.equity.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("[{b},{}]", jf(v)));
    }
    // 真实净敞口时间线（bar, long_units, short_units）——magnitude 真值（非 trade 反推）。
    s.push_str("],\"exposure_series\":[");
    for (i, &(b, lu, su)) in res.exposure_series.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("[{b},{},{}]", jf(lu), jf(su)));
    }
    // 按级别诊断计数器（入场/sink/recover/强平）。
    let arr = |a: &[u64]| -> String {
        a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
    };
    s.push_str(&format!(
        "],\"n_entries_by_ladder\":[{}],\"n_cycle_opens_by_ladder\":[{}],\
\"n_cycle_closes_by_ladder\":[{}],\"n_liquidations_by_ladder\":[{}],\
\"phys_long_bars\":{},\"phys_short_bars\":{}}}",
        arr(&res.n_entries_by_ladder),
        arr(&res.n_cycle_opens_by_ladder),
        arr(&res.n_cycle_closes_by_ladder),
        arr(&res.n_liquidations_by_ladder),
        res.phys_long_bars,
        res.phys_short_bars,
    ));
    s
}

#[test]
#[ignore = "重型全量回测，需 analysis/data_cache/*.json；显式 --ignored 运行"]
fn t_engine_8x3() {
    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");

    // 可选标的白名单（逗号分隔，如 `BT_SYMBOLS=OKLO,CL`）；空 = 全部 8 标的。
    let only: Vec<String> = std::env::var("BT_SYMBOLS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
        .unwrap_or_default();

    let mut rows: Vec<Row> = Vec::new();

    for (sym, file) in SYMBOLS {
        if !only.is_empty() && !only.contains(&sym.to_uppercase()) {
            continue;
        }
        let path = data_dir.join(file);
        if !path.exists() {
            eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
            continue;
        }
        let t0 = std::time::Instant::now();
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n_bars = c.len();
        let bh_pct = if n_bars > 0 && c[0] > 0.0 {
            (c[n_bars - 1] / c[0] - 1.0) * 100.0
        } else {
            0.0
        };
        eprintln!("[{sym}] bars={n_bars} ({:.1}s) → 三模式操作层引擎回测…", t0.elapsed().as_secs_f64());

        let mut row = Row {
            sym: sym.to_string(),
            n_bars,
            bh_pct,
            strat: [0.0; 3],
            trades: [0; 3],
            long_bars: [0; 3],
            short_bars: [0; 3],
            max_dd: [0.0; 3],
            pnl_by_ladder: [[0.0; MAX_LADDER]; 3],
        };

        for (mi, mode) in BacktestMode::ALL.iter().enumerate() {
            let tm = std::time::Instant::now();
            let mut core = TFugueStreamCore::new(mode.to_perfection());
            for i in 0..n_bars {
                core.push_bar(o[i], h[i], l[i], c[i]);
            }
            core.finish();
            let res = core.result();
            let strat = (res.final_nav / INITIAL_CAPITAL - 1.0) * 100.0;
            let dd = max_drawdown(res);

            row.strat[mi] = strat;
            row.trades[mi] = res.trades.len();
            row.long_bars[mi] = res.phys_long_bars;
            row.short_bars[mi] = res.phys_short_bars;
            row.max_dd[mi] = dd;
            row.pnl_by_ladder[mi] = res.mobile_realized_pnl_by_ladder;

            // 写汇总 JSON（操作层引擎口径，与 backtest.rs 的简化口径分开命名）。
            let out = data_dir.join(format!("t_engine_{sym}_{}.json", mode.as_str()));
            let json = format!(
                "{{\"symbol\":\"{}\",\"mode\":\"{}\",\"engine\":\"operation_self_replication\",\
\"strat_pct\":{},\"bh_pct\":{},\"n_trades\":{},\"final_nav\":{},\"max_drawdown\":{},\
\"phys_long_bars\":{},\"phys_short_bars\":{},\"max_concurrent_voices\":{},\
\"max_chiral_same_dir\":{},\"pnl_by_ladder\":[{}],\"n_bars\":{}}}",
                sym,
                mode.as_str(),
                jf(strat),
                jf(bh_pct),
                res.trades.len(),
                jf(res.final_nav),
                jf(dd),
                res.phys_long_bars,
                res.phys_short_bars,
                res.max_concurrent_voices,
                res.max_chiral_same_dir,
                res.mobile_realized_pnl_by_ladder
                    .iter()
                    .map(|x| jf(*x))
                    .collect::<Vec<_>>()
                    .join(","),
                n_bars,
            );
            std::fs::write(&out, json).unwrap_or_else(|e| panic!("写 {out:?} 失败: {e}"));

            // 逐笔导出（BT_DUMP_TRADES=1 门控）：供 Python 诊断重建净敞口/churn。
            if std::env::var("BT_DUMP_TRADES").is_ok() {
                let tout = data_dir.join(format!("t_engine_{sym}_{}_trades.json", mode.as_str()));
                let tjson = dump_trades_json(res, sym, mode.as_str(), n_bars);
                std::fs::write(&tout, tjson).unwrap_or_else(|e| panic!("写 {tout:?} 失败: {e}"));
                eprintln!("  [{sym}/{:>10}] 逐笔导出 → {tout:?}", mode.as_str());
            }

            eprintln!(
                "  [{sym}/{:>10}] strat={:+.1}% bh={:+.1}% trades={} long_bars={} short_bars={} mdd={:.1}% ({:.1}s)",
                mode.as_str(),
                strat,
                bh_pct,
                res.trades.len(),
                res.phys_long_bars,
                res.phys_short_bars,
                dd,
                tm.elapsed().as_secs_f64(),
            );
            // 每级别独立盈亏明细（多尺度滤波器各 LevelEngine 的 cash 贡献；只列非零级别）。
            let lvl_pnl: Vec<String> = (BASE_LADDER..MAX_LADDER)
                .filter(|&k| res.mobile_realized_pnl_by_ladder[k].abs() > 1e-6)
                .map(|k| format!("L{}={:+.0}", k - BASE_LADDER, res.mobile_realized_pnl_by_ladder[k]))
                .collect();
            if !lvl_pnl.is_empty() {
                eprintln!("      └ 每级别独立盈亏: {}", lvl_pnl.join("  "));
            }
            if res.n_emergence_upgrades > 0 {
                eprintln!(
                    "      └ 自下而上涌现升级: {} 次（核心仓随 level 涌现 relabel 升级归属，不等高级别 BSP）",
                    res.n_emergence_upgrades
                );
            }
            // 持仓三阶段观测（137号 make-decision-observable）：退本金触发/增股数部署/累计退本金。
            // n_capital_recovered=0 ⇒ 三阶段在该标的有效域为空（退本金从未触发，回测 bit-identical 基线）。
            eprintln!(
                "      └ 三阶段: 退本金={} 增股数部署={}次 加股数={:.1} 部署现金={:.0} 降成本进度峰值={:.3}（=1.0⇒成本归0）短差腿pnl={:.0} 翻转={}",
                res.n_capital_recovered, res.n_earning_deploys, res.earning_units_added,
                res.earning_cash_deployed, res.max_core_gain_x1000 as f64 / 1000.0,
                res.short_leg_pnl, res.n_campaign_resets
            );
        }
        rows.push(row);
    }

    // ── 汇总矩阵 ──
    println!("\n===== T 操作层引擎（每级别自我复制）8 标的 × 3 模式 strat_pct =====");
    println!(
        "{:<6} {:>9} | {:>12} {:>12} {:>12} | {:>10}",
        "标的", "bars", "Structural", "AND", "OR", "BH"
    );
    println!("{}", "-".repeat(78));
    for r in &rows {
        println!(
            "{:<6} {:>9} | {:>+11.1}% {:>+11.1}% {:>+11.1}% | {:>+9.1}%",
            r.sym, r.n_bars, r.strat[0], r.strat[1], r.strat[2], r.bh_pct
        );
    }
    println!("{}", "-".repeat(78));
    println!("{:<6} {:>9} | {:>12} {:>12} {:>12} |", "trades", "", "S", "A", "O");
    for r in &rows {
        println!(
            "{:<6} {:>9} | {:>12} {:>12} {:>12} |",
            r.sym, "", r.trades[0], r.trades[1], r.trades[2]
        );
    }
    println!("{:<6} {:>9} | {:>12} {:>12} {:>12} |", "mdd%", "", "S", "A", "O");
    for r in &rows {
        println!(
            "{:<6} {:>9} | {:>11.1}% {:>11.1}% {:>11.1}% |",
            r.sym, "", r.max_dd[0], r.max_dd[1], r.max_dd[2]
        );
    }
    println!(
        "{:<6} {:>9} | {:>12} {:>12} {:>12} |  (long/short 在场 bar)",
        "expo", "", "S", "A", "O"
    );
    for r in &rows {
        println!(
            "{:<6} {:>9} | {:>5}/{:<6} {:>5}/{:<6} {:>5}/{:<6} |",
            r.sym,
            "",
            r.long_bars[0],
            r.short_bars[0],
            r.long_bars[1],
            r.short_bars[1],
            r.long_bars[2],
            r.short_bars[2],
        );
    }
    println!("================================================================\n");

    // ── 每级别独立盈亏矩阵（多尺度滤波器核心产出：哪个尺度赚/亏一目了然）──
    println!("===== 每级别（ladder）独立已实现盈亏（cash）8 标的 × 3 模式 =====");
    println!(
        "{:<6} {:>5} | {:>13} {:>13} {:>13}",
        "标的", "Lvl", "Structural", "AND", "OR"
    );
    println!("{}", "-".repeat(60));
    for r in &rows {
        let mut any = false;
        for k in BASE_LADDER..MAX_LADDER {
            let p = [r.pnl_by_ladder[0][k], r.pnl_by_ladder[1][k], r.pnl_by_ladder[2][k]];
            if p.iter().all(|x| x.abs() < 1e-6) {
                continue;
            }
            any = true;
            println!(
                "{:<6} {:>5} | {:>+13.0} {:>+13.0} {:>+13.0}",
                r.sym,
                format!("L{}", k - BASE_LADDER),
                p[0],
                p[1],
                p[2]
            );
        }
        if any {
            println!("{}", "·".repeat(60));
        }
    }
    println!("================================================================\n");

    assert!(!rows.is_empty(), "至少跑出一个标的");
}

/// **诊断（信号层结构 dump）**：跑 orchestrator → batch `iterate`，dump T 每个 level 的走势类型
/// （UpTrend/DownTrend/Consolidation，含 completed）+ 中枢数 + BSP kind 分布。回答「核心层
/// （lad6/7=level 3/4）卖点 fire 后同级别有没有产出下跌走势类型」。structural 模式（与
/// trades.json 同口径）；batch 最终塔 = stream 最后一次 rerun 塔（同 segments，纯结构不读 MACD）。
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::t_engine_run::t_level_structure_dump -- --ignored --nocapture`
#[test]
#[ignore = "诊断: dump T per-level 结构（走势类型/中枢/BSP），BT_SYMBOLS 选标的"]
fn t_level_structure_dump() {
    use super::backtest::build_a0_from_segments;
    use super::iterate;
    use super::types::{BSPKind, PerfectionMode, TrendKind};
    use crate::orchestrator::RecursiveOrchestrator;

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");
    let sym = std::env::var("BT_SYMBOLS").unwrap_or_else(|_| "BTC".into());
    let file = SYMBOLS
        .iter()
        .find(|(s, _)| s.eq_ignore_ascii_case(&sym))
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("未知标的 {sym}"));
    let path = data_dir.join(file);
    let (o, h, l, c) = load_clean_ohlc(&path);
    let n = c.len();
    eprintln!("[{sym}] bars={n} 跑 orchestrator…");
    let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
    for i in 0..n {
        orch.process_bar(o[i], h[i], l[i], c[i]);
    }
    let segs = orch.segments();
    let m2r = orch.merged_to_raw();
    let a0 = build_a0_from_segments(segs, m2r, &c);
    eprintln!("[{sym}] confirmed&&settled 段(a0)={} 单元，iterate(structural)…", a0.len());
    let tree = iterate(a0, PerfectionMode::Structural);

    let disc = |k: BSPKind| -> usize {
        match k {
            BSPKind::Type1Buy => 0,
            BSPKind::Type1Sell => 1,
            BSPKind::Type2Buy => 2,
            BSPKind::Type2Sell => 3,
            BSPKind::Type3Buy => 4,
            BSPKind::Type3Sell => 5,
        }
    };
    let names = ["t1buy", "t1sell", "t2buy", "t2sell", "t3buy", "t3sell"];

    eprintln!("\n===== T per-level 结构 dump ({sym}, structural) =====");
    eprintln!(
        "ceiling(最高完整走势级别 r*) = {}  总 level 数 = {}",
        tree.emergent_ceiling(),
        tree.levels.len()
    );
    for (k, lvl) in tree.levels.iter().enumerate() {
        let ladder = k + BASE_LADDER;
        let (mut up, mut down, mut cons) = (0u64, 0u64, 0u64);
        let (mut upc, mut downc, mut consc) = (0u64, 0u64, 0u64);
        for t in &lvl.trends {
            match t.kind {
                TrendKind::UpTrend => {
                    up += 1;
                    if t.completed {
                        upc += 1
                    }
                }
                TrendKind::DownTrend => {
                    down += 1;
                    if t.completed {
                        downc += 1
                    }
                }
                TrendKind::Consolidation => {
                    cons += 1;
                    if t.completed {
                        consc += 1
                    }
                }
            }
        }
        let mut bc = [0u64; 6];
        for b in &lvl.bsps {
            bc[disc(b.kind)] += 1;
        }
        let bsp_str: Vec<String> = (0..6)
            .filter(|&i| bc[i] > 0)
            .map(|i| format!("{}={}", names[i], bc[i]))
            .collect();
        eprintln!(
            "level {k} (lad{ladder}): 走势 ↑Up={up}(完{upc}) ↓Down={down}(完{downc}) ↔Cons={cons}(完{consc}) | 中枢={} | BSP: {}",
            lvl.centers.len(),
            if bsp_str.is_empty() { "无".into() } else { bsp_str.join(" ") }
        );
    }
    eprintln!("================================================\n");
}

/// **诊断（下跌走势背驰逐条件）**：对 lad6/lad5（level 3/2）的每个趋势走势（UpTrend/DownTrend）
/// 重算第37课 5 条件（镜像 `judge_trend_divergence`），打印哪个条件 fail，对比上涨/下跌方向。
/// 回答「为什么下跌走势在 lad6 从未产出 type1_buy」。
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::t_engine_run::t_downtrend_diagnosis -- --ignored --nocapture`
#[test]
#[ignore = "诊断: lad6/5 下跌走势逐条件背驰诊断"]
fn t_downtrend_diagnosis() {
    use super::backtest::build_a0_from_segments;
    use super::divergence::judge_divergence;
    use super::iterate;
    use super::types::{Direction, PerfectionMode, TrendKind, Unit};
    use crate::orchestrator::RecursiveOrchestrator;

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");
    let sym = std::env::var("BT_SYMBOLS").unwrap_or_else(|_| "BTC".into());
    let file = SYMBOLS
        .iter()
        .find(|(s, _)| s.eq_ignore_ascii_case(&sym))
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("未知标的 {sym}"));
    let (o, h, l, c) = load_clean_ohlc(&data_dir.join(file));
    let n = c.len();
    eprintln!("[{sym}] bars={n} 跑 orchestrator + iterate…");
    let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
    for i in 0..n {
        orch.process_bar(o[i], h[i], l[i], c[i]);
    }
    let a0 = build_a0_from_segments(orch.segments(), orch.merged_to_raw(), &c);
    let tree = iterate(a0, PerfectionMode::Structural);

    fn leg_strength(units: &[Unit]) -> f64 {
        let nest: usize = units.iter().map(|u| u.inner_zhongshu_count).sum();
        if nest > 0 {
            nest as f64
        } else {
            let hi = units.iter().map(|u| u.high).fold(f64::MIN, f64::max);
            let lo = units.iter().map(|u| u.low).fold(f64::MAX, f64::min);
            hi - lo
        }
    }
    fn leg_ext(units: &[Unit], dir: Direction) -> f64 {
        match dir {
            Direction::Up => units.iter().map(|u| u.high).fold(f64::MIN, f64::max),
            Direction::Down => units.iter().map(|u| u.low).fold(f64::MAX, f64::min),
        }
    }

    for lvl_idx in [3usize, 2, 1] {
        if lvl_idx >= tree.levels.len() {
            continue;
        }
        let lvl = &tree.levels[lvl_idx];
        eprintln!("\n===== level {lvl_idx} (lad{}) 趋势走势逐条件诊断 =====", lvl_idx + 3);
        for (i, t) in lvl.trends.iter().enumerate() {
            if !matches!(t.kind, TrendKind::UpTrend | TrendKind::DownTrend) {
                continue;
            }
            let nc = t.zhongshus.len();
            let dir = t.direction;
            let actual = judge_divergence(t, PerfectionMode::Structural).map(|b| b.kind);
            if nc < 2 {
                eprintln!("  #{i} {:?} 中枢={nc} → 【条件1 FAIL: 中枢<2】实际={actual:?}", t.kind);
                continue;
            }
            let prev_c = &t.zhongshus[nc - 2];
            let last_c = &t.zhongshus[nc - 1];
            if prev_c.units.is_empty() || last_c.units.is_empty() {
                eprintln!("  #{i} {:?} 中枢空 units → FAIL", t.kind);
                continue;
            }
            let a_end = *prev_c.units.last().unwrap();
            let a_leg = &t.units[0..=a_end];
            let c_start = *last_c.units.last().unwrap() + 1;
            if c_start >= t.units.len() {
                eprintln!(
                    "  #{i} {:?} 中枢={nc} → 【无离开段c FAIL: c_start={c_start}>=len={}】实际={actual:?}",
                    t.kind,
                    t.units.len()
                );
                continue;
            }
            let c_leg = &t.units[c_start..];
            let c_ext = leg_ext(c_leg, dir);
            let prior_ext = leg_ext(&t.units[0..c_start], dir);
            let cond4 = match dir {
                Direction::Up => c_ext > prior_ext,
                Direction::Down => c_ext < prior_ext,
            };
            let has_nest = t.units.iter().any(|u| u.inner_zhongshu_count > 0);
            let (cond2, cond5_cnest, cond3) = if has_nest {
                let c2 = c_leg.iter().any(|u| match dir {
                    Direction::Up => u.low > last_c.high,
                    Direction::Down => u.high < last_c.low,
                });
                let cn: usize = c_leg.iter().map(|u| u.inner_zhongshu_count).sum();
                let b_start = a_end + 1;
                let b_endx = *last_c.units.first().unwrap();
                let bn: usize = if b_start < b_endx {
                    t.units[b_start..b_endx].iter().map(|u| u.inner_zhongshu_count).sum()
                } else {
                    0
                };
                (Some(c2), Some(cn), Some(bn <= cn))
            } else {
                (None, None, None)
            };
            let sc = leg_strength(c_leg);
            let sa = leg_strength(a_leg);
            let decay = sc < sa;
            // 找第一个 fail 的条件
            let fail = if !cond4 {
                "条件4(创新极值)"
            } else if cond2 == Some(false) {
                "条件2(回抽守沿)"
            } else if cond5_cnest.map_or(false, |x| x < 2) {
                "条件5(c段中枢<2)"
            } else if cond3 == Some(false) {
                "条件3(b级别>c)"
            } else if !decay {
                "力度未衰减"
            } else {
                "全过✓"
            };
            eprintln!(
                "  #{i} {:?} 中枢={nc} c段{}单元 | 条件4创新极值={cond4}({c_ext:.0}vs{prior_ext:.0}) has_nest={has_nest} 条件2守沿={cond2:?} 条件5(c_nest)={cond5_cnest:?} 条件3={cond3:?} 力度衰减={decay}(c{sc:.1}/a{sa:.1}) | 首个FAIL={fail} | 实际={actual:?}",
                t.kind, c_leg.len()
            );
        }
    }
}

/// **诊断（全 BSP 列表 dump）**：dump T 全塔每个 BSP 的 `(level, kind, raw_bar, close_price, bsp_price)`
/// 到 CSV，供 Python 在任意 bar 区间（如 BTC 69k→15k 大下跌）筛选买卖点分布。raw_bar 经
/// `merged_to_raw[bsp.bar].1`（raw_end，BSP 可见时点）对齐 closes。structural 口径。
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::t_engine_run::t_bsp_dump -- --ignored --nocapture`
#[test]
#[ignore = "诊断: dump 全 BSP 列表到 CSV（level,kind,raw_bar,close,bsp_price）"]
fn t_bsp_dump() {
    use super::backtest::build_a0_from_segments;
    use super::iterate;
    use super::types::PerfectionMode;
    use crate::orchestrator::RecursiveOrchestrator;

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");
    let sym = std::env::var("BT_SYMBOLS").unwrap_or_else(|_| "BTC".into());
    let file = SYMBOLS
        .iter()
        .find(|(s, _)| s.eq_ignore_ascii_case(&sym))
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("未知标的 {sym}"));
    let (o, h, l, c) = load_clean_ohlc(&data_dir.join(file));
    let n = c.len();
    eprintln!("[{sym}] bars={n} 跑 orchestrator + iterate…");
    let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
    for i in 0..n {
        orch.process_bar(o[i], h[i], l[i], c[i]);
    }
    let m2r = orch.merged_to_raw().to_vec();
    let a0 = build_a0_from_segments(orch.segments(), &m2r, &c);
    let tree = iterate(a0, PerfectionMode::Structural);

    let mut out = String::from("level,kind,raw_bar,close_price,bsp_price\n");
    let mut count = 0usize;
    for lvl in &tree.levels {
        for b in &lvl.bsps {
            let raw_bar = m2r.get(b.bar as usize).map(|&(_, e)| e).unwrap_or(0);
            let close = c.get(raw_bar).copied().unwrap_or(0.0);
            out.push_str(&format!(
                "{},{},{},{:.1},{:.1}\n",
                b.level,
                b.kind.as_str(),
                raw_bar,
                close,
                b.price
            ));
            count += 1;
        }
    }
    let outpath = data_dir.join(format!("t_bsp_dump_{sym}.csv"));
    std::fs::write(&outpath, out).unwrap_or_else(|e| panic!("写 {outpath:?} 失败: {e}"));
    eprintln!("[{sym}] dump {count} 个 BSP → {outpath:?}");
}

/// **诊断（BSP 操作类型）**：跑 `TFugueStreamCore`（stream→t_engine 完整操作层引擎）structural 模式，
/// 统计每个 BSP 信号触发时引擎做的操作（sink 减1/3 / drain 减1/3 / recover 升回 / flip 翻空 / ascend /
/// enter），回答：(1) 卖点是减1/3 还是清全塔翻空？(2) 核心层 units 变化？(3) 翻空在强牛做空亏钱否？
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::t_engine_run::t_bsp_op_diagnosis -- --ignored --nocapture`
#[test]
#[ignore = "诊断: BSP 操作类型统计（sink/drain/flip）+ 翻空强牛做空，需数据"]
fn t_bsp_op_diagnosis() {
    use crate::trading::types::MAX_LADDER;

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");
    let only: Vec<String> = std::env::var("BT_SYMBOLS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
        .unwrap_or_else(|| vec!["BTC".to_string()]);

    for (sym, file) in SYMBOLS {
        if !only.contains(&sym.to_uppercase()) {
            continue;
        }
        let path = data_dir.join(file);
        if !path.exists() {
            eprintln!("[{sym}] 数据缺失，跳过");
            continue;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n_bars = c.len();
        let bh = if n_bars > 0 && c[0] > 0.0 { (c[n_bars - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
        let mut core = TFugueStreamCore::new(BacktestMode::Structural.to_perfection());
        for i in 0..n_bars {
            core.push_bar(o[i], h[i], l[i], c[i]);
        }
        core.finish();
        let res = core.result();
        let d = core.op_diag();
        let strat = (res.final_nav / INITIAL_CAPITAL - 1.0) * 100.0;

        let sells = d.sell_sink + d.sell_drain + d.sell_recover + d.sell_flip + d.sell_ascend + d.sell_enter + d.sell_noop;
        let buys = d.buy_sink + d.buy_drain + d.buy_recover + d.buy_flip + d.buy_ascend + d.buy_enter + d.buy_noop;
        let pct = |x: u64, tot: u64| if tot > 0 { 100.0 * x as f64 / tot as f64 } else { 0.0 };

        eprintln!("\n========== [{sym}] BSP 操作诊断（c段修复后, structural）==========");
        eprintln!("bars={n_bars}  strat={strat:+.1}%  bh={bh:+.1}%  final_nav={:.0}  翻转(campaign_resets)={}", res.final_nav, res.n_campaign_resets);

        // ════ 真空期/敞口诊断（编排者：绝对值敞口,零真空,绩效=Σ|涨跌幅|）════
        let tot = (d.n_vacuum_bars + d.n_long_only_bars + d.n_short_only_bars + d.n_both_bars).max(1);
        let pp = |x: u64| 100.0 * x as f64 / tot as f64;
        eprintln!("\n╔══════ 真空期/敞口诊断（绝对值敞口? 零真空?）══════╗");
        eprintln!("  完全无敞口(真空)   : {:>9} bar ({:>5.2}%)  ★绝对值敞口要求=0", d.n_vacuum_bars, pp(d.n_vacuum_bars));
        eprintln!("  仅多头敞口         : {:>9} bar ({:>5.2}%)", d.n_long_only_bars, pp(d.n_long_only_bars));
        eprintln!("  仅空头敞口         : {:>9} bar ({:>5.2}%)", d.n_short_only_bars, pp(d.n_short_only_bars));
        eprintln!("  多空同时敞口       : {:>9} bar ({:>5.2}%)  ←sink短差(核心多+子级空对冲)", d.n_both_bars, pp(d.n_both_bars));
        let span = if d.first_active_bar >= 0 && d.last_active_bar >= 0 { d.last_active_bar - d.first_active_bar + 1 } else { 0 };
        eprintln!("  首次建仓bar={} 末次有敞口bar={} 跨度={} bar", d.first_active_bar, d.last_active_bar, span);
        eprintln!("  首仓后最长连续真空 = {} bar ({:.2}% 跨度)  {}", d.max_vacuum_gap,
            if span > 0 { 100.0 * d.max_vacuum_gap as f64 / span as f64 } else { 0.0 },
            if d.max_vacuum_gap > (span / 100).max(1) { "★存在长真空=执行断链" } else { "无显著真空" });
        eprintln!("╚════════════════════════════════════════════════╝");
        eprintln!("\n── Q1: 卖点信号({sells}个)触发的操作类型 ──");
        eprintln!("  sink   (子级卖点→父多真减仓1/3+下放做空=正确短差) : {:>7} ({:>5.1}%)", d.sell_sink, pct(d.sell_sink, sells));
        eprintln!("  drain  (子级卖点→减暴露1/3不翻转)              : {:>7} ({:>5.1}%)", d.sell_drain, pct(d.sell_drain, sells));
        eprintln!("  recover(子级卖点→父空平空头短差升回)          : {:>7} ({:>5.1}%)", d.sell_recover, pct(d.sell_recover, sells));
        eprintln!("  flip   (核心卖点→清全塔+翻空) ★过度减仓/翻空   : {:>7} ({:>5.1}%)", d.sell_flip, pct(d.sell_flip, sells));
        eprintln!("  ascend (核心持空,更高卖点骑乘)                : {:>7} ({:>5.1}%)", d.sell_ascend, pct(d.sell_ascend, sells));
        eprintln!("  enter  (全空卖点→首次建空)                    : {:>7} ({:>5.1}%)", d.sell_enter, pct(d.sell_enter, sells));
        eprintln!("  noop   (卖点不动)                            : {:>7} ({:>5.1}%)", d.sell_noop, pct(d.sell_noop, sells));
        eprintln!("\n── Q2: 核心层(最高活跃ladder)units 变化 ──");
        eprintln!("  sink 父级=核心层 (核心被短差直接减1/3) : {} 次, 累计减 {:.4} units", d.n_sink_on_core, d.sink_core_reduced_units);
        eprintln!("  flip 翻空清掉核心多头 (清全塔)         : {} 次, 累计清 {:.4} 多头units", d.flip_from_long, d.flip_long_units_cleared);
        eprintln!("  ⟹ 卖点对核心仓: 减1/3({}次sink-on-core) vs 全清({}次flip-from-long)", d.n_sink_on_core, d.flip_from_long);
        eprintln!("\n── Q3: 翻空是否在强牛中做空亏钱 ──");
        eprintln!("  卖点翻空(原核心持多, 强牛清多翻空) : {} 次", d.flip_from_long);
        eprintln!("  买点翻多(原核心持空)               : {} 次", d.flip_from_short);
        eprintln!("  短差/做空腿累计 realized pnl (short_leg_pnl) : {:+.0}  ⟸ <0 = 做空净亏", res.short_leg_pnl);
        eprintln!("  物理在场 bar: 多头={} 空头={} (空/多比={:.2})", res.phys_long_bars, res.phys_short_bars,
            if res.phys_long_bars > 0 { res.phys_short_bars as f64 / res.phys_long_bars as f64 } else { 0.0 });
        eprintln!("\n── 对照: 买点信号({buys}个) ──");
        eprintln!("  sink={} drain={} recover={} flip={} ascend={} enter={} noop={}",
            d.buy_sink, d.buy_drain, d.buy_recover, d.buy_flip, d.buy_ascend, d.buy_enter, d.buy_noop);

        // ════ 「回来」诊断（编排者：卖是对的，问题在平空+做多）════
        eprintln!("\n╔══════ 「回来」诊断：平空+做多是否到位 ══════╗");
        eprintln!("\n── Q1: sink→recover 间隔（空头短差持仓时长 bar）──");
        let avg_iv = if d.recover_interval_count > 0 { d.recover_interval_sum / d.recover_interval_count as f64 } else { 0.0 };
        eprintln!("  recover 次数={} 平均持空={:.0} bar 最长持空={} bar", d.recover_interval_count, avg_iv, d.recover_interval_max);

        eprintln!("\n── Q2: 信号层密度（买点是否缺失，per ladder）──");
        eprintln!("  {:>4} {:>9} {:>9} {:>7} | {:>9} {:>9} {:>9}", "lad", "卖信号", "买信号", "买/卖", "sink开空", "recover平", "残留空");
        for k in BASE_LADDER..MAX_LADDER {
            let ss = d.sell_sig_by_ladder[k];
            let bs = d.buy_sig_by_ladder[k];
            let opens = res.n_cycle_opens_by_ladder[k];
            let closes = res.n_cycle_closes_by_ladder[k];
            let liq = res.n_liquidations_by_ladder[k];
            if ss + bs + opens + closes == 0 { continue; }
            let resid = opens as i64 - closes as i64 - liq as i64;
            let ratio = if ss > 0 { bs as f64 / ss as f64 } else { 0.0 };
            eprintln!("  L{:<3} {:>9} {:>9} {:>7.2} | {:>9} {:>9} {:>+9}",
                k - BASE_LADDER, ss, bs, ratio, opens, closes, resid);
        }

        eprintln!("\n── Q3: 平空后有没有重新做多 ──");
        eprintln!("  buy_noop_no_short (买点触发但无空可平=平了空就停/不做多回来) : {}", d.buy_noop_no_short);
        eprintln!("  recover (平空头同时升回父级=做多回来, 全量) : 卖侧{} 买侧{} 合计{}", d.sell_recover, d.buy_recover, d.sell_recover + d.buy_recover);
        eprintln!("  ⟹ sink开空合计{} vs recover平空合计{} ⟹ 平空率={:.1}% (缺口={}个空未买回)",
            res.n_cycle_opens_by_ladder.iter().sum::<u64>(),
            res.n_cycle_closes_by_ladder.iter().sum::<u64>(),
            if res.n_cycle_opens_by_ladder.iter().sum::<u64>() > 0 {
                100.0 * res.n_cycle_closes_by_ladder.iter().sum::<u64>() as f64 / res.n_cycle_opens_by_ladder.iter().sum::<u64>() as f64
            } else { 0.0 },
            res.n_cycle_opens_by_ladder.iter().sum::<u64>() as i64 - res.n_cycle_closes_by_ladder.iter().sum::<u64>() as i64);

        eprintln!("\n── Q4: 核心仓 units 在「卖→空→平→多」周期后是否恢复 ──");
        eprintln!("  核心被 sink 减 (累计)   : {:.2} units ({} 次)", d.sink_core_reduced_units, d.n_sink_on_core);
        eprintln!("  核心被 recover 升回(累计): {:.2} units ({} 次)", d.recover_core_restored_units, d.n_recover_on_core);
        let recov_ratio = if d.sink_core_reduced_units > 1e-9 { 100.0 * d.recover_core_restored_units / d.sink_core_reduced_units } else { 0.0 };
        eprintln!("  ⟹ 核心恢复率 = {:.1}% {}", recov_ratio,
            if recov_ratio < 90.0 { "★核心未充分恢复 = 踏空（减多升回少）" } else { "核心基本恢复" });
        eprintln!("╚════════════════════════════════════════════╝");

        // ════ 每笔空头 P&L（编排者：卖点100%对,空头应赚,亏=平空时机错=没在底部平）════
        {
            use crate::trading::types::Polarity;
            let shorts: Vec<&_> = res.trades.iter().filter(|t| t.polarity == Polarity::Short).collect();
            let spnl: Vec<f64> = shorts.iter().map(|t| t.shares * (t.entry_price - t.exit_price)).collect();
            let tot_spnl: f64 = spnl.iter().sum();
            let win = spnl.iter().filter(|&&p| p > 1e-9).count();
            let lose = spnl.iter().filter(|&&p| p < -1e-9).count();
            let n_higher = shorts.iter().filter(|t| t.exit_price > t.entry_price + 1e-9).count();
            // 按 exit_reason 分组空头平仓（recover=正常平/eod=收尾强平/liq=强平/flip=翻转）。
            let by_reason = |r: &str| -> (usize, f64) {
                let mut n = 0; let mut p = 0.0;
                for (i, t) in shorts.iter().enumerate() {
                    if t.exit_reason == r { n += 1; p += spnl[i]; }
                }
                (n, p)
            };
            eprintln!("\n╔══════ 每笔空头 P&L（开空价 vs 平空价，编排者：空头应赚）══════╗");
            eprintln!("  空头交易 {} 笔: 赚{} 亏{} 总pnl={:+.0}", shorts.len(), win, lose, tot_spnl);
            eprintln!("  ★平空价>开空价(价格涨回来才平=做空亏): {} 笔 ({:.1}%)", n_higher, 100.0*n_higher as f64/shorts.len().max(1) as f64);
            for r in ["recover", "eod", "liq", "flip", "clear"] {
                let (n, p) = by_reason(r);
                if n > 0 { eprintln!("    平仓原因 {:>8}: {:>5}笔 pnl={:+.0}", r, n, p); }
            }
            // top 10 亏损空头：开空价→平空价→期间最低价（c[entry..=exit] 的 min）。
            let mut idx: Vec<usize> = (0..shorts.len()).collect();
            idx.sort_by(|&a, &b| spnl[a].partial_cmp(&spnl[b]).unwrap());
            eprintln!("  ── 亏损最大10笔: 开空@→平空@(涨跌%)|期间最低@|本可平@底盈亏|{}bar|原因 ──", "持仓");
            for &i in idx.iter().take(10) {
                let t = shorts[i];
                let (eb, xb) = (t.entry_bar.max(0) as usize, t.exit_bar.max(0) as usize);
                let lo = if eb <= xb && xb < c.len() { c[eb..=xb].iter().cloned().fold(f64::MAX, f64::min) } else { f64::NAN };
                let chg = if t.entry_price > 0.0 { (t.exit_price/t.entry_price - 1.0)*100.0 } else { 0.0 };
                let could = t.shares * (t.entry_price - lo); // 若在期间最低平空的盈利
                eprintln!("    lad{} 开@{:.0}→平@{:.0}({:+.1}%)|最低@{:.0}|底部本可+{:.0}|{}bar|{}|pnl={:+.0}",
                    t.ladder, t.entry_price, t.exit_price, chg, lo, could, t.exit_bar - t.entry_bar, t.exit_reason, spnl[i]);
            }
            eprintln!("╚════════════════════════════════════════════════════════╝");
        }

        eprintln!("\n── 每级别独立已实现盈亏 (cash) ──");
        for k in BASE_LADDER..MAX_LADDER {
            let p = res.mobile_realized_pnl_by_ladder[k];
            if p.abs() > 1e-6 {
                eprintln!("  L{} (lad{k}): {:+.0}", k - BASE_LADDER, p);
            }
        }
        eprintln!("  sink次数/级别(n_cycle_opens): {:?}", &res.n_cycle_opens_by_ladder[BASE_LADDER..]);
        eprintln!("  recover次数/级别(n_cycle_closes): {:?}", &res.n_cycle_closes_by_ladder[BASE_LADDER..]);
        eprintln!("  强平次数/级别(n_liquidations): {:?}", &res.n_liquidations_by_ladder[BASE_LADDER..]);
        eprintln!("================================================================\n");
    }
}
