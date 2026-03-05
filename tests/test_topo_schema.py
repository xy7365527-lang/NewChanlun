"""
Tests for block-topology schema normalization.

验证：
  1. 规范化后所有区块包含必需字段
  2. content_hash (id) 保持不变
  3. 关系完整性不受影响
  4. genealogy_id 正确提取
  5. 幂等性——二次运行无变化
"""
from __future__ import annotations

import json
import shutil
import tempfile
from pathlib import Path

import pytest

from scripts.normalize_block_schema import (
    extract_depends_on,
    extract_genealogy_id,
    extract_title,
    is_already_canonical,
    normalize_block,
    run,
)


CANONICAL_REQUIRED_KEYS = {"id", "type", "timestamp", "source", "content", "refs", "git_ref"}


# --- Unit tests for extraction functions ---


class TestExtractGenealogyId:
    def test_from_top_level(self):
        block = {"genealogy_id": "353", "id": "abc" * 21 + "a"}
        assert extract_genealogy_id(block) == "353"

    def test_from_content_id(self):
        block = {"content": {"id": "206"}}
        assert extract_genealogy_id(block) == "206"

    def test_from_content_number(self):
        block = {"content": {"number": 42}}
        assert extract_genealogy_id(block) == "42"

    def test_from_top_level_number(self):
        block = {"number": 206}
        assert extract_genealogy_id(block) == "206"

    def test_from_frontmatter(self):
        block = {
            "content": {
                "full_text": "---\nid: '328'\nnumber: 328\ntype: meta-rule\n---\n# Title"
            }
        }
        assert extract_genealogy_id(block) == "328"

    def test_sha256_id_not_extracted(self):
        sha = "a" * 64
        block = {"content": {"id": sha}}
        assert extract_genealogy_id(block) is None

    def test_none_when_missing(self):
        block = {"content": {}}
        assert extract_genealogy_id(block) is None

    def test_suffix_id(self):
        block = {"content": {"full_text": "---\nid: '005b'\n---\n"}}
        assert extract_genealogy_id(block) == "005b"


class TestExtractTitle:
    def test_from_top_level(self):
        block = {"title": "My Title"}
        assert extract_title(block) == "My Title"

    def test_from_content(self):
        block = {"content": {"title": "Content Title"}}
        assert extract_title(block) == "Content Title"

    def test_none_when_missing(self):
        block = {"content": {}}
        assert extract_title(block) is None


class TestExtractDependsOn:
    def test_from_top_level(self):
        block = {"depends_on": ["036", "016"]}
        assert extract_depends_on(block) == ["036", "016"]

    def test_from_content(self):
        block = {"content": {"depends_on": [350, 267]}}
        assert extract_depends_on(block) == ["350", "267"]

    def test_none_when_missing(self):
        block = {"content": {}}
        assert extract_depends_on(block) is None


# --- Unit tests for normalize_block ---


