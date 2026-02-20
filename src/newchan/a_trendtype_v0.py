"""A 系统 — 走势类型实例构造 v0

从 segments（Move[0]）+ centers 构造走势类型实例（趋势/盘整对象）。
确认后的走势类型实例 = Move[k]，可递归参与更高级别中枢构造。

规格引用: docs/chan_spec.md §8 走势类型 TrendType Instance

缠论原文定义（缠中说禅定理思维导图）：
  - 走势类型 = { 趋势, 盘整 }
  - 趋势（对象）：某完成的走势类型至少包含两个以上依次同向的走势中枢
  - 盘整（对象）：某完成的走势类型只包含一个走势中枢
  - 走势分解定理：任何级别的任何走势，都可以分解为同级别
    盘整、下跌与上涨三种走势类型的连接（首尾相连，完全覆盖）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.a_center_v0 import Center
from newchan.a_segment_v0 import Segment


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class TrendTypeInstance:
    """走势类型实例——趋势对象或盘整对象。

    确认后即为 Move[k]，可参与更高级别中枢构造。

    Attributes
    ----------
    kind : ``"trend"`` | ``"consolidation"``
        趋势（≥2 同向中枢）或盘整（1 中枢）。
    direction : ``"up"`` | ``"down"``
        趋势方向。趋势：中枢整体抬升=up，下移=down。
        盘整：取第一段 segment 的方向。
    seg0 : int
        起点 segment index（在 segments 列表中的位置）。
    seg1 : int
        终点 segment index。
    i0 : int
        对应 merged idx 起点（= segments[seg0].i0）。
    i1 : int
        对应 merged idx 终点（= segments[seg1].i1）。
    high : float
        实例内所有 segment 的 high 最大值。
    low : float
        实例内所有 segment 的 low 最小值。
    center_indices : tuple[int, ...]
        包含的 center 索引（在 centers 列表中的位置）。
    confirmed : bool
        最后一个实例 ``False``，其余 ``True``。
    level_id : int
        走势类型所属递归级别（由递归引擎填充）。
    """

    kind: Literal["trend", "consolidation"]
    direction: Literal["up", "down"]
    seg0: int
    seg1: int
    i0: int
    i1: int
    high: float
    low: float
    center_indices: tuple[int, ...]
    confirmed: bool
    # ── 新增字段（有默认值，向下兼容） ──
    level_id: int = 0


# ====================================================================
# 内部工具
# ====================================================================

def _centers_relation(
    c_prev: Center, c_next: Center,
) -> str:
    """判断前后同级别两个中枢的关系。

    严格按缠论定理思维导图 #32-35 使用 GG/DD：

    - 后GG < 前DD → ``"down"`` （下跌及其延续）
    - 后DD > 前GG → ``"up"``   （上涨及其延续）
    - 后ZG < 前ZD 且 后GG >= 前DD → ``"higher_center"`` （形成高级别中枢）
    - 后ZD > 前ZG 且 后DD <= 前GG → ``"higher_center"`` （形成高级别中枢）
    - 其余情况 → ``"none"``

    Parameters
    ----------
    c_prev, c_next : Center
        前后两个 settled 中枢。需有 gg/dd/high(ZG)/low(ZD) 字段。

    Returns
    -------
    ``"up"`` | ``"down"`` | ``"higher_center"`` | ``"none"``
    """
    has_gg_dd = (
        c_prev.gg != 0.0 and c_prev.dd != 0.0
        and c_next.gg != 0.0 and c_next.dd != 0.0
    )

    if has_gg_dd:
        # ── 严格理论判定（思维导图 #32-35） ──
        prev_gg, prev_dd = c_prev.gg, c_prev.dd
        next_gg, next_dd = c_next.gg, c_next.dd

        # 下跌及其延续：后GG < 前DD
        if next_gg < prev_dd:
            return "down"

        # 上涨及其延续：后DD > 前GG
        if next_dd > prev_gg:
            return "up"

        # 形成高级别中枢：
        #   后ZG < 前ZD 且 后GG >= 前DD
        #   或 后ZD > 前ZG 且 后DD <= 前GG
        if c_next.high < c_prev.low and next_gg >= prev_dd:
            return "higher_center"
        if c_next.low > c_prev.high and next_dd <= prev_gg:
            return "higher_center"

        return "none"
    else:
        # ── 兼容路径：GG/DD 缺失时用 ZG/ZD（§8.2 弱化版本） ──
        if c_next.high > c_prev.high and c_next.low > c_prev.low:
            return "up"
        if c_next.high < c_prev.high and c_next.low < c_prev.low:
            return "down"
        return "none"


def _centers_same_direction(
    c_prev: Center, c_next: Center,
) -> str | None:
    """判断两个相邻中枢是否同向（兼容旧接口）。

    内部调用 ``_centers_relation``，将 ``"up"``/``"down"`` 返回，
    ``"higher_center"``/``"none"`` 返回 None。
    """
    rel = _centers_relation(c_prev, c_next)
    if rel in ("up", "down"):
        return rel
    return None


def _group_centers_by_direction(
    centers: list[Center],
) -> list[tuple[list[int], str | None]]:
    """将中枢序列按方向分组。

    Returns
    -------
    list[tuple[list[int], str | None]]
        每个元素 = (center_indices, direction)。
        - 连续同向中枢归入同一组，direction = "up"/"down"
        - 单独的中枢独立成组，direction 取 None（后续由 segment 推导）
    """
    if not centers:
        return []

    # 只处理 settled 中枢（candidate 不参与走势类型构造）
    settled_indices = [i for i, c in enumerate(centers) if c.kind == "settled"]
    if not settled_indices:
        return []

    result: list[tuple[list[int], str | None]] = [([settled_indices[0]], None)]

    for k in range(1, len(settled_indices)):
        ci_prev = settled_indices[k - 1]
        ci_curr = settled_indices[k]
        d = _centers_same_direction(centers[ci_prev], centers[ci_curr])

        if d is not None and (result[-1][1] is None or result[-1][1] == d):
            # 同向 → 归入当前组
            result[-1][0].append(ci_curr)
            result[-1] = (result[-1][0], d)
        else:
            # 方向变化 → 开新组
            result.append(([ci_curr], None))

    return result


def _merge_adjacent_same_direction_trends(
    instances: list[TrendTypeInstance],
) -> list[TrendTypeInstance]:
    """合并相邻且同向的趋势对象，得到极大走势类型实例。

    若出现 ``trend(up)`` 紧邻 ``trend(up)``（或 down/down），
    说明当前分组产生了同向碎片，应并成一个更大的趋势对象。
    """
    if not instances:
        return []

    merged: list[TrendTypeInstance] = [instances[0]]
    for inst in instances[1:]:
        prev = merged[-1]
        can_merge = (
            prev.kind == "trend"
            and inst.kind == "trend"
            and prev.direction == inst.direction
            and prev.seg1 == inst.seg0
        )
        if not can_merge:
            merged.append(inst)
            continue

        merged[-1] = TrendTypeInstance(
            kind="trend",
            direction=prev.direction,
            seg0=prev.seg0,
            seg1=inst.seg1,
            i0=prev.i0,
            i1=inst.i1,
            high=max(prev.high, inst.high),
            low=min(prev.low, inst.low),
            center_indices=prev.center_indices + tuple(
                ci for ci in inst.center_indices if ci not in prev.center_indices
            ),
            confirmed=True,
        )
    return merged


def _resolve_kind_and_direction(
    center_idxs: list[int],
    group_dir: "Literal['up', 'down'] | None",
    centers: list[Center],
    segments: list,
    n_seg: int,
) -> tuple["Literal['trend', 'consolidation']", "Literal['up', 'down']"]:
    """确定走势类型实例的 kind 和 direction。"""
    kind: Literal["trend", "consolidation"] = (
        "trend" if len(center_idxs) >= 2 else "consolidation"
    )
    if group_dir is not None:
        return kind, group_dir
    # 单中枢盘整：取第一段 segment 的方向
    first_center = centers[center_idxs[0]]
    if first_center.seg0 < n_seg:
        return kind, segments[first_center.seg0].direction
    return kind, "up"  # fallback


def _compute_seg_boundaries(
    gi: int,
    center_idxs: list[int],
    centers: list[Center],
    groups: list,
    n_seg: int,
) -> tuple[int, int]:
    """确定 segment 边界 (seg0, seg1)，保证连续性。"""
    seg0 = 0 if gi == 0 else centers[center_idxs[0]].seg0
    if gi == len(groups) - 1:
        seg1 = n_seg - 1
    else:
        next_first_ci = groups[gi + 1][0][0]
        seg1 = centers[next_first_ci].seg0
    seg0 = max(0, min(seg0, n_seg - 1))
    seg1 = max(seg0, min(seg1, n_seg - 1))
    return seg0, seg1


def _compute_instance_metrics(
    segments: list, seg0: int, seg1: int, n_seg: int,
) -> tuple[int, int, float, float, int]:
    """计算 i0, i1, high, low；不足 3 段时扩展 seg1。返回 (i0, i1, high, low, seg1)。"""
    if seg1 - seg0 < 2:
        seg1 = min(seg0 + 2, n_seg - 1)
    i0 = segments[seg0].i0
    i1 = segments[seg1].i1
    seg_slice = segments[seg0 : seg1 + 1]
    high = max(s.high for s in seg_slice) if seg_slice else 0.0
    low = min(s.low for s in seg_slice) if seg_slice else 0.0
    return i0, i1, high, low, seg1


def _mark_last_instance_unconfirmed(
    instances: list[TrendTypeInstance],
) -> list[TrendTypeInstance]:
    """最后一个实例标记为未确认（返回新列表）。"""
    if not instances:
        return instances
    last = instances[-1]
    return instances[:-1] + [TrendTypeInstance(
        kind=last.kind,
        direction=last.direction,
        seg0=last.seg0,
        seg1=last.seg1,
        i0=last.i0,
        i1=last.i1,
        high=last.high,
        low=last.low,
        center_indices=last.center_indices,
        confirmed=False,
    )]


# ====================================================================
# 主函数
# ====================================================================

def trend_instances_from_centers(
    segments: list,
    centers: list[Center],
) -> list[TrendTypeInstance]:
    """从 segments + centers 构造走势类型实例（趋势/盘整对象）。

    Parameters
    ----------
    segments : list[Segment]
    centers : list[Center]

    Returns
    -------
    list[TrendTypeInstance]
        按 seg0 递增排序。最后一个 confirmed=False，其余 True。
        **连续性保证**: instances[i].seg1 == instances[i+1].seg0
    """
    if not segments or not centers:
        return []

    groups = _group_centers_by_direction(centers)
    if not groups:
        return []

    n_seg = len(segments)
    instances: list[TrendTypeInstance] = []

    for gi, (center_idxs, group_dir) in enumerate(groups):
        kind, direction = _resolve_kind_and_direction(
            center_idxs, group_dir, centers, segments, n_seg,
        )
        seg0, seg1 = _compute_seg_boundaries(
            gi, center_idxs, centers, groups, n_seg,
        )
        i0, i1, high, low, seg1 = _compute_instance_metrics(
            segments, seg0, seg1, n_seg,
        )
        instances.append(TrendTypeInstance(
            kind=kind, direction=direction,
            seg0=seg0, seg1=seg1,
            i0=i0, i1=i1,
            high=high, low=low,
            center_indices=tuple(center_idxs),
            confirmed=True,
        ))

    instances = _merge_adjacent_same_direction_trends(instances)
    return _mark_last_instance_unconfirmed(instances)


# ====================================================================
# 中枢发展标签
# ====================================================================

def label_centers_development(
    centers: list[Center],
) -> list[Center]:
    """根据相邻中枢关系，为每个 settled 中枢打上 development 标签。

    规则（§8 中枢中心定理二）：
    - 与后续中枢关系为 "up"/"down" → 本中枢 development = "newborn"（新生：形成趋势）
    - 与后续中枢关系为 "higher_center" → 本中枢 development = "expansion"（扩展）
    - 无后续 settled 中枢且中枢延伸中 → "extension"（延伸）
    - candidate 中枢不标注

    Parameters
    ----------
    centers : list[Center]

    Returns
    -------
    list[Center]
        带 development 标签的新 Center 列表（frozen，返回新对象）。
    """
    if not centers:
        return []

    settled_indices = [i for i, c in enumerate(centers) if c.kind == "settled"]
    result = list(centers)  # shallow copy

    for pos, si in enumerate(settled_indices):
        dev = ""
        if pos < len(settled_indices) - 1:
            # 有后续 settled 中枢 → 用关系判定
            next_si = settled_indices[pos + 1]
            rel = _centers_relation(centers[si], centers[next_si])
            if rel in ("up", "down"):
                dev = "newborn"
            elif rel == "higher_center":
                dev = "expansion"
            else:
                dev = "extension"
        else:
            # 最后一个 settled 中枢
            c = centers[si]
            if c.terminated:
                dev = "newborn"  # 已终结，意味着走势离开形成新结构
            elif c.sustain > 0:
                dev = "extension"

        if dev:
            c = result[si]
            result[si] = Center(
                seg0=c.seg0, seg1=c.seg1, low=c.low, high=c.high,
                kind=c.kind, confirmed=c.confirmed, sustain=c.sustain,
                direction=c.direction,
                gg=c.gg, dd=c.dd, g=c.g, d=c.d,
                zg_dynamic=c.zg_dynamic, zd_dynamic=c.zd_dynamic,
                development=dev,
                level_id=c.level_id,
                terminated=c.terminated,
                termination_side=c.termination_side,
            )

    return result
