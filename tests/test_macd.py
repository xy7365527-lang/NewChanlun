"""MACD 指标 — 单元测试

测试项：
  compute_macd:
    A) 正常计算：验证 EMA 逻辑及 hist = macd - signal
    B) 短序列：仅 1 条数据
    C) 自定义周期参数
    D) 输出列与索引一致性

  macd_area_for_range:
    E) 正常区间面积计算
    F) 全正 hist / 全负 hist
    G) 空区间（i0 > i1）
    H) 边界裁剪（负索引、超长索引）
    I) 单 bar 区间
"""

import pytest
import pandas as pd

from newchan.a_macd import compute_macd, macd_area_for_range


# ── helpers ──

def _make_close_df(prices: list) -> pd.DataFrame:
    """从价格列表构造含 close 列的 DataFrame。"""
    return pd.DataFrame({"close": prices})


# ── compute_macd ──

class TestComputeMacd:
    """compute_macd 函数测试组。"""

    def test_normal_calculation(self):
        """正常序列：hist = macd - signal，且各列长度正确。"""
        prices = [10.0, 11.0, 12.0, 11.5, 13.0,
                  14.0, 13.5, 15.0, 14.5, 16.0,
                  15.5, 17.0, 16.5, 18.0, 17.5,
                  19.0, 18.5, 20.0, 19.5, 21.0,
                  20.5, 22.0, 21.5, 23.0, 22.5,
                  24.0, 23.5, 25.0, 24.5, 26.0]
        df = _make_close_df(prices)

        result = compute_macd(df)

        assert list(result.columns) == ["macd", "signal", "hist"]
        assert len(result) == len(df)
        # hist = macd - signal 对所有行成立
        for i in range(len(result)):
            assert result["hist"].iloc[i] == pytest.approx(
                result["macd"].iloc[i] - result["signal"].iloc[i]
            )

    def test_single_row(self):
        """仅 1 条数据：EMA 退化为原值，macd/signal/hist 均有值。"""
        df = _make_close_df([100.0])

        result = compute_macd(df)

        assert len(result) == 1
        # 单条数据：fast EMA = slow EMA = close → macd = 0
        assert result["macd"].iloc[0] == pytest.approx(0.0)
        assert result["signal"].iloc[0] == pytest.approx(0.0)
        assert result["hist"].iloc[0] == pytest.approx(0.0)

    def test_two_rows(self):
        """2 条数据：验证结果存在且 hist 一致性。"""
        df = _make_close_df([50.0, 55.0])

        result = compute_macd(df)

        assert len(result) == 2
        for i in range(len(result)):
            assert result["hist"].iloc[i] == pytest.approx(
                result["macd"].iloc[i] - result["signal"].iloc[i]
            )

    def test_custom_periods(self):
        """自定义 fast/slow/signal 参数不报错且输出正确结构。"""
        prices = list(range(1, 51))
        df = _make_close_df([float(p) for p in prices])

        result = compute_macd(df, fast=5, slow=10, signal=3)

        assert list(result.columns) == ["macd", "signal", "hist"]
        assert len(result) == 50

    def test_index_preserved(self):
        """输出索引与输入索引一致。"""
        idx = pd.date_range("2024-01-01", periods=10, freq="D")
        df = pd.DataFrame({"close": range(10, 20)}, index=idx)

        result = compute_macd(df)

        assert result.index.equals(idx)

    def test_constant_prices(self):
        """常量价格序列：macd/signal/hist 全为 0。"""
        df = _make_close_df([42.0] * 30)

        result = compute_macd(df)

        for col in ["macd", "signal", "hist"]:
            for v in result[col]:
                assert v == pytest.approx(0.0)

    def test_monotonic_up_macd_positive(self):
        """单调上升序列：fast EMA > slow EMA → macd 最终为正。"""
        prices = [float(i) for i in range(1, 31)]
        df = _make_close_df(prices)

        result = compute_macd(df)

        # 第 30 条时 macd 应当为正（fast EMA 更贴近最新高价）
        assert result["macd"].iloc[-1] > 0


# ── macd_area_for_range ──

