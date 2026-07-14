# acc-alpha 预注册 estimand 冻结 v0（beta 去污 alpha acceptance）

- **工位**：swarm/ws-acc-alpha | goal g-20260701T200047Z-8f4f50e7 | acceptance acc-alpha-beta-decontaminated
- **冻结时刻**：2026-07-01（**看任何本轮 Π_full L2 回测结果之前**——防数据挖掘）
- **状态**：预注册草案，供 codex 预审 + 编排者 /ritual。依赖（acc-classification 完整分类 + acc-P2 MACD→feature）齐备后由 Lead 通知启动正式回测。
- **认识论**：本文件 **L0/L1**（纯规格冻结，零 L2 信息增量）。正式回测产出为 **L2**（BTC 单标的）/ **L3**（跨标的），逐桶如实标注。

> ★ 本冻结**修正了 task 下发判据**中的一处与既有谱系（667/665/231）矛盾的口径（见 §6）。修正是**定理类**（从 667 的 `LCB>0 ⟺ n_eff>(1.645·CV)²` + 231 否定膨胀禁止逻辑必然推出），已在 §3 决策规则落地，并 TaskCreate 上报 Lead 备案。不是价值选择，不 escalate。

---

## 1. 冻结的 estimand（分类格 + 切分 + 估计量 + 判据口径）

### 1.1 分类格（bucket key，先验给定）

估计对象为逐桶条件期望 μ(z)，桶键：

```
z = (ℓ, δ, bsp_class)
  ℓ         ∈ {L0, L1, L2, L3, L4, L5}     缠论级别
  δ         ∈ {+1(买/多腿), −1(卖/空腿)}    交易方向
  bsp_class ∈ {1, 2, 3}                     一/二/三类买卖点
```

**关键（665 方向不对称免疫）**：`(ℓ, δ, bsp_class)` 是缠论**先验给定**的桶键，**非从收益挑出**。按结构先验定桶键不引入选择偏差；从收益挑桶键会。这是命题①（除偏差）的结构支撑——但**先验桶键 ⊬ 该桶有 alpha**（665 命题②逻辑独立）。

### 1.2 样本切分（引用已冻结协议，不重造）

切分**不新造**——复用 `docs/backtest-protocol-v0.md` §2 + `rust/src/theta_v0/backtest/prereg_windows.rs`（8 品种 IS/OOS/Holdout + walk-forward 窗，冻结于 2026-06-26）：

- μ̂ 估计与 beta 去污在 **OOS**（`2023-01-01…2025-06-30` 核心/扩展池）上做。
- **Holdout**（OOS 末之后）保留为最终一次性确认，**任何本轮探索不得触碰**（protocol §2.3 防漂移）。
- 主判据用非 clipped walk-forward 窗；clipped/OKLO 特例报告但不进主判据（裁定 W1/OK1）。

### 1.3 估计量与 LCB 口径（引用已实装，不重造）

- μ̂(z) = 桶内逐信号 `actual_pnl` 均值（`MuEstimator`，Welford 并存方差）。`actual_pnl = δ·(P_out − P_in) − C`（663 恒等式）。
- **LCB 口径**：`mu_lcb(z) = μ̂ − z_α·std/√n`，`z_α = 1.645`（单边 95% 下界，`MuEstimator::mu_lcb` 已实装）。`n<2 ⟹ None`（不冒充 LCB=mean）。
- **UCB 口径（本冻结新增要求）**：`mu_ucb(z) = μ̂ + z_α·std/√n`。用于区分「证伪」与「inconclusive」（667，见 §3）——`mu_estimator` 当前只实装 LCB，UCB 为对称量，正式回测前须补 `mu_ucb`（同 z_α）。
- **CV / 功效量**：`CV(z) = std / |μ̂|`；`n_eff(z)` = 事件聚集校正后有效样本数（`Σρk` 自相关校正，665 neff/nraw）。

---

## 2. beta 去污方法（★冻结——单标的直接扣净 B_market 被禁）

**禁用（665 单标的退化定理）**：单标的 BTC 上 `raw ≡ B_market`（`corr=1.0`，`mean(raw−B)=−3.6e−14`）——扣同期市场 beta 后残差**恒为 −C**（纯成本零 alpha）。「扣 BTC beta（market-neutral 减法）」在单标的**有效域退化**（减数≡被减数），**不是**合法的 beta 去污。

冻结两条**互斥可执行**的去污路径，按数据可得性择一并如实标注等级：

