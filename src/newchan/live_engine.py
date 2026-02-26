"""LiveEngine — 实时数据驱动 RecursiveOrchestrator 增量计算

接收实时 bar（来自 DatabentoLiveFeeder 或其他数据源），
逐 bar 驱动 RecursiveOrchestrator，产出 RecursiveOrchestratorSnapshot。

与回放共用 WsSnapshot 结构，不引入新消息格式。
"""

from __future__ import annotations

import logging
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

    def reset(self, symbol: str | None = None) -> None:
        """重置引擎状态。symbol=None 重置全部。"""
        if symbol is not None:
            if symbol in self._orchestrators:
                self._orchestrators[symbol].reset()
                self._bar_counts[symbol] = 0
                self._latest_snapshots.pop(symbol, None)
        else:
            for orch in self._orchestrators.values():
                orch.reset()
            self._bar_counts.clear()
            self._latest_snapshots.clear()
