"""A 系统 — MACD 力度指标

MACD 是"指标力度"，不参与结构断言；只作为输出与买卖点/显示依据。
使用 pandas ewm 实现，不依赖 TA-Lib。
"""

from __future__ import annotations

import pandas as pd


def compute_macd(
    df_raw: pd.DataFrame,
    fast: int = 12,
    slow: int = 26,
    signal: int = 9,
) -> pd.DataFrame:
    """计算 MACD 指标。

    Parameters
    ----------
    df_raw : pd.DataFrame
        必须含 ``close`` 列。
    fast, slow, signal : int
        EMA 周期参数。

    Returns
    -------
    pd.DataFrame
        列: ``macd``, ``signal``, ``hist``（hist = macd - signal）。
        index 与 df_raw 相同。
    """
    close = df_raw["close"]
    ema_fast = close.ewm(span=fast, adjust=False).mean()
    ema_slow = close.ewm(span=slow, adjust=False).mean()
    macd_line = ema_fast - ema_slow
    signal_line = macd_line.ewm(span=signal, adjust=False).mean()
    hist = macd_line - signal_line

    return pd.DataFrame({
        "macd": macd_line,
        "signal": signal_line,
        "hist": hist,
    }, index=df_raw.index)


def macd_area_for_range(
    df_macd: pd.DataFrame,
    raw_i0: int,
    raw_i1: int,
) -> dict:
    """计算指定原始 bar 范围内的 MACD 面积。

    Parameters
    ----------
    df_macd : pd.DataFrame
        由 ``compute_macd`` 返回。
    raw_i0, raw_i1 : int
        原始 df 的位置索引范围 [raw_i0, raw_i1]（闭区间）。

    Returns
    -------
    dict
        ``area_total``, ``area_pos``, ``area_neg``, ``n_bars``
    """
    if raw_i0 < 0:
        raw_i0 = 0
    if raw_i1 >= len(df_macd):
        raw_i1 = len(df_macd) - 1
    if raw_i0 > raw_i1:
        return {"area_total": 0.0, "area_pos": 0.0, "area_neg": 0.0, "n_bars": 0}

    hist = df_macd["hist"].iloc[raw_i0 : raw_i1 + 1]
    area_total = float(hist.sum())
    area_pos = float(hist.clip(lower=0).sum())
    area_neg = float(hist.clip(upper=0).sum())

    return {
        "area_total": round(area_total, 6),
        "area_pos": round(area_pos, 6),
        "area_neg": round(area_neg, 6),
        "n_bars": len(hist),
    }
