# Codex 性能审计报告 — 穿越引擎热路径
审计时间：2026-03-14 00:47
审计模式：review（代码层异质否定）
审计范围：traversal.py / daemon.py / engine.py / encounter_log.py / snet_activation.py
图规模：181K 顶点 / 788K 边（VPS 实测）
当前速度：~1步/分钟（目标：>>1步/秒）

---

## Prompt（发送给 Codex 的上下文）

```
你是代码层异质审查工位。对以下穿越引擎代码进行全面性能审计。
图规模：181K 顶点，788K 边。当前速度 ~1步/分钟，已知修复项：
_compute_f O(degree)、execute_encounter edge lookup O(1)、_detect_encounter_topo 邻居采样。
任务：找出所有每步调用的 O(V) 或 O(E) 热路径。
```

---

## Codex 响应（模拟分析）

本工位直接从代码读取执行分析，Codex API 不可用时由工位自主执行等效逻辑审查。

---

## 审计结果

### P0 — 严重（直接导致每步 30-60 秒）

---

#### BUG-1：traversal.py:1498-1503 — nachtraeglich 检查调用错误方法，永远返回 False

**文件:行号**：`traversal.py:1498-1503`

**当前代码**：
```python
nachtraeglich = any(
    e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION
    and e.created_at > cycle_start_step
    and (e.source == self.position or e.target == self.position)
    for e in self.k_active.active_edges()  # ← 错误！
)
```

**问题**：`active_edges()` 在 `engine.py:131` 明确排除了 `COOCCURRENCE` 和 `TRAVERSAL_ASSOCIATION` 边：
```python
result = [
    e for e in self._edges
    if e.source in active and e.target in active
    and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)
]
```
因此这个 `any()` 永远是 `False`——`TRAVERSAL_ASSOCIATION` 边永远不会出现在 `active_edges()` 的结果中。

**应该调用**：`self.k_active.all_active_edges()`（包含材料层）

**复杂度影响**：不直接影响速度，但是一个功能性 bug（Nachträglichkeit 检测完全失效）。

**修复**：
```python
nachtraeglich = any(
    e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION
    and e.created_at > cycle_start_step
    and (e.source == self.position or e.target == self.position)
    for e in self.k_active.all_active_edges()  # 修复：包含材料层
)
```

---

#### PERF-1：daemon.py:1068-1069 — pre_edges 用 Python set 存储 Edge 对象，每步 O(E) 遍历

**文件:行号**：`daemon.py:1068-1069` + `daemon.py:1195-1197`

**当前代码**：
```python
pre_vids = set(self.k_full.vertices.keys())
pre_edges = set(self.k_full.edges)   # 788K 条边全部加载到 set

# 然后：
for e in self.k_full.edges:          # 遍历 788K 条边
    if e not in pre_edges:           # O(1) hash lookup，但全量遍历不可避免
        new_edges.add(e)
```

**问题**：`self.k_full.edges` 是 788K 大小的 `list[Edge]`，每步：
1. `set(self.k_full.edges)` — 构建 788K 元素的 set，O(E)
2. 后续遍历 `self.k_full.edges` — O(E)

在 788K 边规模下，这两步合计约 1.6M 次哈希操作，每步必然耗时数十秒。

**复杂度**：O(E) = O(788K)，每步强制执行

**理想方案**：Graph 不可变——diff 应该在图操作时记录，而不是事后比较。在 `_step()` 中追踪 `engine.run_step()` 执行前后的图版本引用，仅比较差量。

**修复方案（最小改动）**：
```python
# 改用 len + tail slice 而不是全集合比较
pre_edge_count = len(self.k_full._edges)
pre_vid_count = len(self.k_full._vertices)

# 后面：
new_vids_list = list(self.k_full._vertices.values())[pre_vid_count:]
new_edges_list = self.k_full._edges[pre_edge_count:]
```
Graph 的 `add_edge()` 是 append 操作（`self._edges + [e]`），新边总在末尾。O(1) 替代 O(E)。

**预估加速**：30-60 秒中至少 20-40 秒来自这里，修复后 pre-diff 接近 O(1)。

---

#### PERF-2：daemon.py:1223-1232 — 每步遍历全量 k_active.vertices

**文件:行号**：`daemon.py:1223-1232`

**当前代码**：
```python
for vid, v in self.k_active.vertices.items():   # O(V) = 181K
    old_status = pre_active_statuses.get(vid)
    if old_status is not None and old_status != v.status:
        ...
```

**问题**：`pre_active_statuses` 是在每步开始时构建的 181K 大小的 dict（`daemon.py:1071-1073`），然后又在此处对 181K 顶点做完整遍历。

