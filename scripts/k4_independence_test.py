"""K4 独立边独立性数据绑定实验

用具体标的检验三条独立边 E/$, C/$, R/$ 是否真正独立。

数据绑定方案：
  E（股票）: SPY — S&P 500 ETF
  C（商品）: GLD — 黄金 ETF
  R（债券）: TLT — 20年+国债 ETF
  $（美元）: UUP — 美元指数 ETF

三条独立边：
  E/$ = SPY/UUP（股票相对美元的走势）
  C/$ = GLD/UUP（商品相对美元的走势）
  R/$ = TLT/UUP（债券相对美元的走势）

谱系依据：k4-independence-conditions.md; ontology-v2-push.md 六6.2
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np

# ---------------------------------------------------------------------------
# 项目路径设置
# ---------------------------------------------------------------------------
_PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

# ---------------------------------------------------------------------------
# 数据获取：优先 AlphaVantage，不可用时回退合成数据
# ---------------------------------------------------------------------------

_SYMBOLS = ("SPY", "GLD", "TLT", "UUP")
_EDGE_LABELS = ("E_dollar", "C_dollar", "R_dollar")  # E/$, C/$, R/$
_EDGE_PAIRS = (("E_dollar", "C_dollar"), ("E_dollar", "R_dollar"), ("C_dollar", "R_dollar"))
_ROLLING_WINDOW = 60  # 滚动相关窗口（交易日）


def _try_fetch_real_data() -> dict[str, np.ndarray] | None:
    """尝试从 AlphaVantage 获取四个标的的日线收盘价。

    返回 {symbol: close_array} 或 None（不可用时）。
    """
    try:
        from newchan.config import ALPHAVANTAGE_API_KEY
        if not ALPHAVANTAGE_API_KEY:
            print("[INFO] ALPHAVANTAGE_API_KEY 未设置，回退合成数据")
            return None

        from newchan.data_av import AlphaVantageProvider
        provider = AlphaVantageProvider()

        result: dict[str, np.ndarray] = {}
        dates_ref: list[str] | None = None

        for sym in _SYMBOLS:
            bars = provider.fetch_daily(sym, outputsize="full")
            if len(bars) < 200:
                print(f"[WARN] {sym} 数据不足 ({len(bars)} bars)，回退合成数据")
                return None

            # 按日期对齐：使用 YYYY-MM-DD 作为 key
            date_close = {b.ts.strftime("%Y-%m-%d"): b.close for b in bars}

            if dates_ref is None:
                dates_ref = sorted(date_close.keys())
            else:
                # 取交集保证对齐
                dates_ref = sorted(set(dates_ref) & set(date_close.keys()))

            result[sym] = date_close  # type: ignore[assignment]  # 暂存 dict，后面对齐

        if dates_ref is None or len(dates_ref) < 200:
            print(f"[WARN] 对齐后数据不足 ({len(dates_ref) if dates_ref else 0} bars)，回退合成数据")
            return None

        # 对齐后转 array
        aligned: dict[str, np.ndarray] = {}
        for sym in _SYMBOLS:
            dc = result[sym]  # type: ignore[index]
            aligned[sym] = np.array([dc[d] for d in dates_ref], dtype=np.float64)  # type: ignore[index]

        print(f"[INFO] AlphaVantage 真实数据：{len(dates_ref)} 个交易日（{dates_ref[0]} ~ {dates_ref[-1]}）")
        return aligned

    except Exception as e:
        print(f"[WARN] AlphaVantage 获取失败 ({e})，回退合成数据")
        return None


def _generate_synthetic_data(n_days: int = 500, seed: int = 42) -> dict[str, np.ndarray]:
    """生成四个资产的合成价格序列（几何布朗运动）。

    设计原则：
      - SPY/GLD/TLT 的绝对收益率有各自的特质性波动
      - UUP（美元）有独立的波动
      - 通过控制 $ 因子强度可调节耦合程度
    """
    rng = np.random.default_rng(seed)

    # 年化波动率参数（合理的 ETF 量级）
    vol_spy = 0.18   # SPY ~18%
    vol_gld = 0.15   # GLD ~15%
    vol_tlt = 0.14   # TLT ~14%
    vol_uup = 0.08   # UUP ~8%

    # 日波动率
    dt = 1 / 252
    daily_vol = {
        "SPY": vol_spy * np.sqrt(dt),
        "GLD": vol_gld * np.sqrt(dt),
        "TLT": vol_tlt * np.sqrt(dt),
        "UUP": vol_uup * np.sqrt(dt),
    }

    # 日漂移（简化为 0）
    result: dict[str, np.ndarray] = {}
    for sym in _SYMBOLS:
        log_returns = rng.normal(0, daily_vol[sym], n_days)
        prices = 100.0 * np.exp(np.cumsum(log_returns))
        result[sym] = prices

    print(f"[INFO] 合成数据：{n_days} 个交易日（种子 {seed}）")
    return result


def _load_data() -> tuple[dict[str, np.ndarray], str]:
    """加载数据。返回 (prices_dict, data_source_label)。"""
    real = _try_fetch_real_data()
    if real is not None:
        return real, "alphavantage"
    return _generate_synthetic_data(), "synthetic"


# ---------------------------------------------------------------------------
# 核心计算
# ---------------------------------------------------------------------------

def _compute_ratio_returns(prices: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
    """计算三条独立边的日收益率。

    E/$ = SPY/UUP, C/$ = GLD/UUP, R/$ = TLT/UUP
    收益率 = ln(ratio_t / ratio_{t-1})
    """
    uup = prices["UUP"]
    ratios = {
        "E_dollar": prices["SPY"] / uup,
        "C_dollar": prices["GLD"] / uup,
        "R_dollar": prices["TLT"] / uup,
    }
    returns = {}
    for label, ratio in ratios.items():
        log_ret = np.diff(np.log(ratio))
        returns[label] = log_ret
    return returns


def _pearson_correlation(x: np.ndarray, y: np.ndarray) -> float:
    """Pearson 相关系数。"""
    if len(x) < 2:
        return float("nan")
    return float(np.corrcoef(x, y)[0, 1])


def _partial_correlation_controlling_dollar(
    returns: dict[str, np.ndarray],
    prices: dict[str, np.ndarray],
    edge_a: str,
    edge_b: str,
) -> float:
    """计算控制 $ 因子后的偏相关系数。

    $ 因子代理：UUP 的日收益率。
    方法：对 UUP 收益率回归各边收益率，取残差的相关系数。
    """
    uup_ret = np.diff(np.log(prices["UUP"]))
    ra = returns[edge_a]
    rb = returns[edge_b]

    # 确保长度一致
    n = min(len(ra), len(rb), len(uup_ret))
    ra, rb, uup_ret = ra[:n], rb[:n], uup_ret[:n]

    if n < 3:
        return float("nan")

    # OLS 残差：r_edge = alpha + beta * r_uup + epsilon
    # 使用 numpy lstsq
    X = np.column_stack([np.ones(n), uup_ret])

    beta_a, _, _, _ = np.linalg.lstsq(X, ra, rcond=None)
    residual_a = ra - X @ beta_a

    beta_b, _, _, _ = np.linalg.lstsq(X, rb, rcond=None)
    residual_b = rb - X @ beta_b

    return _pearson_correlation(residual_a, residual_b)


def _rolling_correlation(
    x: np.ndarray,
    y: np.ndarray,
    window: int,
) -> np.ndarray:
    """滚动 Pearson 相关系数。"""
    n = len(x)
    if n < window:
        return np.array([])

    result = np.full(n - window + 1, np.nan)
    for i in range(n - window + 1):
        xi = x[i : i + window]
        yi = y[i : i + window]
        result[i] = _pearson_correlation(xi, yi)
    return result


def _walk_direction(returns: np.ndarray, ma_window: int = 20) -> np.ndarray:
    """将日收益率映射为 WalkDirection（简单均线判断）。

    规则：
      - 累计收益率的 ma_window 日均线斜率 > 阈值 → UP (+1)
      - 斜率 < -阈值 → DOWN (-1)
      - 其他 → FLAT (0)

    阈值 = 0.5 * 标准差（避免噪声方向判断）
    """
    cumret = np.cumsum(returns)
    n = len(cumret)
    if n < ma_window + 1:
        return np.zeros(n, dtype=int)

    # 简单移动平均
    ma = np.convolve(cumret, np.ones(ma_window) / ma_window, mode="valid")
    # 斜率 = ma[t] - ma[t-1]
    slope = np.diff(ma)
    threshold = 0.5 * np.std(slope) if len(slope) > 1 else 0.0

    direction = np.zeros(len(slope), dtype=int)
    direction[slope > threshold] = 1
    direction[slope < -threshold] = -1

    # 填充前部（无法计算的部分设为 0）
    pad_len = n - len(direction)
    return np.concatenate([np.zeros(pad_len, dtype=int), direction])


def _config_space_coverage(
    directions: dict[str, np.ndarray],
) -> dict[str, Any]:
    """计算配置空间覆盖率。

    27 种配置 = {-1, 0, +1}^3
    """
    e_dir = directions["E_dollar"]
    c_dir = directions["C_dollar"]
    r_dir = directions["R_dollar"]

    n = min(len(e_dir), len(c_dir), len(r_dir))
    e_dir, c_dir, r_dir = e_dir[:n], c_dir[:n], r_dir[:n]

    # 统计每种配置的出现次数
    config_counts: dict[str, int] = {}
    for i in range(n):
        key = f"({int(e_dir[i])},{int(c_dir[i])},{int(r_dir[i])})"
        config_counts[key] = config_counts.get(key, 0) + 1

    # 所有 27 种配置
    all_configs = []
    for e in (-1, 0, 1):
        for c in (-1, 0, 1):
            for r in (-1, 0, 1):
                all_configs.append(f"({e},{c},{r})")

    observed = sum(1 for cfg in all_configs if config_counts.get(cfg, 0) > 0)

    return {
        "total_configs_possible": 27,
        "configs_observed": observed,
        "coverage_pct": round(100.0 * observed / 27, 1),
        "config_distribution": {
            cfg: config_counts.get(cfg, 0) for cfg in all_configs
        },
        "total_samples": n,
    }


def _polarity_index_stats(directions: dict[str, np.ndarray]) -> dict[str, Any]:
    """极性指数 S = sigma_p + sigma_c + sigma_r 的分布统计。"""
    e_dir = directions["E_dollar"]
    c_dir = directions["C_dollar"]
    r_dir = directions["R_dollar"]

    n = min(len(e_dir), len(c_dir), len(r_dir))
    S = e_dir[:n] + c_dir[:n] + r_dir[:n]

    # S 的分布
    unique, counts = np.unique(S, return_counts=True)
    dist = {int(v): int(c) for v, c in zip(unique, counts)}

    return {
        "mean": round(float(np.mean(S)), 4),
        "std": round(float(np.std(S)), 4),
        "distribution": dist,
    }


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def run_independence_test() -> dict[str, Any]:
    """执行完整的独立性检验，返回结果字典。"""
    prices, data_source = _load_data()
    returns = _compute_ratio_returns(prices)

    n_bars = len(next(iter(returns.values())))

    # --- 1. Pearson 相关系数 ---
    correlations: dict[str, Any] = {}
    for edge_a, edge_b in _EDGE_PAIRS:
        pair_label = f"{edge_a}_vs_{edge_b}"
        pearson = _pearson_correlation(returns[edge_a], returns[edge_b])
        partial = _partial_correlation_controlling_dollar(returns, prices, edge_a, edge_b)
        correlations[pair_label] = {
            "pearson": round(pearson, 6),
            "partial_controlling_dollar": round(partial, 6),
        }

    # --- 2. 滚动相关 ---
    rolling_summary: dict[str, Any] = {}
    for edge_a, edge_b in _EDGE_PAIRS:
        pair_label = f"{edge_a}_vs_{edge_b}"
        rc = _rolling_correlation(returns[edge_a], returns[edge_b], _ROLLING_WINDOW)
        if len(rc) > 0:
            valid = rc[~np.isnan(rc)]
            rolling_summary[pair_label] = {
                "mean": round(float(np.mean(valid)), 6) if len(valid) > 0 else None,
                "std": round(float(np.std(valid)), 6) if len(valid) > 0 else None,
                "max": round(float(np.max(valid)), 6) if len(valid) > 0 else None,
                "min": round(float(np.min(valid)), 6) if len(valid) > 0 else None,
                "window": _ROLLING_WINDOW,
                "n_windows": len(valid),
            }
        else:
            rolling_summary[pair_label] = {"error": "数据不足"}

    # --- 3. WalkDirection 映射 + 配置空间覆盖率 ---
    directions = {
        label: _walk_direction(returns[label]) for label in _EDGE_LABELS
    }
    coverage = _config_space_coverage(directions)
    polarity = _polarity_index_stats(directions)

    # --- 4. 独立性评估 ---
    assessment_lines = []
    theoretical_findings: list[dict[str, str]] = []

    # 评估 Pearson 相关
    for pair_label, corr_data in correlations.items():
        p = abs(corr_data["pearson"])
        pp = abs(corr_data["partial_controlling_dollar"])

        if p > 0.5:
            assessment_lines.append(
                f"{pair_label}: 强相关 (|pearson|={p:.3f})，独立性不成立"
            )
            if pp < 0.2:
                theoretical_findings.append({
                    "finding": f"{pair_label} 原始相关 {p:.3f} 但偏相关 {pp:.3f}",
                    "implication": "相关性主要由 $ 因子介导，控制 $ 后独立性恢复",
                    "category": "independence_violation",
                })
            else:
                theoretical_findings.append({
                    "finding": f"{pair_label} 原始相关 {p:.3f}，偏相关仍 {pp:.3f}",
                    "implication": "存在 $ 之外的耦合机制（risk-on/off 或结构性相关）",
                    "category": "independence_violation",
                })
        elif p > 0.2:
            assessment_lines.append(
                f"{pair_label}: 中等相关 (|pearson|={p:.3f})，独立性部分成立"
            )
        else:
            assessment_lines.append(
                f"{pair_label}: 弱相关 (|pearson|={p:.3f})，独立性成立"
            )

    # 评估配置空间覆盖
    cov_pct = coverage["coverage_pct"]
    if cov_pct < 70:
        theoretical_findings.append({
            "finding": f"配置空间覆盖率仅 {cov_pct}%（{coverage['configs_observed']}/27）",
            "implication": "直积结构退化，部分配置不可达",
            "category": "coverage_gap",
        })
    elif cov_pct < 90:
        assessment_lines.append(f"配置空间覆盖率 {cov_pct}%，部分配置稀缺")

    # 评估滚动相关的时变性
    for pair_label, rs in rolling_summary.items():
        if isinstance(rs, dict) and rs.get("std") is not None:
            if rs["std"] > 0.15:
                theoretical_findings.append({
                    "finding": f"{pair_label} 滚动相关标准差 {rs['std']:.3f}",
                    "implication": "独立性是时变的，存在独立性失效期和恢复期的交替",
                    "category": "time_varying",
                })

    assessment = "; ".join(assessment_lines) if assessment_lines else "未检测到显著相关"

    # --- 组装结果 ---
    result: dict[str, Any] = {
        "data_source": data_source,
        "data_period": f"{n_bars} trading days",
        "bar_count": n_bars,
        "binding": {
            "E": "SPY",
            "C": "GLD",
            "R": "TLT",
            "$": "UUP",
            "edges": {
                "E_dollar": "SPY/UUP",
                "C_dollar": "GLD/UUP",
                "R_dollar": "TLT/UUP",
            },
        },
        "correlations": correlations,
        "rolling_correlation_summary": rolling_summary,
        "config_space_coverage": coverage,
        "polarity_index_stats": polarity,
        "independence_assessment": assessment,
        "theoretical_findings": theoretical_findings,
        "methodology": {
            "pearson": "日收益率 Pearson 相关系数",
            "partial_correlation": "控制 UUP 收益率后的偏相关（OLS 残差法）",
            "rolling_window": f"{_ROLLING_WINDOW} 日滚动窗口",
            "walk_direction": "20 日均线斜率阈值法（0.5 * std）",
            "config_space": "三元组 (sigma_p, sigma_c, sigma_r) in {-1,0,+1}^3",
        },
        "timestamp": datetime.now().isoformat(),
    }

    return result


def main() -> None:
    result = run_independence_test()

    # 输出到 tmp/
    out_path = _PROJECT_ROOT / "tmp" / "k4-independence-results.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    print(f"\n[结果已写入] {out_path}")
    print(f"[数据来源] {result['data_source']}")
    print(f"[数据量] {result['bar_count']} 交易日")
    print(f"\n=== 相关性分析 ===")
    for pair, data in result["correlations"].items():
        print(f"  {pair}: pearson={data['pearson']:.4f}, partial={data['partial_controlling_dollar']:.4f}")

    print(f"\n=== 配置空间覆盖 ===")
    cov = result["config_space_coverage"]
    print(f"  观测到 {cov['configs_observed']}/27 种配置 ({cov['coverage_pct']}%)")

    print(f"\n=== 极性指数 ===")
    pol = result["polarity_index_stats"]
    print(f"  mean={pol['mean']}, std={pol['std']}")
    print(f"  分布: {pol['distribution']}")

    print(f"\n=== 独立性评估 ===")
    print(f"  {result['independence_assessment']}")

    if result["theoretical_findings"]:
        print(f"\n=== 理论发现 ({len(result['theoretical_findings'])}) ===")
        for i, finding in enumerate(result["theoretical_findings"], 1):
            print(f"  [{i}] {finding['category']}: {finding['finding']}")
            print(f"      含义: {finding['implication']}")


if __name__ == "__main__":
    main()
