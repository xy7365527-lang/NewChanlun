"""转换拓扑测试（81 边图，527号定理：σ 走势方向态，可 +↔− 直接跳变）。"""

from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    ConfigurationSpace,
    WalkDirection,
)
from newchan.topology.transition import (
    adjacent,
    all_paths,
    all_shortest_paths,
    graph_diameter,
    graph_edge_count,
    graph_radius,
    is_adjacent,
    is_bipartite,
    node_degree,
    reachable,
    shortest_path,
    shortest_path_count,
)


class TestAdjacent:
    def test_center_has_6_neighbors(self):
        nbs = adjacent(CENTER)
        assert len(nbs) == 6

    def test_corner_has_6_neighbors(self):
        """81 边图中角节点也有 6 个邻居（+↔− 直接可达）。"""
        nbs = adjacent(FULL_RISK_ON)
        assert len(nbs) == 6

    def test_all_nodes_degree_6(self):
        """81 边图中每个节点度数恒为 6。"""
        space = ConfigurationSpace()
        for c in space:
            assert node_degree(c) == 6

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

    def test_direct_reversal_is_adjacent(self):
        """527号：+ → - 可直接跳转（走势必完美），故相邻。"""
        c1 = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.FLAT, WalkDirection.FLAT)
        assert is_adjacent(c1, c2)

    def test_same_config(self):
        assert not is_adjacent(CENTER, CENTER)


class TestShortestPath:
    def test_same_config(self):
        assert shortest_path(CENTER, CENTER) == 0

    def test_adjacent(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        assert shortest_path(CENTER, c) == 1

    def test_full_reversal_distance_3(self):
        """527号：d((+,+,+), (-,-,-)) = 3（每分量一步直跳，Hamming 距离）。"""
        assert shortest_path(FULL_RISK_ON, FULL_RISK_OFF) == 3

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

    def test_full_reversal_6_paths(self):
        """527号：(+,+,+) -> (-,-,-) 有 3! = 6 条最短路径（3 分量翻转次序任意）。"""
        assert shortest_path_count(FULL_RISK_ON, FULL_RISK_OFF) == 6


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
        """81 边图中角节点也有 6 邻居，故 1 步可达 7 个。"""
        r = reachable(FULL_RISK_ON, 1)
        assert len(r) == 7

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
        """验证最短路径枚举数量与公式 k! 一致。"""
        c1 = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.FLAT, WalkDirection.UP)
        # 3 个分量全不同 -> 3! = 6
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
        """中心到角节点 (+,+,+) 的最短路径数 = 3! = 6（3 分量各一步直达）。"""
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
    def test_edge_count_81(self):
        """527号：配置转换拓扑图恰好 81 条边（旧 54 边是实现错误）。"""
        assert graph_edge_count() == 81

    def test_diameter_3(self):
        """527号：直径 = 3（Hamming 距离上界）。"""
        assert graph_diameter() == 3

    def test_radius_3(self):
        """527号：半径 = 3（每节点偏心率均为 3）。"""
        assert graph_radius() == 3

    def test_not_bipartite(self):
        """527号：每分量 3 态全连通 = K3 含奇环，整图非二部。"""
        assert not is_bipartite()
