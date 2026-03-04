"""Tests for scripts/migrate_to_content_addressed.py — Migration to Content-Addressed Storage."""
from __future__ import annotations

import hashlib
import json

import pytest

from scripts.block_ops import compute_content_hash
from scripts.migrate_to_content_addressed import (
    _is_already_content_addressed,
    execute_migration,
    plan_migration,
)


@pytest.fixture
def mock_repo(tmp_path):
    """Create a minimal repository structure for migration testing."""
    bt = tmp_path / ".chanlun" / "block-topology"
    blocks = bt / "blocks"
    blocks.mkdir(parents=True)

    gen = tmp_path / ".chanlun" / "genealogy"
    settled = gen / "settled"
    settled.mkdir(parents=True)

    # Create meta.json
    meta = {
        "version": "1.0.0",
        "genesis_block_id": "genesis_hash",
        "block_count": 0,
        "relation_count": 0,
        "id_mapping": {},
    }
    (bt / "meta.json").write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    # Create empty relations.jsonl
    (bt / "relations.jsonl").write_text("", encoding="utf-8")

    return tmp_path


def _write_md(root, num, title, text):
    """Helper: write a genealogy .md file."""
    settled = root / ".chanlun" / "genealogy" / "settled"
    fname = f"{num:03d}-{title}.md"
    fpath = settled / fname
    fpath.write_text(text, encoding="utf-8")
    return fpath


def _write_legacy_block(root, block_id, genealogy_num, title, source_file):
    """Helper: write a legacy-format block (no full_text)."""
    blocks = root / ".chanlun" / "block-topology" / "blocks"
    block = {
        "id": block_id,
        "type": "event",
        "timestamp": "2026-03-01T00:00:00+00:00",
        "source": "migration",
        "content": {
            "id": str(genealogy_num),
            "title": title,
            "source_file": source_file,
        },
        "refs": [],
    }
    (blocks / f"{block_id}.json").write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return block


def _write_content_addressed_block(root, text, extra_fields=None):
    """Helper: write a content-addressed block (has full_text, id=SHA256(text))."""
    blocks = root / ".chanlun" / "block-topology" / "blocks"
    block_id = compute_content_hash(text)
    block = {
        "id": block_id,
        "timestamp": "2026-03-01T00:00:00+00:00",
        "content": {
            "full_text": text,
        },
    }
    if extra_fields:
        block.update(extra_fields)
    (blocks / f"{block_id}.json").write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return block_id


def _update_meta(root, mapping):
    """Helper: update meta.json id_mapping."""
    meta_path = root / ".chanlun" / "block-topology" / "meta.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    meta["id_mapping"].update(mapping)
    meta["block_count"] = len(meta["id_mapping"])
    meta_path.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def _write_relations(root, relations):
    """Helper: write relations.jsonl."""
    path = root / ".chanlun" / "block-topology" / "relations.jsonl"
    lines = [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in relations]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# --- _is_already_content_addressed ---

def test_is_content_addressed_true():
    text = "# Test"
    h = compute_content_hash(text)
    block = {"id": h, "content": {"full_text": text}}
    assert _is_already_content_addressed(block) is True


def test_is_content_addressed_false_no_full_text():
    block = {"id": "abc", "content": {"source_file": "x.md"}}
    assert _is_already_content_addressed(block) is False


def test_is_content_addressed_false_wrong_hash():
    block = {"id": "wrong_hash", "content": {"full_text": "# Test"}}
    assert _is_already_content_addressed(block) is False


def test_is_content_addressed_false_non_dict_content():
    block = {"id": "x", "content": "string"}
    assert _is_already_content_addressed(block) is False


# --- plan_migration ---

def test_plan_migration_legacy_block(mock_repo):
    text = "# Legacy block"
    _write_md(mock_repo, 1, "legacy", text)
    _write_legacy_block(mock_repo, "a" * 64, 1, "legacy", "settled/001-legacy.md")

    report = plan_migration(mock_repo)
    assert report["total"] == 1
    assert report["to_migrate"] == 1
    assert report["already_migrated"] == 0

    detail = report["details"][0]
    assert detail["action"] == "migrate"
    assert detail["old_id"] == "a" * 64
    assert detail["new_id"] == compute_content_hash(text)
    assert detail["id_changed"] is True


