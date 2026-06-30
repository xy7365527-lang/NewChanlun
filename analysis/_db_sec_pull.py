"""Databento 秒级试水拉取：CL ohlcv-1s 1个月 → data_cache JSON（对齐 databento_10y 格式）。

非常规时段囊括：ohlcv-1s 返回 Globex 全时段（每个有成交的秒一条），无 RTH 过滤。
"""
import base64
import csv
import datetime
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
OUT = Path(__file__).resolve().parent / "data_cache" / "cl_1s_databento_1mo.json"

DATASET = "GLBX.MDP3"
SYMBOL = "CL.v.0"
START = "2025-04-01"
END = "2025-05-01"


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

# 流式下载 CSV（无压缩，~count×60B）
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
ts, dates, opens, highs, lows, closes, vols = [], [], [], [], [], [], []
dropped = 0
for row in reader:
    o, h, l, c = (float(row["open"]), float(row["high"]),
                  float(row["low"]), float(row["close"]))
    if any(math.isnan(x) for x in (o, h, l, c)):
        dropped += 1
        continue
    tns = int(row["ts_event"])
    ts.append(tns)
    # theta_v0 加载器（data.rs:47/162）要 ISO 含秒串：dates[..10] 切日期窗，
    # 全串解析 14 位 YYYYMMDDHHMMSS 排序。秒位区分同分钟 60 根 1s bar（B 点）。
    dates.append(datetime.datetime.fromtimestamp(
        tns / 1e9, tz=datetime.timezone.utc).strftime("%Y-%m-%d %H:%M:%S"))
    opens.append(o)
    highs.append(h)
    lows.append(l)
    closes.append(c)
    vols.append(int(row["volume"]))

OUT.write_text(json.dumps({
    "symbol": "CL", "schema": "ohlcv-1s", "dataset": DATASET,
    "stype": "continuous(.v.0)", "start": START, "end": END,
    "timestamps_ns": ts, "dates": dates, "opens": opens, "highs": highs,
    "lows": lows, "closes": closes, "volumes": vols,
}))
print(f"写入 {OUT.name}: {len(closes)} bars (dropped {dropped} nan), "
      f"{OUT.stat().st_size/1e6:.1f} MB", flush=True)