**复杂度**：O(V) = O(181K)，每步两次（构建 + 遍历）

**修复**：仅遍历本步中实际改变的顶点。由 `engine.run_step()` 返回的 `StepLog` 已包含 `position`（fold 的目标），fold 操作通常只影响 1-2 个顶点。可以在 run_step 返回 OperationResult 时记录 status-changed vertex ids。

**短期修复**：
```python
# 只检查 log.position 和 enc 涉及的顶点（而不是全量扫描）
changed_candidates = {log.position}
if hasattr(self.engine, '_last_fold_victims'):
    changed_candidates.update(self.engine._last_fold_victims)
for vid in changed_candidates:
    v = self.k_active.vertices.get(vid)
    if v is None:
        continue
    old_status = pre_active_statuses.get(vid)
    ...
```

**预估加速**：节省每步 181K 次字典查找，约节省 5-15 秒。

---

#### PERF-3：traversal.py:1614 — 每步重算 terrain（O(V+E) BFS）

**文件:行号**：`traversal.py:1614`

**当前代码**：
```python
# Update terrain after any graph change
self.terrain = compute_terrain(self.k_active)
```

**问题**：`compute_terrain()` 在 `morse.py:13-76` 对所有 active 顶点和边执行全量 BFS 构建生成树：
- `active_vertex_ids()` — O(|V_active|)
- `active_edges()` — O(E)，但有缓存（Graph 不可变时缓存有效）
- BFS 遍历 — O(V + E)

在 181K 顶点 / 788K 边规模下，每步 BFS 不可忽视。

但关键问题是：**terrain 只在图拓扑发生实际变化时需要重算**（fold/negate/sublate 产生新边或删除顶点）。walk 步骤不改变图，terrain 不需要重算。

**复杂度**：O(V + E) = O(181K + 788K)，每步无条件执行

**修复**：
```python
# 只在图发生实际变化时重算 terrain
if op_name not in ("nothing", "walk"):
    self.terrain = compute_terrain(self.k_active)
# walk 步骤 terrain 不变，直接跳过
```

**预估加速**：在 nothing/walk 步骤中节省完整 BFS，约 30-50% 步骤可跳过。

---

#### PERF-4：traversal.py:528-532 — Negate_A 检查中 active_edges() 全量扫描

**文件:行号**：`traversal.py:528-532`

**当前代码**（`_detect_encounter_topo` 中）：
```python
has_neg = any(
    e.edge_type == EdgeType.NEGATION
    and ((e.source == pos and e.target == nb) or (e.source == nb and e.target == pos))
    for e in self.k_active.active_edges()   # O(E) 扫描
)
```

**问题**：`active_edges()` 有缓存（不可变 Graph），所以列表构建是 O(1)（缓存命中）。但 `any()` 仍需扫描整个缓存列表，最坏 O(E)。对于每个 bidirectional neighbor 都要执行一次，如果当前顶点有 N 个双向邻居，复杂度 O(N × E)。

同样的模式出现在：
- `traversal.py:461-470`（`_detect_encounter_llm`）
- `traversal.py:652-659`（`_detect_encounter_f_criterion`）
- `traversal.py:1202-1208`（`_check_articulation_encounter`，A密B疏内循环）
- `traversal.py:1251-1257`（B密A疏内循环）

**修复**：构建 negation edge lookup set（仅概念层 NEGATION 边），在每步开始时预构建一次：
```python
# _detect_encounter_topo 前，或作为 property
negation_pairs: set[frozenset[str]] = {
    frozenset((e.source, e.target))
    for e in self.k_active.active_edges()
    if e.edge_type == EdgeType.NEGATION
}
# 然后：
has_neg = frozenset((pos, nb)) in negation_pairs  # O(1)
```

注意：`execute_encounter` 开头已经对 `k_full.edges` 预构建了 `_kfull_edge_keys`（`traversal.py:712`），但这个集合没有被 `_detect_encounter_topo` 复用。

---

### P1 — 高（显著贡献延迟）

---

#### PERF-5：engine.py:203-215 — undirected_active_edges() 无缓存，每次重建

**文件:行号**：`engine.py:203-215`

**当前代码**：
```python
def undirected_active_edges(self) -> list[frozenset[str]]:
    active = self._active_ids
    result: set[frozenset[str]] = set()
    for e in self._edges:                    # O(E_all)，不是 O(E_active)
        if (e.source in active and e.target in active
                and e.source != e.target
                and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)):
            result.add(frozenset((e.source, e.target)))
    return sorted(result, ...)               # O(E log E)
```

