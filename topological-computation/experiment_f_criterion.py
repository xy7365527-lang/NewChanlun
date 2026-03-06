"""f-criterion experiment — replace rule-based encounter detection with f(v,w).

Runs 2000 steps on the real genealogy graph using f(v,w) as the intrinsic
encounter criterion:
  - f(v,w) <= 2 → fold candidate
  - f(v,w) >= 10 → negate candidate
  - sublation: same as rule-based (pending negation pair + contested vertex)

Compares results against the rule-based 2000-step data (experiment_long_run_2000.json).
"""

from __future__ import annotations

import json
import random
import time
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from engine import (
    Graph, Vertex, Edge, VertexStatus, EdgeType,
    SettlementTracker, OperationResult,
    compute_beta_1, _connected_components,
    fold, negate, sublate,
)
from morse import compute_terrain, critical_neighbors


# ---------------------------------------------------------------------------
# f(v, w) computation
# ---------------------------------------------------------------------------

def compute_f(graph: Graph, v: str, w: str) -> int:
    """Compute f(v, w) on the current active graph.

    f(v,w) = (c - 1) + n_loop
    where:
      S = {v, w}
      n_loop = number of edges within S
      lower_link = neighbors of S not in S (among active vertices)
      c = connected components of the subgraph induced by lower_link
    """
    active = set(graph.active_vertex_ids())
    s = {v, w}

    # n_loop: edges within S (directed)
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )

    # lower link: active neighbors of S not in S
    lower_link: set[str] = set()
    for x in s:
        for n in graph.neighbors(x):
            if n not in s and n in active:
                lower_link.add(n)

    # connected components of lower link
    if lower_link:
        ll_edges: list[frozenset[str]] = []
        for e in graph.active_edges():
            if (e.source in lower_link and e.target in lower_link
                    and e.source != e.target):
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link), ll_edges)
    else:
        c = 0

    return (c - 1 if c > 0 else 0) + n_loop


# ---------------------------------------------------------------------------
# Step log
# ---------------------------------------------------------------------------

@dataclass
class StepLog:
    step: int
    position: str
    encounter: str
    operation: str
    beta_1_before: int
    beta_1_after: int
    delta_beta_1: int
    settled_count: int
    blocked: bool
    vertices_active: int
    edges_active: int
    vertices_full: int
    edges_full: int
    f_value: Optional[int] = None     # f(pos, target) that triggered this encounter
    target: Optional[str] = None      # which vertex was the encounter target


# ---------------------------------------------------------------------------
# f-criterion traversal engine
# ---------------------------------------------------------------------------

