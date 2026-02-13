"""Databento Live 实时数据 — 期货 ohlcv-1m 流式订阅

在后台线程运行 Databento Live 客户端，收到 1min bar 后增量追加到缓存。
与 Historical API 使用同一数据源、同一 symbol、同一展期规则，保证数据一致性。

OHLCVMsg 字段说明：
- open/high/low/close: int64 fixed-point, 1 unit = 1e-9（用 pretty_* 属性获取 float）
- volume: int, 直接使用
- ts_event: int, 纳秒时间戳（用 pretty_ts_event 获取 datetime）
- instrument_id: int, 通过 client.symbology_map 映射回 symbol
"""

from __future__ import annotations

import logging
import threading

import databento as db
import pandas as pd

from newchan.cache import append_df
from newchan.config import DATABENTO_API_KEY
from newchan.data_databento import _FUTURES_MAP

logger = logging.getLogger(__name__)

# 默认订阅的期货品种（CME Globex）
DEFAULT_LIVE_SYMBOLS: list[str] = ["BZ", "CL", "GC", "ES", "NQ", "SI"]


class DatabentoLiveFeeder:
    """Databento Live ohlcv-1m 实时推送器。

    用法：
        feeder = DatabentoLiveFeeder(symbols=["BZ"])
        feeder.start()
        # ... 服务运行中 ...
        feeder.stop()
    """

    def __init__(
        self,
        symbols: list[str] | None = None,
        dataset: str = "GLBX.MDP3",
    ):
        self._symbols = [s.upper() for s in (symbols or DEFAULT_LIVE_SYMBOLS)]
        self._dataset = dataset
        self._client: db.Live | None = None
        self._thread: threading.Thread | None = None
        self._running = False
        self._bar_count = 0
        self._last_error: str | None = None

        # db_symbol (e.g. "BZ.c.0") → our_symbol (e.g. "BZ")
        self._db_to_our: dict[str, str] = {}
        # our_symbol → cache_name
        self._cache_map: dict[str, str] = {}
        # db_symbols 列表（用于 subscribe）
        self._db_symbols: list[str] = []

        for sym in self._symbols:
            db_sym = _FUTURES_MAP.get(sym, f"{sym}.c.0")
            self._db_to_our[db_sym] = sym
            self._cache_map[sym] = f"{sym}_1min_raw"
            self._db_symbols.append(db_sym)

    @property
    def is_running(self) -> bool:
        return self._running

    @property
    def bar_count(self) -> int:
        return self._bar_count

    @property
    def last_error(self) -> str | None:
        return self._last_error

    def status(self) -> dict:
        """返回当前状态。"""
        return {
            "running": self._running,
            "symbols": self._symbols,
            "bar_count": self._bar_count,
            "last_error": self._last_error,
        }

    def start(self) -> None:
        """启动后台线程订阅实时数据。"""
        if self._running:
            logger.warning("Live feeder 已在运行")
            return
        if not DATABENTO_API_KEY:
            self._last_error = "DATABENTO_API_KEY 未设置"
            logger.error(self._last_error)
            return

        self._running = True
        self._last_error = None
        self._thread = threading.Thread(
            target=self._run, daemon=True, name="databento-live",
        )
        self._thread.start()
        logger.info("Databento Live 已启动: %s", self._symbols)

    def stop(self) -> None:
        """停止实时订阅。"""
        self._running = False
        if self._client is not None:
            try:
                self._client.close()
            except Exception:
                pass
            self._client = None
        logger.info("Databento Live 已停止")

    def _run(self) -> None:
        """后台线程主循环。"""
        try:
            self._client = db.Live(key=DATABENTO_API_KEY)

            logger.info(
                "Live subscribe: dataset=%s symbols=%s",
                self._dataset, self._db_symbols,
            )

            self._client.subscribe(
                dataset=self._dataset,
                schema="ohlcv-1m",
                stype_in="continuous",
                symbols=self._db_symbols,
            )

            for record in self._client:
                if not self._running:
                    break
                if isinstance(record, db.OHLCVMsg):
                    self._handle_ohlcv(record)
                elif isinstance(record, db.ErrorMsg):
                    self._last_error = record.err
                    logger.error("Live error: %s", record.err)

        except Exception as e:
            self._last_error = str(e)
            logger.error("Live feeder 异常: %s", e)
        finally:
            self._running = False

    def _handle_ohlcv(self, msg: db.OHLCVMsg) -> None:
        """处理一条 OHLCVMsg，增量追加到缓存。"""
        try:
            # 通过 symbology_map 把 instrument_id 映射回 db_symbol
            sym_map = getattr(self._client, "symbology_map", {})
            db_sym = sym_map.get(msg.instrument_id)

            if db_sym is None:
                return  # 未知 instrument，跳过

            our_sym = self._db_to_our.get(db_sym)
            if our_sym is None:
                return  # 不在我们的订阅列表中

            # 用 pretty_* 属性获取 float 价格和 datetime 时间戳
            ts = msg.pretty_ts_event
            if hasattr(ts, "tz") and ts.tzinfo is not None:
                ts = ts.replace(tzinfo=None)  # 去时区

            row = pd.DataFrame(
                {
                    "open": [msg.pretty_open],
                    "high": [msg.pretty_high],
                    "low": [msg.pretty_low],
                    "close": [msg.pretty_close],
                    "volume": [msg.volume],
                },
                index=pd.DatetimeIndex([ts], name="ts_event"),
            )

            cache_name = self._cache_map[our_sym]
            append_df(cache_name, row)
            self._bar_count += 1

            if self._bar_count <= 5 or self._bar_count % 100 == 0:
                logger.info(
                    "Live bar #%d: %s %s O=%.2f H=%.2f L=%.2f C=%.2f V=%d",
                    self._bar_count, our_sym, ts,
                    msg.pretty_open, msg.pretty_high,
                    msg.pretty_low, msg.pretty_close, msg.volume,
                )

        except Exception as e:
            logger.warning("Live bar 处理失败: %s", e)
