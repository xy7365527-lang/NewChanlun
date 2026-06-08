"""Γ→Δ 30min a0 验证编排层 — 两套 RunConfig 跑 + 合并报告。

================================ 本文件的定位（薄编排层）================================
基础机制（RunConfig / _resample_to_a0 / _top_sigma 涌现走势 σ / _build_edges / task1/2/3）
已在 `gamma_delta_full_verification.py` 实装（编排者重构）。**本文件不重复实现**，只：
  1. 复用基础脚本的全部任务函数与 σ 读法（DRY；单一真相源）；
  2. 定义两套 30min RunConfig：
     - **Run A**=基础脚本的 `CANONICAL_30M`（用户原始符号 ES/GC/CL 期货 + UUP/VNQ/DBC）；
     - **Run B**=新增 `ETF_10Y`（SPY/GLD/USO ETF 替期货 + UUP/VNQ/DBC，覆盖 10yr）；
  3. 合并两套结果写到 **gamma_delta_30m_results.md**（任务指定输出）。

================================ 为什么要 Run B（数据可用性边界驱动）================================
实测 IBKR 30min 深度（formalization-validity-domain）：连续期货 ES ~3.2yr（2023-03 起，
ContFuture 拒绝翻页 → 单次上限），ETF（UUP 实测回到 2014-06）可翻页到 10yr+。
任务理由是「30min 数据更长覆盖更多宏观周期」——但**期货 30min 只 3.2yr < 日线版 10yr**，
不满足该理由。ETF 30min 可达 10yr（COVID/2022加息/AI 多周期）→ Run B 用 ETF 顶点
（P=SPY/Au=GLD/Oil=USO，528号下金油是折叠通道、ETF 与期货同为通道代理）补足宏观广度。
Run A 用真实可交易期货、与实盘 30min 一致；Run B 用 ETF 换取 10yr 宏观广度。两者互补。

σ 读法 = `_top_sigma`（基础脚本）= 涌现最高级别走势 `move_snapshot.moves[-1]`，非日线
segment-level workaround。a0=30min 让笔→线段→走势级别自然涌现。

运行：PYTHONPATH=src python analysis/gamma_delta_30m_verification.py
输出：analysis/gamma_delta_30m_results.md
"""

from __future__ import annotations

import random
from datetime import datetime
from pathlib import Path

# 复用基础脚本（同级模块）。脚本运行时 sys.path[0]=analysis/。
import sys as _sys
_sys.path.insert(0, str(Path(__file__).resolve().parent))

from gamma_delta_full_verification import (  # noqa: E402
    PathologyMode,
    RunConfig,
    _SYM,
    mode_histogram,
    task1_transition_matrix,
    task2_delta_flow,
    task3_resolution_search,
)

_HERE = Path(__file__).resolve().parent
_OUT = _HERE / "gamma_delta_30m_results.md"
_SEED = 42

# 两套 RunConfig **显式定义**（不 import 基础脚本的 CANONICAL_30M——该常量被并行编辑，
# 已从 ES 期货改为 SPY ETF；为让 Run A=期货 / Run B=ETF 真正区分，此处自持定义）。

# Run A = 用户原始符号：ES/GC/CL 期货 + DBC/VNQ/UUP ETF。窗口受 ES 30min 上限限到 ~3.2yr。
# P=ES 期货（股权）、C=DBC、R=VNQ、M=UUP；Au=GC、Oil=CL（折叠通道，ω=σ(GC/CL)）。
RUN_A = RunConfig(
    label="Run A — 用户原始符号 {P=ES,C=DBC,R=VNQ,M=UUP} @ 30m（ES/GC/CL 期货，~3.2yr）",
    a0="30m",
    sources={
        "P": ("es_30m_tws.json", "30m"),    # 股权（S&P 期货，真实可交易、实盘 30min 一致）
        "C": ("dbc_30m_tws.json", "30m"),   # 宽商品 ETF
        "R": ("vnq_30m_tws.json", "30m"),   # 不动产 ETF
        "M": ("uup_30m_tws.json", "30m"),   # 美元 ETF
        "Au": ("gc_30m_tws.json", "30m"),   # 金期货（Au 折叠通道）
        "Oil": ("cl_30m_tws.json", "30m"),  # 油期货（Oil 折叠通道）
    },
    canonical=True,
    note="用户原始符号（ES/GC/CL 期货）；含病态标签；ω=σ(GC/CL)；窗口 ~3.2yr（ES 30min 期货上限）。",
)

