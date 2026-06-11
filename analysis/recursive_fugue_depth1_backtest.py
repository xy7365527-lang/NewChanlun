"""递归赋格深度1 L2 实验 — V2of（depth=0 基线）vs V2ofS1（depth=1）。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者）：递归赋格机制实现后的首个 L2 实验
═══════════════════════════════════════════════════════════════════════
设计来源：analysis/recursive_fugue_ultimate_design.md（§2.4 mini-fugue 形态、
§4 三范畴终止条件、§5.4 路径裁决——L2 先于全量架构投入，231号）。

机制（rust/src/trading/level_operating_unit.rs SubLou）：
  voice k 的 REV 腿（DownLeg）窗口内挂一个 k−1 级别子 LOU：
    开腿（先买）= k−1 confirmed Buy1 / 盘整背驰买 × k−1 存活中枢
                  × 振幅 ≥ 2×sub_friction_rt（经济终止）× c < ZG（兑现空间）
    闭腿（后卖）= k−1 confirmed Sell3（hard/pre）/ 同锚 confirmed Sell1
                  / ZG 触线 / 父腿闭合级联强闭
  预算基 = 父 REV 腿敞口（设计 §3.3）；账本 DiffSide::Long（先买后卖），
  profit 同式 (sell−buy)×shares 降 cost_basis（ShareConserving 镜像）。

预注册判据（设计报告 §5.4 + 任务原文，双侧）：
  O_sub1（递归有效）：depth1 复利 > depth0 复利（Δ > 0）
                      ∧ 子腿聚合净现金 > 0 ∧ REV 腿本体胜率不降
  O_sub0（否证）   ：depth1 复利 ≤ depth0 复利 或 子腿净现金 ≤ 0
                      → "更细短差先验为负"链延长，递归赋格当前信号质量下不可行

守卫：O0≡P5 四面对账（SlotKey 路径化重构的零侵入证明）+ 磁带指纹。
认识论等级：O0 守卫 L1；O_sub 判据 L2（OKLO 单标的真实数据，无 oracle）。

输出：analysis/data_cache/recursive_fugue_depth1.json（增量续跑）
      analysis/data_cache/recursive_fugue_depth1_tables.md（自动表格）
      analysis/recursive_fugue_depth1_experiment.md（终版报告，手工定稿）
用法：PYTHONPATH=src .venv/bin/python analysis/recursive_fugue_depth1_backtest.py
      （env：BT_SYMBOLS=OKLO  BT_FORCE=1 强制重跑变体）
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

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

DATA_DIR = ROOT / "analysis" / "data_cache"
# P0-b 加性适配（2026-06-11）：CL 秒级标的注册（parallel-array 含 timestamps_ns，
# load_ohlc 原生兼容，years=None）+ 输出路径 env 覆盖——与并行工位共用本脚本时
# 写独立结果文件，避免整文件读-改-写竞态（nohup 双实例竞态陷阱先例）。
# 默认值不变，现有数据路径行为零改动。
SYMBOL_FILES["CL1S"] = DATA_DIR / "cl_1s_databento_1y.json"
OUT_JSON = DATA_DIR / os.environ.get(
    "BT_OUT_JSON", "recursive_fugue_depth1.json")
OUT_MD = DATA_DIR / os.environ.get(
    "BT_OUT_MD", "recursive_fugue_depth1_tables.md")

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
# depth=0 基线即 V2of 本体（不另设名）；S1 = REV 窗口内 k−1 反弹腿（中枢域，
# 已否证 O_sub0 两票）；F1 = 笔级分型短差腿（38课"向下段顶卖底买"直读，
# 2026-06-11 任务：分型定位进出点，不需要中枢做域）
VARIANTS = ["V2of", "V2ofS1", "V2ofF1"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_open", "n_rev_open_osc", "n_rev_close_t5", "n_rev_close_t6",
    "n_rev_close_t7", "n_rev_zd_close", "n_rev_depth_rejects",
    "rev_osc_pairs", "rev_osc_wins", "rev_osc_net_cash",
    "n_sub_open", "n_sub_close", "n_sub_forced_close",
    "n_sub_amp_rejects", "n_sub_nocenter_rejects", "n_sub_earning_rejects",
    "sub_pairs", "sub_wins", "sub_net_cash",
)


def _leg_stats(diag: list) -> dict:
    """按腿类型聚合（含 rev_sub——_leg_stats_rust 硬编码三类，此处扩展）。"""
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


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 递归赋格深度1 实验\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # 磁带指纹守卫（并发引擎修改致混表的事故先例——fail-fast 不静默混表）
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(
            f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}——引擎/信号层"
            f"语义已变，禁止混表。")

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
        # O0≡P5 守卫：SlotKey 路径化重构后 legacy 路径零侵入的证明面
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
        rev, sub = legs["rev"], legs["rev_sub"]
        print(f"  [{name:6s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev对={rev['n_pairs']} 胜率={rev['win_rate']}%"
              f" 净={rev['net_cash']:+.0f}"
              f" | sub开={c['n_sub_open']} 对={c['sub_pairs']}"
              f" 胜率={sub['win_rate']}% 净现金={c['sub_net_cash']:+.1f}"
              f" 强闭={c['n_sub_forced_close']}"
              f" 振幅拒={c['n_sub_amp_rejects']}"
              f" 无中枢拒={c['n_sub_nocenter_rejects']} [{el:.3f}s]", flush=True)
    return out


def _verdict(r: dict, depth_variant: str = "V2ofS1") -> tuple[str, str]:
    """预注册判据求值 → (判决, 推导)。判据对 S1/F1 同式（双侧）。"""
    v0 = r["variants"]["V2of"]
    v1 = r["variants"][depth_variant]
    d = round(v1["metrics"]["total_compound"] - v0["metrics"]["total_compound"], 2)
    sub_cash = v1["counters"]["sub_net_cash"]
    wr0 = v0["leg_stats"]["rev"]["win_rate"]
    wr1 = v1["leg_stats"]["rev"]["win_rate"]
    wr_ok = (wr0 is None) or (wr1 is not None and wr1 >= wr0)
    chain = (f"Δ复利={d:+.2f}pp; 子腿净现金={sub_cash:+.1f}; "
             f"rev胜率 {wr0}%→{wr1}%")
    if d > 0 and sub_cash > 0 and wr_ok:
        return f"O_sub1[{depth_variant}]（递归有效，值得继续）", chain
    return f"O_sub0[{depth_variant}]（否证）", chain


def _write_tables(results: dict) -> None:
    L: list[str] = []
    L.append("# 递归赋格深度1 实验 — 自动表格（增量产物）\n")
    L.append("| 标的 | O0≡P5 | 变体 | 复利% | Δ(vs P5)pp | rev对 | rev胜率% | "
             "rev净 | sub开 | sub对 | sub胜率% | sub净现金 | 强闭 | 振幅拒 | 无中枢拒 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for sym, r in results.items():
        for v, p in r["variants"].items():
            rev, sub = p["leg_stats"]["rev"], p["leg_stats"]["rev_sub"]
            c = p["counters"]
            L.append(
                f"| {sym} | {r['o0_guard']} | {v} "
                f"| {p['metrics']['total_compound']:+.2f} | {p['delta_vs_p5']:+.1f} "
                f"| {rev['n_pairs']} | {rev['win_rate']} | {rev['net_cash']:+.0f} "
                f"| {c['n_sub_open']} | {c['sub_pairs']} | {sub['win_rate']} "
                f"| {c['sub_net_cash']:+.1f} | {c['n_sub_forced_close']} "
                f"| {c['n_sub_amp_rejects']} | {c['n_sub_nocenter_rejects']} |")
        for dv in VARIANTS[1:]:
            if "V2of" in r["variants"] and dv in r["variants"]:
                verdict, chain = _verdict(r, dv)
                L.append(f"\n**{sym} 判决：{verdict}** — {chain}\n")
    OUT_MD.write_text("\n".join(L) + "\n", encoding="utf-8")
    print(f"\n表格 → {OUT_MD}", flush=True)


def main() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text(encoding="utf-8"))
    for sym in SYMBOLS:
        results[sym] = process_symbol(sym, results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=1, ensure_ascii=False),
                            encoding="utf-8")
    _write_tables(results)
    for sym in SYMBOLS:
        for dv in VARIANTS[1:]:
            if all(v in results[sym]["variants"] for v in ("V2of", dv)):
                verdict, chain = _verdict(results[sym], dv)
                print(f"\n[{sym}] 判决：{verdict}\n  推导：{chain}", flush=True)


if __name__ == "__main__":
    main()
