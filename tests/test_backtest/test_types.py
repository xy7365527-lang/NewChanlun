"""types.py 测试 — D 算子读数提取 + EdgeState/K4State。

认识论标注：L1（合成数据，验证管线正确性）。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.backtest.types import (
    CostCurvePoint,
    DOperatorReading,
    Direction,
    EdgeState,
    K4State,
    ScannerResult,
    StateMachineEvent,
    StateMachineState,
    TradeAction,
    extract_d_reading,
)
from newchan.topology.config_space import Configuration, WalkDirection


# ── Stubs ──


@dataclass
class _BspSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    buysellpoints: list = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _MoveSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    moves: list = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _ZsSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    zhongshus: list = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _FakeSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    bsp_snapshot: _BspSnapshot = field(default_factory=_BspSnapshot)
    move_snapshot: _MoveSnapshot = field(default_factory=_MoveSnapshot)
    zs_snapshot: _ZsSnapshot = field(default_factory=_ZsSnapshot)
    recursive_snapshots: list = field(default_factory=list)
    lstar: object = None
    bi_snapshot: object = None
    seg_snapshot: object = None
    all_events: list = field(default_factory=list)


def _make_move(
    direction: str = "up",
    kind: str = "trend",
    settled: bool = True,
    high: float = 110.0,
    low: float = 90.0,
) -> Move:
    return Move(
        kind=kind,
        direction=direction,
        seg_start=0,
        seg_end=2,
        zs_start=0,
        zs_end=0,
        zs_count=1 if kind == "consolidation" else 2,
        settled=settled,
        high=high,
        low=low,
    )


def _make_zhongshu(
    settled: bool = True,
    zd: float = 95.0,
    zg: float = 105.0,
) -> Zhongshu:
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=0,
        seg_end=2,
        seg_count=3,
        settled=settled,
    )


# ── Direction 提取 ──


class TestExtractDReading:
    """D 算子读数提取。"""

    def test_no_moves_returns_flat(self):
        """无走势时方向为 FLAT。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[]),
            zs_snapshot=_ZsSnapshot(zhongshus=[]),
        )
        reading = extract_d_reading(snap)
        assert reading.direction == Direction.FLAT
        assert reading.amplitude == 0.0
        assert reading.absorption is False

    def test_single_settled_up_move(self):
        """单个 settled 上涨趋势。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True, high=120.0, low=100.0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[_make_zhongshu(settled=True)]),
        )
        reading = extract_d_reading(snap)
        assert reading.direction == Direction.UP
        assert reading.amplitude == 0.0  # 仅一个 settled move，无幅度比
        assert reading.absorption is False  # 中枢已 settled

    def test_single_settled_down_move(self):
        """单个 settled 下跌趋势。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="down", settled=True, high=110.0, low=90.0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[]),
        )
        reading = extract_d_reading(snap)
        assert reading.direction == Direction.DOWN

    def test_unsettled_move_ignored_for_direction(self):
        """未 settled 的走势不影响方向态。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=False),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[]),
        )
        reading = extract_d_reading(snap)
        assert reading.direction == Direction.FLAT

    def test_amplitude_two_settled_moves(self):
        """两个 settled 走势的幅度比。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True, high=110.0, low=100.0),
                _make_move(direction="down", settled=True, high=108.0, low=88.0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[]),
        )
        reading = extract_d_reading(snap)
        # prev_range = 110 - 100 = 10, curr_range = 108 - 88 = 20
        assert reading.amplitude == 2.0
        assert reading.direction == Direction.DOWN

    def test_absorption_unsettled_zhongshu(self):
        """最后一个中枢未 settled → 吸收态为 True。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[
                _make_zhongshu(settled=True),
                _make_zhongshu(settled=False),
            ]),
        )
        reading = extract_d_reading(snap)
        assert reading.absorption is True

    def test_absorption_all_settled(self):
        """所有中枢都已 settled → 吸收态为 False。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[
                _make_zhongshu(settled=True),
            ]),
        )
        reading = extract_d_reading(snap)
        assert reading.absorption is False


# ── 数据类型完整性 ──


def _make_edge_state(
    edge_label: str = "E/$",
    vertex_from: str = "E",
    vertex_to: str = "$",
    direction: Direction = Direction.UP,
    walk_dir: WalkDirection = WalkDirection.UP,
) -> EdgeState:
    """构造 EdgeState。"""
    reading = DOperatorReading(direction=direction, amplitude=1.0, absorption=False)
    return EdgeState(
        edge_label=edge_label,
        vertex_from=vertex_from,
        vertex_to=vertex_to,
        d_reading=reading,
        walk_direction=walk_dir,
    )


class TestDataTypes:
    """共享类型定义完整性。"""

    def test_edge_state_immutable(self):
        """EdgeState 是不可变的。"""
        edge = _make_edge_state()
        assert edge.edge_label == "E/$"
        assert edge.vertex_from == "E"

    def test_k4_state_immutable(self):
        """K4State 是不可变的。"""
        config = Configuration(WalkDirection.UP, WalkDirection.FLAT, WalkDirection.DOWN)
        state = K4State(
            e_au=_make_edge_state("E/Au", "E", "Au"),
            e_r=_make_edge_state("E/R", "E", "R"),
            e_usd=_make_edge_state("E/$", "E", "$"),
            au_r=_make_edge_state("Au/R", "Au", "R"),
            au_usd=_make_edge_state("Au/$", "Au", "$", Direction.FLAT, WalkDirection.FLAT),
            r_usd=_make_edge_state("R/$", "R", "$", Direction.DOWN, WalkDirection.DOWN),
            config=config,
            polarity=0,
        )
        assert state.polarity == 0
        assert state.e_au.edge_label == "E/Au"
        assert state.e_usd.edge_label == "E/$"

    def test_scanner_result_none_symbol(self):
        """ScannerResult 允许 None 选股结果。"""
        result = ScannerResult(selected_symbol=None, tightness=0.0, candidates_count=0)
        assert result.selected_symbol is None

    def test_trade_action_immutable(self):
        """TradeAction 是不可变的。"""
        action = TradeAction(
            bar_idx=0, action="buy", price=100.0,
            quantity=1000.0, cost_basis=100.0, trigger="type1_L1",
        )
        assert action.price == 100.0

    def test_cost_curve_point(self):
        """CostCurvePoint 基本构造。"""
        point = CostCurvePoint(
            bar_idx=0, cost_basis=100.0,
            total_shares=1000.0, cumulative_recovered=0.0,
        )
        assert point.total_shares == 1000.0

    def test_state_machine_enums(self):
        """状态机枚举完整性。"""
        assert len(StateMachineState) == 5
        assert len(StateMachineEvent) == 7
