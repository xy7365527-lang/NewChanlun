"""ZhongshuEngine — 事件驱动中枢引擎

核心流程（Diff-based，与 SegmentEngine 同构）：
1. 接收 SegmentSnapshot（含 segments 快照 + 线段事件）
2. 调用 zhongshu_from_segments(seg_snap.segments) 全量计算中枢
3. diff_zhongshu(prev, curr) 产生中枢事件
4. 为每个事件计算确定性 event_id

架构对齐：
- BiEngine → SegmentEngine → ZhongshuEngine 三层同构
- ZhongshuEngine 只消费已确认段（confirmed=True），不修改 Segment
"""

from __future__ import annotations

from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot, diff_zhongshu
from newchan.events import DomainEvent


class ZhongshuEngine:
    """事件驱动中枢引擎 — 消费 SegmentSnapshot，产生 zhongshu 事件。

    用法::

        zs_engine = ZhongshuEngine()
        for bar in bars:
            bi_snap = bi_engine.process_bar(bar)
            seg_snap = seg_engine.process_snapshot(bi_snap)
            zs_snap = zs_engine.process_segment_snapshot(seg_snap)
            for event in zs_snap.events:
                handle(event)

    Parameters
    ----------
    stream_id : str
        所属流标识（透传到事件中，仅用于日志）。
    """

    def __init__(self, stream_id: str = "") -> None:
        self._prev_zhongshus: list[Zhongshu] = []
        self._event_seq: int = 0
        self._stream_id = stream_id

    @property
    def current_zhongshus(self) -> list[Zhongshu]:
        """当前中枢列表（浅拷贝）。"""
        return list(self._prev_zhongshus)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_zhongshus = []
        self._event_seq = 0

    def process_segment_snapshot(self, seg_snap: SegmentSnapshot) -> ZhongshuSnapshot:
        """处理一个 SegmentSnapshot，产生 zhongshu 事件。

        Parameters
        ----------
        seg_snap : SegmentSnapshot
            包含当前线段列表和线段事件的快照。

        Returns
        -------
        ZhongshuSnapshot
            包含当前中枢列表和本轮产生的中枢事件。
        """
        # 1. 全量计算中枢
        curr_zhongshus = zhongshu_from_segments(seg_snap.segments)

        # 2. diff 产生事件
        events = diff_zhongshu(
            self._prev_zhongshus,
            curr_zhongshus,
            bar_idx=seg_snap.bar_idx,
            bar_ts=seg_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        # 3. 更新状态
        self._prev_zhongshus = curr_zhongshus

        return ZhongshuSnapshot(
            bar_idx=seg_snap.bar_idx,
            bar_ts=seg_snap.bar_ts,
            zhongshus=curr_zhongshus,
            events=events,
        )
