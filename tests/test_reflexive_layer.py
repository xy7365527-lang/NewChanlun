"""Tests for BT2 Reflexive Layer — scripts/reflexive_layer.py

Covers:
  1. diff_relations: detect newly appended relations
  2. needs_rewrite: order-1 filtering
  3. create_rewrite_block: rewrite block generation
  4. create_records_relation: order-2 records relation
  5. apply_reflexive_layer: full pipeline
  6. verify_reflexive_closure: recursive closure verification
  7. created_by recursive reference: rewrite id usable as created_by
  8. order-2 does NOT trigger rewrite (recursion stops at one step)
"""
from __future__ import annotations

import json

import pytest

from scripts.block_topology import (
    make_block,
    make_relation,
    read_all_relations,
    read_block,
    write_block,
)
from scripts.reflexive_layer import (
    apply_reflexive_layer,
    create_records_relation,
    create_rewrite_block,
    diff_relations,
    needs_rewrite,
    verify_reflexive_closure,
)

VALID_SHA = "a" * 64
VALID_SHA_B = "b" * 64


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


# --- 1. diff_relations ---

class TestDiffRelations:
    def test_empty_old_returns_all_new(self):
        new = [{"from": "a", "to": "b", "relation": "depends_on"}]
        assert diff_relations([], new) == new

    def test_identical_returns_empty(self):
        rels = [{"from": "a", "to": "b", "relation": "depends_on"}]
        assert diff_relations(rels, rels) == []

    def test_detects_appended_relations(self):
        old = [{"from": "a", "to": "b", "relation": "depends_on"}]
        new_rel = {"from": "c", "to": "d", "relation": "negates"}
        new = old + [new_rel]
        result = diff_relations(old, new)
        assert len(result) == 1
        assert result[0] == new_rel

    def test_preserves_order(self):
        old = [{"from": "a", "to": "b"}]
        new = old + [{"from": "c", "to": "d"}, {"from": "e", "to": "f"}]
        result = diff_relations(old, new)
        assert len(result) == 2
        assert result[0]["from"] == "c"
        assert result[1]["from"] == "e"

    def test_key_order_invariant(self):
        """Different key ordering in dicts should still match."""
        old = [{"from": "a", "to": "b", "x": 1}]
        new = [{"to": "b", "x": 1, "from": "a"}]
        assert diff_relations(old, new) == []


# --- 2. needs_rewrite ---

class TestNeedsRewrite:
    def test_order_1_triggers(self):
        assert needs_rewrite([{"order": 1}]) is True

    def test_order_2_does_not_trigger(self):
        assert needs_rewrite([{"order": 2}]) is False

    def test_empty_does_not_trigger(self):
        assert needs_rewrite([]) is False

    def test_mixed_orders(self):
        assert needs_rewrite([{"order": 2}, {"order": 1}]) is True


# --- 3. create_rewrite_block ---

class TestCreateRewriteBlock:
    def test_creates_block_on_disk(self, tmp_base):
        primary = make_block("event", "cc", {"test": 1}, [])
        write_block(primary, tmp_base)
        order1_rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)
        ]

        rewrite = create_rewrite_block(
            "cc", primary["id"], order1_rels, tmp_base
        )

        assert rewrite["type"] == "rewrite"
        assert rewrite["content"]["primary_block"] == primary["id"]
        assert rewrite["content"]["relation_count"] == 1
        assert "depends_on" in rewrite["content"]["relation_types"]
        assert rewrite["refs"] == [primary["id"]]

        # Verify on disk
        on_disk = read_block(rewrite["id"], tmp_base)
        assert on_disk is not None
        assert on_disk["id"] == rewrite["id"]

    def test_idempotent(self, tmp_base):
        primary = make_block("event", "cc", {"test": 1}, [])
        write_block(primary, tmp_base)
        rels = [make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)]

        rw1 = create_rewrite_block("cc", primary["id"], rels, tmp_base)
        rw2 = create_rewrite_block("cc", primary["id"], rels, tmp_base)
        assert rw1["id"] == rw2["id"]


# --- 4. create_records_relation ---

class TestCreateRecordsRelation:
    def test_creates_order2_records_relation(self, tmp_base):
        rewrite_id = VALID_SHA
        primary_id = VALID_SHA_B

        rel = create_records_relation(rewrite_id, primary_id, tmp_base)

        assert rel["from"] == rewrite_id
        assert rel["to"] == primary_id
        assert rel["relation"] == "records"
        assert rel["order"] == 2
        assert rel["created_by"] == rewrite_id

    def test_appended_to_jsonl(self, tmp_base):
        create_records_relation(VALID_SHA, VALID_SHA_B, tmp_base)

        relations = read_all_relations(tmp_base)
        assert len(relations) == 1
        assert relations[0]["relation"] == "records"


# --- 5. apply_reflexive_layer ---

class TestApplyReflexiveLayer:
    def test_order1_triggers_rewrite(self, tmp_base):
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)
        rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)
        ]

        rewrite = apply_reflexive_layer("cc", primary["id"], rels, tmp_base)

        assert rewrite is not None
        assert rewrite["type"] == "rewrite"
        # Records relation should exist
        all_rels = read_all_relations(tmp_base)
        records = [r for r in all_rels if r["relation"] == "records"]
        assert len(records) == 1
        assert records[0]["from"] == rewrite["id"]
        assert records[0]["to"] == primary["id"]

    def test_order2_only_returns_none(self, tmp_base):
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)
        rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "records", 2, VALID_SHA)
        ]

        result = apply_reflexive_layer("cc", primary["id"], rels, tmp_base)
        assert result is None

    def test_mixed_orders_only_counts_order1(self, tmp_base):
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)
        rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA),
            make_relation(VALID_SHA, VALID_SHA_B, "records", 2, VALID_SHA),
        ]

        rewrite = apply_reflexive_layer("cc", primary["id"], rels, tmp_base)
        assert rewrite is not None
        assert rewrite["content"]["relation_count"] == 1

    def test_empty_relations_returns_none(self, tmp_base):
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)

        result = apply_reflexive_layer("cc", primary["id"], [], tmp_base)
        assert result is None


