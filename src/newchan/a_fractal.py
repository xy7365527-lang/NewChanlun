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


def fractals_from_merged(df_merged: pd.DataFrame) -> list[Fractal]:
    """在 MergedBar 序列上识别全部分型。

    Parameters
    ----------
    df_merged : pd.DataFrame
        包含处理后的 K 线序列，必须含 ``high, low`` 列。

    Returns
    -------
    list[Fractal]
        按 ``idx`` 递增排序的分型列表。
        **不做去重/过滤**——留给后续笔构造步骤。

    Notes
    -----
    严格按 docs/chan_spec.md §3 双条件判定：

    §3.1 顶分型 Top（双条件）::

        K[i].high > K[i-1].high  AND  K[i].high > K[i+1].high
        AND
        K[i].low  > K[i-1].low   AND  K[i].low  > K[i+1].low

        price = K[i].high

    §3.2 底分型 Bottom（双条件）::

        K[i].low  < K[i-1].low   AND  K[i].low  < K[i+1].low
        AND
        K[i].high < K[i-1].high  AND  K[i].high < K[i+1].high

        price = K[i].low

    §3.3 分型确认需要看到 i+1；第一根和最后一根不可能是分型。
    """
    n = len(df_merged)
    if n < 3:
        return []

    highs = df_merged["high"].values.astype(np.float64)
    lows = df_merged["low"].values.astype(np.float64)

    result: list[Fractal] = []

    for i in range(1, n - 1):
        h_prev, h_curr, h_next = highs[i - 1], highs[i], highs[i + 1]
        l_prev, l_curr, l_next = lows[i - 1], lows[i], lows[i + 1]

        # §3.1 顶分型：双条件
        if (
            h_curr > h_prev
            and h_curr > h_next
            and l_curr > l_prev
            and l_curr > l_next
        ):
            result.append(Fractal(idx=i, kind="top", price=float(h_curr)))

        # §3.2 底分型：双条件
        elif (
            l_curr < l_prev
            and l_curr < l_next
            and h_curr < h_prev
            and h_curr < h_next
        ):
            result.append(Fractal(idx=i, kind="bottom", price=float(l_curr)))

    return result
