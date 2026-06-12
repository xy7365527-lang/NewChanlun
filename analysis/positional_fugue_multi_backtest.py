"""positional fugue（hold26 趋势基座）多标的回测——L3 升级。

BTC 单标的判决（positional_fugue_btc_results.md：hold26 +1247% / BH +1380%，
P3 成立）扩展到 8 标的：BTC OKLO BRN CL ES GC QQQ DX。

三方对照：hold26 / V2oa25_ht（在册 osc_scco_<sym>.json cells.V2oa25_ht）/ BH。
模式：hold26（主假设）、hold26_t1（卖点词汇消融）、cycle45（否证基线）。

regime 口径（跨标的统一，替代 BTC 线手工年份表）：
  每年 BH 对数收益 ≥ +0.10 nats = bull；≤ −0.10 = bear；其余 = range。

tape_fp 守卫：与在册 osc_scco_<sym>.json 指纹逐字段比对，漂移 ⇒ 该标的
记 failed（不可与在册结果同表比较），不中止其余标的。

用法：PYTHONPATH=src .venv/bin/python analysis/positional_fugue_multi_backtest.py
输出：analysis/data_cache/positional_fugue_multi_<SYM>.json（增量逐标的落盘）
      analysis/data_cache/positional_fugue_multi_summary.json
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
MODES = ["hold26", "hold26_t1", "cycle45"]
SYMBOLS = ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
REGIME_NATS = 0.10
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}


def _regime_split(yearly: dict) -> dict:
    """每年 BH 对数收益 ±REGIME_NATS 阈值 → bull/bear/range 聚合。"""
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

    return {r: agg(ys) for r, ys in regime_years.items()}


def analyze(res: dict, closes, years) -> dict:
    """单模式结果 → NAV 重建 + MDD + 分年 + regime（BH nats 阈值）+ 分层归因。

    years=None（数据无时间戳，如 QQQ parallel-array）⇒ yearly/regime 标记
    不可得（None），不伪造时间轴。
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
        yearly_out = yearly
        regime = _regime_split(yearly)
    by_ladder: dict = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) in trades:
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
        "yearly": yearly_out,
        "regime": regime,
        "by_ladder": {LADDER_NAMES.get(k, str(k)): v
                      for k, v in sorted(by_ladder.items())},
        "counters": {k: res[k] for k in (
            "n_entries_by_ladder", "n_exits_by_ladder", "held_bars_by_ladder",
            "n_pending_cancels_by_ladder", "n_noref_skips_by_ladder",
            "n_partial_by_ladder", "n_deferred_by_ladder")},
    }


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
    ht = ref["cells"]["V2oa25_ht"]["metrics"]["total_compound"]
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1),
           "v2oa25_ht_pct_in_book": round(ht, 1),
           "regime_rule": f"yearly bh_log >= +{REGIME_NATS} bull / "
                          f"<= -{REGIME_NATS} bear / else range",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 数据/磁带异常按标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"positional_fugue_multi_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "v2oa25_ht_pct": out["v2oa25_ht_pct_in_book"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_mdd"] = m["mdd_pct"]
            summary.append(row)
        (DATA_DIR / "positional_fugue_multi_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
