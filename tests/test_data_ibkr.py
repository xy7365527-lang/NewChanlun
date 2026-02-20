"""Tests for newchan.data_ibkr — IBKR 数据源模块。

所有 IB 连接和 API 调用均 mock，只测试本地逻辑。

在全量测试中，其他测试可能导致 ib_insync 模块状态异常，
因此在导入 data_ibkr 之前通过 sys.modules 注入 mock。
"""

from __future__ import annotations

import collections
import importlib
import sys
from datetime import datetime
from types import ModuleType, SimpleNamespace
from unittest.mock import MagicMock, patch

import pytest

from newchan.types import Bar

# ── 在导入 data_ibkr 之前，确保 ib_insync mock 就位 ──


def _make_contract_factory(sec_type: str):
    """创建一个模拟 ib_insync 合约类的工厂函数。"""
    def factory(**kwargs):
        ns = SimpleNamespace(secType=sec_type, **kwargs)
        return ns
    factory.__name__ = sec_type
    return factory


_mock_ib_module = ModuleType("ib_insync")
_mock_ib_module.IB = MagicMock  # type: ignore[attr-defined]
_mock_ib_module.ContFuture = _make_contract_factory("CONTFUT")  # type: ignore[attr-defined]
_mock_ib_module.Contract = _make_contract_factory("CONTRACT")  # type: ignore[attr-defined]
_mock_ib_module.Future = _make_contract_factory("FUT")  # type: ignore[attr-defined]
_mock_ib_module.Stock = _make_contract_factory("STK")  # type: ignore[attr-defined]
_mock_ib_module.util = MagicMock()  # type: ignore[attr-defined]


def _get_module():
    """获取 data_ibkr 模块，必要时强制重新加载以确保 mock 生效。"""
    # 如果模块已加载但 IB 属性丢失，强制重新加载
    mod = sys.modules.get("newchan.data_ibkr")
    if mod is not None and hasattr(mod, "IB"):
        return mod

    # 确保 ib_insync mock 在 sys.modules 中
    orig = sys.modules.get("ib_insync")
    sys.modules["ib_insync"] = _mock_ib_module
    try:
        if mod is not None:
            # 模块已加载但 IB 丢失，重新加载
            importlib.reload(mod)
            return mod
        from newchan import data_ibkr
        return data_ibkr
    finally:
        # 恢复原始模块（如果有的话）
        if orig is not None:
            sys.modules["ib_insync"] = orig


@pytest.fixture(autouse=True)
def _ensure_ib_mock():
    """确保每个测试都能访问到带有 mock 属性的 data_ibkr 模块。"""
    orig = sys.modules.get("ib_insync")
    sys.modules["ib_insync"] = _mock_ib_module
    mod = _get_module()
    # 强制将 mock 类绑定到模块上，覆盖可能已导入的真实类
    mod.IB = MagicMock
    mod.ContFuture = _mock_ib_module.ContFuture
    mod.Stock = _mock_ib_module.Stock
    mod.Contract = _mock_ib_module.Contract
    mod.Future = _mock_ib_module.Future
    yield
    if orig is not None:
        sys.modules["ib_insync"] = orig


def _import_module():
    """延迟导入 data_ibkr 模块。"""
    return _get_module()


# ── supported_symbols ──


class TestSupportedSymbols:
    def test_returns_sorted_list(self):
        mod = _import_module()
        symbols = mod.supported_symbols()
        assert isinstance(symbols, list)
        assert symbols == sorted(symbols)

    def test_contains_known_symbols(self):
        mod = _import_module()
        symbols = mod.supported_symbols()
        for s in ["CL", "GC", "ES", "NQ"]:
            assert s in symbols


# ── _BAR_SIZE_MAP ──


