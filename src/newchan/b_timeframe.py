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

_TF_SECONDS: dict[str, int] = {
    "1m": 60,
    "5m": 5 * 60,
    "15m": 15 * 60,
    "30m": 30 * 60,
    "1h": 60 * 60,
    "4h": 4 * 60 * 60,
    "1d": 24 * 60 * 60,
    "1w": 7 * 24 * 60 * 60,
}

SUPPORTED_TF = sorted(_TF_MAP.keys())


def _infer_source_seconds(index: pd.Index) -> float | None:
    """从 DatetimeIndex 推断原始 bar 间隔秒数；样本不足时返回 None。"""
    if not isinstance(index, pd.DatetimeIndex) or len(index) < 2:
        return None

    ordered = index.sort_values()
    diffs = ordered.to_series().diff().dropna()
    positive_diffs = diffs[diffs > pd.Timedelta(0)]
    if positive_diffs.empty:
        return None
    return float(positive_diffs.median().total_seconds())


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

    source_seconds = _infer_source_seconds(df.index)
    target_seconds = _TF_SECONDS[display_tf]
    if source_seconds is not None and target_seconds < source_seconds:
        raise ValueError(
            "不能重采样到更细周期: "
            f"源数据约 {source_seconds:g} 秒/bar，目标周期 {display_tf}"
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
