"""A 系统 — 背驰 / 盘整背驰判定

理论依据（§9 力度、背驰、盘整背驰）：
  - 背驰：趋势中 C 段力度 < A 段力度（力量衰竭）
  - 盘整背驰：盘整中，当下同向离开段力度 < 前一次同向离开段力度
  - 背驰-买卖点定理：任一背驰都必然制造某级别买卖点

MACD 辅助判断（§9.3）：
  - A、B、C 三段：A/C 同向趋势段，B 中间连接（中枢/盘整）
  - C 段 MACD 柱子面积 < A 段 → 标准背驰
  - 上涨看 area_pos，下跌看 |area_neg|

规格引用: 缠论知识库 §9
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Literal

import pandas as pd

from newchan.a_center_v0 import Center
from newchan.a_macd import macd_area_for_range
from newchan.a_trendtype_v0 import TrendTypeInstance

logger = logging.getLogger(__name__)


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class Divergence:
    """背驰/盘整背驰判定结果。

    Attributes
    ----------
    kind : ``"trend"`` | ``"consolidation"``
        趋势背驰 / 盘整背驰。
    direction : ``"top"`` | ``"bottom"``
        顶背驰（上涨力竭）/ 底背驰（下跌力竭）。
    level_id : int
        所属递归级别。
    seg_a_start, seg_a_end : int
        A 段在 moves 列表中的索引范围 [start, end]（闭区间）。
    seg_c_start, seg_c_end : int
        C 段在 moves 列表中的索引范围。
    center_idx : int
        B 段对应的中枢在 centers 列表中的索引。
    force_a : float
        A 段力度（MACD 面积或价格范围代理）。
    force_c : float
        C 段力度。
    confirmed : bool
        背驰是否已确认（C 段完成后确认）。
    dif_peak_a : float
        A 段 DIF 峰值（T6 维度）。无 MACD 时为 0.0。
    dif_peak_c : float
        C 段 DIF 峰值（T6 维度）。
    hist_peak_a : float
        A 段 HIST 峰值（T7 维度）。无 MACD 时为 0.0。
    hist_peak_c : float
        C 段 HIST 峰值（T7 维度）。
    """

    kind: Literal["trend", "consolidation"]
    direction: Literal["top", "bottom"]
    level_id: int
    seg_a_start: int
    seg_a_end: int
    seg_c_start: int
    seg_c_end: int
    center_idx: int
    force_a: float
    force_c: float
    confirmed: bool
    dif_peak_a: float = 0.0
    dif_peak_c: float = 0.0
    hist_peak_a: float = 0.0
    hist_peak_c: float = 0.0


# ====================================================================
# 内部工具
# ====================================================================

def _move_merged_range(moves: list, start: int, end: int) -> tuple[int, int]:
    """获取 moves[start:end+1] 覆盖的 merged bar 索引范围。"""
    if start > end or start < 0 or end >= len(moves):
        return (0, 0)
    i0 = moves[start].i0
    i1 = moves[end].i1
    return (i0, i1)


def _compute_force(
    moves: list,
    seg_start: int,
    seg_end: int,
    trend_direction: str,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> float:
    """计算一段 moves 的力度。

    有 MACD 数据时用 MACD 面积，否则用价格振幅 × 持续长度。
    """
    if seg_start > seg_end or seg_start < 0 or seg_end >= len(moves):
        return 0.0

    i0, i1 = _move_merged_range(moves, seg_start, seg_end)

    if df_macd is not None and merged_to_raw is not None:
        # 转换 merged → raw 索引
        raw_i0 = merged_to_raw[i0][0] if i0 < len(merged_to_raw) else 0
        raw_i1 = merged_to_raw[i1][1] if i1 < len(merged_to_raw) else 0
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)
        # 上涨看 area_pos（红柱），下跌看 |area_neg|（绿柱）
        if trend_direction == "up":
            return abs(area["area_pos"])
        else:
            return abs(area["area_neg"])

    # Fallback：价格振幅 × 持续 bar 数
    high = max(moves[k].high for k in range(seg_start, seg_end + 1))
    low = min(moves[k].low for k in range(seg_start, seg_end + 1))
    duration = max(1, i1 - i0)
    return (high - low) * duration


# ====================================================================
# 趋势背驰
# ====================================================================

def _trend_ac_segments(
    centers: list[Center],
    trend: TrendTypeInstance,
    moves: list,
) -> tuple[int, int, int, int] | None:
    """计算趋势背驰的 A 段和 C 段范围。返回 None 表示无法计算。"""
    ci_prev = trend.center_indices[-2]
    ci_last = trend.center_indices[-1]
    c_prev = centers[ci_prev]
    c_last = centers[ci_last]

    a_start = c_prev.seg1 + 1
    a_end = c_last.seg0 - 1
    if a_start > a_end:
        a_start = c_prev.seg1
        a_end = c_prev.seg1

    c_start = c_last.seg1 + 1
    c_end = trend.seg1
    if c_start > c_end:
        return None

    if a_start >= len(moves) or c_end >= len(moves):
        return None

    return a_start, a_end, c_start, c_end


def _detect_trend_divergence(
    moves: list,
    centers: list[Center],
    trend: TrendTypeInstance,
    trend_idx: int,
    level_id: int,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> Divergence | None:
    """检测单个趋势实例中的背驰。"""
    if trend.kind != "trend" or len(trend.center_indices) < 2:
        return None

    ac = _trend_ac_segments(centers, trend, moves)
    if ac is None:
        return None
    a_start, a_end, c_start, c_end = ac

    force_a = _compute_force(moves, a_start, a_end, trend.direction,
                             df_macd, merged_to_raw)
    force_c = _compute_force(moves, c_start, c_end, trend.direction,
                             df_macd, merged_to_raw)

    if force_a <= 0:
        return None

    if force_c < force_a:
        ci_last = trend.center_indices[-1]
        div_dir: Literal["top", "bottom"] = (
            "top" if trend.direction == "up" else "bottom"
        )
        return Divergence(
            kind="trend",
            direction=div_dir,
            level_id=level_id,
            seg_a_start=a_start,
            seg_a_end=a_end,
            seg_c_start=c_start,
            seg_c_end=c_end,
            center_idx=ci_last,
            force_a=force_a,
            force_c=force_c,
            confirmed=trend.confirmed,
        )

    return None


# ====================================================================
# 盘整背驰
# ====================================================================

def _collect_exit_moves(
    moves: list,
    centers: list[Center],
    trend: TrendTypeInstance,
) -> dict[str, list[int]]:
    """收集中枢内和中枢后的离开段索引，按方向分组。"""
    ci = trend.center_indices[0]
    c = centers[ci]
    exit_moves_by_dir: dict[str, list[int]] = {"up": [], "down": []}

    for k in range(c.seg0, min(c.seg1 + 1, len(moves))):
        m = moves[k]
        direction = getattr(m, "direction", "")
        if (m.high > c.high or m.low < c.low) and direction in exit_moves_by_dir:
            exit_moves_by_dir[direction].append(k)

    for k in range(c.seg1 + 1, min(trend.seg1 + 1, len(moves))):
        m = moves[k]
        direction = getattr(m, "direction", "")
        if direction in exit_moves_by_dir:
            exit_moves_by_dir[direction].append(k)

    return exit_moves_by_dir


def _detect_consolidation_divergence(
    moves: list,
    centers: list[Center],
    trend: TrendTypeInstance,
    trend_idx: int,
    level_id: int,
    df_macd: pd.DataFrame | None,
    merged_to_raw: list[tuple[int, int]] | None,
) -> Divergence | None:
    """检测单个盘整实例中的盘整背驰。"""
    if trend.kind != "consolidation" or len(trend.center_indices) < 1:
        return None

    ci = trend.center_indices[0]
    exit_moves_by_dir = _collect_exit_moves(moves, centers, trend)

    for direction, exits in exit_moves_by_dir.items():
        if len(exits) < 2:
            continue

        a_idx = exits[-2]
        c_idx = exits[-1]

        force_a = _compute_force(moves, a_idx, a_idx, direction,
                                 df_macd, merged_to_raw)
        force_c = _compute_force(moves, c_idx, c_idx, direction,
                                 df_macd, merged_to_raw)

        if force_a <= 0:
            continue

        if force_c < force_a:
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
                center_idx=ci,
                force_a=force_a,
                force_c=force_c,
                confirmed=trend.confirmed,
            )

    return None


# ====================================================================
# 主函数
# ====================================================================

def divergences_from_level(
    moves: list,
    centers: list[Center],
    trends: list[TrendTypeInstance],
    level_id: int,
    *,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[Divergence]:
    """检测某一递归层级中所有走势类型实例的背驰。

    Parameters
    ----------
    moves : list
        Move[k-1] 对象列表（Segment 或 TrendTypeInstance）。
    centers : list[Center]
        该层级的中枢列表。
    trends : list[TrendTypeInstance]
        该层级的走势类型实例列表。
    level_id : int
        递归层级。
    df_macd : pd.DataFrame | None
        MACD 数据（由 compute_macd 返回）。None 则用价格振幅 fallback。
    merged_to_raw : list[tuple[int, int]] | None
        merged → raw 索引映射。

    Returns
    -------
    list[Divergence]
        检测到的背驰列表。
    """
    result: list[Divergence] = []

    for ti, trend in enumerate(trends):
        # 趋势背驰
        div = _detect_trend_divergence(
            moves, centers, trend, ti, level_id, df_macd, merged_to_raw,
        )
        if div is not None:
            result.append(div)
            continue  # 一个走势类型实例最多报告一个背驰

        # 盘整背驰
        div = _detect_consolidation_divergence(
            moves, centers, trend, ti, level_id, df_macd, merged_to_raw,
        )
        if div is not None:
            result.append(div)

    return result
