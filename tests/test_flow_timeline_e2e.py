"""流转状态时间序列端到端集成测试。

用真实 ETF 缓存数据跑通完整管线：
  缓存 OHLCV → 比价K线 → 包含处理 → 分型 → 笔
  → 对每根笔提取端点时间戳和方向 → EdgeEvent
  → 合并所有边的 EdgeEvent → build_flow_timeline
  → FlowSnapshot 时间序列

验证：
  1. 时间序列非空
  2. 快照按时间排序
  3. 每个快照都守恒（check_conservation）
  4. 快照数 = 所有边的笔数之和
  5. 共振强度从 NONE 演变到有值的时间点存在
  6. 打印头尾快照供人工审查

数据依赖：
  .cache/ 中需要 SPY/GLD/TLT/SLV 四个 ETF 的日线缓存。

概念溯源：
  [新缠论] 流转状态时间序列端到端管线验证
  谱系引用：023-equivalence-relation-formalization（已结算）
"""

from __future__ import annotations

from itertools import combinations

import pandas as pd
import pytest

from newchan.cache import load_df
from newchan.capital_flow import FlowDirection
from newchan.equivalence import make_ratio_kline
from newchan.flow_relation import (
    ResonanceStrength,
    check_conservation,
)
from newchan.flow_timeline import EdgeEvent, FlowSnapshot, build_flow_timeline
from newchan.matrix_topology import AssetVertex
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import Stroke, strokes_from_fractals


# ── 标的→顶点 映射 ─────────────────────────────────────────
# 复用 test_flow_relation_e2e.py 的映射方案。
# REAL_ESTATE 缺数据，用 SLV（白银）暂时占位以测试拓扑完整性。

SYMBOL_TO_VERTEX: dict[str, AssetVertex] = {
    "SPY": AssetVertex.EQUITY,
    "GLD": AssetVertex.COMMODITY,
    "TLT": AssetVertex.CASH,
    "SLV": AssetVertex.REAL_ESTATE,  # 代用
}

ALL_SYMBOLS = list(SYMBOL_TO_VERTEX.keys())


# ── fixtures ───────────────────────────────────────────────


@pytest.fixture(scope="module")
def cached_data() -> dict[str, pd.DataFrame]:
    """加载所有缓存 ETF 数据。缺少任何一个则跳过整个模块。"""
    data: dict[str, pd.DataFrame] = {}
    for sym in ALL_SYMBOLS:
        df = load_df(f"{sym}_1day_raw")
        if df is None:
            pytest.skip(f"缓存缺失：{sym}_1day_raw.parquet")
        data[sym] = df
    return data


# ── 边管线辅助函数 ─────────────────────────────────────────


def _run_pipeline(
    df_a: pd.DataFrame, df_b: pd.DataFrame
) -> tuple[pd.DataFrame, list[Stroke]]:
    """对一对标的运行比价K线 → 包含 → 分型 → 笔 管线。

    Returns
    -------
    (df_merged, strokes)
        包含处理后的 DataFrame 和笔序列。
    """
    ratio_kline = make_ratio_kline(df_a, df_b)
    df_merged, _ = merge_inclusion(ratio_kline)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(df_merged, fractals)
    return df_merged, strokes


def _stroke_to_flow_direction(stroke: Stroke) -> FlowDirection:
    """将单根笔的方向映射为 FlowDirection。

    映射规则（与 capital_flow._map_stroke 一致）：
      - stroke.direction == "up"   → 比价上涨 → B_TO_A
      - stroke.direction == "down" → 比价下跌 → A_TO_B
    """
    if stroke.direction == "up":
        return FlowDirection.B_TO_A
    return FlowDirection.A_TO_B


def _build_edge_events(
    sym_a: str,
    sym_b: str,
    df_merged: pd.DataFrame,
    strokes: list[Stroke],
) -> list[EdgeEvent]:
    """为一条边的每根笔构造 EdgeEvent 列表。

    每根笔的端点时间戳取自 df_merged.index[stroke.i1]（笔的终点）。
    方向使用 _stroke_to_flow_direction 映射。

    EdgeEvent 的 vertex_a/vertex_b 对齐规则：
      sym_a → SYMBOL_TO_VERTEX[sym_a] = vertex_a
      sym_b → SYMBOL_TO_VERTEX[sym_b] = vertex_b
      方向语义：比价 K 线 = sym_a / sym_b
        up   → B_TO_A（资本从 vertex_b 流向 vertex_a）
        down → A_TO_B（资本从 vertex_a 流向 vertex_b）
    """
    va = SYMBOL_TO_VERTEX[sym_a]
    vb = SYMBOL_TO_VERTEX[sym_b]
    events: list[EdgeEvent] = []
    for stroke in strokes:
        timestamp = df_merged.index[stroke.i1]
        direction = _stroke_to_flow_direction(stroke)
        events.append(
            EdgeEvent(
                vertex_a=va,
                vertex_b=vb,
                direction=direction,
                timestamp=timestamp,
            )
        )
    return events


