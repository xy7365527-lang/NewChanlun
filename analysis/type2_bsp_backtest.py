"""type2 买卖点交易路径回测 — V2r vs V2r+T2o/T2c/T2（2026-06-11 任务）。

═══════════════════════════════════════════════════════════════════════
背景与对比矩阵
═══════════════════════════════════════════════════════════════════════
C段修复后 type2 BSP 从 0 解锁到数百个（OKLO L2 type2≈360），但交易层
配对模式（V2r）此前显式排除 type2：
  - 开腿侧：rev_open_signal 只接 Sell1/盘背/Sell3（Sell2 不是触发源）
  - 闭腿侧：Buy2 被配对谓词拒绝，计入 n_rev_mismatch_holds 反事实

原文操作语义（第17课/买卖点定律一）：type2 由次级别 type1 构成——
type1 说"趋势可能结束"，type2 说"确实结束"（回落不创新低/回升不创新高）。
type2 是确认点。

变体（基线 = V2r，两位独立掩码）：
  1. O0      ：≡P5 逐位守卫（确认 type2 轴零侵入 legacy 路径）
  2. V2r     ：在册基线（震荡型 + θ=1% + kind 配对闭腿）
  3. V2rT2o  ：+ sell2_open（confirmed Sell2 入开腿触发集，trigger bit3）
  4. V2rT2c  ：+ buy2_close（confirmed Buy2 同锚配对闭腿，reason=10）
  5. V2rT2   ：+ 两者

预注册判据：
  (a) T2c：Buy2 闭腿数 > 0 且 rev 净现金不劣于 V2r（确认点提前回补
      应改善或持平兑现价位；若恶化 = Buy2 过早回补，确认语义被否证）
  (b) T2o：Sell2 触发开腿数 > 0；其腿的胜率 ≥ 震荡型整体（type2 更确认
      的预期）；若新增腿净负 = type2 开腿假设被否证
  (c) 任一变体 Δ(vs P5) 相对 V2r 的变化即该轴独立贡献

认识论等级：O0 守卫 L1（管线等价）；变体结论 L2（OKLO 单标的真实数据，
新语义无 oracle）。

输出：analysis/data_cache/type2_bsp_backtest.json（增量续跑）
      analysis/data_cache/type2_bsp_backtest_tables.md（自动表格）
用法：PYTHONPATH=src .venv/bin/python analysis/type2_bsp_backtest.py
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
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue_rust_backtest import (  # noqa: E402
    _bitexact_o0,
    _leg_stats_rust,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "type2_bsp_backtest.json"
OUT_MD = DATA_DIR / "type2_bsp_backtest_tables.md"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
VARIANTS = ["V2r", "V2rT2o", "V2rT2c", "V2rT2"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_attempts", "n_rev_nocenter_rejects", "n_rev_depth_rejects",
    "n_rev_open", "n_rev_open_osc", "n_rev_open_esc",
    "n_rev_sell2_open", "n_rev_buy2_close",
    "n_rev_close_t5", "n_rev_close_t6", "n_rev_close_t7",
    "n_rev_zd_close", "n_rev_mismatch_holds",
    "rev_osc_pairs", "rev_osc_wins", "rev_osc_net_cash",
)


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — type2 买卖点交易路径回测\n{'=' * 64}",
          flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # 磁带指纹守卫（tape_fp 先例：并发引擎修改致 bsp 混表事故）
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(
            f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}——引擎/信号层"
            f"语义已变，禁止混表。删除该标的 json 条目全量重跑。")

    # type2 事件密度（任务背景数字的复核：C段修复后 L2 type2 解锁量）
    n_sell2 = [0] * 11
    n_buy2 = [0] * 11
    for s in tape:
        if not s.bsp_events:
            continue
        for l in range(11):
            for e in s.bsp_events[l]:
                # 事件元组：(kind, side, seg_idx, confirmed, cs, zd, zg, price)
                if e[0] == "type2":
                    if e[1] == "sell":
                        n_sell2[l] += 1
                    else:
                        n_buy2[l] += 1
    t2_density = {f"L{l}": {"sell2": n_sell2[l], "buy2": n_buy2[l]}
                  for l in range(11) if n_sell2[l] or n_buy2[l]}
    print(f"  type2 事件密度：{t2_density}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips)

    if prev is not None and "P5" in prev and prev.get("o0_guard") == "PASS":
        out = prev
        p5c = out["P5"]["metrics"]["total_compound"]
        print(f"  [增量] 复用在册 P5({p5c:+.2f}%) / O0 守卫", flush=True)
    else:
        trades_p5, _ = run_version_i(
            tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"])
        m_p5 = extended_metrics(trades_p5, years)
        p5c = m_p5["total_compound"]
        print(f"  P5 复利 {p5c:+.2f}%", flush=True)

        o0 = _bitexact_o0(tape, rtape, years)
        print(f"  [O0≡P5] PASS  {o0['n_trades']} 笔 / {o0['n_trace_rows']} "
              f"trace 行", flush=True)

        out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
               "n_dir_flips": len(dir_flips), "tape_fp": fp,
               "t2_density": t2_density, "o0_guard": "PASS",
               "P5": {"metrics": {k: m_p5[k] for k in
                                  ("n", "win_rate", "total_compound",
                                   "sharpe", "max_dd")}},
               "O0": o0, "variants": {}}
    out["t2_density"] = t2_density

    todo = [v for v in VARIANTS if v not in out["variants"]
            or os.environ.get("BT_FORCE", "0") == "1"]
    for name in todo:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = _leg_stats_rust(res["diag"])
        c = res["counters"]
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        rev = legs["rev"]
        print(f"  [{name:7s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:4d}"
              f" sell2开={c['n_rev_sell2_open']:3d}"
              f" buy2关={c['n_rev_buy2_close']:3d}"
              f" rev胜率={rev['win_rate']}% 净现金={rev['net_cash']:+.0f}"
              f" 错配持仓={c['n_rev_mismatch_holds']} [{el:.3f}s]", flush=True)
    return out


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# type2 买卖点交易路径回测 — V2r vs +T2o/+T2c/+T2\n")
    L.append("> 自动表格（终版报告 analysis/type2_bsp_integration.md 手工定稿）。\n")
    L.append("## 主对照表\n")
    L.append("| 标的 | P5复利% | 变体 | 复利% | Δ(vs P5) pp | rev对数 | rev胜率% "
             "| rev净现金 | sell2开 | buy2关 | zd关 | T5关 | 错配持仓bar |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        p5c = r["P5"]["metrics"]["total_compound"]
        for v, p in r["variants"].items():
            rev = p["leg_stats"]["rev"]
            c = p["counters"]
            L.append(
                f"| {s} | {p5c:+.1f} | {v} | "
                f"{p['metrics']['total_compound']:+.1f} | "
                f"**{p['delta_vs_p5']:+.1f}** | {rev['n_pairs']} | "
                f"{rev['win_rate']} | {rev['net_cash']:+.0f} | "
                f"{c['n_rev_sell2_open']} | {c['n_rev_buy2_close']} | "
                f"{c['n_rev_zd_close']} | {c['n_rev_close_t5']} | "
                f"{c['n_rev_mismatch_holds']} |")
    L.append("")
    L.append("## type2 事件密度（磁带层）\n")
    for s, r in results.items():
        L.append(f"- {s}: {json.dumps(r.get('t2_density', {}))}")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists() and os.environ.get("BT_FORCE", "0") != "1":
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        results[sym] = process_symbol(sym, results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=1))
        _write_report(results)
    print(f"\n落盘：{OUT_JSON}\n      {OUT_MD}", flush=True)


if __name__ == "__main__":
    main()
