"""域事件定义 — 笔事件 + 线段事件 + 中枢事件 + 走势类型事件 + 买卖点事件

笔事件（MVP-0）：四种差分快照产生的笔事件流。
线段事件（MVP-B1）：三种线段事件（pending/settle/invalidate）。
中枢事件（MVP-C0）：三种中枢事件（candidate/settle/invalidate）。
走势类型事件（MVP-D0）：三种 Move 事件（candidate/settle/invalidate）。
买卖点事件（MVP-E0）：四种 BSP 事件（candidate/confirm/settle/invalidate）。
所有时间戳使用 epoch 秒（与 overlay 坐标系一致）。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal


# ── 事件基类 ──


@dataclass(frozen=True, slots=True)
class DomainEvent:
    """所有域事件的基类。

    Attributes
    ----------
    event_type : str
        事件类型标识符。
    bar_idx : int
        触发此事件的 bar 在原始序列中的位置索引（0-based）。
    bar_ts : float
        触发此事件的 bar 的时间戳（epoch 秒）。
    seq : int
        全局单调递增事件序号。
    """

    event_type: str
    bar_idx: int
    bar_ts: float
    seq: int
    event_id: str = ""
    schema_version: int = 1


# ── 笔相关事件 ──


@dataclass(frozen=True, slots=True)
class StrokeCandidate(DomainEvent):
    """新笔候选出现（对应 Stroke.confirmed=False）。"""

    event_type: str = field(default="stroke_candidate", init=False)
    stroke_id: int = 0
    direction: Literal["up", "down"] = "up"
    i0: int = 0
    i1: int = 0
    p0: float = 0.0
    p1: float = 0.0


@dataclass(frozen=True, slots=True)
class StrokeSettled(DomainEvent):
    """笔结算 — 从 candidate 变为 confirmed。

    当新的一笔候选产生时，前一笔自动结算。
    """

    event_type: str = field(default="stroke_settled", init=False)
    stroke_id: int = 0
    direction: Literal["up", "down"] = "up"
    i0: int = 0
    i1: int = 0
    p0: float = 0.0
    p1: float = 0.0


@dataclass(frozen=True, slots=True)
class StrokeExtended(DomainEvent):
    """笔延伸 — 末笔的终点分型移动了（同一笔，但 i1/p1 变了）。"""

    event_type: str = field(default="stroke_extended", init=False)
    stroke_id: int = 0
    direction: Literal["up", "down"] = "up"
    old_i1: int = 0
    new_i1: int = 0
    old_p1: float = 0.0
    new_p1: float = 0.0


@dataclass(frozen=True, slots=True)
class StrokeInvalidated(DomainEvent):
    """笔否定 — 之前的笔在新快照中消失（被替换或被合并）。"""

    event_type: str = field(default="stroke_invalidated", init=False)
    stroke_id: int = 0
    direction: Literal["up", "down"] = "up"
    i0: int = 0
    i1: int = 0
    p0: float = 0.0
    p1: float = 0.0


# ── 线段相关事件（MVP-B1）──


@dataclass(frozen=True, slots=True)
class SegmentBreakPendingV1(DomainEvent):
    """线段断裂挂起 — 特征序列分型已触发，标记潜在断裂位置。

    这是一个事件锚：旧段 *可能* 在此终结，但尚未被新段确认结算。
    只有后续新段满足起始三笔重叠后，才会发出 SegmentSettleV1 确认。
    """

    event_type: str = field(default="segment_break_pending", init=False)
    segment_id: int = 0
    direction: Literal["up", "down"] = "up"
    break_at_stroke: int = 0
    gap_class: Literal["none", "gap"] = "none"
    fractal_type: Literal["top", "bottom"] = "top"
    s0: int = 0
    s1: int = 0


@dataclass(frozen=True, slots=True)
class SegmentSettleV1(DomainEvent):
    """线段结算 — 旧段终结确认，新段已满足起始三笔重叠。

    old_end = k-1, new_start = k（k = break_at_stroke）。
    """

    event_type: str = field(default="segment_settle", init=False)
    segment_id: int = 0
    direction: Literal["up", "down"] = "up"
    s0: int = 0
    s1: int = 0
    ep0_price: float = 0.0
    ep1_price: float = 0.0
    gap_class: Literal["none", "gap"] = "none"
    new_segment_s0: int = 0
    new_segment_direction: Literal["up", "down"] = "down"


@dataclass(frozen=True, slots=True)
class SegmentInvalidateV1(DomainEvent):
    """线段否定 — 之前的线段在新快照中消失（因笔否定导致重算）。"""

    event_type: str = field(default="segment_invalidate", init=False)
    segment_id: int = 0
    direction: Literal["up", "down"] = "up"
    s0: int = 0
    s1: int = 0


# ── 中枢相关事件（MVP-C0）──


@dataclass(frozen=True, slots=True)
class ZhongshuCandidateV1(DomainEvent):
    """中枢成立 — 三段重叠确认。

    当连续 3 段以上已确认线段的价格区间有严格重叠（ZG > ZD）时触发。
    """

    event_type: str = field(default="zhongshu_candidate", init=False)
    zhongshu_id: int = 0
    zd: float = 0.0
    zg: float = 0.0
    seg_start: int = 0
    seg_end: int = 0
    seg_count: int = 3
    level_id: int = 1


@dataclass(frozen=True, slots=True)
class ZhongshuSettleV1(DomainEvent):
    """中枢闭合 — 突破段到来，中枢终结。

    某段离开 [ZD, ZG] 区间时触发。
    """

    event_type: str = field(default="zhongshu_settle", init=False)
    zhongshu_id: int = 0
    zd: float = 0.0
    zg: float = 0.0
    seg_start: int = 0
    seg_end: int = 0
    seg_count: int = 3
    break_seg_id: int = 0
    break_direction: Literal["up", "down"] = "up"
    level_id: int = 1


@dataclass(frozen=True, slots=True)
class ZhongshuInvalidateV1(DomainEvent):
    """中枢否定 — 构成段被否定导致中枢消失。"""

    event_type: str = field(default="zhongshu_invalidate", init=False)
    zhongshu_id: int = 0
    zd: float = 0.0
    zg: float = 0.0
    seg_start: int = 0
    seg_end: int = 0
    level_id: int = 1


# ── 走势类型相关事件（MVP-D0）──


@dataclass(frozen=True, slots=True)
class MoveCandidateV1(DomainEvent):
    """走势类型候选 — 从 settled 中枢分组确认。

    盘整 = 1 中枢，趋势 = 2+ 同向中枢。
    """

    event_type: str = field(default="move_candidate", init=False)
    move_id: int = 0
    kind: str = "consolidation"
    direction: str = "up"
    seg_start: int = 0
    seg_end: int = 0
    zs_start: int = 0
    zs_end: int = 0
    zs_count: int = 1
    level_id: int = 1


@dataclass(frozen=True, slots=True)
class MoveSettleV1(DomainEvent):
    """走势类型结算 — 后续新 move 出现确认当前 move 终结。"""

    event_type: str = field(default="move_settle", init=False)
    move_id: int = 0
    kind: str = "consolidation"
    direction: str = "up"
    seg_start: int = 0
    seg_end: int = 0
    zs_start: int = 0
    zs_end: int = 0
    zs_count: int = 1
    level_id: int = 1


@dataclass(frozen=True, slots=True)
class MoveInvalidateV1(DomainEvent):
    """走势类型否定 — 构成中枢被否定导致 move 消失。"""

    event_type: str = field(default="move_invalidate", init=False)
    move_id: int = 0
    kind: str = "consolidation"
    direction: str = "up"
    seg_start: int = 0
    seg_end: int = 0
    level_id: int = 1


# ── 买卖点相关事件（MVP-E0）──


@dataclass(frozen=True, slots=True)
class BuySellPointCandidateV1(DomainEvent):
    """买卖点候选 — 新买卖点出现或状态更新。"""

    event_type: str = field(default="bsp_candidate", init=False)
    bsp_id: int = 0
    kind: str = "type1"      # type1 / type2 / type3
    side: str = "buy"        # buy / sell
    level_id: int = 1
    seg_idx: int = 0
    price: float = 0.0
    move_seg_start: int = 0
    center_seg_start: int = 0
    overlaps_with: str = ""  # "" / "type2" / "type3"


@dataclass(frozen=True, slots=True)
class BuySellPointConfirmV1(DomainEvent):
    """买卖点确认 — confirmed: False → True。"""

    event_type: str = field(default="bsp_confirm", init=False)
    bsp_id: int = 0
    kind: str = "type1"
    side: str = "buy"
    level_id: int = 1
    seg_idx: int = 0
    price: float = 0.0


@dataclass(frozen=True, slots=True)
class BuySellPointSettleV1(DomainEvent):
    """买卖点结算 — settled: False → True（后续走势验证）。"""

    event_type: str = field(default="bsp_settle", init=False)
    bsp_id: int = 0
    kind: str = "type1"
    side: str = "buy"
    level_id: int = 1
    seg_idx: int = 0
    price: float = 0.0


@dataclass(frozen=True, slots=True)
class BuySellPointInvalidateV1(DomainEvent):
    """买卖点否定 — 前提条件被否定导致买卖点消失。"""

    event_type: str = field(default="bsp_invalidate", init=False)
    bsp_id: int = 0
    kind: str = "type1"
    side: str = "buy"
    level_id: int = 1
    seg_idx: int = 0


# ── 审计事件 ──


@dataclass(frozen=True, slots=True)
class InvariantViolation(DomainEvent):
    """运行时不变量违规事件。

    由 InvariantChecker 在检测到不变量违规时生成。
    不参与核心事件流，仅用于审计和诊断。
    """

    event_type: str = field(default="invariant_violation", init=False)
    code: str = ""  # I1_SETTLED_OVERWRITE / I2_TIME_BACKWARD / ...
    reason: str = ""
    snapshot_hash: str = ""


# ── 便捷类型 ──

StrokeEvent = StrokeCandidate | StrokeSettled | StrokeExtended | StrokeInvalidated
SegmentEvent = SegmentBreakPendingV1 | SegmentSettleV1 | SegmentInvalidateV1
ZhongshuEvent = ZhongshuCandidateV1 | ZhongshuSettleV1 | ZhongshuInvalidateV1
MoveEvent = MoveCandidateV1 | MoveSettleV1 | MoveInvalidateV1
BuySellPointEvent = (
    BuySellPointCandidateV1
    | BuySellPointConfirmV1
    | BuySellPointSettleV1
    | BuySellPointInvalidateV1
)
