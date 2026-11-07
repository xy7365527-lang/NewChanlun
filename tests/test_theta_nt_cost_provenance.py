"""#1774 I-3：NT 回测输出把「准入记录」与「NT 撮合实际成本来源」分开，P0 已签后也不冒充 P0 成本。"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest
from nautilus_trader.backtest.models import FillModel

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "trading_system/backtest/theta_backtest_py.py"
sys.path.insert(0, str(ROOT))

from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402


def _load_cli():
    spec = importlib.util.spec_from_file_location("theta_backtest_py_cost_probe", CLI)
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


# NT 的 Rust 日志器每进程只能初始化一次，所以每次真引擎运行放进独立子进程。驱动用薄代理记下
# add_venue / add_instrument 的实参，跑完 main() 后把这些实参算出的成本来源行写到 stderr。
_DRIVER = r"""
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("theta_cli", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
calls = {}
real_engine = module.BacktestEngine

class Recorder:
    def __init__(self, **kwargs):
        self._engine = real_engine(**kwargs)

    def add_venue(self, **kwargs):
        calls["venue"] = kwargs
        return self._engine.add_venue(**kwargs)

    def add_instrument(self, instrument):
        calls["instrument"] = instrument
        return self._engine.add_instrument(instrument)

    def __getattr__(self, name):
        return getattr(self._engine, name)

module.BacktestEngine = Recorder
sys.argv = ["theta_backtest_py", *sys.argv[2:]]
code = module.main()
print(json.dumps({
    "exit": code,
    "fee_model_passed": "fee_model" in calls["venue"],
    "expected": module.cost_provenance(calls["instrument"], calls["venue"]["fill_model"]),
}), file=sys.stderr)
"""


@pytest.mark.parametrize("diagnostic", [False, True])
def test_cli_prints_cost_provenance_from_the_objects_passed_to_nt(tmp_path, diagnostic):
    closes = [30_000.0 + 5.0 * (i % 7) - 3.0 * (i % 5) for i in range(40)]
    data = tmp_path / "bars.json"
    data.write_text(json.dumps({
        "opens": closes,
        "highs": [c + 4.0 for c in closes],
        "lows": [c - 4.0 for c in closes],
        "closes": closes,
    }))
    args = ["--data", str(data), "--bars", "0", "--funding-profile", "P0"]
    if diagnostic:
        args.append("--diagnostic-fixture")
    proc = subprocess.run(
        [sys.executable, "-c", _DRIVER, str(CLI), *args],
        cwd=ROOT, capture_output=True, text=True, timeout=300)
    assert proc.returncode == 0, proc.stderr[-4000:]
    summary = json.loads(proc.stderr.strip().splitlines()[-1])
    assert summary["exit"] == 0
    assert summary["fee_model_passed"] is False, "NT 费用模型须为默认 MakerTakerFeeModel"
    expected = summary["expected"]
    assert "taker_fee=0.00035" in expected and "prob_slippage=0.0" in expected
    lines = proc.stdout.splitlines()
    assert f"撮合成本来源: {expected}" in lines
    (records,) = [line for line in lines if line.startswith("准入记录: ")]
    if diagnostic:
        assert "venue-profile-diagnostic-fixture/1" in records
        assert "准入: 诊断夹具，不参与准入" in lines
    else:
        assert "venue-profile-P0/4" in records
        assert "research-override" not in records
        assert "准入: 诊断夹具，不参与准入" not in lines
