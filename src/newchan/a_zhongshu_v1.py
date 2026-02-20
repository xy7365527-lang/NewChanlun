"""A 系统 — 中枢 v1（三段重叠法）

从已确认线段列表计算中枢。

核心规则（冻结 v1 spec）：
- 中枢 = 至少 3 段连续已确认线段的价格区间重叠
- ZD = max(seg_i.low)，ZG = min(seg_i.high)，ZG > ZD 严格成立
- 固定区间：初始 3 段确定 [ZD, ZG] 后，延伸段只判重叠不改区间
- 波动区间：GG = max(所有段 high)，DD = min(所有段 low)
- 续进：突破后从 break_seg_idx - 2 开始扫描下一个中枢
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.a_segment_v0 import Segment


@dataclass(frozen=True, slots=True)
class Zhongshu:
    """一个中枢实例。

    Attributes
    ----------
    zd : float
        中枢下沿 = max(初始3段的 low)。
    zg : float
        中枢上沿 = min(初始3段的 high)。
    seg_start : int
        第一段在已确认段列表中的索引。
    seg_end : int
        最后包含段的索引（含延伸段）。
    seg_count : int
        构成段数 (>= 3)。
    settled : bool
        True = 已被突破段闭合。
    break_seg : int
        突破段在已确认段列表中的索引（-1 = 未闭合）。
    break_direction : str
        突破方向："up" / "down" / ""（未闭合时为空）。
    first_seg_s0 : int
        第一段的 stroke s0（用于前端时间定位）。
    last_seg_s1 : int
        最后包含段的 stroke s1。
    gg : float
        波动区间上界 GG = max(所有构成段的 high)。
    dd : float
        波动区间下界 DD = min(所有构成段的 low)。
    """

    zd: float
    zg: float
    seg_start: int
    seg_end: int
    seg_count: int
    settled: bool
    break_seg: int = -1
    break_direction: str = ""
    first_seg_s0: int = 0
    last_seg_s1: int = 0
    gg: float = 0.0   # max(所有段的 high)，波动区间上界
    dd: float = 0.0   # min(所有段的 low)，波动区间下界


def _extend_zhongshu(
    confirmed: list[Segment], i: int, n: int, zd: float, zg: float,
) -> tuple[int, int, float, float]:
    """从初始三段 (i, i+1, i+2) 开始尝试延伸中枢。

    返回 (seg_end_idx, j, gg, dd)，其中 j 是第一个不重叠段的索引。
    """
    gg = max(confirmed[i].high, confirmed[i + 1].high, confirmed[i + 2].high)
    dd = min(confirmed[i].low, confirmed[i + 1].low, confirmed[i + 2].low)
    seg_end_idx = i + 2
    j = i + 3
    while j < n:
        sj = confirmed[j]
        if sj.high >= zd and sj.low <= zg:
            seg_end_idx = j
            gg = max(gg, sj.high)
            dd = min(dd, sj.low)
            j += 1
        else:
            break
    return seg_end_idx, j, gg, dd


def _break_direction(breaker: Segment, zg: float, zd: float) -> str:
    """判断突破方向。"""
    if breaker.low > zg:
        return "up"
    if breaker.high < zd:
        return "down"
    return "up" if breaker.high > zg else "down"


def zhongshu_from_segments(segments: list[Segment]) -> list[Zhongshu]:
    """从线段列表计算中枢（只处理 confirmed=True 的段）。

    算法：滑窗三段重叠 → 延伸 → 突破 → 续进（break_seg_idx - 2）。
    """
    confirmed = [s for s in segments if s.confirmed]
    n = len(confirmed)
    if n < 3:
        return []

    result: list[Zhongshu] = []
    i = 0

    while i + 2 < n:
        s1, s2, s3 = confirmed[i], confirmed[i + 1], confirmed[i + 2]
        zd = max(s1.low, s2.low, s3.low)
        zg = min(s1.high, s2.high, s3.high)

        if zg <= zd:
            i += 1
            continue

        seg_end_idx, j, gg, dd = _extend_zhongshu(confirmed, i, n, zd, zg)
        settled = j < n
        break_seg_idx = j if settled else -1
        break_dir = _break_direction(confirmed[j], zg, zd) if settled else ""

        result.append(Zhongshu(
            zd=zd, zg=zg, seg_start=i, seg_end=seg_end_idx,
            seg_count=seg_end_idx - i + 1, settled=settled,
            break_seg=break_seg_idx, break_direction=break_dir,
            first_seg_s0=confirmed[i].s0, last_seg_s1=confirmed[seg_end_idx].s1,
            gg=gg, dd=dd,
        ))

        if settled:
            i = max(break_seg_idx - 2, seg_end_idx)
        else:
            break

    return result
