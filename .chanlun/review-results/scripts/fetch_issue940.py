#!/usr/bin/env python3
"""#940 结构输入 A/B/C 三套 K 线的采集器（只读，不改仓内任何生产件）。

产出（默认落 `.chanlun/review-results/data/`，**不入仓**，体积见报告）：
  - `y940_<SYM>.json`    Yahoo 1h 原始响应（真实美股，含 meta.tradingPeriods）
  - `hl940_<SYM>.json`   Hyperliquid xyz-dex 1h 原始 candle 数组（24/7 永续）
  - `bn940_<SYM>.json`   Binance TRADIFI 永续 1h（**本环境地域封锁，通常写不出**，见 errors log）
  - `issue940_fetch_errors.log`

⚠️ 采集侧的两条实测约束（2026-08-07，本机）：
  1. Binance `fapi.binance.com` 对本机所有端点（含 `/fapi/v1/ping`）返回
     `{"code":0,"msg":"Service unavailable from a restricted location ..."}`，
     即**地域封锁**，非限流。故永续臂改用 Hyperliquid HIP-3 `xyz` dex
     （#934 已实测 Binance↔Hyperliquid 1h 收盘价中位偏差 -0.041%，n=28619）。
  2. Yahoo `query1` 主机 429；`query2` 需 **Windows Chrome UA**（与
     `fetch_yahoo_curl.sh` 同一 incantation），urllib 默认 UA 与 macOS Chrome UA 均 429。

用法：`python3 fetch_issue940.py`
"""
import json
import os
import subprocess
import time

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.normpath(os.path.join(HERE, "..", "data"))
os.makedirs(DATA, exist_ok=True)

# xyz dex 上有美股永续的标的（名单来源：#934 报告 Q3，实测 10/13 命中）。
SYMS = ["AAPL", "NVDA", "TSLA", "MSFT", "META", "GOOGL", "AMZN", "COST", "NFLX", "LLY"]

# 与 fetch_yahoo_curl.sh 逐字相同的 UA——换任何一个都 429（本机实测）。
YAHOO_UA = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
            "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")

HOUR_MS = 3600_000
ERRORS = []


def curl_get(url, out_path, ua, timeout=40):
    """走 curl（HTTP/2 + 真实 UA）；urllib 在 Yahoo 上被 429，见模块头。"""
    code = subprocess.run(
        ["curl", "-s", "--max-time", str(timeout), "-H", f"User-Agent: {ua}", url,
         "-o", out_path, "-w", "%{http_code}"],
        capture_output=True, text=True).stdout.strip()
    return code


def curl_post_json(url, body, timeout=40):
    p = subprocess.run(
        ["curl", "-s", "--max-time", str(timeout), "-X", "POST", url,
         "-H", "Content-Type: application/json", "-H", "User-Agent: Mozilla/5.0",
         "-d", json.dumps(body)],
        capture_output=True, text=True)
    return p.stdout


def fetch_yahoo():
    for sym in SYMS:
        out = os.path.join(DATA, f"y940_{sym}.json")
        url = (f"https://query2.finance.yahoo.com/v8/finance/chart/{sym}"
               f"?interval=1h&range=2y&events=div,split")
        code = curl_get(url, out, YAHOO_UA)
        n = "?"
        if code == "200":
            try:
                d = json.load(open(out))
                n = len(d["chart"]["result"][0]["timestamp"])
            except Exception as e:  # noqa: BLE001
                ERRORS.append(f"yahoo {sym}: parse {e}")
        else:
            ERRORS.append(f"yahoo {sym}: HTTP {code} {open(out).read()[:200]}")
        print(f"yahoo {sym}: HTTP {code}, n={n}")
        time.sleep(3)


def fetch_hyperliquid():
    now_ms = int(time.time() * 1000)
    # 起点取足够早（2025-08-01），HL 只会返回它实际有的那段。
    start0 = 1754006400000
    for sym in SYMS:
        coin = f"xyz:{sym}"
        all_c, cursor, page = [], start0, 0
        while cursor < now_ms and page <= 40:
            chunk_end = min(cursor + 4000 * HOUR_MS, now_ms)
            body = {"type": "candleSnapshot",
                    "req": {"coin": coin, "interval": "1h",
                            "startTime": cursor, "endTime": chunk_end}}
            raw = curl_post_json("https://api.hyperliquid.xyz/info", body)
            try:
                data = json.loads(raw)
            except Exception as e:  # noqa: BLE001
                ERRORS.append(f"hl {coin} page{page}: parse {e} raw={raw[:200]}")
                break
            if not isinstance(data, list):
                ERRORS.append(f"hl {coin} page{page}: unexpected {str(data)[:200]}")
                break
            all_c.extend(data)
            if not data or data[-1]["t"] <= cursor:
                cursor = chunk_end + 1
            else:
                cursor = data[-1]["t"] + HOUR_MS
            page += 1
            time.sleep(0.25)
        with open(os.path.join(DATA, f"hl940_{sym}.json"), "w") as f:
            json.dump(all_c, f)
        print(f"hl {coin}: {len(all_c)} candles")


def try_binance():
    """照实记录：本机地域封锁。留在脚本里是为了换环境时能直接跑通。"""
    url = "https://fapi.binance.com/fapi/v1/exchangeInfo"
    out = os.path.join(DATA, "bn940_exchangeInfo.json")
    code = curl_get(url, out, "Mozilla/5.0")
    body = open(out).read()[:200] if os.path.exists(out) else ""
    if code != "200" or '"symbols"' not in body:
        ERRORS.append(f"binance exchangeInfo: HTTP {code} body={body}")
        print(f"binance: UNAVAILABLE (HTTP {code}) {body}")
        return False
    print("binance: available")
    return True


if __name__ == "__main__":
    try_binance()
    fetch_yahoo()
    fetch_hyperliquid()
    if ERRORS:
        with open(os.path.join(DATA, "issue940_fetch_errors.log"), "w") as f:
            f.write("\n".join(ERRORS))
        print(f"{len(ERRORS)} errors -> issue940_fetch_errors.log")
