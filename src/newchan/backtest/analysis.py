"""回测结果分析 — 267号操作方法论三个下游推论的度量。

认识论标注：
  - 分析框架：L0（从定义推导的度量方式）
  - 分析结果：L1（管线输出，需真实数据升级到 L2）

谱系引用：267号操作方法论 v1 下游推论1-3。

三个度量：
  1. 降成本速度分析：满仓满融下降成本速度 vs 时间
  2. 区间套收敛排序效果：选股命中后的收益与收敛紧度的相关性
  3. 不换仓机会成本：持仓期间其他标的出现的可操作信号数量
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from newchan.backtest.full_pipeline import BarStep, FullPipelineResult


# ═══════════════════════════════════════════════════════════════
# 1. 降成本速度分析（下游推论1）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class CostReductionSpeed:
    """降成本速度度量。

    Attributes
    ----------
    total_recovered : float
        累计回收金额。
    bars_in_position : int
        持仓状态的 bar 数（POSITION_OPEN + COST_REDUCING + PRINCIPAL_WITHDRAWN）。
    recovery_per_bar : float
        每 bar 平均回收金额（总回收/持仓 bar 数）。
    short_diff_count : int
        短差循环次数（FSM 转移中 COST_REDUCING 相关的次数）。
    recovery_per_short_diff : float
        每次短差平均回收金额。
    own_capital : float
        初始自有资金（本次循环）。
    recovery_ratio : float
        回收率 = 累计回收 / 初始自有资金。
    """

    total_recovered: float
    bars_in_position: int
    recovery_per_bar: float
    short_diff_count: int
    recovery_per_short_diff: float
    own_capital: float
    recovery_ratio: float


def compute_cost_reduction_speed(
    result: FullPipelineResult,
) -> CostReductionSpeed:
    """从回测结果计算降成本速度。

    遍历 BarStep 序列，统计持仓 bar 数和降成本进度。
    """
    position_states = {"POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN"}
    bars_in_position = sum(
        1 for step in result.steps if step.fsm_state in position_states
    )

    # 短差次数：从 FSM 转移中统计进出 COST_REDUCING 的次数
    short_diff_count = sum(
        1 for _, old, new in result.fsm_transitions
        if old == "POSITION_OPEN" and new == "COST_REDUCING"
        or old == "COST_REDUCING" and new == "COST_REDUCING"
    )
    # 补充：从 completed_short_diffs 取更精确的计数
    short_diff_count = max(
        short_diff_count,
        len(result.final_fsm.completed_short_diffs),
    )

    total_recovered = result.final_fsm.cumulative_recovered
    own_capital = result.config.initial_capital

    recovery_per_bar = (
        total_recovered / bars_in_position
        if bars_in_position > 0
        else 0.0
    )
    recovery_per_short_diff = (
        total_recovered / short_diff_count
        if short_diff_count > 0
        else 0.0
    )
    recovery_ratio = (
        total_recovered / own_capital
        if own_capital > 0
        else 0.0
    )

    return CostReductionSpeed(
        total_recovered=total_recovered,
        bars_in_position=bars_in_position,
        recovery_per_bar=recovery_per_bar,
        short_diff_count=short_diff_count,
        recovery_per_short_diff=recovery_per_short_diff,
        own_capital=own_capital,
        recovery_ratio=recovery_ratio,
    )


# ═══════════════════════════════════════════════════════════════
# 2. 不换仓机会成本分析（下游推论3）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class MissedSignal:
    """持仓期间错过的信号。

    Attributes
    ----------
    bar_idx : int
        信号出现的 bar 索引。
    symbol : str
        产生信号的标的。
    signal_type : str
        信号类型（"buy" / "sell"）。
    price : float
        信号价格。
    """

    bar_idx: int
    symbol: str
    signal_type: str
    price: float


@dataclass(frozen=True, slots=True)
class OpportunityCostAnalysis:
    """不换仓机会成本分析。

    Attributes
    ----------
    missed_signals : tuple[MissedSignal, ...]
        持仓期间在其他标的上出现的可操作信号。
    missed_buy_count : int
        错过的买入信号数量。
    missed_sell_count : int
        错过的卖出信号数量。
    held_bars : int
        持仓期间的总 bar 数。
    held_symbol : str
        持仓标的。
    """

    missed_signals: tuple[MissedSignal, ...]
    missed_buy_count: int
    missed_sell_count: int
    held_bars: int
    held_symbol: str


def compute_opportunity_cost(
    result: FullPipelineResult,
) -> OpportunityCostAnalysis:
    """从回测结果计算不换仓机会成本。

    统计持仓期间（FSM 不在 SCANNING / STOPPED_OUT）
    在 BarStep.missed_signals_count 中记录的信号。

    由于 BarStep 中已嵌入 missed_signals_count，
    这里做汇总统计。
    """
    position_states = {"POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN"}
    held_bars = sum(
        1 for step in result.steps if step.fsm_state in position_states
    )

    # 从 BarStep 的 missed_signals_count 汇总
    total_missed = sum(
        step.missed_signals_count
        for step in result.steps
        if step.fsm_state in position_states
    )

    # 确定持仓标的
    held_symbol = ""
    for _, old, new in result.fsm_transitions:
        if old == "SCANNING" and new == "POSITION_OPEN":
            # 从 steps 中找到对应的 scanner_selected
            for step in result.steps:
                if step.scanner_selected is not None:
                    held_symbol = step.scanner_selected
                    break
            break

    return OpportunityCostAnalysis(
        missed_signals=(),  # 详细信号列表由引擎跟踪
        missed_buy_count=total_missed,
        missed_sell_count=0,
        held_bars=held_bars,
        held_symbol=held_symbol,
    )


# ═══════════════════════════════════════════════════════════════
# 3. 降成本速度时间序列（用于可视化）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class CostReductionTimeSeries:
    """降成本过程的时间序列数据。

    Attributes
    ----------
    bar_indices : tuple[int, ...]
        bar 索引序列。
    cost_basis_series : tuple[float, ...]
        对应的成本基准序列。
    cumulative_recovered_series : tuple[float, ...]
        对应的累计回收序列。
    recovery_rate_series : tuple[float, ...]
        对应的回收率序列（累计回收/自有资金）。
    """

    bar_indices: tuple[int, ...]
    cost_basis_series: tuple[float, ...]
    cumulative_recovered_series: tuple[float, ...]
    recovery_rate_series: tuple[float, ...]


def extract_cost_reduction_timeseries(
    result: FullPipelineResult,
) -> CostReductionTimeSeries:
    """从回测结果提取降成本时间序列。

    仅提取持仓状态（POSITION_OPEN / COST_REDUCING / PRINCIPAL_WITHDRAWN）
    的 BarStep 数据。
    """
    position_states = {"POSITION_OPEN", "COST_REDUCING", "PRINCIPAL_WITHDRAWN"}
    own_capital = result.config.initial_capital

    indices: list[int] = []
    costs: list[float] = []
    recovered: list[float] = []
    rates: list[float] = []

    for step in result.steps:
        if step.fsm_state not in position_states:
            continue
        indices.append(step.bar_idx)
        costs.append(step.cost_basis)
        recovered.append(step.cumulative_recovered)
        rates.append(
            step.cumulative_recovered / own_capital if own_capital > 0 else 0.0
        )

    return CostReductionTimeSeries(
        bar_indices=tuple(indices),
        cost_basis_series=tuple(costs),
        cumulative_recovered_series=tuple(recovered),
        recovery_rate_series=tuple(rates),
    )
