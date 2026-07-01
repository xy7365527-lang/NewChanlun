# 多级别买卖点信号分布（BTC 全历史 461万 bar）

**任务**：Task #110 acc-multilevel-sample。纯统计，读 663 台账 `/tmp/btc_663_ledger_full.csv`（12626 行），不碰引擎。
**认识论等级**：L2（BTC 单标的真实数据，非交叉验证）。

## 结论

### 级别分布表

| level | n_buy (δ+1) | n_sell (δ−1) | n_total | mean(actual_pnl) | Σ(actual_pnl) |
|-------|-------------|--------------|---------|------------------|---------------|
| 0 | 5829 | 5320 | 11149 | 26.69 | 297520.20 |
| 1 | 526 | 533 | 1059 | 24.26 | 25689.13 |
| 2 | 166 | 158 | 324 | 23.59 | 7643.61 |
| 3 | 36 | 33 | 69 | −119.06 | −8215.38 |
| 4 | 9 | 12 | 21 | −51.36 | −1078.66 |
| 5 | 1 | 3 | 4 | −36.98 | −147.93 |

样本量随级别几何衰减（约 ×0.1/级）：0→11149，1→1059，2→324，3→69，4→21，5→4。

### 样本充足性判定（阈值 ≥30 可 OOS 验）

| level | n_total | ≥30? | 判定 |
|-------|---------|------|------|
| 2 | 324 | ✅ | 充足 |
| 3 | 69 | ✅ | 充足（**解锁 acc-highlevel-mu**）|
| 4 | 21 | ❌ | 结构性稀疏 |
| 5 | 4 | ❌ | 结构性稀疏 |

**level3 = 69 ≥ 30 → 解锁高级别 μ̂ OOS 验（acc-highlevel-mu）。**

level4/5 样本不足（21/4）：按 663 口径（ChatGPT §11），级别照报，**不判 Le Cam 硬墙**，标注「样本不足，inconclusive ≠ 无 alpha」——稀疏是级别几何衰减的结构属性，不是 alpha 缺失的证据。

### 高级别收益方向（含 beta 警告）

mean/Σ(actual_pnl) 在 level0-2 为正（+26.7/+24.3/+23.6），level3+ 转负（−119/−51/−37）。

**警告**：actual_pnl = X_i 含 beta（见 [[project_oddeven_mu_identity]] μ̂ vs X_i 的 beta 混入问题）。此处方向不可直接读作级别 alpha——level3+ 负 mean 可能是 (a) 高级别买卖点真反向，或 (b) 样本稀疏 + beta 漂移伪结构。区分需 acc-highlevel-mu 的反事实 perm 检验，非本任务范围。

## 定义依据

台账 δ 列取值 {−1,+1}（无 0）；δ=+1 记为买点、δ=−1 记为卖点（任务 §1 口径）。level 列 0-5 为递归级别归属（[[project_recursive_level_emergence]] 涌现级别）。样本充足阈值 30 引自任务约束「≥30-50 可 OOS 验的最小样本」。

## 边界条件

- 若阈值上调到 50：level3(69) 仍充足，判定不翻转；level2(324) 充足。仅在阈值 >69 时 level3 才转入稀疏。
- 若 663 台账口径变更（如 close 口径 vs 触线口径改 actual_pnl 定义），收益方向表全部翻转，但 n_buy/n_sell/n_total 分布不变（δ 不依赖 pnl 口径）。
- 若换标的（非 BTC），几何衰减比率可能不同 → 需 L3 交叉验证才能声明「级别稀疏是普适结构属性」。当前仅 BTC 单标的 = L2。

## 下游推论

- **解锁 acc-highlevel-mu**：level3 样本充足，可对 level3+ 买卖点做 μ̂ OOS 反事实 perm 验（区分真 alpha vs beta 伪结构，[[project_oddeven_mu_identity]] 方法论）。
- level4/5 不可 OOS 验，任何基于其收益的判断都是 inconclusive，不得作为决策依据。
- 若 acc-highlevel-mu 显示 level3 μ̂ perm_p 高（如 >0.5），则与 level0-2 正收益共同指向：alpha 集中在低级别，高级别信号是级别截断伪影（[[project_regime_is_level_truncation_artifact]]）。

## 谱系引用

- [[project_oddeven_mu_identity]]：μ̂ 与 X_i 的 beta 混入——本报告收益方向的 beta 警告依据。
- [[project_recursive_level_emergence]]：递归级别涌现，level 列语义来源。
- [[project_regime_is_level_truncation_artifact]]：高级别稀疏与截断伪影的潜在关联（待 acc-highlevel-mu 确认）。

## 影响声明

- 新增文件：本报告。未改动任何代码/引擎/定义。
- 解锁下游工位 acc-highlevel-mu（level3 样本充足）。
- 未改动 663 台账，未改动 acceptance 其他项。
