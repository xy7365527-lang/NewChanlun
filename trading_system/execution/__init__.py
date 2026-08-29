"""执行层：LMT-only 执行器 + maker 状态机 + 杠杆计算器 + 试运行风控四条（#1311）。

分层（nautilus 依赖边界）：
  - 纯函数层（零 nautilus）：LeverageCalculator（结构杠杆）、RiskGate/risk_controls
    （paper 风控四条）——可脱离 nautilus 独立单测（tests/test_trading_system_skeleton.py、
    tests/test_risk_controls.py）；
  - nautilus 层：LmtExecutor / MakerOptimizer（依赖 nautilus_trader.model）。

LmtExecutor / MakerOptimizer 经模块级 __getattr__（PEP 562）惰性导入——
`import trading_system.execution` 不强制 nautilus；直接 `from trading_system.execution
.lmt_executor import ...` 与 `from trading_system.execution import LmtExecutor` 语义不变。
"""

from __future__ import annotations

import importlib

from trading_system.execution.leverage_calculator import LeverageCalculator
from trading_system.execution.risk_controls import (
    OrderIntent,
    RiskGate,
    RiskLimits,
    RiskVerdict,
    RiskVerdictAction,
)

__all__ = [
    "LeverageCalculator",
    "LmtExecutor",
    "MakerOptimizer",
    "MarketOrderForbidden",
    "OrderIntent",
    "RiskGate",
    "RiskLimits",
    "RiskVerdict",
    "RiskVerdictAction",
]

# nautilus 依赖名 → (模块, 属性)。惰性：只在真被访问时才导入。
_NAUTILUS_LAZY = {
    "LmtExecutor": ("trading_system.execution.lmt_executor", "LmtExecutor"),
    "MarketOrderForbidden": ("trading_system.execution.lmt_executor", "MarketOrderForbidden"),
    "MakerOptimizer": ("trading_system.execution.maker_optimizer", "MakerOptimizer"),
}


def __getattr__(name: str):
    spec = _NAUTILUS_LAZY.get(name)
    if spec is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    module_name, attr = spec
    try:
        module = importlib.import_module(module_name)
    except ImportError as exc:
        # PEP 562 契约：属性取不到必须抛 AttributeError，否则 nautilus 未装时
        # hasattr()/getattr(..., default) 会被 ModuleNotFoundError 击穿。
        raise AttributeError(
            f"module {__name__!r} has no attribute {name!r}（依赖模块不可导入）"
        ) from exc
    return getattr(module, attr)
