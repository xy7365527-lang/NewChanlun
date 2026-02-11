"""A→B 桥接 overlay — 单元测试 (Step15)

构造小 df_raw（DateTimeIndex + OHLC），调 build_overlay_newchan，
验证输出 schema 正确性。不依赖真实行情数据。
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.ab_bridge_newchan import build_overlay_newchan
from newchan.a_macd import compute_macd, macd_area_for_range


# ── 测试数据 ──

def _make_zigzag_df(n: int = 40) -> pd.DataFrame:
    """构造锯齿 OHLC，足以产生分型→笔→(可能)段。

    20 根下行 + 20 根上行 的 V 形，重复两次的半周期。
    """
    dates = pd.date_range("2025-01-01", periods=n, freq="h")
    highs, lows = [], []
    for i in range(n):
        cycle = i % 20
        if cycle < 10:
            # 下行段
            h = 100 - cycle * 3
            l = h - 5
        else:
            # 上行段
            h = 70 + (cycle - 10) * 3
            l = h - 5
        highs.append(float(h))
        lows.append(float(l))
    opens = [(h + l) / 2 for h, l in zip(highs, lows)]
    closes = [(h + l) / 2 for h, l in zip(highs, lows)]
    return pd.DataFrame(
        {"open": opens, "high": highs, "low": lows, "close": closes},
        index=dates,
    )


# =====================================================================
# schema 基础测试
# =====================================================================

class TestOverlaySchema:
    """验证输出 schema 结构。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_zigzag_df()
        self.result_full = build_overlay_newchan(
            self.df, symbol="TEST", tf="1h", detail="full",
        )
        self.result_min = build_overlay_newchan(
            self.df, symbol="TEST", tf="1h", detail="min",
        )

    def test_schema_version(self):
        assert self.result_full["schema_version"] == "newchan_overlay_v2"
        assert self.result_min["schema_version"] == "newchan_overlay_v2"

    def test_top_level_keys(self):
        expected_keys = {
            "schema_version", "symbol", "tf", "detail",
            "lstar", "strokes", "segments", "centers", "trends", "levels", "macd",
        }
        assert set(self.result_full.keys()) == expected_keys

    def test_symbol_tf(self):
        assert self.result_full["symbol"] == "TEST"
        assert self.result_full["tf"] == "1h"

    def test_strokes_is_list(self):
        assert isinstance(self.result_full["strokes"], list)

    def test_segments_is_list(self):
        assert isinstance(self.result_full["segments"], list)

    def test_centers_is_list(self):
        assert isinstance(self.result_full["centers"], list)

    def test_trends_is_list(self):
        assert isinstance(self.result_full["trends"], list)


# =====================================================================
# MACD 输出
# =====================================================================

