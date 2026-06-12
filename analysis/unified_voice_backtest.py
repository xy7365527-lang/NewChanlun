"""统一递归 voice FSM（fusion_v）八标的 L3 回测——一个配置，零标的级参数。

上游：
- `analysis/unified_recursive_voice_fsm_design.md` v2（设计 + 预注册体系）
- `rust/src/trading/unified_voice.rs`（fusion_v 实装：Φ 三值化 + freeze_d
  递归传导 + R4 双侧出口 + R14 默认落点 Gated 全层 + nest 恒开 + 统一 osc
  三门恒开 + R18-20 T2W）

══════════════ 预注册判据（先于数据声明）══════════════

P1（编排者最高优先级）：fusion_v ≥ BH，逐标的符号 8/8 标注。
U1：fusion_v ≥ fusion_t（统一接线不劣于在册单轴基座），8/8 标注。
U2（4/4 硬线）：纯净检验域 {BTC, CL, ES, QQQ} 上 fusion_v ≥ 在册最优臂
   × 0.8。在册最优 = max over 在册 json（p7_conj / universal_v2 全臂）；
   失败 ⇒ 门共享 + 读数内生化命题否证。
U2x：预名允许失败集 {GC, DX}（负先验在册：GC"任何停削窗口都伤它"/
   DX 无 bull 年）——失败按 D3 残差结算，不计框架否证。
U3c（R4 正域守卫）：BTC/CL 上 fusion_v ≥ fusion_t × 0.95——R4 出口在
   正域放血 ⇒ R4 多侧降消融位。
U6（声部吸收，OKLO/BRN）：fusion_v ≥ 声部系在册线 × 0.8（OKLO _ht
   +2436.2 / BRN _ht +681.3，v2oa25_ht 代次）——失败 = 声部机器吸收
   不足，非门架构失败（独立工程预注册）。
U7（绩效公理可证伪形式）：bear α 与 bull α 分桶同时非负，逐标的标注
   （analyze 的 regime 聚合直读）。
可观测性：freeze_up/dn 驻留、R4 出口驻留与窗口内削减/回补、T2W
   武装/触发/否定、Gated 进出、nest 触发、osc 路由拒因——全部非零判别。

guard-a：tape_fp 与 p7_conj 在册逐项一致（代次钉死，M15-d）。
guard-b：hold26 / fusion_tr 逐位复现在册（0.05pp）。

诚实声明（090号）：杠杆三元组（L_max 逐 bar）与真实空头/认沽载体不在
本轮——设计 v2 Phase 1 范围外，开放轴。η = 实现/M·TV_adm 报告延后
（可达全变差分母需独立计算管线），本轮以 bear/bull 分桶承载 U7 主体。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/unified_voice_backtest.py [SYM ...]
输出：analysis/data_cache/unified_voice_<SYM>.json + unified_voice_summary.json
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

MODES = ["hold26", "fusion_tr", "fusion_t", "fusion_v",
         # 消融臂（U3c/M7-a 失败语义的定位工具——三新轴各自边际）：
         "fusion_v_nor4",   # 关 R4 双侧出口
         "fusion_v_flat",   # 削减落点 Flat（Gated 义务门摘除）
         "fusion_v_self"]   # freeze 仅自层（递归传导摘除）
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
PURE_DOMAIN = {"BTC", "CL", "ES", "QQQ"}
ALLOWED_FAIL = {"GC", "DX"}
# 声部系在册线（v2oa25_ht 代次；U6 对照——非 guard，结构性对照）。
VOICE_LINE = {"OKLO": 2436.2, "BRN": 681.3}


def in_book_best(sym: str) -> tuple[float, str]:
    """在册最优臂（fusion 系全臂 max；代次 = p7_conj/universal_v2 指纹）。"""
    best, src = float("-inf"), "none"
    for stem in (f"universal_v2_{sym}", f"universal_combo_{sym}",
                 f"p7_conj_{sym}"):
        f = DATA_DIR / f"{stem}.json"
        if not f.exists():
            continue
        d = json.loads(f.read_text())
        for mode, a in d.get("modes", {}).items():
            if isinstance(a, dict) and "strat_pct" in a:
                if a["strat_pct"] > best:
                    best, src = a["strat_pct"], f"{stem}:{mode}"
    return best, src


def gates_v(res: dict) -> dict:
    """fusion_v 观测面（非零判别义务）。"""
    return {
        "freeze_up_bars": res["freeze_up_bars_by_ladder"],
        "freeze_dn_bars": res["freeze_dn_bars_by_ladder"],
        "r4_up_exit_bars": res["r4_up_exit_bars_by_ladder"],
        "r4_dn_exit_bars": res["r4_dn_exit_bars_by_ladder"],
        "r4_up_trims": res["n_r4_up_trims_by_ladder"],
        "r4_dn_restores": res["n_r4_dn_restores_by_ladder"],
        "t2w_arms": res["n_t2w_arms_by_ladder"],
        "t2w_fires": res["n_t2w_fires_by_ladder"],
        "t2w_negates": res["n_t2w_negates_by_ladder"],
        "gate_enters": res["n_gate_enters_by_ladder"],
        "gate_restores": res["n_gate_restores_by_ladder"],
        "gate_moveup_restores": res["n_gate_moveup_restores_by_ladder"],
        "gate_held_bars": res["gate_held_bars_by_ladder"],
        "trend_holds": res["n_trend_holds_by_ladder"],
        "anc_exempt_blocks": res["n_anc_exempt_blocks_by_ladder"],
        "movedown_holds": res["n_short_trend_holds_by_ladder"],
        "restore_r2_blocks": res["n_short_r2_blocks_by_ladder"],
        "trim_r2_blocks": res["n_r2_pos_blocks_by_ladder"],
        "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
        "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
        "nest_breaks": res["n_nest_breaks_by_ladder"],
        "osc_opens": res["n_osc_opens_by_ladder"],
        "osc_escalates": res["n_osc_escalates_by_ladder"],
        "route_phase_skips": [int(x) for x in res["n_route_phase_skips"]],
        "route_amp_rejects": res["n_route_amp_rejects"],
        "route_weak_rejects": res["n_route_weak_rejects"],
        "route_h1_freezes": res["n_route_h1_freezes"],
        "osc_net_cash": [round(x, 0) for x in res["osc_net_cash_by_ladder"]],
    }


def verdicts(sym: str, bh: float, modes: dict, best: float) -> dict:
    """预注册判据逐标的评估。"""
    fv = modes["fusion_v"]["strat_pct"]
    ft = modes["fusion_t"]["strat_pct"]
    v = {
        "P1_ge_bh": fv >= bh,
        "U1_ge_fusion_t": fv >= ft,
        "fv": fv, "ft": ft, "bh": round(bh, 1), "in_book_best": best,
    }
    if sym in PURE_DOMAIN:
        v["U2_ge_book08"] = fv >= best * 0.8 if best > 0 else fv >= best
    if sym in ALLOWED_FAIL:
        v["U2x_d3_residual"] = True
    if sym in ("BTC", "CL"):
        v["U3c_ge_ft095"] = fv >= ft * 0.95 if ft > 0 else fv >= ft
    if sym in VOICE_LINE:
        v["U6_ge_voice08"] = fv >= VOICE_LINE[sym] * 0.8
    reg = modes["fusion_v"].get("regime") or {}
    bear = reg.get("bear", {}).get("alpha") if reg.get("bear", {}).get("years") else None
    bull = reg.get("bull", {}).get("alpha") if reg.get("bull", {}).get("years") else None
    if bear is not None or bull is not None:
        v["U7_bear_bull_nonneg"] = all(
            x is None or x >= 0 for x in (bear, bull))
        v["U7_bear_alpha"] = bear
        v["U7_bull_alpha"] = bull
    return v


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
           "design": "统一递归 voice FSM fusion_v 八标的（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        if mode == "fusion_v":
            a["gates_v"] = gates_v(res)
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

    best, src = in_book_best(sym)
    out["in_book_best_src"] = src
    out["verdicts"] = verdicts(sym, bh_pct, out["modes"], round(best, 1))
    # 消融边际（fusion_v − 单轴摘除臂 = 该轴的贡献符号）。
    s = {m: out["modes"][m]["strat_pct"] for m in MODES}
    out["ablation"] = {
        "d_r4": round(s["fusion_v"] - s["fusion_v_nor4"], 1),
        "d_gated": round(s["fusion_v"] - s["fusion_v_flat"], 1),
        "d_anc": round(s["fusion_v"] - s["fusion_v_self"], 1),
    }
    print(f"[{sym}] verdicts={json.dumps(out['verdicts'])}", flush=True)
    print(f"[{sym}] ablation={json.dumps(out['ablation'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"unified_voice_{sym}.json").write_text(
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
        (DATA_DIR / "unified_voice_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    ok = [r for r in summary if "failed" not in r]
    for key in ("P1_ge_bh", "U1_ge_fusion_t"):
        hits = {r["symbol"]: r["verdicts"].get(key) for r in ok}
        npos = sum(bool(x) for x in hits.values())
        print(f"{key}: {npos}/{len(ok)} {hits}", flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
