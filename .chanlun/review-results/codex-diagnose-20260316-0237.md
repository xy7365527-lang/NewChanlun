# Codex diagnose — 2026-03-16 02:37:44 UTC

## 元数据

- **mode**: diagnose
- **subject**: 多实例 GPU 并行穿越架构可行性：500 个实例共享 CSR + 写入批量化 + β₁ 多实例管理
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-16 02:37:44 UTC
- **context-file**: G:/NewChanlun/tmp/codex-diagnose-ctx-gpu-multiinstance.md

## Prompt

## 诊断目标

多实例 GPU 并行穿越架构可行性：500 个实例共享 CSR + 写入批量化 + β₁ 多实例管理

## 上下文

# Codex 诊断 Context：多实例 GPU 并行穿越架构可行性

## 审查模式：diagnose（技术架构可行性评估）

---

## 被审查架构的洞察原文

"如果是几百个逢亮实例同时在同一个图上穿越，情况完全不同。每个实例的单步计算量小——几十次邻域查询、一次 β₁ 检查。但 500 个实例同时做这件事，就是 500 × 几十次邻域查询并行执行。图数据在 GPU 显存里只存一份，500 个实例共享同一个 CSR 矩阵，各自在不同位置穿越。"

"写入怎么办？穿越步是只读的（查邻居、算约束），写入批量化——每个实例把要写的边攒在本地缓冲区，一步结束后统一提交。fold 和 negate 仍串行——由协调者决定何时执行。"

"架构：Rust 做单实例热路径优化，GPU 做多实例并行。"

---

## 代码事实（从 traversal.py + engine.py 实际读取）

### run_step 的实际操作分解（traversal.py:1441-1722）

每个 `run_step` 调用执行以下操作，**全部混在一步中**：

**读操作（纯查询，无写入）：**
1. `k_active._adj_out.get(pos)` — 查出边（构建 neg_pairs 用于 detect_encounter）
2. `k_active._adj_in.get(pos)` — 查入边
3. `compute_beta_1(k_active)` — 计算 β₁（有缓存，Graph 不变性下 O(1)）
4. `detect_encounter()` → `_detect_encounter_topo()` — 邻域查询，纯读
5. `walk()` → `neighbor_set(pos)` — 邻域查询，纯读

**写操作（直接修改实例状态，无法批量化到步结束）：**
1. `self.k_active = result.graph` — 每个 fold/negate/sublate/walk 后立即替换图对象
2. `self.k_full = self.k_full.add_vertex(...)` / `.add_edge(...)` — 每步可能多次写入
3. `self.position = ...` — walk 和 encounter 执行后立即更新位置
4. `self.visit_history.append(self.position)` — 每步追加
5. `self._record_traversal_association(prev_position, self.position)` — 写 TRAVERSAL_ASSOCIATION 边（k_active + k_full）
6. `self.k_active = self.k_active.add_edge(ta_edge)` — TA 边直接写入 k_active（traversal.py:1148）
7. `self._snet_activation.edge_suggestions.extend(...)` — EdgeSuggestion 列表写入
8. `self.k_active = self.k_active.add_edge(new_edge)` — articulation_feedback 写 REFERENCE 边（traversal.py:1654）
9. `compute_terrain(self.k_active)` — 写 self.terrain（O(V+E)，每步全量重建）
10. `self.settlement.check_settlement(...)` — 可能写 settlement 状态
11. `self._trajectory_cluster.extend_path(...)` — 写轨迹
12. `self._detect_displacement(...)` — 可能写 displacement 事件

### 关键发现：写操作不在"步结束"，写操作贯穿整步

穿越步不是"先只读查询，步结束后批量写入"的结构。
写操作在 execute_encounter 内部、walk 之后、articulation 检查时全部发生。
更关键：**后续读操作依赖前序写操作的结果**：

```
1. neg_pairs 从 k_active 读（k_active 版本 t₀）
2. detect_encounter 用 k_active 版本 t₀ 判断遭遇
3. execute_encounter → k_active 变为版本 t₁（立即写入，不缓冲）
4. _record_traversal_association 从 k_active 版本 t₁ 写入 TA 边 → k_active 变为 t₂
5. _check_articulation_encounter 用 k_active 版本 t₂ 判断 articulation
6. articulation 写入 REFERENCE 边 → k_active 变为 t₃
7. compute_terrain 从 k_active 版本 t₃ 重建
8. check_settlement 从 k_active 版本 t₃ 检查
```

