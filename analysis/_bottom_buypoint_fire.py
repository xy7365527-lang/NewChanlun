"""底部买点信号 fire 追踪 —— 回答用户精确二分。

═══════════════ 用户问题（核心二分）═══════════════

"为什么空头没有平？在下跌段做空后，到底是什么阻止了引擎在底部附近平空？
 是没有买点信号 fire？还是买点 fire 了但引擎没执行平空？"

⚠ 重要张力：用户预设"空头没平/扛到底没平"，但 L3 数据显示空头恰恰是**早平**
（BTC structural 168 笔空头，早平 161/168，平在底部±15%内仅 1/168）。
所以真正要查的是"平空的触发机制"——空头被什么信号、在什么价位平掉的，
为什么不能持有到底部大买点。

本脚本回答二分：
  (A) 没有买点信号 fire（底部窗口里 reduce/recover/core_clear_short 计数 = 0）
  (B) fire 了但已无空头可平（底部窗口里有 fire，但 polarity=short 的 exit 很少/为 0
      —— 因为空头在到底部前就被早平光了）

═══════════════ 数据源（严格）═══════════════

  trading_system/data_cache/t_fugue_BTC_{structural,and,or}.json（多空双向）
  trade11 schema:
    [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
     weight_at_entry, deferred_bars, partial, exit_reason, polarity]
  short pnl = shares*(entry_price - exit_price)；NAV贡献% = pnl/100000*100

═══════════════ 坐标系（关键，否则日期错位=声明膨胀）═══════════════

  entry_bar/exit_bar = load_ohlc 清洗后索引。复刻清洗逻辑 + 携带 dates，
  assert 清洗后 len == JSON n_bars，保证 dates[bar] 是真实日期。
  锚点已锁定（high/low 极值，独立于交易）：
    BTC: top_bar=2218375 (2021-11-10, high 69000), bot_bar=2760271 (2022-11-21, low 15476)
    ES : top_bar=4026006 (2022-01-04, high 4808), bot_bar=4301928 (2022-10-13, low 3502)

认识论等级：L3（真实数据逐笔归因，可否证）。

用法（仓库根目录）:
    python3 analysis/_bottom_buypoint_fire.py
"""

from __future__ import annotations

import json
import math
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = REPO_ROOT / "analysis" / "data_cache"
FUGUE_DIR = REPO_ROOT / "trading_system" / "data_cache"
INITIAL_CAPITAL = 100_000.0
MODES = ("structural", "and", "or")
WINDOW_BARS = 43_200  # ±30 天 = 30*1440

RAW_FILES = {
    "BTC": DATA_DIR / "btc_1m_full.json",
    "ES": DATA_DIR / "es_1m_databento_10y.json",
}

# 锚点已锁定（任务给定，high/low 极值，独立于交易）。
ANCHORS = {
    "BTC": dict(top_bar=2_218_375, bot_bar=2_760_271, bot_px=15_476.0),
    "ES": dict(top_bar=4_026_006, bot_bar=4_301_928, bot_px=3_502.0),
}

# 买点类信号（清空头方向）：reduce/recover = 次级别短差买点；core_clear_short = 本级别买点清空头。
BUYPOINT_REASONS = {"reduce", "recover", "core_clear_short"}


# ════════════════════════ 数据加载（复刻 load_ohlc 清洗 + 携带 dates）════════════════════════


