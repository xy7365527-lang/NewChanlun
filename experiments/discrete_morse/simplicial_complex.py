"""
DM1: 单纯复形构造 + 同调计算 + 离散 Morse 函数

从 .chanlun/block-topology/relations.jsonl 构造抽象单纯复形：
- 0-单形 = 区块节点（content-addressed hash）
- 1-单形 = 关系边（from→to，忽略方向，按无向边处理）
- 2-单形 = 三角形闭包（A-B, B-C, A-C 同时存在时构成 2-单形）

计算 Betti 数 β₀（连通分量数）和 β₁（独立环路数）。
构造离散 Morse 函数（贪心匹配），找到临界单纯形集合。
验证 Morse 不等式：m_k >= β_k。

认识论等级：L0→L1（代数构造 + 管线正确性验证）
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import FrozenSet, Tuple

import numpy as np


@dataclass(frozen=True)
class SimplicialComplex:
    """抽象单纯复形，存储各维度的单纯形集合。"""
    vertices: Tuple[str, ...]
    vertex_index: dict  # vertex -> int index
    edges: Tuple[FrozenSet[str], ...]
    triangles: Tuple[FrozenSet[str], ...]

    @property
    def num_vertices(self) -> int:
        return len(self.vertices)

    @property
    def num_edges(self) -> int:
        return len(self.edges)

    @property
    def num_triangles(self) -> int:
        return len(self.triangles)

    def dimension(self) -> int:
        if self.triangles:
            return 2
        if self.edges:
            return 1
        return 0


@dataclass(frozen=True)
class BettiNumbers:
    """Betti 数。"""
    beta_0: int  # 连通分量数
    beta_1: int  # 独立环路数
    beta_2: int  # 空腔数（对 2-复形通常为 0）

    def euler_characteristic(self) -> int:
        return self.beta_0 - self.beta_1 + self.beta_2


@dataclass(frozen=True)
class MorseResult:
    """离散 Morse 函数结果。"""
    # 配对集合：(低维单形, 高维单形) 的列表
    pairs: Tuple[Tuple[FrozenSet[str], FrozenSet[str]], ...]
    # 临界单纯形：未被配对的单纯形
    critical_0: Tuple[FrozenSet[str], ...]  # 临界 0-单形（节点）
    critical_1: Tuple[FrozenSet[str], ...]  # 临界 1-单形（边）
    critical_2: Tuple[FrozenSet[str], ...]  # 临界 2-单形（三角形）

    @property
    def m0(self) -> int:
        return len(self.critical_0)

    @property
    def m1(self) -> int:
        return len(self.critical_1)

    @property
    def m2(self) -> int:
        return len(self.critical_2)


def load_relations(path: Path) -> list[dict]:
    """从 JSONL 文件加载关系数据。"""
    relations = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            relations.append(json.loads(line))
    return relations


def build_simplicial_complex(relations: list[dict]) -> SimplicialComplex:
    """从关系数据构造单纯复形。

    0-单形 = 节点
    1-单形 = 无向边（关系的 from 和 to，忽略方向和重复）
    2-单形 = 三角形闭包（三个节点两两有边时形成）
    """
    # 收集所有节点和无向边
    node_set: set[str] = set()
    edge_set: set[FrozenSet[str]] = set()

    for rel in relations:
        src = rel["from"]
        tgt = rel["to"]
        node_set.add(src)
        node_set.add(tgt)
        if src != tgt:  # 排除自环
            edge_set.add(frozenset([src, tgt]))

    # 排序节点以获得确定性索引
    vertices = tuple(sorted(node_set))
    vertex_index = {v: i for i, v in enumerate(vertices)}

    # 构造邻接表用于三角形检测
    adjacency: dict[str, set[str]] = {v: set() for v in node_set}
    for edge in edge_set:
        a, b = tuple(edge)
        adjacency[a].add(b)
        adjacency[b].add(a)

    # 三角形闭包检测：对每条边 (a, b)，找到公共邻居 c
    triangle_set: set[FrozenSet[str]] = set()
    for edge in edge_set:
        a, b = tuple(edge)
        common = adjacency[a] & adjacency[b]
        for c in common:
            triangle_set.add(frozenset([a, b, c]))

    return SimplicialComplex(
        vertices=vertices,
        vertex_index=vertex_index,
        edges=tuple(sorted(edge_set, key=lambda e: tuple(sorted(e)))),
        triangles=tuple(sorted(triangle_set, key=lambda t: tuple(sorted(t)))),
    )


def boundary_matrix_1(sc: SimplicialComplex) -> np.ndarray:
    """构造 ∂₁ 矩阵（边→顶点的边界算子），在 Z₂ 上工作。

    ∂₁ 是 |V| x |E| 矩阵，列 j 对应边 j，非零行对应边 j 的两个端点。
    在 Z₂ 系数下，∂₁[i,j] = 1 当且仅当顶点 i 是边 j 的端点。
    """
    n_v = sc.num_vertices
    n_e = sc.num_edges
    if n_e == 0:
        return np.zeros((n_v, 0), dtype=np.int8)

    mat = np.zeros((n_v, n_e), dtype=np.int8)
    for j, edge in enumerate(sc.edges):
        a, b = tuple(edge)
        mat[sc.vertex_index[a], j] = 1
        mat[sc.vertex_index[b], j] = 1
    return mat


def boundary_matrix_2(sc: SimplicialComplex) -> np.ndarray:
    """构造 ∂₂ 矩阵（三角形→边的边界算子），在 Z₂ 上工作。

    ∂₂ 是 |E| x |T| 矩阵，列 j 对应三角形 j，
    非零行对应三角形 j 的三条边。
    """
    n_e = sc.num_edges
    n_t = sc.num_triangles
    if n_t == 0:
        return np.zeros((n_e, 0), dtype=np.int8)

    # 边到索引的映射
    edge_index = {e: i for i, e in enumerate(sc.edges)}

    mat = np.zeros((n_e, n_t), dtype=np.int8)
    for j, tri in enumerate(sc.triangles):
        verts = sorted(tri)
        # 三角形 {a,b,c} 的三条边
        edges_of_tri = [
            frozenset([verts[0], verts[1]]),
            frozenset([verts[0], verts[2]]),
            frozenset([verts[1], verts[2]]),
        ]
        for e in edges_of_tri:
            if e in edge_index:
                mat[edge_index[e], j] = 1
    return mat


def rank_z2(mat: np.ndarray) -> int:
    """计算 Z₂ 上矩阵的秩（通过高斯消元）。

    在 GF(2) 上做行简化，计算秩。
    使用列主元消元法，正确处理零列（跳过零列不递增行号）。
    """
    if mat.size == 0:
        return 0
    m = mat.copy() % 2
    rows, cols = m.shape
    pivot_row = 0
    for col in range(cols):
        if pivot_row >= rows:
            break
        # 在当前列中 pivot_row 及以下找非零元素
        found = -1
        for i in range(pivot_row, rows):
            if m[i, col] == 1:
                found = i
                break
        if found == -1:
            continue  # 零列，跳过（不递增 pivot_row）
        # 交换行
        if found != pivot_row:
            m[[pivot_row, found]] = m[[found, pivot_row]]
        # 消元
        for i in range(rows):
            if i != pivot_row and m[i, col] == 1:
                m[i] = (m[i] + m[pivot_row]) % 2
        pivot_row += 1
    return pivot_row


def compute_betti_numbers(sc: SimplicialComplex) -> BettiNumbers:
    """计算 Z₂ 系数下的 Betti 数。

    β_k = dim(ker(∂_k)) - dim(im(∂_{k+1}))
    即 β_k = nullity(∂_k) - rank(∂_{k+1})

    β₀ = |V| - rank(∂₁)
    β₁ = nullity(∂₁) - rank(∂₂) = (|E| - rank(∂₁)) - rank(∂₂)
    β₂ = nullity(∂₂) = |T| - rank(∂₂)  (因为没有 ∂₃)
    """
    d1 = boundary_matrix_1(sc)
    d2 = boundary_matrix_2(sc)

    rank_d1 = rank_z2(d1)
    rank_d2 = rank_z2(d2)

    beta_0 = sc.num_vertices - rank_d1
    beta_1 = (sc.num_edges - rank_d1) - rank_d2
    beta_2 = sc.num_triangles - rank_d2

    return BettiNumbers(beta_0=beta_0, beta_1=beta_1, beta_2=beta_2)


def discrete_morse_greedy(sc: SimplicialComplex) -> MorseResult:
    """贪心算法构造离散 Morse 函数。

    策略：从低维到高维，尝试将每个未配对的单纯形与其唯一未配对的余面配对。
    这是 Forman 离散 Morse 理论中的标准贪心方法。

    具体算法（改进贪心）：
    1. 维护每个单纯形的"未配对余面计数"
    2. 优先配对余面计数为 1 的单纯形（forced pairing）
    3. 无 forced pairing 时，当前最低维未配对单纯形标记为临界
    """
    # 将所有单纯形统一编号
    # dim 0: 顶点
    # dim 1: 边
    # dim 2: 三角形
    all_simplices: list[tuple[int, FrozenSet[str]]] = []
    simplex_to_idx: dict[FrozenSet[str], int] = {}

    for v in sc.vertices:
        fv = frozenset([v])
        idx = len(all_simplices)
        all_simplices.append((0, fv))
        simplex_to_idx[fv] = idx

    for e in sc.edges:
        idx = len(all_simplices)
        all_simplices.append((1, e))
        simplex_to_idx[e] = idx

    for t in sc.triangles:
        idx = len(all_simplices)
        all_simplices.append((2, t))
        simplex_to_idx[t] = idx

    # 构造面关系：cofacet[sigma] = 包含 sigma 的高一维单纯形列表
    # facet[tau] = tau 的面列表
    facets: dict[int, list[int]] = {i: [] for i in range(len(all_simplices))}
    cofacets: dict[int, list[int]] = {i: [] for i in range(len(all_simplices))}

    # 边的面 = 两个顶点
    for e_idx, (dim, simplex) in enumerate(all_simplices):
        if dim == 1:
            for v in simplex:
                fv = frozenset([v])
                v_idx = simplex_to_idx[fv]
                facets[e_idx].append(v_idx)
                cofacets[v_idx].append(e_idx)
        elif dim == 2:
            verts = sorted(simplex)
            face_edges = [
                frozenset([verts[0], verts[1]]),
                frozenset([verts[0], verts[2]]),
                frozenset([verts[1], verts[2]]),
            ]
            for fe in face_edges:
                if fe in simplex_to_idx:
                    fe_idx = simplex_to_idx[fe]
                    facets[e_idx].append(fe_idx)
                    cofacets[fe_idx].append(e_idx)

    # 标准 Coreduction 算法（Mrozek 2009）
    #
    # 维护 alpha[σ] = σ 的未配对余面数。
    # 队列 Q 存放 alpha[σ] <= 1 且未处理的 σ。
    #
    # - alpha[σ] == 0 → σ 是临界的
    # - alpha[σ] == 1 → σ 与其唯一未配对余面 τ 配对
    #
    # 配对 (σ, τ) 后更新 τ 的其他面的 alpha 值。
    # 临界 σ 后不需要更新（σ 没有余面了）。
    from collections import deque

    processed: set[int] = set()
    critical_set: set[int] = set()
    pairs: list[tuple[FrozenSet[str], FrozenSet[str]]] = []

    # alpha[i] = 单纯形 i 的未配对余面数
    alpha: list[int] = [len(cofacets[i]) for i in range(len(all_simplices))]

    # 队列：alpha <= 1 的单纯形
    queue: deque[int] = deque()
    for i in range(len(all_simplices)):
        if alpha[i] <= 1:
            queue.append(i)

    while queue:
        sigma = queue.popleft()
        if sigma in processed:
            continue

        if alpha[sigma] == 0:
            # σ 是临界的——没有未配对余面
            critical_set.add(sigma)
            processed.add(sigma)
            # 不需要更新（σ 没有未配对余面）
            # 但需要更新 σ 的面的 alpha（因为从 σ 的面视角，σ 作为余面消失了）
            # 不对——alpha 追踪的是余面数，σ 被标记临界但不从复形中移除
            # 在 coreduction 中，临界单纯形被移除，所以需要更新面的 alpha
            for f in facets[sigma]:
                if f not in processed:
                    alpha[f] -= 1
                    if alpha[f] <= 1:
                        queue.append(f)

        elif alpha[sigma] == 1:
            # σ 恰好有一个未配对余面 τ → 配对 (σ, τ)
            tau = -1
            for cf in cofacets[sigma]:
                if cf not in processed:
                    tau = cf
                    break
            if tau == -1:
                # 安全网：所有余面已被处理（alpha 不一致）
                critical_set.add(sigma)
                processed.add(sigma)
                continue

            # 配对 (sigma, tau)
            processed.add(sigma)
            processed.add(tau)
            pairs.append((all_simplices[sigma][1], all_simplices[tau][1]))

            # 更新 tau 的面（除 sigma）和 sigma 的面的 alpha 值
            for f in facets[tau]:
                if f not in processed:
                    alpha[f] -= 1
                    if alpha[f] <= 1:
                        queue.append(f)
            for f in facets[sigma]:
                if f not in processed:
                    alpha[f] -= 1
                    if alpha[f] <= 1:
                        queue.append(f)

    # 处理可能未进入队列的残留单纯形（安全网）
    for i in range(len(all_simplices)):
        if i not in processed:
            critical_set.add(i)
            processed.add(i)

    # 分类临界单纯形
    critical_0 = []
    critical_1 = []
    critical_2 = []

    for idx in critical_set:
        dim_s, simplex = all_simplices[idx]
        if dim_s == 0:
            critical_0.append(simplex)
        elif dim_s == 1:
            critical_1.append(simplex)
        elif dim_s == 2:
            critical_2.append(simplex)

    return MorseResult(
        pairs=tuple(pairs),
        critical_0=tuple(critical_0),
        critical_1=tuple(critical_1),
        critical_2=tuple(critical_2),
    )


def verify_morse_inequalities(betti: BettiNumbers, morse: MorseResult) -> dict:
    """验证 Morse 不等式：m_k >= β_k。

    弱 Morse 不等式：
      m_0 >= β_0
      m_1 >= β_1
      m_2 >= β_2

    强 Morse 不等式（交替和）：
      m_0 >= β_0
      m_0 - m_1 <= β_0 - β_1
      m_0 - m_1 + m_2 >= β_0 - β_1 + β_2 = χ (Euler特征数)

    Morse 等式（最终等式）：
      m_0 - m_1 + m_2 = β_0 - β_1 + β_2 = χ
    """
    chi_betti = betti.euler_characteristic()
    chi_morse = morse.m0 - morse.m1 + morse.m2

    weak_0 = morse.m0 >= betti.beta_0
    weak_1 = morse.m1 >= betti.beta_1
    weak_2 = morse.m2 >= betti.beta_2

    strong_1 = (morse.m0 - morse.m1) <= (betti.beta_0 - betti.beta_1)
    euler_eq = chi_morse == chi_betti

    return {
        "weak_morse_0": {"holds": weak_0, "m0": morse.m0, "beta_0": betti.beta_0},
        "weak_morse_1": {"holds": weak_1, "m1": morse.m1, "beta_1": betti.beta_1},
        "weak_morse_2": {"holds": weak_2, "m2": morse.m2, "beta_2": betti.beta_2},
        "strong_morse_1": {
            "holds": strong_1,
            "m0_minus_m1": morse.m0 - morse.m1,
            "beta_0_minus_beta_1": betti.beta_0 - betti.beta_1,
        },
        "euler_equality": {
            "holds": euler_eq,
            "chi_morse": chi_morse,
            "chi_betti": chi_betti,
        },
        "all_pass": weak_0 and weak_1 and weak_2 and euler_eq,
    }


def run_dm1(relations_path: Path) -> dict:
    """执行完整的 DM1 实验管线。"""
    # 1. 加载数据
    relations = load_relations(relations_path)

    # 2. 构造单纯复形
    sc = build_simplicial_complex(relations)

    # 3. 计算 Betti 数
    betti = compute_betti_numbers(sc)

    # 4. 构造离散 Morse 函数
    morse = discrete_morse_greedy(sc)

    # 5. 验证 Morse 不等式
    verification = verify_morse_inequalities(betti, morse)

    return {
        "complex": {
            "vertices": sc.num_vertices,
            "edges": sc.num_edges,
            "triangles": sc.num_triangles,
            "dimension": sc.dimension(),
        },
        "betti": {
            "beta_0": betti.beta_0,
            "beta_1": betti.beta_1,
            "beta_2": betti.beta_2,
            "euler_characteristic": betti.euler_characteristic(),
        },
        "morse": {
            "num_pairs": len(morse.pairs),
            "critical_0": morse.m0,
            "critical_1": morse.m1,
            "critical_2": morse.m2,
        },
        "verification": verification,
    }


if __name__ == "__main__":
    import sys

    repo_root = Path(__file__).resolve().parent.parent.parent
    relations_path = repo_root / ".chanlun" / "block-topology" / "relations.jsonl"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)

    print(f"Loading relations from {relations_path}...")
    result = run_dm1(relations_path)

    print("\n=== DM1 单纯复形构造实验结果 ===\n")
    print(f"单纯复形规模:")
    print(f"  0-单形 (顶点): {result['complex']['vertices']}")
    print(f"  1-单形 (边):   {result['complex']['edges']}")
    print(f"  2-单形 (三角形): {result['complex']['triangles']}")
    print(f"  维度: {result['complex']['dimension']}")
    print()
    print(f"Betti 数 (Z₂ 系数):")
    print(f"  β₀ (连通分量): {result['betti']['beta_0']}")
    print(f"  β₁ (独立环路): {result['betti']['beta_1']}")
    print(f"  β₂ (空腔):     {result['betti']['beta_2']}")
    print(f"  Euler 特征数 χ: {result['betti']['euler_characteristic']}")
    print()
    print(f"离散 Morse 函数:")
    print(f"  配对数: {result['morse']['num_pairs']}")
    print(f"  临界 0-单形 m₀: {result['morse']['critical_0']}")
    print(f"  临界 1-单形 m₁: {result['morse']['critical_1']}")
    print(f"  临界 2-单形 m₂: {result['morse']['critical_2']}")
    n_total = (result['complex']['vertices']
               + result['complex']['edges']
               + result['complex']['triangles'])
    n_critical = (result['morse']['critical_0']
                  + result['morse']['critical_1']
                  + result['morse']['critical_2'])
    print(f"  简化率: {result['morse']['num_pairs'] * 2}/{n_total}"
          f" = {result['morse']['num_pairs'] * 2 / n_total * 100:.1f}%"
          f"  (配对消除的单纯形占比)")
    print()
    print(f"Morse 不等式验证:")
    v = result["verification"]
    print(f"  弱不等式 m₀ >= β₀: {v['weak_morse_0']['holds']}"
          f"  ({v['weak_morse_0']['m0']} >= {v['weak_morse_0']['beta_0']})")
    print(f"  弱不等式 m₁ >= β₁: {v['weak_morse_1']['holds']}"
          f"  ({v['weak_morse_1']['m1']} >= {v['weak_morse_1']['beta_1']})")
    print(f"  弱不等式 m₂ >= β₂: {v['weak_morse_2']['holds']}"
          f"  ({v['weak_morse_2']['m2']} >= {v['weak_morse_2']['beta_2']})")
    print(f"  强不等式:           {v['strong_morse_1']['holds']}"
          f"  (m₀-m₁={v['strong_morse_1']['m0_minus_m1']}"
          f" <= β₀-β₁={v['strong_morse_1']['beta_0_minus_beta_1']})")
    print(f"  Euler 等式:         {v['euler_equality']['holds']}"
          f"  (χ_Morse={v['euler_equality']['chi_morse']}"
          f" == χ_Betti={v['euler_equality']['chi_betti']})")
    print(f"  全部通过: {v['all_pass']}")

    # 保存结果到 JSON
    output_path = Path(__file__).resolve().parent / "dm1_results.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\n结果已保存到: {output_path}")
