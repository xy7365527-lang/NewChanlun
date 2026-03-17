"""Topological Computation Engine — graph, operations, β₁, settlement.

Pure Python, no external dependencies. Graph is mutable — mutation methods
modify in-place and return self (v254: O(1) add_edge instead of O(E) copy).

When the optional graph_rs Rust extension is available (compiled via maturin),
``create_graph()`` returns a ``RustGraphAdapter`` that delegates to the Rust
implementation while exposing the same interface (including internal attributes
like ``_active_ids``, ``_adj_out``, ``_adj_in``) so that downstream code in
daemon.py, traversal.py, and the free functions in this module work unchanged.
When graph_rs is not installed, everything falls back to the pure-Python Graph.
"""

from __future__ import annotations

import dataclasses
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

# ---------------------------------------------------------------------------
# Optional Rust backend
# ---------------------------------------------------------------------------

try:
    from graph_rs import Graph as RustGraph, Vertex as RustVertex, Edge as RustEdge
    from graph_rs import VertexStatus as RustVertexStatus, EdgeType as RustEdgeType
    # 临时禁用 Rust Graph——Rust clone() 比 Python 浅拷贝更重（全图深克隆）
    # 等 Rust 侧实装增量 add_edge（COW/persistent DS）后再启用
    _RUST_GRAPH_AVAILABLE = False  # TODO: 改回 True when Rust add_edge is incremental
except ImportError:
    _RUST_GRAPH_AVAILABLE = False


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
    COOCCURRENCE = "cooccurrence"  # 物质层(Dass): S_net共现边按需拉入，不参与fold/negate/sublate
    TRAVERSAL_ASSOCIATION = "traversal_association"  # 物质层(Dass): 穿越痕迹，immutable，不参与fold/negate/sublate
    ARTICULATED = "articulated"  # 概念层: 物质层积累涌现的概念连接（ça parle），参与fold/negate/sublate


# Material edge types excluded from beta_1 and terrain computations
_MATERIAL_EDGE_TYPES = frozenset((EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION))


