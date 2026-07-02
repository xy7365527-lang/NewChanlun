//! W-VERIFY acc-alpha 全定义域跑批（真实 BTC 全量，walk-forward OOS，锚 d906648041 段2完整实装）。
//! `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture`。
//! 数据=btc_1m_full.json（461万 bar）；μ 估计覆盖**全 OOS 定义域**（2023-01-01…2025-06-30，无 MAX_BARS
//! 截断——覆盖全定义域，区别 L3 的 32K 有效域边界；231号：全域结论非截断结论）。
//!
//! ## a0 盘点 G-A1/A2/A4 实装（task #51）
//!
//! - **G-A1**（σ^H 桶键）：逐笔投影加 `parent_dir`（父声部方向 σ^H）维——[`MuClass`] 本已携带，
//!   此前只在投影到 `(ℓ,bsp_class,δ)` 三元组时被丢弃。现桶键=`(ℓ,bsp_class,δ,σ^H)`，
//!   与 [`perm_test::BucketKey`] 同构。
//! - **G-A4**（walk-forward LCB）：`est` 不再由整个 OOS 窗单块 `build_mu_from_bars` 估计（那是
//!   in-sample 正态近似——SE 用的是"同一批数据自己"的方差），而是消费 `prereg_windows.rs`
//!   冻结的 BTC anchored walk-forward 窗口：只取 `test_start` 落在 OOS 起点之后的窗口，每窗
//!   独立对 **test 段**估计（`build_mu_from_bars` 作用于该窗 test 切片），再逐笔 `observe` 累积
//!   （**非 `merge`**——`MuEstimator::merge` 只合并 Welford 桶不携带逐笔 `trades`，而下游需 `trades()`）
//!   ——不拼接 Dataset 防接缝伪相邻。
//!   聚合后的 `est.trades()` 里每一笔观测都来自"该窗训练截止之后"的样本外区间，LCB 由此
//!   变成真正的 walk-forward 样本外统计，而非单窗 in-sample 近似。
//! - **G-A2**（分层报告，部分）：报告加 σ^H 列（本模块）+ 按 walk-forward 窗口的 time block
//!   分解（`/tmp/wv_full_timeblocks.md`）。**诚实缺口**：持有桶 h（holding bucket）需要逐笔
//!   持仓时长数据，当前 `MuClass`/`trades()` 管线不携带该字段（构造在 l3_delta_r_alpha.rs，
//!   超出本工位文件域 perm_test.rs/wverify_run.rs/mu_estimator.rs/prereg_windows.rs）——
//!   未实装，非遗漏，留给独立工位在 l3_delta_r_alpha.rs 接通 entry/exit bar 差值。

use super::super::config::ThetaConfig;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::mu_estimator::{MuEstimator, MuObservation};
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::BTreeMap;

/// walk-forward OOS 聚合（G-A4）：取 `symbol` 在 `PREREG_WINDOWS` 冻结的 anchored 窗口中
/// `test_start ≥ OOS_START` 的子集（落在 OOS 起点前的窗测的是 IS 期，非 OOS 证据），逐窗对
/// **test 段**独立 `build_mu_from_bars` 后 `merge` 聚合。返回聚合估计器 + 各窗独立估计器
/// （后者供调用方产出 time block 报告，G-A2）。
///
/// anchored（非 rolling）模式：prereg_windows.rs §W2 已裁定 anchored train 扩张窗不产生拟合
/// 自由度（Θ v0 无参数），故用 anchored 序列即可覆盖 OOS，无需额外消费 rolling 序列。
fn walk_forward_oos_mu(symbol: &str, ds: &data::Dataset, cfg: &ThetaConfig) -> (MuEstimator, Vec<(u32, MuEstimator)>) {
    let sw = PREREG_WINDOWS
        .iter()
        .find(|w| w.symbol == symbol)
        .unwrap_or_else(|| panic!("{symbol} 不在 PREREG_WINDOWS（预注册缺口，非静默兜底）"));
    let mut agg = MuEstimator::new();
    let mut time_blocks: Vec<(u32, MuEstimator)> = Vec::new();
    for win in sw.wf_anchored {
        if win.test_start < OOS_START {
            continue; // 窗测的是 IS 期（train/test 划分早于协议 OOS 边界），不算样本外证据
        }
        let test_ds = ds.slice_date_window(win.test_start, win.test_end);
        if test_ds.bars.is_empty() {
            continue;
        }
        let est_i = build_mu_from_bars(&test_ds.bars, cfg);
        // 逐笔 `observe` 累积（非 `merge`）：`MuEstimator::merge` 契约只合并 Welford 桶、不携带逐笔
        // `trades`（cross-fit 只需桶聚合），而下游从 `est.trades()` 做逐桶 perm_test 投影 + Welford 再
        // 分桶——必须要逐笔记录。用 `observe` 同时正确维护 buckets+trades，跨窗拼接保各笔不变（perm_test
        // 层内 δ 置换/再聚合皆序无关，接缝不产生虚假相邻笔）。
        for (c, x) in est_i.trades() {
            agg.observe(MuObservation { class: *c, x_gamma: *x });
        }
        eprintln!("[wf-oos] win{} {}→{} bars={} trades={}", win.i, win.test_start, win.test_end, test_ds.bars.len(), est_i.trades().len());
        time_blocks.push((win.i, est_i));
    }
    (agg, time_blocks)
}

