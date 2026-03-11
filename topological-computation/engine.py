"""Topological Computation Engine — graph, operations, β₁, settlement.

Pure Python, no external dependencies. All data structures immutable-by-convention
(operations return new objects, originals unchanged).
"""

from __future__ import annotations

import dataclasses
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


# ---------------------------------------------------------------------------
# Enums
# ---------------------------------------------------------------------------

class VertexStatus(str, Enum):
    ACTIVE = "active"
    CONTESTED = "contested"
    FOLDED = "folded"


class EdgeType(str, Enum):
    DEPENDENCY = "dependency"
    NEGATION = "negation"
    SUBLATION = "sublation"
    REFERENCE = "reference"
    FOLD = "fold"


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class Vertex:
    id: str
    status: VertexStatus = VertexStatus.ACTIVE
    content: Optional[str] = None
    created_at: int = 0


@dataclass(frozen=True, slots=True)
class Edge:
    source: str
    target: str
    edge_type: EdgeType
    created_at: int = 0
    surface: Optional[str] = None   # 连接两概念的原始语言片段（动词/谓语部分）
    context: Optional[str] = None   # 完整原句，用于叙事生成时还原语境


# ---------------------------------------------------------------------------
# Graph
# ---------------------------------------------------------------------------

class Graph:
    """Directed typed graph with dual-view (K_full / K_active).

    Mutation methods return *new* Graph instances.
    """

    def __init__(
        self,
        vertices: dict[str, Vertex] | None = None,
        edges: list[Edge] | None = None,
    ) -> None:
        self._vertices: dict[str, Vertex] = dict(vertices) if vertices else {}
        self._edges: list[Edge] = list(edges) if edges else []
        # Adjacency index: O(1) lookup by source/target
        self._adj_out: dict[str, list[Edge]] = {}
        self._adj_in: dict[str, list[Edge]] = {}
        for e in self._edges:
            self._adj_out.setdefault(e.source, []).append(e)
            self._adj_in.setdefault(e.target, []).append(e)

    # -- accessors ----------------------------------------------------------

    @property
    def vertices(self) -> dict[str, Vertex]:
        return dict(self._vertices)

    @property
    def edges(self) -> list[Edge]:
        return list(self._edges)

    def vertex(self, vid: str) -> Vertex | None:
        return self._vertices.get(vid)

    def active_vertex_ids(self) -> list[str]:
        return [
            v.id for v in self._vertices.values()
            if v.status != VertexStatus.FOLDED
        ]

    def active_edges(self) -> list[Edge]:
        active = set(self.active_vertex_ids())
        return [e for e in self._edges if e.source in active and e.target in active]

    def neighbors(self, vid: str) -> list[str]:
        """Return ids of vertices adjacent to vid (outgoing + incoming) among active vertices."""
        active = set(self.active_vertex_ids())
        out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
        inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
        return sorted(out | inc)

    def out_neighbors(self, vid: str) -> list[str]:
        active = set(self.active_vertex_ids())
        return sorted({e.target for e in self._adj_out.get(vid, ()) if e.target in active})

    def in_neighbors(self, vid: str) -> list[str]:
        active = set(self.active_vertex_ids())
        return sorted({e.source for e in self._adj_in.get(vid, ()) if e.source in active})

    def has_path(self, source: str, target: str) -> bool:
        """BFS on active subgraph (directed edges only)."""
        active = set(self.active_vertex_ids())
        if source not in active or target not in active:
            return False
        visited: set[str] = set()
        queue = deque([source])
        while queue:
            cur = queue.popleft()
            if cur == target and cur != source:
                return True
            if cur in visited:
                continue
            visited.add(cur)
            for e in self._adj_out.get(cur, ()):
                if e.target in active and e.target not in visited:
                    queue.append(e.target)
        return False

    def local_subgraph(self, center: str, radius: int = 1) -> tuple[list[str], list[Edge]]:
        """Return vertices and edges within `radius` hops of `center`."""
        active = set(self.active_vertex_ids())
        verts: set[str] = {center}
        for _ in range(radius):
            new: set[str] = set()
            for v in verts:
                new.update(n for n in self.neighbors(v) if n in active)
            verts |= new
        edges = [
            e for e in self._edges
            if e.source in verts and e.target in verts
        ]
        return sorted(verts), edges

    # -- undirected projection for β₁ computation --------------------------

    def undirected_active_edges(self) -> list[frozenset[str]]:
        """Return undirected edge set from active directed edges (no self-loops)."""
        active = set(self.active_vertex_ids())
        result: set[frozenset[str]] = set()
        for e in self._edges:
            if e.source in active and e.target in active and e.source != e.target:
                result.add(frozenset((e.source, e.target)))
        return sorted(result, key=lambda fs: tuple(sorted(fs)))

    def self_loops(self) -> list[Edge]:
        """Return self-loops in active graph."""
        active = set(self.active_vertex_ids())
        return [e for e in self._edges if e.source == e.target and e.source in active]

    # -- mutation (returns new Graph) ---------------------------------------

    def add_vertex(self, v: Vertex) -> Graph:
        new_verts = dict(self._vertices)
        new_verts[v.id] = v
        return Graph(new_verts, self._edges)

    def add_edge(self, e: Edge) -> Graph:
        return Graph(self._vertices, self._edges + [e])

    def set_vertex_status(self, vid: str, status: VertexStatus) -> Graph:
        new_verts = dict(self._vertices)
        old = new_verts[vid]
        new_verts[vid] = Vertex(old.id, status, old.content, old.created_at)
        return Graph(new_verts, self._edges)

    def merge_vertices(self, keep: str, remove: str) -> Graph:
        """Merge `remove` into `keep`. Redirect all edges, mark `remove` as folded."""
        new_verts = dict(self._vertices)
        old = new_verts[remove]
        new_verts[remove] = Vertex(old.id, VertexStatus.FOLDED, old.content, old.created_at)

        new_edges: list[Edge] = []
        for e in self._edges:
            src = keep if e.source == remove else e.source
            tgt = keep if e.target == remove else e.target
            # Preserve surface/context fields when redirecting edges
            new_edges.append(Edge(src, tgt, e.edge_type, e.created_at, e.surface, e.context))

        return Graph(new_verts, new_edges)


