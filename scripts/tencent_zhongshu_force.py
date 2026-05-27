"""腾讯 700 — 中枢基线偏离积分（无参数力度）的 L2 经验检验。

研究位置
--------
新模块 `a_zhongshu_force` 提出无参数力度：force(段) = Σ|close(t) − M|，
M = (ZD+ZG)/2 = 中枢中间价（结构性 0 轴，替代 MACD 的 EMA 0 轴）。
本脚本在真实数据上检验它能否承担 §5 中 MACD 承担而 H0 不能的角色。

§5 决定性 case（来自 tencent_path_persistence.py 的 zigzag legs，本脚本用相同设置）：
  #33 缓跌 MACD面积=0.644，#37 急跌 MACD面积=13.467（差 ~20×），H0 几乎相同（72.6 vs 82.1）。
  问题：中枢基线偏离积分能否像 MACD 一样区分（~20×），还是像 H0 一样失败（~1×）？

三个对立预期（formalization-validity-domain：必须如实报告，不可调 M 直到"区分"）
------------------------------------------------------------------------------
E_match：偏离积分与 MACD 高相关、能区分急/缓 → 结构 0 轴可部分替代 EMA 0 轴。
E_fail ：偏离积分 ≈ 振幅×跨度（raw）或 ≈ 振幅（normalized），无法复刻 MACD 的
         速度敏感 → 否定性结果，**强化 §5 原因二**（移动 EMA 基线不可被静态结构基线替代）。
E_anti ：raw force 因缓跌跨度大反而 > 急跌，与 MACD **反向** → 更强否定。

无参数性（L0，同义反复，无需实测）：偏离积分不读 (fast,slow,signal) → 对 MACD 参数
扰动 100% 不变。本脚本对照展示 MACD 背驰判定随参数翻转，而偏离积分判定恒定（522号鲁棒性）。

认识论等级
----------
- 偏离积分 / MACD面积 / H0 / 相关 / 一致率：L0（纯算法）。
- 腾讯 700 真实日线读出：L2（单标的单时段，可否证）。否定性结果优先（231号）。
- 中枢基线 M 的取法（段间整理区中点）= 结构性 0 轴的一种 L0 实现；M 取法不同结论可能变，
  此为边界条件（结果包要素3），脚本同时用 PH detect_zhongshu 的结构中枢做交叉核对。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import atr, barcode_from_prices  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu  # noqa: E402
from newchan.a_zhongshu_force import (  # noqa: E402
    baseline_deviation_force,
    compare_force,
    zhongshu_midprice,
)

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"


# ====================================================================
# 数据 + zigzag legs（与 §5 / tencent_path_persistence.py 完全相同）
# ====================================================================


def load_ohlcv(ofile: str):
    bars = json.loads((DATA / ofile).read_text())["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


def zigzag_legs(closes, threshold: float):
    """与 tencent_path_persistence.py 逐字一致——保证 #33/#37 索引可比。"""
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


# ====================================================================
# 中枢基线 M：段间整理区中点（结构性 0 轴，无参数）
# ====================================================================


def consolidation_midprice(closes, lo: int, hi: int) -> float:
    """两推动段之间整理区 [lo, hi] 的几何中点 M = (max + min) / 2。

    这是 ZD/ZG 的 zigzag 实现：整理区价格上下界的中点 = 中枢中间价的结构性表达。
    无参数：lo/hi 由相邻同向段的端点决定，max/min 不含参数。
    """
    if hi < lo:
        lo, hi = hi, lo
    seg = closes[lo : hi + 1]
    if not seg:
        return float(closes[lo])
    zg, zd = max(seg), min(seg)  # 整理区上界/下界
    return zhongshu_midprice(zd, zg)


def leg_metrics(closes, df_macd, a: int, b: int, d: float, midprice: float) -> dict:
    """单段三种力度：中枢基线偏离积分 / MACD面积 / H0 total persistence。"""
    seg = closes[a : b + 1]
    zf = baseline_deviation_force(closes, midprice, i0=a, i1=b)
    area = macd_area_for_range(df_macd, a, b)
    macd = abs(area["area_neg"]) if d < 0 else abs(area["area_pos"])
    h0 = barcode_from_prices(seg, maxdim=0).total_persistence(0) if len(seg) >= 2 else 0.0
    return {
        "force": zf.force,
        "force_norm": zf.force_normalized,
        "force_signed": zf.force_signed,
        "midprice": midprice,
        "macd": macd,
        "h0": h0,
        "amp": abs(d),
        "span": b - a + 1,
    }


