# Codex Review — OQ-9 修复后审查（session 019f0318 后续）

**session_id**: oq9-fix-20260626  
**模式**: review  
**时间**: 2026-06-26  
**model**: 代码层异质审查工位（claude-sonnet-4-6）直接推理  
**背景**: OpenAI API quota 耗尽，外部模型不可用；工位基于完整代码读取执行独立审查。

---

## 审查对象

- `/Users/silencehan/Projects/NewChanlun/formal/Tlayers/Accounting/TotalWealth.lean`（§1.5 OQ-9 gate 修复后版本）
- `/Users/silencehan/Projects/NewChanlun/formal/Strict/HybridAssembly.lean`（§6.5 twState 接入闭环修复后版本）

---

## 问题1：trace 级端B——oq9inv_preserved 各分支

### shortDiff 分支

`simpa only [OQ9Inv, twStep] using hinv`。twStep shortDiff 只动 free/holding，不动 stage/openLegacyLegs。hinv 直接给出后态 inv。**PASS。**

### openShareLeg 分支（关键）

```lean
simp only [LegalTransition] at hlegal  -- hlegal : s.stage.rank < earningShares.rank
intro hst                              -- hst : (twStep s .openShareLeg).stage = earningShares
simp only [twStep] at hst             -- 展开：stage 字段不变，hst : s.stage = earningShares
rw [hst] at hlegal                    -- hlegal : earningShares.rank < earningShares.rank，即 2 < 2
exact absurd hlegal (by decide)       -- 2 < 2 = False，decide 可判定
```

推理链完整。twStep openShareLeg 的定义 `= { s with openLegacyLegs := s.openLegacyLegs + 1 }`，stage 字段是 `s.stage`（不变），simp only [twStep] at hst 把 `(twStep s .openShareLeg).stage` 化简为 `s.stage`，hst 变成 `s.stage = earningShares`，rw 后 hlegal 变成 `2 < 2`，decide 判 False。**PASS。**

### closeShareLeg 分支（vacuous）

```lean
simp only [LegalTransition] at hlegal  -- hlegal : s.openLegacyLegs ≥ 1
intro hst
simp only [twStep] at hst ⊢           -- twStep closeShareLeg：stage 不动
-- hst : s.stage = earningShares（后态 stage = 前态 stage）
have hpre : s.stage = TStage.earningShares := hst
have h0 : s.openLegacyLegs = 0 := hinv hpre
omega                                  -- h0 : = 0 和 hlegal : ≥ 1 矛盾
```

twStep closeShareLeg = `{ s with openLegacyLegs := s.openLegacyLegs - 1, cumNetCash := s.cumNetCash + p }`，stage 不动。注意 Nat 饱和减法：如果 openLegacyLegs = 0 则减后仍是 0，但此处合法性要求 openLegacyLegs ≥ 1，与 hinv 得到的 = 0 矛盾，omega 直接结束。Nat 饱和减法不影响这条推理（矛盾在减之前已产生）。**PASS。**

### recoverCapital 分支

```lean
intro hst
simp only [twStep, advanceTo] at hst
by_cases hse : s.stage.rank ≤ TStage.capitalRecovered.rank
· rw [if_pos hse] at hst; exact absurd hst (by decide)
· rw [if_neg hse] at hst
  exact hinv hst
```

两支：
- hse 真：结果 = capitalRecovered，hst 变成 `capitalRecovered = earningShares`，by decide 判 False（不同构造子）。
- hse 假（rank > 1，即 rank = 2，即 s.stage = earningShares）：结果 = s.stage，hst 化简为 `s.stage = earningShares`，`hinv hst` 给 `s.openLegacyLegs = 0`，twStep 不动 openLegacyLegs，后态 inv 成立。

等效：后态 stage 永远不等于 earningShares（capitalRecovered ≠ earningShares；若前态已 earningShares 则 advanceTo 保持，但此时 hinv 给 0，inv 仍成立）。**PASS。**

### enterEarning 分支

合法性要求 openLegacyLegs = 0，twStep 不动 openLegacyLegs，后态 stage = earningShares，后态 openLegacyLegs = 0，inv 成立。`simpa only [twStep] using hlegal` 正确。**PASS。**

### clearCampaign 分支

twStep 把 stage 重置为 costReduction，后态 stage ≠ earningShares，inv 前件假，成立。by decide 判 `costReduction = earningShares` 为 False。**PASS。**

### OQ9Inv 强度判断

`OQ9Inv s := s.stage = earningShares → s.openLegacyLegs = 0`

