"""级别递归引擎 (RecursiveLevelEngine) — 单元测试

覆盖：
  1. 空输入（无 settled Move）→ 空快照
  2. 不足 3 个 settled Move → 无中枢
  3. 3 个重叠 settled Move → 候选中枢 + 候选走势
  4. 4 个 settled Move（第4个突破）→ 中枢 settled
  5. 引擎 reset 后恢复初始状态
  6. 增量处理：多轮 snapshot 产生正确的域事件
  7. diff_level_zhongshu: invalidate 事件
  8. diff_level_moves: invalidate 事件
  9. level_id 正确透传
  10. 只过滤 settled=True 的 Move
"""

from __future__ import annotations

from typing import Literal

import pytest

from newchan.a_move_v1 import Move
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_engine import RecursiveLevelEngine
from newchan.core.recursion.recursive_level_state import (
    RecursiveLevelSnapshot,
    diff_level_moves,
    diff_level_zhongshu,
)
from newchan.events import (
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)


# ── helpers ──


def _move(
    idx: int,
    *,
    kind: Literal["consolidation", "trend"] = "consolidation",
    direction: Literal["up", "down"] = "up",
    high: float = 10.0,
    low: float = 5.0,
    settled: bool = True,
    seg_start: int = -1,
    seg_end: int = -1,
) -> Move:
    """构造测试用 Move，seg_start/seg_end 默认等于 idx。"""
    s0 = seg_start if seg_start >= 0 else idx * 2
    s1 = seg_end if seg_end >= 0 else idx * 2 + 1
    return Move(
        kind=kind,
        direction=direction,
        seg_start=s0,
        seg_end=s1,
        zs_start=0,
        zs_end=0,
        zs_count=1,
        settled=settled,
        high=high,
        low=low,
        first_seg_s0=s0,
        last_seg_s1=s1,
    )


def _snap(moves: list[Move], bar_idx: int = 100, bar_ts: float = 1000.0) -> MoveSnapshot:
    """构造 MoveSnapshot。"""
    return MoveSnapshot(bar_idx=bar_idx, bar_ts=bar_ts, moves=moves, events=[])


# ── 引擎基本测试 ──


class TestRecursiveLevelEngineBasic:
    """基本功能测试。"""

    def test_empty_input(self) -> None:
        """无 Move → 空快照。"""
        engine = RecursiveLevelEngine(level_id=2)
        snap = engine.process_move_snapshot(_snap([]))
        assert snap.zhongshus == []
        assert snap.moves == []
        assert snap.zhongshu_events == []
        assert snap.move_events == []
        assert snap.level_id == 2

    def test_no_settled_moves(self) -> None:
        """只有 unsettled Move → 全部被过滤，空结果。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=False, high=10, low=5),
            _move(1, settled=False, high=12, low=6),
            _move(2, settled=False, high=11, low=7),
        ]
        snap = engine.process_move_snapshot(_snap(moves))
        assert snap.zhongshus == []
        assert snap.moves == []

    def test_fewer_than_3_settled(self) -> None:
        """不足 3 个 settled Move → 无法形成中枢。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
        ]
        snap = engine.process_move_snapshot(_snap(moves))
        assert snap.zhongshus == []
        assert snap.moves == []

    def test_level_id_property(self) -> None:
        """level_id 属性正确返回。"""
        engine = RecursiveLevelEngine(level_id=3)
        assert engine.level_id == 3

    def test_event_seq_starts_at_zero(self) -> None:
        """初始 event_seq = 0。"""
        engine = RecursiveLevelEngine(level_id=2)
        assert engine.event_seq == 0


# ── 中枢形成测试 ──


