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
use super::super::config::{ThetaConfig, ThetaDirPreset};
use super::super::strategy::interp::ExitType;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::mu_estimator::{MuClass, ResidualTrade, UClass};
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::{BTreeMap, HashMap};

/// ★A1（prereg-rev2-20260704）：force_state 第 8 维的 dump 编码（离线 round-trip 无损）。
/// None=0 / Dominated=1 / Dominates=2 / Tie=3 / Incomparable=4——`load_deltafree_dump` 逆映射。
fn force_state_code(fs: Option<ForceStateA5>) -> u8 {
    match fs {
        None => 0,
        Some(ForceStateA5::Dominated) => 1,
        Some(ForceStateA5::Dominates) => 2,
        Some(ForceStateA5::Tie) => 3,
        Some(ForceStateA5::Incomparable) => 4,
    }
}

/// force_state 报告标签（None/Dom-/Dom+/Tie/Inc，δ-free 主裁决桶表可读列）。
fn force_state_label(fs: Option<ForceStateA5>) -> &'static str {
    match fs {
        None => "None",
        Some(ForceStateA5::Dominated) => "Dom-",
        Some(ForceStateA5::Dominates) => "Dom+",
        Some(ForceStateA5::Tie) => "Tie",
        Some(ForceStateA5::Incomparable) => "Inc",
    }
}

/// ★A1（prereg-rev2 §1.2）ForceState⊥δ 检验：对每个出现的 force_state 态统计 δ∈{+1,−1} 计数。
///
/// 判据（i_class×δ 共线自毁铁律 memory `iclass_delta_collinearity_perm_degeneracy`）：
/// - **PASS**：每态 δ 两向都非空（min(n+,n−)≥1）⟹ 置换在该态内有交换自由度，非共线 ⟹ ForceState 合法进
///   主裁决聚合基。
/// - **FAIL**：任一态 δ 完全单向（n+=0 或 n−=0）⟹ 进基即毁其内 δ 置换 ⟹ A1 判 FALSIFIED，退出主裁决基
///   （仅报告桶维），停下上浮（fail 条件 3）。
/// 返回 markdown 报告串（含 PASS/FAIL 结论 + 逐态计数）——**不 panic**：FAIL 是诚实产出（照实入结果包，
/// 由 Lead/上浮决策 A1 处置），非管线错误（161 否定性照实）。
fn forcestate_delta_orthogonality(records: &[ResidualTrade]) -> String {
    // 逐 force_state 态计 (n_δ+1, n_δ−1)。只检 Some(态)——None 是「无力度源」大桶，δ 两向天然混合
    // （不细分即不改置换），非 A1 关心的方向性维共线风险。
    let mut counts: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
    for r in records {
        if let Some(fs) = r.class.force_state {
            let e = counts.entry(force_state_code(Some(fs))).or_insert((0, 0));
            match r.class.delta {
                1 => e.0 += 1,
                -1 => e.1 += 1,
                _ => {}
            }
        }
    }
    let mut fail_states: Vec<String> = Vec::new();
    let mut rows = String::from("| force_state | n(δ+1) | n(δ−1) | 交换自由度 |\n|---|---|---|---|\n");
    for (&code, &(np, nm)) in &counts {
        let lbl = force_state_label(force_state_decode(code));
        let ok = np >= 1 && nm >= 1;
        rows.push_str(&format!("| {lbl} | {np} | {nm} | {} |\n", if ok { "有(非共线)" } else { "无(单向共线)" }));
        if !ok {
            fail_states.push(format!("{lbl}(n+={np},n−={nm})"));
        }
    }
    let verdict = if counts.is_empty() {
        "PASS(空——无 Some(force_state) 记录，force_state 全 None ⟹ A1 细分退化为单 None 桶，不改置换)"
            .to_string()
    } else if fail_states.is_empty() {
        "PASS(每态 δ 两向非空，ForceState⊥δ 有交换自由度 ⟹ 合法进主裁决聚合基)".to_string()
    } else {
        format!("FAIL(单向共线态: {}) ⟹ A1 判 FALSIFIED，ForceState 退出主裁决基，停下上浮(fail 条件 3)", fail_states.join(", "))
    };
    format!("**ForceState⊥δ 检验结论**：{verdict}\n\n{rows}")
}

/// force_state 编码逆映射（[`force_state_code`]），离线 dump 复现器还原第 8 维。
fn force_state_decode(code: u8) -> Option<ForceStateA5> {
    match code {
        1 => Some(ForceStateA5::Dominated),
        2 => Some(ForceStateA5::Dominates),
        3 => Some(ForceStateA5::Tie),
        4 => Some(ForceStateA5::Incomparable),
        _ => None,
    }
}

/// ExitType 诊断切片的 dump 编码（codex-ruling-exittype-20260704 裁定甲：逐笔存档合法）。
/// **不进桶键/门控/裁决基**——纯诊断列（[`exit_type_decode`] 逆映射，[`exit_type_label`] 报告标签）。
fn exit_type_code(et: ExitType) -> u8 {
    match et {
        ExitType::CloseRoot => 0,
        ExitType::ReduceCore => 1,
        ExitType::CloseReverseOpen => 2,
        ExitType::RiskExit => 3,
        ExitType::Hold => 4,
    }
}

/// ExitType 编码逆映射（[`exit_type_code`]），离线 dump 复现器还原诊断列。
fn exit_type_decode(code: u8) -> ExitType {
    match code {
        0 => ExitType::CloseRoot,
        1 => ExitType::ReduceCore,
        2 => ExitType::CloseReverseOpen,
        3 => ExitType::RiskExit,
        _ => ExitType::Hold,
    }
}

/// ExitType 报告标签（W-VERIFY 5 变体占比拆解节可读列）。
fn exit_type_label(et: ExitType) -> &'static str {
    match et {
        ExitType::CloseRoot => "CloseRoot(P5)",
        ExitType::ReduceCore => "ReduceCore(P6)",
        ExitType::CloseReverseOpen => "CloseReverseOpen(P7)",
        ExitType::RiskExit => "RiskExit(P1)",
        ExitType::Hold => "Hold(P0)",
    }
}

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

/// issue #357 验收③：`THETA_CENTER_OSCILLATION=1` 激活中枢震荡短差门（`config.center_oscillation
/// .enabled`）——跑批入口同 [`apply_theta_dir_preset_from_env`] 先例。无 env/非 "1" = default
/// false 不变（既有 `m8_e2e_all_systems_oos` 默认关轨迹逐字节不变，bit-exact 回归锁）。
fn apply_center_oscillation_from_env(cfg: &mut ThetaConfig) {
    if std::env::var("THETA_CENTER_OSCILLATION").as_deref() == Ok("1") {
        cfg.center_oscillation.enabled = true;
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

/// Z_decision 逐笔序列落盘（问题F 一等输出：`wverify_full` 主路径无条件外化）。
///
/// 每笔外化 δ-free 主裁决键 Z_decision=(level,bsp_class,parent_dir,force_state) + 残差成分（TSV：
/// level bsp δ σ^H force_state h_bucket time_block resid_base_bits cost_bits d_bits，A1/A6 增
/// force_state+d 两列）。默认落 `/tmp/wv_full_zdecision.tsv`
/// （与 /tmp/wv_full_rows.md 等主路径产物同级，无 env 门控）；`DELTAFREE_DUMP=<path>` 覆盖路径（离线
/// 复算/自检指定用）。resid_base/cost 用 `f64::to_bits` 十六进制（round-trip 精确）——离线
/// [`deltafree_exact_recompute`] 逐字节还原内存值，n_eff（Geyer IPS）/ δ-free perm_p 与在线口径 bit-exact
/// 可比。**本 dump 是输出产物，不进判定路径**——在线 δ-free 主裁决直接在内存 `records` 上算
/// （见 [`deltafree_verdict`]），不消费本文件。保 records 顺序（walk-forward 时间序）⟹ effective_n
/// 的成交时间序前提成立（decontam 口径）。
fn dump_deltafree_pertrade(records: &[ResidualTrade]) {
    let path = std::env::var("DELTAFREE_DUMP")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/tmp/wv_full_zdecision.tsv".into());
    // A1/A6（prereg-rev2-20260704）：dump 增 force_state（δ-free 主裁决基第 8 维）+ d_bits（μ_R 分母）
    // 两列——离线复现器 [`deltafree_exact_recompute`] 逐字节还原 ⟹ 与在线主裁决/μ_R bit-exact。
    // 路线.pdf p13 逐笔存档：末列 exit_type 诊断切片（裁定甲存档合法，不进裁决桶键）。
    let mut out = String::from("level\tbsp\tdelta\tsigma_h\tforce_state\th_bucket\ttime_block\tresid_base_bits\tcost_bits\td_bits\texit_type\n");
    for r in records {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{:016x}\t{}\n",
            r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir,
            force_state_code(r.class.force_state),
            r.h_bucket, r.time_block, r.resid_base.to_bits(), r.cost.to_bits(), r.d.to_bits(),
            exit_type_code(r.exit_type)
        ));
    }
    std::fs::write(&path, &out).unwrap_or_else(|e| panic!("Z_decision dump 落盘失败 {path}：{e}"));
    eprintln!("[zdecision-dump] {} 笔 → {path}", records.len());
}

// ── δ-free 主裁决键形状冻结（编译期守卫，a3）──────────────────────────────────
// Z_decision 键 = (level, bsp_class, parent_dir, force_state) 四元组，**不含 δ 分量**
// （prereg-rev2 §1 + ★A1 force_state 第 8 维）。恒等函数指针赋值 ⟹ 若未来有人把 δ 加回
// [`perm_test::DeltaFreeKey`]（形状/分量类型改变），本行编译失败——比运行期测试更早拦截。
// 语义级守卫（仅 δ 不同的记录池化同桶）见 `tests::deltafree_verdict_key_is_delta_free_four_tuple`。
const _DELTAFREE_KEY_SHAPE_FROZEN: fn(
    perm_test::DeltaFreeKey,
) -> (u32, u8, i8, Option<ForceStateA5>) = |k| k;