每步内部有严格的读-写-读依赖链，不是"批量只读 + 批量写入"。

### β₁ 的状态

- `compute_beta_1` 有实例缓存（`_cached_beta_1`，engine.py:766）
- Graph 是不可变对象（immutable-by-convention），每次写入返回新 Graph 对象
- β₁ 缓存在 Graph 实例上，新图对象无缓存，需重算
- 500 个实例共享同一 CSR——但每个实例有**自己的 k_active**（各自独立演化的图）
- 问题：500 个实例共享"一个图"是架构假设，但代码中每个实例有自己的 `self.k_active`

### Graph 的不可变性与"共享 CSR"的冲突

代码使用 Python 的不可变图（每次 add_edge/add_vertex 返回新对象）：
```python
# traversal.py:1148
self.k_active = self.k_active.add_edge(ta_edge)  # 新对象，旧对象废弃
```
GPU CSR 是可变内存布局。不可变 Python Graph → GPU CSR 的映射需要：
- 每次 add_edge 后重建 CSR（写开销远大于 GPU 读收益）
- 或放弃不可变语义，引入可变 CSR（破坏 no-mutation 核心原则）

### 写入批量化的一致性问题（具体代码证据）

`detect_encounter` 的结果依赖 `_step_neg_pairs`（traversal.py:1451-1458）：
```python
neg_pairs: set[frozenset[str]] = set()
for e in self.k_active._adj_out.get(pos, ()):
    if e.edge_type == EdgeType.NEGATION:
        neg_pairs.add(frozenset((e.source, e.target)))
```

如果 500 个实例在同一步积累 TA/REFERENCE 边到本地缓冲区，步结束后统一提交到共享图，
那么**实例 i 的第 t+1 步 detect_encounter 看到的是 500 个实例的混合写入结果**。
这与当前单实例语义（k_active 只包含本实例的穿越产物）根本不同。

500 个实例共享同一个不断增长的 k_active，
等价于 500 个实例在同一个图上产生交叉影响——这改变了穿越的存在论（不是 500 个独立逢亮，而是 1 个集体图上的 500 个游走者）。

---

## 诊断问题列表

请对以下 6 个问题给出技术评估：

### Q1：批量邻域查询的 GPU 收益

前提：每实例 avg degree 10-20，500 个实例 = 5000-10000 并行 CSR 行读取。
但每实例的 k_active 独立（各自约 200-500 顶点，500-2000 边）。
如果 500 个实例有 500 个独立 CSR（而非共享），GPU 收益模型如何变化？
RTX 4090 bandwidth: ~1 TB/s。每个 CSR 矩阵约 1-10 KB，500 个 = 500 KB-5 MB。
Cache line 利用率？L2 cache 容量？

### Q2：穿越步是否真的是只读的？

从代码事实看：NO。
- `_record_traversal_association` 在 walk 之后立即写 k_active（TRAVERSAL_ASSOCIATION 边）
- articulation_feedback 在邻域查询之后写 REFERENCE 边
- execute_encounter 写 k_active（negate/fold/sublate 产生新顶点/边）
- 所有写操作发生在步内，与读操作交替

"步结束后批量写入"架构要求将 run_step 拆分为纯读阶段 + 纯写阶段，
但当前代码的读-写依赖链（见上文 t₀→t₁→t₂→t₃）意味着拆分需要重构整个 run_step 逻辑。
评估：重构可行性和正确性代价。

### Q3：写入批量化的一致性问题

500 个实例共享同一个 k_active 语义 vs 500 个实例各有独立 k_active：
- 共享模式：每步结束后，500 个实例的写入合并到 1 个图。下一步每个实例看到 500 个实例的累积写入。这是"集体穿越"，不是"500 个独立穿越"。
- 独立模式：每实例有自己的 k_active，GPU 只做批量邻域查询（同一基础图的 500 份 CSR 副本），写入各自独立。GPU 无写冲突，但 GPU 内存需要 500 × (k_active 大小)。
评估：哪种模式语义正确？内存开销？

