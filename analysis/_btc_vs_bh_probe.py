"""调研探针：BTC 为什么不能超过 BH？——scco 后剩余缺口的结构分解（纯调研）。

对 V2oa25_ht_scco（当前最优，+630% vs BH +1380%）做四项分解：
1. 逐 gap（master 出场→再入场）损失：log return、gap 内峰值、时长、年份；
   top-N 最大踏空 gap；踏空集中度（top 10% gap 占总踏空比例）。
2. sell1 事后验证：出场后 +1h/+1d/+3d/+7d 前向收益分布——出场时趋势
   是否真的结束（统计口径）。
3. 分年（牛/熊/震荡 regime）分解：每自然年 BH log、持仓期 BH log、
   踏空/避跌 log、策略 in-trade alpha（按出场年归属）。
4. 反事实上限：若免除 top-K 踏空 gap（即那 K 次不出场），策略总收益。

用法：PYTHONPATH=src .venv/bin/python analysis/_btc_vs_bh_probe.py
输出：analysis/data_cache/_btc_vs_bh_probe.json
"""

from __future__ import annotations

import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
VARIANT = "V2oa25_ht_scco"
# 1min bar 口径的前向窗口（BTC 24/7 无停盘）
HORIZONS = {"1h": 60, "1d": 1440, "3d": 4320, "7d": 10080}


def _q(xs: list, p: float):
    if not xs:
        return None
    s = sorted(xs)
    return round(s[min(len(s) - 1, int(p * (len(s) - 1)))], 4)