# ---------------------------------------------------------------------------
# β₁ computation (Z₂ Gaussian elimination, pure Python)
# ---------------------------------------------------------------------------

def _connected_components(vertex_ids: list[str], edges: list[frozenset[str]]) -> int:
    """Count connected components using Union-Find."""
    parent: dict[str, str] = {v: v for v in vertex_ids}

    def find(x: str) -> str:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a: str, b: str) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    for e in edges:
        pair = sorted(e)
        if len(pair) >= 2:
            union(pair[0], pair[1])

    return len({find(v) for v in vertex_ids})


def compute_beta_1(graph: Graph) -> int:
    """Compute β₁ of the active subgraph.

    β₁ = |E_undirected| - |V_active| + connected_components + self_loops

    Self-loops each contribute +1 to β₁ (∂(loop) = 0, each is an independent 1-cycle).
    """
    active_vids = graph.active_vertex_ids()
    if not active_vids:
        return 0
    undirected = graph.undirected_active_edges()
    n_loops = len(graph.self_loops())
    n_v = len(active_vids)
    n_e = len(undirected)
    c = _connected_components(active_vids, undirected)
    return n_e - n_v + c + n_loops


# ---------------------------------------------------------------------------
# Settlement tracking
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class SettledCycle:
    """A specific set of edges forming an irreducible cycle, marked as settled.

    residue: transformation products of settlement (396号).
    Each entry: {"type": "edge"|"tension"|"pressure", "data": {...}}
    Settlement is transformation, not closure — every Aufhebung is also
    the starting point of a new contradiction.

    status: "active" (protecting lockzone) or "sublated" (superseded by
    topological motion — fold destroyed the cycle's physical basis).
    415号: ghost settlement is a signal, not a bug.
    """
    edges: frozenset[tuple[str, str]]
    settled_at_step: int
    residue: tuple[dict, ...] = ()
    status: str = "active"
    sublated_at_step: int | None = None
    sublated_by: str | None = None


def find_new_cycle_edges(
    graph_before: Graph,
    graph_after: Graph,
) -> frozenset[tuple[str, str]] | None:
    """Find the edges of a newly created cycle (if β₁ increased).

    Simple approach: find an edge in graph_after but not graph_before
    that participates in a cycle. Return the cycle's edge set.
    """
    before_edges = {
        (e.source, e.target)
        for e in graph_before.active_edges()
    }
    after_edges = {
        (e.source, e.target)
        for e in graph_after.active_edges()
    }
    new_edges = after_edges - before_edges
    if not new_edges:
        return None

    # For each new edge (u, v), check if v→u path exists in graph_after
    # If so, that path + this edge = cycle
    for u, v in new_edges:
        # Find path from v back to u in the active directed graph
        path = _find_path_edges(graph_after, v, u)
        if path is not None:
            cycle = frozenset(path | {(u, v)})
            return cycle

    # Check self-loops
    for u, v in new_edges:
        if u == v:
            return frozenset({(u, v)})

    return None


