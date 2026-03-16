# Codex diagnose — 2026-03-16 05:43:30 UTC

## 元数据

- **mode**: diagnose
- **subject**: traversal worker 子进程在步数 20 时 OOM 59GB——根因排序
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-16 05:43:30 UTC
- **context-file**: G:/NewChanlun/tmp/codex-diagnose-ctx.md

## Prompt

## 诊断目标

traversal worker 子进程在步数 20 时 OOM 59GB——根因排序

## 上下文

# Codex 诊断上下文：穿越 OOM 根因分析

## 任务

诊断 traversal worker 子进程在步数 20 时占 59GB 被 OOM 杀掉的根因。

**不要提修复方案。只做根因诊断和排序。**

---

## 系统架构概述

逢亮是一个自主穿越引擎：
- **K_active**：当前活跃概念图（fold/negate/sublate 操作返回新对象）
- **K_full**：历史完整图（永不清理，仅追加）
- **S_net**：能指网络，263K 能指，456K 边（hyperedges）
- **daemon.py**：主进程，持有 k_active/k_full/engine/encounter_log
- **traversal.py/TraversalEngine**：穿越引擎，每步 run_step()
- **multiprocessing fork**：子进程继承父进程内存

已尝试三个方案，全部 OOM：
1. 纯 Python Graph：每步 add_edge 创建新对象 → OOM
2. Plan A 零拷贝代理（AdjProxy FFI 风暴）→ 仍 OOM（3分钟 52GB）
3. Plan B+ 版本化缓存 RustGraphAdapter → 仍 OOM（traversal worker 59GB 被杀）

Plan B+ 下主进程 5 分钟后 1.4GB（合理），但 traversal worker **子进程**在步数 20 时 59GB 被杀。

---

## 关键代码片段

### 1. Rust Graph.add_edge（图克隆模式）

```rust
// graph-rs/src/graph.rs:623
// Graph 结构：
// #[derive(Clone, Debug)]
// pub struct Graph {
//     vertices: HashMap<String, Vertex>,   // 所有顶点
//     edges: Vec<Edge>,                    // 所有边（历史累积）
//     adj_out: HashMap<String, Vec<usize>>,  // vertex_id -> edge indices
//     adj_in: HashMap<String, Vec<usize>>,
//     active_ids: HashSet<String>,
//     edge_keys: HashSet<EdgeKey>,
//     active_rev: u64,
//     edge_rev: u64,
// }

fn add_edge(&self, e: Edge) -> Self {
    let mut g = self.clone();  // 完整克隆：vertices + edges + adj_out/adj_in + edge_keys
    let idx = g.edges.len();
    let key = edge_key(&e);
    g.adj_out.entry(e.source.clone()).or_default().push(idx);
    g.adj_in.entry(e.target.clone()).or_default().push(idx);
    g.edge_keys.insert(key);
    g.edges.push(e);
    g.edge_rev += 1;
    g
}

fn add_vertex(&self, v: Vertex) -> Self {
    let mut g = self.clone();  // 完整克隆
    // ...
    g
}
```

**关键点**：`derive(Clone)` = 深克隆。每次 add_edge/add_vertex：
- 克隆所有 vertices HashMap
- 克隆所有 edges Vec（含全部历史边）
- 克隆 adj_out/adj_in（含全部邻接表）
- 克隆 edge_keys HashSet

### 2. RustGraphAdapter._wrap_versioned（Python 侧包装器）

```python
# engine.py:792
def _wrap_versioned(self, new_rg: "RustGraph", active_changed: bool, edge_changed: bool) -> "RustGraphAdapter":
    a = RustGraphAdapter.__new__(RustGraphAdapter)
    object.__setattr__(a, '_rg', new_rg)          # 新 Rust Graph 对象（旧的暂时保留）
    object.__setattr__(a, '_adj_out', _AdjProxy(new_rg, "out"))  # 新代理对象
    object.__setattr__(a, '_adj_in', _AdjProxy(new_rg, "in"))
    object.__setattr__(a, '_vertices', _VerticesProxy(new_rg))
    object.__setattr__(a, '_edges', _EdgesProxy(new_rg))
    # ... 缓存传播逻辑
    return a  # 返回新 adapter，旧 adapter 等待 Python GC
```

**关键问题**：旧 `RustGraphAdapter._rg`（Rust Graph 句柄）何时被 Drop？
- Python GC 不是即时的（引用计数为 0 时立即回收，但 Rust 侧 Drop 依赖 PyO3 的 `__del__`）
- 如果有引用循环 → GC 延迟 → Rust 堆内存积压

