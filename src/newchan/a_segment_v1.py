"""A 系统 — 线段 v1（从缠论原文定义重写）

缠论原文三条核心规则：
  1. 线段至少由连续的三笔构成，且起始三笔必须有重叠部分
  2. 线段被破坏，当且仅当至少被有重叠部分的连续三笔的其中一笔破坏
  3. 线段被破坏的充要条件就是另一个线段生成

实现方法：
  - 增量构建特征序列（反向笔），逐笔推进
  - 标准包含处理（局部方向，和K线包含一致）
  - 分型触发收段（向上段找顶分型，向下段找底分型）
  - 段终点 = 分型中心 b 对应反向笔之前的同向笔 (stroke[k-1])
  - 新段起点 = 分型中心 b 对应的反向笔 (stroke[k])

规格引用: 缠论.pdf L35-41, L175
"""

from __future__ import annotations

import logging
from typing import Literal

from newchan.a_segment_v0 import BreakEvidence, Segment
from newchan.a_stroke import Stroke

logger = logging.getLogger(__name__)


# ====================================================================
# 内部工具
# ====================================================================

def _three_stroke_overlap(s1: Stroke, s2: Stroke, s3: Stroke) -> bool:
    """三笔交集重叠判定。"""
    return max(s1.low, s2.low, s3.low) < min(s1.high, s2.high, s3.high)


def _find_overlap_start(strokes: list[Stroke], from_s: int) -> int | None:
    """从 from_s 开始找到第一个满足三笔交集重叠的起点。"""
    n = len(strokes)
    j = from_s
    while j <= n - 3:
        if _three_stroke_overlap(strokes[j], strokes[j + 1], strokes[j + 2]):
            return j
        j += 1
    return None


def _segment_endpoint_types(
    direction: str,
) -> tuple[Literal["top", "bottom"], Literal["top", "bottom"]]:
    """根据段方向返回 (起点分型类型, 终点分型类型)。"""
    if direction == "up":
        return "bottom", "top"
    return "top", "bottom"


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


def _make_segment(
    strokes: list[Stroke],
    s0: int,
    s1: int,
    direction: Literal["up", "down"],
    confirmed: bool,
    break_evidence: BreakEvidence | None = None,
    kind: Literal["candidate", "settled"] = "settled",
) -> Segment:
    """创建 Segment：端点从边界笔取，保证相邻段视觉连续。"""
    seg_strokes = strokes[s0 : s1 + 1]
    seg_high = max(s.high for s in seg_strokes)
    seg_low = min(s.low for s in seg_strokes)
    start_type, end_type = _segment_endpoint_types(direction)
    ep0_i, ep0_price = _stroke_endpoint_by_type(strokes[s0], start_type)
    ep1_i, ep1_price = _stroke_endpoint_by_type(strokes[s1], end_type)
    return Segment(
        s0=s0, s1=s1,
        i0=strokes[s0].i0, i1=strokes[s1].i1,
        direction=direction,
        high=seg_high, low=seg_low,
        confirmed=confirmed,
        kind=kind,
        ep0_i=ep0_i, ep0_price=ep0_price, ep0_type=start_type,
        ep1_i=ep1_i, ep1_price=ep1_price, ep1_type=end_type,
        p0=ep0_price, p1=ep1_price,
        break_evidence=break_evidence,
    )


# ====================================================================
# 增量特征序列 + 包含处理 + 分型检测
# ====================================================================

def _apply_inclusion(
    elements: list[list[float]], h: float, l: float, dir_state: str | None,
) -> str | None:
    """对 elements 尾部做包含处理或追加新元素，返回更新后的 dir_state。"""
    last = elements[-1]
    last_h, last_l = last[0], last[1]

    left_inc = last_h >= h and last_l <= l
    right_inc = h >= last_h and l <= last_l

    if left_inc or right_inc:
        effective_up = dir_state != "DOWN"
        if effective_up:
            last[0] = max(last_h, h)
            last[1] = max(last_l, l)
        else:
            last[0] = min(last_h, h)
            last[1] = min(last_l, l)
        return dir_state

    if h > last_h and l > last_l:
        dir_state = "UP"
    elif h < last_h and l < last_l:
        dir_state = "DOWN"
    elements.append([h, l])
    return dir_state


def _has_any_fractal(elements: list[list[float]]) -> bool:
    """在元素序列中检测是否存在任意分型（顶或底）。"""
    n = len(elements)
    for j in range(1, n - 1):
        a_h, a_l = elements[j - 1][0], elements[j - 1][1]
        b_h, b_l = elements[j][0], elements[j][1]
        c_h, c_l = elements[j + 1][0], elements[j + 1][1]
        if b_h > a_h and b_h > c_h and b_l > a_l and b_l > c_l:
            return True
        if b_l < a_l and b_l < c_l and b_h < a_h and b_h < c_h:
            return True
    return False


