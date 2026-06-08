"""递归分解树 + 两层圈闭合的单元测试（L0/L1：合成数据验证代数正确性）。

认识论标注：本测试用**合成数据**验证两层圈闭合代数与树结构约束（L0/L1）——
验证管线正确性，零信息增量，**不验证选股 alpha**（那是 L2，需真实数据回测）。
对应 testing-override.md：纯代数 L0 模块，测试目的是确认代数无误。

两层（设计文档 §3）：
  - Layer A: a0 精确闭合（数据质量门，等式，残差应≈0）
  - Layer B: 走势状态软闭合（兼容性，不强制级别对齐；级别加权）
"""

from __future__ import annotations

import math

import pytest

from newchan.topology.config_space import WalkDirection
from newchan.topology.decomposition_tree import (
    THETA_INCOMPAT,
    ComponentReading,
    ComponentRole,
    DecompositionNode,
    DecompositionTree,
    SelectionRanker,
    TreeLayer,
    VerticalClosure,
    a0_precision_residual,
    level_confidence,
    sedimentation_signal,
)
from newchan.topology.graph import Vertex

UP = WalkDirection.UP
FLAT = WalkDirection.FLAT
DOWN = WalkDirection.DOWN


# ═══════════════════════════════════════════════════════════════
# DecompositionTree 结构约束（L0 定义检查）
# ═══════════════════════════════════════════════════════════════


def _leaf(label: str, symbol: str, weight: float) -> DecompositionNode:
    return DecompositionNode(
        label=label, layer=TreeLayer.INSTRUMENT, weight=weight, symbol=symbol
    )


def _sample_tree() -> DecompositionTree:
    soxx = _leaf("SOXX", "SOXX", 0.6)
    spy = _leaf("SPY", "SPY", 0.4)
    p_node = DecompositionNode(
        label="P", layer=TreeLayer.VERTEX_DECOMPOSITION, weight=1.0,
        vertex=Vertex.P, symbol="ES", children=(soxx, spy),
    )
    us = DecompositionNode(
        label="US", layer=TreeLayer.NATIONAL_K4, weight=1.0, children=(p_node,)
    )
    return DecompositionTree(us)


def test_tree_builds_and_validates():
    assert _sample_tree().root.label == "US"


def test_tree_layer_query():
    tree = _sample_tree()
    assert len(tree.nodes_at_layer(TreeLayer.VERTEX_DECOMPOSITION)) == 1
    assert len(tree.nodes_at_layer(TreeLayer.INSTRUMENT)) == 2


def test_tree_leaves():
    assert {n.symbol for n in _sample_tree().leaves()} == {"SOXX", "SPY"}


def test_tree_children_of():
    assert {n.label for n in _sample_tree().children_of("P")} == {"SOXX", "SPY"}


def test_tree_children_of_missing_raises():
    with pytest.raises(KeyError):
        _sample_tree().children_of("NONEXISTENT")


def test_tree_weight_not_normalized_raises():
    bad = DecompositionNode(
        label="P", layer=TreeLayer.VERTEX_DECOMPOSITION, weight=1.0, vertex=Vertex.P,
        children=(_leaf("A", "A", 0.6), _leaf("B", "B", 0.6)),
    )
    with pytest.raises(ValueError, match="权重和"):
        DecompositionTree(bad)


def test_tree_leaf_without_symbol_raises():
    bad_leaf = DecompositionNode(label="A", layer=TreeLayer.INSTRUMENT, weight=1.0)
    root = DecompositionNode(
        label="P", layer=TreeLayer.VERTEX_DECOMPOSITION, weight=1.0,
        vertex=Vertex.P, children=(bad_leaf,),
    )
    with pytest.raises(ValueError, match="symbol"):
        DecompositionTree(root)


def test_tree_vertex_node_without_vertex_raises():
    bad = DecompositionNode(
        label="P", layer=TreeLayer.VERTEX_DECOMPOSITION, weight=1.0,
        children=(_leaf("A", "A", 1.0),),
    )
    with pytest.raises(ValueError, match="vertex"):
        DecompositionTree(bad)


