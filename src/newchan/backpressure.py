"""背压控制 — WebSocket 推送路径的有界队列

当客户端消费速度跟不上服务端推送速度时，队列满后丢弃非关键帧（中间状态），
保留买卖点信号帧（bsp_*），防止内存无限增长。
"""

from __future__ import annotations

import asyncio
from collections import deque
from typing import Any

# 买卖点信号事件类型 — 这些帧在背压丢弃时必须保留
_BSP_EVENT_TYPES: frozenset[str] = frozenset({
    "bsp_candidate",
    "bsp_confirm",
    "bsp_settle",
    "bsp_invalidate",
})

DEFAULT_MAX_SIZE: int = 100


def is_critical_frame(message: dict[str, Any]) -> bool:
    """判断消息是否为关键帧（买卖点信号）。

    关键帧定义：type == "event" 且 event_type 属于 bsp 信号集合。
    其余所有消息（bar、snapshot、replay_status、非 bsp event）均为非关键帧。
    """
    return (
        message.get("type") == "event"
        and message.get("event_type", "") in _BSP_EVENT_TYPES
    )


class BackpressureQueue:
    """异步有界队列，队列满时丢弃非关键帧。

    Parameters
    ----------
    maxsize : int
        队列容量上限，默认 100。
    """

    __slots__ = ("_maxsize", "_queue", "_notify", "_dropped_count")

    def __init__(self, maxsize: int = DEFAULT_MAX_SIZE) -> None:
        if maxsize < 1:
            raise ValueError(f"maxsize must be >= 1, got {maxsize}")
        self._maxsize: int = maxsize
        self._queue: deque[dict[str, Any]] = deque()
        self._notify: asyncio.Event = asyncio.Event()
        self._dropped_count: int = 0

    @property
    def maxsize(self) -> int:
        return self._maxsize

    @property
    def dropped_count(self) -> int:
        """累计丢弃的非关键帧数量。"""
        return self._dropped_count

    def qsize(self) -> int:
        return len(self._queue)

    def full(self) -> bool:
        return len(self._queue) >= self._maxsize

    def empty(self) -> bool:
        return len(self._queue) == 0

    def put_nowait(self, message: dict[str, Any]) -> bool:
        """尝试入队。

        - 队列未满：直接入队，返回 True。
        - 队列已满 + 关键帧：驱逐最旧的非关键帧腾出空间，入队，返回 True。
          如果队列全是关键帧且已满，仍然强制入队（突破 maxsize），返回 True。
        - 队列已满 + 非关键帧：丢弃该消息，返回 False。
        """
        critical = is_critical_frame(message)

        if not self.full():
            self._queue.append(message)
            self._notify.set()
            return True

        if not critical:
            # 非关键帧，队列满，丢弃
            self._dropped_count += 1
            return False

        # 关键帧，队列满 — 驱逐最旧的非关键帧
        evicted = self._evict_oldest_non_critical()
        if not evicted:
            # 队列全是关键帧，强制入队（不丢失信号）
            pass
        self._queue.append(message)
        self._notify.set()
        return True

    async def get(self) -> dict[str, Any]:
        """从队列头部取出一条消息。队列为空时等待。"""
        while self.empty():
            self._notify.clear()
            await self._notify.wait()
        message = self._queue.popleft()
        return message

    def get_nowait(self) -> dict[str, Any]:
        """非阻塞取出。队列为空时抛出 IndexError。"""
        if self.empty():
            raise IndexError("queue is empty")
        return self._queue.popleft()

    def clear(self) -> int:
        """清空队列，返回被清除的消息数量。"""
        count = len(self._queue)
        self._queue.clear()
        self._notify.clear()
        return count

    def _evict_oldest_non_critical(self) -> bool:
        """驱逐队列中最旧的非关键帧。返回是否成功驱逐。"""
        for i, msg in enumerate(self._queue):
            if not is_critical_frame(msg):
                del self._queue[i]
                self._dropped_count += 1
                return True
        return False
