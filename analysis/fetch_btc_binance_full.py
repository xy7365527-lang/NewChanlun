"""BTC/USDT 1min 全历史拉取 — Binance 公开归档（data.binance.vision）。

BTCUSDT 现货自 2017-08-17 上市。本脚本批量下载：
  - 已完结月份：monthly klines zip（每月一个）。
  - 当前未完结月份：daily klines zip（每日一个，约 T+1 可用）。

合并、按 open_time 排序去重，落盘为列式 JSON（与项目 data_cache 约定一致）：
  {opens, highs, lows, closes, volumes, dates, symbol}

BTC 24/7 连续交易，不做时段过滤。

时间戳口径：Binance 历史 klines 的 open_time 为毫秒；2025-01 起切换为微秒。
本脚本按量级自动检测（>1e14 视为微秒）归一到秒，再转 UTC 'YYYY-MM-DD HH:MM:SS'。

纯技术性产出（数据拉取/格式转换），按结果包简化版处理。
"""

from __future__ import annotations

import csv
import io
import sys
import time
import zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "analysis" / "data_cache" / "btc_1m_full.json"

SYMBOL = "BTCUSDT"
INTERVAL = "1m"
BASE = "https://data.binance.vision/data/spot"
START_YEAR, START_MONTH = 2017, 8  # BTCUSDT 现货上市 2017-08-17
MAX_WORKERS = 8


def _month_urls() -> list[tuple[str, str]]:
    """(标签, URL) 列表：从 2017-08 到上个完整月的月度归档。"""
    now = datetime.now(timezone.utc)
    cur_y, cur_m = now.year, now.month
    out: list[tuple[str, str]] = []
    y, m = START_YEAR, START_MONTH
    while (y, m) < (cur_y, cur_m):  # 严格小于当前月 → 当前月用日度
        tag = f"{y:04d}-{m:02d}"
        out.append((tag, f"{BASE}/monthly/klines/{SYMBOL}/{INTERVAL}/"
                         f"{SYMBOL}-{INTERVAL}-{tag}.zip"))
        m += 1
        if m > 12:
            m, y = 1, y + 1
    return out


def _day_urls() -> list[tuple[str, str]]:
    """当前未完结月份的日度归档（截至昨日 UTC，T+1 可用性留余量）。"""
    now = datetime.now(timezone.utc)
    first = datetime(now.year, now.month, 1, tzinfo=timezone.utc)
    last = datetime(now.year, now.month, now.day, tzinfo=timezone.utc) - timedelta(days=1)
    out: list[tuple[str, str]] = []
    d = first
    while d <= last:
        tag = d.strftime("%Y-%m-%d")
        out.append((tag, f"{BASE}/daily/klines/{SYMBOL}/{INTERVAL}/"
                         f"{SYMBOL}-{INTERVAL}-{tag}.zip"))
        d += timedelta(days=1)
    return out


def _fetch_zip(tag: str, url: str) -> tuple[str, bytes | None]:
    """下载单个 zip，返回原始字节；404（归档尚未生成）视为可跳过。"""
    for attempt in range(4):
        try:
            req = Request(url, headers={"User-Agent": "chanlun-fetch/1.0"})
            with urlopen(req, timeout=60) as resp:
                return tag, resp.read()
        except HTTPError as e:
            if e.code == 404:
                return tag, None  # 该月/日归档不存在（如当前月尚未生成 monthly）
            if attempt == 3:
                raise
            time.sleep(2 * (attempt + 1))
        except (URLError, TimeoutError):
            if attempt == 3:
                raise
            time.sleep(2 * (attempt + 1))
    return tag, None


def _parse_zip(data: bytes) -> list[tuple[int, float, float, float, float, float]]:
    """解压并解析 CSV → (open_time, o, h, l, c, v) 行列表。跳过表头行。"""
    rows: list[tuple[int, float, float, float, float, float]] = []
    with zipfile.ZipFile(io.BytesIO(data)) as zf:
        name = zf.namelist()[0]
        with zf.open(name) as fh:
            text = io.TextIOWrapper(fh, encoding="utf-8")
            for rec in csv.reader(text):
                if not rec:
                    continue
                try:
                    t = int(rec[0])
                except ValueError:
                    continue  # 表头行（'open_time' 等）
                rows.append((t, float(rec[1]), float(rec[2]),
                             float(rec[3]), float(rec[4]), float(rec[5])))
    return rows


def _ts_to_seconds(t: int) -> int:
    """归一时间戳到秒：>1e14 视为微秒，>1e11 视为毫秒，否则秒。"""
    if t > 1_000_000_000_000_00:   # >1e14 → 微秒
        return t // 1_000_000
    if t > 1_000_000_000_000:      # >1e12 → 毫秒
        return t // 1000
    return t


def main() -> None:
    jobs = _month_urls() + _day_urls()
    print(f"BTC/USDT 1min 全历史拉取：{len(jobs)} 个归档"
          f"（{jobs[0][0]} → {jobs[-1][0]}），{MAX_WORKERS} 并发", flush=True)

    raw: dict[str, list] = {}
    missing: list[str] = []
    t0 = time.time()
    done = 0
    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as ex:
        futs = {ex.submit(_fetch_zip, tag, url): tag for tag, url in jobs}
        for fut in as_completed(futs):
            tag, data = fut.result()
            done += 1
            if data is None:
                missing.append(tag)
            else:
                raw[tag] = _parse_zip(data)
            if done % 20 == 0 or done == len(jobs):
                print(f"  下载进度 {done}/{len(jobs)}（缺失 {len(missing)}，"
                      f"{time.time()-t0:.0f}s）", flush=True)

    # 合并：按 open_time 排序去重（跨月/日边界可能重叠）
    merged: dict[int, tuple] = {}
    for tag in raw:
        for row in raw[tag]:
            merged[row[0]] = row
    keys = sorted(merged)
    print(f"合并：{len(keys):,} 根唯一 bar（去重前 "
          f"{sum(len(v) for v in raw.values()):,}）", flush=True)

    opens, highs, lows, closes, volumes, dates = [], [], [], [], [], []
    for k in keys:
        _, o, h, l, c, v = merged[k]
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        volumes.append(v)
        sec = _ts_to_seconds(k)
        dates.append(datetime.fromtimestamp(sec, tz=timezone.utc)
                     .strftime("%Y-%m-%d %H:%M:%S"))

    import json
    OUT.write_text(json.dumps({
        "opens": opens, "highs": highs, "lows": lows, "closes": closes,
        "volumes": volumes, "dates": dates, "symbol": SYMBOL,
    }))
    mb = OUT.stat().st_size / 1e6
    print(f"\n✓ 落盘 {OUT}（{mb:.0f} MB）", flush=True)
    print(f"  范围：{dates[0]} → {dates[-1]}，{len(closes):,} bar", flush=True)
    if missing:
        print(f"  缺失归档（{len(missing)}，多为当前月尾部 T+1 未生成）：{missing[:10]}"
              f"{' …' if len(missing) > 10 else ''}", flush=True)


if __name__ == "__main__":
    main()
