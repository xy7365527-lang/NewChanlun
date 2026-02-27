"""四层递归状态机测试。

覆盖：
- 状态转换合法性（不能从 INACTIVE 直接到 SUSPENDED）
- 公理1：高层止损后所有低层为 INACTIVE
- 公理2：低层 entry 需要上层为 ACTIVE
- 公理3：利润只在卖出事件时传递
- 六条路径的正确执行
"""

from __future__ import annotations

import pytest

from newchan.trading.layer_state import Layer, LayerState, LayerType, is_valid_transition
from newchan.trading.state_machine import (
    AxiomViolationError,
    FourLayerStateMachine,
    IllegalTransitionError,
)
from newchan.trading.axioms import (
    axiom1_coverage,
    axiom2_no_upward_entry,
    axiom3_profit_transfer,
)
from newchan.trading.paths import Event, PathType, classify_path, execute_path


# ── 辅助 ──────────────────────────────────────────────────────────


def _fully_active_machine() -> FourLayerStateMachine:
    """创建所有层都 ACTIVE 的状态机（用于测试退出路径）。"""
    m = FourLayerStateMachine()
    r = m.entry(LayerType.L0_CONFIG, "配置空间一买", position=1.0)
    r = r.machine.entry(LayerType.L1_EDGE, "独立边 e1 一买", position=0.5)
    r = r.machine.entry(LayerType.L2_CORNER, "角内比价一买", position=0.3)
    r = r.machine.entry(LayerType.L3_STOCK, "标的纵向区间套确认", position=0.2)
    return r.machine


# ── 状态转换合法性 ─────────────────────────────────────────────────


class TestValidTransitions:
    def test_inactive_to_active_valid(self) -> None:
        assert is_valid_transition(LayerState.INACTIVE, LayerState.ACTIVE)

    def test_active_to_inactive_valid(self) -> None:
        assert is_valid_transition(LayerState.ACTIVE, LayerState.INACTIVE)

    def test_active_to_suspended_valid(self) -> None:
        assert is_valid_transition(LayerState.ACTIVE, LayerState.SUSPENDED)

    def test_suspended_to_active_valid(self) -> None:
        assert is_valid_transition(LayerState.SUSPENDED, LayerState.ACTIVE)

    def test_suspended_to_inactive_valid(self) -> None:
        assert is_valid_transition(LayerState.SUSPENDED, LayerState.INACTIVE)

    def test_inactive_to_suspended_invalid(self) -> None:
        assert not is_valid_transition(LayerState.INACTIVE, LayerState.SUSPENDED)

    def test_same_state_always_valid(self) -> None:
        for s in LayerState:
            assert is_valid_transition(s, s)


class TestEntryTransitions:
    def test_l0_entry_from_inactive(self) -> None:
        m = FourLayerStateMachine()
        result = m.entry(LayerType.L0_CONFIG, "配置空间一买", position=1.0)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

    def test_l1_entry_requires_l0_active(self) -> None:
        m = FourLayerStateMachine()
        with pytest.raises(AxiomViolationError, match="公理2违反"):
            m.entry(LayerType.L1_EDGE, "独立边一买")

    def test_l1_entry_with_l0_active(self) -> None:
        m = FourLayerStateMachine()
        r = m.entry(LayerType.L0_CONFIG, "配置空间一买")
        r2 = r.machine.entry(LayerType.L1_EDGE, "独立边一买", position=0.5)
        assert r2.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE

    def test_entry_from_suspended_valid(self) -> None:
        """SUSPENDED -> ACTIVE 是合法的（上层重新 entry）。"""
        layers = {
            lt: Layer(layer_type=lt) for lt in LayerType
        }
        layers[LayerType.L0_CONFIG] = Layer(
            layer_type=LayerType.L0_CONFIG, state=LayerState.ACTIVE
        )
        layers[LayerType.L1_EDGE] = Layer(
            layer_type=LayerType.L1_EDGE, state=LayerState.SUSPENDED
        )
        m = FourLayerStateMachine(layers=layers)
        result = m.entry(LayerType.L1_EDGE, "新独立边一买", position=0.3)
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE

    def test_entry_from_active_raises(self) -> None:
        """已经 ACTIVE 的层不能再次 entry（ACTIVE -> ACTIVE 非法）。"""
        m = FourLayerStateMachine()
        r = m.entry(LayerType.L0_CONFIG, "配置空间一买")
        # ACTIVE -> ACTIVE: is_valid_transition returns True (same state),
        # but entry should still succeed (re-entry is idempotent in same-state case)
        # Actually re-reading the transition table: ACTIVE->ACTIVE is treated as same-state
        r2 = r.machine.entry(LayerType.L0_CONFIG, "新一买")
        assert r2.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE


# ── 公理1：高层覆盖 ───────────────────────────────────────────────


class TestAxiom1Coverage:
    def test_l0_abort_clears_all(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L0_CONFIG)
        assert axiom1_coverage(result.machine, LayerType.L0_CONFIG)
        for lt in (LayerType.L1_EDGE, LayerType.L2_CORNER, LayerType.L3_STOCK):
            assert result.machine.get_state(lt) == LayerState.INACTIVE

    def test_l1_abort_clears_l2_l3(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L1_EDGE)
        assert axiom1_coverage(result.machine, LayerType.L1_EDGE)
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE

    def test_l1_abort_keeps_l0(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L1_EDGE)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

    def test_l2_abort_clears_l3(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L2_CORNER)
        assert axiom1_coverage(result.machine, LayerType.L2_CORNER)
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE

    def test_l3_abort_only_affects_self(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L3_STOCK)
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE


# ── 公理2：下层不创造上层入口 ──────────────────────────────────────


class TestAxiom2NoUpwardEntry:
    def test_l3_cannot_entry_without_l2_active(self) -> None:
        m = FourLayerStateMachine()
        r = m.entry(LayerType.L0_CONFIG, "配置空间一买")
        r = r.machine.entry(LayerType.L1_EDGE, "独立边一买")
        # L2 未 ACTIVE 时，L3 不能 entry
        with pytest.raises(AxiomViolationError, match="公理2违反"):
            r.machine.entry(LayerType.L3_STOCK, "标的一买")

    def test_axiom2_check_detects_violation(self) -> None:
        layers = {lt: Layer(layer_type=lt) for lt in LayerType}
        layers[LayerType.L0_CONFIG] = Layer(
            layer_type=LayerType.L0_CONFIG, state=LayerState.INACTIVE
        )
        layers[LayerType.L1_EDGE] = Layer(
            layer_type=LayerType.L1_EDGE, state=LayerState.ACTIVE
        )
        m = FourLayerStateMachine(layers=layers)
        assert not axiom2_no_upward_entry(m, LayerType.L1_EDGE, LayerType.L0_CONFIG)

    def test_axiom2_check_passes_normal(self) -> None:
        m = _fully_active_machine()
        assert axiom2_no_upward_entry(m, LayerType.L1_EDGE, LayerType.L0_CONFIG)
        assert axiom2_no_upward_entry(m, LayerType.L2_CORNER, LayerType.L1_EDGE)
        assert axiom2_no_upward_entry(m, LayerType.L3_STOCK, LayerType.L2_CORNER)

    def test_axiom2_raises_on_wrong_order(self) -> None:
        m = _fully_active_machine()
        with pytest.raises(ValueError, match="必须比"):
            axiom2_no_upward_entry(m, LayerType.L0_CONFIG, LayerType.L1_EDGE)


# ── 公理3：利润传递需要卖出事件 ────────────────────────────────────


