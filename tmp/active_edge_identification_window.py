"""
活跃边辨识窗口分析——2020-21 不可折叠相中六条边的波动率分化时间线

核心问题：在不可折叠相中，从体制进入到活跃边可辨识之间有多少天？
即"盲区窗口"有多长？

方法：
1. 读取已有的 daily_vol_2020_21.csv（六条边的 rolling 20日波动率）
2. 计算边间波动率分化指标：
   a. 含Commodity边 vs 不含Commodity边的 vol ratio 均值之差（已知断裂节点=Commodity）
   b. 六条边两两 vol ratio 的极差（max/min）
   c. 单条边 vol ratio 显著偏离其他边的首次日期
3. 确定"活跃边可辨识"的时间点
4. 计算盲区窗口长度

认识论等级：L2（2020-21单体制真实数据）
"""

import json
from pathlib import Path

import numpy as np
import pandas as pd

OUT_DIR = Path("C:/Users/hanju/NewChanlun/tmp")
DATA_PATH = Path(
    "C:/Users/hanju/NewChanlun/tmp/regime-analysis/"
    "edge_freeze_sequence/daily_vol_2020_21.csv"
)

EDGES = [
    "Au/USD", "Au/Equity", "Au/Commodity",
    "USD/Equity", "USD/Commodity", "Equity/Commodity",
]

COMMODITY_EDGES = ["Au/Commodity", "USD/Commodity", "Equity/Commodity"]
NON_COMMODITY_EDGES = ["Au/USD", "Au/Equity", "USD/Equity"]

REGIME_START = "2020-04-03"
REGIME_END = "2021-04-23"


def load_data() -> pd.DataFrame:
    df = pd.read_csv(DATA_PATH)
    df["date"] = pd.to_datetime(df["date"])
    df = df.set_index("date")
    return df


def extract_vol_ratios(df: pd.DataFrame) -> pd.DataFrame:
    """Extract vol ratio columns for all edges."""
    ratio_cols = {}
    for edge in EDGES:
        col = f"{edge}_ratio"
        if col in df.columns:
            ratio_cols[edge] = df[col].astype(float)
    return pd.DataFrame(ratio_cols)


def compute_divergence_metrics(ratios: pd.DataFrame) -> pd.DataFrame:
    """Compute daily divergence metrics between Commodity and non-Commodity edges."""
    metrics = pd.DataFrame(index=ratios.index)

    # Mean vol ratio of Commodity-containing edges
    metrics["commodity_mean"] = ratios[COMMODITY_EDGES].mean(axis=1)
    # Mean vol ratio of non-Commodity edges
    metrics["non_commodity_mean"] = ratios[NON_COMMODITY_EDGES].mean(axis=1)
    # Divergence = commodity_mean - non_commodity_mean
    metrics["divergence"] = metrics["commodity_mean"] - metrics["non_commodity_mean"]
    # Ratio of means
    metrics["divergence_ratio"] = (
        metrics["commodity_mean"] / metrics["non_commodity_mean"]
    )

    # Range across all 6 edges
    metrics["max_ratio"] = ratios.max(axis=1)
    metrics["min_ratio"] = ratios.min(axis=1)
    metrics["spread"] = metrics["max_ratio"] - metrics["min_ratio"]
    metrics["spread_ratio"] = metrics["max_ratio"] / metrics["min_ratio"]

    # Which edge has max vol ratio each day
    metrics["max_edge"] = ratios.idxmax(axis=1)
    metrics["min_edge"] = ratios.idxmin(axis=1)

    return metrics


