"""回测框架共享类型定义。

从 RecursiveOrchestratorSnapshot 提取 D 算子读数：
- 方向态（direction）：最新 settled 走势的方向
- 幅度态（amplitude）：当前走势相对前一走势的幅度比
- 吸收态（absorption）：中枢是否在扩展（新笔是否被中枢吸收）

认识论标注：
  - 类型定义：L0（从缠论定义直接推导）
  - D 算子读数提取：L0（映射规则从定义推导）

谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Literal

from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.orchestrator.recursive import RecursiveOrchestratorSnapshot
from newchan.topology.config_space import Configuration, WalkDirection


# ═══════════════════════════════════════════════════════════════
# D 算子读数
# ═══════════════════════════════════════════════════════════════


class Direction(Enum):
    """方向态：最新 settled 走势的方向。"""

    UP = "up"
    DOWN = "down"
    FLAT = "flat"


@dataclass(frozen=True, slots=True)
class DOperatorReading:
    """单标的 D 算子读数。

    Attributes
    ----------
    direction : Direction
        最新 settled 走势的方向。无 settled 走势时为 FLAT。
    amplitude : float
        当前走势相对前一走势的幅度比。
        amplitude = |current_range| / |prev_range|。
        无前一走势时为 0.0。
    absorption : bool
        中枢是否在扩展（最后一个中枢未 settled = 新段被吸收）。
        无中枢时为 False。
    """

    direction: Direction
    amplitude: float
    absorption: bool


def extract_d_reading(snapshot: RecursiveOrchestratorSnapshot) -> DOperatorReading:
    """从 RecursiveOrchestratorSnapshot 提取 D 算子读数。

    方向态：取 move_snapshot.moves 中最后一个 settled=True 的 move 的 direction。
    幅度态：最后两个 settled move 的价格区间之比。
    吸收态：最后一个中枢是否 settled=False（未闭合=正在吸收新段）。
    """
    moves = snapshot.move_snapshot.moves

    # 方向态
    settled_moves = [m for m in moves if m.settled]
    if settled_moves:
        last_settled = settled_moves[-1]
        direction = Direction.UP if last_settled.direction == "up" else Direction.DOWN
    else:
        direction = Direction.FLAT

    # 幅度态
    amplitude = 0.0
    if len(settled_moves) >= 2:
        prev_move = settled_moves[-2]
        curr_move = settled_moves[-1]
        prev_range = abs(prev_move.high - prev_move.low)
        curr_range = abs(curr_move.high - curr_move.low)
        if prev_range > 0:
            amplitude = curr_range / prev_range

    # 吸收态
    zhongshus = snapshot.zs_snapshot.zhongshus
    absorption = False
    if zhongshus:
        last_zs = zhongshus[-1]
        absorption = not last_zs.settled

    return DOperatorReading(
        direction=direction,
        amplitude=amplitude,
        absorption=absorption,
    )


# ═══════════════════════════════════════════════════════════════
# K4 配置状态
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class AssetState:
    """单资产状态：D 算子读数 + WalkDirection。"""

    symbol: str
    d_reading: DOperatorReading
    walk_direction: WalkDirection


@dataclass(frozen=True, slots=True)
class K4State:
    """K4 三资产配置状态。

    Attributes
    ----------
    e : AssetState
        E/$ (SPY) 状态。
    au : AssetState
        Au/$ (GLD) 状态。
    r : AssetState
        R/$ (TLT) 状态。
    config : Configuration
        K4 配置三元组。
    polarity : int
        极性指数 S。
    """

    e: AssetState
    au: AssetState
    r: AssetState
    config: Configuration
    polarity: int


# ═══════════════════════════════════════════════════════════════
# 选股结果
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class ScannerResult:
    """选股扫描结果。

    Attributes
    ----------
    selected_symbol : str | None
        选中的标的，None = 无可操作标的。
    tightness : float
        区间套收敛紧度。
    candidates_count : int
        候选标的数量。
    """

    selected_symbol: str | None
    tightness: float
    candidates_count: int


# ═══════════════════════════════════════════════════════════════
# 状态机事件
# ═══════════════════════════════════════════════════════════════


class StateMachineState(Enum):
    """5 状态降成本状态机。"""

    EMPTY = "empty"
    BASE = "base"
    ADDED = "added"
    SHORT_TRADE = "short_trade"
    FULL = "full"


class StateMachineEvent(Enum):
    """7 事件。"""

    BUY_SIGNAL = "buy_signal"
    SELL_SIGNAL = "sell_signal"
    SHORT_ENTRY = "short_entry"
    SHORT_EXIT = "short_exit"
    STOP_LOSS = "stop_loss"
    FULL_TRIGGER = "full_trigger"
    CLEAR_TRIGGER = "clear_trigger"


# ═══════════════════════════════════════════════════════════════
# 操作记录
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class TradeAction:
    """一次交易操作记录。

    Attributes
    ----------
    bar_idx : int
        操作发生的 bar 索引。
    action : str
        操作类型（"buy", "sell", "short_sell", "short_cover"）。
    price : float
        成交价。
    quantity : float
        成交量。
    cost_basis : float
        操作后的平均持仓成本。
    trigger : str
        触发原因描述。
    """

    bar_idx: int
    action: str
    price: float
    quantity: float
    cost_basis: float
    trigger: str


@dataclass(frozen=True, slots=True)
class CostCurvePoint:
    """降成本曲线上的一个点。"""

    bar_idx: int
    cost_basis: float
    total_shares: float
    cumulative_recovered: float
