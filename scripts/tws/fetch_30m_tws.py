#!/usr/bin/env python3
"""通过 TWS / IB Gateway 批量拉取 30min 历史数据（Γ→Δ 30min a0 验证用）。

派生自 fetch_1m_tws.py，barSizeSetting="30 mins"。一次运行串行拉取多个标的
（避免并发 pacing violation）。输出列式格式与 es_1h_databento.json / *_1m_tws.json
**完全一致**：{symbol, dates, opens, highs, lows, closes, volumes}，dates 为 UTC
tz-aware 字符串（"YYYY-MM-DD HH:MM:SS+00:00"）。

================================ 两种拉取模式（数据可用性边界）================================
实测 IBKR 30min 历史数据深度（formalization-validity-domain，硬边界，非 workaround）：

  - **ContFuture（连续期货）**：拒绝 endDateTime（Error 10339"不允许为连续期货设置结束
    日期"）→ 无法翻页。单次 endDateTime="" 请求返回的最大深度 ≈ **3.2 年**（ES 实测
    "5 Y"/"8 Y" 均只回 10601 bars，最早 2023-03-22）。这是 IBKR 连续期货 intraday 上限。
    → FUT 模式：**单次大请求，不翻页**，取 IBKR 给的最大深度。

  - **STK（ETF）**：接受 endDateTime → 可翻页。单次 "5 Y" 返回完整 5 年（UUP 实测
    16268 bars，最早 2021-06）。→ STK 模式：**endDateTime 按 2 年/块向前翻页**到 start。

标的清单（Γ→Δ 30min 顶点 + 折叠通道，528号折叠模型）：
  ES  FUT CME   USD  → P 顶点（S&P 期货，SPY 等价）         [~3.2yr]
  GC  FUT COMEX USD  → Au 折叠通道（金）                      [~3.2yr]
  CL  FUT NYMEX USD  → Oil 折叠通道（油）                     [~3.2yr]
  UUP STK SMART USD  → M 货币锚                               [可达 10yr]
  VNQ STK SMART USD  → R 不动产顶点                           [可达 10yr]
  DBC STK SMART USD  → C 商品顶点                             [可达 10yr]
  SPY STK SMART USD  → P 顶点（ETF，10yr 宏观跑用，替 ES）    [可达 10yr]
  GLD STK SMART USD  → Au 折叠通道（ETF，替 GC）              [可达 10yr]
  USO STK SMART USD  → Oil 折叠通道（ETF，替 CL）             [可达 10yr]

用法：
    PYTHONPATH=src python scripts/tws/fetch_30m_tws.py            # 拉全部
    PYTHONPATH=src python scripts/tws/fetch_30m_tws.py ES SPY     # 只拉指定标的
输出：analysis/data_cache/<symbol_lower>_30m_tws.json
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from ib_insync import IB, ContFuture, Forex, Stock  # noqa: F401

_ROOT = Path(__file__).resolve().parents[2]
CACHE_DIR = _ROOT / "analysis" / "data_cache"

PAUSE_SECONDS = 12
BAR_SIZE = "30 mins"
DEFAULT_START = "2014-01-01"
PORTS = [7496, 7497, 4001, 4002]
CLIENT_ID = 251

# STK 翻页：每块 2 年（~7000 bars，单次 ~40s），最多回溯 6 块 = 12 年。
STK_CHUNK_DURATION = "2 Y"
STK_MAX_CHUNKS = 7
STK_TIMEOUT = 150
# FUT 单次：请求 6 年（IBKR 实际只回 ~3.2yr 上限），长超时容纳大请求。
FUT_DURATION = "6 Y"
FUT_TIMEOUT = 280

# (symbol, secType, exchange, currency)
TARGETS: dict[str, tuple[str, str, str, str]] = {
    "ES": ("FUT", "CME", "USD"),
    "GC": ("FUT", "COMEX", "USD"),
    "CL": ("FUT", "NYMEX", "USD"),
    "UUP": ("STK", "SMART", "USD"),
    "VNQ": ("STK", "SMART", "USD"),
    "DBC": ("STK", "SMART", "USD"),
    "SPY": ("STK", "SMART", "USD"),
    "GLD": ("STK", "SMART", "USD"),
    "USO": ("STK", "SMART", "USD"),
}


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
    dt = bar_date
    if not isinstance(dt, datetime):
        dt = datetime(dt.year, dt.month, dt.day, tzinfo=timezone.utc)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    else:
        dt = dt.astimezone(timezone.utc)
    return dt.strftime("%Y-%m-%d %H:%M:%S+00:00")


def load_existing(out_path: Path) -> dict[str, list]:
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


def _merge_bars(bars, rows: dict[str, list]) -> tuple[int, datetime | None]:
    """把一批 bars 并入 rows（去重），返回 (新增数, 最早 bar 时间)。"""
    new_count = 0
    earliest: datetime | None = None
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
            if earliest is None or bdt < earliest:
                earliest = bdt
    return new_count, earliest


def fetch_fut(ib: IB, contract, what: str, rows: dict[str, list]) -> dict[str, list]:
    """连续期货：单次 endDateTime='' 大请求（无法翻页），取 IBKR 最大深度。"""
    print(f"[FUT] 单次请求 {FUT_DURATION}（IBKR 连续期货拒绝 endDateTime，不翻页）", flush=True)
    try:
        bars = ib.reqHistoricalData(
            contract, endDateTime="", durationStr=FUT_DURATION,
            barSizeSetting=BAR_SIZE, whatToShow=what, useRTH=True,
            formatDate=2, timeout=FUT_TIMEOUT,
        )
    except Exception as e:  # noqa: BLE001
        print(f"  请求错误: {e!r}", flush=True)
        return rows
    if not bars:
        print("  空响应（连续期货无 intraday 历史或超时）", flush=True)
        return rows
    n, earliest = _merge_bars(bars, rows)
    print(f"  收到 {len(bars)} bars, {n} 新. 最早: {earliest}", flush=True)
    return rows


def fetch_stk(ib: IB, contract, what: str, rows: dict[str, list], start: str) -> dict[str, list]:
    """ETF：endDateTime 按 2 年/块向前翻页到 start。"""
    start_dt = datetime.strptime(start, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    if rows:
        earliest = min(rows.keys())
        end_dt = datetime.strptime(earliest[:19], "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
        print(f"[STK] 增量模式: 从 {end_dt} 向前", flush=True)
    else:
        end_dt = datetime.now(timezone.utc)
        print(f"[STK] 全量模式: 从 {end_dt} 向前到 {start_dt}", flush=True)

    empty_streak = 0
    for chunk_idx in range(STK_MAX_CHUNKS):
        if end_dt <= start_dt:
            print(f"  已到达 start 截止 {start}，停止。", flush=True)
            break
        end_str = end_dt.strftime("%Y%m%d-%H:%M:%S")
        print(f"  Chunk {chunk_idx + 1}/{STK_MAX_CHUNKS}, endDateTime={end_str} (UTC)", flush=True)
        try:
            bars = ib.reqHistoricalData(
                contract, endDateTime=end_str, durationStr=STK_CHUNK_DURATION,
                barSizeSetting=BAR_SIZE, whatToShow=what, useRTH=True,
                formatDate=2, timeout=STK_TIMEOUT,
            )
        except Exception as e:  # noqa: BLE001
            msg = str(e).lower()
            print(f"  请求错误: {e}", flush=True)
            if "pacing" in msg:
                time.sleep(60)
                continue
            break
        if not bars:
            empty_streak += 1
            print(f"  空响应 (streak={empty_streak})", flush=True)
            if empty_streak >= 2:
                print("  连续两次空响应，到达数据上限。", flush=True)
                break
            end_dt -= timedelta(days=730)
            time.sleep(PAUSE_SECONDS)
            continue
        empty_streak = 0
        n, earliest = _merge_bars(bars, rows)
        print(f"    收到 {len(bars)} bars, {n} 新. 最早: {earliest}. 累计 {len(rows)}", flush=True)
        if n == 0 or earliest is None:
            print("  无新 bar，到达重叠上限。", flush=True)
            break
        end_dt = earliest
        if chunk_idx < STK_MAX_CHUNKS - 1:
            time.sleep(PAUSE_SECONDS)
    return rows


def fetch_one(
    ib: IB, symbol: str, sec_type: str, exchange: str, currency: str,
    start: str, out_path: Path,
) -> dict[str, list]:
    if sec_type == "STK":
        contract, what = Stock(symbol, exchange, currency), "TRADES"
    elif sec_type == "FUT":
        contract, what = ContFuture(symbol, exchange, currency), "TRADES"
    elif sec_type == "CASH":
        contract, what = Forex(symbol), "MIDPOINT"
    else:
        raise ValueError(f"不支持的 secType={sec_type}")

    qualified = ib.qualifyContracts(contract)
    if not qualified:
        raise ValueError(f"无法 qualify 合约 {symbol} {sec_type} {exchange} {currency}")
    contract = qualified[0]
    print(f"合约: {contract.localSymbol} ({contract.conId}) | what={what} | bar={BAR_SIZE}", flush=True)

    rows = load_existing(out_path)
    if sec_type == "FUT":
        rows = fetch_fut(ib, contract, what, rows)
    else:
        rows = fetch_stk(ib, contract, what, rows, start)
    _save_columnar(symbol, rows, out_path)
    return rows


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    requested = [s.upper() for s in sys.argv[1:]] if len(sys.argv) > 1 else list(TARGETS)
    unknown = [s for s in requested if s not in TARGETS]
    if unknown:
        print(f"未知标的 {unknown}，可选：{list(TARGETS)}")
        return 1

    ib = connect_tws()
    summary: list[str] = []
    try:
        for symbol in requested:
            sec_type, exchange, currency = TARGETS[symbol]
            out_path = CACHE_DIR / f"{symbol.lower()}_30m_tws.json"
            print(f"\n{'#'*60}\n# 拉取 {symbol} ({sec_type} {exchange})\n{'#'*60}", flush=True)
            try:
                rows = fetch_one(ib, symbol, sec_type, exchange, currency, DEFAULT_START, out_path)
            except Exception as e:  # noqa: BLE001
                print(f"[{symbol}] 拉取失败: {e!r}", flush=True)
                summary.append(f"{symbol}: 失败 {repr(e)[:80]}")
                continue
            if rows:
                dates = sorted(rows.keys())
                days = len({d[:10] for d in dates})
                size_mb = out_path.stat().st_size / 1024 / 1024
                line = (f"{symbol}: {len(rows)} bars | {dates[0]} → {dates[-1]} | "
                        f"{days} 交易日 | {size_mb:.1f}MB")
                print(f"\n=== {line} ===", flush=True)
                summary.append(line)
            else:
                summary.append(f"{symbol}: 无数据")
            time.sleep(PAUSE_SECONDS)
    finally:
        ib.disconnect()
        print("已断开。", flush=True)

    print("\n" + "=" * 60 + "\n拉取汇总：")
    for line in summary:
        print("  " + line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
