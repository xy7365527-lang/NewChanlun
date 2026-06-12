"""P6 相位机八标的预注册回测（fusion_p / fusion_pu）。

上游：analysis/p6_phase_machine_research.md（调研+设计）
+ unified_config_regime_research.md §6 P6（预注册母条款）
+ unified_rr_results.md §3（8/8 符号反相关——实证动机）。
引擎：fusion_p = 相位机基座配对臂（停削窗口 = candidate 离开窗口 ∪
confirmed Buy3 锁定区间 [Buy3, settle(C′)]，kind 行零消费）；
fusion_pu = 相位机基座 + 统一 osc 层（双侧同步——同一时钟修两侧）。

对照臂：hold26 / fusion_t（守卫+基线）/ fusion_u（kind 时钟 osc 层，
反相关对照——本轮引擎含铰链可达性修复 23604dd5d0，与在册 unified_rr
数字口径不同，两数字不可混表，修复影响并报）/ fusion_p / fusion_pu / BH。

══════════════ 预注册判据（先于回测数据声明）══════════════

口径：零摩擦 close 主口径（在册线同口径可比）；摩擦调整面（每侧 0.05%，
rt=0.001 与引擎 SUB_FRICTION_RT 同常数）全臂并报为副判据。

P6a（research §6 P6 母条款，基座差分 fusion_p vs fusion_t，其余全同）：
    预测 ΔGC > 0 ∧ ΔDX > 0（kind 窗口错关的 range 震荡削减恢复）
    ∧ BTC 比值守卫 nav_p/nav_t ≥ 0.95（"ΔBTC ≥ −5%"的口径精确化：
    NAV 比值——research 未声明口径，本脚本先于数据固定为乘性读法）。
    失败处理（预注册）：ΔGC≤0 ∨ ΔDX≤0 ⇒ §2.2 错位定理 L3 检验失败，
    相位机不升默认；BTC 比值 < 0.95 ⇒ 049:54 与 hold26 bull 赤字证据
    定义冲突，/escalate（不改参数不调窗口）。
P6b（任务判据 1）：8/8 fusion_pu ≥ fusion_t（零摩擦；摩擦面并报）。
P6c（任务判据 2，osc 正域不恶化）：OKLO/BRN 上 Δosc′ =
    fusion_pu − fusion_p ≥ 0（osc 层增量在在册 osc 正域标的不为负）。
P6d（任务判据 3，反相关消失）：sign(Δosc′) vs −sign(Δt′)，
    Δosc′ = fusion_pu − fusion_p，Δt′ = fusion_p − hold26。
    在册 U 的反相关是 8/8 完美一致；判据 = 一致计数 ≤ 6/8（完美反相关
    被打破）。注意（边界条件）：反相关是对事后发现规律的检验，其不消失
    不构成 P6 否证（P6a/P6b 是主判据），只构成机制解释的反证。
P3 同型（机制可观测性）：相位机转移计数（up_opens/settle/div/dir 关、
    dn_opens/closes）+ MOVE↑/↓ 驻留 bar 数（与 kind×dir 窗口可比）+
    三门路由计数。任一转移类型八标的合计 == 0 ⇒ 按 H2 先例单独审查。
观察性报告（非判据）：fusion_pu vs max(BH, 0.9×best_in_book)（P1 同型
    白名单消除读数，为下一轮立项提供数据，不在本轮结算）。

守卫：hold26/fusion_t 逐位复现在册（容差 0.05pp；相位机代码对在册模式
零接触的运行时验证）；tape_fp 对 osc_scco_<SYM>.json。缺在册文件 =
守卫不可执行 ⇒ failed（不静默跳过）。

PHASE_KEYS/ROUTE_KEYS/OSC_KEYS marshal 缺键 = 引擎能力不符 ⇒ fail-fast。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/p6_phase_machine_backtest.py [SYM ...]
输出：analysis/data_cache/p6_phase_<SYM>.json + p6_phase_summary.json
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
MODES = ["hold26", "fusion_t", "fusion_u", "fusion_p", "fusion_pu"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
REGIME_NATS = 0.10
FRICTION_SIDE = 0.0005
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}
GUARD_TOL = 0.05
OSC_POSITIVE_DOMAIN = ("OKLO", "BRN")  # 在册 osc 白名单正域（P6c 对象）

ROUTE_KEYS = ["n_route_phase_skips", "n_route_no_center",
              "n_route_amp_rejects", "n_route_amp_noref",
              "n_route_weak_rejects", "n_route_weak_noref"]
OSC_KEYS = ["n_osc_opens_by_ladder", "n_osc_open_at_level",
            "n_osc_upshift_opens_by_ladder", "n_osc_out_bars_by_ladder",
            "n_osc_up_out_bars_by_ladder", "n_osc_zd_restores_by_ladder",
            "n_osc_death_restores_by_ladder", "n_osc_shift_restores_by_ladder",
            "n_osc_kbuy_restores_by_ladder", "n_osc_phase_restores_by_ladder",
            "n_osc_sell3_vetos_by_ladder", "n_osc_escalates_by_ladder",
            "n_osc_trend_hold_sells_by_ladder"]
PHASE_KEYS = ["n_phase_up_opens_by_ladder", "n_phase_up_settle_closes_by_ladder",
              "n_phase_up_div_closes_by_ladder", "n_phase_up_dir_closes_by_ladder",
              "n_phase_dn_opens_by_ladder", "n_phase_dn_closes_by_ladder",
              "phase_up_bars_by_ladder", "phase_dn_bars_by_ladder"]

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
    """单模式结果 → 双面 NAV 重建（零摩擦守卫 + 摩擦调整）+ 归因 + 观测面。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    dfric = [0.0] * (n + 1)
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

    missing = [k for k in ROUTE_KEYS + OSC_KEYS + PHASE_KEYS
               + ["n_route_exhausted"] if k not in res]
    if missing:
        raise KeyError(f"观测面 marshal 缺键（引擎能力不符）：{missing}")
    gates = {k: res[k] for k in ROUTE_KEYS}
    gates["n_route_exhausted"] = res["n_route_exhausted"]
    osc_obs = {k: res[k] for k in OSC_KEYS}
    osc_obs["n_osc_restore_defer_bars"] = res.get("n_osc_restore_defer_bars")
    phase_obs = {k: res[k] for k in PHASE_KEYS}

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
        "phase_obs": phase_obs,
        "yearly": yearly_out,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
    }


