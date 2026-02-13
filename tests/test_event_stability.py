"""事件顺序稳定性测试 — MVP-A PR-A1

验证 diff_strokes 产生的事件在 seq 和因果顺序上完全确定：
  - invalidated 先于 settled（因果顺序）
  - seq 单调递增、无间隙
  - 相同输入 → 相同事件序列（确定性）
  - 无重复事件
  - confirmed 笔只产生 StrokeSettled
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.bi_differ import diff_strokes
from newchan.events import (
    DomainEvent,
    StrokeCandidate,
    StrokeExtended,
    StrokeInvalidated,
    StrokeSettled,
)


# ── 辅助函数 ──────────────────────────────────────────────────────


def _mk(
    i0: int,
    i1: int,
    direction: str = "up",
    p0: float = 10.0,
    p1: float = 20.0,
    confirmed: bool = True,
) -> Stroke:
    """快速构造一个 Stroke。high/low 从 p0/p1 推导。"""
    return Stroke(
        i0=i0,
        i1=i1,
        direction=direction,
        high=max(p0, p1),
        low=min(p0, p1),
        p0=p0,
        p1=p1,
        confirmed=confirmed,
    )


_BAR_IDX = 10
_BAR_TS = 1707800000.0


def _diff(
    prev: list[Stroke],
    curr: list[Stroke],
    seq_start: int = 0,
    bar_idx: int = _BAR_IDX,
    bar_ts: float = _BAR_TS,
) -> list[DomainEvent]:
    return diff_strokes(
        prev, curr, bar_idx=bar_idx, bar_ts=bar_ts, seq_start=seq_start
    )


# =====================================================================
# 共用测试场景
# =====================================================================


def _scenario_invalidate_and_settle() -> tuple[list[Stroke], list[Stroke]]:
    """候选笔被确认 + 新候选出现 → 产生 invalidate + settled + candidate。"""
    prev = [_mk(0, 5, "up", 10.0, 20.0, confirmed=False)]
    curr = [
        _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
        _mk(5, 10, "down", 20.0, 12.0, confirmed=False),
    ]
    return prev, curr


def _scenario_multi_stroke_change() -> tuple[list[Stroke], list[Stroke]]:
    """多笔变化：两笔被 invalidate，再产生新的 settled + candidate。"""
    prev = [
        _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
        _mk(5, 10, "down", 20.0, 12.0, confirmed=True),
        _mk(10, 15, "up", 12.0, 22.0, confirmed=False),
    ]
    curr = [
        _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
        _mk(5, 12, "down", 20.0, 8.0, confirmed=True),
        _mk(12, 18, "up", 8.0, 25.0, confirmed=False),
    ]
    return prev, curr


# =====================================================================
# 测试类
# =====================================================================


class TestEventStability:
    """事件顺序稳定性测试。"""

    # ── 1. 因果顺序：invalidated 在 settled 之前 ──

    def test_invalidate_before_settle_order(self):
        """invalidated 事件必须在 settled 事件之前出现（因果顺序）。

        场景：候选笔确认 + 新候选 → 先 invalidate 旧候选，再 settle 已确认笔。
        """
        prev, curr = _scenario_invalidate_and_settle()
        events = _diff(prev, curr)

        # 至少有 invalidated 和 settled 事件
        inv_indices = [
            i for i, e in enumerate(events) if isinstance(e, StrokeInvalidated)
        ]
        settled_indices = [
            i for i, e in enumerate(events) if isinstance(e, StrokeSettled)
        ]
        assert len(inv_indices) > 0, "应存在 invalidated 事件"
        assert len(settled_indices) > 0, "应存在 settled 事件"

        # 所有 invalidated 事件的位置 < 所有 settled 事件的位置
        assert max(inv_indices) < min(settled_indices), (
            f"invalidated 事件必须全部排在 settled 事件之前: "
            f"inv={inv_indices}, settled={settled_indices}"
        )

    # ── 2. 同一 bar 内 seq 单调递增 ──

    def test_seq_monotonic_within_bar(self):
        """同一 bar 内产生的多个事件，其 seq 必须严格单调递增。"""
        prev, curr = _scenario_invalidate_and_settle()
        events = _diff(prev, curr)
        assert len(events) >= 2, "此场景至少产生 2 个事件"

        seqs = [e.seq for e in events]
        for i in range(1, len(seqs)):
            assert seqs[i] > seqs[i - 1], (
                f"seq 非单调递增: seq[{i - 1}]={seqs[i - 1]}, seq[{i}]={seqs[i]}"
            )

    # ── 3. 跨 bar 的 seq 全局单调递增 ──

    def test_seq_monotonic_across_bars(self):
        """跨 bar 的事件 seq 全局单调递增。

        模拟两个 bar 分别产生事件，第二个 bar 的 seq_start 接续第一个 bar。
        """
        # Bar 1: 空 → 一笔候选
        prev_1: list[Stroke] = []
        curr_1 = [_mk(0, 5, "up", 10.0, 20.0, confirmed=False)]
        events_1 = _diff(prev_1, curr_1, seq_start=0, bar_idx=10, bar_ts=1000.0)

        # Bar 2: 候选确认 + 新候选
        prev_2 = curr_1
        curr_2 = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=False),
        ]
        next_seq = events_1[-1].seq + 1 if events_1 else 0
        events_2 = _diff(prev_2, curr_2, seq_start=next_seq, bar_idx=11, bar_ts=1001.0)

        all_events = events_1 + events_2
        assert len(all_events) >= 2, "跨两个 bar 至少产生 2 个事件"

        seqs = [e.seq for e in all_events]
        for i in range(1, len(seqs)):
            assert seqs[i] > seqs[i - 1], (
                f"跨 bar seq 非单调递增: seq[{i - 1}]={seqs[i - 1]}, seq[{i}]={seqs[i]}"
            )

    # ── 4. 相同输入不产生重复事件 ──

    def test_no_duplicate_events_same_input(self):
        """相同 prev/curr 输入不产生重复事件（event_id 唯一）。"""
        prev, curr = _scenario_multi_stroke_change()
        events = _diff(prev, curr)

        event_ids = [e.event_id for e in events]
        assert len(event_ids) == len(set(event_ids)), (
            f"存在重复 event_id: {event_ids}"
        )

        # 额外校验：(event_type, stroke_id) 组合也不重复
        type_stroke_pairs = [(e.event_type, getattr(e, "stroke_id", None)) for e in events]
        assert len(type_stroke_pairs) == len(set(type_stroke_pairs)), (
            f"存在重复 (event_type, stroke_id) 对: {type_stroke_pairs}"
        )

    # ── 5. 幂等性：prev === curr 时无事件 ──

    def test_idempotent_no_change(self):
        """prev 和 curr 完全相同时，不产生任何事件。"""
        strokes = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=True),
            _mk(10, 15, "up", 12.0, 25.0, confirmed=False),
        ]
        events = _diff(strokes, strokes)
        assert events == [], f"prev === curr 时应无事件，实际得到 {len(events)} 个"

    # ── 6. 确定性：100 次执行事件顺序完全一致 ──

    def test_event_order_deterministic_100_runs(self):
        """循环 100 次，每次事件顺序、event_id 完全一致。"""
        prev, curr = _scenario_multi_stroke_change()

        reference_events = _diff(prev, curr)
        reference_ids = [e.event_id for e in reference_events]
        reference_seqs = [e.seq for e in reference_events]
        reference_types = [e.event_type for e in reference_events]

        for run in range(100):
            events = _diff(prev, curr)
            ids = [e.event_id for e in events]
            seqs = [e.seq for e in events]
            types = [e.event_type for e in events]

            assert ids == reference_ids, (
                f"第 {run} 次运行 event_id 不一致: {ids} != {reference_ids}"
            )
            assert seqs == reference_seqs, (
                f"第 {run} 次运行 seq 不一致: {seqs} != {reference_seqs}"
            )
            assert types == reference_types, (
                f"第 {run} 次运行 event_type 不一致: {types} != {reference_types}"
            )

    # ── 7. seq 无间隙（连续递增） ──

    def test_seq_gap_free(self):
        """seq 序列必须无间隙：每个 seq 比前一个恰好大 1。"""
        prev, curr = _scenario_multi_stroke_change()

        for start in (0, 1, 42, 100):
            events = _diff(prev, curr, seq_start=start)
            if not events:
                continue

            seqs = [e.seq for e in events]
            assert seqs[0] == start, (
                f"首个 seq 应为 {start}，实际为 {seqs[0]}"
            )
            for i in range(1, len(seqs)):
                assert seqs[i] == seqs[i - 1] + 1, (
                    f"seq 存在间隙: seq[{i - 1}]={seqs[i - 1]}, seq[{i}]={seqs[i]}, "
                    f"完整序列={seqs}"
                )

    # ── 8. confirmed 笔只产生 StrokeSettled ──

    def test_event_type_correct_for_confirmed(self):
        """confirmed=True 的笔只产生 StrokeSettled，不产生 StrokeCandidate。

        场景：prev 为空，curr 中有多笔 confirmed=True 的笔和末笔 confirmed=False。
        新增的 confirmed 笔必须产生 StrokeSettled，不能产生 StrokeCandidate。
        """
        prev: list[Stroke] = []
        curr = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=True),
            _mk(10, 15, "up", 12.0, 25.0, confirmed=True),
            _mk(15, 20, "down", 25.0, 15.0, confirmed=False),
        ]
        events = _diff(prev, curr)

        # 按 stroke_id 分组检查
        for e in events:
            stroke_id = getattr(e, "stroke_id", None)
            if stroke_id is None:
                continue

            # 找到 curr 中对应的笔
            if stroke_id < len(curr):
                stroke = curr[stroke_id]
                if stroke.confirmed:
                    assert isinstance(e, StrokeSettled), (
                        f"confirmed 笔 (stroke_id={stroke_id}) 应产生 StrokeSettled，"
                        f"实际产生 {type(e).__name__}"
                    )
                    assert not isinstance(e, StrokeCandidate), (
                        f"confirmed 笔 (stroke_id={stroke_id}) 不应产生 StrokeCandidate"
                    )

        # 验证确实有 StrokeSettled 事件产生
        settled_events = [e for e in events if isinstance(e, StrokeSettled)]
        assert len(settled_events) == 3, (
            f"应有 3 个 StrokeSettled 事件（对应 3 个 confirmed 笔），"
            f"实际有 {len(settled_events)} 个"
        )

        # 末笔 confirmed=False → StrokeCandidate
        candidate_events = [e for e in events if isinstance(e, StrokeCandidate)]
        assert len(candidate_events) == 1, (
            f"应有 1 个 StrokeCandidate 事件（对应末笔未确认），"
            f"实际有 {len(candidate_events)} 个"
        )
        assert candidate_events[0].stroke_id == 3
