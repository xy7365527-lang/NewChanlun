"""a_dual_merge_tree 测试 —— 双树法 K 线 PH 引擎（证伪式 TDD）。

设计同构（与 a_settle_trigger 测试一脉）：
- T_low（喂 low）的 settle 语义 == 单树法（喂 close）：底被反弹**涨过**屏障确认。
  → 用已验证的 [10,2,6,4] 底例直接复用，property 比对 settle_triggers_from_prices。
- T_high（内部喂 -high）的 settle 语义是镜像：顶被**跌破**屏障杀死。
  → 用 [0,8,4,6]（= 10−[10,2,6,4]）顶例，手算 valley/屏障与底例对称。

关键证伪点（不是"通过"，而是把取负还原与方向翻转钉死）：
1. alive_components_high() 报真实价（peak），不报 -peak。
2. T_high settle trigger_price < extreme_price（跌破方向），喂该 high 使顶 settle。
3. persistence 在取负下不变 = 真实下跌幅度。
"""

from __future__ import annotations

import random

import pytest

from newchan.a_dual_merge_tree import (
    DualComponent,
    DualMergeTree,
    DualSettleTrigger,
    StrokeCandidate,
)
from newchan.a_settle_trigger import settle_triggers_from_prices


# --------------------------------------------------------------------
# 构造辅助：喂 (high, low) 对
# --------------------------------------------------------------------


def _feed(highs, lows):
    dt = DualMergeTree()
    for h, l in zip(highs, lows):
        dt.update(h, l)
    return dt


# 镜像底例 [10,2,6,4] → 顶例 [0,8,4,6]（= 10 − low）。
# highs=[0,8,4,6]: -high=[0,-8,-4,-6]
#   global 顶 @idx1 peak=8（-high valley -8，全局最低）
#   次顶   @idx3 peak=6（-high valley -6），屏障 -4 @idx2（high trough=4）
#   → 次顶 settle ⟺ -high 涨过 -4 ⟺ high 跌破 4。
TOP_HIGHS = [0.0, 8.0, 4.0, 6.0]
TOP_LOWS = [-1.0, 7.0, 3.0, 5.0]  # 任意平行序列；顶测试只断言 high 侧


# --------------------------------------------------------------------
# 1. 取负还原：alive 分量报真实价
# --------------------------------------------------------------------


@pytest.mark.unit
def test_alive_high_components_report_real_peak_not_negated():
    """T_high alive 分量 extreme_price 必须是真实 high peak（正数 8/6），不是 -8/-6。"""
    dt = _feed(TOP_HIGHS, TOP_LOWS)
    comps = dt.alive_components_high()
    peaks = sorted(c.extreme_price for c in comps)
    assert peaks == pytest.approx([6.0, 8.0])
    # 全部为 'top'，且价为正（已还原）
    assert all(c.kind == "top" for c in comps)
    assert all(c.extreme_price > 0 for c in comps)


@pytest.mark.unit
def test_alive_low_components_report_real_valley():
    """T_low alive 分量 == 直接喂 low 的语义：valley=底真实价。"""
    dt = _feed([11.0, 3.0, 7.0, 5.0], [10.0, 2.0, 6.0, 4.0])
    comps = dt.alive_components_low()
    valleys = sorted(c.extreme_price for c in comps)
    assert valleys == pytest.approx([2.0, 4.0])
    assert all(c.kind == "bottom" for c in comps)


# --------------------------------------------------------------------
# 2. settle 方向：顶=跌破，底=涨过
# --------------------------------------------------------------------


@pytest.mark.unit
def test_top_settle_trigger_is_below_peak_a_breakdown():
    """次顶 peak=6 的 trigger_price=4（跌破 4 确认顶死），4 < 6（跌破方向）。"""
    dt = _feed(TOP_HIGHS, TOP_LOWS)
    trigs = dt.settle_triggers_high()
    by_idx = {t.birth_idx: t for t in trigs}
    sec = by_idx[3]  # 次顶 peak=6
    assert sec.kind == "top"
    assert sec.can_settle is True
    assert sec.extreme_price == pytest.approx(6.0)
    assert sec.trigger_price == pytest.approx(4.0)
    assert sec.trigger_price < sec.extreme_price  # 跌破方向
    assert sec.trigger_persistence == pytest.approx(2.0)
    # last high=6，还需跌 2 才到 4
    assert sec.last_price == pytest.approx(6.0)
    assert sec.gap == pytest.approx(2.0)


@pytest.mark.unit
def test_bottom_settle_trigger_is_above_valley_a_rebound():
    """底 valley=4 的 trigger_price=6（涨过 6 确认底），6 > 4（涨过方向）。"""
    dt = _feed([11.0, 3.0, 7.0, 5.0], [10.0, 2.0, 6.0, 4.0])
    trigs = dt.settle_triggers_low()
    by_idx = {t.birth_idx: t for t in trigs}
    bot = by_idx[3]  # 底 valley=4
    assert bot.kind == "bottom"
    assert bot.can_settle is True
    assert bot.extreme_price == pytest.approx(4.0)
    assert bot.trigger_price == pytest.approx(6.0)
    assert bot.trigger_price > bot.extreme_price  # 涨过方向
    assert bot.trigger_persistence == pytest.approx(2.0)
    assert bot.last_price == pytest.approx(4.0)  # last low
    assert bot.gap == pytest.approx(2.0)


