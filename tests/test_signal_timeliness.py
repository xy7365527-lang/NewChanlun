"""signal_timeliness 纯逻辑单元测试。

覆盖：
- forward_window_stats: 信号后 N bar 的价格统计
- classify_completion: 走势完成度分类
- aggregate_by_kind: 按买卖点类型聚合
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.types import Bar

signal_tl = pytest.importorskip("signal_timeliness")


# ── helpers ──


def _make_bars(closes: list[float]) -> list[Bar]:
    epoch = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars = []
    for i, c in enumerate(closes):
        bars.append(Bar(
            ts=epoch + timedelta(minutes=i),
            open=c - 0.5,
            high=c + 1.0,
            low=c - 1.0,
            close=c,
        ))
    return bars


def _make_signal(
    bar_idx: int = 5,
    kind: str = "type1",
    side: str = "buy",
    level_id: int = 1,
    price: float = 100.0,
    confirmed: bool = True,
) -> dict:
    return {
        "symbol": "TEST",
        "kind": kind,
        "side": side,
        "level_id": level_id,
        "seg_idx": 0,
        "bar_idx": bar_idx,
        "price": price,
        "confirmed": confirmed,
    }


# ── forward_window_stats ──


class TestForwardWindowStats:
    def test_buy_signal_price_up(self):
        """buy 信号后价格上涨 → 正收益。"""
        # bar_idx=5, horizon=5 → target=10 (105), horizon=10 → target=15 (105)
        closes = [100.0] * 10 + [105.0] * 10
        bars = _make_bars(closes)
        sig = _make_signal(bar_idx=5, side="buy", price=100.0)
        stats = signal_tl.forward_window_stats(sig, bars, horizons=[5, 10])
        assert stats["fwd_5"] == pytest.approx(0.05, abs=1e-9)
        assert stats["fwd_10"] == pytest.approx(0.05, abs=1e-9)

    def test_sell_signal_price_down(self):
        """sell 信号后价格下跌 → 正收益（反转）。"""
        closes = [100.0] * 10 + [95.0] * 10
        bars = _make_bars(closes)
        sig = _make_signal(bar_idx=5, side="sell", price=100.0)
        stats = signal_tl.forward_window_stats(sig, bars, horizons=[10])
        assert stats["fwd_10"] == pytest.approx(0.05, abs=1e-9)

    def test_horizon_beyond_data(self):
        """horizon 超出数据范围 → None。"""
        bars = _make_bars([100.0] * 10)
        sig = _make_signal(bar_idx=8, price=100.0)
        stats = signal_tl.forward_window_stats(sig, bars, horizons=[5])
        assert stats["fwd_5"] is None

    def test_max_adverse_excursion(self):
        """计算最大不利偏移（MAE）。"""
        # buy 信号后先跌后涨
        closes = [100.0] * 6 + [97.0, 96.0, 98.0, 102.0]
        bars = _make_bars(closes)
        sig = _make_signal(bar_idx=5, side="buy", price=100.0)
        stats = signal_tl.forward_window_stats(sig, bars, horizons=[4])
        # MAE = (96 - 100) / 100 = -0.04
        assert stats["mae_4"] == pytest.approx(-0.04, abs=1e-9)

    def test_max_favorable_excursion(self):
        """计算最大有利偏移（MFE）。"""
        closes = [100.0] * 6 + [103.0, 105.0, 102.0, 101.0]
        bars = _make_bars(closes)
        sig = _make_signal(bar_idx=5, side="buy", price=100.0)
        stats = signal_tl.forward_window_stats(sig, bars, horizons=[4])
        # MFE = (105 - 100) / 100 = 0.05
        assert stats["mfe_4"] == pytest.approx(0.05, abs=1e-9)


# ── classify_completion ──


class TestClassifyCompletion:
    def test_full_completion(self):
        """MFE 远大于 MAE → full。"""
        stats = {"mfe_20": 0.08, "mae_20": -0.01, "fwd_20": 0.06}
        assert signal_tl.classify_completion(stats, horizon=20) == "full"

    def test_partial_completion(self):
        """MFE 可观但最终回吐大部分 → partial。"""
        stats = {"mfe_20": 0.06, "mae_20": -0.02, "fwd_20": 0.01}
        assert signal_tl.classify_completion(stats, horizon=20) == "partial"

    def test_failed(self):
        """MFE 很小，MAE 很大 → failed。"""
        stats = {"mfe_20": 0.01, "mae_20": -0.06, "fwd_20": -0.05}
        assert signal_tl.classify_completion(stats, horizon=20) == "failed"

    def test_none_values(self):
        """数据不足 → unknown。"""
        stats = {"mfe_20": None, "mae_20": None, "fwd_20": None}
        assert signal_tl.classify_completion(stats, horizon=20) == "unknown"


# ── aggregate_by_kind ──


class TestAggregateByKind:
    def test_grouping(self):
        records = [
            {"kind": "type1", "side": "buy", "fwd_20": 0.05, "completion": "full"},
            {"kind": "type1", "side": "buy", "fwd_20": -0.02, "completion": "failed"},
            {"kind": "type2", "side": "sell", "fwd_20": 0.03, "completion": "full"},
        ]
        agg = signal_tl.aggregate_by_kind(records)
        assert "type1_buy" in agg
        assert "type2_sell" in agg
        assert agg["type1_buy"]["count"] == 2
        assert agg["type2_sell"]["count"] == 1

    def test_completion_distribution(self):
        records = [
            {"kind": "type1", "side": "buy", "fwd_20": 0.05, "completion": "full"},
            {"kind": "type1", "side": "buy", "fwd_20": 0.02, "completion": "partial"},
            {"kind": "type1", "side": "buy", "fwd_20": -0.03, "completion": "failed"},
        ]
        agg = signal_tl.aggregate_by_kind(records)
        dist = agg["type1_buy"]["completion_dist"]
        assert dist["full"] == 1
        assert dist["partial"] == 1
        assert dist["failed"] == 1

    def test_empty_records(self):
        agg = signal_tl.aggregate_by_kind([])
        assert agg == {}
