"""A 系统 — 包含关系处理（缠论地基）

实现 K 线包含关系的识别与合并，这是分型、笔、线段等所有上层结构的前提。

规格引用: docs/chan_spec.md §2 包含关系（Inclusion）
"""

from __future__ import annotations

import pandas as pd
import numpy as np


def merge_inclusion(
    df: pd.DataFrame,
) -> tuple[pd.DataFrame, list[tuple[int, int]]]:
    """对 K 线序列执行包含关系处理。

    Parameters
    ----------
    df : pd.DataFrame
        必须包含 ``open, high, low, close`` 四列，按时间升序排列。
        index 推荐为 DateTimeIndex（会保留到输出）。

    Returns
    -------
    df_merged : pd.DataFrame
        包含处理后的 K 线序列（保留 ``open, high, low, close`` + 原始 index）。
        index 取每组合并 K 线中**最后一根**的原始 index。
    merged_to_raw : list[tuple[int, int]]
        长度与 ``df_merged`` 行数相同。
        ``merged_to_raw[i] = (start_i, end_i)`` 表示第 i 根合并 K 线
        对应原始 ``df`` 中 **位置索引** ``start_i`` 到 ``end_i``（闭区间）。

    Notes
    -----
    合并规则严格遵循 docs/chan_spec.md §2：

    §2.1  包含判定（只看 high/low）
    §2.2  合并方向由 dir ∈ {UP, DOWN} 决定
          - 向上合并: high=max(h1,h2), low=max(l1,l2)
          - 向下合并: high=min(h1,h2), low=min(l1,l2)
          - OHLC 工程处理: open=左K.open, close=右K.close
    §2.3  dir 更新仅在"相邻两根无包含"时发生（双条件）：
          - high_i > high_{i-1} AND low_i > low_{i-1} → dir=UP
          - high_i < high_{i-1} AND low_i < low_{i-1} → dir=DOWN
          - 否则 dir 保持不变
          - dir 初始为 None；dir 为 None 时遇到包含，默认按 UP 合并
    §2.4  先左后右递推，完成后序列无任何相邻包含
    """
    n = len(df)
    if n == 0:
        return df.iloc[:0].copy(), []

    # 提取 numpy 数组提速
    highs = df["high"].values.astype(np.float64)
    lows = df["low"].values.astype(np.float64)
    opens = df["open"].values.astype(np.float64)
    closes = df["close"].values.astype(np.float64)
    idx_labels = df.index  # 保留原始 index（DateTimeIndex 等）

    if n == 1:
        return df.iloc[:1].copy(), [(0, 0)]

    # ── 结果缓冲 ──
    # 每个 merged bar: [open, high, low, close, raw_start, raw_end]
    buf: list[list[float | int]] = [
        [opens[0], highs[0], lows[0], closes[0], 0, 0]
    ]

    # §2.3: dir 初始为 None，直到遇到第一对满足双条件的无包含相邻K线
    dir_state: str | None = None  # None / "UP" / "DOWN"

    for i in range(1, n):
        curr_h, curr_l = highs[i], lows[i]
        last = buf[-1]
        last_h, last_l = last[1], last[2]

        # ── §2.1 包含判定 ──
        left_contains_right = last_h >= curr_h and last_l <= curr_l
        right_contains_left = curr_h >= last_h and curr_l <= last_l
        has_inclusion = left_contains_right or right_contains_left

        if has_inclusion:
            # §2.3: dir 为 None 时默认按 UP 合并
            # （缠论惯例：初始方向未定时以向上处理为安全默认）
            effective_up = dir_state != "DOWN"

            # §2.2 合并
            if effective_up:
                new_h = max(last_h, curr_h)
                new_l = max(last_l, curr_l)
            else:
                new_h = min(last_h, curr_h)
                new_l = min(last_l, curr_l)

            last[1] = new_h         # high
            last[2] = new_l         # low
            last[3] = closes[i]     # close = 右K.close (§2.2 SHOULD)
            last[5] = i             # 扩展 raw_end
        else:
            # §2.3: 相邻两根无包含 → 更新 dir（双条件）
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            # else: 高低一升一降 → dir 保持不变
            #       （经包含处理后的相邻bar理论上不会出现此情况，
            #        但作为防御保留，参见 §2.3 "否则保持 dir 不变"）

            buf.append([opens[i], curr_h, curr_l, closes[i], i, i])

    # ── 构造输出 ──
    merged_to_raw: list[tuple[int, int]] = [
        (int(row[4]), int(row[5])) for row in buf
    ]

    # index 取每组最后一根原始 bar 的 index (§2.2 SHOULD: ts=B.ts)
    out_index = [idx_labels[int(row[5])] for row in buf]

    arr = np.array(
        [[row[0], row[1], row[2], row[3]] for row in buf],
        dtype=np.float64,
    )
    df_merged = pd.DataFrame(
        arr,
        columns=["open", "high", "low", "close"],
        index=out_index,
    )
    df_merged.index.name = df.index.name

    return df_merged, merged_to_raw
