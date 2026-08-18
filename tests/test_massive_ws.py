"""#1044 massive_ws 离线测试锁：T/Q 映射、分区转正、回补分页、auth/订阅消息。"""
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))
from newchan.massive_ws import (  # noqa: E402
    QUOTES_COLUMNS, TRADES_COLUMNS, DayWriter, map_quote, map_trade,
    ns_date, rest_fetch_day, _schema_for,
)

T_EVENT = {
    "ev": "T", "sym": "AAPL", "x": 4, "i": "12345", "z": 3, "p": 114.125,
    "s": 100, "ds": "100.5", "c": [0, 12], "t": 1536036818784,
    "pt": 1536036818763, "q": 3681328, "trfi": 202, "trft": 1536036818700,
}
Q_EVENT = {
    "ev": "Q", "sym": "MSFT", "bx": 4, "bp": 114.125, "bs": 100,
    "ax": 7, "ap": 114.128, "as": 160, "c": 0, "i": [604],
    "t": 1536036818784, "q": 50385480, "z": 3,
}


def test_map_trade_ms_to_ns_and_null_correction():
    row = map_trade(T_EVENT)
    assert row["sip_timestamp"] == 1536036818784 * 1_000_000
    assert row["participant_timestamp"] == 1536036818763 * 1_000_000
    assert row["trf_timestamp"] == 1536036818700 * 1_000_000
    assert row["correction"] is None          # WS 缺列置 null
    assert row["ticker"] == "AAPL"
    assert row["id"] == "12345"
    assert set(row.keys()) == set(TRADES_COLUMNS)


def test_map_quote_round_lot_and_null_pt():
    row = map_quote(Q_EVENT)
    assert row["bid_size"] == 100 * 100       # round lots ×100
    assert row["ask_size"] == 160 * 100
    assert row["participant_timestamp"] is None  # Q 无 pt
    assert row["conditions"] == [0]
    assert row["indicators"] == [604]
    assert set(row.keys()) == set(QUOTES_COLUMNS)


def test_ns_date_utc():
    # 1536036818784 ms = 2018-09-04 UTC
    assert ns_date(1536036818784 * 1_000_000) == "2018-09-04"


def test_daywriter_rollover_finalize(tmp_path):
    w = DayWriter(ticker="AAPL", root=tmp_path, schemas=("T",))
    w.write(map_trade({**T_EVENT, "t": 1536036818784}), "T")           # 2018-09-04
    w.write(map_trade({**T_EVENT, "t": 1536036818784 + 5}), "T")
    w.write(map_trade({**T_EVENT, "t": 1536036818784 + 86_400_000}), "T")  # 次日
    w.close()
    p1 = tmp_path / "AAPL" / "dt=2018-09-04" / "trades.parquet"
    p2 = tmp_path / "AAPL" / "dt=2018-09-05" / "trades.parquet"
    assert p1.exists() and p2.exists()
    assert not (tmp_path / "AAPL" / "dt=2018-09-04" / "trades.parquet.tmp").exists()
    import pyarrow.parquet as pq
    assert pq.read_table(p1).num_rows == 2
    assert pq.read_table(p2).num_rows == 1


def test_backfill_pagination(monkeypatch):
    pages = [
        {"results": [{"id": "1", "price": 1.0}], "next_url": "https://api.massive.com/v3/trades/AAPL?page=2"},
        {"results": [{"id": "2", "price": 2.0}], "next_url": None},
    ]
    class FakeResp:
        def __init__(self, payload):
            self._p = payload
        def raise_for_status(self):
            pass
        def json(self):
            return self._p
    calls = []
    def fake_get(url, **kw):
        calls.append(url)
        return FakeResp(pages.pop(0))
    monkeypatch.setattr("newchan.massive_ws.requests.get", fake_get)
    rows = rest_fetch_day("AAPL", "2026-08-14", "k")
    assert len(rows) == 2 and rows[1]["id"] == "2"
    assert len(calls) == 2 and "timestamp=2026-08-14" in calls[0]
    assert "page=2" in calls[1]


def test_schema_columns():
    t = _schema_for("T")
    q = _schema_for("Q")
    assert [f.name for f in t] == TRADES_COLUMNS
    assert [f.name for f in q] == QUOTES_COLUMNS
