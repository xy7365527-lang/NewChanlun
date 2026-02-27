"""降成本递归测试。"""

from __future__ import annotations

import pytest

from newchan.trading.cost_reduction import (
    CostTracker,
    global_efficiency,
    transfer_efficiency,
)


# ── CostTracker ──


class TestCostTracker:
    def test_initial_state(self) -> None:
        t = CostTracker()
        assert t.remaining_cost() == 0.0
        assert t.is_negative_cost() is False

    def test_single_entry(self) -> None:
        t = CostTracker()
        t.add_entry(1000.0)
        assert t.remaining_cost() == 1000.0
        assert t.is_negative_cost() is False

    def test_entry_then_exit(self) -> None:
        t = CostTracker()
        t.add_entry(1000.0)
        profit = t.add_exit(600.0, ratio=0.5)
        # remaining = 1000 - 600 = 400
        assert t.remaining_cost() == 400.0
        # profit = 600 - 1000 * 0.5 = 100
        assert abs(profit - 100.0) < 1e-9

    def test_negative_cost_after_sufficient_exits(self) -> None:
        """当累积利润 > 初始成本时 Xi < 0。"""
        t = CostTracker()
        t.add_entry(1000.0)
        # 多次出场，收益总额超过入场成本
        t.add_exit(400.0, ratio=0.3)
        t.add_exit(400.0, ratio=0.3)
        t.add_exit(400.0, ratio=0.3)
        # remaining = 1000 - 1200 = -200
        assert t.remaining_cost() == -200.0
        assert t.is_negative_cost() is True

    def test_multiple_entries_and_exits(self) -> None:
        t = CostTracker()
        t.add_entry(500.0)
        t.add_exit(300.0, ratio=0.5)
        t.add_entry(200.0)
        t.add_exit(450.0, ratio=0.8)
        # remaining = (500 + 200) - (300 + 450) = 700 - 750 = -50
        assert abs(t.remaining_cost() - (-50.0)) < 1e-9
        assert t.is_negative_cost() is True

    def test_cost_reduction_convergence(self) -> None:
        """模拟标的内降成本递归：买低卖高，成本逐步下降。"""
        t = CostTracker()
        t.add_entry(1000.0)  # 一买入场

        # 次级别一卖减仓，每次减30%获利
        for _ in range(5):
            current = t.remaining_cost()
            # 假设每次减仓收回 current * 0.3 * 1.15（15%利润）
            revenue = max(current, 0) * 0.3 * 1.15
            t.add_exit(revenue, ratio=0.3)

        # 经过5轮降成本，净成本应显著低于初始值
        assert t.remaining_cost() < 1000.0

    def test_entry_cost_must_be_positive(self) -> None:
        t = CostTracker()
        with pytest.raises(ValueError, match="entry cost must be > 0"):
            t.add_entry(0.0)
        with pytest.raises(ValueError, match="entry cost must be > 0"):
            t.add_entry(-100.0)

    def test_exit_revenue_must_be_non_negative(self) -> None:
        t = CostTracker()
        t.add_entry(100.0)
        with pytest.raises(ValueError, match="revenue must be >= 0"):
            t.add_exit(-10.0, ratio=0.5)

    def test_exit_ratio_bounds(self) -> None:
        t = CostTracker()
        t.add_entry(100.0)
        with pytest.raises(ValueError, match="ratio must be in"):
            t.add_exit(50.0, ratio=0.0)
        with pytest.raises(ValueError, match="ratio must be in"):
            t.add_exit(50.0, ratio=1.5)

    def test_total_properties(self) -> None:
        t = CostTracker()
        t.add_entry(1000.0)
        t.add_entry(500.0)
        t.add_exit(600.0, ratio=0.3)
        assert t.total_entries == 1500.0
        assert t.total_exits == 600.0


# ── transfer_efficiency ──


class TestTransferEfficiency:
    def test_no_losses(self) -> None:
        eta = transfer_efficiency([100.0, 200.0], [])
        assert eta == 1.0

    def test_partial_losses(self) -> None:
        eta = transfer_efficiency([100.0, 200.0], [50.0])
        # (300 - 50) / 300 = 250/300 ≈ 0.833
        assert abs(eta - 250.0 / 300.0) < 1e-9

    def test_losses_equal_profits(self) -> None:
        eta = transfer_efficiency([100.0], [100.0])
        assert eta == 0.0

    def test_losses_exceed_profits(self) -> None:
        """亏损超过盈利时 eta = 0（不会为负）。"""
        eta = transfer_efficiency([100.0], [200.0])
        assert eta == 0.0

    def test_no_profits(self) -> None:
        eta = transfer_efficiency([], [50.0])
        assert eta == 0.0

    def test_realistic_example(self) -> None:
        """§四.5 示例：eta = 0.7（30%被止损消耗）。"""
        # profits = [700], losses = [210] => eta = 490/700 = 0.7
        eta = transfer_efficiency([700.0], [210.0])
        assert abs(eta - 0.7) < 1e-9


# ── global_efficiency ──


class TestGlobalEfficiency:
    def test_single_layer(self) -> None:
        assert abs(global_efficiency([0.85]) - 0.85) < 1e-9

    def test_three_layers(self) -> None:
        """§四.6：三层 eta=0.7 → 全局 0.343。"""
        result = global_efficiency([0.7, 0.7, 0.7])
        assert abs(result - 0.343) < 1e-9

    def test_three_layers_high(self) -> None:
        """§四.6：三层 eta=0.85 → 全局 0.614。"""
        result = global_efficiency([0.85, 0.85, 0.85])
        assert abs(result - 0.614125) < 1e-6

    def test_empty(self) -> None:
        assert global_efficiency([]) == 1.0

    def test_one_zero_kills_all(self) -> None:
        """任一层效率为0则全局为0。"""
        assert global_efficiency([0.9, 0.0, 0.8]) == 0.0

    def test_invalid_eta_raises(self) -> None:
        with pytest.raises(ValueError, match="eta must be in"):
            global_efficiency([1.5])
        with pytest.raises(ValueError, match="eta must be in"):
            global_efficiency([-0.1])
