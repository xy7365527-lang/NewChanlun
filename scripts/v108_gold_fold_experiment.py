"""v108 黄金折叠剥离实验 — 259号谱系。

验证 234号 C-$ 吸收结论是否为黄金自指伪影。
直接决定 232号底空间 B=E×C 直积结构的存废。

实验设计：
  E=SPY, R=TLT, $=UUP 不变
  C 替换为 DBA（Invesco DB Agriculture Fund，纯农产品 ETF）
    - 不使用 DBC/GSG（含贵金属）
    - 不使用 CPER（铜的宏观周期耦合会引入 E-C 噪声）
  拉取与 234号实验同期数据（2007-03-01 ~ 2026-02-27）

决胜判据：
  情景 A：C_pure-$ 仍在 ker(D)（高吸收率）→ 234号结论成立，黄金折叠是局部奇点
  情景 B：C_pure-$ 移出 ker(D)（保留或放大）→ 234号 C 列结论为黄金伪影
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np

_PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

from newchan.bi_engine import BiEngine
from newchan.topology.discretization_kernel import (
    CouplingClassification,
    classify_coupling,
)
from newchan.types import Bar


# ---------------------------------------------------------------------------
# 常量
# ---------------------------------------------------------------------------

# 234号原始标的：SPY(E), GLD(C), TLT(R), UUP($)
# 259号替换：C = DBA（纯农产品）
_SYMBOLS = ("SPY", "DBA", "TLT", "UUP")
_SYMBOL_LABELS = {"SPY": "E", "DBA": "C_pure", "TLT": "R", "UUP": "$"}

# 234号数据期间
_START_DATE = "2007-01-05"  # DBA inception: 2007-01-05
_END_DATE = "2026-02-27"

# 六条边定义（与 234号结构相同，C 换为 C_pure）
_ALL_EDGES = {
    "E-R": ("SPY", "TLT"),
    "E-C_pure": ("SPY", "DBA"),
    "E-$": ("SPY", "UUP"),
    "C_pure-R": ("DBA", "TLT"),
    "C_pure-$": ("DBA", "UUP"),
    "R-$": ("TLT", "UUP"),
}

# 234号原始结果（对比基准）
_BASELINE_234 = {
    "E-R": {"partial_corr": -0.317, "beta": -0.020, "type": "amplitude", "in_kernel": True, "absorption": 0.949},
    "E-$": {"partial_corr": -0.183, "beta": -0.006, "type": "amplitude", "in_kernel": True, "absorption": 0.967},
    "C-$": {"partial_corr": -0.414, "beta": -0.030, "type": "amplitude", "in_kernel": True, "absorption": 0.928},
    "R-$": {"partial_corr": -0.065, "beta": 0.050, "type": "amplitude", "in_kernel": True, "absorption": 0.233},
    "C-R": {"partial_corr": 0.166, "beta": 0.009, "type": "direction", "in_kernel": False},
    "E-C": {"partial_corr": -0.029, "beta": 0.023, "type": "independent", "in_kernel": False},
}


# ---------------------------------------------------------------------------
# 数据获取（yfinance）
# ---------------------------------------------------------------------------


def fetch_data_yfinance() -> dict[str, list[Bar]]:
    """用 yfinance 获取四个标的的日线数据。"""
    import yfinance as yf

    result: dict[str, list[Bar]] = {}

    for sym in _SYMBOLS:
        print(f"[FETCH] {sym} via yfinance...")
        ticker = yf.Ticker(sym)
        df = ticker.history(start=_START_DATE, end=_END_DATE, auto_adjust=True)

        bars: list[Bar] = []
        for idx, row in df.iterrows():
            bars.append(
                Bar(
                    ts=idx.to_pydatetime().replace(tzinfo=None),
                    open=float(row["Open"]),
                    high=float(row["High"]),
                    low=float(row["Low"]),
                    close=float(row["Close"]),
                    volume=float(row["Volume"]) if "Volume" in row else None,
                )
            )
        print(f"  -> {len(bars)} bars ({bars[0].ts.date()} ~ {bars[-1].ts.date()})")
        result[sym] = bars

    return result


# ---------------------------------------------------------------------------
# 数据对齐
# ---------------------------------------------------------------------------


def align_bars(
    all_bars: dict[str, list[Bar]],
) -> tuple[list[str], dict[str, np.ndarray], dict[str, list[Bar]]]:
    """按日期对齐四个标的。

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
    for sym, bars in all_bars.items():
        date_maps[sym] = {b.ts.strftime("%Y-%m-%d"): b for b in bars}

    common_dates_set: set[str] | None = None
    for sym in _SYMBOLS:
        dates = set(date_maps[sym].keys())
        common_dates_set = dates if common_dates_set is None else common_dates_set & dates

    common_dates = sorted(common_dates_set)
    print(f"[ALIGN] {len(common_dates)} common trading days")
    print(f"  range: {common_dates[0]} ~ {common_dates[-1]}")

    closes: dict[str, np.ndarray] = {}
    aligned: dict[str, list[Bar]] = {}
    for sym in _SYMBOLS:
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

    # 构建控制变量矩阵
    X = np.column_stack([returns[c] for c in controls])
    X_with_intercept = np.column_stack([np.ones(len(r_a)), X])

    # 回归 a ~ controls
    coeffs_a, _, _, _ = np.linalg.lstsq(X_with_intercept, r_a, rcond=None)
    resid_a = r_a - X_with_intercept @ coeffs_a

    # 回归 b ~ controls
    coeffs_b, _, _, _ = np.linalg.lstsq(X_with_intercept, r_b, rcond=None)
    resid_b = r_b - X_with_intercept @ coeffs_b

    return pearson(resid_a, resid_b)