# --------------------------------------------------------------------
# 3. 证伪：喂触发价使分量真的 settle
# --------------------------------------------------------------------


@pytest.mark.unit
def test_top_settles_when_high_breaks_down_to_trigger():
    """喂 high=trigger_price(4) → 次顶(idx3)进入 T_high settled；喂 4.01 不 settle。"""
    dt = _feed(TOP_HIGHS, TOP_LOWS)
    trig = {t.birth_idx: t for t in dt.settle_triggers_high()}[3]

    # 证伪 1：high 仍在 4 之上（4.01）→ 不 settle
    dt_above = _feed(TOP_HIGHS, TOP_LOWS)
    dt_above.update(4.01, 3.0)
    settled_above = {c.birth_idx for c in dt_above.settled_components_high()}
    assert 3 not in settled_above

    # 证伪 2：high 跌破到 4.0 → 次顶 settle，persistence==2
    dt_break = _feed(TOP_HIGHS, TOP_LOWS)
    dt_break.update(trig.trigger_price, 3.0)
    settled = {c.birth_idx: c for c in dt_break.settled_components_high()}
    assert 3 in settled
    assert settled[3].persistence == pytest.approx(2.0)
    assert settled[3].extreme_price == pytest.approx(6.0)
    assert settled[3].confirm_price == pytest.approx(4.0)


@pytest.mark.unit
def test_global_top_cannot_settle_by_breakdown():
    """全局最高顶(peak=8,idx1)不可被跌破 settle（栈底镜像），can_settle=False。"""
    dt = _feed(TOP_HIGHS, TOP_LOWS)
    glob = {t.birth_idx: t for t in dt.settle_triggers_high()}[1]
    assert glob.extreme_price == pytest.approx(8.0)
    assert glob.can_settle is False
    assert glob.trigger_price is None
    assert glob.is_dominant is True
    assert glob.reversal_amplitude is not None and glob.reversal_amplitude > 0


# --------------------------------------------------------------------
# 4. persistence 取负不变 + T_low == 单树引擎
# --------------------------------------------------------------------


@pytest.mark.unit
def test_persistence_invariant_under_negation():
    """T_high 分量 persistence = 真实下跌幅度（peak − confirm），取负不改变它。"""
    dt = _feed(TOP_HIGHS, TOP_LOWS)
    for c in dt.settled_components_high() + dt.alive_components_high():
        # persistence 恒 = |extreme − confirm|（top: 下跌幅度）
        assert c.persistence == pytest.approx(abs(c.extreme_price - c.confirm_price))


@pytest.mark.unit
def test_low_tree_settle_matches_single_close_engine_property():
    """property：T_low 的 settle 阶梯 ≡ 把同一 low 序列喂单树 settle_triggers。"""
    rng = random.Random(20260530)
    for _ in range(40):
        n = rng.randint(4, 30)
        lows = [round(rng.uniform(1.0, 100.0), 2) for _ in range(n)]
        highs = [x + 1.0 for x in lows]
        dt = _feed(highs, lows)
        dual = {t.birth_idx: t for t in dt.settle_triggers_low()}
        single = {t.birth_idx: t for t in settle_triggers_from_prices(lows)}
        assert dual.keys() == single.keys()
        for k in single:
            s = single[k]
            d = dual[k]
            assert d.extreme_price == pytest.approx(s.birth_price)
            assert d.can_settle == s.can_settle_by_rebound
            if s.settle_price is None:
                assert d.trigger_price is None
            else:
                assert d.trigger_price == pytest.approx(s.settle_price)


# --------------------------------------------------------------------
# 5. strokes：交替提取 + 同类合并
# --------------------------------------------------------------------


@pytest.mark.unit
def test_strokes_alternate_up_down():
    """底-顶-底-顶 的 high/low 序列 → strokes 方向交替 up/down/up...。"""
    # 构造清晰的 极小-极大 交替：lows 给底，highs 给顶
    #   t: 0   1   2   3   4   5   6
    # low:  5   1   5   2   6   3   7   （底在 1,3,5: 值 1,2,3）
    # high: 6   2   9   3  10   4  11   （顶在 0?,2,4,6: 值 9,10,11）
    lows = [5.0, 1.0, 5.0, 2.0, 6.0, 3.0, 7.0]
    highs = [6.0, 2.0, 9.0, 3.0, 10.0, 4.0, 11.0]
    dt = _feed(highs, lows)
    strokes = dt.strokes()
    assert len(strokes) >= 2
    # 相邻 stroke 方向必交替
    for a, b in zip(strokes, strokes[1:]):
        assert a.direction != b.direction
    # 每个 stroke 端点类型与方向自洽
    for s in strokes:
        if s.direction == "up":
            assert s.start_kind == "bottom" and s.end_kind == "top"
        else:
            assert s.start_kind == "top" and s.end_kind == "bottom"
        assert s.amplitude == pytest.approx(abs(s.end_price - s.start_price))
        assert s.start_idx < s.end_idx


