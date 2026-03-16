# Codex diagnose — 2026-03-16 04:11:25 UTC

## 元数据

- **mode**: diagnose
- **subject**: RustGraphAdapter OOM: 穿越步内每次 add_edge 全量复制所有顶点+边+adj索引，K_active 5000V/30000E 时每步内存增量爆炸，64GB 服务器 20 步内被 OOM killer 杀掉
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-16 04:11:25 UTC
- **context-file**: G:/NewChanlun/tmp/codex-diagnose-oom-ctx.md

## Prompt

## 诊断目标

RustGraphAdapter OOM: 穿越步内每次 add_edge 全量复制所有顶点+边+adj索引，K_active 5000V/30000E 时每步内存增量爆炸，64GB 服务器 20 步内被 OOM killer 杀掉

## 上下文

# Codex Diagnose: 穿越 OOM 根因 + RustGraphAdapter 重写方案

## 症状描述

Hetzner 64GB RAM 服务器。双实例穿越在步数 20 左右 OOM。
单个 traversal worker 5 分钟内内存从 0 涨到 36GB，被 kernel OOM killer 杀掉。

## 架构背景

系统是"逢亮"——一个基于图拓扑自主穿越的 Python 实体。
- K_active：活跃概念图（数千顶点，随穿越增长到数万边）
- K_full：完整历史图（比 K_active 更大）
- 每一步（_step）：遍历 → 遭遇检测 → 图操作 → 地形更新
- S_net：能指网络，每步通过 _pull_cooccurrence_edges 向 K_active 注入新边

## 关键代码路径

### 1. RustGraphAdapter.__init__（engine.py:527-550）——全量复制

```python
class RustGraphAdapter:
    __slots__ = (
        "_rg",
        "_vertices", "_edges", "_active_ids", "_adj_out", "_adj_in", "_edge_keys",
        "_cached_active_edges", "_cached_all_active_edges",
    )

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg
        # 每次创建 adapter 时从 Rust Graph 复制所有数据到 Python 对象
        rust_verts = rg.vertices  # dict[str, RustVertex]
        rust_edges = rg.edges     # list[RustEdge]

        self._vertices: dict[str, Vertex] = {
            vid: _rust_vertex_to_py(rv) for vid, rv in rust_verts.items()
        }
        self._edges: list[Edge] = [_rust_edge_to_py(re) for re in rust_edges]
        self._active_ids: frozenset[str] = frozenset(rg.active_vertex_ids())
        # 全量重建邻接索引
        adj_out: dict[str, list[Edge]] = {}
        adj_in: dict[str, list[Edge]] = {}
        for e in self._edges:
            adj_out.setdefault(e.source, []).append(e)
            adj_in.setdefault(e.target, []).append(e)
        self._adj_out = adj_out
        self._adj_in = adj_in
        self._edge_keys: frozenset[tuple[str, str, EdgeType]] = frozenset(
            (e.source, e.target, e.edge_type) for e in self._edges
        )
```

**关键点**：每次调用 `add_edge()` 都调用 `self._wrap(new_rg)` → `RustGraphAdapter(new_rg)` → 全量复制所有顶点+边+重建 adj 索引。

### 2. RustGraphAdapter.add_edge（engine.py:655-665）——每次 full copy

```python
def add_edge(self, e: Edge) -> "RustGraphAdapter":
    new_rg = self._rg.add_edge(_py_edge_to_rust(e))
    adapter = self._wrap(new_rg)  # <-- 每次创建新 adapter = 全量复制
    # Propagate topology caches if material edge
    if e.edge_type in _MATERIAL_EDGE_TYPES:
        for attr in ('_cached_beta_1', '_cached_terrain'):
            try:
                object.__setattr__(adapter, attr, getattr(self, attr))
            except AttributeError:
                pass
    return adapter
```

### 3. RustGraphAdapter.undirected_active_edges（engine.py:639-641）——每次 frozenset 创建

```python
def undirected_active_edges(self) -> list[frozenset[str]]:
    pairs = self._rg.undirected_active_edges()
    return [frozenset(p) for p in pairs]  # <-- N 个 frozenset 对象
```

