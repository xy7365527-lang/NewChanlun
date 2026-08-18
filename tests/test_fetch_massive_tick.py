"""#1040 Massive tick 拉取管线测试锁——离线合成，不碰网络、不读 key。

锁三类行为：
1. 落盘/schema：13 字段列序 + ticker 补 + trf 两列 Int64 + Parquet(ZSTD) 原子落盘；
2. 分页/退避：next_url 游标（绝对/相对）+ 429/5xx 指数退避 + 4xx 致命；
3. 断点续拉：已存在且行数>0 的分区跳过；失败日显式清单（failed_days）。

传输 + 落盘政策直接 import 政策模块 `massive_fetch`（#1068 / #1063 裁定：测试不碰 CLI 面）；
起点解析（resolve_start_date/get_list_date/probe_first_nonempty_day）留脚本层，单独 import
`fetch_massive_tick` 锁纯逻辑（不测 main/plan，CLI 行为零差走手工/快照对照）。

不依赖真实 Massive 数据（#1040 未落盘、沙盒无 MASSIVE_API_KEY，见 issue 评论）。
"""

from __future__ import annotations

import datetime as dt

import pandas as pd
import pytest

import fetch_massive_tick as cli  # 脚本层：起点解析（不碰 CLI main/plan 面）
import massive_fetch as fmt     # 政策模块：传输 + 落盘政策


def _row(i: int, **over) -> dict:
    """一行原样 JSON 形态（13 字段，correction 可选）。"""
    r = {
        "id": f"t{i}",
        "conditions": [0],
        "exchange": 1,
        "participant_timestamp": 1_700_000_000_000_000_000 + i,
        "sip_timestamp": 1_700_000_000_000_000_000 + i,
        "trf_timestamp": None,
        "price": 100.0 + i,
        "sequence_number": i,
        "size": 100,
        "tape": 1,
        "trf_id": None,
        "decimal_size": 100.0,
    }
    r.update(over)
    return r


# ---------------------------------------------------------------------------
# 日期 / 分区 / 断点续拉
# ---------------------------------------------------------------------------


def test_iter_dates_skips_weekends():
    days = list(fmt.iter_dates(dt.date(2003, 9, 10), dt.date(2003, 9, 16)))
    assert [d.isoformat() for d in days] == [
        "2003-09-10", "2003-09-11", "2003-09-12", "2003-09-15", "2003-09-16",
    ]
    assert all(d.weekday() < 5 for d in days)
    # 不跳周末
    all_days = list(fmt.iter_dates(dt.date(2003, 9, 10), dt.date(2003, 9, 16), skip_weekends=False))
    assert len(all_days) == 7


def test_partition_path_format():
    p = fmt.partition_path("analysis/data_cache/massive_tick", "COST", "2024-01-02")
    assert str(p) == "analysis/data_cache/massive_tick/COST/dt=2024-01-02/trades.parquet"


def test_partition_done_true_and_empty(tmp_path):
    done = fmt.write_partition(tmp_path, "COST", "2024-01-02", [_row(1)])
    assert fmt.partition_done(done) is True
    # 空文件 / 损坏文件 → 未完成
    empty = tmp_path / "AAPL" / "dt=2024-01-02" / "trades.parquet"
    empty.parent.mkdir(parents=True)
    empty.write_bytes(b"not a parquet")
    assert fmt.partition_done(empty) is False
    missing = tmp_path / "MSFT" / "dt=2024-01-02" / "trades.parquet"
    assert fmt.partition_done(missing) is False


# ---------------------------------------------------------------------------
# schema / 落盘
# ---------------------------------------------------------------------------


def test_normalize_frame_fills_ticker_and_fixes_order():
    rows = [_row(1), _row(2, trf_id=201, trf_timestamp=1_700_000_000_000_000_000)]
    df = fmt.normalize_frame(rows, "COST")
    assert list(df.columns) == fmt.CANONICAL_COLUMNS, list(df.columns)
    assert (df["ticker"] == "COST").all()
    assert df["trf_id"].dtype.name == "Int64"
    assert df["trf_timestamp"].dtype.name == "Int64"
    assert df["trf_id"].tolist()[1] == 201
    assert pd.isna(df["trf_id"].tolist()[0])


