"""中枢 v1 — Golden 用例

覆盖：
  1. 三段重叠 → 中枢成立（ZhongshuCandidateV1）
  2. 三段无重叠 → 无中枢
  3. ZG == ZD 精确相切 → 不产生中枢（严格不等）
  4. 第四段仍重叠 → 延伸，seg_count=4, 区间不变
  5. 突破段到来 → ZhongshuSettleV1 + break_direction
  6. 突破方向 up
  7. 突破方向 down
  8. 突破后形成新中枢（续进 break_seg_idx - 2）
  9. 每个 settle 前有 candidate（I12）
  10. 通过 ZhongshuEngine 接口 end-to-end
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot, diff_zhongshu
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.events import (
    ZhongshuCandidateV1,
    ZhongshuSettleV1,
    ZhongshuInvalidateV1,
)


# ── helpers ──

def _seg(idx: int, direction: str, high: float, low: float,
         confirmed: bool = True, s0: int = -1, s1: int = -1) -> Segment:
    """快速构造 Segment。s0/s1 默认用 idx*3, idx*3+2 模拟。"""
    if s0 == -1:
        s0 = idx * 3
    if s1 == -1:
        s1 = idx * 3 + 2
    return Segment(
        s0=s0, s1=s1,
        i0=s0 * 5, i1=s1 * 5,
        direction=direction,
        high=high, low=low,
        confirmed=confirmed,
    )


def _make_seg_snap(segments: list[Segment], bar_idx: int = 100,
                   bar_ts: float = 1000.0) -> ZhongshuSnapshot:
    """构造一个最小 SegmentSnapshot 来驱动 diff。

    注意：这里用 ZhongshuSnapshot 代替 SegmentSnapshot
    只是因为 ZhongshuEngine.process_segment_snapshot 需要的是 SegmentSnapshot。
    对于纯函数测试我们直接调用 zhongshu_from_segments + diff_zhongshu。
    """
    return ZhongshuSnapshot(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        zhongshus=[],
        events=[],
    )


# =====================================================================
# 1) 三段重叠 → 中枢成立
# =====================================================================

class TestThreeSegOverlap:
    """三段价格区间有交集 → 产生一个中枢。"""

    def test_basic_overlap(self):
        """三段 high/low 有交集 → Zhongshu 成立，zg > zd。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        # zd = max(10, 8, 12) = 12, zg = min(20, 18, 22) = 18
        assert zs.zd == 12
        assert zs.zg == 18
        assert zs.seg_start == 0
        assert zs.seg_end == 2
        assert zs.seg_count == 3
        assert not zs.settled

    def test_overlap_events(self):
        """diff 从空到三段中枢 → 产生 ZhongshuCandidateV1。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        zs_list = zhongshu_from_segments(segs)
        events = diff_zhongshu([], zs_list, bar_idx=10, bar_ts=100.0)
        assert len(events) == 1
        ev = events[0]
        assert isinstance(ev, ZhongshuCandidateV1)
        assert ev.zd == 12
        assert ev.zg == 18
        assert ev.zhongshu_id == 0
        assert ev.seg_count == 3


# =====================================================================
# 2) 三段无重叠 → 无中枢
# =====================================================================

class TestThreeSegNoOverlap:
    def test_no_overlap(self):
        """三段无重叠 → 返回空列表。"""
        segs = [
            _seg(0, "up",   10, 5),    # [5, 10]
            _seg(1, "down", 20, 15),   # [15, 20]
            _seg(2, "up",   30, 25),   # [25, 30]
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 0

    def test_no_overlap_events(self):
        """无重叠 → diff 不产生事件。"""
        segs = [
            _seg(0, "up",   10, 5),
            _seg(1, "down", 20, 15),
            _seg(2, "up",   30, 25),
        ]
        zs_list = zhongshu_from_segments(segs)
        events = diff_zhongshu([], zs_list, bar_idx=10, bar_ts=100.0)
        assert len(events) == 0


# =====================================================================
# 3) ZG == ZD 精确相切 → 不产生中枢
# =====================================================================

class TestBoundaryZgEqZd:
    def test_zg_eq_zd_no_zhongshu(self):
        """ZG == ZD → 不满足严格不等，不产生中枢。"""
        segs = [
            _seg(0, "up",   15, 10),   # [10, 15]
            _seg(1, "down", 20, 15),   # [15, 20]
            _seg(2, "up",   25, 20),   # [20, 25]
        ]
        # zd = max(10, 15, 20) = 20, zg = min(15, 20, 25) = 15
        # zg < zd → 无中枢
        result = zhongshu_from_segments(segs)
        assert len(result) == 0

    def test_exact_touch(self):
        """精确相切 ZG == ZD 的另一个构造。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 25, 15),   # [15, 25]
            _seg(2, "up",   30, 20),   # [20, 30]
        ]
        # zd = max(10, 15, 20) = 20, zg = min(20, 25, 30) = 20
        # zg == zd → 严格不等不满足
        result = zhongshu_from_segments(segs)
        assert len(result) == 0


