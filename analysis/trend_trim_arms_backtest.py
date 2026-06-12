"""趋势态停削严格化三轴回测——TrendAxisOpts {g,d,b} 消融（预注册判据 T1-T5）。

任务：研究并实装趋势态停削（hold26 L3 牛市 α 6/6 全负开放轴 1 的机制改进）。
设计：`analysis/trend_trimming_mechanism_research.md`。
实现：`rust/src/trading/positional.rs::TrendAxisOpts` + positional_fusion.rs。

七臂（同磁带）：
  hold26      在册基线守卫（逐位复现 1247.1 / 96.0 / 274.9）
  fusion_t    在册 T 轴守卫（逐位复现 4174.6 / 378.9 / 351.0——opts 全 false
              零漂移由构造保证，此臂将其升为数据守卫）
  fusion_tg   + 41课:22 衰竭门（父层 dir==Down ∧ 本 run 无向下背驰 ⇒ 趋势相
              否决——F5 bear rally 漏出修复臂）
  fusion_td   + 49课:54 背驰出场（趋势相内 sell1@k ⇒ 全抛"trend_div"）
  fusion_tb   + 49课:60 三买起点（confirmed Buy3 开窗至新中枢/向上背驰/dir翻落）
  fusion_tgb  g×b 合取（d 不入合取候选：D 轴词汇密度证据先验不利——
              sell1_in_trend 占 sellany_in_trend ~63%，见研究文档 §D）
  fusion_tgdb 49课全严格形式对照（生成史完整性）

预注册判据（先于运行声明；VLg 负交互先例 ⇒ 组合臂判据必列）：
  T1g 衰竭门修复 F5：fusion_tg bear α ≥ fusion_t bear α（BTC/CL；OKLO 熊市空集
      自动 pass）。反向风险已知：decomp g41 标记 trim 全年净额 BTC −145K
      （门重新放行的削减历史净错）——若 T1g 否证且总收益恶化，则 41课门在
      kind 直读趋势相上是错误嫁接（衰竭门的对象是"参与小级别买卖点"的开仓
      侧，不是恒仓基座的停削侧）。
  T2g 衰竭门不伤总账：fusion_tg ≥ fusion_t − 5%（绝对 pp）。
  T3d 背驰出场方向：fusion_td vs fusion_t（探索性，无先验方向——49课:54
      字面 vs 引擎 type1 词汇粒度错配的裁决）。
  T4b 三买起点牛市增益：fusion_tb bull α ≥ fusion_t bull α（decomp b3win
      桶近空 ⇒ 预期近零；若显著为负则窗口关闭词汇不完备）。
  T5  最优臂判定：max(臂) 按每标的列出——升级 positional 基座的候选输入。
  guard hold26/fusion_t 三标的逐位复现在册（±0.05pp）。

用法：PYTHONPATH=src .venv/bin/python analysis/trend_trim_arms_backtest.py [SYM...]
输出：analysis/data_cache/trend_trim_arms_<SYM>.json + _summary.json
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
MODES = ["hold26", "fusion_t", "fusion_tg", "fusion_td", "fusion_tb",
         "fusion_tgb", "fusion_tgdb"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "CL"]
REGIME_NATS = 0.10
# 在册守卫（hold26_counterseg_fusion_results.md §0）
IN_BOOK = {
    "BTC": {"hold26": 1247.1, "fusion_t": 4174.6},
    "OKLO": {"hold26": 96.0, "fusion_t": 378.9},
    "CL": {"hold26": 274.9, "fusion_t": 351.0},
}


def analyze(res: dict, closes, years) -> dict:
    """NAV 重建 + MDD + 分年/regime（与 fusion 回测逐字同口径）。"""
    n = len(closes)
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100
    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason) in trades:
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep
        dcash[xb] += sh * xp
    shares = 0.0
    pool = 100_000.0
    peak = mdd = 0.0
    prev_nav = None
    prev_expo = 0.0
    yearly: dict[str, dict] = {}
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav = pool + shares * closes[i]
        peak = max(peak, nav)
        mdd = min(mdd, nav / peak - 1.0)
        if i >= 1:
            y = yearly.setdefault(str(years[i]),
                                  {"bh_log": 0.0, "strat_log": 0.0,
                                   "exp_sum": 0.0, "bars": 0})
            y["bh_log"] += math.log(closes[i] / closes[i - 1])
            y["strat_log"] += math.log(nav / prev_nav)
            y["exp_sum"] += prev_expo
            y["bars"] += 1
        prev_nav = nav
        prev_expo = shares * closes[i] / nav if nav > 0 else 0.0
    assert abs(prev_nav - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{prev_nav} ≠ {res['final_nav']}"
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        y["bh_log"] = round(y["bh_log"], 4)
        y["strat_log"] = round(y["strat_log"], 4)

    regime_years = {"bull": [], "bear": [], "range": []}
    for yk, v in sorted(yearly.items()):
        r = ("bull" if v["bh_log"] >= REGIME_NATS else
             "bear" if v["bh_log"] <= -REGIME_NATS else "range")
        regime_years[r].append(yk)

    def agg(ys):
        b = sum(yearly[y]["bh_log"] for y in ys)
        s = sum(yearly[y]["strat_log"] for y in ys)
        return {"bh_log": round(b, 4), "strat_log": round(s, 4),
                "alpha": round(s - b, 4), "years": ys}

    reason_counts: dict[str, int] = {}
    for t in trades:
        reason_counts[t[9]] = reason_counts.get(t[9], 0) + 1
    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "yearly": yearly,
        "regime": {r: agg(ys) for r, ys in regime_years.items()},
        "n_trend_holds": sum(res["n_trend_holds_by_ladder"]),
        "n_trend_div_exits": sum(res["n_trend_div_exits_by_ladder"]),
        "n_gate41_blocks": sum(res["n_gate41_blocks_by_ladder"]),
    }


def prereg(sym: str, modes: dict) -> dict:
    t = modes["fusion_t"]
    tg, td, tb = modes["fusion_tg"], modes["fusion_td"], modes["fusion_tb"]
    out: dict = {}
    bear_t = t["regime"]["bear"]["alpha"]
    bear_tg = tg["regime"]["bear"]["alpha"]
    has_bear = bool(t["regime"]["bear"]["years"])
    out["T1g_bear_alpha_recovers"] = (
        [bear_tg, bear_t, bear_tg >= bear_t] if has_bear
        else ["bear空集", "bear空集", True])
    out["T2g_total_kept"] = [tg["strat_pct"], t["strat_pct"],
                             tg["strat_pct"] >= t["strat_pct"] - 5.0]
    out["T3d_div_exit_direction"] = [td["strat_pct"], t["strat_pct"],
                                     td["strat_pct"] > t["strat_pct"]]
    out["T4b_bull_alpha"] = [tb["regime"]["bull"]["alpha"],
                             t["regime"]["bull"]["alpha"],
                             tb["regime"]["bull"]["alpha"]
                             >= t["regime"]["bull"]["alpha"]]
    best = max(MODES, key=lambda m: modes[m]["strat_pct"])
    out["T5_best_arm"] = [best, modes[best]["strat_pct"]]
    for m in ("hold26", "fusion_t"):
        out[f"guard_{m}_in_book"] = [
            modes[m]["strat_pct"], IN_BOOK[sym][m],
            abs(modes[m]["strat_pct"] - IN_BOOK[sym][m]) < 0.05]
    return out


def run_symbol(sym: str) -> dict:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[sym])
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
           "design": "TrendAxisOpts{g=41课衰竭门, d=49课:54背驰出场, "
                     "b=49课:60三买起点}",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"holds={a['n_trend_holds']} divx={a['n_trend_div_exits']} "
              f"g41={a['n_gate41_blocks']}", flush=True)
    out["preregistered"] = prereg(sym, out["modes"])
    pre = json.dumps(out["preregistered"], ensure_ascii=False)
    print(f"[{sym}] prereg={pre}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"trend_trim_arms_{sym}.json").write_text(
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
        (DATA_DIR / "trend_trim_arms_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
