"""
谱系区块拓扑化——一次性迁移脚本

将 177 条已结算谱系 + dag.yaml 边升格为区块拓扑。

迁移逻辑：
1. 创建 genesis 区块（type=event, source=migration）
2. 扫描 settled/*.md → 每条谱系一个 event 区块（refs 一律为空）
3. 解析 dag.yaml edges → relations.jsonl（created_by = genesis block id）
4. 生成 meta.json（id_mapping + genesis_block_id）

编排者决断：
- migration 区块 refs 一律为空（关系全走边表）
- created_by 字段类型始终为 SHA256（genesis 区块 id）
- 幂等：相同内容产生相同 hash，重复运行不产生重复区块
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml

# Add project root to path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE,
    append_relation,
    compute_block_id,
    make_block,
    make_concept_id,
    make_relation,
    write_block,
    write_meta,
)


def _try_content_analysis(genealogy_id: str, text: str) -> dict | None:
    """Attempt content analysis on a genealogy file.

    Returns content_analysis dict if successful, None otherwise.
    Does not fail the migration if analysis is unavailable.
    """
    try:
        from scripts.concept_extractor import analyze_content
        ca = analyze_content(genealogy_id, text)
        return {
            "sections": [
                {"level": s.level, "number": s.number, "title": s.title}
                for s in ca.sections
            ],
            "concepts": [
                {"term": c.term, "definition": c.definition}
                for c in ca.concepts
            ],
            "references": list(ca.references),
            "new_concepts": [
                {"term": nc.term, "qualifier": nc.qualifier,
                 "definition": nc.definition}
                for nc in ca.new_concepts
            ],
            "modifications": [
                {"target_id": m.target_id, "target_desc": m.target_desc,
                 "modification": m.modification}
                for m in ca.modifications
            ],
            "conclusion_summary": ca.conclusion_summary,
        }
    except Exception:
        return None


def parse_frontmatter(text: str) -> dict:
    """Extract metadata from a Markdown file.

    Supports two formats:
    1. YAML frontmatter (--- delimited)
    2. Markdown bold metadata (**key**: value) at top of file
    """
    # Try YAML frontmatter first
    match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
    if match:
        return yaml.safe_load(match.group(1)) or {}

    # Fallback: parse Markdown bold metadata (**key**: value)
    # Key field name mapping (Chinese → English)
    KEY_MAP = {"前置": "depends_on"}

    result = {}
    for line in text.split("\n"):
        line = line.strip()
        if not line:
            continue
        # Skip heading lines
        if line.startswith("#"):
            continue
        # Match **key**: value pattern (supports Chinese and ASCII keys)
        bold_match = re.match(r"\*\*([^*]+)\*\*:\s*(.*)", line)
        if bold_match:
            key = bold_match.group(1).strip()
            value = bold_match.group(2).strip()
            # Map Chinese keys to English
            key = KEY_MAP.get(key, key)
            # Normalize known list fields
            if key in ("depends_on", "related", "negates", "negated_by",
                        "tensions_with"):
                # Could be comma-separated or bracket list
                if value.startswith("["):
                    try:
                        value = yaml.safe_load(value)
                    except Exception:
                        value = [v.strip().strip("'\"")
                                 for v in value.strip("[]").split(",")
                                 if v.strip()]
                elif "," in value:
                    value = [v.strip() for v in value.split(",") if v.strip()]
                elif value:
                    value = [value]
                else:
                    value = []
            result[key] = value
        else:
            # Stop parsing metadata when we hit non-metadata content
            if result:  # Only stop if we've already found some metadata
                break

    return result


def migrate_settled_files(
    settled_dir: Path,
    base: Path = DEFAULT_BASE,
    with_content_analysis: bool = False,
) -> tuple[dict[str, str], list[dict]]:
    """Migrate settled/*.md → event blocks.

    Args:
        settled_dir: Path to settled/ directory
        base: Block topology base directory
        with_content_analysis: If True, include content_analysis in block
            content for new files. Existing blocks are unchanged (idempotent).

    Returns:
        id_mapping: {old_genealogy_id: block_id}
        blocks: list of block dicts
    """
    id_mapping: dict[str, str] = {}
    blocks: list[dict] = []

    for md_file in sorted(settled_dir.glob("*.md")):
        text = md_file.read_text(encoding="utf-8")
        fm = parse_frontmatter(text)
        if not fm.get("id"):
            continue

        old_id = str(fm["id"])

        # Build content from frontmatter fields
        content = {}
        for key in ("id", "title", "status", "type", "date",
                     "negation_source", "negation_form",
                     "depends_on", "related", "negates", "negated_by",
                     "provenance", "topo_effect", "topo_executed_at",
                     "tensions_with"):
            if key in fm and fm[key] is not None:
                val = fm[key]
                # Normalize list fields: ensure string ids
                if isinstance(val, list):
                    val = [str(v) for v in val]
                else:
                    val = str(val) if not isinstance(val, str) else val
                content[key] = val

        # Add source file path
        content["source_file"] = str(md_file.relative_to(
            settled_dir.parent))

        # Content analysis for new files
        if with_content_analysis:
            ca = _try_content_analysis(old_id, text)
            if ca is not None:
                content["content_analysis"] = ca

        block = make_block(
            block_type="event",
            source="migration",
            content=content,
            refs=[],  # migration blocks: refs always empty
        )

        write_block(block, base)
        id_mapping[old_id] = block["id"]
        blocks.append(block)

    return id_mapping, blocks


def migrate_edges(
    dag_path: Path,
    id_mapping: dict[str, str],
    genesis_id: str,
    base: Path = DEFAULT_BASE,
) -> int:
    """Migrate dag.yaml edges → relations.jsonl.

    Returns count of relations written.
    """
    with open(dag_path, "r", encoding="utf-8") as f:
        dag = yaml.safe_load(f)

    edges = dag.get("edges", {})
    count = 0

    # depends_on: directed {from, to}
    for edge in edges.get("depends_on", []) or []:
        from_id = id_mapping.get(str(edge["from"]))
        to_id = id_mapping.get(str(edge["to"]))
        if from_id and to_id:
            rel = make_relation(from_id, to_id, "depends_on",
                                order=1, created_by=genesis_id)
            append_relation(rel, base)
            count += 1

    # negates: directed {from, to}, may have scope
    for edge in edges.get("negates", []) or []:
        from_id = id_mapping.get(str(edge["from"]))
        to_id = id_mapping.get(str(edge["to"]))
        if from_id and to_id:
            extra = {}
            if "scope" in edge:
                extra["scope"] = str(edge["scope"])
            rel = make_relation(from_id, to_id, "negates",
                                order=1, created_by=genesis_id,
                                **extra)
            append_relation(rel, base)
            count += 1

    # related: undirected {between: [a, b]}
    for edge in edges.get("related", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id = id_mapping.get(str(pair[0]))
            b_id = id_mapping.get(str(pair[1]))
            if a_id and b_id:
                rel = make_relation(a_id, b_id, "related",
                                    order=1, created_by=genesis_id)
                append_relation(rel, base)
                count += 1

    # tensions_with: undirected {between: [a, b]}, may have valid_until,
    # settled_by
    for edge in edges.get("tensions_with", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id = id_mapping.get(str(pair[0]))
            b_id = id_mapping.get(str(pair[1]))
            if a_id and b_id:
                extra = {}
                if "valid_until" in edge:
                    extra["valid_until"] = str(edge["valid_until"])
                if "settled_by" in edge:
                    extra["settled_by"] = str(edge["settled_by"])
                rel = make_relation(a_id, b_id, "tensions_with",
                                    order=1, created_by=genesis_id,
                                    **extra)
                append_relation(rel, base)
                count += 1

    # negated_by: directed {target, by}, may have scope
    for edge in edges.get("negated_by", []) or []:
        target_id = id_mapping.get(str(edge["target"]))
        by_id = id_mapping.get(str(edge["by"]))
        if target_id and by_id:
            extra = {}
            if "scope" in edge:
                extra["scope"] = str(edge["scope"])
            rel = make_relation(target_id, by_id, "negated_by",
                                order=1, created_by=genesis_id,
                                **extra)
            append_relation(rel, base)
            count += 1

    return count


def run_migration(
    project_root: Path | None = None,
    base: Path | None = None,
) -> dict:
    """Execute full migration.

    Returns meta dict with migration results.
    """
    if project_root is None:
        project_root = PROJECT_ROOT
    if base is None:
        base = project_root / ".chanlun" / "block-topology"

    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"
    dag_path = project_root / ".chanlun" / "genealogy" / "dag.yaml"

    print(f"[migration] Project root: {project_root}")
    print(f"[migration] Target: {base}")

    # Step 1: Migrate settled files → event blocks
    id_mapping, blocks = migrate_settled_files(settled_dir, base)
    print(f"[migration] Migrated {len(blocks)} genealogy records → blocks")

    # Step 2: Create genesis block (after migration, so count is known)
    genesis_content = {
        "action": "genesis",
        "migrated_from": "dag.yaml + settled/*.md",
        "description": "One-time migration from genealogy to block topology",
        "count": len(blocks),
    }
    genesis = make_block("event", "migration", genesis_content, refs=[])
    write_block(genesis, base)
    print(f"[migration] Genesis block: {genesis['id'][:16]}...")

    # Step 3: Migrate edges → relations
    relation_count = migrate_edges(dag_path, id_mapping,
                                   genesis["id"], base)
    print(f"[migration] Migrated {relation_count} edges → relations")

    # Step 4: Write meta.json
    meta = {
        "version": "1.0.0",
        "genesis_block_id": genesis["id"],
        "block_count": len(blocks) + 1,  # +1 for genesis
        "relation_count": relation_count,
        "id_mapping": id_mapping,
        "migrated_at": genesis["timestamp"],
    }
    write_meta(meta, base)
    print(f"[migration] meta.json written")
    print(f"[migration] Done. {meta['block_count']} blocks, "
          f"{relation_count} relations")

    return meta


if __name__ == "__main__":
    meta = run_migration()
    print(f"\nMigration complete.")
    print(f"  Blocks: {meta['block_count']}")
    print(f"  Relations: {meta['relation_count']}")
    print(f"  Genesis: {meta['genesis_block_id'][:16]}...")
