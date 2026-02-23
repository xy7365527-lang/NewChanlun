"""等价关系 C-3 流动性条件测试。

定义依据（dengjia.md #13 §等价对）：
  条件3. 流动性 — A 和 B 都有充足的市场深度
  （否则比价反映的不是资本流转，而是流动性噪音）

审计缺口（audit-cross-3.md §2 MEDIUM）：
  等价关系中 C-3 流动性条件无显式实现。
  三层退化检测可间接过滤部分低流动性标的，但并不等价于流动性验证。
"""

from __future__ import annotations

import math

import pandas as pd
import pytest

from newchan.equivalence import (
    ValidationResult,
    validate_pair,
)


# ── 测试数据工厂 ─────────────────────────────────────────


def _ohlcv(
    prices: list[float],
    volumes: list[int] | None = None,
    start: str = "2024-01-01",
) -> pd.DataFrame:
    """从 close 列表生成 OHLCV，volume 可独立指定。"""
    n = len(prices)
    if volumes is None:
        volumes = [1000] * n
    idx = pd.date_range(start, periods=n, freq="D")
    return pd.DataFrame(
        {
            "open": prices,
            "high": [p + 1 for p in prices],
            "low": [p - 1 for p in prices],
            "close": prices,
            "volume": volumes,
        },
        index=idx,
    )


def _oscillating(
    base: float,
    amp: float,
    n: int,
    freq: float = 0.3,
    volumes: list[int] | None = None,
    start: str = "2024-01-01",
) -> pd.DataFrame:
    """生成振荡价格序列的 OHLCV，volume 可独立指定。"""
    prices = [base + amp * math.sin(i * freq) for i in range(n)]
    return _ohlcv(prices, volumes=volumes, start=start)


# ── C-3 流动性条件 ───────────────────────────────────────


class TestLiquidityCondition:
    """C-3 流动性：A 和 B 都有充足的市场深度。"""

    def test_normal_volume_passes(self) -> None:
        """正例：两个标的都有正常成交量 → 流动性通过。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[1000, 1200, 800, 1500, 900],
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[2000, 1800, 2200, 1900, 2100],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is True

    def test_zero_volume_a_fails(self) -> None:
        """A 的成交量全为 0 → 流动性不足。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[0, 0, 0, 0, 0],
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[1000, 1200, 800, 1500, 900],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "liquidity" in result.reason.lower()

    def test_zero_volume_b_fails(self) -> None:
        """B 的成交量全为 0 → 流动性不足。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[1000, 1200, 800, 1500, 900],
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[0, 0, 0, 0, 0],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "liquidity" in result.reason.lower()

    def test_both_zero_volume_fails(self) -> None:
        """A 和 B 的成交量都为 0 → 流动性不足。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[0, 0, 0, 0, 0],
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[0, 0, 0, 0, 0],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "liquidity" in result.reason.lower()

    def test_mostly_zero_volume_fails(self) -> None:
        """大部分K线成交量为 0（> 50%）→ 流动性不足。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[1000, 0, 0, 0, 0],  # 80% 零成交量
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[2000, 1800, 2200, 1900, 2100],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is False
        assert "liquidity" in result.reason.lower()

    def test_sparse_volume_passes(self) -> None:
        """少量K线成交量为 0（< 50%）但整体有流动性 → 通过。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[1000, 0, 800, 1500, 900],  # 20% 零成交量
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[2000, 1800, 2200, 1900, 2100],
        )
        result = validate_pair(df_a, df_b)
        assert result.valid is True

    def test_no_volume_column_skips_check(self) -> None:
        """如果没有 volume 列（如汇率数据），跳过流动性检查。

        边界条件（dengjia.md）：汇率类天然满足等价对条件。
        """
        df_a = _ohlcv([100, 102, 101, 105, 103])
        df_b = _ohlcv([50, 51, 49, 52, 50])
        # 移除 volume 列
        df_a = df_a.drop(columns=["volume"])
        df_b = df_b.drop(columns=["volume"])
        result = validate_pair(df_a, df_b)
        # 不应因流动性失败（汇率类天然满足）
        # 可能因其他条件通过或失败，但不应报 liquidity 错误
        if not result.valid:
            assert "liquidity" not in result.reason.lower()

    def test_liquidity_threshold_customizable(self) -> None:
        """流动性阈值可通过参数定制。"""
        df_a = _ohlcv(
            [100, 102, 101, 105, 103],
            volumes=[0, 0, 0, 0, 100],  # 80% 零成交量
        )
        df_b = _ohlcv(
            [50, 51, 49, 52, 50],
            volumes=[2000, 1800, 2200, 1900, 2100],
        )
        # 严格阈值：应该失败
        result_strict = validate_pair(df_a, df_b, t_liquidity=0.5)
        assert result_strict.valid is False

        # 宽松阈值：允许 90% 零成交量
        result_lenient = validate_pair(df_a, df_b, t_liquidity=0.9)
        assert result_lenient.valid is True
