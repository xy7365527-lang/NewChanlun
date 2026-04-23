"""多 TF 输入架构 — 绕过 D2 递归压缩瓶颈。

237号诊断结论：T6/T7 低贡献率的结构性根因是 D2（线段引擎）的弱方向
吸收在递归叠加中累积——单 TF 递归架构下 recursive_levels 无法突破 1。

本模块用不同 TF 的 bar 数据分别构建各级别，绕过单 TF 递归中
D2 的压缩率瓶颈。每个 TF 独立运行完整管线（BiEngine → ... → Move），
跨 TF 关联通过时间戳对齐和力度比较实现。

adapter 模式：不修改现有 BiEngine / pipeline / RecursiveOrchestrator。
MultiTFOrchestrator 调用现有引擎多次，每次用不同 TF 数据。

概念溯源:
  - 229号: T6/T7 贡献率分析（递归深度不足实证）
  - 237号: 三态诊断（D2 弱方向吸收根因）
  - 缠论第27课: 区间套精确大转折点寻找程序定理
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Literal

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

logger = logging.getLogger(__name__)


# ── 数据类型 ──────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class TimeframeLevel:
    """一个时间周期级别的配置。

    Attributes
    ----------
    tf_name : str
        时间周期名称（如 "1min", "5min", "30min", "daily", "weekly"）。
    bar_source : str
        数据源标识。
    level_index : int
        级别索引，0 = 最低级。
    """

    tf_name: str
    bar_source: str
    level_index: int


@dataclass(frozen=True, slots=True)
class LevelResult:
    """单个 TF 级别的构造结果。

    Attributes
    ----------
    tf : TimeframeLevel
        该级别的配置。
    snapshot : RecursiveOrchestratorSnapshot | None
        最后一根 bar 处理后的完整快照。无 bar 时为 None。
    bar_count : int
        已处理 bar 数量。
    move_count : int
        L1 走势数量。
    zhongshu_count : int
        L1 中枢数量。
    direction : str
        最新走势方向（"up"/"down"/"none"）。
    last_move : Move | None
        最新的走势实例。
    """

    tf: TimeframeLevel
    snapshot: RecursiveOrchestratorSnapshot | None
    bar_count: int
    move_count: int
    zhongshu_count: int
    direction: Literal["up", "down", "none"]
    last_move: Move | None


@dataclass(frozen=True, slots=True)
class CrossLevelDivergence:
    """跨级别背驰 — T6 的多 TF 替代实现。

    比较相邻 TF 层级的走势方向与力度。当高级别走势方向与低级别
    走势方向一致，但低级别力度衰减时，构成跨级别背驰。

    这绕过了单 TF 递归中 D2 压缩导致的 recursive_levels=1 限制。

    Attributes
    ----------
    high_tf : TimeframeLevel
        高级别 TF。
    low_tf : TimeframeLevel
        低级别 TF。
    direction : str
        背驰方向（"top" = 上涨力竭, "bottom" = 下跌力竭）。
    high_move : Move
        高级别走势实例。
    low_move : Move
        低级别走势实例。
    force_high : float
        高级别力度（价格振幅）。
    force_low : float
        低级别力度（价格振幅）。
    ratio : float
        力度比 = force_low / force_high。< 1.0 时背驰成立。
    confirmed : bool
        是否已确认（两个级别的走势都 settled）。
    """

    high_tf: TimeframeLevel
    low_tf: TimeframeLevel
    direction: Literal["top", "bottom"]
    high_move: Move
    low_move: Move
    force_high: float
    force_low: float
    ratio: float
    confirmed: bool


@dataclass(frozen=True, slots=True)
class BuySellPoint:
    """跨级别买卖点。

    Attributes
    ----------
    kind : str
        买卖点类型（"type1"/"type2"/"type3"）。
    side : str
        方向（"buy"/"sell"）。
    tf : TimeframeLevel
        产生该买卖点的 TF 级别。
    price : float
        买卖点价格。
    cross_divergence : CrossLevelDivergence | None
        关联的跨级别背驰（如有）。
    """

    kind: str
    side: Literal["buy", "sell"]
    tf: TimeframeLevel
    price: float
    cross_divergence: CrossLevelDivergence | None


@dataclass(slots=True)
class MultiTFResult:
    """多 TF 编排的完整结果。

    Attributes
    ----------
    levels : dict[str, LevelResult]
        每个 TF 的独立结果（key = tf_name）。
    cross_level_divergences : list[CrossLevelDivergence]
        跨级别背驰列表。
    buysellpoints : list[BuySellPoint]
        跨级别买卖点列表。
    """

    levels: dict[str, LevelResult] = field(default_factory=dict)
    cross_level_divergences: list[CrossLevelDivergence] = field(
        default_factory=list,
    )
    buysellpoints: list[BuySellPoint] = field(default_factory=list)


# ── 力度计算 ──────────────────────────────────────────────


def _move_amplitude(move: Move) -> float:
    """计算走势的价格振幅力度。

    force = |high - low|
    这是最基本的力度度量，不依赖 MACD。
    """
    return abs(move.high - move.low)


# ── 跨级别背驰检测 ───────────────────────────────────────


def _detect_cross_level_divergence(
    high_tf: TimeframeLevel,
    high_result: LevelResult,
    low_tf: TimeframeLevel,
    low_result: LevelResult,
) -> CrossLevelDivergence | None:
    """检测相邻 TF 层级间的跨级别背驰。

    条件：
    1. 两个级别都有走势（last_move 非 None）
    2. 两个级别走势方向一致
    3. 低级别力度 < 高级别力度（力量衰减）

    Returns
    -------
    CrossLevelDivergence | None
        检测到的背驰，或 None。
    """
    if high_result.last_move is None or low_result.last_move is None:
        return None

    high_move = high_result.last_move
    low_move = low_result.last_move

    # 方向一致才构成背驰条件
    if high_move.direction != low_move.direction:
        return None

    force_high = _move_amplitude(high_move)
    force_low = _move_amplitude(low_move)

    if force_high <= 0:
        return None

    ratio = force_low / force_high

    # 低级别力度 < 高级别力度 → 背驰
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


# ── 跨级别买卖点 ─────────────────────────────────────────


def _derive_buysellpoints(
    divergences: list[CrossLevelDivergence],
) -> list[BuySellPoint]:
    """从跨级别背驰推导买卖点。

    背驰-买卖点定理（缠论）：任一背驰都必然制造某级别买卖点。

    - 顶背驰 → 卖点
    - 底背驰 → 买点

    Returns
    -------
    list[BuySellPoint]
        推导出的买卖点。
    """
    result: list[BuySellPoint] = []
    for div in divergences:
        side: Literal["buy", "sell"] = (
            "sell" if div.direction == "top" else "buy"
        )
        # 价格取低级别走势的端点
        price = (
            div.low_move.high
            if div.direction == "top"
            else div.low_move.low
        )
        result.append(
            BuySellPoint(
                kind="type1",
                side=side,
                tf=div.low_tf,
                price=price,
                cross_divergence=div,
            ),
        )
    return result


# ── 时间戳对齐 ────────────────────────────────────────────


def align_bars_by_timestamp(
    bars: list[Bar],
    reference_start: datetime,
    reference_end: datetime,
) -> list[Bar]:
    """按时间戳范围过滤 bar 数据。

    用于跨 TF 对齐：将低级别 bar 数据限制到高级别走势覆盖的时间范围内。

    Parameters
    ----------
    bars : list[Bar]
        原始 bar 数据。
    reference_start : datetime
        参考时间范围起点。
    reference_end : datetime
        参考时间范围终点。

    Returns
    -------
    list[Bar]
        过滤后的 bar 数据。
    """
    return [
        bar for bar in bars
        if reference_start <= bar.ts <= reference_end
    ]


def extract_c_segment_timestamps(
    move: Move,
    segments: list,
    bars: list[Bar],
) -> tuple[datetime, datetime]:
    """从 Move 提取 C 段时间范围（484号谱系下游推论1-2）。

    映射链：
      Move.seg_start/seg_end
        → segments[seg_*].i0/i1（合并 bar 索引）
        → 重建 merged_to_raw（本地重跑 merge_inclusion）
        → bars[raw_idx].ts

    本地重建 merged_to_raw 是 adapter 层的幂等重建——不修改
    BiEngineSnapshot 的公共接口，仅为时间戳对齐复算一次索引映射。

    Parameters
    ----------
    move : Move
        高级别走势实例（C段 = seg_start..seg_end 范围）。
    segments : list
        高级别 snapshot.seg_snapshot.segments 列表。
    bars : list[Bar]
        高级别原始 bar 列表（与 snapshot 同源）。

    Returns
    -------
    tuple[datetime, datetime]
        (ts_start, ts_end)：C段在原始时间轴上的起止时间戳。

    Raises
    ------
    ValueError
        segments 或 bars 为空；或 seg_start/seg_end 越界。
    IndexError
        合并 bar 索引越出 merged_to_raw 范围。
    """
    if not segments:
        raise ValueError("segments must not be empty")
    if not bars:
        raise ValueError("bars must not be empty")
    if move.seg_start < 0 or move.seg_end < move.seg_start:
        raise ValueError(
            f"invalid seg range: seg_start={move.seg_start}, "
            f"seg_end={move.seg_end}",
        )
    if move.seg_end >= len(segments):
        raise IndexError(
            f"seg_end={move.seg_end} exceeds segments length={len(segments)}",
        )

    # 本地重建 merged_to_raw（209号商空间映射的幂等重跑）
    import pandas as pd
    from newchan.a_inclusion import merge_inclusion

    df_raw = pd.DataFrame(
        {
            "open": [b.open for b in bars],
            "high": [b.high for b in bars],
            "low": [b.low for b in bars],
            "close": [b.close for b in bars],
        },
        index=[b.ts for b in bars],
    )
    _, merged_to_raw = merge_inclusion(df_raw)

    if not merged_to_raw:
        raise ValueError("merge_inclusion returned empty merged_to_raw")

    seg_first = segments[move.seg_start]
    seg_last = segments[move.seg_end]
    n_merged = len(merged_to_raw)
    n_raw = len(bars)

    i0 = max(0, min(seg_first.i0, n_merged - 1))
    i1 = max(0, min(seg_last.i1, n_merged - 1))

    # merged idx → raw idx (取合并区间的逻辑终点，与 ab_bridge 约定一致)
    raw_start = merged_to_raw[i0][0]  # 起点段：取合并块起始
    raw_end = merged_to_raw[i1][1]    # 终点段：取合并块终点
    raw_start = max(0, min(raw_start, n_raw - 1))
    raw_end = max(0, min(raw_end, n_raw - 1))

    return bars[raw_start].ts, bars[raw_end].ts


# ── 核心编排器 ────────────────────────────────────────────


class MultiTFOrchestrator:
    """多 TF 编排器 — 绕过 D2 递归压缩瓶颈。

    每个 TF 独立运行完整管线（BiEngine → SegmentEngine →
    ZhongshuEngine → MoveEngine），跨 TF 通过时间戳对齐和力度比较关联。

    不修改现有 BiEngine / pipeline / RecursiveOrchestrator。

    Parameters
    ----------
    timeframes : list[TimeframeLevel]
        TF 级别配置列表，按 level_index 升序。
    stroke_mode : str
        笔模式（透传到 RecursiveOrchestrator）。
    max_levels : int
        每个 TF 内部的最大递归深度。
    """

    def __init__(
        self,
        timeframes: list[TimeframeLevel],
        stroke_mode: str = "wide",
        max_levels: int = 6,
    ) -> None:
        if not timeframes:
            raise ValueError("timeframes must not be empty")

        # 按 level_index 排序
        self._timeframes = sorted(timeframes, key=lambda tf: tf.level_index)

        # 每个 TF 一个独立的 RecursiveOrchestrator
        self._orchestrators: dict[str, RecursiveOrchestrator] = {}
        for tf in self._timeframes:
            self._orchestrators[tf.tf_name] = RecursiveOrchestrator(
                stream_id=f"multi_tf_{tf.tf_name}",
                max_levels=max_levels,
                stroke_mode=stroke_mode,
            )

    @property
    def timeframes(self) -> list[TimeframeLevel]:
        """TF 级别配置列表（只读）。"""
        return list(self._timeframes)

    def reset(self) -> None:
        """重置所有 TF 的编排器。"""
        for orch in self._orchestrators.values():
            orch.reset()

    def _run_single_tf(
        self,
        tf: TimeframeLevel,
        bars: list[Bar],
    ) -> LevelResult:
        """运行单个 TF 的完整管线。"""
        orch = self._orchestrators[tf.tf_name]
        orch.reset()

        snap: RecursiveOrchestratorSnapshot | None = None
        for bar in bars:
            snap = orch.process_bar(bar)

        if snap is None:
            return LevelResult(
                tf=tf,
                snapshot=None,
                bar_count=0,
                move_count=0,
                zhongshu_count=0,
                direction="none",
                last_move=None,
            )

        moves = snap.move_snapshot.moves
        zhongshus = snap.zs_snapshot.zhongshus

        last_move = moves[-1] if moves else None
        direction: Literal["up", "down", "none"] = (
            last_move.direction if last_move is not None else "none"
        )

        return LevelResult(
            tf=tf,
            snapshot=snap,
            bar_count=len(bars),
            move_count=len(moves),
            zhongshu_count=len(zhongshus),
            direction=direction,
            last_move=last_move,
        )

    def run(self, tf_bars: dict[str, list[Bar]]) -> MultiTFResult:
        """运行多 TF 编排，返回完整结果。

        Parameters
        ----------
        tf_bars : dict[str, list[Bar]]
            每个 TF 的 bar 数据（key = tf_name）。

        Returns
        -------
        MultiTFResult
            包含每个 TF 的独立结果、跨级别背驰、跨级别买卖点。
        """
        # 1. 每个 TF 独立运行管线
        levels: dict[str, LevelResult] = {}
        for tf in self._timeframes:
            bars = tf_bars.get(tf.tf_name, [])
            if not bars:
                logger.warning("No bars for TF %s, skipping", tf.tf_name)
                levels[tf.tf_name] = LevelResult(
                    tf=tf,
                    snapshot=None,
                    bar_count=0,
                    move_count=0,
                    zhongshu_count=0,
                    direction="none",
                    last_move=None,
                )
                continue
            levels[tf.tf_name] = self._run_single_tf(tf, bars)

        # 2. 跨级别背驰检测（相邻 TF 层级两两比较）
        divergences: list[CrossLevelDivergence] = []
        for i in range(len(self._timeframes) - 1):
            high_tf = self._timeframes[i + 1]
            low_tf = self._timeframes[i]
            high_result = levels.get(high_tf.tf_name)
            low_result = levels.get(low_tf.tf_name)
            if high_result is None or low_result is None:
                continue

            div = _detect_cross_level_divergence(
                high_tf, high_result, low_tf, low_result,
            )
            if div is not None:
                divergences.append(div)

        # 3. 从背驰推导买卖点
        buysellpoints = _derive_buysellpoints(divergences)

        return MultiTFResult(
            levels=levels,
            cross_level_divergences=divergences,
            buysellpoints=buysellpoints,
        )
