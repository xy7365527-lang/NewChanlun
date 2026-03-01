"""Tests for scripts/topo_repetition_detect.py — 拓扑重复检测三层架构

测试设计遵循 272 号编排者裁定：
- 拓扑层：多重边检测 + 否定边短环检测
- canonical form 层：缺陷本质 ID 归一化 + 语义重复检测
- 整合层：run_topo_detection 三层联动

所有测试使用构造数据，不依赖实际 relations.jsonl。
"""

from __future__ import annotations

import pytest

from scripts.topo_repetition_detect import (
    CanonicalDuplicate,
    MultiEdge,
    ShortCycle,
    TopoDetectionResult,
    detect_canonical_duplicates,
    detect_multi_edges,
    detect_short_cycles,
    extract_defect_id,
    run_topo_detection,
)


# ── Helpers ──


def _make_rel(
    from_id: str,
    to_id: str,
    relation: str = "depends_on",
    order: int = 1,
    timestamp: str = "2026-01-01T00:00:00+00:00",
) -> dict:
    return {
        "from": from_id,
        "to": to_id,
        "relation": relation,
        "order": order,
        "created_by": "test",
        "timestamp": timestamp,
    }


# ── 第 1 层：多重边检测 ──


class TestDetectMultiEdges:
    """多重边检测：同源+同目标+同边类型出现 >= 2 次。"""

    def test_no_multi_edges(self):
        """无重复边 → 空结果。"""
        rels = [
            _make_rel("a", "b", "depends_on"),
            _make_rel("a", "c", "depends_on"),
            _make_rel("b", "c", "negates"),
        ]
        result = detect_multi_edges(rels)
        assert result == ()

    def test_detects_duplicate_edge(self):
        """同 (from, to, relation) 出现两次 → 检测到多重边。"""
        rels = [
            _make_rel("a", "b", "depends_on", timestamp="t1"),
            _make_rel("a", "b", "depends_on", timestamp="t2"),
        ]
        result = detect_multi_edges(rels)
        assert len(result) == 1
        assert result[0].from_id == "a"
        assert result[0].to_id == "b"
        assert result[0].relation == "depends_on"
        assert result[0].count == 2

    def test_different_relation_type_not_duplicate(self):
        """同 (from, to) 但不同 relation → 不是多重边。"""
        rels = [
            _make_rel("a", "b", "depends_on"),
            _make_rel("a", "b", "negates"),
        ]
        result = detect_multi_edges(rels)
        assert result == ()

    def test_order_2_excluded(self):
        """order=2 的关系不参与多重边检测。"""
        rels = [
            _make_rel("a", "b", "records", order=2, timestamp="t1"),
            _make_rel("a", "b", "records", order=2, timestamp="t2"),
        ]
        result = detect_multi_edges(rels)
        assert result == ()

    def test_triple_edge(self):
        """同边出现三次 → count=3。"""
        rels = [
            _make_rel("x", "y", "negates", timestamp="t1"),
            _make_rel("x", "y", "negates", timestamp="t2"),
            _make_rel("x", "y", "negates", timestamp="t3"),
        ]
        result = detect_multi_edges(rels)
        assert len(result) == 1
        assert result[0].count == 3


# ── 第 1 层：短环检测 ──


class TestDetectShortCycles:
    """否定边短环：A negates B 且 B negates A。"""

    def test_no_cycles(self):
        """无互否 → 空结果。"""
        rels = [
            _make_rel("a", "b", "negates"),
            _make_rel("c", "d", "negates"),
        ]
        result = detect_short_cycles(rels)
        assert result == ()

    def test_detects_mutual_negation(self):
        """A→B + B→A negates → 检测到短环。"""
        rels = [
            _make_rel("a", "b", "negates"),
            _make_rel("b", "a", "negates"),
        ]
        result = detect_short_cycles(rels)
        assert len(result) == 1
        # 归一化顺序：min(a, b) = a
        assert result[0].node_a == "a"
        assert result[0].node_b == "b"

    def test_depends_on_not_counted(self):
        """depends_on 互指不算短环（只检查 negates）。"""
        rels = [
            _make_rel("a", "b", "depends_on"),
            _make_rel("b", "a", "depends_on"),
        ]
        result = detect_short_cycles(rels)
        assert result == ()

    def test_order_2_excluded(self):
        """order=2 的 negates 不参与短环检测。"""
        rels = [
            _make_rel("a", "b", "negates", order=2),
            _make_rel("b", "a", "negates", order=2),
        ]
        result = detect_short_cycles(rels)
        assert result == ()

    def test_multiple_cycles(self):
        """多对互否 → 检测到多个短环。"""
        rels = [
            _make_rel("a", "b", "negates"),
            _make_rel("b", "a", "negates"),
            _make_rel("c", "d", "negates"),
            _make_rel("d", "c", "negates"),
        ]
        result = detect_short_cycles(rels)
        assert len(result) == 2


# ── 第 2 层：canonical form ──


