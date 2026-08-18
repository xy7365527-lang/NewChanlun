#!/usr/bin/env python3
"""Massive 个股线全史 tick 拉取管线（issue #1040，ADR 0023 §四-3）——薄 CLI + 起点解析。

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

传输 + 落盘政策（重试/退避/分页/续拉/落盘）在 `massive_fetch.py` 政策模块
（#1068 / #1063 裁定）；本脚本只留 CLI（main/plan）+ 起点解析。

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
import sys
from pathlib import Path

import requests
from dotenv import load_dotenv

from massive_fetch import (
    BASE_URL,
    DEFAULT_LIMIT,
    MAX_LIMIT,
    fetch_day,
    fetch_symbol,
    http_get_json,
    iter_dates,
    log,
    partition_done,
    partition_path,
)

_ROOT = Path(__file__).resolve().parents[1]

FLOOR_DATE = dt.date(2003, 9, 10)
DEFAULT_SYMBOLS = [
    "AAPL", "AMZN", "COST", "GOOGL", "LLY",
    "META", "MSFT", "NFLX", "NVDA", "TSLA",
]
# 上市日晚于数据集起点（2003-09-10），起点需联网解析的三只。
LATE_LISTED = frozenset({"GOOGL", "META", "TSLA"})
DEFAULT_ROOT = Path("analysis/data_cache/massive_tick")


def parse_date(s: str) -> dt.date:
    try:
        return dt.date.fromisoformat(s)
    except ValueError as e:
        raise SystemExit(f"非法日期 {s!r}（应为 YYYY-MM-DD）") from e


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
    failed_days: list[tuple[str, str]] = []
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
        failed_days.extend(stats["failed_days"])
        log(
            f"[{ticker}] 完成：拉 {stats['fetched']} 天 / 跳过 {stats['skipped']} 天 / "
            f"空 {stats['empty']} 天 / 失败 {stats['failed']} 天"
        )
    log(
        f"全部完成：拉 {totals['fetched']} 天 / 跳过 {totals['skipped']} 天 / "
        f"空 {totals['empty']} 天 / 失败 {totals['failed']} 天"
    )
    if failed_days:
        log(f"失败日清单（{len(failed_days)} 条，单日失败=续拉状态，退出码 0）：")
        for date_str, err in failed_days:
            log(f"  - {date_str}: {err}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
