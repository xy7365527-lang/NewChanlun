"""A 股数据源 — 基于 akshare 获取 A 股历史行情

支持：
  - 分钟线（1/5/15/30/60 分钟）— stock_zh_a_hist_min_em
  - 日线（前复权/后复权/不复权）— stock_zh_a_hist
  - 停牌日自动跳过（akshare 返回空 DataFrame）
  - 除权处理通过 adjust 参数（qfq=前复权, hfq=后复权）
"""

from __future__ import annotations

import logging
import re
from datetime import datetime

import akshare as ak
import pandas as pd

from newchan.convert import bars_to_df
from newchan.types import Bar

logger = logging.getLogger(__name__)

# interval → akshare period 映射
_INTERVAL_TO_PERIOD: dict[str, str] = {
    "1min": "1",
    "5min": "5",
    "15min": "15",
    "30min": "30",
    "60min": "60",
    "1hour": "60",
}

# 前缀/后缀正则
_PREFIX_RE = re.compile(r"^(?:SZ|SH|sz|sh)", re.IGNORECASE)
_SUFFIX_RE = re.compile(r"\.(?:SZ|SH|sz|sh)$", re.IGNORECASE)


def _normalize_symbol(symbol: str) -> str:
    """标准化 A 股代码为纯 6 位数字。

    支持格式: 000001, SZ000001, 000001.SZ, SH600000, 600000.SH
    """
    s = symbol.strip()
    s = _PREFIX_RE.sub("", s)
    s = _SUFFIX_RE.sub("", s)
    return s


class AStockProvider:
    """A 股历史行情提供者（基于 akshare 东方财富接口）。

    Parameters
    ----------
    adjust : str
        复权方式: "qfq"（前复权）, "hfq"（后复权）, ""（不复权）。
        默认 "qfq"。
    """

    def __init__(self, adjust: str = "qfq") -> None:
        self.adjust = adjust

    # ------------------------------------------------------------------
    # 分钟线
    # ------------------------------------------------------------------

    def fetch_minute(
        self,
        symbol: str,
        period: str = "1",
    ) -> list[Bar]:
        """获取 A 股分钟线数据。

        Parameters
        ----------
        symbol : str
            股票代码（如 "000001", "SZ000001", "000001.SZ"）。
        period : str
            分钟周期: "1", "5", "15", "30", "60"。

        Returns
        -------
        list[Bar]
            按时间正序排列的 Bar 列表。停牌日返回空列表。
        """
        sym = _normalize_symbol(symbol)
        logger.info("akshare: 分钟线 %s period=%s adjust=%s", sym, period, self.adjust)

        df = ak.stock_zh_a_hist_min_em(
            symbol=sym,
            period=period,
            adjust=self.adjust,
        )
        if df is None or df.empty:
            logger.warning("akshare 返回空数据（停牌或无数据）: %s", sym)
            return []

        return _minute_df_to_bars(df)

    # ------------------------------------------------------------------
    # 日线
    # ------------------------------------------------------------------

    def fetch_daily(
        self,
        symbol: str,
        start_date: str = "20200101",
        end_date: str = "21000101",
    ) -> list[Bar]:
        """获取 A 股日线数据（前复权）。

        Parameters
        ----------
        symbol : str
            股票代码。
        start_date : str
            起始日期 "YYYYMMDD"。
        end_date : str
            结束日期 "YYYYMMDD"。

        Returns
        -------
        list[Bar]
        """
        sym = _normalize_symbol(symbol)
        logger.info(
            "akshare: 日线 %s [%s → %s] adjust=%s",
            sym, start_date, end_date, self.adjust,
        )

        df = ak.stock_zh_a_hist(
            symbol=sym,
            period="daily",
            start_date=start_date,
            end_date=end_date,
            adjust=self.adjust,
        )
        if df is None or df.empty:
            logger.warning("akshare 返回空数据: %s", sym)
            return []

        return _daily_df_to_bars(df)

    # ------------------------------------------------------------------
    # 统一接口: fetch_ohlcv → DataFrame
    # ------------------------------------------------------------------

    def fetch_ohlcv(
        self,
        symbol: str,
        interval: str = "1min",
        start_date: str = "20200101",
        end_date: str = "21000101",
    ) -> pd.DataFrame:
        """统一 OHLCV 接口，返回标准 DataFrame。

        Parameters
        ----------
        symbol : str
            股票代码。
        interval : str
            K 线周期: "1min", "5min", "15min", "30min", "60min", "1hour", "1day"。
        start_date, end_date : str
            日期范围（仅日线有效）。

        Returns
        -------
        pd.DataFrame
            标准 OHLCV DataFrame（tz-naive DateTimeIndex）。
        """
        if interval == "1day":
            bars = self.fetch_daily(symbol, start_date, end_date)
        else:
            period = _INTERVAL_TO_PERIOD.get(interval, "1")
            bars = self.fetch_minute(symbol, period=period)

        if not bars:
            return pd.DataFrame(columns=["open", "high", "low", "close", "volume"])

        return bars_to_df(bars)

    # ------------------------------------------------------------------
    # 缓存接口
    # ------------------------------------------------------------------

    def fetch_and_cache(
        self,
        symbol: str,
        interval: str = "1min",
        start_date: str = "20200101",
        end_date: str = "21000101",
    ) -> tuple[str, int]:
        """拉取数据并增量追加到缓存。返回 (cache_name, total_rows)。"""
        from newchan.cache import append_df, load_df

        df = self.fetch_ohlcv(symbol, interval, start_date, end_date)
        sym = _normalize_symbol(symbol)
        cache_name = f"{sym}_{interval}_raw"

        if df.empty:
            return cache_name, 0

        append_df(cache_name, df)
        df_cached = load_df(cache_name)
        count = len(df_cached) if df_cached is not None else len(df)
        return cache_name, count


# ====================================================================
# 内部转换
# ====================================================================


def _minute_df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """将 akshare 分钟线 DataFrame 转为 Bar 列表。"""
    bars: list[Bar] = []
    for _, row in df.iterrows():
        bars.append(Bar(
            ts=datetime.strptime(str(row["时间"]), "%Y-%m-%d %H:%M:%S"),
            open=float(row["开盘"]),
            high=float(row["最高"]),
            low=float(row["最低"]),
            close=float(row["收盘"]),
            volume=float(row["成交量"]),
        ))
    bars.sort(key=lambda b: b.ts)
    return bars


def _daily_df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """将 akshare 日线 DataFrame 转为 Bar 列表。"""
    bars: list[Bar] = []
    for _, row in df.iterrows():
        bars.append(Bar(
            ts=datetime.strptime(str(row["日期"]), "%Y-%m-%d"),
            open=float(row["开盘"]),
            high=float(row["最高"]),
            low=float(row["最低"]),
            close=float(row["收盘"]),
            volume=float(row["成交量"]),
        ))
    bars.sort(key=lambda b: b.ts)
    return bars
