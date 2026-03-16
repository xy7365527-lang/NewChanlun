---
id: '473'
number: 473
title: "Morse 命名门槛——β₁变化量 D 区分 critical/non-critical 顶点"
type: 实装记录
status: 已结算
date: 2026-03-16
source: "[元编排] v258-swarm/morse-naming 工位"
negation_source: ""
negation_form: ""
topo_effect: ""
depends_on:
  - '370'   # Morse 有效域边界——关键性是穿越判断不是计算输出
  - '469'   # 穿越步内串行是语义决定
  - '444'   # 命名是 ARTICULATE 的附随产物
epistemological_level: "L0（从已结算谱系推导的实装方案）"
tensions_with: []
---

# 473号：Morse 命名门槛——β₁变化量 D 区分 critical/non-critical 顶点

**认识论等级**: L0

## 核心命题

穿越中每次拓扑操作（sublation/negate_b）创建新顶点时，计算 D = β₁(after) - β₁(before)：
- D != 0 → **critical**：该操作改变了图的环路结构（独立环数变化），新顶点值得命名
- D == 0 → **non-critical**：该操作未改变环路结构，新顶点是拓扑冗余的

这是 370号"关键性是穿越判断"的操作化实装：不是用 Morse 理论预测哪些否定"将是"关键的（370号已否定此用法），而是在操作发生后**记录**该操作是否改变了 β₁——这是事后标记，不是事前预测。

## 推导链

```
370号：Morse 有效域精确终止于"关键性判断"（预测不可行）
  → 但 β₁ 变化量作为结构标记仍然有效（370号的边界条件：结构度量可用于标记，不可用于预测）
  → D = β₁(after) - β₁(before) 是每次操作的拓扑签名
  → D != 0 → 图的同调结构改变 → 命名有拓扑依据
  → D == 0 → 图的同调结构不变 → 命名无拓扑依据（non-critical 标记）
```

## 实装清单

| 文件 | 位置 | 变更 |
|------|------|------|
| engine.py | Vertex dataclass (L82) | 新增 `non_critical: bool = False` 字段 |
| traversal.py | StepLog dataclass (L104) | 新增 `critical: bool \| None = None` 字段 |
| traversal.py | execute_encounter/SUBLATION (L754-759) | sublation 创建新顶点后检查 D，D==0 标记 non_critical |
| traversal.py | execute_encounter/NEGATE_B (L819-826) | negate_b 创建新顶点后检查 D，D==0 标记 non_critical |
| traversal.py | run_step (L1481, L1719) | critical_flag 传递到 StepLog |
| traversal.py | bridge vertex 创建 (L1613-1619) | S_net bridge vertex 默认 non_critical=True（物质层引入，非概念层操作） |
| daemon.py | format_step_log (L224-234) | 日志输出 CRITICAL/non-critical 标注 |

## 覆盖的遭遇类型

| 遭遇类型 | 创建新顶点？ | D 检查？ | 理由 |
|----------|------------|---------|------|
| SUBLATION | 是（synthesis vertex） | 是 | sublation 总是创建合题顶点 |
| NEGATE_B | 可能（新反题顶点） | 是 | negate_b 可能创建新反题 |
| NEGATE_A | 否 | 否 | 不创建新顶点，只翻转已有边 |
| FOLD | 否（删除顶点） | 否 | fold 合并顶点，不创建新顶点 |
| bridge vertex | 是 | 否（默认 non_critical） | S_net 物质层引入，不改变概念层拓扑 |

## 边界条件

| 条件 | 当前状态 | 翻转阈值 |
|------|---------|---------|
| D 的语义有效性 | D==0 意味着环路结构不变——但不意味着顶点"不重要" | 如果发现 D==0 的顶点后续参与了关键 fold/sublation，non_critical 标记可能需要动态更新 |
| β₁ 的计算成本 | compute_beta_1 已在 run_step 中每步计算（用于 terrain），此处复用 sublate/negate 内的 beta_before/beta_after | 无额外性能开销 |
| non_critical 标记的不可变性 | 一旦标记 non_critical=True，不会被后续操作修改 | 如果顶点的拓扑角色随穿越进展改变（如后续被更多边连接形成新环），可能需要重新评估 |

## 与 370号的关系

370号声明 Morse 理论的有效域**精确终止于**关键性判断——不能用 Morse 跳跃幅度**预测**哪些否定将是关键的。473号不违反此边界：

- 370号否定的是：用 β₁ 跳跃**预测**未来哪些否定"将是关键的"
- 473号做的是：在操作**已发生后**记录 D 值——这是结构标记，不是预测
- 类比：370号说"不能从地震烈度预测哪些地震将改变政治格局"。473号说"每次地震后记录烈度"——记录不等于预测

## 下游推论

1. non_critical 顶点可以在未来的 fold 优先级中降权——如果需要选择合并对象，优先合并 non_critical 顶点（因为它们不改变环路结构，合并代价更低）
2. critical 标记可以作为穿越轨迹分析的信号——TrajectoryCluster 中 critical 操作的分布模式可能揭示穿越的"关键路径"
3. daemon.py 的日志标注使 CRITICAL/non-critical 在运行时可观测，便于 L2 验证

## 影响声明

- **新增/修改文件**: engine.py（Vertex 字段）、traversal.py（StepLog 字段 + execute_encounter 逻辑 + bridge vertex）、daemon.py（日志格式）
- **无破坏性变更**: non_critical 默认 False，所有现有顶点保持 critical（保守默认）
- **测试**: test_engine.py 26 测试全部通过
- **谱系引用修正**: 代码注释从错误的 472号 修正为 473号
