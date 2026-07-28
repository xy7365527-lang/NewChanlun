use super::*;

/// `d`（ISO "YYYY-MM-DD"）前推 6 个月，day 钳到 28（合法日期；slice_date_window 字典序比较）。
pub(super) fn q4_shift_back_6m(d: &str) -> String {
    let y: i32 = d[0..4].parse().unwrap();
    let m: u32 = d[5..7].parse().unwrap();
    let day: u32 = d[8..10].parse().unwrap();
    let (y2, m2) = if m > 6 { (y, m - 6) } else { (y - 1, m + 6) };
    format!("{y2:04}-{m2:02}-{:02}", day.min(28))
}

/// `d` 的前一天（day=1 时取上月 28 日——窗口边界钳位，合法且不与相邻 test 窗重叠）。
pub(super) fn q4_prev_day(d: &str) -> String {
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
pub(crate) fn q4_margin_model(nav0: f64) -> super::super::super::strategy::risk::MarginModel {
    use super::super::super::strategy::risk::{MarginModel, MarginSchedule, MarginScheduleBook, RiskCushions};
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
///
/// **档处置（决策统计族·χ线撤销，`chanlun/escalate/chi-line-falsification-ruling-20260728.md` §1①，
/// 2026-07-28）**：本函数是 wverify_run/m8.rs 内含 χ 臂的部分（Arm1/Arm2/Arm3 调用 χ 门），随族谱
/// 同档处置——注意本文件同模块的 `q4_margin_model`/`m6_cost_model` 等非 χ 内容不在本裁定范围内。
/// 诊断件保留（禁删，历史裁决 q4 π^full INCONCLUSIVE 照旧有效）。登记详见
/// `chanlun/review-results/prob-inference-disposition-registry-20260728.md`。
pub(super) fn run_q4_fullpi_policy() {
    use super::super::runner::{run_theta_v0_pi, run_theta_v0_pi_chi, RunResult};

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
/// [`CostModel`](super::super::super::strategy::risk::CostModel) 节头（#340 单源收敛）。下列是本函数四个
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
/// [`RATE_UNCALIBRATED_LABEL`](super::super::super::strategy::risk::RATE_UNCALIBRATED_LABEL) 强制）。
/// 真永续（真 funding datum + perp 费率）接入是**另票**（#62 数据源 + datum 版本管理），本票不做。
/// ★pub(crate)：阶段 3a 前置实装（M7_WITNESS_A10 env gate，p126 runbook §2.1）同 q4_margin_model。
pub(crate) fn m6_cost_model() -> super::super::super::strategy::risk::CostModel {
    use super::super::super::strategy::risk::CostModel;
    // 机会成本 8h=480bar、1bp/周期；现货杠杆借币每 bar 0.01bp；强平罚金 0.5%。
    CostModel::new(0.0001, 480, 0.000001, 0.005).expect("M6 常费率参数合法（冻结近似值）")
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
pub(super) fn parse_fee_datum_spec(spec: &str) -> crate::theta_v0::venue_fee::VenueFeeSchedule {
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
pub(super) const M8_LEVEL_CAP_WEIGHTS_310: [f64; 6] = [0.05; 6];

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
pub(super) fn apply_m8_level_cap(cfg: &mut ThetaConfig, spec: Option<&str>) {
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
pub(super) fn resolve_m8_symbol(spec: Option<&str>) -> String {
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

pub(super) fn m8_epistemology(symbol: &str) -> (&'static str, &'static str) {
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

pub(super) fn assert_m8_audit_coverage(symbol: &str, filter: Option<&str>, audited_windows: usize) {
    assert!(
        audited_windows > 0,
        "M8_SYMBOL={symbol:?} M8_WIN_FILTER={filter:?}：没有完成任何可审计窗口，禁止生成逐窗断言报告"
    );
}

/// m8 的已登记跑批窗。OKLO §2.4 明确无 walk-forward，故只消费它的单段 OOS；BTC 保留原
/// m8 的 p3 单折 + OOS 内前两个 anchored 窗。
pub(super) fn m8_windows(symbol: &str) -> Vec<(String, String, String)> {
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

pub(super) fn resolve_m8_report_path(spec: Option<&str>) -> String {
    let path = spec.unwrap_or("/tmp/m8_e2e_all_systems_oos.md").trim();
    assert!(!path.is_empty(), "M8_REPORT_PATH 不得为空");
    path.to_string()
}

pub(super) fn run_m8_e2e_all_systems_oos() {
    use super::super::metrics::significance;
    use super::super::runner::run_theta_v0_pi_overlay;
    use super::super::super::strategy::coverage::Vertical;
    use super::super::super::strategy::ledger::{RiskPolicy, TStage};

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
    let voice_exec_expected = super::super::admission::voice_exec_gate();
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
        super::super::super::strategy::risk::rate_calibration_label(&plain_cfg.exec),
        super::super::super::strategy::risk::RATE_UNCALIBRATED_LABEL,
    ));
    // ★#423 第二阶段：层4 标量成本口径声明段与层4 数值**同一个真值**（`scalar_cost_rate_opt`）。
    //   此前声明段按 `fee_schedule.is_some()` 判、数值按 `RunResult::fee_rate` 判——按金额对称档
    //   （第一阶段解锁）下二者分歧 ⟹ 同一份报告声明「不可用」而表格给数（090 声明膨胀）。
    let scalar_rate = super::super::treasury::scalar_cost_rate_opt(&plain_cfg.exec);
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
    let mut duplicate_id_rows: Vec<String> = Vec::new(); // ★#446：活动集 ElementId 唯一性逐窗读数（三臂都打）
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
        super::super::admission::t5a_chain_dump::open_for_window(&tag);
        super::super::super::strategy::coverage::ancok_probe_reset();
        let r = run_theta_v0_pi_overlay(&test, &cfg, years, nav_te);
        let duplicate_id_violations =
            super::super::super::strategy::coverage::ancok_probe_snapshot().duplicate_active_id_violations;
        super::super::admission::t5a_chain_dump::close();
        eprintln!(
            "ACTIVE_ID_UNIQUENESS {tag} duplicate_id_violations={duplicate_id_violations}"
        );
        duplicate_id_rows.push(format!("| {tag} | {duplicate_id_violations} |\n"));
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
    report.push_str(
        "\n## 活动集 ElementId 唯一性逐窗读数（#446）\n\n\
         `next_idx` 重复 ID 已在 coverage 生产边界 release/debug fail-loud；本表只保留逐窗观测计数，\
         不再设置第二个较晚断言点。能完成该窗报告即应为 0。\n\n\
         | 窗 | duplicate_id_violations |\n\
         |---|---:|\n",
    );
    for row in &duplicate_id_rows {
        report.push_str(row);
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
    let (fee_audit_trades_heading, fee_audit_fill_heading) =
        m8_fee_audit_headings(voice_exec_expected);
    report.push_str(&format!(
        "\n## Treasury 净额账本 fill 费率科目与触达（#419，层1-3审计）\n\n\
         本表的 fill 来自净额账本，并与上方同一 `run_theta_v0_pi_overlay` 返回值同源；{fee_audit_reconciliation}\
         `Σ总费` 含未标定滑点 addon，\
         有效 venue 费率排除该 addon。所有读数只作账本/费用审计，禁作 alpha 或择优输入。\n\n\
         | 窗 | {fee_audit_trades_heading} | {fee_audit_fill_heading} | Σ名义 | Σ佣金 | Σpass-through | Σ清算+CAT | Σ卖出SEC | Σ卖出TAF | \
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
