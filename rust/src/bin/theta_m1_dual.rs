//! # `theta_m1_dual` —— M1 最终实验：生产引擎 7 期货回放 + 预注册 7/7 判据（#1317）。
//!
//! #1279 C 组收口：**M1 最终 alpha 读数来自生产引擎本体**——替代已退役的 analysis 简化代理
//! （#1309/#1312 的 `fugue_alpha_diagnosis.py` + `p3_random_gate_control`），其 1/7 读数仅作
//! 参考基线，不归因于缠论体系。
//!
//! ## 三件事
//!
//! 1. **生产 runner 路径回放**：`run_theta_v0_pi`（七链 π_Θ 生产 runner，`theta_replay`/
//!    `theta_accept` 共核的生产决策面）在 7 期货（ES/GC/CL/ZN/6E/BRN/DX）全量 1min 上跑批。
//!    判据面 = θ_v0 生产路径（三类买卖点 confirm-bar 部署 + nest 门/χ 门/风控门/候选过滤）。
//!    ⚠ 边界照实：`trading::run_organic`（FLAT/ARMED/LONG 分派）的 41课门/fatigue门 是
//!    **tape-driven 的独立分派层**（消费 `organic_signals.py` 产出的 SignalTape，非本 bin 的
//!    OHLC 数据路径；#1312 已建门列导出基建）——若「完整判据面」须含该层，属后续集成决定，
//!    不在本 bin 数据路径内。
//! 2. **同预注册判据测量**（#1282 冻结，跑前不改一字）：7/7 标的百分位全部越过各自随机中位
//!    （符号检验单侧 p=0.0078125）——`backtest::m1_dual::large_bootstrap_percentile_dual`
//!    与 Python 冻结函数 `p3_random_gate_control.large_bootstrap_percentile_dual` **逐位同构
//!    （含 MT19937 PRNG）**；前后对照 vs #1309 归档的简化代理双开百分位（归因差距）；
//! 3. **双跑逐位自检**：每标的 `replay_dump::run_replay_double`（同输入双跑逐位一致）。
//!
//! 结果原样入档 `.chanlun/review-results/issue1317-prod-engine-backtest.{md,json}`——
//! **过与不过都是 M1 有效结论**。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin theta_m1_dual -- [--out DIR] [SYMBOL...]
//!   SYMBOL...   ES/GC/CL/ZN/6E/BRN/DX（省略 = #1282 冻结表序 7 标的）
//! ```
//!
//! ## 外部数据依赖（卡点即报，见 main）
//!
//! 数据文件 `analysis/data_cache/{es,gc,cl,zn,usd6e,brn,dx}_1m_databento_10y.json` 被 gitignore、
//! 需 Databento 订阅 key 拉取。缺数据时本 bin fail-loud 列出缺文件 + 解除条件 + 续跑命令，
//! 退出码 2（不空转、不硬凑，#1066）。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use newchan_rust::theta_v0::backtest::data::{bars_per_year, load_by_symbol};
use newchan_rust::theta_v0::backtest::m1_dual::{
    dual_leg_metrics, large_bootstrap_percentile_dual, Percentile, MIN_N_FOR_P3, N_BOOTSTRAP_LARGE,
};
use newchan_rust::theta_v0::backtest::replay_dump::{initial_nav_for, run_replay_double};
use newchan_rust::theta_v0::backtest::runner::run_theta_v0_pi;
use newchan_rust::theta_v0::config::ThetaConfig;

/// #1282 冻结表序 7 标的。
const SYMBOL_ORDER: [&str; 7] = ["ES", "GC", "CL", "ZN", "6E", "BRN", "DX"];

/// 7/7 符号检验单侧 p（#1282 冻结）。
const SIGN_TEST_P_7OF7: f64 = 0.0078125; // = 1/128

/// 第一轮简化代理（#1309，analysis 代理）归档双开百分位——**仅作参考基线**（票面明示
/// 不作为本实验预期值），用于前后对照逐标的 Δ（归因差距）。
const ARCHIVED_SIMPLIFIED_DUAL_PCT: [(&str, f64); 7] = [
    ("ES", 31.0),
    ("GC", 37.0),
    ("CL", 42.0),
    ("ZN", 66.0),
    ("6E", 45.0),
    ("BRN", 40.0),
    ("DX", 43.0),
];

