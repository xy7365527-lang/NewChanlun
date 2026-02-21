"""适配层 — 旧类型 → 新类型桥接

Bar → BarV1、tf → StreamId 等转换函数。
BiEngine.process_bar() 签名不变（仍接收旧 Bar），
适配层在 Orchestrator 层按需使用。
"""

from __future__ import annotations

from datetime import datetime, timezone

from newchan.core.bar import BarV1
from newchan.core.instrument import InstrumentId
from newchan.core.scale import ScaleSpec
from newchan.core.stream import StreamId
from newchan.types import Bar


def _dt_to_epoch(dt: datetime) -> float:
    """datetime → epoch 秒。naive datetime 视为 UTC。"""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()


def bar_to_v1(bar: Bar, idx: int = 0, stream_id: str = "") -> BarV1:
    """旧 Bar → BarV1。

    Parameters
    ----------
    bar : Bar
        旧 Bar 实例（ts 为 datetime）。
    idx : int
        bar 在序列中的位置索引（用于调试，BarV1 不存储）。
    stream_id : str
        所属流标识。
    """
    return BarV1(
        bar_time=_dt_to_epoch(bar.ts),
        open=bar.open,
        high=bar.high,
        low=bar.low,
        close=bar.close,
        volume=bar.volume if bar.volume is not None else 0.0,
        is_closed=True,
        stream_id=stream_id,
    )


def tf_to_stream_id(
    symbol: str,
    tf: str,
    interval: str = "1min",
    source: str = "replay",
) -> StreamId:
    """从 (symbol, tf, interval) 构造 StreamId。

    查找 SYMBOL_CATALOG 获取 inst_type/exchange，
    未知品种使用默认值。
    """
    from newchan.core.symbol_catalog import SYMBOL_CATALOG

    sym = symbol.upper() if symbol else ""
    inst_type = "STK"
    exchange = "UNKNOWN"

    if sym:
        for item in SYMBOL_CATALOG:
            if item["symbol"] == sym:
                inst_type = item["type"]
                exchange = item["exchange"]
                break

    instrument = InstrumentId(
        symbol=sym or "UNKNOWN",
        inst_type=inst_type,
        exchange=exchange,
    )
    scale = ScaleSpec(
        base_interval=interval,
        display_tf=tf,
        level_id=0,
    )
    return StreamId(
        instrument=instrument,
        scale=scale,
        source=source,
    )


def stream_id_to_tf(stream_id: StreamId) -> str:
    """StreamId → tf 字符串（向后兼容）。"""
    return stream_id.scale.display_tf
