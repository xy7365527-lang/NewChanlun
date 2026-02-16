"""P9 交叉验证测试：level=1 新旧路径一致性。

口径 A（RecursiveOrchestrator.process_bar）和手动管线链
（BiEngine→SegmentEngine→ZhongshuEngine→MoveEngine→BuySellPointEngine）
对同一组 bars 应产生相同的 level=1 快照。

规范引用：level_recursion_interface_v1.md §7 P9
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.core.recursion.buysellpoint_engine import BuySellPointEngine
from newchan.core.recursion.move_engine import MoveEngine
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar


# ── 辅助 ──────────────────────────────────────


def _bar(idx: int, o: float, h: float, l: float, c: float) -> Bar:
    """从序号创建 Bar（5 分钟间隔）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc) + timedelta(seconds=idx * 300),
        open=o,
        high=h,
        low=l,
        close=c,
    )


def _generate_zigzag_bars(n: int = 60) -> list[Bar]:
    """生成锯齿形价格序列，每 8 根 bar 一个半周期。"""
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            base = 100.0 + cycle_pos * 3.0
        else:
            base = 100.0 + (16 - cycle_pos) * 3.0
        noise = (i % 3) * 0.3
        o = base + noise
        h = base + 2.0 + noise
        l = base - 2.0 + noise
        c = base + 1.0 + noise
        bars.append(_bar(i, o, h, l, c))
    return bars


# ── 手动管线链 ──────────────────────────────────


def _run_manual_pipeline(bars: list[Bar]):
    """手动链接五层引擎，逐 bar 驱动，返回最终快照。"""
    bi = BiEngine()
    seg = SegmentEngine()
    zs = ZhongshuEngine()
    mv = MoveEngine()
    bsp = BuySellPointEngine(level_id=1)

    bi_snap = seg_snap = zs_snap = move_snap = bsp_snap = None

    for bar in bars:
        bi_snap = bi.process_bar(bar)
        seg_snap = seg.process_snapshot(bi_snap)
        zs_snap = zs.process_segment_snapshot(seg_snap)
        move_snap = mv.process_zhongshu_snapshot(zs_snap)
        bsp_snap = bsp.process_snapshots(move_snap, zs_snap, seg_snap)

    return bi_snap, seg_snap, zs_snap, move_snap, bsp_snap


# ═══════════════════════════════════════════════════════════
# P9 交叉验证测试
# ═══════════════════════════════════════════════════════════


