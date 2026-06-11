"""P1 双门四格消融 — Fractal 子腿的 35课成本门 + 41课门（OKLO + BRN，1min）。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者，P1 工位）：分型模式（SubMode::Fractal）双门消融
═══════════════════════════════════════════════════════════════════════
四格（基线 = V2ofF1 裸分型子腿，recursive_fugue_depth1_backtest.py 在册）：
  V2ofF1    裸（双门关）
  V2ofF1c   仅成本门（35课：θ_eff = max(θ_q, k×friction_rt)，k=2/friction=10bps；
            该层中枢振幅因果滚动中位数不够覆盖 2 倍往返成本 ⇒ 级别自动关闭）
  V2ofF1g   仅41课门（父级别相邻同向段创新低 ∧ 无盘整背驰 = 趋势未完 = 拒开）
  V2ofF1cg  双门

预注册判据（任务原文）：
  - 成本门应自动关闭振幅/成本比 < 2 的级别（与 P0-c 比值表互证）；
  - 41课门若恒开或恒关（触发率 ≈0% 或 ≈100%）则判死门，如实报告。

守卫：O0≡P5 四面对账 + 磁带指纹（在册 fp 对照）+ 裸格与在册
recursive_fugue_depth1.json 的 V2ofF1 逐位对照（默认关 = 行为零漂移）。

运行环境（worktree 工位）：本脚本在 git worktree 中提交，数据/helper 依赖
主仓绝对路径；newchan_rust 用 worktree 构建的 wheel（解包目录置于
sys.path 首位，shadow 主 venv 安装——不 maturin develop 进共享 venv）。

用法：
  PYTHONPATH=/tmp/p1wheel <主仓>/.venv/bin/python analysis/p1_dual_gate_backtest.py
  （env：BT_SYMBOLS=OKLO,BRN  P1_WHEEL_DIR=/tmp/p1wheel 覆盖 wheel 路径）

输出：<主仓>/analysis/data_cache/p1_dual_gate.json（增量续跑，独立文件
      ——不写 recursive_fugue_depth1.json，避免与并行工位竞态）
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

MAIN = Path("/Users/silencehan/Projects/NewChanlun")
WHEEL_DIR = Path(os.environ.get("P1_WHEEL_DIR", "/tmp/p1wheel"))
sys.path.insert(0, str(MAIN / "src"))
sys.path.insert(0, str(MAIN / "analysis"))
sys.path.insert(0, str(WHEEL_DIR))

import newchan_rust as nr  # noqa: E402

assert str(WHEEL_DIR) in nr.__file__, (
    f"newchan_rust 未从 P1 wheel 加载（{nr.__file__}）——双门变体不在主 venv "
    f"安装版中，混用会 fail-fast 或静默错配")

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue_rust_backtest import (  # noqa: E402
    _bitexact_o0,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = MAIN / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "p1_dual_gate.json"
REGISTERED_JSON = DATA_DIR / "recursive_fugue_depth1.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
VARIANTS = ["V2of", "V2ofF1", "V2ofF1c", "V2ofF1g", "V2ofF1cg"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_open", "n_rev_open_osc", "n_rev_close_t5", "n_rev_close_t6",
    "n_rev_close_t7", "n_rev_zd_close", "n_rev_depth_rejects",
    "rev_osc_pairs", "rev_osc_wins", "rev_osc_net_cash",
    "n_sub_open", "n_sub_close", "n_sub_forced_close",
    "n_sub_amp_rejects", "n_sub_nocenter_rejects", "n_sub_earning_rejects",
    "n_sub_cost_rejects", "n_sub_cost_noref_rejects", "n_sub_l41_rejects",
    "sub_pairs", "sub_wins", "sub_net_cash",
)


def _leg_stats(diag: list) -> dict:
    by_leg: dict[str, list] = {"main": [], "osc": [], "rev": [], "rev_sub": []}
    for _header, diffs in diag:
        for (_key, leg, _sb, sp, _bb, _bp), (_sh, df, pf, *_rest) in diffs:
            by_leg[leg].append((sp, df, pf))
    out = {}
    for leg, rows in by_leg.items():
        n = len(rows)
        if n == 0:
            out[leg] = {"n_pairs": 0, "win_rate": None, "avg_diff_pct": None,
                        "net_cash": 0.0}
            continue
        out[leg] = {
            "n_pairs": n,
            "win_rate": round(sum(1 for _sp, df, _pf in rows if df > 0) / n * 100, 1),
            "avg_diff_pct": round(sum(df / sp for sp, df, _pf in rows) / n * 100, 4),
            "net_cash": round(sum(pf for _sp, _df, pf in rows), 1),
        }
    return out


def process_symbol(symbol: str, prev: dict | None, registered: dict) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — P1 双门四格消融\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # 磁带指纹守卫（在册 depth1 fp + 本文件历史 fp 双对照）
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    reg_fp = registered.get(symbol, {}).get("tape_fp")
    if reg_fp is not None and reg_fp != fp:
        raise SystemExit(
            f"磁带指纹与在册 recursive_fugue_depth1.json 不匹配：{reg_fp} vs {fp}"
            f"——引擎/信号层语义已变，裸格对照失效，禁止混表。")
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}")

    rtape = pack_tape(tape, dir_flips=dir_flips)

    if prev is not None and "P5" in prev and prev.get("o0_guard") == "PASS":
        out = prev
        p5c = out["P5"]["metrics"]["total_compound"]
        print(f"  [增量] 复用在册 P5({p5c:+.2f}%) / O0 守卫", flush=True)
    else:
        trades_p5, _ = run_version_i(
            tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"])
        m_p5 = extended_metrics(trades_p5, years)
        p5c = m_p5["total_compound"]
        print(f"  P5 复利 {p5c:+.2f}%", flush=True)
        o0 = _bitexact_o0(tape, rtape, years)
        print(f"  [O0≡P5] PASS  {o0['n_trades']} 笔 / {o0['n_trace_rows']} trace 行",
              flush=True)
        out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
               "n_dir_flips": len(dir_flips), "tape_fp": fp, "o0_guard": "PASS",
               "P5": {"metrics": {k: m_p5[k] for k in
                                  ("n", "win_rate", "total_compound",
                                   "sharpe", "max_dd")}},
               "variants": {}}

    todo = [v for v in VARIANTS if v not in out["variants"]
            or os.environ.get("BT_FORCE", "0") == "1"]
    for name in todo:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = _leg_stats(res["diag"])
        c = res["counters"]
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        sub = legs["rev_sub"]
        print(f"  [{name:9s}] 复利={m['total_compound']:+10.2f}%"
              f" | sub开={c['n_sub_open']} 对={c['sub_pairs']}"
              f" 胜率={sub['win_rate']}% 净现金={c['sub_net_cash']:+.1f}"
              f" | 成本拒={c['n_sub_cost_rejects']}"
              f" 无参照拒={c['n_sub_cost_noref_rejects']}"
              f" 41课拒={c['n_sub_l41_rejects']} [{el:.3f}s]", flush=True)

    # 裸格零漂移对照（默认关 ⇒ 与在册 V2ofF1 逐位一致）
    reg_v = registered.get(symbol, {}).get("variants", {})
    for v in ("V2of", "V2ofF1"):
        if v in reg_v and v in out["variants"]:
            a = out["variants"][v]["metrics"]["total_compound"]
            b = reg_v[v]["metrics"]["total_compound"]
            tag = "PASS" if a == b else "FAIL"
            print(f"  [零漂移对照 {v}] {tag}  本次 {a} vs 在册 {b}", flush=True)
            if a != b:
                raise SystemExit(
                    f"{symbol} {v} 与在册 recursive_fugue_depth1.json 漂移"
                    f"——默认关行为被双门改动污染，违反硬约束1。")
    return out


def main() -> None:
    registered: dict = {}
    if REGISTERED_JSON.exists():
        registered = json.loads(REGISTERED_JSON.read_text(encoding="utf-8"))
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text(encoding="utf-8"))
    for sym in SYMBOLS:
        results[sym] = process_symbol(sym, results.get(sym), registered)
        OUT_JSON.write_text(json.dumps(results, indent=1, ensure_ascii=False),
                            encoding="utf-8")
    print(f"\nJSON → {OUT_JSON}", flush=True)


if __name__ == "__main__":
    main()