class TestMACDOutput:
    """MACD series 输出验证。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_zigzag_df()
        self.result = build_overlay_newchan(self.df, symbol="TEST", tf="1h")

    def test_macd_keys(self):
        macd = self.result["macd"]
        assert "fast" in macd
        assert "slow" in macd
        assert "signal" in macd
        assert "series" in macd

    def test_macd_series_non_empty(self):
        series = self.result["macd"]["series"]
        assert len(series) > 0

    def test_macd_series_has_time(self):
        series = self.result["macd"]["series"]
        assert "time" in series[0]

    def test_macd_series_monotonic_time(self):
        series = self.result["macd"]["series"]
        times = [s["time"] for s in series]
        assert times == sorted(times)

    def test_macd_series_has_hist(self):
        series = self.result["macd"]["series"]
        assert "hist" in series[0]


# =====================================================================
# detail=min vs full
# =====================================================================

class TestDetailLevel:
    """detail=min 时 anchors=null；detail=full 时 anchors 可能为 dict。"""

    def test_detail_min_anchors_null(self):
        df = _make_zigzag_df()
        result = build_overlay_newchan(df, detail="min")
        lstar = result["lstar"]
        if lstar is not None:
            assert lstar["anchors"] is None

    def test_detail_full_anchors_dict_or_none(self):
        df = _make_zigzag_df()
        result = build_overlay_newchan(df, detail="full")
        lstar = result["lstar"]
        if lstar is not None:
            # full → anchors 为 dict（当 lstar 存在时）
            assert isinstance(lstar["anchors"], dict)


# =====================================================================
# stroke/segment 对象结构
# =====================================================================

class TestObjectStructure:
    """验证每个对象有必要字段。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_zigzag_df()
        self.result = build_overlay_newchan(self.df, symbol="TEST", tf="1h")

    def test_stroke_fields(self):
        for s in self.result["strokes"]:
            assert "id" in s
            assert "t0" in s
            assert "t1" in s
            assert "dir" in s
            assert "confirmed" in s
            assert "macd_area_total" in s

    def test_segment_fields(self):
        for seg in self.result["segments"]:
            assert "id" in seg
            assert "t0" in seg
            assert "t1" in seg
            assert "dir" in seg
            assert "ep0" in seg
            assert "ep1" in seg
            assert "macd_area_total" in seg

    def test_segment_endpoint_no_bridge_override(self):
        """桥接层只映射端点语义：t/p 必须与 ep 对应字段一致。"""
        for seg in self.result["segments"]:
            assert seg["t0"] == seg["ep0"]["time"]
            assert seg["t1"] == seg["ep1"]["time"]
            assert seg["p0"] == pytest.approx(seg["ep0"]["price"])
            assert seg["p1"] == pytest.approx(seg["ep1"]["price"])

    def test_center_fields(self):
        for c in self.result["centers"]:
            assert "id" in c
            assert "ZD" in c
            assert "ZG" in c
            assert "kind" in c
            assert "macd_area_total" in c


# =====================================================================
# v0 vs v1 算法选择
# =====================================================================

class TestAlgoSelection:
    """segment_algo 参数切换。"""

    def test_v0_runs(self):
        df = _make_zigzag_df()
        result = build_overlay_newchan(df, segment_algo="v0")
        assert result["schema_version"] == "newchan_overlay_v2"

    def test_v1_runs(self):
        df = _make_zigzag_df()
        result = build_overlay_newchan(df, segment_algo="v1")
        assert result["schema_version"] == "newchan_overlay_v2"


# =====================================================================
# 边界情况
# =====================================================================

class TestEdgeCases:

    def test_empty_df(self):
        df = pd.DataFrame(columns=["open", "high", "low", "close"])
        result = build_overlay_newchan(df)
        assert result["schema_version"] == "newchan_overlay_v2"
        assert result["strokes"] == []
        assert result["macd"]["series"] == []

    def test_tiny_df(self):
        """2 根 bar → 不足以产生任何结构。"""
        dates = pd.date_range("2025-01-01", periods=2, freq="h")
        df = pd.DataFrame(
            {"open": [10, 12], "high": [15, 14], "low": [8, 10], "close": [12, 13]},
            index=dates,
        )
        result = build_overlay_newchan(df)
        assert result["schema_version"] == "newchan_overlay_v2"
        assert result["strokes"] == []


# =====================================================================
# MACD 面积计算
# =====================================================================

class TestMACDArea:

    def test_compute_macd_columns(self):
        df = _make_zigzag_df()
        macd_df = compute_macd(df)
        assert "macd" in macd_df.columns
        assert "signal" in macd_df.columns
        assert "hist" in macd_df.columns
        assert len(macd_df) == len(df)

    def test_area_for_range_basic(self):
        df = _make_zigzag_df()
        macd_df = compute_macd(df)
        area = macd_area_for_range(macd_df, 0, 10)
        assert "area_total" in area
        assert "area_pos" in area
        assert "area_neg" in area
        assert "n_bars" in area
        assert area["n_bars"] == 11

    def test_area_empty_range(self):
        df = _make_zigzag_df()
        macd_df = compute_macd(df)
        area = macd_area_for_range(macd_df, 10, 5)  # inverted
        assert area["n_bars"] == 0

    def test_area_pos_neg_sum(self):
        """area_total ≈ area_pos + area_neg。"""
        df = _make_zigzag_df()
        macd_df = compute_macd(df)
        area = macd_area_for_range(macd_df, 5, 35)
        assert abs(area["area_total"] - (area["area_pos"] + area["area_neg"])) < 1e-4
