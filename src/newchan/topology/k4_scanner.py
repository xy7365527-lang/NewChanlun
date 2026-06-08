"""K4 配置读取器 — 从 RecursiveOrchestratorSnapshot 到 K4 Configuration 的映射。

认识论标注：L0（从配置空间定义直接推导的映射，不依赖数据）

概念溯源：ontology-v2-push.md §三

K4 完全图模型：正典四顶点 M/P/C/R（330号），六条边。
D 算子跑在六条边的比价序列上。

⚠ 消费者层绑定偏离（528号"四套冲突映射"之一，待统一）：本 scanner 的 Configuration
三元组实际绑定为 (P/M, Au/$, R/$)——即 slot σ_C 绑黄金（Au 折叠通道观测量）、
slot σ_R 绑利率，而非正典的 σ_C=C/M(广义商品)、σ_R=R/M(不动产)。正典单一真相源见
topology/data_mapping.py；重写本 scanner 读取单一真相源属 528号下游"复核回测↔实时
可比性"任务，不在 529号结构重构范围内，此处诚实标注不静默。

映射规则：
- trend（趋势）→ 保留 direction（"up" → UP, "down" → DOWN）
- consolidation（盘整）→ FLAT（盘整概念上无方向，第31课）
- moves 为空 → FLAT（保守默认，不猜测方向）
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.a_move_v1 import Move
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.topology.config_space import Configuration, WalkDirection, polarity_index


def _direction_from_move(move: Move) -> WalkDirection:
    """单个 Move → WalkDirection。

    - trend + "up" → UP
    - trend + "down" → DOWN
    - consolidation → FLAT（盘整无概念方向，第31课）
    """
    if move.kind == "consolidation":
        return WalkDirection.FLAT
    if move.direction == "up":
        return WalkDirection.UP
    return WalkDirection.DOWN


def _get_moves_at_level(
    snapshot: RecursiveOrchestratorSnapshot,
    level: int,
) -> list[Move]:
    """从快照中提取指定级别的 moves 列表。

    level=0 → move_snapshot.moves（基础层级）
    level>0 → recursive_snapshots[level-1].moves
    """
    if level == 0:
        return snapshot.move_snapshot.moves
    idx = level - 1
    if idx < len(snapshot.recursive_snapshots):
        return snapshot.recursive_snapshots[idx].moves
    return []


def walk_direction_from_snapshot(
    snapshot: RecursiveOrchestratorSnapshot,
    level: int = 0,
) -> WalkDirection:
    """从 RecursiveOrchestratorSnapshot 读取走势方向。

    逻辑：
    - 读取指定级别的 moves 列表
    - 取最后一个 **settled** move 的方向（与 D 算子读数一致）
    - consolidation → FLAT
    - 无 settled move → FLAT（保守默认）

    统一映射：K4 配置状态直接基于 D 算子的走势方向（settled moves），
    不存在独立的 sigma 映射。config.sigma_p/c/r 和 d_reading.direction
    来自同一数据源。

    Parameters
    ----------
    snapshot : RecursiveOrchestratorSnapshot
        编排器快照。
    level : int
        级别。0 = 基础层级（move_snapshot），>0 = recursive_snapshots[level-1]。

    Returns
    -------
    WalkDirection
    """
    moves = _get_moves_at_level(snapshot, level)
    settled_moves = [m for m in moves if m.settled]
    if not settled_moves:
        return WalkDirection.FLAT
    return _direction_from_move(settled_moves[-1])


def k4_configuration(
    e_usd_snapshot: RecursiveOrchestratorSnapshot,
    au_usd_snapshot: RecursiveOrchestratorSnapshot,
    r_usd_snapshot: RecursiveOrchestratorSnapshot,
    level: int = 0,
) -> Configuration:
    """从三条顶点→现金边（P/M, Au/$, R/$）的快照组合成 K4 Configuration。

    Configuration 是六条边信息的压缩视图，仅使用三条直接对现金的边。

    Parameters
    ----------
    e_usd_snapshot : RecursiveOrchestratorSnapshot
        P/M (SPY) 快照。
    au_usd_snapshot : RecursiveOrchestratorSnapshot
        Au/$ (GLD) 快照。
    r_usd_snapshot : RecursiveOrchestratorSnapshot
        R/$ (TLT) 快照。
    level : int
        级别，默认 0。

    Returns
    -------
    Configuration
    """
    return Configuration(
        sigma_p=walk_direction_from_snapshot(e_usd_snapshot, level),
        sigma_c=walk_direction_from_snapshot(au_usd_snapshot, level),
        sigma_r=walk_direction_from_snapshot(r_usd_snapshot, level),
    )


@dataclass(frozen=True, slots=True)
class K4ScanResult:
    """K4 扫描结果。

    Attributes
    ----------
    config : Configuration
        K4 配置三元组（P/M, Au/$, R/$ 压缩视图）。
    polarity : int
        极性指数 S = sigma_p + sigma_c + sigma_r。
    e_au_direction : WalkDirection
        E/Au 边方向。
    e_r_direction : WalkDirection
        E/R 边方向。
    e_usd_direction : WalkDirection
        P/M 边方向。
    au_r_direction : WalkDirection
        Au/R 边方向。
    au_usd_direction : WalkDirection
        Au/$ 边方向。
    r_usd_direction : WalkDirection
        R/$ 边方向。
    bar_ts : float
        时间戳（取自 e_usd_snapshot）。
    """

    config: Configuration
    polarity: int
    e_au_direction: WalkDirection
    e_r_direction: WalkDirection
    e_usd_direction: WalkDirection
    au_r_direction: WalkDirection
    au_usd_direction: WalkDirection
    r_usd_direction: WalkDirection
    bar_ts: float


def scan_k4(
    e_au_snapshot: RecursiveOrchestratorSnapshot,
    e_r_snapshot: RecursiveOrchestratorSnapshot,
    e_usd_snapshot: RecursiveOrchestratorSnapshot,
    au_r_snapshot: RecursiveOrchestratorSnapshot,
    au_usd_snapshot: RecursiveOrchestratorSnapshot,
    r_usd_snapshot: RecursiveOrchestratorSnapshot,
    level: int = 0,
) -> K4ScanResult:
    """完整的 K4 扫描：从六条边的快照计算配置 + 极性 + 打包结果。

    Configuration 从 P/M, Au/$, R/$ 三条边推导（压缩视图）。
    六条边的完整方向态全部记录在结果中。

    Parameters
    ----------
    e_au_snapshot : RecursiveOrchestratorSnapshot
        E/Au (SPY/GLD) 比价快照。
    e_r_snapshot : RecursiveOrchestratorSnapshot
        E/R (SPY/TLT) 比价快照。
    e_usd_snapshot : RecursiveOrchestratorSnapshot
        P/M (SPY) 快照。
    au_r_snapshot : RecursiveOrchestratorSnapshot
        Au/R (GLD/TLT) 比价快照。
    au_usd_snapshot : RecursiveOrchestratorSnapshot
        Au/$ (GLD) 快照。
    r_usd_snapshot : RecursiveOrchestratorSnapshot
        R/$ (TLT) 快照。
    level : int
        级别，默认 0。

    Returns
    -------
    K4ScanResult
    """
    e_au_dir = walk_direction_from_snapshot(e_au_snapshot, level)
    e_r_dir = walk_direction_from_snapshot(e_r_snapshot, level)
    e_usd_dir = walk_direction_from_snapshot(e_usd_snapshot, level)
    au_r_dir = walk_direction_from_snapshot(au_r_snapshot, level)
    au_usd_dir = walk_direction_from_snapshot(au_usd_snapshot, level)
    r_usd_dir = walk_direction_from_snapshot(r_usd_snapshot, level)

    config = Configuration(sigma_p=e_usd_dir, sigma_c=au_usd_dir, sigma_r=r_usd_dir)
    return K4ScanResult(
        config=config,
        polarity=polarity_index(config),
        e_au_direction=e_au_dir,
        e_r_direction=e_r_dir,
        e_usd_direction=e_usd_dir,
        au_r_direction=au_r_dir,
        au_usd_direction=au_usd_dir,
        r_usd_direction=r_usd_dir,
        bar_ts=e_usd_snapshot.bar_ts,
    )