# 概念层边类型集合——fold/negate/sublate/settlement 只操作这些边
CONCEPT_EDGE_TYPES = frozenset({
    EdgeType.DEPENDENCY,
    EdgeType.NEGATION,
    EdgeType.SUBLATION,
    EdgeType.REFERENCE,
    EdgeType.FOLD,
    EdgeType.ARTICULATED,
})


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class Vertex:
    id: str
    status: VertexStatus = VertexStatus.ACTIVE
    content: Optional[str] = None
    created_at: int = 0
    non_critical: bool = False  # 473号: Morse 命名门槛——β₁变化量D=0的顶点


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

    Mutation methods modify in-place and return self (v254: mutable Graph).
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
        # Cached active vertex ids — O(1) lookup instead of O(|V|) filter
        self._active_ids: set[str] = set(
            v.id for v in self._vertices.values()
            if v.status != VertexStatus.FOLDED
        )
        # Cached edge key set — O(1) membership test instead of O(E) scan
        self._edge_keys: set[tuple[str, str, EdgeType]] = set(
            (e.source, e.target, e.edge_type) for e in self._edges
        )

    def __getattr__(self, name: str):
        """Lazy init for attributes missing in old pickles."""
        if name == '_edge_keys':
            # Graph restored from pickle before _edge_keys was added
            keys = set(
                (e.source, e.target, e.edge_type) for e in self._edges
            )
            object.__setattr__(self, '_edge_keys', keys)
            return keys
        raise AttributeError(f"'Graph' object has no attribute '{name}'")

    # -- accessors ----------------------------------------------------------

    @property
    def vertices(self) -> dict[str, Vertex]:
        return self._vertices

    @property
    def edges(self) -> list[Edge]:
        return self._edges

    def vertex(self, vid: str) -> Vertex | None:
        return self._vertices.get(vid)

    def active_vertex_ids(self) -> list[str]:
        return list(self._active_ids)

    def has_edge_key(self, source: str, target: str, edge_type: EdgeType) -> bool:
        """O(1) check whether an edge with (source, target, edge_type) exists."""
        return (source, target, edge_type) in self._edge_keys

    @property
    def edge_keys(self) -> set[tuple[str, str, EdgeType]]:
        """Return edge key set."""
        return self._edge_keys

    def active_edges(self) -> list[Edge]:
        """Return edges between active vertices, excluding material layer (COOCCURRENCE, TRAVERSAL_ASSOCIATION).

        Material layer edges are visible to traversal (via neighbors/all_active_edges)
        but invisible to fold/negate/sublate/beta_1/settlement — they are material (Dass),
        not conceptual (Was).

        Cached; invalidated by mutation methods.
        """
        try:
            return self._cached_active_edges
        except AttributeError:
            active = self._active_ids
            result = [
                e for e in self._edges
                if e.source in active and e.target in active
                and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)
            ]
            object.__setattr__(self, '_cached_active_edges', result)
            return result

    def all_active_edges(self) -> list[Edge]:
        """Return ALL edges between active vertices, including material layer.

        Used by traversal for neighbor candidate selection — both concept layer
        and material layer edges are visible during walk.

        Cached; invalidated by mutation methods.
        """
        try:
            return self._cached_all_active_edges
        except AttributeError:
            active = self._active_ids
            result = [e for e in self._edges if e.source in active and e.target in active]
            object.__setattr__(self, '_cached_all_active_edges', result)
            return result

    def neighbors(self, vid: str) -> list[str]:
        """Return ids of vertices adjacent to vid (outgoing + incoming) among active vertices."""
        return sorted(self.neighbor_set(vid))

    def neighbor_set(self, vid: str) -> set[str]:
        """Return set of vertex ids adjacent to vid among active vertices (no sorting)."""
        active = self._active_ids
        out = {e.target for e in self._adj_out.get(vid, ()) if e.target in active}
        inc = {e.source for e in self._adj_in.get(vid, ()) if e.source in active}
        return out | inc

    def out_neighbors(self, vid: str) -> list[str]:
        active = self._active_ids
        return sorted({e.target for e in self._adj_out.get(vid, ()) if e.target in active})

    def out_neighbor_set(self, vid: str) -> set[str]:
        """Return set of outgoing neighbor ids (no sorting)."""
        active = self._active_ids
        return {e.target for e in self._adj_out.get(vid, ()) if e.target in active}

    def in_neighbors(self, vid: str) -> list[str]:
        active = self._active_ids
        return sorted({e.source for e in self._adj_in.get(vid, ()) if e.source in active})

    def in_neighbor_set(self, vid: str) -> set[str]:
        """Return set of incoming neighbor ids (no sorting)."""
        active = self._active_ids
        return {e.source for e in self._adj_in.get(vid, ()) if e.source in active}

    def has_path(self, source: str, target: str) -> bool:
        """BFS on active subgraph (directed edges only)."""
        active = self._active_ids
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
        active = self._active_ids
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

    def copy(self) -> "Graph":
        """Return an independent shallow copy of this graph."""
        g = Graph.__new__(Graph)
        g._vertices = dict(self._vertices)
        g._edges = list(self._edges)
        g._adj_out = {k: list(v) for k, v in self._adj_out.items()}
        g._adj_in = {k: list(v) for k, v in self._adj_in.items()}
        g._active_ids = set(self._active_ids)
        g._edge_keys = set(self._edge_keys)
        return g

    # -- undirected projection for β₁ computation --------------------------

    def undirected_active_edges(self) -> list[frozenset[str]]:
        """Return undirected edge set from active directed edges (no self-loops).

        Excludes material layer edges — beta_1 measures concept-layer topology only.
        Cached; invalidated by mutation methods.
        """
        try:
            return self._cached_undirected
        except AttributeError:
            pass
        active = self._active_ids
        result: set[frozenset[str]] = set()
        for e in self._edges:
            if (e.source in active and e.target in active
                    and e.source != e.target
                    and e.edge_type not in (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)):
                result.add(frozenset((e.source, e.target)))
        out = list(result)
        object.__setattr__(self, '_cached_undirected', out)
        return out

    def self_loops(self) -> list[Edge]:
        """Return self-loops in active graph."""
        active = self._active_ids
        return [e for e in self._edges if e.source == e.target and e.source in active]

    # -- mutation (in-place, returns self) -----------------------------------

    def _invalidate_caches(self) -> None:
        """Clear all derived caches after mutation."""
        for attr in ('_cached_active_edges', '_cached_all_active_edges',
                     '_cached_undirected', '_cached_beta_1', '_cached_terrain'):
            try:
                delattr(self, attr)
            except AttributeError:
                pass

    def add_vertex(self, v: Vertex) -> "Graph":
        self._vertices[v.id] = v
        if v.status != VertexStatus.FOLDED:
            self._active_ids.add(v.id)
        self._invalidate_caches()
        return self

    def add_edge(self, e: Edge) -> "Graph":
        self._edges.append(e)
        self._adj_out.setdefault(e.source, []).append(e)
        self._adj_in.setdefault(e.target, []).append(e)
        self._edge_keys.add((e.source, e.target, e.edge_type))
        # Only invalidate topology caches for concept edges
        if e.edge_type in _MATERIAL_EDGE_TYPES:
            # Material edges don't affect beta_1/terrain — only invalidate edge-list caches
            for attr in ('_cached_active_edges', '_cached_all_active_edges'):
                try:
                    delattr(self, attr)
                except AttributeError:
                    pass
        else:
            self._invalidate_caches()
        return self

    def add_edges_batch(self, edges: list[Edge]) -> "Graph":
        """Add multiple edges in one operation — O(len(edges))."""
        if not edges:
            return self
        self._edges.extend(edges)
        for e in edges:
            self._adj_out.setdefault(e.source, []).append(e)
            self._adj_in.setdefault(e.target, []).append(e)
            self._edge_keys.add((e.source, e.target, e.edge_type))
        if all(e.edge_type in _MATERIAL_EDGE_TYPES for e in edges):
            for attr in ('_cached_active_edges', '_cached_all_active_edges'):
                try:
                    delattr(self, attr)
                except AttributeError:
                    pass
        else:
            self._invalidate_caches()
        return self

    def add_vertices_and_edges_batch(self, vertices: list[Vertex], edges: list[Edge]) -> "Graph":
        """Add multiple vertices and edges in one operation."""
        for v in vertices:
            self._vertices[v.id] = v
            if v.status != VertexStatus.FOLDED:
                self._active_ids.add(v.id)
        if edges:
            self._edges.extend(edges)
            for e in edges:
                self._adj_out.setdefault(e.source, []).append(e)
                self._adj_in.setdefault(e.target, []).append(e)
                self._edge_keys.add((e.source, e.target, e.edge_type))
        self._invalidate_caches()
        return self

    def set_vertex_status(self, vid: str, status: VertexStatus) -> "Graph":
        old = self._vertices[vid]
        self._vertices[vid] = Vertex(old.id, status, old.content, old.created_at, old.non_critical)
        old_active = old.status != VertexStatus.FOLDED
        new_active = status != VertexStatus.FOLDED
        if old_active and not new_active:
            self._active_ids.discard(vid)
        elif not old_active and new_active:
            self._active_ids.add(vid)
        self._invalidate_caches()
        return self

    def merge_vertices(self, keep: str, remove: str) -> "Graph":
        """Merge `remove` into `keep`. Redirect all edges, mark `remove` as folded."""
        old = self._vertices[remove]
        self._vertices[remove] = Vertex(old.id, VertexStatus.FOLDED, old.content, old.created_at, old.non_critical)
        self._active_ids.discard(remove)

        # Redirect edges: rebuild _edges, _adj_out, _adj_in, _edge_keys
        new_edges: list[Edge] = []
        for e in self._edges:
            src = keep if e.source == remove else e.source
            tgt = keep if e.target == remove else e.target
            new_edges.append(Edge(src, tgt, e.edge_type, e.created_at, e.surface, e.context))

        self._edges = new_edges
        self._adj_out = {}
        self._adj_in = {}
        for e in self._edges:
            self._adj_out.setdefault(e.source, []).append(e)
            self._adj_in.setdefault(e.target, []).append(e)
        self._edge_keys = set(
            (e.source, e.target, e.edge_type) for e in self._edges
        )
        self._invalidate_caches()
        return self


