"""tests for multi_tf_adapter — 多 TF 输入架构测试。

测试覆盖：
1. TimeframeLevel 正确构造
2. MultiTFOrchestrator 接受多 TF 输入
3. 跨 TF 对齐正确性
4. CrossLevelDivergence 检测逻辑
5. BuySellPoint 从背驰推导
6. 空输入边界条件
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.a_move_v1 import Move
from newchan.topology.multi_tf_adapter import (
    BuySellPoint,
    CrossLevelDivergence,
    LevelResult,
    MultiTFOrchestrator,
    MultiTFResult,
    TimeframeLevel,
    _detect_cross_level_divergence,
    _derive_buysellpoints,
    _move_amplitude,
    align_bars_by_timestamp,
)
from newchan.types import Bar


# ── helpers ──────────────────────────────────────────────


def _make_bar(ts: datetime, o: float, h: float, l: float, c: float) -> Bar:
    return Bar(ts=ts, open=o, high=h, low=l, close=c)


def _make_bars(n: int, base_price: float = 100.0, start: datetime | None = None) -> list[Bar]:
    """生成 n 根合成 bar，带简单价格波动。"""
    if start is None:
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    price = base_price
    for i in range(n):
        ts = start + timedelta(minutes=i)
        delta = 0.5 * (1 if i % 3 != 0 else -1)
        o = price
        h = price + abs(delta)
        l = price - abs(delta)
        c = price + delta
        bars.append(_make_bar(ts, o, h, l, c))
        price = c
    return bars


def _make_move(
    direction: str = "up",
    high: float = 110.0,
    low: float = 100.0,
    settled: bool = False,
) -> Move:
    return Move(
        kind="consolidation",
        direction=direction,  # type: ignore[arg-type]
        seg_start=0,
        seg_end=1,
        zs_start=0,
        zs_end=0,
        zs_count=1,
        settled=settled,
        high=high,
        low=low,
    )


def _make_tf(name: str, index: int) -> TimeframeLevel:
    return TimeframeLevel(tf_name=name, bar_source="test", level_index=index)


def _make_level_result(
    tf: TimeframeLevel,
    direction: str = "up",
    high: float = 110.0,
    low: float = 100.0,
    settled: bool = False,
) -> LevelResult:
    move = _make_move(direction=direction, high=high, low=low, settled=settled)
    return LevelResult(
        tf=tf,
        snapshot=None,
        bar_count=100,
        move_count=1,
        zhongshu_count=1,
        direction=direction,  # type: ignore[arg-type]
        last_move=move,
    )


# ── TimeframeLevel tests ────────────────────────────────


class TestTimeframeLevel:
    def test_construction(self) -> None:
        tf = TimeframeLevel(tf_name="daily", bar_source="yfinance", level_index=0)
        assert tf.tf_name == "daily"
        assert tf.bar_source == "yfinance"
        assert tf.level_index == 0

    def test_frozen(self) -> None:
        tf = TimeframeLevel(tf_name="daily", bar_source="yf", level_index=0)
        with pytest.raises(AttributeError):
            tf.tf_name = "weekly"  # type: ignore[misc]


# ── Move amplitude tests ────────────────────────────────


class TestMoveAmplitude:
    def test_positive_range(self) -> None:
        m = _make_move(high=110.0, low=100.0)
        assert _move_amplitude(m) == pytest.approx(10.0)

    def test_zero_range(self) -> None:
        m = _make_move(high=100.0, low=100.0)
        assert _move_amplitude(m) == pytest.approx(0.0)


# ── CrossLevelDivergence detection tests ────────────────


class TestCrossLevelDivergence:
    def test_divergence_detected_when_low_force_smaller(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up", high=120.0, low=100.0)
        low_result = _make_level_result(low_tf, "up", high=105.0, low=100.0)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is not None
        assert div.direction == "top"
        assert div.force_high == pytest.approx(20.0)
        assert div.force_low == pytest.approx(5.0)
        assert div.ratio == pytest.approx(0.25)

    def test_no_divergence_when_low_force_larger(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up", high=105.0, low=100.0)
        low_result = _make_level_result(low_tf, "up", high=120.0, low=100.0)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is None

    def test_no_divergence_when_directions_differ(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up", high=120.0, low=100.0)
        low_result = _make_level_result(low_tf, "down", high=105.0, low=100.0)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is None

    def test_no_divergence_when_no_moves(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = LevelResult(
            tf=high_tf, snapshot=None, bar_count=0,
            move_count=0, zhongshu_count=0, direction="none", last_move=None,
        )
        low_result = _make_level_result(low_tf, "up")

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is None

    def test_bottom_divergence(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "down", high=100.0, low=80.0)
        low_result = _make_level_result(low_tf, "down", high=100.0, low=95.0)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is not None
        assert div.direction == "bottom"

    def test_confirmed_flag(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up", high=120.0, low=100.0, settled=True)
        low_result = _make_level_result(low_tf, "up", high=105.0, low=100.0, settled=True)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is not None
        assert div.confirmed is True

    def test_not_confirmed_when_one_unsettled(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up", high=120.0, low=100.0, settled=True)
        low_result = _make_level_result(low_tf, "up", high=105.0, low=100.0, settled=False)

        div = _detect_cross_level_divergence(high_tf, high_result, low_tf, low_result)
        assert div is not None
        assert div.confirmed is False


# ── BuySellPoint derivation tests ───────────────────────


class TestDeriveBuySellPoints:
    def test_top_divergence_produces_sell(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0),
            low_move=_make_move("up", 105.0, 100.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        bsps = _derive_buysellpoints([div])
        assert len(bsps) == 1
        assert bsps[0].side == "sell"
        assert bsps[0].price == 105.0  # low_move.high

    def test_bottom_divergence_produces_buy(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="bottom",
            high_move=_make_move("down", 100.0, 80.0),
            low_move=_make_move("down", 100.0, 95.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        bsps = _derive_buysellpoints([div])
        assert len(bsps) == 1
        assert bsps[0].side == "buy"
        assert bsps[0].price == 95.0  # low_move.low

    def test_empty_divergences(self) -> None:
        assert _derive_buysellpoints([]) == []

    def test_unconfirmed_divergences_produce_no_bsp(self) -> None:
        """未确认背驰不得进入 type1 买卖点（假信号防护）。"""
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0, settled=False),
            low_move=_make_move("up", 105.0, 100.0, settled=False),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=False,
        )
        assert _derive_buysellpoints([div]) == []


# ── Timestamp alignment tests ───────────────────────────


class TestAlignBarsByTimestamp:
    def test_filter_within_range(self) -> None:
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
        bars = _make_bars(10, start=start)
        ref_start = start + timedelta(minutes=2)
        ref_end = start + timedelta(minutes=5)
        aligned = align_bars_by_timestamp(bars, ref_start, ref_end)
        assert len(aligned) == 4  # minutes 2,3,4,5
        assert all(ref_start <= b.ts <= ref_end for b in aligned)

    def test_empty_range(self) -> None:
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
        bars = _make_bars(10, start=start)
        ref_start = start + timedelta(hours=1)
        ref_end = start + timedelta(hours=2)
        aligned = align_bars_by_timestamp(bars, ref_start, ref_end)
        assert aligned == []


# ── MultiTFOrchestrator tests ───────────────────────────


class TestMultiTFOrchestrator:
    def test_construction(self) -> None:
        tfs = [_make_tf("daily", 0), _make_tf("weekly", 1)]
        orch = MultiTFOrchestrator(tfs)
        assert len(orch.timeframes) == 2
        assert orch.timeframes[0].level_index == 0

    def test_empty_timeframes_raises(self) -> None:
        with pytest.raises(ValueError, match="must not be empty"):
            MultiTFOrchestrator([])

    def test_sorts_by_level_index(self) -> None:
        tfs = [_make_tf("weekly", 1), _make_tf("daily", 0)]
        orch = MultiTFOrchestrator(tfs)
        assert orch.timeframes[0].tf_name == "daily"
        assert orch.timeframes[1].tf_name == "weekly"

    def test_run_with_synthetic_data(self) -> None:
        """用合成数据验证 MultiTFOrchestrator.run 的基本功能。"""
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        orch = MultiTFOrchestrator(tfs, stroke_mode="wide")

        start = datetime(2024, 1, 1, 9, 30, tzinfo=timezone.utc)
        bars_1min = _make_bars(200, base_price=100.0, start=start)
        bars_5min = _make_bars(40, base_price=100.0, start=start)

        result = orch.run({"1min": bars_1min, "5min": bars_5min})

        assert isinstance(result, MultiTFResult)
        assert "1min" in result.levels
        assert "5min" in result.levels
        assert result.levels["1min"].bar_count == 200
        assert result.levels["5min"].bar_count == 40

    def test_run_with_empty_bars(self) -> None:
        tfs = [_make_tf("1min", 0)]
        orch = MultiTFOrchestrator(tfs)
        result = orch.run({"1min": []})
        assert result.levels["1min"].bar_count == 0
        assert result.levels["1min"].last_move is None

    def test_run_with_missing_tf_data(self) -> None:
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        orch = MultiTFOrchestrator(tfs)
        start = datetime(2024, 1, 1, 9, 30, tzinfo=timezone.utc)
        bars_1min = _make_bars(100, start=start)
        # 5min 数据缺失
        result = orch.run({"1min": bars_1min})
        assert result.levels["5min"].bar_count == 0

    def test_reset(self) -> None:
        tfs = [_make_tf("daily", 0)]
        orch = MultiTFOrchestrator(tfs)
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
        bars = _make_bars(50, start=start)
        orch.run({"daily": bars})
        orch.reset()
        # After reset, running again should produce same results
        result = orch.run({"daily": bars})
        assert result.levels["daily"].bar_count == 50


# ── MultiTFResult structure tests ───────────────────────


class TestMultiTFResult:
    def test_default_construction(self) -> None:
        result = MultiTFResult()
        assert result.levels == {}
        assert result.cross_level_divergences == []
        assert result.buysellpoints == []
