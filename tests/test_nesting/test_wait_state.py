"""等待状态测试。"""

from __future__ import annotations

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.horizontal import HorizontalStep
from newchan.nesting.wait_state import (
    WaitState,
    can_operate,
    max_position,
)


def _make_wait(step: HorizontalStep, scope: frozenset[str] | None = None) -> WaitState:
    if scope is None:
        scope = frozenset(["E/$", "C/$", "R/$"])
    return WaitState(step=step, scope=scope, level=3, time=100.0)


def _make_bsp(edge_id: str = "E/$", bsp_type: BSPType = BSPType.B1) -> BSP:
    return BSP(edge_id=edge_id, level=3, time=105.0, bsp_type=bsp_type, price=50.0)


class TestCanOperate:
    def test_signal_in_scope(self):
        """§8.3: 搜索空间内出现买卖点 -> 可操作。"""
        wait = _make_wait(HorizontalStep.EDGE_BSP)
        signal = _make_bsp(edge_id="E/$")
        assert can_operate(wait, signal)

    def test_signal_not_in_scope(self):
        """信号不在搜索空间 -> 不可操作。"""
        wait = _make_wait(HorizontalStep.EDGE_BSP, scope=frozenset(["C/$"]))
        signal = _make_bsp(edge_id="E/$")
        assert not can_operate(wait, signal)

    def test_no_signal(self):
        """无信号 -> 不可操作。"""
        wait = _make_wait(HorizontalStep.EDGE_BSP)
        assert not can_operate(wait, None)

    def test_none_bsp_type(self):
        """BSPType.NONE 不算信号。"""
        wait = _make_wait(HorizontalStep.EDGE_BSP)
        signal = _make_bsp(bsp_type=BSPType.NONE)
        assert not can_operate(wait, signal)


class TestMaxPosition:
    def test_step1_light_position(self):
        """§8.4: 等配置层买点 -> 轻仓。"""
        wait = _make_wait(HorizontalStep.CONFIG_BSP)
        assert max_position(wait) == 0.1

    def test_step2_medium_position(self):
        """§8.4: 配置层已确认 -> 中等仓位。"""
        wait = _make_wait(HorizontalStep.EDGE_BSP)
        assert max_position(wait) == 0.3

    def test_step6_high_position(self):
        """§8.4: 标的已确认 -> 高仓位但不满仓。"""
        wait = _make_wait(HorizontalStep.VERTICAL_ENTRY)
        assert max_position(wait) == 0.9

    def test_position_increases_with_steps(self):
        """仓位上限随步骤递增。"""
        positions = []
        for step in HorizontalStep:
            wait = _make_wait(step)
            positions.append(max_position(wait))
        # 每步仓位 >= 前一步
        for i in range(1, len(positions)):
            assert positions[i] >= positions[i - 1]

    def test_never_full_position(self):
        """等待状态不允许满仓。"""
        for step in HorizontalStep:
            wait = _make_wait(step)
            assert max_position(wait) < 1.0
