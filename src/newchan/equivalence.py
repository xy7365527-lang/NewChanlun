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
    """等价对验证结果。"""

    valid: bool
    reason: str = ""


# ── 最小重叠K线数 ────────────────────────────────────────

MIN_OVERLAP = 5


def validate_pair(
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
    *,
    min_overlap: int = MIN_OVERLAP,
    degeneracy_threshold: float = 1e-6,
) -> ValidationResult:
    """验证两个标的是否满足等价对条件。

    Parameters
    ----------
    df_a, df_b : pd.DataFrame
        OHLCV 数据，需有 DatetimeIndex 和 close 列。
    min_overlap : int
        最小重叠K线数。
    degeneracy_threshold : float
        比价标准差低于此值视为退化（常数比价）。
    """
    # 条件0：B 不能有零价格
    if (df_b["close"] == 0).any() or (df_b["open"] == 0).any():
        return ValidationResult(valid=False, reason="Zero price in B — division undefined")

    # 条件1：可比性 — 重叠时间窗口
    overlap = df_a.index.intersection(df_b.index)
    if len(overlap) < min_overlap:
        return ValidationResult(
            valid=False,
            reason=f"Insufficient overlap: {len(overlap)} bars (need {min_overlap})",
        )

    # 对齐
    a_aligned = df_a.loc[overlap]
    b_aligned = df_b.loc[overlap]

    # 条件2：非退化 — 比价不是常数
    ratio = a_aligned["close"] / b_aligned["close"]
    if np.std(ratio) < degeneracy_threshold:
        return ValidationResult(
            valid=False,
            reason=f"Degenerate (constant) ratio — std={np.std(ratio):.2e}",
        )

    return ValidationResult(valid=True)


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
