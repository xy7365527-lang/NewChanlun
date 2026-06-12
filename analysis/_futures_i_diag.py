"""期货 Version I（slice / I_none）交易行为诊断。

每笔交易磁带 + 亏损原因分类 + 降成本逐笔分解。

分解（关键）：全仓核心 = INITIAL_CAPITAL/entry_price 股，清仓市值 = core×exit。
  directional_pct = (exit_price/entry_price - 1) * 100   # 方向核心贡献
  slice_pct       = pnl_pct - directional_pct            # 降成本机动仓净贡献
pnl_pct 已含 core + 各声部 slice_gain（见 run_version_i._close）。
故 slice_pct 是该笔「多重赋格降成本」对总收益的净增减——逐笔可证伪降成本提款机假设。

认识论 L2（真实数据 ES/CL/GC 5.5M bar 全量；含否定性结果）。
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_version_i_slice as VI  # noqa: E402
import m1_e_futures_backtest as EM  # noqa: E402
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = ["ES", "CL", "GC"]


def classify(t) -> dict:
    """单笔分类 + 降成本分解。"""
    dirn = (t.exit_price / t.entry_price - 1.0) * 100.0 if t.entry_price > 0 else 0.0
    slc = t.pnl_pct - dirn  # 降成本机动仓净贡献
    win = t.pnl_pct > 0
    held = t.exit_bar - t.entry_bar
    if win:
        cat = "win"
    elif dirn > 0 >= t.pnl_pct:
        cat = "loss_slice_dragged"   # 核心方向盈利，被降成本短差拖入亏损（churn 伤害签名）
    elif dirn <= 0 and slc < 0:
        cat = "loss_both_neg"        # 方向亏 + 降成本也亏（双重失败）
    elif dirn <= 0 <= slc:
        cat = "loss_dir_slice_cushion"  # 方向亏，降成本部分缓冲但不足
    else:
        cat = "loss_other"
    return {
        "entry_bar": t.entry_bar, "exit_bar": t.exit_bar, "held": held,
        "entry_price": round(t.entry_price, 4), "exit_price": round(t.exit_price, 4),
        "pnl_pct": round(t.pnl_pct, 4), "dir_pct": round(dirn, 4),
        "slice_pct": round(slc, 4), "n_short_diffs": t.n_short_diffs,
        "cost_basis_at_exit": round(t.cost_basis_at_exit, 4),
        "exit_reason": t.exit_reason, "win": win, "cat": cat,
    }


def summarize(sym, rows, extra, bh, n_bars):
    n = len(rows)
    wins = [r for r in rows if r["win"]]
    losses = [r for r in rows if not r["win"]]
    cats: dict[str, int] = {}
    for r in rows:
        cats[r["cat"]] = cats.get(r["cat"], 0) + 1
    # 降成本逐笔统计
    active = [r for r in rows if r["n_short_diffs"] > 0]
    slc_help = [r for r in rows if r["slice_pct"] > 1e-9]
    slc_hurt = [r for r in rows if r["slice_pct"] < -1e-9]
    sum_dir = sum(r["dir_pct"] for r in rows)
    sum_slc = sum(r["slice_pct"] for r in rows)
    sum_pnl = sum(r["pnl_pct"] for r in rows)
    worst = sorted(rows, key=lambda r: r["pnl_pct"])[:12]
    best = sorted(rows, key=lambda r: r["pnl_pct"], reverse=True)[:12]
    # 被降成本拖入亏损的笔：核心方向本是盈利
    dragged = [r for r in rows if r["cat"] == "loss_slice_dragged"]
    dragged_dir = sum(r["dir_pct"] for r in dragged)
    dragged_slc = sum(r["slice_pct"] for r in dragged)
    return {
        "symbol": sym, "n_bars": n_bars, "bh": round(bh, 2), "n_trades": n,
        "n_wins": len(wins), "n_losses": len(losses),
        "win_rate": round(len(wins) / n * 100, 2) if n else 0.0,
        "categories": cats,
        "sum_pnl_pct_simple": round(sum_pnl, 2),
        "sum_dir_pct_simple": round(sum_dir, 2),
        "sum_slice_pct_simple": round(sum_slc, 2),
        "n_cr_active": len(active),
        "n_slice_help": len(slc_help), "n_slice_hurt": len(slc_hurt),
        "slice_help_sum": round(sum(r["slice_pct"] for r in slc_help), 2),
        "slice_hurt_sum": round(sum(r["slice_pct"] for r in slc_hurt), 2),
        "dragged_n": len(dragged), "dragged_dir_sum": round(dragged_dir, 2),
        "dragged_slice_sum": round(dragged_slc, 2),
        "mean_pnl": round(sum_pnl / n, 4) if n else 0.0,
        "mean_dir": round(sum_dir / n, 4) if n else 0.0,
        "mean_slice": round(sum_slc / n, 4) if n else 0.0,
        "ladder_attribution": extra.get("ladder_attribution", {}),
        "ladder_avg_held_bars": extra.get("ladder_avg_held_bars", {}),
        "fsm_contribution": extra.get("fsm_contribution", {}),
        "worst12": worst, "best12": best,
    }


def main():
    summaries = {}
    for sym in SYMBOLS:
        t0 = time.time()
        path = EM.SYMBOL_FILES[sym]
        opens, highs, lows, closes = EM.load_ohlc(path)
        n_bars = len(closes)
        bh = (closes[-1] - closes[0]) / closes[0] * 100
        print(f"[{sym}] {n_bars:,} bars, BH={bh:+.2f}%, 信号计算中…", flush=True)
        sigs = compute_i_signals_rust(opens, highs, lows, closes)
        trades, extra = VI.run_version_i(sigs, floor_ladder=VI.MIN_FLOOR_LADDER,
                                         stop_mode="none")
        rows = [classify(t) for t in trades]
        # 全量磁带落盘
        tape_path = DATA_DIR / f"_i_diag_tape_{sym}.json"
        tape_path.write_text(json.dumps(rows, ensure_ascii=False))
        summ = summarize(sym, rows, extra, bh, n_bars)
        summaries[sym] = summ
        del sigs
        print(f"[{sym}] {len(rows)} 笔, 胜率={summ['win_rate']}%, "
              f"Σdir={summ['sum_dir_pct_simple']}, Σslice={summ['sum_slice_pct_simple']}, "
              f"拖累笔={summ['dragged_n']} ({time.time()-t0:.0f}s)", flush=True)
    out = DATA_DIR / "_i_diag_summary.json"
    out.write_text(json.dumps(summaries, ensure_ascii=False, indent=1))
    print(f"\n✓ 汇总写入 {out}")


if __name__ == "__main__":
    main()
