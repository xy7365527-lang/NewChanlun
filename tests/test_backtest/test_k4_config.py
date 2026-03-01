"""k4_config.py 测试 — K4 配置读取 + D 算子读数。

认识论标注：L1（合成数据，验证管线正确性）。
"""

from __future__ import annotations

from dataclasses import dataclass, field

from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.backtest.k4_config import read_k4_config
from newchan.backtest.types import Direction
from newchan.topology.config_space import WalkDirection


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


def _make_move(direction: str = "up", kind: str = "trend", settled: bool = True) -> Move:
    return Move(
        kind=kind,
        direction=direction,
        seg_start=0, seg_end=2,
        zs_start=0, zs_end=0,
        zs_count=1 if kind == "consolidation" else 2,
        settled=settled,
        high=110.0, low=90.0,
    )


# ── Tests ──


class TestReadK4Config:
    """K4 配置读取。"""

    def test_all_flat(self):
        """三标的均无走势 → 全 FLAT，polarity=0。"""
        snap = _FakeSnapshot()
        k4 = read_k4_config(snap, snap, snap)
        assert k4.polarity == 0
        assert k4.e.walk_direction == WalkDirection.FLAT
        assert k4.au.walk_direction == WalkDirection.FLAT
        assert k4.r.walk_direction == WalkDirection.FLAT

    def test_all_up(self):
        """三标的均上涨趋势 → polarity=3。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="trend", settled=True),
            ]),
        )
        k4 = read_k4_config(snap, snap, snap)
        assert k4.polarity == 3
        assert k4.config.label == "(+,+,+)"

    def test_mixed(self):
        """E=up, Au=flat, R=down → polarity=0。"""
        e_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="trend", settled=True),
            ]),
        )
        au_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="consolidation", settled=True),
            ]),
        )
        r_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="down", kind="trend", settled=True),
            ]),
        )
        k4 = read_k4_config(e_snap, au_snap, r_snap)
        # consolidation → FLAT, so polarity = UP + FLAT + DOWN = 0
        assert k4.polarity == 0
        assert k4.config.label == "(+,0,-)"

    def test_d_readings_attached(self):
        """D 算子读数正确附加到每个标的。"""
        snap_with_moves = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[
                Zhongshu(zd=95, zg=105, seg_start=0, seg_end=2, seg_count=3, settled=False),
            ]),
        )
        snap_empty = _FakeSnapshot()

        k4 = read_k4_config(snap_with_moves, snap_empty, snap_empty)

        # E/$ has up direction and unsettled zhongshu (absorption=True)
        assert k4.e.d_reading.direction == Direction.UP
        assert k4.e.d_reading.absorption is True

        # Au/$ and R/$ are flat with no absorption
        assert k4.au.d_reading.direction == Direction.FLAT
        assert k4.r.d_reading.direction == Direction.FLAT

    def test_custom_symbols(self):
        """自定义标的代码。"""
        snap = _FakeSnapshot()
        k4 = read_k4_config(
            snap, snap, snap,
            e_symbol="QQQ", au_symbol="SLV", r_symbol="IEF",
        )
        assert k4.e.symbol == "QQQ"
        assert k4.au.symbol == "SLV"
        assert k4.r.symbol == "IEF"
