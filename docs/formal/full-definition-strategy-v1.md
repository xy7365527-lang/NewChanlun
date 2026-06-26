# 全定义策略 v1 — 单一闭环混合状态机 S_Θ 装配蓝图

> **状态**：生成态蓝图（codex 编排者代理裁决 R1-R4，2026-06-26）。这是 Lean 装配的**设计契约**，不是证明。
> **来源**：GPT/codex 结果包（strict_hybrid_state_machine_strategy.md）整合 spec + 本蜂群 formal/Strict + Tlayers 60 个已证零件。
> **主线裁决（codex R1=B）**：把现有 60 零件装配成 `S_Θ + hybridStep + T` 单一闭环状态机，与 #84 引擎并行；#84 只填 Rec/Class 槽位，不改总结构。

---

## 0. 为什么需要这份蓝图（问题陈述）

本蜂群已证 **60 个零件**（formal/Strict 12 文件 + Tlayers T₁-T₅₉），但它们**从未装配成闭环**：

| 现状（散件） | 缺口 |
|---|---|
| `StrategyFamily.piTheta` 抽象 4 段产出**订单就停** | 无 T，**不读 C_Θ** |
| `Chain.chanlun_chain_total_unique` 把 C_Θ 与 π_Θ **并列合取 `⟨_,_⟩`** | 无连接、无闭环、无单一 x_t |
| `Dynamics.δ` 状态转移 | 仅 4 字段微状态（无账本/声部/风险/订单） |
| `Tlayers/Operational/Accounting/Signal/Spiral` T₁-T₅₉ | 各自独立 conditional 定理，未装进 π_Θ |

GPT 包的全定义策略**已装配**：单一闭合 `S_Θ` + 完整态 x_t + `x_{t+1}=T(x_t,π(x_t),e)` 闭环 + 单一 `hybrid_step_complete_unique`。**装配维度上 GPT 领先我们一整层。** 本蓝图定义如何把我们的零件接成同等闭环——并因保留 60 零件细节 + 诚实标注而**严格强于 GPT**。

---

## 1. 单一总式 S_Θ（编排者公式 + 我们的组件映射）

```text
S_Θ = (X_Θ, E_Θ, Rec_Θ, C_Θ, Intent_Θ, K_Θ, J_Θ, Schedule_Θ, T_Θ)
```

```text
hybridStep_Θ : HybridState → Event → HybridState          -- 单一闭环一步
x_{t+1} = T_Θ(x_t, Schedule_Θ(x_t, u*_t), e_{t+1})         -- 闭环写回
u*_t   = RiskProj_Θ(x_t, Intent_Θ(C_Θ(x_t)))              -- π̄ 真读 C_Θ
```

**与 Chain.lean 的关键差异（codex R2 硬约束）**：
```text
现在 (Chain):   ⟨ C_Θ(x) 唯一 ,  π_Θ(h,z) 唯一 ⟩    ← 并列，π 不读 C，无 T
装配后:          x ─e→ C_Θ(x) ─Intent→ ũ ─RiskProj→ u* ─Schedule→ O ─T→ x'
                 即 π_Θ = π̄_Θ ∘ C_Θ（真读 C_Θ）；T 真写回完整 x_t
```

---

## 2. 完整态 HybridState x_t（乘积态，字段清单）

GPT 完整态：`x_t = (h_t, D_t, σ_{r,t}, (q_v,a_v,ω_v)_v, Φ_t, I_0, Π_t, A_t, W_t, R_t, E_t, Cash_t, O_t, M_t, ν_t)`。

我们的乘积态（codex R2 字段，**复用已有层状态**，标来源）：

