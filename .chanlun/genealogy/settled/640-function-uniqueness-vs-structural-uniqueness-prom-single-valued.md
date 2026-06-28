---
id: 640
status: 已结算
title: 函数性单值 vs 结构性单值——§16 假设2"Prom 单值"的概念分离（前者真 by-construction，后者假 derivation_not_unique）
date: 2026-06-28
type: 概念分离
authority_chain: [codex异质审查(Q2), Lean derivation_not_unique 机器证明, no-patch 090号]
related: [639, 635, lead-self-cleared-inflation-flag-is-rationalization, 231]
---

# 640 函数性单值 vs 结构性单值

## 触发

codex 异质审查（aa5b7935ee）质询 `PromQual.lean` 的 `lift_single_valued` 声明膨胀；Lead 先 rationalize "合法非膨胀" 被推翻（见 memory [[lead-self-cleared-inflation-flag-is-rationalization]]）；修复工位（a4c99fa8）试方案B失败后发现真概念分离。

## 概念分离的精确形式

§16 假设2"Prom_* 全定义且**单值**"的"单值"被旧 `IsLiftOf`（钉死 `centers = windowCenters es`）**混同了两个不同的单值性**：

| | 函数性单值（function-uniqueness）| 结构性单值（structural uniqueness）| canonical 选择唯一性（canonical-selector uniqueness）|
|---|---|---|---|
| 命题 | `promote` 是全函数 ⟹ 同输入唯一输出 | 任意满足提升语义的 E（含非 canonical centers）唯一 | `windowCenters` 作为 deterministic first-window selector 在其定义域上产唯一结果 |
| 真假 | **真**（by-construction，L0 零信息增量）| **假** | **真**（deterministic selector，L0） |
| 性质来源 | 函数应用确定性（promote 良定义） | 派生谓词对**所有**合法 centers 的唯一性约束 | 规范选择策略的首窗口确定性 |
| 证据 | `promote_function_unique`（∃E,E=promote∧∀E',…）| **`derivation_not_unique`**（∃ es cs1 cs2, CentersDerivedFrom 二者∧cs1≠cs2）| **`windowCenters_deterministic`**（∃ c, c=windowCenters∧∀ c', c'=windowCenters→c'=c，RecursiveConstruction:297）|
| 有效域 | promote 的定义域 | CentersDerivedFrom 的定义域（全 centers） | **canonical selector 的定义域**（首窗口可产中枢的 es；首三段无中枢时产空，仍在 selector 定义域内） |

**关键机器事实**：`CentersDerivedFrom`（RecursiveConstruction:144）是**松 soundness 谓词（多见证）**——空 `starts=[]` 对任意非空 es 平凡满足，异窗 `starts=[1]` 派生异中枢亦 sound。`windowCenters`（:293）只查首三段产 ≤1 中枢 = canonical **选择**，非"唯一全派生"。故结构性单值假。

**第三类为何独立**：`windowCenters` 的唯一结果既不是 `promote` 函数应用的确定性（它不是 promote），也不是 `CentersDerivedFrom` 对所有合法 centers 的唯一性约束（selector 只挑一个 canonical 窗口，不约束其他合法 centers 的存在性）。它是"规范选择策略产唯一结果"——deterministic selector 的 by-construction 单值，有效域 = selector 的定义域（非全 centers 定义域）。把 canonical 选择唯一性混入结构性单值 = 有效域膨胀（声明 selector 唯一 = 全派生唯一）；把 canonical 选择唯一性混入函数性单值 = 错认性质来源（selector 不是 promote）。

旧 `lift_single_valued` 用 `centers=windowCenters es` 钉死，把结构性单值**偷换**为函数性单值（`IsLiftOf↔E=promote`），以"定理"形式包装函数应用确定性的同义反复 = 声明膨胀（090号）。

## 结果包六要素

1. **结论**：§16 假设2"单值"由**函数性单值**（promote 全函数，真，by-construction）discharge，**非**结构性单值（假，`derivation_not_unique` 证伪）。删 `IsLiftOf`/`lift_single_valued` 膨胀簇，替为 `promote_function_unique`（诚实函数性）+ `derivation_not_unique`（否定性结果）。
2. **定义依据**：spec §16 line 781 假设2；`CentersDerivedFrom`(RecursiveConstruction:144)多见证 + `windowCenters`(:293)canonical 选择。函数良定义性满足"单值"的函数性义；结构唯一性义被否定性定理证伪。
3. **边界条件**：若 RecursiveConstruction 引入"覆盖全 canonical 窗口且唯一"的派生谓词使派生唯一性可证 → 结构性单值复活，本分离失效。若编排者裁定假设2"单值"**必须**是结构唯一性 → `derivation_not_unique` 与 spec 构成真矛盾须 escalate（当前判定：假设2 只需函数确定性，无矛盾）。
4. **下游推论**：§16 假设2 discharge 诚实分层 = total(genuine)+单值(by-construction 函数性)+自相似(genuine)；MainTheorem 假设2 注释已改（删"单值=IsLiftOf 确定"）；任何声称"Prom 结构性单值"的下游都是膨胀。
5. **谱系引用**：639（同轮 σ_p 反转）；635（反膨胀守卫）；231/222/223/230（有效域⊊定义域；否定性结果缩小有效域——`derivation_not_unique` 本类）；090/087/089（声明膨胀禁止）。memory [[lead-self-cleared-inflation-flag-is-rationalization]]（Lead 自清膨胀=rationalization，本分离是其触发的正面产物）。
6. **影响声明**：登记函数性单值 vs 结构性单值的概念分离。影响 PromQual.lean（已改）+ MainTheorem 假设2 注释（已改）+ 未来任何"单值"声明须分层。不改 total/自相似（genuine 未动）。

## 为何 settled

概念分离由 Lean `derivation_not_unique` 机器证明钉死（结构性单值假是定理，非推测），codex 异质审查独立确认，无悬置。函数性单值真亦机器证明。
