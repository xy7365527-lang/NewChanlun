"""比价分析调度器 — 多对比价并行分析。

概念溯源：
  [旧缠论] 第9课 — "比价关系的变动，也可以构成一个买卖系统"
  [旧缠论] 第72/73课 — 比价关系为三个独立系统之一
  [新缠论] 等价关系严格定义 — ratio_relation_v1.md §2

职责：
  接收多个 (EquivalencePair, df_a, df_b) 元组，
  对每对执行 validate → make_ratio_kline → 标准管线（包含处理→分型→笔→线段），
  返回结构化结果。
"""

from __future__ import annotations

from dataclasses import dataclass

import pandas as pd

from newchan.a_fractal import Fractal, fractals_from_merged
from newchan.a_inclusion import merge_inclusion
from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_stroke import Stroke, strokes_from_fractals
from newchan.equivalence import EquivalencePair, make_ratio_kline, validate_pair


@dataclass(frozen=True, slots=True)
class RatioAnalysis:
    """单对比价分析结果。"""

    pair: EquivalencePair
    ratio_kline: pd.DataFrame
    fractals: list[Fractal]
    strokes: list[Stroke]
    segments: list[Segment]


@dataclass(frozen=True, slots=True)
class RatioAnalysisError:
    """分析失败。"""

    pair: EquivalencePair
    reason: str


def _run_pipeline(
    ratio_kline: pd.DataFrame,
) -> tuple[list[Fractal], list[Stroke], list[Segment]]:
    """标准管线：包含处理 → 分型 → 笔 → 线段。"""
    df_merged, _m2r = merge_inclusion(ratio_kline)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(df_merged, fractals)
    segments = segments_from_strokes_v1(strokes)
    return fractals, strokes, segments


def analyze_pair(
    pair: EquivalencePair,
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
) -> RatioAnalysis | RatioAnalysisError:
    """分析单个比价对。

    流程：validate → make_ratio_kline → 标准管线（包含处理→分型→笔→线段）。
    """
    # Step 1: 验证
    validation = validate_pair(df_a, df_b)
    if not validation.valid:
        return RatioAnalysisError(pair=pair, reason=validation.reason)

    # Step 2: 比价K线
    try:
        ratio_kline = make_ratio_kline(df_a, df_b)
    except Exception as exc:
        return RatioAnalysisError(pair=pair, reason=f"make_ratio_kline failed: {exc}")

    if ratio_kline.empty:
        return RatioAnalysisError(pair=pair, reason="Ratio kline is empty")

    # Step 3-6: 标准管线
    try:
        fractals, strokes, segments = _run_pipeline(ratio_kline)
    except Exception as exc:
        return RatioAnalysisError(pair=pair, reason=f"Pipeline failed: {exc}")

    return RatioAnalysis(
        pair=pair,
        ratio_kline=ratio_kline,
        fractals=fractals,
        strokes=strokes,
        segments=segments,
    )


def analyze_batch(
    items: list[tuple[EquivalencePair, pd.DataFrame, pd.DataFrame]],
) -> list[RatioAnalysis | RatioAnalysisError]:
    """批量分析多个比价对。

    对每个元组调用 analyze_pair，保持输入顺序。

    Parameters
    ----------
    items : list[tuple[EquivalencePair, pd.DataFrame, pd.DataFrame]]
        每个元组为 (pair, df_a, df_b)。

    Returns
    -------
    list[RatioAnalysis | RatioAnalysisError]
        与输入等长，顺序对应。
    """
    return [analyze_pair(pair, df_a, df_b) for pair, df_a, df_b in items]


def _analyze_pair_tuple(
    item: tuple[EquivalencePair, pd.DataFrame, pd.DataFrame],
) -> "RatioAnalysis | RatioAnalysisError":
    """顶层 worker 函数（可 pickle）——multiprocessing 入口。"""
    pair, df_a, df_b = item
    return analyze_pair(pair, df_a, df_b)


def analyze_batch_parallel(
    items: list[tuple[EquivalencePair, pd.DataFrame, pd.DataFrame]],
    *,
    processes: int | None = None,
) -> list[RatioAnalysis | RatioAnalysisError]:
    """批量分析的 multiprocessing 并行版——逐字段等价于 analyze_batch。

    各比价对相互独立（局部依赖原则，275号），天然可并行。结果按输入顺序
    返回（Pool.map 保序）。并行只改变执行调度，不改变计算结果。

    Parameters
    ----------
    items : list[tuple[EquivalencePair, pd.DataFrame, pd.DataFrame]]
        每个元组为 (pair, df_a, df_b)。
    processes : int | None
        进程数。默认 min(len(items), cpu_count)。items 数 ≤ 1 时退化为顺序，
        避免进程池开销。

    认识论等级：L0（执行调度，计算结果不变）。
    """
    import os

    n = len(items)
    if n == 0:
        return []
    if n == 1:
        return analyze_batch(items)

    if processes is None:
        processes = min(n, os.cpu_count() or 1)
    processes = max(1, min(processes, n))

    import multiprocessing as mp

    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=processes) as pool:
        return pool.map(_analyze_pair_tuple, items)
