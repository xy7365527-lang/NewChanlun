"""H3 级别上移五标的回测：V2oa25_ht / _co / _h1 / _h3 同磁带四变体。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者任务：H3 级别上移——osc 白名单消除的第三方向）
═══════════════════════════════════════════════════════════════════════
h3 = osc_domain=TrendUpshift（26课行183"最好别按1分钟弄，5分钟甚至更长
     都可以"）：趋势态下 osc 重路由到 k+1 层中枢（删除→重路由）——触发
     判据 c≥ZG(k+1)∧sub_sell(k)、锚、ZD 边界、死亡出口全按 k+1 级别运行；
     腿占 k 层槽、用 frac_of(k)（量是物理归属，上移改变操作节奏不改资金
     归属）。趋势态 k 层永不开（重路由非 fallback）；k+1 无中枢/无触发 ⇒
     自然不开。盘整态维持 k 层（ConsolidationOnly 同语义）。

对照：V2oa25_ht（基线）+ V2oa25_ht_co（删腿对照——其 n_osc_domain_rejects
= 趋势态 k 层触发点数 = h3"本来会被删除的腿"参照）+ V2oa25_ht_h1（已实装
的白名单消除候选，533号判决在册）。

预注册判据（osc_whitelist_elimination_research.md §4 H3 + 变体注册注释；
任一不满足按对应轴否证）：
1. 正域 OKLO/BRN 不恶化（h3 ≥ co 的正域表现——盈利腿经 k+1 通道保留）；
2. 负域 BTC/CL/ES osc 亏损大幅缩减（量级对照 co）；
3. 风险轴：k+1 中枢稀疏 ⇒ 腿数大减（n_osc_upshift_open 可观测）——若
   正域收益被频率损失吃掉（OKLO h3 < h1）按增强轴否证；
4. G1 全域判据：五标的全部 ≥ 基线（不需要白名单）。

口径：四变体同磁带（同一 compute_organic_signals pass）；osc 腿盈亏从
diag 腿 trace 分解（leg_kind ∈ {"osc","osc_up"}——上移腿独立投影，
sc/co/scco/h1 任务同一口径的扩展）。tape_fp 守卫：与在册
osc_scco_{SYM}.json 指纹对账，不一致即 fail-fast。

输出：analysis/data_cache/h3_lu_{SYM}.json（每标的独立文件，规避双实例
竞态陷阱——多级别赋格任务在册教训）。
用法：BT_SYMBOLS=OKLO,BRN,BTC,CL,ES PYTHONPATH=src .venv/bin/python \
      analysis/h3_level_upgrade_backtest.py
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
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN,BTC,CL,ES").split(",")]
BASELINE = "V2oa25_ht"
VARIANTS = [BASELINE, "V2oa25_ht_co", "V2oa25_ht_h1", "V2oa25_ht_h3"]


def osc_leg_stats(diag: list) -> dict:
    """diag 腿 trace → osc 域腿盈亏分解（"osc" 本层腿 / "osc_up" H3 上移腿
    分离统计；profit_total 为两者合计——与在册口径（osc 全体）同表可比）。"""
    stats = {kind: {"n_legs": 0, "n_wins": 0, "profit": 0.0, "loss": 0.0}
             for kind in ("osc", "osc_up")}
    for _header, diffs in diag:
        for (key, leg_kind, *_), (_sh, _df, profit, *_rest) in diffs:
            if leg_kind not in stats:
                continue
            s = stats[leg_kind]
            s["n_legs"] += 1
            s["profit"] += profit
            if profit > 0:
                s["n_wins"] += 1
            else:
                s["loss"] += profit
    home, up = stats["osc"], stats["osc_up"]
    n_total = home["n_legs"] + up["n_legs"]
    return {
        "n_legs": n_total,
        "n_wins": home["n_wins"] + up["n_wins"],
        "win_rate": (round((home["n_wins"] + up["n_wins"]) / n_total, 4)
                     if n_total else None),
        "profit_total": round(home["profit"] + up["profit"], 2),
        "loss_total": round(home["loss"] + up["loss"], 2),
        "home": {"n_legs": home["n_legs"], "n_wins": home["n_wins"],
                 "profit": round(home["profit"], 2),
                 "loss": round(home["loss"], 2)},
        "upshift": {"n_legs": up["n_legs"], "n_wins": up["n_wins"],
                    "profit": round(up["profit"], 2),
                    "loss": round(up["loss"], 2)},
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
            "n_osc_domain_rejects": c.get("n_osc_domain_rejects"),
            "n_osc_cf_rejects": c.get("n_osc_cf_rejects"),
            "n_osc_upshift_open": c.get("n_osc_upshift_open"),
        },
        "osc_legs": osc_leg_stats(res["diag"]),
        "elapsed_s": round(el, 3),
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
            "四变体矩阵不可与在册 scco/h1 判决同表比较，停。")
    print("  [fp守卫] 与在册 scco 缓存指纹一致", flush=True)


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"h3_lu_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 64}\n  {symbol} — H3 级别上移四变体矩阵（floor={FLOOR}）"
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
        print(f"  [{variant:14s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" osc腿={o['n_legs']:5d}(上移{o['upshift']['n_legs']:4d})"
              f" osc盈亏={o['profit_total']:+12.2f}"
              f" 上移开={co['n_osc_upshift_open']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
