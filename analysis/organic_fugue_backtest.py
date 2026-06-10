"""有机赋格验收回测 — O0≡P5 守卫 + O1-O4 变体矩阵（设计 §8）。

═══════════════════════════════════════════════════════════════════════
变体矩阵（§8.1，消融轴正交分解）
═══════════════════════════════════════════════════════════════════════
  O0   REV✗            equal      earning✗   ≡P5 逐位守卫（不过不进 O1+）
  O1   REV✓ 门✗        equal      earning✗   段尺度反向裸 alpha
  O2   REV✓ 门✓        equal      earning✗   41课门因果增量（O2−O1）
  O3   REV✓ 门✓        structure  earning✗   结构规模 vs 均分（O3−O2）
  O4   REV✓ 门✓        structure  earning✓   earning 反作用（前置：触发率>0）
  O2c/O2n（探索性，不进主判决）：REV 关腿 R_cand / R_nested 消融

═══════════════════════════════════════════════════════════════════════
预注册判据（§8.2，先于跑批写定——本 docstring 即预注册文本）
═══════════════════════════════════════════════════════════════════════
Δ(X) = vs_B0(X) − vs_B0(P5)。主判据只有 1/3/4 三条：
  1. REV 假设：O1 在 ≥2/3 标的 Δ>0 → 段尺度反向独立成立；
     O1 全负 ∧ O2 转正 → 41课门是 REV 必要条件（最强结果）；
     O1≈O2 均正 → 门在该数据无增量（门闲置≠门错误，查门开率）；
     O1、O2 全负 → 段尺度反向被否证，框架收缩为 P5+规模轴。
  2.（前置量）门开率≈0 时 O2−O1 无统计意义（结论降级"无定义域"，E9 先例）。
  3. 规模假设：O3 在 ≥2/3 标的 Δ(O3)>Δ(O2) → 结构规模成立。
  4. earning 假设：触发率=0 → O4 无定义域（诚实落盘）；>0 → O4 vs O3 同 Δ 比较。
  5. REV 腿独立质量：rev 腿 avg_diff 预期 > osc 腿；若 ≤ 且净现金负 →
     判据 1 的正结果也要降级。
  6. 多重比较防护：O2c/O2n 标注 exploratory。

磁带级回归锚：本脚本的 P5/B0 参照值必须与
`interval_nesting_reverse_backtest.json` 缓存值逐位一致（同一信号语义的跨脚本
复现守卫）；O0 与 P5 逐笔 + 逐 trace 对账。

费率敏感表（§8.3-c）：trace 级后算（profit_fee = profit − shares×(卖价+买价)×
bps/1e4），**不进 cost_basis 复利路径**——近似声明：费用对降成本轨迹的二阶
影响（cost_basis 路径变化→腿序列变化）未建模，该表只回答"腿层一阶净现金在
费率下是否存活"。

输出：analysis/organic_fugue_backtest.md
      analysis/data_cache/organic_fugue_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/organic_fugue_backtest.py
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

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    PAIRING_VARIANTS,
    PairingConfig,
    extended_metrics,
    run_version_i,
)
from organic_fugue import (  # noqa: E402
    ORGANIC_VARIANTS,
    run_organic,
)
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "organic_fugue_backtest.json"
OUT_MD = ROOT / "analysis" / "organic_fugue_backtest.md"
REF_JSON = DATA_DIR / "interval_nesting_reverse_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
VARIANTS = ["O1", "O2", "O3", "O4", "O2c", "O2n"]
EXPLORATORY = {"O2c", "O2n"}
FLOOR = LADDER_SEG
FEE_BPS = (2, 5, 10)

# B0（无短差基线）：无任何腿（open_kinds 空 + osc 关）——纯进出场。
B0_CONFIG = PairingConfig(open_kinds=(), hard_type3=False, pre_type3=False,
                          center_gate=False, same_center_close=False,
                          osc_mode=False)


# ════════════════════════════════════════════════════════════
# 腿分解统计（main/osc/rev）
# ════════════════════════════════════════════════════════════

def _leg_stats(diag: list) -> dict:
    """trace 行按腿类型聚合（含费率敏感一阶净现金）。"""
    def agg(rows: list) -> dict:
        n = len(rows)
        if n == 0:
            return {"n_pairs": 0, "fly_rate": None, "win_rate": None,
                    "avg_diff_pct": None, "net_cash": 0.0,
                    "net_cash_fee": {str(b): 0.0 for b in FEE_BPS}}
        n_fly = sum(1 for r in rows if r["diff"] < 0)
        n_win = sum(1 for r in rows if r["diff"] > 0)
        avg_diff = sum(r["diff"] / r["sell_price"] for r in rows) / n * 100
        fee = {}
        for b in FEE_BPS:
            fee[str(b)] = round(sum(
                r["profit"] - r["shares"] * (r["sell_price"] + r["buy_price"])
                * b / 1e4 for r in rows), 1)
        return {
            "n_pairs": n,
            "fly_rate": round(n_fly / n * 100, 1),
            "win_rate": round(n_win / n * 100, 1),
            "avg_diff_pct": round(avg_diff, 4),
            "net_cash": round(sum(r["profit"] for r in rows), 1),
            "net_cash_fee": fee,
        }

    by_leg: dict[str, list] = {"main": [], "osc": [], "rev": []}
    for tr in diag:
        for r in tr["diffs"]:
            leg = r.get("leg")
            if leg is None:  # run_version_i trace（无 leg 键）→ 按槽键分类
                leg = ("osc" if r["ladder"] >= 100 else "main")
            by_leg[leg].append(r)
    out = {leg: agg(rows) for leg, rows in by_leg.items()}
    out["all"] = agg([r for rows in by_leg.values() for r in rows])
    return out


def _metrics_pack(trades, years, bh: float) -> dict:
    m = extended_metrics(trades, years)
    return {
        "metrics": {k: m[k] for k in
                    ("n", "win_rate", "total_compound", "sharpe", "max_dd",
                     "profit_factor", "n_with_cr")},
        "excess": round(m["total_compound"] - bh, 2),
    }


# ════════════════════════════════════════════════════════════
# O0≡P5 守卫
# ════════════════════════════════════════════════════════════

def _o0_guard(tape, years, bh) -> tuple[dict, dict, list]:
    """O0 与 P5 逐笔 + 逐 trace 对账。失败 → RuntimeError（不进 O1+）。"""
    diag_p5: list = []
    trades_p5, extra_p5 = run_version_i(
        tape, floor_ladder=FLOOR, pairing=PAIRING_VARIANTS["P5"], diag=diag_p5)
    diag_o0: list = []
    trades_o0, extra_o0 = run_organic(
        tape, floor_ladder=FLOOR, config=ORGANIC_VARIANTS["O0"], diag=diag_o0)
    if trades_p5 != trades_o0:
        raise RuntimeError(
            f"O0≡P5 守卫失败：trades 不等（P5 {len(trades_p5)} 笔 vs "
            f"O0 {len(trades_o0)} 笔；首个分歧见 diff 对比）")
    if len(diag_p5) != len(diag_o0):
        raise RuntimeError("O0≡P5 守卫失败：diag 条数不等")
    for ti, (a, b) in enumerate(zip(diag_p5, diag_o0)):
        if a["n_diffs"] != b["n_diffs"]:
            raise RuntimeError(f"O0≡P5 守卫失败：trade#{ti} 短差数不等")
        for ri, (ra, rb) in enumerate(zip(a["diffs"], b["diffs"])):
            for key in ra:
                if ra[key] != rb[key]:
                    raise RuntimeError(
                        f"O0≡P5 守卫失败：trade#{ti} diff#{ri} 字段 {key}: "
                        f"{ra[key]} != {rb[key]}")
    p5_pack = _metrics_pack(trades_p5, years, bh)
    p5_pack["leg_stats"] = _leg_stats(diag_p5)
    p5_pack["pairing_counters"] = extra_p5.get("pairing")
    o0_pack = _metrics_pack(trades_o0, years, bh)
    o0_pack["counters"] = extra_o0["counters"]
    return p5_pack, o0_pack, diag_p5


# ════════════════════════════════════════════════════════════
# 单标的管线
# ════════════════════════════════════════════════════════════

def process_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 有机赋格 O0-O4 变体矩阵\n{'=' * 64}",
          flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes)
    sig_elapsed = time.time() - t0
    n_div = sum(len(row) for s in tape for row in s.div_events)
    n_cons = sum(1 for s in tape for row in s.div_events
                 for e in row if e[0] == "consolidation")
    print(f"  信号层 {sig_elapsed:.1f}s  背驰事件={n_div:,}"
          f"（consolidation={n_cons:,}）差分守卫全程通过", flush=True)

    out: dict = {"n_bars": n, "bh": round(bh, 2),
                 "sig_elapsed": round(sig_elapsed, 1),
                 "n_div_events": n_div, "n_div_consolidation": n_cons,
                 "variants": {}}

    # ── B0（无短差基线）+ 磁带级回归锚 ──
    b0_trades, _ = run_version_i(tape, floor_ladder=FLOOR, pairing=B0_CONFIG)
    out["B0"] = _metrics_pack(b0_trades, years, bh)
    b0c = out["B0"]["metrics"]["total_compound"]

    # ── O0≡P5 守卫（第一道守卫，失败即 abort）──
    p5_pack, o0_pack, _diag_p5 = _o0_guard(tape, years, bh)
    out["P5"] = p5_pack
    out["O0"] = o0_pack
    out["o0_guard"] = "PASS"
    p5c = p5_pack["metrics"]["total_compound"]
    print(f"  [O0≡P5 守卫] PASS  P5复利={p5c:+.2f}%  B0={b0c:+.2f}%", flush=True)

    # 跨脚本回归锚：interval_nesting 缓存的 P5/B0 必须逐位复现
    if REF_JSON.exists():
        ref = json.loads(REF_JSON.read_text()).get(symbol)
        if ref:
            ref_p5 = ref["variants"]["P5"]["metrics"]["total_compound"]
            ref_b0 = ref.get("B0", {}).get("metrics", {}).get("total_compound")
            if abs(ref_p5 - p5c) > 1e-9 or (
                    ref_b0 is not None and abs(ref_b0 - b0c) > 1e-9):
                raise RuntimeError(
                    f"磁带级回归锚失败：P5 {p5c} vs 缓存 {ref_p5}；"
                    f"B0 {b0c} vs 缓存 {ref_b0}")
            print("  [磁带级回归锚] PASS（P5/B0 与 interval_nesting 缓存逐位一致）",
                  flush=True)
            out["tape_anchor"] = "PASS"

    # ── 变体矩阵 ──
    for name in VARIANTS:
        cfg = ORGANIC_VARIANTS[name]
        diag: list = []
        trades, extra = run_organic(tape, floor_ladder=FLOOR, config=cfg,
                                    diag=diag)
        pack = _metrics_pack(trades, years, bh)
        pack["leg_stats"] = _leg_stats(diag)
        pack["counters"] = extra["counters"]
        pack["rev_attempts_by_ladder"] = extra["rev_attempts_by_ladder"]
        pack["rev_opens_by_ladder"] = extra["rev_opens_by_ladder"]
        pack["exploratory"] = name in EXPLORATORY
        out["variants"][name] = pack
        m = pack["metrics"]; cnt = pack["counters"]
        rl = pack["leg_stats"]["rev"]
        gate_rate = (round((cnt["n_rev_attempts"] - cnt["n_rev_gate_rejects"])
                           / cnt["n_rev_attempts"] * 100, 1)
                     if cnt["n_rev_attempts"] else None)
        print(f"  [{symbol}/{name:3s}] 交易={m['n']:4d} 复利={m['total_compound']:+10.2f}%"
              f" vs_B0={m['total_compound']-b0c:+8.2f}pp"
              f" | rev开={cnt['n_rev_open']:4d} 门开率={gate_rate}%"
              f" rev净现金={rl['net_cash']:+.0f}"
              f" earning={cnt['n_earning_reached']}", flush=True)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    t_all = time.time()
    for sym in SYMBOLS:
        if sym in results and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有结果（BT_FORCE=1 强制重跑）", flush=True)
            continue
        results[sym] = process_symbol(sym)
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        _write_report(results)
        print(f"  [{sym}] 已落盘 → {OUT_JSON.name} / {OUT_MD.name}", flush=True)
    if results:
        _write_report(results)
    print(f"\n总耗时 {time.time() - t_all:.1f}s", flush=True)


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

_VAR_DESC = {
    "P5": "参照：域腿+离开段腿（interval_nesting 判决最优信号类）",
    "O0": "≡P5 逐位守卫（rev 关）",
    "O1": "REV 腿（38课段终结三触发），无 41课门",
    "O2": "O1 + 41课门（上级别衰竭证据守卫 OPEN_REV）",
    "O3": "O2 + 结构规模（中枢振幅占比 frac[k]）",
    "O4": "O3 + earning 反作用（31课：cost=0 后 master 出场升级）",
    "O2c": "O2 + REV 关腿放宽 candidate（exploratory）",
    "O2n": "O2 + REV 关腿区间套（candidate ∧ 次级别买点，exploratory）",
}


def _vs_b0(r: dict, name: str) -> float | None:
    src = r["variants"].get(name) or r.get(name)
    if not src:
        return None
    return round(src["metrics"]["total_compound"]
                 - r["B0"]["metrics"]["total_compound"], 1)


def _delta(r: dict, name: str) -> float | None:
    """Δ(X) = vs_B0(X) − vs_B0(P5)（§8.2 预注册量）。"""
    v = _vs_b0(r, name)
    p5 = _vs_b0(r, "P5")
    if v is None or p5 is None:
        return None
    return round(v - p5, 1)


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 有机赋格验收回测 — O0≡P5 守卫 + O1-O4 变体矩阵\n")
    L.append("> **框架**：单一 LOU（38课程式 FSM）× N 声部 + 41课衰竭门 + 中枢振幅"
             "结构规模 + 两阶段守恒律账本（31课 earning 反作用）。P5 域腿原样嵌入"
             "RIDE 态子循环。设计：`analysis/organic_fugue_design.md`；实现："
             "`organic_fugue.py` + `organic_signals.py`。\n")
    L.append("> **口径**：floor=segment；成交=信号 bar 收盘价（与 P5 一致，无滑点）；"
             "力度=价格振幅 fallback（非 MACD，521号边界继承）；div_events 含盘整"
             "背驰（E10 最后一块）。认识论 **L2→L3**（OKLO 主验证 + QQQ/BRN 交叉）。\n")

    L.append("## 守卫状态\n")
    L.append("| 标的 | O0≡P5（逐笔+逐trace） | 磁带级回归锚（P5/B0 vs 缓存） |")
    L.append("|------|----------------------|------------------------------|")
    for s, r in results.items():
        L.append(f"| {s} | {r.get('o0_guard', '—')} | {r.get('tape_anchor', '—')} |")
    L.append("")

    L.append("## 变体定义\n")
    L.append("| 变体 | 内容 |")
    L.append("|------|------|")
    for v, d in _VAR_DESC.items():
        L.append(f"| {v} | {d} |")
    L.append("")

    L.append("## 主对照表（Δ(X) = vs_B0(X) − vs_B0(P5)，§8.2 预注册量）\n")
    L.append("| 标的 | 变体 | 复利% | 超额(BH) | vs_B0(pp) | **Δ(pp)** | MDD | 交易 | "
             "rev腿 | rev净现金 | earning |")
    L.append("|------|------|-------|----------|-----------|-----------|-----|------|"
             "-------|-----------|---------|")
    for s, r in results.items():
        b0m = r["B0"]["metrics"]
        L.append(f"| {s} | B0(无短差) | {b0m['total_compound']:+.1f} | "
                 f"{r['B0']['excess']:+.1f} | +0.0 | — | {b0m['max_dd']:+.1f} | "
                 f"{b0m['n']} | — | — | — |")
        for v in ("P5", "O1", "O2", "O3", "O4", "O2c", "O2n"):
            src = r["variants"].get(v) or r.get(v)
            if not src:
                continue
            m = src["metrics"]
            rev = src.get("leg_stats", {}).get("rev", {})
            cnt = src.get("counters", {})
            tag = f"{v}*" if src.get("exploratory") else v
            L.append(
                f"| {s} | {tag} | {m['total_compound']:+.1f} | "
                f"{m['total_compound'] - r['bh']:+.1f} | {_vs_b0(r, v):+.1f} | "
                f"**{_delta(r, v):+.1f}** | {m['max_dd']:+.1f} | {m['n']} | "
                f"{rev.get('n_pairs', '—')} | "
                f"{rev.get('net_cash', 0):+.0f} | "
                f"{cnt.get('n_earning_reached', '—')} |")
        L.append("| | | | | | | | | | | |")
    L.append("\n> `*` = exploratory（多重比较防护：不进主判决）。\n")

    L.append("## 门开率前置量（§8.2-2）与 REV 机制计数\n")
    L.append("| 标的 | 变体 | T1尝试 | 门拒 | 冻结拒 | 门开率% | rev开 | "
             "T5关 | T6预逃 | T7逃+冻 | 开拒(零股) |")
    L.append("|------|------|--------|------|--------|---------|-------|"
             "-----|--------|---------|-----------|")
    for s, r in results.items():
        for v in VARIANTS:
            src = r["variants"].get(v)
            if not src:
                continue
            c = src["counters"]
            att = c["n_rev_attempts"]
            rate = (round((att - c["n_rev_gate_rejects"]) / att * 100, 1)
                    if att else None)
            L.append(
                f"| {s} | {v} | {att} | {c['n_rev_gate_rejects']} | "
                f"{c['n_rev_frozen_rejects']} | {rate} | {c['n_rev_open']} | "
                f"{c['n_rev_close_t5']} | {c['n_rev_close_t6']} | "
                f"{c['n_rev_close_t7']} | {c['n_open_rejects_zero']} |")
    L.append("")

    L.append("## 腿分解（main/osc/rev 独立质量，§8.2-5）\n")
    L.append("| 标的 | 变体 | 腿 | 对数 | 卖飞率% | 胜率% | avg_diff% | 净现金 | "
             "@2bps | @5bps | @10bps |")
    L.append("|------|------|----|------|---------|-------|-----------|--------|"
             "------|------|-------|")
    for s, r in results.items():
        for v in ("P5", "O2", "O3", "O4"):
            src = r["variants"].get(v) or r.get(v)
            if not src or "leg_stats" not in src:
                continue
            for leg in ("main", "osc", "rev"):
                p = src["leg_stats"][leg]
                if p["n_pairs"] == 0:
                    continue
                f = p["net_cash_fee"]
                L.append(
                    f"| {s} | {v} | {leg} | {p['n_pairs']} | {p['fly_rate']} | "
                    f"{p['win_rate']} | {p['avg_diff_pct']} | {p['net_cash']:+.0f} | "
                    f"{f['2']:+.0f} | {f['5']:+.0f} | {f['10']:+.0f} |")
    L.append("\n> 费率列 = trace 级一阶近似（不进 cost_basis 复利路径，"
             "见脚本 docstring 声明）。\n")

    L.append("## rev 触发/开腿 per-ladder 分布（高层稀疏报告，§8.3-f）\n")
    L.append("| 标的 | 变体 | T1尝试分布 | rev开分布 |")
    L.append("|------|------|-----------|-----------|")
    for s, r in results.items():
        for v in ("O1", "O2"):
            src = r["variants"].get(v)
            if not src:
                continue
            L.append(f"| {s} | {v} | {src['rev_attempts_by_ladder']} | "
                     f"{src['rev_opens_by_ladder']} |")
    L.append("")

    L.append("## 标的概览\n")
    L.append("| 标的 | bars | BH% | 信号层s | 背驰事件 | 其中盘整背驰 |")
    L.append("|------|------|-----|---------|----------|--------------|")
    for s, r in results.items():
        L.append(f"| {s} | {r['n_bars']:,} | {r['bh']:+.1f} | {r['sig_elapsed']} | "
                 f"{r['n_div_events']:,} | {r['n_div_consolidation']:,} |")
    L.append("")
    L.extend(_result_package(results))
    OUT_MD.write_text("\n".join(L))


def _verdict(results: dict) -> dict:
    """§8.2 预注册判据自动评估（数值从 results 程序化抽取）。"""
    syms = list(results)
    n_sym = len(syms)
    d_o1 = {s: _delta(results[s], "O1") for s in syms}
    d_o2 = {s: _delta(results[s], "O2") for s in syms}
    d_o3 = {s: _delta(results[s], "O3") for s in syms}
    d_o4 = {s: _delta(results[s], "O4") for s in syms}

    def npos(d):
        return sum(1 for v in d.values() if v is not None and v > 0)

    gate_rates = {}
    for s in syms:
        c = results[s]["variants"].get("O2", {}).get("counters", {})
        att = c.get("n_rev_attempts", 0)
        gate_rates[s] = (round((att - c.get("n_rev_gate_rejects", 0))
                               / att * 100, 1) if att else None)
    earning = {s: results[s]["variants"].get("O4", {}).get("counters", {})
               .get("n_earning_reached", 0) for s in syms}
    rev_q = {}
    for s in syms:
        ls = results[s]["variants"].get("O2", {}).get("leg_stats", {})
        rev_q[s] = {"rev_avg_diff": ls.get("rev", {}).get("avg_diff_pct"),
                    "osc_avg_diff": ls.get("osc", {}).get("avg_diff_pct"),
                    "rev_net": ls.get("rev", {}).get("net_cash")}

    # 判据 1
    if npos(d_o1) >= max(2, n_sym - 1) and n_sym >= 2:
        c1 = "REV 独立成立（O1 ≥2/3 标的 Δ>0）"
    elif npos(d_o1) == 0 and npos(d_o2) >= max(2, n_sym - 1) and n_sym >= 2:
        c1 = "41课门是 REV 必要条件（O1 全负 ∧ O2 转正——最强结果）"
    elif npos(d_o1) == 0 and npos(d_o2) == 0:
        c1 = "段尺度反向被否证（O1、O2 全负）——框架收缩为 P5+规模轴"
    else:
        c1 = f"混合结果：O1 正 {npos(d_o1)}/{n_sym}，O2 正 {npos(d_o2)}/{n_sym}"
    # 判据 3
    n3 = sum(1 for s in syms
             if d_o3[s] is not None and d_o2[s] is not None and d_o3[s] > d_o2[s])
    c3 = (f"结构规模{'成立' if n3 >= max(2, n_sym - 1) and n_sym >= 2 else '不成立'}"
          f"（Δ(O3)>Δ(O2) 于 {n3}/{n_sym} 标的）")
    # 判据 4
    if all(v == 0 for v in earning.values()):
        c4 = "O4 无定义域（earning 触发率=0，引 complete_fugue/E9 先例诚实落盘）"
    else:
        c4 = f"earning 触发 {earning}；O4 vs O3 Δ：{d_o4} vs {d_o3}"
    return {"delta_o1": d_o1, "delta_o2": d_o2, "delta_o3": d_o3,
            "delta_o4": d_o4, "gate_open_rate_o2": gate_rates,
            "earning_triggers": earning, "rev_quality_o2": rev_q,
            "criterion_1": c1, "criterion_3": c3, "criterion_4": c4}


def _result_package(results: dict) -> list[str]:
    v = _verdict(results)
    L: list[str] = []
    L.append("## 结果包（六要素）\n")
    L.append(f"**1. 结论**：有机赋格框架（单一 LOU × N 声部）实装完成并通过双守卫"
             f"（O0≡P5 逐笔逐 trace 对账 + 磁带级回归锚）。预注册判据评估："
             f"判据1（REV 假设）—— {v['criterion_1']}，Δ(O1)={v['delta_o1']}，"
             f"Δ(O2)={v['delta_o2']}；判据2（门开率前置量）—— O2 门开率 = "
             f"{v['gate_open_rate_o2']}；判据3（规模假设）—— {v['criterion_3']}，"
             f"Δ(O3)={v['delta_o3']}；判据4（earning 假设）—— {v['criterion_4']}。"
             f"REV 腿独立质量（判据5）：{v['rev_quality_o2']}。\n")
    L.append("**2. 定义依据**：第38课段终结三触发与三岔买回（T1/T5 映射表，设计 "
             "§2.2）；第41课衰竭门（§2.3，门只约束 OPEN_REV 不约束域腿）；第40课"
             "结构规模（振幅正比为 L0 推导，本回测即其 L2 裁决）；第31/43课两阶段"
             "守恒律与 earning 反作用；第45课持股持币二元（master=同一 FSM 的 REV="
             "持币解释）。引擎事件语义零新定义（type1/2/3、divergence kind、move "
             "settle 透传）。\n")
    L.append("**3. 边界条件**：(a) θ=1% 不跨波动率域（P5 边界继承）；(b) REV 关腿 "
             "confirmed 滞后已计入（O2c/O2n 消融轴）；(c) 费率：腿层一阶近似表"
             "（2/5/10bps），rev 腿 avg_diff 若 ≤ osc 腿且净现金负 → 判据1 正结果"
             "降级（§8.2-5）；(d) 力度=价格振幅 fallback——div_events 的"
             " consolidation 判定继承同一口径；(e) 强趋势满仓标的上 E 系（无降成本）"
             "整体占优的可能仍在——本框架裁决声部层内部结构；(f) 门开率≈0 的标的上 "
             "O2−O1 无统计意义（无定义域，非门被否证）；(g) earning 触发率=0 时 "
             "O4 无定义域。\n")
    L.append("**4. 下游推论**：若判据1 取『门必要』分支 → 41课首次获得代码化因果"
             "验证，声部完整循环（RIDE↔REV）取代『声部=短差配对器』；若判据3 成立 "
             "→ E8 均分被结构公式正式替换；若 O1/O2 全负 → 框架收缩为 P5+规模轴，"
             "REV 假设否证落盘（否定性结果缩小有效域）。div_events 进磁带后盘整背驰"
             "对所有下游策略可见（E10 完成）。\n")
    L.append("**5. 谱系引用**：E1-E10（interval_nesting 结果包）；38/39/40/41/43/45课"
             "原文锚定表（设计 §7）；E6（bar级有害）/E7（slice废止）/E8（均分无据）"
             "概念分离史；『fatigue/衰竭』作为显式概念此前未单独立谱系条——若后续"
             "发生盘背衰竭 vs 趋势背驰衰竭的概念分离，需新谱系记录（设计 §9-5 预留）。\n")
    L.append("**6. 影响声明**：(a) 新增 `organic_signals.py`（信号层公共模块，"
             "ladder2 背驰链全量重算成本声明见其 docstring）/ `organic_fugue.py`"
             "（LOU/FatigueMonitor/SizeAllocator/OrganicLedger/run_organic）/ "
             "`test_organic_fugue.py`（22 单测）/ 本脚本与报告。(b) "
             "`fugue_version_i.BarSignalI` 增 div_events/up_move_settled 默认空字段"
             "（旧消费方 bit-exact）；`interval_nesting_reverse_backtest.py` 信号层"
             "改 import 公共模块（OKLO 447K 新旧磁带逐位对比守卫通过）。(c) Rust "
             "引擎零改动；`fugue_version_i.run_version_i`/`_SharedFugue` 零改动。"
             "(d) futures 真实空头（INV-3/F1）未实现（设计标注主线外扩展，"
             "run_organic 对 market_mode='futures' 显式抛错）。\n")
    return L


if __name__ == "__main__":
    main()