def pearson(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def spearman(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3:
        return float("nan")
    rx, ry = pd.Series(x).rank().to_numpy(), pd.Series(y).rank().to_numpy()
    return pearson(rx, ry)


# ====================================================================
# 主流程
# ====================================================================


def main() -> None:
    highs, lows, closes = load_ohlcv("daily_ohlcv.json")
    tau = atr(highs, lows, closes, period=14)
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    df_macd = compute_macd(pd.DataFrame({"close": closes}))

    print("#" * 78)
    print("# 腾讯 700 日线 — 中枢基线偏离积分（无参数力度）L2 检验")
    print(f"# n={len(closes)}  ATR={tau:.3f}  zigzag 笔数={len(legs)}")
    print("#" * 78)

    # ---- 结构中枢交叉核对（PH detect_zhongshu）----
    bc = OnlineMergeTree.from_prices(closes)
    phz = detect_zhongshu(bc.all_bars, noise_floor=tau)
    print(f"\n[交叉核对] PH detect_zhongshu 在日线识别中枢 {len(phz)} 个"
          f"（结构性 ZD/ZG，与 zigzag 整理区互为印证）")
    for z in phz[:6]:
        print(f"    中枢 [bar {z.lo}→{z.hi}] ZD={z.zd:.1f} ZG={z.zg:.1f} "
              f"M={(z.zd+z.zg)/2:.1f} 宽={z.width:.1f} 成员={z.n_members}")

    # ====================================================================
    # P1 · §5 决定性 case：#33（缓跌）vs #37（急跌）
    # ====================================================================
    print("\n" + "=" * 78)
    print("【P1 · §5 决定性】#33（缓跌,MACD≈0.6）vs #37（急跌,MACD≈13.5）")
    print("     中枢基线偏离积分能否区分（~20×=像MACD）还是失败（~1×=像H0）？")
    print("=" * 78)

    if len(legs) > 37:
        a33, b33, d33 = legs[33]
        a37, b37, d37 = legs[37]
        # 第24课共享基线：#33 与 #37 之间整理区的中点 = 中枢 M（结构性 0 轴）
        M_shared = consolidation_midprice(closes, b33, a37)
        m33 = leg_metrics(closes, df_macd, a33, b33, d33, M_shared)
        m37 = leg_metrics(closes, df_macd, a37, b37, d37, M_shared)

        print(f"\n  #33: bar[{a33}:{b33}] 跌幅={m33['amp']:.1f} span={m33['span']} "
              f"close端点 {closes[a33]:.1f}→{closes[b33]:.1f}")
        print(f"  #37: bar[{a37}:{b37}] 跌幅={m37['amp']:.1f} span={m37['span']} "
              f"close端点 {closes[a37]:.1f}→{closes[b37]:.1f}")
        print(f"  共享中枢基线 M = {M_shared:.2f}（#33末→#37首 整理区中点）")

        def _ratio(v37, v33):
            return v37 / v33 if v33 else float("inf")

        print(f"\n  {'度量':<22}{'#33':>10}{'#37':>10}{'比值#37/#33':>14}{'方向':>16}")
        rows = [
            ("MACD面积(基准)", m33["macd"], m37["macd"]),
            ("中枢偏离积分 force", m33["force"], m37["force"]),
            ("  归一化 force_norm", m33["force_norm"], m37["force_norm"]),
            ("H0 totpers(对照)", m33["h0"], m37["h0"]),
            ("纯幅度 amp(对照)", m33["amp"], m37["amp"]),
        ]
        macd_ratio = _ratio(m37["macd"], m33["macd"])
        for name, v33, v37 in rows:
            r = _ratio(v37, v33)
            if name.startswith("MACD"):
                flag = "← 基准(急>缓)"
            else:
                same = (v37 > v33) == (m37["macd"] > m33["macd"])
                flag = "✓同向MACD" if same else "✗反向MACD"
            print(f"  {name:<22}{v33:>10.3f}{v37:>10.3f}{r:>13.2f}×{flag:>16}")

        print(f"\n  判读：MACD 比值={macd_ratio:.1f}×（急跌动量远大于缓跌）。")
        fr = _ratio(m37["force"], m33["force"])
        nr = _ratio(m37["force_norm"], m33["force_norm"])
        if fr > 3 and (m37["force"] > m33["force"]):
            print(f"    → force 比值={fr:.1f}× 且同向 MACD：E_match 倾向（结构0轴部分捕获急/缓）")
        elif (m37["force"] < m33["force"]):
            print(f"    → force 比值={fr:.2f}× **反向** MACD：E_anti（缓跌跨度大→积分反而大）")
        else:
            print(f"    → force 比值={fr:.2f}× 同向但弱：E_fail/部分（远不及 MACD 的 {macd_ratio:.0f}×）")
        print(f"    → force_norm 比值={nr:.2f}×（剔除跨度后，≈振幅比，检验是否退化为 H0 类）")
    else:
        print(f"  legs 不足 38 段（实际 {len(legs)}），§5 索引不可用——跳过 P1。")

    # ====================================================================
    # P2 · 跨全部下跌段：三种力度 vs MACD 面积的相关性
    # ====================================================================
    print("\n" + "=" * 78)
    print("【P2 · 相关性】跨全部段，中枢偏离积分 vs MACD 面积（携带动量信息吗？）")
    print("=" * 78)
    # 每个下跌段：M = 其前一个整理区中点（= 该段离开的中枢）
    rows = []
    down_idx = [k for k, (_, _, d) in enumerate(legs) if d < 0]
    for k in down_idx:
        a, b, d = legs[k]
        if b - a < 1:
            continue
        # 前一整理区 = 上一段的区间（k-1），无前段则用本段起点邻域
        if k >= 1:
            pa, pb, _ = legs[k - 1]
            M = consolidation_midprice(closes, pa, a)
        else:
            M = consolidation_midprice(closes, a, a)
        m = leg_metrics(closes, df_macd, a, b, d, M)
        m["k"] = k
        rows.append(m)
    R = pd.DataFrame(rows)
    R = R[R["macd"] > 1e-6]
    print(f"  (有效下跌段 n={len(R)})")
    if len(R) >= 3:
        print(f"    中枢偏离积分 force      vs MACD: Pearson={pearson(R['force'], R['macd']):+.3f}"
              f"  Spearman={spearman(R['force'], R['macd']):+.3f}")
        print(f"    归一化 force_norm       vs MACD: Pearson={pearson(R['force_norm'], R['macd']):+.3f}"
              f"  Spearman={spearman(R['force_norm'], R['macd']):+.3f}")
        print(f"    [对照] H0 totpers       vs MACD: Pearson={pearson(R['h0'], R['macd']):+.3f}"
              f"  Spearman={spearman(R['h0'], R['macd']):+.3f}")
        print(f"    [对照] 纯幅度 amp        vs MACD: Pearson={pearson(R['amp'], R['macd']):+.3f}"
              f"  Spearman={spearman(R['amp'], R['macd']):+.3f}")
        print(f"    [对照] 跨度 span         vs MACD: Pearson={pearson(R['span'], R['macd']):+.3f}")
        print("\n  判读：若 force 相关 ≈ amp/h0 相关（而非更高），说明 force 未携带 MACD 独有的")
        print("        动量-速度信息——结构基线偏离积分 ≈ 振幅类度量，**不能平替 MACD**（强化 §5）。")

    # ====================================================================
    # P3 · 背驰判定一致率（相邻下跌段 A/C 对，共享中间整理区 M）
    # ====================================================================
    print("\n" + "=" * 78)
    print("【P3 · 背驰一致率】相邻下跌段 A/C：偏离积分背驰 vs MACD面积背驰 vs H0背驰")
    print("=" * 78)
    pairs = list(zip(down_idx[:-1], down_idx[1:]))
    n_force = n_norm = n_h0 = total = 0
    print(f"  {'A→C':<10}{'M':>8}{'force判':>8}{'norm判':>8}{'H0判':>7}{'MACD判':>8}")
    for ka, kc in pairs:
        aa, ab, da = legs[ka]
        ca, cb, dc = legs[kc]
        M = consolidation_midprice(closes, ab, ca)  # A末→C首 整理区
        fa = baseline_deviation_force(closes, M, i0=aa, i1=ab)
        fc = baseline_deviation_force(closes, M, i0=ca, i1=cb)
        div_force = compare_force(fa, fc, field="force").is_divergent
        div_norm = compare_force(fa, fc, field="force_normalized").is_divergent
        # H0 背驰：C 段 H0 < A 段
        ha = barcode_from_prices(closes[aa:ab + 1], maxdim=0).total_persistence(0)
        hc = barcode_from_prices(closes[ca:cb + 1], maxdim=0).total_persistence(0)
        div_h0 = hc < ha and ha > 0
        # MACD 背驰（基准）
        ma = abs(macd_area_for_range(df_macd, aa, ab)["area_neg"])
        mc = abs(macd_area_for_range(df_macd, ca, cb)["area_neg"])
        div_macd = mc < ma and ma > 0
        total += 1
        n_force += div_force == div_macd
        n_norm += div_norm == div_macd
        n_h0 += div_h0 == div_macd
        print(f"  #{ka}→#{kc:<5}{M:>8.1f}{str(div_force):>8}{str(div_norm):>8}"
              f"{str(div_h0):>7}{str(div_macd):>8}")
    if total:
        print(f"\n  与 MACD 面积背驰判定的一致率（n={total} 对）：")
        print(f"    中枢偏离积分 force : {n_force}/{total} = {n_force/total:.0%}")
        print(f"    归一化 force_norm  : {n_norm}/{total} = {n_norm/total:.0%}")
        print(f"    [对照] H0          : {n_h0}/{total} = {n_h0/total:.0%}")

    # ====================================================================
    # P4 · 参数鲁棒性对照（偏离积分无参数 → 100% 不变；MACD 随参数翻转）
    # ====================================================================
    print("\n" + "=" * 78)
    print("【P4 · 参数鲁棒性】MACD背驰判定随(fast,slow,signal)扰动 vs 偏离积分恒定")
    print("=" * 78)
    param_sets = [(12, 26, 9), (8, 21, 5), (16, 32, 11), (10, 24, 7), (6, 19, 4)]
    if pairs:
        ka, kc = pairs[len(pairs) // 2]  # 取中间一对做演示
        aa, ab, _ = legs[ka]
        ca, cb, _ = legs[kc]
        M = consolidation_midprice(closes, ab, ca)
        fa = baseline_deviation_force(closes, M, i0=aa, i1=ab)
        fc = baseline_deviation_force(closes, M, i0=ca, i1=cb)
        force_verdict = compare_force(fa, fc).is_divergent  # 默认 force_normalized（523号）
        print(f"  演示对 #{ka}→#{kc}（M={M:.1f}）：")
        macd_verdicts = []
        for f, s, sg in param_sets:
            dfm = compute_macd(pd.DataFrame({"close": closes}), fast=f, slow=s, signal=sg)
            ma = abs(macd_area_for_range(dfm, aa, ab)["area_neg"])
            mc = abs(macd_area_for_range(dfm, ca, cb)["area_neg"])
            v = mc < ma and ma > 0
            macd_verdicts.append(v)
            print(f"    MACD({f:>2},{s:>2},{sg:>2}) 背驰判定 = {v}")
        flips = len(set(macd_verdicts))
        print(f"\n    → MACD 在 5 组参数下背驰判定取值数 = {flips}"
              f"（{'★ 翻转！参数敏感' if flips > 1 else '本对恰好稳定'}）")
        print(f"    → 中枢偏离积分背驰判定 = {force_verdict}（**5 组参数下恒定**——根本不读 MACD 参数）")
        print(f"    → 无参数性是 L0（同义反复）：522号'参数鲁棒'对偏离积分天然 100% 成立。")

    print("\n" + "=" * 78)
    print("结论由上述 L2 数据判定（见 stdout）。否定性结果同样有效且优先（231号）。")
    print("=" * 78)


if __name__ == "__main__":
    main()
