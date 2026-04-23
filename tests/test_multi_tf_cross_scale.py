"""跨TF区间套链路测试——484号谱系下游推论。

测试覆盖：
1. extract_c_segment_timestamps(): Move → (ts_start, ts_end) 时间戳提取
2. run_cross_scale_nested_search(): 高级别背驰 → 时间戳对齐 → 低级别
   RecursiveOrchestrator → 区间套搜索的完整链路
3. 端到端集成：合成多TF数据验证链路可执行

规范引用:
  - 484号谱系: multi_tf 架构审计，跨TF区间套缺口
  - 缠论第27课: 区间套精确大转折点寻找程序定理
  - 238号/239号: 多TF输入架构/方向性力度
  - 237号: T6 贡献率诊断（递归深度不足根因）

认识论等级: L1（代码层验证，合成数据验证管线正确性，不验证假设）
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from typing import Literal

import pytest

from newchan.a_move_v1 import Move
from newchan.a_nested_divergence import NestedDivergence
from newchan.topology.multi_tf_adapter import (
    CrossLevelDivergence,
    LevelResult,
    MultiTFOrchestrator,
    MultiTFResult,
    TimeframeLevel,
)
from newchan.types import Bar


# ── mock helpers ────────────────────────────────────────────


@dataclass(frozen=True)
class _MockSegment:
    """最小化线段 mock（只暴露区间套计算所需字段）。"""

    i0: int  # 合并 bar 起始索引
    i1: int  # 合并 bar 终止索引
    high: float = 110.0
    low: float = 100.0
    direction: Literal["up", "down"] = "up"
    s0: int = 0
    s1: int = 0
    confirmed: bool = True


def _make_bar(ts: datetime, o: float, h: float, l: float, c: float) -> Bar:
    return Bar(ts=ts, open=o, high=h, low=l, close=c)


def _make_bars(
    n: int,
    base_price: float = 100.0,
    start: datetime | None = None,
    step: timedelta = timedelta(minutes=1),
) -> list[Bar]:
    """生成 n 根合成 bar（价格带轻微波动）。"""
    if start is None:
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    price = base_price
    for i in range(n):
        ts = start + step * i
        delta = 1.0 if i % 2 == 0 else -0.8
        o = price
        h = price + max(delta, 0.0) + 0.3
        l = price - max(-delta, 0.0) - 0.3
        c = price + delta
        bars.append(_make_bar(ts, o, h, l, c))
        price = c
    return bars


def _make_trending_bars(
    n: int,
    base_price: float = 100.0,
    start: datetime | None = None,
    step: timedelta = timedelta(minutes=1),
    slope: float = 0.5,
) -> list[Bar]:
    """生成 n 根趋势性 bar——使分型/笔/段有机会生成。"""
    if start is None:
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    for i in range(n):
        ts = start + step * i
        # 锯齿状趋势：每 5 根一个小波
        phase = i % 10
        swing = 3.0 if phase < 5 else -2.0
        o = base_price + slope * i
        c = o + swing
        h = max(o, c) + 0.5
        l = min(o, c) - 0.5
        bars.append(_make_bar(ts, o, h, l, c))
    return bars


def _make_move(
    kind: str = "trend",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
    zs_count: int = 1,
    settled: bool = True,
    high: float = 120.0,
    low: float = 100.0,
) -> Move:
    return Move(
        kind=kind,  # type: ignore[arg-type]
        direction=direction,  # type: ignore[arg-type]
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=0,
        zs_end=max(zs_count - 1, 0),
        zs_count=zs_count,
        settled=settled,
        high=high,
        low=low,
    )


def _make_tf(name: str, index: int) -> TimeframeLevel:
    return TimeframeLevel(tf_name=name, bar_source="test", level_index=index)


# ────────────────────────────────────────────────────────────
# 1. extract_c_segment_timestamps 单元测试
# ────────────────────────────────────────────────────────────


class TestExtractCSegmentTimestamps:
    """extract_c_segment_timestamps(): Move → (ts_start, ts_end)。

    核心映射链：
      Move.seg_start/seg_end
        → segments[seg_*].i0/i1（合并 bar 索引）
        → merged_to_raw（商映射纤维）
        → bars[raw_idx].ts
    """

    def test_basic_extraction(self) -> None:
        """Move 覆盖 seg[0..2]，应返回 [seg[0].i0, seg[2].i1] 对应的时间戳。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        bars = _make_trending_bars(60)
        # 构造 3 根 segment 覆盖合并 bar [0..30]
        segments = [
            _MockSegment(i0=0, i1=10, direction="up"),
            _MockSegment(i0=10, i1=20, direction="down"),
            _MockSegment(i0=20, i1=30, direction="up"),
        ]
        move = _make_move(seg_start=0, seg_end=2, high=120.0, low=100.0)

        ts_start, ts_end = extract_c_segment_timestamps(move, segments, bars)

        assert isinstance(ts_start, datetime)
        assert isinstance(ts_end, datetime)
        assert ts_start <= ts_end
        # 起点 ≥ bars[0].ts，终点 ≤ bars[-1].ts
        assert ts_start >= bars[0].ts
        assert ts_end <= bars[-1].ts

    def test_partial_range(self) -> None:
        """Move 仅覆盖 seg[1..2]，不应包含 seg[0] 的时间范围。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        bars = _make_trending_bars(60)
        segments = [
            _MockSegment(i0=0, i1=10, direction="up"),
            _MockSegment(i0=10, i1=20, direction="down"),
            _MockSegment(i0=20, i1=30, direction="up"),
        ]
        move_full = _make_move(seg_start=0, seg_end=2)
        move_partial = _make_move(seg_start=1, seg_end=2)

        full_start, _ = extract_c_segment_timestamps(move_full, segments, bars)
        partial_start, _ = extract_c_segment_timestamps(move_partial, segments, bars)

        # 部分区间起点应 > 完整区间起点
        assert partial_start >= full_start

    def test_single_segment_move(self) -> None:
        """Move 只含一根段（seg_start == seg_end）。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        bars = _make_trending_bars(40)
        segments = [_MockSegment(i0=0, i1=15, direction="up")]
        move = _make_move(seg_start=0, seg_end=0)

        ts_start, ts_end = extract_c_segment_timestamps(move, segments, bars)

        assert ts_start <= ts_end
        assert bars[0].ts <= ts_start <= bars[-1].ts

    def test_empty_segments_raises(self) -> None:
        """segments 为空时应抛 ValueError（边界条件）。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        bars = _make_trending_bars(10)
        move = _make_move(seg_start=0, seg_end=0)

        with pytest.raises(ValueError):
            extract_c_segment_timestamps(move, [], bars)

    def test_empty_bars_raises(self) -> None:
        """bars 为空时应抛 ValueError。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        segments = [_MockSegment(i0=0, i1=5)]
        move = _make_move(seg_start=0, seg_end=0)

        with pytest.raises(ValueError):
            extract_c_segment_timestamps(move, segments, [])

    def test_seg_index_out_of_range_raises(self) -> None:
        """seg_end 超出 segments 长度时应抛 IndexError 或 ValueError。"""
        from newchan.topology.multi_tf_adapter import extract_c_segment_timestamps

        bars = _make_trending_bars(30)
        segments = [_MockSegment(i0=0, i1=5)]
        move = _make_move(seg_start=0, seg_end=5)  # 越界

        with pytest.raises((IndexError, ValueError)):
            extract_c_segment_timestamps(move, segments, bars)