# ---------------------------------------------------------------------------
# Rust Graph adapter + factory
# ---------------------------------------------------------------------------

# Enum mapping tables — used by RustGraphAdapter to convert between
# Rust pyclass enums and Python str enums so downstream code that does
# ``e.edge_type == EdgeType.NEGATION`` continues to work.

_PY_TO_RUST_EDGE_TYPE: dict[EdgeType, object] | None = None
_RUST_TO_PY_EDGE_TYPE: dict[object, EdgeType] | None = None
_PY_TO_RUST_VERTEX_STATUS: dict[VertexStatus, object] | None = None
_RUST_TO_PY_VERTEX_STATUS: dict[object, VertexStatus] | None = None


def _init_enum_maps() -> None:
    """Build bidirectional enum mapping tables (called once on first use)."""
    global _PY_TO_RUST_EDGE_TYPE, _RUST_TO_PY_EDGE_TYPE
    global _PY_TO_RUST_VERTEX_STATUS, _RUST_TO_PY_VERTEX_STATUS
    if _PY_TO_RUST_EDGE_TYPE is not None:
        return
    _PY_TO_RUST_EDGE_TYPE = {
        EdgeType.DEPENDENCY: RustEdgeType.Dependency,
        EdgeType.NEGATION: RustEdgeType.Negation,
        EdgeType.SUBLATION: RustEdgeType.Sublation,
        EdgeType.REFERENCE: RustEdgeType.Reference,
        EdgeType.FOLD: RustEdgeType.Fold,
        EdgeType.COOCCURRENCE: RustEdgeType.Cooccurrence,
        EdgeType.TRAVERSAL_ASSOCIATION: RustEdgeType.TraversalAssociation,
        EdgeType.ARTICULATED: RustEdgeType.Articulated,
    }
    _RUST_TO_PY_EDGE_TYPE = {v: k for k, v in _PY_TO_RUST_EDGE_TYPE.items()}
    _PY_TO_RUST_VERTEX_STATUS = {
        VertexStatus.ACTIVE: RustVertexStatus.Active,
        VertexStatus.CONTESTED: RustVertexStatus.Contested,
        VertexStatus.FOLDED: RustVertexStatus.Folded,
    }
    _RUST_TO_PY_VERTEX_STATUS = {v: k for k, v in _PY_TO_RUST_VERTEX_STATUS.items()}