def compute_edge_continuous(
    returns: dict[str, np.ndarray],
) -> dict[str, dict[str, float]]:
    """计算六条边的连续层 Pearson 相关 + 偏相关。"""
    result: dict[str, dict[str, float]] = {}

    for edge_name, (sym_a, sym_b) in _ALL_EDGES.items():
        r = pearson(returns[sym_a], returns[sym_b])

        # 偏相关：控制其余两个标的
        other_syms = [s for s in _SYMBOLS if s != sym_a and s != sym_b]
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
    """将笔序列映射为逐 bar 的方向数组。

    up -> +1, down -> -1, 无笔覆盖 -> 0
    """
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
    """计算两个离散方向序列之间的相关性。

    beta: 方向序列线性回归斜率（classify_coupling 的输入）。
    """
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
) -> dict[str, dict[str, Any]]:
    """对六条边进行 ker(D) 分类。"""
    results: dict[str, dict[str, Any]] = {}

    for edge_name, (sym_a, sym_b) in _ALL_EDGES.items():
        cont = continuous[edge_name]
        disc = discrete.get(edge_name, {})

        # 使用偏相关作为 classify_coupling 的输入
        partial_corr_val = cont["partial_corr"]
        beta_val = disc.get("beta", 0.0)

        classification = classify_coupling(
            partial_corr=partial_corr_val,
            beta=beta_val,
        )

        label_a = _SYMBOL_LABELS.get(sym_a, sym_a)
        label_b = _SYMBOL_LABELS.get(sym_b, sym_b)

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
# 决胜判定
# ---------------------------------------------------------------------------


