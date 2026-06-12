"""NautilusTrader 实盘入口（阶段4+ 骨架，当前不可运行）。

整体架构：IBKR（期货/美股/期权）+ Hyperliquid（加密永续）+ Databento（历史+期货实时）。

约束（设计 §7）：
- 单进程单 TradingNode（全局单例）——IBKR + Hyperliquid 并联在单 node 内。
- 启动序列（断线恢复同序，§7.3 + persistence/ 恢复协议）：
    1. Nautilus reconciliation：拉 venue 订单/成交/仓位报告对齐 Cache
    2. 引擎冷启动重放：persistence.BarCache 全史 → 引擎，记录 watermark
    3. 缺口补齐：request_bars(start=watermark) → 喂引擎 → 推进水位线
    4. 影子账本 ↔ Portfolio 对账检查（trade_journal.voice_state vs venue 仓位）
    5. 全部通过后才恢复意图产生（对不齐 = 人工介入，不自动强平/纠偏）
- 重放期间禁止下单（ChanlunStrategy._warmed_up 门控，阶段4实装）。
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from trading_system.config.broker_config import (
    databento_live_config,
    hyperliquid_config,
    ibkr_paper_config,
)


def main() -> int:
    """TODO(阶段4): TradingNode 装配。

    伪代码（实装时逐项替换）：

        from nautilus_trader.live.node import TradingNode
        from nautilus_trader.config import TradingNodeConfig
        from nautilus_trader.adapters.hyperliquid.factories import (
            HyperliquidLiveDataClientFactory, HyperliquidLiveExecClientFactory,
        )

        hl_data, hl_exec = hyperliquid_config(testnet=True)
        config = TradingNodeConfig(
            trader_id="CHANLUN-LIVE-001",
            data_clients={
                "HYPERLIQUID": hl_data,          # 加密侧行情（WS，零额外成本）
                "DATABENTO": ...,                # 期货侧行情（databento_live_config）
            },
            exec_clients={
                "HYPERLIQUID": hl_exec,          # 私钥经 HYPERLIQUID_TESTNET_PK 环境变量
                "INTERACTIVE_BROKERS": ...,      # 期货/美股执行+对账（ibkr_paper_config）
            },
        )
        node = TradingNode(config=config)
        node.add_data_client_factory("HYPERLIQUID", HyperliquidLiveDataClientFactory)
        node.add_exec_client_factory("HYPERLIQUID", HyperliquidLiveExecClientFactory)
        node.trader.add_strategy(ChanlunStrategy(...))  # 每标的一实例（设计 §3.2）
        node.build()
        node.run()
    """
    print("实盘入口为阶段4骨架，尚未实装。先用 trading_system/backtest/runner.py。")
    print(f"IBKR paper 配置读取: {ibkr_paper_config()}")
    print(f"Databento live 配置读取: {{'api_key': {'<set>' if databento_live_config()['api_key'] else '<missing>'}}}")
    try:
        hl_data, hl_exec = hyperliquid_config(testnet=True)
        print(f"Hyperliquid 配置构造 OK: {type(hl_data).__name__} / {type(hl_exec).__name__}")
    except Exception as e:  # noqa: BLE001 —— 骨架诊断输出
        print(f"Hyperliquid 配置构造失败: {e}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
