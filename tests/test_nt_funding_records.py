"""#1601：NT 回放的 venue 与 Rust 引擎必须共用 funding-record。"""

from __future__ import annotations

import importlib.util
import sys
import types
from pathlib import Path
from types import SimpleNamespace as NS

import pytest

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    "fugue_v3": ("trading_system/backtest_fugue_v3.py", ("ERROR", False)),
    "t_fugue": ("trading_system/backtest_t_fugue.py", ("structural", "ERROR")),
    "unn_stream": ("trading_system/backtest_unn_stream.py", ("ERROR",)),
}


class EngineRecorder:
    def __init__(self, **kwargs):
        self.kwargs = kwargs
        self.calls = []

    def add_venue(self, **kwargs):
        self.calls.append(("add_venue", kwargs))

    def add_instrument(self, instrument):
        self.calls.append(("add_instrument", instrument))

    def add_strategy(self, strategy):
        self.calls.append(("add_strategy", strategy))


class StrategyConfig:
    def __init_subclass__(cls, **kwargs):
        pass

    def __init__(self, **kwargs):
        self.__dict__.update(kwargs)


class Strategy:
    def __init__(self, config):
        self.config = config

    def subscribe_bars(self, bar_type):
        pass


def _module(monkeypatch, name, **attrs):
    module = types.ModuleType(name)
    for key, value in attrs.items():
        setattr(module, key, value)
    monkeypatch.setitem(sys.modules, name, module)
    return module


def _install(monkeypatch):
    """只替换 NT/数据/引擎外部协作者；被测 build_engine 仍是生产源码。"""
    _module(monkeypatch, "nautilus_trader")
    _module(monkeypatch, "nautilus_trader.backtest")
    _module(monkeypatch, "nautilus_trader.backtest.engine", BacktestEngine=EngineRecorder)
    _module(monkeypatch, "nautilus_trader.backtest.config", BacktestEngineConfig=lambda **kw: NS(**kw))
    _module(monkeypatch, "nautilus_trader.backtest.models", FillModel=lambda **kw: NS(**kw))
    _module(
        monkeypatch,
        "nautilus_trader.config",
        LoggingConfig=lambda **kw: NS(**kw),
        StrategyConfig=StrategyConfig,
    )
    _module(monkeypatch, "nautilus_trader.model")
    _module(monkeypatch, "nautilus_trader.model.currencies", USD="USD")
    _module(
        monkeypatch,
        "nautilus_trader.model.data",
        Bar=object,
        BarType=type("BarType", (), {"from_str": staticmethod(lambda value: value)}),
    )
    _module(
        monkeypatch,
        "nautilus_trader.model.enums",
        AccountType=NS(MARGIN="margin"),
        OmsType=NS(NETTING="netting"),
    )
    _module(monkeypatch, "nautilus_trader.model.identifiers", InstrumentId=NS, TraderId=lambda value: value)
    _module(
        monkeypatch,
        "nautilus_trader.model.objects",
        Money=lambda amount, currency: NS(amount=amount, currency=currency),
    )
    _module(monkeypatch, "nautilus_trader.trading")
    _module(monkeypatch, "nautilus_trader.trading.strategy", Strategy=Strategy)

    calls = []

    def read_funding_record(profile):
        calls.append(profile)
        if profile is None:
            raise ValueError("AwaitingPolicy{funding_record}")
        amounts = {"P0": 100_000, "P0-acceptance-5m": 5_000_000}
        return {"q_total": amounts[profile], "record_id": f"{profile}/v1"}

    _module(monkeypatch, "newchan_rust", read_funding_record=read_funding_record)
    _module(
        monkeypatch,
        "nested_recursive_fugue_final_backtest",
        FLOOR=2,
        LADDER_NAMES={},
        bh_mdd=lambda _closes: 0.0,
        analyze=lambda *_args, **_kwargs: {},
    )
    _module(
        monkeypatch,
        "organic_signals",
        StreamingSignalReader=object,
        push_signal=lambda *_args: [],
        compute_organic_signals=lambda *_args, **_kwargs: [],
    )
    _module(monkeypatch, "organic_fugue_rust_check", pack_tape=lambda value: value)
    _module(monkeypatch, "trading_system.backtest_unn", load_bars=lambda *_args: None)
    _module(
        monkeypatch,
        "trading_system.config.instruments",
        INSTRUMENTS={"CL": NS()},
        make_instrument=lambda _spec: NS(id=NS(venue="GLBX")),
    )
    return calls


def _load_target(monkeypatch, key):
    path, _ = TARGETS[key]
    name = f"nt_funding_probe_{key}"
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    monkeypatch.setitem(sys.modules, name, module)
    spec.loader.exec_module(module)
    return module


def _build(module, key):
    _, args = TARGETS[key]
    engine, _, _ = module.build_engine(object(), *args)
    return engine


@pytest.mark.parametrize("key", TARGETS)
@pytest.mark.parametrize(
    ("profile", "expected"),
    [("P0", 100_000), ("P0-acceptance-5m", 5_000_000)],
)
def test_venue_uses_the_same_funding_record(monkeypatch, key, profile, expected):
    calls = _install(monkeypatch)
    module = _load_target(monkeypatch, key)
    monkeypatch.setattr(module, "FUNDING_PROFILE", profile)

    engine = _build(module, key)

    venue = next(value for kind, value in engine.calls if kind == "add_venue")
    balance = venue["starting_balances"][0]
    assert balance.amount == expected
    assert balance.currency == "USD"
    assert calls == [profile]


