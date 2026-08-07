#!/usr/bin/env python3
"""Fetch Yahoo Finance hourly chart data (+ dividend events) for the ticker subset.
Writes data/yahoo_<SYM>.json raw response, and logs errors.
Requires User-Agent header (else 429).
"""
import json, time, urllib.request, os

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")

SUBSET = ["AAPL","NVDA","TSLA","MSFT","META","GOOGL","AMZN","SPY","QQQ","COST","NFLX","JPM","LLY"]

def http_get(url, timeout=30):
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
    })
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except Exception as e:
        code = getattr(e, "code", None)
        body = e.read() if hasattr(e, "read") else str(e).encode()
        return code, body

def main():
    errors = []
    for sym in SUBSET:
        url = f"https://query2.finance.yahoo.com/v8/finance/chart/{sym}?interval=1h&range=2y&events=div,split"
        status, body = http_get(url)
        if status != 200:
            errors.append(f"{sym}: HTTP {status} {body[:300]}")
            time.sleep(1)
            continue
        with open(os.path.join(DATA, f"yahoo_{sym}.json"), "wb") as f:
            f.write(body)
        try:
            d = json.loads(body)
            n = len(d["chart"]["result"][0]["timestamp"])
        except Exception as e:
            n = f"parse-error: {e}"
        print(f"{sym}: {n} points, HTTP {status}")
        time.sleep(1)
    if errors:
        with open(os.path.join(DATA, "yahoo_fetch_errors.log"), "w") as f:
            f.write("\n".join(errors))
        print(f"{len(errors)} errors -> yahoo_fetch_errors.log")

if __name__ == "__main__":
    main()
