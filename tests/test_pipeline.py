"""交易管道端到端测试。

覆盖 pipeline.py 的核心流程：
topology(Configuration) -> nesting(HorizontalNesting) -> trading(StateMachine)

测试策略：集成测试，不 mock 内部模块。
"""

from __future__ import annotations

import pytest

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.horizontal import HorizontalStep
from newchan.nesting.resonance import ResonanceLevel, ResonanceSignal, SignalLayer
from newchan.topology.config_space import CENTER, FULL_RISK_ON, Configuration, WalkDirection
from newchan.trading.layer_state import LayerState, LayerType


# ── 辅助函数 ───────────────────────────────────────────────


def _make_bsp(
    bsp_type: BSPType = BSPType.B1,
    price: float = 100.0,
    time: float = 1.0,
) -> BSP:
    """创建测试用买卖点。"""
    return BSP(edge_id="e1", level=0, time=time, bsp_type=bsp_type, price=price)


def _make_resonance_signal(
    bsp: BSP,
    layer: SignalLayer = SignalLayer.CONFIG,
    time: float = 1.0,
) -> ResonanceSignal:
    """创建测试用共振信号。"""
    return ResonanceSignal(
        edge_id=bsp.edge_id,
        level=bsp.level,
        bsp=bsp,
        layer=layer,
        time=time,
    )


def _time_tolerance(l1: int, l2: int) -> float:
    """宽容时间容差——测试不关注时间窗口严格性。"""
    return 100.0


# ── TestCreateContext ──────────────────────────────────────


class TestCreateContext:
    """测试 create_context：从 Configuration 创建 TradingContext。"""

    def test_create_from_full_risk_on(self) -> None:
        """FULL_RISK_ON (+,+,+) 配置，极性指数 S=3。"""
        from newchan.pipeline import create_context

        ctx = create_context(FULL_RISK_ON)
        assert ctx.polarity == 3
        assert ctx.config is FULL_RISK_ON

    def test_create_from_center(self) -> None:
        """CENTER (0,0,0) 配置，极性指数 S=0。"""
        from newchan.pipeline import create_context

        ctx = create_context(CENTER)
        assert ctx.polarity == 0

    def test_create_default_states(self) -> None:
        """初始状态：horizontal 阻塞在 CONFIG_BSP，state_machine 全 INACTIVE。"""
        from newchan.pipeline import create_context

        ctx = create_context(FULL_RISK_ON)
        assert ctx.horizontal.current_step is HorizontalStep.CONFIG_BSP
        assert ctx.horizontal.blocked is True
        assert ctx.state_machine.all_inactive()


# ── TestScanConfiguration ──────────────────────────────────


class TestScanConfiguration:
    """测试 scan_configuration：根据极性指数判断方向。"""

    def test_positive_polarity_buy(self) -> None:
        """S > 0 时返回 buy 方向信号。"""
        from newchan.pipeline import create_context, scan_configuration

        ctx = create_context(FULL_RISK_ON)
        new_ctx, signals = scan_configuration(ctx)
        assert any("buy" in s.lower() for s in signals)

    def test_negative_polarity_sell(self) -> None:
        """S < 0 时返回 sell 方向信号。"""
        from newchan.pipeline import create_context, scan_configuration

        config = Configuration(WalkDirection.DOWN, WalkDirection.DOWN, WalkDirection.DOWN)
        ctx = create_context(config)
        new_ctx, signals = scan_configuration(ctx)
        assert any("sell" in s.lower() for s in signals)

    def test_zero_polarity_neutral(self) -> None:
        """S = 0 时返回 neutral 信号。"""
        from newchan.pipeline import create_context, scan_configuration

        ctx = create_context(CENTER)
        new_ctx, signals = scan_configuration(ctx)
        assert any("neutral" in s.lower() for s in signals)


# ── TestLocateBSP ──────────────────────────────────────────


