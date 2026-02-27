"""反向传递测试。"""

from __future__ import annotations

import pytest

from newchan.nesting.bsp import DivergenceType
from newchan.nesting.reverse_propagation import (
    DivergenceRecord,
    div_count,
    exhaustion_check,
    threshold_lower_bound,
)


def _make_records(n: int, div_type: DivergenceType, level: int = 1, time: float = 100.0) -> list[DivergenceRecord]:
    """创建 n 条背驰记录。"""
    return [
        DivergenceRecord(entity_id=f"s{i}", level=level, time=time, divergence_type=div_type)
        for i in range(n)
    ]


class TestDivCount:
    def test_all_divergent(self):
        records = _make_records(5, DivergenceType.TOP_DIV)
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0) == 5

    def test_none_divergent(self):
        records = _make_records(5, DivergenceType.NONE)
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0) == 0

    def test_mixed(self):
        records = (
            _make_records(3, DivergenceType.TOP_DIV)
            + _make_records(2, DivergenceType.BOT_DIV)
        )
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0) == 3
        assert div_count(records, DivergenceType.BOT_DIV, level=1, time=100.0) == 2

    def test_level_filter(self):
        records = _make_records(3, DivergenceType.TOP_DIV, level=2)
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0) == 0
        assert div_count(records, DivergenceType.TOP_DIV, level=2, time=100.0) == 3

    def test_time_tolerance(self):
        records = [
            DivergenceRecord(entity_id="s0", level=1, time=100.0, divergence_type=DivergenceType.TOP_DIV),
            DivergenceRecord(entity_id="s1", level=1, time=102.0, divergence_type=DivergenceType.TOP_DIV),
            DivergenceRecord(entity_id="s2", level=1, time=110.0, divergence_type=DivergenceType.TOP_DIV),
        ]
        # tolerance=0 -> 只匹配 time=100.0
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0, time_tolerance=0.0) == 1
        # tolerance=3 -> 匹配 100.0 和 102.0
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0, time_tolerance=3.0) == 2
        # tolerance=15 -> 全部匹配
        assert div_count(records, DivergenceType.TOP_DIV, level=1, time=100.0, time_tolerance=15.0) == 3


class TestExhaustionCheck:
    def test_above_threshold(self):
        """§3.1: DivCount/n >= theta 时力度衰竭。"""
        # 6/10 >= 0.5
        records = (
            _make_records(6, DivergenceType.TOP_DIV)
            + _make_records(4, DivergenceType.NONE)
        )
        assert exhaustion_check(records, DivergenceType.TOP_DIV, level=1, time=100.0, threshold=0.5)

    def test_below_threshold(self):
        # 4/10 < 0.5
        records = (
            _make_records(4, DivergenceType.TOP_DIV)
            + _make_records(6, DivergenceType.NONE)
        )
        assert not exhaustion_check(records, DivergenceType.TOP_DIV, level=1, time=100.0, threshold=0.5)

    def test_exact_threshold(self):
        # 5/10 = 0.5 -> >= 0.5 -> True
        records = (
            _make_records(5, DivergenceType.TOP_DIV)
            + _make_records(5, DivergenceType.NONE)
        )
        assert exhaustion_check(records, DivergenceType.TOP_DIV, level=1, time=100.0, threshold=0.5)

    def test_empty_records(self):
        assert not exhaustion_check([], DivergenceType.TOP_DIV, level=1, time=100.0, threshold=0.5)


class TestThresholdLowerBound:
    def test_equal_weights(self):
        """等权情况：theta > 1/2。"""
        weights = [1.0, 1.0, 1.0, 1.0]
        bound = threshold_lower_bound(weights)
        assert bound > 0.5

    def test_single_dominant(self):
        """一个标的占 >50% 权重。"""
        weights = [0.6, 0.1, 0.1, 0.1, 0.1]
        bound = threshold_lower_bound(weights)
        # 最大权重 0.6 > 0.5 -> 只需 1/5 = 0.2
        assert bound == pytest.approx(0.2)

    def test_empty_weights(self):
        assert threshold_lower_bound([]) == 1.0

    def test_two_equal(self):
        weights = [1.0, 1.0]
        bound = threshold_lower_bound(weights)
        # 每个归一化权重 = 0.5，单个不满足 > 0.5，需要 2 个 -> 2/2 = 1.0
        assert bound == pytest.approx(1.0)

    def test_three_equal(self):
        weights = [1.0, 1.0, 1.0]
        bound = threshold_lower_bound(weights)
        # 每个 1/3，需要 2 个超过 0.5 -> 2/3
        assert bound == pytest.approx(2.0 / 3.0)
