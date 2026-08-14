# FROZEN(#951)：绑 recursive_t.RecTStream.finish_full 九字段；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""TC 判别特征探针（#170/R4）：margin<0 的腿中，TC vs 盈利腿的区别。

发现：50-58% 多腿在 margin<0（开仓价已破 ZD 否定线）处开仓，但只有 ~15% 是 TC。
问题：margin<0 不是 TC 判别（会误杀盈利腿）。真正的判别是什么？

候选判别量（逐笔，可证伪）：
  A. hold：TC=1，盈利腿 hold>1（但 hold 是结果不是开仓时可知 ⇒ 不可作开仓 gate）
  B. 开仓后下一 bar 即被止损 vs 存活：等价于 hold≤1，同 A 问题
  C. margin<0 的腿整体盈亏：若 margin<0 腿总体亏损 ⇒ gate 仍有价值（频率可约=拒掉净亏的一批）
     若 margin<0 腿总体盈利 ⇒ gate 杀盈利腿 ⇒ 不是 TC 修复

核心拷问（no-workaround）：「开仓价已破否定线」这个状态在开仓时是否可知？
  - ZD = view.zd[k]（开仓 bar 的中枢下沿），c = 开仓价 ⇒ margin=(c-ZD)/c 开仓时完全可知（纯因果）。
  - 故 margin<0 是合法 gate（无 look-ahead）。问题仅是判别力（是否只杀 TC 不杀盈利）。

本探针：把所有有否定线的多腿按 margin 符号 + 盈亏 四分，量化各格 pnl。
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import newchan_rust as nr  # noqa: F401

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "analysis"))
from capture_ratio_matrix import (  # noqa: E402
    load_clean_ohlc, run_face_a,
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
    res = run_face_a(o, h, l, c, mode)
    trades = res["leg_trades"]
    stops = res["leg_entry_stops"]

    # 四分：margin符号 × pnl符号（仅多腿，有否定线）
    quad = {("neg", "loss"): [0, 0.0], ("neg", "win"): [0, 0.0],
            ("pos", "loss"): [0, 0.0], ("pos", "win"): [0, 0.0]}
    # margin<0 腿中按 hold 分（hold=1 即 TC 候选）
    neg_hold1 = [0, 0.0]
    neg_holdN = [0, 0.0]
    for (k, eb, xb, ep, xp, u, is_short, pnl), stop in zip(trades, stops):
        if is_short or not np.isfinite(stop):
            continue
        margin = (ep - stop) / ep
        msign = "neg" if margin < 0 else "pos"
        psign = "win" if pnl > 0 else "loss"
        quad[(msign, psign)][0] += 1
        quad[(msign, psign)][1] += pnl
        if margin < 0:
            hold = xb - eb
            if hold <= 1:
                neg_hold1[0] += 1
                neg_hold1[1] += pnl
            else:
                neg_holdN[0] += 1
                neg_holdN[1] += pnl

    print(f"\n===== [{sym}] 多腿 margin符号 × 盈亏 四分 =====")
    for key in [("neg", "loss"), ("neg", "win"), ("pos", "loss"), ("pos", "win")]:
        n, p = quad[key]
        print(f"  margin{key[0]:>3} / {key[1]:>4}: {n:5d} 笔  pnl={p:+12.2f}")
    print(f"  --- margin<0 腿按 hold 分 ---")
    print(f"  margin<0 & hold<=1 (TC): {neg_hold1[0]:5d} 笔  pnl={neg_hold1[1]:+12.2f}")
    print(f"  margin<0 & hold >1     : {neg_holdN[0]:5d} 笔  pnl={neg_holdN[1]:+12.2f}")
    neg_total_n = neg_hold1[0] + neg_holdN[0]
    neg_total_p = neg_hold1[1] + neg_holdN[1]
    print(f"  margin<0 合计          : {neg_total_n:5d} 笔  pnl={neg_total_p:+12.2f}")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        probe(sym, fname)
