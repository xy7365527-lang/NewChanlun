"""
体制内部 eff.dim 演化动态分析 (v2)
====================================

v2 改进:
1. 使用全可用历史数据计算 p5 阈值 (而非仅扩展窗口)
2. 1987 体制因 WTI 数据从 1986 才开始, rolling 252d 窗口
   导致 eff.dim 从 ~1987-01 才有值 → 承认此限制,
   用实际可用的 eff.dim 点做分析
3. 进入/退出分析基于 eff.dim 时间序列的绝对值和相对变化,
   不仅仅是 p5 阈值跨越
4. 输出完整的扩展窗口 eff.dim 时间序列

认识论等级: L2 (真实数据, 可否证)
"""

from __future__ import annotations

import json
import warnings
from collections import Counter
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd
import yfinance as yf

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent
WTI_CSV_PATH = Path(__file__).parent.parent.parent / "fred-k4-cluster" / "wti_fred.csv"

EFFDIM_WINDOW = 252


# ============================================================
# 数据获取 (与 v1 相同)
# ============================================================

def download_yahoo(ticker: str, start: str, end: str) -> pd.Series:
    df = yf.download(ticker, start=start, end=end, progress=False)
    if df.empty:
        raise RuntimeError(f"Yahoo Finance 返回空: {ticker}")
    if isinstance(df.columns, pd.MultiIndex):
        close = df["Close"]
        if isinstance(close, pd.DataFrame):
            close = close.iloc[:, 0]
    else:
        close = df["Close"]
    s = close.copy()
    s.index = pd.DatetimeIndex(s.index)
    return s


def load_wti_from_csv(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path, parse_dates=["observation_date"],
                     index_col="observation_date")
    s = df["DCOILWTICO"].replace(".", np.nan).astype(float)
    s.index = pd.DatetimeIndex(s.index)
    s.name = "WTI"
    return s.dropna()


# ============================================================
# eff.dim 计算
# ============================================================

def compute_log_ratio_returns(prices: pd.DataFrame, node_col: str,
                              peer_cols: list[str]) -> pd.DataFrame:
    node_price = prices[node_col]
    returns = {}
    for i, peer in enumerate(peer_cols):
        ratio = node_price / prices[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns, index=prices.index).dropna()


def compute_effdim_series(returns: pd.DataFrame,
                          window: int = EFFDIM_WINDOW) -> pd.Series:
    values = []
    dates = []
    arr = returns.values
    for i in range(window, len(arr)):
        w = arr[i - window: i]
        cov = np.cov(w.T)
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 1e-15]
        if len(eigvals) == 0:
            values.append(1.0)
        else:
            p = eigvals / eigvals.sum()
            entropy = -np.sum(p * np.log(p))
            values.append(np.exp(entropy))
        dates.append(returns.index[i])
    return pd.Series(values, index=pd.DatetimeIndex(dates))


# ============================================================
# 三节点/四节点节点定义
# ============================================================

NODES_3 = {
    "$": {"col": "DXY", "peers": ["SPX", "WTI"]},
    "Equity": {"col": "SPX", "peers": ["DXY", "WTI"]},
    "Commodity": {"col": "WTI", "peers": ["DXY", "SPX"]},
}

NODES_4 = {
    "Au": {"col": "Au", "peers": ["DXY", "SPX", "WTI"]},
    "$": {"col": "DXY", "peers": ["Au", "SPX", "WTI"]},
    "Equity": {"col": "SPX", "peers": ["Au", "DXY", "WTI"]},
    "Commodity": {"col": "WTI", "peers": ["Au", "DXY", "SPX"]},
}


def compute_all_effdim(prices: pd.DataFrame,
                       node_defs: dict) -> dict[str, pd.Series]:
    result = {}
    for name, cfg in node_defs.items():
        rets = compute_log_ratio_returns(prices, cfg["col"], cfg["peers"])
        ed = compute_effdim_series(rets)
        result[name] = ed
    return result


# ============================================================
# 主分析: 全历史 + 体制窗口
# ============================================================

