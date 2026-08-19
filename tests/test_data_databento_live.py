"""DatabentoLiveFeeder 重连与回调测试。"""

from __future__ import annotations

import sys
import types
from unittest.mock import patch

# data_databento_live 在模块顶层 import databento；测试中注入 stub。
if "databento" not in sys.modules:
    _db = types.ModuleType("databento")

    class _Live:
        def __init__(self, *args, **kwargs):
            pass

        def subscribe(self, *args, **kwargs):
            return None

        def close(self):
            return None

        def __iter__(self):
            return iter(())

    _db.Live = _Live
    _db.OHLCVMsg = type("OHLCVMsg", (), {})
    _db.ErrorMsg = type("ErrorMsg", (), {})
    sys.modules["databento"] = _db


def _make_feeder(symbols: list[str] | None = None):
    with patch("newchan.data_databento_live.DATABENTO_API_KEY", "fake-key"):
        from newchan.data_databento_live import DatabentoLiveFeeder
        return DatabentoLiveFeeder(symbols=symbols or ["ES"])


class TestLiveFeederReconnect:
    """流断开后必须自动重连，不能把 _running 永久打成 False。"""

    def test_reconnects_after_stream_error(self):
        feeder = _make_feeder(["ES"])
        calls = {"n": 0}

        def fake_live(*_args, **_kwargs):
            calls["n"] += 1
            if calls["n"] >= 2:
                feeder.stop()
            raise RuntimeError("network blip")

        with patch("newchan.data_databento_live.DATABENTO_API_KEY", "fake-key"):
            with patch("newchan.data_databento_live.db.Live", side_effect=fake_live):
                with patch("newchan.data_databento_live.time.sleep"):
                    feeder._running = True
                    feeder._run()

        assert calls["n"] >= 2
        assert feeder._reconnect_count >= 1
        assert feeder.is_running is False
