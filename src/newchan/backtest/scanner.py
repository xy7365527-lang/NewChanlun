"""选股扫描 — 包装 stock_scanner，在回测框架中使用。

接收 K4State，确定目标矩阵，在候选标的上跑 RecursiveOrchestrator，
计算区间套收敛紧度，输出收敛最紧的标的。

认识论标注：
  - 接口封装：L0
  - 选股规则（target_universe, nesting tightness）：L2（需真实数据验证）

谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from newchan.backtest.types import K4State, ScannerResult, extract_d_reading
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.trading.stock_scanner import (
    ScanCandidate,
    ScanResult,
    compute_nesting_tightness,
    scan_candidates,
    target_universe,
)


# 行业 ETF 候选列表
SECTOR_ETFS: tuple[str, ...] = (
    "XLK", "XLF", "XLE", "XLV", "XLY",
    "XLI", "XLB", "XLP", "XLU", "XLRE", "XLC",
)


def scan_stocks(
    k4_state: K4State,
    candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
) -> ScannerResult:
    """选股扫描。

    1. 调用 stock_scanner.scan_candidates() 完成实际扫描
    2. 包装结果为 ScannerResult

    Parameters
    ----------
    k4_state : K4State
        当前 K4 配置状态。
    candidate_snapshots : dict[str, RecursiveOrchestratorSnapshot]
        候选标的的最新快照。

    Returns
    -------
    ScannerResult
        扫描结果。
    """
    result = scan_candidates(k4_state.config, candidate_snapshots)

    if result.selected is not None:
        return ScannerResult(
            selected_symbol=result.selected.symbol,
            tightness=result.selected.nesting_tightness,
            candidates_count=len(result.candidates),
        )

    return ScannerResult(
        selected_symbol=None,
        tightness=0.0,
        candidates_count=len(result.candidates),
    )


def rank_by_tightness(
    candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
) -> list[tuple[str, float, str]]:
    """对所有候选标的按区间套收敛紧度排序。

    返回 [(symbol, tightness, operation_level)]，按 tightness 降序。
    """
    results: list[tuple[str, float, str]] = []

    for symbol, snap in candidate_snapshots.items():
        tightness, op_level = compute_nesting_tightness(snap)
        results.append((symbol, tightness, op_level))

    return sorted(results, key=lambda x: x[1], reverse=True)
