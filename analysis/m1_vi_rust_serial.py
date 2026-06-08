"""M1 Version I 完整版回测 — 7 标的 1min 10 年期货全量（Rust 驱动，**严格串行**）。

存在论位置：`m1_i_rust_backtest.py` 的串行孪生。并行版在 7 期货并发时 OOM 崩溃
（free 仅 26MB）——6E 单文件 620MB，解析为 Python float list 后膨胀至数 GB，7 进程
同时驻留必然耗尽内存。本 runner **一次只跑一个标的**，复用并行版的 `run_symbol`
（纯函数，内部 `del e_signals/i_signals` 主动释放），标的间 `gc.collect()` 回收。

与并行版的差异（仅 I/O 与调度，信号层逐位复用 `run_symbol`，无算法改动）：
  1. **无 multiprocessing**：单进程顺序遍历，峰值内存 = 单标的（非 7×单标的）。
  2. **每标的独立 JSON**（`m1_vi_<sym>_serial.json`）：append-only 语义，崩溃只丢
     正在跑的标的，已完成标的的独立文件不被覆写破坏（聚合 JSON 覆写有此风险）。
  3. **汇总 MD** 输出到 `m1_vi_rust_results_serial.md`（重绑定 `M.OUT_MD`，复用 `_write_report`）。
  4. **断点续跑**：标的独立 JSON 已存在 → 跳过（崩溃后直接重启即可）。

顺序（用户指令）：ES → GC → CL → ZN → DX → BRN → 6E。
每标的 4 组核心对比（run_symbol 实际跑 E + 3 floor × 3 stop = E + 9 变体，覆盖
用户要求的 E / I / I+stopA / I+stopB）。

认识论：移植正确性 L0/L1（管线 bit-exact，承自 run_symbol）；回测结论 L2（真实数据）。
"""

from __future__ import annotations

import gc
import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import m1_i_rust_backtest as M  # noqa: E402

# 串行顺序（用户指令）。6E 真实文件名为 usd6e_*（SYMBOL_FILES 已正确映射）。
SERIAL_ORDER = ["ES", "GC", "CL", "ZN", "DX", "BRN", "6E"]

OUT_MD = ROOT / "analysis" / "m1_vi_rust_results_serial.md"
PER_SYM_JSON = lambda sym: ROOT / "analysis" / f"m1_vi_{sym.lower()}_serial.json"  # noqa: E731
AGG_JSON = ROOT / "analysis" / "m1_vi_rust_results_serial.json"

# 复用并行版报表逻辑，但重绑定输出路径到 serial 专属文件。
M.OUT_MD = OUT_MD


def _load_done() -> dict:
    """从每标的独立 JSON 重建已完成结果（断点续跑）。"""
    results: dict = {}
    for sym in SERIAL_ORDER:
        p = PER_SYM_JSON(sym)
        if p.exists():
            try:
                results[sym] = json.loads(p.read_text())
            except Exception:
                pass  # 损坏（崩溃于写盘中途）→ 视为未完成，重跑
    return results


def main() -> None:
    order = SERIAL_ORDER
    only = os.environ.get("BT_SYMBOLS")
    if only:  # 覆盖顺序（冒烟测试 / 崩溃后单标的重跑）
        order = [s.strip().upper() for s in only.split(",") if s.strip()]
    symbols = [s for s in order if s in M.SYMBOL_FILES and M.SYMBOL_FILES[s].exists()]
    missing = [s for s in order if s not in M.SYMBOL_FILES or not M.SYMBOL_FILES[s].exists()]
    if missing:
        print(f"⚠ 数据文件缺失，跳过：{missing}", flush=True)

    results = _load_done()
    pending = [s for s in symbols if s not in results]
    print(f"M1 Version I 串行回测（OOM-safe，单进程一次一标的）")
    print(f"顺序：{symbols}")
    print(f"待跑：{pending}（已完成：{list(results)}）\n", flush=True)

    t_all = time.time()
    for sym in pending:
        t0 = time.time()
        print(f"\n{'=' * 64}\n▶ {sym} 开始（{M.SYMBOL_FILES[sym].name}）\n{'=' * 64}", flush=True)
        _, out = M.run_symbol(sym)
        results[sym] = out

        # 1) 每标的独立 JSON（防崩溃丢数据，先写最关键的单标的产物）
        PER_SYM_JSON(sym).write_text(
            json.dumps(out, indent=2, ensure_ascii=False, default=str))
        # 2) 聚合 JSON + 汇总 MD（增量覆写）
        AGG_JSON.write_text(
            json.dumps(results, indent=2, ensure_ascii=False, default=str))
        M._write_report(results)

        gc.collect()  # 标的间强制回收大 list，压低跨标的峰值内存
        print(f"  ✓ {sym} 完成并已写盘（本标的 {time.time()-t0:.0f}s，累计 "
              f"{time.time()-t_all:.0f}s）→ {PER_SYM_JSON(sym).name}", flush=True)

    print(f"\n总耗时 {time.time()-t_all:.0f}s（{len(pending)} 标的）")
    print(f"汇总报告：{OUT_MD}")
    print(f"聚合 JSON：{AGG_JSON}")
    print(f"每标的 JSON：m1_vi_<sym>_serial.json", flush=True)


if __name__ == "__main__":
    main()
