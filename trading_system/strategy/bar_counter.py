"""BarCounter——数据管线冒烟用空策略（不下单，只计数）。

用途：
- 回测：验证 BacktestNode 能从 ParquetDataCatalog 读 bar 并送达 on_bar()
- 实盘：验证 Databento Live 数据流能送达 on_bar()

可经 ImportableStrategyConfig 装配（BacktestNode），也可直接实例化（TradingNode）。
"""

from __future__ import annotations

from nautilus_trader.config import StrategyConfig
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.trading.strategy import Strategy


class BarCounterConfig(StrategyConfig, frozen=True):
    """bar_types: 待订阅 BarType 字符串列表，如 'ESM6.GLBX-1-MINUTE-LAST-EXTERNAL'。"""

    bar_types: list[str]
    log_first_n: int = 3  # 头 N 根逐条打印（肉眼核对价格量级/时间戳口径）


class BarCounter(Strategy):
    def __init__(self, config: BarCounterConfig) -> None:
        super().__init__(config)
        self.bar_counts: dict[BarType, int] = {}

    def on_start(self) -> None:
        for bt_str in self.config.bar_types:
            bar_type = BarType.from_str(bt_str)
            self.bar_counts[bar_type] = 0
            self.subscribe_bars(bar_type)
            self.log.info(f"已订阅 {bar_type}")

    def on_bar(self, bar: Bar) -> None:
        count = self.bar_counts.get(bar.bar_type, 0) + 1
        self.bar_counts[bar.bar_type] = count
        if count <= self.config.log_first_n:
            self.log.info(f"on_bar #{count}: {bar}", color=2)

    def on_stop(self) -> None:
        for bar_type, count in self.bar_counts.items():
            self.log.info(f"[计数] {bar_type}: {count} bars 送达 on_bar()", color=2)
            self.unsubscribe_bars(bar_type)

    def on_reset(self) -> None:
        self.bar_counts = {bt: 0 for bt in self.bar_counts}
