"""调研探针2：双向（反手做空）方向的磁带级测量（纯调研，零引擎接触）。

编排者方向：出场→反手做空，基准改为 Σ|各段涨跌幅|（TV 上限，在册
bidirectional_nested_fugue_research.md §5.4 望远镜恒等式的 ④净空头镜像分量）。
本探针执行在册阶段1设计（§8：因果基线 + oracle 夹逼）并扩展：

对每标的 × 每 ladder k：
1. **方向跟随 ±1 因果捕获**：按 dir_state[k]（confirmed 方向行）持仓
   +1/−1/0（None），run 端点 close 结算。报告 long_only（+1/0）、
   bidir（+1/−1）、TV⁽ᵏ⁾ = Σ|run return|（该层分段下的全变差 = 编排者
   "涨跌幅绝对值之和"基准）、BH。
   bidir − long_only = 反手做空相对平仓的全部增量（该层全覆盖口径）。
2. **因果净空头基线**（在册阶段1）：trend_state[k] ∧ dir==Down 窗口
   确认 bar 满配空头持至翻转 bar 的 P&L。
3. **oracle 峰谷界**（允许前视，显式标注非因果）：同窗口内最高价 bar →
   其后最低价 bar 的满配空头 P&L——真上界。
   夹逼判据（在册预注册）：中位 oracle 增益 < 在册最优 10% ⇒ 关轴。
4. **镜像祖先窗口空头**：dir_state[k]==Down ∧ ∃j>k trend∧Down 的
   交集窗口空头 P&L——anc 豁免（26:80 下沉）的做空镜像形态。

用法：PYTHONPATH=src .venv/bin/python analysis/_slow_bull_probe2.py [SYM ...]
输出：analysis/data_cache/_slow_bull_probe2_<SYM>.json
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

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["QQQ", "DX", "GC", "CL", "ES", "BTC"]
MAX_LADDER = 11
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}


def build_segments(n: int, dir_flips: list, trend_flips: list):
    """事件流 → 分段常值 (starts, du, dd, mu, md) 位掩码序列。
    du/dd = dir up/down；mu/md = trend∧up / trend∧down。"""
    events = ([(b, 0, l, d) for (b, l, d) in dir_flips]
              + [(b, 1, l, t) for (b, l, t) in trend_flips])
    events.sort(key=lambda e: e[0])
    dirs: list = [None] * MAX_LADDER
    tr = [False] * MAX_LADDER
    starts, du_s, dd_s, mu_s, md_s = [0], [0], [0], [0], [0]
    ptr = 0
    for b in sorted({e[0] for e in events}):
        while ptr < len(events) and events[ptr][0] == b:
            _, kind, lad, val = events[ptr]
            if kind == 0:
                dirs[lad] = val
            else:
                tr[lad] = val
            ptr += 1
        du = dd = mu = md = 0
        for k in range(MAX_LADDER):
            if dirs[k] == "up":
                du |= 1 << k
                if tr[k]:
                    mu |= 1 << k
            elif dirs[k] == "down":
                dd |= 1 << k
                if tr[k]:
                    md |= 1 << k
        if (du, dd, mu, md) != (du_s[-1], dd_s[-1], mu_s[-1], md_s[-1]):
            starts.append(b)
            du_s.append(du)
            dd_s.append(dd)
            mu_s.append(mu)
            md_s.append(md)
    return starts, du_s, dd_s, mu_s, md_s


def ladder_direction_stats(k: int, n: int, closes: list, segs) -> dict:
    """单 ladder 的 ±1 捕获 / TV / 因果净空头 / oracle / 镜像祖先空头。"""
    starts, du_s, dd_s, mu_s, md_s = segs
    log = math.log

    def run_stats(bit_of) -> tuple[float, int, list]:
        """bit_of(i)->bool 的最大连续 True 窗口：Σ ln(end/start)、窗口数、窗口表。"""
        pnl, nwin, wins = 0.0, 0, []
        cur = None
        for i, s in enumerate(starts):
            e = starts[i + 1] if i + 1 < len(starts) else n
            if bit_of(i):
                if cur is None:
                    cur = s
            else:
                if cur is not None:
                    pnl += log(closes[s] / closes[cur])
                    wins.append((cur, s))
                    nwin += 1
                    cur = None
        if cur is not None:
            pnl += log(closes[n - 1] / closes[cur])
            wins.append((cur, n - 1))
            nwin += 1
        return pnl, nwin, wins

    # ── ±1 方向跟随（dir runs）──
    long_only = bidir = tv = 0.0
    n_up = n_dn = 0
    run_dir, run_start = None, None
    for i, s in enumerate(starts):
        e = starts[i + 1] if i + 1 < len(starts) else n
        d = ("up" if du_s[i] >> k & 1 else
             "down" if dd_s[i] >> k & 1 else None)
        if d != run_dir:
            if run_dir is not None:
                rr = log(closes[s] / closes[run_start])
                tv += abs(rr)
                if run_dir == "up":
                    long_only += rr
                    bidir += rr
                    n_up += 1
                else:
                    bidir -= rr
                    n_dn += 1
            run_dir, run_start = d, s
    if run_dir is not None:
        rr = log(closes[n - 1] / closes[run_start])
        tv += abs(rr)
        if run_dir == "up":
            long_only += rr
            bidir += rr
            n_up += 1
        else:
            bidir -= rr
            n_dn += 1

    # ── 因果净空头基线：trend∧down 窗口持空 ──
    dn_run_pnl, dn_nwin, dn_wins = run_stats(lambda i: md_s[i] >> k & 1)
    causal_short = -dn_run_pnl  # 空头 P&L = −价格 log 变化

    # ── oracle 峰谷界（前视，标注非因果）──
    oracle = 0.0
    per_win = []
    for (a, b) in dn_wins:
        hi_i, hi = a, closes[a]
        for j in range(a, b + 1):
            if closes[j] > hi:
                hi, hi_i = closes[j], j
        lo = hi
        for j in range(hi_i, b + 1):
            if closes[j] < lo:
                lo = closes[j]
        g = log(hi / lo)
        oracle += g
        per_win.append(g)
    per_win.sort()

    # ── 镜像祖先窗口空头：dir_k==Down ∧ ∃j>k trend∧Down ──
    anc_pnl, anc_nwin, _ = run_stats(
        lambda i: (dd_s[i] >> k & 1) and bool(md_s[i] >> (k + 1)))
    mirror_anc_short = -anc_pnl

    return {
        "n_runs_up": n_up, "n_runs_down": n_dn,
        "long_only": round(long_only, 4),
        "bidir": round(bidir, 4),
        "short_increment": round(bidir - long_only, 4),
        "TV": round(tv, 4),
        "capture_eff_bidir": round(bidir / tv, 4) if tv else None,
        "causal_net_short": {"pnl": round(causal_short, 4), "n": dn_nwin},
        "oracle_peak_valley": {"pnl": round(oracle, 4), "n": len(per_win),
                               "p50_win": round(per_win[len(per_win) // 2], 5)
                               if per_win else None,
                               "lookahead": True},
        "mirror_anc_short": {"pnl": round(mirror_anc_short, 4),
                             "n": anc_nwin},
    }


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
    ref = json.loads((DATA_DIR / f"osc_scco_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        raise RuntimeError(f"tape_fp 漂移：{fp} ≠ {ref['tape_fp']}，停。")
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s", flush=True)

    segs = build_segments(n, dir_flips, trend_flips)
    bh = math.log(closes[-1] / closes[0])
    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_log": round(bh, 4),
           "bh_pct": round((closes[-1] / closes[0] - 1) * 100, 1),
           "ladders": {}}
    for k in range(2, 8):
        st = ladder_direction_stats(k, n, closes, segs)
        if st["n_runs_up"] + st["n_runs_down"] == 0:
            continue
        out["ladders"][LADDER_NAMES.get(k, str(k))] = st
        print(f"[{sym}][{LADDER_NAMES.get(k, str(k))}] "
              f"BH={bh:+.3f} long_only={st['long_only']:+.3f} "
              f"bidir={st['bidir']:+.3f} (空头增量 {st['short_increment']:+.3f}) "
              f"TV={st['TV']:.2f} 捕获率={st['capture_eff_bidir']} | "
              f"因果空={st['causal_net_short']['pnl']:+.3f}"
            f"/n{st['causal_net_short']['n']} "
              f"oracle={st['oracle_peak_valley']['pnl']:+.3f} "
              f"镜像anc空={st['mirror_anc_short']['pnl']:+.3f}"
              f"/n{st['mirror_anc_short']['n']}", flush=True)
    return out


def main() -> None:
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"_slow_bull_probe2_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        print(f"[{sym}] 写盘完成", flush=True)


if __name__ == "__main__":
    main()
