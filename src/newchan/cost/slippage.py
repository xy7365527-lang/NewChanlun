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
    """平方根律市场冲击滑点（#920 修复后）。

    冲击幅度 = ``y * sigma_daily * (volume / avg_daily_volume) ** delta``。

    与旧版的两处差异（#920）：
    1. **删除 threshold_pct 死区**——旧版在参与率 < 1% 时冲击恒 0，与平方根律
       **方向相反**：定律的核心主张恰是小单的边际冲击异常地大、在 Q^{-1/2} 处发散
       （Tóth et al. 2011, Phys. Rev. X 1, 021006, p.3）。本仓各容量级实测参与率
       （0.090% / 0.261% / 0.786% / 4.95%）最关心的三级全部落在旧死区里。
    2. **impact_coeff 拆成两个显式量纲**——旧版单系数 0.1 数学上等于 ``y · sigma_daily``，
       相对文献标定偏大约 4.4 倍（0.9 × 0.02531 ≈ 0.0228）。现在系数随标的自变，
       不必每换品种重填。

    Attributes
    ----------
    y : float
        无量纲冲击系数，默认 0.9——文献一手标定：Donier & Bonart 2015
        （"A Million Metaorder Analysis of Market Impact on the Bitcoin"），
        MtGox 全量约 1300 万笔成交重建 100 万 metaorder，比特币市场。
    sigma_daily : float
        标的日波动率（小数，0.025 即 2.5%/日）。**须由调用方从 OHLCV 实算后传入**
        （如 Garman-Klass 口径）。默认 0.0 = 无波动率信息 ⟹ 冲击为 0（诚实无信息，
        不臆造默认波动率）。
    delta : float
        冲击指数，默认 0.5（平方根律）；文献带 0.4–0.7。
    """

    y: float = 0.9
    sigma_daily: float = 0.0
    delta: float = 0.5

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
        impact_pct = self.y * self.sigma_daily * participation_rate**self.delta
        if side == "buy":
            return price * (1.0 + impact_pct)
        return price * (1.0 - impact_pct)
