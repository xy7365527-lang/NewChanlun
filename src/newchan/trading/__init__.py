"""操盘方法包 — 仓位确定性函数 + 背驰力度分配 + 降成本递归 + 四层递归状态机。"""

from newchan.trading.cost_reduction import CostTracker, global_efficiency, transfer_efficiency
from newchan.trading.divergence_allocation import (
    GammaByLayer,
    allocate,
    divergence_ratio,
)
from newchan.trading.position_sizing import (
    STANDARD_SIGNALS,
    PositionSizer,
    Signal,
)
from newchan.trading.layer_state import Layer, LayerState, LayerType
from newchan.trading.state_machine import FourLayerStateMachine
from newchan.trading.axioms import (
    axiom1_coverage,
    axiom2_no_upward_entry,
    axiom3_profit_transfer,
)
from newchan.trading.paths import PathType, classify_path, execute_path

__all__ = [
    "CostTracker",
    "FourLayerStateMachine",
    "GammaByLayer",
    "Layer",
    "LayerState",
    "LayerType",
    "PathType",
    "PositionSizer",
    "STANDARD_SIGNALS",
    "Signal",
    "allocate",
    "axiom1_coverage",
    "axiom2_no_upward_entry",
    "axiom3_profit_transfer",
    "classify_path",
    "divergence_ratio",
    "execute_path",
    "global_efficiency",
    "transfer_efficiency",
]
