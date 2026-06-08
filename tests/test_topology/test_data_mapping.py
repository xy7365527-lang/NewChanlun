"""数据映射测试（528号单一真相源 + 231号诚实数据缺口标注）。"""

from newchan.topology.data_mapping import (
    DataAvailability,
    data_gaps,
    fold_channel_source,
    vertex_source,
)
from newchan.topology.fold_channel import AU, OIL
from newchan.topology.graph import Vertex


class TestVertexSources:
    def test_all_four_vertices_mapped(self):
        for v in Vertex:
            assert vertex_source(v) is not None

    def test_money_uup_available(self):
        s = vertex_source(Vertex.M)
        assert s.symbol == "UUP"
        assert s.availability is DataAvailability.AVAILABLE

    def test_production_es_available(self):
        s = vertex_source(Vertex.P)
        assert s.symbol == "ES"
        assert s.availability is DataAvailability.AVAILABLE

    def test_commodity_dbc_available(self):
        """C 顶点 = DBC（广义商品 ETF），1min 经 TWS 拉取后可用。"""
        s = vertex_source(Vertex.C)
        assert s.symbol == "DBC"
        assert s.availability is DataAvailability.AVAILABLE

    def test_realestate_vnq_available(self):
        """R 顶点 = VNQ（房地产 ETF），1min 经 TWS 拉取后可用。"""
        s = vertex_source(Vertex.R)
        assert s.symbol == "VNQ"
        assert s.availability is DataAvailability.AVAILABLE


class TestFoldChannelSources:
    def test_au_gold_available(self):
        s = fold_channel_source(AU)
        assert s.symbol == "GC"
        assert s.availability is DataAvailability.AVAILABLE

    def test_oil_crude_available(self):
        s = fold_channel_source(OIL)
        assert s.symbol == "CL"
        assert s.availability is DataAvailability.AVAILABLE


class TestDataGaps:
    def test_no_us_gaps(self):
        """美国四顶点 1min 全部齐备（C=DBC、R=VNQ 经 TWS 补齐后无缺口）。"""
        assert data_gaps() == ()
