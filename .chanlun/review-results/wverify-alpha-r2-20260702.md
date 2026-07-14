# W-VERIFY acc-alpha 定版重跑 r2（n_eff 自相关校正落实 + 全 OOS 窗）

- 工位 ws-neff | 任务 #15 | codex #4 verdict=AUDIT_DOWNGRADE 的修复重跑
- 锚 SHA=`bb21146d91385e56baede49d055f9775d86e8443`（HEAD，n_eff 校正落实后）
- 认识论 **L2**（单标的 BTC）。**有效域升级**：不再是 r1 的 32k 前缀截断——本轮跑**全 OOS 窗** `2023-01-01…2025-06-30`（`build_mu_from_bars` 非 O(n²)，156s 完成），r1 的 O(n²) 截窗有效域边界（231号）在本轮消除。
- 输入 trades=3602，buckets=11，levels=[0,1,2,3,4,5]

## 修复内容（codex 三缺口对应）

1. **n_eff 自相关校正落实**（codex 致命缺口）：`decontam::effective_n`（新增）实现 `n_eff = n/(1+2Σρk)`，Σρk 用 Geyer 初始正序列估计（逐滞后样本自相关，首个非正 ρk 处截断，保证 Σρk≥0 ⟹ n_eff≤n）。`wverify_run.rs` 按成交时间序逐桶收集 X_γ 序列喂入；`powered`/`classify_bucket` 的 `n_eff` 形参 usize→**f64**（codex 建议）。生产调用点从 `n as usize` 改为校正后 `n_eff`。
2. **多重检验 family 注记**（见下 §多重检验）。
3. **时段限定**：全 OOS 窗（见上，比 r1 更强）。

## 校正后逐桶三态表（n_eff 列为自相关校正量）

| ℓ | bsp_class | δ | n_raw | **n_eff** | μ̂ | LCB | UCB | CV | perm_p | state |
|---|---|---|---|---|---|---|---|---|---|---|
| L0 | 3 | -1 | 1503 | 355.24 | -73.49 | -129.21 | -17.77 | 17.869 | 1.000 | Inconclusive |
| **L0** | **3** | **+1** | **1714** | **158.71** | **259.64** | **183.95** | **335.34** | **7.337** | **0/200** | **Validated** |
| L1 | 2 | -1 | 132 | 116.09 | 75.07 | -96.19 | 246.33 | 15.933 | 0.190 | Inconclusive |
| L1 | 2 | +1 | 132 | 92.78 | -67.47 | -258.59 | 123.65 | 19.784 | 0.810 | Inconclusive |
| L2 | 2 | -1 | 46 | 28.44 | 62.08 | -192.57 | 316.74 | 16.912 | 0.645 | Inconclusive |
| L2 | 2 | +1 | 48 | 48.00 | 190.58 | -111.85 | 493.01 | 6.683 | 0.355 | Inconclusive |
| L3 | 2 | -1 | 9 | 9.00 | -90.39 | -355.83 | 175.05 | 5.355 | 0.525 | Inconclusive |
| L3 | 2 | +1 | 11 | 10.86 | -69.36 | -621.08 | 482.36 | 16.038 | 0.475 | Inconclusive |
| L4 | 2 | -1 | 4 | 4.00 | -149.60 | -308.44 | 9.24 | 1.291 | 0.770 | Inconclusive |
| L4 | 2 | +1 | 2 | 2.00 | 589.19 | -1017.26 | 2195.63 | 2.344 | 0.270 | Inconclusive |
| L5 | 2 | -1 | 1 | 1.00 | -9.93 | NaN | NaN | NaN | 1.000 | Inconclusive |

代码级 verdict=**Pass**（V=1 F=0 I=10）。

## 关键发现：codex 预期被全窗数据翻转（照实，161）

codex 预期"校正后大概率 Global=INCONCLUSIVE（n_eff≈9-22<27.52）"，前提是 **r1 的 32k 截窗** 唯一 VALIDATED 桶 n=44、门槛≈27.52。本轮全窗后该桶 **n_raw=1714**：

- 自相关校正 n_eff=158.71（**neff/nraw=0.093**，比 665 的 0.21 更狠——1714 个同向三类买点时间聚集更强），校正**确实生效**（不是原始 n）。
- 功效门 = `(1.645·7.337)² = 145.67`；n_eff=158.71 ≥ 145.67 ⟹ **powered=True**。
- μ̂=259.64>0 ∧ LCB=183.95>0 ∧ perm_p<perm_α ⟹ 四条件齐 ⟹ **VALIDATED 存活**。

