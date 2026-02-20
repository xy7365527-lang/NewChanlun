"""A 系统 — 分型识别（Fractal Detection）

在包含处理后的 MergedBar 序列上识别顶分型与底分型。

规格引用: docs/chan_spec.md §3 分型（Fractal）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import numpy as np
import pandas as pd


@dataclass(frozen=True, slots=True)
class Fractal:
    """一个分型。

    Attributes
    ----------
    idx : int
        分型中间 K 线在 df_merged 中的**位置索引**（0-based，即 iloc 序号）。
    kind : ``"top"`` | ``"bottom"``
        分型类型。
    price : float
        分型极值。顶分型 = ``K[idx].high``；底分型 = ``K[idx].low``。
    """

    idx: int
    kind: Literal["top", "bottom"]
    price: float


def _classify_fractal(
    h_prev: float, h_curr: float, h_next: float,
    l_prev: float, l_curr: float, l_next: float,
    idx: int,
) -> Fractal | None:
    """对单个位置做双条件分型判定，返回 Fractal 或 None。"""
    if (
        h_curr > h_prev and h_curr > h_next
        and l_curr > l_prev and l_curr > l_next
    ):
        return Fractal(idx=idx, kind="top", price=float(h_curr))
    if (
        l_curr < l_prev and l_curr < l_next
        and h_curr < h_prev and h_curr < h_next
    ):
        return Fractal(idx=idx, kind="bottom", price=float(l_curr))
    return None


def fractals_from_merged(df_merged: pd.DataFrame) -> list[Fractal]:
    """在 MergedBar 序列上识别全部分型（双条件，docs/chan_spec.md §3）。"""
    n = len(df_merged)
    if n < 3:
        return []

    highs = df_merged["high"].values.astype(np.float64)
    lows = df_merged["low"].values.astype(np.float64)

    result: list[Fractal] = []
    for i in range(1, n - 1):
        f = _classify_fractal(
            highs[i - 1], highs[i], highs[i + 1],
            lows[i - 1], lows[i], lows[i + 1],
            idx=i,
        )
        if f is not None:
            result.append(f)

    return result
