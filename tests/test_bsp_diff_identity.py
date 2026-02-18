"""BspDiff 身份键映射 diff 测试 — 暴露位置对位 diff 缺陷。

关键场景：
- 中间删除：列表中间的 BSP 消失时，后续 BSP 不应被误杀
- 中间插入：新 BSP 出现在列表中间，已有 BSP 不应被误杀
- I27 验证：同身份 BSP 不能先 invalidate 后 candidate（不复活）
- 否定传播：上游否定只影响依赖的 BSP，不影响独立 BSP

谱系引用：maimai_rules_v1.md §5（身份键查找规范）、§7（I27 不变量）
"""

from __future__ import annotations

from typing import Literal

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
)

_BAR_IDX = 200
_BAR_TS = 1700100000.0


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
    """快速构造 BuySellPoint。"""
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


def _diff(prev: list[BuySellPoint], curr: list[BuySellPoint]) -> list:
    return diff_buysellpoints(
        prev, curr, bar_idx=_BAR_IDX, bar_ts=_BAR_TS, seq_start=0
    )


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 中间删除场景
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestMidListDeletion:
    """中间 BSP 消失时，后续 BSP 不应被误杀。"""

    def test_middle_bsp_removed_tail_survives(self):
        """[A, B, C] → [A, C]：只 invalidate B，C 不受影响。

        位置对位 diff 的错误行为：
        - prev[1]=B vs curr[1]=C → 身份不同 → invalidate B ✓
        - prev[2]=C vs curr[2]=不存在 → invalidate C ✗ 误杀！
        - curr[1]=C 被当作全新 → candidate C ✗ 幽灵重生！

        正确行为：只 invalidate B，C 保持不变。
        """
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy", confirmed=True)
        b = _mk_bsp(seg_idx=3, kind="type1", side="sell")
        c = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=True)

        prev = [a, b, c]
        curr = [a, c]

        events = _diff(prev, curr)

        # 只应有一个 invalidate 事件（B 消失）
        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 1, (
            f"应只 invalidate 1 个 BSP(seg_idx=3)，实际 invalidate {len(inv_events)} 个"
        )
        assert inv_events[0].seg_idx == 3

        # C 不应出现任何事件（不应被 invalidate 也不应被 re-candidate）
        c_events = [e for e in events if e.seg_idx == 7]
        assert len(c_events) == 0, (
            f"BSP(seg_idx=7) 未变更，不应产生事件，实际产生 {len(c_events)} 个"
        )

    def test_first_bsp_removed_rest_survives(self):
        """[A, B, C] → [B, C]：只 invalidate A，B 和 C 不受影响。"""
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy")
        b = _mk_bsp(seg_idx=3, kind="type2", side="buy", confirmed=True)
        c = _mk_bsp(seg_idx=5, kind="type3", side="sell")

        prev = [a, b, c]
        curr = [b, c]

        events = _diff(prev, curr)

        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 1
        assert inv_events[0].seg_idx == 1

        # B 和 C 不应产生任何事件
        bc_events = [e for e in events if e.seg_idx in (3, 5)]
        assert len(bc_events) == 0

    def test_multiple_middle_removed(self):
        """[A, B, C, D, E] → [A, E]：invalidate B, C, D，E 不受影响。"""
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy", confirmed=True)
        b = _mk_bsp(seg_idx=3, kind="type1", side="sell")
        c = _mk_bsp(seg_idx=5, kind="type2", side="buy")
        d = _mk_bsp(seg_idx=7, kind="type3", side="sell")
        e = _mk_bsp(seg_idx=9, kind="type3", side="buy", confirmed=True)

        prev = [a, b, c, d, e]
        curr = [a, e]

        events = _diff(prev, curr)

        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 3
        inv_segs = sorted(e.seg_idx for e in inv_events)
        assert inv_segs == [3, 5, 7]

        # A 和 E 不应产生事件
        ae_events = [e for e in events if e.seg_idx in (1, 9)]
        assert len(ae_events) == 0


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 中间插入场景
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestMidListInsertion:
    """新 BSP 插入到列表中间时，已有 BSP 不应被误杀。"""

    def test_insert_middle_existing_survives(self):
        """[A, C] → [A, B, C]：新增 B，A 和 C 不受影响。

        位置对位 diff 的错误行为：
        - prev[1]=C vs curr[1]=B → 身份不同 → invalidate C ✗ 误杀！
        - curr[1]=B 是全新 → candidate B ✓
        - curr[2]=C 无 prev 对应 → candidate C ✗ 幽灵重生！

        正确行为：只 candidate B。
        """
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy", confirmed=True)
        b_new = _mk_bsp(seg_idx=3, kind="type2", side="buy")
        c = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=True)

        prev = [a, c]
        curr = [a, b_new, c]

        events = _diff(prev, curr)

        # 不应有 invalidate 事件
        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 0, (
            f"无 BSP 消失，不应有 invalidate，实际有 {len(inv_events)} 个"
        )

        # 应只有一个 candidate 事件（B 新增）
        cand_events = [e for e in events if isinstance(e, BuySellPointCandidateV1)]
        assert len(cand_events) == 1
        assert cand_events[0].seg_idx == 3

    def test_insert_at_beginning(self):
        """[B, C] → [A, B, C]：新增 A 到头部。"""
        a_new = _mk_bsp(seg_idx=1, kind="type1", side="sell")
        b = _mk_bsp(seg_idx=3, kind="type2", side="buy", confirmed=True)
        c = _mk_bsp(seg_idx=5, kind="type3", side="buy")

        prev = [b, c]
        curr = [a_new, b, c]

        events = _diff(prev, curr)

        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 0

        cand_events = [e for e in events if isinstance(e, BuySellPointCandidateV1)]
        assert len(cand_events) == 1
        assert cand_events[0].seg_idx == 1


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# I27 不变量：invalidate 后同身份不复活
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestI27NoResurrection:
    """同身份 BSP 不能在同一轮 diff 中先 invalidate 后 candidate。"""

    def test_no_invalidate_then_candidate_same_identity(self):
        """中间删除导致位移时，存活的 BSP 不应被先杀后生。

        [A, B, C] → [A, C]
        位置对位会产生：invalidate(C) + candidate(C) → 违反 I27
        身份键 diff 不会对 C 产生任何事件。
        """
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy")
        b = _mk_bsp(seg_idx=3, kind="type1", side="sell")
        c = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=True)

        prev = [a, b, c]
        curr = [a, c]

        events = _diff(prev, curr)

        # 收集 C 的身份键 (seg_idx=7, kind=type3, side=buy, level_id=1)
        c_inv = [
            e for e in events
            if isinstance(e, BuySellPointInvalidateV1)
            and e.seg_idx == 7 and e.kind == "type3" and e.side == "buy"
        ]
        c_cand = [
            e for e in events
            if isinstance(e, BuySellPointCandidateV1)
            and e.seg_idx == 7 and e.kind == "type3" and e.side == "buy"
        ]

        # C 不应被 invalidate（它仍然存在）
        assert len(c_inv) == 0, "存活的 BSP 不应被 invalidate"
        # C 也不应被 re-candidate（它没变化）
        assert len(c_cand) == 0, "未变更的 BSP 不应被 re-candidate"

    def test_confirmed_bsp_preserves_state_after_sibling_removal(self):
        """已确认的 BSP 在兄弟消失后，不应丢失 confirmed 状态。

        prev: [BSP(3, confirmed=T, settled=T), BSP(5), BSP(9, confirmed=T)]
        curr: [BSP(3, confirmed=T, settled=T), BSP(9, confirmed=T)]
        → 只 invalidate BSP(5)，BSP(9) 保持 confirmed，无事件。
        """
        a = _mk_bsp(seg_idx=3, confirmed=True, settled=True)
        b = _mk_bsp(seg_idx=5, kind="type2")
        c = _mk_bsp(seg_idx=9, kind="type3", confirmed=True)

        prev = [a, b, c]
        curr = [a, c]

        events = _diff(prev, curr)

        # 只 invalidate B
        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 1
        assert inv_events[0].seg_idx == 5

        # C 不产生任何事件（状态完全保持）
        c_events = [e for e in events if e.seg_idx == 9]
        assert len(c_events) == 0


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 混合场景：删除 + 插入 + 状态变化
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


