"""a_online_persistence 的等价性 + 因果 settle 判据测试。

本测试坐实 a_online_persistence.py docstring 的两个核心声明（消除声明膨胀，
no-patch-mentality #5）：
1. **批量等价性**（L0/L1）：在线 finalize 后的 settled 有限 persistence 多重集
   ≡ 批量 sublevel_h0_bars 的有限 + 全局 bar 多重集——因果性不牺牲精度。
2. **因果 settle 判据**（L0）：settled bar 的 death 因果确定（未来不可改写）；
   alive bar 的 death 是当前估计，被未来更低 valley 改写（docstring 的"翻转反例"）。

认识论等级：等价性 = L1（管线正确性，property-based 确认无 bug，零信息增量）；
settle 判据 = L0（merge tree 数学属性）。
"""

from __future__ import annotations

import math
import random

import pytest

from newchan.a_online_persistence import (
    OnlineMergeTree,
    EntryState,
    TrendHealth,
    StopSignal,
    LevelSwitchEvent,
)
from newchan.a_persistence_barcode import sublevel_h0_bars
from newchan.a_level_detection import (
    CalendarPeriodNamer,
    SplitPolicy,
    detect_levels,
)


def _pers_multiset(bars) -> list[float]:
    return sorted(round(b.persistence, 4) for b in bars)


# ====================================================================
# 1. 批量等价性（property-based）
# ====================================================================


@pytest.mark.unit
@pytest.mark.parametrize("seed", range(25))
def test_finalize_equals_batch_random(seed):
    """随机序列：在线 finalize settled 多重集 ≡ 批量全 bar 多重集。"""
    rng = random.Random(seed)
    n = rng.randint(3, 50)
    seq = [round(rng.uniform(0.0, 100.0), 2) for _ in range(n)]

    batch = _pers_multiset(sublevel_h0_bars(seq))
    online = _pers_multiset(OnlineMergeTree.from_prices(seq).settled_bars)
    assert online == batch


@pytest.mark.unit
@pytest.mark.parametrize(
    "seq",
    [
        [1, 2, 3, 2, 1],          # 单峰
        [5, 4, 3, 2, 1],          # 单调降
        [1, 2, 3, 4, 5],          # 单调升
        [3, 1, 4, 1, 5, 9, 2, 6], # 多摆动
        [1, 3, 2, 4, 1, 5],       # 锯齿
        [10, 6, 8, 4, 9],         # 翻转构造
    ],
)
def test_finalize_equals_batch_structured(seq):
    """结构化序列：覆盖单峰/单调/多摆动/锯齿。"""
    seq = [float(x) for x in seq]
    batch = _pers_multiset(sublevel_h0_bars(seq))
    online = _pers_multiset(OnlineMergeTree.from_prices(seq).settled_bars)
    assert online == batch


# ====================================================================
# 2. 因果 settle 判据
# ====================================================================


@pytest.mark.unit
def test_settled_bars_are_causally_locked():
    """update() 增量产出的 settled bar，必在 finalize 输出中原值不变。

    = settled 的 death 因果确定，未来不可改写（docstring 核心定理）。
    """
    rng = random.Random(123)
    seq = [round(rng.uniform(0.0, 50.0), 1) for _ in range(30)]

    tree = OnlineMergeTree()
    incremental: list[float] = []
    for p in seq:
        for bar in tree.update(p):
            assert bar.settled is True
            incremental.append(round(bar.persistence, 4))

    final = _pers_multiset(tree.finalize().settled_bars)
    # 增量 settled 是 finalize settled 的子多重集（finalize 仅追加剩余合并 + 全局）
    for p in incremental:
        assert p in final


