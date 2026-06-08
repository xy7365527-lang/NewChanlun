"""事件驱动买卖点引擎 — 消费三层快照，产生 BSP 事件。

BuySellPointEngine 是 v1 管线的最终层引擎。
消费 MoveSnapshot + ZhongshuSnapshot + SegmentSnapshot，
内部计算背驰 → 全量计算买卖点 → diff 产生事件。

五层引擎链：
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → **BuySellPointEngine**
"""

from __future__ import annotations

import pandas as pd

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
        self._last_input_key: tuple = ()

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
        self._last_input_key = ()

    def process_snapshots(
        self,
        move_snap: MoveSnapshot,
        zs_snap: ZhongshuSnapshot,
        seg_snap: SegmentSnapshot,
        *,
        df_macd: pd.DataFrame | None = None,
        merged_to_raw: list[tuple[int, int]] | None = None,
    ) -> BuySellPointSnapshot:
        """处理一组上游快照，产生买卖点事件。

        df_macd / merged_to_raw：传入时趋势背驰用 MACD 三维度（T2/T6/T7）判定，
        否则退化为价格振幅 fallback（见 a_divergence_v1._compute_force）。两者
        都默认 None 以保持原有调用方行为不变（engine_vs_tv_comparison.md §7.4）。
        """
        segs = seg_snap.segments
        n_seg = len(segs)
        n_zs = len(zs_snap.zhongshus)
        n_mv = len(move_snap.moves)
        if n_seg >= 2:
            s = segs[-2]
            input_key = (n_seg, s.s0, s.s1, n_zs, n_mv)
        else:
            input_key = (n_seg, n_zs, n_mv)

        if input_key == self._last_input_key:
            return BuySellPointSnapshot(
                bar_idx=move_snap.bar_idx,
                bar_ts=move_snap.bar_ts,
                buysellpoints=list(self._prev_bsps),
                events=[],
            )
        self._last_input_key = input_key

        curr_bsps = self._compute_buysellpoints(
            move_snap, zs_snap, seg_snap, df_macd, merged_to_raw,
        )
        events = self._diff_and_advance(curr_bsps, move_snap)
        self._prev_bsps = curr_bsps

        return BuySellPointSnapshot(
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            buysellpoints=curr_bsps,
            events=events,
        )

    def _compute_buysellpoints(
        self,
        move_snap: MoveSnapshot,
        zs_snap: ZhongshuSnapshot,
        seg_snap: SegmentSnapshot,
        df_macd: pd.DataFrame | None = None,
        merged_to_raw: list[tuple[int, int]] | None = None,
    ) -> list[BuySellPoint]:
        """计算背驰 + 全量买卖点。"""
        divergences = divergences_from_moves_v1(
            seg_snap.segments,
            zs_snap.zhongshus,
            move_snap.moves,
            self._level_id,
            df_macd=df_macd,
            merged_to_raw=merged_to_raw,
        )
        return buysellpoints_from_level(
            seg_snap.segments,
            zs_snap.zhongshus,
            move_snap.moves,
            divergences,
            self._level_id,
        )

    def _diff_and_advance(
        self, curr_bsps: list[BuySellPoint], move_snap: MoveSnapshot,
    ) -> list:
        """diff 产生事件并推进 seq。"""
        events = diff_buysellpoints(
            self._prev_bsps,
            curr_bsps,
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)
        return events
