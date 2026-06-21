#!/usr/bin/env python3
"""T 引擎**翻转版**（commit 512f784a74，4 状态翻转机）空头逐笔诊断（BTC vs CL）。

数据版本（已核实）：analysis/data_cache/t_engine_{SYM}_structural_trades.json
由 HEAD 512f784a74（**4 状态翻转版**）的 `BT_DUMP_TRADES=1 cargo test recursive_t::t_engine_run`
重新生成，schema 含**第 12 字段 `origin`**（腿出生途径，引擎严格标注）。

翻转版语义（git show 512f784a74:rust/src/recursive_t/t_engine.rs 核实）：
  - 同级别/更高级别反向 BSP → **整仓翻转**（FullLong↔FullShort，M=N，永远在市场）= 途径 a
  - 次级别反向 BSP（Full 态）→ sink 减 1/3 下放做短差 = 途径 b（与 5 状态版相同）
  - 翻转检查在 drive() 中**优先且 return** ⟹ 同级别反向 BSP 抢先触发翻转，sink 几乎到不了。

出生途径（origin，引擎打标，与盈亏极性正交）：
  - "flip"  = flip_core 整仓翻转建的反向核心   → **途径 a 核心翻空**（全仓）
  - "sink"  = sink 减 1/3 下放次级别建的子腿   → **途径 b 次级别短差**（1/3 仓）
  - "entry" = try_enter 全资金建核心（入场/强平后重入）→ 入场建空（核心，非翻转）

exit_reason（如何平仓，**不**等于出生途径）：flip / reduce / recover / liq_short / liq_long / eod。
  关键：翻转版里 exit_reason 不能区分途径（flip 既平翻空核心又级联平 sink 子腿），故用 origin。

5 状态版对比数据：analysis/data_cache/_5state_backup/（覆盖前备份，commit a81c6e384c）。

trade schema（12 字段）：
  [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
   weight_at_entry, deferred_bars, partial, exit_reason, polarity, origin]
"""
import json
import sys
from collections import defaultdict

DATA = "analysis/data_cache"
BACKUP = "analysis/data_cache/_5state_backup"
PRICE_FILE = {"BTC": "btc_1m_full.json", "CL": "cl_1m_databento_10y.json"}

# 12 字段 schema 索引
LAD, EB, EP, XB, XP, SH, W, DEF, PART, REASON, POL, ORIGIN = range(12)


def load_clean_closes(sym):
    """复现 Rust load_clean_ohlc 两遍清洗，返回与 trade bar 同索引的 close 序列。"""
    with open(f"{DATA}/{PRICE_FILE[sym]}") as f:
        d = json.load(f)
    o, h, l, c = d["opens"], d["highs"], d["lows"], d["closes"]
    del d
    oo, hh, ll, cc = [], [], [], []
    for i in range(len(c)):
        oi, hi, li, ci = o[i], h[i], l[i], c[i]
        if oi is None or hi is None or li is None or ci is None:
            continue
        if oi <= 0 or hi <= 0 or li <= 0 or ci <= 0:
            continue
        oo.append(oi); hh.append(hi); ll.append(li); cc.append(ci)
    del o, h, l, c
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


def fwd_pct(closes, bar, w):
    if bar < 0 or bar + w >= len(closes):
        return None
    base = closes[bar]
    if base <= 0:
        return None
    return (closes[bar + w] - base) / base * 100.0


