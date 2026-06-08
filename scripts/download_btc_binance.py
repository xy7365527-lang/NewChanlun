"""从 Binance 数据归档下载 BTCUSDT 1min K线并合并为 columnar JSON。

数据源: https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1m/
每月一个 zip，解压后是 CSV（12列，无header）。

CSV 列: open_time, open, high, low, close, volume,
        close_time, quote_volume, count, taker_buy_base,
        taker_buy_quote, ignore
"""

from __future__ import annotations

import csv
import io
import json
import sys
import zipfile
from datetime import datetime
from pathlib import Path
from urllib.request import urlretrieve

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT = DATA_DIR / "btc_1m_3year.json"

START_YEAR, START_MONTH = 2023, 1
END_YEAR, END_MONTH = 2026, 5

BASE_URL = (
    "https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1m"
)


def month_range(
    sy: int, sm: int, ey: int, em: int,
) -> list[tuple[int, int]]:
    result: list[tuple[int, int]] = []
    y, m = sy, sm
    while (y, m) <= (ey, em):
        result.append((y, m))
        m += 1
        if m > 12:
            m = 1
            y += 1
    return result


def download_month(year: int, month: int) -> list[list[str]]:
    fname = f"BTCUSDT-1m-{year}-{month:02d}.zip"
    url = f"{BASE_URL}/{fname}"
    tmp = DATA_DIR / fname

    print(f"  下载 {fname} ... ", end="", flush=True)
    try:
        urlretrieve(url, tmp)
    except Exception as e:
        print(f"FAILED: {e}")
        return []

    rows: list[list[str]] = []
    with zipfile.ZipFile(tmp) as zf:
        for name in zf.namelist():
            if name.endswith(".csv"):
                with zf.open(name) as f:
                    reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8"))
                    for row in reader:
                        if len(row) >= 6:
                            rows.append(row)

    tmp.unlink(missing_ok=True)
    print(f"{len(rows):,} bars")
    return rows


def main() -> None:
    months = month_range(START_YEAR, START_MONTH, END_YEAR, END_MONTH)
    print(f"下载范围: {START_YEAR}-{START_MONTH:02d} → {END_YEAR}-{END_MONTH:02d}")
    print(f"共 {len(months)} 个月\n")

    all_rows: list[list[str]] = []
    for y, m in months:
        rows = download_month(y, m)
        all_rows.extend(rows)

    if not all_rows:
        print("未获取到数据")
        sys.exit(1)

    def _normalize_ts(raw: int) -> int:
        return raw // 1000 if raw > 9_999_999_999_999 else raw

    all_rows.sort(key=lambda r: _normalize_ts(int(r[0])))

    seen: set[int] = set()
    deduped: list[list[str]] = []
    for r in all_rows:
        ts = _normalize_ts(int(r[0]))
        if ts not in seen:
            seen.add(ts)
            deduped.append(r)

    print(f"\n去重后: {len(deduped):,} bars")

    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    volumes: list[float] = []
    dates: list[str] = []

    from datetime import timezone
    ts_min = datetime(START_YEAR, START_MONTH, 1,
                       tzinfo=timezone.utc).timestamp() * 1000
    if END_MONTH < 12:
        ts_max = datetime(END_YEAR, END_MONTH + 1, 1,
                           tzinfo=timezone.utc).timestamp() * 1000
    else:
        ts_max = datetime(END_YEAR + 1, 1, 1,
                           tzinfo=timezone.utc).timestamp() * 1000

    for r in deduped:
        ts_raw = int(r[0])
        ts_ms = ts_raw // 1000 if ts_raw > 9_999_999_999_999 else ts_raw
        if ts_ms < ts_min or ts_ms >= ts_max:
            continue
        dt = datetime.fromtimestamp(ts_ms / 1000, tz=timezone.utc)
        opens.append(float(r[1]))
        highs.append(float(r[2]))
        lows.append(float(r[3]))
        closes.append(float(r[4]))
        volumes.append(float(r[5]))
        dates.append(dt.strftime("%Y-%m-%d %H:%M:%S"))

    data = {
        "opens": opens,
        "highs": highs,
        "lows": lows,
        "closes": closes,
        "volumes": volumes,
        "dates": dates,
        "symbol": "BTCUSDT",
    }

    OUTPUT.write_text(json.dumps(data))
    size_mb = OUTPUT.stat().st_size / 1024 / 1024
    print(f"\n保存到: {OUTPUT}")
    print(f"大小: {size_mb:.1f} MB")
    print(f"范围: {dates[0]} → {dates[-1]}")
    print(f"总 bars: {len(closes):,}")


if __name__ == "__main__":
    main()