# --- 6. verify_reflexive_closure ---

class TestVerifyReflexiveClosure:
    def test_empty_topology_is_ok(self, tmp_base):
        report = verify_reflexive_closure(tmp_base)
        assert report["ok"] is True
        assert report["rewrite_blocks"] == 0
        assert report["records_relations"] == 0
        assert report["errors"] == []

    def test_valid_closure(self, tmp_base):
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)
        rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)
        ]
        apply_reflexive_layer("cc", primary["id"], rels, tmp_base)

        report = verify_reflexive_closure(tmp_base)
        assert report["ok"] is True
        assert report["rewrite_blocks"] == 1
        assert report["records_relations"] == 1

    def test_orphan_rewrite_block_detected(self, tmp_base):
        """Rewrite block without records relation is an error."""
        rewrite = make_block(
            "rewrite", "cc",
            {"action": "relation_append", "primary_block": VALID_SHA,
             "relation_count": 1},
            refs=[VALID_SHA],
        )
        write_block(rewrite, tmp_base)

        report = verify_reflexive_closure(tmp_base)
        assert report["ok"] is False
        assert len(report["errors"]) == 1
        assert "no 'records' relation" in report["errors"][0]

    def test_dangling_records_relation_detected(self, tmp_base):
        """Records relation pointing to non-existent rewrite block."""
        # Write a records relation but no rewrite block
        rel = make_relation(
            VALID_SHA, VALID_SHA_B, "records", 2, VALID_SHA
        )
        from scripts.block_topology import append_relation
        append_relation(rel, tmp_base)

        report = verify_reflexive_closure(tmp_base)
        assert report["ok"] is False
        assert len(report["errors"]) == 1
        assert "no rewrite block found" in report["errors"][0]


# --- 7. created_by recursive reference ---

class TestCreatedByRecursion:
    def test_rewrite_id_usable_as_created_by(self, tmp_base):
        """The rewrite block id can be used as created_by in subsequent
        relations — this is the recursive closure mechanism."""
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)
        rels = [
            make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)
        ]

        rewrite = apply_reflexive_layer("cc", primary["id"], rels, tmp_base)

        # Use rewrite id as created_by for a subsequent relation
        subsequent_rel = make_relation(
            VALID_SHA, VALID_SHA_B, "depends_on", 1,
            created_by=rewrite["id"],
        )
        assert subsequent_rel["created_by"] == rewrite["id"]

        # The block exists — reference is valid
        assert read_block(rewrite["id"], tmp_base) is not None

    def test_chain_of_rewrites(self, tmp_base):
        """Two successive reflexive operations: second rewrite references first."""
        # First operation
        p1 = make_block("event", "cc", {"data": "first"}, [])
        write_block(p1, tmp_base)
        rw1 = apply_reflexive_layer(
            "cc", p1["id"],
            [make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)],
            tmp_base,
        )

        # Second operation — created_by = first rewrite's id
        p2 = make_block("event", "cc", {"data": "second"}, [])
        write_block(p2, tmp_base)
        rw2 = apply_reflexive_layer(
            "cc", p2["id"],
            [make_relation(
                VALID_SHA, VALID_SHA_B, "negates", 1,
                created_by=rw1["id"],
                validity="active",
            )],
            tmp_base,
        )

        # Both rewrites exist
        assert read_block(rw1["id"], tmp_base) is not None
        assert read_block(rw2["id"], tmp_base) is not None
        assert rw1["id"] != rw2["id"]

        # Verification passes
        report = verify_reflexive_closure(tmp_base)
        assert report["ok"] is True
        assert report["rewrite_blocks"] == 2
        assert report["records_relations"] == 2


# --- 8. order-2 does NOT trigger rewrite ---

class TestRecursionBound:
    def test_records_relation_is_order2(self, tmp_base):
        """The records relation generated by reflexive layer is order 2,
        so it does NOT trigger another rewrite. Recursion = exactly one step."""
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)

        rewrite = apply_reflexive_layer(
            "cc", primary["id"],
            [make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)],
            tmp_base,
        )

        all_rels = read_all_relations(tmp_base)
        records = [r for r in all_rels if r["relation"] == "records"]
        assert len(records) == 1
        assert records[0]["order"] == 2

        # Feeding the records relation back should NOT produce another rewrite
        result = apply_reflexive_layer("cc", rewrite["id"], records, tmp_base)
        assert result is None

    def test_no_infinite_recursion(self, tmp_base):
        """Ensure reflexive layer does not produce cascading rewrites."""
        primary = make_block("event", "cc", {"data": "x"}, [])
        write_block(primary, tmp_base)

        # Apply reflexive layer
        rewrite = apply_reflexive_layer(
            "cc", primary["id"],
            [make_relation(VALID_SHA, VALID_SHA_B, "depends_on", 1, VALID_SHA)],
            tmp_base,
        )

        # Count rewrite blocks — should be exactly 1
        blocks_dir = tmp_base / "blocks"
        rewrite_count = 0
        for f in blocks_dir.iterdir():
            if f.suffix == ".json":
                blk = json.loads(f.read_text(encoding="utf-8"))
                if blk.get("type") == "rewrite":
                    rewrite_count += 1

        assert rewrite_count == 1
