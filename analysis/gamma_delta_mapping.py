"""Γ→Δ 映射性质的信息论实证 — Γ(三主边) 能否确定交叉边/Δ？

任务（编排者）：检验 Γ=(σ_P,σ_C,σ_R) 到交叉边/资本流量 Δ 的映射性质
（确定 / 多对一 / 一对多）。设计文档 §11。

理论预测（§11.1）：σ 算子不保比值同态（σ(A/B)≠g(σ(A),σ(B))，反例：A、B 均涨，
A/B 可涨可跌可平）→ Γ 不唯一确定交叉边 → Γ→Δ 一对多 → 需 ω 分辨。

实证方法（信息论）：
  - K4 实例 {ES,GC,CL,$}（k4_path_c 同款；$=UUP）。
  - 主边 Γ = (σ(ES/$), σ(GC/$), σ(CL/$))；交叉边 X = (σ(ES/GC), σ(ES/CL), σ(GC/CL))。
  - 6 条边各跑增量缠论递归，按日频采样 (Γ, X)。
  - H(X|Γ)：给定主边，交叉边的条件熵。=0 确定；>0 一对多。
  - ω = σ(GC/CL)（金油比方向，交叉边之一）。H(X_rest|Γ,ω) vs H(X_rest|Γ)：ω 分辨力。

认识论：**L2**（真实数据，可证伪）。数据窗口 2020-2026（UUP 1h 限制）。
Δ 直接实证缺口：无 ETF fund flows / COT positioning 数据 → Γ→Δ 完整实证无法做，
但 Γ→交叉边一对多是 Γ→Δ 一对多的**充分条件**（Δ 依赖交叉边，§11.1）。

运行：PYTHONPATH=src python analysis/gamma_delta_mapping.py
"""

from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from datetime import datetime
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.config_space import WalkDirection
from newchan.types import Bar

_DATA = Path(__file__).resolve().parent / "data_cache"
_SAMPLE_EVERY = 7   # 每 ~7 根 1h bar（约日频）采样一次 (Γ,X)
_START, _END = "2020-01-01", "2026-12-31"


def _load(f: str) -> dict:
    return json.load(open(_DATA / f))


def _to_hourly(d: dict) -> dict:
    """1min → 1h 降采样（按 date[:13] 小时分组）。"""
    g: dict[str, list[int]] = defaultdict(list)
    for i, ds in enumerate(d["dates"]):
        g[ds[:13]].append(i)
    dates, o, h, l, c = [], [], [], [], []
    for hr in sorted(g):
        idx = g[hr]
        dates.append(hr + ":00:00")
        o.append(d["opens"][idx[0]])
        h.append(max(d["highs"][i] for i in idx))
        l.append(min(d["lows"][i] for i in idx))
        c.append(d["closes"][idx[-1]])
    return {"dates": dates, "opens": o, "highs": h, "lows": l, "closes": c}


def _index_by_hour(d: dict) -> dict[str, int]:
    return {ds[:13]: i for i, ds in enumerate(d["dates"])}


def _parse(s: str) -> datetime:
    for fmt in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H", "%Y-%m-%d"):
        try:
            return datetime.strptime(s.strip(), fmt)
        except ValueError:
            continue
    return datetime.strptime(s[:13].strip(), "%Y-%m-%d %H")


def _sigma(orch: RecursiveOrchestrator, bar: Bar) -> WalkDirection:
    """喂一根 bar，返回当前最高级别走势方向 σ。"""
    snap = orch.process_bar(bar)
    moves = snap.move_snapshot.moves
    if not moves or moves[-1].kind == "consolidation":
        return WalkDirection.FLAT
    if moves[-1].direction == "up":
        return WalkDirection.UP
    if moves[-1].direction == "down":
        return WalkDirection.DOWN
    return WalkDirection.FLAT


def _ratio_bar(na: dict, ia: int, nb: dict, ib: int, ts: str) -> Bar:
    """构造 na/nb 在对齐索引处的比价 bar（正确比值 OHLC）。"""
    return Bar(
        ts=_parse(ts),
        open=na["opens"][ia] / nb["opens"][ib],
        high=na["highs"][ia] / nb["lows"][ib],
        low=na["lows"][ia] / nb["highs"][ib],
        close=na["closes"][ia] / nb["closes"][ib],
    )


def _entropy(counts: list[int]) -> float:
    """香农熵（bits）。"""
    tot = sum(counts)
    if tot == 0:
        return 0.0
    h = 0.0
    for n in counts:
        if n > 0:
            p = n / tot
            h -= p * math.log2(p)
    return h


def _conditional_entropy(pairs: list[tuple]) -> float:
    """H(Y|X) = Σ_x P(x) H(Y|x)，pairs=[(x,y),...]。"""
    by_x: dict = defaultdict(Counter)
    for x, y in pairs:
        by_x[x][y] += 1
    total = len(pairs)
    h = 0.0
    for x, yc in by_x.items():
        px = sum(yc.values()) / total
        h += px * _entropy(list(yc.values()))
    return h


