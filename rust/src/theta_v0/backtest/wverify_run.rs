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
        ExitType::CloseShortDiff => 2,
        ExitType::RiskExit => 3,
        ExitType::Hold => 4,
    }
}

/// ExitType 编码逆映射（[`exit_type_code`]），离线 dump 复现器还原诊断列。
fn exit_type_decode(code: u8) -> ExitType {
    match code {
        0 => ExitType::CloseRoot,
        1 => ExitType::ReduceCore,
        2 => ExitType::CloseShortDiff,
        3 => ExitType::RiskExit,
        _ => ExitType::Hold,
    }
}

/// ExitType 报告标签（W-VERIFY 5 变体占比拆解节可读列）。
fn exit_type_label(et: ExitType) -> &'static str {
    match et {
        ExitType::CloseRoot => "CloseRoot(P5)",
        ExitType::ReduceCore => "ReduceCore(P6)",
        ExitType::CloseShortDiff => "CloseShortDiff(P7)",
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

/// ★#388 T2（标定臂）：`M8_FEE_DATUM=<datum 文件名>:<symbol>:<tier>` spec → venue 费率档。
///
/// **仓内 datum 文件优先，路径即契约**（#385 Implementation Decisions）：文件名相对
/// [`datum_dir`](crate::theta_v0::venue_fee::datum_dir)（= `analysis/data_cache`），装载经
/// `load_datum` 的 sha256 sidecar 校验（未校验的费率表进跑批 = 口径标签谎报 L2）。
///
/// **全程 fail-loud**（格式非法 / 文件缺失 / 哈希不符 / (symbol,tier) 未在册）——静默退回未标定档
/// 会让产物打着 `[L1机制/费率未标定]` 标签冒充标定臂读数。**跨品种借档**由
/// `data::load_by_symbol` 的 #360 校验在数据加载期承保（本函数不重复实现该判定）。
fn parse_fee_datum_spec(spec: &str) -> crate::theta_v0::venue_fee::VenueFeeSchedule {
    use crate::theta_v0::venue_fee::{datum_dir, load_datum};
    let parts: Vec<&str> = spec.split(':').collect();
    assert_eq!(
        parts.len(),
        3,
        "M8_FEE_DATUM 格式须为 `<datum 文件名>:<symbol>:<tier>`（三段冒号分隔），实得 {spec:?}"
    );
    let path = datum_dir().join(parts[0]);
    let book = load_datum(&path)
        .unwrap_or_else(|e| panic!("M8_FEE_DATUM datum 装载失败（{}）：{e}", path.display()));
    book.resolve(parts[1], parts[2])
        .unwrap_or_else(|e| panic!("M8_FEE_DATUM 档位解析失败（spec={spec:?}）：{e}"))
}

/// ★#388 T2：`M8_FEE_DATUM` 未设 ⟹ no-op（臂R 逐位不变，bit-exact 中性——与
/// [`apply_theta_dir_preset_from_env`] / `M8_WIN_FILTER` 同款 env-gate 先例）；
/// 设置 ⟹ 注入 [`parse_fee_datum_spec`] 解析出的档，跑批升为**标定臂（臂D）**，
/// 产物标签自动升 `[L2费率标定: datum <前12位>]`（`risk::rate_calibration_label` 契约）。
fn apply_m8_fee_datum_from_env(cfg: &mut ThetaConfig) {
    if let Ok(spec) = std::env::var("M8_FEE_DATUM") {
        cfg.exec.fee_schedule = Some(parse_fee_datum_spec(&spec));
    }
}

/// ★#389 T3（帽臂 C）：级别帽臂的 `level_weights` = **#310 M4 验收既有配置**（6 级各 0.05，
/// Σw_ℓ=0.3≤1）。
///
/// **来源（不自造参数）**：#310（LEE M4 级别 sizing/risk）落地时的两处在册验收配置
/// —— `runner.rs::lee_m4_level_cap_narrows_position_when_enabled`（#310 本体验收：刻意不压到 0，
/// 使帽「真收窄」而非「交易停摆」）与 `runner.rs::lee_m4_cap_on_sparsity_has_no_unexplained_violation`
/// （#351 MED-3 补课，cap-on 稀疏性覆盖）逐字同值。#310 票体本身只钉 `Σw_ℓ≤1` 的代数约束，
/// **不钉具体数值**；仓内唯一「既有配置」即此二测的 `vec![0.05; 6]`，本常量是它的单一来源化。
///
/// **参数归属声明（#310 票体逐字）**：`w_ℓ` 全属 `Θ_risk`，**禁冒充缠论可导**（090/v3）。
const M8_LEVEL_CAP_WEIGHTS_310: [f64; 6] = [0.05; 6];

/// ★#389 T3：`M8_LEVEL_CAP=true` ⟹ 帽臂（臂C）= 臂D 配置 + `enforce_level_cap=true`
/// + [`M8_LEVEL_CAP_WEIGHTS_310`]；未设 ⟹ **no-op**（臂R/臂D 逐位不变，bit-exact 中性，
/// 与 [`apply_enforce_gross_cap_from_env`] / [`apply_m8_fee_datum_from_env`] 同款 env-gate 先例）。
///
/// **非法值 fail-loud**（不静默忽略）：`ENFORCE_GROSS_CAP` 的 `!= "true" ⟹ 静默 false` 先例在本处
/// **不适用**——帽臂产物若因拼写错误静默退回臂D，报告会把臂D 读数当帽臂登记（口径谎报）。
/// 帽的施加点在 `fill.rs::pi_theta_position`（`risk.enforce_level_cap` 双施加点 + `cap_narrowed`
/// 归因，#351 MED-3 / #363 逐级判据）。
///
/// `spec` 为 `None` 表示 env 未设（本函数与 env 解耦，便于测试无副作用地覆盖三分支）。
///
/// **`Σw_ℓ≤1` 不在此处运行时断言**（评审 Standards 轴指出）：注入值是编译期常量
/// [`M8_LEVEL_CAP_WEIGHTS_310`]，对它做运行时断言恒真 ⟹ 零信息增量的 L0 同义反复
/// （`formalization-validity-domain` 231号），把它读成「机器承保」会高估强度。
/// 该约束的**真实承保点** = 测试 `m8_level_cap_true_applies_310_weights`（常量一旦被改，测试红）。
fn apply_m8_level_cap(cfg: &mut ThetaConfig, spec: Option<&str>) {
    match spec {
        None => {} // 未设 = 帽关（臂R/臂D 路径逐位不变）
        Some("true") => {
            cfg.risk.level_weights = M8_LEVEL_CAP_WEIGHTS_310.to_vec();
            cfg.risk.enforce_level_cap = true;
        }
        Some(other) => panic!(
            "M8_LEVEL_CAP 仅接受 \"true\"（实得 {other:?}）——静默忽略会让帽臂产物冒充臂D 读数"
        ),
    }
}

/// [`apply_m8_level_cap`] 的 env 入口（`M8_LEVEL_CAP`）。
fn apply_m8_level_cap_from_env(cfg: &mut ThetaConfig) {
    apply_m8_level_cap(cfg, std::env::var("M8_LEVEL_CAP").ok().as_deref());
}

/// ★#389 T3：LEE 稀疏性六字段的**单一来源**（m8 跑批的 stdout 诊断行与产物表共用一份取值，
/// 两处手抄同一组字段会静默漂移）。顺序 = 产物表列序：
/// `n_decisions` / `n_cap_narrowed` / `off_clock 订单` / `其中风控·帽可解释` /
/// `off_clock 级别Δq` / `未解释`。
fn lee_row_cells(s: &super::super::strategy::level_order::LevelOrderStats) -> [u64; 6] {
    [
        s.n_decisions,
        s.n_cap_narrowed,
        s.n_orders_off_structural_clock,
        s.n_orders_off_structural_clock_risk_explained,
        s.n_levels_off_clock_delta,
        s.n_levels_off_clock_delta_unexplained,
    ]
}

/// ★#374 有效域收窄 / #388 T2 / #423 的**报告层标注**：单标量成本口径**无良定义**的档下，
/// 随机对照系读数一律标此串。
///
/// **适用范围 = 标量无良定义档，不是「标定档」全体**（★#423 第二阶段订正）。判定本体 =
/// [`treasury::scalar_cost_rate_opt`](super::treasury::scalar_cost_rate_opt)（→
/// `venue_fee::VenueFeeSchedule::constant_effective_rate` 的结构检查），两档落本标注：
/// - **按股档（per-share）**：逐笔费率随 (qty, px, side) 非线性变（最低佣金托底 / 1% 名义额上限 /
///   仅卖出监管费）⟹ 不存在常数等效费率 ⟹ 依赖它的 LCB_OOS(R)（block bootstrap 的成本口径）
///   与三态判据**确无**合法取值；
/// - **按金额档但 maker≠taker**：角色维不消失 ⟹ 常数不由 datum 内容唯一确定，选一侧充数就是把
///   不对称藏进一个常数。
///
/// **不落本标注的档**：未标定档（`fee_schedule = None`，常率）与**按金额对称档**
/// （per-notional 且 maker/taker 逐位对称，如 Binance 现货 VIP0 = 10bp/side）——后者自 ★#423
/// 第一阶段起标量**可得**，其层4 读数照常产出（此前实现按 `fee_schedule.is_some()` 一刀切判
/// `None`、连带撤下该档读数，那道保守收窄已拆除）。
///
/// 喂近似值（实际有效费率等）在无良定义档已由 #374 明文否决——那只是把不对称藏进一个更贵的
/// 常数（090 声明膨胀）。
const SCALAR_UNDEFINED_UNAVAILABLE: &str = "不可用(单标量成本口径无良定义 #374/#423)";

/// m8 四层报告的**层4 两格**（LCB_OOS(R) / 三态）渲染。
///
/// - `lcb = Some(x)`（标量可得档：未标定档，或按金额对称标定档）⟹ 与降级改造前**逐字符相同**的
///   原口径（M8:168 三分支）；
/// - `lcb = None`（标量无良定义档，`RunResult::fee_rate = None`）⟹ 两格均
///   [`SCALAR_UNDEFINED_UNAVAILABLE`]。三态**不用 R 的正负顶替**：判据是 `LCB>0`，缺 LCB 即无
///   判据（诚实缺席，不降格冒充）。
///
/// 本函数的入参与层4 声明段（[`layer4_scalar_caliber_notice`]）/ 结算行
/// （[`layer4_verdict_line`]）**同一个真值**——三者都由 `scalar_cost_rate_opt` 的可得性驱动，
/// 不存在第二个判据（★#423 第二阶段：此前声明段按 `fee_schedule.is_some()` 判、数值按
/// `fee_rate` 判，按金额对称档下二者分歧 ⟹ 同一报告内声明「不可用」而表格给数，090 声明膨胀）。
fn layer4_cells(lcb: Option<f64>, r_total: f64) -> (String, String) {
    match lcb {
        Some(lcb_r) => {
            let verdict = if lcb_r > 0.0 {
                "CONFIRMED"
            } else if r_total > 0.0 {
                "INCONCLUSIVE"
            } else {
                "无(R≤0)"
            };
            (format!("{lcb_r:+.0}"), verdict.to_string())
        }
        None => (
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
        ),
    }
}

/// m8 四层报告的**层4 随机对照三格**（★#423 收尾轮 F）——`Θ>随机` / `p_shift` / `p_indep`。
///
/// ## 为什么加这三格（票体交付缺口）
///
/// `metrics::significance` 在标量可得档被**完整调用**，`theta_beats_random` / `shift_pvalue` /
/// `indep_pvalue` 三值都已算出，但此前 m8 只取 `boot_ci95_lo` 一个字段 ⟹ 票体 What-to-build
/// 明写要出的「随机对照」读数**无落盘出口、采不到**。本函数是那个出口，**不新增任何计算**
/// （同一次 `significance` 调用的既有字段）⟹ 不改 RNG 消耗、不改 dump 字节。
///
/// ## 渲染口径
///
/// - `None`（标量无良定义档：按股 / 按金额非对称）⟹ 三格均 [`SCALAR_UNDEFINED_UNAVAILABLE`]。
///   该档本就不调 `significance`（无输入费率）⟹ 三值**未定义**，照实标不可用，
///   **不填 0、不留空**（090：空格会被读成"算了但为 0"）。与层4 两格同一个真值、同一个标注串。
/// - `Some(sig)` ⟹ `theta_beats_random` 渲染「是/否」；两个 p 值 `{:.4}`。
///
/// **退化标注**：`sig.controls_degenerate` 为真时（schedule-shift 无非零合法平移，或
/// independent 全笔 `hold ≥ len` ⟹ 无真随机样本）p_upper 无统计含义（`metrics` 该字段文档）。
/// 此时三格各附 `(对照退化)` / `(退化)`——p=1.0 是真算出来的值，但不得被读作"未能否证"。
/// 该标志是**行级**的（`metrics::Significance` 只给一个合并标志，不分 shift/indep）⟹ 两个 p
/// 格同标；不在本处推断"是哪一个退化"（那要改 `metrics` 的返回形态，另票）。
fn layer4_random_control_cells(sig: Option<&super::metrics::Significance>) -> (String, String, String) {
    match sig {
        None => (
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
        ),
        Some(s) => {
            let deg = if s.controls_degenerate { "(退化)" } else { "" };
            (
                format!(
                    "{}{}",
                    if s.theta_beats_random { "是" } else { "否" },
                    if s.controls_degenerate { "(对照退化)" } else { "" }
                ),
                format!("{:.4}{deg}", s.shift_pvalue),
                format!("{:.4}{deg}", s.indep_pvalue),
            )
        }
    }
}

/// m8 报告头的**层4 标量成本口径声明段**（★#423 第二阶段）——`None` = 不出声明段。
///
/// 三分叉，判据**只有一个**：[`treasury::scalar_cost_rate_opt`](super::treasury::scalar_cost_rate_opt)
/// 在本 `exec` 上的可得性（与层4 两格 [`layer4_cells`] 的数值来源同一函数）。
///
/// | 档 | 产出 |
/// |---|---|
/// | 未标定（`fee_schedule = None`） | `None`——不出声明段（臂R 产物逐字节不变） |
/// | 标定 + 标量可得（按金额对称档） | 「可得」正文：层4 读数**照常产出**，并登记本声明不覆盖什么 |
/// | 标定 + 标量无良定义（按股档 / 按金额非对称档） | 「不可用」正文，读数标 [`SCALAR_UNDEFINED_UNAVAILABLE`] |
fn layer4_scalar_caliber_notice(exec: &ExecConfig) -> Option<String> {
    exec.fee_schedule.as_ref()?; // 未标定档（臂R）不出声明段——产物逐字节不变。
    Some(match super::treasury::scalar_cost_rate_opt(exec) {
        Some(rate) => format!(
            "> **标定档单标量成本口径可得声明（★#423）**：本跑批经 `M8_FEE_DATUM` 注入 venue 费率 \
             datum，且该档为**按金额档（per-notional）且 maker/taker 逐位对称**、撮合角色取自编译期\
             常量 `venue_fee::PRODUCTION_LIQUIDITY_ROLE` ⟹ 存在与 (qty, px, side) 无关的常数等效\
             费率（判定本体 = `venue_fee::VenueFeeSchedule::constant_effective_rate`）。单标量成本\
             费率（`RunResult::fee_rate`，经 `treasury::scalar_cost_rate_opt`）= 档 bps/1e4 + \
             `slippage_bps`/1e4 + `tax_bps`/1e4 = **{:.6}**（标定档下 `tax_bps` 由 \
             `treasury::fee_quoter` 的 assert 强制为 0）。\n\
             > 故层4 的 `LCB_OOS(R)` 与三态判据在本臂**照常产出**——与未标定臂同一函数、同一口径，\
             反事实臂（随机对照在不同 px 上重执行）用同一个常数，不引入口径不对称。\n\
             >\n\
             > **★#423 收尾轮 F 新增输出**：层4 表新增三列 `Θ>随机` / `p_shift(平移)` / \
             `p_indep(独立)`，取自**同一次** `metrics::significance` 调用的既有字段\
             （`theta_beats_random` / `shift_pvalue` / `indep_pvalue`）——此前只取 `boot_ci95_lo`，\
             这三值算了却无落盘出口。**旧产物没有这三列**（本轮新增列，**不是**读数漂移）；\
             三值不新增任何计算 ⟹ `trades.jsonl` / `tower_events.jsonl` 逐字节不变（回归门 + `cmp` 实证）。\
             p 值口径 = `p_upper=(1+count(rand≥theta))/(N+1)`，N=1000，seed 冻结；\
             `Θ>随机 ⟺ 两对照 p_upper 均 ≤0.05 且对照未退化`；对照退化时格内附 `(退化)`\
             （p=1.0 是真算出的值，但**无统计含义**，不得读作「未能否证」）。\n\
             >\n\
             > **本声明不覆盖**（照实登记，不膨胀）：\n\
             > - `slippage_bps` 仍**未标定**（venue datum 只标佣金/监管/清算科目，报告 §3.3）⟹ 本臂\
             标量是「L2 标定佣金 + L1 未标定滑点」的合成；成交费率科目的口径标签见上方抬头；\n\
             > - `l3_delta_r_alpha` 鞅守卫的成本剥离**不在本跑批路径内**（其唯一调用方是同文件的 \
             `#[ignore]` 合成鞅守卫测试）；\n\
             > - 层1 signal 的 INCONCLUSIVE 与本档无关（转引，不重算）。\n\n",
            rate
        ),
        None => format!(
            "> **单标量成本口径无良定义声明（#374 / #385 / ★#423）**：本跑批经 `M8_FEE_DATUM` 注入 \
             venue 费率 datum，且该档**不存在**与 (qty, px, side) 无关的常数等效费率 ⟹ 单标量成本\
             费率（`RunResult::fee_rate`）**无良定义**（`treasury::scalar_cost_rate_opt` 判 `None`，\
             判定本体 = `venue_fee::VenueFeeSchedule::constant_effective_rate`）。两种档落此支：\n\
             > - **按股档（per-share）**：逐笔费率随 (qty, px, side) 非线性变（最低佣金托底 / 1% 名义\
             额上限 / 仅卖出监管费）；\n\
             > - **按金额档但 maker≠taker**：角色维不消失，常数不由 datum 内容唯一确定。\n\
             >\n\
             > 故下列读数标 `{SCALAR_UNDEFINED_UNAVAILABLE}`，**不以近似费率顶替**——喂实际有效费率\
             （Σfee/Σnotional 等）只是把口径不对称藏进一个更贵的常数（#374 明文否决，090 声明膨胀）：\n\
             > - `LCB_OOS(R)`（block bootstrap 的成本口径依赖单标量费率）；\n\
             > - **三态判据**（判据是 `LCB>0`，缺 LCB 即无判据——不用 R 的正负降格顶替）；\n\
             > - `metrics::significance` 派生的随机对照系——层4 表的 `Θ>随机` / `p_shift(平移)` / \
             `p_indep(独立)` 三列（★#423 收尾轮 F 新增列；本档下 `significance` **一律不调**，\
             无输入费率 ⟹ 三值未定义，故三格同标不可用，**不填 0 也不留空**）；\n\
             > - `l3_delta_r_alpha` 鞅守卫的成本剥离（不在本跑批路径内；本档要接须改逐笔实付累计）。\n\
             >\n\
             > **仍然有效**（与单标量费率无关，逐笔实付经 `treasury::fee_quoter` 解析）：n_orders / \
             ΣN_tΔP_t / Comm+Slip / Funding / Borrow / LiqLoss / net_r(execR) / MaxDD / 声部数 / \
             终Stage / Q_T / W_T / η 列 / R(含浮盈) / `NEST_GATE_STATS`。\n\
             >\n\
             > **对上文抬头的更正**：抬头「signal 层无 alpha ⟹ 端到端负/INCONCLUSIVE 照实」一句\
             描述的是**标量可得档**的三态判读。本臂层4 **无结论**（判据缺输入），该句对本臂不适用——\
             不得把「不可用」读作「负」或「INCONCLUSIVE」。\n\n"
        ),
    })
}

/// m8「判据结算」段的**层4 结算行**（★#423 第二阶段）——与 [`layer4_cells`] /
/// [`layer4_scalar_caliber_notice`] 同一个真值（`scalar_cost_rate_opt` 的可得性）。
///
/// - `Some`（标量可得：未标定档，或按金额对称标定档）⟹ 常规判据行（见 LCB 列，未过 ⟹
///   INCONCLUSIVE）；
/// - `None`（标量无良定义档）⟹ **无结论**行。措辞§5.6 的 INCONCLUSIVE 是「有 LCB 且 ≤0」的态，
///   拿它套「LCB 不存在」是静默越域。
fn layer4_verdict_line(scalar_rate: Option<f64>) -> &'static str {
    match scalar_rate {
        None => {
            "- **层4 完整策略** `LCB_OOS(R)>0`：**本臂无结论**——本档单标量成本口径无良定义 ⟹ \
             LCB_OOS(R) 不可用（#374 / ★#423，见上方声明），判据无输入。**既不宣称 confirmed \
             alpha，也不判 INCONCLUSIVE**（后者是「有 LCB 且 ≤0」的态，套用于「LCB 不存在」是越域）。\
             该层要在本档下有结论，须先给随机对照接 (qty, px, side) 逐笔费率缝（另票）。\n"
        }
        Some(_) => {
            "- **层4 完整策略** `LCB_OOS(R)>0`：见 LCB_OOS(R) 列——**未过 ⟹ INCONCLUSIVE**，\
             不宣称 confirmed alpha（措辞§5.6：INCONCLUSIVE≠无 alpha；§5.3：不外推 max-full）。\n"
        }
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
    // 5 变体固定序（CloseRoot/ReduceCore/CloseShortDiff/RiskExit/Hold）——即使某变体 0 笔也列出（穷尽）。
    let variants = [
        ExitType::CloseRoot,
        ExitType::ReduceCore,
        ExitType::CloseShortDiff,
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

/// M6 成本模型口径（参数化常费率，有效域 L1）：**Binance 现货（spot）近似**——#303 编排者裁定
/// 2026-07-26 切 spot。venue 依据：本函数服务的 BTC 数据 = `btc_1m_full.json` ←
/// `data.binance.vision/data/**spot**/monthly/klines/BTCUSDT/1m`（`scripts/download_btc_binance.py:38-40`）。
/// 三通道的现货语义、基数重叠与「三项之和读作持有成本上界」的结论**不在此复制**——唯一权威处是
/// [`CostModel`](super::super::strategy::risk::CostModel) 节头（#340 单源收敛）。下列是本函数四个
/// 常参数的取值与按 bar 频率的年化换算（数值 = 切 spot 前原值，未动）：
///
/// - 周期通道（[`CostModel::funding_accrual`]）每 480 根 1 分钟 bar（= 8h 记账周期）按 |净名义|
///   收 1bp ⟹ ≈ 0.03%/日 ≈ **10.95%/年**；
/// - `borrow` 每 bar 0.01bp（基数 = 借入名义 max(0,|N|−E)）⟹ 1m bar 下 ≈ 0.144%/日 ≈ **52.6%/年**；
/// - `liq` 0.5%（作用于强平时刻 |净名义|）。
///
/// 两个年化数是本注释按 bar 频率做的换算（≈，非 venue 原文）。**照实（090）：现货借币利率的
/// 一手数字本仓未取到**（`venue-fee-source-research-20260726.md` §2.1 记 Binance 现货借贷利率
/// 需认证端点、§4 列为未核项）——故这两个保底值**不声称对齐任何真实档位**；borrow 的 52.6%/年
/// 明显高于常见现货借币档，方向是**高估成本**（保守，不美化回测）。
///
/// **有效域声明（231号 / A10 waiver）**：真实现货借贷利率曲线 / 机会成本基准是**外部数据源缺口**
/// （L2），waiver 豁免的是外部数据，机制在此真实装、费率待外部标定（口径标签
/// [`RATE_UNCALIBRATED_LABEL`](super::super::strategy::risk::RATE_UNCALIBRATED_LABEL) 强制）。
/// 真永续（真 funding datum + perp 费率）接入是**另票**（#62 数据源 + datum 版本管理），本票不做。
/// ★pub(crate)：阶段 3a 前置实装（M7_WITNESS_A10 env gate，p126 runbook §2.1）同 q4_margin_model。
pub(crate) fn m6_cost_model() -> super::super::strategy::risk::CostModel {
    use super::super::strategy::risk::CostModel;
    // 机会成本 8h=480bar、1bp/周期；现货杠杆借币每 bar 0.01bp；强平罚金 0.5%。
    CostModel::new(0.0001, 480, 0.000001, 0.005).expect("M6 常费率参数合法（冻结近似值）")
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

/// ★M8 端到端全策略 OOS 跑批（TARGET_STRATEGY_MAXFULL.md M8:161-168 / 路线.pdf p17,p20-21）：
/// 三系统**同时开启**（M5 净额执行 + overlay 旁路/账本 + M6 cost_model 成本 + M7 三阶段 TW
/// 账本）跑同一 BTC
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
fn resolve_m8_symbol(spec: Option<&str>) -> String {
    let symbol = spec.unwrap_or("BTC").trim().to_ascii_uppercase();
    assert!(!symbol.is_empty(), "M8_SYMBOL 不得为空");
    let _ = m8_epistemology(&symbol);
    assert!(
        data::SYMBOLS.iter().any(|(s, _, _)| *s == symbol)
            && PREREG_WINDOWS.iter().any(|w| w.symbol == symbol),
        "M8_SYMBOL={symbol:?} 必须同时在 data::SYMBOLS 与 PREREG_WINDOWS 登记（禁静默回退 BTC）"
    );
    symbol
}

fn m8_epistemology(symbol: &str) -> (&'static str, &'static str) {
    match symbol {
        "BTC" => (
            "**认识论 L2**：真实 BTC OOS 假设检验；signal 层无 alpha ⟹ 端到端负/INCONCLUSIVE照实（否定性结果合法）。",
            "### 层2/3/4（本跑批 L2 实测，三系统同开）",
        ),
        "OKLO" => (
            "**认识论 L1**：真实 OKLO 观察池窗口的 treasury 路径、费用算术与触达审计；严格遵守 v3，读数不作 alpha 论据、不作策略择优输入。",
            "### 层2/3/4（本跑批 L1 实测，三系统同开）",
        ),
        _ => panic!("M8_SYMBOL={symbol:?}：m8 报告认识论仅支持 BTC/OKLO"),
    }
}

fn assert_m8_audit_coverage(symbol: &str, filter: Option<&str>, audited_windows: usize) {
    assert!(
        audited_windows > 0,
        "M8_SYMBOL={symbol:?} M8_WIN_FILTER={filter:?}：没有完成任何可审计窗口，禁止生成逐窗断言报告"
    );
}

/// m8 的已登记跑批窗。OKLO §2.4 明确无 walk-forward，故只消费它的单段 OOS；BTC 保留原
/// m8 的 p3 单折 + OOS 内前两个 anchored 窗。
fn m8_windows(symbol: &str) -> Vec<(String, String, String)> {
    let sw = PREREG_WINDOWS
        .iter()
        .find(|w| w.symbol == symbol)
        .unwrap_or_else(|| panic!("M8_SYMBOL={symbol:?} 不在 PREREG_WINDOWS"));
    if sw.wf_anchored.is_empty() {
        return vec![("oklo_oos".into(), sw.oos.0.into(), sw.oos.1.into())];
    }
    let mut wins = vec![("p3fold".into(), "2023-01-01".into(), "2023-06-30".into())];
    for w in sw
        .wf_anchored
        .iter()
        .filter(|w| w.test_start >= OOS_START)
        .take(2)
    {
        wins.push((format!("wf{}", w.i), w.test_start.into(), w.test_end.into()));
    }
    wins
}

fn resolve_m8_report_path(spec: Option<&str>) -> String {
    let path = spec.unwrap_or("/tmp/m8_e2e_all_systems_oos.md").trim();
    assert!(!path.is_empty(), "M8_REPORT_PATH 不得为空");
    path.to_string()
}

/// ★#490 MED-3：报告 header 的执行域必须与 `OverlayRunResult.voice_exec` 同口径。
fn m8_execution_projection_label(voice_exec_is_some: bool) -> &'static str {
    if voice_exec_is_some {
        "声部执行投影 + 净额影子账本"
    } else {
        "净额执行 + overlay 旁路/账本"
    }
}

/// ★#490 MED-2：逐窗 treasury 行的单一渲染入口；`unclassified_venue` 是未标定/
/// per-notional 档的真实 venue 科目，必须与已分类科目并列可见。
fn format_m8_fee_audit_row(
    tag: &str,
    projected_trades: usize,
    fee: super::treasury::FeeAudit,
) -> String {
    let effective_venue_rate = if fee.notional > 0.0 {
        fee.venue_total() / fee.notional
    } else {
        0.0
    };
    format!(
        "| {tag} | {projected_trades} | {} | {} | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} | \
         {:.6} | {:.6} | {:.6} | {}/{} | {}/{} | {}/{} | {:.6e} |\n",
        fee.n_fills,
        fee.notional,
        fee.commission,
        fee.passthru,
        fee.clearing_cat,
        fee.sec,
        fee.taf,
        fee.unclassified_venue,
        fee.slippage,
        fee.total_fee,
        fee.min_commission_hits,
        fee.n_fills,
        fee.notional_cap_hits,
        fee.n_fills,
        fee.taf_cap_hits,
        fee.n_fills,
        effective_venue_rate,
    )
}

fn m8_layer23_settlement(
    symbol: &str,
    rows: &[(String, f64, super::super::strategy::ledger::TStage)],
) -> (String, String) {
    let execution = rows
        .iter()
        .map(|(tag, net_r, _)| format!("`{tag}` net_r={net_r:+.0}"))
        .collect::<Vec<_>>()
        .join("；");
    let treasury = rows
        .iter()
        .map(|(tag, _, stage)| {
            let stage = match stage {
                super::super::strategy::ledger::TStage::CostReduction => "I(降成本)",
                super::super::strategy::ledger::TStage::CapitalRecovered => "II(已回本)",
                super::super::strategy::ledger::TStage::EarningShares => "III(赚份额)",
            };
            format!("`{tag}`={stage}")
        })
        .collect::<Vec<_>>()
        .join("；");
    (
        format!(
            "- **层2 execution** `E[R(Π_exec)]>0`：{execution}。\
             {symbol} 本窗读数仅作 execution/treasury 路径与费用算术审计，禁作 alpha 论据或策略择优输入。"
        ),
        format!(
            "- **层3 treasury** `Reach(StageIII)>0`：{treasury}；\
             终态按本次真实窗 TW 账本逐窗登记，不由 signal 层结论静态推断。"
        ),
    )
}

#[test]
#[ignore]
fn m8_e2e_all_systems_oos() {
    use super::metrics::significance;
    use super::runner::run_theta_v0_pi_overlay;
    use super::super::strategy::coverage::Vertical;
    use super::super::strategy::ledger::{RiskPolicy, TStage};

    // ★#388 T2（标定臂 D）：`M8_FEE_DATUM=<file>:<symbol>:<tier>` ⟹ venue 费率标定；未设 = 臂R
    //   逐位不变。**此处也注入**（不只循环内的 cfg）的两个理由：① 报告口径标签取自本 cfg，
    //   不同步就会给标定臂产物打 `[L1机制/费率未标定]`（标签谎报）；② `load_by_symbol` 的 #360
    //   品种绑定校验读本 cfg ⟹ 跨品种借档（如 OKLO 档配 BTC 数据集）在数据加载期即 fail-loud。
    let plain_cfg = {
        let mut c = ThetaConfig::default();
        apply_m8_fee_datum_from_env(&mut c);
        // ★#389 T3（帽臂 C）：同样在此注入——报告头的帽臂声明块取自本 cfg，不同步就会让
        //   帽臂产物看上去与臂D 无异（口径谎报）。数据加载不受帽影响（帽在 fill 层）。
        apply_m8_level_cap_from_env(&mut c);
        c
    };
    let symbol = resolve_m8_symbol(std::env::var("M8_SYMBOL").ok().as_deref());
    let ds = data::load_by_symbol(&symbol, &plain_cfg)
        .unwrap_or_else(|e| panic!("{symbol} 数据加载失败：{e}"));
    let nav_of = |d: &data::Dataset| {
        d.bars.iter().find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * plain_cfg.tick.tick_size).unwrap_or(1.0) * 1000.0
    };

    // OOS 窗清单：p3 单折 + 前两个 anchored walk-forward（与 M6 跑批同窗，可差分对照）。
    let mut wins = m8_windows(&symbol);
    // ★T3 (#172)/#164 复现副本同款先例：`M8_WIN_FILTER=<tag>` ⟹ 只跑指定窗（逐窗重放/shadow
    // dump 分窗落盘需要；未设 = 全窗清单不变，bit-exact 中性——只跳过其他窗，窗内行为逐字节同）。
    let win_filter = std::env::var("M8_WIN_FILTER").ok();
    if let Some(filter) = &win_filter {
        wins.retain(|(tag, _, _)| tag == filter);
    }

    let (epistemology, layer234_heading) = m8_epistemology(&symbol);
    let voice_exec_expected = super::admission::voice_exec_gate();
    let execution_projection = m8_execution_projection_label(voice_exec_expected);
    let mut report = format!(
        "# M8 端到端全策略 OOS（TARGET_STRATEGY_MAXFULL.md M8 / 路线.pdf p17,p20-21）\n\n\
         三系统同开：M5 {execution_projection} + M6 cost_model（参数化持有成本三项，**spot 口径**：Funding 列＝\
         资金占用机会成本、Borrow＝现货杠杆借币、Liq＝强平罚金；#303）+ M7 三阶段 TW 账本。\n\
         口径：margin=CME-simple 单段；cost=参数化常费率；κ=0 冻结（M7 c3 裁定，正 κ 推迟 M8 后 L3）。\n\
         品种：**{symbol}**；窗口由 `PREREG_WINDOWS` 登记消费。{epistemology}\n\n",
    );
    // A10 附则B 裁决2（090 措辞纪律）：带成本 R 数值报告强制口径标签（费率未标定，禁作 alpha 论据）。
    // ★#360：成交费率标签随 `ExecConfig::fee_schedule` 升降级；持有成本三项独立保持 L1。
    report.push_str(&format!(
        "**口径标签：{}**（成交费率科目）／**{}**（cost 三常费率保底未标定）（A10 附则B 强制；数值禁作 alpha 论据/策略择优输入）\n\n",
        super::super::strategy::risk::rate_calibration_label(&plain_cfg.exec),
        super::super::strategy::risk::RATE_UNCALIBRATED_LABEL,
    ));
    // ★#423 第二阶段：层4 标量成本口径声明段与层4 数值**同一个真值**（`scalar_cost_rate_opt`）。
    //   此前声明段按 `fee_schedule.is_some()` 判、数值按 `RunResult::fee_rate` 判——按金额对称档
    //   （第一阶段解锁）下二者分歧 ⟹ 同一份报告声明「不可用」而表格给数（090 声明膨胀）。
    let scalar_rate = super::treasury::scalar_cost_rate_opt(&plain_cfg.exec);
    if let Some(notice) = layer4_scalar_caliber_notice(&plain_cfg.exec) {
        report.push_str(&notice);
    }
    // ★#389 T3（帽臂 C）：帽臂声明块随产物走——「同 config 仅帽开关差」是 D-vs-C 归因的前提，
    //   产物必须自证它是哪个臂（否则报告引用时无法机械核对）。
    if plain_cfg.risk.enforce_level_cap {
        report.push_str(&format!(
            "> **帽臂（臂C）声明（#389 / #385）**：本跑批经 `M8_LEVEL_CAP=true` 开启 M4 级别级风险帽\
             （`risk.enforce_level_cap=true`，`level_weights={:?}` = #310 既有配置，Σw_ℓ={:.2}≤1）。\
             与臂D **唯一配置差异即此开关**（费率 datum / margin / cost_model / κ=0 / 窗口全同）。\n\
             >\n\
             > `w_ℓ` 全属 **Θ_risk**（#310 票体逐字：禁冒充缠论可导）。帽臂读数**只量化政策的成本/\
             形态影响，不评判政策取舍**（#385 Out of Scope：M4 级别帽政策本身的取舍）。\n\n",
            plain_cfg.risk.level_weights,
            plain_cfg.risk.level_weights.iter().sum::<f64>(),
        ));
    }
    report.push_str("## 四层报告\n\n");

    // ── signal 层（转引，不重算）──
    report.push_str(&format!(
        "### 层1 signal alpha（转引 M1-M4 本体结论，不重算）\n\n\
         判据：`LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0`。既有终判（`wverify_full` / goal type1）：\
         **无方向 confirmed alpha**——25 桶双门下高级别桶 n_eff≪n_min（功效门 271~1083），\
         δ-free 主裁决 + μ_R 并列 co-primary 均未过 LCB>0。三态 = **INCONCLUSIVE**\
         （非「无 alpha 存在」，措辞§5.6）。**signal 结果不外推 max-full**（措辞§5.3）。\n\n\
         {layer234_heading}\n\n\
         η 列口径注记（p128 裁定 (i)，A10 C5）：**η_corrected = tw() − cum_holding_cost 为唯一合法判读口径**\
         （cum_holding_cost = r_decomp.tw_holding_cost_bridge，与 M7 witness 增打两行同源；修正只降不升）；\
         η_T/η_* 原列保留对照。\n\n\
         随机对照三列（`Θ>随机` / `p_shift(平移)` / `p_indep(独立)`）= ★#423 收尾轮 F **新增输出**，\
         取自与 `LCB_OOS(R)` **同一次** `metrics::significance` 调用的既有字段（不新增计算）；\
         **旧产物没有这三列**（新增列，非漂移）。标量无良定义档下 `significance` 不调 ⟹ 三格标不可用。\n\n\
         | 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r(execR) | MaxDD | 声部数(A/S/F) | 终Stage | Q_T | W_T | η_T/η_* | cum_holding_cost | η_corrected(判读) | R(含浮盈) | LCB_OOS(R) | 三态 | Θ>随机 | p_shift(平移) | p_indep(独立) |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    ));

    let policy = RiskPolicy::baseline(); // κ=0（M7 冻结口径）
    let i0: i64 = 1_000_000; // I_0 基线（TwState notional_in 同源 = ⌊nav0⌋，此处报告门槛用 1e6 名义）
    let mut lee_rows: Vec<String> = Vec::new(); // ★#389 T3：帽臂 LEE 稀疏性逐窗读数（帽关时恒空）
    let mut fee_rows: Vec<String> = Vec::new(); // ★#419：逐窗 treasury 原生费率科目审计
    let mut layer23_rows = Vec::new(); // ★#419：结算文案与逐窗真实 net_r / Stage 同源
    let mut fee_audit_windows_checked = 0_usize;
    for (tag, te_lo, te_hi) in &wins {
        let test = ds.slice_date_window(te_lo, te_hi);
        if test.bars.is_empty() {
            // 空窗占位行：列数须与表头一致（★#423 收尾轮 F 加三列 ⟹ 此处同步加三个空格）。
            report.push_str(&format!("| {tag} | test 段空 | | | | | | | | | | | | | | | | | | | | |\n"));
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
        apply_m8_fee_datum_from_env(&mut cfg); // ★#388 T2 标定臂（未设 = 臂R 逐位不变）
        apply_m8_level_cap_from_env(&mut cfg); // ★#389 T3 帽臂（未设 = 臂R/臂D 逐位不变）
        cfg.margin = Some(q4_margin_model(nav_te));
        cfg.cost_model = Some(m6_cost_model());
        eprintln!(
            "[m8] {symbol} {tag} test={te_lo}..{te_hi}({}) 三系统同开 run…",
            test.bars.len()
        );
        // ★T5a (#207) shadow dump 分窗接线（T3_SHADOW_DUMP 同型）：env T5A_CHAIN_DUMP_DIR
        // 设置时逐窗开 `<dir>/t5a_chain_dump_<tag>.jsonl`；未设 = no-op（bit-exact 中性）。
        super::admission::t5a_chain_dump::open_for_window(&tag);
        let r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
        super::admission::t5a_chain_dump::close();
        assert_eq!(
            r.voice_exec.is_some(),
            voice_exec_expected,
            "#490 m8 header 执行投影与实际 OverlayRunResult.voice_exec 域不一致（窗 {tag}）"
        );

        // 层2 execution：R 分解 + MaxDD + 逐声部归因。
        let d = r.net_result.r_decomp.expect("overlay 臂经生产 π loop ⟹ 产 R 分解");
        // ★#484 / #481 HIGH-2：FeeAudit 只审计生产唯一账本真值——净额账本的真实 fill。
        // `voice_exec` 是独立证据投影，不改净额影子账本；其 env gate 中性由
        // `voice_exec_env_gate_off_bitexact_on_voice_readings` 逐位锁定。★#490 已在 fill 主循环
        // 用净额 `n_orders_executed` / `cum_fee` 对 FeeAudit 独立硬对账；此处只避免再做跨域伪对账。
        let fee = r.net_result.fee_audit;
        if !voice_exec_expected {
            assert_eq!(
                fee.n_fills, r.net_result.n_orders,
                "#419 每个真实净额 fill 恰落一笔 treasury 费审计（窗 {tag}）"
            );
            let fee_tol = 1e-9 * d.commission_slippage.abs().max(1.0);
            assert!(
                (fee.total_fee - d.commission_slippage).abs() <= fee_tol,
                "#419 {tag} treasury 科目总费 {} != R 分解 Commission+Slippage {}（tol={fee_tol}）",
                fee.total_fee,
                d.commission_slippage,
            );
        }
        let component_tol = 1e-9 * fee.total_fee.abs().max(1.0);
        assert!(
            (fee.component_total() - fee.total_fee).abs() <= component_tol,
            "#419 {tag} treasury 分项和 {} != 实扣总费 {}（tol={component_tol}）",
            fee.component_total(),
            fee.total_fee,
        );
        fee_audit_windows_checked += 1;
        fee_rows.push(format_m8_fee_audit_row(tag, r.net_result.trades.len(), fee));
        let maxdd = r.net_result.metrics.max_drawdown;
        let (mut n_amb, mut n_short, mut n_follow) = (0usize, 0usize, 0usize);
        for c in r.overlay.closed_voices() {
            match c.role_v {
                Vertical::Ambient => n_amb += 1,
                Vertical::ShortDiff => n_short += 1,
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
        layer23_rows.push((tag.clone(), d.net_r, tw.stage));
        let eta_t = tw.tw();
        let eta_star = policy.eta_star(&tw);
        // ★p128 裁定 (i)（A10 C5 口径，additive）：η_corrected = tw() − cum_holding_cost，
        // cum_holding_cost = r_decomp.tw_holding_cost_bridge（与 M7 witness 增打两行同源，禁第二查法）。
        // η_corrected 为唯一合法判读口径（修正只降不升，T-N4 语义）；η_T 原列保留对照。
        let cum_holding_cost = d.tw_holding_cost_bridge;
        let eta_corrected = eta_t - cum_holding_cost;

        // 层4 完整策略：R(含浮盈) + LCB_OOS(R) block bootstrap。
        // ★#388 T2：`fee_rate` 是 `Option`——标量无良定义档（按股档 / 按金额非对称档）为 `None`
        //   ⟹ **不算** significance，两格标不可用。标量可得档（未标定臂R、按金额对称臂D）照常算。
        // ★#423 第二阶段：本 `fee_rate` 与报告头声明段（layer4_scalar_caliber_notice）/ 结算行
        //   （layer4_verdict_line）必须同真值。两者的 exec 分别是循环内 `cfg` 与 `plain_cfg`，
        //   费率档由同一个 `apply_m8_fee_datum_from_env` 注入 ⟹ 下面这条断言把「同真值」从推理
        //   变成机器事实（若将来两处注入分叉，此处即红，不会静默产出自相矛盾的报告）。
        assert_eq!(
            r.net_result.fee_rate.is_some(),
            scalar_rate.is_some(),
            "层4 数值的标量可得性须与报告头声明段同真值（前者来自循环内 cfg、后者来自 plain_cfg）"
        );
        let r_total: f64 = r.net_result.trade_pnls_with_forced.iter().sum();
        // ★#423 收尾轮 F：保留**整个** `Significance`（此前 `.boot_ci95_lo` 就地取字段、丢掉其余）。
        //   `significance` 的调用点/入参/次数一字未动 ⟹ RNG 消耗与浮点序列逐位不变（dump 字节不变）。
        let sig = r.net_result.fee_rate.map(|fee| {
            significance(
                &r.net_result.trade_pnls, // 已实现口径（bootstrap H0:收益≤0 输入）
                &r.net_result.daily_returns,
                &r.net_result.trades,
                &r.net_result.prices,
                fee,
                r.net_result.theta_return_mtm,
            )
        });
        // LCB_OOS(R) = block bootstrap 总收益 2.5 分位下界。
        let lcb_r = sig.as_ref().map(|s| s.boot_ci95_lo);
        // 三态（完整策略层，M8:168）：LCB>0 ⟹ confirmed；R>0∧LCB≤0 ⟹ INCONCLUSIVE；R≤0 ⟹ 无（本层）。
        let (lcb_cell, verdict) = layer4_cells(lcb_r, r_total);
        // 随机对照三值的落盘出口（★#423 收尾轮 F，票体 What-to-build 交付项）。
        let (beats_cell, p_shift_cell, p_indep_cell) = layer4_random_control_cells(sig.as_ref());

        report.push_str(&format!(
            "| {tag} | {} | {:+.0} | {:.0} | {:.0} | {:.0} | {:.0} | {:+.0} | {:.4} | {}/{}/{} | {} | {} | {} | {}/{} | {} | {} | {:+.0} | {} | {} | {} | {} | {} |\n",
            r.net_result.n_orders, d.price_pnl_gross, d.commission_slippage, d.funding, d.borrow,
            d.liquidation_loss, d.net_r, maxdd, n_amb, n_short, n_follow,
            stage_str, tw.notional_in, tw.withdrawn, eta_t, eta_star,
            cum_holding_cost, eta_corrected, r_total, lcb_cell, verdict,
            beats_cell, p_shift_cell, p_indep_cell,
        ));
        eprintln!(
            "[m8] {tag}: execR={:+.0} MaxDD={:.4} stage={} R={:+.0} LCB(R)={} → {} \
             | Θ>随机={beats_cell} p_shift={p_shift_cell} p_indep={p_indep_cell}",
            d.net_r, maxdd, stage_str, r_total, lcb_cell, verdict,
        );

        // 守恒硬校验（R 分解无泄漏，与 M6 同容差）。
        let tol = 1e-3_f64.max(1e-9 * (nav_te.abs() + d.price_pnl_gross.abs()));
        assert!(
            d.conservation_residual.abs() <= tol,
            "M8 {tag} R 守恒残差 {} 超容差 {}（资金泄漏）", d.conservation_residual, tol,
        );
        // treasury 单向不可逆：stage.rank ≤ 2（EarningShares 上界），且 W_T≤notional_in（退本金不超投入）。
        assert!(tw.withdrawn <= tw.notional_in, "W_T={} 不得超 notional_in={}", tw.withdrawn, tw.notional_in);

        // ★#389 T3（帽臂 C）：LEE 稀疏性硬约束在**生产跑批**上逐窗断言（此前只在 runner.rs 的
        //   9000-bar 合成 fixture 上覆盖，见 `lee_m4_cap_on_sparsity_has_no_unexplained_violation`）。
        //   逐级判据（#363）严格强于 bar 级（#351 把 `cap_narrowed` 并入 `risk_or_cap_active`），
        //   两条都断言。**帽关时不断言**——帽关路径 `n_cap_narrowed` 恒 0，判据平凡为真（无信息）。
        //   读数行**无条件打印**（帽关时也打）——D-vs-C 的「帽政策形态影响」需要两臂同口径读数，
        //   只在帽开时打会让对照缺一半（`max_abs_net_units` / `n_cap_narrowed` 的帽关侧基准）。
        let s = r.level_order;
        // 六个稀疏性字段**单一来源**（stdout 行与产物表共用，防两处手抄漂移）。
        let [n_dec, n_cap, off_clk, off_clk_exp, off_delta, off_delta_unexp] = lee_row_cells(&s);
        eprintln!(
            "LEE_M4_ARM {tag} cap={} n_decisions={n_dec} n_cap_narrowed={n_cap} n_rescaled={} \
             max_abs_net_units={} n_orders_generated={} off_clock={off_clk} \
             off_clock_explained={off_clk_exp} off_clock_delta={off_delta} \
             off_clock_delta_unexplained={off_delta_unexp}",
            cfg.risk.enforce_level_cap, s.n_rescaled, s.max_abs_net_units, s.n_orders_generated,
        );
        if cfg.risk.enforce_level_cap {
            assert!(s.n_decisions > 0, "非空前置：{tag} 决策点跑过");
            assert!(
                s.sparsity_has_no_unexplained_violation(),
                "帽臂 {tag} bar 级稀疏性违例：无结构钟点却产订单 {} 次，其中仅 {} 次可由风控/帽解释",
                s.n_orders_off_structural_clock, s.n_orders_off_structural_clock_risk_explained,
            );
            assert!(
                s.per_level_sparsity_has_no_unexplained_violation(),
                "帽臂 {tag} 逐级稀疏性违例：无 tick 级别 Δq_ℓ≠0 共 {} 次，其中 {} 次无缩放可解释",
                s.n_levels_off_clock_delta, s.n_levels_off_clock_delta_unexplained,
            );
            lee_rows.push(format!(
                "| {tag} | {n_dec} | {n_cap} | {off_clk} | {off_clk_exp} | {off_delta} | {off_delta_unexp} |\n"
            ));
        }
    }

    // ★#423 第二阶段：层4 结算措辞随**标量可得性**分叉（不是随 `fee_schedule.is_some()`）——
    //   与声明段/两格同一个真值。分叉正文见 `layer4_verdict_line`。
    let layer4_line = layer4_verdict_line(scalar_rate);
    assert_m8_audit_coverage(&symbol, win_filter.as_deref(), fee_audit_windows_checked);
    let (layer2_line, layer3_line) = m8_layer23_settlement(&symbol, &layer23_rows);
    report.push_str(&format!(
        "\n## 判据结算（M8:163-168）\n\n\
         - **层1 signal**：INCONCLUSIVE（转引，无方向 alpha）。\n\
         {layer2_line}\n\
         {layer3_line}\n\
         {layer4_line}\n\
         I_0 报告门槛 = {i0}（notional_in 同源 ⌊nav0⌋，各窗 nav 不同 ⟹ 门槛按 notional_in 列读）。\n",
    ));
    // ★#389 T3：帽臂 LEE 稀疏性逐窗读数随产物落盘（断言已在循环内逐窗执行，本表是读数登记）。
    if !lee_rows.is_empty() {
        report.push_str(
            "\n## 帽臂 LEE 稀疏性逐窗读数（#389 / 设计文档 §F③）\n\n\
             判据：`sparsity_has_no_unexplained_violation`（bar 级）∧ \
             `per_level_sparsity_has_no_unexplained_violation`（逐级，严格更强）——\
             **两条已在循环内逐窗断言，通过才有本表**。`unexplained` 列恒 0 是硬约束，非观测。\n\n\
             | 窗 | n_decisions | n_cap_narrowed | off_clock 订单 | 其中风控/帽可解释 | off_clock 级别Δq | 未解释 |\n\
             |---|---|---|---|---|---|---|\n",
        );
        for row in &lee_rows {
            report.push_str(row);
        }
        report.push_str(
            "\n**非平凡性**：`n_cap_narrowed>0` 表示帽在本窗真 binding（否则逐级判据平凡通过，\
             #376 LOW-1 纪律）；该列若为 0，本窗的逐级绿是空断言，须照此读。\n",
        );
    }
    let fee_audit_reconciliation = if !voice_exec_expected {
        "`n_fills == n_orders`、逐科目和 `== fee_audit.total_fee == \
         RDecomposition.commission_slippage` 已逐窗硬断言。"
    } else {
        "`VOICE_EXEC=1` 时执行投影的 `n_orders/trades/RDecomposition` 与净额 `FeeAudit` \
         不同域；fill 主循环已用净额 `n_orders_executed` 与独立 `cum_fee` 对 \
         `FeeAudit.n_fills/total_fee` 逐窗硬断言，本表再硬断言逐科目和 \
         `== fee_audit.total_fee`，不作跨域伪对账。"
    };
    let fee_audit_trades_heading = if !voice_exec_expected {
        "净额trades"
    } else {
        "声部投影trades"
    };
    report.push_str(&format!(
        "\n## Treasury 真实 fill 费率科目与触达（#419，层1-3审计）\n\n\
         本表与上方同一 `run_theta_v0_pi_overlay` 返回值同源；{fee_audit_reconciliation}\
         `Σ总费` 含未标定滑点 addon，\
         有效 venue 费率排除该 addon。所有读数只作账本/费用审计，禁作 alpha 或择优输入。\n\n\
         | 窗 | {fee_audit_trades_heading} | 真实fill | Σ名义 | Σ佣金 | Σpass-through | Σ清算+CAT | Σ卖出SEC | Σ卖出TAF | \
         Σ未分类venue | Σ滑点addon | Σ总费 | 最低佣金触达 | 1%上限触达 | TAF上限触达 | venue有效费率 |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    ));
    for row in &fee_rows {
        report.push_str(row);
    }
    let report_path = resolve_m8_report_path(std::env::var("M8_REPORT_PATH").ok().as_deref());
    std::fs::write(&report_path, &report)
        .unwrap_or_else(|e| panic!("m8 报告落盘失败 {report_path}：{e}"));
    eprintln!("[m8] 端到端四层报告落盘 {report_path}");
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

    /// ★#419 RED：OKLO 不伪造 walk-forward；m8 只消费 PREREG_WINDOWS 已冻结的 §2.4 单段 OOS。
    /// BTC 未设路径仍保留原 p3fold + 前两个 OOS anchored 窗。**认识论 L0**（路由契约）。
    #[test]
    fn m8_symbol_routes_oklo_to_preregistered_single_oos() {
        assert_eq!(resolve_m8_symbol(None), "BTC");
        let btc = m8_windows("BTC");
        assert_eq!(btc.first().map(|w| w.0.as_str()), Some("p3fold"));

        assert_eq!(resolve_m8_symbol(Some("oklo")), "OKLO");
        let oklo_reg = PREREG_WINDOWS.iter().find(|w| w.symbol == "OKLO").unwrap();
        let oklo = m8_windows("OKLO");
        assert_eq!(
            oklo,
            vec![(
                "oklo_oos".to_string(),
                oklo_reg.oos.0.to_string(),
                oklo_reg.oos.1.to_string(),
            )]
        );
        assert!(oklo_reg.wf_anchored.is_empty(), "OKLO 明确无 walk-forward");
    }

    /// ★#419：未知 symbol fail-loud，禁止静默退回 BTC 后给报告打 OKLO 标签。
    #[test]
    #[should_panic(expected = "M8_SYMBOL")]
    fn m8_symbol_unknown_fails_loud() {
        let _ = resolve_m8_symbol(Some("NOPE"));
    }

    /// ★#484 / #481 MED-3：m8 报告只定义 BTC L2 与 OKLO L1 两种认识论；
    /// 即使品种已在通用数据/窗口表登记，未定义报告口径也必须 fail-loud。
    #[test]
    #[should_panic(expected = "仅支持 BTC/OKLO")]
    fn m8_registered_but_unsupported_symbol_fails_loud() {
        let _ = resolve_m8_symbol(Some("ES"));
    }

    /// ★#484 / #481 MED-3：heading 的认识论随受支持品种渲染，OKLO 禁挂 L2 实测。
    #[test]
    fn m8_epistemology_headings_match_symbol() {
        let (btc_intro, btc_layers) = m8_epistemology("BTC");
        assert!(btc_intro.contains("认识论 L2"));
        assert!(btc_layers.contains("L2 实测"));

        let (oklo_intro, oklo_layers) = m8_epistemology("OKLO");
        assert!(oklo_intro.contains("认识论 L1"));
        assert!(oklo_layers.contains("L1 实测"));
        assert!(!oklo_layers.contains("L2 实测"));
    }

    /// ★#490 MED-3 RED：报告 header 必须随实际 `voice_exec` 投影动态陈述执行域。
    #[test]
    fn m8_execution_projection_header_matches_voice_exec_state() {
        assert_eq!(
            m8_execution_projection_label(true),
            "声部执行投影 + 净额影子账本"
        );
        assert_eq!(
            m8_execution_projection_label(false),
            "净额执行 + overlay 旁路/账本"
        );
    }

    /// ★#490 MED-2 RED：未标定 BTC 费用全落 unclassified_venue 时，审计行必须显式展示，
    /// 禁止形成“可见科目全 0、Σ总费非 0”的误读面。
    #[test]
    fn m8_fee_row_renders_unclassified_venue() {
        let fee = super::super::treasury::FeeAudit {
            n_fills: 1,
            notional: 4_510_009_151.37,
            unclassified_venue: 1_353_002.745411,
            total_fee: 1_353_002.745411,
            ..Default::default()
        };
        let row = format_m8_fee_audit_row("wf8", 7, fee);
        let cells = row.split('|').map(str::trim).collect::<Vec<_>>();
        assert_eq!(
            cells[10], "1353002.745411",
            "第 10 列须为 unclassified_venue"
        );
        assert_eq!(cells[12], "1353002.745411", "Σ总费须保持原值");
    }

    /// ★#484 复审：过滤零匹配或登记窗为空时不得谎称“已逐窗硬断言”。
    #[test]
    #[should_panic(expected = "没有完成任何可审计窗口")]
    fn m8_zero_audited_windows_fails_loud() {
        assert_m8_audit_coverage("BTC", Some("missing"), 0);
    }

    /// ★#419：产物路径默认兼容旧值；本票可显式隔离到 `/tmp/419_*`。
    #[test]
    fn m8_report_path_default_and_override() {
        assert_eq!(
            resolve_m8_report_path(None),
            "/tmp/m8_e2e_all_systems_oos.md"
        );
        assert_eq!(
            resolve_m8_report_path(Some("/tmp/419_m8_oklo_treasury.md")),
            "/tmp/419_m8_oklo_treasury.md"
        );
    }

    /// ★#419 RED：OKLO 层2/3结算必须按真实逐窗值渲染，禁止复用 BTC 静态“极负 /
    /// CostReduction”模板与数据行自相矛盾。**认识论 L0**（报告一致性契约）。
    #[test]
    fn m8_oklo_layer23_settlement_tracks_actual_positive_stage_ii() {
        use super::super::super::strategy::ledger::TStage;
        let (l2, l3) = m8_layer23_settlement(
            "OKLO",
            &[("oklo_oos".to_string(), 46_635.0, TStage::CapitalRecovered)],
        );
        assert!(l2.contains("+46635"), "层2须落真实 net_r：{l2}");
        assert!(l3.contains("II(已回本)"), "层3须落真实 Stage：{l3}");
        assert!(!l2.contains("极负"), "正值不得渲染成极负：{l2}");
        assert!(
            !l3.contains("CostReduction"),
            "Stage II 不得渲染成 Stage I：{l3}"
        );
    }

    /// ★#388 T2：**标量可得档**的层4 两格（LCB_OOS(R) / 三态）渲染与降级前**逐字符相同**——
    /// 臂R（`fee_schedule=None`）产物不因本次改造漂移一个字节。三分支全覆盖
    /// （M8:168：LCB>0 ⟹ CONFIRMED；R>0∧LCB≤0 ⟹ INCONCLUSIVE；R≤0 ⟹ 无）。
    /// **认识论 L0**（渲染契约，零数据依赖）。
    ///
    /// ★#423 第二阶段：`Some` 分支的适用档从「未标定档」扩到「标量可得档」（+ 按金额对称标定档），
    /// 渲染分支本身未改 ⟹ 本测断言逐字不变（臂R bit-exact 的承保点）。
    #[test]
    fn layer4_cells_uncalibrated_render_is_unchanged() {
        assert_eq!(layer4_cells(Some(1234.5), 9000.0), ("+1234".to_string(), "CONFIRMED".to_string()));
        assert_eq!(layer4_cells(Some(-2105181.0), 6851062.0), ("-2105181".to_string(), "INCONCLUSIVE".to_string()));
        assert_eq!(layer4_cells(Some(-3550861.0), -1191916.0), ("-3550861".to_string(), "无(R≤0)".to_string()));
    }

    /// ★#388 T2 / #385 Implementation Decisions（「臂 D/C 的相关读数须标注不可用或改述」）：
    /// **标量无良定义档**⟹ 随机对照系派生的两格一律标"不可用"，**不喂近似费率**、
    /// **不落一个看似有效的数**。**认识论 L0**（契约）。
    ///
    /// ★#423 第二阶段：本标注的适用范围从「标定档」收窄到「标量无良定义档」（按股档 /
    /// 按金额非对称档）——按金额对称标定档自第一阶段起标量可得、走 `Some` 分支给真数。
    /// 档位→分支的映射由 `layer4_notice_*` 四测按真实 datum / 手工构造档逐档钉住。
    #[test]
    fn layer4_cells_calibrated_marks_unavailable() {
        let (lcb, verdict) = layer4_cells(None, 6851062.0);
        assert_eq!(lcb, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(verdict, SCALAR_UNDEFINED_UNAVAILABLE);
        assert!(SCALAR_UNDEFINED_UNAVAILABLE.contains("不可用"), "标注须自解释");
        assert!(
            SCALAR_UNDEFINED_UNAVAILABLE.contains("#374"),
            "标注须可追溯到有效域收窄票"
        );
        assert!(
            SCALAR_UNDEFINED_UNAVAILABLE.contains("#423"),
            "标注须可追溯到适用范围收窄票（分叉后不再覆盖标定档全体）"
        );
        assert!(
            !SCALAR_UNDEFINED_UNAVAILABLE.contains("标定档"),
            "标注正文不得把适用范围说成「标定档」——按金额对称标定档的读数照常产出"
        );
        // R 的正负不影响结论——三态判据本身依赖 LCB，缺 LCB 即无判据（不用 R 顶替）。
        assert_eq!(layer4_cells(None, -1.0), layer4_cells(None, 1.0));
    }

    /// ★#423 第二阶段：层4 声明段与层4 数值**同一个真值**（`scalar_cost_rate_opt` 的可得性），
    /// 不是两个判据。本组四测覆盖三分叉全部形态。
    ///
    /// **未标定档（臂R）⟹ 不出声明段**（`None`）——臂R 产物逐字节不变（bit-exact 中性）。
    /// **认识论 L0**（渲染契约，零数据依赖）。
    #[test]
    fn layer4_notice_absent_for_uncalibrated_arm() {
        let exec = ExecConfig::default();
        assert!(exec.fee_schedule.is_none(), "前置：default = 未标定档");
        assert_eq!(layer4_scalar_caliber_notice(&exec), None, "未标定档不出声明段");
    }

    /// **按金额对称档（本批臂D = Binance 现货 VIP0，maker=taker=10bp）⟹ 标量可得**：
    /// 声明段须说「照常产出」且**不含**不可用标注；层4 两格给真数、结算行走常规判据。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「读真实 datum 文件的档位形态」）。订正理由：
    /// 读真实 datum ≠ L2。L2 要求真实数据上的**假设检验**（可产生否定性结果）；本测断言的是
    /// 「档位形态 ⟹ 声明段/两格/结算行三处渲染一致」这条**渲染与分叉契约**，两格里的 LCB 数字
    /// 是写死的字面量（`-2105181.0`），不来自任何跑批 ⟹ 不可否证任何市场假设，信息增量为零。
    /// **本测能否证的假设：无**（同文件同型先例 `fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn layer4_notice_declares_available_for_symmetric_notional() {
        let exec = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_binance_spot_20260726.json:BTC:VIP0",
            )),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(
            scalar,
            Some(1e-3 + exec.slippage_bps / 10_000.0 + exec.tax_bps / 10_000.0),
            "对称按金额档：标量 = 档 10bp + 未标定滑点 + tax(=0)"
        );
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(
            !notice.contains(SCALAR_UNDEFINED_UNAVAILABLE),
            "标量可得档的声明段不得含不可用标注（那是声明与数值自相矛盾）：{notice}"
        );
        assert!(notice.contains("照常产出"), "须明说本臂层4 读数照常产出：{notice}");
        // 数值侧同一真值：层4 两格给真数，结算行走常规判据（与未标定臂同函数同口径）。
        assert_eq!(
            layer4_cells(scalar.map(|_| -2105181.0), 6851062.0),
            ("-2105181".to_string(), "INCONCLUSIVE".to_string())
        );
        assert_eq!(layer4_verdict_line(scalar), layer4_verdict_line(Some(3e-4)));
    }

    /// **按股档（IBKR Pro per-share）⟹ 标量无良定义**：声明段须含不可用标注 + 点名 per-share
    /// 理由；层4 两格标不可用、结算行给「无结论」。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「真实 IBKR datum」）。订正理由同上一测：
    /// 输入虽为真实 IBKR 费率表，被测命题却是「per-share 形态 ⟹ 三处渲染标不可用」的渲染契约，
    /// 不承载任何可被市场否证的假设 ⟹ L1。
    /// **本测能否证的假设：无**（同文件同型先例 `fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn layer4_notice_declares_unavailable_for_per_share() {
        let exec = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES",
            )),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(scalar, None, "按股档无常数等效费率");
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(notice.contains(SCALAR_UNDEFINED_UNAVAILABLE), "须标不可用：{notice}");
        assert!(notice.contains("per-share"), "须点名按股档理由：{notice}");
        assert_eq!(
            layer4_cells(scalar, 6851062.0),
            (SCALAR_UNDEFINED_UNAVAILABLE.to_string(), SCALAR_UNDEFINED_UNAVAILABLE.to_string())
        );
        assert!(layer4_verdict_line(scalar).contains("无结论"), "结算行须为无结论");
    }

    /// **按金额非对称档（maker≠taker）⟹ 落回无良定义**：角色维不消失 ⟹ 常数不由 datum 内容
    /// 唯一确定。声明段须标不可用。**认识论 L0**（手工构造档，覆盖 datum 内不存在的形态）。
    #[test]
    fn layer4_notice_declares_unavailable_for_asymmetric_notional() {
        use super::super::super::venue_fee::{FeeUnit, VenueFeeSchedule};
        let exec = ExecConfig {
            fee_schedule: Some(VenueFeeSchedule {
                venue: "SYNTH".into(),
                symbol: "BTC".into(),
                tier: "ASYM".into(),
                unit: FeeUnit::Notional { maker_bps: 5.0, taker_bps: 10.0 },
                datum_sha256: "0".repeat(64),
            }),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(scalar, None, "非对称按金额档无良定义");
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(notice.contains(SCALAR_UNDEFINED_UNAVAILABLE), "须标不可用：{notice}");
        assert!(notice.contains("maker≠taker"), "须点名非对称理由：{notice}");
        assert!(layer4_verdict_line(scalar).contains("无结论"));
    }

    /// ★#423 收尾轮 F：**随机对照三值的落盘出口**——`Θ>随机` / `p_shift` / `p_indep` 三格。
    ///
    /// 覆盖三件事：(1) 标量无良定义档（`None`）三格标不可用，与层4 两格**同一个标注串**（单一来源）；
    /// (2) 标量可得档给真数，`theta_beats_random` 渲染「是/否」、p 值四位小数；
    /// (3) 对照退化时附退化标注——p=1.0 是真算出来的，但不得被读作「未能否证」。
    /// **认识论 L0**（渲染契约，零数据依赖：`Significance` 由字面量构造）。
    #[test]
    fn layer4_random_control_cells_render_contract() {
        use super::super::metrics::Significance;
        // (1) 无良定义档：三格均标不可用（不填 0、不留空）。
        let (b, ps, pi) = layer4_random_control_cells(None);
        assert_eq!(b, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(ps, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(pi, SCALAR_UNDEFINED_UNAVAILABLE);

        let base = Significance {
            boot_mean_total_pnl: 0.0,
            boot_pvalue_pnl_le_0: 0.5,
            boot_ci95_lo: -1.0,
            boot_ci95_hi: 1.0,
            sharpe: 0.0,
            sharpe_se: 0.0,
            sharpe_ci95_lo: 0.0,
            sharpe_ci95_hi: 0.0,
            shift_mean_return: 0.0,
            shift_pvalue: 0.6234,
            indep_mean_return: 0.0,
            indep_pvalue: 0.0421,
            theta_return_same_caliber: 0.0,
            theta_return_mtm: 0.0,
            theta_beats_random: false,
            controls_degenerate: false,
        };
        // (2) 标量可得 + 非退化：真数照出。
        let (b, ps, pi) = layer4_random_control_cells(Some(&base));
        assert_eq!((b.as_str(), ps.as_str(), pi.as_str()), ("否", "0.6234", "0.0421"));
        let win = Significance { theta_beats_random: true, ..base.clone() };
        assert_eq!(layer4_random_control_cells(Some(&win)).0, "是");

        // (3) 退化：三格各附退化标注（p 值仍照实渲染，不改数也不抹掉）。
        let deg = Significance {
            shift_pvalue: 1.0,
            indep_pvalue: 1.0,
            controls_degenerate: true,
            ..base.clone()
        };
        let (b, ps, pi) = layer4_random_control_cells(Some(&deg));
        assert_eq!((b.as_str(), ps.as_str(), pi.as_str()), ("否(对照退化)", "1.0000(退化)", "1.0000(退化)"));
    }

    /// ★#423 收尾轮 F：**声明段与新增三列自洽**——加列后声明段不得再说随机对照系「本表不列」。
    ///
    /// 为什么必须有这条：无良定义档的声明段原文写「`metrics::significance` 派生的随机对照系……
    /// **本表不列**，本档下一律不产」。收尾轮 F 给三值加了落盘出口 ⟹ 本表**列**了这三列（标不可用）。
    /// 若声明段不同步，同一份报告里声明与表格自相矛盾（090 声明膨胀的镜像：声明**萎缩**）。
    /// **认识论 L0**（措辞契约，零数据依赖）。
    #[test]
    fn layer4_notice_matches_random_control_columns() {
        let per_share = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES",
            )),
            ..Default::default()
        };
        let notice = layer4_scalar_caliber_notice(&per_share).expect("标定档须出声明段");
        assert!(
            !notice.contains("本表不列"),
            "加列后不得再声明「本表不列」——三列已在表内（标不可用）：{notice}"
        );
        assert!(
            notice.contains("Θ>随机") && notice.contains("p_shift") && notice.contains("p_indep"),
            "声明段须点名这三列的列名，读者才能机械核对表头：{notice}"
        );

        let symmetric = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_binance_spot_20260726.json:BTC:VIP0",
            )),
            ..Default::default()
        };
        let notice = layer4_scalar_caliber_notice(&symmetric).expect("标定档须出声明段");
        assert!(
            notice.contains("Θ>随机") && notice.contains("★#423 收尾轮 F"),
            "标量可得档的声明段须登记这三列是本轮**新增输出**（旧产物没有这些列，不是漂移）：{notice}"
        );
    }

    /// ★#388 T2：`M8_FEE_DATUM` spec 解析——仓内 datum 文件（路径即契约）逐字段落地，
    /// sha256 随 datum 内容而来（口径标签 `[L2费率标定: datum <前12位>]` 的来源）。
    /// **认识论 L1**（读真实 datum 文件的装载算术；不主张任何 alpha）。
    #[test]
    fn fee_datum_spec_resolves_repo_datum() {
        let s = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC:VIP0");
        assert_eq!(s.venue, "BINANCE_SPOT");
        assert_eq!(s.symbol, "BTC");
        assert_eq!(s.tier, "VIP0");
        assert_eq!(s.datum_sha256.len(), 64);
        let o = parse_fee_datum_spec("venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES");
        assert_eq!(o.venue, "IBKR_PRO_US_EQUITY");
        assert_eq!(o.symbol, "OKLO");
    }

    /// ★#389 T3：`M8_LEVEL_CAP` 未设 ⟹ **no-op**——帽字段逐位不变（臂R/臂D 的 bit-exact 中性
    /// 由此承保；回归门 `scripts/check_armR_trades_digest.py` 是它的端到端实证）。
    /// **认识论 L0**（配置契约，零数据依赖）。
    #[test]
    fn m8_level_cap_unset_is_noop() {
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, None);
        assert!(!cfg.risk.enforce_level_cap, "未设 env 不得开帽");
        assert!(cfg.risk.level_weights.is_empty(), "未设 env 不得注入权重（default 空表）");
    }

    /// ★#389 T3：`M8_LEVEL_CAP=true` ⟹ 开帽 + 注入 **#310 既有配置**（6 级各 0.05，Σ=0.3≤1）。
    /// 参数归属：`w_ℓ ∈ Θ_risk`（#310 票体逐字，禁冒充缠论可导）。**认识论 L0**。
    #[test]
    fn m8_level_cap_true_applies_310_weights() {
        use super::super::super::strategy::level_risk::{level_weights_sum, level_weights_sum_le_one};
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, Some("true"));
        assert!(cfg.risk.enforce_level_cap, "帽臂须开 enforce_level_cap");
        assert_eq!(cfg.risk.level_weights, vec![0.05; 6], "权重须为 #310 既有配置");
        assert!((level_weights_sum(&cfg.risk) - 0.3).abs() < 1e-12, "Σw_ℓ=0.3");
        assert!(level_weights_sum_le_one(&cfg.risk), "Σw_ℓ≤1 机器断言（#310 验收）");
        // 与 runner.rs 两处 #310/#351 在册验收配置同值（单一来源化的见证）。
        assert_eq!(M8_LEVEL_CAP_WEIGHTS_310.to_vec(), vec![0.05; 6]);
    }

    /// ★#389 T3：非法值 ⟹ **fail-loud**。静默退回帽关会让帽臂产物 = 臂D 读数却按帽臂登记
    /// （口径谎报，090 声明膨胀）。**认识论 L0**。
    #[test]
    #[should_panic(expected = "M8_LEVEL_CAP")]
    fn m8_level_cap_invalid_value_fails_loud() {
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, Some("1"));
    }

    /// ★#388 T2：spec 三段式格式非法 ⟹ fail-loud（禁静默按未标定档跑，那会让报告标签谎报）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_malformed_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC");
    }

    /// ★#388 T2：datum 文件不存在 ⟹ fail-loud（`load_datum` 的 Err 不被吞）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_missing_file_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_does_not_exist.json:BTC:VIP0");
    }

    /// ★#388 T2：(symbol, tier) 未在册 ⟹ fail-loud（`VenueFeeBook::resolve` 的 Err 不被吞，
    /// 禁静默借别档）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_unknown_tier_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC:VIP99");
    }

    /// ★#388 T2：**品种借档拦截实证**（#360 `data::load_by_symbol` 的 fail-loud 承保本 env 钩子）——
    /// 把 OKLO 的 per-share 档配给 BTC 数据集 ⟹ `Err`，跑批在数据加载期就停，不会静默产出
    /// 「按股收费的 BTC」这种无意义费用。本测**读仓内 datum 文件**（经 `parse_fee_datum_spec`
    /// → `load_datum` 读 `venue_fee_ibkr_pro_20260726.json`），只是**不依赖市场数据文件**——
    /// 品种校验先于 `data_dir().join(file)` 读盘（`data.rs` 的 `load_by_symbol`：
    /// `if let Some(sched)` 校验块在 299 行，读盘在 308 行），故不加载 BTC 全量 bar。
    ///
    /// **语义重叠登记（#418 LOW-4）**：`data::tests::fee_schedule_symbol_must_match_dataset`
    /// （#360 已有，`data.rs:321`）覆盖同一条「品种不符 ⟹ Err」断言。本测保留的增量在于覆盖
    /// **经 env spec 解析出档位**（`parse_fee_datum_spec`）这条路径——#360 那条直接构造
    /// schedule，不穿过 env 钩子。两测断言相同、入口不同，故并存不算冗余。
    #[test]
    fn fee_datum_cross_symbol_borrow_is_blocked() {
        let mut cfg = ThetaConfig::default();
        cfg.exec.fee_schedule =
            Some(parse_fee_datum_spec("venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES"));
        let err = data::load_by_symbol("BTC", &cfg).expect_err("跨品种借档须被拦截");
        assert!(err.contains("品种不符"), "错误须点明品种不符，实得：{err}");
    }

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
        let ets = [ExitType::CloseRoot, ExitType::ReduceCore, ExitType::CloseShortDiff, ExitType::RiskExit, ExitType::Hold];
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
