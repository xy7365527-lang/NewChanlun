"""Type 3 买卖点 golden case 测试。

覆盖 8 个场景：

TestType3Detection（检测正确性）
  1. test_type3_buy_standard — 标准 3B
  2. test_type3_sell_standard — 标准 3S
  3. test_type3_buy_pullback_below_zg — 回试段 low <= ZG，无 3B
  4. test_type3_sell_rally_above_zd — 回抽段 high >= ZD，无 3S

TestType3EdgeCases（边界情况）
  5. test_type3_unsettled_zhongshu_skip — 中枢未 settled，跳过
  6. test_type3_no_break_direction_skip — 无 break_direction，跳过
  7. test_type3_confirmed_from_move_settled — confirmed 来自 Move.settled
  8. test_type3_no_move_confirmed_false — 无 Move 覆盖，confirmed=False
"""

from dataclasses import dataclass

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu


# ── 辅助 Segment stub ──────────────────────────────────────


@dataclass
class _Seg:
    """最小化 Segment stub，只保留 BSP 检测所需字段。"""

    direction: str
    high: float
    low: float
    i0: int = 0
    i1: int = 0
    s0: int = 0
    s1: int = 0
    confirmed: bool = False


# ── 辅助构造器 ────────────────────────────────────────


def _make_zhongshu(
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    settled: bool = True,
    break_direction: str = "",
    break_seg: int = -1,
) -> Zhongshu:
    """构造最小 Zhongshu。"""
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_end - seg_start + 1,
        settled=settled,
        break_seg=break_seg,
        break_direction=break_direction,
        dd=zd - 5,
        gg=zg + 5,
    )


# ═══════════════════════════════════════════════════════════
# TestType3Detection — 检测正确性
# ═══════════════════════════════════════════════════════════


class TestType3Detection:
    """Type 3 买卖点检测正确性。"""

    def test_type3_buy_standard(self):
        """标准 3B：中枢向上突破后回试段 low > ZG。

        结构：
        - seg[0..2]: 构成中枢 (zd=52, zg=60)
        - seg[3]: 向上突破段 (break_seg=3)
        - seg[4]: 回试段 (down, low=66 > zg=60) -> 3B
        """
        segments = [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),   # 离开段
            _Seg(direction="down", high=72.0, low=66.0, i0=20, i1=25), # 回试段
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True, break_direction="up", break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 1
        bsp = type3_bsps[0]
        assert bsp.kind == "type3"
        assert bsp.side == "buy"
        assert bsp.seg_idx == 4
        assert bsp.price == 66.0

    def test_type3_sell_standard(self):
        """标准 3S：中枢向下突破后回抽段 high < ZD。

        结构：
        - seg[0..2]: 构成中枢 (zd=55, zg=65)
        - seg[3]: 向下突破段 (break_seg=3)
        - seg[4]: 回抽段 (up, high=53 < zd=55) -> 3S
        """
        segments = [
            _Seg(direction="up", high=70.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=68.0, low=58.0, i0=10, i1=15),
            _Seg(direction="down", high=50.0, low=40.0, i0=15, i1=20),  # 离开段
            _Seg(direction="up", high=53.0, low=45.0, i0=20, i1=25),    # 回抽段
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=55.0, zg=65.0,
                settled=True, break_direction="down", break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 1
        bsp = type3_bsps[0]
        assert bsp.kind == "type3"
        assert bsp.side == "sell"
        assert bsp.seg_idx == 4
        assert bsp.price == 53.0

    def test_type3_buy_pullback_below_zg(self):
        """回试段 low <= ZG -> 不满足 3B 条件，无 type3 产出。

        同标准 3B 场景但 seg[4].low=58 <= zg=60。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),   # 离开段
            _Seg(direction="down", high=72.0, low=58.0, i0=20, i1=25), # low=58 <= zg=60
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True, break_direction="up", break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 0

    def test_type3_sell_rally_above_zd(self):
        """回抽段 high >= ZD -> 不满足 3S 条件，无 type3 产出。

        同标准 3S 场景但 seg[4].high=57 >= zd=55。
        """
        segments = [
            _Seg(direction="up", high=70.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=68.0, low=58.0, i0=10, i1=15),
            _Seg(direction="down", high=50.0, low=40.0, i0=15, i1=20),  # 离开段
            _Seg(direction="up", high=57.0, low=45.0, i0=20, i1=25),    # high=57 >= zd=55
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=55.0, zg=65.0,
                settled=True, break_direction="down", break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 0


# ═══════════════════════════════════════════════════════════
# TestType3EdgeCases — 边界情况
# ═══════════════════════════════════════════════════════════


class TestType3EdgeCases:
    """Type 3 买卖点边界情况。"""

    def _build_standard_3b_segments(self):
        """构造标准 3B 的 5 段结构（复用）。"""
        return [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),   # 离开段
            _Seg(direction="down", high=72.0, low=66.0, i0=20, i1=25), # 回试段 low=66 > zg=60
        ]

    def test_type3_unsettled_zhongshu_skip(self):
        """中枢未 settled -> 跳过，无 type3 产出。"""
        segments = self._build_standard_3b_segments()

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=False,  # 未 settled
                break_direction="up", break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 0

    def test_type3_no_break_direction_skip(self):
        """无 break_direction -> 跳过，无 type3 产出。"""
        segments = self._build_standard_3b_segments()

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True,
                break_direction="",  # 空 break_direction
                break_seg=3,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 0

    def test_type3_confirmed_from_move_settled(self):
        """confirmed 来自 Move.settled：有 Move 覆盖 pullback_idx 且 settled=True。"""
        segments = self._build_standard_3b_segments()

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True, break_direction="up", break_seg=3,
            ),
        ]

        # Move 覆盖 seg[0..4]，settled=True
        moves = [
            Move(
                kind="consolidation",
                direction="up",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=0,
                zs_count=1,
                settled=True,
                high=75.0,
                low=50.0,
                first_seg_s0=0,
                last_seg_s1=25,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves=moves, divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 1
        assert type3_bsps[0].confirmed is True

    def test_type3_no_move_confirmed_false(self):
        """无 Move 覆盖 pullback_idx -> confirmed=False。"""
        segments = self._build_standard_3b_segments()

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True, break_direction="up", break_seg=3,
            ),
        ]

        # moves 为空
        bsps = buysellpoints_from_level(segments, zhongshus, moves=[], divergences=[], level_id=1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type3_bsps) == 1
        assert type3_bsps[0].confirmed is False
