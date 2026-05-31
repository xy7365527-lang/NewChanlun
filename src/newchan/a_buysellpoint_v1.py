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
- [TBD-1] 下跌确立条件（严格 vs 宽松口径） → ✅ maimai #1 已结算（严格口径：≥2中枢=下跌趋势）
- [TBD-2] 走势完成映射 → ✅ 已修订（maimai #2 v0.7 翻转）：走势完成（Move.settled）映射到 BSP.settled，
  不再映射到 confirmed。详见下方"confirmed / settled 语义"。
- [TBD-3] 确认时机定义 → ✅ 已修订：confirmed = 买卖点自身确认条件（见下）。
- [TBD-4] 盘整背驰与买卖点 → ✅ maimai #4 已结算（盘整背驰不触发第一类买卖点；可间接导致第三类买点，由 Type3 机制独立识别）
- [TBD-5] 中枢范围（ZG/ZD 固定 vs 动态） → ✅ maimai #5 已结算（判定范围=[ZD,ZG]，非[DD,GG]）

candidate / confirmed / settled 语义（candidate-fix + confirmed-fix）
-----------------------------------------------------------------
三个状态分别对应买卖点生命周期的三个时刻（左侧形成 → 右侧确认 → 事后验证）：

- **candidate**（结构开始形成，左侧）：买卖点首次进入列表即为候选。由 diff 层
  （buysellpoint_state.diff_buysellpoints）在 curr_only 时发出 BuySellPointCandidateV1。
- **confirmed**（右侧确认完成，可操作信号）：买卖点自身确认条件成立时翻 True，
  diff 层在 confirmed False→True 时发出 BuySellPointConfirmV1。
- **settled**（事后走势验证）：覆盖该买卖点的 Move.settled，无 Move 覆盖时降级 False。

candidate-fix（candidate/confirmed 时间分离，本次翻转）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
旧设计中 Type1（confirmed=div.confirmed，v1 管线恒 True）与 Type3（confirmed 硬编码 True）
在买卖点首次出现的同一 bar 即 confirmed —— candidate 与 confirm 同 bar 发出，
右侧确认形同虚设。本次将两者按缠论的"左侧形成 vs 右侧确认"分离：

- **Type1（第17/24课）背驰区域面积**：
  - candidate = 背驰存在（force_c < force_a，面积开始缩小，由 Divergence 构造保证）。
  - confirmed = 面积比 force_c/force_a ≤ ``TYPE1_CONFIRM_RATIO``（面积比低于阈值，力竭确认）。
    [新缠论:选择] 阈值默认 0.9，可调；面积比靠近 1 的弱背驰只作候选，不确认。
- **Type2（第17/21课）回调/反弹不创新极值**：candidate = confirmed（同步）。
  买:回试 low ≥ 1B low；卖:回抽 high ≤ 1S high。不创新极值时候选即确认（无右侧滞后）。
- **Type3（第20课）中枢突破回试**：
  - candidate = 中枢突破后回试段存在且当前不破边界（回试 low>ZG / 回抽 high<ZD）。
  - confirmed = 回试段之后出现 break_direction 方向的延续段（回试结束、走势延续 →
    回试不破已被后续结构确认）。回试中途跌回中枢则该候选从列表消失（diff 发 Invalidate）。

身份键 (seg_idx, kind, side, level_id) 在 candidate→confirmed 转变中保持稳定：
Type1 锚定 C 段终段，Type3 锚定回试段（保 2B+3B 重合检测与 price 语义不变）。

谱系：candidate-fix（candidate/confirmed 时间分离）；confirmed-fix（maimai #2 翻转，
confirmed 与 Move.settled 解耦）；005b 对象否定对象（走势完成由 Move 内部机制否定）。

525号上下游推论（判据降维：有效性判据 → 构造标签）
-----------------------------------------------------------------
525号将"递归链完整可追溯至 Move[0]"从中枢有效性判据降为口径A构造不变量。
本模块同构落实如下，确保 candidate/confirmed 不重新引入递归链依赖：

1. **组件来源无关（component-source-agnostic）**：``buysellpoints_from_level`` 只要求
   ``segments`` 满足最小区间接口（direction/high/low/i0/i1）。**线段（段中枢）与笔（笔中枢，
   525号 zhongshu_from_strokes）均满足**——同一函数不加修改地服务两条路径。买卖点检测
   不关心组件来自哪一递归层、递归链多深（点1：笔中枢路径；点2：判据放松为"组件已完成"）。
2. **confirmed 与递归链完整性无关**：confirmed 只由本级别可观测的结构条件决定
   （Type1 面积比 / Type2 不创新极值 / Type3 回试后延续段），不依赖"能否下钻追溯到 Move[0]"，
   亦不依赖 Move.settled（点4）。组件在当前级别图上完成即可判定，不下钻（第18课:24）。
