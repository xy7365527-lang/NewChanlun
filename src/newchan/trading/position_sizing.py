"""仓位确定性函数 — 共振信号集 -> 仓位比例映射。

文档三 §七 的实现。

核心公式：
    W = W_base * prod(1 + alpha_i * w_i)

其中 W_base 由最高层已确认信号决定，alpha_i in {0,1} 表示信号是否确认，
w_i > 0 是该信号的权重。
"""

from __future__ import annotations

import math
from dataclasses import dataclass, replace
from enum import Enum
from typing import Sequence


class Layer(Enum):
    """信号所属层级。"""

    L0 = 0  # 配置层
    L1 = 1  # 跨角/边层
    L2 = 2  # 角内轮动
    L3 = 3  # 标的内操作


@dataclass(frozen=True, slots=True)
class Signal:
    """单个共振信号。

    Attributes
    ----------
    id : str
        信号标识符，如 "s1"。
    layer : Layer
        信号所属层级。
    weight : float
        信号权重 w_i > 0。
    confirmed : bool
        信号是否已确认（alpha_i = 1 or 0）。
    """

    id: str
    layer: Layer
    weight: float
    confirmed: bool = False

    def __post_init__(self) -> None:
        if self.weight <= 0:
            raise ValueError(f"weight must be > 0, got {self.weight}")


# -- 标准信号集（文档三 §七 表格） --

STANDARD_SIGNALS: tuple[Signal, ...] = (
    Signal(id="s1", layer=Layer.L0, weight=0.15),  # 配置层一买
    Signal(id="s2", layer=Layer.L1, weight=0.12),  # 独立边一买
    Signal(id="s3", layer=Layer.L1, weight=0.06),  # 派生边共振
    Signal(id="s4", layer=Layer.L1, weight=0.06),  # 跨国同类边共振
    Signal(id="s5", layer=Layer.L1, weight=0.05),  # 货币边共振
    Signal(id="s6", layer=Layer.L2, weight=0.04),  # 板块比价一买
    Signal(id="s7", layer=Layer.L2, weight=0.03),  # 标的比价一买
    Signal(id="s8", layer=Layer.L3, weight=0.02),  # 标的纵向区间套确认
)


# -- W_base 默认值 --

W_CONFIG: float = 0.50   # 有配置层一买时的基础仓位
W_EDGE: float = 0.25     # 无配置层但有独立边一买时
W_SINGLE: float = 0.08   # 仅标的内买点时


@dataclass(frozen=True, slots=True)
class PositionSizer:
    """仓位确定性函数。

    Parameters
    ----------
    w_config : float
        配置层基础仓位（有 s1 时）。
    w_edge : float
        边层基础仓位（无 s1 有 s2 时）。
    w_single : float
        单标的基础仓位（仅 s8 时）。
    """

    w_config: float = W_CONFIG
    w_edge: float = W_EDGE
    w_single: float = W_SINGLE

    def __post_init__(self) -> None:
        for name, val in [
            ("w_config", self.w_config),
            ("w_edge", self.w_edge),
            ("w_single", self.w_single),
        ]:
            if not 0 < val <= 1:
                raise ValueError(f"{name} must be in (0, 1], got {val}")

    def w_base(self, signals: Sequence[Signal]) -> float:
        """根据最高层已确认信号确定 W_base。

        规则：
        - 有 s1（配置层一买）已确认 -> w_config
        - 无 s1 但有 L1 层信号已确认 -> w_edge
        - 仅有 L2/L3 层信号已确认 -> w_single
        - 无任何已确认信号 -> 0.0
        """
        confirmed = [s for s in signals if s.confirmed]
        if not confirmed:
            return 0.0

        highest_layer = min(s.layer.value for s in confirmed)

        if highest_layer == Layer.L0.value:
            return self.w_config
        if highest_layer == Layer.L1.value:
            return self.w_edge
        return self.w_single

    def position(self, signals: Sequence[Signal]) -> float:
        """计算仓位 W = W_base * prod(1 + alpha_i * w_i)。

        返回值 clamp 到 [0, 1]。
        """
        base = self.w_base(signals)
        if base == 0.0:
            return 0.0

        product = math.prod(
            1 + s.weight for s in signals if s.confirmed
        )
        return min(base * product, 1.0)

    def validate_weights(self, signals: Sequence[Signal]) -> bool:
        """检查权重约束：prod(1 + w_i) <= 1 / W_base。

        当所有信号同时确认时 W <= 1 的充分条件。
        使用信号集中最高层对应的 W_base 计算。

        Parameters
        ----------
        signals : Sequence[Signal]
            信号集（不要求已确认，按全确认假设检查）。

        Returns
        -------
        bool
            权重约束是否满足。
        """
        if not signals:
            return True

        # 假设全部确认来确定 W_base
        all_confirmed = [replace(s, confirmed=True) for s in signals]
        base = self.w_base(all_confirmed)
        if base == 0.0:
            return True

        product = math.prod(1 + s.weight for s in signals)
        return product <= 1.0 / base
