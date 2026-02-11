"""A 系统 — 特征序列分型识别

在标准特征序列上识别顶/底分型，供线段 v1 判定终结。

规格引用: docs/chan_spec.md §5.4 v1 特征序列法
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.a_feature_sequence import FeatureBar


# ====================================================================
# 数据类型
# ====================================================================

@dataclass(frozen=True, slots=True)
class FeatureFractal:
    """特征序列上的分型。

    Attributes
    ----------
    idx : int
        分型中心在标准特征序列中的位置索引（0-based）。
    kind : ``"top"`` | ``"bottom"``
    high : float
        中心 FeatureBar 的 high。
    low : float
        中心 FeatureBar 的 low。
    """

    idx: int
    kind: Literal["top", "bottom"]
    high: float
    low: float


# ====================================================================
# 分型识别（双条件，与 Step9 同规则）
# ====================================================================

def fractals_from_feature(seq: list[FeatureBar]) -> list[FeatureFractal]:
    """在标准特征序列上识别全部分型。

    Parameters
    ----------
    seq : list[FeatureBar]
        标准（包含处理后的）特征序列。

    Returns
    -------
    list[FeatureFractal]
        按 idx 递增排序。

    Notes
    -----
    双条件判定（与 docs/chan_spec.md §3 完全一致，但对象是 FeatureBar）：

    顶分型 Top::
        high_i > high_{i-1}  AND  high_i > high_{i+1}
        AND
        low_i  > low_{i-1}   AND  low_i  > low_{i+1}

    底分型 Bottom::
        low_i  < low_{i-1}   AND  low_i  < low_{i+1}
        AND
        high_i < high_{i-1}  AND  high_i < high_{i+1}
    """
    n = len(seq)
    if n < 3:
        return []

    result: list[FeatureFractal] = []

    for i in range(1, n - 1):
        h_prev, h_curr, h_next = seq[i - 1].high, seq[i].high, seq[i + 1].high
        l_prev, l_curr, l_next = seq[i - 1].low, seq[i].low, seq[i + 1].low

        # 顶分型
        if (
            h_curr > h_prev and h_curr > h_next
            and l_curr > l_prev and l_curr > l_next
        ):
            result.append(FeatureFractal(
                idx=i, kind="top", high=h_curr, low=l_curr,
            ))

        # 底分型
        elif (
            l_curr < l_prev and l_curr < l_next
            and h_curr < h_prev and h_curr < h_next
        ):
            result.append(FeatureFractal(
                idx=i, kind="bottom", high=h_curr, low=l_curr,
            ))

    return result
