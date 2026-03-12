"""Traversal engine — walk + topological encounter rules.

Phase 1: encounters detected by topological rules.
Phase 2: encounters detected by LLM via semantic analysis of vertex content.
"""

from __future__ import annotations

import random
from collections import deque
from dataclasses import dataclass
from enum import Enum
from typing import Optional

from engine import (
    Graph, Vertex, Edge, VertexStatus, EdgeType, SettlementTracker, SettledCycle,
    OperationResult, SublationRecord, CONCEPT_EDGE_TYPES,
    compute_beta_1, fold, negate, sublate, _connected_components,
)
from encounter_log import EncounterLog
from morse import compute_terrain, critical_neighbors
from snet_activation import SNetActivation, EdgeSuggestion

EXPLORATION_INTERVAL = 1000  # every N steps, jump to an under-explored vertex
# 425号→431号: ARTICULATION_THRESHOLD 已移除。遭遇触发改为拓扑不一致判据（布尔），不再使用度量阈值。

# -- norm violation → code gap mapping ------------------------------------

def _norm_to_code_gap(violation) -> dict | None:
    """Map a NormViolation to a code gap description, if applicable.

    Returns dict with keys: file, gap, direction — or None if the violation
    does not map to a specific code-level gap.
    """
    norm_text = violation.norm.operational_norm

    if norm_text == "每次settlement的residue非空":
        return {
            "file": "topological-computation/engine.py",
            "gap": "settlement produces empty residue — closure without transformation",
            "direction": "Ensure _compute_residue always produces non-empty residue after settlement",
        }

    if norm_text == "unsettled_count > 0":
        return {
            "file": "topological-computation/engine.py",
            "gap": "all cycles settled, no unsettled cycles remain — heat death",
            "direction": "Add mechanism to generate new cycles when all are settled",
        }

    if norm_text == "settled_cycle_count increases over time":
        return {
            "file": "topological-computation/traversal.py",
            "gap": "prolonged nothing-streak indicates traversal cannot produce new settlements",
            "direction": "Improve encounter detection or walker strategy for stagnant regions",
        }

    return None


class EncounterType(str, Enum):
    FOLD = "fold"
    NEGATE_A = "negate_a"      # existing antithesis
    NEGATE_B = "negate_b"      # create new antithesis
    SUBLATION = "sublation"
    NOTHING = "nothing"
    ARTICULATE = "articulate"  # 425号 Phase 2: 物质層積累涌現為概念層連接（ça parle）


