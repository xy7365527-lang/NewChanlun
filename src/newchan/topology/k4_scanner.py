"""K4 配置读取器 — 从 RecursiveOrchestratorSnapshot 到 K4 Configuration 的映射。

认识论标注：L0（从配置空间定义直接推导的映射，不依赖数据）

概念溯源：ontology-v2-push.md §三
- 三条比价线 E/$=SPY, Au/$=GLD, R/$=TLT
- 走势方向 → WalkDirection → Configuration 三元组
- Configuration → polarity_index

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
    - 取最后一个 move 的方向
    - consolidation → FLAT
    - moves 为空 → FLAT（保守默认）

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
    if not moves:
        return WalkDirection.FLAT
    return _direction_from_move(moves[-1])


def k4_configuration(
    e_snapshot: RecursiveOrchestratorSnapshot,
    au_snapshot: RecursiveOrchestratorSnapshot,
    r_snapshot: RecursiveOrchestratorSnapshot,
    level: int = 0,
) -> Configuration:
    """从三条比价线的快照组合成 K4 Configuration。

    Parameters
    ----------
    e_snapshot : RecursiveOrchestratorSnapshot
        E/$ (SPY) 快照。
    au_snapshot : RecursiveOrchestratorSnapshot
        Au/$ (GLD) 快照。
    r_snapshot : RecursiveOrchestratorSnapshot
        R/$ (TLT) 快照。
    level : int
        级别，默认 0。

    Returns
    -------
    Configuration
    """
    return Configuration(
        sigma_e=walk_direction_from_snapshot(e_snapshot, level),
        sigma_c=walk_direction_from_snapshot(au_snapshot, level),
        sigma_r=walk_direction_from_snapshot(r_snapshot, level),
    )


@dataclass(frozen=True, slots=True)
class K4ScanResult:
    """K4 扫描结果。

    Attributes
    ----------
    config : Configuration
        K4 配置三元组。
    polarity : int
        极性指数 S = sigma_e + sigma_c + sigma_r。
    e_direction : WalkDirection
        E/$ 方向。
    au_direction : WalkDirection
        Au/$ 方向。
    r_direction : WalkDirection
        R/$ 方向。
    bar_ts : float
        时间戳（取自 e_snapshot）。
    """

    config: Configuration
    polarity: int
    e_direction: WalkDirection
    au_direction: WalkDirection
    r_direction: WalkDirection
    bar_ts: float


def scan_k4(
    e_snapshot: RecursiveOrchestratorSnapshot,
    au_snapshot: RecursiveOrchestratorSnapshot,
    r_snapshot: RecursiveOrchestratorSnapshot,
    level: int = 0,
) -> K4ScanResult:
    """完整的 K4 扫描：计算配置 + 极性 + 打包结果。

    Parameters
    ----------
    e_snapshot : RecursiveOrchestratorSnapshot
        E/$ (SPY) 快照。
    au_snapshot : RecursiveOrchestratorSnapshot
        Au/$ (GLD) 快照。
    r_snapshot : RecursiveOrchestratorSnapshot
        R/$ (TLT) 快照。
    level : int
        级别，默认 0。

    Returns
    -------
    K4ScanResult
    """
    e_dir = walk_direction_from_snapshot(e_snapshot, level)
    au_dir = walk_direction_from_snapshot(au_snapshot, level)
    r_dir = walk_direction_from_snapshot(r_snapshot, level)
    config = Configuration(sigma_e=e_dir, sigma_c=au_dir, sigma_r=r_dir)
    return K4ScanResult(
        config=config,
        polarity=polarity_index(config),
        e_direction=e_dir,
        au_direction=au_dir,
        r_direction=r_dir,
        bar_ts=e_snapshot.bar_ts,
    )
