"""Tests for scripts/verify_block_integrity.py — 区块拓扑内容完整性验证"""
from __future__ import annotations

import hashlib
import json

import pytest

from scripts.verify_block_integrity import (
    compute_file_hash,
    get_block_id,
    get_genealogy_number,
    resign_block,
    resolve_source_file,
    stamp_blocks,
    verify_all,
)


@pytest.fixture
def mock_repo(tmp_path):
    """Create a minimal block-topology + genealogy structure."""
    bt = tmp_path / ".chanlun" / "block-topology"
    blocks = bt / "blocks"
    blocks.mkdir(parents=True)

    gen = tmp_path / ".chanlun" / "genealogy"
    settled = gen / "settled"
    settled.mkdir(parents=True)

    return tmp_path


def _write_genealogy(root, num, title, content_text):
    """Helper: write a genealogy .md file and return its path + hash."""
    settled = root / ".chanlun" / "genealogy" / "settled"
    fname = f"{num:03d}-{title}.md"
    fpath = settled / fname
    fpath.write_text(content_text, encoding="utf-8")
    h = hashlib.sha256(content_text.encode("utf-8")).hexdigest()
    return fpath, h


def _write_block(root, block_dict):
    """Helper: write a block JSON file."""
    blocks = root / ".chanlun" / "block-topology" / "blocks"
    bid = block_dict.get("id", block_dict.get("block_id", "unknown"))
    fpath = blocks / f"{bid}.json"
    fpath.write_text(json.dumps(block_dict, ensure_ascii=False, indent=2), encoding="utf-8")
    return fpath


# --- compute_file_hash ---

def test_compute_file_hash(tmp_path):
    f = tmp_path / "test.md"
    f.write_text("hello world", encoding="utf-8")
    expected = hashlib.sha256(b"hello world").hexdigest()
    assert compute_file_hash(f) == expected


# --- resolve_source_file ---

def test_resolve_legacy_source_file(mock_repo):
    _write_genealogy(mock_repo, 1, "test-entry", "# Test")
    block = {
        "id": "a" * 64,
        "content": {"source_file": "settled/001-test-entry.md"},
    }
    gen_dir = mock_repo / ".chanlun" / "genealogy"
    result = resolve_source_file(block, gen_dir)
    assert result is not None
    assert result.name == "001-test-entry.md"


def test_resolve_legacy_backslash(mock_repo):
    _write_genealogy(mock_repo, 2, "backslash", "# Back")
    block = {
        "id": "b" * 64,
        "content": {"source_file": "settled\\002-backslash.md"},
    }
    gen_dir = mock_repo / ".chanlun" / "genealogy"
    result = resolve_source_file(block, gen_dir)
    assert result is not None


def test_resolve_new_format_genealogy_id(mock_repo):
    _write_genealogy(mock_repo, 42, "new-format", "# New")
    block = {"genealogy_id": "42", "title": "test"}
    gen_dir = mock_repo / ".chanlun" / "genealogy"
    result = resolve_source_file(block, gen_dir)
    assert result is not None
    assert "042-new-format.md" in result.name


def test_resolve_no_source_file(mock_repo):
    block = {"id": "c" * 64, "type": "rewrite", "content": {"action": "x"}}
    gen_dir = mock_repo / ".chanlun" / "genealogy"
    result = resolve_source_file(block, gen_dir)
    assert result is None


def test_resolve_missing_file(mock_repo):
    block = {
        "id": "d" * 64,
        "content": {"source_file": "settled/999-nonexistent.md"},
    }
    gen_dir = mock_repo / ".chanlun" / "genealogy"
    result = resolve_source_file(block, gen_dir)
    assert result is None


# --- get_block_id / get_genealogy_number ---

def test_get_block_id_standard():
    assert get_block_id({"id": "abc123"}) == "abc123"


def test_get_block_id_fallback():
    assert get_block_id({"block_id": "xyz"}) == "xyz"


def test_get_block_id_missing():
    assert get_block_id({}) == ""


def test_get_genealogy_number_new_format():
    assert get_genealogy_number({"genealogy_id": "42"}) == "42"


def test_get_genealogy_number_legacy():
    assert get_genealogy_number({"content": {"id": "007"}}) == "007"


# --- verify_all ---

def test_verify_all_pass(mock_repo):
    """Block with matching content_hash passes verification."""
    _, file_hash = _write_genealogy(mock_repo, 1, "test", "# Content")
    _write_block(mock_repo, {
        "id": "a" * 64,
        "type": "event",
        "content": {"source_file": "settled/001-test.md", "id": "001"},
        "content_hash": file_hash,
    })

    result = verify_all(mock_repo)
    assert result["verified"] == 1
    assert result["mismatched"] == []
    assert result["missing_hash"] == []


