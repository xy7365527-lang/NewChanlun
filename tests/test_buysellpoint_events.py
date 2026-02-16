"""买卖点事件层测试 — diff_buysellpoints + BuySellPointEngine。

测试结构同 test_bi_differ.py：
- _mk_bsp() 快速构造 BuySellPoint
- _diff() 包装 diff_buysellpoints
- TestDiffBuySellPoints: diff 算法 10 个场景
- TestBuySellPointEngine: 引擎 4 个场景
"""

from __future__ import annotations

from typing import Literal

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.recursion.buysellpoint_state import (
    BuySellPointSnapshot,
    diff_buysellpoints,
)
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
    BuySellPointSettleV1,
)

_BAR_IDX = 100
_BAR_TS = 1700000000.0


def _mk_bsp(
    seg_idx: int = 0,
    kind: Literal["type1", "type2", "type3"] = "type1",
    side: Literal["buy", "sell"] = "buy",
    level_id: int = 1,
    price: float = 50.0,
    confirmed: bool = False,
    settled: bool = False,
    overlaps_with: Literal["type2", "type3"] | None = None,
    move_seg_start: int = 0,
    center_seg_start: int = 0,
    bar_idx: int = 10,
) -> BuySellPoint:
    """快速构造一个 BuySellPoint（最小必填字段）。"""
    return BuySellPoint(
        kind=kind,
        side=side,
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=move_seg_start,
        divergence_key=None,
        center_zd=40.0,
        center_zg=60.0,
        center_seg_start=center_seg_start,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=settled,
        overlaps_with=overlaps_with,
    )


def _diff(
    prev: list[BuySellPoint],
    curr: list[BuySellPoint],
    seq_start: int = 0,
) -> list:
    return diff_buysellpoints(
        prev, curr, bar_idx=_BAR_IDX, bar_ts=_BAR_TS, seq_start=seq_start
    )


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# diff_buysellpoints 测试
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestDiffBuySellPoints:
    """测试 diff_buysellpoints 的各种场景。"""

    def test_empty_to_empty(self):
        """空→空：无事件。"""
        events = _diff([], [])
        assert events == []

    def test_empty_to_one_candidate(self):
        """空→一个未确认 BSP：产生 Candidate。"""
        bp = _mk_bsp(seg_idx=3, kind="type1", side="buy", price=45.0)
        events = _diff([], [bp])
        assert len(events) == 1
        e = events[0]
        assert isinstance(e, BuySellPointCandidateV1)
        assert e.event_type == "bsp_candidate"
        assert e.bsp_id == 0
        assert e.kind == "type1"
        assert e.side == "buy"
        assert e.price == 45.0
        assert e.bar_idx == _BAR_IDX
        assert e.bar_ts == _BAR_TS

    def test_new_bsp_already_confirmed_emits_candidate_plus_confirm(self):
        """空→已确认 BSP：产生 Candidate + Confirm（I24 保证）。"""
        bp = _mk_bsp(seg_idx=5, confirmed=True, price=42.0)
        events = _diff([], [bp])
        assert len(events) == 2
        assert isinstance(events[0], BuySellPointCandidateV1)
        assert isinstance(events[1], BuySellPointConfirmV1)
        assert events[0].bsp_id == events[1].bsp_id == 0
        assert events[0].seq == 0
        assert events[1].seq == 1

    def test_same_identity_confirm_upgrade(self):
        """同身份 confirmed=F→T：产生 Confirm。"""
        prev = [_mk_bsp(seg_idx=3, confirmed=False)]
        curr = [_mk_bsp(seg_idx=3, confirmed=True)]
        events = _diff(prev, curr)
        assert len(events) == 1
        assert isinstance(events[0], BuySellPointConfirmV1)
        assert events[0].bsp_id == 0

    def test_same_identity_settle_upgrade(self):
        """同身份 settled=F→T：产生 Settle。"""
        prev = [_mk_bsp(seg_idx=3, confirmed=True, settled=False)]
        curr = [_mk_bsp(seg_idx=3, confirmed=True, settled=True)]
        events = _diff(prev, curr)
        assert len(events) == 1
        assert isinstance(events[0], BuySellPointSettleV1)
        assert events[0].bsp_id == 0

    def test_same_identity_price_change(self):
        """同身份 price 变化：产生 Candidate（更新）。"""
        prev = [_mk_bsp(seg_idx=3, price=50.0)]
        curr = [_mk_bsp(seg_idx=3, price=48.0)]
        events = _diff(prev, curr)
        assert len(events) == 1
        assert isinstance(events[0], BuySellPointCandidateV1)
        assert events[0].price == 48.0

    def test_same_identity_overlaps_with_change(self):
        """同身份 overlaps_with 变化：产生 Candidate（更新）。"""
        prev = [_mk_bsp(seg_idx=3, kind="type2")]
        curr = [_mk_bsp(seg_idx=3, kind="type2", overlaps_with="type3")]
        events = _diff(prev, curr)
        assert len(events) == 1
        assert isinstance(events[0], BuySellPointCandidateV1)
        assert events[0].overlaps_with == "type3"

    def test_different_identity_invalidate_and_new(self):
        """不同身份替换：invalidate 旧 + candidate 新。"""
        prev = [_mk_bsp(seg_idx=3, kind="type1")]
        curr = [_mk_bsp(seg_idx=5, kind="type2")]
        events = _diff(prev, curr)
        assert len(events) == 2
        assert isinstance(events[0], BuySellPointInvalidateV1)
        assert events[0].seg_idx == 3
        assert isinstance(events[1], BuySellPointCandidateV1)
        assert events[1].seg_idx == 5

    def test_common_prefix_preserved(self):
        """公共前缀不产生事件。"""
        shared = _mk_bsp(seg_idx=1, price=50.0, confirmed=True, settled=True)
        prev = [shared]
        curr = [shared, _mk_bsp(seg_idx=5)]
        events = _diff(prev, curr)
        assert len(events) == 1
        assert isinstance(events[0], BuySellPointCandidateV1)
        assert events[0].bsp_id == 1

    def test_seq_monotonic(self):
        """seq 全局单调递增。"""
        bp1 = _mk_bsp(seg_idx=3, confirmed=True)
        bp2 = _mk_bsp(seg_idx=7, kind="type3")
        events = _diff([], [bp1, bp2])
        # bp1: Candidate(0) + Confirm(1); bp2: Candidate(2)
        assert len(events) == 3
        seqs = [e.seq for e in events]
        assert seqs == [0, 1, 2]

    def test_seq_start_respected(self):
        """seq_start 偏移正确传递。"""
        bp = _mk_bsp(seg_idx=3)
        events = _diff([], [bp], seq_start=10)
        assert events[0].seq == 10

    def test_invalidate_before_candidate(self):
        """invalidate 事件排在 candidate 之前（因果序）。"""
        prev = [_mk_bsp(seg_idx=3, kind="type1")]
        curr = [_mk_bsp(seg_idx=5, kind="type3", side="sell")]
        events = _diff(prev, curr)
        types = [type(e).__name__ for e in events]
        inv_idx = types.index("BuySellPointInvalidateV1")
        cand_idx = types.index("BuySellPointCandidateV1")
        assert inv_idx < cand_idx

    def test_event_ids_unique(self):
        """同批次事件的 event_id 不重复。"""
        bp1 = _mk_bsp(seg_idx=3, confirmed=True)
        bp2 = _mk_bsp(seg_idx=7)
        events = _diff([], [bp1, bp2])
        ids = [e.event_id for e in events]
        assert len(set(ids)) == len(ids)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# BuySellPointEngine 测试
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