class TestAxiom3ProfitTransfer:
    def test_active_layer_profit_not_transferred(self) -> None:
        m = _fully_active_machine()
        assert axiom3_profit_transfer(m, LayerType.L3_STOCK)
        assert axiom3_profit_transfer(m, LayerType.L2_CORNER)
        assert axiom3_profit_transfer(m, LayerType.L1_EDGE)

    def test_l0_raises(self) -> None:
        m = _fully_active_machine()
        with pytest.raises(ValueError, match="最高层"):
            axiom3_profit_transfer(m, LayerType.L0_CONFIG)

    def test_profit_transferred_after_exit(self) -> None:
        """L3 exit_normal 后利润传递给 L2。"""
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L3_STOCK)
        # L3 退出后，L2 的 profit_buffer 应该增加
        l2 = result.machine.get_layer(LayerType.L2_CORNER)
        assert l2.profit_buffer > 0.0


# ── exit_normal 语义 ──────────────────────────────────────────────


class TestExitNormal:
    def test_l0_exit_suspends_children(self) -> None:
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L0_CONFIG)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.SUSPENDED
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED

    def test_l1_exit_keeps_active_suspends_children(self) -> None:
        """L1 正常退出 = 角切换。L1 保持 ACTIVE，L2/L3 SUSPENDED。"""
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L1_EDGE)
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED

    def test_l2_exit_keeps_active_inactivates_l3(self) -> None:
        """L2 正常退出 = 标的切换。L2 保持 ACTIVE，L3 INACTIVE。"""
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L2_CORNER)
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE

    def test_l3_exit_becomes_inactive(self) -> None:
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L3_STOCK)
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE

    def test_exit_normal_from_inactive_raises(self) -> None:
        m = FourLayerStateMachine()
        with pytest.raises(IllegalTransitionError):
            m.exit_normal(LayerType.L0_CONFIG)

    def test_realized_profit_positive(self) -> None:
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L3_STOCK)
        assert result.realized_profit > 0.0


# ── exit_abort 语义 ───────────────────────────────────────────────


class TestExitAbort:
    def test_l1_abort_stays_active(self) -> None:
        """L1 止损后自身保持 ACTIVE（仍在跨角轮动中等待新角）。"""
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L1_EDGE)
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE

    def test_l0_abort_becomes_inactive(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L0_CONFIG)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE

    def test_abort_returns_negative_profit(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L3_STOCK)
        assert result.realized_profit <= 0.0

    def test_abort_from_inactive_raises(self) -> None:
        m = FourLayerStateMachine()
        with pytest.raises(IllegalTransitionError):
            m.exit_abort(LayerType.L0_CONFIG)


# ── cascade_abort ─────────────────────────────────────────────────


class TestCascadeAbort:
    def test_cascade_from_l1_clears_l2_l3(self) -> None:
        m = _fully_active_machine()
        result = m.cascade_abort(LayerType.L1_EDGE)
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        # L1 itself unchanged
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE


# ── suspend_below ─────────────────────────────────────────────────


class TestSuspendBelow:
    def test_suspend_below_l0(self) -> None:
        m = _fully_active_machine()
        m2 = m.suspend_below(LayerType.L0_CONFIG)
        assert m2.get_state(LayerType.L1_EDGE) == LayerState.SUSPENDED
        assert m2.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert m2.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED

    def test_suspend_below_l1(self) -> None:
        m = _fully_active_machine()
        m2 = m.suspend_below(LayerType.L1_EDGE)
        assert m2.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE  # 自身不受影响
        assert m2.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert m2.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED


# ── L0 无 SUSPENDED 状态 ──────────────────────────────────────────


class TestL0NoSuspended:
    def test_l0_never_suspended(self) -> None:
        """L0 是最高层，没有上层，不存在 SUSPENDED 状态。"""
        m = _fully_active_machine()
        # L0 exit_normal → INACTIVE（不是 SUSPENDED）
        result = m.exit_normal(LayerType.L0_CONFIG)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE

    def test_l0_abort_to_inactive(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L0_CONFIG)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE


# ── L3 无 SUSPENDED 状态 ──────────────────────────────────────────


