//! 增量塔真实数据标度 profile（#[ignore]，需真实数据 + release）。

use super::*;
use super::super::backtest::data;
use super::super::parser;

/// ★真实数据 per-bar 标度：CL 真实段账本逐段追加，增量 vs 全量累积成本 + exp。
///
/// 真实段账本单调追加（parser 前缀稳定语义）⟹ 增量有效域命中。验证真实数据下增量 exp≈1
/// 而全量 exp≈2（塔构造超线性消除）。L2 经验标度（formalization-validity-domain 231号）。
#[test]
#[ignore = "真实数据标度 profile：需 CL 数据；--release（per-bar 双跑对照）"]
fn profile_incremental_tower_real_scaling() {
    let cfg = ThetaConfig::default();
    let ds = match data::load_by_symbol("CL", &cfg) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("DATA BLOCKER: {e}");
            panic!("需真实数据");
        }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    // ponytail: 大规模标度验证 acceptance[4]——全引擎 per-bar exp≈1.0 @16K。
    // a7dfec46: n<5000 不可靠；此处用 [5K, 10K, 16K] 真实大规模。
    let sizes = [5_000usize, 10_000, 16_000];
    let mut full_times = Vec::new();
    let mut inc_times = Vec::new();
    let mut used_sizes = Vec::new();

    for &n in &sizes {
        if n > oos.bars.len() {
            eprintln!("DATA LIMIT: n={n} > oos.bars.len()={}，跳过", oos.bars.len());
            break;
        }
        let bars = &oos.bars[..n];

        // 全量 per-bar 累积。
        let t0 = std::time::Instant::now();
        for i in 50..n {
            let l0 = parser::parse_layer(&bars[..i], &cfg);
            let _ = classify_with_tower(&l0, &cfg);
        }
        full_times.push(t0.elapsed().as_secs_f64());

        // 增量 per-bar 累积。
        let t0 = std::time::Instant::now();
        let mut cache = TowerCache::new();
        for i in 50..n {
            let l0 = parser::parse_layer(&bars[..i], &cfg);
            let _ = classify_with_tower_incremental(&l0, &cfg, &mut cache);
        }
        inc_times.push(t0.elapsed().as_secs_f64());
        used_sizes.push(n);
        eprintln!("n={n} done: full={:.2}s inc={:.2}s", *full_times.last().unwrap(), *inc_times.last().unwrap());
    }

    // 逐相邻对算 exp（log-log 斜率），大规模验证 acceptance[4]。
    eprintln!("\n===== 增量塔真实标度（CL per-bar 累积，大规模）=====");
    for w in used_sizes.windows(2) {
        let (n0, n1) = (w[0], w[1]);
        let i0 = used_sizes.iter().position(|&s| s == n0).unwrap();
        let i1 = i0 + 1;
        let full_exp = (full_times[i1] / full_times[i0].max(1e-12)).ln()
            / (n1 as f64 / n0 as f64).ln();
        let inc_exp = (inc_times[i1] / inc_times[i0].max(1e-12)).ln()
            / (n1 as f64 / n0 as f64).ln();
        eprintln!(
            "  [{n0}→{n1}] full exp≈{full_exp:.2}  inc exp≈{inc_exp:.2}  \
             增量/全量比 @{n1}: {:.2}x",
            inc_times[i1] / full_times[i1].max(1e-12)
        );
    }
}
