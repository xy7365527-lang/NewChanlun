"""等价对管理 — 比价关系的形式化基础。

概念溯源：
  [旧缠论] 第9课 — "比价关系的变动，也可以构成一个买卖系统，
    这个买卖系统是和市场资金的流向相关的"
  [旧缠论] 第72/73课 — 比价关系为三个独立系统之一
  [新缠论] 等价关系严格定义 — ratio_relation_v1.md §2
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd


@dataclass(frozen=True, slots=True)
class EquivalencePair:
    """等价对：两个可构成比价关系的标的。

    等价对条件（ratio_relation_v1.md §2.1）：
      1. 可比性 — 有重叠交易时间窗口
      2. 非退化 — A/B 比价不是常数
      3. 流动性 — 双方有充足市场深度
    """

    sym_a: str
    sym_b: str
    category: str = ""

    @property
    def label(self) -> str:
        return f"{self.sym_a}/{self.sym_b}"


@dataclass(frozen=True, slots=True)
class ValidationResult:
    """等价对验证结果。

    诊断字段（可选）：
      cv — 比价 CV（Layer 1）
      stroke_mean_pct — 笔力度均值（Layer 2，仅在足够数据时计算）
      macd_norm_hist — 归一化 |hist| 均值（Layer 3，仅在 Layer 2 通过时计算）
      n_strokes — 管线产出的笔数（仅在运行 Layer 2 时填充）
    """

    valid: bool
    reason: str = ""
    cv: float | None = None
    stroke_mean_pct: float | None = None
    macd_norm_hist: float | None = None
    n_strokes: int | None = None


# ── 最小重叠K线数 ────────────────────────────────────────

MIN_OVERLAP = 5


# 数据不足此值时只跑 Layer 1（CV 预筛），不跑管线
_MIN_BARS_FOR_PIPELINE = 30

# 三层阈值（spec §1.2）
_T_CV = 0.01
_T_STROKE_PCT = 0.005  # 0.5%
_T_DYNAMICS_PCT = 0.0001  # 0.01%


def _align_pair(
    df_a: pd.DataFrame, df_b: pd.DataFrame, min_overlap: int,
) -> tuple[pd.DataFrame, pd.DataFrame, pd.Index] | ValidationResult:
    """对齐两个标的并做前置检查。返回对齐后的 (a, b, overlap) 或失败 ValidationResult。"""
    if (df_b["close"] == 0).any() or (df_b["open"] == 0).any():
        return ValidationResult(valid=False, reason="Zero price in B — division undefined")
    overlap = df_a.index.intersection(df_b.index)
    if len(overlap) < min_overlap:
        return ValidationResult(
            valid=False,
            reason=f"Insufficient overlap: {len(overlap)} bars (need {min_overlap})",
        )
    return df_a.loc[overlap], df_b.loc[overlap], overlap


def _layer1_cv(a_aligned: pd.DataFrame, b_aligned: pd.DataFrame, t_cv: float,
               ) -> tuple[float, float, ValidationResult | None]:
    """Layer 1: CV 预筛。返回 (cv, ratio_mean, 失败结果或None)。"""
    ratio = a_aligned["close"] / b_aligned["close"]
    ratio_mean = float(np.mean(ratio))
    ratio_std = float(np.std(ratio))
    cv = ratio_std / ratio_mean if ratio_mean != 0 else 0.0
    if cv < t_cv:
        return cv, ratio_mean, ValidationResult(
            valid=False,
            reason=f"Degenerate ratio — CV prescreen failed (cv={cv:.2e} < {t_cv})",
            cv=cv,
        )
    return cv, ratio_mean, None


def _layer2_and_3(
    a_aligned: pd.DataFrame, b_aligned: pd.DataFrame,
    cv: float, ratio_mean: float,
    t_stroke_pct: float, t_dynamics_pct: float,
) -> ValidationResult:
    """Layer 2 (结构退化) + Layer 3 (动力退化) 检测。"""
    ratio_kline = make_ratio_kline(a_aligned, b_aligned)
    stroke_mean_pct, n_strokes = _compute_stroke_intensity(ratio_kline)

    if n_strokes == 0:
        return ValidationResult(valid=True, cv=cv, n_strokes=0)
    if stroke_mean_pct < t_stroke_pct:
        return ValidationResult(
            valid=False, cv=cv, stroke_mean_pct=stroke_mean_pct, n_strokes=n_strokes,
            reason=f"Structure degenerate — stroke intensity too low "
                   f"(mean_pct={stroke_mean_pct:.4%} < {t_stroke_pct:.1%})",
        )

    macd_norm_hist = _compute_macd_dynamics(ratio_kline, ratio_mean)
    if macd_norm_hist < t_dynamics_pct:
        return ValidationResult(
            valid=False, cv=cv, stroke_mean_pct=stroke_mean_pct,
            macd_norm_hist=macd_norm_hist, n_strokes=n_strokes,
            reason=f"Dynamics degenerate — MACD area too low "
                   f"(norm_hist={macd_norm_hist:.2e} < {t_dynamics_pct:.2e})",
        )

    return ValidationResult(valid=True, cv=cv, stroke_mean_pct=stroke_mean_pct,
                            macd_norm_hist=macd_norm_hist, n_strokes=n_strokes)


def validate_pair(
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
    *,
    min_overlap: int = MIN_OVERLAP,
    t_cv: float = _T_CV,
    t_stroke_pct: float = _T_STROKE_PCT,
    t_dynamics_pct: float = _T_DYNAMICS_PCT,
) -> ValidationResult:
    """验证两个标的是否满足等价对条件（C-2 三层退化连锁检测）。"""
    aligned = _align_pair(df_a, df_b, min_overlap)
    if isinstance(aligned, ValidationResult):
        return aligned
    a_aligned, b_aligned, overlap = aligned

    cv, ratio_mean, fail = _layer1_cv(a_aligned, b_aligned, t_cv)
    if fail is not None:
        return fail

    if len(overlap) < _MIN_BARS_FOR_PIPELINE:
        return ValidationResult(valid=True, cv=cv)

    return _layer2_and_3(a_aligned, b_aligned, cv, ratio_mean, t_stroke_pct, t_dynamics_pct)


def _compute_stroke_intensity(ratio_kline: pd.DataFrame) -> tuple[float, int]:
    """运行管线并计算笔力度。返回 (mean_pct, n_strokes)。"""
    from newchan.a_fractal import fractals_from_merged
    from newchan.a_inclusion import merge_inclusion
    from newchan.a_stroke import strokes_from_fractals

    try:
        df_merged, _ = merge_inclusion(ratio_kline)
        fractals = fractals_from_merged(df_merged)
        strokes = strokes_from_fractals(df_merged, fractals)
    except Exception:
        return 0.0, 0

    if not strokes:
        return 0.0, 0

    intensities = []
    for s in strokes:
        if s.p0 != 0:
            intensities.append(abs(s.p1 / s.p0 - 1))

    if not intensities:
        return 0.0, len(strokes)

    return float(np.mean(intensities)), len(strokes)


def _compute_macd_dynamics(ratio_kline: pd.DataFrame, ratio_mean: float) -> float:
    """计算归一化 MACD |hist| 均值。"""
    from newchan.a_macd import compute_macd

    try:
        macd_df = compute_macd(ratio_kline)
        hist = macd_df["hist"]
        abs_mean = float(hist.abs().mean())
        norm = abs_mean / ratio_mean if ratio_mean != 0 else 0.0
    except Exception:
        norm = 0.0

    return norm


def make_ratio_kline(df_a: pd.DataFrame, df_b: pd.DataFrame) -> pd.DataFrame:
    """构造比价K线：A / B。

    对 OHLC 四列分别除法，volume 取 A。
    自动按时间戳对齐（inner join）。

    概念溯源：[旧缠论:隐含] 比价K线构造
    """
    idx = df_a.index.intersection(df_b.index)
    a, b = df_a.loc[idx], df_b.loc[idx]
    result = pd.DataFrame(index=a.index)
    result["open"] = a["open"] / b["open"]
    result["high"] = a["high"] / b["high"]
    result["low"] = a["low"] / b["low"]
    result["close"] = a["close"] / b["close"]
    if "volume" in a.columns:
        result["volume"] = a["volume"]
    return result
