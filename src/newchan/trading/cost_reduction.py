"""降成本递归 — 成本追踪 + 传递效率。

文档三 §四 的实现。

核心概念：
- 累积净成本 Xi = sum(入场成本) - sum(已兑现收益)
- Xi < 0 意味着持仓成本为负（本金已全部收回）
- 传递效率 eta = 实际传递到上层的利润 / 该层总利润
- 全局传递效率 = prod(eta_k)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence


@dataclass(slots=True)
class CostTracker:
    """单层降成本递归的成本追踪器。

    追踪入场成本和出场收益，计算累积净成本 Xi。

    Xi(N) = sum_{n=1}^{N} C(n) - sum_{n=1}^{N-1} R(n)
    即：每次入场投入减去之前所有退出回收。
    """

    _entries: list[float] = field(default_factory=list)
    _exits: list[float] = field(default_factory=list)

    def add_entry(self, cost: float) -> None:
        """记录一次入场成本。

        Parameters
        ----------
        cost : float
            入场成本，必须 > 0。
        """
        if cost <= 0:
            raise ValueError(f"entry cost must be > 0, got {cost}")
        self._entries.append(cost)

    def add_exit(self, revenue: float, ratio: float) -> float:
        """记录一次出场，返回本次利润。

        Parameters
        ----------
        revenue : float
            出场收益金额（卖出所得），必须 >= 0。
        ratio : float
            减仓比例 in (0, 1]。

        Returns
        -------
        float
            本次操作的利润（revenue - 对应比例的成本）。
        """
        if revenue < 0:
            raise ValueError(f"revenue must be >= 0, got {revenue}")
        if not 0 < ratio <= 1:
            raise ValueError(f"ratio must be in (0, 1], got {ratio}")
        # 利润 = 收益 - 对应比例的当前剩余成本（在记录出场之前计算）
        current_cost = self.remaining_cost()
        cost_portion = current_cost * ratio
        self._exits.append(revenue)
        return revenue - cost_portion

    def remaining_cost(self) -> float:
        """当前累积净成本 Xi。

        Xi = sum(entries) - sum(exits)

        Xi < 0 表示持仓成本为负。
        """
        total_entries = sum(self._entries)
        total_exits = sum(self._exits)
        return total_entries - total_exits

    def is_negative_cost(self) -> bool:
        """成本是否为负（本金已全部收回）。"""
        return self.remaining_cost() < 0

    @property
    def total_entries(self) -> float:
        """累积入场成本总额。"""
        return sum(self._entries)

    @property
    def total_exits(self) -> float:
        """累积出场收益总额。"""
        return sum(self._exits)


def transfer_efficiency(profits: Sequence[float], losses: Sequence[float]) -> float:
    """计算单层传递效率 eta。

    eta = (sum(profits) - sum(|losses|)) / sum(profits)

    eta < 1 的原因：止损亏损、摩擦成本、时间损耗。

    Parameters
    ----------
    profits : Sequence[float]
        盈利操作的利润列表，每个值 > 0。
    losses : Sequence[float]
        亏损操作的亏损列表，每个值 > 0（绝对值）。

    Returns
    -------
    float
        传递效率 eta in [0, 1]。无盈利时返回 0。
    """
    total_profit = sum(profits)
    if total_profit <= 0:
        return 0.0
    total_loss = sum(losses)
    eta = (total_profit - total_loss) / total_profit
    return max(eta, 0.0)


def global_efficiency(layer_efficiencies: Sequence[float]) -> float:
    """计算全局传递效率 = prod(eta_k)。

    Parameters
    ----------
    layer_efficiencies : Sequence[float]
        各层传递效率 eta_k，每个值 in [0, 1]。

    Returns
    -------
    float
        全局传递效率 in [0, 1]。
    """
    result = 1.0
    for eta in layer_efficiencies:
        if not 0 <= eta <= 1:
            raise ValueError(f"eta must be in [0, 1], got {eta}")
        result *= eta
    return result