### 3. _pull_cooccurrence_edges（每步调用，无边数上限）

```python
# traversal.py:1010
def _pull_cooccurrence_edges(self) -> None:
    # 查询当前位置对应能指的所有超边
    for he in snet.hyperedges_containing(signifier_id):
        for v in he.vertices:
            if v != signifier_id:
                hyperedge_neighbors.add(v)
    # hyperedge_neighbors 大小 = 当前位置所在的所有超边的总顶点数 - 重复项
    # 无上限守卫

    for target_sig in hyperedge_neighbors:
        target_concepts = self._snet_activation._sig_to_concepts.get(target_sig, [])
        if target_concepts:
            for tgt_cid in target_concepts:
                # ...已有顶点 → add COOC edge
                self.k_active = self.k_active.add_edge(cooc_edge)  # Rust clone
                self.k_full = self.k_full.add_edge(cooc_edge)       # Rust clone
        else:
            # 无 K_active 顶点 → 创建新顶点 + 边
            self.k_active = self.k_active.add_vertex(new_vertex)   # Rust clone
            self.k_full = self.k_full.add_vertex(new_vertex)        # Rust clone
            self.k_active = self.k_active.add_edge(cooc_edge)       # Rust clone
            self.k_full = self.k_full.add_edge(cooc_edge)           # Rust clone

            # 注意：新顶点 ID 写入 _sig_to_concepts（永久增长）
            self._snet_activation._sig_to_concepts.setdefault(target_sig, []).append(new_vid)
```

**关键点**：
- 每步 N 个新 COOC 邻居 → 最少 2N 次 Rust Graph clone（k_active + k_full 各 N 次）
- 大多数邻居没有 K_active 对应顶点 → 路径走 `else` → 每个邻居 4 次 clone（两次 add_vertex + 两次 add_edge）
- `_sig_to_concepts` 字典永久增长（每创建一个 cooc_ 顶点就 append）

### 4. run_step 每步触发的 Rust clone 次数

每步 run_step 触发的 add_edge/add_vertex（每次 = 1 次 Rust Graph clone）：

| 操作 | 触发条件 | k_active | k_full | 总 clone |
|------|---------|---------|--------|---------|
| COOC 边（已有顶点） | 每步，N 个邻居 | N | N | 2N |
| COOC 顶点+边（新顶点） | 每步，M 个新邻居 | 2M | 2M | 4M |
| TRAVERSAL_ASSOCIATION | 每步（位置变化） | 1 | 1 | 2 |
| encounter 操作（fold/negate/sublate） | 遇到时 | 多次 | 多次 | 多次 |
| settlement memory nodes | 结算时 | 多次 | 多次 | 多次 |
| articulation feedback | 每步（check） | 0-多次 | 0-多次 | 0-多次 |
| proprioception | 每 100 步 | 少量 | 少量 | 少量 |

若每步 N=50 个超边邻居，且大多数是新顶点（M=40）：
- 每步触发 ≈ 2×10 + 4×40 + 2 = 182 次 Rust Graph clone

**每次 Rust clone 的代价**：
- 假设步数 10 时 K_full 有 500 顶点、500 边
- 克隆 edges Vec：500 × sizeof(Edge) ≈ 500 × ~500 bytes（含 String fields） ≈ 250KB
- 克隆 vertices HashMap：同量级
- 克隆 adj_out/adj_in：类似
- 单次 clone ≈ 1-2MB
- 182 次 × 1MB = 182MB/步（峰值，旧克隆若立即 Drop 则只是峰值而非累积）

**若旧 Rust Graph 不能及时 Drop**：
- 每步积压 182 个 Rust Graph 克隆
- 20 步后积压 3640 个克隆 × 1-2MB = 3-7GB（不够解释 59GB）

### 5. S_net 在 fork 前的内存占用

S_net：263K 能指，456K 边（超边）
- 每个能指：~100-200 bytes
- 每条超边：包含多个顶点 ID（String），~200-1000 bytes
- 估算：263K × 150 + 456K × 500 = ~39MB + ~228MB ≈ 270MB

S_net 在 fork 前已加载 → 子进程继承 ~270MB（CoW，只读不触发 CoW）

### 6. daemon._step 中 k_active/k_full 的引用链

