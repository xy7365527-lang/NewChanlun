"""区间套嵌套确认测量 — 1min 高层 candidate 锚定后的 1s 细塔加速确认。

这是任务的决定性测量。前两个探针的结论：
  (1) per-ladder 滞后表：1s 塔同尺度层确认更快（结构对应 1s ladder k+1 ≈ 1min ladder k）；
  (2) 特异性对照：裸事件流（无锚定）相对 zigzag 回调 lift≈1——无条件细层确认
      是 base rate 效应，不构成定位能力。
⇒ 区间套的正确形式必须是**条件化**的：高层 candidate 出现（结构开始形成）后，
  用细塔的结构完成提前高层原生 confirmed。

测量设计
========
对 1min 塔每个 ladder∈{2,3} 的 sell candidate（key=(ladder,kind,side,seg_idx)，
candidate 首次入流 bar = 锚 c）：
  - 原生路径 T_A = 1min 塔同 key confirmed 入流时刻 − c（若到来；未到来 = 假 candidate）
  - 嵌套路径 T_B = c 之后 1s 塔 ladder≥K（K=3,4 两档）第一个 sell confirmed 时刻 − c
  - 价格面：confirm 时刻 close 相对锚时刻 close 的已走幅度（sell：下跌为"已走"）
  - 判别力对照：T_B 在真 candidate 组（最终被原生 confirm）vs 假组（从未 confirm）
    的分布差——嵌套确认若在两组同速到达，则它无法区分真假 candidate（无信息）；
  - 随机基线：1s ladder≥K sell confirmed 的无条件中位到达间隔（密度基线）。
    T_B ≈ 基线/2 ⇒ 嵌套确认到达时间与锚无关（泊松流），加速无信息。

用法：PYTHONPATH=src .venv/bin/python analysis/_confirm_accel_nesting.py [BTC|ES]
输出：data_cache/confirm_accel_nest_{sym}.json
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
)
from organic_signals import compute_organic_signals  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
NEST_K = (3, 4)          # 1s 塔嵌套确认的最低 ladder 档
HIGH_LADDERS = (2, 3)    # 1min 塔被锚定的高层


def collect_full_events(d: dict, tag: str) -> list[list]:
    """完整事件行 [bar, ladder, kind, side, seg_idx, confirmed]（带缓存）。"""
    p = CACHE / f"_confirm_events_full_{tag}.json"
    if p.exists():
        return json.loads(p.read_text())
    t0 = time.time()
    tape = compute_organic_signals(d["opens"], d["highs"], d["lows"], d["closes"])
    rows = []
    for bar, s in enumerate(tape):
        if not s.bsp_events:
            continue
        for ladder, evs in enumerate(s.bsp_events):
            for (kind, side, seg_idx, confirmed, *_r) in evs:
                rows.append([bar, ladder, kind, side, seg_idx, bool(confirmed)])
    p.write_text(json.dumps(rows))
    print(f"  [{tag}] 信号层 {time.time()-t0:.1f}s events={len(rows):,}", flush=True)
    return rows


def q(xs: list, p: float) -> float:
    if not xs:
        return float("nan")
    xs = sorted(xs)
    return float(xs[min(len(xs) - 1, int(p * len(xs)))])


def run(sym: str) -> None:
    d1s = load_btc() if sym == "BTC" else load_es()
    d1m = aggregate_1min(d1s)
    ev1s = collect_full_events(d1s, f"{sym.lower()}_1s")
    ev1m = collect_full_events(d1m, f"{sym.lower()}_1m")
    last1s = d1m["last1s"]
    ts1s = d1s["ts"]
    closes1s = d1s["closes"]
    n1s = len(closes1s)

    # 1s 塔 sell confirmed 时刻表（按 K 档）
    nest_idx: dict[int, list[int]] = {}
    for k in NEST_K:
        nest_idx[k] = sorted(bar for (bar, lad, _k, side, _sg, conf) in ev1s
                             if lad >= k and side == "sell" and conf)

    # 密度基线：无条件中位到达间隔（秒）
    baseline: dict[int, float] = {}
    for k, idxs in nest_idx.items():
        gaps = [ts1s[idxs[i + 1]] - ts1s[idxs[i]] for i in range(len(idxs) - 1)]
        baseline[k] = q(gaps, .5)

    # 1min 塔高层 sell candidate / confirmed 配对
    cand: dict[tuple, int] = {}
    conf: dict[tuple, int] = {}
    for (bar, lad, kind, side, seg_idx, confirmed) in ev1m:
        if lad not in HIGH_LADDERS or side != "sell":
            continue
        key = (lad, kind, seg_idx)
        if confirmed:
            conf.setdefault(key, bar)
        else:
            cand.setdefault(key, bar)

    out: dict = {"symbol": sym,
                 "nest_baseline_median_gap_secs": baseline,
                 "high_ladders": {}}
    for hl in HIGH_LADDERS:
        keys = [k for k in cand if k[0] == hl]
        rows = []
        for key in keys:
            cbar_1m = cand[key]
            anchor_1s = last1s[cbar_1m]          # candidate 观测时刻的 1s 索引
            anchor_ts = ts1s[anchor_1s]
            anchor_px = closes1s[anchor_1s]
            is_true = key in conf and conf[key] >= cbar_1m
            ta = ts1s[last1s[conf[key]]] - anchor_ts if is_true else None
            pa = ((anchor_px - closes1s[last1s[conf[key]]]) / anchor_px * 100
                  if is_true else None)
            row = {"true_candidate": is_true, "T_A_secs": ta,
                   "moved_A_pct": pa, "nested": {}}
            for k, idxs in nest_idx.items():
                j = bisect.bisect_right(idxs, anchor_1s)
                if j < len(idxs):
                    nb = idxs[j]
                    row["nested"][k] = {
                        "T_B_secs": ts1s[nb] - anchor_ts,
                        "moved_B_pct": (anchor_px - closes1s[nb]) / anchor_px * 100}
                else:
                    row["nested"][k] = None
            rows.append(row)

        # 汇总
        true_rows = [r for r in rows if r["true_candidate"]]
        false_rows = [r for r in rows if not r["true_candidate"]]
        summ: dict = {"n_candidates": len(rows), "n_true": len(true_rows),
                      "n_false": len(false_rows),
                      "T_A_secs": {"p50": q([r["T_A_secs"] for r in true_rows], .5),
                                   "p90": q([r["T_A_secs"] for r in true_rows], .9)},
                      "moved_A_pct_p50": q([r["moved_A_pct"] for r in true_rows], .5),
                      "nest": {}}
        for k in NEST_K:
            tb_true = [r["nested"][k]["T_B_secs"] for r in true_rows
                       if r["nested"][k]]
            tb_false = [r["nested"][k]["T_B_secs"] for r in false_rows
                        if r["nested"][k]]
            mb_true = [r["nested"][k]["moved_B_pct"] for r in true_rows
                       if r["nested"][k]]
            # 加速比：逐 candidate 配对 T_A/T_B（仅真组、双值齐全）
            ratios = [r["T_A_secs"] / r["nested"][k]["T_B_secs"]
                      for r in true_rows
                      if r["nested"][k] and r["nested"][k]["T_B_secs"] > 0
                      and r["T_A_secs"] is not None and r["T_A_secs"] > 0]
            summ["nest"][f"K>={k}"] = {
                "T_B_true_p50": q(tb_true, .5),
                "T_B_false_p50": q(tb_false, .5),
                "moved_B_pct_p50": q(mb_true, .5),
                "speedup_p50": q(ratios, .5),
                "baseline_gap_p50": baseline[k],
            }
        out["high_ladders"][f"ladder{hl}"] = summ
        print(f"[{sym}] 1min ladder{hl}: cand={len(rows)} "
              f"true={len(true_rows)} false={len(false_rows)} "
              f"T_A_p50={summ['T_A_secs']['p50']:.0f}s "
              f"moved_A_p50={summ['moved_A_pct_p50']:.3f}%", flush=True)
        for k in NEST_K:
            s = summ["nest"][f"K>={k}"]
            print(f"    nest K>={k}: T_B(true)={s['T_B_true_p50']:.0f}s "
                  f"T_B(false)={s['T_B_false_p50']:.0f}s "
                  f"moved_B={s['moved_B_pct_p50']:.3f}% "
                  f"speedup_p50={s['speedup_p50']:.1f}x "
                  f"random_gap={s['baseline_gap_p50']:.0f}s", flush=True)

    p = CACHE / f"confirm_accel_nest_{sym.lower()}.json"
    p.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"✓ 落盘 {p}", flush=True)


if __name__ == "__main__":
    for sym in (sys.argv[1:] or ["BTC", "ES"]):
        run(sym)
