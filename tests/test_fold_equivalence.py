"""折叠等价类构造器测试。

验证：
1. 等价关系三公理（自反性、对称性、传递性）
2. 非等价判定
3. 代表元选取（T 最大 → 流动性打破）
4. 商空间构造 + 排序
5. 边界条件：空输入、单标的、全等价、全不等价

认识论等级：L0（纯代数/定义层，从292号等价关系定义推导）。

谱系引用：292号折叠拓扑本体论、352号多标的扫描器设计。
"""

from __future__ import annotations

import pytest

from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FOLD_WEIGHT,
    FoldChannel,
    TargetAttributes,
    are_fold_equivalent,
    build_quotient_space,
    equivalence_key,
    rank_quotient_space,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


def _target(
    symbol: str,
    fold: FoldChannel,
    *,
    sector: str = "",
    tri_state: DTriState = DTriState.RETAIN,
    tightness: float = 1.0,
    level_magnitude: float = 1.0,
    liquidity: float = 100.0,
) -> TargetAttributes:
    """快速构造 TargetAttributes。"""
    return TargetAttributes(
        symbol=symbol,
        fold_channel=fold,
        sector=sector,
        d_tri_state=tri_state,
        tightness=tightness,
        level_magnitude=level_magnitude,
        liquidity=liquidity,
    )


# Au 折叠通道标的
GLD = _target("GLD", FoldChannel.AU, tightness=3.0, liquidity=500.0)
GCF = _target("GC=F", FoldChannel.AU, tightness=2.5, liquidity=300.0)
GOLD_STOCK = _target("NEM", FoldChannel.AU, tightness=2.0, liquidity=200.0)

# Oil 折叠通道标的
USO = _target("USO", FoldChannel.OIL, tightness=2.0, liquidity=400.0)
CLF = _target("CL=F", FoldChannel.OIL, tightness=2.5, liquidity=600.0)

# Bond 折叠通道标的
TLT = _target("TLT", FoldChannel.BOND, tightness=1.5, liquidity=700.0)
ZNF = _target("ZN=F", FoldChannel.BOND, tightness=1.5, liquidity=400.0)

# RE 折叠通道标的
XLRE = _target("XLRE", FoldChannel.RE, tightness=1.0, liquidity=300.0)

# Equity 同板块标的
XLK_RETAIN = _target(
    "XLK", FoldChannel.EQUITY, sector="tech", tri_state=DTriState.RETAIN,
    tightness=2.0, liquidity=800.0,
)
AAPL_RETAIN = _target(
    "AAPL", FoldChannel.EQUITY, sector="tech", tri_state=DTriState.RETAIN,
    tightness=2.5, liquidity=1000.0,
)
MSFT_RETAIN = _target(
    "MSFT", FoldChannel.EQUITY, sector="tech", tri_state=DTriState.RETAIN,
    tightness=2.5, liquidity=900.0,
)
# 同板块但三态不一致
GOOG_AMPLIFY = _target(
    "GOOG", FoldChannel.EQUITY, sector="tech", tri_state=DTriState.AMPLIFY,
    tightness=3.0, liquidity=1200.0,
)
# 不同板块
XLF_RETAIN = _target(
    "XLF", FoldChannel.EQUITY, sector="finance", tri_state=DTriState.RETAIN,
    tightness=1.8, liquidity=600.0,
)


# ═══════════════════════════════════════════════════════════════
# 等价关系三公理
# ═══════════════════════════════════════════════════════════════


class TestEquivalenceAxioms:
    """验证 ~ 是等价关系（自反性 + 对称性 + 传递性）。"""

    def test_reflexivity_au(self) -> None:
        """自反性：S ~ S 对任意 S 成立。"""
        assert are_fold_equivalent(GLD, GLD)

    def test_reflexivity_equity(self) -> None:
        assert are_fold_equivalent(XLK_RETAIN, XLK_RETAIN)

    def test_symmetry_au(self) -> None:
        """对称性：S1 ~ S2 → S2 ~ S1。"""
        assert are_fold_equivalent(GLD, GCF)
        assert are_fold_equivalent(GCF, GLD)

    def test_symmetry_equity(self) -> None:
        assert are_fold_equivalent(AAPL_RETAIN, MSFT_RETAIN)
        assert are_fold_equivalent(MSFT_RETAIN, AAPL_RETAIN)

    def test_transitivity_au(self) -> None:
        """传递性：S1 ~ S2 且 S2 ~ S3 → S1 ~ S3。"""
        assert are_fold_equivalent(GLD, GCF)
        assert are_fold_equivalent(GCF, GOLD_STOCK)
        assert are_fold_equivalent(GLD, GOLD_STOCK)

    def test_transitivity_equity(self) -> None:
        assert are_fold_equivalent(XLK_RETAIN, AAPL_RETAIN)
        assert are_fold_equivalent(AAPL_RETAIN, MSFT_RETAIN)
        assert are_fold_equivalent(XLK_RETAIN, MSFT_RETAIN)