/// δ-free 聚合基 (level, bsp_class, parent_dir) 精确三态裁决——**主判据**（prereg §3.2；问题F 收口）。
///
/// 输入任意残差序列（在线 `wverify_full` 内存 `records`，或离线 dump 还原），按 Z_decision 键池化两 δ
/// 方向，精确 Welford std（n−1 分母，非由 LCB 反推）+ 精确 n_eff（[`decontam::effective_n`] Geyer IPS）
/// + δ-free perm_p（[`perm_test::stratified_delta_perm_p_deltafree`]，B22 同批置换只读出侧池化）→
/// [`decontam::classify_bucket`] 三态。返回 (22 桶 markdown 表, 全局裁决, (V,F,I) 计数, LCB>0 桶摘要)。
///
/// **单一口径**：在线主路径与离线 [`deltafree_exact_recompute`] 共用本纯函数（两个 caller 一个逻辑，
/// 非「在线包一层调离线」垫片）⟹ 同数据同 records 序 ⟹ 逐字节同结果。records 迭代序须为 walk-forward
/// 时间序（effective_n 成交时间序前提）；BTreeMap 输出确定序。
fn deltafree_verdict(
    records: &[ResidualTrade],
) -> (String, decontam::AcceptanceVerdict, (usize, usize, usize), String) {
    // δ-free 主裁决基 (level,bsp_class,parent_dir,force_state)：池化两 δ 方向，保时间序 Y 序列
    // （records 已按 walk-forward 序）。★A1（prereg-rev2-20260704）：force_state 第 8 维进主裁决基
    // （dfonline-a2 §4 两注入点之一，与 perm_test.rs base_map 键同步——否则 records Some(态) vs 键缺维
    // 全表 miss，fullz G2 前例）。
    let mut series: BTreeMap<perm_test::DeltaFreeKey, Vec<f64>> = BTreeMap::new();
    for r in records {
        series
            .entry((r.class.level, r.class.bsp_class(), r.class.parent_dir, r.class.force_state))
            .or_default()
            .push(r.y());
    }
    let pp = perm_test::stratified_delta_perm_p_deltafree(records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);

    let mut rows = String::from(
        "| L | bsp | σ^H | force | N | n_eff | mean(Y) | std | lcb | ucb | cv | perm_p | state |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut states = Vec::new();
    let mut lcb_pos: Vec<String> = Vec::new();
    for ((lv, bc, pd, fs), ys) in &series {
        let n = ys.len();
        let mean = ys.iter().sum::<f64>() / n as f64;
        // 精确 Welford std（样本 std，n−1 分母）——非由 LCB 反推的正态近似 std。
        let std = if n < 2 {
            f64::NAN
        } else {
            (ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
        };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let n_eff = decontam::effective_n(ys); // 精确 Geyer IPS
        let perm_p = *pp.get(&(*lv, *bc, *pd, *fs)).unwrap_or(&1.0);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        let fs_lbl = force_state_label(*fs);
        if lcb > 0.0 {
            lcb_pos.push(format!("L{lv} bsp{bc} σ{pd:+} f={fs_lbl} (mean{mean:+.2} lcb{lcb:+.2} n_eff{n_eff:.1} perm_p{perm_p:.3} {st:?})"));
        }
        rows.push_str(&format!(
            "| L{lv} | {bc} | σ{pd:+} | {fs_lbl} | {n} | {n_eff:.2} | {mean:+.4} | {std:.2} | {lcb:+.4} | {ucb:+.4} | {cv:.3} | {perm_p:.3} | {st:?} |\n"
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    let lcb_summary = if lcb_pos.is_empty() { "无".into() } else { lcb_pos.join("; ") };
    (rows, v, (nv, nf, ni), lcb_summary)
}

/// ExitType 诊断切片：按 5 变体拆解 n/占比/mean(Y)（codex-ruling-exittype-20260704 裁定甲）。
///
/// **纯描述性**——无三态判定、无 LCB 门、不进 accept/reject。防止诊断切片伪装成裁决层：本函数
/// 只回报每变体的计数/占比/收益均值（异质性阅读），不做 Validated/Falsified 分类。exit_type 在入场
/// 决策点 t 不可观测（post-treatment），故只作事后诊断，绝不进 μ 桶键/χ_t 门控/δ-free 主裁决基。
fn exit_type_breakdown(records: &[ResidualTrade]) -> String {
    let n_total = records.len();
    // 5 变体固定序（CloseRoot/ReduceCore/CloseReverseOpen/RiskExit/Hold）——即使某变体 0 笔也列出（穷尽）。
    let variants = [
        ExitType::CloseRoot,
        ExitType::ReduceCore,
        ExitType::CloseReverseOpen,
        ExitType::RiskExit,
        ExitType::Hold,
    ];
    let mut rows = String::from(
        "| exit_type | n | 占比 | mean(Y) |\n|---|---|---|---|\n",
    );
    for et in variants {
        let ys: Vec<f64> = records.iter().filter(|r| r.exit_type == et).map(|r| r.y()).collect();
        let n = ys.len();
        let frac = if n_total == 0 { 0.0 } else { n as f64 / n_total as f64 };
        let mean = if n == 0 { f64::NAN } else { ys.iter().sum::<f64>() / n as f64 };
        rows.push_str(&format!(
            "| {} | {} | {:.4} | {:+.6} |\n",
            exit_type_label(et), n, frac, mean
        ));
    }
    format!(
        "# ExitType 诊断切片（裁定甲，纯描述性——无三态/无 LCB 门/不进裁决基）\n\n\
         - 总笔数 n={n_total}（5 变体穷尽拆解，占比和 = 1.0）\n\
         - **地位**：事后诊断切片。exit_type 不进 μ 桶键/χ_t 门控/δ-free 主裁决基（exit-μ-BUCKETING-FROZEN #180）。\n\n{rows}"
    )
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
        // f3-C 多重赋格反事实：剔 ReverseOpen 声部（同 no-χ 口径，margin=None 默认），测对冲增量价值。
        let mut sd_off_cfg = base_cfg.clone();
        sd_off_cfg.voice.disable_reverse_open = true;
        let sd_off_r = run_theta_v0_pi(&test_ds, &sd_off_cfg, years, nav);
        let sum = |v: &[f64]| v.iter().sum::<f64>();
        let (chi_pnl, base_pnl, sd_off_pnl) = (sum(&chi_r.trade_pnls_with_forced), sum(&base_r.trade_pnls_with_forced), sum(&sd_off_r.trade_pnls_with_forced));
        report.push_str(&format!(
            "## {sym}（μ̂ 桶数={}, test_bars={}）\n\
             - χ门 μ̂  : Σpnl={chi_pnl:+.2} max_dd={:.4} n_orders={} n_trades={} strat_return={:+.4}\n\
             - 无χ基线: Σpnl={base_pnl:+.2} max_dd={:.4} n_orders={} n_trades={} strat_return={:+.4}\n\
             - 差分   : ΔΣpnl={:+.2} Δmax_dd={:+.4} Δn_orders={}（μ̂ 门增量价值）\n\
             - **f3-C 反事实**（全赋格 vs 剔 ReverseOpen，同 no-χ 口径）:\n\
             &nbsp;&nbsp;全赋格(=无χ基线): Σpnl={base_pnl:+.2} max_dd={:.4} n_orders={}\n\
             &nbsp;&nbsp;剔 ReverseOpen  : Σpnl={sd_off_pnl:+.2} max_dd={:.4} n_orders={}\n\
             &nbsp;&nbsp;ReverseOpen 增量: ΔΣpnl={:+.2} Δmax_dd={:+.4} Δn_orders={}（多重赋格对冲声部的组合级增量价值）\n\n",
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
        eprintln!("POLICY {sym}: χ Σpnl={chi_pnl:+.2} dd={:.4} ord={} | base Σpnl={base_pnl:+.2} dd={:.4} ord={} | ΔΣpnl={:+.2} || f3C ReverseOpen 增量 ΔΣpnl={:+.2} (剔后 Σpnl={sd_off_pnl:+.2} ord={})",
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
/// ★pub(crate)：阶段 3a 前置实装（M7_WITNESS_A10 env gate，p126 runbook §2.1）——runner.rs 三个
/// #[ignore] witness/网格/多窗测试与 m8_e2e 同函数同源注入（禁第二查法），可见性由模块私有提 crate。
pub(crate) fn q4_margin_model(nav0: f64) -> super::super::strategy::risk::MarginModel {
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

/// M6 成本模型口径（参数化常费率，有效域 L1）：Binance 永续近似——funding 8h 周期 = 480 根
/// 1分钟 bar，费率 1bp/周期（业界常见量级）；borrow 每 bar 极低（无杠杆则不 binding）；强平罚金
/// 0.5%（清算费+滑点近似）。**有效域声明（231号 / A10 waiver）**：真实 funding 历史/借贷曲线是
/// **外部数据源缺口**（L2），waiver 豁免的是外部数据，机制在此真实装、费率待外部标定。
/// ★pub(crate)：阶段 3a 前置实装（M7_WITNESS_A10 env gate，p126 runbook §2.1）同 q4_margin_model。
pub(crate) fn m6_cost_model() -> super::super::strategy::risk::CostModel {
    use super::super::strategy::risk::CostModel;
    // funding 8h=480bar、1bp/周期；borrow 每 bar 0.01bp；liq 罚金 0.5%。
    CostModel::new(0.0001, 480, 0.000001, 0.005).expect("M6 常费率参数合法（冻结近似值）")
}

/// ★M6 BTC OOS R 分解跑批（TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关）：
/// 在真实 BTC OOS 窗跑带 margin（CME-simple）+ cost_model（参数化 funding/borrow/liq）的 π^full
/// 臂，落盘 R 分解表（ΣN_tΔP_t / Commission+Slippage / Funding / Borrow / LiquidationLoss / net_r
/// / 守恒残差）。
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
         口径：margin=CME-simple 单段近似；cost=参数化常费率（funding 1bp/8h、borrow 0.01bp/bar、\
         liq 0.5%）。**有效域 L1**：机制真装 + 参数化费率，真实 funding/借贷历史是外部数据缺口（A10 \
         waiver 豁免外部数据源，不豁免机制）。**照实：预期成本拖累 net_r<gross，成本真实化非 alpha 声明。**\n\n",
    );
    // A10 附则B 裁决2（090 措辞纪律）：一切带成本 R 数值报告强制口径标签——费率未标定，
    // 常费率数值禁作 alpha 论据/策略择优输入；datum 注入后升 [L2费率标定: datum 版本哈希]。
    report.push_str(&format!(
        "**口径标签：{}**（A10 附则B 强制；TW桥列＝A10 C5 对账行 ⌊funding+borrow+liq⌋——TW 账本不经构造子见持盾成本，η=tw() 高估在险权益恰此量）\n\n\
         | 窗 | 臂 | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r | 守恒残差 | TW桥 | n_orders |\n\
         |---|---|---|---|---|---|---|---|---|---|---|\n",
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

/// ★M8 端到端全策略 OOS 跑批（TARGET_STRATEGY_MAXFULL.md M8:161-168 / 路线.pdf p17,p20-21）：
/// 三系统**同时开启**（M5 overlay 声部执行臂 + M6 cost_model 成本 + M7 三阶段 TW 账本）跑同一 BTC
/// OOS 窗，产四层报告：
/// - **(1) signal 层**：`LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0`——既有 25 桶双门结论（`wverify_full`/
///   `type1_goal` 终判：无方向 alpha，INCONCLUSIVE），本跑批**转引不重算**（signal 层是 M1-M4
///   本体，端到端只消费其结论，措辞纪律§5.3：signal 结果不外推 max-full）。
/// - **(2) execution 层**：`E[R(Π_exec)]`（`net_result.r_decomp.net_r`）、MaxDD（`metrics.max_drawdown`）、
///   turnover（`n_orders`）、逐声部归因（overlay by-role）——判据 `E[R(Π_exec)]>0`。
/// - **(3) treasury 层**：`Reach(Stage)`/`Q_T`/`W_T`/`η_T`（新接 `OverlayRunResult.tw_final`）——判据
///   `Reach(StageIII)>0 ∧ Q_T>Q_0 ∧ W_T≥I_0 ∧ η_T≥η_*`。
/// - **(4) 完整策略层**：`R(Π_max-full)`（含浮盈总额）+ `LCB_OOS(R)`（block bootstrap 2.5 分位，
///   `significance.boot_ci95_lo`）+ 三态——判据 **`LCB_OOS(R)>0` 才 confirmed alpha**；仅 `R>0`
///   但 LCB≤0 ⟹ **INCONCLUSIVE**（M8:168）。
///
/// **三系统同开的接线证据**：三系统共用同一主 loop `pi_theta_fill_loop_overlay`——cost_model 从
/// `config.cost_model` 读（M6）、TW 账本 loop 内建（M7）、overlay 是 hook（M5）。`run_theta_v0_pi_overlay`
/// 一次调用即三系统全开，无独立组合层（本轮修的唯一接线缺口：overlay 臂此前丢弃 `fill.tw_final`，
/// treasury 层拿不到终态——已加 `OverlayRunResult.tw_final` 透传）。
///
/// **认识论 L2**（formalization-validity-domain 231号）：真实 BTC OOS 窗假设检验，可否证。预期
/// （signal 层无 alpha ⟹ 端到端大概率负/INCONCLUSIVE）照实——否定性结果是 M0-M8 主线合法终点
/// （措辞纪律§5.6：INCONCLUSIVE≠无 alpha）。窗口：p3 单折 + 前两个 anchored walk-forward（O(n²)
/// 前缀重分类，全 461万 bar 不可行，有界多窗，诚实声明覆盖范围）。
///
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`。
#[test]
#[ignore]
fn m8_e2e_all_systems_oos() {
    use super::metrics::significance;
    use super::runner::run_theta_v0_pi_overlay;
    use super::super::strategy::coverage::Vertical;
    use super::super::strategy::ledger::{RiskPolicy, TStage};

    let plain_cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain_cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let nav_of = |d: &data::Dataset| {
        d.bars.iter().find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * plain_cfg.tick.tick_size).unwrap_or(1.0) * 1000.0
    };

    // OOS 窗清单：p3 单折 + 前两个 anchored walk-forward（与 M6 跑批同窗，可差分对照）。
    let mut wins: Vec<(String, String, String)> = vec![
        ("p3fold".into(), "2023-01-01".into(), "2023-06-30".into()),
    ];
    if let Some(sw) = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC") {
        for w in sw.wf_anchored.iter().filter(|w| w.test_start >= OOS_START).take(2) {
            wins.push((format!("wf{}", w.i), w.test_start.into(), w.test_end.into()));
        }
    }
    // ★T3 (#172)/#164 复现副本同款先例：`M8_WIN_FILTER=<tag>` ⟹ 只跑指定窗（逐窗重放/shadow
    // dump 分窗落盘需要；未设 = 全窗清单不变，bit-exact 中性——只跳过其他窗，窗内行为逐字节同）。
    if let Ok(filter) = std::env::var("M8_WIN_FILTER") {
        wins.retain(|(tag, _, _)| tag == &filter);
    }

    let mut report = String::from(
        "# M8 端到端全策略 OOS（TARGET_STRATEGY_MAXFULL.md M8 / 路线.pdf p17,p20-21）\n\n\
         三系统同开：M5 overlay 声部执行臂 + M6 cost_model（参数化 funding/borrow/liq）+ M7 三阶段 TW 账本。\n\
         口径：margin=CME-simple 单段；cost=参数化常费率；κ=0 冻结（M7 c3 裁定，正 κ 推迟 M8 后 L3）。\n\
         **认识论 L2**：真实 BTC OOS 假设检验；signal 层无 alpha ⟹ 端到端负/INCONCLUSIVE 照实（否定性结果合法）。\n\n",
    );
    // A10 附则B 裁决2（090 措辞纪律）：带成本 R 数值报告强制口径标签（费率未标定，禁作 alpha 论据）。
    report.push_str(&format!(
        "**口径标签：{}**（A10 附则B 强制，cost 三常费率保底未标定；数值禁作 alpha 论据/策略择优输入）\n\n\
         ## 四层报告\n\n",
        super::super::strategy::risk::RATE_UNCALIBRATED_LABEL,
    ));

    // ── signal 层（转引，不重算）──
    report.push_str(
        "### 层1 signal alpha（转引 M1-M4 本体结论，不重算）\n\n\
         判据：`LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0`。既有终判（`wverify_full` / goal type1）：\
         **无方向 confirmed alpha**——25 桶双门下高级别桶 n_eff≪n_min（功效门 271~1083），\
         δ-free 主裁决 + μ_R 并列 co-primary 均未过 LCB>0。三态 = **INCONCLUSIVE**\
         （非「无 alpha 存在」，措辞§5.6）。**signal 结果不外推 max-full**（措辞§5.3）。\n\n\
         ### 层2/3/4（本跑批 L2 实测，三系统同开）\n\n\
         η 列口径注记（p128 裁定 (i)，A10 C5）：**η_corrected = tw() − cum_holding_cost 为唯一合法判读口径**\
         （cum_holding_cost = r_decomp.tw_holding_cost_bridge，与 M7 witness 增打两行同源；修正只降不升）；\
         η_T/η_* 原列保留对照。\n\n\
         | 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r(execR) | MaxDD | 声部数(A/S/F) | 终Stage | Q_T | W_T | η_T/η_* | cum_holding_cost | η_corrected(判读) | R(含浮盈) | LCB_OOS(R) | 三态 |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );

    let policy = RiskPolicy::baseline(); // κ=0（M7 冻结口径）
    let i0: i64 = 1_000_000; // I_0 基线（TwState notional_in 同源 = ⌊nav0⌋，此处报告门槛用 1e6 名义）
    for (tag, te_lo, te_hi) in &wins {
        let test = ds.slice_date_window(te_lo, te_hi);
        if test.bars.is_empty() {
            report.push_str(&format!("| {tag} | test 段空 | | | | | | | | | | | | | | | | | |\n"));
            continue;
        }
        let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
        let nav_te = nav_of(&test);
        // ★三系统同开：overlay 臂（M5）+ margin（M6 强平前置）+ cost_model（M6 成本）；TW 账本（M7）loop 内建。
        let mut cfg = ThetaConfig::default();
        // ★B1 步骤4（dw-sizing-diag-20260705）：m8_e2e 接 preset 切换——与 wverify_full 同 env 入口
        // （THETA_DIR_PRESET=follow/adversary），默认 Neutral bit-exact。此前 m8_e2e 恒 Neutral ⟹
        // 从未测 dir_weight 执行层效应；现在 Follow/Adversary 可经 env gate 测 execution R 分解。
        apply_theta_dir_preset_from_env(&mut cfg);
        apply_enforce_gross_cap_from_env(&mut cfg);
        apply_center_oscillation_from_env(&mut cfg); // issue #357 验收③：THETA_CENTER_OSCILLATION=1 覆盖
        cfg.margin = Some(q4_margin_model(nav_te));
        cfg.cost_model = Some(m6_cost_model());
        eprintln!("[m8] BTC {tag} test={te_lo}..{te_hi}({}) 三系统同开 run…", test.bars.len());
        // ★T5a (#207) shadow dump 分窗接线（T3_SHADOW_DUMP 同型）：env T5A_CHAIN_DUMP_DIR
        // 设置时逐窗开 `<dir>/t5a_chain_dump_<tag>.jsonl`；未设 = no-op（bit-exact 中性）。
        super::admission::t5a_chain_dump::open_for_window(&tag);
        let r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
        super::admission::t5a_chain_dump::close();

        // 层2 execution：R 分解 + MaxDD + 逐声部归因。
        let d = r.net_result.r_decomp.expect("overlay 臂经生产 π loop ⟹ 产 R 分解");
        let maxdd = r.net_result.metrics.max_drawdown;
        let (mut n_amb, mut n_short, mut n_follow) = (0usize, 0usize, 0usize);
        for c in r.overlay.closed_voices() {
            match c.role_v {
                Vertical::Ambient => n_amb += 1,
                Vertical::ReverseOpen => n_short += 1,
                Vertical::FollowParent => n_follow += 1,
            }
        }

        // 层3 treasury：TW 终态（新接的 tw_final）。
        let tw = r.tw_final.expect("overlay 臂主 loop 内建 TW 账本 ⟹ tw_final=Some");
        let stage_str = match tw.stage {
            TStage::CostReduction => "I(降成本)",
            TStage::CapitalRecovered => "II(已回本)",
            TStage::EarningShares => "III(赚份额)",
        };
        let eta_t = tw.tw();
        let eta_star = policy.eta_star(&tw);
        // ★p128 裁定 (i)（A10 C5 口径，additive）：η_corrected = tw() − cum_holding_cost，
        // cum_holding_cost = r_decomp.tw_holding_cost_bridge（与 M7 witness 增打两行同源，禁第二查法）。
        // η_corrected 为唯一合法判读口径（修正只降不升，T-N4 语义）；η_T 原列保留对照。
        let cum_holding_cost = d.tw_holding_cost_bridge;
        let eta_corrected = eta_t - cum_holding_cost;

        // 层4 完整策略：R(含浮盈) + LCB_OOS(R) block bootstrap。
        let r_total: f64 = r.net_result.trade_pnls_with_forced.iter().sum();
        let sig = significance(
            &r.net_result.trade_pnls, // 已实现口径（bootstrap H0:收益≤0 输入）
            &r.net_result.daily_returns,
            &r.net_result.trades,
            &r.net_result.prices,
            r.net_result.fee_rate,
            r.net_result.theta_return_mtm,
        );
        let lcb_r = sig.boot_ci95_lo; // LCB_OOS(R) = block bootstrap 总收益 2.5 分位下界
        // 三态（完整策略层，M8:168）：LCB>0 ⟹ confirmed；R>0∧LCB≤0 ⟹ INCONCLUSIVE；R≤0 ⟹ 无（本层）。
        let verdict = if lcb_r > 0.0 {
            "CONFIRMED"
        } else if r_total > 0.0 {
            "INCONCLUSIVE"
        } else {
            "无(R≤0)"
        };

        // ★issue #357 验收③：enabled=true 时 campaign 生产接线产物级见证（减补动作归属分桶/
        // CenterNotAlive 丢弃率/挂起归宿/campaign 生死事件）；enabled=false 时 witness 恒空，
        // 本段照实打印零读数（非省略——证明门关时确实零覆盖，非未触达的假阴性）。
        {
            let w = &r.campaign_witness;
            let drop_rate = if w.trigger_attempts > 0 {
                w.dropped_center_not_alive as f64 / w.trigger_attempts as f64
            } else {
                0.0
            };
            let broken_by_level = w.broken_by_level();
            eprintln!(
                "[m8][#357] {tag}: center_oscillation.enabled={} trigger_attempts={} \
                 dropped_center_not_alive={} ({:.1}%) dropped_other={} \
                 dropped_center_moved_down={} \
                 unclosed_write_off_count={:?} unclosed_write_off_units_gap={:?} \
                 unclosed_write_off_cash_booked={:?} unclosed_write_off_nothing_to_settle={:?} \
                 death_write_off_count={:?} death_write_off_units_gap={:?} \
                 death_write_off_cash_booked={:?} \
                 action_by_level={:?} \
                 suspension_by_source={:?} reset_settlement_count={} \
                 reset_alive_center_leak_by_level_side={:?} broken_by_level={:?} \
                 cross_side_termination_count={} suspension_continued_count={:?} \
                 settlement_by_side={:?} historical_bound_by_level={:?} \
                 historical_multi_owner_same_event={:?} \
                 center_mis_kill_by_level={:?} \
                 replenish_foreign_center_count={:?} \
                 lifecycle_opened={:?} lifecycle_died={:?} \
                 no_active_campaign_count={:?} \
                 resource_exhausted_holding_negative_count={:?} \
                 loss_round_trip_accounted_count={:?} defense_units_exceed_current_holding_count={:?} \
                 replenish_triggered_but_full_count={:?} \
                 stage_recover_capital_count={} stage_enter_earning_count={} stage_events={:?} \
                 profit_ready_but_suspended={:?} \
                 earning_mode_switch_bar={:?} earning_replenish_count={:?} \
                 earning_units_gained={:?} earning_cash_unsound_count={:?} \
                 earning_sizing_rounds_to_zero_count={:?} \
                 cover_by_side={:?} \
                 other_violation_count={} other_violation_by_kind={:?} campaign_active_end={}",
                cfg.center_oscillation.enabled,
                w.trigger_attempts,
                w.dropped_center_not_alive,
                drop_rate * 100.0,
                w.dropped_other_trigger,
                // ★#366/#472：未闭合减出核销四项读数（货缺口/现金分列的产物级见证）。
                w.dropped_center_moved_down,
                w.unclosed_write_off_count,
                w.unclosed_write_off_units_gap,
                w.unclosed_write_off_cash_booked,
                w.unclosed_write_off_nothing_to_settle,
                // ★#441（ADR 补充十二）：death 吞挂起核销三读数——与上方 #366/#472
                // 结构终结核销桶分列（结构终结 vs campaign 死），货缺口与现金不相减。
                w.death_write_off_count,
                w.death_write_off_units_gap,
                w.death_write_off_cash_booked,
                w.action_by_level,
                w.suspension_by_source,
                // ★#489：Reset 清算须为 0；漏发按级别/一类点侧；Broken/级与跨侧终结显式呈报。
                w.reset_settlement_count(),
                w.reset_alive_center_leak_by_level_side,
                broken_by_level,
                w.cross_side_termination_count(),
                // ★#414/#472：延续计数（取代不再终结挂起）/ 清算终局分流（闭合·核销二分）。
                w.suspension_continued_count,
                w.settlement_by_side,
                w.historical_bound_by_level,
                // ★#487：多 Owner 同事件的产物级观测桶（合法多投递，不参与路由或清算判据）。
                w.historical_multi_owner_same_event,
                w.center_mis_kill_by_level,
                w.replenish_foreign_center_count,
                w.lifecycle_opened,
                w.lifecycle_died,
                w.no_active_campaign_count,
                w.resource_exhausted_holding_negative_count,
                w.loss_round_trip_accounted_count,
                w.defense_units_exceed_current_holding_count,
                w.replenish_triggered_but_full_count,
                w.stage_recover_capital_count,
                w.stage_enter_earning_count,
                w.stage_events,
                // ★#384 终验补列：阶段推进的第四项读数——「够本可推进但仍有挂起未收口」的正面
                // 计数（`profit_ready_but_suspended`，ADR 补充十的到 0 判据「挂起空 ∧ free≥本金」
                // 的另一半）。该桶此前只有单测覆盖、从未进 wf8 报告行 ⟹ 真实窗口读数不可见。
                w.profit_ready_but_suspended,
                // ★#383：阶段三报告层三项（切换时点=生效 bar / 等金额回补次数 / 累计净增股数）
                // + 两条分流桶（硬门恒 0；等金额腿买不起一股属预期场景）。
                w.earning_mode_switch_bar,
                w.earning_replenish_count,
                w.earning_units_gained,
                w.earning_cash_unsound_count,
                w.earning_sizing_rounds_to_zero_count,
                w.cover_by_side,
                w.other_violation_count,
                w.other_violation_by_kind,
                r.campaign_book.active_count(),
            );
            // ★no_active_campaign_count/resource_exhausted_holding_negative_count 非 bug——前者
            // 是结构信号独立于本级持仓状态的预期空仓触发（#292 CenterOscillationBook「无门」
            // 设计），后者是 campaign 按级别（非按中枢）聚合共享同一份冻结 sizing 预算、连续同向
            // 触发耗尽 holding 时 cash_sound_gate 的显式拒绝（真实资源约束，见 CampaignWiringWitness
            // 字段文档）；两者均不断言恒 0。`unsupported_short_position_count`（issue #357 关票
            // 条件 C）★#381 已退役：空头侧现有自己的 `(level, Short)` campaign 正常记账，
            // 该桶所指的「有仓但认不出」形态不复存在，读数改由 `action_by_level` 的 `"short"`
            // 侧分桶与 `lifecycle_*` 呈现。`other_violation_count` 才是真正的接线/记账逻辑
            // 错误警报，必须恒 0。
            //
            // ★#380 四项读数（ADR 补充七）：`loss_round_trip_accounted_count`（项一，亏损往返
            // 如实入账——接替退役的 `resource_exhausted_free_negative_count` 拒绝桶）、
            // `defense_units_exceed_current_holding_count`（项二，防线读当时真实持仓的超卖拒绝）、
            // `stage_*`/`stage_events`（项三，阶段推进事件见证：级别+开局以来 bar 数+金额）、
            // `cover_by_side` 与 `replenish_triggered_but_full_count`（项四，挂起归属冲抵
            // 顺序与「触发但货满」）——四者均为预期读数，照实呈现，不断言恒 0。
            assert_eq!(
                w.other_violation_count, 0,
                "接线/记账逻辑错误计数必须恒 0（NoActiveCampaign/未支持空头/资源耗尽除外，见对应分桶字段）"
            );
        }

        report.push_str(&format!(
            "| {tag} | {} | {:+.0} | {:.0} | {:.0} | {:.0} | {:.0} | {:+.0} | {:.4} | {}/{}/{} | {} | {} | {} | {}/{} | {} | {} | {:+.0} | {:+.0} | {} |\n",
            r.net_result.n_orders, d.price_pnl_gross, d.commission_slippage, d.funding, d.borrow,
            d.liquidation_loss, d.net_r, maxdd, n_amb, n_short, n_follow,
            stage_str, tw.notional_in, tw.withdrawn, eta_t, eta_star,
            cum_holding_cost, eta_corrected, r_total, lcb_r, verdict,
        ));
        eprintln!(
            "[m8] {tag}: execR={:+.0} MaxDD={:.4} stage={} R={:+.0} LCB(R)={:+.0} → {}",
            d.net_r, maxdd, stage_str, r_total, lcb_r, verdict,
        );

        // 守恒硬校验（R 分解无泄漏，与 M6 同容差）。
        let tol = 1e-3_f64.max(1e-9 * (nav_te.abs() + d.price_pnl_gross.abs()));
        assert!(
            d.conservation_residual.abs() <= tol,
            "M8 {tag} R 守恒残差 {} 超容差 {}（资金泄漏）", d.conservation_residual, tol,
        );
        // treasury 单向不可逆：stage.rank ≤ 2（EarningShares 上界），且 W_T≤notional_in（退本金不超投入）。
        assert!(tw.withdrawn <= tw.notional_in, "W_T={} 不得超 notional_in={}", tw.withdrawn, tw.notional_in);
    }

    report.push_str(&format!(
        "\n## 判据结算（M8:163-168）\n\n\
         - **层1 signal**：INCONCLUSIVE（转引，无方向 alpha）。\n\
         - **层2 execution** `E[R(Π_exec)]>0`：见 net_r 列（成本真实化后极负 = 高频费主导，非机制缺陷）。\n\
         - **层3 treasury** `Reach(StageIII)>0`：见终Stage 列（signal 无 alpha ⟹ 已实现 PnL 无正累积 ⟹ \
           三阶段停 CostReduction，与 M7 c3 witness 一致）。\n\
         - **层4 完整策略** `LCB_OOS(R)>0`：见 LCB_OOS(R) 列——**未过 ⟹ INCONCLUSIVE**，\
           不宣称 confirmed alpha（措辞§5.6：INCONCLUSIVE≠无 alpha；§5.3：不外推 max-full）。\n\n\
         I_0 报告门槛 = {i0}（notional_in 同源 ⌊nav0⌋，各窗 nav 不同 ⟹ 门槛按 notional_in 列读）。\n",
    ));
    std::fs::write("/tmp/m8_e2e_all_systems_oos.md", &report).ok();
    eprintln!("[m8] 端到端四层报告落盘 /tmp/m8_e2e_all_systems_oos.md");
}

/// ★#270（SPEC #268 T2）翻向守卫修复回归固化：wf8 单窗重放红环 + 40 笔真翻向保护集断言。
///
/// 本测试是 #264 反馈环（`/tmp/bug264_red_loop.sh` + `/tmp/bug264_assert.py`，人类记忆里的
/// /tmp 脚本）的**仓内永久版**——「1-bar 误杀」缺陷（#233 守卫把出生即恒真的 σ=−ε 两轴对立
/// 误判为父翻向，t+1 必剪；wf8 92.3% 交易持仓恰好 1 bar）今后任何人再犯立刻变红。
/// #269（commit 09e4307239）事件化修复后本测试转绿。
///
/// **① 红环断言**（与 /tmp 环同一病理线）：wf8 窗 L1/L3 各自的 **1-bar prune 占比 ≤ 50%**
/// （>50% = 多数仓秒死 = 病态；阈值 50% 为编排者选定的病理线，#261 终裁可调）。
/// #269 修复后读数（2026-07-25 在案，本测试 2026-07-26 复跑复核一致）：
/// L1 51/123 = 41.5%、L3 10/37 = 27.0%，持仓中位 L1 38 / L3 132 bar（修复前 74.5%/94.4%、
/// 中位 1/1）。
///
/// **② 40 笔真翻向保护集断言**（票面「事件口径下仍判 prune」逐笔锁定）：保护集 = bug 态产物
/// （`/tmp/v4_C5_rerun_20260725`，#233 状态轴守卫轨，856 笔）中 hold>1 且 via_structural_prune
/// 的 40 笔（#264 实证 95% 出场 bar 有塔结构事件——真正该杀的笔）。#269 复核
/// （`/tmp/bug264_review_40.py`）：**仍杀 37 / 放过 0 / 判不出 3**。
/// - 仍杀 37 笔：键 (level, ordinal, entry_bar) 硬编码于 [`FLIP_GUARD_PROT_STILL_37`]——
///   断言每键在新轨迹**入场复现且全部候选仍 via_structural_prune**（误伤真翻向判定立刻红）；
/// - 判不出 3 笔（如实登记，不静默放过也不静默杀）：`L0#101 entry=11761`（旧 hold=537
///   CloseRoot）、`L1#51 entry=25893`（旧 hold=172 CloseRoot）、`L0#664 entry=75926`
///   （旧 hold=464 CloseShortDiff）——#269 轨迹下同 (voice,entry_bar) 入场未复现（轨迹分叉：
///   载体被前序存活腿占用/候选湮灭，入场侧零改前提下的合法分叉），事件口径下无从判定，
///   交编排者。断言口径：键**若复现则必须仍判 prune**（永不静默放过）；缺席为在案状态。
///
/// 接线口径（与红环逐字同构）：wf8 = BTC anchored i=8（test 2023-08-17..2024-02-16）；
/// VOICE_EXEC/OPSEM_DUMP_DIR 经**线程局部** override 注入（并行安全——进程级 env 会被并行
/// 测试的 fill loop 读到并 truncate 同一 trades.jsonl，2026-07-13 竞态实录同型规避）；
/// 断言消费 dump 的 trades.jsonl（opsem 只读旁路，生产路径 bit-exact 中性）。
///
/// `#[ignore]`: `cargo test --release --lib theta_v0::backtest::wverify_run::flip_guard_wf8_onebar_prune_replay -- --ignored --nocapture`
#[test]
#[ignore = "#270 wf8 翻向守卫回归；需 BTC 数据（DATA BLOCKER 不伪造）"]
fn flip_guard_wf8_onebar_prune_replay() {
    use super::runner::run_theta_v0_pi_overlay;

    /// 40 笔保护集中 #269 复核「仍杀」的 37 笔键（level, ordinal, entry_bar）——
    /// 旧轨（bug 态）hold>1 真翻向笔，事件口径下必须仍判 prune。
    const FLIP_GUARD_PROT_STILL_37: [(u32, u64, i64); 37] = [
        (1, 21, 11520), (3, 2, 18252), (0, 163, 20309), (0, 193, 23249),
        (0, 236, 28030), (0, 242, 28532), (1, 75, 35531), (0, 363, 42241),
        (0, 397, 45517), (0, 399, 45688), (1, 99, 45938), (0, 476, 54875),
        (1, 127, 59416), (1, 127, 59966), (0, 643, 73841), (1, 161, 75428),
        (1, 245, 117139), (1, 262, 126454), (1, 271, 131111), (0, 1218, 138803),
        (0, 1227, 139741), (1, 315, 149455), (0, 1334, 151638), (1, 328, 154603),
        (0, 1459, 165520), (3, 21, 177410), (0, 1632, 185949), (1, 387, 186470),
        (1, 421, 204718), (0, 1783, 204945), (1, 423, 205795), (0, 2009, 233150),
        (1, 487, 235755), (0, 2081, 241132), (2, 122, 243225), (1, 542, 259968),
        (1, 548, 263584),
    ];
    /// 判不出 3 笔（#269 §5 如实列出交编排者）：若复现必须仍判 prune；缺席为在案状态。
    const FLIP_GUARD_PROT_UNKNOWN_3: [(u32, u64, i64); 3] = [
        (0, 101, 11761), (1, 51, 25893), (0, 664, 75926),
    ];

    // ── wf8 窗重放（与 m8_e2e_all_systems_oos 的 M8_WIN_FILTER=wf8 臂同窗同配置）──
    let plain_cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain_cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
    let w = sw.wf_anchored.iter().find(|w| w.i == 8).expect("wf8 窗（#264 症状窗）");
    let test = ds.slice_date_window(w.test_start, w.test_end);
    assert!(!test.bars.is_empty(), "wf8 test 段非空（否则测试空转）");
    let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let nav_te = test
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * plain_cfg.tick.tick_size)
        .unwrap_or(1.0)
        * 1000.0;
    let mut cfg = ThetaConfig::default();
    apply_theta_dir_preset_from_env(&mut cfg);
    apply_enforce_gross_cap_from_env(&mut cfg);
    cfg.margin = Some(q4_margin_model(nav_te));
    cfg.cost_model = Some(m6_cost_model());

    // dump 目录唯一化（并行/残留进程互不惊扰）；override 测试末尾复位。
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间晚于 epoch")
        .as_nanos();
    let dump_dir = std::env::temp_dir().join(format!(
        "flip_guard_wf8_replay_{}_{}",
        std::process::id(),
        nonce
    ));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dump_dir.clone()));
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(true)));
    let _r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(None));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);

    // ── 消费 dump（红环同一数据源）：voice_id/entry/exit/prune 四字段即可。 ──
    let text = std::fs::read_to_string(dump_dir.join("trades.jsonl"))
        .expect("OPSEM dump 启用 ⟹ trades.jsonl 落盘");
    let _ = std::fs::remove_dir_all(&dump_dir);
    struct Row {
        level: u32,
        ordinal: u64,
        entry_bar: i64,
        exit_bar: i64,
        prune: bool,
    }
    let mut rows: Vec<Row> = Vec::new();
    for line in text.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("trades.jsonl 行合法 JSON");
        rows.push(Row {
            level: v["voice_id"]["level"].as_u64().expect("level") as u32,
            ordinal: v["voice_id"]["ordinal"].as_u64().expect("ordinal"),
            entry_bar: v["entry_bar"].as_i64().expect("entry_bar"),
            exit_bar: v["exit_bar"].as_i64().expect("exit_bar"),
            prune: v["via_structural_prune"].as_bool().expect("via_structural_prune"),
        });
    }
    assert!(!rows.is_empty(), "wf8 dump 非空（否则测试空转）");

    // ── ① 红环断言：L1/L3 1-bar prune 占比 ≤ 50% 病理线（>50% = RED）──
    for lv in [1u32, 3u32] {
        let ts: Vec<&Row> = rows.iter().filter(|r| r.level == lv).collect();
        assert!(!ts.is_empty(), "wf8 L{lv} 笔集非空（否则断言空转）");
        let one = ts.iter().filter(|r| r.prune && r.exit_bar - r.entry_bar <= 1).count();
        let share = one as f64 / ts.len() as f64;
        let mut holds: Vec<i64> = ts.iter().map(|r| r.exit_bar - r.entry_bar).collect();
        holds.sort_unstable();
        let med = holds[holds.len() / 2];
        eprintln!(
            "[#270] L{lv}: {} 笔, 1-bar prune {} ({:.1}%), 持仓中位 {med} bar -> {}",
            ts.len(),
            one,
            share * 100.0,
            if share > 0.5 { "RED" } else { "ok" },
        );
        assert!(
            share <= 0.5,
            "#264 症状回归：wf8 L{lv} 1-bar prune 占比 {:.1}% 越 50% 病理线\
             （多数仓秒死=病态；翻向守卫须只在真翻向事件时开火，#268）",
            share * 100.0,
        );
    }

    // ── ② 40 笔真翻向保护集：37 笔仍杀逐键锁定 + 3 笔判不出登记口径 ──
    let mut n_still = 0usize;
    for &(level, ordinal, entry_bar) in &FLIP_GUARD_PROT_STILL_37 {
        let cands: Vec<&Row> = rows
            .iter()
            .filter(|r| r.level == level && r.ordinal == ordinal && r.entry_bar == entry_bar)
            .collect();
        assert!(
            !cands.is_empty(),
            "保护集键 L{level}#{ordinal} entry={entry_bar} 入场未复现——\
             #269 在案为仍杀键（复现态偏移须逐笔对账后更新本表，禁静默）"
        );
        assert!(
            cands.iter().all(|r| r.prune),
            "真翻向保护集键 L{level}#{ordinal} entry={entry_bar} 出现非 prune 候选——\
             误伤真翻向判定（#233 已结算条款倒退），事件口径下必须仍判 prune"
        );
        n_still += 1;
    }
    let mut n_unknown_absent = 0usize;
    for &(level, ordinal, entry_bar) in &FLIP_GUARD_PROT_UNKNOWN_3 {
        let cands: Vec<&Row> = rows
            .iter()
            .filter(|r| r.level == level && r.ordinal == ordinal && r.entry_bar == entry_bar)
            .collect();
        if cands.is_empty() {
            n_unknown_absent += 1; // #269 在案状态：入场未复现（轨迹分叉合法），如实计数。
        } else {
            assert!(
                cands.iter().all(|r| r.prune),
                "判不出登记键 L{level}#{ordinal} entry={entry_bar} 复现且非 prune——\
                 永不静默放过（若复现须仍判 prune；语义变化须逐笔对账后更新登记）"
            );
        }
    }
    eprintln!(
        "[#270] 40 笔保护集：仍杀键锁定 {n_still}/37 全绿；判不出登记键缺席 {n_unknown_absent}/3\
         （#269 在案 3 笔全缺席：L0#101/11761、L1#51/25893、L0#664/75926）"
    );
}

/// ★#291（SPEC #274 T1）中枢生命周期事件机 wf8 自证：born/broken/reset 三类事件在 wf8 窗
/// 真实产出且计数合理（ADR 0001 修正案一·补充二「中枢=事件」机械化首次在真实数据上见证）。
///
/// 接线口径（与 [`flip_guard_wf8_onebar_prune_replay`] 逐字同构）：wf8 = BTC anchored i=8
/// （test 2023-08-17..2024-02-16）；VOICE_EXEC/OPSEM_DUMP_DIR 经**线程局部** override 注入
/// （并行安全）；断言消费 dump 的 `center_lifecycle.jsonl`（opsem 只读旁路第三产物，生产路径
/// bit-exact 中性——零行为变化的实证 = 同目录 trades.jsonl/tower_events.jsonl 与基线逐字段一致，
/// 该对拍在命令行臂 `M8_WIN_FILTER=wf8 OPSEM_DUMP_DIR=/tmp/center_lifecycle_dump` 下另跑，见票面）。
///
/// ★**#336 R3 口径重写**（旧断言随独立复算路径作废）：事件源已改为消费塔链
/// （`classification.levels[ℓ].centers`），`born_seg`（段号）→ `chain_idx`（链下标），
/// `cleared_segs`（段序列清零证据）**已删除**（本机无段序列）。新增诊断行类
/// `chain_sync`（adopt/rebase）/ `superseded`（链推进取代）/ `stale`（陈旧死亡请求）。
///
/// ★**#337 口径再修**（容读法 + 两形态分桶，用户裁定 2026-07-26）：
/// - Δidx=−1（被取代的旧中枢，其死亡通知晚一格到）对 `broken` 已改判**合法放行**，
///   带 `slot:"tail_prev"`；Reset 不消费容读格，只广播并见证当下主格。**可机检锚**：
///   产物里若还剩 Δidx=−1 的 `stale`，它**只可能**是
///   「该身份此前已收过教义死亡」的二次死亡请求（容读格一次性）——测试逐行累积教义死亡身份集
///   并对每条 Δidx=−1 的 stale 反查；查不到即容读判据没接上（真回归）。
/// - 每条登记了中枢下场的行带 `death_form`（`doctrinal` = 三类破坏；
///   `arena_termination` = 被链推进取代）；Reset 的 `death_form` 恒 null，非空 `died_*`
///   只承载活中枢漏发见证。
/// - `superseded` 由「每 bar 每级一行带 count」改为**逐实例事件行**（带身份 + `chain_idx` +
///   `by_chain_idx`）；`chain_sync` 行补 `tail_*`/`prev_*` 身份与 `revived` 复活标志。
///
/// 断言（结构性不变量 + 计数合理性，均不涉轨迹数值——轨迹不变由对拍臂证）：
/// - 每 born：`zd ≤ zg`（核心非空 = 中枢成立判据，机检不变量）+ 含 `chain_idx`；
/// - 每 broken：`zd ≤ zg` + 含 `chain_idx` + `death_form=="doctrinal"` + `slot ∈ {tail,tail_prev}`；
/// - 每 reset：`death_form==null`；`died_*` 非 null 时
///   `alive_center_leak==true ∧ died_zd ≤ died_zg` 且 `slot` 只定位见证，场空时三者皆空/false；
/// - 每 superseded：`death_form=="arena_termination"` ∧ `by_chain_idx == chain_idx + 1`；
/// - 每 stale：`target_idx < alive_idx`，且 Δidx=−1 者须已在此前收过教义死亡（★#337 锚）；
/// - 计数：`born ≥ 1`（L0 必现）∧ 逐级 `broken ≤ born`（破坏必先有出生）；
/// - ★**MAJOR-A 举证换锚**：锁死解除的举证**不再挂在「miskill=0」上**（那是口径收窄后的
///   不可比读数，见 `center_lifecycle.rs` [`CenterMisKill`] 文档），改挂在**出生身份的多样性**
///   上——断言 L0 born 覆盖 ≥100 个**不同** `(si,zd,zg)` 身份（#331 交付态是「51 次 born 只
///   2 个身份」的复锁；R3 实测 550 个）。这条才是「吸收态锁死已消灭」的直接反证。
/// - 诊断行计数（resync / chain_sync / superseded / stale / miskill / revived）如实打印。
///
/// ⚠️ 旧断言「broken ≥ 1（wf8 有三类点 ⟹ 破坏必现）」**改为如实打印不再硬断言**：R3 下三类点
/// 能否成为 broken 取决于其载体是否落在在场窗内，落更上游 ⟹ 陈旧请求（不是破坏）。
/// 「wf8 有三类点」不再蕴含「必有 broken」，硬断言会把口径变化伪装成回归。
///
/// `#[ignore]`：需 BTC 数据（DATA BLOCKER 不伪造）；wf8 全窗重放。
/// `cargo test --release --lib theta_v0::backtest::wverify_run::center_lifecycle_wf8_events_replay -- --ignored --nocapture`
#[test]
#[ignore = "#291 wf8 中枢生命周期事件自证；需 BTC 数据（DATA BLOCKER 不伪造）"]
fn center_lifecycle_wf8_events_replay() {
    use super::runner::run_theta_v0_pi_overlay;

    // ── wf8 窗重放（与 flip_guard_wf8_onebar_prune_replay 同窗同配置）──
    let plain_cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain_cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
    let w = sw.wf_anchored.iter().find(|w| w.i == 8).expect("wf8 窗");
    let test = ds.slice_date_window(w.test_start, w.test_end);
    assert!(!test.bars.is_empty(), "wf8 test 段非空（否则测试空转）");
    let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let nav_te = test
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * plain_cfg.tick.tick_size)
        .unwrap_or(1.0)
        * 1000.0;
    let mut cfg = ThetaConfig::default();
    apply_theta_dir_preset_from_env(&mut cfg);
    apply_enforce_gross_cap_from_env(&mut cfg);
    cfg.margin = Some(q4_margin_model(nav_te));
    cfg.cost_model = Some(m6_cost_model());

    // dump 目录唯一化（并行/残留进程互不惊扰）；override 测试末尾复位。
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间晚于 epoch")
        .as_nanos();
    let dump_dir = std::env::temp_dir().join(format!(
        "center_lifecycle_wf8_replay_{}_{}",
        std::process::id(),
        nonce
    ));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dump_dir.clone()));
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(true)));
    let _r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(None));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);

    // ── 消费 center_lifecycle.jsonl（#291 第三产物）──
    let text = std::fs::read_to_string(dump_dir.join("center_lifecycle.jsonl"))
        .expect("OPSEM dump 启用 ⟹ center_lifecycle.jsonl 落盘");
    // trades/tower_events 同目录仍在（本测试不比对内容——轨迹不变由命令行臂对拍证，见头注）。
    assert!(
        dump_dir.join("trades.jsonl").exists() && dump_dir.join("tower_events.jsonl").exists(),
        "opsem 三产物并列（trades/tower_events/center_lifecycle）"
    );
    let _ = std::fs::remove_dir_all(&dump_dir);

    // (born, broken, reset, resync) 逐级计数。
    let mut counts: std::collections::BTreeMap<u64, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    let mut n_resync = 0usize;
    let mut n_miskill = 0usize;
    let mut n_lines = 0usize;
    // ★#336 R3 新诊断行类。
    let mut n_chain_sync = 0usize;
    let mut n_superseded = 0usize;
    let mut n_stale = 0usize;
    // ★#337：容读格分桶（放行落在链尾 vs 链尾前一格）+ 重基复活实证 + 锁死解除举证。
    let mut n_slot_tail = 0usize;
    let mut n_slot_tail_prev = 0usize;
    let mut n_reset_alive_leak = 0usize;
    let mut n_revived = 0usize;
    let mut born_ids: std::collections::BTreeMap<u64, std::collections::BTreeSet<(i64, i64, i64)>> =
        std::collections::BTreeMap::new();
    // 已收过教义死亡登记的实例身份（按行序累积；容读格一次性判据的机检输入）。
    let mut doctrinally_dead: std::collections::BTreeSet<(u64, i64, i64, i64)> =
        std::collections::BTreeSet::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        n_lines += 1;
        let v: serde_json::Value =
            serde_json::from_str(line).expect("center_lifecycle.jsonl 行合法 JSON");
        let level = v["level"].as_u64().expect("level");
        let kind = v["kind"].as_str().expect("kind");
        let e = counts.entry(level).or_default();
        match kind {
            "born" => {
                let zd = v["zd"].as_i64().expect("zd");
                let zg = v["zg"].as_i64().expect("zg");
                assert!(zd <= zg, "born 核心非空不变量：zd={zd} ≤ zg={zg}");
                assert!(v["chain_idx"].as_u64().is_some(), "★R3：born 含链下标 chain_idx");
                // ★#337 MAJOR-A：出生**身份**入集合（锁死解除的举证锚，见头注）。
                born_ids
                    .entry(level)
                    .or_default()
                    .insert((v["si"].as_i64().expect("born 含 si"), zd, zg));
                e.0 += 1;
            }
            "broken" => {
                let zd = v["zd"].as_i64().expect("zd");
                let zg = v["zg"].as_i64().expect("zg");
                assert!(zd <= zg, "broken 死中枢核心非空：zd={zd} ≤ zg={zg}");
                assert!(v["chain_idx"].as_u64().is_some(), "★R3：broken 含链下标 chain_idx");
                assert_eq!(
                    v["death_form"].as_str(),
                    Some("doctrinal"),
                    "★#337：三类点破坏 = 教义死亡形态"
                );
                match v["slot"].as_str() {
                    Some("tail") => n_slot_tail += 1,
                    Some("tail_prev") => n_slot_tail_prev += 1,
                    other => panic!("★#337：broken slot ∈ {{tail,tail_prev}}，实得 {other:?}"),
                }
                doctrinally_dead.insert((level, v["si"].as_i64().expect("broken 含 si"), zd, zg));
                e.1 += 1;
            }
            "reset" => {
                assert!(v["death_form"].is_null(), "★#489：Reset 不登记教义死亡");
                if !v["died_zd"].is_null() {
                    let zd = v["died_zd"].as_i64().expect("died_zd");
                    let zg = v["died_zg"].as_i64().expect("died_zg");
                    assert!(
                        zd <= zg,
                        "Reset 漏发见证的活中枢核心非空：died_zd={zd} ≤ died_zg={zg}"
                    );
                    assert_eq!(v["alive_center_leak"].as_bool(), Some(true));
                    assert_eq!(
                        v["slot"].as_str(),
                        Some("tail"),
                        "★#489：Reset 只读见证当前主格，不读取容读格载体"
                    );
                    assert!(v["died_si"].as_i64().is_some(), "漏发见证含 CenterId.si");
                    n_reset_alive_leak += 1;
                } else {
                    // 场为空的一类点广播：无漏发见证 ⟹ 格位仍为 null（不编造）。
                    assert!(
                        v["alive_center_leak"].as_bool() == Some(false) && v["slot"].is_null(),
                        "★#489：场空 Reset 不得带漏发见证/格位"
                    );
                }
                e.2 += 1;
            }
            "resync" => n_resync += 1,
            // ★#329：误杀拒绝诊断行（触发点载体身份 ≠ 在场中枢身份 ⟹ 事件机拒杀）。非教义
            // 事件，与 resync 同属诊断行，单列不进 born/broken/reset 计数。
            "miskill" => {
                assert!(v["alive_si"].as_u64().is_some(), "miskill 行含在场中枢身份 alive_si");
                assert!(v["alive_zd"].as_i64().is_some(), "miskill 行含在场中枢 alive_zd");
                assert!(v["alive_zg"].as_i64().is_some(), "miskill 行含在场中枢 alive_zg");
                assert_eq!(v["trigger"].as_str(), Some("third"), "Reset 不具有 miskill 能力");
                n_miskill += 1;
            }
            // ★#336 R3 诊断行：链同步（adopt/rebase）——静默采纳，不伪造出生/死亡。
            "chain_sync" => {
                assert!(
                    matches!(v["reason"].as_str(), Some("adopt") | Some("rebase")),
                    "chain_sync reason ∈ {{adopt,rebase}}"
                );
                assert!(v["chain_len"].as_u64().is_some(), "chain_sync 含 chain_len");
                // ★#337 MAJOR-B：身份字段 + 复活标志（链空时 tail_* 为 null，故只校字段在场）。
                let chain_len = v["chain_len"].as_u64().expect("chain_sync 含 chain_len");
                if chain_len >= 1 {
                    assert!(
                        v["tail_si"].as_i64().is_some(),
                        "★#337：非空链的 chain_sync 须带在场实例身份 tail_si"
                    );
                }
                if chain_len >= 2 {
                    assert!(
                        v["prev_si"].as_i64().is_some(),
                        "★#337：链长≥2 的 chain_sync 须带容读格身份 prev_si"
                    );
                }
                match v["revived"].as_bool() {
                    Some(true) => n_revived += 1,
                    Some(false) => {}
                    None => panic!("★#337：chain_sync 须带 revived 布尔（重基复活实证）"),
                }
                n_chain_sync += 1;
            }
            // ★#337：在场终结事件行（被链推进取代）——由 #336 的「每 bar 每级一行带 count」
            // 聚合诊断行升为**逐实例事件行**（带身份 + 链下标），与教义死亡分桶。
            "superseded" => {
                assert_eq!(
                    v["death_form"].as_str(),
                    Some("arena_termination"),
                    "★#337：被取代 = 在场终结形态"
                );
                let idx = v["chain_idx"].as_u64().expect("superseded 含 chain_idx");
                let by = v["by_chain_idx"].as_u64().expect("superseded 含 by_chain_idx");
                assert_eq!(by, idx + 1, "取代者恒为链上紧邻后一格（idx={idx} by={by}）");
                n_superseded += 1;
            }
            // ★#336 R3 诊断行：陈旧死亡请求（载体命中链上已退场实例）。
            "stale" => {
                let ai = v["alive_idx"].as_u64().expect("stale 含 alive_idx");
                let ti = v["target_idx"].as_u64().expect("stale 含 target_idx");
                assert!(
                    ti < ai,
                    "stale = 载体落游标上游（target_idx={ti} < alive_idx={ai}）"
                );
                // ★#337 容读法的可机检锚：Δidx=−1（容读格）**只可能**因「该实例此前已收过教义
                // 死亡」而被拒（容读格一次性 / 已死实例不得二次死亡）。若出现一条 Δidx=−1 而
                // 该身份此前**没有**任何教义死亡登记，就是容读判据没接上（真回归）。
                if ai - ti == 1 {
                    let id = (
                        level,
                        v["target_si"].as_i64().expect("stale 含 target_si"),
                        v["target_zd"].as_i64().expect("stale 含 target_zd"),
                        v["target_zg"].as_i64().expect("stale 含 target_zg"),
                    );
                    assert!(
                        doctrinally_dead.contains(&id),
                        "★#337 容读法回归：Δidx=−1 的死亡请求被判 stale，但该身份 {id:?} \
                         此前无任何教义死亡登记 ⟹ 容读格本应放行"
                    );
                }
                assert_eq!(v["trigger"].as_str(), Some("third"), "Reset 不具有 stale 杀伤请求");
                n_stale += 1;
            }
            other => panic!("未知事件类：{other}"),
        }
    }
    assert!(n_lines > 0, "wf8 事件流非空（否则测试空转）");

    // 计数合理性：L0 born ≥ 1；逐级 broken ≤ born（破坏必先有出生）。
    // ★R3：不再硬断言 `broken ≥ 1`（见头注——三类点能否成为 broken 取决于载体是否在游标处）。
    let l0 = counts.get(&0).copied().unwrap_or((0, 0, 0));
    assert!(l0.0 >= 1, "L0 中枢出生必现（wf8 26 万 bar 段数以千计）");
    let (mut tb, mut tk, mut tr) = (0usize, 0usize, 0usize);
    for (lvl, (b, k, r)) in &counts {
        assert!(k <= b, "L{lvl} 破坏({k}) ≤ 出生({b})（破坏必先有出生）");
        tb += b;
        tk += k;
        tr += r;
    }
    // ★#337 MAJOR-A：锁死解除的举证锚 = 出生**身份**的多样性（不是 miskill=0——那是口径收窄
    // 后的不可比读数）。#331 交付态的复锁形态是「51 次 born 只 2 个身份」。
    let l0_ids = born_ids.get(&0).map(|s| s.len()).unwrap_or(0);
    assert!(
        l0_ids >= 100,
        "★MAJOR-A：L0 出生须覆盖 ≥100 个不同中枢身份（吸收态锁死已消灭的直接反证），实得 {l0_ids}"
    );
    let ids_per_level: Vec<(u64, usize)> = born_ids.iter().map(|(l, s)| (*l, s.len())).collect();
    eprintln!(
        "[#291/#336 R3/#337 容读法/#489 Reset 广播] wf8 中枢生命周期（塔链消费）：\
         born={tb} broken={tk} reset={tr} reset_alive_leak={n_reset_alive_leak} \
         miskill={n_miskill}（载体不在链上）stale={n_stale}（容读窗外 或 二次死亡请求）\
         superseded={n_superseded}（在场终结事件行）chain_sync={n_chain_sync} \
         revived={n_revived}（重基复活）resync={n_resync}；\
         ★两形态分桶：教义死亡 {}（其中链尾 {n_slot_tail} / 容读格 {n_slot_tail_prev}）\
         + 在场终结 {n_superseded}；★出生身份数/级 {ids_per_level:?}；逐级 {counts:?}",
        n_slot_tail + n_slot_tail_prev,
    );
}

