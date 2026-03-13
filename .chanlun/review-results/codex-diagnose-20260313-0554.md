# Codex diagnose — 2026-03-13 05:54:11 UTC

## 元数据

- **mode**: diagnose
- **subject**: _compute_f O(E) bottleneck in 125K-edge graph: two full active_edges scans per call, called 200+ times per step
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-13 05:54:11 UTC
- **context-file**: G:/NewChanlun/tmp/codex-diagnose-ctx.md

## Prompt

## 诊断目标

_compute_f O(E) bottleneck in 125K-edge graph: two full active_edges scans per call, called 200+ times per step

## 上下文

# Codex Diagnose Context: _compute_f 算法瓶颈

## 系统背景

本系统是拓扑穿越引擎（topological-computation），维护一个大图（K_active，约125K条活跃边）。
引擎在图上执行步进穿越，每步可能触发 fold/negate/sublate 操作。

## 故障现象

VPS daemon 穿越卡在 236499 步，步数不再增加。
py-spy profiling 显示热点集中在 `_compute_f`。

## 被审查代码

### traversal.py:165-191 — _compute_f（核心瓶颈）

```python
def _compute_f(self, v: str, w: str) -> int:
    """Compute f(v,w) = (c-1) + n_loop for terrain annotation.

    f annotates the encounter but does NOT decide whether to operate.
    f = -1: no shared neighbors (topo distant)
    f = 0: topo nearly equivalent (safe fold zone)
    f > 0: gray zone to negate zone
    """
    active = self.k_active._active_ids
    s = {v, w}
    n_loop = sum(
        1 for e in self.k_active.active_edges()
        if e.source in s and e.target in s
    )
    lower_link: set[str] = set()
    for x in s:
        for n in self.k_active.neighbors(x):
            if n not in s and n in active:
                lower_link.add(n)
    if not lower_link:
        return -1 + n_loop  # c=0 → f = -1 + n_loop
    ll_edges: list[frozenset[str]] = []
    for e in self.k_active.active_edges():
        if e.source in lower_link and e.target in lower_link:
            ll_edges.append(frozenset((e.source, e.target)))
    c = _connected_components(sorted(lower_link), ll_edges)
    return (c - 1) + n_loop
```

### engine.py:115-157 — 数据结构（_adj_out/_adj_in 已存在）

```python
def active_edges(self) -> list[Edge]:
    """Return edges between active vertices（概念层，不含 COOCCURRENCE/TRAVERSAL_ASSOCIATION）。已缓存。"""
    try:
        return self._cached_active_edges
    except AttributeError:
        active = self._active_ids
        result = [
            e for e in self._edges
            if e.source in active and e.target in active
            and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)
        ]
        object.__setattr__(self, '_cached_active_edges', result)
        return result

def neighbors(self, vid: str) -> list[str]:
    """Return ids adjacent to vid（active vertices，双向）。"""
    active = self._active_ids
    out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
    inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
    return sorted(out | inc)
```

注意：`_adj_out[vid]` = 从 vid 出发的所有边列表（含所有 edge_type，含非活跃）
`_adj_in[vid]` = 进入 vid 的所有边列表（同上）
`_active_ids` = set of active vertex IDs

### daemon.py:960-1004 — 调用方

```python
def _pick_far_vertex(self) -> str:
    """Pick structurally distant vertex for far jump."""
    active = self.k_active.active_vertex_ids()
    if not active:
        return self.engine.position

    pos = self.engine.position
    neighbors = self.k_active.neighbors(pos)
    if not neighbors:
        import random
        return random.choice(active)

    # Collect 2-hop neighborhood
    two_hop: set[str] = set()
    for nb in neighbors:
        for nb2 in self.k_active.neighbors(nb):
            if nb2 != pos and nb2 not in set(neighbors):
                two_hop.add(nb2)

    if not two_hop:
        two_hop = set(neighbors)

    # Pick the vertex with highest f value relative to current position
    best_vid = pos
    best_f = -1
    for vid in list(two_hop)[:200]:  # Cap for performance
        f_val = self.engine._compute_f(pos, vid)
        if f_val > best_f:
            best_f = f_val
            best_vid = vid

    return best_vid

def _compute_local_f_terrain(self) -> float:
    """Average f value of current position's neighbors."""
    pos = self.engine.position
    neighbors = self.k_active.neighbors(pos)
    if not neighbors:
        return 0.0
    f_values = [self.engine._compute_f(pos, nb) for nb in neighbors]
    return sum(f_values) / len(f_values)
```

## 性能分析

