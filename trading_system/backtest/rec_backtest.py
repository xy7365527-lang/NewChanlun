"""递归 T 引擎 × NautilusTrader 回测入口（BTC，事件驱动）。

用法（仓库根目录）：
    .venv/bin/python trading_system/backtest/rec_backtest.py --bars 300000   # 子集验证
    .venv/bin/python trading_system/backtest/rec_backtest.py --bars 0        # 全量 4.6M

链路：BTC JSON(parallel arrays) → NT Bar → BacktestEngine(HYPERLIQUID NETTING)
     → RecTStrategy.on_bar → newchan_rust.RecTStream(递归T引擎) → 目标净敞口 → market 提单。

撮合口径（设计判决一）：FillModel(prob_fill_on_limit=0.0)——限价队列末位；market 单恒成交。
账户起始 = 引擎 INITIAL_CAPITAL(100k USDC) ⟹ NT 1:1 镜像引擎仓位，真账本加滑点/佣金/保证金。
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import pandas as pd
import pytz

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from nautilus_trader.backtest.config import BacktestEngineConfig
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.models import FillModel
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USDC
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import TraderId, Venue
from nautilus_trader.model.objects import Money, Price, Quantity

from trading_system.config.instruments import INSTRUMENTS, make_instrument
from trading_system.strategy.rec_t_strategy import RecTStrategy, RecTStrategyConfig

DEFAULT_DATA = REPO_ROOT / "analysis" / "data_cache" / "btc_1m_full.json"
INITIAL_CAPITAL = 100_000.0  # = newchan_rust 引擎 INITIAL_CAPITAL（NT 1:1 镜像）


def load_btc_bars(
    path: Path, bar_type: BarType, price_precision: int, size_precision: int, max_bars: int | None,
) -> tuple[list[Bar], list[float]]:
    """BTC JSON(parallel arrays) → NT Bar 列表 + 清洗后的 close 序列（BH 用）。

    时间戳合成（1min 等距，从 2017-01-01 UTC）——JSON 无真实 ts，引擎以 bar index 为隐式时间轴，
    NT 仅需单调递增 ts。清洗：删 None / ≤0（与 backtest_run.rs load_clean_ohlc 第一遍一致）。
    """
    data = json.loads(path.read_text())
    o, h, l, c = data["opens"], data["highs"], data["lows"], data["closes"]
    n = len(c)
    if max_bars:
        n = min(n, max_bars)
    start = pd.Timestamp("2017-01-01", tz=pytz.utc).value
    step = 60_000_000_000  # 1 min in ns
    pp = price_precision
    bars: list[Bar] = []
    closes: list[float] = []
    vol = Quantity(1.0, size_precision)  # volume.precision 须 == instrument.size_precision
    for i in range(n):
        oi, hi, li, ci = o[i], h[i], l[i], c[i]
        if oi is None or hi is None or li is None or ci is None:
            continue
        if oi <= 0 or hi <= 0 or li <= 0 or ci <= 0:
            continue
        ts = start + i * step
        bars.append(
            Bar(
                bar_type=bar_type,
                open=Price(round(oi, pp), pp),
                high=Price(round(hi, pp), pp),
                low=Price(round(li, pp), pp),
                close=Price(round(ci, pp), pp),
                volume=vol,
                ts_event=ts,
                ts_init=ts,
            )
        )
        closes.append(ci)
    return bars, closes


def main() -> int:
    parser = argparse.ArgumentParser(description="递归 T 引擎 × NautilusTrader BTC 回测")
    parser.add_argument("--data", type=Path, default=DEFAULT_DATA)
    parser.add_argument("--bars", type=int, default=300_000, help="最大 bar 数（0=全量）")
    parser.add_argument("--mode", default="structural", help="structural/and/or")
    parser.add_argument("--log-level", default="ERROR", help="Nautilus 日志级别")
    args = parser.parse_args()

    if not args.data.exists():
        print(f"数据文件不存在: {args.data}", file=sys.stderr)
        return 1

    spec = INSTRUMENTS["BTC"]
    instrument = make_instrument(spec)
    venue = instrument.id.venue  # HYPERLIQUID

    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("RECT-001"),
            logging=LoggingConfig(log_level=args.log_level),
        ),
    )
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDC,
        starting_balances=[Money(INITIAL_CAPITAL, USDC)],
        fill_model=FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0),
    )
    engine.add_instrument(instrument)

    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    strategy = RecTStrategy(
        config=RecTStrategyConfig(
            instrument_id=instrument.id,
            bar_type=bar_type,
            mode=args.mode,
        ),
    )
    engine.add_strategy(strategy)

    print(f"加载 BTC 数据: {args.data}（max_bars={args.bars or '全量'}）")
    t0 = time.time()
    bars, closes = load_btc_bars(
        args.data, bar_type, spec.price_precision, instrument.size_precision, args.bars or None,
    )
    print(f"构造 {len(bars)} 根 Bar（{time.time() - t0:.1f}s）→ engine.add_data")
    engine.add_data(bars)

    t1 = time.time()
    engine.run()
    run_secs = time.time() - t1

    # ── 结果报告 ──
    account = engine.cache.account_for_venue(venue)
    final_bal = account.balance_total(USDC).as_double() if account else float("nan")
    strat_pct = (final_bal / INITIAL_CAPITAL - 1.0) * 100.0
    bh_pct = (closes[-1] / closes[0] - 1.0) * 100.0 if len(closes) >= 2 else 0.0
    print("\n========== 递归 T × NautilusTrader BTC 回测 ==========")
    print(f"mode={args.mode}  bars={len(bars)}  run={run_secs:.1f}s")
    print(f"NT 真账本: 起始={INITIAL_CAPITAL:.0f} 期末={final_bal:.2f}  strat={strat_pct:+.2f}%")
    print(f"BH={bh_pct:+.2f}%  (closes[0]={closes[0]:.2f} → closes[-1]={closes[-1]:.2f})")
    print(f"策略: bars={strategy.n_bars} orders={strategy.n_orders} dups={strategy.n_dups}")
    print(f"引擎操作(enter/sink/recover/spawn/reruns)={strategy.engine.op_counts()}")
    tot = max(1, strategy.n_bars)
    print(
        f"敞口分布: 多={strategy.n_bars_long}({100*strategy.n_bars_long/tot:.1f}%) "
        f"空={strategy.n_bars_short}({100*strategy.n_bars_short/tot:.1f}%) "
        f"平={strategy.n_bars_flat}({100*strategy.n_bars_flat/tot:.1f}%)"
    )
    print("=====================================================")

    engine.reset()
    engine.dispose()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
