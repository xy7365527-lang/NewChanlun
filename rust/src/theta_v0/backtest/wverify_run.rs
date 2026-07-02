//! W-VERIFY acc-alpha 全定义域跑批（真实 BTC 全量，walk-forward OOS，**残差口径**，锚 d906648041 段2）。
//! `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture`。
//! 数据=btc_1m_full.json（461万 bar）；μ 估计覆盖**全 OOS 定义域**（2023-01-01…2025-06-30，无 MAX_BARS
//! 截断——覆盖全定义域，区别 L3 的 32K 有效域边界；231号：全域结论非截断结论）。
//!
//! ## 残差口径 + 分层补全（task #82，alpha分离.pdf §1/§4.2 + alpha检验.pdf §6/§7-§8）
//!
//! - **gap#1 残差减法**：所有 alpha 检验改看残差 `Y_i=δ_i(H_i−B̂_i)−C_i`（§1）——mean/lcb/ucb/perm_p
//!   全在 [`ResidualTrade::y`] 上算，非原始 X_γ（X_γ 混入 BTC 上涨 beta）。B̂_i 由 build_mu_from_bars
//!   因果估计（扩张窗漂移）。
//! - **gap#2 分层补全**：δ 置换分层键补齐 `(ℓ, h桶, time block, σ^H)`（§4.2）——见 [`perm_test`]。
//!   time block 逐窗偏移（`win.i·WF_TIME_STRIDE`）使不同 walk-forward 窗不碰撞。
//! - **gap#4 删尾稳健**（alpha检验.pdf §6）：逐桶删前 3 赢家重估符号（[`perm_test::drop_top_k_mean`]），
//!   报告符号翻转（尾部依赖诊断，主桶 CV 极高尤需）。
//! - **gap#5 H2 方向不对称**（alpha检验.pdf §7-§8）：逐级别 `β=μ_sell−μ_buy` block bootstrap 单边 p
//!   （[`perm_test::direction_asymmetry_beta_pvalue`]，H0:β≤0）。
//!
//! ## G-A4（walk-forward LCB）
//!
//! `est`（残差记录）不由整个 OOS 窗单块估计（in-sample 正态近似），而消费 `prereg_windows.rs` 冻结的
//! BTC anchored walk-forward 窗口：只取 `test_start ≥ OOS_START` 的窗，每窗独立对 **test 段** 估计后逐笔
//! 累积（不拼接 Dataset 防接缝伪相邻）。聚合后每笔残差都来自"该窗训练截止之后"的样本外区间。

use super::super::config::ThetaConfig;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::mu_estimator::ResidualTrade;
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::BTreeMap;

/// walk-forward 窗间 time block 偏移步长（§4.2：不同窗的 time block 不碰撞；窗内块 <stride）。
const WF_TIME_STRIDE: u32 = 10_000;
/// 删尾稳健删除的赢家数（alpha检验.pdf §6：删前 3 最大赢家）。
const TRIM_K: usize = 3;

/// walk-forward OOS 残差聚合（G-A4）：取 `symbol` 在 `PREREG_WINDOWS` 冻结 anchored 窗口中
/// `test_start ≥ OOS_START` 的子集，逐窗对 **test 段** 独立 [`build_mu_from_bars`]（time_block_base
/// = `win.i·WF_TIME_STRIDE`），聚合残差记录。返回 (全聚合残差, 各窗独立残差 for time block 报告)。
fn walk_forward_oos_residuals(
    symbol: &str,
    ds: &data::Dataset,
    cfg: &ThetaConfig,
) -> (Vec<ResidualTrade>, Vec<(u32, Vec<ResidualTrade>)>) {
    let sw = PREREG_WINDOWS
        .iter()
        .find(|w| w.symbol == symbol)
        .unwrap_or_else(|| panic!("{symbol} 不在 PREREG_WINDOWS（预注册缺口，非静默兜底）"));
    let mut agg: Vec<ResidualTrade> = Vec::new();
    let mut time_blocks: Vec<(u32, Vec<ResidualTrade>)> = Vec::new();
    for win in sw.wf_anchored {
        if win.test_start < OOS_START {
            continue; // 窗测的是 IS 期，不算样本外证据
        }
        let test_ds = ds.slice_date_window(win.test_start, win.test_end);
        if test_ds.bars.is_empty() {
            continue;
        }
        let (_est, records) = build_mu_from_bars(&test_ds.bars, cfg, win.i * WF_TIME_STRIDE);
        agg.extend(records.iter().copied());
        eprintln!(
            "[wf-oos] win{} {}→{} bars={} residuals={}",
            win.i, win.test_start, win.test_end, test_ds.bars.len(), records.len()
        );
        time_blocks.push((win.i, records));
    }
    (agg, time_blocks)
}

