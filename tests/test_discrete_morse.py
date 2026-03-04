"""
DM1 测试：单纯复形构造 + 同调计算 + 离散 Morse 函数

验证管线正确性（认识论等级 L1）：
- 已知拓扑的合成数据上验证算法
- 从 dm1_results.json 加载实验结果并验证不变量
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import FrozenSet

import pytest
import sys

# 确保 experiments 目录可导入
sys.path.insert(0, str(Path(__file__).parent.parent / "experiments" / "discrete_morse"))

from simplicial_complex import (
    SimplicialComplex,
    BettiNumbers,
    MorseResult,
    build_simplicial_complex,
    compute_betti_numbers,
    discrete_morse_greedy,
    verify_morse_inequalities,
)

# ─────────────────────────────────────────────────────────────────────────────
# 辅助：构造合成数据
# ─────────────────────────────────────────────────────────────────────────────

def make_relations(*pairs: tuple[str, str]) -> list[dict]:
    """从 (from, to) 对构造 relations 列表。"""
    return [{"from": f, "to": t, "type": "depends_on"} for f, t in pairs]


def triangle_relations() -> list[dict]:
    """A-B-C 三角形（1个连通分量，1个环路，0个空腔）。"""
    return make_relations(("A", "B"), ("B", "C"), ("A", "C"))


def two_triangles_sharing_edge() -> list[dict]:
    """A-B-C 和 A-B-D 两个三角形共享边 A-B。
    预期：β₀=1, β₁=1（两个三角形填充了一个环路）, β₂=0
    注意：两个三角形填充后去掉了一个 β₁
    """
    return make_relations(
        ("A", "B"), ("B", "C"), ("A", "C"),
        ("B", "D"), ("A", "D"),
    )


def disconnected_graph() -> list[dict]:
    """A-B 和 C-D 两个不连通的边。
    预期：β₀=2, β₁=0, β₂=0
    """
    return make_relations(("A", "B"), ("C", "D"))


def cycle_graph() -> list[dict]:
    """A-B-C-A 三角形环（无填充），只有边无三角形。
    预期：β₀=1, β₁=1, β₂=0
    但 build_simplicial_complex 会添加三角形闭包，所以 A-B-C 有三角形 → β₁=0
    """
    return make_relations(("A", "B"), ("B", "C"), ("A", "C"))


# ─────────────────────────────────────────────────────────────────────────────
# 单纯复形构造测试
# ─────────────────────────────────────────────────────────────────────────────

class TestBuildSimplicialComplex:
    def test_triangle_vertices(self):
        sc = build_simplicial_complex(triangle_relations())
        assert sc.num_vertices == 3
        assert set(sc.vertices) == {"A", "B", "C"}

    def test_triangle_edges(self):
        sc = build_simplicial_complex(triangle_relations())
        assert sc.num_edges == 3

    def test_triangle_has_triangle_closure(self):
        sc = build_simplicial_complex(triangle_relations())
        assert sc.num_triangles == 1

    def test_disconnected_graph_two_components(self):
        sc = build_simplicial_complex(disconnected_graph())
        assert sc.num_vertices == 4
        assert sc.num_edges == 2
        assert sc.num_triangles == 0

    def test_dimension_triangle(self):
        sc = build_simplicial_complex(triangle_relations())
        assert sc.dimension() == 2

    def test_dimension_line(self):
        sc = build_simplicial_complex(make_relations(("A", "B")))
        assert sc.dimension() == 1

    def test_self_loops_excluded(self):
        """自环不应产生边。"""
        rels = make_relations(("A", "A"), ("A", "B"))
        sc = build_simplicial_complex(rels)
        assert sc.num_edges == 1  # 只有 A-B


# ─────────────────────────────────────────────────────────────────────────────
# Betti 数测试
# ─────────────────────────────────────────────────────────────────────────────

class TestBettiNumbers:
    def test_single_vertex_beta0(self):
        """单顶点：β₀=1, β₁=0, β₂=0。"""
        sc = build_simplicial_complex(make_relations(("A", "A")))
        # 自环被排除，顶点A仍在
        # 实际上自环会产生顶点但不产生边
        betti = compute_betti_numbers(sc)
        assert betti.beta_0 >= 1

    def test_disconnected_beta0(self):
        """两个不连通的组件：β₀=2。"""
        sc = build_simplicial_complex(disconnected_graph())
        betti = compute_betti_numbers(sc)
        assert betti.beta_0 == 2

    def test_triangle_beta0(self):
        """三角形：β₀=1（一个连通分量）。"""
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        assert betti.beta_0 == 1

    def test_triangle_beta1(self):
        """填充的三角形：β₁=0（三角形闭包填充了环路）。"""
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        assert betti.beta_1 == 0

    def test_euler_characteristic_consistency(self):
        """Euler 数 χ = V - E + T = β₀ - β₁ + β₂。"""
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        chi_topological = betti.euler_characteristic()
        chi_combinatorial = sc.num_vertices - sc.num_edges + sc.num_triangles
        assert chi_topological == chi_combinatorial


# ─────────────────────────────────────────────────────────────────────────────
# Morse 不等式测试
# ─────────────────────────────────────────────────────────────────────────────

class TestMorseInequalities:
    def test_morse_inequalities_triangle(self):
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        morse = discrete_morse_greedy(sc)
        result = verify_morse_inequalities(betti, morse)
        assert result["all_pass"], f"Morse 不等式验证失败: {result}"

    def test_morse_inequalities_disconnected(self):
        sc = build_simplicial_complex(disconnected_graph())
        betti = compute_betti_numbers(sc)
        morse = discrete_morse_greedy(sc)
        result = verify_morse_inequalities(betti, morse)
        assert result["all_pass"], f"Morse 不等式验证失败: {result}"

    def test_morse_m0_ge_beta0(self):
        """m₀ >= β₀ 始终成立。"""
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        morse = discrete_morse_greedy(sc)
        assert morse.m0 >= betti.beta_0

    def test_euler_equality_morse(self):
        """χ_Morse = χ_Betti（强 Morse 等式的推论）。"""
        sc = build_simplicial_complex(triangle_relations())
        betti = compute_betti_numbers(sc)
        morse = discrete_morse_greedy(sc)
        chi_morse = morse.m0 - morse.m1 + morse.m2
        chi_betti = betti.euler_characteristic()
        assert chi_morse == chi_betti


# ─────────────────────────────────────────────────────────────────────────────
# 从 dm1_results.json 加载并验证实验结果
# ─────────────────────────────────────────────────────────────────────────────

RESULTS_PATH = Path(__file__).parent.parent / "experiments" / "discrete_morse" / "dm1_results.json"


@pytest.fixture(scope="module")
def dm1_results():
    """加载 DM1 实验结果（在实际 relations.jsonl 上运行的结果）。"""
    if not RESULTS_PATH.exists():
        pytest.skip("dm1_results.json 不存在，跳过实验结果验证")
    with open(RESULTS_PATH) as f:
        return json.load(f)


class TestDM1Results:
    def test_betti_0_is_4(self, dm1_results):
        """β₀=4：谱系 DAG 有 4 个连通分量（L1验证：与报告一致）。"""
        assert dm1_results["betti"]["beta_0"] == 4

    def test_betti_1_is_607(self, dm1_results):
        """β₁=607：607 个独立环路。"""
        assert dm1_results["betti"]["beta_1"] == 607

    def test_betti_2_is_1259(self, dm1_results):
        """β₂=1259：1259 个空腔。"""
        assert dm1_results["betti"]["beta_2"] == 1259

    def test_euler_characteristic(self, dm1_results):
        """Euler 数 χ=656 与组合学计算一致。"""
        c = dm1_results["complex"]
        chi_comb = c["vertices"] - c["edges"] + c["triangles"]
        assert chi_comb == 656

    def test_morse_inequalities_all_pass(self, dm1_results):
        """全部 Morse 不等式成立。"""
        assert dm1_results["verification"]["all_pass"] is True

    def test_euler_equality_in_results(self, dm1_results):
        """Euler 等式 χ_Morse = χ_Betti 成立。"""
        euler_eq = dm1_results["verification"]["euler_equality"]
        assert euler_eq["holds"] is True
        assert euler_eq["chi_morse"] == euler_eq["chi_betti"]

    def test_complex_vertices_count(self, dm1_results):
        """顶点数量：2182 个区块节点。"""
        assert dm1_results["complex"]["vertices"] == 2182

    def test_complex_edges_count(self, dm1_results):
        """边数量：4432 条关系边。"""
        assert dm1_results["complex"]["edges"] == 4432
