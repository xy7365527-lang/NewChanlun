# exit μ 分桶语义 codex 终局裁定 + 冻结条款（Task #180）

**工位**：ws-exitbkt | **日期**：2026-07-03 | **裁定源**：codex exec（gpt-5.5，--skip-git-repo-check --sandbox read-only）
**认识论等级**：L0（因果可测性 + 样本量代数论证，不依赖跑数——本任务跑数后置，归终局 prereg）
**上游**：A7 #165（commit `4eba6d8fb9`，`TypedTrade.exit_z` 账本列已落地，μ 消费侧待裁）

---

## 1. 结论

**裁定：第四形态 (d)——出场信息不进 μ 桶键，也不进 selector 可查询协变量；只通过 `X^full` 进样本收益值。**

codex 裁定 (d)，独立复核成立。四候选中 (a)/(b) 违反因果可测性直接否决，(c) 被收窄为 (d)。

### 冻结条款（供终局 prereg 直接引用）

> **exit-μ-BUCKETING-FROZEN**（口径冻结，任何重启 alpha 检验自动继承）：
> μ 的生产估计键保持 `Z = z_entry`（`build_mu_from_bars` 现口径 `est.observe(MuObservation{class: t.entry_z, x_gamma})` 不变）。
> `typed exit` / `exit_type` / `exit_z` / 账本态迁移向量 `(exit_z − entry_z)`：
> - **允许**：只通过 `X^full = δ(P_{τ^typed} − P_t) − C` 进入样本收益值 `x_gamma`（§9 计价时刻）；事后诊断 / 异质性报告（W-VERIFY 按 exit_type 拆解占比）。
> - **禁止**：进入 μ 桶键（MuClass key）；作为 `χ_t(z,a) = 1[LCB_t(μ(z,a)) > θ ∧ …]` 门控的可查询协变量。
> 依据：出场信息在入场决策点 t 不可观测（post-treatment），进桶键 = 未来信息泄漏 + 样本碎裂（657）。

## 2. 定义依据（PDF 原文 ↔ 裁定映射）

| PDF 位置 | 原文 | 裁定落点 |
|---------|------|---------|
| **§9 正规出场层** | `X^full_i = δ_i(P_{τ^typed_i} − P_t_i) − C_i`，「收益函数必须使用正规出场」，`τ^typed ≠ τ^reverse` | typed exit 是 **X 的计价时刻**（决定 `P_{τ^typed}`），不是分桶键一维——出场信息**已通过 τ^typed 吸收进 x_gamma** |
| **§12 alpha 选择器层** | `μ(z,a) = E[X(a) \| Z=z]`；`χ_t(z,a) = 1[LCB_t(μ(z,a)) > θ ∧ RiskOK ∧ ConflictOK]` | Z=z 是**入场决策点 t 状态**；χ_t 须在 t 可查 μ ⟹ z_exit（τ 才知）进桶键使 μ 查询在 t 不可计算 = 因果失败 |
| **§12 明文** | 「结构完整不等于有 alpha」「奇偶交替是持有窗口方向恒等式，不是独立可交易信号」 | 细分桶键不产生 alpha，只碎样本（657 同源） |
| **§15 P4/P7** | P4「六类买卖点类型进入状态键」；P7「正规出场进入回测」 | P4 = 入场 z 键（i_class 已在 MuClass 承载）；P7 = 出场进 X 计价（**非进 z 键**）——两者分层，A7 的 exit_z 落在 P7 侧 |

**核对结论**：(d) 与 §9（出场进 X）+ §12（Z 是入场态、χ_t 因果可测）+ §15（P4 入场键 / P7 出场计价分层）逐条一致。§9 的 `X^full` 公式本身即证明「出场信息已在 x_gamma」——把 exit_z 再进桶键是对同一出场机制的二次条件化（outcome variable 冒充 state variable）。

## 3. 样本量可行性论证（n≈2209，BTC 单标的 typed-exit 口径）

| 候选 | 每非空桶期望样本 | 可行性 |
|------|----------------|--------|
| 现口径 (z_entry) | 2209 / K_entry（15 维已高度稀疏，nest_depth 95% 退化 base-case） | 现状（勉强） |
| (b) exit_type 5 层 | 2209 / (5·K_entry)；乐观 K_entry=15 时 ≈29.5/桶，真实 15 维组合更低 | 边际不可行 |
| (a) 三元组 | 2209 / (5·K_entry·K_exit)；乐观 K=15 时 ≈2.0/桶，真实基本归零 | **数值不可行** |
| (c) 账本三元交互 | 2209 / (27..64·K_entry)；乐观 ≈2.3–5.5/桶 | 不可作 alpha 确认层 |

