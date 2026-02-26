"""RecursiveOrchestrator 性能基准测试。

生成合成数据，分层计时全链路处理，检测 O(n²) 瓶颈，输出性能报告。

已知瓶颈：BiEngine._run_pipeline() 每 bar 重建完整 DataFrame 并全量重跑
inclusion → fractals → strokes 管线，导致 O(n²) 复杂度。
"""

from __future__ import annotations

import json
import math
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar


# ── 合成数据生成 ──


def generate_zigzag_bars(
    n: int = 10000,
    amplitude: float = 10.0,
    base: float = 100.0,
    period: int = 10,
) -> list[Bar]:
    """生成锯齿形 K 线序列。

    Parameters
    ----------
    n : int
        K 线数量。
    amplitude : float
        振幅（高低点差值的一半）。
    base : float
        基准价格。
    period : int
        一个完整锯齿周期的 bar 数。
    """
    epoch = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    for i in range(n):
        cycle = (i % period) / period
        if cycle < 0.5:
            price = base + amplitude * (cycle * 2)
        else:
            price = base + amplitude * (1.0 - (cycle - 0.5) * 2)
        drift = i * 0.001
        price += drift
        noise = (i % 3) * 0.1
        bars.append(Bar(
            ts=epoch + timedelta(seconds=i * 60),
            open=price - 0.5 + noise,
            high=price + 1.0 + noise,
            low=price - 1.0 - noise,
            close=price + 0.5 - noise,
        ))
    return bars


def generate_multiscale_bars(
    n: int = 10000,
    base: float = 100.0,
) -> list[Bar]:
    """生成多尺度叠加的 K 线序列（更接近真实行情）。

    三层正弦叠加：短周期(10) + 中周期(50) + 长周期(200)。
    """
    epoch = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    for i in range(n):
        short = 3.0 * math.sin(2 * math.pi * i / 10)
        mid = 8.0 * math.sin(2 * math.pi * i / 50)
        long_ = 15.0 * math.sin(2 * math.pi * i / 200)
        price = base + short + mid + long_ + i * 0.002
        spread = 0.8 + 0.3 * abs(math.sin(i * 0.1))
        bars.append(Bar(
            ts=epoch + timedelta(seconds=i * 60),
            open=price - spread * 0.3,
            high=price + spread,
            low=price - spread,
            close=price + spread * 0.3,
        ))
    return bars


# ── 分层计时 ──


def benchmark_layer_breakdown(
    bars: list[Bar],
) -> dict[str, float]:
    """逐引擎单独跑一遍，返回各层耗时。"""
    from newchan.bi_engine import BiEngine
    from newchan.core.recursion.buysellpoint_engine import BuySellPointEngine
    from newchan.core.recursion.move_engine import MoveEngine
    from newchan.core.recursion.segment_engine import SegmentEngine
    from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
    from newchan.core.recursion.recursive_stack import RecursiveStack

    times: dict[str, float] = {}

    bi_eng = BiEngine(stroke_mode="wide", min_strict_sep=5)
    bi_snaps = []
    t0 = time.perf_counter()
    for bar in bars:
        bi_snaps.append(bi_eng.process_bar(bar))
    times["bi_engine"] = time.perf_counter() - t0

    seg_eng = SegmentEngine(stream_id="perf-bench")
    seg_snaps = []
    t0 = time.perf_counter()
    for bs in bi_snaps:
        seg_snaps.append(seg_eng.process_snapshot(bs))
    times["segment_engine"] = time.perf_counter() - t0

    zs_eng = ZhongshuEngine(stream_id="perf-bench")
    zs_snaps = []
    t0 = time.perf_counter()
    for ss in seg_snaps:
        zs_snaps.append(zs_eng.process_segment_snapshot(ss))
    times["zhongshu_engine"] = time.perf_counter() - t0

    mv_eng = MoveEngine(stream_id="perf-bench")
    mv_snaps = []
    t0 = time.perf_counter()
    for idx, zss in enumerate(zs_snaps):
        mv_snaps.append(mv_eng.process_zhongshu_snapshot(
            zss, num_segments=len(seg_snaps[idx].segments),
        ))
    times["move_engine"] = time.perf_counter() - t0

    bsp_eng = BuySellPointEngine(level_id=1, stream_id="perf-bench")
    t0 = time.perf_counter()
    for idx, mvs in enumerate(mv_snaps):
        bsp_eng.process_snapshots(mvs, zs_snaps[idx], seg_snaps[idx])
    times["buysellpoint_engine"] = time.perf_counter() - t0

    rec_stack = RecursiveStack(max_levels=6, stream_id="perf-bench")
    t0 = time.perf_counter()
    for mvs in mv_snaps:
        rec_stack.process_level1_move_snapshot(mvs)
    times["recursive_stack"] = time.perf_counter() - t0

    return times


def benchmark_orchestrator(
    bars: list[Bar],
    label: str = "default",
    layer_breakdown: bool = False,
) -> dict[str, Any]:
    """对 RecursiveOrchestrator 做全链路基准测试。

    Parameters
    ----------
    bars : list[Bar]
        输入 K 线序列。
    label : str
        场景标签。
    layer_breakdown : bool
        是否做分层计时（会额外跑一遍各引擎）。
    """
    orch = RecursiveOrchestrator(stream_id="perf-bench")

    t0 = time.perf_counter()
    last_snap = None
    for bar in bars:
        last_snap = orch.process_bar(bar)
    total_elapsed = time.perf_counter() - t0

    stats: dict[str, Any] = {
        "n_bars": len(bars),
        "n_strokes": len(last_snap.bi_snapshot.strokes) if last_snap else 0,
        "n_segments": len(last_snap.seg_snapshot.segments) if last_snap else 0,
        "n_zhongshu": len(last_snap.zs_snapshot.zhongshus) if last_snap else 0,
        "n_moves": len(last_snap.move_snapshot.moves) if last_snap else 0,
        "n_recursive_levels": len(last_snap.recursive_snapshots) if last_snap else 0,
    }

    report: dict[str, Any] = {
        "label": label,
        "total_seconds": round(total_elapsed, 4),
        "bars_per_second": round(len(bars) / total_elapsed, 1) if total_elapsed > 0 else 0,
        "structure_stats": stats,
    }

    if layer_breakdown:
        layer_times = benchmark_layer_breakdown(bars)
        report["layer_times"] = {k: round(v, 4) for k, v in layer_times.items()}

    return report


