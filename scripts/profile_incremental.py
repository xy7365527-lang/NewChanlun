#!/usr/bin/env python3
"""增量化优化 — 逐层 wall-clock 缩放归因（步骤1：profile）。

用真实 BZ 1min 数据，在递增规模下逐引擎计时，估计每层缩放指数 α。
α≈1 → O(N)/总 = O(1)/bar；α≈2 → O(N²)/总 = O(N)/bar（瓶颈）。

用法：
    PYTHONPATH=src python3 scripts/profile_incremental.py
    PYTHONPATH=src python3 scripts/profile_incremental.py --sizes 10000,20000,40000,80000
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from newchan.types import Bar
from performance_benchmark import benchmark_layer_breakdown, estimate_scaling_exponent

CACHE = ROOT / ".cache"


def load_bz_bars_fast(limit: int | None = None) -> list[Bar]:
    """向量化加载（避免 iterrows，267 万行快 ~50×）。"""
    frames = []
    for fname in [
        "BZ_1min_2010_2014_raw.parquet",
        "BZ_1min_2015_2018_raw.parquet",
        "BZ_1min_2019_2023_raw.parquet",
        "BZ_1min_2024_raw.parquet",
    ]:
        p = CACHE / fname
        if p.exists():
            frames.append(pd.read_parquet(p))
    df = pd.concat(frames).sort_index()
    if limit is not None:
        df = df.iloc[:limit]
    ts = df.index.to_pydatetime()
    o = df["open"].to_numpy(); h = df["high"].to_numpy()
    lo = df["low"].to_numpy(); c = df["close"].to_numpy()
    has_v = "volume" in df.columns
    v = df["volume"].to_numpy() if has_v else None
    bars: list[Bar] = []
    for i in range(len(df)):
        t = ts[i]
        bars.append(Bar(
            ts=t.replace(tzinfo=None) if getattr(t, "tzinfo", None) else t,
            open=float(o[i]), high=float(h[i]), low=float(lo[i]), close=float(c[i]),
            volume=float(v[i]) if has_v else None,
        ))
    return bars


def load_bz_bars(limit: int | None = None) -> list[Bar]:
    """加载 BZ 1min 真实数据（按时间拼接所有分片）。"""
    frames = []
    for fname in [
        "BZ_1min_2010_2014_raw.parquet",
        "BZ_1min_2015_2018_raw.parquet",
        "BZ_1min_2019_2023_raw.parquet",
        "BZ_1min_2024_raw.parquet",
    ]:
        p = CACHE / fname
        if p.exists():
            frames.append(pd.read_parquet(p))
    df = pd.concat(frames).sort_index()
    if limit is not None:
        df = df.iloc[:limit]
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime().replace(tzinfo=None) if hasattr(ts, "to_pydatetime") else ts,
            open=float(row["open"]), high=float(row["high"]),
            low=float(row["low"]), close=float(row["close"]),
            volume=float(row["volume"]) if "volume" in row and row["volume"] is not None else None,
        ))
    return bars


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sizes", default="10000,20000,40000,80000")
    args = ap.parse_args()
    sizes = [int(s) for s in args.sizes.split(",")]

    max_size = max(sizes)
    print(f"加载 BZ 1min 真实数据 (max {max_size:,} bars)...")
    all_bars = load_bz_bars(limit=max_size)
    print(f"实际加载 {len(all_bars):,} bars\n")

    layer_series: dict[str, list[float]] = {}
    actual_sizes: list[int] = []

    for n in sizes:
        if n > len(all_bars):
            continue
        bars = all_bars[:n]
        times = benchmark_layer_breakdown(bars)
        actual_sizes.append(n)
        total = sum(times.values())
        print(f"=== N={n:,} (total {total:.2f}s, {n/total:,.0f} bars/s) ===")
        for layer, t in sorted(times.items(), key=lambda kv: -kv[1]):
            pct = t / total * 100 if total else 0
            print(f"  {layer:22s} {t:8.3f}s  {pct:5.1f}%")
            layer_series.setdefault(layer, []).append(t)
        print()

    print("=== 缩放指数 α (per-layer, 越接近2越是瓶颈) ===")
    for layer, series in sorted(layer_series.items()):
        if len(series) >= 2:
            alpha = estimate_scaling_exponent(actual_sizes, series)
            flag = " ← O(N²) 瓶颈" if alpha > 1.4 else ""
            print(f"  {layer:22s} α={alpha:.2f}{flag}")


if __name__ == "__main__":
    main()
