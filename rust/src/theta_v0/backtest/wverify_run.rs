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

use super::super::classifier::divergence::ForceStateA5;
use super::super::config::{ExecConfig, ThetaConfig, ThetaDirPreset};
use super::super::strategy::interp::ExitType;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::mu_estimator::{MuClass, ResidualTrade, UClass};
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::{BTreeMap, HashMap};

mod report;
use report::*;
mod m8;
mod issue71_chi_gamma;
pub(crate) use m8::{m6_cost_model, q4_margin_model};
use m8::{run_m8_e2e_all_systems_oos, run_q4_fullpi_policy};

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

/// g3 三套 OOS 入口（prereg-rev5 §5.1）：`THETA_DIR_PRESET` env 切换 Follow/Adversary preset。
/// 无 env/未知值 = Neutral（bit-exact 自检基线不变）。η 冻结（135号）：
/// eta_adv=[0.70,0.70,0.70,0.50,0.50,0.50]（prereg-rev5）/ eta_same=[0.70,0.50,0.70,0.70,0.70,0.70]（codex-ruling-eta）。
fn apply_theta_dir_preset_from_env(cfg: &mut ThetaConfig) {
    // g3 三套 OOS 入口（prereg-rev5 §5.1）：THETA_DIR_PRESET env 切 Follow/Adversary。无 env=Neutral。
    // η 冻结（135号）：eta_adv=[0.70,0.70,0.70,0.50,0.50,0.50]/eta_same=[0.70,0.50,0.70,0.70,0.70,0.70]。
    match std::env::var("THETA_DIR_PRESET").as_deref() {
        Ok("follow") => cfg.voice.theta_dir = ThetaDirPreset::Follow {
            eta_adv: vec![0.70, 0.70, 0.70, 0.50, 0.50, 0.50],
        },
        Ok("adversary") => cfg.voice.theta_dir = ThetaDirPreset::Adversary {
            eta_same: vec![0.70, 0.50, 0.70, 0.70, 0.70, 0.70],
        },
        _ => {} // Neutral default（w_dir≡1.0，bit-exact == v0）
    }
}

