"""T 赋格引擎「已知大下跌段做空」逐笔追踪诊断。

═══════════════ 调查问题 ═══════════════

BTC 69000→15000（2021-11~2022-11，-77.6%）/ ES 4800→3500（2022 年）这类**已知大下跌段**，
若顺势做空应赚数百%。但 T 赋格引擎空头总盈利很小甚至为负（BTC structural strat -96%）。
为什么？churn？过早平仓？方向判断滞后？还是根本没在高位翻空？

═══════════════ 数据源辨认（严格）═══════════════

  消费 T 赋格引擎逐笔明细：trading_system/data_cache/t_fugue_<SYM>_<MODE>.json（多空双向）。
  ⚠ 不消费 analysis/data_cache/t_backtest_*.json —— 那是 backtest.rs long-only 简单版（n_short≡0）。

  trade11 schema:
    [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
     weight_at_entry, deferred_bars, partial, exit_reason, polarity]
    已实现现金: long = sh*(xp-ep)；short = sh*(ep-xp)
    对 NAV 贡献%: pnl_cash / INITIAL_CAPITAL(=100000) * 100（与 strat_pct 同口径，可加总）

═══════════════ 坐标系对齐（关键，否则日期错位=声明膨胀）═══════════════

  entry_bar / exit_bar = load_ohlc 清洗后索引（删 nan/≤0/spike-revert bar）。
  本脚本复刻 *完全相同* 的清洗逻辑并同步携带 dates，再 assert 清洗后长度 == JSON n_bars，
  保证 dates[entry_bar] 是真实日期（BTC 实测无删行，对齐成立；ES 同样 assert）。

═══════════════ 下跌段 = ZigZag-independent 真实日期锚定 ═══════════════

  顶/底用 high/low 极值在指定日期窗口内定位（不看任何交易，避免循环论证）。
  下跌段 = [top_bar, bot_bar]。交易按 entry_bar 归段（开仓时点）。
  同时报「exit 也在段内闭合」子集 → 区分「开在跌段但扛过底部」的死扛仓。

认识论等级：L3（真实数据逐笔归因，可否证）。段定位/churn 度量本身 L0（无数据假设）。

用法（仓库根目录）:
    PYTHONPATH=src .venv/bin/python analysis/t_fugue_downtrend_short_trace.py
    PYTHONPATH=src .venv/bin/python analysis/t_fugue_downtrend_short_trace.py --symbols BTC
"""

from __future__ import annotations

import argparse
import bisect
import json
import math
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

DATA_DIR = REPO_ROOT / "analysis" / "data_cache"
FUGUE_DIR = REPO_ROOT / "trading_system" / "data_cache"
REPORT = REPO_ROOT / "analysis" / "reports" / "t_fugue_downtrend_short_trace.md"
INITIAL_CAPITAL = 100_000.0
MODES = ("structural", "and", "or")

# 标的 → 原始数据文件（与 fugue_v2_full_backtest.SYMBOL_FILES 同源）。
RAW_FILES = {
    "BTC": DATA_DIR / "btc_1m_full.json",
    "ES": DATA_DIR / "es_1m_databento_10y.json",
}

# 顶/底日期窗口（YYYY-MM 前缀）——独立于交易，纯价格极值锚定。
SEGMENTS = {
    # BTC: ATH 2021-11-10 ~69000 → 熊底 2022-11 ~15500
    "BTC": dict(top_win=("2021-09", "2021-12"), bot_win=("2022-09", "2022-12"),
                label="69000 → 15500（2021-11 ~ 2022-11）"),
    # ES: 2022-01-04 顶 ~4800 → 2022-10-13 底 ~3500
    "ES": dict(top_win=("2021-12", "2022-02"), bot_win=("2022-09", "2022-11"),
               label="4800 → 3500（2022-01 ~ 2022-10）"),
}


