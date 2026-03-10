#!/usr/bin/env python3
"""布伦特原油(BRN)级别分析 — 多数据源获取 K 线，RecursiveOrchestrator 计算全级别结构。

数据源：
  - databento: ICE Brent Crude (IFEU.IMPACT / BRN)，支持 1 分钟级别，需 DATABENTO_API_KEY
  - yfinance: BZ=F，免费但 1 分钟数据仅 7 天

用法：
    python scripts/brn_level_analysis.py --source databento --interval 1m --start 2020-01-01
    python scripts/brn_level_analysis.py --source yfinance --interval 1h --period 2y
"""
from __future__ import annotations

import os
import sys
from datetime import datetime, date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import yfinance as yf
import pandas as pd

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

# batch mode imports (top-level to fail fast if missing)
import numpy as np
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_zhongshu_v1 import zhongshu_from_segments
from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_macd import compute_macd
from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_level_protocol import adapt_moves, MoveAsComponent
from newchan.a_zhongshu_level import LevelZhongshu, zhongshu_from_components, moves_from_level_zhongshus
from newchan.bi_engine import BiEngineSnapshot
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.buysellpoint_state import BuySellPointSnapshot
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot


# ── yfinance → Bar 适配 ──


def fetch_bars_yf(
    ticker: str = "BZ=F",
    interval: str = "1h",
    period: str = "2y",
) -> list[Bar]:
    """从 yfinance 获取 K 线并转为 Bar 列表。

    Parameters
    ----------
    ticker : yfinance ticker（BZ=F = 布伦特原油期货）
    interval : K线周期 — 1m/2m/5m/15m/30m/60m/90m/1h/1d/5d/1wk/1mo/3mo
    period : 回溯期 — 1d/5d/1mo/3mo/6mo/1y/2y/5y/10y/ytd/max
    """
    t = yf.Ticker(ticker)
    df = t.history(period=period, interval=interval)
    if df.empty:
        raise ValueError(f"yfinance 返回空数据: ticker={ticker}, interval={interval}, period={period}")

    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts,
            open=float(row["Open"]),
            high=float(row["High"]),
            low=float(row["Low"]),
            close=float(row["Close"]),
            volume=float(row["Volume"]) if pd.notna(row.get("Volume")) else None,
        ))
    return bars


# ── Databento → Bar 适配 ──


def _load_databento_key() -> str:
    """从环境变量或 .env 文件加载 DATABENTO_API_KEY。"""
    key = os.environ.get("DATABENTO_API_KEY")
    if key:
        return key

    env_path = Path(__file__).resolve().parent.parent / ".env"
    if env_path.exists():
        for line in env_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("DATABENTO_API_KEY=") and not line.startswith("#"):
                key = line.split("=", 1)[1].strip()
                if key:
                    return key

    raise ValueError(
        "DATABENTO_API_KEY 未设置。请设置环境变量或在 .env 文件中配置。"
    )


