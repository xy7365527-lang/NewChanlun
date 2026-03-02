---
id: '312'
number: 312
type: consensus
title: "Phase 3 穿越基础设施裁定——Gemini+Codex 讨论 5 共识确认 + 3 分歧裁定 + 1 补充裁定"
date: "2026-03-02"
depends_on: ['311', '309', '273']
status: 已结算
---

# 312号：Phase 3 穿越基础设施裁定——5 共识确认 + 3 分歧裁定 + 1 补充裁定

## 递归判断

任务不可分解：单一谱系写入任务，内容是编排者对 Gemini+Codex Phase 3 讨论报告的逐条裁定。扁平退化特例。

## 背景

311号记录了 Phase 2→Phase 3 的方向变更：flag complex 被取消，替代为穿越基础设施（查询接口 + 偶遇记录 + Morse 地形 + 概念注册表）。本号记录编排者对 Gemini+Codex 关于 Phase 3 穿越基础设施方案讨论报告的完整裁定。

讨论参与方：
- Gemini：challenger，提出方案评审意见
- Codex：reviewer，提供交叉评审
- 编排者：最终裁定

## 共识确认（5项）

编排者确认以下 5 项 Gemini+Codex 共识：

### C1：Morse 地形数学基础正确

无向化 cycle_rank = 非树边数。Morse 地形基于此数学基础构建。数学层面无争议。

### C2：依赖链修正为 四→三→(一,二)

四个穿越基础设施组件的实现依赖链：
- 组件四（概念注册表）最先实现
- 组件三（Morse 地形）依赖组件四
- 组件一（查询接口）和组件二（偶遇记录）依赖组件三，且彼此独立可并行

### C3：偶遇记录与 add_relation() 原子绑定

偶遇记录的写入与 add_relation() 调用原子绑定——每次添加关系时同时写入偶遇记录。不是事后批量补录。

### C4：前置基础设施

classify_layer（关系分层函数）+ 层间映射 + 反向索引是 Phase 3 的前置基础设施。这些在 Morse 地形构建之前必须就绪。

### C5：时序 Kruskal 优于时序 BFS

Morse 地形的生成树构建算法选择时序 Kruskal（按时间戳排序的最小生成树），优于时序 BFS。理由：Kruskal 天然尊重时间序（边按时间戳排序后依次考虑），且产出的生成树更稳定（同一数据多次运行结果一致，前提是 tie-breaking 确定性）。

## 分歧裁定（3项）

### D1：cycle_rank 数值不一致

**分歧**：讨论中出现多个 cycle_rank 数值（早期 1143、references-only 862、Layer 2 全量 1896），Gemini 和 Codex 对使用哪个数值有不同理解。

**裁定**：数据版本差异。早期 1143 是旧数据，现在以实测为准。

- references-only cycle_rank = 862
- Layer 2 全量 cycle_rank = 1896

**Phase 3 Morse 地形基于 references-only 子图构建（862 条临界边）**。

不用 Layer 2 全量的理由：
- `depends_on` 是逻辑关系不是导航关系
- `defines` 是标签关系不是穿越关系
- `related` 语义模糊
- 穿越发生在 references 图上，Morse 地形只标记 references 边

### D2：概念注册表范围

**分歧**：概念注册表应该只包含 15 个 `type: definition` 高权重概念，还是包含全部 997 个唯一概念目标。

**裁定**：997 个唯一概念目标全部入注册表。15 个 `type: definition` 区块加 `authoritative: true` 标记，不排除其余 982 个。

理由：偶遇催化需要完整概念词覆盖面。仅 15 个概念的注册表过于稀疏，无法支撑偶遇记录的触发频率。

### D3：生成树唯一性

**分歧**：时序 Kruskal 在同时间戳边上的排序不确定，可能导致不同运行产出不同的生成树。

**裁定**：需要确定性 tie-breaking。同时间戳边按 `(from_block_id, to_block_id)` 字典序排序。这保证 Morse 标记可重现——同一数据集任意次运行产出相同的生成树和相同的临界边集合。

## 补充裁定：DAG 守护范围修正

**背景**：Layer 1 原来被假设为纯 DAG，但 `negates` 等关系类型（B 否定 A，A 否定 B 的循环结构）产生有向环。

**裁定**：Layer 1 不再是纯 DAG。DAG 守护检测范围修正为：**只检测 `depends_on` 子图是否为 DAG**。

具体规则：
- `depends_on` 子图必须是 DAG（逻辑依赖不能循环——A 依赖 B 且 B 依赖 A 是矛盾）
- `negates`/`revises`/`supersedes` 允许形成有向环——否定和修订的循环结构是谱系学的正常产物（例如：A 否定 B 的某方面，B 的后续修订又否定 A 的某方面）

这是对 309号关系分层的下游修正：309号将 `negates`/`revises`/`supersedes` 与 `depends_on` 归入同一个 Layer 1，但 DAG 约束只适用于 `depends_on` 子图。

## 下游推论

### 推论1：Morse 地形实现参数已确定

- 输入子图：references-only（不含 depends_on/defines/related 等）
- 算法：时序 Kruskal
- 临界边数量：862（当前数据）
- tie-breaking：同时间戳边按 (from_block_id, to_block_id) 字典序排序
- 生成树唯一且可重现

### 推论2：概念注册表范围确定

- 全量：997 个唯一概念目标
- 高权重标记：15 个 `type: definition` 区块加 `authoritative: true`
- 偶遇催化基于完整注册表触发

