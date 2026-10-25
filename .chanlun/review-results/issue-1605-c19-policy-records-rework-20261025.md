# #1605 C-19 版本化记录返修：实施证据

日期：2026-10-25
分支：`sandcastle/issue-1605`
代码候选：`31178ea1d8e177dd0e3c76d87225693d5c47d665`
性质：**作者实施与自审证据，不冒充票面要求的非作者独立复审。**

## 1. 返修结论与范围

本次不是给旧常数加注释，而是把真实生产回测、研究回放和真实数据验收入口接到具名版本化记录。记录缺失、字段缺失或值非法时，真实 runner 在分类、订单和指标读数之前返回 `PolicyError`；其他验收/探针入口在产生读数前显式停止。没有把 `evidence_gate` 的六个字段或采样单位并入 C-19，也没有代做 #1398/#1410/#1441 的下游。

旧根目录自述和未接线的 `docs/policies/*.txt` 平行记录已删除。生效记录统一由 `theta_v0::policy_records` 的 `include_str!` JSON 承载，输出使用 `PolicyRef::label()` 显示记录 id/version。

## 2. 生效记录与数值来源

| 记录 | v1 内容 | 用途与边界 |
|---|---|---|
| `#1605/research-stats/v1` | 20 个研究统计字段；含 `z_alpha=1.645`、`perm_alpha=0.05`、年化 252、显著性门 0.05、CVaR 0.05、CI 0.025/0.975、Sharpe CI z=1.96、ICC 0.3/0.7，以及各采样域的块长、重采样数和最小样本数 | C-19 研究回测与验收。数值是现行实现的等值迁移，不声称重新签署研究判据。 |
| `#1677/evidence-gate/v1` | confidence、block_unit、min_events、min_blocks、min_windows、min_coverage | ADR 0022 独立记录；只迁移/保留读取器，不用它给 C-19 补默认值。 |
| `#1605/l3-probe-profile/v1` | `max(first_px * 1000, 1e6)`，缺首价沿用旧数据回退 1.0 | L3 同族探针资金。 |
| `#1605/no-floor-probe-profile/v1` | `first_px * 1000`，`nav_floor=0`，缺首价沿用旧数据回退 1.0 | W-VERIFY、issue837、issue841；不得套 L3 的 1e6 下限。 |

`research-stats.json` 对交易序列、ΔR/bar 序列和 econ 收益观测分别命名：`trade_block_len=5`、`delta_r_block_len=20`、`econ_block_len=20`，重采样数也分别记录，避免同值数字跨采样单位误接。

## 3. 全部真实消费面

检索目录为 `rust/src/theta_v0/backtest/` 与 `rust/src/bin/`。先按指标、显著性、桶判定、L3、真实验收和探针资金分域，再沿公开 runner/metrics 调用点反查；没有以“文件位于 `tests.rs` 或 `cfg(test)`”为由豁免真实数据验收。

### 3.1 生产 runner、指标与命令行

| 消费入口 | 已接记录的实际量 |
|---|---|
| `runner::run_theta_v0_pi`、`run_theta_v0_pi_overlay` | 入口先加载 research-stats；同一份记录传入核心，指标用 `compute_with_policy`，避免核心再次读出另一版本。 |
| `metrics::compute`、`metrics::significance` | 年化、CVaR tail、Sharpe CI z、bootstrap CI 分位、交易序列块长/重采样数、随机对照显著性门。公开接口返回 `Result<_, PolicyError>`。 |
| `theta_backtest`、`theta_overlay` | 通过生产 runner 消费；stdout 显示实际 `Metrics.research_stats` label。 |
| `pi_bsp_timing`、`pure_bsp_timing` | 直接调用 metrics/significance；stdout 显示实际 metrics label。 |
| `theta_m1_dual` | 生产 runner 与报告统计消费；stdout、Markdown、JSON 均显示 research-stats label。其既有 ×10000 NAV 口径未被混改。 |

### 3.2 decontam / highlow / W-VERIFY

