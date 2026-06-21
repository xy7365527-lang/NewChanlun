#!/usr/bin/env python3
"""涌现升级版（emergence ON, commit b590d4884f/fcba293dd9）T 引擎多空拆解。

数据来源：analysis/data_cache/t_engine_{SYM}_{mode}_trades.json
  —— 这些必须是 emergence ON（default，T_NO_EMERGENCE 未设）重跑生成的逐笔数据。
  注意：同目录的小 summary 文件 t_engine_{SYM}_{mode}.json 实为 emergence OFF
  消融基线（5f93 复现），不可混用（见会话诊断）。

trade schema（12 字段）:
  [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
   weight_at_entry, deferred_bars, partial, exit_reason, polarity, origin]

每笔已实现盈亏:
  long : (exit_price - entry_price) * shares
  short: (entry_price - exit_price) * shares
"""
import json
import os
import sys

BASE = os.path.join(os.path.dirname(__file__), "data_cache")
INITIAL = 100000.0
SYMS = ["BRN", "BTC", "CL", "DX", "ES", "GC", "OKLO", "QQQ"]
MODES = ["structural", "and", "or"]

# 字段索引
LADDER, EB, EP, XB, XP, SH, W, DEF, PART, REASON, POL, ORIG = range(12)


def trade_pnl(t):
    entry, exit_, sh = t[EP], t[XP], t[SH]
    if t[POL] == "long":
        return (exit_ - entry) * sh
    return (entry - exit_) * sh


def analyze(sym, mode):
    path = os.path.join(BASE, f"t_engine_{sym}_{mode}_trades.json")
    if not os.path.exists(path):
        return None
    d = json.load(open(path))
    trades = d["trades"]
    final_nav = d["final_nav"]
    strat_pct = (final_nav / INITIAL - 1.0) * 100.0
    plb, psb = d.get("phys_long_bars", 0), d.get("phys_short_bars", 0)

    long_pnl = short_pnl = 0.0
    n_long = n_short = 0
    long_hold = short_hold = 0  # 累计 bar 数（按笔，未乘 shares）
    # per-ladder × polarity
    nlad = 11
    lad_long = [0.0] * nlad
    lad_short = [0.0] * nlad
    for t in trades:
        p = trade_pnl(t)
        hold = t[XB] - t[EB]
        lad = t[LADDER] if t[LADDER] < nlad else nlad - 1
        if t[POL] == "long":
            long_pnl += p
            n_long += 1
            long_hold += hold
            lad_long[lad] += p
        else:
            short_pnl += p
            n_short += 1
            short_hold += hold
            lad_short[lad] += p

    return {
        "sym": sym, "mode": mode,
        "final_nav": final_nav, "strat_pct": strat_pct,
        "n_trades": len(trades),
        "long_pnl": long_pnl, "short_pnl": short_pnl,
        "n_long": n_long, "n_short": n_short,
        "long_avg_hold": long_hold / n_long if n_long else 0.0,
        "short_avg_hold": short_hold / n_short if n_short else 0.0,
        "phys_long_bars": plb, "phys_short_bars": psb,
        "lad_long": lad_long, "lad_short": lad_short,
        "pnl_sum": long_pnl + short_pnl,
        "nav_delta": final_nav - INITIAL,
    }


def fmt(x, w=12, dec=0):
    return f"{x:+{w},.{dec}f}"


def main():
    only = sys.argv[1:] if len(sys.argv) > 1 else SYMS
    results = {}
    for sym in only:
        for mode in MODES:
            r = analyze(sym, mode)
            if r:
                results[(sym, mode)] = r

    # ── 表1: 多空总盈亏拆解 ──
    print("=" * 110)
    print("涌现升级版（emergence ON）多空拆解 —— 做多/做空各赚了多少")
    print("=" * 110)
    hdr = f"{'标的':<6}{'模式':<11}{'strat%':>9}{'多头PnL':>13}{'空头PnL':>13}{'多笔':>6}{'空笔':>6}{'多均持bar':>11}{'空均持bar':>11}"
    print(hdr)
    print("-" * 110)
    for sym in only:
        for mode in MODES:
            r = results.get((sym, mode))
            if not r:
                continue
            print(f"{sym:<6}{mode:<11}{r['strat_pct']:>+8.1f} "
                  f"{fmt(r['long_pnl'],11)} {fmt(r['short_pnl'],11)} "
                  f"{r['n_long']:>6}{r['n_short']:>6}"
                  f"{r['long_avg_hold']:>11,.0f}{r['short_avg_hold']:>11,.0f}")
        print("." * 110)

    # ── 表2: 自洽性检查 (pnl_sum vs nav_delta) ──
    print("\n" + "=" * 80)
    print("自洽性检查: 逐笔PnL求和 vs (final_nav - INITIAL) —— 差异=成本/未平仓/部分平仓")
    print("=" * 80)
    print(f"{'标的':<6}{'模式':<11}{'PnL求和':>14}{'nav_delta':>14}{'差异':>12}")
    for sym in only:
        for mode in MODES:
            r = results.get((sym, mode))
            if not r:
                continue
            diff = r["pnl_sum"] - r["nav_delta"]
            print(f"{sym:<6}{mode:<11}{fmt(r['pnl_sum'],13)} {fmt(r['nav_delta'],13)} {fmt(diff,11)}")

    # ── 表3: per-ladder × polarity (仅 structural, 高/低 level 盈亏) ──
    print("\n" + "=" * 100)
    print("per-level × 多空 盈亏 (structural 模式) —— 高level多头 vs 低level短差")
    print("=" * 100)
    print(f"{'标的':<6}" + "".join(f"L{i:<1}多/空".rjust(16) for i in range(6)))
    for sym in only:
        r = results.get((sym, "structural"))
        if not r:
            continue
        cells = []
        for i in range(6):
            cells.append(f"{r['lad_long'][i]:+.0f}/{r['lad_short'][i]:+.0f}".rjust(16))
        print(f"{sym:<6}" + "".join(cells))

    return results


if __name__ == "__main__":
    main()
