# Phase 3 穿越基础设施——Round 2 讨论（编排者裁定后）

## 模式：discuss（技术细化，不是质询）

Round 1 讨论已完成，编排者已对 3 项分歧做出裁定。本轮基于确定参数，细化 5 个组件的精确实现规格。

---

## Round 1 共识（已确认，不再讨论）

- **C1**：Morse 地形数学正确（无向化 cycle_rank = 非树边数）
- **C2**：依赖链 四→三→(一,二)
- **C3**：偶遇记录与 add_relation() 原子绑定
- **C4**：前置基础设施（classify_layer + 映射 + 反向索引）
- **C5**：时序 Kruskal 优于时序 BFS

## 编排者 3 项分歧裁定

### D1（cycle_rank 数值）裁定

以实测为准。**Morse 地形基于 references-only 子图构建**（862 条临界边）。

理由：depends_on 是逻辑关系不是导航关系，defines 是标签关系不是穿越关系，related 语义模糊。穿越发生在 references 图上。

### D2（概念注册表范围）裁定

997 个唯一概念目标**全部入注册表**。15 个 type: definition 区块加 authoritative: true 标记，不排除其余 982 个。偶遇催化需要完整覆盖面。

### D3（生成树唯一性）裁定

确定性 tie-breaking。同时间戳边按 (from_block_id, to_block_id) 字典序排序。Morse 标记必须可重现。

### 补充裁定（DAG 守护范围修正）

Layer 1 不再是纯 DAG——negates 等非 DAG 边产生有向环。**DAG 守护只检测 depends_on 子图**（逻辑依赖不能循环）。negates/revises/supersedes 允许有向环——否定和修订的循环是谱系学正常产物。

---

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
| references 连通分量 | 2 |
| references cycle_rank (无向) | 862 |
| defines 唯一概念目标 | 997 |

---

## 5 个讨论焦点（请逐项给出精确实现规格）

### 焦点1：组件三（Morse 地形）精确实现规格

已确定参数：
- 输入子图：references-only（1156 条边，296 个节点，2 个连通分量）
- 算法：时序 Kruskal（Union-Find）
- Tie-breaking：同时间戳边按 (from_block_id, to_block_id) 字典序
- 输出：每条 reference 边标记为 tree 或 critical

请讨论：
1. **输出持久化策略**：边标记集合应该持久化为文件（每次加载），还是每次启动时重算（O(E log E)，1156 条边很快）？持久化的话，文件路径和格式？
2. **无向化处理**：references 是有向边（A references B），Kruskal 时需要无向化。如果 A→B 和 B→A 同时存在（双向引用），算一条无向边还是两条？这影响 cycle_rank 计数。
3. **增量更新**：新增 reference 边时，是否直接在现有 Union-Find 上追加？还是触发全量重算？考虑到数据量小（1156 条边），全量重算可能更简单可靠。
4. **862 条 critical 边的导航策略**：critical 边是穿越的关键资源——每条 critical 边代表一个独立的概念环路。如何在 traverse.py 中暴露这些信息？按 cycle 分组？按概念聚类？
5. **函数签名建议**：`build_morse_landscape(relations: list[dict]) -> MorseLandscape`，MorseLandscape 数据结构应包含哪些字段？

### 焦点2：组件一（traverse.py）接口精确规格

已确定前置：Morse 地形标记可用，概念注册表可用。

请讨论：
1. **query_block(block_id) 返回结构**：
   - 基本信息（genealogy_number, title, layer, status）
   - 出边列表（按关系类型分组，标注 tree/critical）
   - 入边列表
   - 所属概念列表（defines 关系反向）
   - 是否需要返回 Morse 相关信息（该区块涉及多少 critical 边？在哪些 cycle 上？）

2. **query_pair(from_id, to_id) 最短路径**：
   - 在哪个子图上找路径？references-only？还是全量？
   - 算法选择：BFS（无权图）vs Dijkstra（带权——但权重是什么？）
   - 路径输出格式：节点序列 + 边类型序列？

3. **query_concept(concept_id) 反向索引**：
   - 返回所有 defines 该概念的区块列表
   - authoritative 标记（15 个 type: definition 区块优先显示）
   - 该概念参与的 references 边（作为 from 或 to）

4. **CLI 输出格式**：JSON 输出（机器可读）+ 人类可读表格（默认）？--format json/table 参数？

5. **错误处理**：
   - block_id 不存在 → 返回什么？exit code？
   - concept 无 defines 记录 → 空列表还是警告？
   - 路径不可达 → 明确返回 "unreachable"

### 焦点3：组件二（偶遇记录）增量检测

已确定：偶遇检测与 add_relation() 原子绑定。

