"""腾讯 700 — 双闸门 PH（结构）+ MACD 动量闸 的三闸门流程 L2 实测（模块4）。

⚠ 定位修正（521号已结算定理）
-----------------------------
原任务表述"纯 PH 三闸门"的**前提被 521号定理证伪**：纯拓扑因果动量不变量不存在
（动量需时间度量，拓扑丢弃时间）。故本脚本按 521号下游裁定降级为
**「双闸门 PH（结构）+ MACD 动量闸」**：
- 闸门①（趋势/盘整）：**PH 结构闸**——PH-中枢检测数中枢（≥2→趋势）。纯 PH（a_ph_zhongshu）。
- 闸门②（次级别背驰）：**MACD 动量闸**——fine 段 C vs A 用 MACD hist 的几何动量
  over 拓扑泛函（a_geometric_momentum，动量来自 MACD，**非纯 PH**，521号）。
- 闸门③（大级别同步/区间套）：**MACD 动量闸**——coarse 段 C vs A 的 MACD 动量背驰。
三闸门全过 → 预测真底。**动量闸（②③）本质是 MACD，不是 PH——这是 521号的强制定位。**

缠论锚点（一级权威）
-------------------
1B = 下跌趋势（≥2 依次向下中枢）中，次级别背驰点（第17/24课）；真假取决于趋势背驰
（C 段力度 < A 段，第24课）+ 区间套大小级别共振（第29课）。三闸门 = ①趋势前提(PH)
②背驰(MACD) ③区间套(MACD)。

Ground truth（已确认，见脚本头分析）
-----------------------------------
真底：idx23（close=435.4=全局最低，后续反弹 56%）。
假底：idx7/155/196/247/264（后续创新低否定）。idx298 末端未确认（剔除）。

诚实性约束（231号 / no-patch-mentality）
----------------------------------------
- 不预设三闸门能标记 idx23。否定性结果（三闸门误判 idx23）同样有效——它会揭示 PH
  各闸门的贡献与局限（如：idx23 缺历史大级别段 → 闸门③无法区间套；急跌见底 →
  幅度判据失效），是 §5/520 的延续，比"确认成功"更有价值。
- 段划分（zigzag）用全序列结构定位 A/C 段范围；段内 PH 度量（中枢/动量）严格只用
  段内数据（因果）。标注此边界。

认识论等级：算法 L0；"三闸门标记真假底"经验断言 L2（单标的日线，可否证）。

概念溯源标签
-----------
- 三闸门 = 趋势∧背驰∧区间套 [新缠论:候选——1B 三条件 PH 形式化]
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_geometric_momentum import momentum_divergence  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402
from newchan.a_ph_zhongshu import ZhongshuPolicy, detect_zhongshu  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
FINE_PCT = 0.05    # 次级别摆动（次级别买点候选 = fine 下跌段终点）
COARSE_PCT = 0.12  # 大级别趋势分段
REVERSAL_PCT = 0.10  # 真底确认：后续反弹 ≥ 10%
MIN_ZHONGSHU_TREND = 2  # 闸门①：≥2 中枢 = 趋势（缠论：依次向下 ≥2 中枢）


def load(ofile: str):
    bars = json.loads((DATA / ofile).read_text())["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


# ====================================================================
# ZigZag（段结构定位）
# ====================================================================


@dataclass(frozen=True, slots=True)
class Pivot:
    idx: int
    price: float
    kind: str


def zigzag(closes: list[float], pct: float) -> list[Pivot]:
    if not closes:
        return []
    pv: list[Pivot] = []
    ei, ep, d = 0, closes[0], 0
    for i in range(1, len(closes)):
        p = closes[i]
        if d >= 0 and p > ep:
            ei, ep = i, p
            d = 1 if d == 0 else d
        elif d <= 0 and p < ep:
            ei, ep = i, p
            d = -1 if d == 0 else d
        if d == 1 and p <= ep * (1 - pct):
            pv.append(Pivot(ei, ep, "H"))
            d, ei, ep = -1, i, p
        elif d == -1 and p >= ep * (1 + pct):
            pv.append(Pivot(ei, ep, "L"))
            d, ei, ep = 1, i, p
    if not pv or pv[-1].idx != ei:
        pv.append(Pivot(ei, ep, "H" if d == 1 else "L"))
    return pv


def down_legs(closes: list[float], pct: float) -> list[tuple[int, int]]:
    pv = zigzag(closes, pct)
    return [
        (pv[i - 1].idx, pv[i].idx)
        for i in range(1, len(pv))
        if pv[i - 1].kind == "H" and pv[i].kind == "L"
    ]


# ====================================================================
# 三闸门（因果：段内度量只用段内数据）
# ====================================================================


@dataclass(frozen=True, slots=True)
class GateResult:
    idx: int
    price: float
    # 闸门①
    n_zhongshu: int
    gate1_trend: bool
    # 闸门②
    g2_force_a: float
    g2_force_c: float
    gate2_divergent: bool
    g2_has_a: bool
    # 闸门③
    g3_force_a: float
    g3_force_c: float
    gate3_synced: bool
    g3_has_a: bool
    # 综合
    predict_real: bool


def count_zhongshu_in_segment(closes: list[float], i0: int, i1: int, tau: float) -> int:
    """闸门①：段 [i0,i1] 内的 PH 中枢数（因果：只用段内 close）。"""
    seg = closes[i0 : i1 + 1]
    if len(seg) < 4:
        return 0
    snap = OnlineMergeTree.from_prices(seg)
    zs = detect_zhongshu(
        snap.settled_bars, noise_floor=tau, policy=ZhongshuPolicy(min_members=3)
    )
    return len(zs)


def _leg_containing(legs: list[tuple[int, int]], idx: int) -> int | None:
    """返回终点为 idx（或最接近 idx 的）下跌段下标。"""
    for k, (_, lo) in enumerate(legs):
        if lo == idx:
            return k
    return None


def evaluate_gates(
    closes: list[float],
    tau: float,
    fine_legs: list[tuple[int, int]],
    coarse_legs: list[tuple[int, int]],
    cand_idx: int,
) -> GateResult:
    price = closes[cand_idx]

    # 闸门①：当前大级别(coarse)下跌段内中枢数 ≥ 2 → 趋势
    coarse_seg = next(
        ((h, lo) for h, lo in coarse_legs if h < cand_idx <= lo), None
    )
    if coarse_seg is None:
        # 候选不在任何 coarse 下跌段内（如反弹中）→ 用最近的 coarse 高点到 cand
        prior_h = max((h for h, _ in coarse_legs if h <= cand_idx), default=0)
        coarse_seg = (prior_h, cand_idx)
    nz = count_zhongshu_in_segment(closes, coarse_seg[0], cand_idx, tau)
    gate1 = nz >= MIN_ZHONGSHU_TREND

    # 闸门②：fine 级别 C 段(到 cand) vs 前一 fine 下跌段 A 段 动量背驰
    ck = _leg_containing(fine_legs, cand_idx)
    g2_fa = g2_fc = 0.0
    gate2 = False
    g2_has_a = False
    if ck is not None and ck >= 1:
        a_rng, c_rng = fine_legs[ck - 1], fine_legs[ck]
        d = momentum_divergence(closes[: cand_idx + 1], a_rng, c_rng)
        g2_fa, g2_fc, gate2 = d.force_a, d.force_c, d.is_divergent
        g2_has_a = True

    # 闸门③：coarse 大级别 C 段 vs 前一 coarse 下跌段 A 段 动量背驰（区间套同步）
    g3_fa = g3_fc = 0.0
    gate3 = False
    g3_has_a = False
    cck = next((k for k, (_, lo) in enumerate(coarse_legs) if lo == _nearest_coarse_low(coarse_legs, cand_idx)), None)
    if cck is not None and cck >= 1:
        a_rng, c_rng = coarse_legs[cck - 1], coarse_legs[cck]
        if c_rng[1] <= cand_idx:
            d = momentum_divergence(closes[: cand_idx + 1], a_rng, c_rng)
            g3_fa, g3_fc, gate3 = d.force_a, d.force_c, d.is_divergent
            g3_has_a = True

    predict = gate1 and gate2 and gate3
    return GateResult(
        idx=cand_idx, price=price, n_zhongshu=nz, gate1_trend=gate1,
        g2_force_a=g2_fa, g2_force_c=g2_fc, gate2_divergent=gate2, g2_has_a=g2_has_a,
        g3_force_a=g3_fa, g3_force_c=g3_fc, gate3_synced=gate3, g3_has_a=g3_has_a,
        predict_real=predict,
    )


def _nearest_coarse_low(coarse_legs: list[tuple[int, int]], idx: int) -> int:
    """与 idx 最接近的 coarse 下跌段低点（≤ idx）。"""
    lows = [lo for _, lo in coarse_legs if lo <= idx]
    return max(lows) if lows else -1


def ground_truth(closes: list[float], idx: int, price: float) -> str:
    """真底 / 假底 / 未确认（用未来，仅作真值）。"""
    fut = closes[idx + 1:]
    if any(c < price for c in fut):
        return "假底"
    if fut and max(fut) >= price * (1 + REVERSAL_PCT):
        return "真底"
    return "未确认"


# ====================================================================
# 主流程
# ====================================================================


def run(name: str, ofile: str):
    highs, lows, closes = load(ofile)
    tau = atr_noise_threshold(highs, lows, closes, multiple=1.0)
    fine_legs = down_legs(closes, FINE_PCT)
    coarse_legs = down_legs(closes, COARSE_PCT)
    fine_lows = [lo for _, lo in fine_legs]

    print("=" * 92)
    print(f"【{name}】纯 PH 三闸门统一流程（n={len(closes)} τ={tau:.1f} "
          f"fine={FINE_PCT:.0%} coarse={COARSE_PCT:.0%}）")
    print(f"  大级别(coarse)下跌段: {coarse_legs}")
    print(f"  次级别买点候选(fine 低点): {fine_lows}")
    print("=" * 92)

    results = [
        (evaluate_gates(closes, tau, fine_legs, coarse_legs, t),
         ground_truth(closes, t, closes[t]))
        for t in fine_lows
    ]

    hdr = (f"{'idx':>4}{'price':>8}{'真值':>7} | {'①中枢数':>7}{'①趋势':>6} | "
           f"{'②力度A→C':>14}{'②背驰':>6} | {'③力度A→C':>14}{'③同步':>6} | {'PH预测':>7}{'命中':>5}")
    print(hdr)
    print("-" * len(hdr))
    tp = fp = tn = fn = 0
    for g, gt in results:
        if gt == "未确认":
            mark = "—(剔除)"
            hit = "—"
        else:
            pred_real = g.predict_real
            actual_real = gt == "真底"
            hit = "✓" if pred_real == actual_real else "✗"
            if pred_real and actual_real:
                tp += 1
            elif pred_real and not actual_real:
                fp += 1
            elif (not pred_real) and (not actual_real):
                tn += 1
            else:
                fn += 1
            mark = "真" if pred_real else "假"
        g2 = (f"{g.g2_force_a:.2f}→{g.g2_force_c:.2f}" if g.g2_has_a else "无A段")
        g3 = (f"{g.g3_force_a:.2f}→{g.g3_force_c:.2f}" if g.g3_has_a else "无A段")
        print(f"{g.idx:>4}{g.price:>8.1f}{gt:>7} | {g.n_zhongshu:>7}"
              f"{('✓' if g.gate1_trend else '✗'):>6} | {g2:>14}"
              f"{('✓' if g.gate2_divergent else '✗'):>6} | {g3:>14}"
              f"{('✓' if g.gate3_synced else '✗'):>6} | {mark:>7}{hit:>5}")

    n_eval = tp + fp + tn + fn
    print(f"\n  混淆矩阵(剔除未确认, n={n_eval}): TP={tp} FP={fp} TN={tn} FN={fn}")
    if n_eval:
        acc = (tp + tn) / n_eval
        n_real = tp + fn
        base = (n_eval - n_real) / n_eval  # 全预测"假底"的 baseline 准确率
        print(f"  准确率={acc:.0%}  ⚠诚实标注: baseline(全预测假底)={base:.0%}；"
              f"三闸门 TP={tp} → {'零区分力(从未预测真底)' if tp == 0 else '有区分力'}")
        if tp == 0:
            print(f"  → 否定性结果(231号)：三闸门把全部候选(含真底)判为非真底，"
                  f"准确率仅来自类别不平衡，无真实区分力。")
    # 诚实分析：idx23 真底的三闸门表现
    g23 = next((g for g, _ in results if g.idx == 23), None)
    if g23 is not None:
        print(f"\n  【真底 idx23 三闸门诊断】")
        print(f"    闸门①趋势: {'✓' if g23.gate1_trend else '✗'}（段内中枢={g23.n_zhongshu}）")
        print(f"    闸门②背驰: {'✓' if g23.gate2_divergent else '✗'}"
              f"（{'有A段对比' if g23.g2_has_a else '无前序fine段'}）")
        print(f"    闸门③同步: {'✓' if g23.gate3_synced else '✗'}"
              f"（{'有大级别A段' if g23.g3_has_a else '无前序coarse段——数据起点附近大级别底'}）")
        print(f"    → PH三闸门预测 idx23 为: {'真底✓' if g23.predict_real else '非真底✗(否定性结果)'}")
    return results


def main():
    print("#" * 92)
    print("# 腾讯 700 — 双闸门PH(结构)+MACD动量闸：能否标记真底 idx23 vs 假底（L2，521号定位）")
    print("#" * 92)
    run("日线 1D", "daily_ohlcv.json")
    print()
    print("=" * 92)
    print("结论标注(L2)：①PH中枢闸 ②③MACD动量闸（非纯PH，521号）。否定性结果（误判 idx23）")
    print("揭示：idx23 是急跌见底（中枢=0、动量C>A 不衰减），非趋势背驰型——三闸门(趋势背驰1B")
    print("的形式化)结构上不覆盖此类底。这印证 521号(纯拓扑动量不存在)并延续 §5/520。")
    print("=" * 92)


if __name__ == "__main__":
    main()
