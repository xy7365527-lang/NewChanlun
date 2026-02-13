"""PR-B0.1 契约测试 — 域对象 + 适配层

冻结 InstrumentId / ScaleSpec / StreamId / BarV1 / EventEnvelopeV1 的字段规格，
以及 bar_to_v1 / tf_to_stream_id 适配函数的行为。
"""

from __future__ import annotations

import copy
import warnings
from dataclasses import FrozenInstanceError
from datetime import datetime, timezone

import pytest

from newchan.core.instrument import InstrumentId
from newchan.core.scale import ScaleSpec
from newchan.core.stream import StreamId
from newchan.core.bar import BarV1
from newchan.core.envelope import EventEnvelopeV1
from newchan.core.adapters import bar_to_v1, tf_to_stream_id, stream_id_to_tf
from newchan.types import Bar


# ════════════════════════════════════════════════
# InstrumentId
# ════════════════════════════════════════════════


class TestInstrumentId:
    def test_frozen(self):
        inst = InstrumentId(symbol="BZ", inst_type="FUT", exchange="CME")
        with pytest.raises(FrozenInstanceError):
            inst.symbol = "CL"  # type: ignore[misc]

    def test_canonical(self):
        inst = InstrumentId(symbol="BZ", inst_type="FUT", exchange="CME")
        assert inst.canonical == "CME:BZ"

    def test_symbol_must_be_uppercase(self):
        with pytest.raises(ValueError, match="全大写"):
            InstrumentId(symbol="bz", inst_type="FUT", exchange="CME")

    def test_symbol_not_empty(self):
        with pytest.raises(ValueError, match="不能为空"):
            InstrumentId(symbol="", inst_type="FUT", exchange="CME")

    def test_invalid_inst_type(self):
        with pytest.raises(ValueError, match="无效 inst_type"):
            InstrumentId(symbol="BZ", inst_type="OPTION", exchange="CME")

    def test_exchange_not_empty(self):
        with pytest.raises(ValueError, match="不能为空"):
            InstrumentId(symbol="BZ", inst_type="FUT", exchange="")

    def test_all_valid_inst_types(self):
        for t in ("FUT", "STK", "ETF", "SPREAD"):
            inst = InstrumentId(symbol="X", inst_type=t, exchange="TEST")
            assert inst.inst_type == t


# ════════════════════════════════════════════════
# ScaleSpec
# ════════════════════════════════════════════════


class TestScaleSpec:
    def test_frozen(self):
        s = ScaleSpec(base_interval="1min", display_tf="5m")
        with pytest.raises(FrozenInstanceError):
            s.level_id = 1  # type: ignore[misc]

    def test_canonical_default_level(self):
        s = ScaleSpec(base_interval="1min", display_tf="5m")
        assert s.canonical == "1min@5m:L0"

    def test_canonical_with_level(self):
        s = ScaleSpec(base_interval="1min", display_tf="30m", level_id=2)
        assert s.canonical == "1min@30m:L2"

    def test_negative_level_rejected(self):
        with pytest.raises(ValueError, match="不能为负"):
            ScaleSpec(base_interval="1min", display_tf="5m", level_id=-1)

    def test_empty_base_interval_rejected(self):
        with pytest.raises(ValueError, match="不能为空"):
            ScaleSpec(base_interval="", display_tf="5m")

    def test_empty_display_tf_rejected(self):
        with pytest.raises(ValueError, match="不能为空"):
            ScaleSpec(base_interval="1min", display_tf="")


# ════════════════════════════════════════════════
# StreamId
# ════════════════════════════════════════════════


def _make_stream_id(
    symbol="BZ", inst_type="FUT", exchange="CME",
    base_interval="1min", display_tf="5m", level_id=0,
    source="replay",
) -> StreamId:
    return StreamId(
        instrument=InstrumentId(symbol=symbol, inst_type=inst_type, exchange=exchange),
        scale=ScaleSpec(base_interval=base_interval, display_tf=display_tf, level_id=level_id),
        source=source,
    )


class TestStreamId:
    def test_value_format(self):
        sid = _make_stream_id()
        assert sid.value == "CME:BZ/1min@5m:L0/replay"

    def test_value_deterministic(self):
        """同输入 → 同 value。"""
        a = _make_stream_id()
        b = _make_stream_id()
        assert a.value == b.value

    def test_hash_eq(self):
        a = _make_stream_id()
        b = _make_stream_id()
        assert a == b
        assert hash(a) == hash(b)

    def test_different_source_different_value(self):
        a = _make_stream_id(source="replay")
        b = _make_stream_id(source="live")
        assert a.value != b.value
        assert a != b

    def test_dict_key(self):
        sid = _make_stream_id()
        d = {sid: "test"}
        assert d[_make_stream_id()] == "test"

    def test_short_hash_format(self):
        sid = _make_stream_id()
        assert len(sid.short_hash) == 12
        assert all(c in "0123456789abcdef" for c in sid.short_hash)

    def test_short_hash_deterministic(self):
        a = _make_stream_id()
        b = _make_stream_id()
        assert a.short_hash == b.short_hash

    def test_str_is_value(self):
        sid = _make_stream_id()
        assert str(sid) == sid.value

    def test_empty_source_rejected(self):
        with pytest.raises(ValueError, match="不能为空"):
            _make_stream_id(source="")


