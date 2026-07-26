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

import json
import os
import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import newchan_rust  # Rust 引擎（逐位等价于 Python orchestrator）
# ⚠口径变更（#246 裁定 supersede #84 点3；#277 裁路①、#288 落码 2026-07-26）：
# 段层相切边界（三笔重叠含端点、缺口严格 >）两侧同批切——逐位等价（Rust≡Python）
# 在新口径下仍成立，但与旧口径的历史输出在相切边界上不再逐位一致。裁定书：
# chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md

# 纯数据函数复用（与引擎无关）：加载 / 对数包络比价 / σ dataclass
from analysis.k4_1min_lib import (  # noqa: E402
    EdgeDailySigma,
    RatioBars,
    Series,
    direction_to_sigma,
    load_series,
    log_envelope_ratio,
)

# ── 货币锚可配置（env K4_MONEY_ANCHOR，默认 GC）──────────────────────
#   GC：黄金锚（历史默认，σ_M=相对黄金）— 数据 2012-2026 ≈13.6 年
#   DX：美元指数锚（正典 M，528/529 编排者裁决 A；σ_M=相对美元）— 数据 2018-2026 ≈8 年
#   6E：欧元汇率锚（USD6E，美元强度代理）— 数据 2018-2026
# cache key 含 anchor（edge_cache_path），不同锚的边缓存物理隔离，杜绝错误 resume。
ANCHOR_FILES: dict[str, str] = {
    "GC": "gc_1m_databento_10y.json",
    "DX": "dx_1m_databento_10y.json",
    "6E": "usd6e_1m_databento_10y.json",
}
MONEY_ANCHOR: str = os.environ.get("K4_MONEY_ANCHOR", "GC").upper()
if MONEY_ANCHOR not in ANCHOR_FILES:
    raise SystemExit(f"K4_MONEY_ANCHOR={MONEY_ANCHOR!r} 非法，可选 {list(ANCHOR_FILES)}")