/// A-4（formal-criteria H3 / #122 终裁）：`ENFORCE_GROSS_CAP=true` 激活毛头寸约束（strict §11
/// `Σ|s_e| ≤ γ·U_ℓ`，coverage.rs `apply_gross_cap`）。无 env/非 "true" = default false 不变（#122
/// 「约束未配置=不激活」诚实声明 + frozen Θ v0 bit-exact）。跑批入口同 [`apply_theta_dir_preset_from_env`]
/// 先例——`build_mu_from_bars` 残差路径经 `typed_ledger_from_bars`→`pi_theta_fill_loop` 触达 `apply_gross_cap`，
/// 故残差跑批（wverify_full）与 π^full 跑批（m8_e2e）均需本 gate 才能测毛 cap 效应。
fn apply_enforce_gross_cap_from_env(cfg: &mut ThetaConfig) {
    if std::env::var("ENFORCE_GROSS_CAP").as_deref() == Ok("true") {
        cfg.risk.enforce_gross_cap = true;
    }
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
    let mut cfg = ThetaConfig::default();
    apply_theta_dir_preset_from_env(&mut cfg);
    apply_enforce_gross_cap_from_env(&mut cfg);
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let oos_sanity = ds.slice_date_window("2023-01-01", "2025-06-30"); // 数据漂移哨兵；协议 §2.1 BTC OOS
    assert!(!oos_sanity.bars.is_empty(), "OOS 窗空——数据漂移");
    // walk-forward OOS 残差聚合（G-A4）：逐窗 test 段独立估计，残差来自真样本外区间。
    let (records, time_blocks) = walk_forward_oos_residuals("BTC", 0, &ds, &cfg);
    assert!(!records.is_empty(), "walk-forward OOS 聚合未产出残差——窗口/数据不匹配");

    // ── Z_decision 逐笔序列（问题F 一等输出，无条件落盘）──
    // δ-free 主裁决键 Z_decision=(level,bsp_class,parent_dir) 逐笔外化，resid_base/cost 落 f64 bit 模式
    // （to_bits 十六进制）保离线复算逐字节还原。**输出产物不进判定路径**——下方在线 δ-free 主裁决
    // 直接消费内存 `records`（deltafree_verdict），不读本 dump。records 迭代序 = walk-forward 时间序。
    dump_deltafree_pertrade(&records);

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

    // 含 δ 4 元组桶是**描述性报告桶**（prereg §3.1 地位不变，保留不删）——非主裁决。
    eprintln!(
        "WV_FULL 报告桶(含δ,描述性) residuals={} buckets={} verdict={:?} V={} F={} I={} levels={:?} wf_windows={}",
        records.len(), agg.len(), v, nv, nf, ni, lv_set, time_blocks.len()
    );
    std::fs::write("/tmp/wv_full_rows.md", &rows).ok();
    std::fs::write("/tmp/wv_full_h2_asymmetry.md", &h2_rows).ok();

    // ── A1 ForceState⊥δ 前置检验（prereg-rev2 §1.2，i_class×δ 共线自毁铁律）──
    // ForceState 进主裁决聚合基前须验：每个出现的 force_state 态内 δ 两向都非空（有交换自由度，非共线）。
    // FAIL（任一态 δ 完全单向）⟹ A1 判 FALSIFIED，ForceState 退出主裁决基（fail 条件 3，停下上浮）。
    let ortho_report = forcestate_delta_orthogonality(&records);
    eprintln!("WV_FULL A1 ForceState⊥δ 检验:\n{ortho_report}");
    std::fs::write("/tmp/wv_full_forcestate_ortho.md", &ortho_report).ok();

    // ── δ-free 主裁决（prereg §3.2 主判据 + A1 force_state 第 8 维进聚合基）──
    // 直接在内存 records 上按 Z_decision=(level,bsp_class,parent_dir,force_state) 池化两 δ 方向算主裁决
    // ——**不读任何 dump**（deltafree_verdict 纯函数）。含 δ 4 元组降为上方描述性报告桶；本块是主路径。
    let (df_rows, df_v, (df_nv, df_nf, df_ni), df_lcb) = deltafree_verdict(&records);
    eprintln!(
        "WV_FULL δ-free 主裁决(A1 含 force_state) records={} δ-free基桶={} verdict={df_v:?} V={df_nv}/F={df_nf}/I={df_ni} | LCB>0: {df_lcb}",
        records.len(), df_nv + df_nf + df_ni,
    );
    std::fs::write(
        "/tmp/wv_full_deltafree.md",
        format!(
            "# δ-free 主裁决（在线直出，prereg-rev2 §1 Z_decision=(level,bsp_class,parent_dir,force_state)）\n\n\
             - records={} δ-free基桶={} verdict={df_v:?} V={df_nv}/F={df_nf}/I={df_ni}\n\
             - LCB>0 桶：{df_lcb}\n\n## ForceState⊥δ 检验\n\n{ortho_report}\n\n## δ-free 桶表\n\n{df_rows}\n",
            records.len(), df_nv + df_nf + df_ni,
        ),
    )
    .ok();

    // ── A6 μ_R=E[Y/d] co-primary 双门（prereg-rev2 §2，codex-ruling-696 选项 B）──
    // μ_R 样本 = {i : d_i ≥ D_MIN}，逐笔 resid_base/=d; cost/=d ⟹ y()=δ·resid_base−cost 自动 = Y/d
    // （与 σ̂ 跨品种归一化 bit-exact 同模式）。喂**同一** deltafree_verdict（含 A1 force_state 键）——
    // perm/decontam/Welford 透明消费归一化 records。raw μ（上方 df_*）现口径 bit-exact 不受影响。
    let d_min = cfg.tick.tick_size; // 最小合法止损距离 = 1 tick 美元值（prereg §2.3 冻结）
    let mut mu_r_records: Vec<ResidualTrade> = Vec::new();
    let (mut n_dmin_drop, mut n_nan_drop) = (0usize, 0usize);
    for r in &records {
        if !r.d.is_finite() {
            n_nan_drop += 1; // d 不可得（structural_stop None / BspPoint 缺失）⟹ μ_R 剔除
            continue;
        }
        if r.d < d_min {
            n_dmin_drop += 1; // 近零止损距离（D_MIN 守护，防分母放大灾难）⟹ μ_R 剔除
            continue;
        }
        let mut rr = *r;
        rr.resid_base /= r.d;
        rr.cost /= r.d;
        mu_r_records.push(rr);
    }
    let (mur_rows, mur_v, (mur_nv, mur_nf, mur_ni), mur_lcb) = deltafree_verdict(&mu_r_records);
    eprintln!(
        "WV_FULL μ_R co-primary(E[Y/d]) raw_n={} μ_R_n={}（剔除 NAN={n_nan_drop} d<D_MIN={n_dmin_drop}）δ-free基桶={} verdict={mur_v:?} V={mur_nv}/F={mur_nf}/I={mur_ni} | LCB>0: {mur_lcb}",
        records.len(), mu_r_records.len(), mur_nv + mur_nf + mur_ni,
    );
    std::fs::write(
        "/tmp/wv_full_mu_r.md",
        format!(
            "# μ_R=E[Y/d] co-primary 双门（prereg-rev2 §2，codex-ruling-696，桶键不加 d）\n\n\
             - raw_n={} μ_R_n={}（剔除：d 不可得 NAN={n_nan_drop}，d<D_MIN({d_min:.2e}) {n_dmin_drop}）\n\
             - δ-free基桶={} verdict={mur_v:?} V={mur_nv}/F={mur_nf}/I={mur_ni}\n\
             - LCB>0 桶：{mur_lcb}\n\n\
             口径：μ_R 与 raw μ **不可比**（#135：μ_R 除 d + D_MIN 过滤样本集不同）；双 estimand 各自过门，\
             Holm 全族校正（见结果包）。\n\n{mur_rows}\n",
            records.len(), mu_r_records.len(), mur_nv + mur_nf + mur_ni,
        ),
    )
    .ok();

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

    // ── ExitType 诊断切片（裁定甲：5 变体占比拆解，纯描述性，不进裁决基）──
    let exit_diag = exit_type_breakdown(&records);
    eprintln!("WV_FULL ExitType 诊断切片:\n{exit_diag}");
    std::fs::write("/tmp/wv_full_exittype.md", &exit_diag).ok();

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

/// 四口径 D 判定 OOS 批（A2 #163 三口径 + A3 #164 ThetaLex，prereg-a2-thetadom-oos-20260704
/// **冻结先于跑数** + a2 边界(f) Θ_LEX 补齐）。
///
/// G1 MacdArea（对照基线，生产默认）/ G2 ThetaDom（Θ_DOM，ForceStateA5==Dominated，A5 amended——
/// codex-a2-thetadom-20260704 裁定）/ G3 Conjunction（G1∧G2）/ G4 ThetaLex（Θ_LEX，`weak_theta(Lex)`
/// 词典序 DIF▷面积，关于背驰.pdf §9.2 三套 Θ 的第三套，A3 #164 补齐 a2 边界(f)）。gauge 经
/// `ThetaConfig.divergence_gauge` 切换 ⟹ 一类 buy1/sell1 集合 ⟹ 生产 π ledger ⟹ 残差样本——全链真
/// 路径（675号：不另起坐标系）。桶键/统计与 wverify_full 同口径（`bucket_verdict`）；四口径独立报告，
/// 不事后挑桶；负结论功效门槛沿用 prereg（LCB<0 前核 n_eff）。关于背驰.pdf §9.2「分别 OOS 回测，
/// 不能先看结果再选」——四口径同批跑、独立列出，不事后选口径。
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::thetadom_three_gauge_oos -- --ignored --nocapture`。
#[test]
#[ignore]
fn thetadom_three_gauge_oos() {
    use super::super::classifier::divergence::DivergenceGauge;
    let gauges = [
        ("G1-MacdArea", DivergenceGauge::MacdArea),
        ("G2-ThetaDom", DivergenceGauge::ThetaDom),
        ("G3-Conjunction", DivergenceGauge::Conjunction),
        ("G4-ThetaLex", DivergenceGauge::ThetaLex),
    ];
    let base = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &base).expect("BTC 数据加载（btc_1m_full.json）");
    let mut report = String::from(
        "# 四口径 D 判定 OOS（prereg-a2-thetadom-oos-20260704，力度族 𝒜₅ A5 amended；A3 补 Θ_LEX）\n\n\
         口径：BTC wf_anchored 12 窗 walk-forward OOS 残差；桶键 (ℓ,bsp,δ,σ^H)；统计与 wverify_full 同。\n\n",
    );
    for (name, g) in gauges {
        let mut cfg = base.clone();
        cfg.divergence_gauge = g;
        let (records, _tb) = walk_forward_oos_residuals("BTC", 0, &ds, &cfg);
        // 一类交易计数（i_class bit0=buy1 / bit3=sell1，与 wverify_fullz 探针同口径）——G2/G3 相对
        // G1 的信号收缩量是 prereg 预期方向（Dominated ⊆ 宽判）的 L1 观测点。
        let n_t1 = records.iter().filter(|r| r.class.i_class & 0b001_001 != 0).count();
        let (_stats, rows, verdict, (v, f, i)) = bucket_verdict(&records);
        report.push_str(&format!(
            "## {name}\n- residuals={} type1_trades={} verdict={} V={}/F={}/I={}\n\n{}\n",
            records.len(), n_t1, verdict, v, f, i, rows
        ));
        eprintln!(
            "THETADOM_OOS {name}: residuals={} type1={} verdict={} V={}/F={}/I={}",
            records.len(), n_t1, verdict, v, f, i
        );
    }
    std::fs::write("/tmp/thetadom_three_gauge_oos.md", &report).ok();
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

#[test]
#[ignore]
fn q4_fullpi_policy() {
    run_q4_fullpi_policy();
}

#[test]
#[ignore]
fn issue71_chi_gamma_validation() {
    issue71_chi_gamma::run_issue71_chi_gamma_validation();
}

/// ★M6 BTC OOS R 分解跑批（TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关）：
/// 在真实 BTC OOS 窗跑带 margin（CME-simple）+ cost_model（参数化持有成本三项，**spot 口径**见
/// [`m6_cost_model`]）的 π^full 臂，落盘 R 分解表（ΣN_tΔP_t / Commission+Slippage / Funding /
/// Borrow / LiquidationLoss / net_r / 守恒残差）。
///
/// **认识论（照实）**：预期成本拖累（net_r < gross）——这是**成本真实化**（M6 关卡把三项成本纳入
/// PnL），**不是** alpha 声明。守恒残差 ≈0 是「资金无泄漏」物证。有效域 L1（机制正确性 + 参数化
/// 常费率），非 L2 盈利判定。
///
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::m6_btc_oos_r_decomposition -- --ignored --nocapture`。
#[test]
#[ignore]
fn m6_btc_oos_r_decomposition() {
    use super::runner::run_theta_v0_pi;
    let plain_cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain_cfg).expect("BTC 数据加载（btc_1m_full.json）");

    let nav_of = |d: &data::Dataset| {
        d.bars.iter().find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * plain_cfg.tick.tick_size).unwrap_or(1.0) * 1000.0
    };

    let mut report = String::from(
        "# M6 BTC OOS R 分解（TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关）\n\n\
         R = Σ N_t ΔP_t − Commission − Slippage − Funding − Borrow − LiquidationLoss\n\n\
         venue 口径（#303 裁定）：数据 = Binance **现货** BTCUSDT 1m ⟹ **无资金费**；`Funding` 列记的是\
         **资金占用机会成本**（1bp/8h 记账周期），`Borrow` = 现货杠杆借币利息（0.01bp/bar，仅借入名义），\
         `Liq` = 现货杠杆强平罚金（0.5%）。margin=CME-simple 单段近似。\n\n\
         **有效域 L1**：机制真装 + 参数化常费率，真实现货借贷/机会成本利率曲线是外部数据缺口（A10 \
         waiver 豁免外部数据源，不豁免机制）。**照实：预期成本拖累 net_r<gross，成本真实化非 alpha 声明。**\n\n",
    );
    // A10 附则B 裁决2（090 措辞纪律）：一切带成本 R 数值报告强制口径标签——费率未标定，
    // 常费率数值禁作 alpha 论据/策略择优输入；datum 注入后升 [L2费率标定: datum 版本哈希]。
    // ★#360：标签由 `ExecConfig::fee_schedule` 决定——None ⟹ L1 未标定（现状），
    //   Some(datum) ⟹ `[L2费率标定: datum <sha256 前12位>]`（成交费率科目已 venue 标定；
    //   持有成本三项 funding/borrow/liq 仍是常费率保底，见下行括注，不得跳级）。
    report.push_str(&format!(
        "**口径标签：{}**（成交费率科目；持有成本三项仍 {}）（A10 附则B 强制；TW桥列＝A10 C5 对账行 ⌊funding+borrow+liq⌋——TW 账本不经构造子见持盾成本，η=tw() 高估在险权益恰此量）\n\n\
         | 窗 | 臂 | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r | 守恒残差 | TW桥 | n_orders |\n\
         |---|---|---|---|---|---|---|---|---|---|---|\n",
        super::super::strategy::risk::rate_calibration_label(&plain_cfg.exec),
        super::super::strategy::risk::RATE_UNCALIBRATED_LABEL,
    ));

    // OOS 窗清单：p3 可比单折 + 前两个 anchored walk-forward（够 R 分解物证；全窗跑批在 M8）。
    let mut wins: Vec<(String, String, String)> = vec![
        ("p3fold".into(), "2023-01-01".into(), "2023-06-30".into()),
    ];
    if let Some(sw) = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC") {
        for w in sw.wf_anchored.iter().filter(|w| w.test_start >= OOS_START).take(2) {
            wins.push((format!("wf{}", w.i), w.test_start.into(), w.test_end.into()));
        }
    }

    for (tag, te_lo, te_hi) in &wins {
        let test = ds.slice_date_window(te_lo, te_hi);
        if test.bars.is_empty() {
            report.push_str(&format!("| {tag} | — | test 段空 | | | | | | | | |\n"));
            continue;
        }
        let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
        let nav_te = nav_of(&test);
        // M6 臂：margin（CME-simple）+ cost_model（参数化三项）。χ 不启（隔离 M6 成本效应，非信号层）。
        let mut m6_cfg = ThetaConfig::default();
        m6_cfg.margin = Some(q4_margin_model(nav_te));
        m6_cfg.cost_model = Some(m6_cost_model());
        eprintln!("[m6] BTC {tag} test={te_lo}..{te_hi}({}) run…", test.bars.len());
        let r = run_theta_v0_pi(&test, &m6_cfg, years, nav_te);
        match r.r_decomp {
            Some(d) => {
                report.push_str(&format!(
                    "| {tag} | M6 | {:+.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:+.2} | {:.2e} | {} | {} |\n",
                    d.price_pnl_gross, d.commission_slippage, d.funding, d.borrow,
                    d.liquidation_loss, d.net_r, d.conservation_residual,
                    d.tw_holding_cost_bridge, r.n_orders,
                ));
                eprintln!(
                    "M6 BTC {tag}: gross={:+.0} fee={:.0} fund={:.0} borrow={:.0} liq={:.0} net_r={:+.0} resid={:.2e} tw_bridge={} orders={}",
                    d.price_pnl_gross, d.commission_slippage, d.funding, d.borrow,
                    d.liquidation_loss, d.net_r, d.conservation_residual,
                    d.tw_holding_cost_bridge, r.n_orders,
                );
                // 守恒硬校验（照实——真实数据 O(n) 舍入，容差按名义规模）。
                let tol = 1e-3_f64.max(1e-9 * (nav_te.abs() + d.price_pnl_gross.abs()));
                assert!(
                    d.conservation_residual.abs() <= tol,
                    "M6 {tag} 守恒残差 {} 超容差 {}（资金泄漏）", d.conservation_residual, tol
                );
            }
            None => report.push_str(&format!("| {tag} | M6 | R 分解缺失（非 π 路径？）| | | | | | | | |\n")),
        }
    }
    report.push_str("\n**守恒断言**：各窗 |守恒残差| ≤ 容差（价格 PnL − 五项成本 = 账本净变动，无泄漏）。\n");
    std::fs::write("/tmp/m6_btc_oos_r_decomposition.md", &report).ok();
    eprintln!("[m6] R 分解报告落盘 /tmp/m6_btc_oos_r_decomposition.md");
}