# ════════════════════════ 数据加载（复刻 load_ohlc 清洗 + 携带 dates）════════════════════════


def load_ohlc_with_dates(path: Path):
    """复刻 fugue_v2_full_backtest.load_ohlc 的清洗逻辑，但同步保留完整 dates。

    返回 (opens, highs, lows, closes, dates)。清洗顺序与真相源逐字一致：
      1. 删任一 OHLC 为 nan 或 ≤0 的 bar；
      2. spike-and-revert（close 跳变 >50% 且后 bar 回到前 bar ±5% 内）。
    """
    raw = json.loads(path.read_text())
    if "bars" in raw:
        bars = raw["bars"]
        o_in = [b["open"] for b in bars]
        h_in = [b["high"] for b in bars]
        l_in = [b["low"] for b in bars]
        c_in = [b["close"] for b in bars]
        d_in = [b["ts"] for b in bars]
    else:
        o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
        d_in = raw.get("dates")
    if d_in is None:
        raise ValueError(f"{path.name} 无 dates，无法做真实日期锚定")

    o, h, l, c, d = [], [], [], [], []
    for i in range(len(c_in)):
        a, b, e, f = float(o_in[i]), float(h_in[i]), float(l_in[i]), float(c_in[i])
        if math.isnan(a) or math.isnan(b) or math.isnan(e) or math.isnan(f):
            continue
        if a <= 0 or b <= 0 or e <= 0 or f <= 0:
            continue
        o.append(a); h.append(b); l.append(e); c.append(f); d.append(str(d_in[i]))
    drop = {i for i in range(1, len(c) - 1)
            if abs(c[i] / c[i - 1] - 1) > 0.5 and abs(c[i + 1] / c[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(c)) if i not in drop]
        o = [o[i] for i in keep]; h = [h[i] for i in keep]
        l = [l[i] for i in keep]; c = [c[i] for i in keep]; d = [d[i] for i in keep]
    return o, h, l, c, d


def locate_extreme(dates, arr, win, want_max):
    """指定 YYYY-MM 窗口内 arr 的极值 bar（want_max=True 取 max=顶, False 取 min=底）。"""
    lo, hi = win
    best_i, best_v = None, None
    for i in range(len(dates)):
        ym = dates[i][:7]
        if ym < lo or ym > hi:
            continue
        v = arr[i]
        if best_v is None or (v > best_v if want_max else v < best_v):
            best_v, best_i = v, i
    return best_i, best_v


# ════════════════════════ 段内逐笔分析 ════════════════════════


@dataclass
class ShortStat:
    n: int = 0
    pnl_cash: float = 0.0
    wins: int = 0
    held_bars: int = 0
    entry_px_sum: float = 0.0
    exit_px_sum: float = 0.0
    n_closed_in_seg: int = 0          # exit_bar 也在段内
    n_held_past_bottom: int = 0       # exit_bar > bot_bar（扛过底部）
    by_reason: dict = field(default_factory=dict)
    by_ladder: dict = field(default_factory=dict)


def trade_pnl(ep, xp, sh, pol):
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


def analyze_segment(sym, mode, top_bar, bot_bar, closes, dates):
    """单 (标的,模式)：entry_bar∈[top_bar,bot_bar] 的逐笔，分 polarity 聚合 + 关键时刻。"""
    f = FUGUE_DIR / f"t_fugue_{sym}_{mode}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    trades = d["trades"]

    short = ShortStat()
    long_ = ShortStat()
    short_entries = []   # (entry_bar, entry_px, ladder) 用于时间分布 + 首次翻空
    short_trade_rows = []  # 明细
    for (lad, eb, ep, xb, xp, sh, _w, _dfr, _part, reason, pol) in trades:
        if not (top_bar <= eb <= bot_bar):
            continue
        pnl = trade_pnl(ep, xp, sh, pol)
        tgt = short if pol == "short" else long_
        tgt.n += 1
        tgt.pnl_cash += pnl
        tgt.wins += pnl > 0
        tgt.held_bars += xb - eb
        tgt.entry_px_sum += ep
        tgt.exit_px_sum += xp
        tgt.by_reason[reason] = tgt.by_reason.get(reason, 0) + 1
        lk = f"L{lad}"
        lr = tgt.by_ladder.setdefault(lk, {"n": 0, "pnl": 0.0})
        lr["n"] += 1; lr["pnl"] += pnl
        if xb <= bot_bar:
            tgt.n_closed_in_seg += 1
        else:
            tgt.n_held_past_bottom += 1
        if pol == "short":
            short_entries.append((eb, ep, lad, xb, xp, pnl, reason))
            short_trade_rows.append((lad, eb, ep, xb, xp, sh, reason, pnl))

    # equity 段变化（直接 NAV 证据）
    eq = d["equity"]
    eq_bars = [b for (b, _n) in eq]
    nav_top = _nav_at(eq, eq_bars, top_bar)
    nav_bot = _nav_at(eq, eq_bars, bot_bar)

    return {
        "symbol": sym, "mode": mode,
        "strat_pct": d["strat_pct"], "bh_pct": d["bh_pct"],
        "short": short, "long": long_,
        "short_entries": short_entries,
        "short_rows": short_trade_rows,
        "nav_top": nav_top, "nav_bot": nav_bot,
        "n_bars": d["n_bars"],
    }


def _nav_at(eq, eq_bars, bar):
    """equity 序列中 bar 处（或之前最近）的 NAV。"""
    k = bisect.bisect_right(eq_bars, bar) - 1
    if k < 0:
        k = 0
    return eq[k][1]


# ════════════════════════ 报告 ════════════════════════


def pct(cash):
    return cash / INITIAL_CAPITAL * 100.0


def fmt_short(s: ShortStat):
    if s.n == 0:
        return "无空头"
    return (f"n={s.n} pnl={pct(s.pnl_cash):+.1f}%NAV win={s.wins/s.n*100:.0f}% "
            f"avg_held={s.held_bars/s.n:,.0f}b avg_entry={s.entry_px_sum/s.n:,.0f} "
            f"avg_exit={s.exit_px_sum/s.n:,.0f} 闭合段内={s.n_closed_in_seg} 扛过底={s.n_held_past_bottom}")


def build_report(symbols):
    out = []
    w = out.append
    w("# T 赋格引擎 · 已知大下跌段做空逐笔追踪诊断\n")
    w("- 数据源：`trading_system/data_cache/t_fugue_<SYM>_<MODE>.json`（多空双向，真实 entry/exit）")
    w("- 坐标系：entry_bar/exit_bar = load_ohlc 清洗后索引；本脚本复刻清洗携带 dates，assert 对齐")
    w("- 顶/底：high/low 极值在日期窗口内定位（独立于交易，无循环论证）")
    w("- pnl 口径：已实现现金 / 初始资本 100,000 × 100 = NAV 贡献%（可加总）")
    w("- 认识论等级：**L3**（真实数据逐笔，可否证）\n")

    summary_rows = []

    for sym in symbols:
        o, h, l, closes, dates = load_ohlc_with_dates(RAW_FILES[sym])
        # 坐标系 assert
        ref = json.loads((FUGUE_DIR / f"t_fugue_{sym}_structural.json").read_text())
        assert len(closes) == ref["n_bars"], (
            f"{sym} 坐标系错位: cleaned {len(closes)} != JSON n_bars {ref['n_bars']}")

        cfg = SEGMENTS[sym]
        top_bar, top_px = locate_extreme(dates, h, cfg["top_win"], want_max=True)
        bot_bar, bot_px = locate_extreme(dates, l, cfg["bot_win"], want_max=False)
        seg_bars = bot_bar - top_bar
        dd = (bot_px / top_px - 1) * 100

        w(f"## {sym} — {cfg['label']}\n")
        w(f"- 顶 bar=**{top_bar:,}** @ {dates[top_bar]} high=**{top_px:,.0f}**")
        w(f"- 底 bar=**{bot_bar:,}** @ {dates[bot_bar]} low=**{bot_px:,.0f}**")
        w(f"- 下跌段 **{seg_bars:,} bar**（≈{seg_bars/1440:.0f} 天） 跌幅 **{dd:+.1f}%**")
        w(f"- 全程 n_bars={len(closes):,}（坐标系 assert PASS）\n")

        results = {}
        for mode in MODES:
            r = analyze_segment(sym, mode, top_bar, bot_bar, closes, dates)
            if r:
                r["seg_bars"] = seg_bars
                r["top_px"] = top_px
                r["bot_px"] = bot_px
                results[mode] = r

        # ── 段内 NAV 变化（直接证据：引擎在下跌段净赚还是净亏）──
        w("### 段内 NAV 变化（直接证据：下跌段引擎账户净盈亏）\n")
        w("| 模式 | 顶 NAV | 底 NAV | 段内Δ | 段内Δ% | 同期 BH(做空者理想) |")
        w("|------|------:|------:|------:|------:|------:|")
        bh_short_ideal = (top_px / bot_px - 1) * 100  # 满仓做空理想收益
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            dnav = r["nav_bot"] - r["nav_top"]
            w(f"| {mode} | {r['nav_top']:,.0f} | {r['nav_bot']:,.0f} | {dnav:+,.0f} "
              f"| {dnav/r['nav_top']*100:+.1f}% | +{bh_short_ideal:.0f}% |")
        w("")

        # ── 全程空头 entry 年度分布（交叉验证：引擎实际在哪些年做空）──
        w("### 全程空头 entry 年度分布（引擎实际在哪些年份做空）\n")
        w(f"下跌段年份 = {cfg['top_win'][0][:4]}~{cfg['bot_win'][1][:4]}。"
          "若该年份空头数为 0 → 引擎在该大跌中根本未翻空（root 方向未涌现向下）。\n")
        w("| 模式 | 全程空头数 | 年度分布(年:笔数) |")
        w("|------|---:|------|")
        for mode in MODES:
            allt = json.loads((FUGUE_DIR / f"t_fugue_{sym}_{mode}.json").read_text())["trades"]
            sh = [t for t in allt if t[10] == "short"]
            yr = Counter(dates[t[1]][:4] for t in sh)
            dist = " ".join(f"{y}:{n}" for y, n in sorted(yr.items()))
            w(f"| {mode} | {len(sh)} | {dist} |")
        w("")

        # ── 段内空头逐笔聚合 ──
        w("### 段内空头交易聚合（按 entry_bar 归段）\n")
        w("| 模式 | 空头明细 |")
        w("|------|------|")
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            w(f"| {mode} | {fmt_short(r['short'])} |")
        w("")

        w("### 段内多头交易聚合（逆势=下跌段做多，应亏）\n")
        w("| 模式 | 多头明细 |")
        w("|------|------|")
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            w(f"| {mode} | {fmt_short(r['long'])} |")
        w("")

        # ── 空头按 ladder 分解 ──
        w("### 段内空头按 ladder 分解（哪一级别空头亏/赚）\n")
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            if r["short"].n == 0:
                w(f"- **{mode}**: 段内无空头")
                continue
            parts = ", ".join(
                f"{lk}: n={v['n']} {pct(v['pnl']):+.1f}%"
                for lk, v in sorted(r["short"].by_ladder.items()))
            w(f"- **{mode}**: {parts}")
        w("")

        # ── 段内空头 exit_reason 分布：短差 vs 核心持有 ──
        w("### 段内空头 exit_reason 分布（短差 reduce/recover vs 核心 core_clear_short）\n")
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            if r["short"].n == 0:
                w(f"- **{mode}**: 段内无空头")
                continue
            br = r["short"].by_reason
            tot = sum(br.values())
            churn = br.get("reduce", 0) + br.get("recover", 0)
            w(f"- **{mode}**: {dict(sorted(br.items()))} → 短差(reduce+recover)="
              f"{churn}/{tot}({churn/tot*100:.0f}%) 核心清(core_clear_short)={br.get('core_clear_short', 0)}")
        w("")

        # ── 空头开仓时间分布（10 桶）：诊断翻空滞后 vs 高位翻空 ──
        w("### 空头开仓时间分布（下跌段 10 等分桶，诊断翻空时机）\n")
        w("桶 0=顶部附近开空（理想），桶 9=底部附近才开空（翻空滞后）。值=该桶空头开仓数(pnl%)\n")
        w("| 模式 | " + " | ".join(f"桶{i}" for i in range(10)) + " |")
        w("|------|" + "|".join("---" for _ in range(10)) + "|")
        for mode in MODES:
            r = results.get(mode)
            if not r or not r["short_entries"]:
                continue
            buckets = [[0, 0.0] for _ in range(10)]
            for (eb, ep, lad, xb, xp, pnl, reason) in r["short_entries"]:
                frac = (eb - top_bar) / seg_bars if seg_bars else 0
                bi = min(9, max(0, int(frac * 10)))
                buckets[bi][0] += 1
                buckets[bi][1] += pnl
            cells = " | ".join(f"{b[0]}({pct(b[1]):+.0f})" for b in buckets)
            w(f"| {mode} | {cells} |")
        w("")

        # ── 顶后各级别首次翻空（级别越高翻空越晚 = 结构性确认滞后）──
        w("### 顶后各级别首次翻空（root 方向涌现滞后诊断）\n")
        w("缠论：方向由涌现最高级别走势决定。级别越高，走势翻空确认越滞后（区间套确认税）。\n")
        w("| 模式 | 级别 | 首次空头日期 | 价格 | 距顶跌幅 | 距顶天数 |")
        w("|------|------|------|------:|------:|------:|")
        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            allt = json.loads((FUGUE_DIR / f"t_fugue_{sym}_{mode}.json").read_text())["trades"]
            first: dict = {}
            for t in allt:
                if t[10] != "short" or t[1] < top_bar:
                    continue
                lad = t[0]
                if lad not in first or t[1] < first[lad]:
                    first[lad] = t[1]
            if not first:
                w(f"| {mode} | — | 顶后全程无空头 | — | — | — |")
                continue
            for lad in sorted(first):
                eb = first[lad]
                w(f"| {mode} | L{lad} | {dates[eb][:16]} | {closes[eb]:,.0f} "
                  f"| {(closes[eb]/top_px-1)*100:+.1f}% | {(eb-top_bar)/1440:.0f} |")
        w("")

        # ── 底部清仓追踪 ──
        w("### 底部关键时刻（空头最后在何处平仓：底部 or 早平）\n")
        for mode in MODES:
            r = results.get(mode)
            if not r or not r["short_entries"]:
                continue
            # 段内开的空头，其 exit_bar / exit_px 分布；最后一个 exit
            exits = sorted([(xb, xp) for (eb, ep, lad, xb, xp, pnl, reason) in r["short_entries"]])
            last_xb, last_xp = exits[-1]
            # 平仓价相对底部
            near_bot = sum(1 for (xb, xp) in exits if abs(xp / bot_px - 1) < 0.15)
            early = sum(1 for (xb, xp) in exits if xb < bot_bar - seg_bars * 0.2)
            w(f"- **{mode}** 段内空头最后平仓: bar={last_xb:,} @ "
              f"{dates[last_xb] if last_xb < len(dates) else 'EOD'} 价={last_xp:,.0f} "
              f"| 平在底部±15%内: {near_bot}/{len(exits)} | 早平(底前20%段长): {early}/{len(exits)}")
        w("")

        # ── 最大盈/亏空头单 ──
        w("### 段内空头 Top5 盈利 / Top5 亏损单（structural）\n")
        rs = results.get("structural")
        if rs and rs["short_rows"]:
            rows = sorted(rs["short_rows"], key=lambda x: x[7])
            w("最大亏损 5 单（lad, entry日期, entry价→exit价, held, reason, pnl%）：")
            for (lad, eb, ep, xb, xp, sh, reason, pnl) in rows[:5]:
                w(f"  - L{lad} {dates[eb][:10]}→{dates[xb][:10] if xb<len(dates) else 'EOD'} "
                  f"{ep:,.0f}→{xp:,.0f} held={xb-eb:,}b {reason} **{pct(pnl):+.1f}%**")
            w("最大盈利 5 单：")
            for (lad, eb, ep, xb, xp, sh, reason, pnl) in rows[::-1][:5]:
                w(f"  - L{lad} {dates[eb][:10]}→{dates[xb][:10] if xb<len(dates) else 'EOD'} "
                  f"{ep:,.0f}→{xp:,.0f} held={xb-eb:,}b {reason} **{pct(pnl):+.1f}%**")
        w("")

        for mode in MODES:
            r = results.get(mode)
            if not r:
                continue
            summary_rows.append((sym, mode, r))

    # ── 跨标的根因诊断 ──
    w("## 根因诊断\n")
    _diagnose(w, summary_rows)

    return "\n".join(out)


def _diagnose(w, rows):
    w("### a) 引擎在下跌段到底有没有做空？\n")
    for (sym, mode, r) in rows:
        s = r["short"]
        w(f"- {sym}/{mode}: 段内空头 **{s.n} 笔**，多头 {r['long'].n} 笔 → "
          f"{'有做空' if s.n > 0 else '**未做空**'}")
    w("")
    w("### b) 做了空为何没赚到应有利润？\n")
    for (sym, mode, r) in rows:
        s = r["short"]
        if s.n == 0:
            continue
        net = pct(s.pnl_cash)
        avg_entry = s.entry_px_sum / s.n
        avg_held = s.held_bars / s.n
        seg = r["seg_bars"]
        top_px, bot_px = r["top_px"], r["bot_px"]
        # 空头平均开仓价相对顶部：越低=翻空越滞后
        entry_vs_top = (avg_entry / top_px - 1) * 100
        held_ratio = avg_held / seg * 100
        br = s.by_reason
        churn = br.get("reduce", 0) + br.get("recover", 0)
        diag = []
        diag.append(f"均开空价={avg_entry:,.0f}(顶的{entry_vs_top:+.0f}%→翻空滞后)")
        diag.append(f"均持有={held_ratio:.0f}%段长(短差,非顶持到底)")
        diag.append(f"短差占{churn}/{s.n}")
        # 同段逆势多头失血
        ln = pct(r["long"].pnl_cash)
        diag.append(f"同段逆势多头{ln:+.0f}%NAV(吃掉空头利润)")
        w(f"- {sym}/{mode}: 空头净 {net:+.1f}%NAV | " + " | ".join(diag))
    w("")
    w("### c) 段内账户 NAV 净变化（最终裁决）\n")
    for (sym, mode, r) in rows:
        dnav = r["nav_bot"] - r["nav_top"]
        w(f"- {sym}/{mode}: 顶 NAV {r['nav_top']:,.0f} → 底 NAV {r['nav_bot']:,.0f} = "
          f"**{dnav/r['nav_top']*100:+.1f}%**（理想满仓做空者应 +数百%）")
    w("")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbols", default="BTC,ES")
    args = ap.parse_args()
    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    rep = build_report(symbols)
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(rep, encoding="utf-8")
    print(rep)
    print(f"\n[报告写入 {REPORT}]")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
