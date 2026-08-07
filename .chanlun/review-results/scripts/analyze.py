#!/usr/bin/env python3
"""
Core analysis: perp (Binance TRADIFI equity perp, 1h) vs real US equity (Yahoo, 1h) price deviation.
Answers Q1 (distribution), Q2 (open vs closed + weekend predictive power), Q3 (Binance vs Hyperliquid),
Q5 (ex-dividend behavior). Q4 (extreme windows) handled by a companion script (analyze_extreme.py)
because it needs finer-grained data.

Outputs: data/stats_summary.json (machine readable) + prints human-readable tables.
"""
import json, os, math, statistics
import numpy as np
from scipy import stats as sps

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")

SUBSET = ["AAPL","NVDA","TSLA","MSFT","META","GOOGL","AMZN","SPY","QQQ","COST","NFLX","JPM","LLY"]
HL_SYMS = ["AAPL","NVDA","TSLA","MSFT","META","GOOGL","AMZN","COST","NFLX","LLY"]  # on xyz dex

def load_binance(sym):
    path = os.path.join(DATA, f"binance_{sym}.json")
    if not os.path.exists(path):
        return {}
    with open(path) as f:
        rows = json.load(f)
    # openTime(ms) -> close price; bar covers [openTime, openTime+1h)
    return {int(r[0]) // 1000: float(r[4]) for r in rows}

def load_hl(sym):
    path = os.path.join(DATA, f"hl_{sym}.json")
    if not os.path.exists(path):
        return {}
    with open(path) as f:
        rows = json.load(f)
    return {int(r["t"]) // 1000: float(r["c"]) for r in rows}

def load_yahoo(sym):
    path = os.path.join(DATA, f"yahoo_{sym}.json")
    if not os.path.exists(path):
        return None
    with open(path) as f:
        d = json.load(f)
    result = d["chart"]["result"]
    if not result:
        return None
    r = result[0]
    ts = r["timestamp"]
    closes = r["indicators"]["quote"][0]["close"]
    meta = r["meta"]
    events = r.get("events", {})
    # regular trading periods: meta['tradingPeriods'] is nested list of [ [ {start,end} ] ] per day (pre/post absent in this shape sometimes)
    # newer yahoo puts tradingPeriods as {'pre':[[...]], 'regular':[[...]], 'post':[[...]]}
    tp = meta.get("tradingPeriods")
    regular_ranges = []
    if isinstance(tp, dict) and "regular" in tp:
        for day in tp["regular"]:
            for seg in day:
                regular_ranges.append((seg["start"], seg["end"]))
    elif isinstance(tp, list):
        # observed shape (2026 Yahoo API): flat list of [ {start,end,...} ] per trading day = regular session only
        for day in tp:
            if isinstance(day, list):
                for seg in day:
                    regular_ranges.append((seg["start"], seg["end"]))
            elif isinstance(day, dict):
                regular_ranges.append((day["start"], day["end"]))
    return {
        "ts": ts, "closes": closes, "meta": meta, "events": events,
        "regular_ranges": regular_ranges,
    }

def in_regular(ts_sec, regular_ranges):
    for s, e in regular_ranges:
        if s <= ts_sec < e:
            return True
    return False

def pct_stats(vals):
    n = len(vals)
    if n == 0:
        return {"n": 0}
    arr = np.array(vals, dtype=float)
    return {
        "n": n,
        "median_pct": float(np.median(arr)) * 100,
        "p90_abs_pct": float(np.percentile(np.abs(arr), 90)) * 100,
        "p99_abs_pct": float(np.percentile(np.abs(arr), 99)) * 100,
        "max_abs_pct": float(np.max(np.abs(arr))) * 100,
        "mean_pct": float(np.mean(arr)) * 100,
        "stdev_pct": float(np.std(arr)) * 100,
    }

def q1_deviation_distribution():
    print("\n===== Q1: perp vs real price deviation distribution (Binance vs Yahoo, matched hourly bars, regular session only) =====")
    all_devs = []
    per_symbol = {}
    for sym in SUBSET:
        yh = load_yahoo(sym)
        bn = load_binance(sym)
        if yh is None or not bn:
            per_symbol[sym] = {"error": "missing data"}
            continue
        devs = []
        for ts, close in zip(yh["ts"], yh["closes"]):
            if close is None:
                continue
            if yh["regular_ranges"] and not in_regular(ts, yh["regular_ranges"]):
                continue
            hour_key = (ts // 3600) * 3600
            if hour_key in bn:
                perp = bn[hour_key]
                dev = (perp - close) / close
                devs.append(dev)
        all_devs.extend(devs)
        per_symbol[sym] = pct_stats(devs)
        s = per_symbol[sym]
        if s.get("n", 0) > 0:
            print(f"  {sym}: n={s['n']}, median={s['median_pct']:.3f}%, p90|dev|={s['p90_abs_pct']:.3f}%, "
                  f"p99|dev|={s['p99_abs_pct']:.3f}%, max|dev|={s['max_abs_pct']:.3f}%")
        else:
            print(f"  {sym}: no matched bars (data: {s})")
    overall = pct_stats(all_devs)
    print(f"  POOLED ALL SYMBOLS: n={overall.get('n')}, median={overall.get('median_pct',float('nan')):.3f}%, "
          f"p90|dev|={overall.get('p90_abs_pct',float('nan')):.3f}%, p99|dev|={overall.get('p99_abs_pct',float('nan')):.3f}%, "
          f"max|dev|={overall.get('max_abs_pct',float('nan')):.3f}%")
    return {"per_symbol": per_symbol, "pooled": overall}

def q2_open_vs_closed_and_predictive():
    print("\n===== Q2: closed-session (perp-only) volatility, and weekend/afterhours predictive power for next session =====")
    results = {}
    for sym in SUBSET:
        yh = load_yahoo(sym)
        bn = load_binance(sym)
        if yh is None or not bn or not yh["regular_ranges"]:
            results[sym] = {"error": "missing data or no regular_ranges"}
            continue
        bn_hours = sorted(bn.keys())
        if not bn_hours:
            continue
        lo, hi = bn_hours[0], bn_hours[-1] + 3600
        # classify each Binance hour bar within Yahoo's covered range as open/closed
        closed_rets = []
        open_rets = []
        prev_close = None
        prev_ts = None
        for ts in bn_hours:
            if ts < lo or ts >= hi:
                continue
            price = bn[ts]
            if prev_close is not None and prev_ts is not None and ts - prev_ts == 3600:
                ret = math.log(price / prev_close)
                if in_regular(ts, yh["regular_ranges"]):
                    open_rets.append(ret)
                else:
                    closed_rets.append(ret)
            prev_close = price
            prev_ts = ts
        def vol(rets):
            if len(rets) < 2:
                return None
            return float(np.std(rets)) * 100  # % per hour stdev of log return
        results[sym] = {
            "n_open_hours": len(open_rets), "hourly_vol_open_pct": vol(open_rets),
            "n_closed_hours": len(closed_rets), "hourly_vol_closed_pct": vol(closed_rets),
        }
        r = results[sym]
        if r.get("hourly_vol_open_pct") is not None:
            ratio = (r["hourly_vol_closed_pct"] / r["hourly_vol_open_pct"]) if r["hourly_vol_open_pct"] else float("nan")
            print(f"  {sym}: open n={r['n_open_hours']} hourly_vol={r['hourly_vol_open_pct']:.4f}%  |  "
                  f"closed n={r['n_closed_hours']} hourly_vol={r['hourly_vol_closed_pct']:.4f}%  ratio(closed/open)={ratio:.2f}")

    # Weekend predictive power: for each symbol, find Friday regular-session close (real, Yahoo) and the
    # perp return from that Friday close timestamp to the following Monday's regular-session open timestamp
    # (weekend/perp-only move), then regress against the *realized* Monday session return (open->close, Yahoo).
    print("\n  --- weekend perp move -> next Monday session return regression ---")
    reg_results = {}
    for sym in SUBSET:
        yh = load_yahoo(sym)
        bn = load_binance(sym)
        if yh is None or not bn or not yh["regular_ranges"]:
            continue
        # build sorted list of (start,end) regular sessions, deduplicated
        sessions = sorted(set(yh["regular_ranges"]))
        # map ts->close from yahoo
        yts = {t: c for t, c in zip(yh["ts"], yh["closes"]) if c is not None}
        xs, ys = [], []
        for i in range(1, len(sessions)):
            prev_start, prev_end = sessions[i-1]
            cur_start, cur_end = sessions[i]
            gap_hours = (cur_start - prev_end) / 3600
            if gap_hours < 40:  # only weekend/long gaps (~2+ days); typical overnight gap ~17.5h
                continue
            # find yahoo close nearest to prev_end (last bar of prev session) and open nearest to cur_start
            prev_close_ts = max((t for t in yts if t <= prev_end), default=None)
            cur_open_ts = min((t for t in yts if t >= cur_start), default=None)
            cur_close_ts = max((t for t in yts if t <= cur_end), default=None)
            if prev_close_ts is None or cur_open_ts is None or cur_close_ts is None:
                continue
            real_prev_close = yts[prev_close_ts]
            real_cur_open = yts[cur_open_ts]
            real_cur_close = yts[cur_close_ts]
            # perp weekend move: perp price nearest prev_end vs perp price nearest cur_start
            bn_hours_sorted = sorted(bn.keys())
            perp_prev = None
            perp_cur = None
            for t in bn_hours_sorted:
                if t <= prev_end:
                    perp_prev = bn[t]
                if t <= cur_start and t >= prev_end:
                    perp_cur = bn[t]
            if perp_prev is None or perp_cur is None:
                continue
            weekend_perp_ret = math.log(perp_cur / perp_prev)
            monday_session_ret = math.log(real_cur_close / real_cur_open)
            gap_ret = math.log(real_cur_open / real_prev_close)  # actual open gap (what we're trying to predict via perp)
            xs.append(weekend_perp_ret)
            ys.append(gap_ret)
        if len(xs) >= 5:
            slope, intercept, r, p, se = sps.linregress(xs, ys)
            reg_results[sym] = {"n": len(xs), "slope": slope, "r": r, "p": p, "intercept": intercept}
            print(f"  {sym}: n={len(xs)} weekend_perp_move -> next_session_open_gap: slope={slope:.3f} r={r:.3f} p={p:.4f}")
        else:
            reg_results[sym] = {"n": len(xs), "note": "insufficient weekend samples"}
            print(f"  {sym}: n={len(xs)} weekend samples -- insufficient for regression")

    # pooled regression across all symbols (z-normalized within symbol not done; pooled raw for a directional read)
    all_x, all_y = [], []
    for sym in SUBSET:
        pass
    return {"vol": results, "weekend_regression": reg_results}

def q3_exchange_comparison():
    print("\n===== Q3: Binance vs Hyperliquid perp price deviation (both vs each other, matched hourly bars) =====")
    per_symbol = {}
    all_devs = []
    for sym in HL_SYMS:
        bn = load_binance(sym)
        hl = load_hl(sym)
        if not bn or not hl:
            per_symbol[sym] = {"error": "missing data"}
            continue
        devs = []
        for ts, bprice in bn.items():
            if ts in hl:
                hprice = hl[ts]
                devs.append((hprice - bprice) / bprice)
        all_devs.extend(devs)
        per_symbol[sym] = pct_stats(devs)
        s = per_symbol[sym]
        if s.get("n", 0) > 0:
            print(f"  {sym}: n={s['n']}, median={s['median_pct']:.4f}%, p90|dev|={s['p90_abs_pct']:.4f}%, "
                  f"p99|dev|={s['p99_abs_pct']:.4f}%, max|dev|={s['max_abs_pct']:.4f}%")
    overall = pct_stats(all_devs)
    print(f"  POOLED (HL vs Binance): n={overall.get('n')}, median={overall.get('median_pct',float('nan')):.4f}%, "
          f"p99|dev|={overall.get('p99_abs_pct',float('nan')):.4f}%, max|dev|={overall.get('max_abs_pct',float('nan')):.4f}%")
    return {"per_symbol": per_symbol, "pooled": overall}

def q5_dividends():
    print("\n===== Q5: ex-dividend behavior — real stock drop vs perp behavior on ex-date =====")
    results = {}
    for sym in SUBSET:
        yh = load_yahoo(sym)
        bn = load_binance(sym)
        if yh is None:
            continue
        divs = yh["events"].get("dividends", {})
        if not divs:
            results[sym] = {"n_divs_in_window": 0}
            continue
        yts = {t: c for t, c in zip(yh["ts"], yh["closes"]) if c is not None}
        entries = []
        for k, v in divs.items():
            ex_ts = int(v["date"])
            amount = float(v["amount"])
            # find last regular-session close on trading day before ex_ts, and first close on/after ex_ts
            before = max((t for t in yts if t < ex_ts), default=None)
            after = min((t for t in yts if t >= ex_ts), default=None)
            if before is None or after is None:
                continue
            real_before = yts[before]
            real_after = yts[after]
            real_drop_pct = (real_after - real_before) / real_before * 100
            entry = {"ex_date_ts": ex_ts, "amount": amount, "real_drop_pct": real_drop_pct}
            if bn:
                bn_hours = sorted(bn.keys())
                pb = max((t for t in bn_hours if t < ex_ts), default=None)
                pa = min((t for t in bn_hours if t >= ex_ts), default=None)
                if pb is not None and pa is not None:
                    perp_before, perp_after = bn[pb], bn[pa]
                    entry["perp_move_pct"] = (perp_after - perp_before) / perp_before * 100
            entries.append(entry)
        results[sym] = {"n_divs_in_window": len(entries), "entries": entries}
        if entries:
            for e in entries:
                pm = e.get("perp_move_pct")
                print(f"  {sym} ex-div ts={e['ex_date_ts']} amount=${e['amount']:.3f} real_move={e['real_drop_pct']:.3f}% "
                      f"perp_move={'n/a' if pm is None else f'{pm:.3f}%'}")
    return results

def main():
    out = {}
    out["q1"] = q1_deviation_distribution()
    out["q2"] = q2_open_vs_closed_and_predictive()
    out["q3"] = q3_exchange_comparison()
    out["q5"] = q5_dividends()
    with open(os.path.join(DATA, "stats_summary.json"), "w") as f:
        json.dump(out, f, indent=2, default=str)
    print("\nWrote data/stats_summary.json")

if __name__ == "__main__":
    main()
