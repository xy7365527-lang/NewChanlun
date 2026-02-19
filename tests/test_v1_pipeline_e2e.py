"""v1 管线端到端集成测试。

验证完整链路：
  Segment → zhongshu_from_segments → moves_from_zhongshus
  → divergences_from_moves_v1 → buysellpoints_from_level

测试场景：16 段手工构造的下跌趋势（2 个中枢），验证每一层产生有意义的输出。

走势结构示意（价格区间）：

  中枢1 [ZD=84, ZG=93]  (seg 0-6, 7段含4段延伸)
    ↓ 突破(down)
  连接段
    ↓
  中枢2 [ZD=55, ZG=68]  (seg 7-11, 5段含2段延伸)
    ↓ 突破(down)
  回抽 + 后续段

  _is_descending: c2.gg=78 < c1.dd=80 → 下跌趋势
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_buysellpoint_v1 import BuySellPoint, buysellpoints_from_level


# ── helpers ──

def _seg(
    idx: int,
    direction: str,
    high: float,
    low: float,
    i0: int,
    i1: int,
    confirmed: bool = True,
) -> Segment:
    """构造一个简化的 Segment。

    s0/s1 用 idx*2 / idx*2+1 自动编号（笔索引），
    ep* 字段和 p0/p1 使用方向推导的端点价格。
    """
    s0 = idx * 2
    s1 = idx * 2 + 1
    if direction == "down":
        ep0_price, ep0_type = high, "top"
        ep1_price, ep1_type = low, "bottom"
    else:
        ep0_price, ep0_type = low, "bottom"
        ep1_price, ep1_type = high, "top"
    return Segment(
        s0=s0,
        s1=s1,
        i0=i0,
        i1=i1,
        direction=direction,
        high=high,
        low=low,
        confirmed=confirmed,
        kind="settled" if confirmed else "candidate",
        ep0_i=i0,
        ep0_price=ep0_price,
        ep0_type=ep0_type,
        ep1_i=i1,
        ep1_price=ep1_price,
        ep1_type=ep1_type,
        p0=ep0_price,
        p1=ep1_price,
    )


def _build_downtrend_segments() -> list[Segment]:
    """构造 16 段下跌趋势数据。

    结构:
      seg 0-6 : 中枢1 区间 [ZD=84, ZG=93]
                seg 0-2 初始三段, seg 3-6 延伸
      seg 7   : 中枢1 突破段 (break_dir="down")
      seg 7-11: 中枢2 区间 [ZD=55, ZG=68]
                seg 7-9 初始三段, seg 10-11 延伸
      seg 12  : 中枢2 突破段 (break_dir="down")
      seg 13  : 回抽段 (type3 sell 候选)
      seg 14  : 继续下跌
      seg 15  : 最后段 (unconfirmed)
    """
    # fmt: off
    data = [
        #  idx  dir     high   low    i0    i1   confirmed
        (  0, "down",  100.0,  82.0,    0,   10,  True),   # 中枢1 起始
        (  1, "up",     95.0,  84.0,   11,   20,  True),
        (  2, "down",   93.0,  80.0,   21,   30,  True),   # 中枢1 初始三段完成
        (  3, "up",     92.0,  85.0,   31,   40,  True),   # 延伸
        (  4, "down",   91.0,  83.0,   41,   50,  True),   # 延伸
        (  5, "up",     90.0,  86.0,   51,   60,  True),   # 延伸
        (  6, "down",   88.0,  84.0,   61,   70,  True),   # 延伸 (弱不等式)
        (  7, "up",     78.0,  55.0,   71,   90,  True),   # 突破中枢1 → 中枢2 起始
        (  8, "down",   72.0,  48.0,   91,  110,  True),
        (  9, "up",     68.0,  50.0,  111,  130,  True),   # 中枢2 初始三段完成
        ( 10, "down",   65.0,  52.0,  131,  150,  True),   # 延伸
        ( 11, "up",     60.0,  56.0,  151,  165,  True),   # 延伸
        ( 12, "down",   47.0,  30.0,  166,  185,  True),   # 突破中枢2
        ( 13, "up",     42.0,  25.0,  186,  200,  True),   # 回抽(type3 sell)
        ( 14, "down",   23.0,  10.0,  201,  220,  True),   # 继续下跌
        ( 15, "up",     20.0,   8.0,  221,  240,  False),  # unconfirmed
    ]
    # fmt: on
    return [_seg(idx, d, h, l, i0, i1, c) for idx, d, h, l, i0, i1, c in data]


# ── 常量 ──

LEVEL_ID = 1


# =====================================================================
# TestV1PipelineE2E
# =====================================================================

class TestV1PipelineE2E:
    """v1 管线端到端集成测试。"""

    @pytest.fixture()
    def segments(self) -> list[Segment]:
        return _build_downtrend_segments()

    @pytest.fixture()
    def zhongshus(self, segments: list[Segment]) -> list[Zhongshu]:
        return zhongshu_from_segments(segments)

    @pytest.fixture()
    def moves(self, zhongshus: list[Zhongshu]) -> list[Move]:
        return moves_from_zhongshus(zhongshus)

    @pytest.fixture()
    def divergences(
        self,
        segments: list[Segment],
        zhongshus: list[Zhongshu],
        moves: list[Move],
    ) -> list:
        return divergences_from_moves_v1(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_id=LEVEL_ID,
            df_macd=None,
            merged_to_raw=None,
        )

    @pytest.fixture()
    def buysellpoints(
        self,
        segments: list[Segment],
        zhongshus: list[Zhongshu],
        moves: list[Move],
        divergences: list,
    ) -> list[BuySellPoint]:
        return buysellpoints_from_level(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            divergences=divergences,
            level_id=LEVEL_ID,
        )

    # ─── 第一层：zhongshu_from_segments ───

    def test_zhongshu_count(self, zhongshus: list[Zhongshu]):
        """16 段下跌数据应产生恰好 2 个中枢。"""
        assert len(zhongshus) == 2

    def test_zhongshu1_zone(self, zhongshus: list[Zhongshu]):
        """中枢1 区间 [ZD=84, ZG=93]。"""
        zs1 = zhongshus[0]
        assert zs1.zd == pytest.approx(84.0)
        assert zs1.zg == pytest.approx(93.0)

    def test_zhongshu1_settled(self, zhongshus: list[Zhongshu]):
        """中枢1 已被突破（settled）且方向为 down。"""
        zs1 = zhongshus[0]
        assert zs1.settled is True
        assert zs1.break_direction == "down"

    def test_zhongshu1_segments(self, zhongshus: list[Zhongshu]):
        """中枢1 含 7 段（seg 0-6，含 4 段延伸）。"""
        zs1 = zhongshus[0]
        assert zs1.seg_start == 0
        assert zs1.seg_end == 6
        assert zs1.seg_count == 7

    def test_zhongshu1_gg_dd(self, zhongshus: list[Zhongshu]):
        """中枢1 波动区间 GG=100, DD=80。"""
        zs1 = zhongshus[0]
        assert zs1.gg == pytest.approx(100.0)
        assert zs1.dd == pytest.approx(80.0)

    def test_zhongshu2_zone(self, zhongshus: list[Zhongshu]):
        """中枢2 区间 [ZD=55, ZG=68]。"""
        zs2 = zhongshus[1]
        assert zs2.zd == pytest.approx(55.0)
        assert zs2.zg == pytest.approx(68.0)

    def test_zhongshu2_settled(self, zhongshus: list[Zhongshu]):
        """中枢2 已被突破且方向为 down。"""
        zs2 = zhongshus[1]
        assert zs2.settled is True
        assert zs2.break_direction == "down"

    def test_zhongshu2_segments(self, zhongshus: list[Zhongshu]):
        """中枢2 含 5 段（seg 7-11，含 2 段延伸）。"""
        zs2 = zhongshus[1]
        assert zs2.seg_start == 7
        assert zs2.seg_end == 11
        assert zs2.seg_count == 5

    def test_zhongshu2_gg_dd(self, zhongshus: list[Zhongshu]):
        """中枢2 波动区间 GG=78, DD=48。"""
        zs2 = zhongshus[1]
        assert zs2.gg == pytest.approx(78.0)
        assert zs2.dd == pytest.approx(48.0)

    def test_zhongshu_descending(self, zhongshus: list[Zhongshu]):
        """中枢2.gg < 中枢1.dd → 下跌方向。"""
        zs1, zs2 = zhongshus[0], zhongshus[1]
        assert zs2.gg < zs1.dd, (
            f"中枢2.gg={zs2.gg} 应 < 中枢1.dd={zs1.dd} 以满足下跌趋势条件"
        )

    # ─── 第二层：moves_from_zhongshus ───

    def test_move_count(self, moves: list[Move]):
        """2 个同向中枢应产生 1 个走势。"""
        assert len(moves) == 1

    def test_move_is_downtrend(self, moves: list[Move]):
        """走势类型为下跌趋势（kind=trend, direction=down）。"""
        m = moves[0]
        assert m.kind == "trend"
        assert m.direction == "down"
        assert m.zs_count == 2

    def test_move_segment_range(self, moves: list[Move]):
        """走势覆盖 seg_start=0 到 seg_end=11。"""
        m = moves[0]
        assert m.seg_start == 0
        assert m.seg_end == 11

    def test_move_high_low(self, moves: list[Move]):
        """走势高低点来自波动极值：high=max(gg), low=min(dd)。"""
        m = moves[0]
        assert m.high == pytest.approx(100.0)  # max(gg1, gg2)
        assert m.low == pytest.approx(48.0)    # min(dd1, dd2)

    def test_move_last_unsettled(self, moves: list[Move]):
        """最后一个走势 settled=False（moves_from_zhongshus 强制规则）。"""
        assert moves[-1].settled is False

    # ─── 第三层：divergences_from_moves_v1 ───

    def test_divergence_count(self, divergences: list):
        """当前实现中趋势背驰的 C 段区间为空（move.seg_end == last_zs.seg_end），
        因此不会产生趋势背驰。背驰列表预期为空。

        Notes
        -----
        这是 v1 管线的已知特性：Move 的 seg_end 来自 last_zs.seg_end，
        不包含中枢之后的离开段，导致 C 段 [seg_end+1, seg_end] 区间为空。
        若未来 Move 扩展至覆盖离开段，此测试应更新为 >= 1。
        """
        assert len(divergences) == 0

    # ─── 第四层：buysellpoints_from_level ───

    def test_buysellpoint_count(self, buysellpoints: list[BuySellPoint]):
        """无趋势背驰 → 无 type1；2 个 settled 中枢 break_dir=down + 回抽 → 2 个 type3。"""
        assert len(buysellpoints) == 2

    def test_buysellpoint_all_type3(self, buysellpoints: list[BuySellPoint]):
        """所有买卖点均为 type3。"""
        for bp in buysellpoints:
            assert bp.kind == "type3"

    def test_buysellpoint_all_sell(self, buysellpoints: list[BuySellPoint]):
        """下跌突破后回抽不入中枢 → 全部为 sell 点。"""
        for bp in buysellpoints:
            assert bp.side == "sell"

    def test_buysellpoint_level_id(self, buysellpoints: list[BuySellPoint]):
        """买卖点 level_id 应与传入值一致。"""
        for bp in buysellpoints:
            assert bp.level_id == LEVEL_ID

    def test_buysellpoint_seg_indices(self, buysellpoints: list[BuySellPoint]):
        """type3 sell 分别位于 seg 9（中枢1回抽）和 seg 13（中枢2回抽）。"""
        seg_indices = [bp.seg_idx for bp in buysellpoints]
        assert seg_indices == [9, 13]

    def test_buysellpoint_prices(self, buysellpoints: list[BuySellPoint]):
        """type3 sell 价格 = 回抽段的 high（卖出参考价）。"""
        bp1, bp2 = buysellpoints
        # 中枢1 回抽段 seg 9: high=68
        assert bp1.price == pytest.approx(68.0)
        # 中枢2 回抽段 seg 13: high=42
        assert bp2.price == pytest.approx(42.0)

    def test_buysellpoint_bar_indices(self, buysellpoints: list[BuySellPoint]):
        """type3 sell 的 bar_idx = 回抽段的 i1。"""
        bp1, bp2 = buysellpoints
        assert bp1.bar_idx == 130   # seg 9 的 i1
        assert bp2.bar_idx == 200   # seg 13 的 i1

    def test_buysellpoint_center_zone(self, buysellpoints: list[BuySellPoint]):
        """type3 sell 的中枢区间与对应中枢一致。"""
        bp1, bp2 = buysellpoints
        # 中枢1: [ZD=84, ZG=93]
        assert bp1.center_zd == pytest.approx(84.0)
        assert bp1.center_zg == pytest.approx(93.0)
        # 中枢2: [ZD=55, ZG=68]
        assert bp2.center_zd == pytest.approx(55.0)
        assert bp2.center_zg == pytest.approx(68.0)

    def test_buysellpoint_divergence_key_none(
        self, buysellpoints: list[BuySellPoint],
    ):
        """type3 买卖点无关联背驰（divergence_key=None）。"""
        for bp in buysellpoints:
            assert bp.divergence_key is None

    # ─── 全管线完整性检查 ───

    def test_pipeline_all_layers_produce_output(
        self,
        segments: list[Segment],
        zhongshus: list[Zhongshu],
        moves: list[Move],
        divergences: list,
        buysellpoints: list[BuySellPoint],
    ):
        """管线各层均产出有意义的结果（divergences 为空是当前已知特性）。"""
        assert len(segments) == 16
        assert len(zhongshus) >= 2
        assert len(moves) >= 1
        # divergences 可为 0（C 段覆盖限制），此处只验证类型正确
        assert isinstance(divergences, list)
        assert len(buysellpoints) >= 1

    def test_pipeline_sorted_output(self, buysellpoints: list[BuySellPoint]):
        """buysellpoints 按 seg_idx 升序排列。"""
        indices = [bp.seg_idx for bp in buysellpoints]
        assert indices == sorted(indices)

    def test_pipeline_immutability(self, segments: list[Segment]):
        """验证管线不修改输入数据（frozen dataclass 保证）。"""
        # 对 frozen dataclass 尝试赋值应抛出 FrozenInstanceError
        with pytest.raises(AttributeError):
            segments[0].high = 999.0  # type: ignore[misc]
