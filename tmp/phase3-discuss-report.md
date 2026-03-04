# Phase 3 穿越基础设施方案——Gemini + Codex 并行讨论报告

topo_address: v134-swarm/phase3-discuss
date: 2026-03-02

## 当前数据快照

| 指标 | 实测值 |
|------|--------|
| 区块总数 | 1024 |
| 关系总数 | 5197 |
| depends_on | 1553 |
| references | 1156 |
| defines | 1123 |
| related | 907 |
| records | 300 |
| tensions_with | 51 |
| negates | 29 |
| negated_by | 27 |
| refines | 11 |
| splits | 5 |
| residue_of | 4 |
| 未分类 | 31 |
| depends_on 唯一节点数 | 406 |
| references 唯一节点数 | 296 |
| defines 唯一概念目标 | 997 |

### 拓扑不变量实测

| 子图 | V | E | 连通分量 | cycle_rank (无向) | 有向环 |
|------|---|---|---------|-------------------|--------|
| depends_on | 406 | 1553 | 3 | 1150 | **无**（DAG） |
| references | 296 | 1156 | 2 | 862 | 未检测 |
| Layer 2 全量 | 1603 | 3497 | 2 | 1896 | 未检测 |
| Layer 1 全量 | 423 | 1669 | 3 | 1249 | **有**（含 negates 等非 DAG 边） |

**关键发现**：编排者指令中 "Layer 1 cycle_rank = 0" 指的是**有向**环数（depends_on 是 DAG，无有向环），不是无向 cycle_rank。"Layer 2 cycle_rank = 1143" 与实测 862（仅 references）或 1896（Layer 2 全量）均不完全吻合。差异原因可能是：(1) 计算时数据集版本不同；(2) Layer 分层标准差异；(3) 多重边/自环处理差异。

---

## Challenger A 视角（结构主义 / 数学严格性优先）

### 1. Morse 地形的数学正确性

**核心问题：BFS 生成树 + critical 边 = cycle_rank 是否严格成立？**

**结论：条件性成立，但需要严格限定。**

对于连通无向图 G = (V, E)，任何生成树 T 将边分为树边（|V|-1 条）和非树边（|E|-|V|+1 条）。每条非树边恰好与 T 中唯一路径构成一个基本环（fundamental cycle）。因此：

- critical 边数 = |E| - |V| + 1（连通图）
- cycle_rank = E - V + C（一般图，C = 连通分量数）
- 对连通图：critical 边数 = cycle_rank

**这在无向图上严格成立。**

**但方案处理的是有向图。** references 关系是有向的（A references B ≠ B references A）。有向图的 cycle rank（即第一 Betti 数）需要考虑底层无向图。具体而言：

1. 将有向图视为无向图（忽略方向）计算 cycle_rank → 这是方案的隐含假设
2. 但有向图可能存在多重边（A→B 和 B→A 计为两条无向边）→ 需要处理
3. BFS 在有向图上的行为：时序 BFS 需要明确是沿有向边还是无向边遍历

**建议**：
- 明确声明 "cycle_rank 基于无向化后的图"
- 定义多重边处理策略（A→B 和 B→A 是否算一条无向边还是两条）
- BFS 应沿**无向化**边遍历（否则可能有不可达节点）

### 2. 架构合理性

**依赖链 四→三→一→二 评审：**

- 四（概念注册表）→ 三（Morse 地形）：合理。Morse 标记可以不依赖概念注册表——它只标记 reference 边是否 tree/critical。但概念注册表先做可以让组件一的 `query_block` 返回更丰富的概念信息。
- 三（Morse 地形）→ 一（查询接口）：**严格必要**。query_block 需要返回 tree/critical 标记。
- 一（查询接口）→ 二（偶遇记录）：**逻辑上可并行**。偶遇记录只需要 cycle_rank 变化检测，不需要查询接口。它需要的是 Morse 地形的生成树数据，不是查询 API。

**修正建议**：依赖链应为 四→三→(一,二)。一和二可并行，因为它们的输入都是组件三的输出（Morse 标记集），不互相依赖。

### 3. 偶遇记录触发时机

**问题**："ceremony 写入后自动检测 cycle_rank 变化" ——这意味着每次 ceremony 写入都需要重新计算 cycle_rank 吗？

**增量方案**：每次 ceremony 新增关系时，只需检查新增的 reference 边在当前生成树下是否为 critical 边——这是 O(路径长度) 的操作，不需要重算全局 cycle_rank。

