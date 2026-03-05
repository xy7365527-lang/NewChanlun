"""Real genealogy traversal experiment.

Loads the actual block-topology data (~2524 vertices, ~6751 edges, β₁ ~ 2325)
and runs the traversal engine on it. Analyzes fold, negation, settlement patterns.

Pure Python, no external dependencies beyond the project modules.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

from engine import Graph, EdgeType, compute_beta_1, _connected_components
from traversal import TraversalEngine, EncounterType
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Connected component analysis
# ---------------------------------------------------------------------------

def _find_components(graph: Graph) -> list[list[str]]:
    """Find connected components, return sorted by size (largest first).

    Uses BFS on the undirected projection of active edges.
    """
    active_vids = set(graph.active_vertex_ids())
    undirected_edges = graph.undirected_active_edges()

    # Build adjacency
    adj: dict[str, set[str]] = {v: set() for v in active_vids}
    for edge_set in undirected_edges:
        pair = sorted(edge_set)
        if len(pair) == 2:
            adj[pair[0]].add(pair[1])
            adj[pair[1]].add(pair[0])

    visited: set[str] = set()
    components: list[list[str]] = []

    for start in sorted(active_vids):
        if start in visited:
            continue
        component: list[str] = []
        stack = [start]
        while stack:
            v = stack.pop()
            if v in visited:
                continue
            visited.add(v)
            component.append(v)
            for nb in adj.get(v, ()):
                if nb not in visited:
                    stack.append(nb)
        components.append(sorted(component))

    components.sort(key=lambda c: -len(c))
    return components


def _vertex_degrees(graph: Graph) -> dict[str, int]:
    """Compute undirected degree for each active vertex."""
    degrees: dict[str, int] = Counter()
    for edge_set in graph.undirected_active_edges():
        for v in edge_set:
            degrees[v] += 1
    return dict(degrees)


# ---------------------------------------------------------------------------
# Cross-component jump wrapper
# ---------------------------------------------------------------------------

class CrossComponentTraversal:
    """Wraps TraversalEngine with cross-component jump logic.

    When all vertices in the current component have been visited > visit_cap
    times, jump to the largest under-explored component.
    """

    def __init__(
        self,
        graph: Graph,
        start: str,
        settlement_threshold: int = 15,
        seed: int = 42,
        visit_cap: int = 3,
    ):
        self.engine = TraversalEngine(
            graph, start=start,
            settlement_threshold=settlement_threshold,
            seed=seed,
            use_llm=False,
        )
        self.visit_cap = visit_cap
        self.components = _find_components(graph)
        self._component_map = self._build_component_map()
        self.cross_jumps: list[dict] = []

    def _build_component_map(self) -> dict[str, int]:
        """Map vertex_id -> component_index."""
        m: dict[str, int] = {}
        for idx, comp in enumerate(self.components):
            for v in comp:
                m[v] = idx
        return m

    def _current_component_idx(self) -> int:
        return self._component_map.get(self.engine.position, 0)

    def _visit_counts(self) -> dict[str, int]:
        counts: dict[str, int] = Counter()
        for v in self.engine.visit_history:
            counts[v] += 1
        return dict(counts)

    def _should_jump(self) -> bool:
        """Check if all active vertices in current component visited > visit_cap."""
        comp_idx = self._current_component_idx()
        comp_vids = set(self.components[comp_idx])
        active = set(self.engine.k_active.active_vertex_ids())
        comp_active = comp_vids & active
        if not comp_active:
            return True

        visit_counts = self._visit_counts()
        return all(visit_counts.get(v, 0) > self.visit_cap for v in comp_active)

    def _jump_to_next_component(self, step: int) -> bool:
        """Jump to the largest under-explored component. Returns True if jumped."""
        visit_counts = self._visit_counts()
        active = set(self.engine.k_active.active_vertex_ids())
        current_comp = self._current_component_idx()

        best_comp = -1
        best_size = 0
        best_target = ""

        for idx, comp in enumerate(self.components):
            if idx == current_comp:
                continue
            comp_active = [v for v in comp if v in active]
            if not comp_active:
                continue
            under_explored = [v for v in comp_active if visit_counts.get(v, 0) <= self.visit_cap]
            if under_explored and len(comp_active) > best_size:
                best_comp = idx
                best_size = len(comp_active)
                # Pick least-visited vertex in the target component
                best_target = min(under_explored, key=lambda v: (visit_counts.get(v, 0), v))

        if best_comp >= 0:
            old_pos = self.engine.position
            old_comp = current_comp
            self.engine.position = best_target
            self.engine.visit_history.append(best_target)
            self.engine._nothing_streak = 0
            self.cross_jumps.append({
                "step": step,
                "from_component": old_comp,
                "from_position": old_pos,
                "to_component": best_comp,
                "to_position": best_target,
                "target_component_size": best_size,
            })
            return True
        return False

    def run_step(self) -> None:
        """Run one step with cross-component jump check."""
        if self._should_jump():
            self._jump_to_next_component(self.engine.step + 1)
        self.engine.run_step()


# ---------------------------------------------------------------------------
# Analysis helpers
# ---------------------------------------------------------------------------

def _analyze_folds(engine: TraversalEngine, graph: Graph) -> list[dict]:
    """Analyze fold events: content of folded pairs, cross-region check."""
    folds = []
    component_map: dict[str, int] = {}
    components = _find_components(graph)
    for idx, comp in enumerate(components):
        for v in comp:
            component_map[v] = idx

    for log in engine.logs:
        if log.operation == "fold":
            # Find the encounter details from step
            step_idx = log.step - 1
            if step_idx < len(engine.logs):
                # Reconstruct fold targets from visit history context
                pos = log.position
                # The fold merged current position with a historical vertex
                # We can find the target from k_full's fold edges at this step
                fold_edges = [
                    e for e in engine.k_full.edges
                    if e.edge_type == EdgeType.FOLD and e.created_at == log.step
                ]
                for fe in fold_edges:
                    v_a = fe.source
                    v_b = fe.target if fe.target != fe.source else None
                    content_a = ""
                    content_b = ""
                    va = graph.vertex(v_a)
                    if va:
                        content_a = va.content or ""
                    if v_b:
                        vb = graph.vertex(v_b)
                        if vb:
                            content_b = vb.content or ""
                    comp_a = component_map.get(v_a, -1)
                    comp_b = component_map.get(v_b, -1) if v_b else -1
                    folds.append({
                        "step": log.step,
                        "vertex_a": v_a,
                        "vertex_b": v_b,
                        "content_a": content_a[:200],
                        "content_b": content_b[:200],
                        "component_a": comp_a,
                        "component_b": comp_b,
                        "cross_region": comp_a != comp_b and comp_b >= 0,
                    })
    return folds


def _analyze_negations(
    engine: TraversalEngine,
    original_negation_pairs: set[frozenset[str]],
) -> dict:
    """Analyze negation events vs. pre-existing negation edges."""
    negate_events = []
    matched_original = 0
    new_negations = 0

    for log in engine.logs:
        if "negate" in log.operation and not log.blocked:
            neg_edges = [
                e for e in engine.k_active.active_edges()
                if e.edge_type == EdgeType.NEGATION and e.created_at == log.step
            ]
            for ne in neg_edges:
                pair = frozenset((ne.source, ne.target))
                is_original = pair in original_negation_pairs
                if is_original:
                    matched_original += 1
                else:
                    new_negations += 1
                negate_events.append({
                    "step": log.step,
                    "source": ne.source,
                    "target": ne.target,
                    "operation": log.operation,
                    "matches_original_negation": is_original,
                })

    return {
        "total_negate_operations": len(negate_events),
        "matched_original_negation_edges": matched_original,
        "new_negations_created": new_negations,
        "events": negate_events,
    }


def _analyze_settlements(engine: TraversalEngine) -> list[dict]:
    """Analyze settled cycles."""
    settlements = []
    for sc in engine.settlement.settled_cycles:
        edge_list = sorted(sc.edges)
        vertices_involved = set()
        for src, tgt in edge_list:
            vertices_involved.add(src)
            vertices_involved.add(tgt)
        settlements.append({
            "settled_at_step": sc.settled_at_step,
            "edge_count": len(edge_list),
            "vertex_count": len(vertices_involved),
            "vertices": sorted(vertices_involved),
            "edges": [list(e) for e in edge_list],
        })
    return settlements


def _operation_distribution(engine: TraversalEngine) -> dict[str, int]:
    """Count operations by type."""
    dist: dict[str, int] = Counter()
    for log in engine.logs:
        dist[log.operation] += 1
    return dict(dist)


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_experiment(
    repo_root: str | Path,
    max_steps: int = 500,
    seed: int = 42,
    settlement_threshold: int = 15,
    visit_cap: int = 3,
    auto_extend: bool = True,
) -> dict:
    """Run the real genealogy traversal experiment."""
    from genealogy_loader import load_from_repo

    repo_root = Path(repo_root)
    print(f"Loading graph from: {repo_root}")
    t0 = time.time()
    graph, relation_counts = load_from_repo(repo_root)
    load_time = time.time() - t0
    print(f"  Loaded in {load_time:.2f}s")

    # Graph stats
    active_vids = graph.active_vertex_ids()
    n_vertices = len(active_vids)
    n_edges_directed = len(graph.active_edges())
    n_edges_undirected = len(graph.undirected_active_edges())
    initial_beta_1 = compute_beta_1(graph)

    # Count original negation edges
    original_negation_pairs: set[frozenset[str]] = set()
    for e in graph.active_edges():
        if e.edge_type == EdgeType.NEGATION:
            original_negation_pairs.add(frozenset((e.source, e.target)))

    # Component analysis
    components = _find_components(graph)
    n_components = len(components)
    largest_component = components[0] if components else []
    component_sizes = [len(c) for c in components[:10]]

    print(f"  Vertices: {n_vertices}")
    print(f"  Edges: {n_edges_directed} (directed), {n_edges_undirected} (undirected)")
    print(f"  Initial beta_1: {initial_beta_1}")
    print(f"  Components: {n_components}")
    print(f"  Largest component: {len(largest_component)} vertices")
    print(f"  Original negation edges: {len(original_negation_pairs)}")
    print(f"  Top component sizes: {component_sizes}")

    # Find start: highest-degree vertex in largest component
    degrees = _vertex_degrees(graph)
    largest_set = set(largest_component)
    start_vertex = max(
        largest_component,
        key=lambda v: (degrees.get(v, 0), v),
    )
    start_degree = degrees.get(start_vertex, 0)
    start_content = ""
    sv = graph.vertex(start_vertex)
    if sv:
        start_content = sv.content or ""

    print(f"\n  Start vertex: {start_vertex[:32]}...")
    print(f"  Start degree: {start_degree}")
    print(f"  Start content: {start_content[:100]}")

    # Run traversal
    print(f"\n  Running {max_steps} steps (seed={seed}, threshold={settlement_threshold})...")
    t1 = time.time()

    ct = CrossComponentTraversal(
        graph, start=start_vertex,
        settlement_threshold=settlement_threshold,
        seed=seed,
        visit_cap=visit_cap,
    )

    for step_num in range(max_steps):
        ct.run_step()

        # Progress reporting every 50 steps
        if (step_num + 1) % 50 == 0:
            log = ct.engine.logs[-1]
            elapsed = time.time() - t1
            print(f"    Step {step_num + 1}: beta_1={log.beta_1_after} "
                  f"V={log.vertices_active} E={log.edges_active} "
                  f"settled={log.settled_count} ({elapsed:.1f}s)")

    # Auto-extend check
    current_beta = compute_beta_1(ct.engine.k_active)
    extended = False
    if auto_extend and current_beta > initial_beta_1 * 1.5 and max_steps < 1000:
        extra_steps = 1000 - max_steps
        print(f"\n  beta_1 grew from {initial_beta_1} to {current_beta} "
              f"(>{initial_beta_1 * 1.5:.0f}), extending by {extra_steps} steps...")
        for step_num in range(extra_steps):
            ct.run_step()
            if (step_num + 1) % 100 == 0:
                log = ct.engine.logs[-1]
                elapsed = time.time() - t1
                print(f"    Step {max_steps + step_num + 1}: beta_1={log.beta_1_after} "
                      f"V={log.vertices_active} E={log.edges_active} "
                      f"settled={log.settled_count} ({elapsed:.1f}s)")
        extended = True

    run_time = time.time() - t1
    total_steps = len(ct.engine.logs)
    print(f"\n  Completed {total_steps} steps in {run_time:.2f}s "
          f"({run_time / total_steps * 1000:.1f}ms/step)")

    # Analysis
    engine = ct.engine
    final_beta = compute_beta_1(engine.k_active)
    beta_trajectory = [log.beta_1_after for log in engine.logs]
    max_beta = max(beta_trajectory) if beta_trajectory else initial_beta_1

    op_dist = _operation_distribution(engine)
    fold_analysis = _analyze_folds(engine, graph)
    negation_analysis = _analyze_negations(engine, original_negation_pairs)
    settlement_analysis = _analyze_settlements(engine)

    # Visit coverage
    visited_set = set(engine.visit_history)
    active_final = set(engine.k_active.active_vertex_ids())
    coverage = len(visited_set & active_final) / max(len(active_final), 1)

    # Components visited
    components_touched: set[int] = set()
    comp_map: dict[str, int] = {}
    for idx, comp in enumerate(components):
        for v in comp:
            comp_map[v] = idx
    for v in visited_set:
        if v in comp_map:
            components_touched.add(comp_map[v])

    # Build step_logs (sampled: every step for first 100, then every 10th)
    step_logs = []
    for log in engine.logs:
        if log.step <= 100 or log.step % 10 == 0 or log.operation != "walk":
            step_logs.append({
                "step": log.step,
                "position": log.position[:32],
                "encounter": log.encounter,
                "operation": log.operation,
                "beta_1": log.beta_1_after,
                "delta_beta_1": log.delta_beta_1,
                "settled": log.settled_count,
                "blocked": log.blocked,
                "v_active": log.vertices_active,
                "e_active": log.edges_active,
            })

    result = {
        "config": {
            "repo_root": str(repo_root),
            "vertices": n_vertices,
            "edges_directed": n_edges_directed,
            "edges_undirected": n_edges_undirected,
            "initial_beta_1": initial_beta_1,
            "components": n_components,
            "largest_component_size": len(largest_component),
            "original_negation_edges": len(original_negation_pairs),
            "start_vertex": start_vertex,
            "start_degree": start_degree,
            "start_content": start_content[:200],
            "max_steps": total_steps,
            "seed": seed,
            "settlement_threshold": settlement_threshold,
            "visit_cap": visit_cap,
            "auto_extended": extended,
        },
        "summary_metrics": {
            "final_beta_1": final_beta,
            "max_beta_1": max_beta,
            "delta_beta_1": final_beta - initial_beta_1,
            "beta_1_growth_ratio": round(final_beta / max(initial_beta_1, 1), 4),
            "total_steps": total_steps,
            "run_time_seconds": round(run_time, 2),
            "ms_per_step": round(run_time / max(total_steps, 1) * 1000, 1),
            "vertices_final_active": len(active_final),
            "vertices_final_full": len(engine.k_full.vertices),
            "edges_final_active": len(engine.k_active.active_edges()),
            "edges_final_full": len(engine.k_full.edges),
            "coverage": round(coverage, 4),
            "components_touched": len(components_touched),
            "cross_component_jumps": len(ct.cross_jumps),
            "settled_cycles": len(engine.settlement.settled_cycles),
            "blocked_operations": len(engine.settlement.blocked_log),
            "operation_distribution": op_dist,
        },
        "fold_analysis": {
            "total_folds": len(fold_analysis),
            "cross_region_folds": sum(1 for f in fold_analysis if f["cross_region"]),
            "folds": fold_analysis,
        },
        "negation_analysis": negation_analysis,
        "settlement_analysis": {
            "total_settled": len(settlement_analysis),
            "settlements": settlement_analysis,
        },
        "cross_component_jumps": ct.cross_jumps,
        "beta_1_trajectory_sampled": beta_trajectory[::10],
        "step_logs": step_logs,
    }

    # Print summary
    print(f"\n{'=' * 60}")
    print("REAL GENEALOGY TRAVERSAL RESULTS")
    print(f"{'=' * 60}")
    print(f"  beta_1: {initial_beta_1} -> {final_beta} (max={max_beta}, "
          f"delta={final_beta - initial_beta_1:+d})")
    print(f"  Growth ratio: {final_beta / max(initial_beta_1, 1):.4f}")
    print(f"  Steps: {total_steps} ({run_time:.1f}s)")
    print(f"  Vertices: {len(active_final)} active / {len(engine.k_full.vertices)} full")
    print(f"  Coverage: {coverage:.1%} ({len(visited_set & active_final)}/{len(active_final)})")
    print(f"  Components touched: {len(components_touched)}/{n_components}")
    print(f"  Cross-component jumps: {len(ct.cross_jumps)}")
    print(f"\n  Operations:")
    for op, count in sorted(op_dist.items(), key=lambda x: -x[1]):
        print(f"    {op}: {count}")
    print(f"\n  Folds: {len(fold_analysis)} total, "
          f"{sum(1 for f in fold_analysis if f['cross_region'])} cross-region")
    if fold_analysis:
        print(f"  First fold:")
        ff = fold_analysis[0]
        print(f"    Step {ff['step']}: {ff['vertex_a'][:32]} + {(ff.get('vertex_b') or '')[:32]}")
        print(f"    Content A: {ff['content_a'][:80]}")
        print(f"    Content B: {ff['content_b'][:80]}")
    print(f"\n  Negations: {negation_analysis['total_negate_operations']} total")
    print(f"    Matched original: {negation_analysis['matched_original_negation_edges']}")
    print(f"    New: {negation_analysis['new_negations_created']}")
    print(f"\n  Settlements: {len(settlement_analysis)}")
    print(f"  Blocked: {len(engine.settlement.blocked_log)}")

    return result


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Real genealogy traversal experiment",
    )
    parser.add_argument(
        "--steps", type=int, default=500,
        help="Max traversal steps (default: 500)",
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed (default: 42)",
    )
    parser.add_argument(
        "--threshold", type=int, default=15,
        help="Settlement threshold (default: 15)",
    )
    parser.add_argument(
        "--repo-root", type=str, default=None,
        help="Repository root path (default: two levels up from this file)",
    )
    parser.add_argument(
        "--visit-cap", type=int, default=3,
        help="Visit count cap before cross-component jump (default: 3)",
    )
    parser.add_argument(
        "--no-auto-extend", action="store_true",
        help="Disable auto-extension when beta_1 grows > 1.5x",
    )
    parser.add_argument(
        "--output", type=str, default=None,
        help="Output JSON path (default: experiment_real_result.json)",
    )
    args = parser.parse_args()

    if args.repo_root:
        repo_root = Path(args.repo_root)
    else:
        repo_root = Path(__file__).resolve().parent.parent

    output_path = args.output or "experiment_real_result.json"

    result = run_experiment(
        repo_root=repo_root,
        max_steps=args.steps,
        seed=args.seed,
        settlement_threshold=args.threshold,
        visit_cap=args.visit_cap,
        auto_extend=not args.no_auto_extend,
    )

    out = Path(output_path)
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nResults written to {out}")


if __name__ == "__main__":
    main()
