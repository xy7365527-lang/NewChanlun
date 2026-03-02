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
    VALIDITY_CAPABLE_RELATIONS,
    read_all_relations,
    read_meta,
)


# --- Constants ---

EVOLUTION_RELATIONS = frozenset({"modifies", "refines", "revises"})


def _is_evolution_relation(rel: dict) -> bool:
    """Check if relation is an evolution relation (modifies/refines/revises)."""
    return rel.get("relation") in EVOLUTION_RELATIONS


def _should_check_conflict(rel: dict) -> bool:
    """Check if relation should be included in conflict detection.

    Conservative direction principle: detection side = high sensitivity.
    - revises → always check
    - modifies → old data, treat as unknown, conservatively include
    - refines → only include kind=unknown
    """
    rtype = rel.get("relation")
    if rtype == "revises":
        return True
    if rtype == "modifies":
        return True  # old data = unknown → conservative inclusion
    if rtype == "refines":
        kind = rel.get("modification_kind", "unknown")
        return kind == "unknown"
    return False


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
        if _is_evolution_relation(rel):
            modifies_adj[rel.get("from", "")].add(rel.get("to", ""))
            modifies_adj[rel.get("to", "")].add(rel.get("from", ""))

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

        # Folding status: potential_birth when 2nd defines appears
        folding_status = "potential_birth" if len(defs) == 2 else "confirmed_duplicate"

        duplicates.append({
            "concept_id": concept_id,
            "concept_term": next(
                (d["concept_term"] for d in defs if d["concept_term"]),
                defs[0].get("concept_term", ""),
            ),
            "definitions": defs,
            "definition_count": len(definitions),
            "connected_by_modifies": all_connected_by_modifies,
            "folding_status": folding_status,
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

    # Index: modifications using _should_check_conflict
    # "from" is the block that made the modification
    # "to" is the block whose concept was modified
    modifications: list[dict] = []
    for rel in relations:
        if _should_check_conflict(rel):
            from_id = rel.get("from")
            to_id = rel.get("to")
            if not from_id or not to_id:
                continue  # malformed record, skip
            modifications.append({
                "modifier_block": from_id,
                "modified_block": to_id,
                "target_desc": rel.get("target_desc", ""),
                "modification": rel.get("modification", ""),
            })

    if not modifications:
        return []

    # Index: references relations (from → to)
    references_index: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "references":
            from_id = rel.get("from")
            to_id = rel.get("to")
            if from_id and to_id:
                references_index[to_id].add(from_id)

    # Index: depends_on relations (from depends_on to)
    # Compute transitive closure so C→D→A is recognized as C depending on A
    depends_on_direct: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "depends_on":
            from_id = rel.get("from")
            to_id = rel.get("to")
            if from_id and to_id:
                depends_on_direct[from_id].add(to_id)

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

    # --- Dedup: fold stale_reference conflicts by source relation ---
    # Multiple referencing blocks affected by the same (modified, modifier)
    # pair are folded into a single representative entry.
    stale = [c for c in conflicts if c["type"] == "stale_reference"]
    non_stale = [c for c in conflicts if c["type"] != "stale_reference"]

    if stale:
        grouped: dict[tuple[str, str], list[dict]] = defaultdict(list)
        for c in stale:
            key = (c["modified_block"], c["modifier_block"])
            grouped[key].append(c)

        folded: list[dict] = []
        for (_modified, _modifier), items in grouped.items():
            representative = dict(items[0])  # shallow copy of first entry
            affected_blocks = [
                reverse_mapping.get(it["referencing_block"], it["referencing_block"][:16])
                for it in items
            ]
            representative["affected_count"] = len(items)
            representative["affected_blocks"] = affected_blocks
            folded.append(representative)

        return non_stale + folded

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
                "direction": "divergent",
                "signal": "potential_depends_on",
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
                "direction": "convergent",
                "signal": "potential_negates",
            })

    return mismatches


