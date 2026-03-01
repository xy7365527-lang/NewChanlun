"""Tests for scripts/block_topology.py — 区块拓扑核心函数"""

from __future__ import annotations

import json
import time

import pytest

from scripts.block_topology import (
    append_relation,
    compute_block_id,
    invalidate_relation,
    list_blocks,
    make_block,
    make_relation,
    query_relations,
    read_all_relations,
    read_block,
    write_block,
    write_block_with_relations,
)

# A valid SHA256 hex digest for use wherever created_by is needed
VALID_SHA = "a" * 64
VALID_SHA_B = "b" * 64
VALID_SHA_C = "c" * 64


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


# --- 1. compute_block_id: deterministic ---

def test_compute_block_id_deterministic():
    """Same inputs produce same hash, always."""
    id1 = compute_block_id("event", "cc", {"k": "v"}, [])
    id2 = compute_block_id("event", "cc", {"k": "v"}, [])
    assert id1 == id2
    assert len(id1) == 64  # SHA256 hex


# --- 2. compute_block_id: timestamp excluded ---

def test_compute_block_id_timestamp_excluded():
    """Calls at different times produce the same id (timestamp not in hash)."""
    b1 = make_block("event", "cc", {"data": 1}, [])
    time.sleep(0.01)
    b2 = make_block("event", "cc", {"data": 1}, [])
    assert b1["id"] == b2["id"]


# --- 3. write_block creates file ---

def test_write_block_creates_file(tmp_base):
    """write_block creates a JSON file named by block id."""
    block = make_block("event", "cc", {"test": True}, [])
    path = write_block(block, tmp_base)
    assert path.exists()
    assert path.name == f"{block['id']}.json"

    data = json.loads(path.read_text(encoding="utf-8"))
    assert data["id"] == block["id"]
    assert data["type"] == "event"
    assert data["content"]["test"] is True


# --- 4. write_block idempotent ---

def test_write_block_idempotent(tmp_base):
    """Writing the same block twice does not error or change file content."""
    block = make_block("event", "cc", {"test": True}, [])
    path1 = write_block(block, tmp_base)
    content_before = path1.read_text(encoding="utf-8")

    path2 = write_block(block, tmp_base)
    content_after = path2.read_text(encoding="utf-8")

    assert path1 == path2
    assert content_before == content_after

    # Only one file exists
    files = list((tmp_base / "blocks").iterdir())
    assert len(files) == 1


# --- 5. append_relation ---

def test_append_relation(tmp_base):
    """append_relation writes a line to relations.jsonl."""
    rel = make_relation("aaa", "bbb", "depends_on", 1, VALID_SHA)
    append_relation(rel, tmp_base)

    jsonl = (tmp_base / "relations.jsonl").read_text(encoding="utf-8")
    lines = [line for line in jsonl.splitlines() if line.strip()]
    assert len(lines) == 1
    parsed = json.loads(lines[0])
    assert parsed["from"] == "aaa"
    assert parsed["to"] == "bbb"
    assert parsed["relation"] == "depends_on"
    assert parsed["order"] == 1


# --- 6. write_block_with_relations creates rewrite ---

def test_write_block_with_relations_creates_rewrite(tmp_base):
    """When order=1 relations are present, a rewrite block is created."""
    rels = [
        make_relation("placeholder", "target", "depends_on", 1, VALID_SHA),
    ]

    primary = write_block_with_relations(
        "event", "cc", {"test": "data"}, [], rels,
        base=tmp_base,
    )

    # Primary block exists
    assert read_block(primary["id"], tmp_base) is not None

    # Rewrite block should exist
    rewrite_blocks = list_blocks(tmp_base, block_type="rewrite")
    assert len(rewrite_blocks) == 1
    assert rewrite_blocks[0]["content"]["primary_block"] == primary["id"]


# --- 7. rewrite creates order-2 relation ---

def test_rewrite_creates_order_2_relation(tmp_base):
    """The rewrite block creates an order-2 'records' relation."""
    rels = [
        make_relation("placeholder", "target", "depends_on", 1, VALID_SHA),
    ]

    primary = write_block_with_relations(
        "event", "cc", {"test": "data"}, [], rels,
        base=tmp_base,
    )

    all_rels = read_all_relations(tmp_base)
    order_2 = [r for r in all_rels if r["order"] == 2]
    assert len(order_2) == 1
    assert order_2[0]["relation"] == "records"
    assert order_2[0]["to"] == primary["id"]


# --- 8. order=2 does NOT trigger further rewrite ---

