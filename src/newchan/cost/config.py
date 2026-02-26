"""成本模型配置 — Protocol 接口 + CostConfig dataclass。"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol, runtime_checkable


@runtime_checkable
class SlippageModel(Protocol):
    """滑点模型接口。

    Parameters
    ----------
    price : float
        理论成交价。
    side : str
        "buy" 或 "sell"。

    Returns
    -------
    float
        滑点后的实际成交价。
    """

    def apply(self, price: float, side: str) -> float: ...


@runtime_checkable
class CommissionModel(Protocol):
    """手续费模型接口。

    Parameters
    ----------
    price : float
        成交价。
    quantity : float
        成交数量。

    Returns
    -------
    float
        手续费金额（非负）。
    """

    def calculate(self, price: float, quantity: float) -> float: ...


@dataclass(frozen=True, slots=True)
class CostConfig:
    """成本配置。

    Attributes
    ----------
    slippage_model : SlippageModel | None
        滑点模型。None = 不计算滑点。
    commission_model : CommissionModel | None
        手续费模型。None = 不计算手续费。
    quantity : float
        每笔交易数量（用于手续费计算）。默认 1.0。
    """

    slippage_model: SlippageModel | None = None
    commission_model: CommissionModel | None = None
    quantity: float = 1.0