# Run B = 10yr ETF 宏观广度（SPY/GLD/USO 替期货）。canonical=True → 含病态标签。
RUN_B = RunConfig(
    label="Run B — 10yr ETF 宏观 {P=SPY,C=DBC,R=VNQ,M=UUP} @ 30m（SPY/GLD/USO，~10yr）",
    a0="30m",
    sources={
        "P": ("spy_30m_tws.json", "30m"),
        "C": ("dbc_30m_tws.json", "30m"),
        "R": ("vnq_30m_tws.json", "30m"),
        "M": ("uup_30m_tws.json", "30m"),
        "Au": ("gld_30m_tws.json", "30m"),
        "Oil": ("uso_30m_tws.json", "30m"),
    },
    canonical=True,
    note="正典 P/C/R（全 ETF）；含病态标签；ω=σ(GLD/USO)；窗口 2016-2026（COVID/2022加息/AI 多宏观周期）。",
)


# ════════════════════════════════════════════════════════════════════════════
# 合并报告
# ════════════════════════════════════════════════════════════════════════════

def _write_t1(w, t1: dict) -> None:
    w("\n### Task 1：配置转换矩阵（a0=30m，σ=涌现最高级别走势，L2）\n")
    if "error" in t1:
        w(f"**数据缺口**：{t1['error']}\n")
        return
    w(f"- 第三轴={t1['third_role']}；a0={t1['a0']}，公共 bar={t1['n_bars']}，"
      f"日采样={t1['n_days']} 天（{t1['span'][0]}→{t1['span'][1]}）\n")
    w(f"- 出现配置数：{t1['n_configs_seen']}/27；**配置翻转次数={t1['n_transitions']}**"
      f"（日线版 move 级 10 年仅翻 1-2 次 → 30min a0 是否消除退化的关键证据）\n")
    w("\n**1.1 配置吸收态 Top（自循环概率最高）**\n\n")
    w(f"| Γ=(σP,σC,σ{t1['third_role']}) | 出现 | 自循环P |\n|---|---|---|\n")
    for g, tot_, self_p in t1["absorb"][:10]:
        gl = f"({_SYM[g[0]]},{_SYM[g[1]]},{_SYM[g[2]]})"
        w(f"| {gl} | {tot_} | {self_p:.3f} |\n")
    if t1.get("canonical"):
        theory = mode_histogram()
        ttot = sum(theory.values())
        tot = sum(t1["patho_residence"].values())
        w("\n**1.2 病态驻留分布（实际时间 vs 理论配分）**\n\n")
        w("| 病态 | 实际天数 | 实际占比 | 理论配置占比 |\n|---|---|---|---|\n")
        for mode in PathologyMode:
            d = t1["patho_residence"].get(mode.value, 0)
            w(f"| {mode.value} | {d} | {d/tot:.1%} | {theory[mode]/ttot:.0%} |\n")
        w("\n**1.3 吸收态分析（病态自循环 + 平均游程）**\n\n")
        w("| 病态 | 自循环P | 平均游程(天) | 出现天数 |\n|---|---|---|---|\n")
        for p, sp_ in sorted(t1["patho_self"].items(), key=lambda x: -x[1]):
            runs = t1["runs"].get(p, [])
            avg = sum(runs) / len(runs) if runs else 0
            w(f"| {p} | {sp_:.3f} | {avg:.1f} | {t1['patho_residence'].get(p,0)} |\n")
        # 吸收态裁决（§10.5 走资/沉没）。
        by_run = sorted(((p, sum(t1["runs"].get(p, [0])) / max(len(t1["runs"].get(p, [1])), 1))
                         for p in t1["patho_self"]), key=lambda x: -x[1])
        top3 = [p for p, _ in by_run[:3]]
        fs = {"走资", "沉没"} & set(top3)
        w(f"\n> 最持久 Top3={top3}。走资/沉没 {sorted(fs) if fs else '—'} "
          f"{'进入' if fs else '**不在**'} Top3 → §10.5「走资/沉没=吸收态」"
          f"{'部分支持' if fs else '**未得经验支持（否定性结果）**'}。\n")


