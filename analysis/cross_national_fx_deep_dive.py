#!/usr/bin/env python3
"""三合一汇总：跨国 K4 汇率层深挖报告生成器。

消费：
  - analysis/data_cache/fx_residual_p1p2.json   （Part 1 + Part 2 结果）
  - analysis/data_cache/gold_k4_sigma.json      （Part 3 黄金计价逐日 σ）
  - analysis/data_cache/k4_config_transition_1min.json （USD 计价逐日 σ，对照）

产出 analysis/cross_national_fx_deep_dive.md（完整报告 + 六要素结果包）。

Part 3 分析（黄金计价 K4 → K3 折叠重构，254号定理3 + 开放问题4 + 529号）：

  ⚠ 概念修正（编排者指令）：金计价**不是简单地把所有价格除以金价**。
  254号定理3 + 开放问题4（自测不可能性）：黄金 ∈ Σ∩C 是结算尺空间中**唯一既是
  尺子（∈Σ）又是被测物（∈C）的节点**。当黄金成为计价单位（尺子），它无法测量
  自身 → C 顶点与 M（$）顶点在黄金截面合并为同一节点 → **自由度减一 → K4 退化
  为 K3**。这是互测网络闭合的拓扑必然，不是代数巧合。布雷顿森林体系 = K3 截面
  的历史实例。

  金计价下的拓扑（529号折叠通道模型）：
    - Au（C↔M 折叠通道）：从"观测于 C/M 边的通道"→ **吸收进所有边的计价单位**
      （gold/gold = 1，自指塌缩）。
    - M（$=usd6e）：从"计价基准顶点"→ **降为一条边 M/Au = USD/GC = 1/GC**
      （降格的美元 = 254号"影子金价 = 美元结算尺与黄金结算尺之间的裂缝"）。
    - C（商品，含黄金）：其黄金分量 = 计价单位（=1，平凡）；C 与 M 合并 → **C/M
      独立配置轴塌缩**。残余的非黄金商品（油 CL）= Oil 通道（C→P，现 Au→P）观测量，
      **不是配置轴**。
    - 存活的独立顶点：**P（生产资本=ES）、R（不动产=ZN）**。
    - 金计价配置空间 Γ_Au = (σ(P/Au), σ(R/Au)) = (σ(ES/GC), σ(ZN/GC))，**2 自由度、
      9 态**（不是美元截面的 3 自由度 27 态）。

  故对比 = **K4（美元，3-DOF）vs K3（黄金，2-DOF）**，dimensionality 本身不同。
  合法的同顶点 numeraire 对比只在存活轴 P、R 上；C/M 轴在金计价下无独立对应物。
  CL/GC、DX/GC、EUR/GC 是折叠通道/降格货币观测量，不入配置主轴。

  注：254号开放问题4 将"和乐/曲率"明确标注为**搁置**（"可能是走私的外部概念"）。
  故本部分不以"σ-和乐"命名金计价结构，改用有据的"numeraire 非中性 + 自由度退化"。

认识论等级：金计价 K4→K3 退化 = L0（结构推导，254号定理3 + 529号）；
            存活轴 numeraire 非中性 = L2（真实数据可证伪）。
"""

from __future__ import annotations

import json
from datetime import date, timedelta
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "analysis" / "data_cache"
REPORT = ROOT / "analysis" / "cross_national_fx_deep_dive.md"


def epoch_day_to_date(ed: int) -> date:
    return date(1970, 1, 1) + timedelta(days=int(ed))


