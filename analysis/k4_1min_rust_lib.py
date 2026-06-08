#!/usr/bin/env python3
"""K4 1min a0 共享库（**Rust 引擎版**）— 逐日最高涌现级别 σ 提取。

与 `k4_1min_lib.py` 的唯一引擎差异：
  `newchan.orchestrator.recursive.RecursiveOrchestrator`（Python，O(N²)，~9h/5M bar）
  → `newchan_rust.RecursiveOrchestrator`（Rust，逐位等价，~23×，~40-50min/5M bar）。
数据加载 / 对数包络比价 / σ 编码逐字复用 `k4_1min_lib`（纯 numpy，与引擎无关）。

╔══════════════════════════════════════════════════════════════════╗
║ ⚠ K4 顶点映射（本任务 = GC 货币锚版，与 k4_1min_lib 的 USD6E 锚不同）║
╠══════════════════════════════════════════════════════════════════╣
║ 用户给定标的集 {ES, GC, CL, ZN}（无 USD6E）。Γ=(σ_P,σ_C,σ_R) 需要   ║
║ vertex/money 三元组，故以 **黄金 GC 为货币/价值锚 M**（卢麒元『金油  ║
║ 两锚』框架；黄金∈Σ∩C 且为传统价值锚）：                              ║
║   P = ES  (生产资本)        —— 正典一致 ✓                           ║
║   M = GC  (货币/价值锚, 黄金) —— 本任务选择（区别于 USD6E 锚版）       ║
║   C = CL  (商品/原油)        —— ⚠ 折叠通道压平进 C 顶点 → σ_C≡油      ║
║   R = ZN  (10年国债)         —— ⚠⚠ 528号判国债久期属 M；编排者显式    ║
║              覆盖(记忆[K4 1min期货映射分歧]) → σ_R≡利率/债券,非不动产 ║
║                                                                    ║
║ Γ 三条边取 vertex/M（P/M=ES/GC, C/M=CL/GC, R/M=ZN/GC），使 σ_C/σ_R   ║
║ 为 vertex-相对-money 的方向。用户列举的 GC/CL、GC/ZN 是 C/M、R/M 的   ║
║ **倒数**（缠论递归下趋势方向相反、盘整不变）——本实现直接构造 vertex/M  ║
║ 边以**避免倒数近似**，零语义损失。                                   ║
║                                                                    ║
║ 认识论后果（231号/形式化有效域规则）：非正典 K4 顶点集，σ_C≡油、       ║
║ σ_R≡利率、M≡黄金。结论**不可与正典 K4 regime 节点直接比较**。L2。     ║
╚══════════════════════════════════════════════════════════════════╝

认识论等级：L2（真实 1min 期货数据，非合成；可产生否定性结果）。
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import newchan_rust  # Rust 引擎（逐位等价于 Python orchestrator）

# 纯数据函数复用（与引擎无关）：加载 / 对数包络比价 / σ dataclass
from analysis.k4_1min_lib import (  # noqa: E402
    EdgeDailySigma,
    RatioBars,
    Series,
    direction_to_sigma,
    load_series,
    log_envelope_ratio,
)

# ── 顶点 → 数据文件（GC 货币锚版，分歧见模块头注） ──────────────────
VERTEX_FILES: dict[str, str] = {
    "P": "es_1m_databento_10y.json",  # 生产资本
    "M": "gc_1m_databento_10y.json",  # 货币/价值锚（黄金）
    "C": "cl_1m_databento_10y.json",  # 商品（油，压平进 C 顶点）
    "R": "zn_1m_databento_10y.json",  # 10年国债（≡利率，R 代理）
}

# K4 六条边：分子/分母（顶点键）
#   三条 vertex→money（构成 Γ）+ 三条派生（交叉验证）。
#   用户列举的两两边等价覆盖：ES/GC=P/M, ES/CL=P/C, ES/ZN=P/R,
#   GC/CL=(C/M)⁻¹, GC/ZN=(R/M)⁻¹, CL/ZN=C/R。
EDGES: list[tuple[str, str, str]] = [
    ("P/M", "P", "M"),  # ES/GC → σ_P
    ("C/M", "C", "M"),  # CL/GC → σ_C
    ("R/M", "R", "M"),  # ZN/GC → σ_R
    ("P/C", "P", "C"),  # ES/CL（派生）
    ("P/R", "P", "R"),  # ES/ZN（派生）
    ("C/R", "C", "R"),  # CL/ZN（派生）
]
GAMMA_EDGES = ("P/M", "C/M", "R/M")  # Γ=(σ_P, σ_C, σ_R)

# 用户字面边名 → 本实现规范边名（报告映射表）
USER_EDGE_MAP: dict[str, tuple[str, str]] = {
    "ES/GC": ("P/M", "同向"),
    "ES/CL": ("P/C", "同向"),
    "ES/ZN": ("P/R", "同向"),
    "GC/CL": ("C/M", "倒数(σ趋势取反)"),
    "GC/ZN": ("R/M", "倒数(σ趋势取反)"),
    "CL/ZN": ("C/R", "同向"),
}


# ════════════════════════════════════════════════════════════
# 走势方向 σ 提取（最高涌现级别）— Rust orchestrator 接口
# ════════════════════════════════════════════════════════════

def _move_tuple_sigma(move_tuple) -> int:
    """Rust MoveTuple → σ（527号走势方向态）。

    MoveTuple head = (kind, direction, seg_start, seg_end, zs_start, zs_end,
                      zs_count, settled)。kind∈{'trend','consolidation'},
    direction∈{'up','down'}。趋势上→+1 / 趋势下→−1 / 盘整→0。
    """
    head = move_tuple[0]
    return direction_to_sigma(head[1], head[0])


def emergent_sigma_level(orch) -> tuple[int, int]:
    """从 Rust orchestrator 读**最高涌现级别**的 (σ, level)。

    最高涌现级别 = 有 move 的最大 level_id（L1=current_moves()，
    递归层=current_recursive() 的 level_id≥2）。取该级别最后一个 move 的方向。
    无 move → (0, 1)。

    zero-lookahead：仅依赖当前已处理 bar 的引擎状态，无未来信息。
    """
    chosen = None
    max_lvl = -1
    mv1 = orch.current_moves()
    if mv1:
        chosen = mv1[-1]
        max_lvl = 1
    for level_id, _zhongshus, moves in orch.current_recursive():
        if moves and level_id > max_lvl:
            max_lvl = level_id
            chosen = moves[-1]
    if chosen is None:
        return 0, 1
    return _move_tuple_sigma(chosen), max(max_lvl, 1)


# ════════════════════════════════════════════════════════════
# 逐日 σ 流式提取（zero-lookahead，按 UTC 日边界快照）
# ════════════════════════════════════════════════════════════

def run_edge_daily_sigma_rust(
    name: str,
    day: np.ndarray,
    o: np.ndarray,
    h: np.ndarray,
    l: np.ndarray,
    c: np.ndarray,
    *,
    max_levels: int = 6,
    progress_interval: int = 1_000_000,
    verbose: bool = False,
) -> EdgeDailySigma:
    """流式跑 Rust RecursiveOrchestrator，在每个 UTC 日边界记录 as-of σ。

    Rust process_bar 仅需 OHLC（结构化状态不依赖时间戳）。
    """
    t0 = time.time()
    n = len(c)
    orch = newchan_rust.RecursiveOrchestrator(max_levels=max_levels, stroke_mode="wide")

    # numpy 标量装箱开销大 → 一次性转 list（O(N²) 主开销在 Rust 内，这一步可忽略）
    ol = o.tolist()
    hl = h.tolist()
    ll = l.tolist()
    cl = c.tolist()
    dl = day.tolist()

    days_out: list[int] = []
    sigma_out: list[int] = []
    level_out: list[int] = []
    max_lvl_seen = 1

    for i in range(n):
        orch.process_bar(ol[i], hl[i], ll[i], cl[i])
        # 日边界：当前是最后一根，或下一根属于不同 UTC 日
        if i == n - 1 or dl[i + 1] != dl[i]:
            sg, lv = emergent_sigma_level(orch)
            days_out.append(int(dl[i]))
            sigma_out.append(sg)
            level_out.append(lv)
            if lv > max_lvl_seen:
                max_lvl_seen = lv
        if verbose and (i + 1) % progress_interval == 0:
            print(f"    {name}: {i + 1:,}/{n:,} ({time.time() - t0:.0f}s)", flush=True)

    return EdgeDailySigma(
        name=name, n_bars=n, days=days_out, sigma=sigma_out,
        level=level_out, max_level=max_lvl_seen, elapsed=time.time() - t0,
    )


def _worker_rust(args: tuple) -> EdgeDailySigma:
    """multiprocessing 顶层 worker（spawn 可 pickle）。"""
    name, day, o, h, l, c, max_levels = args
    return run_edge_daily_sigma_rust(
        name, day, o, h, l, c, max_levels=max_levels, verbose=True
    )


def build_ratio_for_edge(
    edge: tuple[str, str, str], series: dict[str, Series]
) -> RatioBars:
    """按边定义构造对数包络比价（复用 k4_1min_lib.log_envelope_ratio）。"""
    name, num, den = edge
    return log_envelope_ratio(series[num], series[den], name)
