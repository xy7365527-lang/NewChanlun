"""BackpressureQueue 测试 — 正常入队、队列满丢弃、信号帧保留、队列清空。"""

from __future__ import annotations

import asyncio

import pytest

from newchan.backpressure import BackpressureQueue, is_critical_frame


# ── 辅助工厂 ──


def _bar_msg(idx: int = 0) -> dict:
    return {"type": "bar", "idx": idx, "ts": 0.0, "o": 1, "h": 2, "l": 0.5, "c": 1.5}


def _event_msg(event_type: str = "stroke_candidate", bar_idx: int = 0) -> dict:
    return {"type": "event", "event_type": event_type, "bar_idx": bar_idx}


def _bsp_msg(event_type: str = "bsp_confirm", bar_idx: int = 0) -> dict:
    return {"type": "event", "event_type": event_type, "bar_idx": bar_idx}


def _snapshot_msg(bar_idx: int = 0) -> dict:
    return {"type": "snapshot", "bar_idx": bar_idx, "strokes": [], "event_count": 0}


# ── is_critical_frame ──


class TestIsCriticalFrame:
    def test_bsp_events_are_critical(self):
        for et in ("bsp_candidate", "bsp_confirm", "bsp_settle", "bsp_invalidate"):
            assert is_critical_frame(_bsp_msg(et)) is True

    def test_non_bsp_event_not_critical(self):
        assert is_critical_frame(_event_msg("stroke_candidate")) is False
        assert is_critical_frame(_event_msg("segment_settle")) is False

    def test_bar_not_critical(self):
        assert is_critical_frame(_bar_msg()) is False

    def test_snapshot_not_critical(self):
        assert is_critical_frame(_snapshot_msg()) is False

    def test_empty_dict_not_critical(self):
        assert is_critical_frame({}) is False


# ── 正常入队 ──


class TestNormalEnqueue:
    def test_enqueue_under_capacity(self):
        q = BackpressureQueue(maxsize=10)
        for i in range(10):
            assert q.put_nowait(_bar_msg(i)) is True
        assert q.qsize() == 10
        assert q.dropped_count == 0

    def test_fifo_order(self):
        q = BackpressureQueue(maxsize=5)
        for i in range(3):
            q.put_nowait(_bar_msg(i))
        for i in range(3):
            msg = q.get_nowait()
            assert msg["idx"] == i

    def test_empty_and_full(self):
        q = BackpressureQueue(maxsize=2)
        assert q.empty() is True
        assert q.full() is False
        q.put_nowait(_bar_msg(0))
        q.put_nowait(_bar_msg(1))
        assert q.empty() is False
        assert q.full() is True


# ── 队列满丢弃非关键帧 ──


class TestDropNonCritical:
    def test_non_critical_dropped_when_full(self):
        q = BackpressureQueue(maxsize=3)
        for i in range(3):
            q.put_nowait(_bar_msg(i))
        assert q.full() is True

        # 再入一个非关键帧 → 丢弃
        assert q.put_nowait(_bar_msg(99)) is False
        assert q.qsize() == 3
        assert q.dropped_count == 1

    def test_multiple_drops_counted(self):
        q = BackpressureQueue(maxsize=2)
        q.put_nowait(_bar_msg(0))
        q.put_nowait(_bar_msg(1))

        for _ in range(5):
            q.put_nowait(_event_msg("stroke_candidate"))
        assert q.dropped_count == 5
        assert q.qsize() == 2


# ── 信号帧保留 ──


