"""sc×co 双轴组合十标的回测：V2oa25_ht / _sc / _co / _scco 同磁带四变体。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者任务：sc×co 组合验证——两轴都已实装，纯配置组合）
═══════════════════════════════════════════════════════════════════════
sc = osc_shift_close（中枢上移出口，49课"中枢向上移动时就应该满仓"——
     新中枢 ZD > 锚中枢 ZG ⇒ 旧中枢 osc 空腿立即回补，僵尸腿消灭）。
co = osc_domain=ConsolidationOnly（盘整域限制，38课中枢震荡 + 49课/26课
     趋势不做逆向短差——趋势走势的中枢从定义上不开 osc）。

两轴作用面正交：co 限制开腿对象域，sc 收紧闭腿出口。单轴判决在册：
sc 8/10 转正（BTC+412.7pp）；co Δ符号=−sign(基线osc盈亏) 10/10 反相关
（BTC+489.7pp），白名单形态。组合验证 = 正交性假设的 L3 检验。

预注册判据：
1. scco 在 BTC/GC/ES 上 ≥ sc（co 额外消除趋势域开腿）；
2. scco 在 BRN 上 > co（sc 对盘整域内的僵尸残腿也有效）；
3. 理想情况十标的全正——不需要白名单。

口径：四变体同磁带（同一 compute_organic_signals pass），osc 腿盈亏从
diag 腿 trace 分解（leg_kind == "osc"，与 sc/co 单轴任务同一口径）。
tape_fp 守卫：与在册 osc_shift_{SYM}.json / osc_domain_{SYM}.json 的
指纹对账（磁带指纹守卫——REV腿配对任务在册教训），不一致即 fail-fast。

输出：analysis/data_cache/osc_scco_{SYM}.json（每标的独立文件，规避
双实例竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN PYTHONPATH=src .venv/bin/python \
      analysis/osc_scco_backtest.py
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
BASELINE = "V2oa25_ht"
VARIANTS = [BASELINE, "V2oa25_ht_sc", "V2oa25_ht_co", "V2oa25_ht_scco"]


def osc_leg_stats(diag: list) -> dict:
    """diag 腿 trace → osc 域腿盈亏分解（sc/co 单轴任务同一口径）。

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
            "n_osc_domain_rejects": c.get("n_osc_domain_rejects"),
            "n_osc_zd_close": c.get("n_osc_zd_close"),
            "n_osc_shift_close": c.get("n_osc_shift_close"),
            "n_osc_dead_holds": c.get("n_osc_dead_holds"),
        },
        "osc_legs": osc_leg_stats(res["diag"]),
        "elapsed_s": round(el, 3),
    }


def guard_tape_fp(symbol: str, fp: dict) -> None:
    """磁带指纹守卫：与 sc/co 单轴任务的在册缓存对账，不一致即 fail-fast。

    单轴判决建立在各自磁带上；组合判决要与它们同表比较，前提是同一磁带。
    指纹漂移 = 数据重拉或信号层变更 ⇒ 四格矩阵不可与在册单轴数字混表。
    """
    for prefix in ("osc_shift", "osc_domain"):
        ref = DATA_DIR / f"{prefix}_{symbol}.json"
        if not ref.exists():
            print(f"  [fp守卫] {ref.name} 不存在——跳过对账", flush=True)
            continue
        ref_fp = json.loads(ref.read_text())["tape_fp"]
        if ref_fp != fp:
            raise RuntimeError(
                f"{symbol} tape_fp 漂移：本次 {fp} ≠ {ref.name} {ref_fp}；"
                "四格矩阵不可与在册单轴判决同表比较，停。")
    print("  [fp守卫] 与在册 sc/co 缓存指纹一致", flush=True)


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"osc_scco_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — sc×co 四变体矩阵（floor={FLOOR}）"
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
    guard_tape_fp(symbol, fp)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
           "tape_fp": fp, "floor": FLOOR, "cells": {}}
    base_tc = None
    base_osc = None
    for variant in VARIANTS:
        pack = run_cell(rtape, variant, years, bh, n)
        if variant == BASELINE:
            base_tc = pack["metrics"]["total_compound"]
            base_osc = pack["osc_legs"]["profit_total"]
        pack["delta_vs_ht"] = (
            round(pack["metrics"]["total_compound"] - base_tc, 1)
            if base_tc is not None and variant != BASELINE else None)
        pack["osc_profit_delta"] = (
            round(pack["osc_legs"]["profit_total"] - base_osc, 2)
            if base_osc is not None and variant != BASELINE else None)
        out["cells"][variant] = pack
        m = pack["metrics"]
        o = pack["osc_legs"]
        co = pack["counters_osc"]
        print(f"  [{variant:16s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" osc腿={o['n_legs']:5d} osc盈亏={o['profit_total']:+12.2f}"
              f" 域拒={co['n_osc_domain_rejects']}"
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
