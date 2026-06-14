#!/usr/bin/env python
"""type1 覆盖率漏检诊断（OKLO L1 主走势层）—— 自包含复现 type1_coverage_diagnosis.md。

两部分：
  PART 1 反转点机制分桶（C0/C1/C2/C3），fallback vs MACD 对照。
  PART 2 走势级 force 根因（复刻 detect_trend_divergence + compute_force，零失配对账）。

用法: .venv/bin/python analysis/_type1_coverage_diag.py [N|-] [fallback|macd|both] [out.json]
"""
import sys, json, time, statistics as st
sys.path.insert(0, "src"); sys.path.insert(0, "analysis")
import newchan_rust as R

N = int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1] != "-" else None
MODE = sys.argv[2] if len(sys.argv) > 2 else "both"
OUT = sys.argv[3] if len(sys.argv) > 3 else None

d = json.load(open("analysis/data_cache/oklo_1m_databento.json"))
bars = d["bars"]
BB = bars if N is None else bars[:N]


def run(enable_macd):
    t0 = time.time()
    o = R.RecursiveOrchestrator(enable_macd_divergence=enable_macd)
    for b in BB:
        o.process_bar(b["open"], b["high"], b["low"], b["close"])
    st_ = {"moves": o.current_moves(), "bsps": o.current_buysellpoints(),
           "zss": o.current_zhongshus(), "segs": o.current_segments()}
    # current_trend_divergences 仅 fallback 有效（macd 路径 panic）
    st_["divs"] = None if enable_macd else o.current_trend_divergences()
    st_["secs"] = time.time() - t0
    return st_


def near(L, x, dd=3):
    return any(abs(s - x) <= dd for s in L)


# ── PART 1: 反转点机制分桶 ──
def classify(s):
    moves, bsps = s["moves"], s["bsps"]
    sm = [m for m in moves if m[0][7]]
    conf = {"type1": [], "type2": [], "type3": []}
    all_t1 = []
    for b in bsps:
        if b[0][0] == "type1":
            all_t1.append(b[0][3])
        if b[0][5]:
            conf[b[0][0]].append(b[0][3])
    for k in conf:
        conf[k].sort()
    all_t1.sort()

    buckets = {"C0_covered": 0, "C1_threshold": 0, "C2_no_div": 0, "C3_consol": 0}
    n_rev = anybsp = 0
    marked = {"type3": 0, "type2": 0, "none": 0}
    for i in range(len(sm) - 1):
        a, b = sm[i], sm[i + 1]
        if a[0][1] == b[0][1]:
            continue
        n_rev += 1
        rev, kind, zsc = a[0][3], a[0][0], a[0][6]
        if near(conf["type1"], rev) or near(conf["type2"], rev) or near(conf["type3"], rev):
            anybsp += 1
        if near(conf["type1"], rev):
            buckets["C0_covered"] += 1
            continue
        marked["type3" if near(conf["type3"], rev) else "type2" if near(conf["type2"], rev) else "none"] += 1
        if near(all_t1, rev):
            buckets["C1_threshold"] += 1
        elif kind == "consolidation" or zsc < 2:
            buckets["C3_consol"] += 1
        else:
            buckets["C2_no_div"] += 1
    return {"n_settled": len(sm), "n_rev": n_rev, "anybsp": anybsp,
            "t1_pct": round(100 * buckets["C0_covered"] / max(n_rev, 1), 1),
            "buckets": buckets, "marked": marked,
            "n_conf": {k: len(v) for k, v in conf.items()},
            "n_all_type1": len(all_t1), "secs": round(s["secs"], 1)}


# ── PART 2: 走势级 force 根因（复刻 detect_trend_divergence，仅 fallback 力度）──
def force_rootcause(s):
    moves, zss, segs = s["moves"], s["zss"], s["segs"]
    n = len(segs)
    sm = [m for m in moves if m[0][7]]

    def force(s0, s1):
        if s0 > s1 or s0 < 0 or s1 >= n:
            return 0.0
        i0, i1 = segs[s0][0][2], segs[s1][0][3]
        hi = max(segs[k][0][5] for k in range(s0, s1 + 1))
        lo = min(segs[k][0][6] for k in range(s0, s1 + 1))
        return (hi - lo) * max(i1 - i0, 1)

    def extreme(lo, hi, direction):
        best = lo
        if direction == "up":
            bv = segs[lo][0][5]
            for k in range(lo + 1, hi + 1):
                if segs[k][0][5] > bv:
                    bv, best = segs[k][0][5], k
        else:
            bv = segs[lo][0][6]
            for k in range(lo + 1, hi + 1):
                if segs[k][0][6] < bv:
                    bv, best = segs[k][0][6], k
        return best

    div, nodiv, other = 0, [], {"not_trend": 0, "settled_zs<2": 0, "C_empty": 0, "force_a<=0": 0}
    for m in sm:
        kind, direction, zs0, zs1, zsc = m[0][0], m[0][1], m[0][4], m[0][5], m[0][6]
        if kind != "trend" or zsc < 2:
            other["not_trend"] += 1; continue
        idx = [i for i in range(zs0, min(zs1 + 1, len(zss))) if zss[i][5]]
        if len(idx) < 2:
            other["settled_zs<2"] += 1; continue
        zp, zl = zss[idx[-2]], zss[idx[-1]]
        a_start, a_end = zp[3] + 1, zl[2] - 1
        if a_start > a_end:
            a_start = a_end = zp[3]
        c_start = zl[3] + 1
        nxt = next((z[3] for z in zss[idx[-1] + 1:] if z[5]), None)
        se = min(nxt if nxt is not None else n - 1, n - 1)
        if c_start > se:
            other["C_empty"] += 1; continue
        c_end = extreme(c_start, se, direction)
        fa, fc = force(a_start, a_end), force(c_start, c_end)
        if fa <= 0:
            other["force_a<=0"] += 1; continue
        if fc < fa:
            div += 1
        else:
            nodiv.append(fc / fa)
    out = {"n_settled_trend": len(sm), "div_fc<fa": div, "nodiv_fc>=fa": len(nodiv),
           "actual_trend_div": len(s["divs"]) if s["divs"] else None, "other_none": other}
    if nodiv:
        out["nodiv_ratio"] = {"n": len(nodiv), "min": round(min(nodiv), 2),
                              "median": round(st.median(nodiv), 2), "max": round(max(nodiv), 2),
                              ">3x": sum(1 for r in nodiv if r >= 3)}
    return out


results = {}
for m in (["fallback", "macd"] if MODE == "both" else [MODE]):
    s = run(m == "macd")
    res = classify(s)
    results[m] = res
    print(f"\n===== MODE={m}  (n_bars={len(BB)}, {res['secs']}s) =====")
    print(f"settled={res['n_settled']} reversals={res['n_rev']}  type1覆盖={res['t1_pct']}%")
    print(f"任意BSP(δ3)={res['anybsp']}/{res['n_rev']}  机制桶={res['buckets']}")
    print(f"缺口实际标记={res['marked']}  confirmed={res['n_conf']} all_type1={res['n_all_type1']}")
    if m == "fallback":
        rc = force_rootcause(s)
        results["fallback_rootcause"] = rc
        print(f"[根因] {rc}")

if OUT:
    json.dump(results, open(OUT, "w"), indent=2)
    print(f"\n写入 {OUT}")
