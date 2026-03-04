# Phase 3 穿越基础设施——Round 2 讨论报告（Gemini + Codex 合并）

topo_address: v134-swarm/phase3-discuss-r2
date: 2026-03-02
models: gemini-2.5-pro (fallback) + o3

---

## 审查方法

- 上下文文件：`tmp/phase3-discuss-r2-ctx.md`（5 焦点，基于编排者裁定的确定参数）
- Gemini：challenge 模式，gemini-2.5-pro（gemini-3.1-pro-preview 503 降级）
- Codex：o3 模型，discuss 模式
- 两路产出独立生成，此报告由 Claude Opus 合并

---

## 焦点1：组件三（Morse 地形）——两路收敛

### 共识项

| 项目 | Gemini | Codex | 收敛 |
|------|--------|-------|------|
| 持久化策略 | 无否定 | 全量重算 + mtime 缓存 | **收敛**：全量重算为主，可选缓存 |
| 增量更新 | 无否定 | 全量重算（<1ms） | **收敛**：全量重算 |
| 数据结构 | 无否定 | `MorseLandscape` dataclass | **收敛** |

### 关键澄清：无向化处理

**Gemini 关键发现**：`862 = 1156 - 296 + 2`，验证通过。这意味着 1156 条有向边直接视为 1156 条无向边（不合并双向引用），规格与实测一致。

**Codex 立场**：建议合并双向引用为一条无向边（`(min(a,b), max(a,b))` 去重），得到 ≤1156 条无向边。

**分歧**：Codex 的合并方案会改变 cycle_rank 计数，与实测 862 不一致。

**裁定建议**：采用 Gemini 的分析——有向边直接视为无向边，不合并。这与实测数据一致。`862 = 1156 - 296 + 2` 是硬约束。

### 收敛的实现规格

```python
@dataclass(frozen=True)
class MorseLandscape:
    edge_marks: dict[str, Literal["tree", "critical"]]  # key = f"{from_id}:{to_id}"
    stats: dict[str, Any]  # {nodes: 296, edges: 1156, critical: 862, components: 2}

def build_morse_landscape(
    relations_path: str = "data/relations.jsonl",
    cache_path: str | None = ".chanlun/morse_landscape.json",
) -> MorseLandscape:
    """时序 Kruskal 构建 Morse 地形。

    - 输入：relations.jsonl 中 relation="references" 的边
    - 边排序：timestamp ASC → (from_block_id, to_block_id) 字典序 tie-breaking
    - 无向化：有向边直接视为无向边（不合并双向引用）
    - Union-Find：tree 边连接两个分量，critical 边两端已在同一分量
    - 缓存：cache_path 存在且 mtime >= relations.jsonl 则反序列化
    """
```

---

## 焦点2：组件一（traverse.py）——存在分歧

### Gemini 矛盾发现（重要）

**query_pair 路径定义与穿越核心目标冲突**：

穿越基础设施的核心是 references-only 子图的 Morse 地形。query_pair 如果允许"逃逸"到全量图，会产生语义误导——全量图路径与 Morse 环路无关。

Dijkstra 讨论是凭空出现的——当前数据模型没有边权重定义。

### Codex 方案

query_pair 默认 references 子图，`--all-relations` 切换全图。`--prefer-tree` 时对 tree 边权 0、critical 权 1 用 Dijkstra。

### 合并裁定建议

Gemini 的批评成立。建议：
1. **query_pair 默认且唯一子图 = references-only**。不提供全量图路径。
2. **删除 Dijkstra 选项**——无权图上 BFS 即可，权重定义未奠基。
3. 如果未来确有全图路径需求，另设独立函数（query_any_path），并明确标注"非穿越路径"。

### 收敛的接口规格

