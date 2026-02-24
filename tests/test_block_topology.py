"""Tests for scripts/block_topology.py — 区块拓扑核心函数"""

from __future__ import annotations

import json
import time

import pytest

from scripts.block_topology import (
    append_relation,
    compute_block_id,
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
