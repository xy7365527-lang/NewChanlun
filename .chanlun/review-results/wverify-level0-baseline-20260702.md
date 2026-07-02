# W-VERIFY acc-alpha level0 域基线（待 #12 定版重跑取代）

- 工位 ws-wverify | 任务#3 | 跑批时 HEAD=cf6a5cdfe4（perm_p 生产者 8ccbe58137 + Lead 提交的 shrunk_view trades 编译修复）
- 数据 BTC 5min 2025-05-08..06-25（1188 笔观测；落 prereg OOS 窗内，末尾 < Holdout 2025-07-01，无泄漏）
- 认识论 L2，数据窗=OOS 48 天子集；**有效域=level0**（见下）

## 基线结论（level0 域）
- trades=1188，27 桶有样本，global_verdict=**Inconclusive**（validated=0 falsified=0 inconclusive=27）
- **全 27 桶 ¬powered**（n_eff < (1.645·CV)²）⟹ perm_p 从未参与判定 ⟹ 既未检出 alpha 也不判纯 beta
- 根因：金融单笔高噪声（大 CV）+ 48 天数据量不足 ⟹ 功效门全不满足
- 桶键值域：ℓ∈{0,1,2,3,4}、bsp_class∈{1,2,3}、δ∈{±1}（无非法桶）

## 有效域声明（231号，强制）
- 有效 alpha 判定域实际=**level0（+零星 level2-5 Type1）**，不覆盖全定义域
- codex H1（#7）确认 N^δ 门把背驰谓词误用到 Type2（retrace_no_break 互斥）⟹ **level1-4 门后归零是 by-construction 门 bug，非数据稀疏**
- global_verdict 不得声称"全级别无/有 alpha"，只对有效域内桶成立
- 待 #12（Cand^δ 修复）解封 level1-4 的 1473 信号后，须在 #12 稳定 SHA 上重跑覆盖全定义域

## 已知待做 delta（#12 稳定 SHA 上一次性重做，交 Lead commit）
1. **strata HashMap→BTreeMap**：perm_test.rs 的 `strata` 现为 HashMap，迭代序依赖运行时 RandomState ⟹ 跨进程 perm_p 不可复现（违反 prereg §4 固定种子）。改 BTreeMap（按 key 确定序）+ 加多层可复现回归测试。**本轮结论不受影响**（全 ¬powered，perm_p 未参与判定），但定版必须修。
2. **wverify_run.rs 跑批入口**：#[ignore] 测试，load_bars_from_cache→oos_window→build_mu_estimator_from_bars→est.trades() 投影 (level,bsp_class(),delta,x)→stratified_delta_perm_p→逐桶 Welford 聚合→classify_bucket→global_verdict。数据路径用 concat!(env!("CARGO_MANIFEST_DIR"),"/../analysis/data_cache/BTC_5min_1000.json")。
3. **cv 单测**：MuEstimator::cv 验证 σ̂/|μ̂| + n<2→None。
4. **全定义域报告** + 231 有效域标注（level1-4 由 by-construction 变为真实检验后更新）。

## P4 消费口径（上游边界，已处理）
μ̂ 分桶用 6-bit bsp_disc 掩码（重合买卖点独立桶，Codex Q4 无损超集）。消费侧 MuClass::bsp_class() 折叠 6-bit→{1,2,3}（取最低类号）满足 prereg 36 桶。实跑 27 桶有样本，9 桶无样本（部分因 level1-4 门 bug）。
