"""osc 域腿加 41课门十标的回测：V2oa25_ht（基线）vs V2oa25_ht_o41。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：osc 腿加 41课门实验）
═══════════════════════════════════════════════════════════════════════
根因假设：BTC/GC/ES 跑输 BH 的凶手是 osc 腿（中枢震荡短差）在强趋势中
逆向卖出 → 价格不回 ZD → 中枢死亡 → 高位强制买回。41课门在册只挂 REV
腿（rev_l41_gate），osc 开腿无门。

原文依据：
- 41课"大级别走势没有任何衰竭迹象时参与反向小级别买卖点是刀口舔血"
  ——"一切"包括 osc；
- 49课"中枢向上移动时就应该满仓"——满仓 = 不做逆向短差；
- 26课"除了日线的单边上扬走势，短线必须坚持"——单边时不做短线。

预注册判据：
1. BTC/GC/ES 的 osc 亏损大幅减少（osc 腿盈亏合计向零收敛）；
2. OKLO/BRN 的正 osc 贡献近不变（趋势衰竭时门放行，震荡中正常开腿）。

osc 腿盈亏从 diag 腿 trace 直接分解（leg_kind == "osc" 的 diff 行 profit
合计），非 counters 近似——counters 的 rev_osc_* 是 REV 震荡型腿，与域腿
osc 是不同范畴。

输出：analysis/data_cache/osc_l41_{SYM}.json（每标的独立文件，规避双实例
竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN PYTHONPATH=src .venv/bin/python \
      analysis/osc_l41_gate_backtest.py
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
FLOOR = LADDER_SEG
SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
VARIANTS = ["V2oa25_ht", "V2oa25_ht_o41"]


def osc_leg_stats(diag: list) -> dict:
    """diag 腿 trace → osc 域腿盈亏分解。

    diff 行 = ((key, leg_kind, sell_bar, sell_price, buy_bar, buy_price),
               (shares, diff, profit, was_earning, shares_delta, cb0, cb1))；
    leg_kind == "osc" 即域腿（SlotKey::osc，与 REV 震荡型 kind 范畴不同）。
    """
    n_legs = 0
    n_wins = 0
    profit_total = 0.0
    loss_total = 0.0  # 仅负 profit 合计（亏损侧幅度）
    for _header, diffs in diag:
        for (key, leg_kind, *_), (_sh, _df, profit, *_rest) in diffs:
            if leg_kind != "osc":
                continue
            n_legs += 1
            profit_total += profit
            if profit > 0:
                n_wins += 1
            else:
                loss_total += profit
    return {
        "n_legs": n_legs,
        "n_wins": n_wins,
        "win_rate": round(n_wins / n_legs, 4) if n_legs else None,
        "profit_total": round(profit_total, 2),
        "loss_total": round(loss_total, 2),
    }


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
        "counters_osc": {
            "n_osc_open": c.get("n_osc_open"),
            "n_osc_zd_close": c.get("n_osc_zd_close"),
            "n_osc_dead_holds": c.get("n_osc_dead_holds"),
            "n_osc_l41_rejects": c.get("n_osc_l41_rejects"),
        },
        "osc_legs": osc_leg_stats(res["diag"]),
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"osc_l41_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — V2oa25_ht vs V2oa25_ht_o41（floor={FLOOR}）"
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
    base_osc = None
    for variant in VARIANTS:
        pack = run_cell(rtape, variant, years, bh, n)
        if variant == "V2oa25_ht":
            base_tc = pack["metrics"]["total_compound"]
            base_osc = pack["osc_legs"]["profit_total"]
        pack["delta_vs_ht"] = (
            round(pack["metrics"]["total_compound"] - base_tc, 1)
            if base_tc is not None and variant != "V2oa25_ht" else None)
        pack["osc_profit_delta"] = (
            round(pack["osc_legs"]["profit_total"] - base_osc, 2)
            if base_osc is not None and variant != "V2oa25_ht" else None)
        out["cells"][variant] = pack
        m = pack["metrics"]
        o = pack["osc_legs"]
        co = pack["counters_osc"]
        print(f"  [{variant:14s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" osc腿={o['n_legs']:5d} osc盈亏={o['profit_total']:+12.2f}"
              f" l41拒={co['n_osc_l41_rejects']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
