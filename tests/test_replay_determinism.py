"""回放确定性测试

验证：
1. 两次独立运行 BiEngine 产生完全相同的事件序列（确定性）
2. seek 到中间位置的快照 == 从头逐 bar 跑到该位置（一致性）
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.replay import ReplaySession
from newchan.types import Bar


# ── 测试数据生成（与 test_no_future.py 保持一致的逻辑）────────────


def _generate_test_bars(n: int = 60) -> list[Bar]:
    """生成锯齿形价格序列。

    每 16 根 bar 一个完整周期（8 根下降 + 8 根上升），
    价格在 60-100 之间振荡，确保能产生多笔。
    """
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            base = 100 - cycle_pos * 5
        else:
            base = 65 + (cycle_pos - 8) * 5

        h = base + 1.5
        l = base - 1.5
        o = base + 0.5
        c = base - 0.5
        bars.append(
            Bar(
                ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
                + timedelta(minutes=i * 5),
                open=o,
                high=h,
                low=l,
                close=c,
            )
        )
    return bars


# =====================================================================
# 测试类
# =====================================================================


class TestReplayDeterminism:
    """两次独立运行 BiEngine 产生完全相同的事件序列。"""

    def test_deterministic(self):
        """两次独立运行结果完全一致。"""
        bars = _generate_test_bars(60)

        # 第一次运行
        engine1 = BiEngine()
        events1 = []
        for bar in bars:
            snap = engine1.process_bar(bar)
            events1.extend(snap.events)

        # 第二次运行
        engine2 = BiEngine()
        events2 = []
        for bar in bars:
            snap = engine2.process_bar(bar)
            events2.extend(snap.events)

        assert len(events1) == len(events2), (
            f"Event count mismatch: run1={len(events1)}, run2={len(events2)}"
        )
        for i, (a, b) in enumerate(zip(events1, events2)):
            assert a.event_type == b.event_type, (
                f"Event {i}: event_type mismatch: {a.event_type} vs {b.event_type}"
            )
            assert a.bar_idx == b.bar_idx, (
                f"Event {i}: bar_idx mismatch: {a.bar_idx} vs {b.bar_idx}"
            )
            assert a.seq == b.seq, (
                f"Event {i}: seq mismatch: {a.seq} vs {b.seq}"
            )

    def test_deterministic_strokes(self):
        """两次独立运行的最终笔列表完全一致。"""
        bars = _generate_test_bars(60)

        engine1 = BiEngine()
        for bar in bars:
            engine1.process_bar(bar)

        engine2 = BiEngine()
        for bar in bars:
            engine2.process_bar(bar)

        strokes1 = engine1.current_strokes
        strokes2 = engine2.current_strokes

        assert len(strokes1) == len(strokes2)
        for a, b in zip(strokes1, strokes2):
            assert a.i0 == b.i0
            assert a.i1 == b.i1
            assert a.direction == b.direction
            assert a.confirmed == b.confirmed
            assert abs(a.p0 - b.p0) < 1e-9
            assert abs(a.p1 - b.p1) < 1e-9

    def test_seek_matches_sequential(self):
        """seek 到中间位置的快照 == 从头逐 bar 跑到该位置。"""
        bars = _generate_test_bars(60)
        target = 30

        # 顺序运行到 target
        engine_seq = BiEngine()
        snap_seq: BiEngineSnapshot | None = None
        for bar in bars[: target + 1]:
            snap_seq = engine_seq.process_bar(bar)
        assert snap_seq is not None

        # seek 方式
        session = ReplaySession("test", bars, BiEngine())
        snap_seek = session.seek(target)
        assert snap_seek is not None

        # 比较笔列表
        assert len(snap_seq.strokes) == len(snap_seek.strokes), (
            f"Stroke count mismatch: sequential={len(snap_seq.strokes)}, "
            f"seek={len(snap_seek.strokes)}"
        )
        for j, (a, b) in enumerate(zip(snap_seq.strokes, snap_seek.strokes)):
            assert a.i0 == b.i0, f"stroke {j}: i0 mismatch"
            assert a.i1 == b.i1, f"stroke {j}: i1 mismatch"
            assert a.direction == b.direction, f"stroke {j}: direction mismatch"
            assert a.confirmed == b.confirmed, f"stroke {j}: confirmed mismatch"
            assert abs(a.p0 - b.p0) < 1e-9, f"stroke {j}: p0 mismatch"
            assert abs(a.p1 - b.p1) < 1e-9, f"stroke {j}: p1 mismatch"

    def test_seek_to_end_matches(self):
        """seek 到最后一根 bar == 顺序跑完所有 bar。"""
        bars = _generate_test_bars(60)

        # 顺序运行
        engine_seq = BiEngine()
        snap_seq: BiEngineSnapshot | None = None
        for bar in bars:
            snap_seq = engine_seq.process_bar(bar)
        assert snap_seq is not None

        # seek 到末尾
        session = ReplaySession("test", bars, BiEngine())
        snap_seek = session.seek(len(bars) - 1)
        assert snap_seek is not None

        assert len(snap_seq.strokes) == len(snap_seek.strokes)
        for a, b in zip(snap_seq.strokes, snap_seek.strokes):
            assert a.i0 == b.i0
            assert a.i1 == b.i1
            assert a.direction == b.direction

    def test_seek_to_beginning(self):
        """seek 到 bar 0：引擎只处理了一根 bar。"""
        bars = _generate_test_bars(60)
        session = ReplaySession("test", bars, BiEngine())

        # 先跑一半
        session.step(30)

        # seek 回到开头
        snap = session.seek(0)
        assert snap is not None
        assert snap.bar_idx == 0
        assert snap.strokes == []  # 一根 bar 不可能有笔
        assert session.current_idx == 1  # 已处理了 bar[0]

    def test_replay_session_step(self):
        """ReplaySession step 与直接引擎调用一致。"""
        bars = _generate_test_bars(30)

        # 用 ReplaySession step
        session = ReplaySession("test", bars, BiEngine())
        snaps_session = session.step(len(bars))

        # 用 BiEngine 直接调用
        engine = BiEngine()
        snaps_direct = [engine.process_bar(bar) for bar in bars]

        assert len(snaps_session) == len(snaps_direct)
        for snap_s, snap_d in zip(snaps_session, snaps_direct):
            assert len(snap_s.strokes) == len(snap_d.strokes)
