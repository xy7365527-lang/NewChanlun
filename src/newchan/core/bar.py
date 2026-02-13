"""BarV1 — 标准化 K 线（V1 冻结规格）

相比旧 Bar 的增强：
1. bar_time 语义明确：始终是 bar 的**开始时间**（epoch 秒）
2. is_closed 标记：区分已完成 / 实时更新中的 bar
3. stream_id 归属：知道自己属于哪条流
4. frozen=True：不可变值对象
"""

from __future__ import annotations

import warnings
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class BarV1:
    """标准化 K 线。

    Attributes
    ----------
    bar_time : float
        bar 的**开始时间**，epoch 秒。
    open : float
        开盘价。
    high : float
        最高价。
    low : float
        最低价。
    close : float
        收盘价。
    volume : float
        成交量（无则为 0.0）。
    is_closed : bool
        True = bar 已完成（回放始终 True），False = 实时更新中。
    stream_id : str
        所属流的 StreamId.value（空串 = 未指定）。

    Invariants
    ----------
    - bar_time > 0
    - high >= low（异常数据发出 warning，不 raise）
    - 同一 (stream_id, bar_time) 只有一个 is_closed=True 的最终 bar
    """

    bar_time: float
    open: float
    high: float
    low: float
    close: float
    volume: float = 0.0
    is_closed: bool = True
    stream_id: str = ""

    def __post_init__(self) -> None:
        if self.bar_time <= 0:
            raise ValueError(f"bar_time 必须 > 0: {self.bar_time}")
        if self.high < self.low:
            warnings.warn(
                f"BarV1 high < low: high={self.high}, low={self.low}, "
                f"bar_time={self.bar_time}",
                stacklevel=2,
            )