class TestLocateBSP:
    """测试 locate_bsp：提供买卖点推进横向区间套。"""

    def test_advance_with_buy_point(self) -> None:
        """提供 B1 买点，horizontal 从 CONFIG_BSP 推进到 EDGE_BSP。"""
        from newchan.pipeline import create_context, locate_bsp

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1)
        new_ctx = locate_bsp(ctx, bsp)
        assert new_ctx.horizontal.current_step is HorizontalStep.EDGE_BSP

    def test_blocked_without_bsp(self) -> None:
        """不提供买点（NONE 类型），horizontal 维持阻塞。"""
        from newchan.pipeline import create_context, locate_bsp

        ctx = create_context(FULL_RISK_ON)
        none_bsp = _make_bsp(BSPType.NONE)
        new_ctx = locate_bsp(ctx, none_bsp)
        assert new_ctx.horizontal.current_step is HorizontalStep.CONFIG_BSP
        assert new_ctx.horizontal.blocked is True

    def test_complete_six_steps(self) -> None:
        """连续提供 6 个买点，horizontal 完成所有步骤。"""
        from newchan.pipeline import create_context, locate_bsp

        ctx = create_context(FULL_RISK_ON)
        for i in range(6):
            bsp = _make_bsp(BSPType.B1, time=float(i + 1))
            ctx = locate_bsp(ctx, bsp, target=f"target_{i}")
        assert ctx.horizontal.is_complete

    def test_context_immutability(self) -> None:
        """原 ctx 不变，返回新 ctx。"""
        from newchan.pipeline import create_context, locate_bsp

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1)
        new_ctx = locate_bsp(ctx, bsp)
        # 原 ctx 仍阻塞在第一步
        assert ctx.horizontal.current_step is HorizontalStep.CONFIG_BSP
        assert ctx.horizontal.blocked is True
        # 新 ctx 已推进
        assert new_ctx.horizontal.current_step is HorizontalStep.EDGE_BSP


# ── TestComputePosition ────────────────────────────────────


class TestComputePosition:
    """测试 compute_position：根据共振信号计算仓位。"""

    def test_three_layer_resonance(self) -> None:
        """三层共振（ResStrength >= 6），仓位 > 0.5。"""
        from newchan.pipeline import compute_position, create_context

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1, time=1.0)
        signals = [
            _make_resonance_signal(bsp, SignalLayer.CONFIG, time=1.0),
            _make_resonance_signal(bsp, SignalLayer.INDEPENDENT_EDGE, time=1.0),
            _make_resonance_signal(bsp, SignalLayer.UNDERLYING, time=1.0),
        ]
        position, level = compute_position(ctx, signals, _time_tolerance)
        assert position > 0.5
        assert level is ResonanceLevel.THREE_LAYER

    def test_single_layer_position(self) -> None:
        """单层信号（ResStrength in [1,3)），仓位较小。"""
        from newchan.pipeline import compute_position, create_context

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1, time=1.0)
        signals = [
            _make_resonance_signal(bsp, SignalLayer.UNDERLYING, time=1.0),
        ]
        position, level = compute_position(ctx, signals, _time_tolerance)
        assert position < 0.5
        assert level is ResonanceLevel.SINGLE

    def test_no_resonance_zero(self) -> None:
        """无信号，仓位 = 0。"""
        from newchan.pipeline import compute_position, create_context

        ctx = create_context(FULL_RISK_ON)
        position, level = compute_position(ctx, [], _time_tolerance)
        assert position == 0.0


# ── TestExecuteEntry ───────────────────────────────────────


class TestExecuteEntry:
    """测试 execute_entry：入场操作。"""

    def test_l0_entry(self) -> None:
        """L0 入场，状态从 INACTIVE -> ACTIVE。"""
        from newchan.pipeline import create_context, execute_entry

        ctx = create_context(FULL_RISK_ON)
        new_ctx, result = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        assert new_ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.ACTIVE
        assert result.realized_profit == 0.0

    def test_l1_entry_requires_l0_active(self) -> None:
        """L1 入场需要 L0 ACTIVE。L0 未 ACTIVE 时应报错。"""
        from newchan.pipeline import create_context, execute_entry
        from newchan.trading.state_machine import AxiomViolationError

        ctx = create_context(FULL_RISK_ON)
        with pytest.raises(AxiomViolationError):
            execute_entry(ctx, LayerType.L1_EDGE, "独立边一买", 0.3)

    def test_entry_records_cost(self) -> None:
        """入场后 cost_tracker 记录入场成本。"""
        from newchan.pipeline import create_context, execute_entry

        ctx = create_context(FULL_RISK_ON)
        new_ctx, _ = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        assert new_ctx.cost_tracker.total_entries > 0


# ── TestExecuteExit ────────────────────────────────────────


