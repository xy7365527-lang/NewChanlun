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


def _make_result(
    center_idx: int,
    center: Center,
    is_alive: bool,
    regime: Regime,
    *,
    run_exit_idx: int | None = None,
    run_exit_side: ExitSide | None = None,
    run_exit_extreme: float | None = None,
    event_seen_pullback: bool = False,
    event_pullback_settled: bool = False,
    death_reason: str | None = None,
) -> AliveCenter:
    """构造 AliveCenter，集中 AnchorSet 字段默认值。"""
    return AliveCenter(
        center_idx=center_idx, center=center,
        is_alive=is_alive, regime=regime,
        anchors=AnchorSet(
            settle_core_low=center.low,
            settle_core_high=center.high,
            run_exit_idx=run_exit_idx,
            run_exit_side=run_exit_side,
            run_exit_extreme=run_exit_extreme,
            event_seen_pullback=event_seen_pullback,
            event_pullback_settled=event_pullback_settled,
            death_reason=death_reason,
        ),
    )


def _scan_event_anchor(
    segments: list[Segment],
    exit_idx: int,
    cur_idx: int,
    exit_seg: Segment,
    exit_side: ExitSide,
    exit_extreme: float,
    low: float,
    high: float,
) -> tuple[bool, bool, float]:
    """扫描 exit_idx+1..cur_idx，返回 (seen_pullback, pullback_settled, exit_extreme)。"""
    seen_pullback = False
    pullback_settled = False

    for j in range(exit_idx + 1, cur_idx + 1):
        seg_j = segments[j]

        # 回抽前：同向段推进极值
        if not seen_pullback and seg_j.direction == exit_seg.direction:
            if exit_side == ExitSide.ABOVE:
                exit_extreme = max(exit_extreme, seg_j.high)
            else:
                exit_extreme = min(exit_extreme, seg_j.low)

        # 事件锚触发：反向段 或 触碰核
        if not seen_pullback:
            if seg_j.direction != exit_seg.direction:
                seen_pullback = True
            elif overlap(seg_j.low, seg_j.high, low, high):
                seen_pullback = True

        # 否定条件：回抽后再确认创新高/新低
        if seen_pullback and not pullback_settled:
            if seg_j.direction == exit_seg.direction:
                if (exit_side == ExitSide.ABOVE and seg_j.high > exit_extreme):
                    pullback_settled = True
                elif (exit_side == ExitSide.BELOW and seg_j.low < exit_extreme):
                    pullback_settled = True

    return seen_pullback, pullback_settled, exit_extreme


# ====================================================================
# 1.4 结算锚存活判定
# ====================================================================

def _classify_settle_anchor(
    center: Center, center_idx: int, segments: list[Segment],
    last_price: float, n_seg: int,
) -> AliveCenter | None:
    """结算锚·中枢内判定。返回 AliveCenter 或 None（需继续判定）。"""
    low, high = center.low, center.high
    mk = lambda alive, regime, **kw: _make_result(center_idx, center, alive, regime, **kw)

    if low <= last_price <= high:
        return mk(True, Regime.SETTLE_ANCHOR_IN_CORE)

    cur_idx = n_seg - 1 if n_seg > 0 else -1
    if 0 <= cur_idx <= center.seg1:
        cur_seg = segments[cur_idx]
        if overlap(cur_seg.low, cur_seg.high, low, high):
            return mk(True, Regime.SETTLE_ANCHOR_IN_CORE)

    return None


def _classify_run_anchor(
    center: Center, center_idx: int, segments: list[Segment],
    n_seg: int,
) -> AliveCenter | None:
    """运行锚判定。返回 AliveCenter 或 None（需继续到事件锚）。"""
    low, high = center.low, center.high
    mk = lambda alive, regime, **kw: _make_result(center_idx, center, alive, regime, **kw)

    exit_idx = center.seg1 + 1
    if exit_idx >= n_seg:
        return mk(False, Regime.DEAD_NEGATION_SETTLED, death_reason="no_exit_segment")

    exit_seg = segments[exit_idx]
    if overlap(exit_seg.low, exit_seg.high, low, high):
        return mk(True, Regime.SETTLE_ANCHOR_IN_CORE, run_exit_idx=exit_idx)

    return None


def _determine_exit_side(
    exit_seg: Segment, low: float, high: float,
) -> tuple[ExitSide, float] | None:
    """判定离开方向和初始极值。返回 None 表示无效离开。"""
    if exit_seg.low >= high:
        return ExitSide.ABOVE, exit_seg.high
    if exit_seg.high <= low:
        return ExitSide.BELOW, exit_seg.low
    return None


def _build_event_anchor_result(
    center_idx: int,
    center: Center,
    exit_idx: int,
    exit_seg: Segment,
    exit_side: ExitSide,
    exit_extreme: float,
    segments: list[Segment],
    n_seg: int,
) -> AliveCenter:
    """执行事件锚扫描并构造最终 AliveCenter。"""
    mk = lambda alive, regime, **kw: _make_result(center_idx, center, alive, regime, **kw)
    cur_idx = n_seg - 1 if n_seg > 0 else -1

    seen_pullback, pullback_settled, exit_extreme = _scan_event_anchor(
        segments, exit_idx, cur_idx, exit_seg, exit_side, exit_extreme,
        center.low, center.high,
    )

    if pullback_settled:
        return mk(False, Regime.DEAD_NEGATION_SETTLED,
                  run_exit_idx=exit_idx, run_exit_side=exit_side,
                  run_exit_extreme=exit_extreme,
                  event_seen_pullback=seen_pullback,
                  event_pullback_settled=pullback_settled,
                  death_reason="pullback_settled")

    regime = Regime.EVENT_ANCHOR_FIRST_PULLBACK if seen_pullback else Regime.RUN_ANCHOR_POST_EXIT
    return mk(True, regime,
              run_exit_idx=exit_idx, run_exit_side=exit_side,
              run_exit_extreme=exit_extreme,
              event_seen_pullback=seen_pullback,
              event_pullback_settled=pullback_settled)


def classify_center_practical_newchan(
    center: Center,
    center_idx: int,
    segments: list[Segment],
    last_price: float,
) -> AliveCenter:
    """对单个中枢做三锚存活判定。"""
    n_seg = len(segments)
    mk = lambda alive, regime, **kw: _make_result(center_idx, center, alive, regime, **kw)

    if center.kind != "settled":
        return mk(False, Regime.DEAD_NOT_SETTLED, death_reason="not_settled")

    result = _classify_settle_anchor(center, center_idx, segments, last_price, n_seg)
    if result is not None:
        return result

    result = _classify_run_anchor(center, center_idx, segments, n_seg)
    if result is not None:
        return result

    exit_idx = center.seg1 + 1
    exit_seg = segments[exit_idx]
    exit_info = _determine_exit_side(exit_seg, center.low, center.high)
    if exit_info is None:
        return mk(False, Regime.DEAD_NEGATION_SETTLED, death_reason="invalid_exit_side")

    exit_side, exit_extreme = exit_info
    return _build_event_anchor_result(
        center_idx, center, exit_idx, exit_seg,
        exit_side, exit_extreme, segments, n_seg,
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