def _med(xs):
    if not xs:
        return 0
    s = sorted(xs)
    return s[len(s) // 2]


def find_descent_window(closes, peak_lo, trough_hi):
    """找首次站上 peak_lo 之后、跌破 trough_hi 之前的最高点(peak)及其后最低点(trough)。
    BTC: peak_lo=60000,trough_hi=20000 → 2021-11 ~69k 峰 → 2022-11 ~15.5k 谷。"""
    n = len(closes)
    above = [i for i in range(n) if closes[i] >= peak_lo]
    if not above:
        return None
    start = above[0]
    end = n
    for j in range(start, n):
        if closes[j] < trough_hi:
            end = j
            break
    hi_end = end if end > start else n
    peak_bar = max(range(start, hi_end), key=lambda x: closes[x])
    peak_px = closes[peak_bar]
    lo_end = end if end > peak_bar else n
    trough_bar = min(range(peak_bar, lo_end), key=lambda x: closes[x])
    trough_px = closes[trough_bar]
    return peak_bar, peak_px, trough_bar, trough_px


def short_breakdown(shorts):
    """按 origin 归类空头（途径 a/b/入场）的笔数与 pnl。"""
    by_origin = defaultdict(lambda: [0, 0.0, 0, 0])  # n, pnl, n_win, n_loss
    for t in shorts:
        p = pnl_of(t)
        r = by_origin[t[ORIGIN]]
        r[0] += 1
        r[1] += p
        if p > 0:
            r[2] += 1
        elif p < 0:
            r[3] += 1
    return by_origin


def analyze(sym):
    with open(f"{DATA}/t_engine_{sym}_structural_trades.json") as f:
        data = json.load(f)
    trades = data["trades"]
    nfield = len(trades[0]) if trades else 12
    if nfield != 12:
        print(f"[{sym}] ✗ 期望 12 字段（翻转版含 origin），实得 {nfield} ⟹ 数据非翻转版，中止。")
        return None
    closes = load_clean_closes(sym)

    out = [f"\n{'='*80}\n{sym}  翻转版（final_nav={data['final_nav']:,.1f}, "
           f"strat={(data['final_nav']/100000-1)*100:+.1f}%, n_bars={data['n_bars']:,}, "
           f"clean_closes={len(closes):,}）\n{'='*80}"]

    longs = [t for t in trades if t[POL] == "long"]
    shorts = [t for t in trades if t[POL] == "short"]
    long_pnl = sum(pnl_of(t) for t in longs)
    short_pnl = sum(pnl_of(t) for t in shorts)

    # 对齐校验（exit 端应 100% = closes[exit_bar]）
    x_mis = sum(1 for t in trades if 0 <= t[XB] < len(closes)
                and abs(closes[t[XB]] - t[XP]) / max(t[XP], 1e-9) > 1e-3)
    out.append(f"[对齐校验] EXIT 端不符 {x_mis}/{len(trades)} "
               f"({'✓ bar=raw 索引，趋势判断有效' if x_mis == 0 else '✗ 索引错位'})")

    # ── 全局极性分布 ──
    out.append(f"\n总 trades={len(trades)}  多头 {len(longs)} 笔 pnl={long_pnl:,.1f}  | "
               f"空头 {len(shorts)} 笔 pnl={short_pnl:,.1f}")
    rc = defaultdict(int)
    oc = defaultdict(int)
    for t in trades:
        rc[t[REASON]] += 1
        oc[t[ORIGIN]] += 1
    out.append(f"  exit_reason 分布: {dict(rc)}")
    out.append(f"  origin 分布:      {dict(oc)}")

    # ── (1) 两途径空头区分（origin）──
    out.append("\n── (1) 空头按出生途径区分（origin；途径 a=flip 核心翻空 / 途径 b=sink 次级别短差）──")
    bo = short_breakdown(shorts)
    label = {"flip": "途径a 核心翻空(全仓)", "sink": "途径b 次级别短差(1/3仓)", "entry": "入场建空(核心)"}
    for org in ("flip", "sink", "entry"):
        n, p, nw, nl = bo.get(org, [0, 0.0, 0, 0])
        out.append(f"  {label.get(org, org):<22} {org:<6} n={n:>5} pnl={p:>15,.1f}  "
                   f"赚{nw}/亏{nl}  均{(p/n if n else 0):>11,.1f}")
    # 其他 origin（防御）
    for org in bo:
        if org not in ("flip", "sink", "entry"):
            n, p, nw, nl = bo[org]
            out.append(f"  {'(其他)':<22} {org:<6} n={n:>5} pnl={p:>15,.1f}")

    flip_short = [t for t in shorts if t[ORIGIN] == "flip"]
    sink_short = [t for t in shorts if t[ORIGIN] == "sink"]
    entry_short = [t for t in shorts if t[ORIGIN] == "entry"]
    flip_short_pnl = sum(pnl_of(t) for t in flip_short)
    sink_short_pnl = sum(pnl_of(t) for t in sink_short)

    # ── (2) 翻空（途径 a, origin=flip）逐笔 ──
    out.append("\n── (2) 翻空（途径 a, origin=flip）逐笔画像 ──")
    if flip_short:
        holds = [t[XB] - t[EB] for t in flip_short]
        wins = [t for t in flip_short if pnl_of(t) > 0]
        out.append(f"  翻空笔数={len(flip_short)}  总 pnl={flip_short_pnl:,.1f}  "
                   f"赚 {len(wins)}/{len(flip_short)} ({len(wins)*100//len(flip_short)}%)  "
                   f"中位持有={_med(holds)} bar")
        # 翻空后真的转跌吗？（entry_bar 处未来窗口价格方向；short 赚 ⟺ 价格跌）
        for w, lab in ((1440, "翻空后1天"), (7200, "翻空后5天")):
            dn = up = 0
            for t in flip_short:
                fp = fwd_pct(closes, t[EB], w)
                if fp is None:
                    continue
                if fp < 0:
                    dn += 1
                else:
                    up += 1
            tot = dn + up
            if tot:
                out.append(f"  [{lab}] 翻空后价格下跌(空头对) {dn} ({dn*100//tot}%)  | "
                           f"继续涨(空头错) {up} ({up*100//tot}%)")
        # 最赚/最亏 5 笔
        sp = sorted(((pnl_of(t), t) for t in flip_short), key=lambda x: x[0])
        out.append("  最亏 5 笔翻空 [L | 翻空价→平仓价 | hold | shares | pnl | Δ%]:")
        for pnl, t in sp[:5]:
            d = (t[XP] / t[EP] - 1.0) * 100.0
            out.append(f"    L{t[LAD]} {t[EP]:>11.1f}→{t[XP]:>11.1f} hold={t[XB]-t[EB]:>8d} "
                       f"sh={t[SH]:>10.3f} pnl={pnl:>14,.1f} Δ={d:+.2f}%")
        out.append("  最赚 5 笔翻空:")
        for pnl, t in sp[-5:][::-1]:
            d = (t[XP] / t[EP] - 1.0) * 100.0
            out.append(f"    L{t[LAD]} {t[EP]:>11.1f}→{t[XP]:>11.1f} hold={t[XB]-t[EB]:>8d} "
                       f"sh={t[SH]:>10.3f} pnl={pnl:>14,.1f} Δ={d:+.2f}%")
    else:
        out.append("  无翻空空头（origin=flip & short 为空）。")

    # ── (2b) BTC 69k→15k 下跌段做空归因 ──
    if sym == "BTC":
        win = find_descent_window(closes, 60000, 20000)
        if win:
            pb, ppx, tb, tpx = win
            out.append(f"\n── (2b) BTC 69k→15k 大跌段做空检验 ──")
            out.append(f"  检测下跌段: peak bar={pb:,} (${ppx:,.0f}) → trough bar={tb:,} (${tpx:,.0f})  "
                       f"跌幅 {(tpx/ppx-1)*100:.1f}%  跨 {tb-pb:,} bar")
            # 所有空头 entry_bar 落在下跌段内（翻空+sink+入场），按 origin 分
            in_desc = [t for t in shorts if pb <= t[EB] <= tb]
            for org in ("flip", "sink", "entry"):
                sub = [t for t in in_desc if t[ORIGIN] == org]
                if sub:
                    p = sum(pnl_of(t) for t in sub)
                    w_ = sum(1 for t in sub if pnl_of(t) > 0)
                    out.append(f"  段内建空 {label.get(org, org):<22} n={len(sub):>4} "
                               f"pnl={p:>14,.1f}  赚{w_}/{len(sub)}")
            allp = sum(pnl_of(t) for t in in_desc)
            out.append(f"  ⇒ 下跌段内所有建空 n={len(in_desc)} 总 pnl={allp:,.1f} "
                       f"({'赚' if allp > 0 else '亏'})")
            # 段内是否真有空头在场（exposure 时间覆盖）
            cover = sum(min(t[XB], tb) - max(t[EB], pb)
                        for t in shorts if t[EB] <= tb and t[XB] >= pb and min(t[XB], tb) > max(t[EB], pb))
            out.append(f"  段内空头持仓 bar·笔覆盖={cover:,}（段长 {tb-pb:,}）"
                       f"⇒ {'有空头在场' if cover > 0 else '全程无空头在场'}")

    print("\n".join(out))
    return {
        "sym": sym, "n_short": len(shorts), "short_pnl": short_pnl,
        "n_long": len(longs), "long_pnl": long_pnl,
        "flip_short_n": len(flip_short), "flip_short_pnl": flip_short_pnl,
        "sink_short_n": len(sink_short), "sink_short_pnl": sink_short_pnl,
        "entry_short_n": len(entry_short),
    }


def analyze_5state(sym):
    """5 状态版（备份）空头归因：reduce/recover 短差 vs core_exit 核心。无 origin，用 reason+shares。"""
    path = f"{BACKUP}/t_engine_{sym}_structural_trades.json"
    try:
        with open(path) as f:
            data = json.load(f)
    except FileNotFoundError:
        return None
    trades = data["trades"]
    shorts = [t for t in trades if t[POL] == "short"]  # 5state 也是 [..,reason,polarity]，POL=10
    longs = [t for t in trades if t[POL] == "long"]
    short_pnl = sum(pnl_of(t) for t in shorts)
    long_pnl = sum(pnl_of(t) for t in longs)
    by_reason = defaultdict(lambda: [0, 0.0])
    for t in shorts:
        by_reason[t[REASON]][0] += 1
        by_reason[t[REASON]][1] += pnl_of(t)
    # 短差 = reduce+recover（sink 子腿闭环）；core_exit = 核心随清仓平
    sink_pnl = sum(by_reason[r][1] for r in ("reduce", "recover") if r in by_reason)
    sink_n = sum(by_reason[r][0] for r in ("reduce", "recover") if r in by_reason)
    return {
        "sym": sym, "final_nav": data["final_nav"], "n_short": len(shorts),
        "short_pnl": short_pnl, "long_pnl": long_pnl,
        "by_reason": {k: v for k, v in by_reason.items()},
        "sink_short_pnl": sink_pnl, "sink_short_n": sink_n,
    }


if __name__ == "__main__":
    syms = sys.argv[1:] or ["BTC", "CL"]
    flip_res = {}
    for s in syms:
        r = analyze(s)
        if r:
            flip_res[s] = r

    # ── (3) 5 状态版对比 ──
    print(f"\n{'='*80}\n(3) 翻转版 vs 5 状态版对比（5 状态版来自 _5state_backup，commit a81c6e384c）\n{'='*80}")
    for s in syms:
        f5 = analyze_5state(s)
        fv = flip_res.get(s)
        if not f5 or not fv:
            print(f"  {s}: 缺数据（5state={'有' if f5 else '无'} / flip={'有' if fv else '无'}）")
            continue
        print(f"\n  ── {s} ──")
        print(f"  5状态版: final_nav={f5['final_nav']:,.0f} ({(f5['final_nav']/100000-1)*100:+.1f}%)  "
              f"空头 {f5['n_short']}笔 pnl={f5['short_pnl']:,.1f}  多头 pnl={f5['long_pnl']:,.1f}")
        print(f"           空头 reason 分布: "
              + "  ".join(f"{k}:{v[0]}笔/{v[1]:,.0f}" for k, v in sorted(f5['by_reason'].items(), key=lambda x: x[1][1])))
        print(f"           短差(reduce+recover) {f5['sink_short_n']}笔 pnl={f5['sink_short_pnl']:,.1f}")
        print(f"  翻转版:  final_nav={fv['short_pnl']+fv['long_pnl']+100000:,.0f}  "
              f"空头 {fv['n_short']}笔 pnl={fv['short_pnl']:,.1f}  多头 pnl={fv['long_pnl']:,.1f}")
        print(f"           途径a 翻空(flip) {fv['flip_short_n']}笔 pnl={fv['flip_short_pnl']:,.1f}  | "
              f"途径b 短差(sink) {fv['sink_short_n']}笔 pnl={fv['sink_short_pnl']:,.1f}  | "
              f"入场建空 {fv['entry_short_n']}笔")
        print(f"  ⇒ 空头整体: 5状态版 {f5['short_pnl']:,.0f} vs 翻转版 {fv['short_pnl']:,.0f} "
              f"({'翻转版更好' if fv['short_pnl'] > f5['short_pnl'] else '翻转版更差'} "
              f"Δ={fv['short_pnl']-f5['short_pnl']:,.0f})")
