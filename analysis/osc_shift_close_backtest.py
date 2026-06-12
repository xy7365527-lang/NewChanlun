"""osc 中枢上移出口十标的回测：V2oa25_ht（基线）vs V2oa25_ht_sc。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者任务：osc 闭腿加结构性出口，消灭僵尸腿）
═══════════════════════════════════════════════════════════════════════
在册 osc 闭腿只有：ZD 触线（正常兑现）与 master 强平（僵尸腿最终归宿）。
僵尸腿诊断（在册）：type3 confirmed 在单边趋势中回抽不发生 ⇒ 出口2
（type3 强闭）不触发；尾部 10% 亏损腿 ≈ 全 master 强平，占总亏 62-82%。

出口1（本任务新增）：中枢向上移动——新中枢形成且新中枢 ZD > 锚中枢 ZG。
原文依据：49课"中枢向上移动时就应该满仓"⇒ 旧中枢的 osc 空腿立即回补。
出口2（已存在）：type3 确认强闭（锚中枢死亡回补）——出口1补的正是它
在僵尸场景下失效的情况。

实现（rust/src/trading）：osc_shift_close 开关（变体 V2oa25_ht_sc）；
LegAnchor::Center 加 zg 快照（CenterBook.last 被新中枢覆盖后旧 ZG 不可
回溯）；闭腿优先级 中枢死亡 > ZD 触线 > 中枢上移 > 三卖不回补持有；
n_osc_shift_close 可观测（reason 编码 center_shift_close）。
方向对称声明：osc 腿构造性 short-only（开腿 c≥ZG ∧ sub_sell），下降趋势
osc 多腿类型层不存在，对称分支无承载对象。

预注册判据：
1. BTC/GC/ES 的僵尸腿被中枢移动出口消灭 → osc 亏损减少；
2. OKLO/BRN 的正常腿不受影响（中枢不移动 = 出口不触发）；
3. 十标的不需要白名单——同一逻辑在所有 regime 自适应。

osc 腿盈亏从 diag 腿 trace 直接分解（leg_kind == "osc"），与 o41/co 任务
同一口径。

输出：analysis/data_cache/osc_shift_{SYM}.json（每标的独立文件，规避
双实例竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN PYTHONPATH=src .venv/bin/python \
      analysis/osc_shift_close_backtest.py
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
VARIANTS = ["V2oa25_ht", "V2oa25_ht_sc"]


def osc_leg_stats(diag: list) -> dict:
    """diag 腿 trace → osc 域腿盈亏分解（o41/co 任务同一口径）。

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
            "n_osc_shift_close": c.get("n_osc_shift_close"),
            "n_osc_dead_holds": c.get("n_osc_dead_holds"),
        },
        "osc_legs": osc_leg_stats(res["diag"]),
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"osc_shift_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — V2oa25_ht vs V2oa25_ht_sc（floor={FLOOR}）"
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
              f" 上移闭={co['n_osc_shift_close']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
