"""Master 出场状态驱动消融 — exit_mode 单轴 × OKLO/BRN/CL/BTC。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：master 出场从事件驱动改为状态驱动）
═══════════════════════════════════════════════════════════════════════
问题：master 见 entry 级 confirmed sell1 即清仓。BTC 超强单边（BH +1380%）
上 P5 基线 −45%——频繁出场丢掉趋势主体段利润。

解法（49课"利润最大化"持币不动 + 41课"大级别走势没有衰竭时不做反向"）：
  Signal（在册）   = entry_ladder 的 confirmed sell1 即出（事件驱动）
  HoldTrend（_ht） = sell1 ∧ 父级别（entry_ladder+1）上行趋势衰竭才出；
                     趋势未衰竭（TrendExhaustion 正面延续证据成立）时
                     master 持仓，短差交给 voice（V2oa25 在册逻辑零接触）
  HighestOnly（_ho）= 只认当前最高涌现层（max_ladder）的 sell1（最激进）

变体：V2oa25（基线）/ V2oa25_ht / V2oa25_ho——单轴 exit_mode，其余逐位同。

守卫：
  1. O0≡P5 / 基线零接触：exit_mode=Signal 路径位等价（cargo 127 项测试 +
     V2oa25 cell 与在册 cycle38_voice_backtest.json 对账，OKLO/BRN）。
  2. 首笔入场 bar 同一性：入场侧零改动 ⇒ 三变体第一笔 entry_bar 必须相同
     （此后入场时序合法分叉——出场时点不同改写 FLAT 窗口）。
  3. 磁带指纹守卫（tape_fp）：与在册任务同口径。

观察面：
  - 笔数/胜率/复利/Sharpe/MDD + Δ(BH) + Δ(基线)
  - n_exit_trend_holds（HoldTrend 拦截出场的 bar 数——门活性证据）
  - 出场 reason 直方图 + 平均持仓 bar 数 + 暴露比（持仓 bar/总 bar，
    区分"趋势持有贡献"与"暴露伪影"——递归建仓否证先例）

认识论等级：守卫 L1；四标的回测 = L3（跨资产否证鲁棒性——BTC 单边 /
OKLO 强趋势 / BRN+CL 期货震荡，若 Δ 全负即状态驱动出场被否证）。

输出：analysis/data_cache/master_exit_mode_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/master_exit_mode_backtest.py
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
OUT_JSON = DATA_DIR / "master_exit_mode_backtest.json"
CYCLE38_JSON = DATA_DIR / "cycle38_voice_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN,CL,BTC").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "V2oa25_ht", "V2oa25_ho"]


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
    """V2oa25 基线对账：与在册 cycle38 任务同名 cell 一致 = exit_mode 改动
    对 Signal 路径零接触的 L2 尺度证据。"""
    if not CYCLE38_JSON.exists():
        print("  [基线对账] skip（无在册 cycle38 JSON）", flush=True)
        return
    book = json.loads(CYCLE38_JSON.read_text())
    if symbol not in book or "V2oa25" not in book[symbol].get("cells", {}):
        print(f"  [基线对账] skip（在册无 {symbol}/V2oa25 cell）", flush=True)
        return
    ref = book[symbol]
    ref_fp = {k: ref["tape_fp"][k] for k in ("bsp", "div", "flips")}
    if {k: fp[k] for k in ("bsp", "div", "flips")} != ref_fp:
        print(f"  [基线对账] skip（磁带指纹不同：{ref_fp} vs {fp}）", flush=True)
        return
    ref_m = ref["cells"]["V2oa25"]["metrics"]
    cur_m = pack["metrics"]
    for k in ("n", "total_compound", "max_dd"):
        if cur_m[k] != ref_m[k]:
            raise SystemExit(
                f"FAIL 基线对账 {symbol}/V2oa25 {k}：在册 {ref_m[k]} vs "
                f"本次 {cur_m[k]}——exit_mode 改动泄漏进 Signal 路径")
    print("  [基线对账] PASS（V2oa25 与在册 cycle38 cell 逐键一致）", flush=True)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — master 出场模式消融（floor={FLOOR}）"
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
    # years 是逐 bar 数组（extended_metrics 消费）——不持久化（首跑 149MB 教训）
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
        print(f"  [{variant:10s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25']:+8.1f}pp 笔={m['n']:4d}"
              f" 胜率={m['win_rate']:.2f} MDD={m['max_dd']:.1f}%"
              f" 暴露={pack['exposure']:.2f}"
              f" trend_holds={pack['n_exit_trend_holds']}"
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
