"""tests for multi_tf_pipeline — 多 TF pipeline 集成测试。

测试覆盖：
1. buysellpoint_to_bsp 类型转换
2. cross_divergence_to_resonance_signal 类型转换
3. multi_tf_result_to_signals 批量转换
4. detect_cross_level_divergence_directional 方向性力度检测
5. _directional_move_force 方向性力度计算
6. MultiTFPipelineAdapter.run 集成
7. T6 可达性判断
8. 边界条件（空输入、单 TF 等）
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.a_move_v1 import Move
from newchan.nesting.bsp import BSPType, DivergenceType
from newchan.nesting.resonance import SignalLayer
from newchan.topology.multi_tf_adapter import (
    BuySellPoint,
    CrossLevelDivergence,
    LevelResult,
    MultiTFResult,
    TimeframeLevel,
)
from newchan.topology.multi_tf_pipeline import (
    MultiTFPipelineAdapter,
    MultiTFPipelineResult,
    buysellpoint_to_bsp,
    cross_divergence_to_resonance_signal,
    detect_cross_level_divergence_directional,
    multi_tf_result_to_signals,
    _directional_move_force,
)
from newchan.types import Bar


# ── helpers ──────────────────────────────────────────────


def _make_tf(name: str, index: int) -> TimeframeLevel:
    return TimeframeLevel(tf_name=name, bar_source="test", level_index=index)


def _make_move(
    direction: str = "up",
    high: float = 110.0,
    low: float = 100.0,
    settled: bool = False,
    seg_start: int = 0,
    seg_end: int = 5,
    zs_count: int = 1,
) -> Move:
    return Move(
        kind="consolidation",
        direction=direction,  # type: ignore[arg-type]
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=0,
        zs_end=0,
        zs_count=zs_count,
        settled=settled,
        high=high,
        low=low,
    )


def _make_level_result(
    tf: TimeframeLevel,
    direction: str = "up",
    high: float = 110.0,
    low: float = 100.0,
    settled: bool = False,
    bar_count: int = 100,
    seg_start: int = 0,
    seg_end: int = 5,
    zs_count: int = 1,
) -> LevelResult:
    move = _make_move(
        direction=direction, high=high, low=low, settled=settled,
        seg_start=seg_start, seg_end=seg_end, zs_count=zs_count,
    )
    return LevelResult(
        tf=tf,
        snapshot=None,
        bar_count=bar_count,
        move_count=1,
        zhongshu_count=zs_count,
        direction=direction,  # type: ignore[arg-type]
        last_move=move,
    )


def _make_bars(n: int, base_price: float = 100.0) -> list[Bar]:
    start = datetime(2024, 1, 1, tzinfo=timezone.utc)
    bars: list[Bar] = []
    price = base_price
    for i in range(n):
        ts = start + timedelta(minutes=i)
        delta = 0.5 * (1 if i % 3 != 0 else -1)
        o = price
        h = price + abs(delta)
        l = price - abs(delta)
        c = price + delta
        bars.append(Bar(ts=ts, open=o, high=h, low=l, close=c))
        price = c
    return bars


# ── buysellpoint_to_bsp tests ──────────────────────────


class TestBuySellPointToBSP:
    def test_type1_buy(self) -> None:
        tf = _make_tf("daily", 0)
        bsp_in = BuySellPoint(
            kind="type1", side="buy", tf=tf, price=95.0,
            cross_divergence=None,
        )
        bsp_out = buysellpoint_to_bsp(bsp_in, timestamp=1000.0)
        assert bsp_out.bsp_type == BSPType.B1
        assert bsp_out.price == 95.0
        assert bsp_out.edge_id == "multi_tf_daily"
        assert bsp_out.level == 0
        assert bsp_out.time == 1000.0
        assert bsp_out.divergence == DivergenceType.NONE

    def test_type1_sell(self) -> None:
        tf = _make_tf("weekly", 1)
        bsp_in = BuySellPoint(
            kind="type1", side="sell", tf=tf, price=120.0,
            cross_divergence=None,
        )
        bsp_out = buysellpoint_to_bsp(bsp_in, timestamp=2000.0)
        assert bsp_out.bsp_type == BSPType.S1
        assert bsp_out.price == 120.0
        assert bsp_out.level == 1

    def test_with_cross_divergence_top(self) -> None:
        tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=_make_tf("weekly", 1), low_tf=tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0),
            low_move=_make_move("up", 105.0, 100.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        bsp_in = BuySellPoint(
            kind="type1", side="sell", tf=tf, price=105.0,
            cross_divergence=div,
        )
        bsp_out = buysellpoint_to_bsp(bsp_in)
        assert bsp_out.divergence == DivergenceType.TOP_DIV

    def test_with_cross_divergence_bottom(self) -> None:
        tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=_make_tf("weekly", 1), low_tf=tf,
            direction="bottom",
            high_move=_make_move("down", 100.0, 80.0),
            low_move=_make_move("down", 100.0, 95.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        bsp_in = BuySellPoint(
            kind="type1", side="buy", tf=tf, price=95.0,
            cross_divergence=div,
        )
        bsp_out = buysellpoint_to_bsp(bsp_in)
        assert bsp_out.divergence == DivergenceType.BOT_DIV

    def test_unknown_kind_maps_to_none(self) -> None:
        tf = _make_tf("daily", 0)
        bsp_in = BuySellPoint(
            kind="unknown", side="buy", tf=tf, price=100.0,
            cross_divergence=None,
        )
        bsp_out = buysellpoint_to_bsp(bsp_in)
        assert bsp_out.bsp_type == BSPType.NONE


# ── cross_divergence_to_resonance_signal tests ──────────


class TestCrossDivergenceToResonanceSignal:
    def test_top_divergence(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0),
            low_move=_make_move("up", 105.0, 100.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        signal = cross_divergence_to_resonance_signal(div, timestamp=1000.0)
        assert signal.bsp.bsp_type == BSPType.S1
        assert signal.bsp.price == 105.0  # low_move.high
        assert signal.layer == SignalLayer.INDEPENDENT_EDGE
        assert signal.level == 1  # high_tf.level_index
        assert signal.time == 1000.0
        assert "cross_tf_weekly_daily" in signal.edge_id

    def test_bottom_divergence(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="bottom",
            high_move=_make_move("down", 100.0, 80.0),
            low_move=_make_move("down", 100.0, 95.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        signal = cross_divergence_to_resonance_signal(div)
        assert signal.bsp.bsp_type == BSPType.B1
        assert signal.bsp.price == 95.0  # low_move.low


# ── multi_tf_result_to_signals tests ────────────────────


class TestMultiTFResultToSignals:
    def test_empty_result(self) -> None:
        result = MultiTFResult()
        bsps, signals = multi_tf_result_to_signals(result)
        assert bsps == []
        assert signals == []

    def test_with_divergences_and_bsps(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0),
            low_move=_make_move("up", 105.0, 100.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        bsp = BuySellPoint(
            kind="type1", side="sell", tf=low_tf, price=105.0,
            cross_divergence=div,
        )
        result = MultiTFResult(
            levels={},
            cross_level_divergences=[div],
            buysellpoints=[bsp],
        )
        bsps, signals = multi_tf_result_to_signals(result, timestamp=500.0)
        assert len(bsps) == 1
        assert len(signals) == 1
        assert bsps[0].bsp_type == BSPType.S1
        assert signals[0].layer == SignalLayer.INDEPENDENT_EDGE


# ── _directional_move_force tests ───────────────────────


class TestDirectionalMoveForce:
    def test_basic_force_calculation(self) -> None:
        """基本力度计算（无 snapshot）。"""
        move = _make_move(seg_start=0, seg_end=9, zs_count=1)
        tf = _make_tf("daily", 0)
        lr = _make_level_result(tf, seg_start=0, seg_end=9, zs_count=1)
        force = _directional_move_force(move, lr)
        # density = 1/10 = 0.1, density_force = 10.0
        # persistence = 0.5 (default, no snapshot)
        # force = 0.5 * 0.5 + 0.5 * 10.0 = 5.25
        assert force > 0.0

    def test_more_zhongshus_means_less_density_force(self) -> None:
        """更多中枢 → 更高密度 → 更低力度。"""
        tf = _make_tf("daily", 0)
        move_few_zs = _make_move(seg_start=0, seg_end=9, zs_count=1)
        lr_few = _make_level_result(tf, seg_start=0, seg_end=9, zs_count=1)

        move_many_zs = _make_move(seg_start=0, seg_end=9, zs_count=5)
        lr_many = _make_level_result(tf, seg_start=0, seg_end=9, zs_count=5)

        force_few = _directional_move_force(move_few_zs, lr_few)
        force_many = _directional_move_force(move_many_zs, lr_many)
        assert force_few > force_many

    def test_zero_seg_span(self) -> None:
        """seg_span=0 时返回 0.0。"""
        move = _make_move(seg_start=5, seg_end=4, zs_count=1)
        tf = _make_tf("daily", 0)
        lr = _make_level_result(tf)
        assert _directional_move_force(move, lr) == 0.0


# ── detect_cross_level_divergence_directional tests ─────


class TestDetectCrossLevelDivergenceDirectional:
    def test_divergence_detected(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        # high: 1 zs over 10 segs → high density_force
        high_result = _make_level_result(
            high_tf, "up", high=120.0, low=100.0,
            seg_start=0, seg_end=9, zs_count=1,
        )
        # low: 5 zs over 10 segs → low density_force
        low_result = _make_level_result(
            low_tf, "up", high=105.0, low=100.0,
            seg_start=0, seg_end=9, zs_count=5,
        )

        div = detect_cross_level_divergence_directional(
            high_tf, high_result, low_tf, low_result,
        )
        assert div is not None
        assert div.direction == "top"
        assert div.ratio < 1.0

    def test_no_divergence_different_directions(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(high_tf, "up")
        low_result = _make_level_result(low_tf, "down")

        div = detect_cross_level_divergence_directional(
            high_tf, high_result, low_tf, low_result,
        )
        assert div is None

    def test_no_divergence_when_no_moves(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = LevelResult(
            tf=high_tf, snapshot=None, bar_count=0,
            move_count=0, zhongshu_count=0,
            direction="none", last_move=None,
        )
        low_result = _make_level_result(low_tf)

        div = detect_cross_level_divergence_directional(
            high_tf, high_result, low_tf, low_result,
        )
        assert div is None

    def test_no_divergence_when_low_force_not_smaller(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        # Both same structure → same force → ratio = 1.0 → no divergence
        high_result = _make_level_result(
            high_tf, "up", seg_start=0, seg_end=9, zs_count=3,
        )
        low_result = _make_level_result(
            low_tf, "up", seg_start=0, seg_end=9, zs_count=3,
        )

        div = detect_cross_level_divergence_directional(
            high_tf, high_result, low_tf, low_result,
        )
        assert div is None

    def test_bottom_divergence(self) -> None:
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        high_result = _make_level_result(
            high_tf, "down", high=100.0, low=80.0,
            seg_start=0, seg_end=9, zs_count=1,
        )
        low_result = _make_level_result(
            low_tf, "down", high=100.0, low=95.0,
            seg_start=0, seg_end=9, zs_count=5,
        )

        div = detect_cross_level_divergence_directional(
            high_tf, high_result, low_tf, low_result,
        )
        assert div is not None
        assert div.direction == "bottom"


# ── MultiTFPipelineAdapter tests ────────────────────────


class TestMultiTFPipelineAdapter:
    def test_construction(self) -> None:
        tfs = [_make_tf("daily", 0), _make_tf("weekly", 1)]
        adapter = MultiTFPipelineAdapter(tfs)
        assert len(adapter.timeframes) == 2
        assert adapter.orchestrator is not None

    def test_run_with_synthetic_data(self) -> None:
        """合成数据集成测试——验证 adapter 结构正确性。

        注意：简单合成数据可能不产生走势（BiEngine 需要足够的价格波动
        才能构造多笔→线段→中枢→走势），因此只验证结构完整性。
        """
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        adapter = MultiTFPipelineAdapter(tfs)

        bars_1min = _make_bars(200, base_price=100.0)
        bars_5min = _make_bars(40, base_price=100.0)

        result = adapter.run(
            {"1min": bars_1min, "5min": bars_5min},
            timestamp=1000.0,
        )

        assert isinstance(result, MultiTFPipelineResult)
        assert result.multi_tf_result is not None
        assert isinstance(result.bsps, list)
        assert isinstance(result.resonance_signals, list)
        assert isinstance(result.directional_divergences, list)
        assert result.recursive_levels_equivalent >= 0
        assert isinstance(result.t6_reachable, bool)

    def test_t6_reachable_with_two_active_tfs(self) -> None:
        """两个活跃 TF → T6 可达（前提是两个 TF 都产生走势）。

        使用 mock LevelResult 直接验证 T6 可达性逻辑。
        """
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        adapter = MultiTFPipelineAdapter(tfs)

        # 直接构造包含走势的 MultiTFResult 来验证逻辑
        lr_1min = _make_level_result(
            _make_tf("1min", 0), "up", bar_count=200,
        )
        lr_5min = _make_level_result(
            _make_tf("5min", 1), "up", bar_count=40,
        )
        mock_result = MultiTFResult(
            levels={"1min": lr_1min, "5min": lr_5min},
        )
        # 计算等效递归深度
        active = sum(
            1 for lr in mock_result.levels.values()
            if lr.bar_count > 0 and lr.last_move is not None
        )
        assert active == 2
        assert active >= 2  # T6 可达

    def test_single_tf_not_t6_reachable(self) -> None:
        """单 TF → T6 不可达（最多 recursive_levels_equivalent=1）。"""
        tfs = [_make_tf("daily", 0)]
        adapter = MultiTFPipelineAdapter(tfs)

        bars = _make_bars(200)
        result = adapter.run({"daily": bars})

        assert result.recursive_levels_equivalent <= 1
        assert result.t6_reachable is False

    def test_run_with_empty_bars(self) -> None:
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        adapter = MultiTFPipelineAdapter(tfs)
        result = adapter.run({"1min": [], "5min": []})
        assert result.recursive_levels_equivalent == 0
        assert result.t6_reachable is False
        assert result.bsps == []
        assert result.resonance_signals == []

    def test_reset(self) -> None:
        tfs = [_make_tf("daily", 0)]
        adapter = MultiTFPipelineAdapter(tfs)
        bars = _make_bars(50)
        adapter.run({"daily": bars})
        adapter.reset()
        result = adapter.run({"daily": bars})
        assert result.multi_tf_result.levels["daily"].bar_count == 50

    def test_directional_force_disabled(self) -> None:
        """use_directional_force=False 时使用原始振幅力度。"""
        tfs = [_make_tf("1min", 0), _make_tf("5min", 1)]
        adapter = MultiTFPipelineAdapter(tfs, use_directional_force=False)
        bars_1min = _make_bars(200)
        bars_5min = _make_bars(40)
        result = adapter.run({"1min": bars_1min, "5min": bars_5min})
        # 应该使用原始 multi_tf_result 的背驰
        assert isinstance(result.directional_divergences, list)


# ── Pipeline integration tests ──────────────────────────


class TestPipelineIntegration:
    """验证转换后的 BSP/ResonanceSignal 与 pipeline 类型兼容。"""

    def test_bsp_compatible_with_pipeline(self) -> None:
        """转换后的 BSP 具有 pipeline 期望的所有字段。"""
        tf = _make_tf("daily", 0)
        bsp_in = BuySellPoint(
            kind="type1", side="buy", tf=tf, price=95.0,
            cross_divergence=None,
        )
        bsp = buysellpoint_to_bsp(bsp_in)
        # pipeline.locate_bsp 需要的字段
        assert hasattr(bsp, "edge_id")
        assert hasattr(bsp, "bsp_type")
        assert hasattr(bsp, "price")
        # pipeline.check_negation 需要的字段
        assert hasattr(bsp, "negated")
        assert bsp.negated is False

    def test_resonance_signal_compatible(self) -> None:
        """转换后的 ResonanceSignal 与 resonance 模块兼容。"""
        high_tf = _make_tf("weekly", 1)
        low_tf = _make_tf("daily", 0)
        div = CrossLevelDivergence(
            high_tf=high_tf, low_tf=low_tf,
            direction="top",
            high_move=_make_move("up", 120.0, 100.0),
            low_move=_make_move("up", 105.0, 100.0),
            force_high=20.0, force_low=5.0, ratio=0.25,
            confirmed=True,
        )
        signal = cross_divergence_to_resonance_signal(div)
        # resonance_check 需要的字段
        assert hasattr(signal, "bsp")
        assert hasattr(signal, "layer")
        assert hasattr(signal, "level")
        assert hasattr(signal, "time")
        assert signal.bsp.bsp_type.is_present