fn archived_pct(symbol: &str) -> f64 {
    ARCHIVED_SIMPLIFIED_DUAL_PCT
        .iter()
        .find(|(s, _)| *s == symbol)
        .map(|(_, p)| *p)
        .unwrap_or(f64::NAN)
}

/// 单标的测量结果。
#[derive(Debug, Clone)]
struct SymbolResult {
    symbol: String,
    n_bars: usize,
    untradable_ratio: f64,
    bh: f64,
    n_orders: usize,
    strat_return_mtm: f64,
    dual: DualLegReport,
    percentile: Option<Percentile>,
    beats_median: Option<bool>,
    replay_identical: bool,
    replay_elapsed_s: f64,
    elapsed_s: f64,
}

#[derive(Debug, Clone)]
struct DualLegReport {
    n_long: usize,
    n_short: usize,
    hold_long: f64,
    hold_short: f64,
    long_compound: f64,
    short_compound: f64,
    combined_compound: f64,
}

/// 单标的跑批（生产 runner + 双开百分位 + 双跑自检）。`Err` = 数据加载失败/空窗（卡点）。
fn run_symbol(symbol: &str, config: &ThetaConfig) -> Result<SymbolResult, String> {
    let started = Instant::now();
    let full = load_by_symbol(symbol, config)?;
    if full.bars.is_empty() {
        return Err(format!(
            "品种 {symbol} 数据集为空（无 bar）——无法回放，判 inconclusive"
        ));
    }

    let years = (full.bars.len() as f64 / bars_per_year(full.bar_seconds)).max(1e-9);
    let initial_nav = initial_nav_for(&full, config);

    // ── 生产 runner（七链 π_Θ + 完整判据面）。 ──
    let r = run_theta_v0_pi(&full, config, years, initial_nav);

    // ── 双开腿口径 + 预注册百分位（#1282 判据，#1309 同口径）。 ──
    let legs = dual_leg_metrics(&r.prices, &r.trades);
    let n_total = legs.n_long + legs.n_short;
    let (percentile, beats_median) = if n_total >= MIN_N_FOR_P3 {
        let p = large_bootstrap_percentile_dual(
            &r.prices,
            legs.n_long,
            legs.hold_long.round() as usize,
            legs.n_short,
            legs.hold_short.round() as usize,
            legs.combined_compound,
            N_BOOTSTRAP_LARGE,
        );
        let beat = p.e_percentile > 50.0;
        (Some(p), Some(beat))
    } else {
        (None, None)
    };

    // ── 双跑逐位自检（theta_replay 同款确定性自证）。 ──
    let (replay_identical, replay_elapsed_s) = {
        let rep = run_replay_double(&full, config, None);
        (rep.identical, rep.elapsed.as_secs_f64())
    };

    let first = r.prices.first().copied().unwrap_or(0.0);
    let last = r.prices.last().copied().unwrap_or(0.0);
    let bh = if first > 0.0 {
        (last - first) / first * 100.0
    } else {
        0.0
    };

    Ok(SymbolResult {
        symbol: symbol.to_string(),
        n_bars: r.n_bars,
        untradable_ratio: r.untradable_ratio,
        bh,
        n_orders: r.n_orders,
        strat_return_mtm: r.metrics.strat_return,
        dual: DualLegReport {
            n_long: legs.n_long,
            n_short: legs.n_short,
            hold_long: legs.hold_long,
            hold_short: legs.hold_short,
            long_compound: legs.long_compound,
            short_compound: legs.short_compound,
            combined_compound: legs.combined_compound,
        },
        percentile,
        beats_median,
        replay_identical,
        replay_elapsed_s,
        elapsed_s: started.elapsed().as_secs_f64(),
    })
}

