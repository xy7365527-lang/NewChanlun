#!/usr/bin/env python3
"""
Q4: extreme-window deviation (open 30min / close 30min) using 5m bars, last 60 days,
for AAPL/NVDA/TSLA (Binance vs Yahoo). Also flags the single trading days with the
largest realized move as an "extreme-day" proxy -- NOT a confirmed earnings/macro
calendar match (no free calendar source was used; this is an honest limitation,
stated in the report).
"""
import json, os, datetime
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")
SUBSET = ["AAPL", "NVDA", "TSLA"]

def load_binance5m(sym):
    with open(os.path.join(DATA, f"binance5m_{sym}.json")) as f:
        rows = json.load(f)
    return {int(r[0]) // 1000: float(r[4]) for r in rows}

def load_yahoo5m(sym):
    with open(os.path.join(DATA, f"yahoo_{sym}_5m.json")) as f:
        d = json.load(f)
    r = d["chart"]["result"][0]
    ts = r["timestamp"]
    closes = r["indicators"]["quote"][0]["close"]
    tp = r["meta"].get("tradingPeriods", [])
    regular_ranges = []
    for day in tp:
        if isinstance(day, list):
            for seg in day:
                regular_ranges.append((seg["start"], seg["end"]))
        elif isinstance(day, dict):
            regular_ranges.append((day["start"], day["end"]))
    return ts, closes, sorted(regular_ranges)

def pct_stats(vals):
    n = len(vals)
    if n == 0:
        return {"n": 0}
    arr = np.abs(np.array(vals, dtype=float)) * 100
    return {"n": n, "median_abs_pct": float(np.median(arr)), "p90_abs_pct": float(np.percentile(arr, 90)),
            "max_abs_pct": float(np.max(arr)), "mean_abs_pct": float(np.mean(arr))}

def main():
    out = {}
    for sym in SUBSET:
        bn = load_binance5m(sym)
        ts, closes, sessions = load_yahoo5m(sym)
        yts = {t: c for t, c in zip(ts, closes) if c is not None}
        open_devs, close_devs, mid_devs = [], [], []
        day_moves = {}  # date -> (open real, close real)
        for start, end in sessions:
            # open window: [start, start+1800)
            for t in range(start, start + 1800, 300):
                if t in yts and t in bn:
                    real = yts[t]
                    perp = bn[t]
                    open_devs.append((perp - real) / real)
            for t in range(end - 1800, end, 300):
                if t in yts and t in bn:
                    real = yts[t]
                    perp = bn[t]
                    close_devs.append((perp - real) / real)
            for t in range(start + 1800, end - 1800, 300):
                if t in yts and t in bn:
                    real = yts[t]
                    perp = bn[t]
                    mid_devs.append((perp - real) / real)
            # day move for extreme-day flagging (real regular session return)
            first_t = min((t for t in yts if start <= t < start + 300), default=None)
            last_t = max((t for t in yts if end - 300 <= t < end), default=None)
            if first_t is not None and last_t is not None:
                day_moves[datetime.datetime.utcfromtimestamp(start).date().isoformat()] = \
                    (yts[first_t], yts[last_t], (yts[last_t] - yts[first_t]) / yts[first_t] * 100)

        open_stats = pct_stats(open_devs)
        close_stats = pct_stats(close_devs)
        mid_stats = pct_stats(mid_devs)
        print(f"\n{sym}: open-30min |dev| n={open_stats['n']} median={open_stats.get('median_abs_pct',float('nan')):.3f}% "
              f"p90={open_stats.get('p90_abs_pct',float('nan')):.3f}% max={open_stats.get('max_abs_pct',float('nan')):.3f}%")
        print(f"{sym}: close-30min |dev| n={close_stats['n']} median={close_stats.get('median_abs_pct',float('nan')):.3f}% "
              f"p90={close_stats.get('p90_abs_pct',float('nan')):.3f}% max={close_stats.get('max_abs_pct',float('nan')):.3f}%")
        print(f"{sym}: mid-session |dev| n={mid_stats['n']} median={mid_stats.get('median_abs_pct',float('nan')):.3f}% "
              f"p90={mid_stats.get('p90_abs_pct',float('nan')):.3f}% max={mid_stats.get('max_abs_pct',float('nan')):.3f}%")

        top_days = sorted(day_moves.items(), key=lambda kv: abs(kv[1][2]), reverse=True)[:5]
        print(f"{sym}: top-5 largest single-day regular-session moves (proxy for 'extreme days', NOT earnings-confirmed):")
        for date, (o, c, pct) in top_days:
            print(f"    {date}: open={o:.2f} close={c:.2f} move={pct:.2f}%")

        out[sym] = {"open30": open_stats, "close30": close_stats, "mid": mid_stats,
                     "top_days": [(d, v[2]) for d, v in top_days]}
    with open(os.path.join(DATA, "stats_extreme.json"), "w") as f:
        json.dump(out, f, indent=2, default=str)
    print("\nWrote data/stats_extreme.json")

if __name__ == "__main__":
    main()
