"""Master 出场级别动态爬梯消融 — exit_mode 单轴 × OKLO/BRN/CL/BTC。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：出场级别随趋势发展动态上升）
═══════════════════════════════════════════════════════════════════════
问题：BTC 上 HoldTrend 只救 +14.5pp（vs 基线 −7.9%）。根因：entry=segment
级，41课门拦截率仅 15%——出场基准级别太低，segment 级卖点太频繁。
HighestOnly 是另一极端：exposure=1.00 伪装 buy-hold（已否证）。

解法（Climb）：入场时 exit_ladder = entry_ladder；每 bar 若 max_ladder >
exit_ladder 且该层有趋势结构（trend_state[max_ladder] ∧ dir==Up——Cycle38
宿主趋势态先例，17课 ≥2 同向中枢）⇒ exit_ladder = max_ladder。master 出场
判据 = sell1 @ exit_ladder。与 HighestOnly 的区分：升级需要高层趋势**确认**，
高层无趋势时保留原级别出场能力。

变体（单轴 exit_mode，其余与 V2oa25 逐位同）：
  V2oa25       = Signal（基线）
  V2oa25_ht    = HoldTrend（上个实验的正结果，四标的全正）
  V2oa25_cl    = Climb{hold_trend: false}
  V2oa25_clht  = Climb{hold_trend: true}（爬梯级别 sell1 ∧ 动态父级别衰竭）

守卫：
  1. 在册对账：V2oa25 与 V2oa25_ht cell 必须与 master_exit_mode_backtest.json
     逐键一致（n/total_compound/max_dd）——含 trend_flips 行的磁带对
     Signal/HoldTrend 路径零接触（cycle38 零接触守卫先例）+ Climb 改动
     不泄漏进在册两臂。
  2. 首笔入场 bar 同一性：入场侧零改动 ⇒ 四变体第一笔 entry_bar 相同。
  3. 磁带指纹守卫：bsp/div/flips 三键与在册 JSON 一致（tflips 为本实验新增键）。

观察面（BTC 重点）：
  - n_exit_climbs（爬梯事件数）+ exit reason 直方图（爬到多高的分布读数）
  - 出场次数（笔数）相对基线的收缩
  - exposure：能否接近 BH 暴露但保留出场能力（≠1.00 即非伪装 buy-hold）

认识论等级：守卫 L1；四标的回测 = L3（跨资产否证鲁棒性）。

输出：analysis/data_cache/master_exit_climb_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/master_exit_climb_backtest.py
      （env：BT_SYMBOLS=OKLO,BRN,CL,BTC  BT_FORCE=1 强制重跑）
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
OUT_JSON = DATA_DIR / "master_exit_climb_backtest.json"
REF_JSON = DATA_DIR / "master_exit_mode_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN,CL,BTC").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "V2oa25_ht", "V2oa25_cl", "V2oa25_clht"]
# 在册对账臂（master_exit_mode_backtest.json 已有 cell 的变体）
REF_VARIANTS = {"V2oa25", "V2oa25_ht"}


def run_cell(rtape, variant: str, years: float, bh: float, n_bars: int) -> dict:
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
        "n_exit_trend_holds": c["n_exit_trend_holds"],
        "n_exit_climbs": c["n_exit_climbs"],
        "n_short_diffs_total": sum(t[6] for t in raw),
        "elapsed_s": round(el, 3),
    }


def _ref_ledger_guard(symbol: str, fp: dict, variant: str, pack: dict) -> None:
    """在册对账：V2oa25/V2oa25_ht 与 master_exit_mode 任务同名 cell 一致
    = trend_flips 行 + Climb 改动对在册两臂零接触的 L2 尺度证据。"""
    if not REF_JSON.exists():
        print(f"  [在册对账 {variant}] skip（无在册 exit_mode JSON）", flush=True)
        return
    book = json.loads(REF_JSON.read_text())
    if symbol not in book or variant not in book[symbol].get("cells", {}):
        print(f"  [在册对账 {variant}] skip（在册无 {symbol}/{variant} cell）",
              flush=True)
        return
    ref = book[symbol]
    keys = ("bsp", "div", "flips")
    if {k: fp[k] for k in keys} != {k: ref["tape_fp"][k] for k in keys}:
        print(f"  [在册对账 {variant}] skip（磁带指纹不同）", flush=True)
        return
    ref_m = ref["cells"][variant]["metrics"]
    cur_m = pack["metrics"]
    for k in ("n", "total_compound", "max_dd"):
        if cur_m[k] != ref_m[k]:
            raise SystemExit(
                f"FAIL 在册对账 {symbol}/{variant} {k}：在册 {ref_m[k]} vs "
                f"本次 {cur_m[k]}——trend_flips/Climb 改动泄漏进在册路径")
    print(f"  [在册对账 {variant}] PASS（与在册 exit_mode cell 逐键一致）",
          flush=True)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — master 出场级别爬梯消融（floor={FLOOR}）"
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
        if variant in REF_VARIANTS:
            _ref_ledger_guard(symbol, fp, variant, pack)
        if variant == "V2oa25":
            base_pack = pack
        elif base_pack is not None:
            # 首笔入场 bar 同一性守卫（入场侧零改动；此后时序合法分叉）
            if pack["first_entry_bar"] != base_pack["first_entry_bar"]:
                raise SystemExit(
                    f"FAIL 首笔入场同一性 {symbol}/{variant}：基线 "
                    f"{base_pack['first_entry_bar']} vs {pack['first_entry_bar']}"
                    f"——exit_mode 改写了入场触发（应零改动）")
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"]
                  - base_pack["metrics"]["total_compound"], 1)
            if base_pack is not None else None)
        m = pack["metrics"]
        print(f"  [{variant:12s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25']:+8.1f}pp 笔={m['n']:4d}"
              f" 胜率={m['win_rate']:.2f} MDD={m['max_dd']:.1f}%"
              f" 暴露={pack['exposure']:.2f}"
              f" climbs={pack['n_exit_climbs']}"
              f" holds={pack['n_exit_trend_holds']}"
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
