"""腾讯 700 — 决定性实验：MACD 残差究竟来自跨笔记忆还是笔内路径？

研究位置（2026-05-24 编排者）
----------------------------
前序 L2 结论（已复现）：log-log 回归 log(MACD)=c+a·log(pers)+b·log(span) 的
R² 仅 0.40–0.65——(persistence, span) 只能解释 MACD 面积方差的 40–65%。
commit 4cee96d 把残差归因为「MACD 跨笔 EMA 递归动量记忆」。但这是**断言**，
未经实验。本脚本做决定性检验。

机制
----
MACD 用 ewm(adjust=False) = IIR 滤波器：每笔起点的 EMA 初值由该笔**之前**的
全部价格历史指数加权决定。PH（H0 persistence）只看笔内价格序列。两者
information set 不同：
  全局 MACD 面积 = f(笔内价格, 笔前历史)
  段内 MACD 面积 = f(笔内价格)              ← 与 PH 信息集对齐
  PH (pers, span) = g(笔内价格)

隔离方法：对截取的笔内序列 closes[r0:r1+1] **独立**调用 compute_macd，
adjust=False 使 EMA 从段首自然初始化（EMA[0]=close[r0]），严格消除跨笔记忆。

> 短笔（n<26）的 EMA 未充分预热——这不是 bug，正是「段内无历史」的正确表示。
> 段内本来就没有 26 根可供预热，强行预热就是把跨笔历史偷渡回来。

两个对立假说
------------
H_mem（动量记忆假说）：残差 = 跨笔记忆。
  预测：段内重置后 (pers, span) → 段内 MACD 的 R² 应 → ~1，
        且 全局MACD 与 段内MACD 的差异 = 记忆贡献，应显著。
H_path（笔内路径假说）：残差 = 笔内路径几何（MACD 面积对路径形状敏感，
  persistence 只对端点+prominence 敏感，二者在段内就不可互相决定）。
  预测：段内重置后 R² 与全局几乎不变（仍 ~0.5）。

判据
----
- 段内 R² 显著高于全局 R²（→0.85+）   → H_mem 成立 → 段内 MACD 可被 PH 扬弃，
                                          但全局 MACD 不可（记忆是 PH 缺的维度）。
- 段内 R² ≈ 全局 R²（仍 ~0.5）         → H_mem 被**否证** → 残差是笔内路径几何，
                                          PH 与 MACD 在段内就不可互替。

认识论等级
----------
- 度量/回归：L0（纯算法）。
- 腾讯 700 真实日线+30分钟读出：L2（单标的多时段，可否证）。否定性结果优先。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_fractal import fractals_from_merged  # noqa: E402
from newchan.a_inclusion import merge_inclusion  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_persistence_barcode import barcode_from_prices  # noqa: E402
from newchan.a_stroke import strokes_from_fractals  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"


def build_df(raw: list[dict]) -> pd.DataFrame:
    ts = [datetime.fromtimestamp(b["time"], tz=timezone.utc) for b in raw]
    return pd.DataFrame(
        {
            "open": [float(b["open"]) for b in raw],
            "high": [float(b["high"]) for b in raw],
            "low": [float(b["low"]) for b in raw],
            "close": [float(b["close"]) for b in raw],
        },
        index=pd.DatetimeIndex(ts, name="time"),
    )


def strokes_of(df: pd.DataFrame):
    df_merged, m2r = merge_inclusion(df)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(df_merged, fractals, mode="new", merged_to_raw=m2r)
    return strokes, m2r


def raw_span(stroke, m2r) -> tuple[int, int]:
    r0, r1 = m2r[stroke.i0][0], m2r[stroke.i1][1]
    return (r0, r1) if r0 <= r1 else (r1, r0)


def segment_macd_area(df_seg: pd.DataFrame, direction: str) -> float:
    """段内重置 MACD 面积：对笔内序列独立跑 MACD（EMA 从段首初始化）。

    adjust=False 使 EMA[0]=close[r0]，从段首自然初始化——严格消除跨笔记忆。
    返回该笔方向上的 |面积|（下跌取负面积绝对值，上涨取正面积）。
    """
    df_macd = compute_macd(df_seg)
    hist = df_macd["hist"]
    pos = float(hist.clip(lower=0).sum())
    neg = float(hist.clip(upper=0).sum())
    return abs(neg) if direction == "down" else abs(pos)


def collect(name: str, raw: list[dict]) -> list[dict]:
    df = build_df(raw)
    closes = df["close"].tolist()
    df_macd_global = compute_macd(df)  # 全局 EMA（携带跨笔记忆）
    strokes, m2r = strokes_of(df)
    feats = []
    for s in strokes:
        r0, r1 = raw_span(s, m2r)
        seg = closes[r0 : r1 + 1]
        span = len(seg)
        if span < 2:
            continue
        bc = barcode_from_prices(seg, maxdim=0)
        # 全局 MACD 面积（原始，携带笔前历史）
        area_g = macd_area_for_range(df_macd_global, r0, r1)
        macd_global = abs(area_g["area_neg"]) if s.direction == "down" else abs(area_g["area_pos"])
        # 段内 MACD 面积（重置 EMA，仅笔内信息）
        macd_seg = segment_macd_area(df.iloc[r0 : r1 + 1], s.direction)
        feats.append(
            {
                "tf": name,
                "dir": s.direction,
                "span": span,
                "max_h0": bc.max_persistence(0),
                "macd_global": macd_global,
                "macd_seg": macd_seg,
            }
        )
    return feats


def pearson(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def loglog_fit(pers, span, macd):
    """log(macd)=c+a·log(pers)+b·log(span)。返回 (a,b,R²,n)。"""
    pers, span, macd = map(lambda v: np.asarray(v, float), (pers, span, macd))
    mask = (pers > 0) & (span > 0) & (macd > 1e-6)
    p, s, m = pers[mask], span[mask], macd[mask]
    if len(m) < 4:
        return None
    X = np.column_stack([np.ones_like(p), np.log(p), np.log(s)])
    y = np.log(m)
    coef, *_ = np.linalg.lstsq(X, y, rcond=None)
    pred = X @ coef
    ss_res = np.sum((y - pred) ** 2)
    ss_tot = np.sum((y - y.mean()) ** 2)
    r2 = 1 - ss_res / ss_tot if ss_tot > 0 else float("nan")
    return (coef[1], coef[2], r2, len(m))


def report(label: str, feats: list[dict]) -> None:
    pers = np.array([f["max_h0"] for f in feats], float)
    span = np.array([f["span"] for f in feats], float)
    mg = np.array([f["macd_global"] for f in feats], float)
    ms = np.array([f["macd_seg"] for f in feats], float)

    print(f"\n{'='*78}\n【{label}】(n={len(feats)} 笔)\n{'='*78}")

    fit_g = loglog_fit(pers, span, mg)
    fit_s = loglog_fit(pers, span, ms)
    print("  PH (persistence, span) 对 MACD 面积的 log-log 解释力：")
    if fit_g:
        a, b, r2, n = fit_g
        print(f"    全局MACD(带跨笔记忆)  R²={r2:.3f}  a={a:.2f} b={b:.2f}  (n={n})")
    if fit_s:
        a, b, r2, n = fit_s
        print(f"    段内MACD(重置EMA)     R²={r2:.3f}  a={a:.2f} b={b:.2f}  (n={n})")
    if fit_g and fit_s:
        dr2 = fit_s[2] - fit_g[2]
        print(f"    ΔR² (段内−全局) = {dr2:+.3f}")
        if dr2 > 0.20:
            print("    → ★ 段内 R² 显著上升 → H_mem 成立：残差主要是跨笔记忆")
        elif abs(dr2) <= 0.20 and fit_s[2] < 0.80:
            print("    → ★ 段内 R² 未显著上升且仍 <0.80 → H_mem 被否证：残差是笔内路径几何")
        else:
            print("    → 段内 R² 上升但未达 ~1，记忆+路径混合贡献")

    # 记忆贡献的直接度量：全局 vs 段内 MACD 面积的差异
    corr_gs = pearson(mg, ms)
    rel_diff = np.abs(mg - ms) / (np.abs(mg) + 1e-9)
    print(f"\n  记忆贡献直接度量（全局MACD vs 段内MACD）：")
    print(f"    corr(全局, 段内) = {corr_gs:.3f}")
    print(f"    平均相对差 |全局−段内|/|全局| = {np.median(rel_diff):.1%} (中位数)")
    print(f"    → 差异越大说明跨笔记忆对 MACD 面积贡献越大")


def main() -> None:
    daily = json.loads((DATA / "daily_ohlcv.json").read_text())["bars"]
    m30 = json.loads((DATA / "m30_ohlcv.json").read_text())["bars"]

    print("#" * 78)
    print("# 腾讯 700 — 决定性实验：MACD 残差来源（跨笔记忆 vs 笔内路径）")
    print("#" * 78)

    f_daily = collect("日线", daily)
    f_m30 = collect("30分", m30)
    report("日线", f_daily)
    report("30分钟", f_m30)
    report("日线+30分钟 合并 (弱L3)", f_daily + f_m30)


if __name__ == "__main__":
    main()
