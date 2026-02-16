"""买卖点识别 — v1 管线骨架。

基于 maimai_rules_v1.md 规范实现。
依赖 v1 管线：Segment + Zhongshu + Move + Divergence。

概念溯源标签
------------
- 第一类买卖点 [旧缠论] 第17课
- 第二类买卖点 [旧缠论] 第17课、第21课
- 第三类买卖点 [旧缠论] 第20课
- 买卖点定律一 [旧缠论] 第17课
- 背驰-买卖点定理 [旧缠论] 第24课

已知 TBD（生成态）
------------------
- [TBD-1] 下跌确立条件（严格 vs 宽松口径）
- [TBD-2] 走势完成映射
- [TBD-3] 确认时机定义
- [TBD-4] 盘整背驰与买卖点
- [TBD-5] 中枢范围（ZG/ZD 固定 vs 动态）
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
class BuySellPoint:
    """买卖点实例。

    身份键：(seg_idx, kind, side, level_id)
    """

    kind: Literal["type1", "type2", "type3"]
    side: Literal["buy", "sell"]
    level_id: int

    # 身份键
    seg_idx: int

    # 关联字段
    move_seg_start: int
    divergence_key: tuple[int, int, int] | None
    center_zd: float
    center_zg: float
    center_seg_start: int

    # 状态字段
    price: float
    bar_idx: int
    confirmed: bool
    settled: bool

    # 可选：2B+3B 重合标记
    overlaps_with: Literal["type2", "type3"] | None = None


# ── Type 1: 趋势背驰买卖点 ──

def _detect_type1(
    moves: list[Move],
    divergences: list[Divergence],
    zhongshus: list[Zhongshu],
    segments: list,
    level_id: int,
) -> list[BuySellPoint]:
    """第一类买卖点：趋势背驰点。

    - 遍历 kind="trend" 的 Divergence
    - direction="bottom" → buy, direction="top" → sell
    - seg_idx = div.seg_c_end（背驰段终段）
    - [TBD-4] 盘整背驰不产生 Type 1
    """
    result: list[BuySellPoint] = []

    for div in divergences:
        if div.kind != "trend":
            continue  # [TBD-4] 盘整背驰跳过

        # 反查关联 Move（通过 center_idx → Zhongshu → Move）
        if div.center_idx >= len(zhongshus):
            continue
        zs = zhongshus[div.center_idx]

        # 找包含此中枢的 Move
        assoc_move: Move | None = None
        for m in moves:
            if m.zs_start <= div.center_idx <= m.zs_end:
                assoc_move = m
                break

        if assoc_move is None:
            continue

        # [TBD-1] 严格口径：必须是趋势
        if assoc_move.kind != "trend":
            continue

        side: Literal["buy", "sell"] = (
            "buy" if div.direction == "bottom" else "sell"
        )
        seg_idx = div.seg_c_end

        # 价格：背驰段终点的分型价
        price = 0.0
        bar_idx = 0
        if seg_idx < len(segments):
            seg = segments[seg_idx]
            price = seg.low if side == "buy" else seg.high
            bar_idx = seg.i1

        result.append(BuySellPoint(
            kind="type1",
            side=side,
            level_id=level_id,
            seg_idx=seg_idx,
            move_seg_start=assoc_move.seg_start,
            divergence_key=(div.center_idx, div.seg_c_start, div.seg_c_end),
            center_zd=zs.zd,
            center_zg=zs.zg,
            center_seg_start=zs.seg_start,
            price=price,
            bar_idx=bar_idx,
            confirmed=div.confirmed,  # [TBD-3]
            settled=False,
        ))

    return result


# ── Type 3: 中枢突破回试买卖点 ──

def _detect_type3(
    zhongshus: list[Zhongshu],
    segments: list,
    level_id: int,
) -> list[BuySellPoint]:
    """第三类买卖点：中枢突破后回试/回抽。

    - 遍历 settled 中枢
    - break_direction="up" → 找回试段（下跌段），low > ZG → 3B
    - break_direction="down" → 找回抽段（上涨段），high < ZD → 3S
    """
    result: list[BuySellPoint] = []

    for zs in zhongshus:
        if not zs.settled or not zs.break_direction:
            continue

        break_seg_idx = zs.break_seg
        if break_seg_idx < 0 or break_seg_idx >= len(segments):
            continue

        # 找离开段之后的第一个反方向段
        pullback_idx: int | None = None
        if zs.break_direction == "up":
            # 找第一个向下段
            for k in range(break_seg_idx + 1, len(segments)):
                if segments[k].direction == "down":
                    pullback_idx = k
                    break
        elif zs.break_direction == "down":
            # 找第一个向上段
            for k in range(break_seg_idx + 1, len(segments)):
                if segments[k].direction == "up":
                    pullback_idx = k
                    break

        if pullback_idx is None:
            continue

        pullback_seg = segments[pullback_idx]

        if zs.break_direction == "up":
            # 3B: 回试段的 low > ZG
            if pullback_seg.low > zs.zg:
                result.append(BuySellPoint(
                    kind="type3",
                    side="buy",
                    level_id=level_id,
                    seg_idx=pullback_idx,
                    move_seg_start=zs.seg_start,
                    divergence_key=None,
                    center_zd=zs.zd,
                    center_zg=zs.zg,
                    center_seg_start=zs.seg_start,
                    price=pullback_seg.low,
                    bar_idx=pullback_seg.i1,
                    confirmed=pullback_seg.confirmed,  # [TBD-3]
                    settled=False,
                ))
        elif zs.break_direction == "down":
            # 3S: 回抽段的 high < ZD
            if pullback_seg.high < zs.zd:
                result.append(BuySellPoint(
                    kind="type3",
                    side="sell",
                    level_id=level_id,
                    seg_idx=pullback_idx,
                    move_seg_start=zs.seg_start,
                    divergence_key=None,
                    center_zd=zs.zd,
                    center_zg=zs.zg,
                    center_seg_start=zs.seg_start,
                    price=pullback_seg.high,
                    bar_idx=pullback_seg.i1,
                    confirmed=pullback_seg.confirmed,  # [TBD-3]
                    settled=False,
                ))

    return result


# ── 入口函数 ──

def buysellpoints_from_level(
    segments: list,
    zhongshus: list[Zhongshu],
    moves: list[Move],
    divergences: list[Divergence],
    level_id: int,
) -> list[BuySellPoint]:
    """从某一递归层级的走势结构中识别所有买卖点。

    纯函数：无副作用，每次全量计算。增量通过 diff 层实现。

    Parameters
    ----------
    segments : list
        已确认线段列表。
    zhongshus : list[Zhongshu]
        v1 中枢列表。
    moves : list[Move]
        v1 走势类型列表。
    divergences : list[Divergence]
        背驰列表（可来自 v0 或 v1 管线）。
    level_id : int
        递归级别。

    Returns
    -------
    list[BuySellPoint]
        按 seg_idx 排序的买卖点列表。
    """
    type1 = _detect_type1(moves, divergences, zhongshus, segments, level_id)
    # type2 = _detect_type2(type1, segments, moves, level_id)  # TODO: 实现
    type3 = _detect_type3(zhongshus, segments, level_id)
    # _detect_overlap(type2, type3)  # TODO: 2B+3B 重合标记

    return sorted(type1 + type3, key=lambda bp: bp.seg_idx)
