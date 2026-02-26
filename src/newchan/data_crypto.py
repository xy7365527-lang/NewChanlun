"""加密货币数据源 — 基于 ccxt 获取交易所 OHLCV 数据

支持：
  - 历史 OHLCV（REST: fetch_ohlcv）— 所有 ccxt 支持的交易所
  - 实时 WebSocket 订阅（ccxt.pro: watch_ohlcv）— Binance 等
  - 默认交易所: Binance
"""

from __future__ import annotations

import logging
from datetime import datetime, timezone

import pandas as pd

from newchan.convert import bars_to_df
from newchan.types import Bar

logger = logging.getLogger(__name__)

# ccxt timeframe → 项目 interval 映射
_TIMEFRAME_TO_INTERVAL: dict[str, str] = {
    "1m": "1min",
    "5m": "5min",
    "15m": "15min",
    "30m": "30min",
    "1h": "1hour",
    "4h": "4hour",
    "1d": "1day",
    "1w": "1week",
}


def _get_ccxt():
    """延迟导入 ccxt，避免未安装时模块级报错。"""
    import ccxt
    return ccxt


def _symbol_to_cache_key(symbol: str) -> str:
    """将交易对符号转为缓存键（替换 / 为 _）。"""
    return symbol.replace("/", "_")


def _timeframe_to_interval(timeframe: str) -> str:
    """将 ccxt timeframe 转为项目 interval 字符串。"""
    return _TIMEFRAME_TO_INTERVAL.get(timeframe, timeframe)


def _parse_ws_ohlcv(raw: list[list]) -> list[Bar]:
    """将 ccxt OHLCV 原始数据解析为 Bar 列表。

    格式: [[ts_ms, open, high, low, close, volume], ...]
    """
    bars: list[Bar] = []
    for row in raw:
        ts_ms, o, h, l, c, v = row[0], row[1], row[2], row[3], row[4], row[5]
        bars.append(Bar(
            ts=datetime.fromtimestamp(ts_ms / 1000, tz=timezone.utc).replace(tzinfo=None),
            open=float(o),
            high=float(h),
            low=float(l),
            close=float(c),
            volume=float(v) if v else None,
        ))
    return bars


class CryptoProvider:
    """加密货币 OHLCV 数据提供者（基于 ccxt）。

    Parameters
    ----------
    exchange_id : str
        交易所 ID（ccxt 标识），默认 "binance"。
    """

    def __init__(self, exchange_id: str = "binance") -> None:
        self.exchange_id = exchange_id
        ccxt = _get_ccxt()
        exchange_cls = getattr(ccxt, exchange_id)
        self._exchange = exchange_cls({"enableRateLimit": True})

    # ------------------------------------------------------------------
    # 历史 OHLCV
    # ------------------------------------------------------------------

    def fetch_ohlcv(
        self,
        symbol: str,
        timeframe: str = "1m",
        since: str | None = None,
        limit: int | None = None,
    ) -> list[Bar]:
        """获取历史 OHLCV 数据。

        Parameters
        ----------
        symbol : str
            交易对（如 "BTC/USDT", "ETH/USDT"）。
        timeframe : str
            K 线周期: "1m", "5m", "15m", "30m", "1h", "4h", "1d"。
        since : str | None
            起始日期 "YYYY-MM-DD"，None 表示由交易所决定。
        limit : int | None
            返回条数上限。

        Returns
        -------
        list[Bar]
            按时间正序排列的 Bar 列表。
        """
        since_ms = None
        if since is not None:
            since_ms = int(
                datetime.strptime(since, "%Y-%m-%d")
                .replace(tzinfo=timezone.utc)
                .timestamp() * 1000
            )

        logger.info("ccxt: %s %s tf=%s since=%s limit=%s",
                     self.exchange_id, symbol, timeframe, since, limit)

        raw = self._exchange.fetch_ohlcv(
            symbol, timeframe=timeframe, since=since_ms, limit=limit,
        )
        if not raw:
            logger.warning("ccxt 返回空数据: %s %s", symbol, timeframe)
            return []

        bars = _parse_ws_ohlcv(raw)
        bars.sort(key=lambda b: b.ts)
        logger.info("ccxt 返回 %d 条 %s %s", len(bars), symbol, timeframe)
        return bars

    # ------------------------------------------------------------------
    # DataFrame 接口
    # ------------------------------------------------------------------

    def fetch_ohlcv_df(
        self,
        symbol: str,
        timeframe: str = "1m",
        since: str | None = None,
        limit: int | None = None,
    ) -> pd.DataFrame:
        """获取历史 OHLCV 并返回标准 DataFrame。

        Returns
        -------
        pd.DataFrame
            标准 OHLCV DataFrame（tz-naive DateTimeIndex）。
        """
        bars = self.fetch_ohlcv(symbol, timeframe, since, limit)
        if not bars:
            return pd.DataFrame(columns=["open", "high", "low", "close", "volume"])
        return bars_to_df(bars)

    # ------------------------------------------------------------------
    # 缓存接口
    # ------------------------------------------------------------------

    def fetch_and_cache(
        self,
        symbol: str,
        timeframe: str = "1m",
        since: str | None = None,
        limit: int | None = None,
    ) -> tuple[str, int]:
        """拉取数据并增量追加到缓存。返回 (cache_name, total_rows)。"""
        from newchan.cache import append_df, load_df

        df = self.fetch_ohlcv_df(symbol, timeframe, since, limit)
        interval = _timeframe_to_interval(timeframe)
        cache_key = _symbol_to_cache_key(symbol)
        cache_name = f"{cache_key}_{interval}_raw"

        if df.empty:
            return cache_name, 0

        append_df(cache_name, df)
        df_cached = load_df(cache_name)
        count = len(df_cached) if df_cached is not None else len(df)
        return cache_name, count

    # ------------------------------------------------------------------
    # WebSocket 实时订阅
    # ------------------------------------------------------------------

    def _create_ws_exchange(self):
        """创建 ccxt.pro 异步交易所实例（用于 WebSocket）。"""
        ccxt = _get_ccxt()
        exchange_cls = getattr(ccxt.pro, self.exchange_id)
        return exchange_cls({"enableRateLimit": True})

    async def watch_ohlcv(
        self,
        symbol: str,
        timeframe: str = "1m",
    ) -> list[Bar]:
        """通过 WebSocket 获取一次实时 OHLCV 更新。

        需在 async 上下文中调用。每次调用返回最新的 OHLCV 数据。

        Parameters
        ----------
        symbol : str
            交易对（如 "BTC/USDT"）。
        timeframe : str
            K 线周期。

        Returns
        -------
        list[Bar]
            最新的 OHLCV Bar 列表。
        """
        ws_exchange = self._create_ws_exchange()
        try:
            raw = await ws_exchange.watch_ohlcv(symbol, timeframe)
            return _parse_ws_ohlcv(raw) if raw else []
        finally:
            await ws_exchange.close()