def _py_vertex_to_rust(v: Vertex) -> "RustVertex":
    """Convert a Python Vertex to a Rust Vertex."""
    return RustVertex(
        id=v.id,
        status=_PY_TO_RUST_VERTEX_STATUS[v.status],
        content=v.content,
        created_at=v.created_at,
    )


def _rust_vertex_to_py(rv: "RustVertex") -> Vertex:
    """Convert a Rust Vertex to a Python Vertex."""
    return Vertex(
        id=rv.id,
        status=_RUST_TO_PY_VERTEX_STATUS[rv.status],
        content=rv.content,
        created_at=rv.created_at,
    )


def _py_edge_to_rust(e: Edge) -> "RustEdge":
    """Convert a Python Edge to a Rust Edge."""
    return RustEdge(
        source=e.source,
        target=e.target,
        edge_type=_PY_TO_RUST_EDGE_TYPE[e.edge_type],
        created_at=e.created_at,
        surface=e.surface,
        context=e.context,
    )


def _rust_edge_to_py(re: "RustEdge") -> Edge:
    """Convert a Rust Edge to a Python Edge."""
    return Edge(
        source=re.source,
        target=re.target,
        edge_type=_RUST_TO_PY_EDGE_TYPE[re.edge_type],
        created_at=re.created_at,
        surface=re.surface,
        context=re.context,
    )


# ---------------------------------------------------------------------------
# Zero-copy proxy objects for RustGraphAdapter
# ---------------------------------------------------------------------------

class _AdjProxy:
    """Proxy for _adj_out / _adj_in that delegates to Rust per-vertex edge queries.

    Supports the `.get(vid, default)` pattern used throughout the codebase.
    Each call returns a list of Python Edge objects converted from Rust.
    """

    __slots__ = ("_rg", "_direction")

    def __init__(self, rg: "RustGraph", direction: str) -> None:
        self._rg = rg
        self._direction = direction  # "out" or "in"

    def get(self, vid: str, default=()):
        if self._direction == "out":
            rust_edges = self._rg.out_edges_of(vid)
        else:
            rust_edges = self._rg.in_edges_of(vid)
        if not rust_edges:
            return default
        return [_rust_edge_to_py(re) for re in rust_edges]


class _ActiveIdsProxy:
    """Proxy for _active_ids that delegates to Rust.

    Supports: `in` operator, `len()`, iteration, `set()` / `frozenset()` copy.
    """

    __slots__ = ("_rg",)

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg

    def __contains__(self, vid: str) -> bool:
        return self._rg.contains_active(vid)

    def __len__(self) -> int:
        return self._rg.active_count()

    def __iter__(self):
        return iter(self._rg.active_vertex_ids())

    def __or__(self, other):
        return frozenset(self._rg.active_vertex_ids()) | other

    def __sub__(self, other):
        return frozenset(self._rg.active_vertex_ids()) - other

    def __and__(self, other):
        return frozenset(self._rg.active_vertex_ids()) & other


class _VerticesProxy:
    """Proxy for _vertices that delegates to Rust.

    Supports: `.get(vid)`, `.keys()`, `dict()` copy, `len()`, iteration,
    `.items()`, `vid in proxy`.
    """

    __slots__ = ("_rg",)

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg

    def get(self, vid: str, default=None):
        rv = self._rg.vertex(vid)
        if rv is None:
            return default
        return _rust_vertex_to_py(rv)

    def __getitem__(self, vid: str):
        rv = self._rg.vertex(vid)
        if rv is None:
            raise KeyError(vid)
        return _rust_vertex_to_py(rv)

    def __contains__(self, vid: str) -> bool:
        return self._rg.vertex(vid) is not None

    def __len__(self) -> int:
        return self._rg.vertex_count()

    def keys(self):
        return self._rg.vertex_ids()

    def __iter__(self):
        return iter(self._rg.vertex_ids())

    def items(self):
        rust_verts = self._rg.vertices
        return ((vid, _rust_vertex_to_py(rv)) for vid, rv in rust_verts.items())

    def values(self):
        rust_verts = self._rg.vertices
        return (_rust_vertex_to_py(rv) for rv in rust_verts.values())


