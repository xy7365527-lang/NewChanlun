"""等价对（EquivalencePair）测试。

概念溯源：[旧缠论] 第9课 — 比价关系的变动构成独立买卖系统
         [新缠论] 等价关系严格定义（ratio_relation_v1.md §2）
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.equivalence import EquivalencePair, validate_pair, make_ratio_kline


# ── 测试数据工厂 ─────────────────────────────────────────


def _ohlcv(prices: list[float], start: str = "2024-01-01") -> pd.DataFrame:
    """从 close 列表生成简单 OHLCV（open=close, high=close+1, low=close-1）。"""
    n = len(prices)
    idx = pd.date_range(start, periods=n, freq="D")
    return pd.DataFrame(
        {
            "open": prices,
            "high": [p + 1 for p in prices],
            "low": [p - 1 for p in prices],
            "close": prices,
            "volume": [1000] * n,
        },
        index=idx,
    )


# ── EquivalencePair 不可变性 ─────────────────────────────


class TestEquivalencePairImmutable:
    def test_frozen(self):
        pair = EquivalencePair(sym_a="GLD", sym_b="SLV", category="substitute")
        with pytest.raises(AttributeError):
            pair.sym_a = "SPY"  # type: ignore[misc]

    def test_fields(self):
        pair = EquivalencePair(sym_a="SPY", sym_b="TLT", category="macro_asset")
        assert pair.sym_a == "SPY"
        assert pair.sym_b == "TLT"
        assert pair.category == "macro_asset"

    def test_label(self):
        pair = EquivalencePair(sym_a="GLD", sym_b="SLV")
        assert pair.label == "GLD/SLV"


# ── 等价对验证（§2.1 三个条件）───────────────────────────


class TestValidatePair:
    def test_valid_pair(self):
        df_a = _ohlcv([100, 102, 101, 105, 103])
        df_b = _ohlcv([50, 51, 49, 52, 50])
        result = validate_pair(df_a, df_b)
        assert result.valid is True

    def test_no_overlap_fails(self):
        """可比性条件：必须有重叠时间窗口。"""
        df_a = _ohlcv([100, 102], start="2024-01-01")
        df_b = _ohlcv([50, 51], start="2025-01-01")
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "overlap" in result.reason.lower()

    def test_constant_ratio_fails(self):
        """非退化条件：比价不能是常数。"""
        df_a = _ohlcv([100, 200, 300, 400, 500])
        df_b = _ohlcv([50, 100, 150, 200, 250])  # 完美2x
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "constant" in result.reason.lower() or "degenerate" in result.reason.lower()

    def test_zero_price_fails(self):
        """B 价格为 0 时不可比。"""
        df_a = _ohlcv([100, 102, 101])
        df_b = _ohlcv([50, 0, 49])
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "zero" in result.reason.lower()

    def test_insufficient_data_fails(self):
        """数据量不足（< 5 根K线重叠）。"""
        df_a = _ohlcv([100, 102])
        df_b = _ohlcv([50, 51])
        result = validate_pair(df_a, df_b)
        assert result.valid is False


# ── 比价K线构造 ──────────────────────────────────────────


class TestMakeRatioKline:
    def test_basic_ratio(self):
        """A/B 除法结果正确。"""
        df_a = _ohlcv([100, 200, 150])
        df_b = _ohlcv([50, 100, 50])
        ratio = make_ratio_kline(df_a, df_b)
        assert list(ratio["close"]) == [2.0, 2.0, 3.0]

    def test_symmetry_ir1(self):
        """IR-1：A/B 上涨 ⟺ B/A 下跌。"""
        df_a = _ohlcv([100, 110, 120])
        df_b = _ohlcv([100, 100, 100])
        ratio_ab = make_ratio_kline(df_a, df_b)
        ratio_ba = make_ratio_kline(df_b, df_a)
        # A/B close 递增
        assert ratio_ab["close"].iloc[-1] > ratio_ab["close"].iloc[0]
        # B/A close 递减
        assert ratio_ba["close"].iloc[-1] < ratio_ba["close"].iloc[0]

    def test_independence_ir2(self):
        """IR-2：比价走势独立于各自走势。A涨+B涨更快 → 比价跌。"""
        df_a = _ohlcv([100, 110, 120])  # A 涨 20%
        df_b = _ohlcv([100, 120, 150])  # B 涨 50%
        ratio = make_ratio_kline(df_a, df_b)
        assert ratio["close"].iloc[-1] < ratio["close"].iloc[0]

    def test_alignment(self):
        """不同长度的序列自动对齐（inner join）。"""
        df_a = _ohlcv([100, 110, 120, 130], start="2024-01-01")
        df_b = _ohlcv([50, 55, 60], start="2024-01-02")
        ratio = make_ratio_kline(df_a, df_b)
        assert len(ratio) == 3  # 01-02, 01-03, 01-04 重叠

    def test_ohlcv_columns(self):
        """输出包含标准 OHLCV 列。"""
        df_a = _ohlcv([100, 110])
        df_b = _ohlcv([50, 55])
        ratio = make_ratio_kline(df_a, df_b)
        for col in ["open", "high", "low", "close", "volume"]:
            assert col in ratio.columns
