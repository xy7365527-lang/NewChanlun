# FROZEN(#951)：绑 recursive_t.RecTStream.finish_full 九字段；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""TC 笔平仓触发源拆解（#170/R4 修复点定位）。

把 TC 笔（hold≤1 且 r≤0 且对齐入场）按平仓触发源分类：
  reason 0 = 买卖点反向平(g_pair) → 频率可约（反向信号携零走势，1-bar collapse）
  reason 1 = 否定线止损(pair_stop_loss_step) → 结构破坏，不可约（开仓即破否定线=真实结构破坏）
  reason 2 = NAV强平/finish收尾 → 边界

这决定过滤点：reason 0 是修复靶子（抑制携零走势的反向平仓），reason 1/2 不动。

认识论：L3（真实数据每笔，可证伪）。
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
    CHURN_HOLD_BARS,
)

MAIN_DATA = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")
SYMBOLS = [
    ("OKLO", "oklo_1m_databento.json"),
    ("BTC", "btc_1m_full.json"),
    ("CL", "cl_1m_databento_10y.json"),
]
REASON_NAME = {0: "Bsp反向平", 1: "否定线止损", 2: "NAV强平/收尾"}


def analyze(sym: str, fname: str, mode: str = "structural") -> None:
    path = MAIN_DATA / fname
    if not path.exists():
        print(f"[{sym}] data missing")
        return
    o, h, l, c = load_clean_ohlc(path)
    res = run_face_a(o, h, l, c, mode)
    trades = res["leg_trades"]
    reasons = res["leg_close_reasons"]
    assert len(trades) == len(reasons), f"reason 错位 {len(trades)} vs {len(reasons)}"

    rows = per_trade_capture(res, h, l, c)
    r_le0 = [x for x in rows if x["r"] <= 0]
    fm = classify_failure_modes(r_le0, res, h, l, c)
    tc_coords = fm["per_class"]["TC_瞬时churn"]["coords"]
    # TC coords: (level, dir, eb, xb, hold, r, covered, pnl, xlc)
    tc_keys = {(lvl, d, eb, xb) for (lvl, d, eb, xb, *_) in tc_coords}

    # 把 reason 配对到 TC 笔（用 (level, is_short, eb, xb) 匹配）
    reason_dist = {0: 0, 1: 0, 2: 0}
    reason_pnl = {0: 0.0, 1: 0.0, 2: 0.0}
    for (k, eb, xb, ep, xp, u, is_short, pnl), rsn in zip(trades, reasons):
        d = "short" if is_short else "long"
        if (int(k), d, int(eb), int(xb)) in tc_keys:
            reason_dist[rsn] += 1
            reason_pnl[rsn] += pnl

    print(f"\n===== [{sym}] TC 笔平仓触发源拆解 (n_TC={len(tc_coords)}) =====")
    for rsn in (0, 1, 2):
        n = reason_dist[rsn]
        if len(tc_coords):
            print(f"  reason {rsn} ({REASON_NAME[rsn]}): {n} 笔 ({100*n/len(tc_coords):.1f}%) "
                  f"pnl={reason_pnl[rsn]:+.2f}")
    matched = sum(reason_dist.values())
    print(f"  匹配: {matched}/{len(tc_coords)}")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        analyze(sym, fname)
