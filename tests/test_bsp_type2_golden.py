"""Type 2 买卖点 golden case 测试。

核心逻辑 [旧缠论] 第17课、第21课：
    - 2B = 1B 之后，第一个反弹段(up) 之后的第一个回调段(down) 的结束点
    - 2S = 1S 之后，第一个回调段(down) 之后的第一个反弹段(up) 的结束点
    - confirmed 来自覆盖该段的 Move.settled（maimai #2 语义）

Type 2 依赖 Type 1 先被检出。Type 1 触发条件：
    - 至少 2 个 Zhongshu
    - 至少 1 个 trend Move，且 m.zs_start <= div.center_idx <= m.zs_end
    - 至少 1 个 trend Divergence，且 div.center_idx < len(zhongshus)
    - div.direction="bottom" → 1B (buy), div.direction="top" → 1S (sell)
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


def _make_trend_move(
    direction: Literal["up", "down"],
    seg_start: int,
    seg_end: int,
    zs_start: int,
    zs_end: int,
    settled: bool,
) -> Move:
    """构造最小 trend Move。"""
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
        low=50.0,
        first_seg_s0=0,
        last_seg_s1=10,
    )


def _make_divergence(
    direction: Literal["top", "bottom"],
    center_idx: int,
    seg_c_start: int,
    seg_c_end: int,
    confirmed: bool,
) -> Divergence:
    """构造最小 trend Divergence。"""
    return Divergence(
        kind="trend",
        direction=direction,
        level_id=1,
        seg_a_start=0,
        seg_a_end=1,
        seg_c_start=seg_c_start,
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
# Type 2 Detection — Golden Cases
# ═══════════════════════════════════════════════════════════


class TestType2Detection:
    """Type 2 买卖点检测 golden case。"""

    # ── 1. test_type2_buy_standard ──────────────────────────

    def test_type2_buy_standard(self):
        """标准 2B 检测。

        结构：
        - seg[0]: 背驰段(down) — Type 1 Buy 所在段 (seg_c_end=0)
        - seg[1]: 反弹段(up)
        - seg[2]: 回调段(down) — Type 2 Buy 所在段

        Type 1 触发条件：
        - 2 个 Zhongshu（center_idx=1 指向第二个）
        - 1 个 trend Move 覆盖 zs_start=0..zs_end=1
        - 1 个 trend Divergence direction="bottom", center_idx=1
        """
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
                direction="down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 1, f"应产生恰好 1 个 Type 2 Buy，实际: {type2_bsps}"

        t2 = type2_bsps[0]
        assert t2.side == "buy"
        assert t2.seg_idx == 2
        assert t2.price == 48.0  # seg[2].low

    # ── 2. test_type2_sell_standard ─────────────────────────

    def test_type2_sell_standard(self):
        """标准 2S 检测。

        结构：
        - seg[0]: 背驰段(up) — Type 1 Sell 所在段 (seg_c_end=0)
        - seg[1]: 回调段(down)
        - seg[2]: 反弹段(up) — Type 2 Sell 所在段

        Type 1 触发条件：
        - 2 个 Zhongshu
        - 1 个 trend Move direction="up"
        - 1 个 trend Divergence direction="top"
        """
        segments = [
            _Seg(direction="up", high=80.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=75.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=78.0, low=58.0, i0=10, i1=15),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=60.0, zg=70.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=62.0, zg=72.0),
        ]

        moves = [
            _make_trend_move(
                direction="up",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="top",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 1, f"应产生恰好 1 个 Type 2 Sell，实际: {type2_bsps}"

        t2 = type2_bsps[0]
        assert t2.side == "sell"
        assert t2.seg_idx == 2
        assert t2.price == 78.0  # seg[2].high

    # ── 3. test_type2_no_rebound_after_buy ──────────────────

    def test_type2_no_rebound_after_buy(self):
        """1B 之后没有反弹段 → 不产生 2B。

        只有 seg[0]=背驰段(down)，无后续段。
        Type 1 Buy 在 seg_idx=0 产生，但没有 up 段可作为反弹 → 无 Type 2。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 0, f"不应产生 Type 2 Buy，实际: {type2_bsps}"

    # ── 4. test_type2_no_callback_after_rebound ─────────────

    def test_type2_no_callback_after_rebound(self):
        """1B 之后有反弹但没有回调 → 不产生 2B。

        seg[0]=背驰段(down), seg[1]=反弹段(up)，无第三段。
        Type 1 Buy 在 seg_idx=0 → 找到反弹 seg[1] → 但无 down 段回调 → 无 Type 2。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 0, f"不应产生 Type 2 Buy，实际: {type2_bsps}"

    # ── 5. test_type2_confirmed_from_move_settled ───────────

    def test_type2_confirmed_from_move_settled(self):
        """回调段被一个 settled=True 的 Move 覆盖 → confirmed=True。

        结构同标准 2B，但额外增加一个覆盖回调段(seg[2])的 settled Move。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(direction="down", high=62.0, low=48.0, i0=10, i1=15),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        # 原始 trend Move（触发 Type 1）
        trend_move = _make_trend_move(
            direction="down",
            seg_start=0,
            seg_end=4,
            zs_start=0,
            zs_end=1,
            settled=True,
        )

        # 额外 Move 覆盖回调段 seg[2]，settled=True
        callback_move = Move(
            kind="consolidation",
            direction="up",
            seg_start=1,
            seg_end=2,
            zs_start=0,
            zs_end=0,
            zs_count=1,
            settled=True,
            high=65.0,
            low=48.0,
            first_seg_s0=0,
            last_seg_s1=10,
        )

        moves = [trend_move, callback_move]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 1, f"应产生 1 个 Type 2 Buy，实际: {type2_bsps}"
        assert type2_bsps[0].confirmed is True

    # ── 6. test_type2_no_move_covers_callback ───────────────

    def test_type2_no_move_covers_callback(self):
        """回调段无 Move 覆盖 → confirmed=False。

        结构同标准 2B，但 trend Move 仅覆盖到 seg_end=0（不覆盖回调段 seg[2]）。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(direction="down", high=62.0, low=48.0, i0=10, i1=15),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        # trend Move 只覆盖 seg[0]，不覆盖回调段 seg[2]
        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=0,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) == 1, f"应产生 1 个 Type 2 Buy，实际: {type2_bsps}"
        assert type2_bsps[0].confirmed is False
