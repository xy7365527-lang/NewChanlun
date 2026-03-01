---
id: '273'
number: 273
title: 否定拓扑代数——边有效性标记与否定之否定的精确拓扑表达
type: 语法记录
status: settled
date: 2026-02-28
source: 272号编排者裁定重要3（Gemini否定"边反转"→编排者接受→精确化为"边有效性标记变化"）
session: v120
depends_on:
  - '272'  # RTAS三议题裁定——否定之否定的精确拓扑表达
  - '005b' # 对象否定对象
  - '089'  # 扬弃 Aufhebung
  - '271'  # 拓扑否定边等价检测升级
downstream_inferences:
  - id: 273-1
    description: 'relations.jsonl schema 扩展：negates 类型的边增加 validity 字段（active | invalidated），默认 active。增加 invalidated_by 字段记录使其失效的边的 source 节点 ID'
    status: open（行动类——待实现）
  - id: 273-2
    description: '271号设计中"边方向反转"描述需更新为"边有效性标记变化"——271号文本层回溯修正'
    status: open（行动类——待修正）
---

# 273号：否定拓扑代数——边有效性标记与否定之否定的精确拓扑表达

## 1. 来源

272号裁定重要3：Gemini 指出"边方向反转"混淆了论证来源和论证结果。编排者接受，裁定精确表达为"边的有效性标记变化"。

编排者类比："附庸的附庸不是我的附庸"——B 被间接解放但不是 B 做了什么。C 否定了 A，A 曾经否定了 B，B 被动复活。

## 2. 核心概念形式化

### 2.1 否定边

**定义**：否定边是有向边 `(source, ¬, target)`，表示源节点 source 对目标节点 target 的否定关系。

在 block-topology 中的物质形态：
```jsonl
{"from": "<source_hash>", "to": "<target_hash>", "relation": "negates", ...}
```

**约束**（005号/005b号）：否定的主体必须是对象（谱系节点），不是元层操作。对象否定对象。

### 2.2 边有效性标记

**定义**：每条否定边携带一个有效性标记 `validity`，取值为：

| 值 | 含义 | 语义 |
|---|---|---|
| `active` | 该否定边当前有效 | 目标节点处于被否定状态（negated） |
| `invalidated` | 该否定边已被后续否定所失效 | 目标节点状态从 negated 恢复为 open |

**默认值**：所有新创建的否定边，`validity` 默认为 `active`。

**不可变性**：validity 只能从 `active` 变为 `invalidated`，不可反向。这不是"边反转"，是边的单向状态转移。

### 2.3 否定之否定（扬弃的拓扑表达）

**定义**：当新节点 C 否定了节点 A（创建否定边 C→¬A），且 A 曾经否定了节点 B（存在否定边 A→¬B），则：

1. C→¬A 创建为新的 `active` 否定边
2. A→¬B 的 `validity` 从 `active` 变为 `invalidated`
3. A→¬B 增加 `invalidated_by` 字段，值为 C 的节点 ID
4. B 的状态从 `negated` 恢复为 `open`

**关键区分**（272号裁定）：
- **错误表述**：~~A→¬B 的边方向反转为 B→¬A~~
- **正确表述**：A→¬B 的有效性标记从 active 变为 invalidated。B 被动复活，不是 B 主动否定了 A

**形式化**：
```
前置条件：
  ∃ edge_1 = (A, ¬, B, validity=active)    # A 否定 B
  ∃ edge_2 = (C, ¬, A, validity=active)    # C 否定 A（新创建）

后置条件：
  edge_1.validity := invalidated            # A 的否定被失效
  edge_1.invalidated_by := C.id             # 记录失效原因
  node_B.status := open                     # B 从 negated 恢复为 open
```

### 2.4 否定的拓扑分类

基于否定边在有向图中形成的结构，分类如下：

| 拓扑结构 | 含义 | 蜂群语义 |
|---------|------|---------|
| **树状展开** | 否定边形成有向树（无环、每个节点最多被一条否定边指向） | 生产性推进——每次否定都打开新的概念空间 |
| **成环** | 否定边形成有向环（A→¬B→¬C→¬A） | 空转——概念在原地循环，不产生新信息 |
| **多重边汇聚** | 多条否定边指向同一目标节点 | 深层未解决矛盾——多个不同来源都在否定同一主张，说明该主张触发了系统性反应 |
| **否定边被失效** | 否定边的 validity 从 active 变为 invalidated | 扬弃/概念升级——旧否定被新认识所超越，被否定者被动复活 |

