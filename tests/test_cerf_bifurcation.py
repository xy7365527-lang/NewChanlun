"""
tests/test_cerf_bifurcation.py

离散 Cerf 分岔点检测的测试。

认识论等级：L1（合成数据验证管线正确性）
"""

from __future__ import annotations

import os
import re
import tempfile
from pathlib import Path

import pytest

# 确保 scripts/ 和 experiments/discrete_morse/ 在导入路径上
import sys

_repo_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_repo_root / "scripts"))
sys.path.insert(0, str(_repo_root / "experiments" / "discrete_morse"))

from cerf_bifurcation import (
    MorseSnapshot,
    Bifurcation,
    build_temporal_morse_sequence,
    detect_bifurcations,
    annotate_bifurcations_with_genealogy,
    cerf_summary,
    _parse_frontmatter,
)


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def _make_snapshot(
    t: int,
    genealogy_key: str = "",
    critical_count: int = 0,
    critical_0: int = 0,
    critical_1: int = 0,
    critical_2: int = 0,
    critical_edge_weights: tuple = (),
    betti_0: int = 1,
    betti_1: int = 0,
    betti_2: int = 0,
    complex_size: int = 10,
    num_vertices: int = 5,
    num_edges: int = 4,
    num_triangles: int = 1,
) -> MorseSnapshot:
    """快捷构造 MorseSnapshot 用于测试。"""
    if not genealogy_key:
        genealogy_key = f"{t:03d}"
    return MorseSnapshot(
        t=t,
        genealogy_key=genealogy_key,
        complex_size=complex_size,
        num_vertices=num_vertices,
        num_edges=num_edges,
        num_triangles=num_triangles,
        critical_count=critical_count,
        critical_0=critical_0,
        critical_1=critical_1,
        critical_2=critical_2,
        critical_simplices=(),
        critical_edge_weights=critical_edge_weights,
        betti_0=betti_0,
        betti_1=betti_1,
        betti_2=betti_2,
    )


# ─────────────────────────────────────────────────────────────────────────────
# 1. 平稳序列：无分岔
# ─────────────────────────────────────────────────────────────────────────────

class TestNoBifurcation:
    """平稳序列（无变化或变化低于阈值）不应产生分岔点。"""

    def test_constant_sequence_returns_empty(self):
        """完全不变的序列：无分岔。"""
        seq = [
            _make_snapshot(t=i, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           critical_edge_weights=(2.0,), betti_0=3, betti_1=1, betti_2=1)
            for i in range(10)
        ]
        assert detect_bifurcations(seq, threshold=0.3) == []

    def test_single_snapshot_returns_empty(self):
        """只有一个快照：无法比较，返回空。"""
        seq = [_make_snapshot(t=0, critical_count=5)]
        assert detect_bifurcations(seq, threshold=0.3) == []

    def test_empty_sequence_returns_empty(self):
        """空序列：返回空。"""
        assert detect_bifurcations([], threshold=0.3) == []

    def test_small_change_below_threshold(self):
        """临界数量变化低于阈值。"""
        seq = [
            _make_snapshot(t=0, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           critical_edge_weights=(2.0, 2.0, 2.0), betti_0=5, betti_1=3, betti_2=2),
            _make_snapshot(t=1, critical_count=11, critical_0=5, critical_1=4, critical_2=2,
                           critical_edge_weights=(2.0, 2.0, 2.0, 2.0), betti_0=5, betti_1=3, betti_2=2),
        ]
        # 10 -> 11 = 10% 变化，低于 30% 阈值
        result = detect_bifurcations(seq, threshold=0.3)
        assert result == []


# ─────────────────────────────────────────────────────────────────────────────
# 2. 临界数量变化检测
# ─────────────────────────────────────────────────────────────────────────────

