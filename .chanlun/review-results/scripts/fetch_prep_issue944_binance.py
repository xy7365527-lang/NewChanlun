#!/usr/bin/env python3
"""#944 附加臂：**Binance TRADIFI 永续**版的 B/C，用来判「43% 是不是 Hyperliquid 独有的伪影」。

## 为什么会有这个脚本（订正 #940 与 ADR 0020 的一条在案事实）

[#940](https://github.com/xy7365527-lang/NewChanlun/issues/940) 与 ADR 0020 有效域第 2 条都写着
「Binance `fapi` 全端点 HTTP 451 地域封锁 ⟹ 两所横比无法独立复核」。**2026-08-08 本机复测：
`fapi.binance.com/fapi/v1/ping` → HTTP 200、`/fapi/v1/exchangeInfo` → 854 个 symbol（含
AAPLUSDT/NVDA/TSLA/MSFT/META/GOOGL/AMZN/COST/NFLX/LLY/SPY/QQQ）、`/fapi/v1/klines` 正常返回。**
封锁已解除（或本机出口变了）。故本脚本把 #940 拿不到的那一臂补上。

产出 `../data/issue944_binance_inputs.json`，schema 与 `issue940_inputs.json` **完全一致**
（A/B/C 三臂 + cls），可直接喂 `p944_bsp_layer_mismatch`：
  A = 同一份 Yahoo 真实美股 1h（复用 `y940_<SYM>.json`，与主跑逐字节相同）
  B = Binance 永续 1h 24/7
  C = B 按同一份 `meta.tradingPeriods` 过滤（过滤规则逐字复用 `prep_issue940.py`）

窗口取 [max(A 首根, B 首根), min(A 末根, B 末根)]，与主跑同法（但**因 Binance 与 Hyperliquid
可得历史不同，窗口不必与主跑逐日相同**——跨表比数字前先看窗口行）。

用法：`python3 fetch_prep_issue944_binance.py`
"""
import json
import os
import subprocess
import time

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.normpath(os.path.join(HERE, "..", "data"))
HOUR_S = 3600
HOUR_MS = 3600_000

SYMS = ["AAPL", "NVDA", "TSLA", "MSFT", "META", "GOOGL", "AMZN", "COST", "NFLX", "LLY"]
ERRORS = []

# 复用 prep_issue940 的 Yahoo 载入 + 时段判定，保证 A 臂与 C 的过滤规则逐字相同。
import prep_issue940 as P  # noqa: E402


def fetch_binance(sym):
    """1h klines 分页拉取（limit=1500/页）。返回 [(ts_s, o, h, l, c)]。"""
    out, cursor = {}, int(time.time() * 1000) - 260 * 86400_000
    for _ in range(20):
        url = (f"https://fapi.binance.com/fapi/v1/klines?symbol={sym}USDT"
               f"&interval=1h&startTime={cursor}&limit=1500")
        p = subprocess.run(["curl", "-s", "--max-time", "40", url],
                           capture_output=True, text=True)
        try:
            arr = json.loads(p.stdout)
        except Exception as e:  # noqa: BLE001
            ERRORS.append(f"binance {sym}: parse {e} raw={p.stdout[:200]}")
            break
        if not isinstance(arr, list) or not arr:
            if isinstance(arr, dict):
                ERRORS.append(f"binance {sym}: {str(arr)[:200]}")
            break
        for k in arr:
            out[int(k[0]) // 1000] = (float(k[1]), float(k[2]), float(k[3]), float(k[4]))
        nxt = int(arr[-1][0]) + HOUR_MS
        if nxt <= cursor or len(arr) < 1500:
            break
        cursor = nxt
        time.sleep(0.2)
    return [(t, *out[t]) for t in sorted(out)]


def main():
    res = {"note": "issue #944 supplementary arm: A=Yahoo cash 1h (same file as main run), "
                   "B=Binance TRADIFI perp 1h 24/7, C=B filtered to US regular session",
           "symbols": []}
    for sym in SYMS:
        a_all, sessions = P.load_yahoo(sym)
        b_all = fetch_binance(sym)
        raw_p = os.path.join(DATA, f"bn944_{sym}.json")
        with open(raw_p, "w") as f:
            json.dump(b_all, f)
        if not a_all or not b_all:
            print(f"{sym}: SKIP (a={len(a_all)} b={len(b_all)})")
            continue
        lo = max(a_all[0][0], b_all[0][0])
        hi = min(a_all[-1][0], b_all[-1][0])
        A = [r for r in a_all if lo <= r[0] <= hi]
        B = [r for r in b_all if lo <= r[0] <= hi]
        C = [r for r in B if P.in_session(r[0], sessions)]
        bcols = P.cols(B)
        bcols["cls"] = [P.classify(r[0], sessions) for r in B]
        acols = P.cols(A)
        acols["cls"] = ["session"] * len(A)
        ccols = P.cols(C)
        ccols["cls"] = ["session"] * len(C)
        res["symbols"].append({
            "symbol": sym, "window_start_utc": lo, "window_end_utc": hi,
            "n_sessions_in_window": sum(1 for s, e in sessions if lo <= s <= hi),
            "A": acols, "B": bcols, "C": ccols})
        print(f"{sym}: window {lo}..{hi}  A={len(A)} B={len(B)} C={len(C)}")
    p = os.path.join(DATA, "issue944_binance_inputs.json")
    with open(p, "w") as f:
        json.dump(res, f)
    print("wrote", p, os.path.getsize(p) // 1024, "KiB")
    if ERRORS:
        print("ERRORS:", *ERRORS, sep="\n  ")


if __name__ == "__main__":
    main()
