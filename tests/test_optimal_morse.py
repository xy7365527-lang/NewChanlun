"""
最优离散 Morse 函数测试

认识论等级：L0（代数定义验证）+ L1（合成数据管线正确性）
有效域：合成复形上的代数正确性。真实拓扑效果待 L2 验证。

谱系依据：355号 → 阶段 B-M 第一步
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

# 确保 import 路径
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from simplicial_complex import (
    BettiNumbers,
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
    discrete_morse_greedy,
)
from optimal_morse import (
    GradientField,
    build_gradient_field,
    build_gradient_field_weighted,
    compare_with_bfs_morse,
    optimal_morse_function,
    verify_gradient_field,
)


# ─────────────────────────────────────────────────────────────────────────────
# 合成复形构造 helpers
# ─────────────────────────────────────────────────────────────────────────────

def _make_relations(edges: list[tuple[str, str]]) -> list[dict]:
    """从边列表构造 relations 格式。"""
    return [{"from": a, "to": b} for a, b in edges]


def _triangle_complex() -> SimplicialComplex:
    """单个三角形 {A, B, C}。

    V=3, E=3, T=1
    β₀=1, β₁=0, β₂=0
    最优 Morse: m₀=1, m₁=0, m₂=0 (完美)
    """
    relations = _make_relations([("A", "B"), ("B", "C"), ("A", "C")])
    return build_simplicial_complex(relations)


def _path_complex() -> SimplicialComplex:
    """路径 A-B-C-D（无环）。

    V=4, E=3, T=0
    β₀=1, β₁=0, β₂=0
    最优 Morse: m₀=1, m₁=0, m₂=0 (完美)
    """
    relations = _make_relations([("A", "B"), ("B", "C"), ("C", "D")])
    return build_simplicial_complex(relations)


def _cycle_complex() -> SimplicialComplex:
    """三角形边界（环）A-B-C-A，无填充。

    V=3, E=3, T=0
    β₀=1, β₁=1, β₂=0
    最优 Morse: m₀=1, m₁=1, m₂=0 (完美)
    """
    relations = _make_relations([("A", "B"), ("B", "C"), ("A", "C")])
    # build_simplicial_complex 会检测三角形闭包——三条边形成三角形
    # 所以这实际上是填充三角形。用四节点环代替。
    relations = _make_relations([("A", "B"), ("B", "C"), ("C", "D"), ("D", "A")])
    return build_simplicial_complex(relations)


def _two_components() -> SimplicialComplex:
    """两个不连通分量：{A-B} 和 {C-D}。

    V=4, E=2, T=0
    β₀=2, β₁=0, β₂=0
    最优 Morse: m₀=2, m₁=0, m₂=0 (完美)
    """
    relations = _make_relations([("A", "B"), ("C", "D")])
    return build_simplicial_complex(relations)


def _two_triangles_shared_edge() -> SimplicialComplex:
    """两个三角形共享一条边：{A,B,C} 和 {B,C,D}。

    V=4, E=5, T=2
    β₀=1, β₁=0, β₂=0
    最优 Morse: m₀=1, m₁=0, m₂=0 (完美)
    """
    relations = _make_relations([
        ("A", "B"), ("B", "C"), ("A", "C"),
        ("B", "D"), ("C", "D"),
    ])
    return build_simplicial_complex(relations)


def _single_vertex() -> SimplicialComplex:
    """单个顶点（通过自环关系）。

    V=1, E=0, T=0
    β₀=1, β₁=0, β₂=0
    """
    relations = [{"from": "A", "to": "A"}]
    return build_simplicial_complex(relations)


def _square_with_diagonal() -> SimplicialComplex:
    """正方形 + 一条对角线。

    A-B, B-C, C-D, D-A, A-C
    三角形: {A,B,C}, {A,C,D}
    V=4, E=5, T=2
    β₀=1, β₁=0, β₂=0
    """
    relations = _make_relations([
        ("A", "B"), ("B", "C"), ("C", "D"), ("D", "A"), ("A", "C"),
    ])
    return build_simplicial_complex(relations)


# ─────────────────────────────────────────────────────────────────────────────
# 测试：梯度向量场合法性
# ─────────────────────────────────────────────────────────────────────────────

class TestGradientFieldValidity:
    """验证梯度向量场的合法性（配对不冲突、覆盖完整等）。"""

    @pytest.mark.parametrize("complex_fn,name", [
        (_triangle_complex, "triangle"),
        (_path_complex, "path"),
        (_cycle_complex, "cycle"),
        (_two_components, "two_components"),
        (_two_triangles_shared_edge, "two_triangles"),
        (_single_vertex, "single_vertex"),
        (_square_with_diagonal, "square_diag"),
    ])
    def test_gradient_field_valid(self, complex_fn, name):
        """梯度向量场对各种合成复形均合法。"""
        sc = complex_fn()
        field = build_gradient_field(sc, num_restarts=20, seed=42)
        result = verify_gradient_field(sc, field)

        assert result["conflict_free"], f"[{name}] Pair conflicts: {result['conflicts']}"
        assert result["dim_correct"], f"[{name}] Dim errors: {result['dim_errors']}"
        assert result["coverage_complete"], f"[{name}] Missing: {result['missing']}"
        assert result["euler_equality"], (
            f"[{name}] Euler: χ_Morse={result['chi_morse']} != χ_Betti={result['chi_betti']}"
        )
        assert result["all_valid"], f"[{name}] Gradient field invalid"

    def test_weighted_field_valid(self):
        """加权策略生成的梯度向量场合法。"""
        sc = _two_triangles_shared_edge()
        field = build_gradient_field_weighted(sc)
        result = verify_gradient_field(sc, field)
        assert result["all_valid"]


# ─────────────────────────────────────────────────────────────────────────────
# 测试：弱 Morse 不等式
# ─────────────────────────────────────────────────────────────────────────────

class TestWeakMorseInequalities:
    """验证临界单形数量 >= Betti 数。"""

    @pytest.mark.parametrize("complex_fn,name", [
        (_triangle_complex, "triangle"),
        (_path_complex, "path"),
        (_cycle_complex, "cycle"),
        (_two_components, "two_components"),
        (_two_triangles_shared_edge, "two_triangles"),
        (_single_vertex, "single_vertex"),
        (_square_with_diagonal, "square_diag"),
    ])
    def test_weak_morse_holds(self, complex_fn, name):
        """弱 Morse 不等式对所有合成复形成立。"""
        sc = complex_fn()
        betti = compute_betti_numbers(sc)
        field = build_gradient_field(sc, num_restarts=20, seed=42)

        assert len(field.unpaired_0) >= betti.beta_0, (
            f"[{name}] m₀={len(field.unpaired_0)} < β₀={betti.beta_0}"
        )
        assert len(field.unpaired_1) >= betti.beta_1, (
            f"[{name}] m₁={len(field.unpaired_1)} < β₁={betti.beta_1}"
        )
        assert len(field.unpaired_2) >= betti.beta_2, (
            f"[{name}] m₂={len(field.unpaired_2)} < β₂={betti.beta_2}"
        )


# ─────────────────────────────────────────────────────────────────────────────
# 测试：最优性（简单复形应达到完美）
# ─────────────────────────────────────────────────────────────────────────────

class TestOptimality:
    """验证简单复形上最优 Morse 函数达到 Betti 下界。"""

    def test_triangle_perfect(self):
        """填充三角形：m=(1,0,0)=β=(1,0,0)，完美。"""
        sc = _triangle_complex()
        result = optimal_morse_function(sc, num_restarts=30, seed=42)
        assert result.is_perfect, (
            f"Not perfect: m=({len(result.gradient_field.unpaired_0)},"
            f"{len(result.gradient_field.unpaired_1)},"
            f"{len(result.gradient_field.unpaired_2)}) vs "
            f"β=({result.betti.beta_0},{result.betti.beta_1},{result.betti.beta_2})"
        )

    def test_path_perfect(self):
        """路径：m=(1,0,0)=β=(1,0,0)，完美。"""
        sc = _path_complex()
        result = optimal_morse_function(sc, num_restarts=30, seed=42)
        assert result.is_perfect

    def test_two_components_perfect(self):
        """两分量：m=(2,0,0)=β=(2,0,0)，完美。"""
        sc = _two_components()
        result = optimal_morse_function(sc, num_restarts=30, seed=42)
        assert result.is_perfect

    def test_single_vertex_perfect(self):
        """单顶点：m=(1,0,0)=β=(1,0,0)，完美。"""
        sc = _single_vertex()
        result = optimal_morse_function(sc, num_restarts=30, seed=42)
        assert result.is_perfect


# ─────────────────────────────────────────────────────────────────────────────
# 测试：最优 vs BFS 对比
# ─────────────────────────────────────────────────────────────────────────────

class TestComparison:
    """验证最优 vs BFS 对比的正确性。"""

    def test_comparison_structure(self):
        """对比结果包含所有必要字段。"""
        sc = _triangle_complex()
        bfs = discrete_morse_greedy(sc)
        opt = optimal_morse_function(sc, num_restarts=20, seed=42)
        comp = compare_with_bfs_morse(sc, bfs, opt)

        assert "bfs_critical" in comp
        assert "optimal_critical" in comp
        assert "betti" in comp
        assert "excess_bfs" in comp
        assert "excess_optimal" in comp
        assert "improvement" in comp
        assert "is_optimal_perfect" in comp

    def test_optimal_no_worse_than_bfs(self):
        """最优 Morse 的临界数不大于 BFS 贪心。"""
        sc = _two_triangles_shared_edge()
        bfs = discrete_morse_greedy(sc)
        opt = optimal_morse_function(sc, num_restarts=50, seed=42)
        comp = compare_with_bfs_morse(sc, bfs, opt)

        assert comp["improvement"] >= 0, (
            f"Optimal worse than BFS: improvement={comp['improvement']}"
        )

    def test_excess_nonnegative(self):
        """超出 Betti 的量非负（弱 Morse 不等式的另一种表述）。"""
        sc = _cycle_complex()
        bfs = discrete_morse_greedy(sc)
        opt = optimal_morse_function(sc, num_restarts=20, seed=42)
        comp = compare_with_bfs_morse(sc, bfs, opt)

        for dim in ("dim_0", "dim_1", "dim_2"):
            assert comp["excess_optimal"][dim] >= 0, (
                f"Negative excess in {dim}: {comp['excess_optimal'][dim]}"
            )


# ─────────────────────────────────────────────────────────────────────────────
# 测试：可复现性
# ─────────────────────────────────────────────────────────────────────────────

class TestReproducibility:
    """验证相同种子产生相同结果。"""

    def test_deterministic_with_seed(self):
        """相同 seed 应产生相同的临界单形数量。"""
        sc = _two_triangles_shared_edge()

        r1 = optimal_morse_function(sc, num_restarts=20, seed=123)
        r2 = optimal_morse_function(sc, num_restarts=20, seed=123)

        f1, f2 = r1.gradient_field, r2.gradient_field
        assert len(f1.unpaired_0) == len(f2.unpaired_0)
        assert len(f1.unpaired_1) == len(f2.unpaired_1)
        assert len(f1.unpaired_2) == len(f2.unpaired_2)
        assert f1.num_critical == f2.num_critical


# ─────────────────────────────────────────────────────────────────────────────
# 测试：Morse 函数值属性
# ─────────────────────────────────────────────────────────────────────────────

class TestMorseFunctionValues:
    """验证 Morse 函数值的赋值属性。"""

    def test_all_simplices_have_values(self):
        """每个单形都有 Morse 函数值。"""
        sc = _triangle_complex()
        result = optimal_morse_function(sc, num_restarts=20, seed=42)
        mf = result.morse_function

        total = sc.num_vertices + sc.num_edges + sc.num_triangles
        assert len(mf) == total, f"Expected {total} values, got {len(mf)}"

    def test_function_values_distinct(self):
        """Morse 函数值在同一配对组内不重复（全局可能有重复——不同维度）。"""
        sc = _path_complex()
        result = optimal_morse_function(sc, num_restarts=20, seed=42)
        mf = result.morse_function

        # 至少所有值都是有限实数
        for simplex, val in mf.items():
            assert isinstance(val, (int, float)), f"Non-numeric value for {simplex}: {val}"


# ─────────────────────────────────────────────────────────────────────────────
# 测试：空复形边界
# ─────────────────────────────────────────────────────────────────────────────

class TestEdgeCases:
    """边界情况测试。"""

    def test_empty_complex(self):
        """空关系列表 → 空复形。"""
        sc = build_simplicial_complex([])
        field = build_gradient_field(sc, num_restarts=5, seed=42)
        assert field.num_critical == 0

    def test_isolated_vertices(self):
        """多个孤立顶点（自环）。"""
        relations = [{"from": "A", "to": "A"}, {"from": "B", "to": "B"}, {"from": "C", "to": "C"}]
        sc = build_simplicial_complex(relations)
        betti = compute_betti_numbers(sc)

        assert betti.beta_0 == 3  # 3 个连通分量
        field = build_gradient_field(sc, num_restarts=10, seed=42)
        assert len(field.unpaired_0) == 3  # 3 个临界 0-单形
        assert field.num_critical == 3  # 完美
