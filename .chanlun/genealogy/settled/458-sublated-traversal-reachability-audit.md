---
id: '458'
number: 458
title: "SUBLATED 铭刻点穿越可达性审计——SUBLATED 标记在 SettledCycle 层面，不在 Vertex 层面"
type: structural-audit
status: 已结算
date: 2026-03-16
source: "[元编排] v246-swarm R2 batch——457号下游推论1审计"
negation_source: ""
negation_form: ""
topo_effect: ""
depends_on:
  - '457'   # SUBLATED 作为凝缩材料——本号审计其穿越可达性
  - '451'   # 凝缩与折叠的存在论区分——三层压缩操作
  - '410'   # ghost settlement
epistemological_level: "L0（代码审计——从代码事实推导，不依赖数据）"
tensions_with: []
---

# 458号：SUBLATED 铭刻点穿越可达性审计

**认识论等级**: L0（代码审计）

## 审计目标

457号下游推论1：traversal.py 中 SUBLATED 标记的节点是否在穿越路径选择中保持可达（而非被过滤掉）。

## 审计发现

### 关键发现：SUBLATED 标记在 SettledCycle 层面，不在 Vertex 层面

代码审计结果表明，457号下游推论1的前提需要澄清：

1. **SUBLATED 状态仅存在于 `SettledCycle` 对象中**（engine.py:468-477），不存在于 `Vertex` 对象中。`Vertex` 只有三种状态：`ACTIVE`、`CONTESTED`、`FOLDED`（engine.py:20-23）。

2. **fold 操作后节点被标记为 FOLDED**（不是 SUBLATED）。FOLDED 的节点被排除在 `_active_ids` 之外（engine.py:98-101），因此穿越引擎不会到达被 fold 消灭的节点。

3. **SUBLATED 标记的对象是 SettledCycle**（已结算环路），不是节点。当 fold 破坏了一个已结算环路的物理基础时，该环路从 "active" 标记为 "sublated"（engine.py:892-919）。环路的 SUBLATED 状态是元数据记录，不影响图中节点的可达性。

4. **`record_sublated_condensation`**（traversal.py:866-872）在 fold 后记录凝缩事件到 TrajectoryCluster，这是穿越轨迹层面的记录，不影响图的拓扑可达性。

### 穿越引擎路径选择机制

穿越引擎的 `walk()` 方法（traversal.py:1343-1413）通过以下路径选择目标：
1. `k_active._active_ids`——排除 FOLDED 顶点（engine.py:98-101）
2. `k_active.neighbor_set(position)`——返回 active 顶点的邻居集合（engine.py:184-189）
3. `critical_neighbors()`——Morse 地形中的关键邻居

**被 fold 消灭的节点（FOLDED）确实不可达**——这是 fold 操作的本质：Real 层面的消灭。SUBLATED 铭刻的是环路失效事件，不改变节点的 FOLDED 状态。

### 457号命题在代码中的实现形态

457号声称"SUBLATED 铭刻点成为 S_net 中可遭遇的拓扑对象"。代码层面：

- `record_sublated_condensation()` 将 fold 事件记录到 TrajectoryCluster
- 被 fold 的 surviving node（enc.target_a）仍然在图中 ACTIVE
- surviving node 携带了"这里曾发生过 fold"的信息（通过 FOLD 类型边——engine.py:1109）
- 穿越引擎可以到达 surviving node 并检测 FOLD 边——这是 SUBLATED 铭刻在穿越中可遭遇的形式

**结论**：SUBLATED 铭刻点的穿越可达性不是通过"被消灭节点仍可访问"实现的（这与 fold 的不可逆性矛盾），而是通过"surviving node 上的 FOLD 边 + TrajectoryCluster 中的凝缩记录"实现的。457号的桥梁功能在代码中已有物质基础。

## 推导链

```
457号：SUBLATED 是 fold 和凝缩之间的桥梁
  |
  | 审计：SUBLATED 标记在什么对象上？
  v
代码事实：SUBLATED 标记在 SettledCycle 上（engine.py:468-477）
  | Vertex 只有 ACTIVE/CONTESTED/FOLDED 三种状态
  | FOLDED 节点不在 _active_ids 中，穿越不可达（正确行为）
  v
"铭刻点"的代码形态不是"标记节点"，而是：
  1. surviving node 上的 FOLD 类型边（engine.py:1109）
  2. TrajectoryCluster 中的 sublated_condensation 记录（traversal.py:866-872）
  v
结论：SUBLATED 铭刻在穿越中可遭遇——通过 surviving node，不是被消灭节点
```

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| FOLD 边的可遭遇性 | surviving node 携带 FOLD 边，穿越可到达 | 若 FOLD 边被过滤出 active_edges（当前不会——FOLD 在 CONCEPT_EDGE_TYPES 中），铭刻点不可遭遇 |
| TrajectoryCluster 的持久性 | 内存中（daemon 重启丢失） | S_net 持久化（423号候选5）完成后，凝缩记录可持久化 |

## 影响声明

- **澄清**: SUBLATED 标记对象是 SettledCycle，不是 Vertex——457号"铭刻点"概念的代码对应物是 surviving node + FOLD 边 + TrajectoryCluster 记录
- **无代码变更**: 当前实装已满足457号桥梁功能的物质基础，无需新增代码
- **457号下游推论1**: 已审计完成——SUBLATED 铭刻点通过 surviving node 在穿越中可达