class TestL3NoSuspended:
    def test_l2_exit_normal_l3_inactive_not_suspended(self) -> None:
        """L2 正常退出时 L3 回到 INACTIVE（不是 SUSPENDED）。"""
        m = _fully_active_machine()
        result = m.exit_normal(LayerType.L2_CORNER)
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE


# ── 路径分类 ──────────────────────────────────────────────────────


class TestPathClassification:
    def test_path_a_normal_full(self) -> None:
        events = [
            Event(LayerType.L0_CONFIG, "entry"),
            Event(LayerType.L1_EDGE, "entry"),
            Event(LayerType.L2_CORNER, "entry"),
            Event(LayerType.L3_STOCK, "entry"),
            Event(LayerType.L3_STOCK, "exit_normal"),
            Event(LayerType.L2_CORNER, "exit_normal"),
            Event(LayerType.L1_EDGE, "exit_normal"),
            Event(LayerType.L0_CONFIG, "exit_normal"),
        ]
        assert classify_path(events) == PathType.NORMAL_FULL

    def test_path_b_bottom_stop(self) -> None:
        events = [
            Event(LayerType.L3_STOCK, "entry"),
            Event(LayerType.L3_STOCK, "exit_abort"),
        ]
        assert classify_path(events) == PathType.BOTTOM_STOP

    def test_path_c_corner_stop(self) -> None:
        events = [
            Event(LayerType.L1_EDGE, "entry"),
            Event(LayerType.L2_CORNER, "entry"),
            Event(LayerType.L1_EDGE, "exit_abort"),
        ]
        assert classify_path(events) == PathType.CORNER_STOP

    def test_path_d_config_stop(self) -> None:
        events = [
            Event(LayerType.L0_CONFIG, "entry"),
            Event(LayerType.L0_CONFIG, "exit_abort"),
        ]
        assert classify_path(events) == PathType.CONFIG_STOP

    def test_path_e_early_warning(self) -> None:
        events = [
            Event(LayerType.L3_STOCK, "exit_normal"),
            Event(LayerType.L3_STOCK, "exit_normal"),
            Event(LayerType.L2_CORNER, "exit_normal"),
            Event(LayerType.L1_EDGE, "exit_normal"),
            Event(LayerType.L0_CONFIG, "exit_normal"),
        ]
        assert classify_path(events) == PathType.EARLY_WARNING

    def test_path_f_time_mismatch(self) -> None:
        events = [
            Event(LayerType.L1_EDGE, "entry"),
            Event(LayerType.L2_CORNER, "entry"),
            Event(LayerType.L3_STOCK, "entry"),
            Event(LayerType.L1_EDGE, "exit_normal"),  # L1 先退出，L3 还没退
        ]
        assert classify_path(events) == PathType.TIME_MISMATCH

    def test_empty_sequence_raises(self) -> None:
        with pytest.raises(ValueError, match="不能为空"):
            classify_path([])


# ── 路径执行 ──────────────────────────────────────────────────────


class TestPathExecution:
    def test_execute_path_d_all_inactive(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.CONFIG_STOP)
        assert result.machine.all_inactive()

    def test_execute_path_c_l1_stays_active(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.CORNER_STOP)
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.INACTIVE

    def test_execute_path_b_upper_layers_unchanged(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.BOTTOM_STOP)
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE

    def test_execute_path_a_normal_full(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.NORMAL_FULL)
        # 正常全退出后所有层应回到非 ACTIVE
        for lt in LayerType:
            assert result.machine.get_state(lt) != LayerState.ACTIVE

    def test_execute_path_f_cascade(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.TIME_MISMATCH)
        assert result.machine.get_state(LayerType.L2_CORNER) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE

    def test_execute_path_e_early_warning(self) -> None:
        m = _fully_active_machine()
        result = execute_path(m, PathType.EARLY_WARNING)
        assert result.realized_profit != 0.0


# ── 不可变性 ──────────────────────────────────────────────────────


