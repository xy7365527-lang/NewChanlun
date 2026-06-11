"""Master 入场侧递归建仓回测 — V2oa25_rec* vs V2oa25 基线 × OKLO+BRN。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：Master 入场侧递归建仓实验）
═══════════════════════════════════════════════════════════════════════
保持 master持仓+voice短差 架构，只改 master 入场过程（entry_mode 单轴）：
  Full（在册）   = 区间套确认 bar 一次性满仓
  Recursive     = 同一 bar 只部署 base_frac；此后更高级别 confirmed buy1
                  逐档追加（quota exp2 翻倍）；buy1 落在当前最高涌现层
                  = "最高级别确认" → 补满剩余。出场零改动（entry_ladder
                  的 sell1 一次性全清，未部署现金计入 total_value）。
voice（V2oa25 REV+域腿）逐位不碰。

变体：V2oa25（基线）/ V2oa25_rec（base=0.2）/ V2oa25_rec3（1/3）/
      V2oa25_rec5（0.5）。

守卫：
  1. O0≡P5 / 基线零接触：entry_mode=Full 路径位等价（cargo 126 项测试 +
     本脚本将 V2oa25 结果与在册 cycle38_voice_backtest.json cell 对账）。
  2. 入场 bar 同一性：rec 变体的 entry_bar 序列必须与基线逐位相同
     （入场触发逻辑零改动的可观测断言；earning 触发率 >0 时可豁免）。
  3. 磁带指纹守卫（tape_fp）：与在册任务同口径。

观察面（任务规格四问）：
  - 多少笔交易完成递归建仓（n_rec_entry_full / 笔数）
  - 满仓入场 entry 价 vs 递归入场加权均价（vwap_ratio = vwap/entry_price，
    同 bar 同一 trade 直接可比；<1 = 递归入场摊低成本）
  - 总收益变化（total_compound / Δ vs BH / Δ vs 基线）
  - MDD 变化（trade 级 max_dd，与基线同一口径）

认识论等级：守卫 L1；OKLO+BRN 双标的回测 = L2（可否证：递归入场若
Δ 全负即被否证——错过涨幅 > 入场摊薄收益）。

输出：analysis/data_cache/master_recursive_entry_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/master_recursive_entry_backtest.py
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
from organic_fugue_rust_backtest import _trades_from_rust  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "master_recursive_entry_backtest.json"
CYCLE38_JSON = DATA_DIR / "cycle38_voice_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "V2oa25_rec", "V2oa25_rec3", "V2oa25_rec5"]

REC_KEYS = ("n_rec_entry_fills", "n_rec_entry_full",
            "n_rec_entry_partial_exits", "n_rec_entry_earning_rejects")


def _rec_trade_stats(trades_raw: list, rec_fills: list) -> dict:
    """按 trade 的 [entry_bar, exit_bar] 切分 rec_fills，重建逐仓入场过程。

    rec_fills 行 = (bar, ladder, frac, price)，含初始档。vwap_ratio =
    加权均价/首档价（<1 = 递归追加摊低了相对满仓入场的成本）。
    """
    fills = sorted(rec_fills)
    per_trade = []
    fi = 0
    for (eb, ep, xb, _xp, _pnl, _reason, _nd, _cb) in trades_raw:
        rows = []
        while fi < len(fills) and fills[fi][0] <= xb:
            if fills[fi][0] >= eb:
                rows.append(fills[fi])
            fi += 1
        if not rows:
            continue  # Full 模式（基线）恒空
        tot_frac = sum(r[2] for r in rows)
        vwap = sum(r[2] * r[3] for r in rows) / tot_frac
        full_bar = rows[-1][0] if abs(tot_frac - 1.0) < 1e-9 else None
        per_trade.append({
            "entry_bar": eb, "n_fills": len(rows),
            "filled_frac": round(tot_frac, 6),
            "vwap_ratio": round(vwap / ep, 6),
            "bars_to_full": (full_bar - eb) if full_bar is not None else None,
        })
    if not per_trade:
        return {}
    n = len(per_trade)
    full = [t for t in per_trade if t["bars_to_full"] is not None]
    return {
        "n_trades_with_fills": n,
        "n_reached_full": len(full),
        "avg_fills_per_trade": round(sum(t["n_fills"] for t in per_trade) / n, 2),
        "avg_filled_frac_at_exit": round(
            sum(t["filled_frac"] for t in per_trade) / n, 4),
        "avg_vwap_ratio": round(sum(t["vwap_ratio"] for t in per_trade) / n, 6),
        "avg_bars_to_full": (round(sum(t["bars_to_full"] for t in full)
                                   / len(full), 1) if full else None),
        "per_trade": per_trade,
    }


def run_cell(rtape, variant: str, years: float, bh: float) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=False)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "entry_bars": [t[0] for t in res["trades"]],
        "counters": {k: c[k] for k in REC_KEYS},
        "n_earning_reached": c["n_earning_reached"],
        "rec_stats": _rec_trade_stats(res["trades"], res["rec_fills"]),
        "elapsed_s": round(el, 3),
    }


def _baseline_ledger_guard(symbol: str, fp: dict, pack: dict) -> None:
    """V2oa25 基线对账：与在册 cycle38 任务的同名 cell 数字一致 = 本次
    改动对 entry_mode=Full 路径零接触的 L2 尺度证据。"""
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
                f"本次 {cur_m[k]}——entry_mode 改动泄漏进 Full 路径")
    print("  [基线对账] PASS（V2oa25 与在册 cycle38 cell 逐键一致）", flush=True)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — master 递归建仓回测（floor={FLOOR}）"
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
    out.update({"n_bars": n, "bh": round(bh, 2), "years": years,
                "sig_elapsed": round(sig_s, 1), "tape_fp": fp,
                "floor": FLOOR})
    out.setdefault("cells", {})
    force = os.environ.get("BT_FORCE", "0") == "1"
    base_pack = None
    for variant in VARIANTS:
        if variant in out["cells"] and not force:
            pack = out["cells"][variant]
        else:
            pack = run_cell(rtape, variant, years, bh)
            out["cells"][variant] = pack
        if variant == "V2oa25":
            base_pack = pack
            _baseline_ledger_guard(symbol, fp, pack)
        else:
            # 入场 bar 同一性守卫（earning 触发时豁免——相位差可改写出场时序）
            if (base_pack is not None
                    and pack["entry_bars"] != base_pack["entry_bars"]):
                if (pack["n_earning_reached"] == 0
                        and base_pack["n_earning_reached"] == 0):
                    raise SystemExit(
                        f"FAIL 入场 bar 同一性 {symbol}/{variant}：递归入场"
                        f"改写了入场触发时序（应零改动）")
                print(f"  [入场守卫] WARN {variant} entry_bars 偏离"
                      f"（earning 相位差豁免）", flush=True)
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"]
                  - base_pack["metrics"]["total_compound"], 1)
            if base_pack is not None else None)
        m = pack["metrics"]
        rs = pack["rec_stats"]
        rec_str = (f" | fills/笔={rs['avg_fills_per_trade']}"
                   f" 满仓={rs['n_reached_full']}/{rs['n_trades_with_fills']}"
                   f" 出场仓位={rs['avg_filled_frac_at_exit']:.2f}"
                   f" vwap比={rs['avg_vwap_ratio']:.4f}" if rs else "")
        print(f"  [{variant:12s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25']:+7.1f}pp 笔={m['n']:4d}"
              f" MDD={m['max_dd']:.1f}%{rec_str}"
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
