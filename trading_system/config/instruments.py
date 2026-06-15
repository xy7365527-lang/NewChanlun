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
    """标的元数据（缠论侧视角）。

    price_precision 边界条件（unn NT 流式回测 bit-exact 依据）：
      stream 路径经 NT `Bar→Price(px, price_precision)` 取整，batch 路径用裸 float。
      两条管线 bit-exact ⟺ price_precision ≥ 数据实际小数位（否则取整悄改数=声明膨胀，
      formalization-validity-domain.md）。因此各标的 price_precision = 其数据文件
      全量扫描的精确最小小数位（见 analysis/unn_nt_stream_full_results.md §精度表）。
      此精度是**回测精度**（make_instrument 仅服务回测）；实盘 instrument definitions
      由 adapter 的 InstrumentProvider 从 venue 拉取（tick 可能不同），二者不混表。
    """

    key: str  # 仓库内简称（BTC/CL/BZ/...）
    instrument_id: str  # Nautilus instrument_id 字符串
    price_precision: int
    maint_margin_rate: float  # mm，LeverageCalculator 输入
    bar_floor: str  # 床位（最小操作 bar 周期）
    trading_mode: str  # 在册白名单归属的 voice 变体
    asset_type: str = "future"  # future / equity / crypto_perp（make_instrument 分派）
    multiplier: int = 1_000  # 期货合约乘数（unn 回测装饰性——不经 NT 下单）
    exchange: str = "XNYM"  # 期货 listing 交易所（cosmetic）
    asset_class: str = "COMMODITY"  # 期货 asset_class（COMMODITY/INDEX/FX）
    notes: str = ""


# ── 标的注册表（在册白名单/正域归属；八标的 unn NT 流式回测）─────────────────
# price_precision 全量扫描精确值（exact_min_precision）：OKLO=4 QQQ=3 GC=1 CL=2
#   BTC=3 BRN=2 DX=3 ES=2 —— 见 analysis/unn_nt_stream_full_results.md。
INSTRUMENTS: dict[str, InstrumentSpec] = {
    "OKLO": InstrumentSpec(
        key="OKLO",
        instrument_id="OKLO.XNAS",
        price_precision=4,  # 拆股复权后 4 位（全量扫描）
        maint_margin_rate=0.25,
        bar_floor="1m",
        trading_mode="fusion_t",  # 在册多正域（settle 门 OKLO +137.9pp）
        asset_type="equity",
        notes="股票（NASDAQ）；447K bars-schema（有真实 ts）",
    ),
    "QQQ": InstrumentSpec(
        key="QQQ",
        instrument_id="QQQ.XNAS",
        price_precision=3,  # 全量扫描 3 位
        maint_margin_rate=0.25,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="equity",
        notes="ETF（NASDAQ-100）；728K bars",
    ),
    "GC": InstrumentSpec(
        key="GC",
        instrument_id="GC.GLBX",
        price_precision=1,  # 黄金 tick 0.1（全量扫描 1 位）
        maint_margin_rate=0.05,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="future",
        multiplier=100,  # COMEX GC 100 oz
        exchange="XCEC",
        asset_class="COMMODITY",
        notes="黄金期货（COMEX via GLBX）；5.5M bars",
    ),
    "CL": InstrumentSpec(
        key="CL",
        instrument_id="CL.GLBX",  # TODO(阶段4): 连续 CL.v.0 ↔ 主力单月映射（R7）
        price_precision=2,
        maint_margin_rate=0.10,
        bar_floor="1m",
        trading_mode="fusion_t",  # fusion_t 白名单（CL +351）
        asset_type="future",
        multiplier=1_000,
        exchange="XNYM",
        asset_class="COMMODITY",
        notes="maker 执行精化层标的（限价≤60s撤单不追，净省1bps/侧）",
    ),
    "BTC": InstrumentSpec(
        key="BTC",
        instrument_id="BTC-USD-PERP.HYPERLIQUID",
        # 边界条件：回测数据=Binance 归档 3 位小数（全量扫描 BTC=3）。
        # 旧值=1 对此数据错误（NT 取整 63085.99→63086.0 破坏 bit-exact）。
        # 此为回测精度；实盘 HL tick 由 adapter 从 venue 拉取（可能=1），不混表。
        price_precision=3,
        maint_margin_rate=0.05,  # TODO(阶段4): 从 HL instrument margin_maint 读
        bar_floor="1m",  # 1s 床位待 checkpoint 序列化（风险 R3）
        trading_mode="fusion_tr",  # 在册全史最优 +4298%
        asset_type="crypto_perp",
        notes="恒仓极性 ⟹ 1x 逐仓（hold26 正域）；回测=Binance 归档 4.6M bar（3位小数）",
    ),
    "BRN": InstrumentSpec(
        key="BRN",
        instrument_id="BRN.IFEU",
        price_precision=2,  # 全量扫描 2 位
        maint_margin_rate=0.10,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="future",
        multiplier=1_000,
        exchange="IFEU",
        asset_class="COMMODITY",
        notes="Brent（ICE Europe）；2.4M bars（与 BZ 同物理标的，BZ 保留供 runner.py）",
    ),
    "DX": InstrumentSpec(
        key="DX",
        instrument_id="DX.IFUS",
        price_precision=3,  # 美元指数 tick 0.005（全量扫描 3 位）
        maint_margin_rate=0.04,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="future",
        multiplier=1_000,
        exchange="IFUS",
        asset_class="FX",
        notes="美元指数（ICE US）；2.0M bars",
    ),
    "ES": InstrumentSpec(
        key="ES",
        instrument_id="ES.GLBX",
        price_precision=2,  # 全量扫描 2 位（tick 0.25）
        maint_margin_rate=0.05,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="future",
        multiplier=50,  # CME ES $50/pt
        exchange="XCME",
        asset_class="INDEX",
        notes="S&P500 E-mini（CME via GLBX）；5.6M bars",
    ),
    "BZ": InstrumentSpec(
        key="BZ",
        instrument_id="BZ.GLBX",
        price_precision=2,
        maint_margin_rate=0.10,
        bar_floor="1m",
        trading_mode="fusion_t",
        asset_type="future",
        multiplier=1_000,
        exchange="XNYM",
        asset_class="COMMODITY",
        notes="Brent（NYMEX BZ）；runner.py 在用，保留",
    ),
}


