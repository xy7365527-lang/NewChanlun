"""ReconnectingLiveEngine 测试 — 断线检测 + 指数退避重连"""

from __future__ import annotations

import asyncio
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock

import pytest

from newchan.live_engine import GapInfo, LiveEngine, ReconnectingLiveEngine
from newchan.types import Bar


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_bar(ts: datetime, price: float = 100.0) -> Bar:
    return Bar(ts=ts, open=price, high=price + 1, low=price - 1, close=price, volume=10)


def _ts(minutes: int) -> datetime:
    """UTC 时间戳，从 2026-01-01 00:00 起偏移 minutes 分钟。"""
    return datetime(2026, 1, 1, tzinfo=timezone.utc) + timedelta(minutes=minutes)


async def _bar_stream_factory(bars: list[tuple[str, Bar]]):
    """构造一个 async generator 作为 subscribe_fn 的返回值。"""
    async def subscribe_fn(_symbols: list[str]):
        for item in bars:
            yield item
    return subscribe_fn


# ---------------------------------------------------------------------------
# 正常重连
# ---------------------------------------------------------------------------

class TestNormalReconnect:
    """连接断开后能自动重连并继续消费 bar。"""

    @pytest.mark.asyncio
    async def test_reconnect_after_disconnect(self):
        engine = LiveEngine()
        call_count = 0
        bars_round1 = [("ES", _make_bar(_ts(1))), ("ES", _make_bar(_ts(2)))]
        bars_round2 = [("ES", _make_bar(_ts(5))), ("ES", _make_bar(_ts(6)))]

        async def connect_fn():
            nonlocal call_count
            call_count += 1

        round_idx = 0
        round2_done = asyncio.Event()

        async def subscribe_fn(_symbols):
            nonlocal round_idx
            round_idx += 1
            if round_idx == 1:
                for item in bars_round1:
                    yield item
                raise ConnectionError("stream lost")
            else:
                for item in bars_round2:
                    yield item
                round2_done.set()
                # 挂起直到 stop，避免空流循环
                await asyncio.sleep(10)

        async def fetch_history(_sym, _start, _end):
            return []

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=connect_fn,
            subscribe_fn=subscribe_fn,
            fetch_history_fn=fetch_history,
            symbols=["ES"],
            initial_backoff=0.01,
            max_backoff=0.1,
        )

        async def stop_after():
            await round2_done.wait()
            rle.stop()

        await asyncio.gather(rle.run(), stop_after())

        assert engine.bar_count("ES") == 4
        assert call_count >= 2
        assert rle.reconnect_count >= 1


# ---------------------------------------------------------------------------
# 退避递增
# ---------------------------------------------------------------------------

class TestBackoffEscalation:
    """连续失败时退避指数递增，不超过 max_backoff。"""

    @pytest.mark.asyncio
    async def test_backoff_doubles(self):
        engine = LiveEngine()

        fail_count = 0

        async def connect_fn():
            nonlocal fail_count
            fail_count += 1
            if fail_count <= 4:
                raise ConnectionError("refused")
            # 第5次成功

        connected_event = asyncio.Event()

        async def subscribe_fn(_symbols):
            # 通知测试：已成功连接并进入消费阶段
            connected_event.set()
            # 挂起直到 stop，避免空流导致无限循环
            await asyncio.sleep(10)
            return
            yield  # noqa: unreachable — 使其成为 async generator

        async def fetch_history(_sym, _start, _end):
            return []

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=connect_fn,
            subscribe_fn=subscribe_fn,
            fetch_history_fn=fetch_history,
            symbols=["ES"],
            initial_backoff=0.01,
            max_backoff=0.08,
            backoff_factor=2.0,
        )

        async def stop_after_connected():
            await connected_event.wait()
            rle.stop()

        await asyncio.gather(rle.run(), stop_after_connected())

        # 成功连接后 backoff 已 reset
        assert rle.current_backoff == pytest.approx(0.01)

    @pytest.mark.asyncio
    async def test_backoff_capped_at_max(self):
        """退避值不超过 max_backoff。"""
        engine = LiveEngine()

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=AsyncMock(),
            subscribe_fn=AsyncMock(),
            fetch_history_fn=AsyncMock(),
            symbols=["ES"],
            initial_backoff=1.0,
            max_backoff=5.0,
            backoff_factor=3.0,
        )

        # 手动推进退避
        waits = []
        for _ in range(10):
            waits.append(rle._advance_backoff())

        # 1.0, 3.0, 5.0, 5.0, 5.0, ...
        assert waits[0] == pytest.approx(1.0)
        assert waits[1] == pytest.approx(3.0)
        for w in waits[2:]:
            assert w == pytest.approx(5.0)

    @pytest.mark.asyncio
    async def test_backoff_resets_on_success(self):
        """成功连接后退避重置为 initial_backoff。"""
        engine = LiveEngine()

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=AsyncMock(),
            subscribe_fn=AsyncMock(),
            fetch_history_fn=AsyncMock(),
            symbols=["ES"],
            initial_backoff=1.0,
            max_backoff=60.0,
            backoff_factor=2.0,
        )

        # 推进几次
        rle._advance_backoff()
        rle._advance_backoff()
        assert rle.current_backoff == pytest.approx(4.0)

        rle._reset_backoff()
        assert rle.current_backoff == pytest.approx(1.0)


# ---------------------------------------------------------------------------
# Gap 检测与回填
# ---------------------------------------------------------------------------