| 字段 | 类型/语义 | 我们的来源组件 | 等级 |
|---|---|---|---|
| `hist` | 事件历史 h_t | Causal/Dynamics 事件流 | L0 |
| `microState` | 解析级微状态（bar/stroke/pending） | `Dynamics.State` | L0 |
| `globalState` | C_Θ 输出 = (LevelStates, NestCerts, FugueCerts, RiskMode) | `Chain.GlobalState` | L0 |
| `ledgerState` | (I₀,Π,A,W,R,E,Cash) 账本恒等 R=Π-A-W | `Tlayers/Accounting/Ledger` | L0 结构 / L2 数值 |
| `voiceState` | (σ_r, (q_v,a_v,ω_v)_v) 声部仓位/激活/订单态 | `Fugue.VoiceTree` + 扩展 | L0 |
| `riskMode` | μ ∈ {Insolvent,Liquidation,Deleverage,CloseOnly,Normal} | `Tlayers/Operational` 风险模式 | L0 结构 / L2 阈值 |
| `phase` | Φ ∈ {I,II,III_repair,III_protected,III_accretive} | `Tlayers/Operational` 三阶段(T₂₆) | L0 结构 / L2 阈值 |
| `positions` | 各声部绝对单位数 q_v | `Fugue` + RiskProj | L0 |
| `orders` | 未完成订单 O_t | `StrategyFamily.exec` 输出扩展 | L0 |
| `memory` | M_t 信号使用记录（Fresh） | `Tlayers/Signal` located 记忆 | L0 |
| `metrics` | 连续充分统计量 r_Θ(P,E,G,N,IM,MM,中枢边界…) | 各层连续量 | **L2/L3 数值** |

**合法态不变量**（T 必须保持，codex R3）：
- `R_t = Π_t - A_t - W_t`（账本恒等，Accounting 已证）。
- `σ_v = -σ_{p(v)}`（赋格交替，Fugue `alternating` 已证）。
- `a_v ≤ a_{p(v)}`（祖先闭合，Fugue `ancestor_closed` 已证）。

---

## 3. 六段 step 顺序（每段 IO + 组件 adapter + 等级）

```text
x_t ─e→ [1]Rec ─→ [2]C_Θ ─→ [3]Intent ─→ [4]RiskProj ─→ [5]Schedule ─→ [6]T ─→ x_{t+1}
```

| # | 段 | 输入 → 输出 | adapter（我们的组件证字段义务） | 等级 / 诚实边界 |
|---|---|---|---|---|
| 1 | **Rec** | (microState, e) → D_t 递归结构 | `Parse.lean` + `LevelState.lean`（Θ_parse/Θ_level 参数化） | L0；Θ_parse 是 R（617），#84 引擎补强此槽 |
| 2 | **C_Θ** | (D_t, ledger, voice…) → globalState | `Chain.globalClassify`（fiber partition） | L0 结构事实；**不证标签缠论语义**（逐 claim 承载，615） |
| 3 | **Intent** | C_Θ(x_t) → ũ 原始意图 | `Fugue` 声部证书 + **10 级动作优先级确定选择器** | L0；**优先级是确定选择器，非缠论唯一推出**（codex R3） |
| 4 | **RiskProj** | (x_t, ũ) → u* 唯一控制 | `RiskProj.lean` 确定选择器 / **有限网格 RiskGrid** | L0 确定选择；**禁声明连续 argmin 存在性**（codex R3，已守 StrategyFamily §3） |
| 5 | **Schedule** | (x_t, u*) → O_t 订单 | `StrategyFamily.exec` + 执行顺序（撤单→先平后开→后代先平→…） | L0；执行顺序是 Θ_exec 设计选择 |
| 6 | **T** | (x_t, O_t, e_{t+1}) → x_{t+1} | **新建** `T:HybridState→Order→Event→HybridState`，δ 只更新 microState 分量；**ledger/accounting 更新放进 T** | L0 闭环全定义；**不声明盈利/最优/实盘**（codex R3） |

**codex R2 纠正**：T **不是** `Dynamics.δ` 提升——T 是新建的完整闭环转移，`Dynamics.delta` 仅作其 microState 分量更新器。**账户因果洞**（`piTheta_causal` 只比同一 z）由「ledger 更新放进 T」补上——T 写回 z 的下一态，使 z 的生成因果进入闭环。

---

## 4. 单一总定理（装配义务）

```text
hybrid_step_complete_unique :
  ∀ (x : HybridState) (e : Event),
    ∃! x', hybridStep_Θ x e = x'
```

**证明方式（codex R2）**：函数图唯一 + 各分量 `total_unique` 组合（复用 `Chain.totalUnique_prod` 乘积内核）。实质义务挂到各组件字段——`HybridComponents` 结构的每个字段 `*_total_unique` 由对应 adapter（Parse/Level/Nest/Fugue/RiskProj/Accounting/Dynamics）承载。

