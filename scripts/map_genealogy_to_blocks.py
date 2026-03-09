#!/usr/bin/env python
"""将未映射的谱系条目创建为 block-topology 区块。

对每个指定的谱系编号：
1. 从 settled/*.md 读取完整文本
2. 解析 frontmatter 提取元数据
3. 通过 block_ops.create_block() 创建 content-addressed 区块
4. 更新 meta.json 的 id_mapping
5. 从 dag.yaml 提取相关边 → 写入 relations.jsonl

幂等：重复运行不产生重复区块（content-addressed + 关系去重）。

用法:
  python scripts/map_genealogy_to_blocks.py 384 386 387 ...
  python scripts/map_genealogy_to_blocks.py --all-unmapped
  python scripts/map_genealogy_to_blocks.py --dry-run 384 386
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import yaml

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_ops import create_block
from scripts.block_topology import (
    append_relation,
    make_relation,
    read_all_relations,
    read_meta,
    write_meta,
)


def parse_frontmatter(text: str) -> dict:
    """Extract YAML frontmatter from markdown text."""
    match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
    if match:
        return yaml.safe_load(match.group(1)) or {}
    return {}


def _relation_dedup_key(rel: dict) -> tuple:
    """Build a dedup key for a relation."""
    return (
        rel.get("from", ""),
        rel.get("to", ""),
        rel.get("relation", ""),
        rel.get("order", 0),
    )


def _build_existing_keys(base: Path) -> set[tuple]:
    """Load existing relations and return their dedup keys."""
    existing = read_all_relations(base)
    keys = set()
    for r in existing:
        if r.get("from") and r.get("to") and r.get("relation"):
            keys.add(_relation_dedup_key(r))
    return keys


def find_settled_file(settled_dir: Path, genealogy_id: str) -> Path | None:
    """Find the settled .md file for a genealogy number."""
    pattern = f"{genealogy_id}-*.md"
    matches = list(settled_dir.glob(pattern))
    if len(matches) == 1:
        return matches[0]
    if len(matches) > 1:
        # Prefer exact prefix match
        for m in matches:
            if m.name.startswith(f"{genealogy_id}-"):
                return m
        return matches[0]
    return None


def load_dag_edges(dag_path: Path, target_ids: set[str]) -> list[dict]:
    """Extract depends_on edges from dag.yaml involving target ids.

    Returns list of {edge_type, from, to} dicts.
    """
    dag = yaml.safe_load(dag_path.read_text(encoding="utf-8"))
    edges_section = dag.get("edges", {})
    result = []

    for edge_type, edge_list in edges_section.items():
        if not edge_list:
            continue
        for e in edge_list:
            ids_in_edge = set()
            if "from" in e:
                ids_in_edge.add(str(e["from"]))
            if "to" in e:
                ids_in_edge.add(str(e["to"]))
            if "between" in e:
                for b in e["between"]:
                    ids_in_edge.add(str(b))
            if "target" in e:
                ids_in_edge.add(str(e["target"]))
            if "by" in e:
                ids_in_edge.add(str(e["by"]))

            if ids_in_edge & target_ids:
                result.append({"edge_type": edge_type, **e})

    return result


def map_genealogy_numbers(
    numbers: list[str],
    project_root: Path | None = None,
    dry_run: bool = False,
) -> dict:
    """Map genealogy numbers to block-topology blocks.

    Returns summary dict with results.
    """
    if project_root is None:
        project_root = PROJECT_ROOT

    base = project_root / ".chanlun" / "block-topology"
    blocks_dir = base / "blocks"
    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"
    dag_path = project_root / ".chanlun" / "genealogy" / "dag.yaml"

    meta = read_meta(base)
    if meta is None:
        return {"error": "meta.json not found"}

    id_mapping = meta.get("id_mapping", {})
    target_ids = set(numbers)

    blocks_created = 0
    blocks_skipped = 0
    relations_written = 0
    new_mappings: dict[str, str] = {}
    errors: list[str] = []

    # Phase 1: Create blocks for each number
    for num in numbers:
        if num in id_mapping:
            print(f"  [{num}] already mapped -> {id_mapping[num][:16]}...")
            blocks_skipped += 1
            continue

        md_file = find_settled_file(settled_dir, num)
        if md_file is None:
            errors.append(f"{num}: settled file not found")
            continue

        text = md_file.read_text(encoding="utf-8")
        fm = parse_frontmatter(text)

        # Build metadata from frontmatter
        metadata = {}

        # genealogy_id (top-level, matching existing 383/385 format)
        metadata["genealogy_id"] = str(fm.get("id", num))

        # title
        title = fm.get("title", "")
        if not title:
            # Extract from first heading
            heading_match = re.search(r"^#\s+(.+)$", text, re.MULTILINE)
            if heading_match:
                title = heading_match.group(1).strip()
        if title:
            metadata["title"] = title

        # type
        if fm.get("type"):
            metadata["type"] = str(fm["type"])

        # depends_on (from frontmatter)
        deps = fm.get("depends_on", [])
        if isinstance(deps, list) and deps:
            metadata["depends_on"] = [str(d).strip() for d in deps]
        # Also check 'parent' field (used by some files like 401)
        parent = fm.get("parent", [])
        if isinstance(parent, list) and parent and not deps:
            metadata["depends_on"] = [str(p).strip() for p in parent]

        # content.source_file and content.id
        rel_path = f"settled/{md_file.name}"
        metadata["content"] = {
            "source_file": rel_path,
            "id": str(num),
        }

        if dry_run:
            print(f"  [{num}] would create block from {md_file.name}")
            print(f"    title: {title[:60]}...")
            print(f"    type: {fm.get('type', 'unknown')}")
            print(f"    depends_on: {metadata.get('depends_on', [])}")
            blocks_created += 1
            continue

        block_id = create_block(
            md_file,
            blocks_dir,
            metadata=metadata,
        )
        new_mappings[num] = block_id
        id_mapping[num] = block_id
        blocks_created += 1
        print(f"  [{num}] created block {block_id[:16]}...")

    if dry_run:
        return {
            "mode": "dry_run",
            "blocks_would_create": blocks_created,
            "blocks_skipped": blocks_skipped,
            "errors": errors,
        }

    # Phase 2: Update meta.json with new mappings
    if new_mappings:
        meta["id_mapping"].update(new_mappings)
        meta["block_count"] = meta.get("block_count", 0) + len(new_mappings)
        write_meta(meta, base)
        print(f"  [meta] updated id_mapping with {len(new_mappings)} entries")

    # Phase 3: Create relations from dag.yaml edges
    if new_mappings:
        existing_keys = _build_existing_keys(base)
        dag_edges = load_dag_edges(dag_path, target_ids)

        # Use genesis block id as created_by for migration-style relations
        genesis_id = meta.get("genesis_block_id", "")

        for edge in dag_edges:
            edge_type = edge["edge_type"]

            if edge_type == "depends_on":
                from_id_gen = str(edge["from"])
                to_id_gen = str(edge["to"])
                from_block = id_mapping.get(from_id_gen)
                to_block = id_mapping.get(to_id_gen)
                if from_block and to_block:
                    # Use the from-block as created_by (the new block
                    # that declares this dependency)
                    created_by = from_block
                    rel = make_relation(
                        from_block, to_block, "depends_on",
                        order=1, created_by=created_by,
                    )
                    key = _relation_dedup_key(rel)
                    if key not in existing_keys:
                        append_relation(rel, base)
                        existing_keys.add(key)
                        relations_written += 1

            elif edge_type == "negates":
                from_block = id_mapping.get(str(edge["from"]))
                to_block = id_mapping.get(str(edge["to"]))
                if from_block and to_block:
                    extra = {}
                    if "scope" in edge:
                        extra["scope"] = str(edge["scope"])
                    rel = make_relation(
                        from_block, to_block, "negates",
                        order=1, created_by=from_block, **extra,
                    )
                    key = _relation_dedup_key(rel)
                    if key not in existing_keys:
                        append_relation(rel, base)
                        existing_keys.add(key)
                        relations_written += 1

            elif edge_type == "related":
                pair = edge.get("between", [])
                if len(pair) == 2:
                    a_block = id_mapping.get(str(pair[0]))
                    b_block = id_mapping.get(str(pair[1]))
                    if a_block and b_block:
                        rel = make_relation(
                            a_block, b_block, "related",
                            order=1, created_by=genesis_id,
                        )
                        key = _relation_dedup_key(rel)
                        if key not in existing_keys:
                            append_relation(rel, base)
                            existing_keys.add(key)
                            relations_written += 1

            elif edge_type == "tensions_with":
                pair = edge.get("between", [])
                if len(pair) == 2:
                    a_block = id_mapping.get(str(pair[0]))
                    b_block = id_mapping.get(str(pair[1]))
                    if a_block and b_block:
                        extra = {}
                        if "valid_until" in edge:
                            extra["valid_until"] = str(edge["valid_until"])
                        if "settled_by" in edge:
                            extra["settled_by"] = str(edge["settled_by"])
                        rel = make_relation(
                            a_block, b_block, "tensions_with",
                            order=1, created_by=genesis_id, **extra,
                        )
                        key = _relation_dedup_key(rel)
                        if key not in existing_keys:
                            append_relation(rel, base)
                            existing_keys.add(key)
                            relations_written += 1

            elif edge_type == "negated_by":
                target_block = id_mapping.get(str(edge["target"]))
                by_block = id_mapping.get(str(edge["by"]))
                if target_block and by_block:
                    extra = {}
                    if "scope" in edge:
                        extra["scope"] = str(edge["scope"])
                    rel = make_relation(
                        target_block, by_block, "negated_by",
                        order=1, created_by=genesis_id, **extra,
                    )
                    key = _relation_dedup_key(rel)
                    if key not in existing_keys:
                        append_relation(rel, base)
                        existing_keys.add(key)
                        relations_written += 1

        if relations_written:
            meta["relation_count"] = meta.get("relation_count", 0) + relations_written
            write_meta(meta, base)
            print(f"  [relations] wrote {relations_written} relations")

    return {
        "mode": "write",
        "blocks_created": blocks_created,
        "blocks_skipped": blocks_skipped,
        "relations_written": relations_written,
        "new_mappings": {k: v[:16] + "..." for k, v in new_mappings.items()},
        "errors": errors,
    }


def find_all_unmapped(project_root: Path | None = None) -> list[str]:
    """Find all genealogy numbers that exist in settled/ but not in id_mapping."""
    if project_root is None:
        project_root = PROJECT_ROOT

    base = project_root / ".chanlun" / "block-topology"
    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"

    meta = read_meta(base)
    if meta is None:
        return []

    id_mapping = meta.get("id_mapping", {})
    unmapped = []

    for md_file in sorted(settled_dir.glob("*.md")):
        match = re.match(r"^(\d{3}[a-z]?)-", md_file.name)
        if not match:
            continue
        gen_id = match.group(1)
        # Normalize: strip leading zeros for comparison
        normalized = str(int(gen_id)) if gen_id.isdigit() else gen_id
        if gen_id not in id_mapping and normalized not in id_mapping:
            unmapped.append(gen_id)

    return unmapped


def main():
    parser = argparse.ArgumentParser(
        description="Map genealogy numbers to block-topology blocks"
    )
    parser.add_argument(
        "numbers",
        nargs="*",
        help="Genealogy numbers to map (e.g. 384 386 387)",
    )
    parser.add_argument(
        "--all-unmapped",
        action="store_true",
        help="Map all unmapped genealogy numbers",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Only show what would be done",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output as JSON",
    )
    args = parser.parse_args()

    if args.all_unmapped:
        numbers = find_all_unmapped()
        if not numbers:
            print("No unmapped genealogy numbers found.")
            return
        print(f"Found {len(numbers)} unmapped: {', '.join(numbers)}")
    elif args.numbers:
        numbers = args.numbers
    else:
        parser.error("Provide genealogy numbers or use --all-unmapped")
        return

    result = map_genealogy_numbers(numbers, dry_run=args.dry_run)

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        mode = result.get("mode", "unknown")
        if mode == "dry_run":
            print(f"\n[dry-run] Would create {result['blocks_would_create']} blocks")
            print(f"[dry-run] Skipped {result['blocks_skipped']} (already mapped)")
        else:
            print(f"\n[done] Created {result['blocks_created']} blocks")
            print(f"[done] Skipped {result['blocks_skipped']} (already mapped)")
            print(f"[done] Wrote {result['relations_written']} relations")
        if result.get("errors"):
            print(f"[errors] {len(result['errors'])} errors:")
            for e in result["errors"]:
                print(f"  - {e}")


if __name__ == "__main__":
    main()