# ═══════════════════════════════════════════════════════════════
# 非等价判定
# ═══════════════════════════════════════════════════════════════


class TestNonEquivalence:
    """验证不等价情况的正确判定。"""

    def test_different_fold_channel(self) -> None:
        """不同折叠通道不等价。"""
        assert not are_fold_equivalent(GLD, USO)
        assert not are_fold_equivalent(GLD, TLT)
        assert not are_fold_equivalent(USO, XLRE)

    def test_same_sector_different_tristate(self) -> None:
        """同板块但 D 算子三态不一致 → 不等价。"""
        assert not are_fold_equivalent(AAPL_RETAIN, GOOG_AMPLIFY)

    def test_different_sector_same_tristate(self) -> None:
        """不同板块即使三态一致也不等价。"""
        assert not are_fold_equivalent(XLK_RETAIN, XLF_RETAIN)

    def test_equity_vs_fold_channel(self) -> None:
        """EQUITY 通道与其他折叠通道不等价。"""
        assert not are_fold_equivalent(AAPL_RETAIN, GLD)

    def test_re_vs_equity_not_equivalent(self) -> None:
        """RE 通道与 EQUITY 通道不等价（L2 发现的别名 bug 回归测试）。"""
        re_target = _target("XLRE", FoldChannel.RE, tightness=1.0)
        eq_target = _target("AAPL", FoldChannel.EQUITY, sector="tech",
                            tri_state=DTriState.RETAIN, tightness=1.0)
        assert not are_fold_equivalent(re_target, eq_target)


# ═══════════════════════════════════════════════════════════════
# equivalence_key
# ═══════════════════════════════════════════════════════════════


class TestEquivalenceKey:
    """验证 equivalence_key 的正确性。"""

    def test_au_key(self) -> None:
        assert equivalence_key(GLD) == (FoldChannel.AU, "", None)
        assert equivalence_key(GCF) == (FoldChannel.AU, "", None)

    def test_oil_key(self) -> None:
        assert equivalence_key(USO) == (FoldChannel.OIL, "", None)

    def test_equity_key_includes_sector_and_tristate(self) -> None:
        key = equivalence_key(AAPL_RETAIN)
        assert key == (FoldChannel.EQUITY, "tech", DTriState.RETAIN)

    def test_equity_different_tristate_different_key(self) -> None:
        assert equivalence_key(AAPL_RETAIN) != equivalence_key(GOOG_AMPLIFY)


# ═══════════════════════════════════════════════════════════════
# 代表元选取
# ═══════════════════════════════════════════════════════════════


class TestRepresentativeSelection:
    """验证代表元选取规则。"""

    def test_highest_tightness_wins(self) -> None:
        """T(S) 最大者为代表元。"""
        targets = (GLD, GCF, GOLD_STOCK)
        qs = build_quotient_space(targets)
        assert len(qs) == 1
        assert qs[0].representative.symbol == "GLD"  # T=3.0 > 2.5 > 2.0

    def test_liquidity_breaks_tie(self) -> None:
        """同 T 时流动性最大者为代表元。"""
        # AAPL: T=2.5 liq=1000, MSFT: T=2.5 liq=900
        targets = (AAPL_RETAIN, MSFT_RETAIN)
        qs = build_quotient_space(targets)
        assert len(qs) == 1
        assert qs[0].representative.symbol == "AAPL"  # liq 1000 > 900

    def test_oil_representative(self) -> None:
        """Oil 通道代表元。"""
        targets = (USO, CLF)
        qs = build_quotient_space(targets)
        assert len(qs) == 1
        assert qs[0].representative.symbol == "CL=F"  # T=2.5 > 2.0