# =====================================================================
# 4) 第四段延伸 → seg_count=4, 区间不变
# =====================================================================

class TestExtensionFourthSeg:
    def test_extension(self):
        """第四段仍与 [ZD, ZG] 重叠 → seg_count=4。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            _seg(3, "down", 19, 11),   # [11, 19] — 与 [12, 18] 重叠
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        assert zs.seg_count == 4
        assert zs.seg_end == 3
        # 区间仍然是初始三段确定的 [12, 18]
        assert zs.zd == 12
        assert zs.zg == 18
        assert not zs.settled

    def test_extension_no_interval_change(self):
        """延伸段不改变 [ZD, ZG]，即使其区间更窄。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            _seg(3, "down", 16, 14),   # [14, 16] — 窄，但在 [12, 18] 内
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        # 区间不被第4段影响
        assert zs.zd == 12
        assert zs.zg == 18
        assert zs.seg_count == 4


# =====================================================================
# 5) 突破段到来 → ZhongshuSettleV1
# =====================================================================

class TestBreakProducesSettle:
    def test_break(self):
        """第四段不与 [ZD, ZG] 重叠 → 中枢闭合。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            _seg(3, "down", 10, 2),    # [2, 10] — low=2 < zd=12, high=10 < zd=12 → 不重叠
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        assert zs.settled
        assert zs.break_seg == 3
        assert zs.seg_count == 3

    def test_break_events(self):
        """从 candidate → settle 的事件序列。"""
        # 第一次：只有 3 段（未闭合）
        segs_phase1 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        zs1 = zhongshu_from_segments(segs_phase1)
        ev1 = diff_zhongshu([], zs1, bar_idx=10, bar_ts=100.0)
        assert len(ev1) == 1
        assert isinstance(ev1[0], ZhongshuCandidateV1)

        # 第二次：第4段突破
        segs_phase2 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 10, 2),
        ]
        zs2 = zhongshu_from_segments(segs_phase2)
        ev2 = diff_zhongshu(zs1, zs2, bar_idx=11, bar_ts=101.0, seq_start=1)
        # 应产生 ZhongshuSettleV1（从 candidate 升级）
        assert len(ev2) == 1
        assert isinstance(ev2[0], ZhongshuSettleV1)
        assert ev2[0].break_seg_id == 3


# =====================================================================
# 6) 突破方向 up
# =====================================================================

class TestBreakDirectionUp:
    def test_break_up(self):
        """突破段 low >= zg → break_direction="up"。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            # zd=12, zg=18; 突破段 low=20 >= zg=18 → "up"
            _seg(3, "down", 30, 20),   # [20, 30]
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        assert result[0].break_direction == "up"


# =====================================================================
# 7) 突破方向 down
# =====================================================================

