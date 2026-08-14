# FROZEN(#951)：绑 recursive_t.RecTStream.finish_full 九字段；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""TC 因果 gate 穷尽检验（#170/R4 escalate 前最后穷尽）。

已否证：margin<0（开仓 close 破 ZD）含等量盈利腿 ⇒ 无判别力。
本探针穷尽剩余因果（开仓时可知，无 look-ahead）候选：

  G1. 开仓 bar 盘中 low < ZD（开仓那根 bar 已盘中破否定线）—— 比 close margin 更紧
  G2. r≤0 的 TC 笔（确切止损 collapse）在 (margin<0 & hold=1) 全集中的精确隔离：
      同一开仓状态(margin<0)下，r≤0 笔 vs r>0 笔，开仓时有无任何可区分特征？
  G3. 终极拷问：固定「margin<0 & hold=1」全集（开仓状态同质），其中 r≤0 与 r>0 的
      唯一区别是下一 bar 价格方向（look-ahead）⇒ 若无开仓时特征区分 ⇒ 频率不可约。

认识论：L3。否定性结果（无 gate 可分离）= escalate 依据（#164「频率可约」被否证）。
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
    L = np.array(l)
    H = np.array(h)
    res = run_face_a(o, h, l, c, mode)
    trades = res["leg_trades"]
    stops = res["leg_entry_stops"]
    n = len(c)

    # 固定全集 = margin<0 & hold=1 多腿（开仓状态同质）。在其中按 pnl 符号分，
    # 检验开仓时可知特征（low<ZD / entry距ZD深度）能否区分 win vs loss。
    g_low_below_zd = {"win": 0, "loss": 0}    # 开仓bar low<ZD
    g_low_above_zd = {"win": 0, "loss": 0}
    depth_win = []   # (ZD-entry)/entry 当 margin<0（开仓破ZD多深）
    depth_loss = []
    for (k, eb, xb, ep, xp, u, is_short, pnl), stop in zip(trades, stops):
        if is_short or not np.isfinite(stop):
            continue
        margin = (ep - stop) / ep
        hold = xb - eb
        if not (margin < 0 and hold <= 1):
            continue
        ebi = int(eb)
        psign = "win" if pnl > 0 else "loss"
        low_below = (0 <= ebi < n) and (L[ebi] < stop)
        if low_below:
            g_low_below_zd[psign] += 1
        else:
            g_low_above_zd[psign] += 1
        depth = (stop - ep) / ep  # >0（破ZD深度）
        (depth_win if pnl > 0 else depth_loss).append(depth)

    print(f"\n===== [{sym}] 固定全集 margin<0 & hold=1：win vs loss 开仓时特征 =====")
    nw = g_low_below_zd["win"] + g_low_above_zd["win"]
    nl = g_low_below_zd["loss"] + g_low_above_zd["loss"]
    print(f"  全集: win={nw} loss={nl}")
    print(f"  开仓bar low<ZD : win={g_low_below_zd['win']} loss={g_low_below_zd['loss']}")
    print(f"  开仓bar low>=ZD: win={g_low_above_zd['win']} loss={g_low_above_zd['loss']}")
    if depth_win and depth_loss:
        dw = np.array(depth_win); dl = np.array(depth_loss)
        print(f"  破ZD深度(bps): win median={np.median(dw)*1e4:.2f} | loss median={np.median(dl)*1e4:.2f}")
        print(f"  ⇒ 若深度可分: win/loss median 差 = {abs(np.median(dw)-np.median(dl))*1e4:.2f}bps")


if __name__ == "__main__":
    syms = SYMBOLS if len(sys.argv) < 2 else [(s, dict(SYMBOLS).get(s, "")) for s in sys.argv[1:]]
    for sym, fname in syms:
        probe(sym, fname)
