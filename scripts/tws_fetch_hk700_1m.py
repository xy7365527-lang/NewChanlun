"""
Fetch 700.HK (Tencent) full 1-minute historical data from TWS/IB Gateway.

Strategy: pull in 1-month chunks walking backward from today until TWS
returns no data. TWS keeps 700.HK 1min data back to ~2007-05.
Each request pauses 15s to avoid pacing violations.

Supports incremental fetch: if output file exists, loads existing data
and only fetches what's missing before the earliest existing bar.

港股交易时间 9:30-16:00 = 390 分钟/天。~19年 ≈ 1.1M bars。
"""

from ib_insync import IB, Stock
import json
import time
from datetime import datetime, timedelta
from pathlib import Path


OUTPUT_PATH = Path(__file__).parent.parent / "analysis" / "data_cache" / "hk700_1m_tws.json"
PAUSE_SECONDS = 15
MAX_CHUNKS = 250


def connect_tws() -> IB:
    ib = IB()
    for port in [7496, 7497, 4001, 4002]:
        try:
            ib.connect("127.0.0.1", port, clientId=210, timeout=15)
            print(f"Connected on port {port}")
            return ib
        except Exception as e:
            print(f"Port {port} failed: {e}")
    raise ConnectionError("Cannot connect to TWS/IB Gateway on any port")


def _strip_tz(dt_str: str) -> str:
    if "+" in dt_str:
        return dt_str.rsplit("+", 1)[0]
    if dt_str.endswith("Z"):
        return dt_str[:-1]
    return dt_str


def load_existing() -> tuple[list[dict], set[str]]:
    if not OUTPUT_PATH.exists():
        return [], set()
    with open(OUTPUT_PATH) as f:
        bars = json.load(f)
    seen = {b["date"] for b in bars}
    print(f"已有数据: {len(bars)} bars, {bars[0]['date']} ~ {bars[-1]['date']}")
    return bars, seen


def fetch_1m_data(ib: IB, existing_bars: list[dict], seen_times: set[str]) -> list[dict]:
    contract = Stock("700", "SEHK", "HKD")
    qualified = ib.qualifyContracts(contract)
    if not qualified:
        raise ValueError("Failed to qualify 700.SEHK contract")
    contract = qualified[0]
    print(f"Qualified contract: {contract}")

    all_bars = list(existing_bars)

    if existing_bars:
        earliest_existing = _strip_tz(existing_bars[0]["date"])
        end_dt = datetime.strptime(earliest_existing, "%Y-%m-%d %H:%M:%S")
        print(f"增量模式: 从 {end_dt} 向前拉取")
    else:
        end_dt = datetime.now()
        print(f"全量模式: 从 {end_dt} 向前拉取")

    empty_streak = 0

    for chunk_idx in range(MAX_CHUNKS):
        end_str = end_dt.strftime("%Y%m%d-%H:%M:%S")
        print(f"\nChunk {chunk_idx + 1}/{MAX_CHUNKS}, endDateTime={end_str}")

        try:
            bars = ib.reqHistoricalData(
                contract,
                endDateTime=end_str,
                durationStr="1 M",
                barSizeSetting="1 min",
                whatToShow="TRADES",
                useRTH=True,
                formatDate=1,
            )
        except Exception as e:
            err_msg = str(e).lower()
            print(f"Request error: {e}")
            if "pacing" in err_msg:
                print("Pacing violation, waiting 60s then retry...")
                time.sleep(60)
                continue
            if "no data" in err_msg or "invalid" in err_msg:
                print("No data available for this period, done.")
                break
            print("Unknown error, stopping.")
            break

        if not bars:
            empty_streak += 1
            print(f"Empty response (streak={empty_streak})")
            if empty_streak >= 2:
                print("Two consecutive empty responses, reached data limit.")
                break
            end_dt -= timedelta(days=30)
            time.sleep(PAUSE_SECONDS)
            continue

        empty_streak = 0
        new_count = 0
        earliest = None
        for bar in bars:
            dt_str = str(bar.date)
            if dt_str not in seen_times:
                seen_times.add(dt_str)
                all_bars.append({
                    "date": dt_str,
                    "open": float(bar.open),
                    "high": float(bar.high),
                    "low": float(bar.low),
                    "close": float(bar.close),
                    "volume": int(bar.volume),
                })
                new_count += 1
                if earliest is None or bar.date < earliest:
                    earliest = bar.date

        print(f"  Got {len(bars)} bars, {new_count} new. Earliest: {earliest}")
        print(f"  Running total: {len(all_bars)} bars")

        if new_count == 0:
            print("No new bars, reached overlap limit.")
            break

        end_dt = datetime.strptime(_strip_tz(str(earliest)), "%Y-%m-%d %H:%M:%S")

        if chunk_idx < MAX_CHUNKS - 1:
            print(f"  Waiting {PAUSE_SECONDS}s...")
            time.sleep(PAUSE_SECONDS)

        if (chunk_idx + 1) % 20 == 0:
            all_bars.sort(key=lambda x: x["date"])
            OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
            with open(OUTPUT_PATH, "w") as f:
                json.dump(all_bars, f)
            print(f"  [checkpoint] Saved {len(all_bars)} bars to {OUTPUT_PATH}")

    all_bars.sort(key=lambda x: x["date"])
    return all_bars


def main():
    existing_bars, seen_times = load_existing()
    ib = connect_tws()
    try:
        bars = fetch_1m_data(ib, existing_bars, seen_times)
        print(f"\n{'='*50}")
        print(f"Total bars: {len(bars)}")
        if bars:
            print(f"Range: {bars[0]['date']}  →  {bars[-1]['date']}")
            days = len({b["date"][:10] for b in bars})
            print(f"Trading days: {days}")

            OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
            with open(OUTPUT_PATH, "w") as f:
                json.dump(bars, f)
            size_mb = OUTPUT_PATH.stat().st_size / 1024 / 1024
            print(f"Saved to {OUTPUT_PATH} ({size_mb:.1f} MB)")
    finally:
        ib.disconnect()
        print("Disconnected.")


if __name__ == "__main__":
    main()