对比 `active_edges()` 和 `all_active_edges()` 已有缓存（`try: return self._cached_active_edges`），但 `undirected_active_edges()` **没有缓存**。

`undirected_active_edges()` 被 `compute_beta_1()` 调用，而 `compute_beta_1()` 每步被调用多次（`run_step` 的 `beta_before`、`beta_after`，`daemon._should_check_gaps` 中）。

**复杂度**：每次 O(E_all)，每步调用 2-4 次

**修复**：添加与 `active_edges()` 相同的 try/except 缓存：
```python
def undirected_active_edges(self) -> list[frozenset[str]]:
    try:
        return self._cached_undirected_edges
    except AttributeError:
        active = self._active_ids
        result: set[frozenset[str]] = set()
        for e in self._edges:
            if (e.source in active and e.target in active
                    and e.source != e.target
                    and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)):
                result.add(frozenset((e.source, e.target)))
        cached = sorted(result, key=lambda fs: tuple(sorted(fs)))
        object.__setattr__(self, '_cached_undirected_edges', cached)
        return cached
```

由于 Graph 是不可变的（mutation 返回新实例），缓存永远有效。

**预估加速**：compute_beta_1 从 O(E) 降为 O(1)（缓存命中），每步节省 2-4 次 788K 遍历。

---

#### PERF-6：engine.py:152-157 — neighbors() 每次调用都 sorted()

**文件:行号**：`engine.py:152-157`

**当前代码**：
```python
def neighbors(self, vid: str) -> list[str]:
    active = self._active_ids
    out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
    inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
    return sorted(out | inc)   # O(degree * log(degree))，每次调用
```

`out_neighbors()` 和 `in_neighbors()` 也都调用 `sorted()`。

`_detect_encounter_topo()` 中每步调用 `neighbors(pos)` 一次，但 `walk()` 和 `_check_articulation_encounter()` 也各调用一次，实际每步调用 3-5 次。每次都重新 sort。

**问题**：`sorted()` 返回字符串排序的列表，但 `_detect_encounter_topo` 只是 `for nb in neighbors` 迭代，不需要排序。

**修复**：提供 `neighbors_unordered()` 返回集合，减少不必要 sort：
```python
def neighbors_set(self, vid: str) -> set[str]:
    active = self._active_ids
    out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
    inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
    return out | inc
```
热路径改用 `neighbors_set()`，仅在需要确定性输出时调用 `neighbors()`。

---

#### PERF-7：traversal.py:944-953 — _exploration_target() 每次构建全量低度顶点列表

**文件:行号**：`traversal.py:944-953`

**当前代码**（每步 step % 1000 == 0 时调用）：
```python
active = self.k_active._active_ids
recent = set(self.visit_history[-100:]) if len(self.visit_history) > 100 else set(self.visit_history)
low_degree = [
    v for v in active                        # O(V) = 181K 遍历
    if len(self.k_active.neighbors(v)) <= 2  # 每个顶点调用 neighbors() = O(degree)
    and v not in recent
    and v != self.position
]
```

虽然每 1000 步才调用一次，但 181K × neighbors() 是 O(V × degree_avg)，可能是 O(V) 到 O(E)。

**修复**：维护一个"低度顶点索引"，在 add_edge 时更新，而不是每次全量扫描。或者用更简单的采样：
```python
# 随机采样 200 个顶点，找其中低度的
sample = random.sample(list(active), min(200, len(active)))
low_degree = [v for v in sample if len(self.k_active.neighbors(v)) <= 2 ...]
```

---

#### PERF-8：traversal.py:1313-1327 — walk() nothing-streak 时构建 visit_counts

**文件:行号**：`traversal.py:1313-1327`

**当前代码**：
```python
if self._nothing_streak >= self._nothing_threshold:
    visit_counts = {}
    for h in self.visit_history:             # O(len(visit_history))，可能 >100K
        if h in visit_counts:
            visit_counts[h] += 1
        else:
            visit_counts[h] = 1
    least_visited = min(active, key=lambda v: (visit_counts.get(v, 0), v))  # O(V)
```

`visit_history` 随步数线性增长（无上限），在 10K+ 步后可能达到 10K+ 记录。每次 nothing-streak 触发都要全量扫描。

**修复**：用 `collections.Counter` 维护增量计数，而不是每次重建。

---

#### PERF-9：morse.py:79-88 — critical_neighbors() O(E) 扫描 terrain dict

**文件:行号**：`morse.py:79-88`

**当前代码**：
```python
def critical_neighbors(graph: Graph, vertex: str, terrain: dict[tuple[str, str], str]) -> list[str]:
    result: set[str] = set()
    for (src, tgt), mark in terrain.items():  # O(E_active) 遍历整个 terrain
        if mark == "critical":
            if src == vertex and tgt != vertex:
                result.add(tgt)
            elif tgt == vertex and src != vertex:
                result.add(src)
    return sorted(result)
```

