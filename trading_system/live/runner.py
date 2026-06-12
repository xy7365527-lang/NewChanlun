"""NautilusTrader 实盘入口（阶段4+ 骨架，当前不可运行）。

约束（设计 §7）：
- 单进程单 TradingNode（全局单例）——IBKR + Binance 并联在单 node 内。
- 启动序列（断线恢复同序，§7.3）：
    1. Nautilus reconciliation：拉 venue 订单/成交/仓位报告对齐 Cache
    2. 引擎冷启动重放：catalog 全史 → watermark
    3. 影子账本 ↔ Portfolio 对账检查
    4. 全部通过后才恢复意图产生（对不齐 = 人工介入，不自动强平/纠偏）
- 重放期间禁止下单（ChanlunStrategy._warmed_up 门控，阶段4实装）。
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from trading_system.config.broker_config import (
    binance_testnet_config,
    ibkr_paper_config,
    validate_credentials,
)


def main() -> int:
    """TODO(阶段4): TradingNode 装配。

    伪代码（实装时逐项替换）：

        from nautilus_trader.live.node import TradingNode
        from nautilus_trader.config import TradingNodeConfig

        binance = binance_testnet_config()
        validate_credentials(binance, ["api_key", "api_secret"])
        ibkr = ibkr_paper_config()

        config = TradingNodeConfig(
            trader_id="CHANLUN-LIVE-001",
            data_clients={
                "BINANCE": BinanceDataClientConfig(testnet=True, ...),
                # 期货行情走 Databento Live（R6：IBKR 只做执行+对账）
            },
            exec_clients={
                "BINANCE": BinanceExecClientConfig(
                    testnet=True,
                    futures_leverages={"BTCUSDT": 1},        # 恒仓 1x
                    futures_margin_types={"BTCUSDT": "isolated"},
                ),
                "INTERACTIVE_BROKERS": InteractiveBrokersExecClientConfig(...),
            },
        )
        node = TradingNode(config=config)
        node.trader.add_strategy(ChanlunStrategy(...))  # 每标的一实例（设计 §3.2）
        node.build()
        node.run()
    """
    print("实盘入口为阶段4骨架，尚未实装。先用 trading_system/backtest/runner.py。")
    print(f"IBKR paper 配置读取: {ibkr_paper_config()}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