# ════════════════════════════════════════════════
# BarV1
# ════════════════════════════════════════════════


class TestBarV1:
    def test_frozen(self):
        b = BarV1(bar_time=1000.0, open=1.0, high=2.0, low=0.5, close=1.5)
        with pytest.raises(FrozenInstanceError):
            b.open = 99.0  # type: ignore[misc]

    def test_defaults(self):
        b = BarV1(bar_time=1000.0, open=1.0, high=2.0, low=0.5, close=1.5)
        assert b.volume == 0.0
        assert b.is_closed is True
        assert b.stream_id == ""

    def test_bar_time_must_be_positive(self):
        with pytest.raises(ValueError, match="必须 > 0"):
            BarV1(bar_time=0.0, open=1.0, high=2.0, low=0.5, close=1.5)

    def test_bar_time_negative_rejected(self):
        with pytest.raises(ValueError, match="必须 > 0"):
            BarV1(bar_time=-1.0, open=1.0, high=2.0, low=0.5, close=1.5)

    def test_high_lt_low_warns(self):
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            b = BarV1(bar_time=1000.0, open=1.0, high=0.5, low=2.0, close=1.5)
            assert len(w) == 1
            assert "high < low" in str(w[0].message)
            # 异常数据仍然可以创建
            assert b.high == 0.5
            assert b.low == 2.0

    def test_with_stream_id(self):
        b = BarV1(
            bar_time=1000.0, open=1.0, high=2.0, low=0.5, close=1.5,
            stream_id="CME:BZ/1min@5m:L0/replay",
        )
        assert b.stream_id == "CME:BZ/1min@5m:L0/replay"

    def test_is_closed_false(self):
        b = BarV1(
            bar_time=1000.0, open=1.0, high=2.0, low=0.5, close=1.5,
            is_closed=False,
        )
        assert b.is_closed is False


# ════════════════════════════════════════════════
# EventEnvelopeV1
# ════════════════════════════════════════════════


class TestEventEnvelopeV1:
    def test_frozen(self):
        env = EventEnvelopeV1()
        with pytest.raises(FrozenInstanceError):
            env.event_id = "abc"  # type: ignore[misc]

    def test_defaults(self):
        env = EventEnvelopeV1()
        assert env.schema_version == 1
        assert env.event_id == ""
        assert env.stream_id == ""
        assert env.bar_time == 0.0
        assert env.seq == 0
        assert env.subject_id == ""
        assert env.parents == ()
        assert env.provenance == ""
        assert env.event is None

    def test_parents_is_tuple(self):
        env = EventEnvelopeV1(parents=("abc", "def"))
        assert isinstance(env.parents, tuple)
        assert len(env.parents) == 2

    def test_with_all_fields(self):
        env = EventEnvelopeV1(
            schema_version=1,
            event_id="abcdef0123456789",
            stream_id="CME:BZ/1min@5m:L0/replay",
            bar_time=1000.0,
            seq=42,
            subject_id="stroke:3",
            parents=("parent1", "parent2"),
            provenance="bi_differ:v1",
            event={"mock": True},
        )
        assert env.subject_id == "stroke:3"
        assert env.provenance == "bi_differ:v1"


# ════════════════════════════════════════════════
# 适配层：bar_to_v1
# ════════════════════════════════════════════════