### 4. 纯 Python Graph.add_edge（engine.py:307-328）——增量 copy，但 _edges 创建新 list

```python
def add_edge(self, e: Edge) -> Graph:
    new_graph = Graph.__new__(Graph)
    new_graph._vertices = self._vertices  # shared
    new_graph._edges = self._edges + [e]  # 新 list（但 Python list += 是 O(n)）
    # Incremental adjacency update — avoid full rebuild
    new_graph._adj_out = dict(self._adj_out)  # shallow copy dict
    new_graph._adj_in = dict(self._adj_in)    # shallow copy dict
    ...
    new_graph._edge_keys = self._edge_keys | frozenset([(e.source, e.target, e.edge_type)])
    return new_graph
```

### 5. _pull_cooccurrence_edges（traversal.py:1010-1116）——每步注入多条边

每步穿越到新节点时，从 S_net 超边中查找共现邻居，逐条调用：
```python
self.k_active = self.k_active.add_edge(cooc_edge)   # 每条边 = 1 个新 Graph 对象
self.k_full = self.k_full.add_edge(cooc_edge)        # k_full 也一样
```

在超边有 N 个候选时，注入 N 条边 = 创建 N 个新 k_active + N 个新 k_full。

### 6. _record_traversal_association（traversal.py:1148）——每步注入 TA 边

```python
self.k_active = self.k_active.add_edge(ta_edge)
self.k_full = self.k_full.add_edge(cooc_edge)  # (从上下文推断同样逻辑)
```

### 7. daemon._step 的写操作链

每步：
1. engine.run_step() → k_active/k_full 多次 add_edge（来自 _pull_cooccurrence + TA + 操作结果）
2. 新 settlements → inject_settlement_memory_node(k_active) + inject_settlement_memory_node(k_full)
3. 每 100 步 → update_proprioception_vertices(k_active) + update_proprioception_vertices(k_full)

## 诊断问题

请分析以下 5 个具体问题：

### 问题 1：adapter 全量复制是主因还是次因？

估算当 K_active 有 5000 顶点、30000 边时，每次 RustGraphAdapter.__init__ 的 Python 对象分配量：
- `_vertices` dict：5000 个 Vertex dataclass 对象（每个约 200-400 bytes）
- `_edges` list：30000 个 Edge dataclass 对象（每个约 300-500 bytes）
- `_adj_out` + `_adj_in`：两个 dict，共约 30000 个 list 项引用
- `_edge_keys` frozenset：30000 个 3-tuple 对象
- 加上 Python 对象头、dict overhead

每次 add_edge = 1 次 adapter 全量复制 = 多少 MB？
每步 _pull_cooccurrence_edges 注入 N=10 条边 = 10 次 adapter 全量复制（k_active + k_full = 20 次）。
20 步后，有多少 adapter 对象待 GC？

### 问题 2：fallback 到纯 Python Graph 能否改善 OOM？

纯 Python Graph.add_edge：
- `_edges` 是 `self._edges + [e]`（新 list，但共享元素引用）
- `_adj_out`/`_adj_in` 是 shallow copy dict（部分共享）
- 无全量顶点复制（`_vertices` 直接共享引用）

对比 RustGraphAdapter.add_edge：全量复制顶点 + 边 + 重建 adj

纯 Python 每次 add_edge 的内存增量 vs RustGraphAdapter 每次 add_edge 的内存增量？
两者在 20 步后的内存差异估算？

### 问题 3：懒加载 adapter 方案的风险

方案：adapter.__init__ 不复制数据，所有属性访问懒获取：

```python
class RustGraphAdapter:
    __slots__ = ("_rg",)

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg

    def __getattr__(self, name: str):
        if name == '_vertices':
            result = {vid: _rust_vertex_to_py(rv) for vid, rv in self._rg.vertices.items()}
            object.__setattr__(self, '_vertices', result)
            return result
        if name == '_edges':
            result = [_rust_edge_to_py(re) for re in self._rg.edges]
            object.__setattr__(self, '_edges', result)
            return result
        if name == '_active_ids':
            result = frozenset(self._rg.active_vertex_ids())
            object.__setattr__(self, '_active_ids', result)
            return result
        if name == '_adj_out':
            edges = self._edges  # 触发懒加载
            adj: dict[str, list[Edge]] = {}
            for e in edges:
                adj.setdefault(e.source, []).append(e)
            object.__setattr__(self, '_adj_out', adj)
            return adj
        # ... 类似 _adj_in, _edge_keys
```