#[test]
#[ignore]
fn wverify_full() {
    let cfg = ThetaConfig::default();
    // 真实 API：load_by_symbol（SYMBOLS 表 → btc_1m_full.json 461万 bar）。
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let oos_sanity = ds.slice_date_window("2023-01-01", "2025-06-30"); // 数据漂移哨兵；协议 §2.1 BTC OOS
    assert!(!oos_sanity.bars.is_empty(), "OOS 窗空——数据漂移");
    // walk-forward OOS 聚合（G-A4）：逐窗 test 段独立估计 + merge，LCB 基于真样本外分布。
    let (est, time_blocks) = walk_forward_oos_mu("BTC", &ds, &cfg);
    assert!(!est.trades().is_empty(), "walk-forward OOS 聚合未产出观测——窗口/数据不匹配");
    // 逐笔 (ℓ, bsp_class, δ, σ^H, X_γ) 投影（G-A1：加 parent_dir 维，perm_test 输入 + Welford 聚合键）。
    let trades: Vec<(u32, u8, i8, i8, f64)> = est
        .trades()
        .iter()
        .map(|(c, x)| (c.level, c.bsp_class(), c.delta, c.parent_dir, *x))
        .collect();
    // 逐桶 Welford 聚合 (n, mean, m2)（BTreeMap 确定序）。
    let mut agg: BTreeMap<(u32, u8, i8, i8), (u64, f64, f64)> = BTreeMap::new();
    for &(lv, bc, d, pd, x) in &trades {
        let e = agg.entry((lv, bc, d, pd)).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = x - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (x - e.1);
    }
    // 路径 A 分层内 δ 置换逐桶 perm_p（预注册 §1.4，N_PERM=200 种子 20260701 冻结；层键含 σ^H）。
    let pp = perm_test::stratified_delta_perm_p(&trades, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::from("| L | bsp | δ | σ^H | n | n_eff | mean | lcb | ucb | cv | perm_p | state |\n");
    // 逐桶按成交时间序收集 X_γ 序列，供事件聚集/自相关 n_eff 校正（预注册 §1.3，665 neff/nraw 口径）。
    let mut series: BTreeMap<(u32, u8, i8, i8), Vec<f64>> = BTreeMap::new();
    for &(lv, bc, d, pd, x) in &trades {
        series.entry((lv, bc, d, pd)).or_default().push(x);
    }
    for (&(lv, bc, d, pd), &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&(lv, bc, d, pd)).unwrap_or(&1.0);
        // 三态判据（预注册 §3）：n_eff = 事件聚集/自相关校正后有效样本（665 neff/nraw 口径）。
        let n_eff = decontam::effective_n(series.get(&(lv, bc, d, pd)).map(Vec::as_slice).unwrap_or(&[]));
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        rows.push_str(&format!(
            "| L{} | {} | {:+} | σ{:+} | {} | {:.2} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:?} |\n",
            lv, bc, d, pd, n, n_eff, mean, lcb, ucb, cv, perm_p, st
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
        "WV_FULL trades={} buckets={} verdict={:?} V={} F={} I={} levels={:?} wf_windows={}",
        trades.len(), agg.len(), v, nv, nf, ni, lv_set, time_blocks.len()
    );
    std::fs::write("/tmp/wv_full_rows.md", &rows).ok();

    // time block 报告（G-A2 部分）：逐窗独立分桶均值，σ^H + 窗序号(time block)，无持有桶 h（诚实缺口，见模块文档）。
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC 在 PREREG_WINDOWS");
    let mut wf_rows = String::from("| window_i | test_start | test_end | L | bsp | δ | σ^H | n | mean |\n");
    for (wi, est_i) in &time_blocks {
        let mut bucket: BTreeMap<(u32, u8, i8, i8), (u64, f64)> = BTreeMap::new();
        for (c, x) in est_i.trades() {
            let e = bucket.entry((c.level, c.bsp_class(), c.delta, c.parent_dir)).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += x;
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

    assert!(!trades.is_empty(), "trades 空——管线未产观测");
}
