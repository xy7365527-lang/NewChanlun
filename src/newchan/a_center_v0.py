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
from dataclasses import dataclass
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

        # ── C1: Z走势段方向一致性验证 ──
        # s1 和 s3 必须方向相同（段交替下自然成立），否则跳过
        s1_dir = getattr(s1, "direction", "")
        s3_dir = getattr(s3, "direction", "")
        if s1_dir and s3_dir and s1_dir != s3_dir:
            logger.warning(
                "Center skip: s1.dir=%s != s3.dir=%s at i=%d", s1_dir, s3_dir, i,
            )
            i += 1
            continue

        # ── ZG/ZD 取 Z走势段（s1 与 s3，方向一致） ──
        zd, zg = _zseg_interval(s1, s3)

        # ── 中枢方向 = Z走势段方向 ──
        direction = getattr(s1, "direction", "")

        # ── 初始 GG/DD/G/D（来自前两个 Z走势段） ──
        gg = max(s1.high, s3.high)
        dd = min(s1.low, s3.low)
        g = min(s1.high, s3.high)   # 初始时 G == ZG
        d = max(s1.low, s3.low)     # 初始时 D == ZD

        seg0 = i
        seg1 = i + 2
        sustain = 0

        # ── B) 延伸 + 回抽确认 ──
        j = i + 3
        while j < n:
            seg_j = segments[j]

            if _has_overlap(zd, zg, seg_j):
                # 段与 [ZD, ZG] 重叠 → 延伸
                seg1 = j
                sustain += 1
                # 若该段是 Z走势段（与中枢方向一致），更新统计量
                if getattr(seg_j, "direction", "") == direction:
                    gg = max(gg, seg_j.high)
                    dd = min(dd, seg_j.low)
                    g = min(g, seg_j.high)
                    d = max(d, seg_j.low)
                j += 1
            else:
                # 该段"离开中枢" → 检查回抽
                # 理论："一个次级别走势离开走势中枢，
                #        其后的次级别回抽走势不重新回到该走势中枢内"
                #        → 中枢才算破坏。
                if j + 1 < n and _has_overlap(zd, zg, segments[j + 1]):
                    # 回抽成功 → 中枢未破坏，继续延伸
                    pullback = segments[j + 1]
                    seg1 = j + 1
                    sustain += 2  # exit + pullback 两段都计入中枢生命期
                    # 更新 Z走势段 统计量
                    for seg_k in (seg_j, pullback):
                        if getattr(seg_k, "direction", "") == direction:
                            gg = max(gg, seg_k.high)
                            dd = min(dd, seg_k.low)
                            g = min(g, seg_k.high)
                            d = max(d, seg_k.low)
                    j = j + 2
                    continue
                else:
                    # 回抽失败或无后续段 → 中枢破坏
                    break

        kind: Literal["candidate", "settled"] = (
            "settled" if sustain >= sustain_m else "candidate"
        )

        # C2: 动态 ZG/ZD = 所有构筑中枢的段的重叠部分
        all_segs = segments[seg0 : seg1 + 1]
        zg_dyn = min(s.high for s in all_segs)
        zd_dyn = max(s.low for s in all_segs)

        centers.append(Center(
            seg0=seg0,
            seg1=seg1,
            low=zd,
            high=zg,
            kind=kind,
            confirmed=True,
            sustain=sustain,
            direction=direction,
            gg=gg,
            dd=dd,
            g=g,
            d=d,
            zg_dynamic=zg_dyn,
            zd_dynamic=zd_dyn,
        ))

        # 推进到中枢终点之后
        i = seg1 + 1

    # ── C) 最后一个中枢 confirmed=False ──
    if centers:
        last = centers[-1]
        centers[-1] = Center(
            seg0=last.seg0, seg1=last.seg1,
            low=last.low, high=last.high,
            kind=last.kind, confirmed=False,
            sustain=last.sustain,
            direction=last.direction,
            gg=last.gg, dd=last.dd,
            g=last.g, d=last.d,
            zg_dynamic=last.zg_dynamic,
            zd_dynamic=last.zd_dynamic,
        )

    return centers
