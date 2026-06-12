"""NautilusTrader × 缠论引擎交易系统骨架。

三层架构（analysis/nautilus_integration_design.md §1.2）：

    NautilusTrader 层  —— BacktestNode/TradingNode · DataEngine · ExecEngine · Portfolio
    桥接层（本包）      —— strategy/ execution/ data/ config/：唯一的胶水
    Rust 引擎层        —— newchan_rust.RecursiveOrchestrator（PyO3，不动）

设计原则：Nautilus 不知道缠论，引擎不知道 Nautilus。
两者只通过桥接层的两种原语耦合：下行 (open, high, low, close)，上行信号/意图。
"""

__version__ = "0.1.0"
