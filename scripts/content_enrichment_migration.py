"""content_enrichment_migration.py — 全量内容级迁移

对已有 settled/*.md 创建 content enrichment rewrite 区块：
1. 读取完整文件 → analyze_content() 获取 ContentAnalysis
2. 通过 meta.json id_mapping 找到原 event 区块 SHA
3. 创建 rewrite 区块 + defines/modifies/references 关系
4. 更新 meta.json

幂等：
- 区块级：compute_block_id 基于确定性内容 → 重复运行不产生重复区块。
- 关系级：迁移前加载已有关系做去重，避免重复追加。
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE,
    append_relation,
    compute_block_id,
    make_block,
    make_concept_id,
    make_relation,
    read_all_relations,
    read_meta,
    write_block,
    write_meta,
)
from scripts.concept_extractor import analyze_content


def _relation_dedup_key(rel: dict) -> tuple:
    """Build a dedup key for a relation (from, to, relation, order)."""
    return (rel["from"], rel["to"], rel["relation"], rel.get("order", 0))


def _build_existing_keys(base: Path) -> set[tuple]:
    """Load existing relations and return their dedup keys."""
    try:
        existing = read_all_relations(base)
    except FileNotFoundError:
        return set()
    return {_relation_dedup_key(r) for r in existing}


def _content_analysis_to_dict(ca) -> dict:
    """Convert ContentAnalysis to JSON-serializable dict."""
    return {
        "genealogy_id": ca.genealogy_id,
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
            {"term": nc.term, "qualifier": nc.qualifier, "definition": nc.definition}
            for nc in ca.new_concepts
        ],
        "modifications": [
            {"target_id": m.target_id, "target_desc": m.target_desc,
             "modification": m.modification, "kind": m.kind}
            for m in ca.modifications
        ],
        "negations": [
            {"target_id": n.target_id, "target_desc": n.target_desc,
             "negation_type": n.negation_type}
            for n in ca.negations
        ],
        "frontmatter_negation": ca.frontmatter_negation,
        "conclusion_summary": ca.conclusion_summary,
    }


def _analysis_stats(ca) -> dict:
    """Return summary stats for dry-run output."""
    return {
        "genealogy_id": ca.genealogy_id,
        "sections": len(ca.sections),
        "concepts": len(ca.concepts),
        "references": len(ca.references),
        "new_concepts": len(ca.new_concepts),
        "modifications": len(ca.modifications),
        "negations": len(ca.negations),
        "has_conclusion": bool(ca.conclusion_summary),
    }


def enrich_single_file(
    md_file: Path,
    id_mapping: dict[str, str],
    base: Path,
    dry_run: bool = False,
    existing_keys: set[tuple] | None = None,
) -> dict | None:
    """Process a single genealogy file for content enrichment.

    Returns dict with stats/results, or None if file has no id_mapping entry.
    existing_keys: set of relation dedup keys to skip already-written relations.
    """
    text = md_file.read_text(encoding="utf-8")

    # Extract genealogy id from filename: NNN-title.md or NNNx-title.md
    import re
    fname_match = re.match(r'^(\d{3}[a-z]?)-', md_file.name)
    if not fname_match:
        return None
    genealogy_id = fname_match.group(1)

    # Find original event block SHA
    original_sha = id_mapping.get(genealogy_id)
    if original_sha is None:
        # Try with leading zeros stripped
        try:
            original_sha = id_mapping.get(str(int(genealogy_id)))
        except ValueError:
            pass
    if original_sha is None:
        return {"genealogy_id": genealogy_id, "skipped": "no_block_mapping"}

    # Run content analysis
    ca = analyze_content(genealogy_id, text)

    if dry_run:
        stats = _analysis_stats(ca)
        stats["original_block"] = original_sha[:16] + "..."
        return stats

    # Create rewrite block for content enrichment
    ca_dict = _content_analysis_to_dict(ca)
    rewrite_content = {
        "action": "content_enrichment",
        "original_block": original_sha,
        "content_analysis": ca_dict,
    }
    rewrite_block = make_block(
        block_type="rewrite",
        source="migration",
        content=rewrite_content,
        refs=[original_sha],
    )
    write_block(rewrite_block, base)

    relations_written = 0

    def _write_rel(rel: dict) -> None:
        nonlocal relations_written
        if existing_keys is not None:
            key = _relation_dedup_key(rel)
            if key in existing_keys:
                return
            existing_keys.add(key)
        append_relation(rel, base)
        relations_written += 1

    # records relation: rewrite → original
    records_rel = make_relation(
        from_id=rewrite_block["id"],
        to_id=original_sha,
        relation="records",
        order=2,
        created_by=rewrite_block["id"],
    )
    _write_rel(records_rel)

    # Merge new_concepts and inline concepts: for the same term, prefer
    # the source with a non-empty definition to avoid empty-definition
    # locking out a later non-empty one via the dedup key.
    merged_defines: dict[str, tuple[str, str]] = {}  # term → (term, definition)
    for nc in ca.new_concepts:
        merged_defines[nc.term] = (nc.term, nc.definition)
    for c in ca.concepts:
        existing = merged_defines.get(c.term)
        if existing is None or (not existing[1] and c.definition):
            merged_defines[c.term] = (c.term, c.definition)

    # defines relations: original → concept_id (for each concept)
    for term, definition in merged_defines.values():
        concept_id = make_concept_id(term)
        defines_rel = make_relation(
            from_id=original_sha,
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=rewrite_block["id"],
            concept_term=term,
            concept_definition=definition,
        )
        _write_rel(defines_rel)

    # modifies → refines/revises dispatch (保守方向原则: unknown → refines)
    for mod in ca.modifications:
        if not mod.target_id:
            continue
        target_sha = id_mapping.get(mod.target_id)
        if target_sha is None:
            try:
                target_sha = id_mapping.get(str(int(mod.target_id)))
            except ValueError:
                pass
        if target_sha is None:
            continue
        # Based on kind: revise → revises(order=1), else → refines(order=2)
        if mod.kind == "revise":
            relation_type, order = "revises", 1
        else:
            relation_type, order = "refines", 2  # unknown and refine → refines
        rel = make_relation(
            from_id=original_sha,
            to_id=target_sha,
            relation=relation_type,
            order=order,
            created_by=rewrite_block["id"],
            target_desc=mod.target_desc,
            modification=mod.modification,
            modification_kind=mod.kind,
        )
        _write_rel(rel)

    # Body-extracted negates (order=2)
    for neg in ca.negations:
        if not neg.target_id:
            continue
        neg_target_sha = id_mapping.get(neg.target_id)
        if neg_target_sha is None:
            try:
                neg_target_sha = id_mapping.get(str(int(neg.target_id)))
            except ValueError:
                pass
        if neg_target_sha is None:
            continue
        negates_rel = make_relation(
            from_id=original_sha,
            to_id=neg_target_sha,
            relation="negates",
            order=2,
            created_by=rewrite_block["id"],
            negation_type=neg.negation_type,
        )
        _write_rel(negates_rel)

    # references relations: original → referenced_block (for each body reference)
    for ref_id in ca.references:
        if ref_id == genealogy_id:
            continue  # skip self-reference
        ref_sha = id_mapping.get(ref_id)
        if ref_sha is None:
            try:
                ref_sha = id_mapping.get(str(int(ref_id)))
            except ValueError:
                pass
        if ref_sha is None:
            continue
        references_rel = make_relation(
            from_id=original_sha,
            to_id=ref_sha,
            relation="references",
            order=2,
            created_by=rewrite_block["id"],
        )
        _write_rel(references_rel)

    return {
        "genealogy_id": genealogy_id,
        "rewrite_block": rewrite_block["id"],
        "relations_written": relations_written,
        "concepts": len(ca.concepts) + len(ca.new_concepts),
        "modifications": len(ca.modifications),
        "references": len(ca.references),
    }


def run_enrichment(
    project_root: Path | None = None,
    base: Path | None = None,
    dry_run: bool = False,
) -> dict:
    """Run content enrichment migration on all settled files.

    Args:
        project_root: Project root directory
        base: Block topology base directory
        dry_run: If True, only output stats without writing blocks

    Returns:
        Summary dict with migration results
    """
    if project_root is None:
        project_root = PROJECT_ROOT
    if base is None:
        base = project_root / ".chanlun" / "block-topology"

    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"

    # Read meta.json for id_mapping
    meta = read_meta(base)
    if meta is None:
        return {"error": "meta.json not found — run migration first"}

    id_mapping = meta.get("id_mapping", {})
    if not id_mapping:
        return {"error": "id_mapping empty in meta.json"}

    results = []
    total_blocks = 0
    total_relations = 0
    total_concepts = 0

    # Load existing relation keys for dedup (idempotent migration)
    existing_keys = _build_existing_keys(base) if not dry_run else None

    for md_file in sorted(settled_dir.glob("*.md")):
        result = enrich_single_file(
            md_file, id_mapping, base, dry_run, existing_keys
        )
        if result is None:
            continue
        results.append(result)
        if not dry_run and "rewrite_block" in result:
            total_blocks += 1
            total_relations += result.get("relations_written", 0)
            total_concepts += result.get("concepts", 0)

    summary = {
        "mode": "dry_run" if dry_run else "write",
        "files_processed": len(results),
        "files_skipped": sum(1 for r in results if r.get("skipped")),
    }

    if dry_run:
        summary["file_stats"] = results
        summary["total_concepts"] = sum(r.get("concepts", 0) for r in results)
        summary["total_new_concepts"] = sum(
            r.get("new_concepts", 0) for r in results
        )
        summary["total_modifications"] = sum(
            r.get("modifications", 0) for r in results
        )
        summary["total_references"] = sum(
            r.get("references", 0) for r in results
        )
    else:
        summary["blocks_created"] = total_blocks
        summary["relations_written"] = total_relations
        summary["concepts_defined"] = total_concepts
        summary["file_results"] = results

        # Update meta.json
        meta["content_enrichment"] = {
            "blocks_created": total_blocks,
            "relations_written": total_relations,
            "concepts_defined": total_concepts,
        }
        write_meta(meta, base)

    return summary


def audit_modifies(
    base: Path | None = None,
) -> list[dict]:
    """Scan relations.jsonl for old modifies relations, output audit list.

    Returns list of dicts with relation details for human review.
    """
    if base is None:
        base = PROJECT_ROOT / ".chanlun" / "block-topology"
    relations = read_all_relations(base)
    modifies_rels = [
        r for r in relations if r.get("relation") == "modifies"
    ]
    return modifies_rels


def main():
    parser = argparse.ArgumentParser(
        description="Content enrichment migration for block topology"
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Only output stats, don't write blocks"
    )
    parser.add_argument(
        "--json", action="store_true",
        help="Output as JSON"
    )
    parser.add_argument(
        "--audit-modifies", action="store_true",
        help="Scan and output all old modifies relations for human review"
    )
    args = parser.parse_args()

    if args.audit_modifies:
        results = audit_modifies()
        if args.json:
            print(json.dumps(results, ensure_ascii=False, indent=2))
        else:
            print(f"[audit-modifies] found {len(results)} modifies relations")
            for r in results:
                print(json.dumps(r, ensure_ascii=False))
        return

    result = run_enrichment(dry_run=args.dry_run)

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        mode = result.get("mode", "unknown")
        print(f"[enrichment] mode: {mode}")
        print(f"[enrichment] files processed: {result.get('files_processed', 0)}")
        print(f"[enrichment] files skipped: {result.get('files_skipped', 0)}")
        if mode == "dry_run":
            print(f"[enrichment] total concepts: {result.get('total_concepts', 0)}")
            print(f"[enrichment] total new concepts: {result.get('total_new_concepts', 0)}")
            print(f"[enrichment] total modifications: {result.get('total_modifications', 0)}")
            print(f"[enrichment] total references: {result.get('total_references', 0)}")
        else:
            print(f"[enrichment] blocks created: {result.get('blocks_created', 0)}")
            print(f"[enrichment] relations written: {result.get('relations_written', 0)}")
            print(f"[enrichment] concepts defined: {result.get('concepts_defined', 0)}")


if __name__ == "__main__":
    main()
