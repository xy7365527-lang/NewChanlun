"""等价对（EquivalencePair）测试。

概念溯源：[旧缠论] 第9课 — 比价关系的变动构成独立买卖系统
         [新缠论] 等价关系严格定义（ratio_relation_v1.md §2）
"""

from __future__ import annotations

import math

import pandas as pd
import pytest

import warnings

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


# ── 三层退化连锁检测（024号谱系）──────────────────────────


def _oscillating(base: float, amp: float, n: int, freq: float = 0.3,
                 start: str = "2024-01-01") -> pd.DataFrame:
    """生成振荡价格序列的 OHLCV。"""
    prices = [base + amp * math.sin(i * freq) for i in range(n)]
    return _ohlcv(prices, start=start)


class TestValidatePairThreeLayer:
    """C-2 三层退化连锁检测（024号谱系）。"""

    def test_valid_pair_all_layers(self):
        """正例：有足够波动和足够数据的 pair 通过三层检测。"""
        df_a = _oscillating(100, 20, 60, freq=0.3)
        df_b = _oscillating(60, 5, 60, freq=0.5)
        result = validate_pair(df_a, df_b)
        assert result.valid is True
        assert result.cv is not None and result.cv > 0.01

    def test_cv_prescreen_rejection(self):
        """比价 CV 极低 → Layer 1 拒绝，不跑管线。"""
        # A 和 B 几乎同步变动 → ratio ≈ 常数
        df_a = _oscillating(100, 0.001, 60, freq=0.3)
        df_b = _oscillating(100, 0.001, 60, freq=0.3)
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert result.cv is not None and result.cv < 0.01
        # Layer 2/3 不运行
        assert result.stroke_mean_pct is None

    def test_short_data_cv_only(self):
        """数据不足 30 bars 时只检查 CV，不跑管线。"""
        df_a = _ohlcv([100, 102, 101, 105, 103])
        df_b = _ohlcv([50, 51, 49, 52, 50])
        result = validate_pair(df_a, df_b)
        assert result.valid is True
        assert result.cv is not None and result.cv > 0.01
        # Layer 2/3 不运行
        assert result.stroke_mean_pct is None
        assert result.macd_norm_hist is None

    def test_diagnostics_populated_on_valid(self):
        """通过三层的 pair 应有完整诊断信息。"""
        df_a = _oscillating(100, 20, 60, freq=0.3)
        df_b = _oscillating(60, 5, 60, freq=0.5)
        result = validate_pair(df_a, df_b)
        assert result.valid is True
        assert result.cv is not None
        # 有足够数据时应运行 Layer 2/3
        if result.n_strokes is not None and result.n_strokes > 0:
            assert result.stroke_mean_pct is not None
            assert result.stroke_mean_pct > 0.005  # > T_stroke_pct

    def test_backward_compat_constant_ratio(self):
        """向后兼容：常数比价仍被拒绝（CV=0 → Layer 1 拒绝）。"""
        df_a = _ohlcv([100, 200, 300, 400, 500])
        df_b = _ohlcv([50, 100, 150, 200, 250])
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        # 新实现用 "degenerate" 描述退化
        assert "degenerate" in result.reason.lower()


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


# ── 子频率聚合构造比价K线 ──────────────────────────────────


def _hourly_ohlcv(prices: list[float], start: str = "2024-01-01") -> pd.DataFrame:
    """从 close 列表生成小时频率 OHLCV。"""
    n = len(prices)
    idx = pd.date_range(start, periods=n, freq="h")
    return pd.DataFrame(
        {
            "open": prices,
            "high": [p + 1 for p in prices],
            "low": [p - 1 for p in prices],
            "close": prices,
            "volume": [100] * n,
        },
        index=idx,
    )


