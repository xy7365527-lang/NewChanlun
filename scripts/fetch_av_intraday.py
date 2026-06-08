#!/usr/bin/env python3
"""Alpha Vantage 分钟数据增量拉取 — QQQ 5min 长历史拼接。

存在论位置 / 信息增量
---------------------
yfinance 5min 只回溯 60 天（~4680 根），不足以涌现 L2（递归主级别）。
本脚本用 Alpha Vantage TIME_SERIES_INTRADAY 的 month=YYYY-MM 参数逐月拉取，
拼接出多年 5min 历史（含盘前盘后 extended hours，~192 根/交易日），
为 RecursiveOrchestrator 的自下而上递归提供足够 bar 数。

配额纪律（强制）
----------------
AV 免费版 25 次/天。每个月份 = 1 次调用。本脚本：
1. 每月原始响应落盘到 analysis/data_cache/av_raw/QQQ_<interval>_<month>.json；
2. 已存在且非空的月份缓存**跳过**，绝不重复消耗配额；
3. 检测限流响应（"Note"/"Information" 字段）→ 立即停止，不计入已拉月份；
4. --probe 只拉最近 1 个月验证连通性与格式。

输出
----
拼接缓存 analysis/data_cache/qqq_<interval>_av.json，格式与 load_bars 兼容：
    {"symbol", "interval", "bars": [{"ts"(isoformat), "open","high","low","close","volume"}]}

用法
----
    .venv/bin/python scripts/fetch_av_intraday.py --probe                 # 验证连通性（1 次调用）
    .venv/bin/python scripts/fetch_av_intraday.py --months 24             # 拉最近 24 个月
    .venv/bin/python scripts/fetch_av_intraday.py --months 24 --merge-only # 仅从已有月缓存拼接，0 调用

约束：.venv/bin/python；AV 25 次/天精打细算；不交互提问。
"""
from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "analysis" / "data_cache"
RAW = DATA / "av_raw"
AV_URL = "https://www.alphavantage.co/query"


def _load_api_key() -> str:
    """从 .env.local 读取 ALPHA_VANTAGE_API_KEY。"""
    env = ROOT / ".env.local"
    if not env.exists():
        sys.exit("[✗] .env.local 不存在")
    for line in env.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("ALPHA_VANTAGE_API_KEY="):
            return line.split("=", 1)[1].strip()
    sys.exit("[✗] .env.local 中未找到 ALPHA_VANTAGE_API_KEY")


def _recent_months(n: int) -> list[str]:
    """生成最近 n 个月的 YYYY-MM 列表（从最近往前），无 Date.now 依赖：
    用 currentDate 锚点 2026-05。倒序拉，便于增量。"""
    # 锚点年月（项目 currentDate=2026-05-31）
    y, m = 2026, 5
    out: list[str] = []
    for _ in range(n):
        out.append(f"{y:04d}-{m:02d}")
        m -= 1
        if m == 0:
            m = 12
            y -= 1
    return out


def _raw_path(interval: str, month: str) -> Path:
    return RAW / f"QQQ_{interval}_{month}.json"


def _fetch_month(symbol: str, interval: str, month: str, api_key: str) -> dict:
    """拉取单月 intraday 数据。返回 AV 原始 JSON。"""
    params = {
        "function": "TIME_SERIES_INTRADAY",
        "symbol": symbol,
        "interval": interval,
        "month": month,
        "outputsize": "full",
        "extended_hours": "true",
        "datatype": "json",
        "apikey": api_key,
    }
    url = f"{AV_URL}?{urllib.parse.urlencode(params)}"
    with urllib.request.urlopen(url, timeout=60) as resp:  # noqa: S310 (固定 AV 域名)
        return json.loads(resp.read().decode("utf-8"))


def _is_throttled(payload: dict) -> str | None:
    """检测 AV 限流/错误响应。返回提示文本（若被限流）或 None。"""
    for key in ("Note", "Information", "Error Message"):
        if key in payload:
            return f"{key}: {payload[key]}"
    return None


def _series_key(interval: str) -> str:
    return f"Time Series ({interval})"


