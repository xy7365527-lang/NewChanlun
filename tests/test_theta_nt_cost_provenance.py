"""#1774 I-3：NT 回测输出把「准入记录」与「NT 撮合实际成本来源」分开，P0 已签后也不冒充 P0 成本。"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest
from nautilus_trader.backtest.models import FillModel

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402


def _load_cli():
    spec = importlib.util.spec_from_file_location(
        "theta_backtest_py_cost_probe", ROOT / "trading_system/backtest/theta_backtest_py.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_cost_provenance_reads_nt_instrument_and_fill_model():
    line = _load_cli().cost_provenance(
        make_instrument(INSTRUMENTS["BTC"]), FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0))
    assert "BTC-USD-PERP.HYPERLIQUID" in line
    assert "maker_fee=0.0001" in line
    assert "taker_fee=0.00035" in line
    assert "price_increment=0.001" in line
    assert "prob_slippage=0.0" in line
    assert "未被 NT 撮合使用" in line
    assert "slippage_bps=2" not in line


class _VenueRecorder:
    """真 BacktestEngine 的薄代理：只记下 add_venue / add_instrument 实参，其余原样转发。"""

    def __init__(self, engine_cls, calls, **kwargs):
        self._engine = engine_cls(**kwargs)
        self._calls = calls

    def add_venue(self, **kwargs):
        self._calls.append(("venue", kwargs))
        return self._engine.add_venue(**kwargs)

    def add_instrument(self, instrument):
        self._calls.append(("instrument", instrument))
        return self._engine.add_instrument(instrument)

    def __getattr__(self, name):
        return getattr(self._engine, name)


@pytest.mark.parametrize("diagnostic", [False, True])
def test_cli_prints_cost_provenance_from_the_objects_passed_to_nt(
        monkeypatch, tmp_path, capsys, diagnostic):
    closes = [30_000.0 + 5.0 * (i % 7) - 3.0 * (i % 5) for i in range(40)]
    data = tmp_path / "bars.json"
    data.write_text(json.dumps({
        "opens": closes,
        "highs": [c + 4.0 for c in closes],
        "lows": [c - 4.0 for c in closes],
        "closes": closes,
    }))
    module = _load_cli()
    calls: list[tuple[str, object]] = []
    real_engine = module.BacktestEngine
    monkeypatch.setattr(
        module, "BacktestEngine", lambda **kw: _VenueRecorder(real_engine, calls, **kw))
    argv = ["theta_backtest_py", "--data", str(data), "--bars", "0", "--funding-profile", "P0"]
    if diagnostic:
        argv.append("--diagnostic-fixture")
    monkeypatch.setattr(sys, "argv", argv)

    assert module.main() == 0

    (venue_kwargs,) = [kw for kind, kw in calls if kind == "venue"]
    (instrument,) = [inst for kind, inst in calls if kind == "instrument"]
    assert "fee_model" not in venue_kwargs, "NT 费用模型须为默认 MakerTakerFeeModel"
    expected = module.cost_provenance(instrument, venue_kwargs["fill_model"])
    assert "taker_fee=0.00035" in expected and "prob_slippage=0.0" in expected
    lines = capsys.readouterr().out.splitlines()
    assert f"撮合成本来源: {expected}" in lines
    (records,) = [line for line in lines if line.startswith("准入记录: ")]
    if diagnostic:
        assert "venue-profile-diagnostic-fixture/1" in records
        assert "准入: 诊断夹具，不参与准入" in lines
    else:
        assert "venue-profile-P0/4" in records
        assert "research-override" not in records
        assert "准入: 诊断夹具，不参与准入" not in lines