def test_order_2_no_further_rewrite(tmp_base):
    """Order-2 relations do NOT trigger additional rewrite blocks.
    This verifies the termination condition: recursion stops after one step.
    """
    rels = [
        make_relation("rw_id", "target", "records", 2, VALID_SHA),
    ]

    primary = write_block_with_relations(
        "rewrite", "cc", {"action": "test"}, [], rels,
        base=tmp_base,
    )

    # Only the primary block should exist, no additional rewrite
    all_blocks = list_blocks(tmp_base)
    assert len(all_blocks) == 1
    assert all_blocks[0]["type"] == "rewrite"


# --- 9. read_block ---

def test_read_block(tmp_base):
    """read_block returns the block dict after write."""
    block = make_block("event", "migration", {"id": "001"}, [])
    write_block(block, tmp_base)

    result = read_block(block["id"], tmp_base)
    assert result is not None
    assert result["id"] == block["id"]
    assert result["content"]["id"] == "001"


# --- 10. query_relations ---

def test_query_relations(tmp_base):
    """query_relations filters by block_id, relation, and order."""
    r1 = make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA_C)
    r2 = make_relation(VALID_SHA_B, VALID_SHA_C, "negates", 1, VALID_SHA)
    r3 = make_relation(VALID_SHA, VALID_SHA_C, "depends_on", 2, VALID_SHA_B)
    for r in [r1, r2, r3]:
        append_relation(r, tmp_base)

    # Filter by block_id (matches from OR to)
    by_a = query_relations(tmp_base, block_id=VALID_SHA)
    assert len(by_a) == 2  # r1 (from=a), r3 (from=a)

    # Filter by relation type
    by_neg = query_relations(tmp_base, relation="negates")
    assert len(by_neg) == 1
    assert by_neg[0]["from"] == VALID_SHA_B

    # Filter by order
    by_ord2 = query_relations(tmp_base, order=2)
    assert len(by_ord2) == 1
    assert by_ord2[0]["relation"] == "depends_on"
    assert by_ord2[0]["from"] == VALID_SHA

    # Combined filter
    combined = query_relations(
        tmp_base, block_id=VALID_SHA, relation="depends_on", order=1,
    )
    assert len(combined) == 1
    assert combined[0]["to"] == VALID_SHA_B


# --- 11. make_relation validates created_by ---

def test_make_relation_validates_created_by():
    """created_by that is not a SHA256 hex digest raises ValueError."""
    with pytest.raises(ValueError, match="created_by"):
        make_relation(
            from_id="x" * 64,
            to_id="y" * 64,
            relation="depends_on",
            order=1,
            created_by="not-a-sha256",
        )

    # Valid SHA256 is accepted without error
    rel = make_relation(
        from_id="x" * 64,
        to_id="y" * 64,
        relation="depends_on",
        order=1,
        created_by=VALID_SHA,
    )
    assert rel["created_by"] == VALID_SHA


# --- 12. negates relation has default validity="active" (273号) ---

def test_negates_default_validity():
    """New negates relations have validity='active' by default."""
    rel = make_relation(VALID_SHA, VALID_SHA_B, "negates", 1, VALID_SHA_C)
    assert rel["validity"] == "active"
    assert rel["invalidated_by"] is None
    assert rel["invalidated_at"] is None


# --- 13. non-negates relations do NOT have validity fields ---

def test_non_negates_no_validity():
    """Non-negates relations should not have validity fields."""
    rel = make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA_C)
    assert "validity" not in rel
    assert "invalidated_by" not in rel
    assert "invalidated_at" not in rel


# --- 14. invalidate_relation sets fields correctly (273号) ---

def test_invalidate_relation():
    """invalidate_relation marks a negates edge as invalidated."""
    rel = make_relation(VALID_SHA, VALID_SHA_B, "negates", 1, VALID_SHA_C)
    assert rel["validity"] == "active"

    invalidator_id = "d" * 64
    result = invalidate_relation(rel, invalidator_id)

    # Original is not mutated
    assert rel["validity"] == "active"

    # Result has correct fields
    assert result["validity"] == "invalidated"
    assert result["invalidated_by"] == invalidator_id
    assert result["invalidated_at"] is not None
    # Other fields preserved
    assert result["from"] == VALID_SHA
    assert result["to"] == VALID_SHA_B
    assert result["relation"] == "negates"
    assert result["order"] == 1


# --- 15. invalidate_relation is idempotent (273号边界条件3) ---

