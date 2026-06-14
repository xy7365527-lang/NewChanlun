"""调研探针：ES/QQQ 为什么输 BH——慢牛削减循环的逐笔分解（纯调研，零生产代码接触）。

任务：slow_bull_vs_bh_research。对 ES/QQQ（BH 域）+ BTC（正控制）+ DX（负控制）
在 hold26 / fusion_t 两模式下做四项分解：

1. 逐 trim 配对（同 ladder 出场→下次入场）：trim_alpha = ln(卖价/回补价)
   （正 = 低位回补成功，削减挣钱；负 = 回补价更高，削减失血）。
2. 出场后结构验证：bars_to_new_high（出场价被收盘价超越所需 bar 数）、
   max_pullback（出场→新高之间的最大回撤深度）——"回调深度"假设的直接度量。
3. 祖先趋势占比：每 ladder 的 kind==Trend ∧ dir==Up 逐 bar 占比；
   "∃ 祖先层(≥move/≥recL2) 处于趋势向上"的占比——26:80 豁免条款
   级别相对化（"日线的单边上扬"= 更高级别趋势向上）的对象域大小。
4. 反事实分桶（S1 预检）：把每笔 trim 按出场时刻"∃ 更高 ladder 趋势向上"
   分桶。若豁免桶的 trim_alpha 系统为负（ES 预期）而 DX 豁免桶近空，
   则"祖先趋势豁免"判据同时修 ES（多停削）且不伤 DX（不触发）。

用法：PYTHONPATH=src .venv/bin/python analysis/_slow_bull_probe.py [SYM ...]
输出：analysis/data_cache/_slow_bull_probe_<SYM>.json
"""

from __future__ import annotations

import bisect
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
FLOOR = LADDER_SEG  # =2 segment
MODES = ["hold26", "fusion_t"]
SYMBOLS = sys.argv[1:] or ["QQQ", "DX", "ES", "BTC"]
MAX_LADDER = 11
BLOCK = 2048  # 块跳跃扫描的块大小
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}


def _q(xs: list, p: float):
    if not xs:
        return None
    s = sorted(xs)
    return round(s[min(len(s) - 1, int(p * (len(s) - 1)))], 5)


def build_state_segments(n: int, dir_flips: list, trend_flips: list):
    """稀疏翻转行 → trend∧up bitmask 的分段常值序列 + 占比统计。

    返回 (occupancy, any_ge, query)。query(bar)->mask，二分，无单调约束。
    """
    events = ([(b, 0, l, d == "up") for (b, l, d) in dir_flips]
              + [(b, 1, l, t) for (b, l, t) in trend_flips])
    events.sort(key=lambda e: e[0])
    up = [False] * MAX_LADDER
    tr = [False] * MAX_LADDER
    starts: list[int] = [0]
    masks: list[int] = [0]
    ptr = 0
    bar_set = sorted({e[0] for e in events})
    for b in bar_set:
        while ptr < len(events) and events[ptr][0] == b:
            _, kind, lad, val = events[ptr]
            (up if kind == 0 else tr)[lad] = val
            ptr += 1
        m = 0
        for k in range(MAX_LADDER):
            if up[k] and tr[k]:
                m |= 1 << k
        if m != masks[-1]:
            starts.append(b)
            masks.append(m)
    # 占比统计（按段长加权）
    bars_tu = [0] * MAX_LADDER
    bars_any_ge = [0] * MAX_LADDER
    for i, (s, m) in enumerate(zip(starts, masks)):
        e = starts[i + 1] if i + 1 < len(starts) else n
        dur = e - s
        for k in range(MAX_LADDER):
            if m >> k & 1:
                bars_tu[k] += dur
            if m >> k:
                bars_any_ge[k] += dur
    occupancy = {LADDER_NAMES.get(k, str(k)): round(bars_tu[k] / n, 4)
                 for k in range(2, 8)}
    any_ge = {f"any_ge_{LADDER_NAMES.get(k, str(k))}":
              round(bars_any_ge[k] / n, 4) for k in range(2, 8)}

    def query(bar: int) -> int:
        idx = bisect.bisect_right(starts, bar) - 1
        return masks[idx]

    return occupancy, any_ge, query


class BlockScanner:
    """块跳跃扫描：首个 close>x 的 bar + 途中最低 close。O(blocks+2·BLOCK)/查询。"""

    def __init__(self, closes: list):
        self.c = closes
        self.n = len(closes)
        nb = (self.n + BLOCK - 1) // BLOCK
        self.bmax = [max(closes[i * BLOCK:(i + 1) * BLOCK]) for i in range(nb)]
        self.bmin = [min(closes[i * BLOCK:(i + 1) * BLOCK]) for i in range(nb)]

    def first_above(self, start: int, x: float):
        """(首个 j>start 且 c[j]>x 的 j 或 None, 途中最低 close)。"""
        c, n = self.c, self.n
        lo = x
        # 起始块内逐 bar
        b0 = start // BLOCK
        end0 = min(n, (b0 + 1) * BLOCK)
        for j in range(start + 1, end0):
            v = c[j]
            if v > x:
                return j, lo
            if v < lo:
                lo = v
        # 块跳跃
        nb = len(self.bmax)
        for bi in range(b0 + 1, nb):
            if self.bmax[bi] > x:
                base = bi * BLOCK
                for j in range(base, min(n, base + BLOCK)):
                    v = c[j]
                    if v > x:
                        return j, lo
                    if v < lo:
                        lo = v
                return None, lo  # 不可达（块max>x但逐bar未命中：浮点边界）
            if self.bmin[bi] < lo:
                lo = self.bmin[bi]
        return None, lo