class TestExecuteExit:
    """测试 execute_exit：退出操作。"""

    def test_normal_exit(self) -> None:
        """正常退出，利润 >= 0。"""
        from newchan.pipeline import create_context, execute_entry, execute_exit

        ctx = create_context(FULL_RISK_ON)
        ctx, _ = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        ctx, result = execute_exit(ctx, LayerType.L0_CONFIG, abort=False)
        assert result.realized_profit >= 0.0
        assert ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.INACTIVE

    def test_abort_exit(self) -> None:
        """止损退出，realized_profit < 0。"""
        from newchan.pipeline import create_context, execute_entry, execute_exit

        ctx = create_context(FULL_RISK_ON)
        ctx, _ = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        ctx, result = execute_exit(ctx, LayerType.L0_CONFIG, abort=True)
        assert result.realized_profit < 0.0

    def test_exit_cascades(self) -> None:
        """L0 exit_abort 导致所有下层 INACTIVE。"""
        from newchan.pipeline import create_context, execute_entry, execute_exit

        ctx = create_context(FULL_RISK_ON)
        ctx, _ = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        ctx, _ = execute_entry(ctx, LayerType.L1_EDGE, "独立边一买", 0.3)
        # L0 abort -> L1 应变为 INACTIVE
        ctx, _ = execute_exit(ctx, LayerType.L0_CONFIG, abort=True)
        assert ctx.state_machine.get_state(LayerType.L1_EDGE) is LayerState.INACTIVE


# ── TestCheckNegation ──────────────────────────────────────


class TestCheckNegation:
    """测试 check_negation：买卖点否定判定。"""

    def test_buy_negated(self) -> None:
        """买点被否定：价格跌破买点价。"""
        from newchan.pipeline import check_negation, create_context

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1, price=100.0)
        assert check_negation(ctx, 90.0, bsp) is True

    def test_buy_not_negated(self) -> None:
        """买点未被否定：价格高于买点价。"""
        from newchan.pipeline import check_negation, create_context

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1, price=100.0)
        assert check_negation(ctx, 110.0, bsp) is False


# ── TestPipelineStep ───────────────────────────────────────


class TestPipelineStep:
    """测试 pipeline_step：单步管道推进。"""

    def test_step_with_bsp_advances(self) -> None:
        """有 BSP 时推进 horizontal。"""
        from newchan.pipeline import create_context, pipeline_step

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1)
        new_ctx = pipeline_step(ctx, bsp=bsp)
        assert new_ctx.horizontal.current_step is HorizontalStep.EDGE_BSP

    def test_step_without_bsp_waits(self) -> None:
        """无 BSP 时维持当前状态。"""
        from newchan.pipeline import create_context, pipeline_step

        ctx = create_context(FULL_RISK_ON)
        new_ctx = pipeline_step(ctx)
        assert new_ctx.horizontal.current_step is HorizontalStep.CONFIG_BSP
        assert new_ctx.horizontal.blocked is True

    def test_full_pipeline_buy(self) -> None:
        """完整买入流程：config BSP -> advance -> position -> entry。"""
        from newchan.pipeline import create_context, execute_entry, locate_bsp, pipeline_step

        ctx = create_context(FULL_RISK_ON)
        # 1. 推进横向区间套到 EDGE_BSP
        bsp = _make_bsp(BSPType.B1)
        ctx = locate_bsp(ctx, bsp)
        assert ctx.horizontal.current_step is HorizontalStep.EDGE_BSP
        # 2. 入场 L0
        ctx, result = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        assert ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.ACTIVE
        assert result.realized_profit == 0.0

    def test_full_pipeline_abort(self) -> None:
        """完整止损流程：entry -> negation -> abort。"""
        from newchan.pipeline import (
            check_negation,
            create_context,
            execute_entry,
            execute_exit,
        )

        ctx = create_context(FULL_RISK_ON)
        bsp = _make_bsp(BSPType.B1, price=100.0)
        # 入场
        ctx, _ = execute_entry(ctx, LayerType.L0_CONFIG, "配置层一买", 0.5)
        # 检查否定
        assert check_negation(ctx, 90.0, bsp) is True
        # 止损退出
        ctx, result = execute_exit(ctx, LayerType.L0_CONFIG, abort=True)
        assert result.realized_profit < 0.0
        assert ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.INACTIVE