class TestBarToV1:
    def test_basic_conversion(self):
        bar = Bar(
            ts=datetime(2024, 1, 1, 12, 0, 0, tzinfo=timezone.utc),
            open=100.0, high=105.0, low=99.0, close=103.0, volume=1000.0,
        )
        v1 = bar_to_v1(bar, idx=0)
        assert v1.open == 100.0
        assert v1.high == 105.0
        assert v1.low == 99.0
        assert v1.close == 103.0
        assert v1.volume == 1000.0
        assert v1.is_closed is True

    def test_epoch_conversion(self):
        dt = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
        bar = Bar(ts=dt, open=1.0, high=2.0, low=0.5, close=1.5)
        v1 = bar_to_v1(bar)
        assert v1.bar_time == dt.timestamp()

    def test_naive_datetime_treated_as_utc(self):
        naive_dt = datetime(2024, 1, 1, 0, 0, 0)
        utc_dt = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
        bar_naive = Bar(ts=naive_dt, open=1.0, high=2.0, low=0.5, close=1.5)
        bar_utc = Bar(ts=utc_dt, open=1.0, high=2.0, low=0.5, close=1.5)
        assert bar_to_v1(bar_naive).bar_time == bar_to_v1(bar_utc).bar_time

    def test_volume_none_to_zero(self):
        bar = Bar(
            ts=datetime(2024, 1, 1, tzinfo=timezone.utc),
            open=1.0, high=2.0, low=0.5, close=1.5, volume=None,
        )
        v1 = bar_to_v1(bar)
        assert v1.volume == 0.0

    def test_stream_id_passed_through(self):
        bar = Bar(
            ts=datetime(2024, 1, 1, tzinfo=timezone.utc),
            open=1.0, high=2.0, low=0.5, close=1.5,
        )
        v1 = bar_to_v1(bar, stream_id="CME:BZ/1min@5m:L0/replay")
        assert v1.stream_id == "CME:BZ/1min@5m:L0/replay"


# ════════════════════════════════════════════════
# 适配层：tf_to_stream_id
# ════════════════════════════════════════════════


class TestTfToStreamId:
    def test_known_symbol(self):
        sid = tf_to_stream_id("BZ", "5m")
        assert sid.instrument.symbol == "BZ"
        assert sid.instrument.inst_type == "FUT"
        assert sid.instrument.exchange == "CME"
        assert sid.scale.display_tf == "5m"
        assert sid.scale.base_interval == "1min"
        assert sid.source == "replay"
        assert "CME:BZ" in sid.value

    def test_known_stock(self):
        sid = tf_to_stream_id("AAPL", "1m")
        assert sid.instrument.inst_type == "STK"
        assert "AAPL" in sid.value

    def test_unknown_symbol(self):
        sid = tf_to_stream_id("ZZZZZ", "5m")
        assert sid.instrument.exchange == "UNKNOWN"
        assert sid.instrument.symbol == "ZZZZZ"

    def test_deterministic(self):
        a = tf_to_stream_id("BZ", "5m")
        b = tf_to_stream_id("BZ", "5m")
        assert a.value == b.value

    def test_different_tf_different_stream(self):
        a = tf_to_stream_id("BZ", "5m")
        b = tf_to_stream_id("BZ", "30m")
        assert a.value != b.value

    def test_source_parameter(self):
        sid = tf_to_stream_id("BZ", "5m", source="live")
        assert sid.source == "live"
        assert "live" in sid.value

    def test_empty_symbol(self):
        sid = tf_to_stream_id("", "5m")
        assert sid.instrument.symbol == "UNKNOWN"


# ════════════════════════════════════════════════
# 适配层：stream_id_to_tf
# ════════════════════════════════════════════════


class TestStreamIdToTf:
    def test_roundtrip(self):
        sid = tf_to_stream_id("BZ", "5m")
        assert stream_id_to_tf(sid) == "5m"

    def test_explicit(self):
        sid = _make_stream_id(display_tf="30m")
        assert stream_id_to_tf(sid) == "30m"


# ════════════════════════════════════════════════
# WS 消息 stream_id 可选字段
# ════════════════════════════════════════════════


class TestWsStreamIdField:
    def test_ws_bar_stream_id_default(self):
        from newchan.contracts.ws_messages import WsBar
        bar = WsBar(idx=0, ts=1000.0, o=1.0, h=2.0, l=0.5, c=1.5)
        d = bar.model_dump()
        assert d["stream_id"] == ""
        assert d["tf"] == ""  # 向后兼容

    def test_ws_bar_stream_id_set(self):
        from newchan.contracts.ws_messages import WsBar
        bar = WsBar(idx=0, ts=1000.0, o=1.0, h=2.0, l=0.5, c=1.5,
                     stream_id="CME:BZ/1min@5m:L0/replay")
        assert bar.stream_id == "CME:BZ/1min@5m:L0/replay"

    def test_ws_event_stream_id_default(self):
        from newchan.contracts.ws_messages import WsEvent
        ev = WsEvent(
            event_type="stroke_settled", bar_idx=0, bar_ts=1000.0,
            seq=0, payload={},
        )
        d = ev.model_dump()
        assert d["stream_id"] == ""

    def test_ws_event_stream_id_set(self):
        from newchan.contracts.ws_messages import WsEvent
        ev = WsEvent(
            event_type="stroke_settled", bar_idx=0, bar_ts=1000.0,
            seq=0, payload={}, stream_id="CME:BZ/1min@5m:L0/replay",
        )
        assert ev.stream_id == "CME:BZ/1min@5m:L0/replay"
