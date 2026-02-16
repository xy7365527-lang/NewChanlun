"""级别递归引擎 — 消费 MoveSnapshot，产生递归级别中枢和走势。

RecursiveLevelEngine 是递归层级的核心引擎。它接收来自下级引擎的
MoveSnapshot（走势类型快照），从中过滤出 settled 走势类型，将其
适配为 MoveAsComponent，然后执行中枢检测和走势分组。

概念溯源: [旧缠论] — 级别递归构造
"""

from __future__ import annotations

from newchan.a_level_protocol import adapt_moves
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_level import (
    LevelZhongshu,
    moves_from_level_zhongshus,
    zhongshu_from_components,
)
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_state import (
    RecursiveLevelSnapshot,
    diff_level_moves,
    diff_level_zhongshu,
)


class RecursiveLevelEngine:
    """事件驱动级别递归引擎 — 消费 MoveSnapshot，产生递归级别快照。

    用法::

        move_engine = MoveEngine()
        level_engine = RecursiveLevelEngine(level_id=2)
        for bar in bars:
            ...  # bi → seg → zhongshu → move
            move_snap = move_engine.process_zhongshu_snapshot(zs_snap)
            level_snap = level_engine.process_move_snapshot(move_snap)
            for evt in level_snap.zhongshu_events:
                handle(evt)
            for evt in level_snap.move_events:
                handle(evt)

    Parameters
    ----------
    level_id : int
        本递归级别的 ID（1-based）。level_id=2 表示从 level-1 的
        settled Move 构建 level-2 中枢和走势。
    stream_id : str
        所属流标识（透传到事件中，仅用于日志）。
    """

    def __init__(self, level_id: int, stream_id: str = "") -> None:
        self._level_id = level_id
        self._prev_zhongshus: list[LevelZhongshu] = []
        self._prev_moves: list[Move] = []
        self._event_seq: int = 0
        self._stream_id = stream_id

    @property
    def current_zhongshus(self) -> list[LevelZhongshu]:
        """当前递归级别中枢列表（浅拷贝）。"""
        return list(self._prev_zhongshus)

    @property
    def current_moves(self) -> list[Move]:
        """当前递归级别走势类型列表（浅拷贝）。"""
        return list(self._prev_moves)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    @property
    def level_id(self) -> int:
        """本递归级别 ID。"""
        return self._level_id

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._prev_zhongshus.clear()
        self._prev_moves.clear()
        self._event_seq = 0

    def process_move_snapshot(self, move_snap: MoveSnapshot) -> RecursiveLevelSnapshot:
        """处理一个 MoveSnapshot，产生递归级别中枢和走势事件。

        核心流程：
        1. 过滤 settled 走势类型
        2. adapt_moves → MoveAsComponent
        3. zhongshu_from_components → LevelZhongshu 列表
        4. diff → 中枢事件
        5. moves_from_level_zhongshus → Move 列表
        6. diff → 走势事件

        Parameters
        ----------
        move_snap : MoveSnapshot
            下级引擎产生的走势类型快照。

        Returns
        -------
        RecursiveLevelSnapshot
            包含本级别中枢、走势和两组域事件。
        """
        # 1. 过滤 settled 走势类型 — 只有结算的 Move 才参与递归
        settled_moves = [m for m in move_snap.moves if m.settled]

        # 2. 适配为 MoveAsComponent（带 level_id + component_idx）
        # 输入走势的 level = self._level_id - 1，输出中枢的 level = level_id
        components = adapt_moves(settled_moves, level_id=self._level_id - 1)

        # 3. 全量计算中枢
        curr_zhongshus = zhongshu_from_components(components)

        # 4. diff 产生中枢事件
        zs_events = diff_level_zhongshu(
            self._prev_zhongshus,
            curr_zhongshus,
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(zs_events)

        # 5. 从中枢列表计算走势类型
        curr_moves = moves_from_level_zhongshus(curr_zhongshus)

        # 6. diff 产生走势事件
        move_events = diff_level_moves(
            self._prev_moves,
            curr_moves,
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(move_events)

        # 7. 更新内部状态
        self._prev_zhongshus = curr_zhongshus
        self._prev_moves = curr_moves

        return RecursiveLevelSnapshot(
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            level_id=self._level_id,
            zhongshus=curr_zhongshus,
            moves=curr_moves,
            zhongshu_events=zs_events,
            move_events=move_events,
        )
