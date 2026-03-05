"""2000-step long-run experiment.

Runs 2000 traversal steps on the real genealogy graph,
records metrics every 100 steps, extracts all original negation pairs.
"""

from __future__ import annotations

import json
import time
from collections import Counter
from pathlib import Path

from engine import Graph, EdgeType, compute_beta_1, _connected_components
from traversal import TraversalEngine, EncounterType


def _find_components(graph: Graph) -> list[list[str]]:
    active_vids = set(graph.active_vertex_ids())
    undirected_edges = graph.undirected_active_edges()
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
    degrees: dict[str, int] = Counter()
    for edge_set in graph.undirected_active_edges():
        for v in edge_set:
            degrees[v] += 1
    return dict(degrees)


class CrossComponentTraversal:
    def __init__(self, graph, start, settlement_threshold=15, seed=42, visit_cap=3):
        self.engine = TraversalEngine(
            graph, start=start,
            settlement_threshold=settlement_threshold,
            seed=seed, use_llm=False,
        )
        self.visit_cap = visit_cap
        self.components = _find_components(graph)
        self._component_map = {}
        for idx, comp in enumerate(self.components):
            for v in comp:
                self._component_map[v] = idx
        self.cross_jumps = []

    def _should_jump(self):
        comp_idx = self._component_map.get(self.engine.position, 0)
        comp_vids = set(self.components[comp_idx])
        active = set(self.engine.k_active.active_vertex_ids())
        comp_active = comp_vids & active
        if not comp_active:
            return True
        visit_counts = Counter(self.engine.visit_history)
        return all(visit_counts.get(v, 0) > self.visit_cap for v in comp_active)

    def _jump_to_next_component(self, step):
        visit_counts = Counter(self.engine.visit_history)
        active = set(self.engine.k_active.active_vertex_ids())
        current_comp = self._component_map.get(self.engine.position, 0)
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
                best_target = min(under_explored, key=lambda v: (visit_counts.get(v, 0), v))
        if best_comp >= 0:
            old_pos = self.engine.position
            self.engine.position = best_target
            self.engine.visit_history.append(best_target)
            self.engine._nothing_streak = 0
            self.cross_jumps.append({
                "step": step,
                "from_component": current_comp,
                "to_component": best_comp,
                "to_position": best_target[:32],
                "target_component_size": best_size,
            })
            return True
        return False

    def run_step(self):
        if self._should_jump():
            self._jump_to_next_component(self.engine.step + 1)
        self.engine.run_step()


