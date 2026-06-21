"""ES 1s 信号层 type1 聚合探针（176 缺失诊断辅助，2026-06-15）。

只读：build tape（compute_organic_signals，require_settled=True，与 unn 同口径），
统计每 ladder 的 BSP 事件 kind 分布（type1/type2/type3，candidate vs confirmed 分列）
+ 每 ladder distinct settled move 数（up/down，238 复核）。

不跑引擎，不改任何状态。验证必然性前提：每级是否真有 type1 产出（旧诊断 §4 复算）。
"""
from __future__ import annotations

import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from organic_signals import compute_organic_signals  # noqa: E402
from unified_necessity_backtest import load_ohlc  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES  # noqa: E402

DATA = ROOT / "analysis" / "data_cache"
SYMBOL_FILES["ES1S"] = DATA / "es_1s_databento_1y.json"
MAXL = 11


def main() -> int:
    t0 = time.time()
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES["ES1S"])
    n = len(closes)
    print(f"bars={n:,}", flush=True)

    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=[], require_settled=True)
    print(f"tape built {time.time() - t0:.1f}s", flush=True)

    # 每 ladder：kind × confirmed 计数
    cand: list[Counter] = [Counter() for _ in range(MAXL)]
    conf: list[Counter] = [Counter() for _ in range(MAXL)]
    # 每 ladder：本 bar 出现过 type1（任一侧）的 bar 数（与 has_type1 同口径）
    bars_with_t1 = [0] * MAXL

    for sig in tape:
        evs = sig.bsp_events
        if not evs:
            continue
        for lad in range(MAXL):
            row = evs[lad] if lad < len(evs) else ()
            if not row:
                continue
            saw_t1 = False
            for e in row:
                kind = e[0]
                confirmed = e[3]
                (conf if confirmed else cand)[lad][(kind, e[1])] += 1
                if kind == "type1":
                    saw_t1 = True
            if saw_t1:
                bars_with_t1[lad] += 1

    names = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3", 6: "recL4", 7: "recL5"}
    print("\nladder | kind | candidate(side分列) | confirmed | bars_with_type1")
    for lad in range(2, MAXL):
        if not cand[lad] and not conf[lad]:
            continue
        nm = names.get(lad, str(lad))
        for kind in ("type1", "type2", "type3"):
            cc = {s: v for (k, s), v in cand[lad].items() if k == kind}
            fc = {s: v for (k, s), v in conf[lad].items() if k == kind}
            ct = sum(cc.values())
            ft = sum(fc.values())
            if ct or ft:
                print(f"  {lad}:{nm:9s} {kind}: cand={ct} {dict(cc)}  conf={ft} {dict(fc)}")
        print(f"  {lad}:{nm:9s} → bars_with_type1(任一侧)={bars_with_t1[lad]}")
    print(f"\n总耗时 {time.time() - t0:.1f}s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
