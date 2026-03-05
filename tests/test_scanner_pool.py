"""标的池生成器测试 -- 帕萨卡利亚条件设定链（层0→层1→层2）。

验证：
1. 层1→层2 条件设定：K4 配置 → 极性 → 目标矩阵
2. 折叠等价类构造 + 商空间排序
3. 完整管线：配置 + 候选快照 → 排序后的等价类 + 代表元
4. 边界条件：空快照、无收敛标的、全退化

认识论等级：L0（从352号定义直接推导）。
谱系引用：352号多标的扫描器设计、350号收敛紧度、292号折叠拓扑。
"""

from __future__ import annotations

import pytest

from newchan.topology.config_space import Configuration, WalkDirection
from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FoldChannel,
    TargetAttributes,
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
# 层1→层2 条件设定：目标矩阵
# ═══════════════════════════════════════════════════════════════


class TestLayer1TargetMatrix:
    """验证 K4 配置 → 极性 → 资产类别映射。"""

    def test_risk_on_returns_equity(self) -> None:
        """极性 > 0 → EQUITY 类。"""
        cfg = _config(1, 1, 1)  # polarity = +3
        matrix = layer1_target_matrix(cfg)
        assert "equity" in matrix

    def test_risk_off_returns_rate(self) -> None:
        """极性 < 0 → RATE 类。"""
        cfg = _config(-1, -1, -1)  # polarity = -3
        matrix = layer1_target_matrix(cfg)
        assert "rate" in matrix

    def test_neutral_returns_both(self) -> None:
        """极性 == 0 → EQUITY + RATE。"""
        cfg = _config(1, 0, -1)  # polarity = 0
        matrix = layer1_target_matrix(cfg)
        assert "equity" in matrix
        assert "rate" in matrix

    def test_gold_condition(self) -> None:
        """sigma_e UP 且 sigma_c UP → 包含 AU。"""
        cfg = _config(1, 1, -1)  # E=UP, C=UP, polarity=+1
        matrix = layer1_target_matrix(cfg)
        assert "au" in matrix

    def test_no_gold_when_condition_not_met(self) -> None:
        """sigma_e 或 sigma_c 不为 UP → 不包含 AU。"""
        cfg = _config(1, 0, 1)  # C != UP
        matrix = layer1_target_matrix(cfg)
        assert "au" not in matrix


# ═══════════════════════════════════════════════════════════════
# 层2：折叠等价类构造
# ═══════════════════════════════════════════════════════════════


class TestLayer2EquivalenceClasses:
    """验证从 TargetAttributes 集到排序后等价类的构造。"""

    def test_empty_targets(self) -> None:
        """空标的集 → 空等价类。"""
        result = layer2_build_equivalence_classes(())
        assert result == ()

    def test_single_target(self) -> None:
        """单标的 → 单等价类。"""
        t = _target("GLD", FoldChannel.AU, tightness=3.0)
        result = layer2_build_equivalence_classes((t,))
        assert len(result) == 1
        assert result[0].representative.symbol == "GLD"

    def test_au_targets_grouped(self) -> None:
        """同一折叠通道的标的归入同一等价类。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, liquidity=500.0),
            _target("GC=F", FoldChannel.AU, tightness=2.5, liquidity=300.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 1
        assert result[0].size == 2
        assert result[0].representative.symbol == "GLD"  # T=3.0 > 2.5

    def test_different_channels_separate(self) -> None:
        """不同折叠通道的标的分入不同等价类。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),
            _target("USO", FoldChannel.OIL, tightness=2.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 2

    def test_ordering_by_rank(self) -> None:
        """等价类按 rank 降序排列。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),      # rank = 2.0 * 4 = 8
            _target("USO", FoldChannel.OIL, tightness=3.0),     # rank = 3.0 * 3 = 9
            _target("TLT", FoldChannel.BOND, tightness=1.0),    # rank = 1.0 * 2 = 2
        )
        result = layer2_build_equivalence_classes(targets)
        ranks = [ec.rank for ec in result]
        assert ranks == sorted(ranks, reverse=True)

    def test_zero_tightness_excluded(self) -> None:
        """T=0 的标的被二值筛选排除（不进入等价类构造）。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0),
            _target("BAD", FoldChannel.AU, tightness=0.0),
        )
        result = layer2_build_equivalence_classes(targets)
        assert len(result) == 1
        assert result[0].size == 1
        assert result[0].representative.symbol == "GLD"


# ═══════════════════════════════════════════════════════════════
# 完整管线：build_scanner_pool
# ═══════════════════════════════════════════════════════════════


class TestBuildScannerPool:
    """验证完整的三阶段管线。"""

    def test_full_pipeline(self) -> None:
        """完整管线：配置 + 标的属性 → 排序后的等价类。"""
        cfg = _config(1, 1, 0)  # polarity = +2, E=UP, C=UP → equity + au
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0, liquidity=500.0),
            _target("AAPL", FoldChannel.EQUITY, sector="tech",
                    tri_state=DTriState.RETAIN, tightness=2.0, liquidity=800.0),
        )
        result = build_scanner_pool(cfg, targets)
        assert isinstance(result, ScannerPoolResult)
        assert len(result.equivalence_classes) >= 1
        assert result.top_representative is not None

    def test_empty_targets(self) -> None:
        """空标的集 → top_representative 为 None。"""
        cfg = _config(1, 0, 0)
        result = build_scanner_pool(cfg, ())
        assert result.top_representative is None
        assert result.equivalence_classes == ()

    def test_all_zero_tightness(self) -> None:
        """所有标的 T=0 → 全部被筛掉。"""
        cfg = _config(1, 0, 0)
        targets = (
            _target("A", FoldChannel.AU, tightness=0.0),
            _target("B", FoldChannel.OIL, tightness=0.0),
        )
        result = build_scanner_pool(cfg, targets)
        assert result.top_representative is None

    def test_result_contains_config(self) -> None:
        """结果携带原始配置和极性。"""
        cfg = _config(1, 1, -1)
        result = build_scanner_pool(cfg, ())
        assert result.config is cfg
        assert result.polarity == 1

    def test_target_matrix_in_result(self) -> None:
        """结果携带层1确定的目标矩阵类别。"""
        cfg = _config(1, 1, 0)
        result = build_scanner_pool(cfg, ())
        assert "equity" in result.target_matrix
        assert "au" in result.target_matrix

    def test_top_representative_is_highest_rank(self) -> None:
        """top_representative 是排名第一的等价类的代表元。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=2.0),      # rank = 8
            _target("USO", FoldChannel.OIL, tightness=1.0),     # rank = 3
        )
        cfg = _config(1, 1, 0)
        result = build_scanner_pool(cfg, targets)
        assert result.top_representative is not None
        assert result.top_representative.symbol == "GLD"
