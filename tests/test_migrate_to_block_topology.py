"""Tests for scripts/migrate_to_block_topology.py — 迁移脚本"""

from __future__ import annotations

import hashlib
import json
import textwrap
from pathlib import Path

import pytest
import yaml

from scripts.block_topology import (
    list_blocks,
    read_all_relations,
    read_block,
    read_meta,
)
from scripts.migrate_to_block_topology import (
    migrate_edges,
    migrate_settled_files,
    parse_frontmatter,
    run_migration,
)


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

GENESIS_ID = hashlib.sha256(b"test-genesis").hexdigest()


def _write_md(path: Path, frontmatter: dict, body: str = "") -> None:
    """Write a markdown file with YAML frontmatter."""
    text = "---\n" + yaml.dump(frontmatter, allow_unicode=True) + "---\n" + body
    path.write_text(text, encoding="utf-8")


def _write_dag(path: Path, nodes: list[dict], edges: dict) -> None:
    """Write a minimal dag.yaml."""
    dag = {"nodes": nodes, "edges": edges}
    path.write_text(
        yaml.dump(dag, allow_unicode=True, default_flow_style=False),
        encoding="utf-8",
    )


def _read_relations(base: Path) -> list[dict]:
    """Read relations.jsonl from base directory."""
    jsonl = base / "relations.jsonl"
    if not jsonl.exists():
        return []
    lines = jsonl.read_text(encoding="utf-8").strip().splitlines()
    return [json.loads(line) for line in lines if line.strip()]


@pytest.fixture
def mock_project(tmp_path):
    """Create a minimal mock project structure for testing."""
    settled_dir = tmp_path / ".chanlun" / "genealogy" / "settled"
    settled_dir.mkdir(parents=True)

    files = {
        "001-test.md": {
            "id": "001",
            "title": "Test genealogy one",
            "status": "已结算",
            "type": "矛盾记录",
            "date": "2026-01-01",
            "depends_on": [],
            "related": ["002"],
            "negates": [],
            "negation_form": "",
        },
        "002-test.md": {
            "id": "002",
            "title": "Test genealogy two",
            "status": "已结算",
            "type": "概念分离",
            "date": "2026-01-02",
            "depends_on": ["001"],
            "related": ["001"],
            "negates": [],
            "negation_form": "expansion",
        },
        "003-test.md": {
            "id": "003",
            "title": "Test genealogy three",
            "status": "已结算",
            "type": "语法记录",
            "date": "2026-01-03",
            "depends_on": ["001", "002"],
            "related": [],
            "negates": ["001"],
            "negation_form": "separation",
            "topo_effect": "freeze:001",
        },
    }

    for filename, fm in files.items():
        content = "---\n" + yaml.dump(fm, allow_unicode=True) + "---\n\n"
        content += f"# {fm['title']}\n\nBody text.\n"
        (settled_dir / filename).write_text(content, encoding="utf-8")

    dag = {
        "nodes": [
            {"id": "001", "title": "Test one", "status": "已结算",
             "type": "矛盾记录", "file": "settled/001-test.md"},
            {"id": "002", "title": "Test two", "status": "已结算",
             "type": "概念分离", "file": "settled/002-test.md"},
            {"id": "003", "title": "Test three", "status": "已结算",
             "type": "语法记录", "file": "settled/003-test.md"},
        ],
        "edges": {
            "depends_on": [
                {"from": "002", "to": "001"},
                {"from": "003", "to": "001"},
                {"from": "003", "to": "002"},
            ],
            "negates": [
                {"from": "003", "to": "001", "scope": "定义层"},
            ],
            "related": [
                {"between": ["001", "002"]},
            ],
            "tensions_with": [
                {"between": ["002", "003"], "valid_until": "004"},
            ],
            "negated_by": [
                {"target": "001", "by": "003", "scope": "全局"},
            ],
        },
    }

    dag_path = tmp_path / ".chanlun" / "genealogy" / "dag.yaml"
    dag_path.write_text(yaml.dump(dag, allow_unicode=True), encoding="utf-8")

    return tmp_path


