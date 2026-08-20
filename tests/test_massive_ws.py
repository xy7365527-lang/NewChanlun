"""#1044 massive_ws 离线测试锁：T/Q 映射、分区转正、回补分页、auth/订阅消息。"""
import json
import sys
from pathlib import Path

import pyarrow as pa
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
    assert t.field("price").type == pa.float64()
    assert q.field("ask_price").type == pa.float64()
    assert q.field("bid_price").type == pa.float64()


def test_daywriter_preserves_fractional_trade_price(tmp_path):
    """ADR 0023 tick 正典：成交价必须原样落盘。int64 schema 会把 114.125 静默截成 114。"""
    import pyarrow.parquet as pq

    w = DayWriter(ticker="AAPL", root=tmp_path, schemas=("T",))
    w.write(map_trade(T_EVENT), "T")
    w.close()
    table = pq.read_table(tmp_path / "AAPL" / "dt=2018-09-04" / "trades.parquet")
    prices = table.column("price").to_pylist()
    assert prices == [114.125]


def test_daywriter_preserves_fractional_quote_prices(tmp_path):
    import pyarrow.parquet as pq

    w = DayWriter(ticker="MSFT", root=tmp_path, schemas=("Q",))
    w.write(map_quote(Q_EVENT), "Q")
    w.close()
    table = pq.read_table(tmp_path / "MSFT" / "dt=2018-09-04" / "quotes.parquet")
    assert table.column("bid_price").to_pylist() == [114.125]
    assert table.column("ask_price").to_pylist() == [114.128]


def test_rest_overlay_not_clobbered_by_later_ws(tmp_path):
    """断线 REST 全量覆盖后，后续 WS .tmp 转正不得把回补文件整个盖掉。"""
    import pyarrow.parquet as pq

    w = DayWriter(ticker="AAPL", root=tmp_path, schemas=("T",))
    live_before = map_trade({**T_EVENT, "i": "ws-1", "p": 150.25, "t": 1536036818784})
    w.write(live_before, "T")

    rest_rows = [
        map_trade({**T_EVENT, "i": "rest-1", "p": 150.25, "t": 1536036818784}),
        map_trade({**T_EVENT, "i": "rest-2", "p": 150.50, "t": 1536036818784 + 1}),
    ]
    w.overlay_rest(rest_rows, "2018-09-04", "T")

    live_after = map_trade({**T_EVENT, "i": "ws-2", "p": 150.75, "t": 1536036818784 + 2})
    w.write(live_after, "T")
    w.close()

    table = pq.read_table(tmp_path / "AAPL" / "dt=2018-09-04" / "trades.parquet")
    ids = table.column("id").to_pylist()
    prices = table.column("price").to_pylist()
    assert "rest-1" in ids and "rest-2" in ids
    assert "ws-1" not in ids  # 回补前 WS 被当日 REST 全量覆盖取代
    assert "ws-2" in ids
    assert prices == [150.25, 150.50, 150.75]


def test_rest_overlay_survives_close_without_later_ws(tmp_path):
    """回补后若当日不再有 WS 成交，close 不得把 REST 分区删掉或换成空 .tmp。"""
    import pyarrow.parquet as pq

    w = DayWriter(ticker="AAPL", root=tmp_path, schemas=("T",))
    w.write(map_trade({**T_EVENT, "i": "ws-1", "p": 150.25}), "T")
    rest_rows = [map_trade({**T_EVENT, "i": "rest-1", "p": 150.25})]
    w.overlay_rest(rest_rows, "2018-09-04", "T")
    w.close()
    table = pq.read_table(tmp_path / "AAPL" / "dt=2018-09-04" / "trades.parquet")
    assert table.column("id").to_pylist() == ["rest-1"]
    assert table.column("price").to_pylist() == [150.25]
