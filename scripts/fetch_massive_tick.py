#!/usr/bin/env python3
"""Massive 个股线全史 tick 拉取管线（issue #1040，ADR 0023 §四-3）。

把 Massive Stocks REST 的逐笔成交（trades）**原样**落盘为 Parquet(ZSTD)，
per-symbol/per-day 分区：

    analysis/data_cache/massive_tick/{TICKER}/dt={YYYY-MM-DD}/trades.parquet

- 源端点：`GET https://api.massive.com/v3/trades/{ticker}?timestamp=YYYY-MM-DD&limit=50000`，
  分页走响应 `next_url`（游标），Bearer key 从 env 或仓库根 `.env` 读 `MASSIVE_API_KEY`
  （只进内存、不写任何文件）。
  **订正声明**：issue #1040 票面写 `?date=YYYY-MM-DD&sort=timestamp`，与 #1030 本体实测
  接线要点（`?timestamp=YYYY-MM-DD&limit=<max 50000>`，分页走 next_url，无 sort）不一致；
  按 #1030 实测订正为 `timestamp` 参数、不传 sort（API 默认时间升序）。票面 `date`/`sort`
  视为笔误。
- 原样落地（本票不去重）：per-venue 全量（含 TRF 重复与全条件码）——去重/过滤是 #1041 的活。
- schema（13 字段全保留 + correction 若返回，列序固定）：
    id / ticker(补) / conditions / exchange / participant_timestamp / sip_timestamp /
    trf_timestamp / price / sequence_number / size / tape / trf_id / decimal_size [+ correction]
  缺失的 canonical 列补 None（保证 schema 完整）；`trf_id`/`trf_timestamp` 以 pandas 可空
  Int64 落盘（保 int/null 语义，避免 float NaN 与「整日全空 → null 类型」）。
- 断点续拉：已存在且行数>0 的分区跳过（`pyarrow` 读 num_rows 判定，损坏/空分区视为未完成重拉）；
  API 429/超时/5xx 指数退避（带抖动），4xx 致命（401/403 密钥、404 标的）不重试。
- 十标 = AAPL/AMZN/COST/GOOGL/LLY/META/MSFT/NFLX/NVDA/TSLA；历史起点默认 2003-09-10，
  GOOGL/META/TSLA 上市日晚于起点：用 `/v3/reference/tickers/{ticker}` 的 `list_date` 自动
  探测首个非空日（不可达时逐日向前探测兜底）。

用法：
    # 单标的全史（COST 量最小，验收样例）
    uv run python scripts/fetch_massive_tick.py --symbols COST
    # 十标全史后台拉取
    nohup uv run python scripts/fetch_massive_tick.py \
        > analysis/data_cache/massive_tick_fetch.log 2>&1 &
    # 只打印计划（不联网、不读 key）：哪些日期会拉 / 哪些分区已存在会跳过
    uv run python scripts/fetch_massive_tick.py --plan
"""

from __future__ import annotations

import argparse
import datetime as dt
import os
import random
import sys
import time
from pathlib import Path
from typing import Iterator

import pandas as pd
import requests
from dotenv import load_dotenv

try:
    import pyarrow.parquet as pq
except ImportError:  # pragma: no cover - pyarrow 是 pyproject 显式依赖
    pq = None

_ROOT = Path(__file__).resolve().parents[1]

BASE_URL = "https://api.massive.com"
FLOOR_DATE = dt.date(2003, 9, 10)
DEFAULT_SYMBOLS = [
    "AAPL", "AMZN", "COST", "GOOGL", "LLY",
    "META", "MSFT", "NFLX", "NVDA", "TSLA",
]
# 上市日晚于数据集起点（2003-09-10），起点需联网解析的三只。
LATE_LISTED = frozenset({"GOOGL", "META", "TSLA"})
DEFAULT_ROOT = Path("analysis/data_cache/massive_tick")
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


