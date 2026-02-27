"""横向区间套六步状态机测试。"""

from __future__ import annotations

import pytest

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.horizontal import (
    HorizontalNesting,
    HorizontalStep,
    advance,
    create_horizontal_nesting,
    entry_condition,
    is_blocked,
)


def _make_bsp(bsp_type: BSPType = BSPType.B1, edge_id: str = "E/$") -> BSP:
    return BSP(edge_id=edge_id, level=1, time=100.0, bsp_type=bsp_type, price=50.0)


class TestHorizontalStep:
    def test_only_country_can_skip(self):
        """§1.4: 仅 step_3（选国家）可跳过。"""
        for step in HorizontalStep:
            if step is HorizontalStep.COUNTRY:
                assert step.can_skip
            else:
                assert not step.can_skip

    def test_next_step_sequence(self):
        """六步按顺序推进。"""
        expected = [
            HorizontalStep.EDGE_BSP,
            HorizontalStep.COUNTRY,
            HorizontalStep.SECTOR,
            HorizontalStep.STOCK,
            HorizontalStep.VERTICAL_ENTRY,
            None,
        ]
        for step, expected_next in zip(HorizontalStep, expected):
            assert step.next_step is expected_next


class TestHorizontalNesting:
    def test_initial_state(self):
        """初始状态阻塞在第一步。"""
        state = create_horizontal_nesting()
        assert state.current_step is HorizontalStep.CONFIG_BSP
        assert state.blocked
        assert len(state.completed_results) == 0
        assert not state.is_complete

    def test_blocking_without_bsp(self):
        """§1.4: 无买卖点时阻塞。"""
        state = create_horizontal_nesting()
        new_state = advance(state, None)
        assert is_blocked(new_state)
        assert new_state.current_step is HorizontalStep.CONFIG_BSP

    def test_advance_with_bsp(self):
        """有买卖点时推进到下一步。"""
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        new_state = advance(state, bsp, target="E")
        assert new_state.current_step is HorizontalStep.EDGE_BSP
        assert len(new_state.completed_results) == 1
        assert new_state.completed_results[0].target == "E"

    def test_skip_country_step(self):
        """§1.4: 第三步可跳过。"""
        # 推进到第三步
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        state = advance(state, bsp, target="risk-on")  # step 1
        state = advance(state, bsp, target="E")  # step 2
        assert state.current_step is HorizontalStep.COUNTRY

        # 跳过第三步（无买卖点）
        state = advance(state, None, target="all_countries")
        assert state.current_step is HorizontalStep.SECTOR
        assert state.completed_results[-1].skipped

    def test_non_skippable_step_blocks(self):
        """非第三步无买卖点时阻塞而不跳过。"""
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        state = advance(state, bsp, target="risk-on")  # step 1 完成
        # step 2 无买卖点
        state = advance(state, None)
        assert state.current_step is HorizontalStep.EDGE_BSP
        assert is_blocked(state)

    def test_full_six_steps(self):
        """六步全部完成。"""
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        targets = ["risk-on", "E", "US", "tech", "AAPL", "entry"]
        for target in targets:
            state = advance(state, bsp, target=target)
        assert state.is_complete
        assert len(state.completed_results) == 6

    def test_none_bsp_type_does_not_advance(self):
        """BSPType.NONE 不算有效买卖点。"""
        state = create_horizontal_nesting()
        none_bsp = _make_bsp(BSPType.NONE)
        state = advance(state, none_bsp)
        assert is_blocked(state)
        assert state.current_step is HorizontalStep.CONFIG_BSP


class TestEntryCondition:
    def test_first_step_no_prerequisite(self):
        """第一步无前置条件。"""
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        assert entry_condition(state, bsp)

    def test_first_step_requires_bsp(self):
        """第一步仍需要买卖点。"""
        state = create_horizontal_nesting()
        assert not entry_condition(state, None)

    def test_later_step_requires_previous_completion(self):
        """后续步需要上一步完成。"""
        state = create_horizontal_nesting()
        bsp = _make_bsp()
        state = advance(state, bsp, target="risk-on")
        assert entry_condition(state, bsp)
