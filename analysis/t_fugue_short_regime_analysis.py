"""T 赋格引擎空头交易分段盈亏分析（按 price regime 归因）。

═══════════════ 数据源辨认（关键） ═══════════════

  本分析消费 **T 赋格引擎** 的逐笔明细：trading_system/data_cache/t_fugue_<SYM>_<MODE>.json
  （recursive_t::t_engine 完整递归嵌套多重赋格，**多空双向**，n_short>0）。

  ⚠ 不消费 analysis/data_cache/t_backtest_*.json —— 那是 backtest.rs 的 long-only **简单版**
    （"卖点+空仓→不动；简单版不做空"，backtest.rs:242），n_short≡0，无法做空头分析。

  两者数字差一个数量级（BTC：简单版 +1155%/+1269% vs 赋格 -96%/+191.7%）。

trade11 schema:
  [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
   weight_at_entry, deferred_bars, partial, exit_reason, polarity]
  已实现盈亏（现金）: long = shares*(xp-ep)；short = shares*(ep-xp)
  对 NAV 贡献%（与 strat_pct 同口径，可加总）: pnl_cash / INITIAL_CAPITAL(=100000) * 100

═══════════════ regime 分段（独立于交易结果） ═══════════════

  用 **ZigZag 峰谷分段**（百分比阈值 theta）从 closes 序列本身切出交替的上涨段/下跌段。
  这一步**不看任何交易**——避免"用 pnl 符号定义 regime"的循环论证。
  每笔交易按其 entry_bar 落在哪一段，归类为「顺势/逆势」：
    - 空头开在下跌段 = 顺势空头（应赚）
    - 空头开在上涨段 = 逆势空头（抓回调，多半亏）
  震荡：ZigZag 段天然是单调趋势段；"震荡区"= 相邻短段密集翻转处（用段时长分布诊断），
        另以 theta 敏感性（0.08/0.15/0.30）报告结论稳定性 = 边界条件。

认识论等级：L3（真实数据逐笔归因，可否证）。regime 分段方法本身是 L0 工具（无数据假设）。

用法（仓库根目录）:
    PYTHONPATH=trading_system .venv/bin/python analysis/t_fugue_short_regime_analysis.py
    PYTHONPATH=trading_system .venv/bin/python analysis/t_fugue_short_regime_analysis.py --theta 0.15
"""

from __future__ import annotations

import argparse
import bisect
import json
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "trading_system"))

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402

CACHE = REPO_ROOT / "trading_system" / "data_cache"
REPORT_DIR = REPO_ROOT / "analysis" / "reports"
INITIAL_CAPITAL = 100_000.0
SYMBOLS = ("CL", "BRN", "DX", "GC", "ES", "QQQ", "BTC", "OKLO")
MODES = ("structural", "and", "or")


# ════════════════════════ ZigZag regime 分段 ════════════════════════


@dataclass(frozen=True)
class Segment:
    start_bar: int
    end_bar: int
    direction: str  # 'up' | 'down'
    start_px: float
    end_px: float

    @property
    def ret_pct(self) -> float:
        return (self.end_px / self.start_px - 1.0) * 100.0 if self.start_px else 0.0

    @property
    def n_bars(self) -> int:
        return self.end_bar - self.start_bar


def zigzag_segments(closes: list[float], theta: float) -> list[Segment]:
    """ZigZag 峰谷分段：反向变动 ≥ theta（相对极值）确认转折点。

    返回交替的 up（谷→峰）/down（峰→谷）段，连续覆盖 [0, n-1]，相邻首尾相接。
    最后一段用末值 close 收尾（未确认的活动段，方向取当前 trend）。
    """
    n = len(closes)
    if n < 2:
        return []
    up = 1.0 + theta
    dn = 1.0 - theta

    # 初始趋势：从 closes[0] 起，第一个偏离 ±theta 的方向。
    trend = 0
    piv_i, piv_px = 0, closes[0]
    ext_i, ext_px = 0, closes[0]
    i = 1
    while i < n and trend == 0:
        c = closes[i]
        if c >= piv_px * up:
            trend = 1
            ext_i, ext_px = i, c
        elif c <= piv_px * dn:
            trend = -1
            ext_i, ext_px = i, c
        i += 1

    if trend == 0:  # 全程未超 theta：单一 range 段（方向按净变化）
        d = "up" if closes[-1] >= closes[0] else "down"
        return [Segment(0, n - 1, d, closes[0], closes[-1])]

    segs: list[Segment] = []
    for j in range(i, n):
        c = closes[j]
        if trend == 1:
            if c > ext_px:
                ext_i, ext_px = j, c
            elif c <= ext_px * dn:  # 确认峰
                segs.append(Segment(piv_i, ext_i, "up", piv_px, ext_px))
                piv_i, piv_px = ext_i, ext_px
                trend = -1
                ext_i, ext_px = j, c
        else:
            if c < ext_px:
                ext_i, ext_px = j, c
            elif c >= ext_px * up:  # 确认谷
                segs.append(Segment(piv_i, ext_i, "down", piv_px, ext_px))
                piv_i, piv_px = ext_i, ext_px
                trend = 1
                ext_i, ext_px = j, c
    # 收尾活动段（piv → 末值），方向 = 当前 trend
    last_dir = "up" if trend == 1 else "down"
    segs.append(Segment(piv_i, n - 1, last_dir, piv_px, closes[-1]))
    return segs


