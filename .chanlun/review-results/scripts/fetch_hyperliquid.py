#!/usr/bin/env python3
"""Fetch Hyperliquid xyz-dex equity perp 1h candles, matching Binance onboardDate where available.
Writes data/hl_<SYM>.json (list of candle dicts) and hl_fetch_errors.log
"""
import json, time, urllib.request, os

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")

# HL coin name -> our ticker; only symbols present in xyz dex assetToStreamingOiCap
HL_MAP = {
    "AAPL": "xyz:AAPL", "NVDA": "xyz:NVDA", "TSLA": "xyz:TSLA", "MSFT": "xyz:MSFT",
    "META": "xyz:META", "GOOGL": "xyz:GOOGL", "AMZN": "xyz:AMZN", "COST": "xyz:COST",
    "NFLX": "xyz:NFLX", "LLY": "xyz:LLY",
}
# not on xyz dex: SPY, QQQ, JPM

def http_post(url, body, timeout=30):
    req = urllib.request.Request(url, data=json.dumps(body).encode(),
                                  headers={"Content-Type": "application/json", "User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except Exception as e:
        return None, str(e).encode()

def main():
    with open(os.path.join(DATA, "exchangeInfo.json")) as f:
        info = json.load(f)
    onboard = {s["symbol"]: s.get("onboardDate") for s in info["symbols"]}
    now_ms = int(time.time() * 1000)
    errors = []
    for sym, coin in HL_MAP.items():
        start = onboard.get(sym + "USDT", now_ms - 30*24*3600*1000)
        all_candles = []
        cursor = start
        page = 0
        while cursor < now_ms:
            chunk_end = min(cursor + 5000*3600_000, now_ms)  # ~5000h per request cap safety
            body = {"type": "candleSnapshot", "req": {"coin": coin, "interval": "1h", "startTime": cursor, "endTime": chunk_end}}
            status, resp = http_post("https://api.hyperliquid.xyz/info", body, timeout=30)
            if status != 200:
                errors.append(f"{coin} page{page} start={cursor}: HTTP {status} {resp[:200]}")
                break
            try:
                data = json.loads(resp)
            except Exception as e:
                errors.append(f"{coin} page{page}: parse error {e} body={resp[:200]}")
                break
            if not isinstance(data, list):
                errors.append(f"{coin} page{page}: unexpected response {data}")
                break
            all_candles.extend(data)
            if not data or data[-1]["t"] <= cursor:
                cursor = chunk_end + 1
            else:
                cursor = data[-1]["t"] + 3600_000
            page += 1
            if page > 20:
                errors.append(f"{coin}: page limit hit at page {page}")
                break
            time.sleep(0.3)
        with open(os.path.join(DATA, f"hl_{sym}.json"), "w") as f:
            json.dump(all_candles, f)
        print(f"{sym} ({coin}): {len(all_candles)} candles")

    if errors:
        with open(os.path.join(DATA, "hl_fetch_errors.log"), "w") as f:
            f.write("\n".join(errors))
        print(f"{len(errors)} errors -> hl_fetch_errors.log")

if __name__ == "__main__":
    main()