class FCriterionEngine:
    """Traversal engine using f(v,w) for encounter detection."""

    def __init__(
        self,
        graph: Graph,
        start: str,
        settlement_threshold: int = 15,
        seed: int = 42,
        f_fold_threshold: int = 2,
        f_negate_threshold: int = 10,
    ):
        self.k_full = graph
        self.k_active = graph
        self.position = start
        self.step = 0
        self.settlement = SettlementTracker(settlement_threshold)
        self.visit_history: list[str] = [start]
        self.logs: list[StepLog] = []
        self.rng = random.Random(seed)
        self.terrain = compute_terrain(self.k_active)
        self._pending_negations: list[tuple[str, str]] = []
        self._nothing_streak = 0
        self._blocked_at: dict[str, int] = {}

        self.f_fold_threshold = f_fold_threshold
        self.f_negate_threshold = f_negate_threshold

        # Diagnostics: f-value distribution per encounter
        self.f_fold_values: list[int] = []
        self.f_negate_values: list[int] = []
        self.f_all_values: list[int] = []  # all f values computed (sampled)

    def _detect_encounter(self) -> tuple[str, Optional[str], Optional[str], str, Optional[int]]:
        """Detect encounter using f(v,w).

        Returns (encounter_type, target_a, target_b, reason, f_value).
        encounter_type is one of: sublation, negate_a, fold, negate_b, nothing
        """
        pos = self.position
        active = set(self.k_active.active_vertex_ids())

        # 1. Sublation: same as rule-based (pending negation pair + contested vertex)
        v = self.k_active.vertex(pos)
        if v and v.status == VertexStatus.CONTESTED:
            for thesis, antithesis in self._pending_negations:
                if pos == thesis or pos == antithesis:
                    return ("sublation", thesis, antithesis,
                            f"Contested vertex {pos} with pending negation",
                            None)

        # 2. Compute f(pos, nb) for all neighbors
        neighbors = self.k_active.neighbors(pos)
        f_scores: list[tuple[str, int]] = []
        for nb in neighbors:
            if nb == pos or nb not in active:
                continue
            f_val = compute_f(self.k_active, pos, nb)
            f_scores.append((nb, f_val))
            self.f_all_values.append(f_val)

        if not f_scores:
            return ("nothing", None, None, "No active neighbors", None)

        # 3. Check for negate candidates: f >= threshold, bidirectional edge,
        #    both non-synthetic, no existing negation edge
        pos_is_synthetic = pos.startswith("syn_") or pos.startswith("anti_")
        out_nbs = set(self.k_active.out_neighbors(pos))
        in_nbs = set(self.k_active.in_neighbors(pos))

        best_negate: Optional[tuple[str, int]] = None
        for nb, f_val in f_scores:
            if f_val < self.f_negate_threshold:
                continue
            nb_is_synthetic = nb.startswith("syn_") or nb.startswith("anti_")
            if pos_is_synthetic or nb_is_synthetic:
                continue
            # Must have bidirectional edge
            if nb not in out_nbs or nb not in in_nbs:
                continue
            # No existing negation edge
            has_neg = any(
                e.edge_type == EdgeType.NEGATION
                and ((e.source == pos and e.target == nb)
                     or (e.source == nb and e.target == pos))
                for e in self.k_active.active_edges()
            )
            if has_neg:
                continue
            if best_negate is None or f_val > best_negate[1]:
                best_negate = (nb, f_val)

        if best_negate is not None:
            nb, f_val = best_negate
            self.f_negate_values.append(f_val)
            return ("negate_a", pos, nb,
                    f"f({pos[:16]},{nb[:16]})={f_val} >= {self.f_negate_threshold}",
                    f_val)

        # 4. Check for fold candidates: f <= threshold, both non-synthetic
        best_fold: Optional[tuple[str, int]] = None
        for nb, f_val in f_scores:
            if f_val > self.f_fold_threshold:
                continue
            nb_is_synthetic = nb.startswith("syn_") or nb.startswith("anti_")
            if pos_is_synthetic or nb_is_synthetic:
                continue
            if best_fold is None or f_val < best_fold[1]:
                best_fold = (nb, f_val)

        if best_fold is not None:
            nb, f_val = best_fold
            self.f_fold_values.append(f_val)
            return ("fold", pos, nb,
                    f"f({pos[:16]},{nb[:16]})={f_val} <= {self.f_fold_threshold}",
                    f_val)

        # 5. Negate_B: high tension (many high-f neighbors), vertex not contested
        high_f_count = sum(1 for _, f_val in f_scores if f_val >= self.f_negate_threshold)
        if high_f_count >= 2 and v and v.status == VertexStatus.ACTIVE:
            return ("negate_b", pos, None,
                    f"{high_f_count} neighbors with f >= {self.f_negate_threshold}",
                    None)

        # 6. Nothing
        return ("nothing", None, None, "No f-criterion encounter", None)

    def _execute_encounter(
        self, enc_type: str, target_a: Optional[str], target_b: Optional[str],
    ) -> tuple[str, bool]:
        """Execute the operation. Returns (operation_name, blocked)."""
        if enc_type == "sublation":
            result = sublate(
                self.k_active, target_a, target_b, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "sublate",
                    {"thesis": target_a, "antithesis": target_b},
                    result.blocked_by,
                )
                return "sublate_blocked", True

            self.k_active = result.graph
            self.k_full = self.k_full.add_vertex(self.k_active.vertex(result.new_vertex))
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            self.position = result.new_vertex
            self._pending_negations = [
                p for p in self._pending_negations
                if not (p[0] == target_a and p[1] == target_b)
            ]
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "sublate", False

        elif enc_type == "negate_a":
            result = negate(
                self.k_active, target_a, target_b, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "negate",
                    {"thesis": target_a, "antithesis": target_b},
                    result.blocked_by,
                )
                return "negate_blocked", True

            self.k_active = result.graph
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            self._pending_negations.append((target_a, target_b))
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "negate_a", False

        elif enc_type == "negate_b":
            result = negate(
                self.k_active, target_a, None, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "negate",
                    {"thesis": target_a},
                    result.blocked_by,
                )
                return "negate_blocked", True

            self.k_active = result.graph
            new_v = self.k_active.vertex(result.new_vertex)
            if new_v:
                self.k_full = self.k_full.add_vertex(new_v)
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            if result.new_vertex:
                self.position = result.new_vertex
                self._pending_negations.append((target_a, result.new_vertex))
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "negate_b", False

        elif enc_type == "fold":
            result = fold(
                self.k_active, [target_a, target_b], self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "fold",
                    {"vertices": [target_a, target_b]},
                    result.blocked_by,
                )
                return "fold_blocked", True

            self.k_active = result.graph
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            if self.position == target_b:
                self.position = target_a
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "fold", False

        return "nothing", False

    def _walk(self) -> None:
        """Move to adjacent vertex. Same logic as TraversalEngine.walk()."""
        if self._nothing_streak >= 5:
            active = self.k_active.active_vertex_ids()
            visit_counts: dict[str, int] = {}
            for v_id in active:
                visit_counts[v_id] = sum(1 for h in self.visit_history if h == v_id)
            least_visited = min(active, key=lambda v: (visit_counts.get(v, 0), v))
            if least_visited != self.position:
                self.position = least_visited
                self.visit_history.append(self.position)
                self._nothing_streak = 0
                return

        neighbors = self.k_active.neighbors(self.position)
        if not neighbors:
            return

        crit = critical_neighbors(self.k_active, self.position, self.terrain)
        unvisited = [n for n in crit if n not in self.visit_history[-5:]]
        if unvisited:
            self.position = self.rng.choice(unvisited)
        elif crit:
            self.position = self.rng.choice(crit)
        else:
            unvisited_any = [n for n in neighbors if n not in self.visit_history[-3:]]
            if unvisited_any:
                self.position = self.rng.choice(unvisited_any)
            else:
                self.position = self.rng.choice(neighbors)

        self.visit_history.append(self.position)

    def run_step(self) -> StepLog:
        """Execute one full step."""
        self.step += 1
        beta_before = compute_beta_1(self.k_active)

        enc_type, target_a, target_b, reason, f_val = self._detect_encounter()
        blocked = False
        op_name = "walk"

        # Skip encounter if recently blocked at this position
        if enc_type != "nothing" and self._blocked_at.get(self.position, -99) >= self.step - 2:
            enc_type = "nothing"

        if enc_type != "nothing":
            op_name, blocked = self._execute_encounter(enc_type, target_a, target_b)
            self._nothing_streak = 0
            if blocked:
                self._blocked_at[self.position] = self.step
        else:
            self._nothing_streak += 1
            self._walk()

        self.terrain = compute_terrain(self.k_active)

        beta_after = compute_beta_1(self.k_active)
        if beta_after > beta_before:
            new_edges = frozenset(
                (e.source, e.target) for e in self.k_active.active_edges()
                if e.created_at == self.step
            )
            if new_edges:
                self.settlement.register_new_cycle(new_edges, self.step)

        self.settlement.check_settlement(self.step, self.k_active)

        log = StepLog(
            step=self.step,
            position=self.position,
            encounter=enc_type,
            operation=op_name,
            beta_1_before=beta_before,
            beta_1_after=beta_after,
            delta_beta_1=beta_after - beta_before,
            settled_count=len(self.settlement.settled_cycles),
            blocked=blocked,
            vertices_active=len(self.k_active.active_vertex_ids()),
            edges_active=len(self.k_active.active_edges()),
            vertices_full=len(self.k_full.vertices),
            edges_full=len(self.k_full.edges),
            f_value=f_val,
            target=target_a if enc_type != "nothing" else None,
        )
        self.logs.append(log)
        return log


