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

use super::backtest::{
    build_a0_from_segments, build_a0_from_strokes, result_to_json, run_backtest, BacktestMode,
};
use super::types::A0Source;
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

/// 数据文件（容忍两种 schema：parallel-array 或 bars；忽略 symbol/...）。`dates` 保留供
/// 时间窗切片（[`load_clean_ohlc_window`]，1s vs 1min 同时段对照用）。
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
    /// parallel-array schema 的逐 bar 日期串（如 "2025-04-01 00:00:00+00:00"）。bars-schema 无。
    #[serde(default)]
    dates: Vec<String>,
    /// databento ohlcv schema 的逐 bar UTC 纳秒时间戳（如 1743465600000000000）。与 `dates`（字符串
    /// 日期）互斥：databento 1s 导出仅有此列 → [`load_clean_ohlc_window_ns`] 用它做日期窗切片。
    #[serde(default)]
    timestamps_ns: Vec<i64>,
}

/// 加载 + 清洗 OHLC（逐位复刻 `fugue_v2_full_backtest.load_ohlc` 的清洗口径）。
///
/// 1. 删除任一 OHLC 为 nan 或 ≤0 的 bar（污染 PH/MACD + 令价格阈值止损误触发）；
/// 2. spike-and-revert 孤立坏 bar：close 相对前 bar 跳变 >50% 且后 bar 回到前 bar ±5%
///    内 → 数据源坏 tick（真实极端行情为连续跳变，不满足后 bar 回归合取）。
pub(crate) fn load_clean_ohlc(path: &PathBuf) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
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

