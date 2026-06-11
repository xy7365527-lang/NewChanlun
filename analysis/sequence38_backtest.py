"""Sequence38 — 38课严格形式（段间盘整背驰状态机）vs 分型近似 vs 基线。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者）：SubMode::Sequence38 实装判决（OKLO + BRN，1min）
═══════════════════════════════════════════════════════════════════════
三臂（depth=1，floor=LADDER_SEG）：
  V2of      基线（depth=0，REV 主腿无子腿）
  V2ofF1    分型近似（在册：OKLO −73.8pp / BRN +10.7pp，L3 分裂）
  V2ofSeq1  段间盘整背驰严格形式（38课:36 程式逐条：盘背卖开 /
            盘背买回 / 不跌破第一段低点×次级别结构确认 / 新下跌背驰观望出口）

预注册判据：
  - 分型模式否证的有效域声明检验：若 Seq1 在 OKLO 转正（或显著优于 F1）
    ⇒ 否证确实只及近似实现；若 Seq1 同样为负 ⇒ 否证上移至38课程式本体
    （depth=1、1min 塔底锚的有效域内）。
  - BRN 方向：F1 正票（+10.7pp）若被 Seq1 保持/放大 ⇒ 严格形式无损替代。

守卫：O0≡P5 四面对账 + 磁带指纹（在册 recursive_fugue_depth1.json fp 对照）
+ V2of/V2ofF1 与在册逐位零漂移（Sequence38 仅新增代码路径，在册行为零接触）。

运行环境（worktree 工位）：本脚本在 git worktree 中提交，数据/helper 依赖
主仓绝对路径；newchan_rust 用 worktree 构建的 wheel（解包目录置于
sys.path 首位，shadow 主 venv 安装——不 maturin develop 进共享 venv）。

用法：
  PYTHONPATH=/tmp/seq38wheel <主仓>/.venv/bin/python analysis/sequence38_backtest.py
  （env：BT_SYMBOLS=OKLO,BRN  SEQ38_WHEEL_DIR=/tmp/seq38wheel 覆盖 wheel 路径）

输出：<主仓>/analysis/data_cache/sequence38_depth1.json（增量续跑，独立文件）
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

MAIN = Path("/Users/silencehan/Projects/NewChanlun")
WHEEL_DIR = Path(os.environ.get("SEQ38_WHEEL_DIR", "/tmp/seq38wheel"))
sys.path.insert(0, str(MAIN / "src"))
sys.path.insert(0, str(MAIN / "analysis"))
sys.path.insert(0, str(WHEEL_DIR))

import newchan_rust as nr  # noqa: E402

assert str(WHEEL_DIR) in nr.__file__, (
    f"newchan_rust 未从 seq38 wheel 加载（{nr.__file__}）——V2ofSeq1 不在主 venv "
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
OUT_JSON = DATA_DIR / "sequence38_depth1.json"
REGISTERED_JSON = DATA_DIR / "recursive_fugue_depth1.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
VARIANTS = ["V2of", "V2ofF1", "V2ofSeq1"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_open", "n_rev_open_osc", "n_rev_close_t5", "n_rev_close_t6",
    "n_rev_close_t7", "n_rev_zd_close", "n_rev_depth_rejects",
    "rev_osc_pairs", "rev_osc_wins", "rev_osc_net_cash",
    "n_sub_open", "n_sub_close", "n_sub_forced_close",
    "n_sub_amp_rejects", "n_sub_nocenter_rejects", "n_sub_earning_rejects",
    "n_sub_seq_consbuy_close", "n_sub_seq_nobreak_close",
    "n_sub_seq_newdiv_close",
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
    print(f"\n{'=' * 64}\n  {symbol} — Sequence38 三臂判决\n{'=' * 64}", flush=True)
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
            f"——引擎/信号层语义已变，对照失效，禁止混表。")
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
            "counters": {k: c[k] for k in COUNTER_KEYS if k in c},
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        sub = legs["rev_sub"]
        print(f"  [{name:9s}] 复利={m['total_compound']:+10.2f}%"
              f" | sub开={c['n_sub_open']} 对={c['sub_pairs']}"
              f" 胜率={sub['win_rate']}% 净现金={c['sub_net_cash']:+.1f}"
              f" | 闭腿三岔 盘背={c.get('n_sub_seq_consbuy_close', 0)}"
              f" 不跌破={c.get('n_sub_seq_nobreak_close', 0)}"
              f" 新背驰={c.get('n_sub_seq_newdiv_close', 0)}"
              f" 强闭={c['n_sub_forced_close']} [{el:.3f}s]", flush=True)

    # 在册零漂移对照（Sequence38 仅新增代码路径 ⇒ V2of/V2ofF1 逐位一致）
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
                    f"——在册行为被 Sequence38 改动污染，违反零接触守卫。")
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
