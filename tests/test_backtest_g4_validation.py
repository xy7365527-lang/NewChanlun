"""G4 回测框架验证 — 267号操作方法论三项下游推论的结构验证。

认识论标注：L1（合成数据管线验证）。
谱系引用：
  - 267号：操作方法论 v1 下游推论1-3
  - 349号：赋格状态机形式化定义
  - 265号：帕萨卡利亚三层模型

验证三项：
  1. 满仓满融降成本速度——成本递推公式 + 杠杆加速效应
  2. 区间套收敛排序效果——scan_candidates + tightness 排序
  3. 不换仓原则机会成本——持仓期间错过信号计数
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timedelta
from unittest.mock import MagicMock, patch

import pytest

from newchan.backtest.analysis import (
    CostReductionSpeed,
    CostReductionTimeSeries,
    OpportunityCostAnalysis,
    compute_cost_reduction_speed,
    compute_opportunity_cost,
    extract_cost_reduction_timeseries,
)
from newchan.backtest.full_pipeline import (
    BarStep,
    FullPipelineConfig,
    FullPipelineEngine,
    FullPipelineResult,
)
from newchan.backtest.state_machine import (
    CostReductionStateMachine,
)
from newchan.backtest.types import (
    StateMachineEvent,
    StateMachineState,
)
from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    FugueVoice,
    ShortDiffCycle,
    transition,
)
from newchan.trading.stock_scanner import (
    ScanCandidate,
    ScanResult,
    compute_nesting_tightness,
    scan_candidates,
    target_universe,
)
from newchan.topology.config_space import Configuration, WalkDirection
from newchan.types import Bar


# ═══════════════════════════════════════════════════════════════
# 工厂函数
# ═══════════════════════════════════════════════════════════════


def _make_bar(ts_offset: int, price: float) -> Bar:
    """创建测试 Bar。"""
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(minutes=ts_offset),
        open=price,
        high=price + 1.0,
        low=price - 1.0,
        close=price,
        volume=1000.0,
    )


def _make_fsm(
    *,
    equity: float = 100_000.0,
    margin: float = 100_000.0,
    sub_ratio: float = 0.3,
) -> CostReductionFSM:
    return CostReductionFSM.create(
        own_capital=equity, margin_amount=margin, sub_ratio=sub_ratio,
    )


def _open_position(
    fsm: CostReductionFSM, price: float = 10.0,
) -> CostReductionFSM:
    return transition(
        fsm,
        FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=price,
            level="30min",
        ),
    )


def _start_short_diff(
    fsm: CostReductionFSM, sell_price: float = 11.0,
) -> CostReductionFSM:
    return transition(
        fsm,
        FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=sell_price,
            level="5min",
        ),
    )


def _close_short_diff(
    fsm: CostReductionFSM, buy_price: float = 9.0,
) -> CostReductionFSM:
    return transition(
        fsm,
        FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=buy_price,
            level="5min",
        ),
    )


def _build_mock_result(
    *,
    steps: list[BarStep],
    transitions: list[tuple[int, str, str]] | None = None,
    final_fsm: CostReductionFSM | None = None,
    initial_capital: float = 100_000.0,
) -> FullPipelineResult:
    cfg = FullPipelineConfig(initial_capital=initial_capital)
    if final_fsm is None:
        final_fsm = CostReductionFSM.create(own_capital=initial_capital)
    return FullPipelineResult(
        config=cfg,
        steps=steps,
        fsm_transitions=transitions or [],
        final_fsm=final_fsm,
        total_bars=len(steps),
    )


def _mock_snapshot_with_buy_points(
    n_level1_buys: int = 0,
    n_recursive_buys: int = 0,
    recursive_level_ids: list[int] | None = None,
) -> MagicMock:
    """创建包含指定数量买点的 mock RecursiveOrchestratorSnapshot。"""
    snap = MagicMock()

    # Level 1 BSP
    bps = []
    for i in range(n_level1_buys):
        bp = MagicMock()
        bp.side = "buy"
        bp.confirmed = True
        bp.seg_idx = i
        bp.kind = "type1"
        bp.level_id = 1
        bp.price = 100.0
        bps.append(bp)
    snap.bsp_snapshot.buysellpoints = bps

    # Recursive snapshots
    recursive_snaps = []
    if recursive_level_ids is None:
        recursive_level_ids = list(range(2, 2 + n_recursive_buys))
    for j, lid in enumerate(recursive_level_ids):
        rs = MagicMock()
        rs.level_id = lid
        rs_bp = MagicMock()
        rs_bp.side = "buy"
        rs_bp.confirmed = True
        rs.buysellpoints = [rs_bp]
        recursive_snaps.append(rs)
    snap.recursive_snapshots = recursive_snaps

    # Move snapshot (for extract_d_reading)
    snap.move_snapshot.moves = []
    snap.zs_snapshot.zhongshus = []

    return snap


# ═══════════════════════════════════════════════════════════════
# 1. 满仓满融降成本速度验证（267号下游推论1）
# ═══════════════════════════════════════════════════════════════


class TestCostReductionCurve:
    """成本递推公式验证：cost(t) = cost(t-1) - profit(t) / total_shares。"""

    def test_single_short_diff_cost_reduction(self) -> None:
        """一次短差后成本下降符合公式。

        equity=100k, margin=100k → total_capital=200k
        entry_price=10 → shares=20000
        sub_ratio=0.3 → short_shares=6000
        sell@11, buy@9 → profit=(11-9)*6000=12000
        new_cost = 10 - 12000/20000 = 10 - 0.6 = 9.4
        """
        fsm = _make_fsm(equity=100_000, margin=100_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        assert fsm.state == CostState.POSITION_OPEN
        assert fsm.total_shares == 20_000.0
        assert fsm.cost_basis == 10.0

        fsm = _start_short_diff(fsm, sell_price=11.0)
        assert fsm.state == CostState.COST_REDUCING
        assert fsm.active_short_diff is not None
        assert fsm.active_short_diff.shares == 6_000.0

        fsm = _close_short_diff(fsm, buy_price=9.0)
        assert fsm.cost_basis == pytest.approx(9.4)
        assert fsm.cumulative_recovered == pytest.approx(12_000.0)
        assert fsm.active_short_diff is None

    def test_multiple_short_diffs_cumulative(self) -> None:
        """多次短差后累计回收和成本变化。

        三次短差：
        cycle1: sell@11, buy@9 → profit=12000, cost=9.4, recovered=12000
        cycle2: sell@10, buy@8 → profit=12000, cost=8.8, recovered=24000
        cycle3: sell@9.5, buy@7 → profit=15000, cost=8.05, recovered=39000
        """
        fsm = _make_fsm(equity=100_000, margin=100_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        # Cycle 1
        fsm = _start_short_diff(fsm, sell_price=11.0)
        fsm = _close_short_diff(fsm, buy_price=9.0)
        assert fsm.cost_basis == pytest.approx(9.4)
        assert fsm.cumulative_recovered == pytest.approx(12_000.0)

        # Cycle 2
        fsm = _start_short_diff(fsm, sell_price=10.0)
        fsm = _close_short_diff(fsm, buy_price=8.0)
        assert fsm.cost_basis == pytest.approx(8.8)
        assert fsm.cumulative_recovered == pytest.approx(24_000.0)

        # Cycle 3
        fsm = _start_short_diff(fsm, sell_price=9.5)
        fsm = _close_short_diff(fsm, buy_price=7.0)
        expected_profit_3 = (9.5 - 7.0) * 6_000.0  # 15000
        expected_cost_3 = 8.8 - expected_profit_3 / 20_000.0  # 8.8 - 0.75 = 8.05
        assert fsm.cost_basis == pytest.approx(expected_cost_3)
        assert fsm.cumulative_recovered == pytest.approx(39_000.0)

    def test_leverage_acceleration_effect(self) -> None:
        """杠杆加速效应：满融 vs 非融资的回收速度对比。

        满融：equity=100k, margin=100k → shares=20000, short_shares=6000
        无融资：equity=100k, margin=0 → shares=10000, short_shares=3000

        同样 sell@11, buy@9：
        满融 profit=12000 (recovery_rate=12%)
        无融资 profit=6000 (recovery_rate=6%)
        """
        # 满融
        fsm_leveraged = _make_fsm(equity=100_000, margin=100_000, sub_ratio=0.3)
        fsm_leveraged = _open_position(fsm_leveraged, price=10.0)
        fsm_leveraged = _start_short_diff(fsm_leveraged, sell_price=11.0)
        fsm_leveraged = _close_short_diff(fsm_leveraged, buy_price=9.0)

        # 无融资
        fsm_unleveraged = _make_fsm(equity=100_000, margin=0.0, sub_ratio=0.3)
        fsm_unleveraged = _open_position(fsm_unleveraged, price=10.0)
        fsm_unleveraged = _start_short_diff(fsm_unleveraged, sell_price=11.0)
        fsm_unleveraged = _close_short_diff(fsm_unleveraged, buy_price=9.0)

        # 满融回收额 = 2 * 无融资
        assert fsm_leveraged.cumulative_recovered == pytest.approx(
            2.0 * fsm_unleveraged.cumulative_recovered,
        )
        # 回收率（相对于自有资金）
        lev_rate = fsm_leveraged.cumulative_recovered / 100_000.0
        unlev_rate = fsm_unleveraged.cumulative_recovered / 100_000.0
        assert lev_rate == pytest.approx(2.0 * unlev_rate)

    def test_principal_withdrawal_threshold(self) -> None:
        """累计回收 >= own_capital 时自动转入 PRINCIPAL_WITHDRAWN。

        equity=10000, margin=10000 → total=20000
        price=10 → shares=2000, short_shares=600
        需要 ceil(10000/profit_per_cycle) 次短差。
        sell@15, buy@10 → profit=3000/cycle → 4 次即可。
        """
        fsm = _make_fsm(equity=10_000, margin=10_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        for i in range(3):
            fsm = _start_short_diff(fsm, sell_price=15.0)
            fsm = _close_short_diff(fsm, buy_price=10.0)
            assert fsm.state == CostState.COST_REDUCING

        # 第 4 次应触发 PRINCIPAL_WITHDRAWN（累计=12000 >= 10000）
        fsm = _start_short_diff(fsm, sell_price=15.0)
        fsm = _close_short_diff(fsm, buy_price=10.0)
        assert fsm.state == CostState.PRINCIPAL_WITHDRAWN
        assert fsm.cumulative_recovered >= 10_000.0


class TestCostReductionSpeedMetric:
    """analysis.compute_cost_reduction_speed 指标验证。"""

    def test_recovery_per_bar(self) -> None:
        """每 bar 平均回收 = total_recovered / bars_in_position。"""
        steps = [
            BarStep(
                bar_idx=i, bar_ts=float(i), k4_config_label="(+,+,+)",
                k4_polarity=3, scanner_selected=None,
                fsm_state="COST_REDUCING", cost_basis=100.0 - i * 5,
                cumulative_recovered=i * 500.0, total_shares=200.0,
            )
            for i in range(10)
        ]
        result = _build_mock_result(
            steps=steps,
            final_fsm=CostReductionFSM(
                state=CostState.COST_REDUCING,
                own_capital=100_000.0,
                margin_amount=100_000.0,
                total_shares=200.0,
                cost_basis=55.0,
                cumulative_recovered=4500.0,
                entry_price=100.0,
                entry_level="30min",
                sub_ratio=0.3,
                min_operable_level="",
                active_short_diff=None,
                completed_short_diffs=(),
                fugue_voices=(),
            ),
        )
        speed = compute_cost_reduction_speed(result)
        assert speed.bars_in_position == 10
        assert speed.total_recovered == 4500.0
        assert speed.recovery_per_bar == pytest.approx(450.0)

    def test_recovery_per_short_diff(self) -> None:
        """每次短差平均回收 = total_recovered / short_diff_count。"""
        # 3 个已完成短差
        completed = tuple(
            ShortDiffCycle(
                level="5min", shares=600.0,
                sell_price=11.0, buy_price=9.0, is_open=False,
            )
            for _ in range(3)
        )
        steps = [
            BarStep(
                bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                k4_polarity=3, scanner_selected=None,
                fsm_state="COST_REDUCING", cost_basis=90.0,
                cumulative_recovered=3600.0, total_shares=200.0,
            ),
        ]
        result = _build_mock_result(
            steps=steps,
            final_fsm=CostReductionFSM(
                state=CostState.COST_REDUCING,
                own_capital=100_000.0,
                margin_amount=100_000.0,
                total_shares=200.0,
                cost_basis=90.0,
                cumulative_recovered=3600.0,
                entry_price=100.0,
                entry_level="30min",
                sub_ratio=0.3,
                min_operable_level="",
                active_short_diff=None,
                completed_short_diffs=completed,
                fugue_voices=(),
            ),
        )
        speed = compute_cost_reduction_speed(result)
        assert speed.short_diff_count == 3
        assert speed.recovery_per_short_diff == pytest.approx(1200.0)


class TestCostReductionTimeSeriesExtraction:
    """extract_cost_reduction_timeseries 正确过滤和提取。"""

    def test_only_position_states_extracted(self) -> None:
        """SCANNING/STOPPED_OUT 不出现在时间序列中。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="SCANNING", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0),
            BarStep(bar_idx=1, bar_ts=1.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="XLK",
                    fsm_state="POSITION_OPEN", cost_basis=50.0,
                    cumulative_recovered=0.0, total_shares=400.0),
            BarStep(bar_idx=2, bar_ts=2.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=48.0,
                    cumulative_recovered=800.0, total_shares=400.0),
            BarStep(bar_idx=3, bar_ts=3.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="PRINCIPAL_WITHDRAWN", cost_basis=40.0,
                    cumulative_recovered=100_000.0, total_shares=400.0),
            BarStep(bar_idx=4, bar_ts=4.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="STOPPED_OUT", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0),
        ]
        result = _build_mock_result(steps=steps, initial_capital=100_000.0)
        ts = extract_cost_reduction_timeseries(result)
        assert ts.bar_indices == (1, 2, 3)
        assert len(ts.cost_basis_series) == 3
        assert ts.cost_basis_series[0] == 50.0
        assert ts.cost_basis_series[1] == 48.0
        assert ts.cost_basis_series[2] == 40.0

    def test_monotonic_recovery_rate(self) -> None:
        """回收率时间序列在正短差利润下单调递增。"""
        steps = [
            BarStep(bar_idx=i, bar_ts=float(i), k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING",
                    cost_basis=100.0 - i * 2,
                    cumulative_recovered=i * 1000.0,
                    total_shares=200.0)
            for i in range(5)
        ]
        result = _build_mock_result(steps=steps, initial_capital=100_000.0)
        ts = extract_cost_reduction_timeseries(result)
        for i in range(1, len(ts.recovery_rate_series)):
            assert ts.recovery_rate_series[i] >= ts.recovery_rate_series[i - 1]


