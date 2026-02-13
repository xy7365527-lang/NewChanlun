"""A 系统 — 线段 v0（三笔交集重叠法）

最简占位实现：扫描连续三笔，若三笔交集重叠成立则生成一段。
后续 v1 将用特征序列法替换。

规格引用: docs/chan_spec.md §5 线段（Segment）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Literal

from newchan.a_stroke import Stroke


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class BreakEvidence:
    """线段断段证据。

    记录特征序列分型触发时的关键信息，用于审计和上层裁决。

    Attributes
    ----------
    trigger_stroke_k : int
        分型中心 b 对应的反向笔索引（stroke index）。
    fractal_abc : tuple[int, int, int]
        分型三元组在标准特征序列中的位置索引 (a, b, c)。
    gap_type : ``"none"`` | ``"first"`` | ``"second"``
        缺口类型：none=第一种(无缺口)，second=第二种(有缺口)。
    """

    trigger_stroke_k: int
    fractal_abc: tuple[int, int, int]
    gap_type: Literal["none", "second"]


@dataclass(frozen=True, slots=True)
class Segment:
    """一段线段。

    Attributes
    ----------
    s0 : int
        起点 stroke index（在 strokes 列表中的位置）。
    s1 : int
        终点 stroke index。
    i0 : int
        对应 merged idx 起点（= strokes[s0].i0）。
    i1 : int
        对应 merged idx 终点（= strokes[s1].i1）。
    direction : ``"up"`` | ``"down"``
        v0 约定：第一笔方向即线段方向（§5.2）。
    high : float
        段内所有笔 high 的最大值。
    low : float
        段内所有笔 low 的最小值。
    confirmed : bool
        最后一段 ``False``，其余 ``True``（§5.3）。
    kind : ``"candidate"`` | ``"settled"``
        ``"settled"`` = 已被后续新段确认（结算锚验证通过）；
        ``"candidate"`` = 最后一段，尚未被确认（默认 ``"settled"`` 兼容 v0）。
    """

    s0: int
    s1: int
    i0: int
    i1: int
    direction: Literal["up", "down"]
    high: float
    low: float
    confirmed: bool
    kind: Literal["candidate", "settled"] = "settled"
    # ── 端点分型价格（特征序列驱动） ──
    # ── 结构端点（对象层唯一真相） ──
    # ep*_i: merged idx；ep*_price: 分型价；ep*_type: top/bottom。
    ep0_i: int = -1
    ep0_price: float = 0.0
    ep0_type: Literal["top", "bottom"] | None = None
    ep1_i: int = -1
    ep1_price: float = 0.0
    ep1_type: Literal["top", "bottom"] | None = None
    # DEPRECATED: 历史兼容字段，等同于 ep0_price / ep1_price。
    p0: float = 0.0
    p1: float = 0.0
    # ── 断段证据（v1 特征序列法写入） ──
    break_evidence: BreakEvidence | None = None


def _stroke_endpoint_by_type(
    stroke: Stroke,
    fractal_type: Literal["top", "bottom"],
) -> tuple[int, float]:
    """从笔上按分型类型取端点（返回 merged idx, price）。"""
    if fractal_type == "top":
        if stroke.direction == "down":
            return int(stroke.i0), float(stroke.p0)
        return int(stroke.i1), float(stroke.p1)
    if stroke.direction == "down":
        return int(stroke.i1), float(stroke.p1)
    return int(stroke.i0), float(stroke.p0)


# ====================================================================
# v0 主函数
# ====================================================================

def segments_from_strokes_v0(strokes: list[Stroke]) -> list[Segment]:
    """v0 线段构造：三笔交集重叠 → 生成线段。

    Parameters
    ----------
    strokes : list[Stroke]
        由 ``strokes_from_fractals`` 返回的笔列表。

    Returns
    -------
    list[Segment]
        按 ``s0`` 递增排序。最后一段 ``confirmed=False``，其余 ``True``。

    Notes
    -----
    算法（docs/chan_spec.md §5.1 + §5.4 v0）：

    1. 最小窗口：连续三笔 strokes[j], strokes[j+1], strokes[j+2]
    2. 三笔交集重叠判定：
       - overlap_low  = max(s1.low, s2.low, s3.low)
       - overlap_high = min(s1.high, s2.high, s3.high)
       - 成立条件：overlap_low < overlap_high
    3. 成立则生成 Segment（方向 = 第一笔方向）
    4. 贪心推进：生成一段后 j 跳到 j+2（从第三笔继续尝试下一段）
    5. confirmed：最后一段 False，其余 True
    """
    n = len(strokes)
    if n < 3:
        return []

    segments: list[Segment] = []
    j = 0

    while j <= n - 3:
        s1, s2, s3 = strokes[j], strokes[j + 1], strokes[j + 2]

        # §5.1 三笔交集重叠
        overlap_low = max(s1.low, s2.low, s3.low)
        overlap_high = min(s1.high, s2.high, s3.high)

        if overlap_low < overlap_high:
            seg_high = max(s1.high, s2.high, s3.high)
            seg_low = min(s1.low, s2.low, s3.low)
            start_type: Literal["top", "bottom"] = (
                "bottom" if s1.direction == "up" else "top"
            )
            end_type: Literal["top", "bottom"] = (
                "top" if s1.direction == "up" else "bottom"
            )
            ep0_i, ep0_price = _stroke_endpoint_by_type(s1, start_type)
            ep1_i, ep1_price = _stroke_endpoint_by_type(s3, end_type)

            segments.append(Segment(
                s0=j,
                s1=j + 2,
                i0=s1.i0,
                i1=s3.i1,
                direction=s1.direction,   # §5.2 v0: 第一笔方向
                high=seg_high,
                low=seg_low,
                confirmed=True,
                ep0_i=ep0_i,
                ep0_price=ep0_price,
                ep0_type=start_type,
                ep1_i=ep1_i,
                ep1_price=ep1_price,
                ep1_type=end_type,
                p0=ep0_price,
                p1=ep1_price,
            ))
            j += 2  # 贪心：从第三笔继续
        else:
            j += 1

    # §5.3 最后一段标记为未确认
    if segments:
        last = segments[-1]
        segments[-1] = Segment(
            s0=last.s0, s1=last.s1, i0=last.i0, i1=last.i1,
            direction=last.direction, high=last.high, low=last.low,
            confirmed=False,
            ep0_i=last.ep0_i, ep0_price=last.ep0_price, ep0_type=last.ep0_type,
            ep1_i=last.ep1_i, ep1_price=last.ep1_price, ep1_type=last.ep1_type,
            p0=last.p0, p1=last.p1,
        )

    return segments
