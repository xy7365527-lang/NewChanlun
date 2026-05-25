"""策略级回测引擎测试 — pipeline + backtest 集成。

覆盖：
- 基础流程：pipeline 驱动的入场/退场
- 配置影响：不同 Configuration 下的方向过滤
- 共振信号：有/无共振时的入场差异
- 否定退出：BSP 否定后的止损退出
- 结果一致性：BacktestResult 字段完整性
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.backtest import BacktestResult, Trade
from newchan.nesting.bsp import BSPType
from newchan.pipeline_backtest import PipelineBacktestConfig, PipelineBacktestEngine
from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    WalkDirection,
)
from newchan.trading.layer_state import LayerState, LayerType
from newchan.types import Bar


# ── Stub 类型（与 test_backtest.py 保持一致） ──


@dataclass
class _BspSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    buysellpoints: list[BuySellPoint] = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _FakeSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    bsp_snapshot: _BspSnapshot = field(default_factory=_BspSnapshot)


def _bar(idx: int, close: float) -> Bar:
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(minutes=idx),
        open=close,
        high=close + 1,
        low=close - 1,
        close=close,
    )


def _buy_bsp(
    seg_idx: int = 0,
    kind: str = "type1",
    level_id: int = 1,
    price: float = 100.0,
    bar_idx: int = 0,
    confirmed: bool = True,
) -> BuySellPoint:
    return BuySellPoint(
        kind=kind,
        side="buy",
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=0,
        divergence_key=(0, 0, seg_idx),
        center_zd=90.0,
        center_zg=110.0,
        center_seg_start=0,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=False,
    )


def _sell_bsp(
    seg_idx: int = 1,
    kind: str = "type1",
    level_id: int = 1,
    price: float = 120.0,
    bar_idx: int = 5,
    confirmed: bool = True,
) -> BuySellPoint:
    return BuySellPoint(
        kind=kind,
        side="sell",
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=0,
        divergence_key=(0, 0, seg_idx),
        center_zd=90.0,
        center_zg=110.0,
        center_seg_start=0,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=False,
    )


# ── 基础流程 ──


class TestBasicPipelineFlow:
    """pipeline 驱动的入场/退场完整流程。"""

    def test_buy_then_sell_via_pipeline(self):
        """FULL_RISK_ON 配置 + 买点 -> pipeline 入场 -> 卖点 -> 退场。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        # bar 0: 买入信号
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # bar 1-4: 无新信号
        for i in range(1, 5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[
                    _buy_bsp(seg_idx=0, bar_idx=0),
                ]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i * 5))

        # bar 5: 卖出信号 -> 平仓
        snap5 = _FakeSnapshot(
            bar_idx=5,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=5),
            ]),
        )
        engine.process_snapshot(snap5, _bar(5, 130.0))

        result = engine.result()
        assert result.trade_count == 1
        t = result.trades[0]
        assert t.side == "long"
        assert t.entry_price == 100.0
        assert t.exit_price == 130.0
        assert t.pnl == 30.0
        assert t.hold_bars == 5
        assert t.exit_reason == "reverse_bsp"

    def test_no_signal_no_trade(self):
        """无 BSP 信号时不产生交易。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        for i in range(10):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i))

        result = engine.result()
        assert result.trade_count == 0
        assert result.total_bars == 10

    def test_unconfirmed_bsp_ignored(self):
        """未确认的 BSP 不触发入场。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, confirmed=False),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position

    def test_entry_price_is_bar_close(self):
        """入场价 = bar.close，不是 BSP.price。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, price=95.0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.has_open_position

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, price=95.0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 110.0))

        result = engine.result()
        assert result.trades[0].entry_price == 100.0
        assert result.trades[0].exit_price == 110.0


# ── 配置影响 ──


class TestConfigurationImpact:
    """不同 Configuration 下的入场行为差异。"""

    def test_risk_on_allows_buy(self):
        """FULL_RISK_ON (S=3) 允许买入。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.has_open_position

    def test_center_blocks_entry(self):
        """CENTER (S=0) neutral -> 不入场。"""
        config = PipelineBacktestConfig(config=CENTER)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position

    def test_risk_off_blocks_buy(self):
        """FULL_RISK_OFF (S=-3) sell direction -> 买点不触发入场。"""
        config = PipelineBacktestConfig(config=FULL_RISK_OFF)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position

    def test_risk_off_allows_short_when_enabled(self):
        """FULL_RISK_OFF + allow_short=True -> 卖点触发做空入场。"""
        config = PipelineBacktestConfig(
            config=FULL_RISK_OFF,
            allow_short=True,
        )
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _sell_bsp(seg_idx=0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.has_open_position

    def test_short_disabled_by_default(self):
        """默认 allow_short=False -> sell BSP 不开仓。"""
        config = PipelineBacktestConfig(config=FULL_RISK_OFF)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _sell_bsp(seg_idx=0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position


# ── 否定退出 ──


class TestNegationExit:
    """BSP 否定后的退出行为。"""

    def test_buy_negated_triggers_exit(self):
        """买点被否定（价格跌破买点价）-> bsp_negated 退出。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        # bar 0: 买点开仓，price=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0, price=100.0),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # bar 1: 价格 101 > 100 -> 不否定
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0, price=100.0),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 101.0))
        assert engine.has_open_position

        # bar 2: 价格 90 < 100 -> 否定 -> 退出
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0, price=100.0),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 90.0))
        assert not engine.has_open_position

        result = engine.result()
        assert result.trade_count == 1
        assert result.trades[0].exit_reason == "bsp_negated"
        assert result.trades[0].exit_price == 90.0

    def test_negation_loss_recorded(self):
        """否定退出产生亏损交易。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0, price=100.0),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0, price=100.0),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 80.0))

        result = engine.result()
        assert result.trades[0].pnl < 0


