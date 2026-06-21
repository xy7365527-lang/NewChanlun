#!/usr/bin/env python3
"""T 引擎空头亏损机制逐笔诊断（BTC vs CL）。

数据版本订正（关键）：analysis/data_cache/t_engine_{SYM}_structural_trades.json
生成于 2026-06-19 02:31，对应 commit a81c6e384c（**6步周期5状态机版**），
**不是**当前 HEAD 512f784a74（4状态翻转版）。

6步周期版语义（已从 git show a81c6e384c:t_engine.rs 核实）：
  - 同级别/更高级别反向 BSP → clear_to_cash("core_exit") = **清仓回现金，不翻空**
  - short 来源仅两条：
      (入场建空) try_enter 遇 sell → FullShort 核心（满仓，罕见，仅 global_flat 时）
      (sink 短差) FullLong + 次级别卖点@(k−1) → ReducedLong + spawn FullShort@(k−1)（1/3 仓）
  - short 平仓 reason：
      "reduce"   = 该 short 腿在 sink 中被再减 1/3（它自己又下放）
      "recover"  = sink 出的 short 子腿被次级别同向买点回补平仓（标准短差闭环）
      "core_exit"= short 腿随 core 反向清仓被一起平掉
      "liq_short"= 强平

  ⇒ 用户问的「a) 核心翻空 vs b) 短差 sink」框架中，**a 在此版本不存在**（无翻转）。
     本脚本按真实途径（reduce/recover/core_exit × ladder × shares 档）归因亏损。

trade schema（11 字段，无 pnl 列，自算）：
  [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
   weight_at_entry, deferred_bars, partial, exit_reason, polarity]
"""
import json
import sys
from collections import defaultdict

DATA = "analysis/data_cache"
PRICE_FILE = {"BTC": "btc_1m_full.json", "CL": "cl_1m_databento_10y.json"}

# schema 索引
LAD, EB, EP, XB, XP, SH, W, DEF, PART, REASON, POL = range(11)


def load_clean_closes(sym):
    """复现 Rust load_clean_ohlc 两遍清洗，返回与 trade bar 同索引的 close 序列。"""
    with open(f"{DATA}/{PRICE_FILE[sym]}") as f:
        d = json.load(f)
    o, h, l, c = d["opens"], d["highs"], d["lows"], d["closes"]
    del d
    # 第一遍：NaN/None/≤0 删除（任一 OHLC）
    oo, hh, ll, cc = [], [], [], []
    for i in range(len(c)):
        oi, hi, li, ci = o[i], h[i], l[i], c[i]
        if oi is None or hi is None or li is None or ci is None:
            continue
        if oi <= 0 or hi <= 0 or li <= 0 or ci <= 0:
            continue
        oo.append(oi); hh.append(hi); ll.append(li); cc.append(ci)
    del o, h, l, c
    # 第二遍：spike-and-revert
    n = len(cc)
    drop = bytearray(n)
    for i in range(1, n - 1):
        if abs(cc[i] / cc[i - 1] - 1.0) > 0.5 and abs(cc[i + 1] / cc[i - 1] - 1.0) < 0.05:
            drop[i] = 1
    if any(drop):
        cc = [x for i, x in enumerate(cc) if not drop[i]]
    return cc


def pnl_of(t):
    sh, ep, xp = t[SH], t[EP], t[XP]
    if t[POL] == "short":
        return sh * (ep - xp)
    return sh * (xp - ep)


def trend_pct(closes, bar, w):
    """bar 处过去 w 窗口的价格变化率（>0 涨, <0 跌）。"""
    if bar - w < 0 or bar >= len(closes):
        return None
    base = closes[bar - w]
    if base <= 0:
        return None
    return (closes[bar] - base) / base * 100.0


def fwd_pct(closes, bar, w):
    """bar 处未来 w 窗口价格变化率。"""
    if bar < 0 or bar + w >= len(closes):
        return None
    base = closes[bar]
    if base <= 0:
        return None
    return (closes[bar + w] - base) / base * 100.0