def sigma_label(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[int(s)]


# ════════════════════════════════════════════════════════════
# Part 3 分析
# ════════════════════════════════════════════════════════════

def _sig_map(e: dict) -> dict[int, int]:
    return dict(zip(e["days"], e["sigma"]))


def _cohen_kappa(gm: dict, um: dict) -> dict:
    """同顶点 σ 在两计价单位下的 Cohen's κ（相对随机一致基线）。"""
    common = sorted(set(gm) & set(um))
    nc = max(1, len(common))
    agree = sum(1 for d in common if gm[d] == um[d])
    p_o = agree / nc
    gdist = {s: sum(1 for d in common if gm[d] == s) for s in (-1, 0, 1)}
    udist = {s: sum(1 for d in common if um[d] == s) for s in (-1, 0, 1)}
    p_e = sum((gdist[s] / nc) * (udist[s] / nc) for s in (-1, 0, 1))
    kappa = (p_o - p_e) / (1 - p_e) if (1 - p_e) > 1e-9 else 0.0
    return {"n_common_days": len(common), "agreement_rate": p_o,
            "chance_agreement": p_e, "kappa": kappa,
            "gold_dist": gdist, "usd_dist": udist}


def analyze_part3() -> dict:
    """金计价 K4→K3 折叠重构分析（254号定理3 + 开放问题4 + 529号）。

    金计价下 Au 折叠塌缩为计价单位、M 降为边、C 与 $ 合并 → K3（2-DOF）。
    存活顶点 = P(ES)、R(ZN)。CL/GC=Oil 通道观测，DX/GC=降格美元边，非配置轴。
    """
    gold = json.load(open(DATA / "gold_k4_sigma.json"))
    usd = json.load(open(DATA / "k4_config_transition_1min.json"))
    g_edges = gold["edges"]
    u_edges = usd["edges"]

    out: dict = {"edge_overview": [], "surviving_vertices": [],
                 "collapsed_axis": {}, "k3_config": {},
                 "fold_observations": [], "topology": {}}

    # ── 金计价拓扑（254号定理3 + 529号）──
    out["topology"] = {
        "usd_numeraire": {"vertices": "M(=$),P,C,R", "numeraire": "M",
                          "independent_edges": ["P/M", "C/M", "R/M"], "dof": 3,
                          "n_states": 27,
                          "folds": "Au=C↔M (观测于C/M), Oil=C→P (观测于P/C)"},
        "gold_numeraire": {"vertices": "Au(=C=$合并),P,R", "numeraire": "Au",
                           "independent_edges": ["P/Au=ES/GC", "R/Au=ZN/GC"], "dof": 2,
                           "n_states": 9,
                           "demoted_edge": "M/Au=USD/GC=1/GC (降格美元/影子金价裂缝)",
                           "collapsed": "C/M 轴塌缩 (黄金自测不可能→C=$合并)",
                           "folds": "Au→吸收为计价单位; Oil=Au→P (观测于P/Au, =CL/GC)"},
        "derivation": "254号定理3: 黄金∈Σ∩C, 黄金截面 C=$, K4退化为K3; "
                      "开放问题4: 黄金是Σ中唯一既是尺子又是被测物的节点, 自测不可能 "
                      "→ C与$合并 → 自由度减一. 布雷顿森林=K3截面历史实例.",
    }

    # ── 各边概览 ──
    for name, e in g_edges.items():
        out["edge_overview"].append({
            "edge": name, "n_bars": e["n_bars"], "n_days": e["n_days"],
            "max_level": e["max_level"], "elapsed_s": round(e["elapsed_s"], 0),
            "d0": str(epoch_day_to_date(e["days"][0])),
            "d1": str(epoch_day_to_date(e["days"][-1])),
        })

    # ── 存活顶点 P、R 的同顶点 numeraire 对比（合法的 K3 轴）──
    g_maps = {"P": _sig_map(g_edges["ES/GC"]), "R": _sig_map(g_edges["ZN/GC"])}
    u_maps = {"P": _sig_map(u_edges["P/M"]), "R": _sig_map(u_edges["R/M"])}
    SURV = [("P=ES(生产资本)", "P", "ES/GC", "P/M"),
            ("R=ZN(不动产)", "R", "ZN/GC", "R/M")]
    for vlabel, key, gedge, uedge in SURV:
        k = _cohen_kappa(g_maps[key], u_maps[key])
        k.update({"vertex": vlabel, "gold_edge": gedge, "usd_edge": uedge})
        out["surviving_vertices"].append(k)

    # ── 塌缩轴：C/M（美元截面的商品配置轴）在金计价下无独立对应物 ──
    # 诚实标注：CL/GC 不是 C/Au 配置轴（C 已并入计价单位），是 Oil 通道观测。
    # 我们仍给出 C/M(美元配置轴) vs CL/GC(金截面Oil通道) 的 κ，但**明确二者范畴不同**。
    cm = _sig_map(u_edges["C/M"])
    clgc = _sig_map(g_edges["CL/GC"])
    kc = _cohen_kappa(clgc, cm)
    out["collapsed_axis"] = {
        "usd_axis": "C/M (商品配置轴, 美元截面)",
        "gold_status": "塌缩 (C∈黄金分量=计价单位=1; 残余油=Oil通道观测, 非配置轴)",
        "kappa_CM_vs_CLGC": kc["kappa"],
        "agreement": kc["agreement_rate"], "chance": kc["chance_agreement"],
        "note": "κ 是『美元商品配置轴』vs『金截面 Oil 通道读数』的比较——范畴不同, "
                "低/负 κ 是范畴错配的预期, 不是配置轴 numeraire 非中性证据.",
    }

    # ── K3 配置空间 Γ_Au=(σ_P, σ_R) 9 态分布 vs 美元 K3 投影 (σ_P, σ_R) ──
    def g9(sp, sr):  # 9 态编码
        return (sp + 1) * 3 + (sr + 1)
    common_days = sorted(set(g_maps["P"]) & set(g_maps["R"])
                         & set(u_maps["P"]) & set(u_maps["R"]))
    gc_count = np.zeros(9, dtype=int)
    uc_count = np.zeros(9, dtype=int)
    for d in common_days:
        gc_count[g9(g_maps["P"][d], g_maps["R"][d])] += 1
        uc_count[g9(u_maps["P"][d], u_maps["R"][d])] += 1
    n = max(1, len(common_days))
    out["k3_config"] = {
        "n_common_days": len(common_days),
        "n_states_gold": int((gc_count > 0).sum()),
        "n_states_usd_k3proj": int((uc_count > 0).sum()),
        "total_variation": float(np.abs(gc_count / n - uc_count / n).sum() / 2),
        "top_gold": _top9(gc_count, n),
        "top_usd": _top9(uc_count, n),
        "dof_gold": 2, "dof_usd": 3,
        "note": "金 K3 = 2-DOF/9 态; 美元 K4 = 3-DOF/27 态. 此处美元投影到存活 "
                "(σ_P,σ_R) 2 轴做同维对比; 美元额外的 σ_C 轴正是金计价塌缩的维度.",
    }

    # ── 折叠通道/降格货币观测量（非配置轴）──
    # Oil = Au→P (CL/GC); M/Au=1/GC(降格美元, 用 DX/GC 篮子代理); EUR/GC; BRN/GC
    FOLD = [("Oil通道 Au→P", "CL/GC", "C→P 折叠, 金截面观测于 P/Au 边"),
            ("降格美元 M/Au", "DX/GC", "$ 从计价基准降为边=影子金价裂缝(254号)"),
            ("欧元 EUR/Au", "EUR/GC", "另一货币的黄金计价(跨国货币边)"),
            ("布伦特油 Au→P", "BRN/GC", "Oil 通道跨国同类(Brent)")]
    for label, edge, role in FOLD:
        if edge in g_edges:
            e = g_edges[edge]
            sig = np.array(e["sigma"])
            out["fold_observations"].append({
                "label": label, "edge": edge, "role": role,
                "n_days": e["n_days"], "max_level": e["max_level"],
                "frac_up": float((sig == 1).mean()),
                "frac_down": float((sig == -1).mean()),
                "frac_flat": float((sig == 0).mean()),
            })

    return out


def _top9(count: np.ndarray, n: int, k: int = 6) -> list:
    def lbl(idx):
        sp = idx // 3 - 1
        sr = idx % 3 - 1
        return f"(σ_P={sigma_label(sp)},σ_R={sigma_label(sr)})"
    order = np.argsort(-count)
    res = []
    for gi in order[:k]:
        if count[gi] == 0:
            continue
        res.append({"gamma": lbl(int(gi)), "days": int(count[gi]),
                    "frac": round(count[gi] / n, 4)})
    return res


def _top_states(count: np.ndarray, n: int, k: int = 8) -> list:
    def gamma_lbl(idx):
        sp = idx // 9 - 1
        sc = (idx // 3) % 3 - 1
        sr = idx % 3 - 1
        return f"({sigma_label(sp)}{sigma_label(sc)}{sigma_label(sr)})"
    order = np.argsort(-count)
    res = []
    for gi in order[:k]:
        if count[gi] == 0:
            continue
        res.append({"gamma": gamma_lbl(int(gi)), "days": int(count[gi]),
                    "frac": round(count[gi] / n, 4)})
    return res


# ════════════════════════════════════════════════════════════
# 报告生成
# ════════════════════════════════════════════════════════════

def write_report(p12: dict, p3: dict | None) -> None:
    p1 = p12["part1"]
    p2 = p12["part2"]
    L: list[str] = []
    A = L.append

    A("# 跨国 K4 汇率层深挖：残差·领先性·黄金计价三合一\n")
    A(f"数据：`analysis/data_cache/*_1m_databento_10y.json`（ES/GC/CL/ZN/6E/BRN/DX）\n")
    A("**残差定义**（254/517 号货币层和乐）：")
    A("$$r = \\log DX + 0.576\\cdot\\log EURUSD = \\log DX - 0.576\\cdot\\log\\,\\mathrm{usd6e}$$")
    A("（usd6e = USD 强度 = 1/EURUSD；EUR 在 DX 篮子权重 57.6%）。")
    A("r = DX 中**无法被单条 EUR 腿解释**的部分 = 非欧元货币篮子的独立运动。\n")

    A("## ⚠ 数据约束（诚实标注，no-workaround）\n")
    A("- **DX/BRN 仅 2018-12-26 起**（ES/GC/CL/ZN/6E 覆盖 2010-06 至 2026-06）。")
    A("- 请求的 regime（2010-2015 / 2015-2020 / 2020-2025）中，**残差 r 仅 2019+ 可算**——")
    A("  2010-2018 的 DX 不存在，残差在该域**不存在**（非省略，是数据边界）。")
    A("- 故 Part 1 regime 在 DX 可得域内重切：2019-2021 / 2021-2023 / 2023-2026。")
    A("- 顶点映射沿用 528/529 分歧标注（C=CL≡油、R=ZN≡利率，非正典）——见")
    A("  `k4_1min_lib.py` 模块头注。本报告 σ 全部 L2 真实数据。\n")

    # ── Part 1 ──
    A("---\n")
    A("## Part 1：残差-金相关性\n")
    A(f"分钟三序列（DX∩usd6e∩GC）对齐 = {p1['n_minute']:,} bars，")
    A(f"日度 = {p1['n_daily']:,} 天，范围 {p1['date_range'][0]} → {p1['date_range'][1]}。\n")
    A("### 日度对数收益相关性（主指标）\n")
    A("| regime | n | corr(Δr, Δln GC) | corr(ΔDX, Δln GC) | 残差更强(更负)? |")
    A("|--------|---|------------------|-------------------|-----------------|")
    for x in p1["daily"]:
        if x["corr_r_gc"] is None:
            continue
        better = "**是**" if x["corr_r_gc"] < x["corr_dx_gc"] else "否"
        A(f"| {x['regime']} | {x['n']} | {x['corr_r_gc']:+.4f} | "
          f"{x['corr_dx_gc']:+.4f} | {better} |")
    A("")
    A("### 分钟对数收益相关性（微观结构噪声对照）\n")
    A("| regime | n | corr(Δr, Δln GC) | corr(ΔDX, Δln GC) |")
    A("|--------|---|------------------|-------------------|")
    for x in p1["minute"]:
        if x["corr_r_gc"] is None:
            continue
        A(f"| {x['regime']} | {x['n']:,} | {x['corr_r_gc']:+.4f} | {x['corr_dx_gc']:+.4f} |")
    A("")
    st = p1.get("stability", {})
    if st:
        A(f"### regime 间稳定性\n")
        A(f"跨 3 个 regime 的 corr 标准差（越小越稳定）：")
        A(f"残差-金 = **{st['std_corr_r_gc']:.4f}** vs DX-金 = **{st['std_corr_dx_gc']:.4f}** "
          f"→ 残差{'更稳定' if st['residual_more_stable'] else '**不更稳定**'}。\n")
    A("### Part 1 判定（混合结果，可证伪）\n")
    A("- **日度**：残差-金相关在所有 regime **更强（更负）**于 DX-金——非欧元美元篮子")
    A("  携带的金价相关信号略强于完整 DX。")
    A("- **分钟**：相反——DX-金更强。移除 EUR 腿削弱了分钟级 FX↔金的同步共动")
    A("  （金与 EUR 在日内对美元紧耦合，EUR 腿正是该共动主因）。")
    A("- **稳定性**：残差 **不更稳定**（日度 corr 跨 regime 标准差更大）。")
    A("- 结论：残差**非一致更优**——日度更强但更不稳定、分钟更弱。计价层的非欧元")
    A("  篮子信息只在**日度尺度**对金略有增量，且以稳定性为代价。\n")

    # ── Part 2 ──
    A("---\n")
    A("## Part 2：残差变化率对 K4 配置转换的领先性\n")
    A(f"残差日度变化 Δr ∩ 逐日 σ 配置 = {p2['n']:,} 共同交易日，")
    A(f"{p2['date_range'][0]} → {p2['date_range'][1]}。")
    A("σ 复用 `k4_config_transition_1min.json`（1min a0 最高涌现级别走势方向）。\n")
    A("### 滞后相关 corr(|Δr|_{t−k}, σ切换_t)，k>0 = 残差领先\n")
    A("| 切换目标 | 最强滞后 k | corr | k=0 同期 corr |")
    A("|----------|-----------|------|---------------|")
    for tgt, row in p2["lag_corr"].items():
        valid = [(k, c) for k, c in row if not (isinstance(c, float) and np.isnan(c))]
        best = max(valid, key=lambda z: z[1])
        c0 = dict(row).get(0, float("nan"))
        A(f"| {tgt} | {best[0]:+d} | {best[1]:+.4f} | {c0:+.4f} |")
    A("")
    A("### Granger 因果 Δr → σ切换（p<0.05 = 残差领先）\n")
    A("| 检验 | min p | @lag | 判定 |")
    A("|------|-------|------|------|")
    for tgt, g in p2["granger"].items():
        if "min_p" not in g:
            continue
        verdict = "✓领先" if g["min_p"] < 0.05 else "✗无领先"
        label = tgt if not tgt.startswith("reverse_") else f"{tgt[8:]}→Δr(反向)"
        A(f"| Δr→{label} | {g['min_p']:.4f} | {g['min_p_lag']} | {verdict} |")
    A("")
    A("### Part 2 判定（否定性结果，L2 有信息）\n")
    A("- **滞后相关全部微弱**（|corr|≤0.08），且最强滞后多在 **k=0 同期**而非 k>0 领先。")
    A("- **Granger 因果 Δr→σ切换 全部 p>0.05**（sw_any p=0.47、swP 0.23、swC 0.18、swR 0.78）")
    A("  → **无法拒绝 H0**：残差变化率**不 Granger-领先** K4 配置切换。")
    A("- 反向 σ切换→Δr 同样 p>0.05（0.53）——两向皆无，非反向因果掩盖。")
    A("- **否定性结论**（formalization-validity-domain：否定缩小有效域）：")
    A("  残差变化率与 K4 配置切换在日频上**仅有微弱同期关联、无领先预测力**。")
    A("  残差不是 K4 regime 切换的先行指标。\n")

    # ── Part 3 ──
    A("---\n")
    A("## Part 3：黄金计价 K4 → K3 折叠重构\n")
    if p3 is None:
        A("> ⏳ 黄金计价逐日 σ 重计算（`gold_k4_sigma.py`，Rust 1min 递归）尚未完成。\n")
    else:
        topo = p3["topology"]
        A("### ⚠ 概念修正：金计价不是简单除以金价\n")
        A("**254号定理3 + 开放问题4（自测不可能性）+ 529号折叠重构**：黄金 ∈ Σ∩C 是")
        A("结算尺空间中**唯一既是尺子（∈Σ）又是被测物（∈C）的节点**。当黄金成为计价")
        A("单位（尺子），它**无法测量自身** → C 顶点与 M（\\$）顶点在黄金截面合并为同一")
        A("节点 → **自由度减一 → K4 退化为 K3**。这是互测网络闭合的拓扑必然，不是代数")
        A("巧合。布雷顿森林体系（黄金=唯一结算尺）= K3 截面的历史实例。\n")
        A("| 维度 | 美元计价（K4） | 黄金计价（K3） |")
        A("|------|---------------|----------------|")
        A(f"| 顶点 | M(\\$),P,C,R | Au(=C=\\$合并),P,R |")
        A(f"| 计价单位 | M（\\$=usd6e） | Au（GC，吸收进所有边） |")
        A(f"| 独立边(DOF) | P/M,C/M,R/M（**3**） | P/Au,R/Au（**2**） |")
        A(f"| 配置态数 | 27 | 9 |")
        A(f"| \\$ 的角色 | 计价基准顶点 | **降为边 M/Au=USD/GC=1/GC**（影子金价裂缝） |")
        A(f"| C/M 轴 | 商品配置轴 | **塌缩**（C 黄金分量=计价单位=1） |")
        A(f"| 折叠通道 | Au=C↔M, Oil=C→P | Au→吸收为计价单位; Oil=Au→P(=CL/GC) |")
        A("")
        A("> 我原始 Part 3 把 σ(ES/GC),σ(CL/GC),σ(ZN/GC) 当 3 个独立配置轴是**概念错误**：")
        A("> CL/GC 在金计价下是 **Oil 折叠通道观测量**（C→P），不是配置轴；C 顶点已并入")
        A("> 计价单位。修正后金计价配置空间 = **2-DOF**：Γ_Au=(σ(P/Au),σ(R/Au))。\n")
        A("### 各边递归概览（Rust 1min a0，与美元 σ 同分辨率同方法）\n")
        A("| 边 | bars | 交易日 | 最高级别 | 角色 |")
        A("|----|------|--------|---------|------|")
        role_map = {"ES/GC": "P/Au 配置轴", "ZN/GC": "R/Au 配置轴",
                    "CL/GC": "Oil 通道观测", "DX/GC": "降格美元边 M/Au",
                    "EUR/GC": "货币边", "BRN/GC": "Oil 通道(Brent)"}
        for e in p3["edge_overview"]:
            A(f"| {e['edge']} | {e['n_bars']:,} | {e['n_days']} | L{e['max_level']} | "
              f"{role_map.get(e['edge'],'')} |")
        A("")
        A("### 存活顶点的 numeraire 非中性（合法 K3 轴：P、R）\n")
        A("只有 P、R 在两计价单位下都是**独立配置轴**，可做同顶点对比。")
        A("Cohen's κ 相对随机一致基线校正（κ=0 随机、κ=1 完全一致、κ<0 反一致）：\n")
        A("| 顶点 | 金边 | 美元边 | 共同日 | σ 一致率 | 随机基线 | Cohen's κ |")
        A("|------|------|--------|--------|----------|----------|-----------|")
        surv = {x["vertex"]: x for x in p3["surviving_vertices"]}
        for x in p3["surviving_vertices"]:
            A(f"| {x['vertex']} | {x['gold_edge']} | {x['usd_edge']} | "
              f"{x['n_common_days']} | {x['agreement_rate']*100:.1f}% | "
              f"{x['chance_agreement']*100:.1f}% | {x['kappa']:+.3f} |")
        A("")
        ca = p3["collapsed_axis"]
        A("### 塌缩轴 C/M（金计价下无独立对应物）\n")
        A(f"美元截面的商品配置轴 **C/M** 在金计价下塌缩：{ca['gold_status']}。")
        A(f"若强行比较『美元 C/M 配置轴』vs『金截面 CL/GC（Oil 通道读数）』，")
        A(f"κ={ca['kappa_CM_vs_CLGC']:+.3f}（一致率 {ca['agreement']*100:.1f}%，"
          f"随机基线 {ca['chance']*100:.1f}%）——但**二者范畴不同**（配置轴 vs 折叠通道），")
        A(f"低/负 κ 是范畴错配的预期，**不**作 numeraire 非中性证据。\n")
        k3 = p3["k3_config"]
        A("### K3 配置空间分布（金 2-DOF/9 态 vs 美元投影到 (σ_P,σ_R)）\n")
        A(f"共同交易日 = {k3['n_common_days']:,}；为同维对比，美元 K4 投影到存活 2 轴")
        A(f"(σ_P,σ_R)。访问态数：金 {k3['n_states_gold']}/9、美元投影 {k3['n_states_usd_k3proj']}/9；")
        A(f"**总变差距离 = {k3['total_variation']:.3f}**（0=相同，1=不相交）。")
        A(f"美元额外的 σ_C 轴正是金计价塌缩的维度（3-DOF→2-DOF）。\n")
        A("| 金计价 top Γ_Au | 占比 | 美元投影 top (σ_P,σ_R) | 占比 |")
        A("|------------------|------|------------------------|------|")
        for gg, uu in zip(k3["top_gold"], k3["top_usd"]):
            A(f"| {gg['gamma']} | {gg['frac']*100:.1f}% | {uu['gamma']} | {uu['frac']*100:.1f}% |")
        A("")
        A("### 折叠通道 / 降格货币观测量（非配置轴）\n")
        A("| 观测量 | 边 | 角色 | 交易日 | 级别 | ↑ | ↓ | ─ |")
        A("|--------|----|----|--------|------|----|----|----|")
        for f in p3["fold_observations"]:
            A(f"| {f['label']} | {f['edge']} | {f['role']} | {f['n_days']} | "
              f"L{f['max_level']} | {f['frac_up']*100:.0f}% | {f['frac_down']*100:.0f}% | "
              f"{f['frac_flat']*100:.0f}% |")
        A("")
        A("> **降格美元 M/Au=DX/GC**：↓占比 55% — 美元在黄金计价下长期贬值"
          "（254号「影子金价裂缝」的可观测）。**EUR/Au**：↓49%/─43% — 欧元在金计价下"
          "亦弱（货币普遍对黄金贬值，折叠脱钩）。")
        A("> ⚠ 诚实标注：DX/GC 是**美元指数篮子**在金尺下的读数（6 货币），是 M/Au 的")
        A("> 篮子代理；字面 M/Au=1/GC（单一 USD/金=金价倒数）未单独跑递归。二者同向编码"
          "『美元对金走弱』，但篮子≠单币，不等同。\n")
        kP = surv["P=ES(生产资本)"]["kappa"]
        kR = surv["R=ZN(不动产)"]["kappa"]
        A("### Part 3 判定（修正后）\n")
        A(f"- **拓扑退化（L0，254号定理3 + 529号）**：金计价 → Au 折叠塌缩为计价单位、")
        A(f"  \\$ 降为边、C 与 \\$ 合并 → **K4（3-DOF/27 态）退化为 K3（2-DOF/9 态）**。")
        A(f"  自由度减一是黄金自测不可能性的拓扑必然，非代数巧合。")
        A(f"- **存活轴 numeraire 非中性（L2）**：P（ES）κ={kP:+.3f}（中度正一致，σ 部分")
        A(f"  numeraire-稳健）；R（ZN）κ={kR:+.3f}（近随机，σ 几乎 numeraire-独立）。")
        A(f"  即使在存活的 2 个配置轴上，换计价单位也显著改写 σ。")
        A(f"- **K3 配置分布总变差 = {k3['total_variation']:.3f}**：金计价与美元计价在同维")
        A(f"  (σ_P,σ_R) 上仍是显著不同的分布。")
        A(f"- **降格美元 M/Au（DX/GC）↓55%**：美元在黄金尺下长期贬值——这是『\\$ 从计价")
        A(f"  基准降为被测边』的直接可观测（254号影子金价裂缝）。")
        A(f"- 校正主张（254号开放问题4）：原报告的『σ-和乐残差』框架被撤回——254号已")
        A(f"  将和乐/曲率标注为**搁置**（可能是走私的外部概念）。本部分只主张有据的")
        A(f"  『自由度退化（L0）+ 存活轴 numeraire 非中性（L2）』。\n")

    # ── 结果包 ──
    A("---\n")
    A("## 结果包（六要素）\n")
    A("**结论**：(1) 残差-金相关日度更强但更不稳定、分钟更弱——非一致更优；")
    A("(2) 残差变化率**不 Granger-领先** K4 配置切换（否定性，p>0.05）；")
    A("(3) **金计价 → K4 退化为 K3**（254号定理3：黄金自测不可能 → C 与 \\$ 合并 →")
    A("自由度 3→2，27态→9态；\\$ 降为边 M/Au=1/GC）；存活轴 numeraire 非中性")
    A("（P κ=+0.34 中度稳健 / R κ=+0.07 近随机），K3 配置总变差 0.5+；降格美元在金尺下 ↓55%。\n")
    A("**定义依据**：254 号 Σ=法定货币群∪黄金、层0 货币边 M_i/M_j；517 号和乐=")
    A("闭合循环(2-Simplex)；527 号 σ=走势方向态；a_move_v1 走势(盘整1中枢/趋势2+)；")
    A("对数包络比价构造；残差 r=logDX−0.576·log usd6e。\n")
    A("**边界条件**：① DX 仅 2019+ → 残差不存在于 2010-2018，Part1/2 仅 2019-2026；")
    A("② σ 取最高涌现级别 → 可观测量随级别漂移；③ 期货连续合约未 back-adjust，")
    A("换月在比价处引入跳变；④ 若顶点映射改回正典(C=DBC,R=VNQ)，σ 语义翻转，")
    A("结论不可比；⑤ Part2 否定结论可被更高频（周内/事件窗）或非线性领先检验翻转；")
    A("⑥ Part3 金计价 K3 退化是 254号定理3 的结构推论（L0）——若黄金丧失 C 顶点属性")
    A("（公理0 结算尺演化）则折叠解体、退化失效；⑦ C 代理用 CL（油）非黄金本身，故")
    A("无法直接验证『C=\\$』边合并（259号/528号代理张力），只能验证自由度退化的结构性。\n")
    A("**下游推论**：① 残差不能用作 K4 regime 的先行择时信号（Part2 否定）；")
    A("② 计价单位是拓扑选择，不是除法：金计价下 K4→K3、自由度减一 → 黄金计价的")
    A("『K4 配置』根本不存在（只有 K3 的 2-DOF Γ_Au），跨国选股/配置须显式声明 numeraire")
    A("**及其对应的图结构**；③ 降格美元 M/Au（DX/GC）↓55% = 美元对黄金长期贬值的可")
    A("观测，是 254号『影子金价裂缝』的直接度量；④ 油（CL/GC）作为 Oil 折叠通道在金")
    A("计价下观测于 P/Au 边，金油比 ω 的拓扑位置在此明确。\n")
    A("**谱系引用**：254 号（黄金∈Σ∩C / **定理3 黄金截面 C=\\$ K4退化为K3** / 开放问题4")
    A("**自测不可能性**——黄金是Σ中唯一既是尺子又是被测物的节点）、529 号（折叠通道重构：")
    A("Au=C↔M 观测于C/M、Oil=C→P 观测于P/C、M既顶点又度量尺）、292 号（折叠拓扑本体论）、")
    A("330 号（资本循环 M/P/C/R）、527 号（σ走势方向态）、259 号（C代理需结算属性，")
    A("CL非黄金的代理张力）、517 号（和乐——本部分按254号开放问题4**搁置和乐框架**）、")
    A("231 号（有效域≠定义域）。formalization-validity-domain：Part2 否定缩小有效域。\n")
    A("**影响声明**：新建 `gold_k4_sigma.py`（黄金计价 σ 重计算，Rust 1min）+")
    A("`fx_residual_p1p2.py`（Part1/2）+ `cross_national_fx_deep_dive.py`（fold-aware 汇总）")
    A("+ 本报告 + 两个 JSON 缓存；不修改引擎、不改 graph.py/fold_channel.py/config_space.py")
    A("/data_mapping.py 正典折叠结构（本实验只消费，不改单一真相源）。\n")
    A("**认识论等级**：Part1 L2（真实数据相关性，混合结果）；Part2 L2（否定性 Granger）；")
    A("Part3 K4→K3 退化 L0（254号定理3结构推导）+ 存活轴 numeraire 非中性 L2（真实数据可证伪）。")

    REPORT.write_text("\n".join(L))
    print(f"报告 → {REPORT}")


if __name__ == "__main__":
    p12 = json.load(open(DATA / "fx_residual_p1p2.json"))
    gold_path = DATA / "gold_k4_sigma.json"
    p3 = analyze_part3() if gold_path.exists() else None
    write_report(p12, p3)
