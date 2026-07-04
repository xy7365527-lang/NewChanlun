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
use super::mu_estimator::{MuClass, ResidualTrade, UClass};
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::{BTreeMap, HashMap};

/// walk-forward 窗间 time block 偏移步长（§4.2：不同窗的 time block 不碰撞；窗内块 <stride）。
const WF_TIME_STRIDE: u32 = 10_000;
/// 品种间 time block 偏移步长（跨标的 L3 预注册 §3）：使不同品种的 time_block 值域两两不相交
/// ⟹ 置换 stratum 天然按品种隔离（perm_test 分层键含 time_block），跨品种 shuffle δ 不混 beta。
/// 单品种值域 ≤ 15 窗·WF_TIME_STRIDE + 窗内块 ≈ 150K < 10^7；7 品种·10^7 < u32::MAX（不溢出）。
const SYMBOL_STRIDE: u32 = 10_000_000;
/// 跨标的 L3 池化域（预注册冻结 §1.2）：7 品种，OKLO 剔除（Observation 池 wf_anchored=[]，无 walk-forward OOS）。
const L3_UNIVERSE: &[&str] = &["BTC", "ES", "CL", "GC", "BRN", "DX", "QQQ"];
/// 删尾稳健删除的赢家数（alpha检验.pdf §6：删前 3 最大赢家）。
const TRIM_K: usize = 3;
/// 归一化 σ̂ 估计的 pre-OOS 可交易 bar 下限（prereg-l3norm-20260703 §1）：不足则诚实剔除，不静默兜底。
const SIGMA_MIN_BARS: usize = 1000;

/// 连续价的逐 bar 差分样本标准差（σ̂ 计算核心，money path）。`pxs` = 时间序可交易价序列。
fn stdev_consecutive_diffs(pxs: &[f64]) -> f64 {
    let diffs: Vec<f64> = pxs.windows(2).map(|w| w[1] - w[0]).collect();
    let n = diffs.len();
    if n < 2 {
        return 0.0;
    }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let var = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    var.sqrt()
}

/// 品种归一化尺度 σ̂_symbol（prereg-l3norm-20260703 §1，跨品种残差通约）：pre-OOS（date<OOS_START）
/// 全体可交易 bar 逐 bar close-to-close $ 涨跌的样本 std。**因果无前视**：池化残差 entry 全 ≥OOS_START，
/// σ̂ 由严格早于 OOS 的数据估 ⟹ F_entry-可测。返回 (σ̂, pre-OOS 可交易 bar 数)。
fn sigma_pre_oos(ds: &data::Dataset, cfg: &ThetaConfig) -> (f64, usize) {
    let pre = ds.slice_date_window("1900-01-01", "2022-12-31"); // 严格早于 OOS_START=2023-01-01（闭区间）
    let pxs: Vec<f64> = pre
        .bars
        .iter()
        .filter(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * cfg.tick.tick_size)
        .collect();
    (stdev_consecutive_diffs(&pxs), pxs.len())
}

