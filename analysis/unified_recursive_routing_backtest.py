"""统一配置 U（fusion_u：相位递归路由 osc 层）八标的预注册回测。

上游：analysis/unified_config_regime_research.md §5/§6（v2，commit fdfad636b0）
+ fusion_t_eight_asset_l3.md（基座在册 L3）。引擎实装为并行工位任务 1
（unified_osc.rs + OscRouting + run_fusion 集成 + lib.rs marshal），本脚本
零接触 rust/，纯消费 mode ∈ {"fusion_u", "fusion_uw"}。

对照臂：hold26 / fusion_t（守卫+P2 基线）/ fusion_uw（U−③ 强震荡门消融）
/ fusion_u（完整 U）/ BH。

══════════════ 预注册判据（先于回测数据声明）══════════════

本脚本结算 research §6 中引擎已实装轴的判据子集 P1-P5（P6-P8 需相位机
基座/位置门变体，引擎未实装，不在本轮）。

口径声明：在册最优各线（ht/sc/co/scco/h1/h3/h4/hold26/fusion_t）均为
零摩擦 close 口径 ⇒ P1/P2 主判据在零摩擦面结算（同口径可比）；
摩擦调整面（每侧 0.05%，rt=0.001 与引擎 SUB_FRICTION_RT 同常数）
对本轮四臂 + BH 全部并报，作 P1f/P2f 副判据（H4 判决：实盘口径=
含摩擦口径；在册线无摩擦值，跨线摩擦比较不可得，显式声明）。

P1 白名单消除主判据：fusion_u ≥ max(BH, 0.9×best_in_book) 逐标的，8/8。
    best_in_book = max{ht, ht_sc, ht_co, ht_scco, ht_h1*, ht_h3*, ht_h4*,
    hold26, fusion_t}（* 仅 BTC/OKLO/BRN/CL/ES 五标的在册），取数自
    data_cache 各在册 JSON，逐标的记录 provenance。
    P1f 副判据：fa(fusion_u) ≥ fa(BH) 逐标的。
P2 不伤基座：fusion_u ≥ max(fusion_t, hold26) 逐标的，8/8（两面）。
    osc 层是纯增量层——最坏应退化为全塔拒绝（恒仓吃趋势）。
P3 机制可观测性（门判别力）：逐标的报告 n_route_phase_skips /
    n_route_amp_{rejects,noref} / n_route_weak_{rejects,noref} /
    n_route_no_center / n_route_exhausted / n_osc_open_at_level /
    n_osc_*_restores。任一门八标的合计 rejects==0（从不触发）或
    opens 合计==0（全拒退化）⇒ 按 H2 先例单独否证该门（判决在
    结果文档，本脚本只产数据）。
P4 床位兜底假设（CL 专项）：Δ_CL = fusion_u − fusion_t（零摩擦 pp）。
    Δ_CL < −14.7pp（在册 h3 无门上移的亏损）⇒「门系兜底 k+1 床位
    真实性」假设否证；同时记录 CL 上移开腿计数（≈0 = 门拦截式保护，
    P4 经由拦截通过亦成立）。
P5 ③强震荡门消融：Δ③ = fusion_u − fusion_uw 逐标的 8/8 全表。
    ③是本架构唯一新词汇（093:26 严格可导出，research §2.4 g2），
    必须单独消融；fusion_uw 含 route 计数对照（weak_rejects 恒 0）。

guard-a hold26 / guard-b fusion_t：逐位复现在册（容差 0.05pp）；
tape_fp 守卫对 osc_scco_<SYM>.json。缺在册文件 = 守卫不可执行 ⇒
该标的 failed（不静默跳过）。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/unified_recursive_routing_backtest.py [SYM ...]
输出：analysis/data_cache/unified_rr_<SYM>.json + unified_rr_summary.json
"""

from __future__ import annotations

import json
import math
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

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
MODES = ["hold26", "fusion_t", "fusion_uw", "fusion_u"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
REGIME_NATS = 0.10
FRICTION_SIDE = 0.0005  # rt=0.001 / 2，与引擎 SUB_FRICTION_RT 同常数
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}
GUARD_TOL = 0.05

# P3 观测面 marshal 键（lib.rs，缺键 = 引擎能力不符 ⇒ fail-fast）
ROUTE_KEYS = ["n_route_phase_skips", "n_route_no_center",
              "n_route_amp_rejects", "n_route_amp_noref",
              "n_route_weak_rejects", "n_route_weak_noref"]
OSC_KEYS = ["n_osc_opens_by_ladder", "n_osc_open_at_level",
            "n_osc_upshift_opens_by_ladder", "n_osc_out_bars_by_ladder",
            "n_osc_up_out_bars_by_ladder", "n_osc_zd_restores_by_ladder",
            "n_osc_death_restores_by_ladder", "n_osc_shift_restores_by_ladder",
            "n_osc_kbuy_restores_by_ladder", "n_osc_phase_restores_by_ladder",
            "n_osc_sell3_vetos_by_ladder", "n_osc_escalates_by_ladder"]

