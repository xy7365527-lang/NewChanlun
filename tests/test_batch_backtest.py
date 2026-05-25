"""batch_backtest 纯逻辑单元测试。

覆盖：
- backtest_result_to_row: BacktestResult → 扁平 dict
- results_to_dataframe: 多标的结果 → DataFrame
- CSV 输出格式
"""

from __future__ import annotations

import csv
import io
from pathlib import Path

import pytest

from newchan.backtest import BacktestResult, Trade

# 延迟导入，RED 阶段预期 ImportError
batch_bt = pytest.importorskip("batch_backtest")


# ── fixtures ──


def _make_trade(
    pnl_pct: float,
    *,
    side: str = "long",
    kind: str = "type1",
    level: int = 1,
    entry_bar: int = 0,
    exit_bar: int = 10,
    entry_price: float = 100.0,
    exit_reason: str = "reverse_bsp",
) -> Trade:
    exit_price = entry_price * (1 + pnl_pct) if side == "long" else entry_price * (1 - pnl_pct)
    return Trade(
        side=side,
        bsp_kind=kind,
        bsp_level=level,
        entry_bar=entry_bar,
        exit_bar=exit_bar,
        entry_price=entry_price,
        exit_price=exit_price,
        exit_reason=exit_reason,
    )


def _make_result(pnl_pcts: list[float], total_bars: int = 100) -> BacktestResult:
    trades = tuple(
        _make_trade(p, entry_bar=i * 10, exit_bar=i * 10 + 5)
        for i, p in enumerate(pnl_pcts)
    )
    return BacktestResult(trades=trades, total_bars=total_bars)


# ── backtest_result_to_row ──


class TestBacktestResultToRow:
    def test_basic_fields(self):
        result = _make_result([0.05, -0.02, 0.03])
        row = batch_bt.backtest_result_to_row("AAPL", result)
        assert row["symbol"] == "AAPL"
        assert row["trade_count"] == 3
        assert row["total_bars"] == 100
        assert row["total_cost"] >= 0.0

    def test_empty_result(self):
        result = _make_result([])
        row = batch_bt.backtest_result_to_row("EMPTY", result)
        assert row["trade_count"] == 0

    # 注：test_profit_loss_ratio 已删除（pending-004）——profit_loss_ratio 是策略
    # 评价指标，已从 BacktestResult/batch row 移除。


# ── results_to_dataframe ──


class TestResultsToDataframe:
    def test_columns_and_rows(self):
        results = {
            "AAPL": _make_result([0.05, -0.02]),
            "MSFT": _make_result([0.03]),
        }
        df = batch_bt.results_to_dataframe(results)
        assert len(df) == 2
        assert "symbol" in df.columns
        assert "trade_count" in df.columns
        assert set(df["symbol"]) == {"AAPL", "MSFT"}

    def test_sorted_by_symbol(self):
        results = {
            "ZZZ": _make_result([0.01]),
            "AAA": _make_result([0.02]),
        }
        df = batch_bt.results_to_dataframe(results)
        assert list(df["symbol"]) == ["AAA", "ZZZ"]


# ── CSV 输出 ──


class TestCsvOutput:
    def test_write_csv(self, tmp_path: Path):
        results = {
            "SYM1": _make_result([0.05, -0.03]),
        }
        out = tmp_path / "report.csv"
        batch_bt.write_csv(results, out)
        assert out.exists()
        with open(out, encoding="utf-8") as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        assert len(rows) == 1
        assert rows[0]["symbol"] == "SYM1"
        assert int(rows[0]["trade_count"]) == 2
