"""K4 Databento tick 抓取政策模块（issue #1089 / ADR 0023 §三）。

只收「Databento SDK 传输 + Parquet 落盘政策」，供 `fetch_k4_databento_tick.py`（薄 CLI）import：

- `get_day_dbn`（重试/退避）：单日 trades `get_range`，429/5xx/超时/连接错误/流中断指数退避
  （带抖动，读 Retry-After），422 数据越界（data_start_before_available / unavailable_range）
  判空返回 None，其余 4xx 致命（401/403 密钥、404 标的）；
- `partition_done`（续拉判据）：分区存在且行数>0 视为已完成；
- `write_partition`（PARQUET 直出原子落盘）：DBNStore.to_parquet 流式直写临时文件 + rename，
  空分区不落盘；
- `fetch_symbol`（逐日续拉）：断点续拉主循环，单日失败不中断，`failed_days` 显式清单。

六边 M/P/C/R 标的清单、地板起点、CLI（main/plan）留在脚本层 `fetch_k4_databento_tick.py`
——K4 特有语义，非传输政策。与 Massive 政策模块（scripts/massive_fetch.py）物理隔离
（#1063 第 3 条 seam 保持：只服务 Massive；K4 走 SDK 直出，无 REST 政策可共享）。
"""

from __future__ import annotations

import datetime as dt
import os
import random
import time
from pathlib import Path
from typing import Iterator

import requests

try:
    import pyarrow.parquet as pq
except ImportError:  # pragma: no cover - pyarrow 是 pyproject 显式依赖
    pq = None

from databento.common.error import BentoClientError, BentoError, BentoServerError

# trades schema 全字段（DBN TradeMsg._ordered_fields，除隐藏字段 length）：
# ts_recv / ts_event / rtype / publisher_id / instrument_id / action / side /
# depth / price / size / flags / ts_in_delta / sequence —— 全部由 DBNStore.to_parquet
# 原样保留（pretty_ts=True 把 ts_recv/ts_event 落 timestamp[ns]，历史段 ns 全精度）。
TRADES_SCHEMA = "trades"


def log(msg: str) -> None:
    print(f"[{dt.datetime.now():%Y-%m-%d %H:%M:%S}] {msg}", flush=True)


def iter_days(start: dt.date, end: dt.date) -> Iterator[dt.date]:
    """[start, end] 闭区间逐日迭代，**不跳周末**。

    K4 线为期货：CME Globex 周日晚（UTC）开盘、周六才整日休市；且分区键是 UTC
    自然日存储切片（按 ts_recv 落档），不按交易时段裁，故不套个股线 skip_weekends。
    无成交的自然日（周六/节假日）会得到空分区并被跳过（见 write_partition 判空）。
    """
    d = start
    while d <= end:
        yield d
        d += dt.timedelta(days=1)


def partition_path(root, symbol: str, date_str: str) -> Path:
    return Path(root) / symbol / f"dt={date_str}" / "trades.parquet"


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


def _retry_after(headers, attempt: int, *, base: float, cap: float) -> float:
    """优先读服务端 Retry-After 头（秒），缺失/非法退指数退避。"""
    raw = headers.get("Retry-After") if headers else None
    if raw:
        try:
            return float(raw)
        except (TypeError, ValueError):
            pass
    return backoff_delay(attempt, base=base, cap=cap)


def _is_range_422(exc: BentoClientError) -> bool:
    """422 数据越界（start 早于地板 / end 超出可用）→ 判空，不致命。"""
    if exc.http_status != 422:
        return False
    msg = str(exc)
    return "data_start_before_available" in msg or "unavailable_range" in msg


def get_day_dbn(
    client,
    *,
    dataset: str,
    symbol: str,
    stype_in: str,
    start: dt.datetime,
    end: dt.datetime,
    max_retries: int = 6,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
    sleep=time.sleep,
):
    """拉单标的单日 trades DBN（schema=trades, stype_in=continuous），带退避。

    返回 DBNStore（供 write_partition 直出 Parquet）；422 数据越界返回 None（判空）。
    429/5xx/超时/连接错误/流中断指数退避重试；其余 4xx 致命（密钥/标的/参数错误）。
    """
    last_err: Exception | None = None
    for attempt in range(1, max_retries + 1):
        try:
            return client.timeseries.get_range(
                dataset=dataset,
                symbols=[symbol],
                schema=TRADES_SCHEMA,
                stype_in=stype_in,
                start=start,
                end=end,
            )
        except BentoClientError as e:
            last_err = e
            if e.http_status == 429:
                wait = _retry_after(e.headers, attempt, base=backoff_base, cap=max_backoff)
                log(f"[{symbol} {start:%Y-%m-%d}] HTTP 429 退避 {wait:.1f}s 后重试（{attempt}/{max_retries}）")
                sleep(wait)
                continue
            if _is_range_422(e):
                log(f"[{symbol} {start:%Y-%m-%d}] 数据越界（422）判空跳过：{e}")
                return None
            raise  # 其余 4xx（401/403/404/参数）致命
        except BentoServerError as e:
            last_err = e
            wait = _retry_after(e.headers, attempt, base=backoff_base, cap=max_backoff)
            log(f"[{symbol} {start:%Y-%m-%d}] HTTP {e.http_status} 退避 {wait:.1f}s 后重试（{attempt}/{max_retries}）")
            sleep(wait)
        except (requests.exceptions.RequestException, BentoError) as e:
            last_err = e
            wait = backoff_delay(attempt, base=backoff_base, cap=max_backoff)
            log(f"[{symbol} {start:%Y-%m-%d}] 请求异常 {e!r}，退避 {wait:.1f}s 后重试（{attempt}/{max_retries}）")
            sleep(wait)
    raise RuntimeError(
        f"重试耗尽（{max_retries} 次）：{dataset} {symbol} {start:%Y-%m-%d}（最后错误 {last_err!r}）"
    )