class TestBreakDirectionDown:
    def test_break_down(self):
        """突破段 high <= zd → break_direction="down"。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20]
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            # zd=12, zg=18; 突破段 high=10 <= zd=12 → "down"
            _seg(3, "down", 10, 2),    # [2, 10]
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        assert result[0].break_direction == "down"


# =====================================================================
# 8) 突破后续进 → 形成新中枢
# =====================================================================

class TestConsecutiveZhongshu:
    def test_two_zhongshu(self):
        """突破后从 break_seg_idx - 2 开始扫描，形成第二个中枢。"""
        segs = [
            _seg(0, "up",   20, 10),   # [10, 20] — zs1
            _seg(1, "down", 18, 8),    # [8, 18]
            _seg(2, "up",   22, 12),   # [12, 22]
            _seg(3, "down", 10, 2),    # 突破 → zs1 闭合
            # 续进从 max(3-2, 2) = 2 开始扫描
            # seg[2]=[12,22], seg[3]=[2,10], seg[4]=[8,15]
            # zd=max(12,2,8)=12, zg=min(22,10,15)=10 → 10<12 无重叠
            # seg[3]=[2,10], seg[4]=[8,15], seg[5]=[6,12]
            # zd=max(2,8,6)=8, zg=min(10,15,12)=10 → 10>8 成立！
            _seg(4, "up",   15, 8),    # [8, 15]
            _seg(5, "down", 12, 6),    # [6, 12]
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2
        # 第一个中枢
        assert result[0].settled
        assert result[0].break_seg == 3
        # 第二个中枢
        assert result[1].seg_start == 3
        assert result[1].zd == 8
        assert result[1].zg == 10
        assert not result[1].settled


# =====================================================================
# 9) settle 前必有 candidate（I12 验证）
# =====================================================================

class TestCandidateBeforeSettle:
    def test_single_diff_settled(self):
        """一次 diff 中即已闭合的中枢：先 candidate 后 settle。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 10, 2),    # 突破
        ]
        zs_list = zhongshu_from_segments(segs)
        events = diff_zhongshu([], zs_list, bar_idx=10, bar_ts=100.0)

        # 首次出现即已闭合 → candidate + settle
        types = [type(e) for e in events]
        # 可能有两个中枢的事件（如果续进产生第二个）
        # 但至少第一个中枢要保证 candidate 在 settle 之前
        candidate_idx = None
        settle_idx = None
        for i, ev in enumerate(events):
            if isinstance(ev, ZhongshuCandidateV1) and ev.zhongshu_id == 0:
                candidate_idx = i
            if isinstance(ev, ZhongshuSettleV1) and ev.zhongshu_id == 0:
                settle_idx = i
        assert candidate_idx is not None, "应有 candidate 事件"
        assert settle_idx is not None, "应有 settle 事件"
        assert candidate_idx < settle_idx, "candidate 必须在 settle 之前"


# =====================================================================
# 10) ZhongshuEngine end-to-end
# =====================================================================

class TestEngineEndToEnd:
    def test_engine_produces_events(self):
        """通过 ZhongshuEngine 接口产生正确事件。"""
        from newchan.core.recursion.segment_state import SegmentSnapshot

        engine = ZhongshuEngine()

        # Phase 1: 三段 → candidate
        segs1 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        seg_snap1 = SegmentSnapshot(
            bar_idx=10, bar_ts=100.0,
            segments=segs1, events=[],
        )
        zs_snap1 = engine.process_segment_snapshot(seg_snap1)
        assert len(zs_snap1.events) == 1
        assert isinstance(zs_snap1.events[0], ZhongshuCandidateV1)
        assert len(zs_snap1.zhongshus) == 1

        # Phase 2: 第四段突破 → settle
        segs2 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 10, 2),
        ]
        seg_snap2 = SegmentSnapshot(
            bar_idx=11, bar_ts=101.0,
            segments=segs2, events=[],
        )
        zs_snap2 = engine.process_segment_snapshot(seg_snap2)
        # settle（+可能的续进 candidate）
        settle_events = [e for e in zs_snap2.events if isinstance(e, ZhongshuSettleV1)]
        assert len(settle_events) >= 1
        assert settle_events[0].break_seg_id == 3

    def test_engine_reset(self):
        """reset 后引擎回到初始状态。"""
        engine = ZhongshuEngine()
        from newchan.core.recursion.segment_state import SegmentSnapshot

        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        seg_snap = SegmentSnapshot(bar_idx=10, bar_ts=100.0, segments=segs, events=[])
        engine.process_segment_snapshot(seg_snap)
        assert engine.event_seq > 0

        engine.reset()
        assert engine.event_seq == 0
        assert engine.current_zhongshus == []

    def test_unconfirmed_segments_ignored(self):
        """未确认段不参与中枢计算。"""
        segs = [
            _seg(0, "up",   20, 10, confirmed=True),
            _seg(1, "down", 18, 8, confirmed=True),
            _seg(2, "up",   22, 12, confirmed=False),  # 未确认
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 0  # 只有 2 段已确认，不足 3 段


# =====================================================================
# 11) first_seg_s0 / last_seg_s1 前端时间定位
# =====================================================================

class TestFrontendLocators:
    def test_s0_s1_correct(self):
        """Zhongshu 的 first_seg_s0 和 last_seg_s1 与 Segment 端点一致。"""
        segs = [
            _seg(0, "up",   20, 10, s0=0, s1=2),
            _seg(1, "down", 18, 8, s0=2, s1=5),
            _seg(2, "up",   22, 12, s0=5, s1=8),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        assert result[0].first_seg_s0 == 0
        assert result[0].last_seg_s1 == 8
