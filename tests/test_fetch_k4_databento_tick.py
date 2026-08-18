"""#1089 K4 Databento tick 拉取管线测试锁——离线合成，不碰网络、不读 key。

锁三类行为：
1. 落盘/分区：PARQUET 直出（DBNStore.to_parquet 委托）+ 原子落盘 + 空分区不落盘 + 续拉判据；
2. 退避：429/5xx/超时/连接错误指数退避（读 Retry-After）+ 422 数据越界判空 + 其余 4xx 致命；
3. 断点续拉：已存在且行数>0 的分区跳过；失败日显式清单（failed_days）；
4. 六边标的清单：三交易所地板对齐 ADR 0023 + 三 dataset 全覆盖。

传输 + 落盘政策 import 政策模块 `k4_databento_fetch`；六边标的清单/地板在脚本层
`fetch_k4_databento_tick`（只锁纯逻辑，不测 main/plan——与 #1040 Massive 同口径）。

不依赖真实 Databento 数据（沙盒无 DATABENTO_API_KEY、无外网，见 issue 评论）。
"""

from __future__ import annotations

import datetime as dt

import pandas as pd
import pytest
import requests

import fetch_k4_databento_tick as cli
import k4_databento_fetch as k
from databento.common.error import BentoClientError, BentoError, BentoServerError


class _FakeStore:
    """伪造 DBNStore：to_parquet 直接写 pyarrow Parquet（rows=0 时什么都不写，模拟空流）。"""

    def __init__(self, rows: int = 2):
        self.rows = rows
        self.calls: list[tuple[str, dict]] = []

    def to_parquet(self, path, **kw):
        self.calls.append((str(path), kw))
        if self.rows <= 0:
            return  # 空 DBN：to_parquet 不会创建文件（writer 不初始化）
        pd.DataFrame({
            "ts_event": pd.to_datetime(
                [1_700_000_000_000_000_000 + i for i in range(self.rows)], utc=True),
            "price": [100.0 + i for i in range(self.rows)],
            "size": [10] * self.rows,
        }).to_parquet(path, engine="pyarrow", compression="zstd", index=False)


class _FakeTimeseries:
    def __init__(self, seq):
        self.seq = list(seq)
        self.calls = 0

    def get_range(self, **kw):
        self.calls += 1
        r = self.seq.pop(0)
        if isinstance(r, Exception):
            raise r
        return r


class _FakeClient:
    def __init__(self, seq):
        self.timeseries = _FakeTimeseries(seq)


_DAY_START = dt.datetime(2024, 1, 2, tzinfo=dt.timezone.utc)
_DAY_END = _DAY_START + dt.timedelta(days=1)


def _get_day(client, *, seq_ok="STORE", max_retries=6, **kw):
    return k.get_day_dbn(
        client, dataset="GLBX.MDP3", symbol="ES.v.0", stype_in="continuous",
        start=_DAY_START, end=_DAY_END, max_retries=max_retries, **kw,
    )


# ---------------------------------------------------------------------------
# 日期 / 分区 / 断点续拉
# ---------------------------------------------------------------------------


def test_iter_days_no_weekend_skip():
    days = [d.isoformat() for d in k.iter_days(dt.date(2024, 1, 5), dt.date(2024, 1, 8))]
    # 期货周日晚开盘，周六才整日休市 → 不跳任何自然日
    assert days == ["2024-01-05", "2024-01-06", "2024-01-07", "2024-01-08"]


def test_partition_path_format():
    p = k.partition_path("analysis/data_cache/k4_databento_tick", "ES", "2024-01-02")
    assert str(p) == "analysis/data_cache/k4_databento_tick/ES/dt=2024-01-02/trades.parquet"


def test_partition_done_true_empty_corrupt(tmp_path):
    done = k.write_partition(tmp_path, "ES", "2024-01-02", _FakeStore(rows=2))
    assert k.partition_done(done) is True
    # 空文件 / 损坏文件 → 未完成
    empty = tmp_path / "GC" / "dt=2024-01-02" / "trades.parquet"
    empty.parent.mkdir(parents=True)
    empty.write_bytes(b"not a parquet")
    assert k.partition_done(empty) is False
    missing = tmp_path / "CL" / "dt=2024-01-02" / "trades.parquet"
    assert k.partition_done(missing) is False


