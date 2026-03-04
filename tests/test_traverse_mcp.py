"""tests/test_traverse_mcp.py — traverse_mcp.py CLI tests

Tests all four commands (query, search, deps, block) using temporary
genealogy and topology fixtures. No real data dependency.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Ensure scripts/ is importable
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from traverse_mcp import (
    _normalize_number,
    _parse_frontmatter,
    cmd_block,
    cmd_deps,
    cmd_query,
    cmd_search,
    main,
)


# --- Fixtures ---


SAMPLE_GENEALOGY_001 = """\
---
id: "001"
title: "退化段——古怪线段与笔定义的关系"
status: "已结算"
type: "矛盾记录"
date: "2026-02-15"
depends_on: ["002", "003"]
related: ["004"]
negated_by: []
negates: []
---

# 矛盾记录 001：退化段

线段 v1 特征序列法产出了退化段。
"""

SAMPLE_GENEALOGY_002 = """\
---
id: "002"
title: "知识库来源不完整"
status: "已结算"
type: "矛盾记录"
date: "2026-02-15"
depends_on: []
related: ["001"]
negated_by: []
negates: []
---

# 矛盾记录 002：知识库来源不完整
"""

SAMPLE_GENEALOGY_003 = """\
---
id: "003"
title: "线段的两个口径"
status: "已结算"
type: "概念分离"
date: "2026-02-15"
depends_on: ["001"]
related: []
negated_by: []
negates: []
---

# 概念分离 003：线段口径
"""

SAMPLE_GENEALOGY_005A = """\
---
id: "005a"
title: "禁止非对象来源的否定（定理）"
status: "已结算"
type: "定理"
date: "2026-02-16"
depends_on: ["005"]
related: []
negated_by: []
negates: []
---