class TestMixedChanges:
    """复合变化场景。"""

    def test_delete_and_insert_simultaneously(self):
        """[A, B, C] → [A, D, C]：B 消失 + D 新增，C 不受影响。"""
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy")
        b = _mk_bsp(seg_idx=3, kind="type1", side="sell")
        c = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=True)
        d_new = _mk_bsp(seg_idx=5, kind="type2", side="buy")

        prev = [a, b, c]
        curr = [a, d_new, c]

        events = _diff(prev, curr)

        # B 应被 invalidate
        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 1
        assert inv_events[0].seg_idx == 3

        # D 应被 candidate
        cand_events = [e for e in events if isinstance(e, BuySellPointCandidateV1)]
        assert len(cand_events) == 1
        assert cand_events[0].seg_idx == 5

        # C 不产生事件
        c_events = [e for e in events if e.seg_idx == 7]
        assert len(c_events) == 0

    def test_delete_middle_and_state_change_on_tail(self):
        """[A, B, C(unconfirmed)] → [A, C(confirmed)]：B 消失 + C confirmed 升级。"""
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy", confirmed=True)
        b = _mk_bsp(seg_idx=3, kind="type2", side="buy")
        c_old = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=False)
        c_new = _mk_bsp(seg_idx=7, kind="type3", side="buy", confirmed=True)

        prev = [a, b, c_old]
        curr = [a, c_new]

        events = _diff(prev, curr)

        # B 应被 invalidate
        inv_events = [e for e in events if isinstance(e, BuySellPointInvalidateV1)]
        assert len(inv_events) == 1
        assert inv_events[0].seg_idx == 3

        # C 应有 confirm 事件
        confirm_events = [e for e in events if isinstance(e, BuySellPointConfirmV1)]
        assert len(confirm_events) == 1
        assert confirm_events[0].seg_idx == 7

        # C 不应被 invalidate 或 re-candidate
        c_inv = [e for e in events if isinstance(e, BuySellPointInvalidateV1) and e.seg_idx == 7]
        assert len(c_inv) == 0

    def test_event_ordering_invalidate_before_candidate(self):
        """因果序：所有 invalidate 排在 candidate 之前。"""
        a = _mk_bsp(seg_idx=1, kind="type1", side="buy")
        b = _mk_bsp(seg_idx=3, kind="type1", side="sell")
        c = _mk_bsp(seg_idx=7, kind="type3", side="buy")
        d_new = _mk_bsp(seg_idx=5, kind="type2", side="sell")

        prev = [a, b, c]
        curr = [a, d_new, c]

        events = _diff(prev, curr)

        # 找 invalidate 和 candidate 的位置
        inv_positions = [
            i for i, e in enumerate(events)
            if isinstance(e, BuySellPointInvalidateV1)
        ]
        cand_positions = [
            i for i, e in enumerate(events)
            if isinstance(e, BuySellPointCandidateV1)
        ]

        if inv_positions and cand_positions:
            assert max(inv_positions) < min(cand_positions), (
                "invalidate 事件必须排在 candidate 事件之前（因果序）"
            )