# ═══════════════════════════════════════════════════════════════
# level_confidence（§3.2.2：级别清晰度，与 σ 正交）
# ═══════════════════════════════════════════════════════════════


def test_level_confidence_monotone():
    assert level_confidence(0) == 0.0
    assert level_confidence(1) == pytest.approx(0.5)
    assert level_confidence(2) == pytest.approx(0.75)
    assert level_confidence(3) == pytest.approx(0.875)
    # 单调递增，渐近 1。
    assert level_confidence(1) < level_confidence(2) < level_confidence(3) < 1.0


# ═══════════════════════════════════════════════════════════════
# Layer A：a0 精确闭合（数据质量门，L1——等式残差≈0）
# ═══════════════════════════════════════════════════════════════


def test_a0_exact_replication_clean():
    """成分加权精确复制宏观 → 残差≈0，is_clean=True。"""
    s1 = [10.0, 11.0, 12.0]
    s2 = [20.0, 22.0, 18.0]
    w1, w2 = 0.5, 0.5
    macro = [w1 * a + w2 * b for a, b in zip(s1, s2)]
    r = a0_precision_residual(macro, ((w1, s1), (w2, s2)))
    assert r.is_clean is True
    assert r.max_abs_residual == pytest.approx(0.0, abs=1e-9)
    assert r.n_bars == 3


def test_a0_replication_failure_not_clean():
    """成分无法复制宏观（如 PoC 用 ETF 代理非真成分）→ 残差大，is_clean=False。"""
    s1 = [10.0, 11.0, 12.0]
    s2 = [20.0, 22.0, 18.0]
    macro = [100.0, 130.0, 90.0]  # 与加权和不符
    r = a0_precision_residual(macro, ((0.5, s1), (0.5, s2)))
    assert r.is_clean is False
    assert r.max_abs_residual > 1e-3
    # 残差 = |log(macro) − log(synth)|，手算首 bar 校验。
    synth0 = 0.5 * 10.0 + 0.5 * 20.0
    assert r.max_abs_residual >= abs(math.log(100.0) - math.log(synth0)) - 1e-9


def test_a0_weights_not_normalized_raises():
    with pytest.raises(ValueError, match="权重和"):
        a0_precision_residual([1.0], ((0.5, [1.0]), (0.3, [1.0])))


def test_a0_length_mismatch_raises():
    with pytest.raises(ValueError, match="不对齐"):
        a0_precision_residual([1.0, 2.0], ((1.0, [1.0]),))


def test_a0_empty_components_raises():
    with pytest.raises(ValueError, match="不能为空"):
        a0_precision_residual([1.0], ())


# ═══════════════════════════════════════════════════════════════
# Layer B：走势状态软闭合（兼容性 + 级别加权，L0）
# ═══════════════════════════════════════════════════════════════


def test_closure_synth_level_weighted():
    """synth 用有效权重 = market_weight × conf(level)。"""
    comps = (
        ComponentReading("A", 0.5, UP, level=2),  # eff=0.5*0.75=0.375
        ComponentReading("B", 0.5, UP, level=2),  # eff=0.375
    )
    r = VerticalClosure.check(UP, comps)
    assert r.synth == pytest.approx(0.75)   # 0.375+0.375
    by = {c.symbol: c for c in r.contributions}
    assert by["A"].eff_weight == pytest.approx(0.375)
    assert by["A"].contribution == pytest.approx(0.375)  # eff*(1*1)+0


def test_closure_high_level_opposite_incompatible():
    """高级别反向边 → 不兼容（eff 大，incompatibility > θ）。"""
    comps = (
        ComponentReading("up", 0.4, UP, level=3),
        ComponentReading("down_hi", 0.6, DOWN, level=3),  # eff=0.6*0.875=0.525 > θ=0.5
    )
    r = VerticalClosure.check(UP, comps, theta=THETA_INCOMPAT)
    assert r.incompatibility == pytest.approx(0.525)
    assert r.compatible is False


