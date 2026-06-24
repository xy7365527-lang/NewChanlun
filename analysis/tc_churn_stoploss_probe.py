"""TC=否定线止损 collapse 深度探针（#170/R4）。

L3 实证否证了 #164「TC=买卖点反向 churn」假设：TC 100% reason=1（否定线止损）。
本探针验证因果：TC 笔 = 开多腿后 1 bar 内价格跌破开仓锁定的 ZD 否定线。

关键问题（决定可约性）：
  Q1. TC 笔开仓价 ep 距 ZD 否定线有多近？（近=买点在临近否定线处开多=开仓时机差）
  Q2. bar N+1 的跌破幅度（穿透 ZD 多深）？collapse 是浅穿（噪声）还是深穿（真破）？
  Q3. 这些买点是哪一类（type1/type3/普通）？(无法直接读 view，用级别+方向间接)

认识论：L3。复用引擎 leg_trades + close_reasons + 重算 ZD（需引擎暴露 long_stop，
此处用 entry_px 与 exit_px 关系间接推断：止损平 ⟹ exit_px(bar N+1 close) < ZD ≤ entry 中枢）。
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import newchan_rust as nr  # noqa: F401

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "analysis"))
from capture_ratio_matrix import (  # noqa: E402
    load_clean_ohlc, run_face_a, per_trade_capture, classify_failure_modes,
)

MAIN_DATA = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")
SYMBOLS = [
    ("OKLO", "oklo_1m_databento.json"),
    ("BTC", "btc_1m_full.json"),
    ("CL", "cl_1m_databento_10y.json"),
]


def probe(sym: str, fname: str, mode: str = "structural") -> None:
    path = MAIN_DATA / fname
    if not path.exists():
        print(f"[{sym}] data missing")
        return
    o, h, l, c = load_clean_ohlc(path)
    c_arr = np.array(c)
    res = run_face_a(o, h, l, c, mode)
    trades = res["leg_trades"]
    reasons = res["leg_close_reasons"]

    rows = per_trade_capture(res, h, l, c)
    r_le0 = [x for x in rows if x["r"] <= 0]
    fm = classify_failure_modes(r_le0, res, h, l, c)
    tc_coords = fm["per_class"]["TC_瞬时churn"]["coords"]
    tc_keys = {(lvl, d, eb, xb) for (lvl, d, eb, xb, *_) in tc_coords}

    # 对 TC 笔（reason=1 止损）量化跌幅 = (entry_px - exit_px) / entry_px
    drops = []
    n_long = 0
    n_short = 0
    for (k, eb, xb, ep, xp, u, is_short, pnl), rsn in zip(trades, reasons):
        d = "short" if is_short else "long"
        if (int(k), d, int(eb), int(xb)) not in tc_keys:
            continue
        if is_short:
            n_short += 1
            drop = (xp - ep) / ep  # 空头止损=涨破，xp>ep
        else:
            n_long += 1
            drop = (ep - xp) / ep  # 多头止损=跌破，xp<ep ⇒ drop>0
        drops.append(abs(drop))
    if not drops:
        print(f"[{sym}] 无 TC")
        return
    drops = np.array(drops)
    print(f"\n===== [{sym}] TC=否定线止损 跌幅分布 (n={len(drops)}, long={n_long} short={n_short}) =====")
    print(f"  |穿透幅度|/entry: min={drops.min()*1e4:.2f}bps  p25={np.percentile(drops,25)*1e4:.2f}bps  "
          f"median={np.median(drops)*1e4:.2f}bps  p75={np.percentile(drops,75)*1e4:.2f}bps  max={drops.max()*1e4:.2f}bps")
    print(f"  浅穿 <5bps: {100*np.mean(drops<5e-4):.1f}%  | 深穿 >50bps: {100*np.mean(drops>50e-4):.1f}%")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        probe(sym, fname)