# ────────────────────────────────────────────────────────────
# 2. run_cross_scale_nested_search 链路测试
# ────────────────────────────────────────────────────────────


class TestRunCrossScaleNestedSearch:
    """run_cross_scale_nested_search(): 完整跨TF区间套链路。

    输入：
      - MultiTFResult（含 cross_level_divergences）
      - 各 TF 的 bars 字典
    输出：
      - 每个 CrossLevelDivergence 对应的 NestedDivergence 列表
    """

    def test_empty_divergences_returns_empty(self) -> None:
        """无跨级别背驰时应返回空字典。"""
        from newchan.topology.multi_tf_pipeline import run_cross_scale_nested_search

        tf_high = _make_tf("30min", index=2)
        tf_low = _make_tf("5min", index=1)

        multi_result = MultiTFResult(
            levels={
                tf_high.tf_name: LevelResult(
                    tf=tf_high, snapshot=None, bar_count=0,
                    move_count=0, zhongshu_count=0,
                    direction="none", last_move=None,
                ),
                tf_low.tf_name: LevelResult(
                    tf=tf_low, snapshot=None, bar_count=0,
                    move_count=0, zhongshu_count=0,
                    direction="none", last_move=None,
                ),
            },
            cross_level_divergences=[],
        )

        tf_bars = {
            tf_high.tf_name: _make_bars(5),
            tf_low.tf_name: _make_bars(20),
        }

        nested_map = run_cross_scale_nested_search(multi_result, tf_bars)

        assert nested_map == {}

    def test_missing_low_tf_bars_skips_divergence(self) -> None:
        """背驰指向的低级别 TF 在 tf_bars 中缺失时应跳过（不报错）。"""
        from newchan.topology.multi_tf_pipeline import run_cross_scale_nested_search

        tf_high = _make_tf("30min", index=2)
        tf_low = _make_tf("5min", index=1)

        # 构造高级别 snapshot（需含 segments 以便提取时间戳）
        high_snap = _make_snapshot_with_segments(
            [
                _MockSegment(i0=0, i1=5, direction="up"),
                _MockSegment(i0=5, i1=10, direction="down"),
                _MockSegment(i0=10, i1=15, direction="up"),
            ],
        )
        high_move = _make_move(seg_start=0, seg_end=2)
        low_move = _make_move(seg_start=0, seg_end=1, high=105.0, low=100.0)

        high_result = LevelResult(
            tf=tf_high, snapshot=high_snap, bar_count=30,
            move_count=1, zhongshu_count=1,
            direction="up", last_move=high_move,
        )
        low_result = LevelResult(
            tf=tf_low, snapshot=None, bar_count=0,
            move_count=0, zhongshu_count=0,
            direction="up", last_move=low_move,
        )

        div = CrossLevelDivergence(
            high_tf=tf_high, low_tf=tf_low,
            direction="top",
            high_move=high_move, low_move=low_move,
            force_high=20.0, force_low=5.0,
            ratio=0.25, confirmed=True,
        )

        multi_result = MultiTFResult(
            levels={tf_high.tf_name: high_result, tf_low.tf_name: low_result},
            cross_level_divergences=[div],
        )

        # 只提供高级别 bars，缺失低级别
        tf_bars = {tf_high.tf_name: _make_trending_bars(30)}

        nested_map = run_cross_scale_nested_search(multi_result, tf_bars)

        # 无低级别 bars 应跳过，不抛错
        assert isinstance(nested_map, dict)

    def test_returns_dict_keyed_by_divergence_index(self) -> None:
        """有低级别 bars 时应返回 dict[int, list[NestedDivergence]]。

        key = CrossLevelDivergence 在列表中的索引。
        """
        from newchan.topology.multi_tf_pipeline import run_cross_scale_nested_search

        tf_high = _make_tf("30min", index=2)
        tf_low = _make_tf("5min", index=1)

        high_snap = _make_snapshot_with_segments(
            [
                _MockSegment(i0=0, i1=10, direction="up"),
                _MockSegment(i0=10, i1=20, direction="down"),
                _MockSegment(i0=20, i1=30, direction="up"),
            ],
        )
        high_move = _make_move(seg_start=0, seg_end=2, high=120.0, low=100.0)
        low_move = _make_move(seg_start=0, seg_end=1, high=105.0, low=100.0)

        high_bars = _make_trending_bars(60, step=timedelta(minutes=30))
        low_bars = _make_trending_bars(
            200,
            start=high_bars[0].ts,
            step=timedelta(minutes=5),
        )

        high_result = LevelResult(
            tf=tf_high, snapshot=high_snap,
            bar_count=len(high_bars), move_count=1, zhongshu_count=1,
            direction="up", last_move=high_move,
        )
        low_result = LevelResult(
            tf=tf_low, snapshot=None, bar_count=0,
            move_count=0, zhongshu_count=0,
            direction="up", last_move=low_move,
        )

        div = CrossLevelDivergence(
            high_tf=tf_high, low_tf=tf_low,
            direction="top",
            high_move=high_move, low_move=low_move,
            force_high=20.0, force_low=5.0,
            ratio=0.25, confirmed=True,
        )

        multi_result = MultiTFResult(
            levels={tf_high.tf_name: high_result, tf_low.tf_name: low_result},
            cross_level_divergences=[div],
        )

        tf_bars = {tf_high.tf_name: high_bars, tf_low.tf_name: low_bars}

        nested_map = run_cross_scale_nested_search(multi_result, tf_bars)

        assert isinstance(nested_map, dict)
        # 若该背驰被处理，key 应为 0（第一个背驰的索引）
        if 0 in nested_map:
            assert isinstance(nested_map[0], list)
            for nd in nested_map[0]:
                assert isinstance(nd, NestedDivergence)


