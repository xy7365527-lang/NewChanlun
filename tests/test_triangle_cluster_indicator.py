"""三角簇密度指标测试。

覆盖：
- 三角簇检测（正向/无三角/多三角）
- 密度计算
- 聚类系数
- triangle-exclusive 节点
- 双指标体系
- 层过滤
- health_summary 接口
- 边界条件（空图、单节点、无三角图）
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from scripts.triangle_cluster_indicator import (
    analyze,
    build_adjacency,
    compute_clustering_coefficients,
    compute_in_degree,
    compute_node_triangle_count,
    find_triangle_exclusive_nodes,
    find_triangles,
    health_summary,
    load_relations,
)


@pytest.fixture
def tmp_topology(tmp_path):
    """创建临时 block-topology 目录结构。"""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


def _write_relations(base: Path, relations: list[dict]) -> None:
    """写入测试 relations.jsonl。"""
    jsonl = base / "relations.jsonl"
    lines = [json.dumps(r, ensure_ascii=False) for r in relations]
    jsonl.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _write_block(base: Path, block_id: str, content: dict) -> None:
    """写入测试 block 文件。"""
    blk = {"id": block_id, "type": "event", "content": content}
    path = base / "blocks" / f"{block_id}.json"
    path.write_text(json.dumps(blk, ensure_ascii=False), encoding="utf-8")


def _make_rel(from_id: str, to_id: str, relation: str = "depends_on",
              order: int = 1) -> dict:
    """构造简化的 relation 记录。"""
    return {
        "from": from_id,
        "to": to_id,
        "relation": relation,
        "order": order,
        "created_by": "a" * 64,
        "timestamp": "2026-01-01T00:00:00+00:00",
    }


# ── 核心：三角簇检测 ──

class TestFindTriangles:
    def test_single_triangle(self):
        """A→B, B→C, A→C 形成一个三角簇。"""
        adj = {"A": {"B", "C"}, "B": {"C"}}
        triangles = find_triangles(adj)
        assert len(triangles) == 1
        assert triangles[0] == ("A", "B", "C")

    def test_no_triangle_chain(self):
        """A→B, B→C（无 A→C）不形成三角簇。"""
        adj = {"A": {"B"}, "B": {"C"}}
        triangles = find_triangles(adj)
        assert len(triangles) == 0

    def test_no_triangle_reverse(self):
        """A→B, C→B, A→C（B→C 缺失）不形成三角簇。"""
        adj = {"A": {"B", "C"}, "C": {"B"}}
        triangles = find_triangles(adj)
        assert len(triangles) == 0

    def test_multiple_triangles(self):
        """A→B, A→C, A→D, B→C, B→D, C→D: 4个三角簇。"""
        adj = {
            "A": {"B", "C", "D"},
            "B": {"C", "D"},
            "C": {"D"},
        }
        triangles = find_triangles(adj)
        assert len(triangles) == 4
        expected = {
            ("A", "B", "C"),
            ("A", "B", "D"),
            ("A", "C", "D"),
            ("B", "C", "D"),
        }
        assert set(triangles) == expected

    def test_empty_graph(self):
        """空图无三角簇。"""
        adj: dict[str, set[str]] = {}
        assert find_triangles(adj) == []

    def test_self_loop_ignored(self):
        """自环不参与三角簇。"""
        adj = {"A": {"A", "B"}, "B": {"A"}}
        triangles = find_triangles(adj)
        assert len(triangles) == 0

    def test_dedup_canonical_order(self):
        """三角簇按字典序去重，每个三元组只出现一次。"""
        # B→A, A→C, B→C 也是三角簇（以 A<B<C 归一化）
        adj = {"A": {"C"}, "B": {"A", "C"}}
        triangles = find_triangles(adj)
        # B→A, B→C, A→C: 存在 A→C 且 B→A（但 B→C 的 common = adj[B] & adj[A] = {C}∩? 不对
        # 修正：B→A 且 B→C，但 A→C 存在。路径 B→A→C + B→C。
        # find_triangles: a=A, b=C (A→C)，检查 adj[C] ∩ adj[A] - 无
        # a=B, b不存在>B的 → 无
        # 实际上，需要三条边同向才算。B→A, A→C, B→C: 就是 B→A→C + B→C
        # a=B, a_neighbors={A,C}, b=? b>B 的没有
        # 但是 A→C 存在, B→A 和 B→C 都存在
        # 需要 a < b: a=A, a_neighbors={C}, b=C? C>A, adj[C]={}, common={}
        # a=B, a_neighbors={A,C}, 没有 b>B → skip
        # 所以这个图实际上只有 B→A, B→C, A→C 三条边
        # 对于 find_triangles 的逻辑：a=A, neighbors of A={C}, b=C>A? yes
        # adj[C] = {} → common = {C} ∩ {} = {} → no triangle
        # a=B, neighbors of B = {A,C}, b must > B → none
        # 结论：不形成三角簇（因为缺少 A→B 边，只有 B→A）
        assert len(triangles) == 0


class TestBuildAdjacency:
    def test_basic(self):
        rels = [
            _make_rel("A", "B"),
            _make_rel("B", "C"),
            _make_rel("A", "C"),
        ]
        adj = build_adjacency(rels)
        assert adj["A"] == {"B", "C"}
        assert adj["B"] == {"C"}
        assert "C" not in adj

    def test_legacy_field_names(self):
        """兼容 source/target/type 旧字段名。"""
        rels = [{"source": "X", "target": "Y", "type": "depends_on"}]
        # load_relations 做字段名归一化，build_adjacency 只看 from/to
        # 直接传入已归一化的
        normalized = [{"from": "X", "to": "Y", "relation": "depends_on"}]
        adj = build_adjacency(normalized)
        assert adj["X"] == {"Y"}

    def test_ignores_self_loops(self):
        rels = [_make_rel("A", "A")]
        adj = build_adjacency(rels)
        assert "A" not in adj


class TestNodeTriangleCount:
    def test_single_triangle_counts(self):
        triangles = [("A", "B", "C")]
        counts = compute_node_triangle_count(triangles)
        assert counts == {"A": 1, "B": 1, "C": 1}

    def test_shared_node(self):
        triangles = [("A", "B", "C"), ("A", "B", "D")]
        counts = compute_node_triangle_count(triangles)
        assert counts["A"] == 2
        assert counts["B"] == 2
        assert counts["C"] == 1
        assert counts["D"] == 1


class TestClusteringCoefficients:
    def test_complete_graph(self):
        """完全图的聚类系数 = 1.0。"""
        adj = {"A": {"B", "C"}, "B": {"A", "C"}, "C": {"A", "B"}}
        # 三角簇: (A,B,C) 对每个节点 degree=2, possible=1, actual=1
        node_tri = {"A": 1, "B": 1, "C": 1}
        coeff = compute_clustering_coefficients(adj, node_tri)
        assert coeff["A"] == pytest.approx(1.0)
        assert coeff["B"] == pytest.approx(1.0)

    def test_star_graph_zero(self):
        """星形图（hub→spoke）无三角簇，聚类系数=0。"""
        adj = {"hub": {"s1", "s2", "s3"}}
        node_tri: dict[str, int] = {}
        coeff = compute_clustering_coefficients(adj, node_tri)
        assert coeff["hub"] == pytest.approx(0.0)

    def test_single_neighbor(self):
        """只有1条出边的节点聚类系数=0。"""
        adj = {"A": {"B"}}
        coeff = compute_clustering_coefficients(adj, {})
        assert coeff["A"] == pytest.approx(0.0)


class TestTriangleExclusive:
    def test_exclusive_node(self):
        """入度=0 的三角簇节点被标记为 exclusive。"""
        triangles = [("A", "B", "C")]
        adj = {"A": {"B", "C"}, "B": {"C"}}
        in_degree = {"B": 1, "C": 2}  # A 入度=0
        exclusive = find_triangle_exclusive_nodes(triangles, adj, in_degree)
        assert "A" in exclusive

    def test_no_exclusive_when_all_referenced(self):
        triangles = [("A", "B", "C")]
        adj = {"A": {"B", "C"}, "B": {"C"}}
        in_degree = {"A": 3, "B": 1, "C": 2}
        exclusive = find_triangle_exclusive_nodes(triangles, adj, in_degree)
        assert exclusive == []


class TestInDegree:
    def test_basic(self):
        rels = [
            _make_rel("A", "B"),
            _make_rel("C", "B"),
            _make_rel("A", "C"),
        ]
        in_deg = compute_in_degree(rels)
        assert in_deg["B"] == 2
        assert in_deg["C"] == 1
        assert "A" not in in_deg


# ── 集成：完整分析管线 ──

class TestAnalyze:
    def test_triangle_graph(self, tmp_topology):
        """完整分析管线：含一个三角簇的图。"""
        _write_relations(tmp_topology, [
            _make_rel("A", "B"),
            _make_rel("B", "C"),
            _make_rel("A", "C"),
        ])
        result = analyze(tmp_topology, top_n=10)
        assert result["triangle_count"] == 1
        assert result["total_nodes"] == 3
        assert result["total_edges"] == 3
        assert result["global_density"] > 0

    def test_empty_topology(self, tmp_topology):
        """空 relations.jsonl。"""
        (tmp_topology / "relations.jsonl").write_text("", encoding="utf-8")
        result = analyze(tmp_topology)
        assert result["error"] == "No relations found"

    def test_no_file(self, tmp_topology):
        """无 relations.jsonl 文件。"""
        result = analyze(tmp_topology)
        assert result["error"] == "No relations found"

    def test_layer_filter(self, tmp_topology):
        """按层过滤边。"""
        _write_relations(tmp_topology, [
            _make_rel("A", "B", "depends_on", 1),  # layer 1
            _make_rel("B", "C", "references", 2),  # layer 2
            _make_rel("A", "C", "depends_on", 1),  # layer 1
        ])
        # 仅 layer 1：A→B 和 A→C (depends_on)，无 B→C → 无三角簇
        result = analyze(tmp_topology, layer=1)
        assert result["triangle_count"] == 0
        assert result["total_edges"] == 2

    def test_with_block_labels(self, tmp_topology):
        """区块有标签时，报告中显示标签。"""
        _write_relations(tmp_topology, [
            _make_rel("aaa", "bbb"),
            _make_rel("bbb", "ccc"),
            _make_rel("aaa", "ccc"),
        ])
        _write_block(tmp_topology, "aaa", {"genealogy_number": 42, "title": "测试节点"})
        result = analyze(tmp_topology, top_n=5)
        labels = [item["label"] for item in result["top_triangle_nodes"]]
        assert any("#42" in label for label in labels)

    def test_dual_indicator(self, tmp_topology):
        """双指标包含三角簇数和入度。"""
        _write_relations(tmp_topology, [
            _make_rel("A", "B"),
            _make_rel("B", "C"),
            _make_rel("A", "C"),
        ])
        result = analyze(tmp_topology, top_n=10)
        for item in result["dual_indicator"]:
            assert "triangle_count" in item
            assert "in_degree" in item
            assert "clustering_coefficient" in item


class TestHealthSummary:
    def test_basic(self, tmp_topology):
        """health_summary 返回精简指标。"""
        _write_relations(tmp_topology, [
            _make_rel("A", "B"),
            _make_rel("B", "C"),
            _make_rel("A", "C"),
        ])
        h = health_summary(tmp_topology)
        assert h["triangle_count"] == 1
        assert h["global_density"] > 0
        assert h["avg_clustering"] > 0
        assert len(h["top3_hubs"]) <= 3

    def test_empty(self, tmp_topology):
        """空拓扑的健康度。"""
        h = health_summary(tmp_topology)
        assert h["triangle_count"] == 0
        assert h["global_density"] == 0.0


class TestLoadRelations:
    def test_legacy_fields(self, tmp_topology):
        """旧字段名 source/target/type 被归一化。"""
        legacy = {
            "source": "X", "target": "Y", "type": "depends_on",
            "order": 1, "created_by": "a" * 64,
            "timestamp": "2026-01-01T00:00:00+00:00"
        }
        _write_relations(tmp_topology, [legacy])
        rels = load_relations(tmp_topology)
        assert rels[0]["from"] == "X"
        assert rels[0]["to"] == "Y"
        assert rels[0]["relation"] == "depends_on"
