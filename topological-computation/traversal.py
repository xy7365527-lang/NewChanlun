"""Traversal engine — walk + topological encounter rules.

Phase 1: encounters detected by topological rules.
Phase 2: encounters detected by LLM via semantic analysis of vertex content.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from enum import Enum
from typing import Optional

from engine import (
    Graph, VertexStatus, EdgeType, SettlementTracker, OperationResult,
    SublationRecord,
    compute_beta_1, fold, negate, sublate, _connected_components,
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
    f_value: int = -99  # terrain annotation: f(target_a, target_b) if applicable


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
    f_value: int = -99  # f(v,w) terrain annotation: -1=no shared neighbors, 0=topo equivalent, high=topo distant


class TraversalEngine:
    """Traversal + encounter detection + operation execution."""

    def __init__(
        self,
        graph: Graph,
        start: str,
        settlement_threshold: int = 5,
        seed: int = 42,
        use_llm: bool = False,
        conservative_prompt: bool = False,
        use_f_criterion: bool = False,
        f_fold_threshold: int = 2,
        f_negate_threshold: int = 10,
    ):
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
        self._use_llm = use_llm
        self._conservative_prompt = conservative_prompt
        self._llm_log: list[dict] = []  # LLM call log for debugging
        self._use_f_criterion = use_f_criterion
        self._f_fold_threshold = f_fold_threshold
        self._f_negate_threshold = f_negate_threshold

    # -- f(v,w) terrain annotation (not a filter, not a trigger) ---------------

    def _compute_f(self, v: str, w: str) -> int:
        """Compute f(v,w) = (c-1) + n_loop for terrain annotation.

        f annotates the encounter but does NOT decide whether to operate.
        f = -1: no shared neighbors (topo distant)
        f = 0: topo nearly equivalent (safe fold zone)
        f > 0: gray zone to negate zone
        """
        active = set(self.k_active.active_vertex_ids())
        s = {v, w}
        n_loop = sum(
            1 for e in self.k_active.active_edges()
            if e.source in s and e.target in s
        )
        lower_link: set[str] = set()
        for x in s:
            for n in self.k_active.neighbors(x):
                if n not in s and n in active:
                    lower_link.add(n)
        if not lower_link:
            return -1 + n_loop  # c=0 → f = -1 + n_loop
        ll_edges: list[frozenset[str]] = []
        for e in self.k_active.active_edges():
            if e.source in lower_link and e.target in lower_link:
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link), ll_edges)
        return (c - 1) + n_loop

    # -- encounter detection (topological rules) ----------------------------

    def detect_encounter(self) -> Encounter:
        """Detect encounter at current position.

        Phase 1 (use_llm=False, use_f_criterion=False): topological rules.
        Phase 2 (use_llm=True): LLM semantic analysis.
        f-criterion mode (use_f_criterion=True): f(v,w) as intrinsic criterion.
        """
        if self._use_llm:
            return self._detect_encounter_llm()
        if self._use_f_criterion:
            return self._detect_encounter_f_criterion()
        return self._detect_encounter_topo()

    def _detect_encounter_llm(self) -> Encounter:
        """Detect encounter using LLM semantic analysis of vertex content."""
        from llm_encounter import detect_encounter_llm

        beta_1 = compute_beta_1(self.k_active)
        settled_count = len(self.settlement.settled_cycles)

        result = detect_encounter_llm(
            position=self.position,
            graph=self.k_active,
            terrain=self.terrain,
            beta_1=beta_1,
            settled_count=settled_count,
            pending_negations=self._pending_negations,
            conservative=self._conservative_prompt,
            visit_history=self.visit_history,
        )

        self._llm_log.append({
            "step": self.step + 1,
            "position": self.position,
            "action": result.action,
            "target_a": result.target_a,
            "target_b": result.target_b,
            "reasoning": result.reasoning,
        })

        active = set(self.k_active.active_vertex_ids())
        neighbors = set(self.k_active.neighbors(self.position))

        if result.action == "SUBLATE" and result.target_a and result.target_b:
            # Validate: pair must be in pending negations
            pair_valid = any(
                (t == result.target_a and a == result.target_b)
                or (t == result.target_b and a == result.target_a)
                for t, a in self._pending_negations
            )
            if pair_valid:
                return Encounter(
                    EncounterType.SUBLATION, result.target_a, result.target_b,
                    f"LLM sublation: {result.reasoning}",
                )

        if result.action == "NEGATE" and result.target_a and result.target_b:
            a, b = result.target_a, result.target_b
            # Validate: a is current position, b is neighbor (or vice versa)
            if a in active and b in active and (a == self.position or b == self.position):
                if a != self.position:
                    a, b = b, a  # ensure a = current position
                # Check if b is a neighbor and negation edge doesn't already exist
                if b in neighbors:
                    has_neg = any(
                        e.edge_type == EdgeType.NEGATION
                        and ((e.source == a and e.target == b) or (e.source == b and e.target == a))
                        for e in self.k_active.active_edges()
                    )
                    if not has_neg:
                        return Encounter(
                            EncounterType.NEGATE_A, a, b,
                            f"LLM negation: {result.reasoning}",
                        )

        if result.action == "FOLD" and result.target_a and result.target_b:
            a, b = result.target_a, result.target_b
            # Validate: both must be active, distinct vertices.
            # Accept fold if:
            #   (1) both are neighbors of current position (original rule), OR
            #   (2) a is current position and b is any active vertex (LLM history-based fold)
            if a in active and b in active and a != b:
                if (a in neighbors and b in neighbors) or \
                   (a == self.position) or (b == self.position):
                    return Encounter(
                        EncounterType.FOLD, a, b,
                        f"LLM fold: {result.reasoning}",
                    )

        return Encounter(EncounterType.NOTHING, reason=f"LLM: {result.reasoning}")

    def _detect_encounter_topo(self) -> Encounter:
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
        #    Filter: at least one edge in the bidirectional pair must be critical (non-tree)
        #    in Morse terrain. Tree edges are hub-internal redundancy; critical edges
        #    participate in irreducible cycles. Zero parameters — uses intrinsic
        #    topological structure from already-computed terrain.
        neighbors = self.k_active.neighbors(pos)
        out_nbs = self.k_active.out_neighbors(pos)
        in_nbs = self.k_active.in_neighbors(pos)
        for nb in neighbors:
            if nb == pos:
                continue
            if nb in out_nbs and nb in in_nbs:
                # Morse critical filter: at least one direction must be a critical edge
                fwd_mark = self.terrain.get((pos, nb), "critical")  # default critical if not in terrain
                rev_mark = self.terrain.get((nb, pos), "critical")
                if fwd_mark == "tree" and rev_mark == "tree":
                    continue  # both tree edges → hub-internal redundancy, not contradiction
                # Bidirectional relationship with topological weight — contradiction signal
                # But skip if already negation edge between them
                has_neg = any(
                    e.edge_type == EdgeType.NEGATION
                    and ((e.source == pos and e.target == nb) or (e.source == nb and e.target == pos))
                    for e in self.k_active.active_edges()
                )
                if not has_neg:
                    f_val = self._compute_f(pos, nb)
                    return Encounter(
                        EncounterType.NEGATE_A, pos, nb,
                        f"Bidirectional {pos}<->{nb} with critical edge ({fwd_mark}/{rev_mark}) f={f_val}",
                        f_value=f_val,
                    )

        # 3. Fold: current position shares multiple neighbors with a previously visited vertex
        # (Nachträglichkeit — identity detected across traversal history, not local neighborhood)
        if len(self.visit_history) > 3:
            pos_nbs = set(self.k_active.neighbors(pos))
            seen: set[str] = set()
            for past_vid in reversed(self.visit_history[:-1]):
                if past_vid in seen or past_vid == pos or past_vid not in active:
                    continue
                seen.add(past_vid)
                past_nbs = set(self.k_active.neighbors(past_vid))
                shared = (pos_nbs & past_nbs) - {pos, past_vid}
                if len(shared) >= 2:  # share at least 2 neighbors = structural similarity
                    f_val = self._compute_f(pos, past_vid)
                    return Encounter(
                        EncounterType.FOLD, pos, past_vid,
                        f"Current {pos} and historical {past_vid} share {len(shared)} neighbors: {shared} f={f_val}",
                        f_value=f_val,
                    )
                if len(seen) >= 15:
                    break

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

    def _detect_encounter_f_criterion(self) -> Encounter:
        """Detect encounter using f(v,w) as the intrinsic criterion.

        f(v,w) <= f_fold_threshold  -> fold candidate
        f(v,w) >= f_negate_threshold -> negate candidate
        Sublation: same as rule-based (pending negation pair + contested vertex).
        """
        pos = self.position
        active = set(self.k_active.active_vertex_ids())

        # 1. Sublation: contested vertex with pending negation pair
        v = self.k_active.vertex(pos)
        if v and v.status == VertexStatus.CONTESTED:
            for thesis, antithesis in self._pending_negations:
                if pos == thesis or pos == antithesis:
                    return Encounter(
                        EncounterType.SUBLATION, thesis, antithesis,
                        f"Contested vertex {pos} with pending negation {thesis}-{antithesis}",
                    )

        # 2. Compute f(pos, nb) for all active neighbors
        neighbors = self.k_active.neighbors(pos)
        f_scores: list[tuple[str, int]] = []
        for nb in neighbors:
            if nb == pos or nb not in active:
                continue
            f_val = self._compute_f(pos, nb)
            f_scores.append((nb, f_val))

        if not f_scores:
            return Encounter(EncounterType.NOTHING, reason="No active neighbors for f-criterion")

        # 3. Negate candidates: f >= threshold, bidirectional edge, no existing negation
        pos_is_synthetic = pos.startswith("syn_") or pos.startswith("anti_")
        out_nbs = set(self.k_active.out_neighbors(pos))
        in_nbs = set(self.k_active.in_neighbors(pos))

        best_negate: Optional[tuple[str, int]] = None
        for nb, f_val in f_scores:
            if f_val < self._f_negate_threshold:
                continue
            nb_is_synthetic = nb.startswith("syn_") or nb.startswith("anti_")
            if pos_is_synthetic or nb_is_synthetic:
                continue
            if nb not in out_nbs or nb not in in_nbs:
                continue
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
            return Encounter(
                EncounterType.NEGATE_A, pos, nb,
                f"f-criterion negate: f({pos[:16]},{nb[:16]})={f_val} >= {self._f_negate_threshold}",
                f_value=f_val,
            )

        # 4. Fold candidates: f <= threshold, both non-synthetic
        best_fold: Optional[tuple[str, int]] = None
        for nb, f_val in f_scores:
            if f_val > self._f_fold_threshold:
                continue
            nb_is_synthetic = nb.startswith("syn_") or nb.startswith("anti_")
            if pos_is_synthetic or nb_is_synthetic:
                continue
            if best_fold is None or f_val < best_fold[1]:
                best_fold = (nb, f_val)

        if best_fold is not None:
            nb, f_val = best_fold
            return Encounter(
                EncounterType.FOLD, pos, nb,
                f"f-criterion fold: f({pos[:16]},{nb[:16]})={f_val} <= {self._f_fold_threshold}",
                f_value=f_val,
            )

        # 5. Negate_B: many high-f neighbors, vertex not contested
        high_f_count = sum(1 for _, f_val in f_scores if f_val >= self._f_negate_threshold)
        if high_f_count >= 2 and v and v.status == VertexStatus.ACTIVE:
            return Encounter(
                EncounterType.NEGATE_B, pos, None,
                f"f-criterion tension: {high_f_count} neighbors with f >= {self._f_negate_threshold}",
            )

        # 6. Nothing
        return Encounter(EncounterType.NOTHING, reason="No f-criterion encounter")

    # -- operation execution ------------------------------------------------

    def execute_encounter(self, enc: Encounter) -> tuple[str, bool]:
        """Execute the operation corresponding to an encounter.

        Returns (operation_name, blocked).
        """
        if enc.encounter_type == EncounterType.SUBLATION:
            # Build synthesis content from source vertices + negation edge
            v_a = self.k_active.vertex(enc.target_a)
            v_b = self.k_active.vertex(enc.target_b)
            content_a = (v_a.content or enc.target_a) if v_a else enc.target_a
            content_b = (v_b.content or enc.target_b) if v_b else enc.target_b

            negation_surface = ""
            for e in self.k_active.active_edges():
                if e.edge_type == EdgeType.NEGATION and (
                    (e.source == enc.target_a and e.target == enc.target_b)
                    or (e.source == enc.target_b and e.target == enc.target_a)
                ):
                    negation_surface = e.surface or ""
                    break

            contradiction = negation_surface if negation_surface else f"{content_a} vs {content_b}"
            synthesis_content = f"[{content_a}] + [{content_b}] → sublated via: {contradiction}"

            sublation_record = SublationRecord(
                source_a_id=enc.target_a,
                source_b_id=enc.target_b,
                contradiction=contradiction,
                negated=f"incompatibility between {content_a} and {content_b}",
                preserved=f"{content_a}; {content_b}",
                elevated=synthesis_content,
            )

            result = sublate(
                self.k_active, enc.target_a, enc.target_b, self.step, self.settlement,
                synthesis_content=synthesis_content,
                sublation_record=sublation_record,
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

    @property
    def _nothing_threshold(self) -> int:
        """Adaptive threshold for nothing-streak jump.

        Capped at 20 to prevent dead zones in large graphs.
        Previous bug: sqrt(16827) = 129, making the jump unreachable
        while the walker oscillated in synthetic vertex traps.
        """
        return max(3, min(20, int(len(self.k_active.active_vertex_ids()) ** 0.5)))

    def walk(self) -> None:
        """Move to an adjacent vertex. Prefer critical edges, then unvisited, then random.

        Anti-oscillation: if stuck in nothing streak >= threshold, jump to least-visited active vertex.
        """
        if self._nothing_streak >= self._nothing_threshold:
            # Jump to least-visited active vertex to escape local trap
            active = self.k_active.active_vertex_ids()
            # Count visits efficiently using Counter over history
            visit_counts = {}
            for h in self.visit_history:
                if h in visit_counts:
                    visit_counts[h] += 1
                else:
                    visit_counts[h] = 1
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
            f_value=enc.f_value,
        )
        self.logs.append(log)
        return log