def main():
    print("=" * 70)
    print("体制内部 eff.dim 演化动态分析 v2")
    print("=" * 70)

    # ============================================================
    # 步骤 1: 下载全历史数据
    # ============================================================
    print("\n步骤 1: 下载全历史三节点数据 (1986-2026)...")

    dxy = download_yahoo("DX-Y.NYB", "1985-01-01", "2026-03-01")
    print(f"  DXY: {dxy.index[0].date()} to {dxy.index[-1].date()}, {len(dxy)} obs")

    spx = download_yahoo("^GSPC", "1985-01-01", "2026-03-01")
    print(f"  SPX: {spx.index[0].date()} to {spx.index[-1].date()}, {len(spx)} obs")

    wti_full = load_wti_from_csv(WTI_CSV_PATH)
    print(f"  WTI: {wti_full.index[0].date()} to {wti_full.index[-1].date()}, {len(wti_full)} obs")

    gc = download_yahoo("GC=F", "2000-01-01", "2026-03-01")
    print(f"  GC=F: {gc.index[0].date()} to {gc.index[-1].date()}, {len(gc)} obs")

    # 三节点对齐
    df3 = pd.DataFrame({"DXY": dxy, "SPX": spx, "WTI": wti_full}).dropna()
    neg_wti = df3["WTI"] <= 0
    if neg_wti.any():
        print(f"  过滤 {neg_wti.sum()} 个 WTI 非正值")
        df3 = df3[~neg_wti]
    print(f"  三节点对齐: {df3.index[0].date()} to {df3.index[-1].date()}, {len(df3)} obs")

    # 四节点对齐
    df4 = pd.DataFrame({"Au": gc, "DXY": dxy, "SPX": spx, "WTI": wti_full}).dropna()
    neg_wti4 = df4["WTI"] <= 0
    if neg_wti4.any():
        df4 = df4[~neg_wti4]
    print(f"  四节点对齐: {df4.index[0].date()} to {df4.index[-1].date()}, {len(df4)} obs")

    # ============================================================
    # 步骤 2: 计算全历史 eff.dim
    # ============================================================
    print("\n步骤 2: 计算全历史 eff.dim...")

    print("  三节点:")
    effdim3_full = compute_all_effdim(df3, NODES_3)
    for name, ed in effdim3_full.items():
        print(f"    {name}: {len(ed)} pts, {ed.index[0].date()} to {ed.index[-1].date()}, "
              f"mean={ed.mean():.3f}, p5={np.percentile(ed, 5):.4f}")

    print("  四节点:")
    effdim4_full = compute_all_effdim(df4, NODES_4)
    for name, ed in effdim4_full.items():
        print(f"    {name}: {len(ed)} pts, {ed.index[0].date()} to {ed.index[-1].date()}, "
              f"mean={ed.mean():.3f}, p5={np.percentile(ed, 5):.4f}")

    # 全样本 p5 阈值
    p5_3node = {name: float(np.percentile(ed, 5)) for name, ed in effdim3_full.items()}
    p5_4node = {name: float(np.percentile(ed, 5)) for name, ed in effdim4_full.items()}
    print(f"\n  三节点全样本 p5: {p5_3node}")
    print(f"  四节点全样本 p5: {p5_4node}")

    # ============================================================
    # 步骤 3: 分析每个体制
    # ============================================================

    regimes = [
        {
            "name": "1987",
            "regime_start": "1987-01-06",
            "regime_end": "1987-02-04",
            "ext_start": "1986-10-01",
            "ext_end": "1987-05-01",
            "n_nodes": 3,
            "desc": "1987 体制 (3节点: $/Equity/Commodity, 无Au)",
        },
        {
            "name": "1990-91",
            "regime_start": "1990-10-23",
            "regime_end": "1991-09-06",
            "ext_start": "1990-07-01",
            "ext_end": "1992-01-01",
            "n_nodes": 3,
            "desc": "1990-91 体制 (3节点: $/Equity/Commodity, 无Au)",
        },
        {
            "name": "2020-21",
            "regime_start": "2020-04-03",
            "regime_end": "2021-04-23",
            "ext_start": "2019-10-01",
            "ext_end": "2021-10-01",
            "n_nodes": 4,
            "desc": "2020-21 体制 (4节点: Au/$/Equity/Commodity)",
        },
    ]

    all_regime_results = []

    for reg in regimes:
        print(f"\n{'=' * 70}")
        print(f"体制: {reg['name']} ({reg['desc']})")
        print(f"{'=' * 70}")

        if reg["n_nodes"] == 3:
            effdim_full = effdim3_full
            p5_thresh = p5_3node
        else:
            effdim_full = effdim4_full
            p5_thresh = p5_4node

        regime_result = analyze_single_regime(
            effdim_full, p5_thresh, reg
        )
        all_regime_results.append(regime_result)

    # ============================================================
    # 步骤 4: 跨体制比较
    # ============================================================
    print(f"\n{'=' * 70}")
    print("跨体制比较")
    print(f"{'=' * 70}")

    comparison = cross_regime_comparison(all_regime_results)
    print_comparison(comparison)

    # ============================================================
    # 步骤 5: 保存结果
    # ============================================================
    output = {
        "meta": {
            "description": "三个同步压缩体制的 eff.dim 时间序列动态分析",
            "epistemological_level": "L2 (真实数据, 可否证)",
            "effdim_window": EFFDIM_WINDOW,
            "data_sources": {
                "USD": "Yahoo Finance DX-Y.NYB",
                "Equity": "Yahoo Finance ^GSPC",
                "Commodity": "FRED DCOILWTICO (WTI spot, 本地 CSV)",
                "Au": "Yahoo Finance GC=F (仅 2020-21 体制)",
            },
            "p5_thresholds": {
                "3node_full_sample": {k: round(v, 4) for k, v in p5_3node.items()},
                "4node_full_sample": {k: round(v, 4) for k, v in p5_4node.items()},
            },
            "3node_sample": {
                "start": str(list(effdim3_full.values())[0].index[0].date()),
                "end": str(list(effdim3_full.values())[0].index[-1].date()),
                "n_days": len(list(effdim3_full.values())[0]),
            },
            "4node_sample": {
                "start": str(list(effdim4_full.values())[0].index[0].date()),
                "end": str(list(effdim4_full.values())[0].index[-1].date()),
                "n_days": len(list(effdim4_full.values())[0]),
            },
        },
        "regimes": all_regime_results,
        "cross_regime_comparison": comparison,
    }

    json_path = OUTPUT_DIR / "effdim_dynamics_results.json"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False, default=str)
    print(f"\n结果已保存到 {json_path}")

    return output


