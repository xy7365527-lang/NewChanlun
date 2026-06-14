"""为什么不挣钱——逐笔 P&L 分解诊断（调研，非 patch；只读分析）。

任务（编排者 2026-06-14）：信号层正确(buysellpoint.rs)+会计层正确(守恒零违反)，
但 P1 最好 5/8。开任务系统性定位 alpha 瓶颈。本脚本对 OKLO/QQQ/CL 三标的重跑
URS 引擎(mode="urs"，当前最优 5/8)，dump res["trades"] 逐笔磁带，输出五维度数据：

  1. 逐笔 P&L 分解（进场级别/方向/持仓/盈亏比/胜负结构差异）
  2. 信号延迟（deferred_bars 在册 + 进场价 vs 反转极值 gap）
  3. 方向正确率（进场后 N=10/50/100 bar 价格方向 vs 交易方向）
  4. BH 漂移（年化）
  5. 按 source_level(ladder) 的盈利贡献

逐笔元组 = (ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
            weight_at_entry, deferred_bars, partial, exit_reason, polarity)
进场 BSP type 经 tape[entry_bar-deferred].bsp_events[ladder] 关联重建。

用法: PYTHONPATH=src .venv/bin/python analysis/_why_not_profitable_diag.py [SYM ...]
输出: analysis/data_cache/_why_not_profitable_<SYM>.json
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))
import newchan_rust as nr  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, LADDER_NAMES  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "CL"]
FWD = (10, 50, 100)
EXTREME_W = 50  # 反转极值回看窗口（bar）


def entry_kind(tape, eb: int, deferred: int, ladder: int, pol: str) -> str:
    """confirm_bar = eb - deferred；在该 bar 的 bsp_events[ladder] 找触发侧 BSP kind。"""
    cb = max(0, eb - max(0, deferred))
    want = "buy" if pol == "long" else "sell"
    for b in (cb, eb):  # 容错：confirm bar 与成交 bar 都看
        if b >= len(tape):
            continue
        evs = tape[b].bsp_events
        if ladder < len(evs):
            for ev in evs[ladder]:
                kind, side = ev[0], ev[1]
                if side == want:
                    return kind
    return "unknown"


def analyze_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    yrs = (years[-1] - years[0] + 1) if years else max(1, n / (252 * 390))
    bh_ann = ((1 + bh_pct / 100) ** (1 / yrs) - 1) * 100 if bh_pct > -100 else float("nan")
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% span≈{yrs:.1f}y ann={bh_ann:+.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(
        opens, highs, lows, closes, dir_flips=dir_flips, require_settled=True
    )
    print(f"[{sym}] signal layer {time.time()-t0:.1f}s flips={len(dir_flips)}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="urs")
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100
    trades = res["trades"]
    print(f"[{sym}] urs {time.time()-t1:.1f}s strat={strat_pct:+.1f}% trades={len(trades)}", flush=True)

    rows = []
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        eb, xb = int(eb), int(xb)
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        held = xb - eb
        # 方向正确率
        dir_ok = {}
        for N in FWD:
            j = min(eb + N, n - 1)
            fwd_ret = closes[j] / closes[eb] - 1
            dir_ok[N] = (fwd_ret > 0) == (pol == "long")
        # 进场价 vs 反转极值 gap（long：离 W 窗最低点多远；short：离最高点）
        lo = eb - EXTREME_W
        if pol == "long":
            ext = min(lows[max(0, lo):eb + 1])
            gap = (ep - ext) / ext * 100 if ext > 0 else float("nan")
        else:
            ext = max(highs[max(0, lo):eb + 1])
            gap = (ext - ep) / ext * 100 if ext > 0 else float("nan")
        rows.append({
            "lad": int(lad), "level": LADDER_NAMES.get(int(lad), str(lad)),
            "pol": pol, "eb": eb, "xb": xb, "held": held,
            "deferred": int(dfr), "kind": entry_kind(tape, eb, int(dfr), int(lad), pol),
            "pnl": round(pnl, 1), "ret_pct": round((xp / ep - 1) * 100 * (1 if pol == "long" else -1), 3),
            "ext_gap_pct": round(gap, 2),
            "dir_ok": {str(N): dir_ok[N] for N in FWD},
            "reason": reason,
        })

    # ── 聚合 ──
    def agg(sub):
        if not sub:
            return {"n": 0}
        wins = [r for r in sub if r["pnl"] > 0]
        losses = [r for r in sub if r["pnl"] <= 0]
        aw = sum(r["pnl"] for r in wins) / len(wins) if wins else 0.0
        al = sum(r["pnl"] for r in losses) / len(losses) if losses else 0.0
        return {
            "n": len(sub),
            "pnl_cash": round(sum(r["pnl"] for r in sub), 0),
            "win_rate": round(len(wins) / len(sub), 3),
            "avg_win": round(aw, 1), "avg_loss": round(al, 1),
            "payoff": round(aw / -al, 2) if al < 0 else None,
            "avg_held": round(sum(r["held"] for r in sub) / len(sub), 0),
            "avg_deferred": round(sum(r["deferred"] for r in sub) / len(sub), 1),
            "avg_ext_gap": round(sum(r["ext_gap_pct"] for r in sub if r["ext_gap_pct"] == r["ext_gap_pct"]) / max(1, len(sub)), 2),
        }

    by_level = {}
    for lv in set(r["level"] for r in rows):
        for pol in ("long", "short"):
            sub = [r for r in rows if r["level"] == lv and r["pol"] == pol]
            if sub:
                by_level[f"{lv}/{pol}"] = agg(sub)
    by_kind = {k: agg([r for r in rows if r["kind"] == k]) for k in set(r["kind"] for r in rows)}
    # 方向正确率（全体）
    dir_acc = {str(N): round(sum(r["dir_ok"][str(N)] for r in rows) / max(1, len(rows)), 3) for N in FWD}
    dir_acc_long = {str(N): round(sum(r["dir_ok"][str(N)] for r in rows if r["pol"] == "long") / max(1, sum(1 for r in rows if r["pol"] == "long")), 3) for N in FWD}
    dir_acc_short = {str(N): round(sum(r["dir_ok"][str(N)] for r in rows if r["pol"] == "short") / max(1, sum(1 for r in rows if r["pol"] == "short")), 3) for N in FWD}
    # 胜负结构差异
    wins = [r for r in rows if r["pnl"] > 0]
    losses = [r for r in rows if r["pnl"] <= 0]
    def struct(sub):
        if not sub:
            return {}
        return {
            "n": len(sub),
            "avg_held": round(sum(r["held"] for r in sub) / len(sub), 0),
            "avg_deferred": round(sum(r["deferred"] for r in sub) / len(sub), 1),
            "avg_ext_gap": round(sum(r["ext_gap_pct"] for r in sub if r["ext_gap_pct"] == r["ext_gap_pct"]) / len(sub), 2),
            "frac_long": round(sum(1 for r in sub if r["pol"] == "long") / len(sub), 3),
            "level_dist": {lv: sum(1 for r in sub if r["level"] == lv) for lv in set(r["level"] for r in sub)},
        }
    # 单笔最大盈利占比（mega-long 检验）
    total_pnl = sum(r["pnl"] for r in rows)
    top = sorted(rows, key=lambda r: r["pnl"], reverse=True)[:3]

    out = {
        "symbol": sym, "n_bars": n, "bh_pct": round(bh_pct, 1),
        "bh_ann_pct": round(bh_ann, 1), "span_years": round(yrs, 1),
        "strat_pct": round(strat_pct, 1), "n_trades": len(trades),
        "total_pnl_cash": round(total_pnl, 0),
        "overall": agg(rows),
        "dir_accuracy_all": dir_acc, "dir_accuracy_long": dir_acc_long, "dir_accuracy_short": dir_acc_short,
        "by_level": by_level, "by_kind": by_kind,
        "winners_struct": struct(wins), "losers_struct": struct(losses),
        "top3_pnl": [{"level": r["level"], "pol": r["pol"], "pnl": r["pnl"], "held": r["held"],
                      "pct_of_total": round(r["pnl"] / total_pnl * 100, 1) if total_pnl else None,
                      "kind": r["kind"]} for r in top],
    }
    print(f"[{sym}] win_rate={out['overall']['win_rate']} payoff={out['overall'].get('payoff')} "
          f"dir@100={dir_acc['100']} top1%={out['top3_pnl'][0]['pct_of_total']}", flush=True)
    return out


def main() -> None:
    for sym in SYMBOLS:
        try:
            out = analyze_symbol(sym)
        except Exception as e:  # noqa: BLE001
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
        (DATA_DIR / f"_why_not_profitable_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
    print("DONE", flush=True)


if __name__ == "__main__":
    main()