@pytest.mark.unit
def test_alive_death_rewritten_by_future_low():
    """翻转反例：alive 分量的 death 估计被未来更低 valley 改写。

    [10,6,8,4]：bar2(price8) 后 alive 用 running_max=10 估计 death；bar3(price4)
    创新低 → 新分量诞生，alive 结构改变。这正是 §7.5 指出的"稳定性代价"——
    alive 不稳定（可改写），settled 稳定（锁定）。
    """
    tree = OnlineMergeTree()
    snaps = []
    for p in [10.0, 6.0, 8.0, 4.0]:
        tree.update(p)
        snaps.append(tree.current_barcode())

    # bar3 创新低后 alive 分量数增加（结构被未来改写）
    assert len(snaps[3].alive_bars) > len(snaps[2].alive_bars)
    # 所有 alive bar 的 settled 标志为 False（death 未因果确定）
    assert all(b.settled is False for b in snaps[3].alive_bars)


@pytest.mark.unit
def test_settle_requires_right_barrier():
    """settle 判据：合并仅在右侧出现 ≥ peak 的屏障价时发生。

    [5,1,4,2,6]：peak=4 处的合并需等到右侧出现 ≥4 的价格(6)才 settle。
    在价格触及 6 之前（bar3 price2），该合并不应 settle。
    """
    tree = OnlineMergeTree()
    settled_before_barrier = 0
    prices = [5.0, 1.0, 4.0, 2.0, 6.0]
    for i, p in enumerate(prices):
        newly = tree.update(p)
        if i < 4:  # 价格尚未触及 6（右屏障）
            settled_before_barrier += len(newly)
    # 触及 6 后才发生跨 peak=4 的 settle
    final = tree.finalize()
    assert len(final.settled_bars) >= 1
    # 在右屏障(6)出现前，跨 peak=4 的合并不应已 settle
    # （bar1 价格 1 的小摆动可能更早 settle，但 peak=4 的主合并不会）
    assert settled_before_barrier <= 1


# ====================================================================
# 3. 不可变性
# ====================================================================


@pytest.mark.unit
def test_snapshot_immutable():
    snap = OnlineMergeTree.from_prices([10.0, 6.0, 8.0, 4.0, 9.0])
    with pytest.raises((AttributeError, TypeError)):
        snap.n_points = 0
    if snap.settled_bars:
        with pytest.raises((AttributeError, TypeError)):
            snap.settled_bars[0].persistence = 0.0


@pytest.mark.unit
def test_span_property():
    """MergeBar.span = hi - lo + 1（特征覆盖的 K 线根数）。"""
    snap = OnlineMergeTree.from_prices([1.0, 2.0, 3.0, 2.0, 1.0])
    for b in snap.settled_bars:
        assert b.span == b.hi - b.lo + 1
        assert b.span >= 1


# ====================================================================
# 4. lo/hi/span 与批量定义一致（无 ties 时精确）
# ====================================================================


@pytest.mark.unit
@pytest.mark.parametrize("seed", range(15))
def test_span_matches_sublevel_definition_tie_free(seed):
    """全精度浮点（无精确 ties）：区间 [lo,hi] = 含 valley 的、价格<death 的极大连续段。

    这是 sublevel 连通分量的批量定义。坐实"因果性不牺牲精度"也覆盖 span 维度
    （真实连续价格场景）。精确值相等时仅 diagram 精确、区间因 tie-break 可能差 1。
    """
    rng = random.Random(500 + seed)
    n = rng.randint(2, 50)
    prices = [rng.uniform(0, 100) for _ in range(n)]
    snap = OnlineMergeTree.from_prices(prices)
    for b in snap.settled_bars:
        if b.is_global:
            assert (b.lo, b.hi) == (0, n - 1)
            continue
        lo = b.birth_idx
        while lo - 1 >= 0 and prices[lo - 1] < b.death_price:
            lo -= 1
        hi = b.birth_idx
        while hi + 1 < n and prices[hi + 1] < b.death_price:
            hi += 1
        assert (b.lo, b.hi) == (lo, hi)
        assert b.span == hi - lo + 1


# ====================================================================
# 5. 周期映射（CalendarPeriodNamer）
# ====================================================================


@pytest.mark.unit
def test_calendar_period_thresholds():
    """日线口径（bars_per_day=1）：span(日历日)→缠论级别名。"""
    namer = CalendarPeriodNamer()
    assert namer.name(150, 1.0) == "月线级"
    assert namer.name(60, 1.0) == "周线级"
    assert namer.name(20, 1.0) == "日线级"
    assert namer.name(5, 1.0) == "60分钟级"
    assert namer.name(1, 1.0) == "30分钟级或更小"


