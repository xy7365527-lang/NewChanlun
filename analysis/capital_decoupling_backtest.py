"""C 轴资金解耦（earmark）BTC+OKLO+CL 回测——预注册判据 D1-D5。

任务：消解 B+C 合取负交互的资金池耦合根因（hold26_counterseg_fusion_results.md
§3.3：T 轴满仓占资 ⇒ C 轴回补义务推迟 6.6×，被迫踏空追价）。
方案：earmark（§5.2 候选）——短差卖出所得不入共享池，锁定为该层回补专款。
049:64"如数接回"义务语义的资金面物理化；53课:34 资金配额留白区的资金语义
之一，与耦合臂同为原文合法读法，本回测裁决。零新参数（231号纪律）。
实现：rust/src/trading/positional_fusion.rs（capital_decoupled，统一代数
escrow+pool——耦合臂 escrow≡0 行为逐位同旧，由六臂守卫直接验证）。

六臂（同磁带；前四臂 = 在册判决复跑守卫，须与 hold26_cs_fusion_<SYM>.json
逐位一致）：
  hold26     26课恒仓基座（在册守卫）
  fusion_t   趋势相停削单轴（在册守卫；任务约束=不可破坏其三标的全胜）
  fusion_s   层内 C 短差单轴·耦合（在册守卫）
  fusion     B+C 合取·耦合（在册守卫；BTC +1863.9 负交互基线）
  fusion_se  C 短差单轴·解耦（新臂——交互项分解用单轴）
  fusion_e   B+C 合取·解耦（新臂——本实验主臂）

预注册判据（先于数据声明）：
  D1 正交互重测（每标的）：fusion_e > max(fusion_t, fusion_se)
     ——解耦后 F3 重测；成立 ⇒ 在册"OKLO 特异"改写为"资源约束伪影"
  D2 解耦增益（每标的）：fusion_e > fusion（耦合在册：BTC +1863.9 /
     OKLO +452.4 / CL +198.3）
  D3 OKLO 主场改善：fusion_e ≥ fusion@OKLO（任务约束：C 轴解耦后
     震荡域标的应不恶化）
  D4 守卫（每标的）：hold26/fusion_t/fusion_s/fusion 四臂与在册 json
     逐位一致（|Δ| < 0.05pp）——耦合语义零漂移
  D5 机械根因验证（每标的）：fusion_e 回补推迟 bar 数 < fusion
     （BTC 874K 应崩落——若推迟不降则 earmark 未命中根因）

用法：PYTHONPATH=src .venv/bin/python analysis/capital_decoupling_backtest.py
输出：analysis/data_cache/capital_decoupling_<SYM>.json + _summary.json
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
GUARD_MODES = ["hold26", "fusion_t", "fusion_s", "fusion"]
NEW_MODES = ["fusion_se", "fusion_e"]
MODES = GUARD_MODES + NEW_MODES
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "CL"]
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}

FUSION_COUNTERS = (
    "n_trend_holds_by_ladder", "n_sub_opens_by_ladder",
    "n_sub_restores_by_ladder", "n_sub_kbuy_restores_by_ladder",
    "n_sub_escalates_by_ladder", "n_sub_phase_closes_by_ladder",
    "n_sub_cost_rejects_by_ladder", "n_sub_noref_rejects_by_ladder",
    "n_sub_pool_topup_by_ladder",
)


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → NAV 重建 + MDD + 分年/regime + 分层归因 + fusion 观测面。

    与在册 hold26_counterseg_fusion_backtest.analyze 同构（守卫可比性），
    仅 fusion_obs 增 topup 行。NAV 重建只用 trade 行现金流——escrow 是
    资金归属标签不是现金流事件，重建对两种资金语义同样成立（会计自检）。
    """
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


def prereg(sym: str, modes: dict, in_book: dict | None) -> dict:
    """预注册判据 D1-D5 裁决（先验声明见模块 docstring）。"""
    fe, fse = modes["fusion_e"], modes["fusion_se"]
    f, ft = modes["fusion"], modes["fusion_t"]
    out: dict = {}
    single_max = max(ft["strat_pct"], fse["strat_pct"])
    out["D1_positive_interaction_decoupled"] = [
        fe["strat_pct"], single_max, fe["strat_pct"] > single_max]
    out["D2_decoupling_gain"] = [fe["strat_pct"], f["strat_pct"],
                                 fe["strat_pct"] > f["strat_pct"]]
    if sym == "OKLO":
        out["D3_oklo_no_worse"] = [fe["strat_pct"], f["strat_pct"],
                                   fe["strat_pct"] >= f["strat_pct"]]
    if in_book is not None:
        guard = {}
        for m in GUARD_MODES:
            ours, ref = modes[m]["strat_pct"], in_book["modes"][m]["strat_pct"]
            guard[m] = [ours, ref, abs(ours - ref) < 0.05]
        out["D4_coupled_arms_in_book"] = guard
    else:
        out["D4_coupled_arms_in_book"] = "在册 json 缺失——守卫不可执行"
    defer_e = fe["fusion_obs"]["n_sub_restore_defer_bars"]
    defer_f = f["fusion_obs"]["n_sub_restore_defer_bars"]
    out["D5_defer_collapse"] = [defer_e, defer_f, defer_e < defer_f]
    # 任务约束（非判据，违规即实现信号）：fusion_t 仍 > hold26
    out["constraint_fusion_t_beats_hold26"] = [
        ft["strat_pct"], modes["hold26"]["strat_pct"],
        ft["strat_pct"] > modes["hold26"]["strat_pct"]]
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
    in_book_path = DATA_DIR / f"hold26_cs_fusion_{sym}.json"
    in_book = (json.loads(in_book_path.read_text())
               if in_book_path.exists() else None)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1),
           "design": "positional_fusion.rs capital_decoupled"
                     "（049:64 义务专款 earmark，53课留白区资金语义）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} "
              f"defer={a['fusion_obs']['n_sub_restore_defer_bars']}",
              flush=True)
    out["preregistered"] = prereg(sym, out["modes"], in_book)
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
        (DATA_DIR / f"capital_decoupling_{sym}.json").write_text(
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
        (DATA_DIR / "capital_decoupling_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
