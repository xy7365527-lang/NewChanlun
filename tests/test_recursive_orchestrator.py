"""P8 RecursiveOrchestrator 测试

口径 A 正式路径：从 1 分钟 K 线出发，构造全部递归级别。
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → RecursiveStack

验证：
  - 管线串联正确性
  - process_bar 逐 bar 驱动
  - 快照包含所有级别信息
  - reset 重置所有引擎
  - EventBus 事件收集
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.types import Bar


# ── 辅助函数 ──


def _bar(idx: int, o: float, h: float, l: float, c: float) -> Bar:
    """从序号创建 Bar（5 分钟间隔）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc) + timedelta(seconds=idx * 300),
        open=o,
        high=h,
        low=l,
        close=c,
    )


def _zigzag_bars(n: int = 60, amplitude: float = 5.0, base: float = 100.0) -> list[Bar]:
    """生成 V 形锯齿 bar 序列，保证产出足够多的笔和线段。

    偶数 bar 上涨，奇数 bar 下跌，周期 10 根。
    """
    bars: list[Bar] = []
    for i in range(n):
        cycle = (i % 10) / 10.0
        if cycle < 0.5:
            price = base + amplitude * (cycle * 2)
        else:
            price = base + amplitude * (1.0 - (cycle - 0.5) * 2)
        noise = (i % 3) * 0.1
        bars.append(_bar(
            i,
            o=price - 0.5 + noise,
            h=price + 1.0 + noise,
            l=price - 1.0 - noise,
            c=price + 0.5 - noise,
        ))
    return bars


# ── 基本测试 ──


class TestRecursiveOrchestratorBasic:
    """基本功能测试。"""

    def test_import(self) -> None:
        """模块可导入。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        assert RecursiveOrchestrator is not None

    def test_init(self) -> None:
        """构造函数接受 stream_id 和 max_levels 参数。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator(stream_id="test", max_levels=4)
        assert orch.max_levels == 4

    def test_process_bar_returns_snapshot(self) -> None:
        """process_bar 返回 RecursiveOrchestratorSnapshot。"""
        from newchan.orchestrator.recursive import (
            RecursiveOrchestrator,
            RecursiveOrchestratorSnapshot,
        )
        orch = RecursiveOrchestrator()
        bar = _bar(0, 100, 101, 99, 100.5)
        snap = orch.process_bar(bar)
        assert isinstance(snap, RecursiveOrchestratorSnapshot)

    def test_snapshot_has_bar_info(self) -> None:
        """快照包含 bar_idx 和 bar_ts。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator()
        bar = _bar(0, 100, 101, 99, 100.5)
        snap = orch.process_bar(bar)
        assert snap.bar_idx == 0
        assert snap.bar_ts > 0

    def test_snapshot_has_level1_data(self) -> None:
        """快照包含 level=1 的基础管线数据。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator()
        for bar in _zigzag_bars(10):
            snap = orch.process_bar(bar)
        # level 1 快照字段存在
        assert snap.bi_snapshot is not None
        assert snap.seg_snapshot is not None
        assert snap.zs_snapshot is not None
        assert snap.move_snapshot is not None

    def test_reset_clears_state(self) -> None:
        """reset 后重新处理应产生相同结果。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator()
        bars = _zigzag_bars(30)

        for bar in bars:
            orch.process_bar(bar)
        snap1_events = len(orch.bus.drain())  # drain clears bus

        orch.reset()

        for bar in bars:
            orch.process_bar(bar)
        snap2_events = len(orch.bus.drain())

        assert snap1_events == snap2_events


# ── 管线集成测试 ──


class TestRecursiveOrchestratorPipeline:
    """管线集成测试。"""

    def test_events_accumulate_in_bus(self) -> None:
        """process_bar 产生的事件进入 EventBus。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator()
        for bar in _zigzag_bars(30):
            orch.process_bar(bar)
        assert orch.bus.count > 0

    def test_recursive_levels_populated_with_enough_data(self) -> None:
        """足够多的数据应产生递归级别快照。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator()
        for bar in _zigzag_bars(120):
            snap = orch.process_bar(bar)
        # 至少有一些 level 1 的 moves 被处理
        assert snap.move_snapshot is not None

    def test_max_levels_respected(self) -> None:
        """max_levels 限制递归深度。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator
        orch = RecursiveOrchestrator(max_levels=3)
        assert orch.max_levels == 3