/// D4（#606 S1 第三修复车：换结构，level 传参穿透，删反查机器）：一类点 T3-in-c 固定首对
/// 分级观测——wf8 全窗重放。观测挂点 = 生产 `judge_segment`（`judge_first_cached` 返回后、
/// `points.push` 前）——真实调用级别（`extract_first_third_resume` 新增的 `level: u32` 参数，
/// 由 `mod.rs` 逐级循环 `level_idx` 直接传入）随捕获同时写入，不再需要收尾阶段按身份反查
/// `classification.levels[lvl].bsp` 补齐（旧版反查/候选分配机器已删，见
/// `OtherwiseDomainSidecarCollector` 模块头谱系注记）。逐个 `buy1 ∨ sell1` 点产一条
/// [`classifier::signal::FirstClassGradeRecord`]（`grade` 含 `Present`/`Missing(reason)`）。
///
/// 三项统计口径（原「三锁」表述过誉——键唯一是 upsert `HashMap` 的结构性保证，重新验证它只是
/// 验证数据结构本身，非独立断言，本版不再列为「锁」）：
///
/// ①**账平**（记录总数 = native(Present) + otherwise(Missing)）——构造性恒等：这是
/// `T3InCGrade` 定义本身的 `Present`/`Missing` 二分对同一个 `records` vec 的重新求和（同一份
/// 数据分两类计数再相加，数学上必然回到原总数），不是独立验证，**不作验收证据陈列**，只作
/// 统计展示（供 #585 逐案对拍读数）。
///
/// ②**基数对拍**（`sidecar.records.len()` 与独立读 `center_lifecycle.jsonl` 的 `kind=="reset"`
/// 行数相等）——两个独立数据源的**总数**核对，不是按 `(level,source_index,side)` 逐键核对
/// 独立统计的 `bsp` 一类点计数（代码里没有任何这样的 per-key 计数）。基数相等不排除"漏一多一"
/// 相抵（一处漏记、另一处多记，总数照样对得上）；Reset 广播与一类点记录也只是间接对应（广播
/// 时点=瞬时 true 那帧，记录 grade=末次重判，见
/// [`super::opsem_dump::OtherwiseDomainSidecarSummary`] 文档 F7 口径登记）。
///
/// ③**五桶分级分侧计数**（missing_leave/missing_retest/same_direction/leave_not_outside/
/// retest_reentered）——计数分级分侧打印进验收行，供人工核对。
///
/// `#[ignore]`：需 BTC 数据（DATA BLOCKER 不伪造）；wf8 全窗重放，
/// env `THETA_OTHERWISE_DOMAIN_SIDECAR=1`（测试内部设置，无需外部前缀）。
/// `cargo test --release --lib theta_v0::backtest::wverify_run::otherwise_domain_wf8_grade_buckets -- --ignored --nocapture`
#[test]
#[ignore = "#606 S1 第三修复车 D4：wf8 全窗一类点分级观测三项口径 + 五桶；需 BTC 数据（DATA BLOCKER 不伪造）"]
fn otherwise_domain_wf8_grade_buckets() {
    use super::super::classifier::signal::{T3InCGrade, T3InCGradeReason};
    use super::super::types::Side;
    use super::runner::run_theta_v0_pi_overlay;

    let plain_cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain_cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
    let w = sw.wf_anchored.iter().find(|w| w.i == 8).expect("wf8 窗");
    let test = ds.slice_date_window(w.test_start, w.test_end);
    assert!(!test.bars.is_empty(), "wf8 test 段非空（否则测试空转）");
    let years = test.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let nav_te = test
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * plain_cfg.tick.tick_size)
        .unwrap_or(1.0)
        * 1000.0;
    let mut cfg = ThetaConfig::default();
    apply_theta_dir_preset_from_env(&mut cfg);
    apply_enforce_gross_cap_from_env(&mut cfg);
    cfg.margin = Some(q4_margin_model(nav_te));
    cfg.cost_model = Some(m6_cost_model());

    // 开臂（THETA_CENTER_OSCILLATION=1）+ D4 sidecar + OPSEM dump 同一次重放（与 S1 报告
    // §4.2 命令口径一致——独立 dump 目录唯一化，override 测试末尾复位，同
    // `center_lifecycle_wf8_events_replay` 先例）。
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间晚于 epoch")
        .as_nanos();
    let dump_dir = std::env::temp_dir().join(format!(
        "otherwise_domain_wf8_grade_dump_{}_{}",
        std::process::id(),
        nonce
    ));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dump_dir.clone()));
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(true)));
    std::env::set_var("THETA_CENTER_OSCILLATION", "1");
    std::env::set_var("THETA_OTHERWISE_DOMAIN_SIDECAR", "1");
    let r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
    std::env::remove_var("THETA_OTHERWISE_DOMAIN_SIDECAR");
    std::env::remove_var("THETA_CENTER_OSCILLATION");
    super::admission::VOICE_EXEC_OVERRIDE.with(|c| c.set(None));
    super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);

    let sidecar = r.net_result.otherwise_domain_sidecar.expect("env 门开 ⟹ sidecar Some");
    assert!(sidecar.frames >= 1, "至少观察到一帧（wf8 非空窗）");

    // ── 独立对照：`center_lifecycle.jsonl` 的 Reset 行（一类确认事件 → Reset 广播，#489/#585
    // 同口径，与 sidecar 记录数分别独立统计）——逐行同时取出 `level`/`trigger_side`/
    // `alive_center_leak`，供②基数对拍 + 下方 D5 逐级对拍（两者共用同一次解析，不重复读盘）──
    let lifecycle_text = std::fs::read_to_string(dump_dir.join("center_lifecycle.jsonl"))
        .expect("OPSEM dump 启用 ⟹ center_lifecycle.jsonl 落盘");
    let _ = std::fs::remove_dir_all(&dump_dir);
    let reset_rows: Vec<(u32, u8, bool)> = lifecycle_text
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let v: serde_json::Value =
                serde_json::from_str(line).expect("center_lifecycle.jsonl 行合法 JSON");
            if v["kind"].as_str() != Some("reset") {
                return None;
            }
            let level = v["level"].as_u64().expect("reset 行含 level") as u32;
            let side_u8 = match v["trigger_side"].as_str().expect("reset 行含 trigger_side") {
                "Long" => 0u8,
                "Short" => 1u8,
                other => panic!("reset 行 trigger_side 未知取值：{other}"),
            };
            let leak = v["alive_center_leak"]
                .as_bool()
                .expect("reset 行含 alive_center_leak");
            Some((level, side_u8, leak))
        })
        .collect();
    let reset_count = reset_rows.len();

    // ── ③五桶分级分侧计数 + 分级分侧 native/otherwise 总表（供 #585 逐案对拍）。键唯一由
    // `OtherwiseDomainSidecarCollector` 内部 upsert `HashMap` 结构性保证（见其模块头谱系
    // 注记），本测试不再重复断言（重新验证只是验证数据结构本身，非独立证据）──
    // (level, side) -> [missing_leave, missing_retest, same_direction, leave_not_outside, retest_reentered]
    let mut bucket_counts: std::collections::BTreeMap<(u32, u8), [usize; 5]> =
        std::collections::BTreeMap::new();
    // (level, side) -> (native, otherwise)
    let mut level_side_totals: std::collections::BTreeMap<(u32, u8), (usize, usize)> =
        std::collections::BTreeMap::new();
    let mut native_count = 0usize;
    let mut otherwise_count = 0usize;
    for rec in &sidecar.records {
        let side_u8 = match rec.side {
            Side::Long => 0u8,
            Side::Short => 1u8,
        };
        let totals = level_side_totals.entry((rec.level, side_u8)).or_insert((0, 0));
        match rec.grade {
            T3InCGrade::Present { .. } => {
                native_count += 1;
                totals.0 += 1;
            }
            T3InCGrade::Missing(reason) => {
                otherwise_count += 1;
                totals.1 += 1;
                let bucket_idx = match reason {
                    T3InCGradeReason::MissingLeave => 0,
                    T3InCGradeReason::MissingRetest => 1,
                    T3InCGradeReason::SameDirection => 2,
                    T3InCGradeReason::LeaveNotOutside => 3,
                    T3InCGradeReason::RetestReentered => 4,
                };
                bucket_counts.entry((rec.level, side_u8)).or_insert([0; 5])[bucket_idx] += 1;
            }
        }
    }

    // ── ②基数对拍：sidecar 记录数 = center_lifecycle Reset 行数（两个独立数据源的总数核对，
    // 非按 (level,source_index,side) 逐键核对独立 bsp 计数——不排除漏一多一相抵，见上文档）──
    let grand_total = sidecar.records.len();
    assert_eq!(
        grand_total, reset_count,
        "②基数对拍：sidecar 记录数({grand_total}) == center_lifecycle Reset 行数({reset_count})"
    );

    // ── ①账平（构造性恒等，非独立验证，仅供 #585 对拍统计口径行；见上文档）──
    assert_eq!(
        grand_total,
        native_count + otherwise_count,
        "①账平：一类点总数 = 趋势一类(native) + 否则域(otherwise)（Present/Missing 二分恒等）"
    );

    // ── D5 逐级对拍（#606 S1 抛光车终审 F-中1）：② 只核对总数，本断言逐桶核对——records 按
    // (level,side) 分桶总数（native+otherwise）与 center_lifecycle reset 行按 (level,trigger_side)
    // 分桶重新分 leak=true/false 两类计数。两个独立数据源在每个桶上应满足
    // `records桶总数 - leak=true桶计数 == leak=false桶计数`（若一桶 record 有多算/漏算，diff 会偏离该桶
    // leak=false 计数，逐桶断言比②的总数断言更细，能抓总数抵消但分桶错位的情形）。照实测写，
    // 不预先硬编码期望值。
    let mut reset_leak_true: std::collections::BTreeMap<(u32, u8), usize> =
        std::collections::BTreeMap::new();
    let mut reset_leak_false: std::collections::BTreeMap<(u32, u8), usize> =
        std::collections::BTreeMap::new();
    for &(level, side_u8, leak) in &reset_rows {
        let map = if leak { &mut reset_leak_true } else { &mut reset_leak_false };
        *map.entry((level, side_u8)).or_insert(0) += 1;
    }
    let mut d5_keys: std::collections::BTreeSet<(u32, u8)> =
        level_side_totals.keys().copied().collect();
    d5_keys.extend(reset_leak_true.keys().copied());
    d5_keys.extend(reset_leak_false.keys().copied());
    for key in &d5_keys {
        let record_count = level_side_totals.get(key).map(|(n, o)| n + o).unwrap_or(0);
        let leak_true = reset_leak_true.get(key).copied().unwrap_or(0);
        let leak_false = reset_leak_false.get(key).copied().unwrap_or(0);
        let diff = record_count as i64 - leak_true as i64;
        assert_eq!(
            diff, leak_false as i64,
            "D5 逐级对拍 (level={},side={}): records 桶总数({record_count}) - \
             reset leak=true 桶计数({leak_true}) 应恰等于该桶 leak=false 计数({leak_false})",
            key.0, key.1
        );
    }
    eprintln!(
        "[#606 S1 D5] 逐级对拍：records(level,side)→总数={:?}；reset(level,trigger_side)→\
         leak=true 计数={reset_leak_true:?}；leak=false 计数={reset_leak_false:?}",
        level_side_totals
            .iter()
            .map(|(k, (n, o))| (*k, n + o))
            .collect::<std::collections::BTreeMap<_, _>>(),
    );

    eprintln!(
        "[#606 S1 D4] wf8 一类点 T3-in-c 分级观测：frames={} 一类点总数={grand_total} \
         (center_lifecycle reset={reset_count}) native(趋势一类)={native_count} \
         otherwise(否则域)={otherwise_count} \
         (level,side=0long/1short)→(native,otherwise)={level_side_totals:?} \
         ③五桶(level,side)→[missing_leave,missing_retest,same_direction,\
         leave_not_outside,retest_reentered]={bucket_counts:?}",
        sidecar.frames,
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  #327 真覆盖见证锁：#321 从严判据（单点核心 ZD==ZG 不成立）落在中枢级联变动块上的命中断言
// ════════════════════════════════════════════════════════════════════════════
//
// ## 为什么要这把锁（#323 影子评审 Critical-2）
//
// #321/#323 把中枢核心非空判据从弱口径（`zd<=zg`，单点核心成立）改严为 `zd<zg`。原实施复跑的
// 四把 BTC 见证锁（`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity`
// / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke`）全部零翻动——但影子评审查证
// 出：**四把锁的窗口（BTC OOS 前 16000 bar）内 `zd==zg` 命中 0 次**。零翻动是**零覆盖**的结果，
// 不构成安全证据（`.chanlun/review-results/center-strict-single-point-impl-20260726.md` §3.2/§3.3）。
//
// 本节两把锁交付「真覆盖」：**先证明判据分支真的开火（命中>0）**，再断言开火处的行为符合新口径
// （单点不成立），最后把级联重排计数固化为 GOLDEN。
//
// ## 反事实是锁自身的一部分（先红后绿内建）
//
// 两把锁都同时跑**双口径**级联：生产（严格）+ 旧弱口径本地副本。逐命中点断言
// `strict==None ∧ weak==Some ∧ zd==zg` —— 这一对断言若把生产判据退回弱口径立刻红（strict 侧
// 变 Some），若把本地副本写错也立刻红。锁不依赖「跑过一次记住数字」，它自带对照臂。
//
// ## 复算路径（`classify_impl` 中枢级联的只读复刻，非新算法）
//
// [`cascade_dual`] 复刻 `classifier::classify_impl` 的**中枢级联**：`units → detect_centers_windowed_resume
// → decompose → LeveledMove::compose → project_to_units → 下一级 units`。BSP/背驰/投影层不复刻——
// 它们**不回流**中枢级联（`classify_impl` 里 bsp/pan_div/level_projection 只写 `LevelState`，
// 不参与下一级 units 的构造）。复刻的忠实性由 [`cascade_dual`] 严格臂与生产 `classify` 的逐级中枢
// 序列**逐字段相等**守恒断言证（见 `assert_cascade_faithful`），不是口头声明。

use super::super::classifier::center as classifier_center;
use super::super::classifier::center::UnitRange;
use super::super::parser::ParseLayer;
use crate::theta_v0::types::{Center, Segment, Tick};

/// L0 线段 → 走势单元（`classifier::mod` 私有 `segment_to_unit` 的本地镜像，`p89_dual_core_audit`
/// 同款先例——私有函数不为测试放开可见性，本地镜像逐字段复刻并由守恒断言兜底）。
fn c327_seg_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange { start_index: seg.start_index, end_index: seg.end_index, direction: seg.direction, lo, hi }
}