def estimate_scaling_exponent(
    sizes: list[int],
    times: list[float],
) -> float:
    """从 (size, time) 对估算复杂度指数。

    假设 T(n) ∝ n^α，用最小二乘法在 log-log 空间拟合 α。
    """
    import math as m
    n = len(sizes)
    if n < 2:
        return 0.0
    log_s = [m.log(s) for s in sizes]
    log_t = [m.log(t) for t in times]
    mean_ls = sum(log_s) / n
    mean_lt = sum(log_t) / n
    num = sum((ls - mean_ls) * (lt - mean_lt) for ls, lt in zip(log_s, log_t))
    den = sum((ls - mean_ls) ** 2 for ls in log_s)
    return num / den if den > 0 else 0.0


def run_scaling_analysis() -> dict[str, Any]:
    """多尺度计时 + 复杂度指数估算。"""
    sizes = [200, 500, 1000, 2000]
    results: list[dict[str, Any]] = []

    for n in sizes:
        bars = generate_zigzag_bars(n)
        report = benchmark_orchestrator(bars, label=f"zigzag-{n}")
        results.append(report)
        print(f"  {n:>5} bars: {report['total_seconds']:.3f}s  "
              f"({report['bars_per_second']:.0f} bars/s)")

    times = [r["total_seconds"] for r in results]
    alpha = estimate_scaling_exponent(sizes, times)

    # 外推 10k
    if times[-1] > 0:
        extrapolated_10k = times[-1] * (10000 / sizes[-1]) ** alpha
    else:
        extrapolated_10k = 0.0

    return {
        "sizes": sizes,
        "times": [round(t, 4) for t in times],
        "scaling_exponent": round(alpha, 2),
        "scaling_class": "O(n)" if alpha < 1.3 else f"O(n^{alpha:.1f})",
        "extrapolated_10k_seconds": round(extrapolated_10k, 1),
        "target_10k_met": extrapolated_10k < 10.0,
        "results": results,
    }


def run_all_benchmarks() -> dict[str, Any]:
    """运行全部基准测试。"""
    output: dict[str, Any] = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

    # 1. 复杂度分析
    print("Phase 1: Scaling Analysis")
    print("-" * 40)
    scaling = run_scaling_analysis()
    output["scaling_analysis"] = {
        k: v for k, v in scaling.items() if k != "results"
    }
    print(f"\n  Scaling exponent α = {scaling['scaling_exponent']}")
    print(f"  Complexity class: {scaling['scaling_class']}")
    print(f"  Extrapolated 10k: {scaling['extrapolated_10k_seconds']:.1f}s "
          f"({'PASS' if scaling['target_10k_met'] else 'FAIL'})")

    # 2. 分层计时（1000 bars — 可在合理时间内完成）
    print(f"\nPhase 2: Layer Breakdown (1000 bars)")
    print("-" * 40)
    bars_1k = generate_zigzag_bars(1000)
    layer_report = benchmark_orchestrator(bars_1k, label="layer-1k", layer_breakdown=True)
    output["layer_breakdown"] = layer_report

    if "layer_times" in layer_report:
        total = layer_report["total_seconds"]
        for layer, t in sorted(layer_report["layer_times"].items(), key=lambda x: -x[1]):
            pct = (t / total * 100) if total > 0 else 0
            print(f"  {layer:<25} {t:.4f}s  ({pct:.1f}%)")

    # 3. 多尺度数据对比（500 bars）
    print(f"\nPhase 3: Data Pattern Comparison (500 bars)")
    print("-" * 40)
    patterns: dict[str, Any] = {}
    for name, bars in [
        ("zigzag-500", generate_zigzag_bars(500)),
        ("multiscale-500", generate_multiscale_bars(500)),
    ]:
        r = benchmark_orchestrator(bars, label=name)
        patterns[name] = r
        s = r["structure_stats"]
        print(f"  {name}: {r['total_seconds']:.3f}s | "
              f"{s['n_strokes']} strokes, {s['n_segments']} segs")
    output["pattern_comparison"] = patterns

    # 4. 瓶颈诊断
    output["bottleneck_diagnosis"] = {
        "root_cause": "BiEngine._run_pipeline() rebuilds full DataFrame on every bar",
        "location": "src/newchan/bi_engine.py:107-122",
        "complexity": "O(n^2) — _build_df + merge_inclusion + fractals_from_merged + strokes_from_fractals per bar",
        "fix_suggestion": "Incremental pipeline: maintain merged bars / fractals / strokes state, only process new/changed tail on each bar",
    }

    return output


def main() -> None:
    print("RecursiveOrchestrator Performance Benchmark")
    print("=" * 50)

    results = run_all_benchmarks()

    out_dir = Path(__file__).resolve().parent.parent / "tmp"
    out_dir.mkdir(exist_ok=True)
    out_path = out_dir / "perf-report.json"
    out_path.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\nReport saved to {out_path}")


if __name__ == "__main__":
    main()
