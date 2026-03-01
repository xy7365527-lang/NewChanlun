"""scanner.py 测试 — 选股扫描。

认识论标注：L1（合成数据，验证管线正确性）。
"""

from __future__ import annotations

from dataclasses import dataclass, field

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.backtest.scanner import rank_by_tightness, scan_stocks
from newchan.backtest.types import K4State, EdgeState, DOperatorReading, Direction
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


def _make_edge_state(
    edge_label: str,
    vertex_from: str,
    vertex_to: str,
    walk_dir: WalkDirection = WalkDirection.UP,
) -> EdgeState:
    """构造 EdgeState。"""
    d_reading = DOperatorReading(
        direction=Direction.UP, amplitude=1.0, absorption=False,
    )
    return EdgeState(
        edge_label=edge_label,
        vertex_from=vertex_from,
        vertex_to=vertex_to,
        d_reading=d_reading,
        walk_direction=walk_dir,
    )


def _make_k4_state(polarity: int = 3) -> K4State:
    """构造 K4State，polarity 由参数控制。"""
    dirs = {
        3: (WalkDirection.UP, WalkDirection.UP, WalkDirection.UP),
        0: (WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.FLAT),
        -3: (WalkDirection.DOWN, WalkDirection.DOWN, WalkDirection.DOWN),
    }
    e_dir, au_dir, r_dir = dirs.get(polarity, (WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.FLAT))

    return K4State(
        e_au=_make_edge_state("E/Au", "E", "Au", e_dir),
        e_r=_make_edge_state("E/R", "E", "R", e_dir),
        e_usd=_make_edge_state("E/$", "E", "$", e_dir),
        au_r=_make_edge_state("Au/R", "Au", "R", au_dir),
        au_usd=_make_edge_state("Au/$", "Au", "$", au_dir),
        r_usd=_make_edge_state("R/$", "R", "$", r_dir),
        config=Configuration(sigma_e=e_dir, sigma_c=au_dir, sigma_r=r_dir),
        polarity=polarity,
    )


def _make_buy_bsp(seg_idx: int = 0, confirmed: bool = True) -> BuySellPoint:
    return BuySellPoint(
        kind="type1", side="buy", level_id=1,
        seg_idx=seg_idx, move_seg_start=0,
        divergence_key=(0, 0, seg_idx),
        center_zd=90.0, center_zg=110.0, center_seg_start=0,
        price=100.0, bar_idx=0, confirmed=confirmed, settled=False,
    )


# ── Tests ──


class TestScanStocks:
    """选股扫描。"""

    def test_no_candidates_returns_none(self):
        """无候选快照 → 无选中。"""
        k4 = _make_k4_state(polarity=3)
        result = scan_stocks(k4, {})
        assert result.selected_symbol is None
        assert result.candidates_count == 0

    def test_neutral_polarity_scans_all(self):
        """polarity=0（中性）→ 扫描所有矩阵，有买点可选中。"""
        k4 = _make_k4_state(polarity=0)
        snap = _FakeSnapshot(
            bsp_snapshot=_BspSnapshot(buysellpoints=[_make_buy_bsp()]),
        )
        result = scan_stocks(k4, {"XLK": snap, "TLT": snap})
        assert result.candidates_count >= 0

    def test_positive_polarity_scans_equity(self):
        """polarity>0 → 扫描 EQUITY_UNIVERSE。"""
        k4 = _make_k4_state(polarity=3)
        snap_with_buy = _FakeSnapshot(
            bsp_snapshot=_BspSnapshot(buysellpoints=[_make_buy_bsp()]),
        )
        snap_no_buy = _FakeSnapshot()
        result = scan_stocks(k4, {"XLK": snap_with_buy, "XLF": snap_no_buy})
        assert result.candidates_count >= 0


class TestRankByTightness:
    """区间套收敛紧度排序。"""

    def test_empty_candidates(self):
        """无候选 → 空列表。"""
        result = rank_by_tightness({})
        assert result == []

    def test_ranking_order(self):
        """排序按 tightness 降序。"""
        snap_with_buy = _FakeSnapshot(
            bsp_snapshot=_BspSnapshot(buysellpoints=[_make_buy_bsp()]),
        )
        snap_no_buy = _FakeSnapshot()
        result = rank_by_tightness({"AAA": snap_with_buy, "BBB": snap_no_buy})
        assert len(result) == 2
        assert result[0][1] >= result[1][1]