```python
def query_block(block_id: str, morse: MorseLandscape, registry: ConceptRegistry) -> dict:
    """返回区块的完整穿越视图。"""
    # 返回结构（Codex 方案，已采纳）：
    # {block_id, genealogy_number, title, layer, status,
    #  out_edges: {references: [{to, mark}], ...},
    #  in_edges: {...},
    #  concepts: [{concept_id, term, authoritative}],
    #  morse_summary: {critical_out, critical_in, cycle_ids}}

def query_pair(src: str, dst: str) -> dict | None:
    """在 references-only 子图上 BFS 最短路径。"""
    # 返回：{nodes: [...], edges: [...], marks: ["tree","critical",...]}
    # 不可达 → None

def query_concept(concept_id: str, registry: ConceptRegistry) -> dict:
    """从概念注册表查询概念的定义区块和引用。"""
    # 返回：{term, authoritative_blocks, other_blocks, reference_edges}
```

CLI 输出：`--format json/table`，默认 table。错误处理：block 不存在 → exit 1，概念无定义 → 空数组，路径不可达 → unreachable。

---

## 焦点3：组件二（偶遇记录）——存在致命矛盾

### Gemini 矛盾发现（致命）

**Union-Find 双事实来源**：如果组件二维护增量 Union-Find 状态，而组件三使用全量重算，两者会产生状态偏差。

### Codex 方案

推荐方案 A：每次 add_relation() 时重建 Union-Find（全量）。<1ms，无需锁。

### 合并裁定建议

**两路实际已收敛**——Codex 的方案 A（每次重建）就是 Gemini 建议的"纯重算策略"。矛盾点的根源是上下文中同时列出了三种方案（A/B/C），但两路都选择了方案 A。

**最终方案**：纯重算策略。add_relation() 中新增 reference 边时：
1. 加载所有现有 reference 边 + 新边
2. 运行时序 Kruskal 构建完整 Union-Find
3. 检查新边是否为 critical（O(α(n))）
4. 如果 critical → 写入 topo_event

**无状态、无锁、可重现。** 组件二和组件三共享同一算法（时序 Kruskal），只是入口不同。

### 收敛的实现规格

```python
# 在 block_topology.py 的 add_relation() 中：

def add_relation(from_id, to_id, relation, timestamp, ...):
    # ... 现有写入逻辑 ...

    if relation == "references":
        # 偶遇检测：全量重算
        landscape = build_morse_landscape()  # 含新边
        edge_key = f"{from_id}:{to_id}"
        if landscape.edge_marks.get(edge_key) == "critical":
            _record_topo_event(from_id, to_id, timestamp, "new_critical")

    if relation == "depends_on":
        # DAG 守护：增量环检测
        if _creates_cycle(from_id, to_id):
            raise CircularDependencyError(from_id, to_id, cycle_path)
```

**topo_event 写入位置**：`.chanlun/topo_events.jsonl`（append-only 日志，Codex 方案 B，两路一致）。

**depends_on 环检测行为**：Codex 主张硬拒绝（raise）。Gemini 未表态。**裁定建议：硬拒绝**——depends_on 循环是逻辑错误，不是可警告的边界情况。

---

## 焦点4：组件四（概念注册表）——两路收敛

| 项目 | Gemini | Codex | 收敛 |
|------|--------|-------|------|
| 格式 | 无否定 | 单文件 JSON | **收敛** |
| 路径 | 无否定 | `.chanlun/concept_registry.json` | **收敛** |
| 更新策略 | 无否定 | ceremony 后全量重导 | **收敛** |
| authoritative 标记 | 无否定 | block_topology 的 type=definition | **收敛** |
| 人类可读 | 无否定 | term 字段 + SHA-256 id | **收敛** |

### 收敛的导出格式

```json
{
  "<concept_id>": {
    "term": "笔",
    "authoritative": true,
    "defining_blocks": ["<block_id_1>", "<block_id_2>"],
    "reference_count": 37
  }
}
```

997 个条目，15 个 authoritative: true。ceremony 后全量重导。

---

## 焦点5：前置基础设施——存在冗余

### Gemini 矛盾发现（重要）

**concept_id -> block_id 反向索引与概念注册表功能冗余**：注册表的 defining_blocks 字段已经是反向索引。独立的反向索引 = 两个事实来源。

### Codex 方案

从注册表反序列化（`registry[concept_id]["defining_blocks"]`），注册表缺失时回落到全量扫描。