def test_plan_migration_already_migrated(mock_repo):
    text = "# Already content-addressed"
    _write_content_addressed_block(mock_repo, text)

    report = plan_migration(mock_repo)
    assert report["total"] == 1
    assert report["to_migrate"] == 0
    assert report["already_migrated"] == 1


def test_plan_migration_no_source(mock_repo):
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    block = {
        "id": "b" * 64,
        "type": "rewrite",
        "content": {"action": "relation_append"},
    }
    (blocks / f"{'b' * 64}.json").write_text(
        json.dumps(block), encoding="utf-8"
    )

    report = plan_migration(mock_repo)
    assert report["no_source"] == 1


def test_plan_migration_missing_file(mock_repo):
    _write_legacy_block(mock_repo, "c" * 64, 999, "missing", "settled/999-missing.md")

    report = plan_migration(mock_repo)
    assert report["missing_file"] == 1


def test_plan_migration_mixed(mock_repo):
    # 1 legacy + 1 already migrated + 1 no-source
    text1 = "# Legacy"
    _write_md(mock_repo, 1, "legacy", text1)
    _write_legacy_block(mock_repo, "d" * 64, 1, "legacy", "settled/001-legacy.md")

    _write_content_addressed_block(mock_repo, "# Already migrated")

    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    no_source = {"id": "e" * 64, "type": "rewrite", "content": {"action": "x"}}
    (blocks / f"{'e' * 64}.json").write_text(
        json.dumps(no_source), encoding="utf-8"
    )

    report = plan_migration(mock_repo)
    assert report["total"] == 3
    assert report["to_migrate"] == 1
    assert report["already_migrated"] == 1
    assert report["no_source"] == 1


# --- execute_migration ---

def test_execute_migration_basic(mock_repo):
    text = "# Migrate me"
    _write_md(mock_repo, 1, "migrate", text)
    old_id = "f" * 64
    _write_legacy_block(mock_repo, old_id, 1, "migrate", "settled/001-migrate.md")
    _update_meta(mock_repo, {"001": old_id})

    result = execute_migration(mock_repo)
    assert result["migrated"] == 1
    assert result["id_remapped"] == 1
    assert result["meta_updated"] is True

    # Old block file should be gone, new one should exist
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    assert not (blocks / f"{old_id}.json").is_file()

    new_id = compute_content_hash(text)
    assert (blocks / f"{new_id}.json").is_file()

    # Check new block has full_text
    new_block = json.loads(
        (blocks / f"{new_id}.json").read_text(encoding="utf-8")
    )
    assert new_block["content"]["full_text"] == text
    assert new_block["id"] == new_id
    assert "content_hash" not in new_block  # Legacy field removed


def test_execute_migration_updates_meta(mock_repo):
    text = "# Meta update test"
    _write_md(mock_repo, 5, "meta", text)
    old_id = "a1" + "0" * 62
    _write_legacy_block(mock_repo, old_id, 5, "meta", "settled/005-meta.md")
    _update_meta(mock_repo, {"005": old_id})

    execute_migration(mock_repo)

    meta = json.loads(
        (mock_repo / ".chanlun" / "block-topology" / "meta.json")
        .read_text(encoding="utf-8")
    )
    new_id = compute_content_hash(text)
    assert meta["id_mapping"]["005"] == new_id


def test_execute_migration_updates_relations(mock_repo):
    text = "# Relations test"
    _write_md(mock_repo, 10, "rel", text)
    old_id = "b1" + "0" * 62
    _write_legacy_block(mock_repo, old_id, 10, "rel", "settled/010-rel.md")
    _update_meta(mock_repo, {"010": old_id})

    # Write a relation referencing the old ID
    _write_relations(mock_repo, [
        {
            "from": old_id,
            "to": "other_block",
            "relation": "depends_on",
            "order": 1,
            "created_by": "migration",
        },
        {
            "from": "another",
            "to": old_id,
            "relation": "related",
            "order": 2,
            "created_by": old_id,
        },
    ])

    execute_migration(mock_repo)

    new_id = compute_content_hash(text)
    rel_path = mock_repo / ".chanlun" / "block-topology" / "relations.jsonl"
    lines = [l for l in rel_path.read_text(encoding="utf-8").strip().split("\n") if l]
    assert len(lines) == 2

    rel1 = json.loads(lines[0])
    assert rel1["from"] == new_id  # Remapped
    assert rel1["to"] == "other_block"  # Unchanged

    rel2 = json.loads(lines[1])
    assert rel2["to"] == new_id  # Remapped
    assert rel2["created_by"] == new_id  # Remapped