class TestMacdAreaForRange:
    """macd_area_for_range 函数测试组。"""

    @pytest.fixture()
    def sample_macd_df(self) -> pd.DataFrame:
        """构造一个已知 hist 的 DataFrame 用于面积测试。"""
        hist_values = [1.0, -2.0, 3.0, -1.0, 0.5]
        return pd.DataFrame({
            "macd": [0.0] * 5,
            "signal": [0.0] * 5,
            "hist": hist_values,
        })

    def test_full_range(self, sample_macd_df: pd.DataFrame):
        """完整范围 [0, 4]：面积计算正确。"""
        result = macd_area_for_range(sample_macd_df, 0, 4)

        # hist = [1, -2, 3, -1, 0.5]
        assert result["area_total"] == pytest.approx(1.5)
        assert result["area_pos"] == pytest.approx(4.5)     # 1 + 3 + 0.5
        assert result["area_neg"] == pytest.approx(-3.0)    # -2 + -1
        assert result["n_bars"] == 5

    def test_sub_range(self, sample_macd_df: pd.DataFrame):
        """子区间 [1, 3]。"""
        result = macd_area_for_range(sample_macd_df, 1, 3)

        # hist[1:4] = [-2, 3, -1]
        assert result["area_total"] == pytest.approx(0.0)
        assert result["area_pos"] == pytest.approx(3.0)
        assert result["area_neg"] == pytest.approx(-3.0)
        assert result["n_bars"] == 3

    def test_single_bar(self, sample_macd_df: pd.DataFrame):
        """单 bar 区间 [2, 2]。"""
        result = macd_area_for_range(sample_macd_df, 2, 2)

        assert result["area_total"] == pytest.approx(3.0)
        assert result["area_pos"] == pytest.approx(3.0)
        assert result["area_neg"] == pytest.approx(0.0)
        assert result["n_bars"] == 1

    def test_empty_range_i0_gt_i1(self, sample_macd_df: pd.DataFrame):
        """空区间：i0 > i1 → 全零返回。"""
        result = macd_area_for_range(sample_macd_df, 3, 1)

        assert result["area_total"] == pytest.approx(0.0)
        assert result["area_pos"] == pytest.approx(0.0)
        assert result["area_neg"] == pytest.approx(0.0)
        assert result["n_bars"] == 0

    def test_negative_i0_clipped(self, sample_macd_df: pd.DataFrame):
        """负 i0 裁剪为 0。"""
        result = macd_area_for_range(sample_macd_df, -5, 2)

        # 等效 [0, 2]，hist = [1, -2, 3]
        assert result["area_total"] == pytest.approx(2.0)
        assert result["area_pos"] == pytest.approx(4.0)
        assert result["area_neg"] == pytest.approx(-2.0)
        assert result["n_bars"] == 3

    def test_i1_exceeds_length_clipped(self, sample_macd_df: pd.DataFrame):
        """i1 超出长度裁剪为 len-1。"""
        result = macd_area_for_range(sample_macd_df, 3, 100)

        # 等效 [3, 4]，hist = [-1, 0.5]
        assert result["area_total"] == pytest.approx(-0.5)
        assert result["area_pos"] == pytest.approx(0.5)
        assert result["area_neg"] == pytest.approx(-1.0)
        assert result["n_bars"] == 2

    def test_all_positive_hist(self):
        """全正 hist：area_neg = 0。"""
        df = pd.DataFrame({"hist": [1.0, 2.0, 3.0]})

        result = macd_area_for_range(df, 0, 2)

        assert result["area_total"] == pytest.approx(6.0)
        assert result["area_pos"] == pytest.approx(6.0)
        assert result["area_neg"] == pytest.approx(0.0)

    def test_all_negative_hist(self):
        """全负 hist：area_pos = 0。"""
        df = pd.DataFrame({"hist": [-1.0, -2.0, -3.0]})

        result = macd_area_for_range(df, 0, 2)

        assert result["area_total"] == pytest.approx(-6.0)
        assert result["area_pos"] == pytest.approx(0.0)
        assert result["area_neg"] == pytest.approx(-6.0)

    def test_result_rounded_to_six_decimals(self):
        """结果四舍五入到 6 位小数。"""
        df = pd.DataFrame({"hist": [1.0 / 3.0]})

        result = macd_area_for_range(df, 0, 0)

        assert result["area_total"] == pytest.approx(round(1.0 / 3.0, 6))
        assert result["area_pos"] == pytest.approx(round(1.0 / 3.0, 6))
