"""RecursiveStack — 多层自动递归调度器单元测试

覆盖：
  1. 空输入 → 空快照列表
  2. 不足组件 → level-2 产生空快照
  3. 3 个重叠 settled Move → level-2 形成中枢 + 走势
  4. 多层递归：level-1 → level-2 → level-3
  5. max_levels 安全阀
  6. reset 清除所有引擎
  7. 懒创建引擎
  8. 事件在各层正确产生
  9. 递归终止条件：moves < 3
  10. 增量处理：多轮 snapshot
"""

from __future__ import annotations

from typing import Literal

import pytest

from newchan.a_move_v1 import Move
from newchan.core.recursion.move_state import MoveSnapshot
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot
from newchan.events import ZhongshuCandidateV1, ZhongshuSettleV1


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
    """构造测试用 Move，seg_start/seg_end 默认按 idx 递增。"""
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


# ── 基本功能测试 ──


class TestRecursiveStackBasic:
    """基本初始化和空输入。"""

    def test_import(self) -> None:
        """RecursiveStack 可从 recursion 包导入。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        assert stack is not None

    def test_empty_input(self) -> None:
        """空 MoveSnapshot → 空快照列表（level-2 无组件，立即终止）。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        snaps = stack.process_level1_move_snapshot(_snap([]))
        # level-2 处理空输入，产生空快照，moves=0 < 3 → 终止
        assert len(snaps) == 1
        assert snaps[0].level_id == 2
        assert snaps[0].zhongshus == []
        assert snaps[0].moves == []

    def test_insufficient_settled_moves(self) -> None:
        """不足 3 个 settled Move → level-2 无中枢，递归终止。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=False, high=11, low=7),  # unsettled，不参与
        ]
        snaps = stack.process_level1_move_snapshot(_snap(moves))
        assert len(snaps) == 1
        assert snaps[0].level_id == 2
        assert snaps[0].zhongshus == []


# ── 单层递归测试 ──


class TestSingleLevelRecursion:
    """level-1 → level-2 的单层递归。"""

    def test_three_overlapping_form_zhongshu(self) -> None:
        """3 个重叠 settled Move → level-2 形成 1 个中枢候选。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snaps = stack.process_level1_move_snapshot(_snap(moves))
        assert len(snaps) == 1
        assert snaps[0].level_id == 2
        assert len(snaps[0].zhongshus) == 1
        assert snaps[0].zhongshus[0].zd == 7.0
        assert snaps[0].zhongshus[0].zg == 10.0

    def test_breakthrough_produces_move(self) -> None:
        """突破 → 中枢 settled → 产生走势类型。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
            _move(3, settled=True, high=15, low=11, direction="up"),
        ]
        snaps = stack.process_level1_move_snapshot(_snap(moves))
        assert len(snaps) == 1
        # level-2 应有 1 个 settled 中枢 + 1 个 move
        assert len(snaps[0].zhongshus) == 1
        assert snaps[0].zhongshus[0].settled is True
        assert len(snaps[0].moves) == 1

    def test_zhongshu_events_produced(self) -> None:
        """中枢形成时产生 ZhongshuCandidateV1 事件。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snaps = stack.process_level1_move_snapshot(_snap(moves))
        zs_candidates = [
            e for e in snaps[0].zhongshu_events
            if isinstance(e, ZhongshuCandidateV1)
        ]
        assert len(zs_candidates) == 1


# ── 多层递归测试 ──


class TestMultiLevelRecursion:
    """level-1 → level-2 → level-3 的多层递归。"""

    def _build_deep_moves(self) -> list[Move]:
        """构造足够多的重叠 settled Move，使 level-2 产生足够的中枢形成 level-3。

        需要：level-2 有 ≥ 2 个 settled 中枢（需要 ≥ 2 次突破）
        = 至少 3+1 + 3+1 = 8 个重叠 settled Move，最后一个 unsettled。

        设计：9 个 settled Move，交替方向，全部在 [5,12] 价格重叠区间内前几个，
        第4个突破第一个中枢，第5-7个形成第二个中枢，第8个突破第二个中枢。
        """
        # 中枢1：Move 0,1,2 重叠于 [7,10]，Move 3 向上突破
        # 中枢2：Move 4,5,6 重叠于 [13,16]，Move 7 向上突破
        # 这需要精心构造使得两层都能产生中枢
        moves = [
            # 中枢1 组件
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
            # 突破1
            _move(3, settled=True, high=16, low=11, direction="up"),
            # 中枢2 组件
            _move(4, settled=True, high=17, low=12, direction="down"),
            _move(5, settled=True, high=18, low=13, direction="up"),
            _move(6, settled=True, high=16, low=12, direction="down"),
            # 突破2
            _move(7, settled=True, high=22, low=17, direction="up"),
            # 额外 — 确保 level-2 有足够 moves 让 level-3 尝试
            _move(8, settled=True, high=24, low=18, direction="down"),
            _move(9, settled=True, high=23, low=19, direction="up"),
            _move(10, settled=False, high=25, low=20, direction="up"),
        ]
        return moves

    def test_produces_multiple_level_snapshots(self) -> None:
        """足够复杂的输入 → 至少 level-2 快照存在。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        snaps = stack.process_level1_move_snapshot(_snap(self._build_deep_moves()))
        # 至少 level-2 应该存在
        assert len(snaps) >= 1
        assert snaps[0].level_id == 2

    def test_level_ids_ascending(self) -> None:
        """快照列表中 level_id 严格递增。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        snaps = stack.process_level1_move_snapshot(_snap(self._build_deep_moves()))
        for i in range(1, len(snaps)):
            assert snaps[i].level_id == snaps[i - 1].level_id + 1


