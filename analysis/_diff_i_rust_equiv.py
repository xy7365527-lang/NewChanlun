"""Differential test：Rust 驱动 I 信号 vs Python compute_signals_i 的 i_signals。

no-patch-mentality 硬门：PnL 相关字段（buy1/sell1/sell_any/buy_any/max_ladder）必须逐位
等价。type2_buy 为报告口径（仅 addon 计数，不进 PnL），单列对比，不作等价门。
"""
import sys
import time

sys.path.insert(0, "src")
sys.path.insert(0, "analysis")

import json

from fugue_version_i import compute_signals_i
from m1_i_rust_engine import compute_i_signals_rust

N = int(sys.argv[1]) if len(sys.argv) > 1 else 60000

raw = json.load(open("analysis/data_cache/oklo_1m_databento.json"))
bars = raw["bars"][:N]
o = [float(b["open"]) for b in bars]
h = [float(b["high"]) for b in bars]
l = [float(b["low"]) for b in bars]
c = [float(b["close"]) for b in bars]
print(f"differential test: OKLO 前 {N:,} bar")

t0 = time.time()
_, py_i = compute_signals_i(o, h, l, c)
t_py = time.time() - t0
print(f"  Python compute_signals_i: {t_py:.1f}s")

t0 = time.time()
ru_i = compute_i_signals_rust(o, h, l, c)
t_ru = time.time() - t0
print(f"  Rust  compute_i_signals_rust: {t_ru:.1f}s  (加速 {t_py / max(t_ru, 1e-9):.1f}×)")

assert len(py_i) == len(ru_i) == N, (len(py_i), len(ru_i), N)

mism_pnl = 0
mism_max_ladder = 0
mism_type2 = 0
first_mism = None
for i in range(N):
    p = py_i[i]; r = ru_i[i]
    pnl_ok = (p.buy1 == r.buy1 and p.sell1 == r.sell1
              and p.sell_any == r.sell_any and p.buy_any == r.buy_any)
    if not pnl_ok:
        mism_pnl += 1
        if first_mism is None:
            first_mism = i
    if p.max_ladder != r.max_ladder:
        mism_max_ladder += 1
    if p.type2_buy != r.type2_buy:
        mism_type2 += 1

print(f"\n  PnL 字段不匹配 bar: {mism_pnl} / {N}")
print(f"  max_ladder 不匹配 bar: {mism_max_ladder} / {N}")
print(f"  type2_buy 不匹配 bar（报告口径，不进 PnL）: {mism_type2} / {N}")

if mism_pnl == 0 and mism_max_ladder == 0:
    print("\n  ✅ PnL 字段 + max_ladder 逐位等价 PASS")
else:
    print(f"\n  ❌ FAIL  首个不匹配 bar={first_mism}")
    if first_mism is not None:
        i = first_mism
        p = py_i[i]; r = ru_i[i]
        for name in ("buy1", "sell1", "sell_any", "buy_any"):
            pv = getattr(p, name); rv = getattr(r, name)
            if pv != rv:
                diff = [(k, pv[k], rv[k]) for k in range(len(pv)) if pv[k] != rv[k]]
                print(f"    {name}: py={pv} rust={rv} diff@ladder={diff}")
