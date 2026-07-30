# 七处统计口径实现：产出去向清点（issue #392，map #390 子票）

**范围**：只读代码 + 归档文档 + MEMORY，不改任何 `.rs` 文件。
**方法**：逐处读源码取 estimand/se/检验方法；`grep`/`git log` 取产出去向与采信证据；凡未能用文件级证据核实的一律标注"查不到"，不推测。

---

## 一、七处逐处成表

| # | module:行 | estimand | se / 方差构造 | 检验方法 | 产出去向 | 有无生产调用 |
|---|---|---|---|---|---|---|
| 1 | `decontam.rs:66` `effective_n` | 自相关校正后有效样本数 n_eff（非 estimand 本身，是 estimand 的功效输入） | Geyer 配对初始正序列（IPS）估计 Σρk，`n_eff=n/(1+2Σρk)`，clamp τ≥1 | 不是检验，是三态判据的功效门输入 | 纯函数，L0，供 `classify_bucket`/`powered` 消费 | 是——经 `powered`/`classify_bucket` |
| 1 | `decontam.rs:115` `powered` | 功效门槛 `n_eff≥(z_α·CV)²` | 复用上条 n_eff + 调用方传入 CV | 功效判定（非 p 值） | 供 `classify_bucket` 消费 | 是 |
| 1 | `decontam.rs:123` `classify_bucket` | 逐桶 μ(z) 三态判据（Validated/Falsified/Inconclusive） | 消费外部传入 `(μ̂,LCB,UCB,perm_p,n_eff,CV)`，本身不构造 se | H0 隐含于 LCB/UCB/perm_p 输入；本函数是纯逻辑门（`powered∧μ̂>0∧perm_p<α∧LCB>0`⟹Validated 等） | **冻结判据本体**，`acc-alpha-estimand-prereg-20260701.md` §3-4 | 是——q4-fullpi-results-20260703.md 明文"三态=decontam classify_bucket" |
| 1 | `decontam.rs:146` `global_verdict` | 逐桶三态聚合为全局 Pass/Falsified/Inconclusive | 无 se，纯聚合逻辑 | 无 | 同上 | 是 |
| 2 | `perm_test.rs:196/224/255/286` 四个 `stratified_delta_perm_p*` | δ-free 残差基 `r_i=H_i−B̂_i` 池化/分桶均值（残差口径，非裸 PnL） | 无参数 se，直接置换分布经验分位 | **分层内 δ 置换**（Fisher-Yates，N_PERM=200，PERM_SEED=20260701 冻结），H0：给定分层 `(ℓ,h,time,σ^H)`，δ 标签对残差无解释力 | `HashMap<K,f64>` 返回值，供跑批层写报告 | 是——`highlow_mu.rs:98`、`wverify_run.rs` 多处（生产 wverify 管线） |
| 2 | `perm_test.rs:380` `direction_asymmetry_beta_pvalue` | 单 dummy OLS `β̂=mean(sell)−mean(buy)`（方向不对称量） | block bootstrap（非参数） | H0:β≤0，block bootstrap 单边 p | 同上 | 是——`wverify_run.rs:470` |
| 3 | `econ_positive.rs:3408` `neff_autocorr` | 逐笔 PnL 序列自相关校正有效样本数（decontam 的平行重实现，非同一函数） | lag_max=20 固定、只累加 ρk>0（无 Geyer 配对，方法与 decontam 不同） | 非检验，功效辅助量 | 见下 | **否**——仅 `#[cfg(test)] mod tests`（econ_positive.rs 内部）调用 |
| 3 | `econ_positive.rs:3425` `mean_se_lcb` | 逐笔 PnL 均值 μ，`se=σ/√neff` | 用上条本地 neff，非 decontam 的 effective_n | 构造 `LCB=μ−1.645·se`（z_alpha 硬编码 1.645，非参数化） | 见下 | 否，仅本文件测试内 |
| 3 | `econ_positive.rs:3447` `block_bootstrap_pvalue` | 同一 PnL 序列右尾显著性 | 循环移动块 bootstrap，固定种子 `0x9E3779B97F4A7C15`（与 acc-alpha 的 PERM_SEED=20260701 无关） | H0:μ≤0，block bootstrap p（非置换检验，方法与 `perm_test.rs` 不同） | 见下 | 否，仅本文件测试内 |
| 4 | `metrics.rs:612` `block_bootstrap_total` | 单次 bootstrap 重采样总收益 | 循环块 bootstrap，PREREG_SEED=20260625（`backtest-protocol-v0.md` 冻结，**独立于** acc-alpha 的 20260701 冻结） | 供 `significance()` 构造 p 值/CI | `Significance` 结构体字段 | 是——`runner.rs`/`wverify_run.rs`/`l3_fullwindow.rs`/`l3_pi_falsify.rs`/`pi_bsp_timing.rs`/`pure_bsp_timing.rs` 均调用 |
| 4 | `metrics.rs:629` `percentile_ci` | bootstrap 分布经验分位 CI | 就地排序取分位 | 供 CI 区间 | 同上 | 是 |
| 5 | `mu_estimator.rs:468` `mu_lcb` | μ(z)=E[X_γ\|Z=z] 的单边置信下界 | `mean−z_alpha·std/√n`（参数化正态近似，z_alpha 由调用方传入） | 无检验，纯置信下界估计 | 供 selector/诊断消费 | 是——`pi_bsp_timing.rs:435`（内联，非经 selector 包装）、`l3_delta_r_alpha.rs:1272`、`selector.rs` 内部 |
| 6 | `selector.rs:176` `chi_open_gate_lcb` | 包装 `mu_lcb` 做开仓门 χ | 同上（转调 mu_estimator） | 无新检验，纯选择器逻辑（L1） | 布尔开仓决策 | **否**——repo 内搜索零生产调用点，仅本文件 `#[cfg(test)]` 自测；`pi_bsp_timing.rs:435` 是逻辑重复内联而非调用本函数 |
| 7 | `l3_delta_r_alpha.rs:390` `sign_test_pvalue` | 跨品种 ΔR 均值符号一致性（系统性方向 alpha 的符号检验量） | 无参数 se，二项精确分布 | H0：符号随机 p=0.5，上单边二项检验 | 见下 | 仅本文件内部（多处 `#[test]`/`#[ignore]` 慢测调用），stdout `eprintln!`，**无 `fs::write`/无 markdown 落盘** |