@pytest.fixture
def mock_base(tmp_path):
    """Create a temporary block-topology output directory."""
    base = tmp_path / ".chanlun" / "block-topology"
    base.mkdir(parents=True)
    (base / "blocks").mkdir()
    return base


# ---------------------------------------------------------------------------
# parse_frontmatter
# ---------------------------------------------------------------------------


def test_parse_frontmatter():
    """YAML frontmatter is correctly extracted."""
    text = textwrap.dedent("""\
        ---
        id: "042"
        title: "Hook network pattern"
        status: "已结算"
        ---

        # Body
        """)
    fm = parse_frontmatter(text)
    assert fm["id"] == "042"
    assert fm["title"] == "Hook network pattern"
    assert fm["status"] == "已结算"


def test_parse_frontmatter_empty():
    """No frontmatter → empty dict."""
    assert parse_frontmatter("# Just a heading\n\nSome text.\n") == {}
    assert parse_frontmatter("") == {}


# ---------------------------------------------------------------------------
# migrate_settled_files
# ---------------------------------------------------------------------------


def test_migrate_single_genealogy(mock_project, mock_base):
    """Each settled file becomes one event block with refs=[]."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    id_mapping, blocks = migrate_settled_files(settled_dir, mock_base)

    assert len(blocks) == 3
    assert len(id_mapping) == 3

    for block in blocks:
        assert block["type"] == "event"
        assert block["source"] == "migration"
        assert block["refs"] == []

    # block files actually written to disk
    block_files = list((mock_base / "blocks").glob("*.json"))
    assert len(block_files) == 3


def test_migrate_creates_correct_content(mock_project, mock_base):
    """Block content contains original frontmatter fields."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    _, blocks = migrate_settled_files(settled_dir, mock_base)

    block_003 = next(b for b in blocks if b["content"].get("id") == "003")
    content = block_003["content"]

    assert content["title"] == "Test genealogy three"
    assert content["status"] == "已结算"
    assert content["type"] == "语法记录"
    assert content["topo_effect"] == "freeze:001"
    assert content["negation_form"] == "separation"
    assert content["depends_on"] == ["001", "002"]
    assert "source_file" in content


# ---------------------------------------------------------------------------
# migrate_edges
# ---------------------------------------------------------------------------


def test_migrate_edges_depends_on(mock_project, mock_base):
    """depends_on edges become relations with order=1."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"

    id_mapping, _ = migrate_settled_files(settled_dir, mock_base)
    count = migrate_edges(dag_path, id_mapping, GENESIS_ID, mock_base)

    rels = _read_relations(mock_base)
    depends = [r for r in rels if r["relation"] == "depends_on"]
    assert len(depends) == 3  # 002→001, 003→001, 003→002

    for r in depends:
        assert r["order"] == 1
        assert r["created_by"] == GENESIS_ID


def test_migrate_edges_negates(mock_project, mock_base):
    """negates edges become relations, scope preserved."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"

    id_mapping, _ = migrate_settled_files(settled_dir, mock_base)
    migrate_edges(dag_path, id_mapping, GENESIS_ID, mock_base)

    rels = _read_relations(mock_base)
    negates = [r for r in rels if r["relation"] == "negates"]
    assert len(negates) == 1
    assert negates[0]["from"] == id_mapping["003"]
    assert negates[0]["to"] == id_mapping["001"]
    assert negates[0]["scope"] == "定义层"


