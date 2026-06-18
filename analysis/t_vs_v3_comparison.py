"""统一递归算子 T（纯结构, 零 MACD）vs v3(fugue_v3) nf 信号 L2 对照 — §6.6 阶段2。

设计依据: docs/unified_recursive_operator_T.md §6.6/§6.7（编排者裁决: 纯结构优先渐进,
阶段2 与 v3 nf 对照实测纯拓扑替代率, L2, 否定性结果定位缺口）。

对照轴（关键设计决策, 见模块末 NOTE）:
  - **price 匹配**（坐标无关）: T BSP 与 nf fire 的 price 都是同一段极值。Stroke.i0/i1
    是 K线合并坐标, nf_fires.bar 是 raw bar 序号——两者坐标系不同, bar 不可直接相减
    （记忆 current_strokes_i1_merged_coord）。price 是买卖点本质标识且坐标无关 ⇒ 主匹配轴。
  - **level 映射**: T.level+3 == v3.ladder（segment=ladder2 不在 T 递归序列中,
    T 从 move/L1=ladder3 起; 设计文档 §5.2 bug-a）。脚本输出实测分布验证此映射, 不硬编码。
  - **极性**: T type1_sell/type3_sell ↔ nf is_sell=True; buy ↔ is_sell=False。
  - nf fire 来自 candidate(type1/type3)武装+次级别证据 ⇒ 主对照用 T 的 type1+type3;
    type2(跨级投影)单列为 T 特有。

a₀ 一致性: T 用 orchestrator.current_segments()(confirmed+settled 过滤在 run_recursive_t 内),
  v3 用同配置 orchestrator(compute_organic_signals 内部)。同款 wide 笔 + strict 段 ⇒ a₀ 同源。

认识论等级: L2（真实数据, 可否证）。否定性结果 = 纯结构在某 level/regime 漏判(nf_only)
  或误判(t_only) 的具体位置 = 阶段3 加 MACD 的定位器。

用法:
  .venv/bin/python analysis/t_vs_v3_comparison.py [symbols] [max_bars] [--diag]
  symbols: 逗号分隔（默认 8 标的）; max_bars: 采样上限（默认全量）; --diag: 只跑诊断不写报告
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import newchan_rust as R  # noqa: E402
from organic_signals import compute_organic_signals, push_signal  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402

MAX_LEVELS = 6
LADDER_MOVE = 3          # v3 move/L1 = T.level 0 的对照层（设计文档 §5.2 bug-a）
PRICE_REL_TOL = 1e-4     # 相对价格容差（同段极值应 bit-exact, 留浮点裕度）
LAD_NAME = {2: "seg", 3: "move/L1", 4: "recL2", 5: "recL3", 6: "recL4", 7: "recL5",
            8: "L6", 9: "L7", 10: "L8"}
DEFAULT_SYMS = ["CL", "BRN", "DX", "GC", "ES", "QQQ", "BTC", "OKLO"]
OUT_MD = Path(__file__).parent / "T_vs_v3_comparison.md"


def extract_t_bsps(opens, highs, lows, closes):
    """orchestrator 产段（= v3 同款 a₀）→ run_recursive_t → T 全塔买卖点。

    返回 (bsps, n_segs)，bsps 行 = (kind, bar, price, level)。
    kind ∈ {type1_buy/sell, type2_*, type3_*}; level 0 = 最低走势级别（线段构成）。
    """
    orch = R.RecursiveOrchestrator(
        max_levels=MAX_LEVELS, enable_macd_divergence=True, require_settled_subseg=True
    )
    for o, h, l, c in zip(opens, highs, lows, closes):
        orch.process_bar(o, h, l, c)
    segs = orch.current_segments()
    # SegmentTuple[0] = (s0, s1, i0, i1, dir, high, low, confirmed, kind)
    seg_in = [
        (s[0][2], s[0][3], s[0][4], s[0][5], s[0][6], s[0][7], s[0][8] == "settled")
        for s in segs
    ]
    bsps = R.run_recursive_t(seg_in)  # [(kind, bar, price, level)]
    return bsps, len(segs)


def extract_v3_nf(opens, highs, lows, closes):
    """compute_organic_signals → FugueV3Stream → nf fire 明细。

    返回 (nf_fires, n_bars)，nf_fires 行 = (bar, ladder, is_sell, price)。
    """
    dir_flips: list = []
    tape = compute_organic_signals(
        opens, highs, lows, closes, dir_flips=dir_flips, require_settled=True
    )
    flips_by_bar: dict = {}
    for (bar, lad, d) in dir_flips:
        flips_by_bar.setdefault(bar, []).append((lad, d))
    stream = R.FugueV3Stream(floor_ladder=2)
    for i, s in enumerate(tape):
        push_signal(stream, s, flips_by_bar.get(i, []))
    res = stream.finish()
    return res["nf_fires"], len(tape)


def _price_dist(prices):
    """价格分布摘要（count/min/max/样本）。"""
    if not prices:
        return {"n": 0}
    sp = sorted(prices)
    return {"n": len(sp), "min": round(sp[0], 4), "max": round(sp[-1], 4),
            "sample": [round(p, 4) for p in sp[:5]]}


def diagnose(sym, t_bsps, nf_fires):
    """诊断: T levels / nf ladders 的 price 分布 + level 映射验证。"""
    print(f"\n===== [{sym}] 诊断 =====", flush=True)
    # T by level（type1+type3 分 buy/sell；type2 单列）
    print("  T 算子（纯结构）by level:")
    t_levels = sorted({b[3] for b in t_bsps})
    for lv in t_levels:
        for tag, pred in (("t1+t3 sell", lambda k: k in ("type1_sell", "type3_sell")),
                          ("t1+t3 buy", lambda k: k in ("type1_buy", "type3_buy")),
                          ("type2", lambda k: k.startswith("type2"))):
            pr = [b[2] for b in t_bsps if b[3] == lv and pred(b[0])]
            if pr:
                print(f"    level{lv}(↔ladder{lv + LADDER_MOVE}) {tag:11} {_price_dist(pr)}")
    # nf by ladder
    print("  v3 nf fire by ladder:")
    nf_ladders = sorted({f[1] for f in nf_fires})
    for lad in nf_ladders:
        for tag, sell in (("nf_sell", True), ("nf_buy", False)):
            pr = [f[3] for f in nf_fires if f[1] == lad and f[2] == sell]
            if pr:
                print(f"    ladder{lad}({LAD_NAME.get(lad, '?')}) {tag:7} {_price_dist(pr)}")


def _match_prices(t_prices, nf_prices, rel_tol):
    """贪心双指针 price 匹配（升序）。返回 (n_matched, t_only_prices, nf_only_prices）。

    同段极值应 bit-exact ⇒ rel_tol 极小; 双指针对近重合集合无歧义。
    """
    t = sorted(t_prices)
    n = sorted(nf_prices)
    i = j = 0
    matched = 0
    t_only: list = []
    nf_only: list = []
    while i < len(t) and j < len(n):
        denom = max(abs(n[j]), 1e-9)
        if abs(t[i] - n[j]) <= rel_tol * denom:
            matched += 1
            i += 1
            j += 1
        elif t[i] < n[j]:
            t_only.append(t[i])
            i += 1
        else:
            nf_only.append(n[j])
            j += 1
    t_only.extend(t[i:])
    nf_only.extend(n[j:])
    return matched, t_only, nf_only


def compare(sym, t_bsps, nf_fires):
    """by (ladder, side) price 匹配对照矩阵。返回结构化结果。"""
    rows = []
    nf_ladders = {f[1] for f in nf_fires}
    t_ladders = {b[3] + LADDER_MOVE for b in t_bsps if b[0].startswith(("type1", "type3"))}
    all_ladders = sorted(nf_ladders | t_ladders)
    for lad in all_ladders:
        lv = lad - LADDER_MOVE
        for side_tag, is_sell in (("sell", True), ("buy", False)):
            t_kinds = ("type1_sell", "type3_sell") if is_sell else ("type1_buy", "type3_buy")
            t_pr = [b[2] for b in t_bsps if b[3] == lv and b[0] in t_kinds]
            nf_pr_raw = [f[3] for f in nf_fires if f[1] == lad and f[2] == is_sell]
            # nf 去重（范畴对齐）: nf fire 是逐 bar 事件流(同结构点可多次 fire), T BSP 是去重
            # 结构点集。按 price 去重把 nf 投影回点集范畴, 才与 T 可比; n_nf_raw 另列保留频率维度。
            nf_pr = sorted(set(nf_pr_raw))
            if not t_pr and not nf_pr:
                continue
            matched, t_only, nf_only = _match_prices(t_pr, nf_pr, PRICE_REL_TOL)
            rows.append({
                "ladder": lad, "level": lv, "name": LAD_NAME.get(lad, "?"), "side": side_tag,
                "n_T": len(t_pr), "n_nf": len(nf_pr), "n_nf_raw": len(nf_pr_raw),
                "matched": matched, "t_only": len(t_only), "nf_only": len(nf_only),
                "t_match_rate": round(matched / len(t_pr), 3) if t_pr else None,
                "nf_match_rate": round(matched / len(nf_pr), 3) if nf_pr else None,
            })
    # type2（T 特有）计数
    n_type2 = sum(1 for b in t_bsps if b[0].startswith("type2"))
    return {"rows": rows, "n_type2": n_type2}


def regime_segments(closes, n_seg=5):
    """价格序列按等长 bar 窗口分段, 每段标 bull/bear/range（首尾涨跌幅 ±10% 阈值）。"""
    n = len(closes)
    if n < n_seg:
        return []
    out = []
    w = n // n_seg
    for s in range(n_seg):
        lo = s * w
        hi = (s + 1) * w if s < n_seg - 1 else n
        ret = closes[hi - 1] / closes[lo] - 1 if closes[lo] else 0.0
        kind = "bull" if ret > 0.10 else "bear" if ret < -0.10 else "range"
        out.append({"bar_lo": lo, "bar_hi": hi, "ret_pct": round(ret * 100, 1), "regime": kind})
    return out


def regime_nf_breakdown(nf_fires, regimes):
    """nf fire 按 raw bar 落入 regime 段统计（nf 有 raw bar, 可定位）。"""
    if not regimes:
        return []
    out = []
    for rg in regimes:
        cnt = sum(1 for f in nf_fires if rg["bar_lo"] <= f[0] < rg["bar_hi"])
        out.append({**rg, "n_nf": cnt})
    return out


def run_symbol(sym, max_bars, diag_only):
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, _years = load_ohlc(path)
    if max_bars:
        opens, highs, lows, closes = (x[:max_bars] for x in (opens, highs, lows, closes))
    n = len(closes)
    print(f"\n[{sym}] bars={n:,}", flush=True)

    t0 = time.time()
    t_bsps, n_segs = extract_t_bsps(opens, highs, lows, closes)
    print(f"[{sym}] T: {time.time() - t0:.1f}s segs={n_segs:,} BSPs={len(t_bsps):,}", flush=True)

    t1 = time.time()
    nf_fires, n_tape = extract_v3_nf(opens, highs, lows, closes)
    print(f"[{sym}] v3 nf: {time.time() - t1:.1f}s fires={len(nf_fires):,}", flush=True)

    diagnose(sym, t_bsps, nf_fires)
    if diag_only:
        return None

    cmp = compare(sym, t_bsps, nf_fires)
    regimes = regime_segments(closes)
    cmp["regime"] = regime_nf_breakdown(nf_fires, regimes)
    cmp["symbol"] = sym
    cmp["n_bars"] = n
    cmp["n_segs"] = n_segs
    cmp["n_t_bsps"] = len(t_bsps)
    cmp["n_nf_fires"] = len(nf_fires)
    return cmp


def write_report(results, max_bars):
    lines = ["# T 算子（纯结构, 零 MACD）vs v3 nf 信号 L2 对照\n",
             "**设计依据**: `docs/unified_recursive_operator_T.md` §6.6 阶段2"
             "（纯结构优先渐进, 否定性结果定位缺口）。",
             "**认识论等级**: L2（真实数据, 可否证）。",
             f"**采样**: {'全量' if not max_bars else f'前 {max_bars:,} bar'}。",
             "**对照轴**: price 匹配（坐标无关）+ level 映射(T.level+3=v3.ladder) + 极性。",
             "**口径边界**: nf fire = 区间套(candidate type1/3 武装+次级别证据触发); T 对照用 "
             "type1+type3; type2(跨级投影)为 T 特有单列。\n"]
    for r in results:
        if r is None:
            continue
        lines.append(f"\n## {r['symbol']}  (bars={r['n_bars']:,}, segs={r['n_segs']:,}, "
                     f"T BSP={r['n_t_bsps']:,}, nf fire={r['n_nf_fires']:,}, T type2={r['n_type2']:,})\n")
        lines.append("| ladder | 级别 | side | n_T(t1+t3) | n_nf(去重) | n_nf_raw(fire) | matched | "
                     "T_only(误判?) | nf_only(漏判?) | T匹配率 | nf匹配率 |")
        lines.append("|---|---|---|---|---|---|---|---|---|---|---|")
        for row in r["rows"]:
            lines.append(f"| {row['ladder']} | {row['name']} | {row['side']} | {row['n_T']} | "
                         f"{row['n_nf']} | {row['n_nf_raw']} | {row['matched']} | {row['t_only']} | "
                         f"{row['nf_only']} | {row['t_match_rate']} | {row['nf_match_rate']} |")
        if r["regime"]:
            lines.append("\n**regime 分布（nf fire by raw-bar 段）**:")
            lines.append("| 段 | bar 区间 | 收益% | regime | nf fire 数 |")
            lines.append("|---|---|---|---|---|")
            for i, rg in enumerate(r["regime"]):
                lines.append(f"| {i} | [{rg['bar_lo']:,},{rg['bar_hi']:,}) | {rg['ret_pct']} | "
                             f"{rg['regime']} | {rg['n_nf']} |")
    OUT_MD.write_text("\n".join(lines))
    print(f"\n报告已写入 {OUT_MD}", flush=True)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    diag_only = "--diag" in sys.argv
    syms = args[0].split(",") if args else DEFAULT_SYMS
    max_bars = int(args[1]) if len(args) > 1 and args[1] else None
    results = []
    for sym in syms:
        sym = sym.strip().upper()
        if sym not in SYMBOL_FILES:
            print(f"标的 {sym} 无数据映射, 跳过", file=sys.stderr)
            continue
        try:
            results.append(run_symbol(sym, max_bars, diag_only))
        except Exception as e:  # noqa: BLE001
            print(f"[{sym}] 失败: {type(e).__name__}: {e}", file=sys.stderr)
            import traceback
            traceback.print_exc()
    if not diag_only and any(r is not None for r in results):
        write_report(results, max_bars)


if __name__ == "__main__":
    main()
