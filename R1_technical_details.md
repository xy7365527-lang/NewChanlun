# 联合研究 R1：技术细节与计算框架附件

## Q1：商空间操作的数学形式化

**精确定义**：
设全量图为有向 1-单纯复形 $K = (V, E, \tau)$，其中 $\tau: E \to \{dep, neg, mod, sup, ref\}$ 是边类型映射。
1. **删除子复形 (Deletion)**：设 $N \subset K$ 为所有被 `negates` 边指向的节点及其关联边构成的子复形。定义存活复形 $K_{surv} = K \setminus N$。
2. **等价关系 (Equivalence)**：在 $K_{surv}$ 上定义等价关系 $\sim$：$u \sim v$ 当且仅当存在无向的 `supersedes` 或 `modifies` 路径连接 $u, v$。
3. **商映射 (Quotient Map)**：$\phi: K_{surv} \to K_{surv}/\sim$。
活跃图定义为 $K_{active} = K_{surv}/\sim$。

**同调关系与长正合列**：
计算 $\beta_1(K) - \beta_1(K_{active})$ 分两步：
1. **从 $K$ 到 $K_{surv}$**：使用相对同调的长正合列 (Hatcher, 2002)：
   $\dots \to H_1(N) \to H_1(K) \to H_1(K, N) \to \tilde{H}_0(N) \to \dots$
   这里损失的 $\beta_1$ 对应于被 $N$ 覆盖的环。
2. **从 $K_{surv}$ 到 $K_{active}$**：由于 $\sim$ 是将连通子图（版本链）收缩为一点，如果这些版本链本身是树（无环），则商映射是同伦等价的局部化，$\beta_1$ 的变化精确等于版本链中存在的环的数量。

## Q2：有向有类型复形上的 Morse 理论

**理论局限与扩展**：
Forman (1998) 的离散 Morse 理论基于无向单纯复形。对于有向无环图 (DAG)，可以将其视为偏序集 (Poset)。
- **Minian (2012)** 和 **Chari (2000)** 证明了 Poset 上的离散 Morse 理论。
- **类型标签的嵌入**：不能简单地将类型映射为标量。必须构造一个**分层 Morse 函数 (Stratified Morse Function)** $f: K \to \mathbb{R}^k$，利用字典序。例如，赋予 `negates` 最高的拓扑破坏权重。
- **结论**：直接套用 Forman 理论会失败，必须使用 Poset Morse 理论，并将有向边视为偏序关系 $u \prec v$。

## Q3：参数化族和 Cerf 分岔

**Vineyard 的失效与 Zigzag 持续同调**：
Vineyard (Cohen-Steiner et al., 2006) 假设复形是单调递增的 ($K_1 \hookrightarrow K_2 \hookrightarrow \dots$)。
但在我们的系统中，`negates` 和 `supersedes` 会导致节点在 $K_{active}$ 中消失或合并，这构成了**非单调序列**。
**正确工具**：Zigzag 持续同调 (Carlsson & de Silva, 2010)。
序列形式为：$K_{active}(t_1) \hookrightarrow K_{union} \hookleftarrow K_{active}(t_2) \dots$
**Cerf 分岔点**：对应于 Zigzag 持续图中条带 (barcode) 的生灭点。主动预测需要对 LLM 的输出概率分布求导，寻找梯度爆炸点。

## Q4：连续-离散界面

**LLM-拓扑 Morse 函数**：
设 LLM 的语义空间为黎曼流形 $\mathcal{M} \subset \mathbb{R}^d$。区块 $v_i$ 是 $\mathcal{M}$ 中的点。
定义连续函数 $f: \mathcal{M} \times \mathcal{M} \to \mathbb{R}$ 为余弦距离。
离散拓扑通过 Vietoris-Rips 复形 $VR_\epsilon(V)$ 生成。
**临界点结构**：当 $\epsilon$ 越过某两个区块的距离 $d(v_i, v_j)$ 时，$VR_\epsilon$ 的拓扑发生跳变。这完全符合 Morse 引理：拓扑变化仅在临界值处发生。

## Q5：Mapper 和商空间的关系

**活跃图即 Mapper Nerve**：
活跃图 $K_{active}$ 是 Mapper 算法的严格特例：
1. **空间**：$X = K_{surv}$（去除了被否定节点的图）。
2. **滤函数 (Filter)**：$f: X \to \mathcal{C}$，其中 $\mathcal{C}$ 是版本链的连通分量标识符。
3. **覆盖 (Cover)**：$\mathcal{U} = \{ f^{-1}(c) \mid c \in \mathcal{C} \}$。由于每个版本链是一个等价类，覆盖是互斥的（在 0-单纯形上）。
4. **Nerve**：Mapper 的 Nerve 恰好将每个等价类收缩为一个节点，类之间的依赖边构成了 Nerve 的 1-单纯形。
**结论**：$K_{active} \cong Nerve(\mathcal{U})$。Nerve 定理 (Borsuk, 1948) 保证了如果每个版本链是可收缩的（树状），则 $K_{active}$ 与 $K_{surv}$ 同伦等价。

## Q6：最有希望的第一个计算实验 (Python 框架)

**推荐路径**：路径 1（商复形同调计算）+ 路径 2（Mapper 应用）。这是最严谨且几周内可实现的。

```python
import json
import networkx as nx
import gudhi

def build_complexes(relations_file):
    K_full = nx.DiGraph()
    K_surv = nx.DiGraph()
    
    # 1. Load relations and build K_full
    with open(relations_file, 'r') as f:
        for line in f:
            rel = json.loads(line)
            K_full.add_edge(rel['source'], rel['target'], type=rel['type'])
            
    # 2. Identify negated nodes
    negated_nodes = {v for u, v, d in K_full.edges(data=True) if d['type'] == 'negates'}
    
    # 3. Build K_surv (remove negated)
    K_surv = K_full.subgraph(set(K_full.nodes()) - negated_nodes).copy()
    
    # 4. Build K_active (Quotient by supersedes/modifies)
    equiv_edges = [(u, v) for u, v, d in K_surv.edges(data=True) if d['type'] in ['supersedes', 'modifies']]
    equiv_graph = nx.Graph(equiv_edges)
    equiv_classes = list(nx.connected_components(equiv_graph))
    
    # Create mapping from node to its equivalence class representative
    mapping = {}
    for i, comp in enumerate(equiv_classes):
        rep = list(comp)[0]
        for node in comp:
            mapping[node] = rep
            
    # Add unmapped nodes as their own class
    for node in K_surv.nodes():
        if node not in mapping:
            mapping[node] = node
            
    K_active = nx.quotient_graph(K_surv, partition=[set(comp) for comp in equiv_classes] + [{n} for n in K_surv.nodes() if n not in mapping])
    
    return K_full, K_active

def compute_betti(graph):
    # Convert to undirected for standard simplicial homology (Betti 1)
    st = gudhi.SimplexTree()
    for u, v in graph.edges():
        st.insert([u, v])
    st.compute_persistence()
    return st.betti_numbers()

# Execution
# K_full, K_active = build_complexes('relations.jsonl')
# b_full = compute_betti(K_full)
# b_active = compute_betti(K_active)
# print(f"Betti-1 Full: {b_full[1] if len(b_full)>1 else 0}, Active: {b_active[1] if len(b_active)>1 else 0}")
```
