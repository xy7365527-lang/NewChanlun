"""背驰 / 盘整背驰 — v1 管线。

使用 v1 的 Zhongshu + Move 替代 v0 的 Center + TrendTypeInstance。
Divergence 输出类型复用 v0 的定义（语义不变）。

参考规范
--------
- beichi.md v0.1（生成态）
- maimai_rules_v1.md §13（v0/v1 管线分裂标注）
- 缠论第 24/25 课：趋势背驰/盘整背驰定义

概念溯源标签
------------
- 趋势背驰力度比较 [旧缠论]
- 盘整背驰力度比较 [旧缠论]
- MACD 辅助力度 [旧缠论:隐含]

已知缺失（生成态）
------------------
- T4: MACD 黄白线回拉 0 轴前提检查（beichi.md §T4）→ ✅ 已实现（方案3: B段穿越0轴）
- T6: 黄白线创新高比较（beichi.md §T6）→ ✅ 已实现（dif_peak_for_range 工具函数）
- T7: 柱子伸长高度比较（beichi.md §T7）→ ✅ 已实现（histogram_peak_for_range 工具函数）
- T6/T7 与 T5(面积) 的组合方式：未结算（beichi.md #2 "或 vs 且"问题）
"""

from __future__ import annotations

import logging
from typing import Literal

import pandas as pd

from newchan.a_divergence import Divergence
from newchan.a_macd import macd_area_for_range
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu

logger = logging.getLogger(__name__)


# ── 力度计算（从 a_divergence.py 独立复制，避免跨模块导入私有函数） ──

def _seg_merged_range(segments: list, start: int, end: int) -> tuple[int, int]:
    """获取 segments[start:end+1] 覆盖的 merged bar 索引范围。"""
    if start > end or start < 0 or end >= len(segments):
        return (0, 0)
    i0 = segments[start].i0
    i1 = segments[end].i1
    return (i0, i1)


def _compute_force(
    segments: list,
    seg_start: int,
    seg_end: int,
    trend_direction: str,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> float:
    """计算一段 segments 的力度。

    有 MACD 数据时用 MACD 面积，否则用价格振幅 x 持续长度。
    """
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(segments):
        return 0.0

    i0, i1 = _seg_merged_range(segments, seg_start, seg_end)

    if df_macd is not None and merged_to_raw is not None:
        raw_i0 = merged_to_raw[i0][0] if i0 < len(merged_to_raw) else 0
        raw_i1 = merged_to_raw[i1][1] if i1 < len(merged_to_raw) else 0
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)
        if trend_direction == "up":
            return abs(area["area_pos"])
        else:
            return abs(area["area_neg"])

    # Fallback：价格振幅 x 持续 bar 数
    high = max(segments[k].high for k in range(seg_start, seg_end + 1))
    low = min(segments[k].low for k in range(seg_start, seg_end + 1))
    duration = max(1, i1 - i0)
    return (high - low) * duration


# ── T4: B 段 MACD 黄白线穿越 0 轴检测 ──

def _b_segment_crosses_zero(
    segments: list,
    zs_last: Zhongshu,
    df_macd: pd.DataFrame,
    merged_to_raw: list[tuple[int, int]],
) -> bool:
    """检查 B 段（最后中枢）覆盖的 raw bar 范围内，MACD 黄白线（DIF）是否穿越 0 轴。

    方案3（无阈值）：穿越 = 同时存在 strictly positive 和 strictly negative 值。
    0 值不算穿越（恰好在 0 轴上不算回拉到另一侧）。

    原文依据（第25课）：
    > "背驰需要多少个条件？光柱子缩短就背驰？前面的黄白线有回拉0轴吗？"
    > "进入第二个中枢的形成过程中，同时MACD的黄白线会逐步回到0轴附近"

    概念溯源: [旧缠论] — 第24/25课MACD标准背驰前提
    """
    # B 段的 merged bar 范围
    i0 = segments[zs_last.seg_start].i0 if zs_last.seg_start < len(segments) else 0
    i1 = segments[zs_last.seg_end].i1 if zs_last.seg_end < len(segments) else 0

    # merged → raw
    raw_i0 = merged_to_raw[i0][0] if i0 < len(merged_to_raw) else 0
    raw_i1 = merged_to_raw[i1][1] if i1 < len(merged_to_raw) else 0

    if raw_i0 < 0:
        raw_i0 = 0
    if raw_i1 >= len(df_macd):
        raw_i1 = len(df_macd) - 1
    if raw_i0 > raw_i1:
        return False

    macd_line = df_macd["macd"].iloc[raw_i0 : raw_i1 + 1]
    has_positive = bool((macd_line > 0).any())
    has_negative = bool((macd_line < 0).any())

    return has_positive and has_negative