# ---------------------------------------------------------------------------
# Cross-component traversal (same as experiment_long_run.py)
# ---------------------------------------------------------------------------

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


class CrossComponentFCriterion:
    def __init__(self, graph, start, settlement_threshold=15, seed=42,
                 visit_cap=3, f_fold=2, f_negate=10):
        self.engine = FCriterionEngine(
            graph, start=start,
            settlement_threshold=settlement_threshold,
            seed=seed,
            f_fold_threshold=f_fold,
            f_negate_threshold=f_negate,
        )
        self.visit_cap = visit_cap
        self.components = _find_components(graph)
        self._component_map: dict[str, int] = {}
        for idx, comp in enumerate(self.components):
            for v in comp:
                self._component_map[v] = idx
        self.cross_jumps: list[dict] = []

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
            under_explored = [
                v for v in comp_active
                if visit_counts.get(v, 0) <= self.visit_cap
            ]
            if under_explored and len(comp_active) > best_size:
                best_comp = idx
                best_size = len(comp_active)
                best_target = min(
                    under_explored, key=lambda v: (visit_counts.get(v, 0), v)
                )
        if best_comp >= 0:
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


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_experiment():
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

    # Find start: highest-degree vertex in largest component (same as rule-based)
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
    print(f"  Start vertex: {start_vertex[:48]} (degree={start_degree})")
    print(f"  Start content: {start_content[:100]}")

    original_negation_pairs: set[frozenset[str]] = set()
    for e in graph.active_edges():
        if e.edge_type == EdgeType.NEGATION:
            original_negation_pairs.add(frozenset((e.source, e.target)))
    print(f"  Original negation edges: {len(original_negation_pairs)}")

    # Run 2000 steps
    max_steps = 2000
    f_fold = 2
    f_negate = 10
    print(f"\nRunning {max_steps} steps with f-criterion "
          f"(fold<={f_fold}, negate>={f_negate}, seed=42, threshold=15)...")
    t1 = time.time()

    ct = CrossComponentFCriterion(
        graph, start=start_vertex,
        settlement_threshold=15, seed=42, visit_cap=3,
        f_fold=f_fold, f_negate=f_negate,
    )

    window_data = []
    window_ops: Counter = Counter()

    for step_num in range(max_steps):
        ct.run_step()
        log = ct.engine.logs[-1]
        window_ops[log.operation] += 1

        if (step_num + 1) % 100 == 0:
            elapsed = time.time() - t1
            window_start = step_num - 98
            window_entry = {
                "window": f"{window_start}-{step_num + 1}",
                "beta_1_start": (ct.engine.logs[step_num - 99].beta_1_before
                                 if step_num >= 99 else initial_beta_1),
                "beta_1_end": log.beta_1_after,
                "delta_beta_1": log.beta_1_after - (
                    ct.engine.logs[step_num - 99].beta_1_before
                    if step_num >= 99 else initial_beta_1
                ),
                "settled_cumulative": log.settled_count,
                "fold": window_ops.get("fold", 0),
                "negate_a": window_ops.get("negate_a", 0),
                "sublate": window_ops.get("sublate", 0),
                "walk": window_ops.get("walk", 0),
                "negate_b": window_ops.get("negate_b", 0),
                "blocked": sum(
                    1 for l in ct.engine.logs[step_num - 99:step_num + 1]
                    if l.blocked
                ),
                "v_active": log.vertices_active,
                "e_active": log.edges_active,
            }
            total_in_window = sum(window_ops.values())
            non_walk = total_in_window - window_ops.get("walk", 0)
            window_entry["encounter_pct"] = round(
                non_walk / max(total_in_window, 1) * 100, 1
            )
            window_data.append(window_entry)
            window_ops = Counter()

            print(f"  Step {step_num + 1}: beta_1={log.beta_1_after} "
                  f"settled={log.settled_count} encounter={window_entry['encounter_pct']}% "
                  f"({elapsed:.1f}s)")

    run_time = time.time() - t1
    engine = ct.engine
    final_beta = compute_beta_1(engine.k_active)

    print(f"\nCompleted {max_steps} steps in {run_time:.2f}s "
          f"({run_time / max_steps * 1000:.1f}ms/step)")
    print(f"beta_1: {initial_beta_1} -> {final_beta} "
          f"(delta={final_beta - initial_beta_1:+d})")

    # ---- Operation distribution ----
    op_dist: Counter = Counter()
    for log in engine.logs:
        op_dist[log.operation] += 1

    # ---- Load rule-based data for comparison ----
    rule_json_path = Path(__file__).parent / "experiment_long_run_2000.json"
    rule_data = None
    if rule_json_path.exists():
        with open(rule_json_path, encoding="utf-8") as f:
            rule_data = json.load(f)
        print("\nLoaded rule-based comparison data.")

    # ---- Build comparison table ----
    comparison = {}
    if rule_data:
        rule_summary = rule_data["summary"]
        rule_op = rule_summary["operation_distribution"]

        comparison = {
            "metric": [
                "final_beta_1", "delta_beta_1", "growth_ratio",
                "settled_cycles", "blocked_operations",
                "encounter_density_%",
                "fold_count", "negate_a_count", "sublate_count",
                "walk_count", "negate_b_count",
                "fold_blocked_count",
                "ms_per_step",
            ],
            "rule_based": [
                rule_summary["final_beta_1"],
                rule_summary["delta_beta_1"],
                rule_summary["growth_ratio"],
                rule_summary["settled_cycles"],
                rule_summary["blocked_operations"],
                round((1 - rule_op.get("walk", 0) / 2000) * 100, 1),
                rule_op.get("fold", 0),
                rule_op.get("negate_a", 0),
                rule_op.get("sublate", 0),
                rule_op.get("walk", 0),
                rule_op.get("negate_b", 0),
                rule_op.get("fold_blocked", 0),
                rule_summary["ms_per_step"],
            ],
            "f_criterion": [
                final_beta,
                final_beta - initial_beta_1,
                round(final_beta / max(initial_beta_1, 1), 4),
                len(engine.settlement.settled_cycles),
                len(engine.settlement.blocked_log),
                round((1 - op_dist.get("walk", 0) / max_steps) * 100, 1),
                op_dist.get("fold", 0),
                op_dist.get("negate_a", 0),
                op_dist.get("sublate", 0),
                op_dist.get("walk", 0),
                op_dist.get("negate_b", 0),
                op_dist.get("fold_blocked", 0),
                round(run_time / max_steps * 1000, 1),
            ],
        }

    # ---- f-value distribution diagnostics ----
    f_fold_values = engine.f_fold_values
    f_negate_values = engine.f_negate_values
    f_all_sampled = engine.f_all_values

    f_diagnostics = {
        "fold_trigger_count": len(f_fold_values),
        "fold_f_values": dict(Counter(f_fold_values).most_common(10)),
        "negate_trigger_count": len(f_negate_values),
        "negate_f_values": dict(Counter(f_negate_values).most_common(10)),
        "all_f_sampled_count": len(f_all_sampled),
    }
    if f_all_sampled:
        f_sorted = sorted(f_all_sampled)
        f_diagnostics["all_f_percentiles"] = {
            "p10": f_sorted[len(f_sorted) // 10],
            "p25": f_sorted[len(f_sorted) // 4],
            "p50": f_sorted[len(f_sorted) // 2],
            "p75": f_sorted[3 * len(f_sorted) // 4],
            "p90": f_sorted[9 * len(f_sorted) // 10],
            "min": f_sorted[0],
            "max": f_sorted[-1],
        }
        f_diagnostics["all_f_mean"] = round(sum(f_all_sampled) / len(f_all_sampled), 2)

    # Fold signal-to-noise: proportion of fold triggers that led to actual
    # successful fold (vs blocked)
    fold_total = op_dist.get("fold", 0) + op_dist.get("fold_blocked", 0)
    fold_success = op_dist.get("fold", 0)
    fold_snr = round(fold_success / max(fold_total, 1), 4)

    if rule_data:
        rule_fold_total = rule_op.get("fold", 0) + rule_op.get("fold_blocked", 0)
        rule_fold_success = rule_op.get("fold", 0)
        rule_fold_snr = round(rule_fold_success / max(rule_fold_total, 1), 4)
    else:
        rule_fold_snr = None

    # ---- Save JSON ----
    step_logs_sampled = []
    for log in engine.logs:
        if log.step <= 100 or log.step % 10 == 0 or log.operation != "walk":
            entry = {
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
            }
            if log.f_value is not None:
                entry["f_value"] = log.f_value
            step_logs_sampled.append(entry)

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
            "f_fold_threshold": f_fold,
            "f_negate_threshold": f_negate,
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
            "fold_signal_to_noise": fold_snr,
        },
        "comparison_with_rule_based": comparison,
        "f_diagnostics": f_diagnostics,
        "fold_snr": {
            "f_criterion": fold_snr,
            "rule_based": rule_fold_snr,
        },
        "window_100_step_summaries": window_data,
        "step_logs": step_logs_sampled,
    }

    out_json = Path(__file__).parent / "experiment_f_criterion_2000.json"
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nJSON saved to {out_json}")

    # ---- Print comparison table ----
    if comparison:
        print(f"\n{'=' * 70}")
        print("COMPARISON: f-criterion vs rule-based (2000 steps)")
        print(f"{'=' * 70}")
        print(f"{'Metric':<25} {'Rule-based':>15} {'f-criterion':>15} {'Delta':>12}")
        print("-" * 70)
        for i, metric in enumerate(comparison["metric"]):
            rv = comparison["rule_based"][i]
            fv = comparison["f_criterion"][i]
            if isinstance(rv, (int, float)) and isinstance(fv, (int, float)):
                delta = fv - rv
                delta_str = f"{delta:+.1f}" if isinstance(delta, float) else f"{delta:+d}"
            else:
                delta_str = "—"
            print(f"{metric:<25} {str(rv):>15} {str(fv):>15} {delta_str:>12}")

        print(f"\n{'Fold signal-to-noise:':<25} "
              f"{'rule=' + str(rule_fold_snr):>15} "
              f"{'f=' + str(fold_snr):>15}")

    # ---- Print 100-step window table ----
    print(f"\n{'=' * 120}")
    print("100-STEP WINDOW SUMMARY (f-criterion)")
    print(f"{'=' * 120}")
    print(f"{'Window':<12} {'beta_1_s':>8} {'beta_1_e':>8} {'d_beta':>7} {'settled':>8} "
          f"{'fold':>5} {'neg_a':>6} {'sublate':>8} {'walk':>5} {'enc%':>6}")
    print("-" * 120)
    for w in window_data:
        print(f"{w['window']:<12} {w['beta_1_start']:>8} {w['beta_1_end']:>8} "
              f"{w['delta_beta_1']:>+7} {w['settled_cumulative']:>8} "
              f"{w['fold']:>5} {w['negate_a']:>6} {w['sublate']:>8} {w['walk']:>5} "
              f"{w['encounter_pct']:>5.1f}%")

    # ---- Print f-value diagnostics ----
    print(f"\n{'=' * 70}")
    print("f-VALUE DIAGNOSTICS")
    print(f"{'=' * 70}")
    print(f"Total f values computed: {f_diagnostics['all_f_sampled_count']}")
    if 'all_f_percentiles' in f_diagnostics:
        p = f_diagnostics['all_f_percentiles']
        print(f"Percentiles: min={p['min']} p10={p['p10']} p25={p['p25']} "
              f"p50={p['p50']} p75={p['p75']} p90={p['p90']} max={p['max']}")
        print(f"Mean: {f_diagnostics['all_f_mean']}")
    print(f"\nFold triggers: {f_diagnostics['fold_trigger_count']}")
    print(f"  f-value distribution: {f_diagnostics['fold_f_values']}")
    print(f"Negate triggers: {f_diagnostics['negate_trigger_count']}")
    print(f"  f-value distribution: {f_diagnostics['negate_f_values']}")

    return result


if __name__ == "__main__":
    run_experiment()
