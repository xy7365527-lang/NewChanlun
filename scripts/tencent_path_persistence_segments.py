"""腾讯 700 — 段间路径空间 (t,p) Rips 持续同调的 L2 实测（走势级，非笔级）。

与 §8（scripts/tencent_path_persistence.py，笔级）的根本区别
--------------------------------------------------------------
§8 在**单笔**（ATR-zigzag leg）上做路径空间 PH，被 Gemini L0 + 700 L2 双重否决
（路径空间只给"最大单根速度"，无 H1，自由参数 σ）。编排者 2026-05-27 提出新论点：

  1. "MACD 差异来自跨笔 EMA 记忆而非笔内几何"——但那是**笔内**做的。在**整段走势**
     （A段整体 vs C段整体）上做，段内含多笔+中枢，不存在"笔内几何太贫乏"问题。
  2. 缠论买点充分条件 = 大级别 C段力度 < A段力度（**段间对比**）。
  3. §8.4 Q-c 的累积 persistence dP/dt 速度判据被 700 否证（真底是加速赶底非减速）——
     本实验用**段级总力度的段间对比**替代 dP/dt 局部速度判据。

本脚本检验：走势/中级别段上的路径空间力度，能否
  (Q1) 区分 §5 的 #33 缓跌 vs #37 急跌（时间盲）——对照笔级已知失败；
  (Q2) 与 MACD 面积在段间背驰判定上一致（段间对比 = 缠论判据）；
  (Q3) 区分 700 真底 idx23 vs 末端底 idx298（真假买点）。

核心张力（no-patch：正面处理，不掩盖）
--------------------------------------
段间背驰对比要求**保留各段幅度**。但路径空间 σ 归一化若按**各段内部 std**算，
每段被缩放到单位方差 → 幅度被抹除 → 段间对比失效（见 test_path_persistence
test_per_segment_normalization_erases_amplitude）。故必须用**全局 σ**（跨段共享尺子）。
代价：σ_p/σ_t 比值是显式自由参数，且该比值同时控制"幅度保真"与"时间信息"——
σ_t 大 → 价格主导 → 退化为时间盲 sublevel H0；σ_t 小 → 时间主导 → 幅度被冲淡。
本脚本量化此权衡（aspect-ratio sweep），让数据判定有无"两者兼得"的比值。

认识论等级（formalization-validity-domain）
------------------------------------------
- 分段 / 力度计算 / Rips：L0（纯算法）。
- "段间路径空间力度区分真假/复现 MACD"经验断言：L2（腾讯 700 单标的日线，可否证）。
  否定性结果同样合法且有价值（缩小有效域边界，231号）。

概念溯源标签
-----------
- 段间路径空间背驰 [新缠论:候选——走势级时间盲解，编排者提出 2026-05-27]
- C段力度 < A段力度 [新缠论:知识库——0027 区间套 / 0024 背驰判据]
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_persistence_barcode import atr, barcode_from_prices  # noqa: E402
from newchan.a_path_persistence import (  # noqa: E402
    max_step_distance,
    path_persistence,
    scale_estimate,
)

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
NORMS = ("std", "mad", "range")


# ====================================================================
# ZigZag
# ====================================================================


@dataclass(frozen=True, slots=True)
class Pivot:
    idx: int
    price: float
    kind: str  # "H" | "L"


def zigzag(closes: list[float], pct: float) -> list[Pivot]:
    """百分比反转 zigzag（含端点修正）。"""
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


def zigzag_legs_amp(closes: list[float], threshold_abs: float):
    """绝对幅度阈值 zigzag（复现 §8 的笔级 ATR 分段，返回 (a,b,Δ)）。"""
    if len(closes) < 3:
        return []
    pivots = [0]
    direction = 0
    piv, pidx = closes[0], 0
    for i in range(1, len(closes)):
        c = closes[i]
        if direction >= 0 and c - piv >= threshold_abs:
            direction = 1 if direction == 0 else direction
            piv, pidx = c, i
        elif direction <= 0 and piv - c >= threshold_abs:
            direction = -1 if direction == 0 else direction
            piv, pidx = c, i
        if direction == 1 and piv - c >= threshold_abs:
            pivots.append(pidx)
            direction, piv, pidx = -1, c, i
        elif direction == -1 and c - piv >= threshold_abs:
            pivots.append(pidx)
            direction, piv, pidx = 1, c, i
    if pivots[-1] != len(closes) - 1:
        pivots.append(len(closes) - 1)
    return [(a, b, closes[b] - closes[a]) for a, b in zip(pivots[:-1], pivots[1:])]


# ====================================================================
# 段（走势/中级别）+ 力度度量
# ====================================================================


@dataclass(frozen=True, slots=True)
class Segment:
    a: int          # 起点 bar 索引
    b: int          # 终点 bar 索引（闭）
    kind: str       # "down" | "up"
    parent: str     # 所属大趋势标签


@dataclass(frozen=True, slots=True)
class GlobalScales:
    """全局共享尺子（段间对比的前提）。"""

    sigma_p: dict[str, float]   # 各归一化方法下的价格尺度
    sigma_t: float              # 时间尺度（balanced 基准）


def compute_global_scales(closes: list[float]) -> GlobalScales:
    n = len(closes)
    sp = {m: scale_estimate(closes, m) for m in NORMS}
    # balanced 时间尺度：使典型单根时间步(=1)/σ_t ≈ 典型单根价格步/σ_p(std)
    mean_abs_step = float(np.mean(np.abs(np.diff(closes)))) or 1.0
    sigma_t_balanced = sp["std"] / mean_abs_step  # 1/σ_t = mean|Δp|/σ_p
    return GlobalScales(sigma_p=sp, sigma_t=sigma_t_balanced)


def medium_down_legs(closes: list[float], lo: int, hi: int, pct: float) -> list[Segment]:
    """大趋势 [lo,hi] 内部的中级别下跌子段（绝对 bar 索引）。"""
    sub = closes[lo : hi + 1]
    piv = zigzag(sub, pct)
    legs: list[Segment] = []
    for i in range(len(piv) - 1):
        a, c = piv[i], piv[i + 1]
        kind = "down" if (a.kind == "H" and c.kind == "L") else (
            "up" if (a.kind == "L" and c.kind == "H") else "flat"
        )
        if kind in ("down", "up"):
            legs.append(Segment(a.idx + lo, c.idx + lo, kind, f"[{lo}->{hi}]"))
    return legs


def macd_force(df_macd: pd.DataFrame, seg: Segment) -> float:
    area = macd_area_for_range(df_macd, seg.a, seg.b)
    return abs(area["area_neg"]) if seg.kind == "down" else abs(area["area_pos"])


def sublevel_force(closes: list[float], seg: Segment) -> float:
    return barcode_from_prices(closes[seg.a : seg.b + 1], maxdim=0).total_persistence(0)


def path_force(
    closes: list[float],
    seg: Segment,
    *,
    sigma_p: float,
    sigma_t: float,
    dim: int,
) -> float:
    """段的路径空间总力度（全局 σ，含时间维度）。"""
    seg_prices = closes[seg.a : seg.b + 1]
    if len(seg_prices) < 3:
        return 0.0
    times = list(range(seg.a, seg.b + 1))
    res = path_persistence(
        seg_prices, times, maxdim=max(dim, 1), sigma_t=sigma_t, sigma_p=sigma_p
    )
    return res.total_persistence(dim)


# ====================================================================
# 相关性工具
# ====================================================================


def pearson(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 2 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def spearman(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 2:
        return float("nan")
    return pearson(pd.Series(x).rank().to_numpy(), pd.Series(y).rank().to_numpy())


# ====================================================================
# Q1：#33/#37 笔级复现（对照已知失败）
# ====================================================================


def q1_decisive_case(
    closes: list[float], highs: list[float], lows: list[float],
    df_macd: pd.DataFrame, gs: GlobalScales,
):
    print("=" * 78)
    print("【Q1 · 时间盲】#33 缓跌 vs #37 急跌——走势级 σ 下路径空间能否区分？")
    print("  §5 决定性 case：两笔幅度≈55，MACD 面积差 20×（缓跌 vs 急跌）。")
    print("  对照：§8.3 笔级已知 path-H0 区分塌缩/方向翻转。本节用全局 σ 复测。")
    print("=" * 78)
    tau = atr(highs, lows, closes, period=14)  # 复现 §8 的真实 ATR 分段
    legs = zigzag_legs_amp(closes, threshold_abs=max(tau, 1e-9))
    print(f"  (ATR={tau:.3f}, zigzag 笔数={len(legs)})")
    if len(legs) <= 37:
        print(f"  ⚠ ATR-zigzag 仅 {len(legs)} 笔，不足 38——无法定位 #33/#37（口径差异）。")
        return
    for k in (33, 37):
        a, b, d = legs[k]
        seg = Segment(a, b, "down" if d < 0 else "up", "fine")
        mac = macd_force(df_macd, seg)
        sub = sublevel_force(closes, seg)
        ph0 = path_force(closes, seg, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=0)
        ph1 = path_force(closes, seg, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=1)
        # 同时报告各档 σ_t 下的 path-H0（看是否有任何比值能区分）
        ratios = {}
        for lbl, mult in [("时间主导", 0.1), ("平衡", 1.0), ("价格主导", 10.0)]:
            ratios[lbl] = path_force(
                closes, seg, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t * mult, dim=0
            )
        print(f"  #{k}: [{a}->{b}] Δ={d:.1f} span={b-a} | MACD={mac:.3f} "
              f"subH0={sub:.1f} pathH1={ph1:.3f}")
        print(f"       pathH0 各档: 时间主导={ratios['时间主导']:.3f} "
              f"平衡={ratios['平衡']:.3f} 价格主导={ratios['价格主导']:.3f}")


# ====================================================================
# Q2：段间相关性 + 背驰判定一致性（核心）
# ====================================================================


@dataclass(frozen=True, slots=True)
class SegForce:
    seg: Segment
    macd: float
    amp: float
    sub_h0: float
    path_h0: dict[str, float]   # 按归一化方法
    path_h1: dict[str, float]


def measure_segments(
    closes: list[float], df_macd: pd.DataFrame, segs: list[Segment], gs: GlobalScales
) -> list[SegForce]:
    out: list[SegForce] = []
    for s in segs:
        amp = abs(closes[s.b] - closes[s.a])
        ph0 = {m: path_force(closes, s, sigma_p=gs.sigma_p[m], sigma_t=gs.sigma_t, dim=0)
               for m in NORMS}
        ph1 = {m: path_force(closes, s, sigma_p=gs.sigma_p[m], sigma_t=gs.sigma_t, dim=1)
               for m in NORMS}
        out.append(SegForce(s, macd_force(df_macd, s), amp,
                            sublevel_force(closes, s), ph0, ph1))
    return out


def q2_correlations(forces: list[SegForce]):
    print("\n" + "=" * 78)
    print("【Q2a · 段间相关性】路径空间段力度 vs MACD面积（是否携带 MACD 信息？）")
    print("  对照基准：sublevel-H0 / 纯幅度 与 MACD 的相关。path-space 须显著更高才有独立价值。")
    print("=" * 78)
    macd = [f.macd for f in forces]
    print(f"  (有效段 n={len(forces)})")
    print(f"  {'度量':<22}{'Pearson':>9}{'Spearman':>10}")
    print("  " + "-" * 42)
    for name, vals in [
        ("纯幅度 amp", [f.amp for f in forces]),
        ("sublevel-H0", [f.sub_h0 for f in forces]),
        ("path-H0[std]", [f.path_h0["std"] for f in forces]),
        ("path-H0[mad]", [f.path_h0["mad"] for f in forces]),
        ("path-H0[range]", [f.path_h0["range"] for f in forces]),
        ("path-H1[std]", [f.path_h1["std"] for f in forces]),
        ("path-H1[mad]", [f.path_h1["mad"] for f in forces]),
    ]:
        print(f"  {name:<22}{pearson(vals, macd):>+9.3f}{spearman(vals, macd):>+10.3f}")


def divergence_verdict(force_a: float, force_c: float) -> bool:
    """C < A → 背驰（后段力度衰减）。"""
    return force_c < force_a


def q2_ac_agreement(closes, forces: list[SegForce]):
    print("\n" + "=" * 78)
    print("【Q2b · 段间背驰判定一致性】同向相邻下跌子段 (A,C)：各度量判 C<A 是否一致？")
    print("  缠论判据 = C段力度 < A段力度。path-space 与 MACD 判定一致 → 可作交叉验证。")
    print("=" * 78)
    # 按大趋势分组的下跌段，时间排序
    downs = [f for f in forces if f.seg.kind == "down"]
    by_parent: dict[str, list[SegForce]] = {}
    for f in downs:
        by_parent.setdefault(f.seg.parent, []).append(f)

    hdr = (f"  {'A段':>10}{'C段':>10}{'MACD':>6}{'subH0':>7}"
           f"{'pH0std':>8}{'pH0mad':>8}{'pH1std':>8}")
    n_pairs = 0
    agree_macd = {"sub": 0, "ph0std": 0, "ph0mad": 0, "ph1std": 0}
    for parent, segs in by_parent.items():
        segs.sort(key=lambda f: f.seg.a)
        if len(segs) < 2:
            continue
        print(f"\n  大趋势 {parent} 内 {len(segs)} 个下跌子段：")
        print(hdr)
        for i in range(len(segs) - 1):
            A, C = segs[i], segs[i + 1]
            v_macd = divergence_verdict(A.macd, C.macd)
            v_sub = divergence_verdict(A.sub_h0, C.sub_h0)
            v_p0s = divergence_verdict(A.path_h0["std"], C.path_h0["std"])
            v_p0m = divergence_verdict(A.path_h0["mad"], C.path_h0["mad"])
            v_p1s = divergence_verdict(A.path_h1["std"], C.path_h1["std"])
            mk = lambda v: "背驰" if v else " — "  # noqa: E731
            print(f"  [{A.seg.a:>3}->{A.seg.b:>3}][{C.seg.a:>3}->{C.seg.b:>3}]"
                  f"{mk(v_macd):>6}{mk(v_sub):>7}{mk(v_p0s):>8}{mk(v_p0m):>8}{mk(v_p1s):>8}")
            n_pairs += 1
            agree_macd["sub"] += v_sub == v_macd
            agree_macd["ph0std"] += v_p0s == v_macd
            agree_macd["ph0mad"] += v_p0m == v_macd
            agree_macd["ph1std"] += v_p1s == v_macd
    if n_pairs:
        print(f"\n  与 MACD 判定一致率（{n_pairs} 对）：")
        for k, v in agree_macd.items():
            print(f"    {k:>8}: {v}/{n_pairs} = {v/n_pairs:.0%}")
    else:
        print("  ⚠ 无 ≥2 下跌子段的大趋势——无 A/C 对（L2 样本边界）。")


# ====================================================================
# Q3：σ_p/σ_t 比值敏感度（幅度保真 vs 时间信息的权衡）
# ====================================================================


def q3_aspect_sweep(closes, df_macd, segs: list[Segment], gs: GlobalScales):
    print("\n" + "=" * 78)
    print("【Q3 · σ 比值权衡】扫 σ_t（时间轴权重）：幅度保真 vs 时间信息能否兼得？")
    print("  σ_t 大→价格主导(≈时间盲 sublevel)；σ_t 小→时间主导(幅度被冲淡)。")
    print("  报告 path-H0[std] 与 (a)纯幅度 (b)MACD 的相关——理想信号应低幅度相关、高MACD相关。")
    print("=" * 78)
    downs = [s for s in segs if s.kind == "down"]
    if len(downs) < 2:
        print("  ⚠ 下跌子段 < 2，无法做比值敏感度相关（L2 样本边界）。")
        return
    macd = [macd_force(df_macd, s) for s in downs]
    amp = [abs(closes[s.b] - closes[s.a]) for s in downs]
    base = gs.sigma_t  # balanced
    print(f"  {'σ_t 档位':<18}{'中位(Δp步/Δt步)':>16}{'corr(pH0,幅度)':>15}{'corr(pH0,MACD)':>15}")
    print("  " + "-" * 62)
    for label, mult in [("时间主导 0.1×", 0.1), ("平衡 1×", 1.0),
                        ("价格主导 10×", 10.0), ("极端价格 100×", 100.0)]:
        st = base * mult
        ph0 = [path_force(closes, s, sigma_p=gs.sigma_p["std"], sigma_t=st, dim=0)
               for s in downs]
        # 诊断该档位下 价格步/时间步 的中位比（>1 价格主导）
        ratios = []
        for s in downs:
            seg = closes[s.a : s.b + 1]
            dp = np.abs(np.diff(seg)) / gs.sigma_p["std"]
            dt = 1.0 / st
            ratios.append(float(np.median(dp) / dt))
        print(f"  {label:<18}{np.median(ratios):>16.2f}"
              f"{pearson(ph0, amp):>+15.3f}{pearson(ph0, macd):>+15.3f}")


# ====================================================================
# Q4：真假买点（段级路径空间判据替代 dP/dt）
# ====================================================================


def q4_real_vs_fake(closes, df_macd, gs: GlobalScales, coarse_legs):
    print("\n" + "=" * 78)
    print("【Q4 · 真假买点】真底 idx23（单段赶底）vs 末端底 idx298（多段下跌）")
    print("  判据：底所在大下跌内部，末子段(C) 力度 < 首子段(A) → 段间背驰 → 真底候选。")
    print("=" * 78)
    for h, lo in coarse_legs:
        legs = medium_down_legs(closes, h, lo, pct=0.06)
        downs = [s for s in legs if s.kind == "down"]
        print(f"\n  大下跌 [{h}->{lo}] {closes[h]:.0f}->{closes[lo]:.0f}: "
              f"{len(downs)} 个中级别下跌子段")
        if len(downs) < 2:
            print(f"    → 单段下跌，无 A/C 段间结构。底={'真底(已确认反转)' if lo==23 else '末端底'}。")
            print("      含义：此底不是'多段背驰底'，是单推赶底——段间判据在此不适用。")
            continue
        downs.sort(key=lambda s: s.a)
        A, C = downs[0], downs[-1]
        for nm, lbl in [("macd", "MACD面积"), ("sub", "subH0"),
                        ("p0", "path-H0std"), ("p1", "path-H1std")]:
            if nm == "macd":
                fa, fc = macd_force(df_macd, A), macd_force(df_macd, C)
            elif nm == "sub":
                fa, fc = sublevel_force(closes, A), sublevel_force(closes, C)
            elif nm == "p0":
                fa = path_force(closes, A, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=0)
                fc = path_force(closes, C, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=0)
            else:
                fa = path_force(closes, A, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=1)
                fc = path_force(closes, C, sigma_p=gs.sigma_p["std"], sigma_t=gs.sigma_t, dim=1)
            verdict = "背驰(真底信号)" if fc < fa else "无背驰(力度未衰)"
            print(f"    {lbl:<12} A=[{A.a}->{A.b}]={fa:.3f} "
                  f"C=[{C.a}->{C.b}]={fc:.3f} → {verdict}")


# ====================================================================
# 主流程
# ====================================================================


def find_coarse_down_legs(closes: list[float], pct: float):
    cz = zigzag(closes, pct)
    return [(cz[i - 1].idx, cz[i].idx)
            for i in range(1, len(cz))
            if cz[i - 1].kind == "H" and cz[i].kind == "L"]


def main():
    ofile = sys.argv[1] if len(sys.argv) > 1 else "daily_ohlcv.json"
    blob = json.loads((DATA / ofile).read_text())
    bars = blob["bars"]
    closes = [float(b["close"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    df_macd = compute_macd(pd.DataFrame({"close": closes}))
    gs = compute_global_scales(closes)

    print("#" * 78)
    print(f"# 腾讯 700 {ofile} — 段间路径空间 (t,p) Rips 持续同调 L2 实测（走势级）")
    print(f"# n={len(closes)} | 全局 σ_p[std]={gs.sigma_p['std']:.2f} "
          f"σ_t[balanced]={gs.sigma_t:.2f}")
    print("#" * 78)

    # 大下跌段
    coarse = find_coarse_down_legs(closes, pct=0.12)
    print(f"大下跌段(coarse 12%): {[(h, lo) for h, lo in coarse]}")

    # 收集所有大趋势内的中级别子段（用于 Q2 相关性 + 背驰一致性）
    all_segs: list[Segment] = []
    for h, lo in coarse:
        all_segs.extend(medium_down_legs(closes, h, lo, pct=0.06))
    # 大上涨内部也分段（增加段间相关性样本）
    cz = zigzag(closes, 0.12)
    for i in range(1, len(cz)):
        if cz[i - 1].kind == "L" and cz[i].kind == "H":
            all_segs.extend(medium_down_legs(closes, cz[i - 1].idx, cz[i].idx, pct=0.06))
    print(f"中级别子段(medium 6%): 共 {len(all_segs)} 段 "
          f"({sum(s.kind=='down' for s in all_segs)} 下跌 / "
          f"{sum(s.kind=='up' for s in all_segs)} 上涨)\n")

    q1_decisive_case(closes, highs, lows, df_macd, gs)
    forces = measure_segments(closes, df_macd, all_segs, gs)
    q2_correlations(forces)
    q2_ac_agreement(closes, forces)
    q3_aspect_sweep(closes, df_macd, all_segs, gs)
    q4_real_vs_fake(closes, df_macd, gs, coarse)

    print("\n" + "#" * 78)
    print("# 结论由上述 L2 数据判定。否定性结果同样有效（231号有效域规则）。")
    print("#" * 78)


if __name__ == "__main__":
    main()
