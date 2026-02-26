"""手续费模型 — 固定费率 / 分档费率。

两种使用方式：
- 通过 CostConfig Protocol 接口：calculate(price, quantity) -> float
- 直接调用丰富接口：calculate_commission(trade_value, monthly_volume=None) -> float
"""

from __future__ import annotations

from dataclasses import dataclass


# -- 市场默认预设 --

MARKET_PRESETS: dict[str, float] = {
    "a_stock": 0.0003,   # A股：万三
    "us_stock": 0.0001,  # 美股：万一
    "futures": 0.00005,  # 期货：万0.5
}

TIERED_PRESETS: dict[str, list[tuple[float, float]]] = {
    "a_stock": [
        (1_000_000, 0.0003),
        (10_000_000, 0.00025),
        (float("inf"), 0.0002),
    ],
    "us_stock": [
        (500_000, 0.0001),
        (5_000_000, 0.00008),
        (float("inf"), 0.00005),
    ],
    "futures": [
        (2_000_000, 0.00005),
        (20_000_000, 0.00004),
        (float("inf"), 0.00003),
    ],
}


@dataclass(frozen=True, slots=True)
class FixedRateCommission:
    """固定费率手续费：成交金额 * rate。

    满足 config.CommissionModel Protocol（calculate 方法）。

    Parameters
    ----------
    rate : float
        费率，如 0.0003 表示万三。
    """

    rate: float = MARKET_PRESETS["a_stock"]

    def calculate(self, price: float, quantity: float) -> float:
        """Protocol 接口 — CostConfig 使用。"""
        return self.calculate_commission(price * quantity)

    def calculate_commission(
        self, trade_value: float, monthly_volume: float | None = None,
    ) -> float:
        """计算手续费。

        Parameters
        ----------
        trade_value : float
            本笔成交金额。
        monthly_volume : float | None
            固定费率模型忽略此参数。

        Returns
        -------
        float
            手续费金额（>= 0）。
        """
        if trade_value < 0:
            raise ValueError(f"trade_value must be >= 0, got {trade_value}")
        return trade_value * self.rate


@dataclass(frozen=True, slots=True)
class TieredCommission:
    """分档费率手续费：按月交易量匹配费率档位。

    满足 config.CommissionModel Protocol（calculate 方法）。

    tiers 是 (volume_threshold, rate) 的升序列表。
    月交易量 < 第一档阈值 → 用第一档费率；
    月交易量 >= 第 i 档阈值但 < 第 i+1 档阈值 → 用第 i+1 档费率。

    Parameters
    ----------
    tiers : tuple[tuple[float, float], ...]
        分档结构，每项为 (volume_threshold, rate)。必须按 threshold 升序排列。
    monthly_volume : float
        当月累计交易量，用于 calculate() Protocol 调用。默认 0。
    """

    tiers: tuple[tuple[float, float], ...] = tuple(
        (t, r) for t, r in TIERED_PRESETS["a_stock"]
    )
    monthly_volume: float = 0.0

    def __post_init__(self) -> None:
        if not self.tiers:
            raise ValueError("tiers must not be empty")
        prev = -1.0
        for threshold, rate in self.tiers:
            if threshold <= prev:
                raise ValueError(
                    f"tiers must be in ascending threshold order, "
                    f"got {threshold} after {prev}"
                )
            if rate < 0:
                raise ValueError(f"rate must be >= 0, got {rate}")
            prev = threshold

    def calculate(self, price: float, quantity: float) -> float:
        """Protocol 接口 — CostConfig 使用。使用实例的 monthly_volume。"""
        return self.calculate_commission(price * quantity, self.monthly_volume)

    def calculate_commission(
        self, trade_value: float, monthly_volume: float | None = None,
    ) -> float:
        """计算手续费。

        Parameters
        ----------
        trade_value : float
            本笔成交金额。
        monthly_volume : float | None
            当月累计交易量（金额）。None 视为 0。

        Returns
        -------
        float
            手续费金额（>= 0）。
        """
        if trade_value < 0:
            raise ValueError(f"trade_value must be >= 0, got {trade_value}")
        volume = monthly_volume if monthly_volume is not None else 0.0
        rate = self.tiers[-1][1]  # fallback: 最高档
        for threshold, tier_rate in self.tiers:
            if volume < threshold:
                rate = tier_rate
                break
        return trade_value * rate
