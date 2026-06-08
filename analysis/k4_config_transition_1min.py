#!/usr/bin/env python3
"""任务1：1min a0 K4 配置转换矩阵。

用 1min 数据构造 K4 六条边对数包络比价 OHLC，每条边从 1min a0 开始缠论递归，
取**涌现的最高级别**走势方向 σ。每个 UTC 日计算 Γ=(σ_P, σ_C, σ_R) 配置 →
27 态转换矩阵 T[i][j]（日频转移计数）。

六条边并行（multiprocessing，局部依赖原则 275 号）。

顶点映射与 528/529 分歧、认识论等级见 k4_1min_lib.py 模块头注（L2，非正典 K4）。

用法：
  PYTHONPATH=src python analysis/k4_config_transition_1min.py [--years N] [--procs P]
    --years N : 仅用最近 N 年数据（快速验证；缺省=全量 16 年）
    --procs P : 进程数（缺省=6）
"""

from __future__ import annotations

import argparse
import json
import multiprocessing as mp
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for p in (str(ROOT), str(ROOT / "src")):
    if p not in sys.path:
        sys.path.insert(0, p)

from analysis.k4_1min_lib import (
    EDGES,
    GAMMA_EDGES,
    VERTEX_FILES,
    EdgeDailySigma,
    _worker,
    build_ratio_for_edge,
    load_series,
)

REPORT_PATH = ROOT / "analysis" / "k4_config_transition_1min.md"
JSON_PATH = ROOT / "analysis" / "data_cache" / "k4_config_transition_1min.json"


# ════════════════════════════════════════════════════════════
# 27 态编码
# ════════════════════════════════════════════════════════════

def gamma_to_index(sp: int, sc: int, sr: int) -> int:
    """Γ=(σ_P,σ_C,σ_R), σ∈{-1,0,+1} → 0..26（每位 +1 后三进制）。"""
    return (sp + 1) * 9 + (sc + 1) * 3 + (sr + 1)