---

## 二、给谁看（产出去向，逐处文件级证据）

- **decontam.rs**：本身不落盘，是判据内核；其输出被 `q4-fullpi-results-20260703.md` §2 明文引用："判据：主判据 LCB_OOS(μ(z,a))>0…三态 = decontam `classify_bucket`"。
- **perm_test.rs**：`stratified_delta_perm_p`/`_fullz`/`_uclass`/`_deltafree` 均在 `wverify_run.rs`（生产 wverify 跑批模块）内以冻结常量 `perm_test::N_PERM`/`perm_test::PERM_SEED` 调用，产出落进 `.chanlun/review-results/wverify-alpha-retest-20260702.md`。
- **econ_positive.rs 本地三件套**：仅存在于该文件 `#[cfg(test)] mod tests`（1668 行起）内，被两个测试函数调用：
  - `acc_walkforward_trainonly`（约 3183-3385 行）→ 报告落盘 `std::fs::write(...).join(".chanlun/review-results/econpositive-walkforward-20260630.md")`（econ_positive.rs:3376-3377）。
  - `acc_multilevel_highlevel_mu`（约 3571-3781 行）→ 报告落盘 `.join(".chanlun/review-results/econ-multilevel-mu-20260701.md")`（econ_positive.rs:3780-3781）。
  两份报告均采用本仓库"结果包六要素"格式（结论/定义依据/边界条件/下游推论/谱系引用/影响声明），是**具名归档判决文档**，不是纯 scratch 输出。