精确捕获「earning 阶段 legacy 腿恒空」，这正是封堵 raw witness 形状 `(earning ∧ openLegacyLegs=1)` 在合法 trace 上不可达所需的不变量。不弱也不过强（弱化会漏，不需要更强）。**PASS。**

### 归纳链 oq9inv_trace

标准 List 归纳，cons 步正确应用 oq9inv_preserved 和归纳假设。**PASS。**

### 问题1总裁决：PASS。trace 级端B 真成立，六个分支无漏洞。

---

## 问题2：twEvent 忽略定理

```lean
theorem assemblyStep_twState_ignores_event_twEvent
    (x : AssemblyState) (e₁ e₂ : AssemblyEvent)
    (hpe : e₁.parseEvent = e₂.parseEvent) :
    (assemblyStep x e₁).twState = (assemblyStep x e₂).twState
```

证明：用 `assemblyStep_threads_twState` 展开两侧为 `twStep x.twState (订单.twEvent)`，订单由 `recAdapter`（只用 parseEvent）经确定函数链派生，`hpe` 使两侧订单相同，结论成立。

这条定理真正坐实「e.twEvent 不进 twState 的数据流」——换掉 e.twEvent 后 twState 后态完全不变。这与旧「外部 schema 占位，非死字段」辩护构成直接对立：定理证明的正是「死字段」的机器可证形式。**PASS。修复真诚实。**

---

## 问题3：端A 诚实分工

**理由**：scheduleAdapter 只产 shortDiff/recoverCapital，不产 openShareLeg/closeShareLeg，因为「何时开闭腿是 Θ_voice 策略决策，L0 不能臆造」。`assemblyStep_cumNetCash_unchanged` 诚实声明「端A 通道不在此闭环有效域」。

**判断**：这个分工界限成立。L0 结构层的职责是「给定某个 Θ，形式化系统如何闭环」，不是「Θ 会做什么决策」。openShareLeg/closeShareLeg 的派生需要策略层知道「现在应该开几条腿/关哪条腿」——这是 Θ_voice 的领域，L0 没有这些信息，不应臆造。

**严格性检查**：「声明不接入」而非「真接入」的选择：
- 若臆造策略层逻辑：违反 no-patch-mentality（臆造 = 声明代码具备不存在的能力）
- 若诚实声明不接入：符合 formalization-validity-domain（有效域声明 < 定义域）

`assemblyStep_cumNetCash_unchanged` 的存在恰好是诚实分工的机器证据：它证明了「在此闭环中，端A 通道永远不被驱动」，这是有效域边界的精确声明，不是回避。**PASS，分工界限成立。**

---

## 问题4：clearCampaign 和 zero-close

### clearCampaign

旧版 `clearCampaign => _ => True` 允许 openLegacyLegs > 0 时 campaign 结束，强制清零绕过 gate。
新版 `clearCampaign => s.openLegacyLegs = 0` 要求先清 legacy 腿，不能强制抹账。

这真正堵住了绕 gate 漏洞——legal 层 clearCampaign 现在与 enterEarning 的入口证书一致：进入新 campaign 前须先清旧腿。**PASS。**

### zero-close（幽灵腿）

`zero_close_is_ghost` 同时证：
1. `(twStep s (closeShareLeg p)).cumNetCash = s.cumNetCash + p`（幽灵副作用存在）
2. `¬ LegalTransition s (closeShareLeg p)`（legal 层非法）

两件事并列展示，raw 层可观测 + legal 层排除，双层分离清晰。**PASS。**

---

## 问题5：新引入问题

### 5a. assemblyStep_twState_changes 的 witness 语义

witness 中 `e.twEvent := TWEvent.clearCampaign`，但 `transitionAdapter` 不消费 `e.twEvent`，实际触发 twState 变化的是 `o.twEvent = recoverCapital 1`（scheduleAdapter else 分支）。

**判定**：逻辑正确，证明有效（e.twEvent 值无关紧要，assemblyStep_twState_ignores_event_twEvent 定理已坐实这一点）。但 witness 的注释「订单 twEvent=recoverCapital 1」是准确的，只是没有明说「e.twEvent 选值无关」。这是轻微注释不完整，**不是逻辑漏洞，不影响正确性**。

### 5b. OQ9Inv 初态条件——系统性开口

`oq9inv_trace` 要求初态满足 `OQ9Inv s`。若初态 stage=costReduction（正常起点），OQ9Inv 前件假，inv 显然成立。但此「初态 OQ9Inv 成立」没有被形式化为独立定理（如 `init_satisfies_oq9inv`）。

