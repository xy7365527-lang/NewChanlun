"""诊断：OKLO 上 V1f Δ(+607.5pp) > V2o Δ(+456.8pp) 但 rev 净现金 V2o 更高的来源。

假设：复利是逐 trade pnl 的乘积——同样的腿现金落在不同 trade（或同 trade 不同
权重基数）上复利贡献不同；另有 earning 相变时点差异（AmountConserving 改股数）。
输出逐 trade pnl 差异 top10 + earning 触达对比。纯诊断（简化版结果包）。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["OKLO"])
dir_flips: list = []
tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
rtape = pack_tape(tape, dir_flips=dir_flips)

res = {}
for v in ("V1f", "V2o"):
    res[v] = nr.run_organic_rust(rtape, v, floor_ladder=2, stop_mode="none", diag=True)

ta, tb = res["V1f"]["trades"], res["V2o"]["trades"]
print(f"trade 数: V1f={len(ta)} V2o={len(tb)}")
assert len(ta) == len(tb), "master 不动 ⇒ trade 边界应一致"
rows = []
for i, (x, y) in enumerate(zip(ta, tb)):
    assert (x[0], x[2]) == (y[0], y[2]), f"trade#{i} 边界不一致 {x[:4]} {y[:4]}"
    rows.append((i, x[0], x[2], x[4], y[4], y[4] - x[4]))
rows.sort(key=lambda r: abs(r[5]), reverse=True)
print("\ntop10 |pnl 差| trade（V2o − V1f，pp/trade）:")
print(f"{'#':>4} {'entry':>8} {'exit':>8} {'V1f%':>10} {'V2o%':>10} {'diff':>9}")
for i, eb, xb, a, b, d in rows[:10]:
    print(f"{i:>4} {eb:>8} {xb:>8} {a:>10.2f} {b:>10.2f} {d:>+9.2f}")

for v in ("V1f", "V2o"):
    c = res[v]["counters"]
    ne = c["n_earning_reached"]
    print(f"\n{v}: earning_reached={ne}  rev开={c['n_rev_open']}")
# 复利分解：哪些 trade 的差异主导 Δ 差距
import math  # noqa: E402
la = sum(math.log1p(x[4] / 100) for x in ta)
lb = sum(math.log1p(y[4] / 100) for y in tb)
print(f"\nlog 复利和: V1f={la:.4f} V2o={lb:.4f} → 终值比 {math.exp(lb - la):.4f}")
contrib = sorted(
    ((i, math.log1p(y[4] / 100) - math.log1p(x[4] / 100))
     for i, (x, y) in enumerate(zip(ta, tb))),
    key=lambda t: abs(t[1]), reverse=True)
print("log 复利差 top5 trade:", [(i, round(c, 4)) for i, c in contrib[:5]])