class TestCriticalCountBifurcation:
    """临界数量变化超过阈值应触发分岔点。"""

    def test_large_increase(self):
        """临界数量大幅增加。"""
        seq = [
            _make_snapshot(t=0, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           critical_edge_weights=(1.0, 1.0, 1.0), betti_0=5, betti_1=3, betti_2=2),
            _make_snapshot(t=1, critical_count=15, critical_0=8, critical_1=4, critical_2=3,
                           critical_edge_weights=(1.0, 1.0, 1.0, 1.0), betti_0=5, betti_1=3, betti_2=2),
        ]
        # 10 -> 15 = 50% 增加，超过 30% 阈值
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        bif = result[0]
        assert bif.t == 1
        assert bif.type == "critical_count"
        assert bif.magnitude > 0.3

    def test_large_decrease(self):
        """临界数量大幅减少。"""
        seq = [
            _make_snapshot(t=0, critical_count=20, critical_0=10, critical_1=6, critical_2=4,
                           critical_edge_weights=(1.0,) * 6, betti_0=10, betti_1=6, betti_2=4),
            _make_snapshot(t=1, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           critical_edge_weights=(1.0,) * 3, betti_0=10, betti_1=6, betti_2=4),
        ]
        # 20 -> 10 = 50% 减少
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        assert result[0].type == "critical_count"

    def test_from_zero_to_nonzero(self):
        """从 0 到非零临界数是分岔。"""
        seq = [
            _make_snapshot(t=0, critical_count=0, betti_0=0, betti_1=0, betti_2=0),
            _make_snapshot(t=1, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           betti_0=3, betti_1=1, betti_2=1),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        # 应检测到 critical_count 分岔和 betti_change 分岔
        assert len(result) >= 1
        types = [b.type for b in result]
        assert any(t in ("critical_count", "compound") for t in types)


# ─────────────────────────────────────────────────────────────────────────────
# 3. Betti 数变化检测
# ─────────────────────────────────────────────────────────────────────────────

class TestBettiBifurcation:
    """Betti 数变化应触发分岔点。"""

    def test_betti_0_change(self):
        """连通分量数变化。"""
        seq = [
            _make_snapshot(t=0, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           betti_0=3, betti_1=1, betti_2=0),
            _make_snapshot(t=1, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           betti_0=2, betti_1=1, betti_2=0),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        assert result[0].type == "betti_change"
        assert "Betti" in result[0].description

    def test_betti_1_change(self):
        """环路数变化。"""
        seq = [
            _make_snapshot(t=0, critical_count=5, critical_0=2, critical_1=2, critical_2=1,
                           betti_0=2, betti_1=1, betti_2=0),
            _make_snapshot(t=1, critical_count=5, critical_0=2, critical_1=2, critical_2=1,
                           betti_0=2, betti_1=3, betti_2=0),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        details = result[0].details["betti_change"]
        assert details["delta_betti_1"] == 2

    def test_multiple_betti_changes(self):
        """多个 Betti 数同时变化。"""
        seq = [
            _make_snapshot(t=0, critical_count=5, betti_0=3, betti_1=1, betti_2=0),
            _make_snapshot(t=1, critical_count=5, betti_0=2, betti_1=3, betti_2=1),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        details = result[0].details["betti_change"]
        assert details["total_delta"] == 4  # |3-2| + |1-3| + |0-1| = 1+2+1


# ─────────────────────────────────────────────────────────────────────────────
# 4. 权重偏移检测
# ─────────────────────────────────────────────────────────────────────────────

class TestWeightShiftBifurcation:
    """临界边权重显著偏移应触发分岔点。"""

    def test_weight_increase(self):
        """权重大幅上升。"""
        seq = [
            _make_snapshot(t=0, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           critical_edge_weights=(1.0,), betti_0=3, betti_1=1, betti_2=0),
            _make_snapshot(t=1, critical_count=5, critical_0=3, critical_1=1, critical_2=1,
                           critical_edge_weights=(2.0,), betti_0=3, betti_1=1, betti_2=0),
        ]
        # 平均权重 1.0 -> 2.0 = 100% 变化
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        assert result[0].type == "weight_shift"

    def test_no_weight_data(self):
        """没有权重数据时不触发权重分岔。"""
        seq = [
            _make_snapshot(t=0, critical_count=5, critical_edge_weights=(),
                           betti_0=3, betti_1=1, betti_2=0),
            _make_snapshot(t=1, critical_count=5, critical_edge_weights=(),
                           betti_0=3, betti_1=1, betti_2=0),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        assert result == []


# ─────────────────────────────────────────────────────────────────────────────
# 5. 复合分岔
# ─────────────────────────────────────────────────────────────────────────────

class TestCompoundBifurcation:
    """多种标准同时满足时 type = compound。"""

    def test_compound_critical_and_betti(self):
        """临界数量变化 + Betti 数变化同时触发。"""
        seq = [
            _make_snapshot(t=0, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           betti_0=5, betti_1=3, betti_2=0),
            _make_snapshot(t=1, critical_count=20, critical_0=10, critical_1=6, critical_2=4,
                           betti_0=4, betti_1=5, betti_2=1),
        ]
        result = detect_bifurcations(seq, threshold=0.3)
        assert len(result) == 1
        assert result[0].type == "compound"
        assert "critical_count" in result[0].details
        assert "betti_change" in result[0].details


# ─────────────────────────────────────────────────────────────────────────────
# 6. 谱系标注
# ─────────────────────────────────────────────────────────────────────────────

class TestGenealogyAnnotation:
    """谱系标注功能测试。"""

    def test_annotate_with_negation(self, tmp_path):
        """有 negates 字段的谱系文件标注为关键否定。"""
        settled_dir = tmp_path / "settled"
        settled_dir.mkdir()
        (settled_dir / "005-test.md").write_text(
            '---\nid: "005"\ntitle: "Test negation"\nstatus: "已结算"\n'
            'negates: ["003"]\nnegated_by: []\n---\n# Test\n',
            encoding="utf-8",
        )

        bifs = [Bifurcation(t=5, genealogy_key="005", type="betti_change",
                             magnitude=1.0, description="test", details={})]

        result = annotate_bifurcations_with_genealogy(bifs, str(settled_dir))
        assert len(result) == 1
        ann = result[0]["genealogy_annotation"]
        assert ann is not None
        assert ann["is_critical_negation"] is True
        assert ann["negates"] == ["003"]

    def test_annotate_without_negation(self, tmp_path):
        """没有 negates 字段的谱系文件不标注为关键否定。"""
        settled_dir = tmp_path / "settled"
        settled_dir.mkdir()
        (settled_dir / "010-test.md").write_text(
            '---\nid: "010"\ntitle: "Normal entry"\nstatus: "已结算"\n'
            'negates: []\nnegated_by: []\n---\n# Normal\n',
            encoding="utf-8",
        )

        bifs = [Bifurcation(t=10, genealogy_key="010", type="critical_count",
                             magnitude=0.5, description="test", details={})]

        result = annotate_bifurcations_with_genealogy(bifs, str(settled_dir))
        assert len(result) == 1
        ann = result[0]["genealogy_annotation"]
        assert ann is not None
        assert ann["is_critical_negation"] is False

    def test_annotate_missing_file(self, tmp_path):
        """谱系文件不存在时标注为 None。"""
        settled_dir = tmp_path / "settled"
        settled_dir.mkdir()

        bifs = [Bifurcation(t=99, genealogy_key="099", type="betti_change",
                             magnitude=2.0, description="test", details={})]

        result = annotate_bifurcations_with_genealogy(bifs, str(settled_dir))
        assert len(result) == 1
        assert result[0]["genealogy_annotation"] is None


# ─────────────────────────────────────────────────────────────────────────────
# 7. Cerf 总结
# ─────────────────────────────────────────────────────────────────────────────

class TestCerfSummary:
    """cerf_summary 的测试。"""

    def test_summary_fields(self):
        """总结包含所有必需字段。"""
        seq = [_make_snapshot(t=i, critical_count=5, betti_0=3) for i in range(5)]
        bifs = [Bifurcation(t=2, genealogy_key="002", type="betti_change",
                             magnitude=1.5, description="test", details={})]

        result = cerf_summary(seq, bifs)
        assert result["total_steps"] == 5
        assert result["total_bifurcations"] == 1
        assert abs(result["bifurcation_rate"] - 0.2) < 1e-9
        assert "betti_change" in result["type_distribution"]
        assert result["type_distribution"]["betti_change"] == 1
        assert len(result["top_10_bifurcations"]) == 1
        assert "epistemological_level" in result
        assert "validity_domain" in result

    def test_summary_empty(self):
        """空序列的总结。"""
        result = cerf_summary([], [])
        assert result["total_steps"] == 0
        assert result["total_bifurcations"] == 0
        assert result["bifurcation_rate"] == 0.0


# ─────────────────────────────────────────────────────────────────────────────
# 8. Frontmatter 解析
# ─────────────────────────────────────────────────────────────────────────────

class TestFrontmatterParsing:
    """_parse_frontmatter 的测试。"""

    def test_simple_kv(self):
        fm = _parse_frontmatter('id: "005"\ntitle: "Test"')
        assert fm["id"] == "005"
        assert fm["title"] == "Test"

    def test_list_parsing(self):
        fm = _parse_frontmatter('negates: ["003", "004"]\nnegated_by: []')
        assert fm["negates"] == ["003", "004"]
        assert fm["negated_by"] == []

    def test_empty_input(self):
        fm = _parse_frontmatter("")
        assert fm == {}

    def test_with_dashes(self):
        """包含 --- 分隔符的输入应被跳过。"""
        fm = _parse_frontmatter('---\nid: "001"\n---')
        assert fm["id"] == "001"


# ─────────────────────────────────────────────────────────────────────────────
# 9. 阈值敏感性
# ─────────────────────────────────────────────────────────────────────────────

class TestThresholdSensitivity:
    """不同阈值下分岔点检测的行为。"""

    def test_lower_threshold_detects_more(self):
        """更低的阈值检测到更多分岔点。"""
        seq = [
            _make_snapshot(t=0, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           critical_edge_weights=(2.0, 2.0, 2.0), betti_0=5, betti_1=3, betti_2=2),
            _make_snapshot(t=1, critical_count=12, critical_0=6, critical_1=4, critical_2=2,
                           critical_edge_weights=(2.0, 2.0, 2.0, 2.0), betti_0=5, betti_1=3, betti_2=2),
            _make_snapshot(t=2, critical_count=16, critical_0=8, critical_1=5, critical_2=3,
                           critical_edge_weights=(2.0, 2.0, 2.0, 2.0, 2.0),
                           betti_0=5, betti_1=3, betti_2=2),
        ]
        # 10->12 = 20%, 12->16 = 33%
        high = detect_bifurcations(seq, threshold=0.3)
        low = detect_bifurcations(seq, threshold=0.1)
        assert len(low) >= len(high)

    def test_zero_threshold_detects_all_changes(self):
        """阈值 0 检测所有变化。"""
        seq = [
            _make_snapshot(t=0, critical_count=10, critical_0=5, critical_1=3, critical_2=2,
                           critical_edge_weights=(2.0,) * 3, betti_0=5, betti_1=3, betti_2=2),
            _make_snapshot(t=1, critical_count=11, critical_0=6, critical_1=3, critical_2=2,
                           critical_edge_weights=(2.0,) * 3, betti_0=5, betti_1=3, betti_2=2),
        ]
        # 10->11 = 10%, 在 threshold=0 时应该检测到
        result = detect_bifurcations(seq, threshold=0.0)
        assert len(result) >= 1


# ─────────────────────────────────────────────────────────────────────────────
# 10. build_temporal_morse_sequence 集成测试（合成数据）
# ─────────────────────────────────────────────────────────────────────────────

class TestBuildTemporalMorseSequence:
    """build_temporal_morse_sequence 的集成测试（使用临时文件模拟数据）。"""

    def test_simple_three_node_growth(self, tmp_path):
        """三个节点逐步加入的简单序列。"""
        meta = {
            "id_mapping": {
                "001": "aaa",
                "002": "bbb",
                "003": "ccc",
            }
        }
        relations = [
            {"from": "aaa", "to": "bbb", "relation": "depends_on"},
            {"from": "bbb", "to": "ccc", "relation": "negates"},
        ]

        meta_path = tmp_path / "meta.json"
        relations_path = tmp_path / "relations.jsonl"

        meta_path.write_text(json.dumps(meta), encoding="utf-8")
        with open(relations_path, "w", encoding="utf-8") as f:
            for rel in relations:
                f.write(json.dumps(rel) + "\n")

        seq = build_temporal_morse_sequence(
            str(relations_path), meta_path=str(meta_path),
        )

        assert len(seq) == 3
        # 第一步：单节点
        assert seq[0].num_vertices == 1
        assert seq[0].num_edges == 0
        # 最后一步：3 节点 + 2 边
        assert seq[2].num_vertices == 3
        assert seq[2].num_edges == 2

        # 所有快照都有有效的 Betti 数
        for s in seq:
            assert s.betti_0 >= 0
            assert s.betti_1 >= 0
            assert s.betti_2 >= 0

    def test_triangle_formation(self, tmp_path):
        """三角形闭合时的拓扑变化。"""
        meta = {
            "id_mapping": {
                "001": "aaa",
                "002": "bbb",
                "003": "ccc",
            }
        }
        # 三角形：aaa-bbb, bbb-ccc, aaa-ccc
        relations = [
            {"from": "aaa", "to": "bbb", "relation": "depends_on"},
            {"from": "bbb", "to": "ccc", "relation": "depends_on"},
            {"from": "aaa", "to": "ccc", "relation": "depends_on"},
        ]

        meta_path = tmp_path / "meta.json"
        relations_path = tmp_path / "relations.jsonl"
        meta_path.write_text(json.dumps(meta), encoding="utf-8")
        with open(relations_path, "w", encoding="utf-8") as f:
            for rel in relations:
                f.write(json.dumps(rel) + "\n")

        seq = build_temporal_morse_sequence(
            str(relations_path), meta_path=str(meta_path),
        )
        assert len(seq) == 3
        # 第三步加入 ccc 后形成三角形
        final = seq[2]
        assert final.num_triangles == 1
        assert final.num_edges == 3


# 需要 json 用于集成测试
import json
