"""多 TF pipeline 集成——238号架构接入交易管道。

246号谱系：将 MultiTFOrchestrator 的输出（CrossLevelDivergence, BuySellPoint）
转换为 pipeline.py 可消费的格式（BSP, ResonanceSignal），并用 239号方向性力度
替代振幅力度做跨级别背驰检测。

adapter 模式：不修改 pipeline.py / multi_tf_adapter.py / directional_force.py。
新增适配层完成类型转换和力度替代。

概念溯源:
  - 238号: 多 TF 输入架构（MultiTFOrchestrator）
  - 239号: 方向性力度（∉ ker(D)）
  - 237号: T6 三态诊断（递归深度不足根因）
  - 缠论第27课: 区间套精确大转折点寻找程序定理
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Literal, Sequence

from newchan.a_move_v1 import Move
from newchan.nesting.bsp import BSP, BSPType, DivergenceType
from newchan.nesting.resonance import ResonanceSignal, SignalLayer
from newchan.topology.multi_tf_adapter import (
    BuySellPoint,
    CrossLevelDivergence,
    LevelResult,
    MultiTFOrchestrator,
    MultiTFResult,
    TimeframeLevel,
    _compare_move,
)
from newchan.types import Bar

logger = logging.getLogger(__name__)


# ── 方向性力度跨级别背驰检测 ──────────────────────────────────


def _directional_move_force(move: Move, level_result: LevelResult) -> float:
    """用方向性指标计算走势力度（∉ ker(D)）。

    239号替代 238号的 _move_amplitude（∈ ker(D)）。

    使用两种信号的组合：
    1. 结构复杂度（中枢数量 / 走势段数量）：趋势性走势中枢少、段多
    2. 走势方向持续性：settled move 的方向一致性

    当 snapshot 不可用时回退到 zs_count / seg_span 估计。

    Parameters
    ----------
    move : Move
        走势实例。
    level_result : LevelResult
        该 TF 的完整构造结果（含 snapshot）。

    Returns
    -------
    float
        方向性力度（>= 0）。高值 = 走势方向性强。
    """
    seg_span = move.seg_end - move.seg_start + 1
    if seg_span <= 0:
        return 0.0

    # 组件1：中枢密度的倒数（趋势性 = 中枢少 / 段跨度大）
    zs_count = max(move.zs_count, 1)
    density = zs_count / seg_span
    density_force = 1.0 / density if density > 0 else float(seg_span)

    # 组件2：走势内线段方向一致性（从 snapshot 提取）
    persistence = 0.5  # 默认中性
    if level_result.snapshot is not None:
        segments = level_result.snapshot.seg_snapshot.segments
        if segments and move.seg_start < len(segments):
            end = min(move.seg_end, len(segments) - 1)
            start = move.seg_start
            total = end - start + 1
            if total > 0:
                same_dir = sum(
                    1 for i in range(start, end + 1)
                    if segments[i].direction == move.direction
                )
                persistence = same_dir / total

    return 0.5 * persistence + 0.5 * density_force


def detect_cross_level_divergence_directional(
    high_tf: TimeframeLevel,
    high_result: LevelResult,
    low_tf: TimeframeLevel,
    low_result: LevelResult,
) -> CrossLevelDivergence | None:
    """用方向性力度检测跨级别背驰（替代振幅力度）。

    逻辑与 multi_tf_adapter._detect_cross_level_divergence 相同，
    但力度计算使用 _directional_move_force（∉ ker(D)）
    而非 _move_amplitude（∈ ker(D)）。

    Parameters
    ----------
    high_tf, low_tf : TimeframeLevel
        高/低级别 TF 配置。
    high_result, low_result : LevelResult
        高/低级别 TF 的构造结果。

    Returns
    -------
    CrossLevelDivergence | None
        检测到的背驰（力度字段使用方向性值），或 None。
    """
    high_move = _compare_move(high_result)
    low_move = _compare_move(low_result)
    if high_move is None or low_move is None:
        return None

    if high_move.direction != low_move.direction:
        return None

    force_high = _directional_move_force(high_move, high_result)
    force_low = _directional_move_force(low_move, low_result)

    if force_high <= 0:
        return None

    ratio = force_low / force_high

    if ratio < 1.0:
        div_direction: Literal["top", "bottom"] = (
            "top" if high_move.direction == "up" else "bottom"
        )
        return CrossLevelDivergence(
            high_tf=high_tf,
            low_tf=low_tf,
            direction=div_direction,
            high_move=high_move,
            low_move=low_move,
            force_high=force_high,
            force_low=force_low,
            ratio=ratio,
            confirmed=high_move.settled and low_move.settled,
        )

    return None


# ── 类型转换：MultiTF → pipeline 格式 ─────────────────────────


def buysellpoint_to_bsp(
    bsp: BuySellPoint,
    timestamp: float = 0.0,
) -> BSP:
    """将 MultiTF 的 BuySellPoint 转换为 nesting.bsp.BSP。

    映射规则：
    - kind="type1" + side="buy" → BSPType.B1
    - kind="type1" + side="sell" → BSPType.S1
    - kind="type2" + side="buy" → BSPType.B2 (预留)
    - kind="type2" + side="sell" → BSPType.S2 (预留)
    - kind="type3" + side="buy" → BSPType.B3 (预留)
    - kind="type3" + side="sell" → BSPType.S3 (预留)

    divergence 映射：有 cross_divergence → TOP_DIV/BOT_DIV。

    Parameters
    ----------
    bsp : BuySellPoint
        多 TF 买卖点。
    timestamp : float
        买卖点时刻。

    Returns
    -------
    BSP
        pipeline 格式的买卖点。
    """
    _KIND_SIDE_TO_TYPE: dict[tuple[str, str], BSPType] = {
        ("type1", "buy"): BSPType.B1,
        ("type1", "sell"): BSPType.S1,
        ("type2", "buy"): BSPType.B2,
        ("type2", "sell"): BSPType.S2,
        ("type3", "buy"): BSPType.B3,
        ("type3", "sell"): BSPType.S3,
    }

    bsp_type = _KIND_SIDE_TO_TYPE.get(
        (bsp.kind, bsp.side), BSPType.NONE,
    )

    if bsp.cross_divergence is not None:
        divergence = (
            DivergenceType.TOP_DIV
            if bsp.cross_divergence.direction == "top"
            else DivergenceType.BOT_DIV
        )
    else:
        divergence = DivergenceType.NONE

    return BSP(
        edge_id=f"multi_tf_{bsp.tf.tf_name}",
        level=bsp.tf.level_index,
        time=timestamp,
        bsp_type=bsp_type,
        price=bsp.price,
        divergence=divergence,
    )


def cross_divergence_to_resonance_signal(
    div: CrossLevelDivergence,
    timestamp: float = 0.0,
) -> ResonanceSignal:
    """将 CrossLevelDivergence 转换为 ResonanceSignal。

    跨级别背驰作为独立边信号层接入共振检查。

    映射规则：
    - edge_id: "cross_tf_{high_tf}_{low_tf}"
    - level: high_tf.level_index（高级别支配）
    - layer: INDEPENDENT_EDGE（跨 TF 背驰是独立信号源）
    - bsp: 从背驰方向推导——top→S1，bottom→B1

    Parameters
    ----------
    div : CrossLevelDivergence
        跨级别背驰。
    timestamp : float
        信号时刻。

    Returns
    -------
    ResonanceSignal
        共振信号。
    """
    if div.direction == "top":
        bsp_type = BSPType.S1
        price = div.low_move.high
    else:
        bsp_type = BSPType.B1
        price = div.low_move.low

    bsp = BSP(
        edge_id=f"cross_tf_{div.high_tf.tf_name}_{div.low_tf.tf_name}",
        level=div.high_tf.level_index,
        time=timestamp,
        bsp_type=bsp_type,
        price=price,
        divergence=(
            DivergenceType.TOP_DIV
            if div.direction == "top"
            else DivergenceType.BOT_DIV
        ),
    )

    return ResonanceSignal(
        edge_id=bsp.edge_id,
        level=div.high_tf.level_index,
        bsp=bsp,
        layer=SignalLayer.INDEPENDENT_EDGE,
        time=timestamp,
    )


def multi_tf_result_to_signals(
    result: MultiTFResult,
    timestamp: float = 0.0,
) -> tuple[list[BSP], list[ResonanceSignal]]:
    """将 MultiTFResult 完整转换为 pipeline 可消费的信号。

    Parameters
    ----------
    result : MultiTFResult
        多 TF 编排结果。
    timestamp : float
        当前时刻。

    Returns
    -------
    tuple[list[BSP], list[ResonanceSignal]]
        (买卖点列表, 共振信号列表)。
    """
    bsps = [
        buysellpoint_to_bsp(bp, timestamp)
        for bp in result.buysellpoints
    ]

    resonance_signals = [
        cross_divergence_to_resonance_signal(div, timestamp)
        for div in result.cross_level_divergences
        if div.confirmed
    ]

    return bsps, resonance_signals


# ── 多 TF pipeline 适配器 ───────────────────────────────────


@dataclass(frozen=True, slots=True)
class MultiTFPipelineResult:
    """多 TF pipeline 集成结果。

    Attributes
    ----------
    multi_tf_result : MultiTFResult
        原始多 TF 编排结果。
    bsps : list[BSP]
        转换后的 pipeline 格式买卖点。
    resonance_signals : list[ResonanceSignal]
        转换后的共振信号。
    directional_divergences : list[CrossLevelDivergence]
        方向性力度检测的跨级别背驰（替代振幅力度版本）。
    recursive_levels_equivalent : int
        等效递归深度（= TF 层数，绕过 D2 压缩）。
    t6_reachable : bool
        T6 是否可达（recursive_levels_equivalent >= 2）。
    """

    multi_tf_result: MultiTFResult
    bsps: list[BSP]
    resonance_signals: list[ResonanceSignal]
    directional_divergences: list[CrossLevelDivergence]
    recursive_levels_equivalent: int
    t6_reachable: bool


class MultiTFPipelineAdapter:
    """多 TF pipeline 适配器——238号架构到交易管道的桥梁。

    集成三个组件：
    1. MultiTFOrchestrator（238号）：多 TF 独立构造
    2. 方向性力度（239号）：替代振幅力度的跨级别背驰检测
    3. 类型转换：MultiTF 输出 → pipeline 输入格式

    adapter 模式：不修改现有任何模块。

    Parameters
    ----------
    timeframes : list[TimeframeLevel]
        TF 级别配置列表。
    stroke_mode : str
        笔模式。
    max_levels : int
        每个 TF 内部的最大递归深度。
    use_directional_force : bool
        是否使用方向性力度替代振幅力度。默认 True。
    """

    __slots__ = ("_orchestrator", "_use_directional_force")

    def __init__(
        self,
        timeframes: list[TimeframeLevel],
        stroke_mode: str = "wide",
        max_levels: int = 6,
        use_directional_force: bool = True,
    ) -> None:
        self._orchestrator = MultiTFOrchestrator(
            timeframes=timeframes,
            stroke_mode=stroke_mode,
            max_levels=max_levels,
        )
        self._use_directional_force = use_directional_force

    @property
    def orchestrator(self) -> MultiTFOrchestrator:
        """底层多 TF 编排器。"""
        return self._orchestrator

    @property
    def timeframes(self) -> list[TimeframeLevel]:
        """TF 级别配置列表。"""
        return self._orchestrator.timeframes

    def reset(self) -> None:
        """重置所有 TF 的编排器。"""
        self._orchestrator.reset()

    def _detect_directional_divergences(
        self,
        result: MultiTFResult,
    ) -> list[CrossLevelDivergence]:
        """用方向性力度重新检测跨级别背驰。

        遍历相邻 TF 层级，用 _directional_move_force 替代 _move_amplitude。
        """
        tfs = self._orchestrator.timeframes
        divergences: list[CrossLevelDivergence] = []

        for i in range(len(tfs) - 1):
            high_tf = tfs[i + 1]
            low_tf = tfs[i]
            high_result = result.levels.get(high_tf.tf_name)
            low_result = result.levels.get(low_tf.tf_name)
            if high_result is None or low_result is None:
                continue

            div = detect_cross_level_divergence_directional(
                high_tf, high_result, low_tf, low_result,
            )
            if div is not None:
                divergences.append(div)

        return divergences

    def run(
        self,
        tf_bars: dict[str, list[Bar]],
        timestamp: float = 0.0,
    ) -> MultiTFPipelineResult:
        """运行多 TF 编排并转换为 pipeline 格式。

        步骤：
        1. MultiTFOrchestrator.run — 各 TF 独立构造
        2. 方向性力度跨级别背驰检测（如启用）
        3. 类型转换：BuySellPoint → BSP, CrossLevelDivergence → ResonanceSignal
        4. 计算等效递归深度和 T6 可达性

        Parameters
        ----------
        tf_bars : dict[str, list[Bar]]
            每个 TF 的 bar 数据。
        timestamp : float
            当前时刻。

        Returns
        -------
        MultiTFPipelineResult
            包含原始结果 + pipeline 格式信号 + T6 可达性判断。
        """
        multi_tf_result = self._orchestrator.run(tf_bars)

        # 方向性力度背驰检测
        if self._use_directional_force:
            directional_divs = self._detect_directional_divergences(
                multi_tf_result,
            )
        else:
            directional_divs = list(multi_tf_result.cross_level_divergences)

        # 类型转换
        bsps, resonance_signals = multi_tf_result_to_signals(
            multi_tf_result, timestamp,
        )

        # 等效递归深度 = 有实际数据的 TF 层数
        active_levels = sum(
            1 for lr in multi_tf_result.levels.values()
            if lr.bar_count > 0 and lr.last_move is not None
        )
        t6_reachable = active_levels >= 2

        return MultiTFPipelineResult(
            multi_tf_result=multi_tf_result,
            bsps=bsps,
            resonance_signals=resonance_signals,
            directional_divergences=directional_divs,
            recursive_levels_equivalent=active_levels,
            t6_reachable=t6_reachable,
        )
