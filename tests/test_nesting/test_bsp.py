"""买卖点类型定义测试。"""

from __future__ import annotations

import pytest

from newchan.nesting.bsp import BSP, BSPType, DivergenceType, is_negated


class TestBSPType:
    def test_buy_types(self):
        assert BSPType.B1.is_buy
        assert BSPType.B2.is_buy
        assert BSPType.B3.is_buy
        assert not BSPType.S1.is_buy
        assert not BSPType.NONE.is_buy

    def test_sell_types(self):
        assert BSPType.S1.is_sell
        assert BSPType.S2.is_sell
        assert BSPType.S3.is_sell
        assert not BSPType.B1.is_sell
        assert not BSPType.NONE.is_sell

    def test_is_present(self):
        assert BSPType.B1.is_present
        assert BSPType.S3.is_present
        assert not BSPType.NONE.is_present

    def test_values_match_spec(self):
        """BSPType 值与 §1.1 记号一致。"""
        assert BSPType.B1.value == "1B"
        assert BSPType.S1.value == "1S"


class TestBSP:
    def test_frozen(self):
        bsp = BSP(
            edge_id="E/$", level=1, time=100.0,
            bsp_type=BSPType.B1, price=50.0,
        )
        with pytest.raises(AttributeError):
            bsp.price = 60.0  # type: ignore[misc]

    def test_direction_is_buy(self):
        buy = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.B1, price=50.0)
        sell = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.S1, price=50.0)
        assert buy.direction_is_buy
        assert not sell.direction_is_buy


class TestIsNegated:
    def test_buy_negated_when_price_falls_below(self):
        """§4.5: 一买否定——价格跌破买点价格。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.B1, price=50.0)
        assert is_negated(bsp, 49.0)
        assert not is_negated(bsp, 50.0)
        assert not is_negated(bsp, 51.0)

    def test_sell_negated_when_price_rises_above(self):
        """§4.5: 一卖否定——价格突破卖点价格。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.S1, price=100.0)
        assert is_negated(bsp, 101.0)
        assert not is_negated(bsp, 100.0)
        assert not is_negated(bsp, 99.0)

    def test_none_type_not_negated(self):
        """NONE 类型不可能被否定。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.NONE, price=50.0)
        assert not is_negated(bsp, 0.0)

    def test_b2_negated(self):
        """二买否定。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.B2, price=55.0)
        assert is_negated(bsp, 54.0)
        assert not is_negated(bsp, 56.0)

    def test_b3_negated(self):
        """三买否定。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.B3, price=60.0)
        assert is_negated(bsp, 59.0)
        assert not is_negated(bsp, 61.0)

    def test_s2_negated(self):
        """二卖否定。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.S2, price=90.0)
        assert is_negated(bsp, 91.0)
        assert not is_negated(bsp, 89.0)

    def test_s3_negated(self):
        """三卖否定。"""
        bsp = BSP(edge_id="E/$", level=1, time=100.0, bsp_type=BSPType.S3, price=85.0)
        assert is_negated(bsp, 86.0)
        assert not is_negated(bsp, 84.0)