def find_identification_date(
    metrics: pd.DataFrame,
    regime_start: str,
    threshold_divergence_ratio: float = 1.5,
    confirmation_days: int = 5,
) -> dict:
    """Find when Commodity edges become clearly identifiable as active edges.

    Criteria: commodity_mean / non_commodity_mean > threshold for
    confirmation_days consecutive days.
    """
    regime_mask = metrics.index >= regime_start
    regime_metrics = metrics[regime_mask]

    above_threshold = regime_metrics["divergence_ratio"] > threshold_divergence_ratio

    # Find first run of confirmation_days consecutive True
    run_count = 0
    first_confirmed_date = None
    for date, above in above_threshold.items():
        if above:
            run_count += 1
            if run_count >= confirmation_days:
                # The identification date is when the run started
                first_confirmed_date = regime_metrics.index[
                    regime_metrics.index.get_loc(date) - confirmation_days + 1
                ]
                break
        else:
            run_count = 0

    return {
        "threshold": threshold_divergence_ratio,
        "confirmation_days": confirmation_days,
        "first_confirmed_date": (
            str(first_confirmed_date.date()) if first_confirmed_date else None
        ),
    }


def find_single_edge_breakout(
    ratios: pd.DataFrame,
    regime_start: str,
    z_threshold: float = 2.0,
    confirmation_days: int = 3,
) -> list[dict]:
    """Find when each individual edge's vol ratio breaks away from the pack.

    For each edge, compute its z-score relative to the 6-edge cross-sectional
    distribution on that day. When z-score > threshold for confirmation_days,
    the edge is identified as "active".
    """
    regime_mask = ratios.index >= regime_start
    regime_ratios = ratios[regime_mask]

    # Cross-sectional z-score: (edge_ratio - mean_of_6) / std_of_6
    cross_mean = regime_ratios.mean(axis=1)
    cross_std = regime_ratios.std(axis=1)

    breakouts = []
    for edge in EDGES:
        edge_z = (regime_ratios[edge] - cross_mean) / cross_std

        # Find first run of confirmation_days above z_threshold
        run_count = 0
        first_confirmed = None
        for date, z_val in edge_z.items():
            if z_val > z_threshold:
                run_count += 1
                if run_count >= confirmation_days:
                    idx = regime_ratios.index.get_loc(date) - confirmation_days + 1
                    first_confirmed = regime_ratios.index[idx]
                    break
            else:
                run_count = 0

        breakouts.append({
            "edge": edge,
            "is_commodity_edge": edge in COMMODITY_EDGES,
            "first_breakout_date": (
                str(first_confirmed.date()) if first_confirmed else None
            ),
            "mean_z_first_30d": round(float(edge_z.iloc[:30].mean()), 4),
            "mean_z_full_regime": round(float(edge_z.mean()), 4),
        })

    return breakouts


def compute_rolling_divergence(
    ratios: pd.DataFrame,
    windows: list[int],
) -> dict:
    """Compute rolling divergence between Commodity and non-Commodity edges
    at multiple window sizes to find when divergence first emerges."""
    results = {}
    for w in windows:
        commodity_rolling = ratios[COMMODITY_EDGES].mean(axis=1).rolling(w).mean()
        non_commodity_rolling = ratios[NON_COMMODITY_EDGES].mean(axis=1).rolling(w).mean()
        div_ratio = commodity_rolling / non_commodity_rolling

        # Find first date where rolling divergence ratio > 1.5
        above = div_ratio > 1.5
        first_date = above[above].index[0] if above.any() else None

        results[f"rolling_{w}d"] = {
            "window": w,
            "first_above_1.5": str(first_date.date()) if first_date else None,
        }
    return results


def analyze_pre_regime_signals(
    ratios: pd.DataFrame, metrics: pd.DataFrame, regime_start: str
) -> dict:
    """Analyze whether any edge showed divergence before the regime started.

    The riot sequence from edge_freeze_results.json shows edges started rioting
    from 2020-02-21. Check if there was already identifiable divergence then.
    """
    pre_mask = ratios.index < regime_start
    pre_ratios = ratios[pre_mask]
    pre_metrics = metrics[pre_mask]

    # Focus on 2020-02-21 onward (first riot date)
    riot_start = "2020-02-21"
    riot_mask = pre_ratios.index >= riot_start
    riot_ratios = pre_ratios[riot_mask]
    riot_metrics = pre_metrics[riot_mask]

    if riot_ratios.empty:
        return {"pre_regime_divergence": "no data in riot period before regime"}

    return {
        "riot_period": f"{riot_start} to {regime_start}",
        "trading_days": len(riot_ratios),
        "mean_divergence_ratio": round(
            float(riot_metrics["divergence_ratio"].mean()), 4
        ),
        "mean_commodity_ratio": round(
            float(riot_ratios[COMMODITY_EDGES].mean().mean()), 4
        ),
        "mean_non_commodity_ratio": round(
            float(riot_ratios[NON_COMMODITY_EDGES].mean().mean()), 4
        ),
        "max_divergence_ratio": round(
            float(riot_metrics["divergence_ratio"].max()), 4
        ),
        "max_divergence_date": str(
            riot_metrics["divergence_ratio"].idxmax().date()
        ),
    }


