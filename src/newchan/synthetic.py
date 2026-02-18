"""合成标的 — 价差 / 比值计算"""

from __future__ import annotations

import pandas as pd


def _align(df_a: pd.DataFrame, df_b: pd.DataFrame) -> tuple[pd.DataFrame, pd.DataFrame]:
    """按时间戳对齐两个 DataFrame（inner join）。"""
    idx = df_a.index.intersection(df_b.index)
    return df_a.loc[idx], df_b.loc[idx]


def make_spread(df_a: pd.DataFrame, df_b: pd.DataFrame) -> pd.DataFrame:
    """计算价差：A - B。

    对 OHLC 四列分别做减法，volume 取 A 的成交量。

    Parameters
    ----------
    df_a, df_b : pd.DataFrame
        均需有 DatetimeIndex 以及 open/high/low/close 列。

    Returns
    -------
    pd.DataFrame
        价差 OHLCV DataFrame，可直接作为蜡烛图数据源。
    """
    a, b = _align(df_a, df_b)
    result = pd.DataFrame(index=a.index)
    result["open"] = a["open"] - b["open"]
    result["high"] = a["high"] - b["high"]
    result["low"] = a["low"] - b["low"]
    result["close"] = a["close"] - b["close"]
    if "volume" in a.columns:
        result["volume"] = a["volume"]
    return result


def make_ratio(df_a: pd.DataFrame, df_b: pd.DataFrame) -> pd.DataFrame:
    """计算比值：A / B。

    对 OHLC 四列分别做除法，volume 取 A 的成交量。
    委托给 equivalence.make_ratio_kline 实现。

    Parameters
    ----------
    df_a, df_b : pd.DataFrame
        均需有 DatetimeIndex 以及 open/high/low/close 列。

    Returns
    -------
    pd.DataFrame
        比值 OHLCV DataFrame，可直接作为蜡烛图数据源。
    """
    from newchan.equivalence import make_ratio_kline

    return make_ratio_kline(df_a, df_b)
