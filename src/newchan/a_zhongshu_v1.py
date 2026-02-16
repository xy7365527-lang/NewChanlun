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


def zhongshu_from_segments(segments: list[Segment]) -> list[Zhongshu]:
    """从线段列表计算中枢。

    只处理 confirmed=True 的段。

    算法：
    1. 过滤已确认段
    2. 滑窗扫描连续三段：ZD=max(lows), ZG=min(highs)
    3. ZG > ZD → 中枢成立
    4. 延伸：后续段与 [ZD, ZG] 有交集，同时更新 GG/DD
    5. 突破：不重叠的段终结中枢，settled=True
    6. 续进：从 break_seg_idx - 2 开始扫描下一个中枢

    Parameters
    ----------
    segments : list[Segment]
        线段列表（含 confirmed 和 unconfirmed）。

    Returns
    -------
    list[Zhongshu]
        按 seg_start 递增排序的中枢列表。
    """
    confirmed = [s for s in segments if s.confirmed]
    n = len(confirmed)
    if n < 3:
        return []

    result: list[Zhongshu] = []
    i = 0

    while i + 2 < n:
        s1, s2, s3 = confirmed[i], confirmed[i + 1], confirmed[i + 2]

        # 三段重叠检测
        zd = max(s1.low, s2.low, s3.low)
        zg = min(s1.high, s2.high, s3.high)

        if zg <= zd:
            # 无重叠，前进一步
            i += 1
            continue

        # 中枢成立：[ZD, ZG] 固定；波动区间初始化
        gg = max(s1.high, s2.high, s3.high)
        dd = min(s1.low, s2.low, s3.low)
        seg_end_idx = i + 2

        # 尝试延伸
        j = i + 3
        while j < n:
            sj = confirmed[j]
            # 延伸判定：与 [ZD, ZG] 有交集（中心定理一，弱不等式）
            if sj.high >= zd and sj.low <= zg:
                seg_end_idx = j
                gg = max(gg, sj.high)
                dd = min(dd, sj.low)
                j += 1
            else:
                break

        # 判断是否已被突破
        settled = j < n
        break_seg_idx = j if settled else -1

        # 突破方向
        break_dir = ""
        if settled:
            breaker = confirmed[j]
            if breaker.low > zg:
                break_dir = "up"
            elif breaker.high < zd:
                break_dir = "down"
            else:
                # 理论上不应出现（延伸用弱不等式后突破必为严格不等），做防御
                break_dir = "up" if breaker.high > zg else "down"

        result.append(Zhongshu(
            zd=zd,
            zg=zg,
            seg_start=i,
            seg_end=seg_end_idx,
            seg_count=seg_end_idx - i + 1,
            settled=settled,
            break_seg=break_seg_idx,
            break_direction=break_dir,
            first_seg_s0=confirmed[i].s0,
            last_seg_s1=confirmed[seg_end_idx].s1,
            gg=gg,
            dd=dd,
        ))

        # 续进策略：从 break_seg_idx - 2 开始
        if settled:
            i = max(break_seg_idx - 2, seg_end_idx)
        else:
            break  # 最后一个中枢未闭合，结束

    return result
