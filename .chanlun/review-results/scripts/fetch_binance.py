#!/usr/bin/env python3
"""Fetch Binance TRADIFI perpetual 1h klines for a symbol subset, from onboardDate to now.
Deps: requests (or urllib fallback). Writes one JSON-lines-free JSON array per symbol to data/binance_<SYM>.json
Usage: python3 fetch_binance.py
"""
import json, time, urllib.request, urllib.error, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")
os.makedirs(DATA, exist_ok=True)

SUBSET = ["AAPL","NVDA","TSLA","MSFT","META","GOOGL","AMZN","SPY","QQQ","COST","NFLX","JPM","LLY"]

def http_get(url, timeout=30):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:
        return None, str(e).encode()

def main():
    with open(os.path.join(DATA, "exchangeInfo.json")) as f:
        info = json.load(f)
    onboard = {s["symbol"]: s.get("onboardDate") for s in info["symbols"]}

    errors = []
    now_ms = int(time.time() * 1000)
    for sym in SUBSET:
        binsym = sym + "USDT"
        start = onboard.get(binsym)
        if start is None:
            errors.append(f"{binsym}: not found in exchangeInfo")
            continue
        all_klines = []
        cursor = start
        page = 0
        while cursor < now_ms:
            url = (f"https://fapi.binance.com/fapi/v1/klines?symbol={binsym}&interval=1h"
                   f"&startTime={cursor}&limit=1000")
            status, body = http_get(url, timeout=30)
            if status != 200:
                errors.append(f"{binsym} page {page} startTime={cursor}: HTTP {status} {body[:200]}")
                break
            try:
                data = json.loads(body)
            except Exception as e:
                errors.append(f"{binsym} page {page}: json parse error {e}")
                break
            if not data:
                break
            all_klines.extend(data)
            last_open = data[-1][0]
            if last_open <= cursor:
                break
            cursor = last_open + 3600_000
            page += 1
            if page > 20:
                errors.append(f"{binsym}: page limit hit, stopping at page {page}")
                break
            time.sleep(0.3)
        with open(os.path.join(DATA, f"binance_{sym}.json"), "w") as f:
            json.dump(all_klines, f)
        print(f"{sym}: {len(all_klines)} klines, onboard={start}")

    if errors:
        with open(os.path.join(DATA, "binance_fetch_errors.log"), "w") as f:
            f.write("\n".join(errors))
        print(f"{len(errors)} errors logged to binance_fetch_errors.log")

if __name__ == "__main__":
    main()
