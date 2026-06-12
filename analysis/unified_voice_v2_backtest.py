"""统一递归 voice FSM 修正版（fusion_v v2：删 R4 + 字面翻空）八标的 L3 回测。

上游：
- 概念链审计 `analysis/unified_voice_from_concept_chain.md`（21步映射：
  R4 = 链外机制 + 第19步退化为 Gated）
- `rust/src/trading/unified_voice.rs`（修正实装：Φ 直读 freeze + R14
  字面翻空全层，M=N 同股数翻转断面，1x 虚拟逐仓）
- 对照基线 = 在册 fusion_v v1（`data_cache/unified_voice_<SYM>.json`，
  R4 开 + Gated 落点，P1 4/8）

══════════════ 预注册判据（先于数据声明）══════════════

P1（编排者验收）：fusion_v(v2) ≥ BH，逐标的符号，目标 8/8。
V1（对 v1 增量）：fusion_v(v2) ≥ fusion_v(v1 在册值)，逐标的符号——
   删 R4 的已知增量（nor4 5/8）+ 字面翻空的未知增量分解。
U1：fusion_v(v2) ≥ fusion_t（统一接线不劣于在册单轴基座）。
U7：bear α 与 bull α 分桶同时非负，逐标的标注。
失败语义（任务条款）：若 ES/QQQ 做空仍为负，精确报告——
   (a) 翻空时点：nest 领先 confirmed 的 bar 数（nest_lead_bars 直读）；
   (b) 空头腿 P&L 分解（逐 ladder short_net_cash + 逐 exit_reason 聚合）；
   (c) 结构税 vs 实装问题的归因证据（强平数/MoveDown 停回补驻留）。

可观测性（非零判别义务）：freeze_up/dn 驻留、flip_shorts、short_covers、
   moveup_covers、liquidations、T2W 武装/触发/否定、nest 触发、osc 路由。

guard-a：tape_fp 与 p7_conj 在册逐项一致（代次钉死）。
guard-b：hold26 / fusion_tr 逐位复现在册（0.05pp）。

诚实声明（090号）：杠杆三元组与真实空头载体（认沽等）不在本轮——
空头载体 = 1x 虚拟逐仓（fusion_btr 在册会计），认沽载体是部署层动作。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/unified_voice_v2_backtest.py [SYM ...]
输出：analysis/data_cache/unified_voice_v2_<SYM>.json + unified_voice_v2_summary.json
"""

from __future__ import annotations

import json
import sys
import time
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from universal_combination_backtest import analyze, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG

MODES = ["hold26", "fusion_tr", "fusion_t", "fusion_v",
         "fusion_v_self"]  # 消融臂：freeze 仅自层（递归传导摘除）
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]


def gates_v2(res: dict) -> dict:
    """fusion_v v2 观测面（非零判别义务）。"""
    return {
        "freeze_up_bars": res["freeze_up_bars_by_ladder"],
        "freeze_dn_bars": res["freeze_dn_bars_by_ladder"],
        "flip_shorts": res["n_flip_shorts_by_ladder"],
        "short_covers": res["n_short_covers_by_ladder"],
        "short_moveup_covers": res["n_short_moveup_covers_by_ladder"],
        "short_liquidations": res["n_short_liquidations_by_ladder"],
        "short_held_bars": res["short_held_bars_by_ladder"],
        "short_net_cash": [round(x, 0) for x in res["short_net_cash_by_ladder"]],
        "movedown_holds": res["n_short_trend_holds_by_ladder"],
        "restore_r2_blocks": res["n_short_r2_blocks_by_ladder"],
        "trim_r2_blocks": res["n_r2_pos_blocks_by_ladder"],
        "t2w_arms": res["n_t2w_arms_by_ladder"],
        "t2w_fires": res["n_t2w_fires_by_ladder"],
        "t2w_negates": res["n_t2w_negates_by_ladder"],
        "trend_holds": res["n_trend_holds_by_ladder"],
        "anc_exempt_blocks": res["n_anc_exempt_blocks_by_ladder"],
        "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
        "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
        "nest_breaks": res["n_nest_breaks_by_ladder"],
        "nest_lead_bars_sum": res["nest_lead_bars_sum"],
        "nest_lead_n": res["nest_lead_n"],
        "osc_opens": res["n_osc_opens_by_ladder"],
        "osc_escalates": res["n_osc_escalates_by_ladder"],
        "osc_net_cash": [round(x, 0) for x in res["osc_net_cash_by_ladder"]],
    }