```python
# daemon.py:1186
def _step(self) -> None:
    pre_vid_count = len(self.k_full._vertices)  # 触发 _VerticesProxy.__len__ → FFI
    pre_edge_count = len(self.k_full._edges)    # 触发 _EdgesProxy.__len__ → FFI
    pre_vid_keys = self.k_full._vertices.keys() # ← 这是 _VerticesProxy，不是 dict_keys！

    log = self.engine.run_step()  # engine 内部 k_active/k_full 多次更新

    self.k_active = self.engine.k_active  # daemon 引用更新
    self.k_full = self.engine.k_full      # daemon 引用更新

    # 旧 k_active/k_full（run_step 之前的对象）现在只有局部变量持有？
    # 但 pre_vid_keys = self.k_full._vertices.keys() 在 step 开始时捕获
    # 这是对 _VerticesProxy 的引用（不是对 Graph 的引用）
    # _VerticesProxy 持有对旧 RustGraphAdapter._rg 的引用 → 阻止 Rust Graph Drop

    # ... 1321行：
    for vid in self.k_full._vertices:
        if vid not in pre_vid_keys:  # pre_vid_keys 是 _VerticesProxy
            new_vids.add(vid)
```

**关键引用问题**：
- `pre_vid_keys = self.k_full._vertices.keys()` 捕获了旧 `k_full._vertices`（一个 `_VerticesProxy`）
- `_VerticesProxy` 持有对旧 `_rg`（Rust Graph 对象）的引用
- 在 `_step()` 函数结束之前，旧 Rust Graph 无法被 Drop
- 但这只是函数局部作用域 → 函数返回后应释放

---

## 需要 Codex 诊断的核心问题

**问题 1**：`_pull_cooccurrence_edges` 每步拉入多少 COOC 边/顶点？
- S_net 超边：一个词的超边邻居数量分布是什么？能否达到数千？
- 现有代码完全没有上限守卫（只有 `existing_cooc_targets` 去重当步内重复）

**问题 2**：Rust Graph clone 链中的内存积压
- 每次 add_edge：`self.clone()` → 新 Rust Graph → 旧 Rust Graph 等待 Drop
- 在 `_pull_cooccurrence_edges` 循环中：每个 COOC 边两次 add_edge → k_active 和 k_full 各积压 N 个中间 clone
- Python 局部变量 `self.k_active = self.k_active.add_edge(e)` 的旧值：是否立即 Drop？
  - Python 赋值语句：`self.k_active = self.k_active.add_edge(e)` → 先计算右侧（产生新 adapter），再赋值 → 旧 adapter refcount 降为 0 → 触发 PyO3 Drop → Rust Graph Drop
  - 理论上应即时回收 → 但 20 步 59GB 仍无法解释

**问题 3**：multiprocessing fork 的 Copy-on-Write 放大
- fork 子进程继承父进程所有内存（CoW）
- 父进程若已有大量 Python 对象（S_net 加载后的各种数据结构），fork 后任何写操作都触发 CoW 页面分配
- 特别是：Python 引用计数本身（每次对象访问修改 refcount）就是写操作 → 触发大量 CoW
- S_net 263K 能指的引用计数在穿越时被频繁读取（激活），每次读取就是 CoW 页面分配
- 估算：如果父进程有 4GB 内存，每页 4KB，CoW 全部触发 → 子进程额外 4GB
- 但 59GB 远超此估算

**问题 4**：`self._snet_activation._sig_to_concepts` 无限增长
- 每步创建新 cooc_ 顶点时：`self._snet_activation._sig_to_concepts.setdefault(target_sig, []).append(new_vid)`
- 这个字典累积所有已创建的 cooc_ 映射
- 但 K_full 顶点已经增长了，这只是索引 → 内存增长有限

**问题 5**：`self.logs`（StepLog 列表）无限增长
- TraversalEngine.logs：每步 append StepLog
- 循环检测路径（traversal.py:1565）也 append StepLog
- 每条 StepLog 引用当前步的状态（但只是数值，不是 Graph 对象）
- 20 步 = 20 个 StepLog，内存可忽略

**问题 6**：`visit_history` 无限增长
- `self.visit_history`：每步 append 当前位置 ID（字符串）
- 20 步 = 20 个字符串，可忽略

---

## 我的初步排序假设（待 Codex 验证）

### 最可能的根因

