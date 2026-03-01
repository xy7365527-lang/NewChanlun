"""k4_config.py 测试 — K4 六条边配置读取 + D 算子读数。

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
    """K4 六条边配置读取。"""

    def test_all_flat(self):
        """六条边均无走势 → 全 FLAT，polarity=0。"""
        snap = _FakeSnapshot()
        k4 = read_k4_config(snap, snap, snap, snap, snap, snap)
        assert k4.polarity == 0
        assert k4.e_usd.walk_direction == WalkDirection.FLAT
        assert k4.au_usd.walk_direction == WalkDirection.FLAT
        assert k4.r_usd.walk_direction == WalkDirection.FLAT
        assert k4.e_au.walk_direction == WalkDirection.FLAT
        assert k4.e_r.walk_direction == WalkDirection.FLAT
        assert k4.au_r.walk_direction == WalkDirection.FLAT

    def test_all_up(self):
        """六条边均上涨趋势 → polarity=3（E/$, Au/$, R/$ 均 UP）。"""
        snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="trend", settled=True),
            ]),
        )
        k4 = read_k4_config(snap, snap, snap, snap, snap, snap)
        assert k4.polarity == 3
        assert k4.config.label == "(+,+,+)"

    def test_mixed(self):
        """E/$=up, Au/$=flat, R/$=down → polarity=0。"""
        up_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="trend", settled=True),
            ]),
        )
        flat_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", kind="consolidation", settled=True),
            ]),
        )
        down_snap = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="down", kind="trend", settled=True),
            ]),
        )
        empty_snap = _FakeSnapshot()
        # 参数顺序：e_au, e_r, e_usd, au_r, au_usd, r_usd
        k4 = read_k4_config(
            empty_snap,   # E/Au
            empty_snap,   # E/R
            up_snap,      # E/$
            empty_snap,   # Au/R
            flat_snap,    # Au/$
            down_snap,    # R/$
        )
        assert k4.polarity == 0
        assert k4.config.label == "(+,0,-)"

    def test_d_readings_attached(self):
        """D 算子读数正确附加到每条边。"""
        snap_with_moves = _FakeSnapshot(
            move_snapshot=_MoveSnapshot(moves=[
                _make_move(direction="up", settled=True),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[
                Zhongshu(zd=95, zg=105, seg_start=0, seg_end=2, seg_count=3, settled=False),
            ]),
        )
        snap_empty = _FakeSnapshot()

        k4 = read_k4_config(
            snap_empty,       # E/Au
            snap_empty,       # E/R
            snap_with_moves,  # E/$
            snap_empty,       # Au/R
            snap_empty,       # Au/$
            snap_empty,       # R/$
        )

        # E/$ has up direction and unsettled zhongshu (absorption=True)
        assert k4.e_usd.d_reading.direction == Direction.UP
        assert k4.e_usd.d_reading.absorption is True

        # Au/$ and R/$ are flat with no absorption
        assert k4.au_usd.d_reading.direction == Direction.FLAT
        assert k4.r_usd.d_reading.direction == Direction.FLAT

    def test_edge_labels(self):
        """六条边的标签正确。"""
        snap = _FakeSnapshot()
        k4 = read_k4_config(snap, snap, snap, snap, snap, snap)
        assert k4.e_au.edge_label == "E/Au"
        assert k4.e_r.edge_label == "E/R"
        assert k4.e_usd.edge_label == "E/$"
        assert k4.au_r.edge_label == "Au/R"
        assert k4.au_usd.edge_label == "Au/$"
        assert k4.r_usd.edge_label == "R/$"

    def test_vertex_pairs(self):
        """六条边的顶点对正确。"""
        snap = _FakeSnapshot()
        k4 = read_k4_config(snap, snap, snap, snap, snap, snap)
        assert (k4.e_au.vertex_from, k4.e_au.vertex_to) == ("E", "Au")
        assert (k4.e_r.vertex_from, k4.e_r.vertex_to) == ("E", "R")
        assert (k4.e_usd.vertex_from, k4.e_usd.vertex_to) == ("E", "$")
        assert (k4.au_r.vertex_from, k4.au_r.vertex_to) == ("Au", "R")
        assert (k4.au_usd.vertex_from, k4.au_usd.vertex_to) == ("Au", "$")
        assert (k4.r_usd.vertex_from, k4.r_usd.vertex_to) == ("R", "$")