# ── T6: DIF 峰值比较（黄白线创新高） ──


def dif_peak_for_range(
    df_macd: pd.DataFrame,
    raw_i0: int,
    raw_i1: int,
    trend_direction: str,
) -> float:
    """计算指定 raw bar 范围内 DIF（黄白线）的峰值。

    原文依据（第25课）：
    > "黄白线不能创新高"

    Parameters
    ----------
    df_macd : pd.DataFrame
        由 compute_macd() 返回，含 'macd' 列（= DIF）。
    raw_i0, raw_i1 : int
        原始 bar 索引范围 [raw_i0, raw_i1]（闭区间）。
    trend_direction : str
        "up" → 取 max(DIF)；"down" → 取 abs(min(DIF))。

    Returns
    -------
    float
        峰值（≥0）。无效范围返回 0.0。

    概念溯源: [旧缠论] — 第25课 MACD 三维度之一
    """
    if raw_i0 > raw_i1 or raw_i0 < 0 or raw_i1 >= len(df_macd):
        return 0.0
    dif = df_macd["macd"].iloc[raw_i0 : raw_i1 + 1]
    if len(dif) == 0:
        return 0.0
    if trend_direction == "up":
        return max(0.0, float(dif.max()))
    else:
        return abs(min(0.0, float(dif.min())))


# ── T7: 柱子伸长高度比较 ──


def histogram_peak_for_range(
    df_macd: pd.DataFrame,
    raw_i0: int,
    raw_i1: int,
    trend_direction: str,
) -> float:
    """计算指定 raw bar 范围内 MACD 柱子（hist）的最大伸长。

    原文依据（第25课）：
    > "柱子的面积或者伸长的高度不能突破新高"

    Parameters
    ----------
    df_macd : pd.DataFrame
        由 compute_macd() 返回，含 'hist' 列。
    raw_i0, raw_i1 : int
        原始 bar 索引范围 [raw_i0, raw_i1]（闭区间）。
    trend_direction : str
        "up" → 取 max(hist)（红柱子最高）；"down" → 取 abs(min(hist))（绿柱子最长）。

    Returns
    -------
    float
        峰值（≥0）。无效范围返回 0.0。

    概念溯源: [旧缠论] — 第25课 MACD 三维度之一
    """
    if raw_i0 > raw_i1 or raw_i0 < 0 or raw_i1 >= len(df_macd):
        return 0.0
    hist = df_macd["hist"].iloc[raw_i0 : raw_i1 + 1]
    if len(hist) == 0:
        return 0.0
    if trend_direction == "up":
        return max(0.0, float(hist.max()))
    else:
        return abs(min(0.0, float(hist.min())))


# ── 趋势背驰检测 ──

