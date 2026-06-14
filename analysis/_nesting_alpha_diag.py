"""区间套 alpha 诊断（编排者追加 2026-06-14）——回答区间套接入能否改善的 4 个问题。

Q1: 按 ladder 分组，每级别买卖点配对平均幅度（close 净额）。
Q2: 高级别(ladder≥3 move(L1)+) vs 低级别(ladder=2 segment) 幅度差异倍数。
Q3: 反事实——只消费单一级别 BSP 的交替策略累计收益（close 执行扣 0.2% 往返摩擦）。
    long-only：flat 见 buy→开多，long 见 sell→平多（兑现 close-to-close 净幅），忽略 long 期 buy。
    long+short：平多同时反手开空，见 buy 平空反手。非重叠、因果、可实现。
Q4: 利润集中度——高级别 type1 占引擎层视图总利润比例（读 _why_not_profitable_*.json）。

注：ladder 2=segment(笔中枢级,最低可交易) 3=move(L1) 4=recL2 5=recL3 6=recL4；无 ladder 0/1 BSP。

用法: PYTHONPATH=src .venv/bin/python analysis/_nesting_alpha_diag.py [SYM ...]
输出: analysis/data_cache/_nesting_alpha_<SYM>.json
"""
from __future__ import annotations

import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import LADDER_NAMES  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "CL"]
FRIC_LEG = 0.001
RT = 2 * FRIC_LEG  # 往返摩擦


def collect_bsps(tape) -> dict:
    by_lev: dict[int, list] = {}
    for i, s in enumerate(tape):
        evs = s.bsp_events
        for lad in range(len(evs)):
            row = evs[lad]
            if not row:
                continue
            for ev in row:
                if len(ev) < 8 or not ev[3]:
                    continue
                price = ev[7]
                if price is None or (isinstance(price, float) and math.isnan(price)):
                    continue
                by_lev.setdefault(lad, []).append((i, ev[1], ev[0], float(price)))
    for lad in by_lev:
        by_lev[lad].sort(key=lambda r: r[0])
    return by_lev


def alt_sim(seq, closes, allow_short: bool) -> dict:
    """单级别交替策略：close 执行，每腿扣 FRIC_LEG×2。返回累计收益% + 腿统计。"""
    pos = 0
    entry = None
    cum = 1.0
    legs_long, legs_short = [], []
    for (bar, side, _kind, _price) in seq:
        c = closes[bar]
        if pos == 0:
            if side == "buy":
                pos, entry = 1, c
            elif side == "sell" and allow_short:
                pos, entry = -1, c
        elif pos == 1:
            if side == "sell":
                r = (c / entry - 1) - RT
                cum *= (1 + r)
                legs_long.append(r)
                if allow_short:
                    pos, entry = -1, c
                else:
                    pos, entry = 0, None
        elif pos == -1:
            if side == "buy":
                r = (entry / c - 1) - RT
                cum *= (1 + r)
                legs_short.append(r)
                pos, entry = 1, c
    alllegs = legs_long + legs_short
    return {
        "cum_return_pct": round((cum - 1) * 100, 1),
        "n_round_trips": len(alllegs),
        "n_long_legs": len(legs_long), "n_short_legs": len(legs_short),
        "win_rate": round(sum(1 for r in alllegs if r > 0) / len(alllegs), 3) if alllegs else None,
        "avg_leg_net_pct": round(sum(alllegs) / len(alllegs) * 100, 4) if alllegs else None,
    }


def pair_amp(seq, closes) -> dict:
    """buy→下一 sell close 净幅（与 _bsp_pairing 同口径，用于 Q1/Q2）。"""
    n = len(seq)
    next_sell = [None] * n
    nxt = None
    for k in range(n - 1, -1, -1):
        if seq[k][1] == "sell":
            nxt = k
        next_sell[k] = nxt if seq[k][1] != "sell" else None
    amps = []
    for k in range(n):
        if seq[k][1] != "buy":
            continue
        j = next_sell[k]
        if j is None:
            continue
        amps.append((closes[seq[j][0]] - closes[seq[k][0]]) / closes[seq[k][0]])
    if not amps:
        return {"n": 0}
    g = sum(amps) / len(amps)
    return {
        "n": len(amps),
        "close_gross_pct": round(g * 100, 4),
        "close_net_pct": round((g - RT) * 100, 4),
        "dir_correct": round(sum(1 for a in amps if a > 0) / len(amps), 4),
    }


