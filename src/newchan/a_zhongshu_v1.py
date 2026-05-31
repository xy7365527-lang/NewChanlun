"""A 系统 — 中枢 v1（三段重叠法）

从已确认线段列表计算中枢，或从已确认笔列表计算笔中枢。

核心规则（冻结 v1 spec）：
- 中枢 = 至少 3 段连续已确认组件的价格区间重叠
- ZD = max(seg_i.low)，ZG = min(seg_i.high)，ZG > ZD 严格成立
- 固定区间：初始 3 段确定 [ZD, ZG] 后，延伸段只判重叠不改区间
- 波动区间：GG = max(所有段 high)，DD = min(所有段 low)
- 续进：突破后从 break_seg_idx - 2 开始扫描下一个中枢

两条构造路径（共享滑窗算法 `_scan_zhongshu`）：

1. ``zhongshu_from_segments``（线段中枢，107号 C1）：
   组件 = 已完成的线段（次级别走势类型），过滤 ``confirmed AND kind=="settled"``。
   这是缠师原文第17课递归定义的第一递归层级中枢。

2. ``zhongshu_from_strokes``（笔中枢，[新缠论:选择]）：
   组件 = 已确认的笔，过滤 ``confirmed``。把笔当作"最低不可分解级别的单位"，
   类比第17课"对最后不能分解的级别……定义为至少三个该级别单位K线重叠部分"——
   把"单位"从 K 线提升为笔。笔中枢不违反 107号"笔不裁决"（后者禁止笔作为**段**中枢组件），
   它是不同级别的退化基底中枢，可向上构成笔级别走势，再作为更高级别中枢的组件。
   谱系：settled/525-stroke-center-recursion-base.md。
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Literal, Protocol, Sequence

from newchan.a_segment_v0 import Segment
from newchan.a_stroke import Stroke


@dataclass(frozen=True, slots=True)
class Zhongshu:
    """一个中枢实例（线段中枢或笔中枢共用）。

    Attributes
    ----------
    zd : float
        中枢下沿 = max(初始3段的 low)。
    zg : float
        中枢上沿 = min(初始3段的 high)。
    seg_start : int
        第一个组件在已确认组件列表中的索引。
    seg_end : int
        最后包含组件的索引（含延伸段）。
    seg_count : int
        构成组件数 (>= 3)。
    settled : bool
        True = 已被突破组件闭合。
    break_seg : int
        突破组件在已确认列表中的索引（-1 = 未闭合）。
    break_direction : str
        突破方向："up" / "down" / ""（未闭合时为空）。
    first_seg_s0 : int
        第一个组件的起点时间锚（线段路径=stroke s0；笔路径=merged idx i0）。
    last_seg_s1 : int
        最后包含组件的终点时间锚（线段路径=stroke s1；笔路径=merged idx i1）。
    gg : float
        波动区间上界 GG = max(所有构成组件的 high)。
    dd : float
        波动区间下界 DD = min(所有构成组件的 low)。
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


class _RangeComponent(Protocol):
    """中枢组件的最小价格区间接口（线段/笔均满足）。"""

    @property
    def high(self) -> float: ...

    @property
    def low(self) -> float: ...


def _extend_zhongshu(
    confirmed: Sequence[_RangeComponent], i: int, n: int, zd: float, zg: float,
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


def _break_direction(breaker: _RangeComponent, zg: float, zd: float) -> str:
    """判断突破方向。"""
    if breaker.low > zg:
        return "up"
    if breaker.high < zd:
        return "down"
    return "up" if breaker.high > zg else "down"


def _scan_zhongshu(
    confirmed: Sequence[_RangeComponent],
    start_anchor: Callable[[_RangeComponent], int],
    end_anchor: Callable[[_RangeComponent], int],
) -> list[Zhongshu]:
    """共享滑窗算法：三段重叠 → 延伸 → 突破 → 续进（break_seg_idx - 2）。

    ``start_anchor`` / ``end_anchor`` 从组件提取前端时间定位锚点
    （线段=s0/s1 笔索引；笔=i0/i1 merged idx），是线段中枢与笔中枢路径的唯一差异。
    """
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
            first_seg_s0=start_anchor(confirmed[i]),
            last_seg_s1=end_anchor(confirmed[seg_end_idx]),
            gg=gg, dd=dd,
        ))

        if settled:
            i = max(break_seg_idx - 2, seg_end_idx)
        else:
            break

    return result


def zhongshu_from_segments(segments: list[Segment]) -> list[Zhongshu]:
    """从线段列表计算线段中枢（107号 C1：组件 = 次级别走势类型）。

    严格过滤：confirmed AND settled（与递归路径 SegmentAsComponent.completed 一致）。
    confirmed=True 排除最后一段；kind="settled" 排除未经结算锚验证的候选段。
    """
    confirmed = [s for s in segments if s.confirmed and s.kind == "settled"]
    return _scan_zhongshu(confirmed, lambda s: s.s0, lambda s: s.s1)


def zhongshu_from_strokes(strokes: list[Stroke]) -> list[Zhongshu]:
    """从笔列表计算笔中枢（[新缠论:选择]：笔 = 最低不可分解级别的单位）。

    过滤：confirmed（笔无 settled/candidate 二态，最后一笔 confirmed=False 即排除）。
    时间锚点用笔端点 merged idx（i0/i1）——笔无更低的 stroke-index 层。

    谱系：settled/525-stroke-center-recursion-base.md（笔中枢作为退化基底中枢，
    不违反 107号"笔不裁决"——后者禁止笔作为线段中枢组件，本路径笔是终端递归单位）。
    """
    confirmed = [s for s in strokes if s.confirmed]
    return _scan_zhongshu(confirmed, lambda s: s.i0, lambda s: s.i1)