def test_verify_all_mismatch(mock_repo):
    """Block with wrong content_hash is flagged as mismatched."""
    _write_genealogy(mock_repo, 2, "tampered", "# Original content")
    _write_block(mock_repo, {
        "id": "b" * 64,
        "type": "event",
        "content": {"source_file": "settled/002-tampered.md", "id": "002"},
        "content_hash": "0" * 64,  # Wrong hash
    })

    result = verify_all(mock_repo)
    assert result["verified"] == 0
    assert len(result["mismatched"]) == 1
    assert result["mismatched"][0]["genealogy"] == "002"


def test_verify_all_missing_hash(mock_repo):
    """Block without content_hash is reported as missing_hash."""
    _write_genealogy(mock_repo, 3, "no-hash", "# No hash yet")
    _write_block(mock_repo, {
        "id": "c" * 64,
        "type": "event",
        "content": {"source_file": "settled/003-no-hash.md", "id": "003"},
    })

    result = verify_all(mock_repo)
    assert result["verified"] == 0
    assert len(result["missing_hash"]) == 1
    assert result["missing_hash"][0]["genealogy"] == "003"


def test_verify_all_skips_rewrite_blocks(mock_repo):
    """Blocks without source file references are skipped."""
    _write_block(mock_repo, {
        "id": "d" * 64,
        "type": "rewrite",
        "content": {"action": "relation_append"},
    })

    result = verify_all(mock_repo)
    assert result["skipped"] == 1
    assert result["total"] == 1


def test_verify_all_missing_file(mock_repo):
    """Block pointing to nonexistent file is reported."""
    _write_block(mock_repo, {
        "id": "e" * 64,
        "type": "event",
        "content": {"source_file": "settled/999-gone.md", "id": "999"},
    })

    result = verify_all(mock_repo)
    assert len(result["missing_file"]) == 1


# --- stamp_blocks ---

def test_stamp_writes_content_hash(mock_repo):
    """stamp_blocks writes content_hash for blocks that lack it."""
    _, file_hash = _write_genealogy(mock_repo, 5, "stampme", "# Stamp me")
    block_id = "f" * 64
    _write_block(mock_repo, {
        "id": block_id,
        "type": "event",
        "content": {"source_file": "settled/005-stampme.md", "id": "005"},
    })

    result = stamp_blocks(mock_repo)
    assert result["stamped"] == 1

    # Verify the block now has content_hash
    block_path = mock_repo / ".chanlun" / "block-topology" / "blocks" / f"{block_id}.json"
    updated = json.loads(block_path.read_text(encoding="utf-8"))
    assert updated["content_hash"] == file_hash


def test_stamp_skips_already_hashed(mock_repo):
    """stamp_blocks skips blocks that already have content_hash."""
    _, file_hash = _write_genealogy(mock_repo, 6, "already", "# Already")
    _write_block(mock_repo, {
        "id": "a1" + "0" * 62,
        "type": "event",
        "content": {"source_file": "settled/006-already.md", "id": "006"},
        "content_hash": file_hash,
    })

    result = stamp_blocks(mock_repo)
    assert result["stamped"] == 0


# --- resign_block ---

def test_resign_updates_hash(mock_repo):
    """resign_block updates content_hash after file modification."""
    fpath, old_hash = _write_genealogy(mock_repo, 7, "resign", "# Original")
    block_id = "b1" + "0" * 62
    _write_block(mock_repo, {
        "id": block_id,
        "type": "event",
        "content": {"source_file": "settled/007-resign.md", "id": "007"},
        "content_hash": old_hash,
    })

    # Modify the file (legitimate edit)
    fpath.write_text("# Modified content", encoding="utf-8")
    new_hash = hashlib.sha256(b"# Modified content").hexdigest()

    result = resign_block(mock_repo, "007")
    assert result["success"] is True
    assert result["new_hash"] == new_hash
    assert result["old_hash"] == old_hash

    # Verify block file is updated
    block_path = mock_repo / ".chanlun" / "block-topology" / "blocks" / f"{block_id}.json"
    updated = json.loads(block_path.read_text(encoding="utf-8"))
    assert updated["content_hash"] == new_hash


def test_resign_nonexistent_genealogy(mock_repo):
    """resign_block returns error for unknown genealogy number."""
    result = resign_block(mock_repo, "999")
    assert result["success"] is False
    assert "No block found" in result["error"]


# --- Integration: stamp then verify ---

