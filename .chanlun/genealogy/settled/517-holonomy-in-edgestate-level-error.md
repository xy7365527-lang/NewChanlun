---
id: '517'
number: 517
title: "EdgeState 中 h（和乐）字段的层级错误——边属性与循环属性混淆"
status: 已结算
type: 矛盾发现
resolution: >
  分裂——将 W3 K4 状态计算从单一 EdgeState 扁平对象拆分为按 simplex 维度分层的状态域：
  EdgeState 只承载 1-Simplex 边属性，CycleState 承载 2-Simplex/闭合循环属性；
  tau 的归属另行按顶点/全局状态审查，不能默认塞入 EdgeState。
settlement_classification: 分裂
settled_date: 2026-04-27
settled_by: cursor-gpt-5.5
negation_source: heterogeneous
negation_form: separation
negation_model: gemini-3.1-pro-preview
created: 2026-04-27
depends_on:
  - '485'
  - '486'
affects:
  - W3-K4状态计算引擎
  - k4_state.py
  - holonomy.py
  - fiber_bundle.py
related:
  - '516'
---

# 517号：EdgeState 中 h（和乐）字段的层级错误

## 四分法分类

**分类：分裂。**

517 不是“吸收”：485 号已经声明敞口拓扑是多层加权有向图，并说明边有方向、强度、级别，但没有把 W3 的工程状态对象按 simplex 维度显式拆开。

517 不是普通“修正”：错误不只是删除 `h` 字段或改名，而是 `EdgeState{chanlun, phi, h, tau, energy_modes}` 这个扁平对象把不同拓扑维度的属性混在一起。`phi` 和 `omega` 属于边，`h/holonomy` 属于闭合循环，`tau` 还可能属于顶点或 K4 全局状态。单一 `EdgeState` 无法通过局部字段修补恢复类型正确性。

517 不是“废弃”：W3 K4 状态计算引擎仍然必要；被废弃的是“所有状态都挂在边上”的扁平建模方案。

因此采取“分裂”：将 W3 分为至少两个正交计算域：边引擎与循环引擎；必要时再分离顶点/全局状态域。

## 矛盾描述

Claude Phase A Plan（v74）在 W3 K4 状态计算引擎中定义：

```text
EdgeState{chanlun, phi（金融化度）, h（和乐）, tau（扭矩）, energy_modes}
```

这把和乐 `h` 作为边（Edge）的属性字段。这是层级错误：和乐是**循环（Cycle）的属性**，不是边的属性。

## 框架依据

**PDF p.289（拓扑化索罗斯框架）**：

> “金融化度（视图 3）是单条边的判断——实物流量 vs 金融流量比值，这不需要多种一般等价物。”

> “和乐告诉你某个循环在不同一般等价物下是否一致。这是跨边的整体判断。两者不重复：情形 1（金融化度高但和乐为零）说明金融化严重但稳定……”

**蓝图 v3 圈 2（向量丛与和乐）**：

> “沿一个 K4 闭合循环 γ：hol(γ) = exp(∮_γ ω)”

> “拓扑不变量：β₁(K4) = 独立循环数（组合拓扑，不变）。和乐群 = 沿所有循环的旋转构成的群（度量拓扑，可变）。”

K4 的 β₁(K4) = 3（三个独立循环），每个循环都有独立的和乐值。和乐不属于任何一条边，它属于整个闭合循环。

## 具体错误

1. **语义歧义**：如果和乐放在 `EdgeState`，那么 `$→C` 边的 `h` 值无法确定属于哪个循环。一条边可以参与多个循环，循环级别的和乐不能归属到单条边。
2. **数学错误**：和乐的计算需要沿闭合路径积分 `exp(∮_γ ω)`，这要求读取路径上所有边的联络值，再计算整体旋转。边可存联络 `omega`，循环才存 `holonomy`。
3. **工程后果**：W1 视图 4（和乐表）的行是 K4 循环，列是各一般等价物下净流量与和乐。如果和乐放在 `EdgeState`，视图 4 无法获得类型正确的数据源。

## 分裂后的结构要求

W3 必须拆成按拓扑维度分层的状态域：

### 1-Simplex：边引擎

`EdgeState` 只承载单边属性，例如：

```text
EdgeState{edge_id, chanlun, phi, omega, energy_modes?}
```

- `chanlun`：该边对应价格/比价走势的缠论状态
- `phi`：金融化度，实物流量 vs 金融流量比值，单边属性
- `omega`：联络/汇率/平行移动规则，供循环积分使用
- `energy_modes` 是否属于边需在实现时以定义为准，不可默认混入

### 2-Simplex / Cycle：循环引擎

`CycleState` 承载闭合循环属性，例如：

```text
CycleState{cycle_id, edge_path, holonomy_A, holonomy_B, energy_distribution}
```

- `cycle_id`：K4 独立循环或派生循环标识
- `edge_path`：组成闭合路径的有向边序列
- `holonomy_*`：沿该闭合循环计算出的和乐/holonomy
- `energy_distribution`：若定义为循环级分布，则放在这里

### 0-Simplex / Global：待审状态域

`tau`（扭矩）不能默认作为边属性。若扭矩定义在每个非 `$` 顶点上，应进入 `VertexState`；若定义为全局 K4 张力，应进入 `K4State`。在定义未定前，W3 实现不得把 `tau` 塞入 `EdgeState`。

## 推导链

金融化度是单边属性 → 和乐是闭合循环属性 → K4 有 β₁=3 个独立循环 → 单边可参与多个循环 → `EdgeState.h` 无法唯一解释 → 这是拓扑维度混淆，不是命名问题 → W3 状态域必须按 1-Simplex / 2-Simplex 分裂。

## 边界条件

如果 K4 拓扑退化为树状结构（β₁=0），和乐恒为零，不需要 `CycleState`，此矛盾消失。但 K4 本质上含闭合循环，且当前框架明确以 β₁(K4)=3 为前提，因此此边界条件不成立。

## 下游行动

- Phase A 计划中 W3 的 `EdgeState{..., h, tau, ...}` 方案废弃。
- `k4_state.py` 若实现，必须显式区分 `EdgeState` 与 `CycleState`，并为 `tau` 预留非边级归属。
- `holonomy.py` 必须以循环为输入/输出核心：读取 `edge_path` 上的 `omega`，输出 `CycleState` 中的 `holonomy`，不得写入单条边。
- W1 视图 4 的数据接口必须从 `CycleState` 读取，而不是从 `EdgeState` 聚合。
- `fiber_bundle.py` 需要检查其承载的是联络/平行移动规则，还是只存流量数值；若只有流量数值，不能声称已经支持 holonomy。