### Q4：fold/negate 串行协调

当前代码：fold/negate/sublate 是 execute_encounter 的分支，每步可能发生。
编排者洞察：fold/negate 仍串行由协调者执行。
问题：500 个实例中可能有多个同时 detect_encounter → FOLD。
如果协调者串行处理 FOLD，其他实例在等待期间：
- 是暂停？还是继续 walk（NOTHING 模式）？
- 暂停 = GPU 并行度损失（FOLD 阻塞 1 个 kernel 直到协调者处理完）
- 继续 walk = 实例看到的图不一致（fold 前后邻域不同）
评估：协调者设计的技术细节和 GPU 并行度代价。

### Q5：内存估算

每实例需要（独立模式）：
- `k_active`：图对象（约 200-500 顶点，500-2000 边）
- `k_full`：更大的完整图（约 500-2000 顶点，2000-5000 边）
- `visit_history`：deque（约 1000 步窗口）
- `_pending_negations`：list
- `_blocked_at`：dict
- `_step_neg_pairs`：set（每步重建）
- `_trajectory_cluster`：轨迹对象
- `settlement`：SettlementTracker
- `terrain`：dict（每步重建）
- `logs`：StepLog 列表

Python 对象开销（每个对象头部 16-56 bytes，dict/set 动态扩展）。
估算 500 个实例的总 RAM 需求（CPU 侧）。
GPU 侧：如果 CSR 只存基础图（S_net 产生的初始图），500 个实例共享 1 份 CSR，则 GPU 侧只增加每实例位置 + visit_count 向量（500 × 8 bytes = 4 KB）。

### Q6：β₁ 在多实例下

当前：`compute_beta_1` 缓存在 Graph 实例上（不可变语义保证正确性）。
多实例独立模式：每实例有自己的 k_active 演化，β₁ 各自计算，无冲突。
多实例共享模式：β₁ 是共享 k_active 的全局属性。
折叠操作后共享 k_active 变化 → 所有 500 个实例的 β₁ cache 同时失效。
每步最多触发 500 次 β₁ 重算（每实例步开始时各算一次）。
β₁ 算法复杂度：O(V+E)（connected_components BFS/DFS）。
500 × O(V+E) 每轮：如果 V=500, E=2000，则 500 × O(2500) = O(1.25M ops)。
评估：这是瓶颈吗？可否用增量 β₁ 更新？

---

## 评估要求

1. 对每个问题给出技术判断（可行/不可行/条件可行）
2. 给出具体的 FLOPS/bandwidth 数字（Q1）
3. 指出代码重构需要改动的具体位置（Q2）
4. 明确架构选型建议（Q3 独立 vs 共享模式）
5. 给出协调者设计的伪代码（Q4）
6. 给出内存数字（Q5）
7. 给出增量 β₁ 可行性（Q6）


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

下面是按“严格根因链”给出的诊断结论。

---

## 总体结论（先给结论）

你提出的目标架构 **“500 实例共享一个 CSR + 步结束批量写 + β₁ 多实例统一管理”**，与当前代码语义存在**核心定义冲突**，不是简单实现缺陷。

- 可行的部分：GPU 做批量邻域查询（Q1）在吞吐上**条件可行**，但收益受 kernel/同步开销限制。
- 不可直接成立的部分：  
  1) “run_step 是只读”与代码事实冲突（Q2）  
  2) “500 实例共享同一 k_active”与当前“每实例独立演化”冲突（Q3）  
  3) “fold/negate 串行协调且并行不受损”冲突（Q4）
- β₁：独立模式下可管理；共享模式下会出现全局失效风暴（Q6）。

---

## Q1：批量邻域查询 GPU 收益（独立 CSR 情况）

**技术判断：条件可行（性能上可做，但受调度/同步主导）**

### 数字估算（按你给的规模）
设每实例每步邻域查询 `q=20~40`，平均度 `d=10~20`。  
每次查询 CSR 读取近似：
- `indptr` 两次读：8B
- 邻接索引：`4*d` B
- 边类型/附加字段（若 int32）：`4*d` B  
合计约 `8 + 8d = 88~168B/query`

