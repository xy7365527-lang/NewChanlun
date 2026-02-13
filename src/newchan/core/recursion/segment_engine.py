"""SegmentEngine — 事件驱动线段引擎

核心流程（Diff-based）：
1. 接收 BiEngineSnapshot（含 strokes 快照 + 笔事件）
2. 调用 segments_from_strokes_v1(snap.strokes) 全量计算线段
3. diff_segments(prev, curr) 产生线段事件
4. 为每个事件计算确定性 event_id

架构对齐：
- 与 BiEngine 同构——全量纯函数 + 差分产生事件
- 只要有笔变化（stroke events 非空），就触发重算
- SegmentEngine 不修改 BiEngine 的输出
"""

from __future__ import annotations

from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.bi_engine import BiEngineSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot, diff_segments
from newchan.events import DomainEvent


class SegmentEngine:
    """事件驱动线段引擎 — 消费 BiEngineSnapshot，产生 segment 事件。

    用法::

        seg_engine = SegmentEngine()
        for bar in bars:
            bi_snap = bi_engine.process_bar(bar)
            seg_snap = seg_engine.process_snapshot(bi_snap)
            for event in seg_snap.events:
                handle(event)

    Parameters
    ----------
    stream_id : str
        所属流标识（透传到事件中，仅用于日志）。
    """

    def __init__(self, stream_id: str = "") -> None:
        self._prev_segments: list[Segment] = []
        self._event_seq: int = 0
        self._stream_id = stream_id

    @property
    def current_segments(self) -> list[Segment]:
        """当前线段列表（浅拷贝）。"""
        return list(self._prev_segments)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_segments.clear()
        self._event_seq = 0

    def process_snapshot(self, snap: BiEngineSnapshot) -> SegmentSnapshot:
        """处理一个 BiEngine 快照，产生 segment 事件。

        只要 snap.strokes 中有笔（无论本轮是否有笔事件），
        都会重算线段并 diff。这保证 SegmentEngine 的状态
        始终与 BiEngine 同步。

        Parameters
        ----------
        snap : BiEngineSnapshot
            包含当前笔列表和笔事件的快照。

        Returns
        -------
        SegmentSnapshot
            包含当前线段列表和本轮产生的线段事件。
        """
        # 1. 全量计算线段
        curr_segments = segments_from_strokes_v1(snap.strokes)

        # 2. diff 产生事件
        events = diff_segments(
            self._prev_segments,
            curr_segments,
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        # 3. 更新状态
        self._prev_segments = curr_segments

        return SegmentSnapshot(
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            segments=curr_segments,
            events=events,
        )
