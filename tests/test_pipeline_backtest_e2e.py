"""PipelineBacktestEngine 端到端测试。

用 RecursiveOrchestrator 处理合成 bar 序列，
产出真实 RecursiveOrchestratorSnapshot，
驱动 PipelineBacktestEngine 执行完整交易管道。
"""

from __future__ import annotations

import math
from datetime import datetime, timedelta

from newchan.backtest import BacktestConfig, BacktestEngine
from newchan.pipeline_backtest import PipelineBacktestConfig, PipelineBacktestEngine
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.config_space import FULL_RISK_ON
from newchan.types import Bar


# ── 合成 bar 生成工具 ──


def _bar(idx: int, close: float, amplitude: float = 2.0) -> Bar:
    """生成单根 bar，open=close，high/low 按 amplitude 展开。"""
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(minutes=idx),
        open=close,
        high=close + amplitude,
        low=close - amplitude,
        close=close,
    )


def _generate_trend_bars(
    start_idx: int,
    start_price: float,
    end_price: float,
    n_bars: int,
    amplitude: float = 2.0,
) -> list[Bar]:
    """生成线性趋势 bar 序列。

    从 start_price 线性过渡到 end_price，共 n_bars 根。
    """
    bars: list[Bar] = []
    for i in range(n_bars):
        t = i / max(n_bars - 1, 1)
        price = start_price + (end_price - start_price) * t
        bars.append(_bar(start_idx + i, price, amplitude))
    return bars


def _generate_zigzag_bars(
    start_idx: int,
    base_price: float,
    amplitude: float,
    n_waves: int,
    bars_per_wave: int,
) -> list[Bar]:
    """生成锯齿形（上下交替）bar 序列。

    每个 wave 由线性上涨+线性下跌组成。
    用于构造笔/线段/中枢。
    """
    bars: list[Bar] = []
    idx = start_idx
    price = base_price
    for w in range(n_waves):
        # 上涨半波
        peak = base_price + amplitude * (1 + 0.1 * w)
        up_bars = _generate_trend_bars(idx, price, peak, bars_per_wave)
        bars.extend(up_bars)
        idx += bars_per_wave
        price = peak

        # 下跌半波
        valley = base_price - amplitude * (1 + 0.05 * w)
        down_bars = _generate_trend_bars(idx, price, valley, bars_per_wave)
        bars.extend(down_bars)
        idx += bars_per_wave
        price = valley

    return bars


def _generate_uptrend_reversal_bars(
    n_up: int = 150,
    n_down: int = 150,
    start_price: float = 100.0,
    peak_price: float = 200.0,
    end_price: float = 120.0,
    wave_amplitude: float = 8.0,
    bars_per_wave: int = 7,
) -> list[Bar]:
    """生成先涨后跌的合成 bar 序列。

    上涨段和下跌段各自由锯齿波构成（触发笔/线段/中枢构造）。
    """
    bars: list[Bar] = []
    idx = 0
    price = start_price

    # 上涨段：整体从 start_price -> peak_price，叠加锯齿
    n_up_waves = n_up // (2 * bars_per_wave)
    if n_up_waves < 2:
        n_up_waves = 2
    price_step = (peak_price - start_price) / n_up_waves
    for w in range(n_up_waves):
        base = start_price + w * price_step
        up_peak = base + price_step + wave_amplitude
        up_valley = base + price_step - wave_amplitude * 0.5

        up_bars = _generate_trend_bars(idx, price, up_peak, bars_per_wave, amplitude=3.0)
        bars.extend(up_bars)
        idx += bars_per_wave
        price = up_peak

        down_bars = _generate_trend_bars(idx, price, up_valley, bars_per_wave, amplitude=3.0)
        bars.extend(down_bars)
        idx += bars_per_wave
        price = up_valley

    # 下跌段：整体从 peak 附近 -> end_price，叠加锯齿
    n_down_waves = n_down // (2 * bars_per_wave)
    if n_down_waves < 2:
        n_down_waves = 2
    current_peak = price
    price_step_down = (current_peak - end_price) / n_down_waves
    for w in range(n_down_waves):
        base = current_peak - w * price_step_down
        down_valley = base - price_step_down - wave_amplitude
        down_recover = base - price_step_down + wave_amplitude * 0.5

        down_bars = _generate_trend_bars(idx, price, down_valley, bars_per_wave, amplitude=3.0)
        bars.extend(down_bars)
        idx += bars_per_wave
        price = down_valley

        up_bars = _generate_trend_bars(idx, price, down_recover, bars_per_wave, amplitude=3.0)
        bars.extend(up_bars)
        idx += bars_per_wave
        price = down_recover

    return bars