def sign(x: float) -> int:
    return (x > 0) - (x < 0)


def prereg(sym: str, modes: dict, bh_pct: float, book: dict) -> dict:
    """P6a-P6d 逐标的裁决（判据全文见模块 docstring，先于回测声明）。"""
    h = modes["hold26"]["strat_pct"]
    ft = modes["fusion_t"]["strat_pct"]
    fp = modes["fusion_p"]["strat_pct"]
    fpu = modes["fusion_pu"]["strat_pct"]
    out: dict = {"best_in_book": book["best"], "best_line": book["best_line"]}
    # P6a 分量（汇总裁决在 summary 层：GC/DX 的 Δ + BTC 的比值守卫）
    delta_p_ft = round(fp - ft, 1)
    nav_ratio = (1 + fp / 100) / (1 + ft / 100)
    out["P6a_delta_fp_ft"] = delta_p_ft
    out["P6a_nav_ratio_fp_ft"] = round(nav_ratio, 4)
    # P6b：fusion_pu ≥ fusion_t 逐标的
    out["P6b_pu_ge_ft"] = [fpu, ft, fpu >= ft]
    out["P6b_fa"] = [modes["fusion_pu"]["fa_strat_pct"],
                     modes["fusion_t"]["fa_strat_pct"],
                     modes["fusion_pu"]["fa_strat_pct"]
                     >= modes["fusion_t"]["fa_strat_pct"]]
    # P6c：osc 层增量（正域标的不为负）
    d_osc = round(fpu - fp, 1)
    out["P6c_delta_osc"] = d_osc
    if sym in OSC_POSITIVE_DOMAIN:
        out["P6c_positive_domain_ok"] = d_osc >= 0
    # P6d：反相关分量（汇总在 summary 层）
    d_t = round(fp - h, 1)
    out["P6d_signs"] = {"d_osc": sign(d_osc), "d_trend": sign(d_t),
                        "anti": sign(d_osc) == -sign(d_t) and sign(d_osc) != 0}
    # 反相关对照（kind 时钟，修复后口径）
    out["ref_kind_clock"] = {
        "d_osc_u": round(modes["fusion_u"]["strat_pct"] - ft, 1),
        "d_trend_t": round(ft - h, 1)}
    # P3 同型：相位机转移合计
    p = modes["fusion_p"]["phase_obs"]
    out["P3_phase_counts"] = {
        "up_opens": sum(p["n_phase_up_opens_by_ladder"]),
        "up_settle": sum(p["n_phase_up_settle_closes_by_ladder"]),
        "up_div": sum(p["n_phase_up_div_closes_by_ladder"]),
        "up_dir": sum(p["n_phase_up_dir_closes_by_ladder"]),
        "dn_opens": sum(p["n_phase_dn_opens_by_ladder"]),
        "dn_closes": sum(p["n_phase_dn_closes_by_ladder"]),
        "up_bars": sum(p["phase_up_bars_by_ladder"]),
        "dn_bars": sum(p["phase_dn_bars_by_ladder"]),
    }
    # 观察性（非判据）：P1 同型白名单消除读数
    thr = max(bh_pct, 0.9 * book["best"])
    out["obs_P1_pu_ge_max_bh_09best"] = [fpu, round(thr, 1), fpu >= thr]
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
           "engine_note": "含铰链可达性修复 23604dd5d0——fusion_u 数字与在册 "
                          "unified_rr（474334e950 引擎）口径不同，不可混表",
           "design": "P6 相位机八标的预注册（P6a-P6d，判据见探针 docstring）",
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
    out["preregistered"] = prereg(sym, out["modes"], bh_pct, book)
    print(f"[{sym}] prereg={json.dumps(out['preregistered'])}", flush=True)
    return out


