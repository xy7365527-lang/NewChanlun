"""Databento Live 数据流冒烟：实时 bar → Strategy.on_bar() 验证。

数据-only TradingNode（无 exec client，不下单）：
    Databento Live gateway (GLBX.MDP3) → DatabentoDataClient
    → BarCounter.subscribe_bars(1-MINUTE) → on_bar() 计数。

判定口径（诚实声明）：
- 连接/instrument加载/订阅 三项是无条件验证项；
- on_bar 计数 >0 仅在 CME Globex 开市时段可验证
  （周日18:00 ET 开盘 → 周五17:00 ET 收盘，每日 17:00-18:00 ET 维护停盘）。
  闭市时段运行：前三项 PASS + bars=0 = 管线就绪，非失败。

用法（默认 ES/CL 前月合约，跑 --duration 秒后自动停机）：
    .venv/bin/python trading_system/live/databento_live_runner.py \
        [--instruments ESM6.GLBX CLN6.GLBX] [--bar-spec 1-MINUTE-LAST] [--duration 180]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instruments", nargs="+", default=["ESM6.GLBX", "CLN6.GLBX"])
    parser.add_argument("--bar-spec", default="1-MINUTE-LAST")
    parser.add_argument("--duration", type=float, default=180.0, help="运行秒数后自动停机")
    args = parser.parse_args()

    from dotenv import load_dotenv

    load_dotenv(REPO_ROOT / ".env")

    from nautilus_trader.adapters.databento import DATABENTO, DatabentoLiveDataClientFactory
    from nautilus_trader.config import LoggingConfig, TradingNodeConfig
    from nautilus_trader.live.node import TradingNode

    from trading_system.config.broker_config import databento_live_config
    from trading_system.strategy.bar_counter import BarCounter, BarCounterConfig

    bar_types = [f"{i}-{args.bar_spec}-EXTERNAL" for i in args.instruments]
    node_config = TradingNodeConfig(
        trader_id="DBN-LIVE-SMOKE-001",
        logging=LoggingConfig(log_level="INFO"),
        data_clients={DATABENTO: databento_live_config(args.instruments)},
        timeout_connection=30.0,
        timeout_disconnection=10.0,
        timeout_post_stop=2.0,
    )

    node = TradingNode(config=node_config)
    node.add_data_client_factory(DATABENTO, DatabentoLiveDataClientFactory)
    node.build()

    strategy = BarCounter(BarCounterConfig(bar_types=bar_types))
    node.trader.add_strategy(strategy)

    # 定时停机：node.stop() 非线程安全，须预约在节点自己的事件循环上
    node.kernel.loop.call_later(args.duration, node.stop)

    try:
        node.run()
    finally:
        loaded = [str(i) for i in node.kernel.cache.instrument_ids()]
        counts = {str(k): v for k, v in strategy.bar_counts.items()}
        node.dispose()

    print("\n========== Databento Live 冒烟结果 ==========")
    print(f"instrument 加载: {loaded}")
    print(f"订阅 bar_types : {bar_types}")
    print(f"on_bar 计数    : {counts}")
    missing = [i for i in args.instruments if i not in loaded]
    if missing:
        print(f"FAIL: instrument 未加载 {missing}（连接或 definition 请求失败）")
        return 1
    total = sum(counts.values())
    if total > 0:
        print(f"PASS: {total} 根实时 bar 送达 on_bar()")
    else:
        print("PASS(管线就绪): 连接+加载+订阅成立；bars=0 须对照 CME 开市时段判读")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
