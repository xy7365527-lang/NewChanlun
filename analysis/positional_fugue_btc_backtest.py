"""多级别仓位分层（positional fugue）BTC 全历史回测——三模式对照。

设计：analysis/positional_fugue_design.md（D1-D9 + 预注册判据 P1-P3）。
模式：
  hold26    26课恒仓（任意卖点削减/任意买点回复，振荡围绕满仓）——主假设
  hold26_t1 卖点词汇收窄为 type1（VLs1 先例消融）
  cycle45   v1 每层独立45课循环（振荡围绕空仓）——已否证基线（P1 ✗ −3.32）
对照：BH +1380% / V2oa25_ht_scco +630%（在册，_btc_vs_bh_probe.json 逐年表）。

预注册判据（设计文档 §5）：
  P1 牛市年（17/19/20/21/23/24）合计 策略−BH > −0.85 nats（scco −1.706 砍半）
  P2 熊市年（18/22/26）合计 策略−BH ≥ +0.465 nats（scco +0.930 的 ≥50%）
  P3 总收益 > +630%

用法：PYTHONPATH=src .venv/bin/python analysis/positional_fugue_btc_backtest.py
输出：analysis/data_cache/positional_fugue_BTC.json
"""

from __future__ import annotations

import json
import math
import os
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
MODES = ["hold26", "hold26_t1", "cycle45"]
BULL_YEARS = {"2017", "2019", "2020", "2021", "2023", "2024"}
BEAR_YEARS = {"2018", "2022", "2026"}
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}


def analyze(res: dict, closes, years, windows) -> dict:
    """单模式结果 → NAV 重建 + 分年/regime/判据/gap 穿越/分层归因。"""
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
    nav = [0.0] * n
    shares = 0.0
    pool = 100_000.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav[i] = pool + shares * closes[i]
    assert abs(nav[-1] - res["final_nav"]) < 1e-3, \
        f"NAV 重建漂移：{nav[-1]} ≠ {res['final_nav']}"
    peak = mdd = 0.0
    for v in nav:
        peak = max(peak, v)
        mdd = min(mdd, v / peak - 1.0)

    yearly: dict[str, dict] = {}
    shares = 0.0
    pool = 100_000.0
    prev_expo = 0.0
    in_window_expo: dict[int, list] = {wi: [] for wi in range(len(windows))}
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        expo = shares * closes[i] / nav[i] if nav[i] > 0 else 0.0
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
        for wi, (a, b, _, _) in enumerate(windows):
            if a <= i <= b:
                in_window_expo[wi].append(expo)
        prev_expo = expo
    for y in yearly.values():
        y["exposure"] = round(y["exp_sum"] / max(1, y["bars"]), 4)
        del y["exp_sum"]
        for k in ("bh_log", "strat_log", "flat_missed"):
            y[k] = round(y[k], 4)

    def agg(year_set):
        b = sum(v["bh_log"] for k, v in yearly.items() if k in year_set)
        s = sum(v["strat_log"] for k, v in yearly.items() if k in year_set)
        return round(b, 4), round(s, 4), round(s - b, 4)

    bull = agg(BULL_YEARS)
    bear = agg(BEAR_YEARS)
    range_ = agg(set(yearly) - BULL_YEARS - BEAR_YEARS)

    gap_expo = []
    for wi, (a, b, lr, yr) in enumerate(windows):
        es = in_window_expo[wi]
        gap_expo.append({"exit_bar": a, "year": yr, "scco_missed_log": lr,
                         "pf_mean_exposure":
                             round(sum(es) / max(1, len(es)), 4)})

    by_ladder: dict = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason) in trades:
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

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "yearly": yearly,
        "regime": {"bull": bull, "bear": bear, "range": range_},
        "preregistered": {
            "P1_bull_deficit_gt_-0.85": [bull[2], bull[2] > -0.85],
            "P2_bear_alpha_ge_0.465": [bear[2], bear[2] >= 0.465],
            "P3_total_gt_630pct": [round(strat_pct, 1), strat_pct > 630.0],
        },
        "top20_gap_traversal": gap_expo,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
        "counters": {k: res[k] for k in (
            "n_entries_by_ladder", "n_exits_by_ladder", "held_bars_by_ladder",
            "n_pending_cancels_by_ladder", "n_noref_skips_by_ladder",
            "n_partial_by_ladder", "n_deferred_by_ladder")},
    }


def main() -> None:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["BTC"])
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    print(f"bars={n:,} BH={bh_pct:+.1f}%", flush=True)

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
    print(f"信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)
    smoke = int(os.environ.get("BT_MAX_BARS", "0")) > 0
    if not smoke:
        ref_fp = json.loads(
            (DATA_DIR / "osc_scco_BTC.json").read_text())["tape_fp"]
        if ref_fp != fp:
            raise RuntimeError(f"tape_fp 漂移：{fp} ≠ 在册 {ref_fp}，停。")
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    probe = json.loads((DATA_DIR / "_btc_vs_bh_probe.json").read_text()) \
        if not smoke else {"top20_missed_gaps": [], "strat_pct": None}
    windows = [(g["exit_bar"], g["reentry_bar"], g["gap_logret"], g["year"])
               for g in sorted(probe["top20_missed_gaps"],
                               key=lambda g: -g["gap_logret"])]

    out = {"design": "positional_fugue_design.md", "n_bars": n,
           "tape_fp": fp, "bh_pct": round(bh_pct, 1),
           "scco_pct_in_book": probe["strat_pct"], "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years, windows)
        out["modes"][mode] = a
        print(f"[{mode}] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
              f"trades={a['n_trades']} regime={a['regime']} "
              f"prereg={a['preregistered']}", flush=True)

    (DATA_DIR / "positional_fugue_BTC.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1))
    for mode in MODES:
        print(f"\n== {mode} by_ladder ==",
              json.dumps(out["modes"][mode]["by_ladder"], ensure_ascii=False),
              flush=True)


if __name__ == "__main__":
    main()