def _find_path_edges(graph: Graph, source: str, target: str) -> set[tuple[str, str]] | None:
    """BFS to find directed path from source to target, return edge set."""
    active = set(graph.active_vertex_ids())
    if source not in active or target not in active:
        return None

    visited: set[str] = set()
    parent: dict[str, tuple[str, str]] = {}  # child -> (parent, child) edge
    queue = deque([source])

    while queue:
        cur = queue.popleft()
        if cur == target:
            # Reconstruct path edges
            edges: set[tuple[str, str]] = set()
            node = target
            while node in parent:
                edge = parent[node]
                edges.add(edge)
                node = edge[0]
            return edges
        if cur in visited:
            continue
        visited.add(cur)
        for e in graph._adj_out.get(cur, ()):
            if e.target in active and e.target not in visited:
                parent[e.target] = (e.source, e.target)
                queue.append(e.target)

    return None


class SettlementTracker:
    """Track cycle persistence and settlement state.

    396号: settlement is transformation, not closure. Each settlement
    produces residue — unsettled elements that are the starting point
    of new contradictions.
    """

    def __init__(self, threshold: int = 5) -> None:
        self.threshold = threshold
        self._pending: dict[frozenset[tuple[str, str]], int] = {}  # edges -> first_seen_step
        self._settled: list[SettledCycle] = []
        self._blocked_log: list[dict] = []

    @property
    def settled_cycles(self) -> list[SettledCycle]:
        return list(self._settled)

    @property
    def blocked_log(self) -> list[dict]:
        return list(self._blocked_log)

    def register_new_cycle(self, edges: frozenset[tuple[str, str]], step: int) -> None:
        """Register a newly detected cycle for persistence tracking."""
        if edges not in self._pending and not self._is_settled(edges):
            self._pending[edges] = step

    def _compute_residue(
        self, cycle_edges: frozenset[tuple[str, str]], step: int,
        graph: Graph | None = None,
    ) -> tuple[dict, ...]:
        """Compute residue for a settling cycle (396号).

        Five residue types:
        1. edge: two settled cycles share a vertex -> inter-cycle relation
        2. tension: two settled cycles have conflicting edge directions on shared vertex
        3. expression_pressure: every settlement needs articulation
        4. boundary_edge: edge with one end inside settled cycle, other end outside
        5. nachtraeglichkeit: new settlement overlaps prior settlement's jurisdiction
        """
        residue: list[dict] = []

        # Vertices in this cycle
        cycle_vids: set[str] = set()
        for src, tgt in cycle_edges:
            cycle_vids.add(src)
            cycle_vids.add(tgt)

        # Check shared vertices with other settled cycles
        for other_sc in self._settled:
            other_vids: set[str] = set()
            for src, tgt in other_sc.edges:
                other_vids.add(src)
                other_vids.add(tgt)
            shared = cycle_vids & other_vids
            if shared:
                # Type 1: shared vertex -> new edge (inter-cycle relation)
                residue.append({
                    "type": "edge",
                    "data": {
                        "shared_vertices": sorted(shared),
                        "other_cycle_settled_at": other_sc.settled_at_step,
                        "relation": "inter_cycle",
                    },
                })

                # Type 2: check for directional conflict on shared vertices
                # If cycle A has edge X->Y and cycle B has Y->X, that's tension
                cycle_dir = {(s, t) for s, t in cycle_edges if s in shared or t in shared}
                other_dir = {(s, t) for s, t in other_sc.edges if s in shared or t in shared}
                for s1, t1 in cycle_dir:
                    if (t1, s1) in other_dir:
                        residue.append({
                            "type": "tension",
                            "data": {
                                "vertex_a": s1,
                                "vertex_b": t1,
                                "conflict": "directional",
                                "this_direction": f"{s1}->{t1}",
                                "other_direction": f"{t1}->{s1}",
                            },
                        })

                # Type 5: nachtraeglichkeit — new cycle overlaps prior settlement
                residue.append({
                    "type": "nachtraeglichkeit",
                    "data": {
                        "prior_settled_at": other_sc.settled_at_step,
                        "affected_vertices": sorted(shared),
                        "reason": "new settlement altered edge structure within prior's jurisdiction",
                    },
                })

        # Type 4: boundary_edge — edges with one end inside, one end outside
        if graph is not None:
            for vid in cycle_vids:
                for e in graph._adj_out.get(vid, ()):
                    if (e.source, e.target) not in cycle_edges and e.target not in cycle_vids:
                        residue.append({
                            "type": "boundary_edge",
                            "data": {
                                "internal_vertex": vid,
                                "external_vertex": e.target,
                                "edge_source": e.source,
                                "edge_target": e.target,
                            },
                        })
                for e in graph._adj_in.get(vid, ()):
                    if (e.source, e.target) not in cycle_edges and e.source not in cycle_vids:
                        residue.append({
                            "type": "boundary_edge",
                            "data": {
                                "internal_vertex": vid,
                                "external_vertex": e.source,
                                "edge_source": e.source,
                                "edge_target": e.target,
                            },
                        })

        # Type 3: expression_pressure — always produced
        residue.append({
            "type": "expression_pressure",
            "data": {
                "cycle_vertices": sorted(cycle_vids),
                "settled_at_step": step,
                "needs_articulation": True,
            },
        })

        return tuple(residue)

    def check_settlement(self, step: int, graph: Graph) -> list[SettledCycle]:
        """Check if any pending cycles have persisted long enough to settle.

        396号: settlement produces residue (transformation, not closure).
        """
        newly_settled: list[SettledCycle] = []
        active_edges = {(e.source, e.target) for e in graph.active_edges()}

        to_remove: list[frozenset[tuple[str, str]]] = []
        for cycle_edges, first_seen in self._pending.items():
            # Check cycle still exists in active graph
            if not cycle_edges.issubset(active_edges):
                to_remove.append(cycle_edges)
                continue
            if step - first_seen >= self.threshold:
                residue = self._compute_residue(cycle_edges, step, graph=graph)
                sc = SettledCycle(cycle_edges, step, residue=residue)
                self._settled.append(sc)
                newly_settled.append(sc)
                to_remove.append(cycle_edges)

        for ce in to_remove:
            self._pending.pop(ce, None)

        return newly_settled

    def backfill_residue(self, graph: Graph | None = None) -> int:
        """Backfill residue for existing settled cycles that have none (396号).

        Called once at daemon startup to retroactively produce residue
        for the 201 settled cycles that were created before this mechanism.

        Args:
            graph: the active graph, needed for boundary_edge computation.
                   If None, boundary_edge residues will not be produced.

        Returns count of cycles that received residue.
        """
        backfilled = 0
        new_settled: list[SettledCycle] = []

        for sc in self._settled:
            if sc.residue:
                new_settled.append(sc)
                continue
            # Compute residue against all OTHER settled cycles
            # (temporarily exclude self to avoid self-reference)
            cycle_vids: set[str] = set()
            for src, tgt in sc.edges:
                cycle_vids.add(src)
                cycle_vids.add(tgt)

            residue: list[dict] = []

            for other_sc in self._settled:
                if other_sc is sc:
                    continue
                other_vids: set[str] = set()
                for src, tgt in other_sc.edges:
                    other_vids.add(src)
                    other_vids.add(tgt)
                shared = cycle_vids & other_vids
                if shared:
                    residue.append({
                        "type": "edge",
                        "data": {
                            "shared_vertices": sorted(shared),
                            "other_cycle_settled_at": other_sc.settled_at_step,
                            "relation": "inter_cycle",
                        },
                    })
                    cycle_dir = {(s, t) for s, t in sc.edges if s in shared or t in shared}
                    other_dir = {(s, t) for s, t in other_sc.edges if s in shared or t in shared}
                    for s1, t1 in cycle_dir:
                        if (t1, s1) in other_dir:
                            residue.append({
                                "type": "tension",
                                "data": {
                                    "vertex_a": s1,
                                    "vertex_b": t1,
                                    "conflict": "directional",
                                    "this_direction": f"{s1}->{t1}",
                                    "other_direction": f"{t1}->{s1}",
                                },
                            })

                    # Nachträglichkeit: this cycle overlaps prior settlement
                    residue.append({
                        "type": "nachtraeglichkeit",
                        "data": {
                            "prior_settled_at": other_sc.settled_at_step,
                            "affected_vertices": sorted(shared),
                            "reason": "new settlement altered edge structure within prior's jurisdiction",
                        },
                    })

            # Boundary edges: edges with one end inside cycle, one end outside
            if graph is not None:
                for vid in cycle_vids:
                    for e in graph._adj_out.get(vid, ()):
                        if (e.source, e.target) not in sc.edges and e.target not in cycle_vids:
                            residue.append({
                                "type": "boundary_edge",
                                "data": {
                                    "internal_vertex": vid,
                                    "external_vertex": e.target,
                                    "edge_source": e.source,
                                    "edge_target": e.target,
                                },
                            })
                    for e in graph._adj_in.get(vid, ()):
                        if (e.source, e.target) not in sc.edges and e.source not in cycle_vids:
                            residue.append({
                                "type": "boundary_edge",
                                "data": {
                                    "internal_vertex": vid,
                                    "external_vertex": e.source,
                                    "edge_source": e.source,
                                    "edge_target": e.target,
                                },
                            })

            residue.append({
                "type": "expression_pressure",
                "data": {
                    "cycle_vertices": sorted(cycle_vids),
                    "settled_at_step": sc.settled_at_step,
                    "needs_articulation": True,
                },
            })

            new_sc = SettledCycle(sc.edges, sc.settled_at_step, residue=tuple(residue))
            new_settled.append(new_sc)
            backfilled += 1

        self._settled = new_settled
        return backfilled

    def _cycle_residue_vertices(self, sc: SettledCycle) -> set[str]:
        """Return vertex IDs referenced by a single cycle's residue items (410号推論4方案C).

        Same extraction logic as residue_vertices() but scoped to one cycle,
        enabling per-cycle residue exception in would_destroy_settled().
        """
        vids: set[str] = set()
        for item in sc.residue:
            data = item.get("data", {})
            if item["type"] == "edge":
                for v in data.get("shared_vertices", []):
                    vids.add(v)
            elif item["type"] == "tension":
                va = data.get("vertex_a")
                vb = data.get("vertex_b")
                if va:
                    vids.add(va)
                if vb:
                    vids.add(vb)
            elif item["type"] == "expression_pressure":
                for v in data.get("cycle_vertices", []):
                    vids.add(v)
            elif item["type"] == "boundary_edge":
                iv = data.get("internal_vertex")
                ev = data.get("external_vertex")
                if iv:
                    vids.add(iv)
                if ev:
                    vids.add(ev)
            elif item["type"] == "nachtraeglichkeit":
                for v in data.get("affected_vertices", []):
                    vids.add(v)
        return vids

    def residue_vertices(self) -> set[str]:
        """Return all vertex IDs referenced by any residue item (396号).

        These are vertices where operations should NOT be blocked even though
        they participate in settled cycles — the residue is the unsettled
        transformation product.
        """
        vids: set[str] = set()
        for sc in self._settled:
            for item in sc.residue:
                data = item.get("data", {})
                if item["type"] == "edge":
                    for v in data.get("shared_vertices", []):
                        vids.add(v)
                elif item["type"] == "tension":
                    va = data.get("vertex_a")
                    vb = data.get("vertex_b")
                    if va:
                        vids.add(va)
                    if vb:
                        vids.add(vb)
                elif item["type"] == "expression_pressure":
                    for v in data.get("cycle_vertices", []):
                        vids.add(v)
                elif item["type"] == "boundary_edge":
                    iv = data.get("internal_vertex")
                    ev = data.get("external_vertex")
                    if iv:
                        vids.add(iv)
                    if ev:
                        vids.add(ev)
                elif item["type"] == "nachtraeglichkeit":
                    for v in data.get("affected_vertices", []):
                        vids.add(v)
        return vids

    def mark_sublated_cycles(self, graph: Graph, step: int, operation: str = "fold") -> int:
        """Mark settled cycles whose edges no longer exist as SUBLATED.

        415号: ghost settlement is a signal, not a bug. Fold destroying a settled
        cycle means the consensus has been superseded by topological motion.
        The cycle is marked SUBLATED (not deleted) — acknowledging it completed
        its historical mission and releasing its lockzone.

        Returns the number of newly sublated cycles.
        """
        active_edges = {(e.source, e.target) for e in graph.active_edges()}
        updated: list[SettledCycle] = []
        sublated_count = 0
        for sc in self._settled:
            if sc.status == "sublated":
                updated.append(sc)  # already sublated, keep as-is
            elif sc.edges.issubset(active_edges):
                updated.append(sc)  # still valid, keep active
            else:
                updated.append(dataclasses.replace(
                    sc,
                    status="sublated",
                    sublated_at_step=step,
                    sublated_by=operation,
                ))
                sublated_count += 1
        self._settled = updated
        return sublated_count

    def purge_invalid_cycles(self, graph: Graph, step: int = 0, operation: str = "fold") -> int:
        """Backward-compatible alias for mark_sublated_cycles.

        Retains the old name so daemon.py call sites continue to work.
        Semantics changed: cycles are marked SUBLATED, not deleted (415号).
        """
        return self.mark_sublated_cycles(graph, step=step, operation=operation)

    def would_destroy_settled(
        self,
        graph_after: Graph,
        operation_vertices: frozenset[str] | None = None,
        graph_before: Graph | None = None,
        merge_map: dict[str, str] | None = None,
    ) -> SettledCycle | None:
        """Check if graph_after destroys any settled cycle.

        396号: if the operation targets only residue vertices (vertices
        referenced by residue items), allow it — settlement protects the
        cycle's edges, not its residue.

        410号修复: if graph_before is provided, only report a cycle as
        destroyed if it was intact before the operation but broken after.
        Ghost settlements (already broken before the operation) are purged
        rather than blocking the operation.

        410号推论4方案B: if merge_map is provided (fold operation), check
        whether the settled cycle survives the merge via isomorphism —
        map removed vertices to their keep targets and verify the mapped
        edge set is still present in graph_after.

        Args:
            graph_after: the graph state after the proposed operation
            operation_vertices: vertices directly involved in the operation
                (e.g., fold targets, negate endpoints). If all of these are
                residue vertices, the operation is allowed even if it would
                modify a settled cycle's edge set.
            graph_before: the graph state before the operation. If provided,
                cycles already broken in graph_before are purged (not blocked).
            merge_map: vertex merge mapping {removed: keep} from fold.
                If provided, cycles whose mapped edges survive in graph_after
                are considered intact (isomorphic), not destroyed.
        """
        active_edges_after = {(e.source, e.target) for e in graph_after.active_edges()}
        active_edges_before = (
            {(e.source, e.target) for e in graph_before.active_edges()}
            if graph_before is not None else None
        )
        # 410号: purge ghost settlements (already broken before operation)
        # 415号: sublated cycles are already non-blocking, skip them in ghost check
        if active_edges_before is not None:
            ghosts = [
                sc for sc in self._settled
                if sc.status == "active" and not sc.edges.issubset(active_edges_before)
            ]
            if ghosts:
                self._settled = [
                    sc for sc in self._settled
                    if sc.status == "sublated" or sc.edges.issubset(active_edges_before)
                ]

        for sc in self._settled:
            if sc.status == "sublated":
                continue  # 415号: sublated cycles don't block operations
            if not sc.edges.issubset(active_edges_after):
                # This settled cycle would be destroyed.
                # 410号推论4方案B: fold isomorphism — cycle survives if
                # mapped edges (after vertex merge) are still in the graph
                if merge_map:
                    mapped_edges = frozenset(
                        (merge_map.get(s, s), merge_map.get(t, t))
                        for s, t in sc.edges
                    )
                    # Drop self-loops created by fold (A→A after merge)
                    mapped_edges = frozenset(
                        (s, t) for s, t in mapped_edges if s != t
                    )
                    if mapped_edges and mapped_edges.issubset(active_edges_after):
                        continue  # cycle survived the fold (isomorphic)
                # 410号推论4方案C: per-cycle residue exception
                # Check against the current cycle's residue vertices, not global
                if operation_vertices is not None and sc.residue:
                    cycle_residue_vids = self._cycle_residue_vertices(sc)
                    if operation_vertices.issubset(cycle_residue_vids):
                        continue
                return sc
        return None

    def record_blocked(self, step: int, operation: str, detail: dict, cycle: SettledCycle) -> None:
        self._blocked_log.append({
            "step": step,
            "operation": operation,
            "detail": detail,
            "blocked_by": sorted(cycle.edges),
        })

    def _is_settled(self, edges: frozenset[tuple[str, str]]) -> bool:
        return any(sc.edges == edges and sc.status == "active" for sc in self._settled)