def profit_concentration(sym: str) -> dict:
    """读引擎逐笔层视图（_why_not_profitable_<SYM>.json）算高级别 type1 占总利润比例。"""
    p = DATA_DIR / f"_why_not_profitable_{sym}.json"
    if not p.exists():
        return {}
    d = json.loads(p.read_text())
    bk = d.get("by_kind", {})
    bl = d.get("by_level", {})
    total = d.get("total_pnl_cash", 0) or 0
    pos_total = sum(v["pnl_cash"] for v in bl.values() if v.get("pnl_cash", 0) > 0)
    type1 = bk.get("type1", {}).get("pnl_cash", 0)
    # 高级别(move(L1)+)多头层视图利润
    hi_long = sum(v["pnl_cash"] for k, v in bl.items()
                  if k.split("/")[0] in ("move(L1)", "recL2", "recL3", "recL4") and "long" in k)
    seg = sum(v["pnl_cash"] for k, v in bl.items() if k.split("/")[0] == "segment")
    return {
        "total_pnl_cash": total,
        "positive_pnl_sum": round(pos_total, 0),
        "type1_pnl": type1,
        "type1_share_of_total": round(type1 / total, 3) if total else None,
        "type1_share_of_positive": round(type1 / pos_total, 3) if pos_total else None,
        "high_level_long_pnl": round(hi_long, 0),
        "high_level_long_share_of_total": round(hi_long / total, 3) if total else None,
        "segment_all_pnl": round(seg, 0),
        "top1_trade_pct_of_total": d.get("top3_pnl", [{}])[0].get("pct_of_total"),
    }


def analyze_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, _years = load_ohlc(path)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=[], require_settled=True)
    print(f"[{sym}] signal {time.time()-t0:.1f}s bars={len(closes):,} BH={bh_pct:+.1f}%", flush=True)
    by_lev = collect_bsps(tape)

    per_ladder = {}
    for lad, seq in sorted(by_lev.items()):
        name = LADDER_NAMES.get(lad, str(lad))
        amp = pair_amp(seq, closes)
        lo = alt_sim(seq, closes, allow_short=False)
        ls = alt_sim(seq, closes, allow_short=True)
        per_ladder[name] = {
            "ladder": lad,
            "n_buy": sum(1 for r in seq if r[1] == "buy"),
            "n_sell": sum(1 for r in seq if r[1] == "sell"),
            "pair_amp": amp,
            "alt_long_only": lo,
            "alt_long_short": ls,
        }
        print(f"  [{sym}] {name:10}(lad{lad}) pair_net={amp.get('close_net_pct')}% "
              f"LO_cum={lo['cum_return_pct']}%({lo['n_round_trips']}rt) "
              f"LS_cum={ls['cum_return_pct']}%", flush=True)

    # Q2: 高/低幅度倍数
    seg = per_ladder.get("segment", {}).get("pair_amp", {}).get("close_gross_pct")
    ratios = {}
    for name in ("move(L1)", "recL2", "recL3", "recL4"):
        g = per_ladder.get(name, {}).get("pair_amp", {}).get("close_gross_pct")
        if g is not None and seg not in (None, 0):
            ratios[f"{name}/segment"] = round(g / seg, 1)

    return {
        "symbol": sym, "n_bars": len(closes), "bh_pct": round(bh_pct, 1),
        "per_ladder": per_ladder,
        "amp_ratio_vs_segment": ratios,
        "profit_concentration": profit_concentration(sym),
    }


def main() -> None:
    for sym in SYMBOLS:
        try:
            out = analyze_symbol(sym)
        except Exception as e:  # noqa: BLE001
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
        (DATA_DIR / f"_nesting_alpha_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
    print("DONE", flush=True)


if __name__ == "__main__":
    main()