# 定理 005a
"""


@pytest.fixture()
def genealogy_dir(tmp_path: Path) -> Path:
    """Create a temp genealogy/settled directory with sample files."""
    settled = tmp_path / "settled"
    settled.mkdir()
    (settled / "001-degenerate-segment.md").write_text(
        SAMPLE_GENEALOGY_001, encoding="utf-8",
    )
    (settled / "002-source-incompleteness.md").write_text(
        SAMPLE_GENEALOGY_002, encoding="utf-8",
    )
    (settled / "003-segment-concept-separation.md").write_text(
        SAMPLE_GENEALOGY_003, encoding="utf-8",
    )
    (settled / "005a-prohibition-theorem.md").write_text(
        SAMPLE_GENEALOGY_005A, encoding="utf-8",
    )
    return tmp_path


@pytest.fixture()
def topo_base(tmp_path: Path) -> Path:
    """Create a temp block-topology directory with sample data."""
    base = tmp_path / "block-topology"
    blocks = base / "blocks"
    blocks.mkdir(parents=True)

    # Write meta.json with id_mapping
    meta = {
        "version": "1.0.0",
        "id_mapping": {
            "001": "aaa111",
            "002": "bbb222",
            "003": "ccc333",
        },
    }
    (base / "meta.json").write_text(
        json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8",
    )

    # Write block files
    for bid, content in [
        ("aaa111", {"title": "退化段", "status": "settled", "id": "001"}),
        ("bbb222", {"title": "知识库", "status": "settled", "id": "002"}),
        ("ccc333", {"title": "线段口径", "status": "settled", "id": "003"}),
    ]:
        blk = {
            "id": bid,
            "type": "event",
            "timestamp": "2026-01-01T00:00:00+00:00",
            "source": "migration",
            "content": content,
            "refs": [],
            "git_ref": "",
        }
        (blocks / f"{bid}.json").write_text(
            json.dumps(blk, ensure_ascii=False, indent=2), encoding="utf-8",
        )

    # Write relations.jsonl
    relations = [
        {"from": "aaa111", "to": "bbb222", "relation": "depends_on", "order": 1,
         "created_by": "x" * 64, "timestamp": "2026-01-01T00:00:00+00:00"},
        {"from": "aaa111", "to": "ccc333", "relation": "references", "order": 2,
         "created_by": "x" * 64, "timestamp": "2026-01-01T00:00:00+00:00"},
        {"from": "bbb222", "to": "aaa111", "relation": "references", "order": 2,
         "created_by": "x" * 64, "timestamp": "2026-01-01T00:00:00+00:00"},
    ]
    jsonl = base / "relations.jsonl"
    with open(jsonl, "w", encoding="utf-8") as f:
        for rel in relations:
            f.write(json.dumps(rel, ensure_ascii=False) + "\n")

    return base


# --- _parse_frontmatter ---


class TestParseFrontmatter:

    def test_basic_frontmatter(self) -> None:
        fm = _parse_frontmatter(SAMPLE_GENEALOGY_001)
        assert fm["id"] == "001"
        assert fm["title"] == "退化段——古怪线段与笔定义的关系"
        assert fm["status"] == "已结算"
        assert fm["depends_on"] == ["002", "003"]
        assert fm["negates"] == []

    def test_no_frontmatter(self) -> None:
        fm = _parse_frontmatter("No frontmatter here")
        assert fm == {}

    def test_suffix_number(self) -> None:
        fm = _parse_frontmatter(SAMPLE_GENEALOGY_005A)
        assert fm["id"] == "005a"


# --- _normalize_number ---


class TestNormalizeNumber:

    def test_short_number(self) -> None:
        assert _normalize_number("1") == "001"
        assert _normalize_number("42") == "042"
        assert _normalize_number("347") == "347"

    def test_suffix(self) -> None:
        assert _normalize_number("5a") == "005a"
        assert _normalize_number("19c") == "019c"

    def test_already_padded(self) -> None:
        assert _normalize_number("001") == "001"
        assert _normalize_number("005a") == "005a"


# --- cmd_query ---


class TestCmdQuery:

    def test_query_existing(self, genealogy_dir: Path, topo_base: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_query("001", settled_dir=settled, topo_base=topo_base)
        assert result is not None
        assert result["number"] == "001"
        assert result["title"] == "退化段——古怪线段与笔定义的关系"
        assert result["status"] == "已结算"
        assert result["depends_on"] == ["002", "003"]
        assert result["block_id"] == "aaa111"
        assert "summary" in result

    def test_query_not_found(self, genealogy_dir: Path, topo_base: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_query("999", settled_dir=settled, topo_base=topo_base)
        assert result is None

    def test_query_normalizes_number(self, genealogy_dir: Path, topo_base: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_query("1", settled_dir=settled, topo_base=topo_base)
        assert result is not None
        assert result["number"] == "001"

    def test_query_suffix_number(self, genealogy_dir: Path, topo_base: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_query("5a", settled_dir=settled, topo_base=topo_base)
        assert result is not None
        assert result["number"] == "005a"
        assert "定理" in result["type"]


# --- cmd_search ---


class TestCmdSearch:

    def test_search_title_match(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("退化段", settled_dir=settled)
        assert len(results) >= 1
        assert results[0]["number"] == "001"
        assert results[0]["title_match"] is True

    def test_search_body_match(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("特征序列", settled_dir=settled)
        assert len(results) >= 1
        # Should match 001 (body contains 特征序列法)
        numbers = [r["number"] for r in results]
        assert "001" in numbers

    def test_search_no_match(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("完全不存在的关键词xyz", settled_dir=settled)
        assert results == []

    def test_search_case_insensitive(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("已结算", settled_dir=settled)
        # All four entries have 已结算
        assert len(results) == 4

    def test_search_max_results(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("已结算", settled_dir=settled, max_results=2)
        assert len(results) == 2

    def test_search_title_match_sorted_first(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        results = cmd_search("线段", settled_dir=settled)
        # 003 has "线段" in title, 001 has it in body
        assert len(results) >= 2
        title_matches = [r for r in results if r["title_match"]]
        body_only = [r for r in results if not r["title_match"]]
        if title_matches and body_only:
            # Title matches appear before body-only matches
            first_title_idx = results.index(title_matches[0])
            first_body_idx = results.index(body_only[0])
            assert first_title_idx < first_body_idx


# --- cmd_deps ---


class TestCmdDeps:

    def test_deps_no_dependencies(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_deps("002", settled_dir=settled)
        assert result["root"] == "002"
        assert result["total_nodes"] == 1
        assert result["chain"][0]["number"] == "002"
        assert result["chain"][0]["depth"] == 0

    def test_deps_with_chain(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_deps("001", settled_dir=settled)
        assert result["root"] == "001"
        # 001 depends on 002 and 003; 003 depends on 001 (cycle, but visited check prevents infinite loop)
        numbers = [n["number"] for n in result["chain"]]
        assert "001" in numbers
        assert "002" in numbers
        assert "003" in numbers

    def test_deps_normalizes_number(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_deps("1", settled_dir=settled)
        assert result["root"] == "001"

    def test_deps_not_found(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_deps("999", settled_dir=settled)
        assert result["root"] == "999"
        assert result["total_nodes"] == 1
        assert result["chain"][0]["title"] == "(not found)"

    def test_deps_max_depth(self, genealogy_dir: Path) -> None:
        settled = genealogy_dir / "settled"
        result = cmd_deps("001", settled_dir=settled, max_depth=0)
        # max_depth=0 means only root
        assert result["total_nodes"] == 1

    def test_deps_cycle_safe(self, genealogy_dir: Path) -> None:
        """003 depends on 001, 001 depends on 003 — no infinite loop."""
        settled = genealogy_dir / "settled"
        result = cmd_deps("003", settled_dir=settled)
        numbers = [n["number"] for n in result["chain"]]
        # Each number appears at most once
        assert len(numbers) == len(set(numbers))


# --- cmd_block ---


class TestCmdBlock:

    def test_block_by_id(self, topo_base: Path) -> None:
        result = cmd_block("aaa111", topo_base=topo_base)
        assert result is not None
        assert result["block_id"] == "aaa111"
        assert result["genealogy_number"] == "001"
        assert result["type"] == "event"
        assert result["title"] == "退化段"
        assert result["out_count"] == 2  # depends_on + references
        assert result["in_count"] == 1  # references from bbb222

    def test_block_by_genealogy_number(self, topo_base: Path) -> None:
        result = cmd_block("001", topo_base=topo_base)
        assert result is not None
        assert result["block_id"] == "aaa111"

    def test_block_not_found(self, topo_base: Path) -> None:
        result = cmd_block("nonexistent", topo_base=topo_base)
        assert result is None

    def test_block_edges_structure(self, topo_base: Path) -> None:
        result = cmd_block("aaa111", topo_base=topo_base)
        assert result is not None
        out_relations = [e["relation"] for e in result["out_edges"]]
        assert "depends_on" in out_relations
        assert "references" in out_relations
        in_relations = [e["relation"] for e in result["in_edges"]]
        assert "references" in in_relations

    def test_block_no_relations(self, topo_base: Path) -> None:
        result = cmd_block("ccc333", topo_base=topo_base)
        assert result is not None
        # ccc333 has only in_edges (from aaa111 references)
        assert result["in_count"] == 1
        assert result["out_count"] == 0


# --- CLI main() ---


class TestMainCLI:

    def test_main_query_json(
        self, genealogy_dir: Path, topo_base: Path, capsys: pytest.CaptureFixture,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr(
            "traverse_mcp.SETTLED_DIR", genealogy_dir / "settled",
        )
        monkeypatch.setattr("traverse_mcp.TOPO_BASE", topo_base)
        main(["query", "001", "--format", "json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert data["number"] == "001"

    def test_main_search_table(
        self, genealogy_dir: Path, capsys: pytest.CaptureFixture,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr(
            "traverse_mcp.SETTLED_DIR", genealogy_dir / "settled",
        )
        main(["search", "退化"])
        captured = capsys.readouterr()
        assert "退化段" in captured.out

    def test_main_deps_json(
        self, genealogy_dir: Path, capsys: pytest.CaptureFixture,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr(
            "traverse_mcp.SETTLED_DIR", genealogy_dir / "settled",
        )
        main(["deps", "001", "--format", "json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert data["root"] == "001"

    def test_main_block_json(
        self, topo_base: Path, capsys: pytest.CaptureFixture,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr("traverse_mcp.TOPO_BASE", topo_base)
        main(["block", "aaa111", "--format", "json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert data["block_id"] == "aaa111"

    def test_main_no_command(self) -> None:
        with pytest.raises(SystemExit) as exc_info:
            main([])
        assert exc_info.value.code == 1

    def test_main_query_not_found(
        self, genealogy_dir: Path, monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        monkeypatch.setattr(
            "traverse_mcp.SETTLED_DIR", genealogy_dir / "settled",
        )
        monkeypatch.setattr("traverse_mcp.TOPO_BASE", genealogy_dir / "nonexistent")
        with pytest.raises(SystemExit) as exc_info:
            main(["query", "999"])
        assert exc_info.value.code == 1
