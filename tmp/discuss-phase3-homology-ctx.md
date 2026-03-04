# Phase 2 → Phase 3 同调全局表达力讨论

## 模式：discuss（不是 challenge）

这不是质询现有方案的正确性，而是讨论如何在有向图范畴（273号裁定）内保留全局拓扑表达力。

---

## 系统背景

### 数学基底裁定（273号谱系）

Phase 2 在**有向图范畴**内工作。曲面拓扑语言（genus、puncture、surgery）仅作方向性隐喻——有向图上没有 genus 的内禀定义，嵌入不唯一，六种拓扑操作的曲面映射悬浮在未奠基的类比上。Phase 3 可通过 directed flag complex 构造中间层。

### 区块拓扑的具体结构

- 约 630 个节点（block + concept_id）
- 约 3000-4000 条关系边
- 关系类型（RELATION_TYPES）：
  - depends_on, negates, related, tensions_with（dag.yaml 迁移）
  - supersedes, residue_of, reopens（共识/剩余关系）
  - freezes, splits, severs（拓扑操作）
  - records（rewrite→event 记录关系）
  - negated_by（dag.yaml negated_by）
  - defines, modifies, references（内容级）
  - refines, revises（Phase 2 新增，modifies 拆分）
  - annotates（Phase 2 预留）
- 否定边（negates）携带 validity 字段（active | invalidated）
- Phase 2 计划：active_β₀, active_cycle_rank, full_β₀, full_cycle_rank（O(V+E)）

### 折叠命运表（有向图语义）

| 折叠命运 | 有向图操作 | 关系类型 |
|---------|-----------|---------|
| 诞生 | 节点在新子图获得入边 | （代理指标） |
| 稳定态 | 节点在≥2子图均有入边 | （Phase 3） |
| 保持 | 入边属性变化，子图结构不变 | refines |
| 变形 | 入边在子图间重新分配 | revises |
| 撕裂 | 从子图删除入边 | negates |
| 坍缩 | 节点入边被另一节点吸收 | supersedes |

---

## 编排者提出的六个议题

### 议题1：有向 flag complex vs clique complex vs path homology

编排者倾向 **directed flag complex**（有向图→单纯复形，n-单纯形 = n+1 节点的全序链 v₀→v₁→...→vₙ）。

**问题**：
1. 在区块拓扑的具体结构上（~630 节点，~3000-4000 关系边，多种关系类型），三种方式各自的优缺点
2. 方向信息（A depends_on B ≠ B depends_on A）在同调计算中的实际影响——丢失方向会导致什么语义丢失？
3. 多重边问题：A depends_on B 和 A negates B 是两条不同的有向边（同起终点），在复形构造中如何处理？

### 议题2：Betti 数在概念拓扑中的语义

编排者语义映射：
- β₀ = 连通分量数 = 概念空间分裂为几个互不依赖的岛屿
- β₁ = 独立回路数 = 不可消去的循环依赖（"概念中枢"——类比缠论中枢）
- β₂ = 独立空腔数 = 三个概念互相依赖但无统一上位概念（"概念空洞"）

**问题**：
1. β₁ 的"概念中枢"类比是否站得住？缠论中枢是区间交集（连续的），概念拓扑的循环是有向图结构（离散的），类比的有效域是什么？
2. β₂ 在实际系统中是否有足够的实例来产生有意义的信号？630 节点的图中 β₂ > 0 的概率有多大？
3. 折叠命运表中每种命运对 Betti 数变化的签名（见下方修正版）——请验证或否定

### 议题3：活跃图 vs 全量图的双重不变量

编排者关键区分：
- 活跃图（validity="active" 的边）的不变量描述当前有效概念结构
- 全量图（所有边，包括 invalidated）的不变量描述历史积累总量
- 差值 = 否定/替代操作"杀死"的结构量

**问题**：
1. 双重不变量是否引入不必要的复杂性？一个就够还是两个都需要？
2. 全量图的 β₀ 只降不升、cycle rank 只升不降——这个单调性在数学上是否严格成立？
3. delta = full_invariant - active_invariant 的信号是否有实际价值？

### 议题4：增量计算 vs 全量重算