但 "cycle_rank 是否变化" 和 "新边是否 critical" 不完全等价。新边是 critical 当且仅当它连接的两个节点在当前生成树中不在同一路径上（即形成新环）。对于无向图，每条 critical 边 **定义** 上就是非树边，因此：

- 新增一条 reference 边
- 查找两端节点在生成树中的 LCA
- 如果两端在同一连通分量中 → 该边是 critical → cycle_rank += 1
- 如果两端不在同一连通分量中 → 该边是 tree 边（桥接两个分量）→ cycle_rank 不变

**建议**：偶遇记录使用增量检测，避免全量重算。

### 4. 折叠区块格式

**格式充分性评审：**

- `event_type: folding` — 足够
- `fold_from/fold_to` — 携带 block + concept 是足够的
- `fold_tension` — 一句话描述不兼容性，可能不够。建议增加 `tension_type` 枚举（语义冲突/适用域冲突/粒度冲突等）
- 正文三段（穿越路径/同一性判断/张力）— 结构良好

**缺失项**：折叠区块与两端的 references 边是否应该标记为 critical？如果折叠区块连接了已存在于图中的两端，这些 references 边大概率是 critical（形成新环路）。折叠区块本身就是拓扑事件。

### 5. 遗漏与冗余

**可能遗漏**：
- **id_mapping**：relations.jsonl 使用 SHA256 block_id，但谱系文件使用 NNN 号编号。查询接口需要一个 block_id ↔ genealogy_number 的映射层。当前 `content_enrichment_migration.py` 可能已建立这个映射，但 traverse.py 需要暴露它。
- **反向索引**：`query_concept` 需要一个 concept_id → block_id 的反向索引。当前 defines 关系分散在 relations.jsonl 中，全量扫描 5197 行不高效。建议导出为独立索引文件。

**可能冗余**：
- 组件四（概念注册表导出）如果只是导出 JSON，可以是组件一的一个子命令而非独立组件。

---

## Challenger B 视角（工程实用性 / 增量实现优先）

### 1. 架构合理性

**数据流评审：**

当前基础设施状态：
- `block_topology.py`：已有 SHA256 区块写入/读取、关系追加
- `concept_extractor.py`：已有概念提取（section/definition/reference）
- `concept_topology_check.py`：已有概念去重/矛盾检测
- `relations.jsonl`：5197 条关系，已有 Layer 1/Layer 2 分层（通过关系类型隐式分层，非显式 layer 字段）

**关键缺口**：relations.jsonl 中没有 `layer` 字段。Layer 分层是通过关系类型推断的（depends_on → Layer 1, references → Layer 2），但这个推断逻辑在任何现有脚本中都没有被编码为公共函数。traverse.py 需要这个分层函数。

**建议**：在 `block_topology.py` 中增加 `classify_layer(relation_type: str) -> int` 函数，作为分层的单一真相源（single source of truth）。

### 2. Morse 地形实现

**实现路径评审：**

BFS 生成树的选择标准是 "时序 BFS（早期引用倾向于树边，后期引用倾向于临界边）"。这需要：
1. 每条 reference 边有时间戳 → 已有（relations.jsonl 的 timestamp 字段）
2. BFS 起点是最早区块 → 需要 block 创建时间排序
3. BFS 遍历顺序按时间戳 → 边按 timestamp 排序后按序加入

**更精确的描述**：这实际上是 **Kruskal 算法的变体**——按时间戳排序所有 reference 边，从早到晚加入，用 Union-Find 检测环。树边 = 第一次连接两个分量的边；critical 边 = 两端已在同一分量中的边。

这比 BFS 更自然、更正确、更高效：
- BFS 需要选择起点和遍历策略 → 不同起点可能产生不同生成树
- Kruskal 只依赖边排序 → 确定性结果
- Union-Find 是 O(α(n)) per operation → 总复杂度 O(E log E)

**建议**：将 "时序 BFS" 替换为 "时序 Kruskal"（按 timestamp 排序的最小/最早生成森林）。

### 3. 偶遇记录的集成方式

**触发点评审：**

"ceremony 写入后自动检测" ——具体在哪个代码路径触发？

当前 ceremony 写入流程：
1. 谱系 Markdown 写入 `.chanlun/genealogy/settled/`
2. `block_topology.py` 的 `create_block()` 写入区块
3. `add_relation()` 写入关系
4. ceremony_scan.py 执行后续扫描

**建议集成点**：在 `add_relation()` 函数末尾，如果新增关系类型是 `references`，调用偶遇检测函数。这样偶遇检测与写入原子绑定——不需要额外的 post-hook。

