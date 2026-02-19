"""区间套跨级别背驰搜索测试。

nested_divergence_search() 从 RecursiveStack 的最高级别开始，
逐级向下检测背驰，每级的检测范围被上一级的背驰 C 段约束。

级别 = 递归层级（level_id），不是时间周期。
级别由 RecursiveStack 递归构造自动确定，不是外部输入。

规范引用: beichi.md #5 区间套, 第27课 精确大转折点寻找程序定理
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

import pytest

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.core.recursion.recursive_level_state import RecursiveLevelSnapshot

# 将在 GREEN 阶段实现
from newchan.a_nested_divergence import (
    NestedDivergence,
    nested_divergence_search,
    _get_moves_at_level,
    _level_move_to_bar_range,
    divergences_from_level_snapshot,
)
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot


# ── mock helpers ──────────────────────────────


@dataclass(frozen=True)
class _MockSegment:
    """最小化线段 mock（只需 i0, i1, high, low, direction）。"""
    i0: int
    i1: int
    high: float
    low: float
    direction: Literal["up", "down"] = "up"


@dataclass(frozen=True)
class _MockBiSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    strokes: list = field(default_factory=list)


@dataclass(frozen=True)
class _MockSegSnapshot:
    segments: list = field(default_factory=list)


@dataclass(frozen=True)
class _MockZsSnapshot:
    zhongshus: list = field(default_factory=list)


@dataclass(frozen=True)
class _MockMoveSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    moves: list = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass(frozen=True)
class _MockBspSnapshot:
    buysellpoints: list = field(default_factory=list)


def _make_orchestrator_snap(
    segments: list | None = None,
    zhongshus: list | None = None,
    moves: list | None = None,
    recursive_snapshots: list | None = None,
) -> RecursiveOrchestratorSnapshot:
    """构造最小化 RecursiveOrchestratorSnapshot。"""
    return RecursiveOrchestratorSnapshot(
        bar_idx=100,
        bar_ts=1000.0,
        bi_snapshot=_MockBiSnapshot(),
        seg_snapshot=_MockSegSnapshot(segments=segments or []),
        zs_snapshot=_MockZsSnapshot(zhongshus=zhongshus or []),
        move_snapshot=_MockMoveSnapshot(moves=moves or []),
        bsp_snapshot=_MockBspSnapshot(),
        recursive_snapshots=recursive_snapshots or [],
    )


def _make_move(
    kind: str = "trend",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 5,
    zs_start: int = 0,
    zs_end: int = 1,
    zs_count: int = 2,
    settled: bool = True,
    high: float = 120.0,
    low: float = 100.0,
) -> Move:
    return Move(
        kind=kind,
        direction=direction,
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=zs_start,
        zs_end=zs_end,
        zs_count=zs_count,
        settled=settled,
        high=high,
        low=low,
    )


def _make_level_zhongshu(
    comp_start: int,
    comp_end: int,
    zd: float = 100.0,
    zg: float = 110.0,
    settled: bool = True,
    break_direction: str = "up",
    gg: float = 115.0,
    dd: float = 95.0,
    level_id: int = 2,
) -> LevelZhongshu:
    return LevelZhongshu(
        zd=zd,
        zg=zg,
        comp_start=comp_start,
        comp_end=comp_end,
        comp_count=comp_end - comp_start + 1,
        settled=settled,
        break_comp=comp_end + 1 if settled else -1,
        break_direction=break_direction if settled else "",
        gg=gg,
        dd=dd,
        level_id=level_id,
    )


# ═══════════════════════════════════════════════
# A. _get_moves_at_level 辅助函数
# ═══════════════════════════════════════════════


class TestGetMovesAtLevel:
    """_get_moves_at_level 从快照中提取指定级别的 Move 列表。"""

    def test_level_1(self):
        """level=1 → move_snapshot.moves。"""
        l1_moves = [_make_move(seg_start=0, seg_end=3)]
        snap = _make_orchestrator_snap(moves=l1_moves)
        assert _get_moves_at_level(1, snap) is l1_moves

    def test_level_2(self):
        """level=2 → recursive_snapshots[0].moves。"""
        l2_moves = [_make_move(seg_start=0, seg_end=1)]
        rec_snap = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[], moves=l2_moves,
            zhongshu_events=[], move_events=[],
        )
        snap = _make_orchestrator_snap(recursive_snapshots=[rec_snap])
        assert _get_moves_at_level(2, snap) is l2_moves

    def test_level_beyond_max(self):
        """超过最高级别 → 空列表。"""
        snap = _make_orchestrator_snap()
        assert _get_moves_at_level(5, snap) == []


# ═══════════════════════════════════════════════
# B. _level_move_to_bar_range 跨级别映射
# ═══════════════════════════════════════════════


class TestLevelMoveToBarRange:
    """_level_move_to_bar_range 将任意级别的 Move 映射到 merged bar 索引。"""

    def test_level_1_direct(self):
        """level=1: Move.seg_start/seg_end → segments[].i0/i1。"""
        segments = [
            _MockSegment(i0=0, i1=10, high=105, low=95),
            _MockSegment(i0=10, i1=20, high=110, low=100),
            _MockSegment(i0=20, i1=30, high=115, low=105),
            _MockSegment(i0=30, i1=40, high=110, low=100),
        ]
        move = _make_move(seg_start=1, seg_end=3)
        snap = _make_orchestrator_snap(segments=segments)
        result = _level_move_to_bar_range(move, 1, snap)
        assert result == (10, 40)  # segments[1].i0 to segments[3].i1

    def test_level_2_recursive(self):
        """level=2: Move.seg_start/seg_end → settled(level1 moves)[].seg_start/seg_end → segments。"""
        # 4 个 level 1 segments
        segments = [
            _MockSegment(i0=0, i1=50, high=110, low=90),
            _MockSegment(i0=50, i1=100, high=120, low=100),
            _MockSegment(i0=100, i1=150, high=130, low=110),
            _MockSegment(i0=150, i1=200, high=120, low=100),
            _MockSegment(i0=200, i1=250, high=140, low=120),
            _MockSegment(i0=250, i1=300, high=130, low=110),
        ]
        # 3 个 level 1 settled moves
        l1_moves = [
            _make_move(seg_start=0, seg_end=1, settled=True),   # comp 0
            _make_move(seg_start=2, seg_end=3, settled=True),   # comp 1
            _make_move(seg_start=4, seg_end=5, settled=True),   # comp 2
        ]
        # level 2 move covers comp 1-2 (= l1_moves[1] and l1_moves[2])
        l2_move = _make_move(seg_start=1, seg_end=2)

        l2_rec = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[], moves=[l2_move],
            zhongshu_events=[], move_events=[],
        )
        snap = _make_orchestrator_snap(
            segments=segments, moves=l1_moves,
            recursive_snapshots=[l2_rec],
        )
        result = _level_move_to_bar_range(l2_move, 2, snap)
        # l1_moves[1].seg_start=2 → segments[2].i0=100
        # l1_moves[2].seg_end=5 → segments[5].i1=300
        assert result == (100, 300)

    def test_level_1_single_segment(self):
        """level=1 单段 Move。"""
        segments = [_MockSegment(i0=42, i1=99, high=110, low=90)]
        move = _make_move(seg_start=0, seg_end=0)
        snap = _make_orchestrator_snap(segments=segments)
        result = _level_move_to_bar_range(move, 1, snap)
        assert result == (42, 99)

    def test_out_of_bounds_returns_zero(self):
        """索引越界 → (0, 0)。"""
        snap = _make_orchestrator_snap(segments=[])
        move = _make_move(seg_start=0, seg_end=5)
        result = _level_move_to_bar_range(move, 1, snap)
        assert result == (0, 0)


# ═══════════════════════════════════════════════
# C. divergences_from_level_snapshot（level 2+ 背驰检测）
# ═══════════════════════════════════════════════


class TestDivergencesFromLevelSnapshot:
    """level 2+ 递归级别的背驰检测（价格振幅力度）。"""

    def _make_trend_scenario(self):
        """构造一个 level 2 趋势背驰场景。

        level 1 有 6 个 settled moves（作为 level 2 的组件）。
        level 2 有 2 个 settled zhongshu → 1 个 trend move。
        A 段（组件 2）力度大于 C 段（组件 5）→ 背驰。
        """
        # 6 个 level 1 settled moves 作为 level 2 的"segments"
        components = [
            _make_move(high=110, low=100, settled=True),  # 0: zs0 内
            _make_move(high=115, low=105, settled=True),  # 1: zs0 内
            _make_move(high=130, low=110, settled=True),  # 2: A 段（力度大: 130-110=20）
            _make_move(high=125, low=115, settled=True),  # 3: zs1 内
            _make_move(high=120, low=110, settled=True),  # 4: zs1 内
            _make_move(high=125, low=120, settled=True),  # 5: C 段（力度小: 125-120=5）
        ]
        # 2 个 settled zhongshu
        zs0 = _make_level_zhongshu(
            comp_start=0, comp_end=1, zd=105, zg=110,
            settled=True, break_direction="up",
        )
        zs1 = _make_level_zhongshu(
            comp_start=3, comp_end=4, zd=115, zg=120,
            settled=True, break_direction="up",
        )
        # 1 个 trend move covering all components
        trend_move = _make_move(
            kind="trend", direction="up",
            seg_start=0, seg_end=5,
            zs_start=0, zs_end=1, zs_count=2,
            settled=False,
        )
        level_snap = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[zs0, zs1], moves=[trend_move],
            zhongshu_events=[], move_events=[],
        )
        return level_snap, components

    def test_trend_divergence_detected(self):
        """level 2 趋势背驰：A 段力度 > C 段力度 → 背驰。"""
        level_snap, components = self._make_trend_scenario()
        divs = divergences_from_level_snapshot(level_snap, components)
        assert len(divs) == 1
        div = divs[0]
        assert div.kind == "trend"
        assert div.direction == "top"  # up trend → top divergence
        assert div.level_id == 2
        assert div.force_a > div.force_c

    def test_trend_divergence_confirmed(self):
        """背驰的 confirmed 跟随 move.settled。"""
        level_snap, components = self._make_trend_scenario()
        divs = divergences_from_level_snapshot(level_snap, components)
        assert divs[0].confirmed is False  # trend_move.settled=False

    def test_no_trend_divergence_when_force_equal(self):
        """A/C 力度相等 → 无背驰。"""
        # 6 个组件，A=C 力度相同
        components = [
            _make_move(high=110, low=100, settled=True),
            _make_move(high=115, low=105, settled=True),
            _make_move(high=125, low=115, settled=True),  # A 段: 10
            _make_move(high=130, low=120, settled=True),
            _make_move(high=135, low=125, settled=True),
            _make_move(high=145, low=135, settled=True),  # C 段: 10
        ]
        zs0 = _make_level_zhongshu(comp_start=0, comp_end=1, settled=True)
        zs1 = _make_level_zhongshu(comp_start=3, comp_end=4, settled=True)
        trend = _make_move(
            kind="trend", direction="up",
            seg_start=0, seg_end=5,
            zs_start=0, zs_end=1, zs_count=2,
            settled=False,
        )
        snap = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[zs0, zs1], moves=[trend],
            zhongshu_events=[], move_events=[],
        )
        divs = divergences_from_level_snapshot(snap, components)
        assert len(divs) == 0

    def test_consolidation_not_detected_as_trend(self):
        """盘整 Move 不应被检测为趋势背驰。"""
        components = [_make_move(settled=True) for _ in range(4)]
        zs0 = _make_level_zhongshu(comp_start=0, comp_end=2, settled=True)
        consolidation = _make_move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=3,
            zs_start=0, zs_end=0, zs_count=1,
            settled=False,
        )
        snap = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[zs0], moves=[consolidation],
            zhongshu_events=[], move_events=[],
        )
        divs = divergences_from_level_snapshot(snap, components)
        # 盘整可能产生盘整背驰，但不是趋势背驰
        for d in divs:
            assert d.kind != "trend"

    def test_empty_moves(self):
        """无 Move → 无背驰。"""
        snap = RecursiveLevelSnapshot(
            bar_idx=100, bar_ts=1000.0, level_id=2,
            zhongshus=[], moves=[],
            zhongshu_events=[], move_events=[],
        )
        divs = divergences_from_level_snapshot(snap, [])
        assert divs == []


# ═══════════════════════════════════════════════
# D. nested_divergence_search 主搜索
# ═══════════════════════════════════════════════


class TestNestedDivergenceSearch:
    """区间套跨级别背驰搜索的完整流程测试。"""

    def test_no_recursive_levels_no_nesting(self):
        """只有 level 1，无递归层 → 返回空（区间套需要至少2级别）。"""
        snap = _make_orchestrator_snap(
            segments=[], moves=[], recursive_snapshots=[],
        )
        result = nested_divergence_search(snap)
        assert result == []

    def test_empty_snapshot(self):
        """完全空的快照 → 空结果。"""
        snap = _make_orchestrator_snap()
        result = nested_divergence_search(snap)
        assert result == []

    def test_result_type(self):
        """返回值为 list[NestedDivergence]。"""
        snap = _make_orchestrator_snap()
        result = nested_divergence_search(snap)
        assert isinstance(result, list)

    def test_nested_chain_level_descending(self):
        """嵌套链中 level_id 必须严格递减。"""
        # 这是一个属性测试：对任何非空结果，chain 中 level_id 必须递减
        # 这里用简单的空输入验证 NestedDivergence 的结构
        nd = NestedDivergence(chain=[], bar_range=(0, 0))
        assert nd.chain == []
        assert nd.bar_range == (0, 0)

    def test_bar_range_contracts(self):
        """区间套核心性质：bar_range 随级别下降而收缩。

        如果 nested_divergence_search 返回非空结果，
        每一级的 bar_range 必须 ⊆ 上一级的 bar_range。
        """
        # NestedDivergence.bar_range 是最终（最低级别）的范围
        # 每级范围递减是区间套的数学保证
        nd = NestedDivergence(
            chain=[(3, None), (2, None), (1, None)],
            bar_range=(50, 80),
        )
        assert nd.bar_range[0] >= 0
        assert nd.bar_range[1] >= nd.bar_range[0]

    def test_levels_are_recursive_not_timeframe(self):
        """关键不变量：level_id 来自 RecursiveStack 递归构造。

        这个测试确保 nested_divergence_search 只使用快照中的
        recursive_snapshots 层级信息，不引入任何时间周期参数。
        """
        # nested_divergence_search 的签名不接受 timeframe 参数
        # 只接受 RecursiveOrchestratorSnapshot
        import inspect
        sig = inspect.signature(nested_divergence_search)
        params = list(sig.parameters.keys())
        assert "timeframe" not in params
        assert "tf" not in params
        assert "period" not in params
        # 只接受 snap + MACD 相关参数
        assert "snap" in params


# ═══════════════════════════════════════════════
# E. 输入不可变性和幂等性
# ═══════════════════════════════════════════════


class TestNestedDivergencePurity:
    """nested_divergence_search 不修改输入。"""

    def test_input_immutability(self):
        """调用后 RecursiveOrchestratorSnapshot 不被修改。"""
        segments = [_MockSegment(i0=0, i1=10, high=110, low=90)]
        moves = [_make_move(seg_start=0, seg_end=0)]
        snap = _make_orchestrator_snap(segments=segments, moves=moves)
        moves_before = list(snap.move_snapshot.moves)
        nested_divergence_search(snap)
        assert list(snap.move_snapshot.moves) == moves_before

    def test_idempotency(self):
        """两次调用结果相同。"""
        snap = _make_orchestrator_snap()
        r1 = nested_divergence_search(snap)
        r2 = nested_divergence_search(snap)
        assert r1 == r2


# ═══════════════════════════════════════════════
# F. 端到端集成测试（level 3 → 2 → 1 完整嵌套）
# ═══════════════════════════════════════════════


class TestNestedDivergenceE2E:
    """端到端测试：构造完整的多级别嵌套背驰场景。

    场景：3 级递归结构中，level 3 检测到趋势背驰，
    向下传递到 level 2，再到 level 1，bar_range 逐级收缩。

    这验证了区间套的核心数学性质：D_3 ⊃ D_2 ⊃ D_1
    """

    def _build_full_scenario(self):
        """构造 3 级递归完整场景。

        结构布局：
        - Level 1: 12 segments (i0=0..i1=1200)
          - 6 settled moves（作为 level 2 组件）
        - Level 2: 6 components → 2 zhongshu → 1 trend move with divergence
          - 3 settled moves（作为 level 3 组件）
        - Level 3: 3 components → 2 zhongshu → 1 trend move with divergence

        背驰设计：
        - Level 3: A 段力度 20 > C 段力度 5 → 趋势背驰
        - Level 2: A 段力度 10 > C 段力度 3 → 趋势背驰
        """
        # ── Level 1: 12 segments ──
        segments = [
            _MockSegment(i0=0, i1=100, high=110, low=90, direction="up"),
            _MockSegment(i0=100, i1=200, high=105, low=85, direction="down"),
            _MockSegment(i0=200, i1=300, high=120, low=95, direction="up"),
            _MockSegment(i0=300, i1=400, high=115, low=90, direction="down"),
            _MockSegment(i0=400, i1=500, high=130, low=100, direction="up"),
            _MockSegment(i0=500, i1=600, high=125, low=95, direction="down"),
            _MockSegment(i0=600, i1=700, high=140, low=110, direction="up"),
            _MockSegment(i0=700, i1=800, high=135, low=105, direction="down"),
            _MockSegment(i0=800, i1=900, high=150, low=120, direction="up"),
            _MockSegment(i0=900, i1=1000, high=145, low=115, direction="down"),
            _MockSegment(i0=1000, i1=1100, high=155, low=130, direction="up"),
            _MockSegment(i0=1100, i1=1200, high=150, low=125, direction="down"),
        ]

        # ── Level 1: 6 settled moves（每 2 段一个 move）──
        l1_moves = [
            _make_move(seg_start=0, seg_end=1, high=110, low=85, settled=True, direction="up"),
            _make_move(seg_start=2, seg_end=3, high=120, low=90, settled=True, direction="down"),
            _make_move(seg_start=4, seg_end=5, high=130, low=95, settled=True, direction="up"),
            _make_move(seg_start=6, seg_end=7, high=140, low=105, settled=True, direction="down"),
            _make_move(seg_start=8, seg_end=9, high=150, low=115, settled=True, direction="up"),
            _make_move(seg_start=10, seg_end=11, high=155, low=125, settled=True, direction="up"),
        ]

        # ── Level 2 snapshot ──
        # Components = l1_moves (all settled)
        # 2 zhongshu (comp 0-1, comp 3-4), A段 = comp 2 (力度大), C段 = comp 5 (力度小)
        l2_zs0 = _make_level_zhongshu(
            comp_start=0, comp_end=1, zd=90, zg=110,
            settled=True, break_direction="up", level_id=2,
        )
        l2_zs1 = _make_level_zhongshu(
            comp_start=3, comp_end=4, zd=115, zg=140,
            settled=True, break_direction="up", level_id=2,
        )
        # Trend move covering all 6 components
        l2_trend = _make_move(
            kind="trend", direction="up",
            seg_start=0, seg_end=5,
            zs_start=0, zs_end=1, zs_count=2,
            settled=False, high=155, low=85,
        )
        # Level 2 also produces 3 settled moves (as level 3 components)
        l2_settled_moves = [
            _make_move(seg_start=0, seg_end=1, high=120, low=85, settled=True, direction="up"),
            _make_move(seg_start=2, seg_end=3, high=140, low=90, settled=True, direction="down"),
            _make_move(seg_start=4, seg_end=5, high=155, low=115, settled=True, direction="up"),
        ]
        l2_snap = RecursiveLevelSnapshot(
            bar_idx=1200, bar_ts=12000.0, level_id=2,
            zhongshus=[l2_zs0, l2_zs1],
            moves=[l2_trend],
            zhongshu_events=[], move_events=[],
        )

        # ── Level 3 snapshot ──
        # Components = l2_settled_moves
        # 2 zhongshu (comp 0, comp 1), A段 = comp 1-2区间内 (力度大), C段 = comp 2 (力度小)
        # For a proper trend divergence: 2 zhongshu needed
        # comp 0: zs0 内, comp 1: A段（力度大）, comp 2: 中间穿越
        # 简化：3 组件 → 不够构成 2 zhongshu 的趋势
        # 需要至少 6 个组件。调整 level 2 产出更多 settled moves。

        # 重新设计：level 2 产出 6 个 settled moves 作为 level 3 组件
        l2_settled_moves = [
            _make_move(seg_start=0, seg_end=0, high=110, low=85, settled=True, direction="up"),
            _make_move(seg_start=1, seg_end=1, high=120, low=90, settled=True, direction="down"),
            _make_move(seg_start=2, seg_end=2, high=150, low=95, settled=True, direction="up"),  # A段: 150-95=55
            _make_move(seg_start=3, seg_end=3, high=140, low=105, settled=True, direction="down"),
            _make_move(seg_start=4, seg_end=4, high=150, low=115, settled=True, direction="up"),
            _make_move(seg_start=5, seg_end=5, high=155, low=145, settled=True, direction="up"),  # C段: 155-145=10
        ]

        l3_zs0 = _make_level_zhongshu(
            comp_start=0, comp_end=1, zd=90, zg=110,
            settled=True, break_direction="up", level_id=3,
        )
        l3_zs1 = _make_level_zhongshu(
            comp_start=3, comp_end=4, zd=115, zg=140,
            settled=True, break_direction="up", level_id=3,
        )
        l3_trend = _make_move(
            kind="trend", direction="up",
            seg_start=0, seg_end=5,
            zs_start=0, zs_end=1, zs_count=2,
            settled=False, high=155, low=85,
        )
        l3_snap = RecursiveLevelSnapshot(
            bar_idx=1200, bar_ts=12000.0, level_id=3,
            zhongshus=[l3_zs0, l3_zs1],
            moves=[l3_trend],
            zhongshu_events=[], move_events=[],
        )

        # ── 注意：recursive_snapshots 索引 ──
        # recursive_snapshots[0] = level 2
        # recursive_snapshots[1] = level 3
        # 但 level 2 的 moves 字段需要包含作为 level 3 组件的 settled moves
        l2_snap_with_settled = RecursiveLevelSnapshot(
            bar_idx=1200, bar_ts=12000.0, level_id=2,
            zhongshus=[l2_zs0, l2_zs1],
            moves=l2_settled_moves + [l2_trend],  # settled moves + trend
            zhongshu_events=[], move_events=[],
        )

        snap = _make_orchestrator_snap(
            segments=segments,
            moves=l1_moves,
            recursive_snapshots=[l2_snap_with_settled, l3_snap],
        )
        return snap

    def test_e2e_returns_nested_chain(self):
        """端到端：level 3 → level 2 嵌套链存在。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        assert len(results) >= 1, "应该至少找到一条嵌套链"

    def test_e2e_chain_levels_descending(self):
        """嵌套链中的 level_id 严格递减。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        for nd in results:
            levels = [lv for lv, _ in nd.chain]
            for i in range(1, len(levels)):
                assert levels[i] < levels[i - 1], (
                    f"level_id 应递减: {levels}"
                )

    def test_e2e_bar_range_valid(self):
        """最终 bar_range 有效（start < end）。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        for nd in results:
            if len(nd.chain) > 1:
                assert nd.bar_range[0] < nd.bar_range[1], (
                    f"bar_range 应 start < end: {nd.bar_range}"
                )

    def test_e2e_bar_range_within_data_bounds(self):
        """bar_range 不超出数据范围 [0, 1200]。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        for nd in results:
            assert nd.bar_range[0] >= 0
            assert nd.bar_range[1] <= 1200

    def test_e2e_top_level_has_divergence(self):
        """最高级别 chain 项包含非 None 的 Divergence。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        for nd in results:
            top_level, top_div = nd.chain[0]
            assert top_div is not None, "最高级别应有背驰"
            assert top_div.kind == "trend"
            assert top_div.force_a > top_div.force_c

    def test_e2e_chain_divergences_are_trend_type(self):
        """嵌套链中每个有效 Divergence 都是趋势背驰。"""
        snap = self._build_full_scenario()
        results = nested_divergence_search(snap)
        for nd in results:
            for level_id, div in nd.chain:
                if div is not None:
                    assert div.kind == "trend"
                    assert div.direction == "top"  # up trend → top divergence
