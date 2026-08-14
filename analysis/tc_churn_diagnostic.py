# FROZEN(#951)：绑 recursive_t.RecTStream.finish_full 九字段；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""TC 瞬时 churn 诊断（#170/R4 修复前 L3 实证）。

目的：在动手改引擎前，先穷尽理解 TC 笔（hold≤CHURN_HOLD_BARS=1）的结构性质，
确认「1-bar collapse 信号携零走势」（covered_Δ≈floor，未骑任何走势段），
并拆解 TC 笔的方向/级别分布，定位过滤点。

认识论：L3（真实数据每笔，可证伪）。复用 capture_ratio_matrix 同源引擎 finish_full。

coords schema (per_class[cls].coords): (level, dir, entry_bar, exit_bar, hold, r, covered_dz, pnl, xlc)
"""
from __future__ import annotations

import sys
from pathlib import Path

import newchan_rust as nr  # noqa: F401

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "analysis"))
from capture_ratio_matrix import (  # noqa: E402
    load_clean_ohlc, run_face_a, classify_failure_modes, per_trade_capture,
    CHURN_HOLD_BARS,
)

MAIN_DATA = Path("/Users/silencehan/Projects/NewChanlun/analysis/data_cache")
SYMBOLS = [
    ("OKLO", "oklo_1m_databento.json"),
    ("BTC", "btc_1m_full.json"),
    ("CL", "cl_1m_databento_10y.json"),
]


def diagnose(sym: str, fname: str, mode: str = "structural") -> None:
    path = MAIN_DATA / fname
    if not path.exists():
        print(f"[{sym}] data missing: {path}")
        return
    o, h, l, c = load_clean_ohlc(path)
    res = run_face_a(o, h, l, c, mode)
    rows = per_trade_capture(res, h, l, c)
    r_le0 = [x for x in rows if x["r"] <= 0]
    fm = classify_failure_modes(r_le0, res, h, l, c)

    n_rle0 = fm["n_r_le0"]
    tc = fm["per_class"]["TC_瞬时churn"]
    coords = tc["coords"]  # (level, dir, eb, xb, hold, r, covered, pnl, xlc)
    print(f"\n========== [{sym}] mode={mode} n_bars={len(c)} n_trades={len(rows)} ==========")
    print(f"总 r<=0 笔: {n_rle0} | 各类: {fm['counts']}")
    print(f"TC 笔: {tc['count']} ({100*tc['pct_of_r_le0']:.1f}% of r<=0) | xlc(跨级冲突): {tc['n_cross_level_conflict']}")
    if not coords:
        return
    by_dir = {"long": 0, "short": 0}
    by_level: dict[int, int] = {}
    holds = []
    floor_hit = 0
    pnl_sum = 0.0
    for (lvl, d, eb, xb, hold, r, cov, pnl, xlc) in coords:
        by_dir[d] += 1
        by_level[lvl] = by_level.get(lvl, 0) + 1
        holds.append(hold)
        pnl_sum += pnl
        # covered 极小（≈floor，从未有利）：covered <= 1e-6 量级 或 r 接近 -1（净位移全负）
    print(f"TC 方向: long={by_dir['long']} short={by_dir['short']}")
    print(f"TC 级别: {dict(sorted(by_level.items()))}")
    print(f"TC hold 分布: min={min(holds)} max={max(holds)} (CHURN_HOLD_BARS={CHURN_HOLD_BARS})")
    print(f"TC pnl 合计: {pnl_sum:+.2f}")
    print(f"TC 示例(最亏在前) [level,dir,eb,xb,hold,r,covered,pnl,xlc]:")
    for row in coords[:6]:
        print(f"  {row}")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        diagnose(sym, fname)
