"""标的池生成工程测试 — 帕萨卡利亚条件设定链（层0→层1→层2）。

工位：scanner-pool-generation (engineering)
来源：352号下游推论1a + §2

验证内容：
1. 层1→层2 条件设定：K4 配置 → 极性 → 目标矩阵（资产类别标签集合）
2. 层2 折叠等价类构造：二值筛选 + 商空间 F/~ 构造
3. 完整三阶段管线：build_scanner_pool 端到端
4. 帕萨卡利亚特有约束：条件设定链的层间一致性
5. 退化条件：商空间退化为扁平排序的充要条件

认识论等级：L0（从352号定义直接推导）。
谱系引用：352号多标的扫描器设计、350号收敛紧度、265号帕萨卡利亚、292号折叠拓扑。
"""

from __future__ import annotations

import pytest

from newchan.topology.config_space import Configuration, WalkDirection, polarity_index
from newchan.trading.fold_equivalence import (
    FOLD_WEIGHT,
    DTriState,
    EquivalenceClass,
    FoldChannel,
    TargetAttributes,
    build_quotient_space,
)
from newchan.trading.scanner_pool import (
    ScannerPoolResult,
    build_scanner_pool,
    layer1_target_matrix,
    layer2_build_equivalence_classes,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


def _config(e: int, c: int, r: int) -> Configuration:
    """快速构造 Configuration。"""
    return Configuration(
        sigma_e=WalkDirection(e),
        sigma_c=WalkDirection(c),
        sigma_r=WalkDirection(r),
    )


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
# 阶段1：层1→层2 帕萨卡利亚条件设定链
# ═══════════════════════════════════════════════════════════════


class TestPassacagliaConditionChain:
    """265号帕萨卡利亚三层条件设定链在标的池生成中的操作化。"""

    def test_risk_on_equity_only(self) -> None:
        """极性 > 0（risk-on）→ 资产类别 = equity。"""
        cfg = _config(1, 0, 0)  # polarity = +1
        matrix = layer1_target_matrix(cfg)
        assert matrix == frozenset({"equity"})

    def test_risk_off_rate_only(self) -> None:
        """极性 < 0（risk-off）→ 资产类别 = rate。"""
        cfg = _config(-1, -1, 0)  # polarity = -2
        matrix = layer1_target_matrix(cfg)
        assert matrix == frozenset({"rate"})

    def test_neutral_both_classes(self) -> None:
        """极性 == 0（中性）→ equity + rate（K4不筛选，层2独立生效）。"""
        cfg = _config(1, 0, -1)  # polarity = 0
        matrix = layer1_target_matrix(cfg)
        assert matrix == frozenset({"equity", "rate"})

    def test_gold_augmentation_requires_both_up(self) -> None:
        """sigma_e=UP 且 sigma_c=UP → 额外加入 au。"""
        cfg = _config(1, 1, -1)  # E=UP, C=UP, polarity=+1
        matrix = layer1_target_matrix(cfg)
        assert "au" in matrix
        assert "equity" in matrix

    def test_no_gold_if_sigma_c_not_up(self) -> None:
        """sigma_c != UP → 不包含 au。"""
        cfg = _config(1, 0, 0)  # C=FLAT
        matrix = layer1_target_matrix(cfg)
        assert "au" not in matrix

    def test_no_gold_if_sigma_e_not_up(self) -> None:
        """sigma_e != UP → 不包含 au。"""
        cfg = _config(0, 1, 0)  # E=FLAT
        matrix = layer1_target_matrix(cfg)
        assert "au" not in matrix

    def test_max_risk_on_with_gold(self) -> None:
        """(+,+,+) → polarity=3, equity + au。"""
        cfg = _config(1, 1, 1)
        matrix = layer1_target_matrix(cfg)
        assert "equity" in matrix
        assert "au" in matrix
        assert "rate" not in matrix

    def test_max_risk_off(self) -> None:
        """(-,-,-) → polarity=-3, rate only。"""
        cfg = _config(-1, -1, -1)
        matrix = layer1_target_matrix(cfg)
        assert matrix == frozenset({"rate"})

    def test_all_flat_neutral(self) -> None:
        """(0,0,0) → polarity=0, equity + rate。"""
        cfg = _config(0, 0, 0)
        matrix = layer1_target_matrix(cfg)
        assert matrix == frozenset({"equity", "rate"})

    def test_polarity_consistency(self) -> None:
        """layer1_target_matrix 的极性计算与 polarity_index 一致。"""
        for e in (-1, 0, 1):
            for c in (-1, 0, 1):
                for r in (-1, 0, 1):
                    cfg = _config(e, c, r)
                    pol = polarity_index(cfg)
                    matrix = layer1_target_matrix(cfg)
                    if pol > 0:
                        assert "equity" in matrix
                    elif pol < 0:
                        assert "rate" in matrix
                    else:
                        assert "equity" in matrix and "rate" in matrix


# ═══════════════════════════════════════════════════════════════
# 阶段2：折叠等价类构造（二值筛选 + 商空间）
# ═══════════════════════════════════════════════════════════════


class TestLayer2EquivalenceConstruction:
    """从 TargetAttributes 到商空间 F/~ 的完整构造。"""

    def test_binary_filter_excludes_zero_tightness(self) -> None:
        """T=0 的标的在二值筛选阶段被排除。"""
        targets = (
            _target("A", FoldChannel.AU, tightness=2.0),
            _target("B", FoldChannel.AU, tightness=0.0),
            _target("C", FoldChannel.OIL, tightness=0.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 1
        assert result[0].size == 1

    def test_empty_after_filter(self) -> None:
        """全部 T=0 → 空商空间。"""
        targets = (
            _target("A", FoldChannel.AU, tightness=0.0),
            _target("B", FoldChannel.OIL, tightness=0.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert result == ()

    def test_fold_merge_au_channel(self) -> None:
        """同 AU 通道的标的合并为一个等价类。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, liquidity=500.0),
            _target("GC=F", FoldChannel.AU, tightness=2.5, liquidity=300.0),
            _target("NEM", FoldChannel.AU, tightness=2.0, liquidity=200.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 1
        assert result[0].size == 3
        assert result[0].representative.symbol == "GLD"  # T=3.0 最大

    def test_equity_same_sector_same_tristate_merge(self) -> None:
        """同板块 + 同三态 → 合并。"""
        targets = (
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0, liquidity=1000.0),
            _target("MSFT", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=1.5, liquidity=900.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 1
        assert result[0].size == 2

    def test_equity_different_tristate_separate(self) -> None:
        """同板块但三态不一致 → 不合并。"""
        targets = (
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0),
            _target("GOOG", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.AMPLIFY, tightness=2.5),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 2

    def test_equity_different_sector_separate(self) -> None:
        """不同板块 → 不合并。"""
        targets = (
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0),
            _target("XLF", FoldChannel.EQUITY, sector="finance",
                    tri_state=DTriState.RETAIN, tightness=2.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 2

    def test_rank_ordering(self) -> None:
        """等价类按 rank = T * W 降序。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),      # rank = 2.0 * 4 = 8
            _target("USO", FoldChannel.OIL, tightness=3.0),     # rank = 3.0 * 3 = 9
            _target("TLT", FoldChannel.BOND, tightness=1.5),    # rank = 1.5 * 2 = 3
        )
        result = layer2_build_equivalence_classes(targets)
        ranks = [ec.rank for ec in result]
        assert ranks == sorted(ranks, reverse=True)
        assert result[0].fold_channel is FoldChannel.OIL  # rank 9
        assert result[1].fold_channel is FoldChannel.AU   # rank 8


# ═══════════════════════════════════════════════════════════════
# 阶段3：完整管线 build_scanner_pool
# ═══════════════════════════════════════════════════════════════


class TestFullPipeline:
    """三阶段完整管线。"""

    def test_pipeline_basic(self) -> None:
        """配置 + 标的 → 排序后等价类 + 代表元。"""
        cfg = _config(1, 1, 0)  # polarity = +2, equity + au
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, liquidity=500.0),
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0, liquidity=800.0),
        )
        result = build_scanner_pool(cfg, targets)
        assert isinstance(result, ScannerPoolResult)
        assert result.top_representative is not None
        assert result.top_representative.symbol == "GLD"  # rank = 3*4=12 > 2*1=2

    def test_pipeline_empty_targets(self) -> None:
        """无标的 → top_representative = None。"""
        result = build_scanner_pool(_config(1, 0, 0), ())
        assert result.top_representative is None
        assert result.equivalence_classes == ()

    def test_pipeline_carries_config(self) -> None:
        """结果携带原始配置和极性。"""
        cfg = _config(1, 1, -1)  # polarity = +1
        result = build_scanner_pool(cfg, ())
        assert result.config is cfg
        assert result.polarity == 1

    def test_pipeline_carries_target_matrix(self) -> None:
        """结果携带层1确定的目标矩阵。"""
        cfg = _config(1, 1, 0)  # polarity=+2, E=UP, C=UP → equity + au
        result = build_scanner_pool(cfg, ())
        assert "equity" in result.target_matrix
        assert "au" in result.target_matrix

    def test_pipeline_all_zero_tightness(self) -> None:
        """全部 T=0 → 空等价类 + None 代表元。"""
        cfg = _config(1, 0, 0)
        targets = (
            _target("A", FoldChannel.AU, tightness=0.0),
            _target("B", FoldChannel.OIL, tightness=0.0),
        )
        result = build_scanner_pool(cfg, targets)
        assert result.top_representative is None
        assert result.equivalence_classes == ()

    def test_top_representative_is_first_class_rep(self) -> None:
        """top_representative 严格等于排名第一的等价类的代表元。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),    # rank = 8
            _target("USO", FoldChannel.OIL, tightness=3.0),   # rank = 9
        )
        result = build_scanner_pool(_config(1, 1, 0), targets)
        assert result.top_representative is not None
        assert result.top_representative is result.equivalence_classes[0].representative
        assert result.top_representative.symbol == "USO"  # rank 9 > 8


# ═══════════════════════════════════════════════════════════════
# 帕萨卡利亚条件设定链特有约束
# ═══════════════════════════════════════════════════════════════


class TestPassacagliaConstraints:
    """265号§6.3：结构完备性优先于信号质量。"""

    def test_all_27_configs_produce_valid_matrix(self) -> None:
        """所有 27 种 K4 配置（3^3）都产生非空目标矩阵。"""
        for e in (-1, 0, 1):
            for c in (-1, 0, 1):
                for r in (-1, 0, 1):
                    cfg = _config(e, c, r)
                    matrix = layer1_target_matrix(cfg)
                    assert len(matrix) >= 1, f"Empty matrix for ({e},{c},{r})"

    def test_gold_only_with_risk_on_and_both_up(self) -> None:
        """au 只在 sigma_e=UP 且 sigma_c=UP 时出现。"""
        for e in (-1, 0, 1):
            for c in (-1, 0, 1):
                for r in (-1, 0, 1):
                    cfg = _config(e, c, r)
                    matrix = layer1_target_matrix(cfg)
                    if e == 1 and c == 1:
                        assert "au" in matrix, f"Missing au for ({e},{c},{r})"
                    else:
                        assert "au" not in matrix, f"Unexpected au for ({e},{c},{r})"

    def test_layer1_matrix_is_frozenset(self) -> None:
        """目标矩阵是 frozenset（不可变）。"""
        matrix = layer1_target_matrix(_config(1, 0, 0))
        assert isinstance(matrix, frozenset)


# ═══════════════════════════════════════════════════════════════
# 退化条件（352号边界条件1）
# ═══════════════════════════════════════════════════════════════


class TestDegenerationToFlatSort:
    """352号边界条件1：无折叠关系时退化为扁平排序。"""

    def test_all_singletons_equals_flat_rank(self) -> None:
        """每个标的独立成类 → rank 排序等价于 T*W 扁平排序。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),     # rank = 8
            _target("USO", FoldChannel.OIL, tightness=1.5),    # rank = 4.5
            _target("TLT", FoldChannel.BOND, tightness=3.0),   # rank = 6
            _target("XLRE", FoldChannel.RE, tightness=1.0),    # rank = 1
        )
        result = build_scanner_pool(_config(1, 1, 0), targets)
        # 全部单元素
        for ec in result.equivalence_classes:
            assert ec.size == 1

        # 排序：rank 8, 6, 4.5, 1 → GLD, TLT, USO, XLRE
        symbols = [ec.representative.symbol for ec in result.equivalence_classes]
        assert symbols == ["GLD", "TLT", "USO", "XLRE"]

    def test_weight_dominance_over_tightness(self) -> None:
        """W 权重可以改变排序（商空间的核心价值）。

        中金黄金 T=0.8, W=4, rank=3.2 排在万科A T=1.8, W=1, rank=1.8 之前。
        这正是 L2 报告的核心发现。
        """
        targets = (
            _target("中金黄金", FoldChannel.AU, tightness=0.8),     # rank = 3.2
            _target("万科A", FoldChannel.RE, tightness=1.8),       # rank = 1.8
        )
        result = build_scanner_pool(_config(1, 1, 0), targets)
        assert result.top_representative is not None
        assert result.top_representative.symbol == "中金黄金"