def parse_date(s: str) -> dt.date:
    try:
        return dt.date.fromisoformat(s)
    except ValueError as e:
        raise SystemExit(f"非法日期 {s!r}（应为 YYYY-MM-DD）") from e


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


def load_api_key() -> str:
    load_dotenv(_ROOT / ".env")  # 不覆盖已设环境变量
    key = os.environ.get("MASSIVE_API_KEY", "").strip()
    if not key:
        raise SystemExit(
            "未找到 MASSIVE_API_KEY：请 `export MASSIVE_API_KEY=...` 或在仓库根 .env 写一行\n"
            "`MASSIVE_API_KEY=...`（该文件已 gitignore、应 chmod 600）。密钥只进内存，不写任何文件。"
        )
    return key


def auth_headers(key: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {key}"}


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


def get_list_date(
    session,
    ticker: str,
    headers,
    *,
    max_retries: int = 6,
    timeout: float = 30.0,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
) -> dt.date | None:
    """读 `/v3/reference/tickers/{ticker}` 的 list_date；失败/缺失返回 None（交兜底探测）。"""
    url = f"{BASE_URL}/v3/reference/tickers/{ticker}"
    try:
        payload = http_get_json(
            session, url, headers=headers, timeout=timeout,
            max_retries=max_retries, backoff_base=backoff_base, max_backoff=max_backoff,
        )
    except Exception as e:
        log(f"[{ticker}] list_date 查询失败（{e}），退回逐日探测")
        return None
    results = payload.get("results") or {}
    if not isinstance(results, dict):
        return None
    raw = results.get("list_date")
    if not raw:
        return None
    try:
        return dt.date.fromisoformat(str(raw)[:10])
    except ValueError:
        return None


def probe_first_nonempty_day(
    session,
    ticker: str,
    floor: dt.date,
    *,
    headers,
    limit: int = DEFAULT_LIMIT,
    max_retries: int = 6,
    timeout: float = 30.0,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
    max_days: int = 6000,
) -> dt.date | None:
    """兜底：从 floor 起逐日拉取，返回首个非空日（list_date 不可达时用）。"""
    d = floor
    for _ in range(max_days):
        if d > dt.date.today():
            return None
        rows = fetch_day(
            session, ticker, d.isoformat(), headers=headers, limit=limit,
            max_retries=max_retries, timeout=timeout,
            backoff_base=backoff_base, max_backoff=max_backoff,
        )
        if rows:
            return d
        d += dt.timedelta(days=1)
    raise RuntimeError(f"[{ticker}] 逐日探测超过 {max_days} 天仍无数据，中止（避免无限循环）")


def resolve_start_date(
    session,
    ticker: str,
    headers,
    floor: dt.date,
    *,
    limit: int = DEFAULT_LIMIT,
    max_retries: int = 6,
    timeout: float = 30.0,
    backoff_base: float = 1.0,
    max_backoff: float = 60.0,
) -> dt.date:
    """解析标的实际历史起点：非晚期上市 = floor；晚期上市 = max(floor, 上市日)。"""
    if ticker not in LATE_LISTED:
        return floor
    listing = get_list_date(
        session, ticker, headers, max_retries=max_retries, timeout=timeout,
        backoff_base=backoff_base, max_backoff=max_backoff,
    )
    if listing is None:
        listing = probe_first_nonempty_day(
            session, ticker, floor, headers=headers, limit=limit,
            max_retries=max_retries, timeout=timeout,
            backoff_base=backoff_base, max_backoff=max_backoff,
        )
    if listing is not None and listing > floor:
        return listing
    return floor


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
    """拉单标的全史（断点续拉），返回统计 dict。单日失败不中断（下次续拉重试）。"""
    fetched = skipped = empty = failed = 0
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
    }


