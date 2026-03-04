"""Tests for scripts/block_ops.py — Content-Addressed Block Operations."""
from __future__ import annotations

import hashlib
import json

import pytest

from scripts.block_ops import (
    compute_content_hash,
    create_block,
    create_revision,
    get_block_content,
    rebuild_working_copy,
    verify_working_copy,
)


@pytest.fixture
def blocks_dir(tmp_path):
    d = tmp_path / "blocks"
    d.mkdir()
    return d


@pytest.fixture
def genealogy_dir(tmp_path):
    d = tmp_path / "genealogy" / "settled"
    d.mkdir(parents=True)
    return d


def _write_md(genealogy_dir, name, text):
    """Helper: write a .md file and return (path, expected_hash)."""
    path = genealogy_dir / name
    path.write_text(text, encoding="utf-8")
    h = hashlib.sha256(text.encode("utf-8")).hexdigest()
    return path, h


# --- compute_content_hash ---

def test_compute_content_hash_deterministic():
    h1 = compute_content_hash("hello world")
    h2 = compute_content_hash("hello world")
    assert h1 == h2
    assert len(h1) == 64


def test_compute_content_hash_different_inputs():
    h1 = compute_content_hash("hello")
    h2 = compute_content_hash("world")
    assert h1 != h2


def test_compute_content_hash_matches_hashlib():
    text = "# Test content\n\nSome body text."
    expected = hashlib.sha256(text.encode("utf-8")).hexdigest()
    assert compute_content_hash(text) == expected


# --- create_block ---

def test_create_block_basic(genealogy_dir, blocks_dir):
    text = "# Block content\n\nBody."
    md_path, expected_hash = _write_md(genealogy_dir, "001-test.md", text)

    block_id = create_block(md_path, blocks_dir)
    assert block_id == expected_hash

    block_path = blocks_dir / f"{block_id}.json"
    assert block_path.is_file()

    block = json.loads(block_path.read_text(encoding="utf-8"))
    assert block["id"] == expected_hash
    assert block["content"]["full_text"] == text


def test_create_block_with_metadata(genealogy_dir, blocks_dir):
    text = "# Metadata test"
    md_path, _ = _write_md(genealogy_dir, "002-meta.md", text)

    block_id = create_block(md_path, blocks_dir, metadata={
        "genealogy_id": "002",
        "title": "Metadata test",
        "type": "event",
        "depends_on": ["001"],
    })

    block = json.loads(
        (blocks_dir / f"{block_id}.json").read_text(encoding="utf-8")
    )
    assert block["genealogy_id"] == "002"
    assert block["title"] == "Metadata test"
    assert block["type"] == "event"
    assert block["depends_on"] == ["001"]


def test_create_block_idempotent(genealogy_dir, blocks_dir):
    text = "# Idempotent"
    md_path, _ = _write_md(genealogy_dir, "003-idem.md", text)

    id1 = create_block(md_path, blocks_dir)
    id2 = create_block(md_path, blocks_dir)
    assert id1 == id2

    # Only one file
    files = list(blocks_dir.glob("*.json"))
    assert len(files) == 1


def test_create_block_file_not_found(blocks_dir, tmp_path):
    fake = tmp_path / "nonexistent.md"
    with pytest.raises(FileNotFoundError):
        create_block(fake, blocks_dir)


def test_create_block_empty_content(genealogy_dir, blocks_dir):
    md_path = genealogy_dir / "empty.md"
    md_path.write_text("   \n  ", encoding="utf-8")

    with pytest.raises(ValueError, match="Empty content"):
        create_block(md_path, blocks_dir)


def test_create_block_metadata_cannot_overwrite_id(genealogy_dir, blocks_dir):
    text = "# No override"
    md_path, expected_hash = _write_md(genealogy_dir, "004-no-override.md", text)

    block_id = create_block(md_path, blocks_dir, metadata={"id": "hacked"})
    assert block_id == expected_hash  # id is NOT overridden


