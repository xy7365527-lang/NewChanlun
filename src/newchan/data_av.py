"""Alpha Vantage 数据源

支持：
  - 日内/日线 K 线数据（TIME_SERIES_INTRADAY / DAILY）
  - Brent 原油日线
  - MACD 技术指标（function=MACD）
"""

from __future__ import annotations

import time
from datetime import datetime

import pandas as pd
import requests

from newchan.config import ALPHAVANTAGE_API_KEY
from newchan.convert import bars_to_df as bars_to_df  # re-export 兼容旧导入
from newchan.types import Bar

_BASE_URL = "https://www.alphavantage.co/query"


class AlphaVantageProvider:
    """Alpha Vantage REST 数据提供者。

    Parameters
    ----------
    api_key : str | None
        API 密钥，默认读取 config 中的值。
    rate_limit : float
        两次请求之间的最小间隔（秒），默认 12.5 以遵守免费层限流。
    """

    def __init__(
        self,
        api_key: str | None = None,
        rate_limit: float = 12.5,
    ) -> None:
        self.api_key = api_key or ALPHAVANTAGE_API_KEY
        self.rate_limit = rate_limit
        self._last_call: float = 0.0

    # ------------------------------------------------------------------
    # 内部工具
    # ------------------------------------------------------------------

    def _throttle(self) -> None:
        """限流：确保两次请求之间间隔 >= rate_limit 秒。"""
        elapsed = time.monotonic() - self._last_call
        if elapsed < self.rate_limit:
            time.sleep(self.rate_limit - elapsed)
        self._last_call = time.monotonic()

    def _get(self, params: dict) -> dict:
        """发起一次 GET 请求并返回 JSON。"""
        self._throttle()
        params["apikey"] = self.api_key
        resp = requests.get(_BASE_URL, params=params, timeout=30)
        resp.raise_for_status()
        data = resp.json()
        # Alpha Vantage 用 JSON 内嵌 error/information 字段表示业务级错误
        if "Error Message" in data:
            raise RuntimeError(f"Alpha Vantage error: {data['Error Message']}")
        if "Information" in data:
            raise RuntimeError(f"Alpha Vantage info: {data['Information']}")
        return data

    # ------------------------------------------------------------------
    # 公开方法
    # ------------------------------------------------------------------

    def fetch_intraday(
        self,
        symbol: str,
        interval: str = "1min",
        outputsize: str = "compact",
    ) -> list[Bar]:
        """获取日内 K 线数据。

        Parameters
        ----------
        symbol : str
            标的代码（如 "SPY", "AAPL"）。
        interval : str
            K 线周期：1min / 5min / 15min / 30min / 60min。
        outputsize : str
            "compact"（最近 100 条）或 "full"（完整历史）。

        Returns
        -------
        list[Bar]
            按时间正序排列的 Bar 列表。
        """
        raw = self._get({
            "function": "TIME_SERIES_INTRADAY",
            "symbol": symbol,
            "interval": interval,
            "outputsize": outputsize,
            "datatype": "json",
        })
        key = f"Time Series ({interval})"
        series = raw.get(key, {})
        bars: list[Bar] = []
        for ts_str, values in sorted(series.items()):
            bars.append(
                Bar(
                    ts=datetime.strptime(ts_str, "%Y-%m-%d %H:%M:%S"),
                    open=float(values["1. open"]),
                    high=float(values["2. high"]),
                    low=float(values["3. low"]),
                    close=float(values["4. close"]),
                    volume=float(values["5. volume"]),
                )
            )
        return bars

    def fetch_daily(
        self,
        symbol: str,
        outputsize: str = "compact",
    ) -> list[Bar]:
        """获取日线 K 线数据。

        Parameters
        ----------
        symbol : str
            标的代码（如 "SPY", "GLD"）。
        outputsize : str
            "compact"（最近 100 条）或 "full"（完整历史）。

        Returns
        -------
        list[Bar]
            按时间正序排列的 Bar 列表。
        """
        raw = self._get({
            "function": "TIME_SERIES_DAILY",
            "symbol": symbol,
            "outputsize": outputsize,
            "datatype": "json",
        })
        series = raw.get("Time Series (Daily)", {})
        bars: list[Bar] = []
        for ts_str, values in sorted(series.items()):
            bars.append(
                Bar(
                    ts=datetime.strptime(ts_str, "%Y-%m-%d"),
                    open=float(values["1. open"]),
                    high=float(values["2. high"]),
                    low=float(values["3. low"]),
                    close=float(values["4. close"]),
                    volume=float(values["5. volume"]),
                )
            )
        return bars

    def fetch_macd(
        self,
        symbol: str,
        interval: str = "daily",
        series_type: str = "close",
        fastperiod: int = 12,
        slowperiod: int = 26,
        signalperiod: int = 9,
    ) -> pd.DataFrame:
        """获取 Alpha Vantage 计算的 MACD 技术指标。

        直接从 AV 获取 MACD，避免本地 warmup 期问题。
        返回格式与 ``a_macd.compute_macd()`` 兼容，可直接传入
        ``nested_divergence_search(df_macd=...)``。

        Parameters
        ----------
        symbol : str
            标的代码。
        interval : str
            数据周期：1min / 5min / 15min / 30min / 60min / daily / weekly / monthly。
        series_type : str
            价格类型：close / open / high / low。
        fastperiod, slowperiod, signalperiod : int
            MACD 参数（默认 12/26/9）。

        Returns
        -------
        pd.DataFrame
            列: ``macd``, ``signal``, ``hist``。
            index: DatetimeIndex（按时间正序）。
        """
        raw = self._get({
            "function": "MACD",
            "symbol": symbol,
            "interval": interval,
            "series_type": series_type,
            "fastperiod": str(fastperiod),
            "slowperiod": str(slowperiod),
            "signalperiod": str(signalperiod),
            "datatype": "json",
        })
        analysis = raw.get("Technical Analysis: MACD", {})
        records: list[dict] = []
        for ts_str, values in sorted(analysis.items()):
            records.append({
                "ts": ts_str,
                "macd": float(values["MACD"]),
                "signal": float(values["MACD_Signal"]),
                "hist": float(values["MACD_Hist"]),
            })
        df = pd.DataFrame(records)
        if df.empty:
            return pd.DataFrame(columns=["macd", "signal", "hist"])
        df["ts"] = pd.to_datetime(df["ts"])
        df = df.set_index("ts").sort_index()
        return df[["macd", "signal", "hist"]]

    def fetch_brent_daily(self) -> list[Bar]:
        """获取 Brent 原油日线数据，返回 Bar 列表。"""
        raw = self._get({"function": "BRENT", "interval": "daily", "datatype": "json"})
        bars: list[Bar] = []
        for item in raw.get("data", []):
            value_str = item.get("value", ".")
            if value_str == ".":
                continue  # 跳过缺失值
            value = float(value_str)
            bars.append(
                Bar(
                    ts=datetime.strptime(item["date"], "%Y-%m-%d"),
                    open=value,
                    high=value,
                    low=value,
                    close=value,
                    volume=None,
                )
            )
        return bars
