"""腾讯 700 — 真假买点过滤器回测：MACD 背驰 vs PH(sublevel/路径空间) vs 无过滤（L2/L3）。

回应 backlog #8 + 本轮 §8 结论的 L3 回测验证。
§8 已在 L0(Gemini)+L2(#33/#37 case) 否决"路径空间 PH 能区分真假买点"。本回测做 L3 验证：
在腾讯 700 全序列上，对每个候选买点比较各过滤器**预测真假的精度**。

设计
----
- **候选买点** = 每个下跌 leg 终点（候选反转/1B 位置）。
- **真买点(真反转)** = 终点后 LOOKAHEAD 根内价格上行 ≥ REVERSAL_K×ATR（先于继续创新低）。
- **假买点** = 否（继续下跌/未实质反转）。
- **过滤器**（对同向相邻下跌 leg 对 A→C，C 为候选）：
  1. MACD 背驰：macd_area(C) < macd_area(A)（动量衰竭 → 预测真反转）。
  2. sublevel-H0：totH0(C) < totH0(A)（PH 幅度"背驰"）。
  3. 路径空间(全局σ)：pathH0(C) < pathH0(A)。
  4. 无过滤：所有候选。
- **指标** = 精度 P(真 | 过滤器判真)。过滤器"有价值" ⟺ 精度 > 基础率 P(真)。

预期（§8 结论）：MACD 精度 > 基础率（能区分）；PH 精度 ≈ 基础率（不能区分）。
否定性结果（PH 无区分力）= L3 证实 §8/谱系520。

认识论等级：L2/弱 L3（腾讯 700 单标的全序列，可否证）。
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
from newchan.a_path_persistence import path_persistence, scale_estimate  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
LOOKAHEAD = 25       # 反转确认窗口（根）
RETRACE_FRAC = 0.5   # 真反转判据：后续上行回撤 ≥ 该下跌 leg 幅度的此比例


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


def reversal_outcome(closes, end_idx: int, leg_amp: float) -> tuple[bool, float]:
    """候选买点（下跌 leg 终点）的反转结果。

    真反转判据（避免天花板效应）：LOOKAHEAD 内的**最大上行回撤 ≥ 该下跌 leg 幅度的
    RETRACE_FRAC**（实质反转），而非任意小反弹。同时返回连续的前向回撤比例（用于相关）。

    Returns (is_true, forward_retrace_frac)
    """
    n = len(closes)
    base = closes[end_idx]
    if end_idx >= n - 1 or leg_amp <= 0:
        return False, 0.0
    window = closes[end_idx + 1: min(end_idx + 1 + LOOKAHEAD, n)]
    if not window:
        return False, 0.0
    max_up = max(window) - base          # 窗口内最大上行
    retrace = max_up / leg_amp           # 相对下跌 leg 幅度的回撤比例
    return retrace >= RETRACE_FRAC, retrace


def precision(flags: np.ndarray, truth: np.ndarray) -> tuple[float, int]:
    """P(真 | 过滤器判真) + 判真样本数。"""
    sel = flags.astype(bool)
    n = int(sel.sum())
    if n == 0:
        return float("nan"), 0
    return float(truth[sel].mean()), n


def pearson(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def build_rows(name, ofile):
    highs, lows, closes = load_closes(ofile)
    atr_val = atr(highs, lows, closes, period=14)
    legs = zigzag_legs(closes, threshold=max(atr_val, 1e-9))
    df = compute_macd(pd.DataFrame({"close": closes}))
    GP = scale_estimate(closes, "std")
    GT = scale_estimate(list(range(len(closes))), "std")
    downs = [(k, a, b, d) for k, (a, b, d) in enumerate(legs) if d < 0]
    rows = []
    for i in range(1, len(downs)):
        _, aA, bA, dA = downs[i - 1]
        kC, aC, bC, dC = downs[i]
        segA, segC = closes[aA:bA + 1], closes[aC:bC + 1]
        if len(segA) < 3 or len(segC) < 3:
            continue
        macdA = abs(macd_area_for_range(df, aA, bA)["area_neg"]) or 1e-9
        macdC = abs(macd_area_for_range(df, aC, bC)["area_neg"])
        subA = barcode_from_prices(segA, maxdim=0).total_persistence(0) or 1e-9
        subC = barcode_from_prices(segC, maxdim=0).total_persistence(0)
        pathA = path_persistence(segA, sigma_p=GP, sigma_t=GT, maxdim=0).total_persistence(0) or 1e-9
        pathC = path_persistence(segC, sigma_p=GP, sigma_t=GT, maxdim=0).total_persistence(0)
        truth, retrace = reversal_outcome(closes, bC, abs(dC))
        rows.append({
            "tf": name, "kC": kC, "truth": truth, "retrace": retrace,
            "macd_div": macdC < macdA, "sub_div": subC < subA, "path_div": pathC < pathA,
            # 连续"背驰程度"= C/A 比值（越小越背驰 → 越该真反转）
            "macd_ratio": macdC / macdA, "sub_ratio": subC / subA, "path_ratio": pathC / pathA,
        })
    return rows


def main():
    print("#" * 72)
    print("# 腾讯 700 — 真假买点过滤器回测：MACD vs PH(sublevel/路径空间) vs 无过滤")
    print(f"# 真反转判据=后续{LOOKAHEAD}根最大上行回撤≥下跌leg幅度×{RETRACE_FRAC}（避免天花板）")
    print("#" * 72)

    rows = build_rows("日线", "daily_ohlcv.json") + build_rows("30分", "m30_ohlcv.json")
    R = pd.DataFrame(rows)
    if R.empty:
        print("候选对不足。")
        return
    truth = R["truth"].to_numpy()
    base = float(truth.mean())
    print(f"\n候选买点对 n={len(R)}（日线+30分合并）  基础真反转率 P(真)={base:.3f}"
          f"（{int(truth.sum())}/{len(R)}）")
    if base > 0.85 or base < 0.15:
        print("  ⚠ 基础率极端 → 测试区分力弱（样本一边倒），结论须谨慎。")

    print(f"\n  二元精度  {'过滤器':<20}{'判真数':>7}{'精度':>9}{'相对基础率':>11}{'显著?(lift>2SE)':>14}")
    for nm, col in [("MACD 背驰", "macd_div"), ("sublevel-H0", "sub_div"),
                    ("路径空间(全局σ)", "path_div")]:
        prec, n = precision(R[col].to_numpy(), truth)
        lift = prec - base if not np.isnan(prec) else float("nan")
        # 二项标准误 + 显著性：lift 是否超过 2×SE（粗略 95%）
        se = (prec * (1 - prec) / n) ** 0.5 if n > 0 and not np.isnan(prec) else float("inf")
        sig = "显著" if (not np.isnan(lift) and lift > 2 * se) else "✗不显著(噪声内)"
        print(f"            {nm:<20}{n:>7}{prec:>9.3f}{lift:>+11.3f}{sig:>14}")
    print(f"            {'无过滤(基准)':<20}{len(R):>7}{base:>9.3f}{0.0:>+11.3f}")

    # 连续相关：背驰比值(C/A) 越小应越对应大回撤(真反转) → 期望负相关
    print(f"\n  连续相关  corr(背驰比值 C/A, 前向回撤)  [越负=越能预测真反转]")
    for nm, col in [("MACD", "macd_ratio"), ("sublevel-H0", "sub_ratio"),
                    ("路径空间(全局σ)", "path_ratio")]:
        r = pearson(R[col], R["retrace"])
        print(f"            {nm:<20} corr={r:+.3f}")

    print("\n" + "=" * 72)
    print("诚实判读（formalization-validity-domain）：")
    print("- n 小，若所有 lift 均'不显著' → 三过滤器统计上不可区分，回测**欠功效**，")
    print("  既不证实也不推翻 §8 的 L0+L2 否决。把噪声读成'PH有效'=合成确认偏差。")
    print("- PH(全局σ)≈sublevel≈幅度（§8.3 P1′）→ PH 仍未超出 sublevel-H0。")
    print("- 真正的 L3 裁决需多标的大样本；本窗口单标的不足以作 PH 真假过滤的判据。")
    print("认识论等级：L2/弱L3（腾讯700单标的日线+30分，n 偏小，可否证）。")
    print("=" * 72)


if __name__ == "__main__":
    main()
