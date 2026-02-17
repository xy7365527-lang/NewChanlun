"""server.py 纯函数单元测试。"""
from __future__ import annotations

from unittest.mock import patch, MagicMock
import json

import numpy as np
import pandas as pd
import pytest

from newchan.server import _df_to_records


class TestDfToRecords:
    def test_basic_conversion(self):
        dates = pd.date_range("2025-01-02 09:30", periods=3, freq="1min")
        df = pd.DataFrame(
            {"open": [100.0, 101.0, 102.0], "high": [101.0, 102.0, 103.0],
             "low": [99.0, 100.0, 101.0], "close": [100.5, 101.5, 102.5],
             "volume": [1000.0, 1100.0, 1200.0]},
            index=dates,
        )
        records = _df_to_records(df)
        assert len(records) == 3
        assert "time" in records[0]
        assert records[0]["open"] == 100.0
        assert records[0]["close"] == 100.5

    def test_time_format(self):
        dates = pd.date_range("2025-01-02 09:30:15", periods=1, freq="1min")
        df = pd.DataFrame({"close": [100.0]}, index=dates)
        records = _df_to_records(df)
        assert "2025-01-02 09:30:15" in records[0]["time"]

    def test_tz_aware_converted_to_utc(self):
        dates = pd.date_range("2025-01-02 09:30", periods=2, freq="1min", tz="UTC")
        df = pd.DataFrame(
            {"open": [100.0, 101.0], "close": [100.5, 101.5]},
            index=dates,
        )
        records = _df_to_records(df)
        assert len(records) == 2
        # UTC tz should be stripped but time preserved
        assert "09:30" in records[0]["time"]

    def test_nan_values_skipped(self):
        dates = pd.date_range("2025-01-02", periods=2, freq="1min")
        df = pd.DataFrame(
            {"open": [100.0, np.nan], "close": [100.5, 101.5]},
            index=dates,
        )
        records = _df_to_records(df)
        assert "open" in records[0]
        assert "open" not in records[1]  # NaN skipped

    def test_float_rounding(self):
        dates = pd.date_range("2025-01-02", periods=1, freq="1min")
        df = pd.DataFrame({"close": [100.123456789]}, index=dates)
        records = _df_to_records(df)
        assert records[0]["close"] == 100.123457  # rounded to 6 dp

    def test_tz_naive_kept(self):
        dates = pd.date_range("2025-01-02 09:30", periods=2, freq="1min")
        df = pd.DataFrame({"close": [100.0, 101.0]}, index=dates)
        records = _df_to_records(df)
        assert "09:30" in records[0]["time"]

    def test_empty_dataframe(self):
        df = pd.DataFrame(columns=["open", "close"])
        df.index = pd.DatetimeIndex([])
        records = _df_to_records(df)
        assert records == []


class TestJsonResp:
    @patch("newchan.server.response")
    def test_returns_json(self, mock_response):
        from newchan.server import _json_resp
        result = _json_resp({"key": "value"})
        data = json.loads(result)
        assert data["key"] == "value"
        assert mock_response.content_type == "application/json"
        assert mock_response.status == 200

    @patch("newchan.server.response")
    def test_custom_status(self, mock_response):
        from newchan.server import _json_resp
        _json_resp({"error": "not found"}, status=404)
        assert mock_response.status == 404

    @patch("newchan.server.response")
    def test_chinese_chars(self, mock_response):
        from newchan.server import _json_resp
        result = _json_resp({"msg": "中文测试"})
        assert "中文测试" in result
