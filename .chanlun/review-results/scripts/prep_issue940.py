#!/usr/bin/env python3
"""#940：把采集到的原始 K 线整成 A / B / C 三套结构输入，供 Rust 探针消费。

三套的定义（与票面一致）：
  A = 真实美股 1h K 线（Yahoo regular session，有跳空、无休市 bar）
  B = 24/7 永续 1h K 线（无跳空、有休市 bar）
  C = B 按美股常规交易时段过滤掉休市 bar（第三条路的代理）

**C 的过滤规则（写清，见报告「方法说明」）**：保留 open 时间落在 `[t, t+1h)`
且该区间与 Yahoo `meta.tradingPeriods` 里**某一个 regular session 区间有非空交集**
的永续 bar。取「相交」而非「包含」，理由：永续 bar 按整点对齐、美股 session 从
09:30 起，两者边界天然错半小时；取「包含」会把每个交易日首尾各切掉一根，人为造出
一个 A 里没有的差异。取「相交」后每交易日保留 7 根（09:00–15:00 ET 各一根），与
Yahoo 每交易日 7 根（09:30,10:30,…,15:30）**根数相同**，可比性最好。
tradingPeriods 自带节假日/夏令时/提前收盘，故不需要另写美股日历。

窗口 = [max(A 首根, B 首根), min(A 末根, B 末根)]，逐标的各算各的。

用法：`python3 prep_issue940.py`  →  `../data/issue940_inputs.json`
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.normpath(os.path.join(HERE, "..", "data"))
HOUR_S = 3600

SYMS = ["AAPL", "NVDA", "TSLA", "MSFT", "META", "GOOGL", "AMZN", "COST", "NFLX", "LLY"]


def load_yahoo(sym):
    d = json.load(open(os.path.join(DATA, f"y940_{sym}.json")))
    r = d["chart"]["result"][0]
    ts = r["timestamp"]
    q = r["indicators"]["quote"][0]
    rows, seen = [], set()
    for i, t in enumerate(ts):
        o, h, l, c = q["open"][i], q["high"][i], q["low"][i], q["close"][i]
        if None in (o, h, l, c) or t in seen:
            continue
        seen.add(t)
        rows.append((int(t), float(o), float(h), float(l), float(c)))
    rows.sort()
    # tradingPeriods: list of list-of-dict（每交易日一个 regular 区间）
    tp = r["meta"].get("tradingPeriods") or []
    sessions = []
    for day in tp:
        items = day if isinstance(day, list) else [day]
        for it in items:
            sessions.append((int(it["start"]), int(it["end"])))
    sessions.sort()
    return rows, sessions


def load_hl(sym):
    arr = json.load(open(os.path.join(DATA, f"hl940_{sym}.json")))
    by_t = {}
    for k in arr:
        by_t[int(k["t"]) // 1000] = (float(k["o"]), float(k["h"]), float(k["l"]), float(k["c"]))
    return [(t, *by_t[t]) for t in sorted(by_t)]


def in_session(t, sessions):
    """bar 区间 [t, t+1h) 与任一 regular session [s,e) 是否有非空交集（二分）。"""
    lo, hi = 0, len(sessions) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        s, e = sessions[mid]
        if e <= t:
            lo = mid + 1
        elif s >= t + HOUR_S:
            hi = mid - 1
        else:
            return True
    return False


def cols(rows):
    return {"ts": [r[0] for r in rows],
            "o": [r[1] for r in rows], "h": [r[2] for r in rows],
            "l": [r[3] for r in rows], "c": [r[4] for r in rows]}


def classify(t, sessions):
    """每根 B bar 的时段名分：session / overnight / weekend / edge。

    非开市 bar 归入它所落的那段休市区间 [上一 session end, 下一 session start)：
    该区间长度 ≥24h ⟹ `weekend`（含长周末与整日休市的节假日），否则 `overnight`
    （= 当日盘后 + 次日盘前那段隔夜）。窗口首尾越界的记 `edge`，不静默并入。
    """
    if in_session(t, sessions):
        return "session"
    prev_end = None
    next_start = None
    lo, hi = 0, len(sessions) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        if sessions[mid][1] <= t:
            prev_end = sessions[mid][1]
            lo = mid + 1
        else:
            next_start = sessions[mid][0]
            hi = mid - 1
    if prev_end is None or next_start is None:
        return "edge"
    return "weekend" if (next_start - prev_end) >= 24 * 3600 else "overnight"


def main():
    out = {"note": "issue #940 structure-input arms; A=Yahoo cash 1h, "
                   "B=Hyperliquid xyz perp 1h 24/7, C=B filtered to US regular session",
           "symbols": []}
    for sym in SYMS:
        a_all, sessions = load_yahoo(sym)
        b_all = load_hl(sym)
        if not a_all or not b_all:
            print(f"{sym}: SKIP (a={len(a_all)} b={len(b_all)})")
            continue
        lo = max(a_all[0][0], b_all[0][0])
        hi = min(a_all[-1][0], b_all[-1][0])
        A = [r for r in a_all if lo <= r[0] <= hi]
        B = [r for r in b_all if lo <= r[0] <= hi]
        C = [r for r in B if in_session(r[0], sessions)]
        bcols = cols(B)
        bcols["cls"] = [classify(r[0], sessions) for r in B]
        acols = cols(A)
        acols["cls"] = ["session"] * len(A)
        ccols = cols(C)
        ccols["cls"] = ["session"] * len(C)
        out["symbols"].append({
            "symbol": sym,
            "window_start_utc": lo, "window_end_utc": hi,
            "n_sessions_in_window": sum(1 for s, e in sessions if lo <= s <= hi),
            "A": acols, "B": bcols, "C": ccols,
        })
        print(f"{sym}: window {lo}..{hi}  A={len(A)} B={len(B)} C={len(C)}")
    p = os.path.join(DATA, "issue940_inputs.json")
    with open(p, "w") as f:
        json.dump(out, f)
    print("wrote", p, os.path.getsize(p) // 1024, "KiB")


if __name__ == "__main__":
    main()