# ---------------------------------------------------------------------------
# Operations
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class SublationRecord:
    """Audit trail for a sublation: what was negated, preserved, elevated."""
    source_a_id: str
    source_b_id: str
    contradiction: str          # what the negation edge asserts
    negated: str                # what was denied (the incompatibility)
    preserved: str              # what was kept from both sides
    elevated: str               # the synthesis — what emerges


@dataclass
class OperationResult:
    """Result of a topological operation."""
    graph: Graph
    delta_beta_1_predicted: int
    delta_beta_1_actual: int
    new_vertex: str | None = None
    new_cycle_edges: frozenset[tuple[str, str]] | None = None
    blocked: bool = False
    blocked_by: SettledCycle | None = None
    sublation_record: SublationRecord | None = None


def fold(
    graph: Graph,
    vertices: list[str],
    step: int,
    settlement: SettlementTracker,
) -> OperationResult:
    """Fold: identify vertices as equivalent.

    Δβ₁ = (c - 1) + n_loop
    """
    if len(vertices) < 2:
        raise ValueError("Fold requires at least 2 vertices")

    active = set(graph.active_vertex_ids())
    s = set(vertices)
    if not s.issubset(active):
        raise ValueError("All fold vertices must be active")

    # Compute prediction
    # n_loop: edges within S
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )

    # lower link: vertices adjacent to S but not in S
    lower_link_vids: set[str] = set()
    for v in s:
        for n in graph.neighbors(v):
            if n not in s and n in active:
                lower_link_vids.add(n)

    # connected components of lower link
    if lower_link_vids:
        ll_edges: list[frozenset[str]] = []
        for e in graph.active_edges():
            if e.source in lower_link_vids and e.target in lower_link_vids and e.source != e.target:
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link_vids), ll_edges)
    else:
        c = 0

    predicted = (c - 1 if c > 0 else 0) + n_loop

    beta_before = compute_beta_1(graph)

    # Execute fold
    keep = vertices[0]
    result_graph = graph
    for v in vertices[1:]:
        result_graph = result_graph.merge_vertices(keep, v)
        result_graph = result_graph.add_edge(
            Edge(keep, keep, EdgeType.FOLD, step)
        ) if n_loop > 0 else result_graph

    # Check settlement constraint (396号: pass operation vertices for residue check)
    # 410号推论4方案B: build merge map for fold isomorphism detection
    merge_map = {v: keep for v in vertices[1:]}
    violated = settlement.would_destroy_settled(
        result_graph, operation_vertices=frozenset(vertices),
        graph_before=graph,
        merge_map=merge_map,
    )
    if violated is not None:
        return OperationResult(
            graph=graph,
            delta_beta_1_predicted=predicted,
            delta_beta_1_actual=0,
            blocked=True,
            blocked_by=violated,
        )

    # 415号: 将因 fold 操作失效的 settled cycle 标记为 SUBLATED（而非删除）
    # fold 的 merge 可能让未被直接检查的 cycle 的边失效
    settlement.mark_sublated_cycles(result_graph, step=step, operation="fold")

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    new_cycle = find_new_cycle_edges(graph, result_graph) if actual > 0 else None

    return OperationResult(
        graph=result_graph,
        delta_beta_1_predicted=predicted,
        delta_beta_1_actual=actual,
        new_cycle_edges=new_cycle,
    )