#[test]
#[ignore]
fn m8_e2e_all_systems_oos() {
    run_m8_e2e_all_systems_oos();
}
/// δ-free 主裁决的**离线 dump 复现器**（问题F 收口后：主裁决已在线，本测退为快速复现工具）。
///
/// 问题F 收口前 δ-free 主裁决只在此离线算（含 δ 4 元组做主 verdict 是缺口）；收口后 `wverify_full`
/// 主路径直接在内存 records 上调 [`deltafree_verdict`] 直出主裁决。本测保留价值=**不跑 461万 bar 全量**
/// 从冻结 dump 毫秒级复现同一裁决（CI/复算）。读 `DELTAFREE_DUMP`（Z_decision 逐笔序列）→ [`load_deltafree_dump`]
/// 逐字节还原 records（round-trip 精确）→ 与在线共用 [`deltafree_verdict`] ⟹ 输出 = 在线主裁决表。
/// 报告落 /tmp/deltafree_exact.md；`DELTAFREE_DUMP=/path cargo test --release --lib
/// theta_v0::backtest::wverify_run::deltafree_exact_recompute -- --ignored --nocapture`。
#[test]
#[ignore]
fn deltafree_exact_recompute() {
    let dump = std::env::var("DELTAFREE_DUMP").unwrap_or_else(|_| "/tmp/finalpha/deltafree_pertrade.tsv".into());
    let records = load_deltafree_dump(&dump);
    assert!(!records.is_empty(), "δ-free dump 空——先跑 DELTAFREE_DUMP=<path> wverify_full 落盘");

    // 与在线 wverify_full 主路径共用 deltafree_verdict（单一口径）——dump 还原 records 与在线 records
    // 逐字节相等（round-trip 精确，见 deltafree_dump_roundtrip_and_pooling），故本复现 = 在线主裁决。
    let (rows, v, (nv, nf, ni), lcb) = deltafree_verdict(&records);
    let n_buckets = nv + nf + ni;
    let summary = format!(
        "# δ-free 聚合基精确三态重算（Task #186，final-alpha §3.1 精确口径；离线 dump 复现器）\n\n\
         - dump={dump} records={} δ-free基桶={n_buckets} verdict={v:?} V={nv}/F={nf}/I={ni}\n\
         - LCB>0 桶（精确口径）：{lcb}\n\n{rows}\n",
        records.len(),
    );
    std::fs::write("/tmp/deltafree_exact.md", &summary).ok();
    eprintln!("DELTAFREE_EXACT records={} buckets={n_buckets} verdict={v:?} V={nv}/F={nf}/I={ni} | LCB>0: {lcb}", records.len());
}

