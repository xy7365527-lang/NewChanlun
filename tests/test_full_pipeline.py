"""全流程回测引擎测试 — TDD RED phase。

认识论标注：L0（引擎框架从架构定义直接推导，测试用 mock 替代真实模块）。

测试策略：
  - 用 mock 替代 RecursiveOrchestrator / K4Scanner / StockScanner
  - 验证 FSM 驱动逻辑和 BarStep 记录完整性
  - 覆盖正常流程 + 止损路径 + 便捷入口
"""

from __future__ import annotations

from datetime import datetime, timedelta
from unittest.mock import MagicMock, patch

import pytest

from newchan.backtest.full_pipeline import (
    BarStep,
    FullPipelineConfig,
    FullPipelineEngine,
    FullPipelineResult,
    run_full_pipeline,
)
from newchan.trading.cost_reduction_fsm import CostReductionFSM, CostState
from newchan.types import Bar


# ── helpers ──────────────────────────────────────────────────


def _make_bar(ts_offset: int, price: float) -> Bar:
    """创建测试 Bar。"""
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(minutes=ts_offset),
        open=price,
        high=price + 1.0,
        low=price - 1.0,
        close=price,
        volume=1000.0,
    )


def _make_k4_bars(ts_offset: int, prices: tuple[float, float, float]) -> tuple[Bar, Bar, Bar]:
    """创建 K4 三条比价线的 bar。"""
    return (
        _make_bar(ts_offset, prices[0]),
        _make_bar(ts_offset, prices[1]),
        _make_bar(ts_offset, prices[2]),
    )


def _make_candidate_bars(ts_offset: int, symbols_prices: dict[str, float]) -> dict[str, Bar]:
    """创建候选标的 bars。"""
    return {sym: _make_bar(ts_offset, p) for sym, p in symbols_prices.items()}


# ── 1. FullPipelineConfig 默认值 ──────────────────────────────


class TestFullPipelineConfig:
    def test_default_values(self):
        cfg = FullPipelineConfig()
        assert cfg.initial_capital == 1_000_000.0
        assert cfg.margin_ratio == 1.0
        assert cfg.short_diff_ratio == 0.1
        assert cfg.min_operable_level == "5min"
        assert cfg.k4_symbols == ("SPY", "GLD", "TLT")

    def test_margin_amount(self):
        cfg = FullPipelineConfig(initial_capital=500_000.0, margin_ratio=2.0)
        assert cfg.margin_amount == 1_000_000.0

    def test_custom_k4_symbols(self):
        cfg = FullPipelineConfig(k4_symbols=("QQQ", "SLV", "IEF"))
        assert cfg.k4_symbols == ("QQQ", "SLV", "IEF")


# ── 2. FullPipelineEngine 初始状态 ────────────────────────────


class TestFullPipelineEngineInit:
    def test_initial_state_is_scanning(self):
        engine = FullPipelineEngine(FullPipelineConfig())
        result = engine.result()
        assert result.final_fsm.state == CostState.SCANNING
        assert result.total_bars == 0
        assert result.steps == []
        assert result.fsm_transitions == []

    def test_initial_fsm_params(self):
        cfg = FullPipelineConfig(
            initial_capital=800_000.0,
            margin_ratio=0.5,
            short_diff_ratio=0.2,
            min_operable_level="15min",
        )
        engine = FullPipelineEngine(cfg)
        fsm = engine.result().final_fsm
        assert fsm.own_capital == 800_000.0
        assert fsm.margin_amount == 400_000.0
        assert fsm.sub_ratio == 0.2
        assert fsm.min_operable_level == "15min"


# ── 3. K4 配置变化记录 ───────────────────────────────────────