def compute_blind_window(regime_start: str, identification_date: str | None) -> dict:
    """Compute blind window duration."""
    if identification_date is None:
        return {"blind_window_days": None, "note": "no identification date found"}

    rs = pd.Timestamp(regime_start)
    ident = pd.Timestamp(identification_date)
    # Trading days between (use business day count as approximation)
    bdays = pd.bdate_range(rs, ident)
    calendar_days = (ident - rs).days
    return {
        "regime_start": regime_start,
        "identification_date": identification_date,
        "calendar_days": calendar_days,
        "approx_trading_days": len(bdays),
    }


def main():
    print("Loading daily vol data...")
    df = load_data()
    ratios = extract_vol_ratios(df)

    print(f"Data: {len(df)} rows, {df.index[0].date()} to {df.index[-1].date()}")
    print(f"Ratios shape: {ratios.shape}")

    # Compute divergence metrics
    metrics = compute_divergence_metrics(ratios)

    # 1. Group-level identification (Commodity vs non-Commodity)
    print("\n--- Group-level divergence identification ---")
    group_results = {}
    for threshold in [1.3, 1.5, 2.0, 3.0]:
        for conf_days in [3, 5, 10]:
            result = find_identification_date(
                metrics, REGIME_START, threshold, conf_days
            )
            key = f"thresh_{threshold}_conf_{conf_days}"
            group_results[key] = result
            if result["first_confirmed_date"]:
                blind = compute_blind_window(REGIME_START, result["first_confirmed_date"])
                result["blind_window"] = blind
                print(
                    f"  Threshold={threshold}, Confirm={conf_days}d: "
                    f"Identified {result['first_confirmed_date']} "
                    f"(blind window: {blind['calendar_days']}d calendar, "
                    f"~{blind['approx_trading_days']}d trading)"
                )

    # 2. Single-edge breakout
    print("\n--- Single edge breakout analysis ---")
    breakouts = find_single_edge_breakout(ratios, REGIME_START, z_threshold=1.5, confirmation_days=3)
    for b in breakouts:
        print(f"  {b['edge']}: breakout={b['first_breakout_date']}, "
              f"commodity={b['is_commodity_edge']}, "
              f"mean_z_30d={b['mean_z_first_30d']}")

    # 3. Rolling divergence at multiple windows
    print("\n--- Rolling divergence emergence ---")
    rolling_results = compute_rolling_divergence(ratios, [5, 10, 20, 60])
    for k, v in rolling_results.items():
        print(f"  {k}: first above 1.5 = {v['first_above_1.5']}")

    # 4. Pre-regime signals
    print("\n--- Pre-regime riot period signals ---")
    pre_results = analyze_pre_regime_signals(ratios, metrics, REGIME_START)
    for k, v in pre_results.items():
        print(f"  {k}: {v}")

    # 5. Daily divergence time series for the first 60 days of regime
    regime_mask = metrics.index >= REGIME_START
    regime_metrics = metrics[regime_mask].head(60)
    daily_divergence = []
    for date, row in regime_metrics.iterrows():
        daily_divergence.append({
            "date": str(date.date()),
            "commodity_mean_ratio": round(float(row["commodity_mean"]), 4),
            "non_commodity_mean_ratio": round(float(row["non_commodity_mean"]), 4),
            "divergence_ratio": round(float(row["divergence_ratio"]), 4),
            "spread_ratio": round(float(row["spread_ratio"]), 4),
            "max_edge": row["max_edge"],
        })

    # 6. Primary result: use threshold=1.5, confirmation=5 as main metric
    primary = group_results.get("thresh_1.5_conf_5", {})
    primary_date = primary.get("first_confirmed_date")
    primary_blind = compute_blind_window(REGIME_START, primary_date)

    # Also compute from first riot date (2020-02-21) to identification
    riot_blind = None
    if primary_date:
        riot_blind = compute_blind_window("2020-02-21", primary_date)

    # Assemble full result
    result = {
        "analysis": "K4 active edge identification window",
        "regime": {
            "name": "2020-21 COVID/QE sync compression",
            "regime_start": REGIME_START,
            "regime_end": REGIME_END,
            "duration_days": 263,
            "inferred_rupture_node": "Commodity",
        },
        "primary_result": {
            "method": "commodity_vs_non_commodity divergence ratio > 1.5 for 5 consecutive days",
            "identification_date": primary_date,
            "blind_window_from_regime_start": primary_blind,
            "blind_window_from_first_riot": riot_blind,
        },
        "sensitivity_analysis": group_results,
        "single_edge_breakouts": breakouts,
        "rolling_divergence": rolling_results,
        "pre_regime_riot_signals": pre_results,
        "first_60d_daily_divergence": daily_divergence,
        "epistemological_level": "L2",
        "genealogy": ["329号", "330号", "335号"],
    }

    # Write JSON
    json_path = OUT_DIR / "active-edge-identification-window.json"
    json_path.write_text(
        json.dumps(result, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    print(f"\nJSON written to {json_path}")

    # Write report
    write_report(result, OUT_DIR / "active-edge-identification-report.md")
    print(f"Report written to {OUT_DIR / 'active-edge-identification-report.md'}")


def write_report(result: dict, path: Path) -> None:
    primary = result["primary_result"]
    regime = result["regime"]
    blind_regime = primary["blind_window_from_regime_start"]
    blind_riot = primary["blind_window_from_first_riot"]

    lines = []
    lines.append("# K4 活跃边辨识窗口分析报告")
    lines.append("")
    lines.append("**认识论等级**: L2（2020-21单体制真实数据）")
    lines.append("**谱系依据**: 329号、330号、335号")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 一、核心问题")
    lines.append("")
    lines.append("在不可折叠相中切换标的跟随活跃边的策略，盲区窗口有多长？")
    lines.append("即：从体制进入（或 basis 预警/riot 开始）到活跃边可辨识之间有多少天。")
    lines.append("")
    lines.append("## 二、结论")
    lines.append("")

    if blind_regime and blind_regime.get("calendar_days") is not None:
        lines.append(f"### 主结果（阈值=1.5，确认天数=5）")
        lines.append("")
        lines.append(f"- **体制起点**: {regime['regime_start']}")
        lines.append(f"- **活跃边辨识日**: {primary['identification_date']}")
        lines.append(f"- **盲区窗口（从体制起点）**: {blind_regime['calendar_days']} 日历天 / ~{blind_regime['approx_trading_days']} 交易日")
        if blind_riot and blind_riot.get("calendar_days") is not None:
            lines.append(f"- **盲区窗口（从首次 riot 2020-02-21）**: {blind_riot['calendar_days']} 日历天 / ~{blind_riot['approx_trading_days']} 交易日")
    else:
        lines.append("**在给定阈值下未找到辨识日期。**")

    lines.append("")
    lines.append("### 辨识标准")
    lines.append("")
    lines.append("含Commodity边的均值 vol ratio / 不含Commodity边的均值 vol ratio > 1.5，")
    lines.append("连续5个交易日保持。")
    lines.append("")

    # Sensitivity
    lines.append("### 敏感性分析")
    lines.append("")
    lines.append("| 阈值 | 确认天数 | 辨识日期 | 盲区（日历天） | 盲区（交易日） |")
    lines.append("|------|---------|---------|-------------|-------------|")
    for key, val in result["sensitivity_analysis"].items():
        ident = val.get("first_confirmed_date", "-")
        blind = val.get("blind_window", {})
        cal = blind.get("calendar_days", "-")
        trd = blind.get("approx_trading_days", "-")
        lines.append(f"| {val['threshold']} | {val['confirmation_days']} | {ident} | {cal} | {trd} |")
    lines.append("")

    # Single edge breakouts
    lines.append("## 三、单条边突破分析")
    lines.append("")
    lines.append("每条边相对六边截面分布的 z-score > 1.5 且连续3天：")
    lines.append("")
    lines.append("| 边 | 含Commodity | 突破日期 | 前30天均值z | 全体制均值z |")
    lines.append("|----|-----------:|---------|----------:|----------:|")
    for b in result["single_edge_breakouts"]:
        lines.append(
            f"| {b['edge']} | {'是' if b['is_commodity_edge'] else '否'} | "
            f"{b['first_breakout_date'] or '-'} | "
            f"{b['mean_z_first_30d']} | {b['mean_z_full_regime']} |"
        )
    lines.append("")

    # Rolling divergence
    lines.append("## 四、滚动分化首次出现")
    lines.append("")
    lines.append("| 滚动窗口 | 分化比>1.5首次出现日 |")
    lines.append("|---------|-------------------|")
    for k, v in result["rolling_divergence"].items():
        lines.append(f"| {v['window']}天 | {v['first_above_1.5'] or '-'} |")
    lines.append("")

    # Pre-regime
    lines.append("## 五、体制前 Riot 期间信号")
    lines.append("")
    pre = result["pre_regime_riot_signals"]
    for k, v in pre.items():
        lines.append(f"- **{k}**: {v}")
    lines.append("")

    # First 60 days divergence summary
    lines.append("## 六、体制前60天分化比时间线")
    lines.append("")
    lines.append("| 日期 | Commodity边均值 | 非Commodity边均值 | 分化比 | 最大边 |")
    lines.append("|------|---------------|-----------------|-------|-------|")
    for d in result["first_60d_daily_divergence"][:60]:
        lines.append(
            f"| {d['date']} | {d['commodity_mean_ratio']} | "
            f"{d['non_commodity_mean_ratio']} | "
            f"{d['divergence_ratio']} | {d['max_edge']} |"
        )
    lines.append("")

    # Boundary conditions
    lines.append("## 七、边界条件")
    lines.append("")
    lines.append('1. **单一体制**: 仅分析2020-21体制。1987和1990-91是三节点体制（无Au），边数不同不可直接比较')
    lines.append('2. **断裂节点已知**: 分析基于\u201c断裂节点=Commodity\u201d这一先验（来自330号结果），不是盲测')
    lines.append('3. **阈值任意性**: 1.5倍分化比和5天确认窗口是人为设定，不同阈值产生不同盲区长度（见敏感性分析）')
    lines.append('4. **rolling 20日 vol**: 波动率窗口为20天，意味着辨识信号本身有约20天延迟')
    lines.append('5. **Baseline**: daily_vol CSV 中 vol ratio 的 baseline 截止到 2020-02-15（COVID前），不同 baseline 影响 ratio 绝对值')
    lines.append("")

    # Impact
    lines.append("## 八、影响声明")
    lines.append("")
    lines.append("本分析量化了不可折叠相中活跃边辨识的盲区窗口。盲区窗口的存在意味着：")
    lines.append('在体制进入初期，六条边全部极端激活，无法区分哪条边是\u201c活跃\u201d的（全图 riot）。')
    lines.append("需要等待一定天数后，Commodity 相关边与非 Commodity 边的分化才可辨识。")
    lines.append("这对操作方法论的影响：体制初期只能做全图对冲/不操作，活跃边跟随策略")
    lines.append("需要等待分化信号确认后才能启动。")
    lines.append("")

    path.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
