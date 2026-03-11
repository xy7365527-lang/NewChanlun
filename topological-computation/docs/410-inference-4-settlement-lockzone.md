# 410号推论4：settlement 锁区缩小方案

## 状态：方案A已实装（2026-03-11），方案B/C推论（未实装）

## 诊断

### 当前锁区机制

`would_destroy_settled()` 的阻塞判据：操作后 graph 中，某个 settled cycle 的**任一边**消失 → 操作被 blocked。

唯一的例外（396号 residue exception）：如果操作涉及的所有顶点都是 residue 顶点（`operation_vertices.issubset(residue_vids)`），则允许。

### 锁区过大的三个来源

**来源1：negate 不会删除边，但会改变顶点状态**

`negate()` 只添加 NEGATION 边和改变 thesis 状态为 CONTESTED。它**不删除任何边**。因此 `negate` 理论上不应该破坏任何 settled cycle 的边集。

但 `would_destroy_settled` 检查的是 `graph_after.active_edges()`，而 `active_edges()` 过滤条件是两端顶点都必须 non-FOLDED。`negate` 不 fold 任何顶点，所以它**不可能让 settled cycle 的边消失**。

然而实际观测到 negate 被 blocked 171,338 次。这意味着阻塞来源不是 negate 本身破坏了 cycle，而是 **ghost settlements**（cycle 的边已经因其他原因不存在，但 settlement 仍在 `_settled` 列表中）。

→ 410号 ghost purge 修复 + 本次 checkpoint 清除已解决此问题。

**来源2：fold 的 merge 导致边重定向，间接破坏 cycle**

`fold()` 将多个顶点合并为一个。合并时，所有指向被合并顶点的边都重定向到保留顶点。如果 settled cycle 的某条边是 A→B，fold 将 B 合并到 C，那么边变为 A→C，原边 A→B 消失 → cycle 被认为破坏。

这是**合理的锁区**——fold 确实改变了 cycle 的边结构。但可以更精确：如果重定向后的边集仍然构成 cycle（同构），则不应被视为"破坏"。

**来源3：residue exception 的判据过严**

当前 residue exception 要求 `operation_vertices.issubset(residue_vids)`——即操作涉及的**所有**顶点都必须是 residue 顶点。但 negate 的 `operation_vertices` 包含 thesis 和 antithesis 两个顶点。如果其中任一个不在 residue 集合中，exception 不触发，操作被 blocked。

而 residue_vertices 的范围取决于 residue item 引用了哪些顶点——expression_pressure 引用 cycle 内所有顶点，boundary_edge 引用 cycle 内外各一个。如果某个 settled cycle 没有与其他 cycle 共享顶点（无 edge/tension/nachtraeglichkeit residue），那么只有 cycle 自身的顶点和 boundary edge 的外部顶点在 residue 集合中。

## 缩小方案

### 方案A：操作类型感知的锁区（推荐）

核心思想：`negate` 只添加边，不删除/重定向任何边。因此 negate **永远不会破坏 settled cycle 的边集**（除了 ghost 情况，已由 410号 修复）。

实装方式：
- `would_destroy_settled` 增加 `operation_type` 参数
- 当 `operation_type == "negate"` 时，跳过整个检查（negate 不可能删边）
- fold 和 sublate 保持现有检查

代码位置：`engine.py:negate()` 中 `violated = settlement.would_destroy_settled(...)` 可以直接移除，因为 negate 的 result_graph 只比 graph 多了边和状态变化，active_edges 只增不减。

优点：零误阻塞（对 negate 而言）、无需修改 settlement 结构。
风险：需要确认 negate 确实不会导致 active_edges 减少（已由代码分析确认）。

### 方案B：cycle 同构检测

核心思想：如果操作后 settled cycle 的边集虽然字面不同但拓扑同构（如 fold 将 A→B 变为 A→C，但 C 是 B 的合并目标），则不视为破坏。

实装方式：
- 在 fold 操作中，记录 merge 映射（remove→keep）
- `would_destroy_settled` 检查时，将 settled cycle 的边集通过 merge 映射转换，检查转换后是否仍构成 cycle

优点：减少 fold 的误阻塞。
风险：实现复杂度较高，需要追踪 merge 映射链。

### 方案C：per-cycle residue exception

核心思想：当前 residue exception 检查全局 `residue_vids`，应改为 per-cycle 检查——操作涉及的顶点是否是**被阻塞的那个 cycle** 的 residue 顶点。

实装方式：
- 在 for-loop 内部计算 per-cycle residue vertices，而不是全局的 `residue_vertices()`
- 只要操作顶点是**当前 cycle** 的 residue，就允许

代码位置：`engine.py:702-712` 的 for 循环内。

优点：更精确的 residue 判断。
风险：行为变化需要重新验证 residue 语义。

## 推荐优先级

1. **方案A**（立即可做）：negate 免检。这消除了 negate 100% blocked 的根因（在 ghost 修复之外的剩余阻塞）
2. **方案C**（中期）：per-cycle residue exception，减少 fold/sublate 的误阻塞
3. **方案B**（长期）：fold 同构检测，最精确但实现复杂

## 谱系依据

- 410号：negate 100% blocked 的首次发现
- 396号：settlement as transformation（residue 机制的来源）
- 401号：encounter memory purge → ghost settlement 的第一个来源

## 边界条件

- 如果未来 negate 的实现改为会删除边（当前不会），方案A 需要重新评估
- per-cycle residue 可能改变 residue 的语义范围——需要确认 396号 的原始意图是全局还是 per-cycle
