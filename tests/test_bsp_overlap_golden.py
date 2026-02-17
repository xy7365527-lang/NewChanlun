"""2B+3B 重合检测 golden case 测试。

[旧缠论] 第21课：V型反转时 2B 与 3B 可在同一 seg_idx 上重合。
当 Type 2 和 Type 3 的 (seg_idx, side, level_id) 三元组匹配时，
两侧均标记 overlaps_with。
"""

from dataclasses import dataclass
from typing import Literal

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu


# ── 辅助 Segment stub ──────────────────────────────────────


@dataclass
class _Seg:
    """最小化 Segment stub。"""

    direction: str
    high: float
    low: float
    i0: int = 0
    i1: int = 0
    s0: int = 0
    s1: int = 0
    confirmed: bool = False


# ── 辅助构造器 ────────────────────────────────────────


def _make_trend_move(
    direction: Literal["up", "down"],
    seg_start: int,
    seg_end: int,
    zs_start: int,
    zs_end: int,
    settled: bool = True,
) -> Move:
    return Move(
        kind="trend",
        direction=direction,
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=zs_start,
        zs_end=zs_end,
        zs_count=2,
        settled=settled,
        high=100.0,
        low=40.0,
        first_seg_s0=0,
        last_seg_s1=25,
    )


def _make_divergence(
    direction: Literal["top", "bottom"],
    center_idx: int,
    seg_c_end: int,
    confirmed: bool = True,
) -> Divergence:
    return Divergence(
        kind="trend",
        direction=direction,
        level_id=1,
        seg_a_start=0,
        seg_a_end=1,
        seg_c_start=seg_c_end,
        seg_c_end=seg_c_end,
        center_idx=center_idx,
        force_a=100.0,
        force_c=80.0,
        confirmed=confirmed,
    )


def _make_zhongshu(
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    settled: bool = True,
    break_direction: str = "",
    break_seg: int = -1,
) -> Zhongshu:
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
# 2B+3B Overlap Detection
# ═══════════════════════════════════════════════════════════


class TestOverlapDetection:
    """2B+3B 重合检测。"""

    def test_overlap_2b_3b_same_seg(self):
        """V型反转：2B 和 3B 在同一 seg 重合。

        结构：
        - seg[0]: down (背驰段, 1B 在此)
        - seg[1]: up   (反弹段 / 中枢突破段)
        - seg[2]: down (回调段 = 2B; 同时也是中枢回试段, low > ZG = 3B)

        Type 1 触发：div→bottom, center_idx=1, seg_c_end=0
        Type 2 触发：1B→rebound(seg[1])→callback(seg[2]) → 2B at seg[2]
        Type 3 触发：zs[0].break_direction="up", break_seg=1,
                     seg[2].low=66 > zs[0].zg=60 → 3B at seg[2]
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=75.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=72.0, low=66.0, i0=10, i1=15),
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=50.0, zg=60.0,
                settled=True, break_direction="up", break_seg=1,
            ),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=4,
                zs_start=0, zs_end=1,
            ),
        ]

        divergences = [
            _make_divergence(direction="bottom", center_idx=1, seg_c_end=0),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type2_bsps) >= 1, f"应有 Type 2: {bsps}"
        assert len(type3_bsps) >= 1, f"应有 Type 3: {bsps}"

        # 核心断言：重合标记
        t2 = type2_bsps[0]
        t3 = type3_bsps[0]
        assert t2.seg_idx == t3.seg_idx == 2
        assert t2.overlaps_with == "type3"
        assert t3.overlaps_with == "type2"

    def test_no_overlap_different_seg(self):
        """Type 2 和 Type 3 在不同 seg → 无重合标记。

        结构：
        - seg[0]: down (1B)
        - seg[1]: up   (反弹)
        - seg[2]: down (2B at seg[2])
        - seg[3]: up   (中枢突破段, break_seg=3)
        - seg[4]: down (3B at seg[4], low > ZG)

        Type 2 at seg[2], Type 3 at seg[4] → 不同 seg，无重合。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(direction="down", high=62.0, low=48.0, i0=10, i1=15),
            _Seg(direction="up", high=80.0, low=62.0, i0=15, i1=20),
            _Seg(direction="down", high=78.0, low=72.0, i0=20, i1=25),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(
                seg_start=2, seg_end=4, zd=52.0, zg=62.0,
                settled=True, break_direction="up", break_seg=3,
            ),
        ]

        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=4,
                zs_start=0, zs_end=1,
            ),
        ]

        divergences = [
            _make_divergence(direction="bottom", center_idx=1, seg_c_end=0),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type2_bsps) >= 1
        assert len(type3_bsps) >= 1
        assert type2_bsps[0].seg_idx != type3_bsps[0].seg_idx

        # 无重合标记
        assert type2_bsps[0].overlaps_with is None
        assert type3_bsps[0].overlaps_with is None

    def test_no_overlap_type2_only(self):
        """只有 Type 2，无 Type 3 → 无重合标记。"""
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(direction="down", high=62.0, low=48.0, i0=10, i1=15),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=4,
                zs_start=0, zs_end=1,
            ),
        ]

        divergences = [
            _make_divergence(direction="bottom", center_idx=1, seg_c_end=0),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        type3_bsps = [b for b in bsps if b.kind == "type3"]

        assert len(type2_bsps) >= 1
        assert len(type3_bsps) == 0
        assert type2_bsps[0].overlaps_with is None
