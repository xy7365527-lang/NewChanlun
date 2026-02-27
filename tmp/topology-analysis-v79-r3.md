# Block-Topology 冷读分析报告

**工位**: topology-analyst (v79-swarm, round 3)
**日期**: 2026-02-27
**范围**: `.chanlun/block-topology/` 全量 258 区块 + 1084 行关系

---

## 1. 基础统计

| 指标 | 值 |
|------|-----|
| 区块文件数 | 258 (meta 声明) / 259 (实际文件，含 genesis) |
| meta 声明关系数 | 1069 |
| relations.jsonl 行数 | 1084 |
| 解析后有效边数 | 1091 (含 to_genealogy 展开) |
| 关系中涉及的唯一节点数 | 252 |
| 连通分量数 | 15 |

**meta 声明 vs 实际行数差异**: meta 声明 1069 条关系，实际 1084 行。差异 15 行来自后续追加的异构 schema 边（见 Section 5）。

## 2. 区块类型分布

| 类型 | 数量 | 备注 |
|------|------|------|
| event | 223 | 主体（迁移+新增） |
| rewrite | 14 | 关系追加操作 |
| consensus | 4 | 质询收敛产物 |
| tension | 4 | 未解决张力 |
| residue | 4 | 共识残余 |
| 概念发现 | 3 | 中文 type |
| unknown | 2 | 227, 228 号（type 为空） |
| meta-rule | 2 | 217, 219 号 |
| concept-discovery | 1 | 英文 type |
| 概念発見 | 1 | 日文 type（213号） |
| 语法记录 | 1 | 218号 |

**来源分布**: migration 187, cc 63, ceremony-fix 6, ceremony 3

**状态分布**: 已结算 191, settled 14, 已结算（附张力）4, no-status 50

## 3. 关系类型分布

| 关系 | 数量 | 占比 |
|------|------|------|
| depends_on | 554 | 50.8% |
| related | 457 | 41.9% |
| tensions_with | 31 | 2.8% |
| records | 14 | 1.3% |
| negates | 13 | 1.2% |
| negated_by | 13 | 1.2% |
| splits | 5 | 0.5% |
| residue_of | 4 | 0.4% |

## 4. 图结构分析

### 4.1 连通分量

| 分量 | 节点数 | 内容 |
|------|--------|------|
| 主分量 | 228 | 001-227 号谱系主链 |
| 分量 2 | 16 | 质询/共识/残余子图（tension + residue + consensus + rewrite 区块） |
| 分量 3 | 5 | 第二个质询子图（同构） |
| 孤立节点 | 12 个 | 191, 192, 198, 201, 206, 207, 209, 210, 224, 225, genesis, 9cb... |

### 4.2 Hub 节点（高度数中心）

**按入度（被依赖最多）**:

| 节点 | 入度 | 出度 | 角色 |
|------|------|------|------|
| **069** | 36 | 23 | 递归拓扑异步自指蜂群 — **全图最大 hub** |
| **020** | 26 | 19 | 阻断等待 |
| **093** | 20 | 8 | |
| **090** | 19 | 7 | 严格性语法规则 |
| **041** | 18 | 5 | |
| **005b** | 18 | 17 | 对象否定对象 |
| **089** | 18 | 14 | 扬弃 |
| **016** | 15 | 17 | |

**按出度（依赖最多）**:

| 节点 | 出度 | 入度 |
|------|------|------|
| **069** | 23 | 36 |
| **020** | 19 | 26 |
| **005b** | 17 | 18 |
| **016** | 17 | 15 |
| **058** | 15 | 11 |
| **075** | 15 | 12 |

### 4.3 根节点（无入边，42 个）

谱系中段及后段的区块大量作为根节点出现（106-128, 160, 180, 190, 200, 203, 213-228），说明这些区块的依赖关系**未被迁移或建立**。

特别注意：106-128 号区间有 14 个根节点，占全部根节点的 33%。这些不太可能是真正的独立起点——更可能是迁移时遗漏了依赖关系。

### 4.4 叶节点（无出边，13 个）

026, 181, 182, 193, 194, 197, 208, 224, 225, 226 + 3 个非谱系节点

### 4.5 孤立区块（无任何关系边，9 个）

191, 192, 198, 201, 206, 207, 209, 210 + genesis block

其中 191, 192 和 198, 201, 206, 207, 209, 210 均为近期区块，可能是新写入但关系尚未建立。

## 5. 结构异常（奇点候选）

### 5.1 Schema 异构性（11 种 schema）

relations.jsonl 中存在 **11 种不同的 key schema**：

| Schema | 数量 | 特征 |
|--------|------|------|
| 标准 6 键 (from/to/relation/order/created_by/timestamp) | 1019 | 迁移产物，规范格式 |
| 简化 3 键 (from/to/type) | 27 | cc 来源，缺少 metadata |
| 仅 target_genealogy_id + type | 13 | **无 from**，孤儿边 |
| from/source/to/type | 10 | 带 source 标记 |
| 带 scope | 4 | negates 关系 |
| 带 genealogy_from/genealogy_to | 4 | 双重标识 |
| 带 valid_until | 3 | 时效性边 |
| 带 settled_by + valid_until | 3 | 已结算的时效性边 |
| 仅 target + relation + order | 2 | **无 from**，孤儿边 |
| from + to_genealogy (array) + relation | 1 | 一对多 |
| from + to_genealogy (array) + type | 1 | 一对多（type 键名变体） |

