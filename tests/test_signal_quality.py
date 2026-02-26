"""信号质量统计逻辑的单元测试。

用合成数据验证 compute_forward_returns 和 compute_signal_stats，
不依赖真实市场数据或 RecursiveOrchestrator。
"""

from __future__ import annotations

import sys
from datetime import datetime, timedelta
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from newchan.types import Bar
from signal_quality_report import compute_forward_returns, compute_signal_stats


# ── 辅助工厂 ──


def _make_bars(prices: list[float], start: datetime | None = None) -> list[Bar]:
    """从价格序列生成 Bar 列表（open=high=low=close=price）。"""
    t = start or datetime(2025, 1, 1, 9, 30)
    bars = []
    for p in prices:
        bars.append(Bar(ts=t, open=p, high=p, low=p, close=p, volume=100.0))
        t += timedelta(minutes=1)
    return bars


def _make_signal(
    bar_idx: int,
    price: float,
    kind: str = "type1",
    side: str = "buy",
    confirmed: bool = True,
    symbol: str = "TEST",
) -> dict:
    return {
        "symbol": symbol,
        "kind": kind,
        "side": side,
        "level_id": 1,
        "seg_idx": 0,
        "bar_idx": bar_idx,
        "price": price,
        "confirmed": confirmed,
    }


# ── compute_forward_returns 测试 ──


class TestForwardReturns:
    """测试后续价格变动计算。"""

    def test_buy_signal_price_up(self):
        """buy 信号后价格上涨 → 正收益。"""
        bars = _make_bars([100.0] * 5 + [110.0] * 50)
        sig = _make_signal(bar_idx=4, price=100.0, side="buy")
        result = compute_forward_returns(bars, [sig], [5, 10])
        r = result[0]
        assert r["fwd_5"] == pytest.approx(0.10)
        assert r["fwd_10"] == pytest.approx(0.10)

    def test_buy_signal_price_down(self):
        """buy 信号后价格下跌 → 负收益。"""
        bars = _make_bars([100.0] * 5 + [90.0] * 50)
        sig = _make_signal(bar_idx=4, price=100.0, side="buy")
        result = compute_forward_returns(bars, [sig], [5])
        assert result[0]["fwd_5"] == pytest.approx(-0.10)

    def test_sell_signal_price_down(self):
        """sell 信号后价格下跌 → 正收益（方向取反）。"""
        bars = _make_bars([100.0] * 5 + [90.0] * 50)
        sig = _make_signal(bar_idx=4, price=100.0, side="sell")
        result = compute_forward_returns(bars, [sig], [5])
        assert result[0]["fwd_5"] == pytest.approx(0.10)

    def test_sell_signal_price_up(self):
        """sell 信号后价格上涨 → 负收益。"""
        bars = _make_bars([100.0] * 5 + [110.0] * 50)
        sig = _make_signal(bar_idx=4, price=100.0, side="sell")
        result = compute_forward_returns(bars, [sig], [5])
        assert result[0]["fwd_5"] == pytest.approx(-0.10)

    def test_horizon_beyond_data(self):
        """观测窗口超出数据范围 → None。"""
        bars = _make_bars([100.0] * 10)
        sig = _make_signal(bar_idx=5, price=100.0)
        result = compute_forward_returns(bars, [sig], [10])
        assert result[0]["fwd_10"] is None

    def test_zero_price_signal(self):
        """price=0 的信号 → 所有 fwd 为 None。"""
        bars = _make_bars([100.0] * 20)
        sig = _make_signal(bar_idx=5, price=0.0)
        result = compute_forward_returns(bars, [sig], [5, 10])
        assert result[0]["fwd_5"] is None
        assert result[0]["fwd_10"] is None

    def test_multiple_signals(self):
        """多个信号独立计算。"""
        bars = _make_bars([100.0] * 3 + [110.0] * 3 + [105.0] * 20)
        sigs = [
            _make_signal(bar_idx=2, price=100.0, side="buy"),
            _make_signal(bar_idx=5, price=110.0, side="sell"),
        ]
        result = compute_forward_returns(bars, sigs, [5])
        # sig1: buy@100, bar[7]=105 → +5%
        assert result[0]["fwd_5"] == pytest.approx(0.05)
        # sig2: sell@110, bar[10]=105 → (110-105)/110 = +4.545%
        assert result[1]["fwd_5"] == pytest.approx((110.0 - 105.0) / 110.0)


# ── compute_signal_stats 测试 ──


class TestSignalStats:
    """测试统计聚合逻辑。"""

    def test_basic_aggregation(self):
        """基本分组聚合。"""
        enriched = [
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.05},
            {"kind": "type1", "side": "buy", "confirmed": False, "fwd_5": -0.02},
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.03},
        ]
        stats = compute_signal_stats(enriched, [5])
        s = stats["type1_buy"]
        assert s["count"] == 3
        assert s["confirmed_count"] == 2
        assert s["confirm_rate"] == pytest.approx(2 / 3)
        assert s["fwd_5"]["n"] == 3
        assert s["fwd_5"]["mean"] == pytest.approx(0.02, abs=1e-5)

    def test_multiple_groups(self):
        """多个 kind×side 分组。"""
        enriched = [
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.05},
            {"kind": "type2", "side": "sell", "confirmed": False, "fwd_5": 0.10},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert "type1_buy" in stats
        assert "type2_sell" in stats
        assert stats["type1_buy"]["count"] == 1
        assert stats["type2_sell"]["count"] == 1

    def test_empty_signals(self):
        """空信号列表 → 空结果。"""
        stats = compute_signal_stats([], [5, 10])
        assert stats == {}

    def test_all_none_forward(self):
        """所有 fwd 为 None → n=0。"""
        enriched = [
            {"kind": "type3", "side": "buy", "confirmed": True, "fwd_5": None},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert stats["type3_buy"]["fwd_5"]["n"] == 0
        assert stats["type3_buy"]["fwd_5"]["mean"] is None

    def test_median_odd(self):
        """奇数个值的中位数。"""
        enriched = [
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.01},
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.05},
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.03},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert stats["type1_buy"]["fwd_5"]["median"] == pytest.approx(0.03)

    def test_median_even(self):
        """偶数个值的中位数。"""
        enriched = [
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.02},
            {"kind": "type1", "side": "buy", "confirmed": True, "fwd_5": 0.04},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert stats["type1_buy"]["fwd_5"]["median"] == pytest.approx(0.03)

    def test_confirm_rate_zero(self):
        """全部未确认 → confirm_rate=0。"""
        enriched = [
            {"kind": "type1", "side": "sell", "confirmed": False, "fwd_5": 0.01},
            {"kind": "type1", "side": "sell", "confirmed": False, "fwd_5": 0.02},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert stats["type1_sell"]["confirm_rate"] == 0.0

    def test_std_single_value(self):
        """单个值 → std=0。"""
        enriched = [
            {"kind": "type2", "side": "buy", "confirmed": True, "fwd_5": 0.05},
        ]
        stats = compute_signal_stats(enriched, [5])
        assert stats["type2_buy"]["fwd_5"]["std"] == pytest.approx(0.0)
