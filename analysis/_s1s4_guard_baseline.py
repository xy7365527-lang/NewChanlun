"""S1-S4 双向回测的 fusion_tr 零接触守卫基线 — BTC+CL 在册复现预跑。

软租约说明（task 6 子集，feedback_task_queue_owner_liveness 协议）：
task 6（BTC+CL 回测 S1-S4 vs fusion_tr 对照）的守卫前半段——"fusion_tr 复现
在册数字=零接触守卫"——不依赖 P3/P4（fusion_btr 实装）。当前安装的
newchan_rust 扩展是在册版本（P3 改动在原主工作区未编译），现在跑出的数字
即守卫参照。本脚本独立于 task 6 指定文件名（bidirectional_s1s4_backtest.py
留给原主/task 6 执行者），避免 NRF 同型碰撞。

用途：
1. 确认当前扩展未漂移（hold26/fusion_t/fusion_tr 对照 p7_conj_summary 在册值）；
2. P3 实装编译后，task 6 执行者重跑本脚本——数字逐项一致 = short_mask=0
   全在册路径零接触验证通过。

在册参照（p7_conj_summary.json）：
  BTC: hold26=1247.1 fusion_t=4174.6 fusion_tr=4298.4 (fa 2997.0, r2_blocks=61)
  CL : hold26=274.9  fusion_t=351.x  fusion_tr=331.7  (fa 167.1,  r2_blocks=64)

用法：PYTHONPATH=src .venv/bin/python analysis/_s1s4_guard_baseline.py
输出：data_cache/s1s4_guard_baseline.json（含 tape_fp + 全 analyze 面）
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from p6_phase_machine_backtest import analyze, bh_mdd  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUT = CACHE / "s1s4_guard_baseline.json"
MODES = ["hold26", "fusion_t", "fusion_tr"]
GUARD_TOL = 0.05   # pp，与 p7 守卫同容差

BOOK = json.loads((CACHE / "p7_conj_summary.json").read_text())
BOOK_ROWS = {r["symbol"]: r for r in BOOK["rows"]}


def run(sym: str) -> dict:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[sym])
    n = len(closes)
    print(f"[{sym}] bars={n:,}", flush=True)
    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time()-t0:.1f}s fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    book = BOOK_ROWS[sym]
    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round((closes[-1] / closes[0] - 1) * 100, 2),
           "bh_mdd_pct": round(bh_mdd(closes) * 100, 2),
           "modes": {}, "guard": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=LADDER_SEG, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        book_val = book.get(mode)
        ok = (book_val is not None
              and abs(a["strat_pct"] - book_val) <= GUARD_TOL)
        out["guard"][mode] = {"book": book_val, "run": a["strat_pct"],
                              "pass": bool(ok)}
        print(f"[{sym}][{mode}] {time.time()-t1:.1f}s "
              f"strat={a['strat_pct']:+.2f}% book={book_val} "
              f"guard={'PASS' if ok else 'FAIL'}", flush=True)
        if not ok:
            raise RuntimeError(
                f"守卫失败 {sym}/{mode}: run={a['strat_pct']} book={book_val} "
                f"——扩展已漂移或数据文件变化，停止（不带错基线传递下游）")
    return out


def main() -> None:
    results = {"design": "task6 fusion_tr 零接触守卫基线（在册扩展预跑）",
               "guard_tol_pp": GUARD_TOL, "symbols": {}}
    for sym in (sys.argv[1:] or ["BTC", "CL"]):
        results["symbols"][sym] = run(sym)
    OUT.write_text(json.dumps(results, ensure_ascii=False, indent=1))
    print(f"✓ 落盘 {OUT}", flush=True)


if __name__ == "__main__":
    main()