fn sign_test_p(n_pass: usize, n_symbols: usize) -> f64 {
    if n_pass > n_symbols {
        return 0.0;
    }
    // 组合数（n ≤ 7，u64 足够）。
    let binom = |n: u64, k: u64| -> u64 {
        if k > n {
            return 0;
        }
        let k = k.min(n - k);
        let mut num = 1u64;
        let mut den = 1u64;
        for i in 0..k {
            num *= n - i;
            den *= i + 1;
        }
        num / den
    };
    let total: u64 = (n_pass..=n_symbols)
        .map(|k| binom(n_symbols as u64, k as u64))
        .sum();
    total as f64 / 2f64.powi(n_symbols as i32)
}

fn write_report(
    results: &[SymbolResult],
    failures: &[(String, String)],
    out_dir: &Path,
) -> std::io::Result<()> {
    let n_pass = results
        .iter()
        .filter(|r| r.beats_median == Some(true))
        .count();
    let n_total = results.len();
    let verdict = if n_total == 0 {
        "卡点（数据缺失，未执行）"
    } else if n_pass == n_total {
        "过（7/7 全部越过随机中位）"
    } else {
        "不过（如实入档）"
    };

    let mut md = String::new();
    let _ = writeln!(
        md,
        "# #1317 M1 最终实验：生产引擎 7 期货回放 + 预注册 7/7 判据结果\n"
    );
    let _ = writeln!(
        md,
        "> 7 标的（ES/GC/CL/ZN/6E/BRN/DX）1min 全量（prereg 表冻结值）| 生产 runner 路径 "
    );
    let _ = writeln!(
        md,
        "> （`run_theta_v0_pi` 七链 π_Θ + 完整判据面）| 零交易成本 | 过判据 = 7/7 百分位全部越过各自随机中位"
    );
    let _ = writeln!(md, "> （符号检验单侧 p=0.0078）。第一轮简化代理（#1309）1/7 仅作参考基线，不作为本实验预期值。\n");

    if !failures.is_empty() {
        let _ = writeln!(md, "## ⚠ 卡点（外部数据依赖，缺文件不参与判定）\n");
        for (sym, e) in failures {
            let _ = writeln!(md, "- `{sym}`：{e}");
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "## 0. 判决\n");
    if n_total == 0 {
        let _ = writeln!(
            md,
            "**未执行**（全部标的缺数据，见上方卡点）→ 判定：**{verdict}**。\n"
        );
    } else {
        let _ = writeln!(
            md,
            "**{n_pass}/{n_total} 标的百分位越过随机中位** → 判定：**{verdict}**。"
        );
        let _ = writeln!(
            md,
            "（7/7 符号检验单侧 p = {SIGN_TEST_P_7OF7:.4}；本次实际命中 {n_pass}/{n_total}，"
        );
        let _ = writeln!(md, "单侧 p = {:.4}）\n", sign_test_p(n_pass, n_total));
    }

    if !results.is_empty() {
        let _ = writeln!(md, "## 1. 生产引擎双开百分位（主判据）\n");
        let _ = writeln!(
            md,
            "| 标的 | Bars | 生产复利% | 长腿复利% | 空腿复利% | 交易(长/空) | P(随机≥真实) | 百分位 | 随机中位% | 越过中位 |\n"
        );
        let _ = writeln!(
            md,
            "|------|------|----------|-----------|-----------|-------------|-------------|--------|----------|----------|"
        );
        for r in results {
            let d = &r.dual;
            match &r.percentile {
                None => {
                    let _ = writeln!(
                        md,
                        "| {} | {} | {:.2} | {:.2} | {:.2} | {}/{} | — | — | — | N<{MIN_N_FOR_P3} 跳过 |",
                        r.symbol, r.n_bars, d.combined_compound, d.long_compound, d.short_compound,
                        d.n_long, d.n_short
                    );
                }
                Some(p) => {
                    let beat = if r.beats_median == Some(true) {
                        "✅"
                    } else {
                        "❌"
                    };
                    let _ = writeln!(
                        md,
                        "| {} | {} | {:.2} | {:.2} | {:.2} | {}/{} | {:.1}% | {:.0} | {:.2} | {} |",
                        r.symbol,
                        r.n_bars,
                        d.combined_compound,
                        d.long_compound,
                        d.short_compound,
                        d.n_long,
                        d.n_short,
                        p.p_random_ge_e * 100.0,
                        p.e_percentile,
                        p.rnd_median,
                        beat
                    );
                }
            }
        }

        let _ = writeln!(
            md,
            "\n## 2. 前后对照（生产引擎 vs 第一轮简化代理，逐标的 Δ 归因差距）\n"
        );
        let _ = writeln!(
            md,
            "| 标的 | 生产百分位 | 简化代理归档（#1309） | Δ（生产−简化） |\n"
        );
        let _ = writeln!(
            md,
            "|------|-----------|----------------------|----------------|"
        );
        for r in results {
            let prod = r.percentile.as_ref().map(|p| p.e_percentile);
            let prod_s = prod
                .map(|v| format!("{v:.0}"))
                .unwrap_or_else(|| "—".to_string());
            let arch = archived_pct(&r.symbol);
            let delta = prod.map(|v| v - arch);
            let delta_s = delta
                .map(|v| format!("{v:+.1}"))
                .unwrap_or_else(|| "—".to_string());
            let _ = writeln!(
                md,
                "| {} | {} | {:.0} | {} |",
                r.symbol, prod_s, arch, delta_s
            );
        }

        let _ = writeln!(md, "\n## 3. 双跑逐位自检（theta_replay 同款确定性自证）\n");
        let _ = writeln!(
            md,
            "| 标的 | 同输入双跑 | 双跑耗时 |\n|------|-----------|----------|"
        );
        for r in results {
            let _ = writeln!(
                md,
                "| {} | {} | {:.2}s |",
                r.symbol,
                if r.replay_identical {
                    "PASS（逐位一致）"
                } else {
                    "FAIL（有分歧）"
                },
                r.replay_elapsed_s
            );
        }
    }

    let _ = writeln!(md, "\n## 4. 预注册冻结文本逐条对照（#1282 票面）\n");
    let _ = writeln!(
        md,
        "| 冻结项 | 票面 | 本跑执行 | 核对 |\n|--------|------|----------|------|"
    );
    let _ = writeln!(
        md,
        "| 过判据 | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078） | 每标的分位 >50 计数，7/7 判定 + p=0.0078125 | ✅ |"
    );
    let _ = writeln!(
        md,
        "| 随机基线 | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗） | `large_bootstrap_percentile_dual`（#1309 冻结函数）同口径移植：双腿各 N×H 随机进出 5000 次，MT19937 bit-exact | ✅ |"
    );
    let _ = writeln!(
        md,
        "| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | SYMBOL_FILES 全量 1min（prereg 表冻结值；ZN/6E 为 §9 补录 17y 快照） | ✅ |"
    );
    let _ = writeln!(
        md,
        "| 前后对照 | 原 E 单边/双开 vs 生产引擎，同标的同窗配对逐标 Δ | 生产双开百分位 vs #1309 归档双开百分位逐标 Δ | ✅ |"
    );
    let _ = writeln!(
        md,
        "| 纪律附款 | 不搜索/不调参/不挑窗/不加对照/不改判据 | 判据与窗口零改动，结果原样入档 | ✅ |"
    );

    let _ = writeln!(md, "\n## 5. 纪律附款执行记录\n");
    let _ = writeln!(
        md,
        "- 跑前不搜索、不调参、不挑窗口：判据/窗口逐字取自 #1282 冻结文本与 prereg-windows-v0 冻结表，未改一字。\n"
    );
    let _ = writeln!(
        md,
        "- 随机基线 PRNG 与 Python 冻结函数逐位同构（MT19937，测试锁对拍 CPython 实测向量）。\n"
    );
    let _ = writeln!(
        md,
        "- 结果原样入档（本文件 + issue1317-prod-engine-backtest.json）；过与不过都是 M1 有效结论。\n"
    );

    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("issue1317-prod-engine-backtest.md"), md)?;

    // JSON（机器可读，per-symbol 明细 + 汇总）。
    let json: serde_json::Value = serde_json::json!({
        "issue": 1317,
        "criterion": "7/7 百分位全部越过各自随机中位（符号检验单侧 p=0.0078125）",
        "n_pass": n_pass,
        "n_total": n_total,
        "sign_test_p_7of7": SIGN_TEST_P_7OF7,
        "sign_test_p_actual": sign_test_p(n_pass, n_total),
        "verdict": verdict,
        "failures": failures.iter().map(|(s, e)| serde_json::json!({"symbol": s, "error": e})).collect::<Vec<_>>(),
        "symbols": results.iter().map(|r| serde_json::json!({
            "symbol": r.symbol,
            "n_bars": r.n_bars,
            "untradable_ratio": r.untradable_ratio,
            "bh": r.bh,
            "n_orders": r.n_orders,
            "strat_return_mtm": r.strat_return_mtm,
            "dual": {
                "n_long": r.dual.n_long,
                "n_short": r.dual.n_short,
                "hold_long": r.dual.hold_long,
                "hold_short": r.dual.hold_short,
                "long_compound": r.dual.long_compound,
                "short_compound": r.dual.short_compound,
                "combined_compound": r.dual.combined_compound,
            },
            "dual_p3": r.percentile.as_ref().map(|p| serde_json::json!({
                "n_runs": p.n_runs,
                "p_random_ge_e": p.p_random_ge_e,
                "e_percentile": p.e_percentile,
                "rnd_median": p.rnd_median,
                "rnd_mean": p.rnd_mean,
                "rnd_std": p.rnd_std,
            })),
            "dual_beats_median": r.beats_median,
            "archived_simplified_pct": archived_pct(&r.symbol),
            "prod_minus_simplified_pct": r.percentile.as_ref().map(|p| p.e_percentile - archived_pct(&r.symbol)),
            "replay_double_identical": r.replay_identical,
            "replay_elapsed_s": r.replay_elapsed_s,
            "elapsed_s": r.elapsed_s,
        })).collect::<Vec<_>>(),
    });
    std::fs::write(
        out_dir.join("issue1317-prod-engine-backtest.json"),
        serde_json::to_string_pretty(&json).map_err(std::io::Error::other)? + "\n",
    )?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let mut symbols: Vec<String> = Vec::new();
    // 默认落点 = 项目根 `.chanlun/review-results`（data.rs 同款：CARGO_MANIFEST_DIR = rust/，
    // 上一级 = 项目根）——相对 CWD 会因 `cd rust` 跑批而漂到 rust/.chanlun/。
    let mut out_dir: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 的父目录 = 项目根")
        .join(".chanlun/review-results");

    let mut rest = args.iter().skip(1).peekable();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--out" => match rest.next() {
                Some(p) => out_dir = PathBuf::from(p),
                None => {
                    eprintln!("--out 需要路径参数");
                    return std::process::ExitCode::from(2);
                }
            },
            other if other.starts_with("--") => {
                eprintln!("未知选项: {other}");
                return std::process::ExitCode::from(2);
            }
            sym => symbols.push(sym.to_uppercase()),
        }
    }
    if symbols.is_empty() {
        symbols = SYMBOL_ORDER.iter().map(|s| s.to_string()).collect();
    }

    println!("=== M1 最终实验：生产引擎 7 期货回放 + 预注册 7/7 判据（#1317）===");
    println!("判据      : 7/7 百分位全部越过各自随机中位（符号检验单侧 p={SIGN_TEST_P_7OF7:.4}）");
    println!("品种      : {}", symbols.join(", "));
    println!("随机基线  : 双腿各 N×H 随机进出 {N_BOOTSTRAP_LARGE} 次（MT19937，与 #1309 冻结函数逐位同构）");

    let config = ThetaConfig::default();

    let mut results: Vec<SymbolResult> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    for sym in &symbols {
        match run_symbol(sym, &config) {
            Ok(r) => {
                match &r.percentile {
                    None => println!(
                        "  [{:<4}] {} bars | 生产复利 {:+.2}% | N={} <{MIN_N_FOR_P3} 跳过百分位 | 双跑 {}",
                        r.symbol, r.n_bars, r.dual.combined_compound, r.dual.n_long + r.dual.n_short,
                        if r.replay_identical { "PASS" } else { "FAIL" }
                    ),
                    Some(p) => println!(
                        "  [{:<4}] {} bars | 生产复利 {:+.2}% | 百分位 {:.0}（随机中位 {:+.2}%）{} | 双跑 {}",
                        r.symbol, r.n_bars, r.dual.combined_compound, p.e_percentile, p.rnd_median,
                        if r.beats_median == Some(true) { " ✅" } else { " ❌" },
                        if r.replay_identical { "PASS" } else { "FAIL" }
                    ),
                }
                results.push(r);
            }
            Err(e) => {
                println!("  [{sym:<4}] 加载失败：{e}");
                failures.push((sym.clone(), e));
            }
        }
    }

    let n_pass = results
        .iter()
        .filter(|r| r.beats_median == Some(true))
        .count();
    if !results.is_empty() {
        println!(
            "\n判决：{n_pass}/{n_total} 越过随机中位（7/7 单侧 p={SIGN_TEST_P_7OF7:.4}；实际 p={actual_p:.4}）",
            n_total = results.len(),
            actual_p = sign_test_p(n_pass, results.len())
        );
    }

    if results.is_empty() {
        eprintln!("\n卡点：生产引擎 7 期货回放无法执行——全部 7 个 1min 数据文件缺失（外部依赖）。");
        eprintln!("复现证据：env 无 DATABENTO_API_KEY / DATABENTO_KEY；analysis/data_cache/ 无 *_1m_databento_10y.json。");
        eprintln!("解除条件：① 注入 Databento 订阅 key（DATABENTO_API_KEY 或 DATABENTO_KEY）；或");
        eprintln!("          ② 编排层把 7 个数据文件放入 analysis/data_cache/（gitignored）。");
        eprintln!("续跑命令：");
        eprintln!("  # 1) 数据就位（有 key 时拉取；或直接放文件）");
        eprintln!("  DATABENTO_API_KEY=... PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all");
        eprintln!("  # 2) 生产引擎 7 期货回放 + 预注册判据测量（本 bin）");
        eprintln!("  cd rust && cargo run --release --features backtest_bin --bin theta_m1_dual");
        eprintln!(
            "  # 3) theta_replay vs theta_accept 逐位对拍（#1304 对拍 harness，验收第 3 项）"
        );
        eprintln!("  cargo run --release --features backtest_bin --bin theta_accept -- ES GC CL ZN 6E BRN DX --out .chanlun/review-results/theta_accept_m1");

        // 卡点报告也要原样入档（交付物完整性）。
        if let Err(e) = write_report(&results, &failures, &out_dir) {
            eprintln!("写卡点报告失败：{e}");
        } else {
            println!(
                "卡点报告已入档：{}/issue1317-prod-engine-backtest.{{md,json}}",
                out_dir.display()
            );
        }
        return std::process::ExitCode::from(2);
    }

    if let Err(e) = write_report(&results, &failures, &out_dir) {
        eprintln!("写报告失败：{e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "\n报告已入档：{}/issue1317-prod-engine-backtest.{{md,json}}",
        out_dir.display()
    );

    // 过与不过都是 M1 有效结论 → exit 0（卡点/加载失败才非 0）。
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::sign_test_p;

    #[test]
    fn sign_test_p_7of7() {
        assert!(
            (sign_test_p(7, 7) - 0.5f64.powi(7)).abs() < 1e-12,
            "7/7 全胜 p=1/128"
        );
        assert!(
            (sign_test_p(1, 7) - (1.0 - 0.5f64.powi(7))).abs() < 1e-12,
            "1/7 p=1−1/128"
        );
        assert!((sign_test_p(0, 7) - 1.0).abs() < 1e-12, "0/7 p=1");
        assert_eq!(sign_test_p(8, 7), 0.0, "n_pass > n_symbols ⟹ p=0");
    }
}