# ═══════════════════════════════════════════════════════════════
# 2. 区间套收敛排序效果验证（267号下游推论2 / 267号§一第二步）
# ═══════════════════════════════════════════════════════════════


class TestNestingTightness:
    """compute_nesting_tightness 紧度计算。"""

    def test_no_buy_points_zero_tightness(self) -> None:
        """无买点 → tightness=0。"""
        snap = _mock_snapshot_with_buy_points(0, 0)
        tightness, level = compute_nesting_tightness(snap)
        assert tightness == 0.0
        assert level == ""

    def test_level1_only(self) -> None:
        """只有 level1 买点 → tightness=1.0。"""
        snap = _mock_snapshot_with_buy_points(1, 0)
        tightness, level = compute_nesting_tightness(snap)
        assert tightness == 1.0
        assert level == "L1"

    def test_multi_level_buy_points(self) -> None:
        """多级别买点共振 → tightness = 级别数。"""
        snap = _mock_snapshot_with_buy_points(1, 2, [2, 3])
        tightness, level = compute_nesting_tightness(snap)
        assert tightness == 3.0
        assert level == "L3"

    def test_recursive_only(self) -> None:
        """只有 recursive 层有买点。"""
        snap = _mock_snapshot_with_buy_points(0, 1, [2])
        tightness, level = compute_nesting_tightness(snap)
        assert tightness == 1.0
        assert level == "L2"


