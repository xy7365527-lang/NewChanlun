"""Tests for block_topology prereq infrastructure: LAYER_MAP, classify_layer, get_block_mapping."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from scripts.block_topology import (
    LAYER_MAP,
    RELATION_TYPES,
    classify_layer,
    get_block_mapping,
)


# --- LAYER_MAP / classify_layer ---


def test_layer_map_completeness():
    """LAYER_MAP covers every type in RELATION_TYPES."""
    missing = RELATION_TYPES - set(LAYER_MAP)
    assert missing == set(), f"LAYER_MAP missing: {missing}"


def test_classify_layer_known():
    """Known relation types return the correct layer number."""
    assert classify_layer("depends_on") == 1
    assert classify_layer("negates") == 1
    assert classify_layer("references") == 2
    assert classify_layer("defines") == 2
    assert classify_layer("records") == 3
    assert classify_layer("related") == 3


def test_classify_layer_unknown():
    """Unknown relation type raises ValueError."""
    with pytest.raises(ValueError, match="Unknown relation type"):
        classify_layer("nonexistent_relation")


# --- get_block_mapping ---


def _make_block_file(blocks_dir: Path, block_id: str, content: dict,
                     block_type: str = "event") -> None:
    """Helper: write a minimal block JSON to blocks_dir."""
    blk = {
        "id": block_id,
        "type": block_type,
        "content": content,
    }
    (blocks_dir / f"{block_id}.json").write_text(
        json.dumps(blk, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def test_get_block_mapping_empty(tmp_path):
    """Empty blocks directory returns empty dicts."""
    base = tmp_path / "bt"
    (base / "blocks").mkdir(parents=True)
    # Clear lru_cache before test
    get_block_mapping.cache_clear()
    id2num, num2id = get_block_mapping(base)
    assert id2num == {}
    assert num2id == {}


def test_get_block_mapping_no_blocks_dir(tmp_path):
    """Missing blocks directory returns empty dicts."""
    base = tmp_path / "bt"
    base.mkdir()
    get_block_mapping.cache_clear()
    id2num, num2id = get_block_mapping(base)
    assert id2num == {}
    assert num2id == {}


def test_get_block_mapping_with_data(tmp_path):
    """Blocks with genealogy_number or numeric content.id are mapped correctly."""
    base = tmp_path / "bt"
    blocks_dir = base / "blocks"
    blocks_dir.mkdir(parents=True)

    # Block with genealogy_number
    _make_block_file(blocks_dir, "aaa", {"genealogy_number": 42})
    # Block with number
    _make_block_file(blocks_dir, "bbb", {"number": 99})
    # Block with numeric content.id (migration style)
    _make_block_file(blocks_dir, "ccc", {"id": "007"})

    get_block_mapping.cache_clear()
    id2num, num2id = get_block_mapping(base)

    assert id2num["aaa"] == 42
    assert id2num["bbb"] == 99
    assert id2num["ccc"] == 7
    assert num2id[42] == "aaa"
    assert num2id[99] == "bbb"
    assert num2id[7] == "ccc"


def test_get_block_mapping_skips_no_number(tmp_path):
    """Blocks without any genealogy number are excluded from mapping."""
    base = tmp_path / "bt"
    blocks_dir = base / "blocks"
    blocks_dir.mkdir(parents=True)

    # Block with no number at all
    _make_block_file(blocks_dir, "xxx", {"action": "relation_append"},
                     block_type="rewrite")
    # Block with non-numeric content.id (SHA256 hash)
    _make_block_file(blocks_dir, "yyy", {"id": "abc123def"})

    get_block_mapping.cache_clear()
    id2num, num2id = get_block_mapping(base)

    assert "xxx" not in id2num
    assert "yyy" not in id2num
    assert id2num == {}
    assert num2id == {}
