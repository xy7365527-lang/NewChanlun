"""O1 REV 腿亏损归因诊断（OKLO）——临时分析脚本。

三块分析：
  A. master 通道：O1 vs P5 进出场路径（鞭打成本 = 出场后更高价再入场）
  B. REV 腿独立质量：亏损分布 / 持有时间 / 触发类型 / 走势语境分类
  C. osc 腿被 T1 强平的量化（O1v vs P5）
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import PAIRING_VARIANTS, run_version_i  # noqa: E402
from organic_fugue import ORGANIC_VARIANTS, run_organic  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

LADDER_SEG = 2
REV_OFF = 200

sym = sys.argv[1] if len(sys.argv) > 1 else "OKLO"
opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[sym])
print(f"{sym}: {len(closes)} bars", flush=True)
tape = compute_organic_signals(opens, highs, lows, closes)
print("signals done", flush=True)

diag_p5: list = []
trades_p5, _ = run_version_i(tape, floor_ladder=LADDER_SEG,
                             pairing=PAIRING_VARIANTS["P5"], diag=diag_p5)
diag_o1: list = []
trades_o1, _ = run_organic(tape, floor_ladder=LADDER_SEG,
                           config=ORGANIC_VARIANTS["O1"], diag=diag_o1)
diag_o1v: list = []
trades_o1v, _ = run_organic(tape, floor_ladder=LADDER_SEG,
                            config=ORGANIC_VARIANTS["O1v"], diag=diag_o1v)
print(f"runs done: P5 {len(trades_p5)} | O1 {len(trades_o1)} "
      f"| O1v {len(trades_o1v)} trades", flush=True)

n_bars = len(closes)


def master_path(trades):
    held = sum(t.exit_bar - t.entry_bar for t in trades)
    comp = 1.0
    for t in trades:
        comp *= 1 + t.pnl_pct / 100
    # 鞭打成本：上一笔出场价 → 下一笔再入场价的价差（再入场更贵=负贡献）
    whip = []
    for a, b in zip(trades, trades[1:]):
        whip.append(b.entry_price / a.exit_price - 1)
    n_higher = sum(1 for w in whip if w > 0)
    sum_log_whip = sum(__import__("math").log1p(w) for w in whip)
    return {
        "n": len(trades),
        "time_in_mkt_pct": round(held / n_bars * 100, 1),
        "compound_pct": round((comp - 1) * 100, 1),
        "mean_pnl_pct": round(sum(t.pnl_pct for t in trades) / len(trades), 3),
        "win_rate": round(sum(1 for t in trades if t.pnl_pct > 0)
                          / len(trades) * 100, 1),
        "reentry_higher_pct": round(n_higher / len(whip) * 100, 1) if whip else None,
        "whipsaw_log_sum": round(sum_log_whip, 4),
        "whipsaw_compound_pct": round(
            (__import__("math").exp(sum_log_whip) - 1) * 100, 1) if whip else None,
    }


print("\n=== A. master 通道（进出场路径） ===")
for name, tr in (("P5", trades_p5), ("O1", trades_o1), ("O1v", trades_o1v)):
    print(name, master_path(tr))

bh = closes[-1] / closes[0] - 1
print(f"BH: {bh*100:.1f}%")

# 出场原因分布
from collections import Counter  # noqa: E402
print("\nO1 exit reasons:", dict(Counter(t.exit_reason for t in trades_o1)))
print("P5 exit reasons:", dict(Counter(t.exit_reason for t in trades_p5)))


def rev_legs(diag):
    legs = []
    for tr in diag:
        for r in tr["diffs"]:
            if r.get("leg") == "rev":
                legs.append(r)
    return legs


def classify_trigger(sell_bar, ladder):
    """重扫磁带：sell_bar 时该 ladder 上哪个段终结触发命中。"""
    sig = tape[sell_bar]
    evs = sig.bsp_events[ladder] if sig.bsp_events else ()
    devs = sig.div_events[ladder] if sig.div_events else ()
    tags = []
    for e in evs:
        if e[3] and e[1] == "sell" and e[0] in ("type1", "type3"):
            tags.append(e[0] + "_sell")
    for d in devs:
        if d[2] == "sell":
            tags.append("div_" + d[0])  # d[0]: trend / consolidation
    return tags or ["none"]


def regime(bar, w=2000):
    """语境：开腿前 w bar 的价格变化（>+2% 上行 / <-2% 下行 / 否则盘整）。"""
    lo = max(0, bar - w)
    r = closes[bar] / closes[lo] - 1
    if r > 0.02:
        return "uptrend"
    if r < -0.02:
        return "downtrend"
    return "flat"


def leg_report(name, legs):
    print(f"\n=== B. {name} REV 腿（n={len(legs)}） ===")
    if not legs:
        return
    total = sum(r["profit"] for r in legs)
    losses = sorted((r for r in legs if r["profit"] < 0),
                    key=lambda r: r["profit"])
    wins = [r for r in legs if r["profit"] > 0]
    sum_loss = sum(r["profit"] for r in losses)
    sum_win = sum(r["profit"] for r in wins)
    top10 = sum(r["profit"] for r in losses[:10])
    print(f"net={total:+.0f}  win_sum={sum_win:+.0f}  loss_sum={sum_loss:+.0f}"
          f"  top10_loss={top10:+.0f} ({top10/sum_loss*100:.0f}% of losses)"
          if losses else f"net={total:+.0f}")
    hold = [r["buy_bar"] - r["sell_bar"] for r in legs if r["buy_bar"] >= 0]
    hold.sort()
    print(f"持有bar: median={hold[len(hold)//2]} p90={hold[int(len(hold)*.9)]}"
          f" max={hold[-1]}")
    # 按触发类型
    by_trig: dict[str, list] = {}
    for r in legs:
        lad = r["ladder"] - REV_OFF
        for t in classify_trigger(r["sell_bar"], lad):
            by_trig.setdefault(t, []).append(r)
    print("按触发类型（开腿）:")
    for t, rows in sorted(by_trig.items(), key=lambda kv: sum(
            r['profit'] for r in kv[1])):
        net = sum(r["profit"] for r in rows)
        wr = sum(1 for r in rows if r["diff"] > 0) / len(rows) * 100
        avg = sum(r["diff"] / r["sell_price"] for r in rows) / len(rows) * 100
        print(f"  {t:22s} n={len(rows):5d} net={net:+9.0f} win={wr:4.1f}%"
              f" avg_diff={avg:+.4f}%")
    # 按走势语境
    by_reg: dict[str, list] = {}
    for r in legs:
        by_reg.setdefault(regime(r["sell_bar"]), []).append(r)
    print("按开腿前2000bar走势语境:")
    for g, rows in sorted(by_reg.items(), key=lambda kv: sum(
            r['profit'] for r in kv[1])):
        net = sum(r["profit"] for r in rows)
        wr = sum(1 for r in rows if r["diff"] > 0) / len(rows) * 100
        avg = sum(r["diff"] / r["sell_price"] for r in rows) / len(rows) * 100
        print(f"  {g:10s} n={len(rows):5d} net={net:+9.0f} win={wr:4.1f}%"
              f" avg_diff={avg:+.4f}%")


leg_report("O1", rev_legs(diag_o1))
leg_report("O1v", rev_legs(diag_o1v))

# C. osc 腿对比（O1v vs P5）：T1 强平识别 = osc buy_bar 与同 ladder rev
# sell_bar 同 bar
print("\n=== C. osc 腿被 T1 强平（O1v vs P5） ===")


def osc_legs(diag):
    out = []
    for tr in diag:
        revs = {(r["ladder"] - REV_OFF, r["sell_bar"])
                for r in tr["diffs"] if r.get("leg") == "rev"}
        for r in tr["diffs"]:
            leg = r.get("leg") or ("osc" if r["ladder"] >= 100 else "main")
            if leg == "osc":
                forced = (r["ladder"] - 100, r["buy_bar"]) in revs
                out.append((r, forced))
    return out


for name, diag in (("P5", diag_p5), ("O1v", diag_o1v)):
    legs = osc_legs(diag)
    norm = [r for r, f in legs if not f]
    forc = [r for r, f in legs if f]
    for sub, rows in (("normal", norm), ("T1-forced", forc)):
        if not rows:
            print(f"{name:4s} {sub:10s} n=0")
            continue
        net = sum(r["profit"] for r in rows)
        wr = sum(1 for r in rows if r["diff"] > 0) / len(rows) * 100
        avg = sum(r["diff"] / r["sell_price"] for r in rows) / len(rows) * 100
        print(f"{name:4s} {sub:10s} n={len(rows):4d} net={net:+9.0f}"
              f" win={wr:4.1f}% avg_diff={avg:+.4f}%")

# 落盘原始 rev legs 供复查
out = {
    "symbol": sym,
    "master": {n: master_path(t) for n, t in
               (("P5", trades_p5), ("O1", trades_o1), ("O1v", trades_o1v))},
}
Path(ROOT / "analysis" / "_o1_rev_loss_diag_out.json").write_text(
    json.dumps(out, indent=1))
print("\nsaved _o1_rev_loss_diag_out.json")
