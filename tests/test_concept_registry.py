"""Tests for scripts/concept_registry.py — 概念注册表导出"""

from __future__ import annotations

import json

import pytest

from scripts.concept_registry import (
    ConceptEntry,
    ConceptRegistry,
    build_concept_registry,
    export_registry,
    load_registry,
)
from scripts.migrate_to_block_topology import _relations_is_lfs_pointer


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory with empty relations."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    (base / "relations.jsonl").write_text("", encoding="utf-8")
    return base


def _write_relations(base, relations: list[dict]) -> None:
    """Helper: write a list of relation dicts to relations.jsonl."""
    lines = [json.dumps(r, ensure_ascii=False) for r in relations]
    (base / "relations.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")


BLOCK_A = "a" * 64
BLOCK_B = "b" * 64
BLOCK_C = "c" * 64
CONCEPT_X = "x" * 64
CONCEPT_Y = "y" * 64
CREATED_BY = "0" * 64


# --- 1. Empty relations → empty registry ---


def test_build_empty(tmp_base):
    """Empty relations.jsonl produces an empty registry."""
    reg = build_concept_registry(tmp_base)
    assert len(reg.entries) == 0


def test_build_no_file(tmp_path):
    """Missing relations.jsonl produces an empty registry."""
    base = tmp_path / "block-topology"
    base.mkdir()
    reg = build_concept_registry(base)
    assert len(reg.entries) == 0


# --- 2. Basic defines extraction ---


def test_build_with_defines(tmp_base):
    """Defines relations are correctly extracted into concept entries."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "顶底分型间的连接",
        },
        {
            "from": BLOCK_B,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert CONCEPT_X in reg.entries
    entry = reg.entries[CONCEPT_X]
    assert entry.term == "笔"
    assert set(entry.defining_blocks) == {BLOCK_A, BLOCK_B}
    # One defines edge has definition → authoritative
    assert entry.authoritative is True


# --- 3. Reference count ---


def test_reference_count(tmp_base):
    """reference_count = sum of references edges pointing to defining blocks."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "线段",
            "concept_definition": "至少三笔",
        },
        # BLOCK_C references BLOCK_A (a defining block of concept X)
        {
            "from": BLOCK_C,
            "to": BLOCK_A,
            "relation": "references",
            "order": 2,
            "created_by": CREATED_BY,
        },
        # BLOCK_B also references BLOCK_A
        {
            "from": BLOCK_B,
            "to": BLOCK_A,
            "relation": "references",
            "order": 2,
            "created_by": CREATED_BY,
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].reference_count == 2


def test_reference_count_non_defining_block(tmp_base):
    """References to non-defining blocks don't inflate reference_count."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "中枢",
            "concept_definition": "至少三段重叠",
        },
        # BLOCK_C references BLOCK_B (not a defining block)
        {
            "from": BLOCK_C,
            "to": BLOCK_B,
            "relation": "references",
            "order": 2,
            "created_by": CREATED_BY,
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].reference_count == 0


# --- 4. Authoritative flag ---


def test_authoritative_with_definition(tmp_base):
    """Defines edge with concept_definition → authoritative=True."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "走势",
            "concept_definition": "某级别上的运动",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].authoritative is True


def test_not_authoritative_without_definition(tmp_base):
    """Defines edge without concept_definition → authoritative=False."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "背驰",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].authoritative is False


def test_authoritative_mixed_edges(tmp_base):
    """If any defines edge has definition, concept is authoritative."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "买卖点",
        },
        {
            "from": BLOCK_B,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "买卖点",
            "concept_definition": "三类买卖点体系",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].authoritative is True


# --- 5. Export and load roundtrip ---


