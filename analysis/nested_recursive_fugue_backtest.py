"""嵌套递归并发多重赋格（NRF）回测 — BTC + OKLO 双标的判决。

设计：analysis/nested_recursive_fugue_design.md（§6 最小可验证实现 + §7 预注册判据）。
positional 否证矫正：master 满仓入场保暴露（入场侧不拆），级别门只在出场侧。

三条正交轴（全部长在 V2oa25_emht 在册最优出场基座上）：
  E 轴 NRF1  = entry 级完整声部（44课级别门嵌套形态：段级 sell1 在 master
               被趋势延续证据拦截时由 voice@entry 承载为配额短差）
  S 轴 NRF2  = NRF1 + 反向线段子腿严格形式（CounterSeg depth=2：本级别
               买卖点开闭，符号交替塔，35课成本门递归终止）
  Q 轴 NRF2q = NRF2 + 配额涌现候选 B（40课中枢振幅占比归一）

预注册判据（设计 §7，回测前锁定）：
  BTC ：NRF 臂总收益 > V2oa25_ht_scco(+630% 在册) 且 top5 踏空 gap nats
        较 scco 的 0.750 减少；否证 = 踏空不减或总收益 ≤ +630%。
  OKLO：NRF1/NRF2 ≥ V2oa25_ht 同磁带基线（子声部不拖累）；
        否证 = 低于基线（重蹈分型递归 OKLO −73.8pp）。
  深度涌现：n_sub_open > 0 且 +400 区（depth2）腿只在结构允许处出现；
        成本门拒开计数 > 0（35课终止活性）或诚实记录零激活。

用法：BT_SYMBOLS=OKLO,BTC .venv/bin/python analysis/nested_recursive_fugue_backtest.py
输出：analysis/data_cache/nested_recursive_fugue_backtest.json
"""

import json
import math
import os
import sys
import time
from collections import defaultdict
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
OUT_JSON = DATA_DIR / "nested_recursive_fugue_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BTC").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))

# 每标的变体矩阵：osc 白名单在册（BTC 负域须 sc+co；OKLO 正域不加）
VARIANTS_BY_SYMBOL = {
    "OKLO": ["V2oa25_ht", "V2oa25_emht", "V2oa25_emht_q",
             "NRF1", "NRF2", "NRF2q"],
    "BTC": ["V2oa25_ht_scco", "V2oa25_emht_scco", "NRF1_scco",
            "NRF2_scco", "NRF2q_scco"],
}

SUB_KEYS = ("n_sub_open", "n_sub_close", "n_sub_forced_close",
            "sub_pairs", "sub_wins",
            "n_sub_cost_rejects", "n_sub_cost_noref_rejects",
            "n_sub_earning_rejects")
REV_KEYS = ("n_rev_open", "n_rev_close_t5", "n_rev_close_t6",
            "n_rev_close_t7", "n_rev_zd_close", "n_exit_trend_holds",
            "n_exit_emergent_bars")


def gap_analysis(trades, closes) -> dict:
    """空仓期踏空分解（btc_vs_bh_structural_analysis 同口径）：
    gap = 上一笔 exit_bar → 下一笔 entry_bar 的 log 收益（>0 = 踏空）。"""
    gaps = []
    for prev, nxt in zip(trades, trades[1:]):
        a, b = prev.exit_bar, nxt.entry_bar
        if b > a and closes[a] > 0:
            gaps.append(math.log(closes[b] / closes[a]))
    pos = sorted((g for g in gaps if g > 0), reverse=True)
    return {
        "n_gaps": len(gaps),
        "missed_pos_nats": round(sum(pos), 4),
        "avoided_neg_nats": round(sum(g for g in gaps if g < 0), 4),
        "net_nats": round(sum(gaps), 4),
        "top5_nats": round(sum(pos[:5]), 4),
        "top5": [round(g, 4) for g in pos[:5]],
    }


def leg_stats(diag) -> dict:
    """按腿类型聚合（含 rev_sub；在册 _leg_stats_rust 无此键故自带）。
    rev_sub 再按 py_key 区分解深度（+300=depth1 / +400=depth2）。"""
    by_leg = defaultdict(list)
    depth_cash = defaultdict(float)
    depth_n = defaultdict(int)
    for _header, diffs in diag:
        for (key, leg, _sb, _sp, _bb, _bp), (_sh, df, pf, *_r) in diffs:
            by_leg[leg].append((df, pf))
            if leg == "rev_sub":
                depth = (key - 200) // 100  # 300区→1 400区→2
                depth_cash[depth] += pf
                depth_n[depth] += 1
    out = {}
    for leg, rows in sorted(by_leg.items()):
        n = len(rows)
        out[leg] = {
            "n_pairs": n,
            "win_rate": round(sum(1 for df, _ in rows if df > 0) / n * 100, 1),
            "net_cash": round(sum(pf for _, pf in rows), 1),
        }
    if depth_n:
        out["rev_sub_by_depth"] = {
            str(d): {"n": depth_n[d], "net_cash": round(depth_cash[d], 1)}
            for d in sorted(depth_n)}
    return out


def exit_histogram(trades) -> dict:
    h = defaultdict(int)
    for t in trades:
        h[t.exit_reason] += 1
    return dict(sorted(h.items(), key=lambda kv: -kv[1])[:8])


def run_cell(rtape, variant: str, years: float, bh: float, closes) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    c = res["counters"]
    cell = {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "gap": gap_analysis(trades, closes),
        "exit_reasons": exit_histogram(trades),
        "leg_stats": leg_stats(res["diag"]),
        "counters": {k: c[k] for k in SUB_KEYS + REV_KEYS if k in c},
        "sub_cash": round(c["sub_cash"], 1) if "sub_cash" in c else None,
        "elapsed_s": round(el, 1),
    }
    print(f"    {variant:<18} 复利 {m['total_compound']:+10.1f}%  "
          f"笔数 {m['n']:>5}  mdd {m['max_dd']:+.1f}%  "
          f"top5踏空 {cell['gap']['top5_nats']:.3f}nats  "
          f"sub开/闭/强闭 {c['n_sub_open']}/{c['n_sub_close']}/"
          f"{c['n_sub_forced_close']}  [{el:.0f}s]", flush=True)
    return cell


def process_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 70}\n  {symbol} — 嵌套递归并发赋格（floor={FLOOR}）"
          f"\n{'=' * 70}", flush=True)
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
    print(f"  信号层 {time.time() - t0:.1f}s", flush=True)
    fp = {"bsp": sum(len(s.bsp_events[lv]) for s in tape if s.bsp_events
                     for lv in range(11)),
          "div": sum(len(s.div_events[lv]) for s in tape if s.div_events
                     for lv in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"  tape_fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    cells = {}
    for v in VARIANTS_BY_SYMBOL[symbol]:
        cells[v] = run_cell(rtape, v, years, bh, closes)
    return {"bars": n, "bh_pct": round(bh, 2), "tape_fp": fp,
            "variants": cells}


def main() -> None:
    out = {}
    if OUT_JSON.exists():
        out = json.loads(OUT_JSON.read_text())
    for symbol in SYMBOLS:
        out[symbol] = process_symbol(symbol)
        DATA_DIR.mkdir(exist_ok=True)
        OUT_JSON.write_text(json.dumps(out, indent=1, ensure_ascii=False))
        print(f"  → 增量落盘 {OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
