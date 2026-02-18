"""流转关系端到端集成测试。

用真实 ETF 缓存数据跑通完整管线：
  缓存 OHLCV → 比价K线 → 包含处理 → 分型 → 笔 → StrokeFlow → EdgeFlowInput
  → aggregate_vertex_flows → detect_resonance → check_conservation

验证：
  1. 每条比价边能产出至少 1 根笔（管线贯通）
  2. 最后一笔的 StrokeFlow.direction ∈ {A_TO_B, B_TO_A}（非均衡）
  3. 6 条边聚合后守恒约束恒成立
  4. 共振检测结果结构正确

数据依赖：
  .cache/ 中需要 SPY/GLD/TLT 三个 ETF 的日线缓存。
  缺少 REAL_ESTATE 代表 (IYR/VNQ)，用 SLV 替代测试拓扑完整性。

概念溯源：
  [新缠论] 流转关系端到端管线验证（#14 liuzhuan.md）
  谱系引用：023-equivalence-relation-formalization（已结算）
"""

from __future__ import annotations

from itertools import combinations

import pandas as pd
import pytest

from newchan.cache import load_df
from newchan.capital_flow import FlowDirection, strokes_to_flows
from newchan.equivalence import EquivalencePair, make_ratio_kline
from newchan.flow_relation import (
    EdgeFlowInput,
    ResonanceStrength,
    VertexFlowState,
    aggregate_vertex_flows,
    check_conservation,
    detect_resonance,
)
from newchan.matrix_topology import AssetVertex
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import Stroke, strokes_from_fractals


# ── 标的→顶点 映射 ─────────────────────────────────────────
# 真实 ETF 到四矩阵顶点的映射。
# REAL_ESTATE 缺数据，用 SLV（白银）暂时占位以测试拓扑完整性。

SYMBOL_TO_VERTEX: dict[str, AssetVertex] = {
    "SPY": AssetVertex.EQUITY,
    "GLD": AssetVertex.COMMODITY,
    "TLT": AssetVertex.CASH,
    "SLV": AssetVertex.REAL_ESTATE,  # 代用
}

# 所有可用标的
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


# ── helpers ────────────────────────────────────────────────


def _latest_flow_direction(
    pair: EquivalencePair, strokes: list[Stroke]
) -> FlowDirection:
    """取最后一笔的资本流转方向。"""
    if not strokes:
        return FlowDirection.EQUILIBRIUM
    flows = strokes_to_flows(pair, strokes)
    return flows[-1].direction


def _build_edge_flow_input(
    sym_a: str,
    sym_b: str,
    direction: FlowDirection,
) -> EdgeFlowInput:
    """将 (sym, sym, FlowDirection) 映射为 EdgeFlowInput(vertex, vertex, direction)。

    关键对齐：
      StrokeFlow 以 pair(sym_a, sym_b) 定义 A_TO_B / B_TO_A。
      EdgeFlowInput 以 vertex_a / vertex_b 定义方向。
      当 pair 的 sym_a→vertex_a 且 sym_b→vertex_b 时，方向直接透传。
      当 pair 的 sym_a→vertex_b 且 sym_b→vertex_a 时，方向需要翻转。
    """
    va = SYMBOL_TO_VERTEX[sym_a]
    vb = SYMBOL_TO_VERTEX[sym_b]

    # EdgeFlowInput 的 vertex_a/vertex_b 不区分顺序（frozenset 去重），
    # 但 direction 的语义与 vertex_a/vertex_b 的顺序绑定。
    # 保持 va < vb（按 enum value）以与 aggregate_vertex_flows 的去重逻辑一致。
    if va.value > vb.value:
        va, vb = vb, va
        # 翻转方向
        if direction == FlowDirection.A_TO_B:
            direction = FlowDirection.B_TO_A
        elif direction == FlowDirection.B_TO_A:
            direction = FlowDirection.A_TO_B

    return EdgeFlowInput(vertex_a=va, vertex_b=vb, direction=direction)


# ── 单边管线贯通 ───────────────────────────────────────────


