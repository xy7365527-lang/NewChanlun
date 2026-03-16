"""Integration tests for graph_rs Python bindings.

Run after `maturin develop` in the graph-rs directory:
    cd topological-computation/graph-rs
    maturin develop
    pytest tests/test_graph_rs.py -v
"""

import pytest

# Import will fail if maturin develop hasn't been run
graph_rs = pytest.importorskip("graph_rs")

from graph_rs import Graph, Vertex, VertexStatus, Edge, EdgeType


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def empty_graph():
    return Graph()


@pytest.fixture
def triangle_graph():
    """a -> b -> c -> a (triangle with beta_1 = 1)."""
    g = Graph()
    g = g.add_vertex(Vertex("a"))
    g = g.add_vertex(Vertex("b"))
    g = g.add_vertex(Vertex("c"))
    g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
    g = g.add_edge(Edge("b", "c", EdgeType.Dependency))
    g = g.add_edge(Edge("c", "a", EdgeType.Dependency))
    return g


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------

class TestConstruction:
    def test_empty_graph(self, empty_graph):
        g = empty_graph
        assert len(g) == 0
        assert len(g.vertices) == 0
        assert len(g.edges) == 0

    def test_with_initial_data(self):
        verts = [Vertex("a"), Vertex("b")]
        edges = [Edge("a", "b", EdgeType.Dependency)]
        g = Graph(verts, edges)
        assert len(g) == 2
        assert len(g.edges) == 1

    def test_vertex_defaults(self):
        v = Vertex("x")
        assert v.id == "x"
        assert v.status == VertexStatus.Active
        assert v.content is None
        assert v.created_at == 0

    def test_vertex_with_args(self):
        v = Vertex("x", VertexStatus.Contested, "some content", 42)
        assert v.status == VertexStatus.Contested
        assert v.content == "some content"
        assert v.created_at == 42

    def test_edge_defaults(self):
        e = Edge("a", "b", EdgeType.Negation)
        assert e.source == "a"
        assert e.target == "b"
        assert e.edge_type == EdgeType.Negation
        assert e.created_at == 0
        assert e.surface is None
        assert e.context is None

    def test_edge_with_surface(self):
        e = Edge("a", "b", EdgeType.Dependency, surface="implies")
        assert e.surface == "implies"


# ---------------------------------------------------------------------------
# Vertex operations
# ---------------------------------------------------------------------------

