"""事件不可变性与确定性测试

验证 MVP-A PR-A1 引入的 event_id / schema_version 字段：
  - event_id 非空、格式正确、确定性、流内唯一
  - schema_version 固定为 1
  - 两次独立运行产生完全相同的事件 payload
  - compute_event_id / compute_stream_fingerprint 纯函数性
  - 回放 seek 后 event_id 序列与逐步运行一致
"""

from __future__ import annotations

import re
from datetime import datetime, timedelta, timezone

import pytest

from newchan.bi_engine import BiEngine
from newchan.events import DomainEvent
from newchan.fingerprint import compute_event_id, compute_stream_fingerprint
from newchan.replay import ReplaySession
from newchan.types import Bar


# ── 辅助函数（与 test_bi_engine.py 保持一致）──────────────────────


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


def _collect_all_events(bars: list[Bar]) -> list[DomainEvent]:
    """跑完所有 bar，收集全部事件。"""
    engine = BiEngine()
    all_events: list[DomainEvent] = []
    for bar in bars:
        snap = engine.process_bar(bar)
        all_events.extend(snap.events)
    return all_events


# 16 hex 字符的正则
_HEX16_RE = re.compile(r"^[0-9a-f]{16}$")


# =====================================================================
# 测试
# =====================================================================


class TestEventImmutability:
    """事件 event_id / schema_version 不可变性测试。"""

    def test_event_id_nonempty(self):
        """所有事件的 event_id 非空。"""
        bars = _generate_zigzag_bars(60)
        events = _collect_all_events(bars)

        assert len(events) > 0, "应至少产生一个事件"
        for i, ev in enumerate(events):
            assert ev.event_id, (
                f"事件 #{i} (seq={ev.seq}, type={ev.event_type}) 的 event_id 为空"
            )

    def test_event_id_is_16_hex(self):
        """event_id 格式验证：16 个十六进制字符。"""
        bars = _generate_zigzag_bars(60)
        events = _collect_all_events(bars)

        assert len(events) > 0, "应至少产生一个事件"
        for i, ev in enumerate(events):
            assert _HEX16_RE.match(ev.event_id), (
                f"事件 #{i} event_id={ev.event_id!r} 不是 16 hex 字符"
            )

    def test_same_input_same_event_id(self):
        """两个独立 BiEngine 跑相同 bars，event_id 序列完全一致。"""
        bars = _generate_zigzag_bars(60)

        events_a = _collect_all_events(bars)
        events_b = _collect_all_events(bars)

        assert len(events_a) > 0, "应至少产生一个事件"
        assert len(events_a) == len(events_b), (
            f"事件数量不一致: {len(events_a)} vs {len(events_b)}"
        )
        for i, (a, b) in enumerate(zip(events_a, events_b)):
            assert a.event_id == b.event_id, (
                f"事件 #{i} event_id 不一致: {a.event_id} vs {b.event_id}"
            )

    def test_event_id_unique_within_stream(self):
        """同一事件流内所有 event_id 唯一。"""
        bars = _generate_zigzag_bars(60)
        events = _collect_all_events(bars)

        assert len(events) > 0, "应至少产生一个事件"
        ids = [ev.event_id for ev in events]
        assert len(ids) == len(set(ids)), (
            f"存在重复 event_id: "
            f"总数 {len(ids)}, 去重后 {len(set(ids))}"
        )

    def test_schema_version_is_one(self):
        """所有事件的 schema_version == 1。"""
        bars = _generate_zigzag_bars(60)
        events = _collect_all_events(bars)

        assert len(events) > 0, "应至少产生一个事件"
        for i, ev in enumerate(events):
            assert ev.schema_version == 1, (
                f"事件 #{i} schema_version={ev.schema_version}, 期望 1"
            )

    def test_frozen_payload_deterministic(self):
        """同一事件的所有字段在两次运行间完全一致。"""
        bars = _generate_zigzag_bars(60)

        events_a = _collect_all_events(bars)
        events_b = _collect_all_events(bars)

        assert len(events_a) > 0, "应至少产生一个事件"
        assert len(events_a) == len(events_b)

        for i, (a, b) in enumerate(zip(events_a, events_b)):
            # 比较 frozen dataclass 的所有字段完全相等
            assert a == b, (
                f"事件 #{i} 两次运行不一致:\n  run1: {a}\n  run2: {b}"
            )

    def test_stream_fingerprint_deterministic(self):
        """compute_stream_fingerprint 两次运行结果一致。"""
        bars = _generate_zigzag_bars(60)

        events_a = _collect_all_events(bars)
        events_b = _collect_all_events(bars)

        fp_a = compute_stream_fingerprint(events_a)
        fp_b = compute_stream_fingerprint(events_b)

        assert fp_a, "fingerprint 不应为空"
        assert fp_a == fp_b, (
            f"fingerprint 不一致: {fp_a} vs {fp_b}"
        )

    def test_stream_fingerprint_changes_on_different_input(self):
        """不同输入产生不同 fingerprint。"""
        bars_60 = _generate_zigzag_bars(60)
        bars_80 = _generate_zigzag_bars(80)

        events_60 = _collect_all_events(bars_60)
        events_80 = _collect_all_events(bars_80)

        fp_60 = compute_stream_fingerprint(events_60)
        fp_80 = compute_stream_fingerprint(events_80)

        assert fp_60, "fingerprint_60 不应为空"
        assert fp_80, "fingerprint_80 不应为空"
        assert fp_60 != fp_80, (
            f"不同输入产生了相同的 fingerprint: {fp_60}"
        )

    def test_compute_event_id_pure_function(self):
        """compute_event_id 同输入同输出（纯函数）。"""
        payload = {
            "stroke_id": 0,
            "direction": "up",
            "i0": 0,
            "i1": 5,
            "p0": 10.0,
            "p1": 20.0,
        }

        id_a = compute_event_id(
            bar_idx=10,
            bar_ts=1707800000.0,
            event_type="stroke_candidate",
            seq=42,
            payload=payload,
        )
        id_b = compute_event_id(
            bar_idx=10,
            bar_ts=1707800000.0,
            event_type="stroke_candidate",
            seq=42,
            payload=payload,
        )

        assert _HEX16_RE.match(id_a), f"event_id 格式错误: {id_a!r}"
        assert id_a == id_b, f"纯函数性违反: {id_a} vs {id_b}"

        # 修改一个参数，event_id 必须不同
        id_c = compute_event_id(
            bar_idx=10,
            bar_ts=1707800000.0,
            event_type="stroke_candidate",
            seq=43,  # seq 不同
            payload=payload,
        )
        assert id_a != id_c, (
            f"不同 seq 产生了相同 event_id: {id_a}"
        )

    def test_event_id_after_seek(self):
        """回放 seek 后重跑，event_id 序列与逐步运行一致。"""
        bars = _generate_zigzag_bars(60)
        seek_target = 39  # seek 到第 40 根 bar（0-based）

        # ── 方式 1：逐步运行到 seek_target ──
        engine_step = BiEngine()
        events_step: list[DomainEvent] = []
        for i in range(seek_target + 1):
            snap = engine_step.process_bar(bars[i])
            events_step.extend(snap.events)

        # ── 方式 2：通过 ReplaySession.seek 跳转 ──
        session = ReplaySession(
            session_id="test-seek",
            bars=bars,
            engine=BiEngine(),
        )
        session.seek(seek_target)

        # 从 event_log 收集所有事件
        events_seek: list[DomainEvent] = []
        for snap in session.event_log:
            events_seek.extend(snap.events)

        # 两种方式产生的事件数量和 event_id 序列必须完全一致
        assert len(events_step) == len(events_seek), (
            f"事件数量不一致: step={len(events_step)}, seek={len(events_seek)}"
        )
        for i, (ev_step, ev_seek) in enumerate(zip(events_step, events_seek)):
            assert ev_step.event_id == ev_seek.event_id, (
                f"事件 #{i} event_id 不一致 (seek 后):\n"
                f"  step: {ev_step.event_id}\n"
                f"  seek: {ev_seek.event_id}"
            )
