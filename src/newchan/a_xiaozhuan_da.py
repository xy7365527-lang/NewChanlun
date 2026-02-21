"""小转大检测 — 小级别背驰引发大级别转折。

原文依据（三级权威链）
---------------------
- 第43课：转折的两种方式（背驰级别=走势级别 vs 背驰级别<走势级别）
  > "某级别的背驰必然导致该级别原走势类型的终止"
  > "小级别背驰最终转化成大级别转折"
  > "形成一个比1分钟级别要大的中枢，然后向下突破"
- 第53课：小转大时买卖点的选择
  > "小级别转大级别的情况...没有该级别的第一类买卖点"
  > "第二类买卖点就是最佳的"
- 第29课答疑：各级别小转大的存在使各种走势都成为可能
- 第66课：小转大一般都有一个小平台

定义
----
**小转大**：在 a+A+b+B+c 趋势结构中，c 段未出现本级别背驰，
但 c 段内出现了次级别背驰，该次级别背驰可能引发中枢级别升级，
终结本级别走势。

前置条件：
1. 本级别走势为趋势（≥2中枢）
2. 本级别**无**背驰（c 段力度不弱于 a 段）
3. c 段 bar 范围内存在次级别背驰

概念溯源: [旧缠论] 第43课（两种转折方式）+ 第53课（买卖点补充）
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Literal

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu

logger = logging.getLogger(__name__)


@dataclass(frozen=True, slots=True)
class XiaozhuanDa:
    """小转大检测结果。

    Attributes
    ----------
    level_id : int
        本级别（被"转大"的级别）。
    side : Literal["buy", "sell"]
        买卖方向：上涨趋势小转大 → sell，下跌趋势小转大 → buy。
    move_seg_start : int
        触发小转大的趋势 Move 的 seg_start。
    move_direction : str
        趋势方向。
    c_seg_start : int
        C 段起始 seg 索引。
    c_seg_end : int
        C 段结束 seg 索引。
    sub_divergence : Divergence
        触发小转大的次级别背驰。
    """

    level_id: int
    side: Literal["buy", "sell"]
    move_seg_start: int
    move_direction: str
    c_seg_start: int
    c_seg_end: int
    sub_divergence: Divergence


def _has_divergence_for_move(
    move: Move,
    divergences: list[Divergence],
) -> bool:
    """检查本级别是否对该 Move 报告了背驰。"""
    for div in divergences:
        if div.seg_c_start >= move.seg_start and div.seg_c_end <= move.seg_end:
            return True
    return False


def _c_segment_range(
    move: Move,
    zhongshus: list[Zhongshu],
) -> tuple[int, int] | None:
    """计算趋势 Move 的 C 段 seg 范围。

    C 段 = 最后一个 settled 中枢的 seg_end + 1 → move.seg_end。
    """
    last_zs_idx = None
    for i in range(move.zs_start, min(move.zs_end + 1, len(zhongshus))):
        if zhongshus[i].settled:
            last_zs_idx = i

    if last_zs_idx is None:
        return None

    zs_last = zhongshus[last_zs_idx]
    c_start = zs_last.seg_end + 1
    c_end = move.seg_end

    if c_start > c_end:
        return None

    return c_start, c_end


def _sub_divergence_in_c_range(
    sub_divergences: list[Divergence],
    c_seg_start: int,
    c_seg_end: int,
    segments: list,
) -> Divergence | None:
    """查找次级别背驰中 C 段终点落入本级别 c 段 bar 范围内的。

    次级别背驰的 seg_c_end 对应的 bar 位置需在本级别 c 段
    的 bar 范围 [c_bar_start, c_bar_end] 内。

    返回第一个匹配的次级别背驰，None 表示无匹配。
    """
    if c_seg_start >= len(segments) or c_seg_end >= len(segments):
        return None

    c_bar_start = segments[c_seg_start].i0
    c_bar_end = segments[c_seg_end].i1

    for sub_div in sub_divergences:
        # 次级别背驰的 C 段终点的 bar 位置
        sub_c_end_seg = sub_div.seg_c_end
        if sub_c_end_seg >= len(segments):
            continue

        sub_c_bar_end = segments[sub_c_end_seg].i1
        if c_bar_start <= sub_c_bar_end <= c_bar_end:
            return sub_div

    return None


def detect_xiaozhuan_da(
    segments: list,
    zhongshus: list[Zhongshu],
    moves: list[Move],
    level_divergences: list[Divergence],
    sub_divergences: list[Divergence],
    level_id: int,
) -> list[XiaozhuanDa]:
    """检测小转大。

    对每个未 settled 的趋势 Move：
    1. 检查本级别是否无背驰
    2. 计算 C 段 seg 范围
    3. 检查次级别背驰是否落入 C 段 bar 范围

    Parameters
    ----------
    segments : list
        线段列表（需有 i0, i1 属性）。
    zhongshus : list[Zhongshu]
        v1 中枢列表。
    moves : list[Move]
        v1 走势类型列表。
    level_divergences : list[Divergence]
        本级别的背驰列表。
    sub_divergences : list[Divergence]
        次级别的背驰列表。
    level_id : int
        本级别 ID。

    Returns
    -------
    list[XiaozhuanDa]
        检测到的小转大列表。

    概念溯源: [旧缠论] 第43课
    """
    result: list[XiaozhuanDa] = []

    for move in moves:
        # 条件1: 必须是趋势（≥2中枢）
        if move.kind != "trend" or move.zs_count < 2:
            continue

        # 条件2: 必须是未 settled 的 Move（进行中）
        if move.settled:
            continue

        # 条件3: 本级别无背驰
        if _has_divergence_for_move(move, level_divergences):
            continue

        # 条件4: 计算 C 段范围
        c_range = _c_segment_range(move, zhongshus)
        if c_range is None:
            continue
        c_seg_start, c_seg_end = c_range

        # 条件5: 次级别在 C 段 bar 范围内有背驰
        matched_sub_div = _sub_divergence_in_c_range(
            sub_divergences, c_seg_start, c_seg_end, segments,
        )
        if matched_sub_div is None:
            continue

        side: Literal["buy", "sell"] = (
            "sell" if move.direction == "up" else "buy"
        )

        result.append(XiaozhuanDa(
            level_id=level_id,
            side=side,
            move_seg_start=move.seg_start,
            move_direction=move.direction,
            c_seg_start=c_seg_start,
            c_seg_end=c_seg_end,
            sub_divergence=matched_sub_div,
        ))

    return result