- **metrics.rs**：`significance()` 输出落进多个production/诊断二进制的报告流（`runner.rs`、`wverify_run.rs`、`l3_fullwindow.rs`、`l3_pi_falsify.rs`），具体见下一节采信证据。
- **mu_estimator.rs `mu_lcb`**：无独立报告落点，是被上层（selector/pi_bsp_timing/l3_delta_r_alpha）消费的库函数。
- **selector.rs `chi_open_gate_lcb`**：无任何生产调用点，产出仅存在于自身单元测试的断言里，不流向任何报告或门控（见下"重要发现"）。
- **l3_delta_r_alpha.rs `sign_test_pvalue`**：产出只通过 `eprintln!` 写 stdout（该文件内搜索 `fs::write`/`review-results` 均无命中），但其结论被**异质审查**记录进 `.chanlun/review-results/codex-diagnose-20260630-deltar-l3.md`（codex diagnose，commit 230691e0bd，逐条列出 `sign_test_pvalue` 的输入输出：n_L3=5、n_pos=3、sign_p=0.5000）——即产出经由第三方审查文档间接落盘，但本模块自身不写 markdown。

---

## 三、被采信过没有（本票重点，逐处给文件级证据）

### 3.1 decontam.rs / perm_test.rs（冻结口径本体 + perm_p 生产者）—— **是，且是终局依据**

`.chanlun/review-results/q4-fullpi-results-20260703.md`（task #135，"冻结先于跑数"的正式 prereg 流程，MEMORY 中 `project_q4_fullpi_no_alpha.md` 直接引用为"q4 π^full终判=无alpha"）第 8 行明文：

> "判据：主判据 LCB_OOS(μ(z,a))>0（§12 口径，z_α=1.645，非 p<0.05）；三态 = decontam `classify_bucket`（VALIDATED ⟺ powered ∧ LCB>0 ∧ perm_p<0.05；FALSIFIED ⟺ powered ∧ UCB<0）+ 逐桶功效门 `n_eff≥(1.645·CV)²`"

这是当前仓库"无 alpha"这一主线终局结论所依赖的判据本体——**是**，且是被认定为最终依据（非探索性）的那一层。

### 3.2 metrics.rs（`block_bootstrap_total`/`percentile_ci`，经 `significance()`）—— **是**

`.claude` MEMORY 中 `project_stheta_v1_fullwindow_l3_falsified.md`（S_Θ v1 8/8 全否证的结算记录）原文：

> "n_beats_random双口径已实现(**metrics::significance** shift_pvalue+indep_pvalue,theta_beats_random⟺两对照p≤0.05),0/8。"

函数名被直接点名写进已结算的 MEMORY 条目，是明确的采信证据。

### 3.3 mu_estimator.rs `mu_lcb`（经 `wverify_run.rs`/`pi_bsp_timing.rs`）—— **是（后被订正）**

`project_wverify_alpha_retest_pass.md` 记录了基于 `mu_lcb`+`perm_test` 的 W-VERIFY 主桶 "PASS，LCB=+183.86"，随后同一 MEMORY 文件自我订正："★正式作废（#135 q4 定判）...新 typed exit 口径重跑同桶...LCB=−42.6——正点估计完全消失。旧 PASS 的 +183.86 是出场口径伪影"。**采信过，且事后被同一批判据（decontam/mu_estimator/perm_test 体系内部）自行订正**——这是判据内部的迭代收敛，不是本票要追的"另立判据"问题。

### 3.4 `econ_positive.rs` 本地三件套（`neff_autocorr`/`mean_se_lcb`/`block_bootstrap_pvalue`）—— **是，部分采信，且时间线关键**

这是本票要害。证据链：

