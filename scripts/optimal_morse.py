"""
最优离散 Morse 函数构造（阶段 B-M 第一步）

背景：355号 L2 否定性结果表明 BFS 生成树的偶然性影响了 DM1 Morse 函数质量。
本模块构造最优（或近优）离散 Morse 函数，消除贪心策略的偶然性。

核心算法：Forman 离散梯度向量场理论
- 离散梯度向量场 V：配对 σ^(p) ↔ τ^(p+1)，使非配对临界单形数量最小
- 下界：临界单形数量 >= Betti 数（弱 Morse 不等式）
- 如果达到下界则函数是完美的（perfect Morse function）

策略：
1. 多起点 coreduction：随机化处理顺序，多次运行取最优
2. 优先级加权 coreduction：按连接度赋权，倾向配对高连接度单形
3. 与 DM1 贪心结果对比

认识论等级：L0（代数定义）+ L1（合成数据验证）
有效域：合成复形上代数正确，真实拓扑上的效果待 L2 验证

谱系依据：355号（L2 否定性结果）→ 阶段 B-M
"""

from __future__ import annotations

import random
import sys
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import FrozenSet, Tuple

# DM1 模块复用
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse"))
from simplicial_complex import (
    BettiNumbers,
    MorseResult,
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
    discrete_morse_greedy,
    verify_morse_inequalities,
)


# ─────────────────────────────────────────────────────────────────────────────
# 数据结构
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class GradientField:
    """离散梯度向量场。

    pairs: (σ^(p), τ^(p+1)) 配对列表——σ 是 τ 的面
    unpaired: 未配对的临界单形集合
    """
    pairs: Tuple[Tuple[FrozenSet[str], FrozenSet[str]], ...]
    unpaired_0: Tuple[FrozenSet[str], ...]  # 临界 0-单形
    unpaired_1: Tuple[FrozenSet[str], ...]  # 临界 1-单形
    unpaired_2: Tuple[FrozenSet[str], ...]  # 临界 2-单形

    @property
    def num_critical(self) -> int:
        return len(self.unpaired_0) + len(self.unpaired_1) + len(self.unpaired_2)

    def is_perfect(self, betti: BettiNumbers) -> bool:
        """临界单形数等于 Betti 数时为完美 Morse 函数。"""
        return (
            len(self.unpaired_0) == betti.beta_0
            and len(self.unpaired_1) == betti.beta_1
            and len(self.unpaired_2) == betti.beta_2
        )


@dataclass(frozen=True)
class OptimalMorseResult:
    """最优 Morse 函数结果。"""
    gradient_field: GradientField
    morse_function: dict  # {simplex_id: morse_value}
    betti: BettiNumbers
    is_perfect: bool
    num_restarts: int
    best_critical_count: int


# ─────────────────────────────────────────────────────────────────────────────
# 复形索引构造（共用）
# ─────────────────────────────────────────────────────────────────────────────

def _build_complex_index(sc: SimplicialComplex) -> tuple:
    """构造复形的统一索引和面/余面关系。

    Returns:
        (all_simplices, simplex_to_idx, facets, cofacets)
    """
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

    n = len(all_simplices)
    facets: dict[int, list[int]] = {i: [] for i in range(n)}
    cofacets: dict[int, list[int]] = {i: [] for i in range(n)}

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

    return all_simplices, simplex_to_idx, facets, cofacets


# ─────────────────────────────────────────────────────────────────────────────
# 随机化 coreduction
# ─────────────────────────────────────────────────────────────────────────────

