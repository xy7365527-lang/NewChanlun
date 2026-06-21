#!/usr/bin/env python3
"""T 操作层引擎（纯 BSP 驱动 M=N 翻转版）做空腿盈亏诊断。

trade schema(11): [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
                   weight_at_entry, deferred_bars, partial, exit_reason, polarity]
pnl: long = shares*(exit-entry); short = shares*(entry-exit)  (NAV 中性会计)
record_trade 仅在 reduce_at 触发 ⟹ 每行 = 一次平仓 chunk，polarity = 被平层方向。
"""
import json, sys, os
import numpy as np
from collections import Counter, defaultdict

DC = os.path.join(os.path.dirname(__file__), "data_cache")
MODES = ["structural", "and", "or"]
INPUT = {
    "CL": "cl_1m_databento_10y.json", "BRN": "brn_1m_databento_10y.json",
    "DX": "dx_1m_databento_10y.json", "GC": "gc_1m_databento_10y.json",
    "ES": "es_1m_databento_10y.json", "QQQ": "qqq_1m_databento_full.json",
    "BTC": "btc_1m_full.json", "OKLO": "oklo_1m_databento.json",
}

def trade_pnl(t):
    _, eb, ep, xb, xp, sh, _, _, _, reason, pol = t
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)

def load_clean(path):
    """复刻 Rust load_clean_ohlc。返回 (closes, dates|None) 对齐 trade bar 索引。"""
    d = json.load(open(path))
    def to_arr(k):
        return np.array([x if x is not None else np.nan for x in d[k]], dtype=float)
    o, h, l, c = to_arr("opens"), to_arr("highs"), to_arr("lows"), to_arr("closes")
    dates = np.array(d["dates"], dtype=object) if "dates" in d else None
    k1 = np.isfinite(o) & np.isfinite(h) & np.isfinite(l) & np.isfinite(c)
    k1 &= (o > 0) & (h > 0) & (l > 0) & (c > 0)
    c = c[k1]
    if dates is not None: dates = dates[k1]
    if len(c) > 2:
        r1 = np.abs(c[1:-1] / c[:-2] - 1.0) > 0.5
        r2 = np.abs(c[2:] / c[:-2] - 1.0) < 0.05
        drop = np.zeros(len(c), bool); drop[1:-1] = r1 & r2
        if drop.any():
            keep = ~drop; c = c[keep]
            if dates is not None: dates = dates[keep]
    return c, dates

def load_trades(sym, mode):
    p = os.path.join(DC, f"t_engine_{sym}_{mode}_trades.json")
    return json.load(open(p)) if os.path.exists(p) else None

def q1_split(sym, mode):
    d = load_trades(sym, mode)
    if d is None: return None
    long_pnl = short_pnl = 0.0; n_long = n_short = 0
    long_hold = []; short_hold = []; long_win = short_win = 0
    reasons = Counter(); short_by_reason = defaultdict(float); long_by_reason = defaultdict(float)
    for t in d["trades"]:
        eb, xb, pol, reason = t[1], t[3], t[10], t[9]
        pnl = trade_pnl(t); reasons[reason] += 1; hold = xb - eb
        if pol == "long":
            long_pnl += pnl; n_long += 1; long_hold.append(hold); long_by_reason[reason] += pnl
            if pnl > 0: long_win += 1
        else:
            short_pnl += pnl; n_short += 1; short_hold.append(hold); short_by_reason[reason] += pnl
            if pnl > 0: short_win += 1
    return dict(sym=sym, mode=mode, final_nav=d["final_nav"], n_trades=d["n_trades"],
        long_pnl=long_pnl, short_pnl=short_pnl, n_long=n_long, n_short=n_short,
        long_winrate=long_win/max(n_long,1), short_winrate=short_win/max(n_short,1),
        long_hold_med=float(np.median(long_hold)) if long_hold else 0,
        short_hold_med=float(np.median(short_hold)) if short_hold else 0,
        reasons=dict(reasons),
        short_by_reason={k: round(v,1) for k,v in short_by_reason.items()},
        long_by_reason={k: round(v,1) for k,v in long_by_reason.items()},
        n_entries=d["n_entries_by_ladder"], n_cycle_opens=d["n_cycle_opens_by_ladder"],
        n_cycle_closes=d["n_cycle_closes_by_ladder"], n_liq=d["n_liquidations_by_ladder"],
        phys_long_bars=d["phys_long_bars"], phys_short_bars=d["phys_short_bars"])