def _detect_trend_divergence(
    segments: list,
    zhongshus: list[Zhongshu],
    move: Move,
    level_id: int,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> Divergence | None:
    """检测单个趋势 Move 中的背驰。

    算法：
    1. move.kind == "trend" 且 zs_count >= 2
    2. 取 move 范围内最后两个 settled 中枢
    3. A 段 = 前中枢结束到后中枢开始
    4. C 段 = 后中枢结束到 move 终点
    5. T4 前提：B 段黄白线穿越 0 轴（仅有 MACD 时）
    6. 三维度 OR 判定（beichi.md #2 已结算）：
       - T2: force_c < force_a（面积）
       - T6: dif_peak_c < dif_peak_a（DIF 峰值）
       - T7: hist_peak_c < hist_peak_a（HIST 峰值）
       任一满足 → 背驰 [旧缠论:选择]
    """
    if move.kind != "trend" or move.zs_count < 2:
        return None

    # 收集属于此 Move 的 settled 中枢
    move_zs_indices: list[int] = []
    for i in range(move.zs_start, min(move.zs_end + 1, len(zhongshus))):
        if zhongshus[i].settled:
            move_zs_indices.append(i)

    if len(move_zs_indices) < 2:
        return None

    idx_prev = move_zs_indices[-2]
    idx_last = move_zs_indices[-1]
    zs_prev = zhongshus[idx_prev]
    zs_last = zhongshus[idx_last]

    # A 段：从前中枢结束到后中枢开始
    a_start = zs_prev.seg_end + 1
    a_end = zs_last.seg_start - 1
    if a_start > a_end:
        # 两中枢紧邻，退化为前中枢最后段
        a_start = zs_prev.seg_end
        a_end = zs_prev.seg_end

    # C 段：从后中枢结束到 Move 终点
    c_start = zs_last.seg_end + 1
    c_end = move.seg_end
    if c_start > c_end:
        return None  # C 段尚未形成

    # 边界检查
    if a_start >= len(segments) or c_end >= len(segments):
        return None

    # T4 前提：B 段（最后中枢）MACD 黄白线穿越 0 轴（beichi.md §T4）
    # 仅在有 MACD 数据时检查；无 MACD 数据时走 fallback 不检查 T4
    if df_macd is not None and merged_to_raw is not None:
        if not _b_segment_crosses_zero(segments, zs_last, df_macd, merged_to_raw):
            logger.debug(
                "T4 前提不满足: B 段 (zs[%d]) MACD 黄白线未穿越 0 轴，跳过趋势背驰检测",
                idx_last,
            )
            return None

    # T2: 面积维度
    force_a = _compute_force(segments, a_start, a_end, move.direction,
                             df_macd, merged_to_raw)
    force_c = _compute_force(segments, c_start, c_end, move.direction,
                             df_macd, merged_to_raw)

    if force_a <= 0:
        return None

    # T6/T7: DIF 峰值 + HIST 峰值维度（仅有 MACD 时）
    dif_peak_a = 0.0
    dif_peak_c = 0.0
    hist_peak_a = 0.0
    hist_peak_c = 0.0

    if df_macd is not None and merged_to_raw is not None:
        a_i0, a_i1 = _seg_merged_range(segments, a_start, a_end)
        raw_a_i0 = merged_to_raw[a_i0][0] if a_i0 < len(merged_to_raw) else 0
        raw_a_i1 = merged_to_raw[a_i1][1] if a_i1 < len(merged_to_raw) else 0

        c_i0, c_i1 = _seg_merged_range(segments, c_start, c_end)
        raw_c_i0 = merged_to_raw[c_i0][0] if c_i0 < len(merged_to_raw) else 0
        raw_c_i1 = merged_to_raw[c_i1][1] if c_i1 < len(merged_to_raw) else 0

        dif_peak_a = dif_peak_for_range(df_macd, raw_a_i0, raw_a_i1, move.direction)
        dif_peak_c = dif_peak_for_range(df_macd, raw_c_i0, raw_c_i1, move.direction)
        hist_peak_a = histogram_peak_for_range(df_macd, raw_a_i0, raw_a_i1, move.direction)
        hist_peak_c = histogram_peak_for_range(df_macd, raw_c_i0, raw_c_i1, move.direction)

    # 三维度 OR 判定（beichi.md #2 结算：任一满足即背驰）[旧缠论:选择]
    t2_diverged = force_c < force_a
    t6_diverged = dif_peak_a > 0 and dif_peak_c < dif_peak_a
    t7_diverged = hist_peak_a > 0 and hist_peak_c < hist_peak_a

    if t2_diverged or t6_diverged or t7_diverged:
        div_dir: Literal["top", "bottom"] = (
            "top" if move.direction == "up" else "bottom"
        )
        return Divergence(
            kind="trend",
            direction=div_dir,
            level_id=level_id,
            seg_a_start=a_start,
            seg_a_end=a_end,
            seg_c_start=c_start,
            seg_c_end=c_end,
            center_idx=idx_last,
            force_a=force_a,
            force_c=force_c,
            confirmed=move.settled,
            dif_peak_a=dif_peak_a,
            dif_peak_c=dif_peak_c,
            hist_peak_a=hist_peak_a,
            hist_peak_c=hist_peak_c,
        )

    return None


# ── 盘整背驰检测 ──

def _detect_consolidation_divergence(
    segments: list,
    zhongshus: list[Zhongshu],
    move: Move,
    level_id: int,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> Divergence | None:
    """检测单个盘整 Move 中的盘整背驰。

    算法：
    1. move.kind == "consolidation" 且 zs_count >= 1
    2. 取中枢
    3. 收集离开中枢的段（按方向分组）
    4. 对每个方向，比较最后两次离开力度
    5. 三维度 OR 判定（beichi.md #2 已结算）：
       - T2: force_c < force_a（面积）
       - T6: dif_peak_c < dif_peak_a（DIF 峰值）
       - T7: hist_peak_c < hist_peak_a（HIST 峰值）
       任一满足 → 盘整背驰 [旧缠论:选择]
    """
    if move.kind != "consolidation" or move.zs_count < 1:
        return None

    # 取第一个中枢
    if move.zs_start >= len(zhongshus):
        return None
    zs = zhongshus[move.zs_start]

    # 收集离开段（超出 [ZD, ZG] 范围的段）
    exit_segs_by_dir: dict[str, list[int]] = {"up": [], "down": []}

    # 中枢内的段
    for k in range(zs.seg_start, min(zs.seg_end + 1, len(segments))):
        seg = segments[k]
        if seg.high > zs.zg or seg.low < zs.zd:
            d = seg.direction
            if d in exit_segs_by_dir:
                exit_segs_by_dir[d].append(k)

    # 中枢之后到 Move 终点
    for k in range(zs.seg_end + 1, min(move.seg_end + 1, len(segments))):
        seg = segments[k]
        d = seg.direction
        if d in exit_segs_by_dir:
            exit_segs_by_dir[d].append(k)

    # 每个方向比较最后两次离开
    for direction, exits in exit_segs_by_dir.items():
        if len(exits) < 2:
            continue

        a_idx = exits[-2]
        c_idx = exits[-1]

        # T2: 面积维度
        force_a = _compute_force(segments, a_idx, a_idx, direction,
                                 df_macd, merged_to_raw)
        force_c = _compute_force(segments, c_idx, c_idx, direction,
                                 df_macd, merged_to_raw)

        if force_a <= 0:
            continue

        # T6/T7: DIF 峰值 + HIST 峰值维度
        dif_peak_a = 0.0
        dif_peak_c = 0.0
        hist_peak_a = 0.0
        hist_peak_c = 0.0

        if df_macd is not None and merged_to_raw is not None:
            a_i0, a_i1 = _seg_merged_range(segments, a_idx, a_idx)
            raw_a_i0 = merged_to_raw[a_i0][0] if a_i0 < len(merged_to_raw) else 0
            raw_a_i1 = merged_to_raw[a_i1][1] if a_i1 < len(merged_to_raw) else 0

            c_i0, c_i1 = _seg_merged_range(segments, c_idx, c_idx)
            raw_c_i0 = merged_to_raw[c_i0][0] if c_i0 < len(merged_to_raw) else 0
            raw_c_i1 = merged_to_raw[c_i1][1] if c_i1 < len(merged_to_raw) else 0

            dif_peak_a = dif_peak_for_range(df_macd, raw_a_i0, raw_a_i1, direction)
            dif_peak_c = dif_peak_for_range(df_macd, raw_c_i0, raw_c_i1, direction)
            hist_peak_a = histogram_peak_for_range(df_macd, raw_a_i0, raw_a_i1, direction)
            hist_peak_c = histogram_peak_for_range(df_macd, raw_c_i0, raw_c_i1, direction)

        # 三维度 OR 判定 [旧缠论:选择]
        t2_diverged = force_c < force_a
        t6_diverged = dif_peak_a > 0 and dif_peak_c < dif_peak_a
        t7_diverged = hist_peak_a > 0 and hist_peak_c < hist_peak_a

        if t2_diverged or t6_diverged or t7_diverged:
            div_dir: Literal["top", "bottom"] = (
                "top" if direction == "up" else "bottom"
            )
            return Divergence(
                kind="consolidation",
                direction=div_dir,
                level_id=level_id,
                seg_a_start=a_idx,
                seg_a_end=a_idx,
                seg_c_start=c_idx,
                seg_c_end=c_idx,
                center_idx=move.zs_start,
                force_a=force_a,
                force_c=force_c,
                confirmed=move.settled,
                dif_peak_a=dif_peak_a,
                dif_peak_c=dif_peak_c,
                hist_peak_a=hist_peak_a,
                hist_peak_c=hist_peak_c,
            )

    return None


# ── 入口函数 ──

def divergences_from_moves_v1(
    segments: list,
    zhongshus: list[Zhongshu],
    moves: list[Move],
    level_id: int,
    *,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[Divergence]:
    """检测 v1 管线中所有 Move 的背驰。

    Parameters
    ----------
    segments : list
        线段列表（需有 i0, i1, high, low, direction 属性）。
    zhongshus : list[Zhongshu]
        v1 中枢列表。
    moves : list[Move]
        v1 走势类型列表。
    level_id : int
        递归层级。
    df_macd : pd.DataFrame | None
        MACD 数据。None 则用价格振幅 fallback。
    merged_to_raw : list[tuple[int, int]] | None
        merged → raw 索引映射。

    Returns
    -------
    list[Divergence]
        检测到的背驰列表。
    """
    result: list[Divergence] = []

    for move in moves:
        # 趋势背驰
        div = _detect_trend_divergence(
            segments, zhongshus, move, level_id, df_macd, merged_to_raw,
        )
        if div is not None:
            result.append(div)
            continue  # 一个 Move 最多报告一个背驰

        # 盘整背驰
        div = _detect_consolidation_divergence(
            segments, zhongshus, move, level_id, df_macd, merged_to_raw,
        )
        if div is not None:
            result.append(div)

    return result
