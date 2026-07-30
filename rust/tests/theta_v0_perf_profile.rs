//! **全窗瓶颈 profile（task #69，goal 第4要素「性能最大化」+ 解锁全窗 L2 否证）**。
//!
//! ## 问题（机器坐实，非读码推测）
//!
//! 全窗（BTC 1.31M bar）端到端 `run_theta_v0` >5min CPU-bound 无输出 ⟹ 全窗 L2 否证不可行
//! （否证只能截断窗，检验功效受限）。本 profile 对**多个窗口大小**（截取 BTC 全集前 N bar）
//! 分段计时引擎前三段 parse/classify/recognize，观测各段耗时随 bar 数 n 的增长量级
//! （O(n)/O(n log n)/O(n²)），定位**真热点函数 + owner 归属**。
//!
//! ## 段覆盖（pub API，非 cfg(test) gated）
//!
//! crate pub API 暴露 `parser::parse_layer` / `classifier::classify` / `strategy::recognize`
//! （均 pub，非 gated）。`backtest::{data,runner}` 是 `#[cfg(test)]` gated（依赖 dev-dep
//! serde_json + 避免污染 cdylib，见 theta_v0/mod.rs:88）——**集成测试是独立 crate，看不到
//! gated 模块**，故 `run_theta_v0`/`plan_and_fill_mtm`/`run_closed_loop` 无法从此处测。本
//! profile 因此**只测前三段**（恰好覆盖瓶颈归属的判定边界）：
//!
//! - 若前三段（parse/classify/recognize）在中等窗口已 CPU-bound 慢增长（如 O(n²)）⟹ 瓶颈在
//!   parser/classifier（本工位 owner 区，除 signal.rs）或 recognize（strategy l2）。
//! - 若前三段在中等窗口快（线性、秒级）⟹ 全窗 >5min 的瓶颈**不在前三段**，必在 backtest 的
//!   plan_and_fill_mtm / run_closed_loop（owner=l2 工位）——与 runner.rs 已有注释坐实的
//!   「全 OOS 窗 268K bar × 430K 决策的退出生成器逐 bar 检查 590s timeout 跑不完」一致。
//!
//! 数据加载内联（不依赖 gated 的 backtest::data），复用 data.rs 的 parallel-array schema +
//! 量化 + untradable 判据（bit-exact 同口径，见各步注释）。
//!
//! ## 认识论等级：**L1**（profile = 管线 CPU 度量，零信息增量，formalization-validity-domain 231号）
//!
//! 耗时数字是确定性工程度量（可复现），不验证 Θ 在市场有效。
//!
//! 跑法：`cargo test --manifest-path rust/Cargo.toml --release --test theta_v0_perf_profile \
//!   -- --ignored --nocapture`

use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::{quantize, Bar};
use newchan_rust::theta_v0::{classifier, parser, strategy};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;

/// parallel-array schema（与 backtest/data.rs::RawData 同口径，内联避免依赖 gated 模块）。
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
    volumes: Vec<Option<f64>>,
}

/// 加载 BTC 全集为量化 Bar 序列（与 data.rs::load_symbol 同口径：量化 + untradable 判据 +
/// source_index=全集下标）。前缀截取后 source_index 0..n 仍 = 局部下标（下游索引合法）。
fn load_btc_bars(tick_size: f64) -> Vec<Bar> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 父目录 = 项目根")
        .join("analysis/data_cache/btc_1m_full.json");
    let text = std::fs::read_to_string(&path).expect("读取 btc_1m_full.json");
    // Python json(allow_nan) 写出 NaN/Inf → null（与 data.rs 同口径）。
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData = serde_json::from_str(&text).expect("解析 BTC JSON");
    drop(text);

    let n = raw.closes.len();
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let (o, h, l, c) = (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]);
        let v = raw.volumes.get(i).and_then(|x| *x).unwrap_or(0.0);
        let (oq, hq, lq, cq, untradable) =
            if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) {
                let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
                let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
                let untradable = bad_range || bad_price || v <= 0.0;
                (
                    quantize(o, tick_size),
                    quantize(h, tick_size),
                    quantize(l, tick_size),
                    quantize(c, tick_size),
                    untradable,
                )
            } else {
                let prev = bars.last().map(|b: &Bar| b.close).unwrap_or(0);
                (prev, prev, prev, prev, true)
            };
        bars.push(Bar {
            source_index: i,
            timestamp: i as i64, // 单调键（profile 不需历法换算；同序列单调即可）
            open: oq,
            high: hq,
            low: lq,
            close: cq,
            volume: v as i64,
            untradable,
        });
    }
    bars
}