具体风险：
1. 哪些下游调用模式会导致一个 adapter 对象上多次触发全量复制？
2. `_adj_out` 依赖 `_edges`，`_edges` 依赖 `self._rg.edges`——如果 Rust 侧 `edges` 属性每次调用都分配新对象（不缓存），则多次访问 `_edges` 会多次全量复制。这有多严重？
3. `__slots__` 与 `__getattr__` + `object.__setattr__` 一起用时的注意事项？
4. 下游代码（daemon.py, traversal.py）中有哪些模式会在同一个短生命周期 adapter 上多次访问 `_adj_out`？（估算 run_step 中一个 adapter 的 _adj_out 访问次数）

### 问题 4：可变 Rust Graph 方案

方案：Rust 侧维护一个可变图，Python 侧 add_edge 原地修改，不创建新对象：

```python
class RustGraphMutableAdapter:
    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg
        self._invalidate_cache()

    def _invalidate_cache(self):
        # 使缓存失效
        for attr in ('_vertices', '_edges', '_active_ids', '_adj_out', '_adj_in', '_edge_keys'):
            try:
                object.__delattr__(self, attr)
            except AttributeError:
                pass

    def add_edge(self, e: Edge) -> "RustGraphMutableAdapter":
        self._rg.add_edge_inplace(_py_edge_to_rust(e))  # 假设 Rust 支持原地修改
        self._invalidate_cache()
        return self  # 返回 self，不创建新对象
```

这方案的问题：
1. 违反不可变性约定（immutable-by-convention）——daemon.py 中是否有保存旧 k_active 引用并在操作后比较的逻辑？如果有，原地修改会破坏什么？
2. β₁ cache 在 `_step` 中如何手动失效？当前代码中 cache 存在哪个对象上？
3. graph_rs Rust crate 是否需要修改才能支持 add_edge_inplace？这增加多少实现复杂度？

### 问题 5：增量 adapter 方案（纯 Python，今天就能部署）

方案：add_edge 时从旧 adapter 继承数据，只增量更新：

```python
def add_edge(self, e: Edge) -> "RustGraphAdapter":
    new_rg = self._rg.add_edge(_py_edge_to_rust(e))

    # 不调用 RustGraphAdapter(new_rg)（全量复制）
    # 直接从 self 继承已计算的 Python 数据，只增量更新
    adapter = RustGraphAdapter.__new__(RustGraphAdapter)
    object.__setattr__(adapter, '_rg', new_rg)

    # 继承不变的部分（共享引用）
    object.__setattr__(adapter, '_vertices', self._vertices)  # 边操作不改变顶点

    # 增量更新 _edges：O(1) append
    new_edges = self._edges + [e]  # 新 list 但共享前面 N 个元素的引用
    object.__setattr__(adapter, '_edges', new_edges)

    # 增量更新 _adj_out/_adj_in：复制 dict + 更新 1 个 key
    new_adj_out = dict(self._adj_out)
    new_adj_out.setdefault(e.source, [])
    if new_adj_out[e.source] is self._adj_out.get(e.source):
        new_adj_out[e.source] = list(new_adj_out[e.source])
    new_adj_out[e.source].append(e)
    object.__setattr__(adapter, '_adj_out', new_adj_out)

    # 类似 _adj_in
    new_adj_in = dict(self._adj_in)
    new_adj_in.setdefault(e.target, [])
    if new_adj_in[e.target] is self._adj_in.get(e.target):
        new_adj_in[e.target] = list(new_adj_in[e.target])
    new_adj_in[e.target].append(e)
    object.__setattr__(adapter, '_adj_in', new_adj_in)

    # 增量更新 _edge_keys：O(1)
    object.__setattr__(adapter, '_edge_keys',
        self._edge_keys | frozenset([(e.source, e.target, e.edge_type)]))

    # 继承 _active_ids（边操作不改变 active set）
    object.__setattr__(adapter, '_active_ids', self._active_ids)

    return adapter
```