| 路径 | 方法 | beta 如何被剥离 | 等级 |
|------|------|----------------|------|
| **A（L2 主路，单标的 BTC）** | **分层内 δ 置换（逐桶）**（stratified permutation）：对每个桶 `z=(ℓ,δ,bsp_class)` 内固定 entry/持有期/成本，只 shuffle δ 标签，200 次固定种子 Fisher-Yates（666 方法1）。逐桶统计量 `S_z=μ̂(z)`，逐桶置换 p 值 `perm_p(z)` = #{S_z_perm≥S_z_obs}/200。 | beta 被分层**吸收**（同桶同持有窗共享 beta，置换只动方向标签）——绕开显式减法退化（665：分层内置换 beta 被分层吸收，不需分离） | **L2**（BTC 单标的，可否证但不交叉验证） |
| **B（L3 主路，跨标的）** | 8 品种独立市场因子 `B_i`（各品种自身同期收益），`Y_i = δ·(raw_i − B_i) − C`，跨品种池化。 | `raw_i ≠ B_i`（品种≠BTC），减法非退化 | **L3**（跨标的，真去污） |

**冻结裁定**：本轮先跑路径 A（依赖只需 BTC 单标的 Π_full 产出）。路径 A 的 `perm_p<0.05` 是**桶内方向信息超随机**的必要条件，**非充分**——须叠加 §3 判据链。路径 B 待 L3 多标的数据齐备后补（667 pending①⑤）。**不用单标的直接扣 B_market**（退化）。

### 【订正】§2.2 路径 A 统计量口径（2026-07-02）

原文本第 52 行表格中写「统计量 `S=Σ_bucket|μ̂|`，`perm_p` = #{S_perm≥S_obs}/200」，暗示全局 omnibus 置换检验（跨所有 36 桶的全局统计量）。实跑产出（wverify_run.rs:46-60）采用**逐桶（per-bucket）独立置换**——每桶 `z=(ℓ,δ,bsp_class)` 独立计算 μ̂(z) 和 perm_p(z)，非全局汇聚。本订正将表述改为「逐桶统计量 `S_z=μ̂(z)`，逐桶置换 p 值」以准确反映实装口径。这与 task #51 的 σ^H 维度扩展和 walk-forward LCB 逐窗估计保持一致（都是**逐桶/逐窗的局部统计推断**，非全局 omnibus）。依据：impl-gap-inventory-20260702.md G-A6、wverify_run.rs 实装检查、090 号诚实原则（声明须与实装一致）。

---

## 3. ★决策规则（LCB>0 占比 + 667 功效门控，三态非二态）

task 下发判据：「holdout μ̂ 扣 BTC beta 后 LCB>0 占比；>0 → 有超 beta alpha；=0 → 纯 beta 照实 161」。本冻结**保留 >0 分支**，**修正 =0 分支**（667）：`LCB≤0` 有两种，不可都判纯 beta。

### 3.1 逐桶三态判定

对每个桶 `z=(ℓ,δ,bsp_class)`，先算功效门槛：

```
powered(z) := n_eff(z) ≥ (1.645 · CV(z))²        # 667 功效门槛
                                                  # CV=10-20 ⟹ 门槛 n≈271-1083
```

再判三态（**非二态**）：

| 桶态 | 条件 | 含义 |
|------|------|------|
| **VALIDATED（超 beta alpha）** | `μ̂>0` ∧ `perm_p<0.05`（去污）∧ `LCB_OOS>0` ∧ `powered` | 桶携超 beta 可交易 alpha |
| **FALSIFIED（纯 beta / 无 alpha）** | `powered` ∧ `LCB_OOS≤0` ∧ **`UCB_OOS≤0`** | 有功效地判定 μ≤0——真纯 beta，照实 161 |
| **INCONCLUSIVE（未认证）** | `¬powered`（`n_eff<门槛`）**或**（`LCB≤0<UCB`） | 判据无检出力——`LCB≤0 ⊬ μ≤0`（667）。**不判纯 beta，不 161** |

**核心（667/231）**：`¬powered` 时 `LCB≤0` 是**判据无检出力的必然结果**，`⊬ μ≤0`。把 underpowered 的 `LCB≤0` 当纯 beta = 231 否定膨胀 + 090 声明膨胀。INCONCLUSIVE 须走 §3.3 提功效重估，**不落 161**。

### 3.2 全局裁决

```
VALIDATED 占比 > 0                              → 存在超 beta 可交易 alpha（acceptance PASS）
全部桶 ∈ FALSIFIED（powered, UCB≤0）            → 纯 beta，照实 161（acceptance FALSIFIED）
存在 INCONCLUSIVE 桶且无 VALIDATED             → acceptance INCONCLUSIVE（不 PASS 不 161，走 §3.3）
```

**「VALIDATED 占比=0」单独不足以判纯 beta**——必须**所有**桶都是 powered-FALSIFIED（含 UCB≤0）才判 161。任一 INCONCLUSIVE 桶存在 ⟹ 全局 INCONCLUSIVE（诚实第三态，661/667）。

### 3.3 INCONCLUSIVE 的提功效重估（冻结顺序，667 pending）