@pytest.mark.parametrize("key", TARGETS)
def test_missing_profile_stops_before_engine_assembly(monkeypatch, key):
    calls = _install(monkeypatch)
    module = _load_target(monkeypatch, key)
    monkeypatch.setattr(module, "FUNDING_PROFILE", None)

    def forbidden_engine(**_kwargs):
        raise AssertionError("record was not read before BacktestEngine assembly")

    monkeypatch.setattr(module, "BacktestEngine", forbidden_engine)
    with pytest.raises(ValueError, match=r"AwaitingPolicy\{funding_record"):
        _build(module, key)
    assert calls == [None]


@pytest.mark.parametrize("key", TARGETS)
@pytest.mark.parametrize("message", ["RecordMissing", "FieldMissing", "Invalid"])
def test_record_errors_stop_before_engine_assembly(monkeypatch, key, message):
    """记录层的任一读错都不得被 venue 装配吞掉；字段解析由 Rust 自身测试覆盖。"""
    _install(monkeypatch)
    module = _load_target(monkeypatch, key)
    monkeypatch.setattr(module, "FUNDING_PROFILE", "P0")
    calls = []

    def fail_read(profile):
        calls.append(profile)
        raise ValueError(message)

    monkeypatch.setattr(module.nr, "read_funding_record", fail_read)

    def forbidden_engine(**_kwargs):
        raise AssertionError("venue assembly continued after a record error")

    monkeypatch.setattr(module, "BacktestEngine", forbidden_engine)
    with pytest.raises(ValueError, match=message):
        _build(module, key)
    assert calls == ["P0"]


@pytest.mark.parametrize("diagnostic", [False, True])
def test_theta_strategy_forwards_funding_and_venue_separately(monkeypatch, diagnostic):
    _install(monkeypatch)
    sys.modules["nautilus_trader.model.enums"].OrderSide = NS(BUY="buy", SELL="sell")
    calls = []
    monkeypatch.setattr(sys.modules["newchan_rust"], "ThetaStream",
                        lambda **kw: calls.append(kw) or NS(), raising=False)
    spec = importlib.util.spec_from_file_location(
        "theta_strategy_funding_probe", ROOT / "trading_system/strategy/theta_rec_strategy.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.ThetaRecStrategy(module.ThetaRecStrategyConfig(
        instrument_id=NS(venue="venue"), bar_type="bar", funding_profile="P1-hl-testnet",
        preset="diagnostic-fixture" if diagnostic else None,
        venue_profile=None if diagnostic else "P0"))
    assert calls == [{"funding_profile": "P1-hl-testnet",
                      "preset": "diagnostic-fixture" if diagnostic else None,
                      "venue_profile": None if diagnostic else "P0"}]


@pytest.mark.parametrize("diagnostic", [False, True])
@pytest.mark.parametrize("funding", [None, "P1-hl-testnet"])
def test_theta_cli_keeps_both_profiles_and_uses_signed_balance(monkeypatch, tmp_path, diagnostic, funding):
    _install(monkeypatch)
    sys.modules["nautilus_trader.model.currencies"].USDC = "USDC"
    sys.modules["nautilus_trader.model.identifiers"].Venue = str
    objects = sys.modules["nautilus_trader.model.objects"]
    objects.Price = objects.Quantity = object
    _module(monkeypatch, "pandas")
    _module(monkeypatch, "pytz")
    instrument = NS(id=NS(venue="venue"))
    sys.modules["trading_system.config.instruments"].INSTRUMENTS = {"BTC": NS()}
    sys.modules["trading_system.config.instruments"].make_instrument = lambda _s: instrument
    calls = []
    def strategy(config):
        calls.append(config)
        if config.funding_profile is None:
            raise ValueError("AwaitingPolicy{funding_record}")
        return NS(engine=NS(q_total=500.0, funding_record="P1-hl-testnet/v1"))
    _module(monkeypatch, "trading_system.strategy.theta_rec_strategy",
            ThetaRecStrategy=strategy, ThetaRecStrategyConfig=lambda **kw: NS(**kw))
    spec = importlib.util.spec_from_file_location(
        "theta_cli_funding_probe", ROOT / "trading_system/backtest/theta_backtest_py.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    data = tmp_path / "bars.json"
    data.write_text("{}")
    argv = ["theta", "--data", str(data)]
    if diagnostic:
        argv += ["--diagnostic-fixture"]
    if funding:
        argv += ["--funding-profile", funding]
    monkeypatch.setattr(sys, "argv", argv)
    class VenueReached(Exception):
        pass
    def engine(**_kwargs):
        assert funding is not None, "missing funding must stop before engine assembly"
        def add_venue(**kwargs):
            balance = kwargs["starting_balances"][0]
            assert (balance.amount, balance.currency) == (500.0, "USDC")
            raise VenueReached
        return NS(add_venue=add_venue)
    monkeypatch.setattr(module, "BacktestEngine", engine)
    if funding is None:
        assert module.main() == 2
    else:
        with pytest.raises(VenueReached):
            module.main()
    assert len(calls) == 1
    assert calls[0].funding_profile == funding
    assert calls[0].venue_profile == (None if diagnostic else "P0")
    assert calls[0].preset == ("diagnostic-fixture" if diagnostic else None)