**候选 A：每步 COOC 边数量无上限 × Rust 全克隆**
- 如果每步位置有大型超边（100+ 邻居）
- 大多数邻居没有 K_active 对应顶点 → `else` 路径：4 次 Rust clone
- 100 邻居 × 4 clone = 400 Rust Graph clone/步
- K_full 在步数增长时本身也在增大（更多边 → 更大的 clone）
- 步数 5 时 K_full 有 500 边：每 clone ≈ 1MB → 400MB 峰值
- 步数 10 时 K_full 有 1000 边：每 clone ≈ 2MB → 800MB 峰值
- 步数 20 时 K_full 有 2000 边：每 clone ≈ 4MB → 1.6GB 峰值
- 但这假设旧 clone 及时回收 → 仍不够解释 59GB

**候选 B：multiprocessing fork CoW 放大 + S_net 引用计数写入**
- 父进程 S_net 加载后：内存可能 2-4GB（含 Python 对象头部、dict/list overhead）
- fork 子进程：所有页面 CoW 标记
- 穿越时 `snet.hyperedges_containing(signifier_id)` 遍历 S_net → 访问大量 Python 对象 → refcount++ → 页面写入 → CoW 触发
- 如果 S_net 实际占用 4GB（Python dict/list overhead 放大），CoW 全触发 = 子进程额外 4GB
- 加上 Rust Graph clone 的直接分配 → 累积到 59GB？

**候选 C：Python Graph（纯 Python 模式）下的 _edges 列表复制**
- 纯 Python `add_edge`：`new_graph._edges = self._edges + [e]`（Python 列表连接）
- 每次连接创建新 list，旧 list 等 GC
- 若 GC 不够快（Python GC 的 threshold 机制），列表大量积压
- 但 Plan B+ 已切换到 Rust → 此候选不适用于当前 59GB 情况

**候选 D：Rust Graph clone 的字符串复制放大**
- Rust `HashMap<String, Vertex>` 克隆：每个 String 单独分配（不是 Arc<str>，不是共享）
- vertices 中每个顶点的 `content` 字段可能是长字符串（S_net 能指文本）
- 如果 cooc_ 顶点的 content = 完整能指字符串（可能很长）
- 1000 顶点 × 平均 200 字符 = 200KB 字符串数据/clone
- 加上 edges 中的 context 字符串（`snet_hyperedge: {signifier_id}->{target_sig}`）
- 1000 edges × 平均 100 字符 context = 100KB/clone
- 仍然有限

---

## 真实规模数据（已知）

- S_net：263K 能指（signifiers），456K 超边（hyperedges）
- 平均每个超边顶点数：未知（关键数据）
- 平均每个能指所在的超边数：未知（关键数据）
- 父进程 fork 前实际内存：未知（关键数据）
- traversal worker 启动后达到 59GB 的时间：步数 20（约数分钟）

## Codex 诊断任务

请基于以上代码证据：

1. 确认每次 `add_edge`（Rust 侧）的内存分配量是否能累积到 59GB
2. 确认 `_pull_cooccurrence_edges` 是否存在真正的无上限边创建路径
3. 确认 multiprocessing fork CoW 是否是主要贡献者
4. 给出根因排序（主因/次因/次次因）
5. 指出代码中任何我遗漏的内存增长路径


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

收到。按你的约束：**只做根因诊断与排序，不给修复方案**。

---

## 0) 失败现象（对象）

- 现象：`traversal worker` 子进程在约第 20 步时 RSS 达 **59GB**，被 OOM killer 杀掉。
- 对照：同阶段主进程约 1.4GB，说明异常主要在 worker 的步进执行路径内，而非全局静态常驻。

---

## 1) 结论先行：根因排序

## **主因（Rank #1）**
**Rust Graph 在 `add_edge/add_vertex` 上执行全图深克隆（`self.clone()`）+ 被高频调用，形成乘法级内存分配压力。**

- 直接原因（代码级）  
  `graph-rs/src/graph.rs` 的：
  - `add_edge(&self, e) -> Self { let mut g = self.clone(); ... }`
  - `add_vertex(&self, v) -> Self { let mut g = self.clone(); ... }`  
  每次变更都复制 `vertices + edges + adj_out + adj_in + edge_keys` 全量结构。

- 根本原因（设计级）  
  “高频增量写入场景”使用了“每次写入全量复制”的实现语义，复杂度从增量 O(1)/O(log n) 退化为 **每次 O(|G|)**；在 traversal 的批量写路径下变成 **O(写入次数 × 图规模)**。

- 分类  
  **实现错误**（实现策略与运行负载不匹配，不是定义本身要求全量复制）。

---

## **次因（Rank #2）**
**`_pull_cooccurrence_edges` 存在真实的无上限扩张路径，导致每步写入次数本身可非常大。**

