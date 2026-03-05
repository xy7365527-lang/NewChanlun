"""Traversal engine — walk + topological encounter rules.

Pure Python. No LLM in Phase 1 — encounters detected by topological rules.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from enum import Enum
from typing import Optional

from engine import (
    Graph, VertexStatus, EdgeType, SettlementTracker, OperationResult,
    compute_beta_1, fold, negate, sublate,
)
from morse import compute_terrain, critical_neighbors


class EncounterType(str, Enum):
    FOLD = "fold"
    NEGATE_A = "negate_a"      # existing antithesis
    NEGATE_B = "negate_b"      # create new antithesis
    SUBLATION = "sublation"
    NOTHING = "nothing"


@dataclass(frozen=True, slots=True)
class Encounter:
    encounter_type: EncounterType
    target_a: Optional[str] = None
    target_b: Optional[str] = None
    reason: str = ""


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


class TraversalEngine:
    """Traversal + encounter detection + operation execution."""

    def __init__(self, graph: Graph, start: str, settlement_threshold: int = 5, seed: int = 42):
        self.k_full = graph
        self.k_active = graph  # initially same
        self.position = start
        self.step = 0
        self.settlement = SettlementTracker(settlement_threshold)
        self.visit_history: list[str] = [start]
        self.logs: list[StepLog] = []
        self.rng = random.Random(seed)
        self.terrain = compute_terrain(self.k_active)
        self._pending_negations: list[tuple[str, str]] = []  # (thesis, antithesis) pairs
        self._nothing_streak = 0  # consecutive "nothing" steps
        self._blocked_at: dict[str, int] = {}  # position -> last blocked step (skip encounter there)

    # -- encounter detection (topological rules) ----------------------------

    def detect_encounter(self) -> Encounter:
        """Detect encounter at current position using topological rules.

        Priority: Sublation > Negate_A > Fold > Negate_B > Nothing
        """
        pos = self.position
        active = set(self.k_active.active_vertex_ids())

        # 1. Sublation: current position is contested + has pending negation pair
        v = self.k_active.vertex(pos)
        if v and v.status == VertexStatus.CONTESTED:
            for thesis, antithesis in self._pending_negations:
                if pos == thesis or pos == antithesis:
                    return Encounter(
                        EncounterType.SUBLATION, thesis, antithesis,
                        f"Contested vertex {pos} with pending negation {thesis}-{antithesis}",
                    )

        # 2. Negate_A: current vertex and a neighbor have circular dependency
        neighbors = self.k_active.neighbors(pos)
        for nb in neighbors:
            if nb == pos:
                continue
            # Check: pos depends on nb AND nb depends on something that depends on pos
            # Simplified: pos→nb and nb→pos paths both exist (not via direct edge)
            out_nbs = self.k_active.out_neighbors(pos)
            in_nbs = self.k_active.in_neighbors(pos)
            if nb in out_nbs and nb in in_nbs:
                # Bidirectional relationship — contradiction signal
                # But skip if already negation edge between them
                has_neg = any(
                    e.edge_type == EdgeType.NEGATION
                    and ((e.source == pos and e.target == nb) or (e.source == nb and e.target == pos))
                    for e in self.k_active.active_edges()
                )
                if not has_neg:
                    return Encounter(
                        EncounterType.NEGATE_A, pos, nb,
                        f"Bidirectional relationship {pos}<->{nb} — contradiction signal",
                    )

        # 3. Fold: two neighbors share a third neighbor but aren't connected to each other
        # Only fold if there are settled cycles (don't destroy structure prematurely)
        if self.settlement.settled_cycles:
            for i, a in enumerate(neighbors):
                for b in neighbors[i + 1:]:
                    if a == b:
                        continue
                    a_nbs = set(self.k_active.neighbors(a))
                    if b not in a_nbs:
                        b_nbs = set(self.k_active.neighbors(b))
                        shared = (a_nbs & b_nbs) - {pos, a, b}
                        if shared:
                            return Encounter(
                                EncounterType.FOLD, a, b,
                                f"Vertices {a} and {b} share neighbor(s) {shared} but aren't directly connected",
                            )

        # 4. Negate_B: high topological tension (≥2 critical edges at current position)
        crit_nbs = critical_neighbors(self.k_active, pos, self.terrain)
        if len(crit_nbs) >= 2 and v and v.status == VertexStatus.ACTIVE:
            # Don't negate if already contested
            return Encounter(
                EncounterType.NEGATE_B, pos, None,
                f"High topological tension at {pos}: {len(crit_nbs)} critical neighbors",
            )

        # 5. Nothing
        return Encounter(EncounterType.NOTHING, reason="No encounter detected")

    # -- operation execution ------------------------------------------------

    def execute_encounter(self, enc: Encounter) -> tuple[str, bool]:
        """Execute the operation corresponding to an encounter.

        Returns (operation_name, blocked).
        """
        if enc.encounter_type == EncounterType.SUBLATION:
            result = sublate(
                self.k_active, enc.target_a, enc.target_b, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "sublate",
                    {"thesis": enc.target_a, "antithesis": enc.target_b},
                    result.blocked_by,
                )
                return "sublate_blocked", True

            self.k_active = result.graph
            self.k_full = self.k_full.add_vertex(self.k_active.vertex(result.new_vertex))
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)

            # Creation = arrival
            self.position = result.new_vertex

            # Remove from pending
            self._pending_negations = [
                p for p in self._pending_negations
                if not (p[0] == enc.target_a and p[1] == enc.target_b)
            ]

            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "sublate", False

        elif enc.encounter_type == EncounterType.NEGATE_A:
            result = negate(
                self.k_active, enc.target_a, enc.target_b, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "negate",
                    {"thesis": enc.target_a, "antithesis": enc.target_b},
                    result.blocked_by,
                )
                return "negate_blocked", True

            self.k_active = result.graph
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)

            self._pending_negations.append((enc.target_a, enc.target_b))
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "negate_a", False

        elif enc.encounter_type == EncounterType.NEGATE_B:
            result = negate(
                self.k_active, enc.target_a, None, self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "negate",
                    {"thesis": enc.target_a},
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

            # Creation = arrival
            if result.new_vertex:
                self.position = result.new_vertex
                self._pending_negations.append((enc.target_a, result.new_vertex))
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "negate_b", False

        elif enc.encounter_type == EncounterType.FOLD:
            result = fold(
                self.k_active, [enc.target_a, enc.target_b], self.step, self.settlement,
            )
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "fold",
                    {"vertices": [enc.target_a, enc.target_b]},
                    result.blocked_by,
                )
                return "fold_blocked", True

            self.k_active = result.graph
            # Update K_full with fold record
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            # If position was folded away, move to kept vertex
            if self.position == enc.target_b:
                self.position = enc.target_a

            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "fold", False

        return "nothing", False

    # -- walk ---------------------------------------------------------------

    def walk(self) -> None:
        """Move to an adjacent vertex. Prefer critical edges, then unvisited, then random.

        Anti-oscillation: if stuck in nothing streak >= 5, jump to least-visited active vertex.
        """
        if self._nothing_streak >= 5:
            # Jump to least-visited active vertex to escape local trap
            active = self.k_active.active_vertex_ids()
            visit_counts = {}
            for v in active:
                visit_counts[v] = sum(1 for h in self.visit_history if h == v)
            least_visited = min(active, key=lambda v: (visit_counts.get(v, 0), v))
            if least_visited != self.position:
                self.position = least_visited
                self.visit_history.append(self.position)
                self._nothing_streak = 0
                return

        neighbors = self.k_active.neighbors(self.position)
        if not neighbors:
            return  # stuck (should not happen in connected graph)

        # Prefer critical neighbors
        crit = critical_neighbors(self.k_active, self.position, self.terrain)
        unvisited = [n for n in crit if n not in self.visit_history[-5:]]
        if unvisited:
            self.position = self.rng.choice(unvisited)
        elif crit:
            self.position = self.rng.choice(crit)
        else:
            # Fall back to any neighbor
            unvisited_any = [n for n in neighbors if n not in self.visit_history[-3:]]
            if unvisited_any:
                self.position = self.rng.choice(unvisited_any)
            else:
                self.position = self.rng.choice(neighbors)

        self.visit_history.append(self.position)

    # -- main step ----------------------------------------------------------

    def run_step(self) -> StepLog:
        """Execute one full step: detect encounter → execute or walk → update terrain → settle."""
        self.step += 1
        beta_before = compute_beta_1(self.k_active)

        enc = self.detect_encounter()
        blocked = False
        op_name = "walk"

        # Skip encounter if recently blocked at this position
        if enc.encounter_type != EncounterType.NOTHING and self._blocked_at.get(self.position, -99) >= self.step - 2:
            enc = Encounter(EncounterType.NOTHING, reason="Recently blocked here, walking away")

        if enc.encounter_type != EncounterType.NOTHING:
            op_name, blocked = self.execute_encounter(enc)
            self._nothing_streak = 0
            if blocked:
                self._blocked_at[self.position] = self.step
        else:
            self._nothing_streak += 1
            self.walk()

        # Update terrain after any graph change
        self.terrain = compute_terrain(self.k_active)

        # Check settlement
        # If β₁ increased, register the operation's edges as a new cycle candidate
        beta_after = compute_beta_1(self.k_active)
        if beta_after > beta_before:
            # Find edges created at this step as the cycle proxy
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
            encounter=enc.encounter_type.value,
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
        )
        self.logs.append(log)
        return log
