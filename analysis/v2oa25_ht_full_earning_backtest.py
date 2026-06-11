"""V2oa25_ht 全标的回测 + 降成本/挣股数两阶段守恒律实证统计。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：全标的 V2oa25_ht + earning 状态检查）
═══════════════════════════════════════════════════════════════════════
1. 十标的（OKLO/BRN/CL/BTC/PINS + ES/GC/ZN/6E/DX）V2oa25_ht vs V2oa25
   基线 vs BH 完整对照。已有 5 标的在册（master_exit_mode + pins JSON），
   metrics 必须与在册逐键一致（磁带指纹同口径守卫）。
2. ledger.rs 两阶段守恒律的实证面（diag=True 逐腿 trace）：
   - cost_basis_at_exit ≤ 0 的笔数（相变到达率）
   - 挣股数阶段持续 bar 数（相变腿 close bar → exit bar）
   - 仓位变化（total_shares_exit / 初始股数 − 1）
   - n_earning_reached / n_t5_shareconserving_after_earning 计数器
   - 挣股数腿（was_earning=True，金额守恒）数量与 shares_delta 合计

做空侧说明（代码层结论，不依赖回测）：master 无做空入场（master.rs
零 short 命中）；DiffSide::Short 是"先卖后买"短差腿方向（持多仓内的
高抛低吸），非净空头。"做空挣负股数"概念在当前账本中不存在——
EarningShares 的金额守恒只定义在 Short 循环上（open_sub 在 earning
阶段显式拒绝 Long 腿，AmountConserving×Long 不可表示，构造保证）。

输出：analysis/data_cache/v2oa25_ht_earning_{SYM}.json（每标的独立文件，
规避双实例竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=ES,GC PYTHONPATH=src .venv/bin/python \
      analysis/v2oa25_ht_full_earning_backtest.py
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG, extended_metrics  # noqa: E402
from organic_fugue_rust_backtest import _trades_from_rust  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
INITIAL_CAPITAL = 100_000.0
FLOOR = LADDER_SEG
SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
VARIANTS = ["V2oa25", "V2oa25_ht"]


def earning_stats(diag: list, n_bars: int) -> dict:
    """逐 trade diag → 两阶段守恒律实证面。

    diag 行 = (header10, diffs)；header = (entry_bar, entry_price, exit_bar,
    exit_price, reason, ladder, pnl, cb_exit, shares_exit, reached_earning)；
    diff = ((key, leg, sell_bar, sell_price, buy_bar, buy_price),
            (shares, diff, profit, was_earning, shares_delta, cb0, cb1))。
    trace 行按 close 顺序追加 ⇒ 首个 cb1 ≤ 0 的行即相变腿。
    """
    n_trades = len(diag)
    earning_trades = []
    n_cb_le0 = 0
    n_earning_legs_total = 0
    earning_shares_delta_total = 0.0
    for header, diffs in diag:
        (e_bar, e_price, x_bar, _xp, reason, _lad, pnl,
         cb_exit, shares_exit, reached) = header
        if cb_exit <= 0.0:
            n_cb_le0 += 1
        e_legs = [d for d in diffs if d[1][3]]  # was_earning=True（金额守恒腿）
        n_earning_legs_total += len(e_legs)
        earning_shares_delta_total += sum(d[1][4] for d in e_legs)
        if not reached:
            continue
        # 相变腿：首个 cb_after ≤ 0（Short 腿 close bar = buy_bar）
        trans_bar = None
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1) in diffs:
            if cb1 <= 0.0 and cb0 > 0.0:
                trans_bar = bb
                break
        init_shares = INITIAL_CAPITAL / e_price
        earning_trades.append({
            "entry_bar": e_bar, "exit_bar": x_bar, "exit_reason": reason,
            "pnl_pct": pnl,
            "transition_bar": trans_bar,
            "earning_duration_bars": (x_bar - trans_bar
                                      if trans_bar is not None else None),
            "hold_bars": x_bar - e_bar,
            "init_shares": round(init_shares, 4),
            "shares_exit": round(shares_exit, 4),
            "shares_gain_pct": round((shares_exit / init_shares - 1) * 100, 2),
            "n_earning_legs": len(e_legs),
        })
    return {
        "n_trades": n_trades,
        "n_cost_basis_le0_at_exit": n_cb_le0,
        "n_reached_earning": len(earning_trades),
        "n_earning_legs_total": n_earning_legs_total,
        "earning_shares_delta_total": round(earning_shares_delta_total, 4),
        "earning_trades": earning_trades,
        # 降成本侧分布：cost_basis_at_exit / entry_price（残余成本比）
        "cb_ratio_quantiles": _cb_ratio_quantiles(diag),
    }


def _cb_ratio_quantiles(diag: list) -> dict:
    ratios = sorted(h[7] / h[1] for h, _ in diag if h[1] > 0)
    if not ratios:
        return {}
    q = lambda p: round(ratios[min(len(ratios) - 1,
                                   int(p * (len(ratios) - 1)))], 4)
    return {"min": round(ratios[0], 4), "p25": q(0.25), "p50": q(0.5),
            "p75": q(0.75), "max": round(ratios[-1], 4)}


def run_cell(rtape, variant: str, years, bh: float, n_bars: int) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    raw = res["trades"]
    hold_bars = sum(t[2] - t[0] for t in raw)
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "exposure": round(hold_bars / n_bars, 4),
        "n_exit_trend_holds": c.get("n_exit_trend_holds"),
        "counters_earning": {
            "n_earning_reached": c.get("n_earning_reached"),
            "n_t5_shareconserving_after_earning":
                c.get("n_t5_shareconserving_after_earning"),
            "n_open_rejects_zero": c.get("n_open_rejects_zero"),
        },
        "earning": earning_stats(res["diag"], n_bars),
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"v2oa25_ht_earning_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — V2oa25_ht + earning 统计（floor={FLOOR}）"
          f"\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    print(f"  信号层 {sig_s:.1f}s  fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
           "tape_fp": fp, "floor": FLOOR, "cells": {}}
    base_tc = None
    for variant in VARIANTS:
        pack = run_cell(rtape, variant, years, bh, n)
        if variant == "V2oa25":
            base_tc = pack["metrics"]["total_compound"]
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"] - base_tc, 1)
            if base_tc is not None and variant != "V2oa25" else None)
        out["cells"][variant] = pack
        m = pack["metrics"]
        e = pack["earning"]
        print(f"  [{variant:10s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" earning相变={e['n_reached_earning']}/{e['n_trades']}"
              f" t5={pack['counters_earning']['n_t5_shareconserving_after_earning']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
