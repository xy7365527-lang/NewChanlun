"""v109 结算通道可观测性实验 — Alpha Vantage 数据源。

两步实验设计：

Step 1：XAU/USD 替代 GLD 重跑 234号基线
  - 目的：测试 GLD 的 ETF 微观结构（交易时间、折溢价、创设赎回机制）
    是否污染了 234号 E-C 独立性结论
  - E=SPY, C=XAU/USD（现货黄金）, R=TLT, $=UUP
  - GLD 是 ETF，与 SPY 共享股票市场微观结构；XAU/USD 是现货，无股票市场中介
  - 判据：
    - XAU/USD 版 E-C 仍 independent → GLD ETF 微观结构未污染
    - XAU/USD 版 E-C 变为 amplitude/direction → 234号 E-C 独立可能是 ETF 微观结构伪影

Step 2：布伦特原油替代 GLD
  - 目的：测试石油（石油美元锚点）是否对缠论 D 算子可见——即结算通道的可观测性
  - E=SPY, C=BRENT（布伦特原油）, R=TLT, $=UUP
  - 三个情景：
    - A：全吸收（与 DBA 相同）→ 纯商品，D 算子看不见
    - B：部分保留/放大（类似 GLD）→ 石油美元具有结算通道特征
    - C：第三种模式

数据源：Alpha Vantage（直接 HTTP 请求）
  - XAU/USD：FX_DAILY（from_symbol=XAU, to_symbol=USD）
  - BRENT：Commodities endpoint（function=BRENT&interval=daily）
  - SPY/TLT/UUP：TIME_SERIES_DAILY（outputsize=full）
"""

from __future__ import annotations

import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np
import requests

_PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

from newchan.bi_engine import BiEngine
from newchan.topology.discretization_kernel import (
    CouplingClassification,
    classify_coupling,
)
from newchan.types import Bar


# ---------------------------------------------------------------------------
# Alpha Vantage API 常量
# ---------------------------------------------------------------------------

_AV_BASE_URL = "https://www.alphavantage.co/query"

# Alpha Vantage 免费版：5 calls/min
_API_CALL_INTERVAL_SEC = 15  # 保守间隔，避免触发限流


# ---------------------------------------------------------------------------
# 234号基准数据（对比用）
# ---------------------------------------------------------------------------

_BASELINE_234 = {
    "E-R": {"partial_corr": -0.317, "beta": -0.020, "type": "amplitude", "in_kernel": True, "absorption": 0.949},
    "E-$": {"partial_corr": -0.183, "beta": -0.006, "type": "amplitude", "in_kernel": True, "absorption": 0.967},
    "C-$": {"partial_corr": -0.414, "beta": -0.030, "type": "amplitude", "in_kernel": True, "absorption": 0.928},
    "R-$": {"partial_corr": -0.065, "beta": 0.050, "type": "amplitude", "in_kernel": True, "absorption": 0.233},
    "C-R": {"partial_corr": 0.166, "beta": 0.009, "type": "direction", "in_kernel": False},
    "E-C": {"partial_corr": -0.029, "beta": 0.023, "type": "independent", "in_kernel": False},
}

# 259号 DBA 实验结果（如有）
_BASELINE_259: dict[str, Any] = {}  # 由 v108 实验填充，此处仅作对比占位

# 234号时间窗口
_234_START = "2007-03-01"
_234_END = "2026-02-26"


# ---------------------------------------------------------------------------
# Alpha Vantage 数据获取
# ---------------------------------------------------------------------------


def _get_api_key() -> str:
    """从环境变量获取 Alpha Vantage API key。"""
    key = os.environ.get("ALPHA_VANTAGE_API_KEY", "")
    if not key:
        raise RuntimeError(
            "ALPHA_VANTAGE_API_KEY 环境变量未设置。"
            "请设置后重新运行：export ALPHA_VANTAGE_API_KEY=your_key"
        )
    return key


def _rate_limit_sleep() -> None:
    """API 调用间隔等待。"""
    print(f"  [RATE LIMIT] 等待 {_API_CALL_INTERVAL_SEC}s...")
    time.sleep(_API_CALL_INTERVAL_SEC)