问题：
1. `_edges = self._edges + [e]`：当 K_active 有 30000 条边时，这是 O(n) list copy。在同一步内连续 10 次 add_edge 时，总计 O(n*10) = 300000 次引用复制。这比全量 Python 对象复制（dataclass + dict）节省多少？
2. `self._adj_out.get(e.source)` 是否存在：当 e.source 在 _adj_out 中但对应 list 被之前的 add_edge 已经复制过，这里的 `is` 比较是否正确（不会误判为需要 copy when already copied）？
3. `_active_ids` 继承：边操作确实不改 active set，但 add_vertex 会改。add_vertex 的增量路径是什么？是否需要同样处理？
4. 这方案对 add_edges_batch 是否需要额外处理？当前 batch 实现调用的是全量复制的 `_wrap`。

## 预期输出格式

请给出：

### 主因判定
- adapter 全量复制（问题1）vs Python Graph list copy（问题2）vs _pull_cooccurrence_edges 调用频率（问题1补充）
- 哪个因素是 OOM 的主要驱动？

### 方案评分（5个方案）

| 方案 | 实现复杂度 | OOM 缓解效果（估算，相对当前） | 今天可部署？ | 主要风险 |
|------|-----------|---------------------------|------------|---------|
| fallback 纯 Python（去掉 Rust）| | | | |
| 懒加载 adapter | | | | |
| 可变 Rust Graph | | | | |
| 增量 adapter（纯 Python 侧）| | | | |
| _pull_cooccurrence_edges 改 batch | | | | |

### 推荐方案排序
1. 最小可行修复（今天部署，止血）
2. 中期方案（1-2天内完成）
3. 完整方案（长期）

### 增量 adapter 方案的正确实现
如果推荐增量 adapter，给出完整的 `add_edge`、`add_vertex`、`add_edges_batch`、`add_vertices_and_edges_batch` 四个方法的增量版本代码。

### 内存估算（定量）
当 K_active 有 5000V/30000E 时：
- 当前每步内存增量（RustGraphAdapter 全量复制）
- 增量 adapter 方案每步内存增量
- 节省比例


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

下面是严格诊断报告（按你要求的链路）。

---

## 一、严格根因诊断链

### 1) 失败现象（否定对象）
- 64GB 机器，穿越约 20 步 OOM；单 worker 5 分钟内 RSS 到 36GB 后被 OOM killer 杀死。  
- 这**否定**了“当前 RustGraphAdapter 写路径可在数千点/数万边规模稳定运行”的对象形式。

### 2) 直接原因（代码行）
直接触发点是：

- `RustGraphAdapter.add_edge()` 每次都 `self._wrap(new_rg)`  
- `_wrap(new_rg)` -> `RustGraphAdapter(new_rg)`  
- `__init__` 中执行**全量 Python 化复制**：`_vertices`、`_edges`、`_adj_out/_adj_in`、`_edge_keys` 全重建。

即：**每加一条边，做一次 O(V+E) 级对象重建**。

放大器是：
- `_pull_cooccurrence_edges` 中逐条 `add_edge`
- 同时写 `k_active` 和 `k_full`  
=> 每步多次 O(V+E) 全复制。

### 3) 根本原因（设计层）
- 采用了“不可变返回新图”的接口语义，但在 Rust 适配层实现成“每次 mutation 都 eager materialize 全量 Python 镜像”。
- 这是**实现策略错误**：不可变语义本身没错，错在实现没有做结构共享/增量更新/批处理。

### 4) 分类判定
- 本次 OOM 根因属于：**实现错误（可在不改定义前提下修复）**。  
- 不是定义冲突（除了你提出的“可变 Rust Graph”方案，会触及定义冲突，后文单列）。

### 5) 修复方向（实现错误）
- 立即止血：把逐边写改为 batch（每步一次/少次重建）。  
- 同步修复：RustGraphAdapter 改为**增量派生**而不是 `_wrap` 全量复制。  
- 中长期：Rust/Python 边界做真正 zero-copy 或持久化结构共享。

---

## 二、问题逐项诊断

