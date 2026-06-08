"""a_settle_trigger 测试 —— settle 屏障价的因果正确性（证伪式 TDD）。

核心测试不是"通过"，而是把"我算的 settle_price 是真实 merge tree settle 点"**证伪**：
喂 settle_price−ε 必须未 settle，喂 settle_price 必须使该分量进入 settled_bars，
且 realized persistence == 预测 settle_persistence。
"""

from __future__ import annotations

import random

import pytest

from newchan.a_online_persistence import OnlineMergeTree
from newchan.a_settle_trigger import (
    SettleTrigger,
    dominant_trigger,
    nearest_rebound_settle,
    settle_triggers,
    settle_triggers_from_prices,
)


def _feed(prices):
    tree = OnlineMergeTree()
    for p in prices:
        tree.update(p)
    return tree


# --------------------------------------------------------------------
# 核心：settle_price 的因果正确性（证伪）
# --------------------------------------------------------------------


@pytest.mark.unit
def test_two_component_settle_price_matches_real_merge():
    """[10,2,6,4]：较浅分量(valley=4,idx=3)的 settle_price 应为 6；
    喂 5.99 不 settle，喂 6.0 必 settle，realized persistence==2。"""
    prices = [10.0, 2.0, 6.0, 4.0]
    triggers = settle_triggers_from_prices(prices)
    by_idx = {t.birth_idx: t for t in triggers}

    shallow = by_idx[3]
    assert shallow.can_settle_by_rebound is True
    assert shallow.settle_price == pytest.approx(6.0)
    assert shallow.settle_persistence == pytest.approx(2.0)
    assert shallow.gap_to_settle == pytest.approx(6.0 - 4.0)  # last_price=4

    # 证伪 1：喂 5.99（< 屏障）→ idx=3 分量不得进入 settled
    tree = _feed(prices)
    tree.update(5.99)
    settled_idxs = {b.birth_idx for b in tree.current_barcode().settled_bars}
    assert 3 not in settled_idxs

    # 证伪 2：喂 6.0（= 屏障）→ idx=3 分量必须 settle，persistence==2
    tree2 = _feed(prices)
    newly = tree2.update(6.0)
    newly_idxs = {b.birth_idx for b in newly}
    assert 3 in newly_idxs
    bar3 = next(b for b in newly if b.birth_idx == 3)
    assert bar3.persistence == pytest.approx(2.0)
    assert bar3.death_price == pytest.approx(6.0)


@pytest.mark.unit
def test_global_component_cannot_settle_by_rebound():
    """[10,2,6,4]：全局最低分量(valley=2,idx=1)不可反弹 settle，且为 dominant。"""
    triggers = settle_triggers_from_prices([10.0, 2.0, 6.0, 4.0])
    by_idx = {t.birth_idx: t for t in triggers}
    glob = by_idx[1]
    assert glob.can_settle_by_rebound is False
    assert glob.settle_price is None
    assert glob.settle_persistence is None
    assert glob.gap_to_settle is None
    assert glob.is_dominant is True
    # cap=10, val=2 → 反向否定幅度 = 8
    assert glob.reversal_amplitude == pytest.approx(8.0)
    assert glob.current_persistence == pytest.approx(8.0)


@pytest.mark.unit
def test_monotonic_decline_has_no_rebound_settle():
    """单调下跌（无反弹）→ 仅一个全局 alive 分量，无可反弹 settle。"""
    triggers = settle_triggers_from_prices([10.0, 9.0, 8.0, 7.0, 6.0])
    assert len(triggers) == 1
    assert triggers[0].can_settle_by_rebound is False
    assert nearest_rebound_settle(triggers) is None
    dom = dominant_trigger(triggers)
    assert dom is not None and dom.can_settle_by_rebound is False


