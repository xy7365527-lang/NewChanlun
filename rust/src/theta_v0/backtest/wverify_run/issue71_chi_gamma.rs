use super::super::runner::{run_theta_v0_pi, run_theta_v0_pi_chi, RunResult};
use super::m8::{q4_margin_model, q4_prev_day, q4_shift_back_6m};
use super::*;

/// 首批实测冻结品种。
const DEFAULT_SYMBOL: &str = "BTC";
/// 首批实测冻结 walk-forward 窗口。
const DEFAULT_WINDOW_FILTER: &str = "wf8";
/// 未显式指定时的证据输出根。
const DEFAULT_OUTPUT_ROOT: &str = "/tmp/issue71-chi-gamma-validation";
/// p3fold 训练窗起点（闭区间）。
const P3_TRAIN_START: &str = "2022-07-01";
/// p3fold 训练窗终点（闭区间）。
const P3_TRAIN_END: &str = "2022-12-31";
/// p3fold 测试窗起点（闭区间）。
const P3_TEST_START: &str = "2023-01-01";
/// p3fold 测试窗终点（闭区间）。
const P3_TEST_END: &str = "2023-06-30";
/// 1 分钟 bar 的年化换算基数。
const MINUTE_BARS_PER_YEAR: f64 = 365.25 * 24.0 * 60.0;
/// NAV 找不到可交易正价 bar 时使用的单位价格兜底。
const NAV_FALLBACK_UNIT_PRICE: f64 = 1.0;
/// q4 保证金模型的 NAV 名义价格倍数（冻结为 1,000 个价格单位）。
const NAV_NOTIONAL_MULTIPLIER: f64 = 1000.0;
/// 单窗 μ 训练的 time-block 基址；费用率由 `ThetaConfig::exec` 计算。
const MU_TIME_BLOCK_BASE: u32 = 0;
/// χ 准入阈值：LCB(μ)>0。
const CHI_THETA: f64 = 0.0;
/// χ 单侧 95% LCB 的冻结 z_α。
const CHI_Z_ALPHA: f64 = 1.645;
/// Arm1/Arm3 的空桶策略：不放行。
const STRICT_TEAP: bool = false;
/// Arm2 的空桶敏感臂：放行。
const PERMISSIVE_TEAP: bool = true;

struct ValidationSetup {
    symbol: String,
    tag: String,
    output_root: std::path::PathBuf,
    train_lo: String,
    train_hi: String,
    test_lo: String,
    test_hi: String,
    max_bars: Option<usize>,
    train: data::Dataset,
    test: data::Dataset,
    plain_cfg: ThetaConfig,
    fullpi_train: ThetaConfig,
    fullpi_test: ThetaConfig,
    years: f64,
    nav_test: f64,
}

struct BaselineRuns {
    unset: RunResult,
    opsem: RunResult,
    both: RunResult,
    repeat: RunResult,
}

struct ArmResults {
    arm0: RunResult,
    arm1: RunResult,
    arm2: RunResult,
    arm3: RunResult,
}

fn window_dates(symbol: &str, tag: &str) -> (String, String, String, String) {
    if tag == "p3fold" {
        return (
            P3_TRAIN_START.into(),
            P3_TRAIN_END.into(),
            P3_TEST_START.into(),
            P3_TEST_END.into(),
        );
    }
    let suffix = tag
        .strip_prefix("wf")
        .unwrap_or_else(|| panic!("M8_WIN_FILTER={tag:?} 非法：须为 p3fold 或 wfN"));
    let idx: u32 = suffix
        .parse()
        .unwrap_or_else(|error| panic!("M8_WIN_FILTER={tag:?} 的 N={suffix:?} 须为整数：{error}"));
    let win = PREREG_WINDOWS
        .iter()
        .find(|w| w.symbol == symbol)
        .and_then(|w| w.wf_anchored.iter().find(|w| w.i == idx))
        .unwrap_or_else(|| {
            panic!("M8_SYMBOL={symbol:?} 未登记 M8_WIN_FILTER={tag:?}（wf index={idx}）")
        });
    (
        q4_shift_back_6m(win.test_start),
        q4_prev_day(win.test_start),
        win.test_start.to_string(),
        win.test_end.to_string(),
    )
}

fn limited_dataset(dataset: data::Dataset, max_bars: Option<usize>) -> data::Dataset {
    let data::Dataset {
        symbol,
        bars,
        dates,
        bar_seconds,
    } = dataset;
    let limit = max_bars.unwrap_or(bars.len());
    data::Dataset {
        symbol,
        bars: bars.into_iter().take(limit).collect(),
        dates: dates.into_iter().take(limit).collect(),
        bar_seconds,
    }
}