/// M3 分区全历史长跑（TARGET_STRATEGY_MAXFULL.md §M3 `𝒳=⊔C_z`，L2 真实数据零违例）：BTC 全
/// walk-forward OOS 窗逐窗同源取 ledger+records，[`assert_m3_partition`] 零违例——互斥穷尽守恒
/// （|ledger|=kept+排除类、Σ|C_z|=|records|）+ 键值域封闭在 461 万 bar 定义域上成立。若任一窗
/// 违例 = 分类函数缺陷（停下上浮，no-workaround 不放行）。
/// `cargo test --release --lib theta_v0::backtest::wverify_run::m3_partition_btc_fullhistory -- --ignored --nocapture`
#[test]
#[ignore]
fn m3_partition_btc_fullhistory() {
    use super::l3_delta_r_alpha::assert_m3_partition;
    use super::runner::typed_ledger_from_bars;
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");

    let (mut tot_ledger, mut tot_kept, mut tot_records, mut n_win) = (0usize, 0usize, 0usize, 0usize);
    for win in sw.wf_anchored {
        if win.test_start < OOS_START {
            continue; // IS 期窗不算 OOS 证据（与 walk_forward_oos_residuals 同过滤）
        }
        let test_ds = ds.slice_date_window(win.test_start, win.test_end);
        if test_ds.bars.is_empty() {
            continue;
        }
        // 同源：ledger 与 records 由同一 bars 切片产出（build_mu_from_bars 内部即调 typed_ledger_from_bars）。
        let ledger = typed_ledger_from_bars(&test_ds.bars, &cfg);
        let (_est, records) =
            build_mu_from_bars(&test_ds.bars, &cfg, win.i * WF_TIME_STRIDE);
        // 逐窗零违例（内部 panic = 该窗分区破缺，停下上浮）。
        assert_m3_partition(&ledger, &records);
        let kept = ledger.iter().filter(|t| matches!(
            super::l3_delta_r_alpha::ledger_disposition(t),
            super::l3_delta_r_alpha::LedgerDisposition::Kept
        )).count();
        eprintln!(
            "[m3-full] win{} {}→{} |ledger|={} kept={kept} |records|={}",
            win.i, win.test_start, win.test_end, ledger.len(), records.len()
        );
        tot_ledger += ledger.len();
        tot_kept += kept;
        tot_records += records.len();
        n_win += 1;
    }
    assert!(n_win > 0, "BTC OOS 窗非空（否则测试空转）");
    assert_eq!(tot_kept, tot_records, "全窗聚合 kept ≡ |records|（穷尽守恒跨窗一致）");
    eprintln!(
        "[m3-full] BTC 全历史 {n_win} 窗 M3 分区零违例：Σ|ledger|={tot_ledger} Σkept={tot_kept} Σ|records|={tot_records}"
    );
}

#[cfg(test)]
#[path = "wverify_run/tests.rs"]
mod tests;
