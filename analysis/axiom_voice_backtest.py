"""公理演绎统一 voice FSM（fusion_va）八标的证伪检验——一次，零消融。

方法论（编排者定型）：FSM 形态从五条公理的否定运动推导（推导记录
`analysis/unified_voice_axiom_derivation.md`），回测角色 = 证伪检验，
非发现检验。本脚本不含消融臂——如果证伪失败，结论是"推导有误或实装
有误"，修复路径回到推导层，不通过调参数。

══════════════ 预注册判据（先于数据声明）══════════════

P1（编排者验收，回测层）：fusion_va ≥ BH 逐标的，目标 8/8。
U7（绩效公理可证伪形式）：bear α ∧ bull α 分桶同时非负。
可观测性：Φ 相位驻留 / 豁免域驻留 / 配对闭环开-接回-升级三岔 /
   几何接回占比（zd_restores vs 事件接回——特别裁决的存活形态检验）/
   成本门拒 / 位置门双侧拒——全部非零判别。
guard-a：tape_fp 与 p7_conj 在册逐项一致（代次钉死 M15-d）。
guard-b：hold26 / fusion_tr 逐位复现在册（0.05pp）。

证伪失败的解读协议：逐标的报告失血所在的 FSM 转移（exit_reasons +
计数器归因），定位到推导步骤——是公理运动展开错误（概念层）还是
实装映射错误（引擎事件流与概念的对齐缺口）。

用法：PYTHONPATH=src .venv/bin/python analysis/axiom_voice_backtest.py [SYM ...]
输出：analysis/data_cache/axiom_voice_<SYM>.json + axiom_voice_summary.json
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
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from universal_combination_backtest import analyze, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG

MODES = ["hold26", "fusion_tr", "fusion_va"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]


def gates_va(res: dict) -> dict:
    """fusion_va 观测面（非零判别义务）。"""
    return {
        "phi_up_bars": res["freeze_up_bars_by_ladder"],
        "phi_dn_bars": res["freeze_dn_bars_by_ladder"],
        "exempt_up_bars": res["anc_up_bars_by_ladder"],
        "paired_opens": res["n_sub_opens_by_ladder"],
        "zd_restores": res["n_osc_zd_restores_by_ladder"],
        "shift_restores": res["n_osc_shift_restores_by_ladder"],
        "event_restores": res["n_sub_restores_by_ladder"],
        "phase_restores": res["n_sub_phase_closes_by_ladder"],
        "hinge_escalates": res["n_sub_escalates_by_ladder"],
        "restore_defer_bars": res["n_sub_restore_defer_bars"],
        "sub_net_cash": [round(x, 0) for x in res["sub_net_cash_by_ladder"]],
        "sell_freezes": res["n_trend_holds_by_ladder"],
        "exempt_blocks": res["n_anc_exempt_blocks_by_ladder"],
        "buy_freezes": res["n_short_trend_holds_by_ladder"],
        "sell_pos_blocks": res["n_r2_pos_blocks_by_ladder"],
        "buy_pos_blocks": res["n_short_r2_blocks_by_ladder"],
        "cost_rejects": res["n_sub_cost_rejects_by_ladder"],
        "noref_rejects": res["n_sub_noref_rejects_by_ladder"],
        "coin_enters": res["n_gate_enters_by_ladder"],
        "coin_restores": res["n_gate_restores_by_ladder"],
        "coin_held_bars": res["gate_held_bars_by_ladder"],
        "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
        "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
        "nest_breaks": res["n_nest_breaks_by_ladder"],
    }


def verdicts(sym: str, bh: float, modes: dict) -> dict:
    fv = modes["fusion_va"]["strat_pct"]
    v = {"P1_ge_bh": fv >= bh, "fva": fv, "bh": round(bh, 1)}
    reg = modes["fusion_va"].get("regime") or {}
    bear = reg.get("bear", {}).get("alpha") if reg.get("bear", {}).get("years") else None
    bull = reg.get("bull", {}).get("alpha") if reg.get("bull", {}).get("years") else None
    if bear is not None or bull is not None:
        v["U7_bear_bull_nonneg"] = all(x is None or x >= 0 for x in (bear, bull))
        v["U7_bear_alpha"] = bear
        v["U7_bull_alpha"] = bull
    return v


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

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
           "design": "公理演绎 fusion_va 证伪检验（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        if mode == "fusion_va":
            a["gates_va"] = gates_va(res)
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

    out["verdicts"] = verdicts(sym, bh_pct, out["modes"])
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
        (DATA_DIR / f"axiom_voice_{sym}.json").write_text(
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
        (DATA_DIR / "axiom_voice_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    ok = [r for r in summary if "failed" not in r]
    hits = {r["symbol"]: r["verdicts"].get("P1_ge_bh") for r in ok}
    print(f"P1_ge_bh: {sum(bool(x) for x in hits.values())}/{len(ok)} {hits}",
          flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
