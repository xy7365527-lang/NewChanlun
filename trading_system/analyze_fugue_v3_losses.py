#!/usr/bin/env python3
"""fugue_v3 深亏标的交易行为解剖。

读取 data_cache/fugue_v3_<SYM>.json，对每个标的做五维拆解：
  1. 按方向(Long/Short) × exit_reason × ladder 的盈亏拆解
  2. 亏损最大单笔 top-N
  3. 盈亏的时间分布（按 exit_bar 分箱 + 权益曲线 MDD 定位）
  4. sink(reduce) / recover 配对分析
  5. 跨标的对比（ES/BTC/QQQ 深亏 vs CL 健康）

定义依据（rust/src/fugue_v3/cycle.rs）：
  sink @ k  : reduce_at(k,"reduce") + add_at(k-1, flip(d_k))  —— 取 θ 配额下沉，子级别开反手性 chunk
  recover @ k: reduce_at(k-1,"recover") + add_at(k, ...)       —— 子级别 chunk 升回父级别
  完整 sink→recover 净 free += 2m·(c_sink − c_recover)        —— 需价格在两点间下跌才盈利
  per-trade pnl = (exit-entry)*shares [long] / (entry-exit)*shares [short]，求和 == mobile_realized_pnl
"""
import json
import os
from collections import defaultdict, Counter

CACHE = os.path.join(os.path.dirname(__file__), "data_cache")
FOCUS = ["ES", "BTC", "QQQ", "CL"]   # 三个深亏 + 一个健康对照


def ladder_name(lad: int) -> str:
    if lad == 2:
        return "segment"
    if lad == 3:
        return "move(L1)"
    if lad >= 4:
        return f"recL{lad - 2}"
    return f"L{lad}"


def load(sym):
    with open(os.path.join(CACHE, f"fugue_v3_{sym}.json")) as f:
        return json.load(f)


def trade_view(d):
    """返回 (idx, trades_as_dicts)，每笔附 pnl / held。"""
    sch = d["trade_schema"]
    idx = {k: i for i, k in enumerate(sch)}
    out = []
    for t in d["trades"]:
        e = t[idx["entry_price"]]
        x = t[idx["exit_price"]]
        s = t[idx["shares"]]
        pol = t[idx["polarity"]]
        pnl = (x - e) * s if pol == "long" else (e - x) * s
        out.append({
            "ladder": t[idx["ladder"]],
            "lname": ladder_name(t[idx["ladder"]]),
            "entry_bar": t[idx["entry_bar"]],
            "exit_bar": t[idx["exit_bar"]],
            "held": t[idx["exit_bar"]] - t[idx["entry_bar"]],
            "entry_price": e,
            "exit_price": x,
            "shares": s,
            "exit_reason": t[idx["exit_reason"]],
            "polarity": pol,
            "partial": t[idx["partial"]],
            "pnl": pnl,
        })
    return out


def fmt_money(v):
    return f"{v:>14,.0f}"


def section_header(t):
    print("\n" + "=" * 78)
    print(t)
    print("=" * 78)


