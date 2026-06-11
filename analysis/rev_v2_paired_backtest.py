"""REV 腿配对修正 + 深度门槛回测 — V2f vs V1f（2026-06-10 编排者任务）。

═══════════════════════════════════════════════════════════════════════
对比矩阵
═══════════════════════════════════════════════════════════════════════
  标的：OKLO 447K / QQQ 728K / BRN 2.4M（与 v1/v2 验收同三标的同磁带语义）
  1. O0：Rust O0 ≡ Python O0（≡P5 传递）四面对账——确认配对修正没破坏已验证部分
  2. V1f：在册基线（G1 方向锚定，rev 胜率 45.6/45.2/43.5%）——改进幅度的分母
  3. V2p：配对修正（kind 展开开腿 + 配对闭腿），深度门关（θ=0）——配对独立因果
  4. V2f：配对修正 + 深度门槛 θ=1%（任务主变体）——V2f−V2p = 深度门独立贡献

配对修正语义（rust/src/trading/level_operating_unit.rs rev_paired 路径）：
  开腿 kind 展开：
    震荡型 = confirmed Sell1 / 盘整背驰卖 × 所在中枢存活（CenterBook.alive，
             单一真相源——与 P5 域腿同锚源）×深度门 × c>ZD（零利润空间拒）
    逃逸型 = confirmed Sell3（中枢向下终结）× 深度门（被终结中枢振幅）
    撤销触发：趋势背驰卖（Sell1 引擎前体，等事件本体）、candidate 事件
  闭腿配对（穷举三条，T5b 防悬挂不在集合内）：
    T7/T6 = Buy3 回补位（confirmed/candidate，恢复暴露）
    T5    = confirmed Buy1：震荡型同锚（e.cs == 锚 cs）/ 逃逸型任意锚（趋势配对）
    ZD 触线 = 震荡型专属（域腿 77% 胜率同机制）
    拒绝   = Buy2 / 盘背买 / 异锚 Buy1（n_rev_mismatch_holds 反事实计数）

预注册判据（任务原文）：
  (a) REV 胜率 45% → >50%？（按震荡/逃逸分解可观测）
  (b) QQQ/BRN Δ(vs P5) 从负翻正或接近零？

认识论等级：O0 对账 L1（管线等价）；V2p/V2f 结论 L2→L3（三标的真实数据，
新语义无 Python oracle）。

输出：analysis/rev_v2_paired_backtest.md
      analysis/data_cache/rev_v2_paired_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/rev_v2_paired_backtest.py
      （env：BT_SYMBOLS=OKLO,QQQ,BRN  BT_FORCE=1 强制重跑）
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
    _leg_stats_rust,
    _trades_from_rust,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
# BT_TAG：磁带语义代次隔离（C段修复后引擎的重跑用 BT_TAG=enginefix 落新文件，
# 旧磁带在册结果不可被增量混写——tape_fp 守卫的文件级补充）。
_TAG = os.environ.get("BT_TAG", "")
_SUF = f"_{_TAG}" if _TAG else ""
OUT_JSON = DATA_DIR / f"rev_v2_paired_backtest{_SUF}.json"
# 自动表格落 data_cache；终版报告 analysis/rev_v2_paired_backtest.md 为手工定稿
# （含事故记录与判据裁定），脚本不得覆写。
OUT_MD = DATA_DIR / f"rev_v2_paired_backtest_tables{_SUF}.md"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
# V2o/V2of：kind 标注首跑数据驱动的消融（逃逸型三标的一致为负 → 关逃逸型）
VARIANTS = ["V1f", "V2p", "V2f", "V2o", "V2of", "V2r"]
FLOOR = LADDER_SEG

COUNTER_KEYS = (
    "n_rev_attempts", "n_rev_sub_anchor_rejects", "n_rev_frozen_rejects",
    "n_rev_nocenter_rejects", "n_rev_depth_rejects", "n_rev_open",
    "n_rev_open_osc", "n_rev_open_esc", "n_rev_close_t5", "n_rev_close_t6",
    "n_rev_close_t7", "n_rev_zd_close", "n_rev_struct_close",
    "n_rev_mismatch_holds", "n_rev_tranche_closes", "n_open_rejects_zero",
    "rev_osc_pairs", "rev_osc_wins", "rev_esc_pairs", "rev_esc_wins",
    "rev_osc_net_cash", "rev_esc_net_cash",
)


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


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    """prev 含 P5/O0 时复用（增量补变体——O0 守卫与 P5 基线已落盘且磁带语义不变；
    信号层确定性重算，仅跑缺失变体）。"""
    print(f"\n{'=' * 64}\n  {symbol} — REV 配对修正回测\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    # ── 信号层（单 pass：磁带 + D3 方向行——V1f 的 G1 数据依赖）──
    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}", flush=True)

    # ── 磁带指纹守卫（2026-06-10 事故落盘）：并发工作流修改引擎源后磁带语义
    # 变化（OKLO bsp 12,209→10,706）。增量续跑混入不同语义磁带 = 不同实验的
    # 数字进同一对照表——指纹不匹配必须 fail-fast，不提供静默混表。
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(
            f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}——引擎/信号层语义"
            f"已变，禁止与在册条目混表。全量重跑（删除该标的 json 条目）或先核对"
            f"引擎变更。")

    rtape = pack_tape(tape, dir_flips=dir_flips)

    if prev is not None and "P5" in prev and prev.get("o0_guard") == "PASS":
        out = prev
        p5c = out["P5"]["metrics"]["total_compound"]
        print(f"  [增量] 复用在册 P5({p5c:+.2f}%) / O0 守卫", flush=True)
    else:
        # ── P5 基线（Δ 分母）──
        trades_p5, _ = run_version_i(
            tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"])
        m_p5 = extended_metrics(trades_p5, years)
        p5c = m_p5["total_compound"]
        print(f"  P5 复利 {p5c:+.2f}%", flush=True)

        # ── O0≡P5 四面对账（确认配对修正零侵入 legacy 路径）──
        o0 = _bitexact_o0(tape, rtape, years)
        print(f"  [O0≡P5] PASS  {o0['n_trades']} 笔 / {o0['n_trace_rows']} trace 行",
              flush=True)

        out = {"n_bars": n, "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
               "n_dir_flips": len(dir_flips), "tape_fp": fp, "o0_guard": "PASS",
               "P5": {"metrics": {k: m_p5[k] for k in
                                  ("n", "win_rate", "total_compound",
                                   "sharpe", "max_dd")}},
               "O0": o0, "variants": {}}

    # ── 变体矩阵（只跑缺失）──
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
        n_close = (c["n_rev_close_t5"] + c["n_rev_close_t6"]
                   + c["n_rev_close_t7"] + c["n_rev_struct_close"]
                   + c["n_rev_zd_close"])
        t6_share = round(c["n_rev_close_t6"] / n_close * 100, 1) if n_close else None
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "kind_stats": _kind_stats(c),
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "t6_close_share_pct": t6_share,
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        rev = legs["rev"]
        print(f"  [{name:3s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:5d}"
              f"(震={c['n_rev_open_osc']}/逃={c['n_rev_open_esc']})"
              f" rev胜率={rev['win_rate']}% 净现金={rev['net_cash']:+.0f}"
              f" zd关={c['n_rev_zd_close']} t6占比={t6_share}%"
              f" 深度拒={c['n_rev_depth_rejects']} [{el:.3f}s]", flush=True)
    return out


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# REV 腿配对修正 + 深度门槛回测 — V2f vs V1f\n")
    L.append("> 任务（2026-06-10 编排者）：REV 腿与域腿 type1卖→type3买（67.9% 卖飞）"
             "同根因——信号层 kind 折叠 + 盲配对。域腿在 P5 修复（kind-aware + ZD "
             "触线 → 77% 胜率），本轮把同一修复施加到 REV 腿，外加深度门槛 θ=1%。\n")
    L.append("> 预注册判据：(a) rev 胜率 45%→>50%；(b) QQQ/BRN Δ(vs P5) 转正或接近零。"
             "认识论等级：O0 守卫 L1；变体结论 L2→L3（三标的真实数据首跑，无 oracle）。\n")
    L.append("## 主对照表\n")
    L.append("| 标的 | O0≡P5 | P5复利% | 变体 | 复利% | **Δ(vs P5) pp** | rev对数 | "
             "rev胜率% | rev净现金 | 震荡型(对/胜率/净) | 逃逸型(对/胜率/净) | "
             "zd关 | t6占比% | 深度拒 | 错配持仓bar |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        p5c = r["P5"]["metrics"]["total_compound"]
        L.append(f"| {s} | {r['o0_guard']} | {p5c:+.1f} |  |  |  |  |  |  |  |  |  |  |  |")
        for v, p in r["variants"].items():
            rev = p["leg_stats"]["rev"]
            ks = p.get("kind_stats", {})
            c = p["counters"]

            def kfmt(k):
                d = ks.get(k, {})
                if not d or d["n_pairs"] == 0:
                    return "—"
                return f"{d['n_pairs']}/{d['win_rate']}%/{d['net_cash']:+.0f}"

            L.append(
                f"| {s} |  |  | {v} | {p['metrics']['total_compound']:+.1f} | "
                f"**{p['delta_vs_p5']:+.1f}** | {rev['n_pairs']} | {rev['win_rate']} | "
                f"{rev['net_cash']:+.0f} | {kfmt('osc')} | {kfmt('esc')} | "
                f"{c['n_rev_zd_close']} | {p['t6_close_share_pct']} | "
                f"{c['n_rev_depth_rejects']} | {c['n_rev_mismatch_holds']} |")
    L.append("")
    L.append("## REV 机制计数\n")
    L.append("| 标的 | 变体 | 尝试 | G1拒 | 冻结拒 | 无中枢拒 | 深度拒 | 开(震/逃) | "
             "T5配对关 | T6预回补 | T7回补 | ZD触线关 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s, r in results.items():
        for v, p in r["variants"].items():
            c = p["counters"]
            L.append(
                f"| {s} | {v} | {c['n_rev_attempts']} | "
                f"{c['n_rev_sub_anchor_rejects']} | {c['n_rev_frozen_rejects']} | "
                f"{c['n_rev_nocenter_rejects']} | {c['n_rev_depth_rejects']} | "
                f"{c['n_rev_open']}({c['n_rev_open_osc']}/{c['n_rev_open_esc']}) | "
                f"{c['n_rev_close_t5']} | {c['n_rev_close_t6']} | "
                f"{c['n_rev_close_t7']} | {c['n_rev_zd_close']} |")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        done = sym in results and all(
            v in results[sym].get("variants", {}) for v in VARIANTS)
        if done and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有全部变体", flush=True)
            continue
        results[sym] = process_symbol(sym, prev=results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        _write_report(results)
        print(f"  [{sym}] 已落盘", flush=True)
    if results:
        _write_report(results)


if __name__ == "__main__":
    main()