1. **收益率/夏普尺度**：judged 量从 raw-pnl 改收益率或固定 horizon 归一化 pnl——降 CV ⟹ 提功效（667 pending④），部分桶可能翻正或翻真否证。
2. **L0 聚合功效**：L0 `n≈1085-2259` 可能过门槛——若 `powered` ∧ `LCB<0` ∧ `UCB≤0` = L0 真否证（区别 L1/L2 underpowered）。
3. **L3 跨标的**（路径 B）：独立市场因子真去污 + 池化提 n。
4. **多窗 walk-forward**：查单一切分敏感性。

---

## 4. 冻结常量表（看结果前锁定）

| 常量 | 值 | 来源 |
|------|-----|------|
| `z_α`（LCB/UCB 单边） | `1.645`（95%） | `mu_lcb` 已实装 |
| 置换次数 | `200`，固定种子 Fisher-Yates | 666 方法1 |
| 功效门槛 | `n_eff ≥ (1.645·CV)²` | 667 codex-lq-audit |
| 分类格 | `(ℓ,δ,bsp_class)`，6×2×3=36 桶（先验给定） | 本冻结 §1.1 |
| 估计窗 | OOS `2023-01-01…2025-06-30`；Holdout 隔离 | protocol-v0 §2 |
| beta 去污 | 路径 A 分层内 δ 置换（L2）/ 路径 B 跨标的独立因子（L3）；**禁单标的扣 B_market** | 665/666 |
| 主判据 | VALIDATED 占比（三态，§3） | 663+667 |

---

## 5. 结果包六要素

1. **结论**：冻结 acc-alpha-beta-decontaminated 的预注册 estimand——分类格 `(ℓ,δ,bsp_class)`、切分引用 protocol-v0、beta 去污路径 A(分层置换,L2)/B(跨标的,L3)、判据 = VALIDATED 占比的**三态**裁决（VALIDATED/FALSIFIED/INCONCLUSIVE），功效门控 `n_eff≥(1.645·CV)²`。
2. **定义依据**：可交易性判据 = μ(z,a)>0（663，非统计显著性）；桶键先验给定（665 方向不对称免疫）；beta 去污须非退化（665 单标的退化定理）；LCB≤0 的有效域 = 「未能 95% 排除 0」不外推 μ≤0（667+231）。
3. **边界条件（结论翻转）**：(a) 若 codex 预审判定分层内置换仍残留 beta（665 粗吸收）⟹ 路径 A 降为 inconclusive，须路径 B。(b) 若 acc-classification 的 bsp_class 定义未结算 ⟹ 桶键不稳定，冻结暂缓。(c) 若收益率尺度使某桶 powered 且 LCB>0 ⟹ 该桶 INCONCLUSIVE→VALIDATED。(d) 若全部桶 powered-FALSIFIED（UCB≤0）⟹ 纯 beta 161。
4. **下游推论**：acceptance PASS（占比>0）⟹ 存在可交易 alpha，进 M1 里程碑验证；FALSIFIED（全桶 powered UCB≤0）⟹ 缠论买卖点在 BTC 无超 beta edge，照实 161；INCONCLUSIVE ⟹ 触发 §3.3 提功效链，不得声明任何一端。
5. **谱系引用**：663（判据=μ>0 非显著性）/665（除偏差⊬alpha + 单标的 beta 退化）/666（δ 置换 perm_p=0.69 beta 投影）/667（LCB underpowered，inconclusive≠证伪，`n_eff>(1.645·CV)²`）/231（有效域/否定膨胀禁止）。**曾发生判据错误的领域**——本冻结正是为不重蹈 660/663 的判据错误而立。
6. **影响声明**：新增 `.chanlun/review-results/acc-alpha-estimand-prereg-20260701.md`（预注册规格）。要求正式回测前在 `mu_estimator` 补 `mu_ucb`（§1.3）。修正 task 下发判据的 =0 分支（§6）。不改引擎、不产 L2 数字。

---

## 6. ★与 task 下发判据的矛盾（TaskCreate 上报 Lead）

**矛盾**：task 判据「LCB>0 占比=0 → 纯 beta，照实 161」与 667（settled-锚 231）冲突。

- task 隐含：`VALIDATED占比=0 ⟹ μ≤0 ⟹ 纯 beta`。
- 667：`LCB≤0 ⟺ n_eff<(1.645·CV)²` 时是 underpowered，`LCB≤0 ⊬ μ≤0`（inconclusive 非证伪）。L1/L2 OOS `n=38-188 < 门槛 271-1083` ⟹ 结构性无检出力。把 underpowered 的占比=0 当纯 beta = 231 否定膨胀。

**解决（定理类，已在 §3 落地）**：`=0` 分支拆三态——只有**全桶 powered-FALSIFIED（含 UCB≤0）**才判纯 beta 161；存在 INCONCLUSIVE 桶 ⟹ 全局 INCONCLUSIVE，走提功效链。这是从 667+231 逻辑必然推出的判据精确化，非价值选择，故直接修正 + TaskCreate 备案，不 escalate。