fn nav_of(dataset: &data::Dataset, config: &ThetaConfig) -> f64 {
    dataset
        .bars
        .iter()
        .find(|bar| !bar.untradable && bar.close > 0)
        .map(|bar| bar.close as f64 * config.tick.tick_size)
        .unwrap_or(NAV_FALLBACK_UNIT_PRICE)
        * NAV_NOTIONAL_MULTIPLIER
}

fn chi_config(gross: bool, nav: f64) -> ThetaConfig {
    ThetaConfig {
        risk: crate::theta_v0::config::RiskConfig {
            chi_theta: Some(CHI_THETA),
            chi_z_alpha: CHI_Z_ALPHA,
            enforce_gross_cap: gross,
            ..Default::default()
        },
        margin: gross.then(|| q4_margin_model(nav)),
        ..Default::default()
    }
}

fn create_validation_dir(path: &std::path::Path, label: &str) {
    std::fs::create_dir_all(path)
        .unwrap_or_else(|error| panic!("{label} {} 创建失败：{error}", path.display()));
}

fn optional_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            panic!("环境变量 {name} 含非 UTF-8 值")
        }
    }
}

fn max_bars_from_env() -> Option<usize> {
    optional_env("ISSUE71_MAX_BARS").map(|value| {
        let max = value
            .parse::<usize>()
            .unwrap_or_else(|error| panic!("ISSUE71_MAX_BARS={value:?} 须为正整数：{error}"));
        assert!(max > 0, "ISSUE71_MAX_BARS={value:?} 须大于 0，实得 {max}");
        max
    })
}

fn setup_validation() -> ValidationSetup {
    let symbol = optional_env("M8_SYMBOL").unwrap_or_else(|| DEFAULT_SYMBOL.into());
    assert_eq!(
        symbol, DEFAULT_SYMBOL,
        "#71 首批实测固定 M8_SYMBOL={DEFAULT_SYMBOL:?}，实得 {symbol:?}"
    );
    let tag = optional_env("M8_WIN_FILTER").unwrap_or_else(|| DEFAULT_WINDOW_FILTER.into());
    let output_root = std::path::PathBuf::from(
        optional_env("ISSUE71_OUTPUT_DIR").unwrap_or_else(|| DEFAULT_OUTPUT_ROOT.into()),
    );
    create_validation_dir(&output_root, "#71 输出根");
    let plain_cfg = ThetaConfig::default();
    let dataset = data::load_by_symbol(&symbol, &plain_cfg)
        .unwrap_or_else(|error| panic!("加载 M8_SYMBOL={symbol:?} 数据失败：{error}"));
    let (train_lo, train_hi, test_lo, test_hi) = window_dates(&symbol, &tag);
    let max_bars = max_bars_from_env();
    let train = limited_dataset(dataset.slice_date_window(&train_lo, &train_hi), max_bars);
    let test = limited_dataset(dataset.slice_date_window(&test_lo, &test_hi), max_bars);
    assert!(
        !train.bars.is_empty() && !test.bars.is_empty(),
        "#71 数据窗不得为空：symbol={symbol:?} tag={tag:?} \
         train={train_lo}..{train_hi} bars={} test={test_lo}..{test_hi} bars={} \
         ISSUE71_MAX_BARS={max_bars:?}",
        train.bars.len(),
        test.bars.len(),
    );
    let nav_test = nav_of(&test, &plain_cfg);
    let fullpi_train = chi_config(true, nav_of(&train, &plain_cfg));
    let fullpi_test = chi_config(true, nav_test);
    let years = test.bars.len() as f64 / MINUTE_BARS_PER_YEAR;
    ValidationSetup {
        symbol,
        tag,
        output_root,
        train_lo,
        train_hi,
        test_lo,
        test_hi,
        max_bars,
        train,
        test,
        plain_cfg,
        fullpi_train,
        fullpi_test,
        years,
        nav_test,
    }
}

fn run_arm<F>(
    output_root: &std::path::Path,
    name: &str,
    opsem_on: bool,
    gamma_on: bool,
    mut run: F,
) -> RunResult
where
    F: FnMut() -> RunResult,
{
    let dir = output_root.join(name);
    create_validation_dir(&dir, &format!("验证臂 {name:?} 输出目录"));
    super::super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = opsem_on.then(|| dir.join("opsem"));
    });
    super::super::gamma_dump::GAMMA_DUMP_DIR_OVERRIDE.with(|cell| {
        *cell.borrow_mut() = gamma_on.then(|| dir.join("gamma"));
    });
    let result = run();
    super::super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE.with(|cell| *cell.borrow_mut() = None);
    super::super::gamma_dump::GAMMA_DUMP_DIR_OVERRIDE.with(|cell| *cell.borrow_mut() = None);
    result
}

