"""Parquet 本地缓存"""

from __future__ import annotations

import os
import re
import sys
import tempfile
import threading
from contextlib import contextmanager
from pathlib import Path
from typing import Generator, IO

import pandas as pd

from newchan.config import CACHE_DIR

if sys.platform == "win32":
    import msvcrt
else:
    import fcntl

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

# Per-cache-key thread locks (live feeder + /api/fetch share one process).
_thread_locks: dict[str, threading.Lock] = {}
_thread_locks_guard = threading.Lock()


def _cache_dir() -> Path:
    p = Path(CACHE_DIR)
    p.mkdir(parents=True, exist_ok=True)
    return p


def _thread_lock_for(name: str) -> threading.Lock:
    with _thread_locks_guard:
        lock = _thread_locks.get(name)
        if lock is None:
            lock = threading.Lock()
            _thread_locks[name] = lock
        return lock


def _acquire_exclusive(lock_fh: IO[bytes]) -> None:
    if sys.platform == "win32":
        lock_fh.seek(0)
        lock_fh.write(b"\x00")
        lock_fh.flush()
        lock_fh.seek(0)
        msvcrt.locking(lock_fh.fileno(), msvcrt.LK_LOCK, 1)
    else:
        fcntl.flock(lock_fh.fileno(), fcntl.LOCK_EX)


def _release_exclusive(lock_fh: IO[bytes]) -> None:
    if sys.platform == "win32":
        lock_fh.seek(0)
        msvcrt.locking(lock_fh.fileno(), msvcrt.LK_UNLCK, 1)
    else:
        fcntl.flock(lock_fh.fileno(), fcntl.LOCK_UN)


@contextmanager
def _locked_cache_path(name: str) -> Generator[Path, None, None]:
    """Exclusive lock around a cache file (thread + cross-process).

    Prevents lost updates when DatabentoLiveFeeder.append_df races with
    /api/fetch → fetch_and_cache → append_df on the same symbol.
    """
    path = _cache_dir() / f"{name}.parquet"
    lock_path = path.with_suffix(path.suffix + ".lock")
    thread_lock = _thread_lock_for(name)
    with thread_lock:
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        with open(lock_path, "a+b") as lock_fh:
            _acquire_exclusive(lock_fh)
            try:
                yield path
            finally:
                _release_exclusive(lock_fh)


def _atomic_write_parquet(path: Path, df: pd.DataFrame) -> None:
    """Write parquet via temp file + os.replace so readers never see partial files."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(suffix=".parquet", dir=str(path.parent))
    os.close(fd)
    tmp_path = Path(tmp_name)
    try:
        df.to_parquet(tmp_path, engine="pyarrow")
        os.replace(tmp_path, path)
    except Exception:
        if tmp_path.exists():
            tmp_path.unlink(missing_ok=True)
        raise


def load_df(name: str) -> pd.DataFrame | None:
    """从缓存加载 DataFrame，不存在返回 None。"""
    path = _cache_dir() / f"{name}.parquet"
    if not path.exists():
        return None
    return pd.read_parquet(path)


def save_df(name: str, df: pd.DataFrame) -> Path:
    """将 DataFrame 写入缓存，返回文件路径。"""
    with _locked_cache_path(name) as path:
        _atomic_write_parquet(path, df)
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

    Read-modify-write 在锁内完成，避免 live 追加与历史回补互相覆盖丢行。
    """
    df_new = _normalize_tz(df_new)
    with _locked_cache_path(name) as path:
        df_old: pd.DataFrame | None = None
        if path.exists():
            df_old = pd.read_parquet(path)
        if df_old is not None and len(df_old) > 0:
            df_old = _normalize_tz(df_old)
            combined = pd.concat([df_old, df_new])
            combined = combined[~combined.index.duplicated(keep="last")]
            combined = combined.sort_index()
            _atomic_write_parquet(path, combined)
        else:
            _atomic_write_parquet(path, df_new.sort_index())
        return path


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