- `decontam::classify_bucket` 保持纯函数参数接口；真实调用方不再传代码常数。
- `highlow_mu::cell_stats` 经 `cell_stats_with_policy` 给 baseline、D1、D2 切片统一传 `z_alpha`/`perm_alpha`；报告带 research-stats label。
- `wverify_run` 的真实 `classify_bucket` 调用位于 `deltafree_verdict`、`wverify_full`、`bucket_verdict`、`wverify_fullz`；全部从 research-stats 取值。
- `wverify_run` 的生产/回放入口还包括 `wverify_cross_symbol`、`thetadom_three_gauge_oos`、`policy_backtest`、`q4_fullpi_policy`、`m6_btc_oos_r_decomposition`、`m8_e2e_all_systems_oos`、`issue755_level_cap_on_off_btc20k_diff`、`flip_guard_wf8_onebar_prune_replay`、`center_lifecycle_wf8_events_replay`、`otherwise_domain_wf8_grade_buckets`、`deltafree_exact_recompute` 等；runner/metrics 走 research-stats，首价 ×1000 资金走 no-floor profile。
- 实际 Markdown 写盘由 `with_policy_records` 统一加研究统计与资金记录 label，覆盖 full rows、H2、force-state、delta-free、μ/R、timeblocks、exit-type、跨标的、full-z、u-class、三 gauge、policy、Q4、M6、M8、issue755 与 exact recompute 报告。

### 3.3 L3 研究、准入和探针

| 模块/入口 | 已接记录的实际量 |
|---|---|
| `l3_delta_r_alpha::build_mu_from_bars` | time-block bar 数；供 highlow/W-VERIFY 等同源残差链使用。 |
| `delta_r_alpha_multi_symbol` | 最短 train/test bar、ΔR bootstrap 块长/重采样数、显著性门、符号检验最小功效数、L3 探针 NAV。 |
| `crossfit_l2` | min bars、crossfit 构造边界、ΔR bootstrap 与显著性/功效门。 |
| `martingale_impossibility_guard`、`lcb_vs_naive_l2`、`three_way_l2` | 对应 min bars、z_alpha、显著性门及 ΔR/gross 统计参数。 |
| `pooling_icc_multi_symbol` | ICC low/high 门及 crossfit min bars；报告阈值和统计记录 label 随记录渲染。 |
| `l3_fullwindow_multi_symbol_significance`、`l3_pi_falsify_multi_symbol_significance` | runner/metrics、最小交易数、bootstrap 显著性门、L3 NAV；输出研究统计和资金记录 label。 |
| `l3_pi_probe::{pi_probe_scaling_single_symbol, pi_production_orders_at_64k, pi_probe_zero_orders_root_cause}` | runner/metrics 与 L3 NAV；输出两类 label。 |
| `l3_pi_probe::pi_probe_delta_candidates_unlock_orders` | 只直接消费 L3 NAV，因此只输出 L3 profile label，不冒充消费 research-stats。纯结构探针不硬塞无关记录。 |

### 3.4 其他真实数据验收与回放

- `econ_positive/tests.rs` 的真实入口 `acc_level0sell_oos`、`acc_multilevel_highlevel_mu` 及 baseline/分类辅助路径：读取 `econ_neff_lag`、`z_alpha`、`econ_block_len`、`econ_n_resample` 和综合显著性门；报告正文带 label 和实际参数。它们虽在测试模块中，仍按真实验收消费处理。
- `issue837_probe::issue837_cross_level_opposing_actions`：runner/metrics + no-floor NAV；报告同时带两类 label。
- `issue841_probe::{issue841_axis_funded, issue841_level_cap_counterfactual}`：runner/metrics + no-floor NAV；报告同时带两类 label。`issue841_axis_structure` 是纯结构口径，没有 runner/NAV/C-19 消费，未添加无关记录。
- `runner_tests.rs` 的真实数据资金点：`btc_type2_open_short_channel_witness`、`m3_follow_parent_short_leg_termination_witness`、`btc_prune_leg_exit_type_matches_account_identity`、`btc_type2_residual_correction_witness`、`m8_treasury_reach_distribution_real_btc`。全部复用同一次加载的 L3 profile；stderr/报告可追溯到 `#1605/l3-probe-profile/v1`。

### 3.5 输出追溯边界