fn run_baseline_arms(setup: &ValidationSetup) -> BaselineRuns {
    let run_plain = || run_theta_v0_pi(&setup.test, &setup.plain_cfg, setup.years, setup.nav_test);
    BaselineRuns {
        unset: run_plain(),
        opsem: run_arm(
            &setup.output_root,
            "arm0_opsem_only",
            true,
            false,
            run_plain,
        ),
        both: run_arm(&setup.output_root, "arm0_both", true, true, run_plain),
        repeat: run_arm(&setup.output_root, "arm0_repeat", true, true, run_plain),
    }
}

fn read_validation_artifact(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path)
        .unwrap_or_else(|error| panic!("读取验证产物 {} 失败：{error}", path.display()))
}

fn assert_baseline_run(name: &str, observed: &RunResult, baseline: &RunResult) {
    assert_eq!(
        observed.n_orders, baseline.n_orders,
        "{name}: n_orders 须 bit-exact，observed={} baseline={}",
        observed.n_orders, baseline.n_orders,
    );
    assert_eq!(
        observed.trades,
        baseline.trades,
        "{name}: trades 须 bit-exact，observed_len={} baseline_len={}",
        observed.trades.len(),
        baseline.trades.len(),
    );
    assert_eq!(
        observed.trade_pnls_with_forced,
        baseline.trade_pnls_with_forced,
        "{name}: trade_pnls_with_forced 须 bit-exact，observed_len={} baseline_len={}",
        observed.trade_pnls_with_forced.len(),
        baseline.trade_pnls_with_forced.len(),
    );
    assert_eq!(
        observed.equity_curve,
        baseline.equity_curve,
        "{name}: equity 须 bit-exact，observed_len={} baseline_len={}",
        observed.equity_curve.len(),
        baseline.equity_curve.len(),
    );
}

fn three_leg_asserts(output_root: &std::path::Path, runs: &BaselineRuns) {
    for (name, observed) in [
        ("opsem_only", &runs.opsem),
        ("both", &runs.both),
        ("repeat", &runs.repeat),
    ] {
        assert_baseline_run(name, observed, &runs.unset);
    }
    for file in ["trades.jsonl", "tower_events.jsonl"] {
        let isolated_path = output_root.join("arm0_opsem_only/opsem").join(file);
        let both_path = output_root.join("arm0_both/opsem").join(file);
        assert_eq!(
            read_validation_artifact(&isolated_path),
            read_validation_artifact(&both_path),
            "隔离腿 {file} 须逐字节一致：left={} right={}",
            isolated_path.display(),
            both_path.display(),
        );
    }
    let both_path = output_root.join("arm0_both/gamma/gamma_candidates.jsonl");
    let repeat_path = output_root.join("arm0_repeat/gamma/gamma_candidates.jsonl");
    assert_eq!(
        read_validation_artifact(&both_path),
        read_validation_artifact(&repeat_path),
        "确定性腿 gamma_candidates.jsonl 须逐字节一致：left={} right={}",
        both_path.display(),
        repeat_path.display(),
    );
}

fn pnl(result: &RunResult) -> f64 {
    result.trade_pnls_with_forced.iter().sum()
}

fn render_report(setup: &ValidationSetup, arms: &ArmResults) -> String {
    let mut report = format!(
        "# #71 χ/GammaDump 四臂验证原始摘要\n\n\
         - symbol={}; window={}; train={}..{} ({} bars); \
         test={}..{} ({} bars); ISSUE71_MAX_BARS={:?}\n\
         - θ={CHI_THETA}; z_alpha={CHI_Z_ALPHA}; Arm1 teap={STRICT_TEAP}; \
         Arm2 teap={PERMISSIVE_TEAP}\n\
         - 三腿硬断言：基线输出一致；OPSEM 隔离逐字节一致；Gamma 两跑逐字节一致。\n\n\
         | 臂 | n_orders | n_trades | Σpnl |\n|---|---:|---:|---:|\n",
        setup.symbol,
        setup.tag,
        setup.train_lo,
        setup.train_hi,
        setup.train.bars.len(),
        setup.test_lo,
        setup.test_hi,
        setup.test.bars.len(),
        setup.max_bars,
    );
    for (name, result) in [
        ("Arm0 无χ", &arms.arm0),
        ("Arm1 χ teap=false", &arms.arm1),
        ("Arm2 χ teap=true", &arms.arm2),
        ("Arm3 χ 隔离", &arms.arm3),
    ] {
        report.push_str(&format!(
            "| {name} | {} | {} | {:+.8} |\n",
            result.n_orders,
            result.trade_pnls_with_forced.len(),
            pnl(result),
        ));
    }
    report
}

