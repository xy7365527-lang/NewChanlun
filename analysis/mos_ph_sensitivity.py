"""中枢/级别对噪声阈 τ 的敏感性扫描 —— 验证"中枢=0"结论的边界条件。

formalization-validity-domain：中枢计数依赖 τ。报告必须给出结论翻转的 τ 边界。
"""
from __future__ import annotations
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf  # noqa: E402
from newchan.a_persistence_barcode import atr, atr_noise_threshold  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_level_detection import detect_levels  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402

df = yf.download("MOS", period="1y", interval="1d", auto_adjust=False, progress=False)
closes = [float(x) for x in df["Close"].to_numpy().ravel()]
highs = [float(x) for x in df["High"].to_numpy().ravel()]
lows = [float(x) for x in df["Low"].to_numpy().ravel()]
atr14 = atr(highs, lows, closes, period=14)
print(f"ATR14 = {atr14:.4f}   n={len(closes)}")

tree = OnlineMergeTree()
for p in closes:
    tree.update(p)
mbars = list(tree.finalize().all_bars)

print("\n=== τ 敏感性扫描 (multiple × ATR) ===")
print(f"{'mult':>5} {'tau':>7} {'#levels':>8} {'#zhongshu(overlap)':>18} {'#zhongshu(no-overlap)':>21}")
for mult in [0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0]:
    tau = mult * atr14
    lvl = detect_levels(mbars, bars_per_day=1.0, noise_floor=tau)
    zs_o = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                           policy=ZhongshuPolicy(min_members=3, same_level_ratio=3.0,
                                                 require_overlap=True))
    zs_n = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                           policy=ZhongshuPolicy(min_members=3, same_level_ratio=3.0,
                                                 require_overlap=False))
    print(f"{mult:>5.2f} {tau:>7.3f} {len(lvl.levels):>8} {len(zs_o):>18} {len(zs_n):>21}")

print("\n=== same_level_ratio 放宽 (τ=0.5×ATR, overlap=True) ===")
tau = 0.5 * atr14
for ratio in [3.0, 5.0, 8.0, 12.0]:
    zs = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                         policy=ZhongshuPolicy(min_members=3, same_level_ratio=ratio,
                                               require_overlap=True))
    detail = [(z.period_label, z.n_members, round(z.zd, 1), round(z.zg, 1),
               z.lo, z.hi) for z in zs]
    print(f"ratio={ratio:>4}: #zhongshu={len(zs)}  {detail}")