def test_execute_migration_skips_already_migrated(mock_repo):
    text = "# Already done"
    block_id = _write_content_addressed_block(mock_repo, text)

    result = execute_migration(mock_repo)
    assert result["migrated"] == 0
    assert result["skipped_already_content_addressed"] == 1

    # Block should still exist
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    assert (blocks / f"{block_id}.json").is_file()


def test_execute_migration_idempotent(mock_repo):
    text = "# Idempotent migration"
    _write_md(mock_repo, 20, "idem", text)
    old_id = "c1" + "0" * 62
    _write_legacy_block(mock_repo, old_id, 20, "idem", "settled/020-idem.md")
    _update_meta(mock_repo, {"020": old_id})

    # Run twice
    result1 = execute_migration(mock_repo)
    result2 = execute_migration(mock_repo)

    assert result1["migrated"] == 1
    # Second run: block is now content-addressed, so it's skipped
    assert result2["skipped_already_content_addressed"] == 1
    assert result2["migrated"] == 0


def test_execute_migration_preserves_block_fields(mock_repo):
    """Migration preserves existing block fields (type, source, refs, etc.)."""
    text = "# Preserve fields"
    _write_md(mock_repo, 30, "preserve", text)
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    old_id = "d1" + "0" * 62
    block = {
        "id": old_id,
        "type": "event",
        "timestamp": "2026-03-01T00:00:00+00:00",
        "source": "migration",
        "content": {
            "id": "030",
            "title": "Preserve fields",
            "source_file": "settled/030-preserve.md",
        },
        "refs": ["some_ref"],
        "git_ref": "abc123",
    }
    (blocks / f"{old_id}.json").write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    _update_meta(mock_repo, {"030": old_id})

    execute_migration(mock_repo)

    new_id = compute_content_hash(text)
    new_block = json.loads(
        (blocks / f"{new_id}.json").read_text(encoding="utf-8")
    )
    assert new_block["type"] == "event"
    assert new_block["source"] == "migration"
    assert new_block["refs"] == ["some_ref"]
    assert new_block["git_ref"] == "abc123"
    # Content fields preserved alongside full_text
    assert new_block["content"]["full_text"] == text
    assert new_block["content"]["source_file"] == "settled/030-preserve.md"
    assert new_block["content"]["id"] == "030"


def test_execute_migration_genesis_block_remap(mock_repo):
    """If genesis_block_id is remapped, meta.json reflects the change."""
    text = "# Genesis"
    _write_md(mock_repo, 1, "genesis", text)
    genesis_id = "e1" + "0" * 62

    # Set genesis_block_id to this block's id
    meta_path = mock_repo / ".chanlun" / "block-topology" / "meta.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    meta["genesis_block_id"] = genesis_id
    meta["id_mapping"]["001"] = genesis_id
    meta_path.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    _write_legacy_block(mock_repo, genesis_id, 1, "genesis", "settled/001-genesis.md")

    execute_migration(mock_repo)

    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    new_id = compute_content_hash(text)
    assert meta["genesis_block_id"] == new_id


def test_execute_migration_no_source_blocks_unchanged(mock_repo):
    """Blocks without source files are left untouched."""
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    no_source_id = "f1" + "0" * 62
    block = {
        "id": no_source_id,
        "type": "rewrite",
        "content": {"action": "relation_append"},
    }
    (blocks / f"{no_source_id}.json").write_text(
        json.dumps(block), encoding="utf-8"
    )

    result = execute_migration(mock_repo)
    assert result["skipped_no_source"] == 1
    assert (blocks / f"{no_source_id}.json").is_file()


def test_execute_migration_removes_content_hash_field(mock_repo):
    """Legacy content_hash field is removed during migration."""
    text = "# Remove content_hash"
    _write_md(mock_repo, 40, "rm-hash", text)
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    old_id = "g1" + "0" * 62
    block = {
        "id": old_id,
        "type": "event",
        "content_hash": old_id,
        "content": {
            "id": "040",
            "source_file": "settled/040-rm-hash.md",
        },
    }
    (blocks / f"{old_id}.json").write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    execute_migration(mock_repo)

    new_id = compute_content_hash(text)
    new_block = json.loads(
        (blocks / f"{new_id}.json").read_text(encoding="utf-8")
    )
    assert "content_hash" not in new_block
