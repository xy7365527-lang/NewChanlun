"""资本病理学 L2 验证 — 圈闭合残差的缠论走势结构检验。

任务（编排者）：检验"圈闭合残差的走势结构对应卢麒元三资本病态"假说。
核心诚实判据：
  - 残差/比值有缠论走势结构（趋势+中枢+背驰）→ 耗散结构假说成立。
  - 残差是纯噪音（无中枢、无走势）→ 假说被否定。
  诚实报告，不修饰。

认识论等级：**L2**（真实数据，可证伪）。设计文档 §8.5。

数据边界（诚实标注，231号）：
  - 1h databento ES/GC/CL：2016-2026，**覆盖 2020-2022 QE→加息全程** ← 空转/ω/涉Au 主验证。
  - DBC/VNQ：仅 2023-12+（TWS），**不覆盖 2020-2022** → 沉没仅近期窗口验证。
  - csi300/ashr/usdcny 日线 → 走资验证（日线分辨率）。

关键区分（设计文档 §3.1）：
  - a0 精确闭合残差 ρ=log(P/C)−[log(P/M)+log(M/C)]：同源价格**恒为 0**（数据质量门），
    对它做缠论递归是类型错误。本脚本验证其恒 0（数据自洽），不当病态信号。
  - 病态信号在**走势状态层的非平凡比值**：ω=GC/CL、P/C=ES/CL、R/P=VNQ/ES 等。

运行：PYTHONPATH=src python analysis/capital_pathology_validation.py
"""

from __future__ import annotations

import json
import math
from datetime import datetime
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

_DATA = Path(__file__).resolve().parent / "data_cache"


def _load(filename: str) -> dict:
    return json.load(open(_DATA / filename))


def _ratio_ohlc(num: dict, ref: dict, start: str, end: str) -> tuple[list[str], list, list, list, list]:
    """构造 num/ref 比价 OHLC（正确比值：高=num_h/ref_l、低=num_l/ref_h），按日期窗口切片。"""
    rb = {d: i for i, d in enumerate(ref["dates"])}
    dates, o, h, l, c = [], [], [], [], []
    for j, d in enumerate(num["dates"]):
        key = d
        i = rb.get(key)
        if i is None:
            continue
        if d[:10] < start or d[:10] > end:
            continue
        rl, rh, ro, rc = ref["lows"][i], ref["highs"][i], ref["opens"][i], ref["closes"][i]
        if rl <= 0 or rh <= 0 or ro <= 0 or rc <= 0:
            continue
        dates.append(d)
        o.append(num["opens"][j] / ro)
        h.append(num["highs"][j] / rl)
        l.append(num["lows"][j] / rh)
        c.append(num["closes"][j] / rc)
    return dates, o, h, l, c


def _downsample_daily(d: dict) -> dict:
    """1min/分钟序列降采样到日线（open=首,high=max,low=min,close=尾）。"""
    by_day: dict[str, list[int]] = {}
    for i, ds in enumerate(d["dates"]):
        by_day.setdefault(ds[:10], []).append(i)
    dates, o, h, l, c = [], [], [], [], []
    for day in sorted(by_day):
        idx = by_day[day]
        dates.append(day)
        o.append(d["opens"][idx[0]])
        h.append(max(d["highs"][i] for i in idx))
        l.append(min(d["lows"][i] for i in idx))
        c.append(d["closes"][idx[-1]])
    return {"symbol": d.get("symbol", "?"), "dates": dates,
            "opens": o, "highs": h, "lows": l, "closes": c}


def _parse(s: str) -> datetime:
    core = s.split("+")[0].strip()
    for fmt in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%d"):
        try:
            return datetime.strptime(core, fmt)
        except ValueError:
            continue
    return datetime.strptime(core[:19], "%Y-%m-%d %H:%M:%S")