**Layer 1 环检测**（严重告警）：当前 depends_on 是 DAG，但如果 ceremony 写入引入了 depends_on 环路，这是逻辑层拓扑事件。**建议**：在 `add_relation()` 中，如果关系类型是 `depends_on`，执行增量环检测（插入边后检查是否形成有向环）。这是 O(V) 最坏情况，但由于是增量操作（每次只加一条边），实际很快。

### 4. 折叠区块格式

**工程评审：**

折叠区块需要同时修改：
1. `BLOCK_TYPES` — 已包含 "event"（可复用，加 event_type 字段）
2. block 创建函数 — 需要支持 frontmatter 新增字段（event_type, fold_from, fold_to, fold_tension）
3. 关系写入 — 折叠区块自动创建两条 references 边

**Schema 建议**：
```python
# block_topology.py 中新增
FOLD_SCHEMA = {
    "event_type": "folding",
    "fold_from": {"block": str, "concept": str},  # block_id or genealogy number
    "fold_to": {"block": str, "concept": str},
    "fold_tension": str,
}
```

**问题**：fold_from/fold_to 的 block 字段用 block_id（SHA256）还是谱系编号（NNN）？建议用 block_id（内部一致性），但在 traverse.py 查询输出中同时显示 block_id 和谱系编号。

### 5. 遗漏与风险

**实现优先级建议**：

1. **P0（阻塞其他组件）**：`classify_layer()` 函数 + block_id ↔ genealogy_number 映射
2. **P1（核心功能）**：Morse 标记（时序 Kruskal） + traverse.py CLI
3. **P2（增量功能）**：偶遇记录 + 折叠区块 Schema
4. **P3（辅助功能）**：概念注册表导出 + MCP 接入

**风险点**：
- relations.jsonl 中 31 条无 relation 字段的记录 → 需要清理或忽略
- defines 关系的 `to` 字段指向 concept_id（SHA256 of "concept:term"），不是 block_id → 查询时需要区分
- 997 个唯一概念目标 vs 编排者说的 "15 definitions" → 差异巨大。"15 definitions" 可能指显式 `type: definition` 的区块，而 997 是所有 defines 关系的目标。需要明确概念注册表的范围是哪个。

---

## 共识区域（两路讨论收敛点）

### C1：Morse 地形的数学基础是正确的

BFS/Kruskal 生成树 + 非树边 = critical 边，critical 边数 = cycle_rank（无向图上严格成立）。但需要明确声明这是**无向化**后的结果。

### C2：依赖链应修正为 四→三→(一,二)

一和二可并行。二不依赖一的 API，只依赖三的 Morse 标记数据。

### C3：偶遇记录应与 add_relation() 原子绑定

不需要额外的 post-ceremony hook。在关系写入时增量检测。

### C4：需要前置基础设施

在四个组件之前，需要：
- `classify_layer()` 公共函数
- block_id ↔ genealogy_number 映射（可能已存在但需确认和暴露）
- concept_id → block_id 反向索引

### C5：时序 Kruskal 优于时序 BFS

Kruskal 算法变体是确定性的、更高效的、语义更清晰的选择。

---

## 分歧区域（需编排者裁定）

### D1：cycle_rank 数值不一致

编排者指令中 "Layer 2 cycle_rank = 1143"，实测 references-only = 862，Layer 2 全量 = 1896。这可能是数据版本差异，但如果组件三的 Morse 标记建立在错误的 cycle_rank 期望上，标记结果将不可信。

**建议**：Phase 3 第一步应重新计算当前数据的 graph_invariants 并与编排者确认。

### D2："15 definitions" vs 997 unique concept targets

defines 关系有 1123 条指向 997 个唯一概念 id。但编排者说 "15 definitions"——这两个数字指的是完全不同的东西。概念注册表应导出哪个范围？

### D3：生成树的唯一性

Kruskal 生成树在存在同时间戳边时不唯一（tie-breaking 策略影响结果）。是否需要规定确定性 tie-breaking（如按 from 的 block_id 字典序）？

---

## 认识论等级标注

本报告的验证等级：
- 数据统计（V, E, cycle_rank）：**L2**（真实数据计算，可被否证）
- 数学命题（BFS/Kruskal 与 cycle_rank 等价性）：**L0**（图论定理，纯代数/定义）
- 架构建议（依赖链修正、Kruskal 替代）：**L0**（从需求逻辑推导）
- cycle_rank 数值不一致：**L2**（真实数据发现，否定性结果——预期值与实测值不匹配）