def _is_fractal_and_gap(
    a_h: float, a_l: float,
    b_h: float, b_l: float,
    c_h: float, c_l: float,
    seg_direction: str,
) -> tuple[bool, bool]:
    """检测 (a,b,c) 是否构成目标分型，以及 a-b 间是否有缺口。

    Returns (is_fractal, has_gap).
    """
    if seg_direction == "up":
        is_fractal = b_h > a_h and b_h > c_h and b_l > a_l and b_l > c_l
        has_gap = b_l >= a_h if is_fractal else False
    else:
        is_fractal = b_l < a_l and b_l < c_l and b_h < a_h and b_h < c_h
        has_gap = a_l >= b_h if is_fractal else False
    return is_fractal, has_gap


class _FeatureSeqState:
    """增量维护标准特征序列的状态。"""

    # 尾窗扫描大小：分型检测只在最近 N 个元素内进行
    TAIL_WINDOW: int = 7

    def __init__(self, seg_direction: str = "up") -> None:
        # 标准特征序列：每个元素 = [high, low, stroke_idx]
        self.std: list[list[float | int]] = []
        # 向上段特征序列（down笔）趋势向上 → 初始 None（默认UP）
        # 向下段特征序列（up笔）趋势向下 → 初始 "DOWN"
        self.dir_state: str | None = "DOWN" if seg_direction == "down" else None
        self.last_checked: int = 0  # 上次分型检查的起始位置
        self._skip_until_stroke: int = -1  # 跳过 stroke_idx <= 此值的分型

    def reset(self, seg_direction: str = "up") -> None:
        self.std = []
        self.dir_state = "DOWN" if seg_direction == "down" else None
        self.last_checked = 0
        self._skip_until_stroke = -1

    def skip_trigger(self, stroke_idx: int) -> None:
        """标记：跳过 stroke_idx <= 此值的分型触发。

        当主循环因 min_seg_strokes 拒绝了一个触发时调用，
        防止下次 scan_trigger 反复返回同一个分型。
        """
        self._skip_until_stroke = stroke_idx

    def append(self, stroke_idx: int, high: float, low: float) -> None:
        """增量添加一个反向笔并做包含处理。"""
        if not self.std:
            self.std.append([high, low, stroke_idx])
            return

        last = self.std[-1]
        last_h, last_l = last[0], last[1]

        left_inc = last_h >= high and last_l <= low
        right_inc = high >= last_h and low <= last_l
        has_inclusion = left_inc or right_inc

        if has_inclusion:
            effective_up = self.dir_state != "DOWN"
            if effective_up:
                last[0] = max(last_h, high)
                last[1] = max(last_l, low)
            else:
                last[0] = min(last_h, high)
                last[1] = min(last_l, low)
            last[2] = stroke_idx
            # 包含合并修改了尾部元素 → 回退 last_checked 让分型检查覆盖它
            self.last_checked = max(0, len(self.std) - 3)
        else:
            if high > last_h and low > last_l:
                self.dir_state = "UP"
            elif high < last_h and low < last_l:
                self.dir_state = "DOWN"
            self.std.append([high, low, stroke_idx])

    @staticmethod
    def _second_seq_has_fractal(
        strokes: list[Stroke],
        seg_dir: str,
        from_stroke_idx: int,
    ) -> bool:
        """检查第二特征序列是否存在分型。

        第67课第二种情况：特征序列分型的第一、第二元素间有缺口时，
        需要从分型中心开始构建**第二特征序列**（同向笔，即 seg_dir 方向），
        对其独立做包含处理，只要出现任意分型即可。
        """
        elements: list[list[float]] = []
        dir_state: str | None = "DOWN" if seg_dir == "up" else None

        for i in range(from_stroke_idx + 1, len(strokes)):
            sk = strokes[i]
            if sk.direction != seg_dir:
                continue
            if not elements:
                elements.append([sk.high, sk.low])
                continue
            dir_state = _apply_inclusion(elements, sk.high, sk.low, dir_state)

        return _has_any_fractal(elements)

    def scan_trigger(
        self, seg_direction: str, strokes: list[Stroke],
    ) -> tuple[int, tuple[int, int, int], Literal["none", "second"]] | None:
        """从 last_checked 向后扫描（受尾窗限制），找第一个匹配分型。

        向上段找顶分型，向下段找底分型。
        跳过 stroke_idx <= _skip_until_stroke 的分型（已被主循环拒绝）。
        """
        n = len(self.std)
        if n < 3:
            return None

        start = max(1, self.last_checked, n - self.TAIL_WINDOW)
        for i in range(start, n - 1):
            b_stroke = int(self.std[i][2])
            if b_stroke <= self._skip_until_stroke:
                continue

            a_h, a_l = self.std[i - 1][0], self.std[i - 1][1]
            b_h, b_l = self.std[i][0], self.std[i][1]
            c_h, c_l = self.std[i + 1][0], self.std[i + 1][1]

            is_fractal, has_gap = _is_fractal_and_gap(
                a_h, a_l, b_h, b_l, c_h, c_l, seg_direction,
            )
            if not is_fractal:
                continue
            if has_gap and not self._second_seq_has_fractal(
                strokes, seg_direction, b_stroke,
            ):
                continue

            gap_type: Literal["none", "second"] = "second" if has_gap else "none"
            self.last_checked = max(0, i - 1)
            return b_stroke, (i - 1, i, i + 1), gap_type

        return None


