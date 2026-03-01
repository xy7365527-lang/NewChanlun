"""K4 配置读取 — 从六条边的 RecursiveOrchestratorSnapshot 构建 K4State。

K4 完全图模型：四顶点 E/Au/R/$，六条边。
每条边各自跑 RecursiveOrchestrator，产出 D 算子读数。
Configuration 三元组是 E/$, Au/$, R/$ 三条边的压缩视图。

认识论标注：L0（从定义推导的映射）。
谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from newchan.backtest.types import (
    EdgeState,
    K4State,
    extract_d_reading,
)
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.topology.config_space import polarity_index
from newchan.topology.k4_scanner import (
    K4ScanResult,
    k4_configuration,
    scan_k4,
    walk_direction_from_snapshot,
)


def _build_edge_state(
    snapshot: RecursiveOrchestratorSnapshot,
    edge_label: str,
    vertex_from: str,
    vertex_to: str,
    level: int = 0,
) -> EdgeState:
    """从快照构造单条边的 EdgeState。"""
    return EdgeState(
        edge_label=edge_label,
        vertex_from=vertex_from,
        vertex_to=vertex_to,
        d_reading=extract_d_reading(snapshot),
        walk_direction=walk_direction_from_snapshot(snapshot, level),
    )


def read_k4_config(
    e_au_snapshot: RecursiveOrchestratorSnapshot,
    e_r_snapshot: RecursiveOrchestratorSnapshot,
    e_usd_snapshot: RecursiveOrchestratorSnapshot,
    au_r_snapshot: RecursiveOrchestratorSnapshot,
    au_usd_snapshot: RecursiveOrchestratorSnapshot,
    r_usd_snapshot: RecursiveOrchestratorSnapshot,
    *,
    level: int = 0,
) -> K4State:
    """从六条边的快照读取 K4 配置状态。

    Parameters
    ----------
    e_au_snapshot : RecursiveOrchestratorSnapshot
        E/Au (SPY/GLD) 比价快照。
    e_r_snapshot : RecursiveOrchestratorSnapshot
        E/R (SPY/TLT) 比价快照。
    e_usd_snapshot : RecursiveOrchestratorSnapshot
        E/$ (SPY) 快照。
    au_r_snapshot : RecursiveOrchestratorSnapshot
        Au/R (GLD/TLT) 比价快照。
    au_usd_snapshot : RecursiveOrchestratorSnapshot
        Au/$ (GLD) 快照。
    r_usd_snapshot : RecursiveOrchestratorSnapshot
        R/$ (TLT) 快照。
    level : int
        读取级别，默认 0。

    Returns
    -------
    K4State
        完整的 K4 配置状态，含六条边的 D 算子读数。
    """
    config = k4_configuration(e_usd_snapshot, au_usd_snapshot, r_usd_snapshot, level=level)
    polarity = polarity_index(config)

    e_au = _build_edge_state(e_au_snapshot, "E/Au", "E", "Au", level)
    e_r = _build_edge_state(e_r_snapshot, "E/R", "E", "R", level)
    e_usd = _build_edge_state(e_usd_snapshot, "E/$", "E", "$", level)
    au_r = _build_edge_state(au_r_snapshot, "Au/R", "Au", "R", level)
    au_usd = _build_edge_state(au_usd_snapshot, "Au/$", "Au", "$", level)
    r_usd = _build_edge_state(r_usd_snapshot, "R/$", "R", "$", level)

    return K4State(
        e_au=e_au,
        e_r=e_r,
        e_usd=e_usd,
        au_r=au_r,
        au_usd=au_usd,
        r_usd=r_usd,
        config=config,
        polarity=polarity,
    )
