"""bi 引擎 Python vs Rust wall-clock 基准（Phase 4 性能验证）。

纯处理计时——只测 process_bar 流式驱动，不含快照读取/对比开销，
隔离引擎本身的计算成本。

认识论等级：L2（真实数据 wall-clock）。提速倍数取决于 cache 行为/分支预测，
是本机实测，非通用断言。

用法::

    PYTHONPATH=src .venv/bin/python scripts/bench_rust_bi.py
"""

from __future__ import annotations

import time

import pandas as pd

from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

import newchan_rust

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


def main() -> None:
    df = pd.read_parquet(_BZ_PARQUET)
    o = df["open"].tolist()
    h = df["high"].tolist()
    l = df["low"].tolist()
    c = df["close"].tolist()
    ts = [t.timestamp() for t in df.index]
    n = len(o)
    print(f"数据: BZ Brent 1min, {n:,} bars")

    # ── Python ──
    py = PyBiEngine(stroke_mode="new")
    bars = [
        BarV1(bar_time=ts[k], open=o[k], high=h[k], low=l[k], close=c[k])
        for k in range(n)
    ]  # 预构造，排除对象构造开销
    t0 = time.perf_counter()
    for b in bars:
        py.process_bar(b)
    py_dt = time.perf_counter() - t0
    py_strokes = len(py.current_strokes)

    # ── Rust ──
    rs = newchan_rust.BiEngine(stroke_mode="new")
    t0 = time.perf_counter()
    for k in range(n):
        rs.process_bar(o[k], h[k], l[k], c[k])
    rs_dt = time.perf_counter() - t0
    rs_strokes = len(rs.current_strokes())

    print(f"\nPython: {py_dt:8.3f}s  ({n / py_dt:>10,.0f} bars/s)  final strokes={py_strokes}")
    print(f"Rust:   {rs_dt:8.3f}s  ({n / rs_dt:>10,.0f} bars/s)  final strokes={rs_strokes}")
    print(f"\n提速: {py_dt / rs_dt:.1f}× (wall-clock, L2 本机实测)")
    assert py_strokes == rs_strokes, "最终笔数不一致——等价性破坏"


if __name__ == "__main__":
    main()
