"""K4 C 路径（六边同步分析）单元测试。

覆盖：圈闭合纠错（多数表决 + dissent 计数）、联合读数（方向票 + 买卖点票 +
orientation 定号）、资产排序。圈闭合机制是 L0（纯代数），可用确定性断言验证。
"""

from __future__ import annotations

import pytest

from newchan.strategy.k4_path_c import (
    ALL_EDGES,
    DERIVED_EDGES,
    EDGE_CL_USD,
    EDGE_ES_CL,
    EDGE_ES_GC,
    EDGE_ES_USD,
    EDGE_GC_CL,
    EDGE_GC_USD,
    MAIN_EDGES,
    W_L1_DIVERGENCE_BSP,
    CEdge,
    CVertex,
    EdgeReading,
    LeveledEdgeReading,
    closure_consensus,
    joint_reading,
    rank_assets,
)
from newchan.topology.config_space import WalkDirection as W


def _reading(edge: CEdge, d: W, side=None, kind=None, conf=False) -> EdgeReading:
    return EdgeReading(edge=edge, direction=d, bsp_side=side,
                       bsp_kind=kind, bsp_confirmed=conf)


def _leveled(edge: CEdge, l2: W, side=None, conf=True) -> LeveledEdgeReading:
    return LeveledEdgeReading(edge=edge, l2_direction=l2,
                              l1_bsp_side=side, l1_div_confirmed=conf)


def _flat_book(overrides: dict[CEdge, EdgeReading] | None = None) -> dict[CEdge, EdgeReading]:
    """六边全 FLAT 的基线读数，可按边覆盖。"""
    book = {e: _reading(e, W.FLAT) for e in ALL_EDGES}
    if overrides:
        book.update(overrides)
    return book


# ════════════════════════════════════════════════════════════
# K4 图结构
# ════════════════════════════════════════════════════════════


def test_edge_partition():
    """3 主边 + 3 派生边 = 6 边，主边全连 CASH，派生边全为资产三角。"""
    assert len(ALL_EDGES) == 6
    assert len(MAIN_EDGES) == 3
    assert len(DERIVED_EDGES) == 3
    for e in MAIN_EDGES:
        assert e.den is CVertex.CASH
        assert not e.is_derived
    for e in DERIVED_EDGES:
        assert CVertex.CASH not in (e.num, e.den)
        assert e.is_derived


# ════════════════════════════════════════════════════════════
# 圈闭合纠错
# ════════════════════════════════════════════════════════════


def test_consensus_all_consistent_no_dissent():
    """ES涨 GC跌 CL跌 + 派生边一致 → 三派生边 dissent=0。"""
    book = _flat_book(
        {
            EDGE_ES_USD: _reading(EDGE_ES_USD, W.UP),
            EDGE_GC_USD: _reading(EDGE_GC_USD, W.DOWN),
            EDGE_CL_USD: _reading(EDGE_CL_USD, W.DOWN),
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP),
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.UP),
            EDGE_GC_CL: _reading(EDGE_GC_CL, W.FLAT),
        }
    )
    cons = closure_consensus(book)
    assert cons[EDGE_ES_GC].consensus == 1
    assert cons[EDGE_ES_GC].dissent == 0
    assert cons[EDGE_ES_CL].consensus == 1
    assert cons[EDGE_ES_CL].dissent == 0


def test_consensus_corrects_noisy_derived_edge():
    """ES/GC 直接读数反向（DOWN），但主边(过现金)+第三资产路径支持 UP → 纠正为 +1。"""
    book = _flat_book(
        {
            EDGE_ES_USD: _reading(EDGE_ES_USD, W.UP),
            EDGE_GC_USD: _reading(EDGE_GC_USD, W.DOWN),
            EDGE_CL_USD: _reading(EDGE_CL_USD, W.DOWN),
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.DOWN),   # ← 噪声反向
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.UP),
            EDGE_GC_CL: _reading(EDGE_GC_CL, W.FLAT),
        }
    )
    c = closure_consensus(book)[EDGE_ES_GC]
    assert c.direct == -1
    assert c.via_cash == 1     # sign(UP - DOWN) = +1
    assert c.consensus == 1    # 多数表决纠正
    assert c.dissent == 1      # 一条估计（direct）与共识矛盾


def test_consensus_tie_falls_back_to_direct():
    """三估计和为 0（如 +1/-1/0）→ 回退到 direct（该边自身引擎产出优先）。"""
    # direct=+1, via_cash=-1, via_third=0 → sum=0 → consensus=direct=+1
    book = _flat_book(
        {
            EDGE_ES_USD: _reading(EDGE_ES_USD, W.DOWN),
            EDGE_GC_USD: _reading(EDGE_GC_USD, W.UP),   # via_cash(ES/GC)=sign(-1-1)=-1
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP),      # direct=+1
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.FLAT),
            EDGE_GC_CL: _reading(EDGE_GC_CL, W.FLAT),    # via_third=sign(0-0)=0
            EDGE_CL_USD: _reading(EDGE_CL_USD, W.FLAT),
        }
    )
    c = closure_consensus(book)[EDGE_ES_GC]
    assert c.direct == 1
    assert c.via_cash == -1
    assert c.via_third == 0
    assert c.consensus == 1   # 平局回退 direct


