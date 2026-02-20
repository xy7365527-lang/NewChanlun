"""A 系统 — 递归引擎（自下而上构造全部结构层级）

缠论核心机制：
  Move[0] = Segment
  Center[k] = 三个连续 Move[k-1] 的重叠（ZG > ZD）
  TrendTypeInstance[k] = 包含 Center[k] 的走势类型实例 = Move[k]
  递归直到无法形成更高级别中枢为止。

级别是市场"长"出来的，不是预设的。

规格引用: docs/chan_spec.md §6, §7, §8, §9
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field

import pandas as pd

from newchan.a_center_v0 import Center, centers_from_segments_v0
from newchan.a_divergence import Divergence, divergences_from_level
from newchan.a_level_fsm_newchan import LevelView
from newchan.a_segment_v0 import Segment
from newchan.a_trendtype_v0 import (
    TrendTypeInstance,
    label_centers_development,
    trend_instances_from_centers,
)

logger = logging.getLogger(__name__)


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class RecursiveLevel:
    """一层递归的完整数据。

    Attributes
    ----------
    level : int
        结构层级（1 = 第一级中枢，2 = 第二级，...）。
    moves : list
        该层使用的 Move[k-1] 对象列表
        （level=1 时为 Segment，level>=2 时为 TrendTypeInstance）。
    centers : list[Center]
        该层构建的中枢列表。
    trends : list[TrendTypeInstance]
        该层构建的走势类型实例列表。
    """

    level: int
    moves: list
    centers: list[Center]
    trends: list[TrendTypeInstance]
    divergences: list[Divergence] = field(default_factory=list)


# ====================================================================
# 主函数
# ====================================================================

def _stamp_centers(centers: list[Center], level_id: int) -> list[Center]:
    """为中枢列表标注 level_id 并计算 development。"""
    stamped = [
        Center(
            seg0=c.seg0, seg1=c.seg1, low=c.low, high=c.high,
            kind=c.kind, confirmed=c.confirmed, sustain=c.sustain,
            direction=c.direction,
            gg=c.gg, dd=c.dd, g=c.g, d=c.d,
            zg_dynamic=c.zg_dynamic, zd_dynamic=c.zd_dynamic,
            development=c.development,
            level_id=level_id,
            terminated=c.terminated,
            termination_side=c.termination_side,
        )
        for c in centers
    ]
    return label_centers_development(stamped)


def _stamp_trends(trends: list[TrendTypeInstance], level_id: int) -> list[TrendTypeInstance]:
    """为走势类型实例列表标注 level_id。"""
    return [
        TrendTypeInstance(
            kind=t.kind, direction=t.direction,
            seg0=t.seg0, seg1=t.seg1,
            i0=t.i0, i1=t.i1,
            high=t.high, low=t.low,
            center_indices=t.center_indices,
            confirmed=t.confirmed,
            level_id=level_id,
        )
        for t in trends
    ]


def _build_single_level(
    moves: list, k: int, sustain_m: int,
    df_macd: pd.DataFrame | None, merged_to_raw: list[tuple[int, int]] | None,
) -> RecursiveLevel | None:
    """构建单层递归数据。返回 None 表示该层无法形成。"""
    centers = centers_from_segments_v0(moves, sustain_m=sustain_m)
    if not centers:
        logger.debug("Level %d: no centers formed, recursion stops", k)
        return None
    trends = trend_instances_from_centers(moves, centers)
    if not trends:
        logger.debug("Level %d: no trend instances formed, recursion stops", k)
        return None

    centers = _stamp_centers(centers, k)
    trends = _stamp_trends(trends, k)
    divs = divergences_from_level(
        moves, centers, trends, level_id=k,
        df_macd=df_macd, merged_to_raw=merged_to_raw,
    )
    logger.info("Level %d: %d centers (%d settled), %d trend instances",
                k, len(centers), sum(1 for c in centers if c.kind == "settled"),
                len(trends))
    return RecursiveLevel(level=k, moves=moves, centers=centers,
                          trends=trends, divergences=divs)


def build_recursive_levels(
    segments: list[Segment],
    *,
    sustain_m: int = 2,
    max_levels: int = 10,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list[RecursiveLevel]:
    """自下而上递归构造全部结构层级。

    递归规则（docs/chan_spec.md §6）：Move[0]=Segment，
    每层构建 Center[k] → TrendTypeInstance[k] → Move[k]，
    直到 confirmed trends < 3 为止。
    """
    levels: list[RecursiveLevel] = []
    moves: list = list(segments)

    for k in range(1, max_levels + 1):
        if len(moves) < 3:
            logger.debug("Level %d: only %d moves, recursion stops", k, len(moves))
            break

        level = _build_single_level(moves, k, sustain_m, df_macd, merged_to_raw)
        if level is None:
            break
        levels.append(level)

        confirmed_trends = [t for t in level.trends if t.confirmed]
        if len(confirmed_trends) < 3:
            logger.debug("Level %d: only %d confirmed trends, recursion stops",
                        k, len(confirmed_trends))
            break
        moves = confirmed_trends

    return levels


def levels_to_level_views(levels: list[RecursiveLevel]) -> list[LevelView]:
    """将递归层级转换为 L* 选择所需的 LevelView 列表。"""
    return [
        LevelView(
            level=rl.level,
            segments=rl.moves,
            centers=rl.centers,
        )
        for rl in levels
    ]
