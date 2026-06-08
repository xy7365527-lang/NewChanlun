"""BTC 3年1min完整版赋格回测（做多+做空）— Binance 1.8M bars。

数据源：Binance BTCUSDT 1m archive（41个月，2023-01 → 2026-05）
回测：复用 fugue_with_short_backtest_1min.py 的完整 38 课 FSM
方向：both（向上段做多 + 向下段做空）

认识论等级：L2（真实数据，单标的 1min 3.5年；可产生否定性结果）。
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ANALYSIS_DIR = ROOT / "analysis"
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ANALYSIS_DIR))

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_btc_3year_backtest_1min.md"
BTC_FILE = DATA_DIR / "btc_1m_3year.json"

from fugue_with_short_backtest_1min import (  # noqa: E402
    CompletedTrade,
    compute_metrics,
    run_backtest,
)


def load_btc() -> tuple[list[float], list[float], list[float], list[float], list[str]]:
    raw = json.loads(BTC_FILE.read_text())
    return (
        [float(x) for x in raw["opens"]],
        [float(x) for x in raw["highs"]],
        [float(x) for x in raw["lows"]],
        [float(x) for x in raw["closes"]],
        raw.get("dates", [f"bar_{i}" for i in range(len(raw["closes"]))]),
    )


def write_report(
    n_bars: int,
    closes: list[float],
    dates: list[str],
    trades: list[CompletedTrade],
    event_records: list,
    state_counts: dict,
    ph_counts: dict,
    elapsed: float,
    interval_improvements: list[float],
) -> None:
    m = compute_metrics(trades)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    longs = [t for t in trades if t.side == "long"]
    shorts = [t for t in trades if t.side == "short"]
    l_eq = 1.0
    for t in longs:
        l_eq *= 1 + t.pnl_pct / 100
    s_eq = 1.0
    for t in shorts:
        s_eq *= 1 + t.pnl_pct / 100

    eq_curve: list[float] = [1.0]
    for t in trades:
        eq_curve.append(eq_curve[-1] * (1 + t.pnl_pct / 100))

    L: list[str] = []
    L.append("# BTC 3年1min完整版赋格回测（做多+做空）\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}\n")

    L.append("## 数据\n")
    L.append("- 品种：BTCUSDT（Binance）")
    L.append(f"- 数据：**{n_bars:,}** bars (1min)")
    L.append(f"- 范围：{dates[0]} → {dates[-1]}")
    L.append(f"- 价格：{closes[0]:,.2f} → {closes[-1]:,.2f}")
    L.append(f"- Buy-and-hold：**{bh:+.2f}%**")
    L.append(f"- 回测耗时：{elapsed:.1f}s ({n_bars/elapsed:.0f} bars/s)\n")

    L.append("## 架构\n")
    L.append("复用 fugue_with_short_backtest_1min.py 的完整 38 课 FSM：")
    L.append("- 向上段：买入+持有+降成本+段间比较+区间套")
    L.append("- 向下段：做空（L2 PH 翻空确认后）+降成本+段间比较")
    L.append("- L2 PH 方向裁决：翻多→做多，翻空→做空")
    L.append("- D2无加仓版 + persistence过滤\n")

    L.append("## PH 分层统计\n")
    L.append("| 级别 | 更新次数 | settle总数 | rank-1 settle |")
    L.append("|------|---------|-----------|---------------|")
    L.append(
        f"| L0 | {n_bars:,} (每bar)"
        f" | {ph_counts['l0_settles']:,} | {ph_counts['l0_r1_settles']} |"
    )
    L.append(
        f"| L1 | {ph_counts['l1_updates']:,}"
        f" | {ph_counts['l1_settles']:,} | {ph_counts['l1_r1_settles']} |"
    )
    L.append(
        f"| L2 | {ph_counts['l2_updates']:,}"
        f" | {ph_counts['l2_settles']:,} | {ph_counts['l2_r1_settles']} |"
    )
    L.append(
        f"| **L2 方向翻转** | — | —"
        f" | **{ph_counts['l2_direction_flips']}** |"
    )
    et = ph_counts.get("eval_triggered", 0)
    ef = ph_counts.get("eval_filtered", 0)
    if (et + ef) > 0:
        L.append(
            f"| **EVAL** | 触发={et} | 过滤={ef}"
            f" | 过滤率={ef/(et+ef)*100:.0f}% |"
        )
    else:
        L.append("| **EVAL** | 触发=0 | 过滤=0 | — |")
    L.append("")

    L.append("## 总体指标\n")
    L.append("| 指标 | 值 |")
    L.append("|------|-----|")
    L.append(f"| 交易数 | {m['n']} |")
    L.append(f"| 胜率 | {m['win_rate']:.1f}% |")
    L.append(f"| 平均收益 | {m['avg_pnl']:+.3f}% |")
    L.append(f"| 复利累计 | **{m['total_compound']:+.2f}%** |")
    L.append(f"| 最大回撤 | {m['max_dd']:.2f}% |")
    L.append(f"| 平均持仓 | {m['avg_hold_bars']:,} bars |")
    L.append(f"| 有降成本的交易 | {m['n_with_cr']}/{m['n']} |")
    L.append(f"| 达到本金回收 | {m['n_pw']}/{m['n']} |")
    L.append(
        f"| 有段间比较的交易 | {m.get('n_with_seg_compare', 0)}/{m['n']} |"
    )
    L.append("")

    L.append("### 多空拆分\n")
    L.append("| 方向 | 交易数 | 复利 | 胜率 | 平均收益 |")
    L.append("|------|--------|------|------|----------|")
    if longs:
        l_wins = sum(1 for t in longs if t.pnl_pct > 0)
        l_avg = sum(t.pnl_pct for t in longs) / len(longs)
        L.append(
            f"| 做多 | {len(longs)}"
            f" | {(l_eq-1)*100:+.2f}%"
            f" | {l_wins/len(longs)*100:.1f}%"
            f" | {l_avg:+.3f}% |"
        )
    else:
        L.append("| 做多 | 0 | — | — | — |")
    if shorts:
        s_wins = sum(1 for t in shorts if t.pnl_pct > 0)
        s_avg = sum(t.pnl_pct for t in shorts) / len(shorts)
        L.append(
            f"| 做空 | {len(shorts)}"
            f" | {(s_eq-1)*100:+.2f}%"
            f" | {s_wins/len(shorts)*100:.1f}%"
            f" | {s_avg:+.3f}% |"
        )
    else:
        L.append("| 做空 | 0 | — | — | — |")
    L.append("")

    # 年度拆分
    L.append("### 年度拆分\n")
    yearly: dict[str, list[CompletedTrade]] = {}
    for t in trades:
        yr = dates[min(t.entry_bar, n_bars - 1)][:4]
        yearly.setdefault(yr, []).append(t)

    L.append("| 年份 | 交易数 | 胜率 | 复利 | 平均收益 |")
    L.append("|------|--------|------|------|----------|")
    for yr in sorted(yearly.keys()):
        yt = yearly[yr]
        y_eq = 1.0
        for t in yt:
            y_eq *= 1 + t.pnl_pct / 100
        y_wins = sum(1 for t in yt if t.pnl_pct > 0)
        y_avg = sum(t.pnl_pct for t in yt) / len(yt)
        L.append(
            f"| {yr} | {len(yt)}"
            f" | {y_wins/len(yt)*100:.1f}%"
            f" | {(y_eq-1)*100:+.2f}%"
            f" | {y_avg:+.3f}% |"
        )
    L.append("")

    L.append("## 交易明细\n")
    L.append(
        "| # | 方向 | 入场 | 出场 | 持仓bars | PnL%"
        " | 短差 | 段数 | 退出原因 |"
    )
    L.append(
        "|---|------|------|------|---------|------"
        "|------|------|---------|"
    )
    for idx, t in enumerate(trades, 1):
        hold = t.exit_bar - t.entry_bar
        side_cn = "多" if t.side == "long" else "空"
        entry_date = dates[min(t.entry_bar, n_bars - 1)][:10]
        exit_date = dates[min(t.exit_bar, n_bars - 1)][:10]
        L.append(
            f"| {idx} | {side_cn}"
            f" | {t.entry_price:,.2f} ({entry_date})"
            f" | {t.exit_price:,.2f} ({exit_date})"
            f" | {hold:,} | {t.pnl_pct:+.2f}"
            f" | {t.n_short_diffs}"
            f" | {t.n_up_segs}"
            f" | {t.exit_reason} |"
        )
    L.append("")

    L.append("## FSM 状态分布（按 bar 数）\n")
    L.append("| 状态 | Bars | 占比 |")
    L.append("|------|------|------|")
    total_bars = sum(state_counts.values())
    for st_name, cnt in sorted(state_counts.items(), key=lambda x: -x[1]):
        pct = cnt / total_bars * 100 if total_bars else 0
        if cnt > 0:
            L.append(f"| {st_name} | {cnt:,} | {pct:.1f}% |")
    L.append("")

    if interval_improvements:
        pos_imps = [x for x in interval_improvements if x > 0]
        neg_imps = [x for x in interval_improvements if x < 0]
        avg_imp = sum(interval_improvements) / len(interval_improvements)
        L.append("## 区间套改善统计\n")
        L.append(f"- 总次数：{len(interval_improvements)}")
        L.append(f"- 正改善（更好价格）：{len(pos_imps)}")
        L.append(f"- 负改善（更差价格）：{len(neg_imps)}")
        L.append(f"- 平均改善：${avg_imp:+.2f}")
        if pos_imps:
            L.append(f"- 最大正改善：${max(pos_imps):+.2f}")
        L.append("")

    # 权益曲线关键点
    L.append("## 权益曲线关键点\n")
    L.append("| 交易# | 权益倍数 | 变化 |")
    L.append("|-------|---------|------|")
    for idx in range(len(eq_curve)):
        if idx == 0:
            continue
        if idx <= 5 or idx >= len(eq_curve) - 3:
            L.append(
                f"| {idx} | {eq_curve[idx]:.4f}"
                f" | {trades[idx-1].pnl_pct:+.2f}% |"
            )
        elif idx == 6:
            L.append("| ... | ... | ... |")
    L.append("")

    if event_records:
        L.append(
            f"<details><summary>事件流（{len(event_records)}条）"
            f"</summary>\n"
        )
        L.append("| Bar | 价格 | 状态 | 事件 |")
        L.append("|-----|------|------|------|")
        for r in event_records[:300]:
            L.append(
                f"| {r.bar_idx:,} | {r.price:,.2f}"
                f" | {r.state} | {r.event} |"
            )
        if len(event_records) > 300:
            L.append(
                f"| ... | ... | ..."
                f" | （共{len(event_records)}条，显示前300） |"
            )
        L.append("</details>\n")

    L.append("## 结果包六要素\n")
    L.append(
        f"**结论**：BTC 3年1min完整版赋格回测（做多+做空），"
        f"{n_bars/1440:.0f}天真实数据。"
        f" 复利{m['total_compound']:+.2f}% vs BH{bh:+.2f}%。\n"
    )
    L.append("**定义依据**：")
    L.append("- 38课完整操作程式（FSM）")
    L.append("- L2 PH 方向裁决决定做多/做空")
    L.append("- D2无加仓版 + persistence过滤 + 区间套 + 降成本")
    L.append("- 做空为38课程式镜像\n")
    L.append("**边界条件**：")
    L.append("- BTC 24/7 市场，无开盘/收盘缺口")
    L.append("- 不含手续费/滑点/资金费率")
    L.append("- 数据源 Binance BTCUSDT（流动性最好的交易对）")
    L.append(f"- 测试期 {dates[0][:10]} → {dates[-1][:10]}")
    L.append(f"  （{n_bars/1440:.0f}天 = {n_bars/1440/365:.1f}年）\n")
    L.append("**下游推论**：")
    L.append("- 3.5年跨越完整牛熊周期（2023低位→2024牛市→调整），"
             "结论比17个月更稳健")
    L.append("- 做空在BTC下跌段的贡献可独立评估")
    L.append("- 与QQQ/OKLO/HK700横向对比，BTC波动率显著更高\n")
    L.append("**谱系引用**：无直接相关谱系（BTC首次3年长历史回测）。\n")
    L.append(
        "**影响声明**：新建回测脚本，不修改引擎代码或旧版回测。\n"
    )
    L.append(
        "**认识论等级**：L2（真实数据，单标的 1min 3.5年；"
        "可产生否定性结果）。"
    )

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


def main() -> None:
    print("=" * 60)
    print("  BTCUSDT — 3年完整版赋格回测（做多+做空）")
    print("=" * 60)

    opens, highs, lows, closes, dates = load_btc()
    n = len(closes)
    print(f"  数据：{n:,} bars ({dates[0]} → {dates[-1]})")
    print(f"  价格：{closes[0]:,.2f} → {closes[-1]:,.2f}")

    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  BH: {bh:+.2f}%")

    t0 = time.time()
    trades, event_records, state_counts, ph_counts, iv_imps = run_backtest(
        opens, highs, lows, closes,
    )
    elapsed = time.time() - t0

    m = compute_metrics(trades)

    print(f"\n  完成：{elapsed:.1f}s ({n/elapsed:.0f} bars/s)")
    print(
        f"  PH：L0 r1={ph_counts['l0_r1_settles']}, "
        f"L1={ph_counts['l1_updates']} upd/{ph_counts['l1_r1_settles']} r1, "
        f"L2={ph_counts['l2_updates']} upd/{ph_counts['l2_r1_settles']} r1, "
        f"翻转={ph_counts['l2_direction_flips']}"
    )
    et = ph_counts.get("eval_triggered", 0)
    ef = ph_counts.get("eval_filtered", 0)
    if (et + ef) > 0:
        print(f"  EVAL：触发={et}, 过滤={ef}, 过滤率={ef/(et+ef)*100:.0f}%")
    print(f"  交易：{m['n']}笔, 胜率：{m['win_rate']:.1f}%")
    print(f"  复利：{m['total_compound']:+.2f}%, BH：{bh:+.2f}%")

    longs = [t for t in trades if t.side == "long"]
    shorts = [t for t in trades if t.side == "short"]
    l_eq = 1.0
    for t in longs:
        l_eq *= 1 + t.pnl_pct / 100
    s_eq = 1.0
    for t in shorts:
        s_eq *= 1 + t.pnl_pct / 100
    print(f"  多头：{len(longs)}笔 ({(l_eq-1)*100:+.1f}%)")
    print(f"  空头：{len(shorts)}笔 ({(s_eq-1)*100:+.1f}%)")

    write_report(
        n_bars=n, closes=closes, dates=dates,
        trades=trades, event_records=event_records,
        state_counts=state_counts, ph_counts=ph_counts,
        elapsed=elapsed, interval_improvements=iv_imps,
    )


if __name__ == "__main__":
    main()
