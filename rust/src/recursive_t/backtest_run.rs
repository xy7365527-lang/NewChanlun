//! T 回测全量 runner：8 标的 × 3 模式（Structural/AND/OR）对照。
//!
//! `#[cfg(test)]` + `#[ignore]`（重型 + 依赖 `analysis/data_cache/*.json`，不在常规
//! `cargo test` 跑）。跑法：
//! ```text
//! cargo test --release recursive_t::backtest_run -- --ignored --nocapture
//! ```
//!
//! 管线：OHLC(json) → `RecursiveOrchestrator`(产 a₀ 线段) → `build_a0_from_segments`
//! (注入 MACD 面积) → `run_backtest`(三模式) → 写 JSON + 打印矩阵。
//!
//! orchestrator 配置 `enable_macd=false, enable_bsp=false`：segments 输出**不依赖**这两
//! 个 flag（macd/bsp 是下游层），关闭 ⇒ 跳过 O(n²) bsp 重算（纯省，segments bit-exact）。
//! 笔/段参数（wide / min_strict_sep=5 / new_raw_gap_min=3）与 `t_vs_v3_comparison.py`
//! 的 PyO3 默认一致 ⇒ a₀ 同源。

use super::backtest::{build_a0_from_segments, result_to_json, run_backtest, BacktestMode};
use crate::orchestrator::RecursiveOrchestrator;
use serde::Deserialize;
use std::path::PathBuf;

/// bars-schema 的单根 K 线（忽略 ts 等其余字段）。
///
/// 字段用 `Option<f64>`：Python `json.dumps(allow_nan=True)` 写出的 `NaN`/`Infinity`
/// 经预处理替换为 `null`（见 [`load_clean_ohlc`]）→ `None` → 清洗阶段当缺失值删除。
#[derive(Deserialize, Default)]
struct RawBar {
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
}

/// 数据文件（容忍两种 schema：parallel-array 或 bars；忽略 dates/symbol/...）。
#[derive(Deserialize, Default)]
struct RawData {
    #[serde(default)]
    opens: Vec<Option<f64>>,
    #[serde(default)]
    highs: Vec<Option<f64>>,
    #[serde(default)]
    lows: Vec<Option<f64>>,
    #[serde(default)]
    closes: Vec<Option<f64>>,
    #[serde(default)]
    bars: Vec<RawBar>,
}

/// 加载 + 清洗 OHLC（逐位复刻 `fugue_v2_full_backtest.load_ohlc` 的清洗口径）。
///
/// 1. 删除任一 OHLC 为 nan 或 ≤0 的 bar（污染 PH/MACD + 令价格阈值止损误触发）；
/// 2. spike-and-revert 孤立坏 bar：close 相对前 bar 跳变 >50% 且后 bar 回到前 bar ±5%
///    内 → 数据源坏 tick（真实极端行情为连续跳变，不满足后 bar 回归合取）。
fn load_clean_ohlc(path: &PathBuf) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    // Python json(allow_nan) 写出非标准字面量 NaN/Infinity（serde_json 硬拒）→ null。
    // 先 -Infinity 再 Infinity（前者是后者超集，避免残留 "-null"），最后 NaN。
    // 仅出现在数值区（symbol="DX" / 时间戳字符串不含这些 token），替换安全。
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析 {path:?} 失败: {e}"));
    drop(text);

    let nan = f64::NAN;
    // 统一为 parallel-array（bars-schema 拆列）；None（缺失 / NaN / Inf）→ NaN，下方清洗删除。
    let (o_in, h_in, l_in, c_in): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = if !raw.bars.is_empty() {
        (
            raw.bars.iter().map(|b| b.open.unwrap_or(nan)).collect(),
            raw.bars.iter().map(|b| b.high.unwrap_or(nan)).collect(),
            raw.bars.iter().map(|b| b.low.unwrap_or(nan)).collect(),
            raw.bars.iter().map(|b| b.close.unwrap_or(nan)).collect(),
        )
    } else {
        (
            raw.opens.iter().map(|x| x.unwrap_or(nan)).collect(),
            raw.highs.iter().map(|x| x.unwrap_or(nan)).collect(),
            raw.lows.iter().map(|x| x.unwrap_or(nan)).collect(),
            raw.closes.iter().map(|x| x.unwrap_or(nan)).collect(),
        )
    };

    // 第一遍：nan / ≤0 清洗。
    let mut o = Vec::with_capacity(c_in.len());
    let mut h = Vec::with_capacity(c_in.len());
    let mut l = Vec::with_capacity(c_in.len());
    let mut c = Vec::with_capacity(c_in.len());
    for i in 0..c_in.len() {
        let (oi, hi, li, ci) = (o_in[i], h_in[i], l_in[i], c_in[i]);
        if oi.is_nan() || hi.is_nan() || li.is_nan() || ci.is_nan() {
            continue;
        }
        if oi <= 0.0 || hi <= 0.0 || li <= 0.0 || ci <= 0.0 {
            continue;
        }
        o.push(oi);
        h.push(hi);
        l.push(li);
        c.push(ci);
    }

    // 第二遍：spike-and-revert（孤立坏 tick）。
    let n = c.len();
    let mut drop = vec![false; n];
    for i in 1..n.saturating_sub(1) {
        if (c[i] / c[i - 1] - 1.0).abs() > 0.5 && (c[i + 1] / c[i - 1] - 1.0).abs() < 0.05 {
            drop[i] = true;
        }
    }
    if drop.iter().any(|&d| d) {
        let keep = |v: &[f64]| -> Vec<f64> {
            v.iter().enumerate().filter(|(i, _)| !drop[*i]).map(|(_, &x)| x).collect()
        };
        (keep(&o), keep(&h), keep(&l), keep(&c))
    } else {
        (o, h, l, c)
    }
}

