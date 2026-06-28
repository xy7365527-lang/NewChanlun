//! parse_layer 标度 profile（性能工位 #93，纯测量，cfg(test) only）。
//!
//! L1（管线性能测量，非 alpha）：在真实 ES 1min 数据上递增 bar 数实测 parse_layer
//! 单次调用耗时 + 各子步耗时，拟合标度指数，定位 O(n²) 热点。
//!
//! 运行：`cargo test --release -p newchan_rust --lib parser::profile -- --ignored --nocapture`

use super::*;
use crate::theta_v0::backtest::data::{data_dir, load_symbol};
use crate::theta_v0::config::ThetaConfig;
use std::time::Instant;

/// 多次取最小耗时（μs），降低调度噪声。
fn bench<F: FnMut()>(iters: u32, mut f: F) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..iters {
        let t = Instant::now();
        f();
        let us = t.elapsed().as_secs_f64() * 1e6;
        if us < best {
            best = us;
        }
    }
    best
}

/// log-log 局部标度指数（相邻两点）。
fn exp(n0: usize, t0: f64, n1: usize, t1: f64) -> f64 {
    (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln()
}

#[test]
#[ignore]
fn profile_parse_layer_scaling() {
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = load_symbol(&path, "ES", &cfg).expect("加载 ES");
    let total = ds.bars.len();
    eprintln!("ES bars 总数: {total}");

    let sizes = [1000usize, 2000, 4000, 8000, 16000, 32000, 64000, 128000];
    eprintln!(
        "{:>8} | {:>10} | {:>5} | {:>9} {:>9} {:>9} {:>9} {:>9}",
        "n", "parse_us", "exp", "incl", "fractal", "stroke", "segment", "tail"
    );
    let mut prev: Option<(usize, f64)> = None;
    for &n in &sizes {
        if n > total {
            break;
        }
        let bars = &ds.bars[..n];
        // 迭代次数随规模递减（保持总测量时间可控）。
        let iters = if n <= 8000 { 20 } else if n <= 32000 { 6 } else { 2 };

        let t_parse = bench(iters, || {
            let _ = parse_layer(bars, &cfg);
        });

        // 子步分解（复用 parse_layer 内部顺序）。
        let incl = inclusion::process_inclusion(bars);
        let t_incl = bench(iters, || {
            let _ = inclusion::process_inclusion(bars);
        });
        let fractals = fractal::detect_fractals(&incl.merged);
        let t_fractal = bench(iters, || {
            let _ = fractal::detect_fractals(&incl.merged);
        });
        let strokes = stroke::build_strokes(&fractals, &cfg.parse);
        let t_stroke = bench(iters, || {
            let _ = stroke::build_strokes(&fractals, &cfg.parse);
        });
        let t_segment = bench(iters, || {
            let _ = segment::divide_segments(&strokes, &cfg.parse);
        });
        // tail 子步：build_tail 不再重跑段划分，须先算 (segments, pending_start) 再传入。
        let (segs_v, pend_v) = segment::divide_segments_with_tail(&strokes, &cfg.parse);
        let t_tail = bench(iters, || {
            let _ = tail::build_tail(&incl.merged, &fractals, &strokes, &segs_v, pend_v);
        });

        let e = match prev {
            Some((pn, pt)) => exp(pn, pt, n, t_parse),
            None => 0.0,
        };
        eprintln!(
            "{:>8} | {:>10.1} | {:>5.2} | {:>9.1} {:>9.1} {:>9.1} {:>9.1} {:>9.1}  | strokes={} merged={}",
            n, t_parse, e, t_incl, t_fractal, t_stroke, t_segment, t_tail,
            strokes.len(), incl.merged.len()
        );
        prev = Some((n, t_parse));
    }
}

/// 诊断：隔离 second_seq_scan_window 对 segment 标度的影响 + 输出等价性。
/// 若 window=0（无限）是 O(n²) 而有限窗近线性，且输出 bit-identical → 定位热点 = second_seq_has_fractal。
#[test]
#[ignore]
fn diag_segment_window_effect() {
    use crate::theta_v0::config::ParseConfig;
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = load_symbol(&path, "ES", &cfg).expect("加载 ES");

    let sizes = [4000usize, 8000, 16000, 32000, 64000, 128000];
    let windows = [0u32, 50, 200];
    eprintln!("段划分 window 效应（μs）+ 与 window=0 输出等价性");
    eprintln!("{:>8} | {:>12} {:>12} {:>12} | seg_n  eq50 eq200", "n", "w=0", "w=50", "w=200");
    for &n in &sizes {
        if n > ds.bars.len() {
            break;
        }
        let bars = &ds.bars[..n];
        let incl = inclusion::process_inclusion(bars);
        let fractals = fractal::detect_fractals(&incl.merged);
        let strokes = stroke::build_strokes(&fractals, &cfg.parse);
        let iters = if n <= 16000 { 10 } else { 3 };

        let mut times = [0.0f64; 3];
        let mut base_segs = Vec::new();
        let mut eqs = [true; 3];
        for (wi, &w) in windows.iter().enumerate() {
            let pc = ParseConfig { second_seq_scan_window: w, ..cfg.parse };
            times[wi] = bench(iters, || {
                let _ = segment::divide_segments(&strokes, &pc);
            });
            let segs = segment::divide_segments(&strokes, &pc);
            if wi == 0 {
                base_segs = segs;
            } else {
                eqs[wi] = segs == base_segs;
            }
        }
        eprintln!(
            "{:>8} | {:>12.1} {:>12.1} {:>12.1} | {:>5}  {:>4} {:>4}",
            n, times[0], times[1], times[2], base_segs.len(), eqs[1], eqs[2]
        );
    }
}

// ============================================================================
// 增量 parse_layer 标度 + bit-exact 对拍（#93 per-bar substrate O(n²) 根因解）。
// ============================================================================

/// 增量 parse_layer 标度：逐 bar `ParseLayerIncr::append` 总耗时 vs 全量 `parse_layer` 单次。
///
/// **测量目标**：坐实增量 inclusion 是否把 per-bar substrate 的 O(n²) 项降到 O(n)，
/// 以及下游（fractal/stroke/segment）全量重算是否成新 O(n²) 热点（诚实标注边界）。
///
/// 运行：`cargo test --release -p newchan_rust --lib parser::profile -- --ignored --nocapture`
#[test]
#[ignore]
fn profile_parse_layer_incr_scaling() {
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = load_symbol(&path, "ES", &cfg).expect("加载 ES");

    let sizes = [2000usize, 4000, 8000, 16000];
    eprintln!("增量 parse_layer 标度（逐 bar append 总耗时 μs vs 全量单次）");
    eprintln!(
        "{:>8} | {:>12} {:>12} | {:>8} {:>8} {:>6}",
        "n", "incr_total", "full_once", "incr_exp", "full_exp", "merged"
    );
    let mut prev: Option<(usize, f64, f64)> = None;
    for &n in &sizes {
        if n > ds.bars.len() {
            break;
        }
        let bars = &ds.bars[..n];

        // 增量：逐 bar append，取总耗时。
        let t_incr = bench(1, || {
            let mut incr = ParseLayerIncr::new(&cfg);
            for b in bars {
                let _ = incr.append(*b);
            }
        });

        // 全量：单次 parse_layer（对照）。
        let t_full = bench(3, || {
            let _ = parse_layer(bars, &cfg);
        });

        let incl = inclusion::process_inclusion(bars);
        let n_merged = incl.merged.len();

        let (ie, fe) = match prev {
            Some((pn, pi, pf)) => (
                exp(pn, pi, n, t_incr),
                exp(pn, pf, n, t_full),
            ),
            None => (0.0, 0.0),
        };
        eprintln!(
            "{:>8} | {:>12.1} {:>12.1} | {:>8.2} {:>8.2} {:>6}",
            n, t_incr, t_full, ie, fe, n_merged
        );
        prev = Some((n, t_incr, t_full));
    }

    // 子步标度：隔离 inclusion 增量 vs 下游重算，定位残余 O(n²)。
    eprintln!("\n子步标度（inclusion 增量 vs 下游重算，μs）");
    eprintln!(
        "{:>8} | {:>12} {:>12} {:>12} {:>12}",
        "n", "incr_incl", "fractal", "stroke", "segment"
    );
    for &n in &sizes {
        if n > ds.bars.len() {
            break;
        }
        let bars = &ds.bars[..n];
        let t_incr_incl = bench(3, || {
            let mut s = inclusion::IncrInclusion::empty();
            for b in bars {
                s = s.append(*b);
            }
        });
        let incl = inclusion::process_inclusion(bars);
        let fractals = fractal::detect_fractals(&incl.merged);
        let strokes = stroke::build_strokes(&fractals, &cfg.parse);
        let t_fractal = bench(5, || {
            let _ = fractal::detect_fractals(&incl.merged);
        });
        let t_stroke = bench(5, || {
            let _ = stroke::build_strokes(&fractals, &cfg.parse);
        });
        let t_segment = bench(5, || {
            let _ = segment::divide_segments_with_tail(&strokes, &cfg.parse);
        });
        eprintln!(
            "{:>8} | {:>12.1} {:>12.1} {:>12.1} {:>12.1}",
            n, t_incr_incl, t_fractal, t_stroke, t_segment
        );
    }
}

/// **★bit-exact 硬指标：逐 bar 断言 `ParseLayerIncr::append` 链 == `parse_layer(&bars[..=i])`**。
///
/// 镜像 incremental.rs:132 `bit_exact_per_bar` 模式。合成数据（always-run）+ 真实数据（ignored）。
#[test]
fn bit_exact_parse_layer_incr_per_bar_synthetic() {
    // 合成 1500 bar：缓慢趋势 + 周期回撤（产足够 merged/分型/笔/段触发各分支）。
    let bars: Vec<Bar> = (0..1500usize)
        .map(|i| {
            let base = 1000i64 + (i as i64) * 2;
            let cycle = ((i as f64) / 47.0).sin() as i64 * 35;
            let close = base + cycle;
            Bar {
                source_index: i,
                timestamp: i as i64,
                open: close - 1,
                high: close + 5,
                low: close - 5,
                close,
                volume: 1000,
                untradable: false,
            }
        })
        .collect();

    let cfg = ThetaConfig::default();
    let mut incr = ParseLayerIncr::new(&cfg);
    for (i, b) in bars.iter().enumerate() {
        let incr_layer = incr.append(*b);
        let full_layer = parse_layer(&bars[..=i], &cfg);
        assert_eq!(
            incr_layer, full_layer,
            "bar {i}: 增量 parse_layer != 全量（bit-exact 破裂）"
        );
    }
}

/// 真实数据逐 bar bit-exact（ES + CL，ignored 需数据）。
#[test]
#[ignore = "bit-exact 验证：需 ES 数据；--release（O(n²) 全量对照）"]
fn bit_exact_parse_layer_incr_per_bar_es() {
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = match load_symbol(&path, "ES", &cfg) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("DATA BLOCKER: {e}");
            panic!("需真实数据");
        }
    };
    // 4K bar 控时（全量对照 O(i)/bar）。
    let n = 4_000.min(ds.bars.len());
    let bars = &ds.bars[..n];

    let mut incr = ParseLayerIncr::new(&cfg);
    for (i, b) in bars.iter().enumerate() {
        let incr_layer = incr.append(*b);
        let full_layer = parse_layer(&bars[..=i], &cfg);
        assert_eq!(
            incr_layer, full_layer,
            "bar {i}: 增量 parse_layer != 全量（ES bit-exact 破裂）"
        );
    }
    eprintln!("\n===== parse_layer 增量 bit-exact 通过：ES n={n} bars =====");
}

