"""a_level_fsm_adapter 单元测试

覆盖：
1. zhongshu_to_center — 字段映射正确性（settled/candidate）
2. level_zhongshu_to_center — 字段映射正确性（settled/candidate）
3. level_views_from_recursive_snapshot — 各级别 LevelView 构建
4. select_lstar_from_recursive_snapshot — 便捷函数端到端

概念溯源: [新缠论] — FSM 适配器桥接新递归链与旧 v0 类型
"""

from __future__ import annotations

import pytest

from newchan.a_center_v0 import Center
from newchan.a_level_fsm_adapter import (
    level_views_from_recursive_snapshot,
    level_zhongshu_to_center,
    select_lstar_from_recursive_snapshot,
    zhongshu_to_center,
)
from newchan.a_level_fsm_newchan import LevelView
from newchan.a_move_v1 import Move
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.bi_engine import BiEngineSnapshot
from newchan.core.recursion.buysellpoint_state import BuySellPointSnapshot
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot


# ====================================================================
# 测试数据工厂
# ====================================================================


def _zhongshu(
    *,
    zd: float = 10.0,
    zg: float = 20.0,
    seg_start: int = 0,
    seg_end: int = 2,
    seg_count: int = 3,
    settled: bool = True,
    break_seg: int = 3,
    break_direction: str = "up",
    gg: float = 25.0,
    dd: float = 5.0,
) -> Zhongshu:
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_count,
        settled=settled,
        break_seg=break_seg,
        break_direction=break_direction,
        first_seg_s0=0,
        last_seg_s1=10,
        gg=gg,
        dd=dd,
    )


def _level_zhongshu(
    *,
    zd: float = 100.0,
    zg: float = 200.0,
    comp_start: int = 0,
    comp_end: int = 2,
    comp_count: int = 3,
    settled: bool = True,
    break_comp: int = 3,
    break_direction: str = "down",
    gg: float = 250.0,
    dd: float = 50.0,
    level_id: int = 2,
) -> LevelZhongshu:
    return LevelZhongshu(
        zd=zd,
        zg=zg,
        comp_start=comp_start,
        comp_end=comp_end,
        comp_count=comp_count,
        settled=settled,
        break_comp=break_comp,
        break_direction=break_direction,
        gg=gg,
        dd=dd,
        level_id=level_id,
    )


def _seg(
    *,
    idx: int = 0,
    direction: str = "up",
    high: float = 20.0,
    low: float = 10.0,
) -> Segment:
    return Segment(
        s0=idx * 3,
        s1=idx * 3 + 2,
        i0=idx * 6,
        i1=idx * 6 + 5,
        direction=direction,
        high=high,
        low=low,
        confirmed=True,
    )


def _move(
    *,
    kind: str = "consolidation",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
    settled: bool = True,
    high: float = 20.0,
    low: float = 10.0,
) -> Move:
    return Move(
        kind=kind,
        direction=direction,
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=0,
        zs_end=0,
        zs_count=1,
        settled=settled,
        high=high,
        low=low,
    )


def _empty_snap(
    *,
    segments: list[Segment] | None = None,
    zhongshus: list[Zhongshu] | None = None,
    moves: list[Move] | None = None,
    recursive_snapshots: list[RecursiveLevelSnapshot] | None = None,
) -> RecursiveOrchestratorSnapshot:
    """构建最小 RecursiveOrchestratorSnapshot。"""
    return RecursiveOrchestratorSnapshot(
        bar_idx=0,
        bar_ts=0.0,
        bi_snapshot=BiEngineSnapshot(
            bar_idx=0, bar_ts=0.0, strokes=[], events=[],
            n_merged=0, n_fractals=0,
        ),
        seg_snapshot=SegmentSnapshot(
            bar_idx=0, bar_ts=0.0, segments=segments or [], events=[],
        ),
        zs_snapshot=ZhongshuSnapshot(
            bar_idx=0, bar_ts=0.0, zhongshus=zhongshus or [], events=[],
        ),
        move_snapshot=MoveSnapshot(
            bar_idx=0, bar_ts=0.0, moves=moves or [], events=[],
        ),
        bsp_snapshot=BuySellPointSnapshot(
            bar_idx=0, bar_ts=0.0, buysellpoints=[], events=[],
        ),
        recursive_snapshots=recursive_snapshots or [],
    )


# ====================================================================
# A) zhongshu_to_center
# ====================================================================


