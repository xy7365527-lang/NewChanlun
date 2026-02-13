"""Alpha Vantage 数据源"""

from __future__ import annotations

import time
from datetime import datetime

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