def analyze_single_regime(
    effdim_full: dict[str, pd.Series],
    p5_thresh: dict[str, float],
    reg: dict,
) -> dict[str, Any]:
    """分析单个体制"""

    regime_start = pd.Timestamp(reg["regime_start"])
    regime_end = pd.Timestamp(reg["regime_end"])
    ext_start = pd.Timestamp(reg["ext_start"])
    ext_end = pd.Timestamp(reg["ext_end"])

    result: dict[str, Any] = {
        "regime_name": reg["name"],
        "description": reg["desc"],
        "regime_window": {"start": reg["regime_start"], "end": reg["regime_end"]},
        "extended_window": {"start": reg["ext_start"], "end": reg["ext_end"]},
        "p5_thresholds": {k: round(v, 4) for k, v in p5_thresh.items()},
        "nodes": {},
    }

    for node_name, ed_full in effdim_full.items():
        p5 = p5_thresh[node_name]

        # 扩展窗口内数据
        ext_mask = (ed_full.index >= ext_start) & (ed_full.index <= ext_end)
        ed_ext = ed_full[ext_mask]

        if len(ed_ext) == 0:
            result["nodes"][node_name] = {"error": "no data in extended window"}
            continue

        # 体制内数据
        reg_mask = (ed_full.index >= regime_start) & (ed_full.index <= regime_end)
        ed_regime = ed_full[reg_mask]

        # 体制前数据 (扩展窗口内, 体制前)
        pre_mask = (ed_full.index >= ext_start) & (ed_full.index < regime_start)
        ed_pre = ed_full[pre_mask]

        # 体制后数据 (扩展窗口内, 体制后)
        post_mask = (ed_full.index > regime_end) & (ed_full.index <= ext_end)
        ed_post = ed_full[post_mask]

        # ---- 完整时间序列 (扩展窗口) ----
        timeseries = []
        for dt, val in ed_ext.items():
            phase = "pre"
            if dt >= regime_start and dt <= regime_end:
                phase = "regime"
            elif dt > regime_end:
                phase = "post"
            timeseries.append({
                "date": str(dt.date()),
                "effdim": round(float(val), 4),
                "below_p5": bool(val < p5),
                "phase": phase,
            })

        # ---- 进入动态 ----
        entry = _analyze_entry_v2(ed_pre, ed_regime, ed_ext, p5)

        # ---- 内部动态 ----
        internal = _analyze_internal_v2(ed_regime, p5, regime_start, regime_end)

        # ---- 退出动态 ----
        exit_data = _analyze_exit_v2(ed_regime, ed_post, p5, regime_end)

        # ---- 基本统计 ----
        node_result = {
            "p5_threshold": round(p5, 4),
            "data_availability": {
                "ext_window_n": len(ed_ext),
                "pre_regime_n": len(ed_pre),
                "regime_n": len(ed_regime),
                "post_regime_n": len(ed_post),
                "first_effdim_date": str(ed_ext.index[0].date()),
            },
            "ext_stats": {
                "mean": round(float(ed_ext.mean()), 4),
                "std": round(float(ed_ext.std()), 4),
                "min": round(float(ed_ext.min()), 4),
                "max": round(float(ed_ext.max()), 4),
                "median": round(float(ed_ext.median()), 4),
            },
            "regime_stats": _safe_stats(ed_regime, p5),
            "entry": entry,
            "internal": internal,
            "exit": exit_data,
            "timeseries_sample": timeseries[:5] + ["..."] + timeseries[-5:] if len(timeseries) > 12 else timeseries,
        }
        result["nodes"][node_name] = node_result

        # ---- 打印 ----
        _print_node_summary(reg["name"], node_name, node_result)

    # ---- 同步分析 ----
    result["sync_analysis"] = _analyze_sync(
        effdim_full, p5_thresh, regime_start, regime_end
    )
    sync = result["sync_analysis"]
    print(f"\n  同步: {sync.get('n_all_sync_below_p5', '?')}/"
          f"{sync.get('total_days', '?')}天 "
          f"({sync.get('pct_all_sync', '?')}%) 全节点同时<p5")

    return result