class TestZhongshuToCenter:
    """Zhongshu → Center 字段映射。"""

    def test_settled_mapping(self) -> None:
        zs = _zhongshu(
            zd=10.0, zg=20.0, seg_start=1, seg_end=4,
            seg_count=4, settled=True, break_direction="up",
            gg=25.0, dd=5.0,
        )
        c = zhongshu_to_center(zs)

        assert isinstance(c, Center)
        assert c.seg0 == 1
        assert c.seg1 == 4
        assert c.low == 10.0
        assert c.high == 20.0
        assert c.kind == "settled"
        assert c.confirmed is True
        assert c.sustain == 1  # max(0, 4-3) = 1
        assert c.direction == "up"
        assert c.gg == 25.0
        assert c.dd == 5.0

    def test_candidate_mapping(self) -> None:
        zs = _zhongshu(settled=False, seg_count=3)
        c = zhongshu_to_center(zs)

        assert c.kind == "candidate"
        assert c.sustain == 0  # max(0, 3-3)

    def test_sustain_zero_for_minimal(self) -> None:
        zs = _zhongshu(seg_count=3, settled=True)
        c = zhongshu_to_center(zs)
        assert c.sustain == 0

    def test_sustain_positive_for_extended(self) -> None:
        zs = _zhongshu(seg_count=7, settled=True)
        c = zhongshu_to_center(zs)
        assert c.sustain == 4


# ====================================================================
# B) level_zhongshu_to_center
# ====================================================================


class TestLevelZhongshuToCenter:
    """LevelZhongshu → Center 字段映射。"""

    def test_settled_mapping(self) -> None:
        lzs = _level_zhongshu(
            zd=100.0, zg=200.0, comp_start=2, comp_end=5,
            comp_count=4, settled=True, break_direction="down",
            gg=250.0, dd=50.0,
        )
        c = level_zhongshu_to_center(lzs)

        assert isinstance(c, Center)
        assert c.seg0 == 2  # comp_start
        assert c.seg1 == 5  # comp_end
        assert c.low == 100.0
        assert c.high == 200.0
        assert c.kind == "settled"
        assert c.confirmed is True
        assert c.sustain == 1  # max(0, 4-3)
        assert c.direction == "down"
        assert c.gg == 250.0
        assert c.dd == 50.0

    def test_candidate_mapping(self) -> None:
        lzs = _level_zhongshu(settled=False, comp_count=3)
        c = level_zhongshu_to_center(lzs)

        assert c.kind == "candidate"
        assert c.sustain == 0


# ====================================================================
# C) level_views_from_recursive_snapshot
# ====================================================================