class TestMakeRatioKlineSubFreq:
    """子频率聚合路径测试。"""

    def test_sub_freq_aggregation(self):
        """提供小时子频率数据，聚合到日频。"""
        # 日频目标（2天，用于推断目标频率）
        df_a = _ohlcv([100, 110], start="2024-01-01")
        df_b = _ohlcv([50, 55], start="2024-01-01")

        # 小时子频率数据（每天24小时，共48小时）
        sub_prices_a = [100 + i * 0.5 for i in range(48)]
        sub_prices_b = [50 + i * 0.2 for i in range(48)]
        sub_a = _hourly_ohlcv(sub_prices_a, start="2024-01-01")
        sub_b = _hourly_ohlcv(sub_prices_b, start="2024-01-01")

        ratio = make_ratio_kline(df_a, df_b, sub_a=sub_a, sub_b=sub_b)

        # 应该产出日频K线
        assert len(ratio) == 2
        for col in ["open", "high", "low", "close"]:
            assert col in ratio.columns
        assert "volume" in ratio.columns

    def test_sub_freq_data_is_clipped_to_target_window(self):
        """子频率缓存宽于目标窗口时，不应产出窗口外 K 线。"""
        # 目标日频只请求 01-02 和 01-03。
        df_a = _ohlcv([110, 120], start="2024-01-02")
        df_b = _ohlcv([55, 60], start="2024-01-02")

        # 子频率缓存覆盖 01-01 到 01-04；窗口外数据不能泄漏到输出。
        sub_a = _hourly_ohlcv([100 + i for i in range(96)], start="2024-01-01")
        sub_b = _hourly_ohlcv([50 + i * 0.5 for i in range(96)], start="2024-01-01")

        ratio = make_ratio_kline(df_a, df_b, sub_a=sub_a, sub_b=sub_b)

        assert list(ratio.index) == list(df_a.index)

    def test_sub_freq_clip_preserves_non_midnight_target_index(self):
        """目标日线用收盘时间戳时，裁剪不应静默删除聚合结果。"""
        target_idx = pd.date_range("2024-01-02 16:00", periods=2, freq="D")
        df_a = _ohlcv([110, 120], start="2024-01-02")
        df_b = _ohlcv([55, 60], start="2024-01-02")
        df_a.index = target_idx
        df_b.index = target_idx

        sub_a = _hourly_ohlcv([100 + i for i in range(96)], start="2024-01-01")
        sub_b = _hourly_ohlcv([50 + i * 0.5 for i in range(96)], start="2024-01-01")

        ratio = make_ratio_kline(df_a, df_b, sub_a=sub_a, sub_b=sub_b)

        assert list(ratio.index) == list(target_idx)
        assert len(ratio) == 2

    def test_sub_freq_high_low_from_ratio_series(self):
        """子频率聚合的 high/low 来自 ratio 序列的 max/min，不是 OHLC 各自除法。"""
        df_a = _ohlcv([100, 110], start="2024-01-01")
        df_b = _ohlcv([50, 55], start="2024-01-01")

        # 构造子频率（第一天24小时）：A 在第6小时达峰，B 在第12小时达峰
        sub_a_prices = [100.0] * 24 + [110.0] * 24
        sub_a_prices[6] = 120.0  # A 峰值在第一天
        sub_b_prices = [50.0] * 24 + [55.0] * 24
        sub_b_prices[12] = 70.0  # B 峰值在第一天

        sub_a = _hourly_ohlcv(sub_a_prices, start="2024-01-01")
        sub_b = _hourly_ohlcv(sub_b_prices, start="2024-01-01")

        ratio = make_ratio_kline(df_a, df_b, sub_a=sub_a, sub_b=sub_b)

        # ratio 最高点应出现在 A 高且 B 低的时刻：120/50 = 2.4
        assert ratio["high"].iloc[0] == pytest.approx(120.0 / 50.0)
        # ratio 最低点应出现在 A 低且 B 高的时刻：100/70 ≈ 1.4286
        assert ratio["low"].iloc[0] == pytest.approx(100.0 / 70.0)

    def test_sub_freq_explicit_target_freq(self):
        """显式指定 target_freq 覆盖自动推断。"""
        df_a = _ohlcv([100, 110], start="2024-01-01")
        df_b = _ohlcv([50, 55], start="2024-01-01")

        sub_prices_a = [100 + i for i in range(48)]
        sub_prices_b = [50 + i * 0.5 for i in range(48)]
        sub_a = _hourly_ohlcv(sub_prices_a, start="2024-01-01")
        sub_b = _hourly_ohlcv(sub_prices_b, start="2024-01-01")

        ratio = make_ratio_kline(
            df_a, df_b, sub_a=sub_a, sub_b=sub_b, target_freq="1D",
        )
        assert len(ratio) == 2

    def test_fallback_warns(self):
        """不提供子频率数据时发出 warning。"""
        df_a = _ohlcv([100, 110])
        df_b = _ohlcv([50, 55])
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            make_ratio_kline(df_a, df_b)
            assert len(w) == 1
            assert "naive OHLC division" in str(w[0].message)

    def test_fallback_result_matches_old_behavior(self):
        """Fallback 路径结果与旧行为一致。"""
        df_a = _ohlcv([100, 200, 150])
        df_b = _ohlcv([50, 100, 50])
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            ratio = make_ratio_kline(df_a, df_b)
        assert list(ratio["close"]) == [2.0, 2.0, 3.0]
