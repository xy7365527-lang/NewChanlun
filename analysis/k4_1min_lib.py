#!/usr/bin/env python3
"""K4 1min a0 共享库 — 数据加载 / 对数包络比价 / 逐日最高涌现级别 σ。

服务两个任务：
  - k4_config_transition_1min.py（27 态配置转换矩阵）
  - closure_residual_chanlun.py（闭合残差耗散结构验证）

╔══════════════════════════════════════════════════════════════════╗
║ ⚠ 顶点映射与 528/529 号单一真相源的实质分歧（诚实标注，no-workaround）║
╠══════════════════════════════════════════════════════════════════╣
║ 本实验按编排者显式指令使用期货代理（受 10-16 年 1min 数据可得性约束）：║
║   P = ES   (生产资本)        —— 与正典一致 ✓                        ║
║   M = USD6E (美元/货币资本)   —— UUP 的期货代理，概念一致 ✓          ║
║              (usd6e 值域≈0.83 = USD/EUR，已是 6E 倒数 = 美元强度)    ║
║   C = CL   (原油)            —— ⚠ 正典 C=DBC(广义商品)；CL 是 Oil    ║
║              折叠通道观测量(C→P)。此处把折叠压平进 C 顶点 →           ║
║              σ_C 测的是『油』而非广义商品（254/528/529 已识别失效模式）║
║   R = ZN   (10年国债期货)     —— ⚠⚠ 正典 R=VNQ(不动产)；528 号明确把  ║
║              『国债当 R』判为错误（国债久期属 M/CASH，非 R）。        ║
║              σ_R 测的是『利率/债券』而非『不动产』。                  ║
║   Au = GC  (折叠通道 C↔M)     —— 与正典一致 ✓                        ║
║   Oil = CL (折叠通道，与 C 顶点同标的，已压平)                        ║
║                                                                    ║
║ 认识论后果（231 号 / 形式化有效域规则）：本实验产出的转换矩阵是        ║
║ **非正典 K4 顶点集** 上的结构，σ_C≡油、σ_R≡利率。结论 **不可与正典    ║
║ K4 (VERTEX_DATA_SOURCES) 的 regime 节点直接比较**。这是编排者对       ║
║ 528/529 单一真相源的显式覆盖（原则0 + 编排者覆盖权），分歧不静默掩盖。 ║
╚══════════════════════════════════════════════════════════════════╝

认识论等级：L2（真实 1min 期货数据，非合成；可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT / "src") not in sys.path:
    sys.path.insert(0, str(ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

DATA_DIR = ROOT / "analysis" / "data_cache"

# ── 顶点 → 数据文件（本实验期货映射，分歧见模块头注） ──────────────
VERTEX_FILES: dict[str, str] = {
    "P": "es_1m_databento_10y.json",     # 生产资本
    "M": "usd6e_1m_databento_10y.json",  # 货币资本（已倒数=美元强度）
    "C": "cl_1m_databento_10y.json",     # 原油（压平进 C 顶点）
    "R": "zn_1m_databento_10y.json",     # 10年国债（≡利率，非不动产）
}
# 折叠通道观测量（不占顶点）+ 交叉检验源
AUX_FILES: dict[str, str] = {
    "Au": "gc_1m_databento_10y.json",    # 黄金折叠通道 C↔M
    "DX": "dx_1m_databento_10y.json",    # 美元指数（M 交叉检验，2018-2026）
    "BRN": "brn_1m_databento_10y.json",  # 布伦特原油（Oil 交叉检验，2018-2026）
}

# K4 六条边：分子/分母（顶点键）
EDGES: list[tuple[str, str, str]] = [
    ("P/M", "P", "M"),  # 生产资本 vs 货币 → σ_P
    ("C/M", "C", "M"),  # 商品(油) vs 货币 → σ_C
    ("R/M", "R", "M"),  # 国债 vs 货币 → σ_R
    ("P/C", "P", "C"),  # 派生
    ("P/R", "P", "R"),  # 派生
    ("C/R", "C", "R"),  # 派生
]
# 配置 Γ = (σ_P, σ_C, σ_R) 来自三条 vertex→money 边
GAMMA_EDGES = ("P/M", "C/M", "R/M")


# ════════════════════════════════════════════════════════════
# 1. 数据加载（columnar JSON → numpy，向量化时间戳）
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class Series:
    """单标的 OHLC + 时间戳（numpy 向量化）。"""
    symbol: str
    ts: np.ndarray         # int64 epoch 秒
    day: np.ndarray        # int64 epoch//86400（UTC 日序号）
    o: np.ndarray
    h: np.ndarray
    l: np.ndarray
    c: np.ndarray


def load_series(filename: str) -> Series:
    """加载 columnar JSON，时间戳向量化解析为 epoch 秒。"""
    with open(DATA_DIR / filename) as f:
        d = json.load(f)
    ts = np.asarray(
        pd_to_epoch(d["dates"]), dtype=np.int64
    )
    return Series(
        symbol=d["symbol"],
        ts=ts,
        day=ts // 86400,
        o=np.asarray(d["opens"], dtype=np.float64),
        h=np.asarray(d["highs"], dtype=np.float64),
        l=np.asarray(d["lows"], dtype=np.float64),
        c=np.asarray(d["closes"], dtype=np.float64),
    )


def pd_to_epoch(dates: list[str]) -> np.ndarray:
    """向量化把 ISO 日期字符串 → epoch 秒（int64）。

    单位无关：pandas 3.x 解析为 datetime64[us]，先归一到秒分辨率再取 int64，
    避免对纳秒/微秒分辨率写死除数（曾导致 epoch 落在 1970）。
    """
    import pandas as pd
    idx = pd.to_datetime(dates, utc=True)
    # tz-aware → 去 tz → 秒分辨率 → int64 epoch 秒
    return idx.tz_convert(None).to_numpy().astype("datetime64[s]").astype("int64")


# ════════════════════════════════════════════════════════════
# 2. 对数包络比价 OHLC（用户指定方法）
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class RatioBars:
    """A/B 比价 K 线（对数包络法），时间对齐后。"""
    name: str
    ts: np.ndarray
    day: np.ndarray
    o: np.ndarray
    h: np.ndarray
    l: np.ndarray
    c: np.ndarray


def log_envelope_ratio(a: Series, b: Series, name: str) -> RatioBars:
    """对数包络法构造比价 R=A/B 的 OHLC。

    包络法（子频率数据不可得时的保守上下界）：
        R_open  = A_open  / B_open
        R_close = A_close / B_close
        R_high  = A_high  / B_low    （比价极大：分子最高、分母最低）
        R_low   = A_low   / B_high   （比价极小：分子最低、分母最高）

    与朴素逐列除法（A_high/B_high）不同——包络法不低估盘内真实比价极差。
    时间对齐：两序列 epoch 时间戳取交集（inner join）。
    """
    # 时间戳交集（两序列均按时间升序）
    common, ia, ib = np.intersect1d(a.ts, b.ts, return_indices=True)
    ao, ah, al, ac = a.o[ia], a.h[ia], a.l[ia], a.c[ia]
    bo, bh, bl, bc = b.o[ib], b.h[ib], b.l[ib], b.c[ib]

    # 过滤非正值（除法/取对数前）
    valid = (bo > 0) & (bh > 0) & (bl > 0) & (bc > 0) & (ao > 0) & (ah > 0) & (al > 0) & (ac > 0)
    common = common[valid]
    ao, ah, al, ac = ao[valid], ah[valid], al[valid], ac[valid]
    bo, bh, bl, bc = bo[valid], bh[valid], bl[valid], bc[valid]

    ro = ao / bo
    rc = ac / bc
    rh = ah / bl   # 包络上界
    rl = al / bh   # 包络下界
    # 保证 high≥max(o,c), low≤min(o,c)（极少数浮点边界）
    rh = np.maximum.reduce([rh, ro, rc])
    rl = np.minimum.reduce([rl, ro, rc])

    return RatioBars(
        name=name, ts=common, day=common // 86400,
        o=ro, h=rh, l=rl, c=rc,
    )


# ════════════════════════════════════════════════════════════
# 3. 走势方向 σ 提取（最高涌现级别）
# ════════════════════════════════════════════════════════════

def direction_to_sigma(direction: str, kind: str) -> int:
    """走势方向态 σ（527 号：σ=走势方向态）。

    趋势上→+1，趋势下→−1，盘整(consolidation)→0(FLAT)。
    """
    if kind == "consolidation":
        return 0
    if direction == "up":
        return 1
    if direction == "down":
        return -1
    return 0


def emergent_sigma(snap) -> int:
    """从快照读取**最高涌现级别**的走势方向 σ。

    最高涌现级别 = recursive_snapshots 中有 move 的最大 level_id；
    若递归层无 move，回退到 L1 move_snapshot。取该级别最后一个 move 的方向。
    """
    chosen = None
    max_lvl = -1
    for rs in snap.recursive_snapshots:
        if rs.moves and rs.level_id > max_lvl:
            max_lvl = rs.level_id
            chosen = rs.moves[-1]
    if chosen is None:
        moves = snap.move_snapshot.moves
        if moves:
            chosen = moves[-1]
    if chosen is None:
        return 0
    return direction_to_sigma(chosen.direction, chosen.kind)


def emergent_level(snap) -> int:
    """当前涌现的最高级别 id（L1=1，递归层取最大 level_id）。"""
    max_lvl = 1
    for rs in snap.recursive_snapshots:
        if rs.moves:
            max_lvl = max(max_lvl, rs.level_id)
    return max_lvl


# ════════════════════════════════════════════════════════════
# 4. 逐日 σ 流式提取（zero-lookahead，按 UTC 日边界快照）
# ════════════════════════════════════════════════════════════

@dataclass
class EdgeDailySigma:
    """一条边的逐日 σ 时间序列（as-of 每日末状态）。"""
    name: str
    n_bars: int
    days: list[int]          # UTC 日序号
    sigma: list[int]         # 该日末最高涌现级别 σ
    level: list[int]         # 该日末最高涌现级别
    max_level: int
    elapsed: float


def run_edge_daily_sigma(
    name: str,
    ts: np.ndarray,
    day: np.ndarray,
    o: np.ndarray,
    h: np.ndarray,
    l: np.ndarray,
    c: np.ndarray,
    *,
    max_levels: int = 6,
    progress_interval: int = 500_000,
    verbose: bool = False,
) -> EdgeDailySigma:
    """流式跑 RecursiveOrchestrator，在每个 UTC 日边界记录 as-of σ。

    zero-lookahead：每日 σ 仅依赖该日及之前的 bar（流式增量），无未来信息。
    """
    t0 = time.time()
    n = len(c)
    orch = RecursiveOrchestrator(stream_id=name, max_levels=max_levels, stroke_mode="wide")

    days_out: list[int] = []
    sigma_out: list[int] = []
    level_out: list[int] = []
    max_lvl_seen = 1

    snap = None
    for i in range(n):
        bar = Bar(
            ts=datetime.utcfromtimestamp(int(ts[i])),
            open=float(o[i]), high=float(h[i]),
            low=float(l[i]), close=float(c[i]),
        )
        snap = orch.process_bar(bar)
        # 日边界：当前是最后一根，或下一根属于不同 UTC 日
        if i == n - 1 or day[i + 1] != day[i]:
            sg = emergent_sigma(snap)
            lv = emergent_level(snap)
            days_out.append(int(day[i]))
            sigma_out.append(sg)
            level_out.append(lv)
            max_lvl_seen = max(max_lvl_seen, lv)
        if verbose and (i + 1) % progress_interval == 0:
            print(f"    {name}: {i+1:,}/{n:,} ({time.time()-t0:.0f}s)", flush=True)

    return EdgeDailySigma(
        name=name, n_bars=n, days=days_out, sigma=sigma_out,
        level=level_out, max_level=max_lvl_seen, elapsed=time.time() - t0,
    )


def _worker(args: tuple) -> EdgeDailySigma:
    """multiprocessing 顶层 worker（spawn 可 pickle）。"""
    name, ts, day, o, h, l, c, max_levels = args
    return run_edge_daily_sigma(
        name, ts, day, o, h, l, c, max_levels=max_levels, verbose=False
    )


def build_ratio_for_edge(edge: tuple[str, str, str], series: dict[str, Series]) -> RatioBars:
    """按边定义构造对数包络比价。"""
    name, num, den = edge
    return log_envelope_ratio(series[num], series[den], name)
