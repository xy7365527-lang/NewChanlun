"""数据层：历史数据加载 + bar 聚合配置 + 统一数据源抽象（域→源路由）。"""

from trading_system.data.bar_aggregator import external_1m_bar_type, internal_1s_bar_type
from trading_system.data.databento_loader import bars_from_dataframe, load_parquet_bars
from trading_system.data.feed_abstraction import FEEDS, ROUTING, FeedRole, Granularity, feed_for, supports

__all__ = [
    "bars_from_dataframe",
    "load_parquet_bars",
    "internal_1s_bar_type",
    "external_1m_bar_type",
    "FEEDS",
    "ROUTING",
    "FeedRole",
    "Granularity",
    "feed_for",
    "supports",
]