class TestBarSizeMap:
    def test_common_intervals_exist(self):
        mod = _import_module()
        for interval in ["1min", "5min", "15min", "1hour", "1day"]:
            assert interval in mod._BAR_SIZE_MAP

    def test_values_are_ib_format(self):
        mod = _import_module()
        assert mod._BAR_SIZE_MAP["1min"] == "1 min"
        assert mod._BAR_SIZE_MAP["5min"] == "5 mins"
        assert mod._BAR_SIZE_MAP["1day"] == "1 day"


# ── IBKRProvider.make_contract ──


class TestMakeContract:
    def test_futures_symbol_returns_cont_future(self):
        mod = _import_module()
        contract = mod.IBKRProvider.make_contract("CL")
        assert contract.symbol == "CL"
        assert contract.secType == "CONTFUT"

    def test_futures_symbol_case_insensitive(self):
        mod = _import_module()
        contract = mod.IBKRProvider.make_contract("cl")
        assert contract.symbol == "CL"

    def test_unknown_symbol_returns_stock(self):
        mod = _import_module()
        contract = mod.IBKRProvider.make_contract("AAPL")
        assert contract.symbol == "AAPL"
        assert contract.secType == "STK"

    def test_futures_exchange_mapping(self):
        mod = _import_module()
        contract = mod.IBKRProvider.make_contract("GC")
        assert contract.exchange == "COMEX"


# ── IBKRProvider.__init__ ──