def test_closure_low_level_opposite_still_compatible():
    """同权重反向边但**低级别** → 仍兼容（eff 小）。级别不对齐是信息（§3.2.1）。"""
    comps = (
        ComponentReading("up", 0.4, UP, level=3),
        ComponentReading("down_lo", 0.6, DOWN, level=1),  # eff=0.6*0.5=0.3 ≤ θ=0.5
    )
    r = VerticalClosure.check(UP, comps, theta=THETA_INCOMPAT)
    assert r.incompatibility == pytest.approx(0.3)
    assert r.compatible is True   # 低级别反向不否定宏观


def test_closure_flat_not_incompatible():
    """成分盘整不计入 incompatibility（不否定宏观，兼容）。"""
    comps = (
        ComponentReading("up", 0.5, UP, level=3),
        ComponentReading("flat", 0.5, FLAT, level=3),
    )
    r = VerticalClosure.check(UP, comps)
    assert r.incompatibility == pytest.approx(0.0)
    assert r.compatible is True


def test_closure_concentration_uses_market_weight():
    """concentration 用市值权重（结构维度，与级别正交）。"""
    comps = (
        ComponentReading("BIG", 0.70, UP, level=2),
        ComponentReading("s1", 0.15, DOWN, level=2),
        ComponentReading("s2", 0.15, DOWN, level=2),
    )
    r = VerticalClosure.check(UP, comps)
    assert r.weighted_participation == pytest.approx(0.70)   # 市值，非 eff
    assert r.count_participation == pytest.approx(1 / 3)
    assert r.concentration > 0.3   # 趋势集中在龙头 → 脆弱


def test_closure_residuals():
    comps = (
        ComponentReading("aligned", 0.34, UP, level=2),
        ComponentReading("flat", 0.33, FLAT, level=2),
        ComponentReading("opp", 0.33, DOWN, level=2),
    )
    r = VerticalClosure.check(UP, comps)
    by = {c.symbol: c for c in r.contributions}
    assert by["aligned"].residual == 0
    assert by["flat"].residual == 1
    assert by["opp"].residual == 2


def test_closure_contribution_with_bsp():
    """contribution = eff_weight·(σ·σ_macro) + λ·bsp。"""
    comps = (
        ComponentReading("d", 0.5, UP, level=2, bsp_vote=0.0),  # eff=0.375
        ComponentReading("e", 0.5, UP, level=2, bsp_vote=0.8),
    )
    r = VerticalClosure.check(UP, comps, w_bsp=1.0)
    by = {c.symbol: c for c in r.contributions}
    assert by["d"].contribution == pytest.approx(0.375)
    assert by["e"].contribution == pytest.approx(0.375 + 0.8)


def test_closure_macro_flat_not_directional():
    r = VerticalClosure.check(FLAT, (ComponentReading("a", 1.0, UP, level=2),))
    assert r.macro_directional is False


def test_closure_empty_raises():
    with pytest.raises(ValueError, match="不能为空"):
        VerticalClosure.check(UP, ())


def test_closure_weights_not_normalized_raises():
    comps = (ComponentReading("a", 0.5, UP), ComponentReading("b", 0.3, UP))
    with pytest.raises(ValueError, match="权重和"):
        VerticalClosure.check(UP, comps)


# ═══════════════════════════════════════════════════════════════
# SelectionRanker 排序 + 角色分类（L0；alpha 属 L2 未验证）
# ═══════════════════════════════════════════════════════════════


def test_ranker_sorts_descending():
    comps = (
        ComponentReading("low", 0.2, DOWN, level=2),
        ComponentReading("high", 0.5, UP, level=2),
        ComponentReading("mid", 0.3, FLAT, level=2),
    )
    ranking = SelectionRanker.rank(VerticalClosure.check(UP, comps))
    contribs = [e.contribution for e in ranking.entries]
    assert contribs == sorted(contribs, reverse=True)
    assert ranking.entries[0].symbol == "high"


