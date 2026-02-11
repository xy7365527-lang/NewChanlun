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

from newchan.a_feature_sequence import FeatureBar
from newchan.a_segment_v0 import Segment
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
        ep0_i=ep0_i, ep0_price=ep0_price, ep0_type=start_type,
        ep1_i=ep1_i, ep1_price=ep1_price, ep1_type=end_type,
        p0=ep0_price, p1=ep1_price,
    )


# ====================================================================
# 增量特征序列 + 包含处理 + 分型检测
# ====================================================================

class _FeatureSeqState:
    """增量维护标准特征序列的状态。"""

    def __init__(self) -> None:
        # 标准特征序列：每个元素 = [high, low, stroke_idx]
        self.std: list[list[float | int]] = []
        self.dir_state: str | None = None
        self.last_checked: int = 0  # 上次分型检查的起始位置
        self._skip_until_stroke: int = -1  # 跳过 stroke_idx <= 此值的分型

    def reset(self) -> None:
        self.std = []
        self.dir_state = None
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

        # 包含判定
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
            last[2] = stroke_idx  # 保留最新笔索引
            # 包含合并修改了尾部元素 → 回退 last_checked 让分型检查覆盖它
            self.last_checked = max(0, len(self.std) - 3)
        else:
            # 双条件更新方向
            if high > last_h and low > last_l:
                self.dir_state = "UP"
            elif high < last_h and low < last_l:
                self.dir_state = "DOWN"
            self.std.append([high, low, stroke_idx])

    def scan_trigger(
        self, seg_direction: str,
    ) -> int | None:
        """从 last_checked 向后扫描，找第一个匹配分型。

        向上段找顶分型（high 先升后降 AND low 先升后降）。
        向下段找底分型（low 先降后升 AND high 先降后升）。

        跳过 stroke_idx <= _skip_until_stroke 的分型（已被主循环拒绝）。

        Returns: 分型中心 b 对应的 stroke_idx，或 None。
        """
        n = len(self.std)
        if n < 3:
            return None

        start = max(1, self.last_checked)
        for i in range(start, n - 1):
            b_stroke = int(self.std[i][2])

            # 跳过已被拒绝的分型
            if b_stroke <= self._skip_until_stroke:
                continue

            a_h, a_l = self.std[i - 1][0], self.std[i - 1][1]
            b_h, b_l = self.std[i][0], self.std[i][1]
            c_h, c_l = self.std[i + 1][0], self.std[i + 1][1]

            if seg_direction == "up":
                # 顶分型：b 的 high 和 low 都大于两侧
                if b_h > a_h and b_h > c_h and b_l > a_l and b_l > c_l:
                    # 缺口检测：b 和 c 之间是否有缺口（不重叠）
                    has_gap = max(b_l, c_l) >= min(b_h, c_h)
                    if has_gap:
                        if not (b_h > a_h):
                            if i + 2 >= n:
                                continue
                    self.last_checked = max(0, i - 1)
                    return b_stroke
            else:
                # 底分型：b 的 low 和 high 都小于两侧
                if b_l < a_l and b_l < c_l and b_h < a_h and b_h < c_h:
                    has_gap = max(b_l, c_l) >= min(b_h, c_h)
                    if has_gap:
                        if not (b_l < a_l):
                            if i + 2 >= n:
                                continue
                    self.last_checked = max(0, i - 1)
                    return b_stroke

        # 不推进 last_checked：避免跳过后续新增元素可能形成的分型
        return None


# ====================================================================
# v1 主函数
# ====================================================================