### 合并裁定建议

**Gemini 批评成立，但 Codex 的实现已经是正确的**——Codex 的 load_concept_index 就是从注册表读取，不是独立数据结构。只需明确：

1. **concept_id -> block_id 的唯一事实来源 = 概念注册表**
2. `load_concept_index()` 只是注册表的薄封装，不是独立数据结构
3. 删除"独立反向索引"的讨论——它就是注册表查询

### 收敛的前置基础设施规格

```python
# block_topology.py 中新增

LAYER_MAP: dict[str, int] = {
    # Layer 1（逻辑层）
    "depends_on": 1, "negates": 1, "negated_by": 1, "supersedes": 1,
    "residue_of": 1, "reopens": 1, "tensions_with": 1,
    "freezes": 1, "splits": 1, "severs": 1,
    # Layer 2（导航层）
    "references": 2, "defines": 2, "modifies": 2,
    "refines": 2, "revises": 2, "annotates": 2,
    # Layer 3（元数据层）
    "records": 3, "related": 3,
}

def classify_layer(relation_type: str) -> int:
    """关系类型 → 层编号。未知类型抛 ValueError。"""
    try:
        return LAYER_MAP[relation_type]
    except KeyError:
        raise ValueError(f"Unknown relation type: {relation_type}")

@functools.lru_cache(maxsize=1)
def get_block_mapping(
    path: str = "data/block_topology.jsonl",
) -> tuple[dict[str, int], dict[int, str]]:
    """block_id <-> genealogy_number 双向映射。进程级缓存。"""
    id2num, num2id = {}, {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            rec = json.loads(line)
            bid, num = rec["block_id"], rec["genealogy_number"]
            id2num[bid] = num
            num2id[num] = bid
    return id2num, num2id
```

concept_id -> block_id：直接查询 ConceptRegistry 对象，不设独立索引。

---

## 总结：收敛与分歧

### 完全收敛（5 项）

| # | 项目 | 方案 |
|---|------|------|
| R2-C1 | Morse 地形全量重算 + 缓存 | 时序 Kruskal，O(E log E) |
| R2-C2 | topo_event 写入 | `.chanlun/topo_events.jsonl` append-only |
| R2-C3 | 概念注册表格式 | 单文件 JSON，`.chanlun/concept_registry.json` |
| R2-C4 | classify_layer 函数 | LAYER_MAP + ValueError |
| R2-C5 | block_id 映射 | lru_cache 双向 dict |

### Gemini 矛盾发现（已解决/需裁定，3 项）

| # | 矛盾 | 严重性 | 合并后状态 |
|---|------|--------|-----------|
| R2-D1 | query_pair 路径定义 vs 穿越核心 | 重要 | **已解决**：query_pair 默认且唯一 references-only，删除 Dijkstra |
| R2-D2 | Union-Find 双事实来源 | 致命 | **已解决**：两路都选择纯重算策略（方案A），无双事实来源 |
| R2-D3 | concept_id 反向索引冗余 | 重要 | **已解决**：反向索引 = 注册表查询，不是独立数据结构 |

### 无向化分歧（需编排者确认，1 项）

| # | 分歧 | Gemini | Codex | 建议 |
|---|------|--------|-------|------|
| R2-D4 | 双向引用是否合并为一条无向边 | 不合并（与 862 = 1156-296+2 一致） | 合并（得到 ≤1156 条无向边） | 采用 Gemini：不合并，保持与实测一致 |

### 未决项（实现细节，可自行推进）

1. 862 条 critical 边的分组/导航策略（按 cycle 分组？按概念聚类？）
2. BFS 最大深度限制
3. term -> concept_id 二级索引（同义词处理，未来需求）
4. depends_on 环检测的冲突路径返回格式

---

## 认识论等级标注

- 数学验证（862 = 1156-296+2）：**L0**（纯代数）
- 实现规格（函数签名、数据结构）：**L0**（从需求逻辑推导）
- 矛盾发现（3 项）：**L0**（逻辑一致性检查）
- 性能声明（<1ms 重算）：**L1**（未用真实数据计时，但量级估算合理）