@pytest.mark.unit
def test_nearest_settle_is_smallest_barrier():
    """多个可 settle 分量时，nearest = settle_price 最小者（最先被反弹触及）。"""
    # 构造两个较浅分量：深谷 + 两个递减屏障的较高谷
    # [12,1,10,3,8,5]：valley 1(global), 3, 5；屏障 10、8（严格递减）
    prices = [12.0, 1.0, 10.0, 3.0, 8.0, 5.0]
    triggers = settle_triggers_from_prices(prices)
    settleable = [t for t in triggers if t.can_settle_by_rebound]
    assert len(settleable) >= 1
    nearest = nearest_rebound_settle(triggers)
    assert nearest is not None
    assert nearest.settle_price == min(t.settle_price for t in settleable)
    # 证伪：喂 nearest.settle_price 应使该分量 settle
    tree = _feed(prices)
    newly = tree.update(nearest.settle_price)
    assert nearest.birth_idx in {b.birth_idx for b in newly}


@pytest.mark.unit
def test_settle_persistence_equals_realized_for_all_settleable():
    """property：对随机序列，每个可 settle 分量的预测 persistence == 实际 settle 后 persistence。"""
    rng = random.Random(20260529)
    for _ in range(40):
        n = rng.randint(4, 30)
        prices = [round(rng.uniform(1.0, 100.0), 2) for _ in range(n)]
        triggers = settle_triggers_from_prices(prices)
        for t in triggers:
            if not t.can_settle_by_rebound:
                continue
            assert t.settle_price is not None and t.settle_persistence is not None
            # 喂 settle_price，验证该分量进入 settled 且 persistence 匹配
            tree = _feed(prices)
            newly = tree.update(t.settle_price)
            # 该分量本次或之前已 settle（cascade 可能一次触发多个）
            all_settled = {b.birth_idx: b for b in tree.current_barcode().settled_bars}
            assert t.birth_idx in all_settled, (
                f"prices={prices} birth_idx={t.birth_idx} settle_price={t.settle_price}"
            )
            assert all_settled[t.birth_idx].persistence == pytest.approx(
                t.settle_persistence
            )


@pytest.mark.unit
def test_just_below_barrier_does_not_settle_property():
    """property：喂 settle_price 的略小值（向低 0.5%）不应使该分量 settle。"""
    rng = random.Random(424242)
    for _ in range(40):
        n = rng.randint(4, 30)
        prices = [round(rng.uniform(1.0, 100.0), 2) for _ in range(n)]
        triggers = settle_triggers_from_prices(prices)
        for t in triggers:
            if not t.can_settle_by_rebound or t.settle_price is None:
                continue
            below = t.settle_price - max(1e-6, abs(t.settle_price) * 0.005)
            if below <= t.birth_price:
                continue  # 屏障与谷太近，跳过
            tree = _feed(prices)
            tree.update(below)
            settled_idxs = {b.birth_idx for b in tree.current_barcode().settled_bars}
            assert t.birth_idx not in settled_idxs, (
                f"prices={prices} idx={t.birth_idx} below={below} barrier={t.settle_price}"
            )


# --------------------------------------------------------------------
# 边界
# --------------------------------------------------------------------


@pytest.mark.unit
def test_empty_and_single():
    assert settle_triggers_from_prices([]) == ()
    one = settle_triggers_from_prices([5.0])
    assert len(one) == 1
    assert one[0].can_settle_by_rebound is False
    assert one[0].is_dominant is True
    assert one[0].birth_price == pytest.approx(5.0)


@pytest.mark.unit
def test_settle_triggers_after_finalize_residual_global_only():
    """finalize 把栈合并到只剩全局分量（不清空栈，是流式累积器的实现事实）。

    诚实记录真实行为：finalize 后调 settle_triggers 仍读到残留的全局分量
    （1 个，can_settle=False）——这正是模块 docstring"必须在 finalize 之前调用"
    的原因。不伪造为空（那是补丁思维）。
    """
    tree = _feed([10.0, 2.0, 6.0, 4.0])
    tree.finalize()
    residual = settle_triggers(tree)
    assert len(residual) == 1
    assert residual[0].can_settle_by_rebound is False
    assert residual[0].birth_price == pytest.approx(2.0)  # 全局最低 valley