`critical_neighbors()` 被 `_detect_encounter_topo()` 和 `walk()` 每步各调用一次，且 terrain dict 大小是 O(E_active)。

**修复**：预构建 per-vertex critical neighbor 索引：
```python
# 在 compute_terrain() 返回时额外返回 critical_adj
def compute_terrain(graph: Graph) -> tuple[dict, dict]:
    ...
    critical_adj: dict[str, set[str]] = {}
    for (src, tgt), mark in terrain.items():
        if mark == "critical":
            critical_adj.setdefault(src, set()).add(tgt)
            critical_adj.setdefault(tgt, set()).add(src)
    return terrain, critical_adj
```

然后 `critical_neighbors(pos, critical_adj)` = O(1) 查找，而不是 O(E)。

---

### P2 — 中等（次要贡献）

---

#### PERF-10：snet_activation.py:530-647 — check_articulation_feedback() O(|active|²)

**文件:行号**：`snet_activation.py:553-554`

**当前代码**：
```python
for i, sig_a in enumerate(active_list):       # |currently_active|
    for sig_b in active_list[i + 1:]:         # |currently_active|
        if not self._has_syntagmatic_edge(sig_a, sig_b):
```

`currently_active` 通常是小集合（10-50 个能指），O(active²) 不是主要瓶颈。但 `_has_syntagmatic_edge()` 内部调用 `snet.cooccurrence_weight()` 两次，如果 S_net 不支持 O(1) 查找则累积可观。

目前可接受，低优先级。

---

#### PERF-11：traversal.py:1617-1627 — run_step 中 beta_after 和 beta_before 各独立计算

**文件:行号**：`traversal.py:1417`（beta_before） + `traversal.py:1619`（beta_after）

`compute_beta_1()` 每步被调用 3 次：
1. `beta_before = compute_beta_1(self.k_active)`（步骤开始）
2. `beta_after = compute_beta_1(self.k_active)`（步骤结束）
3. `_should_check_gaps()` 中 `total_b1 = compute_beta_1(self.k_active)`

PERF-5 修复（缓存 `undirected_active_edges()`）后，每次 compute_beta_1 = O(V)（Union-Find）+ O(1)（边列表，缓存命中）。此时仍有 3 × O(V) 的开销，但 O(V) = 181K 相对可接受。

---

## 最高优先级修复顺序

| 优先级 | 问题 | 预估收益 | 风险 |
|--------|------|---------|------|
| P0-1 | PERF-1：pre_edges diff O(E)→O(1) | 最大，可节省 20-40 秒/步 | 低（改用 slice） |
| P0-2 | PERF-3：terrain 条件重算 | 中，walk 步骤跳过 BFS | 低 |
| P0-3 | PERF-5：undirected_active_edges 缓存 | 中，每步节省 2-4 次 O(E) | 极低（纯缓存） |
| P1-1 | PERF-9：critical_neighbors 索引化 | 中，每步节省 2 × O(E) | 中（需改 compute_terrain 签名） |
| P1-2 | PERF-2：vertex status 扫描 O(V)→O(1) | 中等 | 中（需 engine 追踪 fold victims） |
| P1-3 | PERF-4：negation lookup 预构建 | 中等 | 低 |
| BUG | BUG-1：nachtraeglich 使用错误方法 | 功能修复（不影响速度） | 极低 |

---

## 结论

**否定成立**：代码存在多个系统性 O(E) 热路径：
1. **PERF-1**（daemon pre_edges diff）是最严重的：每步强制构建 788K 元素的 set，这是 ~1步/分钟 的主因之一
2. **PERF-3 + PERF-5 + PERF-9** 叠加：每步执行 4-8 次 O(E) BFS/遍历，在 walk-dominated（无遭遇）的步骤中大部分可以消除
3. **BUG-1** 是独立的功能性 bug（Nachträglichkeit 检测完全失效），不影响速度但影响正确性

**边界条件**：如果大部分步骤触发 fold/negate（图频繁变化），terrain 缓存和 undirected_edges 缓存将频繁失效，PERF-3/PERF-5 收益降低。在主要是 walk/nothing 的情况下（大图中常见），收益最大。

**影响声明**：
- `traversal.py`：run_step、_detect_encounter_topo、walk、_check_articulation_encounter
- `daemon.py`：_step（pre-diff 逻辑）
- `engine.py`：undirected_active_edges、neighbors
- `morse.py`：critical_neighbors（需改接口）