class TestExtractDefectId:
    """缺陷本质 ID 提取。"""

    def test_both_fields_present(self):
        content = {
            "negation_reason": "定义冲突",
            "source_premise": "089号",
        }
        assert extract_defect_id(content) == "定义冲突|089号"

    def test_no_negation_reason(self):
        content = {"source_premise": "089号"}
        assert extract_defect_id(content) is None

    def test_empty_negation_reason(self):
        content = {"negation_reason": "", "source_premise": "089号"}
        assert extract_defect_id(content) is None

    def test_no_source_premise(self):
        """source_premise 缺失时仍返回 defect_id（premise 为空串）。"""
        content = {"negation_reason": "定义冲突"}
        assert extract_defect_id(content) == "定义冲突|"

    def test_whitespace_normalized(self):
        content = {
            "negation_reason": "  定义冲突  ",
            "source_premise": "  089号  ",
        }
        assert extract_defect_id(content) == "定义冲突|089号"


class TestDetectCanonicalDuplicates:
    """canonical form 层：相同 defect_id 的不同区块 = 语义重复。"""

    def test_no_negates_relations(self):
        """无 negates 关系 → 空结果。"""
        rels = [_make_rel("a", "b", "depends_on")]
        result = detect_canonical_duplicates(rels, blocks_reader=lambda _: None)
        assert result == ()

    def test_no_duplicates(self):
        """不同 defect_id → 无重复。"""
        rels = [
            _make_rel("block_a", "x", "negates"),
            _make_rel("block_b", "y", "negates"),
        ]
        blocks = {
            "block_a": {
                "content": {
                    "negation_reason": "定义冲突A",
                    "source_premise": "001号",
                },
            },
            "block_b": {
                "content": {
                    "negation_reason": "定义冲突B",
                    "source_premise": "002号",
                },
            },
        }
        result = detect_canonical_duplicates(
            rels, blocks_reader=lambda bid: blocks.get(bid),
        )
        assert result == ()

    def test_detects_duplicates(self):
        """相同 defect_id 的两个区块 → 检测到语义重复。"""
        rels = [
            _make_rel("block_a", "x", "negates"),
            _make_rel("block_b", "y", "negates"),
        ]
        blocks = {
            "block_a": {
                "content": {
                    "negation_reason": "定义冲突",
                    "source_premise": "089号",
                },
            },
            "block_b": {
                "content": {
                    "negation_reason": "定义冲突",
                    "source_premise": "089号",
                },
            },
        }
        result = detect_canonical_duplicates(
            rels, blocks_reader=lambda bid: blocks.get(bid),
        )
        assert len(result) == 1
        assert result[0].defect_id == "定义冲突|089号"
        assert set(result[0].block_ids) == {"block_a", "block_b"}

    def test_block_not_found_skipped(self):
        """区块读取返回 None → 跳过。"""
        rels = [_make_rel("missing", "x", "negates")]
        result = detect_canonical_duplicates(
            rels, blocks_reader=lambda _: None,
        )
        assert result == ()


# ── 三层整合 ──


class TestRunTopoDetection:
    """run_topo_detection 三层联动。"""

    def test_clean_topology(self, tmp_path):
        """干净的拓扑 → should_suspend=False。"""
        # 创建空 relations.jsonl
        base = tmp_path / "block-topology"
        base.mkdir()
        (base / "relations.jsonl").write_text("", encoding="utf-8")

        result = run_topo_detection(base=base)
        assert result.should_suspend is False
        assert result.multi_edges == ()
        assert result.short_cycles == ()
        assert result.canonical_duplicates == ()
        assert result.suspend_reason == ""

    def test_multi_edge_triggers_suspend(self, tmp_path):
        """多重边 → should_suspend=True。"""
        import json

        base = tmp_path / "block-topology"
        base.mkdir()
        rels = [
            _make_rel("a", "b", "depends_on", timestamp="t1"),
            _make_rel("a", "b", "depends_on", timestamp="t2"),
        ]
        lines = [json.dumps(r, ensure_ascii=False) for r in rels]
        (base / "relations.jsonl").write_text(
            "\n".join(lines) + "\n", encoding="utf-8",
        )

        result = run_topo_detection(base=base)
        assert result.should_suspend is True
        assert len(result.multi_edges) == 1
        assert "多重边" in result.suspend_reason

    def test_short_cycle_triggers_suspend(self, tmp_path):
        """否定短环 → should_suspend=True。"""
        import json

        base = tmp_path / "block-topology"
        base.mkdir()
        rels = [
            _make_rel("a", "b", "negates"),
            _make_rel("b", "a", "negates"),
        ]
        lines = [json.dumps(r, ensure_ascii=False) for r in rels]
        (base / "relations.jsonl").write_text(
            "\n".join(lines) + "\n", encoding="utf-8",
        )

        result = run_topo_detection(base=base)
        assert result.should_suspend is True
        assert len(result.short_cycles) == 1
        assert "否定短环" in result.suspend_reason