class TestK4ConfigTracking:
    """Mock bar 流推进：K4 配置变化记录。

    场景：推送若干 bar，验证 BarStep 记录了 K4 配置。
    K4Scanner 和 StockScanner 返回空结果（SCANNING 持续）。
    """

    def test_k4_config_recorded_in_steps(self):
        engine = FullPipelineEngine(FullPipelineConfig())
        k4_bars = _make_k4_bars(0, (100.0, 50.0, 30.0))
        candidate_bars = _make_candidate_bars(0, {"AAPL": 150.0, "MSFT": 300.0})

        # 引擎在 SCANNING 阶段，K4Scanner mock 返回配置
        with patch.object(engine, "_scan_k4") as mock_k4, \
             patch.object(engine, "_scan_candidates") as mock_scan:
            mock_k4.return_value = ("(+,0,-)", 1)
            mock_scan.return_value = None  # 无选股命中
            step = engine.process_bar(k4_bars, candidate_bars)

        assert step.bar_idx == 0
        assert step.k4_config_label == "(+,0,-)"
        assert step.k4_polarity == 1
        assert step.scanner_selected is None
        assert step.fsm_state == CostState.SCANNING.name

    def test_multiple_bars_tracking(self):
        engine = FullPipelineEngine(FullPipelineConfig())
        configs = [("(+,+,+)", 3), ("(+,0,-)", 1), ("(-,-,-)", -3)]

        for i, (label, pol) in enumerate(configs):
            k4_bars = _make_k4_bars(i, (100.0 + i, 50.0, 30.0))
            candidate_bars = _make_candidate_bars(i, {"AAPL": 150.0})

            with patch.object(engine, "_scan_k4") as mock_k4, \
                 patch.object(engine, "_scan_candidates") as mock_scan:
                mock_k4.return_value = (label, pol)
                mock_scan.return_value = None
                engine.process_bar(k4_bars, candidate_bars)

        result = engine.result()
        assert result.total_bars == 3
        assert result.steps[0].k4_config_label == "(+,+,+)"
        assert result.steps[1].k4_polarity == 1
        assert result.steps[2].k4_config_label == "(-,-,-)"


# ── 4. 完整流程 mock ─────────────────────────────────────────


class TestFullFlowMock:
    """SCANNING → 选股命中 → POSITION_OPEN → 短差 → COST_REDUCING。"""

    def _run_flow(self) -> FullPipelineResult:
        cfg = FullPipelineConfig(
            initial_capital=100_000.0,
            margin_ratio=1.0,
            short_diff_ratio=0.3,
        )
        engine = FullPipelineEngine(cfg)

        # bar 0: SCANNING, 无选股命中
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan:
            mk4.return_value = ("(+,+,+)", 3)
            mscan.return_value = None
            engine.process_bar(
                _make_k4_bars(0, (100.0, 50.0, 30.0)),
                _make_candidate_bars(0, {"AAPL": 150.0}),
            )

        # bar 1: SCANNING → 选股命中 → POSITION_OPEN
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan, \
             patch.object(engine, "_detect_bsp_signal") as mbsp:
            mk4.return_value = ("(+,+,+)", 3)
            mscan.return_value = ("AAPL", 155.0, "5min")  # (symbol, price, level)
            mbsp.return_value = None
            engine.process_bar(
                _make_k4_bars(1, (101.0, 51.0, 31.0)),
                _make_candidate_bars(1, {"AAPL": 155.0}),
            )

        # bar 2: POSITION_OPEN → 次级别卖点 → COST_REDUCING
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_detect_bsp_signal") as mbsp:
            mk4.return_value = ("(+,+,0)", 2)
            mbsp.return_value = ("SUB_LEVEL_SELL_POINT", 160.0, "5min")
            engine.process_bar(
                _make_k4_bars(2, (102.0, 52.0, 30.0)),
                _make_candidate_bars(2, {"AAPL": 160.0}),
            )

        return engine.result()

    def test_scanning_to_position_open(self):
        result = self._run_flow()
        assert result.steps[0].fsm_state == CostState.SCANNING.name
        assert result.steps[1].fsm_state == CostState.POSITION_OPEN.name

    def test_position_open_to_cost_reducing(self):
        result = self._run_flow()
        assert result.steps[2].fsm_state == CostState.COST_REDUCING.name

    def test_fsm_transitions_recorded(self):
        result = self._run_flow()
        assert len(result.fsm_transitions) >= 2
        # 第一次转移：SCANNING → POSITION_OPEN
        assert result.fsm_transitions[0][1] == CostState.SCANNING.name
        assert result.fsm_transitions[0][2] == CostState.POSITION_OPEN.name

    def test_scanner_selected_recorded(self):
        result = self._run_flow()
        assert result.steps[1].scanner_selected == "AAPL"

    def test_total_shares_after_buy(self):
        result = self._run_flow()
        step1 = result.steps[1]
        assert step1.total_shares > 0

    def test_cost_basis_after_buy(self):
        result = self._run_flow()
        step1 = result.steps[1]
        assert step1.cost_basis == 155.0


