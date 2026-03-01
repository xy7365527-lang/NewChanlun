"""全流程回测子包。

模块结构：
- engine.py      — 旧回测引擎（BacktestEngine, BacktestConfig 等）
- types.py       — 共享类型定义（D 算子读数、K4 状态、操作记录等）
- k4_config.py   — K4 配置读取（含 D 算子读数）
- scanner.py     — 选股扫描（区间套收敛紧度排序）
- state_machine.py — 降成本状态机（5 状态 7 事件）
- orchestrator.py — 全流程编排（K4 → 选股 → 状态机 → 成本跟踪）
- full_pipeline.py — 旧全流程引擎（保留兼容）
- analysis.py     — 回测结果分析
"""

# 从 engine.py 重导出旧 API，保持 `from newchan.backtest import X` 兼容
from newchan.backtest.engine import (  # noqa: F401
    BacktestConfig,
    BacktestEngine,
    BacktestResult,
    CostSummary,
    PositionPhase,
    ShortDiffRecord,
    Trade,
)