# ---------------------------------------------------------------------------
# PARQUET 直出 / 原子落盘
# ---------------------------------------------------------------------------


def test_write_partition_atomic(tmp_path):
    store = _FakeStore(rows=3)
    out = k.write_partition(tmp_path, "ES", "2024-01-02", store)
    assert out.exists()
    assert list(out.parent.glob("*.tmp.*")) == []
    assert k.partition_done(out) is True
    # 委托 DBNStore.to_parquet：pretty_ts / map_symbols 透传（ns 全精度 + symbol 列口径）
    assert store.calls[0][1] == {"pretty_ts": True, "map_symbols": True}
    df = pd.read_parquet(out)
    assert len(df) == 3
    # ts_event 读回 timestamp[ns]（历史段 ns 全精度）
    assert str(df["ts_event"].dtype) == "datetime64[ns, UTC]"


def test_write_partition_empty_returns_none(tmp_path):
    assert k.write_partition(tmp_path, "ES", "2024-01-02", _FakeStore(rows=0)) is None
    assert not (tmp_path / "ES" / "dt=2024-01-02" / "trades.parquet").exists()


# ---------------------------------------------------------------------------
# 退避：429 / 5xx / 超时 / 422 越界 / 4xx 致命
# ---------------------------------------------------------------------------


def test_get_day_dbn_retries_429_then_ok():
    slept = []
    client = _FakeClient([
        BentoClientError(429),
        BentoClientError(429, headers={"Retry-After": "0"}),
        "STORE",
    ])
    out = _get_day(client, sleep=slept.append)
    assert out == "STORE"
    assert client.timeseries.calls == 3
    assert len(slept) == 2


def test_get_day_dbn_retries_5xx():
    client = _FakeClient([BentoServerError(500), "STORE"])
    out = _get_day(client, sleep=lambda _: None)
    assert out == "STORE"
    assert client.timeseries.calls == 2


def test_get_day_dbn_retries_timeout_then_ok():
    client = _FakeClient([requests.exceptions.Timeout(), requests.exceptions.ConnectionError(), "STORE"])
    out = _get_day(client, sleep=lambda _: None)
    assert out == "STORE"
    assert client.timeseries.calls == 3


def test_get_day_dbn_retries_stream_error_then_ok():
    client = _FakeClient([BentoError("Error streaming response"), "STORE"])
    out = _get_day(client, sleep=lambda _: None)
    assert out == "STORE"
    assert client.timeseries.calls == 2


def test_get_day_dbn_422_range_returns_none():
    for msg in ("data_start_before_available", "unavailable_range"):
        client = _FakeClient([BentoClientError(422, message=msg)])
        assert _get_day(client, sleep=lambda _: None) is None
        assert client.timeseries.calls == 1


def test_get_day_dbn_fatal_4xx_no_retry():
    client = _FakeClient([BentoClientError(404, message="symbol_not_found")])
    with pytest.raises(BentoClientError):
        _get_day(client, sleep=lambda _: None)
    assert client.timeseries.calls == 1


def test_get_day_dbn_retry_exhaustion():
    client = _FakeClient([BentoServerError(500)] * 3)
    with pytest.raises(RuntimeError, match="重试耗尽"):
        _get_day(client, max_retries=3, sleep=lambda _: None)
    assert client.timeseries.calls == 3


# ---------------------------------------------------------------------------
# 断点续拉（fetch_symbol）
# ---------------------------------------------------------------------------


def test_fetch_symbol_resume_skips_existing(tmp_path, monkeypatch):
    k.write_partition(tmp_path, "ES", "2024-01-02", _FakeStore(rows=1))
    fetched = []

    def fake_get_day(client, *, dataset, symbol, stype_in, start, end, **kw):
        fetched.append(start.strftime("%Y-%m-%d"))
        return _FakeStore(rows=2)

    monkeypatch.setattr(k, "get_day_dbn", fake_get_day)
    stats = k.fetch_symbol(
        None, "ES", dataset="GLBX.MDP3", db_symbol="ES.v.0", stype_in="continuous",
        root=tmp_path, start=dt.date(2024, 1, 2), end=dt.date(2024, 1, 3),
        max_retries=6, backoff_base=1.0, max_backoff=60.0, sleep=lambda _: None,
    )
    assert fetched == ["2024-01-03"]  # 01-02 已有分区 → 跳过
    assert stats["skipped"] == 1
    assert stats["fetched"] == 1
    assert stats["failed"] == 0


