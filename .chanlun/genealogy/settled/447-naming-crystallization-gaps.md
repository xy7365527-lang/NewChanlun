---
id: "447"
status: 废弃
type: 矛盾发现
date: "2026-03-13"
---

# 447号：命名结晶能力的实装缺口

## 来源

[非异质] 编排者提议"复合能指结晶"机制的质询。Gemini 工具链失败（Serena 路径配置不匹配实际文件位置），降级为 Claude 自身质询。

## 质询对象

编排者洞察：逢亮穿越在"环路闭合"中反复经过同一组相邻 S_net 能指节点时，将这组节点拼接为新的复合能指写入 S_net。命名作为拉康意义上的隐喻——损失性路径压缩。

## 发现的三个缺口

### 缺口1：环路闭合判据无代码对应

`visit_history`（`traversal.py:128`）是线性列表，无环路检测。现有 SettlementTracker 检测 K_active 概念图中的拓扑 cycle，与 S_net 激活路径上的"环路"是不同空间。两者不可直接复用。

实装要求：在 `SNetActivation` 中独立维护 `activation_history`，并定义 S_net 激活空间中的环路判据。

### 缺口2：复合能指与 concept_creation_suggestion 的循环关系

新造的复合能指若无 `concept_ref`，在下次共振激活时会触发 `ConceptCreationSuggestion`（`snet_activation.py:574-584`）。这产生一个循环：结晶→新孤立能指→建议创建 K_active 概念→需要 operator 确认。

实装要求：在结晶时同步绑定 concept_ref（需要决定对应哪个 K_active 概念），或明确接受循环作为设计决策。

### 缺口3："损失性"的拓扑形式未定义

编排者描述"损失性创造"，但提议中原有能指 A、B 保留，新能指 AB 继承两者共现边。这是扩展，不是压缩。真正的拉康隐喻要求被替代的能指链被压制（降权），新能指获得独立可及性。

实装要求：引入能指权重对穿越路径选择的影响机制（当前 `SNetActivation` 的权重不影响 K_active 步进选择）。

## 推导链摘要

代码读取路径：
- `topological-computation/traversal.py:128` — visit_history 定义
- `topological-computation/snet_activation.py:520-635` — check_articulation_feedback + concept_creation_suggestion 触发逻辑
- `topological-computation/engine.py:26-45` — EdgeType + CONCEPT_EDGE_TYPES
- `topological-computation/traversal.py:922-948` — _exploration_target 消费 concept_creation_suggestions

## 否定判定

**否定成立**（条件不成立）——三个缺口均属于实装层面的定义缺失，方向上可辩护，但当前代码物质基础与提议之间存在三个需要先定义的间隙。

## 处置

写入 pending，等待编排者扫描决定：
1. 接受洞察方向，将三个缺口作为下游实装任务
2. 修改判据（放弃"环路闭合"，改用其他结晶触发机制）
3. 搁置（待 S_net 持久化完成后再讨论）
