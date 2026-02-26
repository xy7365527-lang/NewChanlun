"""LiveEngine — 实时数据驱动 RecursiveOrchestrator 增量计算

接收实时 bar（来自 DatabentoLiveFeeder 或其他数据源），
逐 bar 驱动 RecursiveOrchestrator，产出 RecursiveOrchestratorSnapshot。

与回放共用 WsSnapshot 结构，不引入新消息格式。
"""

from __future__ import annotations

import asyncio
import logging
from datetime import datetime, timezone
from typing import Callable

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

logger = logging.getLogger(__name__)

# 监听器签名：(symbol, bar, snapshot) -> None
LiveListener = Callable[[str, Bar, RecursiveOrchestratorSnapshot], None]


class LiveEngine:
    """实时引擎 — 每个 symbol 持有独立的 RecursiveOrchestrator。

    Parameters
    ----------
    stroke_mode : str
        笔模式（透传到 RecursiveOrchestrator）。
    min_strict_sep : int
        严格分型最小间隔（透传到 RecursiveOrchestrator）。
    """

    def __init__(
        self,
        stroke_mode: str = "wide",
        min_strict_sep: int = 5,
    ) -> None:
        self._stroke_mode = stroke_mode
        self._min_strict_sep = min_strict_sep
        self._orchestrators: dict[str, RecursiveOrchestrator] = {}
        self._bar_counts: dict[str, int] = {}
        self._latest_snapshots: dict[str, RecursiveOrchestratorSnapshot] = {}
        self._last_bar_ts: dict[str, datetime] = {}
        self._listeners: list[LiveListener] = []

    def _get_or_create(self, symbol: str) -> RecursiveOrchestrator:
        """获取或创建指定 symbol 的 orchestrator。"""
        if symbol not in self._orchestrators:
            self._orchestrators[symbol] = RecursiveOrchestrator(
                stream_id=f"live:{symbol}",
                stroke_mode=self._stroke_mode,
                min_strict_sep=self._min_strict_sep,
            )
            self._bar_counts[symbol] = 0
        return self._orchestrators[symbol]

    def process_bar(self, symbol: str, bar: Bar) -> RecursiveOrchestratorSnapshot:
        """处理一根实时 bar，返回完整快照。

        驱动 RecursiveOrchestrator.process_bar()，存储最新快照，
        通知所有已注册的监听器。
        """
        orch = self._get_or_create(symbol)
        snap = orch.process_bar(bar)

        self._bar_counts[symbol] = self._bar_counts.get(symbol, 0) + 1
        self._latest_snapshots[symbol] = snap
        self._last_bar_ts[symbol] = bar.ts

        count = self._bar_counts[symbol]
        if count <= 3 or count % 100 == 0:
            logger.info(
                "LiveEngine %s bar #%d idx=%d events=%d",
                symbol, count, snap.bar_idx, len(snap.all_events),
            )

        for listener in self._listeners:
            try:
                listener(symbol, bar, snap)
            except Exception:
                logger.exception("LiveEngine listener error")

        return snap

    def add_listener(self, fn: LiveListener) -> None:
        """注册监听器，每次 process_bar 后回调。"""
        self._listeners.append(fn)

    def remove_listener(self, fn: LiveListener) -> None:
        """移除监听器。"""
        self._listeners = [f for f in self._listeners if f is not fn]

    def latest_snapshot(self, symbol: str) -> RecursiveOrchestratorSnapshot | None:
        """获取指定 symbol 的最新快照。"""
        return self._latest_snapshots.get(symbol)

    def bar_count(self, symbol: str) -> int:
        """获取指定 symbol 已处理的 bar 数量。"""
        return self._bar_counts.get(symbol, 0)

    @property
    def symbols(self) -> list[str]:
        """当前活跃的 symbol 列表。"""
        return list(self._orchestrators.keys())

    def last_bar_ts(self, symbol: str) -> datetime | None:
        """获取指定 symbol 最后一根 bar 的时间戳。"""
        return self._last_bar_ts.get(symbol)

    def reset(self, symbol: str | None = None) -> None:
        """重置引擎状态。symbol=None 重置全部。"""
        if symbol is not None:
            if symbol in self._orchestrators:
                self._orchestrators[symbol].reset()
                self._bar_counts[symbol] = 0
                self._latest_snapshots.pop(symbol, None)
                self._last_bar_ts.pop(symbol, None)
        else:
            for orch in self._orchestrators.values():
                orch.reset()
            self._bar_counts.clear()
            self._latest_snapshots.clear()
            self._last_bar_ts.clear()


# ---------------------------------------------------------------------------
# Gap 检测结果
# ---------------------------------------------------------------------------

class GapInfo:
    """记录某个 symbol 断线期间的数据缺口。"""

    __slots__ = ("symbol", "gap_start", "gap_end", "bars_filled")

    def __init__(
        self,
        symbol: str,
        gap_start: datetime,
        gap_end: datetime,
        bars_filled: int = 0,
    ) -> None:
        self.symbol = symbol
        self.gap_start = gap_start
        self.gap_end = gap_end
        self.bars_filled = bars_filled

    def __repr__(self) -> str:
        return (
            f"GapInfo({self.symbol}, {self.gap_start} → {self.gap_end}, "
            f"filled={self.bars_filled})"
        )


# ---------------------------------------------------------------------------
# 断线检测 + 指数退避重连
# ---------------------------------------------------------------------------