def fetch_equity(symbol: str, api_key: str) -> list[Bar]:
    """通过 Alpha Vantage TIME_SERIES_DAILY 获取股票/ETF 日线数据。

    Parameters
    ----------
    symbol : str
        股票代码（SPY/TLT/UUP）。
    api_key : str
        Alpha Vantage API key。

    Returns
    -------
    list[Bar]
        按时间升序排列的 Bar 列表。
    """
    print(f"[FETCH] {symbol} via Alpha Vantage TIME_SERIES_DAILY...")
    params = {
        "function": "TIME_SERIES_DAILY",
        "symbol": symbol,
        "outputsize": "full",
        "apikey": api_key,
    }
    resp = requests.get(_AV_BASE_URL, params=params, timeout=60)
    resp.raise_for_status()
    data = resp.json()

    if "Error Message" in data:
        raise RuntimeError(f"Alpha Vantage error for {symbol}: {data['Error Message']}")
    if "Note" in data:
        raise RuntimeError(f"Alpha Vantage rate limit for {symbol}: {data['Note']}")

    ts_key = "Time Series (Daily)"
    if ts_key not in data:
        raise RuntimeError(
            f"Alpha Vantage 返回中无 '{ts_key}'。"
            f"返回的 keys: {list(data.keys())}"
        )

    time_series = data[ts_key]
    bars: list[Bar] = []
    for date_str, ohlcv in time_series.items():
        bars.append(
            Bar(
                ts=datetime.strptime(date_str, "%Y-%m-%d"),
                open=float(ohlcv["1. open"]),
                high=float(ohlcv["2. high"]),
                low=float(ohlcv["3. low"]),
                close=float(ohlcv["4. close"]),
                volume=float(ohlcv["5. volume"]),
            )
        )

    bars.sort(key=lambda b: b.ts)
    print(f"  -> {len(bars)} bars ({bars[0].ts.date()} ~ {bars[-1].ts.date()})")
    return bars


def fetch_fx(from_sym: str, to_sym: str, api_key: str) -> list[Bar]:
    """通过 Alpha Vantage FX_DAILY 获取外汇/贵金属日线数据。

    用于 XAU/USD（现货黄金兑美元）。

    Parameters
    ----------
    from_sym : str
        源货币（如 XAU）。
    to_sym : str
        目标货币（如 USD）。
    api_key : str
        Alpha Vantage API key。

    Returns
    -------
    list[Bar]
        按时间升序排列的 Bar 列表。
    """
    print(f"[FETCH] {from_sym}/{to_sym} via Alpha Vantage FX_DAILY...")
    params = {
        "function": "FX_DAILY",
        "from_symbol": from_sym,
        "to_symbol": to_sym,
        "outputsize": "full",
        "apikey": api_key,
    }
    resp = requests.get(_AV_BASE_URL, params=params, timeout=60)
    resp.raise_for_status()
    data = resp.json()

    if "Error Message" in data:
        raise RuntimeError(f"Alpha Vantage error for {from_sym}/{to_sym}: {data['Error Message']}")
    if "Note" in data:
        raise RuntimeError(f"Alpha Vantage rate limit for {from_sym}/{to_sym}: {data['Note']}")

    ts_key = "Time Series FX (Daily)"
    if ts_key not in data:
        raise RuntimeError(
            f"Alpha Vantage 返回中无 '{ts_key}'。"
            f"返回的 keys: {list(data.keys())}"
        )

    time_series = data[ts_key]
    bars: list[Bar] = []
    for date_str, ohlc in time_series.items():
        bars.append(
            Bar(
                ts=datetime.strptime(date_str, "%Y-%m-%d"),
                open=float(ohlc["1. open"]),
                high=float(ohlc["2. high"]),
                low=float(ohlc["3. low"]),
                close=float(ohlc["4. close"]),
                volume=None,  # FX_DAILY 无成交量
            )
        )

    bars.sort(key=lambda b: b.ts)
    print(f"  -> {len(bars)} bars ({bars[0].ts.date()} ~ {bars[-1].ts.date()})")
    return bars


def fetch_commodity_brent(api_key: str) -> list[Bar]:
    """通过 Alpha Vantage Commodities 端点获取布伦特原油日线数据。

    端点：function=BRENT&interval=daily

    注意：Alpha Vantage Commodities 端点返回格式与 TIME_SERIES_DAILY 不同：
    返回 {"name": "...", "interval": "daily", "unit": "...", "data": [{"date": "...", "value": "..."}, ...]}
    只有日期和收盘价（value），无 OHLC。为兼容 BiEngine 管线，
    使用 value 同时填充 open/high/low/close。

    Parameters
    ----------
    api_key : str
        Alpha Vantage API key。

    Returns
    -------
    list[Bar]
        按时间升序排列的 Bar 列表。
    """
    print("[FETCH] BRENT via Alpha Vantage Commodities endpoint...")
    params = {
        "function": "BRENT",
        "interval": "daily",
        "apikey": api_key,
    }
    resp = requests.get(_AV_BASE_URL, params=params, timeout=60)
    resp.raise_for_status()
    data = resp.json()

    if "Error Message" in data:
        raise RuntimeError(f"Alpha Vantage error for BRENT: {data['Error Message']}")
    if "Note" in data:
        raise RuntimeError(f"Alpha Vantage rate limit for BRENT: {data['Note']}")

    if "data" not in data:
        raise RuntimeError(
            f"Alpha Vantage BRENT 返回中无 'data'。"
            f"返回的 keys: {list(data.keys())}"
        )

    bars: list[Bar] = []
    for entry in data["data"]:
        value_str = entry.get("value", "")
        if value_str == "." or not value_str:
            continue  # 跳过缺失值
        price = float(value_str)
        bars.append(
            Bar(
                ts=datetime.strptime(entry["date"], "%Y-%m-%d"),
                open=price,
                high=price,
                low=price,
                close=price,
                volume=None,
            )
        )

    bars.sort(key=lambda b: b.ts)
    print(f"  -> {len(bars)} bars ({bars[0].ts.date()} ~ {bars[-1].ts.date()})")
    return bars