- 直接原因（代码级）  
  `traversal.py:1010` 逻辑：
  1. 遍历 `hyperedges_containing(signifier_id)` 的全部超边；
  2. 对超边中全部顶点并集 `hyperedge_neighbors` 全量处理；
  3. 每个 target 再遍历 `target_concepts`；
  4. 无 `top-k` / 无 hard cap / 无预算中止。  
  并且对 `k_active` 和 `k_full` 各写一次；“新顶点路径”每目标触发 4 次 clone。

- 根本原因（设计级）  
  该函数把“共现邻域实体化”定义成**全量展开**，没有资源边界约束，直接把 S_net 局部高出度映射为大量图变更调用。

- 分类  
  **实现错误**（缺少边界控制是实现层问题；代码确实无上限守卫）。

---

## **次次因（Rank #3，放大项）**
**`k_full` 仅追加（永不清理）+ `_sig_to_concepts` 持续增长，使 clone 的“单次体积”随步数递增。**

- 直接原因（代码级）  
  - `k_full` 每步继续追加边/点；
  - `_sig_to_concepts.setdefault(...).append(new_vid)` 单调增长。  
  结果是 `self.clone()` 的拷贝基数越来越大。

- 根本原因（设计级）  
  历史全保留语义导致状态规模单调上升；在“全量 clone 变更模型”下，成本被二次放大。

- 分类  
  这里有两层：
  - OOM 触发方式本身：**实现错误放大**（因为 clone 模型不该配这种增长）。
  - “状态无界增长”本身：**定义约束**（不是 bug，而是语义选择）。

---

## **低位放大因（Rank #4）**
**fork CoW（含 refcount 写触发页复制）是次要放大，不是主因。**

- 证据链  
  - CoW 上限受父进程触达页规模约束；你给出的主进程量级约 1.4GB，不足以解释 59GB 主体。
  - 且 OOM 与步进写入增长强相关（step 20），更符合“运行期分配风暴”，而非一次性 CoW 平台效应。

- 分类  
  **实现/运行时放大项**，非根因。

---

## 2) 对你 5 个问题的逐条判定

### 问题1：`_pull_cooccurrence_edges` 每步拉入多少？
- **无法从现有材料给出真实分布值**（缺少度分布统计）。
- 但可**确定**：路径是无上限的；理论上邻居规模上界可到该 signifier 所在超边并集的全部顶点（极端可很大），再乘 `target_concepts` fanout。
- 结论：存在“每步数千甚至更高写入”的**机制可能性**，且代码未阻断。

### 问题2：Rust clone 链是否能到 59GB？
- **可以成立**。  
  关键不是单次 clone，而是：
  \[
  A_t \approx C_t \times S_t,\quad C_t=(2N_t+4M_t+\dots),\ S_t \propto |G_t|
  \]
  `C_t` 无上限、`S_t` 随步增长，形成乘法放大。  
- 即便对象可回收，RSS 仍可因持续大分配压力上冲；若存在任何延迟释放/短期并存引用，会进一步加剧。

### 问题3：fork CoW 是否主要贡献者？
- **不是主要贡献者**（按现有证据）。
- 可贡献若干 GB 级放大，但与 59GB 主量级不匹配，也不解释“随步增长到第20步爆”的形态。

### 问题4：根因排序
1. `add_edge/add_vertex` 全图深克隆（主因）
2. `_pull_cooccurrence_edges` 无上限全量展开（次因）
3. `k_full` 仅追加导致 clone 基数持续增大（次次因）
4. fork CoW 放大（低位放大因）

### 问题5：你可能遗漏的内存增长路径
你已覆盖大部分关键点；还应明确两条：
1. **`target_concepts` 内层 fanout**：不是“一邻居一边”，而是“一邻居 × 多 concept”。
2. **双图同步写 (`k_active` + `k_full`)**：所有写入成本至少翻倍，不只是 `k_full` 单边增长。

---

## 3) 定义冲突判定（仅标记）

存在一个结构性冲突（非本次主触发，但是背景）：

- **冲突方 A**：`k_full` 历史完整、永不清理，且 COOC 邻域可全量实体化。  
- **冲突方 B**：worker 内存必须在有限资源内稳定，不发生 OOM。  
- **不可弥合理由**：在有限内存机器上，“无上限输入展开 + 状态永不收缩”语义本身导致状态规模无界；不改变至少一侧语义边界，不能同时长期满足两者。

---

如果你要，我下一步可以只做一件事：把这个排序转成**可验证的观测指标清单**（每步 `N/M/C_t/S_t/RSS` 的最小埋点集合），不涉及修复。
