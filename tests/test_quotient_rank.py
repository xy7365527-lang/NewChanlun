"""商空间排序器测试 — 验证 rank([S]) = T([S]) * W(fold([S])) 四级打破排序。

验证：
1. rank 公式正确性
2. 四级打破排序（352号 §4.3）：rank → T → L → size
3. W 权重与折叠通道的映射
4. rank_quotient_space 输出格式
5. 退化条件：商空间退化为扁平排序
6. level_magnitude 打破层级

认识论等级：L0（纯代数/定义层，从352号排序规则直接推导）。

谱系引用：350号收敛紧度、352号多标的扫描器设计§4。
"""

from __future__ import annotations

import pytest

from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FOLD_WEIGHT,
    FoldChannel,
    TargetAttributes,
    build_quotient_space,
    rank_quotient_space,
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
# rank 公式
# ═══════════════════════════════════════════════════════════════


class TestRankFormula:
    """rank([S]) = T([S]) * W(fold([S]))（352号 §4.1）。"""

    def test_au_rank(self) -> None:
        qs = build_quotient_space((_t("GLD", FoldChannel.AU, tightness=3.0),))
        assert qs[0].rank == pytest.approx(3.0 * 4)

    def test_oil_rank(self) -> None:
        qs = build_quotient_space((_t("USO", FoldChannel.OIL, tightness=2.0),))
        assert qs[0].rank == pytest.approx(2.0 * 3)

    def test_bond_rank(self) -> None:
        qs = build_quotient_space((_t("TLT", FoldChannel.BOND, tightness=1.5),))
        assert qs[0].rank == pytest.approx(1.5 * 2)

    def test_re_rank(self) -> None:
        qs = build_quotient_space((_t("XLRE", FoldChannel.RE, tightness=2.5),))
        assert qs[0].rank == pytest.approx(2.5 * 1)

    def test_equity_rank(self) -> None:
        qs = build_quotient_space((
            _t("AAPL", FoldChannel.EQUITY, sector="tech", tightness=2.0),
        ))
        assert qs[0].rank == pytest.approx(2.0 * 1)

    def test_multi_member_uses_max_tightness(self) -> None:
        """等价类 rank 用类内最大 T，不是代表元 T。"""
        targets = (
            _t("GLD", FoldChannel.AU, tightness=3.0),
            _t("GCF", FoldChannel.AU, tightness=5.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].rank == pytest.approx(5.0 * 4)


# ═══════════════════════════════════════════════════════════════
# 四级打破排序
# ═══════════════════════════════════════════════════════════════


class TestFourLevelTieBreaking:
    """352号 §4.3 四级打破：rank → T → L → size。"""

    def test_rank_primary_sort(self) -> None:
        """rank 不同时，rank 决定顺序。"""
        targets = (
            _t("USO", FoldChannel.OIL, tightness=2.0),   # rank=6
            _t("GLD", FoldChannel.AU, tightness=2.0),     # rank=8
        )
        qs = build_quotient_space(targets)
        assert qs[0].fold_channel is FoldChannel.AU
        assert qs[1].fold_channel is FoldChannel.OIL

    def test_tightness_secondary_sort(self) -> None:
        """rank 相等时，T 更大者优先。"""
        # rank=6 for both: Oil T=2 W=3 vs Bond T=3 W=2
        targets = (
            _t("USO", FoldChannel.OIL, tightness=2.0),
            _t("TLT", FoldChannel.BOND, tightness=3.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].representative.symbol == "TLT"   # T=3 > T=2
        assert qs[1].representative.symbol == "USO"

    def test_level_magnitude_tertiary_sort(self) -> None:
        """rank 和 T 都相等时，L 更大者优先（大级别优先——267号）。"""
        # 两个 EQUITY 等价类，同 T 同 W=1 → 同 rank → 同 T → L 打破
        targets = (
            _t("AAPL", FoldChannel.EQUITY, sector="tech",
                tightness=2.0, level_magnitude=3.0),
            _t("JPM", FoldChannel.EQUITY, sector="finance",
                tightness=2.0, level_magnitude=5.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].representative.symbol == "JPM"   # L=5 > L=3
        assert qs[1].representative.symbol == "AAPL"

    def test_size_quaternary_sort(self) -> None:
        """rank、T、L 都相等时，|[S]| 更大者优先。"""
        targets = (
            _t("AAPL", FoldChannel.EQUITY, sector="tech",
                tightness=2.0, level_magnitude=3.0),
            _t("MSFT", FoldChannel.EQUITY, sector="tech",
                tightness=2.0, level_magnitude=3.0),
            _t("JPM", FoldChannel.EQUITY, sector="finance",
                tightness=2.0, level_magnitude=3.0),
        )
        qs = build_quotient_space(targets)
        # tech: size=2, finance: size=1. rank/T/L all equal → size breaks
        assert qs[0].fold_channel is FoldChannel.EQUITY
        assert qs[0].size == 2
        assert qs[1].size == 1

    def test_full_four_level_cascade(self) -> None:
        """四级打破完整联动测试。"""
        targets = (
            _t("A", FoldChannel.AU, tightness=2.0, level_magnitude=1.0),    # rank=8
            _t("B", FoldChannel.OIL, tightness=2.0, level_magnitude=2.0),   # rank=6
            _t("C", FoldChannel.BOND, tightness=3.0, level_magnitude=3.0),  # rank=6
            _t("D", FoldChannel.RE, tightness=6.0, level_magnitude=4.0),    # rank=6
            _t("E", FoldChannel.EQUITY, sector="s1",
                tightness=6.0, level_magnitude=4.0),                        # rank=6
        )
        qs = build_quotient_space(targets)
        symbols = [ec.representative.symbol for ec in qs]
        # rank=8: A
        # rank=6 group → T=6: D, E → T=3: C → T=2: B
        # D vs E: same T=6, same L=4 → size tie (both 1) → stable
        assert symbols[0] == "A"
        # D and E both have rank=6, T=6, L=4, size=1
        assert {symbols[1], symbols[2]} == {"D", "E"}
        assert symbols[3] == "C"
        assert symbols[4] == "B"


# ═══════════════════════════════════════════════════════════════
# max_level 属性
# ═══════════════════════════════════════════════════════════════


class TestMaxLevel:
    """验证 EquivalenceClass.max_level 正确计算。"""

    def test_single_member(self) -> None:
        qs = build_quotient_space((
            _t("GLD", FoldChannel.AU, tightness=2.0, level_magnitude=5.0),
        ))
        assert qs[0].max_level == pytest.approx(5.0)

    def test_multi_member_takes_max(self) -> None:
        targets = (
            _t("GLD", FoldChannel.AU, tightness=3.0, level_magnitude=2.0),
            _t("GCF", FoldChannel.AU, tightness=2.0, level_magnitude=7.0),
            _t("NEM", FoldChannel.AU, tightness=1.0, level_magnitude=4.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].max_level == pytest.approx(7.0)

    def test_max_level_independent_of_tightness_order(self) -> None:
        """max_level 不一定来自 T 最大的成员。"""
        targets = (
            _t("A", FoldChannel.OIL, tightness=5.0, level_magnitude=1.0),
            _t("B", FoldChannel.OIL, tightness=1.0, level_magnitude=9.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].max_tightness == pytest.approx(5.0)
        assert qs[0].max_level == pytest.approx(9.0)


# ═══════════════════════════════════════════════════════════════
# W 权重映射
# ═══════════════════════════════════════════════════════════════


class TestWeightMapping:
    """折叠共享性权重 W 与通道的映射（352号 §4.2）。"""

    def test_weight_values(self) -> None:
        assert FOLD_WEIGHT[FoldChannel.AU] == 4
        assert FOLD_WEIGHT[FoldChannel.OIL] == 3
        assert FOLD_WEIGHT[FoldChannel.BOND] == 2
        assert FOLD_WEIGHT[FoldChannel.RE] == 1
        assert FOLD_WEIGHT[FoldChannel.EQUITY] == 1

    def test_weight_via_equivalence_class(self) -> None:
        qs = build_quotient_space((_t("GLD", FoldChannel.AU),))
        assert qs[0].weight == 4

    def test_weight_semantics(self) -> None:
        """外层折叠权重 > 内层折叠权重（292号区间套顺序）。"""
        assert FOLD_WEIGHT[FoldChannel.AU] > FOLD_WEIGHT[FoldChannel.OIL]
        assert FOLD_WEIGHT[FoldChannel.OIL] > FOLD_WEIGHT[FoldChannel.BOND]
        assert FOLD_WEIGHT[FoldChannel.BOND] > FOLD_WEIGHT[FoldChannel.RE]


# ═══════════════════════════════════════════════════════════════
# rank_quotient_space 输出格式
# ═══════════════════════════════════════════════════════════════


class TestRankOutputFormat:
    """验证 352号 §4.4 输出格式 [(rank, [S], rep([S]))...]。"""

    def test_triple_structure(self) -> None:
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),
            _t("USO", FoldChannel.OIL, tightness=1.5),
        )
        ranked = rank_quotient_space(build_quotient_space(targets))
        assert len(ranked) == 2
        for rank_val, ec, rep in ranked:
            assert isinstance(rank_val, float)
            assert isinstance(ec, EquivalenceClass)
            assert isinstance(rep, TargetAttributes)

    def test_rep_is_class_representative(self) -> None:
        targets = (
            _t("GLD", FoldChannel.AU, tightness=3.0),
            _t("GCF", FoldChannel.AU, tightness=2.0),
        )
        ranked = rank_quotient_space(build_quotient_space(targets))
        _, ec, rep = ranked[0]
        assert rep is ec.representative
        assert rep.symbol == "GLD"

    def test_output_preserves_rank_order(self) -> None:
        targets = (
            _t("A", FoldChannel.RE, tightness=1.0),       # rank=1
            _t("B", FoldChannel.AU, tightness=2.0),        # rank=8
            _t("C", FoldChannel.BOND, tightness=3.0),      # rank=6
        )
        ranked = rank_quotient_space(build_quotient_space(targets))
        ranks = [r for r, _, _ in ranked]
        assert ranks == sorted(ranks, reverse=True)

    def test_empty_quotient(self) -> None:
        ranked = rank_quotient_space(())
        assert ranked == ()


# ═══════════════════════════════════════════════════════════════
# 退化条件
# ═══════════════════════════════════════════════════════════════


class TestDegenerateCases:
    """352号边界条件1：等价类退化为单元素集时，商空间退化为扁平排序。"""

    def test_all_singletons_equals_flat_rank_sort(self) -> None:
        targets = (
            _t("GLD", FoldChannel.AU, tightness=2.0),
            _t("USO", FoldChannel.OIL, tightness=3.0),
            _t("TLT", FoldChannel.BOND, tightness=1.0),
        )
        qs = build_quotient_space(targets)
        assert all(ec.size == 1 for ec in qs)
        flat_sorted = sorted(
            targets, key=lambda t: -t.tightness * FOLD_WEIGHT[t.fold_channel]
        )
        for ec, t in zip(qs, flat_sorted):
            assert ec.representative.symbol == t.symbol

    def test_single_target_single_class(self) -> None:
        qs = build_quotient_space((_t("GLD", FoldChannel.AU, tightness=5.0),))
        assert len(qs) == 1
        assert qs[0].size == 1
        assert qs[0].rank == pytest.approx(20.0)

    def test_empty_yields_empty(self) -> None:
        assert build_quotient_space(()) == ()


# ═══════════════════════════════════════════════════════════════
# 高 T 低 W 可胜过 低 T 高 W
# ═══════════════════════════════════════════════════════════════


class TestRankCrossover:
    """rank 允许高 T 内层折叠胜过低 T 外层折叠。"""

    def test_high_t_equity_beats_low_t_au(self) -> None:
        """T=10 EQUITY (rank=10) > T=2 AU (rank=8)。"""
        targets = (
            _t("AU", FoldChannel.AU, tightness=2.0),
            _t("EQ", FoldChannel.EQUITY, sector="s1", tightness=10.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].representative.symbol == "EQ"

    def test_equal_rank_crossover(self) -> None:
        """Oil T=4 (rank=12) vs AU T=3 (rank=12) → T 打破: Oil wins。"""
        targets = (
            _t("AU", FoldChannel.AU, tightness=3.0),
            _t("OIL", FoldChannel.OIL, tightness=4.0),
        )
        qs = build_quotient_space(targets)
        assert qs[0].representative.symbol == "OIL"  # T=4 > T=3
