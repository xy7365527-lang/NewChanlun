"""腾讯 700 — PH 能否扬弃 MACD？时间修正 persistence 度量实测。

核心问题（2026-05-24 编排者）
----------------------------
MACD = 动量×时间积分（速度敏感）。H0 persistence ≈ 振幅（时间盲，239号）。
问：PH **加位置/时间维度**后能否完全平替 MACD？若 MACD 面积是 (persistence, span)
的确定函数（高 R²），则 MACD = PH 的降维投影，可被扬弃（aufheben）。

判据
----
- corr(时间修正persistence, MACD面积) > 0.9 → MACD 是 PH 投影。
- 幂律回归 log(MACD)=c+a·log(pers)+b·log(span) 的 R² 与指数 (a,b) 给出**实际**
  时间修正形式（不预设 √span）。用户假设 b=0.5（persistence×√span），实测验证。

PH 是否真有时间维度？
--------------------
有。merge-tree H0 的每个特征带 birth_idx/death_idx（局部极小→合并位置），
特征 span = |death_idx-birth_idx| 是**位置感知 PH 内生的时间量**，不是外加。
因此 (persistence, span) 二元组完全在 PH 框架内，不借用 MACD 任何东西。

认识论等级
----------
- 度量/回归：L0（纯算法）。
- 腾讯 700 日线读出：L2。日线+30分钟双时段交叉：弱 L3（同标的多时段）。
- 笔由仓库新笔引擎从对齐 OHLCV 推出。
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
from newchan.a_persistence_barcode import atr, barcode_from_prices  # noqa: E402
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


def raw_span(stroke, m2r):
    r0, r1 = m2r[stroke.i0][0], m2r[stroke.i1][1]
    return (r0, r1) if r0 <= r1 else (r1, r0)


def collect_features(name: str, raw: list[dict]) -> list[dict]:
    """每笔: persistence(max_h0,tot_h0)、span、velocity、MACD 面积。"""
    df = build_df(raw)
    closes = df["close"].tolist()
    df_macd = compute_macd(df)
    strokes, m2r = strokes_of(df)
    feats = []
    for s in strokes:
        r0, r1 = raw_span(s, m2r)
        seg = closes[r0 : r1 + 1]
        span = len(seg)
        if span < 2:
            continue
        bc = barcode_from_prices(seg, maxdim=0)
        max_h0 = bc.max_persistence(0)
        tot_h0 = bc.total_persistence(0)
        area = macd_area_for_range(df_macd, r0, r1)
        macd = abs(area["area_neg"]) if s.direction == "down" else abs(area["area_pos"])
        feats.append(
            {
                "tf": name,
                "dir": s.direction,
                "r0": r0,
                "r1": r1,
                "span": span,
                "amp": abs(seg[-1] - seg[0]),
                "max_h0": max_h0,
                "tot_h0": tot_h0,
                "velocity": max_h0 / span,
                "macd": macd,
            }
        )
    return feats


def pearson(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3 or np.std(x) == 0 or np.std(y) == 0:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def spearman(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3:
        return float("nan")
    rx = np.argsort(np.argsort(x))
    ry = np.argsort(np.argsort(y))
    return pearson(rx, ry)


def loglog_fit(pers, span, macd):
    """log(macd)=c+a·log(pers)+b·log(span)。返回 (c,a,b,R²,n)。"""
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
    return (coef[0], coef[1], coef[2], r2, len(m))


def report_corr(label: str, feats: list[dict]) -> None:
    macd = [f["macd"] for f in feats]
    span = np.array([f["span"] for f in feats], float)
    pers = np.array([f["max_h0"] for f in feats], float)
    tot = np.array([f["tot_h0"] for f in feats], float)
    # 候选时间修正度量
    cands = {
        "persistence(max_h0) 纯振幅": pers,
        "tot_h0(全diagram)": tot,
        "persistence×√span (用户假设)": pers * np.sqrt(span),
        "persistence×span": pers * span,
        "persistence/span (速度)": pers / span,
        "persistence²/span": pers**2 / span,
    }
    print(f"\n{'='*78}\n【{label}】MACD面积 vs 各 persistence 度量相关性  (n={len(feats)} 笔)\n{'='*78}")
    print(f"  {'度量':<30}{'Pearson':>10}{'Spearman':>10}  {'>0.9?':>6}")
    for cname, cv in cands.items():
        pe, sp = pearson(cv, macd), spearman(cv, macd)
        flag = "✓投影" if (not np.isnan(pe) and pe > 0.9) else ""
        print(f"  {cname:<30}{pe:>10.3f}{sp:>10.3f}  {flag:>6}")
    fit = loglog_fit(pers, span, macd)
    if fit:
        c, a, b, r2, n = fit
        print(f"\n  幂律回归 log(MACD)=c+a·log(pers)+b·log(span)  (n={n}, MACD>0)")
        print(f"    a(振幅指数)={a:.3f}  b(时间指数)={b:.3f}  截距={c:.3f}  R²={r2:.3f}")
        bestmetric = pers ** a * span ** b
        print(f"    → 最优时间修正: persistence^{a:.2f} × span^{b:.2f}  "
              f"Pearson={pearson(bestmetric, macd):.3f}")
        if abs(b - 0.5) < 0.2:
            print(f"    → 时间指数 b≈0.5：用户的 √span 假设接近成立")
        elif b < 0:
            print(f"    → ★ b<0：MACD 随耗时**减小**（速度敏感），√span(b=+0.5) 方向相反")
        else:
            print(f"    → b={b:.2f}≠0.5：实际时间修正非 √span")


def velocity_test(feats: list[dict]) -> None:
    """急跌/缓跌：相近振幅、不同 span 的笔，MACD 是否跟速度而 persistence 跟不上。"""
    print(f"\n{'='*78}\n【急跌/缓跌判别】persistence(振幅) 时间盲 vs MACD 速度敏感\n{'='*78}")
    # 按振幅分箱，箱内看 span 差异时 MACD 与 persistence 的变化
    fs = sorted(feats, key=lambda f: f["amp"])
    # 找振幅相近(±15%)但 span 差异最大的一对
    best = None
    for i in range(len(fs)):
        for j in range(i + 1, len(fs)):
            a, b = fs[i], fs[j]
            if a["amp"] <= 0:
                continue
            if abs(b["amp"] - a["amp"]) / a["amp"] <= 0.15:
                span_ratio = max(a["span"], b["span"]) / min(a["span"], b["span"])
                if best is None or span_ratio > best[0]:
                    best = (span_ratio, a, b)
    if not best:
        print("  无振幅相近样本对。")
        return
    _, a, b = best
    fast, slow = (a, b) if a["span"] < b["span"] else (b, a)
    print(f"  振幅相近的一对笔（控制振幅，比时间）:")
    print(f"    急 [{fast['r0']}:{fast['r1']}] {fast['tf']} span={fast['span']:>2} "
          f"振幅={fast['amp']:.1f} persistence={fast['max_h0']:.1f} MACD={fast['macd']:.2f} "
          f"速度={fast['velocity']:.2f}")
    print(f"    缓 [{slow['r0']}:{slow['r1']}] {slow['tf']} span={slow['span']:>2} "
          f"振幅={slow['amp']:.1f} persistence={slow['max_h0']:.1f} MACD={slow['macd']:.2f} "
          f"速度={slow['velocity']:.2f}")
    print(f"  → persistence 比值 急/缓 = {fast['max_h0']/slow['max_h0']:.2f}（≈1，时间盲，无法区分）")
    print(f"  → MACD 比值     急/缓 = {fast['macd']/max(slow['macd'],1e-6):.2f}"
          f"（≠1 → 区分急缓）")
    print(f"  → persistence/span 急/缓 = {fast['velocity']/slow['velocity']:.2f}"
          f"（恢复速度敏感性 ← PH 内生时间修正）")


def main() -> None:
    daily = json.loads((DATA / "daily_ohlcv.json").read_text())["bars"]
    m30 = json.loads((DATA / "m30_ohlcv.json").read_text())["bars"]

    print("#" * 78)
    print("# 腾讯 700 — PH 能否扬弃 MACD：时间修正 persistence 度量")
    print("#" * 78)

    f_daily = collect_features("日线", daily)
    f_m30 = collect_features("30分", m30)
    pooled = f_daily + f_m30

    report_corr("日线", f_daily)
    report_corr("30分钟", f_m30)
    report_corr("日线+30分钟 合并 (弱L3)", pooled)
    velocity_test(pooled)


if __name__ == "__main__":
    main()
