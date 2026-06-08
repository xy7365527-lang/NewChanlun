"""定位 current_bi_zhongshu_buysellpoints 是否为 wall-clock 主导项（数据驱动诊断）。

复刻 compute_i_signals_rust 的内层循环，但对四个 Rust 调用分别累计计时：
  1. process_bar               (每 bar)
  2. current_bi_zhongshu_buysellpoints  (仅 stroke 增长时)  ← 疑似 O(S²) 源
  3. current_buysellpoints     (每 bar)
  4. current_recursive         (每 bar)

并采样 (2) 的单次耗时随笔序号的增长曲线 —— 若 per-call 成本随 stroke_count 线性增长，
即确认 O(S²) 签名（总和 = Σ per-call ∝ S²）。

用法: PYTHONPATH=src python analysis/_profile_bi_zhongshu_bottleneck.py <symbol> <max_bars>
认识论等级: L2 性能诊断（真实数据，wall-clock 主导项是可证伪的经验问题）。
"""
from __future__ import annotations

import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

MAX_LEVELS = 8
DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOL_FILES = {
    "ES": "es_1m_databento_10y.json",
    "GC": "gc_1m_databento_10y.json",
    "CL": "cl_1m_databento_10y.json",
    "ZN": "zn_1m_databento_10y.json",
}


def load_ohlc(path: Path, max_bars: int):
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    if max_bars > 0:
        o_in, h_in, l_in, c_in = o_in[:max_bars], h_in[:max_bars], l_in[:max_bars], c_in[:max_bars]
    o, h, l, c = [], [], [], []
    for i in range(len(c_in)):
        a, b, d, e = float(o_in[i]), float(h_in[i]), float(l_in[i]), float(c_in[i])
        if math.isnan(a) or math.isnan(b) or math.isnan(d) or math.isnan(e):
            continue
        if a <= 0 or b <= 0 or d <= 0 or e <= 0:
            continue
        o.append(a); h.append(b); l.append(d); c.append(e)
    return o, h, l, c


def main() -> None:
    symbol = sys.argv[1] if len(sys.argv) > 1 else "GC"
    max_bars = int(sys.argv[2]) if len(sys.argv) > 2 else 150_000
    path = DATA_DIR / SYMBOL_FILES[symbol]

    t_load = time.time()
    opens, highs, lows, closes = load_ohlc(path, max_bars)
    n = len(closes)
    print(f"[{symbol}] loaded {n} bars in {time.time()-t_load:.1f}s")

    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)

    t_pb = t_bz = t_cb = t_rec = 0.0
    n_bz_calls = 0
    last_stroke_n = 0
    # 采样曲线: (stroke_count, per_call_seconds)
    bz_curve: list[tuple[int, float]] = []
    cb_curve: list[tuple[int, float]] = []

    t_wall = time.time()
    for i in range(n):
        c = closes[i]

        s = time.perf_counter()
        orch.process_bar(opens[i], highs[i], lows[i], c)
        t_pb += time.perf_counter() - s

        sc = orch.stroke_count()
        if sc > last_stroke_n:
            last_stroke_n = sc
            s = time.perf_counter()
            _ = orch.current_bi_zhongshu_buysellpoints(BI_ZHONGSHU_LEVEL_ID)
            dt = time.perf_counter() - s
            t_bz += dt
            n_bz_calls += 1
            if n_bz_calls % 50 == 0:
                bz_curve.append((sc, dt))

        s = time.perf_counter()
        _ = orch.current_buysellpoints()
        dt_cb = time.perf_counter() - s
        t_cb += dt_cb
        if i % 20000 == 0:
            cb_curve.append((i, dt_cb))

        s = time.perf_counter()
        _ = orch.current_recursive()
        t_rec += time.perf_counter() - s

    wall = time.time() - t_wall
    print(f"\n=== wall-clock 分解 ({n} bars, {last_stroke_n} strokes, wall={wall:.1f}s) ===")
    print(f"  process_bar               : {t_pb:8.2f}s  ({100*t_pb/wall:5.1f}%)")
    print(f"  current_bi_zhongshu_bsp   : {t_bz:8.2f}s  ({100*t_bz/wall:5.1f}%)  [{n_bz_calls} calls]")
    print(f"  current_buysellpoints     : {t_cb:8.2f}s  ({100*t_cb/wall:5.1f}%)")
    print(f"  current_recursive         : {t_rec:8.2f}s  ({100*t_rec/wall:5.1f}%)")

    print(f"\n=== bi_zhongshu_bsp 单次耗时随笔数增长 (O(S²) 签名检验) ===")
    print(f"  {'stroke_count':>12} {'per_call_ms':>12}")
    for sc, dt in bz_curve:
        print(f"  {sc:>12} {dt*1000:>12.3f}")
    print(f"\n=== current_buysellpoints 单次耗时随 bar 增长 (每 bar 调用, O(N²)? 检验) ===")
    print(f"  {'bar_idx':>12} {'per_call_ms':>12}")
    for bi, dt in cb_curve:
        print(f"  {bi:>12} {dt*1000:>12.4f}")

    if len(bz_curve) >= 2:
        (s0, d0), (s1, d1) = bz_curve[0], bz_curve[-1]
        if d0 > 0 and s1 > s0:
            ratio_t = d1 / d0
            ratio_s = s1 / s0
            print(f"\n  笔数 ×{ratio_s:.1f} → 单次耗时 ×{ratio_t:.1f}  "
                  f"(线性=O(S)→比值应≈笔数比; 即总成本O(S²))")


if __name__ == "__main__":
    main()