def fetch_bars_databento(
    symbol: str = "BRN.c.0",
    dataset: str = "IFEU.IMPACT",
    schema: str = "ohlcv-1m",
    start: str = "2020-01-01",
    end: str | None = None,
) -> list[Bar]:
    """从 Databento Historical API 获取 K 线并转为 Bar 列表。

    Parameters
    ----------
    symbol : Databento 连续合约符号（BRN.c.0 = 布伦特原油期货前月连续）
    dataset : Databento 数据集（IFEU.IMPACT = ICE Futures Europe）
    schema : K 线 schema — ohlcv-1m / ohlcv-1h / ohlcv-1d
    start : 起始日期（ISO 格式）
    end : 结束日期（ISO 格式），None 表示至今
    """
    import databento as db

    key = _load_databento_key()
    client = db.Historical(key=key)

    cache_dir = Path(__file__).resolve().parent.parent / "tmp" / "databento_cache"
    cache_dir.mkdir(parents=True, exist_ok=True)

    # 按年分段下载，避免单次请求过大
    from datetime import date as date_cls
    start_date = date_cls.fromisoformat(start)
    end_date = date_cls.fromisoformat(end) if end else date_cls.today()

    all_dfs = []
    year = start_date.year
    while True:
        seg_start = max(start_date, date_cls(year, 1, 1))
        seg_end = min(end_date, date_cls(year, 12, 31))
        if seg_start > end_date:
            break

        seg_start_str = seg_start.isoformat()
        seg_end_str = seg_end.isoformat()
        cache_file = cache_dir / f"{dataset}_{symbol}_{schema}_{seg_start_str}_{seg_end_str}.dbn.zst"

        if cache_file.exists() and cache_file.stat().st_size > 100:
            print(f"  [{year}] 使用缓存: {cache_file.name}", flush=True)
            data = db.DBNStore.from_file(str(cache_file))
        else:
            print(f"  [{year}] 下载 {seg_start_str} ~ {seg_end_str} ...", flush=True)
            if cache_file.exists():
                cache_file.unlink()
            data = client.timeseries.get_range(
                dataset=dataset,
                symbols=symbol,
                schema=schema,
                stype_in="continuous",
                start=seg_start_str,
                end=seg_end_str,
                path=str(cache_file),
            )

        df = data.to_df()
        if not df.empty:
            all_dfs.append(df)
            print(f"  [{year}] {len(df)} 行", flush=True)
        else:
            print(f"  [{year}] 无数据", flush=True)

        year += 1

    if not all_dfs:
        raise ValueError(
            f"Databento 返回空数据: dataset={dataset}, symbol={symbol}, schema={schema}"
        )

    combined = pd.concat(all_dfs).sort_index()
    combined = combined[~combined.index.duplicated(keep='first')]
    print(f"  合计 {len(combined)} 行1分钟数据", flush=True)

    bars: list[Bar] = []
    for ts, row in combined.iterrows():
        ts_dt = ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts
        if hasattr(ts_dt, "tzinfo") and ts_dt.tzinfo is not None:
            ts_dt = ts_dt.replace(tzinfo=None)
        o = float(row["open"])
        h = float(row["high"])
        l = float(row["low"])
        c = float(row["close"])
        # databento fixed-point: 价格 > 1e6 说明是 fixed-point 格式
        if o > 1e6:
            o, h, l, c = o / 1e9, h / 1e9, l / 1e9, c / 1e9
        bars.append(Bar(
            ts=ts_dt,
            open=o,
            high=h,
            low=l,
            close=c,
            volume=int(row["volume"]) if pd.notna(row.get("volume")) else None,
        ))
    return bars


# ── 级别分析 ──