def plan(root, symbols: list[str], start: dt.date, end: dt.date, skip_weekends: bool) -> None:
    """不联网、不读 key：按现有分区打印各标的计划（已存在分区 = 跳过）。"""
    print(f"计划（start={start} end={end} skip_weekends={skip_weekends} root={root}）")
    print("注：GOOGL/META/TSLA 实际起点 = 上市日（需联网解析），此处按传入 start 统计。")
    for ticker in symbols:
        n_days = n_existing = 0
        for d in iter_dates(start, end, skip_weekends):
            n_days += 1
            if partition_done(partition_path(root, ticker, d.isoformat())):
                n_existing += 1
        print(
            f"{ticker}: 需处理 {n_days} 天，已有分区 {n_existing} 天，"
            f"将拉 {n_days - n_existing} 天"
        )


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="Massive 个股线全史 tick 拉取管线（#1040）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument("--symbols", default=",".join(DEFAULT_SYMBOLS),
                    help=f"逗号分隔标的（默认十标：{','.join(DEFAULT_SYMBOLS)}）")
    ap.add_argument("--start", default=FLOOR_DATE.isoformat(),
                    help=f"历史起点（默认 {FLOOR_DATE.isoformat()}）")
    ap.add_argument("--end", default=None, help="结束日（默认今天，含当日）")
    ap.add_argument("--root", type=Path, default=DEFAULT_ROOT, help="落盘根目录")
    ap.add_argument("--limit", type=int, default=DEFAULT_LIMIT,
                    help=f"单请求行数（REST 上限 {MAX_LIMIT}）")
    ap.add_argument("--max-retries", type=int, default=6, help="429/5xx/超时重试次数")
    ap.add_argument("--timeout", type=float, default=30.0, help="单请求超时（秒）")
    ap.add_argument("--backoff-base", type=float, default=1.0, help="指数退避基数（秒）")
    ap.add_argument("--max-backoff", type=float, default=60.0, help="指数退避封顶（秒）")
    ap.add_argument("--skip-weekends", action=argparse.BooleanOptionalAction, default=True,
                    help="跳过周六日（美股周末休市，默认开）")
    ap.add_argument("--plan", action="store_true",
                    help="只打印拉取计划（不联网、不读 key）")
    args = ap.parse_args(argv)

    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    start = parse_date(args.start)
    end = parse_date(args.end) if args.end else dt.date.today()
    if end < start:
        raise SystemExit(f"--end（{end}）早于 --start（{start}）")
    if args.limit > MAX_LIMIT:
        raise SystemExit(f"--limit 最大 {MAX_LIMIT}（REST 上限），收到 {args.limit}")

    if args.plan:
        plan(args.root, symbols, start, end, args.skip_weekends)
        return 0

    key = load_api_key()
    headers = auth_headers(key)
    session = requests.Session()

    totals = {"fetched": 0, "skipped": 0, "empty": 0, "failed": 0}
    for ticker in symbols:
        try:
            sym_start = resolve_start_date(
                session, ticker, headers, start, limit=args.limit,
                max_retries=args.max_retries, timeout=args.timeout,
                backoff_base=args.backoff_base, max_backoff=args.max_backoff,
            )
        except Exception as e:
            log(f"[{ticker}] 起点解析失败，跳过该标的：{e}")
            totals["failed"] += 1
            continue
        log(f"[{ticker}] 起点 {sym_start} → {end}")
        stats = fetch_symbol(
            session, ticker, root=args.root, headers=headers,
            start=sym_start, end=end, skip_weekends=args.skip_weekends,
            limit=args.limit, max_retries=args.max_retries, timeout=args.timeout,
            backoff_base=args.backoff_base, max_backoff=args.max_backoff,
        )
        for k in totals:
            totals[k] += stats[k]
        log(
            f"[{ticker}] 完成：拉 {stats['fetched']} 天 / 跳过 {stats['skipped']} 天 / "
            f"空 {stats['empty']} 天 / 失败 {stats['failed']} 天"
        )
    log(
        f"全部完成：拉 {totals['fetched']} 天 / 跳过 {totals['skipped']} 天 / "
        f"空 {totals['empty']} 天 / 失败 {totals['failed']} 天"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
