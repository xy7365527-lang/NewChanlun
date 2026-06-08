#!/usr/bin/env python3
"""Part 3 重计算：黄金计价 K4 边的 1min a0 逐日 σ（Rust 引擎驱动）。

把资产价格的计价单位从美元（usd6e=M）换成黄金（GC=Au），对黄金计价比价
ES/GC、CL/GC、ZN/GC、BRN/GC、DX/GC、EURUSD/GC 各自从 1min a0 跑缠论递归，
取每 UTC 日末**最高涌现级别**走势方向 σ。

与 k4_config_transition_1min.py 的 USD 计价 σ（缓存 k4_config_transition_1min.json）
**同分辨率同方法**（1min a0、对数包络比价、最高涌现级别 σ、UTC 日边界、
zero-lookahead），唯一差异是分母从 usd6e 换成 GC → 直接可比。

引擎：newchan_rust.RecursiveOrchestrator（逐位等价 Python 引擎，~23× 快）。
六边并行（multiprocessing spawn，局部依赖原则 275 号）。

认识论等级：L2（真实 1min 期货数据，非合成；可产生否定性结果）。
"""

from __future__ import annotations

import json
import multiprocessing as mp
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for p in (str(ROOT), str(ROOT / "src")):
    if p not in sys.path:
        sys.path.insert(0, p)

from analysis.k4_1min_lib import RatioBars, Series, load_series, log_envelope_ratio

JSON_OUT = ROOT / "analysis" / "data_cache" / "gold_k4_sigma.json"

# 黄金计价边：(name, numerator_file, optional_invert_num)
# 分母统一为 GC（黄金）。EURUSD = 1/usd6e（usd6e 是 USD 强度=1/EURUSD）。
GOLD_EDGES = [
    ("ES/GC", "es_1m_databento_10y.json", False),    # 生产资本 P 的黄金计价
    ("CL/GC", "cl_1m_databento_10y.json", False),    # 商品(油) C 的黄金计价
    ("ZN/GC", "zn_1m_databento_10y.json", False),    # 国债 R 的黄金计价
    ("BRN/GC", "brn_1m_databento_10y.json", False),  # 布伦特油的黄金计价（2019+）
    ("DX/GC", "dx_1m_databento_10y.json", False),    # 美元指数的黄金计价（2019+）
    ("EUR/GC", "usd6e_1m_databento_10y.json", True),  # 欧元(EURUSD=1/usd6e)的黄金计价
]


def invert_series(s: Series) -> Series:
    """取倒数：EURUSD = 1/usd6e。OHLC 倒数后 high↔low 互换（保持 high≥low）。"""
    return Series(
        symbol=f"1/{s.symbol}",
        ts=s.ts, day=s.day,
        o=1.0 / s.o, h=1.0 / s.l, l=1.0 / s.h, c=1.0 / s.c,
    )


# ════════════════════════════════════════════════════════════
# Rust 引擎 emergent σ（与 k4_1min_lib.emergent_sigma 同语义，Rust tuple 版）
# ════════════════════════════════════════════════════════════

def _sigma_from_head(head) -> int:
    """MoveTuple head=(kind,direction,...)。consolidation→0，up→+1，down→−1。"""
    kind, direction = head[0], head[1]
    if kind == "consolidation":
        return 0
    if direction == "up":
        return 1
    if direction == "down":
        return -1
    return 0


def _emergent_sigma_level(orch) -> tuple[int, int]:
    """读最高涌现级别的走势方向 σ 与级别 id（Rust orchestrator）。

    最高涌现级别 = current_recursive() 中有 move 的最大 level_id；
    若递归层无 move，回退 L1 current_moves()。取该级别最后一个 move。
    """
    chosen_head = None
    max_lvl = -1
    for level_id, _zs, moves in orch.current_recursive():
        if moves and level_id > max_lvl:
            max_lvl = level_id
            chosen_head = moves[-1][0]
    if chosen_head is None:
        l1 = orch.current_moves()
        if l1:
            chosen_head = l1[-1][0]
            max_lvl = 1
    if chosen_head is None:
        return 0, 1
    return _sigma_from_head(chosen_head), max(1, max_lvl)


@dataclass
class EdgeDailySigma:
    name: str
    n_bars: int
    days: list[int]
    sigma: list[int]
    level: list[int]
    max_level: int
    elapsed: float


def run_edge_daily_sigma_rust(rb: RatioBars, *, max_levels: int = 6) -> EdgeDailySigma:
    """Rust 引擎流式跑比价 bar，每 UTC 日边界记录 as-of 最高涌现级别 σ。"""
    import newchan_rust

    t0 = time.time()
    n = len(rb.c)
    day = rb.day
    o, h, l, c = rb.o, rb.h, rb.l, rb.c
    orch = newchan_rust.RecursiveOrchestrator(max_levels=max_levels, stroke_mode="wide")

    days_out: list[int] = []
    sigma_out: list[int] = []
    level_out: list[int] = []
    max_lvl_seen = 1

    for i in range(n):
        orch.process_bar(float(o[i]), float(h[i]), float(l[i]), float(c[i]))
        if i == n - 1 or day[i + 1] != day[i]:
            sg, lv = _emergent_sigma_level(orch)
            days_out.append(int(day[i]))
            sigma_out.append(sg)
            level_out.append(lv)
            max_lvl_seen = max(max_lvl_seen, lv)

    return EdgeDailySigma(
        name=rb.name, n_bars=n, days=days_out, sigma=sigma_out,
        level=level_out, max_level=max_lvl_seen, elapsed=time.time() - t0,
    )


def _worker(args: tuple) -> dict:
    """spawn worker：构造一条黄金计价比价 + Rust 递归 → 逐日 σ。"""
    name, num_file, invert = args
    num = load_series(num_file)
    if invert:
        num = invert_series(num)
    gc = load_series("gc_1m_databento_10y.json")
    rb = log_envelope_ratio(num, gc, name)
    r = run_edge_daily_sigma_rust(rb)
    print(f"  {name:8s} done: {r.n_bars:>10,} bars, {len(r.days)} days, "
          f"max_lvl=L{r.max_level}, {r.elapsed:.0f}s", flush=True)
    return {
        "name": r.name, "n_bars": r.n_bars, "n_days": len(r.days),
        "max_level": r.max_level, "elapsed_s": r.elapsed,
        "days": r.days, "sigma": r.sigma, "level": r.level,
    }


def main() -> None:
    t_start = time.time()
    print("=" * 64)
    print("  Part 3：黄金计价 K4 边 1min a0 逐日 σ（Rust 引擎，六边并行）")
    print("=" * 64, flush=True)

    procs = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=procs) as pool:
        results = pool.map(_worker, GOLD_EDGES)

    out = {
        "meta": {
            "task": "gold_k4_sigma",
            "numeraire": "GC (gold)",
            "method": "1min a0, log-envelope ratio, highest-emergent-level sigma, "
                      "UTC daily boundary, zero-lookahead, Rust orchestrator",
            "comparable_to": "k4_config_transition_1min.json (USD numeraire, same method)",
            "epistemological_level": "L2",
            "elapsed_s": time.time() - t_start,
        },
        "edges": {r["name"]: r for r in results},
    }
    JSON_OUT.write_text(json.dumps(out))
    print(f"\n总耗时：{time.time()-t_start:.0f}s → {JSON_OUT}")


if __name__ == "__main__":
    main()
