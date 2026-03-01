"""K4 配置读取 — 包装 k4_scanner，增加 D 算子读数。

在 topology.k4_scanner 的基础上，为每个标的附加 D 算子读数
（方向态/幅度态/吸收态），构成完整的 K4State。

认识论标注：L0（从定义推导的映射）。
谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from newchan.backtest.types import (
    AssetState,
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


def read_k4_config(
    e_snapshot: RecursiveOrchestratorSnapshot,
    au_snapshot: RecursiveOrchestratorSnapshot,
    r_snapshot: RecursiveOrchestratorSnapshot,
    *,
    level: int = 0,
    e_symbol: str = "SPY",
    au_symbol: str = "GLD",
    r_symbol: str = "TLT",
) -> K4State:
    """从三条比价线的快照读取 K4 配置状态。

    在 k4_scanner.scan_k4() 的基础上，为每个标的附加 D 算子读数。

    Parameters
    ----------
    e_snapshot, au_snapshot, r_snapshot : RecursiveOrchestratorSnapshot
        三条比价线的最新快照。
    level : int
        读取级别，默认 0。
    e_symbol, au_symbol, r_symbol : str
        标的代码。

    Returns
    -------
    K4State
        完整的 K4 配置状态，含 D 算子读数。
    """
    config = k4_configuration(e_snapshot, au_snapshot, r_snapshot, level=level)
    polarity = polarity_index(config)

    e_state = AssetState(
        symbol=e_symbol,
        d_reading=extract_d_reading(e_snapshot),
        walk_direction=walk_direction_from_snapshot(e_snapshot, level),
    )
    au_state = AssetState(
        symbol=au_symbol,
        d_reading=extract_d_reading(au_snapshot),
        walk_direction=walk_direction_from_snapshot(au_snapshot, level),
    )
    r_state = AssetState(
        symbol=r_symbol,
        d_reading=extract_d_reading(r_snapshot),
        walk_direction=walk_direction_from_snapshot(r_snapshot, level),
    )

    return K4State(
        e=e_state,
        au=au_state,
        r=r_state,
        config=config,
        polarity=polarity,
    )
