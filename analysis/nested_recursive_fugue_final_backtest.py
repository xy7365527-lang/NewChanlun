"""嵌套递归赋格（NRF final，mode="nrf"）八标的回测。

上游：2026-06-12 编排者任务"逐仓独立头寸和递归区间套是同一件事"——
区间套递归 = voice spawn 的时序机制；逐仓独立头寸 = 区间套递归的物质形态。
实装：`rust/src/trading/nested_fugue.rs`（统一递归机制：父层反向 candidate
武装 × 次级别第一证据触发 ⇒ 父仓不动、k−1 开方向交替的独立逐仓头寸；
生命周期 = 该层走势完美 confirmed；否定 = 027:25 破极值级联；终止 =
floor ∨ 35课成本门 ∨ 槽占用；根 = 最高 θ 涌现层买证据开多）。

[镜像推导] 声明（090号）：子层空头头寸无原文净空头锚——38:36 镜像授权止于
判断-动作序列；1x 虚拟逐仓强平 = 损失有界凸性载体（双向会计 §4.3 口径）。

══════════════ 预注册判据（先于运行声明）══════════════

P1（任务判据）：nrf ≥ BH 逐标的；8/8 = 任务达成线。
   在册先验：8/8 ≥ BH 在"零标的级参数"约束下从未达成（fusion_v 4/8、
   nor4 5/8、fusion_t 5/8）；ES/QQQ 慢牛域无任何 fusion 系臂超 BH。
N1 机制活性：n_nrf_spawns > 0 ∧ n_nrf_root_entries > 0（零触发 = 词汇
   空集 ⇒ 判决无效非否证）。
N2 多空嵌套真实并发：nrf_depth_bars 在 depth ≥ 2 有非零质量（stretto），
   且 depth ≥ 3（如 L3多+L2空+L1多）出现 ⇒ 第22环方向交替递归被实证。
N3 递归终止涌现：成本门/floor/槽占用三范畴计数报告；若 spawn 恒被
   noref/cost 全拒（spawns≈0）⇒ 深度未涌现（机制无效域）。
N4 尾部风险：n_short_liquidations 计数；强平损失有界由 1x 逐仓构造保证。
B1 守卫：tape_fp 与 p7_conj_<SYM>.json 在册指纹逐项一致；漂移 ⇒ 读数作废。

════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/nested_recursive_fugue_final_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_final_<SYM>.json + nrf_final_summary.json
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
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5", 8: "recL6"}
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]


def bh_mdd(closes) -> float:
    peak = mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        mdd = min(mdd, c / peak - 1.0)
    return mdd


def analyze(res: dict, closes, years) -> dict:
    """NAV 重建守卫（双极性现金流）+ MDD + 分年 + 层×极性分解 + nrf 计数。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        if pol == "long":
            dshares[eb] += sh
            dshares[xb] -= sh
            dcash[eb] -= sh * ep
            dcash[xb] += sh * xp
        else:
            dshares[eb] -= sh
            dshares[xb] += sh
            dcash[eb] += sh * ep
            dcash[xb] -= sh * xp
    nav = [0.0] * n
    shares = 0.0
    pool = 100_000.0
    peak = mdd = 0.0
    yearly: dict[str, dict] = {}
    prev_expo = 0.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav[i] = pool + shares * closes[i]
        peak = max(peak, nav[i])
        if peak > 0:
            mdd = min(mdd, nav[i] / peak - 1.0)
        if years is not None and i >= 1 and nav[i] > 0 and nav[i - 1] > 0:
            y = yearly.setdefault(str(years[i]),
                                  {"bh_log": 0.0, "strat_log": 0.0,
                                   "exp_sum": 0.0, "bars": 0})
            y["bh_log"] += math.log(closes[i] / closes[i - 1])
            y["strat_log"] += math.log(nav[i] / nav[i - 1])
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}（双极性现金流分派有 bug）"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log"):
            y[k] = round(y[k], 4)

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        d = by_ladder.setdefault((lad, pol), {"n": 0, "pnl_cash": 0.0,
                                              "wins": 0, "held": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["held"] += xb - eb
    for d in by_ladder.values():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["held"]

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly if years is not None else None,
        "by_ladder": {f"{LADDER_NAMES.get(k, str(k))}/{pol}": v
                      for (k, pol), v in sorted(by_ladder.items())},
        "nrf_counters": {
            "root_entries": res["n_nrf_root_entries_by_ladder"],
            "spawns": res["n_nrf_spawns_by_ladder"],
            "negates": res["n_nrf_negate_closes_by_ladder"],
            "cascades": res["n_nrf_cascade_closes_by_ladder"],
            "cost_rejects": res["n_nrf_cost_rejects_by_ladder"],
            "noref_rejects": res["n_nrf_noref_rejects_by_ladder"],
            "busy_skips": res["n_nrf_busy_skips_by_ladder"],
            "floor_stops": res["n_nrf_floor_stops_by_ladder"],
            "dust_skips": res["n_nrf_dust_skips_by_ladder"],
            "depth_bars": res["nrf_depth_bars"],
            "nest_arms": res["n_nest_arms_by_ladder"],
            "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
            "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
            "nest_breaks": res["n_nest_breaks_by_ladder"],
            "liquidations": res["n_short_liquidations_by_ladder"],
            "short_net_cash": [round(x, 0)
                               for x in res["short_net_cash_by_ladder"]],
            "short_held_bars": res["short_held_bars_by_ladder"],
        },
    }


def prereg(a: dict, bh_pct: float) -> dict:
    """预注册判据裁决（P1/N1-N4，先于运行声明在模块 docstring）。"""
    c = a["nrf_counters"]
    depth_bars = c["depth_bars"]
    return {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "N1_active": [sum(c["root_entries"]), sum(c["spawns"]),
                      sum(c["root_entries"]) > 0 and sum(c["spawns"]) > 0],
        "N2_depth2_bars": [depth_bars[2] if len(depth_bars) > 2 else 0,
                           sum(depth_bars[3:]),
                           (depth_bars[2] if len(depth_bars) > 2 else 0) > 0],
        "N3_termination": {"cost": sum(c["cost_rejects"]),
                           "noref": sum(c["noref_rejects"]),
                           "busy": sum(c["busy_skips"]),
                           "floor": sum(c["floor_stops"])},
        "N4_liquidations": sum(c["liquidations"]),
    }


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

    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="nrf")
    a = analyze(res, closes, years)
    pr = prereg(a, bh_pct)
    print(f"[{sym}][nrf] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"roots={sum(a['nrf_counters']['root_entries'])} "
          f"spawns={sum(a['nrf_counters']['spawns'])} "
          f"liq={pr['N4_liquidations']} P1={pr['P1_ge_bh'][2]}",
          flush=True)
    return {"symbol": sym, "n_bars": n, "tape_fp": fp,
            "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "design": "嵌套递归赋格 NRF final（统一递归机制，零 flag；"
                      "判据见模块 docstring，空头侧 [镜像推导]）",
            "nrf": a, "preregistered": pr}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_final_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "nrf_pct": out["nrf"]["strat_pct"],
                            "nrf_mdd": out["nrf"]["mdd_pct"],
                            "preregistered": out["preregistered"]})
        (DATA_DIR / "nrf_final_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
