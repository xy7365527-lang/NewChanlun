"""tests/test_morse_landscape.py — Morse 地形算法测试"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent / "scripts"))

from morse_landscape import MorseLandscape, UnionFind, build_morse_landscape


# ---------------------------------------------------------------------------
# Union-Find 单元测试
# ---------------------------------------------------------------------------

class TestUnionFind:
    def test_find_creates_singleton(self):
        uf = UnionFind()
        assert uf.find("a") == "a"

    def test_union_merges(self):
        uf = UnionFind()
        assert uf.union("a", "b") is True
        assert uf.find("a") == uf.find("b")

    def test_union_same_set_returns_false(self):
        uf = UnionFind()
        uf.union("a", "b")
        assert uf.union("a", "b") is False

    def test_component_count(self):
        uf = UnionFind()
        uf.find("a")
        uf.find("b")
        uf.find("c")
        assert uf.component_count({"a", "b", "c"}) == 3
        uf.union("a", "b")
        assert uf.component_count({"a", "b", "c"}) == 2
        uf.union("b", "c")
        assert uf.component_count({"a", "b", "c"}) == 1


# ---------------------------------------------------------------------------
# 合成数据测试
# ---------------------------------------------------------------------------

def _write_relations(tmp_path: Path, edges: list[dict]) -> Path:
    """写入 relations.jsonl 文件，返回路径。"""
    p = tmp_path / "relations.jsonl"
    with open(p, "w", encoding="utf-8") as f:
        for e in edges:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    return p


class TestSyntheticData:
    def test_empty_relations(self, tmp_path):
        """无 references 边 → 空 landscape，0 节点"""
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:00Z"},
        ])
        ml = build_morse_landscape(relations_path=p, cache_path=None)
        assert ml.stats["nodes"] == 0
        assert ml.stats["edges"] == 0
        assert ml.stats["critical"] == 0
        assert len(ml.edge_marks) == 0

    def test_single_edge(self, tmp_path):
        """单条 reference → 1 tree 边，0 critical"""
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:00Z"},
        ])
        ml = build_morse_landscape(relations_path=p, cache_path=None)
        assert ml.stats["nodes"] == 2
        assert ml.stats["edges"] == 1
        assert ml.stats["tree"] == 1
        assert ml.stats["critical"] == 0
        assert ml.stats["components"] == 1
        assert ml.edge_marks["A:B"] == "tree"

    def test_triangle(self, tmp_path):
        """A→B, B→C, C→A（3 边 3 节点）→ 2 tree + 1 critical，cycle_rank = 3-3+1 = 1"""
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "C", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:03Z"},
        ])
        ml = build_morse_landscape(relations_path=p, cache_path=None)
        assert ml.stats["nodes"] == 3
        assert ml.stats["edges"] == 3
        assert ml.stats["tree"] == 2
        assert ml.stats["critical"] == 1
        assert ml.stats["components"] == 1
        # cycle_rank = edges - nodes + components = 3 - 3 + 1 = 1
        assert ml.stats["critical"] == ml.stats["edges"] - ml.stats["nodes"] + ml.stats["components"]

    def test_sort_by_timestamp(self, tmp_path):
        """验证时间戳排序正确影响 tree/critical 分配。

        边 B→C 时间最早，应先处理成 tree 边。
        """
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "A", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:03Z"},
        ])
        ml = build_morse_landscape(relations_path=p, cache_path=None)
        # B→C 最早 → tree; A→B 次早 → tree; A→C 最晚且 A,C 已连通 → critical
        assert ml.edge_marks["B:C"] == "tree"
        assert ml.edge_marks["A:B"] == "tree"
        assert ml.edge_marks["A:C"] == "critical"

    def test_no_merge_bidirectional(self, tmp_path):
        """A→B 和 B→A 是两条不同的边（不合并），验证两条边独立标记。"""
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        ml = build_morse_landscape(relations_path=p, cache_path=None)
        assert ml.stats["edges"] == 2
        assert ml.edge_marks["A:B"] == "tree"
        assert ml.edge_marks["B:A"] == "critical"
        assert ml.stats["tree"] == 1
        assert ml.stats["critical"] == 1

    def test_cache_roundtrip(self, tmp_path):
        """build → 缓存 → 从缓存加载 → 结果一致"""
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        cache = tmp_path / "cache.json"

        ml1 = build_morse_landscape(relations_path=p, cache_path=cache)
        assert cache.exists()

        ml2 = build_morse_landscape(relations_path=p, cache_path=cache)
        assert ml1.edge_marks == ml2.edge_marks
        assert ml1.stats == ml2.stats


# ---------------------------------------------------------------------------
# 真实数据测试
# ---------------------------------------------------------------------------

REAL_RELATIONS = Path(".chanlun/block-topology/relations.jsonl")


@pytest.mark.skipif(not REAL_RELATIONS.exists(), reason="真实数据文件不存在")
class TestRealData:
    def test_real_data_hard_constraint(self):
        """在真实 relations.jsonl 上运行，验证 862 硬约束。"""
        landscape = build_morse_landscape(
            relations_path=REAL_RELATIONS,
            cache_path=None,
        )
        assert landscape.stats["nodes"] == 427
        assert landscape.stats["edges"] == 1825
        assert landscape.stats["critical"] == 1400
        assert landscape.stats["components"] == 2
        # 硬约束验证：cycle_rank = edges - nodes + components
        assert (
            landscape.stats["critical"]
            == landscape.stats["edges"] - landscape.stats["nodes"] + landscape.stats["components"]
        )
