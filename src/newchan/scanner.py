"""统一扫描器入口 -- 商空间排序作为默认排序方式。

366号下游推论1：L2 确认商空间排序与扁平排序产生实质差异，且差异方向与
292号区间套顺序一致（外层折叠优先）。商空间排序应作为默认排序方式。

本模块提供两条排序路径：
  1. quotient_rank（默认）：折叠等价类 + 商空间排序
  2. flat_rank（fallback）：纯 T 降序

调用方不指定时走商空间路径。

认识论标注：L0（从366号下游推论1直接推导）。
谱系引用：292号折叠拓扑、352号多标的扫描器、366号L2验证结论。
"""

from __future__ import annotations

from newchan.trading.fold_equivalence import (
    EquivalenceClass,
    TargetAttributes,
    build_quotient_space,
    rank_quotient_space,
)


def quotient_rank(
    targets: tuple[TargetAttributes, ...],
) -> tuple[EquivalenceClass, ...]:
    """商空间排序（默认路径）。

    步骤：
    1. 二值筛选：排除 T(S) = 0 的标的
    2. 构造商空间 F/~
    3. 按 rank([S]) = T([S]) * W(fold([S])) 降序排列

    Parameters
    ----------
    targets : tuple[TargetAttributes, ...]
        候选标的属性集合。

    Returns
    -------
    tuple[EquivalenceClass, ...]
        按 rank 降序排列的折叠等价类。
    """
    filtered = tuple(t for t in targets if t.tightness > 0.0)
    if not filtered:
        return ()
    return build_quotient_space(filtered)


def flat_rank(
    targets: tuple[TargetAttributes, ...],
) -> tuple[TargetAttributes, ...]:
    """扁平排序（fallback 路径）。

    纯按 T(S) 降序排序，不考虑折叠等价关系和权重。

    Parameters
    ----------
    targets : tuple[TargetAttributes, ...]
        候选标的属性集合。

    Returns
    -------
    tuple[TargetAttributes, ...]
        按 tightness 降序排列的标的。
    """
    filtered = tuple(t for t in targets if t.tightness > 0.0)
    return tuple(sorted(filtered, key=lambda t: -t.tightness))


def rank_targets(
    targets: tuple[TargetAttributes, ...],
    *,
    use_quotient: bool = True,
) -> list[tuple[str, float]]:
    """排序标的并返回 [(symbol, rank_score)] 列表。

    Parameters
    ----------
    targets : tuple[TargetAttributes, ...]
        候选标的属性集合。
    use_quotient : bool
        True（默认）使用商空间排序，False 使用扁平排序。

    Returns
    -------
    list[tuple[str, float]]
        [(symbol, score)] 按 score 降序。商空间模式下 score = rank([S])，
        扁平模式下 score = T(S)。
    """
    if use_quotient:
        eqs = quotient_rank(targets)
        results: list[tuple[str, float]] = []
        for ec in eqs:
            results.append((ec.representative.symbol, ec.rank))
        return results

    flat = flat_rank(targets)
    return [(t.symbol, t.tightness) for t in flat]