def run_long_experiment():
    from genealogy_loader import load_from_repo

    repo_root = Path("C:/Users/hanju/NewChanlun")
    print("Loading graph...")
    t0 = time.time()
    graph, relation_counts = load_from_repo(repo_root)
    load_time = time.time() - t0
    print(f"  Loaded in {load_time:.2f}s")

    active_vids = graph.active_vertex_ids()
    n_vertices = len(active_vids)
    initial_beta_1 = compute_beta_1(graph)

    # Find start: highest-degree vertex in largest component
    components = _find_components(graph)
    largest_component = components[0]
    degrees = _vertex_degrees(graph)
    start_vertex = max(largest_component, key=lambda v: (degrees.get(v, 0), v))
    start_degree = degrees.get(start_vertex, 0)
    sv = graph.vertex(start_vertex)
    start_content = (sv.content or "") if sv else ""

    print(f"  Vertices: {n_vertices}")
    print(f"  Initial beta_1: {initial_beta_1}")
    print(f"  Components: {len(components)}")
    print(f"  Largest component: {len(largest_component)} vertices")
    print(f"  Start vertex degree: {start_degree}")
    print(f"  Start content: {start_content[:100]}")

    # Count original negation edges for reference
    original_negation_pairs: set[frozenset[str]] = set()
    for e in graph.active_edges():
        if e.edge_type == EdgeType.NEGATION:
            original_negation_pairs.add(frozenset((e.source, e.target)))
    print(f"  Original negation edges: {len(original_negation_pairs)}")

    # Run 2000 steps
    max_steps = 2000
    print(f"\nRunning {max_steps} steps (seed=42, threshold=15)...")
    t1 = time.time()

    ct = CrossComponentTraversal(
        graph, start=start_vertex,
        settlement_threshold=15, seed=42, visit_cap=3,
    )

    # Track 100-step windows
    window_data = []
    window_ops = Counter()

    for step_num in range(max_steps):
        ct.run_step()
        log = ct.engine.logs[-1]
        window_ops[log.operation] += 1

        if (step_num + 1) % 100 == 0:
            elapsed = time.time() - t1
            window_start = step_num - 98  # 1-indexed
            window_entry = {
                "window": f"{window_start}-{step_num + 1}",
                "beta_1_start": ct.engine.logs[step_num - 99].beta_1_before if step_num >= 99 else initial_beta_1,
                "beta_1_end": log.beta_1_after,
                "delta_beta_1": log.beta_1_after - (ct.engine.logs[step_num - 99].beta_1_before if step_num >= 99 else initial_beta_1),
                "settled_cumulative": log.settled_count,
                "fold": window_ops.get("fold", 0),
                "negate_a": window_ops.get("negate_a", 0),
                "sublate": window_ops.get("sublate", 0),
                "walk": window_ops.get("walk", 0),
                "negate_b": window_ops.get("negate_b", 0),
                "blocked": sum(1 for l in ct.engine.logs[step_num-99:step_num+1] if l.blocked),
                "v_active": log.vertices_active,
                "e_active": log.edges_active,
            }
            # encounter % = non-walk / total
            total_in_window = sum(window_ops.values())
            non_walk = total_in_window - window_ops.get("walk", 0)
            window_entry["encounter_pct"] = round(non_walk / max(total_in_window, 1) * 100, 1)
            window_data.append(window_entry)
            window_ops = Counter()  # reset for next window

            print(f"  Step {step_num + 1}: beta_1={log.beta_1_after} "
                  f"settled={log.settled_count} encounter={window_entry['encounter_pct']}% "
                  f"({elapsed:.1f}s)")

    run_time = time.time() - t1
    engine = ct.engine
    final_beta = compute_beta_1(engine.k_active)

    print(f"\nCompleted {max_steps} steps in {run_time:.2f}s "
          f"({run_time / max_steps * 1000:.1f}ms/step)")
    print(f"beta_1: {initial_beta_1} -> {final_beta} (delta={final_beta - initial_beta_1:+d})")

    # ---- Extract negation pairs (negate_a only, excluding synthetic vertices) ----
    negation_pairs = []
    for log in engine.logs:
        if log.operation == "negate_a" and not log.blocked:
            # Find negation edges created at this step
            neg_edges = [
                e for e in engine.k_active.edges
                if e.edge_type == EdgeType.NEGATION and e.created_at == log.step
            ]
            # Also check k_full for edges that may have been removed from k_active later
            if not neg_edges:
                neg_edges = [
                    e for e in engine.k_full.edges
                    if e.edge_type == EdgeType.NEGATION and e.created_at == log.step
                ]
            for ne in neg_edges:
                src, tgt = ne.source, ne.target
                # Skip synthetic vertices
                if src.startswith("syn_") or src.startswith("anti_"):
                    continue
                if tgt.startswith("syn_") or tgt.startswith("anti_"):
                    continue
                # Get content
                src_v = graph.vertex(src)
                tgt_v = graph.vertex(tgt)
                src_content = (src_v.content or "") if src_v else ""
                tgt_content = (tgt_v.content or "") if tgt_v else ""
                negation_pairs.append({
                    "step": log.step,
                    "vertex_a": src[:48],
                    "vertex_b": tgt[:48],
                    "content_a": src_content[:200],
                    "content_b": tgt_content[:200],
                    "position": log.position[:48],
                })

    # Deduplicate by (vertex_a, vertex_b) pair, keep first occurrence
    seen_pairs = set()
    unique_negation_pairs = []
    for np_item in negation_pairs:
        pair_key = frozenset((np_item["vertex_a"], np_item["vertex_b"]))
        if pair_key not in seen_pairs:
            seen_pairs.add(pair_key)
            unique_negation_pairs.append(np_item)

    print(f"\nNegation pairs (negate_a, non-synthetic): {len(unique_negation_pairs)}")

    # ---- Operation distribution ----
    op_dist = Counter()
    for log in engine.logs:
        op_dist[log.operation] += 1

    # ---- Save JSON ----
    # Build sampled step_logs
    step_logs_sampled = []
    for log in engine.logs:
        if log.step <= 100 or log.step % 10 == 0 or log.operation != "walk":
            step_logs_sampled.append({
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
            "initial_beta_1": initial_beta_1,
            "components": len(components),
            "largest_component_size": len(largest_component),
            "original_negation_edges": len(original_negation_pairs),
            "start_vertex": start_vertex[:48],
            "start_degree": start_degree,
            "start_content": start_content[:200],
            "max_steps": max_steps,
            "seed": 42,
            "settlement_threshold": 15,
        },
        "summary": {
            "final_beta_1": final_beta,
            "delta_beta_1": final_beta - initial_beta_1,
            "growth_ratio": round(final_beta / max(initial_beta_1, 1), 4),
            "run_time_seconds": round(run_time, 2),
            "ms_per_step": round(run_time / max_steps * 1000, 1),
            "vertices_final_active": len(engine.k_active.active_vertex_ids()),
            "edges_final_active": len(engine.k_active.active_edges()),
            "settled_cycles": len(engine.settlement.settled_cycles),
            "blocked_operations": len(engine.settlement.blocked_log),
            "cross_component_jumps": len(ct.cross_jumps),
            "operation_distribution": dict(op_dist),
        },
        "window_100_step_summaries": window_data,
        "negation_pairs_negate_a_original": unique_negation_pairs,
        "negation_pairs_count": len(unique_negation_pairs),
        "step_logs": step_logs_sampled,
    }

    out_json = Path(__file__).parent / "experiment_long_run_2000.json"
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nJSON saved to {out_json}")

    # ---- Save negation pairs text ----
    out_txt = Path(__file__).parent / "long_run_negation_pairs.txt"
    with open(out_txt, "w", encoding="utf-8") as f:
        f.write(f"Original negation pairs (negate_a, non-synthetic) from 2000-step run\n")
        f.write(f"Total: {len(unique_negation_pairs)}\n")
        f.write(f"{'=' * 80}\n\n")
        for np_item in unique_negation_pairs:
            f.write(f"Step {np_item['step']}: \"{np_item['content_a']}\" vs \"{np_item['content_b']}\"\n")
    print(f"Negation pairs text saved to {out_txt}")

    # ---- Print 100-step window table ----
    print(f"\n{'=' * 120}")
    print("100-STEP WINDOW SUMMARY")
    print(f"{'=' * 120}")
    print(f"{'Window':<12} {'beta_1_s':>8} {'beta_1_e':>8} {'d_beta':>7} {'settled':>8} "
          f"{'fold':>5} {'neg_a':>6} {'sublate':>8} {'walk':>5} {'enc%':>6}")
    print("-" * 120)
    for w in window_data:
        print(f"{w['window']:<12} {w['beta_1_start']:>8} {w['beta_1_end']:>8} "
              f"{w['delta_beta_1']:>+7} {w['settled_cumulative']:>8} "
              f"{w['fold']:>5} {w['negate_a']:>6} {w['sublate']:>8} {w['walk']:>5} "
              f"{w['encounter_pct']:>5.1f}%")

    # ---- Print first 20 negation pairs ----
    print(f"\n{'=' * 80}")
    print(f"FIRST 20 NEGATION PAIRS (of {len(unique_negation_pairs)} total)")
    print(f"{'=' * 80}")
    for i, np_item in enumerate(unique_negation_pairs[:20]):
        print(f"\n{i+1}. Step {np_item['step']}:")
        print(f"   A: {np_item['content_a']}")
        print(f"   B: {np_item['content_b']}")

    return result


if __name__ == "__main__":
    run_long_experiment()