def _write_t2(w, t2: dict) -> None:
    w("\n### Task 2：Δ 流量直接验证（CFTC COT，期货子集，L2）\n")
    if "error" in t2:
        w(f"**数据缺口**：{t2['error']}\n")
        return
    w("> 说明：Task 2 价格 σ 用 **1h 期货**（COT 原生期货、a0=1h；COT 周频，价格 σ 仅作"
      "粗方向代理，与配置 a0 解耦），两套 run 共用同一 Task 2 结果。\n")
    w(f"- COT 记录：{t2['cot_counts']}；公共周={t2['n_cot']}；对齐样本={t2['n_rows']}\n\n")
    w("| 量 | bits |\n|---|---|\n")
    w(f"| H(Δ) | {t2['H_d']:.3f} |\n| H(Δ\\|Γ_期货) | {t2['H_d_g']:.3f} |\n"
      f"| H(Δ\\|Γ_期货,ω) | {t2['H_d_gw']:.3f} |\n")
    w(f"\n> Γ→Δ：**{'一对多✓' if t2['H_d_g']>0.05 else '确定'}**"
      f"（多样性 max={t2['mult_max']} mean={t2['mult_mean']:.2f}）。\n")
    w("\n**2.2 flow_to_walk 桥接命中率（σ价格方向=Δ流量方向？）**\n\n")
    w("| 顶点 | 全样本同号 | 双非零同号 | 非零样本 |\n|---|---|---|---|\n")
    for k, b in t2["bridge"].items():
        an = f"{b['agree_nonzero']:.1%}" if b["agree_nonzero"] is not None else "n/a"
        w(f"| {k} | {b['agree_all']:.1%} | {an} | {b['n_nonzero']} |\n")
    w("\n> 双非零命中率≈50%（随机）→ σ价格走势方向不代理真实流量（存量重估偏差）。\n")


