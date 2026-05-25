"""腾讯 700 — 压制比（suppression ratio）背驰是否有独立操作意义？

核心问题（2026-05-24 编排者）
----------------------------
已确认 PH 不能平替 MACD（互补不可约）。本脚本验证一个**新的 PH 背驰判据**——
压制比——是否携带 MACD 看不到、且有操作意义的信息。

压制比定义
----------
一段推动浪在其 close 序列上做 sublevel-set H0 持续同调，得到一组按 persistence
降序的 bar：
  - 主bar  = bars[0].persistence = 该段的全幅 prominence（推动浪本身）
  - 次大bar = bars[1].persistence = 段内最大的反向摆动 prominence（最大反抗/反弹）
  suppression_ratio = 主bar / 次大bar
- 比值高（→∞ 为纯单调）= 一路碾压，中间没有像样反抗 → 推动强
- 比值低 = 推动还在，但中间反弹很大，对手方在抵抗 → 推动弱

方向不变性（L1 已验证）
----------------------
1D persistence 的有限 bar 是同一组 (局部极小, 局部极大) 配对，sublevel 与
superlevel 的有限 persistence 多重集相同；全局 bar 两个方向都 = 段全幅。
因此压制比直接在 close 上算即可，对上涨笔/下跌笔都正确度量
"主推动幅度 / 最大内部反抗 prominence"，无需翻转符号。

背驰判据（对同向相邻笔对 A=stroke[i], C=stroke[i+2]，A 在前 C 在后）
------------------------------------------------------------------
- 压制比背驰  SR_div  = suppression_ratio(C) < suppression_ratio(A)  （对手抵抗相对增强）
- MACD面积背驰 MACD_div = macd_area(C) < macd_area(A)                  （动量相对减弱）
两者概念同向：后一段更弱 → 反转预警。

操作意义判据（编排者给定）
------------------------
若存在 ≥1 个 case：SR 背驰但 MACD 未背驰（SR 领先），且该 case 后续被验证
（走势确实反转）→ 压制比有独立操作意义。
若所有 case 中 SR 信号都不早于 MACD → 压制比只是 MACD 的冗余投影。

认识论等级
----------
- 压制比/MACD 计算：L0（纯算法）。
- 腾讯 700 日线读出 + 后续走势验证：L2（单标的单时段真实数据，可否证）。
- 笔由仓库新笔引擎从对齐 OHLCV 推出（与 tencent_ph_macd_aufheben.py 一致）。
  pine 标注（一缠/CZSC 指标）含买卖点+混合数值，无干净的笔端点+bar索引，
  故采用可复现的仓库新笔引擎做笔分段。
"""

from __future__ import annotations

import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_fractal import fractals_from_merged  # noqa: E402
from newchan.a_inclusion import merge_inclusion  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_persistence_barcode import sublevel_h0_bars  # noqa: E402
from newchan.a_stroke import strokes_from_fractals  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
MACD_WARMUP = 35      # MACD 慢线 EMA(26)+信号线(9) 预热；r1<此值的笔 MACD 面积不可信
MACD_FLOOR = 3.0      # MACD 面积 < 此值视为"无动量"，相关笔对的 MACD 背驰判定退化，剔除
LOOKAHEAD = 25        # 反转验证窗口（C 段终点之后看多少根 K 线是否创新极值）


