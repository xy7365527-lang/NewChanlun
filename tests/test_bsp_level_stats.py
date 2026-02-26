"""bsp_level_stats 纯逻辑单元测试。

覆盖：
- compute_level_frequency: 各级别触发频率
- compute_level_effectiveness: 各级别有效性（确认率 + 前向收益）
- compare_levels: 级别间对比
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.types import Bar

bsp_ls = pytest.importorskip("bsp_level_stats")


# ── helpers ──


def _make_bars(closes: list[float]) -> list[Bar]:
    epoch = datetime(2024, 1, 1, tzinfo=timezone.utc)
    return [
        Bar(
            ts=epoch + timedelta(minutes=i),
            open=c - 0.5,
            high=c + 1.0,
            low=c - 1.0,
            close=c,
        )
        for i, c in enumerate(closes)
    ]


def _make_signal(
    level_id: int = 1,
    kind: str = "type1",
    side: str = "buy",
    bar_idx: int = 5,
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


# ── compute_level_frequency ──


class TestComputeLevelFrequency:
    def test_single_level(self):
        signals = [_make_signal(level_id=1) for _ in range(5)]
        freq = bsp_ls.compute_level_frequency(signals)
        assert freq[1]["count"] == 5
        assert freq[1]["ratio"] == pytest.approx(1.0)

    def test_multi_level(self):
        signals = [
            *[_make_signal(level_id=1) for _ in range(3)],
            *[_make_signal(level_id=2) for _ in range(7)],
        ]
        freq = bsp_ls.compute_level_frequency(signals)
        assert freq[1]["count"] == 3
        assert freq[2]["count"] == 7
        assert freq[1]["ratio"] == pytest.approx(0.3)
        assert freq[2]["ratio"] == pytest.approx(0.7)

    def test_empty(self):
        freq = bsp_ls.compute_level_frequency([])
        assert freq == {}


# ── compute_level_effectiveness ──


class TestComputeLevelEffectiveness:
    def test_confirmed_rate(self):
        signals = [
            _make_signal(level_id=1, confirmed=True),
            _make_signal(level_id=1, confirmed=True),
            _make_signal(level_id=1, confirmed=False),
        ]
        bars = _make_bars([100.0] * 50)
        eff = bsp_ls.compute_level_effectiveness(signals, bars, horizon=10)
        assert eff[1]["confirmed_rate"] == pytest.approx(2 / 3)

    def test_forward_return(self):
        # 信号在 bar_idx=5, price=100, bar_idx+10 close=110 → 10% return
        closes = [100.0] * 16
        closes[15] = 110.0
        bars = _make_bars(closes)
        signals = [_make_signal(level_id=1, bar_idx=5, price=100.0, side="buy")]
        eff = bsp_ls.compute_level_effectiveness(signals, bars, horizon=10)
        assert eff[1]["mean_fwd_return"] == pytest.approx(0.10, abs=1e-9)

    def test_multi_level_effectiveness(self):
        closes = [100.0] * 50
        bars = _make_bars(closes)
        signals = [
            _make_signal(level_id=1, bar_idx=5, confirmed=True),
            _make_signal(level_id=2, bar_idx=10, confirmed=False),
        ]
        eff = bsp_ls.compute_level_effectiveness(signals, bars, horizon=10)
        assert 1 in eff
        assert 2 in eff
        assert eff[1]["confirmed_rate"] == pytest.approx(1.0)
        assert eff[2]["confirmed_rate"] == pytest.approx(0.0)


# ── compare_levels ──


class TestCompareLevels:
    def test_ranking_by_effectiveness(self):
        level_stats = {
            1: {"count": 10, "confirmed_rate": 0.8, "mean_fwd_return": 0.05},
            2: {"count": 20, "confirmed_rate": 0.6, "mean_fwd_return": 0.08},
            3: {"count": 5, "confirmed_rate": 0.9, "mean_fwd_return": 0.02},
        }
        ranked = bsp_ls.compare_levels(level_stats, sort_by="mean_fwd_return")
        assert ranked[0]["level_id"] == 2
        assert ranked[1]["level_id"] == 1
        assert ranked[2]["level_id"] == 3

    def test_ranking_by_confirmed_rate(self):
        level_stats = {
            1: {"count": 10, "confirmed_rate": 0.5, "mean_fwd_return": 0.05},
            2: {"count": 20, "confirmed_rate": 0.9, "mean_fwd_return": 0.03},
        }
        ranked = bsp_ls.compare_levels(level_stats, sort_by="confirmed_rate")
        assert ranked[0]["level_id"] == 2

    def test_empty(self):
        ranked = bsp_ls.compare_levels({})
        assert ranked == []
