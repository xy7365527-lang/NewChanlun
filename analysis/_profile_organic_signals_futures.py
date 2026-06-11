"""期货长序列信号层 O(N²) 诊断 profile（2026-06-11）。

方法：计时代理注入 organic_signals.R —— 包裹 RecursiveOrchestrator 全部方法
与模块级纯函数调用，BRN 真实数据前缀逐 bar 跑 compute_organic_signals，
每 200K bar 打印各方法累计耗时/调用次数。若某方法的**每窗口增量耗时**
随窗口序号增长 → 该方法是 O(N×B) 主导项。

用法：PYTHONPATH=src .venv/bin/python analysis/_profile_organic_signals_futures.py
      （env：PROF_SYMBOL=BRN PROF_BARS=1200000）
"""
from __future__ import annotations

import os
import sys
import time
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as _real_R  # noqa: E402

import organic_signals  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402

STATS_T: dict[str, float] = defaultdict(float)
STATS_N: dict[str, int] = defaultdict(int)
_WINDOW = 200_000
_last_snapshot: dict[str, float] = {}
_bar_count = 0
_t_start = 0.0


def _dump(label: str) -> None:
    global _last_snapshot
    rows = []
    for k, t in STATS_T.items():
        dt = t - _last_snapshot.get(k, 0.0)
        rows.append((dt, t, STATS_N[k], k))
    rows.sort(reverse=True)
    wall = time.perf_counter() - _t_start
    print(f"\n── {label}  wall={wall:.1f}s ──")
    print(f"{'Δwindow(s)':>11} {'cum(s)':>9} {'calls':>10}  method")
    for dt, t, n, k in rows:
        if t < 0.05:
            continue
        print(f"{dt:11.2f} {t:9.2f} {n:10,}  {k}")
    _last_snapshot = {k: t for k, t in STATS_T.items()}
    sys.stdout.flush()


def _timed(name: str, fn):
    def wrapper(*a, **kw):
        t0 = time.perf_counter()
        out = fn(*a, **kw)
        STATS_T[name] += time.perf_counter() - t0
        STATS_N[name] += 1
        return out
    return wrapper


class OrchProxy:
    """逐方法计时代理；process_bar 调用计数驱动窗口快照。"""

    def __init__(self, *a, **kw):
        self._o = _real_R.RecursiveOrchestrator(*a, **kw)
        self._cache: dict[str, object] = {}

    def __getattr__(self, name: str):
        w = self._cache.get(name)
        if w is None:
            fn = getattr(self._o, name)
            if name == "process_bar":
                inner = _timed("orch.process_bar", fn)

                def w(*a, **kw):  # noqa: ANN002
                    global _bar_count
                    out = inner(*a, **kw)
                    _bar_count += 1
                    if _bar_count % _WINDOW == 0:
                        _dump(f"bar {_bar_count:,}")
                    return out
            else:
                w = _timed(f"orch.{name}", fn)
            self._cache[name] = w
        return w


class RShim:
    RecursiveOrchestrator = OrchProxy

    def __getattr__(self, name: str):
        fn = getattr(_real_R, name)
        if callable(fn):
            return _timed(f"R.{name}", fn)
        return fn


def main() -> None:
    global _t_start
    sym = os.environ.get("PROF_SYMBOL", "BRN")
    cap = int(os.environ.get("PROF_BARS", "1200000"))
    o, h, l, c, _years = load_ohlc(SYMBOL_FILES[sym])
    n = min(cap, len(c))
    print(f"{sym}: {len(c):,} bars，profile 前 {n:,}")

    organic_signals.R = RShim()
    for fname in ("_scan_events_rust", "_scan_div_events",
                  "_new_settled_up_moves", "_level_bsps_with_divs"):
        setattr(organic_signals, fname,
                _timed(f"py.{fname}", getattr(organic_signals, fname)))

    _t_start = time.perf_counter()
    sigs = organic_signals.compute_organic_signals(o[:n], h[:n], l[:n], c[:n])
    _dump("FINAL")
    print(f"\ntotal wall: {time.perf_counter() - _t_start:.1f}s  "
          f"signals={len(sigs):,}")


if __name__ == "__main__":
    main()