def regime_at(segs: list[Segment], bar: int, starts: list[int]) -> str:
    """entry_bar 落在哪一段 → 段方向。starts = 各段 start_bar（升序）。"""
    k = bisect.bisect_right(starts, bar) - 1
    if k < 0:
        k = 0
    if k >= len(segs):
        k = len(segs) - 1
    return segs[k].direction


# ════════════════════════ 逐笔归因 ════════════════════════


@dataclass
class Bucket:
    n: int = 0
    pnl_cash: float = 0.0
    wins: int = 0
    held_bars: int = 0

    def add(self, pnl: float, held: int) -> None:
        self.n += 1
        self.pnl_cash += pnl
        self.wins += pnl > 0
        self.held_bars += held

    @property
    def nav_pct(self) -> float:
        return self.pnl_cash / INITIAL_CAPITAL * 100.0

    @property
    def win_rate(self) -> float:
        return self.wins / self.n * 100.0 if self.n else 0.0

    @property
    def avg_held(self) -> float:
        return self.held_bars / self.n if self.n else 0.0


def trade_pnl(ep: float, xp: float, sh: float, pol: str) -> float:
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


def analyze_symbol_mode(sym: str, mode: str, segs: list[Segment]) -> dict | None:
    """单 (标的,模式)：逐笔投影到 regime，聚合 polarity × regime。"""
    f = CACHE / f"t_fugue_{sym}_{mode}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    trades = d["trades"]
    starts = [s.start_bar for s in segs]
    # 键: (polarity, regime_dir)
    buckets: dict[tuple[str, str], Bucket] = {}
    for (_lad, eb, ep, xb, xp, sh, _w, _dfr, _part, _reason, pol) in trades:
        reg = regime_at(segs, eb, starts)
        b = buckets.setdefault((pol, reg), Bucket())
        b.add(trade_pnl(ep, xp, sh, pol), xb - eb)
    return {
        "symbol": sym,
        "mode": mode,
        "strat_pct": d["strat_pct"],
        "bh_pct": d["bh_pct"],
        "n_long": d["n_long"],
        "n_short": d["n_short"],
        "buckets": buckets,
    }


def bucket_get(res: dict, pol: str, reg: str) -> Bucket:
    return res["buckets"].get((pol, reg), Bucket())


# ════════════════════════ 报告 ════════════════════════


def fmt_bucket(b: Bucket) -> str:
    if b.n == 0:
        return f"{'—':>30}"
    return (f"n={b.n:<5} pnl={b.nav_pct:+7.1f}%NAV "
            f"win={b.win_rate:4.0f}% held={b.avg_held:6.0f}b")


