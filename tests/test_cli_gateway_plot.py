"""cli.py + gateway.py 纯函数 + b_plot.py 单元测试。"""
from __future__ import annotations

from datetime import datetime, timezone
from types import SimpleNamespace
from unittest.mock import patch, MagicMock

import numpy as np
import pandas as pd
import pytest

from newchan.types import Bar
from newchan.cli import build_parser
from newchan.gateway import (
    _interval_to_tf,
    _df_to_bars,
    _stroke_to_dict,
    _event_to_ws,
    _snapshot_to_ws,
    _bar_to_ws,
)


# ══════════ cli.py ══════════


class TestBuildParser:
    def test_returns_parser(self):
        parser = build_parser()
        assert parser is not None

    def test_fetch_subcommand(self):
        parser = build_parser()
        args = parser.parse_args(["fetch", "--symbol", "CL"])
        assert args.command == "fetch"
        assert args.symbol == "CL"

    def test_plot_subcommand(self):
        parser = build_parser()
        args = parser.parse_args(["plot", "--symbol", "CL"])
        assert args.command == "plot"

    def test_synthetic_subcommand(self):
        parser = build_parser()
        args = parser.parse_args(["synthetic", "--a", "CL", "--b", "GC"])
        assert args.command == "synthetic"
        assert args.sym_a == "CL"
        assert args.sym_b == "GC"

    def test_fetch_db_subcommand(self):
        parser = build_parser()
        args = parser.parse_args(["fetch-db", "--symbol", "AMD"])
        assert args.command == "fetch-db"

    def test_fetch_db_all_flag(self):
        parser = build_parser()
        args = parser.parse_args(["fetch-db", "--all"])
        assert args.fetch_all is True

    def test_chart_subcommand(self):
        parser = build_parser()
        args = parser.parse_args(["chart", "--port", "9000"])
        assert args.command == "chart"
        assert args.port == 9000

    def test_fetch_defaults(self):
        parser = build_parser()
        args = parser.parse_args(["fetch", "--symbol", "CL"])
        assert args.source == "ibkr"
        assert args.interval == "1min"
        assert args.refresh is False


# ══════════ gateway.py pure functions ══════════


class TestIntervalToTf:
    def test_1min(self):
        assert _interval_to_tf("1min") == "1m"

    def test_5min(self):
        assert _interval_to_tf("5min") == "5m"

    def test_15min(self):
        assert _interval_to_tf("15min") == "15m"

    def test_30min(self):
        assert _interval_to_tf("30min") == "30m"

    def test_unknown_passthrough(self):
        assert _interval_to_tf("1day") == "1day"


class TestDfToBars:
    def test_basic_conversion(self):
        dates = pd.date_range("2025-01-02 09:30", periods=5, freq="1min")
        df = pd.DataFrame(
            {"open": [1]*5, "high": [2]*5, "low": [0.5]*5, "close": [1.5]*5, "volume": [100.0]*5},
            index=dates,
        )
        bars = _df_to_bars(df)
        assert len(bars) == 5
        assert isinstance(bars[0], Bar)
        assert bars[0].close == 1.5
        assert bars[0].volume == 100.0

    def test_adds_utc_to_naive(self):
        dates = pd.date_range("2025-01-02", periods=3, freq="1min")
        df = pd.DataFrame(
            {"open": [1]*3, "high": [2]*3, "low": [0.5]*3, "close": [1.5]*3},
            index=dates,
        )
        bars = _df_to_bars(df)
        assert bars[0].ts.tzinfo is not None

    def test_no_volume_column(self):
        dates = pd.date_range("2025-01-02", periods=3, freq="1min")
        df = pd.DataFrame(
            {"open": [1]*3, "high": [2]*3, "low": [0.5]*3, "close": [1.5]*3},
            index=dates,
        )
        bars = _df_to_bars(df)
        assert bars[0].volume is None


