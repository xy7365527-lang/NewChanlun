"""CL 信号层性能诊断（一次性脚本）。

模式：
  scale  — 前 N bar 跑 compute_organic_signals，进度行带时间戳（判定超线性）
  prof   — cProfile 前 N bar，输出 tottime/cumtime Top 30（定位热点函数）

用法：PYTHONPATH=src .venv/bin/python analysis/_profile_cl_signals.py scale 1600000
"""
from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

MODE = sys.argv[1]
N = int(sys.argv[2])

t0 = time.time()
opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["CL"])
print(f"loaded CL bars={len(closes):,} in {time.time()-t0:.1f}s", flush=True)
opens, highs, lows, closes = opens[:N], highs[:N], lows[:N], closes[:N]

# 给进度打印加时间戳：包装 stdout（compute 内部 print 每 200K bar 一行）
class _TsOut:
    def __init__(self, raw):
        self.raw = raw
        self.t0 = time.time()
        self.last = self.t0

    def write(self, s):
        if "organic signal bar" in s:
            now = time.time()
            s = s.rstrip("\n") + f"  [+{now-self.last:.1f}s 累计{now-self.t0:.1f}s]\n"
            self.last = now
        self.raw.write(s)

    def flush(self):
        self.raw.flush()


if MODE == "scale":
    sys.stdout = _TsOut(sys.stdout)
    t1 = time.time()
    sigs = compute_organic_signals(opens, highs, lows, closes)
    print(f"done n={len(sigs):,} total={time.time()-t1:.1f}s", flush=True)
elif MODE == "prof":
    import cProfile
    import pstats

    pr = cProfile.Profile()
    pr.enable()
    compute_organic_signals(opens, highs, lows, closes)
    pr.disable()
    st = pstats.Stats(pr)
    print("===== tottime top30 ====="); st.sort_stats("tottime").print_stats(30)
    print("===== cumtime top30 ====="); st.sort_stats("cumulative").print_stats(30)
else:
    raise SystemExit(f"unknown mode {MODE}")
