"""MoveEngine — 事件驱动走势类型引擎

核心流程（Diff-based，与 ZhongshuEngine 同构）：
1. 接收 ZhongshuSnapshot（含 zhongshus 快照 + 中枢事件）
2. 调用 moves_from_zhongshus(zs_snap.zhongshus) 全量计算 Move
3. diff_moves(prev, curr) 产生走势类型事件
4. 为每个事件计算确定性 event_id

架构对齐：
- BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine 四层同构
- MoveEngine 只消费已闭合中枢（settled=True），不修改 Zhongshu
"""

from __future__ import annotations

from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.core.recursion.move_state import MoveSnapshot, diff_moves
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot
from newchan.events import DomainEvent


class MoveEngine:
    """事件驱动走势类型引擎 — 消费 ZhongshuSnapshot，产生 move 事件。

    用法::

        move_engine = MoveEngine()
        for bar in bars:
            bi_snap = bi_engine.process_bar(bar)
            seg_snap = seg_engine.process_snapshot(bi_snap)
            zs_snap = zs_engine.process_segment_snapshot(seg_snap)
            move_snap = move_engine.process_zhongshu_snapshot(zs_snap)
            for event in move_snap.events:
                handle(event)

    Parameters
    ----------
    stream_id : str
        所属流标识（透传到事件中，仅用于日志）。
    """

    def __init__(self, stream_id: str = "") -> None:
        self._prev_moves: list[Move] = []
        self._event_seq: int = 0
        self._stream_id = stream_id

    @property
    def current_moves(self) -> list[Move]:
        """当前 Move 列表（浅拷贝）。"""
        return list(self._prev_moves)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_moves = []
        self._event_seq = 0

    def process_zhongshu_snapshot(
        self,
        zs_snap: ZhongshuSnapshot,
        num_segments: int | None = None,
    ) -> MoveSnapshot:
        """处理一个 ZhongshuSnapshot，产生 move 事件。

        Parameters
        ----------
        zs_snap : ZhongshuSnapshot
            包含当前中枢列表和中枢事件的快照。
        num_segments : int | None
            当前线段总数。提供时，末组 Move 的 seg_end 扩展覆盖 C段。

        Returns
        -------
        MoveSnapshot
            包含当前 Move 列表和本轮产生的 move 事件。
        """
        # 1. 全量计算 Move（传递 num_segments 扩展 C段覆盖）
        curr_moves = moves_from_zhongshus(zs_snap.zhongshus, num_segments=num_segments)

        # 2. diff 产生事件
        events = diff_moves(
            self._prev_moves,
            curr_moves,
            bar_idx=zs_snap.bar_idx,
            bar_ts=zs_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        # 3. 更新状态
        self._prev_moves = curr_moves

        return MoveSnapshot(
            bar_idx=zs_snap.bar_idx,
            bar_ts=zs_snap.bar_ts,
            moves=curr_moves,
            events=events,
        )