class TestZhongshuFormation:
    """三个重叠 Move 应该形成中枢。"""

    def test_three_overlapping_moves_form_zhongshu(self) -> None:
        """3 个 settled Move 有价格重叠 → 形成 1 个候选中枢。

        Move 0: [5, 10], Move 1: [6, 12], Move 2: [7, 11]
        ZD = max(5,6,7) = 7, ZG = min(10,12,11) = 10, ZG > ZD ✓
        """
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snap = engine.process_move_snapshot(_snap(moves))

        assert len(snap.zhongshus) == 1
        zs = snap.zhongshus[0]
        assert zs.zd == 7.0
        assert zs.zg == 10.0
        assert zs.settled is False  # 没有突破，未闭合
        assert zs.level_id == 2

        # 应产生 ZhongshuCandidateV1 事件
        zs_events = [e for e in snap.zhongshu_events if isinstance(e, ZhongshuCandidateV1)]
        assert len(zs_events) == 1
        assert zs_events[0].zd == 7.0
        assert zs_events[0].zg == 10.0

    def test_breakthrough_settles_zhongshu(self) -> None:
        """第 4 个 Move 突破 → 中枢 settled。

        Move 3: low=11 > ZG=10 → 向上突破。
        """
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
            _move(3, settled=True, high=15, low=11, direction="up"),
        ]
        snap = engine.process_move_snapshot(_snap(moves))

        assert len(snap.zhongshus) == 1
        zs = snap.zhongshus[0]
        assert zs.settled is True
        assert zs.break_direction == "up"

        # 应产生 candidate + settle 事件
        zs_candidates = [e for e in snap.zhongshu_events if isinstance(e, ZhongshuCandidateV1)]
        zs_settles = [e for e in snap.zhongshu_events if isinstance(e, ZhongshuSettleV1)]
        assert len(zs_candidates) == 1
        assert len(zs_settles) == 1

    def test_no_overlap_no_zhongshu(self) -> None:
        """3 个不重叠 Move → 无中枢。

        Move 0: [1, 3], Move 1: [5, 7], Move 2: [9, 11]
        ZD = max(1,5,9) = 9, ZG = min(3,7,11) = 3, ZG < ZD ✗
        """
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=3, low=1, direction="up"),
            _move(1, settled=True, high=7, low=5, direction="down"),
            _move(2, settled=True, high=11, low=9, direction="up"),
        ]
        snap = engine.process_move_snapshot(_snap(moves))
        assert snap.zhongshus == []


# ── 走势类型形成测试 ──


