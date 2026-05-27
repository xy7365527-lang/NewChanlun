"""腾讯 700 — 动量拓扑 vs MACD 面积的参数鲁棒性 L2 实测（模块3）。

任务问题
--------
不同 MACD 参数（8-21-5 / 12-26-9 / 20-40-9）分别算 MACD → 分别做 PH。
看 persistence diagram 在不同参数下的 bottleneck 距离 vs 原始 MACD 面积的差异。
**验证：PH 是否确实比原始 MACD 面积更抗参数变化？**

方法论陷阱（必读——520 号 σ 教训的复现警惕）
---------------------------------------------
bottleneck 距离（hist 量纲）与 MACD 面积（Σhist 积分量纲）**不同量纲**，直接比大小
无意义；要比"鲁棒性"必须先无量纲化，而**归一化方式的选择本身可能是伪自由参数**
（正如 520 号的 σ_p/σ_t）。故本脚本：

1. **主判据（无量纲、无归一化自由度）**：背驰判定量在参数间的**变异系数 CV**
   = std/mean。背驰决策依赖**比值** force_c/force_a（拓扑）与 area_c/area_a（面积），
   二者都是无量纲比值，CV 可直接公平对比。CV 越小 = 越抗参数变化。
   并报告**背驰 bool 判定的翻转率**（参数变了，"是否背驰"的结论是否翻转）——
   这是操盘层最硬的鲁棒性指标。
2. **补充判据（任务字面，带 caveat）**：persistence diagram 的两两 bottleneck 距离，
   按各段 total_persistence 归一化（消量纲），vs 面积的两两相对差。标注：归一化
   选择是一个自由度，故仅作参照，不作主结论。

认识论等级（formalization-validity-domain）
-------------------------------------------
- CV / bottleneck / 翻转率算法：**L0**（确定性统计）。
- "PH 比面积更抗参数"经验断言：**L2**（腾讯 700 单标的日线，可否证；否定性结果
  ——若 PH 不更鲁棒——同样有效且有价值，231号）。

概念溯源标签
-----------
- 参数鲁棒性 = 背驰比值 CV [新缠论:候选——拓扑稳定性 vs 面积]
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import numpy as np  # noqa: E402
from persim import bottleneck  # noqa: E402

from newchan.a_geometric_momentum import momentum_barcode, momentum_divergence  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
PARAM_SETS = [(8, 21, 5), (12, 26, 9), (20, 40, 9)]
# 笔级别摆动阈值（找相邻同向下跌段 A、C）——按周期定，因不同周期单根波幅不同。
# 这只是"样本空间"的划定，不影响被测的鲁棒性对象（拓扑/面积本身在固定段上算）。
ZIGZAG_PCT = {"日线 1D": 0.05, "30分钟": 0.03}


def load_closes(ofile: str) -> list[float]:
    bars = json.loads((DATA / ofile).read_text())["bars"]
    return [float(b["close"]) for b in bars]


# ====================================================================
# ZigZag → 相邻下跌段对 (A, C)
# ====================================================================


@dataclass(frozen=True, slots=True)
class Pivot:
    idx: int
    price: float
    kind: str


def zigzag(closes: list[float], pct: float) -> list[Pivot]:
    if not closes:
        return []
    pivots: list[Pivot] = []
    ext_idx, ext_price, direction = 0, closes[0], 0
    for i in range(1, len(closes)):
        p = closes[i]
        if direction >= 0 and p > ext_price:
            ext_idx, ext_price = i, p
            direction = 1 if direction == 0 else direction
        elif direction <= 0 and p < ext_price:
            ext_idx, ext_price = i, p
            direction = -1 if direction == 0 else direction
        if direction == 1 and p <= ext_price * (1.0 - pct):
            pivots.append(Pivot(ext_idx, ext_price, "H"))
            direction, ext_idx, ext_price = -1, i, p
        elif direction == -1 and p >= ext_price * (1.0 + pct):
            pivots.append(Pivot(ext_idx, ext_price, "L"))
            direction, ext_idx, ext_price = 1, i, p
    if not pivots or pivots[-1].idx != ext_idx:
        pivots.append(Pivot(ext_idx, ext_price, "H" if direction == 1 else "L"))
    return pivots


def down_legs(closes: list[float], pct: float) -> list[tuple[int, int]]:
    """相邻 H→L 下跌段 [high_idx, low_idx]。"""
    pv = zigzag(closes, pct)
    return [
        (pv[i - 1].idx, pv[i].idx)
        for i in range(1, len(pv))
        if pv[i - 1].kind == "H" and pv[i].kind == "L"
    ]


def adjacent_down_pairs(closes: list[float], pct: float) -> list[tuple]:
    """相邻同向下跌段对 (A_range, C_range)，C 在 A 之后（中间隔一次反弹）。

    背驰对比的样本空间：连续两个下跌 leg，C 段创新低（C 的低点 < A 的低点）。
    """
    legs = down_legs(closes, pct)
    pairs = []
    for k in range(1, len(legs)):
        a, c = legs[k - 1], legs[k]
        if closes[c[1]] < closes[a[1]]:  # C 创新低 → 可比背驰
            pairs.append((a, c))
    return pairs


# ====================================================================
# 鲁棒性统计（无量纲）
# ====================================================================


def cv(values: list[float]) -> float:
    """变异系数 = std / |mean|（无量纲离散度）。mean≈0 时返回 inf。"""
    arr = np.asarray([v for v in values if np.isfinite(v)], dtype=float)
    if arr.size < 2:
        return 0.0
    m = float(np.mean(arr))
    if abs(m) < 1e-12:
        return float("inf")
    return float(np.std(arr)) / abs(m)


def pairwise_mean(vals: list[float]) -> float:
    """两两绝对差的均值（参数间离散度的绝对量）。"""
    n = len(vals)
    if n < 2:
        return 0.0
    diffs = [abs(vals[i] - vals[j]) for i in range(n) for j in range(i + 1, n)]
    return sum(diffs) / len(diffs)


def pairwise_bottleneck(diagrams: list[np.ndarray]) -> list[float]:
    out = []
    for i in range(len(diagrams)):
        for j in range(i + 1, len(diagrams)):
            out.append(float(bottleneck(diagrams[i], diagrams[j])))
    return out


# ====================================================================
# 主流程
# ====================================================================


def analyze_segment(closes: list[float], a_range, c_range) -> dict:
    """一个 (A,C) 背驰对在 3 组参数下的鲁棒性指标。"""
    ratio_ph, ratio_area = [], []
    div_ph, div_area = [], []
    dgms_a, dgms_c = [], []
    areas_a, areas_c, totp_a, totp_c = [], [], [], []
    for (f, s, sig) in PARAM_SETS:
        d = momentum_divergence(closes, a_range, c_range, fast=f, slow=s, signal=sig)
        ratio_ph.append(d.ratio)
        ratio_area.append(d.area_c / d.area_a if d.area_a > 0 else float("inf"))
        div_ph.append(d.is_divergent)
        div_area.append(d.area_is_divergent)
        bc_a = momentum_barcode(closes, i0=a_range[0], i1=a_range[1], fast=f, slow=s, signal=sig)
        bc_c = momentum_barcode(closes, i0=c_range[0], i1=c_range[1], fast=f, slow=s, signal=sig)
        dgms_a.append(bc_a.diagram())
        dgms_c.append(bc_c.diagram())
        areas_a.append(bc_a.macd_area)
        areas_c.append(bc_c.macd_area)
        totp_a.append(bc_a.total_persistence)
        totp_c.append(bc_c.total_persistence)

    # 主判据：背驰比值 CV（无量纲）+ 判定翻转
    cv_ph = cv(ratio_ph)
    cv_area = cv(ratio_area)
    flip_ph = len(set(div_ph)) > 1      # 拓扑背驰判定在参数间是否翻转
    flip_area = len(set(div_area)) > 1  # 面积背驰判定在参数间是否翻转

    # 补充判据（任务字面，带归一化 caveat）：bottleneck（按 total_persistence 归一）
    bn_a = pairwise_bottleneck(dgms_a)
    bn_c = pairwise_bottleneck(dgms_c)
    scale_a = np.mean(totp_a) if np.mean(totp_a) > 0 else 1.0
    scale_c = np.mean(totp_c) if np.mean(totp_c) > 0 else 1.0
    norm_bn = (np.mean(bn_a) / scale_a + np.mean(bn_c) / scale_c) / 2.0
    # 面积的相对两两差（无量纲）
    rel_area = (pairwise_mean(areas_a) / scale_a + pairwise_mean(areas_c) / scale_c) / 2.0

    return {
        "a": a_range, "c": c_range,
        "cv_ph": cv_ph, "cv_area": cv_area,
        "flip_ph": flip_ph, "flip_area": flip_area,
        "norm_bottleneck": float(norm_bn), "rel_area_diff": float(rel_area),
        "ratio_ph": ratio_ph, "ratio_area": ratio_area,
        "div_ph": div_ph, "div_area": div_area,
    }


def run(name: str, ofile: str):
    closes = load_closes(ofile)
    pct = ZIGZAG_PCT[name]
    pairs = adjacent_down_pairs(closes, pct)
    print("=" * 80)
    print(f"【{name}】动量拓扑 vs MACD 面积 参数鲁棒性（n={len(closes)}，zigzag={pct:.0%}，"
          f"参数集={PARAM_SETS}）")
    print(f"  背驰对(相邻下跌段, C创新低): {len(pairs)} 个")
    print("=" * 80)
    if not pairs:
        print("  无可比背驰对（L2 边界：该序列同向相邻下跌段不足）。")
        return None

    rows = [analyze_segment(closes, a, c) for a, c in pairs]
    print(f"  {'A段':>10}{'C段':>10}{'CV比值_PH':>10}{'CV比值_面积':>12}"
          f"{'PH翻转':>8}{'面积翻转':>9}{'归一bottleneck':>15}{'面积相对差':>11}")
    print("-" * 80)
    for r in rows:
        print(f"  {str(r['a']):>10}{str(r['c']):>10}{r['cv_ph']:>10.3f}{r['cv_area']:>12.3f}"
              f"{('是' if r['flip_ph'] else '否'):>8}{('是' if r['flip_area'] else '否'):>9}"
              f"{r['norm_bottleneck']:>15.3f}{r['rel_area_diff']:>11.3f}")

    # 聚合
    cv_ph_all = [r["cv_ph"] for r in rows if np.isfinite(r["cv_ph"])]
    cv_area_all = [r["cv_area"] for r in rows if np.isfinite(r["cv_area"])]
    flip_ph_rate = sum(r["flip_ph"] for r in rows) / len(rows)
    flip_area_rate = sum(r["flip_area"] for r in rows) / len(rows)
    med_cv_ph = float(np.median(cv_ph_all)) if cv_ph_all else float("nan")
    med_cv_area = float(np.median(cv_area_all)) if cv_area_all else float("nan")
    med_bn = float(np.median([r["norm_bottleneck"] for r in rows]))
    med_rel_area = float(np.median([r["rel_area_diff"] for r in rows]))

    print("\n  【主判据·无量纲】背驰比值变异系数 CV（越小越抗参数变化）:")
    print(f"    中位 CV(拓扑 force_c/force_a) = {med_cv_ph:.3f}")
    print(f"    中位 CV(面积 area_c/area_a)   = {med_cv_area:.3f}")
    print(f"    → {'拓扑更稳(CV更小)' if med_cv_ph < med_cv_area else '面积更稳或持平'}")
    print(f"  【主判据·操盘】背驰 bool 判定翻转率（参数变了结论是否翻转，越低越鲁棒）:")
    print(f"    拓扑背驰翻转率 = {flip_ph_rate:.0%}   面积背驰翻转率 = {flip_area_rate:.0%}")
    print(f"    → {'拓扑判定更稳' if flip_ph_rate < flip_area_rate else ('面积判定更稳' if flip_area_rate < flip_ph_rate else '两者持平')}")
    print(f"  【补充·带caveat】中位归一bottleneck={med_bn:.3f} vs 中位面积相对差={med_rel_area:.3f}")
    print(f"    caveat: bottleneck 按 total_persistence 归一化是一个自由度选择(520号σ教训)，仅参照。")
    return {
        "med_cv_ph": med_cv_ph, "med_cv_area": med_cv_area,
        "flip_ph_rate": flip_ph_rate, "flip_area_rate": flip_area_rate,
        "med_bn": med_bn, "med_rel_area": med_rel_area, "n_pairs": len(rows),
    }


def main():
    print("#" * 80)
    print("# 腾讯 700 — 参数鲁棒性：动量拓扑 persistence 是否比 MACD 面积更抗参数变化（L2）")
    print("#" * 80)
    results = {}
    for name, ofile in [("日线 1D", "daily_ohlcv.json"), ("30分钟", "m30_ohlcv.json")]:
        results[name] = run(name, ofile)
        print()
    print("=" * 80)
    print("结论标注(L2)：以无量纲的背驰比值 CV + 判定翻转率为准。否定性结果(PH 不更")
    print("鲁棒)同样有效——缩小有效域边界(231号)。bottleneck 因归一化自由度仅作参照。")
    print("=" * 80)


if __name__ == "__main__":
    main()
