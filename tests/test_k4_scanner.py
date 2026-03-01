"""K4 配置读取器测试。

认识论标注：L0（从配置空间定义直接推导的映射）
"""

from __future__ import annotations

import pytest

from newchan.a_move_v1 import Move
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.topology.config_space import Configuration, WalkDirection, polarity_index
from newchan.topology.k4_scanner import (
    K4ScanResult,
    k4_configuration,
    scan_k4,
    walk_direction_from_snapshot,
)


# ── 辅助工厂 ──


def _make_move(
    direction: str = "up",
    kind: str = "trend",
    settled: bool = True,
) -> Move:
    """构造最小 Move 实例。"""
    return Move(
        kind=kind,  # type: ignore[arg-type]
        direction=direction,  # type: ignore[arg-type]
        seg_start=0,
        seg_end=1,
        zs_start=0,
        zs_end=0,
        zs_count=1,
        settled=settled,
    )


def _make_move_snapshot(
    moves: list[Move] | None = None,
    bar_ts: float = 100.0,
) -> MoveSnapshot:
    """构造最小 MoveSnapshot。"""
    return MoveSnapshot(
        bar_idx=0,
        bar_ts=bar_ts,
        moves=moves if moves is not None else [],
        events=[],
    )


def _make_recursive_level_snapshot(
    moves: list[Move] | None = None,
    level_id: int = 2,
) -> RecursiveLevelSnapshot:
    """构造最小 RecursiveLevelSnapshot。"""
    return RecursiveLevelSnapshot(
        bar_idx=0,
        bar_ts=100.0,
        level_id=level_id,
        zhongshus=[],
        moves=moves if moves is not None else [],
        zhongshu_events=[],
        move_events=[],
    )


def _make_orchestrator_snapshot(
    moves: list[Move] | None = None,
    recursive_snapshots: list[RecursiveLevelSnapshot] | None = None,
    bar_ts: float = 100.0,
) -> RecursiveOrchestratorSnapshot:
    """构造最小 RecursiveOrchestratorSnapshot（只填充走势相关字段）。

    bi_snapshot / seg_snapshot / zs_snapshot / bsp_snapshot 使用 None
    因为 k4_scanner 只读取 move_snapshot 和 recursive_snapshots。
    """
    return RecursiveOrchestratorSnapshot(
        bar_idx=0,
        bar_ts=bar_ts,
        bi_snapshot=None,  # type: ignore[arg-type]
        seg_snapshot=None,  # type: ignore[arg-type]
        zs_snapshot=None,  # type: ignore[arg-type]
        move_snapshot=_make_move_snapshot(moves, bar_ts=bar_ts),
        bsp_snapshot=None,  # type: ignore[arg-type]
        recursive_snapshots=recursive_snapshots or [],
    )


# ── walk_direction_from_snapshot 测试 ──


class TestWalkDirectionFromSnapshot:
    """从快照读取走势方向。"""

    def test_up_trend_returns_up(self) -> None:
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend", settled=True)],
        )
        assert walk_direction_from_snapshot(snap) == WalkDirection.UP

    def test_down_trend_returns_down(self) -> None:
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="down", kind="trend", settled=True)],
        )
        assert walk_direction_from_snapshot(snap) == WalkDirection.DOWN

    def test_consolidation_returns_flat(self) -> None:
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="consolidation", settled=True)],
        )
        assert walk_direction_from_snapshot(snap) == WalkDirection.FLAT

    def test_empty_moves_returns_flat(self) -> None:
        snap = _make_orchestrator_snapshot(moves=[])
        assert walk_direction_from_snapshot(snap) == WalkDirection.FLAT

    def test_multiple_moves_uses_last_settled(self) -> None:
        """多个 move 时取最后一个 settled move（与 D 算子读数统一）。"""
        snap = _make_orchestrator_snapshot(
            moves=[
                _make_move(direction="up", kind="trend", settled=True),
                _make_move(direction="down", kind="trend", settled=False),
            ],
        )
        # 只有第一个 move 是 settled，所以方向 = UP
        assert walk_direction_from_snapshot(snap) == WalkDirection.UP

    def test_unsettled_only_returns_flat(self) -> None:
        """只有未 settled 的 move → FLAT（与 D 算子读数统一）。"""
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend", settled=False)],
        )
        assert walk_direction_from_snapshot(snap) == WalkDirection.FLAT

    def test_last_settled_among_multiple(self) -> None:
        """多个 settled move 时取最后一个 settled。"""
        snap = _make_orchestrator_snapshot(
            moves=[
                _make_move(direction="up", kind="trend", settled=True),
                _make_move(direction="down", kind="trend", settled=True),
                _make_move(direction="up", kind="trend", settled=False),
            ],
        )
        assert walk_direction_from_snapshot(snap) == WalkDirection.DOWN

    def test_recursive_level(self) -> None:
        """level > 0 读取 recursive_snapshots。"""
        rl = _make_recursive_level_snapshot(
            moves=[_make_move(direction="down", kind="trend", settled=True)],
            level_id=2,
        )
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend", settled=True)],
            recursive_snapshots=[rl],
        )
        # level=1 → recursive_snapshots[0]
        assert walk_direction_from_snapshot(snap, level=1) == WalkDirection.DOWN

    def test_recursive_level_out_of_range_returns_flat(self) -> None:
        """递归层索引越界时返回 FLAT。"""
        snap = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend", settled=True)],
            recursive_snapshots=[],
        )
        assert walk_direction_from_snapshot(snap, level=1) == WalkDirection.FLAT