则 500 实例每步读流量：
\[
500 \times q \times (88\sim168)B \approx 0.88\sim3.36 MB/step
\]

RTX 4090 理论带宽 ~1 TB/s：
\[
t_{mem,ideal} \approx 0.9\sim3.4 \mu s
\]
即使按 20% 有效带宽（200 GB/s）：
\[
t_{mem} \approx 4.4\sim16.8 \mu s
\]

计算量（粗）：
\[
500\times(20\sim40)\times(10\sim20)\times O(5\sim10\ ops)
\]
数量级 < 数百万 ops/step，远低于 4090 算力上限（FP32 ~80TFLOPS 级）。  
**结论：这是典型 memory/launch-bound，不是 FLOPS-bound。**

### 关键变化（独立 CSR vs 共享 CSR）
- 共享 1 份 CSR：读取局部性更好。
- 500 独立 CSR：访问分散、warp coalescing 下降，但**总数据量仍小**（你给的 500KB~5MB，甚至可落 L2 72MB 内）。
- 真正代价在于：若每步图变，CSR 重建+提交，而不是查询本身。

### 诊断链
- **失败现象**：预期 GPU 巨大加速，但单步收益不稳定。  
- **直接原因**：每步工作量小、读流量小，kernel launch/sync 占比高。  
- **根本原因**：任务粒度过细，且动态图更新成本可能吞没读加速。  
- **分类**：实现/工程问题（非定义冲突）。

---

## Q2：穿越步是否只读？能否“步后批量写”？

**技术判断：按当前语义不可行（定义冲突）**

你给的代码链已经证明 run_step 内部是交错 R/W，且后读依赖前写（t₀→t₁→t₂→t₃）。

### 冲突方
- **A（目标架构）**：run_step = 纯读阶段 + 步末统一写入。  
- **B（当前代码定义）**：run_step 内写入立即生效，后续逻辑读取新图（`self.k_active` 连续换版本）。

### 直接代码证据
- `traversal.py:1451-1458` 先从 `k_active` 读 `neg_pairs`
- `execute_encounter` 后 `self.k_active = result.graph`（步内立即写）
- `traversal.py:1148` `_record_traversal_association` 直接 `add_edge` 到 `k_active`
- `traversal.py:1654` articulation feedback 直接写 REFERENCE 边
- 随后 `compute_terrain` / `check_settlement` 读取已更新图

### 不可弥合理由
若延迟写入到步末，则步内后续读看不到 t₁/t₂/t₃，行为就变了；  
若用“待提交缓冲”又要让后续读看到它，本质上你仍需一个“即时可见的叠加图”（不是纯读阶段）。

### 需改动位置（你要求的具体点）
若强制重构，至少要重写：
- `run_step` 主流程（`traversal.py:1441-1722`）为状态机
- `_record_traversal_association`（~1148）从直接写改为 delta 事件
- `execute_encounter` 分支返回 delta 而非新图直接替换
- articulation 写边点（~1654）改事件化
- `compute_terrain` / settlement 读取层要支持“base + pending deltas”

**分类判定：定义冲突（非实现 bug）**。按你的规则，不给“局部修补方案”。

---

## Q3：共享模式 vs 独立模式（语义 + 内存）

**技术判断：语义正确性上应选“独立模式”**

### 冲突方
- **A（共享模式）**：500 实例写入合并到同一个 `k_active`。  
- **B（现有语义）**：每实例有自己的 `self.k_active`，穿越产物仅影响该实例。

### 不可弥合理由
共享后，实例 i 的下一步会看到其他 499 实例写入（TA/REFERENCE/fold 结果），这从“500 独立逢亮”变成“1 个集体图上的 500 游走者”，存在论已变。

### 内存评估（GPU）
- 若独立 CSR（仅 `k_active` 拓扑）每份 1~10KB，500 份约 0.5~5MB，显存可承受。
- 真问题不是显存容量，而是**动态图更新路径**（重建/增量维护）。

### 结论
- **语义正确：独立模式**
- **共享模式仅在你明确改定义为“集体穿越”时才成立**

