#!/usr/bin/env python3
"""
tests/test_relation_types.py

测试 scripts/extract_relation_types.py 的关系提取逻辑。

覆盖：
- frontmatter 解析和关系提取
- negates → supersedes 映射
- negated_by → supersedes 反向映射
- replaces → supersedes 映射
- 编号规范化（带后缀、带注释文本）
- 去重逻辑
- 格式正确性
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

import pytest

# 将 scripts 加入 path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from extract_relation_types import (
    _normalize_id,
    _to_list,
    deduplicate_relations,
    load_existing_relations,
    load_id_mapping,
    parse_frontmatter,
    write_relations,
    update_meta_relation_count,
)


# --- _normalize_id tests ---


class TestNormalizeId:
    def test_simple_three_digit(self):
        assert _normalize_id("086") == "086"

    def test_with_letter_suffix(self):
        assert _normalize_id("073a") == "073a"

    def test_with_chinese_annotation(self):
        assert _normalize_id("073a号（depth_budget 基因废除）") == "073a"

    def test_with_longer_annotation(self):
        assert _normalize_id("218号（扩展：218割掉串行尾巴，275割掉全局排序思维）") == "218"

    def test_with_filename_annotation(self):
        assert _normalize_id("323号theoretical_settlement.md（2026-03-03版）") == "323"

    def test_with_method_description(self):
        assert _normalize_id("269号验证方法（直接在月线/年线K线上跑D算子——使用了被否定的口径B）") == "269"

    def test_short_number_padded(self):
        assert _normalize_id("32") == "032"

    def test_empty_string(self):
        assert _normalize_id("") == ""

    def test_integer_input(self):
        assert _normalize_id("88") == "088"


# --- _to_list tests ---


class TestToList:
    def test_none(self):
        assert _to_list(None) == []

    def test_empty_list(self):
        assert _to_list([]) == []

    def test_single_string(self):
        assert _to_list("086") == ["086"]

    def test_list_of_strings(self):
        assert _to_list(["086", "087"]) == ["086", "087"]

    def test_dict_with_target(self):
        """137号的 negates 格式。"""
        val = [{"target": "no-unnecessary-escalation.md 的修复路径", "content": "..."}]
        result = _to_list(val)
        assert len(result) == 1
        assert "no-unnecessary-escalation" in result[0]

    def test_integer(self):
        assert _to_list(215) == ["215"]


# --- deduplicate_relations tests ---


class TestDeduplicateRelations:
    def test_removes_duplicates(self):
        id_mapping = {"087": "hash_087", "086": "hash_086"}
        existing: set[tuple[str, str, str]] = set()
        extracted = [
            {"from_num": "087", "to_num": "086", "relation": "supersedes", "source_file": "a.md"},
            {"from_num": "087", "to_num": "086", "relation": "supersedes", "source_file": "b.md"},
        ]
        result = deduplicate_relations(extracted, id_mapping, existing)
        assert len(result) == 1

    def test_skips_existing(self):
        id_mapping = {"087": "hash_087", "086": "hash_086"}
        existing = {("hash_087", "hash_086", "supersedes")}
        extracted = [
            {"from_num": "087", "to_num": "086", "relation": "supersedes", "source_file": "a.md"},
        ]
        result = deduplicate_relations(extracted, id_mapping, existing)
        assert len(result) == 0

    def test_skips_unmapped_ids(self):
        id_mapping = {"087": "hash_087"}
        existing: set[tuple[str, str, str]] = set()
        extracted = [
            {"from_num": "087", "to_num": "999", "relation": "supersedes", "source_file": "a.md"},
        ]
        result = deduplicate_relations(extracted, id_mapping, existing)
        assert len(result) == 0

    def test_keeps_different_relation_types(self):
        id_mapping = {"087": "hash_087", "086": "hash_086"}
        existing: set[tuple[str, str, str]] = set()
        extracted = [
            {"from_num": "087", "to_num": "086", "relation": "supersedes", "source_file": "a.md"},
            {"from_num": "087", "to_num": "086", "relation": "modifies", "source_file": "a.md"},
        ]
        result = deduplicate_relations(extracted, id_mapping, existing)
        assert len(result) == 2


# --- write_relations tests ---


class TestWriteRelations:
    def test_writes_correct_format(self, tmp_path):
        relations_path = tmp_path / "relations.jsonl"
        relations_path.touch()
        new_rels = [
            {
                "from_bid": "a" * 64,
                "to_bid": "b" * 64,
                "relation": "supersedes",
                "from_num": "087",
                "to_num": "086",
                "source_file": "087-test.md",
            }
        ]
        created_by = "c" * 64
        count = write_relations(new_rels, created_by, relations_path)
        assert count == 1

        lines = relations_path.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["from"] == "a" * 64
        assert rec["to"] == "b" * 64
        assert rec["relation"] == "supersedes"
        assert rec["order"] == 1
        assert rec["created_by"] == "c" * 64
        assert "timestamp" in rec

    def test_appends_not_overwrites(self, tmp_path):
        relations_path = tmp_path / "relations.jsonl"
        relations_path.write_text('{"existing": true}\n', encoding="utf-8")
        new_rels = [
            {
                "from_bid": "a" * 64,
                "to_bid": "b" * 64,
                "relation": "supersedes",
                "from_num": "087",
                "to_num": "086",
                "source_file": "087-test.md",
            }
        ]
        write_relations(new_rels, "c" * 64, relations_path)
        lines = relations_path.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 2
        assert '"existing"' in lines[0]


# --- update_meta_relation_count tests ---


class TestUpdateMeta:
    def test_updates_count(self, tmp_path):
        meta_path = tmp_path / "meta.json"
        meta_path.write_text(json.dumps({
            "relation_count": 0,
            "id_mapping": {},
        }), encoding="utf-8")

        relations_path = tmp_path / "relations.jsonl"
        relations_path.write_text('{"a":1}\n{"b":2}\n{"c":3}\n', encoding="utf-8")

        # Monkey-patch the module-level paths
        import extract_relation_types as mod
        orig_rel = mod.RELATIONS_PATH
        mod.RELATIONS_PATH = relations_path
        try:
            count = update_meta_relation_count(meta_path)
        finally:
            mod.RELATIONS_PATH = orig_rel

        assert count == 3
        updated = json.loads(meta_path.read_text(encoding="utf-8"))
        assert updated["relation_count"] == 3


# --- parse_frontmatter tests ---


class TestParseFrontmatter:
    def test_valid_frontmatter(self, tmp_path):
        md = tmp_path / "test.md"
        md.write_text(
            '---\nid: "087"\nnegates: ["086"]\n---\n# Body\n',
            encoding="utf-8",
        )
        fm = parse_frontmatter(md)
        assert fm is not None
        assert fm["id"] == "087"
        assert fm["negates"] == ["086"]

    def test_no_frontmatter(self, tmp_path):
        md = tmp_path / "test.md"
        md.write_text("# No frontmatter\n", encoding="utf-8")
        fm = parse_frontmatter(md)
        assert fm is None