# ── 状态机跟踪 ──


class TestStateMachineTracking:
    """pipeline 入场后状态机状态正确。"""

    def test_l0_active_after_entry(self):
        """入场后 L0 状态为 ACTIVE。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.ACTIVE

    def test_l0_inactive_after_exit(self):
        """退出后 L0 状态回到 INACTIVE。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 120.0))
        assert engine.ctx.state_machine.get_state(LayerType.L0_CONFIG) is LayerState.INACTIVE


# ── 结果一致性 ──


class TestResultConsistency:
    """验证 BacktestResult 字段完整性。"""

    def test_empty_result(self):
        """无交易时 result 正确返回。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)
        result = engine.result()
        assert isinstance(result, BacktestResult)
        assert result.trade_count == 0
        assert result.total_bars == 0

    def test_result_type(self):
        """result() 返回 BacktestResult 类型。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 120.0))

        result = engine.result()
        assert isinstance(result, BacktestResult)
        assert result.trade_count == 1
        assert isinstance(result.trades[0], Trade)
        assert result.total_bars == 2

    def test_statistics_computed(self):
        """两笔交易后统计指标可正确计算。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        # 第一笔：买100 卖120 -> +20
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 120.0))

        # 第二笔：买110 卖100 -> -10
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
                _buy_bsp(seg_idx=2, bar_idx=2),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 110.0))

        snap3 = _FakeSnapshot(
            bar_idx=3,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
                _buy_bsp(seg_idx=2, bar_idx=2),
                _sell_bsp(seg_idx=3, bar_idx=3),
            ]),
        )
        engine.process_snapshot(snap3, _bar(3, 100.0))

        result = engine.result()
        assert result.trade_count == 2
        assert result.total_bars == 4


# ── 去重 ──


class TestDeduplication:
    """同一 BSP 不重复触发。"""

    def test_same_bsp_across_bars(self):
        """同一 confirmed BSP 在多个 bar 中出现，只触发一次开仓。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)
        bsp = _buy_bsp(seg_idx=0, bar_idx=0)

        for i in range(5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[bsp]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i))

        assert engine.has_open_position
        assert engine.result().trade_count == 0  # 未平仓不计入


# ── pipeline 特性 ──


class TestPipelineSpecific:
    """pipeline 特有逻辑的验证。"""

    def test_horizontal_nesting_advance(self):
        """入场后 horizontal nesting 推进。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))

        # horizontal 应已从初始状态推进
        assert len(engine.ctx.horizontal.completed_results) > 0

    def test_context_preserved_between_bars(self):
        """TradingContext 在多个 bar 间持续更新。"""
        config = PipelineBacktestConfig(config=FULL_RISK_ON)
        engine = PipelineBacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        ctx_after_bar0 = engine.ctx

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 105.0))

        # ctx 应是同一实例或更新后的实例，polarity 保持一致
        assert engine.ctx.polarity == ctx_after_bar0.polarity