def test_consensus_via_third_uses_addition_for_es_cl():
    """ES/CL 过金路径 = sign(σ(ES/GC)+σ(GC/CL))（对数收益相加，非相减）。"""
    # ES/GC=UP, GC/CL=UP → via_third(ES/CL)=sign(1+1)=+1
    book = _flat_book(
        {
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP),
            EDGE_GC_CL: _reading(EDGE_GC_CL, W.UP),
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.DOWN),  # direct 反向
        }
    )
    c = closure_consensus(book)[EDGE_ES_CL]
    assert c.via_third == 1
    # direct=-1, via_cash=sign(0-0)=0, via_third=+1 → sum=0 → 回退 direct=-1
    assert c.consensus == -1
    assert c.via_cash == 0


# ════════════════════════════════════════════════════════════
# 联合读数
# ════════════════════════════════════════════════════════════


def test_joint_reading_only_target_incident_edges():
    """target=EQUITY 只聚合 ES-incident 三条边（ES/$, ES/GC, ES/CL）。"""
    book = _flat_book()
    jr = joint_reading(book, CVertex.EQUITY)
    edges = {c.edge for c in jr.contributions}
    assert edges == {EDGE_ES_USD, EDGE_ES_GC, EDGE_ES_CL}


def test_joint_reading_all_flat_is_flat():
    book = _flat_book()
    jr = joint_reading(book, CVertex.EQUITY)
    assert jr.score == 0.0
    assert jr.favored is W.FLAT


def test_joint_reading_three_up_edges_score_three():
    """ES-incident 三边全 UP → score=3、favored=UP。"""
    book = _flat_book(
        {
            EDGE_ES_USD: _reading(EDGE_ES_USD, W.UP),
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP),
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.UP),
        }
    )
    jr = joint_reading(book, CVertex.EQUITY)
    assert jr.score == pytest.approx(3.0)
    assert jr.favored is W.UP


def test_joint_reading_bsp_buy_adds_to_score():
    """ES/GC 给一类买点(confirmed) → equity score 在方向票之上 +1。"""
    base = _flat_book({EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP)})
    s_base = joint_reading(base, CVertex.EQUITY).score
    withbsp = _flat_book(
        {EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP, side="buy",
                                kind="type1", conf=True)}
    )
    s_bsp = joint_reading(withbsp, CVertex.EQUITY).score
    assert s_bsp == pytest.approx(s_base + 1.0)


def test_joint_reading_orientation_sign_flip():
    """目标为分母时方向票翻号：GC/CL 边 UP，对 OIL(分母)是利空。"""
    book = _flat_book({EDGE_GC_CL: _reading(EDGE_GC_CL, W.UP)})
    jr_oil = joint_reading(book, CVertex.OIL)
    # OIL 是 GC/CL 的分母 → orientation=-1 → 方向票 = -1
    contrib = next(c for c in jr_oil.contributions if c.edge is EDGE_GC_CL)
    assert contrib.orientation == -1
    assert contrib.direction_vote == pytest.approx(-1.0)


def test_joint_reading_bsp_sell_is_bearish_for_numerator():
    """ES/$ 一类卖点(confirmed) → 对 equity 利空（score 下降 1）。"""
    book = _flat_book(
        {EDGE_ES_USD: _reading(EDGE_ES_USD, W.FLAT, side="sell",
                                 kind="type1", conf=True)}
    )
    jr = joint_reading(book, CVertex.EQUITY)
    assert jr.score == pytest.approx(-1.0)
    assert jr.favored is W.DOWN


def test_candidate_bsp_half_weight_of_confirmed():
    """candidate 买卖点权重 = confirmed 的一半。"""
    conf = _flat_book(
        {EDGE_ES_USD: _reading(EDGE_ES_USD, W.FLAT, side="buy",
                                 kind="type1", conf=True)}
    )
    cand = _flat_book(
        {EDGE_ES_USD: _reading(EDGE_ES_USD, W.FLAT, side="buy",
                                 kind="type1", conf=False)}
    )
    s_conf = joint_reading(conf, CVertex.EQUITY).score
    s_cand = joint_reading(cand, CVertex.EQUITY).score
    assert s_cand == pytest.approx(s_conf * 0.5)


# ════════════════════════════════════════════════════════════
# 资产排序
# ════════════════════════════════════════════════════════════


def test_rank_assets_excludes_cash_and_orders_by_score():
    """排序含 3 个资产顶点（不含 CASH），按 score 降序。"""
    book = _flat_book(
        {
            EDGE_ES_USD: _reading(EDGE_ES_USD, W.UP),
            EDGE_ES_GC: _reading(EDGE_ES_GC, W.UP),
            EDGE_ES_CL: _reading(EDGE_ES_CL, W.UP),
            EDGE_GC_USD: _reading(EDGE_GC_USD, W.DOWN),
        }
    )
    ranked = rank_assets(book)
    assert len(ranked) == 3
    assert {j.target for j in ranked} == {
        CVertex.EQUITY, CVertex.GOLD, CVertex.OIL,
    }
    assert ranked[0].target is CVertex.EQUITY  # 最强做多
    scores = [j.score for j in ranked]
    assert scores == sorted(scores, reverse=True)