class _EdgesProxy:
    """Proxy for _edges that delegates to Rust.

    Supports: `len()`, index slicing `[n:]`, full iteration.
    """

    __slots__ = ("_rg",)

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg

    def __len__(self) -> int:
        return self._rg.edge_count()

    def __iter__(self):
        return iter(_rust_edge_to_py(re) for re in self._rg.edges)

    def __getitem__(self, key):
        if isinstance(key, slice):
            start = key.start or 0
            # Only tail slices (graph._edges[n:]) are used in codebase
            rust_edges = self._rg.edges_tail(start)
            return [_rust_edge_to_py(re) for re in rust_edges]
        # Single index access
        all_edges = self._rg.edges
        return _rust_edge_to_py(all_edges[key])


class _EdgeKeysProxy:
    """Proxy for _edge_keys that delegates to Rust.

    Supports: `(source, target, edge_type) in proxy` membership test.
    """

    __slots__ = ("_rg",)

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg

    def __contains__(self, key: tuple) -> bool:
        source, target, edge_type = key
        return self._rg.contains_edge_key(source, target, _PY_TO_RUST_EDGE_TYPE[edge_type])


class RustGraphAdapter:
    """Versioned-cache wrapper: holds a Rust Graph handle + revision-gated caches.

    _active_ids and _edge_keys are rebuilt only when the Rust-side revision
    counters change.  All other queries proxy to Rust via lightweight proxy
    objects.  Mutation methods propagate caches where revision hasn't changed.
    """

    __slots__ = (
        "_rg",
        "_adj_out", "_adj_in", "_vertices", "_edges",
        # versioned caches
        "_cached_active_ids", "_cached_active_rev",
        "_cached_edge_keys", "_cached_edge_rev",
        # runtime caches (invalidated on mutation)
        "_cached_active_edges", "_cached_all_active_edges",
        "_cached_beta_1", "_cached_terrain",
        "_cached_undirected",
    )

    def __init__(self, rg: "RustGraph") -> None:
        self._rg = rg
        self._adj_out = _AdjProxy(rg, "out")
        self._adj_in = _AdjProxy(rg, "in")
        self._vertices = _VerticesProxy(rg)
        self._edges = _EdgesProxy(rg)
        # versioned caches: built lazily on first access
        self._cached_active_ids = None
        self._cached_active_rev = -1
        self._cached_edge_keys = None
        self._cached_edge_rev = -1

    # -- versioned cache properties -----------------------------------------

    @property
    def _active_ids(self):
        rev = self._rg.active_revision()
        if self._cached_active_rev != rev:
            object.__setattr__(self, '_cached_active_ids', frozenset(self._rg.active_vertex_ids()))
            object.__setattr__(self, '_cached_active_rev', rev)
        return self._cached_active_ids

    @property
    def _edge_keys(self):
        rev = self._rg.edge_revision()
        if self._cached_edge_rev != rev:
            object.__setattr__(self, '_cached_edge_keys', _EdgeKeysProxy(self._rg))
            object.__setattr__(self, '_cached_edge_rev', rev)
        return self._cached_edge_keys

    # -- accessors ----------------------------------------------------------

    @property
    def vertices(self) -> dict[str, Vertex]:
        """Materialise full vertices dict (for callers that need a real dict)."""
        return {vid: _rust_vertex_to_py(rv) for vid, rv in self._rg.vertices.items()}

    @property
    def edges(self) -> list[Edge]:
        """Materialise full edges list (for callers that need a real list)."""
        return [_rust_edge_to_py(re) for re in self._rg.edges]

    def vertex(self, vid: str) -> Vertex | None:
        rv = self._rg.vertex(vid)
        if rv is None:
            return None
        return _rust_vertex_to_py(rv)

    def active_vertex_ids(self) -> list[str]:
        return self._rg.active_vertex_ids()

    def has_edge_key(self, source: str, target: str, edge_type: EdgeType) -> bool:
        return self._rg.contains_edge_key(source, target, _PY_TO_RUST_EDGE_TYPE[edge_type])

    @property
    def edge_keys(self) -> "_EdgeKeysProxy":
        return self._edge_keys

    @property
    def edge_count(self) -> int:
        return self._rg.edge_count()

    def active_edges(self) -> list[Edge]:
        try:
            return self._cached_active_edges
        except AttributeError:
            result = [_rust_edge_to_py(re) for re in self._rg.active_edges()]
            object.__setattr__(self, '_cached_active_edges', result)
            return result

    def all_active_edges(self) -> list[Edge]:
        try:
            return self._cached_all_active_edges
        except AttributeError:
            result = [_rust_edge_to_py(re) for re in self._rg.all_active_edges()]
            object.__setattr__(self, '_cached_all_active_edges', result)
            return result

    def neighbors(self, vid: str) -> list[str]:
        return sorted(self.neighbor_set(vid))

    def neighbor_set(self, vid: str) -> set[str]:
        return self._rg.neighbor_set(vid)

    def out_neighbors(self, vid: str) -> list[str]:
        return self._rg.out_neighbors(vid)

    def out_neighbor_set(self, vid: str) -> set[str]:
        return self._rg.out_neighbor_set(vid)

    def in_neighbors(self, vid: str) -> list[str]:
        return self._rg.in_neighbors(vid)

    def in_neighbor_set(self, vid: str) -> set[str]:
        return self._rg.in_neighbor_set(vid)

    def has_path(self, source: str, target: str) -> bool:
        return self._rg.has_path(source, target)

    def local_subgraph(self, center: str, radius: int = 1) -> tuple[list[str], list[Edge]]:
        rust_vids, rust_edges = self._rg.local_subgraph(center, radius)
        return rust_vids, [_rust_edge_to_py(re) for re in rust_edges]

    def undirected_active_edges(self) -> list[frozenset[str]]:
        try:
            return self._cached_undirected
        except AttributeError:
            pairs = self._rg.undirected_active_edges()
            result = [frozenset(p) for p in pairs]
            object.__setattr__(self, '_cached_undirected', result)
            return result

    def self_loops(self) -> list[Edge]:
        return [_rust_edge_to_py(re) for re in self._rg.self_loops()]

    # -- mutation (returns new RustGraphAdapter) --------------------------------

    def _wrap_versioned(self, new_rg: "RustGraph", active_changed: bool, edge_changed: bool) -> "RustGraphAdapter":
        """Create new adapter, inheriting caches that haven't been invalidated."""
        a = RustGraphAdapter.__new__(RustGraphAdapter)
        object.__setattr__(a, '_rg', new_rg)
        # Proxies: lightweight, always recreated
        object.__setattr__(a, '_adj_out', _AdjProxy(new_rg, "out"))
        object.__setattr__(a, '_adj_in', _AdjProxy(new_rg, "in"))
        object.__setattr__(a, '_vertices', _VerticesProxy(new_rg))
        object.__setattr__(a, '_edges', _EdgesProxy(new_rg))
        # Versioned caches: inherit if revision unchanged, else mark stale
        if active_changed:
            object.__setattr__(a, '_cached_active_ids', None)
            object.__setattr__(a, '_cached_active_rev', -1)
        else:
            object.__setattr__(a, '_cached_active_ids', self._cached_active_ids)
            object.__setattr__(a, '_cached_active_rev', self._cached_active_rev)
        if edge_changed:
            object.__setattr__(a, '_cached_edge_keys', None)
            object.__setattr__(a, '_cached_edge_rev', -1)
        else:
            object.__setattr__(a, '_cached_edge_keys', self._cached_edge_keys)
            object.__setattr__(a, '_cached_edge_rev', self._cached_edge_rev)
        return a

    def add_vertex(self, v: Vertex) -> "RustGraphAdapter":
        new_rg = self._rg.add_vertex(_py_vertex_to_rust(v))
        active_changed = v.status != VertexStatus.FOLDED
        return self._wrap_versioned(new_rg, active_changed=active_changed, edge_changed=False)

    def add_edge(self, e: Edge) -> "RustGraphAdapter":
        new_rg = self._rg.add_edge(_py_edge_to_rust(e))
        adapter = self._wrap_versioned(new_rg, active_changed=False, edge_changed=True)
        # Propagate topology caches if material edge (beta_1/terrain unaffected)
        if e.edge_type in _MATERIAL_EDGE_TYPES:
            for attr in ('_cached_beta_1', '_cached_terrain', '_cached_undirected'):
                try:
                    object.__setattr__(adapter, attr, getattr(self, attr))
                except AttributeError:
                    pass
        return adapter

    def add_edges_batch(self, edges: list[Edge]) -> "RustGraphAdapter":
        if not edges:
            return self
        new_rg = self._rg.add_edges_batch([_py_edge_to_rust(e) for e in edges])
        adapter = self._wrap_versioned(new_rg, active_changed=False, edge_changed=True)
        if all(e.edge_type in _MATERIAL_EDGE_TYPES for e in edges):
            for attr in ('_cached_beta_1', '_cached_terrain', '_cached_undirected'):
                try:
                    object.__setattr__(adapter, attr, getattr(self, attr))
                except AttributeError:
                    pass
        return adapter

    def add_vertices_and_edges_batch(
        self, vertices: list[Vertex], edges: list[Edge],
    ) -> "RustGraphAdapter":
        new_rg = self._rg.add_vertices_and_edges_batch(
            [_py_vertex_to_rust(v) for v in vertices],
            [_py_edge_to_rust(e) for e in edges],
        )
        active_changed = any(v.status != VertexStatus.FOLDED for v in vertices)
        edge_changed = bool(edges)
        return self._wrap_versioned(new_rg, active_changed=active_changed, edge_changed=edge_changed)

    def set_vertex_status(self, vid: str, status: VertexStatus) -> "RustGraphAdapter":
        new_rg = self._rg.set_vertex_status(vid, _PY_TO_RUST_VERTEX_STATUS[status])
        return self._wrap_versioned(new_rg, active_changed=True, edge_changed=False)

    def merge_vertices(self, keep: str, remove: str) -> "RustGraphAdapter":
        new_rg = self._rg.merge_vertices(keep, remove)
        return self._wrap_versioned(new_rg, active_changed=True, edge_changed=True)

    # -- diagnostics --------------------------------------------------------

    def __repr__(self) -> str:
        return (
            f"RustGraphAdapter(vertices={self._rg.vertex_count()}, "
            f"edges={self._rg.edge_count()}, active={self._rg.active_count()})"
        )

    def __len__(self) -> int:
        return self._rg.vertex_count()