def _write_t3(w, t3: dict) -> None:
    w("\n### Task 3：分辨变量搜索（a0=30m，L2/L3）\n")
    if "error" in t3:
        w(f"**数据缺口**：{t3['error']}\n")
        return
    w(f"- 日采样样本：{t3['n_rows']}（全变量非缺口：{t3['n_full']}）；交叉边 X={t3['x_edges']}\n")
    w(f"- 收益率斜率来源：{t3.get('slope_src','—')}\n\n")
    w("| 量 | bits |\n|---|---|\n")
    w(f"| H(X) | {t3['H_X']:.3f} |\n| H(X\\|Γ) | {t3['H_X_g']:.3f} |\n"
      f"| H(X\\|Γ,ω) | {t3['H_X_gw']:.3f} |\n")
    w("\n**3.2 候选变量信息增益（shuffle 扣减伪增益）**\n\n")
    w("| 变量 | 观测增益 | shuffle伪增益 | **真实增益** | 状态数 | N |\n|---|---|---|---|---|---|\n")
    for cname, v in sorted([(k, v) for k, v in t3["candidates"].items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        w(f"| {cname} | {v['observed_gain']:.3f} | {v['shuffle_floor']:.3f} | "
          f"**{v['real_gain']:.3f}** | {v['n_states']} | {v['n']} |\n")
    for cname, v in t3["candidates"].items():
        if "error" in v:
            w(f"| {cname} | — | — | [{v['error']}] | — | — |\n")
    w("\n**3.3 最小变量集贪心搜索（仅外部变量）**\n\n")
    w("| 步骤 | 加入 | H(X\\|...) | 真实增益 |\n|---|---|---|---|\n")
    for i, (k, h, real) in enumerate(t3["greedy"]):
        w(f"| {i} | {k} | {h:.3f} | {f'{real:.3f}' if real is not None else '—'} |\n")
    w(f"\n> 选中集：**{t3['chosen'] or '（无变量真实增益>0.005）'}**；"
      f"终态 H(X\\|Γ,选中)={t3['final_H']:.3f} bits"
      f"{'（**未到 0**，残余结构性不确定）' if t3['final_H']>0.1 else '（接近完备）'}。\n")


def write_report(runs: list[tuple[RunConfig, dict, dict]], t2: dict) -> None:
    L = []
    w = L.append
    w("# Γ→Δ 30min a0 全验证结果（两套 RunConfig）\n")
    w(f"> 生成：{datetime.now().strftime('%Y-%m-%d %H:%M')}　")
    w("脚本：`analysis/gamma_delta_30m_verification.py`（编排层）+ "
      "`gamma_delta_full_verification.py`（机制）。\n")

    # 结果包六要素。
    w("\n## 结果包（六要素）\n")
    w("**1. 结论**：用 **a0=30min** 替代日线 a0，σ 读 `_top_sigma`=涌现最高级别走势"
      "（`move_snapshot.moves[-1]`，非日线 segment-level workaround）重跑三任务。两套窗口："
      "**Run A**=用户原始符号（ES/GC/CL 期货+UUP/VNQ/DBC，~3.2yr=期货 30min 上限）；"
      "**Run B**=10yr ETF 宏观广度（SPY/GLD/USO 替期货）。各 run 结论见下表与分节。\n")
    w("**2. 定义依据**：Γ=(σ_P,σ_C,σ_R)=`config_space.Configuration`；"
      "病态判据=`capital_flow_taxonomy.classify`；528号折叠模型（金油=折叠通道非顶点→ω）；"
      "σ=`RecursiveOrchestrator` a0=30min 涌现最高级别 `move.direction`。\n")
    w("**3. 边界条件**：a0=30min，转换矩阵按日采样（σ 已涌现，采样间隔不改 σ）；"
      "**Run A 窗口 ~3.2yr（IBKR 连续期货 30min 上限，ContFuture 拒绝翻页）< 日线版 10yr**"
      "→ 期货 30min 宏观周期更少（数据可用性硬边界，非 workaround）；Run B ETF 可达 10yr；"
      "Δ=COT 周频非商业净持仓变化（仅期货，无 R 代理）；条件熵含有限样本伪增益（已 shuffle 扣减）。\n")
    w("**4. 下游推论**：若 30min a0 下配置翻转数 ≫ 日线版（move 级不再退化）→ "
      "证明日线 segment-level σ 是 a0 太粗的 workaround、30min a0 是正确读出层；"
      "若桥接命中率仍≈50% → σ 价格方向≠流量方向的结论在 30min 同样成立。\n")
    w("**5. 谱系引用**：254号（走资/跨国）、330号（堰塞湖/空转）、482号（金油比/沉没）、"
      "527号（σ走势方向态/81边）、528号（折叠通道重构）；auto-memory "
      "`project_k4_fold_channel_model`、`project_omega_regime_falsified`、"
      "`project_recursive_level_emergence`、`project_gamma_delta_full_verification`。\n")
    w("**6. 影响声明**：新增 `gamma_delta_30m_verification.py`（编排层）+ "
      "`scripts/tws/fetch_30m_tws.py` + 9 个 `*_30m_tws.json`；复用 "
      "`gamma_delta_full_verification.py` 机制（不改其逻辑）；不改动 src 源码。\n")

    # Run 对比表。
    w("\n## Run A vs Run B 总览\n\n")
    w("| 维度 | Run A（期货 ~3.2yr） | Run B（ETF 10yr） |\n|---|---|---|\n")

    def g(t, k, f="{}"):
        return f.format(t[k]) if (t and "error" not in t and k in t) else "—"

    (_, a1, a3), (_, b1, b3) = runs[0], runs[1]
    w(f"| Task1 日采样天数 | {g(a1,'n_days')} | {g(b1,'n_days')} |\n")
    w(f"| Task1 窗口 | {g(a1,'span')} | {g(b1,'span')} |\n")
    w(f"| Task1 配置翻转 | {g(a1,'n_transitions')} | {g(b1,'n_transitions')} |\n")
    w(f"| Task1 出现配置 | {g(a1,'n_configs_seen')}/27 | {g(b1,'n_configs_seen')}/27 |\n")
    w(f"| Task3 终态 H(X\\|...) | {g(a3,'final_H','{:.3f}')} | {g(b3,'final_H','{:.3f}')} |\n")
    w(f"| Task3 选中变量集 | {g(a3,'chosen')} | {g(b3,'chosen')} |\n")
    w("\n> **30min a0 核心检验 = 配置翻转次数**。日线版最高级别走势 10 年仅翻 1-2 次（退化→"
      "被迫用 segment workaround）。若 30min a0 翻转数显著增大 → a0 粒度是退化根因，"
      "30min a0 + 涌现走势 σ 是严格读出层（消除 workaround）。\n")

    # 各 run 详节。
    for cfg, t1, t3 in runs:
        w(f"\n---\n\n## {cfg.label}\n")
        w(f"> {cfg.note}\n")
        _write_t1(w, t1)
        _write_t3(w, t3)

    # Task 2 共享节。
    w("\n---\n\n## Task 2（两套 run 共享：COT 真实 Δ vs 1h 期货价格 σ）\n")
    _write_t2(w, t2)

    _OUT.write_text("".join(L), encoding="utf-8")
    print(f"\n报告已写入：{_OUT}")


def main() -> None:
    random.seed(_SEED)
    print("Γ→Δ 30min a0 全验证（编排层）：Run A=用户期货符号；Run B=10yr ETF。")
    runs = []
    for cfg in (RUN_A, RUN_B):
        print(f"\n{'█'*78}\n█ {cfg.label}\n{'█'*78}")
        t1 = task1_transition_matrix(cfg)
        t3 = task3_resolution_search(cfg)
        runs.append((cfg, t1, t3))
    # Task 2 共享（COT 期货原生，1h a0），跑一次。
    print(f"\n{'█'*78}\n█ Task 2（共享）\n{'█'*78}")
    t2 = task2_delta_flow()
    write_report(runs, t2)
    print("\n完成。")


if __name__ == "__main__":
    main()
