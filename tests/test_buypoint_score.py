"""a_buypoint_score 测试 —— 五维归一化范围 + 综合分 + 边界 + 第5维诚实标注。"""

from __future__ import annotations

import random

import pytest

from newchan.a_buypoint_score import (
    BuyPointScore,
    ScorePolicy,
    containment_max_depth,
    score_buypoint_from_prices,
)
from newchan.a_online_persistence import OnlineMergeTree


@pytest.mark.unit
def test_dimensions_in_unit_range_and_composite_in_0_100():
    rng = random.Random(20260529)
    for _ in range(30):
        n = rng.randint(5, 60)
        closes = [round(rng.uniform(10.0, 40.0), 2) for _ in range(n)]
        s = score_buypoint_from_prices(closes)
        for d in s.dimensions:
            assert 0.0 <= d <= 1.0, (closes, s.dimensions)
        assert 0.0 <= s.composite_weighted <= 100.0
        assert 0.0 <= s.composite_product <= 100.0
        # 乘积 ≤ 加权（AM-GM）允许浮点容差
        assert s.composite_product <= s.composite_weighted + 1e-6


@pytest.mark.unit
def test_monotonic_decline_low_completion_and_unclean():
    """单调下跌：大级别未完成（结构完成度低）、alive 不干净、0 中枢。"""
    closes = [40.0 - i for i in range(20)]
    s = score_buypoint_from_prices(closes, highs=[c + 0.5 for c in closes],
                                   lows=[c - 0.5 for c in closes])
    assert s.zhongshu_count == 0
    assert s.structure_completion < 0.5
    # 全局分量恒 alive；单调下跌几乎无 settled → 干净度低
    assert s.alive_cleanliness < 1.0


@pytest.mark.unit
def test_both_modes_runnable():
    closes = [40, 30, 36, 28, 34, 27, 33, 26, 24, 28, 25, 30]
    closes = [float(c) for c in closes]
    sw = score_buypoint_from_prices(closes, policy=ScorePolicy(mode="weighted"))
    sp = score_buypoint_from_prices(closes, policy=ScorePolicy(mode="product"))
    # 两种 mode 都产出有效综合分（字段同时存在）
    assert isinstance(sw, BuyPointScore) and isinstance(sp, BuyPointScore)
    assert 0.0 <= sw.composite_weighted <= 100.0
    assert 0.0 <= sp.composite_product <= 100.0


@pytest.mark.unit
def test_amplitude_decay_zero_when_insufficient_settled():
    """settled 摆动 <2 → 振幅衰减=0 且解释标注非力度背驰（521号）。"""
    closes = [10.0, 5.0]  # 无 settled 完成摆动
    s = score_buypoint_from_prices(closes)
    assert s.amplitude_decay == 0.0
    assert any("521" in e and "力度背驰" in e for e in s.explanations)


@pytest.mark.unit
def test_amplitude_decay_is_morphological_not_dynamical_tag():
    """第5维解释必须显式标注：形态学振幅，非 MACD 力度背驰（521号约束）。"""
    closes = [40, 20, 35, 25, 33, 28, 31, 29]  # 多个递减摆动
    closes = [float(c) for c in closes]
    s = score_buypoint_from_prices(closes)
    decay_expl = [e for e in s.explanations if "振幅衰减" in e]
    assert decay_expl
    assert any("521" in e for e in decay_expl)
    # 不得把它命名/描述为 momentum/动力学背驰确认
    assert not any("力度背驰确认" in e for e in decay_expl)


@pytest.mark.unit
def test_containment_max_depth_known_structure():
    """嵌套价格区间 [20,10,18,11,17,12,19,5,8,2]：包含链 bar⊃bar⊃bar，深度≥2。"""
    closes = [20.0, 10.0, 18.0, 11.0, 17.0, 12.0, 19.0, 5.0, 8.0, 2.0]
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    bars = tree.finalize().settled_bars
    depth = containment_max_depth(bars)
    assert depth >= 2


@pytest.mark.unit
def test_empty_and_single():
    s0 = score_buypoint_from_prices([])
    assert s0.composite_weighted == 0.0 or s0.composite_weighted >= 0.0
    assert s0.zhongshu_count == 0
    s1 = score_buypoint_from_prices([5.0])
    for d in s1.dimensions:
        assert 0.0 <= d <= 1.0


@pytest.mark.unit
def test_containment_depth_empty():
    assert containment_max_depth(()) == 0
