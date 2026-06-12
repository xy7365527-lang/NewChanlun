"""调研探针：为什么 ES/BTC/GC 上跑输 BH？（纯调研，不改生产代码）

逐笔分解 V2oa25_ht：
1. log 空间守恒分解：ln(1+R_BH) = Σ持仓期BH + Σ空仓期BH
   策略 in-trade alpha = Σ[ln(1+r_trade) − ln(1+bh_in_trade)]
   跑输差额 = in-trade alpha − 空仓期错失
2. 空仓期：踏空（gap 上涨）vs 避跌（gap 下跌）分解
3. 出场→再入场精度：reentry/exit 价比分布（>1 = 追高回补）
4. 腿分解（diag）：main/osc/rev 的 net profit
5. 出场距段内顶部的位置

用法：BT_SYMBOLS=OKLO,BRN,ES,GC,BTC PYTHONPATH=src .venv/bin/python \
      analysis/_why_underperform_probe.py
输出：analysis/data_cache/_why_underperform_probe.json
"""

from __future__ import annotations

import json
import math
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
VARIANT = "V2oa25_ht"


def _q(xs: list, p: float):
    if not xs:
        return None
    s = sorted(xs)
    return round(s[min(len(s) - 1, int(p * (len(s) - 1)))], 4)


def decompose(symbol: str) -> dict:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    years_span = (years[-1] - years[0] + 1) if years else None
    bh = closes[-1] / closes[0]

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    sig_s = time.time() - t0
    res = nr.run_organic_rust(rtape, VARIANT, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    trades = res["trades"]
    print(f"  [{symbol}] sig={sig_s:.1f}s trades={len(trades)}", flush=True)

    # ── 1. log 空间守恒分解 ──
    ln_strat = 0.0          # Σ ln(1+r_trade)
    ln_bh_in = 0.0          # Σ ln(持仓期 close-to-close BH)
    in_alpha_rows = []      # 每笔 ln(1+r) − ln(bh_in)
    hold_bars = 0
    for t in trades:
        eb, ep, xb, xp, pnl = t[0], t[1], t[2], t[3], t[4]
        r = math.log(1 + pnl / 100.0)
        b = math.log(closes[xb] / closes[eb])
        ln_strat += r
        ln_bh_in += b
        in_alpha_rows.append(r - b)
        hold_bars += xb - eb

    # 空仓期（含首尾），逐 gap 的 BH log return
    gaps = []
    prev_end = 0
    for t in trades:
        if t[0] > prev_end:
            gaps.append((prev_end, t[0]))
        prev_end = t[2]
    if prev_end < n - 1:
        gaps.append((prev_end, n - 1))
    gap_rows = [(a, b, math.log(closes[b] / closes[a])) for a, b in gaps]
    ln_gap = sum(g[2] for g in gap_rows)
    ln_gap_pos = sum(g[2] for g in gap_rows if g[2] > 0)   # 踏空
    ln_gap_neg = sum(g[2] for g in gap_rows if g[2] < 0)   # 避跌
    gap_bars = sum(b - a for a, b, _ in gap_rows)

    # ── 2. 出场→再入场精度 ──
    reentry_ratio = []   # next_entry_price / exit_price（>1 = 追高）
    gap_peak_ratio = []  # gap 期间最高价 / exit_price（错过的最大涨幅）
    for i in range(len(trades) - 1):
        xp = trades[i][3]
        xb = trades[i][2]
        ne_b, ne_p = trades[i + 1][0], trades[i + 1][1]
        if xp > 0:
            reentry_ratio.append(ne_p / xp)
            seg_hi = max(highs[xb:ne_b + 1]) if ne_b > xb else highs[xb]
            gap_peak_ratio.append(seg_hi / xp)

    # ── 3. 出场距段内顶部 ──
    exit_vs_peak = []    # exit_price / max(high[entry:exit])
    for t in trades:
        eb, xb, xp = t[0], t[2], t[3]
        peak = max(highs[eb:xb + 1])
        if peak > 0:
            exit_vs_peak.append(xp / peak)

    # ── 4. 腿分解（diag）──
    legs = {"main": [0, 0.0], "osc": [0, 0.0], "rev": [0, 0.0]}
    for _header, diffs in res["diag"]:
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, c0, c1) in diffs:
            legs[leg][0] += 1
            legs[leg][1] += pf

    return {
        "n_bars": n, "years_span": years_span,
        "bh_pct": round((bh - 1) * 100, 1),
        "n_trades": len(trades),
        "exposure": round(hold_bars / n, 4),
        "strat_total_pct": round((math.exp(ln_strat) - 1) * 100, 1),
        "log_decomposition": {
            "ln_bh_total": round(math.log(bh), 4),
            "ln_strat": round(ln_strat, 4),
            "ln_bh_in_trade": round(ln_bh_in, 4),
            "in_trade_alpha": round(ln_strat - ln_bh_in, 4),
            "ln_gap_total": round(ln_gap, 4),
            "ln_gap_missed_up": round(ln_gap_pos, 4),
            "ln_gap_avoided_down": round(ln_gap_neg, 4),
            "underperform_check": round(
                ln_strat - (ln_bh_in + ln_gap) - (ln_strat - math.log(bh)), 4),
        },
        "gap_stats": {
            "n_gaps": len(gap_rows), "gap_bars": gap_bars,
            "gap_frac": round(gap_bars / n, 4),
            "n_gap_up": sum(1 for g in gap_rows if g[2] > 0),
            "n_gap_down": sum(1 for g in gap_rows if g[2] < 0),
        },
        "reentry_ratio": {
            "p25": _q(reentry_ratio, .25), "p50": _q(reentry_ratio, .5),
            "p75": _q(reentry_ratio, .75),
            "frac_above_exit": round(
                sum(1 for r in reentry_ratio if r > 1) / len(reentry_ratio), 4)
            if reentry_ratio else None,
        },
        "gap_peak_ratio": {
            "p50": _q(gap_peak_ratio, .5), "p90": _q(gap_peak_ratio, .9),
        },
        "exit_vs_peak": {
            "p25": _q(exit_vs_peak, .25), "p50": _q(exit_vs_peak, .5),
            "p75": _q(exit_vs_peak, .75),
        },
        "in_trade_alpha_per_trade": {
            "p25": _q(in_alpha_rows, .25), "p50": _q(in_alpha_rows, .5),
            "p75": _q(in_alpha_rows, .75),
        },
        "legs": {k: {"n_pairs": v[0], "net_profit": round(v[1], 1)}
                 for k, v in legs.items()},
    }


def main() -> None:
    out_path = DATA_DIR / "_why_underperform_probe.json"
    out = json.loads(out_path.read_text()) if out_path.exists() else {}
    for sym in SYMBOLS:
        if sym in out and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym}", flush=True)
            continue
        print(f"== {sym} ==", flush=True)
        out[sym] = decompose(sym)
        out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False))
        print(json.dumps(out[sym]["log_decomposition"], indent=2), flush=True)
    print("[done]", flush=True)


if __name__ == "__main__":
    main()