# ── k4_configuration 测试 ──


class TestK4Configuration:
    """三元组正确组合。"""

    def test_all_up(self) -> None:
        e = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        au = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        r = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        cfg = k4_configuration(e, au, r)
        assert cfg == Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.UP)

    def test_mixed_directions(self) -> None:
        e = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        au = _make_orchestrator_snapshot(
            moves=[_make_move(direction="down", kind="consolidation")],
        )
        r = _make_orchestrator_snapshot(
            moves=[_make_move(direction="down", kind="trend")],
        )
        cfg = k4_configuration(e, au, r)
        assert cfg == Configuration(
            WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN,
        )

    def test_all_empty(self) -> None:
        e = _make_orchestrator_snapshot(moves=[])
        au = _make_orchestrator_snapshot(moves=[])
        r = _make_orchestrator_snapshot(moves=[])
        cfg = k4_configuration(e, au, r)
        assert cfg == Configuration(
            WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.FLAT,
        )


# ── scan_k4 测试 ──


class TestScanK4:
    """完整 K4 扫描结果。"""

    def test_full_result_fields(self) -> None:
        e = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
            bar_ts=1000.0,
        )
        au = _make_orchestrator_snapshot(
            moves=[_make_move(direction="down", kind="trend")],
            bar_ts=1000.0,
        )
        r = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="consolidation")],
            bar_ts=1000.0,
        )
        result = scan_k4(e, au, r)
        expected_cfg = Configuration(
            WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT,
        )
        assert result.config == expected_cfg
        assert result.polarity == polarity_index(expected_cfg)
        assert result.e_direction == WalkDirection.UP
        assert result.au_direction == WalkDirection.DOWN
        assert result.r_direction == WalkDirection.FLAT
        assert result.bar_ts == 1000.0

    def test_result_is_frozen(self) -> None:
        e = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        au = _make_orchestrator_snapshot(moves=[])
        r = _make_orchestrator_snapshot(moves=[])
        result = scan_k4(e, au, r)
        with pytest.raises(AttributeError):
            result.polarity = 99  # type: ignore[misc]

    def test_polarity_full_risk_on(self) -> None:
        snap_up = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
        )
        result = scan_k4(snap_up, snap_up, snap_up)
        assert result.polarity == 3

    def test_polarity_full_risk_off(self) -> None:
        snap_down = _make_orchestrator_snapshot(
            moves=[_make_move(direction="down", kind="trend")],
        )
        result = scan_k4(snap_down, snap_down, snap_down)
        assert result.polarity == -3

    def test_bar_ts_from_e_snapshot(self) -> None:
        """bar_ts 取自 e_snapshot。"""
        e = _make_orchestrator_snapshot(
            moves=[_make_move(direction="up", kind="trend")],
            bar_ts=42.5,
        )
        au = _make_orchestrator_snapshot(
            moves=[],
            bar_ts=99.0,
        )
        r = _make_orchestrator_snapshot(
            moves=[],
            bar_ts=88.0,
        )
        result = scan_k4(e, au, r)
        assert result.bar_ts == 42.5
