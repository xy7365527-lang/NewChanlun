"""OKLO 447k sanity：新 O(N) 引擎 E+I 测速 + E 回测数值复现。

判据（比值法，消除机器负载差异）：
  旧引擎 I≈5×E（期货实测 ES 188.9min I / 38.5min E ≈ 4.9×）。
  新引擎若 segment 增量化生效 → I/E 比值应降到 ~1-2×。

E 数值复现：与旧引擎落盘基线对比（bit-exact 守卫见 _verify_* 脚本）。
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402
import fugue_version_i_slice as VI  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"


def load():
    raw = json.loads(DATA.read_text())
    bars = raw["bars"]
    o = [float(b["open"]) for b in bars]
    h = [float(b["high"]) for b in bars]
    l = [float(b["low"]) for b in bars]
    c = [float(b["close"]) for b in bars]
    return o, h, l, c


def main() -> None:
    o, h, l, c = load()
    n = len(c)
    bh = (c[-1] - c[0]) / c[0] * 100
    print(f"OKLO sanity: {n:,} bars  BH={bh:+.2f}%")

    # ── E ──
    t0 = time.time()
    e_sig = RE.compute_e_signals_rust(o, h, l, c)
    e_t = time.time() - t0
    trades, _ = F.run_swing_trading(e_sig, F.MODE_NONE)
    m = F.compute_metrics(trades)
    print(f"E: signal {e_t:.1f}s  compound={m['total_compound']:+.4f}%  "
          f"n_trades={len(trades)}  (基线 932.77% / 7笔)")

    # ── I ──
    t1 = time.time()
    i_sig = compute_i_signals_rust(o, h, l, c)
    i_t = time.time() - t1
    tr, _ = VI.run_version_i(i_sig, floor_ladder=VI.MIN_FLOOR_LADDER, stop_mode="none")
    mi = VI.extended_metrics(tr)
    print(f"I: signal {i_t:.1f}s  I_none compound={mi['total_compound']:+.4f}%  "
          f"n_trades={mi['n']}")

    ratio = i_t / e_t if e_t else float("inf")
    print(f"\n判据: I/E 耗时比 = {ratio:.2f}×  "
          f"(旧引擎≈5×;<2.5× 视为 segment 增量化生效→O(N))")
    verdict = "PASS (O(N) 生效)" if ratio < 2.5 else "FAIL (O(N²)墙仍在)"
    print(f"裁决: {verdict}")


if __name__ == "__main__":
    main()