def make_instrument(spec: InstrumentSpec) -> Instrument:
    """由 InstrumentSpec 构造 Nautilus Instrument 对象（回测用）。

    实盘（阶段4+）不走此函数——instrument definitions 由 adapter 的
    InstrumentProvider 从 venue 拉取，本函数仅服务回测 venue 装配。
    分派依据 spec.asset_type（future/equity/crypto_perp）。
    """
    if spec.asset_type == "crypto_perp":
        return _hyperliquid_btc_perp(spec)
    if spec.asset_type == "equity":
        return _equity(spec)
    if spec.asset_type == "future":
        return _future_contract(spec)
    raise KeyError(f"未知 asset_type: {spec.asset_type}（标的 {spec.key}）")


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


_ASSET_CLASS = {
    "COMMODITY": AssetClass.COMMODITY,
    "INDEX": AssetClass.INDEX,
    "FX": AssetClass.FX,
}


def _future_contract(spec: InstrumentSpec) -> FuturesContract:
    """通用期货（回测装配；activation/expiration 覆盖回测窗口即可）。

    multiplier/exchange/asset_class 对 unn 回测装饰性（不经 NT 下单——unn 自有账本），
    唯一影响 bit-exact 的字段是 price_precision。
    """
    symbol, venue = spec.instrument_id.split(".")
    return FuturesContract(
        instrument_id=InstrumentId(symbol=Symbol(symbol), venue=Venue(venue)),
        raw_symbol=Symbol(symbol),
        asset_class=_ASSET_CLASS[spec.asset_class],
        exchange=spec.exchange,
        currency=USD,
        price_precision=spec.price_precision,
        price_increment=Price(10 ** -spec.price_precision, spec.price_precision),
        multiplier=Quantity.from_int(spec.multiplier),
        lot_size=Quantity.from_int(1),
        underlying=symbol,
        activation_ns=pd.Timestamp("2010-01-01", tz=pytz.utc).value,
        expiration_ns=pd.Timestamp("2030-01-01", tz=pytz.utc).value,
        ts_event=0,
        ts_init=0,
    )


def _equity(spec: InstrumentSpec) -> Instrument:
    """股票/ETF（回测装配；OKLO/QQQ）。

    精度参数化（TestInstrumentProvider.equity 写死 2 位，OKLO 需 4/QQQ 需 3——
    不能复用）。lot_size/isin 装饰性（不经 NT 下单）。
    """
    from nautilus_trader.model.instruments import Equity

    symbol, venue = spec.instrument_id.split(".")
    return Equity(
        instrument_id=InstrumentId(symbol=Symbol(symbol), venue=Venue(venue)),
        raw_symbol=Symbol(symbol),
        currency=USD,
        price_precision=spec.price_precision,
        price_increment=Price(10 ** -spec.price_precision, spec.price_precision),
        lot_size=Quantity.from_int(1),
        ts_event=0,
        ts_init=0,
    )


__all__ = ["INSTRUMENTS", "InstrumentSpec", "make_instrument", "USD", "USDT"]