def settle_summary(rows: list) -> dict:
    """P6a/P6d 的跨标的汇总裁决（判据见 docstring）。"""
    ok = [r for r in rows if "failed" not in r]
    verdict: dict = {}
    by = {r["symbol"]: r["preregistered"] for r in ok}
    if {"GC", "DX", "BTC"} <= set(by):
        gc = by["GC"]["P6a_delta_fp_ft"]
        dx = by["DX"]["P6a_delta_fp_ft"]
        btc = by["BTC"]["P6a_nav_ratio_fp_ft"]
        verdict["P6a"] = {
            "delta_GC": gc, "delta_DX": dx, "btc_nav_ratio": btc,
            "pass": gc > 0 and dx > 0 and btc >= 0.95,
            "btc_escalate": btc < 0.95}
    p6b = [(r["symbol"], r["preregistered"]["P6b_pu_ge_ft"][2]) for r in ok]
    verdict["P6b"] = {"per_symbol": dict(p6b),
                      "count": sum(v for _, v in p6b), "n": len(p6b),
                      "pass": all(v for _, v in p6b) and len(p6b) == 8}
    p6c = {r["symbol"]: r["preregistered"].get("P6c_positive_domain_ok")
           for r in ok if r["symbol"] in OSC_POSITIVE_DOMAIN}
    verdict["P6c"] = {"per_symbol": p6c,
                      "pass": all(v for v in p6c.values()) and len(p6c) == 2}
    anti = [(r["symbol"], r["preregistered"]["P6d_signs"]["anti"]) for r in ok]
    n_anti = sum(v for _, v in anti)
    verdict["P6d"] = {"per_symbol": dict(anti), "anti_count": n_anti,
                      "n": len(anti),
                      "anticorrelation_gone": n_anti <= 6 or len(anti) < 8}
    return verdict


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"p6_phase_{sym}.json").write_text(
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
        payload = {"rows": summary}
        if len([r for r in summary if "failed" not in r]) >= 1:
            payload["verdict"] = settle_summary(summary)
        (DATA_DIR / "p6_phase_summary.json").write_text(
            json.dumps(payload, ensure_ascii=False, indent=1))
    print(json.dumps(payload, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
