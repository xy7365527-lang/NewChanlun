"""BiEngine 逐 bar 测试

验证笔事件引擎的核心流程：
  - 初始状态
  - 单 bar / V 形 / 多 bar 场景
  - reset / snapshot 正确性
  - 事件的 bar_idx 一致性
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.events import StrokeCandidate, StrokeSettled
from newchan.types import Bar


# ── 辅助函数 ──────────────────────────────────────────────────────


def _bar(ts_offset: int, o: float, h: float, l: float, c: float) -> Bar:
    """从偏移序号创建 Bar（每 bar 间隔 5 分钟）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
        + timedelta(seconds=ts_offset * 300),
        open=o,
        high=h,
        low=l,
        close=c,
    )


def _generate_zigzag_bars(n: int = 60) -> list[Bar]:
    """生成锯齿形（zigzag）价格序列，确保能产生多笔。

    设计原则：
    - 每 8 根 bar 一个半周期（下降或上升），幅度足够大
    - 相邻 bar 的 high/low 严格不包含（避免合并）
    - 价格范围：在 60-100 之间大幅振荡

    模式（每 16 根 bar 一个完整周期）：
    - bar 0-7: 从 100 下降到 60（每 bar 降 5）
    - bar 8-15: 从 60 上升到 100（每 bar 升 5）
    - bar 16-23: 从 100 下降到 60
    - ...
    """
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            # 下降段
            base = 100 - cycle_pos * 5
        else:
            # 上升段
            base = 60 + (cycle_pos - 8) * 5

        # 确保相邻 bar 不包含：用固定小振幅
        h = base + 1.5
        l = base - 1.5
        o = base + 0.5
        c = base - 0.5
        bars.append(_bar(i, o, h, l, c))
    return bars


# =====================================================================
# 测试类
# =====================================================================


class TestBiEngine:
    """BiEngine 逐 bar 驱动测试。"""

    def test_initial_state(self):
        """初始状态：bar_count=0, strokes 空。"""
        engine = BiEngine()
        assert engine.bar_count == 0
        assert engine.current_strokes == []
        assert engine.event_seq == 0

    def test_single_bar_no_events(self):
        """单 bar：不可能产生笔，无事件。"""
        engine = BiEngine()
        bar = _bar(0, 100.5, 101.5, 98.5, 99.5)
        snap = engine.process_bar(bar)
        assert snap.bar_idx == 0
        assert snap.strokes == []
        assert snap.events == []
        assert engine.bar_count == 1

    def test_v_shape_produces_candidate(self):
        """V 形走势产生至少一个 StrokeCandidate。"""
        bars = _generate_zigzag_bars(60)
        engine = BiEngine()
        all_events = []
        for bar in bars:
            snap = engine.process_bar(bar)
            all_events.extend(snap.events)

        # 60 根 bar 的锯齿形应该能产生至少 3 笔
        assert len(engine.current_strokes) >= 3, (
            f"Expected >= 3 strokes, got {len(engine.current_strokes)}"
        )
        # 应该至少有一个 StrokeCandidate 事件
        candidates = [e for e in all_events if isinstance(e, StrokeCandidate)]
        assert len(candidates) >= 1

    def test_more_bars_settle_previous(self):
        """更多 bar 导致前一笔结算（StrokeSettled 出现）。"""
        bars = _generate_zigzag_bars(60)
        engine = BiEngine()
        all_events = []
        for bar in bars:
            snap = engine.process_bar(bar)
            all_events.extend(snap.events)

        # 多笔场景下，应该有 StrokeSettled 事件
        settled = [e for e in all_events if isinstance(e, StrokeSettled)]
        assert len(settled) >= 1, "Expected at least one StrokeSettled event"

    def test_reset_clears_state(self):
        """reset() 后状态归零。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(30)
        for bar in bars:
            engine.process_bar(bar)

        # 确认有数据
        assert engine.bar_count > 0
        assert engine.event_seq > 0

        # reset
        engine.reset()
        assert engine.bar_count == 0
        assert engine.current_strokes == []
        assert engine.event_seq == 0

    def test_process_bar_returns_snapshot(self):
        """每次 process_bar 返回正确的 BiEngineSnapshot。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(10)
        for k, bar in enumerate(bars):
            snap = engine.process_bar(bar)
            assert isinstance(snap, BiEngineSnapshot)
            assert snap.bar_idx == k
            assert isinstance(snap.bar_ts, float)
            assert isinstance(snap.strokes, list)
            assert isinstance(snap.events, list)
            assert snap.n_merged >= 1  # 至少有 1 根 merged bar
            assert snap.n_fractals >= 0

    def test_events_have_correct_bar_idx(self):
        """事件的 bar_idx 与当前 bar 一致。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(60)
        for k, bar in enumerate(bars):
            snap = engine.process_bar(bar)
            for event in snap.events:
                assert event.bar_idx == k, (
                    f"Event bar_idx={event.bar_idx} != current bar k={k}"
                )

    def test_event_seq_globally_monotonic(self):
        """全局事件序号严格单调递增。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(60)
        all_seqs: list[int] = []
        for bar in bars:
            snap = engine.process_bar(bar)
            for event in snap.events:
                all_seqs.append(event.seq)

        if len(all_seqs) > 1:
            for i in range(1, len(all_seqs)):
                assert all_seqs[i] > all_seqs[i - 1], (
                    f"seq not monotonic at position {i}: "
                    f"{all_seqs[i-1]} -> {all_seqs[i]}"
                )

    def test_strokes_direction_alternates(self):
        """最终笔序列的方向严格交替。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(60)
        for bar in bars:
            engine.process_bar(bar)

        strokes = engine.current_strokes
        if len(strokes) >= 2:
            for i in range(1, len(strokes)):
                assert strokes[i].direction != strokes[i - 1].direction, (
                    f"Direction not alternating at stroke {i}: "
                    f"{strokes[i-1].direction} -> {strokes[i].direction}"
                )

    def test_last_stroke_unconfirmed(self):
        """最后一笔 confirmed=False。"""
        engine = BiEngine()
        bars = _generate_zigzag_bars(60)
        for bar in bars:
            engine.process_bar(bar)

        strokes = engine.current_strokes
        if strokes:
            assert strokes[-1].confirmed is False
            # 非末笔全部 confirmed=True
            for s in strokes[:-1]:
                assert s.confirmed is True