def negate(
    graph: Graph,
    thesis: str,
    antithesis: str | None,
    step: int,
    settlement: SettlementTracker,
) -> OperationResult:
    """Negate: inscribe contradiction.

    Case A (antithesis exists in graph): add negation edge, Δβ₁ = +1 if path exists.
    Case B (antithesis is None -> create new): Δβ₁ = 0, position moves to new vertex.
    """
    active = set(graph.active_vertex_ids())
    beta_before = compute_beta_1(graph)

    if antithesis is not None and antithesis in active:
        # Case A: w already exists
        has_connecting_path = graph.has_path(thesis, antithesis)
        predicted = 1 if has_connecting_path else 0

        result_graph = graph.add_edge(
            Edge(antithesis, thesis, EdgeType.NEGATION, step)
        )
        result_graph = result_graph.set_vertex_status(thesis, VertexStatus.CONTESTED)
    else:
        # Case B: create new antithesis
        if antithesis is None:
            antithesis = f"anti_{thesis}_{step}"
        predicted = 0

        new_v = Vertex(antithesis, VertexStatus.ACTIVE, created_at=step)
        result_graph = graph.add_vertex(new_v)
        result_graph = result_graph.add_edge(
            Edge(antithesis, thesis, EdgeType.NEGATION, step)
        )
        result_graph = result_graph.set_vertex_status(thesis, VertexStatus.CONTESTED)

    # 415号: negate 免检(方案A)不再需要——sublated cycles 自然被 would_destroy_settled 跳过。
    # negate 只加边不删边 → active cycles 不会被破坏。
    # sublated cycles 不阻塞 → 无需特殊处理。
    # 恢复 settlement 检查以保持操作一致性。
    violated = settlement.would_destroy_settled(
        result_graph, operation_vertices=frozenset({thesis, antithesis or ""}),
        graph_before=graph,
    )
    if violated is not None:
        return OperationResult(
            graph=graph,
            delta_beta_1_predicted=predicted,
            delta_beta_1_actual=0,
            blocked=True,
            blocked_by=violated,
        )

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    new_cycle = find_new_cycle_edges(graph, result_graph) if actual > 0 else None
    new_vertex = antithesis if antithesis not in active else None

    return OperationResult(
        graph=result_graph,
        delta_beta_1_predicted=predicted,
        delta_beta_1_actual=actual,
        new_vertex=new_vertex,
        new_cycle_edges=new_cycle,
    )