def index_to_gamma(idx: int) -> tuple[int, int, int]:
    sp = idx // 9 - 1
    sc = (idx // 3) % 3 - 1
    sr = idx % 3 - 1
    return sp, sc, sr


def sigma_label(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[s]


def gamma_label(idx: int) -> str:
    sp, sc, sr = index_to_gamma(idx)
    return f"({sigma_label(sp)}{sigma_label(sc)}{sigma_label(sr)})"


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--years", type=int, default=0, help="仅用最近 N 年（0=全量）")
    ap.add_argument("--procs", type=int, default=6)
    args = ap.parse_args()

    t_start = time.time()
    print("=" * 64)
    print("  任务1：1min a0 K4 配置转换矩阵")
    print("=" * 64)

    # ── 加载顶点序列 ──
    print("\n[1/5] 加载顶点 1min 序列 ...", flush=True)
    series = {}
    for v, fn in VERTEX_FILES.items():
        s = load_series(fn)
        series[v] = s
        print(f"  {v} = {s.symbol:8s}  {len(s.c):>10,} bars", flush=True)

    # ── 构造六条边对数包络比价 ──
    print("\n[2/5] 构造六条边对数包络比价 ...", flush=True)
    ratios = {}
    for edge in EDGES:
        rb = build_ratio_for_edge(edge, series)
        # 可选时间裁剪
        if args.years > 0:
            cutoff = int(
                datetime.now(timezone.utc).timestamp()
            ) - args.years * 365 * 86400
            mask = rb.ts >= cutoff
            rb = type(rb)(
                name=rb.name, ts=rb.ts[mask], day=rb.day[mask],
                o=rb.o[mask], h=rb.h[mask], l=rb.l[mask], c=rb.c[mask],
            )
        ratios[edge[0]] = rb
        print(f"  {rb.name:5s} {len(rb.c):>10,} bars  "
              f"{np.datetime64(int(rb.ts[0]),'s')}..{np.datetime64(int(rb.ts[-1]),'s')}",
              flush=True)

    # ── 六边并行缠论递归 ──
    print(f"\n[3/5] 六边并行缠论递归（procs={args.procs}）...", flush=True)
    job_args = [
        (rb.name, rb.ts, rb.day, rb.o, rb.h, rb.l, rb.c, 6)
        for rb in ratios.values()
    ]
    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=args.procs) as pool:
        results_list: list[EdgeDailySigma] = pool.map(_worker, job_args)
    edge_results = {r.name: r for r in results_list}
    for r in results_list:
        print(f"  {r.name:5s} done: {r.n_bars:>10,} bars, {len(r.days)} days, "
              f"max_lvl=L{r.max_level}, {r.elapsed:.0f}s", flush=True)

    # ── 组装逐日 Γ ──
    print("\n[4/5] 组装逐日 Γ=(σ_P,σ_C,σ_R) 并统计转换矩阵 ...", flush=True)
    # 每条 gamma 边: day -> sigma
    sig_maps = {
        name: dict(zip(edge_results[name].days, edge_results[name].sigma))
        for name in GAMMA_EDGES
    }
    lvl_maps = {
        name: dict(zip(edge_results[name].days, edge_results[name].level))
        for name in GAMMA_EDGES
    }
    common_days = sorted(
        set(sig_maps["P/M"]) & set(sig_maps["C/M"]) & set(sig_maps["R/M"])
    )
    gamma_series: list[tuple[int, int]] = []  # (day, gamma_index)
    level_series: list[tuple[int, int, int]] = []  # 三边 level
    for d in common_days:
        sp = sig_maps["P/M"][d]
        sc = sig_maps["C/M"][d]
        sr = sig_maps["R/M"][d]
        gamma_series.append((d, gamma_to_index(sp, sc, sr)))
        level_series.append(
            (lvl_maps["P/M"][d], lvl_maps["C/M"][d], lvl_maps["R/M"][d])
        )

    # 转换矩阵 T[i][j]：相邻交易日 Γ 转移计数
    T = np.zeros((27, 27), dtype=np.int64)
    for k in range(1, len(gamma_series)):
        i = gamma_series[k - 1][1]
        j = gamma_series[k][1]
        T[i][j] += 1

    # 状态停留分布
    state_count = np.zeros(27, dtype=np.int64)
    for _, gi in gamma_series:
        state_count[gi] += 1

    # ── 写报告 ──
    print("\n[5/5] 写报告 ...", flush=True)
    total_elapsed = time.time() - t_start
    write_report(
        edge_results, gamma_series, level_series, T, state_count,
        common_days, total_elapsed, args,
    )
    dump_json(edge_results, gamma_series, T, state_count, args)
    print(f"\n总耗时：{total_elapsed:.0f}s")
    print(f"报告：{REPORT_PATH}")


def write_report(edge_results, gamma_series, level_series, T, state_count,
                 common_days, total_elapsed, args) -> None:
    L: list[str] = []
    L.append("# 任务1：1min a0 K4 配置转换矩阵\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
             f"总耗时：{total_elapsed:.0f}s　"
             f"数据范围：{'最近 '+str(args.years)+' 年' if args.years else '全量 16 年'}\n")

    # ⚠ 分歧标注
    L.append("## ⚠ 顶点映射分歧（诚实标注，no-workaround）\n")
    L.append("本实验按编排者显式指令使用期货代理，与 528/529 号单一真相源**实质分歧**：\n")
    L.append("| 顶点 | 本实验 | 正典(528/529) | 分歧 |")
    L.append("|------|--------|--------------|------|")
    L.append("| P | ES (生产资本) | ES/UUP系 | 一致 ✓ |")
    L.append("| M | USD6E (美元,已倒数) | UUP | 期货代理,概念一致 ✓ |")
    L.append("| C | **CL (原油)** | DBC (广义商品) | ⚠ CL是Oil折叠通道观测量,压平进C顶点→σ_C测油 |")
    L.append("| R | **ZN (10年国债)** | VNQ (不动产) | ⚠⚠ 528号判『国债当R』为错误(久期属M)→σ_R测利率非不动产 |")
    L.append("")
    L.append("**认识论后果**（231号）：转换矩阵建立在**非正典 K4 顶点集**上，"
             "σ_C≡油、σ_R≡利率。结论不可与正典 K4 regime 节点直接比较。L2 真实数据。\n")

    # σ 定义
    L.append("## 方法\n")
    L.append("- **比价**：对数包络法 R_high=A_high/B_low, R_low=A_low/B_high（不低估盘内极差）")
    L.append("- **a0**：1min，级别自然涌现，max_levels=6，stroke_mode=wide")
    L.append("- **σ**：每 UTC 日末取**最高涌现级别**最后一个 move 的方向"
             "（趋势↑=+1/趋势↓=−1/盘整=0，527号走势方向态）")
    L.append("- **Γ**：三条 vertex→money 边 (P/M,C/M,R/M) 的 σ 三元组")
    L.append("- **转移**：相邻共同交易日 Γ→Γ' 计数")
    L.append("- **zero-lookahead**：每日 σ 仅依赖该日及之前 bar（流式增量）\n")

    # 各边
    L.append("## 1. 各边递归概览\n")
    L.append("| 边 | bars | 交易日 | 最高级别 | 耗时 |")
    L.append("|----|------|--------|---------|------|")
    for r in edge_results.values():
        L.append(f"| {r.name} | {r.n_bars:,} | {len(r.days)} | L{r.max_level} | {r.elapsed:.0f}s |")
    L.append("")

    # 级别构成（方法学警示）
    if level_series:
        lv_arr = np.array(level_series)
        L.append("### 级别构成警示\n")
        L.append("『最高涌现级别 σ』使 Γ 的可观测量随时间漂移（早期低级别/晚期高级别）。"
                 "三条边各自最高级别的日频分布：\n")
        L.append("| 边 | L1 | L2 | L3 | L4 | L5+ |")
        L.append("|----|----|----|----|----|----|")
        for ci, name in enumerate(GAMMA_EDGES):
            col = lv_arr[:, ci]
            counts = [int((col == lv).sum()) for lv in (1, 2, 3, 4)]
            l5p = int((col >= 5).sum())
            L.append(f"| {name} | {counts[0]} | {counts[1]} | {counts[2]} | {counts[3]} | {l5p} |")
        L.append("")

    # 配置停留分布
    L.append(f"## 2. 配置 Γ 停留分布（共 {len(gamma_series)} 交易日）\n")
    L.append("Γ 三元组按符号 (σ_P σ_C σ_R)，↑=+1 ↓=−1 ─=0：\n")
    L.append("| Γ | (σ_P,σ_C,σ_R) | 停留天数 | 占比 | 极性 S |")
    L.append("|----|---------------|---------|------|--------|")
    order = np.argsort(-state_count)
    total = max(1, int(state_count.sum()))
    for gi in order:
        cnt = int(state_count[gi])
        if cnt == 0:
            continue
        sp, sc, sr = index_to_gamma(int(gi))
        L.append(f"| {gamma_label(int(gi))} | ({sp:+d},{sc:+d},{sr:+d}) | "
                 f"{cnt} | {cnt/total*100:.1f}% | {sp+sc+sr:+d} |")
    L.append("")

    # 转换矩阵（只列非零转移，按计数降序）
    L.append("## 3. 配置转换矩阵 T[i→j]（非零转移，降序）\n")
    L.append("含对角（停留）与非对角（切换）。仅列 top 40：\n")
    L.append("| 从 Γ | 到 Γ' | 次数 | 类型 |")
    L.append("|------|-------|------|------|")
    transitions = []
    for i in range(27):
        for j in range(27):
            if T[i][j] > 0:
                transitions.append((int(T[i][j]), i, j))
    transitions.sort(reverse=True)
    for cnt, i, j in transitions[:40]:
        kind = "停留" if i == j else "切换"
        L.append(f"| {gamma_label(i)} | {gamma_label(j)} | {cnt} | {kind} |")
    L.append("")

    # 切换统计
    diag = int(np.trace(T))
    offdiag = int(T.sum()) - diag
    n_states_visited = int((state_count > 0).sum())
    L.append("## 4. 转移统计\n")
    L.append(f"- 访问过的配置数：{n_states_visited}/27")
    L.append(f"- 停留（对角）转移：{diag}（{diag/max(1,diag+offdiag)*100:.1f}%）")
    L.append(f"- 切换（非对角）转移：{offdiag}（{offdiag/max(1,diag+offdiag)*100:.1f}%）")
    L.append(f"- 唯一非零转移对：{len(transitions)}")
    L.append("")

    # 结果包
    L.append("## 结果包（六要素）\n")
    L.append("**结论**：1min a0 六边缠论递归 → 逐日最高涌现级别 σ → "
             f"{len(gamma_series)} 交易日的 Γ 序列与 27 态转换矩阵（访问 {n_states_visited}/27 态）。\n")
    L.append("**定义依据**：a_move_v1.py 走势(盘整1中枢/趋势2+同向中枢)；"
             "a_zhongshu 至少3段重叠；recursive_stack.py 递归(settled move→上级线段)；"
             "527号 σ=走势方向态；比价 K 线对数包络构造。\n")
    L.append("**边界条件**：① σ 取『最高涌现级别』使 Γ 可观测量随级别漂移——"
             "若改为固定级别，转换矩阵会变；② 不同品种交易时段不同，仅保留共同分钟；"
             "③ 期货连续合约换月在比价处引入跳变（未做 back-adjust）；"
             "④ 若顶点映射改回正典(C=DBC,R=VNQ)，σ_C/σ_R 语义翻转，矩阵不可比。\n")
    L.append("**下游推论**：Γ 序列的 regime 结构可输入任务2闭合残差；"
             "高停留率(对角占优)=配置粘滞=趋势持续；高切换率=震荡/转折密集期。\n")
    L.append("**谱系引用**：528号(顶点映射四套冲突,国债≠R)、529号(折叠通道重构)、"
             "527号(σ走势方向态/81边)、254号(黄金∈Σ∩C/油=结算通道)。"
             "本实验顶点映射是编排者对 528/529 单一真相源的**显式覆盖**（原则0+覆盖权）。\n")
    L.append("**影响声明**：新建 analysis/k4_1min_lib.py + 本脚本 + 报告；"
             "不修改引擎；不改 data_mapping.py 单一真相源（分歧仅存在于本实验脚本）。\n")
    L.append("**认识论等级**：L2（真实 1min 期货数据，非合成；可产生否定性结果）。")

    REPORT_PATH.write_text("\n".join(L))


def dump_json(edge_results, gamma_series, T, state_count, args) -> None:
    out = {
        "meta": {
            "task": "k4_config_transition_1min",
            "years": args.years,
            "generated": datetime.now().isoformat(),
            "vertex_mapping": {"P": "ES", "M": "USD6E", "C": "CL", "R": "ZN"},
            "divergence_note": "非正典: C=CL(油,非DBC), R=ZN(国债,非VNQ); 见528/529",
            "epistemological_level": "L2",
        },
        "edges": {
            name: {"n_bars": r.n_bars, "n_days": len(r.days),
                   "max_level": r.max_level, "elapsed_s": r.elapsed,
                   # 全部六边逐日 σ 序列 → 任务2（闭合残差）复用，避免重复重算
                   "days": [int(x) for x in r.days],
                   "sigma": [int(x) for x in r.sigma],
                   "level": [int(x) for x in r.level]}
            for name, r in edge_results.items()
        },
        "gamma_series": [[int(d), int(gi)] for d, gi in gamma_series],
        "transition_matrix": T.tolist(),
        "state_count": state_count.tolist(),
    }
    JSON_PATH.write_text(json.dumps(out))


if __name__ == "__main__":
    main()
