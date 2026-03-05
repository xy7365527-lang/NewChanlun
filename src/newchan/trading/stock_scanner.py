"""选股扫描器 — 从 K4 配置到候选标的筛选和区间套收敛紧度排序。

**注意**：本模块使用扁平排序（纯按 tightness 降序）。366号 L2 验证确认
商空间排序（T*W）与扁平排序产生显著不同的选股序列，且差异方向与 292号
区间套顺序一致。**新代码应使用 scanner_pool.py（商空间排序）作为默认入口。**
本模块保留供 backtest 管线向后兼容使用。

认识论标注：
- 接口本身：L0（从定义推导）
- target_universe 映射规则：L2（需真实数据验证）
- compute_nesting_tightness 算法：L2（需回测验证）
- 排序方式：L2 已否定为非默认（366号——商空间排序优于扁平排序）
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import TYPE_CHECKING

from newchan.topology.config_space import Configuration, WalkDirection, polarity_index

if TYPE_CHECKING:
    from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot


@dataclass(frozen=True, slots=True)
class ScanCandidate:
    """候选标的。"""

    symbol: str
    nesting_tightness: float
    operation_level: str
    has_buy_point: bool


@dataclass(frozen=True, slots=True)
class ScanResult:
    """扫描结果。"""

    config: Configuration
    polarity: int
    candidates: tuple[ScanCandidate, ...]
    selected: ScanCandidate | None
    scan_ts: float


# ── 目标矩阵 ──────────────────────────────────────────────────

EQUITY_UNIVERSE: tuple[str, ...] = ("XLK", "XLF", "XLE", "XLV", "XLY")
GOLD_UNIVERSE: tuple[str, ...] = ("GLD",)
RATE_UNIVERSE: tuple[str, ...] = ("TLT", "IEF", "SHY")


def target_universe(config: Configuration) -> tuple[str, ...]:
    """根据 K4 配置确定目标矩阵。

    三值逻辑（从267号操作方法论推导）：
    - polarity > 0（risk-on，支持）→ EQUITY_UNIVERSE
    - polarity < 0（risk-off，反对）→ RATE_UNIVERSE
    - polarity == 0（中性）→ EQUITY_UNIVERSE + RATE_UNIVERSE
      中性 ≠ 反对。K4 中性时，层2的缠论结构独立生效——
      有完备买卖点就可以操作。K4 不提供额外筛选。
    - sigma_e.UP 且 sigma_c.UP → 额外加入 GLD（Au 同向）

    认识论等级：L2（需真实数据验证的映射规则）。
    """
    pol = polarity_index(config)

    if pol > 0:
        base = EQUITY_UNIVERSE
    elif pol < 0:
        base = RATE_UNIVERSE
    else:
        base = EQUITY_UNIVERSE + RATE_UNIVERSE

    if config.sigma_e is WalkDirection.UP and config.sigma_c is WalkDirection.UP:
        return base + GOLD_UNIVERSE

    return base


def _has_buy_points_at_level1(snapshot: RecursiveOrchestratorSnapshot) -> bool:
    """level=1 的 bsp_snapshot 中是否有活跃买点。"""
    for bp in snapshot.bsp_snapshot.buysellpoints:
        if bp.side == "buy" and bp.confirmed:
            return True
    return False


def _count_recursive_buy_levels(
    snapshot: RecursiveOrchestratorSnapshot,
) -> tuple[int, int]:
    """统计递归层中有买点的层数，返回 (有买点的层数, 最高有买点的 level_id)。"""
    count = 0
    max_level = 0
    for rs in snapshot.recursive_snapshots:
        has_buy = False
        if hasattr(rs, "buysellpoints"):
            for bp in rs.buysellpoints:
                if bp.side == "buy" and bp.confirmed:
                    has_buy = True
                    break
        if has_buy:
            count += 1
            if rs.level_id > max_level:
                max_level = rs.level_id
    return count, max_level


def compute_nesting_tightness(
    snapshot: RecursiveOrchestratorSnapshot,
) -> tuple[float, str]:
    """计算区间套收敛紧度。

    紧度 = 有买点的层数之和。每层有买点贡献 1.0。
    level=1 通过 bsp_snapshot 检查，level>=2 通过 recursive_snapshots 检查。

    多层买点对齐（同一时间窗口内多级别买点共振）→ 高紧度。
    单层无买点 → 低紧度。

    返回 (tightness, operation_level)。
    operation_level = 有买点的最高级别的字符串表示（"L1", "L2" 等），无买点则为 ""。

    认识论等级：L2（需回测验证）。
    """
    tightness = 0.0
    max_buy_level = 0

    if _has_buy_points_at_level1(snapshot):
        tightness += 1.0
        max_buy_level = 1

    recursive_count, recursive_max = _count_recursive_buy_levels(snapshot)
    tightness += recursive_count

    if recursive_max > max_buy_level:
        max_buy_level = recursive_max

    if max_buy_level == 0:
        return 0.0, ""

    return tightness, f"L{max_buy_level}"


def scan_candidates(
    config: Configuration,
    candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
) -> ScanResult:
    """完整扫描流程。

    1. target_universe(config) → 候选 symbol 集合
    2. 对每个 symbol 在 candidate_snapshots 中查找快照
    3. compute_nesting_tightness → (tightness, level)
    4. 过滤：tightness > 0 且有买点
    5. 排序：tightness 降序
    6. selected = 排第一的候选，或 None
    """
    pol = polarity_index(config)
    universe = target_universe(config)
    ts = time.time()

    if not universe:
        return ScanResult(
            config=config,
            polarity=pol,
            candidates=(),
            selected=None,
            scan_ts=ts,
        )

    raw_candidates: list[ScanCandidate] = []

    for symbol in universe:
        snap = candidate_snapshots.get(symbol)
        if snap is None:
            continue

        tightness, op_level = compute_nesting_tightness(snap)
        has_buy = tightness > 0.0

        if tightness > 0.0 and has_buy:
            raw_candidates.append(
                ScanCandidate(
                    symbol=symbol,
                    nesting_tightness=tightness,
                    operation_level=op_level,
                    has_buy_point=has_buy,
                )
            )

    sorted_candidates = tuple(
        sorted(raw_candidates, key=lambda c: c.nesting_tightness, reverse=True)
    )

    selected = sorted_candidates[0] if sorted_candidates else None

    return ScanResult(
        config=config,
        polarity=pol,
        candidates=sorted_candidates,
        selected=selected,
        scan_ts=ts,
    )