class TestTargetUniverse:
    """target_universe 根据 K4 配置返回正确的标的池。"""

    def test_positive_polarity_equity(self) -> None:
        """polarity > 0 → EQUITY_UNIVERSE。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.UP,
            sigma_r=WalkDirection.UP,
        )
        universe = target_universe(config)
        assert "XLK" in universe
        assert "TLT" not in universe

    def test_negative_polarity_rates(self) -> None:
        """polarity < 0 → RATE_UNIVERSE。"""
        config = Configuration(
            sigma_p=WalkDirection.DOWN,
            sigma_c=WalkDirection.DOWN,
            sigma_r=WalkDirection.DOWN,
        )
        universe = target_universe(config)
        assert "TLT" in universe
        assert "XLK" not in universe

    def test_neutral_polarity_both(self) -> None:
        """polarity == 0 → EQUITY + RATE。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.DOWN,
            sigma_r=WalkDirection.FLAT,
        )
        universe = target_universe(config)
        assert "XLK" in universe
        assert "TLT" in universe

    def test_gold_added_when_e_and_c_up(self) -> None:
        """sigma_p=UP 且 sigma_c=UP → 加入 GLD。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.UP,
            sigma_r=WalkDirection.DOWN,
        )
        universe = target_universe(config)
        assert "GLD" in universe


class TestScanCandidatesRanking:
    """scan_candidates 按 tightness 降序排序且选中最紧的。"""

    def test_highest_tightness_selected(self) -> None:
        """最高 tightness 的标的被选中。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.UP,
            sigma_r=WalkDirection.UP,
        )
        # XLK: tightness=3 (level1 + 2 recursive)
        # XLF: tightness=1 (level1 only)
        candidate_snapshots = {
            "XLK": _mock_snapshot_with_buy_points(1, 2, [2, 3]),
            "XLF": _mock_snapshot_with_buy_points(1, 0),
            "XLE": _mock_snapshot_with_buy_points(0, 0),  # no buy points
        }
        result = scan_candidates(config, candidate_snapshots)
        assert result.selected is not None
        assert result.selected.symbol == "XLK"
        assert result.selected.nesting_tightness == 3.0

    def test_no_candidates_returns_none(self) -> None:
        """无候选标的 → selected=None。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.UP,
            sigma_r=WalkDirection.UP,
        )
        candidate_snapshots = {
            "XLK": _mock_snapshot_with_buy_points(0, 0),
        }
        result = scan_candidates(config, candidate_snapshots)
        assert result.selected is None
        assert len(result.candidates) == 0

    def test_candidates_sorted_by_tightness(self) -> None:
        """candidates 按 tightness 降序排列。"""
        config = Configuration(
            sigma_p=WalkDirection.UP,
            sigma_c=WalkDirection.UP,
            sigma_r=WalkDirection.UP,
        )
        candidate_snapshots = {
            "XLK": _mock_snapshot_with_buy_points(1, 0),      # tightness=1
            "XLF": _mock_snapshot_with_buy_points(1, 1, [2]),  # tightness=2
            "XLV": _mock_snapshot_with_buy_points(1, 2, [2, 3]),  # tightness=3
        }
        result = scan_candidates(config, candidate_snapshots)
        assert len(result.candidates) == 3
        tightnesses = [c.nesting_tightness for c in result.candidates]
        assert tightnesses == sorted(tightnesses, reverse=True)
        assert result.candidates[0].symbol == "XLV"

    def test_polarity_filters_universe(self) -> None:
        """负极性不选股票标的。"""
        config = Configuration(
            sigma_p=WalkDirection.DOWN,
            sigma_c=WalkDirection.DOWN,
            sigma_r=WalkDirection.DOWN,
        )
        # XLK 在 equity 池，polarity<0 应排除
        candidate_snapshots = {
            "XLK": _mock_snapshot_with_buy_points(1, 2, [2, 3]),
            "TLT": _mock_snapshot_with_buy_points(1, 0),
        }
        result = scan_candidates(config, candidate_snapshots)
        # XLK 不在候选中（被 universe 过滤）
        selected_symbols = {c.symbol for c in result.candidates}
        assert "XLK" not in selected_symbols
        if result.selected is not None:
            assert result.selected.symbol == "TLT"


# ═══════════════════════════════════════════════════════════════
# 3. 不换仓原则机会成本验证（267号§五 / 267号下游推论3）
# ═══════════════════════════════════════════════════════════════


class TestNoSwapOpportunityCost:
    """持仓期间错过信号的统计验证。"""

    def test_missed_signals_accumulated(self) -> None:
        """持仓期间的 missed_signals_count 正确累加。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected="XLK",
                    fsm_state="POSITION_OPEN", cost_basis=100.0,
                    cumulative_recovered=0.0, total_shares=200.0,
                    missed_signals_count=2),
            BarStep(bar_idx=1, bar_ts=1.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=95.0,
                    cumulative_recovered=500.0, total_shares=200.0,
                    missed_signals_count=1),
            BarStep(bar_idx=2, bar_ts=2.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="COST_REDUCING", cost_basis=90.0,
                    cumulative_recovered=1000.0, total_shares=200.0,
                    missed_signals_count=3),
        ]
        transitions = [(0, "SCANNING", "POSITION_OPEN")]
        result = _build_mock_result(steps=steps, transitions=transitions)
        analysis = compute_opportunity_cost(result)
        assert analysis.missed_buy_count == 6  # 2+1+3
        assert analysis.held_bars == 3
        assert analysis.held_symbol == "XLK"

    def test_scanning_bars_not_counted(self) -> None:
        """SCANNING 状态不计入 held_bars 或 missed_signals。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="SCANNING", cost_basis=0.0,
                    cumulative_recovered=0.0, total_shares=0.0,
                    missed_signals_count=5),
        ]
        result = _build_mock_result(steps=steps)
        analysis = compute_opportunity_cost(result)
        assert analysis.missed_buy_count == 0
        assert analysis.held_bars == 0

    def test_principal_withdrawn_still_counts(self) -> None:
        """PRINCIPAL_WITHDRAWN 阶段仍然跟踪错过的信号。"""
        steps = [
            BarStep(bar_idx=0, bar_ts=0.0, k4_config_label="(+,+,+)",
                    k4_polarity=3, scanner_selected=None,
                    fsm_state="PRINCIPAL_WITHDRAWN", cost_basis=0.0,
                    cumulative_recovered=100_000.0, total_shares=200.0,
                    missed_signals_count=4),
        ]
        result = _build_mock_result(steps=steps)
        analysis = compute_opportunity_cost(result)
        assert analysis.missed_buy_count == 4
        assert analysis.held_bars == 1

    def test_no_swap_event_exists(self) -> None:
        """CostReductionFSM 中不存在 SWAP 事件。

        267号§五：持仓期间不因外部机会换仓。
        验证方式：FsmEventType 中无 SWAP/SWITCH 类事件。
        """
        event_names = {e.name for e in FsmEventType}
        assert "SWAP" not in event_names
        assert "SWITCH_POSITION" not in event_names
        assert "CHANGE_STOCK" not in event_names


# ═══════════════════════════════════════════════════════════════
# 4. 赋格分裂验证（349号下游推论3 — 回测集成）
# ═══════════════════════════════════════════════════════════════


class TestFugueSplitInBacktest:
    """赋格分裂（小转大）事件在回测中的正确处理。"""

    def test_level_upgrade_creates_voices(self) -> None:
        """小转大创建两个声部（349号§4.2）。"""
        fsm = _make_fsm(equity=100_000, margin=100_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)
        fsm = _start_short_diff(fsm, sell_price=11.0)
        fsm = _close_short_diff(fsm, buy_price=9.0)

        # 小转大
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.LEVEL_UPGRADE,
                price=15.0,
                level="daily",
            ),
        )
        assert len(fsm.fugue_voices) == 2
        old_voice = fsm.fugue_voices[0]
        new_voice = fsm.fugue_voices[1]
        assert old_voice.level == "30min"
        assert old_voice.is_profit_floor is True
        assert new_voice.level == "daily"
        assert new_voice.is_profit_floor is False
        assert fsm.entry_level == "daily"
        assert fsm.active_short_diff is None  # 旧短差停止

    def test_fugue_voice_preserves_cumulative_profit(self) -> None:
        """赋格分裂时旧声部保存累计利润。"""
        fsm = _make_fsm(equity=50_000, margin=50_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        # 完成两轮短差
        fsm = _start_short_diff(fsm, sell_price=12.0)
        fsm = _close_short_diff(fsm, buy_price=10.0)
        recovered_before = fsm.cumulative_recovered

        fsm = _start_short_diff(fsm, sell_price=13.0)
        fsm = _close_short_diff(fsm, buy_price=11.0)

        # 小转大
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.LEVEL_UPGRADE,
                price=16.0,
                level="weekly",
            ),
        )
        old_voice = fsm.fugue_voices[0]
        assert old_voice.cumulative_profit == fsm.cumulative_recovered

    def test_recursive_level_upgrade(self) -> None:
        """递归小转大：第二次小转大产生第三个声部。"""
        fsm = _make_fsm(equity=100_000, margin=100_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)
        fsm = _start_short_diff(fsm, sell_price=11.0)
        fsm = _close_short_diff(fsm, buy_price=9.0)

        # 第一次小转大: 30min → daily
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.LEVEL_UPGRADE,
                price=15.0,
                level="daily",
            ),
        )
        assert len(fsm.fugue_voices) == 2

        # 在新级别上做短差
        fsm = _start_short_diff(fsm, sell_price=16.0)
        fsm = _close_short_diff(fsm, buy_price=14.0)

        # 第二次小转大: daily → weekly
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.LEVEL_UPGRADE,
                price=20.0,
                level="weekly",
            ),
        )
        assert len(fsm.fugue_voices) == 4  # 2 from first + 2 from second
        assert fsm.entry_level == "weekly"


# ═══════════════════════════════════════════════════════════════
# 5. 状态机包装器验证
# ═══════════════════════════════════════════════════════════════


class TestStateMachineWrapper:
    """CostReductionStateMachine 包装器（state_machine.py）验证。"""

    def test_create_initial_state(self) -> None:
        """初始状态为 EMPTY（映射自 SCANNING）。"""
        sm = CostReductionStateMachine.create(
            initial_capital=100_000.0,
            margin_ratio=1.0,
            short_diff_ratio=0.3,
        )
        assert sm.state == StateMachineState.EMPTY

    def test_buy_signal_transitions_to_base(self) -> None:
        """买入信号：EMPTY → BASE。"""
        sm = CostReductionStateMachine.create(
            initial_capital=100_000.0,
            margin_ratio=1.0,
            short_diff_ratio=0.3,
        )
        sm = sm.process_event(
            StateMachineEvent.BUY_SIGNAL,
            price=100.0,
            bar_idx=0,
            trigger="buy_type1_L1",
        )
        assert sm.state == StateMachineState.BASE

    def test_short_diff_cycle_tracked(self) -> None:
        """短差循环被追踪到 actions 和 cost_curve。"""
        sm = CostReductionStateMachine.create(
            initial_capital=100_000.0,
            margin_ratio=1.0,
            short_diff_ratio=0.3,
        )
        sm = sm.process_event(
            StateMachineEvent.BUY_SIGNAL,
            price=100.0, bar_idx=0, trigger="buy",
        )
        sm = sm.process_event(
            StateMachineEvent.SHORT_ENTRY,
            price=110.0, bar_idx=1, trigger="sub_sell",
        )
        sm = sm.process_event(
            StateMachineEvent.SHORT_EXIT,
            price=95.0, bar_idx=2, trigger="sub_buy",
        )
        assert len(sm.actions) == 3
        assert len(sm.cost_curve) == 3
        assert sm.cost_basis < 100.0  # 成本降低了

    def test_immutability(self) -> None:
        """每次 process_event 返回新实例。"""
        sm1 = CostReductionStateMachine.create(
            initial_capital=100_000.0,
            margin_ratio=1.0,
            short_diff_ratio=0.3,
        )
        sm2 = sm1.process_event(
            StateMachineEvent.BUY_SIGNAL,
            price=100.0, bar_idx=0, trigger="buy",
        )
        assert sm1 is not sm2
        assert sm1.state == StateMachineState.EMPTY
        assert sm2.state == StateMachineState.BASE


# ═══════════════════════════════════════════════════════════════
# 6. 止损与重置验证（267号§四 + 338号修正1）
# ═══════════════════════════════════════════════════════════════


class TestStopLossAndReset:
    """止损退出 + 动态重置验证。"""

    def test_buy_point_negated_stops_out(self) -> None:
        """买点失效 → STOPPED_OUT。"""
        fsm = _make_fsm()
        fsm = _open_position(fsm, price=10.0)
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.BUY_POINT_NEGATED,
                price=8.0,
                level="30min",
            ),
        )
        assert fsm.state == CostState.STOPPED_OUT

    def test_reset_with_dynamic_capital(self) -> None:
        """RESET 事件携带新的 own_capital（338号修正1）。"""
        fsm = _make_fsm(equity=100_000, margin=100_000)
        fsm = _open_position(fsm, price=10.0)
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.BUY_POINT_NEGATED,
                price=8.0,
                level="30min",
            ),
        )
        assert fsm.state == CostState.STOPPED_OUT

        # RESET: 止损后剩余资金
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.RESET,
                price=0.0,
                level="",
                new_own_capital=80_000.0,
            ),
        )
        assert fsm.state == CostState.SCANNING
        assert fsm.own_capital == 80_000.0

    def test_cost_reducing_stop_loss(self) -> None:
        """短差进行中买点失效仍止损。"""
        fsm = _make_fsm()
        fsm = _open_position(fsm, price=10.0)
        fsm = _start_short_diff(fsm, sell_price=11.0)
        assert fsm.state == CostState.COST_REDUCING

        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.BUY_POINT_NEGATED,
                price=7.0,
                level="30min",
            ),
        )
        assert fsm.state == CostState.STOPPED_OUT
        assert fsm.active_short_diff is None


# ═══════════════════════════════════════════════════════════════
# 7. 完整场景集成验证
# ═══════════════════════════════════════════════════════════════


class TestFullScenarioIntegration:
    """完整操作周期场景：建仓→降成本→本金退出→退出。"""

    def test_full_cycle_to_principal_withdrawn(self) -> None:
        """完整降成本周期直到本金退出。

        equity=10000, margin=10000 → total=20000, price=10 → shares=2000
        sub_ratio=0.3 → short_shares=600
        sell@15, buy@10 → profit=3000/cycle
        4 cycles → recovered=12000 >= own_capital=10000
        """
        fsm = _make_fsm(equity=10_000, margin=10_000, sub_ratio=0.3)
        assert fsm.state == CostState.SCANNING

        fsm = _open_position(fsm, price=10.0)
        assert fsm.state == CostState.POSITION_OPEN
        assert fsm.total_shares == 2_000.0

        # 4 个短差循环
        for i in range(4):
            fsm = _start_short_diff(fsm, sell_price=15.0)
            fsm = _close_short_diff(fsm, buy_price=10.0)

        assert fsm.state == CostState.PRINCIPAL_WITHDRAWN
        assert fsm.cumulative_recovered >= 10_000.0
        assert len(fsm.completed_short_diffs) == 4

    def test_full_cycle_then_exit(self) -> None:
        """本金退出后，主级别卖点退出。"""
        fsm = _make_fsm(equity=10_000, margin=10_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        for _ in range(4):
            fsm = _start_short_diff(fsm, sell_price=15.0)
            fsm = _close_short_diff(fsm, buy_price=10.0)

        assert fsm.state == CostState.PRINCIPAL_WITHDRAWN

        # 主级别卖点退出
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.MAIN_LEVEL_SELL_POINT,
                price=18.0,
                level="30min",
            ),
        )
        assert fsm.state == CostState.STOPPED_OUT

    def test_full_cycle_with_fugue(self) -> None:
        """完整周期含赋格分裂。

        建仓 → 短差 → 小转大 → 新级别短差 → 本金退出。
        """
        fsm = _make_fsm(equity=10_000, margin=10_000, sub_ratio=0.3)
        fsm = _open_position(fsm, price=10.0)

        # 短差降成本
        fsm = _start_short_diff(fsm, sell_price=15.0)
        fsm = _close_short_diff(fsm, buy_price=10.0)

        # 小转大
        fsm = transition(
            fsm,
            FsmEvent(
                event_type=FsmEventType.LEVEL_UPGRADE,
                price=20.0,
                level="daily",
            ),
        )
        assert fsm.entry_level == "daily"
        assert len(fsm.fugue_voices) == 2

        # 新级别上继续短差
        for _ in range(3):
            fsm = _start_short_diff(fsm, sell_price=25.0)
            fsm = _close_short_diff(fsm, buy_price=20.0)

        assert fsm.state == CostState.PRINCIPAL_WITHDRAWN
        assert fsm.cumulative_recovered >= 10_000.0
