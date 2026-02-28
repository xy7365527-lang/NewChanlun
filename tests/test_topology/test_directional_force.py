"""方向性力度指标测试。

239号谱系：验证方向性力度 ∉ ker(D) 的三种方法。
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import pytest

from newchan.topology.directional_force import (
    DirectionalDivergence,
    ForceComparison,
    LevelForceComparison,
    _amplitude_force,
    _max_consecutive_same_direction,
    compare_force_methods,
    compare_level_force,
    composite_directional_force,
    detect_directional_divergence,
    directional_persistence,
    directional_persistence_weighted,
    level_amplitude_force,
    level_component_density_force,
    level_directional_force,
    level_directional_persistence,
    level_directional_persistence_weighted,
    level_zhongshu_drift_force,
    stroke_density_force,
    stroke_density_from_segments,
    zhongshu_drift_force,
)


# ── 测试用 fake 数据类 ──────────────────────────────────────────


@dataclass(frozen=True)
class FakeSegment:
    direction: Literal["up", "down"]
    high: float
    low: float
    i0: int
    i1: int


@dataclass(frozen=True)
class FakeStroke:
    i0: int
    i1: int


@dataclass(frozen=True)
class FakeZhongshu:
    zd: float
    zg: float
    dd: float
    gg: float
    seg_start: int
    seg_end: int
    settled: bool = True


# ── 方法 A: directional_persistence ─────────────────────────────


class TestDirectionalPersistence:
    """测试方向持续性力度。"""

    def test_all_same_direction(self) -> None:
        """所有线段同向 → 持续性 = 1.0。"""
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("up", 12.0, 6.0, 10, 20),
            FakeSegment("up", 15.0, 8.0, 20, 30),
        ]
        result = directional_persistence(segments, 0, 2, "up")
        assert result == pytest.approx(1.0)

    def test_alternating_direction(self) -> None:
        """交替方向 → 持续性 = 0.5。"""
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
            FakeSegment("up", 11.0, 6.0, 20, 30),
            FakeSegment("down", 10.0, 5.0, 30, 40),
        ]
        result = directional_persistence(segments, 0, 3, "up")
        assert result == pytest.approx(0.5)

    def test_no_same_direction(self) -> None:
        """无同向线段 → 持续性 = 0.0。"""
        segments = [
            FakeSegment("down", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
        ]
        result = directional_persistence(segments, 0, 1, "up")
        assert result == pytest.approx(0.0)

    def test_invalid_range(self) -> None:
        """无效范围 → 0.0。"""
        segments = [FakeSegment("up", 10.0, 5.0, 0, 10)]
        assert directional_persistence(segments, 5, 0, "up") == 0.0
        assert directional_persistence(segments, -1, 0, "up") == 0.0
        assert directional_persistence(segments, 0, 5, "up") == 0.0

    def test_single_segment(self) -> None:
        """单线段同向 → 1.0。"""
        segments = [FakeSegment("up", 10.0, 5.0, 0, 10)]
        assert directional_persistence(segments, 0, 0, "up") == pytest.approx(1.0)


class TestMaxConsecutiveSameDirection:
    """测试最长连续同向线段数。"""

    def test_all_same(self) -> None:
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("up", 12.0, 6.0, 10, 20),
            FakeSegment("up", 15.0, 8.0, 20, 30),
        ]
        assert _max_consecutive_same_direction(segments, 0, 2, "up") == 3

    def test_broken_run(self) -> None:
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("up", 12.0, 6.0, 10, 20),
            FakeSegment("down", 11.0, 5.0, 20, 30),
            FakeSegment("up", 13.0, 7.0, 30, 40),
        ]
        assert _max_consecutive_same_direction(segments, 0, 3, "up") == 2

    def test_invalid_range(self) -> None:
        segments = [FakeSegment("up", 10.0, 5.0, 0, 10)]
        assert _max_consecutive_same_direction(segments, 5, 0, "up") == 0


class TestDirectionalPersistenceWeighted:
    """测试加权方向持续性。"""

    def test_perfect_persistence(self) -> None:
        """完美持续性 → 1.0。"""
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("up", 12.0, 6.0, 10, 20),
            FakeSegment("up", 15.0, 8.0, 20, 30),
        ]
        result = directional_persistence_weighted(segments, 0, 2, "up")
        assert result == pytest.approx(1.0)

    def test_degraded_persistence(self) -> None:
        """交替方向 → 加权值 < 比例值。"""
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
            FakeSegment("up", 11.0, 6.0, 20, 30),
            FakeSegment("down", 10.0, 5.0, 30, 40),
        ]
        ratio = directional_persistence(segments, 0, 3, "up")
        weighted = directional_persistence_weighted(segments, 0, 3, "up")
        assert weighted < ratio  # 加权惩罚不连续


# ── 方法 B: stroke_density_force ─────────────────────────────────


class TestStrokeDensityForce:
    """测试笔密度力度。"""

    def test_sparse_strokes_high_force(self) -> None:
        """稀疏笔 → 高力度。"""
        strokes = [
            FakeStroke(0, 50),
            FakeStroke(50, 100),
        ]
        result = stroke_density_force(strokes, 0, 100)
        assert result > 0

    def test_dense_strokes_low_force(self) -> None:
        """密集笔 → 低力度。"""
        strokes = [
            FakeStroke(0, 10),
            FakeStroke(10, 20),
            FakeStroke(20, 30),
            FakeStroke(30, 40),
            FakeStroke(40, 50),
        ]
        result = stroke_density_force(strokes, 0, 50)
        assert result > 0

    def test_dense_less_than_sparse(self) -> None:
        """密集 < 稀疏。"""
        sparse_strokes = [FakeStroke(0, 50), FakeStroke(50, 100)]
        dense_strokes = [
            FakeStroke(0, 10), FakeStroke(10, 20),
            FakeStroke(20, 30), FakeStroke(30, 40),
            FakeStroke(40, 50), FakeStroke(50, 60),
            FakeStroke(60, 70), FakeStroke(70, 80),
            FakeStroke(80, 90), FakeStroke(90, 100),
        ]
        sparse_force = stroke_density_force(sparse_strokes, 0, 100)
        dense_force = stroke_density_force(dense_strokes, 0, 100)
        assert dense_force < sparse_force

    def test_invalid_range(self) -> None:
        strokes = [FakeStroke(0, 10)]
        assert stroke_density_force(strokes, 50, 10) == 0.0

    def test_no_strokes_in_range(self) -> None:
        strokes = [FakeStroke(0, 10)]
        result = stroke_density_force(strokes, 50, 100)
        assert result == pytest.approx(51.0)  # bar_span as fallback


class TestStrokeDensityFromSegments:
    """测试从线段估算笔密度。"""

    def test_basic_estimation(self) -> None:
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 30),
            FakeSegment("down", 9.0, 4.0, 30, 60),
        ]
        result = stroke_density_from_segments(segments, 0, 1)
        assert result > 0

    def test_more_segments_lower_force(self) -> None:
        """更多线段 → 更高笔密度估计 → 更低力度。"""
        few_segs = [
            FakeSegment("up", 10.0, 5.0, 0, 50),
        ]
        many_segs = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
            FakeSegment("up", 11.0, 6.0, 20, 30),
            FakeSegment("down", 10.0, 5.0, 30, 40),
            FakeSegment("up", 12.0, 7.0, 40, 50),
        ]
        few_force = stroke_density_from_segments(few_segs, 0, 0)
        many_force = stroke_density_from_segments(many_segs, 0, 4)
        assert many_force < few_force

    def test_invalid_range(self) -> None:
        segments = [FakeSegment("up", 10.0, 5.0, 0, 10)]
        assert stroke_density_from_segments(segments, 5, 0) == 0.0


# ── 方法 C: zhongshu_drift_force ─────────────────────────────────


class TestZhongshuDriftForce:
    """测试中枢偏移力度。"""

    def test_ascending_drift(self) -> None:
        """上升方向中枢偏移 → 正力度。"""
        zhongshus = [
            FakeZhongshu(zd=10.0, zg=15.0, dd=8.0, gg=17.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=18.0, zg=23.0, dd=16.0, gg=25.0, seg_start=3, seg_end=5),
        ]
        result = zhongshu_drift_force(zhongshus, [0, 1], "up")
        assert result > 0

    def test_descending_drift(self) -> None:
        """下跌方向中枢偏移 → 正力度。"""
        zhongshus = [
            FakeZhongshu(zd=18.0, zg=23.0, dd=16.0, gg=25.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=10.0, zg=15.0, dd=8.0, gg=17.0, seg_start=3, seg_end=5),
        ]
        result = zhongshu_drift_force(zhongshus, [0, 1], "down")
        assert result > 0

    def test_no_drift_wrong_direction(self) -> None:
        """偏移方向与走势方向不一致 → 0。"""
        zhongshus = [
            FakeZhongshu(zd=10.0, zg=15.0, dd=8.0, gg=17.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=18.0, zg=23.0, dd=16.0, gg=25.0, seg_start=3, seg_end=5),
        ]
        result = zhongshu_drift_force(zhongshus, [0, 1], "down")
        assert result == 0.0

    def test_decelerating_drift_is_divergence(self) -> None:
        """偏移减速 = 力度衰减 = 背驰信号。"""
        zhongshus = [
            FakeZhongshu(zd=10.0, zg=15.0, dd=8.0, gg=17.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=20.0, zg=25.0, dd=18.0, gg=27.0, seg_start=3, seg_end=5),
            FakeZhongshu(zd=27.0, zg=30.0, dd=25.0, gg=32.0, seg_start=6, seg_end=8),
        ]
        # A 段：中枢 0→1（大偏移）
        force_a = zhongshu_drift_force(zhongshus, [0, 1], "up")
        # C 段：中枢 1→2（小偏移）
        force_c = zhongshu_drift_force(zhongshus, [1, 2], "up")
        assert force_c < force_a  # 偏移减速 = 背驰

    def test_insufficient_zhongshus(self) -> None:
        zhongshus = [
            FakeZhongshu(zd=10.0, zg=15.0, dd=8.0, gg=17.0, seg_start=0, seg_end=2),
        ]
        assert zhongshu_drift_force(zhongshus, [0], "up") == 0.0
        assert zhongshu_drift_force(zhongshus, [], "up") == 0.0

    def test_three_zhongshu_average(self) -> None:
        """三中枢取平均偏移。"""
        zhongshus = [
            FakeZhongshu(zd=10.0, zg=20.0, dd=8.0, gg=22.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=25.0, zg=35.0, dd=23.0, gg=37.0, seg_start=3, seg_end=5),
            FakeZhongshu(zd=40.0, zg=50.0, dd=38.0, gg=52.0, seg_start=6, seg_end=8),
        ]
        result = zhongshu_drift_force(zhongshus, [0, 1, 2], "up")
        assert result > 0


# ── 复合力度 ──────────────────────────────────────────────────


class TestCompositeDirectionalForce:
    """测试复合方向性力度。"""

    def test_without_zhongshus(self) -> None:
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("up", 12.0, 6.0, 10, 20),
            FakeSegment("up", 15.0, 8.0, 20, 30),
        ]
        result = composite_directional_force(segments, 0, 2, "up")
        assert result > 0

    def test_with_zhongshus(self) -> None:
        segments = [
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
            FakeSegment("up", 12.0, 6.0, 20, 30),
            FakeSegment("down", 11.0, 5.0, 30, 40),
            FakeSegment("up", 15.0, 8.0, 40, 50),
        ]
        zhongshus = [
            FakeZhongshu(zd=5.0, zg=10.0, dd=4.0, gg=12.0, seg_start=0, seg_end=2),
            FakeZhongshu(zd=8.0, zg=13.0, dd=5.0, gg=15.0, seg_start=3, seg_end=4),
        ]
        result = composite_directional_force(
            segments, 0, 4, "up",
            zhongshus=zhongshus, zs_indices=[0, 1],
        )
        assert result > 0


# ── 力度对比 ──────────────────────────────────────────────────


class TestCompareForceMethodsAndDetect:
    """测试力度对比和背驰检测。"""

    def _make_strong_and_weak_segments(
        self,
    ) -> list[FakeSegment]:
        """构造 A 段（强）+ C 段（弱）的线段序列。"""
        return [
            # A 段 [0..2]: 3 段全 up, 大范围
            FakeSegment("up", 20.0, 5.0, 0, 30),
            FakeSegment("up", 25.0, 8.0, 30, 60),
            FakeSegment("up", 30.0, 10.0, 60, 90),
            # C 段 [3..5]: 交替方向, 小范围
            FakeSegment("up", 32.0, 28.0, 90, 100),
            FakeSegment("down", 31.0, 27.0, 100, 110),
            FakeSegment("up", 33.0, 29.0, 110, 120),
        ]

    def test_compare_persistence(self) -> None:
        segments = self._make_strong_and_weak_segments()
        result = compare_force_methods(
            segments, 0, 2, 3, 5, "up", method="persistence",
        )
        assert isinstance(result, ForceComparison)
        assert result.directional_force_a > result.directional_force_c
        assert result.directional_divergent is True

    def test_compare_density(self) -> None:
        segments = self._make_strong_and_weak_segments()
        result = compare_force_methods(
            segments, 0, 2, 3, 5, "up", method="density",
        )
        assert isinstance(result, ForceComparison)
        assert result.method == "density"

    def test_compare_composite(self) -> None:
        segments = self._make_strong_and_weak_segments()
        result = compare_force_methods(
            segments, 0, 2, 3, 5, "up", method="composite",
        )
        assert isinstance(result, ForceComparison)
        assert result.method == "composite"

    def test_detect_divergence(self) -> None:
        segments = self._make_strong_and_weak_segments()
        result = detect_directional_divergence(
            segments, 0, 2, 3, 5, "up", method="persistence",
        )
        assert isinstance(result, DirectionalDivergence)
        assert result.is_divergent is True
        assert result.method == "persistence"
        assert "amplitude_force_a" in result.comparison_with_amplitude

    def test_no_divergence_when_c_stronger(self) -> None:
        """C 段持续性 > A 段 → 非背驰。"""
        segments = [
            # A 段 [0..2]: 交替
            FakeSegment("up", 10.0, 5.0, 0, 10),
            FakeSegment("down", 9.0, 4.0, 10, 20),
            FakeSegment("up", 11.0, 6.0, 20, 30),
            # C 段 [3..5]: 全 up
            FakeSegment("up", 12.0, 7.0, 30, 50),
            FakeSegment("up", 14.0, 8.0, 50, 70),
            FakeSegment("up", 16.0, 9.0, 70, 90),
        ]
        result = detect_directional_divergence(
            segments, 0, 2, 3, 5, "up", method="persistence",
        )
        assert result.is_divergent is False


# ── 振幅力度基线 ──────────────────────────────────────────────


class TestAmplitudeForce:
    """测试振幅力度参照基线。"""

    def test_basic_amplitude(self) -> None:
        segments = [
            FakeSegment("up", 20.0, 5.0, 0, 30),
            FakeSegment("up", 25.0, 8.0, 30, 60),
        ]
        result = _amplitude_force(segments, 0, 1)
        expected = (25.0 - 5.0) * 60  # (high - low) * duration
        assert result == pytest.approx(expected)

    def test_invalid_range(self) -> None:
        segments = [FakeSegment("up", 10.0, 5.0, 0, 10)]
        assert _amplitude_force(segments, 5, 0) == 0.0


# ── 高级别力度（Move-as-component）──────────────────────────────


@dataclass(frozen=True)
class FakeMove:
    direction: Literal["up", "down"]
    high: float
    low: float
    seg_start: int
    seg_end: int


@dataclass(frozen=True)
class FakeLevelZhongshu:
    zd: float
    zg: float
    comp_start: int
    comp_end: int
    settled: bool = True


class TestLevelDirectionalPersistence:
    """测试高级别方向持续性。"""

    def test_all_same(self) -> None:
        components = [
            FakeMove("up", 10.0, 5.0, 0, 3),
            FakeMove("up", 15.0, 8.0, 4, 7),
            FakeMove("up", 20.0, 12.0, 8, 11),
        ]
        assert level_directional_persistence(components, 0, 2, "up") == pytest.approx(1.0)

    def test_mixed(self) -> None:
        components = [
            FakeMove("up", 10.0, 5.0, 0, 3),
            FakeMove("down", 9.0, 4.0, 4, 7),
            FakeMove("up", 12.0, 6.0, 8, 11),
        ]
        result = level_directional_persistence(components, 0, 2, "up")
        assert result == pytest.approx(2 / 3)

    def test_weighted_perfect(self) -> None:
        components = [
            FakeMove("up", 10.0, 5.0, 0, 3),
            FakeMove("up", 15.0, 8.0, 4, 7),
        ]
        assert level_directional_persistence_weighted(components, 0, 1, "up") == pytest.approx(1.0)


class TestLevelComponentDensity:
    """测试高级别组件密度力度。"""

    def test_sparse_high_force(self) -> None:
        components = [FakeMove("up", 20.0, 5.0, 0, 50)]
        result = level_component_density_force(components, 0, 0)
        assert result > 0

    def test_dense_lower_force(self) -> None:
        sparse = [FakeMove("up", 20.0, 5.0, 0, 50)]
        dense = [
            FakeMove("up", 10.0, 5.0, 0, 10),
            FakeMove("down", 9.0, 4.0, 11, 20),
            FakeMove("up", 12.0, 6.0, 21, 30),
            FakeMove("down", 11.0, 5.0, 31, 40),
            FakeMove("up", 15.0, 8.0, 41, 50),
        ]
        assert level_component_density_force(dense, 0, 4) < level_component_density_force(sparse, 0, 0)


class TestLevelZhongshuDrift:
    """测试高级别中枢偏移力度。"""

    def test_ascending(self) -> None:
        zs = [
            FakeLevelZhongshu(zd=10.0, zg=15.0, comp_start=0, comp_end=2),
            FakeLevelZhongshu(zd=18.0, zg=23.0, comp_start=3, comp_end=5),
        ]
        assert level_zhongshu_drift_force(zs, [0, 1], "up") > 0

    def test_descending(self) -> None:
        zs = [
            FakeLevelZhongshu(zd=18.0, zg=23.0, comp_start=0, comp_end=2),
            FakeLevelZhongshu(zd=10.0, zg=15.0, comp_start=3, comp_end=5),
        ]
        assert level_zhongshu_drift_force(zs, [0, 1], "down") > 0

    def test_deceleration_is_divergence(self) -> None:
        """偏移减速 = 力度衰减 = 背驰信号。"""
        zs = [
            FakeLevelZhongshu(zd=10.0, zg=15.0, comp_start=0, comp_end=2),
            FakeLevelZhongshu(zd=20.0, zg=25.0, comp_start=3, comp_end=5),
            FakeLevelZhongshu(zd=27.0, zg=30.0, comp_start=6, comp_end=8),
        ]
        force_a = level_zhongshu_drift_force(zs, [0, 1], "up")
        force_c = level_zhongshu_drift_force(zs, [1, 2], "up")
        assert force_c < force_a


class TestLevelDirectionalForce:
    """测试高级别复合方向性力度。"""

    def test_without_zhongshus(self) -> None:
        components = [
            FakeMove("up", 10.0, 5.0, 0, 5),
            FakeMove("up", 15.0, 8.0, 6, 11),
        ]
        result = level_directional_force(components, 0, 1, "up")
        assert result > 0

    def test_with_zhongshus(self) -> None:
        components = [
            FakeMove("up", 10.0, 5.0, 0, 5),
            FakeMove("down", 9.0, 4.0, 6, 11),
            FakeMove("up", 15.0, 8.0, 12, 17),
        ]
        zs = [
            FakeLevelZhongshu(zd=5.0, zg=10.0, comp_start=0, comp_end=2),
            FakeLevelZhongshu(zd=8.0, zg=13.0, comp_start=3, comp_end=5),
        ]
        result = level_directional_force(
            components, 0, 2, "up",
            zhongshus=zs, zs_indices=[0, 1],
        )
        assert result > 0


class TestCompareLevelForce:
    """测试高级别力度对比——T6 可达性的核心验证。"""

    def test_strong_a_weak_c_divergent(self) -> None:
        """A 段强（全同向 + 宽 seg_span），C 段弱（交替 + 窄 seg_span）→ 背驰。"""
        components = [
            # A 段 [0..2]: 全 up, 宽跨度
            FakeMove("up", 20.0, 5.0, 0, 20),
            FakeMove("up", 25.0, 8.0, 21, 40),
            FakeMove("up", 30.0, 10.0, 41, 60),
            # C 段 [3..5]: 交替, 窄跨度
            FakeMove("up", 32.0, 28.0, 61, 65),
            FakeMove("down", 31.0, 27.0, 66, 70),
            FakeMove("up", 33.0, 29.0, 71, 75),
        ]
        result = compare_level_force(components, 0, 2, 3, 5, "up")
        assert isinstance(result, LevelForceComparison)
        # 振幅力度判定背驰（幅度差异大）
        assert result.amplitude_divergent is True
        # 方向性力度也判定背驰（持续性衰减 + 密度增加）
        assert result.directional_divergent is True

    def test_amplitude_only_divergence(self) -> None:
        """A/C 段方向一致性相同且结构密度相同，但幅度差异大
        → 仅振幅力度判定背驰。

        这是 237号诊断的核心场景：方向信号未衰减，但幅度缩小。
        振幅力度（∈ ker(D)）报告背驰，方向性力度（∉ ker(D)）不报告。
        """
        components = [
            # A 段 [0..1]: 全 up, 大幅度, seg_span=41
            FakeMove("up", 100.0, 50.0, 0, 20),
            FakeMove("up", 120.0, 60.0, 21, 40),
            # C 段 [2..3]: 全 up, 小幅度但方向一致, seg_span=41
            FakeMove("up", 125.0, 120.0, 41, 61),
            FakeMove("up", 130.0, 123.0, 62, 81),
        ]
        result = compare_level_force(components, 0, 1, 2, 3, "up")
        # 振幅差异大：A=(120-50)*2=140, C=(130-120)*2=20 → 振幅背驰
        assert result.amplitude_divergent is True
        # 方向持续性相同（都是 100% up）且密度相同 → 方向性力度相同 → 非背驰
        assert result.directional_divergent is False
        # 两者不一致——这正是 ker(D) 问题的实证
        assert result.agreement is False