@pytest.mark.unit
def test_strokes_collapse_consecutive_same_kind_keeps_extreme():
    """两个相邻顶之间无底 → 折叠为更高的顶（保留更极端者）。

    构造：low 单调上升（仅 idx0 一个底）；high 有两个顶 10@idx1、12@idx3，时间上相邻
    （之间无底）→ 折叠后只保留 12，顶 10 不得作为任何笔端点出现。
    """
    highs = [2.0, 10.0, 3.0, 12.0, 1.0]
    lows = [0.0, 1.0, 2.0, 3.0, 4.0]  # 单调上升 → 仅 idx0 是底
    dt = _feed(highs, lows)
    strokes = dt.strokes()
    endpoint_prices = {p for s in strokes for p in (s.start_price, s.end_price)}
    assert 12.0 in endpoint_prices       # 更高的顶保留
    assert 10.0 not in endpoint_prices   # 较低的顶被折叠掉
    # 笔序列内部严格交替：每笔两端异类，相邻笔方向相反
    for s in strokes:
        assert s.start_kind != s.end_kind
    for a, b in zip(strokes, strokes[1:]):
        assert a.direction != b.direction
        assert a.end_idx == b.start_idx and a.end_kind == b.start_kind  # 共享枢轴


# --------------------------------------------------------------------
# 6. containment：高低分量区间嵌套
# --------------------------------------------------------------------


@pytest.mark.unit
def test_containment_pairs_are_real_interval_nestings():
    """containment_pairs 返回的每一对，区间嵌套关系必须真实成立。"""
    lows = [5.0, 1.0, 5.0, 2.0, 6.0, 3.0, 7.0]
    highs = [6.0, 2.0, 9.0, 3.0, 10.0, 4.0, 11.0]
    dt = _feed(highs, lows)
    pairs = dt.containment_pairs()
    for p in pairs:
        h, l, rel = p.high, p.low, p.relation
        if rel == "high_contains_low":
            assert h.lo <= l.lo and l.hi <= h.hi
        elif rel == "low_contains_high":
            assert l.lo <= h.lo and h.hi <= l.hi
        else:
            assert rel == "equal" and h.lo == l.lo and h.hi == l.hi


# --------------------------------------------------------------------
# 7. 输入接口：update_bar 接受 dict / tuple / namedtuple-like
# --------------------------------------------------------------------


@pytest.mark.unit
def test_update_bar_accepts_dict_and_object():
    dt1 = DualMergeTree()
    dt2 = DualMergeTree()
    dt3 = DualMergeTree()
    for h, l in zip(TOP_HIGHS, TOP_LOWS):
        dt1.update(h, l)
        dt2.update_bar({"high": h, "low": l})

        class Bar:  # 简易对象
            pass

        b = Bar()
        b.high, b.low = h, l
        dt3.update_bar(b)
    a1 = sorted(c.extreme_price for c in dt1.alive_components_high())
    a2 = sorted(c.extreme_price for c in dt2.alive_components_high())
    a3 = sorted(c.extreme_price for c in dt3.alive_components_high())
    assert a1 == pytest.approx(a2)
    assert a1 == pytest.approx(a3)


@pytest.mark.unit
def test_from_ohlc_classmethod_equivalent_to_streaming():
    dt_stream = _feed(TOP_HIGHS, TOP_LOWS)
    dt_batch = DualMergeTree.from_ohlc(TOP_HIGHS, TOP_LOWS)
    s = sorted(c.extreme_price for c in dt_stream.alive_components_high())
    b = sorted(c.extreme_price for c in dt_batch.alive_components_high())
    assert s == pytest.approx(b)


# --------------------------------------------------------------------
# 8. 边界
# --------------------------------------------------------------------


@pytest.mark.unit
def test_empty_dual_tree():
    dt = DualMergeTree()
    assert dt.alive_components_high() == ()
    assert dt.alive_components_low() == ()
    assert dt.settle_triggers_high() == ()
    assert dt.settle_triggers_low() == ()
    assert dt.strokes() == ()
    assert dt.containment_pairs() == ()


@pytest.mark.unit
def test_single_bar():
    dt = DualMergeTree()
    dt.update(10.0, 8.0)
    hi = dt.alive_components_high()
    lo = dt.alive_components_low()
    assert len(hi) == 1 and hi[0].extreme_price == pytest.approx(10.0)
    assert len(lo) == 1 and lo[0].extreme_price == pytest.approx(8.0)
    # 单点：全局分量，不可 settle
    assert dt.settle_triggers_high()[0].can_settle is False
    assert dt.settle_triggers_low()[0].can_settle is False
