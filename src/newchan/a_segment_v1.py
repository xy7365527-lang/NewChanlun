"""A 系统 — 线段 v1（从缠论原文定义重写）

特征序列法（67课精确定义）：
  - 构造标准特征序列（反向笔 + 包含处理）
  - 向上段找顶分型，向下段找底分型
  - 第一种情况：分型第一、第二元素间无缺口 → 线段在分型极值处终结
  - 第二种情况：分型第一、第二元素间有缺口 → 需第二特征序列出现分型
  - 段终点 = 分型中心 b 对应反向笔之前的同向笔 (stroke[k-1])
  - 新段起点 = 分型中心 b 对应的反向笔 (stroke[k])

67课原文："本课，就是把前面'线段破坏的充要条件就是被另一个线段破坏'
精确化了。因此，以后关于线段的划分，都以此精确的定义为基础。"

规格引用: 缠论.pdf L35-41, L175

## 拓扑语义（195号）

### 结构映射
线段是1维CW复形上的子复形（1-chain）。

- 笔序列构成 1 维 CW 复形 X¹（分型=0-cell，笔=1-cell）
- 线段 = X¹ 的一个子复形 σ = {连续笔的链 c₁+c₂+...+cₙ}
- 特征序列（反向笔序列 + 包含处理 + 分型检测）= 子复形边界的标定算法
- 断段操作 = 子复形的分割（subdivision）：在链上切一刀，分出两个子复形
- break_evidence = 分割点的证书（哪个特征序列分型导致了分割）
- 第二特征序列（缺口时构建）= 分割合法性的辅助验证
- 第78课硬约束"顶高于底"= 子复形的定向性约束

### 映射的边界
线段不是2-cell（二维盘）——整个构造在1维骨架上操作，没有二维内部。
特征序列的"分型"和K线分型不是同一层——它是子复形分割的标记，不是CW附着映射。
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


def _top_above_bottom(
    direction: str,
    ep0_price: float,
    ep1_price: float,
) -> bool:
    """第78课硬约束：线段两端的一顶一底，顶肯定要高于底。

    上升线段：ep0 = bottom, ep1 = top → ep1_price > ep0_price
    下降线段：ep0 = top, ep1 = bottom → ep0_price > ep1_price
    """
    if direction == "up":
        return ep1_price > ep0_price
    return ep0_price > ep1_price


def _standardize_endpoints(
    seg_strokes: list[Stroke],
    s0: int,
    direction: Literal["up", "down"],
) -> tuple[int, float, int, float]:
    """第78课标准化：当结构端点违反 L78 时，用实际 high/low 替代。

    第78课原文："如果线段中，最高或最低点不是线段的端点，那么，
    在任何以线段为基础的分析中……都可以把该线段标准化为
    最高低点都在端点。"

    返回 (ep0_i, ep0_price, ep1_i, ep1_price)。
    """
    if direction == "up":
        # 向上线段标准化：ep0=最低点, ep1=最高点
        low_stroke = min(seg_strokes, key=lambda s: s.low)
        high_stroke = max(seg_strokes, key=lambda s: s.high)
        ep0_i = low_stroke.i0 if low_stroke.p0 <= low_stroke.p1 else low_stroke.i1
        ep0_price = low_stroke.low
        ep1_i = high_stroke.i0 if high_stroke.p0 >= high_stroke.p1 else high_stroke.i1
        ep1_price = high_stroke.high
    else:
        # 向下线段标准化：ep0=最高点, ep1=最低点
        high_stroke = max(seg_strokes, key=lambda s: s.high)
        low_stroke = min(seg_strokes, key=lambda s: s.low)
        ep0_i = high_stroke.i0 if high_stroke.p0 >= high_stroke.p1 else high_stroke.i1
        ep0_price = high_stroke.high
        ep1_i = low_stroke.i0 if low_stroke.p0 <= low_stroke.p1 else low_stroke.i1
        ep1_price = low_stroke.low
    return ep0_i, ep0_price, ep1_i, ep1_price


def _make_segment(
    strokes: list[Stroke],
    s0: int,
    s1: int,
    direction: Literal["up", "down"],
    confirmed: bool,
    break_evidence: BreakEvidence | None = None,
    kind: Literal["candidate", "settled"] = "settled",
) -> Segment:
    """创建 Segment：端点从边界笔取，保证相邻段视觉连续。

    第78课硬约束 + 标准化：
    - 结构端点从边界笔的分型取得
    - 若违反"顶高于底"（L78），应用第78课标准化：
      用段内实际 high/low 作为有效端点
    """
    seg_strokes = strokes[s0 : s1 + 1]
    seg_high = max(s.high for s in seg_strokes)
    seg_low = min(s.low for s in seg_strokes)
    start_type, end_type = _segment_endpoint_types(direction)
    ep0_i, ep0_price = _stroke_endpoint_by_type(strokes[s0], start_type)
    ep1_i, ep1_price = _stroke_endpoint_by_type(strokes[s1], end_type)

    if not _top_above_bottom(direction, ep0_price, ep1_price):
        # 279号修复：L78 违反时应用第78课标准化。
        # 248号将 L78 从 reject 改为 warning（解决 71→1 压缩）。
        # 本修复进一步：不仅允许断段，还将端点标准化为实际 high/low，
        # 使下游（中枢/走势）看到的线段区间语义正确。
        # 第78课原文："经过标准化处理后，所有向上线段都是以最低点开始
        # 最高点结束，向下线段都是以最高点开始最低点结束"。
        logger.debug(
            "L78 standardization: direction=%s, s0=%d, s1=%d, "
            "structural_ep0=%.6f, structural_ep1=%.6f → "
            "standardized to high=%.6f, low=%.6f",
            direction, s0, s1, ep0_price, ep1_price, seg_high, seg_low,
        )
        ep0_i, ep0_price, ep1_i, ep1_price = _standardize_endpoints(
            seg_strokes, s0, direction,
        )

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
    """在元素序列中检测是否存在任意分型（顶或底）。

    第二特征序列分型检测：67课"第二个序列中的分型，不分第一二种情况，
    只要有分型就可以"。使用与主特征序列一致的极值条件。
    """
    n = len(elements)
    for j in range(1, n - 1):
        b_h = elements[j][0]
        b_l = elements[j][1]
        a_h = elements[j - 1][0]
        a_l = elements[j - 1][1]
        c_h = elements[j + 1][0]
        c_l = elements[j + 1][1]
        if b_h > a_h and b_h > c_h:
            return True
        if b_l < a_l and b_l < c_l:
            return True
    return False


def _is_fractal_and_gap(
    a_h: float, a_l: float,
    b_h: float, b_l: float,
    c_h: float, c_l: float,
    seg_direction: str,
) -> tuple[bool, bool]:
    """检测 (a,b,c) 是否构成目标分型，以及 a-b 间是否有缺口。

    特征序列分型仅检查趋势方向上的极值：
    - 向上段顶分型：b 的 HIGH 高于两侧（反向笔不再创新高 → 趋势转折）
    - 向下段底分型：b 的 LOW 低于两侧（反向笔不再创新低 → 趋势转折）

    特征序列元素是笔（非单根K线），范围可达数十点。四条件检测
    在宽幅笔上系统性失效——崩盘笔的 LOW 低于后续笔，但其 HIGH
    仍是转折信号。

    Returns (is_fractal, has_gap).
    """
    if seg_direction == "up":
        is_fractal = b_h > a_h and b_h > c_h
        has_gap = b_l >= a_h if is_fractal else False
    else:
        is_fractal = b_l < a_l and b_l < c_l
        has_gap = a_l >= b_h if is_fractal else False
    return is_fractal, has_gap


class _FeatureSeqState:
    """增量维护标准特征序列的状态。"""

    # 尾窗扫描大小：分型检测只在最近 N 个元素内进行
    TAIL_WINDOW: int = 7

    def __init__(
        self,
        seg_direction: str = "up",
        extend_mode: str = "strict",
    ) -> None:
        # 标准特征序列：每个元素 = [high, low, stroke_idx]
        self.std: list[list[float | int]] = []
        # 向上段特征序列（down笔）趋势向上 → 初始 None（默认UP）
        # 向下段特征序列（up笔）趋势向下 → 初始 "DOWN"
        self.dir_state: str | None = "DOWN" if seg_direction == "down" else None
        self.last_checked: int = 0  # 上次分型检查的起始位置
        self._skip_until_stroke: int = -1  # 跳过 stroke_idx <= 此值的分型
        self._extend_mode: str = extend_mode

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

    def append(
        self,
        stroke_idx: int,
        high: float,
        low: float,
        seg_direction: str = "up",
        strokes: list | None = None,
    ) -> None:
        """增量添加一个反向笔，包含处理遵循71课"假设转折点"规则。

        71课："在这假设的转折点前后那两元素，是不存在包含关系的。"

        当检测到包含关系时，先尝试不合并（保持分离），检查是否形成分型：
        - 有分型 → 不合并（此处是转折点，两元素属不同特征序列）
        - 无分型 → 按标准K线包含规则合并
        """
        if not self.std:
            self.std.append([high, low, stroke_idx])
            return

        last = self.std[-1]
        last_h, last_l = last[0], last[1]

        left_inc = last_h >= high and last_l <= low
        right_inc = high >= last_h and low <= last_l
        has_inclusion = left_inc or right_inc

        if has_inclusion:
            self.std.append([high, low, stroke_idx])
            if strokes is not None and self.scan_trigger(seg_direction, strokes) is not None:
                return
            self.std.pop()

            effective_up = self.dir_state != "DOWN"
            if effective_up:
                last[0] = max(last_h, high)
                last[1] = max(last_l, low)
            else:
                last[0] = min(last_h, high)
                last[1] = min(last_l, low)
            last[2] = stroke_idx
            self.last_checked = max(0, len(self.std) - 3)
        else:
            if high > last_h and low > last_l:
                self.dir_state = "UP"
            elif high < last_h and low < last_l:
                self.dir_state = "DOWN"
            self.std.append([high, low, stroke_idx])

    MAX_SECOND_SEQ_SCAN: int = 50

    @staticmethod
    def _second_seq_has_fractal(
        strokes: list[Stroke],
        seg_dir: str,
        from_stroke_idx: int,
        max_scan: int = 50,
    ) -> bool:
        """检查第二特征序列是否存在分型。

        第67课第二种情况：特征序列分型的第一、第二元素间有缺口时，
        需要从分型中心开始构建**第二特征序列**（同向笔，即 seg_dir 方向），
        对其独立做包含处理，只要出现任意分型即可。

        扫描窗口限制在 max_scan 笔以内（含双向）。第二特征序列
        的分型如果存在，应在缺口附近形成。扫描数百笔远处的分型
        在操作上没有意义，且导致 O(n²) 性能退化。
        """
        elements: list[list[float]] = []
        dir_state: str | None = "DOWN" if seg_dir == "up" else None
        end_idx = min(from_stroke_idx + 1 + max_scan, len(strokes))

        for i in range(from_stroke_idx + 1, end_idx):
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

            # 67课严格延续：缺口被 c 封闭 → 按第一种情况处理
            # 原文："特征序列缺口被第一笔就封闭的情况……就变成第一种情况了"
            if has_gap and self._extend_mode == "strict":
                if seg_direction == "up":
                    gap_closed_by_c = c_l <= a_h
                else:
                    gap_closed_by_c = c_h >= a_l
                if gap_closed_by_c:
                    has_gap = False

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

    # L78 后置信息（248号修复 + 279号标准化）
    # 第78课"顶高于底"在 _make_segment 中处理：违反时应用标准化。
    # 此处仅做 debug 记录，不阻止触发。
    start_type, end_type = _segment_endpoint_types(seg_dir)
    _, ep0_price = _stroke_endpoint_by_type(strokes[seg_start], start_type)
    _, ep1_price = _stroke_endpoint_by_type(strokes[end_stroke], end_type)
    if not _top_above_bottom(seg_dir, ep0_price, ep1_price):
        logger.debug(
            "L78 pre-check: seg_dir=%s, s0=%d, s1=%d, "
            "ep0=%.4f, ep1=%.4f — will be standardized in _make_segment",
            seg_dir, seg_start, end_stroke, ep0_price, ep1_price,
        )

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
    extend_mode: Literal["strict", "optimized"] = "strict",
    *,
    _resume: tuple[list[Segment], int, str] | None = None,
) -> list[Segment]:
    """v1 线段构造：增量特征序列法，逐笔推进检查特征序列分型触发断段。

    extend_mode:
        "strict"    — 67课严格延续：缺口被 c 封闭时按第一种情况处理（段更容易终结）
        "optimized" — 优化延续：任何缺口都走第二种情况（段更容易延续）

    _resume:
        增量恢复参数 (pre_segments, seg_start, seg_dir)。
        从已确认线段的断点处恢复计算，跳过已确认部分。
        seg_start = 下一段起始笔索引，seg_dir = 下一段方向。
    """
    n = len(strokes)
    if n < 3:
        return []

    if _resume is not None:
        pre_segments, resume_start, resume_dir = _resume
        segments: list[Segment] = list(pre_segments)
        if resume_start >= n:
            _finalize_last_segment(segments, strokes, resume_start, resume_dir, min_seg_strokes, n)
            _ensure_last_unconfirmed(segments, strokes)
            return segments
        seg_start: int = resume_start
        seg_dir: Literal["up", "down"] = resume_dir  # type: ignore[assignment]
        feat = _FeatureSeqState(seg_dir, extend_mode=extend_mode)
        cursor: int = seg_start
    else:
        segments = []
        seg_start = _find_overlap_start(strokes, 0)
        if seg_start is None:
            return []
        seg_dir = strokes[seg_start].direction
        feat = _FeatureSeqState(seg_dir, extend_mode=extend_mode)
        cursor = seg_start

    while cursor < n:
        sk = strokes[cursor]
        opposite: Literal["up", "down"] = "down" if seg_dir == "up" else "up"
        if sk.direction != opposite:
            cursor += 1
            continue

        feat.append(cursor, sk.high, sk.low, seg_dir, strokes)
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