3. **level_id 是构造标签，非有效性声明**：与 525号"递归链完整=构造不变量而非有效性判据"同构，
   level_id 仅标记买卖点所在的构造层（口径A 来源），不构成"该买卖点是否有效"的判据。
4. **candidate = 缠论结构开始形成；candidate + PH settle = 进场**（点5）：candidate 是左侧
   结构信号，PH settle（分型因果确认）替代传统右侧确认作进场门控。
   见 ``analysis/candidate_settle_backtest_qqq.py``（笔中枢 + candidate + PH settle 回测）。
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace
from typing import Literal

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu

logger = logging.getLogger(__name__)

# Type1 confirmed 阈值（[新缠论:选择]，candidate-fix）：
# 背驰面积比 force_c/force_a ≤ 此值时，第一类买卖点从 candidate 翻为 confirmed。
# 面积比落在 (TYPE1_CONFIRM_RATIO, 1.0) 的弱背驰只作候选（candidate），不确认。
# 默认 0.9 = C 段力度至多为 A 段的 90%；可由调用方按品种/级别调参。
TYPE1_CONFIRM_RATIO = 0.9


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

    confirmed-fix: 此辅助函数在 Type 2/3 检测中查找覆盖段的走势类型，
    用于 BSP.settled（走势完成验证），不再用于 confirmed。
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
            # candidate-fix: confirmed = 面积比 force_c/force_a ≤ 阈值（力竭确认）。
            # 背驰存在（div 被构造）即 candidate；面积比低于阈值才 confirmed。
            # 面积比靠近 1 的弱背驰停留在 candidate（confirmed=False），等待面积进一步缩小。
            confirmed=(div.force_a > 0 and div.force_c / div.force_a <= TYPE1_CONFIRM_RATIO),
            # settled = 走势完成验证（事后）；无关联趋势 Move 时降级 False
            settled=assoc_move.settled,
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
    """构造 Type2 买卖点。

    confirmed-fix（第17/21课）：第二类买卖点确认条件 = 次级别回调/反弹不创新极值。
    - 2B：回调段 low ≥ 一类买点 low（不跌破前低）→ confirmed
    - 2S：反弹段 high ≤ 一类卖点 high（不升破前高）→ confirmed
    t1.price 即一类买卖点的极值价（买点=背驰低点，卖点=背驰高点）。
    settled = 覆盖该回调/反弹段的 Move.settled（走势完成验证，事后）。
    """
    assoc_move = _find_move_for_seg(moves, seg_idx)
    price = seg.low if side == "buy" else seg.high
    if side == "buy":
        confirmed = price >= t1.price  # 回调不创新低
    else:
        confirmed = price <= t1.price  # 反弹不创新高
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
        price=price,
        bar_idx=seg.i1,
        confirmed=confirmed,
        settled=assoc_move.settled if assoc_move else False,
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
    moves: list[Move], level_id: int, confirmed: bool,
) -> BuySellPoint:
    """构造 Type3 买卖点。

    candidate-fix（第20课）：第三类买卖点的 candidate / confirmed 时间分离。
    调用方 _detect_type3 仅在 (回试 low > ZG / 回抽 high < ZD) 成立时才构造本对象，
    即"中枢突破后回试不破边界"——此为 candidate 条件（结构已形成）。
    confirmed 由调用方判定：回试段之后出现 break_direction 方向的延续段时为 True
    （回试结束、走势延续 → 回试不破被后续结构确认）；回试仍在进行时为 False（候选）。
    settled = 覆盖该回试/回抽段的 Move.settled（走势完成验证，事后）。
    """
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
        confirmed=confirmed,
        settled=assoc_move.settled if assoc_move else False,
    )


def _detect_type3(
    zhongshus: list[Zhongshu],
    segments: list,
    moves: list[Move],
    level_id: int,
) -> list[BuySellPoint]:
    """第三类买卖点：中枢突破后回试/回抽。

    "第一次"约束（第20课）：必须是第一次离开后的回试/回抽。
    隐式保证：zs.break_seg 是中枢延伸结束后的第一个突破段（zhongshu_v1 构造保证），
    _find_next_seg_by_direction 从 break_seg+1 向后找的是第一个反向段。
    因此每个中枢最多产生一个第三类买卖点。
    """
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

        # candidate-fix: confirmed = 回试段之后出现 break_direction 方向的延续段
        # （回试结束、走势延续 → 回试不破被后续结构确认）。延续段未出现 → 仅 candidate。
        continuation_idx = _find_next_seg_by_direction(
            segments, pullback_idx + 1, zs.break_direction,
        )
        confirmed = continuation_idx is not None

        if zs.break_direction == "up" and pullback_seg.low > zs.zg:
            result.append(_make_type3_point(
                zs, pullback_idx, pullback_seg, "buy", moves, level_id, confirmed,
            ))
        elif zs.break_direction == "down" and pullback_seg.high < zs.zd:
            result.append(_make_type3_point(
                zs, pullback_idx, pullback_seg, "sell", moves, level_id, confirmed,
            ))

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