def sublate(
    graph: Graph,
    thesis: str,
    antithesis: str,
    step: int,
    settlement: SettlementTracker,
    synthesis_content: str | None = None,
    sublation_record: SublationRecord | None = None,
) -> OperationResult:
    """Sublate: create synthesis on top of contradiction.

    Precondition: negation edge between thesis and antithesis exists.
    Creates C with sublation edges C→thesis, C→antithesis.
    Δβ₁ = +1 (triangle C-thesis-antithesis).

    Gate conditions (all must hold for new sublations):
      1. Necessary: negation edge exists between thesis and antithesis.
      2. Generative: synthesis_content must be non-empty (not template filler).
      3. Audit: sublation_record must be provided with complete provenance.
    """
    # Gate 1 — Necessary: negation edge exists
    has_negation = any(
        e.edge_type == EdgeType.NEGATION
        and (
            (e.source == antithesis and e.target == thesis)
            or (e.source == thesis and e.target == antithesis)
        )
        for e in graph.active_edges()
    )
    if not has_negation:
        # Negation edge may have been removed between detection and execution (race condition).
        # Return a blocked result instead of crashing.
        return OperationResult(
            graph=graph,
            new_vertex="",
            delta_beta_1_predicted=0,
            delta_beta_1_actual=0,
            blocked=True,
            blocked_by=None,
        )

    # Gate 2 — Generative: synthesis content must be articulable
    if not synthesis_content or not synthesis_content.strip():
        raise ValueError(
            f"Sublation gate: synthesis_content must be non-empty. "
            f"Cannot create synthesis from {thesis} and {antithesis} without articulable content."
        )

    # Gate 3 — Audit: sublation record must be complete
    if sublation_record is None:
        raise ValueError(
            f"Sublation gate: sublation_record required for audit trail. "
            f"Provide source IDs, contradiction, negated/preserved/elevated."
        )
    if not sublation_record.contradiction.strip():
        raise ValueError("Sublation gate: sublation_record.contradiction must be non-empty.")
    if not sublation_record.elevated.strip():
        raise ValueError("Sublation gate: sublation_record.elevated must be non-empty.")

    beta_before = compute_beta_1(graph)
    predicted = 1

    synthesis_id = f"syn_{thesis}_{antithesis}_{step}"
    new_v = Vertex(synthesis_id, VertexStatus.ACTIVE, content=synthesis_content, created_at=step)

    result_graph = graph.add_vertex(new_v)
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, thesis, EdgeType.SUBLATION, step)
    )
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, antithesis, EdgeType.SUBLATION, step)
    )

    # Check settlement constraint (396号: pass operation vertices for residue check)
    violated = settlement.would_destroy_settled(
        result_graph, operation_vertices=frozenset({thesis, antithesis}),
        graph_before=graph,
    )
    if violated is not None:
        return OperationResult(
            graph=graph,
            delta_beta_1_predicted=predicted,
            delta_beta_1_actual=0,
            blocked=True,
            blocked_by=violated,
        )

    # 415号: 将因 sublate 操作失效的 settled cycle 标记为 SUBLATED
    settlement.mark_sublated_cycles(result_graph, step=step, operation="sublate")

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    new_cycle = find_new_cycle_edges(graph, result_graph) if actual > 0 else None

    return OperationResult(
        graph=result_graph,
        delta_beta_1_predicted=predicted,
        delta_beta_1_actual=actual,
        new_vertex=synthesis_id,
        new_cycle_edges=new_cycle,
        sublation_record=sublation_record,
    )
