"""Databento ES 1s 两周拉取（与 BTC 1s 同窗 2026-05-29 → 2026-06-12）。

用途：1s a0 区间套加速确认实验的 ES 腿（ES 确认滞后是核心问题）。
格式对齐 cl_1s_databento（timestamps_ns 数组）。成本探针已确认 0.0（免费档）。
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

KEY = os.environ.get("DATABENTO_KEY") or os.environ.get("DATABENTO_API_KEY")
if not KEY:
    raise RuntimeError("DATABENTO_KEY / DATABENTO_API_KEY 环境变量未设置，拒绝执行")
AUTH = base64.b64encode(f"{KEY}:".encode()).decode()
BASE = "https://hist.databento.com/v0"
OUT = Path(__file__).resolve().parent / "data_cache" / "es_1s_2week.json"

DATASET = "GLBX.MDP3"
SYMBOL = "ES.v.0"
START = "2026-05-29"
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
    "encoding": "csv", "pretty_px": "true", "compression": "none",
})
req = urllib.request.Request(
    f"{BASE}/timeseries.get_range",
    data=params.encode(),
    headers={"Authorization": f"Basic {AUTH}"},
    method="POST",
)
t0 = time.time()
buf = io.BytesIO()
with urllib.request.urlopen(req, timeout=1800) as resp:
    while True:
        chunk = resp.read(1 << 20)
        if not chunk:
            break
        buf.write(chunk)
print(f"下载 {buf.tell()/1e6:.1f} MB 用时 {time.time()-t0:.1f}s", flush=True)

buf.seek(0)
reader = csv.DictReader(io.TextIOWrapper(buf, encoding="utf-8"))
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
