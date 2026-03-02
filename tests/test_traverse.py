"""tests/test_traverse.py — traverse.py 查询接口测试"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# 确保 scripts 在 path 中
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from block_topology import get_block_mapping
from concept_registry import ConceptEntry, ConceptRegistry
from morse_landscape import MorseLandscape
from traverse import query_block, query_concept, query_pair


# --- Fixtures ---


@pytest.fixture()
def tmp_base(tmp_path: Path) -> Path:
    """创建 block-topology 临时目录结构。"""
    base = tmp_path / "block-topology"
    (base / "blocks").mkdir(parents=True)
    return base


def _write_block(base: Path, block_id: str, block_type: str = "event",
                 content: dict | None = None) -> None:
    """写入测试用区块 JSON。"""
    if content is None:
        content = {}
    blk = {
        "id": block_id,
        "type": block_type,
        "timestamp": "2026-01-01T00:00:00+00:00",
        "source": "cc",
        "content": content,
        "refs": [],
        "git_ref": "",
    }
    path = base / "blocks" / f"{block_id}.json"
    path.write_text(json.dumps(blk, ensure_ascii=False, indent=2), encoding="utf-8")


def _write_relations(base: Path, relations: list[dict]) -> None:
    """写入测试用 relations.jsonl。"""
    jsonl_path = base / "relations.jsonl"
    with open(jsonl_path, "w", encoding="utf-8") as f:
        for rel in relations:
            f.write(json.dumps(rel, ensure_ascii=False) + "\n")


def _make_rel(from_id: str, to_id: str, relation: str = "references",
              **extra) -> dict:
    """构造测试用关系 dict。"""
    rec = {
        "from": from_id,
        "to": to_id,
        "relation": relation,
        "order": extra.pop("order", 2),
        "created_by": "a" * 64,
        "timestamp": "2026-01-01T00:00:00+00:00",
    }
    rec.update(extra)
    return rec


# --- query_block ---


class TestQueryBlock:

    def test_query_block_basic(self, tmp_base: Path) -> None:
        """构造简单区块 + 关系 → 验证返回结构完整。"""
        # 清除 get_block_mapping LRU 缓存
        get_block_mapping.cache_clear()

        _write_block(tmp_base, "aaa", content={
            "title": "测试区块",
            "status": "settled",
            "genealogy_number": 42,
        })
        _write_block(tmp_base, "bbb", content={"title": "目标区块"})
        _write_relations(tmp_base, [
            _make_rel("aaa", "bbb", "references"),
            _make_rel("bbb", "aaa", "depends_on", order=1),
        ])

        morse = MorseLandscape(
            edge_marks={"aaa:bbb": "tree"},
            stats={"nodes": 2, "edges": 1, "critical": 0, "components": 1, "tree": 1},
        )
        registry = ConceptRegistry()

        result = query_block("aaa", morse, registry, base=tmp_base)

        assert result is not None
        assert result["block_id"] == "aaa"
        assert result["genealogy_number"] == 42
        assert result["title"] == "测试区块"
        assert result["type"] == "event"
        assert result["status"] == "settled"
        assert "references" in result["out_edges"]
        assert result["out_edges"]["references"][0]["to"] == "bbb"
        assert result["out_edges"]["references"][0]["mark"] == "tree"
        assert "depends_on" in result["in_edges"]
        assert result["in_edges"]["depends_on"][0]["from"] == "bbb"

    def test_query_block_with_morse_marks(self, tmp_base: Path) -> None:
        """验证 references 边正确带有 tree/critical 标记。"""
        get_block_mapping.cache_clear()

        _write_block(tmp_base, "x1", content={"title": "X1"})
        _write_block(tmp_base, "x2", content={"title": "X2"})
        _write_block(tmp_base, "x3", content={"title": "X3"})
        _write_relations(tmp_base, [
            _make_rel("x1", "x2", "references"),
            _make_rel("x1", "x3", "references"),
        ])

        morse = MorseLandscape(
            edge_marks={"x1:x2": "tree", "x1:x3": "critical"},
            stats={"nodes": 3, "edges": 2, "critical": 1, "components": 1, "tree": 1},
        )
        registry = ConceptRegistry()

        result = query_block("x1", morse, registry, base=tmp_base)
        assert result is not None

        refs = result["out_edges"]["references"]
        marks = {e["to"]: e["mark"] for e in refs}
        assert marks["x2"] == "tree"
        assert marks["x3"] == "critical"

        assert result["morse_summary"]["critical_out"] == 1
        assert result["morse_summary"]["critical_in"] == 0

    def test_query_block_not_found(self, tmp_base: Path) -> None:
        """不存在的 block_id → 返回 None。"""
        get_block_mapping.cache_clear()

        morse = MorseLandscape(edge_marks={}, stats={
            "nodes": 0, "edges": 0, "critical": 0, "components": 0, "tree": 0,
        })
        registry = ConceptRegistry()

        result = query_block("nonexistent", morse, registry, base=tmp_base)
        assert result is None

    def test_query_block_concepts(self, tmp_base: Path) -> None:
        """验证 concepts 字段正确从注册表提取。"""
        get_block_mapping.cache_clear()

        _write_block(tmp_base, "c1", content={"title": "概念区块"})
        _write_relations(tmp_base, [])

        morse = MorseLandscape(edge_marks={}, stats={
            "nodes": 0, "edges": 0, "critical": 0, "components": 0, "tree": 0,
        })
        registry = ConceptRegistry(entries={
            "concept_abc": ConceptEntry(
                term="笔",
                authoritative=True,
                defining_blocks=["c1"],
                reference_count=5,
            ),
            "concept_def": ConceptEntry(
                term="段",
                authoritative=False,
                defining_blocks=["c2"],  # 不包含 c1
                reference_count=3,
            ),
        })

        result = query_block("c1", morse, registry, base=tmp_base)
        assert result is not None
        assert len(result["concepts"]) == 1
        assert result["concepts"][0]["concept_id"] == "concept_abc"
        assert result["concepts"][0]["term"] == "笔"
        assert result["concepts"][0]["authoritative"] is True


# --- query_pair ---


class TestQueryPair:

    def test_query_pair_direct(self, tmp_base: Path) -> None:
        """A→B → 长度 1。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
        ])

        result = query_pair("A", "B", base=tmp_base)
        assert result is not None
        assert result["length"] == 1
        assert result["nodes"] == ["A", "B"]
        assert result["edges"] == ["A:B"]

    def test_query_pair_indirect(self, tmp_base: Path) -> None:
        """A→B→C → 长度 2。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
            _make_rel("B", "C", "references"),
        ])

        result = query_pair("A", "C", base=tmp_base)
        assert result is not None
        assert result["length"] == 2
        assert result["nodes"] == ["A", "B", "C"]

    def test_query_pair_unreachable(self, tmp_base: Path) -> None:
        """不连通 → None。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
            _make_rel("C", "D", "references"),
        ])

        result = query_pair("A", "C", base=tmp_base)
        assert result is None

    def test_query_pair_bidirectional(self, tmp_base: Path) -> None:
        """A→B 存在时可以从 B 走到 A（无向 BFS）。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
        ])

        result = query_pair("B", "A", base=tmp_base)
        assert result is not None
        assert result["length"] == 1
        assert result["nodes"] == ["B", "A"]
        # edge_key 仍是有向的 A:B
        assert result["edges"] == ["A:B"]

    def test_query_pair_same_node(self, tmp_base: Path) -> None:
        """src == dst → 长度 0。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
        ])

        result = query_pair("A", "A", base=tmp_base)
        assert result is not None
        assert result["length"] == 0
        assert result["nodes"] == ["A"]
        assert result["edges"] == []
        assert result["marks"] == []

    def test_query_pair_not_in_graph(self, tmp_base: Path) -> None:
        """src 或 dst 不在图中 → None。"""
        _write_relations(tmp_base, [
            _make_rel("A", "B", "references"),
        ])

        assert query_pair("Z", "A", base=tmp_base) is None
        assert query_pair("A", "Z", base=tmp_base) is None


# --- query_concept ---


class TestQueryConcept:

    def test_query_concept_exists(self) -> None:
        """已知概念 → 正确返回。"""
        registry = ConceptRegistry(entries={
            "cid_1": ConceptEntry(
                term="走势",
                authoritative=True,
                defining_blocks=["b1", "b2"],
                reference_count=10,
            ),
        })

        result = query_concept("cid_1", registry)
        assert result["concept_id"] == "cid_1"
        assert result["term"] == "走势"
        assert result["authoritative"] is True
        assert result["defining_blocks"] == ["b1", "b2"]
        assert result["reference_count"] == 10

    def test_query_concept_not_exists(self) -> None:
        """未知概念 → 空结果。"""
        registry = ConceptRegistry()

        result = query_concept("nonexistent", registry)
        assert result["concept_id"] == "nonexistent"
        assert result["term"] == ""
        assert result["authoritative"] is False
        assert result["defining_blocks"] == []
        assert result["reference_count"] == 0