def build_df(raw: list[dict]) -> pd.DataFrame:
    ts = [datetime.fromtimestamp(b["time"], tz=timezone.utc) for b in raw]
    return pd.DataFrame(
        {k: [float(b[k]) for b in raw] for k in ("open", "high", "low", "close")},
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


def suppression_ratio(seg: list[float]) -> tuple[float, float, float, int]:
    """返回 (ratio, 主bar persistence, 次大bar persistence, n_bars)。

    主bar = 全幅 prominence；次大bar = 最大内部反抗 prominence。
    纯单调（n_bars==1，无内部反抗）→ ratio = +inf。
    """
    bars = sublevel_h0_bars(seg)  # 已按 persistence 降序
    if not bars:
        return (math.nan, 0.0, 0.0, 0)
    dom = bars[0].persistence
    if len(bars) == 1:
        return (math.inf, dom, 0.0, 1)
    sub = bars[1].persistence
    if sub <= 0:
        return (math.inf, dom, 0.0, len(bars))
    return (dom / sub, dom, sub, len(bars))


def collect(raw: list[dict]) -> tuple[list[dict], list[float]]:
    df = build_df(raw)
    closes = df["close"].tolist()
    df_macd = compute_macd(df)
    strokes, m2r = strokes_of(df)
    feats: list[dict] = []
    for idx, s in enumerate(strokes):
        r0, r1 = raw_span(s, m2r)
        seg = closes[r0 : r1 + 1]
        if len(seg) < 3:
            continue
        ratio, dom, sub, nb = suppression_ratio(seg)
        area = macd_area_for_range(df_macd, r0, r1)
        macd = abs(area["area_neg"]) if s.direction == "down" else abs(area["area_pos"])
        feats.append(
            {
                "idx": idx,
                "dir": s.direction,
                "r0": r0,
                "r1": r1,
                "span": len(seg),
                "amp": abs(seg[-1] - seg[0]),
                "p_start": seg[0],
                "p_end": seg[-1],
                "sr": ratio,
                "dom": dom,
                "sub": sub,
                "n_bars": nb,
                "macd": macd,
            }
        )
    return feats, closes


def sr_disp(sr: float) -> str:
    return "  inf" if math.isinf(sr) else f"{sr:5.2f}"


def reversal_after(c: dict, closes: list[float], lookahead: int = LOOKAHEAD) -> dict | None:
    """C 段是推动浪，检验其终点是否为被验证的趋势极值（背驰预警是否兑现）。

    背驰预警的操作含义 = "这一段是趋势的尽头"。验证标准：C 段终点之后
    lookahead 根 K 线内，价格**没有沿 C 方向创出新极值**（趋势停滞/反转）。
    - C 上涨：终点是高点；后续若不再创新高 → 反转确认。
    - C 下跌：终点是低点；后续若不再创新低 → 反转确认。
    同时报告后续最大反向回撤幅度，便于判断反转的实质性。
    返回 None 表示窗口外无足够数据。
    """
    end = c["r1"]
    p_end = closes[end]
    window = closes[end + 1 : end + 1 + lookahead]
    if len(window) < 3:
        return None
    if c["dir"] == "up":
        new_extreme = max(window) > p_end          # 创新高 → 趋势延续 → 背驰被证伪
        adverse = p_end - min(window)              # 反向（下行）最大回撤
    else:
        new_extreme = min(window) < p_end          # 创新低 → 趋势延续 → 背驰被证伪
        adverse = max(window) - p_end              # 反向（上行）最大反弹
    return {"confirmed": not new_extreme, "adverse": adverse,
            "adverse_ratio": adverse / c["amp"] if c["amp"] > 0 else math.nan}


def analyze(name: str, raw: list[dict]) -> None:
    feats, closes = collect(raw)
    n = len(feats)
    print("#" * 90)
    print(f"# 腾讯 700 {name} — 压制比 vs MACD 面积背驰  (n_strokes={n})")
    print("#" * 90)

    # ---- 每笔度量表 ----
    print(f"\n{'='*90}\n每笔度量\n{'='*90}")
    print(f"{'#':>3} {'dir':>4} {'span':>4} {'振幅':>7} {'主bar':>7} {'次大bar':>8} "
          f"{'压制比':>6} {'nbar':>4} {'MACD面积':>8}")
    for f in feats:
        print(f"{f['idx']:>3} {f['dir']:>4} {f['span']:>4} {f['amp']:>7.1f} "
              f"{f['dom']:>7.1f} {f['sub']:>8.1f} {sr_disp(f['sr']):>6} {f['n_bars']:>4} "
              f"{f['macd']:>8.2f}")

    # ---- 同向相邻笔对背驰对比 ----
    # 背驰前提（缠论）：C 必须沿趋势方向创新极值（上涨创新高/下跌创新低），
    #                  否则只是中枢震荡，无"背驰"可言。
    # 干净测试前提：A、C 都在 MACD 预热之后，且都有实质动量（MACD>FLOOR），
    #              否则 MACD 背驰判定退化为对 0 的算术比较（伪信号）。
    all_pairs = [i for i in range(n - 2) if feats[i]["dir"] == feats[i + 2]["dir"]]
    print(f"\n{'='*90}\n同向相邻笔对背驰对比  (A=stroke[i], C=stroke[i+2]; 共 {len(all_pairs)} 个同向对)\n{'='*90}")
    print(f"{'A→C':>9} {'dir':>4} {'srA':>6} {'srC':>6} {'SR背':>5} "
          f"{'macdA':>7} {'macdC':>7} {'MACD背':>6} {'新极值':>6} {'干净':>5} {'判定':>9} {'反转?':>6}")

    cats = {"both": [], "neither": [], "sr_only": [], "macd_only": []}
    clean_cats = {"both": [], "neither": [], "sr_only": [], "macd_only": []}
    for i in all_pairs:
        A, C = feats[i], feats[i + 2]
        sr_div = C["sr"] < A["sr"]
        macd_div = C["macd"] < A["macd"]
        # 背驰前提：C 创新极值
        new_ext = (C["p_end"] > A["p_end"]) if C["dir"] == "up" else (C["p_end"] < A["p_end"])
        # 干净：脱离预热 + 双侧有动量
        clean = (A["r1"] >= MACD_WARMUP and min(A["macd"], C["macd"]) >= MACD_FLOOR and new_ext)
        if sr_div and macd_div:
            cat, key = "一致背驰", "both"
        elif not sr_div and not macd_div:
            cat, key = "一致无背", "neither"
        elif sr_div and not macd_div:
            cat, key = "★SR独有", "sr_only"
        else:
            cat, key = "MACD独有", "macd_only"
        rev = reversal_after(C, closes)
        cats[key].append((i, A, C, rev))
        if clean:
            clean_cats[key].append((i, A, C, rev))
        rc = "—" if rev is None else ("✓" if rev["confirmed"] else "✗")
        print(f"{f'{i}→{i+2}':>9} {C['dir']:>4} {sr_disp(A['sr']):>6} {sr_disp(C['sr']):>6} "
              f"{('是' if sr_div else '·'):>5} {A['macd']:>7.1f} {C['macd']:>7.1f} "
              f"{('是' if macd_div else '·'):>6} {('是' if new_ext else '·'):>6} "
              f"{('✓' if clean else '·'):>5} {cat:>9} {rc:>6}")

    # ---- 一致性统计 ----
    def stat(block: dict, label: str) -> None:
        tot = sum(len(v) for v in block.values())
        agree = len(block["both"]) + len(block["neither"])
        print(f"\n  [{label}] 共 {tot} 对")
        print(f"    一致背驰 {len(block['both'])} | 一致无背 {len(block['neither'])} | "
              f"★SR独有 {len(block['sr_only'])} | MACD独有 {len(block['macd_only'])}")
        if tot:
            print(f"    一致率 = {agree}/{tot} = {agree/tot*100:.0f}%")

    print(f"\n{'='*90}\n一致性统计\n{'='*90}")
    stat(cats, "全部同向对")
    stat(clean_cats, "干净对：脱离MACD预热+双侧有动量+C创新极值")

    # ---- 领先性检验（核心）：干净的 SR 独有 case 后续是否被验证 ----
    print(f"\n{'='*90}\n★领先性检验：SR 背驰但 MACD 未背驰的【干净】case，后续是否反转？\n{'='*90}")
    print("  (这是判定操作意义的关键：SR 领先 MACD 且被走势验证 → 有独立操作意义)")
    if not clean_cats["sr_only"]:
        print("  无干净的 SR 独有 case → 压制比未在任何干净笔对上领先 MACD。")
    else:
        conf = 0
        for i, A, C, rev in clean_cats["sr_only"]:
            rc = "无后续" if rev is None else ("✓反转确认" if rev["confirmed"] else "✗趋势延续(未反转)")
            if rev and rev["confirmed"]:
                conf += 1
            ar = "—" if rev is None else f"{rev['adverse_ratio']:.2f}"
            print(f"  笔对 {i}→{i+2} ({C['dir']}): SR {sr_disp(A['sr'])}→{sr_disp(C['sr'])}↓背驰, "
                  f"MACD {A['macd']:.1f}→{C['macd']:.1f}↑未背驰; C段[{C['r0']}:{C['r1']}] "
                  f"后续{LOOKAHEAD}根反向/C幅度={ar} → {rc}")
        print(f"\n  干净 SR 独有 case 反转确认率 = {conf}/{len(clean_cats['sr_only'])}")

    # 对照组：干净一致背驰 case 的反转率（基准线）
    base = clean_cats["both"]
    if base:
        bc = sum(1 for *_, rev in base if rev and rev["confirmed"])
        print(f"  对照基准——干净一致背驰 case 反转确认率 = {bc}/{len(base)}")

    # ---- 全部干净对明细（含反向回撤幅度，避免二值化掩盖信息）----
    print(f"\n{'='*90}\n全部干净对明细（反向回撤/C幅度：操作上能否吃到反转的连续量）\n{'='*90}")
    clean_all = sorted(
        [(i, A, C, rev) for key in clean_cats for (i, A, C, rev) in clean_cats[key]],
        key=lambda t: t[0],
    )
    for i, A, C, rev in clean_all:
        which = next(k for k in clean_cats if (i, A, C, rev) in clean_cats[k])
        zh = {"both": "一致背驰", "neither": "一致无背", "sr_only": "★SR独有", "macd_only": "MACD独有"}[which]
        ar = "—" if rev is None else f"{rev['adverse_ratio']:.2f}"
        rc = "—" if rev is None else ("✓反转" if rev["confirmed"] else "趋势续")
        print(f"  {i}→{i+2} {C['dir']:>4} {zh:>9}: SR {sr_disp(A['sr'])}→{sr_disp(C['sr'])} "
              f"MACD {A['macd']:>5.1f}→{C['macd']:>5.1f} | 后续反向/C={ar} {rc}")

    # ---- 混淆检查：低压制比是否只是"长笔/老笔"的代理变量？----
    print(f"\n{'='*90}\n混淆检查：压制比 vs span / MACD 的相关性（低SR是否只是长笔代理？）\n{'='*90}")
    fin = [f for f in feats if f["r1"] >= MACD_WARMUP and not math.isinf(f["sr"])]
    if len(fin) >= 5:
        import numpy as np
        sr = np.array([f["sr"] for f in fin])
        sp = np.array([f["span"] for f in fin], float)
        mc = np.array([f["macd"] for f in fin])
        am = np.array([f["amp"] for f in fin])

        def pe(x, y):
            return float("nan") if x.std() == 0 or y.std() == 0 else float(np.corrcoef(x, y)[0, 1])
        print(f"  (脱离预热的有限SR笔 n={len(fin)})")
        print(f"    corr(压制比, span)     = {pe(sr, sp):+.3f}   (强负→低SR只是长笔代理)")
        print(f"    corr(压制比, MACD面积) = {pe(sr, mc):+.3f}")
        print(f"    corr(压制比, 振幅)     = {pe(sr, am):+.3f}")

    # ---- 反面检验：单笔压制比低但 MACD 面积大 ----
    print(f"\n{'='*90}\n反面检验：单笔压制比低（抵抗强）但 MACD 面积大（动量强）→ 后续是否反转？\n{'='*90}")
    usable = [f for f in feats if f["r1"] >= MACD_WARMUP and not math.isinf(f["sr"])]
    if len(usable) < 6:
        print("  可用样本不足。")
    else:
        srs = sorted(f["sr"] for f in usable)
        macds = sorted(f["macd"] for f in usable)
        sr_lo = srs[len(srs) // 3]
        macd_hi = macds[2 * len(macds) // 3]
        print(f"  阈值: 压制比 < {sr_lo:.2f} (下1/3) 且 MACD面积 > {macd_hi:.2f} (上1/3)；含创新极值前提")
        hits = [f for f in usable if f["sr"] < sr_lo and f["macd"] > macd_hi]
        if not hits:
            print("  无同时满足'压制比低+MACD面积大'的笔。")
        else:
            for f in hits:
                rev = reversal_after(f, closes)
                rc = "无后续" if rev is None else ("✓反转确认(SR对,MACD错)" if rev["confirmed"]
                                                else "✗趋势延续(MACD对,SR错)")
                ar = "—" if rev is None else f"{rev['adverse_ratio']:.2f}"
                print(f"  笔 {f['idx']} ({f['dir']}) [{f['r0']}:{f['r1']}]: SR {sr_disp(f['sr'])}(低), "
                      f"MACD {f['macd']:.1f}(大); 后续{LOOKAHEAD}根反向/本笔={ar} → {rc}")


def main() -> None:
    daily = json.loads((DATA / "daily_ohlcv.json").read_text())["bars"]
    analyze("日线", daily)


if __name__ == "__main__":
    main()
