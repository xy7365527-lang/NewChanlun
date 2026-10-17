import pytest
from trading_system.execution.hl_paper_launcher import LauncherConfig, LauncherConfigError, ReportSink, build_node

def test_environment_scopes_credentials():
    assert LauncherConfig.from_env({"HL_ENVIRONMENT": "testnet", "HYPERLIQUID_TESTNET_PK": "x"}).private_key == "x"
    with pytest.raises(LauncherConfigError): LauncherConfig.from_env({"HL_ENVIRONMENT": "mainnet", "HYPERLIQUID_TESTNET_PK": "x"})

def test_invalid_environment_fails():
    with pytest.raises(LauncherConfigError): LauncherConfig.from_env({"HL_ENVIRONMENT": "prod", "HYPERLIQUID_PK": "x"})

def test_missing_credential_precedes_nautilus_import():
    with pytest.raises(LauncherConfigError): LauncherConfig.from_env({"HL_ENVIRONMENT": "testnet"})

def test_dry_run_never_builds_node():
    c = LauncherConfig.from_env({"HL_ENVIRONMENT": "testnet"}, dry_run=True)
    with pytest.raises(LauncherConfigError): build_node(c)

def test_sink_redacts_secret_fields(tmp_path):
    p = tmp_path / "events.jsonl"
    ReportSink(p, emit=lambda _: None).write("x", private_key="super-secret", nested={"api_token": "another-secret"})
    text = p.read_text()
    assert "super-secret" not in text and "another-secret" not in text
    assert "REDACTED" in text

from trading_system.execution.hl_paper_launcher import HyperliquidPaperFacade
from trading_system.execution.risk_controls import OrderIntent, RiskGate, RiskLimits

class _FakeStrategy:
    def __init__(self):
        self.submitted = []
        self.cancelled = 0
        self.flattened = 0
    def submit_order(self, *args, **kwargs):
        self.submitted.append((args, kwargs))
        return type("Order", (), {"client_order_id": kwargs.get("client_order_id", "generated")})()
    def cancel_order(self, order):
        self.cancelled += 1
    def cancel_all_orders(self):
        self.cancelled += 1
    def close_all_positions(self):
        self.flattened += 1

def test_facade_risk_gate_blocks_before_strategy_and_records_reason():
    events = []
    strategy = _FakeStrategy()
    gate = RiskGate(RiskLimits(max_single_order_frac=0.01))
    facade = HyperliquidPaperFacade(strategy, ReportSink(emit=events.append), risk_gate=gate,
                                    equity=lambda: 100.0, net_position_notional=lambda _: 0.0,
                                    now_ns=lambda: 1)
    assert facade.submit_limit("BTC", 0.02, 100.0, side="BUY", client_order_id="r1") is None
    assert strategy.submitted == []
    assert '"event": "rejected"' in events[0]
    assert "single_order_over_limit" in events[0]

def test_facade_risk_gate_allows_and_uses_single_submit_path():
    events = []
    strategy = _FakeStrategy()
    facade = HyperliquidPaperFacade(strategy, ReportSink(emit=events.append), risk_gate=RiskGate(),
                                    equity=lambda: 10000.0, net_position_notional=lambda _: 0.0,
                                    now_ns=lambda: 1)
    order = facade.submit_limit("BTC", 0.001, 100.0, side="BUY", client_order_id="a1")
    assert order is not None and len(strategy.submitted) == 1
    assert '"event": "accepted"' in events[0]

def test_facade_kill_switch_runs_cancel_and_flatten_actions():
    events = []
    strategy = _FakeStrategy(); gate = RiskGate(); gate.trip("test")
    facade = HyperliquidPaperFacade(strategy, ReportSink(emit=events.append), risk_gate=gate,
                                    equity=lambda: 10000.0, net_position_notional=lambda _: 0.0,
                                    now_ns=lambda: 1)
    assert facade.submit_limit("BTC", 0.001, 100.0, side="BUY", client_order_id="k1") is None
    assert strategy.cancelled == 1 and strategy.flattened == 1
    assert any('kill_switch_actions' in e for e in events)
