"""a_level_detection 的 L0 算法锁定测试。

锁定 log-gap 递归级别检测的纯算法行为（§4/§7.3 横切自动化）。
经验断言（log-gap 边界 = 缠论级别边界）停留 L1，本测试不验证。

认识论等级：log-gap 检测 L0；本测试锁定 L0 行为 + L1 管线正确性。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_level_detection import (
    CalendarPeriodNamer,
    SplitPolicy,
    detect_levels,
)
from newchan.a_online_persistence import MergeBar


def _bar(pers: float, span: int = 1) -> MergeBar:
    """构造一个 persistence=pers、span=span 的 settled MergeBar。"""
    return MergeBar(
        birth_price=0.0,
        death_price=pers,
        persistence=pers,
        birth_idx=0,
        death_idx=1,
        lo=0,
        hi=span - 1,
        settled=True,
    )


@pytest.mark.unit
def test_calendar_namer_thresholds():
    namer = CalendarPeriodNamer()
    assert namer.name(1.0, 1.0) == "30分钟级或更小"
    assert namer.name(5.0, 1.0) == "60分钟级"
    assert namer.name(20.0, 1.0) == "日线级"
    assert namer.name(60.0, 1.0) == "周线级"
    assert namer.name(200.0, 1.0) == "月线级"
    # bars_per_day 桥接：30 分钟港股 11 根/日 → 110 根 = 10 日历日 = 60分钟级边界
    assert namer.name(110.0, 11.0) == namer.name(10.0, 1.0)


@pytest.mark.unit
def test_two_clear_clusters_split():
    """两个对数尺度分离的簇 → 检测出根 + 2 子级别。"""
    bars = [_bar(p, span=100) for p in (240.0, 238.0)] + [
        _bar(p, span=10) for p in (15.0, 14.0, 13.0, 12.0)
    ]
    ls = detect_levels(bars, bars_per_day=1.0, noise_floor=0.0)
    assert len(ls.roots) == 1
    assert len(ls.leaves) == 2
    # 粗簇 persistence 区间在细簇之上
    coarse, fine = ls.leaves
    assert min(coarse.persistence_range) > max(fine.persistence_range) or \
        min(fine.persistence_range) > max(coarse.persistence_range)


@pytest.mark.unit
def test_single_cluster_no_split():
    """persistence 紧凑（无显著 log-gap）→ 不分裂，单一级别。"""
    bars = [_bar(p) for p in (10.0, 10.5, 11.0, 10.2, 9.8)]
    ls = detect_levels(bars, noise_floor=0.0)
    assert len(ls.levels) == 1
    assert ls.levels[0].parent_id is None


@pytest.mark.unit
def test_noise_floor_filters():
    """noise_floor 滤掉低 persistence 特征，不参与级别检测。"""
    bars = [_bar(100.0, span=50), _bar(2.0), _bar(1.0)]
    ls = detect_levels(bars, noise_floor=5.0)
    # 只剩 1 个超过 τ 的特征 → 单级别
    assert all(lv.persistence_range[0] > 5.0 for lv in ls.levels)
    assert ls.levels[0].n_bars == 1


@pytest.mark.unit
def test_max_levels_limits_growth():
    """max_levels 有限制效果：更小的 max_levels 产出更少（或相等）级别。

    这是 max_levels 的 **可验证正向行为**（guard 起作用）。注意它**不是**严格的
    总数上界——见下方 xfail 记录的 spec-execution-gap。
    """
    bars = [_bar(2.0 ** k, span=2 ** k) for k in range(12)]
    base = SplitPolicy(min_gap_ratio=1.5, min_bars_per_level=1, max_levels=8)
    tight = SplitPolicy(min_gap_ratio=1.5, min_bars_per_level=1, max_levels=2)
    n_base = len(detect_levels(bars, policy=base, noise_floor=0.0).levels)
    n_tight = len(detect_levels(bars, policy=tight, noise_floor=0.0).levels)
    assert n_tight <= n_base


@pytest.mark.unit
@pytest.mark.xfail(
    reason="spec-execution-gap（待编排者裁决，2026-05-27）：max_levels 当前是软性分裂闸，"
    "在分裂点检查 current_level_count，但二叉递归一次分裂原子承诺 2 子级别，已承诺子树"
    "仍完成 → 总数可溢出（max_levels=4 实测 7）。严格总数上界需破坏分裂原子性或事后剪枝；"
    "深度上界则干净但与参数名'levels'(总数)冲突。语义(总数 vs 深度)有真实权衡，编排者"
    "答'我不知道'未裁决 → 不猜测性 patch，软闸+本 xfail 诚实记录缺口。",
    strict=True,
)
def test_max_levels_is_strict_total_bound():
    """若 max_levels 为严格总数上界则应成立——当前是软闸，故 xfail（已上浮）。"""
    bars = [_bar(2.0 ** k, span=2 ** k) for k in range(12)]
    policy = SplitPolicy(min_gap_ratio=1.5, min_bars_per_level=1, max_levels=4)
    ls = detect_levels(bars, policy=policy, noise_floor=0.0)
    assert len(ls.levels) <= 4


@pytest.mark.unit
def test_split_policy_gap_ratio():
    """should_split：log_gap 须 >= ln(min_gap_ratio) 才分裂。"""
    policy = SplitPolicy(min_gap_ratio=2.0, min_bars_per_level=1, max_levels=8)
    assert policy.should_split(math.log(2.0), group_size=4, current_level_count=1) is True
    assert policy.should_split(math.log(1.5), group_size=4, current_level_count=1) is False


@pytest.mark.unit
def test_structure_immutable():
    ls = detect_levels([_bar(100.0, 50), _bar(10.0)], noise_floor=0.0)
    with pytest.raises((AttributeError, TypeError)):
        ls.bars_per_day = 0.0
    with pytest.raises((AttributeError, TypeError)):
        ls.levels[0].level_id = 99


@pytest.mark.unit
def test_empty_and_degenerate():
    assert len(detect_levels([], noise_floor=0.0).levels) == 0
    # 全部低于噪声 → 空
    assert len(detect_levels([_bar(1.0), _bar(2.0)], noise_floor=10.0).levels) == 0