def test_create_block_metadata_content_merge(genealogy_dir, blocks_dir):
    text = "# Merge test"
    md_path, _ = _write_md(genealogy_dir, "005-merge.md", text)

    create_block(md_path, blocks_dir, metadata={
        "content": {"source_file": "settled/005-merge.md", "id": "005"},
    })

    block_id = compute_content_hash(text)
    block = json.loads(
        (blocks_dir / f"{block_id}.json").read_text(encoding="utf-8")
    )
    # full_text preserved, extra fields merged
    assert block["content"]["full_text"] == text
    assert block["content"]["source_file"] == "settled/005-merge.md"
    assert block["content"]["id"] == "005"


def test_create_block_chinese_content(genealogy_dir, blocks_dir):
    text = "# 退化段\n\n这是中文内容。"
    md_path, expected_hash = _write_md(genealogy_dir, "006-chinese.md", text)

    block_id = create_block(md_path, blocks_dir)
    assert block_id == expected_hash

    block = json.loads(
        (blocks_dir / f"{block_id}.json").read_text(encoding="utf-8")
    )
    assert block["content"]["full_text"] == text


# --- get_block_content ---

def test_get_block_content(genealogy_dir, blocks_dir):
    text = "# Get content test"
    md_path, _ = _write_md(genealogy_dir, "010-get.md", text)

    block_id = create_block(md_path, blocks_dir)
    retrieved = get_block_content(block_id, blocks_dir)
    assert retrieved == text


def test_get_block_content_not_found(blocks_dir):
    with pytest.raises(FileNotFoundError):
        get_block_content("0" * 64, blocks_dir)


def test_get_block_content_no_full_text(blocks_dir):
    # Write a legacy block without full_text
    block_id = "a" * 64
    block = {"id": block_id, "content": {"source_file": "x.md"}}
    (blocks_dir / f"{block_id}.json").write_text(
        json.dumps(block), encoding="utf-8"
    )
    with pytest.raises(KeyError, match="no content.full_text"):
        get_block_content(block_id, blocks_dir)


# --- verify_working_copy ---

def test_verify_working_copy_match(genealogy_dir, blocks_dir):
    text = "# Verify test"
    md_path, _ = _write_md(genealogy_dir, "020-verify.md", text)

    create_block(md_path, blocks_dir)
    assert verify_working_copy(md_path, blocks_dir) is True


def test_verify_working_copy_tampered(genealogy_dir, blocks_dir):
    text = "# Original"
    md_path, _ = _write_md(genealogy_dir, "021-tamper.md", text)

    create_block(md_path, blocks_dir)

    # Tamper the file
    md_path.write_text("# Tampered", encoding="utf-8")
    assert verify_working_copy(md_path, blocks_dir) is False


def test_verify_working_copy_no_block(genealogy_dir, blocks_dir):
    text = "# No block exists"
    md_path, _ = _write_md(genealogy_dir, "022-no-block.md", text)

    assert verify_working_copy(md_path, blocks_dir) is False


def test_verify_working_copy_file_missing(blocks_dir, tmp_path):
    fake = tmp_path / "missing.md"
    assert verify_working_copy(fake, blocks_dir) is False


# --- rebuild_working_copy ---

def test_rebuild_working_copy(genealogy_dir, blocks_dir, tmp_path):
    text = "# Rebuild test\n\nSome content."
    md_path, _ = _write_md(genealogy_dir, "030-rebuild.md", text)

    block_id = create_block(md_path, blocks_dir)

    # Rebuild to a new location
    output = tmp_path / "rebuilt.md"
    rebuild_working_copy(block_id, output, blocks_dir)

    assert output.is_file()
    assert output.read_text(encoding="utf-8") == text


def test_rebuild_working_copy_creates_parent(blocks_dir, genealogy_dir, tmp_path):
    text = "# Deep rebuild"
    md_path, _ = _write_md(genealogy_dir, "031-deep.md", text)
    block_id = create_block(md_path, blocks_dir)

    output = tmp_path / "deep" / "nested" / "output.md"
    rebuild_working_copy(block_id, output, blocks_dir)
    assert output.is_file()
    assert output.read_text(encoding="utf-8") == text


# --- create_revision ---

