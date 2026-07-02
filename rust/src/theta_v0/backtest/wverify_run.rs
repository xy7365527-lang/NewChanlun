//! W-VERIFY acc-alpha 定版跑批（BTC OOS，锚 d906648041 段2完整实装）。
//! O(n²) 引擎 ⟹ 截 MAX_BARS=32000（复用 L3 有效域边界，非全窗结论；231号标注）。
//! `#[ignore]`: cargo test wverify_full -- --ignored --nocapture。

use super::super::config::ThetaConfig;
use super::data::{self, Dataset};
use super::l3_delta_r_alpha::build_walk_forward_mu;
use super::{decontam, perm_test};
use std::collections::BTreeMap;

#[test]
#[ignore]
fn wverify_full() {
    let config = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &config).expect("load BTC");
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    // ponytail: O(n²) 引擎 ⟹ 截 32000 bar（= L3 MAX_BARS 有效域边界，非全窗）。
    let cut = 32_000.min(oos.bars.len());
    let bars = oos.bars[..cut].to_vec();
    let dates = oos.dates[..cut.min(oos.dates.len())].to_vec();
    let train = Dataset { bars, dates, ..oos };
    let (est, n_sig) = build_walk_forward_mu(&train, &config);
    let trades: Vec<(u32, u8, i8, f64)> =
        est.trades().iter().map(|(c, x)| (c.level, c.bsp_class(), c.delta, *x)).collect();
    let mut agg: BTreeMap<(u32, u8, i8), (u64, f64, f64)> = BTreeMap::new();
    for &(lv, bc, d, x) in &trades {
        let e = agg.entry((lv, bc, d)).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = x - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (x - e.1);
    }
    let pp = perm_test::stratified_delta_perm_p(&trades, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::new();
    for (&(lv, bc, d), &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&(lv, bc, d)).unwrap_or(&1.0);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n as usize, cv, za, pa);
        states.push(st);
        rows.push_str(&format!("| L{} | {} | {:+} | {} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:?} |\n", lv, bc, d, n, mean, lcb, ucb, cv, perm_p, st));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    let mut lv: Vec<u32> = agg.keys().map(|k| k.0).collect();
    lv.sort();
    lv.dedup();
    eprintln!("WV_FULL bars={} n_sig={} trades={} buckets={} verdict={:?} V={} F={} I={} levels={:?}", cut, n_sig, trades.len(), agg.len(), v, nv, nf, ni, lv);
    std::fs::write("/tmp/wv_full_rows.md", &rows).ok();
    assert!(!trades.is_empty(), "trades 空——管线未产观测");
}
