"""BacktestNode × ParquetDataCatalog 冒烟验证。

验证链：databento_catalog.py 产出的 catalog → BacktestNode（高层 API，
instrument 由 catalog 自动解析）→ BarCounter.on_bar() 收到全部 bar。

与 backtest/runner.py（低层 BacktestEngine + 路 B 自有 parquet）互补：
本脚本走高层 BacktestNode + 路 A DBN catalog，是 Databento 管线的消费端验证。

用法：
    .venv/bin/python trading_system/backtest/catalog_smoke.py \
        [--catalog .cache/nautilus_catalog] [--instrument ESM6.GLBX]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from nautilus_trader.backtest.node import BacktestNode
from nautilus_trader.config import (
    BacktestDataConfig,
    BacktestEngineConfig,
    BacktestRunConfig,
    BacktestVenueConfig,
    ImportableStrategyConfig,
    LoggingConfig,
)


def build_run_config(
    catalog_path: str,
    instrument_id: str,
    bar_spec: str = "1-MINUTE-LAST",
) -> BacktestRunConfig:
    venue = instrument_id.split(".", 1)[1]
    bar_type = f"{instrument_id}-{bar_spec}-EXTERNAL"

    return BacktestRunConfig(
        engine=BacktestEngineConfig(
            trader_id="CATALOG-SMOKE-001",
            logging=LoggingConfig(log_level="WARNING"),
            strategies=[
                ImportableStrategyConfig(
                    strategy_path="trading_system.strategy.bar_counter:BarCounter",
                    config_path="trading_system.strategy.bar_counter:BarCounterConfig",
                    config={"bar_types": [bar_type]},
                ),
            ],
        ),
        venues=[
            BacktestVenueConfig(
                name=venue,
                oms_type="NETTING",
                account_type="MARGIN",
                starting_balances=["1_000_000 USD"],
                base_currency="USD",
            ),
        ],
        data=[
            BacktestDataConfig(
                catalog_path=catalog_path,
                data_cls="nautilus_trader.model.data:Bar",
                instrument_id=instrument_id,
                bar_spec=bar_spec,
            ),
        ],
        dispose_on_completion=False,  # run 后保留 engine，读取策略计数
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", default=str(REPO_ROOT / ".cache" / "nautilus_catalog"))
    parser.add_argument("--instrument", default="ESM6.GLBX")
    parser.add_argument("--bar-spec", default="1-MINUTE-LAST")
    args = parser.parse_args()

    config = build_run_config(args.catalog, args.instrument, args.bar_spec)
    node = BacktestNode(configs=[config])
    results = node.run()

    result = results[0]
    engine = node.get_engine(result.run_config_id)
    strategy = engine.trader.strategies()[0]
    delivered = dict(strategy.bar_counts)

    print("\n========== catalog 冒烟结果 ==========")
    print(f"区间        : {result.backtest_start} → {result.backtest_end}")
    print(f"迭代数据条数: {result.iterations:,}")  # total_events 只计订单/仓位事件
    print(f"on_bar 计数 : { {str(k): v for k, v in delivered.items()} }")

    total = sum(delivered.values())
    if total == 0:
        print("FAIL: 没有 bar 送达策略 on_bar()")
        return 1
    print(f"PASS: {total:,} bars 从 catalog 流抵 Strategy.on_bar()")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
