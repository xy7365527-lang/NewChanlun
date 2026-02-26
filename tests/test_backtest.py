"""回测引擎测试 — 合成数据验证。

覆盖：
- 买入信号开仓 + 卖出信号平仓
- 止损触发
- 无信号时无交易
- 统计指标计算
- 防未来函数：入场价 = bar.close
- allow_short 配置
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Literal

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.backtest import BacktestConfig, BacktestEngine, BacktestResult, Trade
from newchan.types import Bar


# ── Stub 类型 ──


@dataclass
class _BspSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    buysellpoints: list[BuySellPoint] = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _FakeSnapshot:
    """最小化 RecursiveOrchestratorSnapshot stub。"""

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


class TestBasicFlow:
    """买入 → 卖出完整流程。"""

    def test_buy_then_sell(self):
        """买入信号开仓，卖出信号平仓，产生一笔盈利交易。"""
        engine = BacktestEngine()

        # bar 0: 买入信号确认，close=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        # bar 1-4: 无新信号，价格上涨
        for i in range(1, 5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i * 5))

        # bar 5: 卖出信号确认，close=130
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

    def test_entry_price_is_bar_close(self):
        """防未来函数：入场价 = bar.close，不是 BSP.price。"""
        engine = BacktestEngine()
        # BSP.price=95（段极值），但 bar.close=100
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, price=95.0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.has_open_position
        # 内部 entry_price 应为 100（bar.close），不是 95（BSP.price）
        # 通过平仓验证
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


class TestNoSignal:
    """无信号时不产生交易。"""

    def test_no_bsp_no_trade(self):
        engine = BacktestEngine()
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
        """未确认的 BSP 不触发开仓。"""
        engine = BacktestEngine()
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, confirmed=False),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position


class TestStopLoss:
    """止损逻辑。"""

    def test_stop_loss_triggers(self):
        """价格跌破止损线时平仓。"""
        config = BacktestConfig(stop_loss_pct=0.05)
        engine = BacktestEngine(config)

        # 开仓 close=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        # 跌到 94（亏 6%，超过 5% 止损线）
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0)]),
        )
        engine.process_snapshot(snap1, _bar(1, 94.0))

        assert not engine.has_open_position
        result = engine.result()
        assert result.trade_count == 1
        assert result.trades[0].exit_reason == "stop_loss"
        assert result.trades[0].pnl == -6.0

    def test_stop_loss_disabled(self):
        """stop_loss_pct=0 时不触发止损。"""
        config = BacktestConfig(stop_loss_pct=0.0)
        engine = BacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0)]),
        )
        engine.process_snapshot(snap1, _bar(1, 80.0))

        assert engine.has_open_position  # 未平仓


class TestStatistics:
    """统计指标。"""

    def test_win_rate_and_ratio(self):
        """两笔交易：一盈一亏，验证胜率和盈亏比。"""
        engine = BacktestEngine()

        # 第一笔：买100 卖120 → +20
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

        # 第二笔：买110 卖100 → -10
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
        assert result.win_count == 1
        assert result.loss_count == 1
        assert result.win_rate == 0.5
        # 盈亏比 = 20 / 10 = 2.0
        assert result.profit_loss_ratio == 2.0

    def test_max_drawdown(self):
        """最大回撤计算。"""
        t1 = Trade(
            side="long", bsp_kind="type1", bsp_level=1,
            entry_bar=0, exit_bar=1, entry_price=100.0, exit_price=120.0,
            exit_reason="reverse_bsp",
        )
        t2 = Trade(
            side="long", bsp_kind="type1", bsp_level=1,
            entry_bar=2, exit_bar=3, entry_price=110.0, exit_price=88.0,
            exit_reason="stop_loss",
        )
        result = BacktestResult(trades=(t1, t2), total_bars=4)
        # t1: pnl_pct = 20/100 = 0.2, cum=0.2, peak=0.2
        # t2: pnl_pct = -22/110 = -0.2, cum=0.0, dd=0.2
        assert result.max_drawdown_pct == 0.2

    def test_empty_result(self):
        """无交易时统计指标安全返回。"""
        result = BacktestResult(trades=(), total_bars=0)
        assert result.win_rate == 0.0
        assert result.profit_loss_ratio == 0.0
        assert result.max_drawdown_pct == 0.0


class TestShortSelling:
    """做空逻辑。"""

    def test_short_disabled_by_default(self):
        """默认不允许做空，sell BSP 不开仓。"""
        engine = BacktestEngine()
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position

    def test_short_enabled(self):
        """allow_short=True 时 sell BSP 开空仓。"""
        config = BacktestConfig(allow_short=True)
        engine = BacktestEngine(config)

        # 卖出信号开空仓 close=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # 买入信号平空仓 close=90 → 盈利 10
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _sell_bsp(seg_idx=0, bar_idx=0),
                _buy_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 90.0))

        result = engine.result()
        assert result.trade_count == 1
        t = result.trades[0]
        assert t.side == "short"
        assert t.pnl == 10.0


class TestDuplicateBspIgnored:
    """同一 BSP 不重复触发。"""

    def test_same_bsp_across_bars(self):
        """同一 confirmed BSP 在多个 bar 的 snapshot 中出现，只触发一次开仓。"""
        engine = BacktestEngine()
        bsp = _buy_bsp(seg_idx=0, bar_idx=0)

        for i in range(5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[bsp]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i))

        # 只开了一次仓，未平仓
        assert engine.has_open_position
        assert engine.result().trade_count == 0  # 未平仓不计入


class TestShortStopLoss:
    """空仓止损。"""

    def test_short_stop_loss(self):
        """空仓价格上涨超过止损线时平仓。"""
        config = BacktestConfig(stop_loss_pct=0.05, allow_short=True)
        engine = BacktestEngine(config)

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        # 涨到 106（亏 6%）
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap1, _bar(1, 106.0))

        assert not engine.has_open_position
        result = engine.result()
        assert result.trades[0].exit_reason == "stop_loss"
        assert result.trades[0].pnl == -6.0
