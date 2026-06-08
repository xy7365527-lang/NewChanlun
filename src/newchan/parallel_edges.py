"""跨边并行执行 + 比价 OHLC 向量化摄入。

存在论位置
----------
K4 / 比价框架在每条"边"（一个比价对，如金油比）上运行一条完整缠论递归管线。
各边之间**无共享状态**（局部依赖原则，275号）——天然可并行。本模块提供：

1. 比价 OHLC 的 numpy 向量化构造（``ratio_frame_numpy``）与向量化摄入
   （``bars_from_frame``：列级 ``to_numpy`` 一次性提取，替代逐行 ``iterrows``）。
2. 跨边 multiprocessing 并行执行 ``RecursiveOrchestrator``（``run_edges_parallel``）。

并行正确性原则
--------------
- worker 内部构建重对象（Bars + orchestrator），跨进程只传 picklable 的
  ``EdgeSpec``（输入数组）与轻量 ``EdgeResult``（摘要）——绝不 pickle 21M
  Bar 或完整 snapshot（pickle 开销会吞掉并行收益）。
- 顺序版与并行版产出逐位一致（见 tests/test_parallel_edges.py）。

认识论等级：L0（向量化是数值恒等变换；并行是执行调度，不改变计算结果）。
"""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from datetime import datetime
from typing import Sequence

import numpy as np
import pandas as pd

from newchan.types import Bar

__all__ = [
    "ratio_frame_numpy",
    "bars_from_frame",
    "EdgeSpec",
    "EdgeResult",
    "run_edge",
    "run_edges_parallel",
    "run_edges_sequential",
]


# ====================================================================
# 比价 OHLC 向量化（numpy）
# ====================================================================


def ratio_frame_numpy(
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
    *,
    columns: Sequence[str] = ("open", "high", "low", "close"),
) -> pd.DataFrame:
    """逐点比价 A/B 的 numpy 向量化构造（朴素逐列除法）。

    与 ``equivalence._make_ratio_kline_naive`` 语义一致（OHLC 四列分别相除，
    volume 取 A），但用 numpy 数组运算替代 pandas Series 逐列对齐，减少中间
    对象分配。两 DataFrame 先按时间戳 inner join 对齐。

    注意：朴素逐列除法的 high/low 不是真实比价极值（需子频率序列才精确，见
    ``equivalence.make_ratio_kline``）。本函数是该朴素路径的向量化等价物，
    用于已对齐同频数据的快速比价。

    认识论等级：L0（A/B 逐元素除法，数值恒等）。
    """
    idx = df_a.index.intersection(df_b.index)
    a = df_a.loc[idx]
    b = df_b.loc[idx]

    out: dict[str, np.ndarray] = {}
    for col in columns:
        out[col] = a[col].to_numpy(dtype=float) / b[col].to_numpy(dtype=float)
    result = pd.DataFrame(out, index=idx)
    if "volume" in a.columns:
        result["volume"] = a["volume"].to_numpy()
    return result


def bars_from_frame(df: pd.DataFrame) -> list[Bar]:
    """向量化 DataFrame→list[Bar]：列级一次性 to_numpy，替代逐行 iterrows。

    OHLC 列与时间索引以 C 级批量提取为 numpy / datetime 数组，再单趟构造
    ``Bar`` 对象（Bar 是 Python frozen dataclass，对象构造本身不可向量化，
    但列提取的向量化消除了逐行 Series 装箱开销）。

    NaN volume → None（保持与现有 driver 的 ``v == v`` 语义一致）。

    认识论等级：L0（纯数据搬运，无领域概念）。
    """
    n = len(df)
    if n == 0:
        return []

    o = df["open"].to_numpy(dtype=float)
    h = df["high"].to_numpy(dtype=float)
    lo = df["low"].to_numpy(dtype=float)
    c = df["close"].to_numpy(dtype=float)

    idx = df.index
    if isinstance(idx, pd.DatetimeIndex):
        ts_arr: Sequence[datetime] = idx.to_pydatetime()
    else:
        ts_arr = list(idx)

    if "volume" in df.columns:
        v = df["volume"].to_numpy(dtype=float)
        bars = [
            Bar(
                ts=ts_arr[i],
                open=float(o[i]), high=float(h[i]),
                low=float(lo[i]), close=float(c[i]),
                volume=None if v[i] != v[i] else float(v[i]),
            )
            for i in range(n)
        ]
    else:
        bars = [
            Bar(
                ts=ts_arr[i],
                open=float(o[i]), high=float(h[i]),
                low=float(lo[i]), close=float(c[i]),
                volume=None,
            )
            for i in range(n)
        ]
    return bars


# ====================================================================
# 跨边并行执行
# ====================================================================


