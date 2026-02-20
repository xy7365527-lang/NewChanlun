"""A 系统 — 笔构造（Stroke Construction）

从分型序列构造笔：分型去重、顶底交替、宽/严笔参数化、确认语义。

规格引用: docs/chan_spec.md §4 笔（Stroke）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import numpy as np
import pandas as pd

from newchan.a_fractal import Fractal


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class Stroke:
    """一笔。

    Attributes
    ----------
    i0 : int
        起点分型中心 idx（df_merged iloc 位置索引）。
    i1 : int
        终点分型中心 idx。
    direction : ``"up"`` | ``"down"``
        向上笔（底→顶）或向下笔（顶→底）。
    high : float
        i0..i1 区间内 df_merged['high'] 的最大值。
    low : float
        i0..i1 区间内 df_merged['low'] 的最小值。
    p0 : float
        起点分型极值价（底分型→low，顶分型→high）。
    p1 : float
        终点分型极值价（顶分型→high，底分型→low）。
    confirmed : bool
        是否已确认。最后一笔默认 ``False``（延伸中），
        新一笔生成后前一笔才变为 ``True``。
    """

    i0: int
    i1: int
    direction: Literal["up", "down"]
    high: float
    low: float
    p0: float
    p1: float
    confirmed: bool


# ====================================================================
# §4.2 分型去重（择优）
# ====================================================================

def dedupe_fractals(fractals: list[Fractal]) -> list[Fractal]:
    """相邻同类分型只保留更极端者。

    规格引用: docs/chan_spec.md §4.2

    - top-top  → 保留 price（high）更高者
    - bottom-bottom → 保留 price（low）更低者
    - 输出保证同类不相邻
    """
    if not fractals:
        return []
    result: list[Fractal] = [fractals[0]]
    for fx in fractals[1:]:
        if fx.kind == result[-1].kind:
            # 同类：保留更极端
            if fx.kind == "top" and fx.price > result[-1].price:
                result[-1] = fx
            elif fx.kind == "bottom" and fx.price < result[-1].price:
                result[-1] = fx
            # 否则丢弃 fx，保留已有的更极端值
        else:
            result.append(fx)
    return result


# ====================================================================
# §4.2 顶底交替强制
# ====================================================================

def enforce_alternation(fractals: list[Fractal]) -> list[Fractal]:
    """保证分型序列严格 top/bottom 交替。

    规格引用: docs/chan_spec.md §4.2

    若 dedupe 后仍有同类相邻（理论上不会，但作为安全网），
    继续丢弃不极端者。保留顺序不回退。
    """
    if not fractals:
        return []
    result: list[Fractal] = [fractals[0]]
    for fx in fractals[1:]:
        if fx.kind == result[-1].kind:
            if fx.kind == "top" and fx.price > result[-1].price:
                result[-1] = fx
            elif fx.kind == "bottom" and fx.price < result[-1].price:
                result[-1] = fx
        else:
            result.append(fx)
    return result


# ====================================================================
# §4 内部辅助
# ====================================================================

def _is_more_extreme(cand: Fractal, start: Fractal) -> bool:
    """同类分型中 cand 是否比 start 更极端。"""
    return (
        (cand.kind == "top" and cand.price > start.price)
        or (cand.kind == "bottom" and cand.price < start.price)
    )


def _extend_prev_stroke(
    strokes: list[Stroke],
    cand: Fractal,
    highs: np.ndarray,
    lows: np.ndarray,
) -> None:
    """锁定态：延伸上一笔至更极端的同类分型（原地替换列表尾元素）。"""
    prev = strokes[-1]
    new_i1 = cand.idx
    seg_high = float(highs[prev.i0 : new_i1 + 1].max())
    seg_low = float(lows[prev.i0 : new_i1 + 1].min())
    strokes[-1] = Stroke(
        i0=prev.i0, i1=new_i1,
        direction=prev.direction,
        high=seg_high, low=seg_low,
        p0=prev.p0, p1=cand.price,
        confirmed=prev.confirmed,
    )


def _check_gap(
    start: Fractal,
    cand: Fractal,
    use_new_bi: bool,
    min_gap: int,
    merged_to_raw: list[tuple[int, int]] | None,
) -> bool:
    """§4.3 gap 检查：宽笔/严笔/新笔三种模式。"""
    merged_gap = cand.idx - start.idx
    if use_new_bi:
        raw_gap = merged_to_raw[cand.idx][0] - merged_to_raw[start.idx][1] - 1  # type: ignore[index]
        return merged_gap >= 2 and raw_gap >= 3
    return merged_gap >= min_gap


def _validate_direction(
    start: Fractal, cand: Fractal,
) -> tuple[Literal["up", "down"], bool] | None:
    """§4.4 方向与有效性。返回 (direction, valid) 或 None（不应发生）。"""
    if start.kind == "bottom" and cand.kind == "top":
        return "up", cand.price > start.price
    return "down", cand.price < start.price


def _build_stroke(
    start: Fractal,
    cand: Fractal,
    direction: Literal["up", "down"],
    highs: np.ndarray,
    lows: np.ndarray,
) -> Stroke:
    """从一对异类分型构造一笔。"""
    i0, i1 = start.idx, cand.idx
    return Stroke(
        i0=i0, i1=i1, direction=direction,
        high=float(highs[i0 : i1 + 1].max()),
        low=float(lows[i0 : i1 + 1].min()),
        p0=start.price, p1=cand.price,
        confirmed=True,
    )


def _mark_last_unconfirmed(strokes: list[Stroke]) -> list[Stroke]:
    """§4.5 最后一笔标记为未确认（返回新列表）。"""
    if not strokes:
        return strokes
    last = strokes[-1]
    return strokes[:-1] + [Stroke(
        i0=last.i0, i1=last.i1, direction=last.direction,
        high=last.high, low=last.low,
        p0=last.p0, p1=last.p1,
        confirmed=False,
    )]


# ====================================================================
# §4 主函数：构造笔
# ====================================================================

def strokes_from_fractals(
    df_merged: pd.DataFrame,
    fractals: list[Fractal],
    mode: str = "wide",
    min_strict_sep: int = 5,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[Stroke]:
    """从分型序列构造笔。最后一笔 confirmed=False，连续性保证 strokes[i].i1 == strokes[i+1].i0。"""
    fxs = enforce_alternation(dedupe_fractals(fractals))
    if len(fxs) < 2:
        return []

    use_new_bi = mode == "new" and merged_to_raw is not None
    min_gap = 4 if mode in ("wide", "new") else min_strict_sep
    highs = df_merged["high"].values.astype(np.float64)
    lows = df_merged["low"].values.astype(np.float64)

    strokes: list[Stroke] = []
    i, j = 0, 1
    while j < len(fxs):
        start, cand = fxs[i], fxs[j]
        locked = len(strokes) > 0

        if cand.kind == start.kind:
            if _is_more_extreme(cand, start):
                if locked:
                    _extend_prev_stroke(strokes, cand, highs, lows)
                i = j
            j += 1
            continue

        if not _check_gap(start, cand, use_new_bi, min_gap, merged_to_raw):
            j += 1
            continue

        direction, valid = _validate_direction(start, cand)  # type: ignore[misc]
        if not valid:
            j += 1
            continue

        strokes.append(_build_stroke(start, cand, direction, highs, lows))
        i = j
        j += 1

    return _mark_last_unconfirmed(strokes)