def analyze_levels(bars: list[Bar], stream_id: str = "BRN") -> dict:
    """运行 RecursiveOrchestrator 并提取全级别结构。"""
    orch = RecursiveOrchestrator(stream_id=stream_id)

    # 逐 bar 处理
    snap = None
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        if i % 100000 == 0:
            print(f"  已处理 {i}/{len(bars)} 根 K 线...", flush=True)

    if snap is None:
        return {"error": "no bars processed"}

    result = {
        "ticker": stream_id,
        "bar_count": len(bars),
        "bar_range": f"{bars[0].ts} ~ {bars[-1].ts}",
        "last_close": bars[-1].close,
        "levels": {},
    }

    # Level-1: 笔/线段/中枢/走势/买卖点
    l1 = {
        "strokes": len(snap.bi_snapshot.strokes),
        "segments": len(snap.seg_snapshot.segments),
        "zhongshus": [],
        "moves": [],
        "buysellpoints": [],
    }

    for zs in snap.zs_snapshot.zhongshus:
        l1["zhongshus"].append({
            "zd": round(zs.zd, 2),
            "zg": round(zs.zg, 2),
            "seg_count": zs.seg_count,
            "settled": zs.settled,
            "break_direction": getattr(zs, "break_direction", ""),
        })

    for mv in snap.move_snapshot.moves:
        l1["moves"].append({
            "direction": mv.direction,
            "kind": mv.kind,
            "settled": mv.settled,
            "zhongshu_count": mv.zs_count,
        })

    for bsp in snap.bsp_snapshot.buysellpoints:
        l1["buysellpoints"].append({
            "kind": bsp.kind,
            "side": bsp.side,
            "seg_idx": bsp.seg_idx,
            "level_id": bsp.level_id,
        })

    result["levels"]["L1"] = l1

    # Level-2+ 递归层 (streaming mode does not compute recursive BSPs)
    for rsnap in snap.recursive_snapshots:
        lvl_key = f"L{rsnap.level_id}"
        lvl = {
            "zhongshus": [],
            "moves": [],
            "buysellpoints": [],
        }
        for zs in rsnap.zhongshus:
            lvl["zhongshus"].append({
                "zd": round(zs.zd, 2),
                "zg": round(zs.zg, 2),
                "comp_count": zs.comp_count,
                "settled": zs.settled,
            })
        for mv in rsnap.moves:
            lvl["moves"].append({
                "direction": mv.direction,
                "kind": mv.kind,
                "settled": mv.settled,
                "zhongshu_count": mv.zs_count,
            })
        result["levels"][lvl_key] = lvl

    return result


def _dt_to_epoch(dt: datetime) -> float:
    """datetime -> epoch seconds. Naive datetime treated as UTC."""
    from datetime import timezone
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()


class _ComponentAsSegment:
    """Adapt MoveAsComponent to the segment-like interface needed by divergence functions.

    Provides i0, i1, high, low, direction attributes so that
    divergences_from_moves_v1 can compute force via MACD (when merged bar indices
    are provided) or price-amplitude fallback.

    When merged_i0/merged_i1 are provided, they point into the L1 merged bar
    index space, allowing _compute_force to use merged_to_raw → df_macd correctly.
    """

    __slots__ = ("i0", "i1", "high", "low", "direction")

    def __init__(
        self,
        comp: MoveAsComponent,
        merged_i0: int | None = None,
        merged_i1: int | None = None,
    ) -> None:
        self.i0 = merged_i0 if merged_i0 is not None else comp.component_idx
        self.i1 = merged_i1 if merged_i1 is not None else comp.component_idx
        self.high = comp.high
        self.low = comp.low
        self.direction = comp.direction


class _LevelZhongshuAsZhongshu:
    """Adapt LevelZhongshu to the Zhongshu-like interface needed by divergence/buysellpoint functions.

    Maps comp_start/comp_end/comp_count/break_comp to seg_start/seg_end/seg_count/break_seg.
    """

    __slots__ = (
        "zd", "zg", "seg_start", "seg_end", "seg_count", "settled",
        "break_seg", "break_direction", "gg", "dd",
    )

    def __init__(self, lzs: LevelZhongshu) -> None:
        self.zd = lzs.zd
        self.zg = lzs.zg
        self.seg_start = lzs.comp_start
        self.seg_end = lzs.comp_end
        self.seg_count = lzs.comp_count
        self.settled = lzs.settled
        self.break_seg = lzs.break_comp
        self.break_direction = lzs.break_direction
        self.gg = lzs.gg
        self.dd = lzs.dd


