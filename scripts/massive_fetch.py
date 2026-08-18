"""Massive 抓取政策模块（issue #1068 / #1063 Resolution 第 2 条）。

只收「传输 + 落盘政策」，供 `fetch_massive_tick.py`（薄 CLI）import：

- `http_get_json`（重试/退避）：429/5xx/超时/连接错误指数退避（带抖动），4xx 致命；
- `fetch_day`（分页遍历）：单标的单日全部逐笔成交，走 next_url 游标；
- `partition_done`（续拉判据）：分区存在且行数>0 视为已完成；
- `write_partition`（原子落盘）：临时文件 + rename，空行不落盘；
- `fetch_symbol`（逐日续拉）：断点续拉主循环，单日失败不中断，`failed_days` 显式清单。

起点解析（`resolve_start_date`/`get_list_date`/`probe_first_nonempty_day`/`LATE_LISTED`）
与 CLI（`main`/`plan`）留在脚本层 `fetch_massive_tick.py`——Massive 特有语义，非传输政策。
"""

from __future__ import annotations

import datetime as dt
import os
import random
import time
from pathlib import Path
from typing import Iterator

import pandas as pd
import requests

try:
    import pyarrow.parquet as pq
except ImportError:  # pragma: no cover - pyarrow 是 pyproject 显式依赖
    pq = None

BASE_URL = "https://api.massive.com"
DEFAULT_LIMIT = 50_000
MAX_LIMIT = 50_000

# 13 字段（#1030 实测字段 + ticker 补），列序固定；correction 若返回追加其后。
CANONICAL_COLUMNS = [
    "id",
    "ticker",
    "conditions",
    "exchange",
    "participant_timestamp",
    "sip_timestamp",
    "trf_timestamp",
    "price",
    "sequence_number",
    "size",
    "tape",
    "trf_id",
    "decimal_size",
]
CORRECTION_COLUMN = "correction"


def log(msg: str) -> None:
    print(f"[{dt.datetime.now():%Y-%m-%d %H:%M:%S}] {msg}", flush=True)


def iter_dates(start: dt.date, end: dt.date, skip_weekends: bool = True) -> Iterator[dt.date]:
    """[start, end] 闭区间逐日迭代；skip_weekends 时跳过周六日（美股周末休市）。"""
    d = start
    while d <= end:
        if not (skip_weekends and d.weekday() >= 5):
            yield d
        d += dt.timedelta(days=1)


def partition_path(root, ticker: str, date_str: str) -> Path:
    return Path(root) / ticker / f"dt={date_str}" / "trades.parquet"


def partition_done(path: Path) -> bool:
    """断点续拉判据：分区存在且行数>0 视为已完成；损坏/空分区视为未完成（重拉）。"""
    if not path.exists():
        return False
    if pq is None:
        return False
    try:
        return pq.read_metadata(str(path)).num_rows > 0
    except Exception:
        return False


def backoff_delay(attempt: int, *, base: float = 1.0, cap: float = 60.0) -> float:
    """指数退避 + 抖动：base*2^(attempt-1) 封顶 cap，再乘 [0.5, 1.0] 抖动。"""
    delay = min(cap, base * (2 ** max(0, attempt - 1)))
    return delay * (0.5 + 0.5 * random.random())


def _retry_after(resp, attempt: int, *, base: float, cap: float) -> float:
    raw = resp.headers.get("Retry-After")
    if raw:
        try:
            return float(raw)
        except (TypeError, ValueError):
            pass
    return backoff_delay(attempt, base=base, cap=cap)


def http_get_json(
    session,
    url: str,
    *,
    params=None,
    headers=None,
    timeout: float = 30.0,
    max_retries: int = 6,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
    sleep=time.sleep,
) -> dict:
    """带退避的 GET→JSON。429/5xx/超时/连接错误指数退避重试；其余 4xx 致命（密钥/标的错误）。"""
    last_err: Exception | None = None
    for attempt in range(1, max_retries + 1):
        try:
            resp = session.get(url, headers=headers, params=params, timeout=timeout)
        except requests.exceptions.RequestException as e:
            last_err = e
        else:
            if resp.status_code == 200:
                return resp.json()
            if resp.status_code == 429 or resp.status_code >= 500:
                wait = _retry_after(resp, attempt, base=backoff_base, cap=max_backoff)
                log(f"HTTP {resp.status_code}（{url}）退避 {wait:.1f}s 后重试（{attempt}/{max_retries}）")
                sleep(wait)
                continue
            raise RuntimeError(
                f"HTTP {resp.status_code}（{url}）：{resp.text[:200]!r}"
            )
        wait = backoff_delay(attempt, base=backoff_base, cap=max_backoff)
        log(f"请求异常（{url}）：{last_err!r}，退避 {wait:.1f}s 后重试（{attempt}/{max_retries}）")
        sleep(wait)
    raise RuntimeError(f"请求重试耗尽（{max_retries} 次）：{url}（最后错误 {last_err!r}）")


def _abs_url(next_url: str) -> str:
    if next_url.startswith("http"):
        return next_url
    if next_url.startswith("/"):
        return BASE_URL + next_url
    return BASE_URL + "/" + next_url