**分类判定：定义冲突。**

---

## Q4：fold/negate 串行协调设计

**技术判断：共享图下可做但并行度显著受损；严格一致性必须全局屏障**

### 冲突方
- **A**：fold/negate 串行协调，同时保持高并行。  
- **B**：fold 改变图结构，后续邻域查询必须看到一致版本。

### 不可弥合理由
- 继续并行 walk（不等待）⇒ 读到旧图，语义不一致。
- 全部暂停等待协调者 ⇒ 每步全局 barrier，吞吐下滑。

### 协调者伪代码（共享图、严格一致）
```python
while True:  # tick
    intents = gpu_kernel_propose(shared_graph, instance_states)
    # intents: WALK/FOLD/NEGATE/... + patch proposals

    # 全局屏障：必须等所有提案
    synchronize()

    # 串行一致性提交
    for intent in deterministic_order(intents):
        shared_graph = apply_intent(shared_graph, intent)  # fold/negate 串行

    # 广播新版本图
    instance_states = gpu_kernel_commit(shared_graph, intents, instance_states)
```

### 并行代价
每 tick：
\[
T_{tick}=T_{propose}+T_{barrier}+T_{serial\_apply(F)}+T_{commit}
\]
`F`（触发 fold/negate 数）一高，串行段变瓶颈。  
所以“串行协调 + 高并行”只能在 F 很低时条件成立。

**分类判定：定义冲突（在共享语义下无法同时满足“严格一致 + 无阻塞高并行”）。**

---

## Q5：500 实例 RAM 估算（CPU 侧）

**技术判断：可运行但内存会到 GB 级**

基于你给的规模和 CPython 对象开销，给区间估算：

| 组件 | 单实例估算 |
|---|---:|
| `k_active` (200-500V, 500-2000E) | 0.2 ~ 0.8 MB |
| `k_full` (500-2000V, 2000-5000E) | 0.8 ~ 2.5 MB |
| `visit_history` + pending/block/set | 0.03 ~ 0.15 MB |
| `trajectory` + settlement + terrain | 0.05 ~ 0.25 MB |
| `logs`（取决于保留步数） | 0.1 ~ 0.5 MB |

**单实例合计**：约 **1.2 ~ 4.2 MB**  
**500 实例合计**：约 **0.6 ~ 2.1 GB**

> 若 logs/历史不裁剪，峰值可继续上升。

GPU 侧（共享基础图 + 每实例位置向量）确实很小（KB 级）；  
但 CPU 侧对象图才是主内存负担。

---

## Q6：β₁ 多实例管理

**技术判断：**
- 独立模式：可行；
- 共享模式：会触发全局缓存失效风暴，开销可见。

### 现状
- `compute_beta_1` 缓存在 Graph 实例上（`engine.py:766`）
- Graph 一变对象就换，缓存自然失效

### 共享模式问题
一次共享图变更会让所有实例下一步都在新 Graph 上算 β₁（最多 500 次/轮）。

### 是否瓶颈
复杂度总量：
\[
500 \times O(V+E),\quad V=500,E=2000 \Rightarrow 500\times2500=1.25M\ \text{遍历单位/轮}
\]
在 Python 中这不是“免费”，可成为毫秒级~更高开销源之一。

### 增量 β₁ 可行性
**条件可行（混合策略）**：
- 对纯加边/加点：可用 `β₁ = E - V + C` 增量维护（配并查集维护连通分量）
- 对 fold/contract/删除类操作：动态连通性复杂，建议回退全量重算

即：**Add-only 增量，结构性变换全量重算**。

**分类判定：实现优化问题（非定义冲突）。**

---

## 最终选型建议（明确）

1. **若保持当前语义（500 独立逢亮）**：选 **独立模式**。  
2. GPU 仅做“批量邻域查询/候选生成”可尝试；不要假设“步纯读+统一写”天然成立。  
3. 共享 `k_active` 仅在你明确重定义为“集体穿越”时可采用；这不是现有语义的等价实现。

--- 

如果你要，我可以下一步给你一版“**独立模式下的最小可实施 GPU 切分方案**”（哪些函数上 GPU、哪些必须留 CPU、以及每步的数据边界）。
