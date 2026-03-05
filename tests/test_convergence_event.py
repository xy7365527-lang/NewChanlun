"""收敛事件产生器测试。

验证：
1. T(S) 从0变为正值 → 产生 CONVERGENCE_STARTED 事件
2. T(S) 增大（新级别背驰出现）→ 产生 CONVERGENCE_DEEPENED 事件
3. T(S) 减小（旧级别背驰消失）→ 产生 CONVERGENCE_WEAKENED 事件
4. T(S) 从正值变为0 → 产生 CONVERGENCE_LOST 事件
5. T(S) 不变 → 不产生事件
6. 边界条件：首次计算（无前值）、连续相同

认识论等级：L0（从350号下游推论3定义推导）。
谱系引用：350号收敛紧度、352号扫描器设计。
"""

from __future__ import annotations

import pytest

from newchan.trading.convergence_event import (
    ConvergenceChange,
    ConvergenceChangeType,
    detect_convergence_change,
)


# ═══════════════════════════════════════════════════════════════
# 基础事件检测
# ═══════════════════════════════════════════════════════════════


class TestDetectConvergenceChange:
    """验证收敛事件检测逻辑。"""

    def test_started_from_zero(self) -> None:
        """T(S) 从 0 变为正值 → STARTED。"""
        change = detect_convergence_change("GLD", prev_score=0.0, new_score=6.0)
        assert change is not None
        assert change.change_type is ConvergenceChangeType.STARTED
        assert change.symbol == "GLD"
        assert change.prev_score == 0.0
        assert change.new_score == 6.0

    def test_deepened(self) -> None:
        """T(S) 增大 → DEEPENED。"""
        change = detect_convergence_change("USO", prev_score=3.0, new_score=6.0)
        assert change is not None
        assert change.change_type is ConvergenceChangeType.DEEPENED

    def test_weakened(self) -> None:
        """T(S) 减小但仍为正 → WEAKENED。"""
        change = detect_convergence_change("TLT", prev_score=6.0, new_score=3.0)
        assert change is not None
        assert change.change_type is ConvergenceChangeType.WEAKENED

    def test_lost(self) -> None:
        """T(S) 从正值变为 0 → LOST。"""
        change = detect_convergence_change("AAPL", prev_score=3.0, new_score=0.0)
        assert change is not None
        assert change.change_type is ConvergenceChangeType.LOST

    def test_no_change(self) -> None:
        """T(S) 不变 → None。"""
        change = detect_convergence_change("GLD", prev_score=3.0, new_score=3.0)
        assert change is None

    def test_both_zero_no_change(self) -> None:
        """前后都是 0 → None。"""
        change = detect_convergence_change("GLD", prev_score=0.0, new_score=0.0)
        assert change is None


# ═══════════════════════════════════════════════════════════════
# 首次计算（无前值）
# ═══════════════════════════════════════════════════════════════


class TestFirstCalculation:
    """验证首次计算场景（prev_score=None）。"""

    def test_first_with_positive_score(self) -> None:
        """首次计算有正 T → STARTED。"""
        change = detect_convergence_change("GLD", prev_score=None, new_score=5.0)
        assert change is not None
        assert change.change_type is ConvergenceChangeType.STARTED
        assert change.prev_score == 0.0  # None 规范化为 0

    def test_first_with_zero_score(self) -> None:
        """首次计算 T=0 → None。"""
        change = detect_convergence_change("GLD", prev_score=None, new_score=0.0)
        assert change is None


# ═══════════════════════════════════════════════════════════════
# ConvergenceChange 属性
# ═══════════════════════════════════════════════════════════════


class TestConvergenceChangeProperties:
    """验证 ConvergenceChange 数据类。"""

    def test_delta(self) -> None:
        """delta = new_score - prev_score。"""
        change = detect_convergence_change("GLD", prev_score=2.0, new_score=5.0)
        assert change is not None
        assert change.delta == pytest.approx(3.0)

    def test_negative_delta(self) -> None:
        """减弱时 delta 为负。"""
        change = detect_convergence_change("GLD", prev_score=5.0, new_score=2.0)
        assert change is not None
        assert change.delta == pytest.approx(-3.0)

    def test_frozen(self) -> None:
        """ConvergenceChange 是不可变的。"""
        change = detect_convergence_change("GLD", prev_score=0.0, new_score=5.0)
        assert change is not None
        with pytest.raises(AttributeError):
            change.symbol = "USO"  # type: ignore[misc]