def fetch_months(
    symbol: str, interval: str, months: list[str], api_key: str, max_calls: int,
) -> tuple[list[str], int]:
    """逐月拉取（带缓存）。返回 (成功就绪的月份列表, 实际调用次数)。"""
    RAW.mkdir(parents=True, exist_ok=True)
    ready: list[str] = []
    calls = 0
    for month in months:
        rp = _raw_path(interval, month)
        if rp.exists():
            try:
                cached = json.loads(rp.read_text(encoding="utf-8"))
                if _series_key(interval) in cached:
                    ready.append(month)
                    print(f"    [cache] {month} 已缓存，跳过", flush=True)
                    continue
            except json.JSONDecodeError:
                pass  # 缓存损坏，重拉

        if calls >= max_calls:
            print(f"    [stop] 达到本次最大调用上限 {max_calls}，剩余月份留待下次", flush=True)
            break

        print(f"    [fetch] {month} ...", flush=True)
        payload = _fetch_month(symbol, interval, month, api_key)
        calls += 1
        throttle = _is_throttled(payload)
        if throttle:
            print(f"    [throttled] {throttle}", flush=True)
            print("    [stop] AV 限流，停止拉取（不缓存限流响应）", flush=True)
            break
        if _series_key(interval) not in payload:
            print(f"    [warn] {month} 响应无时间序列字段，keys={list(payload.keys())}", flush=True)
            break
        rp.write_text(json.dumps(payload), encoding="utf-8")
        n = len(payload[_series_key(interval)])
        print(f"    [ok] {month} → {n} 根，缓存写入", flush=True)
        ready.append(month)
        time.sleep(1.0)  # 温和限速
    return ready, calls


def merge_months(interval: str, months: list[str], symbol: str = "QQQ") -> Path:
    """从已缓存月份拼接为统一 bars JSON（去重、按时间升序）。"""
    series_key = _series_key(interval)
    by_ts: dict[str, dict] = {}
    used = 0
    for month in months:
        rp = _raw_path(interval, month)
        if not rp.exists():
            continue
        payload = json.loads(rp.read_text(encoding="utf-8"))
        series = payload.get(series_key, {})
        if not series:
            continue
        used += 1
        for ts_str, ohlc in series.items():
            # AV intraday ts 形如 "2026-05-29 19:55:00"（美东时间 naive）
            iso = ts_str.replace(" ", "T")
            by_ts[iso] = {
                "ts": iso,
                "open": float(ohlc["1. open"]),
                "high": float(ohlc["2. high"]),
                "low": float(ohlc["3. low"]),
                "close": float(ohlc["4. close"]),
                "volume": float(ohlc["5. volume"]),
            }
    bars = [by_ts[k] for k in sorted(by_ts.keys())]
    out = {
        "symbol": symbol,
        "interval": interval,
        "source": "alpha_vantage",
        "months_used": used,
        "bar_count": len(bars),
        "bars": bars,
    }
    out_path = DATA / f"qqq_{interval.replace('min', 'm')}_av.json"
    out_path.write_text(json.dumps(out), encoding="utf-8")
    span = f"{bars[0]['ts']} ~ {bars[-1]['ts']}" if bars else "（空）"
    print(f"[✓] 拼接 {used} 个月 → {len(bars)} 根，{span}")
    print(f"[✓] 写入 {out_path}")
    return out_path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbol", default="QQQ")
    ap.add_argument("--interval", default="5min", choices=["1min", "5min", "15min", "30min", "60min"])
    ap.add_argument("--months", type=int, default=24, help="拉取最近 N 个月")
    ap.add_argument("--max-calls", type=int, default=25, help="本次最大 API 调用次数（配额保护）")
    ap.add_argument("--probe", action="store_true", help="只拉最近 1 个月验证连通性")
    ap.add_argument("--merge-only", action="store_true", help="仅从已有缓存拼接，0 调用")
    args = ap.parse_args()

    api_key = _load_api_key()
    n_months = 1 if args.probe else args.months
    months = _recent_months(n_months)
    print(f"[*] 目标：{args.symbol} {args.interval}，最近 {n_months} 个月：{months[-1]} ~ {months[0]}", flush=True)

    if args.merge_only:
        merge_months(args.interval, months, symbol=args.symbol)
        return

    max_calls = 1 if args.probe else args.max_calls
    ready, calls = fetch_months(args.symbol, args.interval, months, api_key, max_calls)
    print(f"[*] 本次实际调用 {calls} 次，就绪月份 {len(ready)}/{len(months)}", flush=True)

    if ready:
        merge_months(args.interval, months, symbol=args.symbol)


if __name__ == "__main__":
    main()
