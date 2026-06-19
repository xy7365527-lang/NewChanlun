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
use crate::fugue_v3::layer::FugueResult;
use crate::fugue_v3::INITIAL_CAPITAL;
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
}

/// f64 → JSON 数字（非有限值 → null）。
fn jf(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.6}")
    } else {
        "null".to_string()
    }
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

            // 写汇总 JSON（操作层引擎口径，与 backtest.rs 的简化口径分开命名）。
            let out = data_dir.join(format!("t_engine_{sym}_{}.json", mode.as_str()));
            let json = format!(
                "{{\"symbol\":\"{}\",\"mode\":\"{}\",\"engine\":\"operation_self_replication\",\
\"strat_pct\":{},\"bh_pct\":{},\"n_trades\":{},\"final_nav\":{},\"max_drawdown\":{},\
\"phys_long_bars\":{},\"phys_short_bars\":{},\"max_concurrent_voices\":{},\
\"max_chiral_same_dir\":{},\"n_bars\":{}}}",
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
                n_bars,
            );
            std::fs::write(&out, json).unwrap_or_else(|e| panic!("写 {out:?} 失败: {e}"));

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

    assert!(!rows.is_empty(), "至少跑出一个标的");
}