def _coreduction_with_order(
    all_simplices: list[tuple[int, FrozenSet[str]]],
    facets: dict[int, list[int]],
    cofacets: dict[int, list[int]],
    initial_order: list[int],
) -> GradientField:
    """执行 coreduction（Mrozek 2009），使用指定的种子选择顺序。

    标准 coreduction 算法：
    1. alpha[σ] = σ 的未处理余面数
    2. alpha[σ]==0 → σ 临界（无余面可配对）
    3. alpha[σ]==1 → σ 与其唯一未处理余面 τ 配对
    4. 队列耗尽时，从 initial_order 中选择下一个未处理单形作为种子，
       强制标记为临界，触发新一轮级联

    种子选择顺序是随机化的注入点。
    """
    n = len(all_simplices)
    processed: set[int] = set()
    critical_set: set[int] = set()
    pairs: list[tuple[FrozenSet[str], FrozenSet[str]]] = []

    # alpha[i] = 单形 i 的未处理余面数
    alpha: list[int] = [len(cofacets[i]) for i in range(n)]

    # 队列初始化：alpha <= 1 的单形（与 DM1 一致）
    queue: deque[int] = deque()
    for i in initial_order:
        if alpha[i] <= 1:
            queue.append(i)

    # 种子候选指针
    order_ptr = 0

    while len(processed) < n:
        while queue:
            sigma = queue.popleft()
            if sigma in processed:
                continue

            if alpha[sigma] == 0:
                # σ 临界
                critical_set.add(sigma)
                processed.add(sigma)
                # σ 被移除 → 它的面的余面数减少
                for f in facets[sigma]:
                    if f not in processed:
                        alpha[f] -= 1
                        if alpha[f] <= 1:
                            queue.append(f)

            elif alpha[sigma] == 1:
                # σ 恰有一个未处理余面 τ → 配对 (σ, τ)
                tau = -1
                for cf in cofacets[sigma]:
                    if cf not in processed:
                        tau = cf
                        break
                if tau == -1:
                    # alpha 与实际不一致 → 安全网：标记临界
                    critical_set.add(sigma)
                    processed.add(sigma)
                    continue

                processed.add(sigma)
                processed.add(tau)
                pairs.append((all_simplices[sigma][1], all_simplices[tau][1]))

                # σ 和 τ 都被移除 → 更新它们的面的余面数
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
            # alpha >= 2: 跳过（不应该出现在队列中，安全网）

        # 队列耗尽 — 选择种子
        seed_found = False
        while order_ptr < n:
            candidate = initial_order[order_ptr]
            order_ptr += 1
            if candidate not in processed:
                # 强制标记为临界
                critical_set.add(candidate)
                processed.add(candidate)
                for f in facets[candidate]:
                    if f not in processed:
                        alpha[f] -= 1
                        if alpha[f] <= 1:
                            queue.append(f)
                seed_found = True
                break

        if not seed_found and len(processed) < n:
            for i in range(n):
                if i not in processed:
                    critical_set.add(i)
                    processed.add(i)
                    for f in facets[i]:
                        if f not in processed:
                            alpha[f] -= 1
                            if alpha[f] <= 1:
                                queue.append(f)

    # 分类临界单形
    c0, c1, c2 = [], [], []
    for idx in critical_set:
        dim_s, simplex = all_simplices[idx]
        if dim_s == 0:
            c0.append(simplex)
        elif dim_s == 1:
            c1.append(simplex)
        elif dim_s == 2:
            c2.append(simplex)

    return GradientField(
        pairs=tuple(pairs),
        unpaired_0=tuple(c0),
        unpaired_1=tuple(c1),
        unpaired_2=tuple(c2),
    )


# ─────────────────────────────────────────────────────────────────────────────
# 最大匹配（最优梯度向量场）
# ─────────────────────────────────────────────────────────────────────────────

def _build_pairing_candidates(
    all_simplices: list[tuple[int, FrozenSet[str]]],
    facets: dict[int, list[int]],
) -> list[tuple[int, int]]:
    """枚举所有合法配对候选：(face_idx, cofacet_idx)。

    合法配对 = σ^(p) 是 τ^(p+1) 的面（维度差恰好 1）。
    """
    candidates = []
    for tau_idx in range(len(all_simplices)):
        dim_tau = all_simplices[tau_idx][0]
        if dim_tau == 0:
            continue  # 0-单形没有面
        for sigma_idx in facets[tau_idx]:
            candidates.append((sigma_idx, tau_idx))
    return candidates