# ────────────────────────────────────────────────────────────
# helpers for orchestrator snapshot mock
# ────────────────────────────────────────────────────────────


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


def _make_snapshot_with_segments(segments: list[_MockSegment]):
    """构造含 segments 的最小化 RecursiveOrchestratorSnapshot。"""
    from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot

    return RecursiveOrchestratorSnapshot(
        bar_idx=100,
        bar_ts=1000.0,
        bi_snapshot=_MockBiSnapshot(),
        seg_snapshot=_MockSegSnapshot(segments=segments),
        zs_snapshot=_MockZsSnapshot(),
        move_snapshot=_MockMoveSnapshot(),
        bsp_snapshot=_MockBspSnapshot(),
        recursive_snapshots=[],
    )


# ────────────────────────────────────────────────────────────
# 3. 端到端集成测试（合成数据）
# ────────────────────────────────────────────────────────────


@pytest.mark.integration
class TestEndToEndCrossScale:
    """端到端链路：合成 bars → MultiTFOrchestrator → 跨TF区间套。

    此测试仅验证链路可执行（不抛异常），不验证区间套结果的语义。
    语义验证需要 L2 真实数据，此处是 L1（管线验证）。
    """

    def test_pipeline_runs_without_error(self) -> None:
        """两层 TF 的合成数据能跑通整条链路。"""
        from newchan.topology.multi_tf_pipeline import run_cross_scale_nested_search

        tf_high = _make_tf("30min", index=2)
        tf_low = _make_tf("5min", index=1)

        # 合成 bars：高级别 30 根（每根 30min），低级别 180 根（每根 5min）
        start = datetime(2024, 1, 1, tzinfo=timezone.utc)
        high_bars = _make_trending_bars(
            30, start=start, step=timedelta(minutes=30),
        )
        low_bars = _make_trending_bars(
            180, start=start, step=timedelta(minutes=5),
        )

        orch = MultiTFOrchestrator(
            timeframes=[tf_low, tf_high],
            stroke_mode="wide",
            max_levels=3,
        )
        result = orch.run({
            tf_high.tf_name: high_bars,
            tf_low.tf_name: low_bars,
        })

        assert isinstance(result, MultiTFResult)

        nested_map = run_cross_scale_nested_search(
            result,
            {tf_high.tf_name: high_bars, tf_low.tf_name: low_bars},
        )

        assert isinstance(nested_map, dict)
