"""递归层模块 — 线段引擎 + 中枢引擎 + 走势类型引擎 + 状态管理

从笔事件驱动的线段计算：
- SegmentEngine: 事件驱动线段引擎（消费 BiEngineSnapshot）
- SegmentSnapshot: 线段快照
- diff_segments: 线段列表差分 → 线段事件

从线段事件驱动的中枢计算：
- ZhongshuEngine: 事件驱动中枢引擎（消费 SegmentSnapshot）
- ZhongshuSnapshot: 中枢快照
- diff_zhongshu: 中枢列表差分 → 中枢事件

从中枢事件驱动的走势类型计算：
- MoveEngine: 事件驱动走势类型引擎（消费 ZhongshuSnapshot）
- MoveSnapshot: 走势类型快照
- diff_moves: Move 列表差分 → 走势类型事件
"""

from newchan.core.recursion.segment_state import SegmentSnapshot, diff_segments
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot, diff_zhongshu
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.core.recursion.move_state import MoveSnapshot, diff_moves
from newchan.core.recursion.move_engine import MoveEngine

__all__ = [
    "SegmentEngine",
    "SegmentSnapshot",
    "diff_segments",
    "ZhongshuEngine",
    "ZhongshuSnapshot",
    "diff_zhongshu",
    "MoveEngine",
    "MoveSnapshot",
    "diff_moves",
]
