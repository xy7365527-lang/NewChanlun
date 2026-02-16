"""A 系统 — 级别递归协议 (MoveProtocol) + 适配器

P1: MoveProtocol — 可作为中枢组件的对象的最小接口。
P2: SegmentAsComponent / MoveAsComponent — 将现有 Segment/Move 适配到 MoveProtocol。

设计原则：
- 不修改现有 Segment / Move / Zhongshu 数据类
- 用 Protocol + 适配器实现泛化，不用继承
- 所有新数据类: frozen + slots
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, Protocol, runtime_checkable

from newchan.a_move_v1 import Move
from newchan.a_segment_v0 import Segment

__all__ = [
    "MoveProtocol",
    "SegmentAsComponent",
    "MoveAsComponent",
    "adapt_segments",
    "adapt_moves",
]


# ====================================================================
# P1: MoveProtocol
# ====================================================================


@runtime_checkable
class MoveProtocol(Protocol):
    """可作为中枢组件的对象的最小接口。

    任何满足此协议的对象都可以作为泛化中枢构造函数的输入，
    从而实现级别递归：Move[k] -> Center[k+1] -> Move[k+1] -> ...
    """

    @property
    def component_idx(self) -> int:
        """组件在序列中的位置索引。"""
        ...

    @property
    def high(self) -> float:
        """组件价格上界。"""
        ...

    @property
    def low(self) -> float:
        """组件价格下界。"""
        ...

    @property
    def direction(self) -> Literal["up", "down"]:
        """组件方向。"""
        ...

    @property
    def completed(self) -> bool:
        """组件是否已确认/结算。"""
        ...

    @property
    def level_id(self) -> int:
        """组件所属级别（递归层级）。"""
        ...


# ====================================================================
# P2: 适配器
# ====================================================================


@dataclass(frozen=True, slots=True)
class SegmentAsComponent:
    """将 Segment 适配为 MoveProtocol。

    level_id 固定为 0（线段是最底层组件）。
    completed = confirmed AND kind == "settled"。
    """

    _segment: Segment
    _component_idx: int

    @property
    def component_idx(self) -> int:
        return self._component_idx

    @property
    def high(self) -> float:
        return self._segment.high

    @property
    def low(self) -> float:
        return self._segment.low

    @property
    def direction(self) -> Literal["up", "down"]:
        return self._segment.direction

    @property
    def completed(self) -> bool:
        return self._segment.confirmed and self._segment.kind == "settled"

    @property
    def level_id(self) -> int:
        return 0


@dataclass(frozen=True, slots=True)
class MoveAsComponent:
    """将 Move 适配为 MoveProtocol。

    level_id 由调用方指定（Move 所属的递归层级）。
    completed = Move.settled。
    """

    _move: Move
    _component_idx: int
    _level_id: int

    @property
    def component_idx(self) -> int:
        return self._component_idx

    @property
    def high(self) -> float:
        return self._move.high

    @property
    def low(self) -> float:
        return self._move.low

    @property
    def direction(self) -> Literal["up", "down"]:
        return self._move.direction

    @property
    def completed(self) -> bool:
        return self._move.settled

    @property
    def level_id(self) -> int:
        return self._level_id


# ====================================================================
# 辅助函数
# ====================================================================


def adapt_segments(segments: list[Segment]) -> list[SegmentAsComponent]:
    """批量将 Segment 列表适配为 SegmentAsComponent 列表。

    component_idx 按列表顺序从 0 开始编号。
    """
    return [
        SegmentAsComponent(_segment=s, _component_idx=i)
        for i, s in enumerate(segments)
    ]


def adapt_moves(moves: list[Move], level_id: int) -> list[MoveAsComponent]:
    """批量将 Move 列表适配为 MoveAsComponent 列表。

    component_idx 按列表顺序从 0 开始编号。
    level_id 统一指定。
    """
    return [
        MoveAsComponent(_move=m, _component_idx=i, _level_id=level_id)
        for i, m in enumerate(moves)
    ]