def build_report(theta: float, theta_alt: tuple[float, ...]) -> str:
    out: list[str] = []
    w = out.append

    w("# T 赋格引擎 · 空头交易分段盈亏分析（按 price regime 归因）\n")
    w(f"- 数据源：`trading_system/data_cache/t_fugue_<SYM>_<MODE>.json`（T 赋格，多空双向）")
    w(f"- regime 分段：ZigZag 峰谷，主阈值 **theta={theta:.0%}**（敏感性见末节 {theta_alt}）")
    w(f"- pnl 口径：已实现现金 / 初始资本 {INITIAL_CAPITAL:,.0f} × 100 = 对 NAV 贡献%（可加总，与 strat_pct 同口径）")
    w(f"- 认识论等级：**L3**（真实数据逐笔归因，可否证）\n")

    # 加载 closes + regime（每标的一次）
    seg_by_sym: dict[str, list[Segment]] = {}
    closes_by_sym: dict[str, list[float]] = {}
    for sym in SYMBOLS:
        if sym not in SYMBOL_FILES:
            continue
        try:
            _o, _h, _l, closes, _y = load_ohlc(SYMBOL_FILES[sym])
        except Exception as e:  # noqa: BLE001
            w(f"> ⚠ {sym} closes 加载失败：{e}")
            continue
        closes_by_sym[sym] = closes
        seg_by_sym[sym] = zigzag_segments(closes, theta)

    # ── 1. regime 段落分布 ──
    w("## 1. 各标的 price regime 段落分布（ZigZag）\n")
    w("| 标的 | bars | BH% | 段数 | 上涨段 | 下跌段 | up占bar% | down占bar% |")
    w("|------|-----:|----:|----:|-------:|-------:|--------:|----------:|")
    for sym in SYMBOLS:
        segs = seg_by_sym.get(sym)
        if not segs:
            continue
        closes = closes_by_sym[sym]
        bh = (closes[-1] / closes[0] - 1) * 100
        nb = len(closes)
        ups = [s for s in segs if s.direction == "up"]
        dns = [s for s in segs if s.direction == "down"]
        up_bars = sum(s.n_bars for s in ups)
        dn_bars = sum(s.n_bars for s in dns)
        w(f"| {sym} | {nb:,} | {bh:+.0f} | {len(segs)} | {len(ups)} | {len(dns)} "
          f"| {up_bars/nb*100:.0f} | {dn_bars/nb*100:.0f} |")
    w("")

    # 收集全部 (sym,mode) 结果
    all_res: dict[tuple[str, str], dict] = {}
    for sym in SYMBOLS:
        segs = seg_by_sym.get(sym)
        if not segs:
            continue
        for mode in MODES:
            r = analyze_symbol_mode(sym, mode, segs)
            if r:
                all_res[(sym, mode)] = r

    # ── 2. 空头：上涨段 vs 下跌段（全标的×模式）──
    w("## 2. 空头交易按 regime 分段（顺势=下跌段 / 逆势=上涨段）\n")
    w("| 标的 | 模式 | 空头@下跌段(顺势) | 空头@上涨段(逆势) | 空头净 |")
    w("|------|------|---|---|---:|")
    for (sym, mode), r in all_res.items():
        sd = bucket_get(r, "short", "down")
        su = bucket_get(r, "short", "up")
        net = sd.nav_pct + su.nav_pct
        w(f"| {sym} | {mode} | {fmt_bucket(sd)} | {fmt_bucket(su)} | {net:+.1f}%NAV |")
    w("")

    # ── 3. 多头：上涨段 vs 下跌段 ──
    w("## 3. 多头交易按 regime 分段（顺势=上涨段 / 逆势=下跌段）\n")
    w("| 标的 | 模式 | 多头@上涨段(顺势) | 多头@下跌段(逆势) | 多头净 |")
    w("|------|------|---|---|---:|")
    for (sym, mode), r in all_res.items():
        lu = bucket_get(r, "long", "up")
        ld = bucket_get(r, "long", "down")
        net = lu.nav_pct + ld.nav_pct
        w(f"| {sym} | {mode} | {fmt_bucket(lu)} | {fmt_bucket(ld)} | {net:+.1f}%NAV |")
    w("")

    # ── 4. BTC 三模式深挖 ──
    w("## 4. BTC 三模式对比（AND +191.7% vs Structural -96% vs OR -94.8%）\n")
    w("逐 polarity × regime 分解（NAV 贡献%）：\n")
    w("| 模式 | 空头@跌(顺) | 空头@涨(逆) | 多头@涨(顺) | 多头@跌(逆) | 空头净 | 多头净 | Σ已实现 | strat% |")
    w("|------|---|---|---|---|---:|---:|---:|---:|")
    for mode in MODES:
        r = all_res.get(("BTC", mode))
        if not r:
            w(f"| {mode} | (无数据) |")
            continue
        sd, su = bucket_get(r, "short", "down"), bucket_get(r, "short", "up")
        lu, ld = bucket_get(r, "long", "up"), bucket_get(r, "long", "down")
        sn, ln = sd.nav_pct + su.nav_pct, lu.nav_pct + ld.nav_pct
        w(f"| {mode} "
          f"| {sd.nav_pct:+.0f}(n{sd.n}) | {su.nav_pct:+.0f}(n{su.n}) "
          f"| {lu.nav_pct:+.0f}(n{lu.n}) | {ld.nav_pct:+.0f}(n{ld.n}) "
          f"| {sn:+.0f} | {ln:+.0f} | {sn+ln:+.0f} | {r['strat_pct']:+.1f} |")
    w("")

    # ── 5. 关键问题 ──
    w("## 5. 关键问题作答\n")
    _answer_key_questions(w, all_res)

    # ── 6. theta 敏感性（边界条件）──
    w("## 6. theta 敏感性（结论边界条件）\n")
    w("「下跌段空头净盈亏符号」在不同 theta 下是否稳定（+ = 顺势空头赚钱）：\n")
    w("| 标的 | 模式 | " + " | ".join(f"θ={t:.0%}" for t in theta_alt) + " |")
    w("|------|------|" + "|".join("---" for _ in theta_alt) + "|")
    for sym in ("BTC", "CL", "OKLO", "GC", "ES"):
        for mode in MODES:
            if (sym, mode) not in all_res:
                continue
            cells = []
            closes = closes_by_sym.get(sym)
            if not closes:
                continue
            for t in theta_alt:
                segs = zigzag_segments(closes, t)
                rr = analyze_symbol_mode(sym, mode, segs)
                sd = bucket_get(rr, "short", "down")
                cells.append(f"{sd.nav_pct:+.0f}%(n{sd.n})")
            w(f"| {sym} | {mode} | " + " | ".join(cells) + " |")
    w("")

    return "\n".join(out)