def recurse_summary(name: str, dates: list[str], o, h, l, c) -> dict:
    """对比价序列跑缠论递归，返回走势结构摘要。"""
    orch = RecursiveOrchestrator(stream_id=name, max_levels=6, stroke_mode="wide")
    snap = None
    for i in range(len(c)):
        snap = orch.process_bar(
            Bar(ts=_parse(dates[i]), open=o[i], high=h[i], low=l[i], close=c[i])
        )
    if snap is None:
        return {"name": name, "bars": 0}

    ms = snap.move_snapshot
    n_bi = sum(len(rs.bis) for rs in snap.recursive_snapshots if hasattr(rs, "bis")) \
        if snap.recursive_snapshots else 0
    max_level = 1
    for rs in snap.recursive_snapshots:
        max_level = max(max_level, rs.level_id)
    moves = ms.moves
    n_moves = len(moves)
    n_zs = sum(1 for m in moves if getattr(m, "kind", "") == "consolidation")
    n_trend = sum(1 for m in moves if getattr(m, "kind", "") == "trend")
    last = moves[-1] if moves else None
    # 走势结构判据：有 trend 走势 ∨ (走势数≥2)= 有结构；纯单一盘整/无走势 = 噪音候选。
    has_structure = n_trend >= 1 or n_moves >= 2
    return {
        "name": name,
        "bars": len(c),
        "range": f"{dates[0][:10]}~{dates[-1][:10]}",
        "max_level": max_level,
        "moves": n_moves,
        "trend_moves": n_trend,
        "consol_moves": n_zs,
        "last_kind": getattr(last, "kind", "none") if last else "none",
        "last_dir": getattr(last, "direction", "none") if last else "none",
        "last_settled": getattr(last, "settled", False) if last else False,
        "has_structure": has_structure,
    }


def a0_residual_check(P: dict, C: dict, M: dict, start: str, end: str, label: str) -> None:
    """a0 精确闭合：ρ=log(P/C)−[log(P/M)+log(M/C)] 逐 bar，验证恒≈0（数据质量门）。"""
    # 三序列按日期对齐。
    pm = {d: P["closes"][i] for i, d in enumerate(P["dates"])}
    cm = {d: C["closes"][i] for i, d in enumerate(C["dates"])}
    mm = {d: M["closes"][i] for i, d in enumerate(M["dates"])}
    common = sorted(set(pm) & set(cm) & set(mm))
    common = [d for d in common if start <= d[:10] <= end]
    if not common:
        print(f"  [a0 {label}] 无公共日期（数据窗口不重叠）")
        return
    max_abs = 0.0
    for d in common:
        p, c_, m = pm[d], cm[d], mm[d]
        if p <= 0 or c_ <= 0 or m <= 0:
            continue
        rho = abs(math.log(p / c_) - (math.log(p / m) + math.log(m / c_)))
        max_abs = max(max_abs, rho)
    verdict = "恒0✓数据自洽" if max_abs < 1e-9 else f"非0(数据问题/不同源)"
    print(f"  [a0 {label}] n={len(common)} max|ρ|={max_abs:.2e} → {verdict}")


def _fmt(s: dict) -> str:
    if s.get("bars", 0) == 0:
        return f"  {s['name']:14s} 无数据"
    struct = "有结构✓" if s["has_structure"] else "★噪音?★"
    return (f"  {s['name']:14s} {s['range']} bars={s['bars']:6d} "
            f"L{s['max_level']} 走势={s['moves']}(趋势{s['trend_moves']}/盘整{s['consol_moves']}) "
            f"末[{s['last_kind']},{s['last_dir']},settled={s['last_settled']}] {struct}")