# ---------------------------------------------------------------------------
# 数据对齐
# ---------------------------------------------------------------------------


def align_bars(
    all_bars: dict[str, list[Bar]],
    symbols: tuple[str, ...],
    start_date: str | None = None,
    end_date: str | None = None,
) -> tuple[list[str], dict[str, np.ndarray], dict[str, list[Bar]]]:
    """按日期对齐多个标的。

    Parameters
    ----------
    all_bars : dict[str, list[Bar]]
        各标的的 Bar 列表。
    symbols : tuple[str, ...]
        参与对齐的标的列表。
    start_date : str | None
        起始日期过滤（含），格式 YYYY-MM-DD。
    end_date : str | None
        结束日期过滤（含），格式 YYYY-MM-DD。

    Returns
    -------
    common_dates : list[str]
        公共交易日列表。
    closes : dict[str, np.ndarray]
        各标的收盘价数组。
    aligned : dict[str, list[Bar]]
        按公共日期过滤的 bar 列表。
    """
    date_maps: dict[str, dict[str, Bar]] = {}
    for sym in symbols:
        date_maps[sym] = {b.ts.strftime("%Y-%m-%d"): b for b in all_bars[sym]}

    common_dates_set: set[str] | None = None
    for sym in symbols:
        dates = set(date_maps[sym].keys())
        common_dates_set = dates if common_dates_set is None else common_dates_set & dates

    assert common_dates_set is not None
    common_dates = sorted(common_dates_set)

    # 日期范围过滤
    if start_date:
        common_dates = [d for d in common_dates if d >= start_date]
    if end_date:
        common_dates = [d for d in common_dates if d <= end_date]

    print(f"[ALIGN] {len(common_dates)} common trading days")
    if common_dates:
        print(f"  range: {common_dates[0]} ~ {common_dates[-1]}")
    else:
        raise RuntimeError("对齐后无公共交易日，请检查数据范围。")

    closes: dict[str, np.ndarray] = {}
    aligned: dict[str, list[Bar]] = {}
    for sym in symbols:
        closes[sym] = np.array(
            [date_maps[sym][d].close for d in common_dates],
            dtype=np.float64,
        )
        aligned[sym] = sorted(
            [date_maps[sym][d] for d in common_dates],
            key=lambda b: b.ts,
        )

    return common_dates, closes, aligned


# ---------------------------------------------------------------------------
# 连续层计算
# ---------------------------------------------------------------------------