/// 8 标的（DEFAULT_SYMS，与 t_vs_v3_comparison.py 一致）→ 数据文件名。
const SYMBOLS: [(&str, &str); 8] = [
    ("CL", "cl_1m_databento_10y.json"),
    ("BRN", "brn_1m_databento_10y.json"),
    ("DX", "dx_1m_databento_10y.json"),
    ("GC", "gc_1m_databento_10y.json"),
    ("ES", "es_1m_databento_10y.json"),
    ("QQQ", "qqq_1m_databento_full.json"),
    ("BTC", "btc_1m_full.json"),
    ("OKLO", "oklo_1m_databento.json"),
];

/// 单标的汇总行（矩阵打印用）。
struct Row {
    sym: String,
    n_bars: usize,
    n_segs: usize,
    bh_pct: f64,
    /// [Structural, And, Or] 的 strat_pct。
    strat: [f64; 3],
    /// [Structural, And, Or] 的 n_trades。
    trades: [usize; 3],
    ceiling: usize,
}

#[test]
#[ignore = "重型全量回测，需 analysis/data_cache/*.json；显式 --ignored 运行"]
fn t_backtest_8x3() {
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

        // a₀ 线段（关 macd/bsp，segments bit-exact + 加速）。
        let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
        for i in 0..n_bars {
            orch.process_bar(o[i], h[i], l[i], c[i]);
        }
        let segs: Vec<_> = orch.segments().to_vec();
        let m2r: Vec<(usize, usize)> = orch.merged_to_raw().to_vec();
        let n_segs = segs.len();
        eprintln!(
            "[{sym}] bars={n_bars} segs={n_segs} ({:.1}s) → 三模式回测…",
            t0.elapsed().as_secs_f64()
        );

        let mut row = Row {
            sym: sym.to_string(),
            n_bars,
            n_segs,
            bh_pct: 0.0,
            strat: [0.0; 3],
            trades: [0; 3],
            ceiling: 0,
        };

        // a₀ 对三模式**完全相同**（PerfectionMode 只影响步骤c/iterate，不影响 a₀ 构造）
        // ⇒ 构造一次、clone 给每模式。这是「受控实验」前提的代码体现，且省 3× MACD 重算。
        let a0_base = build_a0_from_segments(&segs, &m2r, &c);

        for (mi, mode) in BacktestMode::ALL.iter().enumerate() {
            let res = run_backtest(a0_base.clone(), *mode, &c, &m2r, n_segs);
            // 写 JSON。
            let out = data_dir.join(format!("t_backtest_{sym}_{}.json", mode.as_str()));
            std::fs::write(&out, result_to_json(sym, &res)).unwrap_or_else(|e| panic!("写 {out:?} 失败: {e}"));
            row.bh_pct = res.summary.bh_pct;
            row.strat[mi] = res.summary.strat_pct;
            row.trades[mi] = res.summary.n_trades;
            row.ceiling = res.summary.ceiling;
            eprintln!(
                "  [{sym}/{:>10}] strat={:+.1}% bh={:+.1}% trades={} bsps={} r*={}",
                mode.as_str(),
                res.summary.strat_pct,
                res.summary.bh_pct,
                res.summary.n_trades,
                res.summary.n_bsps,
                res.summary.ceiling,
            );
        }
        rows.push(row);
    }

    // ── 汇总矩阵 ──
    println!("\n========== T 回测 8 标的 × 3 模式 strat_pct 矩阵 ==========");
    println!(
        "{:<6} {:>8} {:>7} {:>4} | {:>12} {:>12} {:>12} | {:>10}",
        "标的", "bars", "segs", "r*", "Structural", "AND", "OR", "BH"
    );
    println!("{}", "-".repeat(86));
    for r in &rows {
        println!(
            "{:<6} {:>8} {:>7} {:>4} | {:>+11.1}% {:>+11.1}% {:>+11.1}% | {:>+9.1}%",
            r.sym, r.n_bars, r.n_segs, r.ceiling, r.strat[0], r.strat[1], r.strat[2], r.bh_pct
        );
    }
    println!("{}", "-".repeat(86));
    println!(
        "{:<6} {:>8} {:>7} {:>4} | {:>12} {:>12} {:>12} |",
        "n_trades", "", "", "", "S", "A", "O"
    );
    for r in &rows {
        println!(
            "{:<6} {:>8} {:>7} {:>4} | {:>12} {:>12} {:>12} |",
            r.sym, "", "", "", r.trades[0], r.trades[1], r.trades[2]
        );
    }
    println!("===========================================================\n");

    assert!(!rows.is_empty(), "至少跑出一个标的");
}