1. **两份归档判决文档直接由本地口径产出裁决**：
   - `econpositive-walkforward-20260630.md` 用 `mean_se_lcb`（`LCB=−2.9972e2`）+ `block_bootstrap_pvalue`（`p=0.3498`）判定"level0卖…§11 稳健性验收未过…诚实否证"（该文件"结果包六要素"第1条）。
   - `econ-multilevel-mu-20260701.md` 用同一套本地 helper 判定"level2+ 全否证，高级别无稳健 OOS alpha"（同上第1条）。
   两份都是本仓库正式"结果包六要素"格式的归档文档，不是草稿。

2. **这两份文档被后续归档文档具名引用为支持性先例**：
   - `.chanlun/review-results/oddeven-mu-causation-20260701.md` 第83行："μ̂不是…级别方向alpha…是持有窗方向经验分布的产物，winner's curse风险，**见econpositive-walkforward-20260630.md**"。
   - `.chanlun/review-results/oddeven-causation-counterfactual-20260701.md` 第71行："…μ̂正类不可作独立entry升基座(置换坐实winner's curse,非仅怀疑，**呼应 econpositive-walkforward-20260630**)"。

3. **上述因果链结论已结晶进当前仍在 MEMORY.md 索引中的 ★★ 条目** `project_oddeven_mu_identity.md`：
   > "beta 漂移主因结论（反事实置换）结构性稳健，但 μ̂ 数字须 #135 新口径重跑。"
   即：具体数字被 #135 标记为口径过期，但**建立在 econ_positive 本地口径之上的定性判决（"不是可交易 alpha，是 winner's curse/beta 漂移"）本身被当前 MEMORY 当作结构性稳健的结论持有**，未被撤回。

4. **异质审查状态**：`oddeven-mu-causation-20260701.md`/`oddeven-causation-counterfactual-20260701.md` 均自陈"异质审查缺席（codex配额429耗尽）…本成因结论未经异质否定"——即这条依赖本地口径的因果结论从未经过第三方（codex）复核就进入了 MEMORY。

5. **时间线（`git log --follow --diff-filter=A`，逐文件首次提交时间）**——这决定了性质判断：

   | 文件 | 首次提交时间（commit） |
   |---|---|
   | `econpositive-walkforward-20260630.md` | 2026-06-30 15:34（commit 20c0ab5241） |
   | `oddeven-mu-causation-20260701.md` | 2026-07-01 09:32（commit 7088a7cde8） |
   | `oddeven-causation-counterfactual-20260701.md` | 2026-07-01 09:45（commit f212c9323c） |
   | `econ-multilevel-mu-20260701.md` | 2026-07-01 16:09（commit 8d45dc449b） |
   | `decontam.rs`（冻结判据模块本体） | 2026-07-01 21:42（commit f2ba46cede） |
   | `acc-alpha-estimand-prereg-20260701.md`（正式冻结文档） | 2026-07-02 01:21（commit 9bf1c49827） |

   **四份基于 econ_positive 本地口径的文档全部先于 `decontam.rs` 模块创建、也先于 acc-alpha 正式冻结文档落地**。因此不能把这条链定性为"冻结判据已生效后又绕开它另立新判据"（事后改判据，硬性违反预注册 §0）——严格说，当时压根还没有可绕开的冻结判据。**准确定性是**：本地口径先于任何 prereg 存在、未经异质审查即产出了裁决性结论并被写入长期记忆；此后（07-01 21:42 起）仓库建立了 acc-alpha 冻结判据体系，但**从未回头用冻结判据重验 econ_positive 本地口径撑起的那条因果结论**——它至今仍以"结构性稳健"的身份留在 MEMORY 里，与冻结判据体系并存、互不校验。

### 3.5 selector.rs `chi_open_gate_lcb`（生产准入门）—— **未发现采信证据；且未接入生产**