class TestSingleEdgePipeline:
    """每条比价边的管线贯通测试。"""

    @pytest.mark.parametrize(
        "sym_a,sym_b",
        list(combinations(ALL_SYMBOLS, 2)),
        ids=[f"{a}/{b}" for a, b in combinations(ALL_SYMBOLS, 2)],
    )
    def test_pipeline_produces_strokes(
        self, cached_data: dict, sym_a: str, sym_b: str
    ) -> None:
        """比价K线管线能产出笔序列。"""
        ratio_kline = make_ratio_kline(
            cached_data[sym_a], cached_data[sym_b]
        )
        assert not ratio_kline.empty, f"{sym_a}/{sym_b} 比价K线为空"

        df_merged, _ = merge_inclusion(ratio_kline)
        fractals = fractals_from_merged(df_merged)
        strokes = strokes_from_fractals(df_merged, fractals)

        # 日线数据量足够大（> 1年），应该能产出笔
        assert len(strokes) >= 1, (
            f"{sym_a}/{sym_b}: 无笔产出 "
            f"(merged={len(df_merged)}, fractals={len(fractals)})"
        )

    @pytest.mark.parametrize(
        "sym_a,sym_b",
        list(combinations(ALL_SYMBOLS, 2)),
        ids=[f"{a}/{b}" for a, b in combinations(ALL_SYMBOLS, 2)],
    )
    def test_stroke_flow_direction_not_equilibrium(
        self, cached_data: dict, sym_a: str, sym_b: str
    ) -> None:
        """最后一笔的方向不应该是均衡（真实数据中比价不会恰好不变）。"""
        ratio_kline = make_ratio_kline(
            cached_data[sym_a], cached_data[sym_b]
        )
        df_merged, _ = merge_inclusion(ratio_kline)
        fractals = fractals_from_merged(df_merged)
        strokes = strokes_from_fractals(df_merged, fractals)

        if not strokes:
            pytest.skip(f"{sym_a}/{sym_b} 无笔")

        pair = EquivalencePair(sym_a=sym_a, sym_b=sym_b)
        direction = _latest_flow_direction(pair, strokes)
        assert direction in (FlowDirection.A_TO_B, FlowDirection.B_TO_A), (
            f"{sym_a}/{sym_b}: 最后一笔方向为均衡，不符预期"
        )


# ── 四矩阵聚合 ────────────────────────────────────────────


