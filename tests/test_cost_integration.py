"""成本模型集成测试 — mock SlippageModel / CommissionModel 验证回测引擎成本逻辑。"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.backtest import BacktestConfig, BacktestEngine, CostSummary, Trade
from newchan.cost.config import CostConfig


# -- Mock 数据结构 --


@dataclass
class _MockBSP:
    seg_idx: int
    kind: str
    side: str
    level_id: int
    confirmed: bool


@dataclass
class _MockBSPSnapshot:
    buysellpoints: list[_MockBSP]


@dataclass
class _MockSnapshot:
    bar_idx: int
    bsp_snapshot: _MockBSPSnapshot


@dataclass
class _MockBar:
    close: float


# -- Mock 成本模型 --


class FixedSlippage:
    """固定点数滑点：buy +delta, sell -delta。"""

    def __init__(self, delta: float) -> None:
        self._delta = delta

    def apply(self, price: float, side: str) -> float:
        if side == "buy":
            return price + self._delta
        return price - self._delta


class FixedCommission:
    """固定费率手续费：price * quantity * rate。"""

    def __init__(self, rate: float) -> None:
        self._rate = rate

    def calculate(self, price: float, quantity: float) -> float:
        return price * quantity * self._rate


# -- 辅助函数 --


def _make_snap(bar_idx: int, bsps: list[_MockBSP]) -> _MockSnapshot:
    return _MockSnapshot(bar_idx=bar_idx, bsp_snapshot=_MockBSPSnapshot(bsps))


def _buy_bsp(seg_idx: int = 0, level_id: int = 1) -> _MockBSP:
    return _MockBSP(seg_idx=seg_idx, kind="1st", side="buy", level_id=level_id, confirmed=True)


def _sell_bsp(seg_idx: int = 1, level_id: int = 1) -> _MockBSP:
    return _MockBSP(seg_idx=seg_idx, kind="1st", side="sell", level_id=level_id, confirmed=True)


# -- 测试 --


class TestCostConfigDefaults:
    def test_default_no_models(self) -> None:
        cfg = CostConfig()
        assert cfg.slippage_model is None
        assert cfg.commission_model is None
        assert cfg.quantity == 1.0


class TestNoCostModel:
    """无成本模型时，行为与原始引擎一致。"""

    def test_no_cost_zero_fields(self) -> None:
        engine = BacktestEngine()
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        result = engine.result()
        assert result.trade_count == 1
        t = result.trades[0]
        assert t.entry_slippage == 0.0
        assert t.exit_slippage == 0.0
        assert t.entry_commission == 0.0
        assert t.exit_commission == 0.0
        assert t.total_cost == 0.0
        assert t.pnl == 10.0

    def test_cost_summary_zero(self) -> None:
        engine = BacktestEngine()
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        cs = engine.result().cost_summary
        assert cs.total_slippage == 0.0
        assert cs.total_commission == 0.0
        assert cs.total_cost == 0.0
        assert cs.cost_to_gross_profit_ratio == 0.0


class TestSlippageOnly:
    def test_slippage_applied_on_entry_and_exit(self) -> None:
        cost_cfg = CostConfig(slippage_model=FixedSlippage(0.5))
        engine = BacktestEngine(cost_config=cost_cfg)
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        t = engine.result().trades[0]
        # buy: 100 + 0.5 = 100.5
        assert t.entry_price == 100.5
        assert t.entry_slippage == 0.5
        # sell: 110 - 0.5 = 109.5
        assert t.exit_price == 109.5
        assert t.exit_slippage == -0.5
        # pnl = 109.5 - 100.5 = 9.0 (no commission)
        assert t.pnl == 9.0

    def test_cost_summary_slippage(self) -> None:
        cost_cfg = CostConfig(slippage_model=FixedSlippage(0.5))
        engine = BacktestEngine(cost_config=cost_cfg)
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        cs = engine.result().cost_summary
        assert cs.total_slippage == 1.0  # |0.5| + |-0.5|
        assert cs.total_commission == 0.0
        assert cs.total_cost == 1.0


class TestCommissionOnly:
    def test_commission_deducted_from_pnl(self) -> None:
        cost_cfg = CostConfig(commission_model=FixedCommission(0.001), quantity=100.0)
        engine = BacktestEngine(cost_config=cost_cfg)
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        t = engine.result().trades[0]
        # entry_commission = 100.0 * 100 * 0.001 = 10.0
        assert t.entry_commission == 10.0
        # exit_commission = 110.0 * 100 * 0.001 = 11.0
        assert t.exit_commission == 11.0
        # pnl = (110 - 100) - 10 - 11 = -11.0
        assert t.pnl == -11.0


class TestSlippageAndCommission:
    def test_combined_cost(self) -> None:
        cost_cfg = CostConfig(
            slippage_model=FixedSlippage(0.1),
            commission_model=FixedCommission(0.001),
            quantity=1.0,
        )
        engine = BacktestEngine(cost_config=cost_cfg)
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        t = engine.result().trades[0]
        # entry: 100 + 0.1 = 100.1, comm = 100.1 * 1 * 0.001 = 0.1001
        assert t.entry_price == 100.1
        assert abs(t.entry_commission - 0.1001) < 1e-9
        # exit: 110 - 0.1 = 109.9, comm = 109.9 * 1 * 0.001 = 0.1099
        assert t.exit_price == 109.9
        assert abs(t.exit_commission - 0.1099) < 1e-9
        # pnl = (109.9 - 100.1) - 0.1001 - 0.1099 = 9.59
        assert abs(t.pnl - 9.59) < 1e-9

    def test_cost_to_profit_ratio(self) -> None:
        cost_cfg = CostConfig(
            slippage_model=FixedSlippage(0.1),
            commission_model=FixedCommission(0.001),
            quantity=1.0,
        )
        engine = BacktestEngine(cost_config=cost_cfg)
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(110.0))
        cs = engine.result().cost_summary
        # gross_profit (raw) = 109.9 - 100.1 = 9.8
        # total_slippage = 0.1 + 0.1 = 0.2
        # total_commission = 0.1001 + 0.1099 = 0.21
        # total_cost = 0.41
        assert abs(cs.total_slippage - 0.2) < 1e-9
        assert abs(cs.total_commission - 0.21) < 1e-9
        assert abs(cs.total_cost - 0.41) < 1e-9
        assert cs.cost_to_gross_profit_ratio > 0


class TestBackwardCompatibility:
    """确保不传 cost_config 时，原有接口不受影响。"""

    def test_engine_without_cost_config(self) -> None:
        engine = BacktestEngine(config=BacktestConfig(stop_loss_pct=0))
        engine.process_snapshot(_make_snap(0, [_buy_bsp()]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp()]), _MockBar(120.0))
        result = engine.result()
        assert result.trade_count == 1
        assert result.trades[0].pnl == 20.0
        assert result.cost_summary.total_cost == 0.0

    def test_trade_frozen(self) -> None:
        t = Trade(
            side="long", bsp_kind="1st", bsp_level=1,
            entry_bar=0, exit_bar=1,
            entry_price=100.0, exit_price=110.0,
            exit_reason="reverse_bsp",
        )
        assert t.entry_slippage == 0.0
        assert t.exit_commission == 0.0


class TestCostSummaryNoTrades:
    def test_empty_cost_summary(self) -> None:
        engine = BacktestEngine()
        cs = engine.result().cost_summary
        assert cs.total_cost == 0.0
        assert cs.cost_to_gross_profit_ratio == 0.0


class TestMultipleTrades:
    def test_cost_accumulates(self) -> None:
        cost_cfg = CostConfig(slippage_model=FixedSlippage(0.5))
        engine = BacktestEngine(cost_config=cost_cfg)
        # Trade 1: buy@100 -> sell@110
        engine.process_snapshot(_make_snap(0, [_buy_bsp(seg_idx=0)]), _MockBar(100.0))
        engine.process_snapshot(_make_snap(1, [_sell_bsp(seg_idx=1)]), _MockBar(110.0))
        # Trade 2: buy@105 -> sell@115
        engine.process_snapshot(_make_snap(2, [_buy_bsp(seg_idx=2)]), _MockBar(105.0))
        engine.process_snapshot(_make_snap(3, [_sell_bsp(seg_idx=3)]), _MockBar(115.0))
        result = engine.result()
        assert result.trade_count == 2
        cs = result.cost_summary
        # Each trade: |0.5| + |-0.5| = 1.0 slippage
        assert abs(cs.total_slippage - 2.0) < 1e-9
