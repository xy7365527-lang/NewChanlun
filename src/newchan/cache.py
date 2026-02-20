"""Parquet 本地缓存"""

from __future__ import annotations

import re
from pathlib import Path

import pandas as pd

from newchan.config import CACHE_DIR

# 已知 interval 后缀（与 data_ibkr._BAR_SIZE_MAP 保持一致）
_KNOWN_INTERVALS = (
    "1s", "5s", "10s", "15s", "30s",
    "1min", "2min", "3min", "5min", "10min", "15min", "20min", "30min",
    "1hour", "2hour", "3hour", "4hour",
    "1day", "1week", "1month",
)
# 按长度降序排列，确保 "10min" 优先于 "1min" 等前缀匹配
_INTERVAL_PATTERN = "|".join(sorted(_KNOWN_INTERVALS, key=len, reverse=True))
# 缓存文件名格式: {SYMBOL}_{interval}_raw.parquet
# 使用已知 interval 列表做后缀匹配，支持含下划线的合成品种名
_CACHE_RE = re.compile(rf"^(.+)_({_INTERVAL_PATTERN})_raw\.parquet$")


def _cache_dir() -> Path:
    p = Path(CACHE_DIR)
    p.mkdir(parents=True, exist_ok=True)
    return p


def load_df(name: str) -> pd.DataFrame | None:
    """从缓存加载 DataFrame，不存在返回 None。"""
    path = _cache_dir() / f"{name}.parquet"
    if not path.exists():
        return None
    return pd.read_parquet(path)


def save_df(name: str, df: pd.DataFrame) -> Path:
    """将 DataFrame 写入缓存，返回文件路径。"""
    path = _cache_dir() / f"{name}.parquet"
    df.to_parquet(path, engine="pyarrow")
    return path


def _normalize_tz(df: pd.DataFrame) -> pd.DataFrame:
    """统一去除 index 时区信息（tz-naive），避免合并冲突。"""
    if hasattr(df.index, "tz") and df.index.tz is not None:
        df = df.copy()
        df.index = df.index.tz_localize(None)
    return df


def append_df(name: str, df_new: pd.DataFrame) -> Path:
    """增量追加数据到缓存（去重、排序后写回）。

    若缓存不存在则直接保存。若已存在则合并、按 index 去重（保留最新值）、排序。
    自动统一时区（去除 tz 信息）避免 tz-naive vs tz-aware 冲突。
    """
    df_new = _normalize_tz(df_new)
    df_old = load_df(name)
    if df_old is not None and len(df_old) > 0:
        df_old = _normalize_tz(df_old)
        combined = pd.concat([df_old, df_new])
        combined = combined[~combined.index.duplicated(keep="last")]
        combined = combined.sort_index()
        return save_df(name, combined)
    return save_df(name, df_new.sort_index())


def list_cached() -> list[dict]:
    """扫描缓存目录，返回已缓存品种信息列表。

    每个元素: {"name": "CL_1min_raw", "symbol": "CL", "interval": "1min"}
    合成品种: {"name": "CL_GC_spread_1min_raw", "symbol": "CL_GC_spread", "interval": "1min"}
    """
    results: list[dict] = []
    for f in sorted(_cache_dir().glob("*_raw.parquet")):
        m = _CACHE_RE.match(f.name)
        if m:
            symbol, interval = m.group(1), m.group(2)
            results.append({
                "name": f"{symbol}_{interval}_raw",
                "symbol": symbol,
                "interval": interval,
            })
    return results
