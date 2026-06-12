"""H2 力度收敛门五标的回测：V2oa25_ht / _h1 / _h2 / _h1h2 同磁带四变体。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者任务：H2 力度收敛门——补位 H1 无效的负域标的）
═══════════════════════════════════════════════════════════════════════
h2 = osc_strength_gate（49课行38/52）：开腿前比较锚中枢最近两次向上
     离开段力度（excursion = H1 窗口内 max(c) − 当时 ZG，零 surfacing）。
     历史 <2 条 ⇒ 新生保守拒开；最近 > 前次 ⇒ 扩张拒开（三类点预警）；
     收敛 ⇒ 放行。判据时点 = 开腿时刻（只读历史）——覆盖 H1 candidate
     窗口的"窗口前"盲区（533号 BRN 裁决：负域僵尸主体开在窗口前）。

预注册判据（h2_strength_convergence_design.md §5；任一不满足按对应轴否证）：
1. 时序靶：基线僵尸尾部腿（最亏 10%）开腿 bar 被 H2 拒绝率 ≥50%；
2. 负域量级：BTC/CL/ES Δosc 缩减 ≥50%；
3. OKLO 保护（h1h2）：Δosc(h1h2 vs h1) ≥ −10K；
4. 腿消灭 vs 延迟：腿数降 ∧ 尾部亏损等量缩减（双指标）。

口径：四变体同磁带（同一 compute_organic_signals pass），osc 腿盈亏从
diag 腿 trace 分解（leg_kind == "osc"，h1/sc/co/scco 同一口径）。
tape_fp 守卫：与在册 osc_scco_{SYM}.json 指纹对账，不一致即 fail-fast。

时序靶探针（H1 §4 在册方法）：基线 osc 尾部腿 sell_bar ∩ h2 变体
osc_sg_reject_log 的 bar 集。caveat（诚实声明）：变体间持仓状态分歧 ⇒
h2 运行在该 bar 可能因槽位被占而无开腿尝试（无日志条目）——此时该腿
在 h2 中同样未开出（计入 untracked 而非 intercepted，拒绝率是下界）。

输出：analysis/data_cache/h2_sg_{SYM}.json（每标的独立文件，规避双实例
竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN,BTC,CL,ES PYTHONPATH=src .venv/bin/python \
      analysis/h2_strength_convergence_backtest.py
"""

from __future__ import annotations

import json
import math
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
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN,BTC,CL,ES").split(",")]
BASELINE = "V2oa25_ht"
VARIANTS = [BASELINE, "V2oa25_ht_h1", "V2oa25_ht_h2", "V2oa25_ht_h1h2"]


def osc_legs(diag: list) -> list[tuple[int, int, float]]:
    """diag 腿 trace → osc 域腿 (sell_bar, buy_bar, profit) 列表。"""
    legs = []
    for _header, diffs in diag:
        for (_key, leg_kind, sell_bar, _sp, buy_bar, _bp), \
                (_sh, _df, profit, *_rest) in diffs:
            if leg_kind == "osc":
                legs.append((sell_bar, buy_bar, profit))
    return legs


def leg_stats(legs: list[tuple[int, int, float]]) -> dict:
    """osc 腿盈亏聚合 + 尾部 10%（判据4 双指标：腿数 + 尾部亏损）。"""
    n = len(legs)
    profits = [p for _, _, p in legs]
    tail_n = math.ceil(n * 0.1) if n else 0
    tail = sorted(profits)[:tail_n]
    return {
        "n_legs": n,
        "n_wins": sum(1 for p in profits if p > 0),
        "win_rate": round(sum(1 for p in profits if p > 0) / n, 4) if n else None,
        "profit_total": round(sum(profits), 2),
        "loss_total": round(sum(p for p in profits if p <= 0), 2),
        "tail10_n": tail_n,
        "tail10_loss": round(sum(tail), 2),
    }


def run_cell(rtape, variant: str, years, bh: float, n_bars: int) -> tuple[dict, dict]:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    raw = res["trades"]
    hold_bars = sum(t[2] - t[0] for t in raw)
    legs = osc_legs(res["diag"])
    pack = {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "exposure": round(hold_bars / n_bars, 4),
        "counters_osc": {
            "n_osc_open": c.get("n_osc_open"),
            "n_osc_zd_close": c.get("n_osc_zd_close"),
            "n_osc_cf_rejects": c.get("n_osc_cf_rejects"),
            "n_osc_sg_newborn_rejects": c.get("n_osc_sg_newborn_rejects"),
            "n_osc_sg_expand_rejects": c.get("n_osc_sg_expand_rejects"),
            "n_cf_windows": c.get("n_cf_windows"),
            "n_cf_negations": c.get("n_cf_negations"),
        },
        "osc_legs": leg_stats(legs),
        "elapsed_s": round(el, 3),
    }
    raw_logs = {
        "legs": legs,
        "sg_log": c.get("osc_sg_reject_log", []),
        "cf_log": c.get("osc_cf_reject_log", []),
    }
    return pack, raw_logs


