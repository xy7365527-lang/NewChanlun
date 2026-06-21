"""BTC 下跌段空头平仓生命周期逐笔诊断。

═══════════════ 调查的核心问题 ═══════════════

用户预设"空头没有平/持有到底没平"。但 L3 数据显示空头恰恰是**早平**。
本脚本逐笔分析 BTC structural / or 模式下跌段 [top_bar, bot_bar] 内开仓的空头，
追踪它们被**什么 exit_reason、在什么价位、相对底部多远**平掉，回答：
  - 平空是短差(reduce/recover)还是核心清(core_clear_short)主导？
  - 短差类的平仓价是不是远高于底部(=反弹就平)？核心清类是不是才接近底部？

═══════════════ 坐标系（与 t_fugue_downtrend_short_trace.load_ohlc_with_dates 逐字一致）═══════════════

  entry_bar/exit_bar = load_ohlc 清洗后索引。清洗：
    1. 删任一 OHLC 为 nan 或 ≤0 的 bar；
    2. spike-and-revert: close 跳变>50% 且后 bar 回前 bar±5% 内的 bar 删除。
  assert 清洗后 len == JSON n_bars（BTC 实测无删行 cleaned==4625119）。

  锁定锚点（high/low 极值日期窗口定位，独立于交易）:
    BTC top_bar=2218375 (2021-11-10, high 69000), bot_bar=2760271 (2022-11-21, low 15476)。

  trade11 schema:
    [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
     weight_at_entry, deferred_bars, partial, exit_reason, polarity]
    short pnl = shares*(entry_price - exit_price)；NAV% = pnl/100000*100。

认识论等级：L3（真实数据逐笔归因，可否证）。

用法（仓库根目录）:
    python3 analysis/_short_close_lifecycle.py
"""

from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = REPO_ROOT / "analysis" / "data_cache"
FUGUE_DIR = REPO_ROOT / "trading_system" / "data_cache"
INITIAL_CAPITAL = 100_000.0

# 锁定锚点（来自任务，已独立定位）。
BTC_TOP_BAR = 2218375
BTC_BOT_BAR = 2760271
BTC_BOT_PX = 15476.0


def load_ohlc_with_dates(path: Path):
    """复刻 load_ohlc 清洗逻辑（逐字与 t_fugue_downtrend_short_trace 一致），保留 dates。"""
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw.get("dates")
    if d_in is None:
        raise ValueError(f"{path.name} 无 dates")
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


def analyze_mode(mode: str, dates, top_bar: int, bot_bar: int, bot_px: float):
    """返回 (mode 结果 dict, 段内空头逐笔 list)。"""
    p = FUGUE_DIR / f"t_fugue_BTC_{mode}.json"
    d = json.loads(p.read_text())
    n_bars = d["n_bars"]
    assert len(dates) == n_bars, f"{mode}: cleaned {len(dates)} != n_bars {n_bars}"

    trades = d["trades"]
    seg_len = bot_bar - top_bar  # 段长（bar）

    # 段内开仓空头（entry_bar∈[top_bar,bot_bar]）。
    seg_shorts = []
    for t in trades:
        ladder, eb, ep, xb, xp, sh, w, defb, partial, reason, pol = t
        if pol != "short":
            continue
        if not (top_bar <= eb <= bot_bar):
            continue
        pnl = sh * (ep - xp)
        seg_shorts.append(dict(
            ladder=ladder, entry_bar=eb, entry_px=ep, exit_bar=xb, exit_px=xp,
            shares=sh, reason=reason, pnl=pnl,
            pnl_nav=pnl / INITIAL_CAPITAL * 100.0,
            dist_to_bot_pct=(xp / bot_px - 1.0) * 100.0,
            exit_seg_pos_pct=(xb - top_bar) / seg_len * 100.0 if seg_len else 0.0,
        ))

    n_short = len(seg_shorts)

    # 1. 按 exit_reason 分组。
    by_reason = []
    grp = defaultdict(list)
    for s in seg_shorts:
        grp[s["reason"]].append(s)
    for reason in sorted(grp):
        g = grp[reason]
        n = len(g)
        avg_exit = sum(s["exit_px"] for s in g) / n
        avg_dist = (avg_exit / bot_px - 1.0) * 100.0
        avg_seg_pos = sum(s["exit_seg_pos_pct"] for s in g) / n
        pnl_nav = sum(s["pnl_nav"] for s in g)
        by_reason.append(dict(
            reason=reason, n=n, avg_exit_px=round(avg_exit, 2),
            avg_dist_to_bottom_pct=round(avg_dist, 2),
            avg_exit_seg_pos_pct=round(avg_seg_pos, 2),
            pnl_nav=round(pnl_nav, 2),
        ))

    # 2. 平仓价相对底部分桶。
    b_lo, b_hi = bot_px * 0.9, bot_px * 1.1   # 底部±10%
    bucket = Counter()
    for s in seg_shorts:
        xp = s["exit_px"]
        ratio = xp / bot_px - 1.0  # >0 = 高于底部
        if b_lo <= xp <= b_hi:
            bucket["底±10%"] += 1
        elif 0.10 < ratio <= 0.30 or -0.30 <= ratio < -0.10:
            bucket["10-30%"] += 1
        elif 0.30 < ratio <= 0.60 or -0.60 <= ratio < -0.30:
            bucket["30-60%"] += 1
        else:  # |ratio|>60%
            bucket[">60%"] += 1
    exit_buckets = f"底±10%={bucket['底±10%']} / 10-30%={bucket['10-30%']} / 30-60%={bucket['30-60%']} / >60%={bucket['>60%']}"

    # 3. 接近底部 / 高位早平计数。
    n_below_20k = sum(1 for s in seg_shorts if s["exit_px"] < 20000)
    n_above_30k = sum(1 for s in seg_shorts if s["exit_px"] > 30000)

    # 4. exit_bar 时间分布（10 等分桶）。
    seg_len_safe = seg_len if seg_len else 1
    time_buckets = Counter()
    for s in seg_shorts:
        # exit_bar 可能超出 bot_bar（扛过底部）；按相对段位置分桶，>100% 归末桶。
        pos = (s["exit_bar"] - top_bar) / seg_len_safe
        idx = min(9, max(0, int(pos * 10))) if pos <= 1.0 else 10  # 10 = 超出段尾
        time_buckets[idx] += 1
    time_dist = {f"{i*10}-{(i+1)*10}%": time_buckets.get(i, 0) for i in range(10)}
    time_dist[">100%(扛过底)"] = time_buckets.get(10, 0)

    # 5. 短差(reduce/recover) vs 核心清(core_clear_short[+core_clear]) 分组。
    short_diff = [s for s in seg_shorts if s["reason"] in ("reduce", "recover")]
    core_clear = [s for s in seg_shorts if s["reason"] in ("core_clear_short", "core_clear")]

    def grp_stat(g):
        if not g:
            return dict(n=0, avg_exit_px=None, avg_dist_to_bottom_pct=None,
                        avg_seg_pos_pct=None, pnl_nav=0.0)
        n = len(g)
        avg_exit = sum(s["exit_px"] for s in g) / n
        return dict(
            n=n,
            avg_exit_px=round(avg_exit, 2),
            avg_dist_to_bottom_pct=round((avg_exit / bot_px - 1.0) * 100.0, 2),
            avg_seg_pos_pct=round(sum(s["exit_seg_pos_pct"] for s in g) / n, 2),
            pnl_nav=round(sum(s["pnl_nav"] for s in g), 2),
        )

    short_diff_stat = grp_stat(short_diff)
    core_clear_stat = grp_stat(core_clear)

    result = dict(
        mode=mode,
        n_short=n_short,
        seg_len_bars=seg_len,
        by_exit_reason=by_reason,
        exit_px_buckets=exit_buckets,
        n_exit_near_bottom_below_20k=n_below_20k,
        n_exit_high_above_30k=n_above_30k,
        time_distribution=time_dist,
        short_diff=short_diff_stat,       # reduce+recover
        core_clear=core_clear_stat,       # core_clear_short+core_clear
    )
    return result, seg_shorts


