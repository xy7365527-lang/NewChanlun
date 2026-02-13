"""内存级引擎指标

轻量指标收集，零外部依赖。
未来可通过 exporter 接入 Prometheus/StatsD。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from time import monotonic


@dataclass
class EngineMetrics:
    """单 TF 引擎指标。

    Usage::

        metrics = EngineMetrics(tf="5m")
        with metrics.measure_bar():
            snap = engine.process_bar(bar)
        metrics.record_bar(event_count=len(snap.events), bar_ts=snap.bar_ts)
    """

    tf: str = ""
    events_total: int = 0
    violations_total: int = 0
    bars_processed: int = 0
    last_bar_ts: float = 0.0
    last_process_duration_ms: float = 0.0
    _timer_start: float = field(default=0.0, repr=False)

    def start_timer(self) -> None:
        """开始计时（process_bar 前调用）。"""
        self._timer_start = monotonic()

    def stop_timer(self) -> None:
        """停止计时并记录耗时。"""
        if self._timer_start > 0:
            self.last_process_duration_ms = (monotonic() - self._timer_start) * 1000
            self._timer_start = 0.0

    def record_bar(self, *, event_count: int, bar_ts: float) -> None:
        """记录一根 bar 的处理结果。"""
        self.bars_processed += 1
        self.events_total += event_count
        self.last_bar_ts = bar_ts

    def record_violation(self, count: int = 1) -> None:
        """记录违规事件。"""
        self.violations_total += count

    def snapshot(self) -> dict:
        """导出当前指标快照。"""
        return {
            "tf": self.tf,
            "events_total": self.events_total,
            "violations_total": self.violations_total,
            "bars_processed": self.bars_processed,
            "last_bar_ts": self.last_bar_ts,
            "last_process_duration_ms": round(self.last_process_duration_ms, 3),
        }

    def reset(self) -> None:
        """重置所有计数器。"""
        self.events_total = 0
        self.violations_total = 0
        self.bars_processed = 0
        self.last_bar_ts = 0.0
        self.last_process_duration_ms = 0.0