class TestFourMatrixAggregation:
    """用真实数据构建完整四矩阵并验证拓扑性质。"""

    def _build_all_edges(
        self, cached_data: dict
    ) -> list[EdgeFlowInput]:
        """从 4 个标的构建 6 条边的 EdgeFlowInput。"""
        edge_inputs: list[EdgeFlowInput] = []
        for sym_a, sym_b in combinations(ALL_SYMBOLS, 2):
            pair = EquivalencePair(sym_a=sym_a, sym_b=sym_b)
            ratio_kline = make_ratio_kline(
                cached_data[sym_a], cached_data[sym_b]
            )
            df_merged, _ = merge_inclusion(ratio_kline)
            fractals = fractals_from_merged(df_merged)
            strokes = strokes_from_fractals(df_merged, fractals)

            direction = _latest_flow_direction(pair, strokes)
            edge_input = _build_edge_flow_input(sym_a, sym_b, direction)
            edge_inputs.append(edge_input)

        return edge_inputs

    def test_six_edges_produced(self, cached_data: dict) -> None:
        """4 个标的应产出恰好 6 条边。"""
        edges = self._build_all_edges(cached_data)
        assert len(edges) == 6

    def test_conservation_holds(self, cached_data: dict) -> None:
        """守恒约束：Σnet(V) = 0。"""
        edges = self._build_all_edges(cached_data)
        states = aggregate_vertex_flows(edges)
        assert check_conservation(states), (
            f"守恒约束失败！states={[(s.vertex.value, s.net_flow) for s in states]}"
        )

    def test_four_vertices_returned(self, cached_data: dict) -> None:
        """聚合结果包含 4 个顶点。"""
        edges = self._build_all_edges(cached_data)
        states = aggregate_vertex_flows(edges)
        assert len(states) == 4
        vertices = {s.vertex for s in states}
        assert vertices == set(AssetVertex)

    def test_resonance_detection_structure(self, cached_data: dict) -> None:
        """共振检测返回正确结构。"""
        edges = self._build_all_edges(cached_data)
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)

        assert len(resonances) == 4
        for r in resonances:
            assert isinstance(r, VertexFlowState)
            assert isinstance(r.strength, ResonanceStrength)
            assert isinstance(r.net_flow, int)
            assert -3 <= r.net_flow <= 3

    def test_net_flow_range(self, cached_data: dict) -> None:
        """每个顶点 net_flow ∈ [-3, +3]（3条关联边）。"""
        edges = self._build_all_edges(cached_data)
        states = aggregate_vertex_flows(edges)
        for s in states:
            assert -3 <= s.net_flow <= 3, (
                f"{s.vertex.value}: net_flow={s.net_flow} 超出范围"
            )

    def test_resonance_classification_consistent(
        self, cached_data: dict
    ) -> None:
        """strength 与 |net_flow| 一致。"""
        edges = self._build_all_edges(cached_data)
        states = aggregate_vertex_flows(edges)
        for s in states:
            abs_net = abs(s.net_flow)
            if abs_net >= 3:
                expected = ResonanceStrength.STRONG
            elif abs_net >= 2:
                expected = ResonanceStrength.WEAK
            else:
                expected = ResonanceStrength.NONE
            assert s.strength == expected, (
                f"{s.vertex.value}: |net|={abs_net} → "
                f"expected {expected}, got {s.strength}"
            )


# ── 可观测性：打印流转状态快照 ───────────────────────────────


class TestObservability:
    """打印真实数据的流转状态，供人工检查。"""

    def test_print_flow_snapshot(
        self, cached_data: dict, capsys: pytest.CaptureFixture
    ) -> None:
        """输出当前流转状态快照（不作断言，纯观测）。"""
        print("\n" + "=" * 60)
        print("流转状态快照（真实 ETF 日线数据）")
        print("=" * 60)

        edge_directions: list[tuple[str, str, FlowDirection, int]] = []
        edge_inputs: list[EdgeFlowInput] = []

        for sym_a, sym_b in combinations(ALL_SYMBOLS, 2):
            pair = EquivalencePair(sym_a=sym_a, sym_b=sym_b)
            ratio_kline = make_ratio_kline(
                cached_data[sym_a], cached_data[sym_b]
            )
            df_merged, _ = merge_inclusion(ratio_kline)
            fractals = fractals_from_merged(df_merged)
            strokes = strokes_from_fractals(df_merged, fractals)

            direction = _latest_flow_direction(pair, strokes)
            n_strokes = len(strokes)
            edge_directions.append((sym_a, sym_b, direction, n_strokes))

            edge_input = _build_edge_flow_input(sym_a, sym_b, direction)
            edge_inputs.append(edge_input)

        print("\n边方向：")
        for sym_a, sym_b, d, n in edge_directions:
            va = SYMBOL_TO_VERTEX[sym_a]
            vb = SYMBOL_TO_VERTEX[sym_b]
            print(
                f"  {sym_a}({va.value}) / {sym_b}({vb.value}): "
                f"{d.value}  ({n} 笔)"
            )

        states = aggregate_vertex_flows(edge_inputs)
        resonances = detect_resonance(states)

        print("\n顶点状态：")
        for s in resonances:
            flow_type = "汇" if s.net_flow > 0 else ("源" if s.net_flow < 0 else "—")
            print(
                f"  {s.vertex.value}: net={s.net_flow:+d} "
                f"({flow_type}) [{s.strength.value}]"
            )

        conservation = check_conservation(states)
        print(f"\n守恒约束: {'✓' if conservation else '✗'}")
        print("=" * 60)

        # 唯一断言：守恒必须成立
        assert conservation
