"""v1 管线端到端集成测试

验证完整链路：
  Segment → zhongshu_from_segments → moves_from_zhongshus
  → divergences_from_moves_v1 → buysellpoints_from_level

构造手工 Segment 序列，让管线每一层产生有意义输出。
"""

from __future__ import annotations

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import zhongshu_from_segments
from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_buysellpoint_v1 import buysellpoints_from_level


def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


def _make_downtrend_segments() -> list[Segment]:
    """构造一个完整的下跌趋势序列，包含 2 个中枢 + 背驰 C 段。

    结构概览：
      seg0-2  → 中枢 0：三段重叠 [ZD=82, ZG=90]
      seg3-6  → 连接段（从中枢0向下离开、过渡到中枢1区域）
      seg7-9  → 中枢 1：三段重叠 [ZD=52, ZG=58]
      seg10   → C 段（向下离开中枢1，力度弱 → 背驰）
      seg11   → 反弹（用于 Type 2 测试）
      seg12   → 回调（用于 Type 2 测试）

    关键约束：
      - c1.GG(60) < c0.DD(80)  → 下跌趋势成立
      - C 段力度（seg10 振幅×持续）弱于 A 段（seg3-6 振幅×持续）
    """
    return [
        # 中枢 0: [ZD=82, ZG=90], GG=95, DD=80
        _seg(0, 0,   0,  10, "down", 95, 80),    # seg0
        _seg(1, 1,  10,  20, "up",   92, 82),     # seg1
        _seg(2, 2,  20,  30, "down", 90, 82),     # seg2
        # 从中枢0向下离开
        _seg(3, 3,  30,  50, "up",   78, 70),     # seg3: 完全低于中枢0
        _seg(4, 4,  50,  70, "down", 70, 55),     # seg4: A 段向下，大力度（15pt × 20bar = 300）
        _seg(5, 5,  70,  80, "up",   62, 55),     # seg5
        _seg(6, 6,  80,  90, "down", 62, 52),     # seg6
        # 中枢 1: [ZD=52, ZG=58], GG=60, DD=50
        _seg(7, 7,  90, 100, "up",   58, 52),     # seg7
        _seg(8, 8, 100, 110, "down", 58, 50),     # seg8
        _seg(9, 9, 110, 120, "up",   55, 50),     # seg9
        # C 段: 弱力度向下（5pt × 10bar = 50 << 300）
        _seg(10, 10, 120, 130, "down", 50, 45),   # seg10: C 段
        # 反弹 + 回调（用于 Type 2）
        _seg(11, 11, 130, 145, "up",   56, 46),   # seg11: 反弹
        _seg(12, 12, 145, 155, "down", 52, 44),   # seg12: 回调
    ]


class TestV1PipelineE2E:
    """v1 管线端到端：每一层的输出验证。"""

    def test_zhongshu_layer(self):
        """段 → 中枢：至少产生 2 个中枢。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        assert len(zhongshus) >= 2, f"期望 >= 2 中枢，实际 {len(zhongshus)}"

    def test_move_layer(self):
        """中枢 → 走势：至少产生 1 个走势。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) >= 1, f"期望 >= 1 走势，实际 {len(moves)}"

    def test_move_is_trend(self):
        """含 2 个同向中枢的走势应为 trend。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        moves = moves_from_zhongshus(zhongshus)
        trends = [m for m in moves if m.kind == "trend"]
        assert len(trends) >= 1, f"期望 trend 走势，实际 kinds={[m.kind for m in moves]}"

    def test_divergence_layer(self):
        """走势 → 背驰：检测到背驰或至少不报错。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        moves = moves_from_zhongshus(zhongshus)
        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        # 背驰是否被检测取决于中枢的 settled 状态和力度对比
        # 至少不应报错
        assert isinstance(divs, list)

    def test_full_pipeline_no_error(self):
        """完整管线不报错。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        moves = moves_from_zhongshus(zhongshus)
        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        bsps = buysellpoints_from_level(segs, zhongshus, moves, divs, level_id=1)
        assert isinstance(bsps, list)

    def test_pipeline_outputs_consistent(self):
        """管线输出的索引引用在合法范围内。"""
        segs = _make_downtrend_segments()
        zhongshus = zhongshu_from_segments(segs)
        moves = moves_from_zhongshus(zhongshus)
        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        bsps = buysellpoints_from_level(segs, zhongshus, moves, divs, level_id=1)

        n_segs = len(segs)
        n_zs = len(zhongshus)

        # 中枢索引范围检查
        for zs in zhongshus:
            assert 0 <= zs.seg_start < n_segs
            assert 0 <= zs.seg_end < n_segs

        # Move 索引范围检查
        for m in moves:
            assert 0 <= m.seg_start < n_segs
            assert 0 <= m.seg_end < n_segs
            assert 0 <= m.zs_start < n_zs
            assert 0 <= m.zs_end < n_zs

        # 买卖点索引范围检查
        for bp in bsps:
            assert 0 <= bp.seg_idx < n_segs
            assert bp.kind in ("type1", "type2", "type3")
            assert bp.side in ("buy", "sell")

    def test_pipeline_deterministic(self):
        """同输入两次执行产生同输出（I28 回放确定性）。"""
        segs = _make_downtrend_segments()

        def run():
            zhongshus = zhongshu_from_segments(segs)
            moves = moves_from_zhongshus(zhongshus)
            divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
            return buysellpoints_from_level(segs, zhongshus, moves, divs, level_id=1)

        bsps1 = run()
        bsps2 = run()
        assert bsps1 == bsps2

    def test_empty_pipeline(self):
        """空段输入 → 全空输出。"""
        zhongshus = zhongshu_from_segments([])
        moves = moves_from_zhongshus(zhongshus)
        divs = divergences_from_moves_v1([], zhongshus, moves, level_id=1)
        bsps = buysellpoints_from_level([], zhongshus, moves, divs, level_id=1)
        assert zhongshus == []
        assert moves == []
        assert divs == []
        assert bsps == []