def analyze_symbol(sym):
    d = load(sym)
    tv = trade_view(d)
    base = 100000.0
    section_header(f"【{sym}】 strat={d['strat_pct']}%  BH={d['bh_pct']}%  "
                   f"BH_MDD={d['bh_mdd_pct']}%  strat_MDD={d['mdd_pct']}%  "
                   f"n_trades={d['n_trades']}  liq={d['liquidations']}")
    tot = sum(t["pnl"] for t in tv)
    print(f"  现金总盈亏 = {tot:,.0f}  (= mobile_realized_pnl {d['mobile_realized_pnl']:,.0f}; "
          f"基准账户 {base:,.0f} → {tot/base*100:+.1f}%)")
    print(f"  价格区间: {d['closes_first']:.2f} → {d['closes_last']:.2f} "
          f"({(d['closes_last']/d['closes_first']-1)*100:+.0f}%)  n_bars={d['n_bars']:,}")

    # ---- 1A. 方向拆解 ----
    print("\n  [1A] 按方向(polarity):")
    by_pol = defaultdict(lambda: [0, 0.0, 0])  # n, pnl, wins
    for t in tv:
        rec = by_pol[t["polarity"]]
        rec[0] += 1
        rec[1] += t["pnl"]
        rec[2] += 1 if t["pnl"] > 0 else 0
    for pol in ("long", "short"):
        if pol in by_pol:
            n, pnl, w = by_pol[pol]
            print(f"    {pol:>6}: n={n:>5}  pnl={fmt_money(pnl)}  "
                  f"win%={w/n*100:>5.1f}  avg={pnl/n:>10,.0f}")

    # ---- 1B. exit_reason 拆解 ----
    print("\n  [1B] 按 exit_reason:")
    by_rs = defaultdict(lambda: [0, 0.0, 0, 0.0])  # n, pnl, wins, held_sum
    for t in tv:
        rec = by_rs[t["exit_reason"]]
        rec[0] += 1
        rec[1] += t["pnl"]
        rec[2] += 1 if t["pnl"] > 0 else 0
        rec[3] += t["held"]
    for rs, (n, pnl, w, hs) in sorted(by_rs.items(), key=lambda kv: kv[1][1]):
        print(f"    {rs:>16}: n={n:>5}  pnl={fmt_money(pnl)}  "
              f"win%={w/n*100:>5.1f}  avg_held={hs/n:>12,.0f}bar")

    # ---- 1C. ladder 拆解 ----
    print("\n  [1C] 按 ladder × polarity (亏损升序):")
    by_lad = defaultdict(lambda: [0, 0.0, 0, 0.0])
    for t in tv:
        key = f"{t['lname']}/{t['polarity']}"
        rec = by_lad[key]
        rec[0] += 1
        rec[1] += t["pnl"]
        rec[2] += 1 if t["pnl"] > 0 else 0
        rec[3] += t["held"]
    for key, (n, pnl, w, hs) in sorted(by_lad.items(), key=lambda kv: kv[1][1]):
        print(f"    {key:>18}: n={n:>5}  pnl={fmt_money(pnl)}  "
              f"win%={w/n*100:>5.1f}  avg_held={hs/n:>12,.0f}bar")

    # ---- 2. 亏损最大单笔 ----
    print("\n  [2] 亏损最大单笔 top-10:")
    losers = sorted(tv, key=lambda t: t["pnl"])[:10]
    print(f"    {'pnl':>12} {'ladder/pol':>16} {'held(bar)':>12} "
          f"{'entry@bar':>22} {'exit@bar':>22} {'reason':>14}")
    for t in losers:
        emove = (t["exit_price"] / t["entry_price"] - 1) * 100
        print(f"    {t['pnl']:>12,.0f} {t['lname']+'/'+t['polarity']:>16} "
              f"{t['held']:>12,} "
              f"{t['entry_price']:>9.2f}@{t['entry_bar']:>11,} "
              f"{t['exit_price']:>9.2f}@{t['exit_bar']:>11,} "
              f"{t['exit_reason']:>14}  ({emove:+.0f}%)")
    top10_loss = sum(t["pnl"] for t in losers)
    print(f"    → top-10 亏损合计 {top10_loss:,.0f} = 总盈亏的 {top10_loss/tot*100 if tot else 0:.0f}%"
          if tot < 0 else f"    → top-10 亏损合计 {top10_loss:,.0f}")

    # ---- 3. 时间分布 ----
    print("\n  [3] 盈亏时间分布（按 exit_bar 10 等分箱）:")
    nb = d["n_bars"]
    nbins = 10
    bins = [[0, 0.0] for _ in range(nbins)]  # n, pnl
    for t in tv:
        b = min(nbins - 1, int(t["exit_bar"] / nb * nbins))
        bins[b][0] += 1
        bins[b][1] += t["pnl"]
    for i, (n, pnl) in enumerate(bins):
        lo = int(i / nbins * nb)
        hi = int((i + 1) / nbins * nb)
        bar = "█" * min(40, int(abs(pnl) / max(1, abs(tot)) * 80))
        sign = "+" if pnl >= 0 else "-"
        print(f"    bin{i} [{lo:>10,}-{hi:>10,}]: n={n:>5} pnl={pnl:>13,.0f} {sign}{bar}")
    # 权益曲线 MDD 定位
    eq = d["equity"]
    peak = -1e18
    mdd = 0.0
    mdd_bar = 0
    peak_bar = 0
    for bar, e in eq:
        if e > peak:
            peak = e
            peak_bar = bar
        dd = (e - peak) / peak if peak else 0
        if dd < mdd:
            mdd = dd
            mdd_bar = bar
    print(f"    权益曲线: 起 {eq[0][1]:,.0f} → 终 {eq[-1][1]:,.0f}; "
          f"MDD={mdd*100:.1f}% 谷底@bar {mdd_bar:,} (峰@{peak_bar:,})")

    # ---- 4. sink-recover 配对 ----
    print("\n  [4] sink(reduce) vs recover 对比:")
    for rs in ("reduce", "recover"):
        sub = [t for t in tv if t["exit_reason"] == rs]
        if not sub:
            continue
        n = len(sub)
        pnl = sum(t["pnl"] for t in sub)
        wins = sum(1 for t in sub if t["pnl"] > 0)
        hs = sum(t["held"] for t in sub) / n
        # 价格朝持仓方向移动幅度（signed%）
        def signed_move(t):
            m = (t["exit_price"] / t["entry_price"] - 1) * 100
            return m if t["polarity"] == "long" else -m
        winners = [t for t in sub if t["pnl"] > 0]
        loss = [t for t in sub if t["pnl"] <= 0]
        wmove = sum(signed_move(t) for t in winners) / len(winners) if winners else 0
        lmove = sum(signed_move(t) for t in loss) / len(loss) if loss else 0
        whold = sum(t["held"] for t in winners) / len(winners) if winners else 0
        lhold = sum(t["held"] for t in loss) / len(loss) if loss else 0
        print(f"    {rs:>8}: n={n:>5} pnl={fmt_money(pnl)} win%={wins/n*100:>5.1f} "
              f"avg_held={hs:>11,.0f}bar")
        print(f"             赢家({len(winners):>4}): 价格顺势{wmove:+6.2f}% 持有{whold:>11,.0f}bar | "
              f"输家({len(loss):>4}): 价格顺势{lmove:+6.2f}% 持有{lhold:>11,.0f}bar")
    return d, tv, tot


