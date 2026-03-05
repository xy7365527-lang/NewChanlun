"""
加权离散 Morse 函数测试

认识论等级：L0（代数定义验证）+ L1（合成数据管线正确性）
有效域：合成复形上的代数正确性，不验证加权是否在真实拓扑上优于均匀
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

# 添加模块路径
scripts_dir = str(Path(__file__).resolve().parent.parent / "scripts")
dm1_dir = str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse")
if scripts_dir not in sys.path:
    sys.path.insert(0, scripts_dir)
if dm1_dir not in sys.path:
    sys.path.insert(0, dm1_dir)

from simplicial_complex import (
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
)
from optimal_morse import (
    GradientField,
    _build_complex_index,
    build_gradient_field,
    verify_gradient_field,
)
from weighted_morse import (
    DEFAULT_WEIGHTS,
    WeightedComplex,
    build_weighted_complex_from_relations,
    compare_weighted_vs_uniform,
    weighted_morse_function,
)


# ─────────────────────────────────────────────────────────────────────────────
# 合成数据工厂
# ─────────────────────────────────────────────────────────────────────────────

def _make_relations(pairs: list[tuple[str, str, str]]) -> list[dict]:
    """构造合成关系列表。pairs = [(from, to, relation_type), ...]"""
    return [{"from": f, "to": t, "relation": r} for f, t, r in pairs]


def _triangle_relations() -> list[dict]:
    """三角形复形 A-B-C，各边不同权重。"""
    return _make_relations([
        ("A", "B", "negates"),      # weight=4
        ("B", "C", "depends_on"),   # weight=1
        ("A", "C", "references"),   # weight=1
    ])


def _line_relations() -> list[dict]:
    """线性复形 A-B-C（无三角形）。"""
    return _make_relations([
        ("A", "B", "negates"),      # weight=4
        ("B", "C", "depends_on"),   # weight=1
    ])


def _star_relations() -> list[dict]:
    """星形复形：中心 O 连接 A,B,C,D，各边不同权重。"""
    return _make_relations([
        ("O", "A", "negates"),      # weight=4
        ("O", "B", "supersedes"),   # weight=3
        ("O", "C", "modifies"),     # weight=2
        ("O", "D", "depends_on"),   # weight=1
    ])


def _mixed_weight_diamond() -> list[dict]:
    """菱形复形：A-B, A-C, B-D, C-D，形成两条路径。"""
    return _make_relations([
        ("A", "B", "negates"),      # weight=4
        ("A", "C", "depends_on"),   # weight=1
        ("B", "D", "depends_on"),   # weight=1
        ("C", "D", "negates"),      # weight=4
    ])


def _multi_relation_edge() -> list[dict]:
    """同一对节点多种关系类型——取最大权重。"""
    return _make_relations([
        ("A", "B", "depends_on"),   # weight=1
        ("A", "B", "negates"),      # weight=4 (same edge, higher weight wins)
        ("B", "C", "references"),   # weight=1
    ])


# ─────────────────────────────────────────────────────────────────────────────
# 测试：加权复形构造
# ─────────────────────────────────────────────────────────────────────────────

class TestBuildWeightedComplex:
    """测试 build_weighted_complex_from_relations 的正确性。"""

    def test_basic_construction(self):
        """基本构造：顶点、边、权重全部正确。"""
        rels = _triangle_relations()
        wc = build_weighted_complex_from_relations(rels)
        sc = wc.simplicial_complex

        assert sc.num_vertices == 3
        assert sc.num_edges == 3
        assert sc.num_triangles == 1  # A-B-C 形成三角形

    def test_edge_weights_assigned(self):
        """边权重按关系类型正确赋值。"""
        rels = _triangle_relations()
        wc = build_weighted_complex_from_relations(rels)

        ab = frozenset(["A", "B"])
        bc = frozenset(["B", "C"])
        ac = frozenset(["A", "C"])

        assert wc.edge_weights[ab] == 4  # negates
        assert wc.edge_weights[bc] == 1  # depends_on
        assert wc.edge_weights[ac] == 1  # references

    def test_multi_relation_takes_max_weight(self):
        """同一对节点多种关系，取最大权重。"""
        rels = _multi_relation_edge()
        wc = build_weighted_complex_from_relations(rels)

        ab = frozenset(["A", "B"])
        assert wc.edge_weights[ab] == 4  # negates > depends_on

    def test_self_loop_excluded(self):
        """自环不产生边。"""
        rels = _make_relations([
            ("A", "A", "depends_on"),
            ("A", "B", "references"),
        ])
        wc = build_weighted_complex_from_relations(rels)
        assert wc.simplicial_complex.num_edges == 1  # only A-B

    def test_custom_weights(self):
        """自定义权重表。"""
        custom = {"negates": 10, "depends_on": 5}
        rels = _line_relations()
        wc = build_weighted_complex_from_relations(rels, weights=custom)

        ab = frozenset(["A", "B"])
        bc = frozenset(["B", "C"])
        assert wc.edge_weights[ab] == 10
        assert wc.edge_weights[bc] == 5

    def test_unknown_relation_defaults_to_one(self):
        """未知关系类型默认权重 1。"""
        rels = _make_relations([("A", "B", "some_unknown_type")])
        wc = build_weighted_complex_from_relations(rels)
        ab = frozenset(["A", "B"])
        assert wc.edge_weights[ab] == 1

    def test_relation_type_counts(self):
        """统计关系类型出现次数。"""
        rels = _multi_relation_edge()
        wc = build_weighted_complex_from_relations(rels)
        assert wc.relation_type_counts["depends_on"] == 1
        assert wc.relation_type_counts["negates"] == 1
        assert wc.relation_type_counts["references"] == 1


# ─────────────────────────────────────────────────────────────────────────────
# 测试：加权 Morse 函数
# ─────────────────────────────────────────────────────────────────────────────

class TestWeightedMorseFunction:
    """测试 weighted_morse_function 的代数正确性。"""

    def test_gradient_field_valid(self):
        """加权 Morse 产生合法的梯度向量场。"""
        rels = _triangle_relations()
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)

        v = verify_gradient_field(wc.simplicial_complex, result.gradient_field)
        assert v["all_valid"], f"Verification failed: {v}"

    def test_weak_morse_inequalities(self):
        """弱 Morse 不等式：m_k >= beta_k。"""
        rels = _star_relations()
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)

        field = result.gradient_field
        betti = result.betti
        assert len(field.unpaired_0) >= betti.beta_0
        assert len(field.unpaired_1) >= betti.beta_1
        assert len(field.unpaired_2) >= betti.beta_2

    def test_euler_equality(self):
        """Euler 等式：m0 - m1 + m2 = beta0 - beta1 + beta2。"""
        rels = _mixed_weight_diamond()
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)

        field = result.gradient_field
        betti = result.betti
        chi_morse = len(field.unpaired_0) - len(field.unpaired_1) + len(field.unpaired_2)
        chi_betti = betti.beta_0 - betti.beta_1 + betti.beta_2
        assert chi_morse == chi_betti

    def test_empty_complex(self):
        """空复形处理。"""
        rels = _make_relations([("A", "A", "depends_on")])  # only self-loop -> no edges
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)
        assert result.best_critical_count >= 0

    def test_high_weight_edges_tend_to_be_critical(self):
        """高权重边更倾向于成为临界（统计性断言）。

        在星形复形中，negates(w=4) 的边比 depends_on(w=1) 的边
        更可能残留为临界。这不是必然的（取决于复形结构），
        但在星形中中心节点的高权重边应被保留。
        """
        rels = _star_relations()
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)

        field = result.gradient_field
        critical_edges = set(field.unpaired_1)

        if not critical_edges:
            # 如果完美配对则无临界边——此断言退化
            return

        # 临界边的平均权重应 >= 所有边的平均权重
        all_weights = list(wc.edge_weights.values())
        critical_ws = [wc.edge_weights[e] for e in critical_edges]
        avg_all = sum(all_weights) / len(all_weights)
        avg_critical = sum(critical_ws) / len(critical_ws)
        assert avg_critical >= avg_all, (
            f"Critical edge avg weight ({avg_critical:.2f}) < "
            f"all edge avg weight ({avg_all:.2f})"
        )

    def test_critical_edge_weights_populated(self):
        """临界边权重列表与临界边数一致。"""
        rels = _line_relations()
        wc = build_weighted_complex_from_relations(rels)
        result = weighted_morse_function(wc)
        field = result.gradient_field
        assert len(result.critical_edge_weights) == len(field.unpaired_1)


# ─────────────────────────────────────────────────────────────────────────────
# 测试：加权 vs 均匀对比
# ─────────────────────────────────────────────────────────────────────────────

class TestCompareWeightedVsUniform:
    """测试 compare_weighted_vs_uniform 的结构完整性。"""

    def test_comparison_structure_complete(self):
        """对比结果包含所有必要字段。"""
        rels = _triangle_relations()
        wc = build_weighted_complex_from_relations(rels)
        comp = compare_weighted_vs_uniform(wc)

        assert comp.uniform_critical_count >= 0
        assert comp.weighted_critical_count >= 0
        assert isinstance(comp.only_in_weighted, tuple)
        assert isinstance(comp.only_in_uniform, tuple)
        assert isinstance(comp.betti, compute_betti_numbers(wc.simplicial_complex).__class__)

    def test_symmetric_difference_consistent(self):
        """对称差分与两个临界集一致。"""
        rels = _mixed_weight_diamond()
        wc = build_weighted_complex_from_relations(rels)
        comp = compare_weighted_vs_uniform(wc)

        w_set = set(comp.weighted_critical_edges)
        u_set = set(comp.uniform_critical_edges)

        assert set(comp.only_in_weighted) == w_set - u_set
        assert set(comp.only_in_uniform) == u_set - w_set

    def test_avg_weight_nonnegative(self):
        """平均权重非负。"""
        rels = _star_relations()
        wc = build_weighted_complex_from_relations(rels)
        comp = compare_weighted_vs_uniform(wc)

        assert comp.avg_weight_weighted_critical >= 0
        assert comp.avg_weight_uniform_critical >= 0

    def test_betti_consistent(self):
        """对比结果中的 Betti 数与直接计算一致。"""
        rels = _triangle_relations()
        wc = build_weighted_complex_from_relations(rels)
        comp = compare_weighted_vs_uniform(wc)
        betti_direct = compute_betti_numbers(wc.simplicial_complex)
        assert comp.betti.beta_0 == betti_direct.beta_0
        assert comp.betti.beta_1 == betti_direct.beta_1
        assert comp.betti.beta_2 == betti_direct.beta_2


# ─────────────────────────────────────────────────────────────────────────────
# 测试：权重表默认值
# ─────────────────────────────────────────────────────────────────────────────

class TestDefaultWeights:
    """测试权重表的合理性。"""

    def test_negates_highest(self):
        """negates 权重最高。"""
        assert DEFAULT_WEIGHTS["negates"] >= max(
            v for k, v in DEFAULT_WEIGHTS.items() if k != "negates" and k != "negated_by"
        )

    def test_depends_on_lowest(self):
        """depends_on 权重最低。"""
        assert DEFAULT_WEIGHTS["depends_on"] <= min(
            v for k, v in DEFAULT_WEIGHTS.items()
        )

    def test_all_weights_positive(self):
        """所有权重为正整数。"""
        for k, v in DEFAULT_WEIGHTS.items():
            assert isinstance(v, int) and v > 0, f"{k}: {v}"
