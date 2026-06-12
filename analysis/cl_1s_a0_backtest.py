"""CL 秒级 a0 完整回测 — V2oa25_ht × 同窗 1min a0 对照 × 摩擦压力测试。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者）：秒级K线作为 a0 的回测验证
═══════════════════════════════════════════════════════════════════════
背景：a0 是递归的构成性例外——把 a0 从 1min 降到 1s，原 1min"笔"成为
更高级别结构，理论上产生更多递归深度（second_bar_feasibility.md 已证
递归塔 +1~2 层，L3）。本脚本回答剩余三问：

  Q1 同窗对照：同一物理年（2024-06-02 22:00 → 2025-05-30 21:00 UTC），
     1s a0 vs 官方 1min a0（databento 10y 切片），V2oa25_ht 全变体对比。
     在册 1min 结果（v2oa25_ht_earning_CL.json）是 10 年窗口，BH +28.2%
     vs 1s 年窗 BH −20.93%——不可同表比较（OKLO 数据基底不匹配先例）。
  Q2 摩擦：1s rev 腿均差 1.77bps/对、osc 腿 2.28bps/对，CL 1 tick≈1.4bps。
     现金级摩擦折算（非每对按全名义近似）：每个 fill 侧扣 f_side×股数×价格，
     主笔两侧 + diag 每对 diff 两侧，drag 折回该笔资本占比后复利。
  Q3 结构质量：同窗 RecursiveOrchestrator 结构计数（笔/线段/L1 moves/
     递归塔层数）——1s 是否产生有意义结构、递归深度增量多少。

摩擦档位（f_side，单侧）：0 / 0.7 / 1.4（CL 1 tick）/ 2.5 / 5 bps。
CL 期货佣金 ~$1.5/手/侧 ≈ 0.02bps（1000桶×$70）——并入滑点档不单列。

近似声明：drag 折算用 total_shares_exit 作主笔股数（ShareConserving +
earning 未触发时 == 入场股数；reached_earning 笔单独计数供审计）。

守卫：1s 磁带指纹对在册（bsp 120489 / div 83627 / flips 208509）fail-fast。
认识论等级：L2（CL 单标的单时段真实数据，双 a0 同窗对照）。

输出：analysis/data_cache/cl_1s_a0_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/cl_1s_a0_backtest.py
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "cl_1s_a0_backtest.json"

F_1S = DATA_DIR / "cl_1s_databento_1y.json"
F_1M_10Y = DATA_DIR / "cl_1m_databento_10y.json"
F_1M_WIN = DATA_DIR / "cl_1m_1y_window.json"

# 同窗边界（UTC，由 1s 数据 timestamps_ns 首尾导出）
WIN_LO = "2024-06-02 22:00:00"
WIN_HI = "2025-05-30 21:00:00"  # 上界排他（1s 末 bar 20:59:59）

VARIANTS = ["V2of", "V2oa25", "V2oa25_ht"]
FLOOR = LADDER_SEG
F_SIDE_BPS = [0.0, 0.7, 1.4, 2.5, 5.0]
FP_1S_EXPECT = {"bsp": 120489, "div": 83627, "flips": 208509}


def slice_1m_window() -> None:
    """官方 1min 10y → 同窗 1 年切片（dates 字典序窗口，落盘复用）。"""
    if F_1M_WIN.exists():
        return
    raw = json.loads(F_1M_10Y.read_text())
    dates = raw["dates"]
    idx = [i for i, d in enumerate(dates) if WIN_LO <= d[:19] < WIN_HI]
    lo, hi = idx[0], idx[-1] + 1
    out = {"symbol": raw["symbol"], "window": [WIN_LO, WIN_HI],
           "dates": dates[lo:hi]}
    for k in ("opens", "highs", "lows", "closes", "volumes"):
        out[k] = raw[k][lo:hi]
    F_1M_WIN.write_text(json.dumps(out))
    print(f"  1min 同窗切片 {hi - lo:,} bars → {F_1M_WIN.name}", flush=True)


def structure_counts(opens, highs, lows, closes) -> dict:
    """RecursiveOrchestrator 结构计数（与 _db_sec_density.py 同口径）。"""
    orch = nr.RecursiveOrchestrator(max_levels=7)
    t0 = time.time()
    for i in range(len(closes)):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    dt = time.time() - t0
    rec = orch.current_recursive()
    return {
        "n_strokes": len(orch.current_strokes()),
        "n_segments": len(orch.current_segments()),
        "n_l1_moves": len(orch.current_moves()),
        "recursive": [(lid, len(zhs), len(mvs)) for (lid, zhs, mvs) in rec],
        "engine_secs": round(dt, 1),
    }


def friction_table(diag, years) -> dict:
    """现金级摩擦折算：每档 f_side 重算逐笔 pnl 后复利 + 分腿净现金。"""
    out = {}
    n_earning = sum(1 for h, _ in diag if h[9])
    for f_bps in F_SIDE_BPS:
        f = f_bps / 1e4
        comp = 1.0
        wins = 0
        leg_net: dict[str, float] = {}
        n_fills = 0
        for header, diffs in diag:
            (_eb, ep, _xb, xp, _rs, _lad, pnl_pct, _cb, shares, _earn) = header
            cap = ep * shares
            drag = f * shares * (ep + xp)
            n_fills += 2
            for (_k, leg, _sb, sp, _bb, bp), (sh, _df, pf, *_r) in diffs:
                pair_drag = f * sh * (sp + bp)
                drag += pair_drag
                n_fills += 2
                leg_net[leg] = leg_net.get(leg, 0.0) + pf - pair_drag
            adj = pnl_pct - (drag / cap * 100 if cap > 0 else 0.0)
            comp *= 1 + adj / 100
            if adj > 0:
                wins += 1
        n = len(diag)
        out[f"{f_bps}bps"] = {
            "total_compound": round((comp - 1) * 100, 2),
            "win_rate": round(wins / n * 100, 1) if n else None,
            "leg_net_cash": {k: round(v, 1) for k, v in sorted(leg_net.items())},
        }
    out["n_fills_total"] = n_fills
    out["n_reached_earning"] = n_earning
    return out


def process_dataset(tag: str, path: Path, fp_expect: dict | None,
                    prev: dict | None) -> dict:
    print(f"\n{'=' * 64}\n  {tag} — {path.name}\n{'=' * 64}", flush=True)
    if prev is not None and all(v in prev.get("variants", {}) for v in VARIANTS):
        print("  [增量] 在册结果完整，跳过", flush=True)
        return prev
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    struct = structure_counts(opens, highs, lows, closes)
    print(f"  结构: 笔={struct['n_strokes']:,} 段={struct['n_segments']:,}"
          f" L1mv={struct['n_l1_moves']:,} 递归={struct['recursive']}"
          f" [{struct['engine_secs']}s]", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    print(f"  信号层 {time.time() - t0:.1f}s", flush=True)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if fp_expect is not None and fp != fp_expect:
        raise SystemExit(f"磁带指纹不匹配：期望 {fp_expect} vs 本次 {fp}")
    rtape = pack_tape(tape, dir_flips=dir_flips)

    trades_p5, _ = run_version_i(tape, floor_ladder=FLOOR,
                                 pairing=PAIRING_VARIANTS["P5"])
    m_p5 = extended_metrics(trades_p5, years)
    print(f"  P5 复利 {m_p5['total_compound']:+.2f}%  {m_p5['n']} 笔", flush=True)

    out = {"n_bars": n, "bh": round(bh, 2), "tape_fp": fp,
           "structure": struct,
           "P5": {k: round(m_p5[k], 4) for k in
                  ("n", "win_rate", "total_compound", "sharpe", "max_dd")},
           "variants": {}}
    for name in VARIANTS:
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        from organic_fugue_rust_backtest import _trades_from_rust
        m = extended_metrics(_trades_from_rust(res["trades"]), years)
        fr = friction_table(res["diag"], years)
        out["variants"][name] = {
            "metrics": {k: round(m[k], 4) for k in
                        ("n", "win_rate", "total_compound", "sharpe", "max_dd")},
            "friction": fr,
        }
        f0 = fr["0.0bps"]["total_compound"]
        print(f"  [{name:10s}] 零摩擦={f0:+8.2f}%  "
              + "  ".join(f"{b}→{fr[f'{b}bps']['total_compound']:+.2f}%"
                          for b in (0.7, 1.4, 2.5, 5.0))
              + f"  fills={fr['n_fills_total']:,}", flush=True)
        if abs(f0 - m["total_compound"]) > 0.05:
            print(f"    [警告] 零摩擦复利 {f0} ≠ metrics {m['total_compound']:.2f}"
                  f"（diag/trades 口径漂移）", flush=True)
    return out


def main() -> None:
    slice_1m_window()
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text(encoding="utf-8"))
    for tag, path, fpx in (("CL_1S", F_1S, FP_1S_EXPECT),
                           ("CL_1M_WIN", F_1M_WIN, None)):
        results[tag] = process_dataset(tag, path, fpx, results.get(tag))
        OUT_JSON.write_text(json.dumps(results, indent=1, ensure_ascii=False),
                            encoding="utf-8")
    print(f"\n结果 → {OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
