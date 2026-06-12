"""1s 确认加速的特异性对照 — per-ladder consumed/coverage/precision。

动机（严格性缺口）：主测量中 1s 塔 ladder≥2 全体事件的 consumed 远低于
1min 塔，但 1s 塔事件密度是 1min 塔的 30-50 倍——"回调早期总有事件"
可能是 base rate 效应（事件到处都是，自然覆盖回调早期），不是定位能力。

对照设计：
- precision = sell confirmed 事件落在 zigzag 下跌摆动窗口内的比例；
  基线 = 下跌摆动窗口的时间占比（随机时点命中率）；lift = precision/基线。
- 按 (塔 × ladder) 分层 + 按 "ladder≥L" 组合分层，输出
  coverage / consumed_p50 / precision / lift 的权衡面。
  同等 precision 水平上 consumed 更低 = 真定位能力；
  precision ≈ 基线（lift≈1）= base rate 效应。

用法：PYTHONPATH=src .venv/bin/python analysis/_confirm_accel_specificity.py [BTC|ES]
输出：data_cache/confirm_accel_spec_{sym}.json + 事件落盘复用缓存
"""

from __future__ import annotations

import bisect
import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from confirmation_acceleration_1s import (  # noqa: E402
    aggregate_1min,
    load_btc,
    load_es,
    zigzag_swings,
)
from organic_signals import compute_organic_signals  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
THRESHOLDS = (0.005, 0.01)


def collect_events_cached(d: dict, tag: str) -> list[list]:
    """事件收集（带落盘缓存，避免重复跑信号层）。行 = [bar, ladder, side, confirmed]。"""
    p = CACHE / f"_confirm_events_{tag}.json"
    if p.exists():
        return json.loads(p.read_text())
    t0 = time.time()
    tape = compute_organic_signals(d["opens"], d["highs"], d["lows"], d["closes"])
    rows = []
    for bar, s in enumerate(tape):
        if not s.bsp_events:
            continue
        for ladder, evs in enumerate(s.bsp_events):
            for (_kind, side, _seg, confirmed, *_r) in evs:
                rows.append([bar, ladder, side, bool(confirmed)])
    p.write_text(json.dumps(rows))
    print(f"  [{tag}] 信号层 {time.time()-t0:.1f}s events={len(rows):,}", flush=True)
    return rows


def q50(xs: list) -> float:
    if not xs:
        return float("nan")
    xs = sorted(xs)
    return float(xs[len(xs) // 2])


def analyze_layer(ev: list[tuple], swings: list[tuple],
                  closes: list[float], n_bars: int) -> dict:
    """ev = [(idx_1s, side, confirmed)] 已排序。仅用 confirmed sell。"""
    sells = [(i, s, c) for (i, s, c) in ev if s == "sell" and c]
    sell_idx = [e[0] for e in sells]
    downs = [(s, e) for (d, s, e) in swings if d == "down" and e > s
             and closes[s] > closes[e]]
    # precision：事件落在任一下跌窗口内的比例
    starts = [s for (s, _e) in downs]
    in_down = 0
    for i in sell_idx:
        k = bisect.bisect_right(starts, i) - 1
        if k >= 0 and i <= downs[k][1]:
            in_down += 1
    precision = in_down / len(sell_idx) if sell_idx else float("nan")
    down_time = sum(e - s for (s, e) in downs)
    baseline = down_time / n_bars
    # coverage + consumed
    hit = 0
    consumed: list[float] = []
    for (s, e) in downs:
        lo = bisect.bisect_right(sell_idx, s)
        if lo < len(sell_idx) and sell_idx[lo] <= e:
            hit += 1
            peak, trough = closes[s], closes[e]
            consumed.append((peak - closes[sell_idx[lo]]) / (peak - trough))
    return {"n_events": len(sell_idx),
            "precision": round(precision, 4) if precision == precision else None,
            "baseline": round(baseline, 4),
            "lift": (round(precision / baseline, 2)
                     if precision == precision and baseline > 0 else None),
            "coverage": round(hit / len(downs), 4) if downs else None,
            "consumed_p50": (round(q50(consumed), 4) if consumed else None)}


def run(sym: str) -> None:
    d1s = load_btc() if sym == "BTC" else load_es()
    d1m = aggregate_1min(d1s)
    rows_1s = collect_events_cached(d1s, f"{sym.lower()}_1s")
    rows_1m = collect_events_cached(d1m, f"{sym.lower()}_1m")
    last1s = d1m["last1s"]
    n = len(d1s["closes"])

    # 事件映射到 1s 索引域，按 ladder 分桶
    by_ladder_1s: dict[int, list] = {}
    for (bar, lad, side, conf) in rows_1s:
        by_ladder_1s.setdefault(lad, []).append((bar, side, conf))
    by_ladder_1m: dict[int, list] = {}
    for (bar, lad, side, conf) in rows_1m:
        by_ladder_1m.setdefault(lad, []).append((last1s[bar], side, conf))

    out: dict = {"symbol": sym, "thresholds": {}}
    for th in THRESHOLDS:
        swings = zigzag_swings(d1s["closes"], th)
        n_downs = sum(1 for (d, s, e) in swings if d == "down")
        block: dict = {"n_down_swings": n_downs, "towers": {}}
        for tower, by_ladder in (("1s", by_ladder_1s), ("1min", by_ladder_1m)):
            tw: dict = {}
            ladders = sorted(by_ladder)
            for lad in ladders:
                ev = sorted(by_ladder[lad])
                tw[f"ladder{lad}"] = analyze_layer(ev, swings, d1s["closes"], n)
            for lmin in ladders:
                ev = sorted(x for lad in ladders if lad >= lmin
                            for x in by_ladder[lad])
                tw[f"ladder>={lmin}"] = analyze_layer(ev, swings, d1s["closes"], n)
            block["towers"][tower] = tw
        out["thresholds"][f"{th*100:g}%"] = block
        print(f"[{sym}][zigzag {th*100:g}%] downs={n_downs}", flush=True)
        for tower in ("1s", "1min"):
            for name, r in block["towers"][tower].items():
                print(f"  {tower:>4} {name:<10} n={r['n_events']:>6} "
                      f"prec={r['precision']} base={r['baseline']} "
                      f"lift={r['lift']} cover={r['coverage']} "
                      f"consumed_p50={r['consumed_p50']}", flush=True)

    p = CACHE / f"confirm_accel_spec_{sym.lower()}.json"
    p.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"✓ 落盘 {p}", flush=True)


if __name__ == "__main__":
    for sym in (sys.argv[1:] or ["BTC", "ES"]):
        run(sym)
