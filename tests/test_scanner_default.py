"""统一扫描器测试 -- 验证商空间排序是默认路径。

366号下游推论1：商空间排序作为默认排序方式。

验证：
1. quotient_rank 作为默认路径（use_quotient=True 是默认值）
2. flat_rank 作为 fallback
3. rank_targets 默认走商空间路径
4. 两条路径在退化条件下等价

认识论等级：L0（从366号推论直接推导）。
谱系引用：366号L2验证结论、352号多标的扫描器、292号折叠拓扑。
"""

from __future__ import annotations

import pytest

from newchan.scanner import flat_rank, quotient_rank, rank_targets
from newchan.trading.fold_equivalence import (
    DTriState,
    FoldChannel,
    TargetAttributes,
)


# ═══════════════════════════════════════════════════════════════
# Helper
# ═══════════════════════════════════════════════════════════════


def _t(
    symbol: str,
    fold: FoldChannel,
    *,
    sector: str = "",
    tri_state: DTriState = DTriState.RETAIN,
    tightness: float = 1.0,
    level_magnitude: float = 1.0,
    liquidity: float = 100.0,
) -> TargetAttributes:
    return TargetAttributes(
        symbol=symbol,
        fold_channel=fold,
        sector=sector,
        d_tri_state=tri_state,
        tightness=tightness,
        level_magnitude=level_magnitude,
        liquidity=liquidity,
    )


# ═══════════════════════════════════════════════════════════════
# quotient_rank 是默认路径
# ═══════════════════════════════════════════════════════════════


class TestQuotientRankDefault:
    """验证商空间排序是默认排序方式（366号下游推论1）。"""

    def test_rank_targets_defaults_to_quotient(self) -> None:
        """rank_targets 不指定 use_quotient 时走商空间路径。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),      # rank = 2*4 = 8
            _t("USO", FoldChannel.OIL, tightness=3.0),     # rank = 3*3 = 9
        )
        result = rank_targets(targets)
        assert result[0][0] == "USO"  # rank=9 > rank=8
        assert result[0][1] == pytest.approx(9.0)
        assert result[1][0] == "GLD"
        assert result[1][1] == pytest.approx(8.0)

    def test_quotient_differs_from_flat(self) -> None:
        """商空间排序与扁平排序产生不同结果——366号L2已确认。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),      # rank = 8, T = 2
            _t("USO", FoldChannel.OIL, tightness=3.0),     # rank = 9, T = 3
        )
        quotient_result = rank_targets(targets, use_quotient=True)
        flat_result = rank_targets(targets, use_quotient=False)
        # 扁平排序：T=3 USO 先，T=2 GLD 后（与商空间一致但分数不同）
        assert quotient_result[0][1] != flat_result[0][1]

    def test_weight_changes_order(self) -> None:
        """W 权重导致排序差异——外层折叠优先（292号区间套顺序）。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),      # rank = 8
            _t("TLT", FoldChannel.BOND, tightness=3.5),    # rank = 7
        )
        # 扁平排序：T=3.5 TLT 先
        flat_result = rank_targets(targets, use_quotient=False)
        assert flat_result[0][0] == "TLT"
        # 商空间排序：rank=8 GLD 先（W=4 放大效应）
        quotient_result = rank_targets(targets, use_quotient=True)
        assert quotient_result[0][0] == "GLD"


# ═══════════════════════════════════════════════════════════════
# quotient_rank 函数
# ═══════════════════════════════════════════════════════════════


class TestQuotientRank:
    """验证 quotient_rank 函数行为。"""

    def test_empty_targets(self) -> None:
        assert quotient_rank(()) == ()

    def test_filters_zero_tightness(self) -> None:
        targets = (
            _t("A", FoldChannel.AU, tightness=0.0),
            _t("B", FoldChannel.AU, tightness=2.0),
        )
        result = quotient_rank(targets)
        assert len(result) == 1
        assert result[0].representative.symbol == "B"

    def test_returns_equivalence_classes(self) -> None:
        targets = (
            _t("GLD", FoldChannel.AU, tightness=3.0),
            _t("GCF", FoldChannel.AU, tightness=2.0),
        )
        result = quotient_rank(targets)
        assert len(result) == 1
        assert result[0].size == 2

    def test_ordering_matches_build_quotient_space(self) -> None:
        """quotient_rank 的排序与 build_quotient_space 一致。"""
        from newchan.trading.fold_equivalence import build_quotient_space

        targets = (
            _t("A", FoldChannel.AU, tightness=2.0),
            _t("B", FoldChannel.OIL, tightness=3.0),
            _t("C", FoldChannel.BOND, tightness=1.5),
        )
        qr = quotient_rank(targets)
        bqs = build_quotient_space(targets)
        assert len(qr) == len(bqs)
        for a, b in zip(qr, bqs):
            assert a.representative.symbol == b.representative.symbol


# ═══════════════════════════════════════════════════════════════
# flat_rank fallback
# ═══════════════════════════════════════════════════════════════


class TestFlatRank:
    """验证扁平排序 fallback 路径。"""

    def test_empty_targets(self) -> None:
        assert flat_rank(()) == ()

    def test_filters_zero_tightness(self) -> None:
        targets = (
            _t("A", FoldChannel.AU, tightness=0.0),
            _t("B", FoldChannel.AU, tightness=2.0),
        )
        result = flat_rank(targets)
        assert len(result) == 1
        assert result[0].symbol == "B"

    def test_pure_tightness_ordering(self) -> None:
        """扁平排序纯按 T 降序，不考虑 W。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),
            _t("USO", FoldChannel.OIL, tightness=3.0),
            _t("TLT", FoldChannel.BOND, tightness=1.0),
        )
        result = flat_rank(targets)
        assert result[0].symbol == "USO"  # T=3
        assert result[1].symbol == "GLD"  # T=2
        assert result[2].symbol == "TLT"  # T=1