# ════════════════════════════════════════════════════════════
# 多级别读数（LeveledEdgeReading）+ 协议级别对齐
# ════════════════════════════════════════════════════════════


def test_leveled_sigma_is_l2_direction():
    """多级别 σ = L2 方向（圈闭合在此级别对齐，不混级别）。"""
    r = _leveled(EDGE_ES_USD, W.UP)
    assert r.sigma is W.UP
    assert r.sigma is r.l2_direction


def test_leveled_bsp_buy_positive_sell_negative():
    """L1 底背驰买→正利好分子；顶背驰卖→负。"""
    buy = _leveled(EDGE_ES_USD, W.FLAT, side="buy")
    sell = _leveled(EDGE_ES_USD, W.FLAT, side="sell")
    none = _leveled(EDGE_ES_USD, W.FLAT, side=None)
    assert buy.bsp_signed_vote() == pytest.approx(W_L1_DIVERGENCE_BSP)
    assert sell.bsp_signed_vote() == pytest.approx(-W_L1_DIVERGENCE_BSP)
    assert none.bsp_signed_vote() == 0.0


def test_leveled_unconfirmed_divergence_half_weight():
    """未确认背驰（candidate）权重折半（与单级别 candidate 对称）。"""
    conf = _leveled(EDGE_ES_USD, W.FLAT, side="buy", conf=True)
    cand = _leveled(EDGE_ES_USD, W.FLAT, side="buy", conf=False)
    assert cand.bsp_signed_vote() == pytest.approx(conf.bsp_signed_vote() * 0.5)


def test_leveled_closure_consensus_aligns_on_l2():
    """多级别圈闭合用 L2 σ——与单级别 EdgeReading 同 σ 时结果逐位相同（协议对齐）。"""
    # 多级别 book：L2 方向 ES↑ GC↓ CL↓，派生边噪声 ES/GC 反向
    lev = {
        EDGE_ES_USD: _leveled(EDGE_ES_USD, W.UP),
        EDGE_GC_USD: _leveled(EDGE_GC_USD, W.DOWN),
        EDGE_CL_USD: _leveled(EDGE_CL_USD, W.DOWN),
        EDGE_ES_GC: _leveled(EDGE_ES_GC, W.DOWN),   # 噪声
        EDGE_ES_CL: _leveled(EDGE_ES_CL, W.UP),
        EDGE_GC_CL: _leveled(EDGE_GC_CL, W.FLAT),
    }
    # 单级别 book：相同 σ（direction）
    single = {
        EDGE_ES_USD: _reading(EDGE_ES_USD, W.UP),
        EDGE_GC_USD: _reading(EDGE_GC_USD, W.DOWN),
        EDGE_CL_USD: _reading(EDGE_CL_USD, W.DOWN),
        EDGE_ES_GC: _reading(EDGE_ES_GC, W.DOWN),
        EDGE_ES_CL: _reading(EDGE_ES_CL, W.UP),
        EDGE_GC_CL: _reading(EDGE_GC_CL, W.FLAT),
    }
    cl = closure_consensus(lev)[EDGE_ES_GC]
    cs = closure_consensus(single)[EDGE_ES_GC]
    # 圈闭合只读 .sigma——多级别(L2)与单级别(level-0)同 σ 时纠错逐位相同
    assert (cl.direct, cl.via_cash, cl.via_third, cl.consensus, cl.dissent) == (
        cs.direct, cs.via_cash, cs.via_third, cs.consensus, cs.dissent
    )
    assert cl.consensus == 1 and cl.dissent == 1  # 噪声被纠正


def test_leveled_joint_reading_direction_plus_bsp():
    """多级别联合读数：L2 方向票 + L1 背驰买卖点票（target=EQUITY 聚合 ES-incident）。

    ES/$ L2=UP 经圈闭合传播到两条 ES-incident 派生边的共识方向（K4 冗余：
    via_cash(ES/GC)=sign(ES/$−GC/$)=+1、via_cash(ES/CL)=+1）→ 三条 ES-incident
    边方向票各 +1，叠加 ES/$ 的 L1 底背驰买点票 → score = 3 + W_L1_DIVERGENCE_BSP。
    """
    lev = {e: _leveled(e, W.FLAT) for e in ALL_EDGES}
    lev[EDGE_ES_USD] = _leveled(EDGE_ES_USD, W.UP, side="buy")  # 方向+1, 买点+1
    jr = joint_reading(lev, CVertex.EQUITY)
    assert jr.score == pytest.approx(3.0 + W_L1_DIVERGENCE_BSP)
    assert jr.favored is W.UP
    # 买点票确实进入（移除买点 → score 降 W_L1_DIVERGENCE_BSP）
    lev[EDGE_ES_USD] = _leveled(EDGE_ES_USD, W.UP, side=None)
    assert joint_reading(lev, CVertex.EQUITY).score == pytest.approx(3.0)