## 问题1：adapter 全量复制是主因还是次因？

**判定：主因。** 调用频率是放大器，二者乘积导致 OOM。

### 单次 `__init__` 粗估（5000V / 30000E）
按你给的对象量级，单 adapter 复制大致：

- `_vertices`：5000 * (200~400B) ≈ **1~2 MB**
- `_edges`：30000 * (300~500B) ≈ **9~15 MB**
- `_adj_out + _adj_in`（dict + list refs + list obj）≈ **2~4 MB**
- `_edge_keys`（30000 tuple + frozenset table）≈ **3~6 MB**
- 容器头/散列表冗余/临时对象 ≈ **2~4 MB**

**保守合计：约 17~31 MB / 每次 add_edge retained 级别**  
构建过程瞬时峰值常更高（可接近 1.5~2 倍临时占用）。

### 每步 N=10 共现边（只算 cooccurrence）
- k_active: 10 次
- k_full: 10 次  
=> **20 次全量复制/步**

若按 17~31MB/次：
- **每步分配流量：约 340~620 MB**
- **20 步：约 6.8~12.4 GB（仅此路径）**

实际观测 36GB 是合理的（k_full 更大、还有 TA/其他写路径、双实例、分配器不回收 RSS）。

### “20 步后多少 adapter 待 GC？”
- 创建数（仅 cooccurrence）= 20步 * 10 * 2 = **400 个**
- 但 CPython 下多数旧对象在引用归零后会立即释放（非循环），**不一定“堆积待 GC”**。  
- 真问题是：**大对象高频分配 + arena/碎片 + 同步存在的大图状态**导致 RSS 持续上行。

---

## 问题2：fallback 纯 Python Graph 能否改善 OOM？

**判定：能明显改善，但不是最优。**

纯 Python `Graph.add_edge` 的主要成本是：
- `_edges = self._edges + [e]`：复制 30000 个引用（非对象）≈ 0.24MB
- `_adj_out/_adj_in` shallow dict copy：约 0.6~1.5MB
- `_edge_keys` frozenset union：仍是 O(E) 表重建，约 2~4MB

单次约 **3~6MB**（比 RustAdapter 全量对象重建的 17~31MB 小很多）。

### 20步对比（同样 20 add/步）
- RustAdapter 当前：约 **6.8~12.4GB** 分配流量（仅 cooc）
- 纯 Python fallback：约 **1.2~2.4GB**

=> **约 4~8 倍缓解**。  
所以 fallback 可止血，但你仍有 O(E) copy（尤其 `_edge_keys`）瓶颈。

---

## 问题3：懒加载 adapter 方案风险

### 风险1：短生命周期 adapter 反复冷启动
每次 `add_edge` 都是新 adapter。若下游在每个新 adapter 上至少访问一次 `_edges/_adj_out`，懒加载会在**每个新对象上仍触发全量复制**，收益有限。

### 风险2：若 Rust `edges`/`vertices` getter 每次都新分配
- 若缓存没真正落地（见风险3），每次属性访问都会跨 FFI 全量 materialize，**比当前更糟**。

### 风险3：`__slots__` 与 `__getattr__` 组合坑（这是硬错误）
你给的草案：
```python
__slots__ = ("_rg",)
object.__setattr__(self, "_vertices", result)
```
这会 `AttributeError`（没有该 slot 且无 `__dict__`）。  
=> 要么把缓存字段全放进 slots，要么在 slots 里加 `"__dict__"`。

### 风险4：同一 run_step 中 `_adj_out` 访问次数
从你提供片段无法精确计数；但可以确定 traversal/encounter/邻居查询会多次触发图查询。  
结论：懒加载仅在“很多步骤不访问重字段”时收益明显；你当前是写后读频繁，不是最佳止血手段。

---

## 问题4：可变 Rust Graph 方案

这里要分开判定：

- 作为“节省内存手段”技术上可行；
- 但它与当前 API 语义（`add_edge -> new graph`）存在**定义冲突**。

### 定义冲突
- **冲突方 A（现定义）**：图操作是不可变风格，旧引用代表旧状态。
- **冲突方 B（新方案）**：`add_edge_inplace` 原地改，旧引用会看到新状态。

