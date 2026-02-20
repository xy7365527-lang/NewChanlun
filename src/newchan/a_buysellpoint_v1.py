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
- [TBD-2] 走势完成映射 → ✅ maimai #2 已落地（Type 2/3 confirmed = Move.settled）
- [TBD-3] 确认时机定义 → ✅ maimai #2 已落地
- [TBD-4] 盘整背驰与买卖点
- [TBD-5] 中枢范围（ZG/ZD 固定 vs 动态）
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace
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
    center_seg_start: int | None

    # 状态字段
    price: float
    bar_idx: int
    confirmed: bool
    settled: bool

    # 可选：2B+3B 重合标记
    overlaps_with: Literal["type2", "type3"] | None = None


# ── Type 1: 趋势背驰买卖点 ──


def _find_move_for_seg(moves: list[Move], seg_idx: int) -> Move | None:
    """查找包含指定 seg_idx 的 Move。

    maimai #2: confirmed 语义对齐 — BSP.confirmed 应来自 Move.settled，
    而非 Segment.confirmed。此辅助函数在 Type 2/3 检测中查找覆盖段的走势类型。
    """
    for m in moves:
        if m.seg_start <= seg_idx <= m.seg_end:
            return m
    return None

def _find_assoc_trend_move(
    moves: list[Move], div, zhongshus: list[Zhongshu],
) -> Move | None:
    """找包含背驰中枢的趋势 Move。"""
    if div.center_idx >= len(zhongshus):
        return None
    for m in moves:
        if m.zs_start <= div.center_idx <= m.zs_end and m.kind == "trend":
            return m
    return None


def _detect_type1(
    moves: list[Move],
    divergences: list[Divergence],
    zhongshus: list[Zhongshu],
    segments: list,
    level_id: int,
) -> list[BuySellPoint]:
    """第一类买卖点：趋势背驰点。"""
    result: list[BuySellPoint] = []

    for div in divergences:
        if div.kind != "trend":
            continue

        assoc_move = _find_assoc_trend_move(moves, div, zhongshus)
        if assoc_move is None:
            continue

        zs = zhongshus[div.center_idx]
        side: Literal["buy", "sell"] = (
            "buy" if div.direction == "bottom" else "sell"
        )
        seg_idx = div.seg_c_end

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
            confirmed=div.confirmed,
            settled=False,
        ))

    return result


# ── Type 2: 回调/反弹买卖点 ──

def _find_next_seg_by_direction(
    segments: list, start: int, direction: str,
) -> int | None:
    """从 start 开始找第一个指定方向的段索引。"""
    for k in range(start, len(segments)):
        if segments[k].direction == direction:
            return k
    return None


def _make_type2_point(
    t1: BuySellPoint, seg_idx: int, seg, side: str,
    moves: list[Move], level_id: int,
) -> BuySellPoint:
    """构造 Type2 买卖点。"""
    assoc_move = _find_move_for_seg(moves, seg_idx)
    return BuySellPoint(
        kind="type2",
        side=side,
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=t1.move_seg_start,
        divergence_key=t1.divergence_key,
        center_zd=t1.center_zd,
        center_zg=t1.center_zg,
        center_seg_start=t1.center_seg_start,
        price=seg.low if side == "buy" else seg.high,
        bar_idx=seg.i1,
        confirmed=assoc_move.settled if assoc_move else False,
        settled=False,
    )