# ═══════════════════════════════════════════════════════════════
# 商空间构造
# ═══════════════════════════════════════════════════════════════


class TestQuotientSpace:
    """验证商空间 F/~ 的构造和排序。"""

    def test_empty_input(self) -> None:
        """空输入 → 空商空间。"""
        assert build_quotient_space(()) == ()

    def test_single_target(self) -> None:
        """单标的 → 单等价类。"""
        qs = build_quotient_space((GLD,))
        assert len(qs) == 1
        assert qs[0].size == 1
        assert qs[0].representative is GLD

    def test_all_equivalent(self) -> None:
        """所有标的等价 → 单等价类。"""
        targets = (GLD, GCF, GOLD_STOCK)
        qs = build_quotient_space(targets)
        assert len(qs) == 1
        assert qs[0].size == 3

    def test_all_distinct(self) -> None:
        """所有标的不等价 → 等价类数 = 标的数。"""
        targets = (GLD, USO, TLT, XLRE)
        qs = build_quotient_space(targets)
        assert len(qs) == 4

    def test_mixed_grouping(self) -> None:
        """混合情况：多个等价类，不同大小。"""
        targets = (GLD, GCF, USO, CLF, TLT, AAPL_RETAIN, MSFT_RETAIN, GOOG_AMPLIFY)
        qs = build_quotient_space(targets)
        # Au: {GLD, GCF}, Oil: {USO, CLF}, Bond: {TLT},
        # tech/RETAIN: {AAPL, MSFT}, tech/AMPLIFY: {GOOG}
        assert len(qs) == 5

    def test_quotient_preserves_all_members(self) -> None:
        """商空间包含所有原始标的。"""
        targets = (GLD, GCF, USO, TLT)
        qs = build_quotient_space(targets)
        all_symbols = set()
        for ec in qs:
            for m in ec.members:
                all_symbols.add(m.symbol)
        assert all_symbols == {"GLD", "GC=F", "USO", "TLT"}


# ═══════════════════════════════════════════════════════════════
# 商空间排序
# ═══════════════════════════════════════════════════════════════