class TestP9CrossValidation:
    """RecursiveOrchestrator level=1 输出 vs 手动管线链必须一致。"""

    def test_bi_snapshot_consistent(self):
        """笔快照一致。"""
        bars = _generate_zigzag_bars(60)

        orch = RecursiveOrchestrator()
        for bar in bars:
            orch_snap = orch.process_bar(bar)

        bi_manual, *_ = _run_manual_pipeline(bars)

        assert orch_snap.bi_snapshot.bar_idx == bi_manual.bar_idx
        assert orch_snap.bi_snapshot.bar_ts == bi_manual.bar_ts
        assert len(orch_snap.bi_snapshot.strokes) == len(bi_manual.strokes)
        # 逐笔比较核心字段
        for s_a, s_m in zip(orch_snap.bi_snapshot.strokes, bi_manual.strokes):
            assert s_a.direction == s_m.direction
            assert s_a.i0 == s_m.i0
            assert s_a.i1 == s_m.i1
            assert s_a.high == s_m.high
            assert s_a.low == s_m.low

    def test_segment_snapshot_consistent(self):
        """线段快照一致。"""
        bars = _generate_zigzag_bars(60)

        orch = RecursiveOrchestrator()
        for bar in bars:
            orch_snap = orch.process_bar(bar)

        _, seg_manual, *_ = _run_manual_pipeline(bars)

        assert len(orch_snap.seg_snapshot.segments) == len(seg_manual.segments)
        for seg_a, seg_m in zip(
            orch_snap.seg_snapshot.segments, seg_manual.segments,
        ):
            assert seg_a.direction == seg_m.direction
            assert seg_a.high == seg_m.high
            assert seg_a.low == seg_m.low
            assert seg_a.confirmed == seg_m.confirmed

    def test_zhongshu_snapshot_consistent(self):
        """中枢快照一致。"""
        bars = _generate_zigzag_bars(60)

        orch = RecursiveOrchestrator()
        for bar in bars:
            orch_snap = orch.process_bar(bar)

        _, _, zs_manual, *_ = _run_manual_pipeline(bars)

        assert len(orch_snap.zs_snapshot.zhongshus) == len(zs_manual.zhongshus)
        for zs_a, zs_m in zip(
            orch_snap.zs_snapshot.zhongshus, zs_manual.zhongshus,
        ):
            assert zs_a.zd == zs_m.zd
            assert zs_a.zg == zs_m.zg
            assert zs_a.seg_start == zs_m.seg_start
            assert zs_a.settled == zs_m.settled
            assert zs_a.break_direction == zs_m.break_direction

    def test_move_snapshot_consistent(self):
        """走势类型快照一致。"""
        bars = _generate_zigzag_bars(60)

        orch = RecursiveOrchestrator()
        for bar in bars:
            orch_snap = orch.process_bar(bar)

        _, _, _, move_manual, _ = _run_manual_pipeline(bars)

        assert len(orch_snap.move_snapshot.moves) == len(move_manual.moves)
        for mv_a, mv_m in zip(
            orch_snap.move_snapshot.moves, move_manual.moves,
        ):
            assert mv_a.kind == mv_m.kind
            assert mv_a.direction == mv_m.direction
            assert mv_a.seg_start == mv_m.seg_start
            assert mv_a.seg_end == mv_m.seg_end
            assert mv_a.settled == mv_m.settled
            assert mv_a.zs_count == mv_m.zs_count

    def test_bsp_snapshot_consistent(self):
        """买卖点快照一致。"""
        bars = _generate_zigzag_bars(60)

        orch = RecursiveOrchestrator()
        for bar in bars:
            orch_snap = orch.process_bar(bar)

        _, _, _, _, bsp_manual = _run_manual_pipeline(bars)

        assert len(orch_snap.bsp_snapshot.buysellpoints) == len(
            bsp_manual.buysellpoints,
        )
        for bp_a, bp_m in zip(
            orch_snap.bsp_snapshot.buysellpoints, bsp_manual.buysellpoints,
        ):
            assert bp_a.kind == bp_m.kind
            assert bp_a.side == bp_m.side
            assert bp_a.seg_idx == bp_m.seg_idx
            assert bp_a.price == bp_m.price
            assert bp_a.confirmed == bp_m.confirmed

    def test_incremental_consistency(self):
        """逐 bar 增量输出也一致（不只是最终快照）。"""
        bars = _generate_zigzag_bars(30)

        orch = RecursiveOrchestrator()
        bi = BiEngine()
        seg = SegmentEngine()
        zs = ZhongshuEngine()
        mv = MoveEngine()
        bsp = BuySellPointEngine(level_id=1)

        for bar in bars:
            orch_snap = orch.process_bar(bar)
            bi_snap = bi.process_bar(bar)
            seg_snap = seg.process_snapshot(bi_snap)
            zs_snap = zs.process_segment_snapshot(seg_snap)
            move_snap = mv.process_zhongshu_snapshot(zs_snap)
            bsp_snap = bsp.process_snapshots(move_snap, zs_snap, seg_snap)

            # 每 bar 的快照核心数量必须一致
            assert len(orch_snap.bi_snapshot.strokes) == len(bi_snap.strokes)
            assert len(orch_snap.seg_snapshot.segments) == len(seg_snap.segments)
            assert len(orch_snap.zs_snapshot.zhongshus) == len(zs_snap.zhongshus)
            assert len(orch_snap.move_snapshot.moves) == len(move_snap.moves)
            assert len(orch_snap.bsp_snapshot.buysellpoints) == len(
                bsp_snap.buysellpoints,
            )

    def test_reset_then_replay_consistent(self):
        """reset 后重放应与首次结果一致。"""
        bars = _generate_zigzag_bars(30)

        orch = RecursiveOrchestrator()
        for bar in bars:
            snap_first = orch.process_bar(bar)

        first_stroke_count = len(snap_first.bi_snapshot.strokes)
        first_seg_count = len(snap_first.seg_snapshot.segments)

        # reset + 重放
        orch.reset()
        for bar in bars:
            snap_second = orch.process_bar(bar)

        assert len(snap_second.bi_snapshot.strokes) == first_stroke_count
        assert len(snap_second.seg_snapshot.segments) == first_seg_count