def segment_split(sym, mode, closes, dates, k_seg=16):
    """通用等 bar 分段：每段标注价格涨跌 + 多空 pnl。揭示空头盈亏与 regime 的依赖。"""
    d = load_trades(sym, mode)
    if d is None or closes is None: return None
    n = len(closes); edges = np.linspace(0, n, k_seg + 1, dtype=int)
    seg = [dict(lo=int(edges[i]), hi=int(edges[i+1]), long_pnl=0.0, short_pnl=0.0,
                n_long=0, n_short=0) for i in range(k_seg)]
    def which(b):
        j = np.searchsorted(edges, b, side="right") - 1
        return min(max(j, 0), k_seg - 1)
    for t in d["trades"]:
        eb = t[1]
        if eb < 0 or eb >= n: continue
        j = which(eb); pnl = trade_pnl(t)
        if t[10] == "long": seg[j]["long_pnl"] += pnl; seg[j]["n_long"] += 1
        else: seg[j]["short_pnl"] += pnl; seg[j]["n_short"] += 1
    rows = []
    for s in seg:
        p0, p1 = closes[s["lo"]], closes[min(s["hi"], n-1)]
        chg = (p1/p0 - 1.0)*100 if p0 > 0 else 0
        lab = ""
        if dates is not None:
            lab = f"{str(dates[s['lo']])[:7]}~{str(dates[min(s['hi'],n-1)])[:7]}"
        rows.append(dict(span=lab, lo=s["lo"], hi=s["hi"], price_chg=round(chg,1),
                         long_pnl=round(s["long_pnl"],0), short_pnl=round(s["short_pnl"],0),
                         n_long=s["n_long"], n_short=s["n_short"]))
    # 上涨段 vs 下跌段 空头 pnl 聚合（核心判据）
    up_short = sum(r["short_pnl"] for r in rows if r["price_chg"] > 2)
    dn_short = sum(r["short_pnl"] for r in rows if r["price_chg"] < -2)
    up_long = sum(r["long_pnl"] for r in rows if r["price_chg"] > 2)
    dn_long = sum(r["long_pnl"] for r in rows if r["price_chg"] < -2)
    return dict(rows=rows, up_short=round(up_short,0), dn_short=round(dn_short,0),
                up_long=round(up_long,0), dn_long=round(dn_long,0))

def main():
    syms = sys.argv[1:] if len(sys.argv) > 1 else ["CL","BRN","ES","QQQ","BTC"]
    out = {}
    print("="*96)
    print("Q1 多空盈亏拆解（NAV 中性会计，单位=归一化资金 初始 1e5）")
    print("="*96)
    print(f"{'标的':<5}{'模式':<11}{'strat':>10}{'long_pnl':>11}{'short_pnl':>11}{'shortη':>8}{'n_L/n_S':>13}{'L胜/S胜':>12}{'L持/S持':>13}")
    for sym in syms:
        out[sym] = {}
        for mode in MODES:
            r = q1_split(sym, mode)
            if r is None: continue
            out[sym][mode] = r
            net = r["final_nav"] - 1e5
            short_frac = r["short_pnl"]/net*100 if net != 0 else 0
            print(f"{sym:<5}{mode:<11}{net:>+10.0f}{r['long_pnl']:>+11.0f}{r['short_pnl']:>+11.0f}"
                  f"{short_frac:>+7.0f}%{r['n_long']:>6}/{r['n_short']:<6}{r['long_winrate']:>4.0%}/{r['short_winrate']:<5.0%}"
                  f"{r['long_hold_med']:>6.0f}/{r['short_hold_med']:<6.0f}")
    print("\n注: shortη = 空头 pnl 占策略净盈亏的比例（负=空头是亏损源）; 持=持有 bar 中位数")

    print("\n"+"="*96)
    print("Q2/Q4/Q5 时段分桶（16 等分；空头在涨段 vs 跌段的盈亏 = 翻转机的 regime 依赖证据）")
    print("="*96)
    for sym in syms:
        ip = os.path.join(DC, INPUT[sym])
        if not os.path.exists(ip):
            print(f"[{sym}] 输入缺失"); continue
        print(f"\n[{sym}] 加载清洗 closes…", flush=True)
        closes, dates = load_clean(ip)
        rng = f"{str(dates[0])[:10]}→{str(dates[-1])[:10]}" if dates is not None else f"bar 0→{len(closes)}"
        print(f"[{sym}] cleaned bars={len(closes)} {rng}")
        rg = segment_split(sym, "structural", closes, dates)
        if rg is None: continue
        out.setdefault(sym, {}).setdefault("structural", {})["segments"] = rg
        print(f"  {'时段':<16}{'价格涨跌%':>10}{'long_pnl':>11}{'short_pnl':>11}{'n_S':>6}  判读")
        for r in rg["rows"]:
            tag = "↑" if r["price_chg"] > 2 else ("↓" if r["price_chg"] < -2 else "·")
            warn = "  ← 涨段空头亏" if (r["price_chg"] > 2 and r["short_pnl"] < 0) else \
                   ("  ← 跌段空头赚" if (r["price_chg"] < -2 and r["short_pnl"] > 0) else "")
            span = r["span"] if r["span"] else f"[{r['lo']}:{r['hi']}]"
            print(f"  {span:<16}{r['price_chg']:>+9.1f}{tag}{r['long_pnl']:>+11.0f}{r['short_pnl']:>+11.0f}{r['n_short']:>6}{warn}")
        print(f"  ── 聚合: 涨段空头={rg['up_short']:+.0f} | 跌段空头={rg['dn_short']:+.0f} "
              f"| 涨段多头={rg['up_long']:+.0f} | 跌段多头={rg['dn_long']:+.0f}")
        v = "空头方向正确(跌段赚/涨段亏)但涨段亏损淹没跌段盈利" if rg["up_short"] < 0 < rg["dn_short"] else "混合"
        print(f"  → {v}")

    with open(os.path.join(DC, "t_short_leg_diagnosis_out.json"), "w") as f:
        json.dump(out, f, ensure_ascii=False, indent=1, default=float)
    print("\n[写出] data_cache/t_short_leg_diagnosis_out.json")

if __name__ == "__main__":
    main()