def detect_negation_consistency(
    base: Path = DEFAULT_BASE,
) -> list[dict]:
    """Detect inconsistencies between order=1 and order=2 negates.

    Compares frontmatter-declared negates (order=1, from dag.yaml migration)
    with body-extracted negates (order=2, from content enrichment).

    Two types:
    - undeclared_negation (warn): body negates exist but no order=1 counterpart
    - phantom_negation (info): order=1 exists but no body negation found
    """
    relations, reverse_mapping = _load_topology(base)

    # Group negates by (from, to) and order
    negates_order1: dict[str, set[str]] = defaultdict(set)  # from → set of to
    negates_order2: dict[str, set[str]] = defaultdict(set)

    for rel in relations:
        if rel.get("relation") != "negates":
            continue
        from_id = rel.get("from")
        to_id = rel.get("to")
        if not from_id or not to_id:
            continue
        order = rel.get("order")
        if order == 1:
            negates_order1[from_id].add(to_id)
        elif order == 2:
            negates_order2[from_id].add(to_id)

    issues = []

    # Check all blocks that have any negates
    all_blocks = set(negates_order1.keys()) | set(negates_order2.keys())

    for block_id in all_blocks:
        o1_targets = negates_order1.get(block_id, set())
        o2_targets = negates_order2.get(block_id, set())

        # undeclared: body negated but no order=1
        for target in o2_targets - o1_targets:
            issues.append({
                "type": "undeclared_negation",
                "severity": "warn",
                "block": block_id,
                "block_genealogy": reverse_mapping.get(block_id, block_id[:16]),
                "target": target,
                "target_genealogy": reverse_mapping.get(target, target[:16]),
                "detail": "正文否定但 frontmatter 中无对应声明",
            })

        # phantom: order=1 exists but body doesn't mention
        for target in o1_targets - o2_targets:
            issues.append({
                "type": "phantom_negation",
                "severity": "info",
                "block": block_id,
                "block_genealogy": reverse_mapping.get(block_id, block_id[:16]),
                "target": target,
                "target_genealogy": reverse_mapping.get(target, target[:16]),
                "detail": "frontmatter 声明否定但正文无对应否定段落",
            })

    return issues


# --- Graph Invariants (Phase 2) ---

# Topological relation types — only these form meaningful directed edges.
# Excluded: records, defines, annotates (metadata/documentation, not topology).
TOPOLOGICAL_RELATIONS = frozenset({
    "depends_on", "negates", "related", "tensions_with", "supersedes",
    "residue_of", "reopens", "freezes", "splits", "severs",
    "negated_by", "modifies", "refines", "revises", "references",
})


def compute_graph_invariants(
    base: Path = DEFAULT_BASE,
) -> dict:
    """计算有向图不变量（Phase 2: O(V+E) 复杂度）。

    在活跃图和全量图上分别计算：
    - β₀: 弱连通分量数（忽略边方向的连通分量）
    - cycle_rank: Σ(|E_i| - |V_i| + 1) over 强连通分量（Tarjan SCC）

    活跃图：仅包含 validity != "invalidated" 的边
    全量图：包含所有边
    """
    relations = read_all_relations(base)

    # Filter to topological relations only
    topo_rels = [r for r in relations if r.get("relation") in TOPOLOGICAL_RELATIONS]

    # Build two edge lists: full and active
    full_edges: list[tuple[str, str]] = []
    active_edges: list[tuple[str, str]] = []

    for rel in topo_rels:
        src = rel.get("from", "")
        dst = rel.get("to", "")
        if not src or not dst:
            continue
        full_edges.append((src, dst))
        # For VALIDITY_CAPABLE_RELATIONS, exclude invalidated edges from active
        if rel.get("relation") in VALIDITY_CAPABLE_RELATIONS:
            if rel.get("validity") == "invalidated":
                continue
        active_edges.append((src, dst))

    def _compute_invariants(
        edges: list[tuple[str, str]],
    ) -> tuple[int, int]:
        """Compute β₀ and cycle_rank for a set of directed edges.

        Returns (beta_0, cycle_rank).
        """
        if not edges:
            return (0, 0)

        # Collect all nodes
        nodes: set[str] = set()
        # Forward adjacency for Tarjan; undirected adjacency for β₀
        fwd: dict[str, list[str]] = defaultdict(list)
        undirected: dict[str, list[str]] = defaultdict(list)
        # Count edges per SCC (need edge info keyed by source)
        for src, dst in edges:
            nodes.add(src)
            nodes.add(dst)
            fwd[src].append(dst)
            undirected[src].append(dst)
            undirected[dst].append(src)

        # --- β₀: weak connected components via BFS on undirected graph ---
        visited_weak: set[str] = set()
        beta_0 = 0
        for node in nodes:
            if node in visited_weak:
                continue
            beta_0 += 1
            queue = [node]
            while queue:
                cur = queue.pop()
                if cur in visited_weak:
                    continue
                visited_weak.add(cur)
                for nb in undirected.get(cur, ()):
                    if nb not in visited_weak:
                        queue.append(nb)

        # --- cycle_rank: iterative Tarjan SCC ---
        # For each SCC with |V_i| > 0, count edges within SCC,
        # then cycle_rank_i = |E_i| - |V_i| + 1
        index_counter = [0]
        node_index: dict[str, int] = {}
        node_lowlink: dict[str, int] = {}
        on_stack: set[str] = set()
        stack: list[str] = []
        sccs: list[set[str]] = []

        # Iterative Tarjan using explicit call stack
        # Each frame: (node, neighbor_iterator, phase)
        # phase 0 = first visit; phase 1 = returning from child
        for start_node in nodes:
            if start_node in node_index:
                continue
            call_stack: list[tuple[str, list[str], int]] = [
                (start_node, list(fwd.get(start_node, ())), 0)
            ]
            # Track which neighbor index we're at for each frame
            neighbor_idx: list[int] = [0]

            while call_stack:
                v, neighbors, phase = call_stack[-1]

                if phase == 0:
                    # First visit
                    node_index[v] = index_counter[0]
                    node_lowlink[v] = index_counter[0]
                    index_counter[0] += 1
                    stack.append(v)
                    on_stack.add(v)
                    # Update phase to 1 (processing neighbors)
                    call_stack[-1] = (v, neighbors, 1)
                    neighbor_idx[-1] = 0

                # Process neighbors
                idx = neighbor_idx[-1]
                pushed_child = False
                while idx < len(neighbors):
                    w = neighbors[idx]
                    if w not in node_index:
                        # Push child frame
                        neighbor_idx[-1] = idx + 1
                        call_stack.append(
                            (w, list(fwd.get(w, ())), 0)
                        )
                        neighbor_idx.append(0)
                        pushed_child = True
                        break
                    elif w in on_stack:
                        node_lowlink[v] = min(
                            node_lowlink[v], node_index[w]
                        )
                    idx += 1

                if pushed_child:
                    continue

                neighbor_idx[-1] = idx

                # All neighbors processed — check if v is SCC root
                if node_lowlink[v] == node_index[v]:
                    scc: set[str] = set()
                    while True:
                        w = stack.pop()
                        on_stack.discard(w)
                        scc.add(w)
                        if w == v:
                            break
                    sccs.append(scc)

                # Pop this frame and update parent's lowlink
                call_stack.pop()
                neighbor_idx.pop()
                if call_stack:
                    parent = call_stack[-1][0]
                    node_lowlink[parent] = min(
                        node_lowlink[parent], node_lowlink[v]
                    )

        # Compute cycle_rank per SCC
        cycle_rank = 0
        for scc in sccs:
            if len(scc) < 2:
                # Check self-loop
                for n in scc:
                    for dst in fwd.get(n, ()):
                        if dst == n:
                            cycle_rank += 1  # self-loop: E=1, V=1, rank=1
                continue
            # Count edges within SCC
            e_count = 0
            for n in scc:
                for dst in fwd.get(n, ()):
                    if dst in scc:
                        e_count += 1
            v_count = len(scc)
            cycle_rank += e_count - v_count + 1

        return (beta_0, cycle_rank)

    active_beta_0, active_cycle_rank = _compute_invariants(active_edges)
    full_beta_0, full_cycle_rank = _compute_invariants(full_edges)

    return {
        "active_beta_0": active_beta_0,
        "active_cycle_rank": active_cycle_rank,
        "full_beta_0": full_beta_0,
        "full_cycle_rank": full_cycle_rank,
    }