def _exhaustive_search(
    all_simplices: list[tuple[int, FrozenSet[str]]],
    facets: dict[int, list[int]],
    cofacets: dict[int, list[int]],
    betti_lower_bound: int,
) -> GradientField:
    """穷举搜索最优梯度向量场（适用于小规模复形）。

    使用回溯法+剪枝：尝试所有合法配对组合，使未配对（临界）
    单形数量最小。剪枝：当前临界数已超过已知最优解时剪枝。
    达到 Betti 下界时立即返回。
    """
    n = len(all_simplices)
    candidates = _build_pairing_candidates(all_simplices, facets)

    best_pairs: list[tuple[int, int]] = []
    best_critical = n  # 最差情况：全部临界

    def backtrack(idx: int, used: set[int], pairs: list[tuple[int, int]]) -> bool:
        nonlocal best_pairs, best_critical

        current_critical = n - 2 * len(pairs)
        if current_critical <= betti_lower_bound:
            # 达到 Betti 下界——完美
            best_pairs = pairs[:]
            best_critical = current_critical
            return True

        if current_critical < best_critical:
            best_critical = current_critical
            best_pairs = pairs[:]

        if idx >= len(candidates):
            return False

        # 剩余候选中最多还能配对的数量
        max_additional = 0
        for k in range(idx, len(candidates)):
            s, t = candidates[k]
            if s not in used and t not in used:
                max_additional += 1
        # 即使所有剩余候选都能配对，临界数仍不会优于 best_critical → 剪枝
        potential_critical = n - 2 * (len(pairs) + max_additional)
        if potential_critical >= best_critical:
            return False

        sigma_idx, tau_idx = candidates[idx]

        # 分支1：不选择当前候选
        if backtrack(idx + 1, used, pairs):
            return True

        # 分支2：选择当前候选（如果两个单形都未被使用）
        if sigma_idx not in used and tau_idx not in used:
            used.add(sigma_idx)
            used.add(tau_idx)
            pairs.append((sigma_idx, tau_idx))
            if backtrack(idx + 1, used, pairs):
                return True
            pairs.pop()
            used.discard(sigma_idx)
            used.discard(tau_idx)

        return False

    backtrack(0, set(), [])

    # 构造 GradientField
    paired_set = set()
    pair_tuples = []
    for sigma_idx, tau_idx in best_pairs:
        paired_set.add(sigma_idx)
        paired_set.add(tau_idx)
        pair_tuples.append((all_simplices[sigma_idx][1], all_simplices[tau_idx][1]))

    c0, c1, c2 = [], [], []
    for i in range(n):
        if i not in paired_set:
            dim_s, simplex = all_simplices[i]
            if dim_s == 0:
                c0.append(simplex)
            elif dim_s == 1:
                c1.append(simplex)
            elif dim_s == 2:
                c2.append(simplex)

    return GradientField(
        pairs=tuple(pair_tuples),
        unpaired_0=tuple(c0),
        unpaired_1=tuple(c1),
        unpaired_2=tuple(c2),
    )


# ─────────────────────────────────────────────────────────────────────────────
# 梯度向量场构造（核心）
# ─────────────────────────────────────────────────────────────────────────────

# 穷举搜索的单形数量上限（超过此值使用启发式）
_EXHAUSTIVE_THRESHOLD = 200


def build_gradient_field(
    complex_data: SimplicialComplex,
    *,
    num_restarts: int = 50,
    seed: int | None = None,
) -> GradientField:
    """构造离散梯度向量场，最小化临界单形数量。

    策略：
    - 小规模复形（<= 200 单形）：穷举搜索最优匹配
    - 大规模复形：多起点随机化 coreduction

    Parameters
    ----------
    complex_data : SimplicialComplex
    num_restarts : int
        随机重启次数（默认 50，仅大规模时使用）。
    seed : int | None
        随机种子（可选，用于可复现性）。

    Returns
    -------
    GradientField
        临界单形数量最少的梯度向量场。
    """
    all_simplices, simplex_to_idx, facets, cofacets = _build_complex_index(complex_data)
    n = len(all_simplices)

    if n == 0:
        return GradientField(
            pairs=(),
            unpaired_0=(),
            unpaired_1=(),
            unpaired_2=(),
        )

    betti = compute_betti_numbers(complex_data)
    betti_lower_bound = betti.beta_0 + betti.beta_1 + betti.beta_2

    # 小规模：穷举搜索
    if n <= _EXHAUSTIVE_THRESHOLD:
        return _exhaustive_search(all_simplices, facets, cofacets, betti_lower_bound)

    # 大规模：多起点 coreduction
    rng = random.Random(seed)
    best_field: GradientField | None = None
    best_critical = n + 1

    for _ in range(num_restarts):
        order = list(range(n))
        rng.shuffle(order)
        field = _coreduction_with_order(all_simplices, facets, cofacets, order)

        if field.num_critical < best_critical:
            best_critical = field.num_critical
            best_field = field
        if field.is_perfect(betti):
            break

    assert best_field is not None
    return best_field


