"""笔事件引擎 — 逐 bar 驱动的差分快照引擎

核心流程：
1. 累积原始 bar 数据
2. 每次 process_bar 用全量纯函数重跑管线（inclusion→fractals→strokes）
3. 差分前后 Stroke 快照 → 产生域事件

约束：
- 每次计算输入严格为 bars[:bar_idx+1]，保证无未来函数
- 复用现有经过 257 个测试验证的纯函数
- 对外暴露事件流，对内保留 Stroke.confirmed 语义
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

import numpy as np
import pandas as pd

from newchan.a_fractal import Fractal, fractals_from_merged
from newchan.a_inclusion import merge_inclusion
from newchan.a_stroke import Stroke, strokes_from_fractals
from newchan.audit.checker import InvariantChecker
from newchan.bi_differ import diff_strokes
from newchan.events import DomainEvent
from newchan.types import Bar


@dataclass
class BiEngineSnapshot:
    """一次 process_bar 后的完整快照。"""

    bar_idx: int
    bar_ts: float  # epoch 秒
    strokes: list[Stroke]
    events: list[DomainEvent]
    n_merged: int
    n_fractals: int


class BiEngine:
    """笔事件引擎 — 逐 bar 驱动，差分产生域事件。

    用法::

        engine = BiEngine()
        for bar in bars:
            snap = engine.process_bar(bar)
            for event in snap.events:
                handle(event)

    Parameters
    ----------
    stroke_mode : str
        笔模式，``"wide"``（宽笔，gap>=4）或 ``"strict"``（严笔）。
    min_strict_sep : int
        严笔模式下两分型最小间距。
    """

    def __init__(
        self,
        stroke_mode: str = "wide",
        min_strict_sep: int = 5,
    ) -> None:
        self._stroke_mode = stroke_mode
        self._min_strict_sep = min_strict_sep

        # 累积的原始 bar 数据（用于构造 DataFrame）
        self._bar_ohlc: list[list[float]] = []  # [open, high, low, close]
        self._bar_timestamps: list[datetime] = []

        # 状态
        self._prev_strokes: list[Stroke] = []
        self._bar_idx: int = -1
        self._event_seq: int = 0

        # 运行时不变量检查器
        self._checker = InvariantChecker()

    @property
    def bar_count(self) -> int:
        """已处理的 bar 总数。"""
        return len(self._bar_ohlc)

    @property
    def current_strokes(self) -> list[Stroke]:
        """当前快照的笔列表（浅拷贝）。"""
        return list(self._prev_strokes)

    @property
    def event_seq(self) -> int:
        """当前全局事件序号。"""
        return self._event_seq

    def reset(self) -> None:
        """重置引擎到初始状态（用于回放 seek）。"""
        self._bar_ohlc.clear()
        self._bar_timestamps.clear()
        self._prev_strokes.clear()
        self._bar_idx = -1
        self._event_seq = 0
        self._checker.reset()

    def process_bar(self, bar: Bar) -> BiEngineSnapshot:
        """处理一根新 K 线，返回快照（含本 bar 产生的事件）。

        保证：
        1. 仅使用 bars[:bar_idx+1] 的数据（无未来函数）
        2. 调用现有纯函数做全量计算
        3. diff 产生事件
        """
        self._bar_idx += 1
        self._bar_ohlc.append([bar.open, bar.high, bar.low, bar.close])
        self._bar_timestamps.append(bar.ts)

        # 计算当前 bar 的 epoch 秒
        bar_ts = _dt_to_epoch(bar.ts)

        # 构造 DataFrame
        df = _build_df(self._bar_ohlc, self._bar_timestamps)

        # 调用现有管线
        df_merged, _merged_to_raw = merge_inclusion(df)
        fractals = fractals_from_merged(df_merged)
        strokes = strokes_from_fractals(
            df_merged,
            fractals,
            mode=self._stroke_mode,
            min_strict_sep=self._min_strict_sep,
        )

        # 差分
        events = diff_strokes(
            self._prev_strokes,
            strokes,
            bar_idx=self._bar_idx,
            bar_ts=bar_ts,
            seq_start=self._event_seq,
        )
        self._event_seq += len(events)

        # 运行时不变量检查（I1-I4）
        violations = self._checker.check(events, self._bar_idx, bar_ts)
        if violations:
            events = list(events) + violations
            self._event_seq += len(violations)

        # 更新状态
        self._prev_strokes = strokes

        return BiEngineSnapshot(
            bar_idx=self._bar_idx,
            bar_ts=bar_ts,
            strokes=strokes,
            events=events,
            n_merged=len(df_merged),
            n_fractals=len(fractals),
        )


# ── 内部工具函数 ──


def _dt_to_epoch(dt: datetime) -> float:
    """datetime → epoch 秒。naive datetime 视为 UTC。"""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()


def _build_df(
    ohlc: list[list[float]],
    timestamps: list[datetime],
) -> pd.DataFrame:
    """从累积数据构造 pandas DataFrame。"""
    arr = np.array(ohlc, dtype=np.float64)
    return pd.DataFrame(
        arr,
        columns=["open", "high", "low", "close"],
        index=pd.DatetimeIndex(timestamps, name="time"),
    )
