"""回测分析模块测试 — 267号三个下游推论的度量。

认识论标注：L1（管线验证，合成数据）。
谱系引用：267号操作方法论 v1 下游推论1-3。
"""

from __future__ import annotations

from datetime import datetime, timedelta
from unittest.mock import patch

import pytest

from newchan.backtest.analysis import (
    CostReductionSpeed,
    CostReductionTimeSeries,
    OpportunityCostAnalysis,
    compute_cost_reduction_speed,
    compute_opportunity_cost,
    extract_cost_reduction_timeseries,
)
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
    return (
        _make_bar(ts_offset, prices[0]),
        _make_bar(ts_offset, prices[1]),
        _make_bar(ts_offset, prices[2]),
    )


def _make_candidate_bars(ts_offset: int, symbols_prices: dict[str, float]) -> dict[str, Bar]:
    return {sym: _make_bar(ts_offset, p) for sym, p in symbols_prices.items()}


def _build_mock_result(
    *,
    steps: list[BarStep],
    transitions: list[tuple[int, str, str]] | None = None,
    final_fsm: CostReductionFSM | None = None,
    initial_capital: float = 100_000.0,
) -> FullPipelineResult:
    """构建 mock 回测结果。"""
    cfg = FullPipelineConfig(initial_capital=initial_capital)
    if final_fsm is None:
        final_fsm = CostReductionFSM.create(own_capital=initial_capital)
    return FullPipelineResult(
        config=cfg,
        steps=steps,
        fsm_transitions=transitions or [],
        final_fsm=final_fsm,
        total_bars=len(steps),
    )


# ═══════════════════════════════════════════════════════════════
# 1. CostReductionSpeed 测试
# ═══════════════════════════════════════════════════════════════


class TestCostReductionSpeed:
    """降成本速度度量（267号下游推论1）。"""

    def test_zero_bars_in_position(self) -> None:
        """全部在 SCANNING 状态，无持仓。"""
        steps = [
            BarStep(
                bar_idx=i, bar_ts=float(i), k4_config_label="(+,+,+)",
                k4_polarity=3, scanner_selected=None,
                fsm_state="SCANNING", cost_basis=0.0,
                cumulative_recovered=0.0, total_shares=0.0,
            )
            for i in range(5)
        ]
        result = _build_mock_result(steps=steps)
        speed = compute_cost_reduction_speed(result)
        assert speed.bars_in_position == 0
        assert speed.recovery_per_bar == 0.0
        assert speed.recovery_ratio == 0.0

    def test_with_position_bars(self) -> None:
        """有持仓 bar，验证 bars_in_position 计数。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="SCANNING", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0),
            BarStep(bar_idx=1, bar_ts=1.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="AAPL",
                    fsm_state="POSITION_OPEN", cost_basis=100.0,
                    cumulative_recovered=0.0, total_shares=200.0),
            BarStep(bar_idx=2, bar_ts=2.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=95.0,
                    cumulative_recovered=500.0, total_shares=200.0),
            BarStep(bar_idx=3, bar_ts=3.0, k4_config_label="(+,0,+)",
                    k4_polarity=2, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=90.0,
                    cumulative_recovered=1200.0, total_shares=200.0),
        ]
        fsm = CostReductionFSM.create(own_capital=100_000.0)
        # 手动设置 cumulative_recovered 通过 mock 结果
        result = _build_mock_result(
            steps=steps,
            final_fsm=CostReductionFSM(
                state=CostState.COST_REDUCING,
                own_capital=100_000.0,
                margin_amount=100_000.0,
                total_shares=200.0,
                cost_basis=90.0,
                cumulative_recovered=1200.0,
                entry_price=100.0,
                entry_level="30min",
                sub_ratio=0.3,
                min_operable_level="5min",
                active_short_diff=None,
                completed_short_diffs=(),
                fugue_voices=(),
            ),
        )
        speed = compute_cost_reduction_speed(result)
        assert speed.bars_in_position == 3  # POSITION_OPEN + 2x COST_REDUCING
        assert speed.total_recovered == 1200.0
        assert speed.recovery_per_bar == pytest.approx(400.0)
        assert speed.recovery_ratio == pytest.approx(0.012)

    def test_recovery_ratio(self) -> None:
        """回收率 = 累计回收 / 自有资金。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="PRINCIPAL_WITHDRAWN", cost_basis=0.0,
                    cumulative_recovered=100_000.0, total_shares=200.0),
        ]
        result = _build_mock_result(
            steps=steps,
            initial_capital=100_000.0,
            final_fsm=CostReductionFSM(
                state=CostState.PRINCIPAL_WITHDRAWN,
                own_capital=100_000.0,
                margin_amount=100_000.0,
                total_shares=200.0,
                cost_basis=0.0,
                cumulative_recovered=100_000.0,
                entry_price=100.0,
                entry_level="30min",
                sub_ratio=0.3,
                min_operable_level="",
                active_short_diff=None,
                completed_short_diffs=(),
                fugue_voices=(),
            ),
        )
        speed = compute_cost_reduction_speed(result)
        assert speed.recovery_ratio == pytest.approx(1.0)


