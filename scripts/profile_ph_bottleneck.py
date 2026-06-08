"""PH 瓶颈 profiler — 确认 attach_persistence→sublevel_h0_bars 的超线性。

用法：
    PYTHONPATH=src python scripts/profile_ph_bottleneck.py [N]

读 .cache 的 BZ 1min 数据（代理 ES），跑 RecursiveOrchestrator 全链，
cProfile 输出按累计耗时排序的前若干行，并做 N 标度（N/2 vs N）实测复杂度指数。
"""
from __future__ import annotations

import cProfile
import io
import math
import pstats
import sys
import time

import pandas as pd

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar


def load_bars(n: int) -> list[Bar]:
    df = pd.read_parquet(".cache/BZ_1min_2024_raw.parquet")
    df = df.iloc[:n]
    idx = pd.to_datetime(df.index)
    bars = [
        Bar(ts=t, open=float(o), high=float(h), low=float(lo), close=float(c),
            volume=float(v) if v == v else None)
        for t, o, h, lo, c, v in zip(
            idx, df["open"], df["high"], df["low"], df["close"], df["volume"]
        )
    ]
    return bars


def run_once(bars: list[Bar]) -> float:
    orch = RecursiveOrchestrator(stream_id="prof")
    t0 = time.perf_counter()
    for b in bars:
        orch.process_bar(b)
    return time.perf_counter() - t0


def main() -> None:
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 200_000
    bars = load_bars(n)
    print(f"loaded {len(bars)} bars")

    # 标度实测复杂度指数：t(N) / t(N/2) = 2^alpha
    half = run_once(bars[: n // 2])
    full = run_once(bars)
    alpha = math.log(full / half) / math.log(2) if half > 0 else float("nan")
    print(f"t(N/2)={half:.2f}s  t(N)={full:.2f}s  实测复杂度指数 alpha={alpha:.3f}")

    # cProfile 全量（用 N/2 控制时长）
    pr = cProfile.Profile()
    pr.enable()
    run_once(bars[: n // 2])
    pr.disable()
    s = io.StringIO()
    ps = pstats.Stats(pr, stream=s).sort_stats("cumulative")
    ps.print_stats(25)
    print(s.getvalue())

    # 单独看 tottime（自身耗时）
    s2 = io.StringIO()
    pstats.Stats(pr, stream=s2).sort_stats("tottime").print_stats(15)
    print("=== by tottime ===")
    print(s2.getvalue())


if __name__ == "__main__":
    main()