# 在册最优取数线（文件模板, cells/modes 键, 五标的限定与否）
BOOK_LINES = [
    ("osc_scco_{s}.json", "cells", "V2oa25_ht", False),
    ("osc_scco_{s}.json", "cells", "V2oa25_ht_sc", False),
    ("osc_scco_{s}.json", "cells", "V2oa25_ht_co", False),
    ("osc_scco_{s}.json", "cells", "V2oa25_ht_scco", False),
    ("h1_cf_{s}.json", "cells", "V2oa25_ht_h1", True),
    ("h3_lu_{s}.json", "cells", "V2oa25_ht_h3", True),
    ("h4_amp_{s}.json", "cells", "V2oa25_ht_h4", True),
    ("fusion_t_eight_{s}.json", "modes", "hold26", False),
    ("fusion_t_eight_{s}.json", "modes", "fusion_t", False),
]


def best_in_book(sym: str) -> dict:
    """逐线扫描 data_cache 在册零摩擦复利，返回 best + provenance。"""
    lines: dict[str, float] = {}
    for tmpl, container, key, five_only in BOOK_LINES:
        p = DATA_DIR / tmpl.format(s=sym)
        if not p.exists():
            if not five_only:
                raise FileNotFoundError(f"在册参照缺失：{p.name}（守卫不可执行）")
            continue
        d = json.loads(p.read_text())
        c = d[container].get(key)
        if c is None:
            continue
        if container == "cells":
            lines[key] = round(c["metrics"]["total_compound"], 1)
        else:
            lines[f"positional_{key}"] = c["strat_pct"]
    best_line = max(lines, key=lambda k: lines[k])
    return {"best": lines[best_line], "best_line": best_line, "lines": lines}


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → 双面 NAV 重建（零摩擦守卫 + 摩擦调整）+ 归因 + U 观测面。

    引擎 trades 为 long-form 规范化元组（先卖后买腿已拆为平仓+新开仓，
    在册 hold26_cs_fusion 重建守卫先例），统一重建合法。
    """
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    dfric = [0.0] * (n + 1)  # 摩擦面额外现金流（每侧 FRICTION_SIDE）
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) in trades:
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep
        dcash[xb] += sh * xp
        dfric[eb] -= sh * ep * FRICTION_SIDE
        dfric[xb] -= sh * xp * FRICTION_SIDE
    nav = [0.0] * n
    shares = 0.0
    pool = 100_000.0
    pool_f = 100_000.0
    nav_f_i = 0.0
    peak = mdd = peak_f = mdd_f = 0.0
    yearly: dict[str, dict] = {}
    prev_expo = 0.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        pool_f += dcash[i] + dfric[i]
        nav[i] = pool + shares * closes[i]
        nav_f_i = pool_f + shares * closes[i]
        peak = max(peak, nav[i])
        mdd = min(mdd, nav[i] / peak - 1.0)
        peak_f = max(peak_f, nav_f_i)
        mdd_f = min(mdd_f, nav_f_i / peak_f - 1.0)
        if years is not None and i >= 1:
            y = yearly.setdefault(str(years[i]),
                                  {"bh_log": 0.0, "strat_log": 0.0,
                                   "flat_missed": 0.0, "exp_sum": 0.0,
                                   "bars": 0})
            r_mkt = math.log(closes[i] / closes[i - 1])
            y["bh_log"] += r_mkt
            y["strat_log"] += math.log(nav[i] / nav[i - 1])
            y["flat_missed"] += (1.0 - prev_expo) * r_mkt
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log", "flat_missed"):
            y[k] = round(y[k], 4)

    if years is None:
        yearly_out = None
        regime = None
    else:
        regime_years: dict[str, list] = {"bull": [], "bear": [], "range": []}
        for yk, v in sorted(yearly.items()):
            if v["bh_log"] >= REGIME_NATS:
                regime_years["bull"].append(yk)
            elif v["bh_log"] <= -REGIME_NATS:
                regime_years["bear"].append(yk)
            else:
                regime_years["range"].append(yk)

        def agg(ys):
            b = sum(yearly[y]["bh_log"] for y in ys)
            s = sum(yearly[y]["strat_log"] for y in ys)
            return {"bh_log": round(b, 4), "strat_log": round(s, 4),
                    "alpha": round(s - b, 4), "years": ys}

        regime = {r: agg(ys) for r, ys in regime_years.items()}
        yearly_out = yearly

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    osc_pnl_by_ladder: dict[int, float] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        pnl = sh * (xp - ep)
        if reason.startswith("osc_"):
            osc_pnl_by_ladder[lad] = osc_pnl_by_ladder.get(lad, 0.0) + pnl
        d = by_ladder.setdefault(lad, {"n": 0, "pnl_cash": 0.0, "wins": 0,
                                       "w_sum": 0.0, "held": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["w_sum"] += w
        d["held"] += xb - eb
    for lad, d in by_ladder.items():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_weight"] = round(d["w_sum"] / d["n"], 4)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["w_sum"], d["held"]

    missing = [k for k in ROUTE_KEYS + OSC_KEYS + ["n_route_exhausted"]
               if k not in res]
    if missing:
        raise KeyError(f"P3 观测面 marshal 缺键（引擎能力不符）：{missing}")
    gates = {k: res[k] for k in ROUTE_KEYS}
    gates["n_route_exhausted"] = res["n_route_exhausted"]
    osc_obs = {k: res[k] for k in OSC_KEYS}
    osc_obs["n_osc_restore_defer_bars"] = res.get("n_osc_restore_defer_bars")

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "fa_strat_pct": round((nav_f_i / 100_000.0 - 1) * 100, 1),
        "fa_mdd_pct": round(mdd_f * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "osc_pnl_by_ladder": {LADDER_NAMES.get(k, str(k)): round(v, 0)
                              for k, v in sorted(osc_pnl_by_ladder.items())},
        "trend_holds_by_ladder": res["n_trend_holds_by_ladder"],
        "gates": gates,
        "osc_obs": osc_obs,
        "yearly": yearly_out,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
    }


def prereg(sym: str, modes: dict, bh_pct: float, fa_bh_pct: float,
           book: dict) -> dict:
    """P1-P5 逐标的裁决（判据全文见模块 docstring，先于回测声明）。"""
    u, uw = modes["fusion_u"], modes["fusion_uw"]
    ft, h = modes["fusion_t"], modes["hold26"]
    base = max(ft["strat_pct"], h["strat_pct"])
    thr = max(bh_pct, 0.9 * book["best"])
    out: dict = {"best_in_book": book["best"], "best_line": book["best_line"]}
    out["P1_u_ge_max_bh_09best"] = [u["strat_pct"], round(thr, 1),
                                    u["strat_pct"] >= thr]
    out["P1f_fa_u_ge_fa_bh"] = [u["fa_strat_pct"], round(fa_bh_pct, 1),
                                u["fa_strat_pct"] >= fa_bh_pct]
    out["P2_u_ge_base"] = [u["strat_pct"], round(base, 1),
                           u["strat_pct"] >= base]
    out["P2f_fa"] = [u["fa_strat_pct"],
                     round(max(ft["fa_strat_pct"], h["fa_strat_pct"]), 1),
                     u["fa_strat_pct"] >= max(ft["fa_strat_pct"],
                                              h["fa_strat_pct"])]
    g = u["gates"]
    out["P3_gate_counts"] = {
        "phase": sum(g["n_route_phase_skips"]),
        "amp_rej": sum(g["n_route_amp_rejects"]),
        "amp_noref": sum(g["n_route_amp_noref"]),
        "weak_rej": sum(g["n_route_weak_rejects"]),
        "weak_noref": sum(g["n_route_weak_noref"]),
        "no_center": sum(g["n_route_no_center"]),
        "exhausted": g["n_route_exhausted"],
        "opens_at_level": u["osc_obs"]["n_osc_open_at_level"],
    }
    if sym == "CL":
        delta = u["strat_pct"] - ft["strat_pct"]
        out["P4_cl_bed_guard"] = [
            round(delta, 1), delta > -14.7,
            {"upshift_opens": sum(u["osc_obs"]
                                  ["n_osc_upshift_opens_by_ladder"])}]
    out["P5_strong_gate_delta"] = [u["strat_pct"], uw["strat_pct"],
                                   round(u["strat_pct"] - uw["strat_pct"], 1)]
    return out


def run_symbol(sym: str) -> dict:
    book = best_in_book(sym)
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    fa_bh_pct = ((closes[-1] * (1 - FRICTION_SIDE))
                 / (closes[0] * (1 + FRICTION_SIDE)) - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% best_in_book="
          f"{book['best']:+.1f} ({book['best_line']})", flush=True)

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

    ref = json.loads((DATA_DIR / f"osc_scco_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    ft_book = json.loads(
        (DATA_DIR / f"fusion_t_eight_{sym}.json").read_text())["modes"]
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "fa_bh_pct": round(fa_bh_pct, 1),
           "bh_mdd_pct": round(bh_dd, 1), "book": book,
           "friction_side": FRICTION_SIDE,
           "design": "fusion_u 八标的预注册（P1-P5，判据见探针 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% fa={a['fa_strat_pct']:+.1f}% "
              f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
              f"reasons={a['exit_reasons']}", flush=True)
    for mode, key in (("hold26", "hold26"), ("fusion_t", "fusion_t")):
        got = out["modes"][mode]["strat_pct"]
        want = ft_book[key]["strat_pct"]
        if abs(got - want) >= GUARD_TOL:
            return {"symbol": sym,
                    "failed": f"guard_{mode}_drift: {got} ≠ {want}"}
    out["preregistered"] = prereg(sym, out["modes"], bh_pct, fa_bh_pct, book)
    print(f"[{sym}] prereg={json.dumps(out['preregistered'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"unified_rr_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "best_in_book": out["book"]["best"],
                   "best_line": out["book"]["best_line"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_fa"] = m["fa_strat_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "unified_rr_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
