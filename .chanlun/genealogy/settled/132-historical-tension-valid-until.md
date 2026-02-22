---
id: '132'
title: tensions_with边引入valid_until字段——活跃张力与历史失效张力的严格区分
type: 选择
status: 已结算
date: 2026-02-22
depends_on:
  - '121'   # 065的tensions_with边在068否定后自动失效
  - '120'   # 062被扬弃后引用分类规则
  - '090'   # 严格性是蜂群语法规则——排除路径C
  - '068'   # DAG范式——图谱应自足表达
related:
  - '129'   # negates稀疏性审查
  - '012'   # 谱系是发现引擎——更多语义信息有助于发现
negation_source: heterogeneous
negation_form: 选择（从三路径逻辑排除后推导）
negation_model: gemini-3.1-pro-preview
negates: []
negated_by: []
provenance: "[新缠论]"
---

# 132号：tensions_with 边引入 valid_until 字段

**类型**: 选择（路径B——从三路径逻辑排除后推导）
**状态**: 已结算
**日期**: 2026-02-22
**来源**: Gemini异质质询（焦点5第三轮矛盾点5）→ 四分法重审

## 问题

dag.yaml 中 tensions_with 边混用两种语义：活跃张力（当前未解决）和历史失效张力（因否定事件已失效但作为历时性记录保留）。

## 推导链

```
前提1: 090号——严格性是蜂群语法规则（非严格产出语法不合法）
前提2: 068号——DAG范式（图谱应自足表达，不依赖外部文本）
前提3: 012号——谱系是发现引擎（更多语义信息有助于发现）

三路径排除:
- 路径C（维持现状 + 文本谱系）：
  → 图谱语义依赖外部文本解释 = 声明-能力不一致
  → 违反090号（非严格）和068号（图谱应自足）
  → 排除

- 路径A（新增 historical_tension 边类型）vs 路径B（valid_until 字段）：
  → 路径A：二值标记（活跃/历史），丢失失效原因
  → 路径B：保留失效原因（哪个节点的否定事件导致失效）
  → 路径B 信息量严格 ≥ 路径A（012号偏好）
  → 路径B 不改变边类型分类，兼容性更好
  → 选择路径B

结论: 在 tensions_with 边中引入 valid_until 字段
```

## 结论

**选择路径B：在 tensions_with 边中引入 `valid_until` 字段。**

格式：
```yaml
tensions_with:
  - between: ['062', '065']
    valid_until: '068'    # 068否定065后，此张力失效
```

- `valid_until` 为空或不存在 = 活跃张力
- `valid_until: '<node_id>'` = 该节点的否定事件使张力失效（历史记录保留）

## 执行清单

需要扫描所有 tensions_with 边，为涉及已否定节点（negated_by 非空）的边添加 valid_until 字段。

已知候选：
- T16（062↔065）：valid_until: '068'（068否定065）
- 其他涉及062号的边：需检查是否因064/093的否定而失效

## 边界条件

- 如果 valid_until 的语义扩展到非 negates 类型的失效（如概念分离导致的失效），字段语义需要更新
- 如果引入自动化推理引擎，valid_until 字段需要被引擎理解和使用

## 下游推论

- dag.yaml Schema 增加 valid_until 可选字段
- 所有涉及已否定节点的 tensions_with 边需要补充 valid_until
- dag-validation-guard 可选增加：检查 valid_until 引用的节点是否存在

## 谱系链接

- **121号**：065张力边失效定理——本条是其 Schema 层实现
- **120号**：062引用分类——提供"历史参照"概念先例
- **090号**：严格性语法规则——排除路径C的依据
- **068号**：DAG范式——图谱自足表达要求
