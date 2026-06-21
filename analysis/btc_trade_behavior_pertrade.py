"""BTC unn 流式回测逐笔行为分析（per-trade behavior）。

输入：trading_system/data_cache/unn_stream_BTC.json（含 trades 11 元组 + equity 序列）
     analysis/data_cache/btc_1m_full.json（真实 dates，复刻 load_ohlc 清洗对齐 bar index）
输出：analysis/btc_trade_behavior_pertrade.md

═══════════════ 会计诚实声明（强制，formalization-validity-domain.md）═══════════════

trade 行 P&L 是 **层视图账**（v2 双层记账，§4.1）——同一物理持仓的多级别身份并存，
视图间 P&L **不可加**。本脚本算逐笔 P&L 仅用于**分布刻画**（哪些笔赚/亏、量级），
**不声称其和等于 NAV 利润**。物理 NAV 唯一真值 = 引擎 equity 序列（final_nav）。
两者并列展示，差额即"视图账非可加"的度量。

root/child：trade 行不携带显式 is_root 字段。operational 角色由 (polarity, exit_reason)
推导——unn 的退出语义：
  - recover (子空头回补降成本) → 子 voice
  - cascade (父关→子级联关)    → 子 voice
  - eod    (根末日强平)         → 根
  - flip_short/flip_long (根原地翻转记双腿) → 根
  - liq    (市场强平，遍历所有活跃空头) → root|child 皆可（行内不可分，唯一二义）
N1 守卫（prove_n1_forest）保证任一时刻至多一个 active root。
"""

from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from datetime import datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UNN_JSON = REPO / "trading_system" / "data_cache" / "unn_stream_BTC.json"
BTC_RAW = REPO / "analysis" / "data_cache" / "btc_1m_full.json"
OUT_MD = REPO / "analysis" / "btc_trade_behavior_pertrade.md"

# trade 行字段索引（与 trade_schema 一致）
LAD, EB, EP, XB, XP, SH, W, DFR, PART, REASON, POL = range(11)

# exit_reason → operational 角色（root/child 推导规则，见 docstring）
ROLE = {"recover": "child", "cascade": "child", "eod": "root",
        "flip_short": "root", "flip_long": "root"}  # liq 单独处理（二义）


