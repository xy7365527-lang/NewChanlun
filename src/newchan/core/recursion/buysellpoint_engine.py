"""事件驱动买卖点引擎 — 消费三层快照，产生 BSP 事件。

BuySellPointEngine 是 v1 管线的最终层引擎。
消费 MoveSnapshot + ZhongshuSnapshot + SegmentSnapshot，
内部计算背驰 → 全量计算买卖点 → diff 产生事件。

五层引擎链：
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → **BuySellPointEngine**
"""

from __future__ import annotations

from newchan.a_buysellpoint_v1 import BuySellPoint, buysellpoints_from_level
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.core.recursion.buysellpoint_state import (
    BuySellPointSnapshot,
    diff_buysellpoints,
)
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot


class BuySellPointEngine:
    """事件驱动买卖点引擎 — 消费三层快照，产生 BSP 事件。

    用法::

        bsp_engine = BuySellPointEngine(level_id=1)
        for bar in bars:
            bi_snap = bi_engine.process_bar(bar)
            seg_snap = seg_engine.process_snapshot(bi_snap)
            zs_snap = zs_engine.process_segment_snapshot(seg_snap)
            move_snap = move_engine.process_zhongshu_snapshot(zs_snap)
            bsp_snap = bsp_engine.process_snapshots(
                move_snap, zs_snap, seg_snap,
            )
            for event in bsp_snap.events:
                handle(event)

    Parameters
    ----------
    level_id : int
        递归层级（透传给 buysellpoints_from_level）。
    stream_id : str
        所属流标识（仅用于日志）。
    """

    def __init__(self, level_id: int = 1, stream_id: str = "") -> None:
        self._prev_bsps: list[BuySellPoint] = []
        self._event_seq: int = 0
        self._level_id = level_id
        self._stream_id = stream_id

    @property
    def current_buysellpoints(self) -> list[BuySellPoint]:
        """当前买卖点列表（浅拷贝）。"""
        return list(self._prev_bsps)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_bsps = []
        self._event_seq = 0

    def process_snapshots(
        self,
        move_snap: MoveSnapshot,
        zs_snap: ZhongshuSnapshot,
        seg_snap: SegmentSnapshot,
    ) -> BuySellPointSnapshot:
        """处理一组上游快照，产生买卖点事件。

        Parameters
        ----------
        move_snap : MoveSnapshot
            走势类型快照（含 moves 列表）。
        zs_snap : ZhongshuSnapshot
            中枢快照（含 zhongshus 列表）。
        seg_snap : SegmentSnapshot
            线段快照（含 segments 列表）。

        Returns
        -------
        BuySellPointSnapshot
            包含当前买卖点列表和本轮产生的 BSP 事件。
        """
        # 1. 计算背驰（无 MACD 时用价格振幅 fallback）
        divergences = divergences_from_moves_v1(
            seg_snap.segments,
            zs_snap.zhongshus,
            move_snap.moves,
            self._level_id,
        )

        # 2. 全量计算买卖点
        curr_bsps = buysellpoints_from_level(
            seg_snap.segments,
            zs_snap.zhongshus,
            move_snap.moves,
            divergences,
            self._level_id,
        )

        # 3. diff 产生事件
        events = diff_buysellpoints(
            self._prev_bsps,
            curr_bsps,
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        # 4. 更新状态
        self._prev_bsps = curr_bsps

        return BuySellPointSnapshot(
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            buysellpoints=curr_bsps,
            events=events,
        )