#[test]
#[ignore = "全窗瓶颈 profile，需 analysis/data_cache/btc_1m_full.json；release 模式；显式 --ignored"]
fn profile_fullwindow_bottleneck() {
    let config = ThetaConfig::default();

    let t_load = Instant::now();
    let all_bars = load_btc_bars(config.tick.tick_size);
    let load_secs = t_load.elapsed().as_secs_f64();
    let n_full = all_bars.len();
    eprintln!(
        "\n===== 全窗瓶颈 profile（BTC 前三段 parse/classify/recognize，release，L1 度量）====="
    );
    eprintln!(
        "[load] BTC 全集 bars={n_full} 加载耗时={load_secs:.2}s（owner=data.rs，IO+JSON解析）"
    );

    // 多窗口大小（截取前 N bar）：每段耗时 / n 翻倍倍数推断复杂度量级。
    // 从小窗起步，逐窗 flush 进度（避免大窗 CPU-bound 时看不到中间结果）。窗口大小可通过
    // 环境变量 PROFILE_MAX_BARS 上限裁剪（默认 400K，避免单段 O(n²) 在 800K+ 爆炸时跑太久）。
    let max_bars: usize = std::env::var("PROFILE_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400_000);
    let window_sizes: Vec<usize> = [
        25_000usize,
        50_000,
        100_000,
        200_000,
        400_000,
        800_000,
        n_full,
    ]
    .into_iter()
    .filter(|&n| n <= max_bars.min(n_full))
    .collect();

    // (n, parse, classify, recognize, n_segs, n_bsp, n_dec)
    let mut rows: Vec<(usize, f64, f64, f64, usize, usize, usize)> = Vec::new();

    for &n in &window_sizes {
        let n = n.min(n_full);
        if rows.iter().any(|r| r.0 == n) {
            continue; // 去重（n_full 可能撞 800K 上界）
        }
        let bars: Vec<Bar> = all_bars[..n].to_vec();

        // 段1：parse_layer（owner=parser）。
        let t = Instant::now();
        let l0 = parser::parse_layer(&bars, &config);
        let t_parse = t.elapsed().as_secs_f64();
        let n_segs = l0.segments.len();

        // 段2：classify（owner=classifier，本工位 owner 区，除 signal.rs）。
        let t = Instant::now();
        let classification = classifier::classify(&l0, &config);
        let t_classify = t.elapsed().as_secs_f64();
        let n_bsp: usize = classification.levels.iter().map(|lv| lv.bsp.len()).sum();

        // 段3：recognize（owner=strategy/mod.rs，l2 工位 owner）。
        let t = Instant::now();
        let decisions = strategy::recognize(&classification, &bars, &config);
        let t_recognize = t.elapsed().as_secs_f64();
        let n_dec = decisions.len();

        let total = t_parse + t_classify + t_recognize;
        eprintln!("\n[n={n}] segs={n_segs} bsp={n_bsp} decisions={n_dec}  前三段总={total:.3}s");
        eprintln!(
            "  parse     ={t_parse:8.3}s  {:5.1}%  (owner=parser)",
            100.0 * t_parse / total.max(1e-9)
        );
        eprintln!(
            "  classify  ={t_classify:8.3}s  {:5.1}%  (owner=classifier 本工位，除 signal.rs)",
            100.0 * t_classify / total.max(1e-9)
        );
        eprintln!(
            "  recognize ={t_recognize:8.3}s  {:5.1}%  (owner=strategy l2)",
            100.0 * t_recognize / total.max(1e-9)
        );
        use std::io::Write;
        std::io::stderr().flush().ok(); // 逐窗即时 flush（大窗 CPU-bound 时仍见进度）

        rows.push((n, t_parse, t_classify, t_recognize, n_segs, n_bsp, n_dec));
    }

    // ── 复杂度量级推断：相邻窗口 n 翻倍时各段耗时倍数。倍数≈2⟹O(n)，≈4⟹O(n²)，2~2.5⟹O(n log n)。──
    eprintln!("\n===== 复杂度量级推断（相邻窗口耗时倍数 / n 倍数）=====");
    eprintln!("  倍数/n倍数≈1⟹O(n)线性  ≈2⟹O(n²)平方  略>1⟹O(n log n)");
    for w in rows.windows(2) {
        let (n0, p0, c0, r0, ..) = w[0];
        let (n1, p1, c1, r1, ..) = w[1];
        let nr = n1 as f64 / n0 as f64;
        let norm = |a: f64, b: f64| if a > 1e-6 { (b / a) / nr } else { f64::NAN };
        eprintln!(
            "  {n0}→{n1}(n×{nr:.2}): parse {:.2}× classify {:.2}× recognize {:.2}×（耗时倍数/n倍数，≈1=线性）",
            norm(p0, p1),
            norm(c0, c1),
            norm(r0, r1),
        );
    }

    // ── 决策/bsp/segs 随 n 增长（下游 plan_and_fill_mtm 复杂度 ∝ n × decisions，证据链）──
    eprintln!("\n===== segs/bsp/decisions 随 n 增长 =====");
    for r in &rows {
        eprintln!(
            "  n={:>8} segs={:>7} bsp={:>7} decisions={:>8} (dec/n={:.4}, segs/n={:.4})",
            r.0,
            r.4,
            r.5,
            r.6,
            r.6 as f64 / r.0 as f64,
            r.4 as f64 / r.0 as f64
        );
    }

    eprintln!(
        "\n★瓶颈归属判定（机器证据）：\n  \
         - 前三段全窗(n={n_full})总耗时见上 [n={n_full}] 行。若 <60s ⟹ 全窗 >5min 瓶颈**不在前三段**，\n    \
         必在 backtest::plan_and_fill_mtm/run_closed_loop（owner=l2 工位）。\n  \
         - 若某段 O(n²)（倍数/n倍数≈2）⟹ 该段是热点，owner 见标注。\n  \
         ★profile=L1 度量（CPU 耗时，零信息增量，231号）。"
    );

    assert!(!rows.is_empty(), "至少 profile 一个窗口");
}
