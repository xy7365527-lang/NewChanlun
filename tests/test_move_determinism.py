"""走势类型确定性测试 — I22 验证（MVP-D0）

覆盖 3 个场景：
  1. moves_from_zhongshus 确定性
  2. diff_moves 确定性（多种状态变化）
  3. MoveEngine 多步确定性
"""

from __future__ import annotations

import pytest

from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.core.recursion.move_engine import MoveEngine
from newchan.core.recursion.move_state import diff_moves
from newchan.core.recursion.zhongshu_state import ZhongshuSnapshot


# ── helpers ──

def _zs(
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    *,
    settled: bool = True,
    break_direction: str = "up",
) -> Zhongshu:
    seg_count = seg_end - seg_start + 1
    break_seg = seg_end + 1 if settled else -1
    return Zhongshu(
        zd=zd, zg=zg,
        seg_start=seg_start, seg_end=seg_end,
        seg_count=seg_count, settled=settled,
        break_seg=break_seg,
        break_direction=break_direction if settled else "",
    )


# =====================================================================
# 1) moves_from_zhongshus 确定性
# =====================================================================

class TestPureFunctionDeterminism:
    def test_same_input_same_output(self):
        """同输入两次 → 完全相同 Move 列表。"""
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
            _zs(6, 8, 20.0, 28.0, break_direction="up"),
            _zs(12, 14, 30.0, 38.0, break_direction="down"),
        ]
        m1 = moves_from_zhongshus(zhongshus)
        m2 = moves_from_zhongshus(zhongshus)
        assert m1 == m2


# =====================================================================
# 2) diff_moves 确定性（多种状态变化）
# =====================================================================

class TestDiffDeterminism:
    def test_multi_state_change(self):
        """涉及 invalidate + candidate + settle 的复杂 diff → 确定性。"""
        prev_zs = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
        ]
        curr_zs = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
            _zs(6, 8, 20.0, 28.0, break_direction="up"),
        ]
        prev = moves_from_zhongshus(prev_zs)
        curr = moves_from_zhongshus(curr_zs)

        ev1 = diff_moves(prev, curr, bar_idx=10, bar_ts=100.0)
        ev2 = diff_moves(prev, curr, bar_idx=10, bar_ts=100.0)

        assert len(ev1) == len(ev2)
        for a, b in zip(ev1, ev2):
            assert a.event_id == b.event_id
            assert a.event_type == b.event_type


# =====================================================================
# 3) MoveEngine 多步确定性
# =====================================================================

class TestEngineMultiStepDeterminism:
    def test_engine_determinism(self):
        """两个独立 MoveEngine 处理相同快照序列 → 相同事件。"""
        snapshots = [
            ZhongshuSnapshot(
                bar_idx=10, bar_ts=100.0,
                zhongshus=[_zs(0, 2, 10.0, 18.0, break_direction="up")],
                events=[],
            ),
            ZhongshuSnapshot(
                bar_idx=11, bar_ts=101.0,
                zhongshus=[
                    _zs(0, 2, 10.0, 18.0, break_direction="up"),
                    _zs(6, 8, 20.0, 28.0, break_direction="up"),
                ],
                events=[],
            ),
            ZhongshuSnapshot(
                bar_idx=12, bar_ts=102.0,
                zhongshus=[
                    _zs(0, 2, 10.0, 18.0, break_direction="up"),
                    _zs(6, 8, 20.0, 28.0, break_direction="up"),
                    _zs(12, 14, 5.0, 18.0, break_direction="down"),
                ],
                events=[],
            ),
        ]

        engine_a = MoveEngine()
        engine_b = MoveEngine()

        for snap in snapshots:
            result_a = engine_a.process_zhongshu_snapshot(snap)
            result_b = engine_b.process_zhongshu_snapshot(snap)

            assert len(result_a.events) == len(result_b.events)
            for ea, eb in zip(result_a.events, result_b.events):
                assert ea.event_id == eb.event_id
                assert ea.event_type == eb.event_type
                assert ea.seq == eb.seq