def main() -> None:
    print("=" * 78)
    print("资本病理学 L2 验证：圈闭合残差/比值的缠论走势结构")
    print("判据：有走势结构(趋势+中枢)→耗散假说成立；纯噪音→否定。诚实报告。")
    print("=" * 78)

    # ── 验证 1：空转 + ω（1h databento ES/GC/CL，覆盖 2020-2022）──
    print("\n【验证1 空转/ω/涉Au】1h databento ES/GC/CL，全程 2016-2026 + 2020-2022 子窗口")
    es = _load("es_1h_databento.json")
    gc = _load("gc_1h_databento.json")
    cl = _load("cl_1h_databento.json")

    for tag, start, end in [("全程", "2016-01-01", "2026-12-31"),
                            ("QE→加息 2020-2022", "2020-01-01", "2022-12-31")]:
        print(f"\n  ── {tag} ──")
        # ω = GC/CL（金油比，482号空转/剥削率）
        d, o, h, l, c = _ratio_ohlc(gc, cl, start, end)
        print(_fmt(recurse_summary("ω=GC/CL", d, o, h, l, c)))
        # P/C = ES/CL（堰塞湖价格投影，330号；C 用油相位）
        d, o, h, l, c = _ratio_ohlc(es, cl, start, end)
        print(_fmt(recurse_summary("P/C=ES/CL", d, o, h, l, c)))
        # 涉Au：ES/GC（P 对金，循环外锚）
        d, o, h, l, c = _ratio_ohlc(es, gc, start, end)
        print(_fmt(recurse_summary("ES/GC(涉Au)", d, o, h, l, c)))
        # a0 残差：金融循环三角形 ES/GC/CL（C=CL, M=GC 作中介验证恒等）
        a0_residual_check(es, cl, gc, start, end, "ES/CL/GC三角")

    # ── 验证 2：沉没（R 脱耦，VNQ 仅 2023-12+，降采样日线）──
    print("\n【验证2 沉没/R脱耦】VNQ vs ES/DBC（仅 2023-12+，不覆盖2020-2022，诚实标注）")
    vnq = _downsample_daily(_load("vnq_1m_tws.json"))
    es_d = _downsample_daily(_load("es_1m_databento.json"))
    dbc_d = _downsample_daily(_load("dbc_1m_tws.json"))
    for nm, num, ref in [("R/P=VNQ/ES", vnq, es_d), ("R/C=VNQ/DBC", vnq, dbc_d),
                         ("P/C=ES/DBC", es_d, dbc_d)]:
        d, o, h, l, c = _ratio_ohlc(num, ref, "2023-01-01", "2026-12-31")
        print(_fmt(recurse_summary(nm, d, o, h, l, c)))
    print("  → R脱耦判据：若 R/P、R/C 走势方向与 P/C 不同步=R 脱耦（沉没）。看上行末向。")

    # ── 验证 3：走资（CN 资本外流，日线）──
    print("\n【验证3 走资/CN外流】ASHR(美元计价CN股) vs SPY，日线 2013-2026")
    ashr = _load("ashr_1d_yf.json")
    spy = _load("spy_1d_yf.json")
    csi = _load("csi300_1d_yf.json")
    usdcny = _load("usdcny_1d_yf.json")
    for tag, start, end in [("全程", "2013-01-01", "2026-12-31"),
                            ("2020-2022", "2020-01-01", "2022-12-31")]:
        print(f"  ── {tag} ──")
        # 走资 = CN相对US持续走弱（美元计价）。ASHR已含汇率。
        d, o, h, l, c = _ratio_ohlc(ashr, spy, start, end)
        print(_fmt(recurse_summary("ASHR/SPY", d, o, h, l, c)))
        # CSI300本币/USDCNY = CN股美元值（另一路径，csi300仅2021+）
        d, o, h, l, c = _ratio_ohlc(csi, usdcny, start, end)
        if d:
            print(_fmt(recurse_summary("CSI300/CNY", d, o, h, l, c)))
    print("  → 走资判据：ASHR/SPY 持续下降趋势(非中枢震荡)=CN资本外流。看末向是否 down-trend。")

    print("\n" + "=" * 78)
    print("注：a0 残差恒0=数据自洽(数据质量门,非病态信号)；病态在上面比值的走势结构。")
    print("=" * 78)


if __name__ == "__main__":
    main()