def write_partition(root, symbol: str, date_str: str, store) -> Path | None:
    """DBNStore → Parquet(ZSTD) 直出原子落盘（临时文件 + rename）。空分区返回 None（不落盘）。

    `store.to_parquet` 流式分块直写 pyarrow（PARQUET_CHUNK_SIZE=2^16 行），不物化 pandas；
    pretty_ts=True 把 ts_recv/ts_event 落 timestamp[ns]（历史段 ns 全精度，ADR 0023 §三）；
    map_symbols=True 补 symbol 列（instrument_id → raw symbol）。trades schema 全字段保留。
    """
    day_dir = Path(root) / symbol / f"dt={date_str}"
    day_dir.mkdir(parents=True, exist_ok=True)
    tmp = day_dir / f".trades.parquet.tmp.{os.getpid()}.{int(time.time() * 1000)}"
    final = day_dir / "trades.parquet"
    try:
        store.to_parquet(tmp, pretty_ts=True, map_symbols=True)
        if not tmp.exists() or pq.read_metadata(str(tmp)).num_rows == 0:
            tmp.unlink(missing_ok=True)
            return None
        tmp.replace(final)
        return final
    except Exception:
        if tmp.exists():
            tmp.unlink(missing_ok=True)
        raise


def fetch_symbol(
    client,
    symbol: str,
    *,
    dataset: str,
    db_symbol: str,
    stype_in: str,
    root,
    start: dt.date,
    end: dt.date,
    max_retries: int,
    backoff_base: float,
    max_backoff: float,
    sleep=time.sleep,
) -> dict:
    """拉单标的区间 tick（断点续拉），返回统计 dict（含 `failed_days` 显式清单）。

    单日失败不中断（下次续拉重试）；`failed_days` 为 `[(date_str, error_str), ...]`。
    """
    fetched = skipped = empty = failed = 0
    failed_days: list[tuple[str, str]] = []
    days_seen = 0
    for d in iter_days(start, end):
        days_seen += 1
        date_str = d.isoformat()
        path = partition_path(root, symbol, date_str)
        if partition_done(path):
            skipped += 1
        else:
            day_start = dt.datetime(d.year, d.month, d.day, tzinfo=dt.timezone.utc)
            day_end = day_start + dt.timedelta(days=1)
            try:
                store = get_day_dbn(
                    client, dataset=dataset, symbol=db_symbol, stype_in=stype_in,
                    start=day_start, end=day_end, max_retries=max_retries,
                    backoff_base=backoff_base, max_backoff=max_backoff, sleep=sleep,
                )
            except Exception as e:
                failed += 1
                failed_days.append((date_str, str(e)))
                log(f"[{symbol} {date_str}] 拉取失败（跳过，下次续拉重试）：{e}")
            else:
                if store is None:
                    empty += 1
                else:
                    try:
                        out = write_partition(root, symbol, date_str, store)
                    except Exception as e:
                        failed += 1
                        failed_days.append((date_str, str(e)))
                        log(f"[{symbol} {date_str}] 落盘失败（跳过）：{e}")
                    else:
                        if out is None:
                            empty += 1
                        else:
                            fetched += 1
                            try:
                                n_rows = pq.read_metadata(str(out)).num_rows
                            except Exception:
                                n_rows = -1
                            log(f"[{symbol} {date_str}] 落盘 {n_rows} 笔")
        if days_seen % 250 == 0:
            log(
                f"[{symbol}] 进度 {days_seen} 天（已拉 {fetched} / 跳过 {skipped} / "
                f"空 {empty} / 失败 {failed}）"
            )
    return {
        "symbol": symbol,
        "fetched": fetched,
        "skipped": skipped,
        "empty": empty,
        "failed": failed,
        "failed_days": failed_days,
    }