# ── max_levels 安全阀测试 ──


class TestMaxLevels:
    """max_levels 参数限制递归深度。"""

    def test_max_levels_caps_recursion(self) -> None:
        """max_levels=2 → 最多产生 level-2 快照，即使数据允许更深。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack(max_levels=2)
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snaps = stack.process_level1_move_snapshot(_snap(moves))
        # max_levels=2 → 只处理 level 2（从 level 1 到 level 2）
        assert all(s.level_id <= 2 for s in snaps)

    def test_default_max_levels_is_6(self) -> None:
        """默认 max_levels = 6。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        assert stack.max_levels == 6


# ── reset 测试 ──


class TestRecursiveStackReset:
    """reset 恢复初始状态。"""

    def test_reset_clears_engines(self) -> None:
        """reset 后 active_levels 归零。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5),
            _move(1, settled=True, high=12, low=6),
            _move(2, settled=True, high=11, low=7),
        ]
        stack.process_level1_move_snapshot(_snap(moves))
        assert stack.active_levels > 0

        stack.reset()
        assert stack.active_levels == 0

    def test_reset_then_reprocess(self) -> None:
        """reset 后重新处理 → 结果与首次一致。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snap1 = stack.process_level1_move_snapshot(_snap(moves))

        stack.reset()
        snap2 = stack.process_level1_move_snapshot(_snap(moves))

        assert len(snap1) == len(snap2)
        for s1, s2 in zip(snap1, snap2):
            assert s1.level_id == s2.level_id
            assert len(s1.zhongshus) == len(s2.zhongshus)
            assert len(s1.moves) == len(s2.moves)


# ── 懒创建测试 ──


class TestLazyCreation:
    """引擎只在需要时创建。"""

    def test_no_engines_before_processing(self) -> None:
        """初始化后无引擎。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        assert stack.active_levels == 0

    def test_engine_created_on_first_process(self) -> None:
        """首次处理后至少创建 level-2 引擎。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        stack.process_level1_move_snapshot(_snap([]))
        assert stack.active_levels >= 1


# ── 增量处理测试 ──


class TestIncrementalProcessing:
    """多轮 snapshot 的增量处理。"""

    def test_second_round_same_input_no_events(self) -> None:
        """相同输入第二次处理 → 无新事件。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        moves = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snaps1 = stack.process_level1_move_snapshot(_snap(moves, bar_idx=100))
        # 首轮有事件
        assert len(snaps1[0].zhongshu_events) > 0

        snaps2 = stack.process_level1_move_snapshot(
            _snap(moves, bar_idx=101, bar_ts=1001.0)
        )
        # 相同输入 → 无新事件
        assert snaps2[0].zhongshu_events == []
        assert snaps2[0].move_events == []

    def test_growing_input_produces_events(self) -> None:
        """输入增长 → 产生新事件。"""
        from newchan.core.recursion import RecursiveStack
        stack = RecursiveStack()
        # 第一轮：3 个 settled → 候选中枢
        moves1 = [
            _move(0, settled=True, high=10, low=5, direction="up"),
            _move(1, settled=True, high=12, low=6, direction="down"),
            _move(2, settled=True, high=11, low=7, direction="up"),
        ]
        snaps1 = stack.process_level1_move_snapshot(_snap(moves1, bar_idx=100))
        assert len(snaps1[0].zhongshu_events) > 0

        # 第二轮：加入突破 Move → 中枢 settle + 走势 candidate
        moves2 = moves1 + [
            _move(3, settled=True, high=15, low=11, direction="up"),
        ]
        snaps2 = stack.process_level1_move_snapshot(
            _snap(moves2, bar_idx=101, bar_ts=1001.0)
        )
        # 应有 settle 事件
        settle_events = [
            e for e in snaps2[0].zhongshu_events
            if isinstance(e, ZhongshuSettleV1)
        ]
        assert len(settle_events) == 1
