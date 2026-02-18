"""BSP 否定传播测试 — 上游对象否定 → BSP 正确消失/存活。

核心原则（005a 禁止定理 + 005b 语法规则）：
- 对象否定对象，唯一来源是内在否定或外部对象生成
- diff 层通过 prev/curr 差集自动实现：上游消失 → BSP 从 curr 消失 → Invalidate

测试场景：
1. Move 否定 → type1 消失
2. Move 否定 → type2 连带消失（type2 继承 type1 前提）
3. Zhongshu 否定 → type3 消失
4. 部分否定（仅一个 BSP 消失，其余存活）
5. 否定后新生（旧 BSP 消失 + 新身份 BSP 出现）
"""

from __future__ import annotations

from typing import Literal

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointInvalidateV1,
)

_BAR_IDX = 200
_BAR_TS = 1700000000.0


def _mk_bsp(
    seg_idx: int = 0,
    kind: Literal["type1", "type2", "type3"] = "type1",
    side: Literal["buy", "sell"] = "buy",
    level_id: int = 1,
    price: float = 50.0,
    confirmed: bool = False,
    settled: bool = False,
    overlaps_with: Literal["type2", "type3"] | None = None,
    move_seg_start: int = 0,
    center_seg_start: int = 0,
    bar_idx: int = 10,
) -> BuySellPoint:
    """快速构造一个 BuySellPoint（最小必填字段）。"""
    return BuySellPoint(
        kind=kind,
        side=side,
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=move_seg_start,
        divergence_key=None,
        center_zd=40.0,
        center_zg=60.0,
        center_seg_start=center_seg_start,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=settled,
        overlaps_with=overlaps_with,
    )


def _diff(
    prev: list[BuySellPoint],
    curr: list[BuySellPoint],
    seq_start: int = 0,
) -> list:
    return diff_buysellpoints(
        prev, curr, bar_idx=_BAR_IDX, bar_ts=_BAR_TS, seq_start=seq_start
    )


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 否定传播测试
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestMoveNegation:
    """Move 否定 → BSP 消失。"""

    def test_move_negation_type1_disappears(self):
        """Move 消失 → 依赖该 Move 的 type1 BSP 被 invalidate。"""
        bsp_type1 = _mk_bsp(seg_idx=5, kind="type1", side="buy", move_seg_start=3)
        events = _diff([bsp_type1], [])
        assert len(events) == 1
        e = events[0]
        assert isinstance(e, BuySellPointInvalidateV1)
        assert e.kind == "type1"
        assert e.seg_idx == 5

    def test_move_negation_type2_cascade(self):
        """Move 消失 → type1 + type2 同时消失（type2 继承 type1 前提）。"""
        bsp_type1 = _mk_bsp(seg_idx=5, kind="type1", side="buy")
        bsp_type2 = _mk_bsp(seg_idx=5, kind="type2", side="buy")
        events = _diff([bsp_type1, bsp_type2], [])
        assert len(events) == 2
        inv_kinds = {e.kind for e in events}
        assert inv_kinds == {"type1", "type2"}
        for e in events:
            assert isinstance(e, BuySellPointInvalidateV1)
            assert e.seg_idx == 5


class TestZhongshuNegation:
    """Zhongshu 否定 → type3 消失。"""

    def test_zhongshu_negation_type3_disappears(self):
        """Zhongshu 消失 → 依赖该 Zhongshu 的 type3 BSP 被 invalidate。"""
        bsp_type3 = _mk_bsp(seg_idx=7, kind="type3", side="sell", center_seg_start=4)
        events = _diff([bsp_type3], [])
        assert len(events) == 1
        e = events[0]
        assert isinstance(e, BuySellPointInvalidateV1)
        assert e.kind == "type3"
        assert e.seg_idx == 7


class TestPartialNegation:
    """部分否定 — 仅被否定的 BSP 消失，其余存活。"""

    def test_one_negated_others_survive(self):
        """三个 BSP 中一个消失，另外两个不受影响。"""
        bsp_a = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        bsp_b = _mk_bsp(seg_idx=5, kind="type2", side="sell")
        bsp_c = _mk_bsp(seg_idx=7, kind="type3", side="buy")

        # bsp_b 的上游被否定，从 curr 中消失
        events = _diff([bsp_a, bsp_b, bsp_c], [bsp_a, bsp_c])

        assert len(events) == 1
        e = events[0]
        assert isinstance(e, BuySellPointInvalidateV1)
        assert e.kind == "type2"
        assert e.seg_idx == 5

    def test_two_negated_one_survives(self):
        """三个 BSP 中两个消失，一个存活。"""
        bsp_a = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        bsp_b = _mk_bsp(seg_idx=5, kind="type2", side="sell")
        bsp_c = _mk_bsp(seg_idx=7, kind="type3", side="buy")

        # bsp_a 和 bsp_c 的上游被否定
        events = _diff([bsp_a, bsp_b, bsp_c], [bsp_b])

        assert len(events) == 2
        inv_kinds = {e.kind for e in events}
        assert inv_kinds == {"type1", "type3"}
        for e in events:
            assert isinstance(e, BuySellPointInvalidateV1)


class TestNegationAndBirth:
    """否定 + 新生：旧 BSP 消失，新身份 BSP 同时出现。"""

    def test_invalidate_old_candidate_new(self):
        """旧 BSP 被否定 + 新 BSP 出现 → invalidate + candidate。"""
        old_bsp = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        new_bsp = _mk_bsp(seg_idx=9, kind="type3", side="sell")

        events = _diff([old_bsp], [new_bsp])

        assert len(events) == 2
        inv = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        cand = [e for e in events if isinstance(e, BuySellPointCandidateV1)]
        assert len(inv) == 1
        assert len(cand) == 1
        assert inv[0].seg_idx == 3
        assert inv[0].kind == "type1"
        assert cand[0].seg_idx == 9
        assert cand[0].kind == "type3"

    def test_causal_ordering_invalidate_before_candidate(self):
        """因果序：invalidate 事件排在 candidate 之前。"""
        old_bsp = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        new_bsp = _mk_bsp(seg_idx=9, kind="type2", side="sell")

        events = _diff([old_bsp], [new_bsp])

        types = [type(e).__name__ for e in events]
        inv_idx = types.index("BuySellPointInvalidateV1")
        cand_idx = types.index("BuySellPointCandidateV1")
        assert inv_idx < cand_idx, "invalidate 必须排在 candidate 之前"

    def test_negation_and_birth_event_ids_unique(self):
        """否定 + 新生场景中 event_id 不重复。"""
        old_bsp = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        new_bsp = _mk_bsp(seg_idx=9, kind="type3", side="sell")

        events = _diff([old_bsp], [new_bsp])

        ids = [e.event_id for e in events]
        assert len(set(ids)) == len(ids), "event_id 必须唯一"

    def test_seq_monotonic_across_negation_and_birth(self):
        """否定 + 新生场景中 seq 单调递增。"""
        old_bsp = _mk_bsp(seg_idx=3, kind="type1", side="buy")
        new_bsp = _mk_bsp(seg_idx=9, kind="type3", side="sell")

        events = _diff([old_bsp], [new_bsp])

        seqs = [e.seq for e in events]
        assert seqs == sorted(seqs)
        assert len(set(seqs)) == len(seqs), "seq 不能重复"
