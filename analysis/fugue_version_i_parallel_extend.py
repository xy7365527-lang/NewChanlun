#!/usr/bin/env python3
"""完成遗留任务（team 30cd6153 / task 3）：fugue version_i 8标的回测的并行扩展。

前序 session 已完成 OKLO + QQQ（2标的，见 fugue_version_i_results.json），
中断时遗留 task 3 (in_progress)。本 runner 完成剩余 6 标的 ES/GC/CL/ZN/6E/BRN，
满足任务规格『6边并行』，**保留** OKLO/QQQ（不覆盖、不重算）。

诚实标注：本任务属 fugue version_i 工作流，非 K4 1min 资本旋转工作流。
process_symbol 来自 fugue_version_i.py（信号层 RecursiveOrchestrator max_levels=8）。
认识论 L2（真实 1min 数据，单标的回测跨标的对照）。

用法：PYTHONPATH=src python analysis/fugue_version_i_parallel_extend.py [--procs 6] [--only SYM,SYM]
"""

from __future__ import annotations

import argparse
import json
import multiprocessing as mp
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
for p in (str(ROOT), str(ROOT / "src")):
    if p not in sys.path:
        sys.path.insert(0, p)

from analysis.fugue_version_i import DATA_DIR, _write_report, process_symbol

RESULTS_JSON = DATA_DIR / "fugue_version_i_results.json"
# 任务规格的 8 标的顺序；OKLO/QQQ 已完成
ALL_SYMBOLS = ["OKLO", "QQQ", "BRN", "ZN", "GC", "CL", "ES", "6E"]
REMAINING = ["BRN", "ZN", "GC", "CL", "ES", "6E"]


def _worker(symbol: str) -> tuple[str, dict]:
    """multiprocessing 顶层 worker。"""
    return process_symbol(symbol)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--procs", type=int, default=6)
    ap.add_argument("--only", type=str, default="", help="仅跑指定标的(逗号分隔)")
    args = ap.parse_args()

    # 载入已完成结果（保留 OKLO/QQQ）
    results: dict = {}
    if RESULTS_JSON.exists():
        results = json.loads(RESULTS_JSON.read_text())
        print(f"已载入既有结果：{list(results.keys())}")

    todo = ([s.strip().upper() for s in args.only.split(",")] if args.only
            else [s for s in REMAINING if s not in results])
    if not todo:
        print("无待跑标的（全部已完成）。仅重写报告。")
        _write_report(results)
        return

    print(f"并行扩展 {len(todo)} 标的：{todo}（procs={args.procs}）")
    t0 = time.time()
    ctx = mp.get_context("spawn")
    with ctx.Pool(processes=min(args.procs, len(todo))) as pool:
        for sym, out in pool.imap_unordered(_worker, todo):
            results[sym] = out
            # 增量持久化（每个标的完成即写，崩溃可续）
            RESULTS_JSON.write_text(
                json.dumps(results, indent=2, ensure_ascii=False, default=str))
            _write_report(results)
            print(f"  ✓ {sym} 完成并落盘（{time.time()-t0:.0f}s）", flush=True)

    # 按规格顺序重排
    ordered = {s: results[s] for s in ALL_SYMBOLS if s in results}
    ordered.update({s: results[s] for s in results if s not in ordered})
    RESULTS_JSON.write_text(json.dumps(ordered, indent=2, ensure_ascii=False, default=str))
    _write_report(ordered)
    print(f"\n总耗时 {time.time()-t0:.0f}s；最终标的：{list(ordered.keys())}")
    print("报告：fugue_version_i_results.md / .json")


if __name__ == "__main__":
    main()
