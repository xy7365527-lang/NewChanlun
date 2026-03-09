#!/usr/bin/env python3
"""392·3: g(v,w) 计算性能基准测试。

在真实谱系图上测量 compute_g 的耗时，评估能否实时计算。
测试多个不同规模的图以获得完整的性能画像。
"""
import sys
import time
import random
import statistics
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import json
from engine import Graph
from daemon import graph_from_dict
from verify_fg_predictions import compute_g

DATA_DIR = Path(__file__).resolve().parent / "data"


def load_graph_from_json(path: Path) -> Graph:
    """从 JSON 文件加载图。"""
    data = json.loads(path.read_text(encoding="utf-8"))
    return graph_from_dict(data)


def benchmark_single(graph: Graph, name: str, n_samples: int = 100) -> dict:
    """对单个图运行基准测试。"""
    active = list(graph.active_vertex_ids())
    n_vertices = len(active)
    n_edges = sum(1 for _ in graph.active_edges())

    if n_vertices < 2:
        print(f"  {name}: 图太小 ({n_vertices} 顶点), 跳过")
        return {}

    actual_samples = min(n_samples, n_vertices * (n_vertices - 1) // 2)
    pairs = []
    seen = set()
    attempts = 0
    while len(pairs) < actual_samples and attempts < actual_samples * 10:
        attempts += 1
        v = random.choice(active)
        w = random.choice(active)
        if v == w:
            continue
        key = frozenset((v, w))
        if key in seen:
            continue
        seen.add(key)
        pairs.append((v, w))

    times = []
    g_values = []
    for v, w in pairs:
        t0 = time.perf_counter()
        gval = compute_g(graph, v, w)
        t1 = time.perf_counter()
        elapsed_ms = (t1 - t0) * 1000
        times.append(elapsed_ms)
        g_values.append(gval)

    result = {
        "name": name,
        "vertices": n_vertices,
        "edges": n_edges,
        "samples": len(pairs),
        "avg_ms": statistics.mean(times),
        "median_ms": statistics.median(times),
        "p95_ms": sorted(times)[int(len(times) * 0.95)] if times else 0,
        "max_ms": max(times) if times else 0,
        "avg_g": statistics.mean(g_values) if g_values else 0,
        "g_zero_pct": sum(1 for v in g_values if v == 0) / len(g_values) * 100 if g_values else 0,
    }

    print(f"  {name:25s}  {n_vertices:5d}V {n_edges:5d}E  "
          f"avg={result['avg_ms']:.3f}ms  p95={result['p95_ms']:.3f}ms  "
          f"max={result['max_ms']:.3f}ms  g_avg={result['avg_g']:.1f}")

    return result


def main():
    random.seed(42)

    # 找到所有 graph_*.json 文件，按大小排序
    graph_files = sorted(DATA_DIR.glob("graph_*.json"), key=lambda p: p.stat().st_size)

    print(f"找到 {len(graph_files)} 个图文件")
    print(f"\n{'='*90}")
    print(f"  392·3: g(v,w) 性能基准")
    print(f"{'='*90}")
    print(f"  {'图名':25s}  {'规模':>12s}  {'平均':>10s}  {'P95':>10s}  {'最大':>10s}  {'g均值':>8s}")
    print(f"  {'-'*25}  {'-'*12}  {'-'*10}  {'-'*10}  {'-'*10}  {'-'*8}")

    results = []
    for gf in graph_files:
        try:
            graph = load_graph_from_json(gf)
            r = benchmark_single(graph, gf.stem.replace("graph_", ""))
            if r:
                results.append(r)
        except Exception as e:
            print(f"  {gf.stem}: 加载失败 ({e})")

    # 也测试 code 图（更大规模）
    code_files = sorted(DATA_DIR.glob("code_*.json"), key=lambda p: p.stat().st_size)
    for cf in code_files[-2:]:  # 最大的两个
        try:
            graph = load_graph_from_json(cf)
            r = benchmark_single(graph, cf.stem)
            if r:
                results.append(r)
        except Exception as e:
            print(f"  {cf.stem}: 加载失败 ({e})")

    # 汇总
    if results:
        print(f"\n{'='*90}")
        print(f"  汇总")
        print(f"{'='*90}")
        all_avgs = [r["avg_ms"] for r in results]
        all_vertices = [r["vertices"] for r in results]
        print(f"  图规模范围: {min(all_vertices)} ~ {max(all_vertices)} 顶点")
        print(f"  平均耗时范围: {min(all_avgs):.3f} ~ {max(all_avgs):.3f} ms")
        print(f"  总体平均: {statistics.mean(all_avgs):.3f} ms")

        # 实时性判断
        worst = max(all_avgs)
        if worst < 1:
            verdict = "全部亚毫秒级，可在穿越步进中实时计算"
        elif worst < 10:
            verdict = "全部毫秒级，可实时但大图需注意频率"
        elif worst < 100:
            verdict = "大图十毫秒级，建议缓存或增量更新"
        else:
            verdict = "大图百毫秒+，需要缓存策略"
        print(f"\n  结论: {verdict}")


if __name__ == "__main__":
    main()