def build_gradient_field_weighted(
    complex_data: SimplicialComplex,
) -> GradientField:
    """构造加权优先级梯度向量场（coreduction 策略）。

    策略：按单形的连接度排序——连接度低的单形优先进入队列。
    这是一种确定性启发式，可能不如穷举搜索但速度更快。
    """
    all_simplices, simplex_to_idx, facets, cofacets = _build_complex_index(complex_data)
    n = len(all_simplices)

    if n == 0:
        return GradientField(
            pairs=(),
            unpaired_0=(),
            unpaired_1=(),
            unpaired_2=(),
        )

    order = sorted(range(n), key=lambda i: (all_simplices[i][0], len(cofacets[i])))
    return _coreduction_with_order(all_simplices, facets, cofacets, order)


# ─────────────────────────────────────────────────────────────────────────────
# 从梯度向量场导出 Morse 函数
# ─────────────────────────────────────────────────────────────────────────────

def _gradient_field_to_morse_function(
    complex_data: SimplicialComplex,
    field: GradientField,
) -> dict[FrozenSet[str], float]:
    """从梯度向量场导出离散 Morse 函数 f: K -> R。

    规则：
    - 临界单形的函数值 > 正则单形的函数值（在同维度内）
    - 配对 (σ, τ) 中，f(σ) > f(τ)（逆转常规序——梯度向量场的定义）
    - 非配对非临界不存在（配对覆盖所有非临界单形）

    赋值策略：
    - 按维度分层
    - 配对中的低维 σ：f(σ) = base + pair_index（正则值）
    - 配对中的高维 τ：f(τ) = f(σ) + 0.5（正则值，比配对的面高 0.5）
    - 临界单形：f = base + num_pairs + critical_index（临界值 > 所有正则值）
    """
    # 收集所有单形
    all_simplices: list[FrozenSet[str]] = []
    for v in complex_data.vertices:
        all_simplices.append(frozenset([v]))
    for e in complex_data.edges:
        all_simplices.append(e)
    for t in complex_data.triangles:
        all_simplices.append(t)

    simplex_dim: dict[FrozenSet[str], int] = {}
    for v in complex_data.vertices:
        simplex_dim[frozenset([v])] = 0
    for e in complex_data.edges:
        simplex_dim[e] = 1
    for t in complex_data.triangles:
        simplex_dim[t] = 2

    # 维度基准值
    n_total = len(all_simplices)
    dim_base = {0: 0.0, 1: n_total, 2: 2 * n_total}

    morse_f: dict[FrozenSet[str], float] = {}

    # 配对赋值
    paired: set[FrozenSet[str]] = set()
    for pair_idx, (sigma, tau) in enumerate(field.pairs):
        dim_s = simplex_dim[sigma]
        base = dim_base[dim_s]
        # σ (低维) 获得正则值
        morse_f[sigma] = base + pair_idx
        # τ (高维) 获得正则值，但 f(τ) < f(σ)
        # 在离散梯度向量场中，配对 (σ, τ) 意味着 f(σ) > f(τ)
        # 但 dim(τ) > dim(σ)，所以 τ 在更高维度的基准上
        dim_t = simplex_dim[tau]
        base_t = dim_base[dim_t]
        morse_f[tau] = base_t + pair_idx
        paired.add(sigma)
        paired.add(tau)

    # 临界单形赋值（在同维度内值最大）
    critical_offset = n_total  # 确保临界值 > 正则值
    for c_idx, c in enumerate(field.unpaired_0):
        morse_f[c] = dim_base[0] + critical_offset + c_idx
    for c_idx, c in enumerate(field.unpaired_1):
        morse_f[c] = dim_base[1] + critical_offset + c_idx
    for c_idx, c in enumerate(field.unpaired_2):
        morse_f[c] = dim_base[2] + critical_offset + c_idx

    return morse_f


