"""Move C段覆盖修复 — 验证 seg_end 扩展到中枢后离开段。

Bug: moves_from_zhongshus() 设置 seg_end = last_zs.seg_end,
导致 C段（中枢后的离开段）不在 Move 范围内。
背驰检测中 c_start = zs_last.seg_end + 1 > move.seg_end → C段永远不形成。

修复: 扩展 seg_end 到:
  - 非末组: next_group_first_zs.seg_start - 1
  - 末组: num_segments - 1 (若提供)
"""

from __future__ import annotations

import pytest

from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.a_zhongshu_v1 import Zhongshu


def _zs(
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    settled: bool = True,
    break_seg: int = -1,
    break_direction: str = "",
    gg: float | None = None,
    dd: float | None = None,
) -> Zhongshu:
    return Zhongshu(
        zd=zd, zg=zg,
        seg_start=seg_start, seg_end=seg_end,
        seg_count=seg_end - seg_start + 1,
        settled=settled,
        break_seg=break_seg,
        break_direction=break_direction,
        first_seg_s0=seg_start * 2,
        last_seg_s1=seg_end * 2 + 1,
        gg=gg if gg is not None else zg + 2,
        dd=dd if dd is not None else zd - 2,
    )


class TestCSSegmentCoverage:
    """Move.seg_end 应覆盖中枢后的离开段（C段）。"""

    def test_single_trend_c_segment_without_num_segments(self):
        """无 num_segments 时，seg_end 仍为 last_zs.seg_end（向后兼容）。"""
        # ascending 条件: zs1.dd > zs0.gg → 28 > 17 ✓
        zs0 = _zs(0, 2, 10, 15, gg=17, dd=8, settled=True, break_seg=3, break_direction="up")
        zs1 = _zs(4, 6, 30, 40, gg=42, dd=28, settled=True, break_seg=7, break_direction="up")
        moves = moves_from_zhongshus([zs0, zs1])
        assert len(moves) == 1
        assert moves[0].kind == "trend"
        assert moves[0].seg_end == 6  # 无 num_segments → 保持旧行为

    def test_single_trend_c_segment_with_num_segments(self):
        """有 num_segments 时，末组 seg_end 扩展到 num_segments - 1。

        场景: seg 0-2 → zs0, seg 4-6 → zs1, seg 7-8 → C段
        num_segments = 9 → seg_end 应为 8
        """
        zs0 = _zs(0, 2, 10, 15, gg=17, dd=8, settled=True, break_seg=3, break_direction="up")
        zs1 = _zs(4, 6, 30, 40, gg=42, dd=28, settled=True, break_seg=7, break_direction="up")
        moves = moves_from_zhongshus([zs0, zs1], num_segments=9)
        assert len(moves) == 1
        assert moves[0].kind == "trend"
        assert moves[0].seg_end == 8  # C段 seg7 + seg8 被覆盖

    def test_two_groups_inter_group_coverage(self):
        """两组 Move 之间的间隙段应被前一组覆盖。

        场景:
          Group 1 (ascending): zs0(seg 0-2), zs1(seg 4-6)  ascending: zs1.dd=28 > zs0.gg=17
          Group 2 (single):    zs2(seg 10-12)  break from ascending
          seg 7-9 是 Group 1 的 C段
        """
        zs0 = _zs(0, 2, 10, 15, gg=17, dd=8, settled=True, break_seg=3, break_direction="up")
        zs1 = _zs(4, 6, 30, 40, gg=42, dd=28, settled=True, break_seg=7, break_direction="up")
        zs2 = _zs(10, 12, 5, 15, gg=17, dd=3, settled=True, break_seg=13, break_direction="down")
        moves = moves_from_zhongshus([zs0, zs1, zs2], num_segments=14)
        assert len(moves) == 2
        # Group 1: trend up, seg_end should extend to 9 (= zs2.seg_start - 1)
        assert moves[0].seg_end == 9
        # Group 2: consolidation, seg_end should extend to 13 (= num_segments - 1)
        assert moves[1].seg_end == 13

    def test_divergence_c_segment_detectable(self):
        """集成: Move 扩展后，趋势背驰的 C段 能被检测到。"""
        from newchan.a_divergence_v1 import divergences_from_moves_v1
        from newchan.a_segment_v0 import Segment

        segs = [
            Segment(i0=0, i1=10, s0=0, s1=1, direction="up", high=20, low=10, confirmed=True),
            Segment(i0=10, i1=20, s0=2, s1=3, direction="down", high=18, low=12, confirmed=True),
            Segment(i0=20, i1=30, s0=4, s1=5, direction="up", high=22, low=11, confirmed=True),
            # A段 (连接 zs0→zs1)
            Segment(i0=30, i1=40, s0=6, s1=7, direction="down", high=20, low=14, confirmed=True),
            Segment(i0=40, i1=50, s0=8, s1=9, direction="up", high=55, low=30, confirmed=True),
            Segment(i0=50, i1=60, s0=10, s1=11, direction="down", high=50, low=35, confirmed=True),
            Segment(i0=60, i1=70, s0=12, s1=13, direction="up", high=60, low=40, confirmed=True),
            # zs1
            Segment(i0=70, i1=80, s0=14, s1=15, direction="down", high=58, low=42, confirmed=True),
            Segment(i0=80, i1=90, s0=16, s1=17, direction="up", high=62, low=44, confirmed=True),
            Segment(i0=90, i1=100, s0=18, s1=19, direction="down", high=58, low=45, confirmed=True),
            # C段: 弱势离开 → 应触发背驰
            Segment(i0=100, i1=110, s0=20, s1=21, direction="up", high=48, low=46, confirmed=True),
        ]

        zs0 = _zs(0, 2, 12, 18, settled=True, break_seg=3, break_direction="up")
        zs1 = _zs(7, 9, 45, 58, settled=True, break_seg=10, break_direction="up")
        zhongshus = [zs0, zs1]

        # 有 num_segments → C段覆盖
        moves = moves_from_zhongshus(zhongshus, num_segments=11)
        assert moves[0].seg_end == 10  # 覆盖 C段 seg10

        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        assert len(divs) >= 1, "C段被覆盖后应检测到趋势背驰"
        assert divs[0].kind == "trend"
        assert divs[0].direction == "top"
