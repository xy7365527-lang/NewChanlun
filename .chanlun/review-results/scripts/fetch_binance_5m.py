#!/usr/bin/env python3
"""Fetch Binance 5m TRADIFI perp klines for a small symbol subset, last 60 days, for open/close window analysis."""
import json, time, urllib.request, urllib.error, os

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")
SUBSET = ["AAPL", "NVDA", "TSLA"]

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
    now_ms = int(time.time() * 1000)
    start_ms = now_ms - 60 * 24 * 3600 * 1000
    errors = []
    for sym in SUBSET:
        binsym = sym + "USDT"
        all_klines = []
        cursor = start_ms
        page = 0
        while cursor < now_ms:
            url = (f"https://fapi.binance.com/fapi/v1/klines?symbol={binsym}&interval=5m"
                   f"&startTime={cursor}&limit=1500")
            status, body = http_get(url, timeout=30)
            if status != 200:
                errors.append(f"{binsym} page{page}: HTTP {status} {body[:200]}")
                break
            data = json.loads(body)
            if not data:
                break
            all_klines.extend(data)
            last_open = data[-1][0]
            if last_open <= cursor:
                break
            cursor = last_open + 300_000
            page += 1
            if page > 15:
                errors.append(f"{binsym}: page limit at {page}")
                break
            time.sleep(0.3)
        with open(os.path.join(DATA, f"binance5m_{sym}.json"), "w") as f:
            json.dump(all_klines, f)
        print(f"{sym}: {len(all_klines)} 5m klines")
    if errors:
        with open(os.path.join(DATA, "binance5m_errors.log"), "w") as f:
            f.write("\n".join(errors))
        print(f"{len(errors)} errors")

if __name__ == "__main__":
    main()
