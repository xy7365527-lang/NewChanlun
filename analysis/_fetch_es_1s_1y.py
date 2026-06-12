"""Databento ES 1s 一年拉取（2025-06-12 → 2026-06-12，含 2026-02/04 回调段）。

用途：1s a0 nest 覆盖率实验的 ES 决定性窗口（两周窗口无深回调 regime，
P1「ES 做空翻正」需要含回调段的长窗口——1s_confirmation_acceleration.md §3）。
成本探针已确认 0.0（免费档），record_count = 11,765,833。
格式对齐 _fetch_es_1s_2week.py（timestamps_ns 数组）。
"""
import base64
import csv
import io
import json
import math
import os
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

KEY = os.environ.get("DATABENTO_KEY", "db-4Cxk3Q35QPXqFFj8snSbqENcwqr4G")
AUTH = base64.b64encode(f"{KEY}:".encode()).decode()
BASE = "https://hist.databento.com/v0"
OUT = Path(__file__).resolve().parent / "data_cache" / "es_1s_databento_1y.json"

DATASET = "GLBX.MDP3"
SYMBOL = "ES.v.0"
START = "2025-06-12"
END = "2026-06-12"


def get(path: str, **params) -> object:
    qs = urllib.parse.urlencode(params)
    req = urllib.request.Request(
        f"{BASE}/{path}?{qs}", headers={"Authorization": f"Basic {AUTH}"}
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read())


count = get(
    "metadata.get_record_count",
    dataset=DATASET, symbols=SYMBOL, stype_in="continuous",
    schema="ohlcv-1s", start=START, end=END,
)
print(f"record_count {SYMBOL} {START}..{END}: {count}", flush=True)
if not isinstance(count, int) or count == 0:
    sys.exit(f"记录数为零或异常：{count}——符号解析可能失败，停止下载")

params = urllib.parse.urlencode({
    "dataset": DATASET, "symbols": SYMBOL, "stype_in": "continuous",
    "schema": "ohlcv-1s", "start": START, "end": END,
    "encoding": "csv", "pretty_px": "true", "compression": "zstd",
})
req = urllib.request.Request(
    f"{BASE}/timeseries.get_range",
    data=params.encode(),
    headers={"Authorization": f"Basic {AUTH}"},
    method="POST",
)
t0 = time.time()
buf = io.BytesIO()
with urllib.request.urlopen(req, timeout=3600) as resp:
    while True:
        chunk = resp.read(1 << 22)
        if not chunk:
            break
        buf.write(chunk)
        if buf.tell() % (1 << 26) < (1 << 22):
            print(f"  下载中 {buf.tell()/1e6:.0f} MB "
                  f"{time.time()-t0:.0f}s", flush=True)
print(f"下载 {buf.tell()/1e6:.1f} MB 用时 {time.time()-t0:.1f}s", flush=True)

buf.seek(0)
import zstandard  # noqa: E402

reader = csv.DictReader(io.TextIOWrapper(
    zstandard.ZstdDecompressor().stream_reader(buf), encoding="utf-8"))
ts, opens, highs, lows, closes, vols = [], [], [], [], [], []
dropped = 0
for row in reader:
    o, h, l, c = (float(row["open"]), float(row["high"]),
                  float(row["low"]), float(row["close"]))
    if any(math.isnan(x) for x in (o, h, l, c)):
        dropped += 1
        continue
    ts.append(int(row["ts_event"]))
    opens.append(o)
    highs.append(h)
    lows.append(l)
    closes.append(c)
    vols.append(int(row["volume"]))

OUT.write_text(json.dumps({
    "symbol": "ES", "schema": "ohlcv-1s", "dataset": DATASET,
    "stype": "continuous(.v.0)", "start": START, "end": END,
    "timestamps_ns": ts, "opens": opens, "highs": highs,
    "lows": lows, "closes": closes, "volumes": vols,
}))
print(f"写入 {OUT.name}: {len(closes)} bars (dropped {dropped} nan), "
      f"{OUT.stat().st_size/1e6:.1f} MB", flush=True)
