"""A 系统 — 包含关系处理（缠论地基）

实现 K 线包含关系的识别与合并，这是分型、笔、线段等所有上层结构的前提。

规格引用: docs/chan_spec.md §2 包含关系（Inclusion）

## 拓扑语义（195号）

### 结构映射
包含处理是带邻接约束的区间包含等价类的商空间构造。

- raw K线序列 X 上的偏序关系 ≤ 由"包含"定义：a ≤ b ⇔ [a.low, a.high] ⊆ [b.low, b.high]
- 等价关系 ~：x ~ y ⇔ x 与 y 有包含关系且相邻（传递闭包后形成等价类）
- merge_inclusion() 是商映射 π: X → X/~，将每个等价类映射到一个 merged bar
- merged_to_raw 是商映射的纤维结构：记录每个 merged bar 对应的 raw 范围
- 方向状态 dir_state 决定"向上取并/向下取交"——等价类代表元的极值选择规则

### 映射的边界
商映射 π 是精确的——merge_inclusion 确实计算了等价类的代表元。
但不应称 Alexandrov 拓扑——Alexandrov 拓扑在有限离散序列上是平凡的（每个点既开又闭）。
真正的拓扑内容在于商映射的纤维结构（merged_to_raw = π 的逐纤维分解），不在开集。
"""

from __future__ import annotations

import pandas as pd
import numpy as np


def _is_fractal_pattern(buf: list[list[float | int]], length: int) -> bool:
    """检测 buf 末尾三根 merged bar 是否形成分型模式。

    顶分型：中间 bar 的 high 高于左右两侧且 low 也高于左右两侧。
    底分型：中间 bar 的 low 低于左右两侧且 high 也低于左右两侧。
    """
    if length < 3:
        return False
    h_prev, l_prev = buf[length - 3][1], buf[length - 3][2]
    h_curr, l_curr = buf[length - 2][1], buf[length - 2][2]
    h_next, l_next = buf[length - 1][1], buf[length - 1][2]
    is_top = (
        h_curr > h_prev and h_curr > h_next
        and l_curr > l_prev and l_curr > l_next
    )
    is_bottom = (
        l_curr < l_prev and l_curr < l_next
        and h_curr < h_prev and h_curr < h_next
    )
    return is_top or is_bottom


def _merge_loop(
    highs: np.ndarray, lows: np.ndarray,
    opens: np.ndarray, closes: np.ndarray,
    n: int,
    *,
    reset_dir_on_fractal: bool = False,
) -> list[list[float | int]]:
    """执行包含关系合并的主循环，返回 merged bar 缓冲。

    每个 merged bar: [open, high, low, close, raw_start, raw_end]。

    Parameters
    ----------
    reset_dir_on_fractal : bool
        为 True 时，包含方向在分型形成后重置，且在方向翻转时也重置。
        这使后续包含处理不受之前长趋势方向的累积锁定，
        适用于大振幅标的（如金油比）。
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
            if dir_state is not None:
                effective_up = dir_state == "UP"
            else:
                # dir 只能由无包含相邻 K 线确认；未确认前沿用既有默认 UP 语义。
                effective_up = True
            if effective_up:
                last[1] = max(last_h, curr_h)
                last[2] = max(last_l, curr_l)
            else:
                last[1] = min(last_h, curr_h)
                last[2] = min(last_l, curr_l)
            last[3] = closes[i]
            last[5] = i
        else:
            prev_dir = dir_state
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            buf.append([opens[i], curr_h, curr_l, closes[i], i, i])

            if reset_dir_on_fractal:
                # 方向翻转时重置：让下一次包含用默认 UP 处理，
                # 避免长趋势方向锁定掩盖反转信号
                if prev_dir is not None and dir_state != prev_dir:
                    dir_state = None
                # 分型形成后重置
                elif _is_fractal_pattern(buf, len(buf)):
                    dir_state = None

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
    *,
    reset_dir_on_fractal: bool = False,
) -> tuple[pd.DataFrame, list[tuple[int, int]]]:
    """对 K 线序列执行包含关系处理。

    合并规则严格遵循 docs/chan_spec.md §2（§2.1-§2.4）。

    Parameters
    ----------
    reset_dir_on_fractal : bool
        为 True 时，每当 merged bar 序列形成分型模式，重置包含方向。
        默认 False，保持原有行为。
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

    buf = _merge_loop(
        highs, lows, opens, closes, n,
        reset_dir_on_fractal=reset_dir_on_fractal,
    )
    return _buf_to_dataframe(buf, df.index, df.index.name)