def trim_pairs(trades: list, scanner: BlockScanner, query,
               years=None) -> tuple[dict, list]:
    """逐 trim 配对分解 + S1 反事实分桶。
    trades 元组：(lad, eb, ep, xb, xp, sh, w, dfr, part, reason)。
    另返回逐 trim 行 [lad, year, xb, alpha, w, mask]（判据变体离线评估）。"""
    rows: list = []
    by_lad: dict[int, list] = {}
    for t in trades:
        by_lad.setdefault(t[0], []).append(t)
    out: dict = {}
    bucket = {key: {"n": 0, "alpha_sum": 0.0, "w_alpha_sum": 0.0, "alphas": []}
              for key in ("exempt", "rest")}
    for lad, ts in sorted(by_lad.items()):
        ts.sort(key=lambda t: t[1])
        alphas, ttnh, pullbacks, censored = [], [], [], 0
        w_alpha = 0.0
        for j, t in enumerate(ts):
            xb, xp, reason = int(t[3]), t[4], t[9]
            if reason not in ("sellpt", "trend_div"):
                continue
            ep_next = None
            for t2 in ts[j + 1:]:
                if t2[1] >= xb:
                    ep_next = t2[2]
                    break
            if ep_next is None:
                continue
            a = math.log(xp / ep_next)
            alphas.append(a)
            w_alpha += t[6] * a
            hi_bar, lo = scanner.first_above(xb, xp)
            if hi_bar is None:
                censored += 1
            else:
                ttnh.append(hi_bar - xb)
            pullbacks.append(lo / xp - 1.0)
            mask = query(xb)
            rows.append([lad, years[xb] if years is not None else None,
                         xb, round(a, 6), round(t[6], 4), mask])
            exempt = bool(mask >> (lad + 1))
            b = bucket["exempt" if exempt else "rest"]
            b["n"] += 1
            b["alpha_sum"] += a
            b["w_alpha_sum"] += t[6] * a
            b["alphas"].append(a)
        if not alphas:
            continue
        out[LADDER_NAMES.get(lad, str(lad))] = {
            "n_pairs": len(alphas),
            "trim_alpha_sum": round(sum(alphas), 4),
            "trim_alpha_w_sum": round(w_alpha, 4),
            "trim_alpha_p50": _q(alphas, 0.5),
            "trim_alpha_pos_share": round(
                sum(1 for a in alphas if a > 0) / len(alphas), 3),
            "bars_to_new_high_p50": _q(ttnh, 0.5),
            "bars_to_new_high_p90": _q(ttnh, 0.9),
            "ttnh_censored": censored,
            "pullback_p50": _q(pullbacks, 0.5),
            "pullback_p90": _q(pullbacks, 0.9),
            "pullback_deeper_5pct_share": round(
                sum(1 for p in pullbacks if p < -0.05) / len(pullbacks), 3),
        }
    for b in bucket.values():
        b["alpha_p50"] = _q(b["alphas"], 0.5)
        b["pos_share"] = (round(sum(1 for a in b["alphas"] if a > 0)
                                / b["n"], 3) if b["n"] else None)
        b["alpha_sum"] = round(b["alpha_sum"], 4)
        b["w_alpha_sum"] = round(b["w_alpha_sum"], 4)
        del b["alphas"]
    out["_s1_buckets"] = bucket
    return out, rows


def run_symbol(sym: str) -> dict:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[sym])
    n = len(closes)
    print(f"[{sym}] bars={n:,}", flush=True)
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
        raise RuntimeError(f"tape_fp 漂移：{fp} ≠ {ref['tape_fp']}，停。")
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    t1 = time.time()
    occupancy, any_ge, query = build_state_segments(n, dir_flips, trend_flips)
    scanner = BlockScanner(closes)
    print(f"[{sym}] 状态序列+块索引 {time.time() - t1:.1f}s\n"
          f"  occ={occupancy}\n  any_ge={any_ge}", flush=True)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round((closes[-1] / closes[0] - 1) * 100, 1),
           "trend_up_occupancy": occupancy,
           "ancestor_trend_up_share": any_ge,
           "modes": {}}
    for mode in MODES:
        t2 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        trades = res["trades"]
        strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100
        pairs, rows = trim_pairs(trades, scanner, query, years)
        out["modes"][mode] = {
            "strat_pct": round(strat_pct, 1),
            "n_trades": len(trades),
            "trim_pairs": pairs,
            "trim_rows": rows,
        }
        print(f"[{sym}][{mode}] {time.time() - t2:.1f}s "
              f"strat={strat_pct:+.1f}% "
              f"s1={json.dumps(pairs['_s1_buckets'])}", flush=True)
    return out


def main() -> None:
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"_slow_bull_probe_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        print(f"[{sym}] 写盘完成", flush=True)


if __name__ == "__main__":
    main()