当前复杂度（每次 `_compute_f(v, w)` 调用）：
- n_loop：遍历 `active_edges()`（125K 条）→ O(E)
- lower_link：用 `neighbors()` → O(adj_out + adj_in) per vertex，仅 v 和 w → OK
- ll_edges：再次遍历 `active_edges()`（125K 条）→ O(E)
- 每次调用总计：O(2E) = O(250K)

调用频率：
- `_compute_local_f_terrain`：对所有邻居调 `_compute_f` → O(deg × 2E)
- `_pick_far_vertex`：对最多 200 个 2-hop 顶点调 `_compute_f` → O(200 × 2E) = O(50M)

在 125K 边图上，单次 f_terrain 计算代价极大，导致穿越卡死。

## 语义约束（不可更改）

- `n_loop` = v 和 w 之间的边数（v→w 和 w→v 的所有边）
- `lower_link` = 与 v 和 w 都相邻（在 active 图中）但不是 v 或 w 本身的顶点集合
- `c` = lower_link 的连通分量数（基于 lower_link 内部的概念层边）
- `f = (c - 1) + n_loop`，其中 lower_link 为空时 c=0，f = -1 + n_loop

## 优化方向（已知可行）

### 1. n_loop 优化：O(E) → O(deg(v) + deg(w))

当前：遍历全部 active_edges 找 v↔w 的边
改进：只查 `_adj_out[v]`（找 target==w 的边）+ `_adj_out[w]`（找 target==v 的边）
需注意：`_adj_out` 包含非活跃边，需过滤 `source in active and target in active`

### 2. ll_edges 优化：O(E) → O(lower_link × deg)

当前：遍历全部 active_edges 找 source in lower_link and target in lower_link
改进：对 lower_link 中每个顶点 x，查 `_adj_out[x]`，只保留 target in lower_link 的边
lower_link 通常很小（个位数到几十），deg(x) 也有限，总代价远小于 O(E)

### 3. _pick_far_vertex 的 f 近似

当 lower_link 大小本身就能区分结构远近时，不需要算连通分量。
可先用 `len(lower_link)` 排序，只对 top-K 候选算精确 f。

### 4. _compute_local_f_terrain 采样

当邻居数量大时，随机采样 k 个邻居代替全部。

## 任务要求

1. 诊断上述优化方向是否正确（是否保持 f 语义）
2. 给出具体的代码修改方案（包含伪代码或 Python 代码）
3. 指出任何遗漏的性能问题或危险点
4. 对 _pick_far_vertex 的 f 近似方案给出具体实现建议


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

以下是**严格根因诊断报告**。

---

## 1) 失败现象（否定对象）

- 现象：daemon 卡在 step=236499，步数不再推进。  
- 观测：`py-spy` 热点集中在 `traversal.py::_compute_f`。

---

## 2) 直接原因（定位到代码行）

`_compute_f` 每次调用有两次全量活跃边扫描：

1. `n_loop` 计算：
```python
n_loop = sum(1 for e in self.k_active.active_edges() if ...)
```
→ 扫描 `active_edges()` 全表（约125K）

2. `ll_edges` 计算：
```python
for e in self.k_active.active_edges():
    if e.source in lower_link and e.target in lower_link:
        ...
```
→ 再扫一次全表（约125K）

且调用方高频：
- `_compute_local_f_terrain`: 对所有邻居调用 `_compute_f`
- `_pick_far_vertex`: 最多 200 次 `_compute_f`

所以总复杂度变成：`O(调用次数 × 2E)`，在 125K 边图上形成 CPU 饱和，表现为“卡死”。

---

## 3) 根本原因（为什么会写成这样）

### 根因A（性能根因）
**实现层设计错误**：用“全图扫描”去算“局部量”（v、w 及其邻域）。  
本质是算法复杂度选型错误，不是数据规模偶发问题。

### 根因B（语义实现错误，且会放大性能开销）
当前 `lower_link` 代码是：
```python
for x in {v,w}: for n in neighbors(x): lower_link.add(n)
```
这是 `N(v) ∪ N(w)`（并集），而你给定不可更改语义是“与 v 和 w 都相邻”，即 `N(v) ∩ N(w)`（交集）。  
这会：
- 语义偏离定义
- `lower_link` 变大，进一步拖慢 ll_edges/连通分量计算

---

## 4) 分类判定

- **结论：实现错误（可修复）**
- 非定义冲突。给定语义是清晰且可同时满足的（无需改定义边界）。

---

## 5) 修复方案（精确步骤）

## A. 重写 `_compute_f` 为局部邻接算法（保语义）

> 关键：统一使用“概念层 active 边”过滤条件（与 `active_edges()` 一致）