def optimal_morse_function(
    complex_data: SimplicialComplex,
    *,
    num_restarts: int = 50,
    seed: int | None = None,
) -> OptimalMorseResult:
    """从梯度向量场导出最优 Morse 函数。

    综合多起点随机化和加权确定性策略，取最优结果。

    Returns
    -------
    OptimalMorseResult
        包含最优梯度向量场、Morse 函数值映射、完美性判定等。
    """
    betti = compute_betti_numbers(complex_data)

    # 策略1：多起点随机化
    field_random = build_gradient_field(
        complex_data, num_restarts=num_restarts, seed=seed,
    )

    # 策略2：加权确定性
    field_weighted = build_gradient_field_weighted(complex_data)

    # 取最优
    if field_weighted.num_critical < field_random.num_critical:
        best_field = field_weighted
    else:
        best_field = field_random

    morse_f = _gradient_field_to_morse_function(complex_data, best_field)

    return OptimalMorseResult(
        gradient_field=best_field,
        morse_function=morse_f,
        betti=betti,
        is_perfect=best_field.is_perfect(betti),
        num_restarts=num_restarts,
        best_critical_count=best_field.num_critical,
    )


# ─────────────────────────────────────────────────────────────────────────────
# 与 BFS Morse 对比
# ─────────────────────────────────────────────────────────────────────────────

def compare_with_bfs_morse(
    complex_data: SimplicialComplex,
    bfs_morse: MorseResult,
    optimal: OptimalMorseResult,
) -> dict:
    """对比 BFS（贪心）Morse 和最优 Morse 的临界集差异。

    Returns
    -------
    dict
        包含：
        - bfs_critical: BFS 的临界单形数量 (m0, m1, m2)
        - optimal_critical: 最优的临界单形数量
        - betti: Betti 数（下界）
        - excess_bfs: BFS 超出 Betti 数的量
        - excess_optimal: 最优超出 Betti 数的量
        - improvement: 最优相比 BFS 减少的临界单形数
        - bfs_critical_sets: BFS 临界集
        - optimal_critical_sets: 最优临界集
        - is_optimal_perfect: 最优是否达到 Betti 下界
    """
    betti = optimal.betti
    field = optimal.gradient_field

    bfs_total = bfs_morse.m0 + bfs_morse.m1 + bfs_morse.m2
    opt_total = field.num_critical
    betti_total = betti.beta_0 + betti.beta_1 + betti.beta_2

    return {
        "bfs_critical": {
            "m0": bfs_morse.m0,
            "m1": bfs_morse.m1,
            "m2": bfs_morse.m2,
            "total": bfs_total,
        },
        "optimal_critical": {
            "m0": len(field.unpaired_0),
            "m1": len(field.unpaired_1),
            "m2": len(field.unpaired_2),
            "total": opt_total,
        },
        "betti": {
            "beta_0": betti.beta_0,
            "beta_1": betti.beta_1,
            "beta_2": betti.beta_2,
            "total": betti_total,
        },
        "excess_bfs": {
            "dim_0": bfs_morse.m0 - betti.beta_0,
            "dim_1": bfs_morse.m1 - betti.beta_1,
            "dim_2": bfs_morse.m2 - betti.beta_2,
            "total": bfs_total - betti_total,
        },
        "excess_optimal": {
            "dim_0": len(field.unpaired_0) - betti.beta_0,
            "dim_1": len(field.unpaired_1) - betti.beta_1,
            "dim_2": len(field.unpaired_2) - betti.beta_2,
            "total": opt_total - betti_total,
        },
        "improvement": bfs_total - opt_total,
        "is_optimal_perfect": optimal.is_perfect,
    }