样本量证据独立支撑因果可测性否决：即使 (a)/(b) 无因果问题，n≈2209 在现有 15 维稀疏度下也不支持再乘 5×–75× 桶数。

## 4. 边界条件（裁定翻转条件）

1. **若样本量数量级提升**（跨标的池化到 n≈数万，且解决 657「原始-$拼接=尺度伪影」的归一化预注册）→ (b) exit_type 分层可能数值可行，但**因果可测性否决仍成立**——除非把 exit_type 替换为**入场时可预测的 ex-ante exit regime 代理**（届时它就不是 realized exit_type，而是一个新的入场维，须新 prereg，不属本裁定）。
2. **若 (c) 严格限定为诊断特征**（不进 selector、不参与 `LCB_t(μ)` 查询）→ (c) 可接受为 W-VERIFY 报告层，(d) 已包含此许可（诊断/异质性报告允许）。裁 (d) 而非 (c) 是因为 (c) 的表述留了「作 μ 估计协变量」的门控泄漏口子。
3. **若未来定义修改**使 μ 查询点从入场决策点 t 移到出场点 τ（即改 §12 的 Z 语义）→ 因果论证翻转。但这会改变 χ_t 的存在论（择时选择器→事后归因器），属定义冲突须 escalate，非本裁定域。

## 5. 下游推论

- **μ 估计器**：`build_mu_from_bars` 消费侧**保持不变**（仍 `entry_z` 单键）。A7 的 `exit_z` 账本列定位为**诊断/完备性载体**（与 A9 `position_node_id`「账本层有、μ 层不消费」同构确认）。
- **TYPED_TRADE_SCHEMA_VERSION**：本裁定确认 `exit_z` 增列**不触发 μ 样本 schema 重冻结**（`MuObservation = {class, x_gamma}` 不变）——A7 §5 联动声明成立。
- **W-VERIFY**：exit_type 占比（CloseRoot/ReduceCore/CloseShortDiff/RiskExit/Hold 分布）作诊断输出，判「no alpha」来自真稀有还是出场路径语义——诊断层，不进门控。
- **B30 冻结顺序**（与四项对齐，本裁定先冻结）：
  1. **先冻结本裁定**（exit-μ-BUCKETING-FROZEN）：μ 键只许 entry_z。
  2. B30-①：B̂ᵢ 残差减法 + `h = exit_bar − entry_bar` / time-block 分层——`h` 是**检验分层/稳健性维**（出场信息的合法投影），**不是 χ_t 可查键**。与本裁定**不冲突**（分层 ⊥ 桶键：分层是事后稳健性检验的切片，桶键是入场时门控的查询维）。
  3. B30-②：selector LCB 门控只查询 `LCB_t(μ(entry_z, a))`。
  4. B30-③：方向不对称回归 `μ_sell − μ_buy`（block bootstrap / cluster SE）。
  5. B30-④：shrinkage 层级收缩——只收缩 entry-side 结构桶，不引入未来出场态。

## 6. 谱系引用

- **657 号（已结算）**：分类维进桶键 → pooled 符号翻转 + 样本碎裂；方向/路径维进桶键 → δ 置换自毁。本裁定是 657 在「出场维」的直接应用——exit_z/exit_type 是 path/outcome 维，进桶键触发 657 全部病理。
- **q4 INCONCLUSIVE + i_class×δ 共线（MEMORY project_iclass_delta_collinearity_perm_degeneracy）**：方向性维进桶键即毁 δ 置换的实证先例，本裁定预防同类退化在出场维复现。
- **形式化有效域 231 号**：本裁定 L0（因果可测性 + 样本量代数），非 alpha 声明。「出场信息已在 x_gamma」是 §9 恒等式的定义级推论（L0），不需跑数。
- **A9 #166 先例**：账本层增载体、μ 层不消费的模式，A7 exit_z 同构复用，本裁定确认之。

## 7. 影响声明

- **改动**：仅新增本裁定文档（冻结条款供终局 prereg 引用）。**无代码改动**——裁定结论 = 维持现状（`build_mu_from_bars` 保持 entry_z 单键）。
- **未改**：`mu_estimator.rs`（MuObservation 不变）、`l3_delta_r_alpha.rs`（build_mu 消费侧不变）、`runner.rs`（exit_z 账本列定位为诊断载体，A7 已落地）。
- **冻结效力**：exit-μ-BUCKETING-FROZEN 供任何 alpha 重启 prereg 继承，先于 B30 四项冻结。
- **codex 交互全文**：完整 prompt + response 见本次调用（codex exec 单次，未走 newchan.codex CLI 持久化路径——newchan 模块未安装，交互记录内联本文档 §1-§5 裁定摘录 + codex 原文裁定「选 d」+ 四点理由 + 样本量估算 + B30 五步顺序，逐条已并入上表）。