即：n_eff 缩水 90.7% 后该桶仍过功效门，因全窗 n 足够大。codex 的降级推理对 32k 样本成立，对全窗不成立——照实报告，不为迎合预期而人为截窗。

## 多重检验 family 注记（codex 缺口②）

- **perm_p 精度**：VALIDATED 桶 perm_p 报 `0/200`（0 次置换超观测，非真 0）。加一平滑 `(0+1)/(200+1)=1/201≈0.004975`——这是 R=200（预注册 §4 冻结）下的**分辨率下限**，真 p 可能远小于此但 R=200 无法进一步分辨（改 R 违反预注册冻结）。
- **family-wise 校正**：本轮 11 桶同测。Bonferroni α/11 = `0.004545`。VALIDATED 桶加一平滑 p=`0.004975 > 0.004545` ⟹ **严格 FWER 控制下临界失守**（差 0.00043，落在 R=200 分辨率噪声内）。
  - 对比 r1 的 5 桶 α/5=0.01：0.004975<0.01 通过。桶数从 5 升到 11（全窗），Bonferroni 更严。
- **结论口径**：`classify_bucket` 施加的是**逐桶未校正 α=0.05**（perm_p<0.05），故代码判 Pass。FWER family 校正是**报告层注记**，未内置于 classify_bucket——按此注记，VALIDATED 桶在 11 桶 Bonferroni + 加一平滑下为**分辨率受限的临界 INCONCLUSIVE**。两口径并列如实，最终裁由 codex 复审/编排者裁定。

## 结果包六要素

1. **结论**：n_eff 自相关校正已落实并生效（neff/nraw=0.093）；全窗重跑后 L0 三类买点(δ+1)桶经校正仍 powered 且 VALIDATED，代码级 verdict=Pass；但 11 桶 Bonferroni+加一平滑下该桶显著性临界失守（0.004975 vs 0.004545）。
2. **定义依据**：预注册 §1.3 `n_eff=事件聚集/自相关校正后有效样本（Σρk，665 neff/nraw）` + §3.1 powered 门 `n_eff≥(1.645·CV)²` + §4 R=200/z_α=1.645 冻结。L0(3,+1)桶 n_eff=158.71≥145.67 满足 powered，LCB=183.95>0 满足 §3.1 VALIDATED 四条件。
3. **边界条件**：结论翻转条件——(a) 若采 FWER family 校正入判据（11 桶 Bonferroni）⟹ 该桶降 INCONCLUSIVE ⟹ 全局 INCONCLUSIVE；(b) 若 R 增至 ≥2000 且仍 0 次超越 ⟹ 加一平滑 p≈0.0005<<α/11 ⟹ FWER 也通过（但改 R 违反预注册冻结）；(c) 若恢复 32k 截窗（n=44）⟹ 回到 codex 预期的 INCONCLUSIVE。
4. **下游推论**：acc-alpha acceptance 判据是"判定完成+codex 审计"，非"必须 VALIDATED"。本轮判定完成、校正落实、待 codex 复审。若 codex 裁定 FWER 应入判据，则 acc-alpha 收口为 INCONCLUSIVE（诚实收口证据）；若裁定逐桶 α 口径，则 Pass。
5. **谱系引用**：665（neff/nraw 先例，本轮 0.093 同族更强聚集）、667（powered 三态判据）、231（有效域≠定义域——本轮消除 r1 的 32k 截窗边界，但引入 R=200 perm 分辨率新边界）、161（照实报告 codex 预期被翻转）。
6. **影响声明**：改动 `rust/src/theta_v0/backtest/decontam.rs`（新增 `effective_n`；`powered`/`classify_bucket` n_eff 形参 usize→f64；5 处测试字面量 +1 f64 化；新增 `effective_n_autocorr_correction` 自检测试）、`rust/src/theta_v0/backtest/wverify_run.rs`（逐桶时间序 series 收集 + n_eff 校正调用 + 表加 n_eff 列）。cargo test --release --lib theta_v0::backtest::decontam 全绿（6/6），wverify_full 全绿（156s）。不做 git 操作（Lead commit）。