请讨论：
1. **Union-Find 状态管理**：add_relation() 每次调用时，Union-Find 从哪里加载？
   - 方案A：每次 add_relation() 重建 Union-Find（O(E * alpha(V))，1156 条边很快）
   - 方案B：维护持久化的 Union-Find 状态文件（增量 O(alpha(V))，但需要状态同步）
   - 方案C：进程内缓存（仅在同一 ceremony 内有效）

2. **topo_event 标记写入位置**：
   - 方案A：谱系区块 frontmatter 中增加 `topo_events` 字段
   - 方案B：独立文件 `.chanlun/topo_events.jsonl`（事件日志）
   - 方案C：在 relations.jsonl 的关系记录中增加 `is_critical` 字段

3. **depends_on DAG 守护**：
   - 在 add_relation() 中，如果关系类型是 depends_on，执行增量环检测
   - 算法：插入边 (u, v) 后，从 v 出发 DFS/BFS 检查是否可达 u——可达则形成有向环
   - 复杂度：O(V) 最坏情况，但增量操作实际很快
   - 检测到环时的行为：拒绝插入？还是插入并标记警告？

4. **偶遇事件的语义**：新增 reference 边被标记为 critical 意味着什么？
   - 它连接了两个已经在同一连通分量中的节点 → 形成新的概念环路
   - 这是一次"偶遇"——两个概念路径意外相遇
   - 该事件应该触发什么下游动作？（仅记录？提醒用户？自动生成折叠区块？）

### 焦点4：组件四（概念注册表）导出格式

已确定：997 个概念全部入表，15 个 authoritative。

请讨论：
1. **输出文件格式**：
   - JSON：`{ "concept_id": { "term": "...", "defining_blocks": [...], "authoritative": bool, "reference_count": N } }`
   - JSONL：每行一个概念记录
   - YAML：可读性更好但解析更慢

2. **文件路径**：`.chanlun/concept_registry.json`？还是 `data/concept_registry.json`？

3. **更新策略**：
   - 每次 ceremony 后全量重导？
   - 增量更新（新增 defines 关系时追加）？
   - 两者各自的一致性风险

4. **authoritative 标记的确定方式**：
   - 从哪里获取 type: definition 信息？block_topology 的区块元数据？
   - 如果区块类型字段缺失怎么办？

5. **concept_id 的人类可读性**：
   - 当前 concept_id 是 SHA256 of "concept:term"
   - 注册表是否应该包含原始 term 字段以供人类查看？

### 焦点5：前置基础设施

请讨论：
1. **classify_layer() 函数**：
   - 签名：`classify_layer(relation_type: str) -> int`
   - 放置位置：`block_topology.py` 中
   - 映射规则：
     - Layer 1（逻辑层）：depends_on, negates, negated_by, supersedes, residue_of, reopens, tensions_with, freezes, splits, severs
     - Layer 2（导航层）：references, defines, modifies, refines, revises, annotates
     - Layer 3（元数据层）：records, related
   - 未知类型的处理：raise ValueError？还是归入默认层？

2. **block_id <-> genealogy_number 映射**：
   - 数据源：block_topology.jsonl 的每个区块记录都有 block_id（SHA256）和 genealogy_number
   - 实现方式：启动时全量扫描 block_topology.jsonl 构建双向 dict？
   - 缓存策略：进程级缓存，ceremony 期间不失效
   - 函数签名：`get_block_mapping() -> tuple[dict[str, int], dict[int, str]]`

3. **concept_id -> block_id 反向索引**：
   - 数据源：relations.jsonl 中 relation="defines" 的记录
   - 实现方式：启动时全量扫描构建 `dict[str, list[str]]`
   - 或者：概念注册表导出后直接从注册表读取
   - 与组件四（概念注册表）的关系：这个反向索引是注册表的子集还是独立数据结构？

---

## 代码参考

### block_topology.py 关键结构

```python
RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",
    "supersedes", "residue_of", "reopens",
    "freezes", "splits", "severs",
    "records", "negated_by",
    "defines", "modifies", "references",
    "refines", "revises", "annotates",
})
```

### 现有数据文件

- `data/block_topology.jsonl`：区块记录（block_id, genealogy_number, title, type, layer, ...）
- `data/relations.jsonl`：关系记录（from, to, relation, timestamp, ...）
- `data/graph_invariants.json`：图不变量缓存

---

## 回复要求

1. 逐焦点讨论，给出具体的实现建议（函数签名、数据结构、算法选择）
2. 对每个方案选项给出利弊分析
3. 标注你不确定的地方和需要进一步确认的假设
4. 用中文回复，代码示例用 Python
5. 产出控制在 8KB 以内
