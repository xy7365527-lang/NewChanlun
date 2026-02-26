"""Tests for LivePool — 多标的并发管理池。"""

from __future__ import annotations

import asyncio
from datetime import datetime

import pytest

from newchan.live_pool import LivePool
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.types import Bar


def _make_bar(idx: int = 0) -> Bar:
    return Bar(
        ts=datetime(2025, 1, 1, 9, 30 + idx),
        open=100.0 + idx,
        high=101.0 + idx,
        low=99.0 + idx,
        close=100.5 + idx,
        volume=1000.0,
    )


# ── add / remove ──────────────────────────────────────


@pytest.mark.asyncio
async def test_add_symbol() -> None:
    pool = LivePool(max_symbols=5)
    await pool.add_symbol("AAPL")
    assert "AAPL" in pool.active_symbols
    await pool.shutdown()


@pytest.mark.asyncio
async def test_add_symbol_idempotent() -> None:
    pool = LivePool(max_symbols=5)
    await pool.add_symbol("AAPL")
    await pool.add_symbol("aapl")  # 大小写归一化，幂等
    assert pool.active_symbols.count("AAPL") == 1
    await pool.shutdown()


@pytest.mark.asyncio
async def test_remove_symbol() -> None:
    pool = LivePool(max_symbols=5)
    await pool.add_symbol("AAPL")
    await pool.remove_symbol("AAPL")
    assert "AAPL" not in pool.active_symbols
    await pool.shutdown()


@pytest.mark.asyncio
async def test_remove_nonexistent_symbol() -> None:
    pool = LivePool(max_symbols=5)
    await pool.remove_symbol("NOPE")  # 不报错
    await pool.shutdown()


# ── 并发上限 ──────────────────────────────────────────


@pytest.mark.asyncio
async def test_max_symbols_enforced() -> None:
    pool = LivePool(max_symbols=2)
    await pool.add_symbol("A")
    await pool.add_symbol("B")
    with pytest.raises(ValueError, match="并发上限"):
        await pool.add_symbol("C")
    await pool.shutdown()


@pytest.mark.asyncio
async def test_remove_then_add_within_limit() -> None:
    pool = LivePool(max_symbols=2)
    await pool.add_symbol("A")
    await pool.add_symbol("B")
    await pool.remove_symbol("A")
    await pool.add_symbol("C")  # 腾出位置后可以添加
    assert sorted(pool.active_symbols) == ["B", "C"]
    await pool.shutdown()


# ── 状态查询 ──────────────────────────────────────────


@pytest.mark.asyncio
async def test_get_status_empty() -> None:
    pool = LivePool(max_symbols=10)
    status = pool.get_status()
    assert status["max_symbols"] == 10
    assert status["active_count"] == 0
    assert status["symbols"] == {}
    await pool.shutdown()


@pytest.mark.asyncio
async def test_get_status_with_symbols() -> None:
    pool = LivePool(max_symbols=10)
    await pool.add_symbol("AAPL")
    await pool.add_symbol("GOOG")
    status = pool.get_status()
    assert status["active_count"] == 2
    assert "AAPL" in status["symbols"]
    assert "GOOG" in status["symbols"]
    assert status["symbols"]["AAPL"]["task_alive"] is True
    await pool.shutdown()


# ── bar 投递与回调 ────────────────────────────────────


@pytest.mark.asyncio
async def test_feed_bar_triggers_callback() -> None:
    received: list[tuple[str, int]] = []

    async def on_update(
        symbol: str,
        bar: Bar,
        bar_idx: int,
        snap: RecursiveOrchestratorSnapshot,
    ) -> None:
        received.append((symbol, bar_idx))

    pool = LivePool(max_symbols=5, on_update=on_update)
    await pool.add_symbol("AAPL")

    pool.feed_bar("AAPL", _make_bar(0))
    pool.feed_bar("AAPL", _make_bar(1))

    # 让事件循环处理 queue
    await asyncio.sleep(0.1)

    assert len(received) == 2
    assert received[0] == ("AAPL", 0)
    assert received[1] == ("AAPL", 1)
    await pool.shutdown()


@pytest.mark.asyncio
async def test_feed_bar_unknown_symbol_ignored() -> None:
    pool = LivePool(max_symbols=5)
    pool.feed_bar("NOPE", _make_bar(0))  # 不报错
    await pool.shutdown()


@pytest.mark.asyncio
async def test_bar_count_in_status() -> None:
    pool = LivePool(max_symbols=5)
    await pool.add_symbol("AAPL")

    pool.feed_bar("AAPL", _make_bar(0))
    pool.feed_bar("AAPL", _make_bar(1))
    pool.feed_bar("AAPL", _make_bar(2))
    await asyncio.sleep(0.1)

    status = pool.get_status()
    assert status["symbols"]["AAPL"]["bar_count"] == 3
    await pool.shutdown()


# ── shutdown ──────────────────────────────────────────


@pytest.mark.asyncio
async def test_shutdown_cleans_all() -> None:
    pool = LivePool(max_symbols=5)
    await pool.add_symbol("A")
    await pool.add_symbol("B")
    await pool.shutdown()
    assert pool.active_symbols == []
    assert pool.get_status()["active_count"] == 0
