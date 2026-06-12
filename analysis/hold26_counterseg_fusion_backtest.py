"""hold26 × CounterSeg 合流（B+C 合体）BTC+OKLO 回测——预注册判据 F1-F5。

设计：534号谱系（49课行52二相消解）+ analysis/bc_architecture_research.md
（B+C 合体原文判决）+ analysis/nested_recursive_fugue_results.md §5.3（合流方向）。
实现：rust/src/trading/positional_fusion.rs（PolarityMode::Fusion）。

四臂（同磁带；hold26 为在册基线复跑守卫，数字须与
positional_fugue_multi_<SYM>.json 逐位一致）：
  hold26    26课恒仓基座（在册 L2：BTC +1247.1% / OKLO +96.0%）
  fusion_t  + 趋势相停削（49课:52 二相，kind==Trend ∧ dir==Up 停削）
  fusion_s  + 层内 C 短差（53课次级别词汇 + 49课:64 如数接回 + 44课:44 铰链）
  fusion    + 两轴合取（B+C 合体完整形态）

预注册判据（先于数据声明；VLg 负交互先例 ⇒ F3 交互项必列）：
  F1a BTC：fusion > hold26（+1247.1%，任务核心目标下界）
  F1b BTC：fusion ≥ BH（+1380.4%，任务上界目标）
  F2a OKLO：fusion ≥ hold26@OKLO（架构内不恶化）
  F2b OKLO：fusion > +2301.1%（NRF2q 跨架构参照，独立账本不逐位可比仅方向对照）
  F3  正交互：fusion > max(fusion_t, fusion_s)（每标的；否证侧 = 负交互）
  F4  BTC 牛市赤字收窄：fusion bull α > hold26 bull α（−2.02 nats 在册）
  F5  熊市 α 保持：fusion bear α ≥ hold26 bear α − 0.10 nats（停削只在
      dir==Up 活动，熊市承载面理论零接触——大幅恶化即实现违规信号）

用法：PYTHONPATH=src .venv/bin/python analysis/hold26_counterseg_fusion_backtest.py
输出：analysis/data_cache/hold26_cs_fusion_<SYM>.json + _summary.json
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
MODES = ["hold26", "fusion_t", "fusion_s", "fusion"]
# CL = hold26 八标的 L3 判决（hold26_multi_asset_results.md §4）指定的合流
# 首选标的（唯一双相）；任务原指定 BTC+OKLO 保留。argv 可选子集复跑。
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "CL"]
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}
# 在册对照（跨架构参照，独立账本——仅方向对照不逐位比较）
NRF2Q_OKLO = 2301.1
HOLD26_IN_BOOK = {"BTC": 1247.1, "OKLO": 96.0, "CL": 274.9}

FUSION_COUNTERS = (
    "n_trend_holds_by_ladder", "n_sub_opens_by_ladder",
    "n_sub_restores_by_ladder", "n_sub_kbuy_restores_by_ladder",
    "n_sub_escalates_by_ladder", "n_sub_phase_closes_by_ladder",
    "n_sub_cost_rejects_by_ladder", "n_sub_noref_rejects_by_ladder",
)


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → NAV 重建 + MDD + 分年/regime + 分层归因 + fusion 观测面。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100

    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) in trades:
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep
        dcash[xb] += sh * xp
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
        mdd = min(mdd, nav[i] / peak - 1.0)
        if i >= 1:
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

    regime_years = {"bull": [], "bear": [], "range": []}
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

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        d = by_ladder.setdefault(lad, {"n": 0, "pnl_cash": 0.0, "wins": 0,
                                       "w_sum": 0.0, "held": 0})
        d["n"] += 1
        pnl = sh * (xp - ep)
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["w_sum"] += w
        d["held"] += xb - eb
    for lad, d in by_ladder.items():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_weight"] = round(d["w_sum"] / d["n"], 4)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["w_sum"], d["held"]

    fusion_obs = {k: res[k] for k in FUSION_COUNTERS}
    fusion_obs["n_sub_restore_defer_bars"] = res["n_sub_restore_defer_bars"]
    fusion_obs["sub_net_cash_by_ladder"] = [
        round(v, 0) for v in res["sub_net_cash_by_ladder"]]

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
        "fusion_obs": fusion_obs,
    }


def prereg(sym: str, modes: dict) -> dict:
    """预注册判据 F1-F5 裁决（先验声明见模块 docstring）。"""
    f = modes["fusion"]
    h = modes["hold26"]
    ft, fs = modes["fusion_t"], modes["fusion_s"]
    out: dict = {}
    if sym == "BTC":
        out["F1a_fusion_gt_hold26"] = [f["strat_pct"], h["strat_pct"],
                                       f["strat_pct"] > h["strat_pct"]]
        out["F1b_fusion_ge_bh"] = [f["strat_pct"], 1380.4,
                                   f["strat_pct"] >= 1380.4]
        out["F4_bull_deficit_narrows"] = [
            f["regime"]["bull"]["alpha"], h["regime"]["bull"]["alpha"],
            f["regime"]["bull"]["alpha"] > h["regime"]["bull"]["alpha"]]
    if sym == "OKLO":
        out["F2a_fusion_ge_hold26"] = [f["strat_pct"], h["strat_pct"],
                                       f["strat_pct"] >= h["strat_pct"]]
        out["F2b_fusion_gt_nrf2q"] = [f["strat_pct"], NRF2Q_OKLO,
                                      f["strat_pct"] > NRF2Q_OKLO]
    if sym == "CL":
        # 双相标的合流主判据（hold26 L3 判决指定标的；先验声明于运行前）
        out["FCL_fusion_gt_hold26"] = [f["strat_pct"], h["strat_pct"],
                                       f["strat_pct"] > h["strat_pct"]]
    single_max = max(ft["strat_pct"], fs["strat_pct"])
    out["F3_positive_interaction"] = [f["strat_pct"], single_max,
                                      f["strat_pct"] > single_max]
    out["F5_bear_alpha_kept"] = [
        f["regime"]["bear"]["alpha"], h["regime"]["bear"]["alpha"],
        f["regime"]["bear"]["alpha"] >= h["regime"]["bear"]["alpha"] - 0.10]
    out["guard_hold26_in_book"] = [h["strat_pct"], HOLD26_IN_BOOK[sym],
                                   abs(h["strat_pct"] - HOLD26_IN_BOOK[sym])
                                   < 0.05]
    return out


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}%", flush=True)

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
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1),
           "design": "positional_fusion.rs（534号二相 + 44课铰链）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} reasons={a['exit_reasons']}",
              flush=True)
    out["preregistered"] = prereg(sym, out["modes"])
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
        (DATA_DIR / f"hold26_cs_fusion_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        (DATA_DIR / "hold26_cs_fusion_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
