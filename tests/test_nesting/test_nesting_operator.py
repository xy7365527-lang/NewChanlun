"""统一区间套算子测试。"""

from __future__ import annotations

import pytest

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.nesting_operator import (
    NestingPath,
    NestingStep,
    NestingType,
    append_step,
    check_level_monotonicity,
    check_search_space_contraction,
    check_termination,
    validate_path,
)


def _make_bsp(bsp_type: BSPType = BSPType.B1) -> BSP:
    return BSP(edge_id="E/$", level=1, time=100.0, bsp_type=bsp_type, price=50.0)


def _make_step(
    nesting_type: NestingType = NestingType.HORIZONTAL,
    search_space: frozenset[str] | None = None,
    level: int = 3,
    target: str = "E",
    bsp: BSP | None = None,
) -> NestingStep:
    if search_space is None:
        search_space = frozenset(["E/$", "C/$", "R/$"])
    return NestingStep(
        nesting_type=nesting_type,
        search_space=search_space,
        level=level,
        target=target,
        bsp_confirmation=bsp,
    )


class TestNestingPath:
    def test_empty_path(self):
        path = NestingPath(steps=())
        assert path.is_empty
        assert path.current_level is None
        assert path.current_target is None

    def test_single_step(self):
        step = _make_step(level=3, target="E")
        path = NestingPath(steps=(step,))
        assert not path.is_empty
        assert path.current_level == 3
        assert path.current_target == "E"

    def test_append_step(self):
        path = NestingPath(steps=())
        step = _make_step()
        new_path = append_step(path, step)
        assert len(new_path.steps) == 1
        assert path.is_empty  # 原路径不变


class TestLevelMonotonicity:
    def test_monotone_decreasing(self):
        """约束1: k1 >= k2 >= ... >= km。"""
        path = NestingPath(steps=(
            _make_step(level=5),
            _make_step(level=3),
            _make_step(level=1),
        ))
        assert check_level_monotonicity(path)

    def test_constant_level(self):
        path = NestingPath(steps=(
            _make_step(level=3),
            _make_step(level=3),
        ))
        assert check_level_monotonicity(path)

    def test_violation(self):
        """级别增加 = 违规。"""
        path = NestingPath(steps=(
            _make_step(level=3),
            _make_step(level=5),
        ))
        assert not check_level_monotonicity(path)

    def test_empty_path(self):
        path = NestingPath(steps=())
        assert check_level_monotonicity(path)


class TestSearchSpaceContraction:
    def test_contracting(self):
        """约束2: |S1| >= |S2| >= ... >= |Sm|。"""
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["a", "b", "c"])),
            _make_step(search_space=frozenset(["a", "b"])),
            _make_step(search_space=frozenset(["a"])),
        ))
        assert check_search_space_contraction(path)

    def test_violation(self):
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["a"])),
            _make_step(search_space=frozenset(["a", "b"])),
        ))
        assert not check_search_space_contraction(path)


class TestTermination:
    def test_terminated(self):
        """约束3: 终止于具体标的的买卖点。"""
        bsp = _make_bsp(BSPType.B1)
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["AAPL"]), bsp=bsp),
        ))
        assert check_termination(path)

    def test_not_terminated_no_bsp(self):
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["AAPL"]), bsp=None),
        ))
        assert not check_termination(path)

    def test_not_terminated_multiple_targets(self):
        bsp = _make_bsp(BSPType.B1)
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["AAPL", "MSFT"]), bsp=bsp),
        ))
        assert not check_termination(path)

    def test_not_terminated_none_bsp_type(self):
        bsp = _make_bsp(BSPType.NONE)
        path = NestingPath(steps=(
            _make_step(search_space=frozenset(["AAPL"]), bsp=bsp),
        ))
        assert not check_termination(path)

    def test_empty_path(self):
        path = NestingPath(steps=())
        assert not check_termination(path)


class TestValidatePath:
    def test_valid_complete_path(self):
        """完整合法路径。"""
        bsp = _make_bsp(BSPType.B1)
        path = NestingPath(steps=(
            _make_step(level=5, search_space=frozenset(["a", "b", "c"])),
            _make_step(level=3, search_space=frozenset(["a", "b"])),
            _make_step(level=1, search_space=frozenset(["a"]), bsp=bsp),
        ))
        is_valid, violations = validate_path(path)
        assert is_valid
        assert violations == []

    def test_multiple_violations(self):
        """多重违规。"""
        path = NestingPath(steps=(
            _make_step(level=1, search_space=frozenset(["a"])),
            _make_step(level=3, search_space=frozenset(["a", "b"])),
        ))
        is_valid, violations = validate_path(path)
        assert not is_valid
        assert "level_monotonicity_violated" in violations
        assert "search_space_contraction_violated" in violations
        assert "termination_not_reached" in violations