def main() -> None:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["BTC"])
    n = len(closes)
    bh = closes[-1] / closes[0]
    print(f"bars={n:,} BH={(bh - 1) * 100:+.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)
    # 磁带指纹守卫：与在册 scco 缓存对账
    ref_fp = json.loads(
        (DATA_DIR / "osc_scco_BTC.json").read_text())["tape_fp"]
    if ref_fp != fp:
        raise RuntimeError(f"tape_fp 漂移：{fp} ≠ 在册 {ref_fp}，停。")
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    res = nr.run_organic_rust(rtape, VARIANT, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    trades = res["trades"]
    counters = res["counters"]
    print(f"trades={len(trades)} n_exit_trend_holds="
          f"{counters.get('n_exit_trend_holds')}", flush=True)

    # ── 1. 逐 gap 分解 ──
    gaps = []  # (exit_bar, reentry_bar, gap_logret, peak_ratio, year, reason)
    prev_end, prev_price, prev_reason = 0, closes[0], "start"
    for t in trades:
        eb, xb, xp, reason = t[0], t[2], t[3], t[5]
        if eb > prev_end:
            logret = math.log(closes[eb] / closes[prev_end])
            seg_hi = max(highs[prev_end:eb + 1])
            gaps.append({
                "exit_bar": prev_end, "reentry_bar": eb,
                "dur_bars": eb - prev_end,
                "gap_logret": round(logret, 5),
                "peak_ratio": round(seg_hi / closes[prev_end], 5),
                "year": years[prev_end], "exit_reason": prev_reason,
            })
        prev_end, prev_price, prev_reason = xb, xp, reason
    if prev_end < n - 1:
        gaps.append({
            "exit_bar": prev_end, "reentry_bar": n - 1,
            "dur_bars": n - 1 - prev_end,
            "gap_logret": round(math.log(closes[-1] / closes[prev_end]), 5),
            "peak_ratio": round(max(highs[prev_end:]) / closes[prev_end], 5),
            "year": years[prev_end], "exit_reason": prev_reason,
        })
    missed = sorted([g for g in gaps if g["gap_logret"] > 0],
                    key=lambda g: -g["gap_logret"])
    total_missed = sum(g["gap_logret"] for g in missed)
    total_avoided = sum(g["gap_logret"] for g in gaps if g["gap_logret"] < 0)
    k10 = max(1, len(missed) // 10)
    conc_top10pct = sum(g["gap_logret"] for g in missed[:k10]) / total_missed

    # ── 2. sell1 事后验证（出场后前向收益）──
    forward = {h: [] for h in HORIZONS}
    for t in trades:
        xb, xp = t[2], t[3]
        for name, bars in HORIZONS.items():
            j = min(n - 1, xb + bars)
            forward[name].append(closes[j] / xp)
    fwd_stats = {
        name: {
            "p25": _q(v, .25), "p50": _q(v, .5), "p75": _q(v, .75),
            "frac_above_1": round(sum(1 for x in v if x > 1) / len(v), 4),
            "mean_log": round(sum(math.log(x) for x in v) / len(v), 5),
        } for name, v in forward.items()}

    # ── 3. 分年 regime 分解（bar 级持仓 mask，精确归属）──
    in_pos = bytearray(n)
    for t in trades:
        for b in range(t[0], t[2]):
            in_pos[b] = 1
    yearly: dict[int, dict] = {}
    for b in range(n - 1):
        y = years[b]
        r = math.log(closes[b + 1] / closes[b])
        row = yearly.setdefault(y, {
            "bh_log": 0.0, "in_trade_bh_log": 0.0,
            "gap_missed": 0.0, "gap_avoided": 0.0,
            "bars": 0, "pos_bars": 0})
        row["bh_log"] += r
        row["bars"] += 1
        if in_pos[b]:
            row["in_trade_bh_log"] += r
            row["pos_bars"] += 1
        elif r > 0:
            row["gap_missed"] += r
        else:
            row["gap_avoided"] += r
    # 策略每笔 ln(1+pnl) 按出场年归属（近似，跨年长持仓笔注记）
    for t in trades:
        y = years[t[2]]
        yearly[y].setdefault("strat_log", 0.0)
        yearly[y]["strat_log"] += math.log(1 + t[4] / 100.0)
        yearly[y]["n_trades"] = yearly[y].get("n_trades", 0) + 1
        yearly[y].setdefault("n_ht_exits", 0)
        if "trendexh" in t[5]:
            yearly[y]["n_ht_exits"] += 1
    for y, row in yearly.items():
        bh_y = row["bh_log"]
        row["regime"] = ("bull" if bh_y > 0.4
                         else "bear" if bh_y < -0.2 else "range")
        for k in ("bh_log", "in_trade_bh_log", "gap_missed", "gap_avoided"):
            row[k] = round(row[k], 4)
        row["strat_log"] = round(row.get("strat_log", 0.0), 4)
        row["exposure"] = round(row["pos_bars"] / row["bars"], 4)

    # ── 4. 反事实：免除 top-K 踏空 gap（那 K 次不出场，按 BH 持有）──
    ln_strat = sum(math.log(1 + t[4] / 100.0) for t in trades)
    counterfactual = {}
    for K in (5, 10, 20, 50, len(missed)):
        add = sum(g["gap_logret"] for g in missed[:K])
        counterfactual[f"top{K}"] = round(
            (math.exp(ln_strat + add) - 1) * 100, 1)

    # ── 5. 出场原因分布 × 后续 gap 表现 ──
    by_reason: dict[str, dict] = {}
    for g in gaps:
        r = by_reason.setdefault(g["exit_reason"],
                                 {"n": 0, "missed": 0.0, "avoided": 0.0})
        r["n"] += 1
        if g["gap_logret"] > 0:
            r["missed"] += g["gap_logret"]
        else:
            r["avoided"] += g["gap_logret"]
    for r in by_reason.values():
        r["missed"] = round(r["missed"], 3)
        r["avoided"] = round(r["avoided"], 3)

    out = {
        "variant": VARIANT, "n_bars": n, "tape_fp": fp,
        "bh_pct": round((bh - 1) * 100, 1),
        "strat_pct": round((math.exp(ln_strat) - 1) * 100, 1),
        "n_trades": len(trades),
        "counters": {k: counters[k] for k in counters
                     if "exit" in k or "osc" in k or "master" in k},
        "gap_summary": {
            "n_gaps": len(gaps), "n_missed": len(missed),
            "total_missed_nats": round(total_missed, 4),
            "total_avoided_nats": round(total_avoided, 4),
            "concentration_top10pct_gaps": round(conc_top10pct, 4),
            "gap_dur_p50": _q([g["dur_bars"] for g in gaps], .5),
            "peak_ratio_p50": _q([g["peak_ratio"] for g in gaps], .5),
            "peak_ratio_p90": _q([g["peak_ratio"] for g in gaps], .9),
            "frac_peak_gt_2pct": round(
                sum(1 for g in gaps if g["peak_ratio"] > 1.02) / len(gaps), 4),
            "frac_peak_gt_5pct": round(
                sum(1 for g in gaps if g["peak_ratio"] > 1.05) / len(gaps), 4),
        },
        "top20_missed_gaps": missed[:20],
        "forward_after_exit": fwd_stats,
        "yearly": {str(y): yearly[y] for y in sorted(yearly)},
        "counterfactual_no_topK_missed": counterfactual,
        "exit_reason_gaps": by_reason,
    }
    out_path = DATA_DIR / "_btc_vs_bh_probe.json"
    out_path.write_text(json.dumps(out, indent=1, ensure_ascii=False))
    print(json.dumps(out["gap_summary"], indent=1), flush=True)
    print(json.dumps(out["counterfactual_no_topK_missed"], indent=1),
          flush=True)
    print(f"[done] {out_path.name}", flush=True)


if __name__ == "__main__":
    main()