def _generate_flat_bars(
    n_bars: int = 200,
    base_price: float = 100.0,
    noise: float = 0.5,
) -> list[Bar]:
    """生成平坦（几乎无趋势）的 bar 序列。

    价格在 base_price 附近微幅波动。
    """
    bars: list[Bar] = []
    for i in range(n_bars):
        offset = noise * math.sin(i * 0.3)
        bars.append(_bar(i, base_price + offset, amplitude=1.0))
    return bars


def _generate_multi_reversal_bars(
    n_segments: int = 4,
    bars_per_segment: int = 100,
    start_price: float = 100.0,
    swing_range: float = 50.0,
    wave_amplitude: float = 6.0,
    bars_per_wave: int = 7,
) -> list[Bar]:
    """生成多段趋势反转 bar 序列。

    交替上涨/下跌，共 n_segments 段。
    """
    bars: list[Bar] = []
    idx = 0
    price = start_price

    for seg in range(n_segments):
        n_waves = bars_per_segment // (2 * bars_per_wave)
        if n_waves < 2:
            n_waves = 2

        if seg % 2 == 0:
            # 上涨段
            target = price + swing_range
            step = swing_range / n_waves
            for w in range(n_waves):
                peak = price + step + wave_amplitude
                valley = price + step - wave_amplitude * 0.5
                up = _generate_trend_bars(idx, price, peak, bars_per_wave, amplitude=2.5)
                bars.extend(up)
                idx += bars_per_wave
                price = peak
                dn = _generate_trend_bars(idx, price, valley, bars_per_wave, amplitude=2.5)
                bars.extend(dn)
                idx += bars_per_wave
                price = valley
        else:
            # 下跌段
            target = price - swing_range
            step = swing_range / n_waves
            for w in range(n_waves):
                valley = price - step - wave_amplitude
                recover = price - step + wave_amplitude * 0.5
                dn = _generate_trend_bars(idx, price, valley, bars_per_wave, amplitude=2.5)
                bars.extend(dn)
                idx += bars_per_wave
                price = valley
                up = _generate_trend_bars(idx, price, recover, bars_per_wave, amplitude=2.5)
                bars.extend(up)
                idx += bars_per_wave
                price = recover

    return bars


# ── 测试类 ──


class TestBasicE2E:
    """基础端到端：RecursiveOrchestrator -> PipelineBacktestEngine。"""

    def test_uptrend_reversal_completes(self):
        """先涨后跌的合成序列走完全流程，engine 不报错。"""
        bars = _generate_uptrend_reversal_bars()
        assert len(bars) >= 200

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        assert result.total_bars == len(bars)
        # 无论是否产生交易，result 结构正确
        assert result.trade_count >= 0
        assert result.win_rate >= 0.0
        assert result.max_drawdown_pct >= 0.0

    def test_uptrend_reversal_snapshot_has_bsp_snapshot(self):
        """验证 RecursiveOrchestratorSnapshot 有 bsp_snapshot 属性。"""
        bars = _generate_uptrend_reversal_bars()
        orch = RecursiveOrchestrator()

        snapshot = orch.process_bar(bars[0])
        assert hasattr(snapshot, "bsp_snapshot")
        assert hasattr(snapshot.bsp_snapshot, "buysellpoints")

    def test_result_trades_are_valid(self):
        """如果产生了交易，验证交易记录字段合法。"""
        bars = _generate_uptrend_reversal_bars(
            n_up=200, n_down=200,
            start_price=50.0, peak_price=200.0, end_price=80.0,
            wave_amplitude=10.0, bars_per_wave=8,
        )

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        for trade in result.trades:
            assert trade.entry_price > 0
            assert trade.exit_price > 0
            assert trade.entry_bar >= 0
            assert trade.exit_bar > trade.entry_bar
            assert trade.exit_reason in ("reverse_bsp", "bsp_negated")
            assert trade.side in ("long", "short")


class TestNoTradeScenario:
    """平坦序列——无 BSP 产生时的行为。"""

    def test_flat_bars_no_crash(self):
        """平坦 bar 序列走完全流程，不崩溃。"""
        bars = _generate_flat_bars(n_bars=200)

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        assert result.total_bars == 200
        # 平坦序列产生 BSP 的可能性极低
        # 但不做强断言（可能因噪声偶然触发）

    def test_flat_bars_result_valid(self):
        """平坦序列的统计指标正确。"""
        bars = _generate_flat_bars(n_bars=300, noise=0.1)

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        assert result.total_bars == 300
        assert result.max_drawdown_pct >= 0.0
        if result.trade_count == 0:
            assert result.win_rate == 0.0
            assert result.profit_loss_ratio == 0.0