class TestQuotientSpaceOrdering:
    """验证商空间排序规则（352号 §4.3）。"""

    def test_rank_formula(self) -> None:
        """rank([S]) = T([S]) * W(fold([S]))。"""
        targets = (GLD, GCF)  # Au, W=4, max_T=3.0
        qs = build_quotient_space(targets)
        assert qs[0].rank == pytest.approx(3.0 * 4)

    def test_weight_by_channel(self) -> None:
        """折叠共享性权重 W 正确。"""
        assert FOLD_WEIGHT[FoldChannel.AU] == 4
        assert FOLD_WEIGHT[FoldChannel.OIL] == 3
        assert FOLD_WEIGHT[FoldChannel.BOND] == 2
        assert FOLD_WEIGHT[FoldChannel.RE] == 1
        assert FOLD_WEIGHT[FoldChannel.EQUITY] == 1

    def test_re_and_equity_are_distinct(self) -> None:
        """RE 和 EQUITY 必须是不同的 enum 成员（L2 发现的别名 bug）。"""
        assert FoldChannel.RE is not FoldChannel.EQUITY
        assert FoldChannel.RE != FoldChannel.EQUITY

    def test_au_ranks_above_oil_at_equal_tightness(self) -> None:
        """同 T 时，Au (W=4) 排在 Oil (W=3) 前。"""
        au_target = _target("GLD", FoldChannel.AU, tightness=2.0)
        oil_target = _target("USO", FoldChannel.OIL, tightness=2.0)
        qs = build_quotient_space((au_target, oil_target))
        assert qs[0].fold_channel is FoldChannel.AU
        assert qs[1].fold_channel is FoldChannel.OIL

    def test_high_tightness_oil_can_beat_low_tightness_au(self) -> None:
        """高 T 的 Oil 可以排在低 T 的 Au 前。"""
        au_low = _target("GLD", FoldChannel.AU, tightness=1.0)  # rank = 1.0 * 4 = 4
        oil_high = _target("USO", FoldChannel.OIL, tightness=2.0)  # rank = 2.0 * 3 = 6
        qs = build_quotient_space((au_low, oil_high))
        assert qs[0].fold_channel is FoldChannel.OIL  # rank 6 > 4

    def test_full_ordering(self) -> None:
        """完整排序测试。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),        # rank = 8
            _target("USO", FoldChannel.OIL, tightness=2.0),       # rank = 6
            _target("TLT", FoldChannel.BOND, tightness=3.0),      # rank = 6
            _target("XLRE", FoldChannel.RE, tightness=1.0),       # rank = 1
        )
        qs = build_quotient_space(targets)
        ranks = [ec.rank for ec in qs]
        # rank desc: 8, 6, 6, 1
        assert ranks[0] == pytest.approx(8.0)
        # rank=6 tie → T desc: TLT(T=3) > USO(T=2)
        assert qs[1].representative.symbol == "TLT"
        assert qs[2].representative.symbol == "USO"
        assert ranks[3] == pytest.approx(1.0)


# ═══════════════════════════════════════════════════════════════
# rank_quotient_space 输出格式
# ═══════════════════════════════════════════════════════════════


class TestRankOutput:
    """验证 352号 §4.4 输出格式。"""

    def test_output_format(self) -> None:
        """输出为 (rank, [S], rep([S])) 三元组。"""
        targets = (GLD, USO, TLT)
        qs = build_quotient_space(targets)
        ranked = rank_quotient_space(qs)
        assert len(ranked) == 3
        for rank_val, ec, rep in ranked:
            assert isinstance(rank_val, float)
            assert isinstance(ec, EquivalenceClass)
            assert isinstance(rep, TargetAttributes)
            assert rep is ec.representative

    def test_output_ordering(self) -> None:
        """输出按 rank 降序。"""
        targets = (GLD, USO, TLT)
        ranked = rank_quotient_space(build_quotient_space(targets))
        ranks = [r for r, _, _ in ranked]
        assert ranks == sorted(ranks, reverse=True)


# ═══════════════════════════════════════════════════════════════
# EquivalenceClass 属性
# ═══════════════════════════════════════════════════════════════


class TestEquivalenceClassProperties:
    """验证 EquivalenceClass 的计算属性。"""

    def test_size(self) -> None:
        qs = build_quotient_space((GLD, GCF, GOLD_STOCK))
        assert qs[0].size == 3

    def test_weight(self) -> None:
        qs = build_quotient_space((GLD,))
        assert qs[0].weight == 4

    def test_max_tightness(self) -> None:
        qs = build_quotient_space((GLD, GCF, GOLD_STOCK))
        assert qs[0].max_tightness == pytest.approx(3.0)

    def test_members_sorted(self) -> None:
        """类内成员按 tightness desc, liquidity desc 排序。"""
        qs = build_quotient_space((GOLD_STOCK, GLD, GCF))
        symbols = [m.symbol for m in qs[0].members]
        assert symbols == ["GLD", "GC=F", "NEM"]  # T: 3.0, 2.5, 2.0


# ═══════════════════════════════════════════════════════════════
# 退化条件
# ═══════════════════════════════════════════════════════════════


class TestDegenerateCases:
    """352号边界条件1：等价类退化为单元素集时，商空间退化为扁平排序。"""

    def test_all_singletons_degenerates_to_flat_sort(self) -> None:
        """每个标的单独成类时，rank 排序等价于 T*W 扁平排序。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),
            _target("USO", FoldChannel.OIL, tightness=3.0),
            _target("TLT", FoldChannel.BOND, tightness=1.0),
        )
        qs = build_quotient_space(targets)
        assert all(ec.size == 1 for ec in qs)
        # 扁平排序等价于 rank 排序
        flat_sorted = sorted(targets, key=lambda t: -t.tightness * FOLD_WEIGHT[t.fold_channel])
        for ec, t in zip(qs, flat_sorted):
            assert ec.representative.symbol == t.symbol

    def test_equity_without_fold_channels(self) -> None:
        """纯 EQUITY 标的池且板块/三态不同 → 全退化为单元素集。"""
        targets = (
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0),
            _target("JPM", FoldChannel.EQUITY, sector="finance",
                    tri_state=DTriState.AMPLIFY, tightness=1.5),
            _target("XOM", FoldChannel.EQUITY, sector="energy",
                    tri_state=DTriState.ABSORB, tightness=1.8),
        )
        qs = build_quotient_space(targets)
        assert len(qs) == 3
        assert all(ec.size == 1 for ec in qs)