class TestIBKRProviderInit:
    def test_default_config(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            provider = mod.IBKRProvider()
            assert provider.host == mod.IB_HOST
            assert provider.port == mod.IB_PORT

    def test_custom_config(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            provider = mod.IBKRProvider(host="10.0.0.1", port=4001, client_id=42)
            assert provider.host == "10.0.0.1"
            assert provider.port == 4001
            assert provider.client_id == 42


# ── IBKRProvider.fetch_historical ──


class TestFetchHistorical:
    def test_invalid_interval_raises(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            provider = mod.IBKRProvider()
            with pytest.raises(ValueError, match="不支持的 interval"):
                provider.fetch_historical("CL", interval="invalid")

    def test_fetch_returns_bars(self):
        mod = _import_module()
        mock_ib = MagicMock()
        mock_ib.qualifyContracts.return_value = [MagicMock()]

        mock_bar = SimpleNamespace(
            date=datetime(2024, 1, 1, 10, 0),
            open=70.0,
            high=71.0,
            low=69.0,
            close=70.5,
            volume=100,
        )
        mock_ib.reqHistoricalData.return_value = [mock_bar]

        with patch.object(mod, "IB", return_value=mock_ib):
            provider = mod.IBKRProvider()
            provider._ib = mock_ib
            bars = provider.fetch_historical("CL", interval="1min")

        assert len(bars) == 1
        assert isinstance(bars[0], Bar)
        assert bars[0].open == 70.0
        assert bars[0].close == 70.5
        assert bars[0].volume == 100

    def test_fetch_empty_returns_empty_list(self):
        mod = _import_module()
        mock_ib = MagicMock()
        mock_ib.qualifyContracts.return_value = [MagicMock()]
        mock_ib.reqHistoricalData.return_value = []

        with patch.object(mod, "IB", return_value=mock_ib):
            provider = mod.IBKRProvider()
            provider._ib = mock_ib
            bars = provider.fetch_historical("CL")

        assert bars == []

    def test_fetch_qualify_fails_raises(self):
        mod = _import_module()
        mock_ib = MagicMock()
        mock_ib.qualifyContracts.return_value = []

        with patch.object(mod, "IB", return_value=mock_ib):
            provider = mod.IBKRProvider()
            provider._ib = mock_ib
            with pytest.raises(RuntimeError, match="无法解析合约"):
                provider.fetch_historical("CL")

    def test_zero_volume_becomes_none(self):
        mod = _import_module()
        mock_ib = MagicMock()
        mock_ib.qualifyContracts.return_value = [MagicMock()]

        mock_bar = SimpleNamespace(
            date=datetime(2024, 1, 1),
            open=1.0, high=2.0, low=0.5, close=1.5,
            volume=0,
        )
        mock_ib.reqHistoricalData.return_value = [mock_bar]

        with patch.object(mod, "IB", return_value=mock_ib):
            provider = mod.IBKRProvider()
            provider._ib = mock_ib
            bars = provider.fetch_historical("CL")

        assert bars[0].volume is None


# ── IBKRConnection ──


class TestIBKRConnection:
    @pytest.fixture(autouse=True)
    def reset_singleton(self):
        """每个测试重置单例。"""
        mod = _import_module()
        mod.IBKRConnection._instance = None
        yield
        mod.IBKRConnection._instance = None

    def test_singleton(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            a = mod.IBKRConnection.instance()
            b = mod.IBKRConnection.instance()
            assert a is b

    def test_initial_state_not_connected(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            assert conn.connected is False

    def test_get_subscriptions_empty(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            assert conn.get_subscriptions() == []

    def test_get_latest_bars_no_subscription(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            bars, idx = conn.get_latest_bars("CL")
            assert bars == []
            assert idx == 0

    def test_get_latest_bars_with_buffer(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            conn._rt_buffers["CL"] = collections.deque(
                [
                    Bar(ts=datetime(2024, 1, 1, 10, 0), open=70.0, high=71.0, low=69.0, close=70.5, volume=100),
                    Bar(ts=datetime(2024, 1, 1, 10, 5), open=70.5, high=72.0, low=70.0, close=71.0, volume=200),
                ],
                maxlen=500,
            )

            bars, new_idx = conn.get_latest_bars("CL", since_idx=0)
            assert len(bars) == 2
            assert new_idx == 2
            assert bars[0]["open"] == 70.0
            assert bars[1]["close"] == 71.0

    def test_get_latest_bars_incremental(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            conn._rt_buffers["CL"] = collections.deque(
                [
                    Bar(ts=datetime(2024, 1, 1, 10, 0), open=70.0, high=71.0, low=69.0, close=70.5, volume=100),
                    Bar(ts=datetime(2024, 1, 1, 10, 5), open=70.5, high=72.0, low=70.0, close=71.0, volume=200),
                ],
                maxlen=500,
            )

            _, idx = conn.get_latest_bars("CL", since_idx=0)
            bars, new_idx = conn.get_latest_bars("CL", since_idx=idx)
            assert len(bars) == 0
            assert new_idx == 2

    def test_search_symbols_not_connected(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            result = conn.search_symbols("crude")
            assert result == []

    def test_pump_not_connected(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            conn.pump()  # should not raise

    def test_subscribe_not_connected(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            assert conn.subscribe_realtime("CL") is False

    def test_on_rt_bar_callback(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            conn._rt_buffers["CL"] = collections.deque(maxlen=500)

            mock_bar = SimpleNamespace(
                time=datetime(2024, 1, 1, 10, 0),
                open_=70.0, high=71.0, low=69.0, close=70.5,
                volume=100, wap=0, count=0,
            )
            conn._on_rt_bar("CL", [mock_bar], has_new=True)

            assert len(conn._rt_buffers["CL"]) == 1
            bar = conn._rt_buffers["CL"][0]
            assert bar.open == 70.0
            assert bar.close == 70.5

    def test_on_rt_bar_ignores_no_new(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            conn._rt_buffers["CL"] = collections.deque(maxlen=500)
            conn._on_rt_bar("CL", [], has_new=False)
            assert len(conn._rt_buffers["CL"]) == 0

    def test_unsubscribe(self):
        mod = _import_module()
        with patch.object(mod, "IB"):
            conn = mod.IBKRConnection()
            mock_handle = MagicMock()
            conn._rt_subs["CL"] = mock_handle
            conn._rt_buffers["CL"] = collections.deque(maxlen=500)

            mock_ib = MagicMock()
            conn._ib = mock_ib

            conn.unsubscribe("CL")
            assert "CL" not in conn._rt_subs
            assert "CL" not in conn._rt_buffers