def test_invalidate_relation_idempotent():
    """Already invalidated relations are returned unchanged (idempotent)."""
    rel = make_relation(VALID_SHA, VALID_SHA_B, "negates", 1, VALID_SHA_C)
    invalidator_1 = "d" * 64
    invalidator_2 = "e" * 64

    result_1 = invalidate_relation(rel, invalidator_1)
    result_2 = invalidate_relation(result_1, invalidator_2)

    # Second invalidation does not change the record
    assert result_2["validity"] == "invalidated"
    assert result_2["invalidated_by"] == invalidator_1  # first invalidator kept
    assert result_2["invalidated_at"] == result_1["invalidated_at"]


# --- 16. invalidate_relation rejects non-negates ---

def test_invalidate_relation_rejects_non_negates():
    """invalidate_relation raises ValueError for non-negates relations."""
    rel = make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA_C)
    with pytest.raises(ValueError, match="negates"):
        invalidate_relation(rel, "d" * 64)


# --- 17. backward compatibility: old negates without validity (273号) ---

def test_backward_compat_old_negates_read(tmp_base):
    """Old-format negates relations (no validity) get default active on read."""
    # Write a negates relation in old format (no validity fields) directly
    old_format = {
        "from": VALID_SHA,
        "to": VALID_SHA_B,
        "relation": "negates",
        "order": 1,
        "created_by": VALID_SHA_C,
        "timestamp": "2026-01-01T00:00:00+00:00",
    }
    jsonl_path = tmp_base / "relations.jsonl"
    jsonl_path.write_text(
        json.dumps(old_format, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )

    # Read back — should have validity defaults
    rels = read_all_relations(tmp_base)
    assert len(rels) == 1
    assert rels[0]["validity"] == "active"
    assert rels[0]["invalidated_by"] is None
    assert rels[0]["invalidated_at"] is None

    # query_relations uses read_all_relations, so also normalized
    queried = query_relations(tmp_base, relation="negates")
    assert len(queried) == 1
    assert queried[0]["validity"] == "active"


# --- 18. backward compat: non-negates old relations unaffected ---

def test_backward_compat_non_negates_unchanged(tmp_base):
    """Non-negates relations are not modified by normalization."""
    old_format = {
        "from": VALID_SHA,
        "to": VALID_SHA_B,
        "relation": "depends_on",
        "order": 1,
        "created_by": VALID_SHA_C,
        "timestamp": "2026-01-01T00:00:00+00:00",
    }
    jsonl_path = tmp_base / "relations.jsonl"
    jsonl_path.write_text(
        json.dumps(old_format, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )

    rels = read_all_relations(tmp_base)
    assert len(rels) == 1
    assert "validity" not in rels[0]


# --- 19. mixed old/new format relations coexist ---

def test_mixed_old_new_format(tmp_base):
    """Old-format and new-format negates relations coexist correctly."""
    old_negates = json.dumps({
        "from": VALID_SHA, "to": VALID_SHA_B, "relation": "negates",
        "order": 1, "created_by": VALID_SHA_C,
        "timestamp": "2026-01-01T00:00:00+00:00",
    }, separators=(",", ":"))
    new_negates = json.dumps({
        "from": VALID_SHA_B, "to": VALID_SHA_C, "relation": "negates",
        "order": 1, "created_by": VALID_SHA,
        "timestamp": "2026-02-01T00:00:00+00:00",
        "validity": "invalidated",
        "invalidated_by": "d" * 64,
        "invalidated_at": "2026-02-15T00:00:00+00:00",
    }, separators=(",", ":"))
    depends = json.dumps({
        "from": VALID_SHA, "to": VALID_SHA_C, "relation": "depends_on",
        "order": 1, "created_by": VALID_SHA_B,
        "timestamp": "2026-01-15T00:00:00+00:00",
    }, separators=(",", ":"))

    jsonl_path = tmp_base / "relations.jsonl"
    jsonl_path.write_text(
        old_negates + "\n" + new_negates + "\n" + depends + "\n",
        encoding="utf-8",
    )

    rels = read_all_relations(tmp_base)
    assert len(rels) == 3

    # Old negates normalized to active
    assert rels[0]["validity"] == "active"
    assert rels[0]["invalidated_by"] is None

    # New negates keeps its invalidated state
    assert rels[1]["validity"] == "invalidated"
    assert rels[1]["invalidated_by"] == "d" * 64

    # depends_on unchanged
    assert "validity" not in rels[2]


# --- 20. make_relation rejects invalid validity ---

def test_make_relation_rejects_invalid_validity():
    """make_relation raises ValueError for invalid validity values."""
    with pytest.raises(ValueError, match="validity"):
        make_relation(
            VALID_SHA, VALID_SHA_B, "negates", 1, VALID_SHA_C,
            validity="bogus",
        )