### 推论3：DAG 守护范围缩减到 depends_on 子图

- 原范围：Layer 1 全部关系（depends_on + negates + revises + supersedes）
- 新范围：仅 `depends_on` 子图
- `negates`/`revises`/`supersedes` 允许有向环
- 这不改变 309号的 Layer 1/Layer 2 分层——分层是关系语义分类，DAG 约束是拓扑约束，两者正交

### 推论4：四个组件实现顺序确定

依赖链 四→三→(一,二)：
1. 概念注册表（组件四）——无前置依赖，最先实现
2. Morse 地形（组件三）——依赖概念注册表 + classify_layer + 反向索引
3. 查询接口（组件一）和偶遇记录（组件二）——依赖 Morse 地形，彼此独立可并行

## downstream_actions

### 推论1-1：输入子图 references-only — **verified**

代码验证：`scripts/morse_landscape.py:112` — `if rel.get("relation") == "references"` 只过滤 references 边。不读取 depends_on/defines/related。

### 推论1-2：算法为时序 Kruskal — **verified**

代码验证：`scripts/morse_landscape.py:42-73` — `UnionFind` 类实现 path compression + union by rank。`scripts/morse_landscape.py:116` — 边按 `(timestamp, from, to)` 排序后顺序处理，标准 Kruskal 流程。

### 推论1-3：临界边数量 862 — **verified**

运行验证：`build_morse_landscape(cache_path=None)` 返回 `stats.critical = 862`，`stats.tree = 294`，`stats.edges = 1156`，`stats.nodes = 296`。

### 推论1-4：tie-breaking 按 (from_block_id, to_block_id) 字典序 — **verified**

代码验证：`scripts/morse_landscape.py:116` — `edges.sort(key=lambda e: (e.get("timestamp", ""), e["from"], e["to"]))` 同时间戳下按 from ASC → to ASC 排序。

### 推论1-5：生成树唯一且可重现 — **verified**

运行验证：连续两次调用 `build_morse_landscape(cache_path=None)`，`edge_marks` 和 `stats` 完全一致。确定性排序 + 确定性 UnionFind 保证同一数据集的输出唯一。

### 推论2-1：全量 997 个唯一概念目标 — **verified**

运行验证：`load_registry()` 返回 `len(entries) = 997`。

### 推论2-2：15 个 type: definition 区块加 authoritative: true — **rejected**

运行验证：`sum(e.authoritative for e in entries.values()) = 981`（非 15）。
原因分析：当前代码中 `authoritative` 判定基于 defines 边是否携带非空 `concept_definition` 字段（`scripts/concept_registry.py:95`），实测 1106/1123 条 defines 边携带该字段，导致 981/997 概念被标记 authoritative。区块拓扑中不存在 `type: definition` 的区块（0 个）。312号裁定描述的"15 个 type: definition 区块"在当前数据中不存在——要么是设计意图尚未实现，要么 authoritative 判定逻辑需要修正为基于区块 type 而非边属性。

### 推论2-3：偶遇催化基于完整注册表触发 — **rejected**

代码验证：`scripts/encounter_guard.py:39-70` — `check_encounter()` 调用 `build_morse_landscape()` 全量重算 Morse 地形，根据新边是否为 critical 判断偶遇。偶遇检测不依赖概念注册表——它依赖 Morse 地形的 edge_marks。概念注册表（组件四）和偶遇记录（组件二）是独立组件，偶遇催化基于 Morse 地形重算而非注册表查询。

### 推论3-1：DAG 守护范围仅 depends_on 子图 — **verified**

代码验证：`scripts/encounter_guard.py:119` — `if rel.get("relation") == "depends_on"` 只读取 depends_on 关系构建邻接表。negates/revises/supersedes 不进入 DAG 检测范围。

### 推论3-2：tie-breaking 同推论4 机制 — **verified**

同推论1-4。`scripts/morse_landscape.py:116` 的排序逻辑是唯一的 tie-breaking 实现点。

## 边界条件

1. references-only 子图的 cycle_rank = 862 是当前数据快照。随着谱系增长，新 references 边的加入会改变 cycle_rank 值和 Morse 地形结构
2. 997 概念全量入注册表假设偶遇催化需要广覆盖面。如果后续发现高频噪声概念污染偶遇记录，可能需要引入频率过滤或权重衰减
3. DAG 守护范围缩减到 depends_on 子图后，negates/revises/supersedes 中的环不再被检测。如果某些环是错误数据（如 A depends_on B 被误标为 A negates B），该错误将不被 DAG 守护捕获
4. tie-breaking 依赖 block_id 的字典序稳定性。如果 block_id 生成规则变更，生成树唯一性保证可能失效

## 影响声明

- 本裁定确定了 Phase 3 穿越基础设施的全部实现参数，直接驱动组件实现
- DAG 守护范围修正影响 `scripts/` 下的 DAG 检测脚本（需从 Layer 1 全量缩减到 depends_on 子图）
- 概念注册表的 997 全量 + 15 authoritative 设计影响 concept_extractor.py 的导出格式

## 谱系引用

- **311号**：Phase 2→Phase 3 过渡的元观察（方向变更：flag complex → 穿越基础设施）
- **309号**：Phase 2 异质审查裁定（关系分层 Layer 1/Layer 2 的来源——本号 DAG 守护修正是其下游）
- **273号**：有向图范畴裁定（negates 边的有效性标记——本号允许 negates 成环与 273号的否定拓扑代数一致）