def main():
    summary = []
    for sym in FOCUS:
        d, tv, tot = analyze_symbol(sym)
        long_pnl = sum(t["pnl"] for t in tv if t["polarity"] == "long")
        short_pnl = sum(t["pnl"] for t in tv if t["polarity"] == "short")
        summary.append((sym, d["strat_pct"], d["bh_pct"], tot, long_pnl, short_pnl,
                        d["liquidations"]))

    section_header("【跨标的对比】方向盈亏 = 诊断核心")
    print(f"  {'sym':>5} {'strat%':>9} {'BH%':>9} {'总pnl':>13} "
          f"{'long_pnl':>13} {'short_pnl':>13} {'short占比':>9} {'liq':>4}")
    for sym, sp, bh, tot, lp, shp, liq in summary:
        frac = shp / tot * 100 if tot else 0
        print(f"  {sym:>5} {sp:>8.1f} {bh:>8.1f} {tot:>13,.0f} "
              f"{lp:>13,.0f} {shp:>13,.0f} {frac:>8.0f}% {liq:>4}")
    print("\n  解读: short_pnl 在深亏标的(ES/BTC/QQQ)是主要失血源；")
    print("  long_pnl 普遍为正但远不足以补偿。CL 作为震荡标的，short 失血有限。")


if __name__ == "__main__":
    main()