def compute_returns(closes: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
    """计算各标的的日对数收益率。"""
    return {sym: np.diff(np.log(c)) for sym, c in closes.items()}


def pearson(x: np.ndarray, y: np.ndarray) -> float:
    """Pearson 相关系数。"""
    return float(np.corrcoef(x, y)[0, 1])


def partial_corr_controlling(
    returns: dict[str, np.ndarray],
    sym_a: str,
    sym_b: str,
    controls: list[str],
) -> float:
    """计算偏相关系数，控制其他变量。

    使用回归残差法：
    1. 对 sym_a 和 sym_b 分别对 controls 做 OLS 回归
    2. 取残差
    3. 计算残差间的 Pearson 相关
    """
    r_a = returns[sym_a]
    r_b = returns[sym_b]

    if not controls:
        return pearson(r_a, r_b)

    X = np.column_stack([returns[c] for c in controls])
    X_with_intercept = np.column_stack([np.ones(len(r_a)), X])

    coeffs_a, _, _, _ = np.linalg.lstsq(X_with_intercept, r_a, rcond=None)
    resid_a = r_a - X_with_intercept @ coeffs_a

    coeffs_b, _, _, _ = np.linalg.lstsq(X_with_intercept, r_b, rcond=None)
    resid_b = r_b - X_with_intercept @ coeffs_b

    return pearson(resid_a, resid_b)


def compute_edge_continuous(
    returns: dict[str, np.ndarray],
    edges: dict[str, tuple[str, str]],
    symbols: tuple[str, ...],
) -> dict[str, dict[str, float]]:
    """计算边的连续层 Pearson 相关 + 偏相关。"""
    result: dict[str, dict[str, float]] = {}

    for edge_name, (sym_a, sym_b) in edges.items():
        r = pearson(returns[sym_a], returns[sym_b])
        other_syms = [s for s in symbols if s != sym_a and s != sym_b]
        pc = partial_corr_controlling(returns, sym_a, sym_b, other_syms)
        result[edge_name] = {
            "pearson": round(r, 6),
            "partial_corr": round(pc, 6),
        }

    return result


# ---------------------------------------------------------------------------
# 离散层计算：BiEngine 生成笔方向序列
# ---------------------------------------------------------------------------


def run_bi_engine(bars: list[Bar]) -> list[tuple[int, int, str]]:
    """对 bar 序列运行 BiEngine，返回 confirmed 笔列表。"""
    engine = BiEngine(stroke_mode="new")
    for bar in bars:
        engine.process_bar(bar)

    strokes = engine.current_strokes
    return [(s.i0, s.i1, s.direction) for s in strokes]


def strokes_to_bar_direction(
    strokes: list[tuple[int, int, str]], n_bars: int,
) -> np.ndarray:
    """将笔序列映射为逐 bar 的方向数组。up -> +1, down -> -1, 无笔覆盖 -> 0。"""
    direction = np.zeros(n_bars, dtype=np.int8)
    for i0, i1, d in strokes:
        start = max(0, i0)
        end = min(n_bars, i1 + 1)
        val = 1 if d == "up" else -1
        direction[start:end] = val
    return direction


def compute_discrete_correlation(
    dir_a: np.ndarray, dir_b: np.ndarray,
) -> dict[str, float]:
    """计算两个离散方向序列之间的相关性。"""
    mask = (dir_a != 0) & (dir_b != 0)
    n_active = int(mask.sum())

    if n_active < 30:
        return {
            "discrete_pearson": 0.0,
            "beta": 0.0,
            "n_active": n_active,
            "concordance": 0.0,
            "note": "insufficient active bars",
        }

    a_active = dir_a[mask].astype(np.float64)
    b_active = dir_b[mask].astype(np.float64)

    disc_pearson = float(np.corrcoef(a_active, b_active)[0, 1])
    concordance = float(np.mean(a_active == b_active))

    X = np.column_stack([np.ones(n_active), a_active])
    coeffs, _, _, _ = np.linalg.lstsq(X, b_active, rcond=None)
    beta = float(coeffs[1])

    return {
        "discrete_pearson": round(disc_pearson, 6),
        "beta": round(beta, 6),
        "concordance": round(concordance, 6),
        "n_active": n_active,
    }


# ---------------------------------------------------------------------------
# 分类
# ---------------------------------------------------------------------------


def classify_all_edges(
    continuous: dict[str, dict[str, float]],
    discrete: dict[str, dict[str, float]],
    edges: dict[str, tuple[str, str]],
    symbol_labels: dict[str, str],
) -> dict[str, dict[str, Any]]:
    """对所有边进行 ker(D) 分类。"""
    results: dict[str, dict[str, Any]] = {}

    for edge_name, (sym_a, sym_b) in edges.items():
        cont = continuous[edge_name]
        disc = discrete.get(edge_name, {})

        partial_corr_val = cont["partial_corr"]
        beta_val = disc.get("beta", 0.0)

        classification = classify_coupling(
            partial_corr=partial_corr_val,
            beta=beta_val,
        )

        label_a = symbol_labels.get(sym_a, sym_a)
        label_b = symbol_labels.get(sym_b, sym_b)

        results[edge_name] = {
            "symbols": [sym_a, sym_b],
            "labels": [label_a, label_b],
            "continuous": {
                "pearson": cont["pearson"],
                "partial_corr": cont["partial_corr"],
            },
            "discrete": disc,
            "classification": {
                "coupling_type": classification.coupling_type.value,
                "in_kernel": classification.in_kernel,
                "absorption_ratio": round(classification.absorption_ratio, 6),
                "dominant_layer": classification.dominant_layer,
            },
        }

    return results


# ---------------------------------------------------------------------------
# 与 234号对比
# ---------------------------------------------------------------------------


def compare_with_234(
    classifications: dict[str, dict[str, Any]],
    edge_mapping: dict[str, str],
    control_edges: set[str],
) -> dict[str, dict[str, Any]]:
    """将实验结果与 234号基准逐边对比。

    Parameters
    ----------
    classifications : dict
        本次实验的分类结果。
    edge_mapping : dict
        本次实验边名 -> 234号边名 的映射。
    control_edges : set
        控制变量边集合（不变边）。
    """
    comparisons: dict[str, dict[str, Any]] = {}

    for edge_new, edge_234 in edge_mapping.items():
        if edge_new not in classifications:
            continue
        new = classifications[edge_new]
        old = _BASELINE_234.get(edge_234, {})
        if not old:
            continue

        comparisons[edge_new] = {
            "edge_234": edge_234,
            "is_control": edge_new in control_edges,
            "new_type": new["classification"]["coupling_type"],
            "old_type": old["type"],
            "new_in_kernel": new["classification"]["in_kernel"],
            "old_in_kernel": old["in_kernel"],
            "new_partial_corr": new["continuous"]["partial_corr"],
            "old_partial_corr": old["partial_corr"],
            "new_beta": new["discrete"].get("beta", 0.0),
            "old_beta": old["beta"],
            "new_absorption": new["classification"]["absorption_ratio"],
            "old_absorption": old.get("absorption", None),
            "type_match": new["classification"]["coupling_type"] == old["type"],
        }

    return comparisons


# ---------------------------------------------------------------------------
# 共享管线：获取数据 → 对齐 → 连续层 → 离散层 → 分类 → 对比
# ---------------------------------------------------------------------------


def run_pipeline(
    all_bars: dict[str, list[Bar]],
    symbols: tuple[str, ...],
    symbol_labels: dict[str, str],
    edges: dict[str, tuple[str, str]],
    edge_mapping_234: dict[str, str],
    control_edges: set[str],
    step_name: str,
) -> dict[str, Any]:
    """共享的分析管线。

    Returns
    -------
    dict
        包含完整分析结果的字典。
    """
    print(f"\n{'=' * 60}")
    print(f"  {step_name}")
    print(f"{'=' * 60}")

    # 1. 对齐——取与 234号时间窗口的交集
    print(f"\n--- 对齐数据（目标窗口 {_234_START} ~ {_234_END}）---")
    common_dates, closes, aligned = align_bars(
        all_bars, symbols, start_date=_234_START, end_date=_234_END,
    )
    n_bars = len(common_dates)

    # 2. 连续层
    print("\n--- 连续层 Pearson + 偏相关 ---")
    returns = compute_returns(closes)
    continuous = compute_edge_continuous(returns, edges, symbols)
    for edge_name, data in continuous.items():
        print(f"  {edge_name}: pearson={data['pearson']:.4f}, partial_corr={data['partial_corr']:.4f}")

    # 3. 离散层
    print("\n--- BiEngine 离散化 ---")
    bi_directions: dict[str, np.ndarray] = {}
    for sym in symbols:
        bars = aligned[sym]
        strokes = run_bi_engine(bars)
        n_strokes = len(strokes)
        direction_arr = strokes_to_bar_direction(strokes, n_bars)
        n_nonzero = int(np.count_nonzero(direction_arr))
        print(f"  {sym} ({symbol_labels.get(sym, sym)}): {n_strokes} strokes, "
              f"{n_nonzero}/{n_bars} bars with direction")
        bi_directions[sym] = direction_arr

    print("\n--- 离散方向相关 ---")
    discrete: dict[str, dict[str, float]] = {}
    for edge_name, (sym_a, sym_b) in edges.items():
        disc = compute_discrete_correlation(bi_directions[sym_a], bi_directions[sym_b])
        discrete[edge_name] = disc
        print(f"  {edge_name}: beta={disc['beta']:.4f}, "
              f"concordance={disc.get('concordance', 0):.4f}, "
              f"n_active={disc['n_active']}")

    # 4. 分类
    print("\n--- 六条边分类 ---")
    classifications = classify_all_edges(continuous, discrete, edges, symbol_labels)
    for edge_name, data in classifications.items():
        cls = data["classification"]
        marker = "ker(D)" if cls["in_kernel"] else "ker(D)"
        if cls["in_kernel"]:
            marker = "in ker(D)"
        elif cls["coupling_type"] == "independent":
            marker = "independent"
        else:
            marker = "NOT ker(D)"
        print(f"  {edge_name} [{data['labels'][0]} vs {data['labels'][1]}]: "
              f"{cls['coupling_type']} | {marker} | "
              f"absorption={cls['absorption_ratio']:.3f}")

    # 5. 与 234号对比
    print("\n--- 与 234号基准对比 ---")
    comparisons = compare_with_234(classifications, edge_mapping_234, control_edges)

    print("\n  === 控制变量（不变边） ===")
    for edge_name, comp in comparisons.items():
        if comp["is_control"]:
            match_str = "MATCH" if comp["type_match"] else "CHANGED"
            print(f"  {edge_name} (=234 {comp['edge_234']}): "
                  f"type {comp['old_type']}->{comp['new_type']} {match_str} | "
                  f"pc {comp['old_partial_corr']:.3f}->{comp['new_partial_corr']:.3f} | "
                  f"beta {comp['old_beta']:.3f}->{comp['new_beta']:.3f}")

    print("\n  === 实验变量（替换边） ===")
    for edge_name, comp in comparisons.items():
        if not comp["is_control"]:
            old_abs = comp["old_absorption"]
            old_abs_str = f"{old_abs:.3f}" if old_abs is not None else "N/A"
            print(f"  {edge_name} (<-234 {comp['edge_234']}): "
                  f"type {comp['old_type']}->{comp['new_type']} | "
                  f"pc {comp['old_partial_corr']:.3f}->{comp['new_partial_corr']:.3f} | "
                  f"beta {comp['old_beta']:.3f}->{comp['new_beta']:.3f} | "
                  f"abs {old_abs_str}->{comp['new_absorption']:.3f}")

    # 汇总
    n_in_kernel = sum(1 for e in classifications.values() if e["classification"]["in_kernel"])
    n_direction = sum(1 for e in classifications.values()
                      if e["classification"]["coupling_type"] == "direction")
    n_independent = sum(1 for e in classifications.values()
                        if e["classification"]["coupling_type"] == "independent")

    return {
        "data_info": {
            "n_common_bars": n_bars,
            "date_range": f"{common_dates[0]} ~ {common_dates[-1]}",
            "symbols": {sym: symbol_labels.get(sym, sym) for sym in symbols},
        },
        "edges": classifications,
        "comparison_with_234": comparisons,
        "summary": {
            "n_in_kernel": n_in_kernel,
            "n_direction": n_direction,
            "n_independent": n_independent,
        },
    }


# ---------------------------------------------------------------------------
# Step 1：XAU/USD 替代 GLD
# ---------------------------------------------------------------------------


def run_step1(api_key: str) -> dict[str, Any]:
    """Step 1：XAU/USD 替代 GLD 重跑 234号基线。

    E=SPY, C=XAU/USD（现货黄金）, R=TLT, $=UUP
    测试 GLD 的 ETF 微观结构是否污染了 234号 E-C 独立性结论。
    """
    symbols = ("SPY", "XAUUSD", "TLT", "UUP")
    symbol_labels = {"SPY": "E", "XAUUSD": "C_spot", "TLT": "R", "UUP": "$"}

    edges = {
        "E-R": ("SPY", "TLT"),
        "E-C_spot": ("SPY", "XAUUSD"),
        "E-$": ("SPY", "UUP"),
        "C_spot-R": ("XAUUSD", "TLT"),
        "C_spot-$": ("XAUUSD", "UUP"),
        "R-$": ("TLT", "UUP"),
    }

    # 边映射：本次边名 -> 234号边名
    edge_mapping_234 = {
        "E-R": "E-R",
        "E-C_spot": "E-C",
        "E-$": "E-$",
        "C_spot-R": "C-R",
        "C_spot-$": "C-$",
        "R-$": "R-$",
    }

    control_edges = {"E-R", "E-$", "R-$"}

    # 获取数据
    print("\n" + "=" * 70)
    print("Step 1：获取数据")
    print("=" * 70)

    all_bars: dict[str, list[Bar]] = {}

    all_bars["SPY"] = fetch_equity("SPY", api_key)
    _rate_limit_sleep()

    all_bars["TLT"] = fetch_equity("TLT", api_key)
    _rate_limit_sleep()

    all_bars["UUP"] = fetch_equity("UUP", api_key)
    _rate_limit_sleep()

    all_bars["XAUUSD"] = fetch_fx("XAU", "USD", api_key)
    # 最后一个 API 调用，不需要额外等待

    # 运行管线
    result = run_pipeline(
        all_bars=all_bars,
        symbols=symbols,
        symbol_labels=symbol_labels,
        edges=edges,
        edge_mapping_234=edge_mapping_234,
        control_edges=control_edges,
        step_name="Step 1: XAU/USD 替代 GLD — ETF 微观结构污染检测",
    )

    # Step 1 特有判据
    ec_spot = result["edges"]["E-C_spot"]
    ec_cls = ec_spot["classification"]
    ec_type = ec_cls["coupling_type"]

    if ec_type == "independent":
        verdict = {
            "scenario": "A-clean",
            "conclusion": (
                f"XAU/USD 版 E-C 仍为 independent "
                f"(partial_corr={ec_spot['continuous']['partial_corr']:.4f})。"
                f"GLD ETF 微观结构未污染 234号 E-C 独立性结论。"
                f"E-C 独立是黄金资产的内在性质，不依赖 ETF 中介。"
            ),
        }
    else:
        verdict = {
            "scenario": "B-contaminated",
            "conclusion": (
                f"XAU/USD 版 E-C 分类为 {ec_type} "
                f"(partial_corr={ec_spot['continuous']['partial_corr']:.4f}, "
                f"beta={ec_spot['discrete'].get('beta', 0.0):.4f})。"
                f"与 234号 E-C independent 不同。"
                f"234号 E-C 独立性可能受 GLD ETF 微观结构影响——"
                f"GLD 与 SPY 共享股票市场交易机制可能人为抑制了真实耦合。"
            ),
        }

    # 同时检查 C_spot-$ 对比 234号 C-$
    cs_spot = result["edges"]["C_spot-$"]
    cs_cls = cs_spot["classification"]
    cs_note = (
        f"C_spot(XAU/USD)-$: {cs_cls['coupling_type']}, "
        f"absorption={cs_cls['absorption_ratio']:.3f} "
        f"(234号 C-$: amplitude, absorption=0.928)"
    )
    verdict["c_dollar_note"] = cs_note

    result["verdict"] = verdict
    result["experiment_design"] = {
        "purpose": "测试 GLD ETF 微观结构是否污染 234号 E-C 独立性结论",
        "replacement": "GLD (ETF) -> XAU/USD (现货黄金)",
        "controls": "SPY(E), TLT(R), UUP($) 不变",
        "hypothesis": "如果 E-C 独立是黄金内在性质而非 ETF 伪影，XAU/USD 版应保持 independent",
    }

    print(f"\n--- Step 1 判定 ---")
    print(f"  情景: {verdict['scenario']}")
    print(f"  结论: {verdict['conclusion']}")
    print(f"  C-$ 附注: {cs_note}")

    return result


# ---------------------------------------------------------------------------
# Step 2：布伦特原油替代 GLD
# ---------------------------------------------------------------------------


def run_step2(api_key: str) -> dict[str, Any]:
    """Step 2：布伦特原油替代 GLD。

    E=SPY, C=BRENT（布伦特原油）, R=TLT, $=UUP
    测试石油（石油美元锚点）是否对缠论 D 算子可见——结算通道的可观测性。
    """
    symbols = ("SPY", "BRENT", "TLT", "UUP")
    symbol_labels = {"SPY": "E", "BRENT": "C_oil", "TLT": "R", "UUP": "$"}

    edges = {
        "E-R": ("SPY", "TLT"),
        "E-C_oil": ("SPY", "BRENT"),
        "E-$": ("SPY", "UUP"),
        "C_oil-R": ("BRENT", "TLT"),
        "C_oil-$": ("BRENT", "UUP"),
        "R-$": ("TLT", "UUP"),
    }

    edge_mapping_234 = {
        "E-R": "E-R",
        "E-C_oil": "E-C",
        "E-$": "E-$",
        "C_oil-R": "C-R",
        "C_oil-$": "C-$",
        "R-$": "R-$",
    }

    control_edges = {"E-R", "E-$", "R-$"}

    # 获取数据
    print("\n" + "=" * 70)
    print("Step 2：获取数据")
    print("=" * 70)

    all_bars: dict[str, list[Bar]] = {}

    all_bars["SPY"] = fetch_equity("SPY", api_key)
    _rate_limit_sleep()

    all_bars["TLT"] = fetch_equity("TLT", api_key)
    _rate_limit_sleep()

    all_bars["UUP"] = fetch_equity("UUP", api_key)
    _rate_limit_sleep()

    all_bars["BRENT"] = fetch_commodity_brent(api_key)
    # 最后一个 API 调用，不需要额外等待

    # BRENT 数据特殊处理说明
    # Alpha Vantage Commodities 端点仅返回 date+value（收盘价），无 OHLC。
    # BiEngine 对 open=high=low=close 的 bar 可能生成较少的笔（无价格振荡），
    # 这是数据源限制，不是管线错误。记录此约束。
    brent_data_note = (
        "BRENT 数据来自 Alpha Vantage Commodities 端点，仅有收盘价（无 OHLC）。"
        "open=high=low=close，BiEngine 处理此类 bar 时包含处理无效（无高低价差异），"
        "可能导致笔数量偏少。此为数据源约束。"
    )
    print(f"\n  [NOTE] {brent_data_note}")

    # 运行管线
    result = run_pipeline(
        all_bars=all_bars,
        symbols=symbols,
        symbol_labels=symbol_labels,
        edges=edges,
        edge_mapping_234=edge_mapping_234,
        control_edges=control_edges,
        step_name="Step 2: BRENT 替代 GLD — 石油美元结算通道可观测性",
    )

    # Step 2 特有判据：三情景
    co_dollar = result["edges"]["C_oil-$"]
    co_cls = co_dollar["classification"]
    co_type = co_cls["coupling_type"]
    co_absorption = co_cls["absorption_ratio"]

    ec_oil = result["edges"]["E-C_oil"]
    ec_cls = ec_oil["classification"]

    if co_type == "amplitude" and co_cls["in_kernel"]:
        scenario = "A-absorbed"
        conclusion = (
            f"C_oil(BRENT)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"全吸收——与 DBA（259号）模式相同。"
            f"石油对 D 算子不可见，纯商品行为。"
            f"石油美元在缠论离散化层面无结算通道特征。"
        )
    elif co_type == "direction" or co_type == "amplified":
        scenario = "B-channel"
        conclusion = (
            f"C_oil(BRENT)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"部分保留/放大——石油美元具有结算通道特征。"
            f"石油-美元关系在缠论 D 算子下不被完全吸收，"
            f"方向耦合穿透离散化滤波器。"
            f"这与 GLD 的 C-$ amplitude（absorption=0.928）形成对比。"
        )
    else:
        scenario = "C-third"
        conclusion = (
            f"C_oil(BRENT)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"第三种模式——石油-美元关系不属于已知的两种分类。"
            f"可能暗示石油与美元的耦合结构需要新的描述框架。"
        )

    verdict = {
        "scenario": scenario,
        "conclusion": conclusion,
        "ec_oil_note": (
            f"E-C_oil(SPY-BRENT): {ec_cls['coupling_type']}, "
            f"partial_corr={ec_oil['continuous']['partial_corr']:.4f}"
        ),
        "brent_data_note": brent_data_note,
    }

    result["verdict"] = verdict
    result["experiment_design"] = {
        "purpose": "测试石油（石油美元锚点）是否对缠论 D 算子可见——结算通道可观测性",
        "replacement": "GLD (黄金) -> BRENT (布伦特原油)",
        "controls": "SPY(E), TLT(R), UUP($) 不变",
        "scenarios": {
            "A": "全吸收（与 DBA 相同）-> 纯商品，D 算子看不见",
            "B": "部分保留/放大 -> 石油美元具有结算通道特征",
            "C": "第三种模式",
        },
    }

    print(f"\n--- Step 2 判定 ---")
    print(f"  情景: {verdict['scenario']}")
    print(f"  结论: {verdict['conclusion']}")
    print(f"  E-C_oil 附注: {verdict['ec_oil_note']}")

    return result


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------


def main() -> None:
    print("=" * 70)
    print("v109 结算通道可观测性实验 — Alpha Vantage 数据源")
    print("=" * 70)
    print(f"234号基准：SPY(E)/GLD(C)/TLT(R)/UUP($), 4780 bars, 2007-03-01~2026-02-27")
    print(f"数据源：Alpha Vantage (免费版 5 calls/min)")
    print()

    api_key = _get_api_key()

    # ---- Step 1 ----
    print("\n" + "#" * 70)
    print("# STEP 1: XAU/USD 替代 GLD — ETF 微观结构污染检测")
    print("#" * 70)
    step1_result = run_step1(api_key)

    # Step 1 → Step 2 之间等待，避免限流
    print(f"\n[INTER-STEP] 等待 {_API_CALL_INTERVAL_SEC}s 避免 API 限流...")
    time.sleep(_API_CALL_INTERVAL_SEC)

    # ---- Step 2 ----
    print("\n" + "#" * 70)
    print("# STEP 2: BRENT 替代 GLD — 石油美元结算通道可观测性")
    print("#" * 70)
    step2_result = run_step2(api_key)

    # ---- 综合对比 ----
    print("\n" + "=" * 70)
    print("综合对比表")
    print("=" * 70)

    print("\n  C 列替换实验结果汇总：")
    print(f"  {'实验':<20} {'C 标的':<15} {'E-C':<15} {'C-$':<15} {'C-R':<15}")
    print(f"  {'-'*20} {'-'*15} {'-'*15} {'-'*15} {'-'*15}")

    # 234号基准
    print(f"  {'234号(基准)':<20} {'GLD(ETF)':<15} "
          f"{'independent':<15} {'amplitude':<15} {'direction':<15}")

    # Step 1
    s1_ec = step1_result["edges"]["E-C_spot"]["classification"]["coupling_type"]
    s1_cs = step1_result["edges"]["C_spot-$"]["classification"]["coupling_type"]
    s1_cr = step1_result["edges"]["C_spot-R"]["classification"]["coupling_type"]
    print(f"  {'Step1(XAU/USD)':<20} {'XAU/USD(现货)':<15} "
          f"{s1_ec:<15} {s1_cs:<15} {s1_cr:<15}")

    # Step 2
    s2_ec = step2_result["edges"]["E-C_oil"]["classification"]["coupling_type"]
    s2_cs = step2_result["edges"]["C_oil-$"]["classification"]["coupling_type"]
    s2_cr = step2_result["edges"]["C_oil-R"]["classification"]["coupling_type"]
    print(f"  {'Step2(BRENT)':<20} {'BRENT(原油)':<15} "
          f"{s2_ec:<15} {s2_cs:<15} {s2_cr:<15}")

    # 259号（如果有数据）
    # 注：259号使用 DBA（纯农产品），此处仅作占位提醒

    # ---- 写出结果 ----
    out_dir = _PROJECT_ROOT / "tmp"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Step 1 结果
    step1_output = {
        "genealogy": "v109-step1",
        "title": "XAU/USD 替代 GLD — ETF 微观结构污染检测",
        **step1_result,
        "methodology": {
            "continuous": "日对数收益率 Pearson + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
            "data_source": "Alpha Vantage (FX_DAILY for XAU/USD, TIME_SERIES_DAILY for equities)",
        },
        "timestamp": datetime.now().isoformat(),
    }

    step1_path = out_dir / "v109-step1-xauusd-baseline.json"
    with open(step1_path, "w", encoding="utf-8") as f:
        json.dump(step1_output, f, indent=2, ensure_ascii=False)
    print(f"\n[Step 1 结果已写入] {step1_path}")

    # Step 2 结果
    step2_output = {
        "genealogy": "v109-step2",
        "title": "BRENT 替代 GLD — 石油美元结算通道可观测性",
        **step2_result,
        "methodology": {
            "continuous": "日对数收益率 Pearson + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
            "data_source": "Alpha Vantage (Commodities/BRENT for oil, TIME_SERIES_DAILY for equities)",
            "brent_data_note": step2_result["verdict"]["brent_data_note"],
        },
        "timestamp": datetime.now().isoformat(),
    }

    step2_path = out_dir / "v109-step2-brent-oil.json"
    with open(step2_path, "w", encoding="utf-8") as f:
        json.dump(step2_output, f, indent=2, ensure_ascii=False)
    print(f"[Step 2 结果已写入] {step2_path}")

    print("\n" + "=" * 70)
    print("v109 实验完成。")
    print("=" * 70)


if __name__ == "__main__":
    main()