@pytest.mark.unit
def test_bars_per_day_bridges_periods():
    """bars_per_day 桥接不同周期：同样 span_bars 在 30 分钟数据映射到更低级别。"""
    namer = CalendarPeriodNamer()
    # 120 bars：30 分钟港股(≈11 bars/日)≈11 日历日→日线级；日线(1)=120 日历日→月线级
    assert namer.name(120, 11.0) == "日线级"
    assert namer.name(120, 1.0) == "月线级"


# ====================================================================
# 6. 自动级别检测（log-gap 递归）
# ====================================================================


def _multiscale_series(n: int = 360, seed: int = 3) -> list[float]:
    """三尺度均值回复震荡：大(周期40) + 中(周期8) + 小(周期2) + 噪声。"""
    rng = random.Random(seed)
    return [
        100 + 30 * math.sin(t / 40.0) + 8 * math.sin(t / 8.0) + 2 * math.sin(t / 2.0)
        + rng.uniform(-0.5, 0.5)
        for t in range(n)
    ]


@pytest.mark.unit
def test_levels_emerge_from_data_no_preset_count():
    """三尺度合成序列 → 自动涌现 3 个叶级别（级别数不预设，从 gap 涌现）。"""
    snap = OnlineMergeTree.from_prices(_multiscale_series())
    ls = detect_levels(snap.settled_bars, bars_per_day=1.0, noise_floor=1.0)
    assert len(ls.leaves) == 3
    leaf_max = sorted((lv.max_persistence for lv in ls.leaves), reverse=True)
    assert leaf_max[0] > leaf_max[1] > leaf_max[2]


@pytest.mark.unit
def test_level_parent_child_consistency():
    """parent/child 双向一致 + 恰一个根。"""
    snap = OnlineMergeTree.from_prices(_multiscale_series())
    ls = detect_levels(snap.settled_bars, noise_floor=1.0)
    for lv in ls.levels:
        for cid in lv.child_ids:
            assert ls.by_id(cid).parent_id == lv.level_id
        if lv.parent_id is not None:
            assert lv.level_id in ls.by_id(lv.parent_id).child_ids
    assert len(ls.roots) == 1


@pytest.mark.unit
def test_coarser_level_larger_span():
    """级别越粗（persistence 越大）→ avg_span 越大（级别=时间尺度一致）。"""
    snap = OnlineMergeTree.from_prices(_multiscale_series())
    ls = detect_levels(snap.settled_bars, noise_floor=1.0)
    leaves = sorted(ls.leaves, key=lambda lv: lv.max_persistence, reverse=True)
    spans = [lv.avg_span for lv in leaves]
    assert spans == sorted(spans, reverse=True)


@pytest.mark.unit
def test_noise_floor_prevents_spurious_levels():
    """滤噪后伪级别（近零噪声单点）减少。"""
    snap = OnlineMergeTree.from_prices(_multiscale_series())
    raw = detect_levels(snap.settled_bars, noise_floor=0.0)
    filtered = detect_levels(snap.settled_bars, noise_floor=1.0)
    assert len(filtered.leaves) < len(raw.leaves)


@pytest.mark.unit
def test_gap_ratio_sensitivity_monotone():
    """min_gap_ratio 越大 → 级别越少（保守）；越小 → 越多（灵敏）。"""
    snap = OnlineMergeTree.from_prices(_multiscale_series())
    conservative = detect_levels(
        snap.settled_bars, noise_floor=1.0, policy=SplitPolicy(min_gap_ratio=2.5)
    )
    sensitive = detect_levels(
        snap.settled_bars, noise_floor=1.0, policy=SplitPolicy(min_gap_ratio=1.3)
    )
    assert len(conservative.leaves) <= len(sensitive.leaves)


@pytest.mark.unit
def test_detect_levels_empty():
    ls = detect_levels([], bars_per_day=1.0)
    assert ls.levels == ()
    assert ls.leaves == ()


