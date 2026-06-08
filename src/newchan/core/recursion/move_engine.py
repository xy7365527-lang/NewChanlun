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
from newchan.ph_layer import attach_persistence


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
        self._last_zs_key: tuple = ()

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
        self._last_zs_key = ()

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
        zss = zs_snap.zhongshus
        n_zs = len(zss)
        # O(1) settled 推导（替代 O(N_zs)/bar 的 sum + next-reversed 扫描）。
        # 结构不变量：_scan_zhongshu 的控制流（append→settled 则 continue 否则
        # break）保证 zss[:-1] 全部 settled，仅 zss[-1] 可能未 settled。
        # 故 n_settled / last_settled 可由尾元素 O(1) 推导，与全列表扫描逐位等价。
        # 认识论等级：L0（结构恒等式，零信息增量）。回归测试为正确性闸门。
        if n_zs == 0:
            zs_key = (0, 0, num_segments)
        else:
            last = zss[-1]
            if last.settled:
                n_settled = n_zs
                last_settled = last
            else:
                n_settled = n_zs - 1
                last_settled = zss[-2] if n_zs >= 2 else None
            if n_settled >= 1:
                zs_key = (n_zs, n_settled, last_settled.seg_end, num_segments)
            else:
                zs_key = (n_zs, 0, num_segments)

        if zs_key == self._last_zs_key:
            # 返回引用（见 SegmentEngine 同款优化）：消除安静 bar 的 O(N_moves) 拷贝。
            return MoveSnapshot(
                bar_idx=zs_snap.bar_idx,
                bar_ts=zs_snap.bar_ts,
                moves=self._prev_moves,
                events=[],
            )
        self._last_zs_key = zs_key

        curr_moves = moves_from_zhongshus(zss, num_segments=num_segments)
        curr_moves = attach_persistence(curr_moves, zss)

        events = diff_moves(
            self._prev_moves,
            curr_moves,
            bar_idx=zs_snap.bar_idx,
            bar_ts=zs_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        self._prev_moves = curr_moves

        return MoveSnapshot(
            bar_idx=zs_snap.bar_idx,
            bar_ts=zs_snap.bar_ts,
            moves=curr_moves,
            events=events,
        )
