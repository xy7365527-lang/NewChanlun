"""BTC 杠杆控制变量实验 — 在 v3 回测基础上对比降杠杆/止损/组合三类修正方案。

控制变量：数据 (BTC 1min 1.8M bars)、信号引擎 (BiEngine + PH 树)、降成本 (CR full)、
手续费 (0.1%/边) 全部相同。唯一变化的自变量：杠杆 leverage、强平线 liq_move、单笔止损 stop。

强平线统一口径：v3 的 10x 用 liq_move=0.05 = "50% 保证金损失即强平"。本实验保持此规则，
按杠杆缩放：liq_move = 0.5 / leverage（2x→0.25, 3x→0.167, 5x→0.10, 10x→0.05）。
这样杠杆是方案A的唯一自变量。

止损口径：stop_pct_of_margin = 单笔亏损达到保证金的此比例时强制平仓（先于强平触发）。
保证金 = INITIAL_CAPITAL，名义 = capital*leverage，故触发反向移动 = stop_pct / leverage。

认识论等级：L2（真实标的 1min 数据，含杠杆/强平/止损模型，可产生否定性结果）。
"""

from __future__ import annotations

import sys
import time
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass, replace as dc_replace
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT))

import analysis.full_system_backtest_v3 as v3

OUTPUT_MD = ROOT / "analysis" / "btc_leverage_experiment.md"
BTC_FILE = "btc_1m_3year.json"


@dataclass(frozen=True)
class Scenario:
    key: str
    group: str          # "baseline" | "A" | "B" | "C"
    label: str
    leverage: float
    stop_pct: float     # 0 = 无止损；否则为保证金损失比例
    note: str

    @property
    def liq_move(self) -> float:
        # 统一口径：50% 保证金损失即强平 → liq_move = 0.5 / leverage
        return 0.5 / self.leverage


SCENARIOS = [
    Scenario("baseline_10x", "baseline", "基线 10x (v3)", 10.0, 0.0,
             "v3 原始配置：10x 杠杆，强平线 5% 反向 (=50% 保证金)"),
    Scenario("A_2x", "A", "降杠杆 2x", 2.0, 0.0,
             "强平线 25% 反向 (=50% 保证金)"),
    Scenario("A_3x", "A", "降杠杆 3x", 3.0, 0.0,
             "强平线 16.7% 反向 (=50% 保证金)"),
    Scenario("A_5x", "A", "降杠杆 5x", 5.0, 0.0,
             "强平线 10% 反向 (=50% 保证金)"),
    Scenario("B_stop30", "B", "10x + 止损30%", 10.0, 0.30,
             "10x 不变，单笔亏损达保证金 30% 强平 (=3% 反向，先于强平线)"),
    Scenario("C_3x_stop20", "C", "3x + 止损20%", 3.0, 0.20,
             "3x，单笔亏损达保证金 20% 止损 (=6.7% 反向)"),
    Scenario("C_5x_stop15", "C", "5x + 止损15%", 5.0, 0.15,
             "5x，单笔亏损达保证金 15% 止损 (=3% 反向)"),
]


def _run_one(sc: Scenario) -> dict:
    """单个 worker：加载数据 + 跑 run_fsm。每个 worker 独立加载以保证 fork/spawn 鲁棒。"""
    _, dates, opens, highs, lows, closes = v3.load_columnar(BTC_FILE)
    n = len(closes)
    btc = next(a for a in v3.ASSETS if a.symbol == "BTC")
    cfg = dc_replace(btc, leverage=sc.leverage, liq_move=sc.liq_move)
    bar_dirs = [btc.default_dirs] * n

    t = time.time()
    trades = v3.run_fsm(cfg, opens, highs, lows, closes, bar_dirs,
                        stop_pct_of_margin=sc.stop_pct)
    elapsed = time.time() - t
    m = v3.compute_metrics(trades)

    bh = (closes[-1] - closes[0]) / closes[0] * 100.0
    return {
        "key": sc.key, "group": sc.group, "label": sc.label,
        "leverage": sc.leverage, "liq_move": sc.liq_move, "stop_pct": sc.stop_pct,
        "note": sc.note, "n_bars": n, "bh": bh,
        "first_price": closes[0], "last_price": closes[-1],
        "elapsed": elapsed, "metrics": m,
    }


def _fmt_pct(x: float) -> str:
    if x <= -99.99:
        return "-100% (爆仓)"
    return f"{x:+.1f}%"


