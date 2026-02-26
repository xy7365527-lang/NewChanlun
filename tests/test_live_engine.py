"""LiveEngine 测试 — 模拟数据流驱动 RecursiveOrchestrator"""

from __future__ import annotations

from datetime import datetime, timezone

from newchan.live_engine import LiveEngine
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.types import Bar


def _make_bar(idx: int, base_price: float = 100.0) -> Bar:
    """生成一根合成 bar，价格围绕 base_price 波动。"""
    offset = idx * 0.5
    return Bar(
        ts=datetime(2025, 1, 1, 9, 30 + idx, tzinfo=timezone.utc),
        open=base_price + offset,
        high=base_price + offset + 1.0,
        low=base_price + offset - 0.5,
        close=base_price + offset + 0.3,
        volume=1000.0 + idx * 10,
    )


def _make_zigzag_bars(n: int, amplitude: float = 5.0) -> list[Bar]:
    """生成锯齿形 bar 序列，确保产生分型和笔。"""
    bars: list[Bar] = []
    base = 100.0
    for i in range(n):
        if (i // 3) % 2 == 0:
            price = base + (i % 3) * amplitude
        else:
            price = base + (3 - 1) * amplitude - (i % 3) * amplitude
        bars.append(Bar(
            ts=datetime(2025, 1, 1, 9, 30 + i, tzinfo=timezone.utc),
            open=price - 0.2,
            high=price + 1.0,
            low=price - 1.0,
            close=price + 0.2,
            volume=1000.0,
        ))
    return bars


class TestLiveEngineBasic:
    """基础功能测试。"""

    def test_process_single_bar(self):
        engine = LiveEngine()
        bar = _make_bar(0)
        snap = engine.process_bar("TEST", bar)

        assert isinstance(snap, RecursiveOrchestratorSnapshot)
        assert snap.bar_idx == 0
        assert engine.bar_count("TEST") == 1
        assert engine.symbols == ["TEST"]

    def test_process_multiple_bars(self):
        engine = LiveEngine()
        snaps = []
        for i in range(10):
            snap = engine.process_bar("TEST", _make_bar(i))
            snaps.append(snap)

        assert len(snaps) == 10
        assert engine.bar_count("TEST") == 10
        assert snaps[-1].bar_idx == 9

    def test_multiple_symbols(self):
        engine = LiveEngine()
        engine.process_bar("BZ", _make_bar(0, 70.0))
        engine.process_bar("ES", _make_bar(0, 5000.0))
        engine.process_bar("BZ", _make_bar(1, 70.0))

        assert set(engine.symbols) == {"BZ", "ES"}
        assert engine.bar_count("BZ") == 2
        assert engine.bar_count("ES") == 1

    def test_latest_snapshot(self):
        engine = LiveEngine()
        assert engine.latest_snapshot("TEST") is None

        bar = _make_bar(0)
        snap = engine.process_bar("TEST", bar)
        assert engine.latest_snapshot("TEST") is snap

    def test_bar_count_unknown_symbol(self):
        engine = LiveEngine()
        assert engine.bar_count("UNKNOWN") == 0


class TestLiveEngineListener:
    """监听器回调测试。"""

    def test_listener_called(self):
        engine = LiveEngine()
        received: list[tuple[str, Bar, RecursiveOrchestratorSnapshot]] = []

        def on_snap(symbol, bar, snap):
            received.append((symbol, bar, snap))

        engine.add_listener(on_snap)
        bar = _make_bar(0)
        engine.process_bar("TEST", bar)

        assert len(received) == 1
        assert received[0][0] == "TEST"
        assert received[0][1] is bar
        assert isinstance(received[0][2], RecursiveOrchestratorSnapshot)

    def test_listener_removed(self):
        engine = LiveEngine()
        call_count = 0

        def on_snap(symbol, bar, snap):
            nonlocal call_count
            call_count += 1

        engine.add_listener(on_snap)
        engine.process_bar("TEST", _make_bar(0))
        assert call_count == 1

        engine.remove_listener(on_snap)
        engine.process_bar("TEST", _make_bar(1))
        assert call_count == 1  # 不再增加

    def test_listener_error_does_not_break_engine(self):
        engine = LiveEngine()

        def bad_listener(symbol, bar, snap):
            raise ValueError("boom")

        engine.add_listener(bad_listener)
        # 不应抛出异常
        snap = engine.process_bar("TEST", _make_bar(0))
        assert snap is not None
        assert engine.bar_count("TEST") == 1


class TestLiveEngineReset:
    """重置功能测试。"""

    def test_reset_single_symbol(self):
        engine = LiveEngine()
        engine.process_bar("BZ", _make_bar(0))
        engine.process_bar("ES", _make_bar(0))

        engine.reset("BZ")
        assert engine.bar_count("BZ") == 0
        assert engine.latest_snapshot("BZ") is None
        assert engine.bar_count("ES") == 1  # ES 不受影响

    def test_reset_all(self):
        engine = LiveEngine()
        engine.process_bar("BZ", _make_bar(0))
        engine.process_bar("ES", _make_bar(0))

        engine.reset()
        assert engine.bar_count("BZ") == 0
        assert engine.bar_count("ES") == 0


class TestLiveEngineIncremental:
    """增量计算正确性 — 逐 bar 喂入与批量喂入结果一致。"""

    def test_incremental_matches_batch(self):
        bars = _make_zigzag_bars(30)

        # 方式 A：逐 bar 通过 LiveEngine
        live = LiveEngine()
        for bar in bars:
            live.process_bar("TEST", bar)
        live_snap = live.latest_snapshot("TEST")

        # 方式 B：直接用 RecursiveOrchestrator 批量处理
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        batch_orch = RecursiveOrchestrator()
        batch_snap = None
        for bar in bars:
            batch_snap = batch_orch.process_bar(bar)

        # 两者的笔数量和 bar_idx 应一致
        assert live_snap is not None
        assert batch_snap is not None
        assert live_snap.bar_idx == batch_snap.bar_idx
        assert len(live_snap.bi_snapshot.strokes) == len(batch_snap.bi_snapshot.strokes)
        assert len(live_snap.seg_snapshot.segments) == len(batch_snap.seg_snapshot.segments)
