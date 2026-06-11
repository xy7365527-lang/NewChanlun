"""osc 操作域严格化十标的回测：V2oa25_ht（基线）vs V2oa25_ht_co。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：osc 操作域严格化）
═══════════════════════════════════════════════════════════════════════
这不是外加门——是 osc 操作对象定义的严格化：只在盘整走势的中枢里做
震荡短差，不在趋势走势的中枢里做。

前序否证（osc_l41_gate，de02252）：点态门 = 腿延迟非腿消灭——BTC 拒 78
净减 8，BTC/GC osc 亏损反恶化，"未衰竭⇒不回ZD"传导链断裂。本轴换范畴：
对象域定义（锚中枢所在走势 kind，状态范畴）替代点态条件（父级别衰竭
bar 级读数，事件范畴）。

原文依据：
- 38课：中枢震荡操作的隐含前提是确立的中枢（价格在中枢内反复震荡
  = 盘整走势）；
- 49课"中枢向上移动时就应该满仓"——趋势走势不做逆向短差；
- 26课"单边上扬走势，短线最好别做"。

实现（rust/src/trading）：osc_domain = Any（在册）| ConsolidationOnly（新）。
ConsolidationOnly：osc 开腿前读 trend_row[k]（锚中枢所在层尾 move kind，
trend_flips 磁带行直接消费）——kind==Trend ⇒ 不开（n_osc_domain_rejects
可观测）；kind==Consolidation ⇒ 38课域内正常开。闭腿路径零接触（域只
定义操作对象，不约束兑现）。

预注册判据：
1. BTC/GC/ES 的 osc 亏损消除或大幅减少（趋势中 osc 从定义上不开）；
2. OKLO/BRN 的 osc 正贡献保持（盘整中正常开）；
3. 十标的全部 ≥ 基线。

osc 腿盈亏从 diag 腿 trace 直接分解（leg_kind == "osc"），与 o41 任务
同一口径（counters 的 rev_osc_* 是 REV 震荡型腿，范畴不同）。

输出：analysis/data_cache/osc_domain_{SYM}.json（每标的独立文件，规避
双实例竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN PYTHONPATH=src .venv/bin/python \
      analysis/osc_domain_backtest.py
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
VARIANTS = ["V2oa25_ht", "V2oa25_ht_co"]


def osc_leg_stats(diag: list) -> dict:
    """diag 腿 trace → osc 域腿盈亏分解（o41 任务同一口径）。

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
            "n_osc_domain_rejects": c.get("n_osc_domain_rejects"),
        },
        "osc_legs": osc_leg_stats(res["diag"]),
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"osc_domain_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — V2oa25_ht vs V2oa25_ht_co（floor={FLOOR}）"
          f"\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    sig_s = time.time() - t0
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"  信号层 {sig_s:.1f}s  fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

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
              f" 域拒={co['n_osc_domain_rejects']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
