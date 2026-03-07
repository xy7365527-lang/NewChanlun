"""Topological Computation Engine — graph, operations, β₁, settlement.

Pure Python, no external dependencies. All data structures immutable-by-convention
(operations return new objects, originals unchanged).
"""

from __future__ import annotations

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
        # Lazy caches — valid because Graph is immutable (mutations return new instances)
        self._active_ids_cache: list[str] | None = None
        self._active_set_cache: set[str] | None = None

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
        if self._active_ids_cache is None:
            self._active_ids_cache = [
                v.id for v in self._vertices.values()
                if v.status != VertexStatus.FOLDED
            ]
        return self._active_ids_cache

    def _active_set(self) -> set[str]:
        """Cached set of active vertex ids for O(1) membership tests."""
        if self._active_set_cache is None:
            self._active_set_cache = set(self.active_vertex_ids())
        return self._active_set_cache

    def active_edges(self) -> list[Edge]:
        active = self._active_set()
        return [e for e in self._edges if e.source in active and e.target in active]

    def neighbors(self, vid: str) -> list[str]:
        """Return ids of vertices adjacent to vid (outgoing + incoming) among active vertices."""
        active = self._active_set()
        out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
        inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
        return sorted(out | inc)

    def out_neighbors(self, vid: str) -> list[str]:
        active = self._active_set()
        return sorted({e.target for e in self._adj_out.get(vid, ()) if e.target in active})

    def in_neighbors(self, vid: str) -> list[str]:
        active = self._active_set()
        return sorted({e.source for e in self._adj_in.get(vid, ()) if e.source in active})

    def has_path(self, source: str, target: str) -> bool:
        """BFS on active subgraph (directed edges only)."""
        active = self._active_set()
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
        active = self._active_set()
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
        active = self._active_set()
        result: set[frozenset[str]] = set()
        for e in self._edges:
            if e.source in active and e.target in active and e.source != e.target:
                result.add(frozenset((e.source, e.target)))
        return sorted(result, key=lambda fs: tuple(sorted(fs)))

    def self_loops(self) -> list[Edge]:
        """Return self-loops in active graph."""
        active = self._active_set()
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
    """A specific set of edges forming an irreducible cycle, marked as settled."""
    edges: frozenset[tuple[str, str]]
    settled_at_step: int


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
    active = graph._active_set()
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
    """Track cycle persistence and settlement state."""

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

    def check_settlement(self, step: int, graph: Graph) -> list[SettledCycle]:
        """Check if any pending cycles have persisted long enough to settle."""
        newly_settled: list[SettledCycle] = []
        active_edges = {(e.source, e.target) for e in graph.active_edges()}

        to_remove: list[frozenset[tuple[str, str]]] = []
        for cycle_edges, first_seen in self._pending.items():
            # Check cycle still exists in active graph
            if not cycle_edges.issubset(active_edges):
                to_remove.append(cycle_edges)
                continue
            if step - first_seen >= self.threshold:
                sc = SettledCycle(cycle_edges, step)
                self._settled.append(sc)
                newly_settled.append(sc)
                to_remove.append(cycle_edges)

        for ce in to_remove:
            self._pending.pop(ce, None)

        return newly_settled

    def would_destroy_settled(self, graph_after: Graph) -> SettledCycle | None:
        """Check if graph_after destroys any settled cycle. Returns the first violated cycle."""
        active_edges = {(e.source, e.target) for e in graph_after.active_edges()}
        for sc in self._settled:
            if not sc.edges.issubset(active_edges):
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
        return any(sc.edges == edges for sc in self._settled)


# ---------------------------------------------------------------------------
# Operations
# ---------------------------------------------------------------------------

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

    # Check settlement constraint
    violated = settlement.would_destroy_settled(result_graph)
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

    # Check settlement constraint
    violated = settlement.would_destroy_settled(result_graph)
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
) -> OperationResult:
    """Sublate: create synthesis on top of contradiction.

    Precondition: negation edge between thesis and antithesis exists.
    Creates C with sublation edges C→thesis, C→antithesis.
    Δβ₁ = +1 (triangle C-thesis-antithesis).
    """
    # Verify precondition: negation edge exists
    has_negation = any(
        e.edge_type == EdgeType.NEGATION
        and (
            (e.source == antithesis and e.target == thesis)
            or (e.source == thesis and e.target == antithesis)
        )
        for e in graph.active_edges()
    )
    if not has_negation:
        raise ValueError(f"No negation edge between {thesis} and {antithesis}")

    beta_before = compute_beta_1(graph)
    predicted = 1

    synthesis_id = f"syn_{thesis}_{antithesis}_{step}"
    new_v = Vertex(synthesis_id, VertexStatus.ACTIVE, created_at=step)

    result_graph = graph.add_vertex(new_v)
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, thesis, EdgeType.SUBLATION, step)
    )
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, antithesis, EdgeType.SUBLATION, step)
    )

    # Check settlement constraint
    violated = settlement.would_destroy_settled(result_graph)
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

    return OperationResult(
        graph=result_graph,
        delta_beta_1_predicted=predicted,
        delta_beta_1_actual=actual,
        new_vertex=synthesis_id,
        new_cycle_edges=new_cycle,
    )
