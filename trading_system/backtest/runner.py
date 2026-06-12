"""NautilusTrader 回测入口（阶段1：信号贯通验证）。

用法（仓库根目录）：
    .venv/bin/python trading_system/backtest/runner.py                 # BZ 1min 默认3万根
    .venv/bin/python trading_system/backtest/runner.py --bars 100000
    .venv/bin/python trading_system/backtest/runner.py --enable-orders # 阶段3形态

验证链路：parquet → Bar 构造（路B）→ BacktestEngine → ChanlunStrategy.on_bar
         → ChanlunBridge.feed → Rust 引擎 → drain_signals → 日志。

撮合口径（设计判决一，强制）：
    FillModel(prob_fill_on_limit=0.0) —— 队列末位保守口径，只有价格穿越限价位才成交。
    乐观口径（=1.0）只允许作为对照臂，两口径数字不可混表。
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.config import BacktestEngineConfig
from nautilus_trader.backtest.models import FillModel
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import TraderId, Venue
from nautilus_trader.model.objects import Money

from trading_system.config.instruments import INSTRUMENTS, make_instrument
from trading_system.data.databento_loader import load_parquet_bars
from trading_system.strategy.chanlun_strategy import ChanlunStrategy, ChanlunStrategyConfig

DEFAULT_DATA = REPO_ROOT / ".cache" / "BZ_1min_2024_raw.parquet"


def build_engine(
    enable_orders: bool, log_level: str, db_path: str | None = None,
) -> tuple[BacktestEngine, object]:
    spec = INSTRUMENTS["BZ"]
    instrument = make_instrument(spec)

    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("CHANLUN-001"),
            logging=LoggingConfig(log_level=log_level),
        ),
    )
    engine.add_venue(
        venue=Venue("GLBX"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(1_000_000.0, USD)],
        # 判决一：保守撮合口径钉死（队列末位，touch 不成交，穿越才成交）
        fill_model=FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0),
    )
    engine.add_instrument(instrument)

    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    strategy = ChanlunStrategy(
        config=ChanlunStrategyConfig(
            instrument_id=instrument.id,
            bar_type=bar_type,
            trading_mode=spec.trading_mode,
            enable_orders=enable_orders,
            maint_margin_rate=spec.maint_margin_rate,
            db_path=db_path,
        ),
    )
    engine.add_strategy(strategy)
    return engine, (instrument, bar_type, spec)


def main() -> int:
    parser = argparse.ArgumentParser(description="缠论 × NautilusTrader 回测（阶段1骨架）")
    parser.add_argument("--data", type=Path, default=DEFAULT_DATA, help="OHLCV parquet 路径")
    parser.add_argument("--bars", type=int, default=30_000, help="最大 bar 数（0=全量）")
    parser.add_argument("--enable-orders", action="store_true", help="开启下单（阶段3形态）")
    parser.add_argument("--log-level", default="WARNING", help="Nautilus 日志级别")
    parser.add_argument("--db", default=None, help="SQLite 持久化路径（信号/订单/bar缓存/快照）")
    args = parser.parse_args()

    if not args.data.exists():
        print(f"数据文件不存在: {args.data}", file=sys.stderr)
        return 1

    engine, (instrument, bar_type, spec) = build_engine(
        args.enable_orders, args.log_level, db_path=args.db,
    )

    print(f"加载数据: {args.data}（max_bars={args.bars or '全量'}）")
    bars = load_parquet_bars(
        args.data,
        bar_type=bar_type,
        price_precision=spec.price_precision,
        max_bars=args.bars or None,
    )
    print(f"构造 {len(bars)} 根 Bar → engine.add_data")
    engine.add_data(bars)

    engine.run()

    # ── 结果报告 ──
    strategy = engine.trader.strategies()[0]
    bridge = strategy.bridge
    print("\n========== 回测完成 ==========")
    print(f"bars 喂入引擎 : {bridge.bar_count}")
    print(f"信号产出      : {strategy._signal_count}")
    print(f"gap 计数      : {bridge.gap_count}（只记录不填充）")
    print(f"重叠丢弃      : {bridge.dup_count}")
    print(f"结构快照      : {bridge.structure_snapshot()}")
    if args.enable_orders and strategy.maker is not None:
        m = strategy.maker
        print(
            f"maker 状态机  : fills={m.fill_count} cancels={m.cancel_count} "
            f"rejects={m.reject_count} skips={m.skip_count}"
        )
        print(engine.trader.generate_account_report(Venue("GLBX")))

    engine.reset()
    engine.dispose()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