# ====================================================================
# v1 主函数
# ====================================================================

def _try_trigger_segment(
    feat: "_FeatureSeqState",
    seg_dir: "Literal['up', 'down']",
    strokes: list[Stroke],
    seg_start: int,
    min_seg_strokes: int,
    n: int,
) -> tuple[int, BreakEvidence] | None:
    """尝试从特征序列触发断段。返回 (k, break_evidence) 或 None。"""
    trig = feat.scan_trigger(seg_dir, strokes)
    if trig is None:
        return None

    k, fractal_abc, gap_type = trig
    end_stroke = k - 1

    # 保证至少 min_seg_strokes 笔
    if end_stroke - seg_start < min_seg_strokes - 1:
        feat.skip_trigger(k)
        return None

    # 结算锚验证：新段前三笔必须有重叠
    if k + 2 >= n or not _three_stroke_overlap(
        strokes[k], strokes[k + 1], strokes[k + 2]
    ):
        feat.skip_trigger(k)
        return None

    break_ev = BreakEvidence(
        trigger_stroke_k=k,
        fractal_abc=fractal_abc,
        gap_type=gap_type,
    )
    return k, break_ev


def _finalize_last_segment(
    segments: list[Segment],
    strokes: list[Stroke],
    seg_start: int,
    seg_dir: "Literal['up', 'down']",
    min_seg_strokes: int,
    n: int,
) -> None:
    """处理最后一段（未确认）并追加到 segments。"""
    if seg_start >= n:
        return

    last_end = n - 1
    if last_end - seg_start >= min_seg_strokes - 1:
        if seg_start + 2 < n and _three_stroke_overlap(
            strokes[seg_start], strokes[seg_start + 1], strokes[seg_start + 2]
        ):
            last_kind: Literal["candidate", "settled"] = "settled"
        else:
            last_kind = "candidate"
        segments.append(
            _make_segment(strokes, seg_start, last_end, seg_dir, False,
                          kind=last_kind)
        )
    elif segments:
        prev = segments[-1]
        segments[-1] = _make_segment(
            strokes, prev.s0, last_end, prev.direction, False,
            kind=prev.kind,
        )
    else:
        segments.append(
            _make_segment(strokes, seg_start, last_end, seg_dir, False,
                          kind="candidate")
        )


def _ensure_last_unconfirmed(segments: list[Segment], strokes: list[Stroke]) -> None:
    """确保最后一段 confirmed=False（原地修改列表尾元素）。"""
    if segments and segments[-1].confirmed:
        last = segments[-1]
        segments[-1] = _make_segment(
            strokes, last.s0, last.s1, last.direction, False,
            kind=last.kind,
        )


def _emit_segment(
    segments: list[Segment],
    strokes: list[Stroke],
    seg_start: int,
    seg_dir: "Literal['up', 'down']",
    k: int,
    break_ev: BreakEvidence,
) -> None:
    """发射旧段并记录日志。"""
    end_stroke = k - 1
    segments.append(
        _make_segment(strokes, seg_start, end_stroke, seg_dir, True,
                      break_evidence=break_ev, kind="settled")
    )
    logger.debug(
        "segment break: dir=%s, s0=%d, s1=%d, trigger_k=%d, gap=%s",
        seg_dir, seg_start, end_stroke, k, break_ev.gap_type,
    )


def segments_from_strokes_v1(
    strokes: list[Stroke],
    min_seg_strokes: int = 3,
) -> list[Segment]:
    """v1 线段构造：增量特征序列法，逐笔推进检查特征序列分型触发断段。"""
    n = len(strokes)
    if n < 3:
        return []

    segments: list[Segment] = []
    seg_start = _find_overlap_start(strokes, 0)
    if seg_start is None:
        return []

    seg_dir: Literal["up", "down"] = strokes[seg_start].direction
    feat = _FeatureSeqState(seg_dir)
    cursor = seg_start

    while cursor < n:
        sk = strokes[cursor]
        opposite: Literal["up", "down"] = "down" if seg_dir == "up" else "up"
        if sk.direction != opposite:
            cursor += 1
            continue

        feat.append(cursor, sk.high, sk.low)
        result = _try_trigger_segment(
            feat, seg_dir, strokes, seg_start, min_seg_strokes, n,
        )
        if result is None:
            cursor += 1
            continue

        k, break_ev = result
        _emit_segment(segments, strokes, seg_start, seg_dir, k, break_ev)
        seg_start, seg_dir = k, opposite
        feat.reset(seg_dir)
        cursor = k

    _finalize_last_segment(segments, strokes, seg_start, seg_dir, min_seg_strokes, n)
    _ensure_last_unconfirmed(segments, strokes)
    return segments