def write_report(results: list[dict], total_elapsed: float) -> None:
    by_key = {r["key"]: r for r in results}
    ordered = [by_key[sc.key] for sc in SCENARIOS]
    base = by_key["baseline_10x"]
    bh = base["bh"]

    L: list[str] = []
    L.append("# BTC 杠杆控制变量实验\n")
    L.append("> 在 `analysis/full_system_backtest_v3.py` 回测基础上，对比三类杠杆修正方案。\n")

    L.append("## 实验设计（控制变量）\n")
    L.append("**不变量**：数据 (BTC 1min) · 信号引擎 (BiEngine+PH) · 降成本 (CR full) · "
             "手续费 (0.1%/边) · 初始资金 ($100K) · 双向交易。\n")
    L.append("**自变量**：杠杆 `leverage` · 强平线 `liq_move` · 单笔止损 `stop_pct`。\n")
    L.append(f"**数据**：{base['n_bars']:,} bars，"
             f"{base['first_price']:,.0f} → {base['last_price']:,.0f}，"
             f"Buy&Hold = {_fmt_pct(bh)}。\n")
    L.append("**强平线统一口径**：保持 v3 的「50% 保证金损失即强平」规则，按杠杆缩放 "
             "`liq_move = 0.5 / leverage`。这样杠杆是方案A的唯一自变量。\n")
    L.append("**止损口径**：`stop_pct` = 单笔亏损达保证金此比例时强制平仓，"
             "触发反向移动 = `stop_pct / leverage`，永远先于强平线。\n")

    L.append("\n## 对比表\n")
    L.append("| 方案 | 杠杆 | 强平线(反向) | 止损 | 复利收益 | vs BH | 胜率 | 交易 | 多/空 | 强平 | 止损 | MDD | CR笔 | 提本 |")
    L.append("|------|------|------------|------|---------|-------|------|------|-------|------|------|-----|------|------|")
    for r in ordered:
        m = r["metrics"]
        stop_desc = f"{r['stop_pct']*100:.0f}%保证金" if r["stop_pct"] > 0 else "—"
        liq_desc = f"{r['liq_move']*100:.1f}%"
        comp = _fmt_pct(m["total_compound"])
        vs_bh = m["total_compound"] - bh
        L.append(
            f"| {r['label']} | {r['leverage']:.0f}x | {liq_desc} | {stop_desc} | "
            f"{comp} | {vs_bh:+.0f}% | {m['win_rate']:.0f}% | {m['n']} | "
            f"{m['long_n']}/{m['short_n']} | {m['n_liq']} | {m['n_stop']} | "
            f"{m['max_dd']:.0f}% | {m['n_with_cr']} | {m['n_pw']} |"
        )
    L.append(f"\n*Buy&Hold 基准：{_fmt_pct(bh)}*\n")

    # 分组解读
    L.append("\n## 分组解读\n")
    for group, title in [("A", "方案A：降杠杆"), ("B", "方案B：加止损"), ("C", "方案C：降杠杆+止损组合")]:
        rs = [r for r in ordered if r["group"] == group]
        L.append(f"\n### {title}\n")
        L.append("| 方案 | 复利 | 强平 | 止损 | 胜率 | MDD |")
        L.append("|------|------|------|------|------|-----|")
        L.append(f"| _基线 10x_ | {_fmt_pct(base['metrics']['total_compound'])} | "
                 f"{base['metrics']['n_liq']} | {base['metrics']['n_stop']} | "
                 f"{base['metrics']['win_rate']:.0f}% | {base['metrics']['max_dd']:.0f}% |")
        for r in rs:
            m = r["metrics"]
            L.append(f"| {r['label']} | {_fmt_pct(m['total_compound'])} | "
                     f"{m['n_liq']} | {m['n_stop']} | {m['win_rate']:.0f}% | {m['max_dd']:.0f}% |")
        for r in rs:
            L.append(f"- **{r['label']}**：{r['note']}")

    # 结论：区分"有方案存活"与"全部失败"两种定性结局
    survivors = [r for r in ordered if r["metrics"]["total_compound"] > -99.99]
    avg_win = sum(r["metrics"]["win_rate"] for r in ordered) / len(ordered)
    avg_long = sum(r["metrics"]["long_n"] for r in ordered) / len(ordered)
    avg_short = sum(r["metrics"]["short_n"] for r in ordered) / len(ordered)
    L.append("\n## 结论\n")
    L.append(f"- **基线 10x**：复利 {_fmt_pct(base['metrics']['total_compound'])}，"
             f"{base['metrics']['n_liq']} 次强平。\n")

    if not survivors:
        # 否定性结果：全部失败 → 问题不在杠杆层，定位到信号层
        L.append("- **否定性结果：7 个方案全部爆仓（复利 -100%）。** 降杠杆/止损/组合无一存活。\n")
        L.append(f"- **诊断：杠杆不是问题，信号才是。** 全方案胜率均值 {avg_win:.0f}%，"
                 f"在 Buy&Hold {_fmt_pct(bh)} 的市场里平均做空 {avg_short:.0f} 笔 vs 做多 {avg_long:.0f} 笔。\n")
        min_liq = min(ordered, key=lambda r: r["metrics"]["n_liq"])
        L.append(f"- **关键反证**：{min_liq['label']} 仅 {min_liq['metrics']['n_liq']} 次强平仍 -100% —— "
                 f"证明 -100% 是逐笔失血（低胜率复利坍缩），非强平级联。\n")
        L.append("- **推论**：杠杆/止损属 position sizing 层；失败在 signal 层（方向+胜率）。"
                 "在信号层修复前，任何 sizing 都不产生正期望。\n")
    else:
        best = max(survivors, key=lambda r: r["metrics"]["total_compound"])
        L.append(f"- **存活方案**（未爆仓，复利 > -100%）：{len(survivors)} 个。\n")
        L.append(f"- **最优方案**：**{best['label']}**，"
                 f"复利 {_fmt_pct(best['metrics']['total_compound'])}，"
                 f"vs BH {best['metrics']['total_compound'] - bh:+.0f}%，"
                 f"强平 {best['metrics']['n_liq']} / 止损 {best['metrics']['n_stop']}，"
                 f"MDD {best['metrics']['max_dd']:.0f}%。\n")

    L.append("\n## 边界条件\n")
    L.append("- 结论会翻转的条件：(1) 改变强平线口径（如改为「100% 保证金即强平」会让降杠杆方案的强平更晚触发）；"
             "(2) 改变止损是否先于强平的优先级；(3) 数据时段（本数据为单一上行/震荡 BTC 时段，换熊市时段方向收益翻转）。\n")
    L.append("- 强平价基于 bar 内最差价（long=low, short=high），可能略悲观——"
             "实际盘口可能在触及最差价前已成交在更优价。\n")
    L.append("- 止损与强平同 bar 触发时，止损价更接近入场（反向移动更小），优先采用止损价。\n")

    L.append("\n## 认识论等级 / 影响声明\n")
    L.append("- **认识论等级**：L2（真实 BTC 1min 数据，含杠杆/强平/止损模型，可产生否定性结果）。"
             "仅单标的单时段，未做 L3 交叉验证。\n")
    L.append("- **影响声明**：本实验在 `full_system_backtest_v3.py` 中新增 `run_fsm` 的可选参数 "
             "`stop_pct_of_margin`（默认 0，不改变现有 4 标的行为）及 `compute_metrics` 的 `n_stop` 统计。"
             "实验脚本 `analysis/btc_leverage_experiment.py` 通过 `dataclasses.replace` 派生 BTC 杠杆变体，"
             "不修改 `ASSETS` 基线配置。\n")
    L.append(f"- 总耗时：{total_elapsed:.1f}s（{len(SCENARIOS)} 方案并行）。\n")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main() -> None:
    t0 = time.time()
    print("=" * 60)
    print("  BTC 杠杆控制变量实验：7 方案并行")
    print("=" * 60)
    for sc in SCENARIOS:
        print(f"  {sc.key:16s} lev={sc.leverage:.0f}x liq={sc.liq_move*100:.1f}% "
              f"stop={sc.stop_pct*100:.0f}%")

    with ProcessPoolExecutor(max_workers=len(SCENARIOS)) as ex:
        results = list(ex.map(_run_one, SCENARIOS))

    print("\n── 结果 ──")
    for r in sorted(results, key=lambda x: -x["metrics"]["total_compound"]):
        m = r["metrics"]
        print(f"  {r['label']:18s} 复利 {m['total_compound']:+8.1f}%  "
              f"强平 {m['n_liq']:2d}  止损 {m['n_stop']:2d}  "
              f"胜率 {m['win_rate']:4.0f}%  交易 {m['n']:3d}  ({r['elapsed']:.0f}s)")

    write_report(results, time.time() - t0)


if __name__ == "__main__":
    main()
