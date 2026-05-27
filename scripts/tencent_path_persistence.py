"""腾讯 700 — 路径空间 (t,p) Rips 持续同调的 L2 经验检验。

目的：在真实数据上检验 Gemini decide（2026-05-27 选项C 拒绝）的三个可证伪 L0 预言，
而非"让方案跑通"。否定性结果（路径空间不提供新信息）是合法且有价值的 L2 产出（231号）。

检验项
------
P1（信息论否决）：路径空间能否区分 §5 的 #33（缓跌,MACD0.644）vs #37（急跌,MACD13.467）？
                  跨 40 笔，path-H0 与 MACD 面积的 Pearson 相关。
P2（自由参数）  ：path-H0 在 {std, mad, range} 三种归一化下是否给出不同结果（排序是否变）？
P3（数学退化）  ：单调笔的 path-H1 是否恒空？path-H0 max persistence 是否 ≈ max_step_distance？

认识论等级：本脚本读出 = L2（腾讯 700 单标的单时段，可否证）。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_persistence_barcode import barcode_from_prices, atr  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_path_persistence import (  # noqa: E402
    path_persistence,
    max_step_distance,
)

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
NORMS = ("std", "mad", "range")


def load_closes(ofile: str):
    blob = json.loads((DATA / ofile).read_text())
    bars = blob["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


def zigzag_legs(closes, threshold: float):
    if len(closes) < 3:
        return []
    pivots = [0]
    direction = 0
    piv, pidx = closes[0], 0
    for i in range(1, len(closes)):
        c = closes[i]
        if direction >= 0 and c - piv >= threshold:
            direction = 1 if direction == 0 else direction
            piv, pidx = c, i
        elif direction <= 0 and piv - c >= threshold:
            direction = -1 if direction == 0 else direction
            piv, pidx = c, i
        if direction == 1 and piv - c >= threshold:
            pivots.append(pidx)
            direction = -1
            piv, pidx = c, i
        elif direction == -1 and c - piv >= threshold:
            pivots.append(pidx)
            direction = 1
            piv, pidx = c, i
    if pivots[-1] != len(closes) - 1:
        pivots.append(len(closes) - 1)
    return [(a, b, closes[b] - closes[a]) for a, b in zip(pivots[:-1], pivots[1:])]


def pearson(x, y) -> float:
    x = np.asarray(x, float)
    y = np.asarray(y, float)
    if len(x) < 2 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def spearman(x, y) -> float:
    """秩相关——检验单调一致性（对非线性更鲁棒）。"""
    x = np.asarray(x, float)
    y = np.asarray(y, float)
    if len(x) < 2:
        return float("nan")
    rx = pd.Series(x).rank().to_numpy()
    ry = pd.Series(y).rank().to_numpy()
    return pearson(rx, ry)


def main():
    highs, lows, closes = load_closes("daily_ohlcv.json")
    tau = atr(highs, lows, closes, period=14)
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    df = compute_macd(pd.DataFrame({"close": closes}))

    print("#" * 72)
    print("# 腾讯 700 日线 — 路径空间 (t,p) Rips 持续同调 L2 检验")
    print(f"# n={len(closes)}  ATR={tau:.3f}  zigzag 笔数={len(legs)}")
    print("#" * 72)

    rows = []
    for k, (a, b, d) in enumerate(legs):
        seg = closes[a : b + 1]
        if len(seg) < 3:
            continue
        bc = barcode_from_prices(seg, maxdim=0)
        sub_max = bc.max_persistence(0)
        sub_tot = bc.total_persistence(0)
        area = macd_area_for_range(df, a, b)
        macd = abs(area["area_neg"]) if d < 0 else abs(area["area_pos"])

        pp = {nm: path_persistence(seg, normalization=nm, maxdim=1) for nm in NORMS}
        msd = {nm: max_step_distance(seg, normalization=nm) for nm in NORMS}
        h1_empty = all(pp[nm].total_persistence(1) == 0.0 for nm in NORMS)

        rows.append(
            {
                "k": k, "a": a, "b": b, "span": b - a, "dir": "down" if d < 0 else "up",
                "amp": abs(d), "sub_max": sub_max, "sub_tot": sub_tot, "macd": macd,
                **{f"ph0max_{nm}": pp[nm].max_persistence(0) for nm in NORMS},
                **{f"ph0tot_{nm}": pp[nm].total_persistence(0) for nm in NORMS},
                **{f"ph1tot_{nm}": pp[nm].total_persistence(1) for nm in NORMS},
                **{f"msd_{nm}": msd[nm] for nm in NORMS},
                "h1_empty": h1_empty,
            }
        )

    R = pd.DataFrame(rows)

    # ---- P1: #33 vs #37 决定性对照 ----
    print("\n" + "=" * 72)
    print("【P1 · 信息论否决】#33（缓跌）vs #37（急跌）—— 路径空间能否区分？")
    print("=" * 72)
    cols = ["k", "span", "amp", "macd", "sub_max", "sub_tot",
            "ph0max_std", "ph0tot_std", "ph1tot_std"]
    sub = R[R["k"].isin([33, 37])][cols]
    print(sub.to_string(index=False, float_format=lambda x: f"{x:.3f}"))
    if len(sub) == 2:
        r33 = R[R["k"] == 33].iloc[0]
        r37 = R[R["k"] == 37].iloc[0]
        print(f"\n  MACD:        #33={r33['macd']:.3f}  #37={r37['macd']:.3f}  "
              f"比值 #37/#33 = {r37['macd']/max(r33['macd'],1e-9):.1f}×")
        for nm in NORMS:
            v33, v37 = r33[f"ph0tot_{nm}"], r37[f"ph0tot_{nm}"]
            ratio = v37 / max(v33, 1e-9)
            agree = "✓与MACD同向(#37>#33)" if v37 > v33 else "✗与MACD反向(#37<#33)"
            print(f"  path-H0tot[{nm:>5}]: #33={v33:.3f} #37={v37:.3f} "
                  f"比值={ratio:.2f}× → {agree}")

    # ---- P1: 全笔 Pearson/Spearman ----
    print("\n  ── 跨全部笔的相关性（path-H0 是否携带 MACD 的动量信息？）──")
    valid = R[R["macd"] > 1e-6]
    print(f"     (有效笔 n={len(valid)})")
    for nm in NORMS:
        rp = pearson(valid[f"ph0tot_{nm}"], valid["macd"])
        rs = spearman(valid[f"ph0tot_{nm}"], valid["macd"])
        print(f"     path-H0tot[{nm:>5}] vs MACD面积: Pearson={rp:+.3f}  Spearman={rs:+.3f}")
    rp_sub = pearson(valid["sub_tot"], valid["macd"])
    print(f"     [对照] sublevel-H0tot vs MACD面积: Pearson={rp_sub:+.3f}")
    rp_amp = pearson(valid["amp"], valid["macd"])
    print(f"     [对照] 纯幅度 amp     vs MACD面积: Pearson={rp_amp:+.3f}")

    # ---- P2: 自由参数效应 ----
    print("\n" + "=" * 72)
    print("【P2 · 自由参数陷阱】path-H0 在三种归一化下结果是否改变？")
    print("=" * 72)
    for i, nm1 in enumerate(NORMS):
        for nm2 in NORMS[i + 1 :]:
            rp = pearson(R[f"ph0tot_{nm1}"], R[f"ph0tot_{nm2}"])
            rs = spearman(R[f"ph0tot_{nm1}"], R[f"ph0tot_{nm2}"])
            # 排序是否一致：取各自 top-3 笔
            top1 = set(R.nlargest(3, f"ph0tot_{nm1}")["k"])
            top2 = set(R.nlargest(3, f"ph0tot_{nm2}")["k"])
            print(f"  {nm1:>5} vs {nm2:>5}: Pearson={rp:+.3f} Spearman={rs:+.3f}  "
                  f"top3笔交集={len(top1&top2)}/3 {'(排序稳定)' if top1==top2 else '(排序改变!)'}")

    # ---- P3: 数学退化 ----
    print("\n" + "=" * 72)
    print("【P3 · 数学退化】单调笔 H1 是否恒空？path-H0max ≈ max_step_distance？")
    print("=" * 72)
    n_h1_empty = int(R["h1_empty"].sum())
    print(f"  H1 恒空的笔: {n_h1_empty}/{len(R)} "
          f"({'★ 全部单调笔 H1=空，论断3前半成立' if n_h1_empty==len(R) else '部分笔有 H1'})")
    for nm in NORMS:
        rp = pearson(R[f"ph0max_{nm}"], R[f"msd_{nm}"])
        # 相等程度：相对误差中位数
        rel = np.abs(R[f"ph0max_{nm}"] - R[f"msd_{nm}"]) / (R[f"msd_{nm}"] + 1e-9)
        print(f"  [{nm:>5}] corr(path-H0max, max_step_dist)={rp:+.4f}  "
              f"中位相对误差={np.median(rel):.4f} "
              f"{'★ H0max≈maxstep，论断3后半成立' if np.median(rel)<0.05 else ''}")

    print("\n" + "=" * 72)
    print("结论将由上述三项 L2 数据判定（见 stdout）。否定性结果同样有效（231号）。")
    print("=" * 72)


if __name__ == "__main__":
    main()
