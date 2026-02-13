"""中枢 v1 — 确定性验证（I15）

覆盖：
  1. 同输入两次独立运行 → event_id + event_type + seq 完全一致
  2. 逐步喂入 vs 一次性喂入 → 最终中枢列表一致
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import zhongshu_from_segments
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.events import DomainEvent


# ── helpers ──

def _seg(idx: int, direction: str, high: float, low: float,
         confirmed: bool = True) -> Segment:
    s0 = idx * 3
    s1 = idx * 3 + 2
    return Segment(
        s0=s0, s1=s1,
        i0=s0 * 5, i1=s1 * 5,
        direction=direction,
        high=high, low=low,
        confirmed=confirmed,
    )


# 共用段序列：3段成立 + 突破 + 续进3段
SHARED_SEGMENTS = [
    _seg(0, "up",   20, 10),   # zs1: [10, 20]
    _seg(1, "down", 18, 8),    # [8, 18]
    _seg(2, "up",   22, 12),   # [12, 22] → zs1 成立
    _seg(3, "down", 10, 2),    # 突破 zs1 → settled
    _seg(4, "up",   15, 8),    # 续进
    _seg(5, "down", 12, 6),    # zs2 成立
    _seg(6, "up",   16, 9),    # 延伸 zs2
]


def _run_engine_incremental(segments_sequence: list[list[Segment]]) -> tuple[list[DomainEvent], list]:
    """逐步喂入段序列，返回所有事件和最终中枢。"""
    engine = ZhongshuEngine()
    all_events: list[DomainEvent] = []

    for i, segs in enumerate(segments_sequence):
        snap = SegmentSnapshot(
            bar_idx=10 + i,
            bar_ts=100.0 + i,
            segments=segs,
            events=[],
        )
        zs_snap = engine.process_segment_snapshot(snap)
        all_events.extend(zs_snap.events)

    return all_events, engine.current_zhongshus


# =====================================================================
# 1) 同输入两次独立运行 → 完全一致
# =====================================================================

class TestTwoRunsSameEvents:
    def test_identical_events(self):
        """同一段序列两次独立运行，event_id + event_type + seq 完全一致。"""
        # 每次 3 段 → 突破 → 续进
        segment_steps = [
            SHARED_SEGMENTS[:3],   # 3段
            SHARED_SEGMENTS[:4],   # +突破
            SHARED_SEGMENTS[:5],   # +续进1
            SHARED_SEGMENTS[:6],   # +续进2 → zs2
            SHARED_SEGMENTS[:7],   # +延伸
        ]

        events_run1, zs_run1 = _run_engine_incremental(segment_steps)
        events_run2, zs_run2 = _run_engine_incremental(segment_steps)

        # 事件数量一致
        assert len(events_run1) == len(events_run2), (
            f"event count mismatch: {len(events_run1)} vs {len(events_run2)}"
        )

        # 逐一比较
        for i, (e1, e2) in enumerate(zip(events_run1, events_run2)):
            assert e1.event_type == e2.event_type, f"event_type mismatch at index {i}"
            assert e1.seq == e2.seq, f"seq mismatch at index {i}"
            assert e1.event_id == e2.event_id, f"event_id mismatch at index {i}"
            assert e1.bar_idx == e2.bar_idx, f"bar_idx mismatch at index {i}"

        # 最终中枢列表一致
        assert len(zs_run1) == len(zs_run2)
        for z1, z2 in zip(zs_run1, zs_run2):
            assert z1 == z2


# =====================================================================
# 2) 纯函数确定性：同输入 → 同输出
# =====================================================================

class TestPureFunctionDeterminism:
    def test_zhongshu_from_segments_deterministic(self):
        """zhongshu_from_segments 纯函数，同输入多次调用结果一致。"""
        for _ in range(5):
            result = zhongshu_from_segments(SHARED_SEGMENTS)
            assert len(result) == 2
            # 第一个中枢
            assert result[0].zd == 12
            assert result[0].zg == 18
            assert result[0].settled
            # 第二个中枢: seg[3]=[2,10], seg[4]=[8,15], seg[5]=[6,12]
            # zd=max(2,8,6)=8, zg=min(10,15,12)=10
            assert result[1].zd == 8
            assert result[1].zg == 10
            assert not result[1].settled


# =====================================================================
# 3) 最终状态一致性
# =====================================================================

class TestFinalStateConsistency:
    def test_incremental_vs_single_shot(self):
        """逐步喂入 vs 一次性全量 → 最终中枢列表应一致。"""
        # 逐步喂入
        segment_steps = [
            SHARED_SEGMENTS[:3],
            SHARED_SEGMENTS[:4],
            SHARED_SEGMENTS[:5],
            SHARED_SEGMENTS[:6],
            SHARED_SEGMENTS[:7],
        ]
        _, zs_incremental = _run_engine_incremental(segment_steps)

        # 一次性全量
        zs_full = zhongshu_from_segments(SHARED_SEGMENTS)

        assert len(zs_incremental) == len(zs_full)
        for z_inc, z_full in zip(zs_incremental, zs_full):
            assert z_inc.zd == z_full.zd
            assert z_inc.zg == z_full.zg
            assert z_inc.seg_start == z_full.seg_start
            assert z_inc.seg_end == z_full.seg_end
            assert z_inc.settled == z_full.settled
            if z_full.settled:
                assert z_inc.break_seg == z_full.break_seg
                assert z_inc.break_direction == z_full.break_direction
