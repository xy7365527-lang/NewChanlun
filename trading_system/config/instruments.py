"""标的配置（设计 §2.1 标的与数据源矩阵，加密侧改 Hyperliquid）。

先支持：BTC 永续（Hyperliquid）+ CL/BZ 期货（CME GLBX）。
连续合约警告（风险 R7）：信号标的（如 CL.v.0）≠ 下单标的（当前主力单月合约），
执行须经 signal_instrument → exec_instrument 滚动映射（阶段4 定规则）。

恒仓极性标的（hold26 正域，在册）：L_max ≈ 1.0–1.3x
    ⟹ BTC 主仓 1x 逐仓——HL 杠杆 per-position 下单时指定，
    约束在 LeverageGovernor 层钳制（见 broker_config.hyperliquid_config）。
"""

from __future__ import annotations

from dataclasses import dataclass

import pandas as pd
import pytz
from nautilus_trader.model.currencies import USD, USDT
from nautilus_trader.model.enums import AssetClass
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import FuturesContract, Instrument
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.test_kit.providers import TestInstrumentProvider


@dataclass(frozen=True)
class InstrumentSpec:
    """标的元数据（缠论侧视角）。"""

    key: str  # 仓库内简称（BTC/CL/BZ/...）
    instrument_id: str  # Nautilus instrument_id 字符串
    price_precision: int
    maint_margin_rate: float  # mm，LeverageCalculator 输入
    bar_floor: str  # 床位（最小操作 bar 周期）
    trading_mode: str  # 在册白名单归属的 voice 变体
    notes: str = ""


# ── 标的注册表（在册白名单/正域归属）──────────────────────────────
INSTRUMENTS: dict[str, InstrumentSpec] = {
    "BTC": InstrumentSpec(
        key="BTC",
        instrument_id="BTC-USD-PERP.HYPERLIQUID",
        price_precision=1,
        maint_margin_rate=0.05,  # TODO(阶段4): 从 HL instrument margin_maint 读
        bar_floor="1m",  # 1s 床位待 checkpoint 序列化（风险 R3）
        trading_mode="fusion_tr",  # 在册全史最优 +4298%
        notes="恒仓极性 ⟹ 1x 逐仓（hold26 正域）；HL 无 1s K线，1s 床位走 trades→INTERNAL",
    ),
    "CL": InstrumentSpec(
        key="CL",
        instrument_id="CL.GLBX",  # TODO(阶段4): 连续 CL.v.0 ↔ 主力单月映射（R7）
        price_precision=2,
        maint_margin_rate=0.10,
        bar_floor="1m",
        trading_mode="fusion_t",  # fusion_t 白名单（CL +351）
        notes="maker 执行精化层标的（限价≤60s撤单不追，净省1bps/侧）",
    ),
    "BZ": InstrumentSpec(
        key="BZ",
        instrument_id="BZ.GLBX",
        price_precision=2,
        maint_margin_rate=0.10,
        bar_floor="1m",
        trading_mode="fusion_t",
        notes="Brent（NYMEX BZ）；回测验证标的（.cache 现成 1min 数据）",
    ),
}


def make_instrument(spec: InstrumentSpec) -> Instrument:
    """由 InstrumentSpec 构造 Nautilus Instrument 对象（回测用）。

    实盘（阶段4+）不走此函数——instrument definitions 由 adapter 的
    InstrumentProvider 从 venue 拉取，本函数仅服务回测 venue 装配。
    """
    if spec.key == "BTC":
        return _hyperliquid_btc_perp(spec)
    if spec.key in ("CL", "BZ"):
        return _glbx_future(spec)
    raise KeyError(f"未注册标的: {spec.key}")


def _hyperliquid_btc_perp(spec: InstrumentSpec) -> CryptoPerpetual:
    """Hyperliquid BTC-USD-PERP（回测装配）。

    实盘 definitions 由 HyperliquidInstrumentProvider 从 venue 拉取——本构造
    仅服务回测，精度/费率与 venue 实值的偏差在阶段4 对账时收口（不可混表）。
    HL 费率（L0 文档值，待 L2）：maker 0.01% / taker 0.035%（基础档）。
    """
    from decimal import Decimal

    from nautilus_trader.model.currencies import BTC, USDC
    from nautilus_trader.model.instruments import CryptoPerpetual
    from nautilus_trader.model.objects import Money

    return CryptoPerpetual(
        instrument_id=InstrumentId(symbol=Symbol("BTC-USD-PERP"), venue=Venue("HYPERLIQUID")),
        raw_symbol=Symbol("BTC"),
        base_currency=BTC,
        quote_currency=USDC,  # HL 保证金/结算货币 USDC
        settlement_currency=USDC,
        is_inverse=False,
        price_precision=spec.price_precision,
        price_increment=Price(10 ** -spec.price_precision, spec.price_precision),
        size_precision=5,  # HL BTC szDecimals=5
        size_increment=Quantity.from_str("0.00001"),
        max_quantity=Quantity.from_str("10000"),
        min_quantity=Quantity.from_str("0.00001"),
        max_notional=None,
        min_notional=Money(10.00, USDC),
        max_price=Price.from_str("1000000.0"),
        min_price=Price.from_str("0.1"),
        margin_init=Decimal("0.10"),
        margin_maint=Decimal(str(spec.maint_margin_rate)),
        maker_fee=Decimal("0.0001"),
        taker_fee=Decimal("0.00035"),
        ts_event=0,
        ts_init=0,
    )


def _glbx_future(spec: InstrumentSpec) -> FuturesContract:
    """CME GLBX 能源期货（回测装配；activation/expiration 覆盖回测窗口即可）。"""
    symbol = spec.instrument_id.split(".")[0]
    return FuturesContract(
        instrument_id=InstrumentId(symbol=Symbol(symbol), venue=Venue("GLBX")),
        raw_symbol=Symbol(symbol),
        asset_class=AssetClass.COMMODITY,
        exchange="XNYM",
        currency=USD,
        price_precision=spec.price_precision,
        price_increment=Price(10 ** -spec.price_precision, spec.price_precision),
        multiplier=Quantity.from_int(1_000),  # CL/BZ 1000 桶
        lot_size=Quantity.from_int(1),
        underlying=symbol,
        activation_ns=pd.Timestamp("2010-01-01", tz=pytz.utc).value,
        expiration_ns=pd.Timestamp("2030-01-01", tz=pytz.utc).value,
        ts_event=0,
        ts_init=0,
    )


__all__ = ["INSTRUMENTS", "InstrumentSpec", "make_instrument", "USD", "USDT"]
