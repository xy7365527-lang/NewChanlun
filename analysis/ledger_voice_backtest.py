"""并发赋格最小可验证实验 — 账本 voice（VL/VLs1）vs FSM voice（V2oa25/O0）。

背景：concurrent_fugue_deep_think.md Part II §17 开放问题3（控制状态剥离）
的最小化形态。Part II 命题：控制状态（相位/配对/锚记忆）是（结构×账本）
的可失效缓存，可全部消除；voice 的严格形式是"有门的账本"。本实验连门
都不留（裸账本），把全部差额一次性显形。

设计（recursive_position_experiment.md 教训的规避）：
  递归建仓否证的死因 = 从零建仓 + BSP 事件全清 ⇒ 暴露塌缩（slice 占用
  时间 ≪ 趋势长度）。本实验 master 满仓持有逻辑零接触（Full 入场 +
  Signal 出场在册路径）——暴露由 master 兜底，只隔离 voice 消费方式：
    FSM（在册）：main 腿（带门带锚）/ REV 腿（相位+配对+θ门+41课门）
    Ledger（VL）：confirmed Sell@k 且槽空 → 卖出 frac_k；confirmed
                  Buy@k 且槽开 → 买回。无相位、无锚、无门、无配对。
    VLs1：VL + 卖侧词汇收缩（仅 Sell1 开）——递归建仓 s1 臂的镜像消融。

预注册判据：
  VL ≥ V2oa25 ⇒ FSM 控制状态+门是冗余缓存（Part II 命题 L2 确认）；
  VL < V2oa25 ⇒ 差额 = 控制状态+门的联合信息含量，VLs1 分解卖侧词汇轴，
                n_ledger_* 计数器定位事件消费量差异。
  对照锚：O0（P5 域腿 = 带门带锚的账本）隔离"门+锚 vs 裸"的因果。

用法：BT_SYMBOLS=OKLO python analysis/ledger_voice_backtest.py
"""

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
OUT_JSON = DATA_DIR / "ledger_voice_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V2oa25", "O0", "VL", "VLs1"]

LEDGER_KEYS = ("n_ledger_opens", "n_ledger_closes",
               "n_ledger_sell_noops", "n_ledger_buy_noops")
FSM_KEYS = ("n_rev_open", "n_rev_close_t5", "n_rev_zd_close",
            "n_rev_l41_rejects", "n_rev_depth_rejects",
            "n_osc_open", "n_close_normal", "n_open_gate_rejects")


def run_cell(rtape, variant: str, years: float, bh: float) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    # 逐腿 payoff（diag trace：voice 短差兑现分布）
    profits = []
    for _header, diffs in res["diag"]:
        for (_key, _leg, _sb, _sp, _bb, _bp), (_sh, _df, pf, *_rest) in diffs:
            profits.append(pf)
    profits.sort()
    n_p = len(profits)
    payoff = {
        "n": n_p,
        "p25": round(profits[n_p // 4], 2) if n_p else None,
        "p50": round(profits[n_p // 2], 2) if n_p else None,
        "p75": round(profits[3 * n_p // 4], 2) if n_p else None,
        "mean": round(sum(profits) / n_p, 2) if n_p else None,
        "win_rate": round(sum(1 for p in profits if p > 0) / n_p, 3)
        if n_p else None,
    }
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "leg_stats": _leg_stats_rust(res["diag"]),
        "diff_payoff": payoff,
        "counters": {k: c[k] for k in LEDGER_KEYS + FSM_KEYS},
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 账本 voice 最小实验（floor={FLOOR}）"
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
                         f"——引擎/信号层语义已变，先核对引擎变更。")

    rtape = pack_tape(tape, dir_flips=dir_flips)

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
        po = pack["diff_payoff"]
        print(f"  [{variant:7s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
              f" Δ(base)={pack['delta_vs_v2oa25']:+8.1f}pp 笔={m['n']:4d}"
              f" | ledger o/c/sn/bn={c['n_ledger_opens']}/"
              f"{c['n_ledger_closes']}/{c['n_ledger_sell_noops']}/"
              f"{c['n_ledger_buy_noops']}"
              f" | 短差 n={po['n']} wr={po['win_rate']}"
              f" p50={po['p50']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        results[sym] = process_symbol(sym, results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=1))
        print(f"  → 已落盘 {OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