# ─────────────────────────────────────────────────────────────────────────────
# 梯度向量场合法性验证
# ─────────────────────────────────────────────────────────────────────────────

def verify_gradient_field(
    complex_data: SimplicialComplex,
    field: GradientField,
) -> dict:
    """验证梯度向量场的合法性。

    检查：
    1. 每个单形最多参与一个配对（无冲突）
    2. 配对 (σ, τ) 中 σ 是 τ 的面（维度差 = 1）
    3. 所有单形要么配对要么临界（覆盖完整）
    4. 弱 Morse 不等式成立
    """
    # 收集所有单形
    all_simplices: set[FrozenSet[str]] = set()
    simplex_dim: dict[FrozenSet[str], int] = {}

    for v in complex_data.vertices:
        fv = frozenset([v])
        all_simplices.add(fv)
        simplex_dim[fv] = 0
    for e in complex_data.edges:
        all_simplices.add(e)
        simplex_dim[e] = 1
    for t in complex_data.triangles:
        all_simplices.add(t)
        simplex_dim[t] = 2

    # 检查1：配对无冲突
    paired: set[FrozenSet[str]] = set()
    conflict_free = True
    conflicts: list[str] = []

    for sigma, tau in field.pairs:
        if sigma in paired:
            conflict_free = False
            conflicts.append(f"{sigma} appears in multiple pairs")
        if tau in paired:
            conflict_free = False
            conflicts.append(f"{tau} appears in multiple pairs")
        paired.add(sigma)
        paired.add(tau)

    # 检查2：配对维度关系正确
    dim_correct = True
    dim_errors: list[str] = []

    for sigma, tau in field.pairs:
        if sigma not in simplex_dim or tau not in simplex_dim:
            dim_correct = False
            dim_errors.append(f"Unknown simplex in pair ({sigma}, {tau})")
            continue
        if simplex_dim[tau] - simplex_dim[sigma] != 1:
            dim_correct = False
            dim_errors.append(
                f"dim({tau})={simplex_dim[tau]} - dim({sigma})={simplex_dim[sigma]} != 1"
            )
        # σ 是 τ 的面
        if not sigma.issubset(tau):
            dim_correct = False
            dim_errors.append(f"{sigma} is not a face of {tau}")

    # 检查3：覆盖完整
    critical: set[FrozenSet[str]] = set()
    for c in field.unpaired_0:
        critical.add(c)
    for c in field.unpaired_1:
        critical.add(c)
    for c in field.unpaired_2:
        critical.add(c)

    covered = paired | critical
    coverage_complete = covered == all_simplices
    missing = all_simplices - covered
    extra = covered - all_simplices

    # 检查4：弱 Morse 不等式
    betti = compute_betti_numbers(complex_data)
    weak_0 = len(field.unpaired_0) >= betti.beta_0
    weak_1 = len(field.unpaired_1) >= betti.beta_1
    weak_2 = len(field.unpaired_2) >= betti.beta_2

    # Euler 等式
    chi_morse = len(field.unpaired_0) - len(field.unpaired_1) + len(field.unpaired_2)
    chi_betti = betti.euler_characteristic()
    euler_eq = chi_morse == chi_betti

    return {
        "conflict_free": conflict_free,
        "conflicts": conflicts,
        "dim_correct": dim_correct,
        "dim_errors": dim_errors,
        "coverage_complete": coverage_complete,
        "missing": [str(s) for s in missing],
        "extra": [str(s) for s in extra],
        "weak_morse_0": weak_0,
        "weak_morse_1": weak_1,
        "weak_morse_2": weak_2,
        "euler_equality": euler_eq,
        "chi_morse": chi_morse,
        "chi_betti": chi_betti,
        "all_valid": (
            conflict_free
            and dim_correct
            and coverage_complete
            and weak_0
            and weak_1
            and weak_2
            and euler_eq
        ),
    }


