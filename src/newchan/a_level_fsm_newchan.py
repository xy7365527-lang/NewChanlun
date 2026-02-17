"""A 系统 — 三锚体系 L* 裁决（新缠论语汇）

结算锚 · 运行锚 · 事件锚 下的中枢存活判定与唯一裁决级别 L* 选择。

实盘口径：
  结算锚活着 = 中枢内 + 离开段 + 第一次回抽未结算
  死亡 = 回抽结算完成 / 超时否定 / 未结算候选中枢

规格引用: docs/chan_spec.md §9 级别 Level 与 L* 锁定
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Literal

from newchan.a_center_v0 import Center
from newchan.a_segment_v0 import Segment


# ====================================================================
# 1.1 枚举
# ====================================================================

class Regime(str, Enum):
    """三锚状态机状态。"""
    SETTLE_ANCHOR_IN_CORE = "结算锚·中枢内"
    RUN_ANCHOR_POST_EXIT = "运行锚·离开段"
    EVENT_ANCHOR_FIRST_PULLBACK = "事件锚·第一次回抽"
    DEAD_NEGATION_SETTLED = "死亡·否定性已结算"
    DEAD_NOT_SETTLED = "死亡·未结算中枢"


class ExitSide(str, Enum):
    """离开方向。"""
    ABOVE = "above"
    BELOW = "below"


# ====================================================================
# 1.2 输出对象
# ====================================================================

@dataclass(frozen=True, slots=True)
class AnchorSet:
    """锚集合——记录中枢的完整判定状态。"""
    settle_core_low: float
    settle_core_high: float
    run_exit_idx: int | None
    run_exit_side: ExitSide | None
    run_exit_extreme: float | None
    event_seen_pullback: bool
    event_pullback_settled: bool
    death_reason: str | None


@dataclass(frozen=True, slots=True)
class AliveCenter:
    """一个中枢的存活判定结果。"""
    center_idx: int
    center: Center
    is_alive: bool
    regime: Regime
    anchors: AnchorSet


@dataclass(frozen=True, slots=True)
class LevelView:
    """单级别视图。segments 为 Move[k-1] 列表（Segment 或 TrendTypeInstance）。"""
    level: int
    segments: list
    centers: list[Center]


@dataclass(frozen=True, slots=True)
class LStar:
    """唯一裁决级别输出。"""
    level: int
    center_idx: int
    regime: Regime


# ====================================================================
# 1.3 工具函数
# ====================================================================

def overlap(seg_low: float, seg_high: float,
            zlow: float, zhigh: float) -> bool:
    """交集非空判定。"""
    return max(seg_low, zlow) < min(seg_high, zhigh)


# ====================================================================
# 1.4 结算锚存活判定
# ====================================================================

def classify_center_practical_newchan(
    center: Center,
    center_idx: int,
    segments: list[Segment],
    last_price: float,
) -> AliveCenter:
    """对单个中枢做三锚存活判定。

    Parameters
    ----------
    center : Center
    center_idx : int
        center 在 centers 列表中的位置。
    segments : list[Segment]
    last_price : float
        最新价格。

    Returns
    -------
    AliveCenter
    """
    low, high = center.low, center.high
    n_seg = len(segments)

    def _dead(regime: Regime, reason: str) -> AliveCenter:
        return AliveCenter(
            center_idx=center_idx, center=center,
            is_alive=False, regime=regime,
            anchors=AnchorSet(
                settle_core_low=low, settle_core_high=high,
                run_exit_idx=None, run_exit_side=None,
                run_exit_extreme=None,
                event_seen_pullback=False,
                event_pullback_settled=False,
                death_reason=reason,
            ),
        )

    # ── A) candidate 直接死亡 ──
    if center.kind != "settled":
        return _dead(Regime.DEAD_NOT_SETTLED, "not_settled")

    # ── B) 结算锚·中枢内 ──
    # 条件1: last_price 在核内
    if low <= last_price <= high:
        return AliveCenter(
            center_idx=center_idx, center=center,
            is_alive=True, regime=Regime.SETTLE_ANCHOR_IN_CORE,
            anchors=AnchorSet(
                settle_core_low=low, settle_core_high=high,
                run_exit_idx=None, run_exit_side=None,
                run_exit_extreme=None,
                event_seen_pullback=False,
                event_pullback_settled=False,
                death_reason=None,
            ),
        )

    # 条件2: 当前 segment 与核重叠且 index <= seg1
    cur_idx = n_seg - 1 if n_seg > 0 else -1
    if 0 <= cur_idx <= center.seg1:
        cur_seg = segments[cur_idx]
        if overlap(cur_seg.low, cur_seg.high, low, high):
            return AliveCenter(
                center_idx=center_idx, center=center,
                is_alive=True, regime=Regime.SETTLE_ANCHOR_IN_CORE,
                anchors=AnchorSet(
                    settle_core_low=low, settle_core_high=high,
                    run_exit_idx=None, run_exit_side=None,
                    run_exit_extreme=None,
                    event_seen_pullback=False,
                    event_pullback_settled=False,
                    death_reason=None,
                ),
            )

    # ── C) 运行锚判定 ──
    exit_idx = center.seg1 + 1
    if exit_idx >= n_seg:
        return _dead(Regime.DEAD_NEGATION_SETTLED, "no_exit_segment")

    exit_seg = segments[exit_idx]

    # guard: 若离开段仍与核重叠 → 回退到中枢内
    if overlap(exit_seg.low, exit_seg.high, low, high):
        return AliveCenter(
            center_idx=center_idx, center=center,
            is_alive=True, regime=Regime.SETTLE_ANCHOR_IN_CORE,
            anchors=AnchorSet(
                settle_core_low=low, settle_core_high=high,
                run_exit_idx=exit_idx, run_exit_side=None,
                run_exit_extreme=None,
                event_seen_pullback=False,
                event_pullback_settled=False,
                death_reason=None,
            ),
        )

    # exit_side 判定
    if exit_seg.low >= high:
        exit_side = ExitSide.ABOVE
        exit_extreme = exit_seg.high
    elif exit_seg.high <= low:
        exit_side = ExitSide.BELOW
        exit_extreme = exit_seg.low
    else:
        return _dead(Regime.DEAD_NEGATION_SETTLED, "invalid_exit_side")

    # 扫描 exit_idx+1 .. cur_idx（对象事件驱动，无超时）
    seen_pullback = False
    pullback_settled = False

    for j in range(exit_idx + 1, cur_idx + 1):
        seg_j = segments[j]

        # 事件锚触发：反向段 或 触碰核
        if not seen_pullback:
            if seg_j.direction != exit_seg.direction:
                seen_pullback = True
            elif overlap(seg_j.low, seg_j.high, low, high):
                seen_pullback = True

        # 否定条件：回抽后再确认创新高/新低
        if seen_pullback and not pullback_settled:
            if (
                exit_side == ExitSide.ABOVE
                and seg_j.direction == exit_seg.direction
                and seg_j.high > exit_extreme
            ):
                pullback_settled = True
            elif (
                exit_side == ExitSide.BELOW
                and seg_j.direction == exit_seg.direction
                and seg_j.low < exit_extreme
            ):
                pullback_settled = True

    # regime 输出
    if pullback_settled:
        regime = Regime.DEAD_NEGATION_SETTLED
        death_reason: str | None = "pullback_settled"
        is_alive = False
    elif seen_pullback:
        regime = Regime.EVENT_ANCHOR_FIRST_PULLBACK
        death_reason = None
        is_alive = True
    else:
        regime = Regime.RUN_ANCHOR_POST_EXIT
        death_reason = None
        is_alive = True

    return AliveCenter(
        center_idx=center_idx, center=center,
        is_alive=is_alive, regime=regime,
        anchors=AnchorSet(
            settle_core_low=low, settle_core_high=high,
            run_exit_idx=exit_idx, run_exit_side=exit_side,
            run_exit_extreme=exit_extreme,
            event_seen_pullback=seen_pullback,
            event_pullback_settled=pullback_settled,
            death_reason=death_reason,
        ),
    )


# ====================================================================
# 1.5 L* 选择（唯一裁决级别）
# ====================================================================

def select_lstar_newchan(
    level_views: list[LevelView],
    last_price: float,
) -> LStar | None:
    """选择唯一裁决级别 L*。

    按 level 从大到小扫描，找到第一个有 alive center 的 level。
    若有多个 alive center，取最新者（seg1 最大；若相等取 seg0 最大）。

    Returns
    -------
    LStar | None
    """
    # 按 level 降序
    sorted_views = sorted(level_views, key=lambda v: v.level, reverse=True)

    for view in sorted_views:
        alive_list: list[AliveCenter] = []

        for ci, center in enumerate(view.centers):
            ac = classify_center_practical_newchan(
                center=center,
                center_idx=ci,
                segments=view.segments,
                last_price=last_price,
            )
            if ac.is_alive:
                alive_list.append(ac)

        if not alive_list:
            continue

        # 最新者：seg1 最大，若相等 seg0 最大
        best = max(alive_list, key=lambda a: (a.center.seg1, a.center.seg0))
        return LStar(
            level=view.level,
            center_idx=best.center_idx,
            regime=best.regime,
        )

    return None