/// 隔离验证：ES 数据上 IncrInclusion 逐 bar == process_inclusion（不含下游）。
/// 用于定位 bit_exact_parse_layer_incr_per_bar_es 失败是否在 inclusion 层。
#[test]
#[ignore = "隔离 inclusion bit-exact：需 ES 数据"]
fn bit_exact_inclusion_only_es() {
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = match load_symbol(&path, "ES", &cfg) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("DATA BLOCKER: {e}");
            panic!("需真实数据");
        }
    };
    let n = 4_000.min(ds.bars.len());
    let bars = &ds.bars[..n];
    let mut incr = inclusion::IncrInclusion::empty();
    for (i, b) in bars.iter().enumerate() {
        incr = incr.append(*b);
        let incr_merged = incr.to_result_ref().merged;
        let full_merged = &inclusion::process_inclusion(&bars[..=i]).merged;
        assert_eq!(
            incr_merged, full_merged,
            "bar {i}: 增量 inclusion merged != 全量（ES bit-exact 破裂）"
        );
    }
    eprintln!("\n===== inclusion 增量 bit-exact 通过：ES n={n} bars =====");
}

/// 诊断：隔离 IncrStrokes::append 逐 bar 标度（O(n²) 修复验证）。
/// 测量逐 fractals 长度递增的 IncrStrokes::append 总耗时，拟合 exp。
/// 修复前 exp ≈ 2.9（take_while O(n) + 嵌套循环 O(n²)），修复后目标 exp ≈ 1.0。
#[test]
#[ignore = "IncrStrokes 标度诊断：需 ES 数据；--release"]
fn diag_incr_strokes_scaling() {
    let cfg = ThetaConfig::default();
    let path = data_dir().join("es_1m_databento_10y.json");
    let ds = load_symbol(&path, "ES", &cfg).expect("加载 ES");

    let sizes = [2000usize, 4000, 8000, 16000];
    eprintln!("IncrStrokes::append 逐 bar 标度（总耗时 μs vs exp）");
    eprintln!("{:>8} | {:>12} {:>8}", "n", "incr_strokes", "exp");
    let mut prev: Option<(usize, f64)> = None;
    for &n in &sizes {
        if n > ds.bars.len() {
            break;
        }
        let bars = &ds.bars[..n];
        let incl = inclusion::process_inclusion(bars);
        let merged = &incl.merged;
        let fractals_all = fractal::detect_fractals(merged);

        // 逐 fractals 长度 append，测总耗时。
        let t = bench(1, || {
            let mut incr = stroke::IncrStrokes::empty();
            for end in 1..=fractals_all.len() {
                incr = incr.append(&fractals_all[..end], &cfg.parse);
            }
        });

        let e = match prev {
            Some((pn, pt)) => exp(pn, pt, n, t),
            None => 0.0,
        };
        eprintln!("{:>8} | {:>12.1} {:>8.2}", n, t, e);
        prev = Some((n, t));
    }
}
