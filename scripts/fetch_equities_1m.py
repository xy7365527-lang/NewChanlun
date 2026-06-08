#!/usr/bin/env python3
"""Databento XNAS.ITCH 股票 1min 抓取（ES 成分股，纵向圈闭合实测）。

派生自 scripts/fetch_1m_databento_10y.py，但目标是**股票**（equities）而非期货：
  1. 数据集 XNAS.ITCH（Nasdaq），ohlcv-1m 自 2018-05-01；成本 $0（订阅覆盖，已 get_cost 验证）。
  2. stype_in="raw_symbol"（股票用原始 ticker，非期货的 continuous 展期符号）。
  3. 输出 `{ticker}_1m_xnas.json`，列式格式与 es_1m_databento_10y.json 完全一致
     （{symbol,dates,opens,highs,lows,closes,volumes}），供 k4_1min_lib.load_series 直接消费。
  4. 按年分段 get_range（控制单请求大小），逐年拼接。

选标的：Magnificent 7（标普500 最大权重 Nasdaq 成分，约 31% 指数权重，宏观趋势主驱动力）。
  ES = 标普500 E-mini 是 P 顶点；这 7 只是 P 的（部分）分解成分。

用法：
  PYTHONPATH=src python scripts/fetch_equities_1m.py AAPL MSFT NVDA AMZN META GOOGL TSLA
  PYTHONPATH=src python scripts/fetch_equities_1m.py all
输出：analysis/data_cache/{ticker}_1m_xnas.json
"""

from __future__ import annotations

import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_ROOT / "src"))

from dotenv import load_dotenv

load_dotenv(_ROOT / ".env")

import databento as db

CACHE_DIR = _ROOT / "analysis" / "data_cache"
DATASET = "XNAS.ITCH"
TARGET_START = "2018-05-01"   # XNAS.ITCH ohlcv-1m 实际起点

# Magnificent 7（标普500 最大权重 Nasdaq 成分）
MAG7 = ["AAPL", "MSFT", "NVDA", "AMZN", "META", "GOOGL", "TSLA"]


def _client() -> db.Historical:
    key = os.environ.get("DATABENTO_API_KEY")
    if not key:
        raise SystemExit("DATABENTO_API_KEY 缺失（检查 .env）")
    return db.Historical(key=key)


def _now_utc() -> datetime:
    return datetime.now(timezone.utc)


def _year_windows(start: str, end_dt: datetime) -> list[tuple[str, str]]:
    s = datetime.strptime(start, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    out = []
    while s < end_dt:
        nxt = datetime(s.year + 1, 1, 1, tzinfo=timezone.utc)
        e = min(nxt, end_dt)
        out.append((s.strftime("%Y-%m-%d"), e.strftime("%Y-%m-%d")))
        s = nxt
    return out


def fetch_one(ticker: str, client: db.Historical, start: str = TARGET_START) -> Path | None:
    try:
        drange = client.metadata.get_dataset_range(dataset=DATASET)
        ds_end = datetime.strptime(drange["end"][:10], "%Y-%m-%d").replace(tzinfo=timezone.utc)
    except Exception:  # noqa: BLE001
        ds_end = _now_utc()
    end_dt = min(_now_utc(), ds_end)
    out_path = CACHE_DIR / f"{ticker.lower()}_1m_xnas.json"

    windows = _year_windows(start, end_dt)
    print(f"\n[{ticker}] {DATASET} ohlcv-1m [{start} → {end_dt.date()}] 分年", flush=True)

    rows: dict[str, list] = {}
    for ws, we in windows:
        df = None
        for attempt in range(3):
            try:
                data = client.timeseries.get_range(
                    dataset=DATASET, symbols=[ticker],
                    stype_in="raw_symbol", schema="ohlcv-1m", start=ws, end=we,
                )
                df = data.to_df()
                break
            except Exception as e:  # noqa: BLE001
                msg = repr(e)[:140]
                if "data_start_before_available" in msg or "unavailable_range" in msg:
                    print(f"  [{ws}~{we}] 跳过（超出可用范围）：{msg}", flush=True)
                    break
                print(f"  [{ws}~{we}] 重试 {attempt+1}/3：{msg}", flush=True)
                time.sleep(5)
        if df is None or df.empty:
            print(f"  [{ws}~{we}] 空", flush=True)
            continue
        df = df.sort_index()
        df = df[~df.index.duplicated(keep="last")]
        o = df["open"].astype(float).tolist()
        h = df["high"].astype(float).tolist()
        lo = df["low"].astype(float).tolist()
        c = df["close"].astype(float).tolist()
        v = df["volume"].astype("int64").tolist()
        for i, ts in enumerate(df.index):
            rows[str(ts)] = [o[i], h[i], lo[i], c[i], v[i]]
        print(f"  [{ws}~{we}] +{len(df)} 累计 {len(rows)}", flush=True)

    if not rows:
        print(f"[{ticker}] ✗ 无数据", flush=True)
        return None

    ordered = sorted(rows.items())
    payload = {
        "symbol": ticker,
        "dates": [d for d, _ in ordered],
        "opens": [r[0] for _, r in ordered],
        "highs": [r[1] for _, r in ordered],
        "lows": [r[2] for _, r in ordered],
        "closes": [r[3] for _, r in ordered],
        "volumes": [r[4] for _, r in ordered],
    }
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(payload, f)
    size_mb = out_path.stat().st_size / 1024 / 1024
    print(f"[{ticker}] ✓ {len(ordered)} 条 | {payload['dates'][0]} → "
          f"{payload['dates'][-1]} | {out_path.name} ({size_mb:.1f}MB)", flush=True)
    return out_path


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    args = [a.upper() for a in sys.argv[1:]]
    tickers = MAG7 if args == ["ALL"] else args
    client = _client()
    summary = []
    for t in tickers:
        try:
            p = fetch_one(t, client)
            summary.append(f"{t}: {'✓ ' + p.name if p else '✗ 无数据'}")
        except Exception as e:  # noqa: BLE001
            print(f"[{t}] ✗ {e!r}", flush=True)
            summary.append(f"{t}: ✗ {repr(e)[:80]}")
    print("\n=== 汇总 ===")
    for s in summary:
        print("  " + s)
    return 0


if __name__ == "__main__":
    sys.exit(main())