# 数据源协议：connect / subscribe / iter_bars / close
# 历史回填协议：fetch_bars(symbol, start, end) -> list[Bar]

class ReconnectingLiveEngine:
    """在 LiveEngine 之上封装断线检测和指数退避重连。

    Parameters
    ----------
    engine : LiveEngine
        底层引擎实例（重连后不重置，保证状态连续性）。
    connect_fn : async callable
        建立连接的异步函数，失败时应抛出异常。
    subscribe_fn : async callable(symbols) -> AsyncIterator[tuple[str, Bar]]
        订阅并返回 bar 流的异步函数。
    fetch_history_fn : async callable(symbol, start, end) -> list[Bar]
        历史数据回填函数，用于填补断线期间的 gap。
    symbols : list[str]
        订阅的品种列表。
    initial_backoff : float
        初始退避秒数，默认 1.0。
    max_backoff : float
        最大退避秒数，默认 60.0。
    backoff_factor : float
        退避倍数，默认 2.0。
    """

    def __init__(
        self,
        engine: LiveEngine,
        connect_fn: Callable[[], object],
        subscribe_fn: Callable[..., object],
        fetch_history_fn: Callable[..., object],
        symbols: list[str],
        initial_backoff: float = 1.0,
        max_backoff: float = 60.0,
        backoff_factor: float = 2.0,
    ) -> None:
        self._engine = engine
        self._connect_fn = connect_fn
        self._subscribe_fn = subscribe_fn
        self._fetch_history_fn = fetch_history_fn
        self._symbols = list(symbols)

        self._initial_backoff = initial_backoff
        self._max_backoff = max_backoff
        self._backoff_factor = backoff_factor

        self._current_backoff = initial_backoff
        self._running = False
        self._reconnect_count = 0
        self._last_disconnect: datetime | None = None
        self._gaps: list[GapInfo] = []

    @property
    def engine(self) -> LiveEngine:
        return self._engine

    @property
    def reconnect_count(self) -> int:
        return self._reconnect_count

    @property
    def current_backoff(self) -> float:
        return self._current_backoff

    @property
    def gaps(self) -> list[GapInfo]:
        return list(self._gaps)

    @property
    def is_running(self) -> bool:
        return self._running

    def _reset_backoff(self) -> None:
        self._current_backoff = self._initial_backoff

    def _advance_backoff(self) -> float:
        """返回当前退避值，然后递增（不超过 max_backoff）。"""
        wait = self._current_backoff
        self._current_backoff = min(
            self._current_backoff * self._backoff_factor,
            self._max_backoff,
        )
        return wait

    async def _fill_gap(self, symbol: str, disconnect_ts: datetime) -> GapInfo | None:
        """检测并回填某个 symbol 的数据 gap。"""
        last_ts = self._engine.last_bar_ts(symbol)
        if last_ts is None:
            return None

        gap_start = last_ts
        gap_end = disconnect_ts

        if gap_end <= gap_start:
            return None

        try:
            bars = await self._fetch_history_fn(symbol, gap_start, gap_end)
        except Exception:
            logger.exception("Gap 回填失败: %s %s→%s", symbol, gap_start, gap_end)
            return GapInfo(symbol, gap_start, gap_end, bars_filled=0)

        filled = 0
        for bar in bars:
            if bar.ts <= gap_start:
                continue
            self._engine.process_bar(symbol, bar)
            filled += 1

        gap = GapInfo(symbol, gap_start, gap_end, bars_filled=filled)
        if filled > 0:
            logger.info("Gap 已回填: %s", gap)
        return gap

    async def _reconnect_once(self) -> bool:
        """尝试一次重连。成功返回 True，失败返回 False。"""
        try:
            await self._connect_fn()
            return True
        except Exception:
            logger.exception("重连失败")
            return False

    async def run(self) -> None:
        """主循环：连接 → 消费 bar 流 → 断线后退避重连。

        调用 stop() 可终止循环。
        """
        self._running = True
        is_first_connect = True

        while self._running:
            # --- 连接阶段 ---
            connected = await self._reconnect_once()
            if not connected:
                if not self._running:
                    break
                wait = self._advance_backoff()
                logger.warning("将在 %.1fs 后重试连接", wait)
                await asyncio.sleep(wait)
                continue

            # 连接成功：重置退避
            self._reset_backoff()

            if not is_first_connect:
                self._reconnect_count += 1
                # 回填 gap
                reconnect_ts = datetime.now(timezone.utc)
                for sym in self._symbols:
                    gap = await self._fill_gap(sym, reconnect_ts)
                    if gap is not None:
                        self._gaps.append(gap)
            is_first_connect = False

            # --- 消费阶段 ---
            try:
                bar_stream = self._subscribe_fn(self._symbols)
                async for symbol, bar in bar_stream:
                    if not self._running:
                        break
                    self._engine.process_bar(symbol, bar)
            except Exception:
                logger.exception("数据流中断")

            # 记录断线时刻
            self._last_disconnect = datetime.now(timezone.utc)

            if not self._running:
                break

            wait = self._advance_backoff()
            logger.warning(
                "连接断开，%.1fs 后重试（已重连 %d 次）",
                wait, self._reconnect_count,
            )
            await asyncio.sleep(wait)

    def stop(self) -> None:
        """停止主循环。"""
        self._running = False