/// walk-forward OOS 残差聚合（G-A4）：取 `symbol` 在 `PREREG_WINDOWS` 冻结 anchored 窗口中
/// `test_start ≥ OOS_START` 的子集，逐窗对 **test 段** 独立 [`build_mu_from_bars`]（time_block_base
/// = `win.i·WF_TIME_STRIDE`），聚合残差记录。返回 (全聚合残差, 各窗独立残差 for time block 报告)。
fn walk_forward_oos_residuals(
    symbol: &str,
    symbol_index: u32,
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
        // time_block_base = 品种偏移 + 窗偏移（跨品种 stratum 隔离，预注册 §3）。BTC 单标的 index 0 ⟹ bit-exact。
        let (_est, records) =
            build_mu_from_bars(&test_ds.bars, cfg, symbol_index * SYMBOL_STRIDE + win.i * WF_TIME_STRIDE);
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
    let (records, time_blocks) = walk_forward_oos_residuals("BTC", 0, &ds, &cfg);
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

/// 逐桶统计（跨标的 L3 复用：per-symbol 与池化同一口径）。
struct BucketStat {
    n: u64,
    n_eff: f64,
    mean: f64,
    lcb: f64,
    ucb: f64,
    cv: f64,
    perm_p: f64,
    state: decontam::AlphaState,
}

/// 残差记录 → 逐桶 (ℓ,bsp,δ,σ^H) 三态判定（与 wverify_full 同口径：Welford + 残差分层 δ 置换 + decontam）。
/// 返回 (逐桶统计 map, markdown 表, 全局裁决 debug 串, (V,F,I) 计数)。frontier 列标注 ℓ≥2（预注册 §4.1；
/// 污染已于 c546b5633c 修复解除，见列内字符串）。
fn bucket_verdict(records: &[ResidualTrade]) -> (BTreeMap<(u32, u8, i8, i8), BucketStat>, String, String, (usize, usize, usize)) {
    let mut agg: BTreeMap<(u32, u8, i8, i8), (u64, f64, f64)> = BTreeMap::new();
    let mut series: BTreeMap<(u32, u8, i8, i8), Vec<f64>> = BTreeMap::new();
    for r in records {
        let key = (r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir);
        let y = r.y();
        let e = agg.entry(key).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = y - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (y - e.1);
        series.entry(key).or_default().push(y);
    }
    let pp = perm_test::stratified_delta_perm_p(records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut stats: BTreeMap<(u32, u8, i8, i8), BucketStat> = BTreeMap::new();
    let mut states = Vec::new();
    let mut rows = String::from("| L | bsp | δ | σ^H | n | n_eff | mean(Y) | lcb | ucb | cv | perm_p | state | frontier(≥2) |\n");
    for (&(lv, bc, d, pd), &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&(lv, bc, d, pd)).unwrap_or(&1.0);
        let ys = series.get(&(lv, bc, d, pd)).map(Vec::as_slice).unwrap_or(&[]);
        let n_eff = decontam::effective_n(ys);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        let frontier = if lv >= 2 { "frontier已修(c546b5633c bit-exact)，污染标注解除" } else { "—" };
        rows.push_str(&format!(
            "| L{} | {} | {:+} | σ{:+} | {} | {:.2} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:?} | {} |\n",
            lv, bc, d, pd, n, n_eff, mean, lcb, ucb, cv, perm_p, st, frontier
        ));
        stats.insert((lv, bc, d, pd), BucketStat { n, n_eff, mean, lcb, ucb, cv, perm_p, state: st });
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    (stats, rows, format!("{v:?}"), (nv, nf, ni))
}

/// 跨标的 L3 池化 alpha 重测（预注册 prereg-l3-cross-symbol-20260702，codex #85 裁 C）。
/// 7 品种（L3_UNIVERSE）逐品种 walk-forward OOS 残差 → per-symbol 表 + 拼接池化表 + 与 BTC 单标的差分。
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::wverify_cross_symbol -- --ignored --nocapture`。
#[test]
#[ignore]
fn wverify_cross_symbol() {
    let cfg = ThetaConfig::default();
    const MAIN: (u32, u8, i8, i8) = (0, 3, 1, 0); // 主桶 L0/type3/买/σ0（BTC 单标的欠功效点，n_eff=166<290）

    let mut pooled: Vec<ResidualTrade> = Vec::new();
    let mut per_symbol_md = String::new();
    let mut sigma_md = String::from("| symbol | σ̂_symbol | pre-OOS bars |\n|---|---|---|\n");
    let mut btc_main: Option<BucketStat> = None;
    for (idx, &sym) in L3_UNIVERSE.iter().enumerate() {
        let ds = data::load_by_symbol(sym, &cfg).unwrap_or_else(|e| panic!("{sym} 数据加载失败：{e}"));
        let (mut recs, _tb) = walk_forward_oos_residuals(sym, idx as u32, &ds, &cfg);
        // ── σ̂ 归一化（prereg-l3norm-20260703）：Ỹ=Y/σ̂，缩放 resid_base+cost 两字段 ⟹ y()=δ·resid_base−cost 自动归一化。
        //    perm_test/decontam/bucket_verdict 透明消费归一化 records（零改动 bit-exact）。σ̂ 因果无前视（见 sigma_pre_oos）。
        let (sigma, n_pre) = sigma_pre_oos(&ds, &cfg);
        assert!(sigma > 0.0 && n_pre >= SIGMA_MIN_BARS, "{sym} σ̂ 不可估（σ̂={sigma} pre-OOS bars={n_pre}<{SIGMA_MIN_BARS}）——诚实剔除，不静默兜底");
        sigma_md.push_str(&format!("| {sym} | {sigma:.6} | {n_pre} |\n"));
        for r in &mut recs {
            r.resid_base /= sigma;
            r.cost /= sigma;
        }
        eprintln!("[xsym] {sym} residuals={} σ̂={sigma:.6} pre_bars={n_pre}", recs.len());
        let (mut stats, rows, verdict, (nv, nf, ni)) = bucket_verdict(&recs);
        per_symbol_md.push_str(&format!("\n### {sym}（residuals={}, verdict={verdict} V={nv}/F={nf}/I={ni}）\n\n{rows}", recs.len()));
        if sym == "BTC" {
            btc_main = stats.remove(&MAIN);
        }
        pooled.extend(recs);
    }
    assert!(!pooled.is_empty(), "池化残差空——7 品种全无产出（数据/窗口不匹配）");

    let (pstats, prows, pverdict, (nv, nf, ni)) = bucket_verdict(&pooled);
    let pooled_main = pstats.get(&MAIN);

    // 与 BTC 单标的主桶差分（裁定 C 目标：池化增 n 是否使主桶 INCONCLUSIVE→VALIDATED）。
    let diff = match (btc_main.as_ref(), pooled_main) {
        (Some(b), Some(p)) => format!(
            "主桶 {MAIN:?} 差分（σ̂-归一化 Ỹ 单位）：\n  BTC 单标的 : n={} n_eff={:.2} mean={:+.4} lcb={:+.4} perm_p={:.3} → {:?}\n  7 品种池化 : n={} n_eff={:.2} mean={:+.4} lcb={:+.4} perm_p={:.3} → {:?}\n  n_eff {:.0}→{:.0}（功效门=(1.645·CV)²逐桶重算，不复用 v1 的 290），翻转={}",
            b.n, b.n_eff, b.mean, b.lcb, b.perm_p, b.state,
            p.n, p.n_eff, p.mean, p.lcb, p.perm_p, p.state,
            b.n_eff, p.n_eff, if format!("{:?}", b.state) != format!("{:?}", p.state) { "是" } else { "否" },
        ),
        _ => "主桶差分不可算（BTC 或池化缺主桶）".to_string(),
    };

    eprintln!("WV_XSYM(σ̂-归一化) pooled_residuals={} buckets={} verdict={pverdict} V={nv} F={nf} I={ni}\n{sigma_md}\n{diff}", pooled.len(), pstats.len());
    std::fs::write("/tmp/wv_xsym_sigma.md", &sigma_md).ok();
    std::fs::write("/tmp/wv_xsym_persymbol.md", &per_symbol_md).ok();
    std::fs::write(
        "/tmp/wv_xsym_pooled.md",
        format!("# 池化（7 品种，σ̂-归一化 Ỹ）verdict={pverdict} V={nv}/F={nf}/I={ni}\n\n## σ̂ 表\n\n{sigma_md}\n\n{prows}\n\n## {diff}\n"),
    )
    .ok();
}

/// 逐桶三态判定（泛化键 K）——消费预算好的 perm_p map。full-z（K=MuClass）/ UClass（K=UClass）复用。
/// `BTreeMap<K>` 确定序报告；`level_of` 提供 frontier 标注的级别（污染已于 c546b5633c 修复解除；UClass 无干净级别 ⟹ None）。
fn verdict_by<K: Ord + Copy + std::fmt::Debug + std::hash::Hash>(
    records: &[ResidualTrade],
    key_of: impl Fn(&MuClass) -> K,
    perm: &HashMap<K, f64>,
    level_of: impl Fn(&K) -> Option<u32>,
) -> (String, String, (usize, usize, usize)) {
    let mut agg: BTreeMap<K, (u64, f64, f64)> = BTreeMap::new();
    let mut series: BTreeMap<K, Vec<f64>> = BTreeMap::new();
    for r in records {
        let key = key_of(&r.class);
        let y = r.y();
        let e = agg.entry(key).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = y - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (y - e.1);
        series.entry(key).or_default().push(y);
    }
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::from("| key | n | n_eff | mean(Y) | lcb | ucb | cv | perm_p | state | frontier(≥2) |\n");
    for (key, &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *perm.get(key).unwrap_or(&1.0);
        let ys = series.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let n_eff = decontam::effective_n(ys);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        let frontier = match level_of(key) {
            Some(l) if l >= 2 => "frontier已修(c546b5633c bit-exact)，污染标注解除",
            _ => "—",
        };
        rows.push_str(&format!(
            "| {key:?} | {n} | {n_eff:.2} | {mean:.6} | {lcb:.6} | {ucb:.6} | {cv:.3} | {perm_p:.3} | {st:?} | {frontier} |\n"
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    (rows, format!("{v:?}"), (nv, nf, ni))
}

/// full-z×残差逐桶判定（prereg-fullz-policy 阶段2 (A)）：BTC 单标的 walk-forward OOS，桶键=MuClass
/// 8 维（force_state 第 8 维；σ_higher 第 9 维**不进** prereg 桶键——codex-q1 G2 裁定留待新 prereg，
/// 判定键与 perm 表同投影 sigma_higher→None，防 records Some(v) vs 表键 None 的全表 miss）
/// + UClass 降维并列（桶碎裂防护）。`#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::wverify_fullz -- --ignored --nocapture`。
#[test]
#[ignore]
fn wverify_fullz() {
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let (records, _tb) = walk_forward_oos_residuals("BTC", 0, &ds, &cfg);
    assert!(!records.is_empty(), "walk-forward OOS 残差空——窗口/数据不匹配");

    // ★A6（#159）fill loop 侧 force_state 探针：records 经 fill loop z_of_candidate 现携
    // Candidate.force 真值——一类交易（i_class 含 buy1|sell1，bit0|bit3）必有 A/C 段配对 ⟹
    // force_state 须 Some。透传断裂（fullz 置换 force_state 恒 None）在此 fire。
    // 零一类交易的窗不假失败（force 仅一类 A/C 对候选有源，与 econ #115 (e) 同口径）。
    let n_type1_rec = records.iter().filter(|r| r.class.i_class & 0b001_001 != 0).count();
    let n_force_some_rec = records.iter().filter(|r| r.class.force_state.is_some()).count();
    assert!(
        n_type1_rec == 0 || n_force_some_rec > 0,
        "A6 透传断裂：fullz 置换 records 有 {n_type1_rec} 条一类交易但 force_state 全 None"
    );
    eprintln!("WV_FULLZ force probe: type1={n_type1_rec} force_some={n_force_some_rec}（A6 #159：一类>0 ⟹ force 非全 None）");

    // full-z（MuClass 8 维，horizontal=Some 走生产路径；σ_higher 投影 None 与 perm 表键同口径——G2）。
    let pf = perm_test::stratified_delta_perm_p_fullz(&records, perm_test::N_PERM, perm_test::PERM_SEED);
    // G3 第 10-13 维同投影（#138）：records 侧 risk_mode=Some(bar 真值)/origin_level=Some(level)，
    // perm 表键侧四维显式 None——判定键必须同投影，否则重演 G2 修过的全表 miss。
    let fullz_key = |c: &MuClass| MuClass {
        sigma_higher: None,
        cand_channel: None,
        nest_depth: None,
        origin_level: None,
        risk_mode: None,
        t_stage: None, // #149 第 14 维同投影（records 侧 Some(bar 相位)，键侧 None——同 risk_mode）
        eta_bucket: None, // #175 第 15 维同投影（records 侧 Some(bar γ_t)，键侧 None——同 t_stage）
        ..*c
    };
    let (frows, fverdict, (fv, ff, fi)) = verdict_by(&records, fullz_key, &pf, |k: &MuClass| Some(k.level));
    let n_fullz = { let mut s: Vec<MuClass> = records.iter().map(|r| fullz_key(&r.class)).collect(); s.sort(); s.dedup(); s.len() };

    // UClass 降维并列（(level_bucket,δ,role,divergence)，抗碎裂）。
    let pu = perm_test::stratified_delta_perm_p_uclass(&records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (urows, uverdict, (uv, uf, ui)) = verdict_by(&records, UClass::project_to_u, &pu, |_k: &UClass| None);
    let n_uclass = { let mut s: Vec<UClass> = records.iter().map(|r| UClass::project_to_u(&r.class)).collect(); s.sort(); s.dedup(); s.len() };

    eprintln!(
        "WV_FULLZ residuals={} | full-z: buckets={n_fullz} verdict={fverdict} V={fv}/F={ff}/I={fi} | UClass: buckets={n_uclass} verdict={uverdict} V={uv}/F={uf}/I={ui}",
        records.len()
    );
    std::fs::write("/tmp/wv_fullz_rows.md", format!("# full-z（MuClass 8 维，σ_higher 不进 prereg 桶键）verdict={fverdict} V={fv}/F={ff}/I={fi}\n\n{frows}")).ok();
    std::fs::write("/tmp/wv_uclass_rows.md", format!("# UClass 降维并列 verdict={uverdict} V={uv}/F={uf}/I={ui}\n\n{urows}")).ok();
}

/// 完整策略级 π 回测（prereg-fullz-policy 阶段2 (B)）：BTC+CL，χ门μ̂注入 vs 无χ基线。
/// est 由 **train 段** `build_mu_from_bars` 建（无 in-sample 泄漏，L2），test 段跑生产 π；输出 Σpnl(含浮盈)
/// / max_drawdown / n_orders 的 χ-vs-基线差分（μ̂ 选择器增量价值）。
/// ponytail: 单折有界 train（18 月，OOS 前）——限 est-build 成本；若 μ̂ 增量信号需更细 OOS 鲁棒性，
/// 改逐 wf_anchored 窗 walk-forward（成本随窗数×train 扩张增长）。
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::policy_backtest -- --ignored --nocapture`。
#[test]
#[ignore]
fn policy_backtest() {
    use super::runner::{run_theta_v0_pi, run_theta_v0_pi_chi};
    let base_cfg = ThetaConfig::default();
    let mut chi_cfg = ThetaConfig::default();
    chi_cfg.risk.chi_theta = Some(0.0); // χ=1[LCB(μ)>0]（p8-9 选择器）
    chi_cfg.risk.chi_z_alpha = 1.645;
    let (train_lo, train_hi) = ("2022-07-01", "2022-12-31"); // OOS 前 6 月（有界 train，限跑批成本）
    let (test_lo, test_hi) = ("2023-01-01", "2023-06-30"); // OOS 6 月单折（ponytail 有界；逐窗 walk-forward 是升级路径）

    let mut report = String::from(
        "# 策略级 π 回测（BTC+CL，χ门μ̂ vs 无χ基线，单折有界 train 6月→OOS 6月）\n\n\
         口径：Σpnl=含浮盈已实现（trade_pnls_with_forced）；max_dd=metrics.max_drawdown；nav=首可交易价×1000。\n\n",
    );
    for sym in ["BTC", "CL"] {
        let ds = match data::load_by_symbol(sym, &base_cfg) {
            Ok(d) => d,
            Err(e) => { report.push_str(&format!("## {sym}\n加载失败：{e}\n\n")); continue; }
        };
        let train_ds = ds.slice_date_window(train_lo, train_hi);
        let test_ds = ds.slice_date_window(test_lo, test_hi);
        if train_ds.bars.is_empty() || test_ds.bars.is_empty() {
            report.push_str(&format!("## {sym}\ntrain/test 段空（数据不覆盖窗口）\n\n"));
            continue;
        }
        eprintln!("[policy] {sym} train_bars={} test_bars={} 建 est…", train_ds.bars.len(), test_ds.bars.len());
        let (est, _r) = build_mu_from_bars(&train_ds.bars, &base_cfg, 0);
        let first_px = test_ds.bars.iter().find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * base_cfg.tick.tick_size).unwrap_or(1.0);
        let nav = first_px * 1000.0;
        let years = test_ds.bars.len() as f64 / (365.25 * 24.0 * 60.0);

        let chi_r = run_theta_v0_pi_chi(&test_ds, &chi_cfg, years, nav, &est, false);
        let base_r = run_theta_v0_pi(&test_ds, &base_cfg, years, nav);
        // f3-C 多重赋格反事实：剔 ShortDiff 声部（同 no-χ 口径，margin=None 默认），测对冲增量价值。
        let mut sd_off_cfg = base_cfg.clone();
        sd_off_cfg.voice.disable_shortdiff = true;
        let sd_off_r = run_theta_v0_pi(&test_ds, &sd_off_cfg, years, nav);
        let sum = |v: &[f64]| v.iter().sum::<f64>();
        let (chi_pnl, base_pnl, sd_off_pnl) = (sum(&chi_r.trade_pnls_with_forced), sum(&base_r.trade_pnls_with_forced), sum(&sd_off_r.trade_pnls_with_forced));
        report.push_str(&format!(
            "## {sym}（μ̂ 桶数={}, test_bars={}）\n\
             - χ门 μ̂  : Σpnl={chi_pnl:+.2} max_dd={:.4} n_orders={} n_trades={} strat_return={:+.4}\n\
             - 无χ基线: Σpnl={base_pnl:+.2} max_dd={:.4} n_orders={} n_trades={} strat_return={:+.4}\n\
             - 差分   : ΔΣpnl={:+.2} Δmax_dd={:+.4} Δn_orders={}（μ̂ 门增量价值）\n\
             - **f3-C 反事实**（全赋格 vs 剔 ShortDiff，同 no-χ 口径）:\n\
             &nbsp;&nbsp;全赋格(=无χ基线): Σpnl={base_pnl:+.2} max_dd={:.4} n_orders={}\n\
             &nbsp;&nbsp;剔 ShortDiff    : Σpnl={sd_off_pnl:+.2} max_dd={:.4} n_orders={}\n\
             &nbsp;&nbsp;ShortDiff 增量  : ΔΣpnl={:+.2} Δmax_dd={:+.4} Δn_orders={}（多重赋格对冲声部的组合级增量价值）\n\n",
            est.n_classes(), test_ds.bars.len(),
            chi_r.metrics.max_drawdown, chi_r.n_orders, chi_r.trade_pnls_with_forced.len(), chi_r.metrics.strat_return,
            base_r.metrics.max_drawdown, base_r.n_orders, base_r.trade_pnls_with_forced.len(), base_r.metrics.strat_return,
            chi_pnl - base_pnl, chi_r.metrics.max_drawdown - base_r.metrics.max_drawdown,
            chi_r.n_orders as i64 - base_r.n_orders as i64,
            base_r.metrics.max_drawdown, base_r.n_orders,
            sd_off_r.metrics.max_drawdown, sd_off_r.n_orders,
            base_pnl - sd_off_pnl, base_r.metrics.max_drawdown - sd_off_r.metrics.max_drawdown,
            base_r.n_orders as i64 - sd_off_r.n_orders as i64,
        ));
        eprintln!("POLICY {sym}: χ Σpnl={chi_pnl:+.2} dd={:.4} ord={} | base Σpnl={base_pnl:+.2} dd={:.4} ord={} | ΔΣpnl={:+.2} || f3C ShortDiff 增量 ΔΣpnl={:+.2} (剔后 Σpnl={sd_off_pnl:+.2} ord={})",
            chi_r.metrics.max_drawdown, chi_r.n_orders, base_r.metrics.max_drawdown, base_r.n_orders, chi_pnl - base_pnl,
            base_pnl - sd_off_pnl, sd_off_r.n_orders);
    }
    std::fs::write("/tmp/policy_backtest.md", &report).ok();
}

/// `d`（ISO "YYYY-MM-DD"）前推 6 个月，day 钳到 28（合法日期；slice_date_window 字典序比较）。
fn q4_shift_back_6m(d: &str) -> String {
    let y: i32 = d[0..4].parse().unwrap();
    let m: u32 = d[5..7].parse().unwrap();
    let day: u32 = d[8..10].parse().unwrap();
    let (y2, m2) = if m > 6 { (y, m - 6) } else { (y - 1, m + 6) };
    format!("{y2:04}-{m2:02}-{:02}", day.min(28))
}

/// `d` 的前一天（day=1 时取上月 28 日——窗口边界钳位，合法且不与相邻 test 窗重叠）。
fn q4_prev_day(d: &str) -> String {
    let day: u32 = d[8..10].parse().unwrap();
    if day > 1 {
        format!("{}-{:02}", &d[0..7], day - 1)
    } else {
        let y: i32 = d[0..4].parse().unwrap();
        let m: u32 = d[5..7].parse().unwrap();
        let (y2, m2) = if m > 1 { (y, m - 1) } else { (y - 1, 12u32) };
        format!("{y2:04}-{m2:02}-28")
    }
}

/// q4 margin 口径（prereg-q4-fullpi-20260703 §④，冻结）：CME-simple 单段近似全历史——
/// `cme_simple(0.37, 1.10)`（risk.rs L1 golden 同值）+ cushions `B1=0.02·nav₀, B2=0.05·nav₀`。
/// **有效域声明（231）**：非 SPAN、非交易所逐段历史快照、非实盘保证金——报告一律带「CME-simple」标签。
fn q4_margin_model(nav0: f64) -> super::super::strategy::risk::MarginModel {
    use super::super::strategy::risk::{MarginModel, MarginSchedule, MarginScheduleBook, RiskCushions};
    let sched = MarginSchedule::cme_simple(0.37, 1.10).expect("CME-simple 参数合法（冻结值）");
    let book = MarginScheduleBook::new(vec![(i64::MIN, i64::MAX, sched)]).expect("单段全域快照簿");
    let cushions = RiskCushions::new(0.02 * nav0, 0.05 * nav0).expect("0<B1<B2（冻结比例）");
    MarginModel { book, cushions }
}

/// q4 π^full 四臂 policy 回测（prereg-q4-fullpi-20260703 §⑥ R2，冻结 commit 388a9ebc16）。
///
/// 臂位（冻结）：Arm0=无χ/gross off/margin None（漂移归因）；Arm1=χ(teap=false)/gross **on**/
/// margin **CME-simple**（π^full 主臂）；Arm2=Arm1 但 teap=true（χ 空类语义敏感臂）；
/// Arm3=χ(teap=false)/gross off/margin None（隔离 G7+margin：Arm1−Arm3）。
/// 窗口：p3 可比单折（train 2022-07..12 → OOS 2023-01..06）+ walk-forward 逐窗（PREREG_WINDOWS
/// anchored，test_start≥OOS_START，train=test_start 前推 6 月有界）。品种 BTC 主判据 + CL 对照。
/// est 按臂自身生产口径训练（treatment-on-the-treated 的训练/生产分布一致；Arm1/Arm2 共享
/// fullpi-est，Arm3 用 plain-est，χ 不参与训练段 fill loop 故臂内一致）。
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::q4_fullpi_policy -- --ignored --nocapture`。
#[test]
#[ignore]
fn q4_fullpi_policy() {
    use super::runner::{run_theta_v0_pi, run_theta_v0_pi_chi, RunResult};

    let plain_cfg = ThetaConfig::default(); // Arm0/Arm3 基（margin=None, gross off）
    let mk_chi = |gross: bool| {
        let mut c = ThetaConfig::default();
        c.risk.chi_theta = Some(0.0); // χ=1[LCB(μ)>0]（§12 口径，z_α=1.645，非 p<0.05）
        c.risk.chi_z_alpha = 1.645;
        c.risk.enforce_gross_cap = gross;
        c
    };
    let nav_of = |ds: &data::Dataset, cfg: &ThetaConfig| {
        ds.bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * cfg.tick.tick_size)
            .unwrap_or(1.0)
            * 1000.0
    };
    let fmt_arm = |name: &str, r: &RunResult| {
        let pnl: f64 = r.trade_pnls_with_forced.iter().sum();
        format!(
            "| {name} | {pnl:+.2} | {:.4} | {} | {} | {:+.4} |\n",
            r.metrics.max_drawdown, r.n_orders, r.trade_pnls_with_forced.len(), r.metrics.strat_return
        )
    };

    let mut report = String::from(
        "# q4 π^full 四臂 policy 回测（prereg-q4-fullpi-20260703 §⑥，margin=CME-simple 单段近似）\n\n\
         口径：Σpnl=含浮盈（trade_pnls_with_forced）；nav₀=窗首可交易价×1000（p3 同源）。\n\n",
    );
    for sym in ["BTC", "CL"] {
        let ds = match data::load_by_symbol(sym, &plain_cfg) {
            Ok(d) => d,
            Err(e) => {
                report.push_str(&format!("## {sym}\n加载失败：{e}\n\n"));
                continue;
            }
        };
        // 窗清单：p3 可比单折 + anchored walk-forward（test_start≥OOS_START，train=前推 6 月）。
        let mut wins: Vec<(String, String, String, String, String)> = vec![(
            "p3fold".into(), "2022-07-01".into(), "2022-12-31".into(), "2023-01-01".into(), "2023-06-30".into(),
        )];
        if let Some(sw) = PREREG_WINDOWS.iter().find(|w| w.symbol == sym) {
            for w in sw.wf_anchored.iter().filter(|w| w.test_start >= OOS_START) {
                wins.push((
                    format!("wf{}{}", w.i, if w.clipped { "*" } else { "" }),
                    q4_shift_back_6m(w.test_start), q4_prev_day(w.test_start),
                    w.test_start.into(), w.test_end.into(),
                ));
            }
        }
        // 聚合器：wf 窗（不含 p3fold）四臂 Σpnl / n_orders。
        let mut agg: BTreeMap<&'static str, (f64, u64)> = BTreeMap::new();
        for (tag, tr_lo, tr_hi, te_lo, te_hi) in &wins {
            let train = ds.slice_date_window(tr_lo, tr_hi);
            let test = ds.slice_date_window(te_lo, te_hi);
            if train.bars.is_empty() || test.bars.is_empty() {
                report.push_str(&format!("## {sym} {tag}\ntrain/test 段空（数据不覆盖）\n\n"));
                continue;
            }
            let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
            let nav_te = nav_of(&test, &plain_cfg);
            // fullpi 口径 cfg（gross on + margin per-window nav₀）：训练/生产各按所在窗 nav₀。
            let mut fullpi_tr = mk_chi(true);
            fullpi_tr.margin = Some(q4_margin_model(nav_of(&train, &plain_cfg)));
            let mut fullpi_te = mk_chi(true);
            fullpi_te.margin = Some(q4_margin_model(nav_te));
            eprintln!("[q4] {sym} {tag} train={tr_lo}..{tr_hi}({}) test={te_lo}..{te_hi}({}) est×2…", train.bars.len(), test.bars.len());
            let (est_fullpi, _) = build_mu_from_bars(&train.bars, &fullpi_tr, 0);
            let (est_plain, _) = build_mu_from_bars(&train.bars, &plain_cfg, 0);

            let arm0 = run_theta_v0_pi(&test, &plain_cfg, years, nav_te);
            let arm1 = run_theta_v0_pi_chi(&test, &fullpi_te, years, nav_te, &est_fullpi, false);
            let arm2 = run_theta_v0_pi_chi(&test, &fullpi_te, years, nav_te, &est_fullpi, true);
            let arm3 = run_theta_v0_pi_chi(&test, &mk_chi(false), years, nav_te, &est_plain, false);

            report.push_str(&format!(
                "## {sym} {tag}（train {tr_lo}..{tr_hi}, test {te_lo}..{te_hi}, μ类数 fullpi={}/plain={}）\n\n\
                 | 臂 | Σpnl | max_dd | n_orders | n_trades | strat_return |\n|---|---|---|---|---|---|\n",
                est_fullpi.n_classes(), est_plain.n_classes()
            ));
            for (name, r) in [("Arm0 无χ", &arm0), ("Arm1 π^full", &arm1), ("Arm2 teap=true", &arm2), ("Arm3 隔离", &arm3)] {
                report.push_str(&fmt_arm(name, r));
            }
            let p = |r: &RunResult| r.trade_pnls_with_forced.iter().sum::<f64>();
            report.push_str(&format!(
                "- Arm1−Arm3（G7+margin 增量）：ΔΣpnl={:+.2} Δorders={}\n- Arm1−Arm2（χ 空类语义）：ΔΣpnl={:+.2} Δorders={}\n\n",
                p(&arm1) - p(&arm3), arm1.n_orders as i64 - arm3.n_orders as i64,
                p(&arm1) - p(&arm2), arm1.n_orders as i64 - arm2.n_orders as i64,
            ));
            eprintln!(
                "Q4 {sym} {tag}: A0={:+.0}/{} A1={:+.0}/{} A2={:+.0}/{} A3={:+.0}/{} (Σpnl/orders)",
                p(&arm0), arm0.n_orders, p(&arm1), arm1.n_orders, p(&arm2), arm2.n_orders, p(&arm3), arm3.n_orders
            );
            if *tag != "p3fold" {
                for (k, r) in [("Arm0", &arm0), ("Arm1", &arm1), ("Arm2", &arm2), ("Arm3", &arm3)] {
                    let e = agg.entry(k).or_insert((0.0, 0));
                    e.0 += p(r);
                    e.1 += r.n_orders as u64;
                }
            }
        }
        report.push_str(&format!("## {sym} walk-forward 聚合（wf 窗 Σ，不含 p3fold）\n\n| 臂 | ΣΣpnl | Σorders |\n|---|---|---|\n"));
        for (k, (pnl, ord)) in &agg {
            report.push_str(&format!("| {k} | {pnl:+.2} | {ord} |\n"));
        }
        report.push('\n');
    }
    std::fs::write("/tmp/q4_fullpi_policy.md", &report).ok();
    eprintln!("[q4] 报告落盘 /tmp/q4_fullpi_policy.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L1 自检（预注册 §3 承重不变量）：单品种 time_block 值域必须 < SYMBOL_STRIDE，否则跨品种 stratum
    /// 碰撞 ⟹ δ 在跨品种间 shuffle ⟹ beta 污染。最大窗数（ES/CL/GC=15）·WF_TIME_STRIDE + 窗内块上界
    /// 必须落在一个 SYMBOL_STRIDE 内；7 品种偏移不溢出 u32。破了这条 = 跨品种隔离失效。
    #[test]
    fn symbol_stride_isolates_strata() {
        let max_windows = PREREG_WINDOWS.iter().map(|w| w.wf_anchored.len()).max().unwrap() as u32;
        // 窗内 time_block = entry_bar/43_200；6 月窗 1min ≤ ~260K bar ⟹ 窗内块上界 ~10 < WF_TIME_STRIDE。
        let per_symbol_span = max_windows * WF_TIME_STRIDE + WF_TIME_STRIDE;
        assert!(per_symbol_span < SYMBOL_STRIDE, "单品种 time_block 值域 {per_symbol_span} 须 < SYMBOL_STRIDE {SYMBOL_STRIDE}（跨品种 stratum 隔离）");
        let max_offset = (L3_UNIVERSE.len() as u32 - 1) * SYMBOL_STRIDE + per_symbol_span;
        assert!(max_offset < u32::MAX, "7 品种最大 time_block 偏移不溢出 u32");
        assert!(!L3_UNIVERSE.contains(&"OKLO"), "OKLO 剔除（Observation 池，无 walk-forward OOS）");
    }

    /// L1 自检（q4 harness 日期算术，零信息增量）：前推 6 月跨年/钳日 + 前一天跨月。
    #[test]
    fn q4_date_helpers_hand_calc() {
        assert_eq!(q4_shift_back_6m("2023-02-17"), "2022-08-17"); // 跨年
        assert_eq!(q4_shift_back_6m("2023-08-17"), "2023-02-17");
        assert_eq!(q4_shift_back_6m("2023-08-31"), "2023-02-28"); // 钳日
        assert_eq!(q4_prev_day("2023-02-17"), "2023-02-16");
        assert_eq!(q4_prev_day("2023-03-01"), "2023-02-28"); // 跨月钳位
        assert_eq!(q4_prev_day("2023-01-01"), "2022-12-28"); // 跨年钳位
    }

    /// L1 自检（prereg-l3norm §5，管线正确性零信息增量）：σ̂ 计算核心 = 逐 bar 差分样本 std。
    /// 手算对照：pxs=[1,2,4,7] ⟹ diffs=[1,2,3] mean=2 var=((1)+0+(1))/2=1 std=1。防 σ̂ 估计 bug。
    #[test]
    fn sigma_stdev_matches_hand_calc() {
        assert!((stdev_consecutive_diffs(&[1.0, 2.0, 4.0, 7.0]) - 1.0).abs() < 1e-12);
        assert_eq!(stdev_consecutive_diffs(&[5.0]), 0.0, "单点无差分 ⟹ 0（守卫）");
        assert_eq!(stdev_consecutive_diffs(&[]), 0.0, "空序列 ⟹ 0（守卫）");
    }
}
