"""统一数据抽象层——上层不关心数据来自哪个源。

存在论位置：本层是**选型路由**，不是数据搬运。Nautilus 的 DataEngine 已经是
统一的数据总线（所有 adapter 的 bar/tick 进同一 msgbus，Strategy 只认
BarType/InstrumentId）——所以本层不重新发明订阅机制，只回答三个问题：

    1. 这个标的的行情应该从哪个源来？（域→源路由）
    2. 这个源能提供什么粒度？（能力声明，防止声明膨胀）
    3. 回测/实盘各用什么 BarType 字符串？（EXTERNAL/INTERNAL 口径）

数据源评估全文：trading_system/data/DATA_SOURCES.md
"""

from __future__ import annotations

import enum
from dataclasses import dataclass, field


class Granularity(enum.Enum):
    TICK = "tick"
    SEC_1 = "1s"
    MIN_1 = "1m"
    DAY_1 = "1d"


class FeedRole(enum.Enum):
    REALTIME = "realtime"  # 实时推送（WS/流式）
    HISTORICAL = "historical"  # 历史批量
    BACKFILL = "backfill"  # 断线缺口补齐


@dataclass(frozen=True)
class FeedSpec:
    """单个数据源的能力声明（严格：只声明已核实的能力，标注认识论等级）。"""

    name: str
    nautilus_adapter: str | None  # None = 无 adapter，须自建/不集成
    realtime_granularities: tuple[Granularity, ...]
    historical_granularities: tuple[Granularity, ...]
    coverage: tuple[str, ...]  # 资产域：futures/equities/crypto/options
    push_based: bool  # True=WS推送 / False=REST轮询
    cost_note: str
    epistemic_level: str  # L0=文档调研 / L1=源码核对 / L2=实测


# ── 数据源注册表（评估结论的机器可读形态，详见 DATA_SOURCES.md）──────

FEEDS: dict[str, FeedSpec] = {
    "HYPERLIQUID": FeedSpec(
        name="Hyperliquid WS",
        nautilus_adapter="nautilus_trader.adapters.hyperliquid",
        # trades(tick) 推送 + 1m 起的交易所 K 线；1s = trades→INTERNAL 聚合
        realtime_granularities=(Granularity.TICK, Granularity.MIN_1),
        historical_granularities=(Granularity.MIN_1,),  # candleSnapshot，深度有限
        coverage=("crypto",),
        push_based=True,
        cost_note="行情免费（直连交易所 WS，零额外成本）",
        epistemic_level="L1（adapter 源码核对：1m 白名单 + trades 订阅确认）",
    ),
    "DATABENTO": FeedSpec(
        name="Databento Live + Historical",
        nautilus_adapter="nautilus_trader.adapters.databento",
        realtime_granularities=(Granularity.TICK, Granularity.SEC_1, Granularity.MIN_1),
        historical_granularities=(Granularity.TICK, Granularity.SEC_1, Granularity.MIN_1, Granularity.DAY_1),
        coverage=("futures", "equities"),
        push_based=True,  # Live = raw TCP 流式
        cost_note="历史按额度；Live CME bundle ~$179/月（在册调研值，待 L2 复核）",
        epistemic_level="L1（adapter 存在已核对；费率 L0 文档值）",
    ),
    "IBKR": FeedSpec(
        name="IBKR TWS API",
        nautilus_adapter="nautilus_trader.adapters.interactive_brokers",
        # reqMktData 是 ~250ms 聚合快照（非逐笔）；实时 bar 下限 5s
        realtime_granularities=(Granularity.MIN_1,),  # 严格口径：可靠粒度只声明 1m+
        historical_granularities=(Granularity.MIN_1, Granularity.DAY_1),
        coverage=("futures", "equities", "options"),
        push_based=True,
        cost_note="已有账户免费；行情线 ~100 条 + pacing 限制（R6）",
        epistemic_level="L0（文档调研：快照聚合/5s下限/行情线数，待实测）",
    ),
    "ALPHAVANTAGE": FeedSpec(
        name="Alpha Vantage REST",
        nautilus_adapter=None,  # 无 adapter；REST 轮询无推送——不进实时链路
        realtime_granularities=(),  # 严格：轮询不是推送，实时能力声明为空
        historical_granularities=(Granularity.MIN_1, Granularity.DAY_1),
        coverage=("equities", "crypto"),
        push_based=False,
        cost_note="免费档 25 req/天；premium $50/月起。仅作历史 fallback",
        epistemic_level="L0（文档调研）",
    ),
}


# ── 域→源路由（理想方案落码）────────────────────────────────────────

@dataclass(frozen=True)
class FeedRoute:
    """一个资产域的数据源决议。"""

    domain: str
    realtime: str  # FEEDS key
    historical: str
    backfill: str
    fallback: str | None = None
    notes: str = ""


ROUTING: dict[str, FeedRoute] = {
    "crypto": FeedRoute(
        domain="crypto",
        realtime="HYPERLIQUID",  # 交易所直连 WS，零额外成本
        historical="HYPERLIQUID",  # candleSnapshot 深度有限；全史回测用本仓库归档
        backfill="HYPERLIQUID",
        fallback=None,
        notes="1s 床位 = trades→INTERNAL 聚合（无交易所 1s K线）",
    ),
    "futures": FeedRoute(
        domain="futures",
        realtime="DATABENTO",  # 首选（若费率过门）；否则 IBKR
        historical="DATABENTO",  # ohlcv-1s 历史已有订阅
        backfill="DATABENTO",
        fallback="IBKR",  # 已有账户免费，5s/1m 粒度够 1m 床位
        notes="IBKR 只做执行+对账（R6）；Databento Live 费率待 L2 确认后定案",
    ),
    "equities": FeedRoute(
        domain="equities",
        realtime="IBKR",
        historical="DATABENTO",  # XNAS.ITCH
        backfill="IBKR",
        fallback="ALPHAVANTAGE",  # 仅历史 fallback（无推送能力）
        notes="",
    ),
}


def feed_for(domain: str, role: FeedRole = FeedRole.REALTIME) -> FeedSpec:
    """域→源解析（上层唯一入口）。"""
    route = ROUTING.get(domain)
    if route is None:
        raise KeyError(f"未注册资产域: {domain}（可选: {list(ROUTING)}）")
    key = {
        FeedRole.REALTIME: route.realtime,
        FeedRole.HISTORICAL: route.historical,
        FeedRole.BACKFILL: route.backfill,
    }[role]
    return FEEDS[key]


def supports(feed_key: str, granularity: Granularity, realtime: bool = True) -> bool:
    """能力查询（下游在订阅前断言，防止隐式降级）。"""
    spec = FEEDS[feed_key]
    pool = spec.realtime_granularities if realtime else spec.historical_granularities
    return granularity in pool