def timing_probe(base_legs: list[tuple[int, int, float]],
                 sg_log: list[tuple[int, int, int]]) -> dict:
    """预注册判据1 时序靶：基线 osc 尾部腿（最亏 10%）开腿 bar ∈ H2 拒绝集。

    H1 §4 在册探针方法（reject log ∩ 基线僵尸 sell_bar）。拒绝率为下界
    （持仓分歧致无开腿尝试的 bar 无日志——见模块 docstring caveat）。
    """
    n = len(base_legs)
    tail_n = math.ceil(n * 0.1) if n else 0
    tail = sorted(base_legs, key=lambda t: t[2])[:tail_n]
    sg_bars = {bar: reason for _lad, bar, reason in sg_log}
    rows = []
    n_rej = 0
    for sell_bar, _buy_bar, profit in tail:
        reason = sg_bars.get(sell_bar)
        rejected = reason is not None
        n_rej += rejected
        rows.append({"sell_bar": sell_bar, "profit": round(profit, 2),
                     "rejected": rejected,
                     "reason": {0: "newborn", 1: "expand", None: None}[reason]})
    return {
        "tail_n": tail_n,
        "n_rejected": n_rej,
        "reject_rate": round(n_rej / tail_n, 4) if tail_n else None,
        "tail_loss": round(sum(t[2] for t in tail), 2),
        "legs": rows,
    }


def guard_tape_fp(symbol: str, fp: dict) -> None:
    """磁带指纹守卫：与 scco 任务在册缓存对账，不一致即 fail-fast。"""
    ref = DATA_DIR / f"osc_scco_{symbol}.json"
    if not ref.exists():
        print(f"  [fp守卫] {ref.name} 不存在——跳过对账", flush=True)
        return
    ref_fp = json.loads(ref.read_text())["tape_fp"]
    if ref_fp != fp:
        raise RuntimeError(
            f"{symbol} tape_fp 漂移：本次 {fp} ≠ {ref.name} {ref_fp}；"
            "四变体矩阵不可与在册 h1/scco 判决同表比较，停。")
    print("  [fp守卫] 与在册 scco 缓存指纹一致", flush=True)


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"h2_sg_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — H2 力度收敛门四变体矩阵（floor={FLOOR}）"
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
    base_legs = None
    h1_osc = None
    logs_by_variant: dict = {}
    for variant in VARIANTS:
        pack, raw_logs = run_cell(rtape, variant, years, bh, n)
        logs_by_variant[variant] = raw_logs
        if variant == BASELINE:
            base_tc = pack["metrics"]["total_compound"]
            base_osc = pack["osc_legs"]["profit_total"]
            base_legs = raw_logs["legs"]
        if variant == "V2oa25_ht_h1":
            h1_osc = pack["osc_legs"]["profit_total"]
        pack["delta_vs_ht"] = (
            round(pack["metrics"]["total_compound"] - base_tc, 1)
            if variant != BASELINE else None)
        pack["osc_profit_delta"] = (
            round(pack["osc_legs"]["profit_total"] - base_osc, 2)
            if variant != BASELINE else None)
        out["cells"][variant] = pack
        m = pack["metrics"]
        o = pack["osc_legs"]
        co = pack["counters_osc"]
        print(f"  [{variant:16s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" osc腿={o['n_legs']:5d} osc盈亏={o['profit_total']:+12.2f}"
              f" 尾10%亏={o['tail10_loss']:+12.2f}"
              f" sg拒(新生/扩张)={co['n_osc_sg_newborn_rejects']}"
              f"/{co['n_osc_sg_expand_rejects']}"
              f" cf拒={co['n_osc_cf_rejects']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)

    # 预注册判据1 时序靶（h2 单轴日志为准；h1h2 附带）
    out["timing_probe_h2"] = timing_probe(
        base_legs, logs_by_variant["V2oa25_ht_h2"]["sg_log"])
    out["timing_probe_h1h2"] = timing_probe(
        base_legs, logs_by_variant["V2oa25_ht_h1h2"]["sg_log"])
    # 判据3（OKLO 保护）：Δosc(h1h2 vs h1)
    out["osc_h1h2_vs_h1"] = round(
        out["cells"]["V2oa25_ht_h1h2"]["osc_legs"]["profit_total"] - h1_osc, 2)
    tp = out["timing_probe_h2"]
    print(f"  [时序靶] 尾部{tp['tail_n']}腿 被拒={tp['n_rejected']}"
          f" 拒绝率={tp['reject_rate']}"
          f"  Δosc(h1h2 vs h1)={out['osc_h1h2_vs_h1']:+.2f}", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