`/tmp/wv_full_zdecision.tsv` 保持既有机器可读 schema，没有插入会破坏列解析的注释行；同批 Markdown 携带本批记录版本。这里没有把“结构体新增字段”当作输出已完成：五个 CLI 的 stdout、theta_m1_dual 的 Markdown/JSON，以及真实报告正文均另做了输出审计。

## 4. 穷举检索与排除分类

最终检索证据保存在实施环境的：

- `/tmp/issue1605-final-consumers.txt`：字段级消费点；
- `/tmp/issue1605-final-constants.txt`：相关常数复扫；
- `/tmp/issue1605-final-nav.txt`：首价、倍率、NAV 下限扫描；
- `/tmp/issue1605-final-seams.txt`：decontam、runner、metrics 判定/生产接缝；
- `/tmp/issue1605-final-record-loads.txt`：记录加载点；
- `/tmp/issue1605-final-public-calls.txt`：公开 runner/metrics 调用点；
- `/tmp/issue1605-final-output-trace.txt`：报告与命令行 label 输出点。

核心检索式（在上述两个目录执行）：

```bash
rg -n '1\.645|1\.96|0\.025|0\.975|0\.05|0\.3|0\.7|252|1000|2000|43200'
rg -n 'stats\.(z_alpha|perm_alpha|annualization_days|significance_p_threshold|cvar_tail|ci_lower_quantile|ci_upper_quantile|sharpe_ci_z|icc_low_threshold|icc_high_threshold|trade_block_len|trade_n_resample|delta_r_n_resample|delta_r_block_len|min_test_bars|time_block_bars|min_trades_for_l2|econ_neff_lag|econ_block_len|econ_n_resample)'
rg -n 'run_theta_v0_pi|run_theta_v0_pi_overlay|metrics::compute|metrics::significance|classify_bucket'
rg -n 'active_research_stats|active_l3_probe_profile|active_no_floor_probe_profile|first_px|nav_floor|10_000|1000\.0|1\.0e6'
rg -n '统计记录|探针资金记录|research_stats.*label|record\.label'
```

扫描命中按以下类别分别处理，而非一律替换同值数字：

1. 真实 C-19 生产/回放/真实验收消费：本票接线。
2. 普通合成测试夹具：保留固定参数；不作为生产默认值豁免真实数据路径。
3. `replay_dump`、`theta_backtest`、`theta_overlay`、`theta_pi_diff` 等 `first_px.abs()*10000`：是另一资金口径，本票不混改。
4. 通用算法参数、CLI 参数、随机种子和独立实验预注册参数：不因其数值相同就并入 C-19。
5. `evidence_gate`：独立记录、独立样本单位，不与 research-stats 互备。

## 5. 等值对拍、负控与 fail-closed

| 测试 | 实际证明 |
|---|---|
| `research_stats::tests::{active_v1_is_complete_and_carries_its_identity, missing_unsigned_and_invalid_never_fall_back}` | active id/version 与完整字段；RecordMissing、AwaitingPolicy、Invalid 均为 typed error，不读取旧默认值顶替。 |
| `runner::tests::research_stats_record_failure_stops_real_runner_and_recovery_is_bit_exact` | 在真实核心接缝注入缺 z_alpha、z_alpha=0、None：分别 AwaitingPolicy/Invalid/RecordMissing，均无 RunResult；恢复 active 后订单数、metrics、trade_pnls 与 baseline 完全相等。 |
| `metrics::tests::active_record_is_bit_exact_and_policy_mutations_reach_metric_outputs` | 真实公开 `compute` 与 active `_with_policy` 逐字段相等；label 正确；年化、CVaR、Sharpe CI z、CI 分位、交易块长/重采样数的变异均改变相应实际输出。生产若恢复旧常数而 `_with_policy` 仍读记录，等值断言会变红。 |
| `metrics::tests::significance_threshold_reaches_random_control_verdict` | 显著性门变异改变随机对照判定。 |
| `highlow_mu::self_check::research_stats_z_alpha_reaches_cell_stats` | highlow active 等值，z_alpha 变异到达真实 cell stats。 |
| `decontam::tests::perm_alpha_changes_real_bucket_verdict` | 同一 `perm_p=0.04`，α=0.05 为 Validated、α=0.01 为 Inconclusive。 |
| `l3_probe_profile::tests`（3 例） | active identity、两类真实 NAV 与旧公式逐值相同、首根不可交易/空 bars、缺字段/非法值 typed error；倍率负控在无下限 profile 上进行，避免 1e6 下限遮蔽。 |
| `l3_delta_r_alpha::tests::research_stats_reach_delta_r_bootstrap_and_sign_test_power` | 非恒正 ΔR 序列中，改重采样数/块长保持原均值而 boot p 改变；active 最小全正品种数=5，显著性门改 0.01 后为 7。 |
| `econ_positive::tests::research_stats_reach_econ_acceptance_helpers` | neff lag、z_alpha LCB、econ block/resample 的变异均到达真实验收 helper 读数。 |
| `wverify_run::tests::report_body_carries_both_policy_record_versions` | 实际报告正文同时含统计与资金记录 label。 |