# ── 核心 fixture：构建所有边的事件和时间序列 ──────────────


@pytest.fixture(scope="module")
def all_edge_data(
    cached_data: dict[str, pd.DataFrame],
) -> dict[str, object]:
    """构建所有 6 条边的管线数据和 EdgeEvent 列表。

    Returns
    -------
    dict 包含：
      - "events": 所有边的 EdgeEvent 合集
      - "stroke_counts": dict[str, int] 每条边的笔数
      - "timeline": build_flow_timeline 的输出
    """
    all_events: list[EdgeEvent] = []
    stroke_counts: dict[str, int] = {}

    for sym_a, sym_b in combinations(ALL_SYMBOLS, 2):
        df_merged, strokes = _run_pipeline(
            cached_data[sym_a], cached_data[sym_b]
        )
        label = f"{sym_a}/{sym_b}"
        stroke_counts[label] = len(strokes)

        if strokes:
            events = _build_edge_events(sym_a, sym_b, df_merged, strokes)
            all_events.extend(events)

    timeline = build_flow_timeline(all_events)

    return {
        "events": all_events,
        "stroke_counts": stroke_counts,
        "timeline": timeline,
    }


# ── 测试类 ────────────────────────────────────────────────


@pytest.mark.slow
class TestFlowTimelineStructure:
    """flow_timeline 结构性质验证。"""

    def test_timeline_not_empty(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """时间序列非空（真实数据应产出大量笔事件）。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        assert len(timeline) > 0, "时间序列为空——管线未产出任何笔？"

    def test_snapshots_sorted_by_time(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """快照按时间戳非递减排列。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        timestamps = [s.timestamp for s in timeline]
        for i in range(1, len(timestamps)):
            assert timestamps[i] >= timestamps[i - 1], (
                f"时间序列未排序：snapshot[{i - 1}].timestamp={timestamps[i - 1]} "
                f"> snapshot[{i}].timestamp={timestamps[i]}"
            )

    def test_every_snapshot_conserves(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """每个快照都满足守恒约束 Sigma net(V) = 0。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        for idx, snapshot in enumerate(timeline):
            states = list(snapshot.vertex_states)
            assert check_conservation(states), (
                f"snapshot[{idx}] (t={snapshot.timestamp}) 守恒约束失败！"
                f"states={[(s.vertex.value, s.net_flow) for s in states]}"
            )

    def test_snapshot_count_equals_total_strokes(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """快照数 = 所有边的笔数之和（每根笔产生一个快照）。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        stroke_counts: dict[str, int] = all_edge_data["stroke_counts"]  # type: ignore[assignment]
        total_strokes = sum(stroke_counts.values())
        assert len(timeline) == total_strokes, (
            f"快照数({len(timeline)}) != 总笔数({total_strokes})。"
            f"各边笔数: {stroke_counts}"
        )

    def test_resonance_emerges(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """共振强度从 NONE 演变到有值的时间点存在。

        在足够长的真实数据中，必然存在某些时刻某个顶点的
        |net_flow| >= 2（弱共振或强共振）。
        """
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        found_resonance = False
        for snapshot in timeline:
            for state in snapshot.vertex_states:
                if state.strength != ResonanceStrength.NONE:
                    found_resonance = True
                    break
            if found_resonance:
                break

        assert found_resonance, (
            "整个时间序列中没有出现任何共振（|net_flow| >= 2），"
            "在数百根笔的真实数据中这极不可能。"
        )

    def test_each_snapshot_has_four_vertices(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """每个快照都包含 4 个顶点的状态。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        for idx, snapshot in enumerate(timeline):
            assert len(snapshot.vertex_states) == 4, (
                f"snapshot[{idx}]: 期望 4 个顶点，"
                f"实际 {len(snapshot.vertex_states)} 个"
            )
            vertices = {s.vertex for s in snapshot.vertex_states}
            assert vertices == set(AssetVertex), (
                f"snapshot[{idx}]: 顶点集合不完整：{vertices}"
            )

    def test_trigger_edge_is_valid(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """每个快照的 trigger_edge 是合法的顶点对。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        valid_vertices = set(AssetVertex)
        for idx, snapshot in enumerate(timeline):
            va, vb = snapshot.trigger_edge
            assert va in valid_vertices, (
                f"snapshot[{idx}]: trigger_edge[0]={va} 不在顶点集合中"
            )
            assert vb in valid_vertices, (
                f"snapshot[{idx}]: trigger_edge[1]={vb} 不在顶点集合中"
            )
            assert va != vb, (
                f"snapshot[{idx}]: trigger_edge 是自环 ({va}, {vb})"
            )

    def test_net_flow_in_range(
        self, all_edge_data: dict[str, object]
    ) -> None:
        """每个快照中每个顶点的 net_flow 在 [-3, +3] 范围内。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        for idx, snapshot in enumerate(timeline):
            for state in snapshot.vertex_states:
                assert -3 <= state.net_flow <= 3, (
                    f"snapshot[{idx}] {state.vertex.value}: "
                    f"net_flow={state.net_flow} 超出 [-3, +3]"
                )


# ── 可观测性：打印时间序列头尾 ─────────────────────────────


@pytest.mark.slow
class TestFlowTimelineObservability:
    """打印时间序列头尾快照，供人工审查。"""

    def test_print_timeline_summary(
        self,
        all_edge_data: dict[str, object],
    ) -> None:
        """输出时间序列摘要和头尾快照（不作额外断言，纯观测）。"""
        timeline: list[FlowSnapshot] = all_edge_data["timeline"]  # type: ignore[assignment]
        stroke_counts: dict[str, int] = all_edge_data["stroke_counts"]  # type: ignore[assignment]

        print("\n" + "=" * 70)
        print("流转状态时间序列摘要（真实 ETF 日线数据）")
        print("=" * 70)

        # 各边笔数
        print("\n各边笔数：")
        for label, count in stroke_counts.items():
            print(f"  {label}: {count} 笔")
        print(f"  总计: {sum(stroke_counts.values())} 笔")

        print(f"\n时间序列长度: {len(timeline)} 个快照")
        if timeline:
            print(f"起始时间: {timeline[0].timestamp}")
            print(f"结束时间: {timeline[-1].timestamp}")

        # 打印头 5 个快照
        n_show = min(5, len(timeline))
        print(f"\n--- 头 {n_show} 个快照 ---")
        for i in range(n_show):
            _print_snapshot(i, timeline[i])

        # 打印尾 5 个快照
        if len(timeline) > 10:
            print(f"\n--- 尾 5 个快照 ---")
            for i in range(len(timeline) - 5, len(timeline)):
                _print_snapshot(i, timeline[i])

        # 统计共振分布
        resonance_counts: dict[ResonanceStrength, int] = {
            s: 0 for s in ResonanceStrength
        }
        for snapshot in timeline:
            for state in snapshot.vertex_states:
                resonance_counts[state.strength] += 1

        print("\n共振强度分布（顶点 x 快照数）：")
        total_entries = len(timeline) * 4
        for strength, count in resonance_counts.items():
            pct = count / total_entries * 100 if total_entries else 0
            print(f"  {strength.value}: {count} ({pct:.1f}%)")

        # 找到第一个共振时间点
        first_resonance_idx = None
        for i, snapshot in enumerate(timeline):
            for state in snapshot.vertex_states:
                if state.strength != ResonanceStrength.NONE:
                    first_resonance_idx = i
                    break
            if first_resonance_idx is not None:
                break

        if first_resonance_idx is not None:
            print(
                f"\n首次共振出现在 snapshot[{first_resonance_idx}] "
                f"(t={timeline[first_resonance_idx].timestamp})"
            )

        print("=" * 70)

        # 唯一断言：时间序列非空
        assert len(timeline) > 0


def _print_snapshot(idx: int, snapshot: FlowSnapshot) -> None:
    """打印单个快照的详细信息。"""
    va_trigger, vb_trigger = snapshot.trigger_edge
    print(
        f"  [{idx}] t={snapshot.timestamp}  "
        f"trigger={va_trigger.value}/{vb_trigger.value}"
    )
    for state in snapshot.vertex_states:
        flow_type = (
            "汇" if state.net_flow > 0 else ("源" if state.net_flow < 0 else "—")
        )
        print(
            f"       {state.vertex.value:6s}: "
            f"net={state.net_flow:+d} ({flow_type}) "
            f"[{state.strength.value}]"
        )
