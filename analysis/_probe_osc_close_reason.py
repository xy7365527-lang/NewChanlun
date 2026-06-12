"""osc 闭腿原因逐腿探针（纯诊断，不改引擎）。

结构性定理（step_osc 代码直读，L0）：
- osc 开腿条件 c >= zg（卖在 ZG 上方），ZD 闭腿条件 c <= zd，zg > zd 恒成立
  ⇒ ZD 触线腿恒盈利 ⇒ 全部亏损腿 ⊂ 强制回补腿（中枢死亡 / master 强平）。

逐腿提取 diag trace：(sell_bar, sell_price, buy_bar, buy_price, profit)，
分类：loss（必为强闭）/ win；win 内细分 zd 兑现 vs 强闭低位（聚合 counter 对照）。
master 强平腿 = buy_bar ∈ trades 出场 bar 集。
量化：持仓时长分布、不利偏移 (buy/sell−1)、亏损集中度。
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "BTC,OKLO,BRN").split(",")]


def pct(xs, q):
    if not xs:
        return None
    s = sorted(xs)
    i = min(len(s) - 1, int(q * len(s)))
    return s[i]


def leg_rows(diag):
    """diff 行 → osc 腿列表 [(sell_bar, sell_p, buy_bar, buy_p, profit)]。"""
    out = []
    for _header, diffs in diag:
        for (key, leg_kind, sb, sp, bb, bp), (_sh, _df, profit, *_r) in diffs:
            if leg_kind != "osc":
                continue
            out.append((sb, sp, bb, bp, profit))
    return out


def summarize(legs, exit_bars):
    win = [l for l in legs if l[4] > 0]
    loss = [l for l in legs if l[4] <= 0]

    def stats(group):
        holds = [l[2] - l[0] for l in group]
        adverse = [l[3] / l[1] - 1.0 for l in group]  # Short：买回价/卖价−1
        pnl = [l[4] for l in group]
        n_master = sum(1 for l in group if l[2] in exit_bars)
        return {
            "n": len(group),
            "n_master_exit": n_master,
            "pnl_sum": round(sum(pnl), 1),
            "pnl_avg": round(sum(pnl) / len(group), 1) if group else None,
            "hold_p50": pct(holds, 0.5),
            "hold_p90": pct(holds, 0.9),
            "adverse_p50_pct": round(pct(adverse, 0.5) * 100, 3) if group else None,
            "adverse_p90_pct": round(pct(adverse, 0.9) * 100, 3) if group else None,
        }

    # 亏损集中度：最大 10% 亏损腿占总亏损比例
    loss_sorted = sorted((l[4] for l in loss))
    k = max(1, len(loss_sorted) // 10) if loss_sorted else 0
    top_decile = sum(loss_sorted[:k]) if k else 0.0
    total_loss = sum(loss_sorted) if loss_sorted else 0.0
    return {
        "win": stats(win),
        "loss": stats(loss),
        "loss_top10pct_share": round(top_decile / total_loss, 3) if total_loss else None,
        # 亏损腿原始行（尾部相关性分析）：(hold_bars, adverse_pct, profit, master_exit)
        "loss_rows": [
            (l[2] - l[0], round((l[3] / l[1] - 1.0) * 100, 3),
             round(l[4], 1), l[2] in exit_bars)
            for l in loss
        ],
    }


def main():
    results = {}
    for symbol in SYMBOLS:
        print(f"=== {symbol} ===", flush=True)
        opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
        t0 = time.time()
        dir_flips: list = []
        trend_flips: list = []
        sig = compute_organic_signals(opens, highs, lows, closes,
                                      dir_flips=dir_flips,
                                      trend_flips=trend_flips)
        rtape = pack_tape(sig, dir_flips=dir_flips, trend_flips=trend_flips)
        print(f"  signals {time.time()-t0:.1f}s", flush=True)
        res = nr.run_organic_rust(rtape, "V2oa25_ht", floor_ladder=LADDER_SEG,
                                  stop_mode="none", diag=True)
        legs = leg_rows(res["diag"])
        exit_bars = {t[2] for t in res["trades"]}
        c = res["counters"]
        summ = summarize(legs, exit_bars)
        summ["counters"] = {k: c.get(k) for k in
                            ("n_osc_open", "n_osc_zd_close", "n_osc_dead_holds")}
        results[symbol] = summ
        print(json.dumps(summ, indent=1, ensure_ascii=False), flush=True)

    out = DATA_DIR / "_probe_osc_close_reason.json"
    out.write_text(json.dumps(results, indent=1, ensure_ascii=False))
    print(f"saved → {out}")


if __name__ == "__main__":
    main()