**不可弥合理由**：同一时刻不能既“旧引用不变”又“旧引用被原地更新”。

### 你问的三点
1) daemon 是否保存旧引用做比较：你给的片段未显示，但**只要有任何旧引用语义依赖，就会被破坏**。  
2) β₁ cache：当前缓存在 adapter 对象属性上（`_cached_beta_1/_cached_terrain`）。原地改必须显式失效，否则必然脏读。  
3) Rust crate 改造复杂度：高（API、借用/并发语义、PyO3 暴露、测试矩阵全改），不适合“今天上线止血”。

---

## 问题5：增量 adapter（纯 Python 侧）判定

**判定：这是“实现错误”的直接修复路径，可今天部署。**

你提的关键点判断：
1) `_edges + [e]` 是 O(E) 引用复制，**但远小于全量重建 dataclass/dict/adj/tuple**；通常节省 4~8x 内存分配。  
2) `is` 判断可以省掉：更稳妥做法是对被修改 key 的 list 直接 copy-on-write 一次，不依赖 identity 推断。  
3) `_active_ids` 继承仅对 add_edge 成立；add_vertex / vertex batch 必须更新（建议从 `new_rg.active_vertex_ids()` 取一次，保证正确）。  
4) `add_edges_batch` 必须同步改增量，否则 batch 仍回退 `_wrap` 全复制，收益被吃掉。

---

## 三、主因判定（汇总）

- **主驱动**：`RustGraphAdapter.add_edge -> _wrap -> __init__` 的全量复制（O(V+E)对象分配）。
- **次驱动**：`_pull_cooccurrence_edges` 逐条调用 add_edge（高频放大）。
- **较次驱动**：纯 Python 的 list/frozenset O(E) 拷贝（有成本，但显著小于全量对象重建）。

---

## 四、方案评分

| 方案 | 实现复杂度 | OOM 缓解效果（相对当前） | 今天可部署？ | 主要风险 |
|---|---:|---:|---|---|
| fallback 纯 Python（去掉 Rust） | 低 | 中高（约 4~8x 降低写路径分配） | 可 | CPU 变慢；仍有 O(E) copy |
| 懒加载 adapter | 中 | 不确定（0~中） | 勉强可 | `__slots__` 实现坑；每新 adapter 冷启动仍可能全拷贝 |
| 可变 Rust Graph | 高 | 高 | 不可 | **定义冲突**（不可变语义被破坏）、cache/并发风险 |
| 增量 adapter（纯 Python 侧） | 中 | 高（约 70~90% 写路径分配下降） | 可 | 需保证 batch/active_ids/cache 语义一致 |
| `_pull_cooccurrence_edges` 改 batch | 低~中 | 高（N 次复制降到 1 次，常见 5~10x） | 可 | 需处理去重与顺序语义 |

---

## 五、推荐方案排序

1. **最小可行修复（今天止血）**  
   - 先改 `_pull_cooccurrence_edges`：收集后 `add_edges_batch` 一次写入 k_active/k_full。  
   - 同时将 RustGraphAdapter 的 `add_edges_batch` 先做“单次复制路径”优化（不要循环 add_edge）。

2. **中期（1~2天）**  
   - 落地“增量 adapter”四个方法（下方代码），消除 `_wrap` 全量复制。  
   - 补基准：5000V/30000E 下单步峰值、20步RSS曲线。

3. **长期完整方案**  
   - 重新设计 Rust/Python 边界（zero-copy view / 持久化结构共享 / COW 索引），再决定是否引入可变 Rust 图（需先做语义迁移）。

---

## 六、增量 adapter 正确实现（四方法）

> 下面代码按“保持不可变接口语义”实现：返回新 adapter，不原地改。  
> 假设边唯一性键为 `(source, target, edge_type)`（与你现有 `_edge_keys` 一致）。