def _answer_key_questions(w, all_res: dict) -> None:
    # 跨全标的×模式：下跌段空头净、上涨段空头净
    short_down_total = sum(bucket_get(r, "short", "down").nav_pct for r in all_res.values())
    short_up_total = sum(bucket_get(r, "short", "up").nav_pct for r in all_res.values())
    # 下跌段空头盈利的组数
    sd_pos = sum(1 for r in all_res.values() if bucket_get(r, "short", "down").nav_pct > 0)
    sd_tot = sum(1 for r in all_res.values() if bucket_get(r, "short", "down").n > 0)

    w(f"**Q1: 下跌段做空到底赚不赚钱？**")
    w(f"- 全 {len(all_res)} 组中，下跌段空头净盈利的有 **{sd_pos}/{sd_tot}** 组。")
    w(f"- 全样本下跌段空头 NAV 贡献合计 = **{short_down_total:+.0f}%**；上涨段空头合计 = **{short_up_total:+.0f}%**。\n")

    w(f"**Q2: 整体空头为何负——上涨段亏太多 vs 下跌段赚太少？**")
    for (sym, mode), r in all_res.items():
        sd = bucket_get(r, "short", "down")
        su = bucket_get(r, "short", "up")
        if sd.n + su.n == 0:
            continue
        diag = "上涨段失血主导" if abs(su.nav_pct) > abs(sd.nav_pct) else "下跌段未能盈利主导"
        if sd.nav_pct + su.nav_pct >= 0:
            diag = "空头整体为正"
        w(f"- {sym}/{mode}: 跌段{sd.nav_pct:+.0f}% + 涨段{su.nav_pct:+.0f}% = {sd.nav_pct+su.nav_pct:+.0f}% → {diag}")
    w("")

    # BTC AND vs Structural
    rs = all_res.get(("BTC", "structural"))
    ra = all_res.get(("BTC", "and"))
    if rs and ra:
        w(f"**Q3: AND 为何在 BTC 大幅改善（{rs['strat_pct']:+.0f}% → {ra['strat_pct']:+.0f}%）？**")
        s_su, a_su = bucket_get(rs, "short", "up"), bucket_get(ra, "short", "up")
        s_sd, a_sd = bucket_get(rs, "short", "down"), bucket_get(ra, "short", "down")
        s_lu, a_lu = bucket_get(rs, "long", "up"), bucket_get(ra, "long", "up")
        w(f"- 上涨段空头（逆势）：structural n={s_su.n} {s_su.nav_pct:+.0f}% → AND n={a_su.n} {a_su.nav_pct:+.0f}% "
          f"（笔数Δ={a_su.n-s_su.n}, NAVΔ={a_su.nav_pct-s_su.nav_pct:+.0f}%）")
        w(f"- 下跌段空头（顺势）：structural n={s_sd.n} {s_sd.nav_pct:+.0f}% → AND n={a_sd.n} {a_sd.nav_pct:+.0f}%")
        w(f"- 上涨段多头（顺势）：structural n={s_lu.n} {s_lu.nav_pct:+.0f}% → AND n={a_lu.n} {a_lu.nav_pct:+.0f}%")
        w("")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--theta", type=float, default=0.15)
    ap.add_argument("--theta-alt", default="0.08,0.15,0.30")
    args = ap.parse_args()
    theta_alt = tuple(float(x) for x in args.theta_alt.split(","))

    report = build_report(args.theta, theta_alt)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    out = REPORT_DIR / "t_fugue_short_regime_report.md"
    out.write_text(report, encoding="utf-8")
    print(report)
    print(f"\n[报告已写入 {out}]")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
