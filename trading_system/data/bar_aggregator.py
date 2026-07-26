"""bar 聚合配置（tick→1s INTERNAL 聚合，设计 §2.1，加密侧改 Hyperliquid）。

Nautilus 的聚合不需要自写聚合器——BarType 后缀 INTERNAL 即由 DataEngine
从 TradeTick 流内部聚合。本模块只负责生成正确的 BarType 字符串。

Hyperliquid 1s 判决（adapter 源码核对，与原 Binance futures 判决同型）：
    HL 交易所侧 K 线最小 1m（bar_type_to_interval 白名单
    1m/3m/5m/15m/30m/1h/2h/4h/8h/12h/1d/3d/1w/1M，仅 EXTERNAL，
    nautilus_trader/crates/adapters/hyperliquid/src/common/parse.rs:437）。
    秒级路径 = subscribe_trade_ticks（WS trades）→ Nautilus 内部聚合：
    "BTC-USD-PERP.HYPERLIQUID-1-SECOND-LAST-INTERNAL"
    此结论需 testnet 实测收口（L0→L2 缺口，风险 R5：INTERNAL 1s 聚合出的
    1m 与 EXTERNAL 1m candle 对账未验证）。

IBKR 场景（风险 R6）：行情线 ~100 条、秒级 bar 实务下限 ~5s——
    1s 床位不能靠 IBKR 数据；期货行情走 Databento Live，IBKR 只做执行+对账。
"""

from __future__ import annotations

from nautilus_trader.model.data import BarType


def internal_1s_bar_type(instrument_id: str) -> BarType:
    """tick→1s 内部聚合 BarType（Hyperliquid 秒级床位）。"""
    return BarType.from_str(f"{instrument_id}-1-SECOND-LAST-INTERNAL")


def external_1m_bar_type(instrument_id: str) -> BarType:
    """交易所/数据商已聚合的 1m bar（EXTERNAL——Databento/HL candle）。"""
    return BarType.from_str(f"{instrument_id}-1-MINUTE-LAST-EXTERNAL")


# TODO(阶段4): INTERNAL 1s 聚合与 EXTERNAL 1m kline 的对账守卫
#   （聚合 60 根 INTERNAL 1s → 与 EXTERNAL 1m 同窗口 OHLCV 比对，
#    偏差超容差 = 数据口径漂移告警）。
