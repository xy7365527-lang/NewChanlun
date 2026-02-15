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
# §4 主函数：构造笔
# ====================================================================

def strokes_from_fractals(
    df_merged: pd.DataFrame,
    fractals: list[Fractal],
    mode: str = "wide",
    min_strict_sep: int = 5,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[Stroke]:
    """从分型序列构造笔（保证笔首尾相连，完全覆盖走势）。

    Parameters
    ----------
    df_merged : pd.DataFrame
        包含处理后的 K 线，必须含 ``high, low`` 列。
    fractals : list[Fractal]
        由 ``fractals_from_merged`` 返回的原始分型列表。
    mode : ``"wide"`` | ``"strict"`` | ``"new"``
        宽笔 / 严笔 / 新笔。
    min_strict_sep : int
        严笔模式下两分型中心 idx 的最小间距（默认 5）。
    merged_to_raw : list[tuple[int, int]] | None
        ``merge_inclusion()`` 返回的映射表。``mode="new"`` 时必需。
        ``merged_to_raw[i] = (raw_start, raw_end)`` 表示第 i 根 merged bar
        对应的原始K线位置范围（闭区间）。若未提供且 mode="new"，回退到旧笔。

    Returns
    -------
    list[Stroke]
        按 ``i0`` 递增排序。最后一笔 ``confirmed=False``，其余 ``True``。
        **连续性保证**: ``strokes[i].i1 == strokes[i+1].i0`` 对所有相邻笔成立。

    Notes
    -----
    构造流程（docs/chan_spec.md §4）：

    1. §4.2 分型去重 + 顶底交替
    2. 逐对相邻分型尝试构笔，保持连续性：
       - bottom→top = up，top→bottom = down
       - §4.3 gap 检查：wide ``idx2-idx1 >= 4``，strict ``>= min_strict_sep``，
         new = 原始K线间距 >= 3（《忽闻台风可休市》新笔定义）
       - §4.4 有效性：up 时 end.price > start.price；down 时 end.price < start.price
       - **锁定规则**：已成笔的终点 start 不可被同类分型替换；
         遇到同类更极端分型时，延伸上一笔（更新 i1/p1），确保连续
    3. §4.5 最后一笔 ``confirmed=False``
    """
    # ── Step 1: 去重 + 交替 ──
    fxs = enforce_alternation(dedupe_fractals(fractals))
    if len(fxs) < 2:
        return []

    use_new_bi = mode == "new" and merged_to_raw is not None
    min_gap = 4 if mode in ("wide", "new") else min_strict_sep
    highs = df_merged["high"].values.astype(np.float64)
    lows = df_merged["low"].values.astype(np.float64)

    # ── Step 2: 逐对构笔（保持连续性） ──
    strokes: list[Stroke] = []
    # i = 当前 start 在 fxs 中的索引
    i = 0
    j = 1

    while j < len(fxs):
        start = fxs[i]
        cand = fxs[j]
        locked = len(strokes) > 0  # start 是否为已提交笔的终点

        # ── 同类分型处理 ──
        if cand.kind == start.kind:
            is_more_extreme = (
                (cand.kind == "top" and cand.price > start.price)
                or (cand.kind == "bottom" and cand.price < start.price)
            )
            if is_more_extreme:
                if locked:
                    # start 已锁定 → 延伸上一笔至新的更极端分型
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
                    i = j  # start 移至延伸后的新端点
                else:
                    # 未锁定 → 自由更新 start 至更极端分型
                    i = j
            # 非更极端 → 跳过（保留已有的更好 start）
            j += 1
            continue

        # ── 异类分型 → 尝试成笔 ──
        # §4.3 gap 检查
        merged_gap = cand.idx - start.idx
        if use_new_bi:
            # 新笔：在原始K线上计数（《忽闻台风可休市》"不考虑包含关系"）
            # 条件1：分型不共用K线 → merged_gap >= 2（最低门槛）
            # 条件2：极值K线之间原始K线数 >= 3
            raw_gap = merged_to_raw[cand.idx][0] - merged_to_raw[start.idx][1] - 1  # type: ignore[index]
            gap_ok = merged_gap >= 2 and raw_gap >= 3
        else:
            gap_ok = merged_gap >= min_gap
        if not gap_ok:
            j += 1
            continue

        # §4.4 方向与有效性（使用分型 price，而非 merged bar 值）
        if start.kind == "bottom" and cand.kind == "top":
            direction: Literal["up", "down"] = "up"
            valid = cand.price > start.price
        else:  # top → bottom
            direction = "down"
            valid = cand.price < start.price

        if not valid:
            j += 1
            continue

        # ── 构造 stroke ──
        i0, i1 = start.idx, cand.idx
        seg_high = float(highs[i0 : i1 + 1].max())
        seg_low = float(lows[i0 : i1 + 1].min())

        strokes.append(Stroke(
            i0=i0, i1=i1, direction=direction,
            high=seg_high, low=seg_low,
            p0=start.price, p1=cand.price,
            confirmed=True,
        ))
        i = j   # 下一笔的 start = 本笔的 end
        j += 1

    # ── Step 3: 最后一笔标记为未确认 ──
    if strokes:
        last = strokes[-1]
        strokes[-1] = Stroke(
            i0=last.i0, i1=last.i1, direction=last.direction,
            high=last.high, low=last.low,
            p0=last.p0, p1=last.p1,
            confirmed=False,
        )

    return strokes
