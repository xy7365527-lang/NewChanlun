"""配置空间测试。"""

from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    ConfigurationSpace,
    WalkDirection,
    manhattan_distance,
    polarity_index,
)


class TestWalkDirection:
    def test_values(self):
        assert WalkDirection.UP == 1
        assert WalkDirection.FLAT == 0
        assert WalkDirection.DOWN == -1

    def test_three_directions(self):
        assert len(WalkDirection) == 3


class TestConfiguration:
    def test_as_tuple(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN)
        assert c.as_tuple == (1, 0, -1)

    def test_zero_count_corner(self):
        c = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.DOWN)
        assert c.zero_count == 0
        assert c.node_type == "corner"

    def test_zero_count_edge(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN)
        assert c.zero_count == 1
        assert c.node_type == "edge"

    def test_zero_count_face(self):
        c = Configuration(WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.DOWN)
        assert c.zero_count == 2
        assert c.node_type == "face"

    def test_zero_count_center(self):
        assert CENTER.zero_count == 3
        assert CENTER.node_type == "center"

    # 注：旧 `Configuration.degree`（按 zero_count 计 3/4/5/6 度）属于 527号判定为
    # 错误的 54 边位置态模型，已移除。81 边图中每节点度数恒为 6，由
    # transition.node_degree 承载（见 test_transition.py::test_all_nodes_degree_6）。

    def test_label(self):
        c = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN)
        assert c.label == "(+,0,-)"

    def test_frozen(self):
        c = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.UP)
        try:
            c.sigma_p = WalkDirection.DOWN  # type: ignore
            assert False, "应该抛出 FrozenInstanceError"
        except AttributeError:
            pass


class TestPolarityIndex:
    def test_full_risk_on(self):
        """(+,+,+) -> S = +3"""
        assert polarity_index(FULL_RISK_ON) == 3

    def test_full_risk_off(self):
        """(-,-,-) -> S = -3"""
        assert polarity_index(FULL_RISK_OFF) == -3

    def test_center(self):
        """(0,0,0) -> S = 0"""
        assert polarity_index(CENTER) == 0

    def test_mixed(self):
        c = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT)
        assert polarity_index(c) == 0  # +1 + (-1) + 0 = 0

    def test_range(self):
        """极性指数范围 [-3, +3]。"""
        space = ConfigurationSpace()
        polarities = {polarity_index(c) for c in space}
        assert min(polarities) == -3
        assert max(polarities) == 3
        assert polarities == {-3, -2, -1, 0, 1, 2, 3}


class TestManhattanDistance:
    def test_same_config(self):
        assert manhattan_distance(CENTER, CENTER) == 0

    def test_adjacent(self):
        c1 = CENTER
        c2 = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.FLAT)
        assert manhattan_distance(c1, c2) == 1

    def test_full_reversal(self):
        """(+,+,+) 到 (-,-,-) 距离为 6。"""
        assert manhattan_distance(FULL_RISK_ON, FULL_RISK_OFF) == 6

    def test_center_to_corner(self):
        """中心到任意角节点距离为 3。"""
        assert manhattan_distance(CENTER, FULL_RISK_ON) == 3
        assert manhattan_distance(CENTER, FULL_RISK_OFF) == 3

    def test_symmetric(self):
        c1 = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT)
        c2 = Configuration(WalkDirection.DOWN, WalkDirection.UP, WalkDirection.UP)
        assert manhattan_distance(c1, c2) == manhattan_distance(c2, c1)


class TestConfigurationSpace:
    def setup_method(self):
        self.space = ConfigurationSpace()

    def test_size_27(self):
        """27 种配置的枚举完整性。"""
        assert self.space.size == 27
        assert len(self.space) == 27

    def test_all_configs_unique(self):
        tuples = [c.as_tuple for c in self.space]
        assert len(set(tuples)) == 27

    def test_contains(self):
        assert CENTER in self.space
        assert FULL_RISK_ON in self.space

    def test_get(self):
        c = self.space.get(1, 0, -1)
        assert c.as_tuple == (1, 0, -1)

    def test_get_invalid(self):
        try:
            self.space.get(2, 0, 0)
            assert False, "应该抛出 KeyError"
        except KeyError:
            pass

    def test_node_type_counts(self):
        """corner=8, edge=12, face=6, center=1。"""
        assert len(self.space.configs_by_type("corner")) == 8
        assert len(self.space.configs_by_type("edge")) == 12
        assert len(self.space.configs_by_type("face")) == 6
        assert len(self.space.configs_by_type("center")) == 1

    def test_polarity_distribution(self):
        """极性指数的分布。"""
        for s in range(-3, 4):
            configs = self.space.configs_by_polarity(s)
            assert len(configs) >= 1

    def test_polarity_s3_unique(self):
        """S=+3 只有 (+,+,+)，S=-3 只有 (-,-,-)。"""
        assert len(self.space.configs_by_polarity(3)) == 1
        assert len(self.space.configs_by_polarity(-3)) == 1

    def test_polarity_range(self):
        assert self.space.polarity_range == (-3, 3)
