"""A 系统 — 中枢 v0（只吃 Segment / Move[0]，严禁笔）

从 Segment 序列构造中枢。

理论依据（思维导图-缠中说禅定理）：
  - 中枢 = 某级别的走势类型，被至少三个连续次级别走势类型所重叠的部分
  - Z走势段 = 与中枢方向一致的次级别走势类型
  - ZG = min(g1, g2)  （前两个 Z走势段的高点取 min）
  - ZD = max(d1, d2)  （前两个 Z走势段的低点取 max）
  - GG = max(gn), G = min(gn), DD = min(dn), D = max(dn)
  - 延伸: 任意区间 [dn, gn] 与 [ZD, ZG] 有重叠
  - 破坏: 一个次级别走势离开中枢，其后的次级别回抽走势不重新回到中枢内

规格引用: docs/chan_spec.md §7 中枢（Center / Zhongshu）
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace
from typing import Literal

from newchan.a_segment_v0 import Segment

logger = logging.getLogger(__name__)


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class Center:
    """一个中枢。

    Attributes
    ----------
    seg0 : int
        起点 segment index（segments 列表中的位置）。
    seg1 : int
        终点 segment index（初始 = seg0+2，延伸后可更大）。
    low : float
        ZD（中枢区间下沿），由初始 Z走势段 确定，延伸不改变。
    high : float
        ZG（中枢区间上沿），由初始 Z走势段 确定，延伸不改变。
    kind : ``"candidate"`` | ``"settled"``
        candidate → 刚由三段重叠形成；
        settled → sustain >= sustain_m（延伸得到确认）。
    confirmed : bool
        最后一个中枢 ``False``，其余 ``True``。
    sustain : int
        已持续重叠的额外 move 数（不含初始三段）。
    direction : str
        中枢方向（初始三段中 Z走势段 的方向，即 s1/s3 的方向）。
    gg : float
        GG = max(gn)，所有 Z走势段 高点的最大值。
    dd : float
        DD = min(dn)，所有 Z走势段 低点的最小值。
    g : float
        G = min(gn)，所有 Z走势段 高点的最小值。
    d : float
        D = max(dn)，所有 Z走势段 低点的最大值。
    development : ``""`` | ``"extension"`` | ``"newborn"`` | ``"expansion"``
        中枢发展类型（§8）：延伸/新生/扩展。
        由 ``label_centers_development()`` 在走势类型构造后填充。
    level_id : int
        中枢所属递归级别（由递归引擎填充）。
    terminated : bool
        中枢是否已终结（次级别走势离开后回抽不回）。
    termination_side : ``""`` | ``"above"`` | ``"below"``
        终结方向：从上方还是下方离开。
    """

    seg0: int
    seg1: int
    low: float
    high: float
    kind: Literal["candidate", "settled"]
    confirmed: bool
    sustain: int
    # ── 新增字段（有默认值，向下兼容旧构造调用） ──
    direction: str = ""
    gg: float = 0.0
    dd: float = 0.0
    g: float = 0.0
    d: float = 0.0
    # C2: 第二种区间定义——所有构筑段的重叠部分
    # 原文L46："由所有构筑中枢的线段的重叠部分确定中枢区间"
    zg_dynamic: float = 0.0  # 所有段 high 的 min
    zd_dynamic: float = 0.0  # 所有段 low 的 max
    # ── 中枢发展类型（§8 延伸/新生/扩展） ──
    development: Literal["extension", "newborn", "expansion", ""] = ""
    # ── 中枢所属级别（递归层级） ──
    level_id: int = 0
    # ── 中枢终结标记（§8 中枢终结定理） ──
    terminated: bool = False
    termination_side: Literal["above", "below", ""] = ""


# ====================================================================
# 内部工具
# ====================================================================

def _has_overlap(low: float, high: float, seg) -> bool:
    """判断 segment 的 [seg.low, seg.high] 与 [low, high] 是否有严格交集。"""
    return max(low, seg.low) < min(high, seg.high)


def _three_seg_overlap_all(s1, s2, s3) -> tuple[float, float]:
    """三段全部重叠的 (ZD_all, ZG_all)，用于中枢成立判定。"""
    return max(s1.low, s2.low, s3.low), min(s1.high, s2.high, s3.high)


def _zseg_interval(s1, s3) -> tuple[float, float]:
    """Z走势段 (s1 与 s3，方向一致) 的 ZD, ZG。

    理论依据：ZG = min(g1, g2), ZD = max(d1, d2)。
    """
    return max(s1.low, s3.low), min(s1.high, s3.high)


def _update_zseg_stats(
    gg: float, dd: float, g: float, d: float,
    seg, direction: str,
) -> tuple[float, float, float, float]:
    """若 seg 是 Z走势段（方向一致），更新 GG/DD/G/D 统计量。"""
    if getattr(seg, "direction", "") != direction:
        return gg, dd, g, d
    return (
        max(gg, seg.high),
        min(dd, seg.low),
        min(g, seg.high),
        max(d, seg.low),
    )


def _build_center(
    seg0: int, seg1: int, zd: float, zg: float,
    sustain: int, sustain_m: int, direction: str,
    gg: float, dd: float, g: float, d: float,
    segments: list, terminated: bool,
    term_side: Literal["above", "below", ""],
) -> Center:
    """构造 Center，含动态 ZG/ZD 计算。"""
    all_segs = segments[seg0 : seg1 + 1]
    return Center(
        seg0=seg0, seg1=seg1, low=zd, high=zg,
        kind="settled" if sustain >= sustain_m else "candidate",
        confirmed=True,
        sustain=sustain,
        direction=direction,
        gg=gg, dd=dd, g=g, d=d,
        zg_dynamic=min(s.high for s in all_segs),
        zd_dynamic=max(s.low for s in all_segs),
        terminated=terminated,
        termination_side=term_side,
    )


def _extend_center(
    segments: list, start_j: int, n: int,
    zd: float, zg: float, direction: str,
    gg: float, dd: float, g: float, d: float,
) -> tuple[int, int, float, float, float, float, bool, Literal["above", "below", ""]]:
    """延伸 + 回抽确认循环。

    Returns (seg1, sustain, gg, dd, g, d, terminated, term_side)。
    """
    seg1 = start_j - 1  # 初始 seg1 = i + 2
    sustain = 0
    terminated = False
    term_side: Literal["above", "below", ""] = ""

    j = start_j
    while j < n:
        seg_j = segments[j]

        if _has_overlap(zd, zg, seg_j):
            seg1 = j
            sustain += 1
            gg, dd, g, d = _update_zseg_stats(gg, dd, g, d, seg_j, direction)
            j += 1
            continue

        # 该段"离开中枢" → 检查回抽
        if j + 1 < n and _has_overlap(zd, zg, segments[j + 1]):
            pullback = segments[j + 1]
            seg1 = j + 1
            sustain += 2
            for seg_k in (seg_j, pullback):
                gg, dd, g, d = _update_zseg_stats(gg, dd, g, d, seg_k, direction)
            j += 2
            continue

        # 回抽失败或无后续段 → 中枢破坏
        if j + 1 < n:
            terminated = True
            if seg_j.low >= zg:
                term_side = "above"
            elif seg_j.high <= zd:
                term_side = "below"
        break

    return seg1, sustain, gg, dd, g, d, terminated, term_side


# ====================================================================
# v0 主函数
# ====================================================================

def centers_from_segments_v0(
    segments: list,
    sustain_m: int = 2,
) -> list[Center]:
    """v0 中枢构造，严格遵循缠论原文。

    Parameters
    ----------
    segments : list
        Move 对象列表（level=1 时为 Segment，level>=2 时为 TrendTypeInstance）。
        每个元素需有 ``.high``, ``.low``, ``.direction`` 属性。
    sustain_m : int
        candidate 升级为 settled 所需的延伸次数（默认 2）。

    Returns
    -------
    list[Center]
        按 ``seg0`` 递增排序。最后一个 ``confirmed=False``，其余 ``True``。

    Notes
    -----
    算法（严格对标缠论定理思维导图）：

    A) 初始候选：
       连续三段全部存在重叠（max(lows) < min(highs)）→ 中枢成立。
       ZG/ZD 取 Z走势段（s1 与 s3，方向一致的那两段）定义。

    B) 延伸 + 回抽确认（中枢中心定理 + 中枢破坏定理）：
       后续段与 [ZD, ZG] 有交集 → 延伸。
       无交集 → 该段"离开中枢"：
         - 若下一段（回抽）重返 [ZD, ZG] → 中枢未破坏，继续延伸。
         - 若下一段仍不重返 → 中枢破坏，终止。

    C) confirmed：最后一个 False，其余 True。

    硬约束（§7.1）：
       输入必须是 Move[k-1]（Segment 或已确认的 TrendTypeInstance），禁止 Stroke。
    """
    n = len(segments)
    if n < 3:
        return []

    centers: list[Center] = []
    i = 0

    while i <= n - 3:
        s1, s2, s3 = segments[i], segments[i + 1], segments[i + 2]

        # ── A) 三段全部重叠才成立 ──
        zd_all, zg_all = _three_seg_overlap_all(s1, s2, s3)
        if zg_all <= zd_all:
            i += 1
            continue

        # ── Z走势段方向一致性验证 ──
        s1_dir = getattr(s1, "direction", "")
        s3_dir = getattr(s3, "direction", "")
        if s1_dir and s3_dir and s1_dir != s3_dir:
            logger.warning(
                "Center skip: s1.dir=%s != s3.dir=%s at i=%d", s1_dir, s3_dir, i,
            )
            i += 1
            continue

        zd, zg = _zseg_interval(s1, s3)
        direction = getattr(s1, "direction", "")
        gg = max(s1.high, s3.high)
        dd = min(s1.low, s3.low)
        g = min(s1.high, s3.high)
        d = max(s1.low, s3.low)

        # ── B) 延伸 + 回抽确认 ──
        seg1, sustain, gg, dd, g, d, terminated, term_side = _extend_center(
            segments, i + 3, n, zd, zg, direction, gg, dd, g, d,
        )
        seg1 = max(seg1, i + 2)  # 至少包含初始三段

        centers.append(_build_center(
            i, seg1, zd, zg, sustain, sustain_m, direction,
            gg, dd, g, d, segments, terminated, term_side,
        ))
        i = seg1 + 1

    # ── C) 最后一个中枢 confirmed=False ──
    if centers:
        centers[-1] = replace(centers[-1], confirmed=False)

    return centers
