#!/usr/bin/env python3
"""K4/ω L2 验证 — 金油比 regime 与 SPY/QQQ 实际涨跌对比。

认识论等级：L2（真实数据；可产生否定性结果）。

数据源优先级：
1. Databento（1min ETF 数据，存入 data_cache）
2. yfinance（daily 数据，用于 ω 分析；1min fallback）

分析流程：
1. 拉取 GLD, USO, UUP, SPY 历史数据
2. 保存 1min 数据到 data_cache（columnar JSON）
3. 计算 ω = GLD/USO 金油比序列（daily）
4. 用 omega_regime.detect_regime 检测 regime 滚动状态
5. 标记 regime 切换点
6. 对比 regime 切换与 SPY 实际涨跌
7. 输出 L2 验证报告
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from dotenv import load_dotenv
load_dotenv(ROOT / ".env")

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "m2_omega_l2_validation.md"

SYMBOLS_DAILY = ["GLD", "USO", "UUP", "SPY"]
SYMBOLS_1MIN = ["GLD", "USO", "UUP", "SPY"]


# ════════════════════════════════════════════════════════════
# 1. 数据拉取
# ════════════════════════════════════════════════════════════

def fetch_daily_yfinance(
    symbols: list[str],
    period: str = "10y",
) -> dict[str, pd.DataFrame]:
    """用 yfinance 拉取日线数据（最长可用历史）。"""
    import yfinance as yf

    results: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  [yfinance daily] {sym} ...")
        try:
            ticker = yf.Ticker(sym)
            df = ticker.history(period=period, interval="1d")
            if df.empty:
                print(f"    ✗ {sym}: 无数据")
                continue
            df = df.rename(columns={
                "Open": "open", "High": "high", "Low": "low",
                "Close": "close", "Volume": "volume",
            })
            df = df[["open", "high", "low", "close", "volume"]]
            if hasattr(df.index, "tz") and df.index.tz is not None:
                df.index = df.index.tz_localize(None)
            results[sym] = df
            print(f"    ✓ {sym}: {len(df)} days, "
                  f"{df.index[0].date()} → {df.index[-1].date()}")
        except Exception as e:
            print(f"    ✗ {sym} failed: {e}")
    return results


def fetch_1min_databento(
    symbols: list[str],
    start: str = "2020-01-01",
) -> dict[str, pd.DataFrame]:
    """尝试用 Databento 拉取 1min 数据。失败则返回空。"""
    try:
        from newchan.data_databento import fetch_ohlcv
    except ImportError:
        print("  [databento] 无法导入 data_databento，跳过")
        return {}

    results: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  [databento 1min] {sym} ...")
        try:
            df = fetch_ohlcv(sym, interval="1min", start=start)
            if df.empty:
                print(f"    ✗ {sym}: 空数据（可能不支持该品种）")
                continue
            results[sym] = df
            print(f"    ✓ {sym}: {len(df):,} bars")
        except Exception as e:
            err_str = str(e)
            if "not found" in err_str.lower() or "symbol" in err_str.lower():
                print(f"    ✗ {sym}: Databento 不支持 ({err_str[:80]})")
            else:
                print(f"    ✗ {sym}: {err_str[:120]}")
    return results


def fetch_1min_yfinance(
    symbols: list[str],
) -> dict[str, pd.DataFrame]:
    """用 yfinance 拉取 1min 数据（最多 ~30天）。"""
    import yfinance as yf

    results: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  [yfinance 1min] {sym} ...")
        try:
            ticker = yf.Ticker(sym)
            df = ticker.history(period="1mo", interval="1m")
            if df.empty:
                print(f"    ✗ {sym}: 无数据")
                continue
            df = df.rename(columns={
                "Open": "open", "High": "high", "Low": "low",
                "Close": "close", "Volume": "volume",
            })
            df = df[["open", "high", "low", "close", "volume"]]
            if hasattr(df.index, "tz") and df.index.tz is not None:
                df.index = df.index.tz_localize(None)
            results[sym] = df
            print(f"    ✓ {sym}: {len(df):,} bars (1min, ~30d)")
        except Exception as e:
            print(f"    ✗ {sym}: {e}")
    return results


def save_columnar_json(
    df: pd.DataFrame, symbol: str, interval: str,
) -> Path:
    """保存为 columnar JSON 格式（与 QQQ 一致）。"""
    data = {
        "opens": df["open"].tolist(),
        "highs": df["high"].tolist(),
        "lows": df["low"].tolist(),
        "closes": df["close"].tolist(),
        "volumes": df["volume"].tolist(),
        "symbol": symbol,
        "dates": [str(t) for t in df.index],
    }
    path = DATA_DIR / f"{symbol.lower()}_{interval}_full.json"
    path.write_text(json.dumps(data))
    size_mb = path.stat().st_size / 1024 / 1024
    print(f"  → {path.name} ({len(df):,} bars, {size_mb:.1f} MB)")
    return path


# ════════════════════════════════════════════════════════════
# 2. ω 计算 + regime 检测
# ════════════════════════════════════════════════════════════

def compute_omega_series(
    df_gold: pd.DataFrame,
    df_oil: pd.DataFrame,
) -> pd.DataFrame:
    """计算 ω = gold/oil 日线序列（对齐日期）。"""
    gold = df_gold[["close"]].rename(columns={"close": "gold"})
    oil = df_oil[["close"]].rename(columns={"close": "oil"})
    merged = gold.join(oil, how="inner")
    merged = merged.dropna()
    merged = merged[merged["oil"] > 0]
    merged["omega"] = merged["gold"] / merged["oil"]
    return merged


def detect_regime_rolling(
    omega_values: np.ndarray,
    short_window: int = 20,
    long_window: int = 60,
    threshold: float = 0.02,
) -> list[dict]:
    """滚动 detect_regime，返回每日的 regime 判断。"""
    from newchan.strategy.omega_regime import detect_regime

    results = []
    for i in range(long_window, len(omega_values)):
        window = omega_values[:i + 1]
        state = detect_regime(
            window, short_window=short_window,
            long_window=long_window, threshold=threshold,
        )
        results.append({
            "idx": i,
            "omega": state.omega,
            "regime": state.regime.value,
            "strength": state.strength,
            "ma_short": state.omega_ma_short,
            "ma_long": state.omega_ma_long,
        })
    return results


def find_regime_transitions(
    regime_history: list[dict],
) -> list[dict]:
    """找出 regime 切换点。"""
    transitions = []
    prev_regime = regime_history[0]["regime"] if regime_history else None
    for entry in regime_history[1:]:
        if entry["regime"] != prev_regime:
            transitions.append({
                "idx": entry["idx"],
                "from_regime": prev_regime,
                "to_regime": entry["regime"],
                "omega": entry["omega"],
                "strength": entry["strength"],
            })
        prev_regime = entry["regime"]
    return transitions


# ════════════════════════════════════════════════════════════
# 3. SPY/QQQ 性能对比
# ════════════════════════════════════════════════════════════

def analyze_transitions_vs_spy(
    transitions: list[dict],
    df_omega: pd.DataFrame,
    df_spy: pd.DataFrame,
    forward_windows: list[int] = [5, 10, 20, 60],
) -> list[dict]:
    """分析每个 regime 切换点前后 SPY 的涨跌。"""
    omega_dates = df_omega.index
    spy_aligned = df_spy.reindex(omega_dates, method="ffill")

    results = []
    for t in transitions:
        idx = t["idx"]
        if idx >= len(omega_dates):
            continue
        date = omega_dates[idx]
        entry = {"date": str(date.date()), **t}

        # SPY 在切换后的表现
        for w in forward_windows:
            end_idx = idx + w
            if end_idx < len(spy_aligned):
                spy_start = spy_aligned.iloc[idx]["close"]
                spy_end = spy_aligned.iloc[end_idx]["close"]
                if spy_start > 0:
                    ret = (spy_end - spy_start) / spy_start * 100
                    entry[f"spy_{w}d_ret"] = round(ret, 2)
                else:
                    entry[f"spy_{w}d_ret"] = None
            else:
                entry[f"spy_{w}d_ret"] = None

        results.append(entry)
    return results


def compute_regime_statistics(
    transition_analysis: list[dict],
    forward_windows: list[int] = [5, 10, 20, 60],
) -> dict:
    """按 regime 切换方向分组统计 SPY 表现。"""
    stats = {}
    for direction in [
        ("neutral", "bull_equity"),
        ("neutral", "bull_commodity"),
        ("bull_equity", "neutral"),
        ("bull_equity", "bull_commodity"),
        ("bull_commodity", "neutral"),
        ("bull_commodity", "bull_equity"),
    ]:
        from_r, to_r = direction
        subset = [
            t for t in transition_analysis
            if t["from_regime"] == from_r and t["to_regime"] == to_r
        ]
        if not subset:
            continue

        key = f"{from_r} → {to_r}"
        stats[key] = {"count": len(subset)}
        for w in forward_windows:
            col = f"spy_{w}d_ret"
            vals = [t[col] for t in subset if t.get(col) is not None]
            if vals:
                stats[key][f"spy_{w}d_avg"] = round(np.mean(vals), 2)
                stats[key][f"spy_{w}d_med"] = round(np.median(vals), 2)
                stats[key][f"spy_{w}d_pos%"] = round(
                    sum(1 for v in vals if v > 0) / len(vals) * 100, 1,
                )
    return stats


# ════════════════════════════════════════════════════════════
# 4. 报告生成
# ════════════════════════════════════════════════════════════

def write_report(
    daily_data: dict[str, pd.DataFrame],
    min1_data: dict[str, pd.DataFrame],
    df_omega: pd.DataFrame,
    regime_history: list[dict],
    transitions: list[dict],
    transition_analysis: list[dict],
    regime_stats: dict,
    elapsed: float,
) -> None:
    L: list[str] = []
    L.append("# K4/ω L2 验证 — 金油比 regime 与 SPY 涨跌对比\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}\n")

    # 数据摘要
    L.append("## 1. 数据摘要\n")
    L.append("### Daily 数据（用于 ω 分析）\n")
    L.append("| 品种 | 天数 | 起始 | 结束 | 首价 | 末价 |")
    L.append("|------|------|------|------|------|------|")
    for sym, df in daily_data.items():
        L.append(
            f"| {sym} | {len(df):,} | {df.index[0].date()}"
            f" | {df.index[-1].date()}"
            f" | {df.iloc[0]['close']:.2f}"
            f" | {df.iloc[-1]['close']:.2f} |"
        )
    L.append("")

    if min1_data:
        L.append("### 1min 数据（缓存供后续使用）\n")
        L.append("| 品种 | Bars | 来源 |")
        L.append("|------|------|------|")
        for sym, df in min1_data.items():
            L.append(f"| {sym} | {len(df):,} | databento/yfinance |")
        L.append("")

    # ω 序列
    L.append("## 2. ω 金油比序列\n")
    L.append(f"- 有效天数：**{len(df_omega):,}**")
    L.append(f"- 日期范围：{df_omega.index[0].date()} → {df_omega.index[-1].date()}")
    L.append(f"- ω 范围：{df_omega['omega'].min():.2f} ~ {df_omega['omega'].max():.2f}")
    L.append(f"- ω 均值：{df_omega['omega'].mean():.2f}")
    L.append(f"- ω 当前：{df_omega['omega'].iloc[-1]:.2f}\n")

    # ω 分位数
    omega = df_omega["omega"]
    L.append("### ω 分位数\n")
    L.append("| 分位 | 值 |")
    L.append("|------|-----|")
    for q in [0.1, 0.25, 0.5, 0.75, 0.9]:
        L.append(f"| {q:.0%} | {omega.quantile(q):.2f} |")
    L.append("")

    # Regime 分布
    L.append("## 3. Regime 分布\n")
    regime_counts = {}
    for r in regime_history:
        regime_counts[r["regime"]] = regime_counts.get(r["regime"], 0) + 1
    total_days = sum(regime_counts.values())
    L.append("| Regime | 天数 | 占比 |")
    L.append("|--------|------|------|")
    for regime, count in sorted(regime_counts.items()):
        L.append(f"| {regime} | {count} | {count/total_days*100:.1f}% |")
    L.append("")

    # Regime 切换点
    L.append("## 4. Regime 切换点\n")
    L.append(f"共 **{len(transitions)}** 次切换\n")
    if transitions:
        L.append("| # | 日期 | 方向 | ω | 强度 |")
        L.append("|---|------|------|---|------|")
        for i, t in enumerate(transition_analysis, 1):
            L.append(
                f"| {i} | {t['date']}"
                f" | {t['from_regime']}→{t['to_regime']}"
                f" | {t['omega']:.2f} | {t['strength']:.4f} |"
            )
        L.append("")

    # SPY 表现对比
    L.append("## 5. Regime 切换后 SPY 表现\n")
    if transition_analysis:
        L.append("### 逐次切换明细\n")
        L.append("| 日期 | 切换方向 | ω | SPY 5d% | SPY 10d% | SPY 20d% | SPY 60d% |")
        L.append("|------|---------|---|---------|----------|----------|----------|")
        for t in transition_analysis:
            spy5 = t.get("spy_5d_ret", "—")
            spy10 = t.get("spy_10d_ret", "—")
            spy20 = t.get("spy_20d_ret", "—")
            spy60 = t.get("spy_60d_ret", "—")
            fmt = lambda v: f"{v:+.2f}" if isinstance(v, (int, float)) else "—"
            L.append(
                f"| {t['date']}"
                f" | {t['from_regime']}→{t['to_regime']}"
                f" | {t['omega']:.2f}"
                f" | {fmt(spy5)} | {fmt(spy10)} | {fmt(spy20)} | {fmt(spy60)} |"
            )
        L.append("")

    # 分组统计
    L.append("### 按切换方向分组统计\n")
    if regime_stats:
        L.append("| 切换方向 | 次数 | SPY 5d均值 | SPY 20d均值 | SPY 60d均值 | SPY 20d正比例 |")
        L.append("|---------|------|-----------|------------|------------|--------------|")
        for direction, s in regime_stats.items():
            L.append(
                f"| {direction} | {s['count']}"
                f" | {s.get('spy_5d_avg', '—')}"
                f" | {s.get('spy_20d_avg', '—')}"
                f" | {s.get('spy_60d_avg', '—')}"
                f" | {s.get('spy_20d_pos%', '—')}% |"
            )
        L.append("")
    else:
        L.append("无足够数据进行分组统计。\n")

    # K4 框架验证
    L.append("## 6. K4 框架 L2 验证结论\n")

    # 计算 ω 与 SPY 的相关性
    if "SPY" in daily_data and len(df_omega) > 0:
        spy_df = daily_data["SPY"][["close"]].rename(columns={"close": "spy"})
        merged_spy = df_omega.join(spy_df, how="inner")
        merged_spy = merged_spy.dropna()

        if len(merged_spy) > 60:
            omega_ret = merged_spy["omega"].pct_change().dropna()
            spy_ret = merged_spy["spy"].pct_change().dropna()
            aligned = pd.concat([omega_ret, spy_ret], axis=1).dropna()
            corr = aligned.corr().iloc[0, 1]

            L.append(f"### ω 日收益率与 SPY 日收益率相关系数：**{corr:.4f}**\n")

            # 滚动相关
            if len(aligned) > 120:
                rolling_corr = aligned["omega"].rolling(60).corr(aligned["spy"])
                L.append(f"- 60日滚动相关范围：{rolling_corr.min():.4f} ~ {rolling_corr.max():.4f}")
                L.append(f"- 60日滚动相关均值：{rolling_corr.mean():.4f}\n")

    # UUP（美元）分析
    if "UUP" in daily_data:
        uup_df = daily_data["UUP"][["close"]].rename(columns={"close": "uup"})
        merged_uup = df_omega.join(uup_df, how="inner").dropna()
        if len(merged_uup) > 60:
            omega_ret = merged_uup["omega"].pct_change().dropna()
            uup_ret = merged_uup["uup"].pct_change().dropna()
            aligned_uup = pd.concat([omega_ret, uup_ret], axis=1).dropna()
            corr_uup = aligned_uup.corr().iloc[0, 1]
            L.append(f"### ω 与 UUP（美元）日收益率相关系数：**{corr_uup:.4f}**\n")

    L.append(f"### 分析耗时：{elapsed:.1f}s\n")

    # 结果包六要素
    L.append("## 结果包六要素\n")
    L.append("**结论**：ω 金油比 regime 切换与 SPY 涨跌的 L2 验证结果"
             "（见上述统计表格）。\n")
    L.append("**定义依据**：")
    L.append("- ω = gold/oil ratio（卢麒元框架：L = M × ω）")
    L.append("- Regime 定义：omega_regime.py 双均线偏离法（MA20/MA60, threshold=2%）")
    L.append("- BULL_COMMODITY：ω 上升（信用收缩）")
    L.append("- BULL_EQUITY：ω 下降（信用扩张）\n")
    L.append("**边界条件**：")
    L.append("- MA 窗口 = 20/60，threshold = 0.02（不同参数可能改变结果）")
    L.append("- ω 用 GLD/USO 代理，非现货 gold/oil")
    L.append("- SPY 作为美股代理，不代表全部板块\n")
    L.append("**下游推论**：")
    L.append("- 若 regime 切换后 SPY 收益率方向与理论一致 → ω 有预测价值")
    L.append("- 若不一致 → ω regime 检测方法需要改进，或理论在当前数据窗口不成立")
    L.append("- 否定性结果同样有价值（缩小有效域边界）\n")
    L.append("**谱系引用**：")
    L.append("- 用户交易方向记忆：押注金油比下降（油涨）")
    L.append("- 卢麒元框架记忆：资本三流、金油两锚\n")
    L.append("**影响声明**：新建验证脚本和报告，不修改引擎代码。\n")
    L.append("**认识论等级**：L2（真实数据，多品种日线；可产生否定性结果）。")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    t0 = time.time()

    print("=" * 60)
    print("  K4/ω L2 验证 — 金油比 regime 与 SPY 涨跌对比")
    print("=" * 60)

    # ── Step 1: 拉取 daily 数据（yfinance） ──
    print("\n[1/5] 拉取 daily 数据 (yfinance) ...")
    daily_data = fetch_daily_yfinance(SYMBOLS_DAILY, period="10y")

    if "GLD" not in daily_data or "USO" not in daily_data:
        print("FATAL: GLD 或 USO daily 数据拉取失败，无法计算 ω")
        sys.exit(1)

    # ── Step 2: 尝试拉取 1min 数据 ──
    print("\n[2/5] 尝试拉取 1min 数据 ...")
    min1_data: dict[str, pd.DataFrame] = {}

    # 先尝试 Databento
    print("  尝试 Databento ...")
    db_data = fetch_1min_databento(SYMBOLS_1MIN, start="2020-01-01")
    min1_data.update(db_data)

    # Databento 未覆盖的品种，用 yfinance fallback（只有 ~30 天）
    missing = [s for s in SYMBOLS_1MIN if s not in min1_data]
    if missing:
        print(f"  Databento 未覆盖 {missing}，yfinance fallback ...")
        yf_1min = fetch_1min_yfinance(missing)
        min1_data.update(yf_1min)

    # 保存 1min 数据到 data_cache
    for sym, df in min1_data.items():
        if not df.empty:
            save_columnar_json(df, sym, "1m")

    # 也保存 daily 数据
    for sym, df in daily_data.items():
        save_columnar_json(df, sym, "daily")

    # ── Step 3: 计算 ω 序列 ──
    print("\n[3/5] 计算 ω = GLD/USO ...")
    df_omega = compute_omega_series(daily_data["GLD"], daily_data["USO"])
    print(f"  ω 序列：{len(df_omega)} 天，"
          f"{df_omega.index[0].date()} → {df_omega.index[-1].date()}")
    print(f"  ω 当前 = {df_omega['omega'].iloc[-1]:.2f}")

    # 保存 ω 序列
    omega_path = DATA_DIR / "omega_gld_uso_daily.json"
    omega_out = {
        "dates": [str(d.date()) for d in df_omega.index],
        "gold": df_omega["gold"].tolist(),
        "oil": df_omega["oil"].tolist(),
        "omega": df_omega["omega"].tolist(),
    }
    omega_path.write_text(json.dumps(omega_out))
    print(f"  → {omega_path.name}")

    # ── Step 4: Regime 检测 ──
    print("\n[4/5] 滚动 regime 检测 ...")
    omega_arr = df_omega["omega"].values
    regime_history = detect_regime_rolling(omega_arr)
    print(f"  regime 判断：{len(regime_history)} 个交易日")

    transitions = find_regime_transitions(regime_history)
    print(f"  regime 切换：{len(transitions)} 次")

    # ── Step 5: 与 SPY 对比 ──
    print("\n[5/5] 分析 regime 切换与 SPY 表现 ...")
    if "SPY" not in daily_data:
        print("  SPY 数据不可用，跳过对比")
        transition_analysis = []
        regime_stats = {}
    else:
        transition_analysis = analyze_transitions_vs_spy(
            transitions, df_omega, daily_data["SPY"],
        )
        regime_stats = compute_regime_statistics(transition_analysis)

        # 简要打印
        for direction, s in regime_stats.items():
            avg20 = s.get("spy_20d_avg", "—")
            pos20 = s.get("spy_20d_pos%", "—")
            print(f"  {direction}: n={s['count']}, "
                  f"SPY 20d avg={avg20}%, pos={pos20}%")

    elapsed = time.time() - t0
    print(f"\n总耗时：{elapsed:.1f}s")

    # ── 写报告 ──
    write_report(
        daily_data=daily_data,
        min1_data=min1_data,
        df_omega=df_omega,
        regime_history=regime_history,
        transitions=transitions,
        transition_analysis=transition_analysis,
        regime_stats=regime_stats,
        elapsed=elapsed,
    )


if __name__ == "__main__":
    main()