@dataclass(frozen=True, slots=True)
class Encounter:
    encounter_type: EncounterType
    target_a: Optional[str] = None
    target_b: Optional[str] = None
    reason: str = ""
    f_value: int = -99  # terrain annotation: f(target_a, target_b) if applicable
    g_value: int = -99  # vertex-disjoint path count g(target_a, target_b) — 392·2 annotation


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
    g_value: int = -99  # g(v,w) vertex-disjoint path count — 392·2 annotation
    resonance: bool = False  # S_net 耦合振荡：chosen candidate was in resonating set
    exploration: bool = False  # exploration move: jumped to under-explored vertex


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
        encounter_log: EncounterLog | None = None,
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
        self._blocked_streak = 0  # consecutive blocked operation steps
        self._blocked_at: dict[str, int] = {}  # position -> last blocked step (skip encounter there)
        self._last_blocked_by: SettledCycle | None = None  # last cycle that blocked an operation
        self._use_llm = use_llm
        self._conservative_prompt = conservative_prompt
        self._llm_log: list[dict] = []  # LLM call log for debugging
        self._use_f_criterion = use_f_criterion
        self._f_fold_threshold = f_fold_threshold
        self._f_negate_threshold = f_negate_threshold
        self._encounter_log = encounter_log
        self._attempted_folds: set[frozenset[str]] = set()  # 398号: fold pairs already attempted & blocked
        self._snet_activation: SNetActivation | None = None  # 耦合振荡: S_net 激活态
        self._last_resonance: bool = False  # 上一步选择的候选是否处于共振区域
        self._last_articulation: dict | None = None  # 上一步的 articulation 溯源元数据

    def _invalidate_attempted_folds(self, affected_vertices: set[str]) -> None:
        """Remove attempted-fold entries involving any of the affected vertices.

        Called when graph structure changes (new edges, new vertices, settlement
        revocation) that may alter neighborhood relationships, making previously
        blocked fold pairs retryable.
        """
        if not affected_vertices or not self._attempted_folds:
            return
        self._attempted_folds = {
            pair for pair in self._attempted_folds
            if not pair & affected_vertices
        }

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

    def _compute_g(self, v: str, w: str) -> int:
        """Compute g(v,w) = vertex-disjoint path count (Menger) for 392·2 annotation.

        Uses iterative BFS path-removal on undirected view of k_active.
        g annotates encounter robustness but does NOT decide whether to operate.
        """
        active = set(self.k_active.active_vertex_ids())
        if v not in active or w not in active:
            return 0

        # Build undirected adjacency from active edges
        adj: dict[str, set[str]] = {vid: set() for vid in active}
        for e in self.k_active.active_edges():
            if e.source in active and e.target in active and e.source != e.target:
                adj[e.source].add(e.target)
                adj[e.target].add(e.source)

        count = 0
        removed: set[str] = set()

        for _ in range(50):  # cap at 50 disjoint paths
            visited: set[str] = set()
            parent: dict[str, str] = {}
            queue = deque([v])
            found = False

            while queue:
                cur = queue.popleft()
                if cur == w:
                    found = True
                    break
                if cur in visited:
                    continue
                visited.add(cur)
                for nb in adj.get(cur, set()):
                    if nb not in visited and nb not in removed:
                        if nb not in parent:
                            parent[nb] = cur
                            queue.append(nb)

            if not found:
                break

            count += 1
            # Remove internal vertices of this path (not v or w)
            node = w
            while node in parent:
                prev = parent[node]
                if prev != v and node != w:
                    removed.add(node)
                node = prev

        return count

    # -- encounter detection (topological rules) ----------------------------

    def detect_encounter(self) -> Encounter:
        """Detect encounter at current position.

        Phase 1 (use_llm=False, use_f_criterion=False): topological rules.
        Phase 2 (use_llm=True): LLM semantic analysis.
        f-criterion mode (use_f_criterion=True): f(v,w) as intrinsic criterion.

        Proprioception override: when position is a proprioception vertex,
        check self-reflexive norms. Violated norms produce structural encounters.
        """
        proprioception_enc = self._check_proprioception_encounter()
        if proprioception_enc is not None:
            return proprioception_enc

        if self._use_llm:
            return self._detect_encounter_llm()
        if self._use_f_criterion:
            return self._detect_encounter_f_criterion()
        return self._detect_encounter_topo()

    def _check_proprioception_encounter(self) -> Encounter | None:
        """Check self-reflexive norms when at a proprioception vertex.

        If the current position is a proprioception vertex, parse current metrics
        from the graph's proprioception vertices, check norms, and produce a
        negate_b encounter (structural forcing) if any norm is violated.

        Returns None if not at a proprioception vertex or no violations detected.
        """
        if not self.position.startswith("proprioception:"):
            return None

        from proprioception import (
            check_norms, ProprioceptionMetrics, PROPRIOCEPTION_PREFIX,
        )

        # Collect metrics from proprioception vertices in the graph
        metric_values: dict[str, int] = {}
        for vid in self.k_active.active_vertex_ids():
            if vid.startswith("proprioception:"):
                v = self.k_active.vertex(vid)
                if v and v.content and v.content.startswith(PROPRIOCEPTION_PREFIX):
                    key = vid.split(":", 1)[1]
                    parts = v.content.split("=", 1)
                    if len(parts) == 2:
                        try:
                            metric_values[key] = int(parts[1].strip())
                        except ValueError:
                            pass

        if not metric_values:
            return None

        metrics = ProprioceptionMetrics(
            settled_cycle_count=metric_values.get("settled_cycle_count", 0),
            total_vertices=metric_values.get("total_vertices", 0),
            total_edges=metric_values.get("total_edges", 0),
            nothing_streak=metric_values.get("nothing_streak", 0),
            expression_pressure=metric_values.get("expression_pressure", 0),
            blocked_streak=metric_values.get("blocked_streak", 0),
        )

        # Total cycles = pending + settled
        total_cycles = len(self.settlement.settled_cycles) + len(self.settlement._pending)
        violations = check_norms(metrics, total_cycles)

        if not violations:
            return None

        # Most severe violation becomes a structural forcing encounter (negate_b)
        critical = [v for v in violations if v.severity == "critical"]
        chosen = critical[0] if critical else violations[0]

        # Emit code_settlement_request for structural violations that map to code gaps.
        # The norm→code mapping uses the norm's diagnosis to identify the relevant file.
        if chosen.severity in ("critical", "warning"):
            code_mapping = _norm_to_code_gap(chosen)
            if code_mapping is not None:
                self._emit_code_settlement_request(
                    diagnosed_file=code_mapping["file"],
                    gap_description=code_mapping["gap"],
                    proposed_direction=code_mapping["direction"],
                    theoretical_basis=chosen.norm.theoretical_concept,
                    norm_violation={
                        "norm": chosen.norm.operational_norm,
                        "actual_state": chosen.actual_state,
                        "severity": chosen.severity,
                    },
                )

        return Encounter(
            EncounterType.NEGATE_B,
            self.position,
            None,
            f"Self-diagnosis: {chosen.norm.theoretical_concept} violated — "
            f"{chosen.norm.diagnosis} ({chosen.actual_state})",
        )

    def _emit_code_settlement_request(
        self,
        diagnosed_file: str,
        gap_description: str,
        proposed_direction: str,
        theoretical_basis: str,
        norm_violation: dict,
    ) -> dict | None:
        """逢亮的提案权：write code_settlement_request to encounter_log.

        When self-diagnosis via proprioception detects a norm violation and
        the traversal is in the self domain (code vertices), this emits a
        code_settlement_request. The request is proposal-only — execution
        requires operator approval through ceremony_scan → CC workflow.

        去重：同一 gap_description 已有 pending request 时不重复写入。
        使用实例级缓存避免每步读取全量 JSONL。

        Returns the request dict if written, None if no encounter_log available.
        """
        if self._encounter_log is None:
            return None
        # 去重：实例级缓存已提交的 gap_description
        if not hasattr(self, '_emitted_gaps'):
            # 首次调用时从 JSONL 加载已有 pending 的 gap
            existing = self._encounter_log.pending_code_settlement_requests()
            self._emitted_gaps: set[str] = {
                req.get("gap_description", "") for req in existing
            }
        if gap_description in self._emitted_gaps:
            return None
        self._emitted_gaps.add(gap_description)
        return self._encounter_log.record_code_settlement_request(
            diagnosed_file=diagnosed_file,
            gap_description=gap_description,
            proposed_direction=proposed_direction,
            theoretical_basis=theoretical_basis,
            norm_violation=norm_violation,
        )

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
                # 398号: skip already-attempted & blocked fold pairs
                if frozenset((a, b)) in self._attempted_folds:
                    pass  # fall through to NOTHING
                elif (a in neighbors and b in neighbors) or \
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
                    g_val = self._compute_g(pos, nb)
                    return Encounter(
                        EncounterType.NEGATE_A, pos, nb,
                        f"Bidirectional {pos}<->{nb} with critical edge ({fwd_mark}/{rev_mark}) f={f_val} g={g_val}",
                        f_value=f_val,
                        g_value=g_val,
                    )

        # 3. Fold: two detection paths (393号 categorical criterion)
        #    Path A (393·4): f(pos, nb) == 0 for any active neighbor → categorical fold
        #       f=0 means (c=1 ∧ n_loop=0) — shared neighbors fully connected, no loops.
        #       This is a categorical sufficient condition, not a continuous threshold.
        #    Path B (original): current position shares ≥2 neighbors with historical vertex
        #       (Nachträglichkeit — identity detected across traversal history)

        # Path A: f=0 categorical fold (393·4)
        for nb in neighbors:
            if nb == pos or nb not in active:
                continue
            nb_is_synthetic = nb.startswith("syn_") or nb.startswith("anti_")
            if pos.startswith("syn_") or pos.startswith("anti_") or nb_is_synthetic:
                continue
            pair_key = frozenset((pos, nb))
            if pair_key in self._attempted_folds:
                continue
            f_val = self._compute_f(pos, nb)
            if f_val == 0:
                return Encounter(
                    EncounterType.FOLD, pos, nb,
                    f"f=0 categorical fold: {pos} and {nb} are topologically equivalent (c=1, n_loop=0)",
                    f_value=f_val,
                )

        # Path B: shared neighbor fold (Nachträglichkeit)
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
                    pair_key = frozenset((pos, past_vid))
                    if pair_key in self._attempted_folds:
                        continue  # 398号: already attempted & blocked, skip
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
            # 398号: skip already-attempted & blocked fold pairs
            if frozenset((pos, nb)) in self._attempted_folds:
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

            try:
                result = sublate(
                    self.k_active, enc.target_a, enc.target_b, self.step, self.settlement,
                    synthesis_content=synthesis_content,
                    sublation_record=sublation_record,
                )
            except ValueError:
                # Edge may have been removed between detection and execution
                return "walk", False
            if result.blocked:
                self.settlement.record_blocked(
                    self.step, "sublate",
                    {"thesis": enc.target_a, "antithesis": enc.target_b},
                    result.blocked_by,
                )
                self._last_blocked_by = result.blocked_by
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
            # 398号: sublation changes graph structure — invalidate affected fold pairs
            self._invalidate_attempted_folds({enc.target_a, enc.target_b, result.new_vertex})
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
                self._last_blocked_by = result.blocked_by
                return "negate_blocked", True

            self.k_active = result.graph
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)

            self._pending_negations.append((enc.target_a, enc.target_b))
            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            # 398号: negate adds edges — invalidate affected fold pairs
            self._invalidate_attempted_folds({enc.target_a, enc.target_b})
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
                self._last_blocked_by = result.blocked_by
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
            # 398号: negate_b adds vertex+edges — invalidate affected fold pairs
            affected = {enc.target_a}
            if result.new_vertex:
                affected.add(result.new_vertex)
            self._invalidate_attempted_folds(affected)
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
                self._last_blocked_by = result.blocked_by
                # 398号: remember this blocked fold pair
                self._attempted_folds.add(frozenset((enc.target_a, enc.target_b)))
                return "fold_blocked", True

            self.k_active = result.graph
            # Update K_full with fold record
            for e in result.graph.edges:
                if e.created_at == self.step and e not in self.k_full.edges:
                    self.k_full = self.k_full.add_edge(e)
            # If position was folded away, move to kept vertex
            if self.position == enc.target_b:
                self.position = enc.target_a

            # 398号: fold succeeded — clear attempted_folds involving merged vertices
            # (graph structure changed, old fold-block reasons may no longer hold)
            merged_away = enc.target_b  # target_b is the vertex removed by fold
            self._attempted_folds = {
                pair for pair in self._attempted_folds
                if merged_away not in pair
            }

            if result.new_cycle_edges:
                self.settlement.register_new_cycle(result.new_cycle_edges, self.step)
            return "fold", False

        return "nothing", False

    # -- S_net coupling -----------------------------------------------------

    def set_snet_activation(self, snet_activation: SNetActivation) -> None:
        """Set S_net activation for coupled oscillation."""
        self._snet_activation = snet_activation

    def _resonating_candidates(self) -> set[str]:
        """Get K_active concept IDs in the S_net resonating zone."""
        if self._snet_activation is None:
            return set()
        return self._snet_activation.get_resonating_concepts()

    # -- walk ---------------------------------------------------------------

    @property
    def _nothing_threshold(self) -> int:
        """Adaptive threshold for nothing-streak jump.

        Capped at 20 to prevent dead zones in large graphs.
        Previous bug: sqrt(16827) = 129, making the jump unreachable
        while the walker oscillated in synthetic vertex traps.
        """
        return max(3, min(20, int(len(self.k_active.active_vertex_ids()) ** 0.5)))

    def _find_free_zone_target(self) -> str | None:
        """Find a high-degree vertex not in any settled cycle's edge set.

        When all operations are blocked by settled cycles, the traverser
        needs to escape to a region where operations can succeed. A "free"
        vertex is one whose edges are not part of any settled cycle.

        Returns the highest-degree free vertex, or None if all are locked.
        """
        # Collect all vertex IDs that participate in settled cycle edges
        locked_vids: set[str] = set()
        for sc in self.settlement.settled_cycles:
            for src, tgt in sc.edges:
                locked_vids.add(src)
                locked_vids.add(tgt)

        active = self.k_active.active_vertex_ids()
        free_vids = [v for v in active if v not in locked_vids]

        if not free_vids:
            return None

        # Pick highest-degree free vertex (most neighbors = most encounter potential)
        degrees = {v: len(self.k_active.neighbors(v)) for v in free_vids}
        return max(free_vids, key=lambda v: (degrees[v], v))

    def _exploration_target(self) -> str | None:
        """Pick an under-explored vertex for exploration move.

        Priority: unreviewed concept_creation_suggestions (gap queue),
        then low-degree (≤2) active vertices not recently visited.
        Returns None if no suitable target found.
        """
        # 1. Gap queue: unreviewed concept_creation_suggestions → anchor_concept
        if self._snet_activation is not None:
            for suggestion in self._snet_activation.concept_creation_suggestions:
                if not suggestion.reviewed:
                    anchor = suggestion.anchor_concept
                    if anchor in self.k_active.active_vertex_ids() and anchor != self.position:
                        return anchor

        # 2. Low-degree active vertices (degree ≤ 2), not recently visited
        active = self.k_active.active_vertex_ids()
        recent = set(self.visit_history[-100:]) if len(self.visit_history) > 100 else set(self.visit_history)
        low_degree = [
            v for v in active
            if len(self.k_active.neighbors(v)) <= 2
            and v not in recent
            and v != self.position
        ]
        if low_degree:
            return self.rng.choice(low_degree)

        return None

    def _is_code_vertex(self, vid: str) -> bool:
        """Check if vertex belongs to code domain (content starts with [domain:code] or [domain:self])."""
        v = self.k_active.vertex(vid)
        if v is None or v.content is None:
            return False
        return v.content.startswith("[domain:code]") or v.content.startswith("[domain:self]")

    def _try_residue_escape(self, blocked_by: SettledCycle | None) -> bool:
        """Attempt to escape a settled cycle block using residue paths.

        When an operation is blocked by a settled cycle, check its residue:
        - boundary_edge: move to the external vertex (prefer different domain)
        - nachtraeglichkeit: move into the prior settlement's affected region

        Returns True if escape succeeded (position changed), False otherwise.
        """
        if blocked_by is None or not blocked_by.residue:
            return False

        active = set(self.k_active.active_vertex_ids())

        # Collect boundary_edge targets (external vertices)
        boundary_targets: list[str] = []
        for item in blocked_by.residue:
            if item["type"] == "boundary_edge":
                ev = item["data"].get("external_vertex")
                if ev and ev in active and ev != self.position:
                    boundary_targets.append(ev)

        if boundary_targets:
            # Prefer boundary edge leading to a different domain
            current_is_code = self._is_code_vertex(self.position)
            cross_domain = [v for v in boundary_targets if self._is_code_vertex(v) != current_is_code]
            target = self.rng.choice(cross_domain) if cross_domain else self.rng.choice(boundary_targets)
            self.position = target
            self.visit_history.append(self.position)
            return True

        # Collect nachtraeglichkeit targets (affected vertices in prior settlements)
        nachtraeg_targets: list[str] = []
        for item in blocked_by.residue:
            if item["type"] == "nachtraeglichkeit":
                for v in item["data"].get("affected_vertices", []):
                    if v in active and v != self.position:
                        nachtraeg_targets.append(v)

        if nachtraeg_targets:
            target = self.rng.choice(nachtraeg_targets)
            self.position = target
            self.visit_history.append(self.position)
            return True

        return False

    def _pull_cooccurrence_edges(self) -> None:
        """Pull S_net co-occurrence neighbors into K_active as COOCCURRENCE edges on demand.

        425号 Phase 1: 穿越到新节点时，查询 S_net 该节点对应能指的共现邻居。
        共现邻居如果对应 K_active 中的已有顶点 → 添加 COOCCURRENCE 边（不创建概念层边）。
        COOCCURRENCE 边参与导航（neighbors/all_active_edges）但不参与 fold/negate/sublate/beta_1。

        Only pulls for the current position. Edges are deduplicated — if a COOCCURRENCE edge
        already exists between two vertices, it is not added again.
        """
        if self._snet_activation is None:
            return

        concept_id = self.position
        signifier_id = self._snet_activation._concept_to_sig.get(concept_id)
        if not signifier_id:
            return

        snet = self._snet_activation.s_net
        # Get degree-normalized co-occurrence neighbors (top 10)
        cooc_neighbors = snet.degree_normalized_neighbors(signifier_id, n=10)
        if not cooc_neighbors:
            return

        active_vids = set(self.k_active.active_vertex_ids())

        # Build set of existing COOCCURRENCE edge targets from this vertex for dedup
        existing_cooc_targets: set[str] = set()
        for e in self.k_active.all_active_edges():
            if e.edge_type == EdgeType.COOCCURRENCE and e.source == concept_id:
                existing_cooc_targets.add(e.target)
            elif e.edge_type == EdgeType.COOCCURRENCE and e.target == concept_id:
                existing_cooc_targets.add(e.source)

        for edge in cooc_neighbors:
            target_sig = edge.target
            # Map signifier back to K_active concept(s)
            target_concepts = self._snet_activation._sig_to_concepts.get(target_sig, [])

            if target_concepts:
                # Existing K_active vertices — add COOCCURRENCE edge if not present
                for tgt_cid in target_concepts:
                    if tgt_cid == concept_id:
                        continue
                    if tgt_cid not in active_vids:
                        continue
                    if tgt_cid in existing_cooc_targets:
                        continue
                    cooc_edge = Edge(
                        source=concept_id,
                        target=tgt_cid,
                        edge_type=EdgeType.COOCCURRENCE,
                        created_at=self.step,
                        surface=edge.evidence if edge.evidence else None,
                        context=f"snet_cooccurrence: {signifier_id}->{target_sig} w={edge.weight:.3f}",
                    )
                    self.k_active = self.k_active.add_edge(cooc_edge)
                    self.k_full = self.k_full.add_edge(cooc_edge)
                    existing_cooc_targets.add(tgt_cid)
            else:
                # No K_active vertex for this signifier — create one + COOCCURRENCE edge
                new_vid = f"cooc_{target_sig}"
                if new_vid in active_vids:
                    if new_vid not in existing_cooc_targets:
                        cooc_edge = Edge(
                            source=concept_id,
                            target=new_vid,
                            edge_type=EdgeType.COOCCURRENCE,
                            created_at=self.step,
                            surface=edge.evidence if edge.evidence else None,
                            context=f"snet_cooccurrence: {signifier_id}->{target_sig} w={edge.weight:.3f}",
                        )
                        self.k_active = self.k_active.add_edge(cooc_edge)
                        self.k_full = self.k_full.add_edge(cooc_edge)
                        existing_cooc_targets.add(new_vid)
                    continue
                new_vertex = Vertex(
                    id=new_vid,
                    status=VertexStatus.ACTIVE,
                    content=target_sig,
                    created_at=self.step,
                )
                self.k_active = self.k_active.add_vertex(new_vertex)
                self.k_full = self.k_full.add_vertex(new_vertex)
                cooc_edge = Edge(
                    source=concept_id,
                    target=new_vid,
                    edge_type=EdgeType.COOCCURRENCE,
                    created_at=self.step,
                    surface=edge.evidence if edge.evidence else None,
                    context=f"snet_cooccurrence: {signifier_id}->{target_sig} w={edge.weight:.3f}",
                )
                self.k_active = self.k_active.add_edge(cooc_edge)
                self.k_full = self.k_full.add_edge(cooc_edge)
                # Update mapping so future lookups find this vertex
                self._snet_activation._sig_to_concepts.setdefault(target_sig, []).append(new_vid)
                active_vids.add(new_vid)
                existing_cooc_targets.add(new_vid)

    def _record_traversal_association(self, from_vid: str, to_vid: str) -> None:
        """Record a TRAVERSAL_ASSOCIATION edge between two vertices (material layer sediment).

        425号 Phase 1: 穿越步进后，记录穿越路径为 TRAVERSAL_ASSOCIATION 边。
        Only records if both vertices have signifier mappings in S_net.
        TRAVERSAL_ASSOCIATION edges are immutable, participate in navigation but not
        fold/negate/sublate/beta_1.
        """
        if self._snet_activation is None:
            return

        from_sig = self._snet_activation._concept_to_sig.get(from_vid)
        to_sig = self._snet_activation._concept_to_sig.get(to_vid)
        if not from_sig or not to_sig:
            return

        ta_edge = Edge(
            source=from_vid,
            target=to_vid,
            edge_type=EdgeType.TRAVERSAL_ASSOCIATION,
            created_at=self.step,
            surface=f"{from_sig}->{to_sig}",
            context=f"traversal_step:{self.step}",
        )
        self.k_active = self.k_active.add_edge(ta_edge)
        self.k_full = self.k_full.add_edge(ta_edge)

    def _check_articulation_encounter(self) -> Optional[tuple[Encounter, float, dict]]:
        """Check if topological inconsistency between Layer A and Layer B triggers ARTICULATE.

        431号: 拓扑不一致判据（布尔）替代度量阈值——
        Layer A = 组合轴 (syntagmatic) = COOCCURRENCE 边 + TRAVERSAL_ASSOCIATION 边
        Layer B = 聚合轴 (paradigmatic) = S_net paradigmatic 边

        两种失衡模式：
        - A密B疏：K_active 有 COOCCURRENCE 边，但 S_net 无 paradigmatic 边且 K_active 无概念层边
        - B密A疏：S_net 有 paradigmatic 边，但 K_active 无 COOCCURRENCE 边

        Returns (Encounter, score=1.0, meta) or None.
        """
        if self._snet_activation is None:
            return None

        current = self.position
        active_vids = set(self.k_active.active_vertex_ids())
        snet = self._snet_activation.s_net
        current_sig = self._snet_activation._concept_to_sig.get(current)

        # --- A密B疏: COOCCURRENCE exists but no paradigmatic edge and no concept edge ---
        for e in self.k_active.all_active_edges():
            if e.edge_type != EdgeType.COOCCURRENCE:
                continue
            if e.source == current:
                neighbor = e.target
            elif e.target == current:
                neighbor = e.source
            else:
                continue
            if neighbor not in active_vids:
                continue

            # Check concept layer: if concept edge exists, this pair is already connected
            has_concept_edge = False
            for ce in self.k_active.active_edges():
                if ce.edge_type not in CONCEPT_EDGE_TYPES:
                    continue
                if (ce.source == current and ce.target == neighbor) or \
                   (ce.source == neighbor and ce.target == current):
                    has_concept_edge = True
                    break
            if has_concept_edge:
                continue

            # Check Layer B: paradigmatic edge between corresponding signifiers
            neighbor_sig = self._snet_activation._concept_to_sig.get(neighbor)
            has_paradigmatic = False
            if current_sig and neighbor_sig:
                for pe in snet.paradigmatic_alternatives(current_sig):
                    if pe.target == neighbor_sig:
                        has_paradigmatic = True
                        break
                if not has_paradigmatic:
                    for pe in snet.paradigmatic_alternatives(neighbor_sig):
                        if pe.target == current_sig:
                            has_paradigmatic = True
                            break

            if not has_paradigmatic:
                # A密B疏: cooccurrence exists, no paradigmatic, no concept edge
                enc = Encounter(
                    encounter_type=EncounterType.ARTICULATE,
                    target_a=current,
                    target_b=neighbor,
                    reason=(
                        f"articulation[A密B疏]: cooccurrence between {current} and {neighbor} "
                        f"but no paradigmatic edge in S_net and no concept edge in K_active"
                    ),
                )
                return enc, 1.0, {"imbalance_type": "A_dense_B_sparse"}

        # --- B密A疏: paradigmatic edge exists but no COOCCURRENCE edge ---
        if current_sig:
            for pe in snet.paradigmatic_alternatives(current_sig):
                target_sig = pe.target
                # Map signifier back to K_active concept(s)
                target_concepts = self._snet_activation._sig_to_concepts.get(target_sig, [])
                for tgt_cid in target_concepts:
                    if tgt_cid == current or tgt_cid not in active_vids:
                        continue

                    # Check concept layer: skip if already connected
                    has_concept_edge = False
                    for ce in self.k_active.active_edges():
                        if ce.edge_type not in CONCEPT_EDGE_TYPES:
                            continue
                        if (ce.source == current and ce.target == tgt_cid) or \
                           (ce.source == tgt_cid and ce.target == current):
                            has_concept_edge = True
                            break
                    if has_concept_edge:
                        continue

                    # Check Layer A: any COOCCURRENCE edge?
                    has_cooccurrence = False
                    for ae in self.k_active.all_active_edges():
                        if ae.edge_type != EdgeType.COOCCURRENCE:
                            continue
                        if (ae.source == current and ae.target == tgt_cid) or \
                           (ae.source == tgt_cid and ae.target == current):
                            has_cooccurrence = True
                            break
                    if has_cooccurrence:
                        continue

                    # B密A疏: paradigmatic exists, no cooccurrence, no concept edge
                    enc = Encounter(
                        encounter_type=EncounterType.ARTICULATE,
                        target_a=current,
                        target_b=tgt_cid,
                        reason=(
                            f"articulation[B密A疏]: paradigmatic edge between "
                            f"{current_sig} and {target_sig} in S_net "
                            f"but no cooccurrence edge between {current} and {tgt_cid} in K_active"
                        ),
                    )
                    return enc, 1.0, {"imbalance_type": "B_dense_A_sparse"}

        return None

    def _articulate(self, source_vid: str, target_vid: str, score: float) -> Edge:
        """Create ARTICULATED concept-layer edge from topological inconsistency.

        431号: 拓扑不一致（A密B疏 / B密A疏）涌現為概念層——ARTICULATED 邊參與 fold/negate/sublate。
        """
        new_edge = Edge(
            source=source_vid,
            target=target_vid,
            edge_type=EdgeType.ARTICULATED,
            created_at=self.step,
            surface=None,
            context=f"articulated: score={score:.3f} step={self.step}",
        )
        self.k_active = self.k_active.add_edge(new_edge)
        self.k_full = self.k_full.add_edge(new_edge)
        return new_edge

    def walk(self) -> None:
        """Move to an adjacent vertex. Prefer critical edges, then unvisited, then random.

        Anti-oscillation: if stuck in nothing streak >= threshold, jump to least-visited active vertex.
        Domain awareness: if last 5 steps all in code domain, prefer core/text neighbors.
        S_net coupling: resonating candidates get preference within same priority tier (pull, not override).
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

        # S_net resonating concepts (secondary pull)
        resonating = self._resonating_candidates()

        # Domain awareness: if last 5 positions were all code domain,
        # prefer non-code neighbors to avoid getting stuck in code hubs
        code_streak = (
            len(self.visit_history) >= 5
            and all(self._is_code_vertex(h) for h in self.visit_history[-5:])
        )

        # Prefer critical neighbors
        crit = critical_neighbors(self.k_active, self.position, self.terrain)

        if code_streak:
            # Filter for non-code vertices first
            non_code_crit = [n for n in crit if not self._is_code_vertex(n)]
            non_code_any = [n for n in neighbors if not self._is_code_vertex(n)]
            if non_code_crit:
                chosen = self._pick_with_resonance(non_code_crit, resonating)
                self.position = chosen
                self.visit_history.append(self.position)
                return
            if non_code_any:
                chosen = self._pick_with_resonance(non_code_any, resonating)
                self.position = chosen
                self.visit_history.append(self.position)
                return
            # All neighbors are code — fall through to normal logic

        unvisited = [n for n in crit if n not in self.visit_history[-5:]]
        if unvisited:
            self.position = self._pick_with_resonance(unvisited, resonating)
        elif crit:
            self.position = self._pick_with_resonance(crit, resonating)
        else:
            # Fall back to any neighbor
            unvisited_any = [n for n in neighbors if n not in self.visit_history[-3:]]
            if unvisited_any:
                self.position = self._pick_with_resonance(unvisited_any, resonating)
            else:
                self.position = self._pick_with_resonance(neighbors, resonating)

        self.visit_history.append(self.position)

    def _pick_with_resonance(
        self,
        candidates: list[str],
        resonating: set[str],
    ) -> str:
        """Choose from candidates with resonance as secondary pull.

        If any candidates are in the resonating set, prefer those.
        Otherwise, random choice among all candidates.
        Edge type priority is already handled by the caller (crit > unvisited > any).
        Resonance only distinguishes within the same priority tier.
        """
        if not candidates:
            return self.position
        resonating_candidates = [c for c in candidates if c in resonating]
        if resonating_candidates:
            self._last_resonance = True
            return self.rng.choice(resonating_candidates)
        self._last_resonance = False
        return self.rng.choice(candidates)

    # -- main step ----------------------------------------------------------

    def run_step(self) -> StepLog:
        """Execute one full step: detect encounter → execute or walk → update terrain → settle → S_net sync."""
        self.step += 1
        self._last_resonance = False
        self._last_articulation = None
        explored = False
        prev_position = self.position  # 425号: record for TRAVERSAL_ASSOCIATION

        # Exploration move: periodically jump to under-explored vertex
        if self.step > 1 and self.step % EXPLORATION_INTERVAL == 0:
            target = self._exploration_target()
            if target is not None:
                self.position = target
                self.visit_history.append(self.position)
                explored = True

        beta_before = compute_beta_1(self.k_active)

        # S_net: update current step
        if self._snet_activation is not None:
            self._snet_activation.current_step = self.step

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
                self._blocked_streak += 1

                # 396号: try to escape via residue paths (boundary_edge / nachtraeglichkeit)
                if self._try_residue_escape(self._last_blocked_by):
                    self._blocked_streak = 0
                # Settlement deadlock escape: if blocked too many times consecutively,
                # jump to a vertex outside all settled cycles ("free zone")
                elif self._blocked_streak >= 30:
                    free_target = self._find_free_zone_target()
                    if free_target is not None and free_target != self.position:
                        self.position = free_target
                        self.visit_history.append(self.position)
                    self._blocked_streak = 0
            else:
                self._blocked_streak = 0
        else:
            self._nothing_streak += 1
            self.walk()

        # S_net material layer (Dass): pull COOCCURRENCE edges on demand
        self._pull_cooccurrence_edges()

        # 425号: record TRAVERSAL_ASSOCIATION edge (material layer sediment)
        if prev_position != self.position:
            self._record_traversal_association(prev_position, self.position)

        # 431号: check if topological inconsistency (A密B疏 / B密A疏) triggers ARTICULATE
        articulation_result = self._check_articulation_encounter()
        if articulation_result is not None:
            articulation_enc, articulation_score, articulation_meta = articulation_result
            self._articulate(
                articulation_enc.target_a,
                articulation_enc.target_b,
                articulation_score,
            )
            self._last_articulation = {
                "source_vid": articulation_enc.target_a,
                "target_vid": articulation_enc.target_b,
                "score": articulation_score,
                "step": self.step,
                "imbalance_type": articulation_meta["imbalance_type"],
                "reason": articulation_enc.reason,
            }

        # S_net coupling: activate signifier for new position + check articulation feedback
        if self._snet_activation is not None:
            self._snet_activation.activate(self.position)
            new_suggestions = self._snet_activation.check_articulation_feedback(self.k_active)
            self._snet_activation.edge_suggestions.extend(new_suggestions)

            # Consume unreviewed edge suggestions: create REFERENCE edges in K_active
            # This closes the articulation feedback loop:
            #   S_net co-occurrence edge → EdgeSuggestion → K_active REFERENCE edge
            consumed = []
            for idx, suggestion in enumerate(self._snet_activation.edge_suggestions):
                if suggestion.reviewed:
                    continue
                src = suggestion.source_concept
                tgt = suggestion.target_concept

                # Bridge path: orphan signifier needs a new K_active vertex
                if suggestion.origin == "articulation_bridge" and tgt.startswith("__bridge__"):
                    orphan_sig = tgt[len("__bridge__"):]
                    # Use signifier ID as the vertex ID (bridge_ prefix for protection)
                    bridge_vid = f"bridge_{orphan_sig}"
                    # Check not already created
                    if bridge_vid in self.k_active.active_vertex_ids():
                        consumed.append(idx)
                        continue
                    # Verify anchor concept still active
                    if src not in self.k_active.active_vertex_ids():
                        continue
                    # Create new vertex for orphan signifier
                    bridge_vertex = Vertex(
                        id=bridge_vid,
                        status=VertexStatus.ACTIVE,
                        content=orphan_sig,
                        created_at=self.step,
                    )
                    self.k_active = self.k_active.add_vertex(bridge_vertex)
                    self.k_full = self.k_full.add_vertex(bridge_vertex)
                    # Create REFERENCE edge from anchor to bridge vertex
                    surface = "; ".join(suggestion.evidence_patterns[:3]) if suggestion.evidence_patterns else ""
                    new_edge = Edge(
                        source=src,
                        target=bridge_vid,
                        edge_type=EdgeType.REFERENCE,
                        created_at=self.step,
                        surface=surface,
                        context=f"articulation_bridge from S_net: {suggestion.evidence_signifiers}",
                    )
                    self.k_active = self.k_active.add_edge(new_edge)
                    self.k_full = self.k_full.add_edge(new_edge)
                    # Update _sig_to_concepts mapping for consistency
                    self._snet_activation._sig_to_concepts.setdefault(orphan_sig, []).append(bridge_vid)
                    consumed.append(idx)
                    continue

                # Standard path: both concepts already exist in K_active
                # Verify both concepts still active in K_active
                if src not in self.k_active.active_vertex_ids():
                    continue
                if tgt not in self.k_active.active_vertex_ids():
                    continue
                # Check edge doesn't already exist
                if tgt in self.k_active.neighbors(src):
                    continue
                # Create REFERENCE edge with linguistic evidence
                surface = "; ".join(suggestion.evidence_patterns[:3]) if suggestion.evidence_patterns else ""
                new_edge = Edge(
                    source=src,
                    target=tgt,
                    edge_type=EdgeType.REFERENCE,
                    created_at=self.step,
                    surface=surface,
                    context=f"articulation_feedback from S_net: {suggestion.evidence_signifiers}",
                )
                self.k_active = self.k_active.add_edge(new_edge)
                self.k_full = self.k_full.add_edge(new_edge)
                consumed.append(idx)

            # Mark consumed suggestions as reviewed (immutable dataclass — rebuild list)
            if consumed:
                consumed_set = set(consumed)
                self._snet_activation.edge_suggestions = [
                    EdgeSuggestion(
                        source_concept=s.source_concept,
                        target_concept=s.target_concept,
                        evidence_signifiers=s.evidence_signifiers,
                        evidence_patterns=s.evidence_patterns,
                        origin=s.origin,
                        step=s.step,
                        reviewed=True,
                    ) if i in consumed_set else s
                    for i, s in enumerate(self._snet_activation.edge_suggestions)
                ]

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
            g_value=enc.g_value,
            resonance=self._last_resonance,
            exploration=explored,
        )
        self.logs.append(log)
        return log