@dataclass(frozen=True, slots=True)
class EdgeSpec:
    """一条边的可 pickle 执行规格。

    持有比价对的两个标的的 OHLCV DataFrame（或已构造好的 ratio_kline）。
    worker 内部构造 Bars + orchestrator，避免跨进程传 Bar/snapshot。

    Attributes
    ----------
    name : str
        边标识（如 "GC/BZ"）。
    df_a, df_b : pd.DataFrame | None
        比价两腿 OHLCV。提供时 worker 内部 ratio_frame_numpy 构造比价。
    ratio_kline : pd.DataFrame | None
        已构造的比价 K 线。提供时直接使用（df_a/df_b 忽略）。
    orchestrator_kwargs : dict
        透传给 RecursiveOrchestrator 的构造参数。
    """

    name: str
    df_a: pd.DataFrame | None = None
    df_b: pd.DataFrame | None = None
    ratio_kline: pd.DataFrame | None = None
    orchestrator_kwargs: dict = field(default_factory=dict)

    def resolve_kline(self) -> pd.DataFrame:
        """得到本边的比价 K 线（向量化构造或直接返回）。"""
        if self.ratio_kline is not None:
            return self.ratio_kline
        if self.df_a is None or self.df_b is None:
            raise ValueError(f"edge {self.name}: 需提供 ratio_kline 或 (df_a, df_b)")
        return ratio_frame_numpy(self.df_a, self.df_b)


@dataclass(frozen=True, slots=True)
class EdgeResult:
    """一条边的轻量执行摘要（picklable，不含 snapshot/Bar）。

    Attributes
    ----------
    name : str
        边标识。
    n_bars : int
        处理的 bar 数。
    n_strokes, n_segments, n_zhongshus, n_moves, n_levels : int
        末态结构计数（来自最后一个 snapshot）。
    elapsed_s : float | None
        worker 内耗时（秒）；None 表示未计时（避免 Date.now 类不可复现问题，
        计时在 worker 进程内用 time.perf_counter，复现性不受主流程影响）。
    error : str | None
        失败原因；非 None 时其余计数为 0。
    """

    name: str
    n_bars: int = 0
    n_strokes: int = 0
    n_segments: int = 0
    n_zhongshus: int = 0
    n_moves: int = 0
    n_levels: int = 0
    elapsed_s: float | None = None
    error: str | None = None


def run_edge(spec: EdgeSpec) -> EdgeResult:
    """在单进程内跑一条边的完整递归管线，返回轻量摘要。

    顶层函数（可 pickle），既用于 multiprocessing worker，也用于顺序执行。
    异常被捕获并编码进 EdgeResult.error——单边失败不影响其余边（局部依赖）。
    """
    import time

    from newchan.orchestrator.recursive import RecursiveOrchestrator

    try:
        kline = spec.resolve_kline()
        bars = bars_from_frame(kline)
        orch = RecursiveOrchestrator(
            stream_id=spec.name, **spec.orchestrator_kwargs
        )
        t0 = time.perf_counter()
        snap = None
        for bar in bars:
            snap = orch.process_bar(bar)
        elapsed = time.perf_counter() - t0

        if snap is None:
            return EdgeResult(name=spec.name, n_bars=0, elapsed_s=elapsed)

        return EdgeResult(
            name=spec.name,
            n_bars=len(bars),
            n_strokes=len(snap.bi_snapshot.strokes),
            n_segments=len(snap.seg_snapshot.segments),
            n_zhongshus=len(snap.zs_snapshot.zhongshus),
            n_moves=len(snap.move_snapshot.moves),
            n_levels=len(snap.recursive_snapshots),
            elapsed_s=elapsed,
        )
    except Exception as exc:  # noqa: BLE001 — 单边失败隔离，原因回传
        return EdgeResult(name=spec.name, error=f"{type(exc).__name__}: {exc}")


def run_edges_sequential(edges: Sequence[EdgeSpec]) -> list[EdgeResult]:
    """顺序执行所有边（基准 / 调试 / 单核环境）。"""
    return [run_edge(e) for e in edges]


def run_edges_parallel(
    edges: Sequence[EdgeSpec],
    *,
    processes: int | None = None,
) -> list[EdgeResult]:
    """跨边 multiprocessing 并行执行，返回与输入同序的结果列表。

    每条边在独立进程中运行完整递归管线（局部依赖：边间无共享状态）。
    结果按输入顺序返回（``Pool.map`` 保序），与 ``run_edges_sequential``
    逐字段一致——并行只改变执行调度，不改变计算结果。

    Parameters
    ----------
    edges : Sequence[EdgeSpec]
        待处理的边。
    processes : int | None
        进程数。默认 min(len(edges), cpu_count)。边数为 1 或 0 时退化为顺序，
        避免进程池开销。

    认识论等级：L0（执行调度，计算结果不变）。
    """
    n = len(edges)
    if n == 0:
        return []
    if n == 1:
        return run_edges_sequential(edges)

    if processes is None:
        processes = min(n, os.cpu_count() or 1)
    processes = max(1, min(processes, n))

    import multiprocessing as mp

    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=processes) as pool:
        return pool.map(run_edge, list(edges))
