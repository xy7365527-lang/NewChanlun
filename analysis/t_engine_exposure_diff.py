#!/usr/bin/env python3
"""涌现升级 ON vs OFF 敞口对比 —— 回答「敞口为什么降了」。

ON  数据: /tmp/t_on_backup/   (emergence ON, default 重跑)
OFF 数据: analysis/data_cache/ (emergence OFF, T_NO_EMERGENCE=1 重跑覆盖)

对比维度:
  - exposure_series 时间平均 long_units / short_units / 净敞口 / 毛敞口
  - sink(n_cycle_opens) vs recover(n_cycle_closes) 配平率 —— 减仓1/3是否归还
  - entries / liquidations by ladder
"""
import json
import os

ON_DIR = "/tmp/t_on_backup"
OFF_DIR = os.path.join(os.path.dirname(__file__), "data_cache")
INITIAL = 100000.0
LADDER, EB, EP, XB, XP, SH, W, DEF, PART, REASON, POL, ORIG = range(12)


def load(d, sym, mode):
    p = os.path.join(d, f"t_engine_{sym}_{mode}_trades.json")
    if not os.path.exists(p):
        return None
    return json.load(open(p))


def expo_avg(d):
    """exposure_series = [[bar, long_u, short_u], ...] 时间平均。"""
    es = d.get("exposure_series", [])
    if not es:
        return 0.0, 0.0
    lu = sum(x[1] for x in es) / len(es)
    su = sum(x[2] for x in es) / len(es)
    return lu, su


def ls_pnl(d):
    lp = sp = 0.0
    for t in d["trades"]:
        pnl = (t[XP] - t[EP]) * t[SH] if t[POL] == "long" else (t[EP] - t[XP]) * t[SH]
        if t[POL] == "long":
            lp += pnl
        else:
            sp += pnl
    return lp, sp


def total(arr):
    return sum(arr) if arr else 0


def main():
    syms = ["CL", "OKLO", "ES", "BTC"]
    mode = "structural"
    print("=" * 118)
    print(f"涌现升级 ON vs OFF 敞口对比 (mode={mode}) —— 敞口为什么降了")
    print("=" * 118)
    print(f"{'标的':<6}{'版本':<5}{'strat%':>8}{'avg多units':>12}{'avg空units':>12}"
          f"{'净敞口':>11}{'毛敞口':>11}{'多PnL':>10}{'空PnL':>10}")
    print("-" * 118)
    for sym in syms:
        for label, d in [("ON", load(ON_DIR, sym, mode)), ("OFF", load(OFF_DIR, sym, mode))]:
            if not d:
                print(f"{sym:<6}{label:<5}  (缺)")
                continue
            lu, su = expo_avg(d)
            lp, sp = ls_pnl(d)
            strat = (d["final_nav"] / INITIAL - 1.0) * 100.0
            print(f"{sym:<6}{label:<5}{strat:>+7.1f} {lu:>12,.1f}{su:>12,.1f}"
                  f"{lu - su:>+11,.1f}{lu + su:>11,.1f}{lp:>+10,.0f}{sp:>+10,.0f}")
        print("." * 118)

    # ── sink/recover 配平 + entries/liq ──
    print("\n" + "=" * 100)
    print("sink(cycle_opens) vs recover(cycle_closes) 配平率 —— 减仓1/3是否归还 + entries/liq")
    print("=" * 100)
    print(f"{'标的':<6}{'版本':<5}{'entries':>9}{'sink':>8}{'recover':>9}{'rec/sink':>10}{'liq':>8}{'净升级':>8}")
    for sym in syms:
        for label, d in [("ON", load(ON_DIR, sym, mode)), ("OFF", load(OFF_DIR, sym, mode))]:
            if not d:
                continue
            ent = total(d.get("n_entries_by_ladder", []))
            opn = total(d.get("n_cycle_opens_by_ladder", []))
            cls = total(d.get("n_cycle_closes_by_ladder", []))
            liq = total(d.get("n_liquidations_by_ladder", []))
            ratio = cls / opn if opn else 0.0
            print(f"{sym:<6}{label:<5}{ent:>9}{opn:>8}{cls:>9}{ratio:>9.1%}{liq:>8}")
        print("." * 100)

    # ── per-ladder sink/recover (CL 重点) ──
    print("\n" + "=" * 90)
    print("CL per-ladder: sink(opens)/recover(closes)/liq —— 哪个 level 减仓不归还")
    print("=" * 90)
    for label, d in [("ON", load(ON_DIR, "CL", mode)), ("OFF", load(OFF_DIR, "CL", mode))]:
        if not d:
            continue
        opn = d.get("n_cycle_opens_by_ladder", [])
        cls = d.get("n_cycle_closes_by_ladder", [])
        liq = d.get("n_liquidations_by_ladder", [])
        ent = d.get("n_entries_by_ladder", [])
        print(f"\n[CL {label}]  (L: entries sink recover liq)")
        for k in range(11):
            e = ent[k] if k < len(ent) else 0
            o = opn[k] if k < len(opn) else 0
            c = cls[k] if k < len(cls) else 0
            lq = liq[k] if k < len(liq) else 0
            if e or o or c or lq:
                print(f"  L{k}: entries={e:<6} sink={o:<6} recover={c:<6} liq={lq:<6}")


if __name__ == "__main__":
    main()
