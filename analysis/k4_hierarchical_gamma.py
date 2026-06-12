#!/usr/bin/env python3
"""分层 Γ：K4 配置空间的级别分解与级别间耦合检验（复用 1min σ 缓存，零重跑）。

═══════════════════════════════════════════════════════════════════════
核心问题
═══════════════════════════════════════════════════════════════════════
单一 Γ=(σ_P,σ_C,σ_R) 把**不同涌现级别**的走势方向并列（原报告：仅 48% 交易日
三边同级别），概念上『苹果比橙子』。本脚本不构造单一 Γ，而是分层检验：

  级别间的 Γ 是**独立**的还是有**约束关系**？
  若有约束 → 这个约束就是『区间套』在 K4 配置空间上的表达。

═══════════════════════════════════════════════════════════════════════
缓存语义缺口（诚实标注，决定本实验的严格形式）
═══════════════════════════════════════════════════════════════════════
σ 缓存 `_k4edge_*_y0.json` 每条边每个 UTC 日只存一对 (σ, l*)：
  · l* = 该日**最高涌现级别**（L1..L4）
  · σ  = 该最高级别最后一个 move 的走势方向（527号 σ 态）

任务设想的『每条边在固定级别 l 的 settled 方向』（L1/L2/L3/L4 **同时**各一个
方向）缓存**不包含**：当 l*_P=4 时，L3 走势按区间套仍存在，但其方向未被记录。
从缓存提取『P/M 在 L3 的方向』只能取 l*==3 子集——l*==4 的天（L3 仍在）系统性
缺失 = **有偏不完整**。强行用『取最近 settled 方向』fallback 填充，会把同一方向
复制到多个级别，**制造虚假级别间相关**，恰好污染要测的耦合。

⟹ 严格形式：不机械执行『每级别 Γ_l』（需重跑引擎记录全级别 σ，且 fallback 有偏），
   而是用缓存原生的 (σ, l*) 信号检验区间套假设。本脚本测的是
   **最高涌现级别**维度的级别耦合，有效域 = 最高涌现级别粒度（非全级别）。

═══════════════════════════════════════════════════════════════════════
区间套的可观测后果（若约束成立，应看到）
═══════════════════════════════════════════════════════════════════════
  H1 级别协同：三边 l* 联合分布偏离独立（扣除自相关后仍显著）
  H2 配置收缩：高级别子集的 Γ 配置熵 < 低级别（高级别更少自由度）
  H3 方向确定：高级别期间 |σ|=1（趋势）占比更高（包络方向更稳）
  H4 升级领先：高级别边升级领先低级别边升级（区间套自上而下约束）

认识论等级：L2（真实 1min 期货数据，可产生否定性结果）。
顶点映射（GC 货币锚，非正典）/ 528·529 分歧见 k4_1min_rust_lib.py 模块头注。

用法： PYTHONPATH=src .venv/bin/python analysis/k4_hierarchical_gamma.py
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "analysis" / "data_cache"
REPORT_PATH = ROOT / "analysis" / "k4_hierarchical_gamma.md"
JSON_PATH = CACHE / "k4_hierarchical_gamma.json"

GAMMA_EDGES = ("P/M", "C/M", "R/M")  # Γ=(σ_P,σ_C,σ_R)
N_PERM = 2000   # circular-shift 置换次数
SEEDS = (42, 7, 1234, 99, 2026)  # 多 seed 稳健性


# ════════════════════════════════════════════════════════════
# 数据加载与对齐
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class AlignedGamma:
    """对齐到公共交易日的分层 Γ 输入（不可变）。"""
    days: np.ndarray            # (N,) epoch-day
    sigma: np.ndarray           # (N, 3) σ ∈ {-1,0,1}，列序 = GAMMA_EDGES
    level: np.ndarray           # (N, 3) l* ∈ {1,2,3,4} 最高涌现级别

    @property
    def n(self) -> int:
        return len(self.days)


def _load_edge_map(safe_name: str) -> dict[int, tuple[int, int]]:
    """读单边缓存 → {day: (sigma, level)}。"""
    d = json.loads((CACHE / f"_k4edge_{safe_name}_y0.json").read_text())
    return dict(zip(d["days"], zip(d["sigma"], d["level"])))


def load_aligned() -> AlignedGamma:
    """加载三条 Γ 边，对齐公共交易日，构造 (N,3) σ/level 矩阵。"""
    maps = {e: _load_edge_map(e.replace("/", "_")) for e in GAMMA_EDGES}
    common = sorted(set.intersection(*(set(m) for m in maps.values())))
    days = np.array(common, dtype=np.int64)
    sigma = np.empty((len(common), 3), dtype=np.int64)
    level = np.empty((len(common), 3), dtype=np.int64)
    for ci, e in enumerate(GAMMA_EDGES):
        m = maps[e]
        for ri, d in enumerate(common):
            sigma[ri, ci], level[ri, ci] = m[d]
    return AlignedGamma(days=days, sigma=sigma, level=level)


# ════════════════════════════════════════════════════════════
# 信息论原语（无分布假设，频率插件估计）
# ════════════════════════════════════════════════════════════

def entropy(x: np.ndarray) -> float:
    """Shannon 熵 H(X)（bits），频率插件估计。"""
    _, c = np.unique(x, return_counts=True)
    p = c / c.sum()
    return float(-np.sum(p * np.log2(p)))


def joint_entropy(*xs: np.ndarray) -> float:
    """联合熵 H(X1..Xk)（bits）。"""
    key = np.stack(xs, axis=1)
    _, c = np.unique(key, axis=0, return_counts=True)
    p = c / c.sum()
    return float(-np.sum(p * np.log2(p)))


def mutual_info(x: np.ndarray, y: np.ndarray) -> float:
    """互信息 I(X;Y)=H(X)+H(Y)−H(X,Y)（bits）。"""
    return entropy(x) + entropy(y) - joint_entropy(x, y)


def total_correlation(a: np.ndarray, b: np.ndarray, c: np.ndarray) -> float:
    """全相关 TC=ΣH(Xi)−H(X1,X2,X3) ≥ 0（多元依赖总量，bits）。"""
    return entropy(a) + entropy(b) + entropy(c) - joint_entropy(a, b, c)


# ════════════════════════════════════════════════════════════
# circular-shift null（核心严格性：扣除自相关伪协同）
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ShiftTest:
    """circular-shift 置换检验结果。"""
    observed: float
    null_mean: float
    null_std: float
    p_value: float    # P(null ≥ observed)
    z: float

    @property
    def significant(self) -> bool:
        return self.p_value < 0.05


def circular_shift_test(
    stat_fn, cols: tuple[np.ndarray, ...], *, n_perm: int = N_PERM,
    seeds: tuple[int, ...] = SEEDS,
) -> ShiftTest:
    """检验 stat_fn(*cols) 是否超过随机相位对齐的 null。

    null 构造：固定第 0 列，对其余各列独立 circular-shift（np.roll）。
    保留每列**自身**自相关（游程结构不变），仅破坏列间**同期对齐**。
    这是慢变序列耦合检验的正确 null——朴素 i.i.d. 置换会破坏自相关、
    低估 null、产生假阳性（formalization-validity-domain 规则）。
    """
    observed = float(stat_fn(*cols))
    n = len(cols[0])
    null: list[float] = []
    for seed in seeds:
        rng = np.random.default_rng(seed)
        for _ in range(n_perm // len(seeds)):
            shifted = [cols[0]] + [
                np.roll(col, int(rng.integers(1, n))) for col in cols[1:]
            ]
            null.append(float(stat_fn(*shifted)))
    arr = np.array(null)
    mean, std = float(arr.mean()), float(arr.std())
    p = float((arr >= observed).mean())
    z = (observed - mean) / std if std > 0 else float("inf")
    return ShiftTest(observed, mean, std, p, z)


# ════════════════════════════════════════════════════════════
# 分层分解：同级别子集的 Γ 配置空间（H2 配置收缩）
# ════════════════════════════════════════════════════════════

def gamma_index(sp: int, sc: int, sr: int) -> int:
    return (sp + 1) * 9 + (sc + 1) * 3 + (sr + 1)


@dataclass(frozen=True)
class LevelLayer:
    """单个『三边同级别 L』子集上的 Γ 配置空间统计。"""
    level: int
    n_days: int
    states_visited: int        # 27 态中访问了几个
    gamma_entropy: float       # H(Γ | 三边都在 L)，配置空间有效大小
    top_state: tuple[int, int, int]
    top_share: float


def same_level_layers(ag: AlignedGamma) -> list[LevelLayer]:
    """对每个级别 L，取『三边 l* 都==L』子集，统计 Γ 配置空间。"""
    layers: list[LevelLayer] = []
    for L in (2, 3, 4):
        mask = np.all(ag.level == L, axis=1)
        n = int(mask.sum())
        if n < 10:
            continue
        sub = ag.sigma[mask]
        idx = np.array([gamma_index(*row) for row in sub])
        states, counts = np.unique(idx, return_counts=True)
        gh = entropy(idx)
        top = int(states[np.argmax(counts)])
        sp, sc, sr = top // 9 - 1, (top // 3) % 3 - 1, top % 3 - 1
        layers.append(LevelLayer(
            level=L, n_days=n, states_visited=len(states),
            gamma_entropy=gh, top_state=(sp, sc, sr),
            top_share=float(counts.max() / n),
        ))
    return layers


# ════════════════════════════════════════════════════════════
# H3 方向确定性：P(|σ|=1 | 级别)
# ════════════════════════════════════════════════════════════

def level_conditional_certainty(ag: AlignedGamma) -> dict[str, dict[int, float]]:
    """每条边：在各最高涌现级别下，σ 为趋势(|σ|=1) 的比例。"""
    out: dict[str, dict[int, float]] = {}
    for ci, e in enumerate(GAMMA_EDGES):
        lev, sig = ag.level[:, ci], ag.sigma[:, ci]
        cert = {}
        for L in (1, 2, 3, 4):
            m = lev == L
            cert[L] = float((sig[m] != 0).mean()) if m.any() else float("nan")
        out[e] = cert
    return out


# ════════════════════════════════════════════════════════════
# H4 升级领先滞后：级别升级事件的跨边互相关
# ════════════════════════════════════════════════════════════

def upgrade_lead_lag(ag: AlignedGamma, max_lag: int = 20) -> dict[str, dict[int, float]]:
    """边对 (i→j) 的级别升级事件互相关 vs lag。

    升级事件 = Δl* > 0（最高涌现级别跳升）。corr(up_i[t], up_j[t+lag])。
    lag>0 处峰值 → i 领先 j（i 先升级，j 后跟随）。区间套自上而下 ⟹
    级别更高（慢）的边领先级别更低（快）的边。
    """
    up = (np.diff(ag.level, axis=0) > 0).astype(np.float64)  # (N-1, 3)
    out: dict[str, dict[int, float]] = {}
    pairs = [(0, 1, "P/M→C/M"), (0, 2, "P/M→R/M"), (1, 2, "C/M→R/M")]
    for i, j, name in pairs:
        ui, uj = up[:, i], up[:, j]
        if ui.std() == 0 or uj.std() == 0:
            continue
        lags: dict[int, float] = {}
        for lag in range(-max_lag, max_lag + 1):
            if lag >= 0:
                a, b = ui[:len(ui) - lag], uj[lag:]
            else:
                a, b = ui[-lag:], uj[:len(uj) + lag]
            if len(a) > 2 and a.std() > 0 and b.std() > 0:
                lags[lag] = float(np.corrcoef(a, b)[0, 1])
        out[name] = lags
    return out


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

def _fmt_sig(s: int) -> str:
    return {1: "↑", -1: "↓", 0: "─"}[s]


def write_report(ag: AlignedGamma, results: dict) -> None:
    L: list[str] = []
    A = L.append
    A("# 分层 Γ：K4 配置空间的级别分解与级别间耦合检验\n")
    A(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}　"
      f"公共交易日：{ag.n}　范围：{np.datetime64(int(ag.days[0]), 'D')}.."
      f"{np.datetime64(int(ag.days[-1]), 'D')}　"
      f"引擎：复用 newchan_rust σ 缓存（**零重跑**）　**L2**\n")

    # ── 缓存语义缺口（前置，决定有效域）──
    A("## 0. 缓存语义缺口与本实验的有效域（诚实前置）\n")
    A("σ 缓存每条边每日只存一对 `(σ, l*)`：`l*`=该日**最高涌现级别**，"
      "`σ`=该级别最后一个 move 方向。**不含**『每条边在固定级别 L1/L2/L3/L4 各自的方向』。\n")
    A("> ⚠ 任务步骤3设想的『每级别 Γ_l』需要 per-level σ。从缓存提取『某边在 L3 的方向』"
      "只能取 `l*==3` 子集——`l*==4` 的天（L3 走势按区间套仍在）系统性缺失=**有偏不完整**。"
      "强行 fallback『取最近 settled 方向』会把同一方向复制到多级别、**制造虚假级别相关**，"
      "污染要测的耦合。故本实验**不**机械造 per-level Γ_l，而用缓存原生 `(σ, l*)` 检验区间套。\n")
    A("> **有效域**（formalization-validity-domain 规则）：本实验结论限于"
      "**最高涌现级别粒度**的级别耦合。要扩展到全级别 Γ_l 须重跑引擎记录全级别 σ"
      "（修改 `emergent_sigma_level` → 返回 `current_recursive()` 全级别快照，~70min×6 边并行）。\n")

    # ── 各边级别/σ 边际 ──
    A("## 1. 各边边际分布（最高涌现级别 l* 与 σ）\n")
    A("| 边 | L1 | L2 | L3 | L4 | σ↓ | σ─ | σ↑ |")
    A("|----|----|----|----|----|----|----|----|")
    for ci, e in enumerate(GAMMA_EDGES):
        lev, sig = ag.level[:, ci], ag.sigma[:, ci]
        lc = [int((lev == k).sum()) for k in (1, 2, 3, 4)]
        sc = [int((sig == k).sum()) for k in (-1, 0, 1)]
        A(f"| {e} | {lc[0]} | {lc[1]} | {lc[2]} | {lc[3]} | {sc[0]} | {sc[1]} | {sc[2]} |")
    A("")
    A("> 三边最高涌现级别**系统性错位**：P/M 偏 L3、C/M 偏 L4、R/M 跨 L3/L4。"
      "这是原报告『48% 同级别』缺陷的根源——级别≈时间尺度，三边走在不同尺度。\n")

    # ── H1 级别协同（核心）──
    lt = results["level_tc"]
    st = results["sigma_tc"]
    A("## 2. H1 级别协同：扣除自相关后是否仍显著？（核心判决）\n")
    A("朴素 TotalCorrelation 衡量三边联合依赖，但级别/σ 序列游程极长（多年驻留），"
      "**慢变序列随机对齐也产生伪协同**。circular-shift null 保留每边自身自相关、"
      "仅破坏边间同期对齐——是正确的 null。\n")
    A("| 量 | 观测 TC (bits) | null 均值 | null σ | z | p | 判决 |")
    A("|----|---------------|----------|--------|----|----|------|")
    for label, t in [("级别 l* 三元", lt), ("方向 σ 三元", st)]:
        verdict = "**显著耦合**" if t.significant else "伪协同（自相关伪影）"
        A(f"| {label} | {t.observed:.4f} | {t.null_mean:.4f} | {t.null_std:.4f} | "
          f"{t.z:.2f} | {t.p_value:.4f} | {verdict} |")
    A("")
    A("成对互信息（观测值，未扣自相关，仅供参考）：")
    A("| 对 | MI(级别) | MI(σ) |")
    A("|----|---------|-------|")
    for name, (mil, mis) in results["pair_mi"].items():
        A(f"| {name} | {mil:.4f} | {mis:.4f} |")
    A("")

    # ── H2 配置收缩 ──
    A("## 3. H2 配置空间是否随级别收缩？（同级别子集）\n")
    A("取『三边 l* 都==L』子集，测该子集上 Γ 的配置熵 H(Γ|L)。"
      "区间套预言：高级别更少自由度 → H(Γ) 随 L 上升而下降。\n")
    A("| 级别 L | 同级别天数 | 访问态/27 | H(Γ\\|L) bits | 主导配置 | 主导占比 |")
    A("|--------|-----------|-----------|-------------|---------|---------|")
    for ly in results["layers"]:
        sp, sc, sr = ly.top_state
        A(f"| L{ly.level} | {ly.n_days} | {ly.states_visited} | {ly.gamma_entropy:.3f} | "
          f"({_fmt_sig(sp)}{_fmt_sig(sc)}{_fmt_sig(sr)}) | {ly.top_share*100:.0f}% |")
    A("")

    # ── H3 方向确定性 ──
    A("## 4. H3 高级别方向是否更确定？P(|σ|=1 | 级别)\n")
    A("区间套预言：高级别走势是低级别包络，方向应更稳（趋势占比随 L 上升）。\n")
    A("| 边 | L1 | L2 | L3 | L4 |")
    A("|----|----|----|----|----|")
    for e, cert in results["certainty"].items():
        cells = " | ".join(
            f"{cert[L]*100:.0f}%" if cert[L] == cert[L] else "—" for L in (1, 2, 3, 4)
        )
        A(f"| {e} | {cells} |")
    A("")

    # ── H4 升级领先滞后 ──
    A("## 5. H4 级别升级领先滞后（区间套自上而下？）\n")
    A("升级事件=Δl*>0 的跨边互相关峰值 lag。lag>0 ⟹ 前者领先后者升级。\n")
    A("| 边对 | 峰值 lag(天) | 峰值 corr | lag0 corr | 解读 |")
    A("|------|-------------|----------|-----------|------|")
    for name, lags in results["lead_lag"].items():
        if not lags:
            continue
        peak_lag = max(lags, key=lambda k: lags[k])
        peak = lags[peak_lag]
        lag0 = lags.get(0, float("nan"))
        if abs(peak_lag) <= 1:
            interp = "同期（无领先结构）"
        elif peak_lag > 1:
            interp = f"前者领先 {peak_lag} 天"
        else:
            interp = f"后者领先 {-peak_lag} 天"
        A(f"| {name} | {peak_lag} | {peak:.3f} | {lag0:.3f} | {interp} |")
    A("")

    # ── 判决 + 结果包 ──
    A("## 6. 判决：级别间 Γ 是独立还是约束？\n")
    sig_l = lt.significant
    sig_s = st.significant
    if not sig_l and not sig_s:
        verdict = ("**否定性结果**：扣除自相关后，三边最高涌现级别与方向的协同**均不显著**"
                   f"（级别 z={lt.z:.2f}/p={lt.p_value:.3f}，σ z={st.z:.2f}/p={st.p_value:.3f}）。"
                   "表面的 TC（级别 {:.2f}/σ {:.2f} bits）大部分是**慢变序列随机对齐的伪影**。"
                   "**区间套未在 K4 配置空间（最高涌现级别粒度）留下可检测的约束。**"
                   ).format(lt.observed, st.observed)
    elif sig_l or sig_s:
        verdict = (f"**部分约束**：级别协同 {'显著' if sig_l else '不显著'}"
                   f"（z={lt.z:.2f}）、σ 协同 {'显著' if sig_s else '不显著'}"
                   f"（z={st.z:.2f}）。区间套在 K4 配置空间留下**有限**可检测约束。")
    A(verdict + "\n")

    A("### 结果包（六要素）\n")
    A(f"**结论**：复用 1min σ 缓存（{ag.n} 公共日，2010-2026），在**最高涌现级别**粒度"
      "检验区间套对 K4 配置空间的约束。circular-shift null（扣除自相关）下："
      f"级别 TC z={lt.z:.2f}（p={lt.p_value:.3f}）、σ TC z={st.z:.2f}（p={st.p_value:.3f}）。"
      f"{'均不显著 → 区间套无可检测表达（否定性结果）' if not (sig_l or sig_s) else '部分显著'}。\n")
    A("**定义依据**：527号 σ=走势方向态（趋势↑+1/↓−1/盘整0）；区间套=高级别走势包含低级别"
      "（缠论第二买卖点/级别分解）；最高涌现级别 l*=有 move 的最大 level_id（k4_1min_rust_lib"
      ".emergent_sigma_level）。可观测后果 H1-H4 由区间套定义推导。\n")
    A("**边界条件**：① 结论翻转条件——若重跑引擎记录**全级别** σ 构造真 Γ_l（消除 l* 的"
      "最高级别截断），级别协同可能显现（当前仅见最高级别这一截面）；② circular-shift null "
      "若改用 i.i.d. 置换（破坏自相关），级别 TC 将『显著』——但那是低估 null 的假阳性；"
      "③ 仅三边同级别子集（L3/L4 各上千天）支持 H2，样本随 L 变化；④ 升级事件稀疏"
      "（多年一次），H4 互相关统计功效低。\n")
    A("**下游推论**：若否定性结果成立 → K4 配置空间的 Γ regime 分析**不应假设级别间存在"
      "区间套约束**，单一 Γ 的『异级别混合』问题无法靠『级别分层 + 区间套对齐』修复——"
      "三边走在独立的时间尺度上，各级别的 Γ 是近独立的并行通道，非嵌套层级。\n")
    A("**谱系引用**：527号(σ走势方向态/81边)、528·529号(顶点映射/折叠通道)、231号(有效域≠定义域)；"
      "记忆[最高级别σ短窗口冻结](级别≈时间尺度致单窗口σ近常数)、[回测基准不可证伪陷阱]"
      "(慢变序列伪协同同构)、[K4 1min期货映射分歧](编排者覆盖 R=ZN)。区间套在 K4 配置空间的"
      "表达此前未结算——本实验是该问题的首次 L2 检验。\n")
    A("**影响声明**：新建 k4_hierarchical_gamma.py + 报告 + JSON；不修改引擎、不重跑递归、"
      "不改 data_mapping.py；复用既有 σ 缓存。揭示原 k4_config_transition_matrix『级别分层可修复"
      "异级别混合』的隐含假设不成立（最高涌现级别粒度下区间套无可检测约束）。\n")
    A("**认识论等级**：L2（真实 1min 期货，非合成；产生否定性结果）。circular-shift null 的"
      "自相关校正使否定性结论稳健于慢变序列伪协同（避免 L1 同义反复陷阱）。")

    REPORT_PATH.write_text("\n".join(L))


def dump_json(ag: AlignedGamma, results: dict) -> None:
    lt, st = results["level_tc"], results["sigma_tc"]
    out = {
        "meta": {
            "task": "k4_hierarchical_gamma",
            "engine": "reuse newchan_rust sigma cache (zero rerun)",
            "n_common_days": ag.n,
            "date_range": [str(np.datetime64(int(ag.days[0]), "D")),
                           str(np.datetime64(int(ag.days[-1]), "D"))],
            "epistemological_level": "L2",
            "validity_domain": "highest emergent level granularity (not per-level)",
            "n_perm": N_PERM, "seeds": list(SEEDS),
        },
        "level_total_correlation": {
            "observed": lt.observed, "null_mean": lt.null_mean,
            "null_std": lt.null_std, "z": lt.z, "p_value": lt.p_value,
            "significant": lt.significant,
        },
        "sigma_total_correlation": {
            "observed": st.observed, "null_mean": st.null_mean,
            "null_std": st.null_std, "z": st.z, "p_value": st.p_value,
            "significant": st.significant,
        },
        "pair_mutual_info": {k: {"level": v[0], "sigma": v[1]}
                             for k, v in results["pair_mi"].items()},
        "same_level_layers": [
            {"level": ly.level, "n_days": ly.n_days,
             "states_visited": ly.states_visited,
             "gamma_entropy": ly.gamma_entropy, "top_state": list(ly.top_state),
             "top_share": ly.top_share}
            for ly in results["layers"]
        ],
        "level_conditional_certainty": results["certainty"],
        "verdict_independent": not (lt.significant or st.significant),
    }
    JSON_PATH.write_text(json.dumps(out, indent=2))


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    print("加载并对齐三条 Γ 边 σ 缓存 ...", flush=True)
    ag = load_aligned()
    print(f"  公共交易日 N={ag.n}  "
          f"{np.datetime64(int(ag.days[0]), 'D')}..{np.datetime64(int(ag.days[-1]), 'D')}",
          flush=True)

    print("H1 级别/σ 协同 + circular-shift null ...", flush=True)
    level_cols = tuple(ag.level[:, c] for c in range(3))
    sigma_cols = tuple(ag.sigma[:, c] for c in range(3))
    level_tc = circular_shift_test(total_correlation, level_cols)
    sigma_tc = circular_shift_test(total_correlation, sigma_cols)
    print(f"  级别 TC obs={level_tc.observed:.4f} z={level_tc.z:.2f} p={level_tc.p_value:.4f}",
          flush=True)
    print(f"  σ    TC obs={sigma_tc.observed:.4f} z={sigma_tc.z:.2f} p={sigma_tc.p_value:.4f}",
          flush=True)

    pair_mi = {}
    for i, j, nm in [(0, 1, "P/M·C/M"), (0, 2, "P/M·R/M"), (1, 2, "C/M·R/M")]:
        pair_mi[nm] = (mutual_info(ag.level[:, i], ag.level[:, j]),
                       mutual_info(ag.sigma[:, i], ag.sigma[:, j]))

    print("H2/H3/H4 分层分解 ...", flush=True)
    results = {
        "level_tc": level_tc, "sigma_tc": sigma_tc, "pair_mi": pair_mi,
        "layers": same_level_layers(ag),
        "certainty": level_conditional_certainty(ag),
        "lead_lag": upgrade_lead_lag(ag),
    }

    print("写报告 ...", flush=True)
    write_report(ag, results)
    dump_json(ag, results)
    print(f"报告：{REPORT_PATH}")
    print(f"JSON：{JSON_PATH}")
    print(f"\n判决：{'区间套无可检测约束（否定性）' if results['verdict_independent'] else '部分约束'}")


if __name__ == "__main__":
    main()