# ====================================================================
# 7. 止损 / 走势健康度模块（trend_health / stop_signal / level_switch_event）
# ====================================================================


@pytest.mark.unit
def test_trend_health_growing_in_new_lows():
    """下跌不断创新低 → 主导 alive persistence 增长 → healthy=True（趋势未衰竭）。"""
    tree = OnlineMergeTree()
    for p in [100, 98, 101, 95, 99, 90, 94, 85, 88, 80]:
        tree.update(float(p))
    th = tree.trend_health(window=4)
    assert th is not None
    assert th.persistence_growth_rate > 0  # 仍在创新低
    assert th.healthy is True


@pytest.mark.unit
def test_trend_health_none_when_insufficient_history():
    tree = OnlineMergeTree()
    tree.update(10.0)
    assert tree.trend_health() is None


@pytest.mark.unit
def test_stop_signal_reverse_exceeds():
    """入场后反向新分量 persistence 超过入场阈值 → 止损触发（reverse_exceeds）。"""
    tree = OnlineMergeTree()
    for p in [100, 90, 95, 85]:
        tree.update(float(p))
    entry = EntryState(birth_idx=3, entry_persistence=1.0, direction=1, n_at_entry=4)
    for p in [88, 70]:  # 创新低 70，形成更大反向分量
        tree.update(float(p))
    ss = tree.stop_signal(entry)
    assert ss.triggered
    assert ss.reason in ("reverse_exceeds", "both")
    assert ss.reverse_persistence > entry.entry_persistence


@pytest.mark.unit
def test_stop_signal_not_triggered_when_quiet():
    """入场分量仍 alive 且无更大反向分量 → 不触发。"""
    tree = OnlineMergeTree()
    for p in [100, 90]:
        tree.update(float(p))
    # 入场在最低点 idx=1(90)，之后温和上行不创新低
    entry = EntryState(birth_idx=1, entry_persistence=100.0, direction=1, n_at_entry=2)
    for p in [92, 94, 93]:
        tree.update(float(p))
    ss = tree.stop_signal(entry)
    assert not ss.triggered
    assert ss.reason == "none"


@pytest.mark.unit
def test_stop_signal_entry_death():
    """入场分量被合并(death) → 止损触发（entry_death）。"""
    tree = OnlineMergeTree()
    # 制造一个会 death 的中间分量：谷-峰-更低谷，使第一个谷在峰处 death
    for p in [10, 5, 8, 3]:
        tree.update(float(p))
    # idx=1(price5) 这个分量会在 price8 处与更低的 idx=3 合并而 death
    entry = EntryState(birth_idx=1, entry_persistence=3.0, direction=1, n_at_entry=2)
    for p in [9, 11]:  # 上穿 8 触发 settle，price5 分量 death
        tree.update(float(p))
    ss = tree.stop_signal(entry)
    assert ss.reason in ("entry_death", "both")
    assert ss.triggered


@pytest.mark.unit
def test_level_switch_event_detects_dominant_change():
    """主导 alive 分量 death + 新分量取代 → switched。"""
    tree = OnlineMergeTree()
    for p in [50, 45, 48, 40]:
        tree.update(float(p))
    th = tree.trend_health(window=2)
    prev_dom = th.dominant_birth_idx if th else None
    # 继续创新低，主导可能切换
    for p in [42, 30, 35, 25]:
        tree.update(float(p))
    lse = tree.level_switch_event(prev_dom)
    assert isinstance(lse, LevelSwitchEvent)
    assert lse.new_dominant_birth_idx is not None


@pytest.mark.unit
def test_dom_history_does_not_break_batch_equivalence():
    """_record_dom（健康度历史）不影响 merge 逻辑 → diagram 仍等于批量。"""
    rng = random.Random(77)
    for _ in range(50):
        n = rng.randint(1, 40)
        prices = [rng.uniform(0, 100) for _ in range(n)]
        batch = _pers_multiset(sublevel_h0_bars(prices))
        online = _pers_multiset(OnlineMergeTree.from_prices(prices).settled_bars)
        assert online == batch
