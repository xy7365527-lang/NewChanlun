"""P0-a — BRN 分型模式 depth=2 回测（V2ofF2 vs 在册 V2ofF1 基线）。

预注册判据（任务原文）：depth2 > depth1 且 Δ > 0（相对 depth=1 的
+10.7pp 基线，即 V2ofF1−V2of=+10.71pp）。

守卫：
1. 磁带指纹必须与在册 recursive_fugue_depth1.json 的 BRN 条目逐字相等
   （{'bsp': 63977, 'div': 46751, 'flips': 117573}）——引擎语义漂移即 fail-fast；
2. V2of / V2ofF1 复利必须复现在册值（重建共享库后的回归证明，同一
   build 下两深度才可比）。

depth 分解：diag diff key 编码 py_key = home + 200 + 100×depth
（rev=2xx, depth1 子腿=3xx, depth2 子腿=4xx）。

用法：PYTHONPATH=src .venv/bin/python analysis/_p0p1/p0a_brn_depth2_backtest.py
认识论等级：L2（BRN 单标的真实数据）。
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import extended_metrics, LADDER_SEG  # noqa: E402
from organic_fugue_rust_backtest import _trades_from_rust  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

REG_JSON = ROOT / "analysis" / "data_cache" / "recursive_fugue_depth1.json"
OUT_JSON = ROOT / "analysis" / "_p0p1" / "p0a_brn_depth2.json"
SYMBOL = "BRN"
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_open", "n_sub_open", "n_sub_close", "n_sub_forced_close",
    "n_sub_earning_rejects", "sub_pairs", "sub_wins", "sub_net_cash",
)


def leg_stats_by_depth(diag: list) -> dict:
    """按 (leg, depth) 聚合——key//100: 2=rev(d0), 3=sub d1, 4=sub d2。"""
    buckets: dict[str, list] = {}
    for _header, diffs in diag:
        for (key, leg, _sb, sp, _bb, _bp), (_sh, df, pf, *_rest) in diffs:
            name = leg
            if leg == "rev_sub":
                name = f"rev_sub_d{key // 100 - 2}"
            buckets.setdefault(name, []).append((sp, df, pf))
    out = {}
    for name, rows in sorted(buckets.items()):
        n = len(rows)
        out[name] = {
            "n_pairs": n,
            "win_rate": round(sum(1 for _s, df, _p in rows if df > 0) / n * 100, 1),
            "avg_diff_pct": round(sum(df / sp for sp, df, _p in rows) / n * 100, 4),
            "net_cash": round(sum(pf for _s, _d, pf in rows), 1),
        }
    return out


def main() -> None:
    reg = json.loads(REG_JSON.read_text(encoding="utf-8"))[SYMBOL]
    reg_fp = reg["tape_fp"]
    reg_v = {v: reg["variants"][v]["metrics"]["total_compound"]
             for v in ("V2of", "V2ofF1")}

    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[SYMBOL])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"{SYMBOL}: bars={n:,} BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    print(f"信号层 {time.time() - t0:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if fp != reg_fp:
        raise SystemExit(f"磁带指纹不匹配：在册 {reg_fp} vs 本次 {fp}——"
                         f"引擎/信号层语义已变，禁止混表。")
    print(f"磁带指纹守卫 PASS {fp}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips)

    out: dict = {"symbol": SYMBOL, "n_bars": n, "bh": round(bh, 2),
                 "tape_fp": fp, "variants": {}}
    for name in ("V2of", "V2ofF1", "V2ofF2"):
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = leg_stats_by_depth(res["diag"])
        c = res["counters"]
        comp = m["total_compound"]
        # 复现守卫：在册变体必须逐位复现（同 build 可比性证明）
        if name in reg_v:
            if abs(comp - reg_v[name]) > 1e-9:
                raise SystemExit(f"{name} 复现失败：在册 {reg_v[name]} vs "
                                 f"本次 {comp}——重建后语义漂移，禁止比较。")
            print(f"[{name}] 复现守卫 PASS", flush=True)
        out["variants"][name] = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "leg_stats": legs,
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "elapsed_s": round(el, 3),
        }
        print(f"[{name:7s}] 复利={comp:+10.2f}%  sub开={c['n_sub_open']}"
              f" 对={c['sub_pairs']} 胜={c['sub_wins']}"
              f" 净现金={c['sub_net_cash']:+.1f} 强闭={c['n_sub_forced_close']}"
              f" [{el:.3f}s]", flush=True)
        for ln, st in legs.items():
            print(f"    {ln:11s} 对={st['n_pairs']:4d} 胜率={st['win_rate']:5.1f}%"
                  f" 均差={st['avg_diff_pct']:+.4f}% 净={st['net_cash']:+.1f}",
                  flush=True)

    v0 = out["variants"]["V2of"]["metrics"]["total_compound"]
    v1 = out["variants"]["V2ofF1"]["metrics"]["total_compound"]
    v2 = out["variants"]["V2ofF2"]["metrics"]["total_compound"]
    d1, d21 = round(v1 - v0, 2), round(v2 - v1, 2)
    out["verdict"] = {
        "delta_d1_vs_d0_pp": d1,
        "delta_d2_vs_d1_pp": d21,
        "delta_d2_vs_d0_pp": round(v2 - v0, 2),
        "preregistered_pass": bool(v2 > v1 and d21 > 0),
    }
    OUT_JSON.write_text(json.dumps(out, indent=1, ensure_ascii=False),
                        encoding="utf-8")
    print(f"\nΔ(d1−d0)={d1:+.2f}pp  Δ(d2−d1)={d21:+.2f}pp  "
          f"预注册判据(depth2>depth1)："
          f"{'通过' if out['verdict']['preregistered_pass'] else '不通过'}",
          flush=True)
    print(f"JSON → {OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