def determine_verdict(
    classifications: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    """根据 C_pure-$ 的分类结果判定情景。

    情景 A：C_pure-$ 仍在 ker(D) → 234号 C-$ 吸收结论成立
    情景 B：C_pure-$ 移出 ker(D) → 234号 C-$ 结论为黄金伪影
    """
    cpure_dollar = classifications["C_pure-$"]
    cls = cpure_dollar["classification"]

    in_kernel = cls["in_kernel"]
    coupling_type = cls["coupling_type"]
    absorption = cls["absorption_ratio"]

    # 234号 C-$ 基准
    baseline_absorption = _BASELINE_234["C-$"]["absorption"]

    if in_kernel:
        scenario = "A"
        conclusion = (
            f"C_pure(DBA)-$(UUP) 在 ker(D) 中，吸收率={absorption:.3f}。"
            f"234号 C-$(GLD-UUP) 吸收率={baseline_absorption:.3f}。"
            f"黄金折叠是局部奇点（C-$ 吸收是 C 类资产的一般性质，不依赖黄金的 $ 计价自指）。"
            f"232号底空间 B=E×C 直积结构保持。"
        )
    elif coupling_type == "independent":
        scenario = "B-independent"
        conclusion = (
            f"C_pure(DBA)-$(UUP) 分类为 independent（偏相关过弱）。"
            f"这与 234号 C-$(GLD-UUP) 的强幅度耦合（partial_corr=-0.414）形成反差。"
            f"结论：234号 C-$ 的强耦合部分可能是黄金-美元的自指效应。"
            f"但 C_pure-$ 独立不等于 234号 C-$ 是完全伪影——"
            f"只是说明 GLD 的 C-$ 耦合中有黄金-美元反向运动的特殊贡献。"
        )
    else:
        scenario = "B"
        conclusion = (
            f"C_pure(DBA)-$(UUP) 不在 ker(D)（{coupling_type}，吸收率={absorption:.3f}）。"
            f"234号 C-$(GLD-UUP) 吸收结论为黄金伪影。"
            f"底空间 B=E×C 直积结构需要重构。"
        )

    return {
        "scenario": scenario,
        "conclusion": conclusion,
        "cpure_dollar_classification": cls,
        "baseline_234_c_dollar": _BASELINE_234["C-$"],
    }


# ---------------------------------------------------------------------------
# 对比分析
# ---------------------------------------------------------------------------


def compare_with_234(
    classifications: dict[str, dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    """将 259号结果与 234号基准逐边对比。

    三条不变边（E-R, E-$, R-$）应该一致——验证实验控制变量。
    三条替换边（E-C_pure, C_pure-R, C_pure-$）是实验变量。
    """
    comparisons: dict[str, dict[str, Any]] = {}

    # 不变边映射
    invariant_map = {"E-R": "E-R", "E-$": "E-$", "R-$": "R-$"}
    # 替换边映射
    variant_map = {"E-C_pure": "E-C", "C_pure-R": "C-R", "C_pure-$": "C-$"}

    for edge_259, edge_234 in {**invariant_map, **variant_map}.items():
        new = classifications[edge_259]
        old = _BASELINE_234[edge_234]

        comparisons[edge_259] = {
            "edge_234": edge_234,
            "is_control": edge_259 in invariant_map,
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
# 主流程
# ---------------------------------------------------------------------------


def main() -> None:
    print("=" * 70)
    print("v108 黄金折叠剥离实验 — 259号谱系")
    print("C = DBA（纯农产品），替换 GLD（黄金）")
    print("=" * 70)

    # 1. 获取数据
    print("\n--- Step 1: 获取数据 (yfinance) ---")
    all_bars = fetch_data_yfinance()

    # 2. 对齐
    print("\n--- Step 2: 对齐数据 ---")
    common_dates, closes, aligned = align_bars(all_bars)
    n_bars = len(common_dates)

    # 3. 连续层计算
    print("\n--- Step 3: 连续层 Pearson 相关 + 偏相关 ---")
    returns = compute_returns(closes)
    continuous = compute_edge_continuous(returns)
    for edge_name, data in continuous.items():
        print(f"  {edge_name}: pearson={data['pearson']:.4f}, partial_corr={data['partial_corr']:.4f}")

    # 4. 离散层计算
    print("\n--- Step 4: BiEngine 离散化 ---")
    bi_directions: dict[str, np.ndarray] = {}
    for sym in _SYMBOLS:
        bars = aligned[sym]
        strokes = run_bi_engine(bars)
        n_strokes = len(strokes)
        direction_arr = strokes_to_bar_direction(strokes, n_bars)
        n_nonzero = int(np.count_nonzero(direction_arr))
        print(f"  {sym} ({_SYMBOL_LABELS[sym]}): {n_strokes} strokes, "
              f"{n_nonzero}/{n_bars} bars with direction")
        bi_directions[sym] = direction_arr

    # 5. 离散层边相关
    print("\n--- Step 5: 离散方向相关 ---")
    discrete: dict[str, dict[str, float]] = {}
    for edge_name, (sym_a, sym_b) in _ALL_EDGES.items():
        disc = compute_discrete_correlation(bi_directions[sym_a], bi_directions[sym_b])
        discrete[edge_name] = disc
        print(f"  {edge_name}: beta={disc['beta']:.4f}, "
              f"concordance={disc.get('concordance', 0):.4f}, "
              f"n_active={disc['n_active']}")

    # 6. 分类
    print("\n--- Step 6: 六条边分类 ---")
    classifications = classify_all_edges(continuous, discrete)
    for edge_name, data in classifications.items():
        cls = data["classification"]
        marker = "∈ ker(D)" if cls["in_kernel"] else "∉ ker(D)"
        if cls["coupling_type"] == "independent":
            marker = "独立"
        print(f"  {edge_name} [{data['labels'][0]} vs {data['labels'][1]}]: "
              f"{cls['coupling_type']} | {marker} | "
              f"absorption={cls['absorption_ratio']:.3f}")

    # 7. 与 234号对比
    print("\n--- Step 7: 与 234号基准对比 ---")
    comparisons = compare_with_234(classifications)

    print("\n  === 控制变量（不变边） ===")
    for edge_name, comp in comparisons.items():
        if comp["is_control"]:
            match_str = "✓" if comp["type_match"] else "✗ CHANGED"
            print(f"  {edge_name} (=234 {comp['edge_234']}): "
                  f"type {comp['old_type']}→{comp['new_type']} {match_str} | "
                  f"pc {comp['old_partial_corr']:.3f}→{comp['new_partial_corr']:.3f} | "
                  f"β {comp['old_beta']:.3f}→{comp['new_beta']:.3f}")

    print("\n  === 实验变量（替换边） ===")
    for edge_name, comp in comparisons.items():
        if not comp["is_control"]:
            print(f"  {edge_name} (←234 {comp['edge_234']}): "
                  f"type {comp['old_type']}→{comp['new_type']} | "
                  f"pc {comp['old_partial_corr']:.3f}→{comp['new_partial_corr']:.3f} | "
                  f"β {comp['old_beta']:.3f}→{comp['new_beta']:.3f} | "
                  f"abs {comp['old_absorption']}→{comp['new_absorption']:.3f}")

    # 8. 决胜判定
    print("\n--- Step 8: 决胜判定 ---")
    verdict = determine_verdict(classifications)
    print(f"\n  情景: {verdict['scenario']}")
    print(f"  结论: {verdict['conclusion']}")

    # 9. 汇总输出
    print("\n" + "=" * 70)
    n_in_kernel = sum(1 for e in classifications.values() if e["classification"]["in_kernel"])
    n_direction = sum(1 for e in classifications.values()
                      if e["classification"]["coupling_type"] == "direction")
    n_independent = sum(1 for e in classifications.values()
                        if e["classification"]["coupling_type"] == "independent")
    print(f"六条边分类汇总:")
    print(f"  ker(D) (amplitude): {n_in_kernel}")
    print(f"  not ker(D) (direction): {n_direction}")
    print(f"  independent: {n_independent}")

    # 10. 写结果
    output = {
        "genealogy": "259",
        "title": "黄金折叠剥离实验——C 替换为 DBA（纯农产品）",
        "experiment_design": {
            "purpose": "验证 234号 C-$ 吸收结论是否为黄金自指伪影",
            "replacement": "GLD (黄金) → DBA (纯农产品)",
            "controls": "SPY(E), TLT(R), UUP($) 不变",
            "decision_criteria": {
                "scenario_A": "C_pure-$ 仍在 ker(D) → 234号结论成立",
                "scenario_B": "C_pure-$ 移出 ker(D) → 234号 C 列结论为黄金伪影",
            },
        },
        "data_info": {
            "n_common_bars": n_bars,
            "date_range": f"{common_dates[0]} ~ {common_dates[-1]}",
            "symbols": {sym: _SYMBOL_LABELS[sym] for sym in _SYMBOLS},
        },
        "edges": classifications,
        "comparison_with_234": comparisons,
        "verdict": verdict,
        "summary": {
            "n_in_kernel": n_in_kernel,
            "n_direction": n_direction,
            "n_independent": n_independent,
        },
        "methodology": {
            "continuous": "日对数收益率 Pearson 相关 + 偏相关（控制其余两标的）",
            "discrete": "BiEngine（new笔模式）-> 逐bar方向 -> 方向序列线性回归 beta",
            "classification": "classify_coupling(partial_corr, beta) from discretization_kernel.py",
        },
        "timestamp": datetime.now().isoformat(),
    }

    out_path = _PROJECT_ROOT / "tmp" / "v108-gold-fold-experiment.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f"\n[结果已写入] {out_path}")


if __name__ == "__main__":
    main()
