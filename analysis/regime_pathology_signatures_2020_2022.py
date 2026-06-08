#!/usr/bin/env python3
"""2020-2022 历史验证：空转 / 走资 / 沉没 的 K4 配置拓扑签名。

把三种资本病态（卢麒元框架）形式化为 Γ=(σ_P,σ_C,σ_R) 27 态空间上的**可证伪谓词**，
在 2020-2022（QE 放水 → 通胀 → 加息）三阶段上检验预期签名是否出现（或被证伪）。

数据来源（复用，非重算）
------------------------
`analysis/data_cache/gold_k4_sigma.json` —— 已由 `gold_k4_sigma.py` 用 **Rust**
RecursiveOrchestrator 在 ES/GC、CL/GC、ZN/GC（GC=货币锚 M）三条对数包络比价上
从 1min a0 跑缠论递归（max_levels=6，最高涌现级别 σ，UTC 日边界，zero-lookahead，
从 2010 流式预热）。这正是 Γ 的三条 vertex/money 边：
  σ_P = σ(ES/GC)  生产资本 vs 货币    σ_C = σ(CL/GC)  商品(油) vs 货币
  σ_R = σ(ZN/GC)  国债 vs 货币
配置 Γ 只用这 3 条边（27 态编码），六边 K4 的另 3 条派生边（P/C,P/R,C/R）不参与
27 态。复用与 k4_config_transition_matrix_1min.py 的六边重算 **bit-identical**
（同一 Rust 引擎、同一 GC 锚、同一 σ 方法），避免重复 ~72min 的 O(N²) 递归。

╔══════════════════════════════════════════════════════════════════╗
║ ⚠ 有效域诚实标注（formalization-validity-domain / 231号）          ║
╠══════════════════════════════════════════════════════════════════╣
║ 任务定义的三病态与本数据集（单经济体 US/全球期货，M=GC）的映射关系：  ║
║                                                                    ║
║ · 空转 (MCM' 寄生于 P)：可测。σ_P=−1 ⟺ 生产资本跑输货币锚 ⟹ 资本     ║
║     滞留货币/金融 sphere 不入生产。有效域内（L2）。                  ║
║                                                                    ║
║ · 沉没 (资本沉入 R)：⚠ R=ZN(国债)≠不动产。528号判国债久期属 M/CASH。  ║
║     故本实验测的是『沉入债券/久期避险』，不是『沉入不动产』。          ║
║     任务预期『2021 通胀 R 升值』在 R=ZN 下检验的是债券方向——通胀期    ║
║     收益率上行、债券下跌，故此预期**可能被真实数据证伪**（诚实保留）。  ║
║                                                                    ║
║ · 走资 (跨国残差方向翻转)：⚠⚠ 严格定义 **出有效域**。本数据集是单经济  ║
║     体（ES/GC/CL/ZN 均 US/全球期货），无第二经济体 K4 残差可比。无法   ║
║     测『资本从一个经济体流向另一个』。可测的代理 = 资本从**全部**本土  ║
║     实物资产撤入超主权货币锚(黄金)：(σ_P,σ_C,σ_R)=(−1,−1,−1)。这是    ║
║     『撤入货币』非『跨国流动』，标注为 PROXY，不冒充跨国走资。         ║
╚══════════════════════════════════════════════════════════════════╝

认识论等级：L2（真实 1min 期货数据，非合成；预设签名可被证伪）。

用法：PYTHONPATH=src .venv/bin/python analysis/regime_pathology_signatures_2020_2022.py
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

SIGMA_JSON = ROOT / "analysis" / "data_cache" / "gold_k4_sigma.json"
REPORT_PATH = ROOT / "analysis" / "regime_pathology_signatures_2020_2022.md"
JSON_OUT = ROOT / "analysis" / "data_cache" / "regime_pathology_signatures_2020_2022.json"

# Γ 三条边 → 顶点 σ（gold_k4_sigma 边名）
EDGE_SP = "ES/GC"  # σ_P 生产资本 vs 货币
EDGE_SC = "CL/GC"  # σ_C 商品(油) vs 货币
EDGE_SR = "ZN/GC"  # σ_R 国债 vs 货币


# ════════════════════════════════════════════════════════════
# 日期 ↔ UTC 日序号（day = epoch_sec // 86400）
# ════════════════════════════════════════════════════════════

def date_to_day(iso: str) -> int:
    return int(np.datetime64(iso).astype("datetime64[s]").astype("int64") // 86400)


def day_to_date(day: int) -> str:
    return str(np.datetime64(int(day) * 86400, "s"))[:10]


# 分析窗口（UTC 日边界，左闭右闭）—— 2020-2022 三阶段
WINDOWS: list[tuple[str, str, str, str]] = [
    # (key, 起始日, 结束日, 描述/预期签名)
    ("FULL_2020_2022", "2020-01-01", "2022-12-31", "全窗口 QE→通胀→加息"),
    ("QE_2020", "2020-03-15", "2020-12-31", "无限 QE 放水（预期空转）"),
    ("INFLATION_2021", "2021-01-01", "2021-12-31", "通胀（预期沉没 R 升值）"),
    ("HIKES_2022", "2022-03-16", "2022-12-31", "加息周期（预期走资）"),
]


# ════════════════════════════════════════════════════════════
# 27 态编码
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
# 病态签名谓词（Γ 拓扑签名，可证伪）
# ════════════════════════════════════════════════════════════
#   输入 (sp, sc, sr) ∈ {-1,0,+1}³，返回该日是否携带该病态签名。
#   每个谓词都有明确的证伪条件（见报告与模块头注有效域标注）。

def sig_idling(sp: int, sc: int, sr: int) -> bool:
    """空转（MCM' 寄生于 P）：生产资本跑输货币锚 ⟹ 资本滞留货币/金融 sphere。

    核心判据：σ_P = −1（ES/GC 趋势下，生产资本相对黄金贬值）。
    证伪：σ_P=+1 占优 ⟹ 生产资本跑赢货币 ⟹ 资本入生产，非空转。
    """
    return sp == -1


def sig_idling_pure(sp: int, sc: int, sr: int) -> bool:
    """纯空转：σ_P=−1 且商品也未跑赢货币（资本不在 P 也不在 C，纯滞留 M）。"""
    return sp == -1 and sc <= 0


def sig_sinking_bonds(sp: int, sc: int, sr: int) -> bool:
    """沉没（沉入 R=债券，⚠非不动产）：R 相对其他顶点升值。

    核心判据：σ_R=+1（ZN/GC 趋势上，债券跑赢黄金）且 R 是相对领先者
    （σ_R≥σ_P 且 σ_R≥σ_C，至少一个严格）。
    证伪：σ_R=−1 占优（债券贬值，加息/通胀典型）⟹ 无沉没。
    """
    return sr == 1 and sr >= sp and sr >= sc and (sr > sp or sr > sc)


def sig_flight_proxy(sp: int, sc: int, sr: int) -> bool:
    """走资代理（⚠跨国严格定义出域）：全部本土实物资产撤入货币锚。

    可测代理：(σ_P,σ_C,σ_R)=(−1,−1,−1) —— 生产/商品/债券同时跑输黄金，
    资本撤入超主权货币(黄金)。标注 PROXY：是『撤入货币』非『跨国流动』。
    证伪：该全负态从不/极少出现。
    """
    return sp == -1 and sc == -1 and sr == -1


SIGNATURES = {
    "空转(idling σ_P=−1)": sig_idling,
    "纯空转(σ_P=−1,σ_C≤0)": sig_idling_pure,
    "沉没-债券(σ_R=+1领先)": sig_sinking_bonds,
    "走资-代理(全负撤入货币)": sig_flight_proxy,
}


# ════════════════════════════════════════════════════════════
# 数据加载 + Γ 序列
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class GammaSeries:
    days: np.ndarray   # int64 UTC 日序号（升序）
    sp: np.ndarray     # σ_P
    sc: np.ndarray     # σ_C
    sr: np.ndarray     # σ_R
    gidx: np.ndarray   # 27 态索引


def load_gamma_series() -> GammaSeries:
    with open(SIGMA_JSON) as f:
        d = json.load(f)
    edges = d["edges"]
    maps = {
        e: dict(zip(edges[e]["days"], edges[e]["sigma"]))
        for e in (EDGE_SP, EDGE_SC, EDGE_SR)
    }
    common = sorted(set(maps[EDGE_SP]) & set(maps[EDGE_SC]) & set(maps[EDGE_SR]))
    sp = np.array([maps[EDGE_SP][x] for x in common], dtype=np.int64)
    sc = np.array([maps[EDGE_SC][x] for x in common], dtype=np.int64)
    sr = np.array([maps[EDGE_SR][x] for x in common], dtype=np.int64)
    gidx = np.array(
        [gamma_to_index(int(a), int(b), int(c)) for a, b, c in zip(sp, sc, sr)],
        dtype=np.int64,
    )
    return GammaSeries(np.array(common, dtype=np.int64), sp, sc, sr, gidx)


# ════════════════════════════════════════════════════════════
# 窗口统计
# ════════════════════════════════════════════════════════════

@dataclass
class WindowStats:
    key: str
    desc: str
    d0: int
    d1: int
    n_days: int
    state_count: np.ndarray         # (27,)
    T: np.ndarray                   # (27,27) 转移
    sig_counts: dict[str, int]      # 病态签名命中天数
    mean_polarity: float            # 平均 S=σ_P+σ_C+σ_R
    mean_sp: float
    mean_sc: float
    mean_sr: float
    switch_rate: float


def slice_window(gs: GammaSeries, d0: int, d1: int) -> tuple[np.ndarray, ...]:
    m = (gs.days >= d0) & (gs.days <= d1)
    return gs.days[m], gs.sp[m], gs.sc[m], gs.sr[m], gs.gidx[m]


def window_stats(gs: GammaSeries, key: str, d0_iso: str, d1_iso: str, desc: str) -> WindowStats:
    d0, d1 = date_to_day(d0_iso), date_to_day(d1_iso)
    days, sp, sc, sr, gidx = slice_window(gs, d0, d1)
    n = len(days)

    state_count = np.zeros(27, dtype=np.int64)
    for gi in gidx:
        state_count[int(gi)] += 1

    T = np.zeros((27, 27), dtype=np.int64)
    for k in range(1, n):
        T[int(gidx[k - 1])][int(gidx[k])] += 1

    sig_counts = {
        name: int(sum(fn(int(a), int(b), int(c)) for a, b, c in zip(sp, sc, sr)))
        for name, fn in SIGNATURES.items()
    }

    diag = int(np.trace(T))
    total_tr = max(1, int(T.sum()))
    switch_rate = (total_tr - diag) / total_tr

    pol = sp + sc + sr
    return WindowStats(
        key=key, desc=desc, d0=d0, d1=d1, n_days=n,
        state_count=state_count, T=T, sig_counts=sig_counts,
        mean_polarity=float(pol.mean()) if n else 0.0,
        mean_sp=float(sp.mean()) if n else 0.0,
        mean_sc=float(sc.mean()) if n else 0.0,
        mean_sr=float(sr.mean()) if n else 0.0,
        switch_rate=switch_rate,
    )


# ════════════════════════════════════════════════════════════
# 假设检验（预期签名 vs 真实占比）
# ════════════════════════════════════════════════════════════

@dataclass
class Hypothesis:
    window_key: str
    expected_sig: str
    verdict: str          # 确认 / 证伪 / 弱
    frac: float           # 预期签名在窗口内占比
    note: str


def test_hypotheses(stats: dict[str, WindowStats]) -> list[Hypothesis]:
    """三阶段预期：QE→空转、通胀→沉没、加息→走资。确认阈值 ≥50% 占优。"""
    out: list[Hypothesis] = []
    plan = [
        ("QE_2020", "空转(idling σ_P=−1)",
         "QE 应使资本滞留金融 sphere，σ_P=−1 占优"),
        ("INFLATION_2021", "沉没-债券(σ_R=+1领先)",
         "⚠R=ZN测债券非不动产；通胀期债券方向是开放问题"),
        ("HIKES_2022", "走资-代理(全负撤入货币)",
         "⚠跨国走资出域；代理=全负撤入黄金"),
    ]
    for wk, sig, note in plan:
        w = stats[wk]
        frac = w.sig_counts[sig] / max(1, w.n_days)
        if frac >= 0.5:
            verdict = "确认"
        elif frac >= 0.3:
            verdict = "弱"
        else:
            verdict = "证伪"
        out.append(Hypothesis(wk, sig, verdict, frac, note))
    return out


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

def write_report(gs: GammaSeries, stats: dict[str, WindowStats],
                 hyps: list[Hypothesis]) -> None:
    L: list[str] = []
    L.append("# 2020-2022 历史验证：空转 / 走资 / 沉没的 K4 配置拓扑签名\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
             f"引擎：**newchan_rust.RecursiveOrchestrator**（复用 gold_k4_sigma.json，"
             f"GC 货币锚，1min a0，最高涌现级别 σ，zero-lookahead，从 2010 预热）\n")

    # 有效域标注（核心，前置）
    L.append("## ⚠ 有效域诚实标注（formalization-validity-domain / 231号）\n")
    L.append("三种病态在本数据集（单经济体 US/全球期货，M=GC）的可测性：\n")
    L.append("| 病态 | 任务定义 | 本数据集可测性 | 实际测量对象 |")
    L.append("|------|---------|--------------|------------|")
    L.append("| 空转 | MCM' 寄生于 P | ✅ 有效域内 | σ_P=−1：生产资本跑输黄金=滞留货币sphere |")
    L.append("| 沉没 | 资本沉入不动产 R | ⚠ 域偏移 | R=ZN(国债)≠不动产 → 测『沉入债券/久期』 |")
    L.append("| 走资 | 跨国残差方向翻转 | ⚠⚠ **出域** | 单经济体无跨国残差；代理=全负撤入黄金 |")
    L.append("")
    L.append("**关键后果**：① 任务预期『2021 通胀→沉没(R 升值)』在 R=ZN 下检验的是"
             "**债券方向**，通胀期收益率上行→债券下跌，故此预期**可能被证伪**（保留为否定性结果）。"
             "② 走资严格定义需第二经济体，本实验仅给『撤入货币』代理，**不冒充跨国走资**。\n")

    # 顶点映射
    L.append("## 顶点映射（GC 货币锚版）\n")
    L.append("| 顶点 | 标的 | σ 含义 |")
    L.append("|------|------|--------|")
    L.append("| P | ES | σ_P=σ(ES/GC) 生产资本 vs 货币(黄金) |")
    L.append("| M | GC | 货币/价值锚（分母） |")
    L.append("| C | CL | σ_C=σ(CL/GC) 商品(油) vs 货币 |")
    L.append("| R | ZN | σ_R=σ(ZN/GC) 国债 vs 货币（≡利率/久期，非不动产） |")
    L.append("")

    # 病态签名定义
    L.append("## 病态签名定义（Γ 拓扑谓词，可证伪）\n")
    L.append("σ∈{+1=↑跑赢货币, 0=─盘整, −1=↓跑输货币}，极性 S=σ_P+σ_C+σ_R∈[−3,+3]：\n")
    L.append("- **空转**：σ_P=−1（生产资本跑输黄金⟹资本滞留货币/金融sphere不入生产）。"
             "证伪：σ_P=+1 占优。")
    L.append("- **纯空转**：σ_P=−1 且 σ_C≤0（资本不在 P 也不在 C，纯滞留 M）。")
    L.append("- **沉没-债券**：σ_R=+1 且 σ_R≥σ_P,σ_C（债券是相对领先者，⚠非不动产）。"
             "证伪：σ_R=−1 占优。")
    L.append("- **走资-代理**：(σ_P,σ_C,σ_R)=(−1,−1,−1)（全部撤入黄金，⚠非跨国）。\n")

    # 全历史 σ 基线
    L.append("## 0. 全历史 σ 基线（2010-2026，对照用）\n")
    L.append(f"共同交易日 {len(gs.days)} 天，"
             f"{day_to_date(int(gs.days[0]))} .. {day_to_date(int(gs.days[-1]))}：\n")
    L.append("| 边 | ↑(+1) | ─(0) | ↓(−1) | 均值 |")
    L.append("|----|-------|------|-------|------|")
    for name, arr in (("σ_P=ES/GC", gs.sp), ("σ_C=CL/GC", gs.sc), ("σ_R=ZN/GC", gs.sr)):
        L.append(f"| {name} | {int((arr==1).sum())} | {int((arr==0).sum())} | "
                 f"{int((arr==-1).sum())} | {arr.mean():+.3f} |")
    L.append("")

    # 各窗口
    L.append("## 1. 三阶段窗口统计\n")
    L.append("| 窗口 | 期间 | 天数 | 均 σ_P | 均 σ_C | 均 σ_R | 均极性 S | 切换率 |")
    L.append("|------|------|------|--------|--------|--------|---------|--------|")
    for w in stats.values():
        L.append(f"| {w.key} | {day_to_date(w.d0)}..{day_to_date(w.d1)} | {w.n_days} | "
                 f"{w.mean_sp:+.2f} | {w.mean_sc:+.2f} | {w.mean_sr:+.2f} | "
                 f"{w.mean_polarity:+.2f} | {w.switch_rate*100:.0f}% |")
    L.append("")

    # 病态签名占比
    L.append("## 2. 病态签名命中占比（各窗口内携带签名的交易日比例）\n")
    L.append("| 窗口 | " + " | ".join(SIGNATURES.keys()) + " |")
    L.append("|------|" + "|".join(["------"] * len(SIGNATURES)) + "|")
    for w in stats.values():
        cells = [f"{w.sig_counts[s]}/{w.n_days}={w.sig_counts[s]/max(1,w.n_days)*100:.0f}%"
                 for s in SIGNATURES]
        L.append(f"| {w.key} | " + " | ".join(cells) + " |")
    L.append("")

    # 各窗口 top 配置
    L.append("## 3. 各窗口 top-5 配置停留\n")
    for w in stats.values():
        L.append(f"### {w.key}（{w.desc}）\n")
        order = np.argsort(-w.state_count)
        L.append("| Γ | (σ_P,σ_C,σ_R) | 天数 | 占比 | 极性 |")
        L.append("|----|---------------|------|------|------|")
        for gi in order[:5]:
            cnt = int(w.state_count[gi])
            if cnt == 0:
                continue
            sp, sc, sr = index_to_gamma(int(gi))
            L.append(f"| {gamma_label(int(gi))} | ({sp:+d},{sc:+d},{sr:+d}) | {cnt} | "
                     f"{cnt/max(1,w.n_days)*100:.0f}% | {sp+sc+sr:+d} |")
        L.append("")

    # 假设检验
    L.append("## 4. 假设检验：预期签名 vs 真实占比（确认≥50% / 弱≥30% / 否则证伪）\n")
    L.append("| 阶段 | 预期签名 | 真实占比 | 裁决 | 备注 |")
    L.append("|------|---------|---------|------|------|")
    for h in hyps:
        L.append(f"| {h.window_key} | {h.expected_sig} | {h.frac*100:.0f}% | "
                 f"**{h.verdict}** | {h.note} |")
    L.append("")

    # 阶段叙事（基于真实数据，非预设）
    L.append("## 5. 三阶段实证叙事（数据驱动，非预设）\n")
    qe, infl, hike = stats["QE_2020"], stats["INFLATION_2021"], stats["HIKES_2022"]
    L.append(f"**2020 QE（{qe.n_days}天）**：均 σ_P={qe.mean_sp:+.2f}、"
             f"σ_C={qe.mean_sc:+.2f}、σ_R={qe.mean_sr:+.2f}，极性 S={qe.mean_polarity:+.2f}。"
             f"空转签名(σ_P=−1)占 {qe.sig_counts['空转(idling σ_P=−1)']/max(1,qe.n_days)*100:.0f}%。\n")
    L.append(f"**2021 通胀（{infl.n_days}天）**：均 σ_P={infl.mean_sp:+.2f}、"
             f"σ_C={infl.mean_sc:+.2f}、σ_R={infl.mean_sr:+.2f}，极性 S={infl.mean_polarity:+.2f}。"
             f"沉没-债券签名占 {infl.sig_counts['沉没-债券(σ_R=+1领先)']/max(1,infl.n_days)*100:.0f}%"
             f"（⚠测债券非不动产）。\n")
    L.append(f"**2022 加息（{hike.n_days}天）**：均 σ_P={hike.mean_sp:+.2f}、"
             f"σ_C={hike.mean_sc:+.2f}、σ_R={hike.mean_sr:+.2f}，极性 S={hike.mean_polarity:+.2f}。"
             f"走资-代理(全负)占 {hike.sig_counts['走资-代理(全负撤入货币)']/max(1,hike.n_days)*100:.0f}%。\n")

    # 结果包
    n_confirm = sum(1 for h in hyps if h.verdict == "确认")
    n_falsify = sum(1 for h in hyps if h.verdict == "证伪")
    L.append("## 结果包（六要素）\n")
    L.append(f"**结论**：2020-2022 三阶段在 GC 锚 K4 配置空间上，预期病态签名 "
             f"{n_confirm} 个确认、{n_falsify} 个证伪、{3-n_confirm-n_falsify} 个弱。"
             "签名以 Γ=(σ_P,σ_C,σ_R) 拓扑谓词形式给出，逐日由 Rust 缠论递归最高涌现级别 σ 判定。\n")
    L.append("**定义依据**：σ=527号走势方向态（趋势↑/↓=±1，盘整=0）；最高涌现级别 σ 取"
             "recursive_snapshots 有 move 的最大 level（a_move_v1 走势：盘整1中枢/趋势2+同向中枢）；"
             "病态签名 = 卢麒元框架（空转=MCM'寄生 P / 沉没=资本沉入 R / 走资=跨国残差翻转）"
             "在 Γ 上的形式化谓词。\n")
    L.append("**边界条件**：① σ 取最高涌现级别 → Γ 可观测量随级别漂移；"
             "② R=ZN≠不动产，沉没签名测债券方向，若 R 改回 VNQ 结论翻转；"
             "③ 走资严格定义出域，代理仅测撤入货币，引入第二经济体 K4 才能测真跨国走资；"
             "④ M=GC，若 money 锚改 USD6E 则全部 σ 语义翻转、签名不可比；"
             "⑤ 确认阈值 50%/30% 是约定，改阈值改裁决。\n")
    L.append("**下游推论**：被证伪的预期签名缩小了病态-配置映射的有效域（L2 否定性结果）；"
             "确认的签名可作为 regime 门控候选，但需 L3 交叉验证（多 money 锚 / 引入第二经济体）"
             "才能升级。极性 S 时间序列可作为资本旋转动力学（L=M×ω）的配置侧观测量。\n")
    L.append("**谱系引用**：527号(σ走势方向态)、528/529号(顶点映射,国债≠R)、254号(黄金∈Σ∩C)、"
             "231号(有效域≠定义域)；记忆[资本旋转动力学框架](空转=MCM'寄生)、[卢麒元框架]"
             "(空转压价/沉没才能流转)、[ω regime被证伪](ω→美股方向真实数据反向证伪——"
             "本实验的病态签名同属可证伪范畴)、[K4 1min期货映射分歧](编排者覆盖 R=ZN)。\n")
    L.append("**影响声明**：新建 analysis/regime_pathology_signatures_2020_2022.py + 本报告 + JSON；"
             "复用 gold_k4_sigma.json（不重算 Rust 递归）；不修改引擎；不改 data_mapping.py。\n")
    L.append("**认识论等级**：L2（真实 1min 期货数据，非合成；预设签名可被证伪——"
             "本实验产出否定性结果即缩小有效域）。σ 计算的管线正确性 L0/L1"
             "（Rust 引擎 bit-exact，记忆[RecursiveOrchestrator全Rust化]）。")

    REPORT_PATH.write_text("\n".join(L))


def dump_json(gs: GammaSeries, stats: dict[str, WindowStats],
              hyps: list[Hypothesis]) -> None:
    out = {
        "meta": {
            "task": "regime_pathology_signatures_2020_2022",
            "engine": "newchan_rust.RecursiveOrchestrator (reused via gold_k4_sigma.json)",
            "vertex_mapping": {"P": "ES", "M": "GC", "C": "CL", "R": "ZN"},
            "generated": datetime.now().isoformat(),
            "validity_domain_note": (
                "空转 in-domain; 沉没 R=ZN≠不动产(测债券); 走资 跨国严格出域(仅代理)"
            ),
            "epistemological_level": "L2",
        },
        "windows": {
            w.key: {
                "desc": w.desc,
                "range": [day_to_date(w.d0), day_to_date(w.d1)],
                "n_days": w.n_days,
                "mean_sigma": {"P": w.mean_sp, "C": w.mean_sc, "R": w.mean_sr},
                "mean_polarity": w.mean_polarity,
                "switch_rate": w.switch_rate,
                "signature_counts": w.sig_counts,
                "state_count": w.state_count.tolist(),
            }
            for w in stats.values()
        },
        "hypotheses": [
            {"window": h.window_key, "expected": h.expected_sig,
             "verdict": h.verdict, "fraction": h.frac, "note": h.note}
            for h in hyps
        ],
    }
    JSON_OUT.write_text(json.dumps(out, ensure_ascii=False))


def main() -> None:
    print("加载 gold_k4_sigma.json（复用 Rust 引擎逐日 σ）...", flush=True)
    gs = load_gamma_series()
    print(f"  共同交易日 {len(gs.days)} 天，"
          f"{day_to_date(int(gs.days[0]))}..{day_to_date(int(gs.days[-1]))}", flush=True)

    stats: dict[str, WindowStats] = {}
    for key, d0, d1, desc in WINDOWS:
        w = window_stats(gs, key, d0, d1, desc)
        stats[key] = w
        print(f"  [{key}] {w.n_days} 天  S̄={w.mean_polarity:+.2f}  "
              f"σ_P̄={w.mean_sp:+.2f} σ_C̄={w.mean_sc:+.2f} σ_R̄={w.mean_sr:+.2f}",
              flush=True)

    hyps = test_hypotheses(stats)
    print("\n假设检验：")
    for h in hyps:
        print(f"  {h.window_key}: {h.expected_sig} → {h.verdict}（{h.frac*100:.0f}%）")

    write_report(gs, stats, hyps)
    dump_json(gs, stats, hyps)
    print(f"\n报告：{REPORT_PATH}")
    print(f"JSON：{JSON_OUT}")


if __name__ == "__main__":
    main()
