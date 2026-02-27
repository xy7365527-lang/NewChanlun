"""转换拓扑测试。"""

from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    ConfigurationSpace,
    WalkDirection,
    polarity_index,
)
from newchan.topology.transition import (
    adjacent,
    all_paths,
    all_shortest_paths,
    bipartite_partition,
    graph_diameter,
    graph_edge_count,
    graph_radius,
    is_adjacent,
    is_bipartite,
    reachable,
    shortest_path,
    shortest_path_count,
)


class TestAdjacent:
    def test_center_has_6_neighbors(self):
        nbs = adjacent(CENTER)
        assert len(nbs) == 6

    def test_corner_has_3_neighbors(self):
        nbs = adjacent(FULL_RISK_ON)
        assert len(nbs) == 3

    def test_edge_node_has_4_neighbors(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN)
        nbs = adjacent(c)
        assert len(nbs) == 4

    def test_face_node_has_5_neighbors(self):
        c = Configuration(WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.DOWN)
        nbs = adjacent(c)
        assert len(nbs) == 5

    def test_symmetry(self):
        """转换拓扑的对称性：a 是 b 的邻居 iff b 是 a 的邻居。"""
        space = ConfigurationSpace()
        for c1 in space:
            for c2 in adjacent(c1):
                assert c1 in adjacent(c2), (
                    f"{c1.label} -> {c2.label} 但反向不成立"
                )


class TestIsAdjacent:
    def test_adjacent_pair(self):
        assert is_adjacent(CENTER, Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT))

    def test_non_adjacent_two_changes(self):
        c1 = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.DOWN, WalkDirection.FLAT)
        assert not is_adjacent(c1, c2)

    def test_non_adjacent_skip(self):
        """+ -> - 禁止直接跳转。"""
        c1 = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.FLAT, WalkDirection.FLAT)
        assert not is_adjacent(c1, c2)

    def test_same_config(self):
        assert not is_adjacent(CENTER, CENTER)


class TestShortestPath:
    def test_same_config(self):
        assert shortest_path(CENTER, CENTER) == 0

    def test_adjacent(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        assert shortest_path(CENTER, c) == 1

    def test_full_reversal_distance_6(self):
        """d((+,+,+), (-,-,-)) = 6。"""
        assert shortest_path(FULL_RISK_ON, FULL_RISK_OFF) == 6

    def test_center_to_corner(self):
        assert shortest_path(CENTER, FULL_RISK_ON) == 3

    def test_symmetric(self):
        c1 = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.FLAT, WalkDirection.UP)
        assert shortest_path(c1, c2) == shortest_path(c2, c1)


class TestShortestPathCount:
    def test_same_config(self):
        assert shortest_path_count(CENTER, CENTER) == 1

    def test_adjacent(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        assert shortest_path_count(CENTER, c) == 1

    def test_full_reversal_90_paths(self):
        """(+,+,+) -> (-,-,-) 有 90 条最短路径。

        6! / (2! * 2! * 2!) = 720 / 8 = 90。
        """
        assert shortest_path_count(FULL_RISK_ON, FULL_RISK_OFF) == 90


class TestReachable:
    def test_k0(self):
        """0 步可达集只有自身。"""
        r = reachable(CENTER, 0)
        assert r == frozenset({CENTER})

    def test_k1_from_center(self):
        """1 步可达集 = 自身 + 6 个邻居 = 7。"""
        r = reachable(CENTER, 1)
        assert len(r) == 7
        assert CENTER in r

    def test_k3_from_center_all_reachable(self):
        """从中心出发 3 步可达所有 27 个配置。"""
        r = reachable(CENTER, 3)
        assert len(r) == 27

    def test_k1_from_corner(self):
        r = reachable(FULL_RISK_ON, 1)
        assert len(r) == 4  # 自身 + 3 邻居

    def test_negative_k_raises(self):
        try:
            reachable(CENTER, -1)
            assert False, "应该抛出 ValueError"
        except ValueError:
            pass


class TestAllPaths:
    def test_same_config(self):
        paths = all_paths(CENTER, CENTER, max_k=0)
        assert len(paths) == 1
        assert paths[0].length == 0

    def test_adjacent_shortest(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        paths = all_paths(CENTER, c, max_k=1)
        assert len(paths) == 1
        assert paths[0].length == 1

    def test_max_k_too_large_raises(self):
        try:
            all_paths(CENTER, FULL_RISK_ON, max_k=13)
            assert False, "应该抛出 ValueError"
        except ValueError:
            pass

    def test_shortest_paths_count_matches(self):
        """验证最短路径枚举数量与公式一致。"""
        c1 = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.FLAT, WalkDirection.UP)
        # d1=2, d2=1, d3=1 -> 4!/(2!*1!*1!) = 12
        expected_count = shortest_path_count(c1, c2)
        paths = all_shortest_paths(c1, c2)
        assert len(paths) == expected_count


class TestAllShortestPaths:
    def test_adjacent(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        paths = all_shortest_paths(CENTER, c)
        assert len(paths) == 1
        assert paths[0].length == 1

    def test_center_to_corner_6_paths(self):
        """中心到角节点 (+,+,+) 的最短路径数。

        d1=1, d2=1, d3=1 -> 3!/(1!*1!*1!) = 6。
        """
        paths = all_shortest_paths(CENTER, FULL_RISK_ON)
        assert len(paths) == 6
        for p in paths:
            assert p.length == 3

    def test_polarity_sequence_monotone(self):
        """中心到 (+,+,+) 的最短路径上极性指数单调递增。"""
        paths = all_shortest_paths(CENTER, FULL_RISK_ON)
        for p in paths:
            seq = p.polarity_sequence
            for i in range(len(seq) - 1):
                assert seq[i + 1] > seq[i]


class TestGraphProperties:
    def test_edge_count_54(self):
        """转换拓扑图恰好 54 条边。"""
        assert graph_edge_count() == 54

    def test_diameter_6(self):
        """直径 = 6。"""
        assert graph_diameter() == 6

    def test_radius_3(self):
        """半径 = 3。"""
        assert graph_radius() == 3

    def test_bipartite(self):
        """P3^3 是二部图。"""
        assert is_bipartite()

    def test_bipartite_partition_sizes(self):
        """二部划分：14 + 13 = 27。"""
        even, odd = bipartite_partition()
        assert len(even) + len(odd) == 27
        assert len(even) == 14
        assert len(odd) == 13

    def test_no_edge_within_partition(self):
        """二部划分内部无边。"""
        even, odd = bipartite_partition()
        for c1 in even:
            for c2 in adjacent(c1):
                assert c2 not in even, (
                    f"同一划分内有边：{c1.label} -> {c2.label}"
                )