def short_pnl_decomposition(res: dict) -> dict:
    """空头腿 P&L 分解（失败语义条款 (b)：逐 exit_reason × 逐 ladder）。

    trade 行 = (ladder, entry_bar, entry_price, exit_bar, exit_price,
                shares, weight, deferred_bars, partial, exit_reason, polarity)。
    空头腿 P&L = units × (entry_price − exit_price)。
    """
    by_reason: dict = defaultdict(lambda: {"n": 0, "pnl": 0.0, "win": 0})
    by_ladder: dict = defaultdict(lambda: {"n": 0, "pnl": 0.0, "win": 0})
    hold_bars = []
    for t in res["trades"]:
        (lad, ebar, epx, xbar, xpx, sh, _w, _d, _p, reason, pol) = t
        if pol != "short":
            continue
        pnl = sh * (epx - xpx)
        for key, agg in ((reason, by_reason), (lad, by_ladder)):
            agg[key]["n"] += 1
            agg[key]["pnl"] += pnl
            agg[key]["win"] += pnl > 0
        hold_bars.append(xbar - ebar)
    out = {
        "by_reason": {k: {"n": v["n"], "pnl": round(v["pnl"], 0),
                          "win_rate": round(v["win"] / v["n"], 3)}
                      for k, v in sorted(by_reason.items())},
        "by_ladder": {str(k): {"n": v["n"], "pnl": round(v["pnl"], 0),
                               "win_rate": round(v["win"] / v["n"], 3)}
                      for k, v in sorted(by_ladder.items())},
        "n_short_legs": len(hold_bars),
        "median_hold_bars": (sorted(hold_bars)[len(hold_bars) // 2]
                             if hold_bars else None),
    }
    return out


def verdicts(sym: str, bh: float, modes: dict, fv1: float | None) -> dict:
    """预注册判据逐标的评估。"""
    fv = modes["fusion_v"]["strat_pct"]
    ft = modes["fusion_t"]["strat_pct"]
    v = {
        "P1_ge_bh": fv >= bh,
        "U1_ge_fusion_t": fv >= ft,
        "fv2": fv, "ft": ft, "bh": round(bh, 1),
    }
    if fv1 is not None:
        v["V1_ge_fv1"] = fv >= fv1
        v["fv1"] = fv1
    reg = modes["fusion_v"].get("regime") or {}
    bear = reg.get("bear", {}).get("alpha") if reg.get("bear", {}).get("years") else None
    bull = reg.get("bull", {}).get("alpha") if reg.get("bull", {}).get("years") else None
    if bear is not None or bull is not None:
        v["U7_bear_bull_nonneg"] = all(
            x is None or x >= 0 for x in (bear, bull))
        v["U7_bear_alpha"] = bear
        v["U7_bull_alpha"] = bull
    return v


def fv1_in_book(sym: str) -> float | None:
    """v1 在册值（R4 开 + Gated 落点；对照基线，不重跑）。"""
    f = DATA_DIR / f"unified_voice_{sym}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    a = d.get("modes", {}).get("fusion_v")
    return a["strat_pct"] if isinstance(a, dict) and "strat_pct" in a else None


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%",
          flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    ref = json.loads((DATA_DIR / f"p7_conj_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "design": "fusion_v v2（删R4+字面翻空）八标的（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        if mode == "fusion_v":
            a["gates_v2"] = gates_v2(res)
            a["short_pnl"] = short_pnl_decomposition(res)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}%", flush=True)

    guards = {}
    for m in ("hold26", "fusion_tr"):
        book = ref["modes"][m]["strat_pct"]
        now = out["modes"][m]["strat_pct"]
        guards[f"guard_b_{m}"] = [now, book, abs(now - book) < 0.05]
    out["guards"] = guards
    if not all(v[2] for v in guards.values()):
        out["failed"] = "guard_reproduction"
        return out

    out["verdicts"] = verdicts(sym, bh_pct, out["modes"], fv1_in_book(sym))
    s = {m: out["modes"][m]["strat_pct"] for m in MODES}
    out["ablation"] = {"d_anc": round(s["fusion_v"] - s["fusion_v_self"], 1)}
    print(f"[{sym}] verdicts={json.dumps(out['verdicts'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"unified_voice_v2_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "verdicts": out["verdicts"]}
            for mode in MODES:
                row[mode] = out["modes"][mode]["strat_pct"]
                row[f"{mode}_mdd"] = out["modes"][mode]["mdd_pct"]
            summary.append(row)
        (DATA_DIR / "unified_voice_v2_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    ok = [r for r in summary if "failed" not in r]
    for key in ("P1_ge_bh", "V1_ge_fv1", "U1_ge_fusion_t"):
        hits = {r["symbol"]: r["verdicts"].get(key) for r in ok}
        npos = sum(bool(x) for x in hits.values())
        print(f"{key}: {npos}/{len(ok)} {hits}", flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