class TestLevelViewsFromRecursiveSnapshot:
    """LevelView 列表构建。"""

    def test_empty_no_zhongshus(self) -> None:
        """无中枢 → 空 views。"""
        snap = _empty_snap()
        views = level_views_from_recursive_snapshot(snap)
        assert views == []

    def test_level1_only(self) -> None:
        """仅 Level 1：有中枢、无递归层。"""
        segs = [
            _seg(idx=0, direction="up", high=20, low=10),
            _seg(idx=1, direction="down", high=18, low=8),
            _seg(idx=2, direction="up", high=22, low=12),
            _seg(idx=3, direction="down", high=15, low=5),
        ]
        zs = _zhongshu(
            zd=12.0, zg=18.0, seg_start=0, seg_end=2,
            seg_count=3, settled=True, break_direction="down",
        )
        snap = _empty_snap(segments=segs, zhongshus=[zs])
        views = level_views_from_recursive_snapshot(snap)

        assert len(views) == 1
        v = views[0]
        assert v.level == 1
        assert len(v.segments) == 4
        assert len(v.centers) == 1
        assert v.centers[0].seg0 == 0
        assert v.centers[0].seg1 == 2

    def test_level1_plus_recursive(self) -> None:
        """Level 1 + Level 2 递归层。"""
        segs = [_seg(idx=i) for i in range(5)]
        zs1 = _zhongshu(seg_start=0, seg_end=2, settled=True)
        moves = [
            _move(seg_start=0, seg_end=2, settled=True, high=22, low=8),
            _move(seg_start=3, seg_end=5, settled=False, high=25, low=10),
        ]
        lzs2 = _level_zhongshu(
            comp_start=0, comp_end=1, comp_count=3,
            settled=True, level_id=2,
        )
        rs = RecursiveLevelSnapshot(
            bar_idx=0, bar_ts=0.0, level_id=2,
            zhongshus=[lzs2],
            moves=[_move(seg_start=0, settled=False)],
            zhongshu_events=[], move_events=[],
        )
        snap = _empty_snap(
            segments=segs, zhongshus=[zs1],
            moves=moves, recursive_snapshots=[rs],
        )
        views = level_views_from_recursive_snapshot(snap)

        assert len(views) == 2
        # Level 1
        assert views[0].level == 1
        assert len(views[0].segments) == 5
        assert len(views[0].centers) == 1
        # Level 2
        assert views[1].level == 2
        assert len(views[1].segments) == 2  # prev_moves
        assert len(views[1].centers) == 1

    def test_recursive_no_l1_centers(self) -> None:
        """Level 1 无中枢但递归层有中枢 → 只有 Level 2。"""
        moves = [_move(settled=True), _move(settled=False)]
        lzs = _level_zhongshu(comp_start=0, comp_end=1)
        rs = RecursiveLevelSnapshot(
            bar_idx=0, bar_ts=0.0, level_id=2,
            zhongshus=[lzs], moves=[],
            zhongshu_events=[], move_events=[],
        )
        snap = _empty_snap(moves=moves, recursive_snapshots=[rs])
        views = level_views_from_recursive_snapshot(snap)

        assert len(views) == 1
        assert views[0].level == 2
        assert len(views[0].segments) == 2

    def test_recursive_empty_centers_skipped(self) -> None:
        """递归层无中枢 → 该层被跳过。"""
        segs = [_seg(idx=i) for i in range(4)]
        zs1 = _zhongshu(settled=True)
        moves = [_move()]
        rs = RecursiveLevelSnapshot(
            bar_idx=0, bar_ts=0.0, level_id=2,
            zhongshus=[], moves=[],
            zhongshu_events=[], move_events=[],
        )
        snap = _empty_snap(
            segments=segs, zhongshus=[zs1],
            moves=moves, recursive_snapshots=[rs],
        )
        views = level_views_from_recursive_snapshot(snap)

        assert len(views) == 1
        assert views[0].level == 1

    def test_multi_level_recursive(self) -> None:
        """Level 1 + Level 2 + Level 3 多层递归。"""
        segs = [_seg(idx=i) for i in range(6)]
        zs1 = _zhongshu(settled=True)
        l1_moves = [_move(seg_start=0, settled=True), _move(seg_start=3, settled=True)]

        lzs2 = _level_zhongshu(comp_start=0, comp_end=1, level_id=2)
        l2_moves = [_move(seg_start=0, settled=True)]

        lzs3 = _level_zhongshu(comp_start=0, comp_end=0, level_id=3)

        rs2 = RecursiveLevelSnapshot(
            bar_idx=0, bar_ts=0.0, level_id=2,
            zhongshus=[lzs2], moves=l2_moves,
            zhongshu_events=[], move_events=[],
        )
        rs3 = RecursiveLevelSnapshot(
            bar_idx=0, bar_ts=0.0, level_id=3,
            zhongshus=[lzs3], moves=[],
            zhongshu_events=[], move_events=[],
        )
        snap = _empty_snap(
            segments=segs, zhongshus=[zs1],
            moves=l1_moves, recursive_snapshots=[rs2, rs3],
        )
        views = level_views_from_recursive_snapshot(snap)

        assert len(views) == 3
        assert views[0].level == 1
        assert views[1].level == 2
        assert views[2].level == 3
        # Level 3 的 segments = Level 2 的 moves
        assert len(views[2].segments) == 1


# ====================================================================
# D) select_lstar_from_recursive_snapshot
# ====================================================================


class TestSelectLstarFromRecursiveSnapshot:
    """便捷函数端到端。"""

    def test_empty_returns_none(self) -> None:
        snap = _empty_snap()
        result = select_lstar_from_recursive_snapshot(snap, last_price=15.0)
        assert result is None

    def test_with_alive_center(self) -> None:
        """有存活中枢时应返回 LStar。"""
        # 构造：4 段 + 1 个 settled 中枢 + last_price 在中枢区间内
        segs = [
            _seg(idx=0, direction="up", high=20, low=10),
            _seg(idx=1, direction="down", high=18, low=8),
            _seg(idx=2, direction="up", high=22, low=12),
            _seg(idx=3, direction="down", high=19, low=9),
        ]
        zs = _zhongshu(
            zd=12.0, zg=18.0, seg_start=0, seg_end=2,
            seg_count=3, settled=True, break_direction="down",
            gg=22.0, dd=8.0,
        )
        snap = _empty_snap(segments=segs, zhongshus=[zs])
        result = select_lstar_from_recursive_snapshot(snap, last_price=15.0)

        assert result is not None
        assert result.level == 1
        assert result.center_idx == 0

    def test_dead_center_returns_none(self) -> None:
        """中枢未 settled → 死亡 → 无存活 → None。"""
        segs = [_seg(idx=i) for i in range(4)]
        zs = _zhongshu(settled=False)
        snap = _empty_snap(segments=segs, zhongshus=[zs])
        result = select_lstar_from_recursive_snapshot(snap, last_price=15.0)

        # FSM 会将 candidate 判为 DEAD_NOT_SETTLED
        assert result is None
