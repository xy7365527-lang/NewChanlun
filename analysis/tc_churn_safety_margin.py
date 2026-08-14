# FROZEN(#951)：绑 recursive_t.RecTStream.finish_full 九字段；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""TC 笔开仓安全垫探针（#170/R4 修复点严格定位）。

验证严格假设：TC 笔 = 买点在贴近否定线 ZD 处开多（开仓价 c 距 ZD 的安全垫极小），
下一 bar 噪声级波动即穿 ZD 全量止损 collapse。

安全垫 margin = (entry_px - ZD) / entry_px （多腿；空腿对称 (ZG - entry_px)/entry_px）。
若 margin 极小（贴否定线）⇒ 开仓即无安全空间 ⇒ 该买点不应开仓（频率可约修复点）。

对照：非 TC 的对齐入场腿（T2/T3，hold>1）的安全垫分布——若 TC margin 显著小于
非 TC，则「贴否定线」是 TC 的判别特征（修复 gate 有判别力，不误杀 T2/T3）。

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
)

MAIN_DATA = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")
SYMBOLS = [
    ("OKLO", "oklo_1m_databento.json"),
    ("BTC", "btc_1m_full.json"),
    ("CL", "cl_1m_databento_10y.json"),
]


def margin_of(ep, stop, is_short):
    if not np.isfinite(stop):
        return None
    if is_short:
        return (stop - ep) / ep  # ZG > ep，安全垫=涨破前空间
    return (ep - stop) / ep      # ZD < ep，安全垫=跌破前空间


def probe(sym: str, fname: str, mode: str = "structural") -> None:
    path = MAIN_DATA / fname
    if not path.exists():
        print(f"[{sym}] data missing")
        return
    o, h, l, c = load_clean_ohlc(path)
    res = run_face_a(o, h, l, c, mode)
    trades = res["leg_trades"]
    reasons = res["leg_close_reasons"]
    stops = res["leg_entry_stops"]

    rows = per_trade_capture(res, h, l, c)
    r_le0 = [x for x in rows if x["r"] <= 0]
    fm = classify_failure_modes(r_le0, res, h, l, c)
    tc_keys = {(lvl, d, eb, xb) for (lvl, d, eb, xb, *_) in fm["per_class"]["TC_瞬时churn"]["coords"]}
    # T2/T3 对齐入场对照（hold>1）
    aligned_keys = set()
    for cls in ("T2_入场滞后", "T3_出场滞后"):
        for (lvl, d, eb, xb, *_) in fm["per_class"][cls]["coords"]:
            aligned_keys.add((lvl, d, eb, xb))

    tc_margins = []
    tc_no_stop = 0
    aligned_margins = []
    all_long_margins = []  # 所有有否定线的多腿（含盈利），看 gate 普适影响
    n_stop_finite = 0
    for (k, eb, xb, ep, xp, u, is_short, pnl), rsn, stop in zip(trades, reasons, stops):
        d = "short" if is_short else "long"
        m = margin_of(ep, stop, is_short)
        if np.isfinite(stop):
            n_stop_finite += 1
            if not is_short:
                all_long_margins.append(m)
        key = (int(k), d, int(eb), int(xb))
        if key in tc_keys:
            if m is None:
                tc_no_stop += 1
            else:
                tc_margins.append(m)
        elif key in aligned_keys:
            if m is not None:
                aligned_margins.append(m)

    print(f"\n===== [{sym}] TC 开仓安全垫 vs T2/T3 对照 =====")
    print(f"  有否定线的腿: {n_stop_finite}/{len(trades)} ({100*n_stop_finite/max(1,len(trades)):.1f}%)")
    if tc_margins:
        tm = np.array(tc_margins)
        print(f"  TC margin (n={len(tm)}, 无stop={tc_no_stop}): "
              f"median={np.median(tm)*1e4:.2f}bps p75={np.percentile(tm,75)*1e4:.2f}bps "
              f"max={tm.max()*1e4:.2f}bps | <10bps占{100*np.mean(tm<10e-4):.1f}%")
    if aligned_margins:
        am = np.array(aligned_margins)
        print(f"  T2/T3 margin (n={len(am)}): "
              f"median={np.median(am)*1e4:.2f}bps p75={np.percentile(am,75)*1e4:.2f}bps "
              f"max={am.max()*1e4:.2f}bps | <10bps占{100*np.mean(am<10e-4):.1f}%")
    if all_long_margins:
        alm = np.array(all_long_margins)
        for thr in (1e-4, 5e-4, 10e-4, 20e-4):
            print(f"  [若 gate=margin<{thr*1e4:.0f}bps 拒开] 影响多腿: {100*np.mean(alm<thr):.2f}% "
                  f"({int(np.sum(alm<thr))}/{len(alm)})")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        probe(sym, fname)