def test_stamp_then_verify_passes(mock_repo):
    """After stamping, verify should pass."""
    _write_genealogy(mock_repo, 10, "e2e", "# End to end")
    _write_block(mock_repo, {
        "id": "c1" + "0" * 62,
        "type": "event",
        "content": {"source_file": "settled/010-e2e.md", "id": "010"},
    })

    # Before stamp: missing_hash
    result1 = verify_all(mock_repo)
    assert len(result1["missing_hash"]) == 1

    # Stamp
    stamp_blocks(mock_repo)

    # After stamp: verified
    result2 = verify_all(mock_repo)
    assert result2["verified"] == 1
    assert result2["mismatched"] == []
    assert result2["missing_hash"] == []


def test_stamp_verify_tamper_detect(mock_repo):
    """After stamp + file modification, verify detects mismatch."""
    fpath, _ = _write_genealogy(mock_repo, 11, "tamper", "# Original")
    _write_block(mock_repo, {
        "id": "d1" + "0" * 62,
        "type": "event",
        "content": {"source_file": "settled/011-tamper.md", "id": "011"},
    })

    stamp_blocks(mock_repo)
    result1 = verify_all(mock_repo)
    assert result1["verified"] == 1

    # Tamper the file
    fpath.write_text("# Tampered!", encoding="utf-8")

    result2 = verify_all(mock_repo)
    assert len(result2["mismatched"]) == 1
    assert result2["verified"] == 0


# --- Content-addressed block verification ---

def _write_content_addressed_block(root, text, genealogy_num=None, source_file=None):
    """Helper: write a content-addressed block (id = SHA256(full_text))."""
    blocks = root / ".chanlun" / "block-topology" / "blocks"
    block_id = hashlib.sha256(text.encode("utf-8")).hexdigest()
    block = {
        "id": block_id,
        "content": {"full_text": text},
    }
    if genealogy_num:
        block["genealogy_id"] = str(genealogy_num)
    if source_file:
        block["content"]["source_file"] = source_file
        block["content"]["id"] = str(genealogy_num)
    fpath = blocks / f"{block_id}.json"
    fpath.write_text(json.dumps(block, ensure_ascii=False, indent=2), encoding="utf-8")
    return block_id


def test_verify_content_addressed_pass(mock_repo):
    """Content-addressed block with valid id = SHA256(full_text) passes."""
    text = "# Content-addressed block"
    _write_content_addressed_block(mock_repo, text)

    result = verify_all(mock_repo)
    assert result["verified"] == 1
    assert result["mismatched"] == []


def test_verify_content_addressed_tampered_full_text(mock_repo):
    """Content-addressed block with id != SHA256(full_text) fails."""
    blocks = mock_repo / ".chanlun" / "block-topology" / "blocks"
    block_id = "a" * 64
    block = {
        "id": block_id,
        "content": {"full_text": "# This text doesn't match the id"},
    }
    (blocks / f"{block_id}.json").write_text(
        json.dumps(block, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    result = verify_all(mock_repo)
    assert len(result["mismatched"]) == 1
    assert "SHA256(full_text)" in result["mismatched"][0]["reason"]


def test_verify_content_addressed_working_copy_drift(mock_repo):
    """Content-addressed block passes but working copy has drifted."""
    text = "# Original content"
    _write_genealogy(mock_repo, 50, "drift", text)
    _write_content_addressed_block(
        mock_repo, text, genealogy_num=50, source_file="settled/050-drift.md"
    )

    # Now modify the .md file (working copy drifts)
    settled = mock_repo / ".chanlun" / "genealogy" / "settled"
    (settled / "050-drift.md").write_text("# Modified content", encoding="utf-8")

    result = verify_all(mock_repo)
    assert result["verified"] == 0
    assert len(result.get("working_copy_drift", [])) == 1
    assert "working copy differs" in result["working_copy_drift"][0]["reason"]


def test_verify_content_addressed_with_matching_working_copy(mock_repo):
    """Content-addressed block with matching working copy passes fully."""
    text = "# Matching working copy"
    _write_genealogy(mock_repo, 51, "match", text)
    _write_content_addressed_block(
        mock_repo, text, genealogy_num=51, source_file="settled/051-match.md"
    )

    result = verify_all(mock_repo)
    assert result["verified"] == 1
    assert result.get("working_copy_drift", []) == []


def test_verify_mixed_legacy_and_content_addressed(mock_repo):
    """Mix of legacy and content-addressed blocks both verified correctly."""
    # Legacy block with content_hash
    legacy_text = "# Legacy"
    _, file_hash = _write_genealogy(mock_repo, 60, "legacy", legacy_text)
    _write_block(mock_repo, {
        "id": "b" * 64,
        "type": "event",
        "content": {"source_file": "settled/060-legacy.md", "id": "060"},
        "content_hash": file_hash,
    })

    # Content-addressed block
    ca_text = "# Content-addressed"
    _write_content_addressed_block(mock_repo, ca_text)

    result = verify_all(mock_repo)
    assert result["verified"] == 2
    assert result["mismatched"] == []
