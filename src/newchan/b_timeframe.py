"""B 系统 — 显示周期重采样"""

from __future__ import annotations

import pandas as pd

# 用户友好名 -> pandas offset alias
_TF_MAP: dict[str, str] = {
    "1m": "1min",
    "5m": "5min",
    "15m": "15min",
    "30m": "30min",
    "1h": "1h",
    "4h": "4h",
    "1d": "1D",
    "1w": "1W",
}

SUPPORTED_TF = sorted(_TF_MAP.keys())


def resample_ohlc(df: pd.DataFrame, display_tf: str) -> pd.DataFrame:
    """将 OHLCV DataFrame 重采样到指定显示周期。

    Parameters
    ----------
    df : pd.DataFrame
        必须有 DatetimeIndex 以及 open/high/low/close 列，
        可选 volume 列。
    display_tf : str
        目标周期，支持: {tfs}。

    Returns
    -------
    pd.DataFrame
        重采样后的 OHLCV DataFrame。

    Notes
    -----
    若目标周期 <= 原始数据周期（如 daily 数据 resample 到 1m），
    pandas 会原样返回（每个原始 bar 自成一个窗口），不会报错。
    """.format(tfs=", ".join(SUPPORTED_TF))

    offset = _TF_MAP.get(display_tf)
    if offset is None:
        raise ValueError(
            f"不支持的 display_tf '{display_tf}'，可选: {', '.join(SUPPORTED_TF)}"
        )

    agg: dict[str, str] = {
        "open": "first",
        "high": "max",
        "low": "min",
        "close": "last",
    }
    if "volume" in df.columns:
        agg["volume"] = "sum"

    resampled = df.resample(offset).agg(agg).dropna(subset=["close"])
    return resampled