class TestMoveFormation:
    """从 LevelZhongshu 生成走势类型。"""

    def test_settled_zhongshu_produces_move(self) -> None:
        """1 个 settled 中枢 → 1 个 consolidation Move（unsettled）。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
            _move(3, settled=True, high=15, low=11, direction="up"),
        ]
        snap = engine.process_move_snapshot(_snap(moves))

        # 1 个 settled 中枢 → moves_from_level_zhongshus 产生 1 个 move
        assert len(snap.moves) == 1
        m = snap.moves[0]
        assert m.kind == "consolidation"
        assert m.settled is False  # 最后一个总是 unsettled


# ── 增量处理和 diff 测试 ──


class TestIncrementalProcessing:
    """多轮处理产生正确的增量事件。"""

    def test_second_round_no_change_no_events(self) -> None:
        """相同输入第二次处理 → 无新事件。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=True, high=11, low=7),
        ]
        snap1 = engine.process_move_snapshot(_snap(moves, bar_idx=100))
        # 首轮有事件
        assert len(snap1.zhongshu_events) > 0

        snap2 = engine.process_move_snapshot(_snap(moves, bar_idx=101, bar_ts=1001.0))
        # 相同输入 → 无新事件
        assert snap2.zhongshu_events == []
        assert snap2.move_events == []

    def test_event_seq_accumulates(self) -> None:
        """event_seq 跨轮次累计。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=True, high=11, low=7),
        ]
        engine.process_move_snapshot(_snap(moves))
        seq_after_1 = engine.event_seq
        assert seq_after_1 > 0

        # 加入突破 Move → 产生新事件
        moves2 = moves + [_move(3, settled=True, high=15, low=11)]
        engine.process_move_snapshot(_snap(moves2, bar_idx=101, bar_ts=1001.0))
        seq_after_2 = engine.event_seq
        assert seq_after_2 > seq_after_1


# ── reset 测试 ──


class TestReset:
    """引擎 reset 恢复初始状态。"""

    def test_reset_clears_state(self) -> None:
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=True, high=11, low=7),
        ]
        engine.process_move_snapshot(_snap(moves))
        assert engine.event_seq > 0
        assert len(engine.current_zhongshus) > 0

        engine.reset()
        assert engine.event_seq == 0
        assert engine.current_zhongshus == []
        assert engine.current_moves == []


# ── diff 函数直接测试 ──


class TestDiffLevelZhongshu:
    """diff_level_zhongshu 直接测试。"""

    def _make_zs(
        self,
        *,
        zd: float = 7.0,
        zg: float = 10.0,
        comp_start: int = 0,
        comp_end: int = 2,
        settled: bool = False,
        level_id: int = 2,
    ) -> LevelZhongshu:
        return LevelZhongshu(
            zd=zd,
            zg=zg,
            comp_start=comp_start,
            comp_end=comp_end,
            comp_count=comp_end - comp_start + 1,
            settled=settled,
            break_comp=-1,
            break_direction="",
            gg=zg + 2,
            dd=zd - 2,
            level_id=level_id,
        )

    def test_new_zhongshu_candidate(self) -> None:
        """prev=[], curr=[zs] → ZhongshuCandidateV1。"""
        zs = self._make_zs()
        events = diff_level_zhongshu([], [zs], bar_idx=10, bar_ts=100.0)
        assert len(events) == 1
        assert isinstance(events[0], ZhongshuCandidateV1)

    def test_zhongshu_settle_upgrade(self) -> None:
        """同身份 zhongshu 从 unsettled → settled → ZhongshuSettleV1。"""
        zs_old = self._make_zs(settled=False)
        zs_new = LevelZhongshu(
            zd=zs_old.zd,
            zg=zs_old.zg,
            comp_start=zs_old.comp_start,
            comp_end=4,
            comp_count=5,
            settled=True,
            break_comp=5,
            break_direction="up",
            gg=zs_old.gg,
            dd=zs_old.dd,
            level_id=zs_old.level_id,
        )
        events = diff_level_zhongshu([zs_old], [zs_new], bar_idx=10, bar_ts=100.0)
        settle_events = [e for e in events if isinstance(e, ZhongshuSettleV1)]
        assert len(settle_events) == 1

    def test_zhongshu_invalidate(self) -> None:
        """prev=[zs], curr=[] → ZhongshuInvalidateV1。"""
        zs = self._make_zs()
        events = diff_level_zhongshu([zs], [], bar_idx=10, bar_ts=100.0)
        assert len(events) == 1
        assert isinstance(events[0], ZhongshuInvalidateV1)

    def test_identical_no_events(self) -> None:
        """prev == curr → 空事件。"""
        zs = self._make_zs()
        events = diff_level_zhongshu([zs], [zs], bar_idx=10, bar_ts=100.0)
        assert events == []


class TestDiffLevelMoves:
    """diff_level_moves 直接测试。"""

    def _make_move(self, *, seg_start: int = 0, settled: bool = False) -> Move:
        return Move(
            kind="consolidation",
            direction="up",
            seg_start=seg_start,
            seg_end=seg_start + 2,
            zs_start=0,
            zs_end=0,
            zs_count=1,
            settled=settled,
            high=10.0,
            low=5.0,
        )

    def test_new_move_candidate(self) -> None:
        """prev=[], curr=[m] → MoveCandidateV1。"""
        m = self._make_move()
        events = diff_level_moves([], [m], bar_idx=10, bar_ts=100.0)
        assert len(events) == 1
        assert isinstance(events[0], MoveCandidateV1)

    def test_move_settle_upgrade(self) -> None:
        """同身份 move unsettled → settled → MoveSettleV1。"""
        m_old = self._make_move(settled=False)
        m_new = Move(
            kind=m_old.kind,
            direction=m_old.direction,
            seg_start=m_old.seg_start,
            seg_end=m_old.seg_end,
            zs_start=m_old.zs_start,
            zs_end=m_old.zs_end,
            zs_count=m_old.zs_count,
            settled=True,
            high=m_old.high,
            low=m_old.low,
        )
        events = diff_level_moves([m_old], [m_new], bar_idx=10, bar_ts=100.0)
        settle_events = [e for e in events if isinstance(e, MoveSettleV1)]
        assert len(settle_events) == 1

    def test_move_invalidate(self) -> None:
        """prev=[m], curr=[] → MoveInvalidateV1。"""
        m = self._make_move()
        events = diff_level_moves([m], [], bar_idx=10, bar_ts=100.0)
        assert len(events) == 1
        assert isinstance(events[0], MoveInvalidateV1)


# ── settled 过滤测试 ──


class TestSettledFiltering:
    """只有 settled=True 的 Move 参与递归构造。"""

    def test_mixed_settled_unsettled(self) -> None:
        """3 个 settled + 1 个 unsettled → 中枢仅从 settled 构造。"""
        engine = RecursiveLevelEngine(level_id=2)
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=True, high=11, low=7),
            _move(3, settled=False, high=20, low=15),  # 不参与
        ]
        snap = engine.process_move_snapshot(_snap(moves))

        # 中枢应由前 3 个 settled Move 构成
        assert len(snap.zhongshus) == 1
        zs = snap.zhongshus[0]
        assert zs.zd == 7.0
        assert zs.zg == 10.0
        # 第 4 个 unsettled 不参与，不会造成突破
        assert zs.settled is False


# ── Snapshot 数据类测试 ──


class TestRecursiveLevelSnapshot:
    """RecursiveLevelSnapshot 数据类。"""

    def test_snapshot_fields(self) -> None:
        snap = RecursiveLevelSnapshot(
            bar_idx=42,
            bar_ts=12345.0,
            level_id=3,
            zhongshus=[],
            moves=[],
            zhongshu_events=[],
            move_events=[],
        )
        assert snap.bar_idx == 42
        assert snap.bar_ts == 12345.0
        assert snap.level_id == 3