repo 全文搜索 `chi_open_gate_lcb` 只命中 `selector.rs` 自身（定义 + 6 处 `#[cfg(test)]` 断言），**零生产调用点**。`.chanlun/review-results/built-but-unwired-pattern-20260723.md` 中搜索未命中该函数名（查不到，未在已知"建而不接"清单中登记）。

需要说明一个容易误判的细节：`rust/src/bin/pi_bsp_timing.rs:435` 确实在开仓路径里做了"LCB 门"判断（`chi_t(est.mu_lcb(&z, z_alpha), theta, true, true, true)`），但这是**该 bin 自己内联复刻的等价逻辑**，不是调用 `selector::chi_open_gate_lcb`——即 LCB 门控这个*概念*在别处被用了，但 `selector.rs:176` 这个具名函数本身没有被采信、也没有被生产路径调用过。map #390 把它标注为"生产准入门"，但目前代码事实是：这个函数是孤立的、未接线的。

### 3.6 l3_delta_r_alpha.rs `sign_test_pvalue`（跨品种 ΔR 符号检验）—— **进入了异质审查文档，未查到进入 MEMORY**

其结论被完整记录进 `.chanlun/review-results/codex-diagnose-20260630-deltar-l3.md`（codex diagnose 对"无 alpha"否定性结论的异质复核，逐条列出 n_L3=5/n_pos=3/sign_p=0.5000 等具体数字）。但在 MEMORY 目录（`/Users/silencehan/.claude/projects/-Users-silencehan-Projects-NewChanlun/memory/`）中搜索 `delta_r_alpha_multi_symbol`/`Phase-3 ΔR`/`净额增量` **无命中**——查不到它被结晶进任何已结算的 project memory 条目，如实报告为"查不到"，不推测其未被采信或已被采信。

---

## 四、专节：`econ_positive.rs` 本地口径的产出被采信过吗？

**结论：部分（是，但有严格边界）。**

- **是**：其两个本地 helper 产出的 LCB/bootstrap-p 直接构成了两份具名归档"结果包"的裁决依据（3.4 节第1点），这两份归档判决又被后续两份归档文档具名引用为支持性先例（第2点），最终这条因果链被写入当前仍在索引中的 ★★ 结算记忆 `project_oddeven_mu_identity.md`（第3点）——按定性结论而非具体数字口径，它至今仍被当作"结构性稳健"持有，不是一次性用过就废弃的探索性代码。
- **边界**：
  1. 时间线显示这套本地口径先于 decontam.rs 冻结判据体系存在（3.4 节第5点）——不构成"冻结后又另立判据绕开冻结"的严格预注册违规叙事；更准确的定性是"未经冻结、未经异质审查的本地口径产出直接写入长期记忆，且事后从未被冻结判据回溯校验"。
  2. 具体数字（LCB=-2.9972e2 等）已被 #135（typed exit 口径重构）标记为过期，但承载在其上的定性结论未被相应订正或标注为待重验。
  3. 未查到这套本地口径的产出经过 codex 等异质审查（oddeven 两文档均自陈审查缺席）。

---

## 五、查不到的项（如实列出，不填充推测）

- 未找到 `econ_positive.rs` 本地三件套或其两份报告被 MEMORY.md 除 `project_oddeven_mu_identity.md` 外的其他结算条目直接引用的证据。
- 未找到 `selector::chi_open_gate_lcb` 出现在 `.chanlun/review-results/built-but-unwired-pattern-20260723.md`已知清单中的记录（该清单未提及此函数，非"确认未登记"，只是未命中）。
- 未找到 `l3_delta_r_alpha.rs::sign_test_pvalue` 的产出被写入任何 MEMORY project 条目的证据。
- 未逐一核实 `metrics.rs::significance()` 在 `l3_pi_falsify.rs`/`pure_bsp_timing.rs` 内的具体调用上下文是否也各自落过归档报告（只核实了其函数名被 MEMORY 直接点名一次，已足以回答"是否被采信"，但未做穷尽式落点清点）。
