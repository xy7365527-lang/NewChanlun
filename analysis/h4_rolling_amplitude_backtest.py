"""H4 滚动振幅准入五标的回测：V2oa25_ht / _h4 同磁带双变体。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者任务：H4 滚动振幅准入——osc 白名单消除 fallback）
═══════════════════════════════════════════════════════════════════════
h4 = osc_amp_gate（38课行32 + 35课行30）：osc 开腿前提 = 锚层典型中枢
     相对振幅 θ_q（DepthRef 因果滚动 P50，50中枢窗/min_obs=10，零前瞻）
     ≥ theta_cost_k × friction_rt（2 × 10bps = 0.2%）。
     判据从回测盈亏符号（白名单——不可在线、不可证伪）换成
     结构量 × 物理摩擦（事前可读，38课行32"选择一组历史上某级别平均
     震荡幅度最大的股票"的运行时形式）。

预注册判据（config.rs V2oa25_ht_h4 注释在册；任一不满足按对应轴否证）：
1. 负域 BTC/CL/ES osc 亏损缩减 ≥50%；
2. 正域 OKLO 不恶化（OKLO 振幅 P50≈0.93% ≫ 0.2%，门应近零拦截）；
3. BRN（振幅 P50≈0.10% < 门槛）零摩擦 Δosc 为负是判据的诚实后果而非
   否证——裁决口径 = 摩擦调整面：基线 osc 腿净盈亏 − friction_rt ×
   逐腿名义额（10bps 往返），若基线 BRN osc 摩擦调整后净负，则 h4 的
   拦截是保护而非伤害（35课：振幅不足的级别长期操作没有意义）；
4. 双拒因（noref/thin）逐事件可分离（G3）。

口径：双变体同磁带（同一 compute_organic_signals pass），osc 腿盈亏从
diag 腿 trace 分解（leg_kind == "osc"，h1/h2/scco 同一口径）。
tape_fp 守卫：与在册 osc_scco_{SYM}.json 指纹对账，不一致即 fail-fast。

输出：analysis/data_cache/h4_amp_{SYM}.json（每标的独立文件，规避双实例
竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN,BTC,CL,ES PYTHONPATH=src .venv/bin/python \
      analysis/h4_rolling_amplitude_backtest.py
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
VARIANTS = [BASELINE, "V2oa25_ht_h4"]
# 摩擦调整口径常数 = 引擎门槛同源（config.rs friction_rt 默认值，10bps 往返）
FRICTION_RT = 0.001


def osc_legs(diag: list) -> list[tuple[int, float, int, float, float, float]]:
    """diag 腿 trace → osc 域腿 (sell_bar, sell_p, buy_bar, buy_p, shares, profit)。"""
    legs = []
    for _header, diffs in diag:
        for (_key, leg_kind, sell_bar, sp, buy_bar, bp), \
                (sh, _df, profit, *_rest) in diffs:
            if leg_kind == "osc":
                legs.append((sell_bar, sp, buy_bar, bp, sh, profit))
    return legs


def leg_stats(legs: list) -> dict:
    """osc 腿盈亏聚合 + 尾部 10% + 摩擦调整面（35课口径——判据3 数据基础）。

    摩擦调整：每腿净盈亏 = profit − FRICTION_RT × sell_p × shares
    （往返 10bps 对开腿名义额——与引擎门槛 theta_cost_k × friction_rt 同源）。
    """
    n = len(legs)
    profits = [leg[5] for leg in legs]
    fric = [leg[5] - FRICTION_RT * leg[1] * abs(leg[4]) for leg in legs]
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
        "profit_friction_adj": round(sum(fric), 2),
        "n_wins_friction_adj": sum(1 for p in fric if p > 0),
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
            "n_osc_amp_rejects": c.get("n_osc_amp_rejects"),
            "n_osc_amp_noref_rejects": c.get("n_osc_amp_noref_rejects"),
        },
        "osc_legs": leg_stats(legs),
        "elapsed_s": round(el, 3),
    }
    raw_logs = {"legs": legs, "amp_log": c.get("osc_amp_reject_log", [])}
    return pack, raw_logs


def timing_probe(base_legs: list, amp_log: list) -> dict:
    """时序靶：基线 osc 尾部腿（最亏 10%）开腿 bar ∈ H4 拒绝集（H1/H2 在册
    探针方法）。拒绝率为下界（变体间持仓分歧致无开腿尝试的 bar 无日志）。
    """
    n = len(base_legs)
    tail_n = math.ceil(n * 0.1) if n else 0
    tail = sorted(base_legs, key=lambda t: t[5])[:tail_n]
    amp_bars = {bar: reason for _lad, bar, reason in amp_log}
    rows = []
    n_rej = 0
    for leg in tail:
        sell_bar, profit = leg[0], leg[5]
        reason = amp_bars.get(sell_bar)
        rejected = reason is not None
        n_rej += rejected
        rows.append({"sell_bar": sell_bar, "profit": round(profit, 2),
                     "rejected": rejected,
                     "reason": {0: "noref", 1: "thin", None: None}[reason]})
    return {
        "tail_n": tail_n,
        "n_rejected": n_rej,
        "reject_rate": round(n_rej / tail_n, 4) if tail_n else None,
        "tail_loss": round(sum(t[5] for t in tail), 2),
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
            "双变体矩阵不可与在册 h1/h2/scco 判决同表比较，停。")
    print("  [fp守卫] 与在册 scco 缓存指纹一致", flush=True)


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"h4_amp_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — H4 滚动振幅准入双变体矩阵（floor={FLOOR}）"
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
           "tape_fp": fp, "floor": FLOOR, "friction_rt": FRICTION_RT,
           "cells": {}}
    base_tc = None
    base_osc = None
    base_legs = None
    logs_by_variant: dict = {}
    for variant in VARIANTS:
        pack, raw_logs = run_cell(rtape, variant, years, bh, n)
        logs_by_variant[variant] = raw_logs
        if variant == BASELINE:
            base_tc = pack["metrics"]["total_compound"]
            base_osc = pack["osc_legs"]["profit_total"]
            base_legs = raw_logs["legs"]
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
        print(f"  [{variant:14s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" osc腿={o['n_legs']:5d} osc盈亏={o['profit_total']:+12.2f}"
              f" 摩擦调整={o['profit_friction_adj']:+12.2f}"
              f" 尾10%亏={o['tail10_loss']:+12.2f}"
              f" amp拒(noref/thin)={co['n_osc_amp_noref_rejects']}"
              f"/{co['n_osc_amp_rejects']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)

    out["timing_probe_h4"] = timing_probe(
        base_legs, logs_by_variant["V2oa25_ht_h4"]["amp_log"])
    tp = out["timing_probe_h4"]
    print(f"  [时序靶] 尾部{tp['tail_n']}腿 被拒={tp['n_rejected']}"
          f" 拒绝率={tp['reject_rate']}", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