fn write_report(setup: &ValidationSetup, arms: &ArmResults) {
    let mut report = render_report(setup, arms);
    report.push_str(&format!(
        "\n- Arm1−Arm0: Δorders={} Δtrades={} ΔΣpnl={:+.8}\n\
         - Arm2−Arm1: Δorders={} Δtrades={} ΔΣpnl={:+.8}\n\
         - Arm1−Arm3: Δorders={} Δtrades={} ΔΣpnl={:+.8}\n",
        arms.arm1.n_orders as i64 - arms.arm0.n_orders as i64,
        arms.arm1.trade_pnls_with_forced.len() as i64
            - arms.arm0.trade_pnls_with_forced.len() as i64,
        pnl(&arms.arm1) - pnl(&arms.arm0),
        arms.arm2.n_orders as i64 - arms.arm1.n_orders as i64,
        arms.arm2.trade_pnls_with_forced.len() as i64
            - arms.arm1.trade_pnls_with_forced.len() as i64,
        pnl(&arms.arm2) - pnl(&arms.arm1),
        arms.arm1.n_orders as i64 - arms.arm3.n_orders as i64,
        arms.arm1.trade_pnls_with_forced.len() as i64
            - arms.arm3.trade_pnls_with_forced.len() as i64,
        pnl(&arms.arm1) - pnl(&arms.arm3),
    ));
    let report_path = setup.output_root.join("raw-summary.md");
    std::fs::write(&report_path, report)
        .unwrap_or_else(|error| panic!("#71 原始摘要写入 {} 失败：{error}", report_path.display()));
    eprintln!("[issue71] 验证完成：{}", report_path.display());
}

fn run_chi_arms(
    setup: &ValidationSetup,
    baseline: BaselineRuns,
    est_fullpi: &super::super::mu_estimator::MuEstimator,
    est_plain: &super::super::mu_estimator::MuEstimator,
) -> ArmResults {
    let arm1 = run_arm(
        &setup.output_root,
        "arm1_chi_teap_false",
        true,
        true,
        || {
            run_theta_v0_pi_chi(
                &setup.test,
                &setup.fullpi_test,
                setup.years,
                setup.nav_test,
                est_fullpi,
                STRICT_TEAP,
            )
        },
    );
    let arm2 = run_arm(&setup.output_root, "arm2_chi_teap_true", true, true, || {
        run_theta_v0_pi_chi(
            &setup.test,
            &setup.fullpi_test,
            setup.years,
            setup.nav_test,
            est_fullpi,
            PERMISSIVE_TEAP,
        )
    });
    let arm3_cfg = chi_config(false, setup.nav_test);
    let arm3 = run_arm(&setup.output_root, "arm3_chi_isolation", true, true, || {
        run_theta_v0_pi_chi(
            &setup.test,
            &arm3_cfg,
            setup.years,
            setup.nav_test,
            est_plain,
            STRICT_TEAP,
        )
    });
    ArmResults {
        arm0: baseline.both,
        arm1,
        arm2,
        arm3,
    }
}

/// #71 χ L0 接入验证：复用 q4 四臂口径，并将各 run 的 OPSEM/Gamma dump 独立落盘。
pub(super) fn run_issue71_chi_gamma_validation() {
    let setup = setup_validation();
    eprintln!(
        "[issue71] {} {} train={}..{}({}) test={}..{}({}) est×2",
        setup.symbol,
        setup.tag,
        setup.train_lo,
        setup.train_hi,
        setup.train.bars.len(),
        setup.test_lo,
        setup.test_hi,
        setup.test.bars.len(),
    );
    let (est_fullpi, _) =
        build_mu_from_bars(&setup.train.bars, &setup.fullpi_train, MU_TIME_BLOCK_BASE);
    let (est_plain, _) =
        build_mu_from_bars(&setup.train.bars, &setup.plain_cfg, MU_TIME_BLOCK_BASE);
    let baseline = run_baseline_arms(&setup);
    three_leg_asserts(&setup.output_root, &baseline);
    let arms = run_chi_arms(&setup, baseline, &est_fullpi, &est_plain);
    write_report(&setup, &arms);
}