class TestStrokeToDict:
    def test_basic(self):
        from newchan.a_stroke import Stroke

        s = Stroke(i0=0, i1=5, direction="up", high=110.0, low=100.0, p0=100.0, p1=110.0, confirmed=True)
        d = _stroke_to_dict(s)
        assert d["i0"] == 0
        assert d["i1"] == 5
        assert d["direction"] == "up"
        assert d["confirmed"] is True


class TestEventToWs:
    def test_stroke_settled(self):
        from newchan.events import StrokeSettled

        ev = StrokeSettled(
            bar_idx=0, bar_ts=1000.0, seq=0, event_id="test",
            stroke_id=0, direction="up", i0=0, i1=5, p0=100.0, p1=110.0,
        )
        d = _event_to_ws(ev, tf="1m", stream_id="s1")
        assert d["type"] == "event"
        assert d["event_type"] == "stroke_settled"
        assert d["bar_idx"] == 0
        assert d["tf"] == "1m"
        assert d["stream_id"] == "s1"
        # payload 不应包含已提升的字段
        assert "bar_idx" not in d["payload"]
        assert "event_type" not in d["payload"]
        # payload 应包含 stroke 特有字段
        assert d["payload"]["stroke_id"] == 0
        assert d["payload"]["direction"] == "up"


class TestSnapshotToWs:
    def test_basic(self):
        from newchan.a_stroke import Stroke
        from newchan.bi_engine import BiEngineSnapshot

        s = Stroke(i0=0, i1=5, direction="up", high=110.0, low=100.0, p0=100.0, p1=110.0, confirmed=True)
        snap = BiEngineSnapshot(bar_idx=10, bar_ts=2000.0, strokes=[s], events=[], n_merged=0, n_fractals=0)
        d = _snapshot_to_ws(snap)
        assert d["type"] == "snapshot"
        assert d["bar_idx"] == 10
        assert len(d["strokes"]) == 1
        assert d["strokes"][0]["direction"] == "up"
        assert d["event_count"] == 0


class TestBarToWs:
    def test_basic(self):
        bar = Bar(
            ts=datetime(2025, 1, 2, 9, 30, tzinfo=timezone.utc),
            open=100.0, high=105.0, low=99.0, close=103.0, volume=500.0,
        )
        d = _bar_to_ws(bar, idx=42, tf="1m", stream_id="s1")
        assert d["idx"] == 42
        assert d["o"] == 100.0
        assert d["c"] == 103.0
        assert d["tf"] == "1m"

    def test_naive_datetime(self):
        bar = Bar(
            ts=datetime(2025, 1, 2, 9, 30),
            open=100.0, high=105.0, low=99.0, close=103.0, volume=None,
        )
        d = _bar_to_ws(bar, idx=0)
        assert "ts" in d


# ══════════ b_plot.py ══════════


class TestPlotClose:
    @patch("newchan.b_plot.plt")
    def test_calls_show(self, mock_plt):
        mock_fig = MagicMock()
        mock_ax = MagicMock()
        mock_plt.subplots.return_value = (mock_fig, mock_ax)

        dates = pd.date_range("2025-01-01", periods=10, freq="1D")
        df = pd.DataFrame({"close": np.linspace(100, 110, 10)}, index=dates)

        from newchan.b_plot import plot_close
        plot_close(df, title="Test")
        mock_plt.show.assert_called_once()

    @patch("newchan.b_plot.plt")
    def test_long_span(self, mock_plt):
        mock_fig = MagicMock()
        mock_ax = MagicMock()
        mock_plt.subplots.return_value = (mock_fig, mock_ax)

        dates = pd.date_range("2023-01-01", periods=500, freq="1D")
        df = pd.DataFrame({"close": np.linspace(100, 200, 500)}, index=dates)

        from newchan.b_plot import plot_close
        plot_close(df)  # span > 365 days path
        mock_plt.show.assert_called()