/// 核心上沿 `computeZG` = min(三段 hi)（`center::compute_zg` 私有，本地镜像，口径 B 全三段）。
fn c327_zg(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.hi.min(b.hi).min(c.hi)
}
/// 核心下沿 `computeZD` = max(三段 lo)（`center::compute_zd` 私有，本地镜像，口径 B 全三段）。
fn c327_zd(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.lo.max(b.lo).max(c.lo)
}

/// ★旧弱口径（#321 **前**）完整判据副本——反事实臂，**不是**生产路径。
///
/// 与 `center::center_from_segments` 逐字同构，唯一差别 = 支2 用弱口径 `zd > zg` 才拒
/// （⟹ 单点核心 `zd==zg` **成立**）。这正是 #321 改掉的那一行。
fn c327_weak_center_from_segments(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    if !classifier_center::dir_alternates(a, b, c) {
        return None;
    }
    let (zd, zg) = (c327_zd(a, b, c), c327_zg(a, b, c));
    if zd > zg {
        return None;
    }
    Some(Center {
        zd,
        zg,
        dd: a.lo.min(b.lo.min(c.lo)),
        gg: a.hi.max(b.hi.max(c.hi)),
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// ★旧弱口径（#321 **前**）几何判据副本——反事实臂（上级递归层，无方向交替支）。
fn c327_weak_center_from_window(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    let (zd, zg) = (c327_zd(a, b, c), c327_zg(a, b, c));
    if zd > zg {
        return None;
    }
    Some(Center {
        zd,
        zg,
        dd: a.lo.min(b.lo.min(c.lo)),
        gg: a.hi.max(b.hi.max(c.hi)),
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// 一个 `zd==zg` 判据命中点（本票的「真覆盖」原子）——携三元组本体，供逐点双口径复判。
#[derive(Debug, Clone, Copy)]
struct C327Hit {
    /// 命中三元组在本级 `units` 中的起点下标。
    unit_i: usize,
    /// 单点核心价位（`zd==zg` 的公共值，tick）。
    core: Tick,
    /// 三元组首单元的**原始 bar 序**起点（`Segment.start_index` 同口径，非 merged 序，供日期定位）。
    src_start: usize,
    /// 命中所在的检测级（0 = #323 报告口径的 L0；决定复判走完整判据还是几何判据）。
    level: usize,
    /// 命中三元组本体（逐点复判的输入，不再回查 units ⟹ 断言与枚举同源）。
    triple: [UnitRange; 3],
}

impl C327Hit {
    /// 逐点见证：**新口径判不成立 ∧ 旧弱口径判成立 ∧ 核心确为单点**。
    ///
    /// 三条一起断言才是「真覆盖」——只断言 `strict.is_none()` 无法区分「因单点被拒」与「本来
    /// 就不是中枢（方向不交替/核心真空）」；补上弱臂 `Some` 才锁死「这一步的差异恰由 #321 改动
    /// 产生」。生产判据若退回弱口径，`strict.is_none()` 立刻红。
    fn assert_single_point_rejected(&self) {
        let [a, b, c] = &self.triple;
        assert_eq!(
            c327_zd(a, b, c),
            c327_zg(a, b, c),
            "L{} unit_i={} 命中枚举自洽：核心须为单点 zd==zg",
            self.level,
            self.unit_i
        );
        let (strict, weak) = if self.level == 0 {
            (
                classifier_center::center_from_segments(a, b, c),
                c327_weak_center_from_segments(a, b, c),
            )
        } else {
            (
                classifier_center::center_from_window(a, b, c),
                c327_weak_center_from_window(a, b, c),
            )
        };
        assert!(
            strict.is_none(),
            "#321 从严：L{} unit_i={} 单点核心 [{},{}] 须判**不成立**（生产判据返回 Some ⟹ \
             口径倒退回弱口径）",
            self.level,
            self.unit_i,
            self.core,
            self.core
        );
        let w = weak.unwrap_or_else(|| {
            panic!(
                "反事实臂失效：L{} unit_i={} 旧弱口径应判**成立**（单点核心闭区间合法）——\
                 弱臂返回 None ⟹ 本命中点不构成 #321 改动的覆盖证据",
                self.level, self.unit_i
            )
        });
        assert_eq!(
            (w.zd, w.zg),
            (self.core, self.core),
            "L{} unit_i={} 旧弱口径产出的中枢核心须恰为该单点",
            self.level,
            self.unit_i
        );
    }
}

/// 一条口径臂的级联产出。
#[derive(Debug, Default)]
struct C327Arm {
    /// `centers[k]` = 第 k 级检测产出的中枢序列（k=0 即 #323 报告口径的 L0）。
    centers: Vec<Vec<Center>>,
    /// `hits[k]` = 第 k 级 `units` 上 `zd==zg` 的三元组（L0 另需方向交替成立——支1 不成立时
    /// 支2 根本不被求值，计进去就是伪覆盖）。
    hits: Vec<Vec<C327Hit>>,
}

/// 双口径中枢级联复算（严格臂 = 生产判据，弱臂 = #321 前旧口径副本）。
///
/// 复刻 `classify_impl` 的中枢级联（见本节模块注释「复算路径」）：逐级
/// `detect_centers_windowed_resume(units, build, 0)` → `decompose` → `LeveledMove::compose`
/// → `project_to_units` → 下一级 units；自然终止条件（`units.len() < min_parts` / `units` 空 /
/// `l_max` 上界）与生产同源读 `config.level`。
fn cascade_dual(l0: &ParseLayer, config: &ThetaConfig, weak: bool) -> C327Arm {
    use super::super::classifier::decompose::decompose;
    use super::super::classifier::recursive_tower::{
        detect_centers_windowed_resume, project_to_units, ElementId, LeveledMove,
    };

    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;
    let mut units: Vec<UnitRange> = l0.segments.iter().map(c327_seg_to_unit).collect();
    let mut arm = C327Arm::default();
    if units.is_empty() {
        return arm;
    }
    let mut tower: Vec<LeveledMove> = units
        .iter()
        .enumerate()
        .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
        .collect();

    for level_idx in 0..=l_max {
        if units.len() < min_parts {
            break;
        }
        let is_l0 = level_idx == 0;
        let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = match (is_l0, weak) {
            (true, false) => classifier_center::center_from_segments,
            (false, false) => classifier_center::center_from_window,
            (true, true) => c327_weak_center_from_segments,
            (false, true) => c327_weak_center_from_window,
        };

        // 判据命中枚举：本级 units 上**全部**连续三元组（扫描游标只走其中一部分，但「判据在此
        // 数据上是否有 ZD==ZG 落点」是 units 的性质，与游标路径无关——覆盖度问题问的正是这个）。
        let mut hits: Vec<C327Hit> = Vec::new();
        for i in 0..units.len().saturating_sub(2) {
            let (a, b, c) = (&units[i], &units[i + 1], &units[i + 2]);
            if is_l0 && !classifier_center::dir_alternates(a, b, c) {
                continue;
            }
            let (zd, zg) = (c327_zd(a, b, c), c327_zg(a, b, c));
            if zd == zg {
                hits.push(C327Hit {
                    unit_i: i,
                    core: zd,
                    src_start: a.start_index,
                    level: level_idx,
                    triple: [*a, *b, *c],
                });
            }
        }

        let (windowed, _metas, _cursor) = detect_centers_windowed_resume(&units, build, 0);
        let centers: Vec<Center> = windowed.iter().map(|(c, _)| *c).collect();
        let blocks = decompose(&centers);
        let upper: Vec<LeveledMove> = windowed
            .iter()
            .enumerate()
            .map(|(i, (c, win))| {
                LeveledMove::compose(
                    &tower[win.0..=win.1],
                    *c,
                    level_idx as u32 + 1,
                    ElementId { level: level_idx as u32 + 1, ordinal: i as u64 },
                )
            })
            .collect();

        arm.centers.push(centers);
        arm.hits.push(hits);
        units = project_to_units(&upper, &blocks);
        tower = upper;
        if units.is_empty() {
            break;
        }
    }
    arm
}

/// 守恒：[`cascade_dual`] 严格臂 == 生产 `classify` 逐级中枢序列（复刻忠实性的机检，非声明）。
fn assert_cascade_faithful(strict: &C327Arm, l0: &ParseLayer, config: &ThetaConfig) {
    let prod = super::super::classifier::classify(l0, config);
    assert_eq!(
        strict.centers.len(),
        prod.levels.len(),
        "复刻级数须 == 生产级数（否则自然终止条件漂移，级联计数不可信）"
    );
    for (k, lv) in prod.levels.iter().enumerate() {
        assert_eq!(
            strict.centers[k].as_slice(),
            lv.centers.as_slice(),
            "L{k} 复刻严格臂中枢序列须与生产 classify 逐字段相等（复刻忠实性守恒）"
        );
    }
}

/// LCS 长度（`Center` 逐字段相等为「同一元素」）。序列长度 ≤ 万量级，O(n·m) DP 可行。
fn c327_lcs_len(a: &[Center], b: &[Center]) -> usize {
    let m = b.len();
    let mut prev = vec![0usize; m + 1];
    let mut cur = vec![0usize; m + 1];
    for x in a {
        for j in 1..=m {
            cur[j] = if *x == b[j - 1] { prev[j - 1] + 1 } else { prev[j].max(cur[j - 1]) };
        }
        std::mem::swap(&mut prev, &mut cur);
        cur.iter_mut().for_each(|v| *v = 0);
    }
    prev[m]
}

/// 一级的双臂对照读数（严格臂中枢数 / 弱臂中枢数 / LCS 长度）。
///
/// **级联重排计数** = `strict + weak - 2·lcs`（对称差：严格臂里被换掉的 + 弱臂里被换掉的）。
/// #323 报告 §2.1 的 `L0 26 / L1 18 / L2 8 / L3 3 / L4 2` 就是这个量——本锁的独立复算把它拆成
/// 三元读数固化，任一分量漂移都点名到级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct C327LevelReading {
    strict: usize,
    weak: usize,
    lcs: usize,
}

impl C327LevelReading {
    const fn new(strict: usize, weak: usize, lcs: usize) -> Self {
        Self { strict, weak, lcs }
    }
    /// 级联重排数（对称差）。
    fn rearranged(&self) -> usize {
        self.strict + self.weak - 2 * self.lcs
    }
}

/// ★GOLDEN（#327 首次实测，2026-07-26 在案）：BTC 全量 `btc_1m_full.json` 数据身份。
/// 数据换版 ⟹ 本锁点名报数据面变化，而不是把 GOLDEN 漂移伪装成口径回归。
const C327_FULL_BTC_DATA_ID: (usize, usize, usize) = (4_613_599, 3_037_868, 40_003);

/// ★GOLDEN（#327 首次实测）：全量 BTC 逐级 (严格臂中枢数, 弱臂中枢数, LCS)。
/// L5 为空级（自然终止后的零中枢级，`classify` 同产 6 级）。
const C327_FULL_BTC_LEVELS: [C327LevelReading; 6] = [
    C327LevelReading::new(9260, 9266, 9250),
    C327LevelReading::new(2086, 2088, 2078),
    C327LevelReading::new(446, 446, 442),
    C327LevelReading::new(85, 84, 83),
    C327LevelReading::new(13, 13, 12),
    C327LevelReading::new(0, 0, 0),
];

/// ★#323 报告 §2.1 转录的级联重排计数（L0..L4）——本锁**独立复算**后与之对账。
/// #323 §2.4 明记该表「转录自实施方自旗读数，本次返工未独立重跑复现」；本锁即那次缺失的复现。
const C327_CASCADE_REARRANGE_323: [usize; 5] = [26, 18, 8, 3, 2];

/// ★GOLDEN（#327 首次实测）：全量 BTC 逐级 `zd==zg` 判据命中数。
/// #321 commit message 自旗「BTC L0 23 次 / L1 2 次」——本锁独立复现同读数（L2+ 为 0）。
const C327_FULL_BTC_HITS: [usize; 6] = [23, 2, 0, 0, 0, 0];

/// 一把见证锁的公共主体：双臂级联 → 复刻忠实性守恒 → 逐级读数对 GOLDEN → 逐命中点单点见证
/// → 真覆盖硬门。返回逐级（严格臂）命中点，供窗专属断言（如「命中确实落在 2022-02」）。
fn c327_run_witness(
    tag: &str,
    ds: &data::Dataset,
    cfg: &ThetaConfig,
    expect_levels: &[C327LevelReading],
    expect_hits: &[usize],
) -> Vec<Vec<C327Hit>> {
    let layer = super::super::parser::parse_layer(&ds.bars, cfg);
    eprintln!(
        "[#327/{tag}] bars={} merged={} segments={} 日期={}..{}",
        ds.bars.len(),
        layer.merged_bars.len(),
        layer.segments.len(),
        ds.dates.first().map(String::as_str).unwrap_or(""),
        ds.dates.last().map(String::as_str).unwrap_or(""),
    );

    let strict = cascade_dual(&layer, cfg, false);
    let weak = cascade_dual(&layer, cfg, true);
    // 复刻忠实性守恒：严格臂 == 生产 `classify` 逐级中枢序列（否则下面的计数全部不可信）。
    assert_cascade_faithful(&strict, &layer, cfg);

    // `Segment.start_index`（及其上级投影）是**原始 bar 序**下标（`map_src_to_close_idx` 同口径），
    // 非 merged 序 ⟹ 直接查 `ds.dates`。
    let date_of = |si: usize| -> String { ds.dates.get(si).cloned().unwrap_or_default() };

    let n_levels = strict.centers.len().max(weak.centers.len());
    assert_eq!(n_levels, expect_levels.len(), "[{tag}] 级数须与 GOLDEN 同");
    assert_eq!(n_levels, expect_hits.len(), "[{tag}] 命中表长度须与级数同");

    let mut total_hits = 0usize;
    for k in 0..n_levels {
        let sc: &[Center] = strict.centers.get(k).map(|v| &v[..]).unwrap_or(&[]);
        let wc: &[Center] = weak.centers.get(k).map(|v| &v[..]).unwrap_or(&[]);
        let sh = strict.hits.get(k).map(|v| &v[..]).unwrap_or(&[]);
        let reading = C327LevelReading::new(sc.len(), wc.len(), c327_lcs_len(sc, wc));
        eprintln!(
            "[#327/{tag}] L{k}: strict={} weak={} lcs={} 级联重排={} 命中={} 命中区间={}..{}",
            reading.strict,
            reading.weak,
            reading.lcs,
            reading.rearranged(),
            sh.len(),
            sh.first().map(|h| date_of(h.src_start)).unwrap_or_default(),
            sh.last().map(|h| date_of(h.src_start)).unwrap_or_default(),
        );
        for h in sh {
            eprintln!(
                "[#327/{tag}]   命中 L{k} unit_i={} 单点核心={} src={} date={}",
                h.unit_i,
                h.core,
                h.src_start,
                date_of(h.src_start)
            );
            // 逐点见证：新口径不成立 ∧ 旧弱口径成立 ∧ 核心确为单点（反事实内建，见方法 doc）。
            h.assert_single_point_rejected();
        }
        assert_eq!(
            sh.len(),
            expect_hits[k],
            "[{tag}] L{k} `zd==zg` 判据命中数偏离 GOLDEN（覆盖面变化须逐点对账后更新，禁静默）"
        );
        assert_eq!(
            reading, expect_levels[k],
            "[{tag}] L{k} 双臂中枢序列读数偏离 GOLDEN（strict/weak/lcs 任一分量变 ⟹ 级联重排计数不再是在案值）"
        );
        // ★生产真被打到的独立坐实：命中不为空 ⟹ 该级级联重排数须 >0（枚举口径命中与生产路径口径
        // 是两件事，见 docs/canonical-coverage-rust-impl.md B1 「①覆盖口径边界」；重排数 >0 才证明
        // 生产 `detect_centers_windowed_resume` 在这一级真的产出了不同的中枢序列，不只是枚举命中）。
        if !sh.is_empty() {
            assert!(
                reading.rearranged() > 0,
                "[{tag}] L{k} 命中 {} 次但级联重排数为 0——生产判据未被真实打到，覆盖仅是枚举口径上的\
                 名义命中，不构成 #321 改动的安全证据",
                sh.len()
            );
        }
        total_hits += sh.len();
    }

    // ★真覆盖硬门（本票存在的理由）：命中 0 ⟹ 这把锁与四把旧锁一样是零覆盖，不构成证据。
    assert!(
        total_hits > 0,
        "[{tag}] `zd==zg` 判据命中 0 次 ⟹ 本窗零覆盖，锁不构成 #321 改动的安全证据\
         （#323 §3.3：零翻动是零覆盖的结果）"
    );
    eprintln!("[#327/{tag}] 真覆盖：`zd==zg` 判据命中合计 {total_hits} 次（>0 即本锁覆盖成立）");

    // 逐级命中回给调用方（读数已在上面对完 GOLDEN；命中点本体供窗口/日期一类的窗专属断言）。
    strict.hits
}

/// ★#327 真覆盖见证锁 ①（全量对账臂）：BTC 全量数据上的 `zd==zg` 判据命中 + 级联重排计数
/// 与 #323 报告 §2.1 逐级对账。
///
/// 断言：
/// - **真覆盖**：`zd==zg` 命中合计 > 0（实测 L0 23 / L1 2，合计 25）——四把旧锁窗内为 0，本锁坐实覆盖；
/// - **单点不成立**：25 个命中点逐点断言生产判据返回 `None` ∧ 旧弱口径副本返回 `Some` 且核心
///   恰为该单点（反事实内建：生产判据退回弱口径立刻红）；
/// - **级联重排计数对账**：逐级 (strict, weak, lcs) 固化为 GOLDEN，其对称差与 #323 报告
///   `L0 26 / L1 18 / L2 8 / L3 3 / L4 2` **逐级相等**；
/// - **复刻忠实性守恒**：严格臂逐级中枢序列 == 生产 `classify`（[`assert_cascade_faithful`]）。
///
/// #323 §2.4 声明该表未经独立复现——本锁即那次缺失的复现，且**独立复算成立**（六级读数见
/// [`C327_FULL_BTC_LEVELS`]，对称差 26/18/8/3/2 与转录值逐级一致）。
///
/// `#[ignore]`：需 BTC 全量数据（DATA BLOCKER 不伪造）。实测耗时 ~5s（release）。
/// `cargo test --release --lib theta_v0::backtest::wverify_run::center_strict_zd_eq_zg_full_btc_cascade_witness -- --ignored --nocapture`
#[test]
#[ignore = "#327 真覆盖见证锁（全量级联对账）；需 BTC 数据（DATA BLOCKER 不伪造）"]
fn center_strict_zd_eq_zg_full_btc_cascade_witness() {
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let layer = super::super::parser::parse_layer(&ds.bars, &cfg);
    assert_eq!(
        (ds.bars.len(), layer.merged_bars.len(), layer.segments.len()),
        C327_FULL_BTC_DATA_ID,
        "BTC 全量数据身份偏离 GOLDEN（数据换版 ⟹ 下方计数全部须重定，非口径回归）"
    );
    drop(layer);

    c327_run_witness("full", &ds, &cfg, &C327_FULL_BTC_LEVELS, &C327_FULL_BTC_HITS);

    // ── #323 §2.1 级联重排计数逐级对账（L0..L4；L5 空级不在报告表内）──
    for (k, expect) in C327_CASCADE_REARRANGE_323.iter().enumerate() {
        assert_eq!(
            C327_FULL_BTC_LEVELS[k].rearranged(),
            *expect,
            "L{k} 级联重排计数 {} ≠ #323 报告 §2.1 转录值 {expect}——两者必须一致，\
             否则 #323 的爆炸半径披露与本锁的独立复算有一方不实",
            C327_FULL_BTC_LEVELS[k].rearranged()
        );
    }
    eprintln!(
        "[#327/full] #323 §2.1 级联重排对账通过：逐级 {:?} == 报告 {:?}",
        C327_FULL_BTC_LEVELS[..5].iter().map(|r| r.rearranged()).collect::<Vec<_>>(),
        C327_CASCADE_REARRANGE_323,
    );
}

/// #323 报告点名的变动块窗（「如 2022-02 段」）——`slice_date_window` 闭区间日窗。
/// 起点取 2022-01-01 是给 parser 留前置历史（切片重解析，窗首若紧贴命中点则命中随边界效应漂移）。
const C327_BLOCK_WINDOW: (&str, &str) = ("2022-01-01", "2022-02-28");

/// ★GOLDEN（#327 首次实测）：变动块窗数据身份 (bars, merged, segments)。
const C327_BLOCK_DATA_ID: (usize, usize, usize) = (84_960, 61_011, 710);

/// ★GOLDEN（#327 首次实测）：变动块窗逐级 (严格臂中枢数, 弱臂中枢数, LCS)。
const C327_BLOCK_LEVELS: [C327LevelReading; 4] = [
    C327LevelReading::new(153, 155, 151),
    C327LevelReading::new(34, 34, 33),
    C327LevelReading::new(8, 8, 8),
    C327LevelReading::new(2, 2, 2),
];

/// ★GOLDEN（#327 首次实测）：变动块窗逐级 `zd==zg` 判据命中数（两处命中都在 2022-02-01）。
const C327_BLOCK_HITS: [usize; 4] = [2, 0, 0, 0];

/// ★GOLDEN（#327 首次实测）：变动块窗逐级级联重排数（= [`C327_BLOCK_LEVELS`] 的对称差）。
/// L0 两处单点核心被拒 ⟹ L0 重排 6 个中枢 + 级联上传到 L1 再重排 2 个；L2/L3 该窗内已重新对齐。
const C327_BLOCK_REARRANGE: [usize; 4] = [6, 2, 0, 0];

/// ★GOLDEN（#327 首次实测）：变动块窗两处命中的日期前缀——票面「落在 2022-02 变动块」的字面见证。
const C327_BLOCK_HIT_DAY: &str = "2022-02-01";

/// ★GOLDEN（#327 首次实测）：变动块窗两处命中共享的单点核心价位（tick）。
const C327_BLOCK_HIT_CORE: Tick = 3_830_000_000_000;

/// ★#327 真覆盖见证锁 ②（变动块臂，**本票主交付**）：锁直接落在 #323 报告点名的 2022-02 变动块。
///
/// 与锁①（全量）的分工：①证明「全量口径下 #323 的级联重排计数复算成立」，②证明「**在报告点名的
/// 那个变动块上**判据真的开火、且开火处行为符合新口径」。②是票面要求的「落在变动块的见证锁」，
/// 窗小（两个月）跑得快，可作为改判据时的第一道快门。
///
/// 断言与①同构（真覆盖 >0 / 逐点单点不成立 + 反事实 / 逐级读数 GOLDEN / 复刻忠实性守恒），
/// 差别只在数据窗 = [`C327_BLOCK_WINDOW`]。
///
/// ⚠有效域：窗内数据经 `slice_date_window` **重新解析**（parser 无窗外历史），故本窗的中枢序列
/// **不等于**全量跑批在同区间的切片——两把锁的读数各自独立，不可互推。
///
/// `#[ignore]`：需 BTC 数据（DATA BLOCKER 不伪造）。
/// `cargo test --release --lib theta_v0::backtest::wverify_run::center_strict_zd_eq_zg_change_block_witness -- --ignored --nocapture`
#[test]
#[ignore = "#327 真覆盖见证锁（2022-02 变动块）；需 BTC 数据（DATA BLOCKER 不伪造）"]
fn center_strict_zd_eq_zg_change_block_witness() {
    let cfg = ThetaConfig::default();
    let full = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let ds = full.slice_date_window(C327_BLOCK_WINDOW.0, C327_BLOCK_WINDOW.1);
    assert!(!ds.bars.is_empty(), "变动块窗非空（否则测试空转）");
    let layer = super::super::parser::parse_layer(&ds.bars, &cfg);
    assert_eq!(
        (ds.bars.len(), layer.merged_bars.len(), layer.segments.len()),
        C327_BLOCK_DATA_ID,
        "变动块窗数据身份偏离 GOLDEN（数据换版 ⟹ 下方计数全部须重定，非口径回归）"
    );
    drop(layer);

    let hits = c327_run_witness("block", &ds, &cfg, &C327_BLOCK_LEVELS, &C327_BLOCK_HITS);

    // ── 窗专属①：级联重排逐级 GOLDEN（本窗自有读数，与 #323 全量表不同源，不可互推）──
    for (k, expect) in C327_BLOCK_REARRANGE.iter().enumerate() {
        assert_eq!(
            C327_BLOCK_LEVELS[k].rearranged(),
            *expect,
            "变动块窗 L{k} 级联重排计数偏离 GOLDEN"
        );
    }

    // ── 窗专属②：命中确实落在 2022-02（票面「落在变动块」的字面见证，防窗漂移后锁自欺）──
    let l0_hits = &hits[0];
    assert_eq!(l0_hits.len(), 2, "变动块窗 L0 命中数（GOLDEN 2 处）");
    for h in l0_hits {
        let date = ds.dates.get(h.src_start).map(String::as_str).unwrap_or("");
        assert!(
            date.starts_with(C327_BLOCK_HIT_DAY),
            "命中点 src={} 日期 `{date}` 未落在 #323 点名的 {C327_BLOCK_HIT_DAY} 变动块——\
             锁若飘出变动块就不再是本票要的『真覆盖见证』",
            h.src_start
        );
        assert_eq!(
            h.core, C327_BLOCK_HIT_CORE,
            "命中点 src={} 单点核心价位偏离 GOLDEN（覆盖面变化须逐点对账后更新，禁静默）",
            h.src_start
        );
    }
    eprintln!(
        "[#327/block] 变动块见证成立：{C327_BLOCK_HIT_DAY} 两处单点核心（{}）判不成立，\
         L0 重排 {} 个中枢并级联至 L1 重排 {} 个",
        l0_hits[0].core,
        C327_BLOCK_REARRANGE[0],
        C327_BLOCK_REARRANGE[1],
    );
}

/// 从 δ-free dump（[`dump_deltafree_pertrade`] 落盘）逐行重建 `ResidualTrade`（Task #186 离线重算入口）。
/// resid_base/cost/d 由 `f64::from_bits`（十六进制 round-trip）逐字节还原内存值 ⟹ 与在线 records bit-exact。
/// A1/A6：force_state（第 8 维，code 编码）+ d（μ_R 分母）随 dump 还原——force_state 进 δ-free 主裁决基。
/// position 不入任何 δ-free/报告桶分层键 ⟹ 重建恒用 `PositionState::Root`（占位，不影响统计）。
/// bsp_class→BspBits 由 (bsp,δ) 唯一确定；force_state 经 struct-update 覆盖（from_certificate 恒 None）。
fn load_deltafree_dump(path: &str) -> Vec<ResidualTrade> {
    use super::mu_estimator::PositionState;
    use crate::theta_v0::types::BspBits;
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("δ-free dump 读取失败 {path}：{e}"));
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        // 跳表头
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 11, "δ-free dump 行须 11 列（A1/A6 增 force_state+d_bits，裁定甲增 exit_type），得 {}：{line}", f.len());
        let level: u32 = f[0].parse().unwrap();
        let bsp: u8 = f[1].parse().unwrap();
        let delta: i8 = f[2].parse().unwrap();
        let sigma_h: i8 = f[3].parse().unwrap();
        let force_state = force_state_decode(f[4].parse().unwrap());
        let h_bucket: u8 = f[5].parse().unwrap();
        let time_block: u32 = f[6].parse().unwrap();
        let resid_base = f64::from_bits(u64::from_str_radix(f[7], 16).unwrap());
        let cost = f64::from_bits(u64::from_str_radix(f[8], 16).unwrap());
        let d = f64::from_bits(u64::from_str_radix(f[9], 16).unwrap());
        let exit_type = exit_type_decode(f[10].parse().unwrap()); // 诊断切片还原（不进裁决键）
        let bits = match (bsp, delta > 0) {
            (1, true) => BspBits { buy1: true, ..Default::default() },
            (1, false) => BspBits { sell1: true, ..Default::default() },
            (2, true) => BspBits { buy2: true, ..Default::default() },
            (2, false) => BspBits { sell2: true, ..Default::default() },
            (_, true) => BspBits { buy3: true, ..Default::default() },
            (_, false) => BspBits { sell3: true, ..Default::default() },
        };
        let class = MuClass {
            force_state, // A1：δ-free 主裁决基第 8 维还原
            ..MuClass::from_certificate(level, delta, bits, sigma_h, PositionState::Root)
        };
        out.push(ResidualTrade { class, resid_base, cost, h_bucket, time_block, d, exit_type });
    }
    out
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
mod tests {
    use super::*;

    /// δ-free 精确重算自检（Task #186，L1 管线正确性）：round-trip dump→load bit-exact + δ-free
    /// perm_p 池化正确性。合成 records（level0/bsp3/σ+1 两 δ 同号高残差 = beta 漂移签名）：δ-free
    /// 池化后仍显著（同号不抵消，perm_p 小）；对照买卖异号（真方向 alpha）池化抵消 ⟹ perm_p 不显著。
    #[test]
    fn deltafree_dump_roundtrip_and_pooling() {
        use super::super::mu_estimator::PositionState;
        use crate::theta_v0::types::BspBits;
        // ① round-trip：dump 写盘→load 还原逐字节相等（f64 bit 模式精确）。
        let mk = |delta: i8, resid: f64, tb: u32, et: ExitType| {
            let bits = if delta > 0 { BspBits { buy3: true, ..Default::default() } } else { BspBits { sell3: true, ..Default::default() } };
            let class = MuClass::from_certificate(0, delta, bits, 1, PositionState::Root);
            ResidualTrade { class, resid_base: resid, cost: 0.1, h_bucket: 0, time_block: tb, d: 1.0, exit_type: et }
        };
        // exit_type 循环覆盖 5 变体 ⟹ round-trip 实际穿过新诊断列（否则该列是死代码）。
        let ets = [ExitType::CloseRoot, ExitType::ReduceCore, ExitType::CloseReverseOpen, ExitType::RiskExit, ExitType::Hold];
        let recs: Vec<ResidualTrade> = (0..40)
            .map(|i| mk(if i % 2 == 0 { 1 } else { -1 }, 3.14159_f64 * (i as f64 + 1.0), (i % 2) as u32, ets[i % 5]))
            .collect();
        let path = std::env::temp_dir().join("deltafree_roundtrip_test.tsv");
        let p = path.to_str().unwrap();
        // 直接调 dump 逻辑：门控经 env，故临时置位。
        std::env::set_var("DELTAFREE_DUMP", p);
        dump_deltafree_pertrade(&recs);
        std::env::remove_var("DELTAFREE_DUMP");
        let loaded = load_deltafree_dump(p);
        assert_eq!(loaded.len(), recs.len(), "round-trip 笔数");
        for (a, b) in recs.iter().zip(&loaded) {
            assert_eq!(a.resid_base.to_bits(), b.resid_base.to_bits(), "resid_base bit-exact");
            assert_eq!(a.cost.to_bits(), b.cost.to_bits(), "cost bit-exact");
            assert_eq!(a.class.delta, b.class.delta);
            assert_eq!(a.class.bsp_class(), b.class.bsp_class());
            assert_eq!(a.class.parent_dir, b.class.parent_dir);
            assert_eq!((a.h_bucket, a.time_block), (b.h_bucket, b.time_block));
            assert_eq!(a.exit_type, b.exit_type, "exit_type 诊断列 round-trip 无损");
        }

        // ② δ-free 池化：两 δ 同号高残差（beta 签名）。买 r=+10、卖 r=+10 ⟹ Y_buy=+10,Y_sell=−10
        //    池化 mean=0；但 §3.1 的 beta 判据是「δ-free 基 mean 由同号 resid 不抵消」——用 resid_base
        //    同号构造：买卖 resid 都=+10 ⟹ 池化残差基不随 δ 置换改变符号结构。这里验 perm_p 机制运行
        //    （同批置换、只读出侧池化）：H0 独立 δ ⟹ 池化 perm_p 不显著（>0.05）。
        let mut h0: Vec<ResidualTrade> = Vec::new();
        for i in 0..60 {
            h0.push(mk(if i % 2 == 0 { 1 } else { -1 }, (i % 5) as f64 - 2.0, 0, ExitType::Hold));
        }
        let pp = perm_test::stratified_delta_perm_p_deltafree(&h0, perm_test::N_PERM, perm_test::PERM_SEED);
        // A1：δ-free 基键含 force_state 第 8 维——mk 走 from_certificate ⟹ force_state=None。
        assert!(pp.contains_key(&(0, 3, 1, None)), "δ-free 基键 (0,3,+1,None) 应存在");
        assert!(pp[&(0, 3, 1, None)] > 0.05, "H0 独立 δ ⟹ δ-free 池化 perm_p 不显著: {}", pp[&(0, 3, 1, None)]);
    }

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

    /// on2-est2 profile（Task #187）：拆解 R5 est×2 段的耗时归属。q4_fullpi_policy 每窗对同一
    /// train.bars 调 `build_mu_from_bars` **两次**（fullpi/plain）——两次 config 仅 risk/margin 不同，
    /// 而逐 bar 分类（parser+tower）只依赖缠论字段（与 risk/margin 无关）⟹ 两次分类逐 bar 完全相同。
    /// 本 profile 测三段墙钟：①单次全 bar 分类 pass（IncrementalClassifier）②单次 build_mu 全程
    /// ③连跑两次 build_mu（est×2 现状）——定位分类占比与「共享分类可省」的上界。
    /// `cargo test --release --lib theta_v0::backtest::wverify_run::tests::profile_est2_shared_classify -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn profile_est2_shared_classify() {
        use super::super::incremental::IncrementalClassifier;
        use std::time::Instant;
        let cfg = ThetaConfig::default();
        let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载");
        let train = ds.slice_date_window("2022-07-01", "2022-12-31"); // p3fold train（最短段）
        let n = train.bars.len();
        eprintln!("[est2-profile] BTC p3fold train n={n} bar");

        // ① 单次全 bar 分类 pass（黑洞化 tower_generation 防 DCE）。
        let t0 = Instant::now();
        let mut clf = IncrementalClassifier::new(&train.bars, &cfg);
        let mut sink = 0u64;
        for i in 0..n {
            let (_cls, _tower) = clf.classify_at(i);
            sink = sink.wrapping_add(clf.tower_generation()).wrapping_add(clf.forest_epoch());
        }
        let classify_ms = t0.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ① 单次全 bar 分类 pass = {classify_ms:.1} ms (sink={sink})");

        // ② 单次 build_mu 全程（含分类 pass + fill loop）。
        let mut chi = ThetaConfig::default();
        chi.risk.chi_theta = Some(0.0);
        chi.risk.chi_z_alpha = 1.645;
        chi.risk.enforce_gross_cap = true;
        super::super::super::classifier::stage_profile::reset();
        let t1 = Instant::now();
        let (est_a, _) = build_mu_from_bars(&train.bars, &chi, 0);
        let build_one_ms = t1.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ② 单次 build_mu(fullpi) = {build_one_ms:.1} ms (n_classes={})", est_a.n_classes());
        super::super::super::classifier::stage_profile::dump();

        // ③ est×2 现状：连跑两次（fullpi + plain）。
        let t2 = Instant::now();
        let (_e1, _) = build_mu_from_bars(&train.bars, &chi, 0);
        let (_e2, _) = build_mu_from_bars(&train.bars, &cfg, 0);
        let build_two_ms = t2.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ③ est×2 连跑两次 build_mu = {build_two_ms:.1} ms");

        let fill_ms = (build_one_ms - classify_ms).max(0.0);
        eprintln!(
            "[est2-profile] 归属：分类={classify_ms:.1}ms ({:.0}%) fill={fill_ms:.1}ms | est×2 中分类冗余={classify_ms:.1}ms 共享后上界省≈{:.0}%",
            classify_ms / build_one_ms * 100.0,
            classify_ms / build_two_ms * 100.0,
        );

        // ④ 双曲线 scaling exp（n/2 vs n，分别测分类与 fill loop）——定 O(n²) 靶归属。
        let half = &train.bars[..n / 2];
        let t3 = Instant::now();
        let mut clf_h = IncrementalClassifier::new(half, &cfg);
        let mut sink_h = 0u64;
        for i in 0..half.len() {
            let _ = clf_h.classify_at(i);
            sink_h = sink_h.wrapping_add(clf_h.forest_epoch());
        }
        let classify_half_ms = t3.elapsed().as_secs_f64() * 1e3;
        let t4 = Instant::now();
        let (_eh, _) = build_mu_from_bars(half, &chi, 0);
        let build_half_ms = t4.elapsed().as_secs_f64() * 1e3;
        let ratio = (n as f64 / (n / 2) as f64).log2();
        let exp_build = (build_one_ms / build_half_ms).log2() / ratio;
        let exp_classify = (classify_ms / classify_half_ms).log2() / ratio;
        let fill_full = (build_one_ms - classify_ms).max(1e-9);
        let fill_half = (build_half_ms - classify_half_ms).max(1e-9);
        let exp_fill = (fill_full / fill_half).log2() / ratio;
        eprintln!(
            "[est2-profile] ④ 双曲线 (sink_h={sink_h}): build_mu exp={exp_build:.3} | 分类 exp={exp_classify:.3}（{classify_half_ms:.0}→{classify_ms:.0}ms）| fill loop exp={exp_fill:.3}（{fill_half:.0}→{fill_full:.0}ms）",
        );
    }

    /// δ-free 主裁决键守卫（语义级，a3；编译期形状守卫见 `_DELTAFREE_KEY_SHAPE_FROZEN`）：
    /// 断言 [`deltafree_verdict`] 分桶键为 (level, bsp_class, parent_dir, force_state) 四元组
    /// 且**不含 δ 分量**，防止未来把 δ 加回主裁决键。
    /// ① 仅 δ 不同（其余四分量全同）的记录必须池化进同一主裁决桶——若键混入 δ 会裂成 2 桶 ⟹ FAIL；
    /// ② 四键分量各自变异必须各裂新桶（含 force_state Some(态) 变化与 Some→None 的诚实缺维区分）。
    /// 注：[`bucket_verdict`] 是含 δ 的描述性报告桶，非主裁决，不在本守卫范围。
    #[test]
    fn deltafree_verdict_key_is_delta_free_four_tuple() {
        use super::super::mu_estimator::PositionState;
        use crate::theta_v0::types::BspBits;

        // 构造：给定 (level, bsp主类, δ, parent_dir, force_state) 的残差记录。bits 按 δ 选买/卖侧
        // （同 perm_test::tests::rt 模式）——bsp_class() 对买卖同类归并，键不受 δ 侧影响。
        let rt = |level: u32, bsp: u8, delta: i8, parent_dir: i8, fs: Option<ForceStateA5>, resid: f64| {
            let bits = match (bsp, delta > 0) {
                (1, true) => BspBits { buy1: true, ..Default::default() },
                (1, false) => BspBits { sell1: true, ..Default::default() },
                (2, true) => BspBits { buy2: true, ..Default::default() },
                (2, false) => BspBits { sell2: true, ..Default::default() },
                (_, true) => BspBits { buy3: true, ..Default::default() },
                (_, false) => BspBits { sell3: true, ..Default::default() },
            };
            let mut class = MuClass::from_certificate(level, delta, bits, parent_dir, PositionState::Root);
            class.force_state = fs; // from_certificate 诚实 None，测试显式注入第 8 维
            ResidualTrade { class, resid_base: resid, cost: 0.0, h_bucket: 0, time_block: 0, d: 1.0, exit_type: ExitType::Hold }
        };
        // 桶数 = V+F+I 三态计数总和（deltafree_verdict 每桶恰产一个 AlphaState）。
        let n_buckets = |records: &[ResidualTrade]| {
            let (_, _, (nv, nf, ni), _) = deltafree_verdict(records);
            nv + nf + ni
        };

        // ① δ 池化：仅 δ 不同、(level=1,bsp=1,σ_p=+1,force=Dominated) 全同 → 恰 1 桶。
        let base_fs = Some(ForceStateA5::Dominated);
        let pooled: Vec<ResidualTrade> = (0..8)
            .map(|i| rt(1, 1, if i % 2 == 0 { 1 } else { -1 }, 1, base_fs, i as f64 * 0.7 - 2.0))
            .collect();
        assert_eq!(
            n_buckets(&pooled), 1,
            "仅 δ 不同的记录必须池化进同一 δ-free 主裁决桶（=1）；>1 ⟹ 主裁决键混入了 δ 分量"
        );

        // ② 四键分量各自分桶：基桶 + 5 种单分量变异（每变异桶买卖两 δ 侧各 2 笔，保 A1 ⊥δ 交换自由度）。
        let mut recs = pooled.clone();
        for i in 0..4 {
            let d = if i % 2 == 0 { 1 } else { -1 };
            let r = i as f64 * 0.7 - 2.0;
            recs.push(rt(2, 1, d, 1, base_fs, r)); // level 变异
            recs.push(rt(1, 2, d, 1, base_fs, r)); // bsp_class 变异
            recs.push(rt(1, 1, d, -1, base_fs, r)); // parent_dir 变异
            recs.push(rt(1, 1, d, 1, Some(ForceStateA5::Dominates), r)); // force_state 态变异
            recs.push(rt(1, 1, d, 1, None, r)); // force_state Some→None（诚实缺维须独立成桶）
        }
        assert_eq!(
            n_buckets(&recs), 6,
            "四键分量 level/bsp_class/parent_dir/force_state（含 None）各自变异须各裂新桶：1 基桶 + 5 变异桶"
        );
    }
}