def fetch_day(
    session,
    ticker: str,
    date_str: str,
    *,
    headers,
    limit: int = DEFAULT_LIMIT,
    max_retries: int = 6,
    timeout: float = 30.0,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
    max_pages: int = 2000,
) -> list[dict]:
    """拉单标的单日全部逐笔成交（分页遍历 next_url 游标，返回原样 JSON 行列表）。

    max_pages 是防呆上限（单日 ~60 万笔 / 5 万 ≈ 13 页，2000 页 ≈ 1 亿笔远超任何单日量），
    仅在服务端返回永不终结的 next_url 时触发，避免无限循环。
    """
    rows: list[dict] = []
    url = f"{BASE_URL}/v3/trades/{ticker}"
    params: dict | None = {"timestamp": date_str, "limit": limit}
    pages = 0
    while True:
        pages += 1
        if pages > max_pages:
            raise RuntimeError(f"[{ticker} {date_str}] 分页超过 {max_pages} 页，疑似 next_url 永不终结，中止")
        payload = http_get_json(
            session, url, params=params, headers=headers, timeout=timeout,
            max_retries=max_retries, backoff_base=backoff_base, max_backoff=max_backoff,
        )
        if not isinstance(payload, dict):
            raise ValueError(f"响应非 JSON 对象（{url}）")
        page = payload.get("results") or []
        if not isinstance(page, list):
            raise ValueError(f"results 非数组（{url}）")
        rows.extend(page)
        next_url = payload.get("next_url")
        if not next_url:
            break
        url = _abs_url(str(next_url))
        params = None
    return rows


def normalize_frame(rows: list[dict], ticker: str) -> pd.DataFrame:
    """原样行 → DataFrame：补 ticker、补缺失 canonical 列（None）、固定列序、trf 两列 Int64。"""
    if rows:
        df = pd.DataFrame(rows)
    else:
        df = pd.DataFrame(columns=CANONICAL_COLUMNS)
    df = df.copy()
    df["ticker"] = ticker
    for col in CANONICAL_COLUMNS:
        if col not in df.columns:
            df[col] = None
    for col in ("trf_id", "trf_timestamp"):
        if col in df.columns:
            df[col] = pd.to_numeric(df[col], errors="coerce").astype("Int64")
    ordered = list(CANONICAL_COLUMNS)
    if CORRECTION_COLUMN in df.columns:
        ordered.append(CORRECTION_COLUMN)
    extras = [c for c in df.columns if c not in ordered]
    ordered.extend(extras)
    return df[ordered]


def write_partition(root, ticker: str, date_str: str, rows: list[dict]) -> Path | None:
    """落盘单日分区（原子写：临时文件 + rename）。空行返回 None（不落盘）。"""
    if not rows:
        return None
    df = normalize_frame(rows, ticker)
    day_dir = Path(root) / ticker / f"dt={date_str}"
    day_dir.mkdir(parents=True, exist_ok=True)
    tmp = day_dir / f".trades.parquet.tmp.{os.getpid()}.{int(time.time() * 1000)}"
    final = day_dir / "trades.parquet"
    try:
        df.to_parquet(tmp, engine="pyarrow", compression="zstd", index=False)
        tmp.replace(final)
    except Exception:
        if tmp.exists():
            tmp.unlink(missing_ok=True)
        raise
    return final


def fetch_symbol(
    session,
    ticker: str,
    *,
    root,
    headers,
    start: dt.date,
    end: dt.date,
    skip_weekends: bool,
    limit: int,
    max_retries: int,
    timeout: float,
    backoff_base: float,
    max_backoff: float,
) -> dict:
    """拉单标的全史（断点续拉），返回统计 dict（含 `failed_days` 显式清单）。

    单日失败不中断（下次续拉重试）；`failed_days` 为 `[(date_str, error_str), ...]`。
    """
    fetched = skipped = empty = failed = 0
    failed_days: list[tuple[str, str]] = []
    days_seen = 0
    for d in iter_dates(start, end, skip_weekends):
        days_seen += 1
        date_str = d.isoformat()
        path = partition_path(root, ticker, date_str)
        if partition_done(path):
            skipped += 1
        else:
            try:
                rows = fetch_day(
                    session, ticker, date_str, headers=headers, limit=limit,
                    max_retries=max_retries, timeout=timeout,
                    backoff_base=backoff_base, max_backoff=max_backoff,
                )
            except Exception as e:
                failed += 1
                failed_days.append((date_str, str(e)))
                log(f"[{ticker} {date_str}] 拉取失败（跳过，下次续拉重试）：{e}")
            else:
                if not rows:
                    empty += 1
                else:
                    try:
                        write_partition(root, ticker, date_str, rows)
                        fetched += 1
                        log(f"[{ticker} {date_str}] 落盘 {len(rows)} 笔")
                    except Exception as e:
                        failed += 1
                        failed_days.append((date_str, str(e)))
                        log(f"[{ticker} {date_str}] 落盘失败（跳过）：{e}")
        if days_seen % 250 == 0:
            log(
                f"[{ticker}] 进度 {days_seen} 天（已拉 {fetched} / 跳过 {skipped} / "
                f"空 {empty} / 失败 {failed}）"
            )
    return {
        "ticker": ticker,
        "fetched": fetched,
        "skipped": skipped,
        "empty": empty,
        "failed": failed,
        "failed_days": failed_days,
    }