def create_graph(
    vertices: dict[str, Vertex] | list[Vertex] | None = None,
    edges: list[Edge] | None = None,
) -> Graph | RustGraphAdapter:
    """Factory: return Rust-backed graph when available, else pure-Python.

    Accepts the same signatures as both Graph (dict vertices) and RustGraph
    (list vertices) constructors, normalising as needed.
    """
    if _RUST_GRAPH_AVAILABLE:
        _init_enum_maps()
        # Normalise vertices to list[RustVertex]
        if vertices is None:
            rv_list: list = []
        elif isinstance(vertices, dict):
            rv_list = [_py_vertex_to_rust(v) for v in vertices.values()]
        else:
            rv_list = [_py_vertex_to_rust(v) for v in vertices]
        re_list = [_py_edge_to_rust(e) for e in edges] if edges else []
        rg = RustGraph(rv_list or None, re_list or None)
        return RustGraphAdapter(rg)
    # Fallback: pure Python
    if vertices is not None and isinstance(vertices, list):
        vertices = {v.id: v for v in vertices}
    return Graph(vertices, edges)


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

    Cached on Graph instance since Graph is immutable — result never changes.
    """
    try:
        return graph._cached_beta_1
    except AttributeError:
        pass
    active_vids = list(graph._active_ids)
    if not active_vids:
        object.__setattr__(graph, '_cached_beta_1', 0)
        return 0
    undirected = graph.undirected_active_edges()
    n_loops = len(graph.self_loops())
    n_v = len(active_vids)
    n_e = len(undirected)
    c = _connected_components(active_vids, undirected)
    result = n_e - n_v + c + n_loops
    object.__setattr__(graph, '_cached_beta_1', result)
    return result


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
    *,
    before_edge_keys: set[tuple[str, str]] | None = None,
) -> frozenset[tuple[str, str]] | None:
    """Find the edges of a newly created cycle (if β₁ increased).

    Simple approach: find an edge in graph_after but not graph_before
    that participates in a cycle. Return the cycle's edge set.

    If before_edge_keys is provided, use it instead of computing from graph_before
    (needed for mutable Graph where graph_before is the same object as graph_after).
    """
    if before_edge_keys is not None:
        before_edges = before_edge_keys
    else:
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
    active = graph._active_ids
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
        # 类型约束：settlement 只结算 concept 层的边，不结算自己产出的 memory 层顶点。
        # memory: 前缀顶点是 settlement 的产物，不是 settlement 的对象——产物不能回流为输入。
        # 不过滤会导致正反馈循环：residue→REFERENCE边→boundary_edge→更多residue→超线性膨胀。
        _MEMORY_PREFIX = "memory:"
        if graph is not None:
            for vid in cycle_vids:
                for e in graph._adj_out.get(vid, ()):
                    if (e.source, e.target) not in cycle_edges and e.target not in cycle_vids:
                        if not e.target.startswith(_MEMORY_PREFIX):
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
                        if not e.source.startswith(_MEMORY_PREFIX):
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
        if not self._pending:
            return []

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
            # 类型约束：memory: 前缀顶点是 settlement 产物，不参与穿越
            _MEMORY_PREFIX = "memory:"
            if graph is not None:
                for vid in cycle_vids:
                    for e in graph._adj_out.get(vid, ()):
                        if (e.source, e.target) not in sc.edges and e.target not in cycle_vids:
                            if not e.target.startswith(_MEMORY_PREFIX):
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
                            if not e.source.startswith(_MEMORY_PREFIX):
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
        # 410号/415号: ghost settlements（操作前已失效的 cycle）标记为 SUBLATED
        if active_edges_before is not None:
            ghosts = [
                sc for sc in self._settled
                if sc.status == "active" and not sc.edges.issubset(active_edges_before)
            ]
            if ghosts:
                ghost_edges_set = {sc.edges for sc in ghosts}
                updated: list[SettledCycle] = []
                for sc in self._settled:
                    if sc.edges in ghost_edges_set and sc.status == "active":
                        updated.append(dataclasses.replace(
                            sc,
                            status="sublated",
                            sublated_at_step=0,
                            sublated_by="ghost_purge",
                        ))
                    else:
                        updated.append(sc)
                self._settled = updated

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

    active = graph._active_ids
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
    # Snapshot active edge keys before mutation for find_new_cycle_edges
    edges_before = {(e.source, e.target) for e in graph.active_edges()}

    # Execute fold
    keep = vertices[0]
    result_graph = graph
    for v in vertices[1:]:
        result_graph = result_graph.merge_vertices(keep, v)
        result_graph = result_graph.add_edge(
            Edge(keep, keep, EdgeType.FOLD, step)
        ) if n_loop > 0 else result_graph

    # 415号: fold 执行后，先将失去物理基础的 settled cycle 标记为 SUBLATED，
    # 然后再检查是否有 active cycle 被破坏。
    # 顺序至关重要：mark_sublated_cycles 必须在 would_destroy_settled 之前执行，
    # 否则已失效的 cycle 会阻塞操作，而 mark_sublated 永远无法到达（死锁）。
    merge_map = {v: keep for v in vertices[1:]}
    settlement.mark_sublated_cycles(result_graph, step=step, operation="fold")

    # 410号推论4方案B: fold isomorphism detection — 仅检查 mark_sublated 后仍为 active 的 cycle
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

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    new_cycle = find_new_cycle_edges(graph, result_graph, before_edge_keys=edges_before) if actual > 0 else None

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
    active_before = set(graph._active_ids)  # snapshot before mutation
    beta_before = compute_beta_1(graph)
    # Snapshot active edge keys before mutation for find_new_cycle_edges
    edges_before = {(e.source, e.target) for e in graph.active_edges()}

    if antithesis is not None and antithesis in active_before:
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
            antithesis = f"anti_{step}"
        predicted = 0

        new_v = Vertex(antithesis, VertexStatus.ACTIVE, content=f"negate({thesis})", created_at=step)
        result_graph = graph.add_vertex(new_v)
        result_graph = result_graph.add_edge(
            Edge(antithesis, thesis, EdgeType.NEGATION, step)
        )
        result_graph = result_graph.set_vertex_status(thesis, VertexStatus.CONTESTED)

    # 415号: negate 只加边不删边 → active cycles 不会被破坏。
    # 但仍先执行 mark_sublated_cycles 以确保 ghost settlements 被清除，
    # 避免 would_destroy_settled 的 ghost purge 路径删除 cycle 而非标记 SUBLATED。
    settlement.mark_sublated_cycles(result_graph, step=step, operation="negate")

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

    new_cycle = find_new_cycle_edges(graph, result_graph, before_edge_keys=edges_before) if actual > 0 else None
    new_vertex = antithesis if antithesis not in active_before else None

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
    # Snapshot active edge keys before mutation for find_new_cycle_edges
    edges_before = {(e.source, e.target) for e in graph.active_edges()}
    predicted = 1

    synthesis_id = f"syn_{step}"
    new_v = Vertex(synthesis_id, VertexStatus.ACTIVE, content=synthesis_content, created_at=step)

    result_graph = graph.add_vertex(new_v)
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, thesis, EdgeType.SUBLATION, step)
    )
    result_graph = result_graph.add_edge(
        Edge(synthesis_id, antithesis, EdgeType.SUBLATION, step)
    )

    # 415号: sublate 执行后，先标记 SUBLATED，再检查 active cycle
    # （与 fold 相同的顺序修复——避免死锁）
    settlement.mark_sublated_cycles(result_graph, step=step, operation="sublate")

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

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    new_cycle = find_new_cycle_edges(graph, result_graph, before_edge_keys=edges_before) if actual > 0 else None

    return OperationResult(
        graph=result_graph,
        delta_beta_1_predicted=predicted,
        delta_beta_1_actual=actual,
        new_vertex=synthesis_id,
        new_cycle_edges=new_cycle,
        sublation_record=sublation_record,
    )
