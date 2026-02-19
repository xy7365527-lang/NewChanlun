"""A 系统 — 走势类型实例 v1

从已闭合中枢列表构造走势类型实例（Move）。

核心规则（冻结 v1 spec）：
- 盘整 = 包含恰好 1 个走势中枢
- 趋势 = 包含 2+ 个依次同向走势中枢
  - 上涨：C2.DD > C1.GG（后枢波动下界 严格高于 前枢波动上界，中心定理二）
  - 下跌：C2.GG < C1.DD（后枢波动上界 严格低于 前枢波动下界，中心定理二）
- 区间重叠的相邻中枢 → 截断为不同 move
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Literal

from newchan.a_zhongshu_v1 import Zhongshu


@dataclass(frozen=True, slots=True)
class Move:
    """一个走势类型实例。

    Attributes
    ----------
    kind : Literal["consolidation", "trend"]
        盘整（1 中枢）/ 趋势（2+ 同向中枢）。
    direction : Literal["up", "down"]
        走势方向。盘整=break_direction，趋势=ascending/descending。
    seg_start : int
        第一个中枢的 seg_start（identity key）。
    seg_end : int
        最后中枢的 seg_end。
    zs_start : int
        第一个中枢在 zhongshu list 中的索引。
    zs_end : int
        最后中枢的索引。
    zs_count : int
        中枢数量 (>= 1)。
    settled : bool
        是否已被后续 move 确认（最后一个=False）。
    high : float
        max(center.gg) — 波动区间上界（含离开段极值）。
    low : float
        min(center.dd) — 波动区间下界（含离开段极值）。
    first_seg_s0 : int
        前端定位用：第一段的 stroke s0。
    last_seg_s1 : int
        前端定位用：最后段的 stroke s1。
    """

    kind: Literal["consolidation", "trend"]
    direction: Literal["up", "down"]
    seg_start: int
    seg_end: int
    zs_start: int
    zs_end: int
    zs_count: int
    settled: bool
    high: float = 0.0
    low: float = 0.0
    first_seg_s0: int = 0
    last_seg_s1: int = 0


def _is_ascending(c1: Zhongshu, c2: Zhongshu) -> bool:
    """后枢 DD 严格高于 前枢 GG → 上涨延续（中心定理二）。"""
    return c2.dd > c1.gg


def _is_descending(c1: Zhongshu, c2: Zhongshu) -> bool:
    """后枢 GG 严格低于 前枢 DD → 下跌延续（中心定理二）。"""
    return c2.gg < c1.dd


def moves_from_zhongshus(
    zhongshus: list[Zhongshu],
    num_segments: int | None = None,
) -> list[Move]:
    """从中枢列表构造 Move（贪心分组，只处理 settled 中枢）。

    算法：
    1. 过滤 settled=True 的中枢
    2. 贪心向右扫描，同向中枢归入同一 group
    3. 每个 group → 一个 Move
    4. seg_end 扩展覆盖 C段（中枢后的离开段）
    5. 最后一个 move 强制 settled=False

    Parameters
    ----------
    zhongshus : list[Zhongshu]
        中枢列表（含 settled 和 unsettled）。
    num_segments : int | None
        总线段数。提供时，末组 Move 的 seg_end 扩展到 num_segments - 1，
        覆盖最后中枢之后的所有段（C段）。不提供时保持旧行为。

    Returns
    -------
    list[Move]
        按 seg_start 递增排序的走势类型实例列表。
    """
    # 过滤 settled 中枢并记录原始索引
    settled_indices: list[int] = []
    settled_zs: list[Zhongshu] = []
    for i, zs in enumerate(zhongshus):
        if zs.settled:
            settled_indices.append(i)
            settled_zs.append(zs)

    if not settled_zs:
        return []

    # 贪心分组：(group_centers, trend_direction)
    groups: list[tuple[list[int], str]] = []  # [(settled_zs 中的索引列表, "up"/"down"/"")]
    current_offsets: list[int] = [0]  # 在 settled_zs 中的偏移
    current_dir: str = ""  # "" = 单中枢, "up"/"down" = 趋势方向

    for k in range(1, len(settled_zs)):
        prev_zs = settled_zs[k - 1]
        curr_zs = settled_zs[k]

        if _is_ascending(prev_zs, curr_zs) and current_dir in ("", "up"):
            current_offsets.append(k)
            current_dir = "up"
        elif _is_descending(prev_zs, curr_zs) and current_dir in ("", "down"):
            current_offsets.append(k)
            current_dir = "down"
        else:
            # 不兼容 → 截断当前 group，开始新 group
            groups.append((current_offsets, current_dir))
            current_offsets = [k]
            current_dir = ""

    groups.append((current_offsets, current_dir))

    # 转换 groups → Moves（扩展 seg_end 覆盖 C段）
    result: list[Move] = []
    for g_idx, (offsets, direction) in enumerate(groups):
        first_zs = settled_zs[offsets[0]]
        last_zs = settled_zs[offsets[-1]]
        zs_count = len(offsets)

        zs_start = settled_indices[offsets[0]]
        zs_end = settled_indices[offsets[-1]]

        # seg_end 扩展：覆盖最后中枢之后的段（C段/连接段）
        base_seg_end = last_zs.seg_end
        if g_idx < len(groups) - 1:
            next_first_offset = groups[g_idx + 1][0][0]
            next_first_zs = settled_zs[next_first_offset]
            base_seg_end = next_first_zs.seg_start - 1
        elif num_segments is not None and num_segments > 0:
            base_seg_end = num_segments - 1

        if zs_count >= 2:
            kind: Literal["consolidation", "trend"] = "trend"
            move_dir: Literal["up", "down"] = direction  # type: ignore[assignment]
        else:
            kind = "consolidation"
            move_dir = first_zs.break_direction  # type: ignore[assignment]

        group_centers = [settled_zs[o] for o in offsets]
        result.append(Move(
            kind=kind,
            direction=move_dir,
            seg_start=first_zs.seg_start,
            seg_end=base_seg_end,
            zs_start=zs_start,
            zs_end=zs_end,
            zs_count=zs_count,
            settled=True,
            high=max(zs.gg for zs in group_centers),
            low=min(zs.dd for zs in group_centers),
            first_seg_s0=first_zs.first_seg_s0,
            last_seg_s1=last_zs.last_seg_s1,
        ))

    # 最后一个 move → unsettled
    if result:
        result[-1] = replace(result[-1], settled=False)

    return result
