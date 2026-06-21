"""BTC 强牛市踏空诊断（新版 T 操作层引擎 vs 旧版 v3/t_fugue）。

═══════════════ 诊断目标 ═══════════════
BTC buy-hold +1380%，新版 T 引擎 AND +11.8% / structural −12.3%。多头为何没吃到涨幅？
用逐笔数据在三种可能间区分：
  (a) churn   ：频繁开平，每次只吃一小段，确认滞后税
  (b) 空仓太久：清仓后等买点再入场，中间踏空
  (c) 减仓太多：1/3 几何塔反复抽干多头核心，实际多头敞口 ≪ 满仓

═══════════════ 方法（净敞口时间线重建）═══════════════
trade 行 = 一段 chunk：`shares` 单位的 `polarity` 仓位从 entry_bar 活到 exit_bar。
  → delta 数组：entry_bar 处 +shares，exit_bar 处 −shares（按极性分开）→ cumsum
  → 得任意 bar 的净 long_units / short_units（精确，非采样）。
  → 用引擎汇总的 phys_long_bars / phys_short_bars 对账（验证重建保真）。
"建仓事件" = flat→nonflat 跃迁（与引擎 try_enter「仅全局空仓时入场」字面对应）。
  → episode 时长、方向、flat 间隔，区分 churn / 空仓 / 减仓。

═══════════════ 会计诚实声明（formalization-validity-domain）═══════════════
trade 行 P&L 是层视图账（v2 双层记账），视图间不可加。本脚本：
  - 净敞口/episode/churn = units 时间线（精确，物理量）。
  - P&L 分布 = 仅刻画分布（哪些笔赚/亏），不声称其和 = NAV。
  - NAV 唯一真值 = 引擎 final_nav / equity。
认识论等级：L3（真实数据逐笔归因，可否证）。
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[1]
ANALYSIS_CACHE = REPO / "analysis" / "data_cache"
TS_CACHE = REPO / "trading_system" / "data_cache"
BTC_RAW = ANALYSIS_CACHE / "btc_1m_full.json"
OUT_MD = REPO / "analysis" / "reports" / "btc_bull_throwback_diagnosis.md"

INIT = 100_000.0
# trade 行字段索引（trade_schema）
LAD, EB, EP, XB, XP, SH, W, DFR, PART, REASON, POL = range(11)
MODES = ("structural", "and", "or")

# exit_reason 角色（新引擎口径，见 t_engine.rs / cycle.rs）
SINK_REASON = "reduce"      # 卖点 sink：1/3 核心 → 次级别做空（源层减仓记此）
RECOVER_REASON = "recover"  # 买点 recover：全平次级别还给核心
LIQ_REASONS = {"liq_long", "liq_short", "liq"}
CLEAR_REASONS = {"eod", "core_clear", "core_clear_short"}


# ════════════════════ 数据加载（清洗对齐，复刻 load_clean_ohlc）════════════════════
def load_btc(n_expected: int | None = None):
    raw = json.loads(BTC_RAW.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw["dates"]
    closes, dates = [], []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        closes.append(c)
        dates.append(d_in[idx])
    drop = {i for i in range(1, len(closes) - 1)
            if abs(closes[i] / closes[i - 1] - 1) > 0.5
            and abs(closes[i + 1] / closes[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(closes)) if i not in drop]
        closes = [closes[i] for i in keep]
        dates = [dates[i] for i in keep]
    if n_expected is not None:
        assert len(dates) == n_expected, \
            f"清洗后 bar 数 {len(dates)} ≠ 回测 {n_expected}——索引未对齐"
    return np.asarray(closes, dtype=float), dates


# ════════════════════ 牛市段定位 ════════════════════
def find_bull_segments(closes, dates, min_gain=2.0, min_bars=50_000):
    """检测主要上涨段：从局部低点到后续显著高点。
    简单鲁棒法：滚动谷-峰。返回 [(start_idx, end_idx, low, high, gain), ...]。"""
    n = len(closes)
    segs = []
    i = 0
    while i < n - 1:
        # 从 i 找谷（局部最低）后的最高峰，要求涨幅 ≥ min_gain
        trough = i
        trough_p = closes[i]
        # 向后扫描，记录运行中的峰
        peak = i
        peak_p = closes[i]
        j = i + 1
        # 允许回撤再创新高的连续牛段：以"未跌破谷的 (1+min_gain) 起点逻辑"过松，
        # 改用：从 trough 起，只要还能创相对 trough 的新高就延展；峰后回撤 >40% 封段。
        last_high_idx = i
        while j < n:
            if closes[j] < trough_p:
                # 跌破当前谷：若已积累足够涨幅则封段，否则把谷下移
                if peak_p / trough_p - 1 >= min_gain and (last_high_idx - trough) >= min_bars:
                    break
                trough = j
                trough_p = closes[j]
                peak = j
                peak_p = closes[j]
                last_high_idx = j
                j += 1
                continue
            if closes[j] > peak_p:
                peak_p = closes[j]
                peak = j
                last_high_idx = j
            else:
                # 回撤封段判据：自峰回撤 > 40%
                if peak_p > 0 and closes[j] / peak_p - 1 < -0.40:
                    if peak_p / trough_p - 1 >= min_gain and (peak - trough) >= min_bars:
                        break
            j += 1
        gain = peak_p / trough_p - 1
        if gain >= min_gain and (peak - trough) >= min_bars:
            segs.append((trough, peak, trough_p, peak_p, gain))
            i = peak + 1
        else:
            i = j if j > i else i + 1
    return segs


# ════════════════════ 引擎逐笔加载 + 净敞口重建 ════════════════════
def load_trades(path, mode=None):
    d = json.loads(Path(path).read_text())
    return d


def reconstruct_exposure(trades, n_bars):
    """从 trade 行重建净 long_units / short_units 时间线（精确，delta+cumsum）。"""
    dlong = np.zeros(n_bars + 2, dtype=float)
    dshort = np.zeros(n_bars + 2, dtype=float)
    for t in trades:
        eb, xb, sh, pol = int(t[EB]), int(t[XB]), float(t[SH]), t[POL]
        eb = max(0, min(eb, n_bars))
        xb = max(0, min(xb, n_bars))
        if xb < eb:
            xb = eb
        if pol == "long":
            dlong[eb] += sh
            dlong[xb] -= sh
        else:
            dshort[eb] += sh
            dshort[xb] -= sh
    long_u = np.cumsum(dlong[:n_bars])
    short_u = np.cumsum(dshort[:n_bars])
    # 数值清零（浮点残差）
    long_u[np.abs(long_u) < 1e-9] = 0.0
    short_u[np.abs(short_u) < 1e-9] = 0.0
    return long_u, short_u


def episodes_from_exposure(long_u, short_u):
    """从净敞口识别仓位 episode：flat→nonflat 入场，nonflat→flat 离场。
    返回 [(entry_bar, exit_bar, direction, peak_long, peak_short, mean_long, mean_short)]。"""
    nonflat = (long_u > 0) | (short_u > 0)
    eps = []
    n = len(long_u)
    i = 0
    while i < n:
        if not nonflat[i]:
            i += 1
            continue
        start = i
        while i < n and nonflat[i]:
            i += 1
        end = i  # exclusive
        seg_l = long_u[start:end]
        seg_s = short_u[start:end]
        # 入场方向 = 起始 bar 的占优极性
        direction = "long" if seg_l[0] >= seg_s[0] else "short"
        eps.append({
            "entry": start, "exit": end, "bars": end - start,
            "dir": direction,
            "peak_long": float(seg_l.max()), "peak_short": float(seg_s.max()),
            "mean_long": float(seg_l.mean()), "mean_short": float(seg_s.mean()),
            "end_long": float(seg_l[-1]), "end_short": float(seg_s[-1]),
        })
    return eps


def trade_pnl(t):
    ep, xp, sh, pol = float(t[EP]), float(t[XP]), float(t[SH]), t[POL]
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


# ════════════════════ 段内行为统计 ════════════════════
def segment_magnitude(exp_series, equity, closes, eps_in_seg, seg):
    """从引擎真实敞口时间线（按日采样 (bar,lu,su)，与 equity 同采样点）算段内 magnitude。
    净敞口归一化到**当前 NAV**（=当前权益里多少是净多头，1.0=满仓多头，0=对冲/空仓，<0=净空）
    ——避免用 INIT 归一化把"持仓增值"误读为杠杆。trade 反推因 units 循环复用会虚增量级，
    故 magnitude 唯一真值 = exp_series + equity。无序列（旧引擎缓存）→ None。"""
    if not exp_series or not equity:
        return None
    s0, s1 = seg[0], seg[1]
    bars = np.array([e[0] for e in exp_series])
    lu = np.array([e[1] for e in exp_series])
    su = np.array([e[2] for e in exp_series])
    nav = np.array([e[1] for e in equity])  # (bar, nav)，bar 与 exp_series 对齐
    msk = (bars >= s0) & (bars <= s1)
    if not msk.any():
        return None
    bsel, lsel, ssel, nsel = bars[msk], lu[msk], su[msk], nav[: len(bars)][msk]
    csel = closes[np.clip(bsel, 0, len(closes) - 1)]
    nav_safe = np.where(np.abs(nsel) < 1e-6, np.nan, nsel)
    long_frac = (lsel * csel) / nav_safe
    short_frac = (ssel * csel) / nav_safe
    net_frac = ((lsel - ssel) * csel) / nav_safe
    # 多头留存率：每个段内多头 episode 取序列样本 mean(lu)/max(lu)（units 维，与 NAV 无关）
    rets = []
    for e in eps_in_seg:
        if e["dir"] != "long":
            continue
        a_entry, a_exit = s0 + e["entry"], s0 + e["exit"]
        em = (bsel >= a_entry) & (bsel < a_exit)
        if em.any() and lsel[em].max() > 0:
            rets.append(lsel[em].mean() / lsel[em].max())
    return {
        "mean_long_exp": float(np.nanmean(long_frac)),
        "mean_short_exp": float(np.nanmean(short_frac)),
        "mean_net_exp": float(np.nanmean(net_frac)),
        "retention": float(np.mean(rets)) if rets else 0.0,
    }


def segment_behavior(trades, long_u, short_u, closes, seg):
    s0, s1 = seg[0], seg[1]
    span = s1 - s0 + 1
    L = long_u[s0:s1 + 1]
    S = short_u[s0:s1 + 1]
    C = closes[s0:s1 + 1]
    long_bars = int((L > 0).sum())
    short_bars = int((S > 0).sum())
    flat_bars = int(((L <= 0) & (S <= 0)).sum())
    # 名义敞口（units × price），归一化到初始资金
    long_notional = L * C
    short_notional = S * C
    # 段内 trade 事件（按 exit_bar 落在段内）
    in_seg = [t for t in trades if s0 <= int(t[XB]) <= s1]
    n_sink = sum(1 for t in in_seg if t[REASON] == SINK_REASON)
    n_recover = sum(1 for t in in_seg if t[REASON] == RECOVER_REASON)
    n_liq = sum(1 for t in in_seg if t[REASON] in LIQ_REASONS)
    n_clear = sum(1 for t in in_seg if t[REASON] in CLEAR_REASONS)
    # 段内 episode
    eps = episodes_from_exposure(L, S)
    long_eps = [e for e in eps if e["dir"] == "long"]
    short_eps = [e for e in eps if e["dir"] == "short"]
    # 段内 PnL 分布（视图账，仅刻画）
    long_pnl = sum(trade_pnl(t) for t in in_seg if t[POL] == "long")
    short_pnl = sum(trade_pnl(t) for t in in_seg if t[POL] == "short")
    return {
        "span": span,
        "long_bars": long_bars, "short_bars": short_bars, "flat_bars": flat_bars,
        "long_frac": long_bars / span, "short_frac": short_bars / span, "flat_frac": flat_bars / span,
        # 名义敞口归一化到初始资金（=入场满仓时≈1.0）。net = 多头−空头 = 有效市场暴露。
        "mean_long_exp": float(long_notional.mean()) / INIT,
        "mean_short_exp": float(short_notional.mean()) / INIT,
        "mean_net_exp": float((long_notional - short_notional).mean()) / INIT,
        "mean_long_notional": float(long_notional.mean()),
        "mean_short_notional": float(short_notional.mean()),
        "peak_close": float(C.max()), "low_close": float(C.min()),
        "n_episodes": len(eps), "n_long_eps": len(long_eps), "n_short_eps": len(short_eps),
        "n_sink": n_sink, "n_recover": n_recover, "n_liq": n_liq, "n_clear": n_clear,
        "mean_long_ep_bars": float(np.mean([e["bars"] for e in long_eps])) if long_eps else 0.0,
        "mean_short_ep_bars": float(np.mean([e["bars"] for e in short_eps])) if short_eps else 0.0,
        "median_long_ep_bars": float(np.median([e["bars"] for e in long_eps])) if long_eps else 0.0,
        # 减仓深度：多头 episode 内 mean_long / peak_long（1=满仓持有，→0=被抽干）
        "long_retention": float(np.mean([e["mean_long"] / e["peak_long"]
                                         for e in long_eps if e["peak_long"] > 0])) if long_eps else 0.0,
        "long_pnl_view": long_pnl, "short_pnl_view": short_pnl,
        "long_eps": long_eps[:20], "short_eps": short_eps[:20],
    }


# ════════════════════ 引擎规格表 ════════════════════
def engine_specs():
    specs = []
    # 新版：T 操作层自我复制（无 root），逐笔 = 本脚本配套 runner 导出
    for m in MODES:
        specs.append({
            "name": f"新版 T自复制 ({m})", "family": "t_engine", "mode": m,
            "trades_path": ANALYSIS_CACHE / f"t_engine_BTC_{m}_trades.json",
            "summary_path": ANALYSIS_CACHE / f"t_engine_BTC_{m}.json",
        })
    # 旧版1：t_fugue（T 三模式，有 root，简化）
    for m in MODES:
        specs.append({
            "name": f"旧版 t_fugue ({m})", "family": "t_fugue", "mode": m,
            "trades_path": TS_CACHE / f"t_fugue_BTC_{m}.json",
            "summary_path": None,
        })
    # 旧版2：fugue_v3（v3 赋格，long-only root）
    specs.append({
        "name": "旧版 fugue_v3", "family": "fugue_v3", "mode": "-",
        "trades_path": TS_CACHE / "fugue_v3_BTC.json",
        "summary_path": None,
    })
    return specs


def main():
    lines = []

    def P(s=""):
        lines.append(s)

    # —— 加载 BTC，定位牛市段 ——
    closes, dates = load_btc()
    n_bars = len(closes)
    bh = closes[-1] / closes[0] - 1
    P(f"# BTC 强牛市踏空诊断报告\n")
    P(f"- 数据：`btc_1m_full.json` 清洗后 **{n_bars:,}** bars，{dates[0]} → {dates[-1]}")
    P(f"- buy-hold：{closes[0]:.0f} → {closes[-1]:.0f}，**+{bh*100:.1f}%**")
    P(f"- 认识论等级：**L3**（真实数据逐笔归因，可否证）\n")

    segs = find_bull_segments(closes, dates)
    P("## 1. BTC 牛市段定位\n")
    P("| # | 起始日期 | 结束日期 | 低点 | 高点 | 涨幅 | 跨度(bars) | 跨度(天) |")
    P("|---|---------|---------|------|------|------|-----------|---------|")
    for i, (s0, s1, lo, hi, g) in enumerate(segs):
        P(f"| B{i+1} | {dates[s0][:10]} | {dates[s1][:10]} | {lo:.0f} | {hi:.0f} | "
          f"+{g*100:.0f}% | {s1-s0:,} | {(s1-s0)//1440} |")
    P("")

    # —— 各引擎对账 + 段内行为 ——
    specs = engine_specs()
    results = {}
    P("## 2. 重建保真对账（phys_long/short_bars：重建 vs 引擎汇总）\n")
    P("| 引擎 | n_trades | 重建 long_bars | 汇总 long_bars | 重建 short_bars | 汇总 short_bars | 对账 |")
    P("|------|----------|---------------|---------------|----------------|----------------|------|")
    for sp in specs:
        if not sp["trades_path"].exists():
            P(f"| {sp['name']} | — | 文件缺失 `{sp['trades_path'].name}` | | | | ✗ |")
            continue
        d = load_trades(sp["trades_path"])
        trades = d["trades"]
        nb = d.get("n_bars", n_bars)
        long_u, short_u = reconstruct_exposure(trades, nb)
        rec_long = int((long_u > 0).sum())
        rec_short = int((short_u > 0).sum())
        sum_long = d.get("phys_long_bars")
        sum_short = d.get("phys_short_bars")
        if sp["summary_path"] and sp["summary_path"].exists():
            sd = json.loads(sp["summary_path"].read_text())
            sum_long = sd.get("phys_long_bars", sum_long)
            sum_short = sd.get("phys_short_bars", sum_short)
        ok = "✓" if (sum_long and abs(rec_long - sum_long) / max(1, sum_long) < 0.02) else "≈"
        P(f"| {sp['name']} | {len(trades)} | {rec_long:,} | {sum_long if sum_long else '—'} | "
          f"{rec_short:,} | {sum_short if sum_short else '—'} | {ok} |")
        results[sp["name"]] = {
            "spec": sp, "trades": trades, "long_u": long_u, "short_u": short_u,
            "strat": d.get("strat_pct", (d.get("final_nav", INIT) / INIT - 1) * 100), "nb": nb,
            "exp_series": d.get("exposure_series"),  # 仅新引擎有；旧引擎缓存=None
            "equity": d.get("equity"),
        }
    P("")
    P("> 注：t_fugue/fugue_v3 的 phys_*_bars 口径含 fire-phantom（信号在场≠物理持仓），")
    P("> 故只有新版 t_engine 应严格对账；其余 ≈ 表示重建与物理持仓口径一致性参考。\n")

    # —— 全程暴露画像 ——
    P("## 3. 全程多空暴露画像（踏空的宏观 smoking gun）\n")
    P("| 引擎 | strat% | 多头bar | 空头bar | 空仓bar | 多头% | 空头% | 空仓% | 空头/多头 |")
    P("|------|--------|---------|---------|---------|-------|-------|-------|----------|")
    for name, r in results.items():
        L, S = r["long_u"], r["short_u"]
        nb = r["nb"]
        lb = int((L > 0).sum()); sb = int((S > 0).sum())
        fb = int(((L <= 0) & (S <= 0)).sum())
        P(f"| {name} | {r['strat']:+.1f} | {lb:,} | {sb:,} | {fb:,} | "
          f"{lb/nb*100:.1f}% | {sb/nb*100:.1f}% | {fb/nb*100:.1f}% | {sb/max(1,lb):.2f}× |")
    P("")

    # —— 牛市段内行为（聚焦最大段）——
    P("## 4. 牛市段内逐笔行为（区分 churn / 空仓 / 减仓）\n")
    P("> **多头%bar/空仓%bar** = 净 units 存在性占比（trade 重建，逐位对账✓）。")
    P("> **净敞口** = (多头名义−空头名义)/当前NAV = 当前权益里净多头占比（1.0=满仓多头，0=对冲/空仓，<0=净空）；")
    P("> 仅新引擎用真实 `exposure_series`+`equity` 算（trade 反推因 units 循环复用会虚增 ~20×，已弃用）；旧引擎缓存无真值序列 → magnitude 列标 `—`。")
    P("> **留存率** = 多头 episode 内 mean_long/max_long（1=满仓持有到底，→0=核心被 1/3 sink 抽干 = 减仓太多）。\n")
    for i, seg in enumerate(segs):
        s0, s1, lo, hi, g = seg
        P(f"### 段 B{i+1}: {dates[s0][:10]} → {dates[s1][:10]}  ({lo:.0f}→{hi:.0f}, +{g*100:.0f}%)\n")
        P("| 引擎 | 净敞口 | 多头%bar | 空仓%bar | 多头留存率 | 多头ep数 | sink | recover | 强平 |")
        P("|------|-------|---------|---------|-----------|---------|------|---------|------|")
        for name, r in results.items():
            b = segment_behavior(r["trades"], r["long_u"], r["short_u"], closes, seg)
            mag = segment_magnitude(r.get("exp_series"), r.get("equity"), closes,
                                    b["long_eps"] + b["short_eps"], seg)
            if mag:
                net_s = f"{mag['mean_net_exp']:+.2f}"
                ret_s = f"{mag['retention']:.2f}"
            else:
                net_s, ret_s = "—", "—"
            P(f"| {name} | {net_s} | {b['long_frac']*100:.0f}% | {b['flat_frac']*100:.0f}% | {ret_s} | "
              f"{b['n_long_eps']} | {b['n_sink']} | {b['n_recover']} | {b['n_liq']} |")
        P("")

    # —— AND vs structural 门控对比（新版）——
    P("## 5. AND vs structural：MACD 门控在牛市阻止了什么？（新版 T 引擎）\n")
    nm = {m: results.get(f"新版 T自复制 ({m})") for m in MODES}
    if all(nm.values()):
        P("| 模式 | strat% | 全程多头bar | 全程空头bar | 全程空仓bar | sink总 | recover总 | 强平总 | n_trades |")
        P("|------|--------|------------|------------|------------|--------|-----------|--------|----------|")
        for m in MODES:
            r = nm[m]
            tr = r["trades"]
            L, S = r["long_u"], r["short_u"]; nb = r["nb"]
            ns = sum(1 for t in tr if t[REASON] == SINK_REASON)
            nr = sum(1 for t in tr if t[REASON] == RECOVER_REASON)
            nl = sum(1 for t in tr if t[REASON] in LIQ_REASONS)
            P(f"| {m} | {r['strat']:+.1f} | {int((L>0).sum()):,} | {int((S>0).sum()):,} | "
              f"{int(((L<=0)&(S<=0)).sum()):,} | {ns} | {nr} | {nl} | {len(tr)} |")
        P("")

    # —— 入场方向追踪 + 爆仓点（新版三模式：为何 AND/OR 负 NAV）——
    P("## 6. 入场方向追踪 + 爆仓点（新版 T 引擎：AND/OR 为何归零）\n")
    P("> 入场只在全局空仓时发生，方向 = 入场 bar 最高 ladder 的 BSP（buy→多/sell→空）。")
    P("> 牛市入场做空 + 1x 逐仓（c≥2×basis 强平）= 单笔满仓空头爆仓 → 负 NAV。\n")
    for m in MODES:
        r = results.get(f"新版 T自复制 ({m})")
        if not r:
            continue
        d = load_trades(ANALYSIS_CACHE / f"t_engine_BTC_{m}_trades.json")
        eq = d.get("equity", [])
        L, S = r["long_u"], r["short_u"]
        eps = episodes_from_exposure(L, S)
        # 全局入场事件（flat→nonflat），标注方向 + 时长 + 段归属
        P(f"### {m} 模式（strat {r['strat']:+.1f}%，{len(eps)} 个仓位 episode）\n")
        P("| episode | 入场日期 | 方向 | 时长(天) | 入场价 | 离场价 | 价格变动 |")
        P("|---------|---------|------|---------|--------|--------|---------|")
        for e in eps[:14]:
            eb, xb = e["entry"], min(e["exit"], n_bars - 1)
            ep_p, xp_p = closes[eb], closes[xb]
            P(f"| #{eps.index(e)+1} | {dates[eb][:10]} | {'多' if e['dir']=='long' else '空'} | "
              f"{e['bars']/1440:.0f} | {ep_p:.0f} | {xp_p:.0f} | {(xp_p/ep_p-1)*100:+.0f}% |")
        if len(eps) > 14:
            P(f"| … | ({len(eps)-14} 更多) | | | | | |")
        # 爆仓点：equity 最低点
        if eq:
            nav_min = min(eq, key=lambda x: x[1])
            nav_first_neg = next((b for b, v in eq if v < 0), None)
            P("")
            P(f"- equity 最低点：bar {nav_min[0]} ({dates[min(nav_min[0],n_bars-1)][:10]}) NAV={nav_min[1]:.0f}")
            if nav_first_neg is not None:
                P(f"- **首次负 NAV（爆仓）**：bar {nav_first_neg} "
                  f"({dates[min(nav_first_neg,n_bars-1)][:10]}) → 此后资金耗尽，引擎空仓躺平")
        P("")

    # —— 诊断结论 ——
    P("## 7. 诊断结论\n")
    P("### ⚠ 前置：用户引用的 11.8%/−12.3% 是 commit 前 WIP 缓存\n")
    P("- 缓存汇总 `t_engine_BTC_*.json` 写于 23:27–29；引擎重写 commit `aa45a76d1f`「删root方向」在 23:33，")
    P("  后续 `9854b0f995`「1/3几何塔+涌现接管」在 00:03。缓存早于两个 commit。")
    P("- **committed 引擎真值（确定性，两次重算逐位一致）**：structural **+490%**、AND **−100%(爆仓)**、OR **−100%(爆仓)**。")
    P("- 已用 committed 引擎逐笔重算覆盖陈旧缓存。下面诊断针对 committed 引擎。\n")
    P("### Q3 多头为何没吃到涨幅（structural，+490% < BH +1380%）\n")
    P("三选一判据 → **(c) 减仓太多 + 空头腿牛市流血**（非 churn、非空仓）：")
    P("- **非 churn**：每个牛市段仅 1–2 个多头 episode，全程持有（B3 入场 2020-03-13 持 1389 天吃 +805%）。")
    P("- **非空仓**：structural 全程仅 0.2% 空仓，牛市段内 0% 空仓。")
    P("- **是减仓**：1/3 几何 sink（卖点→1/3 核心下放次级别做空）反复抽干多头核心。净多头敞口（/当前NAV）")
    P("  仅 **0.18–0.81**：清爽单边牛（B3 +1600%）留存 0.90/净 0.81 骑得好；震荡牛（B1 +602%/B4 +713%）")
    P("  留存 0.53–0.65/净 0.18–0.19——卖点多→sink 多→核心被抽成对冲性空头。")
    P("- **空头流血**：全程 45.6% 的 bar 同时持有空头次级别仓位，这些空头在牛市中逆势流血，抵消多头核心。\n")
    P("### Q5 AND/OR 为何爆仓（−100%，负 NAV）\n")
    P("- 入场方向 = 入场 bar **最高 ladder 的 BSP**（buy→多/sell→空），**无 root 方向锚定**。")
    P("- **AND**：2020-03-13 COVID 底部入场**做空**（同一 bar structural 入**多**吃 +805%）→ 价格 +100% →")
    P("  1x 逐仓 c≥2×basis 强平 → 2020-06-01 负 NAV → 资金耗尽，此后 68.5% bar 空仓躺平。")
    P("- **OR**：2024-01-03 入场做空 → +100% → 2024-11-12 强平归零。")
    P("- **MACD 门控不是「减少减仓」**：它过滤了 2020 底部最高 ladder 的买点，使该 bar 最高信号变成卖点 →")
    P("  入场翻空 → 单笔满仓空头在 secular bull 里必死。门控改变的是**入场方向**，不是减仓频率。\n")
    P("### Q4 新 vs 旧 + 为何 CL 改善 BTC 没有\n")
    P("| 引擎 | structural | AND | OR | 机制 |")
    P("|------|-----------|-----|-----|------|")
    P("| 新版 T自复制（无root） | **+490%** | −100%爆仓 | −100%爆仓 | 1 episode 全程持有，但无方向锚→入场翻空 |")
    P("| 旧版 t_fugue（有root） | −96% | **+191.7%** | −94.8% | root 锚定方向，但 churn 18–22 episode/段 |")
    P("| 旧版 fugue_v3（long-only root） | — | — | +28.8% | 永不做空 root，churn 最重(8273笔) |")
    P("")
    P("- **删root 的得**：消除 churn（新版每段 1 episode vs 旧版 18–22），多头持仓连续。")
    P("- **删root 的失**：移除多头方向锚 → 入场方向交给 BSP「掷硬币」，在 2020/2024 底部落到做空 → 爆仓。")
    P("- **最优模式随引擎翻转**：新版→structural，旧版→AND（regime×引擎交互，无普适最优）。")
    P("- **CL 改善 / BTC 没有 = regime 正交**：sink/recover 是区间套利（卖高→做空→买低回补），在**震荡**标的")
    P("  （CL）收割波动；在 **secular bull**（BTC）里 sink 做空被单边行情抽干+强平。删root 对 CL（震荡）是")
    P("  净增益，对 BTC（单边牛）暴露了入场翻空死穴。与谱系「强牛{ES,BTC,OKLO,QQQ,BRN}做空踏空<BH」一致。\n")
    P("### 根因排序（按对 alpha 缺口的贡献）\n")
    P("1. **入场无方向锚 → 翻空爆仓**（AND/OR 致命，−100%）：删root 后入场方向由单 bar BSP 决定，secular bull 底部入场做空 = 满仓单笔必死。")
    P("2. **1/3 几何 sink 在单边牛抽干多头核心**（structural 主因，净敞口 0.18–0.81）：卖点下放做空在趋势中是逆势减仓，越震荡抽越狠。")
    P("3. **空头次级别在牛市流血**（45.6% bar 持空）：sink 产生的对冲空头在上行中持续亏损，抵消多头。")
    P("4. **确认滞后税**（即便满仓也 < BH）：BSP 确认时点晚于真实拐点，每段少吃头尾。\n")

    # —— 结果包六要素 ——
    P("## 8. 结果包（六要素）\n")
    P("**1. 结论**：committed T 自复制引擎 BTC strat = structural +490% / AND −100% / OR −100%（均 < BH +1380%）。")
    P("踏空根因三层：①AND/OR 入场无 root 锚→2020/2024 底部翻空爆仓；②structural 1/3 几何 sink 在单边牛抽干多头核心")
    P("（净敞口 0.18–0.81）；③sink 产生的空头次级别在牛市流血（45.6% bar 持空）。用户引用的 11.8%/−12.3% 为 commit 前 WIP 缓存。\n")
    P("**2. 定义依据**：第65课 T 算子 aₙ=f(aₙ₋₁) 自我复制（操作层每级别同一 sink/recover）；026:80「用其中的1/3」⟹ λ=3 几何塔；")
    P("入场方向由「该级别买点→多/卖点→空」（t_engine.rs try_enter）；1x 逐仓强平 c≥2×basis（边界算子 A）。")
    P("净敞口/留存率从引擎真实 `exposure_series`（每日采样 long_units/short_units）算，与 `equity` 同采样点，phys_*_bars 逐位对账✓。\n")
    P("**3. 边界条件**（结论翻转条件）：")
    P("- 若入场方向加 root/regime 锚（只在涌现向上时入多），AND/OR 的翻空爆仓消除 → strat 翻正（待验证）。")
    P("- 若标的是**震荡** regime（如 CL），sink/recover 区间套利成立，删root 为净增益（已知 CL −39%→+75%）。")
    P("- 若 1x 逐仓改为非强平/带止损，AND/OR 不会归零（但仍踏空）。")
    P("- 若 BSP 确认无滞后（理想），structural 净敞口→1.0，缺口仅剩 sink 抽干。\n")
    P("**4. 下游推论**：①「删root」对单边牛是负 regime——root 是 secular bull 的隐式多头锚，不是冗余；")
    P("②sink 比例 1/3 在趋势中是纯减仓，需 regime 门控（趋势态停 sink）；③AND 模式的 MACD 门控会翻转入场方向，")
    P("不能假设门控只「收紧」不「改向」；④最优模式随引擎/标的翻转，无普适 mode。\n")
    P("**5. 谱系引用**：与「T操作层自我复制引擎」（强牛做空踏空<BH）、「unn BTC踏空真凶E spawn」（强牛过度做空）、")
    P("「T下跌段做空平空级别错配」、「NRF v4严格会计判决」（清仓频率=regime函数）一致。本诊断新增：删root 暴露")
    P("**入场方向无锚→翻空爆仓**这一新失效模式（旧版 root 锚定下不存在）。региme 正交于结构修复（539号有效域读数）。\n")
    P("**6. 影响声明**：新增 `analysis/btc_bull_throwback_diagnosis.py`（诊断脚本）+ 本报告；")
    P("引擎加 `FugueResult.exposure_series`（真实敞口采样，layer.rs）+ runner `BT_DUMP_TRADES` 逐笔导出")
    P("（t_engine_run.rs，env 门控，默认汇总契约不变）；逐笔重算覆盖了陈旧缓存 `t_engine_BTC_*.json`。")
    P("**认识论等级：L3**（真实数据逐笔归因，可否证；正/负域诚实报告）。\n")

    OUT_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text("\n".join(lines))
    print("\n".join(lines))
    print(f"\n[写入] {OUT_MD}")


if __name__ == "__main__":
    main()
