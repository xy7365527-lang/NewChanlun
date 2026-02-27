"""K4 图结构测试。"""

from newchan.topology.graph import (
    Edge,
    EdgeType,
    K4Graph,
    Vertex,
)


class TestVertex:
    def test_four_vertices(self):
        assert len(Vertex) == 4

    def test_vertex_values(self):
        assert Vertex.E.value == "动产"
        assert Vertex.C.value == "商品"
        assert Vertex.R.value == "不动产"
        assert Vertex.CASH.value == "现金"


class TestEdge:
    def test_label(self):
        e = Edge(Vertex.E, Vertex.CASH, EdgeType.INDEPENDENT)
        assert e.label == "E/CASH"

    def test_vertices_frozenset(self):
        e = Edge(Vertex.E, Vertex.C, EdgeType.DERIVED)
        assert e.vertices == frozenset({Vertex.E, Vertex.C})

    def test_frozen(self):
        e = Edge(Vertex.E, Vertex.CASH, EdgeType.INDEPENDENT)
        try:
            e.vertex_a = Vertex.C  # type: ignore
            assert False, "应该抛出 FrozenInstanceError"
        except AttributeError:
            pass


class TestK4Graph:
    def setup_method(self):
        self.g = K4Graph()

    def test_vertex_count(self):
        assert self.g.vertex_count == 4

    def test_edge_count_exactly_6(self):
        """K4 恰好 6 条边。"""
        assert self.g.edge_count == 6

    def test_independent_edges_count(self):
        assert len(self.g.get_independent_edges()) == 3

    def test_derived_edges_count(self):
        assert len(self.g.get_derived_edges()) == 3

    def test_independent_edges_all_connect_cash(self):
        for e in self.g.get_independent_edges():
            assert Vertex.CASH in e.vertices
            assert e.edge_type == EdgeType.INDEPENDENT

    def test_derived_edges_no_cash(self):
        for e in self.g.get_derived_edges():
            assert Vertex.CASH not in e.vertices
            assert e.edge_type == EdgeType.DERIVED

    def test_all_edges_have_distinct_vertex_pairs(self):
        pairs = [e.vertices for e in self.g.edges]
        assert len(pairs) == len(set(map(frozenset, pairs)))

    def test_triangle_ecr(self):
        tri = self.g.get_triangle(Vertex.E, Vertex.C, Vertex.R)
        assert len(tri) == 3
        vertex_set = set()
        for e in tri:
            vertex_set.update(e.vertices)
        assert vertex_set == {Vertex.E, Vertex.C, Vertex.R}

    def test_triangle_with_cash(self):
        tri = self.g.get_triangle(Vertex.E, Vertex.C, Vertex.CASH)
        assert len(tri) == 3

    def test_all_triangles_count(self):
        """K4 有 C(4,3) = 4 个三角形。"""
        triangles = self.g.get_all_triangles()
        assert len(triangles) == 4

    def test_triangle_invalid_duplicate_vertex(self):
        try:
            self.g.get_triangle(Vertex.E, Vertex.E, Vertex.C)
            assert False, "应该抛出 ValueError"
        except ValueError:
            pass

    def test_edges_incident_to_cash(self):
        """CASH 关联 3 条边。"""
        edges = self.g.edges_incident_to(Vertex.CASH)
        assert len(edges) == 3

    def test_edges_incident_to_equity(self):
        """E 关联 3 条边。"""
        edges = self.g.edges_incident_to(Vertex.E)
        assert len(edges) == 3