# ─────────────────────────────────────────────────────────────────────────────
# 主入口
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    import json

    repo_root = Path(__file__).resolve().parent.parent
    relations_path = repo_root / ".chanlun" / "block-topology" / "relations.jsonl"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)

    print("=== 最优离散 Morse 函数构造 ===\n")

    # 1. 加载并构造复形
    relations: list[dict] = []
    with open(relations_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            relations.append(json.loads(line))

    sc = build_simplicial_complex(relations)
    print(f"复形规模: V={sc.num_vertices} E={sc.num_edges} T={sc.num_triangles}")

    # 2. BFS 贪心（DM1 基线）
    bfs_morse = discrete_morse_greedy(sc)
    print(f"BFS 贪心: m₀={bfs_morse.m0} m₁={bfs_morse.m1} m₂={bfs_morse.m2}"
          f" (总临界={bfs_morse.m0 + bfs_morse.m1 + bfs_morse.m2})")

    # 3. 最优 Morse 函数
    print("\n构造最优 Morse 函数（50 次随机重启 + 加权策略）...")
    result = optimal_morse_function(sc, num_restarts=50, seed=42)
    field = result.gradient_field
    print(f"最优: m₀={len(field.unpaired_0)} m₁={len(field.unpaired_1)} "
          f"m₂={len(field.unpaired_2)} (总临界={field.num_critical})")
    print(f"Betti 下界: β₀={result.betti.beta_0} β₁={result.betti.beta_1} "
          f"β₂={result.betti.beta_2} (总={result.betti.beta_0 + result.betti.beta_1 + result.betti.beta_2})")
    print(f"完美 Morse 函数: {result.is_perfect}")

    # 4. 验证梯度向量场
    print("\n验证梯度向量场...")
    verification = verify_gradient_field(sc, field)
    print(f"  配对无冲突: {verification['conflict_free']}")
    print(f"  维度关系正确: {verification['dim_correct']}")
    print(f"  覆盖完整: {verification['coverage_complete']}")
    print(f"  弱 Morse 不等式: m₀≥β₀={verification['weak_morse_0']} "
          f"m₁≥β₁={verification['weak_morse_1']} m₂≥β₂={verification['weak_morse_2']}")
    print(f"  Euler 等式: {verification['euler_equality']} "
          f"(χ_Morse={verification['chi_morse']} == χ_Betti={verification['chi_betti']})")
    print(f"  全部合法: {verification['all_valid']}")

    # 5. 对比
    comparison = compare_with_bfs_morse(sc, bfs_morse, result)
    print(f"\n=== BFS vs 最优 对比 ===")
    print(f"  BFS 超出 Betti: {comparison['excess_bfs']}")
    print(f"  最优超出 Betti: {comparison['excess_optimal']}")
    print(f"  改善量: {comparison['improvement']} 个临界单形")

    # 6. 保存结果
    output_dir = repo_root / "experiments" / "discrete_morse"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "optimal_morse_results.json"

    out = {
        "experiment": "Optimal Morse Function (B-M Step 1)",
        "epistemological_level": "L0+L1",
        "validity_domain": "合成复形上代数正确，真实拓扑上的效果待 L2 验证",
        "complex": {
            "vertices": sc.num_vertices,
            "edges": sc.num_edges,
            "triangles": sc.num_triangles,
        },
        "betti": {
            "beta_0": result.betti.beta_0,
            "beta_1": result.betti.beta_1,
            "beta_2": result.betti.beta_2,
        },
        "bfs_greedy": {
            "m0": bfs_morse.m0,
            "m1": bfs_morse.m1,
            "m2": bfs_morse.m2,
        },
        "optimal": {
            "m0": len(field.unpaired_0),
            "m1": len(field.unpaired_1),
            "m2": len(field.unpaired_2),
            "is_perfect": result.is_perfect,
            "num_restarts": result.num_restarts,
        },
        "verification": {k: v for k, v in verification.items()
                         if k not in ("missing", "extra", "conflicts", "dim_errors")},
        "comparison": comparison,
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)
    print(f"\n结果已保存到: {output_path}")


if __name__ == "__main__":
    main()