/// 加载 + 清洗 OHLC，**先按日期范围窗切片**（1s vs 1min 同时段滤波器层数对照用）。
///
/// 仅 parallel-array schema（含 `dates`）支持窗口切片——切片在清洗**前**完成（按原始 index），
/// 然后复用与 [`load_clean_ohlc`] 完全相同的两遍清洗口径（nan/≤0 + spike-and-revert）。
/// 区间判据：`day_start <= dates[i][..10] <= day_end`（ISO 日期串字典序 = 时间序，闭区间）。
/// `day_start`/`day_end` 形如 `"2025-04-01"`/`"2025-04-30"`。窗为空 panic（切片口径错误必须
/// fail-loud，no-patch）。
pub(crate) fn load_clean_ohlc_window(
    path: &PathBuf,
    day_start: &str,
    day_end: &str,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析 {path:?} 失败: {e}"));
    drop(text);
    assert!(
        raw.bars.is_empty(),
        "load_clean_ohlc_window 仅支持 parallel-array schema（{path:?} 是 bars-schema，无 dates 列）"
    );
    assert!(
        !raw.dates.is_empty(),
        "load_clean_ohlc_window 需要 dates 列做时间窗切片（{path:?} 无 dates）"
    );
    assert!(day_start <= day_end, "窗口非法：day_start `{day_start}` > day_end `{day_end}`");
    let nan = f64::NAN;
    let n_raw = raw.closes.len();
    assert_eq!(raw.dates.len(), n_raw, "{path:?} dates 与 closes 长度不一致");

    // 切片（清洗前，按原始 index）：dates 日部分落入 [day_start, day_end] 闭区间。
    let mut o = Vec::new();
    let mut h = Vec::new();
    let mut l = Vec::new();
    let mut c = Vec::new();
    for i in 0..n_raw {
        let day = &raw.dates[i].get(..10).unwrap_or("");
        if *day < day_start || *day > day_end {
            continue;
        }
        let oi = raw.opens.get(i).and_then(|x| *x).unwrap_or(nan);
        let hi = raw.highs.get(i).and_then(|x| *x).unwrap_or(nan);
        let li = raw.lows.get(i).and_then(|x| *x).unwrap_or(nan);
        let ci = raw.closes.get(i).and_then(|x| *x).unwrap_or(nan);
        // 第一遍清洗：nan / ≤0。
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
    assert!(
        !c.is_empty(),
        "时间窗 [{day_start}, {day_end}] 在 {path:?} 内为空（切片口径错误，fail-loud）"
    );

    // 第二遍：spike-and-revert（与 load_clean_ohlc 同口径）。
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

/// `"YYYY-MM-DD"` → UTC 当日 00:00 的 Unix 纳秒（proleptic Gregorian，无 chrono 依赖）。
///
/// days_from_civil 复刻 Howard Hinnant 的 civil→days 算法（公有领域，整数精确，无浮点）。
/// 用于 [`load_clean_ohlc_window_ns`] 把 ISO 日期窗换算成 `timestamps_ns` 的纳秒闭区间端点。
fn iso_day_to_unix_ns(day: &str) -> i64 {
    let parts: Vec<&str> = day.split('-').collect();
    assert_eq!(parts.len(), 3, "日期格式应为 YYYY-MM-DD，得 `{day}`");
    let y: i64 = parts[0].parse().unwrap_or_else(|_| panic!("年解析失败 `{day}`"));
    let m: i64 = parts[1].parse().unwrap_or_else(|_| panic!("月解析失败 `{day}`"));
    let d: i64 = parts[2].parse().unwrap_or_else(|_| panic!("日解析失败 `{day}`"));
    assert!((1..=12).contains(&m), "月越界 `{day}`");
    assert!((1..=31).contains(&d), "日越界 `{day}`");
    // days_from_civil（Hinnant）：返回 1970-01-01 起的天数（可负）。
    let yy = if m <= 2 { y - 1 } else { y };
    let era = if yy >= 0 { yy } else { yy - 399 } / 400;
    let yoe = yy - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    let days = era * 146097 + doe - 719468;
    days * 86_400 * 1_000_000_000
}

/// 加载 + 清洗 OHLC，**先按 ISO 日期范围窗切片（基于 `timestamps_ns` 列）**。
///
/// [`load_clean_ohlc_window`] 的姊妹版：databento ohlcv-1s 导出有 `timestamps_ns`（UTC 纳秒）
/// 而无 `dates` 字符串列 → 用纳秒戳切窗。判据：`start_ns <= timestamps_ns[i] < end_excl_ns`，
/// 其中 `end_excl_ns` = `day_end` **次日** 00:00（即 `day_end` 当天**含**在窗内，闭区间语义与
/// [`load_clean_ohlc_window`] 一致）。切片在清洗前完成（按原始 index），随后复用与
/// [`load_clean_ohlc`] 完全相同的两遍清洗口径。窗为空 panic（fail-loud，no-patch）。
pub(crate) fn load_clean_ohlc_window_ns(
    path: &PathBuf,
    day_start: &str,
    day_end: &str,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析 {path:?} 失败: {e}"));
    drop(text);
    assert!(
        !raw.timestamps_ns.is_empty(),
        "load_clean_ohlc_window_ns 需要 timestamps_ns 列做时间窗切片（{path:?} 无此列）"
    );
    assert!(day_start <= day_end, "窗口非法：day_start `{day_start}` > day_end `{day_end}`");
    let n_raw = raw.closes.len();
    assert_eq!(raw.timestamps_ns.len(), n_raw, "{path:?} timestamps_ns 与 closes 长度不一致");

    let start_ns = iso_day_to_unix_ns(day_start);
    // day_end 含当天 → 上界 = day_end 当天 23:59:59... < 次日 00:00（+1 天纳秒）。
    let end_excl_ns = iso_day_to_unix_ns(day_end) + 86_400 * 1_000_000_000;
    let nan = f64::NAN;

    let mut o = Vec::new();
    let mut h = Vec::new();
    let mut l = Vec::new();
    let mut c = Vec::new();
    for i in 0..n_raw {
        let ts = raw.timestamps_ns[i];
        if ts < start_ns || ts >= end_excl_ns {
            continue;
        }
        let oi = raw.opens.get(i).and_then(|x| *x).unwrap_or(nan);
        let hi = raw.highs.get(i).and_then(|x| *x).unwrap_or(nan);
        let li = raw.lows.get(i).and_then(|x| *x).unwrap_or(nan);
        let ci = raw.closes.get(i).and_then(|x| *x).unwrap_or(nan);
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
    assert!(
        !c.is_empty(),
        "时间窗 [{day_start}, {day_end}] 在 {path:?} 内为空（切片口径错误，fail-loud）"
    );

    // 第二遍：spike-and-revert（与 load_clean_ohlc 同口径）。
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

/// `iso_day_to_unix_ns` 正确性（非 ignored，无数据依赖）：锚点 + 闰年 + 闭区间 end-excl 边界。
/// 锚 1743465600000000000 = databento `cl_1s_databento_1mo.json` 首戳（2025-04-01 00:00 UTC，已对齐核验）。
#[test]
fn iso_day_to_unix_ns_锚点与边界() {
    const NS_DAY: i64 = 86_400 * 1_000_000_000;
    // 锚点（与真实 databento 首戳逐位一致）。
    assert_eq!(iso_day_to_unix_ns("2025-04-01"), 1_743_465_600_000_000_000);
    assert_eq!(iso_day_to_unix_ns("2026-05-29"), 1_780_012_800_000_000_000); // es_1s_2week 首戳
    assert_eq!(iso_day_to_unix_ns("1970-01-01"), 0); // 纪元原点
    // 相邻日恰差 1 天纳秒（end-excl = day_end + 1 天，闭区间语义的算术基础）。
    assert_eq!(iso_day_to_unix_ns("2025-04-02") - iso_day_to_unix_ns("2025-04-01"), NS_DAY);
    // 闰年 2 月：2024-02-29 存在，2024-03-01 = 2024-02-29 + 1 天。
    assert_eq!(iso_day_to_unix_ns("2024-03-01") - iso_day_to_unix_ns("2024-02-29"), NS_DAY);
    // 月/年跨界连续。
    assert_eq!(iso_day_to_unix_ns("2025-01-01") - iso_day_to_unix_ns("2024-12-31"), NS_DAY);
}

/// 8 标的（DEFAULT_SYMS，与 t_vs_v3_comparison.py 一致）→ 数据文件名。
pub(crate) const SYMBOLS: [(&str, &str); 8] = [
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
    // a₀ 来源 A/B 开关（`T_A0=stroke` 切笔底座，526号；默认 segment=线段基线，bit-exact）。
    let a0_source = match std::env::var("T_A0").ok().as_deref() {
        Some("stroke") | Some("bi") => A0Source::Stroke,
        _ => A0Source::Segment,
    };
    eprintln!("[t_backtest_8x3] a₀ 来源 = {a0_source:?}");

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
        let strokes: Vec<_> = orch.strokes().to_vec();
        let m2r: Vec<(usize, usize)> = orch.merged_to_raw().to_vec();
        let n_segs = segs.len();
        eprintln!(
            "[{sym}] bars={n_bars} segs={n_segs} strokes={} ({:.1}s) → 三模式回测…",
            strokes.len(),
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
        // a0_source 决定线段（confirmed&&Settled）/ 笔（confirmed）底座（526号）。
        let a0_base = match a0_source {
            A0Source::Segment => build_a0_from_segments(&segs, &m2r, &c),
            A0Source::Stroke => build_a0_from_strokes(&strokes, &m2r, &c),
        };

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

/// c 段恢复率 + type1 密度诊断（裁决 `docs/c_segment_attribution_reasoning.md` 验证）。
///
/// 逐级别统计：趋势走势数（≥2 中枢）/ 其中有非空离开段 c 的数 / type1 数。
/// c 段判据复刻 `divergence.rs::judge_trend_divergence`：`c_start = 末中枢末单元+1`，
/// `c_start < t.units.len()` ⟺ 离开段非空。
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::backtest_run::t_cseg_diagnostic -- --ignored --nocapture`
#[test]
#[ignore = "c段诊断，需 analysis/data_cache/*.json；显式 --ignored 运行"]
fn t_cseg_diagnostic() {
    use crate::recursive_t::iterate;
    use crate::recursive_t::types::{BSPKind, PerfectionMode, TrendKind};

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("analysis/data_cache");

    // 默认只跑 BTC（裁决标的）；BT_SYMBOLS 覆盖。
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
            eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
            continue;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n_bars = c.len();
        let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
        for i in 0..n_bars {
            orch.process_bar(o[i], h[i], l[i], c[i]);
        }
        let segs: Vec<_> = orch.segments().to_vec();
        let m2r: Vec<(usize, usize)> = orch.merged_to_raw().to_vec();
        let a0 = build_a0_from_segments(&segs, &m2r, &c);
        let tree = iterate(a0, PerfectionMode::Structural);

        println!("\n===== [{sym}] c段恢复 + type1 密度（Structural）=====");
        println!("bars={n_bars} segs={} r*={}", segs.len(), tree.emergent_ceiling());
        println!(
            "{:>5} {:>10} {:>12} {:>10} {:>10} {:>8} {:>8}",
            "level", "n_trend≥2", "n_有c段", "c段率%", "n_type1", "t1_buy", "t1_sell"
        );
        let (mut tot_trend, mut tot_c, mut tot_t1) = (0usize, 0usize, 0usize);
        for lvl in &tree.levels {
            let mut n_trend = 0usize;
            let mut n_c = 0usize;
            for t in &lvl.trends {
                if matches!(t.kind, TrendKind::UpTrend | TrendKind::DownTrend) && t.zhongshus.len() >= 2 {
                    n_trend += 1;
                    let last_c = t.zhongshus.last().unwrap();
                    if let Some(&end) = last_c.units.last() {
                        if end + 1 < t.units.len() {
                            n_c += 1;
                        }
                    }
                }
            }
            let n_t1 = lvl.bsps.iter().filter(|b| matches!(b.kind, BSPKind::Type1Buy | BSPKind::Type1Sell)).count();
            let n_t1b = lvl.bsps.iter().filter(|b| b.kind == BSPKind::Type1Buy).count();
            let n_t1s = lvl.bsps.iter().filter(|b| b.kind == BSPKind::Type1Sell).count();
            let rate = if n_trend > 0 { 100.0 * n_c as f64 / n_trend as f64 } else { 0.0 };
            println!(
                "{:>5} {:>10} {:>12} {:>9.1}% {:>10} {:>8} {:>8}",
                lvl.level, n_trend, n_c, rate, n_t1, n_t1b, n_t1s
            );
            tot_trend += n_trend;
            tot_c += n_c;
            tot_t1 += n_t1;
        }
        let tot_rate = if tot_trend > 0 { 100.0 * tot_c as f64 / tot_trend as f64 } else { 0.0 };
        println!(
            "{:>5} {:>10} {:>12} {:>9.1}% {:>10}",
            "合计", tot_trend, tot_c, tot_rate, tot_t1
        );
        println!("c段缺失率 = {:.1}%（趋势走势中无离开段的占比）", 100.0 - tot_rate);
    }
}

/// 盘整走势（ConsolDown）"无离开段"根因诊断（编排者 2026-06-21）。
///
/// 复刻 `judge_consolidation_divergence` 的 no_leave 判据（`enter_end+1 >= len`），逐级别统计
/// 盘整走势的：总数 / no_leave 数 / completed 数 / completed&&无bsp（被反向终结）数 /
/// 是否末组（gi+1==n_groups）。并 dump 每级别第一个 no_leave 盘整的完整结构（units 长度、
/// 中枢 units 范围、进入/离开段范围）——回答「L4 ConsolDown 全 no_leave 是 r* 边界生长中走势
/// 还是 segment 切分 bug」。
///
/// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::backtest_run::t_consol_no_leave_diagnostic -- --ignored --nocapture`
#[test]
#[ignore = "盘整无离开段诊断，需 analysis/data_cache/*.json；显式 --ignored 运行"]
fn t_consol_no_leave_diagnostic() {
    use crate::recursive_t::iterate;
    use crate::recursive_t::types::{Direction, PerfectionMode, TrendKind};

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
            eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
            continue;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n_bars = c.len();
        let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
        for i in 0..n_bars {
            orch.process_bar(o[i], h[i], l[i], c[i]);
        }
        let segs: Vec<_> = orch.segments().to_vec();
        let m2r: Vec<(usize, usize)> = orch.merged_to_raw().to_vec();
        let a0 = build_a0_from_segments(&segs, &m2r, &c);
        let tree = iterate(a0, PerfectionMode::Structural);

        println!("\n===== [{sym}] 盘整无离开段诊断（batch 最终态，Structural）r*={} =====", tree.emergent_ceiling());
        println!(
            "{:>5} {:>8} {:>9} {:>10} {:>14} {:>9} {:>9}",
            "level", "n_consol", "consolDn", "no_leave", "compl&&无bsp", "末组数", "中间数"
        );
        for lvl in &tree.levels {
            let n_groups = lvl.trends.len();
            let (mut n_consol, mut n_cd, mut n_no_leave) = (0usize, 0usize, 0usize);
            let (mut n_compl_no_bsp, mut n_last, mut n_mid_no_leave) = (0usize, 0usize, 0usize);
            let mut first_dump: Option<String> = None;
            for (gi, t) in lvl.trends.iter().enumerate() {
                if t.kind != TrendKind::Consolidation {
                    continue;
                }
                n_consol += 1;
                if t.direction != Direction::Down {
                    continue;
                }
                n_cd += 1;
                // 复刻 judge_consolidation_divergence：单中枢 + 非空 + enter_end+1>=len。
                if t.zhongshus.len() != 1 {
                    continue;
                }
                let center = &t.zhongshus[0];
                if center.units.is_empty() {
                    continue;
                }
                let enter_end = *center.units.last().unwrap();
                let no_leave = enter_end + 1 >= t.units.len();
                let is_last = gi + 1 == n_groups;
                if no_leave {
                    n_no_leave += 1;
                    if t.completed && t.bsp.is_none() {
                        n_compl_no_bsp += 1;
                    }
                    if is_last {
                        n_last += 1;
                    } else {
                        n_mid_no_leave += 1;
                    }
                    if first_dump.is_none() {
                        first_dump = Some(format!(
                            "units.len={} 中枢units={:?}(first={} last={}) 进入段[0..={}] 离开段[{}..{}](空={}) \
                             completed={} has_bsp={} gi={}/{} 末组={}",
                            t.units.len(),
                            center.units,
                            center.units.first().unwrap(),
                            enter_end,
                            enter_end,
                            enter_end + 1,
                            t.units.len(),
                            enter_end + 1 >= t.units.len(),
                            t.completed,
                            t.bsp.is_some(),
                            gi,
                            n_groups,
                            is_last,
                        ));
                    }
                }
            }
            println!(
                "{:>5} {:>8} {:>9} {:>10} {:>14} {:>9} {:>9}",
                lvl.level, n_consol, n_cd, n_no_leave, n_compl_no_bsp, n_last, n_mid_no_leave
            );
            if lvl.level >= 3 {
                if let Some(d) = first_dump {
                    println!("    L{} 首个 no_leave 盘整: {}", lvl.level, d);
                }
            }
        }
        println!("说明：'中间数'>0 ⟺ 存在中间盘整无离开段（segment bug）；'compl&&无bsp'>0 ⟺ 被反向终结标 completed。");
    }
}
