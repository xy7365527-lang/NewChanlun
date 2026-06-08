"""分段 regime 回测 — 多级别 PH FSM (1min QQQ)

复用 fugue_multilevel_ph_backtest_1min.py 的完整 run_backtest() 引擎，
对 QQQ 728K bars 按市场 regime 切分后独立回测。

每个 regime 从零初始化（fresh PH trees + fresh FSM），
验证策略在不同市场环境下的独立表现。

数据时间范围估算：~Oct 2022 至 ~Jun 2026（836 bars/day, extended hours）。
数据不包含 2020 COVID 暴跌（QQQ 低至 $163）和 2021 牛市。

认识论等级：L2（真实数据单标的分段验证；可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

sys.path.insert(0, str(ROOT / "analysis"))
from fugue_multilevel_ph_backtest_1min import (
    run_backtest,
    compute_metrics,
    CompletedTrade,
    INITIAL_CAPITAL,
)

DATA_PATH = ROOT / "analysis" / "data_cache" / "qqq_1m_databento_full.json"
OUTPUT_MD = ROOT / "analysis" / "fugue_segment_backtest_1min.md"


@dataclass(frozen=True)
class Regime:
    name: str
    bar_start: int
    bar_end: int
    description: str


def detect_regimes(closes: list[float]) -> list[Regime]:
    """从价格数据中检测 regime 边界。

    使用关键价格水平和重大回撤作为分界点，而非日历日期。
    """
    n = len(closes)

    # --- 关键标志点 ---
    min_idx = closes.index(min(closes))  # ~bar 2,661, $260

    # 找到 -25.6% 回撤的峰和谷
    # 峰: $541.18 at ~bar 440,767
    # 谷: $402.75 at ~bar 470,586
    bear_peak_idx = 0
    bear_peak_val = 0.0
    for i in range(min(500000, n)):
        if closes[i] > bear_peak_val:
            bear_peak_val = closes[i]
            bear_peak_idx = i

    bear_trough_idx = bear_peak_idx
    bear_trough_val = bear_peak_val
    for i in range(bear_peak_idx, min(bear_peak_idx + 100000, n)):
        if closes[i] < bear_trough_val:
            bear_trough_val = closes[i]
            bear_trough_idx = i

    bear_dd = (bear_trough_val - bear_peak_val) / bear_peak_val * 100
    if bear_dd > -10:
        bear_peak_idx = 0
        bear_trough_idx = 0

    # 找到回撤恢复到峰值价格的位置
    recovery_idx = n - 1
    if bear_peak_idx > 0:
        for i in range(bear_trough_idx, n):
            if closes[i] >= bear_peak_val:
                recovery_idx = i
                break

    # 找到恢复后的第二次重大回撤（如果有）
    second_bear_peak_idx = 0
    second_bear_peak_val = 0.0
    for i in range(recovery_idx, n):
        if closes[i] > second_bear_peak_val:
            second_bear_peak_val = closes[i]
            second_bear_peak_idx = i

    second_bear_trough_val = second_bear_peak_val
    second_bear_trough_idx = second_bear_peak_idx
    for i in range(second_bear_peak_idx, n):
        if closes[i] < second_bear_trough_val:
            second_bear_trough_val = closes[i]
            second_bear_trough_idx = i
    second_dd = (
        (second_bear_trough_val - second_bear_peak_val) / second_bear_peak_val * 100
        if second_bear_peak_val > 0
        else 0
    )

    # --- 构建 regime 列表 ---
    # 使用 first-cross 找到关键价格水平
    first_400 = n
    for i in range(n):
        if closes[i] >= 400:
            first_400 = i
            break

    regimes = []

    # Regime 1: 底部 → 恢复到 $400 （后 2022 底部复苏）
    r1_end = min(first_400, bear_peak_idx if bear_peak_idx > 0 else first_400)
    if r1_end > 0:
        regimes.append(Regime(
            name="Post-bottom Recovery",
            bar_start=0,
            bar_end=r1_end,
            description=f"${closes[0]:.0f}→${closes[r1_end-1]:.0f}, 后2022底部复苏, ~Oct2022-Jan2024",
        ))

    # Regime 2: $400 → 主要回撤峰值（持续牛市）
    if bear_peak_idx > r1_end:
        regimes.append(Regime(
            name="Bull Run",
            bar_start=r1_end,
            bar_end=bear_peak_idx,
            description=f"${closes[r1_end]:.0f}→${bear_peak_val:.0f}, 持续上涨, ~Jan2024-mid2025",
        ))

    # Regime 3: 主要回撤（-25.6% 熊市修正）
    if bear_peak_idx > 0 and bear_trough_idx > bear_peak_idx:
        regimes.append(Regime(
            name="Bear Correction",
            bar_start=bear_peak_idx,
            bar_end=bear_trough_idx,
            description=f"${bear_peak_val:.0f}→${bear_trough_val:.0f} ({bear_dd:.1f}%), 重大回调",
        ))

    # Regime 4: V 型反弹（从回撤底部到恢复）
    if bear_trough_idx > 0 and recovery_idx > bear_trough_idx:
        regimes.append(Regime(
            name="V-Recovery",
            bar_start=bear_trough_idx,
            bar_end=recovery_idx,
            description=f"${bear_trough_val:.0f}→${closes[recovery_idx-1]:.0f}, V型反弹",
        ))

    # Regime 5: 新高阶段（恢复后到数据结束）
    if recovery_idx < n - 1:
        late_desc = f"${closes[recovery_idx]:.0f}→${closes[-1]:.0f}"
        if second_dd < -8:
            late_desc += f", 含{second_dd:.0f}%回撤"
        regimes.append(Regime(
            name="Late Bull + Volatility",
            bar_start=recovery_idx,
            bar_end=n,
            description=late_desc,
        ))

    # 全量 regime
    regimes.append(Regime(
        name="Full Dataset",
        bar_start=0,
        bar_end=n,
        description=f"${closes[0]:.0f}→${closes[-1]:.0f}, 全量 728K bars",
    ))

    return regimes


@dataclass
class RegimeResult:
    regime: Regime
    n_bars: int
    bh_pct: float
    metrics: dict
    trades: list[CompletedTrade]
    ph_counts: dict[str, int]
    elapsed: float


def run_regime_backtest(
    regime: Regime,
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> RegimeResult:
    s, e = regime.bar_start, regime.bar_end
    seg_opens = opens[s:e]
    seg_highs = highs[s:e]
    seg_lows = lows[s:e]
    seg_closes = closes[s:e]
    n = len(seg_closes)

    t0 = time.time()
    trades, _, state_counts, ph_counts = run_backtest(
        seg_opens, seg_highs, seg_lows, seg_closes,
    )
    elapsed = time.time() - t0

    m = compute_metrics(trades)
    bh = (seg_closes[-1] - seg_closes[0]) / seg_closes[0] * 100

    return RegimeResult(
        regime=regime,
        n_bars=n,
        bh_pct=bh,
        metrics=m,
        trades=trades,
        ph_counts=ph_counts,
        elapsed=elapsed,
    )


def cross_analyze_full_trades(
    full_result: RegimeResult,
    regimes: list[Regime],
    closes: list[float],
) -> list[dict]:
    """用全量回测的交易按 regime 分组统计。

    交叉分析：PH 树保持完整历史上下文，
    仅按入场 bar_idx 将交易归入对应 regime。
    """
    non_full = [r for r in regimes if "Full" not in r.name]
    grouped: dict[str, list[CompletedTrade]] = {r.name: [] for r in non_full}

    for t in full_result.trades:
        for r in non_full:
            if r.bar_start <= t.entry_bar < r.bar_end:
                grouped[r.name].append(t)
                break

    results = []
    for r in non_full:
        trades = grouped[r.name]
        m = compute_metrics(trades)
        bh = (closes[min(r.bar_end, len(closes)) - 1] - closes[r.bar_start]) / closes[r.bar_start] * 100
        results.append({
            "regime": r,
            "trades": trades,
            "metrics": m,
            "bh_pct": bh,
        })
    return results


def write_report(results: list[RegimeResult], closes: list[float]) -> None:
    L: list[str] = []
    L.append("# QQQ 分段 Regime 回测 — 多级别 PH FSM (1min)\n")

    L.append("## 数据说明\n")
    L.append(f"- 总数据：**{len(closes):,}** bars (1min, extended hours)")
    L.append(f"- 价格范围：${min(closes):.2f} — ${max(closes):.2f}")
    L.append("- 时间范围估算：~Oct 2022 至 ~Jun 2026（基于 volume 自相关 ~836 bars/day）")
    L.append("- 数据不包含 2020 COVID 暴跌（QQQ 低至 $163）和 2021 牛市")
    L.append("- 每个 regime 独立初始化（fresh PH trees + fresh FSM state）\n")

    # 汇总表
    L.append("## 汇总对比\n")
    L.append("| Regime | Bars | 价格范围 | BH% | 策略% | 超额% | 胜率 | 交易数 | MaxDD | 耗时 |")
    L.append("|--------|------|---------|-----|-------|-------|------|--------|-------|------|")
    for r in results:
        m = r.metrics
        excess = m["total_compound"] - r.bh_pct
        price_range = f"${r.regime.bar_start}" if r.n_bars == len(closes) else (
            f"${closes[r.regime.bar_start]:.0f}→${closes[min(r.regime.bar_end, len(closes))-1]:.0f}"
        )
        L.append(
            f"| {r.regime.name} | {r.n_bars:,} | {price_range} | "
            f"{r.bh_pct:+.1f} | {m['total_compound']:+.1f} | {excess:+.1f} | "
            f"{m['win_rate']:.0f}% | {m['n']} | {m['max_dd']:.1f}% | {r.elapsed:.0f}s |"
        )
    L.append("")

    # 交叉分析：全量交易按 regime 分组
    full_result = next((r for r in results if "Full" in r.regime.name), None)
    regimes_for_cross = [r.regime for r in results]
    cross_results: list[dict] = []
    if full_result:
        cross_results = cross_analyze_full_trades(full_result, regimes_for_cross, closes)

    if cross_results:
        L.append("## 交叉分析（全量 PH 上下文）\n")
        L.append("独立初始化的 Bear/V-Recovery regime 产生 0 笔交易（L2 PH 冷启动问题）。")
        L.append("以下用全量回测的 55 笔交易按入场时间归入各 regime，")
        L.append("反映**有完整 PH 历史**时各环境的真实表现。\n")
        L.append("| Regime | BH% | 策略% | 超额% | 交易数 | 胜率 | 多/空 | MaxDD |")
        L.append("|--------|-----|-------|-------|--------|------|-------|-------|")
        for cr in cross_results:
            m = cr["metrics"]
            excess = m["total_compound"] - cr["bh_pct"]
            L.append(
                f"| {cr['regime'].name} | {cr['bh_pct']:+.1f} | "
                f"{m['total_compound']:+.1f} | {excess:+.1f} | "
                f"{m['n']} | {m['win_rate']:.0f}% | "
                f"{m['long_n']}/{m['short_n']} | {m['max_dd']:.1f}% |"
            )
        L.append("")

        # 各 regime 交易明细
        for cr in cross_results:
            if cr["trades"]:
                L.append(f"### {cr['regime'].name} — 交叉分析交易明细\n")
                L.append("| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 退出原因 |")
                L.append("|---|------|------|------|---------|------|------|------|---------|")
                for idx, t in enumerate(cr["trades"]):
                    d_str = "多" if t.direction == 1 else "空"
                    hold = t.exit_bar - t.entry_bar
                    L.append(
                        f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                        f"{t.exit_price:.2f}@{t.exit_bar} | {hold:,} | {t.pnl_pct:+.2f} | "
                        f"{t.n_add_positions} | {t.n_short_diffs} | {t.exit_reason} |"
                    )
                L.append("")

    # 关键假说验证
    L.append("## 关键假说验证\n")
    bear_cross = next((cr for cr in cross_results if "Bear" in cr["regime"].name), None)
    bear_result = next((r for r in results if "Bear" in r.regime.name), None)
    full_r = next((r for r in results if "Full" in r.regime.name), None)

    if bear_cross and bear_cross["metrics"]["n"] > 0:
        m = bear_cross["metrics"]
        L.append(f"### 熊市表现（交叉分析: {bear_cross['regime'].name}）\n")
        L.append(f"- Buy-and-hold: **{bear_cross['bh_pct']:+.1f}%**")
        L.append(f"- 策略收益: **{m['total_compound']:+.1f}%**")
        L.append(f"- 超额收益: **{m['total_compound'] - bear_cross['bh_pct']:+.1f}%**")
        L.append(f"- 交易数: {m['n']} (多{m['long_n']}/空{m['short_n']})")
        L.append(f"- 胜率: {m['win_rate']:.1f}%\n")
        if m["total_compound"] > bear_cross["bh_pct"]:
            L.append("**结论**：策略在熊市中跑赢 BH → 支持\"全天候风险控制\"假说。\n")
        elif m["total_compound"] > 0:
            L.append("**结论**：策略在熊市中取得正收益 → 强支持\"全天候风险控制\"假说。\n")
        else:
            L.append("**结论**：策略在熊市中亏损 → 需分析是空头不够还是止损太慢。\n")
    elif bear_result:
        m = bear_result.metrics
        L.append(f"### 熊市表现（独立初始化: {bear_result.regime.name}）\n")
        L.append(f"- Buy-and-hold: **{bear_result.bh_pct:+.1f}%**")
        if m["n"] == 0:
            L.append("- 策略交易数: **0** — L2 PH 冷启动问题，regime 太短无法建立方向")
            L.append("- 需要更长的数据（含 COVID 暴跌）或将 Bear+V-Recovery 合并测试\n")
        else:
            L.append(f"- 策略收益: **{m['total_compound']:+.1f}%**\n")

    # 各 regime 详情
    for r in results:
        m = r.metrics
        ph = r.ph_counts
        L.append(f"## {r.regime.name}\n")
        L.append(f"- 描述：{r.regime.description}")
        L.append(f"- Bars: {r.n_bars:,} (bar {r.regime.bar_start:,} → {r.regime.bar_end:,})")
        L.append(f"- 回测耗时：{r.elapsed:.1f}s\n")

        L.append("### PH 分层统计\n")
        L.append("| 级别 | 更新次数 | settle总数 | rank-1 settle |")
        L.append("|------|---------|-----------|---------------|")
        L.append(f"| L0 | {r.n_bars:,} (每bar) | {ph['l0_settles']:,} | {ph['l0_r1_settles']} |")
        L.append(f"| L1 | {ph['l1_updates']:,} | {ph['l1_settles']:,} | {ph['l1_r1_settles']} |")
        L.append(f"| L2 | {ph['l2_updates']:,} | {ph['l2_settles']:,} | {ph['l2_r1_settles']} |")
        L.append(f"| **L2 方向翻转** | — | — | **{ph['l2_direction_flips']}** |")
        L.append("")

        L.append("### 指标\n")
        L.append("| 指标 | 值 |")
        L.append("|------|-----|")
        L.append(f"| 交易数 | {m['n']} (多{m['long_n']}/空{m['short_n']}) |")
        L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
        L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
        L.append(f"| 复利累计 | **{m['total_compound']:+.2f}%** |")
        L.append(f"| Buy-and-hold | **{r.bh_pct:+.2f}%** |")
        L.append(f"| 超额收益 | **{m['total_compound'] - r.bh_pct:+.2f}%** |")
        L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
        L.append(f"| 平均持仓 | {m['avg_hold_bars']:,} bars |")
        L.append(f"| 有加仓的交易 | {m['n_with_add']}/{m['n']} |")
        L.append(f"| 有降成本的交易 | {m['n_with_cr']}/{m['n']} |")
        L.append(f"| 达到本金回收 | {m['n_pw']}/{m['n']} |")
        L.append("")

        if r.trades:
            L.append("### 交易明细\n")
            L.append("| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 退出原因 |")
            L.append("|---|------|------|------|---------|------|------|------|---------|")
            for idx, t in enumerate(r.trades):
                d_str = "多" if t.direction == 1 else "空"
                hold = t.exit_bar - t.entry_bar
                L.append(
                    f"| {idx+1} | {d_str} | {t.entry_price:.2f}@{t.entry_bar} | "
                    f"{t.exit_price:.2f}@{t.exit_bar} | {hold:,} | {t.pnl_pct:+.2f} | "
                    f"{t.n_add_positions} | {t.n_short_diffs} | {t.exit_reason} |"
                )
            L.append("")

    # 结果包
    L.append("## 结果包六要素\n")
    L.append("**结论**：多级别 PH FSM 在 QQQ 1min 按 regime 分段回测。"
             "验证策略在不同市场环境下的独立表现。\n")
    L.append("**定义依据**：")
    L.append("- Regime 边界基于价格极值和重大回撤（≥10%）自动检测")
    L.append("- 回测引擎与全量回测完全相同（三级别 PH + 5 状态 FSM）")
    L.append("- 每个 regime 独立初始化 → 无前序数据优势/劣势\n")
    L.append("**边界条件**：")
    L.append("- 短 regime（<30K bars）的 L2 PH 可能来不及建立方向 → 交易数少 → 统计不显著")
    L.append("- 独立初始化意味着 regime 边界处的 PH 树为空 → 前几万 bar 可能无交易")
    L.append("- 数据不包含 2020 COVID 暴跌，无法验证极端熊市表现")
    L.append("- 无滑点/手续费建模\n")
    L.append("**下游推论**：")
    L.append("- 若熊市 regime 跑赢 BH → 策略价值在于风控，不在于牛市跑赢")
    L.append("- 若所有 regime 都正收益 → 策略具备全天候适应性")
    L.append("- 若某个 regime 严重亏损 → 需要分析该 regime 的信号特征\n")
    L.append("**谱系引用**：")
    L.append("- 267号：满仓满融降成本体系")
    L.append("- §7.5：在线因果 merge tree")
    L.append("- 526号：递归存在论区分\n")
    L.append("**影响声明**：新建分段回测脚本 + 报告，不修改引擎代码。\n")
    L.append("**认识论等级**：L2（真实数据单标的分段验证；可产生否定性结果）。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main():
    print("加载 QQQ 1min 数据...")
    raw = json.loads(DATA_PATH.read_text())
    opens = [float(x) for x in raw["opens"]]
    highs = [float(x) for x in raw["highs"]]
    lows = [float(x) for x in raw["lows"]]
    closes = [float(x) for x in raw["closes"]]
    n = len(closes)
    print(f"  {n:,} bars, ${closes[0]:.2f} → ${closes[-1]:.2f}")

    regimes = detect_regimes(closes)
    print(f"\n检测到 {len(regimes)} 个 regime:")
    for r in regimes:
        print(f"  [{r.name}] bars {r.bar_start:,}-{r.bar_end:,}: {r.description}")

    results: list[RegimeResult] = []
    for regime in regimes:
        print(f"\n{'='*60}")
        print(f"  {regime.name}")
        print(f"  {regime.description}")
        print(f"  bars {regime.bar_start:,} → {regime.bar_end:,} ({regime.bar_end - regime.bar_start:,} bars)")
        print(f"{'='*60}")

        result = run_regime_backtest(regime, opens, highs, lows, closes)
        m = result.metrics
        print(f"  完成：{result.elapsed:.1f}s")
        print(f"  BH: {result.bh_pct:+.2f}%")
        print(f"  策略: {m['total_compound']:+.2f}% ({m['n']}笔, 胜率{m['win_rate']:.0f}%)")
        print(f"  超额: {m['total_compound'] - result.bh_pct:+.2f}%")
        print(f"  MaxDD: {m['max_dd']:.2f}%")
        results.append(result)

    # 交叉分析打印
    full_r = next((r for r in results if "Full" in r.regime.name), None)
    if full_r:
        regimes_for_cross = [r.regime for r in results]
        cross = cross_analyze_full_trades(full_r, regimes_for_cross, closes)
        print(f"\n{'='*60}")
        print("  交叉分析（全量 PH 上下文, 55笔交易按 regime 分组）")
        print(f"{'='*60}")
        for cr in cross:
            m = cr["metrics"]
            excess = m["total_compound"] - cr["bh_pct"]
            print(f"  {cr['regime'].name}: "
                  f"BH={cr['bh_pct']:+.1f}% 策略={m['total_compound']:+.1f}% "
                  f"超额={excess:+.1f}% 交易={m['n']} 胜率={m['win_rate']:.0f}%")

    write_report(results, closes)


if __name__ == "__main__":
    main()