**严重性**: 这不是简单的格式差异。13 条 `target_genealogy_id` + 2 条 `target` 边**完全没有 from 字段**，这意味着 15 条依赖关系的来源不可知。这些孤儿边指向 137, 143, 178, 181, 218, 224, 225, 057, 173 号。

### 5.2 幻影节点（在关系中被引用但无区块文件）

| 节点 | 问题 |
|------|------|
| **194** | hash 仅 7 字符（`0551915`），明显被截断。无区块文件 |
| **215** | hash 完整（64 字符），但区块文件不存在 |

### 5.3 截断 Hash（10 个）

| 谱系 ID | Hash 长度 | 标准长度 |
|---------|-----------|---------|
| 194 | 7 | 64 |
| 204-212 | 16 | 64 |

204-212 号区块的 hash 都是 16 字符，且区块 content 全部为 `null`。这些是**骨架区块**——只有 id/type/timestamp，没有实质内容。

### 5.4 Type 命名不一致

同一概念使用了三种语言：
- `概念发现` (中文简体, 3 个)
- `概念発見` (日文, 1 个: 213号)
- `concept-discovery` (英文, 1 个: 220号)

227, 228 号的 type 字段**为空字符串**。

### 5.5 Status 命名不一致

- `已结算` (191 个) vs `settled` (14 个) — 同一语义的中英文混用
- `已结算（附张力）` (4 个) — 第三种状态

### 5.6 张力积累（4 个 tension 区块，37 个未解决项）

**tension 区块 001912414a528b13**: 9 个未解决项
- counting_anchor_ambiguity, definition_anchor_collapse, dual_baseline_counting, extreme_value_tie_breaker, extremum_matching_logic, kline_count_metric, lesson81_anchor_ambiguity, new_bi_anchor_ambiguity, new_old_bi_compatibility

**tension 区块 b9929ddfe6805788**: 18 个未解决项
- bi_definition_math_conflict, bi_destruction_logic, bi_kline_count_math_conflict, bi_length_math_contradiction, bi_min_kline_constraint, five_bar_constraint_math_deadlock, 等

**tension 区块 0342207cc0e0c236**: 8 个未解决项
- bi_direction_dependency, bi_greedy_matching, bi_internal_extremum, code_audit_feasibility, extreme_value_constraint, independent_kline_ambiguity, initial_direction_deadlock, missing_code_context

**tension 区块 45fcaeaa501d862a**: 0 个未解决项（空 tension）

这些 tension 全部与**笔的形式化定义**相关，形成一个独立的质询子图（分量 2）。

### 5.7 Genesis Block 异常

Genesis block (`ef30b2e00b40150c...`) 存在于 blocks/ 目录中，出现在 relations.jsonl 的 `created_by` 字段中（作为所有迁移边的创建者），但**不在 id_mapping 中**（没有谱系 ID）。它同时是一个孤立节点（不参与 from/to 关系）。

### 5.8 极端吸收/发射节点

- **极端吸收**: 182 号（入度 5，出度 0）—— 只被依赖，不依赖任何节点
- **极端发射**: 106 号（入度 0，出度 8）、117 号（入度 0，出度 7）、112 号（入度 0，出度 6）—— 这些高出度根节点不被任何其他节点依赖，但自身依赖 6-8 个节点，说明它们是**依赖链的起点但自身悬空**

### 5.9 Negation 结构

- 073b 号被 negated 2 次（最多）
- 062 号是张力中心（4 条 tensions_with 边）
- Negation 关系（negates + negated_by）共 26 条，但 negates 和 negated_by 的数量相同（各 13），说明每条否定都被双向记录

## 6. 总结：需处理的缺陷清单

### P0（结构性缺陷）

1. **194 号 hash 截断**（7 字符）——导致区块文件不存在（幻影节点）
2. **215 号无区块文件**——hash 完整但文件缺失（幻影节点）
3. **15 条孤儿边**——无 from 字段，依赖来源不可知

### P1（一致性缺陷）

4. **11 种 relation schema**——应规范为统一格式
5. **204-212 号骨架区块**——hash 截断（16字符）+ content 为 null
6. **227, 228 号 type 为空**
7. **type 命名三语混用**（概念发现 / 概念発見 / concept-discovery）
8. **status 命名混用**（已结算 / settled）

### P2（完整性缺陷）

9. **42 个根节点**中约 14 个（106-128 区间）可能缺少入边
10. **9 个完全孤立区块**可能缺少关系
11. **张力积累**：37 个未解决的笔定义张力项分布在 3 个 tension 区块中
12. **Genesis block 未纳入 id_mapping**