def test_normalize_frame_preserves_correction_and_missing_column():
    # correction 存在时追加到 decimal_size 之后
    df = fmt.normalize_frame([_row(1, correction=0)], "AAPL")
    assert df.columns[-1] == "correction"
    # 缺 canonical 列（例如 API 未返回 decimal_size）→ 补 None
    rows = [_row(1)]
    del rows[0]["decimal_size"]
    df2 = fmt.normalize_frame(rows, "AAPL")
    assert "decimal_size" in df2.columns
    assert df2["decimal_size"].isna().all()


def test_write_partition_roundtrip_and_atomic(tmp_path):
    rows = [
        _row(1),
        _row(2, conditions=[0, 37], exchange=4, trf_id=201,
             trf_timestamp=1_700_000_000_000_000_002, correction=0),
    ]
    out = fmt.write_partition(tmp_path, "COST", "2024-01-02", rows)
    assert out.exists()
    # 无临时文件残留
    assert list(out.parent.glob("*.tmp.*")) == []
    df = pd.read_parquet(out)
    assert list(df.columns) == fmt.CANONICAL_COLUMNS + ["correction"]
    assert len(df) == 2
    assert (df["ticker"] == "COST").all()
    assert df["trf_id"].dtype.name == "Int64"
    assert df.loc[1, "trf_id"] == 201
    assert list(df.loc[1, "conditions"]) == [0, 37]  # parquet 读回为 numpy 数组


def test_write_partition_empty_returns_none(tmp_path):
    assert fmt.write_partition(tmp_path, "COST", "2024-01-02", []) is None


# ---------------------------------------------------------------------------
# 分页 / 退避
# ---------------------------------------------------------------------------

class _Resp:
    def __init__(self, status_code, payload=None, headers=None, text=""):
        self.status_code = status_code
        self._payload = payload
        self.headers = headers or {}
        self.text = text

    def json(self):
        return self._payload


class _Sess:
    def __init__(self, seq):
        self.seq = list(seq)
        self.calls = 0

    def get(self, url, **kw):
        self.calls += 1
        r = self.seq.pop(0)
        if isinstance(r, Exception):
            raise r
        return r


def test_fetch_day_pagination_absolute_next_url(monkeypatch):
    calls = []

    def fake_http(session, url, *, params=None, headers=None, **kw):
        calls.append((url, params))
        if params is not None:
            return {
                "results": [_row(1), _row(2)],
                "next_url": "https://api.massive.com/v3/trades/AAPL?cursor=xyz",
            }
        return {"results": [_row(3)], "next_url": None}

    monkeypatch.setattr(fmt, "http_get_json", fake_http)
    rows = fmt.fetch_day(object(), "AAPL", "2024-01-02", headers={})
    assert [r["id"] for r in rows] == ["t1", "t2", "t3"]
    assert calls[0][1] == {"timestamp": "2024-01-02", "limit": 50_000}
    assert calls[1] == ("https://api.massive.com/v3/trades/AAPL?cursor=xyz", None)


def test_fetch_day_pagination_relative_next_url(monkeypatch):
    def fake_http(session, url, *, params=None, headers=None, **kw):
        if params is not None:
            return {"results": [_row(1)], "next_url": "/v3/trades/AAPL?cursor=rel"}
        assert url == "https://api.massive.com/v3/trades/AAPL?cursor=rel"
        return {"results": [], "next_url": None}

    monkeypatch.setattr(fmt, "http_get_json", fake_http)
    rows = fmt.fetch_day(object(), "AAPL", "2024-01-02", headers={})
    assert [r["id"] for r in rows] == ["t1"]


def test_http_get_json_retries_429_then_ok():
    slept = []
    s = _Sess([_Resp(429), _Resp(429, headers={"Retry-After": "0"}), _Resp(200, {"results": []})])
    out = fmt.http_get_json(s, "http://x", headers={}, sleep=slept.append)
    assert out == {"results": []}
    assert s.calls == 3
    assert len(slept) == 2