def _detect_type2(
    type1_points: list[BuySellPoint],
    segments: list,
    moves: list[Move],
    level_id: int,
) -> list[BuySellPoint]:
    """第二类买卖点：Type 1 之后的第一次回调/反弹。"""
    result: list[BuySellPoint] = []

    for t1 in type1_points:
        if t1.side == "buy":
            rebound_idx = _find_next_seg_by_direction(segments, t1.seg_idx + 1, "up")
            if rebound_idx is None:
                continue
            callback_idx = _find_next_seg_by_direction(segments, rebound_idx + 1, "down")
            if callback_idx is None:
                continue
            result.append(_make_type2_point(
                t1, callback_idx, segments[callback_idx], "buy", moves, level_id,
            ))
        elif t1.side == "sell":
            pullback_idx = _find_next_seg_by_direction(segments, t1.seg_idx + 1, "down")
            if pullback_idx is None:
                continue
            rebound_idx_s = _find_next_seg_by_direction(segments, pullback_idx + 1, "up")
            if rebound_idx_s is None:
                continue
            result.append(_make_type2_point(
                t1, rebound_idx_s, segments[rebound_idx_s], "sell", moves, level_id,
            ))

    return result


# ── Type 3: 中枢突破回试买卖点 ──

def _make_type3_point(
    zs, seg_idx: int, seg, side: str,
    moves: list[Move], level_id: int,
) -> BuySellPoint:
    """构造 Type3 买卖点。"""
    assoc_move = _find_move_for_seg(moves, seg_idx)
    return BuySellPoint(
        kind="type3",
        side=side,
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=zs.seg_start,
        divergence_key=None,
        center_zd=zs.zd,
        center_zg=zs.zg,
        center_seg_start=zs.seg_start,
        price=seg.low if side == "buy" else seg.high,
        bar_idx=seg.i1,
        confirmed=assoc_move.settled if assoc_move else False,
        settled=False,
    )


def _detect_type3(
    zhongshus: list[Zhongshu],
    segments: list,
    moves: list[Move],
    level_id: int,
) -> list[BuySellPoint]:
    """第三类买卖点：中枢突破后回试/回抽。"""
    result: list[BuySellPoint] = []

    for zs in zhongshus:
        if not zs.settled or not zs.break_direction:
            continue

        break_seg_idx = zs.break_seg
        if break_seg_idx < 0 or break_seg_idx >= len(segments):
            continue

        opposite_dir = "down" if zs.break_direction == "up" else "up"
        pullback_idx = _find_next_seg_by_direction(segments, break_seg_idx + 1, opposite_dir)
        if pullback_idx is None:
            continue

        pullback_seg = segments[pullback_idx]

        if zs.break_direction == "up" and pullback_seg.low > zs.zg:
            result.append(_make_type3_point(zs, pullback_idx, pullback_seg, "buy", moves, level_id))
        elif zs.break_direction == "down" and pullback_seg.high < zs.zd:
            result.append(_make_type3_point(zs, pullback_idx, pullback_seg, "sell", moves, level_id))

    return result


# ── 2B+3B 重合检测 ──

def _detect_overlap(
    type2_points: list[BuySellPoint],
    type3_points: list[BuySellPoint],
) -> tuple[list[BuySellPoint], list[BuySellPoint]]:
    """2B+3B 重合检测。

    [旧缠论] 第21课：V型反转时 2B 与 3B 可在同一 seg_idx 上重合。

    由于 BuySellPoint 是 frozen dataclass，使用 dataclasses.replace
    创建带 overlaps_with 标记的新实例。
    """
    # 构建 type3 的 (seg_idx, side, level_id) → index 映射
    t3_map: dict[tuple[int, str, int], int] = {}
    for i, t3 in enumerate(type3_points):
        t3_map[(t3.seg_idx, t3.side, t3.level_id)] = i

    new_t2 = list(type2_points)
    new_t3 = list(type3_points)

    for i, t2 in enumerate(type2_points):
        key = (t2.seg_idx, t2.side, t2.level_id)
        if key in t3_map:
            j = t3_map[key]
            new_t2[i] = replace(t2, overlaps_with="type3")
            new_t3[j] = replace(type3_points[j], overlaps_with="type2")

    return new_t2, new_t3


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
    type2 = _detect_type2(type1, segments, moves, level_id)
    type3 = _detect_type3(zhongshus, segments, moves, level_id)
    type2, type3 = _detect_overlap(type2, type3)

    return sorted(type1 + type2 + type3, key=lambda bp: bp.seg_idx)
