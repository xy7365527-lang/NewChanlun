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

    def test_commodity_is_gap(self):
        """C 顶点 = 广义商品(DBC)，1min 仍无源，诚实标注为需采集。"""
        s = vertex_source(Vertex.C)
        assert s.availability is DataAvailability.NEEDS_ACQUISITION

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
    def test_gap_is_c_only(self):
        """诚实缺口清单：R 经 TWS 补齐后，仅剩 C（广义商品 DBC 未拉取）。"""
        gaps = set(data_gaps())
        assert gaps == {Vertex.C}