def segments_from_strokes_v1(
    strokes: list[Stroke],
    min_seg_strokes: int = 3,
) -> list[Segment]:
    """v1 线段构造：增量特征序列法。

    严格按缠论原文定义：
    - 逐笔推进，每新增一根反向笔就检查特征序列分型
    - 分型触发 = 旧段终结 + 新段生成
    - 旧段 [seg_start, k-1]，新段从 k 开始（k = 分型中心的反向笔）
    - 段终点由分型中心 b 决定，不做全局极值回退
    """
    n = len(strokes)
    if n < 3:
        return []

    segments: list[Segment] = []

    seg_start = _find_overlap_start(strokes, 0)
    if seg_start is None:
        return []

    seg_dir: Literal["up", "down"] = strokes[seg_start].direction
    feat = _FeatureSeqState()

    cursor = seg_start
    while cursor < n:
        sk = strokes[cursor]
        opposite: Literal["up", "down"] = "down" if seg_dir == "up" else "up"

        # 只有反向笔进入特征序列
        if sk.direction != opposite:
            cursor += 1
            continue

        # 增量添加到特征序列并做包含处理
        feat.append(cursor, sk.high, sk.low)

        # 检测分型触发
        trigger_stroke = feat.scan_trigger(seg_dir)
        if trigger_stroke is None:
            cursor += 1
            continue

        # ── 分型触发：旧段终结，新段生成 ──
        k = trigger_stroke  # 分型中心 b 对应的反向笔索引
        end_stroke = k - 1  # 旧段终点 = b 之前的同向笔

        # 保证至少3笔
        if end_stroke - seg_start < min_seg_strokes - 1:
            # 分型太早，段不够3笔 → 标记跳过此触发，继续找下一个
            feat.skip_trigger(k)
            cursor += 1
            continue

        # 发射旧段
        segments.append(
            _make_segment(strokes, seg_start, end_stroke, seg_dir, True)
        )

        logger.debug(
            "segment break: dir=%s, s0=%d, s1=%d, trigger_k=%d",
            seg_dir, seg_start, end_stroke, k,
        )

        # 新段从 k 开始，方向翻转
        seg_start = k
        seg_dir = opposite
        feat.reset()
        cursor = k  # 从新段起点重新开始扫描
        continue

    # (cursor 只在非触发路径时 += 1)

    # 最后一段（未确认）
    if seg_start < n:
        last_end = n - 1
        if last_end - seg_start >= min_seg_strokes - 1:
            segments.append(
                _make_segment(strokes, seg_start, last_end, seg_dir, False)
            )
        elif segments:
            # 剩余笔不够3笔 → 并入前一段
            prev = segments[-1]
            segments[-1] = _make_segment(
                strokes, prev.s0, last_end, prev.direction, False,
            )
        else:
            # 全部笔不够形成任何段
            segments.append(
                _make_segment(strokes, seg_start, last_end, seg_dir, False)
            )

    # ── 首段覆盖 stroke 0 ──
    if segments and segments[0].s0 > 0:
        first = segments[0]
        segments[0] = _make_segment(
            strokes, 0, first.s1, first.direction, first.confirmed,
        )

    # ── 退化段合并：方向与价格矛盾的段不是有效线段 ──
    def _is_degenerate(seg: Segment) -> bool:
        if seg.direction == "up":
            return not (seg.ep1_price > seg.ep0_price + 1e-9)
        return not (seg.ep1_price < seg.ep0_price - 1e-9)

    changed = True
    while changed and len(segments) >= 3:
        changed = False
        new_segs: list[Segment] = []
        i = 0
        while i < len(segments):
            if 0 < i < len(segments) - 1 and _is_degenerate(segments[i]):
                prev = new_segs[-1] if new_segs else segments[i - 1]
                nxt = segments[i + 1]
                merged = _make_segment(
                    strokes, prev.s0, nxt.s1, prev.direction, nxt.confirmed,
                )
                if new_segs:
                    new_segs[-1] = merged
                else:
                    new_segs.append(merged)
                i += 2
                changed = True
            else:
                new_segs.append(segments[i])
                i += 1
        segments = new_segs

    # ── 确保最后一段 confirmed=False ──
    if segments and segments[-1].confirmed:
        last = segments[-1]
        segments[-1] = _make_segment(
            strokes, last.s0, last.s1, last.direction, False,
        )

    return segments
