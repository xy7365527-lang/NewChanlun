"""trend_trim_decomposition — 牛市 trim 正误分解（趋势态停削机制研究探针）。

对 hold26 与 fusion_t 的每笔削减（exit_reason=="sellpt"）配对其同层下一次
回复入场，分解：
  正确 trim = 回复价 < 削减价（卖高接低，震荡差价兑现）
  错误 trim = 回复价 > 削减价（flat_missed——卖飞后更高接回）
按 (年度 regime, 削减 bar 相位) 分桶。相位 ∈ {trend_up, b3win_up, osc_up, non_up}：
  trend_up  = trend_state[k] ∧ dir[k]==Up（fusion_t 在册停削域）
  b3win_up  = ¬trend ∧ dir==Up ∧ 三买窗口内（confirmed Buy3 后无新中枢事件——
              49课:60"在中枢第三类买点后持股直到新中枢出现"的未覆盖段，B 轴目标）
  osc_up    = ¬trend ∧ dir==Up 其余（中枢震荡，49课:50 合法差价域）
  non_up    = dir[k]≠Up（熊市削减承载面）
另测：
  - 41课门标记：trim/hold 时 dir[k+1]==Down ∧ 父层本 run 无向下背驰（未衰竭）
    ——bear rally 中停削的暴露面（G 轴目标）；
  - D 轴词汇密度：trend 相内 sell1@k 信号 bar 数（49课:54 背驰出场的触发频度）。

用法：PYTHONPATH=src .venv/bin/python analysis/trend_trim_decomposition.py [SYM...]
输出：analysis/data_cache/trend_trim_decomp_<SYM>.json
"""

from __future__ import annotations

import json
import math
import sys
import time
from bisect import bisect_right
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
MAX_LADDER = 11
REGIME_NATS = 0.10
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "CL"]
MODES = ["hold26", "fusion_t"]


def scan_tape(tape, dir_flips, trend_flips):
    """单趟扫描：事件锚集 + D 轴词汇密度（trend 相内 sell1/sell_any bar 数）。

    返回：
      b3 [lad] = [(bar, cs)] confirmed Buy3
      evcs [lad] = ([bars], [cs]) 全部带 cs 事件（新中枢检测用）
      ddiv [lad] = [bar] 向下背驰事件（41课衰竭证据）
      d_axis [lad] = {sell1_in_trend, sellany_in_trend}
    """
    b3 = [[] for _ in range(MAX_LADDER)]
    evcs_b = [[] for _ in range(MAX_LADDER)]
    evcs_c = [[] for _ in range(MAX_LADDER)]
    ddiv = [[] for _ in range(MAX_LADDER)]
    s1t = [0] * MAX_LADDER
    sat = [0] * MAX_LADDER
    dirs: list = [None] * MAX_LADDER
    trend = [False] * MAX_LADDER
    fp = tp = 0
    nf, ntf = len(dir_flips), len(trend_flips)
    for i, s in enumerate(tape):
        while fp < nf and dir_flips[fp][0] == i:
            dirs[dir_flips[fp][1]] = dir_flips[fp][2]
            fp += 1
        while tp < ntf and trend_flips[tp][0] == i:
            trend[trend_flips[tp][1]] = trend_flips[tp][2]
            tp += 1
        if s.bsp_events:
            for lad in range(MAX_LADDER):
                for e in s.bsp_events[lad]:
                    # (kind, side, seg_idx, confirmed, cs, zd, zg, price)
                    if e[4] is not None:
                        evcs_b[lad].append(i)
                        evcs_c[lad].append(e[4])
                    if e[0] == "type3" and e[1] == "buy" and e[3]:
                        b3[lad].append((i, e[4]))
        if s.div_events:
            for lad in range(MAX_LADDER):
                for d in s.div_events[lad]:
                    # (kind, direction, side, seg_idx, fa, fc, price)
                    if d[1] == "down":
                        ddiv[lad].append(i)
        for lad in range(MAX_LADDER):
            if trend[lad] and dirs[lad] == "up":
                if s.sell1[lad]:
                    s1t[lad] += 1
                if s.sell_any[lad]:
                    sat[lad] += 1
    return b3, (evcs_b, evcs_c), ddiv, {"sell1_in_trend": s1t,
                                        "sellany_in_trend": sat}


class FlipView:
    """稀疏翻转行 → (lad, bar) 点查询（bisect）。"""

    def __init__(self, flips):
        self.bars = [[] for _ in range(MAX_LADDER)]
        self.vals = [[] for _ in range(MAX_LADDER)]
        for bar, lad, v in flips:
            self.bars[lad].append(bar)
            self.vals[lad].append(v)

    def at(self, lad, bar, default=None):
        j = bisect_right(self.bars[lad], bar) - 1
        return self.vals[lad][j] if j >= 0 else default

    def flip_bar(self, lad, bar):
        """当前值生效起点 bar（41课父层 run 起点）。"""
        j = bisect_right(self.bars[lad], bar) - 1
        return self.bars[lad][j] if j >= 0 else 0


