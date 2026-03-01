"""orchestrator.py 测试 — 全流程编排集成测试。

认识论标注：L1（合成数据，验证管线正确性）。
"""

from __future__ import annotations

from datetime import datetime, timedelta

from newchan.backtest.orchestrator import (
    BacktestOrchestrator,
    BacktestOrchestratorConfig,
    BacktestOrchestratorResult,
    run_backtest,
)
from newchan.backtest.types import StateMachineState
from newchan.types import Bar


def _make_bar(idx: int, price: float) -> Bar:
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(days=idx),
        open=price,
        high=price + 1.0,
        low=price - 1.0,
        close=price,
        volume=1000.0,
    )


class TestBacktestOrchestratorBasic:
    """编排器基础功能。"""

    def test_create_and_initial_state(self):
        """创建编排器，初始状态为 EMPTY。"""
        config = BacktestOrchestratorConfig()
        orch = BacktestOrchestrator(config)
        result = orch.result()
        assert len(result.steps) == 0
        assert result.actions == ()

    def test_process_single_bar(self):
        """处理单 bar，返回步进记录。"""
        config = BacktestOrchestratorConfig()
        orch = BacktestOrchestrator(config)

        k4_bars = (
            _make_bar(0, 400.0),
            _make_bar(0, 180.0),
            _make_bar(0, 90.0),
        )
        candidate_bars = {
            "XLK": _make_bar(0, 200.0),
            "XLF": _make_bar(0, 40.0),
        }

        step = orch.process_bar(k4_bars, candidate_bars)
        assert step.bar_idx == 0
        assert step.sm_state == StateMachineState.EMPTY
        assert step.held_symbol is None

    def test_multiple_bars_no_signal(self):
        """多 bar 无信号 → 持续空仓。"""
        config = BacktestOrchestratorConfig()
        orch = BacktestOrchestrator(config)

        for i in range(5):
            k4_bars = (
                _make_bar(i, 400.0 + i),
                _make_bar(i, 180.0 + i),
                _make_bar(i, 90.0 + i),
            )
            candidate_bars = {"XLK": _make_bar(i, 200.0 + i)}
            step = orch.process_bar(k4_bars, candidate_bars)

        result = orch.result()
        assert len(result.steps) == 5
        assert all(s.sm_state == StateMachineState.EMPTY for s in result.steps)

    def test_k4_changes_logged(self):
        """K4 配置变化被记录。"""
        config = BacktestOrchestratorConfig()
        orch = BacktestOrchestrator(config)

        # 第一个 bar → 初始配置
        k4_bars = (
            _make_bar(0, 400.0),
            _make_bar(0, 180.0),
            _make_bar(0, 90.0),
        )
        orch.process_bar(k4_bars, {})

        result = orch.result()
        # 第一个 bar 必然记录一次配置变化（初始化）
        assert len(result.k4_changes) >= 1


class TestRunBacktest:
    """run_backtest 便捷入口。"""

    def test_empty_bars(self):
        """空 bar 流 → 空结果。"""
        config = BacktestOrchestratorConfig()
        result = run_backtest(config, ([], [], []), {})
        assert len(result.steps) == 0

    def test_basic_run(self):
        """基本运行：3 bar K4 + 1 候选。"""
        config = BacktestOrchestratorConfig()
        n = 3
        k4_streams = (
            [_make_bar(i, 400.0 + i) for i in range(n)],
            [_make_bar(i, 180.0 + i) for i in range(n)],
            [_make_bar(i, 90.0 + i) for i in range(n)],
        )
        candidate_streams = {
            "XLK": [_make_bar(i, 200.0 + i) for i in range(n)],
        }
        result = run_backtest(config, k4_streams, candidate_streams)
        assert len(result.steps) == n
        assert result.config.initial_capital == 1_000_000.0

    def test_mismatched_lengths(self):
        """K4 流长度不一致 → 取最短长度。"""
        config = BacktestOrchestratorConfig()
        k4_streams = (
            [_make_bar(i, 400.0) for i in range(5)],
            [_make_bar(i, 180.0) for i in range(3)],
            [_make_bar(i, 90.0) for i in range(4)],
        )
        result = run_backtest(config, k4_streams, {})
        assert len(result.steps) == 3


class TestBacktestOrchestratorConfig:
    """配置。"""

    def test_default_values(self):
        config = BacktestOrchestratorConfig()
        assert config.initial_capital == 1_000_000.0
        assert config.margin_ratio == 1.0
        assert config.short_diff_ratio == 0.1
        assert config.k4_symbols == ("SPY", "GLD", "TLT")

    def test_max_capital(self):
        config = BacktestOrchestratorConfig(
            initial_capital=1_000_000.0,
            margin_ratio=1.0,
        )
        assert config.max_capital == 2_000_000.0

    def test_custom_candidates(self):
        config = BacktestOrchestratorConfig(
            candidate_symbols=("XLK", "XLF"),
        )
        assert len(config.candidate_symbols) == 2