**诚实标注（formalization-validity-domain）**：此定理证「给定 Θ ⟹ 闭环每步全定义且唯一」，与 GPT 同为 **L0 在假设下**。**不证**：盈利/最优（L3）、标签缠论语义（逐 claim）、缠论无参数唯一策略（前件必含 Θ，616/617）。

---

## 5. 四级证书阶梯接入（codex 上轮 D1-D6，本蓝图遵守）

全局 `C_Θ` 定级 **L0**（Θ-参数化 fiber partition）。L1/L2/L3 是**可选证书**，按需挂在闭环上：

- **L1 决策充分性** `C^dec_Θ(x)=C^dec_Θ(y) ⟹ π(x)=π(y)`：装配后 π=π̄∘C 使其**可表达**——在 `C^dec_Θ:=(globalState, ledger, risk/exec stats)` 上证（D2）。
- **L2 动态同余** `C(T x O e)=T̄(C x, ē)`：从 `Causal.step_sound`（3 态机）提升到 `globalState` 闭包骨架，T̄ 作用于含连续分量的 product state，**不要求有限态**（D4）。
- **L3 行为等价最小** `C(x)=C(y)⟺x≡_beh y`：**默认证否定结果**（连续风险量 ⟹ 离散标签下 false），保留局部有限网格版（D3）。

---

## 6. 装配后 vs GPT（codex R3 诚实账）

**我们装配后严格强于 GPT 的部分**：同等闭环 S_Θ + **60 个具体 Lean 零件** + 否定结果（∼ₙ 多义/peekAt 非因果）+ 有限网格 RiskProj 实现 + 会计层结构定理 + 诚实有效域标签 + 具体缠论语义（GPT 几乎零具体缠论）。

**装配时必须保持诚实（不因装配而声明膨胀）**：
- RiskProj 只能确定选择器 / 有限网格 `RiskGrid`——**连续 argmin 不许声明**。
- 成本/止损/仓位上限/权重/网格分辨率 = **Θ_risk，非缠论推出**。
- π_Θ **给定 Θ 后唯一，非缠论无参数唯一策略**。
- T **只声明闭环状态转移全定义**，不声明盈利/最优/实盘有效。
- 10 级动作优先级 = **确定优先级选择器**，非缠论唯一推出。

---

## 7. 结果包六要素

1. **结论**：定义全定义策略 v1 装配蓝图——把 60 个散件接成单一闭环 `S_Θ + hybridStep + T`，吸收 GPT 整合优势，因保留细节+诚实而严格强于 GPT。
2. **定义依据**：GPT strict_hybrid_state_machine_strategy.md §1-§22（S_Θ/x_t/T/10级优先级/三阶段账本）+ 本蜂群 Chain.GlobalState/StrategyFamily.piTheta/Fugue.VoiceTree/Dynamics.delta/Accounting.Ledger/RiskProj 已证组件。
3. **边界条件**：若某分量装入闭环时被迫冒充（如 RiskProj 声明连续 argmin、T 声明盈利、优先级声明缠论唯一），则该装配判为声明膨胀（090），必须降级为确定选择器/有效域标注；若 microState 乘积态无法封闭（某事件下 T 无定义），则是真矛盾走 /escalate。
4. **下游推论**：`Strict/HybridStep.lean` 接口定住后，#84 segment 引擎只填 Rec/Class 槽位（不改总结构）；L2 回测获得形式落点（闭环 T = 实时/回测一致性的形式契约）；615/616/617 谱系获装配延伸（C_Θ 与 π_Θ 由并列升级为闭环复合）。
5. **谱系引用**：231（有效域，RiskProj/T/优先级标 R 不冒充 L0）+ 090（禁声明膨胀）+ 615/616/617（C 与 π 都 Θ-参数化，闭环不改此结论）+ 待结晶新谱系（四级证书阶梯 + 全定义策略装配）。
6. **影响声明**：本蓝图不改任何 Lean，定义后续装配的设计契约。影响：新建 `Strict/HybridStep.lean`（接口）+ 各层 adapter；重定义 Phase 2「完成」标准（从「各组件并列 total_unique」升级为「单一闭环 hybridStep」）；#84 降为闭环的 Rec/Class 槽位补强。