负控夹具特意避开退化情形：L3 NAV 下限会遮蔽倍率变化；恒正 ΔR 的 bootstrap p 可始终为 0；Sharpe SE=0 时改 z 不改变 CI；最差收益相同时改 CVaR tail 可能不改变读数。

## 6. 实际远程验证

全程未在本机运行 `cargo`、`rustc` 或 `cargo test`。真实编译/测试使用：

```bash
NC_REPO="$PWD" nc-remote sandcastle/issue-1605 rust <command...>
```

定向测试（均 exit=0）：

| 远程日志 | 覆盖 |
|---|---|
| `/tmp/nc-remote/20261025-091647-447147a352.log` | research-stats 完整身份与缺失/未签/非法，2 passed |
| `/tmp/nc-remote/20261025-091829-447147a352.log` | 真实 runner 失败停止与恢复逐位对拍，1 passed |
| `/tmp/nc-remote/20261025-092045-447147a352.log` | highlow z_alpha 消费域，1 passed |
| `/tmp/nc-remote/20261025-092128-447147a352.log` | decontam perm_alpha 判定负控，1 passed |
| `/tmp/nc-remote/20261025-093104-f6ec3ed39b.log` | 两类 probe profile 等值、NAV 消费和 typed error，3 passed |
| `/tmp/nc-remote/20261025-093424-f6ec3ed39b.log` | econ 真实验收 helper 消费域，1 passed |
| `/tmp/nc-remote/20261025-093659-02212de727.log` | metrics 等值与全部指标变异，1 passed |
| `/tmp/nc-remote/20261025-093823-02212de727.log` | ΔR bootstrap 与最小符号检验功效，1 passed |
| `/tmp/nc-remote/20261025-095428-18f99de04b.log` | W-VERIFY 报告双记录版本，1 passed |

代码候选 `31178ea1d8`：

| 命令 | 结果与日志 |
|---|---|
| `cargo check --locked --all-targets` | exit=0；`/tmp/nc-remote/20261025-101441-31178ea1d8.log` |
| `cargo fmt --all -- --check` | exit=0；`/tmp/nc-remote/20261025-101554-31178ea1d8.log` |
| `cargo test --locked --all-targets` | exit=0；lib **2913 passed / 0 failed / 153 ignored**，其余 all-targets 均通过；`/tmp/nc-remote/20261025-101613-31178ea1d8.log` |

此前 `6a7baf9794` 的完整套件已 exit=0：lib 为 2913 passed、0 failed、153 ignored，其余 all-targets 测试通过；日志 `/tmp/nc-remote/20261025-100354-6a7baf9794.log`。这只作历史证据，不替代最新代码候选的完整测试。

## 7. 未做事项与解释边界

- 默认完整套件会跳过 `#[ignore]` 慢测。本轮没有重跑需要真实 BTC/多品种全历史的 ignored 验收，因此不声称这些真实市场读数已复跑；接通的是它们的真实代码入口，非伪造运行结果。
- W-VERIFY H2 的残差交易序列方向不对称 bootstrap 仍是独立预注册 `N=1000, block=20`。它不是 metrics 交易 bootstrap、ΔR bar bootstrap 或 econ 收益 bootstrap，未被静默并入某个同值字段。
- 本票只做现行数值的等值记录化，不重新签署统计判据，不改 blocking 边、生产服务或其他票参数。
- 非作者独立复审由主控另派；本文件不代替该验收项。
