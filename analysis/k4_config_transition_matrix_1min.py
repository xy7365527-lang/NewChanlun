#!/usr/bin/env python3
"""1min a0 K4 配置转换矩阵（**Rust 引擎驱动**）。

四标的 {ES, GC, CL, ZN} 以黄金 GC 为货币锚 M（P=ES,C=CL,R=ZN），构造六条对数
包络比价 OHLC，每条边从 1min a0 用 **Rust** RecursiveOrchestrator 缠论递归，逐 UTC
日取最高涌现级别走势方向 σ。三条 vertex/M 边组装 Γ=(σ_P,σ_C,σ_R)（27 态），统计：
  · 27×27 配置转换矩阵（观测频率 + 行归一化概率）
  · 吸收态（出度全自持的态）
  · 平均驻留时间（经验游程 + 几何近似）
  · 转换路径偏好（每态最可能的下一态）

顶点映射 / 528·529 分歧 / 认识论等级见 k4_1min_rust_lib.py 模块头注（L2，非正典 K4）。

用法：
  PYTHONPATH=src .venv/bin/python analysis/k4_config_transition_matrix_1min.py \
      [--years N] [--procs P]
    --years N : 仅用最近 N 年（快速验证；缺省=全量）
    --procs P : 进程数（缺省=6）
关键：必须用 .venv/bin/python（newchan_rust 装在 .venv）。
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

from analysis.k4_1min_rust_lib import (  # noqa: E402
    GAMMA_EDGES,
    MONEY_ANCHOR,
    USER_EDGE_MAP,
    compute_or_load_edge_sigmas,
)

REPORT_PATH = ROOT / "analysis" / "k4_config_transition_matrix_1min.md"
JSON_PATH = ROOT / "analysis" / "data_cache" / "k4_config_transition_matrix_1min.json"


# ════════════════════════════════════════════════════════════
# 27 态编码：Γ=(σ_P,σ_C,σ_R), σ∈{-1,0,+1}
# ════════════════════════════════════════════════════════════

def gamma_to_index(sp: int, sc: int, sr: int) -> int:
    return (sp + 1) * 9 + (sc + 1) * 3 + (sr + 1)


def index_to_gamma(idx: int) -> tuple[int, int, int]:
    return idx // 9 - 1, (idx // 3) % 3 - 1, idx % 3 - 1


def sigma_label(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[s]


def gamma_label(idx: int) -> str:
    sp, sc, sr = index_to_gamma(idx)
    return f"({sigma_label(sp)}{sigma_label(sc)}{sigma_label(sr)})"


# ════════════════════════════════════════════════════════════
# 转换矩阵分析
# ════════════════════════════════════════════════════════════

def row_normalize(T: np.ndarray) -> np.ndarray:
    """行归一化 → 经验转移概率 P[i][j]=T[i][j]/Σ_j T[i][j]（空行保持 0）。"""
    P = np.zeros_like(T, dtype=np.float64)
    rs = T.sum(axis=1)
    nz = rs > 0
    P[nz] = T[nz] / rs[nz, None]
    return P


def empirical_dwell_times(gamma_series: list[tuple[int, int]]) -> dict[int, list[int]]:
    """经验驻留游程：连续相同 Γ 的游程长度（按 Γ 分组），无分布假设。"""
    runs: dict[int, list[int]] = {}
    if not gamma_series:
        return runs
    cur = gamma_series[0][1]
    length = 1
    for k in range(1, len(gamma_series)):
        gi = gamma_series[k][1]
        if gi == cur:
            length += 1
        else:
            runs.setdefault(cur, []).append(length)
            cur = gi
            length = 1
    runs.setdefault(cur, []).append(length)
    return runs


def sigma_correlation(gamma_series: list[tuple[int, int]]) -> dict:
    """Γ 三分量 (σ_P,σ_C,σ_R) 的 Pearson 相关矩阵 + 有效自由度。

    检验「共享货币锚 GC 致分量非独立」（gemini 异质否定6 / 230号直积退化）。
    有效自由度 = 相关矩阵特征值的 participation ratio (Σλ)²/Σλ²。
    """
    arr = np.array([index_to_gamma(g) for _, g in gamma_series], dtype=np.float64)
    C = np.corrcoef(arr.T)
    ev = np.linalg.eigvalsh(C)
    pr = float((ev.sum() ** 2) / (ev ** 2).sum())
    allup = int(sum(1 for r in arr if tuple(r) == (1, 1, 1)))
    alldn = int(sum(1 for r in arr if tuple(r) == (-1, -1, -1)))
    return {
        "corr": C, "eigenvalues": np.sort(ev)[::-1],
        "participation_ratio": pr, "allup": allup, "alldn": alldn,
    }


def find_absorbing(T: np.ndarray) -> list[int]:
    """严格吸收态：被访问过且所有出边都回到自身（offdiag 行和==0）。"""
    absorbing = []
    for i in range(27):
        if T[i].sum() == 0:
            continue
        offdiag = T[i].sum() - T[i][i]
        if offdiag == 0:
            absorbing.append(i)
    return absorbing


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--years", type=int, default=0, help="仅用最近 N 年（0=全量）")
    ap.add_argument("--procs", type=int, default=6)
    args = ap.parse_args()

    # 27 态版叙述/meta 硬编码黄金锚（GC）。DX/6E 锚是 anchor-aware 6 维脚本的职责。
    if MONEY_ANCHOR != "GC":
        raise SystemExit(
            f"27 态版仅支持 GC 锚（报告与 meta 硬编码黄金锚），当前 "
            f"K4_MONEY_ANCHOR={MONEY_ANCHOR}。DX/6E 锚请用 anchor-aware 的 "
            f"k4_config_transition_matrix_1min_6d.py。"
        )
    t_start = time.time()
    print("=" * 64)
    print("  1min a0 K4 配置转换矩阵（Rust 引擎）")
    print("=" * 64)

    # ── 加载/比价/六边并行 Rust 递归（resume，lib 共用函数 DRY）──
    edge_results = compute_or_load_edge_sigmas(
        years=args.years, procs=args.procs, verbose=True
    )

    # ── 组装逐日 Γ + 转换矩阵 ──
    print("\n[4/5] 组装逐日 Γ=(σ_P,σ_C,σ_R) 并统计转换矩阵 ...", flush=True)
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
    gamma_series: list[tuple[int, int]] = []
    level_series: list[tuple[int, int, int]] = []
    for d in common_days:
        sp = sig_maps["P/M"][d]
        sc = sig_maps["C/M"][d]
        sr = sig_maps["R/M"][d]
        gamma_series.append((d, gamma_to_index(sp, sc, sr)))
        level_series.append((lvl_maps["P/M"][d], lvl_maps["C/M"][d], lvl_maps["R/M"][d]))

    # 转换矩阵 T[i][j]：相邻共同交易日 Γ→Γ' 计数
    T = np.zeros((27, 27), dtype=np.int64)
    for k in range(1, len(gamma_series)):
        T[gamma_series[k - 1][1]][gamma_series[k][1]] += 1

    state_count = np.zeros(27, dtype=np.int64)
    for _, gi in gamma_series:
        state_count[gi] += 1

    # ── 写报告 ──
    print("\n[5/5] 写报告 ...", flush=True)
    total_elapsed = time.time() - t_start
    write_report(edge_results, gamma_series, level_series, T, state_count,
                 common_days, total_elapsed, args)
    dump_json(edge_results, gamma_series, T, state_count, args)
    print(f"\n总耗时：{total_elapsed:.0f}s")
    print(f"报告：{REPORT_PATH}")


def write_report(edge_results, gamma_series, level_series, T, state_count,
                 common_days, total_elapsed, args) -> None:
    P = row_normalize(T)
    dwell = empirical_dwell_times(gamma_series)
    absorbing = find_absorbing(T)
    diag = int(np.trace(T))
    offdiag = int(T.sum()) - diag
    n_visited = int((state_count > 0).sum())
    total_trans = max(1, diag + offdiag)

    L: list[str] = []
    L.append("# 1min a0 K4 配置转换矩阵（Rust 引擎）\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
             f"总耗时：{total_elapsed:.0f}s　"
             f"数据范围：{'最近 ' + str(args.years) + ' 年' if args.years else '全量'}　"
             f"引擎：**newchan_rust.RecursiveOrchestrator**\n")

    # ⚠ 顶点映射分歧
    L.append("## ⚠ 顶点映射（GC 货币锚版，诚实标注）\n")
    L.append("用户标的集 {ES,GC,CL,ZN} 无 USD6E，故以**黄金 GC 为货币锚 M**"
             "（卢麒元金油两锚；黄金∈Σ∩C 且传统价值锚）：\n")
    L.append("| 顶点 | 本任务 | 正典(528/529) | 分歧 |")
    L.append("|------|--------|--------------|------|")
    L.append("| P | ES (生产资本) | ES/UUP系 | 一致 ✓ |")
    L.append("| M | **GC (黄金)** | UUP/USD | ⚠ 黄金作货币锚(非美元) → σ 是『相对黄金』方向 |")
    L.append("| C | CL (原油) | DBC (广义商品) | ⚠ Oil 折叠通道压平进 C → σ_C≡油 |")
    L.append("| R | ZN (10年国债) | VNQ (不动产) | ⚠⚠ 528号判国债属M；编排者覆盖 → σ_R≡利率 |")
    L.append("")
    L.append("**Γ 边方向**：取 vertex/M（P/M=ES/GC, C/M=CL/GC, R/M=ZN/GC），"
             "使 σ_C/σ_R 为 vertex-相对-money 方向。用户列举的 GC/CL、GC/ZN 是 C/M、R/M "
             "的**倒数**（缠论趋势方向相反），本实现直接构造 vertex/M 边避免倒数近似。\n")
    L.append("**认识论后果**（231号）：非正典 K4 顶点集，σ_C≡油、σ_R≡利率、M≡黄金。"
             "结论**不可与正典 K4 regime 节点直接比较**。L2 真实数据。\n")

    # 用户边映射表
    L.append("### 用户列举边 → 本实现规范边\n")
    L.append("| 用户边 | 规范边 | σ 关系 |")
    L.append("|--------|--------|--------|")
    for ue, (ce, rel) in USER_EDGE_MAP.items():
        L.append(f"| {ue} | {ce} | {rel} |")
    L.append("")

    # 方法
    L.append("## 方法\n")
    L.append("- **引擎**：newchan_rust.RecursiveOrchestrator（Rust，逐位等价 Python 版，~23×）")
    L.append("- **比价**：对数包络 R_high=A_high/B_low, R_low=A_low/B_high（不低估盘内极差）")
    L.append("- **a0**：1min，级别自然涌现，max_levels=6，stroke_mode=wide")
    L.append("- **σ**：每 UTC 日末取**最高涌现级别最后一个 move（settled=False，进行中走势）**"
             "的方向（趋势↑=+1/趋势↓=−1/盘整=0，527号走势方向态）。"
             "⚠ 这是『进行中走势方向』非『已结算走势方向』——日末后可被反向走势推翻"
             "（gemini/codex 异质审查收敛点）。zero-lookahead 仍成立（仅依赖已处理 bar）。")
    L.append("- **Γ**：三条 vertex→money 边 (P/M,C/M,R/M) 的 σ 三元组（27 态）")
    L.append("- **转移**：相邻共同交易日 Γ→Γ' 计数")
    L.append("- **zero-lookahead**：每日 σ 仅依赖该日及之前 bar（Rust 流式 process_bar）\n")

    # 1. 各边概览
    L.append("## 1. 各边递归概览\n")
    L.append("| 边 | bars | 交易日 | 最高级别 | 耗时 |")
    L.append("|----|------|--------|---------|------|")
    for r in edge_results.values():
        L.append(f"| {r.name} | {r.n_bars:,} | {len(r.days)} | L{r.max_level} | {r.elapsed:.0f}s |")
    L.append("")

    # 级别构成警示
    if level_series:
        lv_arr = np.array(level_series)
        L.append("### ⚠ 异级别 σ 组合警示（核心方法缺陷，非边界条件）\n")
        L.append("三条 Γ 边各自最高级别的日频分布：\n")
        L.append("| 边 | L1 | L2 | L3 | L4 | L5+ |")
        L.append("|----|----|----|----|----|----|")
        same_lvl = 0
        for ci, name in enumerate(GAMMA_EDGES):
            col = lv_arr[:, ci]
            counts = [int((col == lv).sum()) for lv in (1, 2, 3, 4)]
            l5p = int((col >= 5).sum())
            L.append(f"| {name} | {counts[0]} | {counts[1]} | {counts[2]} | {counts[3]} | {l5p} |")
        same_lvl = int(np.sum((lv_arr[:, 0] == lv_arr[:, 1]) & (lv_arr[:, 1] == lv_arr[:, 2])))
        L.append("")
        L.append(f"> **gemini+codex 异质审查收敛否定**：仅 {same_lvl}/{len(lv_arr)} "
                 f"（{same_lvl / len(lv_arr) * 100:.0f}%）的交易日三条边处于**同一**涌现级别——"
                 f"其余 {100 - same_lvl / len(lv_arr) * 100:.0f}% 的日子，Γ 的三个分量来自**不同时间尺度**"
                 f"的走势（如 σ_P@L3≈1-2年走势 与 σ_C@L4≈5年走势）。这不是『随时间漂移』的边界条件，"
                 f"而是**逻辑上的苹果比橙子**：Γ 把异级别方向并列为『同日配置』，驻留分布的"
                 f"『配置粘滞』解释被级别异质性混淆。可证伪：固定全 L3 重跑，若驻留分布显著变则"
                 f"异级别混合是粘滞性主因。\n")

    # 2. 配置停留分布
    L.append(f"## 2. 配置 Γ 停留分布（共 {len(gamma_series)} 交易日，访问 {n_visited}/27 态）\n")
    L.append("| Γ | (σ_P,σ_C,σ_R) | 停留天数 | 占比 | 极性 S |")
    L.append("|----|---------------|---------|------|--------|")
    order = np.argsort(-state_count)
    total_days = max(1, int(state_count.sum()))
    for gi in order:
        cnt = int(state_count[gi])
        if cnt == 0:
            continue
        sp, sc, sr = index_to_gamma(int(gi))
        L.append(f"| {gamma_label(int(gi))} | ({sp:+d},{sc:+d},{sr:+d}) | "
                 f"{cnt} | {cnt / total_days * 100:.1f}% | {sp + sc + sr:+d} |")
    L.append("")

    # 3. 转换矩阵（非零转移，降序）
    L.append("## 3. 配置转换矩阵 T[i→j]（非零转移，降序 top 40）\n")
    L.append("| 从 Γ | 到 Γ' | 次数 | P(j\\|i) | 类型 |")
    L.append("|------|-------|------|--------|------|")
    transitions = [(int(T[i][j]), i, j) for i in range(27) for j in range(27) if T[i][j] > 0]
    transitions.sort(reverse=True)
    for cnt, i, j in transitions[:40]:
        kind = "停留" if i == j else "切换"
        L.append(f"| {gamma_label(i)} | {gamma_label(j)} | {cnt} | {P[i][j]:.3f} | {kind} |")
    L.append("")

    # 4. 转移统计
    L.append("## 4. 转移统计\n")
    L.append(f"- 访问过的配置数：{n_visited}/27")
    L.append(f"- 停留（对角）转移：{diag}（{diag / total_trans * 100:.1f}%）")
    L.append(f"- 切换（非对角）转移：{offdiag}（{offdiag / total_trans * 100:.1f}%）")
    L.append(f"- 唯一非零转移对：{len(transitions)}")
    L.append("")
    # 方法学警示：高对角占比是日频采样伪影，非强粘滞结论
    L.append(f"> ⚠ **方法学警示（信息增量诚实标注，231号/形式化有效域规则）**："
             f"{diag / total_trans * 100:.1f}% 对角占比是**日频采样的伪影**——相邻交易日的 Γ "
             f"高度自相关（走势级别 ≥L3，单日难翻转），故转移矩阵近似单位阵。这**不是**『系统"
             f"极度粘滞』的强结论，而与逐日重采样同义反复（类比[回测基准不可证伪陷阱]）。"
             f"真正的信息增量在**切换结构**（§7 路径偏好，仅 {offdiag} 次非对角转移）与"
             f"**驻留分布**（§6 游程），不在对角占比本身。\n")

    # 5. 吸收态
    L.append("## 5. 吸收态分析\n")
    if absorbing:
        L.append("**严格吸收态**（出度全自持，offdiag 行和=0）：")
        for i in absorbing:
            L.append(f"- {gamma_label(i)}　停留 {int(state_count[i])} 天")
        L.append("")
    else:
        L.append("**无严格吸收态**——所有被访问的配置都有切换出边（系统遍历，无陷阱）。\n")
    # 准吸收：自持概率 P_ii 最高 top 5（被访问 ≥5 天）
    L.append("**准吸收态**（自持概率 P(i→i) 最高，停留≥5天）：\n")
    L.append("| Γ | 停留天数 | P(i→i) | 平均驻留(几何 1/(1−P_ii)) |")
    L.append("|----|---------|--------|--------------------------|")
    quasi = sorted(
        [(P[i][i], i) for i in range(27) if state_count[i] >= 5],
        reverse=True,
    )
    for pii, i in quasi[:8]:
        geo = 1.0 / (1.0 - pii) if pii < 1.0 else float("inf")
        geo_s = "∞" if geo == float("inf") else f"{geo:.1f}"
        L.append(f"| {gamma_label(i)} | {int(state_count[i])} | {pii:.3f} | {geo_s} 天 |")
    L.append("")

    # 6. 平均驻留时间（经验游程）
    L.append("## 6. 平均驻留时间（经验游程，无分布假设）\n")
    L.append("连续停留同一 Γ 的游程长度均值（按 Γ 分组）：\n")
    L.append("| Γ | 游程数 | 平均驻留 | 最长游程 | 总停留天 |")
    L.append("|----|-------|---------|---------|---------|")
    dwell_rows = sorted(
        [(np.mean(v), k, v) for k, v in dwell.items()],
        reverse=True,
    )
    for mean_d, k, v in dwell_rows[:12]:
        L.append(f"| {gamma_label(k)} | {len(v)} | {mean_d:.2f} 天 | {max(v)} 天 | {sum(v)} 天 |")
    L.append("")
    all_runs = [r for v in dwell.values() for r in v]
    if all_runs:
        L.append(f"- 全局平均驻留：{np.mean(all_runs):.2f} 天　"
                 f"中位数：{np.median(all_runs):.0f} 天　"
                 f"最长：{max(all_runs)} 天\n")

    # 7. 转换路径偏好
    L.append("## 7. 转换路径偏好（每态最可能的下一态，排除自持）\n")
    L.append("| 当前 Γ | 停留天数 | 最可能切换至 | P(切换\\|i,j≠i) |")
    L.append("|--------|---------|-------------|----------------|")
    pref_rows = []
    for i in range(27):
        if state_count[i] < 5:
            continue
        off = T[i].copy()
        off[i] = 0
        if off.sum() == 0:
            continue
        j = int(np.argmax(off))
        p_switch = off[j] / off.sum()
        pref_rows.append((int(state_count[i]), i, j, p_switch))
    pref_rows.sort(reverse=True)
    for cnt, i, j, p_switch in pref_rows[:15]:
        L.append(f"| {gamma_label(i)} | {cnt} | {gamma_label(j)} | {p_switch:.3f} |")
    L.append("")

    # 结果包
    L.append("## 结果包（六要素）\n")
    L.append("**结论**：1min a0 六边 **Rust** 缠论递归 → 逐日最高涌现级别 σ → "
             f"{len(gamma_series)} 交易日 Γ 序列与 27 态转换矩阵（访问 {n_visited}/27 态，"
             f"{'有 ' + str(len(absorbing)) + ' 个严格吸收态' if absorbing else '无严格吸收态'}，"
             f"全局平均驻留 {np.mean(all_runs):.2f} 天）。\n" if all_runs else "**结论**：无数据。\n")
    L.append("**定义依据**：a_move_v1 走势(盘整1中枢/趋势2+同向中枢)；a_zhongshu 至少3段重叠；"
             "newchan_rust 递归(settled move→上级线段，逐位等价 Python)；527号 σ=走势方向态；"
             "对数包络比价 K 线。\n")
    L.append("**边界条件**：① σ 取『最高涌现级别』使 Γ 可观测量随级别漂移——固定级别会改变矩阵；"
             "② 各品种交易时段不同，仅保留共同分钟；③ 期货连续合约换月在比价处引入跳变"
             "（未 back-adjust）；④ money 锚改回 USD6E/UUP 或 R 改回 VNQ，则 σ 语义翻转、矩阵不可比；"
             "⑤ GC↔CL/ZN 倒数边趋势取反在中枢非完全对称处可能与直接构造的 σ 偏离。\n")
    L.append("**下游推论**：Γ 序列 regime 结构可输入闭合残差/资本旋转分析；高对角占比=配置粘滞"
             "=趋势持续；高切换率=震荡/转折密集；吸收态（若有）=不可逃逸 regime。\n")
    L.append("**谱系引用**：528号(顶点映射冲突,国债≠R)、529号(折叠通道重构)、527号(σ走势方向态/81边)、"
             "254号(黄金∈Σ∩C)、231号(有效域≠定义域)；记忆[K4 1min期货映射分歧]"
             "(编排者覆盖 R=ZN)。GC 作 M 是本任务对标的集 {ES,GC,CL,ZN} 的映射选择，"
             "区别于 k4_1min_lib 的 USD6E 锚版。\n")
    L.append("**影响声明**：新建 k4_1min_rust_lib.py + 本脚本 + 报告 + JSON；"
             "不修改引擎；不改 data_mapping.py 单一真相源（分歧仅存在于本实验脚本）。\n")
    L.append("**认识论等级**：L2（真实 1min 期货数据，非合成；可产生否定性结果）。"
             "管线正确性 L0/L1（Rust 引擎 bit-exact 由记忆[RecursiveOrchestrator全Rust化]保证）。")

    REPORT_PATH.write_text("\n".join(L))


def dump_json(edge_results, gamma_series, T, state_count, args) -> None:
    out = {
        "meta": {
            "task": "k4_config_transition_matrix_1min",
            "engine": "newchan_rust.RecursiveOrchestrator",
            "years": args.years,
            "generated": datetime.now().isoformat(),
            "vertex_mapping": {"P": "ES", "M": "GC", "C": "CL", "R": "ZN"},
            "divergence_note": "非正典: M=GC(黄金锚), C=CL(油,非DBC), R=ZN(国债,非VNQ)",
            "epistemological_level": "L2",
        },
        "edges": {
            name: {"n_bars": r.n_bars, "n_days": len(r.days),
                   "max_level": r.max_level, "elapsed_s": r.elapsed,
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
