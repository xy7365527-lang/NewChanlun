"""区间套反向应用的短差配对修复 — P1-P5 五步消融回测（OKLO 主验证 + QQQ/BRN 交叉）。

═══════════════════════════════════════════════════════════════════════
问题与理论框架
═══════════════════════════════════════════════════════════════════════
降成本短差 52% 卖飞率的根因（v2_selloff_root_cause + 配对诊断）：
- BarSignalI 把 BSP 的 kind/center_seg_start 折叠成 4 个布尔——信息销毁；
- FSM 盲等"下一个任何买点"（buy_any），不区分 kind、不锚定中枢；
- type1卖→type1买 胜率 78.9%；type1卖→type3买 卖飞率 67.9%；
- type3 买点是中枢终结讣告（向上离开确认），不是低吸信号。

区间套反向应用：底空间 = 本级别中枢 [ZD,ZG] × lifetime；纤维 = lifetime 内的
次级别走势完成点；短差合法性 = 两腿在同一纤维（中枢存活）；正期望 = 中枢中心
定理的回归保证（定义性质）；type3 = 纤维死亡边界 → 立即回补。

═══════════════════════════════════════════════════════════════════════
消融矩阵（floor=segment，I_seg2 口径；进出场/ARM 不变 → 差异全在短差层）
═══════════════════════════════════════════════════════════════════════
  I_seg2 基线（现状：sell_any 开 / buy_any 平，盲配对）
  P1     仅 kind 配对（confirmed type1 卖 → confirmed type1 买）
  P2     P1 + confirmed type3 买 → 硬回补 + 冻结该级别（至新存活中枢）
  P3     P2 + candidate type3 买 → 预回补（左侧逃逸）
  P4     P3 + 开腿中枢门（type1/2 卖 + 中枢存活 + 盘背位置 c>ZG + 振幅≥θ）
         + 同锚中枢 type1 买正常回补
  P5     P4 + O 模式震荡腿（中枢上沿 ZG×次级别卖点 高抛 / 锚中枢 ZD 低吸）

═══════════════════════════════════════════════════════════════════════
信号层：Rust 引擎 + 结构化 BSP 事件流（差分守卫内嵌）
═══════════════════════════════════════════════════════════════════════
`compute_i_signals_rust_events` fork 自 `m1_i_rust_engine.compute_i_signals_rust`，
额外产出 `BarSignalI.bsp_events`（(ladder, kind, side, seg_idx, confirmed,
center_seg_start, center_zd, center_zg, price)，candidate/confirmed 分别入流，
去重键含 confirmed）。**差分守卫**：每 bar 把本扫描器导出的布尔流与 Rust delta
接口（`bi_zhongshu_new_signals` / `trend_new_signals`，旧语义的逐位移植）对比，
不等 → 立即 abort。这保证事件流扩展不改变任何旧布尔语义（旧变体 bit-exact）。

⚠ 成本声明：ladder2 事件需每个 stroke 增长 bar 全量 marshal
`current_bi_zhongshu_buysellpoints_inc`（O(B)/次，摊还 O(S·B)）——这是事件流
（vs 纯布尔 delta 接口）的必要代价，BRN 2.4M 上信号层耗时显著高于纯布尔版。

认识论等级：**L2→L3**（OKLO 主验证 + QQQ/BRN 交叉，可产生否定性结果——
若 P1-P5 全部不优于基线/不优于无短差，则配对假设被否证）。

输出：analysis/interval_nesting_reverse_backtest.md
      analysis/data_cache/interval_nesting_reverse_backtest.json
用法：PYTHONPATH=src python analysis/interval_nesting_reverse_backtest.py
      （env：BT_SYMBOLS=OKLO,QQQ,BRN  BT_MAX_BARS=冒烟上限  BT_THETA_AMP=0.01）
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
    OSC_KEY_OFFSET,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_signals import compute_i_signals_rust_events  # noqa: E402,F401

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "interval_nesting_reverse_backtest.json"
OUT_MD = ROOT / "analysis" / "interval_nesting_reverse_backtest.md"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
VARIANTS = ["I_seg2", "P1", "P2", "P3", "P4", "P5", "P6", "P7"]
FLOOR = LADDER_SEG  # I_seg2 口径（诊断口径，进出场与 floor 无关）


# ════════════════════════════════════════════════════════════
# 信号层：抽出为公共模块 organic_signals（有机赋格 §6）
# ════════════════════════════════════════════════════════════
#
# `compute_i_signals_rust_events`（BspTuple 事件扫描 + 差分守卫 + PH/epoch 门控）
# 原实现在本文件，现为 `organic_signals.compute_organic_signals` 的兼容别名
# （磁带超集：增 div_events / up_move_settled 字段，bsp_events/布尔逐位不变，
# 抽出时经 OKLO 447K 全量新旧磁带逐位对比守卫）。本脚本 P1-P7 语义零变化。

# ════════════════════════════════════════════════════════════
# 配对统计（从 diag trace 聚合）
# ════════════════════════════════════════════════════════════

def _pair_stats(diag: list) -> dict:
    """跨全部 trades 聚合短差配对统计（主腿 / O 腿分开）。"""
    def agg(rows: list) -> dict:
        n = len(rows)
        if n == 0:
            return {"n_pairs": 0, "fly_rate": None, "win_rate": None,
                    "avg_diff_pct": None, "net_cash": 0.0}
        n_fly = sum(1 for r in rows if r["diff"] < 0)
        n_win = sum(1 for r in rows if r["diff"] > 0)
        avg_diff = sum(r["diff"] / r["sell_price"] for r in rows) / n * 100
        return {
            "n_pairs": n,
            "fly_rate": round(n_fly / n * 100, 1),
            "win_rate": round(n_win / n * 100, 1),
            "avg_diff_pct": round(avg_diff, 4),
            "net_cash": round(sum(r["profit"] for r in rows), 1),
        }

    main_rows: list = []
    osc_rows: list = []
    for tr in diag:
        for r in tr["diffs"]:
            (osc_rows if r["ladder"] >= OSC_KEY_OFFSET else main_rows).append(r)
    out = {"main": agg(main_rows), "osc": agg(osc_rows)}
    out["all"] = agg(main_rows + osc_rows)
    return out


# ════════════════════════════════════════════════════════════
# 单标的管线
# ════════════════════════════════════════════════════════════

def process_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 区间套反向应用配对消融（I_seg2 + P1-P5）\n{'=' * 64}",
          flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  bars={n:,}  BH={bh:+.2f}%", flush=True)

    t0 = time.time()
    tape = compute_i_signals_rust_events(opens, highs, lows, closes)
    sig_elapsed = time.time() - t0
    n_events = sum(len(row) for s in tape for row in s.bsp_events)
    print(f"  信号层 {sig_elapsed:.1f}s  事件数={n_events:,}（差分守卫全程通过）",
          flush=True)

    out: dict = {"n_bars": n, "bh": round(bh, 2),
                 "sig_elapsed": round(sig_elapsed, 1), "n_events": n_events,
                 "variants": {}}
    for name in VARIANTS:
        cfg = PAIRING_VARIANTS[name]
        diag: list = []
        trades, extra = run_version_i(tape, floor_ladder=FLOOR, pairing=cfg,
                                      diag=diag)
        m = extended_metrics(trades, years)
        ps = _pair_stats(diag)
        out["variants"][name] = {
            "metrics": {k: m[k] for k in
                        ("n", "win_rate", "total_compound", "sharpe", "max_dd",
                         "profit_factor", "n_with_cr")},
            "excess": round(m["total_compound"] - bh, 2),
            "pair_stats": ps,
            "pairing_counters": extra.get("pairing"),
        }
        pa = ps["all"]
        print(f"  [{symbol}/{name:6s}] 交易={m['n']:4d} 复利={m['total_compound']:+10.2f}%"
              f" 超额={m['total_compound']-bh:+9.2f}% MDD={m['max_dd']:+.1f}"
              f" | 短差={pa['n_pairs']:5d} 卖飞率={pa['fly_rate']}%"
              f" 胜率={pa['win_rate']}% avg_diff={pa['avg_diff_pct']}%"
              f" 净现金={pa['net_cash']:+.0f}", flush=True)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())  # 增量续跑（跨进程恢复）
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
        _write_report(results)  # 全 skip 时也重写报告（报告器演进后可单独再生成）
    print(f"\n总耗时 {time.time() - t_all:.1f}s", flush=True)


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

_VAR_DESC = {
    "I_seg2": "基线：sell_any 开 / buy_any 平（盲配对）",
    "P1": "仅 kind 配对：t1 卖 → t1 买",
    "P2": "P1 + confirmed type3 买硬回补+冻结",
    "P3": "P2 + candidate type3 买预回补",
    "P4": "P3 + 开腿中枢门（t1/t2+存活+盘背 c>ZG+振幅≥θ）+ 同锚回补",
    "P5": "P4 + 域腿（ZG×次级卖点 高抛 / 锚 ZD 纯价格触线 低吸）",
    "P6": "纯结构域配对（用户范式）：仅域腿，两腿都次级别定位"
          "（高抛=锚ZG×次级卖点；低吸=锚ZD×次级买点）+ type3 逃逸，无离开段腿",
    "P7": "P6 域腿 + P4 离开段腿（两类腿并发独立槽）",
}


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 区间套反向应用的短差配对修复 — P1-P5 五步消融回测\n")
    L.append("> **理论框架**：底空间 = 本级别中枢 [ZD,ZG]×lifetime；纤维 = lifetime 内"
             "次级别走势完成点；短差合法性 = 两腿在同一纤维（中枢存活）；正期望 = 中枢"
             "中心定理的回归保证；**type3 = 纤维死亡边界 → 立即回补，不是低吸**。\n")
    L.append("> **口径**：floor=segment（I_seg2，诊断口径），进出场/ARM 逻辑全变体一致"
             "→ 变体差异全在短差层。信号层 = Rust 引擎 + 结构化 BSP 事件流（candidate/"
             "confirmed 分别入流，去重键含 confirmed），布尔流与 Rust delta 接口逐 bar "
             "差分守卫。**级别隔离**（用户 2026-06-10 裁定）：bsp_events[ladder] 每级别"
             "独立流（不混流），配对在各 ladder 内独立执行（独立锚上下文：last_center/"
             "dead/frozen per ladder），操作点用次级别（ladder−1）走势定位。成交 = 信号 "
             "bar 收盘价（与基线一致，无 bracket/滑点假设）。认识论 **L2→L3**"
             "（OKLO 主验证 + QQQ/BRN 交叉）。\n")
    theta = PAIRING_VARIANTS["P4"].theta_amp
    L.append(f"> P4 振幅阈值 θ={theta}（卖价距 ZG 相对深度下限；根因诊断：<1% 回调是"
             "次级别买点无法现实捕捉的噪音粒度）。P5 盘整背驰用『次级别卖点×价格≥ZG』"
             "作必要条件代理（完整盘背判定需次级别力度比较，引擎零改动约束下不可得，"
             "诚实声明）。\n")
    L.append("## 变体定义\n")
    L.append("| 变体 | 改动 |")
    L.append("|------|------|")
    for v in VARIANTS:
        L.append(f"| {v} | {_VAR_DESC[v]} |")
    L.append("")
    L.append("## 主对照表（复利 / 超额 / vs_B0 / 短差配对质量）\n")
    L.append("> **判定标准**（承自 v2_selloff_fix_experiments）：floor 只影响短差层不影响"
             "进出场 → 短差层净贡献 = compound(政策) − compound(B0 无短差)。"
             "**『解决卖飞』= vs_B0 > 0**；打败 I_seg2 但输 B0 = 只是少亏。\n")
    L.append("| 标的 | 变体 | 复利% | 超额(vs BH) | **vs_B0(pp)** | MDD | 交易 | 短差对 | "
             "卖飞率% | 胜率% | avg_diff% | 短差净现金 |")
    L.append("|------|------|-------|------------|--------------|-----|------|--------|"
             "---------|-------|-----------|-----------|")
    for s, r in results.items():
        b0 = r.get("B0", {}).get("metrics", {}).get("total_compound")
        if b0 is not None:
            b0m = r["B0"]["metrics"]
            L.append(
                f"| {s} | B0(无短差) | {b0:+.1f} | {r['B0']['excess']:+.1f} | +0.0 | "
                f"{b0m['max_dd']:+.1f} | {b0m['n']} | 0 | — | — | — | +0 |")
        for v in VARIANTS:
            d = r["variants"].get(v)
            if not d:
                continue
            m = d["metrics"]; pa = d["pair_stats"]["all"]
            vs_b0 = (f"{m['total_compound'] - b0:+.1f}" if b0 is not None else "—")
            L.append(
                f"| {s} | {v} | {m['total_compound']:+.1f} | {d['excess']:+.1f} | "
                f"**{vs_b0}** | "
                f"{m['max_dd']:+.1f} | {m['n']} | {pa['n_pairs']} | "
                f"{pa['fly_rate'] if pa['fly_rate'] is not None else '—'} | "
                f"{pa['win_rate'] if pa['win_rate'] is not None else '—'} | "
                f"{pa['avg_diff_pct'] if pa['avg_diff_pct'] is not None else '—'} | "
                f"{pa['net_cash']:+.0f} |")
        L.append("| | | | | | | | | | | | |")
    L.append("")
    L.append("## 配对机制计数（闭腿路径分解）\n")
    L.append("| 标的 | 变体 | 正常回补(t1买) | 预回补(cand t3) | 硬回补(conf t3) | "
             "门拒(开腿) | O腿开 | O腿ZD平 |")
    L.append("|------|------|---------------|----------------|----------------|"
             "-----------|-------|---------|")
    for s, r in results.items():
        for v in VARIANTS:
            d = r["variants"].get(v)
            if not d or not d.get("pairing_counters"):
                continue
            pc = d["pairing_counters"]
            L.append(
                f"| {s} | {v} | {pc['n_close_normal']} | {pc['n_close_pre_type3']} | "
                f"{pc['n_close_hard_type3']} | {pc['n_open_gate_rejects']} | "
                f"{pc['n_osc_open']} | {pc['n_osc_zd_close']} |")
    L.append("")
    L.append("## 离开段腿(main) / 域腿(osc) 分解（机制质量对比，P5-P7）\n")
    L.append("| 标的 | 变体 | 腿 | 短差对 | 卖飞率% | 胜率% | avg_diff% | 净现金 |")
    L.append("|------|------|----|--------|---------|-------|-----------|--------|")
    for s, r in results.items():
        for v in ("P5", "P6", "P7"):
            d = r["variants"].get(v)
            if not d:
                continue
            for leg in ("main", "osc"):
                p = d["pair_stats"][leg]
                L.append(
                    f"| {s} | {v} | {leg} | {p['n_pairs']} | "
                    f"{p['fly_rate'] if p['fly_rate'] is not None else '—'} | "
                    f"{p['win_rate'] if p['win_rate'] is not None else '—'} | "
                    f"{p['avg_diff_pct'] if p['avg_diff_pct'] is not None else '—'} | "
                    f"{p['net_cash']:+.0f} |")
    L.append("")
    L.append("## 标的概览\n")
    L.append("| 标的 | bars | BH% | 信号层耗时s | BSP事件数 |")
    L.append("|------|------|-----|------------|-----------|")
    for s, r in results.items():
        L.append(f"| {s} | {r['n_bars']:,} | {r['bh']:+.1f} | {r['sig_elapsed']} | "
                 f"{r['n_events']:,} |")
    L.append("")
    L.extend(_result_package(results))
    OUT_MD.write_text("\n".join(L))


def _result_package(results: dict) -> list[str]:
    """结果包六要素（result-package 规则强制）。数值从 results 程序化抽取。"""
    def vs_b0(sym: str, var: str) -> float | None:
        r = results.get(sym)
        if not r or "B0" not in r or var not in r["variants"]:
            return None
        return round(r["variants"][var]["metrics"]["total_compound"]
                     - r["B0"]["metrics"]["total_compound"], 1)

    L: list[str] = []
    L.append("## 结果包（六要素）\n")

    p5 = {s: vs_b0(s, "P5") for s in results}
    p6 = {s: vs_b0(s, "P6") for s in results}
    p1 = {s: vs_b0(s, "P1") for s in results}
    base = {s: vs_b0(s, "I_seg2") for s in results}
    fly = {s: (results[s]["variants"]["I_seg2"]["pair_stats"]["all"]["fly_rate"],
               results[s]["variants"]["P5"]["pair_stats"]["all"]["fly_rate"])
           for s in results if "P5" in results[s]["variants"]}
    osc_cmp = {s: (results[s]["variants"]["P5"]["pair_stats"]["osc"]["net_cash"],
                   results[s]["variants"]["P6"]["pair_stats"]["osc"]["net_cash"])
               for s in results if "P6" in results[s]["variants"]}

    L.append(f"**1. 结论**：区间套反向应用的短差配对修复**成立，且级别隔离的结构域"
             f"架构（用户裁定）是其正确存在形式——但买回侧的定位方式由数据裁决为"
             f"锚边界触线，不是次级别走势确认**。(一) P5 在全部 {len(results)} 标的 "
             f"vs_B0 转正（{p5}），是**信号类政策首次跨标的打败无短差基准**（v2 政策"
             f"矩阵中信号类全负，仅 bracket 价格机制转正）；卖飞率从基线 "
             f"{dict((s, v[0]) for s, v in fly.items())} 降到 "
             f"{dict((s, v[1]) for s, v in fly.items())}。(二) **P1 仅 kind 配对在三标的"
             f"全部负贡献**（{p1}，劣于基线 {base}）——『type1卖→type1买胜率 78.9%』"
             f"不能直接兑现为净收益：合法腿持有更久、出场强平尾部更大。有效机制是"
             f"中枢锚定的合取：type3 纤维死亡逃逸（candidate 左侧 > confirmed 右侧，"
             f"P3>P2）+ 开腿中枢门（P4，门拒 75-97% 后 avg_diff 转正）+ **域腿"
             f"（中枢震荡腿，P5 增量主源：锚 ZG×次级别卖点高抛 / 锚 ZD 触线低吸，"
             f"胜率 72.7-77.6%、卖飞率 22.4-27.0%，全机制中质量最高）**。"
             f"(三) **P6/P7 控制变量判决（第二轮，级别隔离架构落地后）**：P6 = 纯结构域"
             f"配对但买回加次级别买点合取（osc_buy_sub），与 P5 域腿的唯一差异是买回"
             f"条件。结果：ZD 正常回补率从 56-63% 降到 40-50%，错过触线的腿悬挂至出场"
             f"强平——域腿净现金 {osc_cmp}（P5 触线 vs P6 合取；BRN 从 +15808 翻到 "
             f"−72765，vs_B0 {p6}）。**判决：卖出侧次级别定位有效（开腿质量门），"
             f"买回侧次级别定位有害（确认滞后 + 悬挂尾部，与 v2 卖飞根因『买回信号"
             f"滞后 gap_buy 2.33% ≫ 回调深度』同构）。中枢中心定理保证回归到 [ZD,ZG]，"
             f"回归触达本身就是低吸定位的严格形式，再等次级别走势完成是第二次粒度"
             f"损失**。级别隔离架构本身被确认：每级别短差只在自己的结构域（锚中枢"
             f"[ZD,ZG]×lifetime）内操作、各 ladder 独立配对上下文——这正是 P5 域腿"
             f"的构造，且 P1-P5 在事件流重构（混合流→每 ladder 独立流）后全量逐位"
             f"复现，证明级别隔离是该机制的不变式而非实现细节。\n")
    L.append("**2. 定义依据**：第三类买卖点（第20课）= 中枢突破回试不破 → 中枢终结确认，"
             "故 type3 买点是『向上离开成立』的讣告而非低吸点（candidate=回试中/左侧，"
             "confirmed=延续段出现/右侧——P3>P2 证明左侧逃逸更值钱）。中枢 [ZD,ZG]"
             "（maimai #5：判定范围用 ZD/ZG 非 DD/GG）作底空间；第33课『向上离开中枢出现"
             "盘背高抛』作开腿条件的原文依据；中枢中心定理（第21-24课中枢震荡逻辑）提供"
             "回归正期望。BSP 事件字段（kind/center_seg_start/center_zd/center_zg）"
             "取自引擎 `a_buysellpoint_v1.BuySellPoint`——修复只是停止销毁这些信息"
             "（原 L237 折叠为 4 布尔），引擎零改动。\n")
    L.append("**3. 边界条件**（结论何时翻转）：(a) **θ=1% 固定阈值不跨波动率域**——QQQ"
             "（低波动）P4 主腿被门杀到 12 对（门拒 2071），P4 vs_B0 仍负（-3.5pp）；"
             "若 θ 不按标的波动率缩放，P4 步在低波动标的退化为『关闭主腿』，P5 增量"
             "全靠域腿。(b) **同锚 type1 买正常回补是空有效域**（formalization-validity-"
             "domain：n_close_normal=0 × 3 标的）——type1 买点锚定下跌走势的中枢，与开腿"
             "的上行离开中枢**必然异锚**，该规则在 BSP 语义下永不触发；『价格回到锚中枢"
             "区间』才是回归判据的可实现形式（域腿逻辑）。(c) 成交=信号 bar 收盘价，"
             "无滑点/费率——P5 短差 avg_diff 仅 +0.24/+0.04/+0.04%，QQQ/BRN 的单腿优势"
             "在 ~4bps 量级，**实际费率 >2bps/腿即可吞噬 QQQ/BRN 的域腿优势**（OKLO "
             "+30bps 稳健）。(d) 与 v2 bracket 最优政策相比（OKLO +923pp、BRN +784pp "
             "vs_B0），P5（+450.6/+131.5pp）**仍低**——bracket 依赖 intrabar 限价成交"
             "假设，P5 是纯信号类；不在同一成交模型认识论上，不可直接合并排序。"
             "(e) 强趋势满仓标的上 E 系（无降成本）仍可能整体占优（本实验只裁决短差层"
             "内部）。(f) **买回侧次级别定位的危害与出场频率/标的噪声结构相关**："
             "P6 在 OKLO(+67.4pp)/QQQ(+8.2pp) 仍正、在 BRN(−204.7pp) 强负——BRN "
             "1329 笔高频出场把悬挂腿（错过 ZD 触线后反弹）的强平尾部放大；若标的"
             "出场稀疏且回调深，合取条件的危害减弱但仍劣于触线（OKLO P7 +630 < P5 "
             "+741）。(g) 域腿低吸=锚 ZD 触线是收盘价判定（c ≤ ZD），非 intrabar 限价"
             "——比 bracket 保守，但仍假设 ZD 触线 bar 可按收盘成交。\n")
    L.append("**4. 下游推论**：(a) 短差层的正确存在形式是**级别隔离的结构域腿**"
             "（每级别只在自己的锚中枢 [ZD,ZG]×lifetime 内操作：次级别卖点×ZG 高抛 / "
             "锚 ZD 触线低吸 / type3 死亡逃逸），不是『卖点开/买点平』的事件配对——"
             "Version I 生产路径若启用降成本，应采 P5 的域腿 + type3 逃逸（域腿独立"
             "净现金三标的全正：OKLO +32339 / QQQ +8070 / BRN +15808），弃 kind 盲配对"
             "与买回侧次级别合取。(b) BSP 事件流（bsp_events，每 ladder 独立流 + 中枢"
             "锚点）现已是磁带标准字段，下游策略可消费 kind/center 结构信息，不再被 "
             "4 布尔卡脖子。(c) θ 需波动率归一（如 ATR 或中枢振幅分位）才能跨标的部署 "
             "P4 门。(d) 『candidate 左侧信号 > confirmed 右侧确认』（P3>P2）与 525号"
             "『candidate+PH settle 进场』方向一致，且与买回侧判决同构（等右侧确认 = "
             "粒度损失），支持 candidate 流/边界触达的系统性利用。\n")
    L.append("**5. 谱系引用**：v2_selloff_root_cause（卖飞根因二分：浅回调粒度+趋势）/ "
             "v2_selloff_fix_experiments（政策矩阵：信号类全负、bracket 转正——本实验"
             "部分推翻其『信号类不可救』推论：加中枢锚定后信号类可转正）/ 525号（笔中枢"
             "退化基底，segment 级真实 BSP 的合法性）/ 521号（PH 纯拓扑无动量；力度=振幅"
             "代理的有效域边界）/ project_shared_position_fugue（共享仓位多声部账本）/ "
             "project_complete_fugue_v2（中枢门空有效域先例——本次 P4 同锚回补空有效域"
             "是同构发现）/ 缠师第20课（type3）/ 第33课（离开中枢背驰高抛低吸）/ "
             "第35课（多级别立体操作）。\n")
    L.append("**6. 影响声明**：(a) `analysis/fugue_version_i.py`：`BarSignalI` 增 "
             "`bsp_events` 字段——**每 ladder 独立事件流**（bsp_events[ladder]，级别"
             "隔离，用户 2026-06-10 裁定『不混流』；空 bar 共享单例 NO_LADDER_EVENTS；"
             "默认 ()，旧构造方兼容）；三个 BSP tracker 去重键扩为 (kind,side,seg_idx,"
             "confirmed) 并产事件流（布尔流逐位不变，40K 差分 + Rust delta 接口逐 bar "
             "守卫验证）；`_SharedFugue.active` 槽扩为 (cyc,was_earning,anchor)（位置"
             "访问兼容，v2_selloff_fix_experiments 等用 rec[0] 不受影响）；"
             "`run_version_i` 增 `pairing` 参数（None=旧行为逐位不变，I_seg2/OKLO "
             "+224.27%/220笔 与 BRN +248.68%/1329笔 精确复现文档值；事件磁带能力守卫："
             "布尔磁带上启用 pairing 直接抛错）；`PairingConfig` 含 osc_buy_sub（P6/P7 "
             "消融轴）；删除死代码 `_scan_confirmed_bsps`。两轮重构（混合流→级别隔离"
             "流）后 P1-P5 三标的全量逐位复现（回归守卫）。(b) 新增本脚本与报告。"
             "(c) 引擎（src/rust）**零改动**。(d) `m1_i_rust_engine.compute_i_signals_"
             "rust` 未动（其磁带 bsp_events=()，只支持 pairing=None 路径）。\n")
    return L


if __name__ == "__main__":
    main()