def _safe_stats(series: pd.Series, p5: float) -> dict[str, Any]:
    if len(series) == 0:
        return {"n_days": 0}
    return {
        "n_days": len(series),
        "mean": round(float(series.mean()), 4),
        "std": round(float(series.std()), 4) if len(series) > 1 else 0.0,
        "min": round(float(series.min()), 4),
        "max": round(float(series.max()), 4),
        "median": round(float(series.median()), 4),
        "pct_below_p5": round(float((series < p5).mean()) * 100, 1),
    }


def _analyze_entry_v2(ed_pre: pd.Series, ed_regime: pd.Series,
                      ed_ext: pd.Series, p5: float) -> dict[str, Any]:
    """进入动态分析 v2"""
    if len(ed_pre) < 5:
        return {
            "data_insufficient": True,
            "pre_regime_n": len(ed_pre),
            "note": "扩展窗口内体制前数据不足 (可能是 rolling 252d 窗口的启动延迟)",
        }

    # 方法: 看 pre-regime 的 eff.dim 从什么水平开始下降
    # "正常水平" = pre-regime 前半段的中位数
    half_n = max(1, len(ed_pre) // 2)
    normal_level = float(ed_pre.iloc[:half_n].median())

    # 体制第一天的值
    regime_first = float(ed_regime.iloc[0]) if len(ed_regime) > 0 else None

    # 找 pre-regime 中 eff.dim 开始持续下降的转折点
    # 定义: 连续 N 天 eff.dim 递减 (用 5 天移动平均平滑)
    if len(ed_pre) >= 10:
        ed_smooth = ed_pre.rolling(5, min_periods=3).mean()
        # 找从正常水平开始的第一次连续下降
        descent_start = _find_descent_start(ed_smooth, normal_level)
    else:
        descent_start = None

    # 下降速率: 从 normal_level 到 p5 的 eff.dim 变化 / 天数
    below_p5_pre = ed_pre[ed_pre < p5]
    if len(below_p5_pre) > 0:
        first_below_date = below_p5_pre.index[0]
        days_above_to_below = len(ed_pre[(ed_pre.index >= ed_pre.index[0]) &
                                         (ed_pre.index <= first_below_date)])
    else:
        first_below_date = None
        days_above_to_below = None

    # 进入类型判断
    # 如果 pre-regime 的 eff.dim 已经全部 < p5 → 已在压缩状态
    if len(ed_pre[ed_pre >= p5]) == 0:
        entry_type = "already_compressed"
    elif days_above_to_below is not None and days_above_to_below <= 10:
        entry_type = "abrupt"
    elif days_above_to_below is not None and days_above_to_below <= 30:
        entry_type = "rapid"
    elif days_above_to_below is not None and days_above_to_below <= 60:
        entry_type = "gradual"
    elif days_above_to_below is not None:
        entry_type = "slow"
    else:
        entry_type = "no_p5_crossing_in_pre"

    # eff.dim 下降幅度
    pre_max = float(ed_pre.max())
    pre_min = float(ed_pre.min())
    regime_min_val = float(ed_regime.min()) if len(ed_regime) > 0 else None
    drop_pct = round((pre_max - (regime_min_val or pre_min)) / pre_max * 100, 1) if pre_max > 0 else None

    return {
        "data_insufficient": False,
        "pre_regime_n": len(ed_pre),
        "normal_level_median": round(normal_level, 4),
        "pre_max": round(pre_max, 4),
        "pre_min": round(pre_min, 4),
        "regime_first_value": round(regime_first, 4) if regime_first is not None else None,
        "first_below_p5_date": str(first_below_date.date()) if first_below_date is not None else None,
        "days_to_first_below_p5": days_above_to_below,
        "descent_start_date": descent_start,
        "entry_type": entry_type,
        "drop_from_max_pct": drop_pct,
    }


def _find_descent_start(ed_smooth: pd.Series, normal_level: float) -> str | None:
    """找持续下降的起点"""
    vals = ed_smooth.dropna().values
    if len(vals) < 10:
        return None

    # 找最后一次接近 normal_level 后开始持续下降的点
    # 简化: 找从右到左第一个 >= normal_level * 0.95 的点
    for i in range(len(vals) - 1, -1, -1):
        if vals[i] >= normal_level * 0.95:
            return str(ed_smooth.dropna().index[i].date())
    return None


def _analyze_internal_v2(ed_regime: pd.Series, p5: float,
                         regime_start: pd.Timestamp,
                         regime_end: pd.Timestamp) -> dict[str, Any]:
    """体制内部结构分析 v2"""
    if len(ed_regime) == 0:
        return {"error": "no regime data"}

    total_days = len(ed_regime)
    min_idx = ed_regime.idxmin()
    min_val = float(ed_regime.min())
    max_val = float(ed_regime.max())
    days_to_min = len(ed_regime[ed_regime.index <= min_idx])
    min_position_pct = round(days_to_min / total_days * 100, 1)

    # 分段统计 (四分位)
    q1 = total_days // 4
    q2 = total_days // 2
    q3 = 3 * total_days // 4
    quartile_means = []
    for start, end in [(0, q1), (q1, q2), (q2, q3), (q3, total_days)]:
        if end > start:
            quartile_means.append(round(float(ed_regime.iloc[start:end].mean()), 4))
        else:
            quartile_means.append(None)

    # 内部趋势
    if quartile_means[0] is not None and quartile_means[3] is not None:
        if quartile_means[3] < quartile_means[0] * 0.95:
            trend = "deepening"
        elif quartile_means[3] > quartile_means[0] * 1.05:
            trend = "recovering_late"
        elif (quartile_means[1] is not None and
              quartile_means[1] < quartile_means[0] * 0.95 and
              quartile_means[3] > quartile_means[1] * 1.05):
            trend = "V_shaped"
        else:
            trend = "flat"
    else:
        trend = "unknown"

    # 体制内低于 p5 的百分比
    pct_below = round((ed_regime < p5).mean() * 100, 1)

    # 局部极值
    local_minima = _find_local_extrema(ed_regime, 10)

    # 内部波动率
    if len(ed_regime) > 1:
        internal_volatility = round(float(ed_regime.std() / ed_regime.mean()), 4)
    else:
        internal_volatility = 0.0

    # 分类
    if pct_below < 20:
        pattern = "barely_compressed"     # 大部分时间在 p5 以上
    elif pct_below > 80:
        if trend == "deepening":
            pattern = "deep_and_deepening"
        elif trend == "recovering_late":
            pattern = "deep_then_recovering"
        else:
            pattern = "sustained_deep"
    else:
        pattern = "intermittent"          # 在 p5 上下波动

    return {
        "total_days": total_days,
        "min_value": round(min_val, 4),
        "min_date": str(min_idx.date()),
        "max_value": round(max_val, 4),
        "days_to_min": days_to_min,
        "min_position_pct": min_position_pct,
        "quartile_means": quartile_means,
        "trend": trend,
        "pct_below_p5": pct_below,
        "internal_volatility": internal_volatility,
        "pattern": pattern,
        "local_minima": local_minima,
    }


def _find_local_extrema(series: pd.Series, half_window: int = 10) -> list[dict]:
    """找局部最低点"""
    if len(series) < half_window * 3:
        return [{"date": str(series.idxmin().date()),
                 "value": round(float(series.min()), 4),
                 "note": "global minimum (series too short)"}]

    arr = series.values
    minima = []
    for i in range(half_window, len(arr) - half_window):
        left = arr[max(0, i - half_window): i]
        right = arr[i + 1: min(len(arr), i + half_window + 1)]
        if arr[i] <= left.min() and arr[i] <= right.min():
            minima.append({
                "date": str(series.index[i].date()),
                "value": round(float(arr[i]), 4),
            })

    if not minima:
        return [{"date": str(series.idxmin().date()),
                 "value": round(float(series.min()), 4),
                 "note": "global minimum"}]
    return minima


def _analyze_exit_v2(ed_regime: pd.Series, ed_post: pd.Series,
                     p5: float, regime_end: pd.Timestamp) -> dict[str, Any]:
    """退出动态分析 v2"""
    if len(ed_regime) == 0:
        return {"error": "no regime data"}

    regime_last = float(ed_regime.iloc[-1])

    if len(ed_post) < 5:
        return {
            "regime_end_value": round(regime_last, 4),
            "data_insufficient": True,
            "post_regime_n": len(ed_post),
        }

    # 体制后数据的正常化过程
    # 找第一次连续 10 天 >= p5 (稳定回到正常)
    stable_exit_date = None
    consecutive_above = 0
    for dt, val in ed_post.items():
        if val >= p5:
            consecutive_above += 1
            if consecutive_above >= 10:
                stable_exit_date = dt
                break
        else:
            consecutive_above = 0

    # 第一次 >= p5
    above_p5 = ed_post[ed_post >= p5]
    first_above_date = above_p5.index[0] if len(above_p5) > 0 else None

    if first_above_date is not None:
        days_to_first_above = len(ed_post[(ed_post.index <= first_above_date)])
    else:
        days_to_first_above = None

    if stable_exit_date is not None:
        days_to_stable = len(ed_post[(ed_post.index <= stable_exit_date)])
    else:
        days_to_stable = None

    # 退出后半年 (或可用数据) 的统计
    post_half = min(len(ed_post), 126)  # ~6个月
    post_half_mean = float(ed_post.iloc[:post_half].mean())
    post_half_trend = "recovering" if ed_post.iloc[:post_half].iloc[-1] > ed_post.iloc[:post_half].iloc[0] else "still_low"

    # 是否有反复 (回到 p5 上方后又跌回)?
    n_recrossings = 0
    above = False
    for val in ed_post.values:
        if val >= p5:
            if not above:
                above = True
        else:
            if above:
                n_recrossings += 1
                above = False

    # 退出速度分类
    if days_to_stable is not None:
        if days_to_stable <= 10:
            exit_type = "abrupt"
        elif days_to_stable <= 30:
            exit_type = "rapid"
        elif days_to_stable <= 90:
            exit_type = "gradual"
        else:
            exit_type = "slow"
    elif first_above_date is not None:
        if days_to_first_above <= 10:
            exit_type = "unstable_quick"
        else:
            exit_type = "unstable_slow"
    else:
        exit_type = "no_recovery"

    return {
        "regime_end_value": round(regime_last, 4),
        "post_regime_n": len(ed_post),
        "first_above_p5_date": str(first_above_date.date()) if first_above_date is not None else None,
        "days_to_first_above_p5": days_to_first_above,
        "stable_exit_date": str(stable_exit_date.date()) if stable_exit_date is not None else None,
        "days_to_stable_exit": days_to_stable,
        "n_recrossings": n_recrossings,
        "post_6m_mean": round(post_half_mean, 4),
        "post_6m_trend": post_half_trend,
        "exit_type": exit_type,
    }


def _analyze_sync(effdim_dict: dict[str, pd.Series],
                  p5_thresh: dict[str, float],
                  regime_start: pd.Timestamp,
                  regime_end: pd.Timestamp) -> dict[str, Any]:
    """跨节点同步分析"""
    below_frames = {}
    for name, ed in effdim_dict.items():
        mask = (ed.index >= regime_start) & (ed.index <= regime_end)
        below_frames[name] = (ed[mask] < p5_thresh[name])

    df_below = pd.DataFrame(below_frames).dropna()
    if len(df_below) == 0:
        return {"error": "no common dates"}

    n_total = len(df_below)
    n_sync_all = int(df_below.all(axis=1).sum())
    per_node = {}
    for name in df_below.columns:
        n_below = int(df_below[name].sum())
        per_node[name] = {
            "n_below_p5": n_below,
            "pct_below_p5": round(n_below / n_total * 100, 1),
        }

    return {
        "total_days": n_total,
        "n_all_sync_below_p5": n_sync_all,
        "pct_all_sync": round(n_sync_all / n_total * 100, 1),
        "per_node": per_node,
    }


def _print_node_summary(regime_name: str, node_name: str,
                        node_result: dict) -> None:
    """打印节点摘要"""
    entry = node_result.get("entry", {})
    internal = node_result.get("internal", {})
    exit_data = node_result.get("exit", {})

    avail = node_result.get("data_availability", {})
    print(f"\n  {node_name} (pre={avail.get('pre_regime_n', '?')}, "
          f"regime={avail.get('regime_n', '?')}, "
          f"post={avail.get('post_regime_n', '?')} 个 eff.dim 数据点):")

    if entry.get("data_insufficient"):
        print(f"    进入: 数据不足 ({entry.get('note', '')})")
    else:
        print(f"    进入: {entry.get('entry_type', 'N/A')}, "
              f"正常水平={entry.get('normal_level_median', '?')}, "
              f"首次<p5: {entry.get('first_below_p5_date', 'N/A')}, "
              f"下降{entry.get('drop_from_max_pct', '?')}%")

    if "error" not in internal:
        print(f"    内部: {internal.get('pattern', 'N/A')} "
              f"(trend={internal.get('trend', '?')}, "
              f"最低{internal.get('min_value', '?')}@{internal.get('min_date', '?')}, "
              f"位置{internal.get('min_position_pct', '?')}%, "
              f"{internal.get('pct_below_p5', '?')}%时间<p5)")
        print(f"    四分位均值: {internal.get('quartile_means', [])}")

    if exit_data.get("data_insufficient"):
        print(f"    退出: 数据不足")
    elif "error" not in exit_data:
        print(f"    退出: {exit_data.get('exit_type', 'N/A')}, "
              f"首次>=p5: {exit_data.get('first_above_p5_date', 'N/A')} "
              f"({exit_data.get('days_to_first_above_p5', '?')}天), "
              f"稳定退出: {exit_data.get('stable_exit_date', 'N/A')} "
              f"({exit_data.get('days_to_stable_exit', '?')}天), "
              f"反复{exit_data.get('n_recrossings', '?')}次")


# ============================================================
# 跨体制比较
# ============================================================

def cross_regime_comparison(all_results: list[dict]) -> dict[str, Any]:
    """跨体制共同模式"""
    entries = []
    internals = []
    exits = []
    asymmetries = []

    for r in all_results:
        for node_name, nd in r["nodes"].items():
            if "error" in nd:
                continue
            entry = nd.get("entry", {})
            internal = nd.get("internal", {})
            exit_d = nd.get("exit", {})

            entries.append({
                "regime": r["regime_name"], "node": node_name,
                "type": entry.get("entry_type"),
                "days": entry.get("days_to_first_below_p5"),
                "drop_pct": entry.get("drop_from_max_pct"),
            })
            internals.append({
                "regime": r["regime_name"], "node": node_name,
                "pattern": internal.get("pattern"),
                "trend": internal.get("trend"),
                "min_pos_pct": internal.get("min_position_pct"),
                "pct_below_p5": internal.get("pct_below_p5"),
            })
            exits.append({
                "regime": r["regime_name"], "node": node_name,
                "type": exit_d.get("exit_type"),
                "days_first": exit_d.get("days_to_first_above_p5"),
                "days_stable": exit_d.get("days_to_stable_exit"),
                "recrossings": exit_d.get("n_recrossings"),
            })

            # 不对称性
            entry_days = entry.get("days_to_first_below_p5")
            exit_stable = exit_d.get("days_to_stable_exit")
            if entry_days and exit_stable and entry_days > 0:
                ratio = exit_stable / entry_days
                asym = "fast_in_slow_out" if ratio > 2.0 else (
                    "slow_in_fast_out" if ratio < 0.5 else "approximately_symmetric"
                )
                asymmetries.append({
                    "regime": r["regime_name"], "node": node_name,
                    "entry_days": entry_days, "exit_days_stable": exit_stable,
                    "ratio": round(ratio, 2), "pattern": asym,
                })

    # 聚合
    entry_types = [e["type"] for e in entries if e["type"]]
    internal_patterns = [i["pattern"] for i in internals if i["pattern"]]
    exit_types = [e["type"] for e in exits if e["type"]]
    asym_patterns = [a["pattern"] for a in asymmetries if a["pattern"]]

    summary = {
        "entry_type_counts": dict(Counter(entry_types)),
        "internal_pattern_counts": dict(Counter(internal_patterns)),
        "exit_type_counts": dict(Counter(exit_types)),
        "asymmetry_pattern_counts": dict(Counter(asym_patterns)),
    }

    # 统计
    entry_days_vals = [e["days"] for e in entries if e["days"] is not None]
    exit_stable_vals = [e["days_stable"] for e in exits if e["days_stable"] is not None]
    if entry_days_vals:
        summary["entry_days_stats"] = {
            "mean": round(np.mean(entry_days_vals), 1),
            "median": round(float(np.median(entry_days_vals)), 1),
            "min": min(entry_days_vals),
            "max": max(entry_days_vals),
        }
    if exit_stable_vals:
        summary["exit_stable_days_stats"] = {
            "mean": round(np.mean(exit_stable_vals), 1),
            "median": round(float(np.median(exit_stable_vals)), 1),
            "min": min(exit_stable_vals),
            "max": max(exit_stable_vals),
        }

    return {
        "entries": entries,
        "internals": internals,
        "exits": exits,
        "asymmetries": asymmetries,
        "summary": summary,
    }


def print_comparison(comparison: dict) -> None:
    """打印跨体制比较"""
    s = comparison["summary"]
    print(f"\n  进入类型分布: {s.get('entry_type_counts', {})}")
    print(f"  内部模式分布: {s.get('internal_pattern_counts', {})}")
    print(f"  退出类型分布: {s.get('exit_type_counts', {})}")
    print(f"  不对称性分布: {s.get('asymmetry_pattern_counts', {})}")

    if "entry_days_stats" in s:
        es = s["entry_days_stats"]
        print(f"\n  进入天数: mean={es['mean']}, median={es['median']}, "
              f"range=[{es['min']}, {es['max']}]")
    if "exit_stable_days_stats" in s:
        xs = s["exit_stable_days_stats"]
        print(f"  稳定退出天数: mean={xs['mean']}, median={xs['median']}, "
              f"range=[{xs['min']}, {xs['max']}]")

    print("\n  不对称性详情:")
    for a in comparison.get("asymmetries", []):
        print(f"    {a['regime']:>8} {a['node']:>10}: "
              f"进入{a['entry_days']:>3}天, 稳定退出{a['exit_days_stable']:>3}天, "
              f"比={a['ratio']:.2f} ({a['pattern']})")

    # 内部最低点位置分布
    print("\n  内部最低点位置 (%):")
    for i in comparison.get("internals", []):
        print(f"    {i['regime']:>8} {i['node']:>10}: "
              f"最低点在 {i['min_pos_pct']}%, "
              f"pattern={i['pattern']}, trend={i['trend']}, "
              f"{i['pct_below_p5']}% 时间 < p5")


if __name__ == "__main__":
    results = main()
