"""38课循环 voice 回测 — V2oa25（单次 REV）vs V2oa25C38（趋势存续期循环短差）。

═══════════════════════════════════════════════════════════════════════
任务与矩阵（2026-06-11 编排者任务：38课循环 voice 实装）
═══════════════════════════════════════════════════════════════════════
循环定义（第38课）：宿主（ladder+1）趋势存续期间，voice@k 反复
  本级别卖点 → 卖出（短差开）→ 次级别（k−1）买点 → 买回（短差闭），
循环终止 = 宿主趋势态翻落（尾 move kind 离开 Trend ∨ direction 离开 Up）。

矩阵：OKLO + BRN × floor=2（在册口径）×
  { V2oa25 = 在册默认门基线（单次 REV：θ 自适应 q25 + 成本门下界 + 41课门）
    V2oa25C38 = rev_cycle=Cycle38（41课门改绑 master——循环路径不消费
                rev_l41_gate；每条循环腿过 35课成本门）}

守卫：
  1. 零接触守卫：同一磁带含/不含 trend_flips 行，O0 与 V2oa25 trades 逐位
     相同（趋势态行对非循环变体零接触——O0≡P5 在册等价链不被本次改动触碰，
     cargo 109 项测试 + 本守卫合取）。
  2. 磁带指纹守卫（tape_fp，含 trend 翻转数）：禁止跨引擎语义混表。

认识论等级：守卫 L1（管线等价）；OKLO/BRN 单 floor 回测 L2→两标的 L3
（可否证：循环假设若 Δ 全负即被否证）。

输出：analysis/data_cache/cycle38_voice_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/cycle38_voice_backtest.py
      （env：BT_SYMBOLS=OKLO,BRN  BT_FORCE=1 强制重跑）
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
from organic_fugue_rust_backtest import (  # noqa: E402
    _leg_stats_rust,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "cycle38_voice_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "V2oa25C38"]

C38_KEYS = ("n_c38_enter", "n_c38_exit", "n_c38_open", "n_c38_close",
            "n_c38_forced_close", "n_c38_cost_rejects",
            "n_c38_cost_noref_rejects", "n_c38_frozen_rejects",
            "c38_pairs", "c38_wins")
REV_KEYS = ("n_rev_attempts", "n_rev_open", "n_rev_open_osc",
            "n_rev_close_t5", "n_rev_close_t6", "n_rev_close_t7",
            "n_rev_zd_close", "n_rev_l41_rejects", "n_rev_depth_rejects",
            "n_rev_theta_fallbacks", "n_rev_theta_cost_floor")


def _zero_touch_guard(rtape_plain, rtape_trend) -> None:
    """趋势态行对非循环变体零接触：O0/V2oa25 在两磁带上 trades 逐位相同。"""
    for v in ("O0", "V2oa25"):
        a = nr.run_organic_rust(rtape_plain, v, floor_ladder=FLOOR,
                                stop_mode="none", diag=False)
        b = nr.run_organic_rust(rtape_trend, v, floor_ladder=FLOOR,
                                stop_mode="none", diag=False)
        if a["trades"] != b["trades"]:
            raise SystemExit(f"FAIL 零接触守卫 {v}：含/不含 trend_flips 的"
                             f" trades 不一致——趋势态行泄漏进非循环路径")
        for k, va in a["counters"].items():
            if va != b["counters"][k]:
                raise SystemExit(f"FAIL 零接触守卫 {v} counter {k}")
    print(f"  [零接触守卫] PASS（O0/V2oa25 两磁带逐位相同）", flush=True)


def run_cell(rtape, variant: str, years: float, bh: float) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    # 循环腿逐对 payoff 分布（diag rev 腿 trace：profit 列）
    rev_profits = []
    for _header, diffs in res["diag"]:
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1) in diffs:
            if leg == "rev":
                rev_profits.append(pf)
    rev_profits.sort()
    n_rp = len(rev_profits)
    payoff = {
        "n": n_rp,
        "p25": rev_profits[n_rp // 4] if n_rp else None,
        "p50": rev_profits[n_rp // 2] if n_rp else None,
        "p75": rev_profits[3 * n_rp // 4] if n_rp else None,
        "mean": round(sum(rev_profits) / n_rp, 2) if n_rp else None,
    }
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "leg_stats": _leg_stats_rust(res["diag"]),
        "rev_payoff_dist": payoff,
        "counters": {k: c[k] for k in C38_KEYS + REV_KEYS},
        "c38_net_cash": round(c["c38_net_cash"], 2),
        "rev_osc_net_cash": round(c["rev_osc_net_cash"], 2),
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 38课循环 voice 回测（floor={FLOOR}）"
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
    print(f"  信号层 {sig_s:.1f}s  D3 翻转 {len(dir_flips):,}"
          f"  趋势态翻转 {len(trend_flips):,}", flush=True)

    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}"
                         f"——引擎/信号层语义已变，全量重跑或先核对引擎变更。")

    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)
    rtape_plain = pack_tape(tape, dir_flips=dir_flips)
    _zero_touch_guard(rtape_plain, rtape)
    del rtape_plain

    out = prev if prev is not None else {}
    out.update({"n_bars": n, "bh": round(bh, 2), "years": years,
                "sig_elapsed": round(sig_s, 1), "tape_fp": fp,
                "floor": FLOOR})
    out.setdefault("cells", {})
    force = os.environ.get("BT_FORCE", "0") == "1"
    base_compound = None
    for variant in VARIANTS:
        if variant in out["cells"] and not force:
            pack = out["cells"][variant]
        else:
            pack = run_cell(rtape, variant, years, bh)
            out["cells"][variant] = pack
        if variant == "V2oa25":
            base_compound = pack["metrics"]["total_compound"]
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"] - base_compound, 1)
            if base_compound is not None else None)
        m = pack["metrics"]
        c = pack["counters"]
        print(f"  [{variant:10s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25']:+7.1f}pp 笔={m['n']:4d}"
              f" | c38 enter/open/close/forced="
              f"{c['n_c38_enter']}/{c['n_c38_open']}/{c['n_c38_close']}/"
              f"{c['n_c38_forced_close']}"
              f" pairs={c['c38_pairs']} wins={c['c38_wins']}"
              f" cash={pack['c38_net_cash']:+.0f}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        done = (sym in results
                and all(v in results[sym].get("cells", {}) for v in VARIANTS))
        if done and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有全部 cell", flush=True)
            continue
        results[sym] = process_symbol(sym, prev=results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        print(f"  [{sym}] 已落盘", flush=True)


if __name__ == "__main__":
    main()
