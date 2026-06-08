#!/usr/bin/env python3
"""重算 2020-2022 逐日**固定级别** σ（L1/L2/L3 + 最高），暴露级别漂移。

动机（诚实，no-workaround）
--------------------------
gold_k4_sigma.json 的『最高涌现级别 σ』在 1min a0 上产出 L3/L4 跨年 move，
单年窗口内**冻结**（实测：CL/GC 整个 2020-2022 是一个 L4 盘整 σ_C≡0；ZN/GC
整个 2022 是一个 L4 上趋势 σ_R≡+1，与『2022 债券崩盘』事实矛盾）。最高级别 σ
在单年窗口内几乎不转换，**无法回答 2020-2022 regime 切换问题**。

本脚本对三条 Γ 边（P/M=ES/GC, C/M=CL/GC, R/M=ZN/GC）从 2010 流式预热到
2022-12-31，在每个 UTC 日边界同时记录 **L1/L2/L3/最高** 四种 σ，使下游可选用
能在单年窗口内分辨的级别（L1/L2）做病态签名分析。

引擎：newchan_rust.RecursiveOrchestrator（Rust，max_levels=6）。
三边并行（multiprocessing spawn）。zero-lookahead（仅依赖当日及之前 bar）。

认识论等级：L2（真实 1min 期货数据）。管线正确性 L0/L1（Rust bit-exact）。

用法：PYTHONPATH=src .venv/bin/python analysis/regime_pathology_recompute_levels.py
"""

from __future__ import annotations

import json
import multiprocessing as mp
import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import newchan_rust  # noqa: E402

from analysis.k4_1min_lib import load_series, log_envelope_ratio  # noqa: E402

CACHE_OUT = ROOT / "analysis" / "data_cache" / "regime_levels_2020_2022.json"

# 三条 Γ 边：(边名, 分子文件, 分母文件)；分母统一 GC（货币锚 M）
GAMMA_EDGE_FILES = [
    ("P/M", "es_1m_databento_10y.json", "gc_1m_databento_10y.json"),  # σ_P
    ("C/M", "cl_1m_databento_10y.json", "gc_1m_databento_10y.json"),  # σ_C
    ("R/M", "zn_1m_databento_10y.json", "gc_1m_databento_10y.json"),  # σ_R
]

# 窗口上界：流式到 2022-12-31 即可（2020-2022 σ 不依赖之后的 bar）
DAY_2022_END = int(np.datetime64("2022-12-31").astype("datetime64[s]").astype("int64") // 86400)
DAY_2020_START = int(np.datetime64("2020-01-01").astype("datetime64[s]").astype("int64") // 86400)


def _head_sigma(head) -> int:
    """MoveTuple head=(kind,direction,...) → σ。consolidation→0/up→+1/down→−1。"""
    kind, direction = head[0], head[1]
    if kind == "consolidation":
        return 0
    return 1 if direction == "up" else (-1 if direction == "down" else 0)


def _sigma_by_level(orch) -> tuple[dict[int, int], int, int]:
    """返回 ({level_id: σ}, σ_highest, level_highest)。

    L1 = current_moves()；L≥2 = current_recursive() 各 level_id 的 moves[-1]。
    某级别无 move → 不在 dict 中（下游缺级别记 0=未涌现）。
    """
    by: dict[int, int] = {}
    l1 = orch.current_moves()
    if l1:
        by[1] = _head_sigma(l1[-1][0])
    high_lvl, high_sig = (1, by.get(1, 0))
    for level_id, _zs, moves in orch.current_recursive():
        if moves:
            sv = _head_sigma(moves[-1][0])
            by[level_id] = sv
            if level_id > high_lvl:
                high_lvl, high_sig = level_id, sv
    return by, high_sig, high_lvl


def _worker(args: tuple) -> dict:
    name, day, o, h, l, c = args
    t0 = time.time()
    n = len(c)
    orch = newchan_rust.RecursiveOrchestrator(max_levels=6, stroke_mode="wide")
    ol, hl, ll, cl, dl = o.tolist(), h.tolist(), l.tolist(), c.tolist(), day.tolist()

    days_out: list[int] = []
    s1_out: list[int] = []
    s2_out: list[int] = []
    s3_out: list[int] = []
    shi_out: list[int] = []
    lvl_out: list[int] = []

    for i in range(n):
        orch.process_bar(ol[i], hl[i], ll[i], cl[i])
        if i == n - 1 or dl[i + 1] != dl[i]:
            di = dl[i]
            if di < DAY_2020_START:   # 预热期不记录（省内存）
                continue
            by, shi, lhi = _sigma_by_level(orch)
            days_out.append(int(di))
            s1_out.append(by.get(1, 0))
            s2_out.append(by.get(2, 0))
            s3_out.append(by.get(3, 0))
            shi_out.append(shi)
            lvl_out.append(lhi)
        if (i + 1) % 1_000_000 == 0:
            print(f"    {name}: {i+1:,}/{n:,} ({time.time()-t0:.0f}s)", flush=True)

    return {
        "name": name, "n_bars": n, "elapsed_s": time.time() - t0,
        "days": days_out, "sigma_L1": s1_out, "sigma_L2": s2_out,
        "sigma_L3": s3_out, "sigma_high": shi_out, "level_high": lvl_out,
    }


def main() -> None:
    t_start = time.time()
    print("[1/3] 加载顶点序列 + GC ...", flush=True)
    gc = load_series("gc_1m_databento_10y.json")
    series_cache = {"gc_1m_databento_10y.json": gc}

    print("[2/3] 构造三条 Γ 边比价（截至 2022-12-31）...", flush=True)
    jobs = []
    for name, num_fn, den_fn in GAMMA_EDGE_FILES:
        num = series_cache.get(num_fn) or load_series(num_fn)
        den = series_cache.get(den_fn) or load_series(den_fn)
        rb = log_envelope_ratio(num, den, name)
        mask = rb.day <= DAY_2022_END
        ts, day = rb.ts[mask], rb.day[mask]
        o, h, l, c = rb.o[mask], rb.h[mask], rb.l[mask], rb.c[mask]
        print(f"  {name:5s} {len(c):>10,} bars (≤2022-12-31)", flush=True)
        jobs.append((name, day, o, h, l, c))

    print("[3/3] 三边并行 Rust 递归（记录 L1/L2/L3/最高 σ）...", flush=True)
    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=3) as pool:
        results = pool.map(_worker, jobs)

    out = {
        "meta": {
            "task": "regime_levels_2020_2022",
            "engine": "newchan_rust.RecursiveOrchestrator max_levels=6",
            "numeraire": "GC", "vertex_mapping": {"P": "ES", "C": "CL", "R": "ZN"},
            "note": "固定级别 σ（L1/L2/L3）+ 最高涌现级别 σ；预热 2010→记录 2020-2022",
            "epistemological_level": "L2",
        },
        "edges": {r["name"]: r for r in results},
    }
    CACHE_OUT.write_text(json.dumps(out, ensure_ascii=False))
    for r in results:
        days = r["days"]
        for lab, key in [("L1", "sigma_L1"), ("L2", "sigma_L2"),
                          ("L3", "sigma_L3"), ("high", "sigma_high")]:
            s = np.array(r[key])
            print(f"  {r['name']:5s} {lab:4s}: +1={int((s==1).sum())} "
                  f"0={int((s==0).sum())} -1={int((s==-1).sum())} "
                  f"mean={s.mean():+.2f}", flush=True)
    print(f"\n总耗时：{time.time()-t_start:.0f}s　缓存：{CACHE_OUT}", flush=True)


if __name__ == "__main__":
    main()
