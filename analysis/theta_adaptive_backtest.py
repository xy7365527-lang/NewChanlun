"""θ 自适应深度门回测 — V2oa25/V2oa50 vs V2of（2026-06-11 编排者任务）。

═══════════════════════════════════════════════════════════════════════
任务背景
═══════════════════════════════════════════════════════════════════════
固定 θ=1% 在 OKLO/BRN（中枢振幅大）工作，QQQ 的 1 分钟中枢振幅几乎全 <1%
⇒ 1065 对震荡型配对被门后只剩 3 对——量级失配，不是策略失败。

三方案严格性裁定（设计章节见 analysis/theta_adaptive_results.md §2）：
  方案A（锚自身振幅的分位数）：自指退化——单个数无分位数；
  方案C（标的级全样本固定分位数）：回测期内前瞻（用未来中枢定过去门槛），
        需独立样本外期才严格——本任务三标的全样本回测下不可用；
  方案B（因果滚动分位数）= 最严格：θ_t(k) = 该层最近 window=50 个中枢
        相对振幅 (ZG−ZD)/c 的 q 分位（nearest-rank，排除当前锚自身），
        样本 < min_obs=10 回退固定 θ=1%（warm-up 不静默放行）。
        q ∈ {0.25, 0.50} 预注册，零标的级拟合。

对比矩阵（每标的）：
  1. O0≡P5 四面对账（新二进制下重跑——确认 DepthRef 零侵入 legacy 路径）
  2. V2of 重跑 ≡ 在册 enginefix V2of（Fixed 模式逐位不变的第二道守卫）
  3. V2oa25 / V2oa50：自适应 θ（B 方案两档分位数）
  4. 锚中枢相对振幅分布（center_amp_log，市场性质——调研任务1）

预注册判据：
  (a) QQQ 震荡型配对从 3 对恢复到百对量级？payoff（净现金/胜率/Δ vs P5）如何？
  (b) OKLO/BRN 在自适应 θ 下是否保住固定 θ 的改善（V2of Δ +410.4 / +19.8pp）？

认识论等级：O0/V2of 守卫 L1（管线等价）；V2oa 结论 L3（三标的真实数据）。

输出：analysis/data_cache/theta_adaptive_backtest.json（增量续跑）
      analysis/data_cache/theta_adaptive_tables.md（自动表格）
用法：PYTHONPATH=src .venv/bin/python analysis/theta_adaptive_backtest.py
      （env：BT_SYMBOLS=OKLO,QQQ,BRN  BT_FORCE=1 强制重跑）
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
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue_rust_backtest import (  # noqa: E402
    _bitexact_o0,
    _leg_stats_rust,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
# BT_OUT_SUFFIX：多进程并行跑不同标的时隔离输出文件（避免 read-modify-write 竞态）
_SUF = os.environ.get("BT_OUT_SUFFIX", "")
OUT_JSON = DATA_DIR / f"theta_adaptive_backtest{_SUF}.json"
OUT_MD = DATA_DIR / f"theta_adaptive_tables{_SUF}.md"
# 在册锚：C段修复后引擎的 V2of 数字（Fixed 模式逐位不变守卫的参照）。
REF_JSON = DATA_DIR / "rev_v2_paired_backtest_enginefix.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
VARIANTS = ["V2of", "V2oa25", "V2oa50"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_attempts", "n_rev_frozen_rejects", "n_rev_nocenter_rejects",
    "n_rev_depth_rejects", "n_rev_theta_fallbacks", "n_rev_open",
    "n_rev_open_osc", "n_rev_open_esc", "n_rev_close_t5", "n_rev_close_t6",
    "n_rev_close_t7", "n_rev_zd_close", "n_rev_struct_close",
    "n_rev_mismatch_holds", "rev_osc_pairs", "rev_osc_wins",
    "rev_osc_net_cash", "rev_esc_pairs", "rev_esc_wins", "rev_esc_net_cash",
)


def _pctile(sorted_vals: list[float], q: float) -> float:
    """nearest-rank 分位数（与 Rust DepthRef 同定义）。"""
    n = len(sorted_vals)
    rank = max(1, min(n, math.ceil(q * n)))
    return sorted_vals[rank - 1]


def _amp_stats(amp_log: list) -> dict:
    """锚中枢相对振幅分布（每中枢一行最终观测值；市场性质，变体无关）。"""
    by_ladder: dict[int, list[float]] = {}
    for lad, _cs, rel in amp_log:
        by_ladder.setdefault(int(lad), []).append(float(rel))
    out: dict = {}
    pooled: list[float] = []
    for lad in sorted(by_ladder):
        vals = sorted(by_ladder[lad])
        pooled.extend(vals)
        out[str(lad)] = {
            "n": len(vals),
            "pct": {f"P{int(q*100)}": round(_pctile(vals, q) * 100, 4)
                    for q in (0.10, 0.25, 0.50, 0.75, 0.90)},
            "share_below_1pct": round(
                sum(v < 0.01 for v in vals) / len(vals) * 100, 1),
        }
    pooled.sort()
    out["pooled"] = {
        "n": len(pooled),
        "pct": {f"P{int(q*100)}": round(_pctile(pooled, q) * 100, 4)
                for q in (0.10, 0.25, 0.50, 0.75, 0.90)},
        "share_below_1pct": round(
            sum(v < 0.01 for v in pooled) / len(pooled) * 100, 1)
        if pooled else None,
    }
    return out


def _kind_stats(c: dict) -> dict:
    out = {}
    for kind in ("osc", "esc"):
        n = c[f"rev_{kind}_pairs"]
        out[kind] = {
            "n_pairs": n,
            "win_rate": round(c[f"rev_{kind}_wins"] / n * 100, 1) if n else None,
            "net_cash": round(c[f"rev_{kind}_net_cash"], 1),
        }
    return out


def process_symbol(symbol: str, ref: dict | None, prev: dict | None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — θ 自适应深度门回测\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s", flush=True)

    # ── 磁带指纹守卫（同 rev_v2_paired_backtest：禁止混表）──
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(
            f"磁带指纹不匹配（prev）：在册 {prev['tape_fp']} vs 本次 {fp}"
            f"——引擎/信号层语义已变，禁止混表。")
    if ref is not None and "tape_fp" in ref and ref["tape_fp"] != fp:
        # 在册 enginefix 磁带与当前数据文件尾部漂移（BRN：bars 2,418,059→
        # 2,418,058，尾部少 1 bsp/1 div/2 flips——数据尾截断伪影，非引擎语义
        # 变化）。处理：放弃 ref 复用（P5 当前磁带重算 + V2of 无在册锚），
        # 本标的与在册 enginefix 数字不可逐位对比——报告须声明。
        print(f"  [警告] ref(enginefix) 指纹不匹配：在册 {ref['tape_fp']} vs "
              f"本次 {fp}——放弃 ref 复用，P5/V2of 在当前磁带上自洽重算。",
              flush=True)
        ref = None

    rtape = pack_tape(tape, dir_flips=dir_flips)

    if prev is not None and "P5" in prev and prev.get("o0_guard") == "PASS":
        out = prev
        p5c = out["P5"]["metrics"]["total_compound"]
        print(f"  [增量] 复用本表在册 P5({p5c:+.2f}%) / O0 守卫", flush=True)
    else:
        # ── O0≡P5 四面对账（新二进制：DepthRef 零侵入 legacy 路径的守卫）──
        o0 = _bitexact_o0(tape, rtape, years)
        print(f"  [O0≡P5] PASS  {o0['n_trades']} 笔 / {o0['n_trace_rows']} trace 行",
              flush=True)
        # P5 基线（Δ 分母）：fp 已对齐 ref ⇒ 直接复用在册 P5（同磁带确定性重算）
        if ref is not None and "P5" in ref:
            m_p5 = dict(ref["P5"]["metrics"])
            print(f"  [复用 ref] P5 复利 {m_p5['total_compound']:+.2f}%", flush=True)
        else:
            trades_p5, _ = run_version_i(
                tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"])
            mm = extended_metrics(trades_p5, years)
            m_p5 = {k: mm[k] for k in ("n", "win_rate", "total_compound",
                                       "sharpe", "max_dd")}
        p5c = m_p5["total_compound"]
        out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
               "tape_fp": fp, "o0_guard": "PASS",
               "P5": {"metrics": m_p5}, "O0": o0, "variants": {}}

    todo = [v for v in VARIANTS if v not in out["variants"]
            or os.environ.get("BT_FORCE", "0") == "1"]
    for name in todo:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = _leg_stats_rust(res["diag"])
        c = res["counters"]
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "kind_stats": _kind_stats(c),
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack

        # 振幅分布调研（市场性质，从首个变体的 diag 提取一次）
        if "amp_dist" not in out and res.get("center_amp_log"):
            out["amp_dist"] = _amp_stats(res["center_amp_log"])
            pooled = out["amp_dist"]["pooled"]
            print(f"  [振幅分布] n={pooled['n']} P50={pooled['pct']['P50']:.3f}%"
                  f" <1% 占比={pooled['share_below_1pct']}%", flush=True)

        # V2of ≡ 在册 enginefix（Fixed 模式逐位不变的第二道守卫）
        if name == "V2of" and ref is not None and "V2of" in ref.get("variants", {}):
            ref_c = ref["variants"]["V2of"]["metrics"]["total_compound"]
            got = m["total_compound"]
            if abs(got - ref_c) > 1e-9:
                raise SystemExit(
                    f"FAIL V2of 漂移：在册 {ref_c} vs 本次 {got}——Fixed 模式"
                    f"被 DepthRef 改动污染，禁止继续。")
            print("  [V2of≡在册] PASS", flush=True)

        rev = legs["rev"]
        print(f"  [{name:7s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:5d} rev胜率={rev['win_rate']}%"
              f" 净现金={rev['net_cash']:+.0f}"
              f" 深度拒={c['n_rev_depth_rejects']}"
              f" 回退={c['n_rev_theta_fallbacks']} [{el:.3f}s]", flush=True)
    return out


def _write_tables(results: dict) -> None:
    L: list[str] = []
    L.append("# θ 自适应深度门 — 自动表格（theta_adaptive_backtest.py 产出）\n")
    L.append("## 锚中枢相对振幅分布（(ZG−ZD)/c ×100%，每中枢最终观测值）\n")
    L.append("| 标的 | ladder | n | P10 | P25 | P50 | P75 | P90 | <1% 占比 |")
    L.append("|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        d = r.get("amp_dist", {})
        for lad, st in d.items():
            p = st["pct"]
            L.append(f"| {s} | {lad} | {st['n']} | {p['P10']} | {p['P25']} | "
                     f"{p['P50']} | {p['P75']} | {p['P90']} | "
                     f"{st['share_below_1pct']}% |")
    L.append("")
    L.append("## 主对照表（V2of = 固定 θ=1% 基线）\n")
    L.append("| 标的 | P5复利% | 变体 | 复利% | **Δ(vs P5) pp** | rev对数 | rev胜率% | "
             "rev净现金 | 震荡型(对/胜率/净) | 深度拒 | θ回退 | zd关 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        p5c = r["P5"]["metrics"]["total_compound"]
        for v, pk in r["variants"].items():
            rev = pk["leg_stats"]["rev"]
            ks = pk["kind_stats"]["osc"]
            c = pk["counters"]
            osc = ("—" if ks["n_pairs"] == 0 else
                   f"{ks['n_pairs']}/{ks['win_rate']}%/{ks['net_cash']:+.0f}")
            L.append(
                f"| {s} | {p5c:+.1f} | {v} | {pk['metrics']['total_compound']:+.1f} | "
                f"**{pk['delta_vs_p5']:+.1f}** | {rev['n_pairs']} | {rev['win_rate']} | "
                f"{rev['net_cash']:+.0f} | {osc} | {c['n_rev_depth_rejects']} | "
                f"{c['n_rev_theta_fallbacks']} | {c['n_rev_zd_close']} |")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    ref_all: dict = {}
    if REF_JSON.exists():
        ref_all = json.loads(REF_JSON.read_text())
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        done = sym in results and all(
            v in results[sym].get("variants", {}) for v in VARIANTS)
        if done and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有全部变体", flush=True)
            continue
        results[sym] = process_symbol(sym, ref=ref_all.get(sym),
                                      prev=results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        _write_tables(results)
        print(f"  [{sym}] 已落盘", flush=True)
    if results:
        _write_tables(results)


if __name__ == "__main__":
    main()
