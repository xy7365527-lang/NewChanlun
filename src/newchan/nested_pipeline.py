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
    """从 K 线序列出发，执行区间套跨级别背驰搜索。

    Parameters
    ----------
    bars : list[Bar]
        K 线序列（按时间正序）。
    df_macd : pd.DataFrame | None
        外部 MACD 数据（如 Alpha Vantage fetch_macd 返回值）。
        列必须包含 ``macd``, ``signal``, ``hist``。
        如果为 None，则从 bars 的 close 价格本地计算。
    macd_fast, macd_slow, macd_signal : int
        本地 MACD 计算参数（仅在 df_macd=None 时使用）。
    stroke_mode, min_strict_sep : ...
        透传到 BiEngine。
    max_levels : int
        最大递归深度。

    Returns
    -------
    tuple[list[NestedDivergence], RecursiveOrchestratorSnapshot | None]
        (嵌套背驰列表, 最后一个 bar 的完整快照)。
        bars 不足时返回 ([], None)。
    """
    if len(bars) < 3:
        return [], None

    # ── 驱动 RecursiveOrchestrator ──
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

    # ── MACD 数据准备 ──
    bar_index = pd.DatetimeIndex([b.ts for b in bars])
    if df_macd is None:
        df_raw = pd.DataFrame(
            [{"close": b.close} for b in bars],
            index=bar_index,
        )
        df_macd = compute_macd(
            df_raw, fast=macd_fast, slow=macd_slow, signal=macd_signal,
        )
    else:
        # 外部 MACD 必须和 bars 在时间轴上对齐；
        # 下游使用 iloc（位置索引），这里统一成 bars 顺序并按缺失补 NaN。
        df_macd = df_macd.reindex(bar_index)

    # ── merged_to_raw 映射 ──
    df_raw = pd.DataFrame({
        "open": [b.open for b in bars],
        "high": [b.high for b in bars],
        "low": [b.low for b in bars],
        "close": [b.close for b in bars],
    }, index=pd.DatetimeIndex([b.ts for b in bars]))
    _, merged_to_raw = merge_inclusion(df_raw)

    # ── 区间套搜索 ──
    results = nested_divergence_search(
        snap, df_macd=df_macd, merged_to_raw=merged_to_raw,
    )

    return results, snap