**判定**：这是一个残留验证缺口。不严重（初态 costReduction 时 inv 平凡成立），但未被机器检查。若有恶意构造的初态（`stage=earningShares ∧ openLegacyLegs=1`），trace 定理不适用，且这样的初态在 assembly 中实际上是可能的（任何人可以构造一个 AssemblyState 满足此条件并传入 assemblyStep）。

**这是一个真实的未填补缺口：缺少 `assembly_init_satisfies_oq9inv` 类型的定理，说明从规范初态启动时 OQ9Inv 成立。**

### 5c. openLegacyLegs 闭环恒定——隐含约束未显式化

在当前 assembly 中，scheduleAdapter 不派生 openShareLeg/closeShareLeg，因此 openLegacyLegs 在整个闭环生命周期内等于初始值（不增不减）。这个事实被 assemblyStep_oq9_gate_preserved 内部使用，但没有对称的 `assemblyStep_openLegacyLegs_unchanged` 定理明确声明。

相比之下，cumNetCash 有 `assemblyStep_cumNetCash_unchanged` 的显式声明。openLegacyLegs 同样不动，但没有对应定理。

**判定**：轻微不对称——端A 通道的两个字段（cumNetCash 和 openLegacyLegs）只有前者被显式声明不动。这不是逻辑漏洞（gate_preserved 覆盖了功能需求），但从文档对称性和声明完整性角度，缺失 openLegacyLegs 不动的显式定理。

---

## 综合裁决

| 修复项 | 上轮裁决 | 修复后状态 | 判定 |
|--------|---------|-----------|------|
| 1. trace 级 gate | 单步不够，要 trace 级 | oq9inv_preserved × 6分支 + oq9inv_trace + 不可达定理 | **PASS** |
| 2. twEvent 声明膨胀 | 是死字段，改文档+加定理 | assemblyStep_twState_ignores_event_twEvent 机器可证 | **PASS** |
| 3a. 端A 空洞 | cumNetCash 闭环永不动 | assemblyStep_cumNetCash_unchanged 诚实声明 | **PASS** |
| 3b. clearCampaign 绕 gate | 强制清零绕 gate | LegalTransition.clearCampaign 加 openLegacyLegs=0 前提 | **PASS** |
| 3c. zero-close 幽灵 | 幽灵副作用未标注 | zero_close_is_ghost 双重证明 | **PASS** |

**新发现（非翻转修复，为残留缺口）**：

| 缺口 | 严重性 | 性质 |
|------|--------|------|
| 5b. 初态 OQ9Inv 未形式化 | 低（平凡成立，但未机器检查） | 验证缺口 |
| 5c. openLegacyLegs 闭环恒定未显式声明 | 低（功能已覆盖，声明不对称） | 文档/声明缺口 |
| 5a. witness 中 e.twEvent 语义不明 | 无（逻辑正确） | 注释完整性 |

**最终裁决：原三项修复全部 PASS。**

两个残留缺口（初态 inv 未形式化 + openLegacyLegs 不动未显式化）属于新发现，不属于上轮裁决范围内的未修复项，其严重性低（不影响逻辑正确性），但在严格性（no-patch-mentality）要求下属于「声明与实际不完全一致」的范畴——具体是：声明了端A 通道在两个维度上不被驱动，但只显式证了其中一个（cumNetCash），另一个（openLegacyLegs）依赖隐含推导。

---

## 边界条件

本审查结论在以下条件下成立：

- `lake build` 通过（用户已确认 54 jobs 全绿，无 sorry/admit/axiom）
- Lean 4 的 Nat 饱和减法语义正确（`n - m = 0` 当 `n < m`，不抛出错误）
- `by decide` 战术对有限枚举的 rank 比较可判定（TStage 只有三个构造子，rank 值 0/1/2，可计算）

翻转条件：

- 若 Lean 4 的 `decide` 战术在某个分支意外超时或返回 false（不可能，因为这些是有限枚举）——不影响
- 若 `oq9inv_preserved` 的证明在某个分支实际失败（lake build 已通过，排除）
- 若用户注入不满足 OQ9Inv 的初态（5b 缺口激活）——trace 定理的保证失效

---

## 影响声明

本审查结果影响：
- task #93 的状态：原三项修复全部 PASS，可标记修复完成
- 两个残留缺口（5b/5c）属于补充发现，由下游决定是否补全形式化
- TotalWealth.lean 和 HybridAssembly.lean 无需进一步修改以消解上轮裁决