```python
from typing import Iterable

class RustGraphAdapter:
    __slots__ = (
        "_rg",
        "_vertices", "_edges", "_active_ids", "_adj_out", "_adj_in", "_edge_keys",
        "_cached_active_edges", "_cached_all_active_edges",
        # 若你确实有这两个缓存且 class 允许，建议也放进 slots
        # "_cached_beta_1", "_cached_terrain",
    )

    @staticmethod
    def _ek(e: "Edge") -> tuple[str, str, "EdgeType"]:
        return (e.source, e.target, e.edge_type)

    @staticmethod
    def _safe_setattr(obj, name: str, value) -> None:
        try:
            object.__setattr__(obj, name, value)
        except AttributeError:
            pass

    def _new_shell(self, new_rg: "RustGraph") -> "RustGraphAdapter":
        a = RustGraphAdapter.__new__(RustGraphAdapter)
        object.__setattr__(a, "_rg", new_rg)
        return a

    def _invalidate_runtime_caches(self, a: "RustGraphAdapter") -> None:
        # 与 edges/active 相关缓存统一失效
        object.__setattr__(a, "_cached_active_edges", None)
        object.__setattr__(a, "_cached_all_active_edges", None)

    def add_edge(self, e: "Edge") -> "RustGraphAdapter":
        k = self._ek(e)
        if k in self._edge_keys:
            return self

        new_rg = self._rg.add_edge(_py_edge_to_rust(e))
        a = self._new_shell(new_rg)

        # vertices / active_ids 不变
        object.__setattr__(a, "_vertices", self._vertices)
        object.__setattr__(a, "_active_ids", self._active_ids)

        # edges: 仅复制引用列表
        object.__setattr__(a, "_edges", self._edges + [e])

        # adj_out: copy dict + copy-on-write 单 key list
        new_adj_out = dict(self._adj_out)
        old_out = self._adj_out.get(e.source)
        new_adj_out[e.source] = ([e] if old_out is None else (list(old_out) + [e]))
        object.__setattr__(a, "_adj_out", new_adj_out)

        # adj_in
        new_adj_in = dict(self._adj_in)
        old_in = self._adj_in.get(e.target)
        new_adj_in[e.target] = ([e] if old_in is None else (list(old_in) + [e]))
        object.__setattr__(a, "_adj_in", new_adj_in)

        # edge_keys
        ks = set(self._edge_keys)
        ks.add(k)
        object.__setattr__(a, "_edge_keys", frozenset(ks))

        self._invalidate_runtime_caches(a)

        # 保持你当前逻辑：material edge 传播拓扑缓存（如果这些属性存在）
        if e.edge_type in _MATERIAL_EDGE_TYPES:
            for attr in ("_cached_beta_1", "_cached_terrain"):
                try:
                    object.__setattr__(a, attr, getattr(self, attr))
                except AttributeError:
                    pass

        return a

    def add_vertex(self, v: "Vertex") -> "RustGraphAdapter":
        vid = v.id
        if vid in self._vertices:
            return self

        new_rg = self._rg.add_vertex(_py_vertex_to_rust(v))
        a = self._new_shell(new_rg)

        new_vertices = dict(self._vertices)
        new_vertices[vid] = v
        object.__setattr__(a, "_vertices", new_vertices)

        # 边与索引不变（共享）
        object.__setattr__(a, "_edges", self._edges)
        object.__setattr__(a, "_adj_out", self._adj_out)
        object.__setattr__(a, "_adj_in", self._adj_in)
        object.__setattr__(a, "_edge_keys", self._edge_keys)

        # active 集合可能改变：从 rust 取一次，保证语义正确
        object.__setattr__(a, "_active_ids", frozenset(new_rg.active_vertex_ids()))

        self._invalidate_runtime_caches(a)
        return a

    def add_edges_batch(self, edges: Iterable["Edge"]) -> "RustGraphAdapter":
        es = list(edges)
        if not es:
            return self

        new_rg = self._rg.add_edges_batch([_py_edge_to_rust(e) for e in es])

        # 依据 edge_key 去重（与当前 _edge_keys 语义一致）
        ks = set(self._edge_keys)
        to_add: list["Edge"] = []
        for e in es:
            k = self._ek(e)
            if k in ks:
                continue
            ks.add(k)
            to_add.append(e)

        if not to_add:
            return self

        a = self._new_shell(new_rg)

        object.__setattr__(a, "_vertices", self._vertices)
        object.__setattr__(a, "_active_ids", self._active_ids)
        object.__setattr__(a, "_edges", self._edges + to_add)

        # adj_out: 每个 source 仅 copy-on-write 一次
        new_adj_out = dict(self._adj_out)
        copied_src: set[str] = set()
        for e in to_add:
            s = e.source
            if s not in copied_src:
                base = self._adj_out.get(s)
                new_adj_out[s] = ([] if base is None else list(base))
                copied_src.add(s)
            new_adj_out[s].append(e)
        object.__setattr__(a, "_adj_out", new_adj_out)

        # adj_in: 每个 target 仅 copy-on-write 一次
        new_adj_in = dict(self._adj_in)
        copied_tgt: set[str] = set()
        for e in to_add:
            t = e.target
            if t not in copied_tgt:
                base = self._adj_in.get(t)
                new_adj_in[t] = ([] if base is None else list(base))
                copied_tgt.add(t)
            new_adj_in[t].append(e)
        object.__setattr__(a, "_adj_in", new_adj_in)

        object.__setattr__(a, "_edge_keys", frozenset(ks))
        self._invalidate_runtime_caches(a)

        if all(e.edge_type in _MATERIAL_EDGE_TYPES for e in to_add):
            for attr in ("_cached_beta_1", "_cached_terrain"):
                try:
                    object.__setattr__(a, attr, getattr(self, attr))
                except AttributeError:
                    pass

        return a

    def add_vertices_and_edges_batch(
        self,
        vertices: Iterable["Vertex"],
        edges: Iterable["Edge"],
    ) -> "RustGraphAdapter":
        vs = list(vertices)
        es = list(edges)
        if not vs and not es:
            return self

        new_rg = self._rg.add_vertices_and_edges_batch(
            [_py_vertex_to_rust(v) for v in vs],
            [_py_edge_to_rust(e) for e in es],
        )

        # vertices 增量
        new_vertices = dict(self._vertices)
        for v in vs:
            if v.id not in new_vertices:
                new_vertices[v.id] = v

        # edges 增量去重
        ks = set(self._edge_keys)
        to_add: list["Edge"] = []
        for e in es:
            k = self._ek(e)
            if k in ks:
                continue
            ks.add(k)
            to_add.append(e)

        a = self._new_shell(new_rg)

        object.__setattr__(a, "_vertices", new_vertices)
        object.__setattr__(a, "_edges", self._edges + to_add)

        new_adj_out = dict(self._adj_out)
        copied_src: set[str] = set()
        for e in to_add:
            s = e.source
            if s not in copied_src:
                base = self._adj_out.get(s)
                new_adj_out[s] = ([] if base is None else list(base))
                copied_src.add(s)
            new_adj_out[s].append(e)
        object.__setattr__(a, "_adj_out", new_adj_out)

        new_adj_in = dict(self._adj_in)
        copied_tgt: set[str] = set()
        for e in to_add:
            t = e.target
            if t not in copied_tgt:
                base = self._adj_in.get(t)
                new_adj_in[t] = ([] if base is None else list(base))
                copied_tgt.add(t)
            new_adj_in[t].append(e)
        object.__setattr__(a, "_adj_in", new_adj_in)

        object.__setattr__(a, "_edge_keys", frozenset(ks))

        # 顶点批量可能改 active set：取 rust 真值
        object.__setattr__(a, "_active_ids", frozenset(new_rg.active_vertex_ids()))

        self._invalidate_runtime_caches(a)
        return a
```

---

## 七、内存估算（定量）

以 5000V / 30000E、每步 cooc N=10（k_active+k_full 共 20 次 add）：

- **当前 RustAdapter 全量复制**：  
  约 **340~620MB / 步**（仅 cooc 路径）

- **增量 adapter（不改调用频率）**：  
  单 add 约 3~6MB => **60~120MB / 步**

- **节省比例**：  
  约 **65%~85%**（保守）  
  若再叠加 cooc batch（10条 -> 1次）：可进一步到 **>90%** 级别。

---

如果你愿意，我下一步可以直接给你一个**最小差异补丁清单**（按文件/函数级别，含修改顺序与回归测试点），让你今天就能上线止血。