class TestGapDetection:
    """断线后重连时检测并回填数据 gap。"""

    @pytest.mark.asyncio
    async def test_gap_filled_on_reconnect(self):
        engine = LiveEngine()
        # 预先喂入一根 bar，建立 last_bar_ts
        engine.process_bar("ES", _make_bar(_ts(0), 100.0))

        # gap bars 的时间戳必须在 round1 最后一根 bar 之后
        # round1 最后 bar 是 ts(3)，所以 gap bars 应该是 ts(4), ts(5)
        gap_bars = [
            _make_bar(_ts(4), 104.0),
            _make_bar(_ts(5), 105.0),
        ]

        async def fetch_history(symbol, start, end):
            return gap_bars

        round_idx = 0
        round2_done = asyncio.Event()

        async def connect_fn():
            pass

        async def subscribe_fn(_symbols):
            nonlocal round_idx
            round_idx += 1
            if round_idx == 1:
                yield ("ES", _make_bar(_ts(3), 103.0))
                raise ConnectionError("断线")
            else:
                yield ("ES", _make_bar(_ts(8), 108.0))
                round2_done.set()
                await asyncio.sleep(10)

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=connect_fn,
            subscribe_fn=subscribe_fn,
            fetch_history_fn=fetch_history,
            symbols=["ES"],
            initial_backoff=0.01,
            max_backoff=0.05,
        )

        async def stop_after():
            await round2_done.wait()
            rle.stop()

        await asyncio.gather(rle.run(), stop_after())

        # 1(预喂 ts0) + 1(round1 ts3) + 2(gap fill ts4,ts5) + 1(round2 ts8) = 5
        assert engine.bar_count("ES") == 5
        assert len(rle.gaps) >= 1
        assert rle.gaps[0].bars_filled == 2

    @pytest.mark.asyncio
    async def test_gap_fill_with_correct_timestamps(self):
        """gap 回填只处理 gap_start 之后的 bar。"""
        engine = LiveEngine()
        engine.process_bar("ES", _make_bar(_ts(0), 100.0))

        # 模拟 _fill_gap 直接调用
        gap_bars = [
            _make_bar(_ts(0), 100.0),  # == gap_start, 应跳过
            _make_bar(_ts(1), 101.0),  # > gap_start, 应处理
            _make_bar(_ts(2), 102.0),  # > gap_start, 应处理
        ]

        async def fetch_history(_sym, _start, _end):
            return gap_bars

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=AsyncMock(),
            subscribe_fn=AsyncMock(),
            fetch_history_fn=fetch_history,
            symbols=["ES"],
        )

        gap = await rle._fill_gap("ES", _ts(5))
        assert gap is not None
        assert gap.bars_filled == 2
        assert engine.bar_count("ES") == 3  # 1 预喂 + 2 回填

    @pytest.mark.asyncio
    async def test_no_gap_when_no_prior_bars(self):
        """没有历史 bar 时不触发 gap 回填。"""
        engine = LiveEngine()

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=AsyncMock(),
            subscribe_fn=AsyncMock(),
            fetch_history_fn=AsyncMock(),
            symbols=["ES"],
        )

        gap = await rle._fill_gap("ES", _ts(5))
        assert gap is None

    @pytest.mark.asyncio
    async def test_fetch_history_failure_returns_zero_filled(self):
        """历史 API 失败时 gap 记录 bars_filled=0。"""
        engine = LiveEngine()
        engine.process_bar("ES", _make_bar(_ts(0)))

        async def fetch_history(_sym, _start, _end):
            raise RuntimeError("API down")

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=AsyncMock(),
            subscribe_fn=AsyncMock(),
            fetch_history_fn=fetch_history,
            symbols=["ES"],
        )

        gap = await rle._fill_gap("ES", _ts(5))
        assert gap is not None
        assert gap.bars_filled == 0


# ---------------------------------------------------------------------------
# 状态连续性
# ---------------------------------------------------------------------------

class TestStateContinuity:
    """重连后 RecursiveOrchestrator 状态不重置。"""

    @pytest.mark.asyncio
    async def test_orchestrator_not_reset_on_reconnect(self):
        engine = LiveEngine()
        # 喂入一些 bar
        for i in range(5):
            engine.process_bar("ES", _make_bar(_ts(i), 100.0 + i))

        original_count = engine.bar_count("ES")

        round_idx = 0
        round2_done = asyncio.Event()

        async def connect_fn():
            pass

        async def subscribe_fn(_symbols):
            nonlocal round_idx
            round_idx += 1
            if round_idx == 1:
                yield ("ES", _make_bar(_ts(10), 110.0))
                raise ConnectionError("断线")
            else:
                yield ("ES", _make_bar(_ts(11), 111.0))
                round2_done.set()
                await asyncio.sleep(10)

        async def fetch_history(_sym, _start, _end):
            return []

        rle = ReconnectingLiveEngine(
            engine=engine,
            connect_fn=connect_fn,
            subscribe_fn=subscribe_fn,
            fetch_history_fn=fetch_history,
            symbols=["ES"],
            initial_backoff=0.01,
            max_backoff=0.05,
        )

        async def stop_after():
            await round2_done.wait()
            rle.stop()

        await asyncio.gather(rle.run(), stop_after())

        # bar_count 应该是累加的，不是重置的
        assert engine.bar_count("ES") == original_count + 2
        # orchestrator 实例应该是同一个（未重建）
        assert "ES" in engine.symbols


# ---------------------------------------------------------------------------
# GapInfo repr
# ---------------------------------------------------------------------------

class TestGapInfo:
    def test_repr(self):
        gap = GapInfo("ES", _ts(0), _ts(5), bars_filled=3)
        r = repr(gap)
        assert "ES" in r
        assert "filled=3" in r
