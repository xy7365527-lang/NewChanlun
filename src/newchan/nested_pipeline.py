"""区间套管线 — 从 K 线序列出发，贯通到嵌套背驰搜索。

便利函数：将 RecursiveOrchestrator 逐 bar 驱动后的最终快照
与 MACD 数据连接，执行 nested_divergence_search。

支持两种 MACD 来源：
  1. 本地计算（a_macd.compute_macd）
  2. Alpha Vantage 远程获取（data_av.fetch_macd）

概念溯源：[旧缠论] 第27课 区间套（精确大转折点寻找程序定理）
"""

from __future__ import annotations

import pandas as pd

from newchan.a_inclusion import merge_inclusion
from newchan.a_macd import compute_macd
from newchan.a_nested_divergence import NestedDivergence, nested_divergence_search
from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.types import Bar


def _prepare_macd(
    bars: list[Bar],
    df_macd: pd.DataFrame | None,
    macd_fast: int,
    macd_slow: int,
    macd_signal: int,
) -> pd.DataFrame:
    """准备 MACD 数据：本地计算或对齐外部数据。"""
    bar_index = pd.DatetimeIndex([b.ts for b in bars])
    if df_macd is None:
        df_raw = pd.DataFrame(
            [{"close": b.close} for b in bars],
            index=bar_index,
        )
        return compute_macd(df_raw, fast=macd_fast, slow=macd_slow, signal=macd_signal)
    return df_macd.reindex(bar_index)


def run_nested_search(
    bars: list[Bar],
    *,
    df_macd: pd.DataFrame | None = None,
    macd_fast: int = 12,
    macd_slow: int = 26,
    macd_signal: int = 9,
    stroke_mode: str = "wide",
    min_strict_sep: int = 5,
    max_levels: int = 6,
) -> tuple[list[NestedDivergence], RecursiveOrchestratorSnapshot | None]:
    """从 K 线序列出发，执行区间套跨级别背驰搜索。"""
    if len(bars) < 3:
        return [], None

    orch = RecursiveOrchestrator(
        stream_id="nested_pipeline",
        max_levels=max_levels,
        stroke_mode=stroke_mode,
        min_strict_sep=min_strict_sep,
    )
    snap: RecursiveOrchestratorSnapshot | None = None
    for bar in bars:
        snap = orch.process_bar(bar)

    if snap is None:
        return [], None

    df_macd_final = _prepare_macd(bars, df_macd, macd_fast, macd_slow, macd_signal)

    df_raw = pd.DataFrame({
        "open": [b.open for b in bars],
        "high": [b.high for b in bars],
        "low": [b.low for b in bars],
        "close": [b.close for b in bars],
    }, index=pd.DatetimeIndex([b.ts for b in bars]))
    _, merged_to_raw = merge_inclusion(df_raw)

    results = nested_divergence_search(
        snap, df_macd=df_macd_final, merged_to_raw=merged_to_raw,
    )

    return results, snap