# ═══════════════════════════════════════════════════════════════
# 2. OpportunityCostAnalysis 测试
# ═══════════════════════════════════════════════════════════════


class TestOpportunityCost:
    """不换仓机会成本分析（267号下游推论3）。"""

    def test_no_missed_signals(self) -> None:
        """无持仓状态时 missed = 0。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="SCANNING", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0,
                    missed_signals_count=0),
        ]
        result = _build_mock_result(steps=steps)
        analysis = compute_opportunity_cost(result)
        assert analysis.missed_buy_count == 0
        assert analysis.held_bars == 0

    def test_missed_signals_during_position(self) -> None:
        """持仓期间有错过的信号。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="AAPL",
                    fsm_state="POSITION_OPEN", cost_basis=100.0,
                    cumulative_recovered=0.0, total_shares=200.0,
                    missed_signals_count=2),
            BarStep(bar_idx=1, bar_ts=1.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=95.0,
                    cumulative_recovered=500.0, total_shares=200.0,
                    missed_signals_count=1),
        ]
        transitions = [(0, "SCANNING", "POSITION_OPEN")]
        result = _build_mock_result(steps=steps, transitions=transitions)
        analysis = compute_opportunity_cost(result)
        assert analysis.missed_buy_count == 3  # 2 + 1
        assert analysis.held_bars == 2
        assert analysis.held_symbol == "AAPL"


# ═══════════════════════════════════════════════════════════════
# 3. CostReductionTimeSeries 测试
# ═══════════════════════════════════════════════════════════════


class TestCostReductionTimeSeries:
    """降成本时间序列提取。"""

    def test_empty_result(self) -> None:
        result = _build_mock_result(steps=[])
        ts = extract_cost_reduction_timeseries(result)
        assert ts.bar_indices == ()
        assert ts.cost_basis_series == ()

    def test_filters_position_states_only(self) -> None:
        """仅提取持仓状态的数据。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="SCANNING", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0),
            BarStep(bar_idx=1, bar_ts=1.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="AAPL",
                    fsm_state="POSITION_OPEN", cost_basis=100.0,
                    cumulative_recovered=0.0, total_shares=200.0),
            BarStep(bar_idx=2, bar_ts=2.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=95.0,
                    cumulative_recovered=500.0, total_shares=200.0),
            BarStep(bar_idx=3, bar_ts=3.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="STOPPED_OUT", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0),
        ]
        result = _build_mock_result(steps=steps, initial_capital=100_000.0)
        ts = extract_cost_reduction_timeseries(result)
        assert ts.bar_indices == (1, 2)
        assert ts.cost_basis_series == (100.0, 95.0)
        assert ts.cumulative_recovered_series == (0.0, 500.0)
        assert ts.recovery_rate_series == pytest.approx((0.0, 0.005))

    def test_recovery_rate_calculation(self) -> None:
        """回收率时间序列 = 累计回收 / 自有资金。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="AAPL",
                    fsm_state="COST_REDUCING", cost_basis=90.0,
                    cumulative_recovered=50_000.0, total_shares=200.0),
        ]
        result = _build_mock_result(steps=steps, initial_capital=100_000.0)
        ts = extract_cost_reduction_timeseries(result)
        assert ts.recovery_rate_series == pytest.approx((0.5,))