def main() -> None:
    print("=" * 78)
    print("Γ→Δ 映射性质实证（信息论）：Γ(三主边) 能否确定交叉边/Δ？")
    print(f"K4 实例 {{ES,GC,CL,$=UUP}} 1h，窗口 {_START}~{_END}，每{_SAMPLE_EVERY}根采样")
    print("=" * 78)

    es = _load("es_1h_databento.json")
    gc = _load("gc_1h_databento.json")
    cl = _load("cl_1h_databento.json")
    uup = _to_hourly(_load("uup_1m_full.json"))

    # 公共小时时间网格（四标的交集）。
    idx = {"ES": _index_by_hour(es), "GC": _index_by_hour(gc),
           "CL": _index_by_hour(cl), "UUP": _index_by_hour(uup)}
    hours = sorted(set(idx["ES"]) & set(idx["GC"]) & set(idx["CL"]) & set(idx["UUP"]))
    hours = [h for h in hours if _START <= h[:10] <= _END]
    print(f"\n对齐后公共 1h bar 数：{len(hours)}")

    src = {"ES": es, "GC": gc, "CL": cl, "UUP": uup}

    # 6 条边各一个 orchestrator。主边对 $(UUP)，交叉边资产间。
    edges = {
        "ES/$": ("ES", "UUP"), "GC/$": ("GC", "UUP"), "CL/$": ("CL", "UUP"),
        "ES/GC": ("ES", "GC"), "ES/CL": ("ES", "CL"), "GC/CL": ("GC", "CL"),
    }
    orchs = {e: RecursiveOrchestrator(stream_id=e, max_levels=6, stroke_mode="wide")
             for e in edges}

    samples: list[dict] = []
    for k, hr in enumerate(hours):
        sig = {}
        for ename, (a, b) in edges.items():
            bar = _ratio_bar(src[a], idx[a][hr], src[b], idx[b][hr], hours[k])
            sig[ename] = _sigma(orchs[ename], bar)
        if k % _SAMPLE_EVERY == 0:
            samples.append({e: sig[e].value for e in edges})

    print(f"采样点数：{len(samples)}")

    # Γ=(σES/$,σGC/$,σCL/$)，X=(σES/GC,σES/CL,σGC/CL)，ω=σGC/CL。
    gamma_X = [((s["ES/$"], s["GC/$"], s["CL/$"]),
               (s["ES/GC"], s["ES/CL"], s["GC/CL"])) for s in samples]
    gamma_Xrest_omega = [((s["ES/$"], s["GC/$"], s["CL/$"]),
                          (s["ES/GC"], s["ES/CL"]), s["GC/CL"]) for s in samples]

    # 每个 Γ 的交叉边多样性。
    by_gamma: dict = defaultdict(Counter)
    for g, x in gamma_X:
        by_gamma[g][x] += 1
    multiplicities = [len(xc) for xc in by_gamma.values()]
    n_gamma = len(by_gamma)
    n_det = sum(1 for m in multiplicities if m == 1)

    print(f"\n── 映射多样性 ──")
    print(f"出现的 Γ 配置数：{n_gamma}/27")
    print(f"Γ→交叉边 唯一确定(多样性=1)的 Γ：{n_det}/{n_gamma}")
    print(f"交叉边组合多样性：max={max(multiplicities)} mean={sum(multiplicities)/n_gamma:.2f}")

    # 条件熵。
    H_X_given_G = _conditional_entropy(gamma_X)
    # H(X_rest|Γ) 与 H(X_rest|Γ,ω)。
    H_Xrest_given_G = _conditional_entropy([(g, xr) for g, xr, _ in gamma_Xrest_omega])
    H_Xrest_given_G_omega = _conditional_entropy(
        [((g, w), xr) for g, xr, w in gamma_Xrest_omega])

    print(f"\n── 条件熵（bits）──")
    print(f"H(交叉边 | Γ)             = {H_X_given_G:.3f}  "
          f"({'确定✓' if H_X_given_G < 0.01 else '★一对多★'})")
    print(f"H(其余两交叉边 | Γ)        = {H_Xrest_given_G:.3f}")
    print(f"H(其余两交叉边 | Γ, ω)     = {H_Xrest_given_G_omega:.3f}")
    gain = H_Xrest_given_G - H_Xrest_given_G_omega
    print(f"ω 的分辨力 (信息增益)      = {gain:.3f} bits "
          f"({'ω有分辨力✓' if gain > 0.05 else 'ω分辨力弱'})")
    print(f"  → (Γ,ω) 后剩余不确定性  = {H_Xrest_given_G_omega:.3f} "
          f"({'完全确定✓' if H_Xrest_given_G_omega < 0.01 else '仍一对多(需更多信息)'})")

    # 最一对多的 Γ 示例。
    worst = sorted(by_gamma.items(), key=lambda kv: -len(kv[1]))[:3]
    print(f"\n── 最一对多的 Γ（同一主边配置，多种交叉边）──")
    sym = {1: "+", 0: "0", -1: "-"}
    for g, xc in worst:
        gl = f"({sym[g[0]]},{sym[g[1]]},{sym[g[2]]})"
        print(f"  Γ={gl}: {len(xc)}种交叉边组合, 共{sum(xc.values())}样本")
        for x, n in xc.most_common(3):
            xl = f"({sym[x[0]]},{sym[x[1]]},{sym[x[2]]})"
            print(f"      交叉边{xl}: {n}次")

    print("\n" + "=" * 78)
    verdict = ("一对多（理论确认）：Γ 不唯一确定交叉边，σ 不保同态"
               if H_X_given_G >= 0.01 else "确定：Γ 唯一确定交叉边")
    print(f"裁决：Γ→交叉边 = {verdict}")
    print("Δ 直接实证：无 fund flows/COT 数据（缺口）；Γ→交叉边一对多 ⟹ Γ→Δ 一对多。")
    print("=" * 78)


if __name__ == "__main__":
    main()