class TestMultiTradeScenario:
    """多段趋势反转——多次入场/退场的累积结果。"""

    def test_multi_reversal_completes(self):
        """多段反转序列走完全流程。"""
        bars = _generate_multi_reversal_bars(
            n_segments=6,
            bars_per_segment=120,
            swing_range=60.0,
            wave_amplitude=8.0,
            bars_per_wave=8,
        )

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        assert result.total_bars == len(bars)
        # 多段反转应有更高概率产生交易
        assert result.trade_count >= 0

    def test_multi_reversal_cumulative_result(self):
        """多段反转后统计指标一致性。"""
        bars = _generate_multi_reversal_bars(
            n_segments=8,
            bars_per_segment=150,
            swing_range=80.0,
            wave_amplitude=10.0,
            bars_per_wave=9,
        )

        orch = RecursiveOrchestrator()
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for bar in bars:
            snapshot = orch.process_bar(bar)
            engine.process_snapshot(snapshot, bar)

        result = engine.result()
        # 统计一致性
        assert result.win_count + result.loss_count == result.trade_count
        if result.trade_count > 0:
            assert 0.0 <= result.win_rate <= 1.0
            assert result.max_drawdown_pct >= 0.0


class TestBacktestEngineComparison:
    """BacktestEngine（信号级）vs PipelineBacktestEngine（策略级）对比。

    相同数据输入，策略级交易数应 <= 信号级交易数。
    策略级经过配置扫描+共振过滤，只有部分信号能通过 pipeline。
    """

    def test_pipeline_trades_leq_signal_trades(self):
        """策略级交易数 <= 信号级交易数。"""
        bars = _generate_uptrend_reversal_bars(
            n_up=200, n_down=200,
            start_price=50.0, peak_price=250.0, end_price=70.0,
            wave_amplitude=12.0, bars_per_wave=9,
        )

        orch_signal = RecursiveOrchestrator()
        signal_engine = BacktestEngine()

        orch_pipeline = RecursiveOrchestrator()
        pipeline_config = PipelineBacktestConfig(config=FULL_RISK_ON)
        pipeline_engine = PipelineBacktestEngine(pipeline_config)

        for bar in bars:
            snap_s = orch_signal.process_bar(bar)
            signal_engine.process_snapshot(snap_s, bar)

            snap_p = orch_pipeline.process_bar(bar)
            pipeline_engine.process_snapshot(snap_p, bar)

        signal_result = signal_engine.result()
        pipeline_result = pipeline_engine.result()

        # 策略级经过配置扫描 + 共振过滤，交易数 <= 信号级
        assert pipeline_result.trade_count <= signal_result.trade_count

    def test_both_engines_same_bar_count(self):
        """两个引擎处理相同数量的 bar。"""
        bars = _generate_multi_reversal_bars(
            n_segments=4,
            bars_per_segment=100,
        )

        orch_signal = RecursiveOrchestrator()
        signal_engine = BacktestEngine()

        orch_pipeline = RecursiveOrchestrator()
        pipeline_config = PipelineBacktestConfig(config=FULL_RISK_ON)
        pipeline_engine = PipelineBacktestEngine(pipeline_config)

        for bar in bars:
            snap_s = orch_signal.process_bar(bar)
            signal_engine.process_snapshot(snap_s, bar)

            snap_p = orch_pipeline.process_bar(bar)
            pipeline_engine.process_snapshot(snap_p, bar)

        assert signal_engine.result().total_bars == pipeline_engine.result().total_bars
        assert signal_engine.result().total_bars == len(bars)

    def test_comparison_multi_reversal(self):
        """多段反转下对比。"""
        bars = _generate_multi_reversal_bars(
            n_segments=6,
            bars_per_segment=120,
            swing_range=60.0,
            wave_amplitude=8.0,
            bars_per_wave=8,
        )

        orch_signal = RecursiveOrchestrator()
        signal_engine = BacktestEngine()

        orch_pipeline = RecursiveOrchestrator()
        pipeline_config = PipelineBacktestConfig(config=FULL_RISK_ON)
        pipeline_engine = PipelineBacktestEngine(pipeline_config)

        for bar in bars:
            snap_s = orch_signal.process_bar(bar)
            signal_engine.process_snapshot(snap_s, bar)

            snap_p = orch_pipeline.process_bar(bar)
            pipeline_engine.process_snapshot(snap_p, bar)

        signal_result = signal_engine.result()
        pipeline_result = pipeline_engine.result()

        assert pipeline_result.trade_count <= signal_result.trade_count
        # 两个引擎的交易记录都应有效
        for trade in pipeline_result.trades:
            assert trade.entry_price > 0
            assert trade.exit_price > 0
        for trade in signal_result.trades:
            assert trade.entry_price > 0
            assert trade.exit_price > 0