def test_http_get_json_retries_5xx():
    s = _Sess([_Resp(500), _Resp(200, {"results": []})])
    out = fmt.http_get_json(s, "http://x", headers={}, sleep=lambda _: None)
    assert out == {"results": []}
    assert s.calls == 2


def test_http_get_json_fatal_4xx_no_retry():
    s = _Sess([_Resp(403, text="denied")])
    with pytest.raises(RuntimeError, match="403"):
        fmt.http_get_json(s, "http://x", headers={}, sleep=lambda _: None)
    assert s.calls == 1


# ---------------------------------------------------------------------------
# 起点解析（脚本层纯逻辑） / 全标的主循环
# ---------------------------------------------------------------------------


def test_resolve_start_date_non_late_returns_floor():
    floor = dt.date(2003, 9, 10)
    assert cli.resolve_start_date(object(), "AAPL", {}, floor) == floor


def test_resolve_start_date_late_uses_list_date(monkeypatch):
    floor = dt.date(2003, 9, 10)
    monkeypatch.setattr(cli, "get_list_date", lambda *a, **kw: dt.date(2004, 8, 19))
    assert cli.resolve_start_date(object(), "GOOGL", {}, floor) == dt.date(2004, 8, 19)
    # list_date 早于 floor → 用 floor
    monkeypatch.setattr(cli, "get_list_date", lambda *a, **kw: dt.date(1999, 1, 1))
    assert cli.resolve_start_date(object(), "TSLA", {}, floor) == floor


def test_resolve_start_date_late_falls_back_to_probe(monkeypatch):
    floor = dt.date(2003, 9, 10)
    monkeypatch.setattr(cli, "get_list_date", lambda *a, **kw: None)
    monkeypatch.setattr(cli, "probe_first_nonempty_day", lambda *a, **kw: dt.date(2010, 6, 29))
    assert cli.resolve_start_date(object(), "TSLA", {}, floor) == dt.date(2010, 6, 29)


def test_fetch_symbol_resume_skips_existing(tmp_path, monkeypatch):
    fmt.write_partition(tmp_path, "COST", "2024-01-02", [_row(1)])
    fetched = []

    def fake_fetch_day(session, ticker, date_str, **kw):
        fetched.append(date_str)
        return [_row(2)]

    monkeypatch.setattr(fmt, "fetch_day", fake_fetch_day)
    stats = fmt.fetch_symbol(
        None, "COST", root=tmp_path, headers={},
        start=dt.date(2024, 1, 2), end=dt.date(2024, 1, 3),
        skip_weekends=True, limit=50_000, max_retries=6, timeout=30.0,
        backoff_base=1.0, max_backoff=60.0,
    )
    assert fetched == ["2024-01-03"]  # 01-02 已有分区 → 跳过
    assert stats["skipped"] == 1
    assert stats["fetched"] == 1


def test_fetch_symbol_failed_days_list(tmp_path, monkeypatch):
    """failed_days 显式清单锁：成功/失败日混合夹具，失败日记 (date_str, error)，成功日不记。"""
    def fake_fetch_day(session, ticker, date_str, **kw):
        if date_str == "2024-01-03":
            raise RuntimeError("boom")
        if date_str == "2024-01-05":
            raise RuntimeError("timeout")
        return [_row(2)]

    monkeypatch.setattr(fmt, "fetch_day", fake_fetch_day)
    stats = fmt.fetch_symbol(
        None, "COST", root=tmp_path, headers={},
        start=dt.date(2024, 1, 2), end=dt.date(2024, 1, 5),
        skip_weekends=True, limit=50_000, max_retries=6, timeout=30.0,
        backoff_base=1.0, max_backoff=60.0,
    )
    assert stats["fetched"] == 2          # 01-02 / 01-04 成功
    assert stats["failed"] == 2           # 01-03 / 01-05 失败
    assert stats["failed_days"] == [
        ("2024-01-03", "boom"),
        ("2024-01-05", "timeout"),
    ]
    # 成功落盘的分区仍可读
    assert fmt.partition_done(fmt.partition_path(tmp_path, "COST", "2024-01-02"))