# ── 5. 止损路径 ──────────────────────────────────────────────


class TestStopOutPath:
    """BUY_POINT_NEGATED → STOPPED_OUT → RESET → SCANNING。"""

    def test_stop_out_and_reset(self):
        cfg = FullPipelineConfig(initial_capital=100_000.0, margin_ratio=1.0)
        engine = FullPipelineEngine(cfg)

        # bar 0: 选股命中 → POSITION_OPEN
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan:
            mk4.return_value = ("(+,+,+)", 3)
            mscan.return_value = ("AAPL", 100.0, "5min")
            engine.process_bar(
                _make_k4_bars(0, (100.0, 50.0, 30.0)),
                _make_candidate_bars(0, {"AAPL": 100.0}),
            )

        # bar 1: 买点失效 → STOPPED_OUT
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_detect_bsp_signal") as mbsp:
            mk4.return_value = ("(-,-,-)", -3)
            mbsp.return_value = ("BUY_POINT_NEGATED", 80.0, "5min")
            engine.process_bar(
                _make_k4_bars(1, (90.0, 40.0, 25.0)),
                _make_candidate_bars(1, {"AAPL": 80.0}),
            )

        # bar 2: 自动 RESET → SCANNING
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan:
            mk4.return_value = ("(0,0,0)", 0)
            mscan.return_value = None
            engine.process_bar(
                _make_k4_bars(2, (95.0, 45.0, 28.0)),
                _make_candidate_bars(2, {"AAPL": 85.0}),
            )

        result = engine.result()
        assert result.steps[1].fsm_state == CostState.STOPPED_OUT.name
        assert result.steps[2].fsm_state == CostState.SCANNING.name


# ── 6. BarStep 记录完整性 ────────────────────────────────────


class TestBarStepCompleteness:
    def test_all_fields_present(self):
        engine = FullPipelineEngine(FullPipelineConfig())

        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan:
            mk4.return_value = ("(+,0,-)", 1)
            mscan.return_value = None
            step = engine.process_bar(
                _make_k4_bars(0, (100.0, 50.0, 30.0)),
                _make_candidate_bars(0, {"AAPL": 150.0}),
            )

        assert isinstance(step, BarStep)
        assert step.bar_idx == 0
        assert isinstance(step.bar_ts, float)
        assert isinstance(step.k4_config_label, str)
        assert isinstance(step.k4_polarity, int)
        assert step.scanner_selected is None
        assert isinstance(step.fsm_state, str)
        assert isinstance(step.cost_basis, float)
        assert isinstance(step.cumulative_recovered, float)
        assert isinstance(step.total_shares, float)


# ── 7. run_full_pipeline 便捷入口 ────────────────────────────


class TestRunFullPipeline:
    def test_runs_all_bars(self):
        cfg = FullPipelineConfig()
        n_bars = 5
        k4_streams = (
            [_make_bar(i, 100.0 + i) for i in range(n_bars)],
            [_make_bar(i, 50.0 + i) for i in range(n_bars)],
            [_make_bar(i, 30.0 + i) for i in range(n_bars)],
        )
        candidate_streams = {
            "AAPL": [_make_bar(i, 150.0 + i) for i in range(n_bars)],
        }

        result = run_full_pipeline(cfg, k4_streams, candidate_streams)

        assert isinstance(result, FullPipelineResult)
        assert result.total_bars == n_bars
        assert len(result.steps) == n_bars
        assert result.config is cfg

    def test_empty_streams(self):
        cfg = FullPipelineConfig()
        result = run_full_pipeline(cfg, ([], [], []), {})
        assert result.total_bars == 0
        assert result.steps == []

    def test_mismatched_stream_lengths(self):
        """如果 K4 流长度不一致，取最短长度。"""
        cfg = FullPipelineConfig()
        k4_streams = (
            [_make_bar(i, 100.0) for i in range(5)],
            [_make_bar(i, 50.0) for i in range(3)],
            [_make_bar(i, 30.0) for i in range(4)],
        )
        result = run_full_pipeline(cfg, k4_streams, {})
        assert result.total_bars == 3  # min(5, 3, 4)
