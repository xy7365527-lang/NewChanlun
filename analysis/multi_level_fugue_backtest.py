"""多级别赋格回测 — master+voice 双级别并发（2026-06-11 编排者任务）。

═══════════════════════════════════════════════════════════════════════
任务与矩阵
═══════════════════════════════════════════════════════════════════════
目标配置：master = 高级别 P5（49课：sell1 出场、下跌段持币不动），
          voice  = 次级别 38课（向下段先卖后买降成本），
          共享 OrganicLedger 仓位、SlotKey 槽隔离独立决策。

架构事实（rust/src/trading/runner.rs）：master 级别 = entry_ladder
（FLAT→ARMED 取最高 confirmed buy1 层，区间套次级别确认入场）——数据驱动
的动态级别，不是配置常量。voice 域 = [floor_ladder, entry_ladder)。
因此"master=L2、voice=L1"在本架构的严格形式是：
  floor_ladder = LADDER_MOVE(3) ⇒ voice 域从 move(L1) 起；
  entry_ladder ≥ 4（recL2+）的 trade 子集 = "master 在 L2+ 操作"。
master 级别归属由 ladder_attribution 报告——任务规格的实现度是可观测
数据，不是声明。

矩阵（OKLO 447K）：
  floor ∈ { 2 = segment（在册口径，对照）, 3 = move(L1)（任务主配置） }
  变体  ∈ { V0  = P5 基线（master+osc，无 REV）
            V1  = legacy 38课段终结三触发 REV（盲配对，对照）
            V2of= 配对修正 REV（仅震荡型+θ_depth=1%+kind 配对闭腿，在册最优）}
守卫：双 floor 各做一次 Rust O0 ≡ Python O0 四面对账（trades/trace/counters）。

认识论等级：O0 对账 L1（管线等价）；floor=3 各变体 L2（OKLO 单标的真实
数据首跑，可否证）；floor 轴对比结论待 QQQ/BRN 复验后才到 L3。

输出：analysis/data_cache/multi_level_fugue_backtest.json（增量续跑）
      analysis/data_cache/multi_level_fugue_tables.md（自动表格）
      analysis/multi_level_fugue_backtest.md（手工定稿报告，脚本不覆写）
用法：PYTHONPATH=src .venv/bin/python analysis/multi_level_fugue_backtest.py
      （env：BT_SYMBOLS=OKLO  BT_FORCE=1 强制重跑）
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
from fugue_version_i import LADDER_MOVE, LADDER_SEG, extended_metrics  # noqa: E402
from organic_fugue import ORGANIC_VARIANTS, run_organic  # noqa: E402
from organic_fugue_rust_backtest import (  # noqa: E402
    _leg_stats_rust,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "multi_level_fugue_backtest.json"
OUT_MD = DATA_DIR / "multi_level_fugue_tables.md"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
FLOORS = [LADDER_SEG, LADDER_MOVE]          # 2=segment 对照 / 3=move(L1) 主配置
VARIANTS = ["V0", "V1", "V2of"]

LADDER_NAMES = ["bar", "bi", "segment", "move(L1)"] + [
    f"recL{k - 2}" for k in range(4, 11)]

MASTER_KEYS = ("n_master_rev_open", "n_master_rev_close", "n_earning_reached",
               "n_exit_upgraded")
REV_KEYS = ("n_rev_attempts", "n_rev_open", "n_rev_open_osc", "n_rev_open_esc",
            "n_rev_nocenter_rejects", "n_rev_depth_rejects", "n_rev_frozen_rejects",
            "n_rev_close_t5", "n_rev_close_t6", "n_rev_close_t7",
            "n_rev_zd_close", "n_rev_mismatch_holds")
OSC_KEYS = ("n_osc_open", "n_osc_zd_close", "n_close_normal",
            "n_close_pre_type3", "n_close_hard_type3")


def _o0_guard(tape, rtape, years: float, floor: int) -> dict:
    """Rust O0 ≡ Python O0 四面对账（floor 参数化；不过 → SystemExit）。"""
    diag_py: list = []
    trades_py, extra_py = run_organic(
        tape, floor_ladder=floor, config=ORGANIC_VARIANTS["O0"], diag=diag_py)
    res = nr.run_organic_rust(rtape, "O0", floor_ladder=floor,
                              stop_mode="none", diag=True)
    if len(trades_py) != len(res["trades"]):
        raise SystemExit(f"FAIL O0 floor={floor} 笔数 "
                         f"py={len(trades_py)} rust={len(res['trades'])}")
    for ti, (tp, tr) in enumerate(zip(trades_py, res["trades"])):
        if (tp.entry_bar, tp.entry_price, tp.exit_bar, tp.exit_price, tp.pnl_pct,
                tp.exit_reason, tp.n_short_diffs, tp.cost_basis_at_exit) != tuple(tr):
            raise SystemExit(f"FAIL O0 floor={floor} trade#{ti}")
    n_rows = 0
    for ti, (dp, (_header, diffs)) in enumerate(zip(diag_py, res["diag"])):
        if dp["n_diffs"] != len(diffs):
            raise SystemExit(f"FAIL O0 floor={floor} trade#{ti} 短差数")
        for rp, ((key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1)) \
                in zip(dp["diffs"], diffs):
            rust_row = {"ladder": key, "leg": leg, "sell_bar": sb, "sell_price": sp,
                        "buy_bar": bb, "buy_price": bp, "shares": sh, "diff": df,
                        "profit": pf, "was_earning": we, "shares_delta": sd,
                        "cost_basis_before": cb0, "cost_basis_after": cb1}
            for k in rp:
                if rp[k] != rust_row[k]:
                    raise SystemExit(
                        f"FAIL O0 floor={floor} trace {k}: "
                        f"{rp[k]!r}!={rust_row[k]!r}")
            n_rows += 1
    for k, v in extra_py["counters"].items():
        if v != res["counters"][k]:
            raise SystemExit(
                f"FAIL O0 floor={floor} counter {k}: {v}!={res['counters'][k]}")
    m = extended_metrics(trades_py, years)
    return {"n_trades": len(trades_py), "n_trace_rows": n_rows,
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")}}


def _ladder_table(attribution: list, held: list) -> dict:
    """entry_ladder 归因（master 级别分布——任务规格"master=L2"的实现度）。"""
    out = {}
    for k, (n, b) in enumerate(zip(attribution, held)):
        if n:
            out[LADDER_NAMES[k]] = {"n_trades": n, "held_bars": b}
    total = sum(attribution)
    n_l2plus = sum(n for k, n in enumerate(attribution) if k >= 4)
    out["_master_at_L2plus_pct"] = round(n_l2plus / total * 100, 1) if total else None
    return out


def run_cell(rtape, variant: str, floor: int, years: float, v0_compound: float | None,
             bh: float) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=floor,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    pack = {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "delta_vs_v0_same_floor": (round(m["total_compound"] - v0_compound, 1)
                                   if v0_compound is not None else None),
        "ladder_attribution": _ladder_table(res["ladder_attribution"],
                                            res["ladder_held_bars"]),
        "rev_opens_by_ladder": {
            LADDER_NAMES[k]: v for k, v in enumerate(res["rev_opens_by_ladder"]) if v},
        "leg_stats": _leg_stats_rust(res["diag"]),
        "counters": {k: c[k] for k in MASTER_KEYS + REV_KEYS + OSC_KEYS},
        "elapsed_s": round(el, 3),
    }
    return pack


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 多级别赋格回测（master+voice 双级别）"
          f"\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # 磁带指纹守卫（rev_v2_paired_backtest 同纪律：禁止跨引擎语义混表）
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
                "sig_elapsed": round(sig_s, 1), "n_dir_flips": len(dir_flips),
                "tape_fp": fp})
    out.setdefault("o0_guard", {})
    out.setdefault("cells", {})

    force = os.environ.get("BT_FORCE", "0") == "1"
    for floor in FLOORS:
        fkey = str(floor)
        if fkey not in out["o0_guard"] or force:
            g = _o0_guard(tape, rtape, years, floor)
            out["o0_guard"][fkey] = g
            print(f"  [O0≡P5 floor={floor}] PASS  {g['n_trades']} 笔 / "
                  f"{g['n_trace_rows']} trace 行", flush=True)
        v0_compound = None
        for variant in VARIANTS:
            ckey = f"floor{floor}/{variant}"
            if ckey in out["cells"] and not force:
                pack = out["cells"][ckey]
            else:
                pack = run_cell(rtape, variant, floor, years, v0_compound, bh)
                out["cells"][ckey] = pack
            if variant == "V0":
                v0_compound = pack["metrics"]["total_compound"]
                # 增量路径下补 delta（V0 在册时 run_cell 未执行）
                for v2 in VARIANTS[1:]:
                    k2 = f"floor{floor}/{v2}"
                    if k2 in out["cells"]:
                        out["cells"][k2]["delta_vs_v0_same_floor"] = round(
                            out["cells"][k2]["metrics"]["total_compound"]
                            - v0_compound, 1)
            m = pack["metrics"]
            c = pack["counters"]
            la = pack["ladder_attribution"]
            print(f"  [{ckey:14s}] 复利={m['total_compound']:+10.2f}%"
                  f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp"
                  f" 笔数={m['n']:4d}"
                  f" | rev开={c['n_rev_open']:4d} osc开={c['n_osc_open']:4d}"
                  f" master@L2+={la['_master_at_L2plus_pct']}%"
                  f" [{pack['elapsed_s']:.3f}s]", flush=True)
    return out


def _write_tables(results: dict) -> None:
    L: list[str] = []
    L.append("# 多级别赋格回测 — 自动表格（multi_level_fugue_backtest.py 产出）\n")
    L.append("## 主对照表\n")
    L.append("| 标的 | floor(voice域下界) | 变体 | 复利% | Δ(vs BH)pp | "
             "Δ(vs 同floor V0)pp | 笔数 | 胜率% | maxDD% | master@L2+% |")
    L.append("|---|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        for ckey, p in r["cells"].items():
            floor, variant = ckey.split("/")
            m = p["metrics"]
            d0 = p["delta_vs_v0_same_floor"]
            L.append(
                f"| {s} | {floor} | {variant} | {m['total_compound']:+.1f} | "
                f"{p['delta_vs_bh']:+.1f} | "
                f"{'—' if d0 is None else f'{d0:+.1f}'} | {m['n']} | "
                f"{m['win_rate']} | {m['max_dd']} | "
                f"{p['ladder_attribution']['_master_at_L2plus_pct']} |")
    L.append("")
    L.append("## 腿分解（main/osc/rev：对数/胜率%/净现金）\n")
    L.append("| 标的 | cell | main | osc | rev | master_rev开/关 | rev开(震/逃) | "
             "T5/T6/T7/ZD关 |")
    L.append("|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        for ckey, p in r["cells"].items():
            ls, c = p["leg_stats"], p["counters"]

            def f(leg):
                d = ls[leg]
                if d["n_pairs"] == 0:
                    return "—"
                return f"{d['n_pairs']}/{d['win_rate']}%/{d['net_cash']:+.0f}"

            L.append(
                f"| {s} | {ckey} | {f('main')} | {f('osc')} | {f('rev')} | "
                f"{c['n_master_rev_open']}/{c['n_master_rev_close']} | "
                f"{c['n_rev_open']}({c['n_rev_open_osc']}/{c['n_rev_open_esc']}) | "
                f"{c['n_rev_close_t5']}/{c['n_rev_close_t6']}/"
                f"{c['n_rev_close_t7']}/{c['n_rev_zd_close']} |")
    L.append("")
    L.append("## master 级别归因（entry_ladder 分布）\n")
    L.append("| 标的 | cell | 级别 | 笔数 | 持有bar |")
    L.append("|---|---|---|---|---|")
    for s, r in results.items():
        for ckey, p in r["cells"].items():
            for name, d in p["ladder_attribution"].items():
                if name.startswith("_"):
                    continue
                L.append(f"| {s} | {ckey} | {name} | {d['n_trades']} | "
                         f"{d['held_bars']} |")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        all_cells = [f"floor{f}/{v}" for f in FLOORS for v in VARIANTS]
        done = (sym in results
                and all(c in results[sym].get("cells", {}) for c in all_cells)
                and all(str(f) in results[sym].get("o0_guard", {}) for f in FLOORS))
        if done and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有全部 cell", flush=True)
            continue
        results[sym] = process_symbol(sym, prev=results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        _write_tables(results)
        print(f"  [{sym}] 已落盘", flush=True)
    if results:
        _write_tables(results)


if __name__ == "__main__":
    main()
