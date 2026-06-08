#!/usr/bin/env python3
"""通过 TWS / IB Gateway 拉取 1min 历史数据（K4 顶点/折叠通道补缺）。

输出格式与 analysis/data_cache/es_1m_databento.json **完全一致**（列式）：
    {symbol, dates, opens, highs, lows, closes, volumes}
dates 为 UTC tz-aware 字符串（"YYYY-MM-DD HH:MM:SS+00:00"），与现有 databento
文件对齐，可被 K4 管线统一消费。

策略：按 1 个月一块向后回溯，直到到达 --start 截止日或 TWS 无数据。每块暂停
PAUSE_SECONDS 避免 pacing violation。支持增量（已有文件则只补最早 bar 之前）。

用法：
    PYTHONPATH=src python scripts/tws/fetch_1m_tws.py VNQ [start] [secType] [exchange] [currency]
  start 默认 2024-01-01（与 es/gc/cl databento 1m 对齐）。
  US ETF 默认：secType=STK exchange=SMART currency=USD。
  例：fetch_1m_tws.py VNQ 2024-01-01
      fetch_1m_tws.py IYR 2024-01-01

输出文件：analysis/data_cache/<symbol_lower>_1m_tws.json
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from ib_insync import IB, Stock, util  # noqa: F401

_ROOT = Path(__file__).resolve().parents[2]
CACHE_DIR = _ROOT / "analysis" / "data_cache"

PAUSE_SECONDS = 12
MAX_CHUNKS = 60          # 1 月/块 → 最多覆盖 ~5 年
DEFAULT_START = "2024-01-01"
PORTS = [7496, 7497, 4001, 4002]
CLIENT_ID = 217


def connect_tws() -> IB:
    ib = IB()
    last_err: Exception | None = None
    for port in PORTS:
        try:
            ib.connect("127.0.0.1", port, clientId=CLIENT_ID, timeout=15)
            print(f"已连接 TWS 端口 {port}", flush=True)
            return ib
        except Exception as e:  # noqa: BLE001
            last_err = e
            print(f"端口 {port} 失败: {e}", flush=True)
    raise ConnectionError(f"无法连接 TWS/IB Gateway（任何端口）：{last_err!r}")


def _to_utc_str(bar_date) -> str:
    """把 ib_insync bar.date 规整为 UTC tz-aware 字符串，与 databento 对齐。"""
    dt = bar_date
    if not isinstance(dt, datetime):
        # 日线返回 date；本脚本只拉 1min，理论不会走到这里
        dt = datetime(dt.year, dt.month, dt.day, tzinfo=timezone.utc)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    else:
        dt = dt.astimezone(timezone.utc)
    return dt.strftime("%Y-%m-%d %H:%M:%S+00:00")


def load_existing(out_path: Path) -> dict[str, list]:
    """加载已有列式文件 → 行式 dict（date→bar），用于增量去重。"""
    if not out_path.exists():
        return {}
    with open(out_path) as f:
        d = json.load(f)
    rows: dict[str, list] = {}
    for i, dt in enumerate(d["dates"]):
        rows[dt] = [d["opens"][i], d["highs"][i], d["lows"][i], d["closes"][i], d["volumes"][i]]
    print(f"已有数据: {len(rows)} bars, {d['dates'][0]} ~ {d['dates'][-1]}", flush=True)
    return rows


def _save_columnar(symbol: str, rows: dict[str, list], out_path: Path) -> None:
    """行式 dict → 列式 databento 格式，按 date 升序写出。"""
    ordered = sorted(rows.items())
    payload = {
        "symbol": symbol,
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


def fetch(
    ib: IB,
    symbol: str,
    sec_type: str,
    exchange: str,
    currency: str,
    start: str,
    out_path: Path,
) -> dict[str, list]:
    if sec_type != "STK":
        raise ValueError(f"本脚本目前仅支持 STK（ETF/股票）；收到 secType={sec_type}")
    contract = Stock(symbol, exchange, currency)
    qualified = ib.qualifyContracts(contract)
    if not qualified:
        raise ValueError(f"无法 qualify 合约 {symbol} {exchange} {currency}")
    contract = qualified[0]
    print(f"合约: {contract}", flush=True)

    rows = load_existing(out_path)
    start_dt = datetime.strptime(start, "%Y-%m-%d").replace(tzinfo=timezone.utc)

    if rows:
        earliest = min(rows.keys())
        end_dt = datetime.strptime(earliest[:19], "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
        print(f"增量模式: 从 {end_dt} 向前", flush=True)
    else:
        end_dt = datetime.now(timezone.utc)
        print(f"全量模式: 从 {end_dt} 向前到 {start_dt}", flush=True)

    empty_streak = 0
    for chunk_idx in range(MAX_CHUNKS):
        if end_dt <= start_dt:
            print(f"已到达 start 截止 {start}，停止。", flush=True)
            break

        end_str = end_dt.strftime("%Y%m%d-%H:%M:%S")
        print(f"\nChunk {chunk_idx + 1}/{MAX_CHUNKS}, endDateTime={end_str} (UTC)", flush=True)
        try:
            bars = ib.reqHistoricalData(
                contract,
                endDateTime=end_str,
                durationStr="1 M",
                barSizeSetting="1 min",
                whatToShow="TRADES",
                useRTH=True,
                formatDate=2,  # UTC
            )
        except Exception as e:  # noqa: BLE001
            msg = str(e).lower()
            print(f"请求错误: {e}", flush=True)
            if "pacing" in msg:
                print("Pacing violation，等待 60s 重试…", flush=True)
                time.sleep(60)
                continue
            if "no data" in msg or "invalid" in msg:
                print("该时段无数据，停止。", flush=True)
                break
            print("未知错误，停止。", flush=True)
            break

        if not bars:
            empty_streak += 1
            print(f"空响应 (streak={empty_streak})", flush=True)
            if empty_streak >= 2:
                print("连续两次空响应，到达数据上限。", flush=True)
                break
            end_dt -= timedelta(days=30)
            time.sleep(PAUSE_SECONDS)
            continue

        empty_streak = 0
        new_count = 0
        earliest_bar: datetime | None = None
        for bar in bars:
            dt_str = _to_utc_str(bar.date)
            if dt_str not in rows:
                rows[dt_str] = [
                    float(bar.open), float(bar.high), float(bar.low),
                    float(bar.close), int(bar.volume) if bar.volume >= 0 else 0,
                ]
                new_count += 1
            bdt = bar.date if isinstance(bar.date, datetime) else None
            if bdt is not None:
                if bdt.tzinfo is None:
                    bdt = bdt.replace(tzinfo=timezone.utc)
                if earliest_bar is None or bdt < earliest_bar:
                    earliest_bar = bdt

        print(f"  收到 {len(bars)} bars, {new_count} 新. 最早: {earliest_bar}", flush=True)
        print(f"  累计: {len(rows)} bars", flush=True)

        if new_count == 0:
            print("无新 bar，到达重叠上限。", flush=True)
            break
        if earliest_bar is None:
            break

        end_dt = earliest_bar
        if chunk_idx < MAX_CHUNKS - 1:
            time.sleep(PAUSE_SECONDS)

        if (chunk_idx + 1) % 10 == 0:
            _save_columnar(symbol, rows, out_path)
            print(f"  [checkpoint] 已存 {len(rows)} bars → {out_path}", flush=True)

    _save_columnar(symbol, rows, out_path)
    return rows


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    symbol = sys.argv[1].upper()
    start = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_START
    sec_type = sys.argv[3] if len(sys.argv) > 3 else "STK"
    exchange = sys.argv[4] if len(sys.argv) > 4 else "SMART"
    currency = sys.argv[5] if len(sys.argv) > 5 else "USD"

    out_path = CACHE_DIR / f"{symbol.lower()}_1m_tws.json"
    ib = connect_tws()
    try:
        rows = fetch(ib, symbol, sec_type, exchange, currency, start, out_path)
        if rows:
            dates = sorted(rows.keys())
            days = len({d[:10] for d in dates})
            size_mb = out_path.stat().st_size / 1024 / 1024
            print(f"\n{'='*50}")
            print(f"完成: {len(rows)} bars | {dates[0]} → {dates[-1]} | {days} 交易日")
            print(f"已存 {out_path} ({size_mb:.1f} MB)")
        else:
            print("未获取到任何数据。")
    finally:
        ib.disconnect()
        print("已断开。", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
