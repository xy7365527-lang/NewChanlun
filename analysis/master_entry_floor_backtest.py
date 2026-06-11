"""Master 入场级别下限消融 — entry_min_ladder 单轴 × OKLO/BRN/CL/BTC。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者：BTC master 出场问题入场侧调研）
═══════════════════════════════════════════════════════════════════════
前序：出场侧四方案全否证（Signal 基线 / HoldTrend 仅 +14.5pp / HighestOnly
伪装 buy-hold / Climb 爆仓域）。BTC 死因 = entry 级（segment, ladder=2）
出场→重入循环 1254 次每次期望为负（exit_reasons 直方图 = 入场级别分布代理：
segment 1254 / move(L1) 188 / recL2 17 / recL3 2）。

假设（剩余开放轴 1：入场级别提高）：FLAT 布防下界从 segment 抬到 move(L1)
（e3）或 recL2（e4），出场判据级别自动 = 入场级别——**对称升级**，不违反
Climb 否证的"出场判据级别不可高于入场级别"定理。循环数从 1254+ 砍到
≈188（e3）/ ≈17（e4），直接消除往返损耗的主要来源。

变体：V2oa25（基线）/ V2oa25_e3 / V2oa25_e3ht / V2oa25_e4 / V2oa25_e4ht
——单轴 entry_min_ladder（ht 臂叠加在册正确认的 HoldTrend 出场门）。

预注册判据：
  - BTC：e3/e3ht Δ(base) 显著为正且非纯暴露伪影（增量 ≉ BH×Δ暴露）⇒
    "入场级别太低"假设确认；全臂负 ⇒ 入场侧关闭，接受方向 4（BTC 适合
    BH 不适合 P5）。
  - OKLO/BRN/CL 交叉检查：segment 级循环在这些标的为正贡献（基线 alpha
    来源），预期 e3 砍掉它们后 Δ(base) 为负——若如此，结论形态 =
    "入场级别是标的属性"（REV 白名单同构），非全局开关。

守卫：
  1. 基线零接触：entry_min_ladder=0 路径位等价（cargo 128 项测试 +
     V2oa25 cell 与在册 master_exit_mode_backtest.json 逐键对账）。
  2. 磁带指纹守卫（tape_fp）：与在册任务同口径。
  3. 首笔入场 bar 同一性守卫**不适用**（入场侧改动是本轴语义——改为
     记录 first_entry_bar 漂移作为观察面）。

认识论等级：守卫 L1；四标的回测 = L3（跨资产，可产生否定性结果）。

输出：analysis/data_cache/master_entry_floor_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/master_entry_floor_backtest.py
      （env：BT_SYMBOLS=BTC,OKLO,BRN,CL  BT_FORCE=1 强制重跑）
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
OUT_JSON = DATA_DIR / "master_entry_floor_backtest.json"
REF_JSON = DATA_DIR / "master_exit_mode_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "BTC,OKLO,BRN,CL").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "V2oa25_e3", "V2oa25_e3ht", "V2oa25_e4", "V2oa25_e4ht"]


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
        "n_short_diffs_total": sum(t[6] for t in raw),
        "elapsed_s": round(el, 3),
    }


def _baseline_ledger_guard(symbol: str, fp: dict, pack: dict) -> None:
    """V2oa25 基线对账：与在册 master_exit_mode 任务同名 cell 一致 =
    entry_min_ladder 改动对默认路径零接触的 L2 尺度证据。"""
    if not REF_JSON.exists():
        print("  [基线对账] skip（无在册 exit_mode JSON）", flush=True)
        return
    book = json.loads(REF_JSON.read_text())
    if symbol not in book or "V2oa25" not in book[symbol].get("cells", {}):
        print(f"  [基线对账] skip（在册无 {symbol}/V2oa25 cell）", flush=True)
        return
    ref = book[symbol]
    if {k: fp[k] for k in ("bsp", "div", "flips")} != ref["tape_fp"]:
        print(f"  [基线对账] skip（磁带指纹不同：{ref['tape_fp']} vs {fp}）",
              flush=True)
        return
    ref_m = ref["cells"]["V2oa25"]["metrics"]
    cur_m = pack["metrics"]
    for k in ("n", "total_compound", "max_dd"):
        if cur_m[k] != ref_m[k]:
            raise SystemExit(
                f"FAIL 基线对账 {symbol}/V2oa25 {k}：在册 {ref_m[k]} vs "
                f"本次 {cur_m[k]}——entry_min_ladder 改动泄漏进默认路径")
    print("  [基线对账] PASS（V2oa25 与在册 exit_mode cell 逐键一致）",
          flush=True)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — master 入场级别下限消融（floor={FLOOR}）"
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
        if variant == "V2oa25":
            base_pack = pack
            _baseline_ledger_guard(symbol, fp, pack)
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"]
                  - base_pack["metrics"]["total_compound"], 1)
            if base_pack is not None and variant != "V2oa25" else None)
        # 入场侧改动是本轴语义——first_entry_bar 漂移是观察面非守卫
        pack["entry_bar_shift"] = (
            pack["first_entry_bar"] - base_pack["first_entry_bar"]
            if base_pack is not None and variant != "V2oa25"
            and pack["first_entry_bar"] is not None
            and base_pack["first_entry_bar"] is not None else None)
        m = pack["metrics"]
        print(f"  [{variant:12s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25'] or 0:+8.1f}pp 笔={m['n']:4d}"
              f" 胜率={m['win_rate']:.2f} MDD={m['max_dd']:.1f}%"
              f" 暴露={pack['exposure']:.2f}"
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
