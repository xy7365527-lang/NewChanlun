"""操作判据严格形式替换回测 — V2oa25_ht 基线 × not6/o3s × OKLO/BRN。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 操作判据严格形式审计，analysis/strict_form_audit.md §3）
═══════════════════════════════════════════════════════════════════════
两个可立即替换验证的严格形式（审计表 4b/5c）：
  not6 = pre_type3=false：去 candidate Buy3 预回补/预闭腿——原文无
         candidate 操作点（38课答疑:160 三买=回抽完成的确认）；
  o3s  = osc_sell3_no_recover=true：49课"一旦出现第三类卖点，就不能
         回补了"——osc 腿锚中枢死亡不强制回补。

守卫：
  1. 基线零泄漏：V2oa25_ht cell 与在册 master_exit_mode_backtest.json
     同名 cell 逐键一致（n/total_compound/max_dd）——新配置位默认 false
     对在册路径位等价的 L2 尺度证据。
  2. 首笔入场 bar 同一性：两替换均不触入场侧。
  3. 磁带指纹守卫（tape_fp）：与在册任务同口径。

认识论等级：双标的跨 regime = L3。
输出：analysis/data_cache/strict_form_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/strict_form_backtest.py
      （env：BT_SYMBOLS=OKLO,BRN  BT_FORCE=1 强制重跑）
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import Counter
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
OUT_JSON = DATA_DIR / "strict_form_backtest.json"
REF_JSON = DATA_DIR / "master_exit_mode_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25_ht", "V2oa25_ht_not6", "V2oa25_ht_o3s"]


def run_cell(rtape, variant: str, years, bh: float, n_bars: int) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=False)
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
        "first_entry_bar": raw[0][0] if raw else None,
        "exit_reasons": dict(Counter(t[5] for t in raw)),
        "avg_hold_bars": round(hold_bars / len(raw), 1) if raw else None,
        "exposure": round(hold_bars / n_bars, 4),
        "n_short_diffs_total": sum(t[6] for t in raw),
        # 替换轴可观测面
        "n_close_pre_type3": c["n_close_pre_type3"],
        "n_rev_close_t6": c["n_rev_close_t6"],
        "n_osc_open": c["n_osc_open"],
        "n_osc_zd_close": c["n_osc_zd_close"],
        "n_osc_dead_holds": c["n_osc_dead_holds"],
        "elapsed_s": round(el, 3),
    }


def _baseline_guard(symbol: str, fp: dict, pack: dict) -> None:
    """V2oa25_ht 基线对账：与在册 master_exit_mode 任务同名 cell 一致 =
    新配置位（osc_sell3_no_recover 默认 false）对在册路径零泄漏。"""
    if not REF_JSON.exists():
        print("  [基线对账] skip（无在册 master_exit_mode JSON）", flush=True)
        return
    book = json.loads(REF_JSON.read_text())
    if symbol not in book or "V2oa25_ht" not in book[symbol].get("cells", {}):
        print(f"  [基线对账] skip（在册无 {symbol}/V2oa25_ht cell）", flush=True)
        return
    ref = book[symbol]
    ref_fp = {k: ref["tape_fp"][k] for k in ("bsp", "div", "flips")}
    if {k: fp[k] for k in ("bsp", "div", "flips")} != ref_fp:
        print(f"  [基线对账] skip（磁带指纹不同：{ref_fp} vs {fp}）", flush=True)
        return
    ref_m = ref["cells"]["V2oa25_ht"]["metrics"]
    cur_m = pack["metrics"]
    for k in ("n", "total_compound", "max_dd"):
        if cur_m[k] != ref_m[k]:
            raise SystemExit(
                f"FAIL 基线对账 {symbol}/V2oa25_ht {k}：在册 {ref_m[k]} vs "
                f"本次 {cur_m[k]}——严格形式改动泄漏进基线路径")
    print("  [基线对账] PASS（V2oa25_ht 与在册 cell 逐键一致）", flush=True)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 严格形式替换回测（floor={FLOOR}）"
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
    print(f"  信号层 {sig_s:.1f}s  D3 翻转 {len(dir_flips):,}", flush=True)

    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}"
                         f"——引擎/信号层语义已变，全量重跑或先核对引擎变更。")

    rtape = pack_tape(tape, dir_flips=dir_flips)

    out = prev if prev is not None else {}
    out.update({"n_bars": n, "bh": round(bh, 2),
                "sig_elapsed": round(sig_s, 1), "tape_fp": fp,
                "floor": FLOOR})
    out.setdefault("cells", {})
    force = os.environ.get("BT_FORCE", "0") == "1"
    base_pack = None
    for variant in VARIANTS:
        if variant in out["cells"] and not force:
            pack = out["cells"][variant]
        else:
            pack = run_cell(rtape, variant, years, bh, n)
            out["cells"][variant] = pack
        if variant == "V2oa25_ht":
            base_pack = pack
            _baseline_guard(symbol, fp, pack)
        elif base_pack is not None:
            if pack["first_entry_bar"] != base_pack["first_entry_bar"]:
                raise SystemExit(
                    f"FAIL 首笔入场同一性 {symbol}/{variant}：基线 "
                    f"{base_pack['first_entry_bar']} vs {pack['first_entry_bar']}"
                    f"——严格形式替换改写了入场触发（应零改动）")
        pack["delta_vs_base"] = (
            round(pack["metrics"]["total_compound"]
                  - base_pack["metrics"]["total_compound"], 1)
            if base_pack is not None else None)
        m = pack["metrics"]
        print(f"  [{variant:16s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_base']:+8.1f}pp 笔={m['n']:4d}"
              f" 胜率={m['win_rate']:.2f} MDD={m['max_dd']:.1f}%"
              f" t6={pack['n_rev_close_t6']}"
              f" pre={pack['n_close_pre_type3']}"
              f" oscZD={pack['n_osc_zd_close']}"
              f" deadHold={pack['n_osc_dead_holds']}"
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
    print("\n[done] 全部标的完成", flush=True)


if __name__ == "__main__":
    main()
