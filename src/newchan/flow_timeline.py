"""流转状态时间序列：四矩阵流场的动态演化。

定义编号：#14 扩展（liuzhuan.md）
概念层级：跨标的维度·拓扑层（L2）

概念溯源：
  [新缠论] 结构同步而非时钟同步——不同边有各自的笔节奏，
  时间序列在笔事件发生时更新，不是按固定时钟采样。

核心操作：
  1. 收集所有边的笔端点事件（timestamp + direction）
  2. 按时间排序
  3. 维护各边当前方向状态（初始 = EQUILIBRIUM）
  4. 每个事件触发时重新聚合 → FlowSnapshot
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from newchan.capital_flow import FlowDirection
from newchan.flow_relation import (
    EdgeFlowInput,
    VertexFlowState,
    aggregate_vertex_flows,
)
from newchan.matrix_topology import AssetVertex


# ====================================================================
# 数据结构
# ====================================================================


@dataclass(frozen=True, slots=True)
class EdgeEvent:
    """一条边上的笔方向变化事件。

    Attributes
    ----------
    vertex_a : AssetVertex
        边的一端。
    vertex_b : AssetVertex
        边的另一端。
    direction : FlowDirection
        笔完成时的资本流转方向。
    timestamp : Any
        笔端点时间戳（通常为 pd.Timestamp 或 datetime）。
    """

    vertex_a: AssetVertex
    vertex_b: AssetVertex
    direction: FlowDirection
    timestamp: Any


@dataclass(frozen=True, slots=True)
class FlowSnapshot:
    """单个时刻的四矩阵流转状态快照。

    Attributes
    ----------
    timestamp : Any
        触发此快照的事件时间戳。
    trigger_edge : tuple[AssetVertex, AssetVertex]
        哪条边的笔变化触发了此快照。
    vertex_states : tuple[VertexFlowState, ...]
        4 个顶点的流转状态。
    """

    timestamp: Any
    trigger_edge: tuple[AssetVertex, AssetVertex]
    vertex_states: tuple[VertexFlowState, ...]


# ====================================================================
# 核心函数
# ====================================================================


def _make_edge_key(va: AssetVertex, vb: AssetVertex) -> frozenset[AssetVertex]:
    """边的无序键。"""
    return frozenset([va, vb])


def _current_state_to_edges(
    current: dict[frozenset[AssetVertex], tuple[AssetVertex, AssetVertex, FlowDirection]],
) -> list[EdgeFlowInput]:
    """从当前状态构建 6 条 EdgeFlowInput。"""
    edges: list[EdgeFlowInput] = []
    for _key, (va, vb, direction) in sorted(
        current.items(), key=lambda item: (min(v.value for v in item[0]), max(v.value for v in item[0]))
    ):
        edges.append(EdgeFlowInput(vertex_a=va, vertex_b=vb, direction=direction))
    return edges


def _resolve_direction(
    event: EdgeEvent,
    stored_va: AssetVertex,
) -> FlowDirection:
    """解析事件方向，考虑顶点顺序翻转。"""
    if event.vertex_a == stored_va:
        return event.direction
    if event.direction == FlowDirection.A_TO_B:
        return FlowDirection.B_TO_A
    if event.direction == FlowDirection.B_TO_A:
        return FlowDirection.A_TO_B
    return event.direction


def build_flow_timeline(events: list[EdgeEvent]) -> list[FlowSnapshot]:
    """从笔事件序列构建流转状态时间序列。"""
    if not events:
        return []

    sorted_events = sorted(events, key=lambda e: e.timestamp)

    from itertools import combinations
    current: dict[frozenset[AssetVertex], tuple[AssetVertex, AssetVertex, FlowDirection]] = {}
    for va, vb in combinations(AssetVertex, 2):
        current[frozenset([va, vb])] = (va, vb, FlowDirection.EQUILIBRIUM)

    snapshots: list[FlowSnapshot] = []
    for event in sorted_events:
        key = _make_edge_key(event.vertex_a, event.vertex_b)

        if key in current:
            stored_va, stored_vb, _ = current[key]
            direction = _resolve_direction(event, stored_va)
            current[key] = (stored_va, stored_vb, direction)

        edge_inputs = _current_state_to_edges(current)
        vertex_states = aggregate_vertex_flows(edge_inputs)

        snapshots.append(
            FlowSnapshot(
                timestamp=event.timestamp,
                trigger_edge=(event.vertex_a, event.vertex_b),
                vertex_states=tuple(vertex_states),
            )
        )

    return snapshots
