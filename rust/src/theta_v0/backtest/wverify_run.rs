//! W-VERIFY acc-alpha 全定义域跑批（真实 BTC 全量，全 OOS 窗，锚 d906648041 段2完整实装）。
//! `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture`。
//! 数据=btc_1m_full.json（461万 bar）；μ 估计在**全 OOS 窗**（2023-01-01…2025-06-30，无 MAX_BARS 截断——
//! 覆盖全定义域，区别 L3 的 32K 有效域边界；231号：全域结论非截断结论）。

use super::super::config::ThetaConfig;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::{data, decontam, perm_test};
use std::collections::BTreeMap;

#[test]
#[ignore]
fn wverify_full() {
    let cfg = ThetaConfig::default();
    // 真实 API：load_by_symbol（SYMBOLS 表 → btc_1m_full.json 461万 bar）→ slice_date_window 全 OOS 窗。
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30"); // 协议 §2.1 BTC OOS；Holdout(2025-07-01+) 隔离
    assert!(!oos.bars.is_empty(), "OOS 窗空——数据漂移");
    // 全定义域 μ 表：逐 bar 因果分类收集所有级别新确认买卖点（无 N^δ 门，估 μ 覆盖全候选集）。
    let est = build_mu_from_bars(&oos.bars, &cfg);
    // 逐笔 (ℓ, bsp_class, δ, X_γ) 投影（perm_test 输入 + Welford 聚合键）。
    let trades: Vec<(u32, u8, i8, f64)> = est
        .trades()
        .iter()
        .map(|(c, x)| (c.level, c.bsp_class(), c.delta, *x))
        .collect();
    // 逐桶 Welford 聚合 (n, mean, m2)（BTreeMap 确定序）。
    let mut agg: BTreeMap<(u32, u8, i8), (u64, f64, f64)> = BTreeMap::new();
    for &(lv, bc, d, x) in &trades {
        let e = agg.entry((lv, bc, d)).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = x - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (x - e.1);
    }
    // 路径 A 分层内 δ 置换逐桶 perm_p（预注册 §1.4，N_PERM=200 种子 20260701 冻结）。
    let pp = perm_test::stratified_delta_perm_p(&trades, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::new();
    // 逐桶按成交时间序收集 X_γ 序列，供事件聚集/自相关 n_eff 校正（预注册 §1.3，665 neff/nraw 口径）。
    let mut series: BTreeMap<(u32, u8, i8), Vec<f64>> = BTreeMap::new();
    for &(lv, bc, d, x) in &trades {
        series.entry((lv, bc, d)).or_default().push(x);
    }
    for (&(lv, bc, d), &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&(lv, bc, d)).unwrap_or(&1.0);
        // 三态判据（预注册 §3）：n_eff = 事件聚集/自相关校正后有效样本（665 neff/nraw 口径）。
        let n_eff = decontam::effective_n(series.get(&(lv, bc, d)).map(Vec::as_slice).unwrap_or(&[]));
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        rows.push_str(&format!(
            "| L{} | {} | {:+} | {} | {:.2} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:?} |\n",
            lv, bc, d, n, n_eff, mean, lcb, ucb, cv, perm_p, st
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    let mut lv_set: Vec<u32> = agg.keys().map(|k| k.0).collect();
    lv_set.sort();
    lv_set.dedup();
    eprintln!(
        "WV_FULL trades={} buckets={} verdict={:?} V={} F={} I={} levels={:?}",
        trades.len(), agg.len(), v, nv, nf, ni, lv_set
    );
    std::fs::write("/tmp/wv_full_rows.md", &rows).ok();
    assert!(!trades.is_empty(), "trades 空——管线未产观测");
}