def load_ohlc_with_dates(path: Path):
    """复刻 load_ohlc 清洗逻辑并同步保留 dates。返回 (opens, highs, lows, closes, dates)。

    清洗顺序逐字一致：
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


def pct(cash: float) -> float:
    return cash / INITIAL_CAPITAL * 100.0


def trade_pnl(ep: float, xp: float, sh: float, pol: str) -> float:
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


def date_at(dates, bar: int) -> str:
    if 0 <= bar < len(dates):
        return dates[bar]
    return "OOB"


# ════════════════════════ 分析 ════════════════════════


def analyze_btc(dates, closes):
    """BTC: 回答二分问题 1-4。"""
    a = ANCHORS["BTC"]
    top_bar, bot_bar, bot_px = a["top_bar"], a["bot_bar"], a["bot_px"]
    win_lo, win_hi = bot_bar - WINDOW_BARS, bot_bar + WINDOW_BARS

    result = {"top_bar": top_bar, "bot_bar": bot_bar, "bot_px": bot_px,
              "window": [win_lo, win_hi], "modes": {}}

    for mode in ("structural", "or"):
        f = FUGUE_DIR / f"t_fugue_BTC_{mode}.json"
        if not f.exists():
            continue
        trades = json.loads(f.read_text())["trades"]

        # ── Q1: top_bar 之后首个 exit_reason=core_clear_short 的 trade ──
        ccs = [t for t in trades if t[9] == "core_clear_short" and t[1] >= top_bar]
        # 按 exit_bar 排序（"首个清空头事件"应以 exit 时点为准，因为 core_clear_short 是平仓事件）
        ccs_by_exit = sorted(ccs, key=lambda t: t[3])
        first_ccs = ccs_by_exit[0] if ccs_by_exit else None

        # ── Q2: 底部窗口 bot_bar±WINDOW 内各 (exit_reason, polarity) 计数（按 exit_bar 落窗）──
        win_exit = Counter()
        win_buypoint_fire = 0  # 买点类信号在底部 fire 数
        for t in trades:
            xb = t[3]
            if win_lo <= xb <= win_hi:
                win_exit[(t[9], t[10])] += 1
                if t[9] in BUYPOINT_REASONS:
                    win_buypoint_fire += 1

        # ── Q3: 底部窗口内 polarity=short 的 exit（底部时引擎手上还有空头可平吗）──
        win_short_exits = [t for t in trades if win_lo <= t[3] <= win_hi and t[10] == "short"]

        # ── Q4: 空头开仓(entry_bar) vs 平仓(exit_bar) 分布——空头是否中段就平完 ──
        shorts = [t for t in trades if t[10] == "short"]
        # 段内（top..bot）开的空头
        seg_shorts = [t for t in shorts if top_bar <= t[1] <= bot_bar]
        seg_len = bot_bar - top_bar
        # exit_bar 相对底部的位置：早平 = exit < bot - 20%段长
        early_thresh = bot_bar - seg_len * 0.2
        n_seg = len(seg_shorts)
        n_exit_before_early = sum(1 for t in seg_shorts if t[3] < early_thresh)
        n_exit_near_bot = sum(1 for t in seg_shorts if abs(t[4] / bot_px - 1) < 0.15)
        n_exit_after_bot = sum(1 for t in seg_shorts if t[3] > bot_bar)
        # 平均持有占段长
        avg_held_ratio = (sum(t[3] - t[1] for t in seg_shorts) / n_seg / seg_len * 100) if n_seg else 0.0
        # 段内空头的 exit_bar 最大值（最后一笔段内空头平在哪）
        last_seg_exit = max((t[3] for t in seg_shorts), default=None)
        last_seg_exit_px = None
        if last_seg_exit is not None:
            last_seg_exit_px = max(seg_shorts, key=lambda t: t[3])[4]
        # 段内空头 exit_bar 的 10 桶分布（按段长归一化；可超 1.0=扛过底）
        exit_buckets = [0] * 11  # 桶 0-9 在段内，桶 10 = 扛过底
        for t in seg_shorts:
            frac = (t[3] - top_bar) / seg_len if seg_len else 0
            bi = min(10, max(0, int(frac * 10)))
            exit_buckets[bi] += 1

        result["modes"][mode] = {
            "first_core_clear_short": first_ccs,
            "win_exit_counts": dict(win_exit),
            "win_buypoint_fire": win_buypoint_fire,
            "win_short_exits": win_short_exits,
            "n_win_short_exits": len(win_short_exits),
            "n_seg_shorts": n_seg,
            "n_exit_before_early": n_exit_before_early,
            "n_exit_near_bot": n_exit_near_bot,
            "n_exit_after_bot": n_exit_after_bot,
            "avg_held_ratio": avg_held_ratio,
            "early_thresh": early_thresh,
            "last_seg_exit": last_seg_exit,
            "last_seg_exit_px": last_seg_exit_px,
            "exit_buckets": exit_buckets,
            "seg_len": seg_len,
            "n_total_short": len(shorts),
        }
    return result


def analyze_es(dates, closes):
    """ES: 回答二分问题 5——下跌段窗口内有任何 polarity=short 吗？"""
    a = ANCHORS["ES"]
    top_bar, bot_bar = a["top_bar"], a["bot_bar"]
    result = {"top_bar": top_bar, "bot_bar": bot_bar, "modes": {}}
    for mode in MODES:
        f = FUGUE_DIR / f"t_fugue_ES_{mode}.json"
        if not f.exists():
            result["modes"][mode] = {"file_missing": True}
            continue
        trades = json.loads(f.read_text())["trades"]
        # 下跌段窗口 [top, bot]：按 entry_bar 或 exit_bar 任一落段
        seg_short_entry = [t for t in trades if t[10] == "short" and top_bar <= t[1] <= bot_bar]
        seg_short_exit = [t for t in trades if t[10] == "short" and top_bar <= t[3] <= bot_bar]
        seg_short_any = [t for t in trades if t[10] == "short"
                         and (top_bar <= t[1] <= bot_bar or top_bar <= t[3] <= bot_bar)]
        total_short = sum(1 for t in trades if t[10] == "short")
        result["modes"][mode] = {
            "n_seg_short_entry": len(seg_short_entry),
            "n_seg_short_exit": len(seg_short_exit),
            "n_seg_short_any": len(seg_short_any),
            "total_short": total_short,
            "n_trades": len(trades),
        }
    return result


def main() -> int:
    out = []
    w = out.append

    # ── BTC ──
    o, h, l, closes_btc, dates_btc = load_ohlc_with_dates(RAW_FILES["BTC"])
    ref = json.loads((FUGUE_DIR / "t_fugue_BTC_structural.json").read_text())
    assert len(closes_btc) == ref["n_bars"], \
        f"BTC 坐标系错位: cleaned {len(closes_btc)} != JSON n_bars {ref['n_bars']}"
    w(f"[BTC 坐标系 assert PASS] cleaned={len(closes_btc):,} == n_bars={ref['n_bars']:,}")
    a = ANCHORS["BTC"]
    w(f"[BTC 锚点] top_bar={a['top_bar']:,} @ {date_at(dates_btc, a['top_bar'])} "
      f"high={h[a['top_bar']]:,.0f} | bot_bar={a['bot_bar']:,} @ {date_at(dates_btc, a['bot_bar'])} "
      f"low={l[a['bot_bar']]:,.0f} (锚定 bot_px={a['bot_px']:,.0f})")
    w(f"[BTC 底部窗口] bot_bar±{WINDOW_BARS:,}bar(±30天) = "
      f"[{a['bot_bar']-WINDOW_BARS:,} ({date_at(dates_btc, a['bot_bar']-WINDOW_BARS)}), "
      f"{a['bot_bar']+WINDOW_BARS:,} ({date_at(dates_btc, a['bot_bar']+WINDOW_BARS)})]")
    w("")

    btc = analyze_btc(dates_btc, closes_btc)
    for mode in ("structural", "or"):
        m = btc["modes"].get(mode)
        if not m:
            continue
        w(f"════════ BTC / {mode} ════════")
        # Q1
        fc = m["first_core_clear_short"]
        if fc:
            xb, xp = fc[3], fc[4]
            dist = (xp / a["bot_px"] - 1) * 100
            w(f"[Q1] top 后首个 core_clear_short（按 exit_bar）: "
              f"exit_bar={xb:,} @ {date_at(dates_btc, xb)} exit_px={xp:,.0f} "
              f"(距底 {dist:+.1f}%) | entry_bar={fc[1]:,} @ {date_at(dates_btc, fc[1])} "
              f"entry_px={fc[2]:,.0f} ladder=L{fc[0]} polarity={fc[10]}")
            w(f"     → 在底部15476附近? {'是' if abs(xp/a['bot_px']-1)<0.15 else '否（高位早现）'}")
        else:
            w("[Q1] top 后无 core_clear_short")
        # Q2
        w(f"[Q2] 底部窗口内 (exit_reason, polarity) 计数:")
        for k in sorted(m["win_exit_counts"]):
            w(f"       {k}: {m['win_exit_counts'][k]}")
        w(f"     → 买点类信号(reduce/recover/core_clear_short) 在底部 fire: {m['win_buypoint_fire']} 次")
        # Q3
        w(f"[Q3] 底部窗口内 polarity=short 的 exit: {m['n_win_short_exits']} 笔"
          f" → {'底部还有空头可平' if m['n_win_short_exits'] > 0 else '底部已无空头可平（早平光）'}")
        if m["win_short_exits"]:
            for t in m["win_short_exits"][:10]:
                w(f"       short exit: exit_bar={t[3]:,} @ {date_at(dates_btc, t[3])} "
                  f"exit_px={t[4]:,.0f} reason={t[9]} L{t[0]} "
                  f"pnl={pct(trade_pnl(t[2], t[4], t[5], t[10])):+.2f}%")
        # Q4
        w(f"[Q4] 段内空头(top..bot 开仓) {m['n_seg_shorts']} 笔（全程空头 {m['n_total_short']} 笔）:")
        w(f"       平均持有 = {m['avg_held_ratio']:.1f}% 段长")
        w(f"       早平(exit < bot-20%段长, thresh bar={m['early_thresh']:,.0f}): "
          f"{m['n_exit_before_early']}/{m['n_seg_shorts']}")
        w(f"       平在底部±15%价内: {m['n_exit_near_bot']}/{m['n_seg_shorts']}")
        w(f"       扛过底部(exit_bar > bot): {m['n_exit_after_bot']}/{m['n_seg_shorts']}")
        if m["last_seg_exit"] is not None:
            w(f"       段内空头最后平仓 exit_bar={m['last_seg_exit']:,} @ "
              f"{date_at(dates_btc, m['last_seg_exit'])} px={m['last_seg_exit_px']:,.0f} "
              f"(距底 {(m['last_seg_exit_px']/a['bot_px']-1)*100:+.1f}%)")
        w(f"       exit_bar 10桶分布(桶0=顶平,桶9=底前,桶10=扛过底): {m['exit_buckets']}")
        w("")

    # ── ES ──
    o2, h2, l2, closes_es, dates_es = load_ohlc_with_dates(RAW_FILES["ES"])
    ref_es = json.loads((FUGUE_DIR / "t_fugue_ES_structural.json").read_text())
    assert len(closes_es) == ref_es["n_bars"], \
        f"ES 坐标系错位: cleaned {len(closes_es)} != JSON n_bars {ref_es['n_bars']}"
    w(f"[ES 坐标系 assert PASS] cleaned={len(closes_es):,} == n_bars={ref_es['n_bars']:,}")
    ae = ANCHORS["ES"]
    w(f"[ES 锚点] top_bar={ae['top_bar']:,} @ {date_at(dates_es, ae['top_bar'])} "
      f"high={h2[ae['top_bar']]:,.0f} | bot_bar={ae['bot_bar']:,} @ {date_at(dates_es, ae['bot_bar'])} "
      f"low={l2[ae['bot_bar']]:,.0f}")
    w("")
    es = analyze_es(dates_es, closes_es)
    for mode in MODES:
        m = es["modes"].get(mode, {})
        if m.get("file_missing"):
            w(f"════════ ES / {mode} ════════ 文件缺失")
            continue
        w(f"════════ ES / {mode} ════════")
        w(f"[Q5] 下跌段[top,bot]窗口内 polarity=short: "
          f"entry落段={m['n_seg_short_entry']} exit落段={m['n_seg_short_exit']} "
          f"任一落段={m['n_seg_short_any']} | 全程空头={m['total_short']} (n_trades={m['n_trades']})")
        w("")

    text = "\n".join(out)
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
