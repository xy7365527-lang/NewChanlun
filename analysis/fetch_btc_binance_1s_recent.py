"""BTC/USDT 1s 近两周拉取 — Binance 公开归档（data.binance.vision）。

复用 fetch_btc_binance_full.py 的下载/解析管线，差异：
  - interval = 1s（Binance 现货 1s klines 归档自 2023 起提供）；
  - 只取最近 N_DAYS 个日度归档（T+1 可用，截至昨日 UTC）；
  - 落盘 btc_1s_2week.json（列式 JSON，与项目 data_cache 约定一致）。

BTC 24/7 连续交易，不做时段过滤。1s 归档允许有缺秒（无成交秒不产 bar），
缺秒不填充——引擎按事件序列消费，与 1min 管线口径一致。

纯技术性产出（数据拉取/格式转换），按结果包简化版处理：
  结论：近 14 天 1s 列式 JSON 落盘。
  边界条件：今日 UTC 归档未生成（T+1）则终止日为昨日；若某日 404 记入 missing。
  影响声明：新增 data_cache/btc_1s_2week.json，不触碰任何已有数据/引擎。
"""

from __future__ import annotations

import json
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "analysis"))

from fetch_btc_binance_full import _fetch_zip, _parse_zip, _ts_to_seconds  # noqa: E402

OUT = ROOT / "analysis" / "data_cache" / "btc_1s_2week.json"
SYMBOL = "BTCUSDT"
INTERVAL = "1s"
BASE = "https://data.binance.vision/data/spot"
N_DAYS = 14
MAX_WORKERS = 8


def _day_urls() -> list[tuple[str, str]]:
    """最近 N_DAYS 个日度归档（截至昨日 UTC，T+1 可用性留余量）。"""
    now = datetime.now(timezone.utc)
    last = datetime(now.year, now.month, now.day, tzinfo=timezone.utc) - timedelta(days=1)
    out: list[tuple[str, str]] = []
    for k in range(N_DAYS - 1, -1, -1):
        d = last - timedelta(days=k)
        tag = d.strftime("%Y-%m-%d")
        out.append((tag, f"{BASE}/daily/klines/{SYMBOL}/{INTERVAL}/"
                         f"{SYMBOL}-{INTERVAL}-{tag}.zip"))
    return out


def main() -> None:
    jobs = _day_urls()
    print(f"BTC/USDT 1s 近两周拉取：{len(jobs)} 个日度归档"
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
            print(f"  {tag}: {'缺失' if data is None else f'{len(raw[tag]):,} 行'}"
                  f"（{done}/{len(jobs)}，{time.time()-t0:.0f}s）", flush=True)

    if missing:
        print(f"⚠ 缺失归档：{sorted(missing)}", flush=True)
    if not raw:
        raise SystemExit("全部归档缺失——1s 数据不可用，需退 1min 口径")

    merged: dict[int, tuple] = {}
    for tag in raw:
        for row in raw[tag]:
            merged[row[0]] = row
    keys = sorted(merged)
    print(f"合并：{len(keys):,} 根唯一 bar（理论满秒 {N_DAYS * 86400:,}）", flush=True)

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

    OUT.write_text(json.dumps({
        "opens": opens, "highs": highs, "lows": lows, "closes": closes,
        "volumes": volumes, "dates": dates, "symbol": SYMBOL,
        "interval": INTERVAL, "missing_days": sorted(missing),
    }))
    mb = OUT.stat().st_size / 1e6
    print(f"\n✓ 落盘 {OUT}（{mb:.0f} MB）", flush=True)
    print(f"  范围：{dates[0]} → {dates[-1]}，{len(closes):,} bar", flush=True)


if __name__ == "__main__":
    main()
