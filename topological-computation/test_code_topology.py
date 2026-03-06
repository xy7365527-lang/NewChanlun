"""Self-analysis: the engine analyzes its own code.

Parse all .py files in topological-computation/, compute graph statistics,
find fold candidates, detect circular dependencies.

Output to tmp/code_topology_test.txt
"""

from __future__ import annotations

import os
import sys

script_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, script_dir)

from code_topology import parse_directory
from engine import compute_beta_1, EdgeType


def main() -> str:
    lines: list[str] = []
    lines.append("=" * 70)
    lines.append("CODE TOPOLOGY SELF-ANALYSIS")
    lines.append("=" * 70)
    lines.append("")

    graph = parse_directory(script_dir, "*.py")

    active_vids = graph.active_vertex_ids()
    active_edges = graph.active_edges()
    beta_1 = compute_beta_1(graph)

    lines.append(f"Vertices (active): {len(active_vids)}")
    lines.append(f"Edges (active): {len(active_edges)}")
    lines.append(f"beta_1: {beta_1}")
    lines.append("")

    # --- Vertex list ---
    lines.append("--- Functions/Classes (vertices) ---")
    lines.append("")
    for vid in sorted(active_vids):
        v = graph.vertex(vid)
        content = (v.content or "")[:80].replace("\n", " ")
        lines.append(f"  {vid}: {content}")
    lines.append("")

    # --- Edge statistics by type ---
    edge_counts: dict[str, int] = {}
    for e in active_edges:
        edge_counts[e.edge_type.value] = edge_counts.get(e.edge_type.value, 0) + 1
    lines.append("--- Edge counts by type ---")
    for etype, count in sorted(edge_counts.items()):
        lines.append(f"  {etype}: {count}")
    lines.append("")

    # --- Dependency edges ---
    dep_edges = [e for e in active_edges if e.edge_type == EdgeType.DEPENDENCY]
    lines.append(f"--- Dependency edges ({len(dep_edges)}) ---")
    for e in sorted(dep_edges, key=lambda e: (e.source, e.target)):
        lines.append(f"  {e.source} --> {e.target}")
    lines.append("")

    # --- Degree analysis ---
    in_degree: dict[str, int] = {v: 0 for v in active_vids}
    out_degree: dict[str, int] = {v: 0 for v in active_vids}
    for e in active_edges:
        if e.source in out_degree:
            out_degree[e.source] += 1
        if e.target in in_degree:
            in_degree[e.target] += 1

    total_degree = {v: in_degree.get(v, 0) + out_degree.get(v, 0) for v in active_vids}

    lines.append("--- High-degree functions (hub functions, top 15) ---")
    top_hubs = sorted(active_vids, key=lambda v: total_degree[v], reverse=True)[:15]
    for v in top_hubs:
        lines.append(f"  {v}: in={in_degree[v]}, out={out_degree[v]}, total={total_degree[v]}")
    lines.append("")

    # --- Fold candidates ---
    # Find pairs of functions with smallest "distance" (shared neighbors / total neighbors)
    lines.append("--- Fold candidates (function pairs with high neighbor overlap) ---")
    neighbor_sets: dict[str, set[str]] = {}
    for vid in active_vids:
        neighbors = set(graph.neighbors(vid))
        neighbor_sets[vid] = neighbors

    # Only consider function vertices (not imports/modules)
    func_vids = [v for v in active_vids if not v.startswith("<import>") and ".<module>" not in v]

    candidates: list[tuple[float, str, str]] = []
    seen: set[frozenset[str]] = set()
    for i, va in enumerate(func_vids):
        for vb in func_vids[i + 1:]:
            key = frozenset((va, vb))
            if key in seen:
                continue
            seen.add(key)
            na = neighbor_sets.get(va, set())
            nb = neighbor_sets.get(vb, set())
            if not na and not nb:
                continue
            union = na | nb
            if not union:
                continue
            intersection = na & nb
            jaccard = len(intersection) / len(union)
            if jaccard > 0:
                candidates.append((jaccard, va, vb))

    candidates.sort(key=lambda x: -x[0])
    for jaccard, va, vb in candidates[:10]:
        lines.append(f"  Jaccard={jaccard:.3f}: {va} <-> {vb}")

    if not candidates:
        lines.append("  (no overlapping neighbor pairs found)")
    lines.append("")

    # --- Circular dependency detection ---
    lines.append("--- Circular dependency detection ---")
    cycles_found = 0
    checked: set[str] = set()
    for vid in active_vids:
        if vid in checked:
            continue
        if graph.has_path(vid, vid):
            lines.append(f"  Self-cycle: {vid}")
            cycles_found += 1
        checked.add(vid)

    # Check for A->B->A patterns
    pair_cycles: list[tuple[str, str]] = []
    for e in dep_edges:
        if graph.has_path(e.target, e.source):
            pair = tuple(sorted((e.source, e.target)))
            if pair not in pair_cycles:
                pair_cycles.append(pair)

    for a, b in pair_cycles[:10]:
        lines.append(f"  Mutual dependency: {a} <-> {b}")
        cycles_found += 1

    if cycles_found == 0:
        lines.append("  (no circular dependencies detected)")
    lines.append("")

    # --- Isolated vertices ---
    isolated = [v for v in active_vids if total_degree[v] == 0]
    if isolated:
        lines.append(f"--- Isolated vertices ({len(isolated)}) ---")
        for v in sorted(isolated):
            lines.append(f"  {v}")
        lines.append("")

    # --- Summary ---
    lines.append("--- Summary ---")
    lines.append(f"Total vertices: {len(active_vids)}")
    lines.append(f"Total edges: {len(active_edges)}")
    lines.append(f"beta_1 (independent cycles): {beta_1}")
    lines.append(f"Hub functions: {', '.join(top_hubs[:5])}")
    lines.append(f"Fold candidates: {len(candidates)}")
    lines.append(f"Circular dependencies: {cycles_found}")
    lines.append(f"Isolated vertices: {len(isolated)}")

    return "\n".join(lines)


if __name__ == "__main__":
    report = main()
    print(report)

    out_dir = os.path.join(
        os.path.dirname(script_dir),
        "tmp",
    )
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "code_topology_test.txt")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\nSaved to {out_path}", file=sys.stderr)