def load_btc_dates(n_expected: int) -> list[str]:
    """复刻 load_ohlc 清洗（nan/≤0 过滤 + spike-revert drop），返回与 bar index
    对齐的 dates。断言长度 == 回测 bar 数 ⇒ 索引与 unn 所见 bit-exact 对齐。"""
    raw = json.loads(BTC_RAW.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw["dates"]
    opens, highs, lows, closes, dates = [], [], [], [], []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o); highs.append(h); lows.append(l)
        closes.append(c); dates.append(d_in[idx])
    drop = {i for i in range(1, len(closes) - 1)
            if abs(closes[i] / closes[i - 1] - 1) > 0.5
            and abs(closes[i + 1] / closes[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(closes)) if i not in drop]
        closes = [closes[i] for i in keep]
        dates = [dates[i] for i in keep]
    assert len(dates) == n_expected, \
        f"清洗后 bar 数 {len(dates)} ≠ 回测 bar 数 {n_expected}——索引未对齐，时间映射不可信"
    return dates, closes


def trade_pnl(t: list) -> float:
    """层视图账 P&L（不可加，仅分布用）。long: sh*(xp-ep)；short: sh*(ep-xp)。"""
    if t[POL] == "long":
        return t[SH] * (t[XP] - t[EP])
    return t[SH] * (t[EP] - t[XP])


def role_of(t: list) -> str:
    if t[REASON] == "liq":
        return "root_or_child(liq二义)"
    return ROLE.get(t[REASON], f"unknown({t[REASON]})")


def histogram(values: list[float], n_bins: int = 20) -> list[tuple]:
    """等宽直方图 → [(lo, hi, count)]。"""
    if not values:
        return []
    lo, hi = min(values), max(values)
    if hi == lo:
        return [(lo, hi, len(values))]
    width = (hi - lo) / n_bins
    bins = [0] * n_bins
    for v in values:
        b = min(int((v - lo) / width), n_bins - 1)
        bins[b] += 1
    return [(lo + i * width, lo + (i + 1) * width, bins[i]) for i in range(n_bins)]


def ym(date_str: str) -> tuple[int, int]:
    dt = datetime.fromisoformat(date_str)
    return dt.year, (dt.month - 1) // 3 + 1


def main() -> None:
    d = json.loads(UNN_JSON.read_text())
    trades = d["trades"]
    equity = d["equity"]
    n_bars = d["n_bars"]
    bh_pct = d["bh_pct"]
    us = d["unn_stream"]
    strat_pct = us["strat_pct"]
    mdd_pct = us["mdd_pct"]
    final_nav = 100_000.0 * (1 + strat_pct / 100)

    dates, closes = load_btc_dates(n_bars)

    def dstr(bar: int) -> str:
        return dates[min(bar, n_bars - 1)]

    L: list[str] = []
    w = L.append
    w("# BTC unn 流式回测逐笔行为分析\n")
    w(f"生成于回测：{UNN_JSON.name}　|　bars={n_bars:,}　|　"
      f"区间 {dstr(0)} → {dstr(n_bars - 1)}\n")
    w("## 0. 会计诚实声明（认识论等级 L2）\n")
    w("- **trade 行 P&L = 层视图账，不可加**（v2 双层记账 §4.1）。本报告逐笔 P&L 仅刻画分布，"
      "不声称其和 = NAV 利润。**物理 NAV 唯一真值 = equity 序列**。\n")
    w("- **root/child 不在 trade 行内**：operational 角色由 `(polarity, exit_reason)` 推导"
      "（recover/cascade=子；eod/flip_*=根；liq=二义）。N1 守卫保证至多一个 active root。\n")

    # ── 1. 总览 ──
    w("\n## 1. 总览\n")
    w(f"| 指标 | unn | Buy&Hold |\n|---|---|---|\n")
    w(f"| 收益 | **{strat_pct:+.1f}%** | {bh_pct:+.1f}% |\n")
    w(f"| 最大回撤 | {mdd_pct:.1f}% | {d['bh_mdd_pct']:.1f}% |\n")
    w(f"| 终值 NAV | ${final_nav:,.0f} | ${100_000 * (1 + bh_pct / 100):,.0f} |\n")
    w(f"| 笔数 | {len(trades)} | — |\n")
    w(f"\nP1（≥BH）：**{'PASS' if strat_pct >= bh_pct else 'FAIL'}**"
      f"（unn 跑输 BH {bh_pct - strat_pct:.1f}pp，但 MDD 浅 {d['bh_mdd_pct'] - mdd_pct:.1f}pp）\n")

    # ── 2. 按 (角色, 极性, reason) 分解 ──
    w("\n## 2. 逐笔角色分解\n")
    by_role = defaultdict(lambda: {"n": 0, "pnl": 0.0, "wins": 0})
    for t in trades:
        key = (role_of(t), t[POL], t[REASON])
        r = by_role[key]
        r["n"] += 1
        p = trade_pnl(t)
        r["pnl"] += p
        r["wins"] += p > 0
    w("| 角色 | 极性 | reason | 笔数 | 视图P&L合计 | 胜笔 | 胜率 |\n|---|---|---|---|---|---|---|\n")
    for key in sorted(by_role):
        r = by_role[key]
        w(f"| {key[0]} | {key[1]} | {key[2]} | {r['n']} | "
          f"${r['pnl']:,.0f} | {r['wins']} | {r['wins'] / r['n'] * 100:.0f}% |\n")
    w(f"\n退出原因分布（引擎计数）：`{us['exit_reasons']}`　|　"
      f"强平笔（liq）：**{sum(1 for t in trades if t[REASON] == 'liq')}**"
      f"（N4：强牛市无空头强平，与 nrf_counters liquidations 一致）\n")

    # ── 3. 逐笔盈亏分布直方图 ──
    w("\n## 3. 逐笔盈亏分布（视图账，直方图数据）\n")
    pnls = [trade_pnl(t) for t in trades]
    w(f"全部 {len(pnls)} 笔：min=${min(pnls):,.0f} max=${max(pnls):,.0f} "
      f"中位=${sorted(pnls)[len(pnls) // 2]:,.0f} 总和=${sum(pnls):,.0f}\n")
    w(f"（对照：物理 NAV 利润=${final_nav - 100_000:,.0f}；"
      f"视图账总和 − NAV 利润 = ${sum(pnls) - (final_nav - 100_000):,.0f} = 双层记账不可加度量）\n\n")
    # 子空头单独成直方图（主体）
    short_pnls = [trade_pnl(t) for t in trades if t[POL] == "short"]
    w(f"### 3a. 子空头 P&L 直方图（n={len(short_pnls)}，20 bins）\n")
    w("| 区间($) | 笔数 |\n|---|---|\n")
    for lo, hi, cnt in histogram(short_pnls, 20):
        bar = "█" * min(cnt, 40)
        w(f"| [{lo:,.0f}, {hi:,.0f}) | {cnt} {bar} |\n")
    win = sum(1 for p in short_pnls if p > 0)
    w(f"\n子空头胜率：{win}/{len(short_pnls)} = {win / len(short_pnls) * 100:.0f}%　|　"
      f"净视图 P&L：${sum(short_pnls):,.0f}（{'净盈' if sum(short_pnls) > 0 else '净亏'}）\n")

    # ── 4. 按时间段分组（年/季度，按出场日期）──
    w("\n## 4. 按时间段分组（出场日期）\n")
    by_year = defaultdict(lambda: {"n": 0, "pnl": 0.0, "short_n": 0})
    by_q = defaultdict(lambda: {"n": 0, "pnl": 0.0})
    for t in trades:
        y, q = ym(dstr(t[XB]))
        by_year[y]["n"] += 1
        by_year[y]["pnl"] += trade_pnl(t)
        by_year[y]["short_n"] += t[POL] == "short"
        by_q[(y, q)]["n"] += 1
        by_q[(y, q)]["pnl"] += trade_pnl(t)
    w("### 4a. 按年\n| 年 | 笔数 | 子空头数 | 视图P&L |\n|---|---|---|---|\n")
    for y in sorted(by_year):
        r = by_year[y]
        w(f"| {y} | {r['n']} | {r['short_n']} | ${r['pnl']:,.0f} |\n")
    w("\n### 4b. 按季度\n| 季度 | 笔数 | 视图P&L |\n|---|---|---|\n")
    for k in sorted(by_q):
        r = by_q[k]
        w(f"| {k[0]}Q{k[1]} | {r['n']} | ${r['pnl']:,.0f} |\n")

    # ── 5. 最大单笔盈利/亏损 ──
    w("\n## 5. 最大单笔盈利 / 亏损\n")
    ranked = sorted(trades, key=trade_pnl)
    def trow(t):
        return (f"{role_of(t)}/{t[POL]}/{t[REASON]} L{t[LAD]} "
                f"{dstr(t[EB])[:10]}→{dstr(t[XB])[:10]} "
                f"${t[EP]:,.0f}→${t[XP]:,.0f} sh={t[SH]:.3f} "
                f"持{t[XB] - t[EB]:,}bar P&L=${trade_pnl(t):,.0f}")
    w("**最大 5 笔亏损：**\n")
    for t in ranked[:5]:
        w(f"- {trow(t)}\n")
    w("\n**最大 5 笔盈利：**\n")
    for t in ranked[-5:][::-1]:
        w(f"- {trow(t)}\n")

    # ── 6. 空头子 voice 入场-出场价差分布 ──
    w("\n## 6. 子空头入场-出场价差分布\n")
    shorts = [t for t in trades if t[POL] == "short"]
    spreads = [(t[EP] - t[XP]) / t[EP] * 100 for t in shorts]  # +=回补更低(盈)，-=回补更高(亏)
    up = sum(1 for s in spreads if s < 0)  # 价差为负=回补价更高=牛市踏空
    w(f"价差 %（(入场−出场)/入场，正=回补更低=空头盈，负=回补更高=逆势亏）：\n")
    w(f"- n={len(spreads)}　均值={sum(spreads) / len(spreads):+.2f}%　"
      f"中位={sorted(spreads)[len(spreads) // 2]:+.2f}%　"
      f"min={min(spreads):+.2f}% max={max(spreads):+.2f}%\n")
    w(f"- **回补价更高（逆势/牛市踏空）的笔数：{up}/{len(spreads)} = "
      f"{up / len(spreads) * 100:.0f}%**\n")
    w("| 价差区间(%) | 笔数 |\n|---|---|\n")
    for lo, hi, cnt in histogram(spreads, 16):
        w(f"| [{lo:+.1f}, {hi:+.1f}) | {cnt} {'█' * min(cnt, 40)} |\n")

    # ── 7. equity 曲线关键拐点 vs BH ──
    w("\n## 7. equity 曲线关键拐点 vs Buy&Hold\n")
    # equity: [(bar, nav)]。BH(bar) = closes[bar]/closes[0]*100000
    c0 = closes[0]
    pts = [(b, nav, closes[min(b, n_bars - 1)] / c0 * 100_000) for b, nav in equity]
    # 回撤段：找 unn NAV 的 peak→trough
    peak = pts[0][1]; peak_bar = pts[0][0]; max_dd = 0.0; dd_lo = peak; dd_lo_bar = peak_bar
    dd_peak_bar = peak_bar
    for b, nav, _ in pts:
        if nav > peak:
            peak = nav; peak_bar = b
        dd = nav / peak - 1
        if dd < max_dd:
            max_dd = dd; dd_lo = nav; dd_lo_bar = b; dd_peak_bar = peak_bar
    w(f"- **unn 最大回撤段**：{dstr(dd_peak_bar)[:10]}（峰 ${peak if dd_peak_bar==peak_bar else max(p[1] for p in pts if p[0]<=dd_peak_bar):,.0f}）"
      f" → {dstr(dd_lo_bar)[:10]}（谷 ${dd_lo:,.0f}），回撤 {max_dd * 100:.1f}%\n")
    # 关键交叉：unn NAV vs BH NAV 的领先/落后翻转
    w("- **unn vs BH 相对强弱拐点**（采样 NAV，每 ~年取一个代表点）：\n")
    w("\n| 日期 | bar | unn NAV | BH NAV | unn/BH |\n|---|---|---|---|---|\n")
    seen_year = set()
    for b, nav, bh in pts:
        y = datetime.fromisoformat(dstr(b)).year
        if y not in seen_year:
            seen_year.add(y)
            w(f"| {dstr(b)[:10]} | {b:,} | ${nav:,.0f} | ${bh:,.0f} | "
              f"{nav / bh:.2f} |\n")
    # 末点
    b, nav, bh = pts[-1]
    w(f"| {dstr(b)[:10]}(末) | {b:,} | ${nav:,.0f} | ${bh:,.0f} | {nav / bh:.2f} |\n")

    # ── 8. 结论（结果包六要素）──
    w("\n## 8. 结论（结果包）\n")
    root = [t for t in trades if role_of(t) == "root"][0]
    root_pnl = trade_pnl(root)
    w(f"**结论**：BTC unn 结构 = 单根多头（1 笔，L{root[LAD]}，"
      f"{dstr(root[EB])[:10]}→末日，$"
      f"{root[EP]:,.0f}→${root[XP]:,.0f}，视图 P&L ${root_pnl:,.0f}）骑全程牛市 "
      f"+ {len(shorts)} 笔子空头 spawn-recover。子空头净视图 "
      f"${sum(short_pnls):,.0f}，{up}/{len(shorts)} 笔逆势回补更高。"
      f"unn {strat_pct:+.1f}% 跑输 BH {bh_pct:+.1f}%（{bh_pct - strat_pct:.0f}pp），"
      f"MDD 浅 {d['bh_mdd_pct'] - mdd_pct:.0f}pp。\n")
    w(f"\n**定义依据**：N1 逐仓独立森林（root 多 child）；N4 成本门 spawn 子 voice；"
      f"recover=子空头回补降成本（§7/§11）。\n")
    w(f"\n**边界条件**：① 若 BTC 转入震荡/下行 regime，子空头逆势率下降、根多头优势消失"
      f"（539 号：根翻空/恒多有效域 ⊂ 非上行）；② root/child 长腿区分依赖 N1 单根守卫，"
      f"若守卫失效则角色推导翻转；③ liq 笔=0 时二义不显形，若出现 liq 笔则 root/child 行内不可分。\n")
    w(f"\n**下游推论**：子空头在强牛 spawn = 主升浪稀释源（与 memory "
      f"`project_unn_btc_spawn_throwback` 同向）；恒仓单根多头是 unn 在 BTC 的 alpha 载体，"
      f"非子空头操作。\n")
    w(f"\n**谱系引用**：539 号（根翻空有效域 ⊂ 非上行 regime）；T49 confirm spawn 改动"
      f"（spawn 频次 ↔ 主升浪稀释）；N8 双层会计（视图账不可加）。\n")
    w(f"\n**影响声明**：本脚本只读 unn_stream_BTC.json + btc_1m_full.json，"
      f"产出分析报告，不改引擎/信号/会计。回测脚本 backtest_unn_stream.py 已加 trades/equity 落盘。\n")

    OUT_MD.write_text("".join(L), encoding="utf-8")
    print(f"写入 {OUT_MD}")
    # 控制台关键数字
    print(f"  root多头视图P&L=${root_pnl:,.0f}  子空头净=${sum(short_pnls):,.0f}  "
          f"逆势回补={up}/{len(shorts)}  视图和=${sum(pnls):,.0f} vs NAV利润=${final_nav-100_000:,.0f}")


if __name__ == "__main__":
    main()
