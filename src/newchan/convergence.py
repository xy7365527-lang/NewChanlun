"""区间套收敛紧度量化（350号谱系）。

T(S) = D(S) * L(S) * C(S)

- D(S)：嵌套层数（多少个级别检测到背驰）
- L(S)：级别大小（链中最高 level_id）
- C(S)：各层一致性（方向一致率）

概念溯源: [旧缠论] 第27课 区间套（精确大转折点寻找程序定理）
谱系依据: 267号下游推论3, 350号
认识论等级: L0（从定义推导）
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from newchan.a_nested_divergence import NestedDivergence


@dataclass(frozen=True, slots=True)
class ConvergenceTightness:
    """收敛紧度分量 + 总分。"""

    depth: int          # D(S) 嵌套层数
    level_max: int      # L(S) 最高级别
    consistency: float  # C(S) 一致性 [0, 1]
    score: float        # T(S) = D * L * C


def convergence_tightness(nd: NestedDivergence) -> ConvergenceTightness:
    """从 NestedDivergence 计算收敛紧度。

    Parameters
    ----------
    nd : NestedDivergence
        区间套搜索结果（一条从高级别到低级别的背驰嵌套链）。

    Returns
    -------
    ConvergenceTightness
        三项分量 + 总分。D=0 时 score=0。
    """
    effective = [(lvl, div) for lvl, div in nd.chain if div is not None]

    if not effective:
        return ConvergenceTightness(depth=0, level_max=0, consistency=0.0, score=0.0)

    depth = len(effective)
    level_max = max(lvl for lvl, _ in effective)

    directions = [div.direction for _, div in effective]
    counts = Counter(directions)
    max_count = counts.most_common(1)[0][1]
    consistency = max_count / depth

    score = depth * level_max * consistency
    return ConvergenceTightness(
        depth=depth,
        level_max=level_max,
        consistency=consistency,
        score=score,
    )


def rank_by_convergence(
    results: list[tuple[str, NestedDivergence]],
) -> list[tuple[str, ConvergenceTightness]]:
    """多标的收敛排序。

    Parameters
    ----------
    results : list[tuple[str, NestedDivergence]]
        (标的标识, 区间套搜索结果) 列表。

    Returns
    -------
    list[tuple[str, ConvergenceTightness]]
        按 T(S) 降序排列。同 T 按 L 降序，再按 D 降序。
        T=0 的标的排在末尾。
    """
    scored = [
        (symbol, convergence_tightness(nd))
        for symbol, nd in results
    ]
    scored.sort(key=lambda x: (x[1].score, x[1].level_max, x[1].depth), reverse=True)
    return scored