def test_fetch_symbol_failed_days_list(tmp_path, monkeypatch):
    """failed_days 显式清单锁：成功/失败日混合夹具，失败日记 (date_str, error)，成功日不记。"""

    def fake_get_day(client, *, dataset, symbol, stype_in, start, end, **kw):
        d = start.strftime("%Y-%m-%d")
        if d == "2024-01-03":
            raise RuntimeError("boom")
        if d == "2024-01-05":
            raise requests.exceptions.Timeout()
        return _FakeStore(rows=2)

    monkeypatch.setattr(k, "get_day_dbn", fake_get_day)
    stats = k.fetch_symbol(
        None, "ES", dataset="GLBX.MDP3", db_symbol="ES.v.0", stype_in="continuous",
        root=tmp_path, start=dt.date(2024, 1, 2), end=dt.date(2024, 1, 5),
        max_retries=6, backoff_base=1.0, max_backoff=60.0, sleep=lambda _: None,
    )
    assert stats["fetched"] == 2          # 01-02 / 01-04 成功
    assert stats["failed"] == 2           # 01-03 / 01-05 失败
    # 失败日显式清单：键 = (date_str, error_str)，顺序按日期；01-05 为超时（RequestException）
    assert [d for d, _ in stats["failed_days"]] == ["2024-01-03", "2024-01-05"]
    assert stats["failed_days"][0][1] == "boom"
    assert isinstance(stats["failed_days"][1][1], str)
    # 成功落盘的分区仍可读
    assert k.partition_done(k.partition_path(tmp_path, "ES", "2024-01-02"))


# ---------------------------------------------------------------------------
# 六边标的清单 + 地板（脚本层纯逻辑）
# ---------------------------------------------------------------------------


def test_k4_symbols_floors_match_adr_0023():
    """三交易所地板对齐 ADR 0023 §一：CME 2010-06-06 / ICE 2018-12-23 / Eurex 2025-03-10。"""
    floors = {(s.dataset, s.floor) for s in cli.K4_SYMBOLS}
    assert ("GLBX.MDP3", "2010-06-06") in floors
    assert ("IFEU.IMPACT", "2018-12-23") in floors
    assert ("IFUS.IMPACT", "2018-12-23") in floors
    assert ("XEUR.EOBI", "2025-03-10") in floors
    # 三交易所 dataset 全覆盖，且无地板早于各自 dataset 地板
    datasets = {s.dataset for s in cli.K4_SYMBOLS}
    assert {"GLBX.MDP3", "IFEU.IMPACT", "IFUS.IMPACT", "XEUR.EOBI"} <= datasets


def test_k4_symbols_cover_four_vertices_and_fold_channels():
    seats = {s.seat for s in cli.K4_SYMBOLS}
    for need in ("P 顶点", "M 顶点", "C 顶点（ICE）", "Au 折叠通道 C↔M", "Oil 折叠通道 C→P / C 顶点"):
        assert any(need in s for s in seats), need


def test_resolve_start_clamps_to_floor():
    fesx = cli.symbol_by_key("FESX")
    assert cli.resolve_start(fesx, None) == dt.date(2025, 3, 10)
    assert cli.resolve_start(fesx, dt.date(2024, 1, 1)) == dt.date(2025, 3, 10)  # 早于地板 → 钳到地板
    assert cli.resolve_start(fesx, dt.date(2025, 6, 1)) == dt.date(2025, 6, 1)


def test_symbol_by_key_case_insensitive():
    assert cli.symbol_by_key("es").symbol == "ES"
    assert cli.symbol_by_key("6e").symbol == "6E"
    assert cli.symbol_by_key("nope") is None