class TestCriticalFramePreserved:
    def test_bsp_evicts_oldest_non_critical(self):
        q = BackpressureQueue(maxsize=3)
        q.put_nowait(_bar_msg(0))       # 非关键
        q.put_nowait(_bar_msg(1))       # 非关键
        q.put_nowait(_event_msg())      # 非关键
        assert q.full() is True

        # bsp 帧入队 → 驱逐最旧的非关键帧 (bar idx=0)
        assert q.put_nowait(_bsp_msg("bsp_confirm")) is True
        assert q.qsize() == 3
        assert q.dropped_count == 1

        # 验证 bar(0) 被驱逐，剩余顺序：bar(1), stroke_candidate, bsp_confirm
        msg0 = q.get_nowait()
        assert msg0["type"] == "bar" and msg0["idx"] == 1
        msg1 = q.get_nowait()
        assert msg1["event_type"] == "stroke_candidate"
        msg2 = q.get_nowait()
        assert msg2["event_type"] == "bsp_confirm"

    def test_bsp_force_enqueue_when_all_critical(self):
        """队列全是关键帧时，新的关键帧仍然入队（突破 maxsize）。"""
        q = BackpressureQueue(maxsize=2)
        q.put_nowait(_bsp_msg("bsp_candidate"))
        q.put_nowait(_bsp_msg("bsp_confirm"))
        assert q.full() is True

        # 再入一个 bsp → 强制入队
        assert q.put_nowait(_bsp_msg("bsp_settle")) is True
        assert q.qsize() == 3  # 突破 maxsize
        assert q.dropped_count == 0

    def test_mixed_queue_eviction_targets_non_critical(self):
        """混合队列中，bsp 入队只驱逐非关键帧。"""
        q = BackpressureQueue(maxsize=4)
        q.put_nowait(_bsp_msg("bsp_candidate"))  # 关键
        q.put_nowait(_bar_msg(0))                 # 非关键
        q.put_nowait(_bsp_msg("bsp_confirm"))     # 关键
        q.put_nowait(_snapshot_msg())              # 非关键

        # 入 bsp → 驱逐最旧的非关键帧 (bar idx=0)
        assert q.put_nowait(_bsp_msg("bsp_settle")) is True
        assert q.qsize() == 4
        assert q.dropped_count == 1

        msgs = [q.get_nowait() for _ in range(4)]
        types = [(m.get("type"), m.get("event_type", "")) for m in msgs]
        assert types == [
            ("event", "bsp_candidate"),
            ("event", "bsp_confirm"),
            ("snapshot", ""),
            ("event", "bsp_settle"),
        ]


# ── 队列清空 ──


class TestClear:
    def test_clear_returns_count(self):
        q = BackpressureQueue(maxsize=10)
        for i in range(5):
            q.put_nowait(_bar_msg(i))
        cleared = q.clear()
        assert cleared == 5
        assert q.qsize() == 0
        assert q.empty() is True

    def test_clear_empty_queue(self):
        q = BackpressureQueue(maxsize=10)
        assert q.clear() == 0

    def test_clear_does_not_reset_dropped_count(self):
        q = BackpressureQueue(maxsize=1)
        q.put_nowait(_bar_msg(0))
        q.put_nowait(_bar_msg(1))  # dropped
        assert q.dropped_count == 1
        q.clear()
        assert q.dropped_count == 1


# ── async get ──


class TestAsyncGet:
    @pytest.mark.asyncio
    async def test_get_waits_for_item(self):
        q = BackpressureQueue(maxsize=10)

        async def _delayed_put():
            await asyncio.sleep(0.01)
            q.put_nowait(_bar_msg(42))

        asyncio.create_task(_delayed_put())
        msg = await asyncio.wait_for(q.get(), timeout=1.0)
        assert msg["idx"] == 42

    @pytest.mark.asyncio
    async def test_get_immediate_when_non_empty(self):
        q = BackpressureQueue(maxsize=10)
        q.put_nowait(_bar_msg(7))
        msg = await asyncio.wait_for(q.get(), timeout=0.1)
        assert msg["idx"] == 7


# ── 边界 ──


class TestEdgeCases:
    def test_maxsize_one(self):
        q = BackpressureQueue(maxsize=1)
        q.put_nowait(_bar_msg(0))
        assert q.put_nowait(_bar_msg(1)) is False
        assert q.put_nowait(_bsp_msg()) is True  # 驱逐 bar(0)
        assert q.qsize() == 1
        assert q.get_nowait()["event_type"] == "bsp_confirm"

    def test_invalid_maxsize(self):
        with pytest.raises(ValueError):
            BackpressureQueue(maxsize=0)
        with pytest.raises(ValueError):
            BackpressureQueue(maxsize=-1)

    def test_get_nowait_empty_raises(self):
        q = BackpressureQueue(maxsize=5)
        with pytest.raises(IndexError):
            q.get_nowait()
