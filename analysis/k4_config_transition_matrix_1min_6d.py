#!/usr/bin/env python3
"""1min a0 K4 配置转换矩阵 —— **6 维 Γ / 729 态**（anchor-aware，Rust 引擎）。

与 27 态版（k4_config_transition_matrix_1min.py，生成树 3 条 vertex/M 边）的区别：
本版用 **K4 完全图全部 6 条边** 的 σ 组装 Γ ∈ {-1,0,+1}^6 → 理论 3^6=729 态，
并量化任务所述「实际有 K4 约束」——6 条边 σ 在多大程度上可由 4 顶点势诱导
（非平边有向图无矛盾循环）。这是 6 维相对 3 维的核心信息增量。

货币锚 anchor-aware（env K4_MONEY_ANCHOR，见 k4_1min_rust_lib）：
  · GC：黄金锚（历史默认）
  · DX：美元指数锚（528/529 编排者裁决 A=正典 M；GC/6E 降为折叠通道观测量）
  · 6E：欧元汇率锚
不同锚的边 σ 缓存物理隔离（_k4edge_{ANCHOR}_*）；输出文件按锚加后缀，互不覆盖。

用法：
  K4_MONEY_ANCHOR=DX PYTHONPATH=src .venv/bin/python \
      analysis/k4_config_transition_matrix_1min_6d.py [--procs P]
  缺省 anchor=GC（复用已落盘 GC 6 边缓存，秒级）；DX/6E 首次运行触发 6 边递归。

认识论等级：L2（真实 1min 期货数据；可证伪 K4 约束假设）。
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

from analysis.k4_1min_rust_lib import (  # noqa: E402
    ANCHOR_FILES,
    EDGES,
    MONEY_ANCHOR,
    compute_or_load_edge_sigmas,
)

# K4 完全图 6 条边 = lib.EDGES（顺序固定 = Γ 6 分量）。
EDGES6 = list(EDGES)
VERTS = ("P", "M", "C", "R")

ANCHOR_SYMBOL = {"GC": "GC(黄金)", "DX": "DX(美元指数)", "6E": "USD6E(欧元汇率)"}
_SUFFIX = f"_{MONEY_ANCHOR.lower()}" if MONEY_ANCHOR != "GC" else ""
REPORT_PATH = ROOT / "analysis" / f"k4_config_transition_matrix_1min_6d{_SUFFIX}.md"
JSON_PATH = (ROOT / "analysis" / "data_cache"
             / f"k4_config_transition_matrix_1min_6d{_SUFFIX}.json")


def gamma6_to_index(sigmas: tuple[int, ...]) -> int:
    """6 位三进制：σ∈{-1,0,+1}→{0,1,2}，高位=EDGES6[0]。"""
    idx = 0
    for s in sigmas:
        idx = idx * 3 + (s + 1)
    return idx


def index_to_gamma6(idx: int) -> tuple[int, ...]:
    out = []
    for _ in range(6):
        out.append(idx % 3 - 1)
        idx //= 3
    return tuple(reversed(out))


def _slabel(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[s]


def gamma6_label(idx: int) -> str:
    return "".join(_slabel(s) for s in index_to_gamma6(idx))


def k4_consistent(sigmas: tuple[int, ...]) -> bool:
    """6 条边 σ 是否可由 4 顶点势诱导（非平边有向图无矛盾循环 → 拓扑可排序）。

    σ_{num/den}=+1 → num 强于 den（num→den）；=-1 → den→num；=0 → 平局（无边）。
    """
    adj: dict[str, set[str]] = {v: set() for v in VERTS}
    indeg: dict[str, int] = {v: 0 for v in VERTS}
    edge_set: set[tuple[str, str]] = set()
    for (_, num, den), s in zip(EDGES6, sigmas):
        if s == 0:
            continue
        a, b = (num, den) if s == 1 else (den, num)
        if (a, b) not in edge_set:
            edge_set.add((a, b))
            adj[a].add(b)
            indeg[b] += 1
    queue = [v for v in VERTS if indeg[v] == 0]
    seen = 0
    indeg = dict(indeg)
    while queue:
        u = queue.pop()
        seen += 1
        for w in adj[u]:
            indeg[w] -= 1
            if indeg[w] == 0:
                queue.append(w)
    return seen == len(VERTS)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--procs", type=int, default=6)
    args = ap.parse_args()

    print("=" * 64)
    print(f"  1min a0 K4 6 维 Γ/729 态（anchor={MONEY_ANCHOR}, "
          f"M={ANCHOR_FILES[MONEY_ANCHOR]}）")
    print("=" * 64)

    # ── 6 条边 σ（lib 共用：加载/比价/并行递归 resume；DX 首跑触发递归）──
    edge_results = compute_or_load_edge_sigmas(years=0, procs=args.procs, verbose=True)
    sig = {name: dict(zip(r.days, r.sigma)) for name, r in edge_results.items()}

    common_days = sorted(set.intersection(*[set(s) for s in sig.values()]))
    gamma_series: list[tuple[int, int]] = []
    for d in common_days:
        sigmas = tuple(sig[name][d] for name, _, _ in EDGES6)
        gamma_series.append((d, gamma6_to_index(sigmas)))
    n = len(gamma_series)
    if n == 0:
        raise SystemExit("共同交易日为 0——检查各边数据时间范围是否重叠。")

    state_count: dict[int, int] = defaultdict(int)
    for _, gi in gamma_series:
        state_count[gi] += 1

    trans: dict[tuple[int, int], int] = defaultdict(int)
    for k in range(1, n):
        trans[(gamma_series[k - 1][1], gamma_series[k][1])] += 1
    diag = sum(c for (i, j), c in trans.items() if i == j)
    offdiag = sum(c for (i, j), c in trans.items() if i != j)
    total_trans = max(1, diag + offdiag)

    runs: dict[int, list[int]] = defaultdict(list)
    cur, length = gamma_series[0][1], 1
    for k in range(1, n):
        gi = gamma_series[k][1]
        if gi == cur:
            length += 1
        else:
            runs[cur].append(length)
            cur, length = gi, 1
    runs[cur].append(length)

    out_off: dict[int, int] = defaultdict(int)
    for (i, j), c in trans.items():
        if i != j:
            out_off[i] += c
    absorbing = [s for s in state_count if out_off.get(s, 0) == 0]

    consistent_days = sum(
        1 for _, gi in gamma_series if k4_consistent(index_to_gamma6(gi))
    )
    consistent_states = {gi for gi in state_count if k4_consistent(index_to_gamma6(gi))}
    n_states = len(state_count)
    theory_consistent = sum(
        1 for idx in range(729) if k4_consistent(index_to_gamma6(idx))
    )

    proj3: dict[int, int] = defaultdict(int)
    for _, gi in gamma_series:
        s = index_to_gamma6(gi)
        i3 = (s[0] + 1) * 9 + (s[1] + 1) * 3 + (s[2] + 1)
        proj3[i3] += 1

    _write_report(edge_results, gamma_series, state_count, trans, runs, absorbing,
                  diag, offdiag, total_trans, n, n_states,
                  consistent_days, consistent_states, theory_consistent, proj3)

    JSON_PATH.write_text(json.dumps({
        "meta": {
            "task": "k4_config_transition_matrix_1min_6d",
            "engine": "newchan_rust.RecursiveOrchestrator",
            "money_anchor": MONEY_ANCHOR,
            "anchor_file": ANCHOR_FILES[MONEY_ANCHOR],
            "gamma_dim": 6,
            "n_states_theory": 729,
            "edges6": [e[0] for e in EDGES6],
            "vertex_mapping": {"P": "ES", "M": MONEY_ANCHOR, "C": "CL", "R": "ZN"},
            "divergence_note": (
                "528/529 裁决 A：M=DX 正典美元锚，GC(Au)/CL(Oil) 折叠通道观测量。"
                if MONEY_ANCHOR == "DX" else
                f"M={MONEY_ANCHOR} 锚（非正典，见报告 §0）。"
            ),
            "epistemological_level": "L2",
            "generated": datetime.now().isoformat(),
        },
        "edges": {name: {"n_bars": r.n_bars, "n_days": len(r.days),
                         "max_level": r.max_level, "elapsed_s": r.elapsed}
                  for name, r in edge_results.items()},
        "common_days": n,
        "gamma_series": [[int(d), int(gi)] for d, gi in gamma_series],
        "state_count": {str(k): int(v) for k, v in state_count.items()},
        "transition": {f"{i}->{j}": int(c) for (i, j), c in trans.items()},
        "k4_consistency": {
            "consistent_days": consistent_days,
            "consistent_states_visited": len(consistent_states),
            "states_visited": n_states,
            "theory_consistent_of_729": theory_consistent,
        },
    }))
    print(f"\n完成。anchor={MONEY_ANCHOR} 共同交易日={n} 访问态={n_states}/729 "
          f"K4一致日={consistent_days}({100*consistent_days/n:.1f}%) "
          f"对角={diag}({100*diag/total_trans:.1f}%)")
    print(f"报告：{REPORT_PATH}")


def _vertex_table() -> list[str]:
    """anchor-aware 顶点映射表。"""
    L: list[str] = []
    if MONEY_ANCHOR == "DX":
        L.append("**528/529 编排者裁决 A：M=DX 美元指数正典锚**，黄金/原油降为折叠通道观测量"
                 "（[K4折叠通道模型]：Au=GC=C↔M 通道，Oil=CL=C→P 通道）。\n")
        L.append("| 顶点 | 标的 | 角色 | 备注 |")
        L.append("|------|------|------|------|")
        L.append("| P | ES | 生产资本 | 正典一致 ✓ |")
        L.append("| M | **DX(美元指数)** | 货币锚（正典）| 任务字面 DX(M)，σ_M=相对美元 ✓ |")
        L.append("| C | CL | 商品/原油 | Oil 折叠通道压平进 C → σ_C≡油 |")
        L.append("| R | ZN | 10年国债 | 528 覆盖：σ_R≡利率/债券，非不动产 |")
        L.append("| 通道 | GC(Au) | C↔M 折叠观测 | 不占顶点（黄金价值锚观测量）|")
        L.append("| 通道 | 6E(汇率) | M 强度交叉量 | 不占顶点（与 DX 高度反相关）|")
        L.append("")
        L.append("> **数据范围**：DX 1min 为 2018-2026（≈8 年），短于 GC 锚版（2012-2026≈13.6 年）；"
                 "DX 锚共同交易日数与态分布因此较少。**结论限 DX 数据起始后**，不可与 GC 锚版逐态"
                 "比较（边界条件①）。\n")
    else:
        L.append(f"货币锚 M = {ANCHOR_SYMBOL.get(MONEY_ANCHOR, MONEY_ANCHOR)}（非 DX 正典锚，"
                 f"σ_M=相对{MONEY_ANCHOR}方向）。\n")
        L.append("| 顶点 | 标的 |")
        L.append("|------|------|")
        L.append(f"| P | ES |")
        L.append(f"| M | {ANCHOR_SYMBOL.get(MONEY_ANCHOR, MONEY_ANCHOR)} |")
        L.append(f"| C | CL |")
        L.append(f"| R | ZN |")
        L.append("")
    return L


def _write_report(edge_results, gamma_series, state_count, trans, runs, absorbing,
                  diag, offdiag, total_trans, n, n_states,
                  consistent_days, consistent_states, theory_consistent, proj3):
    L: list[str] = []
    L.append(f"# 1min a0 K4 配置转换矩阵 —— 6 维 Γ / 729 态（M={MONEY_ANCHOR} 锚，Rust 引擎）\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　货币锚：**{MONEY_ANCHOR}"
             f"（{ANCHOR_FILES[MONEY_ANCHOR]}）**　共同交易日：{n}　"
             f"引擎：**newchan_rust.RecursiveOrchestrator**\n")

    L.append("## 0. 顶点映射\n")
    L.extend(_vertex_table())

    L.append("## 1. 各边递归概览\n")
    L.append("| 边 | bars | 交易日 | 最高级别 | 耗时 |")
    L.append("|----|------|--------|---------|------|")
    for name, _, _ in EDGES6:
        r = edge_results[name]
        L.append(f"| {name} | {r.n_bars:,} | {len(r.days)} | L{r.max_level} | {r.elapsed:.0f}s |")
    L.append("")

    L.append(f"## 2. 6 维 Γ 态分布（访问 {n_states}/729 态）\n")
    L.append("Γ = (σ_P/M, σ_C/M, σ_R/M, σ_P/C, σ_P/R, σ_C/R)，6 位：↑=+1/↓=−1/─=0\n")
    L.append("| Γ(6位) | 停留天数 | 占比 | K4一致 |")
    L.append("|--------|---------|------|--------|")
    for gi, cnt in sorted(state_count.items(), key=lambda x: -x[1])[:30]:
        ok = "✓" if k4_consistent(index_to_gamma6(gi)) else "✗循环"
        L.append(f"| {gamma6_label(gi)} | {cnt} | {100*cnt/n:.1f}% | {ok} |")
    L.append("")

    L.append("## 3. 转移统计\n")
    L.append(f"- 访问态：{n_states}/729（理论 K4 一致态上界 {theory_consistent}/729）")
    L.append(f"- 对角（停留）：{diag}（{100*diag/total_trans:.1f}%）")
    L.append(f"- 非对角（切换）：{offdiag}（{100*offdiag/total_trans:.1f}%）")
    L.append(f"- 唯一非零转移对：{len(trans)}")
    L.append("")
    L.append("### 转移矩阵 T[i→j]（非零，降序 top 30）\n")
    L.append("| 从 Γ | 到 Γ' | 次数 | 类型 |")
    L.append("|------|-------|------|------|")
    for (i, j), c in sorted(trans.items(), key=lambda x: -x[1])[:30]:
        L.append(f"| {gamma6_label(i)} | {gamma6_label(j)} | {c} | {'停留' if i == j else '切换'} |")
    L.append("")

    L.append("## 4. ★ K4 约束量化（核心信息增量，map-invariant）\n")
    L.append("任务述「理论 3^6=729 态，实际有 K4 约束」。K4 一致 = 6 条边 σ 可由 4 顶点势"
             "（全/偏序）诱导，即非平边有向图无矛盾循环（拓扑可排序）。\n")
    incons = n - consistent_days
    L.append(f"- **理论**：729 态中仅 {theory_consistent} 态（{100*theory_consistent/729:.1f}%）"
             f"K4 一致；其余 {729-theory_consistent} 态含矛盾循环（如 P≻C≻R≻P）。")
    L.append(f"- **经验**：{consistent_days}/{n} 交易日（{100*consistent_days/n:.1f}%）K4 一致；"
             f"访问的 {n_states} 态中 {len(consistent_states)} 态一致。")
    L.append(f"- **矛盾循环日**：{incons}（{100*incons/n:.1f}%）——6 条边最高涌现级别不同，"
             f"σ 无法由单一顶点势解释，K4 约束被经验破坏（缠论多级别背离候选标记）。")
    L.append("")
    L.append("> **认识论判读（231号有效域）**：一致率≈100% → 6 维 Γ 退化为 3 维（多出 3 边无信息）；"
             "显著 <100% → 6 维携带 3 维丢失的边间级别异质性信息。这是相对 27 态版的可证伪增量。\n")

    L.append("## 5. 平均驻留（经验游程）\n")
    L.append("| Γ(6位) | 游程数 | 平均驻留 | 最长 | 总停留 |")
    L.append("|--------|-------|---------|------|--------|")
    for mean_d, k, v in sorted([(np.mean(v), k, v) for k, v in runs.items()], reverse=True)[:12]:
        L.append(f"| {gamma6_label(k)} | {len(v)} | {mean_d:.1f} 天 | {max(v)} 天 | {sum(v)} 天 |")
    all_runs = [r for v in runs.values() for r in v]
    L.append("")
    L.append(f"- 全局平均驻留：{np.mean(all_runs):.2f} 天　中位数：{np.median(all_runs):.0f} 天　"
             f"最长：{max(all_runs)} 天\n")

    L.append("## 6. 吸收态\n")
    if absorbing:
        L.append("**严格吸收态**（无切换出边）：")
        for s in sorted(absorbing, key=lambda x: -state_count[x]):
            L.append(f"- {gamma6_label(s)}　停留 {state_count[s]} 天")
    else:
        L.append("**无严格吸收态**——所有访问态都有切换出边。")
    L.append("")

    L.append("## 7. 6维→3维投影对照\n")
    L.append("3 维 Γ=(σ_P/M,σ_C/M,σ_R/M) 是 6 维前 3 分量投影。top 态：\n")
    L.append("| 3维Γ | 停留天数 |")
    L.append("|------|---------|")
    for i3, cnt in sorted(proj3.items(), key=lambda x: -x[1])[:8]:
        sp, sc, sr = i3 // 9 - 1, (i3 // 3) % 3 - 1, i3 % 3 - 1
        L.append(f"| ({_slabel(sp)}{_slabel(sc)}{_slabel(sr)}) | {cnt} |")
    L.append("")

    L.append("## 结果包（六要素）\n")
    L.append(f"**结论**：1min a0 K4 全 6 条边 σ（M={MONEY_ANCHOR} 锚）→ 6 维 Γ（729 理论态，"
             f"访问 {n_states} 态）转换矩阵；K4 经验一致率 {100*consistent_days/n:.1f}%，"
             f"全局平均驻留 {np.mean(all_runs):.2f} 天，"
             f"{'有'+str(len(absorbing))+'个吸收态' if absorbing else '无严格吸收态'}。\n")
    L.append("**定义依据**：a_move_v1 走势/a_zhongshu≥3段重叠/newchan_rust 递归（逐位等价Python）；"
             "527号 σ=走势方向态；K4=4顶点完全图6边；K4一致=非平边有向图拓扑可排序。\n")
    L.append(f"**边界条件**：① M={MONEY_ANCHOR} 锚数据范围限定结论起始（DX:2018-2026）——"
             "态分布随锚变，K4一致性 map-invariant；② σ 取最高涌现级别（随级别漂移）；"
             "③ 仅共同交易日；④ 期货换月跳变未back-adjust；⑤ 若 K4 一致率→100%，6维退化为3维。\n")
    L.append("**下游推论**：K4 一致率量化「6条边方向是否被单一regime势支配」；"
             "高一致=单一资本旋转相位，低一致=多级别背离；矛盾循环日=缠论级别背离候选标记。\n")
    L.append("**谱系引用**：527号(σ走势方向态)、528/529号(顶点映射,裁决A=DX正典锚)、"
             "231号(有效域≠定义域,K4一致性L2可证伪)、254号(黄金∈Σ∩C,DX锚下降为折叠通道)；"
             "记忆[K4折叠通道模型][K4 1min期货映射分歧]。\n")
    L.append("**影响声明**：k4_1min_rust_lib 货币锚参数化(env K4_MONEY_ANCHOR)+cache锚隔离+"
             "compute_or_load_edge_sigmas(DRY,27态版共用)；本脚本anchor-aware；"
             f"产出按锚加后缀(本次={_SUFFIX or 'GC无后缀'})不覆盖；不改引擎/data_mapping。\n")
    L.append("**认识论等级**：L2（真实1min期货数据;K4约束假设可证伪）。"
             "管线L0/L1（Rust引擎bit-exact）。")

    REPORT_PATH.write_text("\n".join(L))


if __name__ == "__main__":
    main()