def test_export_and_load(tmp_base, tmp_path):
    """Export then load produces identical data."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "顶底分型间连接",
        },
        {
            "from": BLOCK_B,
            "to": CONCEPT_Y,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "线段",
            "concept_definition": "至少三笔",
        },
        {
            "from": BLOCK_C,
            "to": BLOCK_A,
            "relation": "references",
            "order": 2,
            "created_by": CREATED_BY,
        },
    ])
    reg = build_concept_registry(tmp_base)
    out = tmp_path / "registry.json"
    export_registry(reg, out)
    loaded = load_registry(out)

    assert set(loaded.entries.keys()) == set(reg.entries.keys())
    for cid in reg.entries:
        orig = reg.entries[cid]
        copy = loaded.entries[cid]
        assert copy.term == orig.term
        assert copy.authoritative == orig.authoritative
        assert copy.defining_blocks == orig.defining_blocks
        assert copy.reference_count == orig.reference_count


# --- 6. to_dict format ---


def test_to_dict_format(tmp_base):
    """to_dict output matches the specified JSON schema."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "定义内容",
        },
    ])
    reg = build_concept_registry(tmp_base)
    d = reg.to_dict()

    assert CONCEPT_X in d
    entry = d[CONCEPT_X]
    assert set(entry.keys()) == {"term", "authoritative", "defining_blocks", "reference_count"}
    assert isinstance(entry["term"], str)
    assert isinstance(entry["authoritative"], bool)
    assert isinstance(entry["defining_blocks"], list)
    assert isinstance(entry["reference_count"], int)


# --- 7. Multiple concepts ---


def test_multiple_concepts(tmp_base):
    """Multiple concepts from different defines edges are tracked separately."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "def1",
        },
        {
            "from": BLOCK_A,
            "to": CONCEPT_Y,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "线段",
            "concept_definition": "def2",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert len(reg.entries) == 2
    assert reg.entries[CONCEPT_X].term == "笔"
    assert reg.entries[CONCEPT_Y].term == "线段"


# --- 8. Duplicate block in defining_blocks ---


def test_no_duplicate_defining_blocks(tmp_base):
    """Same block defining same concept twice → only one entry in defining_blocks."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "def1",
        },
        {
            "from": BLOCK_A,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "def2",
        },
    ])
    reg = build_concept_registry(tmp_base)
    assert reg.entries[CONCEPT_X].defining_blocks == [BLOCK_A]


# --- 9. load_registry on missing file ---


def test_load_missing_file(tmp_path):
    """Loading from a nonexistent file returns an empty registry."""
    reg = load_registry(tmp_path / "nonexistent.json")
    assert len(reg.entries) == 0


# --- 10. Sorted output ---


def test_to_dict_sorted_by_term(tmp_base):
    """to_dict output is sorted by term."""
    _write_relations(tmp_base, [
        {
            "from": BLOCK_A,
            "to": CONCEPT_Y,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "线段",
            "concept_definition": "def",
        },
        {
            "from": BLOCK_B,
            "to": CONCEPT_X,
            "relation": "defines",
            "order": 2,
            "created_by": CREATED_BY,
            "concept_term": "笔",
            "concept_definition": "def",
        },
    ])
    reg = build_concept_registry(tmp_base)
    d = reg.to_dict()
    terms = [v["term"] for v in d.values()]
    assert terms == sorted(terms)


# --- 11. Real data count (integration) ---


def test_real_data_count():
    """Integration: real relations.jsonl should produce ~997 unique concepts."""
    from pathlib import Path

    real_base = Path(".chanlun/block-topology")
    if not (real_base / "relations.jsonl").exists():
        pytest.skip("No real data available")
    if _relations_is_lfs_pointer(real_base):
        pytest.skip("relations.jsonl is an unresolved Git LFS pointer")

    reg = build_concept_registry(real_base)
    # 从实际数据中统计到 997 个唯一概念
    assert len(reg.entries) >= 900, f"Expected ~997 concepts, got {len(reg.entries)}"
    assert len(reg.entries) <= 1500, f"Expected ~997 concepts, got {len(reg.entries)}"

    # 至少一些概念应该是 authoritative
    auth_count = sum(1 for e in reg.entries.values() if e.authoritative)
    assert auth_count > 0, "Expected some authoritative concepts"