class TestNormalizeBlock:
    def test_canonical_block_unchanged(self):
        block = {
            "id": "abc123",
            "type": "event",
            "timestamp": "2026-01-01T00:00:00+00:00",
            "source": "cc",
            "content": {"data": "test"},
            "refs": [],
            "git_ref": "",
        }
        result = normalize_block(block)
        assert CANONICAL_REQUIRED_KEYS.issubset(result.keys())
        assert result["id"] == block["id"]

    def test_legacy_format(self):
        """Format: created/number/source_file (14 blocks of this type)."""
        block = {
            "id": "abc123",
            "number": 206,
            "source_file": ".chanlun/genealogy/settled/206-test.md",
            "type": "event",
            "created": "2026-02-25T21:04:51.243882",
            "source": "cc",
        }
        result = normalize_block(block)
        assert CANONICAL_REQUIRED_KEYS.issubset(result.keys())
        assert result["id"] == "abc123"
        assert result["genealogy_id"] == "206"
        assert result["type"] == "event"
        assert result["timestamp"] == "2026-02-25T21:04:51.243882"

    def test_genealogy_mapper_format(self):
        """Format: genealogy_id/title/depends_on/content (newest blocks)."""
        block = {
            "id": "def456",
            "genealogy_id": "353",
            "title": "Test Title",
            "type": "meta-rule",
            "depends_on": ["036", "016"],
            "content": {
                "source_file": "settled/353-test.md",
                "id": "353",
                "full_text": "---\nid: '353'\n---\n# content",
            },
        }
        result = normalize_block(block)
        assert CANONICAL_REQUIRED_KEYS.issubset(result.keys())
        assert result["id"] == "def456"
        assert result["genealogy_id"] == "353"
        assert result["title"] == "Test Title"
        assert result["depends_on"] == ["036", "016"]
        assert result["type"] == "event"  # meta-rule -> event
        assert result["content"]["original_type"] == "meta-rule"

    def test_minimal_format(self):
        """Format: content/id/timestamp only (5 blocks)."""
        block = {
            "id": "ghi789",
            "timestamp": "2026-03-04T18:42:20.927443+00:00",
            "content": {
                "full_text": "---\nid: '320'\nnumber: 320\n---\n# content",
                "source_file": "settled/320-test.md",
                "id": "320",
            },
        }
        result = normalize_block(block)
        assert CANONICAL_REQUIRED_KEYS.issubset(result.keys())
        assert result["id"] == "ghi789"
        assert result["genealogy_id"] == "320"

    def test_id_preserved(self):
        """id must never change during normalization."""
        block = {
            "id": "test_id_12345",
            "type": "event",
            "content": {},
        }
        result = normalize_block(block)
        assert result["id"] == "test_id_12345"

    def test_non_standard_type_becomes_event(self):
        block = {
            "id": "t1",
            "type": "概念定义",
            "content": {},
        }
        result = normalize_block(block)
        assert result["type"] == "event"
        assert result["content"]["original_type"] == "概念定义"

    def test_dict_source_defaults_to_migration(self):
        block = {
            "id": "t2",
            "type": "event",
            "source": {"foo": "bar"},
            "content": {},
        }
        result = normalize_block(block)
        assert result["source"] == "migration"

    def test_date_only_becomes_timestamp(self):
        block = {
            "id": "t3",
            "date": "2026-03-04",
            "content": {},
        }
        result = normalize_block(block)
        assert "T" in result["timestamp"]

    def test_refs_defaults_to_empty_list(self):
        block = {"id": "t4", "content": {}}
        result = normalize_block(block)
        assert result["refs"] == []

    def test_git_ref_defaults_to_empty_string(self):
        block = {"id": "t5", "content": {}}
        result = normalize_block(block)
        assert result["git_ref"] == ""


class TestIsAlreadyCanonical:
    def test_canonical(self):
        block = {
            "id": "x",
            "type": "event",
            "timestamp": "t",
            "source": "cc",
            "content": {},
            "refs": [],
            "git_ref": "",
        }
        assert is_already_canonical(block) is True

    def test_missing_field(self):
        block = {
            "id": "x",
            "type": "event",
            "content": {},
        }
        assert is_already_canonical(block) is False


# --- Integration tests using temp directory ---


