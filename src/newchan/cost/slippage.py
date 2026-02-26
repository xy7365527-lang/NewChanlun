"""滑点模型 — 三种滑点成本建模。

提供统一接口 apply_slippage(price, side, ...) -> float，
买入方向滑点使成交价上升，卖出方向滑点使成交价下降。
"""

from __future__ import annotations

import math
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Literal


@dataclass(frozen=True, slots=True)
class SlippageBase(ABC):
    """滑点模型基类。"""

    @abstractmethod
    def apply_slippage(
        self,
        price: float,
        side: Literal["buy", "sell"],
        volume: float | None = None,
        avg_daily_volume: float | None = None,
    ) -> float:
        """计算滑点后的成交价。

        Parameters
        ----------
        price : float
            原始价格。
        side : "buy" | "sell"
            交易方向。买入滑点使价格上升，卖出滑点使价格下降。
        volume : float | None
            本笔成交量（仅 MarketImpactSlippage 需要）。
        avg_daily_volume : float | None
            日均成交量（仅 MarketImpactSlippage 需要）。

        Returns
        -------
        float
            滑点后的成交价。
        """


@dataclass(frozen=True, slots=True)
class FixedSlippage(SlippageBase):
    """固定点数滑点。

    Attributes
    ----------
    slippage_points : float
        每笔固定滑点点数（≥ 0）。
    """

    slippage_points: float = 0.0

    def apply(self, price: float, side: str) -> float:
        """Protocol 接口 — CostConfig / BacktestEngine 使用。"""
        return self.apply_slippage(price, side)  # type: ignore[arg-type]

    def apply_slippage(
        self,
        price: float,
        side: Literal["buy", "sell"],
        volume: float | None = None,
        avg_daily_volume: float | None = None,
    ) -> float:
        if side == "buy":
            return price + self.slippage_points
        return price - self.slippage_points


@dataclass(frozen=True, slots=True)
class PercentSlippage(SlippageBase):
    """百分比滑点。

    Attributes
    ----------
    slippage_pct : float
        滑点百分比（0.001 = 0.1%）。
    """

    slippage_pct: float = 0.001

    def apply(self, price: float, side: str) -> float:
        """Protocol 接口 — CostConfig / BacktestEngine 使用。"""
        return self.apply_slippage(price, side)  # type: ignore[arg-type]

    def apply_slippage(
        self,
        price: float,
        side: Literal["buy", "sell"],
        volume: float | None = None,
        avg_daily_volume: float | None = None,
    ) -> float:
        if side == "buy":
            return price * (1.0 + self.slippage_pct)
        return price * (1.0 - self.slippage_pct)


@dataclass(frozen=True, slots=True)
class MarketImpactSlippage(SlippageBase):
    """基于成交量占比的市场冲击滑点。

    仅在本笔成交量超过日均成交量的 threshold_pct 时激活。
    冲击大小 = impact_coeff * sqrt(volume / avg_daily_volume)。

    Attributes
    ----------
    threshold_pct : float
        激活阈值（0.01 = 1%）。成交量占比低于此值时无滑点。
    impact_coeff : float
        冲击系数，控制滑点幅度。
    """

    threshold_pct: float = 0.01
    impact_coeff: float = 0.1

    def apply(self, price: float, side: str) -> float:
        """Protocol 接口 — 无 volume 信息时返回原价。"""
        return self.apply_slippage(price, side)  # type: ignore[arg-type]

    def apply_slippage(
        self,
        price: float,
        side: Literal["buy", "sell"],
        volume: float | None = None,
        avg_daily_volume: float | None = None,
    ) -> float:
        if volume is None or avg_daily_volume is None or avg_daily_volume <= 0:
            return price

        participation_rate = volume / avg_daily_volume
        if participation_rate < self.threshold_pct:
            return price

        impact_pct = self.impact_coeff * math.sqrt(participation_rate)
        if side == "buy":
            return price * (1.0 + impact_pct)
        return price * (1.0 - impact_pct)