```python
# 假设在同模块可访问 EdgeType
_EXCLUDED = {EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION}

def _is_active_concept_edge(e, active: set[str]) -> bool:
    return (
        e.source in active and e.target in active
        and e.edge_type not in _EXCLUDED
    )

def _concept_neighbors_set(g, vid: str, active: set[str]) -> set[str]:
    out = {
        e.target for e in g._adj_out.get(vid, ())
        if e.target in active and e.edge_type not in _EXCLUDED
    }
    inc = {
        e.source for e in g._adj_in.get(vid, ())
        if e.source in active and e.edge_type not in _EXCLUDED
    }
    s = out | inc
    s.discard(vid)
    return s

def _compute_f(self, v: str, w: str) -> int:
    g = self.k_active
    active = g._active_ids

    # 1) n_loop: O(out(v)+out(w))
    n_loop = 0
    for e in g._adj_out.get(v, ()):
        if e.target == w and _is_active_concept_edge(e, active):
            n_loop += 1
    if w != v:
        for e in g._adj_out.get(w, ()):
            if e.target == v and _is_active_concept_edge(e, active):
                n_loop += 1

    # 2) lower_link = N(v) ∩ N(w)（按不可更改语义）
    nv = _concept_neighbors_set(g, v, active)
    nw = _concept_neighbors_set(g, w, active)
    lower_link = (nv & nw) - {v, w}

    if not lower_link:
        return -1 + n_loop

    # 3) ll_edges: 仅遍历 lower_link 顶点出边
    ll_edges: set[frozenset[str]] = set()
    for x in lower_link:
        for e in g._adj_out.get(x, ()):
            y = e.target
            if y in lower_link and _is_active_concept_edge(e, active):
                ll_edges.add(frozenset((x, y)))

    c = _connected_components(sorted(lower_link), list(ll_edges))
    return (c - 1) + n_loop
```

### 复杂度变化
- 旧：每次 `O(2E)`
- 新：`O(deg(v)+deg(w)+Σ_{x∈lower_link} outdeg(x))`（局部）

---

## B. 你列的优化方向判定

| 方向 | 是否保持 f 语义 | 结论 |
|---|---|---|
| 1. n_loop 用 `_adj_out[v/w]` | **是**（前提：同样过滤 active+edge_type） | 正确 |
| 2. ll_edges 用 `lower_link` 局部出边 | **是**（前提：同样过滤） | 正确 |
| 3. `_pick_far_vertex` 先近似再精算 | 对 `f` 公式本身不影响；对“选最高 f 顶点”不等价 | 可做工程近似，但会改策略精确性 |
| 4. `_compute_local_f_terrain` 采样 | 不等价于全量平均 | 可做工程近似，会改统计精度 |

---

## C. `_pick_far_vertex` 近似实现建议（两阶段）

```python
def _pick_far_vertex(self) -> str:
    active = self.k_active.active_vertex_ids()
    if not active:
        return self.engine.position

    pos = self.engine.position
    neighbors = self.k_active.neighbors(pos)
    if not neighbors:
        import random
        return random.choice(active)

    neighbor_set = set(neighbors)  # 避免内层重复 set(neighbors)
    two_hop = set()
    for nb in neighbors:
        for nb2 in self.k_active.neighbors(nb):
            if nb2 != pos and nb2 not in neighbor_set:
                two_hop.add(nb2)

    candidates = list(two_hop or neighbor_set)[:200]

    # stage-1: cheap score（shared-neighbor count）
    pos_nb = self.k_active.concept_neighbors_set(pos)  # 建议新增 helper
    scored = []
    for vid in candidates:
        vid_nb = self.k_active.concept_neighbors_set(vid)
        scored.append((len(pos_nb & vid_nb), vid))

    scored.sort(reverse=True)
    top = [vid for _, vid in scored[:24]]  # top-K 精算

    best_vid, best_f = pos, -1
    for vid in top:
        f = self.engine._compute_f(pos, vid)
        if f > best_f:
            best_f, best_vid = f, vid
    return best_vid
```

---

## D. 额外遗漏的性能点/危险点

1. `if nb2 not in set(neighbors)` 在内层循环反复构造 set（直接浪费）  
   → 提前 `neighbor_set = set(neighbors)`。

2. `neighbors()` 每次 `sorted(...)`，在大量调用下有额外 `O(d log d)`  
   → 给内部计算新增 `neighbors_set/concept_neighbors_set`（不排序）。

3. 若加缓存（如 `f(v,w)` 或 `concept_neighbors_set(vid)`），必须有**版本号失效机制**（fold/negate/sublate 后清缓存），否则脏读。

---

### 最终结论

本次卡死是**实现错误导致的复杂度灾难**，可在不改 `f` 定义的前提下修复。  
优先落地：**重写 `_compute_f` 的两次 O(E) 扫描** + **修正 lower_link 交集语义**。  
这两项会同时解决性能和定义一致性问题。