# ── 顶点 → 数据文件（M 随 anchor，分歧见模块头注 + 528/529 裁决 A）──────
VERTEX_FILES: dict[str, str] = {
    "P": "es_1m_databento_10y.json",   # 生产资本
    "M": ANCHOR_FILES[MONEY_ANCHOR],   # 货币锚（GC/DX/6E，见上）
    "C": "cl_1m_databento_10y.json",   # 商品（油，压平进 C 顶点）
    "R": "zn_1m_databento_10y.json",   # 10年国债（≡利率，R 代理）
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

    # 恒定内存迭代：不 tolist（曾导致 5.4M×5列 Python float list ≈1.1GB/worker，
    # 六路并行叠加触顶内存）。numpy 标量 float() 转换的 Python 层开销相对 Rust
    # O(N²) 主开销可忽略。
    days_out: list[int] = []
    sigma_out: list[int] = []
    level_out: list[int] = []
    max_lvl_seen = 1

    for i in range(n):
        orch.process_bar(float(o[i]), float(h[i]), float(l[i]), float(c[i]))
        # 日边界：当前是最后一根，或下一根属于不同 UTC 日
        if i == n - 1 or day[i + 1] != day[i]:
            sg, lv = emergent_sigma_level(orch)
            days_out.append(int(day[i]))
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


# ════════════════════════════════════════════════════════════
# 单边结果落盘 / resume（防 87min 工作因末尾崩溃全丢）
# ════════════════════════════════════════════════════════════

EDGE_CACHE_DIR = ROOT / "analysis" / "data_cache"


def edge_cache_path(name: str, years: int) -> Path:
    """单边 σ 结果 cache 路径（anchor + years 区分，避免不同锚/范围错误 resume）。"""
    safe = name.replace("/", "_")
    return EDGE_CACHE_DIR / f"_k4edge_{MONEY_ANCHOR}_{safe}_y{years}.json"


def dump_edge_cache(r: EdgeDailySigma, years: int) -> None:
    edge_cache_path(r.name, years).write_text(json.dumps({
        "name": r.name, "n_bars": r.n_bars,
        "days": [int(x) for x in r.days],
        "sigma": [int(x) for x in r.sigma],
        "level": [int(x) for x in r.level],
        "max_level": r.max_level, "elapsed": r.elapsed,
    }))


def load_edge_cache(name: str, years: int) -> EdgeDailySigma | None:
    p = edge_cache_path(name, years)
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    return EdgeDailySigma(
        name=d["name"], n_bars=d["n_bars"], days=d["days"],
        sigma=d["sigma"], level=d["level"],
        max_level=d["max_level"], elapsed=d["elapsed"],
    )


def _worker_rust(args: tuple) -> EdgeDailySigma:
    """multiprocessing 顶层 worker（spawn 可 pickle）。

    完成即落盘单边 cache → 即使其他 worker 在末尾崩溃，本边结果不丢、可 resume。
    Rust 递归异常（panic/资源）显式打印边名后重抛，不静默吞（no-workaround）。
    """
    name, day, o, h, l, c, max_levels, years = args
    try:
        r = run_edge_daily_sigma_rust(
            name, day, o, h, l, c, max_levels=max_levels, verbose=True
        )
    except BaseException as e:  # noqa: BLE001 — 含 PyO3 PanicException
        print(f"  !! edge {name} FAILED: {type(e).__name__}: {e}", flush=True)
        raise
    dump_edge_cache(r, years)
    print(f"  {name}: cached → {edge_cache_path(name, years).name}", flush=True)
    return r


def build_ratio_for_edge(
    edge: tuple[str, str, str], series: dict[str, Series]
) -> RatioBars:
    """按边定义构造对数包络比价（复用 k4_1min_lib.log_envelope_ratio）。"""
    name, num, den = edge
    return log_envelope_ratio(series[num], series[den], name)


def compute_or_load_edge_sigmas(
    years: int = 0, procs: int = 6, *, verbose: bool = True
) -> dict[str, "EdgeDailySigma"]:
    """加载顶点 1min 序列 → 6 条边对数包络比价 → 并行 Rust 缠论递归（resume+缓存）。

    返回 {edge_name: EdgeDailySigma}。当前 MONEY_ANCHOR 决定 M 顶点与 cache key，
    不同锚的缓存物理隔离（edge_cache_path 含 anchor），杜绝错误 resume。
    27 态版与 6 维版共用本函数（DRY）。
    """
    import multiprocessing as mp
    from datetime import datetime, timezone

    if verbose:
        print(f"[1/3] 加载顶点 1min 序列（anchor={MONEY_ANCHOR}, M={VERTEX_FILES['M']}）...",
              flush=True)
    series = {}
    for v, fn in VERTEX_FILES.items():
        s = load_series(fn)
        series[v] = s
        if verbose:
            print(f"  {v} = {s.symbol:8s}  {len(s.c):>11,} bars  "
                  f"{np.datetime64(int(s.ts[0]), 's')}..{np.datetime64(int(s.ts[-1]), 's')}",
                  flush=True)

    if verbose:
        print("[2/3] 构造六条边对数包络比价 ...", flush=True)
    ratios = {}
    for edge in EDGES:
        rb = build_ratio_for_edge(edge, series)
        if years > 0:
            # 对齐 UTC 日边界：消除 datetime.now() 秒级漂移导致 cache resume 永久 miss。
            cutoff = (int(datetime.now(timezone.utc).timestamp())
                      - years * 365 * 86400) // 86400 * 86400
            mask = rb.ts >= cutoff
            rb = type(rb)(name=rb.name, ts=rb.ts[mask], day=rb.day[mask],
                          o=rb.o[mask], h=rb.h[mask], l=rb.l[mask], c=rb.c[mask])
        ratios[edge[0]] = rb
        if verbose:
            print(f"  {rb.name:5s} {len(rb.c):>11,} bars", flush=True)

    if verbose:
        print(f"[3/3] 六边 Rust 缠论递归（procs={procs}，resume 可用）...", flush=True)
    edge_results: dict[str, EdgeDailySigma] = {}
    todo = []
    for rb in ratios.values():
        cached = load_edge_cache(rb.name, years)
        if cached is not None and cached.n_bars == len(rb.c):
            edge_results[rb.name] = cached
            if verbose:
                print(f"  {rb.name:5s} resumed: {cached.n_bars:>11,} bars, "
                      f"{len(cached.days)} days, max_lvl=L{cached.max_level}", flush=True)
        else:
            todo.append((rb.name, rb.day, rb.o, rb.h, rb.l, rb.c, 6, years))
    if todo:
        ctx = mp.get_context("spawn")
        with ctx.Pool(processes=procs) as pool:
            for r in pool.imap_unordered(_worker_rust, todo):
                edge_results[r.name] = r
                if verbose:
                    print(f"  {r.name:5s} done: {r.n_bars:>11,} bars, {len(r.days)} days, "
                          f"max_lvl=L{r.max_level}, {r.elapsed:.0f}s", flush=True)
    return edge_results