from newchan.core.recursion.buysellpoint_engine import BuySellPointEngine
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot


def _empty_seg_snap(bar_idx: int = 0) -> SegmentSnapshot:
    return SegmentSnapshot(bar_idx=bar_idx, bar_ts=0.0, segments=[], events=[])


def _empty_zs_snap(bar_idx: int = 0) -> ZhongshuSnapshot:
    return ZhongshuSnapshot(bar_idx=bar_idx, bar_ts=0.0, zhongshus=[], events=[])


def _empty_move_snap(bar_idx: int = 0) -> MoveSnapshot:
    return MoveSnapshot(bar_idx=bar_idx, bar_ts=0.0, moves=[], events=[])


class TestBuySellPointEngine:
    """测试 BuySellPointEngine 基本行为。"""

    def test_initial_state(self):
        """初始状态：无买卖点，seq=0。"""
        eng = BuySellPointEngine(level_id=1)
        assert eng.current_buysellpoints == []
        assert eng.event_seq == 0

    def test_empty_snapshots_no_events(self):
        """空快照输入：无事件产生。"""
        eng = BuySellPointEngine()
        snap = eng.process_snapshots(
            _empty_move_snap(), _empty_zs_snap(), _empty_seg_snap()
        )
        assert isinstance(snap, BuySellPointSnapshot)
        assert snap.buysellpoints == []
        assert snap.events == []
        assert eng.event_seq == 0

    def test_reset_clears_state(self):
        """reset 后回到初始状态。"""
        eng = BuySellPointEngine()
        # 先处理一次空快照确保不出错
        eng.process_snapshots(
            _empty_move_snap(), _empty_zs_snap(), _empty_seg_snap()
        )
        eng.reset()
        assert eng.current_buysellpoints == []
        assert eng.event_seq == 0

    def test_snapshot_returns_correct_type(self):
        """返回 BuySellPointSnapshot 类型。"""
        eng = BuySellPointEngine()
        snap = eng.process_snapshots(
            _empty_move_snap(), _empty_zs_snap(), _empty_seg_snap()
        )
        assert isinstance(snap, BuySellPointSnapshot)
        assert hasattr(snap, "bar_idx")
        assert hasattr(snap, "bar_ts")
        assert hasattr(snap, "buysellpoints")
        assert hasattr(snap, "events")