def test_create_revision_basic(genealogy_dir, blocks_dir):
    text_v1 = "# Version 1"
    md_path, _ = _write_md(genealogy_dir, "040-rev.md", text_v1)
    old_id = create_block(md_path, blocks_dir)

    # Modify the file
    text_v2 = "# Version 2 — revised"
    md_path.write_text(text_v2, encoding="utf-8")

    new_id = create_revision(old_id, md_path, blocks_dir)
    assert new_id != old_id
    assert new_id == compute_content_hash(text_v2)

    # Check replaces field
    new_block = json.loads(
        (blocks_dir / f"{new_id}.json").read_text(encoding="utf-8")
    )
    assert new_block["replaces"] == old_id
    assert new_block["content"]["full_text"] == text_v2


def test_create_revision_with_metadata(genealogy_dir, blocks_dir):
    md_path, _ = _write_md(genealogy_dir, "041-rev-meta.md", "# V1")
    old_id = create_block(md_path, blocks_dir)

    md_path.write_text("# V2 with meta", encoding="utf-8")
    new_id = create_revision(old_id, md_path, blocks_dir, metadata={
        "genealogy_id": "041",
        "type": "revision",
    })

    new_block = json.loads(
        (blocks_dir / f"{new_id}.json").read_text(encoding="utf-8")
    )
    assert new_block["genealogy_id"] == "041"
    assert new_block["type"] == "revision"
    assert new_block["replaces"] == old_id


def test_create_revision_old_block_not_found(genealogy_dir, blocks_dir):
    md_path, _ = _write_md(genealogy_dir, "042-no-old.md", "# New")
    with pytest.raises(FileNotFoundError, match="Old block not found"):
        create_revision("0" * 64, md_path, blocks_dir)


def test_create_revision_no_change(genealogy_dir, blocks_dir):
    text = "# Same"
    md_path, _ = _write_md(genealogy_dir, "043-same.md", text)
    old_id = create_block(md_path, blocks_dir)

    with pytest.raises(ValueError, match="Content unchanged"):
        create_revision(old_id, md_path, blocks_dir)


def test_create_revision_idempotent(genealogy_dir, blocks_dir):
    md_path, _ = _write_md(genealogy_dir, "044-idem-rev.md", "# V1")
    old_id = create_block(md_path, blocks_dir)

    md_path.write_text("# V2", encoding="utf-8")
    id1 = create_revision(old_id, md_path, blocks_dir)
    id2 = create_revision(old_id, md_path, blocks_dir)
    assert id1 == id2


# --- Round-trip: create → verify → rebuild ---

def test_full_round_trip(genealogy_dir, blocks_dir, tmp_path):
    text = "# Full round trip\n\n中文内容 + English."
    md_path, _ = _write_md(genealogy_dir, "050-round.md", text)

    # Create
    block_id = create_block(md_path, blocks_dir)

    # Verify
    assert verify_working_copy(md_path, blocks_dir) is True

    # Rebuild
    output = tmp_path / "rebuilt.md"
    rebuild_working_copy(block_id, output, blocks_dir)
    assert output.read_text(encoding="utf-8") == text

    # Get content
    assert get_block_content(block_id, blocks_dir) == text


def test_revision_chain(genealogy_dir, blocks_dir):
    """Create a chain of revisions: v1 → v2 → v3."""
    md_path, _ = _write_md(genealogy_dir, "060-chain.md", "# V1")
    id_v1 = create_block(md_path, blocks_dir)

    md_path.write_text("# V2", encoding="utf-8")
    id_v2 = create_revision(id_v1, md_path, blocks_dir)

    md_path.write_text("# V3", encoding="utf-8")
    id_v3 = create_revision(id_v2, md_path, blocks_dir)

    # All three blocks exist
    assert (blocks_dir / f"{id_v1}.json").is_file()
    assert (blocks_dir / f"{id_v2}.json").is_file()
    assert (blocks_dir / f"{id_v3}.json").is_file()

    # Check replaces chain
    v2_block = json.loads(
        (blocks_dir / f"{id_v2}.json").read_text(encoding="utf-8")
    )
    v3_block = json.loads(
        (blocks_dir / f"{id_v3}.json").read_text(encoding="utf-8")
    )
    assert v2_block["replaces"] == id_v1
    assert v3_block["replaces"] == id_v2
