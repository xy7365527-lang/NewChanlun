"""腾讯 700 日线 — Wasserstein diagram 丰富度背驰 vs MACD vs H0 三度量对照。

研究问题（2026-05-24）
--------------------
CC 此前结论：用单个 bar 的 H0 persistence 替代 MACD 是降级（H0 总持续度 ≈ 振幅
∈ ker(D)，239号）。新思路：不用单一标量，而用**整段笔的 persistence diagram 形状**：
- W(diagram, 噪声基线)  = diagram 到对角线/ATR-trivial 的 Wasserstein-1 = 总持续度
- W(A_diagram, C_diagram) = 两段 diagram 直接距离 = 结构重组程度
- 丰富度 = 总持续度 − 主导 bar（= 主运动之外的内部子结构）= 真正独立于振幅的量

度量对象（必读，validity-domain 规则）
--------------------------------------
| 度量            | 捕捉什么            | 与 ker(D)/MACD 关系                    |
|----------------|--------------------|---------------------------------------|
| 幅度 |p1-p0|     | 端点位移            | ∈ ker(D)，时间盲                       |
| H0 总持续度 W_noise | 全部摆动 prominence 之和 | 单调笔 ≈ 幅度；有子结构则 > 幅度  |
| H0 主导 bar     | 最大摆动            | ≈ 幅度（单调笔=全幅）                   |
| H0 丰富度=总−主导 | 主运动之外的子摆动   | **独立于振幅** ← 新信号                 |
| H0 活跃 bar 数  | 超 ATR 的子结构计数 | **独立于振幅**，纯结构计数 ← 新信号      |
| H1 中枢 loop    | 相空间往返几何       | **超出幅度**（振荡几何）← 新信号         |
| MACD 面积       | 动量×时间积分        | 速度/耗时敏感（幅度+时间）              |

诚实预判（待真实数据否证）
------------------------
单根日线笔近似单调 → H0 总持续度（用户的 W_noise）仍由单一全幅 bar 主导 ≈ 幅度，
**不会**超出 MACD 的幅度分量。真正 MACD 看不到的信息在「丰富度/活跃 bar 数/H1」，
且这些只在「段内有子结构」时才非平凡——即趋势级（含中枢）而非单根笔级。

认识论等级
----------
- 三度量计算：L0（纯算法）。
- 腾讯 700 真实日线读出：L2（单标的单时段，可否证）。
- 笔由仓库新笔引擎（分型+包含+笔）从对齐 300 根日线推出，非一禅指标导出
  （一禅 label 导出的 price 字段与价格图不对齐，不可用作笔端点）。
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
from newchan.a_divergence_topo import topo_divergence  # noqa: E402
from newchan.a_stroke import strokes_from_fractals  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"


def load_raw() -> list[dict]:
    return json.loads((DATA / "daily_ohlcv.json").read_text())["bars"]


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


def compute_strokes(df: pd.DataFrame):
    """新笔引擎纯函数管线 → strokes + merged_to_raw 映射。"""
    df_merged, m2r = merge_inclusion(df)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode="new", merged_to_raw=m2r
    )
    return strokes, m2r


def raw_span(stroke, m2r) -> tuple[int, int]:
    """笔的 merged 端点 → 原始 bar 范围 [r0, r1]。"""
    r0 = m2r[stroke.i0][0]
    r1 = m2r[stroke.i1][1]
    return (r0, r1) if r0 <= r1 else (r1, r0)


def seg_metrics(closes_seg, df_macd, r0, r1, direction, noise) -> dict:
    """一段笔的全部度量。"""
    bc = barcode_from_prices(closes_seg, maxdim=1)
    h0 = bc.by_dimension(0)
    max_h0 = bc.max_persistence(0)
    tot_h0 = bc.total_persistence(0)  # = W(diagram, 对角线) = 用户的 W_noise
    n_active = sum(1 for b in h0 if b.persistence > noise)  # 超 ATR 的子结构数
    secondary = tot_h0 - max_h0  # 主运动之外的丰富度（独立于振幅）
    tot_h1 = bc.total_persistence(1)  # 中枢 loop 几何
    area = macd_area_for_range(df_macd, r0, r1)
    macd_force = abs(area["area_neg"]) if direction == "down" else abs(area["area_pos"])
    return {
        "amp": abs(closes_seg[-1] - closes_seg[0]),
        "max_h0": max_h0,
        "tot_h0": tot_h0,
        "n_active": n_active,
        "secondary": secondary,
        "tot_h1": tot_h1,
        "macd": macd_force,
        "n_bars": len(closes_seg),
    }


def verdict(a_val, c_val) -> str:
    return "★背驰" if c_val < a_val else "不背驰"


def main() -> None:
    raw = load_raw()
    df = build_df(raw)
    closes = df["close"].tolist()
    highs = df["high"].tolist()
    lows = df["low"].tolist()
    noise = atr(highs, lows, closes, period=14)
    df_macd = compute_macd(df)

    strokes, m2r = compute_strokes(df)

    print("#" * 78)
    print("# 腾讯 700（HKEX:700）日线 — Wasserstein diagram 丰富度 vs MACD vs H0")
    print(f"# n={len(closes)} 根日线  价 {min(closes):.1f}-{max(closes):.1f}  "
          f"ATR(14)={noise:.2f}  笔数={len(strokes)}")
    print(f"# 时段 {raw[0]['time']}→{raw[-1]['time']}  末价 {closes[-1]:.1f}")
    print("#" * 78)

    # ── 逐笔度量（最近 10 笔）──
    rows = []
    for s in strokes:
        r0, r1 = raw_span(s, m2r)
        seg = closes[r0 : r1 + 1]
        if len(seg) < 2:
            continue
        m = seg_metrics(seg, df_macd, r0, r1, s.direction, noise)
        rows.append((s, r0, r1, m))

    print(f"\n{'='*78}\n【逐笔三度量】（拓扑自笔引擎，新笔模式；最近 10 笔）\n{'='*78}")
    print(f"  {'笔':<4}{'向':<4}{'bar区间':<13}{'根数':>4}{'幅度':>8}"
          f"{'MACD面积':>10}{'H0总(W噪)':>11}{'H0主导':>9}{'丰富度':>8}{'活跃':>5}{'H1环':>8}")
    start = max(0, len(rows) - 10)
    for idx in range(start, len(rows)):
        s, r0, r1, m = rows[idx]
        d = "下跌" if s.direction == "down" else "上涨"
        print(f"  #{idx:<3}{d:<4}"
              f"[{r0:>3}:{r1:<3}]  {m['n_bars']:>4}{m['amp']:>8.1f}"
              f"{m['macd']:>10.2f}{m['tot_h0']:>11.1f}{m['max_h0']:>9.1f}"
              f"{m['secondary']:>8.1f}{m['n_active']:>5}{m['tot_h1']:>8.2f}")

    # ── 同向相邻笔对：最近 3 对 ──
    print(f"\n{'='*78}\n【同向相邻笔背驰判定对照】最近 3 对（A=较早 C=较晚同向笔）\n{'='*78}")
    pairs = []
    for i in range(len(rows) - 1, 0, -1):
        c = rows[i]
        for j in range(i - 1, -1, -1):
            a = rows[j]
            if a[0].direction == c[0].direction:
                pairs.append((a, c))
                break
        if len(pairs) >= 3:
            break

    for k, (a, c) in enumerate(pairs):
        (sa, ra0, ra1, ma) = a
        (sc, rc0, rc1, mc) = c
        d = "下跌" if sc.direction == "down" else "上涨"
        # 直接 Wasserstein A↔C（dim0 与 dim1）
        seg_a = closes[ra0 : ra1 + 1]
        seg_c = closes[rc0 : rc1 + 1]
        div0 = topo_divergence(
            barcode_from_prices(seg_a, maxdim=1),
            barcode_from_prices(seg_c, maxdim=1),
            dimension=0, noise_floor=noise,
        )
        div1 = topo_divergence(
            barcode_from_prices(seg_a, maxdim=1),
            barcode_from_prices(seg_c, maxdim=1),
            dimension=1, noise_floor=noise,
        )
        print(f"\n  ── 对 {k+1}：{d}笔  A=[{ra0}:{ra1}]({ma['n_bars']}根) "
              f"C=[{rc0}:{rc1}]({mc['n_bars']}根) ──")
        print(f"     {'度量':<22}{'A':>10}{'C':>10}{'C/A':>8}  判定")
        def line(name, av, cv):
            r = cv / av if av else 0.0
            print(f"     {name:<22}{av:>10.2f}{cv:>10.2f}{r:>8.2f}  {verdict(av, cv)}")
        line("幅度 |p1-p0|(振幅)", ma["amp"], mc["amp"])
        line("MACD 面积(动量×时间)", ma["macd"], mc["macd"])
        line("H0 总持续度(W到噪声)", ma["tot_h0"], mc["tot_h0"])
        line("H0 主导bar(≈振幅)", ma["max_h0"], mc["max_h0"])
        line("H0 丰富度(总−主导)", ma["secondary"], mc["secondary"])
        line("H0 活跃bar数(结构)", float(ma["n_active"]), float(mc["n_active"]))
        line("H1 中枢loop几何", ma["tot_h1"], mc["tot_h1"])
        print(f"     {'W1(A,C) dim0':<22}{'':>10}{'':>10}{div0.wasserstein_ac:>8.2f}"
              f"  结构重组幅度(dim0)")
        print(f"     {'W1(A,C) dim1':<22}{'':>10}{'':>10}{div1.wasserstein_ac:>8.2f}"
              f"  结构重组幅度(dim1)")
        # 一致性诊断
        macd_div = mc["macd"] < ma["macd"]
        wnoise_div = mc["tot_h0"] < ma["tot_h0"]
        rich_div = mc["secondary"] < ma["secondary"]
        print(f"     ↳ MACD={'背驰' if macd_div else '不背驰'} | "
              f"W噪声={'背驰' if wnoise_div else '不背驰'} | "
              f"丰富度={'背驰' if rich_div else '不背驰'}")
        if wnoise_div == macd_div:
            print(f"       · W噪声 与 MACD 一致 → 此对上 W噪声 未提供 MACD 之外信息")
        else:
            print(f"       · ★ W噪声 与 MACD 分歧 → 振幅与动量背离（缓跌/急跌）")
        if rich_div != macd_div or rich_div != wnoise_div:
            print(f"       · ★ 丰富度判定与 MACD/W噪声 不同 → 子结构信号独立于振幅+动量")


if __name__ == "__main__":
    main()