def is_structurally_significant(
    relation: dict,
    active_graph_stats: dict | None = None,
) -> bool | None:
    """判定一条 validity-capable 关系是否具有拓扑结构显著性。

    Phase 2 实现：接口预留，返回 None。
    Phase 3 实现时填充实际逻辑（检查边是否为 SCC 桥）。
    """
    if active_graph_stats is None:
        return None
    return None


def run_all_checks(
    base: Path | None = None,
) -> dict:
    """Run all concept topology checks and return combined report."""
    if base is None:
        base = PROJECT_ROOT / ".chanlun" / "block-topology"

    duplicates = detect_duplicate_concepts(base)
    conflicts = detect_concept_conflicts(base)
    mismatches = detect_reference_dependency_mismatch(base)
    negation_issues = detect_negation_consistency(base)
    graph_invariants = compute_graph_invariants(base)

    # Summary counts
    warn_mismatches = [m for m in mismatches if m["severity"] == "warn"]
    info_mismatches = [m for m in mismatches if m["severity"] == "info"]
    warn_negations = [n for n in negation_issues if n["severity"] == "warn"]
    info_negations = [n for n in negation_issues if n["severity"] == "info"]

    # Proposed annotations from convergent mismatches
    proposed_annotations = [
        {
            "type": "annotation",
            "order": 1,
            "target_block": m["block"],
            "target_genealogy": m["block_genealogy"],
            "signal_type": m.get("signal", "unknown"),
            "direction": m.get("direction", "unknown"),
            "status": "proposed",
        }
        for m in mismatches
        if m.get("direction") == "convergent"
    ]

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
        "negation_consistency": {
            "total": len(negation_issues),
            "warn_count": len(warn_negations),
            "info_count": len(info_negations),
            "warn_items": warn_negations,
            "info_items": info_negations,
        },
        "proposed_annotations": proposed_annotations,
        "graph_invariants": graph_invariants,
        "invariant_status": "graph_invariants_computed",
        "formalization_scope": "directed_graph_only",
        "ruling_273_ack": True,
        "summary": {
            "duplicates": len(duplicates),
            "conflicts": len(conflicts),
            "missing_dependencies": len(warn_mismatches),
            "structural_only": len(info_mismatches),
            "undeclared_negations": len(warn_negations),
            "phantom_negations": len(info_negations),
            "health": "clean" if (
                len(duplicates) == 0 and len(conflicts) == 0
                and len(warn_mismatches) == 0
                and len(warn_negations) == 0
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
