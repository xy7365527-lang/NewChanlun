"""v109 结算通道可观测性实验 — yfinance 数据源。

三步实验设计：

Step 1：GC=F（COMEX 黄金期货）替代 GLD 重跑 234号基线
  - 目的：测试 GLD 的 ETF 微观结构（交易时间、折溢价、创设赎回机制）
    是否污染了 234号 E-C 独立性结论
  - E=SPY, C=GC=F（COMEX 黄金期货）, R=TLT, $=UUP
  - GLD 是 ETF，与 SPY 共享股票市场微观结构；GC=F 是期货，无股票市场中介
  - 判据：
    - GC=F 版 E-C 仍 independent → GLD ETF 微观结构未污染
    - GC=F 版 E-C 变为 amplitude/direction → 234号 E-C 独立可能是 ETF 微观结构伪影

Step 2：BZ=F（NYMEX 布伦特原油期货）替代 GLD
  - 目的：测试石油（石油美元锚点）是否对缠论 D 算子可见——即结算通道的可观测性
  - E=SPY, C=BZ=F（NYMEX 布伦特原油期货）, R=TLT, $=UUP
  - 三个情景：
    - A：全吸收（与 DBA 相同）→ 纯商品，D 算子看不见
    - B：部分保留/放大（类似 GLD）→ 石油美元具有结算通道特征
    - C：第三种模式

Step 3：控制变量一致性检验
  - 目的：三组实验（GC=F / DBA / BZ=F）统一使用 yfinance 数据源后，
    对比 E-R、E-$、R-$ 三条控制变量边的稳定性
  - GC=F 基线（Step 1 已跑），DBA 和 BZ=F 作为对照组
  - 判据：
    - 三条控制边在三组间稳定 → 259号 R-$ 异常归因于数据对齐
    - 三条控制边仍不稳定 → 六条边间接耦合，需要诊断

数据源：yfinance
  - GC=F：COMEX 黄金期货
  - BZ=F：NYMEX 布伦特原油期货
  - SPY/TLT/UUP/DBA：股票/ETF 日线
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np
import yfinance as yf

_PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

from newchan.bi_engine import BiEngine
from newchan.topology.discretization_kernel import (
    CouplingClassification,
    classify_coupling,
)
from newchan.types import Bar


# ---------------------------------------------------------------------------
# yfinance 数据获取
# ---------------------------------------------------------------------------


def fetch_yf(symbol: str, start: str = "2002-01-01") -> list[Bar]:
    """通过 yfinance 获取日线数据。"""
    print(f"[FETCH] {symbol} via yfinance...")
    ticker = yf.Ticker(symbol)
    df = ticker.history(start=start, auto_adjust=True)
    bars: list[Bar] = []
    for idx, row in df.iterrows():
        bars.append(Bar(
            ts=idx.to_pydatetime().replace(tzinfo=None),
            open=float(row["Open"]),
            high=float(row["High"]),
            low=float(row["Low"]),
            close=float(row["Close"]),
            volume=float(row["Volume"]) if "Volume" in row and row["Volume"] else None,
        ))
    if bars:
        print(f"  -> {len(bars)} bars ({bars[0].ts.date()} ~ {bars[-1].ts.date()})")
    else:
        raise RuntimeError(f"No data for {symbol}")
    return bars


# ---------------------------------------------------------------------------
# 234号基准数据（对比用）
# ---------------------------------------------------------------------------

_BASELINE_234 = {
    "E-R": {"partial_corr": -0.317, "beta": -0.020, "type": "amplitude", "in_kernel": True, "absorption": 0.949},
    "E-$": {"partial_corr": -0.183, "beta": -0.006, "type": "amplitude", "in_kernel": True, "absorption": 0.967},
    "C-$": {"partial_corr": -0.414, "beta": -0.030, "type": "amplitude", "in_kernel": True, "absorption": 0.928},
    "R-$": {"partial_corr": -0.065, "beta": 0.050, "type": "amplitude", "in_kernel": True, "absorption": 0.233},
    "C-R": {"partial_corr": 0.166, "beta": 0.009, "type": "amplitude", "in_kernel": True, "absorption": 0.946},
    "E-C": {"partial_corr": -0.029, "beta": 0.023, "type": "independent", "in_kernel": False},
}

# 259号 DBA 实验结果（如有）
_BASELINE_259: dict[str, Any] = {}  # 由 v108 实验填充，此处仅作对比占位

# 234号时间窗口
_234_START = "2007-03-01"
_234_END = "2026-02-26"


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
# Step 1：GC=F（COMEX 黄金期货）替代 GLD
# ---------------------------------------------------------------------------


def run_step1() -> dict[str, Any]:
    """Step 1：GC=F 替代 GLD 重跑 234号基线。

    E=SPY, C=GC=F（COMEX 黄金期货）, R=TLT, $=UUP
    测试 GLD 的 ETF 微观结构是否污染了 234号 E-C 独立性结论。
    """
    symbols = ("SPY", "GC=F", "TLT", "UUP")
    symbol_labels = {"SPY": "E", "GC=F": "C_spot", "TLT": "R", "UUP": "$"}

    edges = {
        "E-R": ("SPY", "TLT"),
        "E-C_spot": ("SPY", "GC=F"),
        "E-$": ("SPY", "UUP"),
        "C_spot-R": ("GC=F", "TLT"),
        "C_spot-$": ("GC=F", "UUP"),
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

    all_bars["SPY"] = fetch_yf("SPY")
    all_bars["TLT"] = fetch_yf("TLT")
    all_bars["UUP"] = fetch_yf("UUP")
    all_bars["GC=F"] = fetch_yf("GC=F")

    # 运行管线
    result = run_pipeline(
        all_bars=all_bars,
        symbols=symbols,
        symbol_labels=symbol_labels,
        edges=edges,
        edge_mapping_234=edge_mapping_234,
        control_edges=control_edges,
        step_name="Step 1: GC=F（COMEX 黄金期货）替代 GLD — ETF 微观结构污染检测",
    )

    # Step 1 特有判据
    ec_spot = result["edges"]["E-C_spot"]
    ec_cls = ec_spot["classification"]
    ec_type = ec_cls["coupling_type"]

    if ec_type == "independent":
        verdict = {
            "scenario": "A-clean",
            "conclusion": (
                f"GC=F 版 E-C 仍为 independent "
                f"(partial_corr={ec_spot['continuous']['partial_corr']:.4f})。"
                f"GLD ETF 微观结构未污染 234号 E-C 独立性结论。"
                f"E-C 独立是黄金资产的内在性质，不依赖 ETF 中介。"
            ),
        }
    else:
        verdict = {
            "scenario": "B-contaminated",
            "conclusion": (
                f"GC=F 版 E-C 分类为 {ec_type} "
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
        f"C_spot(GC=F)-$: {cs_cls['coupling_type']}, "
        f"absorption={cs_cls['absorption_ratio']:.3f} "
        f"(234号 C-$: amplitude, absorption=0.928)"
    )
    verdict["c_dollar_note"] = cs_note

    result["verdict"] = verdict
    result["experiment_design"] = {
        "purpose": "测试 GLD ETF 微观结构是否污染 234号 E-C 独立性结论",
        "replacement": "GLD (ETF) -> GC=F (COMEX 黄金期货)",
        "controls": "SPY(E), TLT(R), UUP($) 不变",
        "hypothesis": "如果 E-C 独立是黄金内在性质而非 ETF 伪影，GC=F 版应保持 independent",
    }

    print(f"\n--- Step 1 判定 ---")
    print(f"  情景: {verdict['scenario']}")
    print(f"  结论: {verdict['conclusion']}")
    print(f"  C-$ 附注: {cs_note}")

    return result


# ---------------------------------------------------------------------------
# Step 2：BZ=F（NYMEX 布伦特原油期货）替代 GLD
# ---------------------------------------------------------------------------


def run_step2() -> dict[str, Any]:
    """Step 2：BZ=F 替代 GLD。

    E=SPY, C=BZ=F（NYMEX 布伦特原油期货）, R=TLT, $=UUP
    测试石油（石油美元锚点）是否对缠论 D 算子可见——结算通道的可观测性。
    """
    symbols = ("SPY", "BZ=F", "TLT", "UUP")
    symbol_labels = {"SPY": "E", "BZ=F": "C_oil", "TLT": "R", "UUP": "$"}

    edges = {
        "E-R": ("SPY", "TLT"),
        "E-C_oil": ("SPY", "BZ=F"),
        "E-$": ("SPY", "UUP"),
        "C_oil-R": ("BZ=F", "TLT"),
        "C_oil-$": ("BZ=F", "UUP"),
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

    all_bars["SPY"] = fetch_yf("SPY")
    all_bars["TLT"] = fetch_yf("TLT")
    all_bars["UUP"] = fetch_yf("UUP")
    all_bars["BZ=F"] = fetch_yf("BZ=F")

    # 运行管线
    result = run_pipeline(
        all_bars=all_bars,
        symbols=symbols,
        symbol_labels=symbol_labels,
        edges=edges,
        edge_mapping_234=edge_mapping_234,
        control_edges=control_edges,
        step_name="Step 2: BZ=F（NYMEX 布伦特原油期货）替代 GLD — 石油美元结算通道可观测性",
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
            f"C_oil(BZ=F)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"全吸收——与 DBA（259号）模式相同。"
            f"石油对 D 算子不可见，纯商品行为。"
            f"石油美元在缠论离散化层面无结算通道特征。"
        )
    elif co_type == "direction" or co_type == "amplified":
        scenario = "B-channel"
        conclusion = (
            f"C_oil(BZ=F)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"部分保留/放大——石油美元具有结算通道特征。"
            f"石油-美元关系在缠论 D 算子下不被完全吸收，"
            f"方向耦合穿透离散化滤波器。"
            f"这与 GLD 的 C-$ amplitude（absorption=0.928）形成对比。"
        )
    else:
        scenario = "C-third"
        conclusion = (
            f"C_oil(BZ=F)-$: {co_type}, absorption={co_absorption:.3f}。"
            f"第三种模式——石油-美元关系不属于已知的两种分类。"
            f"可能暗示石油与美元的耦合结构需要新的描述框架。"
        )

    verdict = {
        "scenario": scenario,
        "conclusion": conclusion,
        "ec_oil_note": (
            f"E-C_oil(SPY-BZ=F): {ec_cls['coupling_type']}, "
            f"partial_corr={ec_oil['continuous']['partial_corr']:.4f}"
        ),
    }

    result["verdict"] = verdict
    result["experiment_design"] = {
        "purpose": "测试石油（石油美元锚点）是否对缠论 D 算子可见——结算通道可观测性",
        "replacement": "GLD (黄金) -> BZ=F (NYMEX 布伦特原油期货)",
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
# Step 3：控制变量一致性检验
# ---------------------------------------------------------------------------


def run_step3_control_consistency(
    step1_result: dict[str, Any],
    step2_result: dict[str, Any],
) -> dict[str, Any]:
    """Step 3：控制变量一致性检验。

    三组实验（GC=F / DBA / BZ=F）统一使用 yfinance 数据源后，
    对比 E-R、E-$、R-$ 三条控制变量边的稳定性。

    GC=F 数据来自 Step 1 结果（不重复获取）。
    DBA 需要新获取（yfinance）。
    BZ=F 数据来自 Step 2 结果（不重复获取）。

    Parameters
    ----------
    step1_result : dict
        Step 1（GC=F）的完整结果。
    step2_result : dict
        Step 2（BZ=F）的完整结果。

    Returns
    -------
    dict
        控制变量一致性检验结果。
    """
    # ---- DBA 基线 ----
    symbols = ("SPY", "DBA", "TLT", "UUP")
    symbol_labels = {"SPY": "E", "DBA": "C_agri", "TLT": "R", "UUP": "$"}

    edges = {
        "E-R": ("SPY", "TLT"),
        "E-C_agri": ("SPY", "DBA"),
        "E-$": ("SPY", "UUP"),
        "C_agri-R": ("DBA", "TLT"),
        "C_agri-$": ("DBA", "UUP"),
        "R-$": ("TLT", "UUP"),
    }

    edge_mapping_234 = {
        "E-R": "E-R",
        "E-C_agri": "E-C",
        "E-$": "E-$",
        "C_agri-R": "C-R",
        "C_agri-$": "C-$",
        "R-$": "R-$",
    }

    control_edges = {"E-R", "E-$", "R-$"}

    # 获取数据
    print("\n" + "=" * 70)
    print("Step 3：获取 DBA 基线数据（yfinance）")
    print("=" * 70)

    all_bars: dict[str, list[Bar]] = {}

    all_bars["SPY"] = fetch_yf("SPY")
    all_bars["TLT"] = fetch_yf("TLT")
    all_bars["UUP"] = fetch_yf("UUP")
    all_bars["DBA"] = fetch_yf("DBA")

    # 运行管线
    dba_result = run_pipeline(
        all_bars=all_bars,
        symbols=symbols,
        symbol_labels=symbol_labels,
        edges=edges,
        edge_mapping_234=edge_mapping_234,
        control_edges=control_edges,
        step_name="Step 3 基线: DBA (yfinance) — 农产品商品 ETF",
    )

    # ---- 控制变量对比 ----
    # 三条控制边：E-R, E-$, R-$
    # 三组实验：GC=F（step1）, DBA（本步）, BZ=F（step2）
    control_edge_names = ["E-R", "E-$", "R-$"]

    # DBA：直接用本步骤结果
    dba_controls = {}
    for edge in control_edge_names:
        edge_data = dba_result["edges"][edge]
        dba_controls[edge] = {
            "partial_corr": edge_data["continuous"]["partial_corr"],
            "beta": edge_data["discrete"].get("beta", 0.0),
            "classification": edge_data["classification"]["coupling_type"],
            "absorption": edge_data["classification"]["absorption_ratio"],
        }

    # Step1 (GC=F)：控制边名相同
    step1_controls = {}
    for edge in control_edge_names:
        edge_data = step1_result["edges"][edge]
        step1_controls[edge] = {
            "partial_corr": edge_data["continuous"]["partial_corr"],
            "beta": edge_data["discrete"].get("beta", 0.0),
            "classification": edge_data["classification"]["coupling_type"],
            "absorption": edge_data["classification"]["absorption_ratio"],
        }

    # Step2 (BZ=F)：控制边名相同
    step2_controls = {}
    for edge in control_edge_names:
        edge_data = step2_result["edges"][edge]
        step2_controls[edge] = {
            "partial_corr": edge_data["continuous"]["partial_corr"],
            "beta": edge_data["discrete"].get("beta", 0.0),
            "classification": edge_data["classification"]["coupling_type"],
            "absorption": edge_data["classification"]["absorption_ratio"],
        }

    # 计算变异度
    print("\n" + "=" * 60)
    print("  控制变量一致性检验")
    print("=" * 60)

    consistency_table: dict[str, dict[str, Any]] = {}

    for edge in control_edge_names:
        dba = dba_controls[edge]
        s1 = step1_controls[edge]
        s2 = step2_controls[edge]

        pc_values = [s1["partial_corr"], dba["partial_corr"], s2["partial_corr"]]
        beta_values = [s1["beta"], dba["beta"], s2["beta"]]
        abs_values = [s1["absorption"], dba["absorption"], s2["absorption"]]
        types = [s1["classification"], dba["classification"], s2["classification"]]

        pc_std = float(np.std(pc_values, ddof=0))
        beta_std = float(np.std(beta_values, ddof=0))
        abs_std = float(np.std(abs_values, ddof=0))
        type_unanimous = len(set(types)) == 1

        consistency_table[edge] = {
            "234_baseline": {
                "partial_corr": _BASELINE_234[edge]["partial_corr"],
                "beta": _BASELINE_234[edge]["beta"],
                "type": _BASELINE_234[edge]["type"],
                "absorption": _BASELINE_234[edge].get("absorption", None),
            },
            "gc_f": s1,
            "dba": dba,
            "bz_f": s2,
            "variability": {
                "partial_corr_std": round(pc_std, 6),
                "beta_std": round(beta_std, 6),
                "absorption_std": round(abs_std, 6),
                "type_unanimous": type_unanimous,
                "types": types,
            },
        }

        print(f"\n  {edge}:")
        print(f"    234号(yfinance): pc={_BASELINE_234[edge]['partial_corr']:.4f}, "
              f"beta={_BASELINE_234[edge]['beta']:.4f}, type={_BASELINE_234[edge]['type']}")
        print(f"    GC=F:            pc={s1['partial_corr']:.4f}, "
              f"beta={s1['beta']:.4f}, type={s1['classification']}")
        print(f"    DBA:             pc={dba['partial_corr']:.4f}, "
              f"beta={dba['beta']:.4f}, type={dba['classification']}")
        print(f"    BZ=F:            pc={s2['partial_corr']:.4f}, "
              f"beta={s2['beta']:.4f}, type={s2['classification']}")
        print(f"    -> pc_std={pc_std:.4f}, beta_std={beta_std:.4f}, "
              f"abs_std={abs_std:.4f}, type_unanimous={type_unanimous}")

    # 判定
    all_unanimous = all(
        v["variability"]["type_unanimous"] for v in consistency_table.values()
    )
    max_pc_std = max(
        v["variability"]["partial_corr_std"] for v in consistency_table.values()
    )

    # 阈值：偏相关标准差 < 0.05 视为稳定
    _STABILITY_THRESHOLD = 0.05
    all_stable = max_pc_std < _STABILITY_THRESHOLD

    if all_unanimous and all_stable:
        verdict = {
            "stable": True,
            "conclusion": (
                f"三条控制边在三组实验中保持稳定（max pc_std={max_pc_std:.4f} < {_STABILITY_THRESHOLD}，"
                f"分类全一致）。259号 R-$ 异常可归因于数据对齐差异。"
                f"统一 yfinance 数据源后，控制变量一致性成立。"
            ),
        }
    else:
        unstable_edges = [
            edge for edge, v in consistency_table.items()
            if not v["variability"]["type_unanimous"]
            or v["variability"]["partial_corr_std"] >= _STABILITY_THRESHOLD
        ]
        verdict = {
            "stable": False,
            "unstable_edges": unstable_edges,
            "conclusion": (
                f"控制边不稳定（不稳定边：{unstable_edges}，max pc_std={max_pc_std:.4f}）。"
                f"六条边存在间接耦合——替换 C 列资产影响了控制变量边。"
                f"需要进一步诊断耦合结构。"
            ),
        }

    print(f"\n--- Step 3 判定 ---")
    print(f"  稳定: {verdict['stable']}")
    print(f"  结论: {verdict['conclusion']}")

    return {
        "dba_full_result": dba_result,
        "consistency_table": consistency_table,
        "verdict": verdict,
        "experiment_design": {
            "purpose": "控制变量一致性检验——统一 yfinance 数据源后，三条控制边是否稳定",
            "control_edges": control_edge_names,
            "groups": ["GC=F", "DBA", "BZ=F"],
            "stability_threshold": _STABILITY_THRESHOLD,
            "hypothesis_if_stable": "259号 R-$ 异常归因于数据对齐差异",
            "hypothesis_if_unstable": "六条边间接耦合，替换 C 列影响控制变量",
        },
    }


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------


def main() -> None:
    print("=" * 70)
    print("v109 结算通道可观测性实验 — yfinance 数据源")
    print("=" * 70)
    print(f"234号基准：SPY(E)/GLD(C)/TLT(R)/UUP($), 4780 bars, 2007-03-01~2026-02-27")
    print(f"数据源：yfinance")
    print()

    # ---- Step 1 ----
    print("\n" + "#" * 70)
    print("# STEP 1: GC=F（COMEX 黄金期货）替代 GLD — ETF 微观结构污染检测")
    print("#" * 70)
    step1_result = run_step1()

    # ---- Step 2 ----
    print("\n" + "#" * 70)
    print("# STEP 2: BZ=F（NYMEX 布伦特原油期货）替代 GLD — 石油美元结算通道可观测性")
    print("#" * 70)
    step2_result = run_step2()

    # ---- Step 3 ----
    print("\n" + "#" * 70)
    print("# STEP 3: 控制变量一致性检验（GC=F + DBA + BZ=F 三组对比）")
    print("#" * 70)
    step3_result = run_step3_control_consistency(step1_result, step2_result)

    # ---- 综合对比表（四列：234原始/GC=F/DBA/BZ=F，六条边全覆盖）----
    print("\n" + "=" * 70)
    print("综合对比表（四列 x 六条边）")
    print("=" * 70)

    # 收集各组六条边数据
    # 映射：统一边名 -> 各组实际边名
    unified_edges = ["E-R", "E-C", "E-$", "C-R", "C-$", "R-$"]

    # Step1 边名映射（C_spot -> C）
    step1_edge_map = {
        "E-R": "E-R", "E-C": "E-C_spot", "E-$": "E-$",
        "C-R": "C_spot-R", "C-$": "C_spot-$", "R-$": "R-$",
    }
    # Step2 边名映射（C_oil -> C）
    step2_edge_map = {
        "E-R": "E-R", "E-C": "E-C_oil", "E-$": "E-$",
        "C-R": "C_oil-R", "C-$": "C_oil-$", "R-$": "R-$",
    }
    # DBA 边名映射（C_agri -> C）
    dba_edge_map = {
        "E-R": "E-R", "E-C": "E-C_agri", "E-$": "E-$",
        "C-R": "C_agri-R", "C-$": "C_agri-$", "R-$": "R-$",
    }

    dba_edges = step3_result["dba_full_result"]["edges"]

    # 表头
    header = f"  {'边':<8} {'234号(yf)':<18} {'GC=F':<18} {'DBA':<18} {'BZ=F':<18}"
    sep = f"  {'-'*8} {'-'*18} {'-'*18} {'-'*18} {'-'*18}"
    print(header)
    print(sep)

    comprehensive_table: dict[str, dict[str, Any]] = {}

    for edge in unified_edges:
        # 234号基准
        b234 = _BASELINE_234.get(edge, {})
        b234_str = (
            f"{b234.get('type', '?'):<10} pc={b234.get('partial_corr', 0):.3f}"
            if b234 else "N/A"
        )

        # GC=F (Step1)
        s1_key = step1_edge_map[edge]
        s1_data = step1_result["edges"].get(s1_key, {})
        if s1_data:
            s1_type = s1_data["classification"]["coupling_type"]
            s1_pc = s1_data["continuous"]["partial_corr"]
            s1_str = f"{s1_type:<10} pc={s1_pc:.3f}"
        else:
            s1_type, s1_pc = "?", 0.0
            s1_str = "N/A"

        # DBA (Step3)
        dba_key = dba_edge_map[edge]
        dba_data = dba_edges.get(dba_key, {})
        if dba_data:
            dba_type = dba_data["classification"]["coupling_type"]
            dba_pc = dba_data["continuous"]["partial_corr"]
            dba_str = f"{dba_type:<10} pc={dba_pc:.3f}"
        else:
            dba_type, dba_pc = "?", 0.0
            dba_str = "N/A"

        # BZ=F (Step2)
        s2_key = step2_edge_map[edge]
        s2_data = step2_result["edges"].get(s2_key, {})
        if s2_data:
            s2_type = s2_data["classification"]["coupling_type"]
            s2_pc = s2_data["continuous"]["partial_corr"]
            s2_str = f"{s2_type:<10} pc={s2_pc:.3f}"
        else:
            s2_type, s2_pc = "?", 0.0
            s2_str = "N/A"

        is_control = edge in {"E-R", "E-$", "R-$"}
        marker = " [ctrl]" if is_control else ""
        print(f"  {edge:<8} {b234_str:<18} {s1_str:<18} {dba_str:<18} {s2_str:<18}{marker}")

        comprehensive_table[edge] = {
            "is_control": is_control,
            "234_yfinance": {
                "type": b234.get("type", None),
                "partial_corr": b234.get("partial_corr", None),
                "beta": b234.get("beta", None),
                "absorption": b234.get("absorption", None),
            },
            "gc_f": {
                "type": s1_type,
                "partial_corr": s1_pc,
                "beta": s1_data.get("discrete", {}).get("beta", 0.0) if s1_data else None,
                "absorption": s1_data["classification"]["absorption_ratio"] if s1_data else None,
            },
            "dba": {
                "type": dba_type,
                "partial_corr": dba_pc,
                "beta": dba_data.get("discrete", {}).get("beta", 0.0) if dba_data else None,
                "absorption": dba_data["classification"]["absorption_ratio"] if dba_data else None,
            },
            "bz_f": {
                "type": s2_type,
                "partial_corr": s2_pc,
                "beta": s2_data.get("discrete", {}).get("beta", 0.0) if s2_data else None,
                "absorption": s2_data["classification"]["absorption_ratio"] if s2_data else None,
            },
        }

    # ---- 写出结果 ----
    out_dir = _PROJECT_ROOT / "tmp"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Step 1 结果
    step1_output = {
        "genealogy": "v109-step1",
        "title": "GC=F（COMEX 黄金期货）替代 GLD — ETF 微观结构污染检测",
        **step1_result,
        "methodology": {
            "continuous": "日对数收益率 Pearson + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
            "data_source": "yfinance (GC=F for gold futures, equities/ETFs direct)",
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
        "title": "BZ=F（NYMEX 布伦特原油期货）替代 GLD — 石油美元结算通道可观测性",
        **step2_result,
        "methodology": {
            "continuous": "日对数收益率 Pearson + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
            "data_source": "yfinance (BZ=F for Brent crude futures, equities/ETFs direct)",
        },
        "timestamp": datetime.now().isoformat(),
    }

    step2_path = out_dir / "v109-step2-brent-oil.json"
    with open(step2_path, "w", encoding="utf-8") as f:
        json.dump(step2_output, f, indent=2, ensure_ascii=False)
    print(f"[Step 2 结果已写入] {step2_path}")

    # Step 3 结果
    step3_output = {
        "genealogy": "v109-step3",
        "title": "控制变量一致性检验 — 统一 yfinance 数据源",
        "consistency_table": step3_result["consistency_table"],
        "verdict": step3_result["verdict"],
        "experiment_design": step3_result["experiment_design"],
        "comprehensive_table": comprehensive_table,
        "methodology": {
            "continuous": "日对数收益率 Pearson + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
            "data_source": "yfinance (all symbols)",
            "stability_metric": "三组间偏相关标准差 + 分类一致性",
        },
        "timestamp": datetime.now().isoformat(),
    }

    step3_path = out_dir / "v109-step3-control-consistency.json"
    with open(step3_path, "w", encoding="utf-8") as f:
        json.dump(step3_output, f, indent=2, ensure_ascii=False)
    print(f"[Step 3 结果已写入] {step3_path}")

    print("\n" + "=" * 70)
    print("v109 实验完成（三步 + 综合对比表）。")
    print("=" * 70)


if __name__ == "__main__":
    main()
