"""LivePool — 多标的并发管理池

通过 asyncio task pool 管理多个标的的实时数据订阅。
每个标的持有独立的 LiveEngine 实例和 asyncio.Queue，
由独立的 asyncio.Task 消费 bar 并驱动引擎。

与回放共用 WsSnapshot 结构，不引入新消息格式。
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any, Awaitable, Callable

from newchan.live_engine import LiveEngine
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.types import Bar

logger = logging.getLogger(__name__)

# 广播回调签名：(symbol, bar, bar_idx, snapshot) -> None
BroadcastCallback = Callable[
    [str, Bar, int, RecursiveOrchestratorSnapshot], Awaitable[None]
]


class LivePool:
    """多标的并发管理池。

    Parameters
    ----------
    max_symbols : int
        最大并发标的数（默认 10）。
    stroke_mode : str
        笔模式，透传到 LiveEngine。
    min_strict_sep : int
        严格分型最小间隔，透传到 LiveEngine。
    on_update : BroadcastCallback | None
        每次 bar 处理完成后的异步回调，用于 WebSocket 广播。
    """

    def __init__(
        self,
        max_symbols: int = 10,
        stroke_mode: str = "wide",
        min_strict_sep: int = 5,
        on_update: BroadcastCallback | None = None,
    ) -> None:
        self._max_symbols = max_symbols
        self._stroke_mode = stroke_mode
        self._min_strict_sep = min_strict_sep
        self._on_update = on_update

        self._engines: dict[str, LiveEngine] = {}
        self._queues: dict[str, asyncio.Queue[Bar | None]] = {}
        self._tasks: dict[str, asyncio.Task[None]] = {}
        self._bar_counts: dict[str, int] = {}

    @property
    def max_symbols(self) -> int:
        return self._max_symbols

    @property
    def active_symbols(self) -> list[str]:
        return list(self._engines.keys())

    async def add_symbol(self, symbol: str) -> None:
        """添加标的订阅。幂等：重复添加不报错。"""
        symbol = symbol.upper()
        if symbol in self._engines:
            return
        if len(self._engines) >= self._max_symbols:
            raise ValueError(
                f"并发上限 {self._max_symbols}，无法添加 {symbol}"
            )

        engine = LiveEngine(
            stroke_mode=self._stroke_mode,
            min_strict_sep=self._min_strict_sep,
        )
        queue: asyncio.Queue[Bar | None] = asyncio.Queue()

        self._engines[symbol] = engine
        self._queues[symbol] = queue
        self._bar_counts[symbol] = 0
        self._tasks[symbol] = asyncio.create_task(
            self._run_symbol(symbol),
            name=f"live-pool-{symbol}",
        )
        logger.info(
            "LivePool: added %s (%d/%d)",
            symbol,
            len(self._engines),
            self._max_symbols,
        )

    async def remove_symbol(self, symbol: str) -> None:
        """移除标的订阅。幂等：移除不存在的标的不报错。"""
        symbol = symbol.upper()
        if symbol not in self._engines:
            return

        # 发送哨兵值停止消费循环
        queue = self._queues.get(symbol)
        if queue is not None:
            await queue.put(None)

        # 等待 task 结束
        task = self._tasks.pop(symbol, None)
        if task is not None and not task.done():
            try:
                await asyncio.wait_for(asyncio.shield(task), timeout=5.0)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                task.cancel()
                try:
                    await task
                except asyncio.CancelledError:
                    pass

        self._engines.pop(symbol, None)
        self._queues.pop(symbol, None)
        self._bar_counts.pop(symbol, None)
        logger.info(
            "LivePool: removed %s (%d/%d)",
            symbol,
            len(self._engines),
            self._max_symbols,
        )

    def feed_bar(self, symbol: str, bar: Bar) -> None:
        """向指定标的投递一根 bar。

        线程安全（asyncio.Queue.put_nowait）。
        标的不存在时静默忽略。
        """
        symbol = symbol.upper()
        queue = self._queues.get(symbol)
        if queue is not None:
            queue.put_nowait(bar)

    def get_status(self) -> dict[str, Any]:
        """返回池状态快照。"""
        return {
            "max_symbols": self._max_symbols,
            "active_count": len(self._engines),
            "symbols": {
                sym: {
                    "bar_count": self._bar_counts.get(sym, 0),
                    "task_alive": (
                        sym in self._tasks and not self._tasks[sym].done()
                    ),
                    "queue_size": (
                        self._queues[sym].qsize() if sym in self._queues else 0
                    ),
                }
                for sym in self._engines
            },
        }

    async def shutdown(self) -> None:
        """关闭所有标的，清理资源。"""
        symbols = list(self._engines.keys())
        for sym in symbols:
            await self.remove_symbol(sym)

    async def _run_symbol(self, symbol: str) -> None:
        """单标的消费循环：从 queue 取 bar → 驱动 engine → 回调广播。"""
        queue = self._queues[symbol]
        engine = self._engines[symbol]

        try:
            while True:
                bar = await queue.get()
                if bar is None:
                    break
                snap = engine.process_bar(symbol, bar)
                idx = self._bar_counts.get(symbol, 0)
                self._bar_counts[symbol] = idx + 1

                if self._on_update is not None:
                    try:
                        await self._on_update(symbol, bar, idx, snap)
                    except Exception:
                        logger.exception(
                            "LivePool on_update error for %s", symbol
                        )
        except asyncio.CancelledError:
            pass
        except Exception:
            logger.exception("LivePool _run_symbol %s crashed", symbol)