编排者倾向 Phase 3 先用全量重算，性能瓶颈出现后再切增量。

**问题**：
1. 对 630 节点 + 4000 边的 directed flag complex，2 维截断的 Betti 数计算复杂度估算
2. 是否存在比全量重算和增量更新之间的中间方案？

### 议题5：cycle rank（图层面）vs β₁（复形层面）

编排者关键洞察：
- cycle rank = E - V + C（图的第一 Betti 数，1-维复形）
- flag complex β₁ ≤ cycle rank（2-单纯形可以"填充"环使之可缩）
- 差值 = 被传递依赖"解释"的环数量
- Phase 2 用 cycle rank 粗估（O(V+E)），Phase 3 用 flag complex β₁ 精确值

**问题**：
1. cycle rank - flag_β₁ 在实际区块拓扑中的典型比值是多少？如果差值很小，Phase 3 的精确化是否值得？
2. 有向图的 cycle rank 定义：应该用弱连通分量的 cycle rank（忽略方向）还是强连通分量的有向 cycle rank？编排者倾向后者（有向 cycle rank）。

### 议题6：折叠命运签名表（编排者修正版）

基于有向图、活跃边子图：

| 折叠命运 | 添加的边 | invalidate 的边 | active_β₀ 变化 | active_cycle_rank 变化 |
|---------|----------|----------------|----------------|----------------------|
| negates | +negates 边 | 被否定的边 | 可能 +1（断开） | 净效应不确定（+1 然后 -1） |
| supersedes | +supersedes 边 | 被替代区块的所有出边 | 可能 +N | 通常大幅 - |
| revises | +revises 边 | 被修订的关系 | 不确定 | 不确定 |
| refines | +refines 边 | 无 | 0 或 -1 | +1 或 0 |

编排者注意到：refines "几乎总是 cycle_rank +1"，说明 cycle rank 区分力不足——所有新边都倾向增加 cycle rank。这正是需要 flag complex β₁ 的原因：2-单纯形可区分"新的不可还原环"和"冗余边"。

**问题**：请验证或修正这个签名表，特别是 negates 和 supersedes 的净效应分析。

---

## 编排者的 Phase 2 最终判断

Phase 2 计算有向图上的四个数字：
- active_β₀（活跃图弱连通分量数）
- active_cycle_rank（活跃图强连通分量有向 cycle rank）
- full_β₀（全量图弱连通分量数）
- full_cycle_rank（全量图强连通分量有向 cycle rank）

O(V+E) 复杂度，可直接进 Phase 2。invariant_status 改为 "graph_invariants_computed"。

Phase 3 构造 directed flag complex 做精确同调，计算 Δβ₁_correction = graph_cycle_rank - flag_complex_β₁。

请对这个分层策略给出你的评估。

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
    # Phase 2 新增：
    "refines", "revises", "annotates",
})

VALIDITY_VALUES = frozenset({"active", "invalidated"})

# make_relation 中 negates 边携带 validity 字段（273号扩展）
# validity 默认 "active"，可从 active 单向转为 invalidated
```

### 否定边的有效性（273号谱系）

- 否定之否定：C→¬A 且 A→¬B（active）→ A→¬B 变为 invalidated，B 被动复活
- 活跃子图 = 只保留 validity="active" 的 negates 边 + 所有其他类型边
- 全量图 = 所有边（不过滤 validity）

---

## 讨论焦点

请深入讨论以下内容（数学推导优先，不需要代码）：

1. **三种同调构造的选择**：在这个具体的系统中（多关系类型、多重边、方向信息重要），directed flag complex 的优势是否足以克服其构造复杂性？

2. **Betti 数语义的有效域**：β₁ 的"概念中枢"类比在什么条件下成立，在什么条件下失效？

3. **分层策略的合理性**：Phase 2（图不变量）→ Phase 3（复形同调）的分层是否遗漏了重要信息，还是足够覆盖核心语义？

4. **签名表的精确化**：negates 和 supersedes 在活跃子图上的 Betti 数效应是否可以给出更精确的数学分析？

请使用中文回复，数学公式使用 LaTeX 表示。
