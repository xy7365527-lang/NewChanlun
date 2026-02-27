"""共振测试。"""

from __future__ import annotations

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.resonance import (
    ResonanceLevel,
    ResonanceSignal,
    SignalLayer,
    classify_resonance,
    resonance_check,
    resonance_strength,
)


def _make_signal(
    edge_id: str = "E/$",
    level: int = 3,
    bsp_type: BSPType = BSPType.B1,
    layer: SignalLayer = SignalLayer.INDEPENDENT_EDGE,
    time: float = 100.0,
) -> ResonanceSignal:
    bsp = BSP(edge_id=edge_id, level=level, time=time, bsp_type=bsp_type, price=50.0)
    return ResonanceSignal(edge_id=edge_id, level=level, bsp=bsp, layer=layer, time=time)


def _constant_tolerance(_a: int, _b: int) -> float:
    return 10.0


class TestResonanceCheck:
    def test_valid_resonance(self):
        """R1+R2+R3 全部满足。"""
        signals = [
            _make_signal(edge_id="config", layer=SignalLayer.CONFIG, time=100.0),
            _make_signal(edge_id="E/$", layer=SignalLayer.INDEPENDENT_EDGE, time=102.0),
        ]
        assert resonance_check(signals, _constant_tolerance)

    def test_single_signal_not_resonance(self):
        """单一信号不构成共振。"""
        signals = [_make_signal()]
        assert not resonance_check(signals, _constant_tolerance)

    def test_direction_mismatch(self):
        """R2 违反：方向不一致。"""
        signals = [
            _make_signal(bsp_type=BSPType.B1, time=100.0),
            _make_signal(bsp_type=BSPType.S1, time=100.0, edge_id="C/$"),
        ]
        assert not resonance_check(signals, _constant_tolerance)

    def test_time_too_far(self):
        """R3 违反：时间不重叠。"""
        signals = [
            _make_signal(time=100.0),
            _make_signal(time=200.0, edge_id="C/$"),
        ]
        assert not resonance_check(signals, _constant_tolerance)

    def test_none_bsp_type(self):
        """R1 违反：某条线无买卖点。"""
        signals = [
            _make_signal(bsp_type=BSPType.B1),
            _make_signal(bsp_type=BSPType.NONE, edge_id="C/$"),
        ]
        assert not resonance_check(signals, _constant_tolerance)


class TestResonanceStrength:
    def test_three_layer(self):
        """三层共振：config(3) + independent(2) + underlying(1) = 6。"""
        signals = [
            _make_signal(layer=SignalLayer.CONFIG),
            _make_signal(layer=SignalLayer.INDEPENDENT_EDGE, edge_id="C/$"),
            _make_signal(layer=SignalLayer.UNDERLYING, edge_id="AAPL"),
        ]
        assert resonance_strength(signals) == 6.0

    def test_two_layer(self):
        """双层共振：config(3) + independent(2) = 5。"""
        signals = [
            _make_signal(layer=SignalLayer.CONFIG),
            _make_signal(layer=SignalLayer.INDEPENDENT_EDGE, edge_id="C/$"),
        ]
        assert resonance_strength(signals) == 5.0

    def test_single_layer(self):
        """单层：independent(2) = 2。"""
        signals = [_make_signal(layer=SignalLayer.INDEPENDENT_EDGE)]
        assert resonance_strength(signals) == 2.0

    def test_derived_edge_adds_one(self):
        """§5.4: 派生边增加 1。"""
        signals = [
            _make_signal(layer=SignalLayer.INDEPENDENT_EDGE),
            _make_signal(layer=SignalLayer.DERIVED_EDGE, edge_id="E/C"),
        ]
        assert resonance_strength(signals) == 3.0

    def test_none_bsp_excluded(self):
        """NONE 类型不计入强度。"""
        signals = [
            _make_signal(layer=SignalLayer.CONFIG),
            _make_signal(layer=SignalLayer.INDEPENDENT_EDGE, bsp_type=BSPType.NONE, edge_id="C/$"),
        ]
        assert resonance_strength(signals) == 3.0


class TestClassifyResonance:
    def test_three_layer(self):
        assert classify_resonance(6.0) is ResonanceLevel.THREE_LAYER
        assert classify_resonance(7.0) is ResonanceLevel.THREE_LAYER

    def test_two_layer(self):
        assert classify_resonance(3.0) is ResonanceLevel.TWO_LAYER
        assert classify_resonance(5.9) is ResonanceLevel.TWO_LAYER

    def test_single(self):
        assert classify_resonance(1.0) is ResonanceLevel.SINGLE
        assert classify_resonance(2.9) is ResonanceLevel.SINGLE
        assert classify_resonance(0.0) is ResonanceLevel.SINGLE
