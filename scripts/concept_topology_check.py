"""concept_topology_check.py — 概念去重 + 矛盾检测 + 引用-依赖一致性检查

三个检测函数：
1. detect_duplicate_concepts() — 同一 concept_id 被多个区块 defines 且定义文本不一致
2. detect_concept_conflicts() — modifies(A→B) 但 references(C→B) 中 C 不 depends_on A
3. detect_reference_dependency_mismatch() — references vs depends_on 不一致

输出 JSON 报告。可作为 ceremony_scan 工位。
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE,
    read_all_relations,
    read_meta,
)


def _load_topology(base: Path) -> tuple[list[dict], dict[str, str]]:
    """Load relations and id_mapping from block topology."""
    relations = read_all_relations(base)
    meta = read_meta(base)
    id_mapping = meta.get("id_mapping", {}) if meta else {}
    # Build reverse mapping: sha → genealogy_id
    reverse_mapping = {}
    for gid, sha in id_mapping.items():
        reverse_mapping[sha] = gid
    return relations, reverse_mapping


def detect_duplicate_concepts(
    base: Path = DEFAULT_BASE,
) -> list[dict]:
    """Detect concept definitions that may be duplicates.

    Collects all `defines` relations, groups by concept_id (the `to` field).
    Same concept_id defined by multiple blocks with inconsistent definitions
    → potential duplicate.

    Excludes pairs connected by `modifies` relations (those are evolution,
    not duplication).
    """
    relations, reverse_mapping = _load_topology(base)

    # Collect defines relations grouped by concept_id
    concept_defs: dict[str, list[dict]] = defaultdict(list)
    for rel in relations:
        if rel.get("relation") == "defines":
            concept_id = rel["to"]
            concept_defs[concept_id].append({
                "from_block": rel["from"],
                "from_genealogy": reverse_mapping.get(rel["from"], rel["from"][:16]),
                "concept_term": rel.get("concept_term", ""),
                "concept_definition": rel.get("concept_definition", ""),
            })

    # Build modifies adjacency and compute connected components (union-find)
    # so that evolution chains A→B→C are recognized even without direct A↔C edge
    modifies_adj: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "modifies":
            modifies_adj[rel["from"]].add(rel["to"])
            modifies_adj[rel["to"]].add(rel["from"])

    def _find_component(start: str) -> set[str]:
        """BFS to find the connected component containing *start*."""
        visited: set[str] = set()
        queue = [start]
        while queue:
            node = queue.pop()
            if node in visited:
                continue
            visited.add(node)
            for neighbor in modifies_adj.get(node, ()):
                if neighbor not in visited:
                    queue.append(neighbor)
        return visited

    # Cache: block_id → component set (computed lazily)
    _component_cache: dict[str, set[str]] = {}

    def _get_component(block_id: str) -> set[str]:
        if block_id in _component_cache:
            return _component_cache[block_id]
        comp = _find_component(block_id)
        for member in comp:
            _component_cache[member] = comp
        return comp

    duplicates = []
    for concept_id, defs in concept_defs.items():
        if len(defs) < 2:
            continue

        # Check if all defining blocks belong to the same modifies component
        block_ids = [d["from_block"] for d in defs]
        first_comp = _get_component(block_ids[0])
        all_connected_by_modifies = all(
            bid in first_comp for bid in block_ids[1:]
        )

        if all_connected_by_modifies:
            continue  # evolution chain, not duplication

        # Check definition consistency
        definitions = set(d["concept_definition"] for d in defs if d["concept_definition"])
        if len(definitions) <= 1:
            continue  # same definition text, not a real duplicate

        duplicates.append({
            "concept_id": concept_id,
            "concept_term": next(
                (d["concept_term"] for d in defs if d["concept_term"]),
                defs[0].get("concept_term", ""),
            ),
            "definitions": defs,
            "definition_count": len(definitions),
            "connected_by_modifies": all_connected_by_modifies,
        })

    return duplicates


def detect_concept_conflicts(
    base: Path = DEFAULT_BASE,
) -> list[dict]:
    """Detect concept conflicts: A modifies B, but C references B without depending on A.

    If A modifies a concept in B, any block C that references B should either:
    - depend on A (knows about the modification), or
    - be older than A (written before the modification)

    Blocks that reference B but don't depend on A may be using an outdated
    version of B's concept.
    """
    relations, reverse_mapping = _load_topology(base)

    # Index: modifies relations (from → to)
    # "from" is the block that made the modification
    # "to" is the block whose concept was modified
    modifications: list[dict] = []
    for rel in relations:
        if rel.get("relation") == "modifies":
            modifications.append({
                "modifier_block": rel["from"],
                "modified_block": rel["to"],
                "target_desc": rel.get("target_desc", ""),
                "modification": rel.get("modification", ""),
            })

    if not modifications:
        return []

    # Index: references relations (from → to)
    references_index: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "references":
            references_index[rel["to"]].add(rel["from"])

    # Index: depends_on relations (from depends_on to)
    # Compute transitive closure so C→D→A is recognized as C depending on A
    depends_on_direct: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "depends_on":
            depends_on_direct[rel["from"]].add(rel["to"])

    # Cache for transitive closure (lazily computed per block)
    _deps_closure_cache: dict[str, set[str]] = {}

    def _get_deps_closure(block_id: str) -> set[str]:
        """BFS to compute transitive depends_on closure."""
        if block_id in _deps_closure_cache:
            return _deps_closure_cache[block_id]
        visited: set[str] = set()
        queue = list(depends_on_direct.get(block_id, ()))
        while queue:
            node = queue.pop()
            if node in visited:
                continue
            visited.add(node)
            for dep in depends_on_direct.get(node, ()):
                if dep not in visited:
                    queue.append(dep)
        _deps_closure_cache[block_id] = visited
        return visited

    def _genealogy_order(block_id: str) -> int:
        """Extract a numeric ordering from the genealogy id.

        Returns a large sentinel for blocks without a genealogy mapping so
        they are never considered "older" than the modifier.
        """
        gid = reverse_mapping.get(block_id, "")
        # Strip optional letter suffix (e.g. "005b" → 5)
        import re as _re
        m = _re.match(r"^(\d+)", gid)
        if m:
            return int(m.group(1))
        return 999999  # unknown → treat as newer (flag it)

    conflicts = []
    for mod in modifications:
        modified_block = mod["modified_block"]
        modifier_block = mod["modifier_block"]
        modifier_order = _genealogy_order(modifier_block)

        # Find blocks that reference the modified block
        referencers = references_index.get(modified_block, set())

        for ref_block in referencers:
            if ref_block == modifier_block:
                continue  # the modifier itself
            # Temporal guard: if the referencing block was written before
            # the modifier, it could not have known about the modification.
            ref_order = _genealogy_order(ref_block)
            if ref_order < modifier_order:
                continue  # older block — not stale, just pre-modification
            # Check if ref_block transitively depends_on modifier_block
            deps = _get_deps_closure(ref_block)
            if modifier_block not in deps:
                conflicts.append({
                    "type": "stale_reference",
                    "referencing_block": ref_block,
                    "referencing_genealogy": reverse_mapping.get(
                        ref_block, ref_block[:16]
                    ),
                    "modified_block": modified_block,
                    "modified_genealogy": reverse_mapping.get(
                        modified_block, modified_block[:16]
                    ),
                    "modifier_block": modifier_block,
                    "modifier_genealogy": reverse_mapping.get(
                        modifier_block, modifier_block[:16]
                    ),
                    "target_desc": mod["target_desc"],
                    "modification": mod["modification"],
                })

    return conflicts


def detect_reference_dependency_mismatch(
    base: Path = DEFAULT_BASE,
) -> list[dict]:
    """Detect mismatches between references (body) and depends_on (frontmatter).

    Two types:
    - missing_reference (warn): body references a block not in depends_on
    - structural_only (info): depends_on declares a block not referenced in body
      (may be a negative/structural dependency)
    """
    relations, reverse_mapping = _load_topology(base)

    # Group by from-block
    references_by_block: dict[str, set[str]] = defaultdict(set)
    depends_by_block: dict[str, set[str]] = defaultdict(set)

    for rel in relations:
        if rel.get("relation") == "references":
            references_by_block[rel["from"]].add(rel["to"])
        elif rel.get("relation") == "depends_on":
            depends_by_block[rel["from"]].add(rel["to"])

    mismatches = []

    # Collect all blocks that have either references or depends_on
    all_blocks = set(references_by_block.keys()) | set(depends_by_block.keys())

    for block_id in all_blocks:
        refs = references_by_block.get(block_id, set())
        deps = depends_by_block.get(block_id, set())

        # missing_reference: referenced in body but not in depends_on
        missing = refs - deps
        for m in missing:
            mismatches.append({
                "type": "missing_dependency",
                "severity": "warn",
                "block": block_id,
                "block_genealogy": reverse_mapping.get(block_id, block_id[:16]),
                "target": m,
                "target_genealogy": reverse_mapping.get(m, m[:16]),
                "detail": "正文引用但不在 depends_on 中",
            })

        # structural_only: in depends_on but not referenced in body
        structural = deps - refs
        for s in structural:
            mismatches.append({
                "type": "structural_only",
                "severity": "info",
                "block": block_id,
                "block_genealogy": reverse_mapping.get(block_id, block_id[:16]),
                "target": s,
                "target_genealogy": reverse_mapping.get(s, s[:16]),
                "detail": "depends_on 中声明但正文未引用（可能是否定性依赖）",
            })

    return mismatches


def run_all_checks(
    base: Path | None = None,
) -> dict:
    """Run all concept topology checks and return combined report."""
    if base is None:
        base = PROJECT_ROOT / ".chanlun" / "block-topology"

    duplicates = detect_duplicate_concepts(base)
    conflicts = detect_concept_conflicts(base)
    mismatches = detect_reference_dependency_mismatch(base)

    # Summary counts
    warn_mismatches = [m for m in mismatches if m["severity"] == "warn"]
    info_mismatches = [m for m in mismatches if m["severity"] == "info"]

    return {
        "duplicate_concepts": {
            "count": len(duplicates),
            "items": duplicates,
        },
        "concept_conflicts": {
            "count": len(conflicts),
            "items": conflicts,
        },
        "reference_dependency_mismatch": {
            "total": len(mismatches),
            "warn_count": len(warn_mismatches),
            "info_count": len(info_mismatches),
            "warn_items": warn_mismatches,
            "info_items": info_mismatches,
        },
        "summary": {
            "duplicates": len(duplicates),
            "conflicts": len(conflicts),
            "missing_dependencies": len(warn_mismatches),
            "structural_only": len(info_mismatches),
            "health": "clean" if (
                len(duplicates) == 0 and len(conflicts) == 0
                and len(warn_mismatches) == 0
            ) else "issues_found",
        },
    }


def main():
    parser = argparse.ArgumentParser(
        description="Concept topology health check"
    )
    parser.add_argument(
        "--base", type=str, default=None,
        help="Block topology base directory"
    )
    args = parser.parse_args()

    base = Path(args.base) if args.base else None
    report = run_all_checks(base)
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
