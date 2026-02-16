"""递归栈调度器 — 自底向上驱动多层递归。

[新缠论] 管理多层 RecursiveLevelEngine 的自动递归：
  MoveSnapshot[1] → RecursiveLevelEngine(level=2) → RecursiveLevelSnapshot[2]
                                                          │ → MoveSnapshot
                                                          ↓
                    RecursiveLevelEngine(level=3) → RecursiveLevelSnapshot[3]
                                                          │
                                                         ... → 终止: len(moves) < 3
"""

from __future__ import annotations

from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_engine import RecursiveLevelEngine
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot


class RecursiveStack:
    """递归栈调度器 — 自底向上驱动多层递归。

    从 level=1 的 MoveSnapshot 开始，懒创建 RecursiveLevelEngine，
    逐层向上递归，直到某层 Move 不足 3 个时终止。

    Parameters
    ----------
    max_levels : int
        最大递归深度（安全阀）。默认 6。level_id 最大值 = max_levels。
    stream_id : str
        流标识（透传到各层引擎）。
    """

    def __init__(self, max_levels: int = 6, stream_id: str = "") -> None:
        self._max_levels = max_levels
        self._stream_id = stream_id
        self._engines: dict[int, RecursiveLevelEngine] = {}

    @property
    def max_levels(self) -> int:
        """最大递归深度。"""
        return self._max_levels

    @property
    def active_levels(self) -> int:
        """当前已创建的引擎数量。"""
        return len(self._engines)

    def reset(self) -> None:
        """重置所有引擎到初始状态（用于回放 seek）。"""
        for engine in self._engines.values():
            engine.reset()
        self._engines.clear()

    def process_level1_move_snapshot(
        self, move_snap: MoveSnapshot
    ) -> list[RecursiveLevelSnapshot]:
        """从 level=1 的 MoveSnapshot 开始，递归向上处理所有可处理的层级。

        Returns
        -------
        list[RecursiveLevelSnapshot]
            按 level_id 递增排序的各层快照。至少包含 level=2 的快照
            （可能为空快照，即无中枢无走势）。
        """
        snapshots: list[RecursiveLevelSnapshot] = []
        current_move_snap = move_snap
        current_level = 1

        while current_level < self._max_levels:
            next_level = current_level + 1

            # 懒创建引擎
            if next_level not in self._engines:
                self._engines[next_level] = RecursiveLevelEngine(
                    level_id=next_level, stream_id=self._stream_id,
                )

            engine = self._engines[next_level]
            snap = engine.process_move_snapshot(current_move_snap)
            snapshots.append(snap)

            # 递归终止条件：本层 Move 不足 3 个
            if len(snap.moves) < 3:
                break

            # 向上递归：将本层产出转换为下一层的输入
            current_move_snap = MoveSnapshot(
                bar_idx=snap.bar_idx,
                bar_ts=snap.bar_ts,
                moves=snap.moves,
                events=snap.move_events,
            )
            current_level = next_level

        return snapshots