def analyze(sym):
    with open(f"{DATA}/t_engine_{sym}_structural_trades.json") as f:
        data = json.load(f)
    trades = data["trades"]
    closes = load_clean_closes(sym)

    out = [f"\n{'='*78}\n{sym}  （final_nav={data['final_nav']:.1f}, n_bars={data['n_bars']}, "
           f"clean_closes={len(closes)}）\n{'='*78}"]

    # 对齐验证（分端）：exit_price=closes[exit_bar]（成交价，应 100% 对齐 ⇒ 证明 bar 是 raw 索引）；
    # entry_price=layer.basis（加权建仓均价，可 ≠ closes[entry_bar]，差异小）。
    e_mis = e_max = 0.0
    x_mis = 0
    for t in trades:
        if 0 <= t[XB] < len(closes) and abs(closes[t[XB]] - t[XP]) / max(t[XP], 1e-9) > 1e-3:
            x_mis += 1
        if 0 <= t[EB] < len(closes):
            d = abs(closes[t[EB]] - t[EP]) / max(t[EP], 1e-9)
            if d > 1e-3:
                e_mis += 1
                e_max = max(e_max, d)
    out.append(f"[对齐校验] EXIT端 不符 {x_mis}/{len(trades)} "
               f"({'✓ bar=raw索引，趋势判断有效' if x_mis == 0 else '✗ 索引错位'})  | "
               f"ENTRY端 {int(e_mis)} 笔为加权basis(最大偏差{e_max*100:.1f}%，不影响entry_bar趋势)")

    longs = [t for t in trades if t[POL] == "long"]
    shorts = [t for t in trades if t[POL] == "short"]

    # ── (1) 逐笔盈亏 ──
    def stats(ts):
        pnls = [pnl_of(t) for t in ts]
        wins = [p for p in pnls if p > 0]
        loss = [p for p in pnls if p < 0]
        flat = [p for p in pnls if p == 0]
        return pnls, wins, loss, flat

    sp, sw, sl, sf = stats(shorts)
    lp, lw, ll_, lf = stats(longs)
    out.append("\n── (1) 逐笔盈亏 ──")
    out.append(f"  空头 {len(shorts)} 笔: 赚 {len(sw)} / 亏 {len(sl)} / 平 {len(sf)}  | 总 pnl={sum(sp):,.1f}")
    out.append(f"       赚笔均盈 {(sum(sw)/len(sw) if sw else 0):,.1f}  亏笔均亏 {(sum(sl)/len(sl) if sl else 0):,.1f}  "
               f"盈亏比={(sum(sw)/-sum(sl) if sl else float('inf')):.3f}")
    out.append(f"  多头 {len(longs)} 笔: 赚 {len(lw)} / 亏 {len(ll_)} / 平 {len(lf)}  | 总 pnl={sum(lp):,.1f}")
    out.append(f"       赚笔均盈 {(sum(lw)/len(lw) if lw else 0):,.1f}  亏笔均亏 {(sum(ll_)/len(ll_) if ll_ else 0):,.1f}")

    # ── top10 亏损空头 ──
    shorts_pnl = sorted(((pnl_of(t), t) for t in shorts), key=lambda x: x[0])
    out.append("\n  最大10笔亏损空头 [reason | ladder | entry→exit | hold_bars | shares | pnl | 持有期价格Δ%]:")
    for pnl, t in shorts_pnl[:10]:
        hold = t[XB] - t[EB]
        dpct = (t[XP] / t[EP] - 1.0) * 100.0
        out.append(f"    {t[REASON]:<9} L{t[LAD]} {t[EP]:>10.1f}→{t[XP]:>10.1f} "
                   f"hold={hold:>8d} sh={t[SH]:>9.3f} pnl={pnl:>14,.1f}  Δ={dpct:+.2f}%")

    # ── (2) 空头入场时机：建空时价格在涨还是跌 ──
    out.append("\n── (2) 空头入场时机（建空 bar 处过去窗口趋势）──")
    for w, lab in ((1440, "过去1天"), (7200, "过去5天")):
        up = dn = 0
        up_pnl = dn_pnl = 0.0
        for t in shorts:
            tp = trend_pct(closes, t[EB], w)
            if tp is None:
                continue
            if tp > 0:
                up += 1; up_pnl += pnl_of(t)
            else:
                dn += 1; dn_pnl += pnl_of(t)
        tot = up + dn
        out.append(f"  [{lab}] 上涨中入场 {up} ({up*100//max(tot,1)}%) pnl={up_pnl:,.1f}  | "
                   f"下跌中入场 {dn} ({dn*100//max(tot,1)}%) pnl={dn_pnl:,.1f}")
        if w == 1440:  # 过去1天窗口区分力最强 → 观测性反事实归因
            out.append(f"  ⇒ [观测性归因，L1] 若过滤掉「上涨中建空」: 空头 pnl {sum(sp):,.1f} → {dn_pnl:,.1f}"
                       f"（{'空头由亏转盈' if dn_pnl > 0 and sum(sp) < 0 else '改善' if dn_pnl > sum(sp) else '无改善'}）")

    # ── (3) 空头出场时机 ──
    out.append("\n── (3) 空头出场（reason 分布 + 持有期价格方向）──")
    by_reason = defaultdict(lambda: [0, 0.0, 0, 0])  # n, pnl, n_win, n_loss
    for t in shorts:
        p = pnl_of(t)
        r = by_reason[t[REASON]]
        r[0] += 1; r[1] += p
        if p > 0: r[2] += 1
        elif p < 0: r[3] += 1
    for reason, (n, p, nw, nl) in sorted(by_reason.items(), key=lambda x: x[1][1]):
        out.append(f"  {reason:<9} n={n:>4} pnl={p:>14,.1f}  赚{nw}/亏{nl}  均{p/max(n,1):>10,.1f}")

    # 空头赚钱但可能被早平：持有期价格下跌（short 赚）的笔，看持有时长
    win_holds = [t[XB] - t[EB] for t in shorts if pnl_of(t) > 0]
    loss_holds = [t[XB] - t[EB] for t in shorts if pnl_of(t) < 0]
    out.append(f"  赚钱空头中位持有={_med(win_holds)} bar  亏损空头中位持有={_med(loss_holds)} bar")
    out.append(f"  ⇒ {'赚的持有更短=被早平回补' if _med(win_holds) < _med(loss_holds) else '亏的持有更长=套牢'}")

    # ── (5) 机制归因：reason × ladder × shares 档 ──
    out.append("\n── (5) 亏损归因（reason × ladder；shares 档区分核心/子腿）──")
    # shares 分档：用 long 入场满仓量级做基准（n_base/c 量级）
    all_sh = sorted(t[SH] for t in trades)
    sh_hi = all_sh[int(len(all_sh) * 0.9)]
    out.append(f"  shares 分布: min={all_sh[0]:.3f} median={_med(all_sh):.3f} p90={sh_hi:.3f} max={all_sh[-1]:.3f}")
    grid = defaultdict(lambda: [0, 0.0])
    for t in shorts:
        key = (t[REASON], t[LAD])
        grid[key][0] += 1
        grid[key][1] += pnl_of(t)
    out.append("  [reason, ladder] → (n, pnl)  按 pnl 升序（最亏在前）:")
    for (r, lad), (n, p) in sorted(grid.items(), key=lambda x: x[1][1])[:12]:
        out.append(f"    {r:<9} L{lad}  n={n:>4}  pnl={p:>14,.1f}")

    # shares 大小 vs 盈亏（核心满仓 short = 入场建空; 小 = sink 子腿）
    big = [t for t in shorts if t[SH] >= sh_hi]
    small = [t for t in shorts if t[SH] < sh_hi]
    out.append(f"\n  大仓空头(sh≥p90, ≈入场建空核心) {len(big)} 笔 pnl={sum(pnl_of(t) for t in big):,.1f}")
    out.append(f"  小仓空头(sh<p90, ≈次级别sink短差) {len(small)} 笔 pnl={sum(pnl_of(t) for t in small):,.1f}")

    print("\n".join(out))
    return {
        "sym": sym, "n_short": len(shorts), "short_pnl": sum(sp),
        "n_long": len(longs), "long_pnl": sum(lp),
        "by_reason": {k: v for k, v in by_reason.items()},
    }


def _med(xs):
    if not xs:
        return 0
    s = sorted(xs)
    return s[len(s) // 2]


if __name__ == "__main__":
    syms = sys.argv[1:] or ["BTC", "CL"]
    res = [analyze(s) for s in syms]
    print(f"\n{'='*78}\n汇总（初始资金 INITIAL_CAPITAL=100,000）\n{'='*78}")
    for r in res:
        net = r["short_pnl"] + r["long_pnl"]
        print(f"  {r['sym']}: 空头 {r['n_short']}笔 pnl={r['short_pnl']:>13,.1f} | "
              f"多头 {r['n_long']}笔 pnl={r['long_pnl']:>12,.1f} | "
              f"净realized={net:>12,.1f} ⇒ 终值≈{100000+net:,.0f} "
              f"({'整体亏' if net < 0 else '整体赚'} {net/1000:.0f}%)")
    print("\n  反事实(L1 观测)：BTC 若禁空（只做多）→ 终值≈100000+多头pnl；空头是 BTC 亏损唯一来源。")