def test_ranker_roles():
    comps = (
        ComponentReading("driver", 0.4, UP, level=2),
        ComponentReading("laggard", 0.3, FLAT, level=2),
        ComponentReading("contra", 0.3, DOWN, level=2),
    )
    ranking = SelectionRanker.rank(VerticalClosure.check(UP, comps))
    roles = {e.symbol: e.role for e in ranking.entries}
    assert roles["driver"] is ComponentRole.DRIVER
    assert roles["laggard"] is ComponentRole.LAGGARD
    assert roles["contra"] is ComponentRole.CONTRARIAN


def test_ranker_higher_level_driver_ranks_first():
    """同向同市值，高级别成分贡献更大 → 排序靠前（级别清晰度=信号权重）。"""
    comps = (
        ComponentReading("lo", 0.5, UP, level=1),  # eff=0.25
        ComponentReading("hi", 0.5, UP, level=3),  # eff=0.4375
    )
    ranking = SelectionRanker.rank(VerticalClosure.check(UP, comps))
    assert ranking.entries[0].symbol == "hi"   # 高级别清晰 → 龙头排前


def test_ranker_macro_flat_empty():
    ranking = SelectionRanker.rank(
        VerticalClosure.check(FLAT, (ComponentReading("a", 1.0, UP, level=2),))
    )
    assert ranking.entries == ()


def test_ranker_macro_down_driver():
    comps = (
        ComponentReading("short_driver", 0.6, DOWN, level=2),
        ComponentReading("resist", 0.4, UP, level=2),
    )
    ranking = SelectionRanker.rank(VerticalClosure.check(DOWN, comps))
    roles = {e.symbol: e.role for e in ranking.entries}
    assert roles["short_driver"] is ComponentRole.DRIVER
    assert roles["resist"] is ComponentRole.CONTRARIAN


# ═══════════════════════════════════════════════════════════════
# 资本病理学：纵向沉没检测（§8.3，L0；操作指导属 L2 未验证）
# ═══════════════════════════════════════════════════════════════


def test_sedimentation_only_flat_components():
    """仅 σ=FLAT 成分计入沉没；有向成分不沉没。"""
    comps = (
        ComponentReading("alive", 0.5, UP, level=1),
        ComponentReading("dead", 0.5, FLAT, level=1),
    )
    r = sedimentation_signal(VerticalClosure.check(UP, comps))
    syms = {e.symbol for e in r.entries}
    assert syms == {"dead"}                       # 只有 FLAT 的沉没
    assert r.entries[0].score == pytest.approx(0.5 * (1 - 0.5))  # 0.25


def test_sedimentation_low_level_is_deadwater():
    """低级别 FLAT（死水）沉没度 > 高级别 FLAT（健康中枢）。"""
    comps = (
        ComponentReading("deadwater", 0.5, FLAT, level=1),  # conf=0.5 → score=0.25
        ComponentReading("healthy", 0.5, FLAT, level=3),    # conf=0.875 → score=0.0625
    )
    r = sedimentation_signal(VerticalClosure.check(FLAT, comps))
    by = {e.symbol: e.score for e in r.entries}
    assert by["deadwater"] > by["healthy"]        # 死水沉没度更高
    assert r.entries[0].symbol == "deadwater"     # 降序首位
    assert by["deadwater"] == pytest.approx(0.25)
    assert by["healthy"] == pytest.approx(0.0625)


def test_sedimentation_total():
    comps = (
        ComponentReading("a", 0.4, FLAT, level=1),  # 0.4*0.5=0.20
        ComponentReading("b", 0.3, FLAT, level=1),  # 0.3*0.5=0.15
        ComponentReading("c", 0.3, UP, level=1),    # 不沉没
    )
    r = sedimentation_signal(VerticalClosure.check(UP, comps))
    assert r.total == pytest.approx(0.35)


def test_sedimentation_none_when_all_directional():
    comps = (
        ComponentReading("a", 0.5, UP, level=2),
        ComponentReading("b", 0.5, DOWN, level=2),
    )
    r = sedimentation_signal(VerticalClosure.check(UP, comps))
    assert r.entries == ()
    assert r.total == pytest.approx(0.0)