# ═══════════════════════════════════════════════════════════════
# 4. BarStep.missed_signals_count 字段测试
# ═══════════════════════════════════════════════════════════════


class TestBarStepMissedSignals:
    """BarStep 中 missed_signals_count 字段的默认值和可设置性。"""

    def test_default_is_zero(self) -> None:
        step = BarStep(
            bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
            k4_polarity=3, scanner_selected=None,
            fsm_state="SCANNING", cost_basis=0.0,
            cumulative_recovered=0.0, total_shares=0.0,
        )
        assert step.missed_signals_count == 0

    def test_can_be_set(self) -> None:
        step = BarStep(
            bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
            k4_polarity=3, scanner_selected=None,
            fsm_state="POSITION_OPEN", cost_basis=100.0,
            cumulative_recovered=0.0, total_shares=200.0,
            missed_signals_count=3,
        )
        assert step.missed_signals_count == 3


# ═══════════════════════════════════════════════════════════════
# 5. 引擎集成测试：机会成本跟踪
# ═══════════════════════════════════════════════════════════════


class TestEngineOpportunityCostTracking:
    """引擎在持仓期间跟踪其他标的信号。"""

    def test_missed_signals_recorded_in_steps(self) -> None:
        """持仓时引擎跟踪其他标的的买点信号。"""
        engine = FullPipelineEngine(FullPipelineConfig(
            initial_capital=100_000.0,
            margin_ratio=1.0,
        ))

        # bar 0: SCANNING → 选股命中
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_scan_candidates") as mscan:
            mk4.return_value = ("(+,+,+)", 3)
            mscan.return_value = ("AAPL", 100.0, "5min")
            engine.process_bar(
                _make_k4_bars(0, (100.0, 50.0, 30.0)),
                _make_candidate_bars(0, {"AAPL": 100.0, "MSFT": 200.0}),
            )

        # bar 1: POSITION_OPEN, 引擎应在跟踪 MSFT 的信号
        with patch.object(engine, "_scan_k4") as mk4, \
             patch.object(engine, "_detect_bsp_signal") as mbsp, \
             patch.object(engine, "_track_missed_signals") as mtrack:
            mk4.return_value = ("(+,+,+)", 3)
            mbsp.return_value = None
            mtrack.return_value = None
            step = engine.process_bar(
                _make_k4_bars(1, (101.0, 51.0, 31.0)),
                _make_candidate_bars(1, {"AAPL": 105.0, "MSFT": 210.0}),
            )

        assert step.fsm_state == "POSITION_OPEN"
        # _track_missed_signals 被调用
        mtrack.assert_called_once()


# ═══════════════════════════════════════════════════════════════
# 6. 现有功能回归测试
# ═══════════════════════════════════════════════════════════════


class TestBackwardCompatibility:
    """确保新增字段不破坏现有功能。"""

    def test_run_full_pipeline_still_works(self) -> None:
        """run_full_pipeline 便捷入口不受影响。"""
        cfg = FullPipelineConfig()
        n_bars = 3
        k4_streams = (
            [_make_bar(i, 100.0 + i) for i in range(n_bars)],
            [_make_bar(i, 50.0 + i) for i in range(n_bars)],
            [_make_bar(i, 30.0 + i) for i in range(n_bars)],
        )
        candidate_streams = {
            "AAPL": [_make_bar(i, 150.0 + i) for i in range(n_bars)],
        }
        result = run_full_pipeline(cfg, k4_streams, candidate_streams)
        assert result.total_bars == n_bars
        # 每个 step 都有 missed_signals_count 字段
        for step in result.steps:
            assert hasattr(step, "missed_signals_count")
            assert step.missed_signals_count >= 0