def test_migrate_edges_related(mock_project, mock_base):
    """related edges: between [a, b] → one relation a→b."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"

    id_mapping, _ = migrate_settled_files(settled_dir, mock_base)
    migrate_edges(dag_path, id_mapping, GENESIS_ID, mock_base)

    rels = _read_relations(mock_base)
    related = [r for r in rels if r["relation"] == "related"]
    assert len(related) == 1
    assert related[0]["from"] == id_mapping["001"]
    assert related[0]["to"] == id_mapping["002"]


def test_migrate_edges_tensions_with(mock_project, mock_base):
    """tensions_with preserves valid_until field."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"

    id_mapping, _ = migrate_settled_files(settled_dir, mock_base)
    migrate_edges(dag_path, id_mapping, GENESIS_ID, mock_base)

    rels = _read_relations(mock_base)
    tensions = [r for r in rels if r["relation"] == "tensions_with"]
    assert len(tensions) == 1
    assert tensions[0]["valid_until"] == "004"
    assert tensions[0]["from"] == id_mapping["002"]
    assert tensions[0]["to"] == id_mapping["003"]


def test_migrate_edges_negated_by(mock_project, mock_base):
    """negated_by edges: target→by mapping, scope preserved."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"

    id_mapping, _ = migrate_settled_files(settled_dir, mock_base)
    migrate_edges(dag_path, id_mapping, GENESIS_ID, mock_base)

    rels = _read_relations(mock_base)
    negated_by = [r for r in rels if r["relation"] == "negated_by"]
    assert len(negated_by) == 1
    assert negated_by[0]["from"] == id_mapping["001"]
    assert negated_by[0]["to"] == id_mapping["003"]
    assert negated_by[0]["scope"] == "全局"


# ---------------------------------------------------------------------------
# id_mapping completeness
# ---------------------------------------------------------------------------


def test_id_mapping_complete(mock_project, mock_base):
    """id_mapping covers all migrated old ids with valid SHA256 values."""
    settled_dir = mock_project / ".chanlun" / "genealogy" / "settled"
    id_mapping, blocks = migrate_settled_files(settled_dir, mock_base)

    assert set(id_mapping.keys()) == {"001", "002", "003"}
    assert len(id_mapping) == len(blocks)

    # Each value is a unique 64-char hex string (SHA256)
    assert len(set(id_mapping.values())) == 3
    for v in id_mapping.values():
        assert len(v) == 64
        assert all(c in "0123456789abcdef" for c in v)


# ---------------------------------------------------------------------------
# run_migration (end-to-end)
# ---------------------------------------------------------------------------


def test_run_migration_full(mock_project):
    """Full migration: genesis + blocks + relations + meta.json."""
    base = mock_project / ".chanlun" / "block-topology"
    meta = run_migration(project_root=mock_project, base=base)

    # meta.json written and readable
    meta_path = base / "meta.json"
    assert meta_path.exists()
    meta_disk = json.loads(meta_path.read_text(encoding="utf-8"))

    assert meta_disk["version"] == "1.0.0"
    assert "genesis_block_id" in meta_disk
    # block_count = 3 genealogy + 1 genesis (final) = 4
    assert meta_disk["block_count"] == 4
    assert meta_disk["relation_count"] > 0
    assert "001" in meta_disk["id_mapping"]
    assert "002" in meta_disk["id_mapping"]
    assert "003" in meta_disk["id_mapping"]

    # blocks directory populated (>=4: genesis_initial + genesis_final + 3 genealogy)
    block_files = list((base / "blocks").glob("*.json"))
    assert len(block_files) >= 4

    # relations file populated
    rels = _read_relations(base)
    assert len(rels) > 0
    # All expected relation types present
    rel_types = {r["relation"] for r in rels}
    assert "depends_on" in rel_types
    assert "negates" in rel_types
    assert "related" in rel_types
    assert "tensions_with" in rel_types
    assert "negated_by" in rel_types


# ---------------------------------------------------------------------------
# run_incremental_migration (626号)
# ---------------------------------------------------------------------------


def _add_settled(settled_dir: Path, filename: str, fm: dict) -> None:
    content = "---\n" + yaml.dump(fm, allow_unicode=True) + "---\n\n# body\n"
    (settled_dir / filename).write_text(content, encoding="utf-8")


def test_incremental_appends_without_destroying(mock_project, mock_base):
    """626号: incremental entry appends new ids, preserving existing meta
    fields (content_enrichment) and every prior id_mapping key."""
    from scripts.block_topology import read_meta, write_meta
    from scripts.migrate_to_block_topology import run_incremental_migration, run_migration

    # Seed via full migration, then simulate real meta state: enrichment +
    # last_mapped_genealogy (which full migration does not itself write).
    run_migration(mock_project, mock_base)
    meta = read_meta(mock_base)
    meta["content_enrichment"] = {"blocks_created": 7, "concepts_defined": 42}
    meta["last_mapped_genealogy"] = 3
    write_meta(meta, mock_base)
    prev_keys = set(meta["id_mapping"])
    prev_relcount = meta["relation_count"]

    # New settled genealogy appears (004), plus a merged-node record whose
    # frontmatter id is a range but filename id is 005.
    settled = mock_project / ".chanlun" / "genealogy" / "settled"
    _add_settled(settled, "004-new.md", {
        "id": "004", "title": "four", "status": "已结算",
        "type": "矛盾记录", "date": "2026-02-01", "depends_on": ["003"],
    })
    _add_settled(settled, "005-006-merged.md", {
        "id": "005-006", "title": "merged", "status": "已结算",
        "type": "回溯结算", "date": "2026-02-02", "related": ["004"],
    })
    # Add a dag edge for 004 so edge migration has something to do.
    dag_path = mock_project / ".chanlun" / "genealogy" / "dag.yaml"
    dag = yaml.safe_load(dag_path.read_text(encoding="utf-8"))
    dag["edges"].setdefault("depends_on", []).append({"from": "004", "to": "003"})
    dag_path.write_text(yaml.dump(dag, allow_unicode=True), encoding="utf-8")

    result = run_incremental_migration(mock_project, mock_base)

    assert set(result["migrated"]) == {"004", "005"}  # keyed by FILENAME id
    after = read_meta(mock_base)
    # append, not replace: every old key survives
    assert prev_keys <= set(after["id_mapping"])
    # content_enrichment preserved verbatim
    assert after["content_enrichment"] == {"blocks_created": 7, "concepts_defined": 42}
    # merged node keyed under filename id 005, block content keeps range id
    assert "005" in after["id_mapping"]
    blk = read_block(after["id_mapping"]["005"], mock_base)
    assert blk["content"]["id"] == "005-006"
    # last_mapped advanced (005 is not pure-numeric, so 004 drives it)
    assert after["last_mapped_genealogy"] == 5
    # edge for 004 migrated (mock base has no LFS pointer)
    assert result["relations_written"] >= 1
    assert after["relation_count"] == prev_relcount + result["relations_written"]


def test_incremental_idempotent_and_skips_malformed(mock_project, mock_base):
    """Rerun migrates nothing; a file with malformed frontmatter is skipped
    (not crashing) and reported via the still-unmapped diff."""
    from scripts.block_topology import read_meta, write_meta
    from scripts.migrate_to_block_topology import run_incremental_migration, run_migration

    run_migration(mock_project, mock_base)
    settled = mock_project / ".chanlun" / "genealogy" / "settled"

    # Malformed frontmatter: a flow sequence with inline prose (illegal YAML) —
    # the exact 650/651 failure mode. Written raw to bypass yaml.dump.
    (settled / "004-bad.md").write_text(
        "---\nid: \"004\"\nrelated: ['003'(prose here), '002']\n---\n\n# body\n",
        encoding="utf-8",
    )

    result = run_incremental_migration(mock_project, mock_base)
    assert result["skipped_unparseable_frontmatter"] == ["004"]
    assert "004" not in read_meta(mock_base)["id_mapping"]

    # Idempotent: nothing new to migrate on rerun (004 still unparseable).
    result2 = run_incremental_migration(mock_project, mock_base)
    assert result2["blocks_created"] == 0