#[test]
#[ignore]
fn wverify_full() {
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let oos_sanity = ds.slice_date_window("2023-01-01", "2025-06-30"); // 数据漂移哨兵；协议 §2.1 BTC OOS
    assert!(!oos_sanity.bars.is_empty(), "OOS 窗空——数据漂移");
    // walk-forward OOS 残差聚合（G-A4）：逐窗 test 段独立估计，残差来自真样本外区间。
    let (records, time_blocks) = walk_forward_oos_residuals("BTC", &ds, &cfg);
    assert!(!records.is_empty(), "walk-forward OOS 聚合未产出残差——窗口/数据不匹配");

    // ── 残差桶 (ℓ, bsp, δ, σ^H) 的 Welford (n, mean, m2) on Y_i（§1 残差口径）+ 逐桶 Y 序列 ──
    let mut agg: BTreeMap<(u32, u8, i8, i8), (u64, f64, f64)> = BTreeMap::new();
    let mut series: BTreeMap<(u32, u8, i8, i8), Vec<f64>> = BTreeMap::new();
    for r in &records {
        let key = (r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir);
        let y = r.y();
        let e = agg.entry(key).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = y - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (y - e.1);
        series.entry(key).or_default().push(y);
    }
    // 残差分层 δ 置换逐桶 perm_p（§4.2 分层键 (ℓ,h桶,time block,σ^H)；N_PERM=200 种子 20260701 冻结）。
    let pp = perm_test::stratified_delta_perm_p(&records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::from(
        "| L | bsp | δ | σ^H | n | n_eff | mean(Y) | lcb | ucb | cv | perm_p | 删尾mean(−3) | 翻转 | state |\n",
    );
    for (&(lv, bc, d, pd), &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&(lv, bc, d, pd)).unwrap_or(&1.0);
        let ys = series.get(&(lv, bc, d, pd)).map(Vec::as_slice).unwrap_or(&[]);
        let n_eff = decontam::effective_n(ys); // 665 neff/nraw 事件聚集校正
        let (trim_mean, flipped) = perm_test::drop_top_k_mean(ys, TRIM_K); // 删尾稳健（§6）
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        rows.push_str(&format!(
            "| L{} | {} | {:+} | σ{:+} | {} | {:.2} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:.6} | {} | {:?} |\n",
            lv, bc, d, pd, n, n_eff, mean, lcb, ucb, cv, perm_p, trim_mean, flipped, st
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    let mut lv_set: Vec<u32> = agg.keys().map(|k| k.0).collect();
    lv_set.sort();
    lv_set.dedup();

    // ── H2 方向不对称（alpha检验.pdf §7-§8）：逐级别 β=μ_sell−μ_buy block bootstrap 单边 p ──
    let mut h2_rows = String::from("| L | n_buy | n_sell | mean_buy(Y) | mean_sell(Y) | β=μ_sell−μ_buy | boot_p(H0:β≤0) |\n");
    for &lv in &lv_set {
        let buy: Vec<f64> = records.iter().filter(|r| r.class.level == lv && r.class.delta == 1).map(|r| r.y()).collect();
        let sell: Vec<f64> = records.iter().filter(|r| r.class.level == lv && r.class.delta == -1).map(|r| r.y()).collect();
        if buy.is_empty() || sell.is_empty() {
            continue;
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let (beta, p) = perm_test::direction_asymmetry_beta_pvalue(&buy, &sell, 1000, 20, perm_test::PERM_SEED);
        h2_rows.push_str(&format!(
            "| L{} | {} | {} | {:.6} | {:.6} | {:.6} | {:.4} |\n",
            lv, buy.len(), sell.len(), mean(&buy), mean(&sell), beta, p
        ));
    }

    eprintln!(
        "WV_FULL residuals={} buckets={} verdict={:?} V={} F={} I={} levels={:?} wf_windows={}",
        records.len(), agg.len(), v, nv, nf, ni, lv_set, time_blocks.len()
    );
    std::fs::write("/tmp/wv_full_rows.md", &rows).ok();
    std::fs::write("/tmp/wv_full_h2_asymmetry.md", &h2_rows).ok();

    // time block 报告（G-A2）：逐窗独立分桶残差均值，σ^H + 窗序号(time block)（h桶已并入 perm 分层）。
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC 在 PREREG_WINDOWS");
    let mut wf_rows = String::from("| window_i | test_start | test_end | L | bsp | δ | σ^H | n | mean(Y) |\n");
    for (wi, recs) in &time_blocks {
        let mut bucket: BTreeMap<(u32, u8, i8, i8), (u64, f64)> = BTreeMap::new();
        for r in recs {
            let e = bucket.entry((r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir)).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += r.y();
        }
        let win = sw.wf_anchored.iter().find(|w| w.i == *wi).expect("窗序号来自同一 wf_anchored 序列");
        for (&(lv, bc, d, pd), &(n, sum)) in &bucket {
            wf_rows.push_str(&format!(
                "| {} | {} | {} | L{} | {} | {:+} | σ{:+} | {} | {:.6} |\n",
                wi, win.test_start, win.test_end, lv, bc, d, pd, n, sum / n as f64
            ));
        }
    }
    std::fs::write("/tmp/wv_full_timeblocks.md", &wf_rows).ok();

    assert!(!records.is_empty(), "residuals 空——管线未产观测");
}