def in_b3_window(lad, bar, b3, evcs):
    """confirmed Buy3 后、首个新中枢（更大 cs 的事件）前。"""
    lst = b3[lad]
    j = bisect_right(lst, (bar, float("inf"))) - 1
    if j < 0:
        return False
    b3bar, b3cs = lst[j]
    bars, css = evcs[0][lad], evcs[1][lad]
    lo = bisect_right(bars, b3bar)
    hi = bisect_right(bars, bar)
    return not any(css[q] > b3cs for q in range(lo, hi))


def main() -> None:
    for sym in SYMBOLS:
        t0 = time.time()
        opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[sym])
        n = len(closes)
        dir_flips: list = []
        trend_flips: list = []
        tape = compute_organic_signals(opens, highs, lows, closes,
                                       dir_flips=dir_flips,
                                       trend_flips=trend_flips)
        print(f"[{sym}] 信号层 {time.time() - t0:.1f}s bars={n:,}", flush=True)

        t1 = time.time()
        b3, evcs, ddiv, d_axis = scan_tape(tape, dir_flips, trend_flips)
        print(f"[{sym}] 扫描 {time.time() - t1:.1f}s "
              f"b3={[len(x) for x in b3]} d_axis={d_axis}", flush=True)

        # 年度 regime（与 fusion 回测同口径）
        yr_log: dict[str, float] = {}
        for i in range(1, n):
            yr_log[str(years[i])] = yr_log.get(str(years[i]), 0.0) + \
                math.log(closes[i] / closes[i - 1])
        regime_of = {y: ("bull" if v >= REGIME_NATS else
                         "bear" if v <= -REGIME_NATS else "range")
                     for y, v in yr_log.items()}

        dirv = FlipView(dir_flips)
        trendv = FlipView(trend_flips)
        last_close = closes[-1]
        rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

        out = {"symbol": sym, "n_bars": n, "d_axis": d_axis,
               "regime_years": regime_of, "modes": {}}
        for mode in MODES:
            res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
            by_lad: dict[int, list] = {}
            for t in res["trades"]:
                by_lad.setdefault(t[0], []).append(t)
            buckets: dict[str, dict] = {}
            g41_exposure = {"n_trims_g41": 0, "cash_g41": 0.0}
            for lad, ts in by_lad.items():
                ts.sort(key=lambda t: t[1])  # entry_bar
                for idx, t in enumerate(ts):
                    (_, eb, ep, xb, xp, sh, w, dfr, part, reason, _pol) = t
                    if reason != "sellpt":
                        continue
                    # 下一次同层回复入场
                    re_p, re_known = (None, False)
                    if idx + 1 < len(ts):
                        re_p, re_known = ts[idx + 1][2], True
                    else:
                        re_p = last_close  # 反事实：持有到末 bar
                    d = dirv.at(lad, xb)
                    tr = trendv.at(lad, xb, False)
                    if d != "up":
                        phase = "non_up"
                    elif tr:
                        phase = "trend_up"
                    elif in_b3_window(lad, xb, b3, evcs):
                        phase = "b3win_up"
                    else:
                        phase = "osc_up"
                    # 41课门标记：父层向下且本 run 无向下背驰
                    p = lad + 1
                    g41 = False
                    if phase == "trend_up" and p < MAX_LADDER and \
                            dirv.at(p, xb) == "down":
                        run0 = dirv.flip_bar(p, xb)
                        lo = bisect_right(ddiv[p], run0)
                        hi = bisect_right(ddiv[p], xb)
                        g41 = hi == lo  # 无衰竭证据
                    reg = regime_of[str(years[xb])]
                    key = f"{reg}/{phase}"
                    b = buckets.setdefault(key, {
                        "n": 0, "n_wrong": 0, "n_open_end": 0,
                        "cash_avoided": 0.0, "cash_missed": 0.0,
                        "net_cash": 0.0})
                    b["n"] += 1
                    delta = sh * (xp - re_p)  # >0 = 卖高接低（正确）
                    b["net_cash"] += delta
                    if delta >= 0:
                        b["cash_avoided"] += delta
                    else:
                        b["n_wrong"] += 1
                        b["cash_missed"] += -delta
                    if not re_known:
                        b["n_open_end"] += 1
                    if g41:
                        g41_exposure["n_trims_g41"] += 1
                        g41_exposure["cash_g41"] += delta
            for b in buckets.values():
                for k in ("cash_avoided", "cash_missed", "net_cash"):
                    b[k] = round(b[k], 0)
            g41_exposure["cash_g41"] = round(g41_exposure["cash_g41"], 0)
            out["modes"][mode] = {
                "final_nav": round(res["final_nav"], 0),
                "buckets": dict(sorted(buckets.items())),
                "g41_marked_trims": g41_exposure,
                "n_trend_holds": list(res["n_trend_holds_by_ladder"]),
            }
            print(f"[{sym}][{mode}] nav={res['final_nav']:,.0f}", flush=True)
            for k, v in sorted(buckets.items()):
                print(f"  {k:18s} n={v['n']:6d} wrong={v['n_wrong']:6d} "
                      f"net={v['net_cash']:+14,.0f} "
                      f"avoid={v['cash_avoided']:14,.0f} "
                      f"miss={v['cash_missed']:14,.0f}", flush=True)
        (DATA_DIR / f"trend_trim_decomp_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        print(f"[{sym}] 完成 {time.time() - t0:.1f}s", flush=True)


if __name__ == "__main__":
    main()
