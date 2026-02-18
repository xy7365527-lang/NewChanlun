"""资本流转方向推断（capital_flow）测试。

概念溯源：[旧缠论:隐含] 比价走势语义（从第9课"资金流向"推出）
规范引用：ratio_relation_v1.md §1.2 语义映射表
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.equivalence import EquivalencePair
from newchan.capital_flow import (
    FlowDirection,
    StrokeFlow,
    strokes_to_flows,
)


# ── 测试数据工厂 ─────────────────────────────────────────


def _pair(sym_a: str = "GLD", sym_b: str = "SLV") -> EquivalencePair:
    return EquivalencePair(sym_a=sym_a, sym_b=sym_b)


def _stroke(
    i0: int,
    i1: int,
    direction: str,
    p0: float,
    p1: float,
    confirmed: bool = True,
) -> Stroke:
    """构造测试用 Stroke（high/low 自动填充）。"""
    high = max(p0, p1)
    low = min(p0, p1)
    return Stroke(
        i0=i0,
        i1=i1,
        direction=direction,  # type: ignore[arg-type]
        high=high,
        low=low,
        p0=p0,
        p1=p1,
        confirmed=confirmed,
    )


# ── FlowDirection 枚举 ──────────────────────────────────


class TestFlowDirection:
    def test_values(self):
        assert FlowDirection.A_TO_B.value == "A→B"
        assert FlowDirection.B_TO_A.value == "B→A"
        assert FlowDirection.EQUILIBRIUM.value == "均衡"

    def test_members_count(self):
        assert len(FlowDirection) == 3


# ── StrokeFlow 不可变性 ─────────────────────────────────


class TestStrokeFlowImmutable:
    def test_frozen(self):
        sf = StrokeFlow(
            stroke_index=0,
            direction=FlowDirection.A_TO_B,
            sym_from="GLD",
            sym_to="SLV",
            magnitude=0.05,
        )
        with pytest.raises(AttributeError):
            sf.direction = FlowDirection.B_TO_A  # type: ignore[misc]

    def test_fields(self):
        sf = StrokeFlow(
            stroke_index=3,
            direction=FlowDirection.B_TO_A,
            sym_from="SLV",
            sym_to="GLD",
            magnitude=0.10,
        )
        assert sf.stroke_index == 3
        assert sf.direction == FlowDirection.B_TO_A
        assert sf.sym_from == "SLV"
        assert sf.sym_to == "GLD"
        assert sf.magnitude == pytest.approx(0.10)


# ── strokes_to_flows 核心映射 ────────────────────────────


class TestStrokesToFlows:
    """ratio_relation_v1.md §1.2 语义映射表的验证。"""

    def test_up_stroke_means_b_to_a(self):
        """比价一笔向上 → 资本从 B 流向 A。"""
        pair = _pair("GLD", "SLV")
        strokes = [_stroke(0, 5, "up", 1.50, 1.65)]
        flows = strokes_to_flows(pair, strokes)

        assert len(flows) == 1
        f = flows[0]
        assert f.direction == FlowDirection.B_TO_A
        assert f.sym_from == "SLV"
        assert f.sym_to == "GLD"

    def test_down_stroke_means_a_to_b(self):
        """比价一笔向下 → 资本从 A 流向 B。"""
        pair = _pair("GLD", "SLV")
        strokes = [_stroke(0, 5, "down", 1.65, 1.50)]
        flows = strokes_to_flows(pair, strokes)

        assert len(flows) == 1
        f = flows[0]
        assert f.direction == FlowDirection.A_TO_B
        assert f.sym_from == "GLD"
        assert f.sym_to == "SLV"

    def test_magnitude_calculation(self):
        """magnitude = abs(p1 - p0) / p0。"""
        pair = _pair()
        strokes = [_stroke(0, 5, "up", 2.00, 2.20)]
        flows = strokes_to_flows(pair, strokes)

        expected = abs(2.20 - 2.00) / 2.00  # 0.10
        assert flows[0].magnitude == pytest.approx(expected)

    def test_magnitude_down(self):
        """向下笔的 magnitude 同样是 abs(p1 - p0) / p0。"""
        pair = _pair()
        strokes = [_stroke(0, 5, "down", 2.00, 1.80)]
        flows = strokes_to_flows(pair, strokes)

        expected = abs(1.80 - 2.00) / 2.00  # 0.10
        assert flows[0].magnitude == pytest.approx(expected)

    def test_multiple_strokes(self):
        """多笔序列的映射保持顺序和正确方向。"""
        pair = _pair("SPY", "TLT")
        strokes = [
            _stroke(0, 5, "up", 1.00, 1.10),
            _stroke(5, 10, "down", 1.10, 1.05),
            _stroke(10, 15, "up", 1.05, 1.20),
        ]
        flows = strokes_to_flows(pair, strokes)

        assert len(flows) == 3
        assert flows[0].direction == FlowDirection.B_TO_A
        assert flows[1].direction == FlowDirection.A_TO_B
        assert flows[2].direction == FlowDirection.B_TO_A

        # stroke_index 顺序
        assert [f.stroke_index for f in flows] == [0, 1, 2]

    def test_empty_strokes(self):
        """空笔列表 → 空流转列表。"""
        pair = _pair()
        flows = strokes_to_flows(pair, [])
        assert flows == []

    def test_stroke_index_sequential(self):
        """stroke_index 从 0 开始依次递增。"""
        pair = _pair()
        strokes = [
            _stroke(0, 4, "up", 1.0, 1.1),
            _stroke(4, 8, "down", 1.1, 1.0),
        ]
        flows = strokes_to_flows(pair, strokes)
        assert flows[0].stroke_index == 0
        assert flows[1].stroke_index == 1

    def test_sym_from_to_correct_for_up(self):
        """上涨笔：sym_from = pair.sym_b, sym_to = pair.sym_a。"""
        pair = _pair("A_STOCK", "B_STOCK")
        strokes = [_stroke(0, 5, "up", 1.0, 1.5)]
        flows = strokes_to_flows(pair, strokes)

        assert flows[0].sym_from == "B_STOCK"
        assert flows[0].sym_to == "A_STOCK"

    def test_sym_from_to_correct_for_down(self):
        """下跌笔：sym_from = pair.sym_a, sym_to = pair.sym_b。"""
        pair = _pair("A_STOCK", "B_STOCK")
        strokes = [_stroke(0, 5, "down", 1.5, 1.0)]
        flows = strokes_to_flows(pair, strokes)

        assert flows[0].sym_from == "A_STOCK"
        assert flows[0].sym_to == "B_STOCK"


# ── 边界条件 ─────────────────────────────────────────────


class TestEdgeCases:
    def test_p0_zero_raises(self):
        """p0 = 0 时 magnitude 除零 → 应抛 ValueError。"""
        pair = _pair()
        strokes = [_stroke(0, 5, "up", 0.0, 1.0)]
        with pytest.raises(ValueError, match="p0.*zero"):
            strokes_to_flows(pair, strokes)

    def test_very_small_p0(self):
        """p0 极小但非零时不应报错。"""
        pair = _pair()
        strokes = [_stroke(0, 5, "up", 0.001, 0.002)]
        flows = strokes_to_flows(pair, strokes)

        expected = abs(0.002 - 0.001) / 0.001
        assert flows[0].magnitude == pytest.approx(expected)

    def test_unconfirmed_stroke_still_mapped(self):
        """未确认笔也应被映射（最后一笔通常 confirmed=False）。"""
        pair = _pair()
        strokes = [
            _stroke(0, 5, "up", 1.0, 1.1, confirmed=True),
            _stroke(5, 10, "down", 1.1, 1.05, confirmed=False),
        ]
        flows = strokes_to_flows(pair, strokes)
        assert len(flows) == 2

    def test_single_stroke(self):
        """单笔也能正常映射。"""
        pair = _pair()
        strokes = [_stroke(0, 5, "down", 2.0, 1.8)]
        flows = strokes_to_flows(pair, strokes)

        assert len(flows) == 1
        assert flows[0].direction == FlowDirection.A_TO_B
        assert flows[0].magnitude == pytest.approx(0.1)


# ── IR-1 对称性验证 ──────────────────────────────────────


class TestSymmetryIR1:
    """IR-1：A/B 比价上涨 ⟺ B/A 比价下跌。
    体现为：交换 pair 后，同一 stroke 的 direction 字段不变
    （因为 stroke 已经是在 A/B 比价K线上构造的），
    但 sym_from / sym_to 互换。
    """

    def test_swap_pair_reverses_flow_semantics(self):
        pair_ab = _pair("GLD", "SLV")
        pair_ba = _pair("SLV", "GLD")
        strokes = [_stroke(0, 5, "up", 1.5, 1.65)]

        flows_ab = strokes_to_flows(pair_ab, strokes)
        flows_ba = strokes_to_flows(pair_ba, strokes)

        # 同一笔向上 → 在 A/B 对中资本从 B→A；在 B/A 对中资本从 A(=原B)→B(=原A)
        # 即：pair_ab 的 sym_to 和 pair_ba 的 sym_from 是同一标的
        assert flows_ab[0].sym_to == flows_ba[0].sym_from
        assert flows_ab[0].sym_from == flows_ba[0].sym_to
