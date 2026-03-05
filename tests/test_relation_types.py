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
- 至少 5 个已知 supersedes 关系
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
    extract_relations,
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


# --- extract_relations tests with real genealogy ---


REAL_GENEALOGY = Path(".chanlun/genealogy/settled")


@pytest.mark.skipif(
    not REAL_GENEALOGY.exists(),
    reason="Genealogy directory not available",
)
class TestExtractRelationsReal:
    """在真实谱系目录上测试提取逻辑。"""

    def setup_method(self):
        self.relations = extract_relations(REAL_GENEALOGY)

    def test_extracts_nonzero_relations(self):
        assert len(self.relations) > 0

    def test_known_supersedes_087_negates_086(self):
        """087号 negates 086号 → supersedes (may appear from both negates and negated_by)."""
        matches = [
            r for r in self.relations
            if r["from_num"] == "087" and r["to_num"] == "086"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) >= 1

    def test_known_supersedes_068_negates_065(self):
        """068号 negates 065号 → supersedes (may appear from both negates and negated_by)."""
        matches = [
            r for r in self.relations
            if r["from_num"] == "068" and r["to_num"] == "065"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) >= 1

    def test_known_supersedes_068_negates_066(self):
        """068号 negates 066号 → supersedes (may appear from both negates and negated_by)."""
        matches = [
            r for r in self.relations
            if r["from_num"] == "068" and r["to_num"] == "066"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) >= 1

    def test_known_supersedes_088_negates_032(self):
        """088号 negates 032号 → supersedes (may appear from both negates and negated_by)."""
        matches = [
            r for r in self.relations
            if r["from_num"] == "088" and r["to_num"] == "032"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) >= 1

    def test_known_supersedes_073a_negates_073(self):
        """073a号 negates 073号 → supersedes (may appear from both negates and negated_by)."""
        matches = [
            r for r in self.relations
            if r["from_num"] == "073a" and r["to_num"] == "073"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) >= 1

    def test_known_supersedes_from_negated_by_086(self):
        """086号 negated_by 087号 → 087 supersedes 086（与 negates 方向一致）。"""
        # 应只有 1 条（去重后）：无论从 negates 还是 negated_by 提取
        matches = [
            r for r in self.relations
            if r["from_num"] == "087" and r["to_num"] == "086"
            and r["relation"] == "supersedes"
        ]
        # extract_relations 可能从 087.negates 和 086.negated_by 各提取一条
        # 但 deduplicate_relations 会合并。这里验证提取阶段至少有 1 条。
        assert len(matches) >= 1

    def test_known_supersedes_replaces_329(self):
        """329号 replaces 323号 → supersedes。"""
        matches = [
            r for r in self.relations
            if r["from_num"] == "329" and r["to_num"] == "323"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) == 1

    def test_all_relations_have_required_fields(self):
        """所有提取的关系都有必要字段。"""
        for rel in self.relations:
            assert "from_num" in rel
            assert "to_num" in rel
            assert "relation" in rel
            assert "source_file" in rel
            assert rel["relation"] in ("supersedes", "reopens", "modifies")

    def test_no_self_referencing_relations(self):
        """不应存在自引用关系。"""
        for rel in self.relations:
            assert rel["from_num"] != rel["to_num"], (
                f"Self-reference: {rel['from_num']} → {rel['to_num']} "
                f"({rel['relation']}) in {rel['source_file']}"
            )

    def test_at_least_five_supersedes(self):
        """至少有 5 个 supersedes 关系。"""
        supersedes = [r for r in self.relations if r["relation"] == "supersedes"]
        assert len(supersedes) >= 5, f"Only {len(supersedes)} supersedes found"

    def test_dedup_collapses_dual_sources(self):
        """negates + negated_by 双向提取的同一关系在 dedup 后只剩 1 条。"""
        id_mapping = load_id_mapping()
        existing: set[tuple[str, str, str]] = set()
        deduped = deduplicate_relations(self.relations, id_mapping, existing)
        # 087→086 should appear only once after dedup
        matches = [
            r for r in deduped
            if r["from_num"] == "087" and r["to_num"] == "086"
            and r["relation"] == "supersedes"
        ]
        assert len(matches) == 1


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
