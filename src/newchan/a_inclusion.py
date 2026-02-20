"""A 系统 — 包含关系处理（缠论地基）

实现 K 线包含关系的识别与合并，这是分型、笔、线段等所有上层结构的前提。

规格引用: docs/chan_spec.md §2 包含关系（Inclusion）
"""

from __future__ import annotations

import pandas as pd
import numpy as np


def _merge_loop(
    highs: np.ndarray, lows: np.ndarray,
    opens: np.ndarray, closes: np.ndarray,
    n: int,
) -> list[list[float | int]]:
    """执行包含关系合并的主循环，返回 merged bar 缓冲。

    每个 merged bar: [open, high, low, close, raw_start, raw_end]。
    """
    buf: list[list[float | int]] = [
        [opens[0], highs[0], lows[0], closes[0], 0, 0]
    ]
    dir_state: str | None = None  # §2.3: dir 初始为 None

    for i in range(1, n):
        curr_h, curr_l = highs[i], lows[i]
        last = buf[-1]
        last_h, last_l = last[1], last[2]

        has_inclusion = (last_h >= curr_h and last_l <= curr_l) or (
            curr_h >= last_h and curr_l <= last_l
        )

        if has_inclusion:
            effective_up = dir_state != "DOWN"
            if effective_up:
                last[1] = max(last_h, curr_h)
                last[2] = max(last_l, curr_l)
            else:
                last[1] = min(last_h, curr_h)
                last[2] = min(last_l, curr_l)
            last[3] = closes[i]
            last[5] = i
        else:
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            buf.append([opens[i], curr_h, curr_l, closes[i], i, i])

    return buf


def _buf_to_dataframe(
    buf: list[list[float | int]], idx_labels: pd.Index, index_name: object,
) -> tuple[pd.DataFrame, list[tuple[int, int]]]:
    """将 merged bar 缓冲转换为 DataFrame + merged_to_raw 映射。"""
    merged_to_raw: list[tuple[int, int]] = [
        (int(row[4]), int(row[5])) for row in buf
    ]
    out_index = [idx_labels[int(row[5])] for row in buf]
    arr = np.array(
        [[row[0], row[1], row[2], row[3]] for row in buf],
        dtype=np.float64,
    )
    df_merged = pd.DataFrame(arr, columns=["open", "high", "low", "close"], index=out_index)
    df_merged.index.name = index_name
    return df_merged, merged_to_raw


def merge_inclusion(
    df: pd.DataFrame,
) -> tuple[pd.DataFrame, list[tuple[int, int]]]:
    """对 K 线序列执行包含关系处理。

    合并规则严格遵循 docs/chan_spec.md §2（§2.1-§2.4）。
    """
    n = len(df)
    if n == 0:
        return df.iloc[:0].copy(), []
    if n == 1:
        return df.iloc[:1].copy(), [(0, 0)]

    highs = df["high"].values.astype(np.float64)
    lows = df["low"].values.astype(np.float64)
    opens = df["open"].values.astype(np.float64)
    closes = df["close"].values.astype(np.float64)

    buf = _merge_loop(highs, lows, opens, closes, n)
    return _buf_to_dataframe(buf, df.index, df.index.name)
