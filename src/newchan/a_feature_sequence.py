"""A 系统 — 特征序列构建与包含处理

从笔序列中提取反向笔作为"特征序列"，并对特征序列做包含处理
生成"标准特征序列"，供线段 v1 使用。

规格引用: docs/chan_spec.md §5.4 v1 特征序列法
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.a_stroke import Stroke


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class FeatureBar:
    """特征序列中的一个元素。

    Attributes
    ----------
    idx : int
        该元素对应的笔在 strokes 列表中的位置索引。
    high : float
        该笔的 high。
    low : float
        该笔的 low。
    """

    idx: int
    high: float
    low: float


# ====================================================================
# 构建特征序列
# ====================================================================

def build_feature_sequence(
    strokes: list[Stroke],
    start_s: int,
    end_s: int,
    direction: Literal["up", "down"],
) -> list[FeatureBar]:
    """从笔序列中提取特征序列（反向笔）。

    Parameters
    ----------
    strokes : list[Stroke]
    start_s, end_s : int
        笔索引范围 [start_s, end_s]（闭区间）。
    direction : ``"up"`` | ``"down"``
        当前线段方向。向上段取 down 笔，向下段取 up 笔。

    Returns
    -------
    list[FeatureBar]
        按 idx 递增排序。
    """
    opposite = "down" if direction == "up" else "up"
    result: list[FeatureBar] = []
    for i in range(start_s, min(end_s + 1, len(strokes))):
        if strokes[i].direction == opposite:
            result.append(FeatureBar(
                idx=i, high=strokes[i].high, low=strokes[i].low,
            ))
    return result


# ====================================================================
# 特征序列包含处理 → 标准特征序列
# ====================================================================

def _merge_feature_bar(
    buf: list[list[float | int]], last: list[float | int],
    curr_h: float, curr_l: float, i: int, seq_idx: int,
    dir_state: str | None,
) -> str | None:
    """处理单个特征序列元素的包含/非包含。返回更新后的 dir_state。"""
    last_h, last_l = last[0], last[1]
    left_inc = last_h >= curr_h and last_l <= curr_l
    right_inc = curr_h >= last_h and curr_l <= last_l

    if left_inc or right_inc:
        effective_up = dir_state != "DOWN"
        if effective_up:
            last[0] = max(last_h, curr_h)
            last[1] = max(last_l, curr_l)
        else:
            last[0] = min(last_h, curr_h)
            last[1] = min(last_l, curr_l)
        last[3] = i
        last[4] = seq_idx
    else:
        if curr_h > last_h and curr_l > last_l:
            dir_state = "UP"
        elif curr_h < last_h and curr_l < last_l:
            dir_state = "DOWN"
        buf.append([curr_h, curr_l, i, i, seq_idx])

    return dir_state


def merge_inclusion_feature(
    seq: list[FeatureBar],
) -> tuple[list[FeatureBar], list[tuple[int, int]]]:
    """对特征序列做包含处理，产出标准特征序列。"""
    n = len(seq)
    if n == 0:
        return [], []
    if n == 1:
        return [seq[0]], [(0, 0)]

    buf: list[list[float | int]] = [
        [seq[0].high, seq[0].low, 0, 0, seq[0].idx]
    ]
    dir_state: str | None = None

    for i in range(1, n):
        dir_state = _merge_feature_bar(
            buf, buf[-1], seq[i].high, seq[i].low, i, seq[i].idx, dir_state,
        )

    merged_seq = [
        FeatureBar(idx=int(row[4]), high=row[0], low=row[1])
        for row in buf
    ]
    merged_to_raw: list[tuple[int, int]] = [
        (int(row[2]), int(row[3])) for row in buf
    ]
    return merged_seq, merged_to_raw
