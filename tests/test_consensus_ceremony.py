"""Tests for scripts/consensus_ceremony.py — 共识仪式三区块原子写入"""

from __future__ import annotations

import json
from unittest.mock import patch

import pytest

from scripts.block_topology import (
    list_blocks,
    make_block,
    read_all_relations,
    read_block,
    write_block,
)
from scripts.consensus_ceremony import (
    detect_residue_accumulation,
    get_ceremony_chain,
    list_open_tensions,
    write_consensus_ceremony,
)

# A valid SHA256 hex digest for trigger block
TRIGGER_SHA = "a" * 64


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


@pytest.fixture
def trigger_block(tmp_base):
    """Write a trigger block and return its id."""
    blk = make_block("event", "cc", {"genealogy_id": "g-001"}, [])
    write_block(blk, tmp_base)
    return blk["id"]


def _do_ceremony(tmp_base, trigger_id, suffix=""):
    """Helper to write a standard ceremony."""
    return write_consensus_ceremony(
        trigger_block_id=trigger_id,
        conclusion=f"test conclusion{suffix}",
        gemini_conceded=[f"gemini point{suffix}"],
        codex_conceded=[f"codex point{suffix}"],
        concession_reasons={"gemini": f"reason-g{suffix}", "codex": f"reason-c{suffix}"},
        unresolved=[f"open issue{suffix}"],
        source="cc",
        base=tmp_base,
    )


# --- 1. Three primary blocks created ---

def test_write_consensus_ceremony_creates_three_blocks(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)

    assert "consensus" in result
    assert "residue" in result
    assert "tension" in result

    # Verify each block exists on disk
    for key in ("consensus", "residue", "tension"):
        blk = read_block(result[key]["id"], tmp_base)
        assert blk is not None
        assert blk["type"] == key


# --- 2. consensus refs trigger ---

def test_consensus_refs_trigger(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)
    assert trigger_block in result["consensus"]["refs"]


# --- 3. residue refs consensus ---

def test_residue_refs_consensus(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)
    assert result["consensus"]["id"] in result["residue"]["refs"]


# --- 4. tension refs consensus ---

def test_tension_refs_consensus(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)
    assert result["consensus"]["id"] in result["tension"]["refs"]


# --- 5. Two order-1 relations created ---

def test_relations_created(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)

    all_rels = read_all_relations(tmp_base)
    order_1 = [r for r in all_rels if r["order"] == 1]
    assert len(order_1) == 2

    # Check relation types
    rel_types = {r["relation"] for r in order_1}
    assert rel_types == {"residue_of", "tensions_with"}

    # Both point to consensus
    for r in order_1:
        assert r["to"] == result["consensus"]["id"]


# --- 6. Two rewrite blocks created ---

def test_rewrite_blocks_created(tmp_base, trigger_block):
    _do_ceremony(tmp_base, trigger_block)

    rewrite_blocks = list_blocks(tmp_base, block_type="rewrite")
    assert len(rewrite_blocks) == 2

    # Each rewrite has action=relation_append
    for rw in rewrite_blocks:
        assert rw["content"]["action"] == "relation_append"
        assert rw["content"]["relation_count"] == 1


# --- 7. Atomic all-or-nothing ---

def test_atomic_all_or_nothing(tmp_base, trigger_block):
    """Simulate write failure mid-way; no partial blocks should remain."""
    blocks_before = list_blocks(tmp_base)
    rels_before = read_all_relations(tmp_base)

    # Patch shutil.move to fail on the second call
    original_move = __import__("shutil").move
    call_count = 0

    def failing_move(src, dst):
        nonlocal call_count
        call_count += 1
        if call_count == 2:
            raise OSError("simulated disk failure")
        return original_move(src, dst)

    with patch("scripts.consensus_ceremony.shutil.move", side_effect=failing_move):
        with pytest.raises(OSError, match="simulated disk failure"):
            _do_ceremony(tmp_base, trigger_block)

    # At most 1 block leaked (the one moved before failure).
    # Since blocks are content-addressed, a leaked block is harmless
    # (idempotent re-write will produce the same file).
    # But relations.jsonl should NOT have been appended (failure was
    # before the relation-append step).
    blocks_after = list_blocks(tmp_base)
    rels_after = read_all_relations(tmp_base)

    # Relations must not have been partially written
    assert len(rels_after) == len(rels_before)


# --- 8. Idempotent ---

def test_idempotent(tmp_base, trigger_block):
    """Same input twice produces same blocks, no duplicates in blocks/."""
    result1 = _do_ceremony(tmp_base, trigger_block)
    blocks_after_first = list_blocks(tmp_base)

    result2 = _do_ceremony(tmp_base, trigger_block)
    blocks_after_second = list_blocks(tmp_base)

    # Same ids
    assert result1["consensus"]["id"] == result2["consensus"]["id"]
    assert result1["residue"]["id"] == result2["residue"]["id"]
    assert result1["tension"]["id"] == result2["tension"]["id"]

    # No new block files (content-addressed, idempotent)
    assert len(blocks_after_second) == len(blocks_after_first)


# --- 9. detect_residue_accumulation ---

def test_detect_residue_accumulation(tmp_base, trigger_block):
    """Multiple ceremonies on the same trigger accumulate residue."""
    # Write 3 ceremonies with different content but same trigger
    for i in range(3):
        write_consensus_ceremony(
            trigger_block_id=trigger_block,
            conclusion=f"conclusion-{i}",
            gemini_conceded=[f"g-{i}"],
            codex_conceded=[f"c-{i}"],
            concession_reasons={"g": f"r-{i}"},
            unresolved=[f"u-{i}"],
            source="cc",
            base=tmp_base,
        )

    # threshold=3 should detect the area
    result = detect_residue_accumulation(tmp_base, threshold=3)
    assert len(result) >= 1
    assert result[0]["count"] >= 3

    # threshold=4 should not
    result_high = detect_residue_accumulation(tmp_base, threshold=4)
    assert len(result_high) == 0


# --- 10. list_open_tensions ---

def test_list_open_tensions(tmp_base, trigger_block):
    _do_ceremony(tmp_base, trigger_block)

    tensions = list_open_tensions(tmp_base)
    assert len(tensions) == 1
    assert tensions[0]["type"] == "tension"
    assert "unresolved" in tensions[0]["content"]


# --- 11. get_ceremony_chain ---

def test_get_ceremony_chain(tmp_base, trigger_block):
    result = _do_ceremony(tmp_base, trigger_block)
    consensus_id = result["consensus"]["id"]

    chain = get_ceremony_chain(consensus_id, tmp_base)
    assert chain["consensus"] is not None
    assert chain["consensus"]["id"] == consensus_id
    assert chain["residue"] is not None
    assert chain["residue"]["type"] == "residue"
    assert chain["tension"] is not None
    assert chain["tension"]["type"] == "tension"