def main():
    raw_path = DATA_DIR / "btc_1m_full.json"
    o, h, l, c, dates = load_ohlc_with_dates(raw_path)
    print(f"[cleaned] BTC bars = {len(dates)}")
    print(f"[anchor] top_bar={BTC_TOP_BAR} ({dates[BTC_TOP_BAR][:10]} high={h[BTC_TOP_BAR]:.0f}) "
          f"bot_bar={BTC_BOT_BAR} ({dates[BTC_BOT_BAR][:10]} low={l[BTC_BOT_BAR]:.0f})")
    print(f"[anchor] seg_len = {BTC_BOT_BAR - BTC_TOP_BAR} bars, "
          f"drop {(h[BTC_TOP_BAR]/l[BTC_BOT_BAR]-1)*-100:.1f}% (top high -> bot low)")
    print(f"[anchor] bot_px (locked) = {BTC_BOT_PX}, actual low[bot_bar] = {l[BTC_BOT_BAR]:.2f}")
    print()

    out = {}
    for mode in ("structural", "or", "and"):
        res, seg_shorts = analyze_mode(mode, dates, BTC_TOP_BAR, BTC_BOT_BAR, BTC_BOT_PX)
        out[mode] = res
        print("=" * 70)
        print(f"MODE = {mode}   段内开仓空头 n_short = {res['n_short']}")
        print("-" * 70)
        print("[1] 按 exit_reason 分组:")
        for r in res["by_exit_reason"]:
            print(f"    {r['reason']:18s} n={r['n']:4d}  avg_exit={r['avg_exit_px']:>10.2f}  "
                  f"dist_to_bot={r['avg_dist_to_bottom_pct']:>8.2f}%  "
                  f"seg_pos={r['avg_exit_seg_pos_pct']:>6.2f}%  pnl_nav={r['pnl_nav']:>9.2f}%")
        print(f"[2] 平仓价相对底部分桶: {res['exit_px_buckets']}")
        print(f"[3] exit<20000 (近底): {res['n_exit_near_bottom_below_20k']}笔  |  "
              f"exit>30000 (高位早平): {res['n_exit_high_above_30k']}笔")
        print(f"[4] exit_bar 时间分布(10等分): {res['time_distribution']}")
        print(f"[5] 短差类(reduce+recover): {res['short_diff']}")
        print(f"    核心清类(core_clear_short+core_clear): {res['core_clear']}")
        print()

    # JSON 落盘供下游消费。
    out_path = REPO_ROOT / "analysis" / "_short_close_lifecycle_out.json"
    out_path.write_text(json.dumps(out, ensure_ascii=False, indent=2))
    print(f"[saved] {out_path}")


if __name__ == "__main__":
    main()
