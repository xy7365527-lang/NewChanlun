"""A 系统 — 分型识别（Fractal Detection）

在包含处理后的 MergedBar 序列上识别顶分型与底分型。

规格引用: docs/chan_spec.md §3 分型（Fractal）

## 拓扑语义（195号）

### 结构映射
分型是一维价格函数的局部极值点——Morse 临界点的离散一维类比。

- merged bar 序列上的价格函数 f: {merged bars} → ℝ 是离散函数
- 顶分型 = 局部极大值点（index-1 临界点的类比）
- 底分型 = 局部极小值点（index-0 临界点的类比）
- 双条件（h_curr > h_prev AND h_curr > h_next AND l_curr > l_prev AND l_curr > l_next）
  = 严格局部极值条件（Morse 非退化条件的离散一维对应：严格不等式 = 非退化）
- fractals_from_merged() 是局部极值点的枚举
- 114号谱系：分型形式化稳定性（双条件保证中心bar=极值bar）

### 映射的边界
这里的"Morse"是连续 Morse 理论的一维类比（一维函数的局部极值 = 临界点），
不是 Forman 离散 Morse 理论的精确实例。Forman 理论需要 CW 复形上的
梯度向量场配对结构，分型检测不具备这个结构。
类比的有效部分：严格不等式条件排除了退化（平台/相等），这和 Morse 非退化条件精神一致。
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
