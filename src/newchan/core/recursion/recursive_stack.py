"""递归栈调度器 — 自底向上驱动多层递归。

[新缠论] 管理多层 RecursiveLevelEngine 的自动递归：
  MoveSnapshot[1] → RecursiveLevelEngine(level=2) → RecursiveLevelSnapshot[2]
                                                          │ → MoveSnapshot
                                                          ↓
                    RecursiveLevelEngine(level=3) → RecursiveLevelSnapshot[3]
                                                          │
                                                         ... → 终止条件

终止条件（双重）：
  1. len(moves) < 3（数据耗尽）
  2. PH persistence 尺度分离（当前层 max < 前一层 min）
"""

from __future__ import annotations

from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_engine import RecursiveLevelEngine
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.ph_layer import should_stop_recursion


class RecursiveStack:
    """递归栈调度器 — 自底向上驱动多层递归。

    从 level=1 的 MoveSnapshot 开始，懒创建 RecursiveLevelEngine，
    逐层向上递归，直到终止条件满足。

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
        # 顶层短路缓存：level-1 输入 move 未变 ⟹ 全部递归层输出不变
        self._last_input_key: tuple = ()
        self._cached_snapshots: list[RecursiveLevelSnapshot] = []

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
        self._last_input_key = ()
        self._cached_snapshots = []

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
        # 顶层短路：递归只消费 settled moves（排除末未settled move）。由单调前缀
        # 不变量，settled 集合只能经 append（n变）或末move settle（settled变）改变。
        # 故 in_key 未变 ⟹ 全部递归层输出不变，直接复用缓存（仅刷新 bar_idx/ts）。
        # 认识论等级：L0（结构不变量推导，逐位等价）。
        in_moves = move_snap.moves
        n_in = len(in_moves)
        if n_in >= 1:
            lm = in_moves[-1]
            in_key = (n_in, lm.seg_start, lm.seg_end, lm.settled)
        else:
            in_key = (0,)

        if in_key == self._last_input_key:
            return [
                RecursiveLevelSnapshot(
                    bar_idx=move_snap.bar_idx,
                    bar_ts=move_snap.bar_ts,
                    level_id=s.level_id,
                    zhongshus=s.zhongshus,
                    moves=s.moves,
                    zhongshu_events=[],
                    move_events=[],
                )
                for s in self._cached_snapshots
            ]
        self._last_input_key = in_key

        snapshots: list[RecursiveLevelSnapshot] = []
        current_move_snap = move_snap
        current_level = 1
        prev_moves = move_snap.moves

        while current_level < self._max_levels:
            next_level = current_level + 1

            if next_level not in self._engines:
                self._engines[next_level] = RecursiveLevelEngine(
                    level_id=next_level, stream_id=self._stream_id,
                )

            engine = self._engines[next_level]
            # persistence 已在 RecursiveLevelEngine 守卫内附着（不再每 bar 重算）。
            snap = engine.process_move_snapshot(current_move_snap)

            snapshots.append(snap)

            if len(snap.moves) < 3:
                break

            if should_stop_recursion(snap.moves, prev_moves):
                break

            current_move_snap = MoveSnapshot(
                bar_idx=snap.bar_idx,
                bar_ts=snap.bar_ts,
                moves=snap.moves,
                events=snap.move_events,
            )
            prev_moves = snap.moves
            current_level = next_level

        self._cached_snapshots = snapshots
        return snapshots