def _recursive_level_bsp(
    components: list[MoveAsComponent],
    level_zhongshus: list[LevelZhongshu],
    level_moves: list,
    level_id: int,
    comp_merged_ranges: list[tuple[int, int]] | None = None,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> list:
    """Compute divergences and buysellpoints for a recursive level.

    Adapts LevelZhongshu → Zhongshu-like and MoveAsComponent → segment-like.
    When comp_merged_ranges, df_macd, and merged_to_raw are provided, MACD
    three-dimensional divergence detection is used (same as L1). Otherwise
    falls back to price-amplitude.

    Parameters
    ----------
    comp_merged_ranges : list[tuple[int, int]] | None
        For each component (by component_idx), the (merged_i0, merged_i1)
        range in L1 merged bar index space.
    df_macd : pd.DataFrame | None
        MACD computed on raw bars.
    merged_to_raw : list[tuple[int, int]] | None
        L1 merged → raw bar index mapping.
    """
    if comp_merged_ranges is not None:
        seg_like = [
            _ComponentAsSegment(c, *comp_merged_ranges[c.component_idx])
            for c in components
        ]
    else:
        seg_like = [_ComponentAsSegment(c) for c in components]
    zs_like = [_LevelZhongshuAsZhongshu(lzs) for lzs in level_zhongshus]

    divs = divergences_from_moves_v1(
        seg_like, zs_like, level_moves, level_id,
        df_macd=df_macd, merged_to_raw=merged_to_raw,
    )
    bsps = buysellpoints_from_level(seg_like, zs_like, level_moves, divs, level_id)
    return bsps


def _build_comp_merged_ranges(
    components: list[MoveAsComponent],
    l1_segments: list,
    prev_comp_merged_ranges: list[tuple[int, int]] | None,
) -> list[tuple[int, int]]:
    """Build per-component (merged_i0, merged_i1) mapping for MACD lookups.

    For L2 components (prev_comp_merged_ranges is None):
        Each component wraps an L1 Move whose seg_start/seg_end index into
        l1_segments. We read segments[seg_start].i0 and segments[seg_end].i1
        to get merged bar indices.

    For L3+ components (prev_comp_merged_ranges is provided):
        Each component wraps a higher-level Move whose seg_start/seg_end are
        component_idx values in the previous level's components list. We look
        up those indices in prev_comp_merged_ranges and take the union range.
    """
    result: list[tuple[int, int]] = []
    for comp in components:
        move = comp._move
        if prev_comp_merged_ranges is None:
            # L2: Move.seg_start/seg_end → L1 segments → merged bar indices
            seg_start = min(move.seg_start, len(l1_segments) - 1)
            seg_end = min(move.seg_end, len(l1_segments) - 1)
            merged_i0 = l1_segments[seg_start].i0
            merged_i1 = l1_segments[seg_end].i1
        else:
            # L3+: Move.seg_start/seg_end → previous level component indices
            idx_start = min(move.seg_start, len(prev_comp_merged_ranges) - 1)
            idx_end = min(move.seg_end, len(prev_comp_merged_ranges) - 1)
            merged_i0 = prev_comp_merged_ranges[idx_start][0]
            merged_i1 = prev_comp_merged_ranges[idx_end][1]
        result.append((merged_i0, merged_i1))
    return result


def _recursive_levels(
    move_snap: MoveSnapshot,
    *,
    max_levels: int = 6,
    l1_segments: list | None = None,
    df_macd: pd.DataFrame | None = None,
    merged_to_raw: list[tuple[int, int]] | None = None,
) -> tuple[list[RecursiveLevelSnapshot], dict[int, list]]:
    """Run recursive stack logic without engine state — pure function chain.

    Returns (snapshots, bsp_by_level) where bsp_by_level maps level_id to buysellpoints.

    When l1_segments, df_macd, and merged_to_raw are all provided, MACD
    three-dimensional divergence detection is used for all recursive levels.
    """
    snapshots: list[RecursiveLevelSnapshot] = []
    bsp_by_level: dict[int, list] = {}
    current_move_snap = move_snap
    current_level = 1
    use_macd = l1_segments is not None and df_macd is not None and merged_to_raw is not None
    prev_comp_merged_ranges: list[tuple[int, int]] | None = None

    while current_level < max_levels:
        next_level = current_level + 1
        settled_moves = [m for m in current_move_snap.moves if m.settled]
        components = adapt_moves(settled_moves, level_id=current_level)
        curr_zhongshus = zhongshu_from_components(components)
        curr_moves = moves_from_level_zhongshus(curr_zhongshus)

        # Build merged bar ranges for this level's components
        comp_merged_ranges: list[tuple[int, int]] | None = None
        if use_macd and len(components) > 0:
            comp_merged_ranges = _build_comp_merged_ranges(
                components, l1_segments,
                prev_comp_merged_ranges if current_level > 1 else None,
            )

        # Divergences + buysellpoints for this recursive level
        bsps = _recursive_level_bsp(
            components, curr_zhongshus, curr_moves, next_level,
            comp_merged_ranges=comp_merged_ranges,
            df_macd=df_macd if use_macd else None,
            merged_to_raw=merged_to_raw if use_macd else None,
        )
        bsp_by_level[next_level] = bsps

        # Batch mode: no incremental diff — events are intentionally empty
        # (streaming mode populates these via diff_level_zhongshu / diff_level_moves)
        snap = RecursiveLevelSnapshot(
            bar_idx=current_move_snap.bar_idx,
            bar_ts=current_move_snap.bar_ts,
            level_id=next_level,
            zhongshus=curr_zhongshus,
            moves=curr_moves,
            zhongshu_events=[],
            move_events=[],
        )
        snapshots.append(snap)

        if len(curr_moves) < 3:
            break

        # Pass current level's merged ranges to next level
        prev_comp_merged_ranges = comp_merged_ranges

        # Batch mode: no incremental diff events (same rationale as zhongshu/move_events above)
        current_move_snap = MoveSnapshot(
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            moves=curr_moves,
            events=[],
        )
        current_level = next_level

    return snapshots, bsp_by_level


def analyze_levels_batch(
    bars: list[Bar],
    stream_id: str = "BRN",
    *,
    stroke_mode: str = "wide",
    min_strict_sep: int = 5,
    max_levels: int = 6,
) -> dict:
    """Batch mode: O(n) single-pass analysis — no per-bar rebuild.

    Builds the DataFrame once from all bars, runs each pure function
    layer exactly once, and constructs the final result dict.

    Parameters
    ----------
    stroke_mode : Stroke construction mode passed to strokes_from_fractals.
    min_strict_sep : Minimum strict separation for stroke construction.
    max_levels : Maximum number of recursive levels to compute.
    """
    if not bars:
        return {"error": "no bars"}

    # 1. Build DataFrame once
    print("  [batch] 构建 DataFrame...", flush=True)
    arr = np.array(
        [[b.open, b.high, b.low, b.close] for b in bars],
        dtype=np.float64,
    )
    df = pd.DataFrame(
        arr,
        columns=["open", "high", "low", "close"],
        index=pd.DatetimeIndex([b.ts for b in bars], name="time"),
    )

    # Validate time series ordering
    if not df.index.is_monotonic_increasing:
        raise ValueError("bars 时间序列非单调递增 — 请检查输入数据排序")
    if not df.index.is_unique:
        raise ValueError("bars 存在重复时间戳 — 请检查输入数据去重")

    # 1b. Compute MACD on raw bars (L1 only — recursive levels lack raw K-lines)
    print("  [batch] 计算 MACD...", flush=True)
    df_macd = compute_macd(df)

    # 2. Inclusion → Fractals → Strokes (once)
    print("  [batch] 包含处理...", flush=True)
    df_merged, merged_to_raw = merge_inclusion(df)
    print(f"  [batch] 合并K线: {len(df_merged)}", flush=True)

    print("  [batch] 分型检测...", flush=True)
    fractals = fractals_from_merged(df_merged)
    print(f"  [batch] 分型: {len(fractals)}", flush=True)

    print("  [batch] 笔构造...", flush=True)
    all_strokes = strokes_from_fractals(
        df_merged, fractals, mode=stroke_mode, min_strict_sep=min_strict_sep,
        merged_to_raw=merged_to_raw,
    )
    print(f"  [batch] 笔: {len(all_strokes)}", flush=True)

    # 3. Segments (once)
    print("  [batch] 线段构造...", flush=True)
    segments = segments_from_strokes_v1(all_strokes)
    print(f"  [batch] 线段: {len(segments)}", flush=True)

    # 4. Zhongshu (once)
    print("  [batch] 中枢检测...", flush=True)
    zhongshus = zhongshu_from_segments(segments)
    print(f"  [batch] 中枢: {len(zhongshus)}", flush=True)

    # 5. Moves (once)
    print("  [batch] 走势分组...", flush=True)
    moves = moves_from_zhongshus(zhongshus, num_segments=len(segments))
    print(f"  [batch] 走势: {len(moves)}", flush=True)

    # 6. Buy/sell points (once)
    print("  [batch] 背驰+买卖点...", flush=True)
    divergences = divergences_from_moves_v1(
        segments, zhongshus, moves, 1,
        df_macd=df_macd, merged_to_raw=merged_to_raw,
    )
    bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
    print(f"  [batch] 买卖点: {len(bsps)}", flush=True)

    # 7. Construct snapshots for recursive stack
    last_bar = bars[-1]
    bar_idx = len(bars) - 1
    bar_ts = _dt_to_epoch(last_bar.ts)

    # Batch mode: no incremental diff events (batch computes full state in one pass)
    move_snap = MoveSnapshot(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        moves=moves,
        events=[],
    )

    # 8. Recursive levels (once)
    print("  [batch] 递归级别...", flush=True)
    recursive_snaps, recursive_bsps = _recursive_levels(
        move_snap,
        max_levels=max_levels,
        l1_segments=segments,
        df_macd=df_macd,
        merged_to_raw=merged_to_raw,
    )
    print(f"  [batch] 递归层数: {len(recursive_snaps)}", flush=True)
    for lvl_id, lvl_bsps in recursive_bsps.items():
        if lvl_bsps:
            print(f"  [batch] L{lvl_id} 买卖点: {len(lvl_bsps)}", flush=True)

    # 9. L* selection
    bi_snap = BiEngineSnapshot(
        bar_idx=bar_idx, bar_ts=bar_ts, strokes=all_strokes,
        events=[], n_merged=len(df_merged), n_fractals=len(fractals),
    )
    seg_snap = SegmentSnapshot(
        bar_idx=bar_idx, bar_ts=bar_ts, segments=segments, events=[],
    )
    zs_snap = ZhongshuSnapshot(
        bar_idx=bar_idx, bar_ts=bar_ts, zhongshus=zhongshus, events=[],
    )
    bsp_snap = BuySellPointSnapshot(
        bar_idx=bar_idx, bar_ts=bar_ts, buysellpoints=bsps, events=[],
    )
    orch_snap = RecursiveOrchestratorSnapshot(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        bi_snapshot=bi_snap,
        seg_snapshot=seg_snap,
        zs_snapshot=zs_snap,
        move_snapshot=move_snap,
        bsp_snapshot=bsp_snap,
        recursive_snapshots=recursive_snaps,
        all_events=[],
        lstar=None,
    )

    from newchan.a_level_fsm_adapter import select_lstar_from_recursive_snapshot
    orch_snap.lstar = select_lstar_from_recursive_snapshot(orch_snap, last_bar.close)

    # 10. Build result dict (same format as analyze_levels)
    result = {
        "ticker": stream_id,
        "bar_count": len(bars),
        "bar_range": f"{bars[0].ts} ~ {bars[-1].ts}",
        "last_close": bars[-1].close,
        "levels": {},
    }

    l1 = {
        "strokes": len(all_strokes),
        "segments": len(segments),
        "zhongshus": [],
        "moves": [],
        "buysellpoints": [],
    }
    for zs in zhongshus:
        l1["zhongshus"].append({
            "zd": round(zs.zd, 2),
            "zg": round(zs.zg, 2),
            "seg_count": zs.seg_count,
            "settled": zs.settled,
            "break_direction": getattr(zs, "break_direction", ""),
        })
    for mv in moves:
        l1["moves"].append({
            "direction": mv.direction,
            "kind": mv.kind,
            "settled": mv.settled,
            "zhongshu_count": mv.zs_count,
        })
    for bsp in bsps:
        l1["buysellpoints"].append({
            "kind": bsp.kind,
            "side": bsp.side,
            "seg_idx": bsp.seg_idx,
            "level_id": bsp.level_id,
        })
    result["levels"]["L1"] = l1

    for rsnap in recursive_snaps:
        lvl_key = f"L{rsnap.level_id}"
        lvl = {"zhongshus": [], "moves": [], "buysellpoints": []}
        for zs in rsnap.zhongshus:
            lvl["zhongshus"].append({
                "zd": round(zs.zd, 2),
                "zg": round(zs.zg, 2),
                "comp_count": zs.comp_count,
                "settled": zs.settled,
            })
        for mv in rsnap.moves:
            lvl["moves"].append({
                "direction": mv.direction,
                "kind": mv.kind,
                "settled": mv.settled,
                "zhongshu_count": mv.zs_count,
            })
        for bsp in recursive_bsps.get(rsnap.level_id, []):
            lvl["buysellpoints"].append({
                "kind": bsp.kind,
                "side": bsp.side,
                "seg_idx": bsp.seg_idx,
                "level_id": bsp.level_id,
            })
        result["levels"][lvl_key] = lvl

    return result


def print_report(result: dict) -> None:
    """打印人类可读的级别报告。"""
    print(f"\n{'='*60}", flush=True)
    print(f"  布伦特原油 (BRN) 级别分析", flush=True)
    print(f"{'='*60}", flush=True)
    print(f"  K线数量: {result['bar_count']}", flush=True)
    print(f"  时间范围: {result['bar_range']}", flush=True)
    print(f"  最新收盘: {result['last_close']}", flush=True)
    print(flush=True)

    for lvl_key in sorted(result["levels"].keys(), key=lambda x: int(x[1:])):
        lvl = result["levels"][lvl_key]
        print(f"── {lvl_key} {'─'*50}", flush=True)

        if "strokes" in lvl:
            print(f"  笔: {lvl['strokes']}  线段: {lvl['segments']}", flush=True)

        zs_list = lvl["zhongshus"]
        mv_list = lvl["moves"]
        bsp_list = lvl.get("buysellpoints", [])

        print(f"  中枢: {len(zs_list)} 个", flush=True)
        for i, zs in enumerate(zs_list[-3:]):  # 只显示最近3个
            settled_mark = "已破" if zs["settled"] else "运行中"
            print(f"    [{len(zs_list)-3+i+1 if len(zs_list)>3 else i+1}] "
                  f"[{zs['zd']:.2f}, {zs['zg']:.2f}] "
                  f"({zs.get('seg_count', zs.get('comp_count', '?'))}段) {settled_mark}", flush=True)

        print(f"  走势: {len(mv_list)} 段", flush=True)
        for i, mv in enumerate(mv_list[-3:]):
            settled_mark = "已完成" if mv["settled"] else "进行中"
            kind_zh = "盘整" if mv["kind"] == "consolidation" else "趋势"
            dir_zh = "上" if mv["direction"] == "up" else "下"
            print(f"    [{len(mv_list)-3+i+1 if len(mv_list)>3 else i+1}] "
                  f"{dir_zh}{kind_zh} (含{mv['zhongshu_count']}中枢) {settled_mark}", flush=True)

        if bsp_list:
            recent_bsp = bsp_list[-5:]
            print(f"  买卖点: {len(bsp_list)} 个 (最近{len(recent_bsp)}个)", flush=True)
            for bsp in recent_bsp:
                side_zh = "买" if bsp["side"] == "buy" else "卖"
                print(f"    第{bsp['kind']}类{side_zh}点 (seg={bsp['seg_idx']}, L{bsp['level_id']})", flush=True)
        print(flush=True)

    # 当前位置判断
    levels = result["levels"]
    print(f"── 综合判断 {'─'*46}", flush=True)

    max_level = 0
    for lvl_key, lvl in levels.items():
        lid = int(lvl_key[1:])
        if lvl["zhongshus"] or lvl["moves"]:
            max_level = max(max_level, lid)

    print(f"  最高有效级别: L{max_level}", flush=True)

    # 当前走势状态
    for lid in range(max_level, 0, -1):
        lvl_key = f"L{lid}"
        if lvl_key not in levels:
            continue
        mvs = levels[lvl_key]["moves"]
        zss = levels[lvl_key]["zhongshus"]
        if mvs:
            latest = mvs[-1]
            kind_zh = "盘整" if latest["kind"] == "consolidation" else "趋势"
            dir_zh = "上涨" if latest["direction"] == "up" else "下跌"
            status = "进行中" if not latest["settled"] else "已完成"
            print(f"  L{lid}: {dir_zh}{kind_zh} ({status})", flush=True)
        if zss:
            latest_zs = zss[-1]
            zs_status = "已破" if latest_zs["settled"] else "运行中"
            print(f"       中枢 [{latest_zs['zd']:.2f}, {latest_zs['zg']:.2f}] {zs_status}", flush=True)

    print(f"\n{'='*60}\n", flush=True)


def main():
    import argparse
    parser = argparse.ArgumentParser(description="布伦特原油级别分析")
    parser.add_argument("--source", choices=["databento", "yfinance"], default="databento",
                        help="数据源 (default: databento)")
    # yfinance 参数
    parser.add_argument("--ticker", default="BZ=F", help="yfinance ticker")
    parser.add_argument("--interval", default="1m", help="K线周期 (databento: ohlcv-1m/1h/1d; yfinance: 1m/1h/1d)")
    parser.add_argument("--period", default="2y", help="yfinance 回溯期")
    # databento 参数
    parser.add_argument("--symbol", default="BRN.c.0", help="Databento 连续合约符号")
    parser.add_argument("--dataset", default="IFEU.IMPACT", help="Databento 数据集")
    parser.add_argument("--start", default="2020-01-01", help="Databento 起始日期")
    parser.add_argument("--end", default=None, help="Databento 结束日期 (default: now)")
    parser.add_argument("--batch", action="store_true",
                        help="批量模式: 一次性构建全量数据，O(n) 而非 O(n^2)")
    args = parser.parse_args()

    if args.source == "databento":
        # 将简写 interval 映射为 databento schema
        interval_to_schema = {"1m": "ohlcv-1m", "1h": "ohlcv-1h", "1d": "ohlcv-1d"}
        schema = interval_to_schema.get(args.interval, f"ohlcv-{args.interval}")

        print(f"获取 {args.symbol} 数据 (source=databento, schema={schema})...", flush=True)
        bars = fetch_bars_databento(
            symbol=args.symbol,
            dataset=args.dataset,
            schema=schema,
            start=args.start,
            end=args.end,
        )
    else:
        print(f"获取 {args.ticker} 数据 (source=yfinance, interval={args.interval}, period={args.period})...", flush=True)
        bars = fetch_bars_yf(ticker=args.ticker, interval=args.interval, period=args.period)

    print(f"获取到 {len(bars)} 根 K 线", flush=True)

    if args.batch:
        print("计算级别结构 (batch 模式)...", flush=True)
        result = analyze_levels_batch(bars, stream_id="BRN")
    else:
        print("计算级别结构...", flush=True)
        result = analyze_levels(bars, stream_id="BRN")

    # 保存 JSON
    import json
    out_path = Path(__file__).resolve().parent.parent / "tmp" / "brn-level-report.json"
    out_path.write_text(json.dumps(result, indent=2, default=str, ensure_ascii=False), encoding="utf-8")
    print(f"JSON 报告已保存: {out_path}", flush=True)

    # 打印可读报告
    print_report(result)


if __name__ == "__main__":
    main()
