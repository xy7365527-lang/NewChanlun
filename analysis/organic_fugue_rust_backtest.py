"""有机赋格 v2 Rust 交易层三标的对比回测 — O0≡P5 bit-exact + REV 变体增量贡献。

═══════════════════════════════════════════════════════════════════════
对比矩阵（编排者 2026-06-10 追加要求）
═══════════════════════════════════════════════════════════════════════
  标的：OKLO 447K / QQQ 728K / BRN 2.4M（与 v1 验收同三标的同磁带语义）
  1. Rust O0 ≡ Python O0（≡P5 传递）四面对账（trades/trace/counters/归因）
  2. REV 变体 vs P5 增量（Δ = 复利差，§8.2 口径——vs_B0 差分中 B0 抵消）：
     V1  = rev 裸开（v1 O1v 的 v2 语义类比：唯一差异 C2 osc 不截断 +
           C5 单 tranche 区间套读出在 home 层退化为同形）→ 隔离 C2 因果
     V1f = V1 + G1 方向锚定（C3）→ 判据1（t6 占比降幅 / rev 对数收缩）
     V1r = V1f + tranche 递归建仓/平仓（C4/C5）→ 判据2（Δ(V1r) vs Δ(V1f)）
  在册 v1 基线（organic_fugue_backtest.json）：Δ(O1v) = −63.7/−41.4/−186.2pp，
  O1v rev t6 闭腿占比 25.3/28.3/27.4%，rev 对数 917/1913/5538。

D3 方向行：同一引擎 pass 收集稀疏翻转行（organic_signals dir_flips 收集器），
锚语义 = 信号观测时点（声明见 organic_signals docstring）。

认识论等级：O0 对账 L1（管线等价）；变体结论 L2→L3（三标的真实数据）。

输出：analysis/organic_fugue_rust_backtest.md
      analysis/data_cache/organic_fugue_rust_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/organic_fugue_rust_backtest.py
      （env：BT_SYMBOLS=OKLO,QQQ,BRN  BT_FORCE=1 强制重跑）
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

from fugue_alpha_diagnosis import CompletedTrade  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue import ORGANIC_VARIANTS, run_organic  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "organic_fugue_rust_backtest.json"
OUT_MD = ROOT / "analysis" / "organic_fugue_rust_backtest.md"
V1_JSON = DATA_DIR / "organic_fugue_backtest.json"   # v1 在册基线（O1v）

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
REV_VARIANTS = ["V1", "V1f", "V1r"]
FLOOR = LADDER_SEG


def _trades_from_rust(rows: list) -> list[CompletedTrade]:
    return [CompletedTrade(entry_bar=r[0], entry_price=r[1], exit_bar=r[2],
                           exit_price=r[3], pnl_pct=r[4], exit_reason=r[5],
                           n_short_diffs=r[6], cost_basis_at_exit=r[7])
            for r in rows]


def _leg_stats_rust(diag: list) -> dict:
    """Rust diag → 按腿类型聚合（口径与 organic_fugue_backtest._leg_stats 一致）。"""
    by_leg: dict[str, list] = {"main": [], "osc": [], "rev": []}
    for _header, diffs in diag:
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1) in diffs:
            by_leg[leg].append((sp, df, pf))
    out = {}
    for leg, rows in by_leg.items():
        n = len(rows)
        if n == 0:
            out[leg] = {"n_pairs": 0, "win_rate": None, "avg_diff_pct": None,
                        "net_cash": 0.0}
            continue
        out[leg] = {
            "n_pairs": n,
            "win_rate": round(sum(1 for sp, df, pf in rows if df > 0) / n * 100, 1),
            "avg_diff_pct": round(
                sum(df / sp for sp, df, pf in rows) / n * 100, 4),
            "net_cash": round(sum(pf for _sp, _df, pf in rows), 1),
        }
    return out


def _bitexact_o0(tape, rtape, years: float) -> dict:
    """Rust O0 ≡ Python O0 四面对账（不过 → SystemExit）。返回 O0 指标包。"""
    diag_py: list = []
    trades_py, extra_py = run_organic(
        tape, floor_ladder=FLOOR, config=ORGANIC_VARIANTS["O0"], diag=diag_py)
    res = nr.run_organic_rust(rtape, "O0", floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    if len(trades_py) != len(res["trades"]):
        raise SystemExit(f"FAIL O0 笔数 py={len(trades_py)} rust={len(res['trades'])}")
    for ti, (tp, tr) in enumerate(zip(trades_py, res["trades"])):
        if (tp.entry_bar, tp.entry_price, tp.exit_bar, tp.exit_price, tp.pnl_pct,
                tp.exit_reason, tp.n_short_diffs, tp.cost_basis_at_exit) != tuple(tr):
            raise SystemExit(f"FAIL O0 trade#{ti}")
    n_rows = 0
    for ti, (dp, (header, diffs)) in enumerate(zip(diag_py, res["diag"])):
        if dp["n_diffs"] != len(diffs):
            raise SystemExit(f"FAIL O0 trade#{ti} 短差数")
        for rp, ((key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1)) \
                in zip(dp["diffs"], diffs):
            rust_row = {"ladder": key, "leg": leg, "sell_bar": sb, "sell_price": sp,
                        "buy_bar": bb, "buy_price": bp, "shares": sh, "diff": df,
                        "profit": pf, "was_earning": we, "shares_delta": sd,
                        "cost_basis_before": cb0, "cost_basis_after": cb1}
            for k in rp:
                if rp[k] != rust_row[k]:
                    raise SystemExit(f"FAIL O0 trace 字段 {k}: {rp[k]!r}!={rust_row[k]!r}")
            n_rows += 1
    for k, v in extra_py["counters"].items():
        if v != res["counters"][k]:
            raise SystemExit(f"FAIL O0 counter {k}: {v}!={res['counters'][k]}")
    m = extended_metrics(trades_py, years)
    return {"n_trades": len(trades_py), "n_trace_rows": n_rows,
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")}}


def process_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — Rust 交易层对比回测\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    # ── 信号层（单 pass：磁带 + D3 方向行收集）──
    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # ── 磁带回归锚：D3 收集开启不得改变磁带本体（P5 复利逐位）──
    trades_p5, _ = run_version_i(
        tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"])
    m_p5 = extended_metrics(trades_p5, years)
    p5c = m_p5["total_compound"]
    if V1_JSON.exists():
        ref = json.loads(V1_JSON.read_text()).get(symbol)
        if ref:
            ref_p5 = ref["P5"]["metrics"]["total_compound"]
            if abs(ref_p5 - p5c) > 1e-9:
                raise SystemExit(
                    f"磁带回归锚失败：P5 {p5c} vs 在册 {ref_p5}"
                    "（D3 收集器污染了磁带本体）")
            print("  [磁带回归锚] PASS（P5 与 v1 在册逐位一致——D3 收集零侵入）",
                  flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips)

    # ── O0≡P5 四面对账 ──
    o0 = _bitexact_o0(tape, rtape, years)
    print(f"  [O0≡P5] PASS  {o0['n_trades']} 笔 / {o0['n_trace_rows']} trace 行",
          flush=True)

    out: dict = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
                 "n_dir_flips": len(dir_flips), "o0_guard": "PASS",
                 "P5": {"metrics": {k: m_p5[k] for k in
                                    ("n", "win_rate", "total_compound",
                                     "sharpe", "max_dd")}},
                 "O0": o0, "variants": {}}

    # ── REV 变体矩阵 ──
    for name in REV_VARIANTS:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = _leg_stats_rust(res["diag"])
        c = res["counters"]
        n_close = (c["n_rev_close_t5"] + c["n_rev_close_t6"]
                   + c["n_rev_close_t7"] + c["n_rev_struct_close"])
        t6_share = round(c["n_rev_close_t6"] / n_close * 100, 1) if n_close else None
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "counters": {k: c[k] for k in
                         ("n_rev_attempts", "n_rev_sub_anchor_rejects",
                          "n_rev_frozen_rejects", "n_rev_open",
                          "n_rev_close_t5", "n_rev_close_t6", "n_rev_close_t7",
                          "n_rev_struct_close", "n_rev_tranche_adds",
                          "n_rev_tranche_closes", "n_open_rejects_zero")},
            "t6_close_share_pct": t6_share,
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        print(f"  [{name:3s}] 交易={m['n']:4d} 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:5d} G1拒={c['n_rev_sub_anchor_rejects']:5d}"
              f" tranche加={c['n_rev_tranche_adds']:4d}"
              f" t6占比={t6_share}% rev净现金={legs['rev']['net_cash']:+.0f}"
              f" [{el:.3f}s]", flush=True)
    return out


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 有机赋格 v2 Rust 交易层对比回测 — O0≡P5 + REV 变体增量\n")
    L.append("> Rust 交易层（`rust/src/trading/`）vs Python oracle。O0 四面对账"
             "（trades 全字段 / trace 12 字段 / counters 18 键 / 归因）；REV 变体"
             "为 v2 新语义（C2-C5 修正后），无 Python oracle——L2 真实数据首跑。\n")
    L.append("> 在册 v1 基线：Δ(O1v) = OKLO −63.7 / QQQ −41.4 / BRN −186.2 pp；"
             "O1v rev t6 闭腿占比 25.3/28.3/27.4%；rev 对数 917/1913/5538。\n")
    L.append("## 守卫与主对照\n")
    L.append("| 标的 | bars | O0≡P5 | P5复利% | 变体 | 复利% | **Δ(vs P5) pp** | "
             "rev对数 | rev净现金 | rev胜率% | t6占比% | G1拒 | tranche加 |")
    L.append("|------|------|-------|---------|------|-------|------|"
             "--------|----------|---------|--------|------|-----------|")
    for s, r in results.items():
        p5c = r["P5"]["metrics"]["total_compound"]
        L.append(f"| {s} | {r['n_bars']:,} | {r['o0_guard']} | {p5c:+.1f} | — | — |"
                 f" — | — | — | — | — | — | — |")
        for v, p in r["variants"].items():
            rev = p["leg_stats"]["rev"]
            c = p["counters"]
            L.append(
                f"| {s} | | | | {v} | {p['metrics']['total_compound']:+.1f} | "
                f"**{p['delta_vs_p5']:+.1f}** | {rev['n_pairs']} | "
                f"{rev['net_cash']:+.0f} | {rev['win_rate']} | "
                f"{p['t6_close_share_pct']} | {c['n_rev_sub_anchor_rejects']} | "
                f"{c['n_rev_tranche_adds']} |")
    L.append("")
    L.append("## REV 机制计数\n")
    L.append("| 标的 | 变体 | T1尝试 | G1拒 | 冻结拒 | rev开 | T5关 | T6预逃 | "
             "T7逃+冻 | T5b结构关 | tranche加/关 |")
    L.append("|------|------|--------|------|--------|-------|------|--------|"
             "---------|-----------|--------------|")
    for s, r in results.items():
        for v, p in r["variants"].items():
            c = p["counters"]
            L.append(
                f"| {s} | {v} | {c['n_rev_attempts']} | "
                f"{c['n_rev_sub_anchor_rejects']} | {c['n_rev_frozen_rejects']} | "
                f"{c['n_rev_open']} | {c['n_rev_close_t5']} | {c['n_rev_close_t6']} | "
                f"{c['n_rev_close_t7']} | {c['n_rev_struct_close']} | "
                f"{c['n_rev_tranche_adds']}/{c['n_rev_tranche_closes']} |")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        if sym in results and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有结果", flush=True)
            continue
        results[sym] = process_symbol(sym)
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        _write_report(results)
        print(f"  [{sym}] 已落盘", flush=True)
    if results:
        _write_report(results)


if __name__ == "__main__":
    main()