class TestVertexOps:
    def test_add_vertex(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        assert "a" in g.vertices
        assert "a" in g.active_vertex_ids()

    def test_add_folded_vertex(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a", VertexStatus.Folded))
        assert "a" in g.vertices
        assert "a" not in g.active_vertex_ids()

    def test_vertex_lookup(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        v = g.vertex("a")
        assert v is not None
        assert v.id == "a"
        assert g.vertex("missing") is None

    def test_set_vertex_status(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g2 = g.set_vertex_status("a", VertexStatus.Folded)
        assert "a" in g.active_vertex_ids()
        assert "a" not in g2.active_vertex_ids()

    def test_set_vertex_status_unfold(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a", VertexStatus.Folded))
        g2 = g.set_vertex_status("a", VertexStatus.Active)
        assert "a" not in g.active_vertex_ids()
        assert "a" in g2.active_vertex_ids()

    def test_set_vertex_status_not_found(self, empty_graph):
        with pytest.raises(KeyError):
            empty_graph.set_vertex_status("missing", VertexStatus.Active)


# ---------------------------------------------------------------------------
# Edge operations & neighbors
# ---------------------------------------------------------------------------

class TestEdgeOps:
    def test_add_edge(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert g.has_edge_key("a", "b", EdgeType.Dependency)
        assert not g.has_edge_key("b", "a", EdgeType.Dependency)

    def test_neighbors(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert g.neighbors("a") == ["b"]
        assert g.neighbors("b") == ["a"]

    def test_out_in_neighbors(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert g.out_neighbors("a") == ["b"]
        assert g.out_neighbors("b") == []
        assert g.in_neighbors("b") == ["a"]
        assert g.in_neighbors("a") == []

    def test_neighbor_set(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert g.neighbor_set("a") == {"b"}
        assert g.out_neighbor_set("a") == {"b"}
        assert g.in_neighbor_set("b") == {"a"}

    def test_neighbors_exclude_folded(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.set_vertex_status("b", VertexStatus.Folded)
        assert g.neighbors("a") == []


# ---------------------------------------------------------------------------
# Active edges
# ---------------------------------------------------------------------------

class TestActiveEdges:
    def test_active_edges_exclude_material(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("a", "b", EdgeType.Cooccurrence))
        g = g.add_edge(Edge("a", "b", EdgeType.TraversalAssociation))
        assert len(g.active_edges()) == 1
        assert len(g.all_active_edges()) == 3

    def test_active_edges_exclude_folded(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.set_vertex_status("b", VertexStatus.Folded)
        assert len(g.active_edges()) == 0
        assert len(g.all_active_edges()) == 0


# ---------------------------------------------------------------------------
# Self-loops
# ---------------------------------------------------------------------------

class TestSelfLoops:
    def test_self_loops(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_edge(Edge("a", "a", EdgeType.Dependency))
        loops = g.self_loops()
        assert len(loops) == 1
        assert loops[0].source == "a"
        assert loops[0].target == "a"

    def test_self_loops_include_material(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_edge(Edge("a", "a", EdgeType.Cooccurrence))
        assert len(g.self_loops()) == 1


# ---------------------------------------------------------------------------
# Beta_1
# ---------------------------------------------------------------------------

class TestBeta1:
    def test_empty(self, empty_graph):
        assert empty_graph.beta_1() == 0

    def test_triangle(self, triangle_graph):
        assert triangle_graph.beta_1() == 1

    def test_tree(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_vertex(Vertex("c"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("a", "c", EdgeType.Dependency))
        assert g.beta_1() == 0

    def test_with_self_loop(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("a", "a", EdgeType.Negation))
        assert g.beta_1() == 1

    def test_material_excluded(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Cooccurrence))
        assert g.beta_1() == 0


# ---------------------------------------------------------------------------
# has_path
# ---------------------------------------------------------------------------

class TestHasPath:
    def test_direct(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert g.has_path("a", "b")
        assert not g.has_path("b", "a")

    def test_transitive(self, triangle_graph):
        assert triangle_graph.has_path("a", "c")

    def test_through_folded(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_vertex(Vertex("c"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("b", "c", EdgeType.Dependency))
        g = g.set_vertex_status("b", VertexStatus.Folded)
        assert not g.has_path("a", "c")


# ---------------------------------------------------------------------------
# Batch operations
# ---------------------------------------------------------------------------

class TestBatch:
    def test_add_edges_batch(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_vertex(Vertex("c"))
        g = g.add_edges_batch([
            Edge("a", "b", EdgeType.Dependency),
            Edge("b", "c", EdgeType.Reference),
        ])
        assert g.has_edge_key("a", "b", EdgeType.Dependency)
        assert g.has_edge_key("b", "c", EdgeType.Reference)

    def test_add_vertices_and_edges_batch(self, empty_graph):
        g = empty_graph.add_vertices_and_edges_batch(
            [Vertex("a"), Vertex("b"), Vertex("c")],
            [
                Edge("a", "b", EdgeType.Dependency),
                Edge("b", "c", EdgeType.Reference),
            ],
        )
        assert len(g) == 3
        assert len(g.edges) == 2


# ---------------------------------------------------------------------------
# Merge
# ---------------------------------------------------------------------------

class TestMerge:
    def test_merge_vertices(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_vertex(Vertex("c"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("b", "c", EdgeType.Negation))
        g = g.merge_vertices("a", "b")
        assert "b" not in g.active_vertex_ids()
        assert g.has_edge_key("a", "a", EdgeType.Dependency)
        assert g.has_edge_key("a", "c", EdgeType.Negation)


# ---------------------------------------------------------------------------
# local_subgraph
# ---------------------------------------------------------------------------

class TestLocalSubgraph:
    def test_radius_1(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_vertex(Vertex("c"))
        g = g.add_vertex(Vertex("d"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        g = g.add_edge(Edge("b", "c", EdgeType.Dependency))
        g = g.add_edge(Edge("c", "d", EdgeType.Dependency))
        verts, edges = g.local_subgraph("b", 1)
        assert "a" in verts
        assert "b" in verts
        assert "c" in verts
        assert "d" not in verts
        assert len(edges) == 2

    def test_radius_0(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        verts, edges = g.local_subgraph("a", 0)
        assert verts == ["a"]
        assert len(edges) == 0


# ---------------------------------------------------------------------------
# Edge keys property
# ---------------------------------------------------------------------------

class TestEdgeKeys:
    def test_edge_keys_getter(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        keys = g.edge_keys
        assert len(keys) == 1
        s, t, et = keys[0]
        assert s == "a"
        assert t == "b"
        assert et == "dependency"


# ---------------------------------------------------------------------------
# Immutability semantics (Python-side)
# ---------------------------------------------------------------------------

class TestImmutability:
    def test_add_vertex_immutable(self, empty_graph):
        g2 = empty_graph.add_vertex(Vertex("a"))
        assert len(empty_graph) == 0
        assert len(g2) == 1

    def test_add_edge_immutable(self, empty_graph):
        g = empty_graph.add_vertex(Vertex("a"))
        g = g.add_vertex(Vertex("b"))
        g2 = g.add_edge(Edge("a", "b", EdgeType.Dependency))
        assert len(g.edges) == 0
        assert len(g2.edges) == 1


# ---------------------------------------------------------------------------
# Enum values
# ---------------------------------------------------------------------------

class TestEnumValues:
    def test_vertex_status_values(self):
        assert VertexStatus.Active.value == "active"
        assert VertexStatus.Contested.value == "contested"
        assert VertexStatus.Folded.value == "folded"

    def test_edge_type_values(self):
        assert EdgeType.Dependency.value == "dependency"
        assert EdgeType.Negation.value == "negation"
        assert EdgeType.Sublation.value == "sublation"
        assert EdgeType.Reference.value == "reference"
        assert EdgeType.Fold.value == "fold"
        assert EdgeType.Cooccurrence.value == "cooccurrence"
        assert EdgeType.TraversalAssociation.value == "traversal_association"
        assert EdgeType.Articulated.value == "articulated"

    def test_repr(self, empty_graph):
        r = repr(empty_graph)
        assert "Graph(" in r
        assert "vertices=0" in r