@pytest.fixture
def temp_blocks_dir(tmp_path):
    """Create a temporary blocks directory with sample blocks."""
    base = tmp_path / ".chanlun" / "block-topology"
    blocks_dir = base / "blocks"
    blocks_dir.mkdir(parents=True)

    # Canonical block
    b1 = {
        "id": "canonical_block",
        "type": "event",
        "timestamp": "2026-01-01T00:00:00+00:00",
        "source": "cc",
        "content": {"data": "test"},
        "refs": ["ref1"],
        "git_ref": "abc123",
    }
    (blocks_dir / "canonical_block.json").write_text(
        json.dumps(b1, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    # Legacy block
    b2 = {
        "id": "legacy_block",
        "number": 100,
        "source_file": "test.md",
        "type": "event",
        "created": "2026-01-01T00:00:00",
        "source": "cc",
    }
    (blocks_dir / "legacy_block.json").write_text(
        json.dumps(b2, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    # Genealogy mapper block
    b3 = {
        "id": "genealogy_block",
        "genealogy_id": "200",
        "title": "Test Genealogy",
        "type": "概念定义",
        "depends_on": ["100", "150"],
        "content": {
            "source_file": "settled/200-test.md",
            "id": "200",
            "full_text": "---\nid: '200'\n---\ncontent",
        },
    }
    (blocks_dir / "genealogy_block.json").write_text(
        json.dumps(b3, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    return tmp_path


class TestRunIntegration:
    def test_dry_run_no_changes(self, temp_blocks_dir):
        blocks_dir = temp_blocks_dir / ".chanlun" / "block-topology" / "blocks"

        # Save original content
        originals = {}
        for f in blocks_dir.iterdir():
            originals[f.name] = f.read_text(encoding="utf-8")

        stats = run(base=temp_blocks_dir, dry_run=True)
        assert stats["total"] == 3

        # Verify no files changed
        for f in blocks_dir.iterdir():
            assert f.read_text(encoding="utf-8") == originals[f.name]

    def test_migration_updates_files(self, temp_blocks_dir):
        stats = run(base=temp_blocks_dir, dry_run=False)
        assert stats["total"] == 3
        assert stats["migrated"] >= 2  # legacy + genealogy blocks

        blocks_dir = temp_blocks_dir / ".chanlun" / "block-topology" / "blocks"
        for f in blocks_dir.iterdir():
            block = json.loads(f.read_text(encoding="utf-8"))
            assert CANONICAL_REQUIRED_KEYS.issubset(block.keys()), \
                f"{f.name} missing keys: {CANONICAL_REQUIRED_KEYS - set(block.keys())}"

    def test_idempotent(self, temp_blocks_dir):
        """Second run should find all blocks canonical."""
        run(base=temp_blocks_dir, dry_run=False)
        stats2 = run(base=temp_blocks_dir, dry_run=False)
        assert stats2["migrated"] == 0
        assert stats2["already_canonical"] == stats2["total"]

    def test_id_preserved_in_migration(self, temp_blocks_dir):
        blocks_dir = temp_blocks_dir / ".chanlun" / "block-topology" / "blocks"

        # Collect original ids
        original_ids = {}
        for f in blocks_dir.iterdir():
            block = json.loads(f.read_text(encoding="utf-8"))
            original_ids[f.name] = block.get("id")

        run(base=temp_blocks_dir, dry_run=False)

        for f in blocks_dir.iterdir():
            block = json.loads(f.read_text(encoding="utf-8"))
            assert block["id"] == original_ids[f.name], \
                f"id changed in {f.name}: {original_ids[f.name]} -> {block['id']}"

    def test_genealogy_id_extracted(self, temp_blocks_dir):
        run(base=temp_blocks_dir, dry_run=False)

        blocks_dir = temp_blocks_dir / ".chanlun" / "block-topology" / "blocks"
        genealogy_block = json.loads(
            (blocks_dir / "genealogy_block.json").read_text(encoding="utf-8")
        )
        assert genealogy_block.get("genealogy_id") == "200"
        assert genealogy_block.get("title") == "Test Genealogy"
        assert genealogy_block.get("depends_on") == ["100", "150"]

    def test_legacy_block_normalized(self, temp_blocks_dir):
        run(base=temp_blocks_dir, dry_run=False)

        blocks_dir = temp_blocks_dir / ".chanlun" / "block-topology" / "blocks"
        legacy = json.loads(
            (blocks_dir / "legacy_block.json").read_text(encoding="utf-8")
        )
        assert legacy.get("genealogy_id") == "100"
        assert "timestamp" in legacy
        assert "refs" in legacy
        assert isinstance(legacy["refs"], list)


# --- Real data tests (if blocks directory exists) ---


REAL_BLOCKS_DIR = Path(".chanlun/block-topology/blocks")


@pytest.mark.skipif(
    not REAL_BLOCKS_DIR.exists(),
    reason="Real blocks directory not available"
)
class TestRealData:
    def test_all_blocks_normalizable(self):
        """All real blocks can be normalized without errors."""
        for f in REAL_BLOCKS_DIR.iterdir():
            if f.suffix != ".json":
                continue
            block = json.loads(f.read_text(encoding="utf-8"))
            result = normalize_block(block)
            assert CANONICAL_REQUIRED_KEYS.issubset(result.keys()), \
                f"{f.name} missing keys after normalization"

    def test_all_ids_preserved(self):
        """No id changes across all real blocks."""
        for f in REAL_BLOCKS_DIR.iterdir():
            if f.suffix != ".json":
                continue
            block = json.loads(f.read_text(encoding="utf-8"))
            result = normalize_block(block)
            assert result["id"] == block.get("id", f.stem), \
                f"id mismatch in {f.name}"

    def test_dry_run_stats(self):
        """Dry run completes without error."""
        stats = run(dry_run=True)
        assert stats["total"] > 0
        assert not stats.get("errors")