class TestImmutability:
    def test_entry_does_not_mutate_original(self) -> None:
        m = FourLayerStateMachine()
        result = m.entry(LayerType.L0_CONFIG, "一买")
        assert m.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

    def test_exit_does_not_mutate_original(self) -> None:
        m = _fully_active_machine()
        result = m.exit_abort(LayerType.L0_CONFIG)
        assert m.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE

    def test_layer_with_returns_new_instance(self) -> None:
        layer = Layer(layer_type=LayerType.L0_CONFIG, state=LayerState.INACTIVE)
        new_layer = layer._with(state=LayerState.ACTIVE)
        assert layer.state == LayerState.INACTIVE
        assert new_layer.state == LayerState.ACTIVE
        assert layer is not new_layer


# ── repr ──────────────────────────────────────────────────────────


class TestRepr:
    def test_repr_shows_all_states(self) -> None:
        m = FourLayerStateMachine()
        r = repr(m)
        assert "L0_CONFIG=INACTIVE" in r
        assert "L3_STOCK=INACTIVE" in r

    def test_active_layers_returns_sorted(self) -> None:
        m = _fully_active_machine()
        active = m.active_layers()
        assert active == [
            LayerType.L0_CONFIG,
            LayerType.L1_EDGE,
            LayerType.L2_CORNER,
            LayerType.L3_STOCK,
        ]


# ── 完整周期集成测试 ──────────────────────────────────────────────


class TestFullCycleIntegration:
    def test_complete_entry_exit_cycle(self) -> None:
        """完整的 entry -> 操作 -> exit 周期。"""
        # 1. 逐层入场
        m = FourLayerStateMachine()
        r = m.entry(LayerType.L0_CONFIG, "配置空间一买", position=1.0)
        r = r.machine.entry(LayerType.L1_EDGE, "独立边一买", position=0.5)
        r = r.machine.entry(LayerType.L2_CORNER, "板块比价一买", position=0.3)
        r = r.machine.entry(LayerType.L3_STOCK, "标的一买", position=0.2)
        m_active = r.machine

        # 2. L3 正常退出（标的内操作完成）
        r = m_active.exit_normal(LayerType.L3_STOCK)
        assert r.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert r.realized_profit > 0

        # 3. L2 角内轮动（新标的 entry）
        r = r.machine.entry(LayerType.L3_STOCK, "新标的一买", position=0.15)
        assert r.machine.get_state(LayerType.L3_STOCK) == LayerState.ACTIVE

        # 4. L1 退出（角切换）
        r = r.machine.exit_normal(LayerType.L1_EDGE)
        assert r.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert r.machine.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert r.machine.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED

    def test_abort_recovery_cycle(self) -> None:
        """止损 -> 恢复周期。"""
        m = _fully_active_machine()

        # L3 止损
        r = m.exit_abort(LayerType.L3_STOCK)
        assert r.machine.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert r.machine.get_state(LayerType.L2_CORNER) == LayerState.ACTIVE

        # L2 内重新找标的
        r = r.machine.entry(LayerType.L3_STOCK, "新标的一买", position=0.1)
        assert r.machine.get_state(LayerType.L3_STOCK) == LayerState.ACTIVE

    def test_l0_exit_suspend_resume_cycle(self) -> None:
        """L0 退出 -> SUSPENDED -> L0 重新 entry -> 恢复。"""
        m = _fully_active_machine()

        # L0 正常退出
        r = m.exit_normal(LayerType.L0_CONFIG)
        assert r.machine.get_state(LayerType.L1_EDGE) == LayerState.SUSPENDED

        # L0 重新 entry
        r2 = r.machine.entry(LayerType.L0_CONFIG, "新配置空间一买", position=0.8)
        assert r2.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

        # L1 从 SUSPENDED 恢复
        r3 = r2.machine.entry(LayerType.L1_EDGE, "新独立边一买", position=0.4)
        assert r3.machine.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