# ═══════════════════════════════════════════════════════════════
# rank_targets 接口
# ═══════════════════════════════════════════════════════════════


class TestRankTargets:
    """验证 rank_targets 统一接口。"""

    def test_empty_targets(self) -> None:
        assert rank_targets(()) == []

    def test_explicit_quotient(self) -> None:
        targets = (_t("GLD", FoldChannel.AU, tightness=2.0),)
        result = rank_targets(targets, use_quotient=True)
        assert len(result) == 1
        assert result[0][0] == "GLD"
        assert result[0][1] == pytest.approx(8.0)  # 2.0 * W=4

    def test_explicit_flat(self) -> None:
        targets = (_t("GLD", FoldChannel.AU, tightness=2.0),)
        result = rank_targets(targets, use_quotient=False)
        assert len(result) == 1
        assert result[0][0] == "GLD"
        assert result[0][1] == pytest.approx(2.0)  # pure T

    def test_all_zero_tightness(self) -> None:
        targets = (
            _t("A", FoldChannel.AU, tightness=0.0),
            _t("B", FoldChannel.OIL, tightness=0.0),
        )
        assert rank_targets(targets) == []
        assert rank_targets(targets, use_quotient=False) == []

    def test_equivalence_class_merging_in_quotient(self) -> None:
        """商空间模式下，同等价类标的合并为一个条目（代表元）。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=3.0),
            _t("GCF", FoldChannel.AU, tightness=2.0),
        )
        quotient_result = rank_targets(targets, use_quotient=True)
        flat_result = rank_targets(targets, use_quotient=False)
        # 商空间：1 个等价类 → 1 个条目
        assert len(quotient_result) == 1
        assert quotient_result[0][0] == "GLD"  # 代表元
        # 扁平：2 个独立标的
        assert len(flat_result) == 2


# ═══════════════════════════════════════════════════════════════
# 退化条件：单元素等价类 → 商空间退化为加权扁平排序
# ═══════════════════════════════════════════════════════════════


class TestDegenerateCases:
    """当每个等价类只有一个成员时，商空间排序退化为 T*W 加权排序。"""

    def test_singletons_maintain_weight_ordering(self) -> None:
        targets = (
            _t("A", FoldChannel.AU, tightness=1.0),      # rank = 4
            _t("B", FoldChannel.OIL, tightness=1.0),     # rank = 3
            _t("C", FoldChannel.BOND, tightness=1.0),    # rank = 2
            _t("D", FoldChannel.RE, tightness=1.0),      # rank = 1
        )
        result = rank_targets(targets)
        assert [s for s, _ in result] == ["A", "B", "C", "D"]

    def test_equal_t_different_w(self) -> None:
        """同 T 不同 W → 商空间排序按 W 降序（292号区间套顺序）。"""
        targets = (
            _t("D", FoldChannel.RE, tightness=5.0),       # rank = 5
            _t("A", FoldChannel.AU, tightness=5.0),        # rank = 20
        )
        result = rank_targets(targets)
        assert result[0][0] == "A"   # W=4 → rank=20
        assert result[1][0] == "D"   # W=1 → rank=5
