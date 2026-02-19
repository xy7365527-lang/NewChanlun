"""区间套管线端到端测试。

验证 run_nested_search 的管线贯通性：
  - bars → RecursiveOrchestrator → nested_divergence_search
  - 本地 MACD vs 外部 df_macd 两条路径
  - 空输入/不足输入的边界处理
"""

from __future__ import annotations

from datetime import datetime, timedelta

import pandas as pd
import pytest

from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar


def _make_bars(n: int, base_price: float = 100.0) -> list[Bar]:
    """生成 n 根模拟 K 线（带简单波动）。"""
    bars: list[Bar] = []
    t0 = datetime(2025, 1, 1, 9, 30)
    for i in range(n):
        # 简单锯齿波动
        offset = (i % 7 - 3) * 0.5
        c = base_price + offset + i * 0.01
        bars.append(Bar(
            ts=t0 + timedelta(minutes=i),
            open=c - 0.1,
            high=c + 0.5,
            low=c - 0.5,
            close=c,
            volume=1000.0,
        ))
    return bars


class TestRunNestedSearch:
    def test_empty_bars(self):
        """空 bars → ([], None)。"""
        results, snap = run_nested_search([])
        assert results == []
        assert snap is None

    def test_insufficient_bars(self):
        """不足 3 根 → ([], None)。"""
        bars = _make_bars(2)
        results, snap = run_nested_search(bars)
        assert results == []
        assert snap is None

    def test_returns_snap_with_enough_bars(self):
        """足够 bars → snap 非 None。"""
        bars = _make_bars(100)
        results, snap = run_nested_search(bars)
        assert snap is not None
        assert snap.bar_idx >= 0

    def test_local_macd_computed(self):
        """df_macd=None 时本地计算 MACD，不报错。"""
        bars = _make_bars(50)
        results, snap = run_nested_search(bars, df_macd=None)
        assert snap is not None

    def test_external_macd_accepted(self):
        """传入外部 df_macd 被正确使用。"""
        bars = _make_bars(50)
        # 构造与 bars 等长的假 MACD 数据
        df_macd = pd.DataFrame({
            "macd": [0.1] * 50,
            "signal": [0.05] * 50,
            "hist": [0.05] * 50,
        })
        results, snap = run_nested_search(bars, df_macd=df_macd)
        assert snap is not None

    def test_results_are_list(self):
        """返回值类型正确。"""
        bars = _make_bars(100)
        results, snap = run_nested_search(bars)
        assert isinstance(results, list)

    def test_snap_has_recursive_snapshots(self):
        """管线驱动后快照包含递归层信息。"""
        # 200 根 bars 可能产生递归层（取决于数据结构）
        bars = _make_bars(200)
        results, snap = run_nested_search(bars, max_levels=3)
        assert snap is not None
        # recursive_snapshots 是列表（可能为空，取决于数据复杂度）
        assert isinstance(snap.recursive_snapshots, list)
