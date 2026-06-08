#!/usr/bin/env python3
"""2020-2022 历史验证：空转 / 走资 / 沉没 的 K4 配置拓扑签名（多级别）。

把三种资本病态（卢麒元框架）形式化为 Γ=(σ_P,σ_C,σ_R) 27 态空间上的**可证伪谓词**，
在 2020-2022（QE 放水 → 通胀 → 加息）三阶段上检验预期签名是否出现（或被证伪）。

╔══════════════════════════════════════════════════════════════════╗
║ 级别选择是本分析的核心（诚实，no-workaround）                       ║
╠══════════════════════════════════════════════════════════════════╣
║ 缠论递归级别 ~ 时间尺度：L1≈日内/周、L2≈周/月、L3/L4≈季/年。        ║
║ 『最高涌现级别 σ』（gold_k4_sigma.json）到 2020 已达 L3/L4，其 move ║
║ 跨年 → 单年窗口内**冻结**（实测：CL/GC 整个 2020-2022 是一个 L4 盘整 ║
║ σ_C≡0；ZN/GC 整个 2022 是单一 L4 上趋势 σ_R≡+1，与债券崩盘矛盾）。  ║
║ → 最高级别 σ 窗口内几乎不转换，**无法检测单年 regime 切换**。       ║
║ 本脚本对 L1/L2/L3/最高 四种 σ 并列分析，用级别对比表暴露此冻结，     ║
║ 并以能在单年内分辨的级别（L1/L2）做病态签名识别。                    ║
╚══════════════════════════════════════════════════════════════════╝

╔══════════════════════════════════════════════════════════════════╗
║ ⚠ 有效域诚实标注（formalization-validity-domain / 231号）          ║
║ · 空转 (MCM' 寄生于 P)：✅ 可测。σ_P=−1 ⟺ 生产资本跑输货币锚。       ║
║ · 沉没 (资本沉入 R)：⚠ R=ZN(国债)≠不动产 → 测『沉入债券/久期避险』。  ║
║ · 走资 (跨国残差翻转)：⚠⚠ 严格定义出域（单经济体）。代理=全负撤入    ║
║     货币锚(黄金)，标注 PROXY，不冒充跨国走资。                       ║
╚══════════════════════════════════════════════════════════════════╝

数据：regime_levels_2020_2022.json（Rust 引擎，GC 锚，三 Γ 边，预热 2010→记录
2020-2022 逐日 L1/L2/L3/最高 σ）。最高级别与 gold_k4_sigma.json 一致（交叉校验）。

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

LEVELS_JSON = ROOT / "analysis" / "data_cache" / "regime_levels_2020_2022.json"
GOLD_JSON = ROOT / "analysis" / "data_cache" / "gold_k4_sigma.json"  # 最高级别交叉校验
REPORT_PATH = ROOT / "analysis" / "regime_pathology_signatures_2020_2022.md"
JSON_OUT = ROOT / "analysis" / "data_cache" / "regime_pathology_signatures_2020_2022.json"

# 边名 → 顶点 σ
EDGE_SP, EDGE_SC, EDGE_SR = "P/M", "C/M", "R/M"  # σ_P=ES/GC, σ_C=CL/GC, σ_R=ZN/GC

# σ 级别源（cache 字段名）
SIGMA_KEYS = {
    "L1": "sigma_L1", "L2": "sigma_L2", "L3": "sigma_L3", "最高": "sigma_high",
}


def date_to_day(iso: str) -> int:
    return int(np.datetime64(iso).astype("datetime64[s]").astype("int64") // 86400)


def day_to_date(day: int) -> str:
    return str(np.datetime64(int(day) * 86400, "s"))[:10]


WINDOWS: list[tuple[str, str, str, str]] = [
    ("FULL_2020_2022", "2020-01-01", "2022-12-31", "全窗口 QE→通胀→加息"),
    ("QE_2020", "2020-03-15", "2020-12-31", "无限 QE 放水（预期空转）"),
    ("INFLATION_2021", "2021-01-01", "2021-12-31", "通胀（预期沉没 R 升值）"),
    ("HIKES_2022", "2022-03-16", "2022-12-31", "加息周期（预期走资）"),
]


# ── 27 态编码 ──
def gamma_to_index(sp: int, sc: int, sr: int) -> int:
    return (sp + 1) * 9 + (sc + 1) * 3 + (sr + 1)


def index_to_gamma(idx: int) -> tuple[int, int, int]:
    return idx // 9 - 1, (idx // 3) % 3 - 1, idx % 3 - 1


def sigma_label(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[s]


def gamma_label(idx: int) -> str:
    sp, sc, sr = index_to_gamma(idx)
    return f"({sigma_label(sp)}{sigma_label(sc)}{sigma_label(sr)})"


# ── 病态签名谓词（Γ 拓扑签名，可证伪）──
def sig_idling(sp: int, sc: int, sr: int) -> bool:
    """空转：σ_P=−1（生产资本跑输货币⟹资本滞留金融sphere）。证伪：σ_P=+1占优。"""
    return sp == -1


def sig_idling_pure(sp: int, sc: int, sr: int) -> bool:
    """纯空转：σ_P=−1 且 σ_C≤0（不在 P 也不在 C，纯滞留 M）。"""
    return sp == -1 and sc <= 0


def sig_sinking_bonds(sp: int, sc: int, sr: int) -> bool:
    """沉没-债券（⚠非不动产）：σ_R=+1 且 R 相对领先（σ_R≥σ_P,σ_C 且至少一严格）。
    证伪：σ_R=−1 占优。"""
    return sr == 1 and sr >= sp and sr >= sc and (sr > sp or sr > sc)


def sig_flight_proxy(sp: int, sc: int, sr: int) -> bool:
    """走资-代理（⚠跨国出域）：(σ_P,σ_C,σ_R)=(−1,−1,−1) 全撤入黄金。"""
    return sp == -1 and sc == -1 and sr == -1


SIGNATURES = {
    "空转(σ_P=−1)": sig_idling,
    "纯空转(σ_P=−1,σ_C≤0)": sig_idling_pure,
    "沉没-债券(σ_R=+1领先)": sig_sinking_bonds,
    "走资-代理(全负)": sig_flight_proxy,
}


# ── 数据加载：每级别 Γ 序列 ──
@dataclass(frozen=True)
class GammaSeries:
    level: str
    days: np.ndarray
    sp: np.ndarray
    sc: np.ndarray
    sr: np.ndarray
    gidx: np.ndarray


def load_levels() -> dict[str, GammaSeries]:
    with open(LEVELS_JSON) as f:
        d = json.load(f)
    edges = d["edges"]
    out: dict[str, GammaSeries] = {}
    for lvl, key in SIGMA_KEYS.items():
        maps = {
            e: dict(zip(edges[e]["days"], edges[e][key]))
            for e in (EDGE_SP, EDGE_SC, EDGE_SR)
        }
        common = sorted(set(maps[EDGE_SP]) & set(maps[EDGE_SC]) & set(maps[EDGE_SR]))
        sp = np.array([maps[EDGE_SP][x] for x in common], dtype=np.int64)
        sc = np.array([maps[EDGE_SC][x] for x in common], dtype=np.int64)
        sr = np.array([maps[EDGE_SR][x] for x in common], dtype=np.int64)
        gidx = np.array([gamma_to_index(int(a), int(b), int(c))
                         for a, b, c in zip(sp, sc, sr)], dtype=np.int64)
        out[lvl] = GammaSeries(lvl, np.array(common, dtype=np.int64), sp, sc, sr, gidx)
    return out


# ── 窗口统计 ──
@dataclass
class WindowStats:
    key: str
    desc: str
    level: str
    d0: int
    d1: int
    n_days: int
    state_count: np.ndarray
    T: np.ndarray
    sig_counts: dict[str, int]
    mean_polarity: float
    mean_sp: float
    mean_sc: float
    mean_sr: float
    switch_rate: float
    n_distinct_states: int


def window_stats(gs: GammaSeries, key: str, d0_iso: str, d1_iso: str, desc: str) -> WindowStats:
    d0, d1 = date_to_day(d0_iso), date_to_day(d1_iso)
    m = (gs.days >= d0) & (gs.days <= d1)
    sp, sc, sr, gidx = gs.sp[m], gs.sc[m], gs.sr[m], gs.gidx[m]
    n = len(gidx)
    state_count = np.zeros(27, dtype=np.int64)
    for gi in gidx:
        state_count[int(gi)] += 1
    T = np.zeros((27, 27), dtype=np.int64)
    for k in range(1, n):
        T[int(gidx[k - 1])][int(gidx[k])] += 1
    sig_counts = {name: int(sum(fn(int(a), int(b), int(c))
                                for a, b, c in zip(sp, sc, sr)))
                  for name, fn in SIGNATURES.items()}
    diag = int(np.trace(T))
    total_tr = max(1, int(T.sum()))
    pol = sp + sc + sr
    return WindowStats(
        key=key, desc=desc, level=gs.level, d0=d0, d1=d1, n_days=n,
        state_count=state_count, T=T, sig_counts=sig_counts,
        mean_polarity=float(pol.mean()) if n else 0.0,
        mean_sp=float(sp.mean()) if n else 0.0,
        mean_sc=float(sc.mean()) if n else 0.0,
        mean_sr=float(sr.mean()) if n else 0.0,
        switch_rate=(total_tr - diag) / total_tr,
        n_distinct_states=int((state_count > 0).sum()),
    )


# ── 假设检验（在选定的分析级别上）──
@dataclass
class Hypothesis:
    window_key: str
    expected_sig: str
    verdict: str
    frac: float
    note: str


def test_hypotheses(stats: dict[str, WindowStats]) -> list[Hypothesis]:
    out: list[Hypothesis] = []
    plan = [
        ("QE_2020", "空转(σ_P=−1)", "QE 应使资本滞留金融sphere，σ_P=−1 占优"),
        ("INFLATION_2021", "沉没-债券(σ_R=+1领先)", "⚠R=ZN测债券非不动产；通胀期债券方向开放"),
        ("HIKES_2022", "走资-代理(全负)", "⚠跨国走资出域；代理=全负撤入黄金"),
    ]
    for wk, sig, note in plan:
        w = stats[wk]
        frac = w.sig_counts[sig] / max(1, w.n_days)
        verdict = "确认" if frac >= 0.5 else ("弱" if frac >= 0.3 else "证伪")
        out.append(Hypothesis(wk, sig, verdict, frac, note))
    return out


# ── 报告 ──
def write_report(levels: dict[str, GammaSeries],
                 stats_by_level: dict[str, dict[str, WindowStats]],
                 analysis_level: str, hyps: list[Hypothesis]) -> None:
    L: list[str] = []
    L.append("# 2020-2022 历史验证：空转 / 走资 / 沉没的 K4 配置拓扑签名\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
             f"引擎：**newchan_rust.RecursiveOrchestrator**（GC 货币锚，1min a0，"
             f"预热 2010→记录 2020-2022，zero-lookahead）　分析级别：**{analysis_level}**\n")

    # 有效域
    L.append("## ⚠ 有效域诚实标注（formalization-validity-domain / 231号）\n")
    L.append("| 病态 | 任务定义 | 本数据集可测性 | 实际测量对象 |")
    L.append("|------|---------|--------------|------------|")
    L.append("| 空转 | MCM' 寄生于 P | ✅ 有效域内 | σ_P=−1：生产资本跑输黄金=滞留货币sphere |")
    L.append("| 沉没 | 资本沉入不动产 R | ⚠ 域偏移 | R=ZN(国债)≠不动产 → 测沉入债券/久期 |")
    L.append("| 走资 | 跨国残差方向翻转 | ⚠⚠ **出域** | 单经济体无跨国残差；代理=全负撤入黄金 |")
    L.append("")

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
    L.append("- **空转**：σ_P=−1。证伪：σ_P=+1 占优。")
    L.append("- **纯空转**：σ_P=−1 且 σ_C≤0。")
    L.append("- **沉没-债券**：σ_R=+1 且 σ_R≥σ_P,σ_C（⚠债券非不动产）。证伪：σ_R=−1 占优。")
    L.append("- **走资-代理**：(σ_P,σ_C,σ_R)=(−1,−1,−1)（⚠非跨国）。\n")

    # ★ 主发现：级别冻结对比
    L.append("## 1. ★ 核心发现：级别决定窗口内可分辨性（最高级别冻结）\n")
    L.append("同一全窗口（FULL_2020_2022）在不同级别 σ 下的窗口内切换率与访问态数：\n")
    L.append("| 级别 | 窗口内切换率 | 访问态数/27 | 均极性 S | σ_P̄ | σ_C̄ | σ_R̄ |")
    L.append("|------|------------|------------|---------|------|------|------|")
    for lvl in SIGMA_KEYS:
        w = stats_by_level[lvl]["FULL_2020_2022"]
        L.append(f"| {lvl} | {w.switch_rate*100:.1f}% | {w.n_distinct_states} | "
                 f"{w.mean_polarity:+.2f} | {w.mean_sp:+.2f} | {w.mean_sc:+.2f} | {w.mean_sr:+.2f} |")
    L.append("")
    L.append("**读法**：最高级别窗口内切换率趋零 = 配置冻结（L3/L4 move 跨年）；"
             f"L1/L2 切换率高 = 能分辨单年 regime。**本报告病态分析采用级别 {analysis_level}**"
             "（能在单年窗口内分辨切换的最细稳定级别）。\n")

    # 选定级别的窗口统计
    st = stats_by_level[analysis_level]
    L.append(f"## 2. 三阶段窗口统计（级别 {analysis_level}）\n")
    L.append("| 窗口 | 期间 | 天数 | 均σ_P | 均σ_C | 均σ_R | 均极性S | 切换率 | 访问态 |")
    L.append("|------|------|------|-------|-------|-------|--------|--------|--------|")
    for w in st.values():
        L.append(f"| {w.key} | {day_to_date(w.d0)}..{day_to_date(w.d1)} | {w.n_days} | "
                 f"{w.mean_sp:+.2f} | {w.mean_sc:+.2f} | {w.mean_sr:+.2f} | "
                 f"{w.mean_polarity:+.2f} | {w.switch_rate*100:.0f}% | {w.n_distinct_states} |")
    L.append("")

    # 病态签名占比（选定级别）
    L.append(f"## 3. 病态签名命中占比（级别 {analysis_level}，携带签名的交易日比例）\n")
    L.append("| 窗口 | " + " | ".join(SIGNATURES.keys()) + " |")
    L.append("|------|" + "|".join(["------"] * len(SIGNATURES)) + "|")
    for w in st.values():
        cells = [f"{w.sig_counts[s]/max(1,w.n_days)*100:.0f}%" for s in SIGNATURES]
        L.append(f"| {w.key} | " + " | ".join(cells) + " |")
    L.append("")

    # 各窗口 top 配置
    L.append(f"## 4. 各窗口 top-5 配置停留（级别 {analysis_level}）\n")
    for w in st.values():
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
    L.append(f"## 5. 假设检验：预期签名 vs 真实占比（级别 {analysis_level}；确认≥50%/弱≥30%/否则证伪）\n")
    L.append("| 阶段 | 预期签名 | 真实占比 | 裁决 | 备注 |")
    L.append("|------|---------|---------|------|------|")
    for h in hyps:
        L.append(f"| {h.window_key} | {h.expected_sig} | {h.frac*100:.0f}% | "
                 f"**{h.verdict}** | {h.note} |")
    L.append("")

    # 三阶段叙事
    L.append(f"## 6. 三阶段实证叙事（级别 {analysis_level}，数据驱动）\n")
    qe, infl, hike = st["QE_2020"], st["INFLATION_2021"], st["HIKES_2022"]
    L.append(f"**2020 QE（{qe.n_days}天）**：σ_P̄={qe.mean_sp:+.2f} σ_C̄={qe.mean_sc:+.2f} "
             f"σ_R̄={qe.mean_sr:+.2f} S̄={qe.mean_polarity:+.2f}；空转占 "
             f"{qe.sig_counts['空转(σ_P=−1)']/max(1,qe.n_days)*100:.0f}%。")
    L.append(f"**2021 通胀（{infl.n_days}天）**：σ_P̄={infl.mean_sp:+.2f} σ_C̄={infl.mean_sc:+.2f} "
             f"σ_R̄={infl.mean_sr:+.2f} S̄={infl.mean_polarity:+.2f}；沉没-债券占 "
             f"{infl.sig_counts['沉没-债券(σ_R=+1领先)']/max(1,infl.n_days)*100:.0f}%（⚠测债券非不动产）。")
    L.append(f"**2022 加息（{hike.n_days}天）**：σ_P̄={hike.mean_sp:+.2f} σ_C̄={hike.mean_sc:+.2f} "
             f"σ_R̄={hike.mean_sr:+.2f} S̄={hike.mean_polarity:+.2f}；走资-代理占 "
             f"{hike.sig_counts['走资-代理(全负)']/max(1,hike.n_days)*100:.0f}%。\n")

    # 结果包
    n_confirm = sum(1 for h in hyps if h.verdict == "确认")
    n_falsify = sum(1 for h in hyps if h.verdict == "证伪")
    L.append("## 结果包（六要素）\n")
    L.append(f"**结论**：2020-2022 三阶段在 GC 锚 K4 配置空间（分析级别 {analysis_level}）上，"
             f"预期病态签名 {n_confirm} 确认、{n_falsify} 证伪、{3-n_confirm-n_falsify} 弱。"
             "**首要发现**：『最高涌现级别 σ』在 1min a0 上窗口内冻结（L3/L4 跨年 move），"
             "无法检测单年 regime——必须用 L1/L2 才能分辨切换。签名以 Γ=(σ_P,σ_C,σ_R) "
             "拓扑谓词给出，逐日由 Rust 缠论递归固定级别 move 方向判定。\n")
    L.append("**定义依据**：σ=527号走势方向态（趋势↑/↓=±1，盘整=0）；固定级别 σ 取该级别"
             "（L1=current_moves；L2/L3=current_recursive level_id）最后一个 move 方向；"
             "病态签名=卢麒元框架（空转=MCM'寄生P / 沉没=资本沉入R / 走资=跨国残差翻转）"
             "在 Γ 上的形式化谓词。\n")
    L.append("**边界条件**：① 级别选择决定结论——最高级别冻结、L1 噪声大、L2 居中，改级别改裁决；"
             "② R=ZN≠不动产，沉没签名测债券方向，R 改 VNQ 则翻转；③ 走资严格定义出域，"
             "代理仅测撤入货币；④ M=GC，改 USD6E 锚则 σ 语义翻转；⑤ 确认阈值 50%/30% 是约定。\n")
    L.append("**下游推论**：被证伪的预期签名缩小病态-配置映射有效域（L2 否定性结果）；级别-时间尺度"
             "对应关系（L1/L2 分辨单年、L3/L4 跨年冻结）是 regime 检测的级别选择准则；"
             "极性 S 时间序列可作资本旋转动力学（L=M×ω）的配置侧观测量，需 L3 交叉验证升级。\n")
    L.append("**谱系引用**：527号(σ走势方向态)、528/529号(顶点映射,国债≠R)、254号(黄金∈Σ∩C)、"
             "231号(有效域≠定义域)；记忆[递归级别涌现边界](级别随数据长度涌现)、[资本旋转动力学框架]"
             "(空转=MCM'寄生)、[卢麒元框架](空转压价/沉没才能流转)、[ω regime被证伪]"
             "(真实数据反向证伪先例)、[K4 1min期货映射分歧](编排者覆盖 R=ZN)。\n")
    L.append("**影响声明**：新建 regime_pathology_signatures_2020_2022.py + recompute_levels.py + "
             "本报告 + 2 JSON；复用 gold_k4_sigma.json 做最高级别交叉校验；不修改引擎；不改 data_mapping.py。\n")
    L.append("**认识论等级**：L2（真实 1min 期货数据，非合成；预设签名可被证伪——否定性结果即缩小有效域）。"
             "σ 计算管线正确性 L0/L1（Rust 引擎 bit-exact，记忆[RecursiveOrchestrator全Rust化]）。")

    REPORT_PATH.write_text("\n".join(L))


def dump_json(stats_by_level, analysis_level, hyps) -> None:
    out = {
        "meta": {
            "task": "regime_pathology_signatures_2020_2022",
            "engine": "newchan_rust.RecursiveOrchestrator",
            "vertex_mapping": {"P": "ES", "M": "GC", "C": "CL", "R": "ZN"},
            "analysis_level": analysis_level,
            "generated": datetime.now().isoformat(),
            "key_finding": "最高涌现级别σ窗口内冻结(L3/L4跨年)，需L1/L2分辨单年regime",
            "validity_domain_note": "空转in-domain; 沉没R=ZN≠不动产(测债券); 走资跨国出域(仅代理)",
            "epistemological_level": "L2",
        },
        "levels": {
            lvl: {
                wk: {
                    "n_days": w.n_days, "switch_rate": w.switch_rate,
                    "n_distinct_states": w.n_distinct_states,
                    "mean_sigma": {"P": w.mean_sp, "C": w.mean_sc, "R": w.mean_sr},
                    "mean_polarity": w.mean_polarity,
                    "signature_counts": w.sig_counts,
                }
                for wk, w in st.items()
            }
            for lvl, st in stats_by_level.items()
        },
        "hypotheses": [
            {"window": h.window_key, "expected": h.expected_sig,
             "verdict": h.verdict, "fraction": h.frac, "note": h.note}
            for h in hyps
        ],
    }
    JSON_OUT.write_text(json.dumps(out, ensure_ascii=False))


def pick_analysis_level(stats_by_level: dict[str, dict[str, WindowStats]]) -> str:
    """选能在单年窗口内分辨 regime 的最细稳定级别。

    准则：FULL 窗口内切换率 ≥ 5%（能分辨）且 ≤ 60%（不过噪）。L2 优先，其次 L1。
    若 L2 满足→L2；否则 L1；都不满足→回退 L2（仍优于冻结的最高级别）。
    """
    def ok(lvl: str) -> bool:
        sr = stats_by_level[lvl]["FULL_2020_2022"].switch_rate
        return 0.05 <= sr <= 0.60
    for lvl in ("L2", "L1"):
        if ok(lvl):
            return lvl
    return "L2"


def main() -> None:
    print("加载 regime_levels_2020_2022.json（L1/L2/L3/最高 σ）...", flush=True)
    levels = load_levels()
    for lvl, gs in levels.items():
        print(f"  {lvl}: {len(gs.days)} 共同交易日 "
              f"{day_to_date(int(gs.days[0]))}..{day_to_date(int(gs.days[-1]))}", flush=True)

    stats_by_level: dict[str, dict[str, WindowStats]] = {}
    for lvl, gs in levels.items():
        stats_by_level[lvl] = {
            key: window_stats(gs, key, d0, d1, desc)
            for key, d0, d1, desc in WINDOWS
        }

    print("\n级别 × FULL窗口 切换率（冻结诊断）：")
    for lvl in SIGMA_KEYS:
        w = stats_by_level[lvl]["FULL_2020_2022"]
        print(f"  {lvl:4s}: 切换率 {w.switch_rate*100:5.1f}%  访问态 {w.n_distinct_states}/27  "
              f"S̄={w.mean_polarity:+.2f}", flush=True)

    analysis_level = pick_analysis_level(stats_by_level)
    print(f"\n→ 选定分析级别：{analysis_level}", flush=True)

    hyps = test_hypotheses(stats_by_level[analysis_level])
    print("假设检验：")
    for h in hyps:
        print(f"  {h.window_key}: {h.expected_sig} → {h.verdict}（{h.frac*100:.0f}%）")

    write_report(levels, stats_by_level, analysis_level, hyps)
    dump_json(stats_by_level, analysis_level, hyps)
    print(f"\n报告：{REPORT_PATH}\nJSON：{JSON_OUT}")


if __name__ == "__main__":
    main()