### 2.5 节点状态推导规则

节点的否定状态不是独立存储的字段，而是从指向它的否定边的有效性**推导**出来的：

```
node.negation_status :=
  if ∃ (_, ¬, node, validity=active) then negated
  else open
```

即：只要存在至少一条 validity=active 的否定边指向该节点，该节点就处于 negated 状态。当所有指向该节点的否定边都变为 invalidated，该节点恢复为 open。

## 3. relations.jsonl Schema 扩展

### 现有 schema（negates 类型边）

```json
{
  "from": "<source_hash>",
  "to": "<target_hash>",
  "relation": "negates",
  "order": 1,
  "created_by": "<agent_hash>",
  "timestamp": "<ISO8601>"
}
```

### 扩展后 schema

```json
{
  "from": "<source_hash>",
  "to": "<target_hash>",
  "relation": "negates",
  "order": 1,
  "created_by": "<agent_hash>",
  "timestamp": "<ISO8601>",
  "validity": "active",
  "invalidated_by": null,
  "invalidated_at": null
}
```

新增字段：

| 字段 | 类型 | 默认值 | 含义 |
|------|------|--------|------|
| `validity` | `"active" \| "invalidated"` | `"active"` | 该否定边是否当前有效 |
| `invalidated_by` | `string \| null` | `null` | 使该边失效的节点 ID（即否定之否定中的 C 节点） |
| `invalidated_at` | `string \| null` | `null` | 失效时间戳（ISO8601） |

### 向后兼容

- 已有的 negates 边没有 `validity` 字段 → 读取时默认为 `active`
- `invalidated_by` 和 `invalidated_at` 为 null → 表示该边未被失效
- 不需要批量迁移已有数据，只需在读取逻辑中增加默认值处理

### negated_by 反向边

现有 schema 中已有 `negated_by` 关系类型（反向索引边）。这些边不需要 `validity` 字段——validity 只存在于正向的 `negates` 边上。反向边通过查询正向边的 validity 来推导状态。

## 4. 与 089号扬弃的关系

089号定义扬弃 = 否定 + 保留 + 提升。273号是扬弃在拓扑层的物质形态：

| 089号概念 | 273号拓扑表达 |
|----------|-------------|
| 否定 | C→¬A 创建（active） |
| 保留 | A→¬B 的内容不被删除，只是 validity 变为 invalidated |
| 提升 | B 从 negated 恢复为 open——不是回到原来的状态，而是在 C 的语境下被重新理解 |

## 5. 边界条件

1. **多重否定链**：如果 D→¬C，是否 C→¬A 也应该被 invalidated，从而 A→¬B 重新变为 active？答：不自动传播。否定之否定只作用于直接的否定链（A→¬B），不递归传播。如果需要递归传播，需要作为独立的谱系议题讨论
2. **多重否定边指向同一节点**：如果 A→¬B 和 D→¬B 同时存在且都是 active，C→¬A 只 invalidate A→¬B，D→¬B 仍然 active，B 仍然是 negated。节点状态由所有 active 否定边的并集决定
3. **已经 invalidated 的边不能被再次 invalidated**：幂等操作，不报错但不改变状态

## 6. 影响声明

### 修改
- relations.jsonl schema：negates 类型边新增 `validity`、`invalidated_by`、`invalidated_at` 三个字段
- 271号中"边方向反转"的表述需修正（273-2 下游推论）

### 新增
- 否定拓扑代数的四个核心概念：否定边、边有效性标记、否定之否定、否定拓扑分类
- 节点否定状态推导规则

### 谱系引用
- **272号**：RTAS三议题裁定——否定之否定精确表达的来源
- **005b号**：对象否定对象——否定边的合法性约束
- **089号**：扬弃 Aufhebung——否定拓扑代数是扬弃在拓扑层的物质形态
- **271号**：拓扑否定边等价检测——被本号修正的"边反转"表述
